//! Seasons over the 30 nights (`design/handson.md` §11.1): late autumn (wet leaves,
//! heavy rain), frost (frosted panes, breath puffs), the festive Advent weeks (garlands,
//! a wreath, a star on the sign) and the first snow (snowfall, snow on the street,
//! the sills and the sign).
//!
//! Everything is driven by a [`SeasonLook`] — a bundle of 0..1 amounts that blends
//! smoothly ([`SeasonLook::lerp`], [`Season::blend`]), so a phase change can fade over
//! a few seconds. All drawing here is stateless (a function of `t`).
//!
//! Frame order (outside → inside):
//! ```ignore
//! draw_outside(street, t);
//! draw_outside_season(street, &look, t);        // leaves, wet sheen, snow cover, snowy roofs
//! draw_train(street, p, t);
//! draw_holo_sign(pos, name, t);  draw_sign_season(pos, name, &look, t);
//! weather.draw_rain(street, t);  Snowfall::from_look(&look).draw(street, t);
//! statics.draw();                // built with draw_floor_season(..) over pavement tiles
//! for g in glass { weather.draw_window_rain(g, t); draw_window_season(g, &look, t); }
//! // … queue, lighting …
//! for person outside { draw_breath(look, pose, t, season.breath) }   // after lighting
//! ```

use macroquad::prelude::*;

use super::colors as c;
use super::draw::*;
use super::look::{Facing, PersonLook, PersonPose, Pose};
use super::outside::{bands, far_houses, lamp_xs};
use super::theme::leaf_shape;
use crate::map::TILE;

/// A season phase of the story calendar (ids as in `assets/data/seasons.ron`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Season {
    /// `late_autumn` (nights 1–13): rain, wet leaves, amber street light.
    #[default]
    LateAutumn,
    /// `frost` (14–20): frosted window corners, breath puffs, fewer leaves.
    Frost,
    /// `advent` (21–26): garlands and fairy lights, a wreath, a star on the sign.
    Festive,
    /// `first_snow` (27–30): snowfall, snow caps, frosted glass — still festive.
    FirstSnow,
}

impl Season {
    pub const ALL: [Season; 4] = [
        Season::LateAutumn,
        Season::Frost,
        Season::Festive,
        Season::FirstSnow,
    ];

    pub fn from_id(id: &str) -> Option<Season> {
        Some(match id {
            "late_autumn" | "autumn" => Season::LateAutumn,
            "frost" => Season::Frost,
            "advent" | "festive" => Season::Festive,
            "first_snow" | "winter" | "snow" => Season::FirstSnow,
            _ => return None,
        })
    }

    pub fn id(self) -> &'static str {
        match self {
            Season::LateAutumn => "late_autumn",
            Season::Frost => "frost",
            Season::Festive => "advent",
            Season::FirstSnow => "first_snow",
        }
    }

    /// The full look of this phase.
    pub fn look(self) -> SeasonLook {
        match self {
            Season::LateAutumn => SeasonLook {
                leaves: 1.0,
                wet: 1.0,
                ..SeasonLook::NONE
            },
            Season::Frost => SeasonLook {
                leaves: 0.45,
                wet: 0.6,
                frost: 0.5,
                breath: 1.0,
                ..SeasonLook::NONE
            },
            Season::Festive => SeasonLook {
                leaves: 0.12,
                wet: 0.5,
                frost: 0.6,
                breath: 1.0,
                festive: 1.0,
                wreath: 1.0,
                ..SeasonLook::NONE
            },
            Season::FirstSnow => SeasonLook {
                leaves: 0.0,
                wet: 0.2,
                frost: 0.85,
                breath: 1.0,
                snowfall: 1.0,
                snow_cover: 1.0,
                festive: 1.0,
                wreath: 1.0,
            },
        }
    }

    /// A smooth transition `from` → `to` (`t` 0..1, eased).
    pub fn blend(from: Season, to: Season, t: f32) -> SeasonLook {
        let t = t.clamp(0.0, 1.0);
        let e = t * t * (3.0 - 2.0 * t);
        SeasonLook::lerp(&from.look(), &to.look(), e)
    }
}

