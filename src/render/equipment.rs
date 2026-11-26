//! Hands-on appliance states (design/v0.2.md §B): drawn right after the piece itself (same
//! sort key + a hair), so people in front still cover them. The machine's warm lamp and the
//! tray of shots, the soup pot on the stove, the oven light / timer / the cake, the milk out
//! of the fridge, the counted till, the bag in the bin, a worn machine sputtering — plus the
//! small glowing cue chips over pieces that offer something. Stateless (time `t` in seconds).

use macroquad::prelude::*;

use super::colors as c;
use super::draw::*;
use super::furniture::COUNTER_H;
use super::person::{draw_cup, draw_steam};
use crate::map::TILE;

/// Which appliance (mirrors `sim::EquipKind` without the lamp).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplianceKind {
    Machine,
    Stove,
    Oven,
    Fridge,
    Register,
    Trash,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum PotLook {
    #[default]
    None,
    Warm,
    Bubbling,
    Stirred,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum CakeLook {
    #[default]
    None,
    /// 0–1 of the baking time.
    Baking(f32),
    Ready,
    /// Slices left (of `of`).
    Out {
        left: u32,
        of: u32,
    },
}

/// One appliance's look tonight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ApplianceLook {
    pub kind: ApplianceKind,
    /// The machine was warmed up (its lamp glows).
    pub warm: bool,
    /// Karolina works at it now: 0–1 progress.
    pub working: Option<f32>,
    pub worn: bool,
    pub tray: u32,
    pub pot: PotLook,
    pub cake: CakeLook,
    pub milk_out: bool,
    pub counted: bool,
    pub bag: bool,
}

impl ApplianceLook {
    pub fn new(kind: ApplianceKind) -> Self {
        ApplianceLook {
            kind,
            warm: false,
            working: None,
            worn: false,
            tray: 0,
            pot: PotLook::None,
            cake: CakeLook::None,
            milk_out: false,
            counted: false,
            bag: false,
        }
    }
}

/// The sort key for the state layer of a piece at `tile_origin` (just after the piece).
pub fn appliance_sort_key(tile_origin: Vec2, size: (u8, u8)) -> f32 {
    tile_origin.y + size.1.max(1) as f32 * TILE + 0.05
}

/// Draws the state over the piece (world px of its footprint's top-left, like
/// `draw_furniture`).
pub fn draw_appliance_state(tile_origin: Vec2, size: (u8, u8), look: &ApplianceLook, t: f32) {
    let o = tile_origin;
    let w = size.0.max(1) as f32 * TILE;
    let d = size.1.max(1) as f32 * TILE;
    let cx = o.x + w / 2.0;
    match look.kind {
        ApplianceKind::Machine => machine(o, w, d, cx, look, t),
        ApplianceKind::Stove => stove(o, w, d, look, t),
        ApplianceKind::Oven => oven(o, w, d, cx, look, t),
        ApplianceKind::Fridge => fridge(o, w, d, look, t),
        ApplianceKind::Register => register(o, d, cx, look, t),
        ApplianceKind::Trash => trash(o, d, cx, look, t),
    }
}

