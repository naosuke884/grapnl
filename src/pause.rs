//! 遊んでいる途中の一時停止。Esc で止め、もう一度 Esc かクリックで再開する。
//! タブを離れたときやウィンドウのフォーカスが外れたときも止める（`page::lost_focus`）。
//! 止まっている間は `Game::step` を呼ばないので、止めた時間は `Game::elapsed` に入らない。
//! ゲームオーバーの画面では止めない。

/// このフレームの一時停止の状態。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frame {
    /// 止まっていない。ゲームを進める。
    Running,
    /// 止まっている。止めたフレームも含む。ゲームを進めず、押された入力は捨てる。
    Paused,
    /// このフレームで再開した。止まっていた時間を追いかけないよう `accumulator` を 0 に戻し、
    /// 再開に使ったクリックはワイヤーに使わず捨てる。
    Resumed,
}

#[derive(Default)]
pub struct Pause {
    paused: bool,
}

impl Pause {
    pub fn paused(&self) -> bool {
        self.paused
    }

    /// 描画フレームごとに 1 回呼ぶ。`click` は左右どちらかのボタンを押した瞬間。
    /// `lost_focus` はタブを離れたか、フォーカスが外れたか。
    pub fn update(&mut self, game_over: bool, esc: bool, click: bool, lost_focus: bool) -> Frame {
        if game_over {
            self.paused = false;
            return Frame::Running;
        }
        if self.paused {
            // 離れたままのフレームでは再開しない。
            if (esc || click) && !lost_focus {
                self.paused = false;
                return Frame::Resumed;
            }
            return Frame::Paused;
        }
        if esc || lost_focus {
            self.paused = true;
            return Frame::Paused;
        }
        Frame::Running
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_until_esc() {
        let mut pause = Pause::default();
        assert_eq!(pause.update(false, false, false, false), Frame::Running);
        // クリックだけでは止まらない。ワイヤーを撃つ。
        assert_eq!(pause.update(false, false, true, false), Frame::Running);
        assert_eq!(pause.update(false, true, false, false), Frame::Paused);
        assert!(pause.paused());
        assert_eq!(pause.update(false, false, false, false), Frame::Paused);
    }

    #[test]
    fn esc_or_click_resumes() {
        let mut pause = Pause::default();
        pause.update(false, true, false, false);
        assert_eq!(pause.update(false, true, false, false), Frame::Resumed);
        assert!(!pause.paused());
        assert_eq!(pause.update(false, false, false, false), Frame::Running);

        pause.update(false, true, false, false);
        assert_eq!(pause.update(false, false, true, false), Frame::Resumed);
        assert_eq!(pause.update(false, false, false, false), Frame::Running);
    }

    #[test]
    fn losing_focus_pauses_and_needs_input_to_resume() {
        let mut pause = Pause::default();
        assert_eq!(pause.update(false, false, false, true), Frame::Paused);
        // 戻ってきただけでは再開しない。
        assert_eq!(pause.update(false, false, false, false), Frame::Paused);
        // 離れたのと同じフレームの入力では再開しない。
        assert_eq!(pause.update(false, true, true, true), Frame::Paused);
        assert_eq!(pause.update(false, false, true, false), Frame::Resumed);
    }

    #[test]
    fn not_pausable_on_game_over() {
        let mut pause = Pause::default();
        assert_eq!(pause.update(true, true, false, false), Frame::Running);
        assert_eq!(pause.update(true, false, false, true), Frame::Running);
        assert!(!pause.paused());
    }
}
