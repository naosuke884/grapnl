use macroquad::prelude::*;

/// 画面の文字のフォント。どれも SIL OFL 1.1（assets/fonts/OFL-*.txt）。scripts/subset-font.py が置く。
/// 見出し: Press Start 2P。スコア・GAME OVER・ボタンなど目立たせる所。8 ピクセルで作られている。
const HEADING_BYTES: &[u8] = include_bytes!("../assets/fonts/PressStart2P-Regular.ttf");
/// 本文: Pixelify Sans の Regular を、src/ の文字列リテラルで使う文字だけに絞ったもの。説明やランキングなど読ませる所。
const BODY_BYTES: &[u8] = include_bytes!("../assets/fonts/PixelifySans-subset.ttf");
/// 日本語: DotGothic16 を絞ったもの。上の 2 つで描けない文字と「名無し#1a2b」の名前に使う。16 ピクセルで作られている。
const JAPANESE_BYTES: &[u8] = include_bytes!("../assets/fonts/DotGothic16-Regular-subset.ttf");

/// 本文の文字の大きさ。Pixelify Sans はドット 1 つが 1 em の約 1/11 なので、ドットが滲まないよう 11 の倍数を使う。
pub const SMALL: f32 = 22.0;
pub const MEDIUM: f32 = 33.0;
pub const LARGE: f32 = 44.0;
/// 見出しの文字の大きさ。Press Start 2P は 8 ピクセルで作られているので、ドットが滲まないよう 8 の倍数だけを使う。
pub const HEAD_SMALL: f32 = 16.0;
pub const HEAD_MEDIUM: f32 = 24.0;
pub const HEAD_LARGE: f32 = 48.0;
/// 一時停止・ゲームオーバーの画面の囲みの地の色。
pub const PANEL_COLOR: Color = Color::from_rgba(20, 24, 32, 230);

/// 画面の文字の 3 つのフォント。
pub struct Fonts {
    pub heading: Text,
    pub body: Text,
    pub japanese: Text,
}

impl Fonts {
    /// 遊んでいる途中で文字の画像を作ってカクつかないよう、使う大きさの画像を始める前に作っておく。
    pub fn load() -> Self {
        Self {
            heading: Text::embedded(HEADING_BYTES, &[HEAD_SMALL, HEAD_MEDIUM, HEAD_LARGE]),
            body: Text::embedded(BODY_BYTES, &[SMALL, MEDIUM, LARGE]),
            japanese: Text::embedded(JAPANESE_BYTES, &[SMALL, MEDIUM]),
        }
    }
}

/// 画面の文字を描く。macroquad の既定のフォントは日本語を描けないので、すべての文字はこれで描く。
pub struct Text {
    font: Font,
    /// フォントに入っている文字。並べてある。
    chars: Vec<char>,
}

impl Text {
    fn embedded(bytes: &[u8], sizes: &[f32]) -> Self {
        let mut font = load_ttf_font_from_bytes(bytes).expect("同梱のフォントを読めない");
        // 高 DPI の画面で拡大されても、ドットが滲まないようにする。
        font.set_filter(FilterMode::Nearest);
        let chars = font_chars(bytes);
        // 見出しのフォントは絞っていないので、画像は ASCII の分だけ先に作る。ほかの文字は描くときに作る。
        let cached: Vec<char> = chars.iter().copied().filter(|c| c.is_ascii()).collect();
        for &size in sizes {
            font.populate_font_cache(&cached, size as u16);
        }
        Self { font, chars }
    }

