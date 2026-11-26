//! Festive decorations (`design/handson.md` §11.1, `m_hang_garlands`): garlands of warm
//! bulbs along windows, walls and the counter, white-gold fairy lights, a small spruce,
//! Grandma's box of decorations and the Advent wreath.
//!
//! Garlands show progressively: [`Garlands::draw`] takes `progress` 0..1 — span by span,
//! the cord unrolls and the bulbs pop on one after another, so Karolina can hang them one
//! hang spot at a time ([`Garlands::hang_spots`] → moment markers).
//!
//! ```ignore
//! // before lighting (cords, sockets — lit by the room):   garlands.draw(p, t);
//! // into the light map:  for l in garlands.lights(p) { lighting.add_light(l) }
//! // after lighting (emissive bulbs, not darkened):        garlands.draw_glow(p, t);
//! ```

use std::f32::consts::{PI, TAU};

use macroquad::prelude::*;

use super::colors as c;
use super::draw::*;
use super::furniture::COUNTER_H;
use super::light::Light;
use super::outside::world_text;
use super::season::star;
use crate::map::TILE;

/// The look of a garland.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GarlandStyle {
    /// Big warm bulbs on a dark cord (Grandma's garland from the box).
    #[default]
    WarmBulbs,
    /// Tiny white-gold fairy lights on a thin wire (the `winter_lights` theme).
    FairyLights,
    /// A spruce bough with red ribbons and warm bulbs.
    Bough,
}

/// One hanging span from `a` to `b` (world pixels), sagging by `sag` px in the middle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GarlandSpan {
    pub a: Vec2,
    pub b: Vec2,
    pub sag: f32,
}

impl GarlandSpan {
    pub fn new(a: Vec2, b: Vec2, sag: f32) -> GarlandSpan {
        GarlandSpan { a, b, sag }
    }

    /// The point at `u` 0..1 along the span (a parabola sag).
    pub fn at(&self, u: f32) -> Vec2 {
        self.a.lerp(self.b, u) + vec2(0.0, self.sag * 4.0 * u * (1.0 - u))
    }

    fn len(&self) -> f32 {
        self.a.distance(self.b) + self.sag * 0.6
    }
}

/// A set of garland spans hung in order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Garlands {
    pub spans: Vec<GarlandSpan>,
    pub style: GarlandStyle,
}

impl Garlands {
    pub fn new(spans: Vec<GarlandSpan>, style: GarlandStyle) -> Garlands {
        Garlands { spans, style }
    }

    /// Spans over runs of adjacent window panes (`window_glass_rect`s), scalloped:
    /// one swag per pane, hung just above the frames.
    pub fn over_windows(glass: &[Rect], style: GarlandStyle) -> Garlands {
        let mut g: Vec<Rect> = glass.to_vec();
        g.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
        let mut spans = Vec::new();
        for r in g {
            let y = r.y - 7.0;
            spans.push(GarlandSpan::new(
                vec2(r.x - 5.0, y),
                vec2(r.x + r.w + 5.0, y),
                9.0,
            ));
        }
        Garlands { spans, style }
    }

    /// Swags along the front of a counter (`x0..x1` world px, `front_y` — the bottom of
    /// the counter's footprint), `swags` scallops.
    pub fn along_counter(x0: f32, x1: f32, front_y: f32, swags: usize) -> Vec<GarlandSpan> {
        let n = swags.max(1);
        let y = front_y - 6.0 - COUNTER_H + 4.0;
        (0..n)
            .map(|i| {
                let a = x0 + (x1 - x0) * i as f32 / n as f32;
                let b = x0 + (x1 - x0) * (i + 1) as f32 / n as f32;
                GarlandSpan::new(vec2(a, y), vec2(b, y), 7.0)
            })
            .collect()
    }

    /// The middle of every span (for "hang it here" moment markers).
    pub fn hang_spots(&self) -> Vec<Vec2> {
        self.spans.iter().map(|s| s.at(0.5)).collect()
    }