/// How much of each seasonal effect to draw (all 0..1, blendable).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SeasonLook {
    /// Falling leaves and wet leaves stuck on the pavement and the doorstep.
    pub leaves: f32,
    /// Extra wet sheen on the street.
    pub wet: f32,
    /// Frost in the window corners.
    pub frost: f32,
    /// Breath puffs for people outside.
    pub breath: f32,
    /// Falling snow.
    pub snowfall: f32,
    /// Snow lying on the street, sills, roofs and lamp caps.
    pub snow_cover: f32,
    /// Garlands allowed, the star on the sign (garlands still render only via [`super::Garlands`]).
    pub festive: f32,
    /// The Advent wreath on the counter.
    pub wreath: f32,
}

impl SeasonLook {
    pub const NONE: SeasonLook = SeasonLook {
        leaves: 0.0,
        wet: 0.0,
        frost: 0.0,
        breath: 0.0,
        snowfall: 0.0,
        snow_cover: 0.0,
        festive: 0.0,
        wreath: 0.0,
    };

    /// From the `visuals` of a `seasons.ron` phase (missing values = off).
    pub fn from_flags(
        leaves: bool,
        frost: f32,
        breath: bool,
        snow: bool,
        garlands: bool,
        wreath: bool,
    ) -> SeasonLook {
        let b = |v: bool| if v { 1.0 } else { 0.0 };
        SeasonLook {
            leaves: b(leaves),
            wet: if snow { 0.2 } else { 0.6 + 0.4 * b(leaves) },
            frost: frost.clamp(0.0, 1.0),
            breath: b(breath),
            snowfall: b(snow),
            snow_cover: b(snow),
            festive: b(garlands),
            wreath: b(wreath),
        }
    }

    pub fn lerp(a: &SeasonLook, b: &SeasonLook, t: f32) -> SeasonLook {
        if t <= 0.0 {
            return *a;
        }
        if t >= 1.0 {
            return *b;
        }
        let l = |x: f32, y: f32| x + (y - x) * t.clamp(0.0, 1.0);
        SeasonLook {
            leaves: l(a.leaves, b.leaves),
            wet: l(a.wet, b.wet),
            frost: l(a.frost, b.frost),
            breath: l(a.breath, b.breath),
            snowfall: l(a.snowfall, b.snowfall),
            snow_cover: l(a.snow_cover, b.snow_cover),
            festive: l(a.festive, b.festive),
            wreath: l(a.wreath, b.wreath),
        }
    }

    /// The static parts (pavement leaves / snow, quantized) for a static-layer cache key.
    pub fn static_key(&self) -> u64 {
        let q = |v: f32| (v.clamp(0.0, 1.0) * 8.0).round() as u64;
        q(self.leaves) | q(self.snow_cover) << 4
    }
}

/// Snow colours (before lighting): lavender-white, shadows in soft violet.
const SNOW: Color = Color::from_hex(0xcdc6ec);
const SNOW_SHADE: Color = Color::from_hex(0x948bc4);
/// Falling flakes (they are small, so they can be brighter).
const FLAKE: Color = Color::from_hex(0xeeeaff);
/// Autumn leaf colours.
const LEAF_COLS: [u32; 5] = [0xe08a3a, 0xc9603a, 0xd9a440, 0x9a4a3a, 0x7a5a3a];

fn leaf_col(k: u32) -> Color {
    Color::from_hex(LEAF_COLS[(hash(k) % LEAF_COLS.len() as u32) as usize])
}

// ───────────────────────── outside ─────────────────────────

