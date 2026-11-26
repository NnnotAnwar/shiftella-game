//! Персонажи в виде 3/4 спереди, портреты для диалогов, эмоции и выделение.
//!
//! Фигура строится из мягких градиентных форм без контура. Рост ≈ 1.3 тайла
//! при `height = 1.0`. Объём даёт лавандовый «rim light» сверху-слева
//! (свет из окон) — силуэт читается и без чёрной обводки.

use std::f32::consts::PI;

use macroquad::prelude::*;

use super::colors as c;
use super::draw::*;
use super::look::*;
use crate::map::TILE;

const SHOE: Color = Color::new(0.16, 0.13, 0.2, 1.0);
const EYE: Color = Color::new(0.17, 0.13, 0.22, 1.0);
const BLUSH: Color = Color::new(0.94, 0.45, 0.52, 0.3);

// ───────────────────────── тон ─────────────────────────

fn tone_color(tone: Tone, col: Color) -> Color {
    match tone {
        Tone::Normal => col,
        Tone::Memory => mix(
            desaturate(col, 0.35),
            Color::new(0.86, 0.7, 0.6, col.a),
            0.22,
        ),
        Tone::Faded => mix(
            desaturate(col, 0.42),
            Color::new(0.62, 0.66, 0.8, col.a),
            0.12,
        ),
    }
}

/// Копия внешности с применённым тоном (сепия воспоминания, блёклость 3:33).
fn toned(l: &PersonLook) -> PersonLook {
    let f = |col| tone_color(l.tone, col);
    PersonLook {
        skin: f(l.skin),
        hair: f(l.hair),
        top: f(l.top),
        bottom: f(l.bottom),
        accent: f(l.accent),
        ..*l
    }
}

/// Цвет фартука и значка (тоже подчиняются тону).
fn apron_colors(l: &PersonLook) -> (Color, Color) {
    (
        tone_color(l.tone, c::INDIGO_600),
        tone_color(l.tone, Color::from_hex(0x372fa0)),
    )
}

// ───────────────────────── голова ─────────────────────────

/// Голова: центр и радиус в пикселях. Внутри смещения в «юнитах» u = r/14,
/// ось y вниз, x зеркалится по `s` (для профиля: +1 смотрит вправо).
struct Head<'a> {
    c: Vec2,
    u: f32,
    s: f32,
    facing: Facing,
    l: &'a PersonLook,
}

