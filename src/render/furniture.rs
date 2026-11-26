//! Furniture and appliances in 3/4 view: the top face and the front are visible.
//!
//! Oblique projection: a floor point (x, y) at height z is drawn at (x, y − z).
//! So an object's y sort key is the bottom edge of its footprint on the floor
//! (`furniture_sort_key`). All sizes are in world pixels (tile = 64).

use std::f32::consts::PI;

use macroquad::prelude::*;

use super::colors as c;
use super::draw::*;
use super::light::Light;
use super::look::{Expression, Facing, PersonLook};
use super::person::{draw_cup, draw_portrait, draw_steam};
use crate::map::TILE;

/// A piece's state this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FurnitureState {
    /// On / working (the lamp is lit, the coffee machine brews, the candle burns…).
    pub active: bool,
    /// Dirt 0..1 (crumbs on the table, dishes in the sink, a full bucket).
    pub dirty: f32,
    /// Animation time, s.
    pub t: f32,
    /// Where the piece "faces" (a chair: where the sitter looks). Default `Down`.
    pub facing: Facing,
}

impl Default for FurnitureState {
    fn default() -> Self {
        Self {
            active: true,
            dirty: 0.0,
            t: 0.0,
            facing: Facing::Down,
        }
    }
}

/// Every id `draw_furniture` can draw (the contract with the content data).
pub const FURNITURE_IDS: [&str; 32] = [
    "table_round_2",
    "table_square_4",
    "chair",
    "stool",
    "bar_counter",
    "counter_corner",
    "coffee_machine",
    "espresso_grinder",
    "stove",
    "oven",
    "sink",
    "fridge",
    "dish_rack",
    "register",
    "menu_holo_board",
    "plant_small",
    "plant_tall",
    "floor_lamp",
    "pendant_lamp",
    "sofa",
    "armchair",
    "bookshelf",
    "jukebox",
    "rug",
    "trash_bin",
    "coat_rack",
    "window_seat",
    "drone_dock",
    "grandma_photo",
    "radio",
    "candle",
    "bucket",
];

/// Height of the counter / kitchen units' work surface.
pub const COUNTER_H: f32 = 30.0;

/// A piece's y sort key. A rug is always under everything, a pendant lamp above everything,
/// a photo on the wall right behind the wall.
pub fn furniture_sort_key(sprite: &str, tile_origin: Vec2, size: (u8, u8)) -> f32 {
    let d = size.1.max(1) as f32 * TILE;
    match sprite {
        "rug" => tile_origin.y - 100_000.0,
        "pendant_lamp" => tile_origin.y + 100_000.0,
        "grandma_photo" => tile_origin.y,
        _ => tile_origin.y + d,
    }
}

/// Light sources a piece gives (lamps, screens, the stove, a candle…).
/// Add them to `Lighting` every frame.
pub fn furniture_lights(sprite: &str, o: Vec2, size: (u8, u8), st: &FurnitureState) -> Vec<Light> {
    let (w, d) = dims(size);
    let cx = o.x + w / 2.0;
    let cy = o.y + d / 2.0;
    let mut v = Vec::new();
    if !st.active {
        return v;
    }
    match sprite {
        "floor_lamp" => {
            v.push(Light::new(vec2(cx, cy - 10.0), 3.4 * TILE, c::AMBER, 0.85).flicker(0.03));
            v.push(Light::new(vec2(cx, cy - 86.0), 0.9 * TILE, c::AMBER, 0.7));
        }
        "pendant_lamp" => {
            v.push(Light::new(vec2(cx, cy - 18.0), 2.9 * TILE, c::AMBER, 0.95).flicker(0.025));
            v.push(Light::new(vec2(cx, cy - 92.0), 0.8 * TILE, c::AMBER, 0.8));
        }
        "candle" | "table_round_2" | "table_square_4" => {
            let h = if sprite == "candle" { 8.0 } else { 34.0 };
            v.push(Light::new(vec2(cx, cy - h), 1.5 * TILE, c::AMBER_DEEP, 0.55).flicker(0.12));
        }
        "coffee_machine" => {
            v.push(Light::new(
                vec2(cx, o.y + d * 0.4 - COUNTER_H - 22.0),
                1.4 * TILE,
                c::INDIGO_GLOW,
                0.45,
            ));
        }
        "register" => {
            v.push(Light::new(
                vec2(cx, o.y + d * 0.5 - COUNTER_H - 16.0),
                1.1 * TILE,
                c::INDIGO_GLOW,
                0.45,
            ));
        }
        "menu_holo_board" => {
            v.push(Light::new(vec2(cx, cy - 60.0), 2.0 * TILE, c::INDIGO_GLOW, 0.55).flicker(0.08));
        }
        "jukebox" => {
            v.push(Light::new(vec2(cx, cy - 40.0), 2.4 * TILE, c::VIOLET_SOFT, 0.6).flicker(0.05));
        }
        "stove" => {
            v.push(
                Light::new(vec2(cx, cy - COUNTER_H), 1.3 * TILE, c::AMBER_DEEP, 0.45).flicker(0.1),
            );
        }
        "oven" => {
            v.push(Light::new(
                vec2(cx, o.y + d),
                1.3 * TILE,
                c::AMBER_DEEP,
                0.5,
            ));
        }
        "fridge" => {
            v.push(Light::new(
                vec2(cx, o.y + d),
                0.8 * TILE,
                c::INDIGO_SOFT,
                0.2,
            ));
        }
        "radio" => {
            v.push(Light::new(vec2(cx, cy - 30.0), 0.9 * TILE, c::AMBER, 0.4));
        }
        "drone_dock" => {
            v.push(Light::new(vec2(cx, cy), 1.2 * TILE, c::INDIGO_GLOW, 0.4).flicker(0.15));
        }
        "grandma_photo" => {
            v.push(Light::new(vec2(cx, o.y - 30.0), 1.0 * TILE, c::AMBER, 0.35).flicker(0.1));
        }
        _ => {}
    }
    v
}

fn dims(size: (u8, u8)) -> (f32, f32) {
    (size.0.max(1) as f32 * TILE, size.1.max(1) as f32 * TILE)
}

// ───────────────────────── 3/4 primitives ─────────────────────────

/// A box: footprint on the floor (x, y0, w, d), height h. Draws the shadow, front, top, edge.
#[allow(clippy::too_many_arguments)]
fn box3(x: f32, y0: f32, w: f32, d: f32, h: f32, top: Color, front: Color, r: f32) {
    let y1 = y0 + d;
    soft_shadow(
        x + w / 2.0 + 3.0,
        y1 - d * 0.35,
        w * 0.62,
        d * 0.55 + 4.0,
        0.42,
    );
    rrect_v(x, y1 - h - r, w, h + r, r, front, shade(front, 0.66));
    rrect_v(x, y0 - h, w, d, r, tint_light(top, 0.05), top);
    // a light front edge + a lavender glint along the back edge
    draw_line(
        x + r,
        y1 - h,
        x + w - r,
        y1 - h,
        1.6,
        alpha(tint_light(top, 0.45), 0.7),
    );
    draw_line(
        x + r,
        y0 - h + 0.8,
        x + w - r,
        y0 - h + 0.8,
        1.2,
        alpha(c::LAVENDER_RIM, 0.35),
    );
}

/// A kitchen / bar unit at counter height (the shared base for appliances on the counter).
fn counter_base(o: Vec2, w: f32, d: f32, front: Color, top: Color) {
    let (x, y0) = (o.x, o.y + 6.0);
    let dd = d - 10.0;
    box3(x, y0, w, dd, COUNTER_H, top, front, 2.5);
    // plinth
    rect_v(
        x + 2.0,
        y0 + dd - 5.0,
        w - 4.0,
        5.0,
        shade(front, 0.45),
        shade(front, 0.35),
    );
}

/// A plant leaf — a convex "lens" from `base` at angle `ang`.
fn leaf(base: Vec2, ang: f32, len: f32, width: f32, col: Color) {
    let dir = vec2(ang.cos(), ang.sin());
    let n = vec2(-dir.y, dir.x);
    let mut pts = [Vec2::ZERO; 10];
    for (i, p) in pts.iter_mut().enumerate().take(5) {
        let t = i as f32 / 4.0;
        *p = base + dir * len * t + n * width * (t * PI).sin();
    }
    for i in 0..5 {
        let t = 1.0 - i as f32 / 4.0;
        pts[5 + i] = base + dir * len * t - n * width * 0.7 * (t * PI).sin();
    }
    poly_v(&pts, tint_light(col, 0.12), shade(col, 0.8));
    let tip = base + dir * len * 0.85;
    draw_line(
        base.x,
        base.y,
        tip.x,
        tip.y,
        0.8,
        alpha(shade(col, 0.6), 0.6),
    );
}

