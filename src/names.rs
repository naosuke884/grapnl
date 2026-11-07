//! 人の名前（「名前#1a2b」）を描く。名前は誰でも付けられ、画面の文字だけに絞ったフォントでは描けないことが多い。
//! 描けない名前に出会ったときだけ、名前用に全文字のフォント（約 1.8 MB）を後から読み込み、
//! それまでは描けない文字を「?」にする。ランキングと、同じコースを走っている人の影の名前で使う。

use macroquad::experimental::coroutines::{start_coroutine, Coroutine};
use macroquad::prelude::*;

use crate::text::Text;

/// 名前の無い人は、roommate が匿名 ID の頭 4 文字から作る（「名無し#1a2b」）。
/// よく出るので、絞ったフォントに入れておき、名前用のフォントを読まずに描けるようにする。
#[allow(dead_code)]
const NAME_CHARS: &str = "名無し#0123456789abcdef";
/// 名前用のフォント。scripts/subset-font.py が web/ に置く。ブラウザではページと同じ場所から読む。
#[cfg(target_arch = "wasm32")]
const NAMES_FONT: &str = "DotGothic16-Regular-names.ttf";
#[cfg(not(target_arch = "wasm32"))]
const NAMES_FONT: &str = "web/DotGothic16-Regular-names.ttf";

/// 名前用のフォント。一度読み込んだら、ページを開いている間は持っておく。
#[derive(Default)]
pub enum NameFont {
    #[default]
    NotNeeded,
    Loading(Coroutine<Option<Vec<u8>>>),
    Ready(Text),
    /// 読み込めなかった。描けない文字は「?」のままにし、読み直さない。
    Failed,
}

impl NameFont {
    /// 描く名前を見せる。`text` で描けない文字があれば、名前用のフォントを読み始める。
    pub fn want(&mut self, text: &Text, name: &str) {
        if matches!(self, Self::NotNeeded) && !name.chars().all(|c| text.has(c)) {
            *self = Self::Loading(start_coroutine(async { load_file(NAMES_FONT).await.ok() }));
        }
    }

    /// 描画フレームごとに 1 回呼ぶ。読み終えたらフォントにする。
    pub fn update(&mut self) {
        if let Self::Loading(loading) = self {
            if loading.is_done() {
                *self = match loading
                    .retrieve()
                    .flatten()
                    .and_then(|bytes| Text::from_bytes(&bytes))
                {
                    Some(font) => Self::Ready(font),
                    None => Self::Failed,
                };
            }
        }
    }

    /// `name` を描くフォントと、そのフォントで描ける形にした名前。
    /// 画面の文字のフォントで描ければそれを、無理なら名前用のフォントを、それも無ければ「?」に置き換えて返す。
    pub fn pick<'a>(&'a self, text: &'a Text, name: &str) -> (&'a Text, String) {
        if name.chars().all(|c| text.has(c)) {
            (text, name.to_owned())
        } else if let Self::Ready(names) = self {
            (names, names.drawable(name))
        } else {
            (text, text.drawable(name))
        }
    }
}
