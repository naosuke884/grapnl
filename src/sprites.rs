//! ドット絵。絵は文字の並びでここに書き、起動時にテクスチャにする。素材のファイルは使わない。
//! 文字 1 つがドット 1 つで、`.` は透明。色は絵ごとのパレットで決める。
//! 拡大しても滲まないよう、テクスチャは最近傍で拡大する。

use macroquad::prelude::*;

/// 自機と敵のドット 1 つのワールドでの大きさ。16:9 の画面で約 1.5 ピクセルになる。
pub const SPRITE_DOT: f32 = 4.0;
/// 地形の模様のドット 1 つの大きさ。
const TILE_DOT: f32 = 4.0;

/// 自機。8×8 で、ワールドで 32（当たり判定の直径 28 より少し大きい）。右を向いた目がある。
const PLAYER: &[&str] = &[
    "..OOOO..", //
    ".OWWWWO.", "OWWWWKWO", "OWWWWKWO", "OWWWWWWO", "OLWWWWLO", ".OLLLLO.", "..OOOO..",
];
const PLAYER_PALETTE: &[(char, Color)] = &[
    ('O', Color::new(0.16, 0.18, 0.26, 1.0)),
    ('W', Color::new(0.94, 0.94, 0.98, 1.0)),
    ('L', Color::new(0.70, 0.74, 0.86, 1.0)),
    ('K', Color::new(0.10, 0.12, 0.20, 1.0)),
];

/// 揺れながら流れる敵。16×16 で、ワールドで 64（当たり判定の直径と同じ）。
const FLOATER: &[&str] = &[
    ".....OOOOOO.....",
    "...OOGGGGGGOO...",
    "..OGGHHGGGGGGO..",
    ".OGGHHGGGGGGGGO.",
    ".OGHHGGGGGGGGGO.",
    "OGGHGGWWGGWWGGGO",
    "OGGGGGWKGGWKGGGO",
    "OGGGGGWKGGWKGGGO",
    "OGGGGGGGGGGGGGGO",
    "OGGGGGGGGGGGGGGO",
    "OGGGGGGKKKKGGGDO",
    ".OGGGGGGGGGGGDO.",
    ".OGGGGGGGGGGDDO.",
    "..ODGGGGGGGDDO..",
    "...OODDDDDDOO...",
    ".....OOOOOO.....",
];
/// 上下に往復する敵。揺れる敵の上下にひれを付け、縦に動くことが分かるようにする。16×22 で、体は真ん中の 16 行。
const BOBBER: &[&str] = &[
    ".......OO.......",
    "......OGGO......",
    ".....OGHGDO.....",
    ".....OOOOOO.....",
    "...OOGGGGGGOO...",
    "..OGGHHGGGGGGO..",
    ".OGGHHGGGGGGGGO.",
    ".OGHHGGGGGGGGGO.",
    "OGGHGGWWGGWWGGGO",
    "OGGGGGWKGGWKGGGO",
    "OGGGGGWKGGWKGGGO",
    "OGGGGGGGGGGGGGGO",
    "OGGGGGGGGGGGGGGO",
    "OGGGGGGKKKKGGGDO",
    ".OGGGGGGGGGGGDO.",
    ".OGGGGGGGGGGDDO.",
    "..ODGGGGGGGDDO..",
    "...OODDDDDDOO...",
    ".....OOOOOO.....",
    ".....ODGGDO.....",
    "......ODDO......",
    ".......OO.......",
];
/// 突進する敵。左へ尖ったとげと、つり上がった目。28×16 で、体の中心が絵の中心に来るよう右を空ける。
const DASHER: &[&str] = &[
    "...........OOOOOO...........",
    ".........OOGGGGGGOO.........",
    "........OGGHHGGGGGGO........",
    ".......OGGHHGGGGGGGGO.......",
    ".......OGHHGGGGGGGGGO.......",
    "...OOOOGGWGGGGWGGGGGGO......",
    ".OOGGGGGGWWGGWWGGGGGGO......",
    "OGGGGGGGGWKKGKWGGGGGGO......",
    "OGGGGGGGGGKKGKKGGGGGGO......",
    ".OODDDDGGGGGGGGGGGGGGO......",
    "...OOOOGGGGGGGGGGGGGDO......",
    ".......OGGKKKKKKGGGDO.......",
    ".......OGGGGGGGGGGDDO.......",
    "........ODGGGGGGGDDO........",
    ".........OODDDDDDOO.........",
    "...........OOOOOO...........",
];
const ENEMY_PALETTE: &[(char, Color)] = &[
    ('O', Color::new(0.08, 0.22, 0.14, 1.0)),
    ('G', Color::new(0.47, 0.86, 0.55, 1.0)),
    ('H', Color::new(0.78, 0.98, 0.80, 1.0)),
    ('D', Color::new(0.27, 0.62, 0.38, 1.0)),
    ('W', Color::new(0.97, 0.98, 0.95, 1.0)),
    ('K', Color::new(0.08, 0.10, 0.12, 1.0)),
];

