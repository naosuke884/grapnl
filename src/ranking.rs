//! ゲームオーバーの画面に出す、今日のコースのランキング。
//! ゲームオーバーになったらスコアを送ってから読み始め（`roommate_sdk::request_ranking`）、
//! 読み終えたら上位と自分の順位を出す。送ったばかりのスコアも順位に入る。
//! 名前は誰でも付けられ、画面の文字だけに絞ったフォントでは描けないことが多い。
//! そのときだけ、名前用に全文字のフォント（約 1.6 MB）を後から読み込み、それまでは描けない文字を「?」にする。

use macroquad::experimental::coroutines::{start_coroutine, Coroutine};
use macroquad::prelude::*;
use roommate_sdk::{Ranking, RankingStatus};

use crate::text::Text;

/// 上位から出す行数の上限。画面が低いときはこれより減らす。
pub const MAX_ROWS: usize = 5;
/// 1 行の高さ。文字は 22 の大きさで描く。
pub const ROW_HEIGHT: f32 = 28.0;
/// 表の横幅。順位・名前・スコアを並べる。
const WIDTH: f32 = 360.0;
/// 順位の列の右端（表の左端から）。
const RANK_RIGHT: f32 = 52.0;
/// 名前の列の左端（表の左端から）。
const NAME_LEFT: f32 = 72.0;
/// 名前の無い人は、roommate が匿名 ID の頭 4 文字から作る（「名無し#1a2b」）。
/// よく出るので、絞ったフォントに入れておき、名前用のフォントを読まずに描けるようにする。
#[allow(dead_code)]
const NAME_CHARS: &str = "名無し#0123456789abcdef";
/// 名前用のフォント。scripts/subset-font.py が web/ に置く。ブラウザではページと同じ場所から読む。
#[cfg(target_arch = "wasm32")]
const NAMES_FONT: &str = "MPLUS1p-Bold-names.ttf";
#[cfg(not(target_arch = "wasm32"))]
const NAMES_FONT: &str = "web/MPLUS1p-Bold-names.ttf";

const HEADER_COLOR: Color = Color::new(0.59, 0.78, 0.92, 1.0);
const ROW_COLOR: Color = Color::new(0.85, 0.88, 0.92, 1.0);
/// 自分の行。連鎖の表示と同じ色。
const MINE_COLOR: Color = Color::new(1.0, 0.78, 0.31, 1.0);
const NOTE_COLOR: Color = Color::new(0.59, 0.63, 0.69, 1.0);

/// 表に出す 1 行。
#[derive(Debug, Clone, PartialEq, Eq)]
enum Line {
    /// 読み込み中などの知らせ。
    Note(&'static str),
    Row {
        rank: u32,
        name: String,
        score: u32,
        mine: bool,
    },
    /// 上位の行に入っていない自分の順位。
    Mine(u32),
}

/// 名前用のフォント。一度読み込んだら、やり直しても持っておく。
enum NamesFont {
    NotNeeded,
    Loading(Coroutine<Option<Vec<u8>>>),
    Ready(Text),
    /// 読み込めなかった。描けない文字は「?」のままにし、読み直さない。
    Failed,
}

pub struct Board {
    status: RankingStatus,
    names: NamesFont,
}

impl Default for Board {
    fn default() -> Self {
        Self {
            status: RankingStatus::NotRequested,
            names: NamesFont::NotNeeded,
        }
    }
}

impl Board {
    /// ゲームオーバーになって、スコアを送った後に 1 回呼ぶ。
    pub fn request(&mut self) {
        roommate_sdk::request_ranking();
        self.status = RankingStatus::Loading;
    }

    /// やり直すときに呼ぶ。次のゲームオーバーで読み直す。
    pub fn reset(&mut self) {
        self.status = RankingStatus::NotRequested;
    }

    /// 毎フレーム呼ぶ。読み終えたら手元に持ち、それ以降は roommate-sdk に聞かない。
    /// `text` で描けない名前があれば、名前用のフォントを読み始める。
    pub fn update(&mut self, text: &Text) {
        if self.status == RankingStatus::Loading {
            match roommate_sdk::ranking() {
                // 頼んだ直後に頼んでいないことになるのは、データを消したときなど。読み込み中のままにはしない。
                RankingStatus::NotRequested => self.status = RankingStatus::Failed,
                status => self.status = status,
            }
        }
        match &self.names {
            NamesFont::NotNeeded => {
                if let RankingStatus::Ready(ranking) = &self.status {
                    let missing = ranking
                        .entries
                        .iter()
                        .any(|entry| !entry.name.chars().all(|c| text.has(c)));
                    if missing {
                        self.names = NamesFont::Loading(start_coroutine(async {
                            load_file(NAMES_FONT).await.ok()
                        }));
                    }
                }
            }
            NamesFont::Loading(loading) => {
                if loading.is_done() {
                    self.names = match loading
                        .retrieve()
                        .flatten()
                        .and_then(|bytes| Text::from_bytes(&bytes))
                    {
                        Some(font) => NamesFont::Ready(font),
                        None => NamesFont::Failed,
                    };
                }
            }
            NamesFont::Ready(_) | NamesFont::Failed => {}
        }
    }

