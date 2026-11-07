use macroquad::prelude::*;

use crate::sprites::Sprites;

/// 天井や浮いたブロックなど、ワイヤーを刺せて自機がぶつかる地形。
/// すべて軸に沿った長方形で表す。座標は y 下向きで、画面の上端が y = 0。
#[derive(Default)]
pub struct Level {
    pub blocks: Vec<Rect>,
}

/// 円を地形の外へ押し戻した結果。
pub struct Contact {
    pub position: Vec2,
    /// 押し戻した向き（地形から外へ向く単位ベクトル）。
    pub normal: Vec2,
}

impl Level {
    pub fn add(&mut self, block: Rect) {
        self.blocks.push(block);
    }

    /// 右端が `x` より左にある地形を捨てる。画面の後ろへ流れたものを消すのに使う。
    pub fn remove_before(&mut self, x: f32) {
        self.blocks.retain(|b| b.right() >= x);
    }

    /// `origin` から `dir` の向きへ最大 `max_len` まで線を伸ばし、最初に当たる地形の点を返す。
    pub fn raycast(&self, origin: Vec2, dir: Vec2, max_len: f32) -> Option<Vec2> {
        let dir = dir.normalize_or_zero();
        if dir == Vec2::ZERO {
            return None;
        }
        self.blocks
            .iter()
            .filter_map(|b| ray_rect(origin, dir, *b))
            .filter(|&t| t <= max_len)
            .min_by(|a, b| a.total_cmp(b))
            .map(|t| origin + dir * t)
    }

    /// 半径 `radius` の円が地形にめり込んでいれば外へ押し戻す。めり込んでいなければ `None`。
    /// 複数の地形に当たるときは順に押し戻し、最後に押し戻した向きを返す。
    pub fn push_out(&self, center: Vec2, radius: f32) -> Option<Contact> {
        let mut position = center;
        let mut normal = None;
        for block in &self.blocks {
            if let Some((pushed, n)) = push_circle_out_of_rect(position, radius, *block) {
                position = pushed;
                normal = Some(n);
            }
        }
        normal.map(|normal| Contact { position, normal })
    }

    /// 刺さった点 `point` がある地形の面の、外へ向く向き（上下左右のどれか）。
    /// 一番近いブロックの、一番近い辺で決める。地形が無ければ下向き（天井の下の面）。
    pub fn surface_normal(&self, point: Vec2) -> Vec2 {
        let Some(block) = self
            .blocks
            .iter()
            .min_by(|a, b| distance_to_rect(point, **a).total_cmp(&distance_to_rect(point, **b)))
        else {
            return Vec2::Y;
        };
        let sides = [
            ((point.y - block.y).abs(), -Vec2::Y),
            ((block.bottom() - point.y).abs(), Vec2::Y),
            ((point.x - block.x).abs(), -Vec2::X),
            ((block.right() - point.x).abs(), Vec2::X),
        ];
        sides
            .into_iter()
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map_or(Vec2::Y, |(_, normal)| normal)
    }

    pub fn draw(&self, sprites: &Sprites) {
        for &block in &self.blocks {
            sprites.draw_block(block);
        }
    }
}

/// 線と長方形が交わる最初の距離。始点が長方形の中にあるときは 0。
fn ray_rect(origin: Vec2, dir: Vec2, rect: Rect) -> Option<f32> {
    let min = rect.point();
    let max = min + rect.size();
    let mut t_near = 0.0_f32;
    let mut t_far = f32::INFINITY;
    for axis in 0..2 {
        let (o, d, lo, hi) = (origin[axis], dir[axis], min[axis], max[axis]);
        if d.abs() < 1e-8 {
            if o < lo || o > hi {
                return None;
            }
            continue;
        }
        let (mut t0, mut t1) = ((lo - o) / d, (hi - o) / d);
        if t0 > t1 {
            std::mem::swap(&mut t0, &mut t1);
        }
        t_near = t_near.max(t0);
        t_far = t_far.min(t1);
        if t_near > t_far {
            return None;
        }
    }
    Some(t_near)
}