/// 天井と浮いたブロックの模様。8×8 を敷き詰める。石を積んだ形。
const TILE: &[&str] = &[
    "MMMMMMMS", //
    "MLMMMMMS", "MMMMMMMS", "SSSSSSSS", "MMMSMMMM", "MMMSMLMM", "MMMSMMMM", "SSSSSSSS",
];
const TILE_PALETTE: &[(char, Color)] = &[
    ('M', Color::new(0.20, 0.24, 0.33, 1.0)),
    ('L', Color::new(0.27, 0.32, 0.43, 1.0)),
    ('S', Color::new(0.13, 0.15, 0.22, 1.0)),
];
/// ブロックの縁。暗い輪郭で囲み、その内側の上と左を明るく、下と右を暗くして、石の塊に見せる。
const OUTLINE_COLOR: Color = Color::new(0.05, 0.06, 0.10, 1.0);
const LIT_COLOR: Color = Color::new(0.42, 0.55, 0.78, 1.0);
const SHADE_COLOR: Color = Color::new(0.14, 0.17, 0.25, 1.0);

/// 撃てば刺さる点に出す照準。白で描き、使うときに色を掛ける。7×7。
const AIM: &[&str] = &[
    "..XXX..", //
    ".X...X.", "X.....X", "X.....X", "X.....X", ".X...X.", "..XXX..",
];
/// ワイヤーが届かない向きで、伸び切る所に出す ×。7×7。
const OUT_OF_REACH: &[&str] = &[
    "XX...XX", //
    ".XX.XX.", "..XXX..", "...X...", "..XXX..", ".XX.XX.", "XX...XX",
];
const MARK_PALETTE: &[(char, Color)] = &[('X', WHITE)];

pub struct Sprites {
    pub player: Texture2D,
    pub floater: Texture2D,
    pub bobber: Texture2D,
    pub dasher: Texture2D,
    /// 突進する敵が溜めている間に、明滅させる白い絵。
    pub dasher_flash: Texture2D,
    tile: Texture2D,
    pub aim: Texture2D,
    pub out_of_reach: Texture2D,
}

impl Sprites {
    pub fn load() -> Self {
        Self {
            player: texture(&image(PLAYER, PLAYER_PALETTE)),
            floater: texture(&image(FLOATER, ENEMY_PALETTE)),
            bobber: texture(&image(BOBBER, ENEMY_PALETTE)),
            dasher: texture(&image(DASHER, ENEMY_PALETTE)),
            dasher_flash: texture(&silhouette(&image(DASHER, ENEMY_PALETTE))),
            tile: texture(&image(TILE, TILE_PALETTE)),
            aim: texture(&image(AIM, MARK_PALETTE)),
            out_of_reach: texture(&image(OUT_OF_REACH, MARK_PALETTE)),
        }
    }