fn machine(o: Vec2, w: f32, d: f32, cx: f32, look: &ApplianceLook, t: f32) {
    let top = o.y + 6.0 + (d - 10.0) * 0.45 - COUNTER_H;
    let (mw, mh) = (w * 0.72, 30.0);
    let mx = cx - mw / 2.0;
    // the little "ready" lamp on the front: amber once it's warm
    let lamp = vec2(mx + 6.0, top - mh + 7.0);
    if look.warm {
        let k = 0.85 + 0.15 * (t * 1.7).sin();
        glow(lamp.x, lamp.y, 9.0, alpha(c::AMBER, 0.55 * k));
        ellipse(lamp.x, lamp.y, 2.2, 2.2, mix(c::AMBER, WHITE, 0.35));
    } else {
        ellipse(lamp.x, lamp.y, 2.0, 2.0, shade(c::GRAPHITE, 1.2));
    }
    // warming up / purging: it hisses — big puffs from the steam wand and the group heads
    if let Some(p) = look.working {
        let k = 0.6 + 0.4 * p;
        draw_steam(vec2(mx + mw - 2.0, top - mh - 2.0), t * 1.6, 1.4 * k);
        draw_steam(vec2(cx + mw * 0.22, top - 12.0), t * 1.9 + 0.7, 1.1 * k);
        for i in 0..3 {
            let a = ((t * 6.0 + i as f32 * 0.9) % 1.0).clamp(0.0, 1.0);
            let x = mx + mw + 2.0 + a * 10.0;
            let y = top - mh + 6.0 + i as f32 * 4.0 - a * 3.0;
            draw_line(
                x,
                y,
                x + 4.0,
                y - 1.0,
                1.0,
                Color::new(0.9, 0.92, 1.0, 0.5 * (1.0 - a)),
            );
        }
        if !look.warm {
            glow(lamp.x, lamp.y, 7.0, alpha(c::AMBER, 0.3 * p));
        }
    }
    // a worn machine sputters now and then: grey puffs and a flicker
    if look.worn && look.working.is_none() {
        let cyc = (t * 0.45) % 1.0;
        if cyc < 0.22 {
            let a = cyc / 0.22;
            for i in 0..3 {
                let x = cx - mw * 0.22 + (i as f32 - 1.0) * 5.0 + a * 4.0;
                let y = top - 13.0 - a * 14.0 - i as f32 * 3.0;
                glow(
                    x,
                    y,
                    3.0 + a * 5.0,
                    Color::new(0.55, 0.55, 0.62, 0.45 * (1.0 - a)),
                );
            }
            ellipse(cx, top - mh + 6.5, 3.0, 1.6, alpha(c::ROSE, 0.7));
        }
    }
    // the tray pulled ahead: little cups lined up on the drip tray front
    for i in 0..look.tray.min(3) {
        let p = vec2(cx - 11.0 + i as f32 * 11.0, top + 5.0);
        draw_cup(p, 0.75, t + i as f32 * 0.4, true);
    }
}

fn stove(o: Vec2, w: f32, d: f32, look: &ApplianceLook, t: f32) {
    if look.pot == PotLook::None && look.working.is_none() {
        return;
    }
    let top_y = o.y + 6.0 - COUNTER_H;
    let dd = d - 10.0;
    let p = vec2(o.x + w * 0.7, top_y + dd * 0.32);
    // the burner glows under the pot
    glow(p.x, p.y + 2.0, 16.0, alpha(c::AMBER_DEEP, 0.4));
    if look.pot == PotLook::None {
        // putting it on: the pot is being set down
        let k = look.working.unwrap_or(0.0);
        pot(
            p - vec2(0.0, 10.0 * (1.0 - k)),
            PotLook::Warm,
            t,
            k.max(0.3),
        );
        return;
    }
    pot(p, look.pot, t, 1.0);
}

/// A copper soup pot standing at `p` (its base on the burner).
fn pot(p: Vec2, state: PotLook, t: f32, a: f32) {
    let copper_t = Color::from_hex(0xd9875a);
    let copper_b = Color::from_hex(0x8f4a2c);
    let (pw, ph) = (26.0, 18.0);
    soft_shadow(p.x, p.y + 1.0, pw * 0.5, 4.0, 0.35 * a);
    rrect_v(
        p.x - pw / 2.0,
        p.y - ph,
        pw,
        ph,
        4.0,
        alpha(copper_t, a),
        alpha(copper_b, a),
    );
    // handles
    for s in [-1.0, 1.0] {
        draw_line(
            p.x + s * pw / 2.0,
            p.y - ph + 5.0,
            p.x + s * (pw / 2.0 + 4.0),
            p.y - ph + 4.0,
            2.2,
            alpha(shade(copper_b, 0.8), a),
        );
    }
    // a warm highlight
    draw_line(
        p.x - pw / 2.0 + 4.0,
        p.y - ph + 4.0,
        p.x - pw / 2.0 + 4.0,
        p.y - 4.0,
        1.4,
        alpha(Color::from_hex(0xffd2a8), 0.5 * a),
    );
    let rim = p.y - ph;
    match state {
        PotLook::Bubbling => {
            // the lid rattles a little, bubbles at the rim, lots of steam
            // (the lid tilts up on one side so the foam shows)
            let jig = (t * 18.0).sin().max(0.0) * 1.6;
            soup_surface(p.x, rim, pw, t, true);
            for i in 0..3 {
                let ph = (t * 2.4 + i as f32 * 0.33) % 1.0;
                let bx = p.x - 9.0 + i as f32 * 8.0;
                draw_circle(
                    bx,
                    rim - 1.0 - ph * 5.0,
                    1.4 + ph,
                    Color::new(1.0, 0.97, 0.9, 0.85 * (1.0 - ph)),
                );
            }
            lid(p.x + 7.0, rim - 6.0 - jig, pw * 0.8, a);
            draw_steam(vec2(p.x - 6.0, rim - 4.0), t * 1.4, 1.3);
            draw_steam(vec2(p.x + 6.0, rim - 4.0), t * 1.4 + 0.9, 1.1);
        }
        PotLook::Stirred => {
            // the lid off, a ladle in the soup
            soup_surface(p.x, rim, pw, t, false);
            let ang = (t * 1.4).sin() * 0.25;
            let tip = vec2(p.x + ang * 6.0, rim + 1.0);
            draw_line(
                tip.x,
                tip.y,
                tip.x + 12.0,
                tip.y - 16.0,
                2.0,
                c::STEEL_LIGHT,
            );
            draw_steam(vec2(p.x, rim - 3.0), t, 1.0);
        }
        _ => {
            lid(p.x, rim - 1.0, pw, a);
            draw_steam(vec2(p.x + 2.0, rim - 5.0), t, 0.7);
        }
    }
}