    /// 名前を描く。名前用のフォントがあればそれで、無ければ描けない文字を「?」にして描く。
    fn draw_name(&self, text: &Text, name: &str, x: f32, baseline: f32, color: Color) {
        if text.drawable(name) == name {
            text.draw(name, x, baseline, 22.0, color);
        } else if let NamesFont::Ready(names) = &self.names {
            names.draw(&names.drawable(name), x, baseline, 22.0, color);
        } else {
            text.draw(&text.drawable(name), x, baseline, 22.0, color);
        }
    }

    /// 表の高さ。見出しと、知らせか上位の行と、自分の順位の行の分。
    pub fn height(rows: usize) -> f32 {
        ROW_HEIGHT * (rows as f32 + 2.0)
    }

    /// `top` を見出しの上端にして、`center_x` を中心に描く。上位は `rows` 行まで出す。
    pub fn draw(&self, text: &Text, center_x: f32, top: f32, rows: usize) {
        let left = center_x - WIDTH / 2.0;
        let mut baseline = top + ROW_HEIGHT - 6.0;
        text.draw_centered("今日のランキング", center_x, baseline, 22.0, HEADER_COLOR);
        for line in lines(&self.status, rows) {
            baseline += ROW_HEIGHT;
            match line {
                Line::Note(note) => text.draw_centered(note, center_x, baseline, 22.0, NOTE_COLOR),
                Line::Row {
                    rank,
                    name,
                    score,
                    mine,
                } => {
                    let color = if mine { MINE_COLOR } else { ROW_COLOR };
                    draw_right(
                        text,
                        &format!("{rank}位"),
                        left + RANK_RIGHT,
                        baseline,
                        color,
                    );
                    self.draw_name(text, &name, left + NAME_LEFT, baseline, color);
                    draw_right(text, &score.to_string(), left + WIDTH, baseline, color);
                }
                Line::Mine(rank) => text.draw_centered(
                    &format!("あなたは {rank}位"),
                    center_x,
                    baseline,
                    22.0,
                    MINE_COLOR,
                ),
            }
        }
    }
}

fn draw_right(text: &Text, s: &str, right: f32, baseline: f32, color: Color) {
    let width = text.measure(s, 22.0).width;
    text.draw(s, right - width, baseline, 22.0, color);
}

/// 表の中身。見出しの下に並べる順。
fn lines(status: &RankingStatus, rows: usize) -> Vec<Line> {
    match status {
        RankingStatus::NotRequested | RankingStatus::Loading => {
            vec![Line::Note("読み込み中…")]
        }
        RankingStatus::Failed => vec![Line::Note("読み込めませんでした")],
        RankingStatus::Ready(ranking) => ranking_lines(ranking, rows),
    }
}

fn ranking_lines(ranking: &Ranking, rows: usize) -> Vec<Line> {
    if ranking.entries.is_empty() {
        return vec![Line::Note("まだ誰も記録していません")];
    }
    let mut lines: Vec<Line> = ranking
        .entries
        .iter()
        .take(rows)
        .map(|entry| Line::Row {
            rank: entry.rank,
            name: entry.name.clone(),
            score: entry.score,
            mine: Some(entry.rank) == ranking.my_rank,
        })
        .collect();
    let shown_mine = lines
        .iter()
        .any(|line| matches!(line, Line::Row { mine: true, .. }));
    if let (Some(rank), false) = (ranking.my_rank, shown_mine) {
        lines.push(Line::Mine(rank));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use roommate_sdk::RankingEntry;

    fn ranking(len: u32, my_rank: Option<u32>) -> Ranking {
        Ranking {
            entries: (1..=len)
                .map(|rank| RankingEntry {
                    rank,
                    name: format!("名無し#{rank:04x}"),
                    score: 1000 - rank,
                })
                .collect(),
            my_rank,
        }
    }

    #[test]
    fn marks_my_row_when_it_is_in_the_top() {
        let lines = ranking_lines(&ranking(10, Some(2)), MAX_ROWS);
        assert_eq!(lines.len(), MAX_ROWS);
        assert!(matches!(
            lines[1],
            Line::Row {
                rank: 2,
                mine: true,
                ..
            }
        ));
        assert!(matches!(lines[0], Line::Row { mine: false, .. }));
    }

    #[test]
    fn adds_my_rank_below_the_top() {
        let lines = ranking_lines(&ranking(50, Some(37)), 3);
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[3], Line::Mine(37));
    }

    #[test]
    fn says_so_when_nobody_has_played() {
        assert_eq!(
            ranking_lines(&ranking(0, None), MAX_ROWS),
            vec![Line::Note("まだ誰も記録していません")]
        );
        // 自分の記録が無いとき（データを消した後など）は、自分の行を足さない。
        assert_eq!(ranking_lines(&ranking(3, None), MAX_ROWS).len(), 3);
    }

    #[test]
    fn notes_loading_and_failure() {
        assert_eq!(
            lines(&RankingStatus::Loading, MAX_ROWS),
            vec![Line::Note("読み込み中…")]
        );
        assert_eq!(
            lines(&RankingStatus::Failed, MAX_ROWS),
            vec![Line::Note("読み込めませんでした")]
        );
    }

    #[test]
    fn fits_in_the_reserved_height() {
        // 上位の行と自分の行を合わせても、見出しを含めて height の行数を超えない。
        let lines = ranking_lines(&ranking(50, Some(40)), MAX_ROWS);
        assert!((lines.len() + 1) as f32 * ROW_HEIGHT <= Board::height(MAX_ROWS));
    }
}
