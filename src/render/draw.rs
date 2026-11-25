//! Низкоуровневые «кисти» рендера: градиентные фигуры через `draw_mesh`,
//! мягкие тени и свечения, работа с цветом, детерминированный шум.
//!
//! Все фигуры выпуклые и рисуются веером треугольников из центра, поэтому
//! полупрозрачный цвет ложится ровно (без двойного наложения).

use std::cell::RefCell;
use std::f32::consts::TAU;

use macroquad::models::{Mesh, Vertex, draw_mesh};
use macroquad::prelude::*;

// ───────────────────────── цвет ─────────────────────────

/// Линейное смешивание двух цветов (включая альфу).
pub fn mix(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

/// Умножить яркость (альфа не меняется).
pub fn shade(c: Color, k: f32) -> Color {
    Color::new(
        (c.r * k).min(1.0),
        (c.g * k).min(1.0),
        (c.b * k).min(1.0),
        c.a,
    )
}

/// Тот же цвет с другой альфой.
pub fn alpha(c: Color, a: f32) -> Color {
    Color { a, ..c }
}

/// Осветлить к белому на `t`.
pub fn tint_light(c: Color, t: f32) -> Color {
    mix(c, Color::new(1.0, 1.0, 1.0, c.a), t)
}

/// Обесцветить на `t` (0 — как было, 1 — серый).
pub fn desaturate(c: Color, t: f32) -> Color {
    let l = c.r * 0.3 + c.g * 0.59 + c.b * 0.11;
    mix(c, Color::new(l, l, l, c.a), t)
}

// ───────────────────────── шум ─────────────────────────

/// Быстрый целочисленный хеш (вариант splitmix/murmur финализатора).
pub fn hash(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}

/// Хеш → число 0..1.
pub fn hash01(x: u32) -> f32 {
    (hash(x) & 0x00ff_ffff) as f32 / 16_777_215.0
}

/// Хеш двух чисел (например, координат тайла) → 0..1.
pub fn hash2(x: i32, y: i32) -> f32 {
    hash01((x as u32).wrapping_mul(73_856_093) ^ (y as u32).wrapping_mul(19_349_663))
}

/// Гладкий одномерный шум 0..1 (value noise со smoothstep).
pub fn noise1(x: f32, seed: u32) -> f32 {
    let i = x.floor();
    let f = x - i;
    let a = hash01(seed.wrapping_add(i as i32 as u32));
    let b = hash01(seed.wrapping_add(i as i32 as u32).wrapping_add(1));
    let s = f * f * (3.0 - 2.0 * f);
    a + (b - a) * s
}

// ─────────────────────── меш-кисти ───────────────────────

thread_local! {
    static SCRATCH: RefCell<Mesh> = RefCell::new(Mesh {
        vertices: Vec::with_capacity(256),
        indices: Vec::with_capacity(512),
        texture: None,
    });
    static POINTS: RefCell<Vec<Vec2>> = RefCell::new(Vec::with_capacity(128));
}

fn vtx(p: Vec2, c: Color) -> Vertex {
    Vertex::new(p.x, p.y, 0.0, 0.0, 0.0, c)
}

/// Веер треугольников: центр `center` + контур `rim`, цвет каждой точки — `color_at`.
fn fan(center: Vec2, rim: &[Vec2], color_at: impl Fn(Vec2, bool) -> Color) {
    if rim.len() < 3 {
        return;
    }
    SCRATCH.with(|m| {
        let mut m = m.borrow_mut();
        m.vertices.clear();
        m.indices.clear();
        m.vertices.push(vtx(center, color_at(center, true)));
        for &p in rim {
            m.vertices.push(vtx(p, color_at(p, false)));
        }
        let n = rim.len() as u16;
        for i in 0..n {
            m.indices.push(0);
            m.indices.push(1 + i);
            m.indices.push(1 + (i + 1) % n);
        }
        draw_mesh(&m);
    });
}

/// Веер с вертикальным градиентом для звёздных (не обязательно выпуклых) фигур.
pub fn fan_pub(center: Vec2, rim: &[Vec2], y0: f32, y1: f32, top: Color, bottom: Color) {
    fan(center, rim, vgrad(y0, y1, top, bottom));
}

fn with_points<R>(f: impl FnOnce(&mut Vec<Vec2>) -> R) -> R {
    POINTS.with(|p| {
        let mut p = p.borrow_mut();
        p.clear();
        f(&mut p)
    })
}

fn push_rrect(pts: &mut Vec<Vec2>, x: f32, y: f32, w: f32, h: f32, r: f32) {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    const STEPS: usize = 5;
    let corners = [
        (vec2(x + w - r, y + r), -90.0_f32),
        (vec2(x + w - r, y + h - r), 0.0),
        (vec2(x + r, y + h - r), 90.0),
        (vec2(x + r, y + r), 180.0),
    ];
    for (c, start) in corners {
        if r <= 0.01 {
            pts.push(c);
            continue;
        }
        for i in 0..=STEPS {
            let a = (start + 90.0 * i as f32 / STEPS as f32).to_radians();
            pts.push(c + vec2(a.cos(), a.sin()) * r);
        }
    }
}

fn push_ellipse(pts: &mut Vec<Vec2>, c: Vec2, rx: f32, ry: f32, segs: usize) {
    for i in 0..segs {
        let a = i as f32 / segs as f32 * TAU;
        pts.push(c + vec2(a.cos() * rx, a.sin() * ry));
    }
}

fn segs_for(r: f32) -> usize {
    ((r * 0.9) as usize).clamp(14, 40)
}

/// Вертикальный градиент по y: `top` на `y0`, `bottom` на `y1`.
fn vgrad(y0: f32, y1: f32, top: Color, bottom: Color) -> impl Fn(Vec2, bool) -> Color {
    let span = (y1 - y0).max(0.001);
    move |p: Vec2, _| mix(top, bottom, (p.y - y0) / span)
}

/// Скруглённый прямоугольник одним цветом.
pub fn rrect(x: f32, y: f32, w: f32, h: f32, r: f32, c: Color) {
    rrect_v(x, y, w, h, r, c, c);
}

/// Скруглённый прямоугольник с вертикальным градиентом.
pub fn rrect_v(x: f32, y: f32, w: f32, h: f32, r: f32, top: Color, bottom: Color) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    with_points(|pts| {
        push_rrect(pts, x, y, w, h, r);
        fan(
            vec2(x + w / 2.0, y + h / 2.0),
            pts,
            vgrad(y, y + h, top, bottom),
        );
    });
}