/// Seasonal layers over the street (call right after `draw_outside`).
pub fn draw_outside_season(rect_world: Rect, look: &SeasonLook, t: f32) {
    let r = rect_world;
    let b = bands(r);
    let (wy0, wy1) = b.sidewalk;
    let (sy0, sy1) = b.street;
    let (py0, py1) = b.platform;
    let (ty0, ty1) = b.tracks;

    // ── extra wet sheen: long lamp reflections on the cobbles
    if look.wet > 0.01 && look.snow_cover < 0.9 {
        let k = look.wet * (1.0 - look.snow_cover);
        for lx in lamp_xs(r) {
            for j in 0..5 {
                let yy = wy0 + 8.0 + j as f32 * (wy1 - wy0 - 12.0) / 5.0;
                let wob = (t * 1.7 + j as f32 * 0.9).sin() * 2.0;
                let w = 22.0 - j as f32 * 3.0;
                rrect(
                    lx + 14.0 - w / 2.0 + wob,
                    yy,
                    w,
                    4.0,
                    2.0,
                    alpha(c::AMBER, 0.12 * k),
                );
            }
        }
        let mut x = r.x;
        let mut i = 0u32;
        while x < r.x + r.w {
            let yy = sy0 + 6.0 + hash01(i * 3 + 5) * (sy1 - sy0 - 12.0);
            let w = 20.0 + hash01(i) * 50.0;
            draw_line(x, yy, x + w, yy, 1.0, alpha(c::LAVENDER_RIM, 0.14 * k));
            x += w + 30.0 + hash01(i * 7) * 60.0;
            i += 1;
        }
    }

    // ── wet leaves stuck on the pavement and the road edge
    if look.leaves > 0.01 {
        let n = (r.w / 14.0 * look.leaves) as u32;
        for i in 0..n {
            let x = r.x + hash01(i * 5 + 1) * r.w;
            let band = hash01(i * 5 + 2);
            let y = if band < 0.75 {
                wy0 + 4.0 + hash01(i * 5 + 3) * (wy1 - wy0 - 6.0)
            } else {
                sy1 - 4.0 - hash01(i * 5 + 3) * 14.0
            };
            let col = mix(leaf_col(i), Color::from_hex(0x3a2a3a), 0.25);
            leaf_shape(
                vec2(x, y),
                hash01(i * 5 + 4) * std::f32::consts::TAU,
                6.0 + hash01(i) * 3.0,
                alpha(col, 0.85),
            );
        }
    }

    // ── snow cover
    let s = look.snow_cover;
    if s > 0.01 {
        // far roofs
        for h in far_houses(r) {
            let col = alpha(SNOW, 0.85 * s);
            if h.pointed {
                // a snow line along both roof slopes
                let apex = vec2(h.x + h.w / 2.0, h.top - 18.0);
                for (a, bb) in [
                    (vec2(h.x - 4.0, h.top), apex),
                    (apex, vec2(h.x + h.w + 4.0, h.top)),
                ] {
                    draw_line(a.x, a.y - 1.0, bb.x, bb.y - 1.0, 3.0, col);
                }
            } else {
                rrect(h.x - 2.0, h.top - 3.0, h.w + 4.0, 4.5, 2.0, col);
            }
        }
        // the track bed: snow between the sleepers, rails stay dark
        rect_v(
            r.x,
            ty0 + 4.0,
            r.w,
            ty1 - ty0 - 8.0,
            alpha(SNOW_SHADE, 0.35 * s),
            alpha(SNOW, 0.45 * s),
        );
        for ry in [ty0 + 24.0, ty1 - 24.0] {
            draw_line(
                r.x,
                ry,
                r.x + r.w,
                ry,
                2.0,
                alpha(Color::from_hex(0x6b6f8a), s),
            );
        }
        // the platform: a thin layer, the yellow edge still peeks through
        rect_v(
            r.x,
            py0 + 7.0,
            r.w,
            py1 - py0 - 7.0,
            alpha(SNOW, 0.7 * s),
            alpha(SNOW_SHADE, 0.6 * s),
        );
        rect_v(r.x, py0, r.w, 3.0, alpha(SNOW, 0.6 * s), alpha(SNOW, 0.0));
        // the road: snowy edges, two dark wet tyre tracks
        rect_v(
            r.x,
            sy0,
            r.w,
            10.0,
            alpha(SNOW, 0.65 * s),
            alpha(SNOW_SHADE, 0.0),
        );
        rect_v(
            r.x,
            sy1 - 16.0,
            r.w,
            16.0,
            alpha(SNOW_SHADE, 0.0),
            alpha(SNOW, 0.75 * s),
        );
        let mid = (sy0 + sy1) / 2.0;
        for (k, off) in [-14.0_f32, 14.0].iter().enumerate() {
            rect_v(
                r.x,
                mid + off - 5.0,
                r.w,
                10.0,
                alpha(SNOW_SHADE, 0.18 * s),
                alpha(SNOW_SHADE, 0.12 * s),
            );
            let _ = k;
        }
        // the pavement: a soft blanket with a lumpy edge and a trodden path
        rect_v(
            r.x,
            wy0,
            r.w,
            wy1 - wy0,
            alpha(SNOW, 0.62 * s),
            alpha(mix(SNOW, SNOW_SHADE, 0.35), 0.7 * s),
        );
        // soft drifts and hollows
        let mut x = r.x;
        let mut i = 0u32;
        while x < r.x + r.w {
            let y = wy0 + 8.0 + hash01(i * 7 + 3) * (wy1 - wy0 - 16.0);
            let rx = 18.0 + hash01(i * 7 + 4) * 30.0;
            let col = if i.is_multiple_of(3) {
                SNOW_SHADE
            } else {
                SNOW
            };
            ellipse(x, y, rx, 4.0 + hash01(i) * 3.0, alpha(col, 0.18 * s));
            x += rx * 0.9;
            i += 1;
        }
        let mut x = r.x;
        let mut i = 0u32;
        while x < r.x + r.w {
            let rr = 6.0 + hash01(i * 3) * 8.0;
            ellipse(
                x,
                wy0 + 1.0,
                rr,
                3.0 + hash01(i) * 2.0,
                alpha(SNOW, 0.7 * s),
            );
            x += rr * 1.4;
            i += 1;
        }
        // trodden path with footprints
        let py = wy0 + (wy1 - wy0) * 0.55;
        rect_v(
            r.x,
            py - 9.0,
            r.w,
            18.0,
            alpha(SNOW_SHADE, 0.18 * s),
            alpha(SNOW_SHADE, 0.3 * s),
        );
        let mut fx = r.x + 6.0;
        let mut k = 0u32;
        while fx < r.x + r.w {
            let side = if k.is_multiple_of(2) { -4.0 } else { 4.0 };
            ellipse(
                fx,
                py + side + (hash01(k) - 0.5) * 2.0,
                2.6,
                1.6,
                alpha(Color::from_hex(0x8a82b8), 0.35 * s),
            );
            fx += 11.0 + hash01(k * 3) * 3.0;
            k += 1;
        }
        // snow caps on the streetlamp heads
        for lx in lamp_xs(r) {
            let base = vec2(lx, wy0 + 0.5 * TILE);
            let head = base - vec2(0.0, 2.1 * TILE);
            let bulb = head + vec2(14.0, 0.0);
            ellipse_v(
                bulb.x,
                bulb.y - 6.5,
                9.0,
                2.6,
                alpha(SNOW, s),
                alpha(SNOW_SHADE, s),
            );
            ellipse(head.x + 6.0, head.y - 3.5, 6.0, 1.6, alpha(SNOW, 0.9 * s));
            // a small mound at the post's foot
            ellipse_v(
                base.x,
                base.y + 1.0,
                12.0,
                4.0,
                alpha(SNOW, 0.9 * s),
                alpha(SNOW_SHADE, 0.9 * s),
            );
        }
    }
}

