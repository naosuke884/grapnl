//! 画面のどこにワールドを映すか。
//! 同じ日は全員が同じコースでスコアを比べるので、画面の形で先が見える幅を変えない。
//! 16:9 より横長の画面では、左右に黒い余白を付けて、映す横幅を 16:9 の幅にとどめる。
//! 16:9 より縦長の画面は、縦の高さを固定したまま横幅を縦横比で決める（余白は付けない）。
//! スマートフォンのような背の低い画面だけは、自機や地形が小さすぎないよう寄って映す（`zoom_for`）。
//! 映す範囲は狭くなるだけで、どの画面でも 16:9 の横幅より先は見えない。

use macroquad::prelude::*;

use crate::game::VIEW_HEIGHT;

/// これより横長の画面では、横幅を増やさず左右に余白を付ける。
pub const MAX_ASPECT: f32 = 16.0 / 9.0;
/// 画面の文字やボタンを、縮めずに描ける画面の大きさ。一時停止とゲームオーバーの画面がこれで収まる。
pub const UI_MIN: Vec2 = vec2(820.0, 640.0);
/// 映すワールドの横幅の上限。16:9 の画面に映す横幅（約 3300）。
pub const MAX_VIEW_WIDTH: f32 = VIEW_HEIGHT * MAX_ASPECT;
/// 背の低い画面で寄る倍率の上限。スマートフォンの横向き（高さ 420 くらいまで）はこの倍率で映す。
pub const MAX_ZOOM: f32 = 1.5;
/// これより背の高い画面では寄らない。ここから低くなるほど寄り、`MAX_ZOOM` で止める。
const ZOOM_FROM_HEIGHT: f32 = 640.0;
/// 寄って縦の全体を映せないときは、自機が映す範囲の上からこの割合より下へ行ったときだけ下へ追う。
/// 振り子で揺れる程度では動かさず、天井を映したままにする。
const FOLLOW_DOWN_FROM: f32 = 0.6;

/// 画面の高さ（論理ピクセル）から、寄る倍率を決める。
fn zoom_for(screen_h: f32) -> f32 {
    (ZOOM_FROM_HEIGHT / screen_h).clamp(1.0, MAX_ZOOM)
}

/// ワールドを映す範囲。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Playfield {
    /// ワールドを映す画面上の長方形（論理ピクセル）。縦は常に画面いっぱい。
    pub rect: Rect,
    /// `rect` に映すワールドの幅と高さ。寄らない画面では高さが `VIEW_HEIGHT`。
    pub world: Vec2,
    /// 寄る倍率。寄らない画面では 1。
    pub zoom: f32,
    /// 画面全体の大きさ（論理ピクセル）。
    pub screen: Vec2,
}

/// 画面の大きさ（論理ピクセル）から、ワールドを映す範囲を決める。
pub fn playfield(screen_w: f32, screen_h: f32) -> Playfield {
    let zoom = zoom_for(screen_h);
    let height = VIEW_HEIGHT / zoom;
    let aspect = screen_w / screen_h;
    // 16:9 ちょうどでは画面いっぱいを返す。幅を掛け算で出し直すと、誤差で 1 px の余白が出る。
    if height * aspect <= MAX_VIEW_WIDTH {
        return Playfield {
            rect: Rect::new(0.0, 0.0, screen_w, screen_h),
            world: vec2(height * aspect, height),
            zoom,
            screen: vec2(screen_w, screen_h),
        };
    }
    let width = screen_h * MAX_VIEW_WIDTH / height;
    Playfield {
        rect: Rect::new((screen_w - width) / 2.0, 0.0, width, screen_h),
        world: vec2(MAX_VIEW_WIDTH, height),
        zoom,
        screen: vec2(screen_w, screen_h),
    }
}

impl Playfield {
    /// 今の画面の大きさで決める。
    pub fn current() -> Self {
        playfield(screen_width(), screen_height())
    }

