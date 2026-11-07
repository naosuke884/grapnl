//! 敵を倒した瞬間に、ゲームの更新を一瞬だけ止める（ヒットストップ）。
//! 止まっている間は `Game::step` を呼ばないので、止めた時間は `Game::elapsed` に入らない。
//! 止めた時間は固定刻みの `accumulator` にも貯めない。貯めると止まり終えた直後にまとめて追いつく。

use crate::game::Event;

/// 敵を倒したときに止める時間（秒）。
const KILL_DURATION: f32 = 0.05;

/// 残りの止まる時間。止まっていないときは 0。
#[derive(Default)]
pub struct HitStop {
    remaining: f32,
}

impl HitStop {
    /// 出来事に合わせて止める。止まっている途中でまた倒したら、残りを長いほうにそろえる（足さない）。
    pub fn kick(&mut self, event: &Event) {
        if let Event::EnemyKilled { .. } = event {
            self.remaining = self.remaining.max(KILL_DURATION);
        }
    }

    /// 止まっているか。止まった直後の固定刻みの更新を、同じ描画フレームのうちに回さないために見る。
    pub fn active(&self) -> bool {
        self.remaining > 0.0
    }

    /// 描画フレームの経過時間から止まっている分を差し引き、ゲームを進めてよい時間を返す。
    /// 止まり終えたフレームでは、止まり終えた後の分だけを返す。
    pub fn consume(&mut self, frame_time: f32) -> f32 {
        let stopped = self.remaining.min(frame_time);
        self.remaining -= stopped;
        frame_time - stopped
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macroquad::prelude::Vec2;

    const DT: f32 = 1.0 / 60.0;

    fn kill() -> Event {
        Event::EnemyKilled {
            at: Vec2::ZERO,
            multiplier: 1,
        }
    }

    #[test]
    fn passes_time_through_until_kicked() {
        let mut stop = HitStop::default();
        stop.kick(&Event::Boosted);
        stop.kick(&Event::WireShot);
        stop.kick(&Event::GameOver);
        assert!(!stop.active());
        assert_eq!(stop.consume(DT), DT);
    }

    #[test]
    fn a_kill_stops_for_the_duration_without_banking_it() {
        let mut stop = HitStop::default();
        stop.kick(&kill());
        assert!(stop.active());
        let mut stopped = 0.0;
        let mut passed = 0.0;
        for _ in 0..10 {
            let game_time = stop.consume(DT);
            stopped += DT - game_time;
            passed += game_time;
        }
        assert!(!stop.active());
        assert!((stopped - KILL_DURATION).abs() < 1e-5);
        // 止まっていた分は返さない。10 フレームのうち、止まっていない分だけが進む。
        assert!((passed - (10.0 * DT - KILL_DURATION)).abs() < 1e-5);
    }

    #[test]
    fn the_frame_that_ends_the_stop_returns_only_the_rest() {
        let mut stop = HitStop::default();
        stop.kick(&kill());
        assert_eq!(stop.consume(0.03), 0.0);
        let rest = stop.consume(0.03);
        assert!((rest - 0.01).abs() < 1e-5);
        assert!(!stop.active());
    }

    #[test]
    fn kills_during_a_stop_do_not_stack() {
        let mut stop = HitStop::default();
        stop.kick(&kill());
        stop.consume(0.02);
        stop.kick(&kill());
        stop.kick(&kill());
        assert_eq!(stop.consume(1.0), 1.0 - KILL_DURATION);
    }
}
