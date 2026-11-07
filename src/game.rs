use macroquad::prelude::*;

use crate::enemy::Enemies;
use crate::generator::Generator;
use crate::level::Level;
use crate::particles::Particles;
use crate::player::{self, Player, WireInput, MAX_WIRE_LENGTH, PLAYER_RADIUS};
use crate::score_popup::Popups;
use crate::sprites::{draw_sprite, Sprites, SPRITE_DOT};
use crate::text::Text;
use crate::view::MAX_VIEW_WIDTH;

/// 自機がこの高さより下へ落ちたら終わり。天井の下 (y = 120 前後) からかなり下まで落ちられる。
pub const FALL_LIMIT_Y: f32 = 1800.0;
/// 画面に映すワールドの高さ。縦はカメラを動かさず、y = 0 から終わる高さの少し下までを一度に映す。
pub const VIEW_HEIGHT: f32 = FALL_LIMIT_Y + 60.0;
/// カメラは自機がカメラの左端からこの位置に来るよう左右どちらへも追いかける。
/// 16:9 の画面（横幅は約 3300、横長の画面もこの幅までしか映さない。`view::MAX_VIEW_WIDTH`）で
/// 中央より左に置き、進む先を広く見せる。
const FOLLOW_OFFSET: f32 = 1200.0;
/// 最も右まで進んだときのカメラの左端から、これだけ右まで地形を作っておく。
/// 映す横幅の上限（`view::MAX_VIEW_WIDTH`）の右端の先まで足りる幅にする。
const GENERATE_AHEAD: f32 = 6000.0;
/// 最も右まで進んだときのカメラの左端より、これだけ左の地形は捨てる。
/// 少し戻るのは自由だが、ずっと戻ると地形がなくなり、刺す先がなくて落ちる。
const KEEP_BEHIND: f32 = 3000.0;
/// 開始時の自機の位置。
const START_OFFSET: Vec2 = vec2(300.0, 450.0);
/// 開始位置より左と、開始位置から右へこの幅までは平らな天井にし、毎回同じ形で始まるようにする。
const START_FLAT_WIDTH: f32 = 1100.0;
/// 敵との当たり判定は見た目より小さくし、かすっただけでは終わらないようにする。
const PLAYER_HURT_RADIUS: f32 = PLAYER_RADIUS * 0.7;
/// 敵 1 体の基本の得点。連鎖の倍率を掛けて入る。
pub const KILL_POINTS: u32 = 100;
/// 連鎖の倍率の上限。上限がないと連鎖を続けるほど点が伸び、サーバーの「1 秒あたりのスコアの上限」（500）で
/// ありえない速さのスコアを見分けられなくなる。上限の倍率でも 1 体 500 点にとどめる。
pub const MAX_MULTIPLIER: u32 = 5;
/// 左から迫ってくる、触れたら終わる領域。開始時は自機のこれだけ左から始まる。
const HAZARD_START_GAP: f32 = 900.0;
/// 迫ってくる速さ。最初はゆっくりで、上限なしに一定の割合で速くなり続ける。
/// コースは終わらないが、いずれ自機より速くなって必ず追いつくので、実質の制限時間になる。
const HAZARD_START_SPEED: f32 = 120.0;
const HAZARD_ACCELERATION: f32 = 4.0;
/// 自機の最高速より速くなるまでの時間（秒）。これより長くは逃げ切れない。
const HAZARD_OUTRUNS_PLAYER_AT: f32 =
    (player::MAX_SPEED - HAZARD_START_SPEED) / HAZARD_ACCELERATION;
// 6 分ちょっと（370 秒）で、どう逃げても追いつかれる。
const _: () = assert!(HAZARD_OUTRUNS_PLAYER_AT > 300.0 && HAZARD_OUTRUNS_PLAYER_AT < 420.0);
/// 画面の距離表示の単位。ワールドの 10 を 1 m と数える。
const UNITS_PER_METER: f32 = 10.0;

/// 更新の中で起きたこと。音やパーティクルなど、判定に関わらない演出は `Game::events` からこれを読んで出す。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// ワイヤーを撃った。刺さったときは続けて `WireAttached` も積む。
    WireShot,
    /// ワイヤーが天井やブロックに刺さった。`at` は刺さった点、`normal` は地形の外（自機の側）へ向く向き。
    WireAttached {
        at: Vec2,
        normal: Vec2,
    },
    /// 刺さっていたワイヤーを手元へ戻した。
    WirePulledBack,
    /// ワイヤーで敵を倒した。`at` はワイヤーが敵に当たった点、`multiplier` はこの 1 体に掛かった連鎖の倍率。
    EnemyKilled {
        at: Vec2,
        multiplier: u32,
    },
    Boosted,
    GameOver,
}

/// 終わった理由。同じ更新で複数が重なったときは、`EndReason::of` の順で 1 つに決める。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndReason {
    /// 敵に触れた。
    Enemy,
    /// 左から迫ってくる領域に追いつかれた。
    Hazard,
    /// 下へ落ちた。
    Fell,
}

