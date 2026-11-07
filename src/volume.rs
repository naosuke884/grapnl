//! 効果音の音量。一時停止の画面で変えたり消したりでき、端末に覚えておく（`page::write_number`）。
//! 音量は 5 段階で、消している間も段階は覚えておき、音を出し直したときに戻す。

use macroquad::prelude::*;

use crate::page;
use crate::text::Text;

/// 音量の段階の数。いちばん上が、音を作ったときの大きさ。
pub const LEVELS: u32 = 5;

const LEVEL_KEY: &str = "volume";
const MUTED_KEY: &str = "muted";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Volume {
    /// 1 から `LEVELS` まで。
    level: u32,
    muted: bool,
}

impl Default for Volume {
    /// 初めては、これまでと同じ大きさで鳴らす。
    fn default() -> Self {
        Self {
            level: LEVELS,
            muted: false,
        }
    }
}

impl Volume {
    /// 端末に覚えておいた音量。無いときや壊れているときは既定に戻す。
    pub fn load() -> Self {
        Self::from_stored(page::read_number(LEVEL_KEY), page::read_number(MUTED_KEY))
    }

    fn from_stored(level: Option<u32>, muted: Option<u32>) -> Self {
        let default = Self::default();
        Self {
            level: level
                .filter(|level| (1..=LEVELS).contains(level))
                .unwrap_or(default.level),
            muted: muted.map_or(default.muted, |muted| muted != 0),
        }
    }

    fn save(&self) {
        page::write_number(LEVEL_KEY, self.level);
        page::write_number(MUTED_KEY, u32::from(self.muted));
    }

    /// 鳴らすときに掛ける大きさ（0 から 1）。耳には音の強さの比が効くので、段階の 2 乗にする。
    pub fn gain(&self) -> f32 {
        if self.muted {
            return 0.0;
        }
        let ratio = self.level as f32 / LEVELS as f32;
        ratio * ratio
    }

    /// 段階を選ぶ。消していたら音を出し直す。
    fn set_level(&mut self, level: u32) {
        self.level = level.clamp(1, LEVELS);
        self.muted = false;
    }

    fn toggle_mute(&mut self) {
        self.muted = !self.muted;
    }

    /// 一時停止の間、描画フレームごとに 1 回呼ぶ。
    /// `left`・`right` は左右の矢印キー、`mute` は M キー、`clicked` はこのフレームでクリックしたボタン。
    /// 変わったら端末に覚えておく。
    pub fn update(&mut self, left: bool, right: bool, mute: bool, clicked: Option<Control>) {
        let before = *self;
        match clicked {
            Some(Control::Level(level)) => self.set_level(level),
            Some(Control::Mute) => self.toggle_mute(),
            None => {}
        }
        if left {
            if self.muted {
                self.muted = false;
            } else if self.level > 1 {
                self.level -= 1;
            } else {
                self.muted = true;
            }
        }
        if right {
            if self.muted {
                self.muted = false;
            } else {
                self.set_level(self.level + 1);
            }
        }
        if mute {
            self.toggle_mute();
        }
        if *self != before {
            self.save();
        }
    }
}

/// 一時停止の画面の音量の操作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    /// その段階を選ぶ（1 から `LEVELS`）。
    Level(u32),
    /// 音を消す・出し直す。
    Mute,
}

const BAR_WIDTH: f32 = 34.0;
const BAR_GAP: f32 = 8.0;
const BAR_HEIGHT: f32 = 36.0;
const LABEL_WIDTH: f32 = 64.0;
const BUTTON_WIDTH: f32 = 128.0;
const BUTTON_GAP: f32 = 24.0;
const PADDING: f32 = 16.0;

/// 一時停止の画面で、音量の操作を置く場所。画面の文字と同じ座標（`Playfield::hud_camera`）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Panel {
    /// 段階のボタンと消すボタンを並べた 1 行。
    row: Rect,
}

impl Panel {
    /// `center_x` を中心に、上端を `top` にして置く。
    pub fn new(center_x: f32, top: f32) -> Self {
        let bars = LEVELS as f32 * BAR_WIDTH + (LEVELS - 1) as f32 * BAR_GAP;
        let width = LABEL_WIDTH + bars + BUTTON_GAP + BUTTON_WIDTH;
        Self {
            row: Rect::new(center_x - width / 2.0, top, width, BAR_HEIGHT),
        }
    }

    /// 囲みの枠。この中のクリックでは一時停止を解かない。
    pub fn frame(&self) -> Rect {
        Rect::new(
            self.row.x - PADDING,
            self.row.y - PADDING,
            self.row.w + PADDING * 2.0,
            self.row.h + PADDING * 2.0,
        )
    }

    fn bar(&self, level: u32) -> Rect {
        let x = self.row.x + LABEL_WIDTH + (level - 1) as f32 * (BAR_WIDTH + BAR_GAP);
        Rect::new(x, self.row.y, BAR_WIDTH, BAR_HEIGHT)
    }

    fn mute_button(&self) -> Rect {
        Rect::new(
            self.row.right() - BUTTON_WIDTH,
            self.row.y,
            BUTTON_WIDTH,
            BAR_HEIGHT,
        )
    }