impl Head<'_> {
    fn p(&self, x: f32, y: f32) -> Vec2 {
        self.c + vec2(x * self.s, y) * self.u
    }
    fn ell(&self, x: f32, y: f32, rx: f32, ry: f32, col: Color) {
        let p = self.p(x, y);
        ellipse(p.x, p.y, rx * self.u, ry * self.u, col);
    }
    fn ell_v(&self, x: f32, y: f32, rx: f32, ry: f32, top: Color, bottom: Color) {
        let p = self.p(x, y);
        ellipse_v(p.x, p.y, rx * self.u, ry * self.u, top, bottom);
    }
    /// Прямоугольник в юнитах (x0 — левый край до зеркалирования).
    fn rr(&self, x0: f32, y0: f32, w: f32, h: f32, r: f32, top: Color, bottom: Color) {
        let a = self.p(x0, y0);
        let b = self.p(x0 + w, y0 + h);
        let x = a.x.min(b.x);
        rrect_v(
            x,
            a.y,
            (b.x - a.x).abs(),
            b.y - a.y,
            r * self.u,
            top,
            bottom,
        );
    }
    fn cap(&self, x0: f32, y0: f32, x1: f32, y1: f32, r: f32, col: Color) {
        capsule(self.p(x0, y0), self.p(x1, y1), r * self.u, col);
    }
    fn line(&self, x0: f32, y0: f32, x1: f32, y1: f32, w: f32, col: Color) {
        let (a, b) = (self.p(x0, y0), self.p(x1, y1));
        draw_line(a.x, a.y, b.x, b.y, w * self.u, col);
    }
    fn arc(&self, x: f32, y: f32, rx: f32, ry: f32, a0: f32, a1: f32, w: f32, col: Color) {
        // зеркалим дугу: углы отражаются относительно вертикали
        let (a0, a1) = if self.s < 0.0 {
            (PI - a1, PI - a0)
        } else {
            (a0, a1)
        };
        arc(
            self.p(x, y),
            rx * self.u,
            ry * self.u,
            a0,
            a1,
            w * self.u,
            col,
        );
    }

    fn hair_top(&self) -> Color {
        tint_light(self.l.hair, 0.12)
    }
    fn hair_bot(&self) -> Color {
        shade(self.l.hair, 0.82)
    }

    /// Купол волос: верх окружности, срезанный горизонталью `cut` (выпуклая фигура).
    fn dome(&self, cx: f32, cy: f32, r: f32, cut: f32, top: Color, bottom: Color) {
        let sin0 = ((cut - cy) / r).clamp(-0.99, 0.99);
        let a0 = sin0.asin(); // угол среза справа (y вниз)
        let a_left = PI - a0;
        let a_right = 2.0 * PI + a0;
        let mut pts = [Vec2::ZERO; 26];
        for (i, p) in pts.iter_mut().enumerate() {
            let a = a_left + (a_right - a_left) * i as f32 / 25.0;
            *p = self.p(cx + a.cos() * r, cy + a.sin() * r);
        }
        poly_v(&pts, top, bottom);
    }

    /// Купол с естественной линией роста волос: по бокам ниже (`cut_side`),
    /// посередине выше (`cut_mid`). Звёздная фигура — веер из точки над линией.
    fn dome_arched(&self, cy: f32, r: f32, cut_side: f32, cut_mid: f32, top: Color, bottom: Color) {
        let sin0 = ((cut_side - cy) / r).clamp(-0.99, 0.99);
        let a0 = sin0.asin();
        let xw = a0.cos() * r;
        let mut pts = [Vec2::ZERO; 36];
        let a_left = PI - a0;
        let a_right = 2.0 * PI + a0;
        for (i, p) in pts.iter_mut().enumerate().take(24) {
            let a = a_left + (a_right - a_left) * i as f32 / 23.0;
            *p = self.p(a.cos() * r, cy + a.sin() * r);
        }
        for i in 0..12 {
            let x = xw - 2.0 * xw * (i as f32 + 0.5) / 12.0;
            let k = (x / xw).powi(2);
            pts[24 + i] = self.p(x, cut_mid + (cut_side - cut_mid) * k);
        }
        let center = self.p(0.0, cy - r * 0.55);
        let (y0, y1) = (self.c.y + (cy - r) * self.u, self.c.y + cut_side * self.u);
        super::draw::fan_pub(center, &pts, y0, y1, top, bottom);
    }

    /// Слой волос ЗА головой (объём, пучок, хвост сбоку).
    fn hair_back(&self) {
        let (t, b) = (self.hair_top(), self.hair_bot());
        let l = self.l;
        let up = self.facing == Facing::Up;
        let side = matches!(self.facing, Facing::Left | Facing::Right);
        match l.hair_style {
            HairStyle::Bob => {
                if !up {
                    self.rr(-15.8, -12.0, 31.6, 23.0, 8.0, t, b);
                }
            }
            HairStyle::Long => {
                if !up {
                    self.rr(-16.0, -12.0, 32.0, 28.0, 9.0, t, b);
                }
            }
            HairStyle::Bun => {
                if !up {
                    let (x, y) = if side { (-9.0, -12.0) } else { (0.0, -15.5) };
                    self.ell_v(x, y, 6.8, 6.2, t, b);
                }
            }
            HairStyle::Ponytail => {
                if self.facing == Facing::Down {
                    self.ell_v(13.5, 5.0, 4.2, 8.0, t, b);
                } else if side {
                    self.ell_v(-15.0, 6.0, 4.4, 9.5, t, b);
                }
            }
            HairStyle::Curly if !up => {
                for i in 0..11 {
                    let a = PI * (0.9 + i as f32 * 0.12);
                    let (x, y) = (a.cos() * 13.5, a.sin() * 13.0 - 1.0);
                    self.ell_v(x, y, 6.0, 6.0, t, b);
                }
            }
            _ => {}
        }
    }

    fn head_shape(&self) {
        let l = self.l;
        // rim light сверху-слева: светлый серп за головой
        let rim = mix(l.skin, c::LAVENDER_RIM, 0.55);
        let pr = self.c + vec2(-0.9, -1.1) * self.u;
        ellipse(pr.x, pr.y, 14.0 * self.u, 13.8 * self.u, rim);
        // уши
        match self.facing {
            Facing::Down => {
                self.ell(-13.4, 2.0, 2.6, 3.4, shade(l.skin, 0.92));
                self.ell(13.4, 2.0, 2.6, 3.4, shade(l.skin, 0.92));
            }
            Facing::Up => {
                self.ell(-13.6, 1.0, 2.4, 3.2, shade(l.skin, 0.9));
                self.ell(13.6, 1.0, 2.4, 3.2, shade(l.skin, 0.9));
            }
            _ => {}
        }
        self.ell_v(
            0.0,
            0.0,
            14.0,
            13.8,
            tint_light(l.skin, 0.08),
            shade(l.skin, 0.88),
        );
        // мягкая тень под подбородком/щеками
        self.ell(0.0, 10.5, 9.0, 2.6, alpha(shade(l.skin, 0.75), 0.25));
    }

    fn face(&self, expr: Expression, blink: f32, gaze: Vec2) {
        let l = self.l;
        let side = matches!(self.facing, Facing::Left | Facing::Right);
        let skin_d = shade(l.skin, 0.6);
        let mouth_col = mix(Color::from_hex(0x8a3d4f), skin_d, 0.25);
        let brow_col = mix(shade(l.hair, 0.8), skin_d, 0.25);

        // румянец
        let blush_a = match expr {
            Expression::Smile | Expression::Laugh => 0.45,
            Expression::Tired | Expression::Sad => 0.16,
            _ => 0.28,
        };
        let blush = alpha(BLUSH, blush_a);
        // борода/щетина — до рта
        match l.facial_hair {
            FacialHair::Stubble => {
                let x = if side { 4.0 } else { 0.0 };
                // лёгкая щетина: только подбородок и линия челюсти
                self.ell(x, 9.4, 8.4, 3.8, alpha(shade(l.hair, 0.8), 0.14));
                self.ell(x, 10.6, 6.0, 2.6, alpha(shade(l.hair, 0.8), 0.1));
            }
            FacialHair::Beard => {
                let x = if side { 3.0 } else { 0.0 };
                // аккуратная борода по линии челюсти
                self.ell_v(x, 9.4, 9.4, 4.4, l.hair, shade(l.hair, 0.8));
                self.ell(x, 6.4, 7.5, 2.2, alpha(l.hair, 0.55));
            }
            _ => {}
        }

        let eye_ys = match expr {
            Expression::Sad => 2.3,
            _ => 1.6,
        };
        let eyes: &[f32] = if side { &[6.6] } else { &[-5.2, 5.2] };
        for &ex in eyes {
            let ey = eye_ys;
            let (mut rx, mut ry) = if side { (1.6, 2.4) } else { (1.9, 2.5) };
            if expr == Expression::Surprised {
                rx *= 1.2;
                ry *= 1.25;
            }
            if expr == Expression::Smile {
                ry *= 0.85;
            }
            let closed = blink > 0.5;
            if expr == Expression::Laugh || (expr == Expression::Smile && closed) {
                // глаза-дуги ^ ^
                self.arc(ex, ey + 1.2, 2.3, 2.0, PI * 1.1, PI * 1.9, 1.1, EYE);
                continue;
            }
            if closed {
                self.arc(ex, ey - 0.2, 2.1, 1.0, PI * 0.1, PI * 0.9, 1.0, EYE);
                continue;
            }
            let g = vec2(gaze.x * 0.7, gaze.y * 0.6);
            self.ell(ex + g.x, ey + g.y, rx, ry, EYE);
            self.ell(ex + g.x - 0.6, ey + g.y - 0.9, 0.75, 0.75, WHITE);
            if l.tired && expr != Expression::Tired {
                // чуть уставшие глаза: лёгкое веко сверху и мягкие тени снизу
                self.rr(
                    ex - 2.4,
                    ey - 3.1,
                    4.8,
                    1.3,
                    0.6,
                    l.skin,
                    shade(l.skin, 0.95),
                );
                self.arc(
                    ex,
                    ey + 1.6,
                    2.1,
                    1.0,
                    PI * 0.2,
                    PI * 0.8,
                    0.55,
                    alpha(shade(l.skin, 0.55), 0.3),
                );
            }
            if expr == Expression::Tired {
                // тяжёлые веки + мешки
                self.rr(
                    ex - 2.6,
                    ey - 3.2,
                    5.2,
                    3.0,
                    1.0,
                    l.skin,
                    shade(l.skin, 0.93),
                );
                self.arc(
                    ex,
                    ey + 1.4,
                    2.2,
                    1.1,
                    PI * 0.15,
                    PI * 0.85,
                    0.6,
                    alpha(skin_d, 0.35),
                );
            }
            if l.elder {
                let ox = if ex > 0.0 { ex + 3.2 } else { ex - 3.2 };
                self.line(
                    ox,
                    ey,
                    ox + ex.signum() * 1.4,
                    ey - 0.6,
                    0.45,
                    alpha(skin_d, 0.35),
                );
                self.line(
                    ox,
                    ey + 0.8,
                    ox + ex.signum() * 1.4,
                    ey + 1.2,
                    0.45,
                    alpha(skin_d, 0.3),
                );
            }
        }

        // брови
        for &ex in eyes {
            let sx = ex.signum();
            let (inner, outer) = match expr {
                Expression::Sad => (-4.2, -2.6),
                Expression::Surprised => (-5.2, -5.0),
                Expression::Tired => (-2.6, -2.6),
                Expression::Thinking if ex > 0.0 || side => (-4.4, -5.0),
                _ => (-3.4, -3.2),
            };
            if side {
                self.line(ex - 2.2, outer, ex + 2.0, inner, 0.85, brow_col);
            } else {
                self.line(ex - sx * 1.4, inner, ex + sx * 2.4, outer, 0.85, brow_col);
            }
        }

        // румянец
        if side {
            self.ell(6.0, 5.6, 2.3, 1.4, blush);
        } else {
            self.ell(-8.2, 5.4, 2.4, 1.4, blush);
            self.ell(8.2, 5.4, 2.4, 1.4, blush);
        }

        // нос
        if side {
            self.ell(13.4, 3.4, 1.8, 1.6, shade(l.skin, 0.97));
            self.ell(13.0, 4.4, 1.0, 0.5, alpha(skin_d, 0.25));
        } else {
            self.ell(0.0, 4.2, 1.0, 0.6, alpha(skin_d, 0.3));
        }

        // рот
        let mx = if side { 9.2 } else { 0.0 };
        match expr {
            Expression::Neutral => {
                self.arc(mx, 5.6, 2.1, 1.1, PI * 0.25, PI * 0.75, 0.9, mouth_col)
            }
            Expression::Smile => self.arc(mx, 5.2, 3.0, 2.0, PI * 0.15, PI * 0.85, 1.0, mouth_col),
            Expression::Laugh => {
                let w = if side { 2.0 } else { 3.2 };
                self.ell(mx, 7.2, w, 2.4, Color::from_hex(0x5a2238));
                self.ell(mx, 8.3, w * 0.6, 1.0, Color::from_hex(0xd8667a));
            }
            Expression::Sad => self.arc(mx, 8.4, 2.4, 1.3, PI * 1.2, PI * 1.8, 0.9, mouth_col),
            Expression::Surprised => {
                self.ell(mx, 7.2, 1.4, 1.8, Color::from_hex(0x5a2238));
            }
            Expression::Thinking => self.line(mx - 0.5, 7.0, mx + 2.8, 6.4, 0.9, mouth_col),
            Expression::Tired => self.line(mx - 1.8, 7.0, mx + 1.8, 7.0, 0.8, mouth_col),
        }
        if l.facial_hair == FacialHair::Mustache {
            if side {
                self.ell(9.6, 5.2, 2.8, 1.2, l.hair);
            } else {
                self.ell(-2.0, 5.0, 2.6, 1.2, l.hair);
                self.ell(2.0, 5.0, 2.6, 1.2, l.hair);
            }
        }
    }

    /// Волосы ПОВЕРХ головы (купол, чёлка, боковые пряди).
    fn hair_front(&self) {
        let l = self.l;
        let (t, b) = (self.hair_top(), self.hair_bot());
        let side = matches!(self.facing, Facing::Left | Facing::Right);
        let up = self.facing == Facing::Up;

        if l.hair_style == HairStyle::Bald {
            if !up && !side {
                self.ell_v(-12.6, -1.0, 2.8, 4.6, t, b);
                self.ell_v(12.6, -1.0, 2.8, 4.6, t, b);
            } else {
                self.ell_v(-6.0, 1.0, 8.0, 5.5, t, b);
            }
            return;
        }
        let buzz = l.hair_style == HairStyle::Buzz;
        let (t, b) = if buzz {
            (mix(t, l.skin, 0.3), mix(b, l.skin, 0.3))
        } else {
            (t, b)
        };

        if up {
            // со спины волосы закрывают почти всю голову
            self.ell_v(0.0, -1.0, 15.0, 14.6, t, b);
            match l.hair_style {
                HairStyle::Bob => self.rr(-15.8, -4.0, 31.6, 15.0, 7.0, b, shade(b, 0.92)),
                HairStyle::Long => self.rr(-15.5, -4.0, 31.0, 30.0, 8.0, b, shade(b, 0.88)),
                HairStyle::Ponytail => {
                    self.ell_v(0.0, 15.0, 4.4, 9.0, b, shade(b, 0.85));
                    self.ell(0.0, 6.5, 3.0, 1.4, self.l.accent);
                }
                HairStyle::Bun => {
                    self.ell_v(0.0, -10.0, 6.8, 6.2, t, b);
                    self.ell(0.0, -5.6, 4.0, 1.2, shade(b, 0.8));
                }
                HairStyle::Curly => {
                    for i in 0..9 {
                        let a = PI * (0.85 + i as f32 * 0.163);
                        self.ell_v(a.cos() * 13.0, a.sin() * 12.0, 6.2, 6.0, t, b);
                    }
                }
                _ => {}
            }
            return;
        }

        if side {
            // купол + масса сзади, лоб и лицо открыты
            let cut = if buzz { -5.5 } else { -4.4 };
            self.dome(-0.5, -0.8, if buzz { 14.6 } else { 15.3 }, cut, t, b);
            let back_r = match l.hair_style {
                HairStyle::Long | HairStyle::Bob => (11.0, 15.5),
                HairStyle::Curly => (11.0, 14.0),
                _ => (9.0, 11.5),
            };
            self.ell_v(-5.5, 0.5, back_r.0, back_r.1, t, b);
            match l.hair_style {
                HairStyle::Long => self.rr(-14.0, 2.0, 14.0, 22.0, 7.0, b, shade(b, 0.85)),
                HairStyle::Bob => self.rr(-14.0, 2.0, 15.0, 10.0, 5.0, b, shade(b, 0.9)),
                HairStyle::Side | HairStyle::Short => self.ell_v(6.0, -5.4, 6.5, 3.2, t, b),
                HairStyle::Curly => {
                    // кудри только сверху и на затылке — профиль лица открыт
                    for i in 0..7 {
                        let a = PI * (0.55 + i as f32 * 0.14);
                        self.ell_v(a.cos() * 11.0 - 3.0, a.sin() * 11.0 - 1.0, 5.2, 5.2, t, b);
                    }
                }
                _ => {}
            }
            // ухо поверх коротких волос
            if !matches!(l.hair_style, HairStyle::Long | HairStyle::Bob) {
                self.ell(-0.5, 2.0, 2.4, 3.2, shade(l.skin, 0.9));
                self.ell(-0.3, 2.0, 1.1, 1.8, shade(l.skin, 0.8));
            }
            return;
        }

        // лицом к камере
        match l.hair_style {
            HairStyle::Buzz => self.dome_arched(-0.6, 14.7, -3.5, -7.5, t, b),
            HairStyle::Short => {
                self.dome(0.0, -0.8, 15.3, -4.6, t, b);
                for (x, w) in [(-7.0, 3.6), (-1.5, 3.8), (4.5, 3.4)] {
                    self.ell_v(x, -4.6, w, 2.2, b, shade(b, 0.95));
                }
                self.rr(-14.4, -5.0, 2.8, 7.0, 1.4, b, b);
                self.rr(11.6, -5.0, 2.8, 7.0, 1.4, b, b);
            }
            HairStyle::Side => {
                self.dome(0.0, -0.8, 15.3, -5.2, t, b);
                self.cap(-10.0, -3.6, 8.0, -6.8, 3.2, b);
                self.rr(-14.4, -5.0, 2.8, 7.0, 1.4, b, b);
                self.rr(11.6, -5.0, 2.8, 6.0, 1.4, b, b);
            }
            HairStyle::Bob => {
                self.dome(0.0, -0.6, 15.6, -3.4, t, b);
                self.rr(-15.8, -6.0, 5.0, 16.5, 2.5, b, shade(b, 0.9));
                self.rr(10.8, -6.0, 5.0, 16.5, 2.5, b, shade(b, 0.9));
            }
            HairStyle::Long => {
                self.dome_arched(-0.8, 15.6, -2.5, -7.0, t, b);
                // пробор посередине
                self.line(0.0, -14.5, 0.0, -9.0, 0.6, alpha(shade(b, 0.7), 0.6));
                self.rr(-16.0, -6.0, 5.0, 24.0, 2.5, b, shade(b, 0.88));
                self.rr(11.0, -6.0, 5.0, 24.0, 2.5, b, shade(b, 0.88));
            }
            HairStyle::Ponytail => {
                self.dome_arched(-0.8, 15.2, -2.8, -7.6, t, b);
                self.line(
                    -7.0,
                    -11.0,
                    -1.0,
                    -14.0,
                    0.5,
                    alpha(tint_light(t, 0.2), 0.5),
                );
            }
            HairStyle::Bun => {
                self.dome_arched(-0.8, 15.0, -3.0, -7.8, t, b);
                self.line(
                    -6.0,
                    -11.5,
                    6.0,
                    -11.5,
                    0.5,
                    alpha(tint_light(t, 0.25), 0.4),
                );
            }
            HairStyle::Curly => {
                self.dome(0.0, -1.0, 15.6, -5.0, t, b);
                for i in 0..6 {
                    let x = -10.0 + i as f32 * 4.0;
                    self.ell_v(x, -5.0 + (i % 2) as f32 * 0.8, 3.0, 2.8, t, b);
                }
            }
            HairStyle::Bald => {}
        }
    }

    fn accessory(&self) {
        let l = self.l;
        let side = matches!(self.facing, Facing::Left | Facing::Right);
        let up = self.facing == Facing::Up;
        let a = l.accent;
        match l.accessory {
            Accessory::Glasses if !up => {
                let frame = alpha(mix(Color::from_hex(0x3a2f45), a, 0.25), 0.95);
                let lens = Color::new(0.85, 0.88, 1.0, 0.18);
                let eyes: &[f32] = if side { &[6.8] } else { &[-5.3, 5.3] };
                for &ex in eyes {
                    self.ell(ex, 1.8, 3.8, 3.3, lens);
                    let p = self.p(ex, 1.8);
                    draw_ellipse_lines(
                        p.x,
                        p.y,
                        3.8 * self.u,
                        3.3 * self.u,
                        0.0,
                        0.8 * self.u,
                        frame,
                    );
                }
                if side {
                    self.line(3.0, 1.6, -0.5, 1.0, 0.8, frame);
                } else {
                    self.line(-1.5, 1.4, 1.5, 1.4, 0.8, frame);
                }
            }
            Accessory::Beanie => {
                let (t, b) = (tint_light(a, 0.1), shade(a, 0.85));
                self.dome(0.0, -1.5, 15.6, -6.0, t, b);
                self.rr(-15.4, -8.5, 30.8, 4.6, 2.2, shade(a, 0.78), shade(a, 0.7));
                for i in 0..6 {
                    let x = -11.0 + i as f32 * 4.4;
                    self.line(x, -7.8, x, -4.6, 0.5, alpha(shade(a, 0.6), 0.6));
                }
                self.ell(0.0, -16.5, 3.0, 3.0, tint_light(a, 0.25));
            }
            Accessory::Headphones => {
                let band = Color::from_hex(0x2c2f3e);
                self.arc(0.0, -1.0, 15.5, 15.0, PI * 1.05, PI * 1.95, 2.0, band);
                if side {
                    self.ell(-0.5, 2.0, 3.8, 4.6, band);
                    self.ell(-0.5, 2.0, 2.0, 2.6, a);
                } else {
                    self.ell(-14.5, 2.0, 3.0, 4.6, band);
                    self.ell(14.5, 2.0, 3.0, 4.6, band);
                    self.ell(-14.8, 2.0, 1.4, 2.6, a);
                    self.ell(14.8, 2.0, 1.4, 2.6, a);
                }
            }
            Accessory::Earrings if !up => {
                let gold = Color::from_hex(0xf2c46a);
                if side {
                    self.ell(-0.4, 6.2, 1.1, 1.1, gold);
                } else {
                    self.ell(-13.6, 6.0, 1.1, 1.1, gold);
                    self.ell(13.6, 6.0, 1.1, 1.1, gold);
                }
            }
            Accessory::FlatCap => {
                let (t, b) = (tint_light(a, 0.12), shade(a, 0.85));
                self.dome(0.0, -2.0, 15.4, -5.6, t, b);
                if side {
                    self.ell_v(11.0, -6.0, 7.0, 2.2, b, shade(b, 0.8));
                } else if !up {
                    self.ell_v(0.0, -5.2, 13.0, 2.6, b, shade(b, 0.8));
                }
            }
            Accessory::Hat => {
                let (t, b) = (tint_light(a, 0.1), shade(a, 0.85));
                self.ell_v(0.0, -6.5, 21.0, 4.0, t, b);
                self.rr(-11.0, -19.5, 22.0, 13.5, 4.5, t, b);
                self.rr(-11.0, -10.0, 22.0, 2.6, 0.8, shade(a, 0.6), shade(a, 0.6));
            }
            Accessory::Hairclip if !up => {
                let x = if side { -4.0 } else { 9.0 };
                self.rr(x - 2.2, -8.6, 4.4, 2.0, 1.0, tint_light(a, 0.2), a);
            }
            _ => {}
        }
    }
}

