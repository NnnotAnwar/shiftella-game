//! The street behind the café walls: wet cobbles with puddles, a streetlamp, the road,
//! the platform edge, rails, a train, fog and city lights. Plus rain,
//! drops on the glass and the holographic sign.
//!
//! All stateless: the picture is a function of time `t` (handy for screenshots).

use std::cell::OnceCell;

use macroquad::prelude::*;

use super::colors as c;
use super::draw::*;
use super::furniture::holo_flicker;
use super::light::Light;
use crate::map::TILE;

/// Street bands from the bottom (by the café wall) upwards (into the distance).
#[derive(Debug, Clone, Copy)]
pub(super) struct Bands {
    pub sidewalk: (f32, f32),
    pub street: (f32, f32),
    pub platform: (f32, f32),
    pub tracks: (f32, f32),
    pub far: (f32, f32),
}

pub(super) fn bands(r: Rect) -> Bands {
    let b = r.y + r.h;
    let sw = (b - 1.5 * TILE, b);
    let st = (sw.0 - 1.7 * TILE, sw.0);
    let pl = (st.0 - 1.1 * TILE, st.0);
    let tr = (pl.0 - 1.2 * TILE, pl.0);
    let far = (r.y, tr.0);
    Bands {
        sidewalk: sw,
        street: st,
        platform: pl,
        tracks: tr,
        far,
    }
}

/// One house silhouette on the far side of the tracks.
#[derive(Debug, Clone, Copy)]
pub(super) struct FarHouse {
    pub x: f32,
    pub w: f32,
    /// The top of the walls (a pointed roof rises 18 px above it).
    pub top: f32,
    pub pointed: bool,
    pub i: u32,
}

/// The far houses (deterministic from the rect).
pub(super) fn far_houses(r: Rect) -> Vec<FarHouse> {
    let b = bands(r);
    let mut v = Vec::new();
    let mut x = r.x - 20.0;
    let mut i = 0u32;
    while x < r.x + r.w {
        let w = 60.0 + hash01(i * 3 + 1) * 90.0;
        let h = 30.0 + hash01(i * 3 + 2) * (b.far.1 - b.far.0 - 30.0).max(10.0) * 0.8;
        v.push(FarHouse {
            x,
            w,
            top: b.far.1 - h,
            pointed: hash01(i * 7) > 0.5,
            i,
        });
        x += w + 6.0 + hash01(i * 11) * 20.0;
        i += 1;
    }
    v
}

/// x coordinates of the streetlamps along the pavement.
pub(super) fn lamp_xs(r: Rect) -> impl Iterator<Item = f32> {
    let step = 7.0 * TILE;
    let first = (r.x / step).ceil() * step + 2.5 * TILE;
    (0..)
        .map(move |i| first + i as f32 * step)
        .take_while(move |x| *x < r.x + r.w)
}

/// Street lights for `Lighting` (lamps, their reflections).
pub fn outside_lights(rect_world: Rect) -> Vec<Light> {
    let b = bands(rect_world);
    let mut v = Vec::new();
    for x in lamp_xs(rect_world) {
        let base = vec2(x, b.sidewalk.0 + 0.5 * TILE);
        v.push(Light::new(base, 3.2 * TILE, c::AMBER, 0.9).flicker(0.02));
        v.push(Light::new(
            base - vec2(0.0, 2.0 * TILE),
            0.9 * TILE,
            c::AMBER,
            0.8,
        ));
    }
    // cool platform lights
    let mut x = rect_world.x + 3.0 * TILE;
    while x < rect_world.x + rect_world.w {
        v.push(Light::new(
            vec2(x, b.platform.0 + 0.4 * TILE),
            2.2 * TILE,
            c::INDIGO_SOFT,
            0.45,
        ));
        x += 5.0 * TILE;
    }
    v
}

