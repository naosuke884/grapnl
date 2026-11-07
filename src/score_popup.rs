//! 敵を倒した場所に出す、入った得点（`+300` など）。
//! 見た目だけで、得点の計算には関わらない。入る点は `Game::step` が数える。
//! 文字は作った `Vec` の空き枠を使い回し、文字列は倍率ごとに決まったものを指すだけにする。
//! 遊んでいる途中で文字列を作り直さない。
//! 動きは破片（`src/particles.rs`）と同じく `Game::step` の固定刻みで進める。

use macroquad::prelude::*;

use crate::game::{Event, MAX_MULTIPLIER};
use crate::text::{self, Text};

/// 同時に出せる得点の数。これを超えたら、残り時間が一番短いものを上書きする。
const MAX_POPUPS: usize = 8;
/// 出てから消えるまでの時間（秒）。
const LIFE: f32 = 0.9;
/// 出た直後に上へ浮く速さ（ワールド座標 / 秒）。だんだん遅くなって止まる。
const RISE_SPEED: f32 = 420.0;
/// 浮く速さが落ちる割合（1 秒あたり）。
const RISE_DAMPING: f32 = 3.5;
/// 残り時間がこの割合を切ったら薄くなり始める。
const FADE_FROM: f32 = 0.5;
/// 出た直後に大きく出て、この時間（秒）で元の大きさへ戻る。
const POP_TIME: f32 = 0.12;
/// 出た直後の大きさの倍率。
const POP_SCALE: f32 = 1.35;
/// 倍率 1 の文字の高さ（画面のピクセル）。倍率が 1 上がるごとに `SIZE_STEP` ずつ大きくする。
const BASE_SIZE: f32 = 30.0;
const SIZE_STEP: f32 = 7.0;
/// 文字の画像を作る大きさ。ドットフォントの元の大きさで作り、ドットのまま拡大して描く。
const RASTER: u16 = text::SMALL as u16;

/// 倍率ごとの文字。`KILL_POINTS × 倍率`。毎フレーム `format!` しないよう、決まった文字列を使う。
const LABELS: [&str; MAX_MULTIPLIER as usize] = ["+100", "+200", "+300", "+400", "+500"];

/// 倍率ごとの色。上がるほど白から黄、橙、赤みへ寄せて目立たせる。
const COLORS: [Color; MAX_MULTIPLIER as usize] = [
    Color::new(1.0, 1.0, 1.0, 1.0),
    Color::new(1.0, 0.93, 0.55, 1.0),
    Color::new(1.0, 0.8, 0.3, 1.0),
    Color::new(1.0, 0.65, 0.25, 1.0),
    Color::new(1.0, 0.45, 0.25, 1.0),
];

/// 倍率の番号。上限を超えた倍率は上限として扱う。
fn level_of(multiplier: u32) -> usize {
    (multiplier.clamp(1, MAX_MULTIPLIER) - 1) as usize
}

/// 倍率に掛かった 1 体の得点の文字。
pub fn label_of(multiplier: u32) -> &'static str {
    LABELS[level_of(multiplier)]
}

/// 倍率ごとの文字の高さ（画面のピクセル）。
fn size_of(multiplier: u32) -> f32 {
    BASE_SIZE + SIZE_STEP * level_of(multiplier) as f32
}

/// 浮いている 1 つの得点。
pub struct Popup {
    /// 文字の中心のワールド座標。
    pub position: Vec2,
    pub multiplier: u32,
    rise: f32,
    life: f32,
}

impl Popup {
    pub fn label(&self) -> &'static str {
        label_of(self.multiplier)
    }

    /// 出てからの時間で変わる色。最後の半分で薄くなって消える。
    pub fn color(&self) -> Color {
        let mut color = COLORS[level_of(self.multiplier)];
        color.a = (self.life / (LIFE * FADE_FROM)).clamp(0.0, 1.0);
        color
    }

    /// 文字の高さ（画面のピクセル）。出た直後だけ少し大きい。
    pub fn size(&self) -> f32 {
        let age = LIFE - self.life;
        let pop = 1.0 + (POP_SCALE - 1.0) * (1.0 - age / POP_TIME).clamp(0.0, 1.0);
        size_of(self.multiplier) * pop
    }
}

/// 場に出ている得点。
pub struct Popups {
    slots: Vec<Popup>,
}

impl Popups {
    pub fn new() -> Self {
        Self {
            // 枠は最初にまとめて取る。足すたびに確保し直さない。
            slots: Vec::with_capacity(MAX_POPUPS),
        }
    }

    /// 敵を倒した場所に得点を出す。ほかの出来事では何もしない。
    pub fn spawn(&mut self, event: Event) {
        let Event::EnemyKilled { at, multiplier } = event else {
            return;
        };
        let popup = Popup {
            position: at,
            multiplier,
            rise: RISE_SPEED,
            life: LIFE,
        };
        if let Some(index) = self.slots.iter().position(|p| p.life <= 0.0) {
            self.slots[index] = popup;
        } else if self.slots.len() < MAX_POPUPS {
            self.slots.push(popup);
        } else if let Some(oldest) = self
            .slots
            .iter_mut()
            .min_by(|a, b| a.life.total_cmp(&b.life))
        {
            *oldest = popup;
        }
    }

