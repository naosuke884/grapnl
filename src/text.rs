use macroquad::prelude::*;

/// 画面の日本語を描くフォント。M PLUS 1p（SIL OFL 1.1、assets/fonts/OFL.txt）を、
/// src/ の文字列リテラルで使う文字だけに絞ったもの。文字を足したら scripts/subset-font.py を実行し直す。
const FONT_BYTES: &[u8] = include_bytes!("../assets/fonts/MPLUS1p-Bold-subset.ttf");
/// 画面で使う文字の大きさ。遊んでいる途中で文字の画像を作ってカクつかないよう、始める前に作っておく。
const SIZES: [u16; 4] = [22, 24, 28, 56];

/// 画面の文字を描く。macroquad の既定のフォントは日本語を描けないので、すべての文字はこれで描く。
pub struct Text {
    font: Font,
}

impl Text {
    pub fn load() -> Self {
        let font = load_ttf_font_from_bytes(FONT_BYTES).expect("同梱のフォントを読めない");
        let chars = font_chars();
        for size in SIZES {
            font.populate_font_cache(&chars, size);
        }
        Self { font }
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

    /// 中央に揃えて描くが、数字がどれも「0」の幅だとして位置を決める。
    /// 毎秒変わる数字（残り時間など）で、文字の列が左右に揺れないようにする。
    pub fn draw_centered_steady(
        &self,
        text: &str,
        center_x: f32,
        baseline_y: f32,
        font_size: f32,
        color: Color,
    ) {
        let template: String = text
            .chars()
            .map(|c| if c.is_ascii_digit() { '0' } else { c })
            .collect();
        let dimensions = measure_text(&template, Some(&self.font), font_size as u16, 1.0);
        self.draw(
            text,
            center_x - dimensions.width / 2.0,
            baseline_y,
            font_size,
            color,
        );
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
    /// 文字の画像は `raster`（`SIZES` のどれか）の大きさで作り、`scale` 倍して描く。
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

/// 絞ったフォントに入っている文字。
fn font_chars() -> Vec<char> {
    let mut chars = Vec::new();
    let Ok(face) = ttf_parser::Face::parse(FONT_BYTES, 0) else {
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
    const SOURCES: [(&str, &str); 17] = [
        ("daily.rs", include_str!("daily.rs")),
        ("enemy.rs", include_str!("enemy.rs")),
        ("game.rs", include_str!("game.rs")),
        ("generator.rs", include_str!("generator.rs")),
        ("hitstop.rs", include_str!("hitstop.rs")),
        ("level.rs", include_str!("level.rs")),
        ("main.rs", include_str!("main.rs")),
        ("page.rs", include_str!("page.rs")),
        ("particles.rs", include_str!("particles.rs")),
        ("pause.rs", include_str!("pause.rs")),
        ("player.rs", include_str!("player.rs")),
        ("realtime.rs", include_str!("realtime.rs")),
        ("score_popup.rs", include_str!("score_popup.rs")),
        ("sfx.rs", include_str!("sfx.rs")),
        ("shadow.rs", include_str!("shadow.rs")),
        ("shake.rs", include_str!("shake.rs")),
        ("text.rs", include_str!("text.rs")),
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
        let face = ttf_parser::Face::parse(FONT_BYTES, 0).unwrap();
        for (name, source) in SOURCES {
            for c in literal_chars(without_tests(source)) {
                if c.is_control() {
                    continue;
                }
                assert!(
                    face.glyph_index(c).is_some(),
                    "{name} の {c:?} がフォントに無い。scripts/subset-font.py を実行し直す"
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