impl EndReason {
    /// 重なったときは「敵に触れた」「追いつかれた」「落ちた」の順で先のものにする。
    /// 敵はその場で避けられたはずの理由なので最も先にし、落ちたのは最後にする。
    fn of(hit: bool, caught: bool, fell: bool) -> Option<Self> {
        if hit {
            Some(Self::Enemy)
        } else if caught {
            Some(Self::Hazard)
        } else if fell {
            Some(Self::Fell)
        } else {
            None
        }
    }

    /// ゲームオーバーの画面に出す言葉。
    pub fn label(self) -> &'static str {
        match self {
            Self::Enemy => "Hit an enemy",
            Self::Hazard => "Caught by the red wall",
            Self::Fell => "Fell",
        }
    }
}

/// 1 回のプレイの状態。
pub struct Game {
    pub level: Level,
    generator: Generator,
    pub player: Player,
    pub enemies: Enemies,
    /// ワイヤーで倒した敵の数。
    pub kills: u32,
    /// 敵を倒して得た点の合計。
    kill_points: u32,
    /// 天井やブロックに刺さずに続けて倒した数。次に倒した敵の倍率は chain + 1 になる。
    pub chain: u32,
    /// カメラの左端のワールド x 座標。
    pub camera_x: f32,
    start_x: f32,
    best_x: f32,
    /// 直前の更新で受けたカーソルの位置。照準の表示に使う。
    aim: Vec2,
    /// 左から迫ってくる領域の右端のワールド x 座標。これより左に入ったら終わり。
    pub hazard_x: f32,
    /// 迫ってくる領域の今の速さ。
    hazard_speed: f32,
    pub over: bool,
    /// 終わった理由。終わるまでは `None`。`over` が true になるのと同時に入る。
    pub end_reason: Option<EndReason>,
    seed: u64,
    /// 始めてからの時間（秒）。スコアと一緒に送り、ありえない速さのスコアをサーバーが見分けるのに使う。
    pub elapsed: f32,
    /// この `step` で起きた出来事。次の `step` の始めに空になる（`Vec` の容量は残す）。
    /// 1 フレームに `step` が複数回走るので、音や画面揺れは `step` の直後にここを読む。
    /// 破片と、倒した場所に出る得点もここから出る。
    pub events: Vec<Event>,
    particles: Particles,
    popups: Popups,
}

impl Game {
    pub fn new() -> Self {
        // 同じ日は誰がいつ遊んでも同じコースになるよう、UTC の日付から種を作る。
        // 日本時間では毎朝 9:00 に次のコースへ切り替わる。
        Self::with_seed(today_seed(macroquad::miniquad::date::now()))
    }

    /// このコースの種。同じ日なら誰が遊んでも同じ。
    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn with_seed(seed: u64) -> Self {
        let mut level = Level::default();
        let mut generator = Generator::new(-KEEP_BEHIND, START_OFFSET.x + START_FLAT_WIDTH, seed);
        generator.fill(&mut level, GENERATE_AHEAD);
        let start = START_OFFSET;
        // 最初から真上の天井へワイヤーを刺した状態で始める。
        let mut player = Player::new(start);
        player.shoot_wire(start + vec2(0.0, -1.0), &level);
        Self {
            level,
            generator,
            player,
            enemies: Enemies::new(seed, start.x - FOLLOW_OFFSET),
            kills: 0,
            kill_points: 0,
            chain: 0,
            camera_x: start.x - FOLLOW_OFFSET,
            start_x: start.x,
            best_x: start.x,
            aim: start,
            hazard_x: start.x - HAZARD_START_GAP,
            hazard_speed: HAZARD_START_SPEED,
            over: false,
            end_reason: None,
            seed,
            elapsed: 0.0,
            events: Vec::with_capacity(8),
            particles: Particles::new(seed),
            popups: Popups::new(),
        }
    }

    /// 出来事を残し、破片や得点を出すものなら同じ場所から出す。
    fn emit(&mut self, event: Event) {
        self.events.push(event);
        self.particles.spawn(event);
        self.popups.spawn(event);
    }