fn pot(cx: f32, floor_y: f32, w: f32, h: f32) {
    soft_shadow(cx + 2.0, floor_y - 2.0, w * 0.75, 6.0, 0.4);
    rrect_v(
        cx - w / 2.0,
        floor_y - h,
        w,
        h,
        4.0,
        c::POT,
        shade(c::POT, 0.65),
    );
    ellipse_v(
        cx,
        floor_y - h,
        w / 2.0 + 1.5,
        4.0,
        tint_light(c::POT, 0.15),
        c::POT,
    );
    ellipse(
        cx,
        floor_y - h + 0.5,
        w / 2.0 - 1.5,
        2.6,
        Color::from_hex(0x3a2a24),
    );
}

/// Crumbs, cup rings and a forgotten cup — by how dirty it is.
fn mess(cx: f32, cy: f32, rx: f32, ry: f32, dirty: f32, seed: u32) {
    if dirty <= 0.05 {
        return;
    }
    let n = (dirty * 9.0) as u32;
    for i in 0..n {
        let a = hash01(seed + i * 3) * PI * 2.0;
        let rr = hash01(seed + i * 3 + 1).sqrt() * 0.8;
        let p = vec2(cx + a.cos() * rx * rr, cy + a.sin() * ry * rr);
        ellipse(p.x, p.y, 1.1, 0.8, Color::from_hex(0xd9b48a));
    }
    if dirty > 0.3 {
        arc(
            vec2(cx + rx * 0.3, cy - ry * 0.1),
            4.0,
            3.0,
            0.3,
            5.5,
            1.0,
            alpha(Color::from_hex(0x6b3e26), 0.55),
        );
    }
    if dirty > 0.6 {
        draw_cup(vec2(cx - rx * 0.35, cy + ry * 0.2), 0.9, 0.0, false);
    }
}

// ───────────────────────── drawing ─────────────────────────