/// The street behind the café in the world rect `rect_world` (bottom = the café's back wall).
pub fn draw_outside(rect_world: Rect, t: f32) {
    let r = rect_world;
    let b = bands(r);

    // ── distance: sky, fog, silhouettes of houses with windows
    rect_v(
        r.x,
        b.far.0,
        r.w,
        b.far.1 - b.far.0,
        c::VIOLET_NIGHT_0,
        c::VIOLET_NIGHT_1,
    );
    for house in far_houses(r) {
        let FarHouse {
            x,
            w,
            top,
            pointed,
            i,
        } = house;
        let h = b.far.1 - top;
        let col = mix(
            Color::from_hex(0x1d1636),
            Color::from_hex(0x2a2048),
            hash01(i * 5),
        );
        rect_v(x, top, w, h, col, shade(col, 0.85));
        // roofs: sometimes pointed
        if pointed {
            poly(
                &[
                    vec2(x - 4.0, top),
                    vec2(x + w + 4.0, top),
                    vec2(x + w / 2.0, top - 18.0),
                ],
                col,
            );
        }
        // warm windows
        let cols = (w / 14.0) as i32;
        let rows = (h / 16.0) as i32;
        for cy in 0..rows {
            for cx in 0..cols {
                let k = i * 131 + (cy * 17 + cx) as u32;
                if hash01(k) > 0.72 {
                    let wx = x + 6.0 + cx as f32 * 14.0;
                    let wy = top + 8.0 + cy as f32 * 16.0;
                    let warm = hash01(k + 9) > 0.3;
                    let wc = if warm { c::AMBER } else { c::INDIGO_SOFT };
                    let on = 0.7 + 0.3 * noise1(t * 0.2, k);
                    rect_v(
                        wx,
                        wy,
                        6.0,
                        7.0,
                        alpha(wc, 0.85 * on),
                        alpha(shade(wc, 0.8), 0.85 * on),
                    );
                }
            }
        }
    }
    // fog
    rect_v(
        r.x,
        b.far.1 - 50.0,
        r.w,
        50.0,
        alpha(Color::from_hex(0x4a3f7a), 0.0),
        alpha(Color::from_hex(0x4a3f7a), 0.55),
    );

    // ── rails
    let (ty0, ty1) = b.tracks;
    rect_v(
        r.x,
        ty0,
        r.w,
        ty1 - ty0,
        Color::from_hex(0x2a2538),
        Color::from_hex(0x221e30),
    );
    let mut sx = r.x;
    while sx < r.x + r.w {
        rect_v(
            sx,
            ty0 + 14.0,
            9.0,
            ty1 - ty0 - 28.0,
            Color::from_hex(0x3b3044),
            Color::from_hex(0x2c2434),
        );
        sx += 22.0;
    }
    for ry in [ty0 + 24.0, ty1 - 24.0] {
        draw_line(r.x, ry, r.x + r.w, ry, 3.0, Color::from_hex(0x6b6f8a));
        draw_line(
            r.x,
            ry - 1.0,
            r.x + r.w,
            ry - 1.0,
            1.0,
            alpha(c::LAVENDER_RIM, 0.6),
        );
    }

    // ── platform: concrete and the yellow tactile strip at the edge
    let (py0, py1) = b.platform;
    rect_v(
        r.x,
        py0,
        r.w,
        py1 - py0,
        Color::from_hex(0x5a5470),
        Color::from_hex(0x4a4560),
    );
    rect_v(
        r.x,
        py0,
        r.w,
        6.0,
        Color::from_hex(0xd8b24a),
        Color::from_hex(0xb8902e),
    );
    draw_line(r.x, py0 + 7.0, r.x + r.w, py0 + 7.0, 1.0, alpha(BLACK, 0.3));
    // reflections of the platform lamps on wet concrete
    let mut px = r.x + 3.0 * TILE;
    while px < r.x + r.w {
        rect_v(
            px - 2.0,
            py0 - 3.0 * TILE * 0.0 + 10.0,
            4.0,
            py1 - py0 - 12.0,
            alpha(c::INDIGO_SOFT, 0.25),
            alpha(c::INDIGO_SOFT, 0.0),
        );
        // platform lamp post
        draw_line(
            px,
            py0 + 12.0,
            px,
            py0 - 40.0,
            2.0,
            Color::from_hex(0x3a3550),
        );
        rrect(
            px - 8.0,
            py0 - 44.0,
            16.0,
            4.0,
            2.0,
            Color::from_hex(0x3a3550),
        );
        glow(px, py0 - 40.0, 14.0, alpha(c::INDIGO_SOFT, 0.5));
        px += 5.0 * TILE;
    }
    // station name board
    let sign_x = r.x + r.w * 0.62;
    rrect_v(
        sign_x - 78.0,
        py0 + 14.0,
        156.0,
        22.0,
        4.0,
        Color::from_hex(0x2f3b7a),
        Color::from_hex(0x232c60),
    );
    world_text(
        "PRAHA-HOLEŠOVICE",
        vec2(sign_x, py0 + 30.0),
        12.0,
        WHITE,
        true,
    );
    draw_line(
        sign_x - 30.0,
        py0 + 36.0,
        sign_x - 30.0,
        py1,
        2.0,
        Color::from_hex(0x3a3550),
    );
    draw_line(
        sign_x + 30.0,
        py0 + 36.0,
        sign_x + 30.0,
        py1,
        2.0,
        Color::from_hex(0x3a3550),
    );

    // ── road: wet asphalt with reflections
    let (sy0, sy1) = b.street;
    rect_v(
        r.x,
        sy0,
        r.w,
        sy1 - sy0,
        Color::from_hex(0x2b2740),
        Color::from_hex(0x332d4a),
    );
    let mut dx = r.x + 10.0;
    let mid = (sy0 + sy1) / 2.0;
    while dx < r.x + r.w {
        rrect(
            dx,
            mid - 2.0,
            34.0,
            4.0,
            2.0,
            alpha(Color::from_hex(0xc9c2e0), 0.35),
        );
        dx += 70.0;
    }
    for lx in lamp_xs(r) {
        // a long trembling reflection of the lamp
        for k in 0..6 {
            let yy = sy0 + 6.0 + k as f32 * (sy1 - sy0 - 10.0) / 6.0;
            let wob = (t * 2.0 + k as f32 * 1.3).sin() * 3.0;
            let w = 10.0 - k as f32 * 0.8;
            rrect(
                lx - w / 2.0 + wob,
                yy,
                w,
                6.0,
                3.0,
                alpha(c::AMBER, 0.18 - k as f32 * 0.02),
            );
        }
    }
    // kerb
    rect_v(
        r.x,
        sy1 - 4.0,
        r.w,
        6.0,
        Color::from_hex(0x6a6488),
        Color::from_hex(0x4a4566),
    );

    // ── pavement: cobbles
    let (wy0, wy1) = b.sidewalk;
    rect_v(
        r.x,
        wy0,
        r.w,
        wy1 - wy0,
        Color::from_hex(0x3a3352),
        Color::from_hex(0x2e2842),
    );
    let stone = 16.0;
    let mut row = 0;
    let mut yy = wy0 + 2.0;
    while yy < wy1 {
        let off = if row % 2 == 0 { 0.0 } else { stone / 2.0 };
        let mut xx = r.x - off;
        let mut k = 0;
        while xx < r.x + r.w {
            let v = hash2(row, k);
            let col = mix(Color::from_hex(0x4a4266), Color::from_hex(0x5a5078), v);
            rrect_v(
                xx + 1.0,
                yy + 1.0,
                stone - 2.0,
                stone * 0.7 - 2.0,
                3.0,
                col,
                shade(col, 0.8),
            );
            // wet highlight
            if v > 0.6 {
                draw_line(
                    xx + 3.0,
                    yy + 2.5,
                    xx + stone - 6.0,
                    yy + 2.5,
                    1.0,
                    alpha(c::LAVENDER_RIM, 0.35),
                );
            }
            xx += stone;
            k += 1;
        }
        yy += stone * 0.7;
        row += 1;
    }
    // puddles with reflections and rings from the drops
    let mut pi = 0u32;
    let mut pxx = r.x + 40.0;
    while pxx < r.x + r.w {
        let w = 40.0 + hash01(pi * 3) * 60.0;
        let cy = wy0 + 14.0 + hash01(pi * 3 + 1) * (wy1 - wy0 - 28.0);
        let cx = pxx + w / 2.0;
        ellipse_v(
            cx,
            cy,
            w / 2.0,
            9.0,
            Color::from_hex(0x1f1a38),
            Color::from_hex(0x2a2250),
        );
        // reflection of the sky and the lamp
        for lx in lamp_xs(r) {
            if (lx - cx).abs() < w / 2.0 + 10.0 {
                ellipse(lx, cy, 4.0, 7.0, alpha(c::AMBER, 0.55));
                glow(lx, cy, 14.0, alpha(c::AMBER, 0.3));
            }
        }
        draw_line(
            cx - w * 0.3,
            cy - 3.0,
            cx + w * 0.1,
            cy - 4.0,
            1.0,
            alpha(c::LAVENDER_RIM, 0.4),
        );
        for k in 0..2 {
            let life = 1.1;
            let a = ((t + hash01(pi * 13 + k) * life) % life) / life;
            let rx = cx + (hash01(pi * 7 + k + (t / life) as u32 * 3) - 0.5) * w * 0.7;
            let ry = cy + (hash01(pi * 5 + k) - 0.5) * 8.0;
            draw_ellipse_lines(
                rx,
                ry,
                1.0 + a * 7.0,
                0.5 + a * 2.6,
                0.0,
                1.0,
                alpha(c::LAVENDER_RIM, 0.45 * (1.0 - a)),
            );
        }
        pxx += w + 80.0 + hash01(pi * 9) * 140.0;
        pi += 1;
    }

    // ── streetlamps
    for lx in lamp_xs(r) {
        let base = vec2(lx, wy0 + 0.5 * TILE);
        soft_shadow(base.x + 4.0, base.y + 2.0, 12.0, 4.0, 0.4);
        ellipse_v(
            base.x,
            base.y,
            7.0,
            3.0,
            Color::from_hex(0x3a3550),
            Color::from_hex(0x221e30),
        );
        rect_v(
            base.x - 2.0,
            base.y - 2.1 * TILE,
            4.0,
            2.1 * TILE,
            Color::from_hex(0x4a4566),
            Color::from_hex(0x2a2540),
        );
        let head = base - vec2(0.0, 2.1 * TILE);
        capsule(
            head,
            head + vec2(12.0, -4.0),
            2.0,
            Color::from_hex(0x4a4566),
        );
        let bulb = head + vec2(14.0, 0.0);
        rrect_v(
            bulb.x - 8.0,
            bulb.y - 6.0,
            16.0,
            6.0,
            3.0,
            Color::from_hex(0x4a4566),
            Color::from_hex(0x2a2540),
        );
        glow(bulb.x, bulb.y + 2.0, 26.0, alpha(c::AMBER, 0.45));
        ellipse(bulb.x, bulb.y + 1.0, 6.0, 2.0, Color::from_hex(0xfff0c8));
        // cone of light with rain
        quad(
            [
                bulb + vec2(-5.0, 2.0),
                bulb + vec2(5.0, 2.0),
                bulb + vec2(34.0, 120.0),
                bulb + vec2(-34.0, 120.0),
            ],
            [
                alpha(c::AMBER, 0.14),
                alpha(c::AMBER, 0.14),
                alpha(c::AMBER, 0.0),
                alpha(c::AMBER, 0.0),
            ],
        );
    }
}

