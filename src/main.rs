mod background;
mod bgm;
mod enemy;
mod game;
mod generator;
mod hitstop;
mod level;
mod names;
mod overlay;
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
use view::Playfield;
use volume::Volume;

/// 更新は固定の刻みで回し、画面のリフレッシュレートで手触りが変わらないようにする。
const FIXED_DT: f32 = 1.0 / 120.0;
/// 処理落ちしたときに 1 フレームで追いつく上限。これを超えた分は捨ててゲームを遅らせる。
const MAX_FRAME_TIME: f32 = 0.1;
/// ゲームオーバー直後の操作でうっかりやり直さないよう、受け付けない時間。
pub(crate) const RETRY_DELAY: f32 = 0.6;

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
    let fonts = text::Fonts::load();
    let text = &fonts.body;
    let heading = &fonts.heading;
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
    // このゲームオーバーで自己ベストを更新したか。
    let mut new_best = false;
    // クリックは描画フレームで受け、次に回る固定刻みの更新で 1 回だけ使う。
    let mut wire_pressed = false;
    let mut shake = Shake::default();
    // 敵を倒した瞬間に更新を止める。止まっている間も描画とクリックの受け付けは続ける。
    let mut hit_stop = HitStop::default();
    let mut pause = Pause::default();
    let mut volume = Volume::load();
    // ゲームオーバーの画面に出す今日のランキング。
    let mut board = Board::default();
    // ランキングと影の名前を描く。描けない名前があれば、名前用のフォントを後から読む。
    let mut name_font = names::NameFont::default();
    // 最後の入力がタッチだったか。タッチではマウスの位置も動くが、同じフレームに touches() にも出る。
    // touches() が空のままマウスが動いたら、マウスに持ち替えたと見る。
    let mut touch_input = false;
    let mut last_mouse = mouse_position();

    loop {
        let frame_time = get_frame_time().min(MAX_FRAME_TIME);
        // このフレームの出来事で揺らし始めた分は、減らす前に描けるよう先に進める。
        shake.update(frame_time);
        // 16:9 より横長の画面では、ワールドを中央の 16:9 の範囲に映す。
        let playfield = Playfield::current();
        let camera = playfield.camera(playfield.origin(game.camera_x, game.player.position));
        let mouse = mouse_position();
        if !touches().is_empty() {
            touch_input = true;
        } else if mouse != last_mouse {
            touch_input = false;
        }
        last_mouse = mouse;
        let aim = camera.screen_to_world(mouse.into());
        let left_click = is_mouse_button_pressed(MouseButton::Left);
        let right_click = is_mouse_button_pressed(MouseButton::Right);
        // 画面の文字は、ワールドを映す範囲の左上を原点にした座標で置く。小さい画面では縮めて描く（`Playfield::ui_scale`）。
        let screen = playfield.ui_size();
        // スマートフォンの縦向きなど、縦長の画面。
        let portrait = playfield.rect.w < playfield.rect.h;
        let pointer = playfield.ui_point(mouse_position().into());
        // 右上の「MENU」ボタン。キーボードの無いスマートフォンでも止められるようにする。押してもワイヤーは撃たない。
        let on_menu_button =
            !game.over() && !pause.paused() && overlay::menu_button(screen).contains(pointer);
        wire_pressed |= left_click && !on_menu_button;
        // 一時停止の画面の音量の操作。画面の文字と同じ座標で、ポインターがどれに乗っているかを見る。
        let pause_ui = overlay::pause_layout(screen);
        let hovered = if pause.paused() {
            pause_ui.panel.control_at(pointer)
        } else {
            None
        };
        // 音量の囲みと、メニューのボタンの上のクリックでは再開しない。
        let menu_hovered = if pause.paused() {
            overlay::menu_item_at(&pause_ui.menu, pointer)
        } else {
            None
        };
        let on_volume_panel =
            pause.paused() && (pause_ui.panel.frame().contains(pointer) || menu_hovered.is_some());
        // 離れたかどうかは毎フレーム読み、覚えていた分を消す。ゲームオーバーの間に離れた分を、次の回へ持ち越さない。
        match pause.update(
            game.over(),
            is_key_pressed(KeyCode::Escape) || (left_click && on_menu_button),
            (left_click || right_click) && !on_volume_panel && !portrait,
            // 縦長の画面では先がほとんど見えず遊べないので、横向きにするまで止めておく。
            page::lost_focus() || portrait,
        ) {
            pause::Frame::Running => {}
            pause::Frame::Paused => {
                if let (true, Some(item)) = (left_click, menu_hovered) {
                    overlay::run_menu_item(item);
                }
                volume.update(
                    is_key_pressed(KeyCode::Left),
                    is_key_pressed(KeyCode::Right),
                    is_key_pressed(KeyCode::M),
                    hovered.filter(|_| left_click),
                );
                // 止まっている間に押した入力は、再開した後に使わない。
                wire_pressed = false;
            }
            pause::Frame::Resumed => {
                // 再開に使ったクリックでワイヤーを撃たない。止まっていた時間も追いかけない。
                wire_pressed = false;
                accumulator = 0.0;
            }
        }
        if game.over() {
            since_over += frame_time;
            // 押した瞬間だけを見る。待ち時間のあいだに押して離した入力は、時間が過ぎてもやり直さない。
            // クリックは「Retry」ボタンの上だけで受け付け、ランキングを見ているときのクリックでは始めない。
            let over_layout = overlay::game_over_layout(screen, &board);
            // 縦長の画面ではゲームオーバーの画面を隠しているので、押しても何もしない。
            let on_retry = !portrait && over_layout.retry.contains(pointer);
            // ゲームオーバーの画面にも、一時停止の画面と同じメニューのボタンを置く。
            let over_item = overlay::menu_item_at(&over_layout.menu, pointer).filter(|_| !portrait);
            if let (true, Some(item)) = (left_click, over_item) {
                overlay::run_menu_item(item);
            }
            if retry_requested(
                since_over,
                left_click && on_retry,
                is_key_pressed(KeyCode::R),
                is_key_pressed(KeyCode::Space),
            ) {
                let old_room = game::room_key(game.seed());
                game = Game::new();
                // 日付が変わっていれば、新しいコースの部屋へ移る。ID は部屋ごとに振られるので、前の部屋の影と名前は捨てる。
                let room = game::room_key(game.seed());
                if room != old_room {
                    shadows = Shadows::default();
                }
                realtime::join(&room);
                accumulator = 0.0;
                since_over = 0.0;
                hit_stop = HitStop::default();
                board.reset();
            }
            wire_pressed = false;
        } else if !pause.paused() {
            // 止まっている時間は貯めない。押されたクリックは残し、止まり終えた後の更新で使う。
            accumulator += hit_stop.consume(frame_time);
            while accumulator >= FIXED_DT && !game.over() && !hit_stop.active() {
                let input = WireInput {
                    aim,
                    pressed: wire_pressed,
                };
                game.step(input, FIXED_DT);
                for event in &game.events {
                    sfx.play(event, volume.gain());
                    shake.kick(event);
                    hit_stop.kick(event);
                }
                wire_pressed = false;
                accumulator -= FIXED_DT;
            }
            if game.over() {
                // 送る前の自己ベストと比べる。送るとすぐ自己ベストが上がる。
                new_best = game.score() > 0
                    && roommate_sdk::best_score().is_none_or(|best| game.score() > best);
                roommate_sdk::submit_score(game.score(), (game.elapsed * 1000.0) as u32);
                board.request();
            }
        }

        // 自己ベストは roommate-sdk が持つ。開いた後にサーバーの値が届くと下がることもあり、
        // データを消すと無くなるので、手元で大きいほうを覚えず毎フレームそのまま読む。
        let best = overlay::best_label(roommate_sdk::best_score());
        board.update(&fonts, &mut name_font);
        for name in shadows.names() {
            name_font.want(&fonts, name);
        }
        name_font.update();
        bgm.update(volume.gain());
        let now = get_time();
        while let Some(message) = realtime::receive() {
            if let Some(incoming) = shadow::read_incoming(&message) {
                shadows.receive(incoming, now);
            }
        }
        shadows.forget_stale(now);
        // 終わった後と一時停止の間は送らない。届かなくなった影は、他の人の画面からしばらくして消える。
        if !game.over() && !pause.paused() {
            until_send -= frame_time;
            if until_send <= 0.0 {
                until_send = shadow::SEND_INTERVAL;
                let state = State::of(&game.player, (now * 1000.0) as u32);
                realtime::send(&state.encode());
            }
        }

        clear_background(Color::from_rgba(16, 18, 24, 255));

        // 揺れは描画用のカメラだけに足す。照準は揺れていない `camera` で変換する。
        let origin = playfield.origin(game.camera_x, game.player.position);
        let world_per_pixel = playfield.world_per_pixel();
        // 背景の夜の街と空は、寄って縦を追うときも上下に動かさず、街の下端を画面の下端に映す。
        let backdrop = vec2(origin.x, game::VIEW_HEIGHT - playfield.world.y);
        let mut shaken = playfield.camera(backdrop);
        shaken.target += shake.offset();
        set_camera(&shaken);
        // 揺れの分も隠れないよう、映る範囲より少し広く塗る。
        let view = playfield.world_view(backdrop);
        let world_view = Rect::new(
            view.x - 200.0,
            view.y - 200.0,
            view.w + 400.0,
            view.h + 400.0,
        );
        background::draw(game.seed(), world_view, game.distance(), now as f32);
        let mut shaken = playfield.camera(origin);
        shaken.target += shake.offset();
        set_camera(&shaken);
        // 影は判定に関わらないので、地形や自機より奥に描く。
        shadows.draw(now, &sprites, &fonts, &name_font, world_per_pixel);
        game.draw(heading, &sprites, world_per_pixel, !touch_input);
        playfield.draw_offscreen_marker(origin, game.player.position, player::PLAYER_RADIUS);

        set_default_camera();
        // ワールドを映す範囲の外は黒く塗り、横長の画面でも先が見える幅を変えない。
        playfield.draw_bars();
        set_camera(&playfield.hud_camera());
        heading.draw(
            &format!(
                "Score {}   {:.0} m   Kills {}",
                game.score(),
                game.distance(),
                game.kills
            ),
            16.0,
            36.0,
            text::HEAD_MEDIUM,
            Color::from_rgba(180, 190, 205, 255),
        );
        text.draw(
            &best,
            16.0,
            64.0,
            text::SMALL,
            Color::from_rgba(150, 160, 175, 255),
        );
        if let Some(playing) = realtime::playing() {
            text.draw(
                &overlay::playing_label(playing),
                16.0,
                88.0,
                text::SMALL,
                Color::from_rgba(150, 160, 175, 255),
            );
        }
        // 遊び方・名前・データの操作は一時停止の画面にまとめたので、開くボタンだけを右上に出す。
        if !game.over() && !pause.paused() {
            overlay::draw_menu_button(heading, screen, pointer);
        }
        if portrait {
            overlay::draw_rotate_hint(&fonts, screen);
        } else if pause.paused() {
            overlay::draw_paused(&fonts, screen, &pause_ui, pointer);
            pause_ui.panel.draw(&sprites, &volume, hovered);
        }
        if game.over() && !portrait {
            overlay::draw_game_over(
                &fonts, &name_font, &game, &board, since_over, new_best, &best, pointer, screen,
            );
        }

        next_frame().await
    }
}

/// 左クリック、R、スペースのどれかでやり直す。
/// 終わった直後の `RETRY_DELAY` の間は、どの入力も受け付けない。
fn retry_requested(since_over: f32, click: bool, key_r: bool, key_space: bool) -> bool {
    since_over >= RETRY_DELAY && (click || key_r || key_space)
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