    pub fn step(&mut self, mut input: WireInput, dt: f32) {
        // 空にしても容量は残る。毎フレーム新しい `Vec` は作らない。
        self.events.clear();
        if self.over {
            return;
        }
        self.elapsed += dt;
        self.camera_x = self.player.position.x - FOLLOW_OFFSET;
        let front = self.best_x - FOLLOW_OFFSET;
        self.generator.fill(&mut self.level, front + GENERATE_AHEAD);
        self.level.remove_before(front - KEEP_BEHIND);

        self.aim = input.aim;
        // 戻る途中のクリックは、戻り切ったフレームのクリックとして扱う。
        // 向きは押したときではなく、このフレームのカーソル。
        let anchored_before = self.player.anchor.is_some();
        if self.player.resolve_wire(input.pressed, dt) {
            input.pressed = true;
        }
        if anchored_before && self.player.anchor.is_none() {
            self.emit(Event::WirePulledBack);
        }
        // 撃ったワイヤーが地形より手前の敵に当たったら、その敵を倒し、引き寄せられながら上へ跳ねる。
        // 天井やブロックに刺すか、何もない所へ撃つと連鎖が切れる。ワイヤーを戻すだけなら切れない。
        // 敵の判定は、ワイヤーが手元にあって撃つフレームだけ行う。
        if input.pressed && self.player.can_shoot() {
            if let Some((index, point)) = self.enemy_target(input.aim) {
                let multiplier = self.next_multiplier();
                self.enemies.list.swap_remove(index);
                self.kills += 1;
                self.kill_points += KILL_POINTS * multiplier;
                self.chain += 1;
                self.emit(Event::EnemyKilled {
                    at: point,
                    multiplier,
                });
                self.player.pull_toward(point);
                input.pressed = false;
            } else {
                self.chain = 0;
            }
        }
        // ワイヤーとブーストの出来事は、自機の更新の前後の状態を比べて拾う。
        // 刺さっていなかった状態から刺さったときだけ火花を出す。最初から刺さっている状態では出さない。
        let was_anchored = self.player.anchor.is_some();
        let cooldown = self.player.boost_cooldown;
        if input.pressed && self.player.can_shoot() {
            self.emit(Event::WireShot);
        }
        self.player.finish_update(input, &self.level, dt);
        if let (false, Some(anchor)) = (was_anchored, self.player.anchor) {
            let normal = (self.player.position - anchor.point).normalize_or_zero();
            self.emit(Event::WireAttached {
                at: anchor.point,
                normal,
            });
        }
        if self.player.boost_cooldown > cooldown {
            self.emit(Event::Boosted);
        }
        self.best_x = self.best_x.max(self.player.position.x);
        let player = self.player.position;
        let front = self.best_x - FOLLOW_OFFSET;
        self.enemies.update(front, self.camera_x, dt);

        self.hazard_speed += HAZARD_ACCELERATION * dt;
        self.hazard_x += self.hazard_speed * dt;

        let fell = self.player.position.y - PLAYER_RADIUS > FALL_LIMIT_Y;
        let caught = self.player.position.x - PLAYER_RADIUS < self.hazard_x;
        let hit = self.enemies.any_touching(player, PLAYER_HURT_RADIUS);
        if let Some(reason) = EndReason::of(hit, caught, fell) {
            self.over = true;
            self.end_reason = Some(reason);
            self.emit(Event::GameOver);
        }
        self.particles.update(dt);
        self.popups.update(dt);
    }

    /// `aim` の方向へ撃ったとき、地形より手前で当たる敵の番号と当たる点。
    fn enemy_target(&self, aim: Vec2) -> Option<(usize, Vec2)> {
        let origin = self.player.position;
        let dir = (aim - origin).normalize_or_zero();
        let (index, t) = self.enemies.raycast(origin, dir, MAX_WIRE_LENGTH)?;
        let blocked = self
            .level
            .raycast(origin, dir, t)
            .is_some_and(|p| p.distance(origin) < t);
        (!blocked).then(|| (index, origin + dir * t))
    }

    /// 次に倒した敵に掛かる連鎖の倍率。
    pub fn next_multiplier(&self) -> u32 {
        (self.chain + 1).min(MAX_MULTIPLIER)
    }

    /// 得点。進んだ距離 1 m につき 1 点と、敵を倒した点の合計。
    pub fn score(&self) -> u32 {
        self.distance() as u32 + self.kill_points
    }

    /// 開始位置から最も右へ進んだ距離（m）。
    pub fn distance(&self) -> f32 {
        (self.best_x - self.start_x) / UNITS_PER_METER
    }