/// Train length and where it is: the x of its nose for `progress` (see [`draw_train`]).
const TRAIN_LEN: f32 = 14.0 * TILE;
const TRAIN_CAR: f32 = 3.5 * TILE;

fn train_x(r: Rect, progress: f32) -> f32 {
    r.x + r.w - (r.w + TRAIN_LEN + 2.0 * TILE) * progress
}

/// The lit carriage windows of a passing train as x-runs `(x0, x1)` in world px — exactly
/// where [`draw_train`] draws them, so the light through the café windows moves with the
/// train (`Lighting::train_sweep`).
pub fn train_window_runs(rect_world: Rect, progress: f32) -> Vec<(f32, f32)> {
    if !(0.0..=1.0).contains(&progress) {
        return Vec::new();
    }
    let x = train_x(rect_world, progress);
    let mut runs: Vec<(f32, f32)> = Vec::new();
    let mut cx = x + 30.0;
    while cx < x + TRAIN_LEN - 20.0 {
        if (cx - x) % TRAIN_CAR < TRAIN_CAR - 30.0 {
            match runs.last_mut() {
                Some(last) if cx - last.1 < 8.0 => last.1 = cx + 22.0,
                _ => runs.push((cx, cx + 22.0)),
            }
        }
        cx += 28.0;
    }
    runs
}

