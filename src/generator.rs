use macroquad::prelude::*;
use macroquad::rand::RandGenerator;

use crate::level::Level;

/// 隣り合う刺し先（天井かブロック）の横の間隔の上限。ワイヤーの最大の長さより十分短くする。
const MAX_ANCHOR_GAP: f32 = 350.0;
/// 刺し先はこれより上に置く（画面の上半分）。
const ANCHOR_LOWEST: f32 = 450.0;
/// 天井の上端。画面より十分上に置き、天井の上を回り込めないようにする。
const CEILING_TOP: f32 = -1000.0;
/// 開始位置のあたりの平らな天井の高さ。最初の数秒は落ち着いて振れるようにする。
const START_CEILING_BOTTOM: f32 = 120.0;
/// 天井の区間の幅と、下端の高さの範囲。
const CEILING_WIDTH: (f32, f32) = (200.0, 500.0);
const CEILING_BOTTOM: (f32, f32) = (80.0, 260.0);
/// 進むほど地形を難しくする。平らな天井が終わってから、この距離で最も難しくなる（3000 m）。
/// 敵の出る間隔が詰まり切る距離（`enemy::SPACING_RAMP`、1500 m）より長くし、その後も変わり続けるようにする。
const DIFFICULTY_RAMP: f32 = 30_000.0;
/// 天井のない区間（穴）の幅の範囲と、出る確率。左が始めのころ、右が最も難しいとき。
const HOLE_WIDTH: Ramp = Ramp((300.0, 900.0), (600.0, 1600.0));
const HOLE_CHANCE: (f32, f32) = (0.35, 0.6);
/// 浮いたブロックの大きさと、上端の高さの範囲。ブロックは進むほど細くし、狙いにくくする。
const BLOCK_WIDTH: Ramp = Ramp((60.0, 140.0), (40.0, 80.0));
const BLOCK_HEIGHT: (f32, f32) = (30.0, 50.0);
const BLOCK_TOP: (f32, f32) = (160.0, 380.0);
/// 穴の中で、前の刺し先からブロックまで空ける間隔の範囲。上限は MAX_ANCHOR_GAP 以下にする。
/// 進むほど空け、1 回の振りで次へ届かせる必要を増やす。
const BLOCK_SPACING: Ramp = Ramp((150.0, 300.0), (260.0, MAX_ANCHOR_GAP));
/// 天井の下に障害物として浮いたブロックを置く確率。
const EXTRA_BLOCK_CHANCE: (f32, f32) = (0.3, 0.5);
/// 天井の下に置くブロックの上端は、天井の下端から少なくともこれだけ下げる。
const EXTRA_BLOCK_BELOW: f32 = 80.0;

/// 始めのころの範囲と、最も難しいときの範囲。難しさ（0 から 1）に応じて両端を間で取る。
#[derive(Clone, Copy)]
struct Ramp((f32, f32), (f32, f32));

impl Ramp {
    fn at(self, difficulty: f32) -> (f32, f32) {
        let Ramp((low0, high0), (low1, high1)) = self;
        (lerp(low0, low1, difficulty), lerp(high0, high1, difficulty))
    }
}

fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t
}

/// `x` の地形の難しさ（0 から 1）。`flat_until` までは 0 で、そこから `DIFFICULTY_RAMP` かけて 1 にする。
fn difficulty(x: f32, flat_until: f32) -> f32 {
    ((x - flat_until) / DIFFICULTY_RAMP).clamp(0.0, 1.0)
}

/// 天井と浮いたブロックを右へ作り足していく。
pub struct Generator {
    /// まだ地形を作っていない範囲の左端。
    next_x: f32,
    /// 平らな天井をここまで続ける。
    flat_until: f32,
    /// これまでに作った刺し先（天井かブロック）の右端のうち、いちばん右。
    last_anchor: f32,
    rng: RandGenerator,
}

impl Generator {
    /// `start_x` から地形を作り始め、`flat_until` までは平らな天井にする。
    pub fn new(start_x: f32, flat_until: f32, seed: u64) -> Self {
        let rng = RandGenerator::new();
        rng.srand(seed);
        Self {
            next_x: start_x,
            flat_until,
            last_anchor: start_x,
            rng,
        }
    }

    /// `until_x` まで地形を作り足す。
    pub fn fill(&mut self, level: &mut Level, until_x: f32) {
        while self.next_x < until_x {
            if self.next_x < self.flat_until {
                let width = self.flat_until - self.next_x;
                self.add_ceiling(level, width, START_CEILING_BOTTOM);
                continue;
            }
            let difficulty = difficulty(self.next_x, self.flat_until);
            let hole_chance = lerp(HOLE_CHANCE.0, HOLE_CHANCE.1, difficulty);
            if self.rng.gen_range(0.0, 1.0) < hole_chance {
                self.add_hole(level, difficulty);
            } else {
                let width = self.range(CEILING_WIDTH);
                let bottom = self.range(CEILING_BOTTOM);
                let start = self.next_x;
                self.add_ceiling(level, width, bottom);
                let extra_chance = lerp(EXTRA_BLOCK_CHANCE.0, EXTRA_BLOCK_CHANCE.1, difficulty);
                if self.rng.gen_range(0.0, 1.0) < extra_chance {
                    let x = start + self.rng.gen_range(0.0, width * 0.5);
                    self.add_block(level, x, bottom + EXTRA_BLOCK_BELOW, difficulty);
                }
            }
        }
    }

    fn add_ceiling(&mut self, level: &mut Level, width: f32, bottom: f32) {
        level.add(Rect::new(
            self.next_x,
            CEILING_TOP,
            width,
            bottom - CEILING_TOP,
        ));
        self.next_x += width;
        self.last_anchor = self.last_anchor.max(self.next_x);
    }

