//! 遊ぶ画面の上に重ねるもの。一時停止の画面・ゲームオーバーの画面・右上の「MENU」ボタン・縦向きの案内。

use macroquad::prelude::*;

use crate::game::Game;
use crate::page;
use crate::ranking::{self, Board};
use crate::text::{self, Fonts, Text, PANEL_COLOR};
use crate::{names, volume, RETRY_DELAY};

/// 一時停止の画面の真ん中に出す、遊び方の説明。キーと、その働き。遊んでいる間は出さず、ここにだけ出す。
const CONTROLS: [(&str, &str); 4] = [
    ("Click / Tap", "Shoot / retract the wire"),
    ("Wire an enemy", "Destroy it and bounce up"),
    ("Red wall", "Keeps speeding up. Run!"),
    ("Esc / MENU", "Pause / resume"),
];
/// 遊び方の説明の 1 行の高さ。文字は `text::MEDIUM`。
const CONTROL_ROW: f32 = 44.0;
/// 遊び方の説明の囲みの、上下の余白。
const CONTROLS_PAD: Vec2 = vec2(0.0, 20.0);
/// 遊び方の説明の見出しの高さ。
const CONTROLS_HEADING: f32 = 32.0;
/// 遊び方の説明の囲みの幅。
const CONTROLS_WIDTH: f32 = 720.0;
/// キーの列と働きの列の間。
const CONTROLS_GUTTER: f32 = 28.0;
/// 一時停止の画面の、上から並べる間隔。
const PAUSE_TITLE: f32 = text::LARGE;
/// 「PAUSED」のベースラインから、遊び方の説明の囲みの上端まで。
const PAUSE_CONTROLS_GAP: f32 = 28.0;
/// 遊び方の説明の囲みの下端から、音量の囲みの上端まで。
const PAUSE_PANEL_GAP: f32 = 24.0;
/// 音量の囲みの下端から、音量のキーの案内のベースラインまで。
const PAUSE_VOLUME_KEYS: f32 = 28.0;
/// 音量のキーの案内のベースラインから、メニューのボタンの上端まで。
const PAUSE_MENU_GAP: f32 = 24.0;
const MENU_BUTTON: Vec2 = vec2(248.0, 44.0);
const MENU_GAP: f32 = 16.0;
/// メニューのボタンの下端から、再開の案内のベースラインまで。
const PAUSE_RESUME: f32 = 48.0;

/// 一時停止の画面の下に並べるボタン。ページの上にあったヘッダーの役目をここへ移した。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuItem {
    ChangeName,
    DeleteData,
    BackToSite,
}

impl MenuItem {
    const ALL: [Self; 3] = [Self::ChangeName, Self::DeleteData, Self::BackToSite];

    fn label(self) -> &'static str {
        match self {
            Self::ChangeName => "Change name",
            Self::DeleteData => "Delete my data",
            Self::BackToSite => "← roommate",
        }
    }
}
/// 背景を暗くする濃さ。後ろの画面の文字が読めて気が散らないくらいにする。
const DIM: Color = Color::new(0.0, 0.0, 0.0, 0.72);

/// 一時停止の画面の置き方。描くときと、音量の操作のクリックを見るときで同じものを使う。
pub(crate) struct PauseLayout {
    title: f32,
    /// 遊び方の説明の囲み。
    controls: Rect,
    pub(crate) panel: volume::Panel,
    volume_keys: f32,
    pub(crate) menu: [(MenuItem, Rect); 3],
    resume: f32,
}

fn controls_height() -> f32 {
    CONTROLS_PAD.y * 2.0 + CONTROLS_HEADING + CONTROL_ROW * CONTROLS.len() as f32
}