fn lid(x: f32, y: f32, pw: f32, a: f32) {
    ellipse_v(
        x,
        y,
        pw / 2.0 + 1.0,
        4.0,
        alpha(c::STEEL_LIGHT, a),
        alpha(c::STEEL, a),
    );
    ellipse(x, y - 3.0, 2.6, 1.8, alpha(c::GRAPHITE, a));
}

fn soup_surface(x: f32, y: f32, pw: f32, t: f32, bubbles: bool) {
    // kulajda: creamy, dill flecks
    ellipse(x, y, pw / 2.0 - 1.0, 3.4, Color::from_hex(0xf0dfb0));
    for i in 0..4 {
        let fx = x - 7.0 + i as f32 * 4.6;
        ellipse(fx, y + ((i % 2) as f32 - 0.5), 1.0, 0.6, c::LEAF);
    }
    if bubbles {
        for i in 0..4 {
            let ph = (t * 1.8 + i as f32 * 0.37) % 1.0;
            let bx = x - 8.0 + i as f32 * 5.3;
            let r = 1.0 + ph * 1.8;
            draw_circle_lines(
                bx,
                y - ph * 1.5,
                r,
                0.8,
                Color::new(1.0, 0.97, 0.9, 0.8 * (1.0 - ph)),
            );
        }
    }
}

fn oven(o: Vec2, w: f32, d: f32, cx: f32, look: &ApplianceLook, t: f32) {
    let fy = o.y + 6.0 + d - 10.0 - COUNTER_H;
    let win = Rect::new(o.x + 8.0, fy + 8.0, w - 16.0, COUNTER_H - 14.0);
    match look.cake {
        CakeLook::Baking(p) => {
            let k = 0.75 + 0.25 * (t * 2.2).sin();
            rrect(
                win.x,
                win.y,
                win.w,
                win.h,
                3.0,
                alpha(Color::from_hex(0xffa64d), 0.55 * k),
            );
            glow(cx, win.y + win.h / 2.0, 26.0, alpha(c::AMBER, 0.35 * k));
            // the cake rising behind the glass
            ellipse(
                cx,
                win.y + win.h - 3.0,
                10.0,
                2.5 + 2.0 * p,
                Color::from_hex(0x9a5a2c),
            );
            timer(vec2(cx, o.y - 8.0), p, t, false);
        }
        CakeLook::Ready => {
            rrect(
                win.x,
                win.y,
                win.w,
                win.h,
                3.0,
                alpha(Color::from_hex(0xffc46b), 0.8),
            );
            glow(cx, win.y + win.h / 2.0, 30.0, alpha(c::AMBER, 0.45));
            timer(vec2(cx, o.y - 8.0), 1.0, t, true);
        }
        CakeLook::Out { left, of } => {
            // grandma's medovník on a stand on top of the oven
            let top = o.y + 6.0 + (d - 10.0) * 0.5 - COUNTER_H;
            cake(vec2(cx, top + 2.0), left, of.max(1), t);
        }
        CakeLook::None => {
            if look.working.is_some() {
                rrect(
                    win.x,
                    win.y,
                    win.w,
                    win.h,
                    3.0,
                    alpha(Color::from_hex(0xffa64d), 0.35),
                );
            }
        }
    }
}