/// Draw a piece. `tile_origin` — the top-left of its footprint in world pixels,
/// `size` — the footprint in tiles. An unknown id → a neat placeholder.
pub fn draw_furniture(sprite: &str, tile_origin: Vec2, size: (u8, u8), state: &FurnitureState) {
    let o = tile_origin;
    let (w, d) = dims(size);
    let cx = o.x + w / 2.0;
    let cy = o.y + d / 2.0;
    let t = state.t;
    let seed = (o.x as i32 as u32).wrapping_mul(31) ^ (o.y as i32 as u32).wrapping_mul(17);
    match sprite {
        "table_round_2" => {
            let h = 28.0;
            let (rx, ry) = (w * 0.38, d * 0.32);
            soft_shadow(cx + 3.0, cy + 4.0, rx * 1.05, ry * 1.05, 0.45);
            ellipse_v(
                cx,
                cy + 1.0,
                12.0,
                4.5,
                c::WOOD_DARK,
                shade(c::WOOD_DARK, 0.7),
            );
            rrect_v(
                cx - 3.0,
                cy - h,
                6.0,
                h,
                2.0,
                c::WOOD_DARK,
                shade(c::WOOD_DARK, 0.6),
            );
            ellipse_v(
                cx,
                cy - h + 3.5,
                rx,
                ry,
                shade(c::WOOD, 0.75),
                shade(c::WOOD_DARK, 0.8),
            );
            ellipse_v(cx, cy - h, rx, ry, tint_light(c::WOOD_LIGHT, 0.08), c::WOOD);
            arc(
                vec2(cx, cy - h),
                rx - 1.0,
                ry - 1.0,
                PI * 1.05,
                PI * 1.6,
                1.2,
                alpha(c::LAVENDER_RIM, 0.4),
            );
            mess(cx, cy - h, rx * 0.8, ry * 0.7, state.dirty, seed);
            if state.active {
                draw_candle(vec2(cx + rx * 0.35, cy - h - ry * 0.15), t, seed);
            }
        }
        "table_square_4" => {
            let h = 28.0;
            let (x, y0, tw, td) = (o.x + 7.0, o.y + 8.0, w - 14.0, d - 18.0);
            soft_shadow(cx + 3.0, y0 + td * 0.6, tw * 0.6, td * 0.55, 0.45);
            for (lx, ly) in [(x + 5.0, y0 + 4.0), (x + tw - 9.0, y0 + 4.0)] {
                rect_v(
                    lx,
                    ly - h,
                    4.0,
                    h,
                    shade(c::WOOD_DARK, 0.7),
                    shade(c::WOOD_DARK, 0.5),
                );
            }
            for (lx, ly) in [(x + 4.0, y0 + td), (x + tw - 9.0, y0 + td)] {
                rect_v(lx, ly - h, 5.0, h, c::WOOD_DARK, shade(c::WOOD_DARK, 0.6));
            }
            rrect_v(
                x,
                y0 + td - h - 1.0,
                tw,
                6.0,
                2.0,
                shade(c::WOOD, 0.8),
                shade(c::WOOD_DARK, 0.8),
            );
            rrect_v(
                x,
                y0 - h,
                tw,
                td,
                4.0,
                tint_light(c::WOOD_LIGHT, 0.08),
                c::WOOD,
            );
            draw_line(
                x + 4.0,
                y0 - h + 1.0,
                x + tw - 4.0,
                y0 - h + 1.0,
                1.2,
                alpha(c::LAVENDER_RIM, 0.4),
            );
            mess(
                cx,
                y0 + td / 2.0 - h,
                tw * 0.35,
                td * 0.3,
                state.dirty,
                seed,
            );
            if state.active {
                draw_candle(vec2(cx, y0 + td * 0.45 - h), t, seed);
            }
        }
        "chair" => draw_chair(o, state.facing),
        "stool" => {
            let h = 26.0;
            soft_shadow(cx + 2.0, cy + 8.0, 13.0, 6.0, 0.4);
            rect_v(cx - 2.0, cy + 6.0 - h, 4.0, h, c::STEEL, c::STEEL_DARK);
            ellipse(cx, cy + 6.0, 9.0, 3.0, c::STEEL_DARK);
            ellipse_v(
                cx,
                cy + 6.0 - h + 3.0,
                12.0,
                6.0,
                shade(c::FABRIC_PLUM, 0.7),
                shade(c::FABRIC_PLUM, 0.55),
            );
            ellipse_v(
                cx,
                cy + 6.0 - h,
                12.0,
                6.0,
                tint_light(c::FABRIC_PLUM, 0.15),
                c::FABRIC_PLUM,
            );
        }
        "bar_counter" | "counter_corner" => {
            let corner = sprite == "counter_corner";
            let (x, y0, dd) = (o.x, o.y + 6.0, d - 10.0);
            let r = if corner { 6.0 } else { 1.2 };
            soft_shadow(cx + 3.0, y0 + dd * 0.7, w * 0.6, dd * 0.5, 0.42);
            rrect_v(
                x,
                y0 + dd - COUNTER_H - r,
                w,
                COUNTER_H + r,
                r,
                c::WOOD_PLUM,
                shade(c::WOOD_PLUM, 0.6),
            );
            // vertical slats
            let n = (w / 10.0) as i32;
            for i in 1..n {
                let lx = x + i as f32 * w / n as f32;
                draw_line(
                    lx,
                    y0 + dd - COUNTER_H + 3.0,
                    lx,
                    y0 + dd - 5.0,
                    1.0,
                    alpha(shade(c::WOOD_PLUM, 0.6), 0.7),
                );
            }
            // brass foot rail
            draw_line(
                x + 2.0,
                y0 + dd - 8.0,
                x + w - 2.0,
                y0 + dd - 8.0,
                2.0,
                Color::from_hex(0xd9a85c),
            );
            rrect_v(
                x - 1.0,
                y0 - COUNTER_H - 2.0,
                w + 2.0,
                dd + 2.0,
                r,
                tint_light(c::WOOD_LIGHT, 0.12),
                c::WOOD_LIGHT,
            );
            draw_line(
                x + r,
                y0 + dd - COUNTER_H,
                x + w - r,
                y0 + dd - COUNTER_H,
                2.0,
                alpha(tint_light(c::WOOD_LIGHT, 0.5), 0.8),
            );
            draw_line(
                x,
                y0 - COUNTER_H - 1.0,
                x + w,
                y0 - COUNTER_H - 1.0,
                1.2,
                alpha(c::LAVENDER_RIM, 0.35),
            );
            mess(
                cx,
                y0 + dd * 0.5 - COUNTER_H,
                w * 0.35,
                dd * 0.3,
                state.dirty,
                seed,
            );
        }
        "coffee_machine" => {
            counter_base(o, w, d, c::STEEL_DARK, c::STEEL_LIGHT);
            let top = o.y + 6.0 + (d - 10.0) * 0.45 - COUNTER_H;
            let (mw, mh) = (w * 0.72, 30.0);
            let mx = cx - mw / 2.0;
            soft_shadow(cx, top + 3.0, mw * 0.55, 6.0, 0.4);
            rrect_v(
                mx,
                top - mh,
                mw,
                mh,
                5.0,
                Color::from_hex(0xd8dcec),
                Color::from_hex(0x8a90aa),
            );
            rrect_v(
                mx,
                top - mh - 5.0,
                mw,
                8.0,
                4.0,
                Color::from_hex(0x6d74a0),
                Color::from_hex(0x4a5180),
            );
            // little screen
            let sc = if state.active {
                c::INDIGO_SOFT
            } else {
                c::GRAPHITE
            };
            rrect(cx - 7.0, top - mh + 4.0, 14.0, 5.0, 1.5, sc);
            if state.active {
                glow(cx, top - mh + 6.5, 10.0, alpha(c::INDIGO_GLOW, 0.5));
            }
            // group heads and portafilters
            for gx in [cx - mw * 0.22, cx + mw * 0.22] {
                rrect_v(gx - 4.0, top - 14.0, 8.0, 5.0, 2.0, c::STEEL, c::STEEL_DARK);
                draw_line(
                    gx,
                    top - 11.0,
                    gx + 9.0,
                    top - 9.0,
                    2.5,
                    Color::from_hex(0x2a2433),
                );
            }
            draw_cup(vec2(cx - mw * 0.22, top - 2.0), 0.8, t, state.active);
            rect_v(
                mx + 3.0,
                top - 3.0,
                mw - 6.0,
                3.0,
                c::STEEL_DARK,
                c::GRAPHITE,
            );
            if state.active {
                draw_steam(vec2(mx + mw - 3.0, top - mh - 2.0), t, 0.9);
            }
        }
        "espresso_grinder" => {
            counter_base(o, w, d, c::STEEL_DARK, c::STEEL_LIGHT);
            let top = o.y + 6.0 + (d - 10.0) * 0.45 - COUNTER_H;
            soft_shadow(cx, top + 2.0, 12.0, 5.0, 0.4);
            rrect_v(
                cx - 9.0,
                top - 24.0,
                18.0,
                24.0,
                4.0,
                Color::from_hex(0x3a3550),
                Color::from_hex(0x1f1c2e),
            );
            // bean hopper
            poly_v(
                &[
                    vec2(cx - 11.0, top - 44.0),
                    vec2(cx + 11.0, top - 44.0),
                    vec2(cx + 5.0, top - 24.0),
                    vec2(cx - 5.0, top - 24.0),
                ],
                Color::new(0.85, 0.88, 1.0, 0.45),
                Color::new(0.6, 0.62, 0.8, 0.5),
            );
            poly(
                &[
                    vec2(cx - 9.0, top - 36.0),
                    vec2(cx + 9.0, top - 36.0),
                    vec2(cx + 5.0, top - 25.0),
                    vec2(cx - 5.0, top - 25.0),
                ],
                Color::from_hex(0x5a3424),
            );
            ellipse(cx, top - 44.0, 11.0, 3.0, Color::from_hex(0x3a3550));
            draw_line(
                cx - 6.0,
                top - 42.0,
                cx - 3.0,
                top - 27.0,
                1.2,
                alpha(WHITE, 0.4),
            );
        }
        "stove" => {
            counter_base(o, w, d, c::STEEL_DARK, Color::from_hex(0x3a3d4e));
            let top_y = o.y + 6.0 - COUNTER_H;
            let dd = d - 10.0;
            for (i, (fx, fy)) in [(0.3, 0.3), (0.7, 0.3), (0.3, 0.72), (0.7, 0.72)]
                .iter()
                .enumerate()
            {
                let p = vec2(o.x + w * fx, top_y + dd * fy);
                ellipse(p.x, p.y, 9.0, 7.0, c::GRAPHITE);
                if state.active && i % 3 == 0 {
                    let fl = 0.75 + noise1(t * 8.0 + i as f32 * 3.0, seed) * 0.25;
                    glow(p.x, p.y, 13.0, alpha(c::AMBER_DEEP, 0.5 * fl));
                    draw_ellipse_lines(
                        p.x,
                        p.y,
                        6.0,
                        4.6,
                        0.0,
                        1.6,
                        alpha(Color::from_hex(0xff8a3d), fl),
                    );
                } else {
                    draw_ellipse_lines(p.x, p.y, 6.0, 4.6, 0.0, 1.2, c::STEEL_DARK);
                }
            }
            // saucepan
            let p = vec2(o.x + w * 0.3, top_y + dd * 0.3);
            rrect_v(
                p.x - 9.0,
                p.y - 12.0,
                18.0,
                12.0,
                3.0,
                c::STEEL_LIGHT,
                c::STEEL,
            );
            ellipse(p.x, p.y - 12.0, 9.0, 3.5, shade(c::STEEL, 0.8));
            draw_line(
                p.x + 9.0,
                p.y - 9.0,
                p.x + 17.0,
                p.y - 10.0,
                2.0,
                c::GRAPHITE,
            );
            if state.active {
                draw_steam(vec2(p.x, p.y - 14.0), t, 0.9);
            }
            // knobs on the front
            for i in 0..4 {
                let kx = o.x + 10.0 + i as f32 * (w - 20.0) / 3.0;
                ellipse(kx, o.y + 6.0 + dd - COUNTER_H + 6.0, 2.4, 2.4, c::GRAPHITE);
            }
        }
        "oven" => {
            counter_base(o, w, d, c::STEEL, c::STEEL_LIGHT);
            let fy = o.y + 6.0 + d - 10.0 - COUNTER_H;
            let win = if state.active {
                (Color::from_hex(0xffb35c), Color::from_hex(0xc2552e))
            } else {
                (Color::from_hex(0x2a2a3a), Color::from_hex(0x15141f))
            };
            rrect_v(
                o.x + 8.0,
                fy + 8.0,
                w - 16.0,
                COUNTER_H - 14.0,
                3.0,
                win.0,
                win.1,
            );
            if state.active {
                glow(cx, fy + 16.0, 22.0, alpha(c::AMBER, 0.35));
            }
            draw_line(
                o.x + 10.0,
                fy + 5.0,
                o.x + w - 10.0,
                fy + 5.0,
                2.0,
                c::STEEL_LIGHT,
            );
        }
        "sink" => {
            counter_base(o, w, d, Color::from_hex(0x6c7aa0), c::STEEL_LIGHT);
            let top_y = o.y + 6.0 - COUNTER_H;
            let dd = d - 10.0;
            rrect_v(
                o.x + 9.0,
                top_y + 7.0,
                w - 18.0,
                dd - 14.0,
                6.0,
                c::STEEL_DARK,
                shade(c::STEEL, 0.9),
            );
            // tap
            let fx = cx;
            capsule(
                vec2(fx, top_y + 6.0),
                vec2(fx, top_y - 8.0),
                1.8,
                c::STEEL_LIGHT,
            );
            capsule(
                vec2(fx, top_y - 8.0),
                vec2(fx, top_y + 2.0 - 6.0 + 10.0),
                1.6,
                c::STEEL_LIGHT,
            );
            // dirty dishes
            let plates = (state.dirty * 5.0).round() as i32;
            for i in 0..plates {
                let p = vec2(
                    o.x + w * 0.35 + (i % 2) as f32 * 14.0,
                    top_y + dd * 0.5 - i as f32 * 2.0,
                );
                ellipse_v(p.x, p.y, 9.0, 4.0, c::CREAM, shade(c::CREAM, 0.8));
                ellipse(
                    p.x + 2.0,
                    p.y,
                    2.0,
                    1.0,
                    alpha(Color::from_hex(0xa0724a), 0.7),
                );
            }
        }
        "fridge" => {
            let (x, y0) = (o.x + 6.0, o.y + 8.0);
            let (fw, fd, h) = (w - 12.0, d - 14.0, 80.0);
            // a mint retro fridge — stands out on any floor
            box3(
                x,
                y0,
                fw,
                fd,
                h,
                Color::from_hex(0xc8eadf),
                Color::from_hex(0x9fd3c4),
                8.0,
            );
            let fy = y0 + fd - h;
            draw_line(
                x + 3.0,
                fy + 28.0,
                x + fw - 3.0,
                fy + 28.0,
                1.2,
                alpha(shade(c::STEEL, 0.7), 0.8),
            );
            rrect_v(
                x + fw - 9.0,
                fy + 8.0,
                3.0,
                14.0,
                1.5,
                c::STEEL_LIGHT,
                c::STEEL,
            );
            rrect_v(
                x + fw - 9.0,
                fy + 34.0,
                3.0,
                20.0,
                1.5,
                c::STEEL_LIGHT,
                c::STEEL,
            );
            // magnets and a note
            rrect(
                x + 8.0,
                fy + 36.0,
                9.0,
                11.0,
                1.0,
                Color::from_hex(0xfff3c9),
            );
            ellipse(x + 12.5, fy + 36.0, 1.6, 1.6, c::ROSE);
            ellipse(x + 22.0, fy + 42.0, 2.0, 2.0, c::INDIGO);
            if state.active {
                rrect(x + 8.0, fy + 12.0, 10.0, 4.0, 1.0, c::INDIGO_SOFT);
            }
        }
        "dish_rack" => {
            counter_base(o, w, d, c::WOOD_PLUM, c::STEEL_LIGHT);
            let top_y = o.y + 6.0 + (d - 10.0) * 0.5 - COUNTER_H;
            let n = 3 + (state.dirty * 4.0) as i32;
            for i in 0..n {
                let px = o.x + 12.0 + i as f32 * (w - 24.0) / (n - 1).max(1) as f32;
                ellipse_v(px, top_y - 9.0, 3.0, 10.0, c::CREAM, shade(c::CREAM, 0.75));
            }
            for k in 0..2 {
                let y = top_y - 2.0 - k as f32 * 6.0;
                draw_line(o.x + 8.0, y, o.x + w - 8.0, y, 1.2, c::STEEL);
            }
        }
        "register" => {
            counter_base(o, w, d, c::WOOD_PLUM, tint_light(c::WOOD_LIGHT, 0.05));
            let top = o.y + 6.0 + (d - 10.0) * 0.5 - COUNTER_H;
            soft_shadow(cx, top + 2.0, 16.0, 5.0, 0.4);
            rrect_v(
                cx - 14.0,
                top - 6.0,
                28.0,
                7.0,
                2.5,
                c::GRAPHITE,
                shade(c::GRAPHITE, 0.7),
            );
            capsule(
                vec2(cx, top - 4.0),
                vec2(cx + 2.0, top - 14.0),
                1.6,
                c::STEEL_DARK,
            );
            // tilted screen
            poly_v(
                &[
                    vec2(cx - 12.0, top - 28.0),
                    vec2(cx + 14.0, top - 26.0),
                    vec2(cx + 13.0, top - 12.0),
                    vec2(cx - 11.0, top - 13.0),
                ],
                c::GRAPHITE,
                shade(c::GRAPHITE, 0.8),
            );
            if state.active {
                poly_v(
                    &[
                        vec2(cx - 10.0, top - 26.0),
                        vec2(cx + 12.0, top - 24.5),
                        vec2(cx + 11.0, top - 14.0),
                        vec2(cx - 9.5, top - 15.0),
                    ],
                    c::INDIGO_GLOW,
                    c::INDIGO_600,
                );
                glow(cx, top - 20.0, 16.0, alpha(c::INDIGO_GLOW, 0.4));
                rrect(cx - 6.0, top - 22.0, 6.0, 5.0, 1.5, alpha(WHITE, 0.8));
                draw_line(
                    cx + 2.0,
                    top - 21.0,
                    cx + 8.0,
                    top - 20.5,
                    1.0,
                    alpha(WHITE, 0.7),
                );
                draw_line(
                    cx + 2.0,
                    top - 18.0,
                    cx + 6.0,
                    top - 17.8,
                    1.0,
                    alpha(WHITE, 0.5),
                );
            }
            // receipt
            rrect(cx + 15.0, top - 9.0, 6.0, 8.0, 1.0, c::CREAM);
        }
        "menu_holo_board" => draw_holo_board(o, w, d, state),
        "plant_small" => {
            let fy = cy + 12.0;
            pot(cx, fy, 20.0, 15.0);
            let sway = (t * 0.9 + seed as f32 * 0.01).sin() * 0.06;
            for i in 0..7 {
                let a = -PI / 2.0 + (i as f32 - 3.0) * 0.42 + sway;
                let len = 15.0 + hash01(seed + i) * 6.0;
                let col = if i % 2 == 0 { c::LEAF } else { c::LEAF_DARK };
                leaf(vec2(cx, fy - 16.0), a, len, 4.5, col);
            }
        }
        "plant_tall" => {
            let fy = cy + 14.0;
            pot(cx, fy, 28.0, 22.0);
            let sway = (t * 0.7 + seed as f32 * 0.013).sin() * 0.04;
            let stem_top = vec2(cx + 2.0, fy - 84.0);
            draw_line(
                cx,
                fy - 22.0,
                stem_top.x,
                stem_top.y + 10.0,
                2.0,
                shade(c::LEAF_DARK, 0.8),
            );
            for i in 0..13 {
                let k = i as f32 / 12.0;
                let base = vec2(cx + (k * 4.0).sin() * 3.0, fy - 26.0 - k * 58.0);
                let side = if i % 2 == 0 { -1.0 } else { 1.0 };
                let a = -PI / 2.0 + side * (0.9 - k * 0.5) + sway * (1.0 + k);
                let col = mix(c::LEAF_DARK, c::LEAF, 0.3 + k * 0.6);
                leaf(base, a, 20.0 - k * 6.0, 7.0 - k * 2.0, col);
            }
        }
        "floor_lamp" => {
            soft_shadow(cx + 2.0, cy + 6.0, 12.0, 5.0, 0.45);
            ellipse_v(
                cx,
                cy + 4.0,
                10.0,
                4.0,
                Color::from_hex(0xd9a85c),
                Color::from_hex(0x8a6234),
            );
            draw_line(cx, cy + 3.0, cx, cy - 78.0, 2.2, Color::from_hex(0xb8894a));
            let sy = cy - 92.0;
            let (lit_t, lit_b) = if state.active {
                (Color::from_hex(0xffe2b0), Color::from_hex(0xffb35c))
            } else {
                (Color::from_hex(0xd8c6b0), Color::from_hex(0x9a8670))
            };
            if state.active {
                glow(cx, sy + 8.0, 34.0, alpha(c::AMBER, 0.35));
            }
            poly_v(
                &[
                    vec2(cx - 9.0, sy),
                    vec2(cx + 9.0, sy),
                    vec2(cx + 15.0, sy + 18.0),
                    vec2(cx - 15.0, sy + 18.0),
                ],
                lit_t,
                lit_b,
            );
            ellipse(
                cx,
                sy + 18.0,
                15.0,
                3.0,
                if state.active {
                    Color::from_hex(0xfff0d0)
                } else {
                    lit_b
                },
            );
        }
        "pendant_lamp" => {
            let ly = cy - 96.0;
            // the cord rises to the "ceiling" and fades — it doesn't cross out the hall
            quad(
                [
                    vec2(cx - 0.6, ly - 46.0),
                    vec2(cx + 0.6, ly - 46.0),
                    vec2(cx + 0.6, ly - 4.0),
                    vec2(cx - 0.6, ly - 4.0),
                ],
                [
                    alpha(Color::from_hex(0x2a2433), 0.0),
                    alpha(Color::from_hex(0x2a2433), 0.0),
                    Color::from_hex(0x2a2433),
                    Color::from_hex(0x2a2433),
                ],
            );
            if state.active {
                glow(cx, ly + 6.0, 40.0, alpha(c::AMBER, 0.3));
                // a soft cone of light downwards
                quad(
                    [
                        vec2(cx - 10.0, ly + 6.0),
                        vec2(cx + 10.0, ly + 6.0),
                        vec2(cx + 34.0, ly + 80.0),
                        vec2(cx - 34.0, ly + 80.0),
                    ],
                    [
                        alpha(c::AMBER, 0.16),
                        alpha(c::AMBER, 0.16),
                        alpha(c::AMBER, 0.0),
                        alpha(c::AMBER, 0.0),
                    ],
                );
            }
            let shade_top = Color::from_hex(0x4c45a8);
            let shade_bot = Color::from_hex(0x2f2a78);
            poly_v(
                &[
                    vec2(cx - 4.0, ly - 6.0),
                    vec2(cx + 4.0, ly - 6.0),
                    vec2(cx + 14.0, ly + 6.0),
                    vec2(cx - 14.0, ly + 6.0),
                ],
                shade_top,
                shade_bot,
            );
            ellipse(
                cx,
                ly + 6.0,
                14.0,
                3.2,
                if state.active {
                    Color::from_hex(0xffe9c2)
                } else {
                    shade(shade_bot, 0.8)
                },
            );
            if state.active {
                ellipse(cx, ly + 6.5, 5.0, 2.2, WHITE);
            }
            draw_line(
                cx - 9.0,
                ly - 1.0,
                cx - 3.0,
                ly - 5.0,
                1.0,
                alpha(c::LAVENDER_RIM, 0.6),
            );
        }
        "sofa" | "armchair" | "window_seat" => {
            let fabric = match sprite {
                "sofa" => c::FABRIC_PLUM,
                "armchair" => c::FABRIC_TEAL,
                _ => c::FABRIC_ROSE,
            };
            draw_seat(o, w, d, fabric, sprite == "window_seat", seed);
        }
        "bookshelf" => draw_bookshelf(o, w, d, seed),
        "jukebox" => draw_jukebox(o, w, d, state),
        "rug" => draw_rug(o, w, d),
        "trash_bin" => {
            let fy = cy + 10.0;
            soft_shadow(cx + 2.0, fy, 13.0, 5.0, 0.4);
            rrect_v(
                cx - 11.0,
                fy - 30.0,
                22.0,
                30.0,
                5.0,
                Color::from_hex(0x4f5672),
                Color::from_hex(0x2e3246),
            );
            for i in 0..3 {
                let lx = cx - 6.0 + i as f32 * 6.0;
                draw_line(
                    lx,
                    fy - 26.0,
                    lx,
                    fy - 4.0,
                    1.0,
                    alpha(Color::from_hex(0x23263a), 0.7),
                );
            }
            ellipse_v(
                cx,
                fy - 30.0,
                11.5,
                4.0,
                Color::from_hex(0x6a7294),
                Color::from_hex(0x4f5672),
            );
            if state.dirty > 0.5 {
                for i in 0..4 {
                    let a = hash01(seed + i) * 6.0 - 3.0;
                    ellipse(
                        cx + a * 2.5,
                        fy - 33.0 - (i % 2) as f32 * 2.5,
                        4.0,
                        3.0,
                        mix(c::CREAM, WHITE, 0.4),
                    );
                }
            }
            ellipse(cx, fy - 13.0, 2.5, 2.5, c::EMERALD);
        }
        "coat_rack" => {
            soft_shadow(cx + 2.0, cy + 8.0, 12.0, 5.0, 0.45);
            for a in [-0.5_f32, 0.5, 2.6] {
                draw_line(
                    cx,
                    cy + 6.0,
                    cx + a.cos() * 12.0,
                    cy + 6.0 + a.sin() * 5.0,
                    2.2,
                    c::WOOD_DARK,
                );
            }
            draw_line(cx, cy + 6.0, cx, cy - 86.0, 3.0, c::WOOD);
            ellipse(cx, cy - 87.0, 3.2, 3.2, c::WOOD_LIGHT);
            // coat
            rrect_v(
                cx - 12.0,
                cy - 80.0,
                13.0,
                46.0,
                5.0,
                Color::from_hex(0x5b6f9a),
                Color::from_hex(0x3b4a6b),
            );
            // scarf
            rrect_v(
                cx + 1.0,
                cy - 82.0,
                6.0,
                30.0,
                2.5,
                c::AMBER_DEEP,
                shade(c::AMBER_DEEP, 0.7),
            );
            // umbrella, dripping
            capsule(
                vec2(cx + 8.0, cy - 62.0),
                vec2(cx + 12.0, cy + 2.0),
                3.0,
                Color::from_hex(0x2f2a50),
            );
            let drip = (t * 0.9 + hash01(seed) * 3.0) % 1.0;
            ellipse(
                cx + 12.0,
                cy + 2.0 + drip * 8.0,
                1.0,
                1.6,
                alpha(c::INDIGO_SOFT, 0.7 * (1.0 - drip)),
            );
            radial(
                cx + 12.0,
                cy + 9.0,
                9.0,
                3.0,
                alpha(c::INDIGO_SOFT, 0.25),
                alpha(c::INDIGO_SOFT, 0.0),
            );
        }
        "drone_dock" => draw_drone_dock(o, w, d, state),
        "grandma_photo" => draw_grandma_photo(o, w, t),
        "radio" => {
            // side table + radio
            box3(
                o.x + 14.0,
                o.y + 18.0,
                w - 28.0,
                d - 30.0,
                24.0,
                c::WOOD,
                c::WOOD_DARK,
                3.0,
            );
            let top = o.y + 18.0 + (d - 30.0) * 0.5 - 24.0;
            rrect_v(
                cx - 14.0,
                top - 18.0,
                28.0,
                18.0,
                5.0,
                Color::from_hex(0xa8794f),
                Color::from_hex(0x6e4532),
            );
            rrect(
                cx - 11.0,
                top - 15.0,
                12.0,
                12.0,
                3.0,
                Color::from_hex(0x3a2a24),
            );
            for i in 0..3 {
                let ly = top - 13.0 + i as f32 * 4.0;
                draw_line(
                    cx - 10.0,
                    ly,
                    cx,
                    ly,
                    0.8,
                    alpha(Color::from_hex(0x8a6a4a), 0.8),
                );
            }
            let dial = if state.active {
                c::AMBER
            } else {
                Color::from_hex(0x6b5a4a)
            };
            rrect(cx + 3.0, top - 14.0, 8.0, 4.0, 1.5, dial);
            ellipse(cx + 7.0, top - 6.0, 2.5, 2.5, c::CREAM);
            draw_line(cx + 10.0, top - 18.0, cx + 16.0, top - 30.0, 1.0, c::STEEL);
            if state.active {
                glow(cx + 7.0, top - 12.0, 10.0, alpha(c::AMBER, 0.45));
                floating_notes(vec2(cx + 4.0, top - 22.0), t, seed, c::AMBER);
            }
        }
        "candle" => draw_candle(vec2(cx, cy), t, seed),
        "bucket" => draw_bucket(vec2(cx, cy + 8.0), state, seed),
        _ => {
            // placeholder: a soft lavender box with a "?"
            let (x, y0) = (o.x + 8.0, o.y + 10.0);
            box3(
                x,
                y0,
                w - 16.0,
                d - 20.0,
                26.0,
                c::VIOLET_SOFT,
                c::VIOLET,
                6.0,
            );
            let q = vec2(cx, y0 + (d - 20.0) / 2.0 - 26.0);
            arc(q + vec2(0.0, -3.0), 4.0, 4.0, PI, PI * 2.4, 2.0, WHITE);
            draw_line(q.x, q.y + 1.0, q.x, q.y + 3.5, 2.0, WHITE);
            ellipse(q.x, q.y + 6.5, 1.3, 1.3, WHITE);
        }
    }
}