    /// `rect` の左上に映すワールドの点。`camera_x` は寄らない画面でのカメラの左端、`focus` は自機の位置。
    /// 寄る画面では、自機の横の位置を変えずに寄る（自機から左端までの幅を `zoom` で割る）。
    /// 縦は天井を映したままにし、自機が下へ落ちていくときだけ追う。寄らない画面では `(camera_x, 0)`。
    pub fn origin(&self, camera_x: f32, focus: Vec2) -> Vec2 {
        let x = focus.x - (focus.x - camera_x) / self.zoom;
        let y = (focus.y - self.world.y * FOLLOW_DOWN_FROM).clamp(0.0, VIEW_HEIGHT - self.world.y);
        vec2(x, y)
    }

    /// 画面の 1 ピクセル（論理ピクセル）に映るワールドの幅。
    pub fn world_per_pixel(&self) -> f32 {
        self.world.y / self.rect.h
    }

    /// `rect` の左上に `origin` を映し、`rect` の高さに `world.y` を映すカメラ。
    /// カメラ自体は画面全体に映し、余白の部分は後から黒く塗って隠す（`draw_bars`）。
    /// `Camera2D::viewport` は実ピクセルで指定し、高 DPI の画面で `screen_to_world` とずれるので使わない。
    /// 画面全体に映すので、`screen_to_world` は余白の中のクリックもそのまま変換できる（ワールドの外を指す）。
    /// ワールドは y 下向き。
    /// `Camera2D::from_display_rect` は画面に描くと上下が逆になるので使わない。
    pub fn camera(&self, origin: Vec2) -> Camera2D {
        let size = vec2(self.screen.x * self.world_per_pixel(), self.world.y);
        // 余白は左右同じ幅なので、画面の中央が `rect` の中央になる。
        Camera2D {
            target: origin + self.world / 2.0,
            zoom: vec2(2.0 / size.x, 2.0 / size.y),
            ..Default::default()
        }
    }

    /// 画面の文字やボタンを描くときの縮め方。スマートフォンのような小さい画面でも、
    /// 一時停止やゲームオーバーの画面が収まるよう、`UI_MIN` より小さい画面では全体を縮めて描く。
    /// 大きい画面では 1 のままにし、ドットの文字を滲ませない。
    pub fn ui_scale(&self) -> f32 {
        (self.rect.w / UI_MIN.x)
            .min(self.rect.h / UI_MIN.y)
            .min(1.0)
    }

    /// 画面の文字やボタンを描く座標での、`rect` の大きさ。
    pub fn ui_size(&self) -> Vec2 {
        self.rect.size() / self.ui_scale()
    }

    /// 画面の座標の点を、画面の文字やボタンを描く座標に直す。
    pub fn ui_point(&self, point: Vec2) -> Vec2 {
        (point - self.rect.point()) / self.ui_scale()
    }

    /// 画面の文字やボタンを描くカメラ。`rect` の左上を原点にし、`ui_scale` 倍で映す。
    pub fn hud_camera(&self) -> Camera2D {
        let scale = self.ui_scale();
        Camera2D {
            target: (self.screen / 2.0 - self.rect.point()) / scale,
            zoom: vec2(2.0 * scale / self.screen.x, 2.0 * scale / self.screen.y),
            ..Default::default()
        }
    }