    /// 地形のブロックを、模様を敷き詰めて縁を付けて描く。
    pub fn draw_block(&self, block: Rect) {
        // 縁がドットの格子からずれないよう、描く範囲を格子に揃える（判定は元のまま）。
        let snap = |v: f32| (v / TILE_DOT).round() * TILE_DOT;
        let (left, top) = (snap(block.x), snap(block.y));
        let block = Rect::new(
            left,
            top,
            snap(block.right()) - left,
            snap(block.bottom()) - top,
        );
        // 模様は輪郭の内側だけに敷く。角のドットを空けて、角を丸く見せる。
        let outer = block;
        let block = Rect::new(
            block.x + TILE_DOT,
            block.y + TILE_DOT,
            block.w - 2.0 * TILE_DOT,
            block.h - 2.0 * TILE_DOT,
        );
        let size = TILE.len() as f32 * TILE_DOT;
        // 模様はワールドの格子に合わせ、ブロックの位置によって模様がずれないようにする。
        let mut y = (block.y / size).floor() * size;
        while y < block.bottom() {
            let mut x = (block.x / size).floor() * size;
            while x < block.right() {
                let cell = Rect::new(x, y, size, size);
                if let Some(visible) = cell.intersect(block) {
                    let source = Rect::new(
                        (visible.x - x) / TILE_DOT,
                        (visible.y - y) / TILE_DOT,
                        visible.w / TILE_DOT,
                        visible.h / TILE_DOT,
                    );
                    draw_texture_ex(
                        &self.tile,
                        visible.x,
                        visible.y,
                        WHITE,
                        DrawTextureParams {
                            dest_size: Some(visible.size()),
                            source: Some(source),
                            ..Default::default()
                        },
                    );
                }
                x += size;
            }
            y += size;
        }
        let d = TILE_DOT;
        let (x, y, w, h) = (outer.x, outer.y, outer.w, outer.h);
        // 内側の明るい面と暗い面。
        draw_rectangle(x + d, y + d, w - 2.0 * d, d, LIT_COLOR);
        draw_rectangle(x + d, y + d, d, h - 2.0 * d, LIT_COLOR);
        draw_rectangle(x + d, y + h - 2.0 * d, w - 2.0 * d, d, SHADE_COLOR);
        draw_rectangle(x + w - 2.0 * d, y + 2.0 * d, d, h - 3.0 * d, SHADE_COLOR);
        // 暗い輪郭。角のドットは抜いて、角を少し丸める。
        draw_rectangle(x + d, y, w - 2.0 * d, d, OUTLINE_COLOR);
        draw_rectangle(x + d, y + h - d, w - 2.0 * d, d, OUTLINE_COLOR);
        draw_rectangle(x, y + d, d, h - 2.0 * d, OUTLINE_COLOR);
        draw_rectangle(x + w - d, y + d, d, h - 2.0 * d, OUTLINE_COLOR);
    }
}

/// 絵を `center` を中心に、ドット 1 つを `SPRITE_DOT` の大きさで描く。`color` を掛ける。
pub fn draw_sprite(texture: &Texture2D, center: Vec2, color: Color) {
    let size = vec2(texture.width(), texture.height()) * SPRITE_DOT;
    draw_texture_ex(
        texture,
        center.x - size.x / 2.0,
        center.y - size.y / 2.0,
        color,
        DrawTextureParams {
            dest_size: Some(size),
            ..Default::default()
        },
    );
}

/// ドットの格子の上に、`a` から `b` へ 1 ドットの太さの線を描く。
/// `with_start` が偽なら始点のドットを描かない。折れ線をつなぐとき、継ぎ目のドットを重ねて濃くしない。
pub fn draw_dot_line(a: Vec2, b: Vec2, color: Color, with_start: bool) {
    for (i, (x, y)) in dot_line(a, b).into_iter().enumerate() {
        if i > 0 || with_start {
            draw_rectangle(
                x as f32 * SPRITE_DOT,
                y as f32 * SPRITE_DOT,
                SPRITE_DOT,
                SPRITE_DOT,
                color,
            );
        }
    }
}