/// A metal bucket under a roof leak. `p` — the bottom point on the floor.
/// `active` — dripping (a drop falls from the ceiling, a ring spreads on the water),
/// `dirty` — water level 0..1.
fn draw_bucket(p: Vec2, st: &FurnitureState, seed: u32) {
    let (w_top, w_bot, h) = (13.0, 10.5, 20.0);
    soft_shadow(p.x + 2.0, p.y, 15.0, 5.0, 0.4);
    // body: a truncated cone with a vertical gradient and hoops
    poly_v(
        &[
            vec2(p.x - w_top, p.y - h),
            vec2(p.x + w_top, p.y - h),
            vec2(p.x + w_bot, p.y),
            vec2(p.x - w_bot, p.y),
        ],
        c::STEEL_LIGHT,
        c::STEEL_DARK,
    );
    ellipse_v(p.x, p.y, w_bot, 3.0, c::STEEL, c::STEEL_DARK);
    for k in [0.3_f32, 0.75] {
        let y = p.y - h + h * k;
        let hw = w_top + (w_bot - w_top) * k;
        draw_line(p.x - hw, y, p.x + hw, y, 1.2, alpha(c::STEEL_DARK, 0.8));
    }
    draw_line(
        p.x - w_top + 2.0,
        p.y - h + 3.0,
        p.x - w_bot + 2.0,
        p.y - 3.0,
        1.2,
        alpha(WHITE, 0.35),
    );
    // rim and the water inside
    let rim = vec2(p.x, p.y - h);
    ellipse(rim.x, rim.y, w_top, 4.4, c::STEEL_DARK);
    let level = st.dirty.clamp(0.0, 1.0);
    let (wx, wy) = (w_top - 1.5, 3.4);
    ellipse_v(
        rim.x,
        rim.y + 0.6 - level * 0.8,
        wx,
        wy,
        Color::from_hex(0x3b3f8a),
        Color::from_hex(0x23265a),
    );
    draw_line(
        rim.x - wx * 0.6,
        rim.y - 0.6,
        rim.x + wx * 0.1,
        rim.y - 1.2,
        0.9,
        alpha(c::LAVENDER_RIM, 0.55),
    );
    // handle
    arc(
        rim + vec2(0.0, 1.0),
        w_top + 0.5,
        9.0,
        PI * 1.05,
        PI * 1.95,
        1.0,
        c::STEEL,
    );
    if !st.active {
        return;
    }
    // the drop: falls from the "ceiling", rings at the water
    let period = 1.6 + hash01(seed) * 0.6;
    let a = ((st.t + hash01(seed + 1) * period) % period) / period;
    let fall = 0.55;
    let top = rim.y - 88.0;
    if a < fall {
        let k = a / fall;
        let y = top + (rim.y - top) * k * k;
        let fade = (k * 4.0).min(1.0);
        draw_line(
            rim.x,
            y - 6.0 * k,
            rim.x,
            y,
            0.9,
            alpha(c::INDIGO_SOFT, 0.35 * fade),
        );
        ellipse(
            rim.x,
            y,
            1.3,
            1.8,
            alpha(Color::new(0.85, 0.9, 1.0, 1.0), 0.85 * fade),
        );
    } else {
        let k = (a - fall) / (1.0 - fall);
        for (i, delay) in [0.0_f32, 0.25].iter().enumerate() {
            let kk = ((k - delay) / (1.0 - delay)).clamp(0.0, 1.0);
            if kk <= 0.0 {
                continue;
            }
            let rx = 1.5 + kk * (wx - 2.5);
            draw_ellipse_lines(
                rim.x,
                rim.y + 0.4,
                rx,
                rx * 0.3,
                0.0,
                0.9,
                alpha(c::LAVENDER_RIM, 0.7 * (1.0 - kk) / (1.0 + i as f32)),
            );
        }
        // splashes in the first moments
        if k < 0.25 {
            let up = (k / 0.25) * PI;
            for sx in [-1.0_f32, 1.0] {
                ellipse(
                    rim.x + sx * 2.5 * (k / 0.25),
                    rim.y - up.sin() * 4.0,
                    0.7,
                    0.9,
                    alpha(WHITE, 0.7),
                );
            }
        }
    }
}