    /// How much of span `i` is up at overall `progress` 0..1.
    pub fn span_progress(&self, i: usize, progress: f32) -> f32 {
        let n = self.spans.len().max(1) as f32;
        (progress.clamp(0.0, 1.0) * n - i as f32).clamp(0.0, 1.0)
    }

    fn bulb_spacing(&self) -> f32 {
        match self.style {
            GarlandStyle::WarmBulbs => 11.0,
            GarlandStyle::FairyLights => 6.0,
            GarlandStyle::Bough => 13.0,
        }
    }

    /// Bulb positions of span `i` with its local progress `k`: (pos, index, popped 0..1).
    fn bulbs(&self, i: usize, k: f32) -> Vec<(Vec2, u32, f32)> {
        let s = &self.spans[i];
        let n = (s.len() / self.bulb_spacing()).round().max(2.0) as u32;
        let mut v = Vec::with_capacity(n as usize);
        for j in 0..n {
            let u = (j as f32 + 0.5) / n as f32;
            // the cord unrolls over the first 70 % of `k`, bulbs pop just behind it
            let reach = k / 0.7;
            let pop = ((reach - u) * 6.0).clamp(0.0, 1.0);
            if pop > 0.0 {
                v.push((s.at(u), i as u32 * 97 + j, pop));
            }
        }
        v
    }

    /// Colours of bulb `j`.
    fn bulb_col(&self, j: u32) -> Color {
        match self.style {
            GarlandStyle::FairyLights => mix(Color::from_hex(0xfff6dc), c::AMBER, hash01(j) * 0.4),
            _ => {
                const COLS: [u32; 5] = [0xffc46b, 0xff9a5a, 0xfff0c0, 0xf0607a, 0xa5b4fc];
                Color::from_hex(COLS[(j % 5) as usize])
            }
        }
    }

    fn twinkle(&self, j: u32, t: f32) -> f32 {
        let sp = if self.style == GarlandStyle::FairyLights {
            2.6
        } else {
            1.1
        };
        0.75 + 0.25 * (t * sp + hash01(j) * TAU).sin()
    }

    /// Cords, sockets, boughs and ribbons (draw before lighting, in the world).
    pub fn draw(&self, progress: f32, t: f32) {
        for i in 0..self.spans.len() {
            let k = self.span_progress(i, progress);
            if k <= 0.0 {
                continue;
            }
            let s = self.spans[i];
            let reach = (k / 0.7).min(1.0);
            let steps = 16;
            let mut prev = s.at(0.0);
            for j in 1..=steps {
                let u = j as f32 / steps as f32 * reach;
                let p = s.at(u);
                match self.style {
                    GarlandStyle::Bough => {
                        capsule(prev, p, 4.0, Color::from_hex(0x2f6a4f));
                    }
                    GarlandStyle::FairyLights => {
                        draw_line(
                            prev.x,
                            prev.y,
                            p.x,
                            p.y,
                            0.6,
                            alpha(Color::from_hex(0xd8c8a0), 0.6),
                        );
                    }
                    GarlandStyle::WarmBulbs => {
                        draw_line(prev.x, prev.y, p.x, p.y, 1.2, Color::from_hex(0x2a2433));
                    }
                }
                prev = p;
            }
            if self.style == GarlandStyle::Bough {
                // needles: little dark and light ticks along the bough
                let n = (s.len() * reach / 4.0) as u32;
                for j in 0..n {
                    let u = j as f32 / n.max(1) as f32 * reach;
                    let p = s.at(u);
                    let a = hash01(j * 7 + i as u32) * TAU;
                    let d = vec2(a.cos(), a.sin()) * 4.5;
                    let col = if j % 2 == 0 {
                        Color::from_hex(0x3f8a5f)
                    } else {
                        Color::from_hex(0x24523c)
                    };
                    draw_line(p.x, p.y, p.x + d.x, p.y + d.y, 1.2, col);
                }
                // red bows at both ends
                for (u, ok) in [(0.0_f32, true), (1.0, reach >= 1.0)] {
                    if ok {
                        bow(s.at(u), 1.0);
                    }
                }
            }
            // sockets
            if self.style != GarlandStyle::FairyLights {
                for (p, _, pop) in self.bulbs(i, k) {
                    rrect(
                        p.x - 1.4,
                        p.y - 1.0,
                        2.8,
                        3.0 * pop,
                        0.8,
                        Color::from_hex(0x3a3344),
                    );
                }
            }
            let _ = t;
        }
    }