pub(crate) fn pause_layout(screen: Vec2) -> PauseLayout {
    let center_x = screen.x / 2.0;
    // 音量の囲みの大きさを知るため、いったん原点に置いて測る。
    let probe = volume::Panel::new(0.0, 0.0).frame();
    let height = PAUSE_TITLE
        + PAUSE_CONTROLS_GAP
        + controls_height()
        + PAUSE_PANEL_GAP
        + probe.h
        + PAUSE_VOLUME_KEYS
        + PAUSE_MENU_GAP
        + MENU_BUTTON.y
        + PAUSE_RESUME;
    let top = ((screen.y - height) / 2.0).max(16.0);
    let title = top + PAUSE_TITLE;
    // 遊び方の説明の囲みは、表（一番長い行で約 620）が余白込みで収まる幅にし、真ん中の主役にする。
    let width = CONTROLS_WIDTH.min(screen.x - 32.0);
    let controls = Rect::new(
        center_x - width / 2.0,
        title + PAUSE_CONTROLS_GAP,
        width,
        controls_height(),
    );
    // `Panel::new` の上端は枠の内側の行なので、枠の上端が囲みの下に来るようずらす。
    let panel = volume::Panel::new(center_x, controls.bottom() + PAUSE_PANEL_GAP - probe.y);
    let volume_keys = panel.frame().bottom() + PAUSE_VOLUME_KEYS;
    let menu_top = volume_keys + PAUSE_MENU_GAP;
    let menu = menu_row(center_x, menu_top);
    PauseLayout {
        title,
        controls,
        panel,
        volume_keys,
        menu,
        resume: menu_top + MENU_BUTTON.y + PAUSE_RESUME,
    }
}

/// 縦長の画面で、横向きにするよう知らせる。その間ゲームは止まっている。
pub(crate) fn draw_rotate_hint(fonts: &Fonts, screen: Vec2) {
    draw_rectangle(
        0.0,
        0.0,
        screen.x,
        screen.y,
        Color::new(0.0, 0.0, 0.0, 0.85),
    );
    let center = screen / 2.0;
    fonts
        .heading
        .draw_centered("ROTATE", center.x, center.y - 24.0, text::HEAD_LARGE, WHITE);
    fonts.body.draw_centered(
        "Turn your phone sideways to play",
        center.x,
        center.y + 40.0,
        text::MEDIUM,
        Color::from_rgba(200, 206, 218, 255),
    );
}

/// 遊んでいる間、右上に置く「MENU」ボタン。押すと一時停止の画面を開く（Esc と同じ）。
pub(crate) fn menu_button(screen: Vec2) -> Rect {
    let size = vec2(104.0, 40.0);
    Rect::new(screen.x - 12.0 - size.x, 10.0, size.x, size.y)
}

/// 右上の「MENU」ボタンを描く。ポインターが乗っていれば明るくする。
pub(crate) fn draw_menu_button(heading: &Text, screen: Vec2, pointer: Vec2) {
    let button = menu_button(screen);
    let hovered = button.contains(pointer);
    let fill = if hovered {
        Color::from_rgba(44, 50, 64, 220)
    } else {
        Color::from_rgba(20, 24, 32, 170)
    };
    draw_rectangle(button.x, button.y, button.w, button.h, fill);
    draw_rectangle_lines(
        button.x,
        button.y,
        button.w,
        button.h,
        2.0,
        Color::from_rgba(90, 98, 116, 255),
    );
    heading.draw_y_centered_middle(
        "MENU",
        button.center(),
        text::HEAD_SMALL,
        Color::from_rgba(200, 206, 218, 255),
    );
}

/// メニューのボタンを、`center_x` を中心に上端を `top` にして横に並べる。
fn menu_row(center_x: f32, top: f32) -> [(MenuItem, Rect); 3] {
    let count = MenuItem::ALL.len() as f32;
    let left = center_x - (MENU_BUTTON.x * count + MENU_GAP * (count - 1.0)) / 2.0;
    let mut i = 0.0;
    MenuItem::ALL.map(|item| {
        let x = left + i * (MENU_BUTTON.x + MENU_GAP);
        i += 1.0;
        (item, Rect::new(x, top, MENU_BUTTON.x, MENU_BUTTON.y))
    })
}

