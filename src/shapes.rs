//! Простые «красивые» фигуры, которых нет в macroquad:
//! скруглённый прямоугольник, тень, мягкий свет лампы.
//! Пока это заглушки вместо нарисованных спрайтов.

use macroquad::prelude::*;

/// Скруглённый прямоугольник. Рисуется веером треугольников —
/// они не перекрываются, поэтому полупрозрачный цвет ложится ровно.
pub fn rounded_rect(x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color) {
    let r = radius.min(w / 2.0).min(h / 2.0);
    const STEPS: usize = 6;
    let corners = [
        (vec2(x + w - r, y + r), -90.0_f32), // правый верхний
        (vec2(x + w - r, y + h - r), 0.0),   // правый нижний
        (vec2(x + r, y + h - r), 90.0),      // левый нижний
        (vec2(x + r, y + r), 180.0),         // левый верхний
    ];

    let mut points = Vec::with_capacity(4 * (STEPS + 1));
    for (c, start) in corners {
        for i in 0..=STEPS {
            let a = (start + 90.0 * i as f32 / STEPS as f32).to_radians();
            points.push(c + vec2(a.cos(), a.sin()) * r);
        }
    }

    let center = vec2(x + w / 2.0, y + h / 2.0);
    for i in 0..points.len() {
        let next = points[(i + 1) % points.len()];
        draw_triangle(center, points[i], next, color);
    }
}

/// Мягкая тень под предметом: чуть ниже и прозрачная.
pub fn shadow(x: f32, y: f32, w: f32, h: f32, radius: f32) {
    rounded_rect(
        x + 2.0,
        y + 5.0,
        w,
        h,
        radius,
        Color::new(0.0, 0.0, 0.0, 0.28),
    );
}

/// Предмет с тенью.
pub fn object(x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color) {
    shadow(x, y, w, h, radius);
    rounded_rect(x, y, w, h, radius, color);
}

/// Тёплое пятно света: несколько прозрачных кругов друг на друге.
pub fn soft_light(center: Vec2, radius: f32, color: Color) {
    const RINGS: usize = 6;
    for i in 0..RINGS {
        let r = radius * (1.0 - i as f32 / RINGS as f32);
        draw_circle(center.x, center.y, r, Color { a: 0.035, ..color });
    }
}