fn draw_candle(p: Vec2, t: f32, seed: u32) {
    let fl = noise1(t * 7.0, seed) * 0.4 + 0.6;
    rrect_v(
        p.x - 3.2,
        p.y - 7.0,
        6.4,
        8.0,
        2.0,
        Color::new(0.85, 0.85, 1.0, 0.35),
        Color::new(0.6, 0.6, 0.85, 0.4),
    );
    rrect_v(
        p.x - 2.0,
        p.y - 6.0,
        4.0,
        6.5,
        1.0,
        c::CREAM,
        shade(c::CREAM, 0.85),
    );
    glow(p.x, p.y - 9.0, 9.0 * fl, alpha(c::AMBER, 0.6));
    let sway = (t * 5.0 + seed as f32).sin() * 0.6;
    ellipse(
        p.x + sway * 0.5,
        p.y - 9.0,
        1.4,
        2.6 * fl,
        Color::from_hex(0xffd27a),
    );
    ellipse(p.x + sway * 0.3, p.y - 8.4, 0.7, 1.2, WHITE);
}

fn floating_notes(base: Vec2, t: f32, seed: u32, col: Color) {
    for i in 0..2 {
        let life = 2.2;
        let a = ((t + i as f32 * 1.1 + hash01(seed) * 2.0) % life) / life;
        let p = base + vec2((a * 5.0 + i as f32).sin() * 5.0 + i as f32 * 6.0, -a * 26.0);
        let fade = (a * (1.0 - a) * 4.0).min(1.0);
        let cc = alpha(col, fade * 0.9);
        ellipse(p.x, p.y, 2.0, 1.5, cc);
        draw_line(p.x + 1.7, p.y, p.x + 1.7, p.y - 6.0, 0.9, cc);
        draw_line(p.x + 1.7, p.y - 6.0, p.x + 4.0, p.y - 4.5, 0.9, cc);
    }
}

