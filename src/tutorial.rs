//! 初めて遊ぶ人への操作の説明。最初の `RUNS` 回のプレイだけ、画面下に吹き出しで 1 つずつ出す。
//! 撃つ・戻す → ブースト → 敵 の順に、その操作を済ませたら次へ進む。
//! 遊んだ回数は端末に覚えておく（`page::write_number`）。

use macroquad::prelude::*;

use crate::game::Event;
use crate::page;
use crate::text::{self, Text};

/// 説明を出すプレイの回数。ゲームオーバーになるたびに 1 回と数える。
pub const RUNS: u32 = 3;
const PLAYS_KEY: &str = "plays";
/// 敵の説明は操作を求めないので、倒さなくてもこの時間（秒）出したら消す。
const ENEMY_TIME: f32 = 8.0;
/// 吹き出しが出てから、はっきり見えるまでの時間（秒）。
const FADE_IN: f32 = 0.25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    /// 左クリックでワイヤーを戻す・撃つ。自分で撃ったワイヤーが刺さったら済み。
    Wire,
    /// 右クリックでブースト。
    Boost,
    /// 敵にワイヤーを当てると倒せる。
    Enemy,
    Done,
}

pub struct Tutorial {
    /// これまでに終えたプレイの回数。
    plays: u32,
    step: Step,
    /// 今の説明を出している時間（秒）。
    shown: f32,
}

impl Tutorial {
    /// 端末に覚えておいた回数を読んで、1 回目のプレイを始める。
    pub fn load() -> Self {
        Self::with_plays(page::read_number(PLAYS_KEY).unwrap_or(0))
    }

    fn with_plays(plays: u32) -> Self {
        Self {
            plays,
            step: if plays < RUNS { Step::Wire } else { Step::Done },
            shown: 0.0,
        }
    }

    /// ゲームオーバーになったら 1 回だけ呼ぶ。回数を数えて覚えておく。
    pub fn finish_run(&mut self) {
        if self.plays < RUNS {
            self.plays += 1;
            page::write_number(PLAYS_KEY, self.plays);
        }
        self.step = Step::Done;
    }

    /// やり直すときに呼ぶ。まだ `RUNS` 回に届いていなければ、最初の説明から出し直す。
    pub fn restart(&mut self) {
        *self = Self::with_plays(self.plays);
    }

    /// 固定刻みの更新ごとに、その更新で起きた出来事を渡す。
    pub fn observe(&mut self, events: &[Event], dt: f32) {
        if self.step == Step::Done {
            return;
        }
        self.shown += dt;
        let done = match self.step {
            Step::Wire => events
                .iter()
                .any(|event| matches!(event, Event::WireAttached { .. })),
            Step::Boost => events.iter().any(|event| matches!(event, Event::Boosted)),
            Step::Enemy => {
                self.shown >= ENEMY_TIME
                    || events
                        .iter()
                        .any(|event| matches!(event, Event::EnemyKilled { .. }))
            }
            Step::Done => false,
        };
        if done {
            self.step = match self.step {
                Step::Wire => Step::Boost,
                Step::Boost => Step::Enemy,
                Step::Enemy | Step::Done => Step::Done,
            };
            self.shown = 0.0;
        }
    }

    /// 今出す説明。`attached` はワイヤーが刺さっているか。
    fn message(&self, attached: bool) -> Option<&'static str> {
        match self.step {
            Step::Wire if attached => Some("Left click: retract the wire"),
            Step::Wire => Some("Left click: shoot the wire toward the cursor"),
            Step::Boost => Some("Right click: boost the way you're moving"),
            Step::Enemy => Some("Hit an enemy with the wire to destroy it and bounce"),
            Step::Done => None,
        }
    }

    /// 画面の下に、吹き出しを描く。画面の文字と同じ座標で呼ぶ。
    pub fn draw(&self, text: &Text, screen: Vec2, attached: bool) {
        let Some(message) = self.message(attached) else {
            return;
        };
        let alpha = (self.shown / FADE_IN).clamp(0.0, 1.0);
        let size = text::MEDIUM;
        let dimensions = text.measure(message, size);
        let padding = vec2(22.0, 14.0);
        let width = dimensions.width + padding.x * 2.0;
        let height = dimensions.height + padding.y * 2.0;
        let bottom = screen.y - 56.0;
        let center_x = screen.x / 2.0;
        let rect = Rect::new(center_x - width / 2.0, bottom - height, width, height);
        let fill = Color::new(0.96, 0.96, 0.98, 0.95 * alpha);
        draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
        text.draw_y_centered(
            message,
            rect.x + padding.x,
            rect.center().y,
            size,
            Color::new(0.1, 0.12, 0.17, alpha),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attached() -> Event {
        Event::WireAttached {
            at: Vec2::ZERO,
            normal: Vec2::Y,
        }
    }

    #[test]
    fn steps_through_wire_boost_and_enemy() {
        let mut tutorial = Tutorial::with_plays(0);
        assert_eq!(tutorial.message(true), Some("Left click: retract the wire"));
        assert!(tutorial.message(false).unwrap().contains("shoot"));
        // 戻しただけでは進まない。撃って刺さったら進む。
        tutorial.observe(&[Event::WirePulledBack], 0.1);
        tutorial.observe(&[Event::WireShot], 0.1);
        assert_eq!(tutorial.step, Step::Wire);
        tutorial.observe(&[attached()], 0.1);
        assert_eq!(tutorial.step, Step::Boost);
        tutorial.observe(&[Event::Boosted], 0.1);
        assert_eq!(tutorial.step, Step::Enemy);
        tutorial.observe(
            &[Event::EnemyKilled {
                at: Vec2::ZERO,
                multiplier: 1,
            }],
            0.1,
        );
        assert_eq!(tutorial.message(false), None);
    }

    #[test]
    fn enemy_hint_goes_away_after_a_while() {
        let mut tutorial = Tutorial::with_plays(0);
        tutorial.observe(&[attached()], 0.1);
        tutorial.observe(&[Event::Boosted], 0.1);
        tutorial.observe(&[], ENEMY_TIME - 0.5);
        assert_eq!(tutorial.step, Step::Enemy);
        tutorial.observe(&[], 0.5);
        assert_eq!(tutorial.step, Step::Done);
    }

    #[test]
    fn shows_only_for_the_first_runs() {
        let mut tutorial = Tutorial::with_plays(RUNS - 1);
        assert_eq!(tutorial.step, Step::Wire);
        tutorial.finish_run();
        tutorial.restart();
        assert_eq!(tutorial.step, Step::Done);
        assert_eq!(Tutorial::with_plays(RUNS + 5).step, Step::Done);
    }

    #[test]
    fn restarts_from_the_first_hint_while_runs_remain() {
        let mut tutorial = Tutorial::with_plays(0);
        tutorial.observe(&[attached()], 0.1);
        tutorial.finish_run();
        assert_eq!(tutorial.message(false), None);
        tutorial.restart();
        assert_eq!(tutorial.step, Step::Wire);
    }
}
