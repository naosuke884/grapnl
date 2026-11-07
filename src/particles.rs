//! 倒した敵と、刺さったワイヤーから飛ぶ破片。
//! 見た目だけで、当たり判定には関わらない。
//! 粒は作った `Vec` の空き枠を使い回す。遊び始めてからの確保は、枠が埋まるまでだけ。
//! 動きは描画フレームの経過時間ではなく、`Game::step` の固定刻みで進める。
//! そのためリフレッシュレートで速さが変わらず、後から入れるヒットストップで更新を止めると破片も一緒に止まる。

use macroquad::prelude::*;
use macroquad::rand::RandGenerator;

use crate::enemy::ENEMY_COLOR;
use crate::game::Event;
use crate::sprites::{draw_dot_line, draw_dot_square, SPRITE_DOT};

/// 同時に存在できる粒の数。これを超えたら、残り時間が一番短い粒を上書きする。
const MAX_PARTICLES: usize = 192;
/// 敵を倒したときに飛ばす破片の数。
pub(crate) const FRAGMENT_COUNT: usize = 16;
/// ワイヤーが刺さったときに出す火花の数。破片より少なく、小さく、早く消える。
pub(crate) const SPARK_COUNT: usize = 6;
/// 破片にかける重力。自機より弱く、散ったあと弧を描いて落ちる。
const GRAVITY: f32 = 700.0;
/// 火花の色。ワイヤーの黄色より少し明るくする。
const SPARK_COLOR: Color = Color {
    r: 1.0,
    g: 230.0 / 255.0,
    b: 150.0 / 255.0,
    a: 1.0,
};

/// 倒した瞬間の白い閃光と、広がる輪の色。
const FLASH_COLOR: Color = Color::new(1.0, 1.0, 0.94, 1.0);
/// 倒した後にふわっと残るきらめきの色。
const TWINKLE_COLOR: Color = Color::new(1.0, 0.95, 0.6, 1.0);
/// 破片の明るい面。
const FRAGMENT_LIGHT: Color = Color::new(0.78, 0.98, 0.80, 1.0);

/// 破片の種類。色と消え方はここから決まる。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticleKind {
    /// 倒した敵の色の破片。
    Fragment,
    /// 刺さった点の火花。
    Spark,
    /// 倒した瞬間に一瞬だけ光る、ひし形の閃光。
    Flash,
    /// 倒した場所から広がる輪。
    Ring,
    /// 倒した後、ゆっくり漂って瞬くきらめき。
    Twinkle,
}

struct Style {
    count: usize,
    speed: (f32, f32),
    life: (f32, f32),
    size: (f32, f32),
    /// 重力のかかり方。閃光と輪は動かさず、きらめきはゆっくり落ちる。
    gravity: f32,
}

fn style_of(kind: ParticleKind) -> Style {
    match kind {
        ParticleKind::Fragment => Style {
            count: FRAGMENT_COUNT,
            speed: (280.0, 700.0),
            life: (0.36, 0.52),
            size: (10.0, 26.0),
            gravity: 1.0,
        },
        ParticleKind::Spark => Style {
            count: SPARK_COUNT,
            speed: (240.0, 640.0),
            life: (0.08, 0.16),
            size: (7.0, 11.0),
            gravity: 1.0,
        },
        ParticleKind::Flash => Style {
            count: 1,
            speed: (0.0, 0.0),
            life: (0.12, 0.12),
            size: (56.0, 56.0),
            gravity: 0.0,
        },
        ParticleKind::Ring => Style {
            count: 1,
            speed: (0.0, 0.0),
            life: (0.32, 0.32),
            size: (110.0, 110.0),
            gravity: 0.0,
        },
        ParticleKind::Twinkle => Style {
            count: 5,
            speed: (90.0, 260.0),
            life: (0.5, 0.9),
            size: (12.0, 12.0),
            gravity: 0.12,
        },
    }
}

/// 飛んでいる 1 粒。
pub struct Particle {
    pub position: Vec2,
    pub velocity: Vec2,
    pub kind: ParticleKind,
    life: f32,
    max_life: f32,
    /// 破片は三角形の半径、火花は線の長さの基準。
    size: f32,
    angle: f32,
    spin: f32,
    gravity: f32,
}

