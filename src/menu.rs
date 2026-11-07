//! 遊んでいない間に出す画面。タイトルと、自分のデータを消す確認・結果の画面。
//!
//! 画面の配置とクリックの判定は、文字の幅を測る関数を受け取って決める。
//! フォントを読まずにテストできるよう、描画とは分けておく。

use macroquad::prelude::*;
use roommate_sdk::DeleteStatus;

use crate::text::Text;

/// 遊んでいない間に出している画面。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    /// 開いたときの画面。後ろに吊られた自機を見せたまま、クリックで始める。
    Title,
    /// 「データを消す」を押した後の確認。
    ConfirmDelete,
    /// 「消す」を押した後。消している間と、消せたか失敗したかを出す。
    DeleteResult,
}

/// 画面のクリックで起きること。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// 何もしない。
    None,
    /// ゲームを始める。
    Start,
    /// 確認の画面を出す。
    OpenConfirm,
    /// データを消し始め、結果の画面を出す。
    Delete,
    /// タイトルへ戻る。
    BackToTitle,
}

/// 小さいボタンの文字の大きさ。text.rs の SIZES にある大きさを使う。
const SMALL: f32 = 22.0;
/// 確認の画面のボタンの文字の大きさ。
const LARGE: f32 = 28.0;
/// ボタンの文字の周りの余白。
const PAD: Vec2 = vec2(14.0, 8.0);
/// 確認の画面で、2 つのボタンの間をあける幅。
const GAP: f32 = 32.0;
/// 画面の端からボタンまでの距離。
const MARGIN: f32 = 16.0;

/// タイトルとゲームオーバーの画面の右下に置く、小さいボタンの文字。
pub const DELETE_LABEL: &str = "データを消す";
pub const TITLE_LABEL: &str = "タイトルへ";

/// 押せる範囲と文字。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Button {
    pub rect: Rect,
    pub label: &'static str,
    pub font_size: f32,
    pub action: Action,
}

impl Button {
    fn new(
        label: &'static str,
        font_size: f32,
        action: Action,
        measure: &dyn Fn(&str, f32) -> f32,
    ) -> Self {
        let size = vec2(measure(label, font_size), font_size) + PAD * 2.0;
        Self {
            rect: Rect::new(0.0, 0.0, size.x, size.y),
            label,
            font_size,
            action,
        }
    }

    fn at(mut self, x: f32, y: f32) -> Self {
        self.rect.x = x;
        self.rect.y = y;
        self
    }

    pub fn contains(&self, point: Vec2) -> bool {
        self.rect.contains(point)
    }

    /// 文字は日本語の字面が上下の真ん中に来るよう、ベースラインを少し下げて置く。
    fn draw(&self, text: &Text, hovered: bool, color: Color) {
        let color = if hovered {
            color
        } else {
            Color {
                a: color.a * 0.7,
                ..color
            }
        };
        let r = self.rect;
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, color);
        let baseline = r.y + r.h / 2.0 + self.font_size * 0.36;
        text.draw_centered(self.label, r.x + r.w / 2.0, baseline, self.font_size, color);
    }
}

/// 画面の右下に置く小さいボタン。遊び方の表示（左下）と重ならない。
pub fn corner_button(
    label: &'static str,
    action: Action,
    screen: Vec2,
    measure: &dyn Fn(&str, f32) -> f32,
) -> Button {
    let button = Button::new(label, SMALL, action, measure);
    button.at(
        screen.x - MARGIN - button.rect.w,
        screen.y - MARGIN - button.rect.h,
    )
}

/// その画面で押せるボタン。データを消し終えた後は、タイトルに「データを消す」を出さない。
pub fn buttons(
    menu: Menu,
    status: DeleteStatus,
    screen: Vec2,
    measure: &dyn Fn(&str, f32) -> f32,
) -> Vec<Button> {
    let center = screen / 2.0;
    // 確認と結果の画面は、ボタンを中央の文の下に横に並べる。
    let row = |list: Vec<Button>| -> Vec<Button> {
        let width: f32 = list.iter().map(|b| b.rect.w).sum::<f32>()
            + GAP * (list.len().saturating_sub(1)) as f32;
        let mut x = center.x - width / 2.0;
        list.into_iter()
            .map(|b| {
                let placed = b.at(x, center.y + 110.0);
                x += b.rect.w + GAP;
                placed
            })
            .collect()
    };
    match (menu, status) {
        (Menu::Title, DeleteStatus::Done) => Vec::new(),
        (Menu::Title, _) => vec![corner_button(
            DELETE_LABEL,
            Action::OpenConfirm,
            screen,
            measure,
        )],
        (Menu::ConfirmDelete, _) => row(vec![
            Button::new("消す", LARGE, Action::Delete, measure),
            Button::new("やめる", LARGE, Action::BackToTitle, measure),
        ]),
        (Menu::DeleteResult, DeleteStatus::Done) => row(vec![Button::new(
            "もどる",
            LARGE,
            Action::BackToTitle,
            measure,
        )]),
        (Menu::DeleteResult, DeleteStatus::Failed) => row(vec![
            Button::new("もう一度", LARGE, Action::Delete, measure),
            Button::new("やめる", LARGE, Action::BackToTitle, measure),
        ]),
        // 消している間は押せるものを出さない。
        (Menu::DeleteResult, _) => Vec::new(),
    }
}