/// Прямоугольник со скруглёнными только ВЕРХНИМИ углами (плечи бюста, вывески).
pub fn rrect_top(x: f32, y: f32, w: f32, h: f32, r: f32, top: Color, bottom: Color) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    let r = r.min(w / 2.0).min(h).max(0.0);
    with_points(|pts| {
        const STEPS: usize = 8;
        for i in 0..=STEPS {
            let a = (180.0 + 90.0 * i as f32 / STEPS as f32).to_radians();
            pts.push(vec2(x + r, y + r) + vec2(a.cos(), a.sin()) * r);
        }
        for i in 0..=STEPS {
            let a = (270.0 + 90.0 * i as f32 / STEPS as f32).to_radians();
            pts.push(vec2(x + w - r, y + r) + vec2(a.cos(), a.sin()) * r);
        }
        pts.push(vec2(x + w, y + h));
        pts.push(vec2(x, y + h));
        fan(
            vec2(x + w / 2.0, y + h * 0.5),
            pts,
            vgrad(y, y + h, top, bottom),
        );
    });
}

/// Скруглённый прямоугольник с горизонтальным градиентом (лево → право).
pub fn rrect_h(x: f32, y: f32, w: f32, h: f32, r: f32, left: Color, right: Color) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    with_points(|pts| {
        push_rrect(pts, x, y, w, h, r);
        let span = w.max(0.001);
        fan(vec2(x + w / 2.0, y + h / 2.0), pts, |p, _| {
            mix(left, right, (p.x - x) / span)
        });
    });
}

