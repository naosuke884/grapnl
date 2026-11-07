//! 敵を倒したときとゲームオーバーで画面を揺らす。
//! 見た目だけで、揺れは描画用のカメラにだけ足す。照準の座標変換には入れない。
//! ゲームオーバー後は `Game::step` が止まるので、揺れは描画フレームの経過時間で減らす。
//! 向きは乱数ではなく周波数の違う正弦波で決め、毎フレームの確保もしない。

use macroquad::prelude::*;

use crate::game::Event;

/// 敵を倒したときの揺れ幅（ワールドの単位）。画面の高さが約 1860 なので、数ピクセルほど。
const KILL_MAGNITUDE: f32 = 24.0;
/// 敵を倒したときに揺れている時間（秒）。
const KILL_DURATION: f32 = 0.18;
/// ゲームオーバーの揺れ幅。倒したときの 3 倍。
const GAME_OVER_MAGNITUDE: f32 = 72.0;
const GAME_OVER_DURATION: f32 = 0.5;
/// 横と縦の揺れの速さ（rad/s）。割り切れない比にして、往復が同じ線をなぞらないようにする。
const FREQUENCY_X: f32 = 71.0;
const FREQUENCY_Y: f32 = 53.0;

/// 今の揺れ。揺れていないときは `remaining` が 0。
#[derive(Default)]
pub struct Shake {
    magnitude: f32,
    duration: f32,
    remaining: f32,
    /// 揺れ始めてからの時間。正弦波の位相に使う。
    time: f32,
}

impl Shake {
    /// 出来事に合わせて揺らす。揺れに関わらない出来事は無視する。
    pub fn kick(&mut self, event: &Event) {
        match event {
            Event::EnemyKilled { .. } => self.start(KILL_MAGNITUDE, KILL_DURATION),
            Event::GameOver => self.start(GAME_OVER_MAGNITUDE, GAME_OVER_DURATION),
            _ => {}
        }
    }

    /// 今の揺れより強いときだけ揺らし直す。大きな揺れの途中に小さな揺れが来ても弱めない。
    fn start(&mut self, magnitude: f32, duration: f32) {
        if magnitude < self.amplitude() {
            return;
        }
        self.magnitude = magnitude;
        self.duration = duration;
        self.remaining = duration;
        self.time = 0.0;
    }

    /// 描画フレームの経過時間だけ揺れを減らす。
    pub fn update(&mut self, dt: f32) {
        self.remaining = (self.remaining - dt).max(0.0);
        self.time += dt;
    }

    /// 今の揺れ幅。残り時間の 2 乗で減らし、最初は強く、終わりはなめらかに止める。
    pub fn amplitude(&self) -> f32 {
        if self.remaining <= 0.0 {
            return 0.0;
        }
        let left = self.remaining / self.duration;
        self.magnitude * left * left
    }

    /// 描画用のカメラに足すずれ。各軸とも `amplitude` を超えない。
    pub fn offset(&self) -> Vec2 {
        let amplitude = self.amplitude();
        if amplitude == 0.0 {
            return Vec2::ZERO;
        }
        // 位相をずらし、揺れ始めのずれが 0 にならないようにする。
        vec2(
            (self.time * FREQUENCY_X + 1.0).sin(),
            (self.time * FREQUENCY_Y + 2.0).sin(),
        ) * amplitude
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn kill() -> Event {
        Event::EnemyKilled {
            at: Vec2::ZERO,
            multiplier: 1,
        }
    }

    #[test]
    fn still_until_kicked() {
        let mut shake = Shake::default();
        assert_eq!(shake.offset(), Vec2::ZERO);
        shake.kick(&Event::Boosted);
        shake.kick(&Event::WireShot);
        shake.update(DT);
        assert_eq!(shake.amplitude(), 0.0);
        assert_eq!(shake.offset(), Vec2::ZERO);
    }

    #[test]
    fn game_over_shakes_harder_and_longer_than_a_kill() {
        let mut small = Shake::default();
        small.kick(&kill());
        let mut big = Shake::default();
        big.kick(&Event::GameOver);
        assert_eq!(small.amplitude(), KILL_MAGNITUDE);
        assert_eq!(big.amplitude(), GAME_OVER_MAGNITUDE);
        assert!(small.offset() != Vec2::ZERO);

        let mut elapsed = 0.0;
        while small.amplitude() > 0.0 {
            small.update(DT);
            big.update(DT);
            elapsed += DT;
        }
        assert!((elapsed - KILL_DURATION).abs() <= DT);
        assert!(big.amplitude() > 0.0);
    }

    #[test]
    fn amplitude_decays_to_zero_and_offset_stays_within_it() {
        let mut shake = Shake::default();
        shake.kick(&Event::GameOver);
        let mut previous = shake.amplitude();
        for _ in 0..60 {
            shake.update(DT);
            let amplitude = shake.amplitude();
            assert!(amplitude <= previous);
            let offset = shake.offset();
            assert!(offset.x.abs() <= amplitude + 1e-4);
            assert!(offset.y.abs() <= amplitude + 1e-4);
            previous = amplitude;
        }
        assert_eq!(shake.amplitude(), 0.0);
        assert_eq!(shake.offset(), Vec2::ZERO);
    }

    #[test]
    fn a_kill_does_not_weaken_the_game_over_shake() {
        let mut shake = Shake::default();
        shake.kick(&Event::GameOver);
        shake.update(DT);
        let before = shake.amplitude();
        shake.kick(&kill());
        assert_eq!(shake.amplitude(), before);
    }

    #[test]
    fn a_new_kill_restarts_a_fading_kill() {
        let mut shake = Shake::default();
        shake.kick(&kill());
        shake.update(KILL_DURATION / 2.0);
        assert!(shake.amplitude() < KILL_MAGNITUDE);
        shake.kick(&kill());
        assert_eq!(shake.amplitude(), KILL_MAGNITUDE);
    }
}
