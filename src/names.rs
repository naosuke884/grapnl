//! 人の名前（「名前#1a2b」）を描く。名前は誰でも付けられ、同梱のフォントでは描けないことが多い。
//! 描けない名前に出会ったときだけ、名前用に全文字のフォント（約 1.8 MB）を後から読み込み、
//! それまでは描けない文字を「?」にする。ランキングと、同じコースを走っている人の影の名前で使う。

use macroquad::experimental::coroutines::{start_coroutine, Coroutine};
use macroquad::prelude::*;

use crate::text::{Fonts, Text};

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
    /// 描く名前を見せる。同梱のフォントで描けない文字があれば、名前用のフォントを読み始める。
    pub fn want(&mut self, fonts: &Fonts, name: &str) {
        if matches!(self, Self::NotNeeded)
            && !fonts.heading.can_draw(name)
            && !fonts.japanese.can_draw(name)
        {
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

    /// `name` を描くフォントと、そのフォントで描ける形にした名前と、描く大きさ。
    /// 英数字だけの名前は見出しのフォントで描く（本文のフォントは「Z」と「2」、「S」と「5」が似ていて見分けにくい）。
    /// 無理なら同梱の日本語のフォントを、それも無理なら名前用のフォントを使う。
    /// どれも無ければ、描けない文字を「?」に置き換えて日本語のフォントで描く。
    /// `heading_size` は見出しのフォントで描くときの大きさ、`size` はそれ以外のときの大きさ。
    pub fn pick<'a>(
        &'a self,
        fonts: &'a Fonts,
        name: &str,
        heading_size: f32,
        size: f32,
    ) -> (&'a Text, String, f32) {
        if fonts.heading.can_draw(name) {
            (&fonts.heading, name.to_owned(), heading_size)
        } else if fonts.japanese.can_draw(name) {
            (&fonts.japanese, name.to_owned(), size)
        } else if let Self::Ready(names) = self {
            (names, names.drawable(name), size)
        } else {
            (&fonts.japanese, fonts.japanese.drawable(name), size)
        }
    }
}
