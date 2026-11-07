//! ブラウザのページの様子。`web/index.html` のプラグインが、タブを離れたこととフォーカスが外れたことを覚えておく。
//! 端末に覚えておく設定（音量など）も、同じプラグインを通して localStorage に置く。
//! ネイティブ実行（開発用）では分からないので、離れなかったことにし、設定は覚えない。

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "env")]
extern "C" {
    fn page_lost_focus() -> i32;
    fn page_storage_read(key: *const u8, key_len: usize) -> i32;
    fn page_storage_write(key: *const u8, key_len: usize, value: i32);
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

/// 端末に覚えておいた 0 以上の整数。無いとき、読めないときは `None`。
pub fn read_number(key: &str) -> Option<u32> {
    #[cfg(target_arch = "wasm32")]
    {
        let value = unsafe { page_storage_read(key.as_ptr(), key.len()) };
        u32::try_from(value).ok()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = key;
        None
    }
}

/// 0 以上の整数を端末に覚えておく。localStorage が使えなければ何もしない。
pub fn write_number(key: &str, value: u32) {
    #[cfg(target_arch = "wasm32")]
    {
        let value = i32::try_from(value).unwrap_or(i32::MAX);
        unsafe { page_storage_write(key.as_ptr(), key.len(), value) }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (key, value);
    }
}