fn push_circle_out_of_rect(center: Vec2, radius: f32, rect: Rect) -> Option<(Vec2, Vec2)> {
    let min = rect.point();
    let max = min + rect.size();
    let closest = center.clamp(min, max);
    let offset = center - closest;
    let distance = offset.length();
    if distance >= radius {
        return None;
    }
    if distance > 0.0 {
        let normal = offset / distance;
        return Some((closest + normal * radius, normal));
    }
    // 中心が長方形の中にあるときは、いちばん近い辺から外へ出す。
    let exits = [
        (center.x - min.x, vec2(-1.0, 0.0)),
        (max.x - center.x, vec2(1.0, 0.0)),
        (center.y - min.y, vec2(0.0, -1.0)),
        (max.y - center.y, vec2(0.0, 1.0)),
    ];
    let (depth, normal) = exits
        .into_iter()
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .unwrap();
    Some((center + normal * (depth + radius), normal))
}

/// 点から長方形までの距離。中なら 0。
fn distance_to_rect(point: Vec2, rect: Rect) -> f32 {
    let dx = (rect.x - point.x).max(point.x - rect.right()).max(0.0);
    let dy = (rect.y - point.y).max(point.y - rect.bottom()).max(0.0);
    vec2(dx, dy).length()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ceiling() -> Level {
        let mut level = Level::default();
        level.add(Rect::new(0.0, -100.0, 1000.0, 200.0)); // 下端は y = 100
        level
    }

    #[test]
    fn raycast_hits_ceiling_bottom() {
        let level = ceiling();
        let hit = level.raycast(vec2(500.0, 400.0), vec2(0.0, -1.0), 1000.0);
        assert_eq!(hit, Some(vec2(500.0, 100.0)));
    }

    #[test]
    fn raycast_respects_max_length_and_direction() {
        let level = ceiling();
        assert!(level
            .raycast(vec2(500.0, 400.0), vec2(0.0, -1.0), 250.0)
            .is_none());
        assert!(level
            .raycast(vec2(500.0, 400.0), vec2(0.0, 1.0), 1000.0)
            .is_none());
    }

    #[test]
    fn raycast_picks_nearest_block() {
        let mut level = ceiling();
        level.add(Rect::new(450.0, 200.0, 100.0, 50.0));
        let hit = level.raycast(vec2(500.0, 400.0), vec2(0.0, -1.0), 1000.0);
        assert_eq!(hit, Some(vec2(500.0, 250.0)));
    }

    #[test]
    fn surface_normal_points_out_of_the_nearest_side() {
        let level = Level {
            blocks: vec![Rect::new(0.0, 0.0, 200.0, 100.0)],
        };
        assert_eq!(level.surface_normal(vec2(50.0, 100.0)), Vec2::Y);
        assert_eq!(level.surface_normal(vec2(50.0, 0.0)), -Vec2::Y);
        assert_eq!(level.surface_normal(vec2(0.0, 50.0)), -Vec2::X);
        assert_eq!(level.surface_normal(vec2(200.0, 40.0)), Vec2::X);
    }

    #[test]
    fn push_out_moves_circle_below_ceiling() {
        let level = ceiling();
        let contact = level.push_out(vec2(500.0, 105.0), 10.0).unwrap();
        assert_eq!(contact.position, vec2(500.0, 110.0));
        assert_eq!(contact.normal, vec2(0.0, 1.0));
        assert!(level.push_out(vec2(500.0, 111.0), 10.0).is_none());
    }

    #[test]
    fn remove_before_drops_passed_blocks() {
        let mut level = Level::default();
        level.add(Rect::new(0.0, 0.0, 100.0, 10.0));
        level.add(Rect::new(200.0, 0.0, 100.0, 10.0));
        level.remove_before(150.0);
        assert_eq!(level.blocks.len(), 1);
        assert_eq!(level.blocks[0].x, 200.0);
    }
}
