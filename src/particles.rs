//! 倒した敵と、刺さったワイヤーから飛ぶ破片。
//! 見た目だけで、当たり判定には関わらない。
//! 粒は作った `Vec` の空き枠を使い回す。遊び始めてからの確保は、枠が埋まるまでだけ。
//! 動きは描画フレームの経過時間ではなく、`Game::step` の固定刻みで進める。
//! そのためリフレッシュレートで速さが変わらず、後から入れるヒットストップで更新を止めると破片も一緒に止まる。

use macroquad::prelude::*;
use macroquad::rand::RandGenerator;

use crate::enemy::ENEMY_COLOR;

/// 同時に存在できる粒の数。これを超えたら、残り時間が一番短い粒を上書きする。
const MAX_PARTICLES: usize = 96;
/// 敵を倒したときに飛ばす破片の数。
pub(crate) const FRAGMENT_COUNT: usize = 12;
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

/// 1 回の更新で起きた、見た目のきっかけ。
/// 判定には使わない。画面揺れやヒットストップも、同じ値を見て始める。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Impact {
    /// 敵を倒した場所。
    EnemyKilled { at: Vec2 },
    /// ワイヤーが地形に刺さった点と、地形の外（自機の側）へ向く向き。
    WireStuck { at: Vec2, normal: Vec2 },
}

/// 破片の種類。色と消え方はここから決まる。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticleKind {
    /// 倒した敵の色の破片。
    Fragment,
    /// 刺さった点の火花。
    Spark,
}

struct Style {
    count: usize,
    speed: (f32, f32),
    life: (f32, f32),
    size: (f32, f32),
}

fn style_of(kind: ParticleKind) -> Style {
    match kind {
        ParticleKind::Fragment => Style {
            count: FRAGMENT_COUNT,
            speed: (280.0, 700.0),
            life: (0.36, 0.52),
            size: (14.0, 22.0),
        },
        ParticleKind::Spark => Style {
            count: SPARK_COUNT,
            speed: (240.0, 640.0),
            life: (0.08, 0.16),
            size: (7.0, 11.0),
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
}

impl Particle {
    pub fn color(&self) -> Color {
        let fade = (self.life / self.max_life).clamp(0.0, 1.0);
        let mut color = match self.kind {
            ParticleKind::Fragment => ENEMY_COLOR,
            ParticleKind::Spark => SPARK_COLOR,
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

    /// `impact` の場所から破片を出す。死んでいる枠があればそこを上書きする。
    pub fn spawn(&mut self, impact: Impact) {
        match impact {
            Impact::EnemyKilled { at } => self.burst(at, ParticleKind::Fragment, Vec2::ZERO),
            Impact::WireStuck { at, normal } => self.burst(at, ParticleKind::Spark, normal),
        }
    }

    fn burst(&mut self, at: Vec2, kind: ParticleKind, normal: Vec2) {
        let style = style_of(kind);
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
                size: self.rng.gen_range(style.size.0, style.size.1),
                angle: self.rng.gen_range(0.0, std::f32::consts::TAU),
                spin: self.rng.gen_range(-10.0, 10.0),
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
            particle.velocity.y += GRAVITY * dt;
            particle.position += particle.velocity * dt;
            particle.angle += particle.spin * dt;
        }
    }

    #[cfg(test)]
    pub(crate) fn alive_count(&self) -> usize {
        self.alive().count()
    }

    pub fn alive(&self) -> impl Iterator<Item = &Particle> {
        self.slots.iter().filter(|p| p.life > 0.0)
    }

    pub fn draw(&self) {
        for particle in self.alive() {
            let color = particle.color();
            match particle.kind {
                ParticleKind::Fragment => {
                    draw_poly(
                        particle.position.x,
                        particle.position.y,
                        3,
                        particle.size,
                        particle.angle,
                        color,
                    );
                }
                ParticleKind::Spark => {
                    let dir = particle.velocity.normalize_or_zero();
                    let end = particle.position + dir * (particle.size * 1.8);
                    draw_line(
                        particle.position.x,
                        particle.position.y,
                        end.x,
                        end.y,
                        2.0,
                        color,
                    );
                    draw_circle(
                        particle.position.x,
                        particle.position.y,
                        particle.size * 0.28,
                        color,
                    );
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
        particles.spawn(Impact::EnemyKilled { at });
        assert_eq!(particles.alive_count(), FRAGMENT_COUNT);
        assert!(particles
            .alive()
            .all(|p| p.kind == ParticleKind::Fragment && p.position == at));
        let first = particles
            .alive()
            .next()
            .unwrap()
            .velocity
            .normalize_or_zero();
        assert!(particles
            .alive()
            .any(|p| p.velocity.normalize_or_zero().dot(first) < 0.5));

        let mean_vy = |particles: &Particles| {
            let mut sum = 0.0;
            let mut count = 0.0;
            for particle in particles.alive() {
                sum += particle.velocity.y;
                count += 1.0;
            }
            sum / count
        };
        let before = mean_vy(&particles);
        particles.update(DT);
        // どの向きへ飛んでも、同じ重力が下へ足される。
        assert!((mean_vy(&particles) - before - super::GRAVITY * DT).abs() < 1e-2);

        for _ in 0..80 {
            particles.update(DT);
        }
        assert_eq!(particles.alive_count(), 0);
    }

    #[test]
    fn sparks_leave_along_the_normal_and_fade_before_fragments() {
        let mut sparks = Particles::new(2);
        let normal = vec2(0.0, 1.0);
        sparks.spawn(Impact::WireStuck {
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
        fragments.spawn(Impact::EnemyKilled { at: Vec2::ZERO });
        let fragment_color = fragments.alive().next().unwrap().color();
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
        particles.spawn(Impact::EnemyKilled { at: Vec2::ZERO });
        let slots = particles.slot_count();
        assert!(slots <= MAX_PARTICLES);
        assert_eq!(particles.capacity(), capacity);

        for _ in 0..30 {
            for _ in 0..80 {
                particles.update(DT);
            }
            assert_eq!(particles.alive_count(), 0);
            particles.spawn(Impact::EnemyKilled {
                at: vec2(10.0, 0.0),
            });
            assert_eq!(particles.slot_count(), slots);
            assert_eq!(particles.capacity(), capacity);
        }

        // 死んだ枠が足りないときは上限まで増やす。超えた分は空きを上書きし、容量は増えない。
        for _ in 0..20 {
            particles.spawn(Impact::EnemyKilled { at: Vec2::ZERO });
        }
        assert_eq!(particles.slot_count(), MAX_PARTICLES);
        assert_eq!(particles.capacity(), capacity);
        assert!(particles.alive_count() <= MAX_PARTICLES);
    }
}