/// `pointer` が乗っているメニューのボタン。
pub(crate) fn menu_item_at(menu: &[(MenuItem, Rect); 3], pointer: Vec2) -> Option<MenuItem> {
    menu.iter()
        .find(|(_, rect)| rect.contains(pointer))
        .map(|(item, _)| *item)
}

pub(crate) fn run_menu_item(item: MenuItem) {
    match item {
        MenuItem::ChangeName => roommate_sdk::open_name_dialog(),
        MenuItem::DeleteData => page::open_delete_dialog(),
        MenuItem::BackToSite => page::go_to_site(),
    }
}

/// メニューのボタンを描く。
fn draw_menu(fonts: &Fonts, menu: &[(MenuItem, Rect); 3], pointer: Vec2) {
    for &(item, rect) in menu {
        let hovered = rect.contains(pointer);
        let (fill, border) = match (item, hovered) {
            (MenuItem::DeleteData, true) => (
                Color::from_rgba(70, 32, 38, 255),
                Color::from_rgba(200, 90, 100, 255),
            ),
            (_, true) => (Color::from_rgba(44, 50, 64, 255), WHITE),
            (_, false) => (
                Color::from_rgba(28, 32, 42, 255),
                Color::from_rgba(70, 78, 96, 255),
            ),
        };
        draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
        draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, border);
        fonts
            .heading
            .draw_y_centered_middle(item.label(), rect.center(), text::HEAD_SMALL, WHITE);
    }
}

pub(crate) fn draw_paused(fonts: &Fonts, screen: Vec2, layout: &PauseLayout, pointer: Vec2) {
    let text = &fonts.body;
    let heading = &fonts.heading;
    draw_rectangle(0.0, 0.0, screen.x, screen.y, DIM);
    let center_x = screen.x / 2.0;
    heading.draw_centered("PAUSED", center_x, layout.title, text::HEAD_LARGE, WHITE);

    let area = layout.controls;
    draw_rectangle(area.x, area.y, area.w, area.h, PANEL_COLOR);
    heading.draw_centered(
        "HOW TO PLAY",
        center_x,
        area.y + CONTROLS_PAD.y + CONTROLS_HEADING - 10.0,
        text::HEAD_SMALL,
        Color::from_rgba(150, 200, 235, 255),
    );
    // キーは右揃え、働きは左揃えにして、表の中で向かい合わせる。列の境目は一番長いキーで決める。
    let key_width = CONTROLS
        .iter()
        .map(|(key, _)| text.measure(key, text::MEDIUM).width)
        .fold(0.0, f32::max);
    let action_width = CONTROLS
        .iter()
        .map(|(_, action)| text.measure(action, text::MEDIUM).width)
        .fold(0.0, f32::max);
    let table_left = center_x - (key_width + CONTROLS_GUTTER + action_width) / 2.0;
    let divider = table_left + key_width;
    let key_color = Color::from_rgba(255, 210, 110, 255);
    let action_color = Color::from_rgba(225, 230, 240, 255);
    for (i, (key, action)) in CONTROLS.iter().enumerate() {
        let y = area.y + CONTROLS_PAD.y + CONTROLS_HEADING + CONTROL_ROW * (i as f32 + 1.0) - 12.0;
        text.draw_right(key, divider, y, text::MEDIUM, key_color);
        text.draw(
            action,
            divider + CONTROLS_GUTTER,
            y,
            text::MEDIUM,
            action_color,
        );
    }

    heading.draw_centered(
        "← →  Volume     M  Mute",
        center_x,
        layout.volume_keys,
        text::HEAD_SMALL,
        Color::from_rgba(150, 160, 175, 255),
    );
    draw_menu(fonts, &layout.menu, pointer);
    text.draw_centered(
        "Click or Esc to resume",
        center_x,
        layout.resume,
        text::MEDIUM,
        WHITE,
    );
}