/// A train on the far tracks. `progress` 0..1 — passing right to left; outside — nothing.
pub fn draw_train(rect_world: Rect, progress: f32, t: f32) {
    if !(0.0..=1.0).contains(&progress) {
        return;
    }
    let r = rect_world;
    let b = bands(r);
    let len = TRAIN_LEN;
    let x = train_x(r, progress);
    let (ty0, ty1) = b.tracks;
    let bot = ty1 - 14.0;
    let h = 58.0;
    let top = bot - h;
    // body with a streamlined nose on the left
    rrect_v(
        x,
        top,
        len,
        h,
        14.0,
        Color::from_hex(0x3a3f6a),
        Color::from_hex(0x1f2140),
    );
    ellipse_v(
        x + 10.0,
        top + h * 0.55,
        26.0,
        h * 0.45,
        Color::from_hex(0x3a3f6a),
        Color::from_hex(0x23264a),
    );
    // indigo light band
    rect_v(
        x + 20.0,
        bot - 14.0,
        len - 30.0,
        3.0,
        c::INDIGO_GLOW,
        c::INDIGO,
    );
    glow(x + 8.0, bot - 22.0, 18.0, alpha(WHITE, 0.6));
    // carriage windows (warm), passengers' silhouettes
    let car = TRAIN_CAR;
    let mut cx = x + 30.0;
    let mut k = 0u32;
    while cx < x + len - 20.0 {
        let in_car = (cx - x) % car;
        if in_car < car - 30.0 {
            let wc = mix(Color::from_hex(0xfff0d0), c::AMBER, hash01(k) * 0.6);
            rrect_v(cx, top + 10.0, 22.0, 22.0, 4.0, wc, shade(wc, 0.85));
            if hash01(k + 7) > 0.6 {
                ellipse(
                    cx + 11.0,
                    top + 26.0,
                    5.0,
                    6.0,
                    alpha(Color::from_hex(0x3a2f45), 0.7),
                );
                ellipse(
                    cx + 11.0,
                    top + 17.0,
                    3.6,
                    3.8,
                    alpha(Color::from_hex(0x3a2f45), 0.7),
                );
            }
        }
        cx += 28.0;
        k += 1;
    }
    // speed glints
    for i in 0..6 {
        let yy = top + 6.0 + i as f32 * 9.0;
        let ox = ((t * 900.0 + i as f32 * 130.0) % (len + 200.0)) - 100.0;
        draw_line(
            x + ox,
            yy,
            x + ox + 60.0,
            yy,
            1.0,
            alpha(c::LAVENDER_RIM, 0.25),
        );
    }
    // soft glow on the rails
    glow(
        x + len / 2.0,
        ty0 + (ty1 - ty0) * 0.7,
        len * 0.45,
        alpha(c::AMBER, 0.12),
    );
}