    pub fn update(&mut self, dt: f32) {
        for popup in &mut self.slots {
            if popup.life <= 0.0 {
                continue;
            }
            popup.life = (popup.life - dt).max(0.0);
            // ワールドの y は下向き。上へ浮かせるので引く。
            popup.position.y -= popup.rise * dt;
            popup.rise *= (1.0 - RISE_DAMPING * dt).max(0.0);
        }
    }

    pub fn alive(&self) -> impl Iterator<Item = &Popup> {
        self.slots.iter().filter(|p| p.life > 0.0)
    }

    /// ワールド座標のカメラの下で描く。`world_per_pixel` は画面の 1 ピクセルに映るワールドの幅で、
    /// 文字の大きさを画面のピクセルで決め、カメラの寄り引きに関わらず同じ大きさに見せる。
    pub fn draw(&self, text: &Text, world_per_pixel: f32) {
        for popup in self.alive() {
            let size = popup.size();
            let scale = size / RASTER as f32 * world_per_pixel;
            text.draw_scaled_centered(popup.label(), popup.position, RASTER, scale, popup.color());
        }
    }

    #[cfg(test)]
    fn slot_count(&self) -> usize {
        self.slots.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::KILL_POINTS;

    const DT: f32 = 1.0 / 120.0;

    fn kill(multiplier: u32) -> Event {
        Event::EnemyKilled {
            at: vec2(100.0, 200.0),
            multiplier,
        }
    }

    #[test]
    fn labels_are_the_points_gained() {
        for multiplier in 1..=MAX_MULTIPLIER {
            assert_eq!(
                label_of(multiplier),
                format!("+{}", KILL_POINTS * multiplier)
            );
        }
        // 上限を超えた倍率は来ないが、来ても上限の文字にする。
        assert_eq!(label_of(MAX_MULTIPLIER + 3), label_of(MAX_MULTIPLIER));
    }

    #[test]
    fn higher_multiplier_is_bigger_and_warmer() {
        for multiplier in 1..MAX_MULTIPLIER {
            assert!(size_of(multiplier + 1) > size_of(multiplier));
            let (a, b) = (
                COLORS[level_of(multiplier)],
                COLORS[level_of(multiplier + 1)],
            );
            assert!(b.g + b.b < a.g + a.b);
        }
    }

    #[test]
    fn only_kills_make_popups() {
        let mut popups = Popups::new();
        popups.spawn(Event::WireShot);
        popups.spawn(Event::WireAttached {
            at: Vec2::ZERO,
            normal: vec2(0.0, 1.0),
        });
        assert_eq!(popups.alive().count(), 0);
        popups.spawn(kill(3));
        let popup = popups.alive().next().unwrap();
        assert_eq!(popup.label(), "+300");
        assert_eq!(popup.position, vec2(100.0, 200.0));
    }

    #[test]
    fn floats_up_fades_and_disappears() {
        let mut popups = Popups::new();
        popups.spawn(kill(1));
        let start = popups.alive().next().unwrap().position;
        let first_size = popups.alive().next().unwrap().size();
        assert_eq!(popups.alive().next().unwrap().color().a, 1.0);

        for _ in 0..60 {
            popups.update(DT);
        }
        let popup = popups.alive().next().unwrap();
        assert!(popup.position.y < start.y);
        assert_eq!(popup.position.x, start.x);
        assert!(popup.color().a < 1.0);
        // 出た直後だけ大きく、すぐ元の大きさに戻る。
        assert!(popup.size() < first_size);
        assert_eq!(popup.size(), size_of(1));

        for _ in 0..60 {
            popups.update(DT);
        }
        assert_eq!(popups.alive().count(), 0);
    }

    #[test]
    fn slots_are_reused() {
        let mut popups = Popups::new();
        let capacity = popups.slots.capacity();
        popups.spawn(kill(1));
        popups.spawn(kill(2));
        assert_eq!(popups.slot_count(), 2);
        for _ in 0..200 {
            popups.update(DT);
        }
        popups.spawn(kill(4));
        assert_eq!(popups.slot_count(), 2);
        assert_eq!(popups.alive().next().unwrap().label(), "+400");

        // 枠が埋まったら、一番早く消えるものを上書きし、容量は増やさない。
        for _ in 0..MAX_POPUPS + 5 {
            popups.spawn(kill(5));
            popups.update(DT);
        }
        assert_eq!(popups.slot_count(), MAX_POPUPS);
        assert_eq!(popups.alive().count(), MAX_POPUPS);
        assert_eq!(popups.slots.capacity(), capacity);
    }
}