    /// ワールド座標のカメラ（`VIEW_HEIGHT` を画面の高さに映す）の下で呼ぶ。
    pub fn draw(&self, text: &Text, sprites: &Sprites) {
        // 左から迫ってくる領域。右端にぎざぎざの明るい縁を付けて、境目を分かりやすくする。
        let hazard_color = Color::from_rgba(120, 40, 50, 160);
        let edge_color = Color::from_rgba(240, 90, 110, 255);
        let hazard_left = self.hazard_x - 10_000.0;
        draw_rectangle(hazard_left, -1000.0, 10_000.0, 4000.0, hazard_color);
        let dot = SPRITE_DOT;
        let edge_x = (self.hazard_x / dot).round() * dot;
        for row in (-1000.0 / dot) as i32..(3000.0 / dot) as i32 {
            // 太さ 2 ドットの縁を、2 ドットごとに内側へ 1 ドット出っ張らせる。縁の右端が触れたら終わる境目。
            let width = if row.rem_euclid(4) < 2 { 3.0 } else { 2.0 };
            draw_rectangle(
                edge_x - width * dot,
                row as f32 * dot,
                width * dot,
                dot,
                edge_color,
            );
        }
        // 落ちたら終わる高さに、横いっぱいの帯を引いて見せる。上の縁も同じくぎざぎざにし、縁の上端が境目。
        let band_left = ((self.camera_x - 400.0) / dot).floor() * dot;
        let band_width = MAX_VIEW_WIDTH + 800.0;
        draw_rectangle(band_left, FALL_LIMIT_Y, band_width, 200.0, hazard_color);
        for column in 0..(band_width / dot) as i32 {
            let x = band_left + column as f32 * dot;
            let height = if ((x / dot) as i32).rem_euclid(4) < 2 {
                3.0
            } else {
                2.0
            };
            draw_rectangle(x, FALL_LIMIT_Y, dot, height * dot, edge_color);
        }
        // ワイヤーとフックは地形より先に描き、地形に食い込んだ爪を隠す。
        self.player.draw_wire();
        self.level.draw(sprites);
        self.enemies.draw(sprites);
        // ワイヤーを持っていないときは、今撃てば刺さる点（敵が手前なら敵）に照準を出す。
        if self.player.anchor.is_none() {
            let target = match self.enemy_target(self.aim) {
                Some((_, point)) => Some(point),
                None => self.player.wire_target(self.aim, &self.level),
            };
            if let Some(target) = target {
                draw_sprite(&sprites.aim, target, Color::from_rgba(255, 200, 80, 180));
            } else if let Some(end) = self.player.reach_end(self.aim) {
                // 届かない向きでは、ワイヤーが伸び切る所に薄い × を出し、そこまでしか届かないと分かるようにする。
                draw_sprite(
                    &sprites.out_of_reach,
                    end,
                    Color::from_rgba(150, 160, 175, 140),
                );
            }
        }
        self.player.draw(sprites);
        // 判定には使わない。自機や地形より手前に描いて、倒した・刺さったことが分かるようにする。
        self.particles.draw();
        // 入った得点は破片より手前に出し、何点入ったかを読めるようにする。
        self.popups.draw(text, VIEW_HEIGHT / screen_height());
    }
}

/// コース生成の版。地形か敵の作り方を変えたら上げる。
/// 影の部屋はこの版ごとに分かれるので、上げ忘れると影が違う地形の上を走る。
pub const COURSE_VERSION: u32 = 3;

/// UNIX 時刻（秒）から、その UTC の日の種を作る。1970-01-01 からの日数。
pub fn today_seed(unix_seconds: f64) -> u64 {
    (unix_seconds / 86_400.0).floor() as u64
}