    /// Warm light pools for the light map: a few per span, not one per bulb.
    pub fn lights(&self, progress: f32) -> Vec<Light> {
        let mut v = Vec::new();
        for i in 0..self.spans.len() {
            let k = self.span_progress(i, progress);
            if k <= 0.05 {
                continue;
            }
            let s = self.spans[i];
            let (radius, inten) = match self.style {
                GarlandStyle::FairyLights => (1.2 * TILE, 0.32),
                _ => (1.4 * TILE, 0.4),
            };
            for u in [0.25_f32, 0.75] {
                if k / 0.7 >= u {
                    let col = if self.style == GarlandStyle::FairyLights {
                        Color::from_hex(0xfff0d0)
                    } else {
                        c::AMBER
                    };
                    v.push(
                        Light::new(s.at(u) + vec2(0.0, 6.0), radius, col, inten * k.min(1.0))
                            .flicker(0.04),
                    );
                }
            }
        }
        v
    }

    /// The glowing bulbs (draw after lighting — they are light sources).
    pub fn draw_glow(&self, progress: f32, t: f32) {
        for i in 0..self.spans.len() {
            let k = self.span_progress(i, progress);
            if k <= 0.0 {
                continue;
            }
            for (p, j, pop) in self.bulbs(i, k) {
                let col = self.bulb_col(j);
                let tw = self.twinkle(j, t);
                // easeOutBack pop
                let e = 1.0 + 2.7 * (pop - 1.0).powi(3) + 1.7 * (pop - 1.0).powi(2);
                match self.style {
                    GarlandStyle::FairyLights => {
                        glow(p.x, p.y + 1.0, 5.0 * e, alpha(col, 0.45 * tw));
                        ellipse(
                            p.x,
                            p.y + 1.0,
                            1.1 * e,
                            1.1 * e,
                            alpha(tint_light(col, 0.5), tw),
                        );
                    }
                    _ => {
                        let b = p + vec2(0.0, 4.0);
                        glow(b.x, b.y, 9.0 * e, alpha(col, 0.38 * tw));
                        ellipse_v(b.x, b.y, 2.3 * e, 3.0 * e, tint_light(col, 0.5), col);
                        ellipse(b.x - 0.6, b.y - 1.0, 0.7, 0.9, alpha(WHITE, 0.8 * tw));
                    }
                }
            }
        }
    }
}

/// A small red ribbon bow (scale `k`).
pub fn bow(p: Vec2, k: f32) {
    let red = Color::from_hex(0xc23a4a);
    poly(&[p, p + vec2(-6.0, -4.0) * k, p + vec2(-6.0, 3.0) * k], red);
    poly(&[p, p + vec2(6.0, -4.0) * k, p + vec2(6.0, 3.0) * k], red);
    draw_line(
        p.x,
        p.y,
        p.x - 2.5 * k,
        p.y + 8.0 * k,
        1.6 * k,
        shade(red, 0.85),
    );
    draw_line(
        p.x,
        p.y,
        p.x + 2.5 * k,
        p.y + 8.0 * k,
        1.6 * k,
        shade(red, 0.85),
    );
    ellipse(p.x, p.y, 1.8 * k, 1.8 * k, tint_light(red, 0.2));
}