/// The sign's season touches: a snow cap on the projector, a holo star in Advent.
/// `pos`/`text` — the same as for `draw_holo_sign`.
pub fn draw_sign_season(pos: Vec2, text: &str, look: &SeasonLook, t: f32) {
    let w = (text.chars().count() as f32 * 11.0 + 40.0).max(90.0);
    let h = 34.0;
    let (x, y) = (pos.x - w / 2.0, pos.y - 70.0 - h);
    if look.festive > 0.01 {
        let k = look.festive;
        let c0 = vec2(x + w - 4.0, y - 2.0);
        let pulse = 0.85 + (t * 2.2).sin() * 0.15;
        glow(c0.x, c0.y, 16.0, alpha(c::AMBER, 0.35 * k * pulse));
        star(
            c0,
            7.0,
            3.0,
            0.2 + t * 0.15,
            alpha(Color::from_hex(0xfff0c0), 0.95 * k),
        );
        star(c0, 3.4, 1.5, 0.2 + t * 0.15, alpha(WHITE, 0.9 * k));
        // a few twinkles along the top edge
        for i in 0..4 {
            let tw = ((t * 3.0 + i as f32 * 1.7).sin() * 0.5 + 0.5).powi(3);
            glow(
                x + 12.0 + i as f32 * (w - 30.0) / 3.0,
                y,
                3.0 + tw * 2.0,
                alpha(WHITE, 0.6 * tw * k),
            );
        }
    }
    let s = look.snow_cover;
    if s > 0.01 {
        ellipse_v(
            pos.x,
            pos.y - 72.0,
            9.0,
            3.0,
            alpha(SNOW, s),
            alpha(SNOW_SHADE, s),
        );
        ellipse_v(
            pos.x,
            pos.y + 1.0,
            16.0,
            5.0,
            alpha(SNOW, 0.9 * s),
            alpha(SNOW_SHADE, 0.9 * s),
        );
    }
}