/// 自己ベストの表示。まだ無いとき（初めて遊ぶ前やデータを消した後）は「なし」と出し、0 点と見分ける。
pub(crate) fn best_label(best: Option<u32>) -> String {
    match best {
        Some(best) => format!("Best {best}"),
        None => "Best -".to_owned(),
    }
}

/// ゲームオーバーの画面の、上から並べる行の間隔。それぞれ前の行のベースラインからの距離。
const OVER_TITLE: f32 = text::LARGE;
const OVER_REASON: f32 = 56.0;
const OVER_SCORE: f32 = 48.0;
/// 自己ベストの行。更新したときは「NEW BEST!」を大きく出す。
const OVER_BEST: f32 = 40.0;
/// 自己ベストの行のベースラインから、ランキングの見出しの上端まで。
const OVER_RANKING_GAP: f32 = 36.0;
/// ランキングの囲みの、表の外の余白。
const RANKING_PAD: Vec2 = vec2(40.0, 22.0);
/// ランキングの表の下端から、「Retry」ボタンの上端まで。
const OVER_RETRY_GAP: f32 = 36.0;
const RETRY_SIZE: Vec2 = vec2(200.0, 52.0);
/// 「Retry」ボタンの下端から、メニューのボタンの上端まで。
const OVER_MENU_GAP: f32 = 16.0;
/// 画面の上下に残す余白。
const OVER_MARGIN: f32 = 16.0;

/// ゲームオーバーの画面の、ランキングを除いた高さ。
fn game_over_fixed_height() -> f32 {
    OVER_TITLE
        + OVER_REASON
        + OVER_SCORE
        + OVER_BEST
        + OVER_RANKING_GAP
        + OVER_RETRY_GAP
        + RETRY_SIZE.y
        + OVER_MENU_GAP
        + MENU_BUTTON.y
}

/// 画面の高さに収まる、ランキングの上位の行数。
fn ranking_rows(screen_h: f32) -> usize {
    let room = screen_h - OVER_MARGIN * 2.0 - game_over_fixed_height() - Board::height(0);
    ((room / ranking::ROW_HEIGHT).floor().max(0.0) as usize).min(ranking::MAX_ROWS)
}

/// ゲームオーバーの画面の置き方。描くときと、「Retry」ボタンのクリックを見るときで同じものを使う。
pub(crate) struct GameOverLayout {
    /// 一番上の「GAME OVER」の上端。
    top: f32,
    /// ランキングの上位の行数。
    rows: usize,
    pub(crate) retry: Rect,
    pub(crate) menu: [(MenuItem, Rect); 3],
}