    /// 後から読み込んだフォント。文字の画像は描くときに作る。読めなければ `None`。
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut font = load_ttf_font_from_bytes(bytes).ok()?;
        font.set_filter(FilterMode::Nearest);
        Some(Self {
            font,
            chars: font_chars(bytes),
        })
    }

    /// その文字を描けるか。
    pub fn has(&self, c: char) -> bool {
        self.chars.binary_search(&c).is_ok()
    }

    /// 描けない文字を「?」に置き換える。
    pub fn drawable(&self, text: &str) -> String {
        text.chars()
            .map(|c| if self.has(c) { c } else { '?' })
            .collect()
    }

    /// 描いたときの大きさ。
    pub fn measure(&self, text: &str, font_size: f32) -> TextDimensions {
        measure_text(text, Some(&self.font), font_size as u16, 1.0)
    }

    /// `baseline_y` は文字の下端（ベースライン）の高さ。
    pub fn draw(&self, text: &str, x: f32, baseline_y: f32, font_size: f32, color: Color) {
        draw_text_ex(
            text,
            x,
            baseline_y,
            TextParams {
                font: Some(&self.font),
                font_size: font_size as u16,
                color,
                ..Default::default()
            },
        );
    }

    pub fn draw_centered(
        &self,
        text: &str,
        center_x: f32,
        baseline_y: f32,
        font_size: f32,
        color: Color,
    ) {
        let dimensions = measure_text(text, Some(&self.font), font_size as u16, 1.0);
        self.draw(
            text,
            center_x - dimensions.width / 2.0,
            baseline_y,
            font_size,
            color,
        );
    }

    /// `center` が文字の中心になるように描く。ボタンの文字に使う。
    pub fn draw_y_centered_middle(&self, text: &str, center: Vec2, font_size: f32, color: Color) {
        let width = self.measure(text, font_size).width;
        self.draw_y_centered(text, center.x - width / 2.0, center.y, font_size, color);
    }

    /// `center_y` が文字の縦の中心になるように描く。`x` は左端。
    pub fn draw_y_centered(&self, text: &str, x: f32, center_y: f32, font_size: f32, color: Color) {
        let dimensions = measure_text(text, Some(&self.font), font_size as u16, 1.0);
        // draw の y はベースライン。文字はそこから offset_y 上、height の高さに収まる。
        self.draw(
            text,
            x,
            center_y + dimensions.offset_y - dimensions.height / 2.0,
            font_size,
            color,
        );
    }
}

impl Text {
    /// 画面のピクセルではなくカメラの座標で、`center` が文字の中心になるように描く。
    /// 文字の画像は `raster`（`Fonts::load` で作っておく大きさのどれか）の大きさで作り、`scale` 倍して描く。
    /// ワールド座標のカメラの下では、`scale` に画面の 1 ピクセルに映るワールドの幅を掛けて渡す。
    pub fn draw_scaled_centered(
        &self,
        text: &str,
        center: Vec2,
        raster: u16,
        scale: f32,
        color: Color,
    ) {
        let dimensions = measure_text(text, Some(&self.font), raster, scale);
        draw_text_ex(
            text,
            center.x - dimensions.width / 2.0,
            center.y + dimensions.offset_y - dimensions.height / 2.0,
            TextParams {
                font: Some(&self.font),
                font_size: raster,
                font_scale: scale,
                color,
                ..Default::default()
            },
        );
    }
}

/// フォントに入っている文字。並べて返す。
fn font_chars(bytes: &[u8]) -> Vec<char> {
    let mut chars = Vec::new();
    let Ok(face) = ttf_parser::Face::parse(bytes, 0) else {
        return chars;
    };
    let Some(cmap) = face.tables().cmap else {
        return chars;
    };
    for subtable in cmap.subtables {
        if subtable.is_unicode() {
            subtable.codepoints(|code| chars.extend(char::from_u32(code)));
        }
    }
    chars.sort_unstable();
    chars.dedup();
    chars
}

