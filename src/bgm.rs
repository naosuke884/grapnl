//! BGM。`scripts/gen-bgm.py` で合成した `web/bgm.ogg` を、ゲームが始まってから読み込んで繰り返し流す。
//! 最初の読み込みを重くしないよう wasm には埋め込まない。読み込めなければ流さないだけで、遊ぶのには困らない。
//! 大きさは効果音と同じ音量の設定に従う。

use macroquad::audio::{
    load_sound_from_bytes, play_sound, set_sound_volume, PlaySoundParams, Sound,
};
use macroquad::experimental::coroutines::{start_coroutine, Coroutine};
use macroquad::prelude::*;

/// ブラウザではページと同じ場所から読む。
#[cfg(target_arch = "wasm32")]
const PATH: &str = "bgm.ogg";
#[cfg(not(target_arch = "wasm32"))]
const PATH: &str = "web/bgm.ogg";
/// 効果音に対する BGM の大きさ。効果音が聞き取れるよう控えめにする。
const LEVEL: f32 = 0.45;

enum State {
    /// 曲のファイルを読んでいる。
    Fetching(Coroutine<Option<Vec<u8>>>),
    /// 読んだファイルを音にしている。
    Decoding(Coroutine<Option<Sound>>),
    Playing {
        sound: Sound,
        volume: f32,
    },
    /// 読めなかった。流さない。
    Off,
}

pub struct Bgm {
    state: State,
}

impl Bgm {
    /// 曲を読み始める。
    pub fn start() -> Self {
        Self {
            state: State::Fetching(start_coroutine(async { load_file(PATH).await.ok() })),
        }
    }

    /// 描画フレームごとに 1 回呼ぶ。`gain` は `Volume::gain`。読み終えたら流し始め、音量の変化に合わせる。
    pub fn update(&mut self, gain: f32) {
        let volume = gain * LEVEL;
        match &mut self.state {
            State::Fetching(fetching) => {
                if fetching.is_done() {
                    self.state = match fetching.retrieve().flatten() {
                        Some(bytes) => State::Decoding(start_coroutine(async move {
                            load_sound_from_bytes(&bytes).await.ok()
                        })),
                        None => State::Off,
                    };
                }
            }
            State::Decoding(decoding) => {
                if decoding.is_done() {
                    self.state = match decoding.retrieve().flatten() {
                        Some(sound) => {
                            play_sound(
                                &sound,
                                PlaySoundParams {
                                    looped: true,
                                    volume,
                                },
                            );
                            State::Playing { sound, volume }
                        }
                        None => State::Off,
                    };
                }
            }
            State::Playing {
                sound,
                volume: playing,
            } => {
                if *playing != volume {
                    set_sound_volume(sound, volume);
                    *playing = volume;
                }
            }
            State::Off => {}
        }
    }
}
