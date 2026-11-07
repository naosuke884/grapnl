//! 背景。遠くの夜の街を何層か重ね、遠い層ほどゆっくり流して奥行きを出す。
//! 空の色は進んだ距離で、夕焼け → 夜 → 夜明け → 夕焼け…と移り、どこまで来たかが目で分かる。
//! 見た目だけで判定には関わらない。地形や敵より暗く、彩度も低くして、遊ぶときの邪魔をしない。
//! ビルの形と窓の明かりはコースの種で決め、同じ日は誰が遊んでも同じ街になる。

use macroquad::prelude::*;

use crate::game::VIEW_HEIGHT;

/// 背景のドット 1 つのワールドでの大きさ。自機や敵（4）より粗くし、遠くに見せる。
const DOT: f32 = 8.0;
/// 空を塗り分ける帯の数。なめらかにせず段を残して、ドット絵らしくする。
const SKY_BANDS: usize = 18;
/// 空の色が一巡する距離（m）。
const CYCLE: f32 = 4000.0;

/// ある時間帯の色。
#[derive(Clone, Copy)]
struct Palette {
    sky_top: Color,
    sky_bottom: Color,
    /// 一番遠い層のビル。手前の層ほど `near` に寄せる。
    far: Color,
    near: Color,
    /// 星の濃さ（0 から 1）。
    stars: f32,
    /// 窓の明かりの濃さ（0 から 1）。
    windows: f32,
}

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
}

/// 時間帯の色と、その色になる距離（m、一巡の中での位置）。間は混ぜる。
const KEYS: [(f32, Palette); 4] = [
    (
        0.0,
        // 夕焼け
        Palette {
            sky_top: rgb(34, 26, 58),
            sky_bottom: rgb(150, 70, 70),
            far: rgb(70, 40, 62),
            near: rgb(26, 18, 34),
            stars: 0.15,
            windows: 0.5,
        },
    ),
    (
        1200.0,
        // 夜
        Palette {
            sky_top: rgb(6, 8, 20),
            sky_bottom: rgb(22, 28, 58),
            far: rgb(26, 32, 60),
            near: rgb(10, 12, 24),
            stars: 1.0,
            windows: 1.0,
        },
    ),
    (
        2600.0,
        // 夜明け
        Palette {
            sky_top: rgb(30, 40, 82),
            sky_bottom: rgb(170, 110, 110),
            far: rgb(62, 56, 92),
            near: rgb(22, 22, 40),
            stars: 0.3,
            windows: 0.4,
        },
    ),
    (
        CYCLE,
        // 一巡して夕焼けへ戻る
        Palette {
            sky_top: rgb(34, 26, 58),
            sky_bottom: rgb(150, 70, 70),
            far: rgb(70, 40, 62),
            near: rgb(26, 18, 34),
            stars: 0.15,
            windows: 0.5,
        },
    ),
];

/// ビルの層。`speed` はカメラに対して流れる割合（0 なら止まって見え、1 なら地形と同じ速さ）。
struct Layer {
    speed: f32,
    /// ビルの幅と高さの範囲（ワールド）。
    width: (f32, f32),
    height: (f32, f32),
    /// ビルとビルの間の範囲。
    gap: (f32, f32),
    /// `Palette::far` から `near` へ寄せる割合。
    depth: f32,
}

const LAYERS: [Layer; 3] = [
    Layer {
        speed: 0.08,
        width: (160.0, 320.0),
        height: (360.0, 760.0),
        gap: (0.0, 40.0),
        depth: 0.0,
    },
    Layer {
        speed: 0.18,
        width: (120.0, 260.0),
        height: (240.0, 560.0),
        gap: (40.0, 160.0),
        depth: 0.5,
    },
    Layer {
        speed: 0.32,
        width: (100.0, 220.0),
        height: (140.0, 420.0),
        gap: (120.0, 320.0),
        depth: 1.0,
    },
];
/// 1 つの層に並べるビルの区画の幅。区画ごとに 1 棟置き、種と区画の番号で形を決める。
const SLOT: f32 = 360.0;
const WINDOW_COLOR: Color = Color::new(1.0, 0.82, 0.45, 1.0);