// ───────────────────────── тело ─────────────────────────

/// Скелет фигуры: смещения в юнитах от ступней, ось y ВВЕРХ.
struct Rig {
    o: Vec2,
    k: f32,
    s: f32,
}

impl Rig {
    fn p(&self, x: f32, y: f32) -> Vec2 {
        self.o + vec2(x * self.s, -y) * self.k
    }
    /// Прямоугольник: `x0` — левый край (до зеркала), `y_top` — верх (вверх положительно).
    fn rr(&self, x0: f32, y_top: f32, w: f32, h: f32, r: f32, top: Color, bottom: Color) {
        let a = self.p(x0, y_top);
        let b = self.p(x0 + w, y_top - h);
        rrect_v(
            a.x.min(b.x),
            a.y,
            (b.x - a.x).abs(),
            b.y - a.y,
            r * self.k,
            top,
            bottom,
        );
    }
    fn cap(&self, a: Vec2, b: Vec2, r: f32, col: Color) {
        capsule(self.p(a.x, a.y), self.p(b.x, b.y), r * self.k, col);
    }
    fn ell(&self, x: f32, y: f32, rx: f32, ry: f32, col: Color) {
        let p = self.p(x, y);
        ellipse(p.x, p.y, rx * self.k, ry * self.k, col);
    }
}

/// Фаза моргания 0 (открыты) … 1 (закрыты).
fn blink_amount(t: f32, phase: f32) -> f32 {
    let period = 3.3 + phase * 1.9;
    let x = (t + phase * 11.0).rem_euclid(period);
    if x < 0.13 { 1.0 } else { 0.0 }
}

/// Где держать «ступни» сидящего на стуле (`chair_origin` — левый верх тайла стула),
/// чтобы y-сортировка с этим стулом была верной. `facing` — куда смотрит стул/гость.
pub fn sit_anchor(chair_origin: Vec2, facing: Facing) -> Vec2 {
    let c = chair_origin + vec2(TILE / 2.0, TILE / 2.0);
    match facing {
        Facing::Down => c + vec2(0.0, 0.28 * TILE),
        Facing::Up => c + vec2(0.0, 0.0),
        // тело над центром сиденья, ноги вытянуты вперёд (к столу)
        Facing::Left | Facing::Right => c + vec2(0.0, 0.25 * TILE),
    }
}

/// Ключ y-сортировки персонажа. Для сидящих учтён стул (см. `sit_anchor`).
pub fn person_sort_key(pose: &PersonPose) -> f32 {
    match (pose.pose, pose.facing) {
        (Pose::Sit, Facing::Up) => pose.feet.y - 0.3 * TILE,
        (Pose::Sit, _) => pose.feet.y + 0.3 * TILE,
        _ => pose.feet.y,
    }
}

/// Точка над головой для `draw_emote` (мировые пиксели).
pub fn emote_anchor(look: &PersonLook, pose: &PersonPose) -> Vec2 {
    let k = look.height;
    let top = match (pose.pose, pose.facing) {
        (Pose::Sit, Facing::Down) => 92.0,
        (Pose::Sit, _) => 80.0,
        _ => 86.0,
    };
    pose.feet - vec2(0.0, top * k)
}

/// Нарисовать персонажа (нейтрально-приветливое лицо).
pub fn draw_person(look: &PersonLook, pose: &PersonPose) {
    draw_person_ex(look, pose, Expression::Neutral);
}