/// Эллипс одним цветом.
pub fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32, c: Color) {
    ellipse_v(cx, cy, rx, ry, c, c);
}

/// Эллипс с вертикальным градиентом.
pub fn ellipse_v(cx: f32, cy: f32, rx: f32, ry: f32, top: Color, bottom: Color) {
    if rx <= 0.0 || ry <= 0.0 {
        return;
    }
    with_points(|pts| {
        push_ellipse(pts, vec2(cx, cy), rx, ry, segs_for(rx.max(ry)));
        fan(vec2(cx, cy), pts, vgrad(cy - ry, cy + ry, top, bottom));
    });
}

/// Радиальный градиент: `inner` в центре → `outer` по краю эллипса.
/// Три кольца со smoothstep-спадом — выглядит как мягкое пятно, а не конус.
pub fn radial(cx: f32, cy: f32, rx: f32, ry: f32, inner: Color, outer: Color) {
    if rx <= 0.0 || ry <= 0.0 {
        return;
    }
    let segs = segs_for(rx.max(ry)).max(24);
    let rings = [0.0_f32, 0.35, 0.7, 1.0];
    SCRATCH.with(|m| {
        let mut m = m.borrow_mut();
        m.vertices.clear();
        m.indices.clear();
        m.vertices.push(vtx(vec2(cx, cy), inner));
        for &k in &rings[1..] {
            // smoothstep(1→0): мягкий спад к краю
            let s = 1.0 - k;
            let w = s * s * (3.0 - 2.0 * s);
            let col = mix(outer, inner, w);
            for i in 0..segs {
                let a = i as f32 / segs as f32 * TAU;
                m.vertices
                    .push(vtx(vec2(cx + a.cos() * rx * k, cy + a.sin() * ry * k), col));
            }
        }
        let s = segs as u16;
        for i in 0..s {
            m.indices.extend_from_slice(&[0, 1 + i, 1 + (i + 1) % s]);
        }
        for ring in 0..2u16 {
            let a0 = 1 + ring * s;
            let b0 = a0 + s;
            for i in 0..s {
                let j = (i + 1) % s;
                m.indices
                    .extend_from_slice(&[a0 + i, b0 + i, b0 + j, a0 + i, b0 + j, a0 + j]);
            }
        }
        draw_mesh(&m);
    });
}

/// Мягкое свечение заданного цвета (центр — `c`, край — прозрачный).
pub fn glow(cx: f32, cy: f32, r: f32, c: Color) {
    radial(cx, cy, r, r, c, alpha(c, 0.0));
}

/// Мягкая тень-эллипс на полу.
pub fn soft_shadow(cx: f32, cy: f32, rx: f32, ry: f32, strength: f32) {
    radial(
        cx,
        cy,
        rx,
        ry,
        Color::new(0.03, 0.02, 0.08, strength),
        Color::new(0.03, 0.02, 0.08, 0.0),
    );
}

/// Четырёхугольник с цветом в каждой вершине (по часовой или против).
pub fn quad(p: [Vec2; 4], c: [Color; 4]) {
    SCRATCH.with(|m| {
        let mut m = m.borrow_mut();
        m.vertices.clear();
        m.indices.clear();
        for i in 0..4 {
            m.vertices.push(vtx(p[i], c[i]));
        }
        m.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
        draw_mesh(&m);
    });
}

/// Прямоугольник с вертикальным градиентом (без скруглений, дёшево).
pub fn rect_v(x: f32, y: f32, w: f32, h: f32, top: Color, bottom: Color) {
    quad(
        [
            vec2(x, y),
            vec2(x + w, y),
            vec2(x + w, y + h),
            vec2(x, y + h),
        ],
        [top, top, bottom, bottom],
    );
}