// ───────────────────────── rain ─────────────────────────

/// Rain. Stateless: set the strength and wind, draw with any `t`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Weather {
    /// 0 — dry, 1 — ordinary rain, 2 — downpour.
    pub intensity: f32,
    /// Slant of the streaks (x share per unit of fall).
    pub wind: f32,
}

impl Default for Weather {
    fn default() -> Self {
        Weather {
            intensity: 1.0,
            wind: 0.18,
        }
    }
}

impl Weather {
    /// Rain streaks and splashes in a world rect (outside). Two parallax layers.
    pub fn draw_rain(&self, rect_world: Rect, t: f32) {
        if self.intensity <= 0.01 {
            return;
        }
        let r = rect_world;
        let area = r.w * r.h / (TILE * TILE);
        let n = (area * 5.0 * self.intensity) as u32;
        for layer in 0..2u32 {
            let (speed, len, thick, a) = if layer == 0 {
                (520.0, 12.0, 0.9, 0.16)
            } else {
                (760.0, 20.0, 1.3, 0.24)
            };
            let count = if layer == 0 { n } else { n / 2 };
            for i in 0..count {
                let k = i * 2 + layer * 7919;
                let x0 = r.x + hash01(k) * r.w;
                let ph = hash01(k + 1);
                let travel = r.h + len;
                let y = r.y + ((ph * travel + t * speed * (0.85 + ph * 0.3)) % travel) - len;
                let x = x0 + (y - r.y) * self.wind;
                let x = r.x + (x - r.x).rem_euclid(r.w);
                let col = Color::new(0.78, 0.8, 1.0, a);
                draw_line(x, y, x + self.wind * len, y + len, thick, col);
            }
        }
        // splashes on the ground
        let splashes = (r.w / TILE * 2.5 * self.intensity) as u32;
        for i in 0..splashes {
            let life = 0.45;
            let ph = hash01(i * 3 + 11);
            let cycle = ((t + ph * life) / life).floor() as u32;
            let a = ((t + ph * life) % life) / life;
            let x = r.x + hash01(i * 7 + cycle * 13) * r.w;
            let y = r.y + r.h * (0.45 + hash01(i * 5 + cycle * 17) * 0.55);
            draw_ellipse_lines(
                x,
                y,
                1.0 + a * 5.0,
                0.6 + a * 1.8,
                0.0,
                0.9,
                Color::new(0.8, 0.82, 1.0, 0.4 * (1.0 - a)),
            );
        }
    }