/// 同じコースを走る人が集まる影の部屋の名前。slug × 日付 × コース生成の版。
pub fn room_key(seed: u64) -> String {
    format!("grapnl:{seed}:{COURSE_VERSION}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enemy::{Enemy, ENEMY_COLOR, ENEMY_RADIUS};
    use crate::particles::{ParticleKind, FRAGMENT_COUNT, SPARK_COUNT};

    const DT: f32 = 1.0 / 120.0;

    #[test]
    fn shadow_room_name_uses_the_slug() {
        assert_eq!(room_key(20_000), format!("grapnl:20000:{COURSE_VERSION}"));
    }

    #[test]
    fn seed_changes_at_utc_midnight() {
        // 2026-10-31 00:00 UTC（日本時間 9:00）。
        let midnight = 1_793_404_800.0;
        assert_eq!(today_seed(midnight), today_seed(midnight + 86_399.0));
        assert_eq!(today_seed(midnight) + 1, today_seed(midnight + 86_400.0));
        assert_ne!(today_seed(midnight - 1.0), today_seed(midnight));
    }

    /// 進み方が違っても、同じ種なら同じ地形になる。
    #[test]
    fn same_seed_gives_same_course_regardless_of_pace() {
        let mut steady = Game::with_seed(42);
        release_wire(&mut steady);
        for i in 1..=40 {
            move_to(&mut steady, i as f32 * 500.0, DT);
        }
        let mut jumpy = Game::with_seed(42);
        release_wire(&mut jumpy);
        for x in [3000.0, 15_000.0, 20_000.0] {
            move_to(&mut jumpy, x, 1.0 / 30.0);
        }
        // 地形を作り足すのは、最も右まで進んだ位置を覚えた次の更新から。
        move_to(&mut steady, 20_000.0, DT);
        move_to(&mut jumpy, 20_000.0, DT);
        assert_eq!(steady.level.blocks, jumpy.level.blocks);
    }

    /// 自機を `x` へ動かして 1 回更新する。終わらないよう、敵と迫る領域は取り除く。
    fn move_to(game: &mut Game, x: f32, dt: f32) {
        game.player.position = vec2(x, 300.0);
        game.player.velocity = Vec2::ZERO;
        game.enemies.list.clear();
        game.hazard_x = f32::MIN;
        game.step(WireInput::default(), dt);
        assert!(!game.over);
    }

    #[test]
    fn camera_does_not_scroll_by_itself() {
        let mut game = Game::with_seed(1);
        let before = game.camera_x;
        game.step(WireInput::default(), DT);
        assert_eq!(game.camera_x, before);
    }

    #[test]
    fn wire_kills_enemy_and_pulls_player() {
        let mut game = Game::with_seed(1);
        release_wire(&mut game);
        place_enemy(&mut game, 300.0);
        shoot(&mut game, vec2(1.0, 0.0));
        assert_eq!(game.kills, 1);
        assert!(game.enemies.list.is_empty());
        assert!(game.player.anchor.is_none());
        assert!(game.player.velocity.x > 600.0);
        // 横の敵を倒しても、上へ大きく跳ねる。
        assert!(game.player.velocity.y < -600.0);
    }

    #[test]
    fn click_while_the_wire_returns_kills_the_enemy_aimed_at_when_it_fires() {
        let mut game = Game::with_seed(1);
        start_retract(&mut game);
        place_enemy(&mut game, 80.0);
        // 敵と反対の向きでクリックしても、その場では倒さない。
        shoot(&mut game, vec2(-1.0, 0.0));
        assert_eq!(game.kills, 0);
        assert!(!game.player.can_shoot());

        let mut killed = false;
        for _ in 0..40 {
            if game.enemies.list.is_empty() {
                break;
            }
            let origin = game.player.position;
            game.enemies.list[0].position = origin + vec2(80.0, 0.0);
            let before = game.kills;
            game.step(
                WireInput {
                    aim: origin + vec2(80.0, 0.0),
                    ..Default::default()
                },
                DT,
            );
            if game.kills > before {
                killed = true;
                break;
            }
        }
        assert!(killed);
        assert_eq!(game.chain, 1);
        assert!(game.player.anchor.is_none());
        assert!(game.player.velocity.y < -600.0);
    }

    #[test]
    fn buffered_click_does_not_keep_the_aim_from_when_it_was_pressed() {
        let mut game = Game::with_seed(1);
        start_retract(&mut game);
        place_enemy(&mut game, 80.0);
        // 戻る途中は敵を狙ってクリックしても、その瞬間には当たらない。
        let enemy = game.enemies.list[0].position;
        game.step(
            WireInput {
                aim: enemy,
                pressed: true,
                ..Default::default()
            },
            DT,
        );
        assert_eq!(game.kills, 0);

        game.enemies.list[0].position = vec2(-10_000.0, 10_000.0);
        let mut attached = false;
        for _ in 0..40 {
            let aim = game.player.position + vec2(0.0, -1.0);
            game.step(
                WireInput {
                    aim,
                    ..Default::default()
                },
                DT,
            );
            if game.player.anchor.is_some() {
                attached = true;
                break;
            }
        }
        assert!(attached);
        assert_eq!(game.player.anchor.unwrap().point.y, 120.0);
        assert_eq!(game.kills, 0);
    }

    /// 刺さっているワイヤーを戻し始める。戻っている途中で返す。
    fn start_retract(game: &mut Game) {
        if game.player.anchor.is_none() {
            shoot(game, vec2(0.0, -1.0));
        }
        assert!(game.player.anchor.is_some());
        shoot(game, vec2(0.0, -1.0));
        assert!(!game.player.can_shoot());
        assert_eq!(game.kills, 0);
    }

    /// 自機の右 `dx` に浮遊する敵を 1 体置く。
    fn place_enemy(game: &mut Game, dx: f32) {
        let p = game.player.position;
        game.enemies.list.push(Enemy::new(p + vec2(dx, 0.0), 0.0));
    }

    /// 自機から `dir` の向きへワイヤーを撃つ。刺さっているときは戻すだけになる。
    fn shoot(game: &mut Game, dir: Vec2) {
        let input = WireInput {
            aim: game.player.position + dir,
            pressed: true,
            ..Default::default()
        };
        game.step(input, DT);
    }

    /// 刺さっているワイヤーを戻し、手元に戻るまで待つ。
    fn release_wire(game: &mut Game) {
        if game.player.anchor.is_some() {
            shoot(game, vec2(0.0, -1.0));
        }
        while !game.player.can_shoot() {
            game.step(WireInput::default(), DT);
        }
    }

    #[test]
    fn starts_hanging_from_the_same_flat_ceiling_every_time() {
        for seed in 0..20 {
            let game = Game::with_seed(seed);
            let anchor = game.player.anchor.unwrap();
            assert_eq!(anchor.point, vec2(START_OFFSET.x, 120.0));
            assert_eq!(game.player.position, START_OFFSET);
        }
    }

    #[test]
    fn chained_kills_multiply_points_until_wire_hits_terrain() {
        let mut game = Game::with_seed(1);
        release_wire(&mut game);
        place_enemy(&mut game, 300.0);
        shoot(&mut game, vec2(1.0, 0.0));
        place_enemy(&mut game, 300.0);
        shoot(&mut game, vec2(1.0, 0.0));
        assert_eq!(game.chain, 2);
        assert_eq!(game.kill_points, KILL_POINTS + KILL_POINTS * 2);

        // 天井に刺すと連鎖が切れ、次の敵は 1 倍に戻る。
        shoot(&mut game, vec2(0.0, -1.0));
        assert!(game.player.anchor.is_some());
        assert_eq!(game.chain, 0);
        release_wire(&mut game);
        place_enemy(&mut game, 300.0);
        shoot(&mut game, vec2(1.0, 0.0));
        assert_eq!(game.kill_points, KILL_POINTS * 4);
        assert_eq!(game.score(), game.distance() as u32 + KILL_POINTS * 4);
    }

    #[test]
    fn chain_multiplier_stops_at_the_cap() {
        let mut game = Game::with_seed(1);
        release_wire(&mut game);
        let kills = MAX_MULTIPLIER + 3;
        for _ in 0..kills {
            place_enemy(&mut game, 300.0);
            shoot(&mut game, vec2(1.0, 0.0));
        }
        assert_eq!(game.chain, kills);
        assert_eq!(game.next_multiplier(), MAX_MULTIPLIER);
        // 1 + 2 + … + 5 倍と、上限に届いた後の 3 体ぶんの 5 倍。
        let expected: u32 = (1..=MAX_MULTIPLIER).sum::<u32>() + MAX_MULTIPLIER * 3;
        assert_eq!(game.kill_points, KILL_POINTS * expected);
    }

    #[test]
    fn wire_events_follow_shoot_attach_and_pull_back() {
        let mut game = Game::with_seed(1);
        // 始めから刺さっているワイヤーは、撃った出来事にしない。
        game.step(WireInput::default(), DT);
        assert!(game.events.is_empty());
        shoot(&mut game, vec2(0.0, -1.0));
        assert_eq!(game.events, vec![Event::WirePulledBack]);
        release_wire(&mut game);
        shoot(&mut game, vec2(0.0, -1.0));
        let point = game.player.anchor.unwrap().point;
        let normal = (game.player.position - point).normalize_or_zero();
        assert_eq!(
            game.events,
            vec![Event::WireShot, Event::WireAttached { at: point, normal }]
        );
    }

    #[test]
    fn kill_events_carry_the_chain_multiplier() {
        let mut game = Game::with_seed(1);
        release_wire(&mut game);
        for multiplier in 1..=2 {
            place_enemy(&mut game, 300.0);
            let at = game.player.position + vec2(300.0 - ENEMY_RADIUS, 0.0);
            shoot(&mut game, vec2(1.0, 0.0));
            assert_eq!(game.events, vec![Event::EnemyKilled { at, multiplier }]);
        }
    }

    #[test]
    fn boost_and_game_over_are_events() {
        let mut game = Game::with_seed(1);
        let input = WireInput {
            boost: true,
            ..Default::default()
        };
        game.step(input, DT);
        assert_eq!(game.events, vec![Event::Boosted]);
        // 待ち時間の間は押してもブーストしない。
        game.step(input, DT);
        assert!(game.events.is_empty());
        game.hazard_x = game.player.position.x;
        game.step(WireInput::default(), DT);
        assert_eq!(game.events, vec![Event::GameOver]);
    }

    #[test]
    fn touching_enemy_ends_the_game() {
        let mut game = Game::with_seed(1);
        place_enemy(&mut game, 0.0);
        game.step(WireInput::default(), DT);
        assert!(game.over);
        assert_eq!(game.end_reason, Some(EndReason::Enemy));
    }

    #[test]
    fn hazard_creeps_in_from_the_left_and_speeds_up() {
        let mut game = Game::with_seed(1);
        let start = game.hazard_x;
        game.step(WireInput::default(), 1.0);
        let first = game.hazard_x - start;
        assert!(first >= HAZARD_START_SPEED);
        let before = game.hazard_x;
        game.step(WireInput::default(), 1.0);
        assert!(game.hazard_x - before > first);
    }

    #[test]
    fn touching_the_hazard_ends_the_game() {
        let mut game = Game::with_seed(1);
        game.hazard_x = game.player.position.x;
        game.step(WireInput::default(), DT);
        assert!(game.over);
        assert_eq!(game.end_reason, Some(EndReason::Hazard));
    }

    #[test]
    fn camera_follows_player_both_ways() {
        let mut game = Game::with_seed(1);
        release_wire(&mut game);
        game.player.position.x = 5000.0;
        game.step(WireInput::default(), DT);
        assert_eq!(game.camera_x, game.player.position.x - FOLLOW_OFFSET);
        game.player.position.x = 4000.0;
        game.step(WireInput::default(), DT);
        assert_eq!(game.camera_x, game.player.position.x - FOLLOW_OFFSET);
    }

    #[test]
    fn falling_off_the_bottom_ends_the_game() {
        let mut game = Game::with_seed(1);
        release_wire(&mut game);
        for _ in 0..600 {
            game.step(WireInput::default(), DT);
        }
        assert!(game.over);
        assert!(game.player.position.y > FALL_LIMIT_Y);
        assert_eq!(game.end_reason, Some(EndReason::Fell));
    }

    #[test]
    fn not_over_has_no_end_reason() {
        let mut game = Game::with_seed(1);
        game.step(WireInput::default(), DT);
        assert!(!game.over);
        assert_eq!(game.end_reason, None);
    }

    /// 同じ更新で敵に触れ、追いつかれ、落ちたときは、敵に触れたことにする。
    #[test]
    fn overlapping_reasons_pick_the_enemy_first() {
        let mut game = Game::with_seed(1);
        release_wire(&mut game);
        game.player.position = vec2(game.player.position.x, FALL_LIMIT_Y + PLAYER_RADIUS * 2.0);
        game.player.velocity = Vec2::ZERO;
        game.hazard_x = game.player.position.x + 1000.0;
        place_enemy(&mut game, 0.0);
        game.step(WireInput::default(), DT);
        assert_eq!(game.end_reason, Some(EndReason::Enemy));

        assert_eq!(EndReason::of(false, true, true), Some(EndReason::Hazard));
        assert_eq!(EndReason::of(false, false, true), Some(EndReason::Fell));
        assert_eq!(EndReason::of(false, false, false), None);
    }

    #[test]
    fn can_move_left_of_the_start() {
        let mut game = Game::with_seed(1);
        release_wire(&mut game);
        game.player.position = vec2(0.0, 300.0);
        game.player.velocity = vec2(-500.0, 0.0);
        game.step(WireInput::default(), DT);
        assert!(!game.over);
        assert!(game.player.position.x < 0.0);
    }

    /// 真上から右斜め上までの向きのうち、ワイヤーが刺さる点が自機の 100 先にいちばん近い向き。
    fn best_aim(game: &Game) -> Option<Vec2> {
        let p = game.player.position;
        (0..=12)
            .map(|i| {
                let angle = -std::f32::consts::FRAC_PI_2 + i as f32 * 0.1;
                p + Vec2::from_angle(angle)
            })
            .filter_map(|aim| game.player.wire_target(aim, &game.level).map(|t| (aim, t)))
            // 近すぎる刺し先は、天井の角に挟まって動けなくなるので選ばない。
            .filter(|(_, t)| t.distance(p) > 150.0)
            .min_by(|a, b| {
                let da = (a.1.x - p.x - 100.0).abs();
                let db = (b.1.x - p.x - 100.0).abs();
                da.total_cmp(&db)
            })
            .map(|(aim, _)| aim)
    }

    /// 単純な操作でも、落ちるまでに 300 m 以上進めることを確かめる。
    /// 刺さっていなければ少し先の刺し先へ撃ち、支点より十分右で上がっている途中にワイヤーを戻す。
    /// 右上へ上がっている途中は、待ち時間が明けるたびにブーストする。
    /// 移動だけを確かめるので、敵は毎回消す。
    /// ボットは粗く、いずれ低い所へ沈んで落ちるので、落ちずに続けることまでは求めない。
    #[test]
    fn simple_swinging_makes_progress() {
        for seed in 1..=10 {
            swing_for_a_while(seed);
        }
    }

    fn swing_for_a_while(seed: u64) {
        let mut game = Game::with_seed(seed);
        for _ in 0..(30.0 / DT) as usize {
            let p = game.player.position;
            let pressed = match game.player.anchor {
                // 刺さっていなければ、落ち始めてから撃つ。
                None => {
                    game.player.can_shoot()
                        && game.player.velocity.y > 0.0
                        && best_aim(&game).is_some()
                }
                // 支点より十分右で上がっている途中に、ワイヤーを戻して飛ぶ。
                // 真下でほとんど止まっている（開始時など）ときも、一度戻して落ちながら先へ撃ち直す。
                Some(a) => {
                    let rising_ahead = p.x > a.point.x + a.length * 0.3
                        && game.player.velocity.x > 0.0
                        && game.player.velocity.y < 0.0;
                    rising_ahead || game.player.velocity.length() < 30.0
                }
            };
            let input = WireInput {
                aim: best_aim(&game).unwrap_or(game.player.position),
                pressed,
                boost: game.player.velocity.y < 0.0 && game.player.velocity.x > 0.0,
            };
            game.enemies.list.clear();
            game.step(input, DT);
            if game.over {
                break;
            }
        }
        assert!(
            game.distance() > 300.0,
            "seed {seed}: {:.0} m",
            game.distance()
        );
    }

    #[test]
    fn terrain_is_generated_ahead_and_discarded_behind() {
        let mut game = Game::with_seed(1);
        release_wire(&mut game);
        game.player.position.x = 10_000.0;
        // 地形を作り足すのは、最も右まで進んだ位置を覚えた次の更新から。
        game.step(WireInput::default(), DT);
        game.step(WireInput::default(), DT);
        let front = game.best_x - FOLLOW_OFFSET;
        let right = game
            .level
            .blocks
            .iter()
            .map(|b| b.right())
            .fold(f32::MIN, f32::max);
        let left = game
            .level
            .blocks
            .iter()
            .map(|b| b.right())
            .fold(f32::MAX, f32::min);
        // 最後が穴で終わると刺し先の右端は少し手前になるので、穴の幅の分だけ余裕を見る。
        assert!(right >= front + GENERATE_AHEAD - 1000.0);
        assert!(left >= front - KEEP_BEHIND);
    }

    #[test]
    fn killing_an_enemy_scatters_fragments_of_its_color() {
        let mut game = Game::with_seed(1);
        release_wire(&mut game);
        let origin = game.player.position;
        place_enemy(&mut game, 300.0);
        shoot(&mut game, vec2(1.0, 0.0));
        let at = origin + vec2(300.0 - ENEMY_RADIUS, 0.0);
        assert_eq!(game.events, vec![Event::EnemyKilled { at, multiplier: 1 }]);
        let bits: Vec<_> = game
            .particles
            .alive()
            .filter(|p| p.kind == ParticleKind::Fragment)
            .collect();
        assert_eq!(bits.len(), FRAGMENT_COUNT);
        assert!(bits.iter().all(|p| p.position.distance(at) < 15.0));
        let color = bits[0].color();
        assert_eq!(
            (color.r, color.g, color.b),
            (ENEMY_COLOR.r, ENEMY_COLOR.g, ENEMY_COLOR.b)
        );
        assert!(color.a > 0.5);
        // 倒した場所に、入った点が出る。
        let popups: Vec<_> = game.popups.alive().collect();
        assert_eq!(popups.len(), 1);
        assert_eq!(popups[0].label(), "+100");
        assert!(popups[0].position.distance(at) < 15.0);

        // 次の更新ではきっかけは空になる。飛んでいる破片はそのまま残る。
        let capacity = game.events.capacity();
        game.step(WireInput::default(), DT);
        assert!(game.events.is_empty());
        assert_eq!(game.events.capacity(), capacity);
        assert!(game.particles.alive_count() > 0);
        assert_eq!(game.popups.alive().count(), 1);
    }

    #[test]
    fn sticking_the_wire_emits_sparks_at_the_anchor() {
        let mut game = Game::with_seed(1);
        release_wire(&mut game);
        game.enemies.list.clear();
        shoot(&mut game, vec2(0.0, -1.0));
        let anchor = game.player.anchor.unwrap();
        let normal = (game.player.position - anchor.point).normalize_or_zero();
        assert_eq!(
            game.events,
            vec![
                Event::WireShot,
                Event::WireAttached {
                    at: anchor.point,
                    normal,
                }
            ]
        );
        let sparks: Vec<_> = game.particles.alive().collect();
        assert_eq!(sparks.len(), SPARK_COUNT);
        assert!(sparks.iter().all(|p| p.kind == ParticleKind::Spark));
        assert!(sparks.iter().all(|p| p.velocity.dot(normal) >= -1e-3));
        assert!(sparks
            .iter()
            .all(|p| p.position.distance(anchor.point) < 20.0));
    }

    #[test]
    fn pulling_the_wire_back_or_missing_does_not_spark() {
        let mut game = Game::with_seed(1);
        shoot(&mut game, vec2(0.0, -1.0));
        assert!(game.player.anchor.is_none());
        assert_eq!(game.events, vec![Event::WirePulledBack]);
        assert_eq!(game.particles.alive_count(), 0);

        release_wire(&mut game);
        game.enemies.list.clear();
        shoot(&mut game, vec2(0.0, 1.0));
        assert!(game.player.anchor.is_none());
        assert_eq!(game.events, vec![Event::WireShot]);
        assert_eq!(game.particles.alive_count(), 0);
    }

    #[test]
    fn fragments_do_not_count_as_a_hit_or_block_the_wire() {
        let mut game = Game::with_seed(1);
        release_wire(&mut game);
        place_enemy(&mut game, 300.0);
        shoot(&mut game, vec2(1.0, 0.0));
        let at = match game.events[0] {
            Event::EnemyKilled { at, .. } => at,
            _ => panic!("倒したのに別の出来事が先に来た"),
        };
        game.player.position = at;
        game.player.velocity = Vec2::ZERO;
        game.enemies.list.clear();
        game.hazard_x = f32::MIN;
        game.step(WireInput::default(), DT);
        assert!(!game.over);
        assert!(game.particles.alive_count() > 0);

        // 破片の中から天井へ撃っても、刺さる先は地形のまま。
        shoot(&mut game, vec2(0.0, -1.0));
        let anchor = game.player.anchor.expect("天井に刺さる");
        assert!(game.events.iter().any(|impact| matches!(
            impact,
            Event::WireAttached { at, .. } if *at == anchor.point
        )));
        assert!(game
            .particles
            .alive()
            .any(|p| p.kind == ParticleKind::Spark));
        assert!(!game.over);
    }
}