    /// 天井のない区間を作る。刺し先の間隔が MAX_ANCHOR_GAP を超えないよう、中に浮いたブロックを並べる。
    fn add_hole(&mut self, level: &mut Level, difficulty: f32) {
        let end = self.next_x + self.range(HOLE_WIDTH.at(difficulty));
        // 穴が続いても間隔が開きすぎないよう、直前の区間で最後に置いた刺し先から数える。
        while end - self.last_anchor > MAX_ANCHOR_GAP {
            let x = self.last_anchor + self.range(BLOCK_SPACING.at(difficulty));
            self.add_block(level, x, BLOCK_TOP.0, difficulty);
        }
        self.next_x = end;
    }

    /// `x` に浮いたブロックを置く。上端は `min_top` 以上でランダムに決める。幅は `difficulty` で細くする。
    fn add_block(&mut self, level: &mut Level, x: f32, min_top: f32, difficulty: f32) {
        let width = self.range(BLOCK_WIDTH.at(difficulty));
        let height = self.range(BLOCK_HEIGHT);
        let top = self
            .rng
            .gen_range(min_top.max(BLOCK_TOP.0), BLOCK_TOP.1.max(min_top));
        // 下端が刺し先の下限を越えないようにする。
        let top = top.min(ANCHOR_LOWEST - height);
        level.add(Rect::new(x, top, width, height));
        self.last_anchor = self.last_anchor.max(x + width);
    }

    fn range(&self, (low, high): (f32, f32)) -> f32 {
        self.rng.gen_range(low, high)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLAT: f32 = 1400.0;

    fn generate(seed: u64, until: f32) -> Level {
        let mut level = Level::default();
        let mut generator = Generator::new(0.0, FLAT, seed);
        generator.fill(&mut level, until);
        level
    }

    #[test]
    fn anchors_are_never_too_far_apart() {
        for seed in 0..20 {
            let level = generate(seed, 30_000.0);
            let mut spans: Vec<(f32, f32)> =
                level.blocks.iter().map(|b| (b.x, b.right())).collect();
            spans.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut reach = spans[0].1;
            for (left, right) in spans {
                assert!(
                    left - reach <= MAX_ANCHOR_GAP + 1e-3,
                    "seed {seed}: {reach} から {left} まで刺し先がない"
                );
                reach = reach.max(right);
            }
            assert!(reach >= 30_000.0 - HOLE_WIDTH.1 .1);
        }
    }

    #[test]
    fn anchors_stay_in_upper_half() {
        let level = generate(3, 30_000.0);
        assert!(level.blocks.iter().all(|b| b.bottom() <= ANCHOR_LOWEST));
    }

    #[test]
    fn has_holes_steps_and_floating_blocks() {
        let level = generate(4, 30_000.0);
        let ceilings: Vec<&Rect> = level.blocks.iter().filter(|b| b.y == CEILING_TOP).collect();
        let floating = level.blocks.len() - ceilings.len();
        assert!(floating > 10);
        let mut bottoms: Vec<i32> = ceilings.iter().map(|b| b.bottom() as i32).collect();
        bottoms.sort();
        bottoms.dedup();
        assert!(bottoms.len() > 5);
        let mut xs: Vec<(f32, f32)> = ceilings.iter().map(|b| (b.x, b.right())).collect();
        xs.sort_by(|a, b| a.0.total_cmp(&b.0));
        assert!(xs.windows(2).any(|w| w[1].0 - w[0].1 > 1.0));
    }

    #[test]
    fn same_seed_gives_same_terrain() {
        let a = generate(7, 10_000.0);
        let b = generate(7, 10_000.0);
        assert_eq!(a.blocks, b.blocks);
        let c = generate(8, 10_000.0);
        assert_ne!(a.blocks, c.blocks);
    }

    /// `from` から `to` までの、天井のない区間の幅の合計の割合。
    fn hole_ratio(level: &Level, from: f32, to: f32) -> f32 {
        let mut ceilings: Vec<(f32, f32)> = level
            .blocks
            .iter()
            .filter(|b| b.y == CEILING_TOP)
            .map(|b| (b.x.max(from), b.right().min(to)))
            .filter(|(left, right)| right > left)
            .collect();
        ceilings.sort_by(|a, b| a.0.total_cmp(&b.0));
        let covered: f32 = ceilings.iter().map(|(left, right)| right - left).sum();
        1.0 - covered / (to - from)
    }

    #[test]
    fn gets_harder_further_on() {
        let end = FLAT + DIFFICULTY_RAMP + 20_000.0;
        let (mut early, mut late) = (0.0, 0.0);
        for seed in 0..20 {
            let level = generate(seed, end);
            early += hole_ratio(&level, FLAT, FLAT + 8_000.0);
            late += hole_ratio(&level, end - 12_000.0, end - 4_000.0);
        }
        assert!(late > early * 1.3, "穴の割合 始め {early} 後半 {late}");
    }

    #[test]
    fn difficulty_ramps_after_the_flat_part() {
        assert_eq!(difficulty(0.0, FLAT), 0.0);
        assert_eq!(difficulty(FLAT, FLAT), 0.0);
        assert!((difficulty(FLAT + DIFFICULTY_RAMP / 2.0, FLAT) - 0.5).abs() < 1e-6);
        assert_eq!(difficulty(FLAT + DIFFICULTY_RAMP * 3.0, FLAT), 1.0);
    }

    #[test]
    fn starts_with_flat_ceiling() {
        let level = generate(5, FLAT);
        assert!(level
            .blocks
            .iter()
            .all(|b| b.bottom() == START_CEILING_BOTTOM));
    }
}
