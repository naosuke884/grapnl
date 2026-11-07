mod bgm;
mod daily;
mod enemy;
mod game;
mod generator;
mod hitstop;
mod level;
mod page;
mod particles;
mod pause;
mod player;
mod ranking;
mod realtime;
mod score_popup;
mod sfx;
mod shadow;
mod shake;
mod sprites;
mod text;
mod tutorial;
mod view;
mod volume;

use macroquad::prelude::*;

use bgm::Bgm;
use game::Game;
use hitstop::HitStop;
use pause::Pause;
use player::WireInput;
use ranking::Board;
use shadow::{Shadows, State};
use shake::Shake;
use text::Text;
use tutorial::Tutorial;
use view::Playfield;
use volume::Volume;

/// 更新は固定の刻みで回し、画面のリフレッシュレートで手触りが変わらないようにする。
const FIXED_DT: f32 = 1.0 / 120.0;
/// 処理落ちしたときに 1 フレームで追いつく上限。これを超えた分は捨ててゲームを遅らせる。
const MAX_FRAME_TIME: f32 = 0.1;
/// ゲームオーバー直後の操作でうっかりやり直さないよう、受け付けない時間。
const RETRY_DELAY: f32 = 0.6;

fn window_conf() -> Conf {
    Conf {
        // ブラウザのタブ（web/index.html の <title>）と同じ正式名。先頭も小文字。
        window_title: "grapnl".to_owned(),
        window_width: 1280,
        window_height: 720,
        // ネイティブウィンドウのドラッグリサイズを受け付ける。
        // 毎フレーム screen_width / screen_height でカメラを作り直すので、WASM 側は
        // キャンバスの CSS サイズ（ブラウザのウィンドウ）にも同じ描画で追従する。
        window_resizable: true,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let text = Text::load();
    let sprites = sprites::Sprites::load();
    // 音は始まる前に全部読み込む。
    let sfx = sfx::Sfx::load().await;
    // 曲は大きいので待たずに始め、読み終えたら流す。
    let mut bgm = Bgm::start();
    let mut game = Game::new();
    realtime::join(&game::room_key(game.seed()));
    let mut shadows = Shadows::default();
    // 次に自機の状態を影の部屋へ送るまでの時間。
    let mut until_send = 0.0;
    let mut accumulator = 0.0;
    // ゲームオーバーになってからの経過時間。
    let mut since_over = 0.0;
    // クリックは描画フレームで受け、次に回る固定刻みの更新で 1 回だけ使う。
    let mut wire_pressed = false;
    let mut boost_pressed = false;
    let mut shake = Shake::default();
    // 敵を倒した瞬間に更新を止める。止まっている間も描画とクリックの受け付けは続ける。
    let mut hit_stop = HitStop::default();
    let mut pause = Pause::default();
    let mut volume = Volume::load();
    // 最初の数回のプレイだけ、画面下に操作の説明を吹き出しで出す。
    let mut tutorial = Tutorial::load();
    // ゲームオーバーの画面に出す今日のランキング。
    let mut board = Board::default();

    loop {
        let frame_time = get_frame_time().min(MAX_FRAME_TIME);
        // このフレームの出来事で揺らし始めた分は、減らす前に描けるよう先に進める。
        shake.update(frame_time);
        // 16:9 より横長の画面では、ワールドを中央の 16:9 の範囲に映す。
        let playfield = Playfield::current();
        let camera = playfield.camera(game.camera_x);
        let aim = camera.screen_to_world(mouse_position().into());
        let left_click = is_mouse_button_pressed(MouseButton::Left);
        let right_click = is_mouse_button_pressed(MouseButton::Right);
        wire_pressed |= left_click;
        boost_pressed |= right_click;
        // 画面の文字は、ワールドを映す範囲の左上を原点にした座標で置く。
        let screen = playfield.rect.size();
        // 一時停止の画面の音量の操作。画面の文字と同じ座標で、ポインターがどれに乗っているかを見る。
        let volume_panel = volume_panel(screen);
        let pointer = Vec2::from(mouse_position()) - playfield.rect.point();
        let hovered = if pause.paused() {
            volume_panel.control_at(pointer)
        } else {
            None
        };
        // 音量の囲みの中のクリックでは再開しない。
        let on_volume_panel = pause.paused() && volume_panel.frame().contains(pointer);
        // 離れたかどうかは毎フレーム読み、覚えていた分を消す。ゲームオーバーの間に離れた分を、次の回へ持ち越さない。
        match pause.update(
            game.over,
            is_key_pressed(KeyCode::Escape),
            (left_click || right_click) && !on_volume_panel,
            page::lost_focus(),
        ) {
            pause::Frame::Running => {}
            pause::Frame::Paused => {
                volume.update(
                    is_key_pressed(KeyCode::Left),
                    is_key_pressed(KeyCode::Right),
                    is_key_pressed(KeyCode::M),
                    hovered.filter(|_| left_click),
                );
                // 止まっている間に押した入力は、再開した後に使わない。
                wire_pressed = false;
                boost_pressed = false;
            }
            pause::Frame::Resumed => {
                // 再開に使ったクリックでワイヤーを撃たない。止まっていた時間も追いかけない。
                wire_pressed = false;
                boost_pressed = false;
                accumulator = 0.0;
            }
        }
        if game.over {
            since_over += frame_time;
            // 押した瞬間だけを見る。待ち時間のあいだに押して離した入力は、時間が過ぎてもやり直さない。
            if retry_requested(
                since_over,
                is_mouse_button_pressed(MouseButton::Left),
                is_key_pressed(KeyCode::R),
                is_key_pressed(KeyCode::Space),
            ) {
                game = Game::new();
                // 日付が変わっていれば、新しいコースの部屋へ移る。
                realtime::join(&game::room_key(game.seed()));
                accumulator = 0.0;
                since_over = 0.0;
                hit_stop = HitStop::default();
                tutorial.restart();
                board.reset();
            }
            wire_pressed = false;
            boost_pressed = false;
        } else if !pause.paused() {
            // 止まっている時間は貯めない。押されたクリックは残し、止まり終えた後の更新で使う。
            accumulator += hit_stop.consume(frame_time);
            while accumulator >= FIXED_DT && !game.over && !hit_stop.active() {
                let input = WireInput {
                    aim,
                    pressed: wire_pressed,
                    boost: boost_pressed,
                };
                game.step(input, FIXED_DT);
                tutorial.observe(&game.events, FIXED_DT);
                for event in &game.events {
                    sfx.play(event, volume.gain());
                    shake.kick(event);
                    hit_stop.kick(event);
                }
                wire_pressed = false;
                boost_pressed = false;
                accumulator -= FIXED_DT;
            }
            if game.over {
                tutorial.finish_run();
                roommate_sdk::submit_score(game.score(), (game.elapsed * 1000.0) as u32);
                board.request();
            }
        }

        // 自己ベストは roommate-sdk が持つ。開いた後にサーバーの値が届くと下がることもあり、
        // データを消すと無くなるので、手元で大きいほうを覚えず毎フレームそのまま読む。
        let best = best_label(roommate_sdk::best_score());
        board.update(&text);
        bgm.update(volume.gain());
        let now = get_time();
        while let Some(message) = realtime::receive() {
            if let Some(incoming) = shadow::read_incoming(&message) {
                shadows.receive(incoming, now);
            }
        }
        shadows.forget_stale(now);
        // 終わった後と一時停止の間は送らない。届かなくなった影は、他の人の画面からしばらくして消える。
        if !game.over && !pause.paused() {
            until_send -= frame_time;
            if until_send <= 0.0 {
                until_send = shadow::SEND_INTERVAL;
                let state = State::of(&game.player, (now * 1000.0) as u32);
                realtime::send(&state.encode());
            }
        }

        clear_background(Color::from_rgba(16, 18, 24, 255));

        // 揺れは描画用のカメラだけに足す。照準は揺れていない `camera` で変換する。
        let mut shaken = playfield.camera(game.camera_x);
        shaken.target += shake.offset();
        set_camera(&shaken);
        // 影は判定に関わらないので、地形や自機より奥に描く。
        shadows.draw(now, &sprites);
        game.draw(&text, &sprites);

        set_default_camera();
        // ワールドを映す範囲の外は黒く塗り、横長の画面でも先が見える幅を変えない。
        playfield.draw_bars();
        set_camera(&playfield.hud_camera());
        text.draw(
            &format!(
                "Score {}   {:.0} m   Kills {}",
                game.score(),
                game.distance(),
                game.kills
            ),
            16.0,
            32.0,
            text::MEDIUM,
            Color::from_rgba(180, 190, 205, 255),
        );
        text.draw(
            &best,
            16.0,
            60.0,
            text::SMALL,
            Color::from_rgba(150, 160, 175, 255),
        );
        // ブーストの待ち時間。使えるときは明るく、待っている間は残り時間を出す。
        let cooldown = game.player.boost_cooldown;
        let (label, color) = if cooldown > 0.0 {
            (
                format!("Boost {cooldown:.1}"),
                Color::from_rgba(140, 150, 165, 255),
            )
        } else {
            ("Boost OK".to_owned(), Color::from_rgba(140, 220, 160, 255))
        };
        text.draw(&label, 16.0, 116.0, text::SMALL, color);
        if game.chain > 0 {
            let multiplier = game.next_multiplier();
            let max = if multiplier == game::MAX_MULTIPLIER {
                " MAX"
            } else {
                ""
            };
            text.draw(
                &format!("Chain x{multiplier}{max}"),
                16.0,
                88.0,
                text::MEDIUM,
                Color::from_rgba(255, 200, 80, 255),
            );
        }
        if !game.over && !pause.paused() {
            tutorial.draw(&text, screen, game.player.anchor.is_some());
        }
        if pause.paused() {
            draw_paused(&text, screen);
            // 操作の説明は遊んでいる間は出さず、一時停止の画面の下にだけ出す。
            text.draw(
                "Left click: shoot / retract wire   Right click: boost   Esc: pause",
                16.0,
                screen.y - 16.0,
                text::SMALL,
                Color::from_rgba(180, 190, 205, 255),
            );
            volume_panel.draw(&text, &sprites, &volume, hovered);
        }
        if game.over {
            draw_game_over(&text, &game, &board, since_over, &best, screen);
        }

        next_frame().await
    }
}

/// 左クリック、R、スペースのどれかでやり直す。
/// 終わった直後の `RETRY_DELAY` の間は、どの入力も受け付けない。
fn retry_requested(since_over: f32, click: bool, key_r: bool, key_space: bool) -> bool {
    since_over >= RETRY_DELAY && (click || key_r || key_space)
}

fn draw_paused(text: &Text, screen: Vec2) {
    draw_rectangle(0.0, 0.0, screen.x, screen.y, Color::from_rgba(0, 0, 0, 140));
    let center = screen / 2.0;
    text.draw_centered("PAUSED", center.x, center.y - 10.0, text::LARGE, WHITE);
    text.draw_centered(
        "Esc or click to resume",
        center.x,
        center.y + 34.0,
        text::SMALL,
        Color::from_rgba(180, 190, 205, 255),
    );
}

/// 一時停止の画面で、音量の操作を「Esc かクリックで再開」の下に置く。
fn volume_panel(screen: Vec2) -> volume::Panel {
    volume::Panel::new(screen.x / 2.0, screen.y / 2.0 + 84.0)
}

/// 自己ベストの表示。まだ無いとき（初めて遊ぶ前やデータを消した後）は「なし」と出し、0 点と見分ける。
fn best_label(best: Option<u32>) -> String {
    match best {
        Some(best) => format!("Best {best}"),
        None => "Best -".to_owned(),
    }
}

/// ゲームオーバーの画面の、上から並べる行の間隔。それぞれ前の行のベースラインからの距離。
const OVER_TITLE: f32 = text::LARGE;
const OVER_REASON: f32 = 48.0;
const OVER_SCORE: f32 = 44.0;
/// スコアの行のベースラインから、ランキングの見出しの上端まで。
const OVER_RANKING_GAP: f32 = 20.0;
/// ランキングの表の下端から、やり直しの案内のベースラインまで。
const OVER_RETRY: f32 = 30.0;
const OVER_COURSE: f32 = 36.0;
/// 画面の上下に残す余白。
const OVER_MARGIN: f32 = 16.0;

/// ゲームオーバーの画面の、ランキングを除いた高さ。
fn game_over_fixed_height() -> f32 {
    OVER_TITLE + OVER_REASON + OVER_SCORE + OVER_RANKING_GAP + OVER_RETRY + OVER_COURSE
}

/// 画面の高さに収まる、ランキングの上位の行数。
fn ranking_rows(screen_h: f32) -> usize {
    let room = screen_h - OVER_MARGIN * 2.0 - game_over_fixed_height() - Board::height(0);
    ((room / ranking::ROW_HEIGHT).floor().max(0.0) as usize).min(ranking::MAX_ROWS)
}

fn draw_game_over(
    text: &Text,
    game: &Game,
    board: &Board,
    since_over: f32,
    best: &str,
    screen: Vec2,
) {
    draw_rectangle(0.0, 0.0, screen.x, screen.y, Color::from_rgba(0, 0, 0, 140));
    let center_x = screen.x / 2.0;
    // 行が収まる分だけ出し、全体を縦の中央に置く。
    let rows = ranking_rows(screen.y);
    let height = game_over_fixed_height() + Board::height(rows);
    let mut y = ((screen.y - height) / 2.0).max(OVER_MARGIN) + OVER_TITLE;
    text.draw_centered("GAME OVER", center_x, y, text::LARGE, WHITE);
    y += OVER_REASON;
    if let Some(reason) = game.end_reason {
        text.draw_centered(
            reason.label(),
            center_x,
            y,
            text::MEDIUM,
            Color::from_rgba(255, 140, 120, 255),
        );
    }
    y += OVER_SCORE;
    text.draw_centered(
        &format!(
            "Score {} ({:.0} m, {} kills)   {best}",
            game.score(),
            game.distance(),
            game.kills
        ),
        center_x,
        y,
        text::MEDIUM,
        WHITE,
    );
    y += OVER_RANKING_GAP;
    board.draw(text, center_x, y, rows);
    y += Board::height(rows) + OVER_RETRY;
    if since_over >= RETRY_DELAY {
        text.draw_centered(
            "Click, R or Space to retry",
            center_x,
            y,
            text::SMALL,
            Color::from_rgba(180, 190, 205, 255),
        );
    }
    y += OVER_COURSE;
    // 日替わりのコース。ゲームオーバーの画面を出している間も、毎フレーム残り時間を進める。
    text.draw_centered_steady(
        &daily::course_line(game.seed(), macroquad::miniquad::date::now()),
        center_x,
        y,
        text::SMALL,
        Color::from_rgba(150, 200, 235, 255),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_ignores_input_during_the_delay() {
        let just_before = RETRY_DELAY - 0.01;
        assert!(!retry_requested(0.0, true, true, true));
        assert!(!retry_requested(just_before, true, false, false));
        assert!(!retry_requested(just_before, false, true, false));
        assert!(!retry_requested(just_before, false, false, true));
    }

    #[test]
    fn game_over_fits_the_screen() {
        assert_eq!(ranking_rows(720.0), ranking::MAX_ROWS);
        let rows = ranking_rows(400.0);
        assert!(rows < ranking::MAX_ROWS);
        assert!(game_over_fixed_height() + Board::height(rows) + OVER_MARGIN * 2.0 <= 400.0);
        assert_eq!(ranking_rows(100.0), 0);
    }

    #[test]
    fn retry_accepts_click_r_or_space_after_the_delay() {
        assert!(retry_requested(RETRY_DELAY, true, false, false));
        assert!(retry_requested(RETRY_DELAY, false, true, false));
        assert!(retry_requested(RETRY_DELAY, false, false, true));
        assert!(!retry_requested(RETRY_DELAY + 1.0, false, false, false));
    }
}