/// Прямоугольник с горизонтальным градиентом.
pub fn rect_h(x: f32, y: f32, w: f32, h: f32, left: Color, right: Color) {
    quad(
        [
            vec2(x, y),
            vec2(x + w, y),
            vec2(x + w, y + h),
            vec2(x, y + h),
        ],
        [left, right, right, left],
    );
}

/// «Капсула» — отрезок с круглыми концами (руки, ноги, ручки).
pub fn capsule(a: Vec2, b: Vec2, r: f32, c: Color) {
    let d = b - a;
    let len = d.length();
    if len < 0.01 {
        draw_circle(a.x, a.y, r, c);
        return;
    }
    let n = vec2(-d.y, d.x) / len * r;
    // Рисуем одной выпуклой фигурой, чтобы прозрачность не двоилась.
    with_points(|pts| {
        let ang = n.y.atan2(n.x);
        const HALF: usize = 8;
        for i in 0..=HALF {
            let t = ang + std::f32::consts::PI * i as f32 / HALF as f32;
            pts.push(a + vec2(t.cos(), t.sin()) * r);
        }
        for i in 0..=HALF {
            let t = ang + std::f32::consts::PI + std::f32::consts::PI * i as f32 / HALF as f32;
            pts.push(b + vec2(t.cos(), t.sin()) * r);
        }
        fan((a + b) / 2.0, pts, |_, _| c);
    });
}

/// Выпуклый многоугольник одним цветом.
pub fn poly(points: &[Vec2], c: Color) {
    if points.len() < 3 {
        return;
    }
    let center = points.iter().copied().sum::<Vec2>() / points.len() as f32;
    fan(center, points, |_, _| c);
}

/// Выпуклый многоугольник с вертикальным градиентом.
pub fn poly_v(points: &[Vec2], top: Color, bottom: Color) {
    if points.len() < 3 {
        return;
    }
    let (mut y0, mut y1) = (f32::MAX, f32::MIN);
    for p in points {
        y0 = y0.min(p.y);
        y1 = y1.max(p.y);
    }
    let center = points.iter().copied().sum::<Vec2>() / points.len() as f32;
    fan(center, points, vgrad(y0, y1, top, bottom));
}

/// Дуга-штрих (для улыбок, бровей, колец выделения).
pub fn arc(c: Vec2, rx: f32, ry: f32, a0: f32, a1: f32, thick: f32, col: Color) {
    let steps = (((a1 - a0).abs() * rx.max(ry) / 3.0) as usize).clamp(4, 40);
    let mut prev = c + vec2(a0.cos() * rx, a0.sin() * ry);
    for i in 1..=steps {
        let a = a0 + (a1 - a0) * i as f32 / steps as f32;
        let p = c + vec2(a.cos() * rx, a.sin() * ry);
        draw_line(prev.x, prev.y, p.x, p.y, thick, col);
        prev = p;
    }
}

/// Мягкая линия: широкий полупрозрачный штрих + тонкое ядро (свечение неона).
pub fn glow_line(a: Vec2, b: Vec2, thick: f32, c: Color) {
    draw_line(a.x, a.y, b.x, b.y, thick * 3.0, alpha(c, c.a * 0.18));
    draw_line(a.x, a.y, b.x, b.y, thick * 1.8, alpha(c, c.a * 0.35));
    draw_line(a.x, a.y, b.x, b.y, thick, c);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mix_endpoints() {
        let a = Color::new(0.0, 0.0, 0.0, 1.0);
        let b = Color::new(1.0, 0.5, 0.2, 0.0);
        assert_eq!(mix(a, b, 0.0), a);
        assert_eq!(mix(a, b, 1.0), b);
        assert_eq!(mix(a, b, 7.0), b, "t is clamped");
    }

    #[test]
    fn noise_is_bounded_and_deterministic() {
        for i in 0..200 {
            let x = i as f32 * 0.37;
            let n = noise1(x, 7);
            assert!((0.0..=1.0).contains(&n));
            assert_eq!(n, noise1(x, 7));
        }
        assert!((0.0..=1.0).contains(&hash2(-5, 9)));
    }
}