/// A five-pointed star (outer radius `r`, inner `ri`, rotated by `rot`).
pub fn star(c0: Vec2, r: f32, ri: f32, rot: f32, col: Color) {
    let mut pts = [Vec2::ZERO; 10];
    for (i, p) in pts.iter_mut().enumerate() {
        let a = rot - std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
        let rr = if i % 2 == 0 { r } else { ri };
        *p = c0 + vec2(a.cos(), a.sin()) * rr;
    }
    fan_pub(c0, &pts, c0.y - r, c0.y + r, col, col);
}

// ───────────────────────── particles ─────────────────────────

/// Falling leaves over a world rect (late autumn; a few tumble even in the frost).
pub fn draw_falling_leaves(rect_world: Rect, amount: f32, wind: f32, t: f32) {
    if amount <= 0.01 {
        return;
    }
    let r = rect_world;
    let n = (r.w * r.h / (TILE * TILE) * 0.12 * amount) as u32;
    for i in 0..n {
        let speed = 26.0 + hash01(i * 7) * 22.0;
        let ph = hash01(i * 7 + 1);
        let travel = r.h + 20.0;
        let y = r.y - 10.0 + (ph * travel + t * speed) % travel;
        let sway = (t * (0.8 + hash01(i * 7 + 2)) + ph * 6.0).sin() * 14.0;
        let x0 = r.x + hash01(i * 7 + 3) * r.w;
        let x = r.x + (x0 + sway + (y - r.y) * wind * 0.6 - r.x).rem_euclid(r.w);
        // tumbling: the leaf narrows as it turns
        let spin = t * (2.0 + hash01(i * 7 + 4) * 3.0) + ph * 9.0;
        let len = 6.0 + hash01(i) * 3.0;
        let col = leaf_col(i * 13 + 1);
        let flat = spin.cos().abs().max(0.25);
        let ang = spin * 0.5;
        let dir = vec2(ang.cos(), ang.sin());
        let n2 = vec2(-dir.y, dir.x) * flat;
        poly(
            &[
                vec2(x, y) - dir * len * 0.5,
                vec2(x, y) + n2 * len * 0.3,
                vec2(x, y) + dir * len * 0.5,
                vec2(x, y) - n2 * len * 0.3,
            ],
            alpha(col, 0.9 * amount.min(1.0)),
        );
    }
}

/// Snowfall. Stateless like [`super::Weather`]: set it, draw with any `t`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Snowfall {
    /// 0 — none, 1 — steady snow, 2 — a flurry.
    pub intensity: f32,
    /// Horizontal drift.
    pub wind: f32,
}

impl Default for Snowfall {
    fn default() -> Self {
        Snowfall {
            intensity: 1.0,
            wind: 0.15,
        }
    }
}

impl Snowfall {
    pub fn from_look(look: &SeasonLook) -> Snowfall {
        Snowfall {
            intensity: look.snowfall,
            ..Default::default()
        }
    }

    /// Flakes in a world rect (outside): two layers + a few big soft near flakes.
    pub fn draw(&self, rect_world: Rect, t: f32) {
        if self.intensity <= 0.01 {
            return;
        }
        let r = rect_world;
        let area = r.w * r.h / (TILE * TILE);
        let n = (area * 2.6 * self.intensity) as u32;
        for layer in 0..3u32 {
            let (speed, size, a, count) = match layer {
                0 => (24.0, 1.1, 0.55, n),
                1 => (40.0, 1.8, 0.75, n / 2),
                _ => (62.0, 3.2, 0.35, n / 14),
            };
            for i in 0..count {
                let k = i * 3 + layer * 7919;
                let ph = hash01(k + 1);
                let travel = r.h + 10.0;
                let y = r.y - 5.0 + (ph * travel + t * speed * (0.8 + ph * 0.4)) % travel;
                let sway = (t * (0.6 + hash01(k + 2) * 0.8) + ph * 7.0).sin() * (4.0 + size * 2.0);
                let x0 = r.x + hash01(k) * r.w;
                let x = r.x + (x0 + sway + (y - r.y) * self.wind - r.x).rem_euclid(r.w);
                if layer == 2 {
                    glow(x, y, size * 2.2, alpha(FLAKE, a));
                } else {
                    ellipse(x, y, size, size * 0.9, alpha(FLAKE, a));
                }
            }
        }
    }
}