// ───────────────────────── the Advent wreath ─────────────────────────

/// The Advent wreath lying on a surface (`p` — its centre on the counter top),
/// `lit` candles 0..=4 (the 1st lit on night 21, the 2nd on night 28).
pub fn draw_advent_wreath(p: Vec2, lit: u8, t: f32) {
    let (rx, ry) = (17.0, 8.0);
    soft_shadow(p.x + 2.0, p.y + 2.0, rx + 4.0, ry + 3.0, 0.4);
    // the spruce ring: overlapping sprigs
    for i in 0..22 {
        let a = i as f32 / 22.0 * TAU;
        let q = p + vec2(a.cos() * rx, a.sin() * ry);
        let col = if i % 3 == 0 {
            Color::from_hex(0x3f8a5f)
        } else if i % 3 == 1 {
            Color::from_hex(0x2f6a4f)
        } else {
            Color::from_hex(0x24523c)
        };
        ellipse_v(
            q.x,
            q.y - 2.0,
            5.5,
            4.0,
            tint_light(col, 0.1),
            shade(col, 0.8),
        );
    }
    // red berries and a ribbon
    for i in 0..6 {
        let a = i as f32 / 6.0 * TAU + 0.4;
        let q = p + vec2(a.cos() * rx * 1.02, a.sin() * ry * 1.02 - 3.0);
        ellipse(q.x, q.y, 1.6, 1.6, Color::from_hex(0xd2404e));
    }
    bow(p + vec2(0.0, ry - 1.0), 0.8);
    // four candles (violet, Advent style), the back ones first
    let spots = [
        vec2(-rx * 0.55, -ry * 0.55),
        vec2(rx * 0.55, -ry * 0.55),
        vec2(-rx * 0.6, ry * 0.35),
        vec2(rx * 0.6, ry * 0.35),
    ];
    for (k, d) in spots.iter().enumerate() {
        let b = p + *d;
        let h = 13.0;
        rrect_v(
            b.x - 2.6,
            b.y - h,
            5.2,
            h,
            1.5,
            Color::from_hex(0xb7a6e6),
            Color::from_hex(0x7c6ab8),
        );
        ellipse(b.x, b.y - h, 2.6, 1.0, Color::from_hex(0xcbbcf0));
        if (k as u8) < lit {
            let fl = noise1(t * 7.0 + k as f32 * 3.0, k as u32 + 9) * 0.4 + 0.6;
            glow(b.x, b.y - h - 3.0, 8.0 * fl, alpha(c::AMBER, 0.6));
            ellipse(b.x, b.y - h - 3.0, 1.3, 2.6 * fl, Color::from_hex(0xffd27a));
            ellipse(b.x, b.y - h - 2.4, 0.6, 1.1, WHITE);
        } else {
            draw_line(
                b.x,
                b.y - h,
                b.x,
                b.y - h - 2.0,
                0.8,
                Color::from_hex(0x2a2433),
            );
        }
    }
}

/// Lights of the lit wreath candles.
pub fn advent_wreath_lights(p: Vec2, lit: u8) -> Vec<Light> {
    (0..lit.min(4))
        .map(|k| {
            let dx = if k % 2 == 0 { -9.0 } else { 9.0 };
            Light::new(p + vec2(dx, -16.0), 1.3 * TILE, c::AMBER_DEEP, 0.4).flicker(0.12)
        })
        .collect()
}

// ───────────────────────── a small spruce & the box ─────────────────────────

