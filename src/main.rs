mod daily;
mod enemy;
mod game;
mod generator;
mod hitstop;
mod level;
mod menu;
mod page;
mod particles;
mod pause;
mod player;
mod realtime;
mod score_popup;
mod sfx;
mod shadow;
mod shake;
mod text;

use macroquad::prelude::*;

use game::{Game, VIEW_HEIGHT};
use hitstop::HitStop;
use menu::{Action, Menu};
use pause::Pause;
use player::WireInput;
use roommate_sdk::DeleteStatus;
use shadow::{Shadows, State};
use shake::Shake;
use text::Text;

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
    // 音は始まる前に全部読み込む。
    let sfx = sfx::Sfx::load().await;
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
    // 遊んでいない間に出している画面。開いたときはタイトルから始め、遊んでいる間は None。
    let mut menu = Some(Menu::Title);
    let mut pause = Pause::default();

    loop {
        let frame_time = get_frame_time().min(MAX_FRAME_TIME);
        // このフレームの出来事で揺らし始めた分は、減らす前に描けるよう先に進める。
        shake.update(frame_time);
        let camera = view_camera(game.camera_x);
        let aim = camera.screen_to_world(mouse_position().into());
        let left_click = is_mouse_button_pressed(MouseButton::Left);
        let right_click = is_mouse_button_pressed(MouseButton::Right);
        wire_pressed |= left_click;
        boost_pressed |= right_click;
        let clicked = left_click;
        let screen = vec2(screen_width(), screen_height());
        let mouse = Vec2::from(mouse_position());
        let measure = |label: &str, size: f32| text.width(label, size);
        let delete_status = roommate_sdk::delete_status();
        // 離れたかどうかは毎フレーム読み、覚えていた分を消す。ゲームオーバーの間に離れた分を、次の回へ持ち越さない。
        // タイトルなどの画面とゲームオーバーの間は止めない。
        match pause.update(
            game.over || menu.is_some(),
            is_key_pressed(KeyCode::Escape),
            left_click || right_click,
            page::lost_focus(),
        ) {
            pause::Frame::Running => {}
            pause::Frame::Paused => {
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
        if let Some(current) = menu {
            // 画面のボタンを押したクリックでゲームが始まったり、ワイヤーを撃ったりしないよう、
            // 遊んでいない間のクリックはここで使い切る。
            if clicked {
                let buttons = menu::buttons(current, delete_status, screen, &measure);
                match menu::click(current, mouse, &buttons) {
                    Action::None => {}
                    Action::Start => {
                        // タイトルを出している間に日付が変わっていても、今日のコースで始める。
                        game = Game::new();
                        realtime::join(&game::room_key(game.seed()));
                        accumulator = 0.0;
                        until_send = 0.0;
                        hit_stop = HitStop::default();
                        menu = None;
                    }
                    Action::OpenConfirm => menu = Some(Menu::ConfirmDelete),
                    Action::Delete => {
                        roommate_sdk::request_delete();
                        menu = Some(Menu::DeleteResult);
                    }
                    Action::BackToTitle => menu = Some(Menu::Title),
                }
            }
            wire_pressed = false;
            boost_pressed = false;
        } else if game.over {
            since_over += frame_time;
            // 押した瞬間だけを見る。待ち時間のあいだに押して離した入力は、時間が過ぎてもやり直さない。
            if retry_requested(
                since_over,
                clicked,
                is_key_pressed(KeyCode::R),
                is_key_pressed(KeyCode::Space),
            ) {
                // 右下の「タイトルへ」をクリックしたときは、始めずにタイトルを出す。データはタイトルから消す。
                let to_title =
                    menu::corner_button(menu::TITLE_LABEL, Action::BackToTitle, screen, &measure);
                let back_to_title = clicked && to_title.contains(mouse);
                game = Game::new();
                // 日付が変わっていれば、新しいコースの部屋へ移る。
                realtime::join(&game::room_key(game.seed()));
                accumulator = 0.0;
                since_over = 0.0;
                hit_stop = HitStop::default();
                if back_to_title {
                    menu = Some(Menu::Title);
                }
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
                for event in &game.events {
                    sfx.play(event);
                    shake.kick(event);
                    hit_stop.kick(event);
                }
                wire_pressed = false;
                boost_pressed = false;
                accumulator -= FIXED_DT;
            }
            if game.over {
                roommate_sdk::submit_score(game.score(), (game.elapsed * 1000.0) as u32);
            }
        }

        // 自己ベストは roommate-sdk が持つ。開いた後にサーバーの値が届くと下がることもあり、
        // データを消すと無くなるので、手元で大きいほうを覚えず毎フレームそのまま読む。
        let best = best_label(roommate_sdk::best_score());
        let now = get_time();
        while let Some(message) = realtime::receive() {
            if let Some(incoming) = shadow::read_incoming(&message) {
                shadows.receive(incoming, now);
            }
        }
        shadows.forget_stale(now);
        // 終わった後、タイトルなどの画面、一時停止の間は送らない。届かなくなった影は、他の人の画面からしばらくして消える。
        if !game.over && menu.is_none() && !pause.paused() {
            until_send -= frame_time;
            if until_send <= 0.0 {
                until_send = shadow::SEND_INTERVAL;
                let state = State::of(&game.player, (now * 1000.0) as u32);
                realtime::send(&state.encode());
            }
        }

        clear_background(Color::from_rgba(16, 18, 24, 255));

        // 揺れは描画用のカメラだけに足す。照準は揺れていない `camera` で変換する。
        let mut shaken = view_camera(game.camera_x);
        shaken.target += shake.offset();
        set_camera(&shaken);
        // 影は判定に関わらないので、地形や自機より奥に描く。
        shadows.draw(now);
        game.draw(&text);

        set_default_camera();
        text.draw(
            &format!(
                "スコア {}   {:.0} m   撃破 {}",
                game.score(),
                game.distance(),
                game.kills
            ),
            16.0,
            32.0,
            28.0,
            Color::from_rgba(180, 190, 205, 255),
        );
        text.draw(
            "左クリック: ワイヤーを撃つ・戻す   右クリック: ブースト   Esc: 一時停止",
            16.0,
            screen_height() - 16.0,
            22.0,
            Color::from_rgba(150, 160, 175, 200),
        );
        text.draw(
            &best,
            16.0,
            60.0,
            22.0,
            Color::from_rgba(150, 160, 175, 255),
        );
        // ブーストの待ち時間。使えるときは明るく、待っている間は残り時間を出す。
        let cooldown = game.player.boost_cooldown;
        let (label, color) = if cooldown > 0.0 {
            (
                format!("ブースト {cooldown:.1}"),
                Color::from_rgba(140, 150, 165, 255),
            )
        } else {
            (
                "ブースト OK".to_owned(),
                Color::from_rgba(140, 220, 160, 255),
            )
        };
        text.draw(&label, 16.0, 116.0, 22.0, color);
        if game.chain > 0 {
            let multiplier = game.next_multiplier();
            let max = if multiplier == game::MAX_MULTIPLIER {
                " 最大"
            } else {
                ""
            };
            text.draw(
                &format!("連鎖 x{multiplier}{max}"),
                16.0,
                88.0,
                28.0,
                Color::from_rgba(255, 200, 80, 255),
            );
        }
        // 赤い領域が左の外へ出ているあいだは、左端に残り距離を出して迫り具合が分かるようにする。
        if !game.over {
            if let Some(meters) = game.offscreen_hazard_distance() {
                draw_offscreen_hazard(&text, meters);
            }
        }
        if pause.paused() {
            draw_paused(&text);
        }
        if game.over {
            draw_game_over(&text, &game, since_over, &best, delete_status);
            if since_over >= RETRY_DELAY {
                let to_title =
                    menu::corner_button(menu::TITLE_LABEL, Action::BackToTitle, screen, &measure);
                menu::draw_corner_button(&text, &to_title, mouse);
            }
        }
        if let Some(current) = menu {
            let buttons = menu::buttons(current, delete_status, screen, &measure);
            menu::draw(&text, current, delete_status, &buttons, mouse);
        }

        next_frame().await
    }
}

/// 画面に映るワールドの範囲を映すカメラ。縦は固定で、横は縦横比で決める。
/// ワールドは y 下向きで、y = 0 を画面の上端に映す。
/// `Camera2D::from_display_rect` は画面に描くと上下が逆になるので使わない。
fn view_camera(camera_x: f32) -> Camera2D {
    let aspect = screen_width() / screen_height();
    let size = vec2(VIEW_HEIGHT * aspect, VIEW_HEIGHT);
    Camera2D {
        target: vec2(camera_x, 0.0) + size / 2.0,
        zoom: vec2(2.0 / size.x, 2.0 / size.y),
        ..Default::default()
    }
}

/// 左クリック、R、スペースのどれかでやり直す。
/// 終わった直後の `RETRY_DELAY` の間は、どの入力も受け付けない。
fn retry_requested(since_over: f32, click: bool, key_r: bool, key_space: bool) -> bool {
    since_over >= RETRY_DELAY && (click || key_r || key_space)
}

/// 画面の左端に、左を向く矢印と、迫ってくる領域までの距離を出す。
fn draw_offscreen_hazard(text: &Text, meters: f32) {
    // 領域の境目の線と同じ赤。
    let color = Color::from_rgba(240, 90, 110, 255);
    let y = screen_height() * 0.5;
    draw_rectangle(0.0, y - 26.0, 4.0, 52.0, color);
    draw_triangle(
        vec2(10.0, y),
        vec2(26.0, y - 12.0),
        vec2(26.0, y + 12.0),
        color,
    );
    text.draw_y_centered(&format!("左 {meters:.0} m"), 34.0, y, 28.0, color);
}

fn draw_paused(text: &Text) {
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::from_rgba(0, 0, 0, 140),
    );
    let center = vec2(screen_width(), screen_height()) / 2.0;
    text.draw_centered("一時停止中", center.x, center.y - 10.0, 56.0, WHITE);
    text.draw_centered(
        "Esc かクリックで再開",
        center.x,
        center.y + 34.0,
        24.0,
        Color::from_rgba(180, 190, 205, 255),
    );
}