// ───────────────────────── windows & floors ─────────────────────────

/// Seasonal touches on a window pane (after `draw_window_rain`): snow drifting behind
/// the glass, a snow ridge on the outer sill, frost creeping in from the corners.
pub fn draw_window_season(glass: Rect, look: &SeasonLook, t: f32) {
    let g = glass;
    let seed = (g.x as i32 as u32).wrapping_mul(2_654_435_761) ^ (g.y as i32 as u32);
    // snow behind the glass
    if look.snowfall > 0.01 {
        let n = (14.0 * look.snowfall) as u32;
        for i in 0..n {
            let k = seed.wrapping_add(i * 5);
            let ph = hash01(k);
            let speed = 10.0 + hash01(k + 1) * 10.0;
            let y = g.y + (ph * g.h + t * speed) % g.h;
            let sway = (t * 0.9 + ph * 6.0).sin() * 3.0;
            let x = g.x + 1.5 + (hash01(k + 2) * (g.w - 3.0) + sway).rem_euclid(g.w - 3.0);
            ellipse(x, y, 0.9, 0.8, alpha(FLAKE, 0.75 * look.snowfall));
        }
    }
    // a snow ridge on the outer sill (seen through the bottom of the pane)
    if look.snow_cover > 0.01 {
        let s = look.snow_cover;
        let h = 4.0 + 2.0 * s;
        rect_v(
            g.x,
            g.y + g.h - h,
            g.w,
            h,
            alpha(SNOW, 0.9 * s),
            alpha(SNOW_SHADE, 0.9 * s),
        );
        let n = 4;
        for i in 0..n {
            let bx = g.x + (i as f32 + 0.5) * g.w / n as f32;
            ellipse(
                bx,
                g.y + g.h - h,
                g.w / n as f32 * 0.7,
                1.6 + hash01(seed.wrapping_add(i as u32)) * 1.4,
                alpha(SNOW, 0.95 * s),
            );
        }
    }
    // frost from the corners: soft white haze + fern-like crystals
    let f = look.frost;
    if f > 0.01 {
        let corners = [
            (vec2(g.x, g.y + g.h), vec2(1.0, -1.0), 1.0),
            (vec2(g.x + g.w, g.y + g.h), vec2(-1.0, -1.0), 1.0),
            (vec2(g.x, g.y), vec2(1.0, 1.0), 0.6),
            (vec2(g.x + g.w, g.y), vec2(-1.0, 1.0), 0.6),
        ];
        let haze = Color::new(0.78, 0.8, 0.95, 1.0);
        for (ci, (p, dir, k)) in corners.iter().enumerate() {
            let rad = g.w.min(g.h) * (0.35 + 0.45 * f) * k;
            radial(p.x, p.y, rad, rad, alpha(haze, 0.38 * f), alpha(haze, 0.0));
            // crystals: a few branching strokes growing towards the centre
            let branches = 3;
            for b in 0..branches {
                let kk = seed.wrapping_add(ci as u32 * 31 + b * 7);
                let ang = std::f32::consts::FRAC_PI_4 * (0.4 + hash01(kk) * 1.2);
                let d = vec2(dir.x * ang.cos(), dir.y * ang.sin());
                let len = rad * (0.6 + hash01(kk + 1) * 0.5);
                let a = *p;
                let bpt = a + d * len;
                draw_line(a.x, a.y, bpt.x, bpt.y, 0.7, alpha(haze, 0.6 * f));
                for j in 1..4 {
                    let q = a + d * len * (j as f32 / 4.0);
                    let side = vec2(-d.y, d.x) * len * 0.18 * (1.0 - j as f32 / 5.0);
                    for sg in [-1.0_f32, 1.0] {
                        let e = q + side * sg + d * len * 0.08;
                        draw_line(q.x, q.y, e.x, e.y, 0.5, alpha(haze, 0.5 * f));
                    }
                }
            }
        }
        // a misted band along the bottom
        rect_v(
            g.x,
            g.y + g.h * 0.7,
            g.w,
            g.h * 0.3,
            alpha(haze, 0.0),
            alpha(haze, 0.14 * f),
        );
    }
}