/// A small potted spruce (furniture `spruce_tree`). `lights` — its bulbs are on,
/// `dressed` 0..1 — how many ornaments are hung (the tree can be decorated gradually).
pub(super) fn draw_spruce(o: Vec2, w: f32, d: f32, lights: bool, dressed: f32, t: f32) {
    let cx = o.x + w / 2.0;
    let floor_y = o.y + d * 0.7;
    soft_shadow(cx + 3.0, floor_y, 22.0, 8.0, 0.45);
    // a wooden tub with a red ribbon
    rrect_v(
        cx - 11.0,
        floor_y - 14.0,
        22.0,
        14.0,
        3.0,
        c::WOOD,
        c::WOOD_DARK,
    );
    rect_v(
        cx - 11.0,
        floor_y - 10.0,
        22.0,
        3.0,
        Color::from_hex(0xc23a4a),
        Color::from_hex(0x8a2a36),
    );
    ellipse(cx, floor_y - 14.0, 11.0, 3.0, Color::from_hex(0x3a2a24));
    // three tiers of soft boughs, the bottom one first, each with a scalloped hem
    let top_y = floor_y - 14.0 - 64.0;
    let tiers = [
        (floor_y - 16.0, top_y + 26.0, 27.0, 0.0_f32),
        (floor_y - 36.0, top_y + 11.0, 20.0, 1.0),
        (floor_y - 54.0, top_y, 13.0, 2.0),
    ];
    for (base_y, apex_y, half, k) in tiers {
        let apex = vec2(cx, apex_y);
        let col_t = mix(
            Color::from_hex(0x3f8a5f),
            Color::from_hex(0x5aa874),
            k * 0.3,
        );
        let col_b = Color::from_hex(0x24523c);
        let mut pts = Vec::with_capacity(12);
        pts.push(apex);
        for i in 0..=6 {
            let u = i as f32 / 6.0;
            let x = cx + half - 2.0 * half * u;
            let y = base_y + (u * PI * 3.0).sin().abs() * 3.5;
            pts.push(vec2(x, y));
        }
        poly_v(&pts, col_t, col_b);
        // a darker band under the next tier and a lavender rim on the window side
        draw_line(
            cx - half * 0.8,
            base_y + 1.0,
            cx + half * 0.8,
            base_y + 1.0,
            1.0,
            alpha(shade(col_b, 0.7), 0.6),
        );
        draw_line(
            apex.x - 1.0,
            apex.y + 3.0,
            cx - half * 0.9,
            base_y - 1.0,
            1.2,
            alpha(c::LAVENDER_RIM, 0.4),
        );
    }
    // ornaments
    let n = (8.0 * dressed.clamp(0.0, 1.0)).round() as u32;
    for i in 0..n {
        let u = hash01(i * 3 + 1);
        let v = 0.2 + hash01(i * 3 + 2) * 0.75;
        let y = floor_y - 18.0 - v * 52.0;
        let half = 22.0 * (1.0 - v * 0.75);
        let x = cx + (u - 0.5) * 2.0 * half * 0.8;
        let col = if i % 2 == 0 {
            Color::from_hex(0xd2404e)
        } else {
            Color::from_hex(0xe0b86a)
        };
        ellipse_v(x, y, 2.4, 2.4, tint_light(col, 0.3), shade(col, 0.8));
        ellipse(x - 0.7, y - 0.8, 0.6, 0.6, alpha(WHITE, 0.8));
    }
    // a string of bulbs spiralling down
    for i in 0..14 {
        let v = i as f32 / 13.0;
        let y = floor_y - 22.0 - (1.0 - v) * 50.0;
        let half = 24.0 * (0.25 + v * 0.75);
        let x = cx + (v * PI * 4.0 + 0.6).sin() * half * 0.85;
        if lights {
            let tw = 0.75 + 0.25 * (t * 1.4 + i as f32 * 1.3).sin();
            glow(x, y, 5.0, alpha(c::AMBER, 0.5 * tw));
            ellipse(x, y, 1.3, 1.3, Color::from_hex(0xfff0c0));
        } else {
            ellipse(x, y, 1.1, 1.1, Color::from_hex(0x8a7a6a));
        }
    }
    // the star
    let top = vec2(cx, top_y - 1.0);
    if lights {
        glow(top.x, top.y, 12.0, alpha(c::AMBER, 0.45));
    }
    star(top, 6.0, 2.6, 0.0, Color::from_hex(0xf2c860));
}

