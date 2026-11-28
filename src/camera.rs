//! The world camera: pan and zoom, like in colony sims.
//!
//! The world is measured in world pixels: one tile = `map::TILE` (64).
//! `zoom` = screen pixels per world pixel.
//!
//! Controls: WASD / arrows (Shift + WASD while they steer Karolina at night,
//! `screens::game::steer_ui`) or right (middle) button drag to move, mouse wheel to zoom
//! towards the cursor.

use macroquad::prelude::*;

const MIN_ZOOM: f32 = 0.2;
const MAX_ZOOM: f32 = 3.0;
/// Keyboard pan speed, in screen pixels per second.
const PAN_SPEED: f32 = 900.0;

/// The window in pixels — a fixed 1280×720 in unit tests, which build screens headless (no
/// macroquad context there: the dev build's tests open every screen).
pub fn screen_px() -> Rect {
    #[cfg(test)]
    {
        Rect::new(0.0, 0.0, 1280.0, 720.0)
    }
    #[cfg(not(test))]
    {
        Rect::new(0.0, 0.0, screen_width(), screen_height())
    }
}

pub struct WorldCamera {
    /// The world point at the centre of the screen.
    pub center: Vec2,
    pub zoom: f32,
    drag_last: Option<Vec2>,
    /// The last update panned it (keys or a drag — not the wheel): the walking follow stops
    /// for that, not for a zoom.
    pub panned: bool,
}

impl WorldCamera {
    /// A camera at `center` (world px) with `zoom` — no window needed (headless tests).
    pub fn at(center: Vec2, zoom: f32) -> Self {
        Self {
            center,
            zoom: zoom.clamp(MIN_ZOOM, MAX_ZOOM),
            drag_last: None,
            panned: false,
        }
    }

    /// A camera that shows the whole world of `world_size`
    /// inside the screen rectangle `area` (screen pixels).
    pub fn fit(world_size: Vec2, area: Rect) -> Self {
        let zoom = (area.w / world_size.x)
            .min(area.h / world_size.y)
            .clamp(MIN_ZOOM, MAX_ZOOM);
        let screen_center = screen_px().size() / 2.0;
        let center = world_size / 2.0 - (area.center() - screen_center) / zoom;
        Self {
            center,
            zoom,
            drag_last: None,
            panned: false,
        }
    }

    /// Screen point → world point.
    pub fn screen_to_world(&self, screen: Vec2) -> Vec2 {
        let screen_center = vec2(screen_width(), screen_height()) / 2.0;
        self.center + (screen - screen_center) / self.zoom
    }

    /// The macroquad camera the world is drawn with.
    pub fn to_camera2d(&self) -> Camera2D {
        Camera2D {
            target: self.center,
            // Positive Y: for the screen (no render target) macroquad flips the axis
            // itself, so world Y points down like the screen.
            zoom: vec2(
                2.0 * self.zoom / screen_width(),
                2.0 * self.zoom / screen_height(),
            ),
            ..Default::default()
        }
    }

    /// Camera input. Works while paused too (uses real frame time). Returns whether the
    /// player moved it (keys, a drag, the wheel) — that ends a follow.
    pub fn update(&mut self) -> bool {
        self.update_keys(true)
    }

    /// [`WorldCamera::update`]; `keys` = WASD / the arrows pan it (not while they steer
    /// Karolina with them).
    pub fn update_keys(&mut self, keys: bool) -> bool {
        let mut moved = false;
        let dt = get_frame_time();

        // Keys
        let mut dir = Vec2::ZERO;
        let key = |a: KeyCode, b: KeyCode| keys && (is_key_down(a) || is_key_down(b));
        if key(KeyCode::W, KeyCode::Up) {
            dir.y -= 1.0;
        }
        if key(KeyCode::S, KeyCode::Down) {
            dir.y += 1.0;
        }
        if key(KeyCode::A, KeyCode::Left) {
            dir.x -= 1.0;
        }
        if key(KeyCode::D, KeyCode::Right) {
            dir.x += 1.0;
        }
        self.center += dir.normalize_or_zero() * PAN_SPEED * dt / self.zoom;
        moved |= dir != Vec2::ZERO;
        self.panned = moved;

        // Drag with the right or middle button
        let mouse = Vec2::from(mouse_position());
        if is_mouse_button_down(MouseButton::Right) || is_mouse_button_down(MouseButton::Middle) {
            if let Some(last) = self.drag_last {
                self.center -= (mouse - last) / self.zoom;
                moved |= mouse.distance(last) > 0.5;
                self.panned |= mouse.distance(last) > 0.5;
            }
            self.drag_last = Some(mouse);
        } else {
            self.drag_last = None;
        }

        // Wheel zoom that keeps the point under the cursor in place
        let wheel = mouse_wheel().1;
        if wheel != 0.0 {
            let before = self.screen_to_world(mouse);
            let factor = (wheel * 0.1).clamp(-0.5, 0.5).exp();
            self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
            let after = self.screen_to_world(mouse);
            self.center += before - after;
            moved = true;
        }
        moved
    }
}