/// Seasonal overlay for a floor tile in the static layer (call after `draw_floor`):
/// wet leaves / snow on `pavement` tiles. Other kinds are untouched.
pub fn draw_floor_season(kind: &str, tile: Vec2, look: &SeasonLook) {
    if kind != "pavement" {
        return;
    }
    let p = tile * TILE;
    let (tx, ty) = (tile.x as i32, tile.y as i32);
    if look.leaves > 0.01 {
        let n = (3.0 * look.leaves + hash2(tx, ty) * 2.0) as i32;
        for k in 0..n {
            let x = p.x + 6.0 + hash2(tx * 7 + k, ty) * (TILE - 12.0);
            let y = p.y + 6.0 + hash2(tx, ty * 7 + k) * (TILE - 12.0);
            let col = mix(
                leaf_col((tx * 31 + ty * 17 + k) as u32),
                Color::from_hex(0x3a2a3a),
                0.2,
            );
            leaf_shape(
                vec2(x, y),
                hash2(tx + k, ty - k) * std::f32::consts::TAU,
                7.0,
                alpha(col, 0.9),
            );
        }
    }
    let s = look.snow_cover;
    if s > 0.01 {
        rect_v(
            p.x,
            p.y,
            TILE,
            TILE,
            alpha(SNOW, 0.86 * s),
            alpha(mix(SNOW, SNOW_SHADE, 0.3), 0.9 * s),
        );
        for k in 0..5 {
            let x = p.x + hash2(tx * 5 + k, ty * 3) * TILE;
            let y = p.y + hash2(tx * 3, ty * 5 + k) * TILE;
            ellipse(
                x,
                y,
                7.0 + hash2(k, tx) * 6.0,
                3.0,
                alpha(if k % 2 == 0 { FLAKE } else { SNOW_SHADE }, 0.25 * s),
            );
        }
        // a few cobbles peek through
        for k in 0..3 {
            let x = p.x + hash2(tx * 11 + k, ty * 13) * (TILE - 14.0);
            let y = p.y + hash2(tx * 13, ty * 11 + k) * (TILE - 8.0);
            rrect(
                x,
                y,
                12.0,
                6.0,
                3.0,
                alpha(Color::from_hex(0x5a5078), 0.35 * (1.0 - s * 0.5)),
            );
        }
    }
}

/// A doorstep tile inside (call after `draw_floor` on the tile behind the door):
/// a wet leaf or two in autumn, slushy footprints in the snow.
pub fn draw_doorstep_season(tile: Vec2, look: &SeasonLook) {
    let p = tile * TILE;
    let (tx, ty) = (tile.x as i32, tile.y as i32);
    if look.leaves > 0.05 {
        for k in 0..2 {
            let x = p.x + 14.0 + hash2(tx * 3 + k, ty) * (TILE - 28.0);
            let y = p.y + 30.0 + hash2(tx, ty * 3 + k) * 26.0;
            leaf_shape(
                vec2(x, y),
                hash2(k, tx) * std::f32::consts::TAU,
                7.0,
                alpha(leaf_col(k as u32 + 3), 0.85 * look.leaves),
            );
        }
    }
    let wet = look.snow_cover.max(look.wet * 0.5);
    if wet > 0.05 {
        // wet footprints walking in from the door (the bottom edge of the tile)
        for k in 0..4 {
            let x = p.x + 22.0 + (k % 2) as f32 * 11.0 + hash2(k, tx) * 3.0;
            let y = p.y + TILE - 7.0 - k as f32 * 12.0;
            let a = 0.22 * wet * (1.0 - k as f32 * 0.18);
            ellipse(x, y, 2.6, 4.2, alpha(Color::from_hex(0x2a2238), a));
            ellipse(x, y - 5.0, 2.0, 1.8, alpha(Color::from_hex(0x2a2238), a));
            if look.snow_cover > 0.05 {
                ellipse(
                    x,
                    y,
                    1.4,
                    2.4,
                    alpha(SNOW, 0.3 * look.snow_cover * (1.0 - k as f32 * 0.25)),
                );
            }
        }
    }
}