    /// 左右の余白を黒く塗る。ワールドを描いた後、画面の座標で呼ぶ。
    pub fn draw_bars(&self) {
        if self.rect.x <= 0.0 {
            return;
        }
        let h = self.screen.y;
        draw_rectangle(0.0, 0.0, self.rect.x, h, BLACK);
        let right = self.rect.right();
        draw_rectangle(right, 0.0, self.screen.x - right, h, BLACK);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_shrinks_only_on_small_screens() {
        assert_eq!(playfield(1280.0, 720.0).ui_scale(), 1.0);
        assert_eq!(playfield(1024.0, 640.0).ui_scale(), 1.0);
        // スマートフォンの横向きと縦向きは縮め、文字とボタンを置く座標では UI_MIN 以上の広さになる。
        for (w, h) in [(844.0, 390.0), (390.0, 844.0)] {
            let field = playfield(w, h);
            assert!(field.ui_scale() < 1.0);
            let size = field.ui_size();
            assert!(size.x >= UI_MIN.x - 0.01 && size.y >= UI_MIN.y - 0.01);
            assert_eq!(field.ui_point(field.rect.point()), Vec2::ZERO);
        }
    }

    fn assert_close(a: f32, b: f32) {
        assert!((a - b).abs() < 1e-3, "{a} != {b}");
    }

    #[test]
    fn sixteen_by_nine_fills_the_screen() {
        let p = playfield(1280.0, 720.0);
        assert_eq!(p.rect, Rect::new(0.0, 0.0, 1280.0, 720.0));
        assert_close(p.world.x, MAX_VIEW_WIDTH);
        assert_eq!(p.world.y, VIEW_HEIGHT);
        // 高解像度の 16:9 も同じ。
        let p = playfield(1920.0, 1080.0);
        assert_eq!(p.rect, Rect::new(0.0, 0.0, 1920.0, 1080.0));
    }

    #[test]
    fn wider_than_sixteen_by_nine_gets_side_bars() {
        // 21:9 と 32:9。
        for (w, h) in [(2560.0, 1080.0), (5120.0, 1440.0)] {
            let p = playfield(w, h);
            assert_eq!(p.world, vec2(MAX_VIEW_WIDTH, VIEW_HEIGHT));
            assert_close(p.rect.w / p.rect.h, MAX_ASPECT);
            assert_eq!(p.rect.y, 0.0);
            assert_eq!(p.rect.h, h);
            // 左右の余白は同じ幅。
            assert_close(p.rect.x, w - p.rect.right());
            assert!(p.rect.x > 0.0);
        }
        let p = playfield(2560.0, 1080.0);
        assert_close(p.rect.w, 1920.0);
        assert_close(p.rect.x, 320.0);
    }

    #[test]
    fn taller_than_sixteen_by_nine_stays_as_before() {
        // 4:3 と縦長の画面は余白を付けず、横幅を縦横比で狭める。
        for (w, h) in [(1024.0, 768.0), (390.0, 844.0)] {
            let p = playfield(w, h);
            assert_eq!(p.rect, Rect::new(0.0, 0.0, w, h));
            assert_close(p.world.x, VIEW_HEIGHT * w / h);
            assert!(p.world.x < MAX_VIEW_WIDTH);
        }
    }

    /// 画面の座標（論理ピクセル）を、`camera` で写した先のワールド座標に直す。`screen_to_world` と同じ計算。
    fn to_world(p: &Playfield, camera: &Camera2D, point: Vec2) -> Vec2 {
        let ndc = vec2(
            point.x / p.screen.x * 2.0 - 1.0,
            1.0 - point.y / p.screen.y * 2.0,
        );
        camera
            .matrix()
            .inverse()
            .transform_point3(ndc.extend(0.0))
            .truncate()
    }

    #[test]
    fn camera_maps_the_playfield_to_the_view() {
        let camera_x = 5000.0;
        for (w, h) in [
            (1280.0, 720.0),
            (2560.0, 1080.0),
            (5120.0, 1440.0),
            (1024.0, 768.0),
        ] {
            let p = playfield(w, h);
            let camera = p.camera(vec2(camera_x, 0.0));
            let top_left = to_world(&p, &camera, p.rect.point());
            assert_close(top_left.x, camera_x);
            assert_close(top_left.y, 0.0);
            let bottom_right = to_world(&p, &camera, vec2(p.rect.right(), p.rect.bottom()));
            assert_close(bottom_right.x, camera_x + p.world.x);
            assert_close(bottom_right.y, VIEW_HEIGHT);
        }
    }

    #[test]
    fn clicks_in_the_bars_aim_outside_the_view() {
        let p = playfield(2560.0, 1080.0);
        let camera = p.camera(Vec2::ZERO);
        assert!(to_world(&p, &camera, vec2(10.0, 500.0)).x < 0.0);
        assert!(to_world(&p, &camera, vec2(2550.0, 500.0)).x > MAX_VIEW_WIDTH);
    }

    #[test]
    fn hud_origin_is_the_top_left_of_the_playfield() {
        let p = playfield(2560.0, 1080.0);
        let hud = p.hud_camera();
        let origin = to_world(&p, &hud, p.rect.point());
        assert_close(origin.x, 0.0);
        assert_close(origin.y, 0.0);
        // 画面の座標と同じ大きさで描く。
        let corner = to_world(&p, &hud, vec2(p.rect.right(), p.rect.bottom()));
        assert_close(corner.x, p.rect.w);
        assert_close(corner.y, p.rect.h);
    }

    #[test]
    fn short_screens_zoom_in() {
        // 大きい画面とスマートフォンの縦向きは寄らず、カメラは今までどおり。
        for (w, h) in [(1280.0, 720.0), (1024.0, 640.0), (390.0, 844.0)] {
            let p = playfield(w, h);
            assert_eq!(p.zoom, 1.0);
            assert_eq!(p.origin(5000.0, vec2(6200.0, 1500.0)), vec2(5000.0, 0.0));
        }
        // スマートフォンの横向きは寄り、16:9 より横長でも余白を付けずに画面を埋める。
        for (w, h) in [(844.0, 390.0), (740.0, 360.0), (667.0, 375.0)] {
            let p = playfield(w, h);
            assert_eq!(p.zoom, MAX_ZOOM);
            assert_close(p.world.y, VIEW_HEIGHT / MAX_ZOOM);
            assert_eq!(p.rect, Rect::new(0.0, 0.0, w, h));
            assert!(p.world.x < MAX_VIEW_WIDTH);
        }
        // その間の高さでは少しだけ寄る。
        let zoom = playfield(853.0, 480.0).zoom;
        assert!(zoom > 1.0 && zoom < MAX_ZOOM);
    }

    #[test]
    fn zoomed_view_keeps_the_player_in_place_and_follows_falls() {
        let p = playfield(844.0, 390.0);
        let camera_x = 5000.0;
        // 自機を中心に寄るので、左端から自機までの幅は寄った分だけ狭まる。
        let focus = vec2(6200.0, 400.0);
        let origin = p.origin(camera_x, focus);
        assert_close(focus.x - origin.x, (focus.x - camera_x) / MAX_ZOOM);
        // 天井の近くでは上端を映し、落ちていくと追い、終わる高さより下は映さない。
        assert_eq!(origin.y, 0.0);
        let falling = p.origin(camera_x, vec2(6200.0, 1200.0)).y;
        assert!(falling > 0.0);
        assert_close(
            p.origin(camera_x, vec2(6200.0, 5000.0)).y,
            VIEW_HEIGHT - p.world.y,
        );
        // カメラは `origin` から `world` の大きさを `rect` に映す。
        let camera = p.camera(vec2(origin.x, falling));
        let top_left = to_world(&p, &camera, Vec2::ZERO);
        assert_close(top_left.x, origin.x);
        assert_close(top_left.y, falling);
        let bottom_right = to_world(&p, &camera, p.screen);
        assert_close(bottom_right.x, origin.x + p.world.x);
        assert_close(bottom_right.y, falling + p.world.y);
    }

    #[test]
    fn visible_width_never_exceeds_the_cap() {
        for w in [
            320.0, 1280.0, 1281.0, 1680.0, 2560.0, 3440.0, 5120.0, 10_000.0,
        ] {
            assert!(playfield(w, 720.0).world.x <= MAX_VIEW_WIDTH);
        }
    }
}
