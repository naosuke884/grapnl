//! ゲームオーバーの画面に出す、今日のコースのランキング。
//! ゲームオーバーになったらスコアを送ってから読み始め（`roommate_sdk::request_ranking`）、
//! 読み終えたら上位と自分の順位を出す。送ったばかりのスコアも順位に入る。
//! 名前は `names::NameFont` で描く。

use macroquad::prelude::*;
use roommate_sdk::{Ranking, RankingStatus};

use crate::names::NameFont;
use crate::text::{self, Fonts, Text};

/// 上位から出す行数の上限。画面が低いときはこれより減らす。
pub const MAX_ROWS: usize = 5;
/// 1 行の高さ。文字は `text::MEDIUM` の大きさで描く。
pub const ROW_HEIGHT: f32 = 40.0;
/// 見出しのフォントの数字は本文より下端が高く見えるので、少し下げて行の文字とそろえる。
const HEADING_DROP: f32 = 2.0;
/// 表の横幅。順位・名前・スコアを並べる。
pub const WIDTH: f32 = 640.0;
/// 順位の列の右端（表の左端から）。
const RANK_RIGHT: f32 = 80.0;
/// 名前の列の左端（表の左端から）。
const NAME_LEFT: f32 = 112.0;
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

pub struct Board {
    status: RankingStatus,
}

impl Default for Board {
    fn default() -> Self {
        Self {
            status: RankingStatus::NotRequested,
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
    /// 名前を `names` に見せ、描けない名前があれば名前用のフォントを読み始めさせる。
    pub fn update(&mut self, fonts: &Fonts, names: &mut NameFont) {
        if self.status == RankingStatus::Loading {
            match roommate_sdk::ranking() {
                // 頼んだ直後に頼んでいないことになるのは、データを消したときなど。読み込み中のままにはしない。
                RankingStatus::NotRequested => self.status = RankingStatus::Failed,
                status => self.status = status,
            }
        }
        if let RankingStatus::Ready(ranking) = &self.status {
            for entry in &ranking.entries {
                names.want(fonts, &entry.name);
            }
        }
    }

    /// 表の高さ。見出しと、知らせか上位の行と、自分の順位の行の分。
    pub fn height(rows: usize) -> f32 {
        ROW_HEIGHT * (rows as f32 + 2.0)
    }

    /// 今の中身を `rows` 行まで出したときの高さ。見出しと、実際に出す行の分。
    /// まだ誰もいないときなどは、`height` より低くなる。
    pub fn used_height(&self, rows: usize) -> f32 {
        ROW_HEIGHT * (lines(&self.status, rows).len() as f32 + 1.0)
    }

    /// `top` を見出しの上端にして、`center_x` を中心に描く。上位は `rows` 行まで出す。
    pub fn draw(&self, fonts: &Fonts, names: &NameFont, center_x: f32, top: f32, rows: usize) {
        let text = &fonts.body;
        let left = center_x - WIDTH / 2.0;
        let mut baseline = top + ROW_HEIGHT - 8.0;
        text.draw_centered(
            "Today's ranking",
            center_x,
            baseline,
            text::MEDIUM,
            HEADER_COLOR,
        );
        for line in lines(&self.status, rows) {
            baseline += ROW_HEIGHT;
            match line {
                Line::Note(note) => {
                    text.draw_centered(note, center_x, baseline, text::MEDIUM, NOTE_COLOR)
                }
                Line::Row {
                    rank,
                    name,
                    score,
                    mine,
                } => {
                    let color = if mine { MINE_COLOR } else { ROW_COLOR };
                    // 順位とスコアの数字は、見分けやすい見出しのフォントで描く。
                    draw_right(
                        &fonts.heading,
                        &format!("#{rank}"),
                        left + RANK_RIGHT,
                        baseline + HEADING_DROP,
                        color,
                    );
                    let (font, name, size) =
                        names.pick(fonts, &name, text::HEAD_MEDIUM, text::MEDIUM);
                    let drop = if size == text::HEAD_MEDIUM {
                        HEADING_DROP
                    } else {
                        0.0
                    };
                    font.draw(&name, left + NAME_LEFT, baseline + drop, size, color);
                    draw_right(
                        &fonts.heading,
                        &score.to_string(),
                        left + WIDTH,
                        baseline + HEADING_DROP,
                        color,
                    );
                }
                Line::Mine(rank) => text.draw_centered(
                    &format!("You: #{rank}"),
                    center_x,
                    baseline,
                    text::MEDIUM,
                    MINE_COLOR,
                ),
            }
        }
    }
}

fn draw_right(heading: &Text, s: &str, right: f32, baseline: f32, color: Color) {
    let width = heading.measure(s, text::HEAD_MEDIUM).width;
    heading.draw(s, right - width, baseline, text::HEAD_MEDIUM, color);
}

/// 表の中身。見出しの下に並べる順。
fn lines(status: &RankingStatus, rows: usize) -> Vec<Line> {
    match status {
        RankingStatus::NotRequested | RankingStatus::Loading => {
            vec![Line::Note("Loading...")]
        }
        RankingStatus::Failed => vec![Line::Note("Couldn't load the ranking")],
        RankingStatus::Ready(ranking) => ranking_lines(ranking, rows),
    }
}

fn ranking_lines(ranking: &Ranking, rows: usize) -> Vec<Line> {
    if ranking.entries.is_empty() {
        return vec![Line::Note("No scores yet today")];
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
            vec![Line::Note("No scores yet today")]
        );
        // 自分の記録が無いとき（データを消した後など）は、自分の行を足さない。
        assert_eq!(ranking_lines(&ranking(3, None), MAX_ROWS).len(), 3);
    }

    #[test]
    fn notes_loading_and_failure() {
        assert_eq!(
            lines(&RankingStatus::Loading, MAX_ROWS),
            vec![Line::Note("Loading...")]
        );
        assert_eq!(
            lines(&RankingStatus::Failed, MAX_ROWS),
            vec![Line::Note("Couldn't load the ranking")]
        );
    }

    #[test]
    fn fits_in_the_reserved_height() {
        // 上位の行と自分の行を合わせても、見出しを含めて height の行数を超えない。
        let lines = ranking_lines(&ranking(50, Some(40)), MAX_ROWS);
        assert!((lines.len() + 1) as f32 * ROW_HEIGHT <= Board::height(MAX_ROWS));
    }
}