/// A round kitchen timer over the oven: the dial fills; ringing = a little bell with rings.
fn timer(c0: Vec2, p: f32, t: f32, ringing: bool) {
    let jig = if ringing { (t * 22.0).sin() * 1.4 } else { 0.0 };
    let c0 = c0 + vec2(jig, 0.0);
    draw_circle(c0.x, c0.y, 7.5, Color::new(0.11, 0.1, 0.24, 0.85));
    let segs = 20;
    for i in 0..(segs as f32 * p.clamp(0.0, 1.0)) as usize {
        let a0 = -std::f32::consts::FRAC_PI_2 + i as f32 / segs as f32 * std::f32::consts::TAU;
        let a1 = a0 + std::f32::consts::TAU / segs as f32;
        draw_line(
            c0.x + a0.cos() * 6.0,
            c0.y + a0.sin() * 6.0,
            c0.x + a1.cos() * 6.0,
            c0.y + a1.sin() * 6.0,
            2.0,
            c::AMBER,
        );
    }
    if ringing {
        for k in 0..2 {
            let a = ((t * 1.6 + k as f32 * 0.5) % 1.0).clamp(0.0, 1.0);
            draw_circle_lines(
                c0.x,
                c0.y,
                8.0 + a * 10.0,
                1.0,
                alpha(c::AMBER, 0.7 * (1.0 - a)),
            );
        }
        // a tiny bell glyph
        poly(
            &[
                vec2(c0.x - 3.0, c0.y + 2.0),
                vec2(c0.x + 3.0, c0.y + 2.0),
                vec2(c0.x + 2.0, c0.y - 2.5),
                vec2(c0.x - 2.0, c0.y - 2.5),
            ],
            c::AMBER,
        );
        ellipse(c0.x, c0.y + 3.0, 1.0, 1.0, c::AMBER);
    }
}

/// A honey cake on a stand, `left` of `of` slices still there (a pie).
fn cake(base: Vec2, left: u32, of: u32, t: f32) {
    let _ = t;
    soft_shadow(base.x, base.y + 1.0, 14.0, 3.5, 0.35);
    // the stand
    draw_line(base.x, base.y, base.x, base.y - 5.0, 2.0, c::CREAM);
    ellipse(base.x, base.y - 5.0, 14.0, 3.6, shade(c::CREAM, 0.9));
    let top = base.y - 9.0;
    let frac = left as f32 / of as f32;
    if frac <= 0.0 {
        return;
    }
    // the side (layers of honey sponge and cream)
    let n = 18;
    let a_end = std::f32::consts::TAU * frac;
    let pts: Vec<Vec2> = (0..=n)
        .map(|i| {
            let a = -std::f32::consts::FRAC_PI_2 + a_end * i as f32 / n as f32;
            vec2(base.x + a.cos() * 11.0, top + a.sin() * 3.6)
        })
        .collect();
    for (k, col) in [
        (4.0, Color::from_hex(0xa8642f)),
        (2.5, Color::from_hex(0xf3e1b6)),
        (1.0, Color::from_hex(0xb8733a)),
    ] {
        for p in &pts {
            draw_line(p.x, p.y + 1.0, p.x, p.y + k, 1.6, col);
        }
    }
    // the top
    let mut fan = vec![vec2(base.x, top)];
    fan.extend(pts.iter().copied());
    poly(&fan, Color::from_hex(0xc98a4c));
    // a crumb pattern
    for i in 0..3 {
        ellipse(
            base.x - 4.0 + i as f32 * 4.0,
            top + ((i % 2) as f32 - 0.5),
            1.0,
            0.6,
            Color::from_hex(0x7a4520),
        );
    }
}

fn fridge(o: Vec2, w: f32, d: f32, look: &ApplianceLook, t: f32) {
    let (x, y0) = (o.x + 6.0, o.y + 8.0);
    let (fw, fd, h) = (w - 12.0, d - 14.0, 80.0);
    let fy = y0 + fd - h;
    if let Some(p) = look.working {
        // the door a little open: cold light spills out
        let k = (p * 3.0).min(1.0);
        glow(
            x + fw * 0.5,
            fy + 40.0,
            24.0,
            Color::new(0.75, 0.9, 1.0, 0.35 * k),
        );
        rect_v(
            x + fw - 2.0,
            fy + 4.0,
            6.0 * k,
            h - 10.0,
            Color::new(0.95, 1.0, 1.0, 0.7),
            Color::new(0.7, 0.85, 0.95, 0.6),
        );
    }
    if look.milk_out {
        // the milk bottle out on top, a drop of condensation
        let b = vec2(x + 10.0, fy + 2.0);
        rrect_v(
            b.x - 4.0,
            b.y - 14.0,
            8.0,
            14.0,
            3.0,
            Color::new(0.98, 0.98, 1.0, 0.95),
            Color::new(0.85, 0.88, 0.95, 0.95),
        );
        rrect(
            b.x - 2.5,
            b.y - 18.0,
            5.0,
            4.0,
            1.0,
            Color::new(0.9, 0.92, 1.0, 0.8),
        );
        rrect(b.x - 2.8, b.y - 19.5, 5.6, 2.0, 0.8, c::INDIGO);
        rrect(b.x - 4.0, b.y - 9.0, 8.0, 3.0, 0.5, c::INDIGO_SOFT);
        let drop = (t * 0.5) % 1.0;
        ellipse(
            b.x + 3.0,
            b.y - 12.0 + drop * 10.0,
            0.8,
            1.1,
            alpha(WHITE, 0.6 * (1.0 - drop)),
        );
    }
}