pub(crate) fn game_over_layout(screen: Vec2, board: &Board) -> GameOverLayout {
    // 行が収まる分だけ出し、全体を縦の中央に置く。
    let rows = ranking_rows(screen.y);
    // 上端は表がいっぱいのときで決め、ランキングが届いても見出しが動かないようにする。
    let full = game_over_fixed_height() + Board::height(rows);
    let top = ((screen.y - full) / 2.0).max(OVER_MARGIN);
    // 行が少ないときは表を縮め、「Retry」ボタンを上へ詰める。
    let menu_top = top + game_over_fixed_height() + board.used_height(rows) - MENU_BUTTON.y;
    let retry_top = menu_top - OVER_MENU_GAP - RETRY_SIZE.y;
    GameOverLayout {
        top,
        rows,
        retry: Rect::new(
            screen.x / 2.0 - RETRY_SIZE.x / 2.0,
            retry_top,
            RETRY_SIZE.x,
            RETRY_SIZE.y,
        ),
        menu: menu_row(screen.x / 2.0, menu_top),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_game_over(
    fonts: &Fonts,
    names: &names::NameFont,
    game: &Game,
    board: &Board,
    since_over: f32,
    new_best: bool,
    best: &str,
    pointer: Vec2,
    screen: Vec2,
) {
    let text = &fonts.body;
    let heading = &fonts.heading;
    draw_rectangle(0.0, 0.0, screen.x, screen.y, DIM);
    let center_x = screen.x / 2.0;
    let layout = game_over_layout(screen, board);
    let mut y = layout.top + OVER_TITLE;
    heading.draw_centered("GAME OVER", center_x, y, text::HEAD_LARGE, WHITE);
    y += OVER_REASON;
    if let Some(reason) = game.end_reason {
        text.draw_centered(
            reason.label(),
            center_x,
            y,
            text::MEDIUM,
            Color::from_rgba(255, 140, 120, 255),
        );
    }
    y += OVER_SCORE;
    text.draw_centered(
        &format!(
            "Score {} ({:.0} m, {} kills)",
            game.score(),
            game.distance(),
            game.kills
        ),
        center_x,
        y,
        text::MEDIUM,
        WHITE,
    );
    y += OVER_BEST;
    if new_best {
        // 自己ベストを更新したときは、明るい黄色で明滅させて目立たせる。
        let pulse = (since_over * 6.0).sin() * 0.5 + 0.5;
        let color = Color::new(1.0, 0.85 + 0.15 * pulse, 0.3 + 0.5 * pulse, 1.0);
        heading.draw_centered("NEW BEST!", center_x, y, text::HEAD_MEDIUM, color);
    } else {
        text.draw_centered(
            best,
            center_x,
            y - 8.0,
            text::SMALL,
            Color::from_rgba(150, 160, 175, 255),
        );
    }
    y += OVER_RANKING_GAP;
    // ランキングは囲みの中に出し、上の自分の結果と分けて見せる。
    let pad = RANKING_PAD;
    draw_rectangle(
        center_x - ranking::WIDTH / 2.0 - pad.x,
        y - pad.y,
        ranking::WIDTH + pad.x * 2.0,
        board.used_height(layout.rows) + pad.y * 2.0,
        PANEL_COLOR,
    );
    board.draw(fonts, names, center_x, y, layout.rows);
    draw_retry_button(heading, layout.retry, since_over >= RETRY_DELAY, pointer);
    draw_menu(fonts, &layout.menu, pointer);
}

/// 「Retry」ボタン。終わった直後の `RETRY_DELAY` の間は暗くして、押せないことを見せる。
fn draw_retry_button(text: &Text, rect: Rect, ready: bool, pointer: Vec2) {
    let hovered = ready && rect.contains(pointer);
    let fill = match (ready, hovered) {
        (false, _) => Color::from_rgba(40, 44, 54, 255),
        (true, false) => Color::from_rgba(60, 120, 80, 255),
        (true, true) => Color::from_rgba(80, 160, 105, 255),
    };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
    if hovered {
        draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, WHITE);
    }
    let color = if ready {
        WHITE
    } else {
        Color::from_rgba(120, 126, 140, 255)
    };
    text.draw_y_centered_middle("Retry", rect.center(), text::HEAD_MEDIUM, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_and_menu_sit_below_the_ranking_on_screen() {
        for height in [720.0, 600.0] {
            let screen = vec2(1280.0, height);
            let layout = game_over_layout(screen, &Board::default());
            let menu_top = layout.menu[0].1.y;
            assert!((layout.retry.bottom() + OVER_MENU_GAP - menu_top).abs() < 1e-3);
            assert!(layout.menu[0].1.bottom() <= height);
        }
    }

    #[test]
    fn game_over_fits_the_screen() {
        assert_eq!(ranking_rows(720.0), ranking::MAX_ROWS);
        let rows = ranking_rows(600.0);
        assert!(rows < ranking::MAX_ROWS);
        assert!(game_over_fixed_height() + Board::height(rows) + OVER_MARGIN * 2.0 <= 600.0);
        assert_eq!(ranking_rows(100.0), 0);
    }
}