/// Персонаж с выбранным выражением лица (для катсцен).
pub fn draw_person_ex(look: &PersonLook, pose: &PersonPose, expr: Expression) {
    let l = toned(look);
    if l.bot {
        draw_bot_world(&l, pose);
        return;
    }
    let s = if pose.facing == Facing::Left {
        -1.0
    } else {
        1.0
    };
    let r = Rig {
        o: pose.feet,
        k: l.height,
        s,
    };
    let t = pose.t;
    let side = matches!(pose.facing, Facing::Left | Facing::Right);
    let up = pose.facing == Facing::Up;
    let moving = matches!(pose.pose, Pose::Walk | Pose::Carry);
    let ph = (t + l.phase * 3.0) * PI * 2.0 * 1.7;
    let bob = if moving { ph.sin().abs() * 1.6 } else { 0.0 };
    let breath = ((t + l.phase * 7.0) * 2.1).sin() * 0.55;
    let sitting = pose.pose == Pose::Sit;

    // Сидя лицом к нам бёдра идут к камере: таз выше ступней на высоту
    // сиденья + глубину бедра (косая проекция), колени — ниже таза.
    let hip_y = match (sitting, pose.facing) {
        (true, Facing::Up) => 18.0,
        (true, Facing::Down) => 30.0,
        (true, _) => 22.0,
        _ => 24.0,
    } + bob;
    let sh_y = hip_y + 25.0 + breath * 0.5 - if l.elder { 1.0 } else { 0.0 };
    let head_y = sh_y + 15.0 + breath * 0.3;
    let head_x = if side && l.elder { 1.5 } else { 0.0 };

    // тень
    let sp = pose.feet;
    let (srx, sry) = if sitting { (17.0, 6.0) } else { (14.0, 5.0) };
    soft_shadow(sp.x, sp.y - 1.0, srx * r.k, sry * r.k, 0.38);

    let top_t = tint_light(l.top, 0.06);
    let top_b = shade(l.top, 0.8);
    let rim_top = mix(l.top, c::LAVENDER_RIM, 0.45);
    let (apron_a, apron_b) = apron_colors(&l);

    let head = Head {
        c: r.p(head_x, head_y),
        u: r.k,
        s,
        facing: pose.facing,
        l: &l,
    };

    // ── длинные волосы за спиной (лицом к нам / профиль)
    if !up && l.hair_style == HairStyle::Long {
        let hp = r.p(head_x - if side { 4.0 } else { 0.0 }, head_y);
        let w = if side { 22.0 } else { 30.0 };
        rrect_v(
            hp.x - w * r.k / 2.0,
            hp.y,
            w * r.k,
            30.0 * r.k,
            9.0 * r.k,
            l.hair,
            shade(l.hair, 0.8),
        );
    }

    // ── руки: позиции кистей
    let walk_sw = if moving { ph.sin() } else { 0.0 };
    let work = pose.pose == Pose::Work;
    let carry = pose.pose == Pose::Carry;
    let wk = (t * 6.0 + l.phase * 5.0).sin();
    let wk2 = (t * 6.0 + l.phase * 5.0).cos();

    // поднос (за телом, если спиной)
    let tray_y = hip_y + 12.0;
    if carry && up {
        draw_tray(r.p(0.0, tray_y + 3.0), r.k, t, 15.0);
    }

    // дальняя рука в профиль / руки за телом со спины при работе
    let sleeve_long = !matches!(l.top_style, TopStyle::Tee | TopStyle::Scrubs);
    let arm = |a: Vec2, h: Vec2, dark: f32| {
        let mid = a + (h - a) * 0.55;
        let sleeve = shade(l.top, dark);
        r.cap(a, mid, 3.7, sleeve);
        let fore = if sleeve_long {
            sleeve
        } else {
            shade(l.skin, dark)
        };
        r.cap(mid, h, 3.3, fore);
        r.ell(h.x, h.y, 3.0, 3.0, shade(l.skin, dark));
    };

    if side {
        let a = vec2(-0.5, sh_y - 2.5);
        let h = if carry || work {
            vec2(9.0, hip_y + 10.0)
        } else if sitting {
            vec2(7.0, hip_y + 5.0)
        } else {
            vec2(-walk_sw * 6.0, hip_y + 1.0)
        };
        arm(a, h, 0.72);
    }
    if up && (carry || work) {
        arm(vec2(-12.0, sh_y - 2.0), vec2(-8.0, hip_y + 12.0), 0.9);
        arm(vec2(12.0, sh_y - 2.0), vec2(8.0, hip_y + 12.0), 0.9);
    }

    // ── ноги
    let leg_c = l.bottom;
    let leg_d = shade(l.bottom, 0.78);
    if sitting {
        if side {
            // бедро горизонтально вперёд, голень вниз
            let hip = vec2(-1.0, hip_y - 2.0);
            let knee = vec2(12.0, hip_y - 3.0);
            let foot = vec2(13.0, 2.5);
            r.cap(hip, knee, 5.0, leg_d);
            r.cap(knee, foot, 4.2, leg_d);
            r.ell(foot.x + 2.0, 1.5, 4.6, 2.3, SHOE);
            r.cap(hip + vec2(0.0, 1.5), knee + vec2(1.0, 1.0), 5.0, leg_c);
            r.cap(knee + vec2(1.0, 1.0), foot + vec2(-1.0, 0.0), 4.2, leg_c);
            r.ell(foot.x + 1.0, 1.2, 4.8, 2.4, SHOE);
        } else if !up {
            for x in [-5.2_f32, 5.2] {
                r.cap(
                    vec2(x * 1.05, 18.0),
                    vec2(x * 1.05, 3.0),
                    4.1,
                    shade(l.bottom, 0.62),
                );
                r.ell(x * 1.05, 1.6, 4.6, 2.6, SHOE);
            }
        }
    } else if side {
        for (i, sgn) in [(0, -1.0_f32), (1, 1.0)] {
            let sw = walk_sw * sgn;
            let lift = if moving {
                (ph.cos() * sgn).max(0.0) * 2.5
            } else {
                0.0
            };
            let foot = vec2(sw * 7.0, 2.0 + lift);
            let col = if i == 0 { leg_d } else { leg_c };
            r.cap(vec2(0.0, hip_y - 2.0), foot, 4.3, col);
            r.ell(foot.x + 2.2, foot.y - 0.8, 4.8, 2.3, SHOE);
        }
    } else {
        for (x, sgn) in [(-5.0_f32, 1.0_f32), (5.0, -1.0)] {
            let lift = if moving {
                (ph.sin() * sgn).max(0.0) * 3.0
            } else {
                0.0
            };
            let foot = vec2(x, 2.5 + lift);
            r.cap(vec2(x, hip_y - 2.0), foot, 4.3, leg_c);
            r.cap(
                vec2(x + x.signum() * 1.6, hip_y - 4.0),
                foot + vec2(x.signum() * 1.6, 0.0),
                1.2,
                leg_d,
            );
            if !up {
                r.ell(x, foot.y - 1.0, 4.7, 2.7, SHOE);
                r.ell(
                    x - 1.2,
                    foot.y - 0.2,
                    1.6,
                    0.9,
                    alpha(c::LAVENDER_RIM, 0.35),
                );
            } else {
                r.ell(x, foot.y - 0.5, 4.5, 2.2, SHOE);
            }
        }
    }

    // ── капюшон за шеей
    if l.top_style == TopStyle::Hoodie && !up {
        let p = r.p(if side { -6.0 } else { 0.0 }, sh_y + 1.0);
        ellipse_v(
            p.x,
            p.y,
            10.0 * r.k,
            4.6 * r.k,
            shade(l.top, 0.75),
            shade(l.top, 0.6),
        );
    }

    // ── туловище
    let tw = if side { 17.0 } else { 25.0 };
    let tx0 = if side { -8.0 } else { -12.5 };
    let coat_ext = if l.top_style == TopStyle::Coat && !sitting {
        12.0
    } else {
        0.0
    };
    let t_top = sh_y + 1.5;
    let t_h = t_top - hip_y + 3.0 + coat_ext;
    // rim light
    let rp = r.p(tx0 - 1.0, t_top + 1.0);
    let rb = r.p(tx0 + tw - 1.0, t_top + 1.0 - t_h);
    rrect(
        rp.x.min(rb.x),
        rp.y,
        (rb.x - rp.x).abs(),
        rb.y - rp.y,
        7.0 * r.k,
        rim_top,
    );
    r.rr(tx0, t_top, tw, t_h, 7.0, top_t, top_b);
    // пояс/низ
    if !side && coat_ext == 0.0 {
        r.rr(
            tx0 + 1.0,
            hip_y + 4.0,
            tw - 2.0,
            6.0,
            3.0,
            shade(l.bottom, 0.95),
            shade(l.bottom, 0.85),
        );
    }

    // детали фасона (лицом к камере)
    if !up && !side {
        match l.top_style {
            TopStyle::Shirt => {
                let col = tint_light(l.top, 0.35);
                poly(
                    &[
                        r.p(-5.0, sh_y + 1.5),
                        r.p(0.0, sh_y - 3.0),
                        r.p(-1.0, sh_y + 1.5),
                    ],
                    col,
                );
                poly(
                    &[
                        r.p(5.0, sh_y + 1.5),
                        r.p(1.0, sh_y + 1.5),
                        r.p(0.0, sh_y - 3.0),
                    ],
                    col,
                );
            }
            TopStyle::Cardigan => {
                r.rr(
                    -3.5,
                    sh_y,
                    7.0,
                    sh_y - hip_y - 2.0,
                    2.0,
                    c::CREAM,
                    shade(c::CREAM, 0.85),
                );
                for i in 0..3 {
                    r.ell(
                        4.6,
                        sh_y - 7.0 - i as f32 * 5.0,
                        0.9,
                        0.9,
                        shade(l.top, 0.55),
                    );
                }
            }
            TopStyle::Jacket | TopStyle::Coat => {
                let d = shade(l.top, 0.7);
                poly(
                    &[
                        r.p(-6.0, sh_y + 1.0),
                        r.p(-1.0, sh_y - 9.0),
                        r.p(-2.5, sh_y + 1.0),
                    ],
                    d,
                );
                poly(
                    &[
                        r.p(6.0, sh_y + 1.0),
                        r.p(2.5, sh_y + 1.0),
                        r.p(1.0, sh_y - 9.0),
                    ],
                    d,
                );
                r.rr(
                    -0.5,
                    sh_y - 2.0,
                    1.0,
                    t_h - 4.0,
                    0.5,
                    shade(l.top, 0.65),
                    shade(l.top, 0.6),
                );
            }
            TopStyle::Scrubs => {
                poly(
                    &[
                        r.p(-4.0, sh_y + 1.5),
                        r.p(4.0, sh_y + 1.5),
                        r.p(0.0, sh_y - 5.0),
                    ],
                    shade(l.skin, 0.92),
                );
                r.rr(
                    4.0,
                    sh_y - 9.0,
                    5.0,
                    4.0,
                    1.0,
                    shade(l.top, 0.85),
                    shade(l.top, 0.8),
                );
            }
            TopStyle::Sweater => {
                r.rr(
                    tx0 + 0.5,
                    hip_y + 4.0,
                    tw - 1.0,
                    3.0,
                    1.5,
                    shade(l.top, 0.78),
                    shade(l.top, 0.72),
                );
            }
            TopStyle::Hoodie => {
                if l.accessory == Accessory::ShiftellaLogo {
                    draw_shiftella_logo(r.p(-5.5, sh_y - 9.0), 4.2 * r.k);
                }
                r.cap(
                    vec2(-2.0, sh_y - 1.0),
                    vec2(-2.5, sh_y - 8.0),
                    0.6,
                    tint_light(l.top, 0.4),
                );
                r.cap(
                    vec2(2.0, sh_y - 1.0),
                    vec2(2.5, sh_y - 8.0),
                    0.6,
                    tint_light(l.top, 0.4),
                );
                r.rr(
                    -6.0,
                    hip_y + 9.0,
                    12.0,
                    5.0,
                    2.0,
                    shade(l.top, 0.85),
                    shade(l.top, 0.78),
                );
            }
            TopStyle::Tee => {}
        }
    }

    // ── фартук персонала
    if l.apron {
        if up {
            // лямки крест-накрест и бант на пояснице
            r.cap(vec2(-8.0, sh_y), vec2(6.0, hip_y + 6.0), 1.2, apron_a);
            r.cap(vec2(8.0, sh_y), vec2(-6.0, hip_y + 6.0), 1.2, apron_a);
            r.rr(tx0, hip_y + 6.0, tw, 2.4, 1.0, apron_a, apron_b);
            r.ell(-2.6, hip_y + 5.0, 2.6, 1.8, apron_a);
            r.ell(2.6, hip_y + 5.0, 2.6, 1.8, apron_a);
        } else {
            let (ax, aw) = if side { (1.0, 9.0) } else { (-9.5, 19.0) };
            let a_top = sh_y - 6.0;
            let a_bot = if sitting { hip_y - 2.0 } else { 9.0 + bob };
            r.rr(ax, a_top, aw, a_top - a_bot, 3.5, apron_a, apron_b);
            r.rr(
                ax - 0.5,
                hip_y + 7.0,
                aw + 1.0,
                1.6,
                0.8,
                tint_light(apron_a, 0.2),
                apron_a,
            );
            if !side {
                r.cap(vec2(-8.0, a_top), vec2(-10.5, sh_y), 1.1, apron_a);
                r.cap(vec2(8.0, a_top), vec2(10.5, sh_y), 1.1, apron_a);
                // значок Shiftella
                let b = r.p(-5.0, a_top - 4.0);
                glow(b.x, b.y, 3.2 * r.k, alpha(c::INDIGO_SOFT, 0.5));
                rrect(
                    b.x - 1.6 * r.k,
                    b.y - 1.6 * r.k,
                    3.2 * r.k,
                    3.2 * r.k,
                    1.0 * r.k,
                    c::INDIGO_SOFT,
                );
                ellipse(b.x, b.y, 0.7 * r.k, 0.7 * r.k, WHITE);
                // кармашек
                r.rr(
                    -4.5,
                    hip_y + 2.0,
                    9.0,
                    4.0,
                    1.5,
                    shade(apron_a, 0.9),
                    shade(apron_b, 0.9),
                );
            } else {
                r.cap(vec2(3.0, a_top), vec2(1.0, sh_y), 1.0, apron_a);
            }
        }
    }

    // ── шарф
    if l.accessory == Accessory::Scarf {
        let a = l.accent;
        let (x0, w) = if side { (-7.0, 15.0) } else { (-9.5, 19.0) };
        r.rr(
            x0,
            sh_y + 3.5,
            w,
            5.5,
            2.5,
            tint_light(a, 0.1),
            shade(a, 0.85),
        );
        if !up {
            let tx = if side { 3.0 } else { 4.0 };
            r.rr(tx, sh_y - 1.0, 4.0, 10.0, 1.6, a, shade(a, 0.75));
        }
    }

    // ── поднос перед собой
    if carry && !up {
        let tp = if side {
            r.p(13.0, tray_y + 1.0)
        } else {
            r.p(0.0, tray_y)
        };
        draw_tray(tp, r.k, t, if side { 10.0 } else { 13.0 });
    }

    // ── бёдра сидящего лицом к нам: идут к камере поверх низа туловища
    if sitting && pose.facing == Facing::Down {
        for x in [-5.2_f32, 5.2] {
            // тень под коленом, затем бедро: верх освещён (горизонтальная поверхность)
            r.ell(
                x * 1.05,
                15.5,
                5.2,
                2.2,
                alpha(Color::new(0.05, 0.03, 0.1, 1.0), 0.35),
            );
            r.cap(
                vec2(x, hip_y - 1.0),
                vec2(x * 1.05, 19.0),
                5.4,
                tint_light(leg_c, 0.12),
            );
            r.ell(x * 1.05, 18.0, 5.0, 2.6, shade(leg_c, 0.85));
            r.ell(
                x * 1.05 - 1.4,
                hip_y - 4.0,
                2.0,
                3.5,
                alpha(c::LAVENDER_RIM, 0.3),
            );
        }
    }

    // ── ближние руки
    if !side && !(up && (carry || work)) {
        for (x, sgn) in [(-12.5_f32, 1.0_f32), (12.5, -1.0)] {
            let a = vec2(x, sh_y - 2.0);
            let sx = x.signum();
            let h = if carry {
                vec2(sx * 9.0, tray_y - 1.0)
            } else if work {
                let w = if sx < 0.0 { wk } else { -wk };
                let w2 = if sx < 0.0 { wk2 } else { -wk2 };
                vec2(sx * 6.5 + w * 3.0, hip_y + 7.0 + w2 * 1.5)
            } else if sitting {
                // руки на коленях
                vec2(sx * 7.5, hip_y - 6.0)
            } else {
                vec2(sx * 14.0, hip_y + 1.0 + walk_sw * sgn * 2.0)
            };
            arm(a, h, 1.0);
        }
    } else if side {
        let a = vec2(1.0, sh_y - 2.5);
        let h = if carry {
            vec2(12.0, tray_y - 1.0)
        } else if work {
            vec2(11.0 + wk * 2.0, hip_y + 9.0 + wk2 * 1.5)
        } else if sitting {
            vec2(9.0, hip_y + 4.0)
        } else {
            vec2(walk_sw * 7.0, hip_y + 1.0)
        };
        arm(a, h, 1.0);
    }

    // ── шея и голова
    let np = r.p(head_x, sh_y + 4.0);
    rrect_v(
        np.x - 3.6 * r.k,
        np.y - 3.0 * r.k,
        7.2 * r.k,
        6.0 * r.k,
        2.5 * r.k,
        shade(l.skin, 0.8),
        shade(l.skin, 0.88),
    );
    head.hair_back();
    head.head_shape();
    if !up {
        let gaze = match pose.facing {
            Facing::Down if sitting => vec2(0.0, 0.4),
            _ => Vec2::ZERO,
        };
        head.face(expr, blink_amount(t, l.phase), gaze);
    }
    head.hair_front();
    head.accessory();
}