/// `point` をクリックしたときに起きること。ボタンの上なら、そのボタンの動きだけを返す。
/// タイトルではボタンの外をクリックすると始める。ほかの画面ではボタンの外は何もしない。
pub fn click(menu: Menu, point: Vec2, buttons: &[Button]) -> Action {
    if let Some(button) = buttons.iter().find(|b| b.contains(point)) {
        return button.action;
    }
    match menu {
        Menu::Title => Action::Start,
        Menu::ConfirmDelete | Menu::DeleteResult => Action::None,
    }
}

/// 右下の小さいボタンの色。遊び方の表示と同じく控えめにする。
const CORNER: Color = Color::new(0.59, 0.63, 0.69, 1.0);

/// 右下の小さいボタンだけを描く。ゲームオーバーの画面の「タイトルへ」に使う。
pub fn draw_corner_button(text: &Text, button: &Button, mouse: Vec2) {
    button.draw(text, button.contains(mouse), CORNER);
}

/// タイトルに出す操作の説明。キーと、それで起きること。
pub const CONTROLS: [(&str, &str); 3] = [
    ("左クリック", "ワイヤーを撃つ・戻す"),
    ("右クリック", "ブースト"),
    ("Esc", "一時停止"),
];
/// 操作の説明の文字の大きさ。
const CONTROL_SIZE: f32 = 22.0;
/// 操作の説明で、キーとその説明の間をあける幅。
const CONTROL_GAP: f32 = 24.0;
/// 操作の説明の行の間隔。
const CONTROL_LINE: f32 = 34.0;

/// 操作の説明の 1 行。キーは右に揃え、説明は左に揃えて、2 列に並べる。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ControlRow {
    pub key: &'static str,
    pub action: &'static str,
    /// キーの文字の左端。
    pub key_x: f32,
    /// 説明の文字の左端。
    pub action_x: f32,
    pub baseline: f32,
}

/// タイトルの画面の配置。高さはどれも文字のベースラインで、操作の説明のほかは横の中央に揃える。
#[derive(Debug, Clone, PartialEq)]
pub struct TitleLayout {
    pub logo: f32,
    pub course: f32,
    pub best: f32,
    pub controls: Vec<ControlRow>,
    pub start: f32,
    pub notice: f32,
}

/// タイトルの画面の配置。上から、ロゴ、今日のコース、自己ベスト、操作の説明、「クリックでスタート」と並べる。
/// 操作の説明は、2 列を合わせた幅で中央に置く。上のほうに吊られている自機と重ならないよう、全体を中央より少し下げる。
pub fn title_layout(screen: Vec2, measure: &dyn Fn(&str, f32) -> f32) -> TitleLayout {
    let center = screen / 2.0;
    let widest = |column: fn(&(&'static str, &'static str)) -> &'static str| {
        CONTROLS
            .iter()
            .map(|row| measure(column(row), CONTROL_SIZE))
            .fold(0.0, f32::max)
    };
    let key_width = widest(|row| row.0);
    let width = key_width + CONTROL_GAP + widest(|row| row.1);
    let left = center.x - width / 2.0;
    let top = center.y + 30.0;
    let controls = CONTROLS
        .iter()
        .enumerate()
        .map(|(i, &(key, action))| ControlRow {
            key,
            action,
            key_x: left + key_width - measure(key, CONTROL_SIZE),
            action_x: left + key_width + CONTROL_GAP,
            baseline: top + i as f32 * CONTROL_LINE,
        })
        .collect();
    TitleLayout {
        logo: center.y - 110.0,
        course: center.y - 64.0,
        best: center.y - 32.0,
        controls,
        start: center.y + 164.0,
        notice: center.y + 208.0,
    }
}

/// タイトルの画面に出す、今日のコースと自己ベストの文字。
pub struct TitleInfo<'a> {
    pub course: &'a str,
    pub best: &'a str,
}

