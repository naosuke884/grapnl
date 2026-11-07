//! ブラウザのページの様子。`web/index.html` のプラグインが、タブを離れたこととフォーカスが外れたことを覚えておく。
//! ネイティブ実行（開発用）では分からないので、離れなかったことにする。

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "env")]
extern "C" {
    fn page_lost_focus() -> i32;
}

/// 前に呼んでからタブを離れたかフォーカスが外れたか、今もタブが隠れているか。
/// 呼ぶと覚えていた分は消える。
pub fn lost_focus() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        unsafe { page_lost_focus() != 0 }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}