/// Поднос с чашкой (и паром) — у официанта в руках.
fn draw_tray(p: Vec2, k: f32, t: f32, rx: f32) {
    soft_shadow(p.x, p.y + 3.0 * k, rx * k, 3.0 * k, 0.25);
    ellipse_v(
        p.x,
        p.y + 1.2 * k,
        rx * k,
        3.8 * k,
        shade(c::STEEL, 0.8),
        shade(c::STEEL_DARK, 0.9),
    );
    ellipse_v(p.x, p.y, rx * k, 3.6 * k, c::STEEL_LIGHT, c::STEEL);
    draw_cup(p + vec2(-2.0 * k, -k), k * 0.9, t, true);
}

/// Чашка на поверхности (`p` — точка донышка), опционально с паром.
pub fn draw_cup(p: Vec2, k: f32, t: f32, steaming: bool) {
    let w = 6.0 * k;
    let h = 5.0 * k;
    // ручка
    draw_circle_lines(
        p.x + w * 0.62,
        p.y - h * 0.55,
        1.6 * k,
        0.9 * k,
        shade(c::CREAM, 0.85),
    );
    rrect_v(
        p.x - w / 2.0,
        p.y - h,
        w,
        h,
        1.8 * k,
        c::CREAM,
        shade(c::CREAM, 0.78),
    );
    ellipse(p.x, p.y - h, w / 2.0, 1.3 * k, shade(c::CREAM, 0.92));
    ellipse(
        p.x,
        p.y - h + 0.2 * k,
        w / 2.0 - 0.8 * k,
        0.9 * k,
        Color::from_hex(0x6b3e26),
    );
    if steaming {
        draw_steam(p - vec2(0.0, h + 1.0 * k), t, 0.6 * k);
    }
}

/// Пар: несколько волнистых струек, поднимаются и тают. Без состояния.
pub fn draw_steam(base: Vec2, t: f32, strength: f32) {
    let seed = (base.x * 13.0 + base.y * 7.0) as i32 as u32;
    for i in 0..3 {
        let life = 1.8;
        let off = i as f32 / 3.0 * life + hash01(seed + i) * 0.3;
        let a = ((t + off) % life) / life; // 0..1 возраст
        let rise = a * 16.0 * strength.max(0.3) * 1.6;
        let wob =
            ((t * 2.3 + i as f32 * 2.1).sin() * 2.2 + (a * 6.0).sin() * 1.2) * strength.max(0.5);
        let x = base.x + (i as f32 - 1.0) * 2.0 * strength + wob;
        let y = base.y - rise;
        let fade = (a * (1.0 - a) * 4.0).clamp(0.0, 1.0);
        let r = (2.2 + a * 3.5) * strength.max(0.4) * 1.3;
        glow(x, y, r, Color::new(0.93, 0.92, 1.0, 0.33 * fade));
    }
}

