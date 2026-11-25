//! Камера мира: панорама и зум, как в колони-симах.
//!
//! Мир измеряется в «мировых пикселях»: один тайл = `map::TILE` (64).
//! `zoom` — сколько пикселей экрана приходится на один мировой пиксель.
//!
//! Управление: WASD / стрелки или перетаскивание правой (средней) кнопкой — двигать,
//! колесо мыши — зум к курсору.

use macroquad::prelude::*;

const MIN_ZOOM: f32 = 0.2;
const MAX_ZOOM: f32 = 3.0;
/// Скорость панорамы клавишами, в пикселях экрана в секунду.
const PAN_SPEED: f32 = 900.0;

pub struct WorldCamera {
    /// Какая точка мира сейчас в центре экрана.
    pub center: Vec2,
    pub zoom: f32,
    drag_last: Option<Vec2>,
}

impl WorldCamera {
    /// A camera at `center` (world px) with `zoom` — no window needed (headless tests).
    pub fn at(center: Vec2, zoom: f32) -> Self {
        Self {
            center,
            zoom: zoom.clamp(MIN_ZOOM, MAX_ZOOM),
            drag_last: None,
        }
    }

    /// Камера, которая целиком показывает мир размером `world_size`
    /// внутри прямоугольника экрана `area` (в пикселях экрана).
    pub fn fit(world_size: Vec2, area: Rect) -> Self {
        let zoom = (area.w / world_size.x)
            .min(area.h / world_size.y)
            .clamp(MIN_ZOOM, MAX_ZOOM);
        let screen_center = vec2(screen_width(), screen_height()) / 2.0;
        let center = world_size / 2.0 - (area.center() - screen_center) / zoom;
        Self {
            center,
            zoom,
            drag_last: None,
        }
    }

    /// Точка экрана → точка мира.
    pub fn screen_to_world(&self, screen: Vec2) -> Vec2 {
        let screen_center = vec2(screen_width(), screen_height()) / 2.0;
        self.center + (screen - screen_center) / self.zoom
    }

    /// Камера macroquad, через которую рисуется мир.
    pub fn to_camera2d(&self) -> Camera2D {
        Camera2D {
            target: self.center,
            // Y положительный: для экрана (без render target) macroquad сам
            // переворачивает ось, и тогда Y мира смотрит вниз, как на экране.
            zoom: vec2(
                2.0 * self.zoom / screen_width(),
                2.0 * self.zoom / screen_height(),
            ),
            ..Default::default()
        }
    }

    /// Ввод камеры. Работает и на паузе (использует реальное время кадра).
    pub fn update(&mut self) {
        let dt = get_frame_time();

        // Клавиши
        let mut dir = Vec2::ZERO;
        if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) {
            dir.y -= 1.0;
        }
        if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) {
            dir.y += 1.0;
        }
        if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) {
            dir.x -= 1.0;
        }
        if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) {
            dir.x += 1.0;
        }
        self.center += dir.normalize_or_zero() * PAN_SPEED * dt / self.zoom;

        // Перетаскивание правой или средней кнопкой
        let mouse = Vec2::from(mouse_position());
        if is_mouse_button_down(MouseButton::Right) || is_mouse_button_down(MouseButton::Middle) {
            if let Some(last) = self.drag_last {
                self.center -= (mouse - last) / self.zoom;
            }
            self.drag_last = Some(mouse);
        } else {
            self.drag_last = None;
        }

        // Зум колесом — так, чтобы точка под курсором осталась на месте
        let wheel = mouse_wheel().1;
        if wheel != 0.0 {
            let before = self.screen_to_world(mouse);
            let factor = (wheel * 0.1).clamp(-0.5, 0.5).exp();
            self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
            let after = self.screen_to_world(mouse);
            self.center += before - after;
        }
    }
}