    /// Drops and trickles on a window pane (`glass` — `window_glass_rect`).
    pub fn draw_window_rain(&self, glass: Rect, t: f32) {
        if self.intensity <= 0.01 {
            return;
        }
        let g = glass;
        let seed = (g.x as i32 as u32).wrapping_mul(2654435761);
        // small resting drops
        for i in 0..(10.0 * self.intensity) as u32 {
            let x = g.x + 2.0 + hash01(seed + i * 3) * (g.w - 4.0);
            let y = g.y + 2.0 + hash01(seed + i * 3 + 1) * (g.h - 4.0);
            ellipse(x, y, 0.9, 1.1, Color::new(0.85, 0.88, 1.0, 0.35));
        }
        // running drops: "stick, then slide"
        for i in 0..(4.0 * self.intensity).ceil() as u32 {
            let speed = 14.0 + hash01(seed + i * 5) * 20.0;
            let ph = hash01(seed + i * 5 + 1) * 10.0;
            let tt = t + ph;
            let stick = (tt * 0.7).floor() + ((tt * 0.7).fract()).powf(3.0);
            let travel = g.h + 10.0;
            let y = g.y + (stick * speed * 1.4) % travel - 4.0;
            let lane = ((stick * speed * 1.4) / travel).floor() as u32;
            let x = g.x + 3.0 + hash01(seed + i * 5 + 2 + lane * 31) * (g.w - 6.0);
            if y < g.y || y > g.y + g.h - 1.0 {
                continue;
            }
            let tail = (y - g.y).min(14.0);
            draw_line(x, y - tail, x, y, 1.0, Color::new(0.8, 0.84, 1.0, 0.22));
            ellipse(x, y, 1.3, 1.7, Color::new(0.9, 0.92, 1.0, 0.6));
            ellipse(x - 0.4, y - 0.5, 0.4, 0.4, alpha(WHITE, 0.9));
        }
    }
}

// ───────────────────────── sign ─────────────────────────

thread_local! {
    static FONT: OnceCell<Option<Font>> = const { OnceCell::new() };
    /// World text changes raster size with the zoom: new glyphs are cached between batches
    /// (see `ui::text::GlyphCache`), never in the middle of one.
    static GLYPHS: crate::ui::text::GlyphCache = crate::ui::text::GlyphCache::default();
}

fn prepare_glyphs(font: Option<&Font>, text: &str, fs: u16) {
    if let Some(f) = font {
        GLYPHS.with(|g| g.prepare(f, text, fs));
    }
}

fn with_font<R>(f: impl FnOnce(Option<&Font>) -> R) -> R {
    FONT.with(|cell| {
        let font = cell.get_or_init(|| {
            load_ttf_font_from_bytes(include_bytes!("../../assets/fonts/Inter-SemiBold.ttf")).ok()
        });
        f(font.as_ref())
    })
}

/// Text in world coordinates (crisp at any zoom). `y` is the baseline.
pub fn world_text(text: &str, pos: Vec2, size: f32, col: Color, centered: bool) {
    world_text_rot(text, pos, size, col, centered, 0.0);
}

/// Like [`world_text`], rotated by `rotation` radians around `pos`.
pub fn world_text_rot(text: &str, pos: Vec2, size: f32, col: Color, centered: bool, rotation: f32) {
    with_font(|font| {
        let (fs, scale, aspect) = camera_font_scale(size);
        prepare_glyphs(font, text, fs);
        let dim = measure_text(text, font, fs, scale);
        let off = if centered { dim.width / 2.0 } else { 0.0 };
        let (s, c0) = rotation.sin_cos();
        draw_text_ex(
            text,
            pos.x - off * c0,
            pos.y - off * s,
            TextParams {
                font,
                font_size: fs,
                font_scale: scale,
                font_scale_aspect: aspect,
                color: col,
                rotation,
            },
        );
    });
}

/// The width of `text` at `size` in the current camera's units.
pub fn world_text_width(text: &str, size: f32) -> f32 {
    with_font(|font| {
        let (fs, scale, _) = camera_font_scale(size);
        prepare_glyphs(font, text, fs);
        measure_text(text, font, fs, scale).width
    })
}

