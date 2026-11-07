//! 効果音。音は `scripts/gen-sfx.py` で合成した `assets/sfx/*.wav` を wasm に埋め込む。
//! ゲームの処理は音を知らず、`game::Event` を受け取ってここで鳴らす。

use macroquad::audio::{load_sound_from_bytes, play_sound, PlaySoundParams, Sound};

use crate::game::{Event, MAX_MULTIPLIER};

pub struct Sfx {
    shoot: Sound,
    attach: Sound,
    retract: Sound,
    /// 連鎖の倍率ごとの撃破音。macroquad は鳴らすときに高さを変えられないので、高さ違いを倍率の数だけ持つ。
    kills: Vec<Sound>,
    boost: Sound,
    game_over: Sound,
}

const KILLS: [&[u8]; MAX_MULTIPLIER as usize] = [
    include_bytes!("../assets/sfx/kill_1.wav"),
    include_bytes!("../assets/sfx/kill_2.wav"),
    include_bytes!("../assets/sfx/kill_3.wav"),
    include_bytes!("../assets/sfx/kill_4.wav"),
    include_bytes!("../assets/sfx/kill_5.wav"),
];

impl Sfx {
    /// 全部の音を読み込む。ブラウザでは音の展開が終わるまでフレームを進めて待つので、遊び始める前に呼ぶ。
    pub async fn load() -> Self {
        let mut kills = Vec::with_capacity(KILLS.len());
        for bytes in KILLS {
            kills.push(load(bytes).await);
        }
        Self {
            shoot: load(include_bytes!("../assets/sfx/shoot.wav")).await,
            attach: load(include_bytes!("../assets/sfx/attach.wav")).await,
            retract: load(include_bytes!("../assets/sfx/retract.wav")).await,
            kills,
            boost: load(include_bytes!("../assets/sfx/boost.wav")).await,
            game_over: load(include_bytes!("../assets/sfx/game_over.wav")).await,
        }
    }

    /// 出来事に合う音を `gain`（0 から 1、`Volume::gain`）の大きさで鳴らす。0 なら鳴らさない。
    /// ブラウザでは最初のクリックより前に鳴らしても無音になるだけ。
    pub fn play(&self, event: &Event, gain: f32) {
        if gain <= 0.0 {
            return;
        }
        let sound = match *event {
            Event::WireShot => &self.shoot,
            Event::WireAttached { .. } => &self.attach,
            Event::WirePulledBack => &self.retract,
            Event::EnemyKilled { multiplier, .. } => {
                let index = (multiplier.max(1) as usize - 1).min(self.kills.len() - 1);
                &self.kills[index]
            }
            Event::Boosted => &self.boost,
            Event::GameOver => &self.game_over,
        };
        play_sound(
            sound,
            PlaySoundParams {
                looped: false,
                volume: gain,
            },
        );
    }
}

async fn load(bytes: &[u8]) -> Sound {
    // 同梱の WAV なので、読めなければビルドの誤り。
    load_sound_from_bytes(bytes)
        .await
        .expect("同梱の効果音を読み込めない")
}