fn register(o: Vec2, d: f32, cx: f32, look: &ApplianceLook, t: f32) {
    let top = o.y + 6.0 + (d - 10.0) * 0.5 - COUNTER_H;
    if let Some(p) = look.working {
        // counting: notes flick one after another
        let n = (p * 6.0) as i32;
        for i in 0..n.min(5) {
            rrect(
                cx - 20.0,
                top - 4.0 - i as f32 * 1.6,
                12.0,
                5.0,
                1.0,
                mix(Color::from_hex(0x8fd1b0), WHITE, 0.15 * (i % 2) as f32),
            );
        }
        glow(
            cx,
            top - 20.0,
            18.0,
            alpha(c::INDIGO_GLOW, 0.3 + 0.2 * (t * 8.0).sin().abs()),
        );
    }
    if look.counted {
        // a neat stack with a paper band, and a longer receipt
        for i in 0..4 {
            rrect(
                cx - 21.0,
                top - 3.0 - i as f32 * 1.7,
                12.0,
                5.0,
                1.0,
                Color::from_hex(0x8fd1b0),
            );
        }
        rect_v(
            cx - 17.0,
            top - 10.0,
            3.0,
            7.0,
            c::CREAM,
            shade(c::CREAM, 0.85),
        );
        rect_v(
            cx + 15.0,
            top - 9.0,
            6.0,
            16.0,
            c::CREAM,
            shade(c::CREAM, 0.8),
        );
        for k in 0..3 {
            draw_line(
                cx + 16.0,
                top - 6.0 + k as f32 * 4.0,
                cx + 20.0,
                top - 6.0 + k as f32 * 4.0,
                0.8,
                alpha(c::GRAPHITE, 0.6),
            );
        }
    }
}

fn trash(o: Vec2, d: f32, cx: f32, look: &ApplianceLook, t: f32) {
    let cy = o.y + d / 2.0;
    let fy = cy + 10.0;
    if look.bag {
        // the full bag's knot peeks over the rim
        let sway = (t * 1.3).sin() * 0.6;
        ellipse_v(
            cx + sway,
            fy - 33.0,
            9.0,
            5.0,
            Color::from_hex(0x2a2b3a),
            Color::from_hex(0x15161f),
        );
        capsule(
            vec2(cx - 2.0 + sway, fy - 37.0),
            vec2(cx + 2.0 + sway, fy - 41.0),
            1.8,
            Color::from_hex(0x2a2b3a),
        );
        draw_line(
            cx - 4.0,
            fy - 34.0,
            cx + 3.0,
            fy - 36.0,
            0.8,
            alpha(WHITE, 0.25),
        );
    }
}

/// What a cue chip shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipGlyph {
    Cup,
    Pot,
    Cake,
    Milk,
    Coins,
    Bag,
    /// Purge / service: a little puff.
    Puff,
}