/// 進んだ距離 `meters` の空の色。
fn palette_at(meters: f32) -> Palette {
    let t = meters.max(0.0) % CYCLE;
    let i = KEYS
        .windows(2)
        .position(|pair| t < pair[1].0)
        .unwrap_or(KEYS.len() - 2);
    let (from, a) = KEYS[i];
    let (to, b) = KEYS[i + 1];
    let f = ((t - from) / (to - from)).clamp(0.0, 1.0);
    // 端でゆっくり、真ん中で速く移る。
    let f = f * f * (3.0 - 2.0 * f);
    Palette {
        sky_top: mix(a.sky_top, b.sky_top, f),
        sky_bottom: mix(a.sky_bottom, b.sky_bottom, f),
        far: mix(a.far, b.far, f),
        near: mix(a.near, b.near, f),
        stars: a.stars + (b.stars - a.stars) * f,
        windows: a.windows + (b.windows - a.windows) * f,
    }
}

fn mix(a: Color, b: Color, f: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * f,
        a.g + (b.g - a.g) * f,
        a.b + (b.b - a.b) * f,
        1.0,
    )
}

/// 種と番号から、0 以上 1 未満の決まった値を作る。
fn hash(seed: u64, a: i64, b: u64) -> f32 {
    let mut x = seed
        ^ (a as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ b.wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    x ^= x >> 33;
    x = x.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    x ^= x >> 33;
    x = x.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    x ^= x >> 33;
    (x >> 40) as f32 / (1u64 << 24) as f32
}

fn snap(v: f32) -> f32 {
    (v / DOT).round() * DOT
}

/// ワールド座標のカメラの下で、地形より先に描く。`view` は映っている範囲（揺れの分を含めて少し広く取る）。
/// `meters` は進んだ距離、`time` は秒（星の瞬きに使う）。
pub fn draw(seed: u64, view: Rect, meters: f32, time: f32) {
    let palette = palette_at(meters);
    draw_sky(&palette, view);
    draw_stars(seed, &palette, view, time);
    for (index, layer) in LAYERS.iter().enumerate() {
        draw_layer(seed, index as u64, layer, &palette, view);
    }
}

fn draw_sky(palette: &Palette, view: Rect) {
    let band = VIEW_HEIGHT / SKY_BANDS as f32;
    for i in 0..SKY_BANDS {
        let color = mix(
            palette.sky_top,
            palette.sky_bottom,
            i as f32 / (SKY_BANDS - 1) as f32,
        );
        let top = if i == 0 { view.y } else { i as f32 * band };
        let bottom = if i + 1 == SKY_BANDS {
            view.bottom()
        } else {
            (i + 1) as f32 * band
        };
        draw_rectangle(view.x, top, view.w, bottom - top, color);
    }
}

/// 星。空の上の方にだけ、ほとんど動かずに散らばる。
fn draw_stars(seed: u64, palette: &Palette, view: Rect, time: f32) {
    if palette.stars <= 0.01 {
        return;
    }
    const SPEED: f32 = 0.03;
    const CELL: f32 = 160.0;
    let shift = view.x * (1.0 - SPEED);
    let first = ((view.x - shift) / CELL).floor() as i64;
    let last = ((view.right() - shift) / CELL).ceil() as i64;
    let rows = (VIEW_HEIGHT * 0.55 / CELL) as i64;
    for column in first..=last {
        for row in 0..rows {
            if hash(seed, column, 1000 + row as u64) > 0.45 {
                continue;
            }
            let x = shift + (column as f32 + hash(seed, column, 2000 + row as u64)) * CELL;
            let y = (row as f32 + hash(seed, column, 3000 + row as u64)) * CELL;
            // ゆっくり瞬かせる。星ごとに速さと位相をずらす。
            let phase = hash(seed, column, 4000 + row as u64) * std::f32::consts::TAU;
            let twinkle = 0.6 + 0.4 * (time * (1.0 + phase * 0.3) + phase).sin();
            let alpha = palette.stars * twinkle * 0.8;
            draw_rectangle(
                snap(x),
                snap(y),
                DOT * 0.5,
                DOT * 0.5,
                Color::new(0.9, 0.92, 1.0, alpha),
            );
        }
    }
}

fn draw_layer(seed: u64, index: u64, layer: &Layer, palette: &Palette, view: Rect) {
    let color = mix(palette.far, palette.near, layer.depth);
    // 層はカメラの `speed` 倍の速さで流れる。層の上での位置 `local` は、ワールドでは `shift + local`。
    let shift = view.x * (1.0 - layer.speed);
    let first = ((view.x - shift) / SLOT).floor() as i64 - 1;
    let last = ((view.right() - shift) / SLOT).ceil() as i64;
    let ground = view.bottom();
    for slot in first..=last {
        let r = |k: u64| hash(seed, slot, index * 100 + k);
        let width = snap(layer.width.0 + (layer.width.1 - layer.width.0) * r(0));
        let height = snap(layer.height.0 + (layer.height.1 - layer.height.0) * r(1));
        let gap = layer.gap.0 + (layer.gap.1 - layer.gap.0) * r(2);
        if gap > SLOT - width {
            continue;
        }
        let left = snap(shift + slot as f32 * SLOT + gap * 0.5);
        let top = snap(VIEW_HEIGHT - height);
        draw_rectangle(left, top, width, ground - top, color);
        // 屋上の段差やアンテナで、ただの箱に見えないようにする。
        match (r(3) * 3.0) as u32 {
            0 => draw_rectangle(
                left + snap(width * 0.25),
                top - DOT * 2.0,
                snap(width * 0.5),
                DOT * 2.0,
                color,
            ),
            1 => draw_rectangle(
                left + snap(width * 0.6),
                top - DOT * 5.0,
                DOT,
                DOT * 5.0,
                color,
            ),
            _ => {}
        }
        draw_windows(seed, slot, index, layer, palette, left, top, width);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_windows(
    seed: u64,
    slot: i64,
    index: u64,
    layer: &Layer,
    palette: &Palette,
    left: f32,
    top: f32,
    width: f32,
) {
    if palette.windows <= 0.01 {
        return;
    }
    // 遠い層ほど窓を小さく暗くする。窓は 2 ドット間隔の格子で、点いている窓だけ描く。
    let size = if layer.depth < 0.25 { DOT * 0.5 } else { DOT };
    let step = size * 3.0;
    let alpha = palette.windows * (0.25 + 0.35 * layer.depth);
    let columns = ((width - step) / step) as i64;
    let rows = ((VIEW_HEIGHT - top - step) / step).min(40.0) as i64;
    for column in 0..columns {
        for row in 0..rows {
            let key = 10_000 + index * 1_000 + (column * 41 + row) as u64;
            if hash(seed, slot, key) > 0.18 {
                continue;
            }
            let x = left + step * (column as f32 + 1.0) - size;
            let y = top + step * (row as f32 + 1.0);
            draw_rectangle(
                x,
                y,
                size,
                size,
                Color {
                    a: alpha,
                    ..WINDOW_COLOR
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sky_moves_through_dusk_night_and_dawn_and_loops() {
        let dusk = palette_at(0.0);
        let night = palette_at(1200.0);
        let dawn = palette_at(2600.0);
        // 夜は暗く、星が多い。
        assert!(night.sky_top.b < dusk.sky_top.b + 0.2 && night.sky_bottom.r < dusk.sky_bottom.r);
        assert!(night.stars > dusk.stars && night.stars > dawn.stars);
        // 一巡したら同じ色に戻る。
        let looped = palette_at(CYCLE + 300.0);
        let first = palette_at(300.0);
        assert!((looped.sky_bottom.r - first.sky_bottom.r).abs() < 1e-4);
    }

    #[test]
    fn hash_is_stable_and_in_range() {
        for slot in -50..50 {
            let value = hash(7, slot, 3);
            assert!((0.0..1.0).contains(&value));
            assert_eq!(value, hash(7, slot, 3));
        }
        assert_ne!(hash(7, 1, 3), hash(8, 1, 3));
    }
}