/// A flickering holographic sign (`pos` = bottom centre, on its projector post).
pub fn draw_holo_sign(pos: Vec2, text: &str, t: f32) {
    let fl = holo_flicker(t, 11);
    let w = (text.chars().count() as f32 * 11.0 + 40.0).max(90.0);
    let h = 34.0;
    let (x, y) = (pos.x - w / 2.0, pos.y - 70.0 - h);
    // projector post
    soft_shadow(pos.x + 3.0, pos.y, 14.0, 5.0, 0.45);
    rect_v(
        pos.x - 2.0,
        pos.y - 70.0,
        4.0,
        70.0,
        Color::from_hex(0x4a4566),
        Color::from_hex(0x2a2540),
    );
    ellipse(pos.x, pos.y - 70.0, 7.0, 2.6, Color::from_hex(0x2a2540));
    quad(
        [
            vec2(pos.x - 3.0, pos.y - 71.0),
            vec2(pos.x + 3.0, pos.y - 71.0),
            vec2(x + w, y + h),
            vec2(x, y + h),
        ],
        [
            alpha(c::INDIGO_GLOW, 0.4 * fl),
            alpha(c::INDIGO_GLOW, 0.4 * fl),
            alpha(c::INDIGO_GLOW, 0.05),
            alpha(c::INDIGO_GLOW, 0.05),
        ],
    );
    glow(
        pos.x,
        y + h / 2.0,
        w * 0.75,
        alpha(c::INDIGO_GLOW, 0.3 * fl),
    );
    rrect_v(
        x,
        y,
        w,
        h,
        8.0,
        alpha(Color::from_hex(0x3b3ea8), 0.55 * fl),
        alpha(Color::from_hex(0x5b3aa8), 0.5 * fl),
    );
    let edge = alpha(c::INDIGO_SOFT, 0.95 * fl);
    draw_rectangle_lines(x, y, w, h, 1.5, edge);
    // cup icon
    let ic = vec2(x + 16.0, y + h / 2.0 + 4.0);
    rrect(
        ic.x - 6.0,
        ic.y - 8.0,
        12.0,
        9.0,
        3.0,
        alpha(c::AMBER, 0.95 * fl),
    );
    draw_circle_lines(ic.x + 7.0, ic.y - 4.0, 2.6, 1.4, alpha(c::AMBER, 0.95 * fl));
    for i in 0..2 {
        let sx = ic.x - 2.0 + i as f32 * 4.0;
        let wv = (t * 3.0 + i as f32 * 2.0).sin() * 1.5;
        draw_line(
            sx,
            ic.y - 10.0,
            sx + wv,
            ic.y - 15.0,
            1.2,
            alpha(WHITE, 0.6 * fl),
        );
    }
    world_text(
        text,
        vec2(x + 30.0 + (w - 30.0) / 2.0, y + h / 2.0 + 6.0),
        16.0,
        alpha(Color::from_hex(0xeef0ff), fl),
        true,
    );
    // scan line + a horizontal "glitch"
    let scan = (t * 0.8).fract();
    draw_line(
        x + 2.0,
        y + scan * h,
        x + w - 2.0,
        y + scan * h,
        1.0,
        alpha(WHITE, 0.2 * fl),
    );
    if fl < 0.8 {
        let gy = y + h * hash01((t * 20.0) as u32);
        draw_line(
            x - 6.0,
            gy,
            x + w + 6.0,
            gy,
            2.0,
            alpha(c::VIOLET_SOFT, 0.5),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn street_bands_are_ordered_bottom_up() {
        let r = Rect::new(0.0, -8.0 * TILE, 30.0 * TILE, 6.5 * TILE);
        let b = bands(r);
        assert_eq!(b.sidewalk.1, r.y + r.h);
        assert!(b.sidewalk.0 > b.street.0 && b.street.0 > b.platform.0);
        assert!(b.platform.0 > b.tracks.0 && b.tracks.0 > b.far.0);
        assert!(b.far.1 > b.far.0, "room left for the town");
    }

    #[test]
    fn lamps_stay_inside_the_strip() {
        let r = Rect::new(-128.0, 0.0, 40.0 * TILE, 400.0);
        let xs: Vec<f32> = lamp_xs(r).collect();
        assert!(!xs.is_empty());
        assert!(xs.iter().all(|x| *x >= r.x && *x < r.x + r.w));
    }
}