// ───────────────────────── breath ─────────────────────────

/// Where a person's breath leaves the face (world pixels).
pub fn breath_anchor(look: &PersonLook, pose: &PersonPose) -> Vec2 {
    let k = look.height;
    let head = match pose.pose {
        Pose::Sit => 52.0,
        _ => 58.0,
    };
    let fwd = match pose.facing {
        Facing::Left => vec2(-9.0, 0.0),
        Facing::Right => vec2(9.0, 0.0),
        Facing::Down => vec2(2.0, 4.0),
        Facing::Up => vec2(0.0, -6.0),
    };
    pose.feet - vec2(0.0, head * k) + fwd * k
}

/// Breath puffs in the cold (draw after lighting, for people outside). `strength` 0..1
/// (`SeasonLook::breath`). Each person breathes on their own rhythm (seeded by `look`).
pub fn draw_breath(look: &PersonLook, pose: &PersonPose, t: f32, strength: f32) {
    if strength <= 0.01 || pose.facing == Facing::Up {
        return;
    }
    let a = breath_anchor(look, pose);
    let period = 2.8 + look.phase * 1.2;
    let dir = match pose.facing {
        Facing::Left => vec2(-1.0, -0.35),
        Facing::Right => vec2(1.0, -0.35),
        _ => vec2(0.25, -0.6),
    };
    let moving = matches!(pose.pose, Pose::Walk | Pose::Carry);
    for puff in 0..2 {
        let age = ((t + look.phase * 5.0 + puff as f32 * period * 0.5) % period) / period;
        if age > 0.55 {
            continue;
        }
        let k = age / 0.55;
        let travel = 14.0 * k * look.height + if moving { 4.0 * k } else { 0.0 };
        let p = a + dir * travel + vec2(0.0, -6.0 * k * k);
        let fade = (k * 5.0).min(1.0) * (1.0 - k);
        let r = (3.5 + 10.0 * k) * look.height;
        glow(
            p.x,
            p.y,
            r,
            Color::new(0.93, 0.93, 1.0, 0.5 * fade * strength),
        );
        glow(
            p.x - dir.x * 2.0,
            p.y + 1.0,
            r * 0.6,
            Color::new(1.0, 1.0, 1.0, 0.25 * fade * strength),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip() {
        for s in Season::ALL {
            assert_eq!(Season::from_id(s.id()), Some(s));
        }
        assert_eq!(Season::from_id("summer"), None);
    }

    #[test]
    fn blend_is_smooth_and_hits_both_ends() {
        let a = Season::LateAutumn;
        let b = Season::FirstSnow;
        assert_eq!(Season::blend(a, b, 0.0), a.look());
        assert_eq!(Season::blend(a, b, 1.0), b.look());
        let mut prev = Season::blend(a, b, 0.0).snowfall;
        for i in 1..=20 {
            let v = Season::blend(a, b, i as f32 / 20.0).snowfall;
            assert!(v >= prev);
            prev = v;
        }
    }

    #[test]
    fn phases_progress_towards_winter() {
        let looks: Vec<SeasonLook> = Season::ALL.iter().map(|s| s.look()).collect();
        assert!(looks[0].leaves > looks[1].leaves && looks[1].leaves > looks[3].leaves);
        assert!(looks[1].frost > looks[0].frost && looks[3].frost > looks[1].frost);
        assert!(looks[2].festive > 0.5 && looks[0].festive < 0.01);
        assert!(looks[3].snowfall > 0.5 && looks[2].snowfall < 0.01);
        // the static key separates the phases that look different on the pavement
        assert_ne!(looks[0].static_key(), looks[3].static_key());
        let f = SeasonLook::from_flags(false, 0.8, true, true, true, true);
        assert_eq!(f.snow_cover, 1.0);
        assert_eq!(f.frost, 0.8);
    }
}