fn draw_chair(o: Vec2, facing: Facing) {
    let cx = o.x + TILE / 2.0;
    let cy = o.y + TILE / 2.0;
    let seat_h = 17.0;
    let (sw, sd) = (28.0, 24.0);
    let (sx, sy0) = (cx - sw / 2.0, cy - sd / 2.0 + 2.0);
    soft_shadow(cx + 2.0, cy + 6.0, 17.0, 11.0, 0.42);
    let leg = |x: f32, y: f32, col: Color| rect_v(x, y - seat_h, 3.0, seat_h, col, shade(col, 0.6));
    let wood = c::WOOD;
    let back_col = c::WOOD_DARK;
    let back_h = 24.0;
    // the backrest behind the seat is drawn before it
    let back = |x: f32, y: f32, bw: f32, bd: f32| {
        rrect_v(
            x,
            y - seat_h - back_h,
            bw,
            back_h + 2.0,
            3.0,
            tint_light(back_col, 0.1),
            shade(back_col, 0.8),
        );
        rrect(
            x,
            y - seat_h - back_h - 1.0,
            bw,
            bd,
            2.0,
            tint_light(c::WOOD, 0.1),
        );
        draw_line(
            x + 2.0,
            y - seat_h - back_h - 0.5,
            x + bw - 2.0,
            y - seat_h - back_h - 0.5,
            1.0,
            alpha(c::LAVENDER_RIM, 0.45),
        );
    };
    leg(sx + 2.0, sy0 + 3.0, shade(back_col, 0.8));
    leg(sx + sw - 5.0, sy0 + 3.0, shade(back_col, 0.8));
    if facing == Facing::Down {
        back(sx + 1.0, sy0 + 3.0, sw - 2.0, 4.0);
    }
    leg(sx + 2.0, sy0 + sd, back_col);
    leg(sx + sw - 5.0, sy0 + sd, back_col);
    // seat: the cushion
    rrect_v(
        sx,
        sy0 + sd - seat_h - 4.0,
        sw,
        5.0,
        2.0,
        shade(wood, 0.85),
        shade(wood, 0.6),
    );
    rrect_v(
        sx,
        sy0 - seat_h,
        sw,
        sd,
        5.0,
        tint_light(c::FABRIC_PLUM, 0.15),
        c::FABRIC_PLUM,
    );
    match facing {
        Facing::Up => back(sx + 1.0, sy0 + sd + 1.0, sw - 2.0, 4.0),
        Facing::Left | Facing::Right => {
            // side backrest: a narrow slat along the seat, rising above it
            let bx = if facing == Facing::Right {
                sx - 1.0
            } else {
                sx + sw - 6.0
            };
            let top = sy0 - seat_h - back_h;
            rrect_v(
                bx,
                top,
                7.0,
                back_h + sd - 2.0,
                3.0,
                tint_light(back_col, 0.12),
                shade(back_col, 0.75),
            );
            rrect(bx, top, 7.0, 5.0, 2.5, tint_light(c::WOOD, 0.15));
            draw_line(
                bx + 1.5,
                top + 4.0,
                bx + 1.5,
                top + back_h + sd - 6.0,
                1.0,
                alpha(c::LAVENDER_RIM, 0.4),
            );
        }
        Facing::Down => {}
    }
}

fn draw_seat(o: Vec2, w: f32, d: f32, fabric: Color, window_seat: bool, seed: u32) {
    let (x, y0) = (o.x + 4.0, o.y + 10.0);
    let (sw, sd) = (w - 8.0, d - 16.0);
    let seat_h = 18.0;
    let back_h = if window_seat { 0.0 } else { 34.0 };
    let arm = if window_seat { 0.0 } else { 9.0 };
    soft_shadow(x + sw / 2.0 + 3.0, y0 + sd * 0.7, sw * 0.6, sd * 0.6, 0.45);
    // backrest
    if back_h > 0.0 {
        box3(
            x,
            y0,
            sw,
            12.0,
            seat_h + back_h,
            tint_light(fabric, 0.1),
            shade(fabric, 0.85),
            7.0,
        );
    }
    // seat base
    box3(
        x,
        y0 + 8.0,
        sw,
        sd - 8.0,
        seat_h - 4.0,
        shade(fabric, 0.9),
        shade(fabric, 0.7),
        5.0,
    );
    // cushions
    let n = ((sw - 2.0 * arm) / 50.0).round().max(1.0) as i32;
    let cw = (sw - 2.0 * arm) / n as f32;
    for i in 0..n {
        let px = x + arm + i as f32 * cw;
        rrect_v(
            px + 1.0,
            y0 + 10.0 - seat_h,
            cw - 2.0,
            sd - 12.0,
            6.0,
            tint_light(fabric, 0.18),
            fabric,
        );
    }
    // armrests
    if arm > 0.0 {
        for ax in [x, x + sw - arm] {
            box3(
                ax,
                y0 + 2.0,
                arm,
                sd - 2.0,
                seat_h + 6.0,
                tint_light(fabric, 0.12),
                shade(fabric, 0.8),
                4.0,
            );
        }
    }
    // throw cushion
    let px = x + arm + 12.0 + hash01(seed) * 8.0;
    rrect_v(
        px,
        y0 + 2.0 - seat_h - 8.0,
        16.0,
        14.0,
        5.0,
        tint_light(c::AMBER, 0.2),
        c::AMBER_DEEP,
    );
    if window_seat {
        let px2 = x + sw - 30.0;
        rrect_v(
            px2,
            y0 + 2.0 - seat_h - 6.0,
            15.0,
            12.0,
            5.0,
            c::INDIGO_SOFT,
            c::INDIGO,
        );
    }
}