impl Particle {
    pub fn color(&self) -> Color {
        let fade = (self.life / self.max_life).clamp(0.0, 1.0);
        let mut color = match self.kind {
            ParticleKind::Fragment => ENEMY_COLOR,
            ParticleKind::Spark => SPARK_COLOR,
            ParticleKind::Flash | ParticleKind::Ring => FLASH_COLOR,
            ParticleKind::Twinkle => TWINKLE_COLOR,
        };
        color.a = fade;
        color
    }
}

/// 場に残っている破片。
pub struct Particles {
    slots: Vec<Particle>,
    rng: RandGenerator,
}

/// 横に `width` の長さ、高さ 1 ドットの帯を、ドットの格子に揃えて描く。
fn draw_dot_square_row(left: f32, center_y: f32, width: f32, color: Color) {
    let dot = SPRITE_DOT;
    let x = (left / dot).round() * dot;
    let y = ((center_y - dot / 2.0) / dot).round() * dot;
    draw_rectangle(x, y, width, dot, color);
}

impl Particles {
    pub fn new(seed: u64) -> Self {
        let rng = RandGenerator::new();
        rng.srand(seed);
        Self {
            // 枠は最初にまとめて取る。粒を足すたびに確保し直さない。
            slots: Vec::with_capacity(MAX_PARTICLES),
            rng,
        }
    }

    /// 敵を倒した・ワイヤーが刺さった場所から破片を出す。ほかの出来事では何もしない。
    /// 死んでいる枠があればそこを上書きする。
    pub fn spawn(&mut self, event: Event) {
        match event {
            Event::EnemyKilled { at, multiplier } => {
                // 閃光と輪で弾けた瞬間を見せ、破片を飛ばし、きらめきを残す。連鎖の倍率が上がるほど大きく、多くする。
                let scale = 1.0 + 0.25 * (multiplier as f32 - 1.0);
                self.burst_scaled(at, ParticleKind::Flash, Vec2::ZERO, scale, 0);
                self.burst_scaled(at, ParticleKind::Ring, Vec2::ZERO, scale, 0);
                self.burst(at, ParticleKind::Fragment, Vec2::ZERO);
                self.burst_scaled(
                    at,
                    ParticleKind::Twinkle,
                    Vec2::ZERO,
                    1.0,
                    multiplier as usize - 1,
                );
            }
            Event::WireAttached { at, normal } => self.burst(at, ParticleKind::Spark, normal),
            _ => {}
        }
    }

    fn burst(&mut self, at: Vec2, kind: ParticleKind, normal: Vec2) {
        self.burst_scaled(at, kind, normal, 1.0, 0);
    }

    /// `scale` 倍の大きさで、`extra` 個多く出す。
    fn burst_scaled(
        &mut self,
        at: Vec2,
        kind: ParticleKind,
        normal: Vec2,
        scale: f32,
        extra: usize,
    ) {
        let mut style = style_of(kind);
        style.count += extra;
        // 向きが無ければ全方向へ。刺さった火花は、地形の中へ埋まらないよう外側の半円だけへ飛ばす。
        let (center, arc) = if normal.length_squared() > 0.0 {
            (normal.y.atan2(normal.x), std::f32::consts::PI)
        } else {
            (0.0, std::f32::consts::TAU)
        };
        for i in 0..style.count {
            let along = (i as f32 + self.rng.gen_range(0.0, 1.0)) / style.count as f32;
            let dir = Vec2::from_angle(center + (along - 0.5) * arc);
            let speed = self.rng.gen_range(style.speed.0, style.speed.1);
            let life = self.rng.gen_range(style.life.0, style.life.1);
            self.insert(Particle {
                position: at,
                velocity: dir * speed,
                kind,
                life,
                max_life: life,
                size: self.rng.gen_range(style.size.0, style.size.1) * scale,
                angle: self.rng.gen_range(0.0, std::f32::consts::TAU),
                spin: self.rng.gen_range(-10.0, 10.0),
                gravity: style.gravity,
            });
        }
    }