// ───────────────────────── бот ─────────────────────────

fn bot_face(c0: Vec2, u: f32, expr: Expression, t: f32, phase: f32) {
    let body_t = tint_light(c::INDIGO, 0.15);
    let body_b = c::INDIGO_600;
    // антенна
    let tip = c0 + vec2(0.0, -17.0 * u + (t * 3.0).sin() * 0.6 * u);
    draw_line(
        c0.x,
        c0.y - 10.0 * u,
        tip.x,
        tip.y,
        1.4 * u,
        shade(c::INDIGO_600, 0.8),
    );
    glow(tip.x, tip.y, 5.0 * u, alpha(c::INDIGO_SOFT, 0.6));
    ellipse(
        tip.x,
        tip.y,
        2.0 * u,
        2.0 * u,
        tint_light(c::INDIGO_SOFT, 0.4),
    );
    // ушки-динамики
    ellipse_v(
        c0.x - 13.0 * u,
        c0.y,
        2.8 * u,
        4.5 * u,
        c::VIOLET_SOFT,
        c::VIOLET,
    );
    ellipse_v(
        c0.x + 13.0 * u,
        c0.y,
        2.8 * u,
        4.5 * u,
        c::VIOLET_SOFT,
        c::VIOLET,
    );
    // rim
    rrect(
        c0.x - 12.6 * u,
        c0.y - 11.4 * u,
        24.0 * u,
        22.0 * u,
        7.5 * u,
        c::LAVENDER_RIM,
    );
    rrect_v(
        c0.x - 12.0 * u,
        c0.y - 11.0 * u,
        24.0 * u,
        22.0 * u,
        7.5 * u,
        body_t,
        body_b,
    );
    // экран-лицо
    rrect_v(
        c0.x - 9.0 * u,
        c0.y - 7.0 * u,
        18.0 * u,
        13.5 * u,
        5.0 * u,
        Color::from_hex(0x1b1a2e),
        Color::from_hex(0x24203f),
    );
    let eye = tint_light(c::INDIGO_SOFT, 0.35);
    let blink = blink_amount(t, phase) > 0.5;
    for ex in [-4.2_f32, 4.2] {
        let p = c0 + vec2(ex * u, -1.2 * u);
        glow(p.x, p.y, 4.0 * u, alpha(c::INDIGO_GLOW, 0.45));
        match expr {
            Expression::Smile | Expression::Laugh => arc(
                p + vec2(0.0, 1.0 * u),
                2.4 * u,
                2.2 * u,
                PI * 1.1,
                PI * 1.9,
                1.3 * u,
                eye,
            ),
            Expression::Surprised => {
                draw_circle_lines(p.x, p.y, 2.4 * u, 1.1 * u, eye);
            }
            Expression::Sad => {
                draw_line(
                    p.x - 2.0 * u,
                    p.y - (ex.signum() * 0.8) * u,
                    p.x + 2.0 * u,
                    p.y + (ex.signum() * 0.8) * u,
                    1.3 * u,
                    eye,
                );
            }
            Expression::Tired => draw_line(
                p.x - 2.2 * u,
                p.y + 0.5 * u,
                p.x + 2.2 * u,
                p.y + 0.5 * u,
                1.3 * u,
                eye,
            ),
            _ if blink => draw_line(p.x - 2.0 * u, p.y, p.x + 2.0 * u, p.y, 1.2 * u, eye),
            _ => ellipse(p.x, p.y, 1.7 * u, 2.3 * u, eye),
        }
    }
    let m = c0 + vec2(0.0, 3.4 * u);
    match expr {
        Expression::Laugh => ellipse(m.x, m.y, 2.0 * u, 1.4 * u, eye),
        Expression::Surprised => draw_circle_lines(m.x, m.y, 1.1 * u, 0.8 * u, eye),
        Expression::Sad => arc(
            m + vec2(0.0, 1.0 * u),
            2.0 * u,
            1.0 * u,
            PI * 1.2,
            PI * 1.8,
            0.9 * u,
            eye,
        ),
        Expression::Thinking => draw_line(
            m.x - 1.0 * u,
            m.y,
            m.x + 2.2 * u,
            m.y - 0.6 * u,
            0.9 * u,
            eye,
        ),
        _ => arc(m, 2.0 * u, 1.2 * u, PI * 0.2, PI * 0.8, 0.9 * u, eye),
    }
    // щёчки
    ellipse(
        c0.x - 8.0 * u,
        c0.y + 3.0 * u,
        1.6 * u,
        1.0 * u,
        alpha(BLUSH, 0.5),
    );
    ellipse(
        c0.x + 8.0 * u,
        c0.y + 3.0 * u,
        1.6 * u,
        1.0 * u,
        alpha(BLUSH, 0.5),
    );
    // блик
    rrect(
        c0.x - 10.0 * u,
        c0.y - 10.0 * u,
        7.0 * u,
        2.0 * u,
        1.0 * u,
        alpha(WHITE, 0.35),
    );
}

fn draw_bot_world(l: &PersonLook, pose: &PersonPose) {
    let k = l.height * 1.25;
    let hover = (pose.t * 2.0 + l.phase * 6.0).sin() * 2.0;
    soft_shadow(
        pose.feet.x,
        pose.feet.y,
        10.0 * k,
        4.0 * k,
        0.3 - hover * 0.03,
    );
    let c0 = pose.feet + vec2(0.0, (-34.0 + hover) * k);
    glow(c0.x, c0.y + 14.0 * k, 9.0 * k, alpha(c::INDIGO_GLOW, 0.35));
    bot_face(c0, k * 0.9, Expression::Smile, pose.t, l.phase);
}

// ───────────────────────── портрет ─────────────────────────

/// Портрет (голова + плечи) для диалогов. `rect` — в текущей камере (обычно UI),
/// фон прозрачный: рамку рисует UI. Дыхание и моргание от `t`.
pub fn draw_portrait(look: &PersonLook, expression: Expression, rect: Rect, t: f32) {
    let l = toned(look);
    let breath = ((t + l.phase * 7.0) * 1.9).sin();
    let hh = rect.h;
    if l.bot {
        let u = hh / 46.0;
        let c0 = vec2(rect.center().x, rect.y + hh * 0.52 + breath * 0.008 * hh);
        glow(c0.x, c0.y + 16.0 * u, 12.0 * u, alpha(c::INDIGO_GLOW, 0.35));
        bot_face(c0, u, expression, t, l.phase);
        return;
    }
    let r = hh * 0.25;
    let u = r / 14.0;
    let cx = rect.center().x;
    let head_c = vec2(cx, rect.y + hh * 0.41 + breath * 0.006 * hh);
    let sh_top = rect.y + hh * 0.72 + breath * 0.01 * hh;
    let bottom = rect.y + hh;

    // длинные волосы за плечами
    if l.hair_style == HairStyle::Long {
        rrect_v(
            head_c.x - 16.0 * u,
            head_c.y - 6.0 * u,
            32.0 * u,
            34.0 * u,
            9.0 * u,
            l.hair,
            shade(l.hair, 0.78),
        );
    }

    // плечи (бюст) c rim light справа-сверху
    let bw = 3.7 * r;
    let rim_top = mix(l.top, c::LAVENDER_RIM, 0.5);
    // плечи: скруглён только верх, низ ровно обрезан краем портрета
    let rad = r * 0.9;
    let (ct, cb) = (tint_light(l.top, 0.06), shade(l.top, 0.72));
    rrect_top(
        cx - bw / 2.0 + 1.2 * u,
        sh_top - 1.2 * u,
        bw,
        bottom - sh_top + 1.2 * u,
        rad,
        rim_top,
        alpha(rim_top, 0.3),
    );
    rrect_top(cx - bw / 2.0, sh_top, bw, bottom - sh_top, rad, ct, cb);
    if l.top_style == TopStyle::Hoodie {
        ellipse_v(
            cx,
            sh_top + 1.0 * u,
            13.0 * u,
            4.6 * u,
            shade(l.top, 0.7),
            shade(l.top, 0.6),
        );
    }
    // шея
    rrect_v(
        cx - 4.2 * u,
        head_c.y + 8.0 * u,
        8.4 * u,
        sh_top - head_c.y - 6.0 * u,
        3.0 * u,
        shade(l.skin, 0.78),
        shade(l.skin, 0.86),
    );
    // вырез / воротник
    let neck_y = sh_top + 0.5 * u;
    match l.top_style {
        TopStyle::Shirt => {
            let col = tint_light(l.top, 0.35);
            poly(
                &[
                    vec2(cx - 7.0 * u, neck_y - 1.0 * u),
                    vec2(cx, neck_y + 5.0 * u),
                    vec2(cx - 1.0 * u, neck_y - 1.0 * u),
                ],
                col,
            );
            poly(
                &[
                    vec2(cx + 7.0 * u, neck_y - 1.0 * u),
                    vec2(cx + 1.0 * u, neck_y - 1.0 * u),
                    vec2(cx, neck_y + 5.0 * u),
                ],
                col,
            );
        }
        TopStyle::Scrubs => {
            poly(
                &[
                    vec2(cx - 5.0 * u, neck_y - 0.5 * u),
                    vec2(cx + 5.0 * u, neck_y - 0.5 * u),
                    vec2(cx, neck_y + 7.0 * u),
                ],
                shade(l.skin, 0.9),
            );
        }
        TopStyle::Cardigan => {
            rrect_v(
                cx - 4.5 * u,
                neck_y,
                9.0 * u,
                bottom - neck_y,
                2.0 * u,
                c::CREAM,
                shade(c::CREAM, 0.8),
            );
            for i in 0..2 {
                ellipse(
                    cx + 6.0 * u,
                    neck_y + (5.0 + i as f32 * 6.0) * u,
                    1.0 * u,
                    1.0 * u,
                    shade(l.top, 0.5),
                );
            }
        }
        TopStyle::Jacket | TopStyle::Coat => {
            let d = shade(l.top, 0.68);
            poly(
                &[
                    vec2(cx - 9.0 * u, neck_y - 0.5 * u),
                    vec2(cx - 1.5 * u, bottom),
                    vec2(cx - 4.0 * u, bottom),
                    vec2(cx - 11.0 * u, neck_y + 4.0 * u),
                ],
                d,
            );
            poly(
                &[
                    vec2(cx + 9.0 * u, neck_y - 0.5 * u),
                    vec2(cx + 11.0 * u, neck_y + 4.0 * u),
                    vec2(cx + 4.0 * u, bottom),
                    vec2(cx + 1.5 * u, bottom),
                ],
                d,
            );
            ellipse(cx, neck_y + 1.5 * u, 4.5 * u, 2.2 * u, shade(l.skin, 0.85));
        }
        _ => {
            ellipse(cx, neck_y + 0.6 * u, 5.0 * u, 2.4 * u, shade(l.skin, 0.84));
        }
    }
    if l.accessory == Accessory::ShiftellaLogo {
        draw_shiftella_logo(vec2(cx - 9.0 * u, sh_top + 11.0 * u), 5.5 * u);
    }
    // фартук
    if l.apron {
        let (aa, ab) = apron_colors(&l);
        let top = sh_top + 9.0 * u;
        rrect_v(cx - 10.0 * u, top, 20.0 * u, bottom - top, 3.0 * u, aa, ab);
        capsule(
            vec2(cx - 9.0 * u, top + 1.0 * u),
            vec2(cx - 12.0 * u, sh_top + 0.5 * u),
            1.4 * u,
            aa,
        );
        capsule(
            vec2(cx + 9.0 * u, top + 1.0 * u),
            vec2(cx + 12.0 * u, sh_top + 0.5 * u),
            1.4 * u,
            aa,
        );
        let b = vec2(cx - 5.0 * u, top + 4.5 * u);
        glow(b.x, b.y, 4.0 * u, alpha(c::INDIGO_SOFT, 0.5));
        rrect(
            b.x - 2.0 * u,
            b.y - 2.0 * u,
            4.0 * u,
            4.0 * u,
            1.2 * u,
            c::INDIGO_SOFT,
        );
        ellipse(b.x, b.y, 0.9 * u, 0.9 * u, WHITE);
    }
    if l.accessory == Accessory::Scarf {
        let a = l.accent;
        rrect_v(
            cx - 12.0 * u,
            sh_top - 3.0 * u,
            24.0 * u,
            7.0 * u,
            3.0 * u,
            tint_light(a, 0.1),
            shade(a, 0.8),
        );
        rrect_v(
            cx + 3.0 * u,
            sh_top + 2.0 * u,
            5.0 * u,
            13.0 * u,
            2.0 * u,
            a,
            shade(a, 0.7),
        );
    }

    let head = Head {
        c: head_c,
        u,
        s: 1.0,
        facing: Facing::Down,
        l: &l,
    };
    head.hair_back();
    // rim light справа-сверху для портрета
    let pr = head_c + vec2(1.0, -1.0) * u;
    ellipse(
        pr.x,
        pr.y,
        14.0 * u,
        13.8 * u,
        mix(l.skin, c::LAVENDER_RIM, 0.6),
    );
    head.head_shape();
    let gaze = match expression {
        Expression::Thinking => vec2(1.4, -1.0),
        Expression::Sad | Expression::Tired => vec2(0.0, 0.6),
        _ => vec2(((t * 0.4 + l.phase * 9.0).sin() * 0.6).round() * 0.5, 0.0),
    };
    head.face(expression, blink_amount(t, l.phase), gaze);
    head.hair_front();
    head.accessory();
}