/// Grandma's cardboard box "Vánoce — L." (furniture `garland_box`). `open` — the lid is
/// off and a garland spills over the edge.
pub(super) fn draw_garland_box(o: Vec2, w: f32, d: f32, open: bool, t: f32) {
    let cx = o.x + w / 2.0;
    let (bw, bd, bh) = (40.0, 26.0, 20.0);
    let (x, y0) = (cx - bw / 2.0, o.y + d * 0.35);
    soft_shadow(cx + 3.0, y0 + bd - 2.0, bw * 0.6, 8.0, 0.4);
    let card = Color::from_hex(0xc8a070);
    rrect_v(
        x,
        y0 + bd - bh - 2.0,
        bw,
        bh + 2.0,
        2.0,
        card,
        shade(card, 0.7),
    );
    rrect_v(
        x,
        y0 - bh,
        bw,
        bd,
        2.0,
        tint_light(card, 0.12),
        shade(card, 0.9),
    );
    if open {
        // the dark inside, a coil of garland with bulbs
        rrect(
            x + 3.0,
            y0 - bh + 3.0,
            bw - 6.0,
            bd - 6.0,
            2.0,
            Color::from_hex(0x4a3428),
        );
        for i in 0..7 {
            let a = i as f32 / 7.0 * TAU;
            let p = vec2(cx + a.cos() * 11.0, y0 - bh + bd / 2.0 + a.sin() * 5.0);
            let tw = 0.7 + 0.3 * (t * 2.0 + i as f32).sin();
            glow(p.x, p.y, 4.0, alpha(c::AMBER, 0.4 * tw));
            ellipse(p.x, p.y, 1.4, 1.4, Color::from_hex(0xfff0c0));
        }
        let mut prev = vec2(x + bw - 4.0, y0 - bh + 4.0);
        for j in 1..6 {
            let p = vec2(
                x + bw - 4.0 + j as f32 * 2.0,
                y0 - bh + 4.0 + j as f32 * 4.0,
            );
            draw_line(prev.x, prev.y, p.x, p.y, 1.2, Color::from_hex(0x2a2433));
            prev = p;
        }
        // the lid leaning on the side
        rrect_v(
            x - 8.0,
            y0 - bh + 6.0,
            6.0,
            bd + 2.0,
            1.5,
            tint_light(card, 0.1),
            shade(card, 0.75),
        );
    } else {
        draw_line(
            cx,
            y0 - bh,
            cx,
            y0 - bh + bd,
            2.0,
            alpha(Color::from_hex(0xd9c8a0), 0.8),
        );
    }
    // the handwritten label on the front
    rrect(
        cx - 15.0,
        y0 + bd - bh + 3.0,
        30.0,
        11.0,
        1.5,
        Color::from_hex(0xf3ead8),
    );
    world_text(
        "Vánoce — L.",
        vec2(cx, y0 + bd - bh + 11.5),
        7.5,
        Color::from_hex(0x6a2430),
        true,
    );
}