/// A small glowing chip over an appliance that offers something (click it, or **E**).
pub fn draw_equip_cue(c0: Vec2, glyph: EquipGlyph, t: f32, hovered: bool, urgent: bool) {
    let bob = (t * 2.0).sin() * 1.5;
    let c0 = c0 + vec2(0.0, bob);
    let pulse = 0.5 + 0.5 * (t * if urgent { 5.0 } else { 2.5 }).sin();
    glow(
        c0.x,
        c0.y,
        15.0 + pulse * 3.0,
        alpha(c::AMBER, if hovered { 0.35 } else { 0.18 + 0.1 * pulse }),
    );
    draw_circle(c0.x, c0.y, 9.5, Color::new(0.11, 0.1, 0.24, 0.92));
    draw_circle_lines(
        c0.x,
        c0.y,
        9.5,
        1.3,
        alpha(mix(c::AMBER, WHITE, 0.25), 0.8 + 0.2 * pulse),
    );
    let ink = mix(c::AMBER, WHITE, 0.35);
    match glyph {
        EquipGlyph::Cup => {
            rrect(c0.x - 4.0, c0.y - 2.0, 7.0, 5.5, 1.5, ink);
            draw_circle_lines(c0.x + 3.6, c0.y + 0.5, 1.6, 0.9, ink);
            draw_line(c0.x - 1.0, c0.y - 4.0, c0.x - 0.4, c0.y - 6.0, 0.9, ink);
            draw_line(c0.x + 1.2, c0.y - 4.0, c0.x + 1.8, c0.y - 6.0, 0.9, ink);
        }
        EquipGlyph::Pot => {
            rrect(c0.x - 5.0, c0.y - 2.0, 10.0, 6.0, 1.5, ink);
            draw_line(c0.x - 6.0, c0.y - 2.5, c0.x + 6.0, c0.y - 2.5, 1.2, ink);
            ellipse(c0.x, c0.y - 4.0, 1.4, 1.0, ink);
        }
        EquipGlyph::Cake => {
            poly(
                &[
                    vec2(c0.x - 5.5, c0.y + 3.5),
                    vec2(c0.x + 5.5, c0.y + 3.5),
                    vec2(c0.x + 5.5, c0.y - 0.5),
                    vec2(c0.x - 5.5, c0.y - 2.5),
                ],
                ink,
            );
            draw_line(
                c0.x - 5.5,
                c0.y + 1.0,
                c0.x + 5.5,
                c0.y + 1.5,
                0.8,
                c::GRAPHITE,
            );
        }
        EquipGlyph::Milk => {
            rrect(c0.x - 3.0, c0.y - 2.0, 6.0, 7.0, 1.5, ink);
            rrect(c0.x - 1.8, c0.y - 5.0, 3.6, 3.0, 0.6, ink);
        }
        EquipGlyph::Coins => {
            for (dx, dy) in [(-2.5, 2.0), (2.5, 2.0), (0.0, -1.5)] {
                ellipse(c0.x + dx, c0.y + dy, 3.0, 1.8, ink);
            }
        }
        EquipGlyph::Bag => {
            ellipse(c0.x, c0.y + 1.5, 4.5, 3.8, ink);
            capsule(
                vec2(c0.x - 1.0, c0.y - 2.0),
                vec2(c0.x + 1.0, c0.y - 4.5),
                1.0,
                ink,
            );
        }
        EquipGlyph::Puff => {
            for (dx, dy, r) in [(-2.5, 1.0, 2.6), (1.5, 0.5, 3.0), (0.0, -2.0, 2.4)] {
                draw_circle(c0.x + dx, c0.y + dy, r, ink);
            }
        }
    }
}

/// A progress ring over a piece while Karolina works there; `hold` adds a soft pulse
/// ("hold to go faster").
pub fn draw_work_ring(c0: Vec2, progress: f32, hold: bool, held: bool, t: f32) {
    let r = 11.0;
    draw_circle(c0.x, c0.y, r, Color::new(0.11, 0.1, 0.24, 0.85));
    let base = if held { 0.9 } else { 0.5 };
    draw_circle_lines(c0.x, c0.y, r, 1.3, Color::new(0.65, 0.7, 1.0, base * 0.6));
    let segs = 24;
    for i in 0..(segs as f32 * progress.clamp(0.0, 1.0)) as usize {
        let a0 = -std::f32::consts::FRAC_PI_2 + i as f32 / segs as f32 * std::f32::consts::TAU;
        let a1 = a0 + std::f32::consts::TAU / segs as f32;
        draw_line(
            c0.x + a0.cos() * r,
            c0.y + a0.sin() * r,
            c0.x + a1.cos() * r,
            c0.y + a1.sin() * r,
            2.4,
            if held {
                c::AMBER
            } else {
                Color::new(0.65, 0.7, 1.0, 1.0)
            },
        );
    }
    if hold && !held {
        let p = (t * 2.0) % 1.0;
        draw_circle_lines(
            c0.x,
            c0.y,
            r + 2.0 + p * 6.0,
            1.0,
            alpha(c::AMBER, 0.5 * (1.0 - p)),
        );
    }
}