// ───────────────────────── Shiftella-логотип и ноутбук ─────────────────────────

/// Логотип Shiftella: индиговый скруглённый квадрат с белой «S»-волной.
/// `c` — центр, `size` — сторона. Для нашивки на худи, стикера на ноутбуке, вывесок.
pub fn draw_shiftella_logo(c0: Vec2, size: f32) {
    let h = size / 2.0;
    glow(c0.x, c0.y, size * 1.1, alpha(c::INDIGO_GLOW, 0.35));
    // the crisp pre-rasterised logo (Inter "S"); the strokes below are the headless fallback
    if super::brand::draw_app_logo(c0, size) {
        return;
    }
    rrect_v(
        c0.x - h,
        c0.y - h,
        size,
        size,
        size * 0.28,
        c::INDIGO_GLOW,
        c::INDIGO_600,
    );
    let w = (size * 0.11).max(0.6);
    let r = size * 0.2;
    arc(
        c0 + vec2(0.0, -r * 0.95),
        r,
        r * 0.95,
        PI * 0.45,
        PI * 1.95,
        w,
        WHITE,
    );
    arc(
        c0 + vec2(0.0, r * 0.95),
        r,
        r * 0.95,
        PI * 1.45,
        PI * 2.95,
        w,
        WHITE,
    );
}

/// Нейтральный стикер: кремовый полумесяц на тёмном кружке (радиус `r`).
fn draw_moon_sticker(c0: Vec2, r: f32) {
    ellipse(c0.x, c0.y, r, r, Color::from_hex(0x2a2550));
    ellipse(c0.x, c0.y, r * 0.7, r * 0.7, c::CREAM);
    ellipse(
        c0.x + r * 0.32,
        c0.y - r * 0.22,
        r * 0.58,
        r * 0.58,
        Color::from_hex(0x2a2550),
    );
}

/// Ноутбук на столе. `p` — точка на поверхности стола под центром ноутбука (мировые px),
/// `user_facing` — куда смотрит владелец (экран повёрнут к нему). Со спины крышки виден
/// маленький стикер-луна, с лицевой — светящийся экран. Для света добавьте `Light::phone(p)`.
pub fn draw_laptop(p: Vec2, user_facing: Facing, t: f32) {
    let shell = Color::from_hex(0x3a3d52);
    let shell_l = Color::from_hex(0x5a5f7a);
    let glow_on = 0.85 + (t * 0.7).sin() * 0.05;
    soft_shadow(p.x + 1.0, p.y + 1.0, 13.0, 5.0, 0.35);
    match user_facing {
        Facing::Up | Facing::Down => {
            // база
            rrect_v(p.x - 12.0, p.y - 4.0, 24.0, 7.0, 2.0, shell_l, shell);
            let (lx, ly, lw, lh) = (p.x - 11.0, p.y - 20.0, 22.0, 16.0);
            if user_facing == Facing::Up {
                // владелец к нам спиной → видим экран
                rrect_v(lx, ly, lw, lh, 2.0, shell, shade(shell, 0.8));
                rrect_v(
                    lx + 1.5,
                    ly + 1.5,
                    lw - 3.0,
                    lh - 3.0,
                    1.0,
                    alpha(c::INDIGO_SOFT, glow_on),
                    alpha(c::INDIGO_600, glow_on),
                );
                for i in 0..3 {
                    let y = ly + 4.5 + i as f32 * 3.2;
                    draw_line(
                        lx + 4.0,
                        y,
                        lx + 4.0 + 6.0 + (i % 2) as f32 * 6.0,
                        y,
                        0.9,
                        alpha(WHITE, 0.7),
                    );
                }
                glow(p.x, ly + lh / 2.0, 18.0, alpha(c::INDIGO_GLOW, 0.35));
            } else {
                // владелец лицом к нам → крышка спиной, на ней стикер
                rrect_v(lx, ly, lw, lh, 2.0, shell_l, shell);
                draw_moon_sticker(vec2(p.x + 4.0, ly + lh / 2.0 - 2.0), 4.0);
                glow(p.x, p.y - 2.0, 14.0, alpha(c::INDIGO_GLOW, 0.25));
            }
        }
        Facing::Left | Facing::Right => {
            // в профиль: база и наклонная крышка, экран смотрит на владельца
            // петля и крышка — на дальней от владельца стороне
            let s = if user_facing == Facing::Right {
                1.0
            } else {
                -1.0
            };
            rrect_v(p.x - 11.0, p.y - 4.0, 22.0, 6.0, 2.0, shell_l, shell);
            let hinge = vec2(p.x + s * 9.0, p.y - 3.0);
            let top = hinge + vec2(s * 4.0, -16.0);
            // свет экрана на базу и в сторону владельца
            glow(
                p.x - s * 4.0,
                p.y - 8.0,
                16.0,
                alpha(c::INDIGO_GLOW, 0.35 * glow_on),
            );
            quad(
                [
                    hinge + vec2(0.0, 4.0),
                    top + vec2(0.0, 3.0),
                    top + vec2(-s * 3.0, 4.0),
                    hinge + vec2(-s * 3.0, 4.5),
                ],
                [shell, shell_l, shell_l, shell],
            );
            draw_line(hinge.x, hinge.y, top.x, top.y, 3.0, shell_l);
            draw_line(
                hinge.x - s * 1.6,
                hinge.y,
                top.x - s * 1.6,
                top.y + 0.5,
                1.2,
                alpha(c::INDIGO_SOFT, 0.9 * glow_on),
            );
            draw_moon_sticker(hinge.lerp(top, 0.55) + vec2(s * 1.4, 0.0), 2.6);
        }
    }
}

// ───────────────────────── эмоции и выделение ─────────────────────────

/// Пузырь-эмоция над головой. `anchor` — см. `emote_anchor`.
pub fn draw_emote(anchor: Vec2, emote: Emote) {
    draw_emote_ex(anchor, emote, 1.0, 0.0);
}