/// 自己ベストの表示。まだ無いとき（初めて遊ぶ前やデータを消した後）は「なし」と出し、0 点と見分ける。
fn best_label(best: Option<u32>) -> String {
    match best {
        Some(best) => format!("自己ベスト {best}"),
        None => "自己ベスト なし".to_owned(),
    }
}

fn draw_game_over(
    text: &Text,
    game: &Game,
    since_over: f32,
    best: &str,
    delete_status: DeleteStatus,
) {
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::from_rgba(0, 0, 0, 140),
    );
    let center = vec2(screen_width(), screen_height()) / 2.0;
    text.draw_centered("ゲームオーバー", center.x, center.y - 20.0, 56.0, WHITE);
    if let Some(reason) = game.end_reason {
        text.draw_centered(
            reason.label(),
            center.x,
            center.y + 24.0,
            28.0,
            Color::from_rgba(255, 140, 120, 255),
        );
    }
    text.draw_centered(
        &format!(
            "スコア {}（{:.0} m・撃破 {}）   {best}",
            game.score(),
            game.distance(),
            game.kills
        ),
        center.x,
        center.y + 62.0,
        28.0,
        WHITE,
    );
    if since_over >= RETRY_DELAY {
        text.draw_centered(
            "クリックか R かスペースでもう一度",
            center.x,
            center.y + 102.0,
            24.0,
            Color::from_rgba(180, 190, 205, 255),
        );
    }
    // 日替わりのコース。ゲームオーバーの画面を出している間も、毎フレーム残り時間を進める。
    text.draw_centered_steady(
        &daily::course_line(game.seed(), macroquad::miniquad::date::now()),
        center.x,
        center.y + 142.0,
        22.0,
        Color::from_rgba(150, 200, 235, 255),
    );
    if delete_status == DeleteStatus::Done {
        text.draw_centered(
            menu::NOT_RECORDED,
            center.x,
            center.y + 172.0,
            22.0,
            menu::NOTICE,
        );
    }
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
    fn retry_accepts_click_r_or_space_after_the_delay() {
        assert!(retry_requested(RETRY_DELAY, true, false, false));
        assert!(retry_requested(RETRY_DELAY, false, true, false));
        assert!(retry_requested(RETRY_DELAY, false, false, true));
        assert!(!retry_requested(RETRY_DELAY + 1.0, false, false, false));
    }
}