/// Rust のソースから、コメントを除いた文字列リテラルの中の文字を集める。
/// scripts/subset-font.py も同じ規則で文字を集めるので、変えるときは両方を揃える。
#[cfg(test)]
fn literal_chars(source: &str) -> Vec<char> {
    let chars: Vec<char> = source.chars().collect();
    let mut found = Vec::new();
    let mut i = 0;
    let n = chars.len();
    let starts = |i: usize, a: char, b: char| i + 1 < n && chars[i] == a && chars[i + 1] == b;
    while i < n {
        if starts(i, '/', '/') {
            while i < n && chars[i] != '\n' {
                i += 1;
            }
        } else if starts(i, '/', '*') {
            i += 2;
            while i < n && !starts(i, '*', '/') {
                i += 1;
            }
            i += 2;
        } else if chars[i] == '\'' {
            // 文字リテラル（'"' など）を読み飛ばす。ライフタイム（'a）はそのまま進む。
            if starts(i, '\'', '\\') {
                i += 2;
                while i < n && chars[i] != '\'' {
                    i += 1;
                }
                i += 1;
            } else if i + 2 < n && chars[i + 2] == '\'' {
                i += 3;
            } else {
                i += 1;
            }
        } else if chars[i] == '"' {
            i += 1;
            while i < n && chars[i] != '"' {
                if chars[i] == '\\' {
                    i += 2;
                    continue;
                }
                found.push(chars[i]);
                i += 1;
            }
            i += 1;
        } else {
            i += 1;
        }
    }
    found.sort_unstable();
    found.dedup();
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    /// src/ の各ファイル。新しいファイルを足したら、ここにも足す（下のテストが数を確かめる）。
    const SOURCES: [(&str, &str); 24] = [
        ("background.rs", include_str!("background.rs")),
        ("bgm.rs", include_str!("bgm.rs")),
        ("enemy.rs", include_str!("enemy.rs")),
        ("game.rs", include_str!("game.rs")),
        ("generator.rs", include_str!("generator.rs")),
        ("hitstop.rs", include_str!("hitstop.rs")),
        ("level.rs", include_str!("level.rs")),
        ("names.rs", include_str!("names.rs")),
        ("main.rs", include_str!("main.rs")),
        ("overlay.rs", include_str!("overlay.rs")),
        ("page.rs", include_str!("page.rs")),
        ("particles.rs", include_str!("particles.rs")),
        ("pause.rs", include_str!("pause.rs")),
        ("ranking.rs", include_str!("ranking.rs")),
        ("player.rs", include_str!("player.rs")),
        ("realtime.rs", include_str!("realtime.rs")),
        ("score_popup.rs", include_str!("score_popup.rs")),
        ("sfx.rs", include_str!("sfx.rs")),
        ("shadow.rs", include_str!("shadow.rs")),
        ("shake.rs", include_str!("shake.rs")),
        ("sprites.rs", include_str!("sprites.rs")),
        ("text.rs", include_str!("text.rs")),
        ("view.rs", include_str!("view.rs")),
        ("volume.rs", include_str!("volume.rs")),
    ];

    /// テストのモジュールより前だけを返す。テストの失敗メッセージは画面に出ないので、フォントに入れない。
    fn without_tests(source: &str) -> &str {
        source
            .find("\n#[cfg(test)]\nmod tests")
            .map_or(source, |end| &source[..end])
    }

    #[test]
    fn every_source_file_is_listed() {
        let listed = SOURCES.len();
        let actual = std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/src"))
            .unwrap()
            .filter(|entry| {
                let path = entry.as_ref().unwrap().path();
                path.extension().is_some_and(|ext| ext == "rs")
            })
            .count();
        assert_eq!(listed, actual, "src/ の .rs を SOURCES に足す");
    }

    #[test]
    fn font_has_every_character_in_string_literals() {
        let faces = [HEADING_BYTES, BODY_BYTES, JAPANESE_BYTES]
            .map(|bytes| ttf_parser::Face::parse(bytes, 0).unwrap());
        for (name, source) in SOURCES {
            for c in literal_chars(without_tests(source)) {
                if c.is_control() {
                    continue;
                }
                assert!(
                    faces.iter().any(|face| face.glyph_index(c).is_some()),
                    "{name} の {c:?} がどのフォントにも無い。scripts/subset-font.py を実行し直す"
                );
            }
        }
    }

    #[test]
    fn collects_only_string_literals() {
        let source = "// コメント\nlet a = \"文字\"; /* 囲み */ let b = '\"'; let c = \"\\\"x\";";
        assert_eq!(literal_chars(source), vec!['x', '字', '文']);
    }
}