/// Эмоция с анимацией появления: `age` — секунд с момента показа, `t` — общее время.
pub fn draw_emote_ex(anchor: Vec2, emote: Emote, age: f32, t: f32) {
    let a = (age / 0.28).clamp(0.0, 1.0);
    // easeOutBack — «выпрыгивает»
    let c1 = 1.70158;
    let pop = 1.0 + (c1 + 1.0) * (a - 1.0).powi(3) + c1 * (a - 1.0).powi(2);
    let k = pop.max(0.0);
    if k <= 0.01 {
        return;
    }
    let bob = (t * 2.4).sin() * 1.2;
    let c0 = anchor + vec2(0.0, -12.0 * k + bob);
    let (w, h) = (22.0 * k, 19.0 * k);
    soft_shadow(c0.x, c0.y + 3.0 * k, 14.0 * k, 11.0 * k, 0.35);
    let bg = Color::from_hex(0xf6f3ff);
    // хвостик
    poly(
        &[
            c0 + vec2(-3.5 * k, h * 0.4),
            c0 + vec2(3.5 * k, h * 0.4),
            c0 + vec2(0.0, h * 0.5 + 5.0 * k),
        ],
        shade(bg, 0.93),
    );
    rrect_v(
        c0.x - w / 2.0,
        c0.y - h / 2.0,
        w,
        h,
        7.0 * k,
        bg,
        Color::from_hex(0xe3defa),
    );
    let u = k;
    match emote {
        Emote::Coffee => {
            draw_cup(c0 + vec2(-u, 5.0 * u), 1.25 * u, t, false);
            for i in 0..2 {
                let x = c0.x + (-3.0 + i as f32 * 3.0) * u;
                let w = ((t * 3.0 + i as f32).sin()) * 1.0 * u;
                draw_line(
                    x,
                    c0.y - 2.5 * u,
                    x + w,
                    c0.y - 6.5 * u,
                    1.1 * u,
                    alpha(c::INDIGO_GLOW, 0.8),
                );
            }
        }
        Emote::Heart => {
            let beat = 1.0 + ((t * 5.0).sin().max(0.0)).powi(4) * 0.12;
            let v = u * beat;
            ellipse(c0.x - 2.7 * v, c0.y - 1.5 * v, 3.4 * v, 3.4 * v, c::ROSE);
            ellipse(c0.x + 2.7 * v, c0.y - 1.5 * v, 3.4 * v, 3.4 * v, c::ROSE);
            poly(
                &[
                    c0 + vec2(-5.9 * v, -0.4 * v),
                    c0 + vec2(5.9 * v, -0.4 * v),
                    c0 + vec2(0.0, 6.0 * v),
                ],
                c::ROSE,
            );
            ellipse(
                c0.x - 3.4 * v,
                c0.y - 2.6 * v,
                1.2 * v,
                0.9 * v,
                alpha(WHITE, 0.6),
            );
        }
        Emote::Zzz => {
            let col = c::INDIGO_600;
            for (i, (dx, dy, s)) in [(-4.5, 3.0, 3.2), (1.0, -1.0, 2.5), (5.0, -4.5, 1.8)]
                .iter()
                .enumerate()
            {
                let f = ((t * 1.5 - i as f32 * 0.3).sin() * 0.5 + 0.5) * 0.4 + 0.6;
                let p = c0 + vec2(*dx, *dy) * u;
                let s = s * u;
                let col = alpha(col, f);
                draw_line(p.x - s, p.y - s, p.x + s, p.y - s, 1.2 * u, col);
                draw_line(p.x + s, p.y - s, p.x - s, p.y + s, 1.2 * u, col);
                draw_line(p.x - s, p.y + s, p.x + s, p.y + s, 1.2 * u, col);
            }
        }
        Emote::Angry => {
            let col = Color::from_hex(0xe04a5f);
            for (sx, sy) in [(-1.0_f32, -1.0_f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                let p = c0 + vec2(sx * 2.6, sy * 2.6) * u;
                // уголки «вены» выгнуты наружу
                let a0 = (sy).atan2(sx) + PI - 0.9;
                arc(
                    p + vec2(sx, sy) * 2.4 * u,
                    2.6 * u,
                    2.6 * u,
                    a0,
                    a0 + 1.8,
                    1.4 * u,
                    col,
                );
            }
        }
        Emote::Note => {
            let p = c0 + vec2(-5.0, -6.0) * u;
            rrect(
                p.x,
                p.y,
                10.0 * u,
                12.0 * u,
                1.2 * u,
                Color::from_hex(0xfff3c9),
            );
            poly(
                &[
                    p + vec2(7.0, 0.0) * u,
                    p + vec2(10.0, 0.0) * u,
                    p + vec2(10.0, 3.0) * u,
                ],
                Color::from_hex(0xe5d39a),
            );
            for i in 0..3 {
                let y = p.y + (4.0 + i as f32 * 2.6) * u;
                draw_line(
                    p.x + 2.0 * u,
                    y,
                    p.x + (8.0 - i as f32 * 1.5) * u,
                    y,
                    0.8 * u,
                    c::INDIGO,
                );
            }
        }
        Emote::Music => {
            let col = c::VIOLET;
            let sway = (t * 3.0).sin() * 0.8 * u;
            let a = c0 + vec2(-3.5 * u + sway, 4.0 * u);
            let b = c0 + vec2(3.5 * u + sway, 2.5 * u);
            ellipse(a.x, a.y, 2.4 * u, 1.8 * u, col);
            ellipse(b.x, b.y, 2.4 * u, 1.8 * u, col);
            draw_line(
                a.x + 2.0 * u,
                a.y,
                a.x + 2.0 * u,
                a.y - 9.0 * u,
                1.1 * u,
                col,
            );
            draw_line(
                b.x + 2.0 * u,
                b.y,
                b.x + 2.0 * u,
                b.y - 9.0 * u,
                1.1 * u,
                col,
            );
            draw_line(
                a.x + 2.0 * u,
                a.y - 9.0 * u,
                b.x + 2.0 * u,
                b.y - 9.0 * u,
                2.2 * u,
                col,
            );
        }
        Emote::Exclamation => {
            let col = c::AMBER_DEEP;
            capsule(
                c0 + vec2(0.0, -6.0 * u),
                c0 + vec2(0.0, 1.5 * u),
                1.9 * u,
                col,
            );
            ellipse(c0.x, c0.y + 5.5 * u, 2.0 * u, 2.0 * u, col);
        }
        Emote::Money => {
            let gold = Color::from_hex(0xf2b84b);
            let p = c0 + vec2(1.5 * u, 0.5 * u);
            ellipse(
                p.x - 3.0 * u,
                p.y + 2.0 * u,
                5.0 * u,
                5.0 * u,
                shade(gold, 0.8),
            );
            ellipse_v(
                p.x,
                p.y,
                5.5 * u,
                5.5 * u,
                tint_light(gold, 0.2),
                shade(gold, 0.85),
            );
            draw_circle_lines(p.x, p.y, 3.8 * u, 0.8 * u, shade(gold, 0.7));
            draw_line(
                p.x,
                p.y - 2.4 * u,
                p.x,
                p.y + 2.4 * u,
                1.0 * u,
                shade(gold, 0.6),
            );
            ellipse(
                p.x - 2.0 * u,
                p.y - 2.5 * u,
                1.2 * u,
                0.8 * u,
                alpha(WHITE, 0.7),
            );
        }
    }
}

/// Кольцо выделения на полу. Рисуйте ПОД персонажем: в очереди с ключом
/// `person_sort_key(pose) - 0.01` (или до `flush`, если персонаж — на открытом полу).
pub fn draw_selection(feet: Vec2) {
    let t = get_time() as f32;
    let (rx, ry) = (24.0, 10.0);
    radial(
        feet.x,
        feet.y,
        rx,
        ry,
        alpha(c::INDIGO_GLOW, 0.32),
        alpha(c::INDIGO_GLOW, 0.0),
    );
    let n = 18;
    for i in 0..n {
        let a0 = i as f32 / n as f32 * PI * 2.0 + t * 0.6;
        arc(
            feet,
            rx,
            ry,
            a0,
            a0 + PI * 2.0 / n as f32 * 0.55,
            2.0,
            c::INDIGO_SOFT,
        );
    }
}

/// Мягкий «прожектор» на актёра в катсцене: свечение пола + световой столб.
/// Рисуйте под персонажем (как `draw_selection`); для света ночью добавьте
/// `Light::actor_spot(feet)` в `Lighting`.
pub fn draw_actor_highlight(feet: Vec2, t: f32) {
    let pulse = 0.85 + (t * 1.6).sin() * 0.15;
    radial(
        feet.x,
        feet.y,
        46.0,
        18.0,
        alpha(c::LAVENDER_RIM, 0.38 * pulse),
        alpha(c::LAVENDER_RIM, 0.0),
    );
    // столб света сверху (очень прозрачный)
    quad(
        [
            feet + vec2(-18.0, -110.0),
            feet + vec2(18.0, -110.0),
            feet + vec2(30.0, 0.0),
            feet + vec2(-30.0, 0.0),
        ],
        [
            alpha(c::LAVENDER_RIM, 0.0),
            alpha(c::LAVENDER_RIM, 0.0),
            alpha(c::LAVENDER_RIM, 0.12 * pulse),
            alpha(c::LAVENDER_RIM, 0.12 * pulse),
        ],
    );
    arc(
        feet,
        30.0,
        12.0,
        0.0,
        PI * 2.0,
        1.2,
        alpha(c::LAVENDER_RIM, 0.5 * pulse),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sitting_sorts_against_chair() {
        let chair = vec2(3.0 * TILE, 2.0 * TILE);
        let chair_key = chair.y + TILE;
        for f in [Facing::Down, Facing::Left, Facing::Right] {
            let p = PersonPose {
                feet: sit_anchor(chair, f),
                facing: f,
                pose: Pose::Sit,
                t: 0.0,
            };
            assert!(person_sort_key(&p) > chair_key, "{f:?} draws after chair");
        }
        let p = PersonPose {
            feet: sit_anchor(chair, Facing::Up),
            facing: Facing::Up,
            pose: Pose::Sit,
            t: 0.0,
        };
        assert!(
            person_sort_key(&p) < chair_key,
            "backrest covers a guest seen from behind"
        );
        assert!(
            person_sort_key(&p) > chair.y,
            "but the guest is in front of the table above"
        );
    }

    #[test]
    fn blink_is_brief() {
        let closed = (0..1000)
            .filter(|i| blink_amount(*i as f32 * 0.01, 0.3) > 0.5)
            .count();
        assert!(closed > 0 && closed < 80, "closed frames: {closed}");
    }
}