/// `a` から `b` までの線が通るドットの格子の番号（ブレゼンハムの方法）。始点と終点を含む。
fn dot_line(a: Vec2, b: Vec2) -> Vec<(i32, i32)> {
    let cell = |p: Vec2| {
        (
            (p.x / SPRITE_DOT).floor() as i32,
            (p.y / SPRITE_DOT).floor() as i32,
        )
    };
    let (mut x, mut y) = cell(a);
    let (x1, y1) = cell(b);
    let dx = (x1 - x).abs();
    let dy = -(y1 - y).abs();
    let sx = if x < x1 { 1 } else { -1 };
    let sy = if y < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut cells = Vec::with_capacity((dx - dy) as usize + 1);
    loop {
        cells.push((x, y));
        if x == x1 && y == y1 {
            return cells;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

/// 中心が `center` で、一辺が `size` に近いドットの倍数の正方形を、ドットの格子に合わせて描く。破片に使う。
pub fn draw_dot_square(center: Vec2, size: f32, color: Color) {
    let side = (size / SPRITE_DOT).round().max(1.0) * SPRITE_DOT;
    let x = ((center.x - side / 2.0) / SPRITE_DOT).round() * SPRITE_DOT;
    let y = ((center.y - side / 2.0) / SPRITE_DOT).round() * SPRITE_DOT;
    draw_rectangle(x, y, side, side, color);
}

/// フックの銀色と、爪の先の暗い色。
const HOOK_COLOR: Color = Color::new(0.86, 0.88, 0.92, 1.0);
const HOOK_TIP_COLOR: Color = Color::new(0.50, 0.54, 0.62, 1.0);
/// フックの形。上の地形に刺さったときの、刺さった点のマスから数えたドットの位置。
/// 横は右が正、縦は下（地形の外）が正。3 本の爪の先は地形に 1 ドット食い込み（地形に隠れる）、
/// 柄の端の輪にワイヤーを結ぶ。フックは地形に刺さって動かず、ワイヤーは輪を中心に振れる。
///
/// ```text
/// +..+..+  -1   + は爪の先。地形の中
/// #..#..#   0   地形の面
/// #..#..#   1
/// .#.#.#.   2
/// ..###..   3
/// ...#...   4
/// ..#.#..   5   輪
/// ...#...   6   輪の下。ここからワイヤーをつなぐ
/// ```
const HOOK_LINES: [((i32, i32), (i32, i32)); 3] =
    [((0, -1), (0, 4)), ((-3, -1), (-3, 1)), ((3, -1), (3, 1))];
const HOOK_DIAGONALS: [((i32, i32), (i32, i32)); 2] = [((-3, 1), (-1, 3)), ((3, 1), (1, 3))];
const HOOK_RING: [(i32, i32); 3] = [(-1, 5), (1, 5), (0, 6)];
const HOOK_TIPS: [(i32, i32); 3] = [(-3, -1), (0, -1), (3, -1)];
const HOOK_TAIL: (i32, i32) = (0, 6);

/// 向きを上下左右の 4 方向に丸めた整数の向き。斜めはドットの形が崩れるので使わない。
fn direction4(toward: Vec2) -> IVec2 {
    if toward.x.abs() > toward.y.abs() {
        ivec2(toward.x.signum() as i32, 0)
    } else if toward.y < 0.0 {
        ivec2(0, -1)
    } else {
        ivec2(0, 1)
    }
}

/// ドットの格子の番号。
type Cell = (i32, i32);

/// フックを形作るドットの格子の番号と色。`tail` は柄の端のマス。
fn hook_cells(point: Vec2, toward: Vec2) -> (Vec<(Cell, Color)>, Cell) {
    let u = direction4(toward);
    // 形の横（右が正）は、ワイヤーの向きを時計回りに 90 度回した向き。下へ引くとき右を向く。
    let side = ivec2(-u.y, u.x);
    let origin = ivec2(
        (point.x / SPRITE_DOT).floor() as i32,
        (point.y / SPRITE_DOT).floor() as i32,
    );
    let place = |(x, y): (i32, i32)| {
        let cell = origin + side * x + u * y;
        (cell.x, cell.y)
    };
    let center = |(x, y): (i32, i32)| vec2(x as f32 + 0.5, y as f32 + 0.5) * SPRITE_DOT;
    let mut cells = Vec::new();
    for (a, b) in HOOK_LINES.iter().chain(&HOOK_DIAGONALS) {
        for cell in dot_line(center(place(*a)), center(place(*b))) {
            cells.push((cell, HOOK_COLOR));
        }
    }
    for ring in HOOK_RING {
        cells.push((place(ring), HOOK_COLOR));
    }
    for tip in HOOK_TIPS {
        cells.push((place(tip), HOOK_TIP_COLOR));
    }
    (cells, place(HOOK_TAIL))
}

/// ワイヤーの先のフックを、ワイヤーと同じくドットの格子の上に描く。
/// `point` は刺さった点、`toward` は柄の向き。刺さっている間は面の外向き（上下左右）、
/// 戻っている間は自機への向きを渡す。上下左右に丸めて描く。ワイヤーは `hook_tail` からつなぎ、フックはその上に描く。
/// 地形より先に描けば、地形に食い込んだ爪の先は地形に隠れる。
pub fn draw_dot_hook(point: Vec2, toward: Vec2) {
    for ((x, y), color) in hook_cells(point, toward).0 {
        draw_rectangle(
            x as f32 * SPRITE_DOT,
            y as f32 * SPRITE_DOT,
            SPRITE_DOT,
            SPRITE_DOT,
            color,
        );
    }
}

/// `draw_dot_hook` と同じ置き方をしたフックの、ワイヤーをつなぐ輪の下のマスの中心。
pub fn hook_tail(point: Vec2, toward: Vec2) -> Vec2 {
    let tail = hook_cells(point, toward).1;
    vec2(tail.0 as f32 + 0.5, tail.1 as f32 + 0.5) * SPRITE_DOT
}

fn texture(image: &Image) -> Texture2D {
    let texture = Texture2D::from_image(image);
    texture.set_filter(FilterMode::Nearest);
    texture
}

/// 文字の並びを絵にする。パレットに無い文字は透明。
fn image(rows: &[&str], palette: &[(char, Color)]) -> Image {
    let width = rows[0].chars().count() as u16;
    let height = rows.len() as u16;
    let mut image = Image::gen_image_color(width, height, BLANK);
    for (y, row) in rows.iter().enumerate() {
        for (x, c) in row.chars().enumerate() {
            if let Some(&(_, color)) = palette.iter().find(|(key, _)| *key == c) {
                image.set_pixel(x as u32, y as u32, color);
            }
        }
    }
    image
}

/// 透明でないドットをすべて白にした絵。
fn silhouette(image: &Image) -> Image {
    let mut white = image.clone();
    for y in 0..image.height() as u32 {
        for x in 0..image.width() as u32 {
            if image.get_pixel(x, y).a > 0.0 {
                white.set_pixel(x, y, WHITE);
            }
        }
    }
    white
}

#[cfg(test)]
mod tests {
    use super::*;

    type Palette = &'static [(char, Color)];
    const ALL: [(&str, &[&str], Palette); 7] = [
        ("AIM", AIM, MARK_PALETTE),
        ("OUT_OF_REACH", OUT_OF_REACH, MARK_PALETTE),
        ("PLAYER", PLAYER, PLAYER_PALETTE),
        ("FLOATER", FLOATER, ENEMY_PALETTE),
        ("BOBBER", BOBBER, ENEMY_PALETTE),
        ("DASHER", DASHER, ENEMY_PALETTE),
        ("TILE", TILE, TILE_PALETTE),
    ];

    #[test]
    fn every_row_has_the_same_width_and_known_colors() {
        for (name, rows, palette) in ALL {
            let width = rows[0].chars().count();
            for (y, row) in rows.iter().enumerate() {
                assert_eq!(row.chars().count(), width, "{name} の {y} 行目の幅");
                for c in row.chars() {
                    assert!(
                        c == '.' || palette.iter().any(|(key, _)| *key == c),
                        "{name} の {y} 行目の {c:?} がパレットに無い"
                    );
                }
            }
        }
    }

    #[test]
    fn dot_lines_are_connected_and_end_where_asked() {
        for (a, b) in [
            (vec2(0.0, 0.0), vec2(40.0, 12.0)),
            (vec2(30.0, 50.0), vec2(-21.0, -3.0)),
            (vec2(5.0, 5.0), vec2(5.0, 5.0)),
        ] {
            let cells = dot_line(a, b);
            assert_eq!(
                cells[0],
                ((a.x / 4.0).floor() as i32, (a.y / 4.0).floor() as i32)
            );
            assert_eq!(
                *cells.last().unwrap(),
                ((b.x / 4.0).floor() as i32, (b.y / 4.0).floor() as i32)
            );
            for pair in cells.windows(2) {
                let (dx, dy) = (pair[1].0 - pair[0].0, pair[1].1 - pair[0].1);
                assert!(dx.abs() <= 1 && dy.abs() <= 1 && (dx, dy) != (0, 0));
            }
        }
    }

    /// フックの形を文字で確かめる。`cargo test hook_shapes -- --nocapture` で見られる。
    #[test]
    fn hook_shapes() {
        for (name, toward) in [
            ("天井に刺さる", vec2(0.0, 1.0)),
            ("斜め下へ戻る", vec2(1.0, 1.2)),
            ("右の面に刺さる", vec2(1.0, 0.0)),
        ] {
            let point = vec2(10.5, 10.5) * SPRITE_DOT;
            let mut grid = [['.'; 21]; 21];
            let (cells, tail) = hook_cells(point, toward);
            for ((x, y), color) in cells {
                grid[y as usize][x as usize] = if color == HOOK_TIP_COLOR { '+' } else { '#' };
            }
            grid[tail.1 as usize][tail.0 as usize] = 'T';
            println!("{name}");
            for row in grid {
                println!("{}", row.iter().collect::<String>());
            }
        }
    }

    #[test]
    fn hook_points_its_shaft_along_the_wire() {
        let point = vec2(10.5, 10.5) * SPRITE_DOT;
        assert_eq!(hook_cells(point, vec2(0.0, 1.0)).1, (10, 16));
        assert_eq!(hook_cells(point, vec2(0.1, -1.0)).1, (10, 4));
        assert_eq!(hook_cells(point, vec2(-1.0, 0.0)).1, (4, 10));
        assert_eq!(hook_cells(point, vec2(1.0, 0.9)).1, (16, 10));
    }

    #[test]
    fn sprites_cover_their_hit_circles() {
        use crate::enemy::ENEMY_RADIUS;
        use crate::player::PLAYER_RADIUS;
        assert!(PLAYER.len() as f32 * SPRITE_DOT >= PLAYER_RADIUS * 2.0);
        assert_eq!(FLOATER.len() as f32 * SPRITE_DOT, ENEMY_RADIUS * 2.0);
        assert_eq!(DASHER.len() as f32 * SPRITE_DOT, ENEMY_RADIUS * 2.0);
        // 体（16 行）の上下にひれが同じ数ずつ付き、体の中心が絵の中心に来る。
        assert_eq!(BOBBER.len(), FLOATER.len() + 6);
        assert_eq!(&BOBBER[3..19], FLOATER);
    }
}