/// 今日のコースの色。ゲームオーバーの画面のコースの行と同じ。
const COURSE: Color = Color::new(0.59, 0.78, 0.92, 1.0);

/// 文の色。
const BODY: Color = Color::new(0.75, 0.78, 0.83, 1.0);
/// 「消す」など、取り消せない操作の色。
const DANGER: Color = Color::new(1.0, 0.55, 0.5, 1.0);
/// データを消した後の注意の色。
pub const NOTICE: Color = Color::new(1.0, 0.8, 0.35, 1.0);

/// データを消した後に出しておく注意。結果の画面を閉じた後も、タイトルとゲームオーバーに出す。
pub const NOT_RECORDED: &str = "データを消したので、ページを開き直すまでスコアは記録されません";

pub fn draw(
    text: &Text,
    menu: Menu,
    status: DeleteStatus,
    title: &TitleInfo,
    buttons: &[Button],
    mouse: Vec2,
) {
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::from_rgba(0, 0, 0, if menu == Menu::Title { 110 } else { 190 }),
    );
    let center = vec2(screen_width(), screen_height()) / 2.0;
    let lines = |lines: &[&str], top: f32, color: Color| {
        for (i, line) in lines.iter().enumerate() {
            text.draw_centered(line, center.x, top + i as f32 * 34.0, 22.0, color);
        }
    };
    match menu {
        Menu::Title => {
            let layout = title_layout(center * 2.0, &|label, size| text.width(label, size));
            text.draw_centered("grapnl", center.x, layout.logo, 56.0, WHITE);
            text.draw_centered(title.course, center.x, layout.course, 22.0, COURSE);
            text.draw_centered(title.best, center.x, layout.best, 22.0, BODY);
            for row in &layout.controls {
                text.draw(row.key, row.key_x, row.baseline, CONTROL_SIZE, WHITE);
                text.draw(row.action, row.action_x, row.baseline, CONTROL_SIZE, BODY);
            }
            text.draw_centered("クリックでスタート", center.x, layout.start, 28.0, WHITE);
            if status == DeleteStatus::Done {
                text.draw_centered(NOT_RECORDED, center.x, layout.notice, 22.0, NOTICE);
            }
        }
        Menu::ConfirmDelete => {
            text.draw_centered(
                "データを消しますか",
                center.x,
                center.y - 110.0,
                28.0,
                WHITE,
            );
            lines(
                &[
                    "この端末の匿名 ID と、送ったスコアをすべて消します。",
                    "ID は roommate のゲームで共通なので、ほかのゲームのスコアも消えます。",
                    "自己ベストと順位も消え、元には戻せません。",
                    "消した後は、ページを開き直すまで遊んでもスコアは記録されません。",
                ],
                center.y - 60.0,
                BODY,
            );
        }
        Menu::DeleteResult => match status {
            DeleteStatus::Done => {
                text.draw_centered(
                    "データを消しました",
                    center.x,
                    center.y - 110.0,
                    28.0,
                    WHITE,
                );
                lines(
                    &[
                        "ほかのゲームの分も含め、スコアと匿名 ID を消しました。",
                        "自己ベストと順位も消えました。",
                        "ページを開き直すまで、遊んでもスコアは記録されません。",
                    ],
                    center.y - 60.0,
                    BODY,
                );
            }
            DeleteStatus::Failed => {
                text.draw_centered("消せませんでした", center.x, center.y - 110.0, 28.0, DANGER);
                lines(
                    &[
                        "データは残っています。",
                        "通信できるか確かめて、もう一度試してください。",
                    ],
                    center.y - 60.0,
                    BODY,
                );
            }
            DeleteStatus::Deleting | DeleteStatus::NotRequested => {
                text.draw_centered("消しています", center.x, center.y, 28.0, WHITE);
            }
        },
    }
    for button in buttons {
        let color = match button.action {
            Action::Delete => DANGER,
            Action::OpenConfirm => CORNER,
            _ => WHITE,
        };
        button.draw(text, button.contains(mouse), color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// フォントを読まずに測る。1 文字を大きさと同じ幅と見なす。
    fn measure(label: &str, size: f32) -> f32 {
        label.chars().count() as f32 * size
    }

    const SCREEN: Vec2 = vec2(1280.0, 720.0);

    fn center_of(button: &Button) -> Vec2 {
        button.rect.center()
    }

    fn find(buttons: &[Button], action: Action) -> Button {
        *buttons.iter().find(|b| b.action == action).unwrap()
    }

    #[test]
    fn clicking_delete_on_the_title_opens_the_confirm_without_starting() {
        let list = buttons(Menu::Title, DeleteStatus::NotRequested, SCREEN, &measure);
        let delete = find(&list, Action::OpenConfirm);
        assert_eq!(
            click(Menu::Title, center_of(&delete), &list),
            Action::OpenConfirm
        );
        assert_eq!(click(Menu::Title, SCREEN / 2.0, &list), Action::Start);
    }

    #[test]
    fn delete_button_sits_small_in_the_bottom_right_corner() {
        let list = buttons(Menu::Title, DeleteStatus::NotRequested, SCREEN, &measure);
        let rect = find(&list, Action::OpenConfirm).rect;
        assert!(rect.right() <= SCREEN.x && rect.bottom() <= SCREEN.y);
        assert!(rect.x > SCREEN.x / 2.0 && rect.y > SCREEN.y * 0.8);
    }

    #[test]
    fn confirm_offers_delete_and_cancel_and_ignores_other_clicks() {
        let list = buttons(
            Menu::ConfirmDelete,
            DeleteStatus::NotRequested,
            SCREEN,
            &measure,
        );
        let delete = find(&list, Action::Delete);
        let cancel = find(&list, Action::BackToTitle);
        assert!(!delete.rect.overlaps(&cancel.rect));
        assert_eq!(
            click(Menu::ConfirmDelete, center_of(&delete), &list),
            Action::Delete
        );
        assert_eq!(
            click(Menu::ConfirmDelete, center_of(&cancel), &list),
            Action::BackToTitle
        );
        assert_eq!(
            click(Menu::ConfirmDelete, vec2(5.0, 5.0), &list),
            Action::None
        );
    }

    #[test]
    fn result_screen_follows_the_delete_status() {
        let deleting = buttons(Menu::DeleteResult, DeleteStatus::Deleting, SCREEN, &measure);
        assert!(deleting.is_empty());
        let failed = buttons(Menu::DeleteResult, DeleteStatus::Failed, SCREEN, &measure);
        assert_eq!(
            failed.iter().map(|b| b.action).collect::<Vec<_>>(),
            vec![Action::Delete, Action::BackToTitle]
        );
        let done = buttons(Menu::DeleteResult, DeleteStatus::Done, SCREEN, &measure);
        assert_eq!(
            done.iter().map(|b| b.action).collect::<Vec<_>>(),
            vec![Action::BackToTitle]
        );
    }

    #[test]
    fn title_controls_line_up_in_two_columns_at_the_center() {
        for screen in [SCREEN, vec2(1024.0, 768.0)] {
            let layout = title_layout(screen, &measure);
            assert_eq!(layout.controls.len(), 3);
            let key_right = |row: &ControlRow| row.key_x + measure(row.key, CONTROL_SIZE);
            let first = layout.controls[0];
            for row in &layout.controls {
                // キーは右端を、説明は左端を揃える。キーと説明は重ならない。
                assert_eq!(key_right(row), key_right(&first));
                assert_eq!(row.action_x, first.action_x);
                assert!(key_right(row) + CONTROL_GAP <= row.action_x);
            }
            let left = layout
                .controls
                .iter()
                .map(|r| r.key_x)
                .fold(f32::MAX, f32::min);
            let right = layout
                .controls
                .iter()
                .map(|r| r.action_x + measure(r.action, CONTROL_SIZE))
                .fold(0.0, f32::max);
            assert!(left > 0.0 && right < screen.x);
            assert!(((left + right) / 2.0 - screen.x / 2.0).abs() < 0.01);
        }
    }

    #[test]
    fn title_lines_go_top_to_bottom_and_stay_clear_of_the_delete_button() {
        for screen in [SCREEN, vec2(1024.0, 768.0)] {
            let layout = title_layout(screen, &measure);
            let mut order = vec![layout.logo, layout.course, layout.best];
            order.extend(layout.controls.iter().map(|row| row.baseline));
            order.extend([layout.start, layout.notice]);
            assert!(order.windows(2).all(|pair| pair[0] < pair[1]));
            // ロゴの上端が画面に収まる。
            assert!(layout.logo - 56.0 > 0.0);
            let list = buttons(Menu::Title, DeleteStatus::NotRequested, screen, &measure);
            let delete = find(&list, Action::OpenConfirm);
            assert!(layout.notice < delete.rect.y);
        }
    }

    #[test]
    fn title_hides_delete_after_the_data_is_gone() {
        let list = buttons(Menu::Title, DeleteStatus::Done, SCREEN, &measure);
        assert!(list.is_empty());
        assert_eq!(click(Menu::Title, SCREEN / 2.0, &list), Action::Start);
    }
}