/// A glass tip jar on the counter top (`p` — its bottom centre). `fill` 0..1 — coins
/// inside, `rhythm` 0..1 — the amber glow of a rush streak (§1.5).
pub fn draw_tip_jar(p: Vec2, fill: f32, rhythm: f32, t: f32) {
    let (w, h) = (16.0, 20.0);
    if rhythm > 0.01 {
        let pulse = 0.8 + (t * 3.0).sin() * 0.2;
        glow(
            p.x,
            p.y - h / 2.0,
            26.0,
            alpha(c::AMBER, 0.35 * rhythm * pulse),
        );
    }
    soft_shadow(p.x + 1.5, p.y, 10.0, 3.5, 0.35);
    // coins inside
    let f = fill.clamp(0.0, 1.0);
    if f > 0.01 {
        let ch = (h - 4.0) * f;
        rrect_v(
            p.x - w / 2.0 + 2.0,
            p.y - ch - 1.0,
            w - 4.0,
            ch,
            3.0,
            Color::from_hex(0xf2c860),
            Color::from_hex(0xb07a2a),
        );
        for i in 0..(f * 6.0) as u32 + 1 {
            let x = p.x - 4.0 + hash01(i * 3) * 8.0;
            let y = p.y - 3.0 - hash01(i * 3 + 1) * (ch - 3.0).max(0.0);
            ellipse(x, y, 2.2, 0.9, alpha(Color::from_hex(0xfff0c0), 0.7));
        }
    }
    // the glass
    rrect_v(
        p.x - w / 2.0,
        p.y - h,
        w,
        h,
        4.0,
        Color::new(0.85, 0.88, 1.0, 0.22),
        Color::new(0.7, 0.72, 0.95, 0.32),
    );
    ellipse(p.x, p.y - h, w / 2.0, 2.4, Color::new(0.9, 0.92, 1.0, 0.45));
    ellipse(
        p.x,
        p.y - h,
        w / 2.0 - 1.5,
        1.5,
        Color::new(0.2, 0.18, 0.3, 0.5),
    );
    draw_line(
        p.x - w / 2.0 + 2.5,
        p.y - h + 4.0,
        p.x - w / 2.0 + 2.5,
        p.y - 4.0,
        1.4,
        alpha(WHITE, 0.45),
    );
    // a little paper label with a heart
    rrect(
        p.x - 5.0,
        p.y - h * 0.55,
        10.0,
        7.0,
        1.2,
        Color::from_hex(0xf3ead8),
    );
    ellipse(p.x - 1.2, p.y - h * 0.55 + 3.0, 1.3, 1.3, c::ROSE);
    ellipse(p.x + 1.2, p.y - h * 0.55 + 3.0, 1.3, 1.3, c::ROSE);
    poly(
        &[
            vec2(p.x - 2.4, p.y - h * 0.55 + 3.4),
            vec2(p.x + 2.4, p.y - h * 0.55 + 3.4),
            vec2(p.x, p.y - h * 0.55 + 5.8),
        ],
        c::ROSE,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn garlands_go_up_span_by_span() {
        let g = Garlands::new(
            vec![
                GarlandSpan::new(vec2(0.0, 0.0), vec2(100.0, 0.0), 10.0),
                GarlandSpan::new(vec2(120.0, 0.0), vec2(220.0, 0.0), 10.0),
            ],
            GarlandStyle::WarmBulbs,
        );
        assert_eq!(g.span_progress(0, 0.25), 0.5);
        assert_eq!(g.span_progress(1, 0.25), 0.0);
        assert_eq!(g.span_progress(1, 1.0), 1.0);
        assert!(g.bulbs(0, 0.0).is_empty());
        let half = g.bulbs(0, 0.35).len();
        let full = g.bulbs(0, 1.0).len();
        assert!(half > 0 && half < full);
        assert!(g.lights(0.0).is_empty());
        assert_eq!(g.lights(1.0).len(), 4);
        // the sag is deepest in the middle
        let s = g.spans[0];
        assert!(s.at(0.5).y > s.at(0.1).y);
        assert_eq!(g.hang_spots().len(), 2);
    }

    #[test]
    fn window_garlands_follow_the_panes() {
        let glass = [
            Rect::new(10.0, 50.0, 54.0, 40.0),
            Rect::new(74.0, 50.0, 54.0, 40.0),
        ];
        let g = Garlands::over_windows(&glass, GarlandStyle::FairyLights);
        assert_eq!(g.spans.len(), 2);
        assert!(g.spans.iter().all(|s| s.a.y < 50.0));
        assert_eq!(Garlands::along_counter(0.0, 300.0, 600.0, 3).len(), 3);
        assert_eq!(advent_wreath_lights(Vec2::ZERO, 2).len(), 2);
    }
}