fn draw_bookshelf(o: Vec2, w: f32, d: f32, seed: u32) {
    let (x, y0) = (o.x + 3.0, o.y + d - 24.0);
    let (bw, bd, h) = (w - 6.0, 18.0, 104.0);
    box3(x, y0, bw, bd, h, c::WOOD, c::WOOD_DARK, 3.0);
    let fy = y0 + bd - h;
    rrect_v(
        x + 4.0,
        fy + 4.0,
        bw - 8.0,
        h - 8.0,
        2.0,
        Color::from_hex(0x2c1e22),
        Color::from_hex(0x3a2826),
    );
    let shelves = 4;
    let sh = (h - 8.0) / shelves as f32;
    let colors = [
        0x7b4f8e, 0x3f8a8c, 0xc79a3e, 0xb5603f, 0x5b6f9a, 0xe8dcc6, 0x9a5a6e, 0x4d6b57,
    ];
    for s in 0..shelves {
        let base = fy + 4.0 + (s + 1) as f32 * sh;
        let mut bx = x + 6.0;
        let mut i = 0u32;
        while bx < x + bw - 10.0 {
            let r = hash01(seed + s as u32 * 97 + i);
            let bwid = 3.5 + r * 4.0;
            let bh = sh * (0.62 + hash01(seed + i * 7 + s as u32) * 0.3);
            let col = Color::from_hex(colors[(hash(seed + i * 13 + s as u32) % 8) as usize]);
            if r > 0.9 && s % 2 == 1 {
                // a small plant on the shelf
                ellipse(bx + 5.0, base - 4.0, 4.5, 4.0, c::POT);
                leaf(
                    vec2(bx + 5.0, base - 7.0),
                    -PI / 2.0 - 0.5,
                    9.0,
                    3.0,
                    c::LEAF,
                );
                leaf(
                    vec2(bx + 5.0, base - 7.0),
                    -PI / 2.0 + 0.5,
                    9.0,
                    3.0,
                    c::LEAF_DARK,
                );
                bx += 12.0;
            } else {
                rect_v(
                    bx,
                    base - bh,
                    bwid,
                    bh,
                    tint_light(col, 0.1),
                    shade(col, 0.75),
                );
                draw_line(
                    bx + 0.5,
                    base - bh + 3.0,
                    bx + bwid - 0.5,
                    base - bh + 3.0,
                    0.7,
                    alpha(c::CREAM, 0.5),
                );
                bx += bwid + 0.8;
            }
            i += 1;
        }
        rect_v(x + 4.0, base - 1.0, bw - 8.0, 3.0, c::WOOD_LIGHT, c::WOOD);
    }
}

fn draw_jukebox(o: Vec2, w: f32, d: f32, st: &FurnitureState) {
    let cx = o.x + w / 2.0;
    let (jw, jd, h) = (w.min(TILE) - 14.0, 22.0, 76.0);
    let x = cx - jw / 2.0;
    let y0 = o.y + d - jd - 10.0;
    let fy = y0 + jd;
    soft_shadow(cx + 3.0, fy - 4.0, jw * 0.65, 10.0, 0.45);
    // arched cabinet
    rrect_v(
        x,
        fy - h + 14.0,
        jw,
        h - 14.0,
        6.0,
        c::WOOD_PLUM,
        shade(c::WOOD_PLUM, 0.6),
    );
    ellipse_v(
        cx,
        fy - h + 16.0,
        jw / 2.0,
        16.0,
        tint_light(c::WOOD_PLUM, 0.15),
        c::WOOD_PLUM,
    );
    // glowing tubes
    let t = st.t;
    let hue = |k: f32| -> Color {
        if !st.active {
            return Color::from_hex(0x3a2f55);
        }
        let s = (t * 1.3 + k).sin() * 0.5 + 0.5;
        mix(
            mix(c::VIOLET_SOFT, c::AMBER, s),
            c::INDIGO_GLOW,
            ((t * 0.7 + k * 2.0).sin() * 0.5 + 0.5) * 0.5,
        )
    };
    for (i, inset) in [3.0_f32, 7.5].iter().enumerate() {
        let col = hue(i as f32 * 1.7);
        arc(
            vec2(cx, fy - h + 16.0),
            jw / 2.0 - inset,
            16.0 - inset * 0.8,
            PI,
            PI * 2.0,
            2.4,
            col,
        );
        draw_line(x + inset, fy - h + 16.0, x + inset, fy - 12.0, 2.4, col);
        draw_line(
            x + jw - inset,
            fy - h + 16.0,
            x + jw - inset,
            fy - 12.0,
            2.4,
            col,
        );
    }
    if st.active {
        glow(cx, fy - h + 26.0, 30.0, alpha(c::VIOLET_SOFT, 0.35));
    }
    // window with the record
    rrect_v(
        cx - 10.0,
        fy - h + 14.0,
        20.0,
        14.0,
        4.0,
        Color::from_hex(0x2a2140),
        Color::from_hex(0x1b1530),
    );
    ellipse(cx, fy - h + 22.0, 7.0, 3.0, Color::from_hex(0x111018));
    ellipse(cx, fy - h + 22.0, 2.0, 1.0, c::AMBER_DEEP);
    // speaker grille
    rrect(
        cx - 11.0,
        fy - 34.0,
        22.0,
        18.0,
        4.0,
        Color::from_hex(0x2a2140),
    );
    for i in 0..4 {
        let ly = fy - 31.0 + i as f32 * 4.0;
        draw_line(cx - 9.0, ly, cx + 9.0, ly, 1.0, Color::from_hex(0xb8894a));
    }
    if st.active {
        floating_notes(vec2(cx - 4.0, fy - h), t, 7, c::VIOLET_SOFT);
    }
}

fn draw_rug(o: Vec2, w: f32, d: f32) {
    let (x, y, rw, rh) = (o.x + 6.0, o.y + 6.0, w - 12.0, d - 12.0);
    rrect(
        x + 1.0,
        y + 2.0,
        rw,
        rh,
        6.0,
        Color::new(0.03, 0.02, 0.08, 0.3),
    );
    rrect_v(
        x,
        y,
        rw,
        rh,
        6.0,
        Color::from_hex(0x3b2f7a),
        Color::from_hex(0x2b2160),
    );
    rrect(
        x + 5.0,
        y + 5.0,
        rw - 10.0,
        rh - 10.0,
        4.0,
        Color::from_hex(0xc98a4b),
    );
    rrect_v(
        x + 8.0,
        y + 8.0,
        rw - 16.0,
        rh - 16.0,
        3.0,
        Color::from_hex(0x5a3f8f),
        Color::from_hex(0x46317a),
    );
    // diamonds along the middle
    let n = ((rw - 16.0) / 40.0).floor().max(1.0) as i32;
    let cy = y + rh / 2.0;
    for i in 0..n {
        let cx = x + 8.0 + (i as f32 + 0.5) * (rw - 16.0) / n as f32;
        let s = ((rh - 16.0) * 0.32).min(16.0);
        poly(
            &[
                vec2(cx, cy - s),
                vec2(cx + s, cy),
                vec2(cx, cy + s),
                vec2(cx - s, cy),
            ],
            Color::from_hex(0xc98a4b),
        );
        poly(
            &[
                vec2(cx, cy - s * 0.5),
                vec2(cx + s * 0.5, cy),
                vec2(cx, cy + s * 0.5),
                vec2(cx - s * 0.5, cy),
            ],
            Color::from_hex(0x7c3aed),
        );
    }
    // fringe
    let mut fx = x + 4.0;
    while fx < x + rw - 3.0 {
        draw_line(fx, y, fx, y - 3.5, 1.0, Color::from_hex(0xe8dcc6));
        draw_line(fx, y + rh, fx, y + rh + 3.5, 1.0, Color::from_hex(0xe8dcc6));
        fx += 4.0;
    }
}