    fn insert(&mut self, particle: Particle) {
        if let Some(index) = self.slots.iter().position(|p| p.life <= 0.0) {
            self.slots[index] = particle;
            return;
        }
        if self.slots.len() < MAX_PARTICLES {
            self.slots.push(particle);
            return;
        }
        let index = self
            .slots
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.life.total_cmp(&b.life))
            .map(|(index, _)| index)
            .expect("枠の上限まで粒がある");
        self.slots[index] = particle;
    }

    pub fn update(&mut self, dt: f32) {
        for particle in &mut self.slots {
            if particle.life <= 0.0 {
                continue;
            }
            particle.life = (particle.life - dt).max(0.0);
            if particle.life <= 0.0 {
                continue;
            }
            particle.velocity.y += GRAVITY * particle.gravity * dt;
            particle.position += particle.velocity * dt;
            particle.angle += particle.spin * dt;
        }
    }

    #[cfg(test)]
    pub(crate) fn count_of(&self, kind: ParticleKind) -> usize {
        self.alive().filter(|p| p.kind == kind).count()
    }

    #[cfg(test)]
    pub(crate) fn alive_count(&self) -> usize {
        self.alive().count()
    }

    pub fn alive(&self) -> impl Iterator<Item = &Particle> {
        self.slots.iter().filter(|p| p.life > 0.0)
    }

    pub fn draw(&self) {
        let dot = SPRITE_DOT;
        for particle in self.alive() {
            let color = particle.color();
            // 0 から 1 へ進む、生まれてからの割合。
            let age = 1.0 - particle.life / particle.max_life;
            match particle.kind {
                // ドット絵に合わせ、回さずにドットの格子に揃えた四角で描く。大きい破片は左上を明るくする。
                ParticleKind::Fragment => {
                    draw_dot_square(particle.position, particle.size, color);
                    if particle.size >= 3.0 * dot {
                        let light = Color {
                            a: color.a,
                            ..FRAGMENT_LIGHT
                        };
                        let corner = particle.position - Vec2::splat(particle.size / 2.0 - dot);
                        draw_dot_square(corner, dot, light);
                    }
                }
                ParticleKind::Spark => {
                    let dir = particle.velocity.normalize_or_zero();
                    let end = particle.position + dir * (particle.size * 1.8);
                    draw_dot_line(particle.position, end, color, true);
                }
                // 中心から縦横に伸びるひし形。すぐに縮んで消える。
                ParticleKind::Flash => {
                    let radius = (particle.size * (1.0 - age) / dot).round() as i32;
                    for row in -radius..=radius {
                        let half = radius - row.abs();
                        let y = particle.position.y + row as f32 * dot;
                        let width = (2 * half + 1) as f32 * dot;
                        draw_dot_square_row(particle.position.x - width / 2.0, y, width, color);
                    }
                }
                // 速く広がって細くなる輪。ドットを円周に並べて描く。
                ParticleKind::Ring => {
                    let eased = 1.0 - (1.0 - age) * (1.0 - age);
                    let radius = particle.size * (0.25 + 0.75 * eased);
                    let steps = ((std::f32::consts::TAU * radius / dot) as usize).max(8);
                    let thickness = if age < 0.5 { 2.0 } else { 1.0 };
                    for i in 0..steps {
                        let dir = Vec2::from_angle(i as f32 / steps as f32 * std::f32::consts::TAU);
                        draw_dot_square(particle.position + dir * radius, thickness * dot, color);
                    }
                }
                // 十字に光るきらめき。瞬きながら薄れる。
                ParticleKind::Twinkle => {
                    let blink = (particle.life * 30.0).sin() > -0.3;
                    if blink {
                        let p = particle.position;
                        draw_dot_square(p, dot, WHITE);
                        for dir in [Vec2::X, -Vec2::X, Vec2::Y, -Vec2::Y] {
                            draw_dot_square(p + dir * dot, dot, color);
                        }
                    }
                }
            }
        }
    }

    #[cfg(test)]
    fn slot_count(&self) -> usize {
        self.slots.len()
    }

    #[cfg(test)]
    fn capacity(&self) -> usize {
        self.slots.capacity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 120.0;

    #[test]
    fn fragments_scatter_outward_then_fall_and_disappear() {
        let mut particles = Particles::new(1);
        let at = vec2(100.0, 200.0);
        particles.spawn(Event::EnemyKilled { at, multiplier: 1 });
        assert_eq!(particles.count_of(ParticleKind::Fragment), FRAGMENT_COUNT);
        assert_eq!(particles.count_of(ParticleKind::Ring), 1);
        assert!(particles.alive().all(|p| p.position == at));
        let first = particles
            .alive()
            .find(|p| p.kind == ParticleKind::Fragment)
            .unwrap()
            .velocity
            .normalize_or_zero();
        assert!(particles
            .alive()
            .filter(|p| p.kind == ParticleKind::Fragment)
            .any(|p| p.velocity.normalize_or_zero().dot(first) < 0.5));

        let mean_vy = |particles: &Particles| {
            let mut sum = 0.0;
            let mut count = 0.0;
            for particle in particles
                .alive()
                .filter(|p| p.kind == ParticleKind::Fragment)
            {
                sum += particle.velocity.y;
                count += 1.0;
            }
            sum / count
        };
        let before = mean_vy(&particles);
        particles.update(DT);
        // どの向きへ飛んでも、同じ重力が下へ足される。
        assert!((mean_vy(&particles) - before - super::GRAVITY * DT).abs() < 1e-2);

        for _ in 0..120 {
            particles.update(DT);
        }
        assert_eq!(particles.alive_count(), 0);
    }

    #[test]
    fn sparks_leave_along_the_normal_and_fade_before_fragments() {
        let mut sparks = Particles::new(2);
        let normal = vec2(0.0, 1.0);
        sparks.spawn(Event::WireAttached {
            at: Vec2::ZERO,
            normal,
        });
        assert_eq!(sparks.alive_count(), SPARK_COUNT);
        assert!(sparks
            .alive()
            .all(|p| { p.kind == ParticleKind::Spark && p.velocity.dot(normal) >= -1e-3 }));
        let spark_color = sparks.alive().next().unwrap().color();
        assert_eq!(spark_color.r, SPARK_COLOR.r);
        assert_ne!(spark_color.g, ENEMY_COLOR.g);

        let mut fragments = Particles::new(2);
        fragments.spawn(Event::EnemyKilled {
            at: Vec2::ZERO,
            multiplier: 1,
        });
        let fragment_color = fragments
            .alive()
            .find(|p| p.kind == ParticleKind::Fragment)
            .unwrap()
            .color();
        assert_eq!(fragment_color.r, ENEMY_COLOR.r);
        assert_eq!(fragment_color.g, ENEMY_COLOR.g);
        assert_eq!(fragment_color.b, ENEMY_COLOR.b);

        // 火花の寿命より長く、破片の寿命より短い時間。
        let dt = 0.25;
        sparks.update(dt);
        fragments.update(dt);
        assert_eq!(sparks.alive_count(), 0);
        assert!(fragments.alive_count() > 0);
    }

    #[test]
    fn slots_and_capacity_are_reused_across_bursts() {
        let mut particles = Particles::new(3);
        let capacity = particles.capacity();
        particles.spawn(Event::EnemyKilled {
            at: Vec2::ZERO,
            multiplier: 1,
        });
        let slots = particles.slot_count();
        assert!(slots <= MAX_PARTICLES);
        assert_eq!(particles.capacity(), capacity);

        for _ in 0..30 {
            for _ in 0..120 {
                particles.update(DT);
            }
            assert_eq!(particles.alive_count(), 0);
            particles.spawn(Event::EnemyKilled {
                at: vec2(10.0, 0.0),
                multiplier: 1,
            });
            assert_eq!(particles.slot_count(), slots);
            assert_eq!(particles.capacity(), capacity);
        }

        // 死んだ枠が足りないときは上限まで増やす。超えた分は空きを上書きし、容量は増えない。
        for _ in 0..20 {
            particles.spawn(Event::EnemyKilled {
                at: Vec2::ZERO,
                multiplier: 1,
            });
        }
        assert_eq!(particles.slot_count(), MAX_PARTICLES);
        assert_eq!(particles.capacity(), capacity);
        assert!(particles.alive_count() <= MAX_PARTICLES);
    }
}