    /// `point` にあるボタン。
    pub fn control_at(&self, point: Vec2) -> Option<Control> {
        if self.mute_button().contains(point) {
            return Some(Control::Mute);
        }
        (1..=LEVELS)
            .find(|&level| {
                // 隙間を押しても近いほうの段階になるよう、左右に隙間の半分ずつ広げて判定する。
                let bar = self.bar(level);
                Rect::new(bar.x - BAR_GAP / 2.0, bar.y, bar.w + BAR_GAP, bar.h).contains(point)
            })
            .map(Control::Level)
    }

    pub fn draw(&self, text: &Text, volume: &Volume, hovered: Option<Control>) {
        let frame = self.frame();
        draw_rectangle(
            frame.x,
            frame.y,
            frame.w,
            frame.h,
            Color::from_rgba(20, 24, 32, 230),
        );
        let label_color = Color::from_rgba(180, 190, 205, 255);
        text.draw_y_centered("音量", self.row.x, self.row.center().y, 24.0, label_color);

        let lit = Color::from_rgba(140, 220, 160, 255);
        // 消している間は、段階を覚えていることが分かるよう暗く残す。
        let lit_muted = Color::from_rgba(80, 110, 95, 255);
        let unlit = Color::from_rgba(60, 66, 80, 255);
        for level in 1..=LEVELS {
            let bar = self.bar(level);
            // 段階が上がるほど高くし、音量の大きさを形でも見せる。
            let height = bar.h * (0.4 + 0.6 * level as f32 / LEVELS as f32);
            let color = if level > volume.level {
                unlit
            } else if volume.muted {
                lit_muted
            } else {
                lit
            };
            draw_rectangle(bar.x, bar.bottom() - height, bar.w, height, color);
            if hovered == Some(Control::Level(level)) {
                draw_rectangle_lines(bar.x, bar.y, bar.w, bar.h, 2.0, WHITE);
            }
        }

        let button = self.mute_button();
        let (label, fill) = if volume.muted {
            ("音を出す", Color::from_rgba(200, 90, 100, 255))
        } else {
            ("音を消す", Color::from_rgba(60, 66, 80, 255))
        };
        draw_rectangle(button.x, button.y, button.w, button.h, fill);
        if hovered == Some(Control::Mute) {
            draw_rectangle_lines(button.x, button.y, button.w, button.h, 2.0, WHITE);
        }
        text.draw_centered(
            label,
            button.center().x,
            button.center().y + 8.0,
            22.0,
            WHITE,
        );

        text.draw_centered(
            "← → で音量   M で消す・出す",
            frame.center().x,
            frame.bottom() + 28.0,
            20.0,
            Color::from_rgba(150, 160, 175, 255),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_at_full_volume() {
        let volume = Volume::from_stored(None, None);
        assert_eq!(volume, Volume::default());
        assert_eq!(volume.gain(), 1.0);
    }

    #[test]
    fn ignores_broken_stored_values() {
        assert_eq!(Volume::from_stored(Some(0), None).level, LEVELS);
        assert_eq!(Volume::from_stored(Some(LEVELS + 1), None).level, LEVELS);
        assert_eq!(
            Volume::from_stored(Some(2), Some(1)),
            Volume {
                level: 2,
                muted: true
            }
        );
    }

    #[test]
    fn muting_keeps_the_level() {
        let mut volume = Volume::from_stored(Some(3), None);
        volume.update(false, false, true, None);
        assert_eq!(volume.gain(), 0.0);
        volume.update(false, false, false, Some(Control::Mute));
        assert_eq!(
            volume,
            Volume {
                level: 3,
                muted: false
            }
        );
    }

    #[test]
    fn choosing_a_level_unmutes() {
        let mut volume = Volume::from_stored(Some(3), Some(1));
        volume.update(false, false, false, Some(Control::Level(1)));
        assert_eq!(
            volume,
            Volume {
                level: 1,
                muted: false
            }
        );
        assert!(volume.gain() > 0.0 && volume.gain() < 0.1);
    }

    #[test]
    fn arrows_step_and_left_at_the_bottom_mutes() {
        let mut volume = Volume::from_stored(Some(2), None);
        volume.update(true, false, false, None);
        assert_eq!(
            volume,
            Volume {
                level: 1,
                muted: false
            }
        );
        volume.update(true, false, false, None);
        assert_eq!(
            volume,
            Volume {
                level: 1,
                muted: true
            }
        );
        volume.update(false, true, false, None);
        assert_eq!(
            volume,
            Volume {
                level: 1,
                muted: false
            }
        );
        for _ in 0..10 {
            volume.update(false, true, false, None);
        }
        assert_eq!(volume.level, LEVELS);
    }

    #[test]
    fn finds_controls_under_the_pointer() {
        let panel = Panel::new(500.0, 300.0);
        let first = panel.bar(1);
        assert_eq!(panel.control_at(first.center()), Some(Control::Level(1)));
        let last = panel.bar(LEVELS);
        assert_eq!(
            panel.control_at(last.center()),
            Some(Control::Level(LEVELS))
        );
        // 隙間は近いほうの段階。
        let gap = vec2(first.right() + 1.0, first.center().y);
        assert_eq!(panel.control_at(gap), Some(Control::Level(1)));
        assert_eq!(
            panel.control_at(panel.mute_button().center()),
            Some(Control::Mute)
        );
        assert_eq!(panel.control_at(vec2(500.0, 100.0)), None);
        assert!(panel.frame().contains(vec2(panel.row.x - 4.0, panel.row.y)));
    }
}