fn draw_holo_board(o: Vec2, w: f32, d: f32, st: &FurnitureState) {
    let cx = o.x + w / 2.0;
    let fy = o.y + d / 2.0 + 10.0;
    let t = st.t;
    soft_shadow(cx + 2.0, fy, 14.0, 5.0, 0.4);
    ellipse_v(
        cx,
        fy - 1.0,
        11.0,
        4.0,
        c::GRAPHITE,
        shade(c::GRAPHITE, 0.7),
    );
    rect_v(cx - 1.5, fy - 40.0, 3.0, 40.0, c::STEEL, c::STEEL_DARK);
    // projector
    ellipse(cx, fy - 40.0, 6.0, 2.5, c::STEEL_DARK);
    let (bw, bh) = ((w - 10.0).min(58.0), 46.0);
    let (bx, by) = (cx - bw / 2.0, fy - 44.0 - bh);
    if !st.active {
        rrect(bx, by, bw, bh, 5.0, Color::new(0.2, 0.2, 0.35, 0.25));
        return;
    }
    // flicker: rare brightness dips + a running scan line
    let fl = holo_flicker(t, 3);
    let a = 0.62 * fl;
    // projector beam
    quad(
        [
            vec2(cx - 3.0, fy - 41.0),
            vec2(cx + 3.0, fy - 41.0),
            vec2(bx + bw, by + bh),
            vec2(bx, by + bh),
        ],
        [
            alpha(c::INDIGO_GLOW, 0.4 * fl),
            alpha(c::INDIGO_GLOW, 0.4 * fl),
            alpha(c::INDIGO_GLOW, 0.05),
            alpha(c::INDIGO_GLOW, 0.05),
        ],
    );
    glow(
        cx,
        by + bh / 2.0,
        bw * 0.9,
        alpha(c::INDIGO_GLOW, 0.22 * fl),
    );
    rrect_v(
        bx,
        by,
        bw,
        bh,
        5.0,
        alpha(Color::from_hex(0x3b3ea8), a * 0.8),
        alpha(Color::from_hex(0x22206a), a * 0.7),
    );
    let edge = alpha(c::INDIGO_SOFT, 0.9 * fl);
    draw_rectangle_lines(bx, by, bw, bh, 1.2, edge);
    // "menu lines": a cup icon + bars for names and prices
    for i in 0..4 {
        let ly = by + 9.0 + i as f32 * 9.5;
        let lw = bw * (0.38 + hash01(i * 11 + 3) * 0.2);
        ellipse(bx + 7.0, ly, 2.2, 2.2, alpha(c::AMBER, 0.9 * fl));
        rrect(bx + 12.0, ly - 1.4, lw, 2.8, 1.4, alpha(WHITE, 0.75 * fl));
        rrect(
            bx + bw - 13.0,
            ly - 1.4,
            8.0,
            2.8,
            1.4,
            alpha(c::INDIGO_SOFT, 0.9 * fl),
        );
    }
    let scan = (t * 0.6).fract();
    let sy = by + scan * bh;
    draw_line(
        bx + 1.0,
        sy,
        bx + bw - 1.0,
        sy,
        1.0,
        alpha(WHITE, 0.25 * fl),
    );
}

/// Hologram flicker 0.55..1: mostly steady, now and then a short "dip".
pub fn holo_flicker(t: f32, seed: u32) -> f32 {
    let base = 0.92 + noise1(t * 3.0, seed) * 0.08;
    let glitch = noise1(t * 13.0, seed + 99);
    if glitch > 0.86 { base * 0.6 } else { base }
}

fn draw_drone_dock(o: Vec2, w: f32, d: f32, st: &FurnitureState) {
    let cx = o.x + w / 2.0;
    let cy = o.y + d / 2.0;
    let t = st.t;
    soft_shadow(cx + 2.0, cy + 4.0, 24.0, 10.0, 0.4);
    // pad
    ellipse_v(cx, cy + 3.0, 22.0, 9.0, c::STEEL_DARK, c::GRAPHITE);
    ellipse_v(cx, cy, 22.0, 9.0, c::STEEL, c::STEEL_DARK);
    let pulse = (t * 2.0).sin() * 0.5 + 0.5;
    draw_ellipse_lines(
        cx,
        cy,
        15.0,
        6.0,
        0.0,
        1.4,
        alpha(c::INDIGO_GLOW, 0.5 + 0.5 * pulse),
    );
    // charging post
    rrect_v(
        cx + 18.0,
        cy - 26.0,
        6.0,
        26.0,
        2.0,
        c::STEEL_LIGHT,
        c::STEEL_DARK,
    );
    ellipse(
        cx + 21.0,
        cy - 22.0,
        1.6,
        1.6,
        if pulse > 0.5 {
            c::EMERALD
        } else {
            shade(c::EMERALD, 0.4)
        },
    );
    if st.active {
        // the drone hovers and lands
        let hover = 14.0 + (t * 1.8).sin() * 3.0;
        let p = vec2(cx, cy - hover);
        soft_shadow(cx, cy, 10.0, 4.0, 0.25);
        for sx in [-1.0_f32, 1.0] {
            draw_line(p.x, p.y, p.x + sx * 11.0, p.y - 2.0, 1.6, c::GRAPHITE);
            let blur = alpha(c::STEEL_LIGHT, 0.35);
            ellipse(p.x + sx * 11.0, p.y - 3.5, 6.0, 1.4, blur);
        }
        rrect_v(
            p.x - 6.0,
            p.y - 4.0,
            12.0,
            7.0,
            3.0,
            c::INDIGO,
            c::INDIGO_600,
        );
        let blink = (t * 3.0).fract() < 0.15;
        ellipse(
            p.x,
            p.y + 3.5,
            1.4,
            1.4,
            if blink { WHITE } else { c::ROSE },
        );
        // parcel
        rrect_v(
            p.x - 4.0,
            p.y + 3.0,
            8.0,
            6.0,
            1.0,
            Color::from_hex(0xc8a070),
            Color::from_hex(0x9a7448),
        );
    }
}

fn draw_grandma_photo(o: Vec2, w: f32, t: f32) {
    let cx = o.x + w / 2.0;
    // hangs on the wall above its footprint: the frame
    let (fw, fh) = (30.0, 36.0);
    let (fx, fy) = (cx - fw / 2.0, o.y - 66.0);
    rrect(
        fx + 2.0,
        fy + 3.0,
        fw,
        fh,
        3.0,
        Color::new(0.03, 0.02, 0.08, 0.35),
    );
    rrect_v(
        fx,
        fy,
        fw,
        fh,
        3.0,
        Color::from_hex(0xd9a85c),
        Color::from_hex(0x8a6234),
    );
    rrect_v(
        fx + 3.5,
        fy + 3.5,
        fw - 7.0,
        fh - 7.0,
        1.5,
        Color::from_hex(0xe9d4b4),
        Color::from_hex(0xc7a985),
    );
    let lida = PersonLook::preset("grandma_lida");
    draw_portrait(
        &lida,
        Expression::Smile,
        Rect::new(fx + 3.5, fy + 5.0, fw - 7.0, fh - 8.5),
        0.0,
    );
    draw_line(
        fx + 5.0,
        fy + 5.0,
        fx + 12.0,
        fy + 5.0,
        1.0,
        alpha(WHITE, 0.4),
    );
    // a little shelf with a candle and a sprig of lavender
    rrect_v(
        cx - 20.0,
        fy + fh + 6.0,
        40.0,
        4.0,
        1.5,
        c::WOOD_LIGHT,
        c::WOOD_DARK,
    );
    draw_candle(vec2(cx + 10.0, fy + fh + 6.0), t, 5);
    for i in 0..3 {
        let a = -PI / 2.0 - 0.35 + i as f32 * 0.3;
        let b = vec2(cx - 10.0, fy + fh + 6.0);
        let tip = b + vec2(a.cos(), a.sin()) * 12.0;
        draw_line(b.x, b.y, tip.x, tip.y, 0.8, c::LEAF_DARK);
        ellipse(tip.x, tip.y, 1.6, 2.6, c::VIOLET_SOFT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_keys_respect_layers() {
        let o = vec2(128.0, 256.0);
        let rug = furniture_sort_key("rug", o, (3, 2));
        let table = furniture_sort_key("table_round_2", o, (1, 1));
        let lamp = furniture_sort_key("pendant_lamp", o, (1, 1));
        assert!(rug < table && table < lamp);
        assert_eq!(table, o.y + TILE);
        assert_eq!(furniture_sort_key("sofa", o, (2, 1)), o.y + TILE);
        assert_eq!(furniture_sort_key("bookshelf", o, (1, 2)), o.y + 2.0 * TILE);
        assert!(FURNITURE_IDS.contains(&"bucket"));
    }

    #[test]
    fn only_active_lamps_shine() {
        let on = FurnitureState::default();
        let off = FurnitureState {
            active: false,
            ..on
        };
        assert!(!furniture_lights("floor_lamp", Vec2::ZERO, (1, 1), &on).is_empty());
        assert!(furniture_lights("floor_lamp", Vec2::ZERO, (1, 1), &off).is_empty());
        assert!(furniture_lights("sofa", Vec2::ZERO, (2, 1), &on).is_empty());
    }

    #[test]
    fn holo_flicker_bounded() {
        for i in 0..500 {
            let f = holo_flicker(i as f32 * 0.037, 3);
            assert!((0.5..=1.0).contains(&f));
        }
    }
}
