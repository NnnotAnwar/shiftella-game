//! Helpers at work in the world: a small, muted progress ring over a helper's head with the
//! task's icon (Karolina's queue chips stay the primary, brighter ring), and the little props
//! of their idle pastimes (a cup and cloth, a broom, a glass of water, a phone…).
//!
//! Stateless like `handson`: positions come from the caller, the picture is a function of
//! `t`. Draw the rings after `Lighting::apply` (with the emotes) so they stay readable at
//! night; they scale with [`world_icon_scale`] when the camera is zoomed out.

use std::f32::consts::{PI, TAU};

use macroquad::prelude::*;

use super::colors as c;
use super::draw::*;
use super::handson::{QueueIcon, draw_queue_icon, world_icon_scale};
use super::look::Facing;

/// What a helper's pastime looks like (its ring icon and the prop in hand).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PastimeIcon {
    /// A cloth: wiping the counter / a table.
    Cloth,
    /// A cup being polished.
    Cup,
    /// A stack of saucers.
    Saucers,
    /// A pastry (checking the case).
    Pastry,
    /// A napkin holder.
    Napkin,
    /// A chair being straightened.
    Chair,
    /// A sugar pot.
    Sugar,
    /// A broom.
    Broom,
    /// A glass of water.
    Water,
    /// Two speech dots: a chat.
    Chat,
    /// A phone.
    Phone,
    /// A window: watching the train.
    Window,
    /// A paw: petting the cat.
    Paw,
}

/// The ring's icon: a task (the same pictures as Karolina's queue chips) or a pastime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelperIcon {
    Task(QueueIcon),
    Pastime(PastimeIcon),
}

/// Diameter of the helper chip in world px (× `world_icon_scale`); Karolina's is 16.
pub const HELPER_CHIP: f32 = 12.0;

/// A helper's work ring over the head. `anchor` = `emote_anchor` of the person; `progress`
/// 0–1. `pastime` rings are a little more transparent than real work.
pub fn draw_helper_ring(anchor: Vec2, icon: HelperIcon, progress: f32, t: f32) {
    let s = world_icon_scale();
    let r = HELPER_CHIP * 0.5 * s;
    let c0 = anchor - vec2(0.0, 4.0 * s + r);
    let pastime = matches!(icon, HelperIcon::Pastime(_));
    let a = if pastime { 0.72 } else { 0.9 };
    soft_shadow(c0.x, c0.y + 3.0 * s, 8.0 * s, 5.0 * s, 0.22 * a);
    // a dim glass disc with a faint rim
    draw_circle(c0.x, c0.y, r + 1.2 * s, alpha(Color::from_hex(0x15132c), 0.8 * a));
    draw_circle(c0.x, c0.y, r, alpha(Color::from_hex(0x242046), 0.92 * a));
    draw_circle_lines(c0.x, c0.y, r + 1.2 * s, 0.8 * s, alpha(c::LAVENDER_RIM, 0.25 * a));
    match icon {
        HelperIcon::Task(q) => draw_queue_icon(c0, 8.0 * s, q, t),
        HelperIcon::Pastime(p) => draw_pastime_icon(c0, 8.0 * s, p, t),
    }
    // the ring: muted lavender (Karolina's is indigo glow with a bright head)
    let p = progress.clamp(0.0, 1.0);
    let a0 = -PI / 2.0;
    let rr = r + 1.9 * s;
    arc(
        c0,
        rr,
        rr,
        a0,
        a0 + TAU,
        1.1 * s,
        alpha(c::INDIGO_SOFT, 0.16 * a),
    );
    if p > 0.0 {
        let col = if pastime {
            alpha(c::LAVENDER_RIM, 0.62)
        } else {
            alpha(c::INDIGO_SOFT, 0.85)
        };
        arc(c0, rr, rr, a0, a0 + TAU * p, 1.5 * s, col);
        let e = a0 + TAU * p;
        ellipse(
            c0.x + e.cos() * rr,
            c0.y + e.sin() * rr,
            1.3 * s,
            1.3 * s,
            alpha(WHITE, 0.55 * a),
        );
    }
}

/// The small picture of a pastime (inside the ring; any camera).
pub fn draw_pastime_icon(c0: Vec2, size: f32, icon: PastimeIcon, t: f32) {
    let u = size / 11.0;
    let ink = Color::from_hex(0xe9e6ff);
    match icon {
        PastimeIcon::Cloth => {
            rrect_v(
                c0.x - 4.0 * u,
                c0.y - 2.5 * u,
                8.0 * u,
                5.5 * u,
                1.4 * u,
                Color::from_hex(0x9fd6e6),
                Color::from_hex(0x5ea3bd),
            );
            for i in 0..2 {
                let y = c0.y - 0.8 * u + i as f32 * 2.0 * u;
                draw_line(c0.x - 3.0 * u, y, c0.x + 3.0 * u, y, 0.6 * u, alpha(WHITE, 0.6));
            }
            // a swipe
            let k = (t * 3.0).sin() * 1.5 * u;
            arc(
                c0 + vec2(k, 4.5 * u),
                4.0 * u,
                1.4 * u,
                0.2,
                PI - 0.2,
                0.7 * u,
                alpha(ink, 0.6),
            );
        }
        PastimeIcon::Cup => {
            rrect_v(
                c0.x - 3.2 * u,
                c0.y - 2.0 * u,
                6.4 * u,
                5.5 * u,
                1.6 * u,
                c::CREAM,
                shade(c::CREAM, 0.75),
            );
            arc(
                c0 + vec2(3.4 * u, 0.6 * u),
                1.6 * u,
                1.6 * u,
                -PI / 2.0,
                PI / 2.0,
                0.8 * u,
                c::CREAM,
            );
            // a glint going round
            let g = (t * 2.0) % 1.0;
            glow(
                c0.x - 2.0 * u + g * 3.5 * u,
                c0.y - 0.5 * u,
                1.8 * u,
                alpha(WHITE, 0.7),
            );
        }
        PastimeIcon::Saucers => {
            for i in 0..3 {
                let y = c0.y + 3.0 * u - i as f32 * 2.2 * u;
                ellipse(c0.x, y, 4.6 * u, 1.3 * u, shade(c::CREAM, 0.7));
                ellipse(c0.x, y - 0.4 * u, 4.2 * u, 1.0 * u, c::CREAM);
            }
        }
        PastimeIcon::Pastry => {
            ellipse(c0.x, c0.y + 1.0 * u, 4.8 * u, 3.0 * u, Color::from_hex(0xd08b4c));
            ellipse(c0.x, c0.y, 4.0 * u, 2.2 * u, Color::from_hex(0xe9b06a));
            for i in -1..=1 {
                draw_line(
                    c0.x + i as f32 * 1.6 * u - 0.6 * u,
                    c0.y - 1.4 * u,
                    c0.x + i as f32 * 1.6 * u + 0.6 * u,
                    c0.y + 1.4 * u,
                    0.6 * u,
                    Color::from_hex(0xa8642f),
                );
            }
        }
        PastimeIcon::Napkin => {
            rrect(
                c0.x - 3.6 * u,
                c0.y + 0.5 * u,
                7.2 * u,
                3.2 * u,
                0.8 * u,
                c::STEEL,
            );
            poly(
                &[
                    c0 + vec2(-2.6 * u, 0.8 * u),
                    c0 + vec2(0.0, -4.2 * u),
                    c0 + vec2(2.6 * u, 0.8 * u),
                ],
                WHITE,
            );
        }
        PastimeIcon::Chair => {
            let col = c::WOOD_LIGHT;
            draw_line(c0.x - 2.5 * u, c0.y - 4.0 * u, c0.x - 2.5 * u, c0.y + 4.0 * u, 1.1 * u, col);
            draw_line(c0.x - 2.5 * u, c0.y + 0.5 * u, c0.x + 3.0 * u, c0.y + 0.5 * u, 1.3 * u, col);
            draw_line(c0.x + 3.0 * u, c0.y + 0.5 * u, c0.x + 3.0 * u, c0.y + 4.0 * u, 1.1 * u, col);
        }
        PastimeIcon::Sugar => {
            rrect_v(
                c0.x - 3.0 * u,
                c0.y - 1.5 * u,
                6.0 * u,
                5.0 * u,
                1.4 * u,
                WHITE,
                shade(WHITE, 0.78),
            );
            ellipse(c0.x, c0.y - 2.0 * u, 3.2 * u, 1.0 * u, c::FABRIC_ROSE);
            ellipse(c0.x, c0.y - 3.2 * u, 0.9 * u, 0.9 * u, c::FABRIC_ROSE);
        }
        PastimeIcon::Broom => {
            let sway = (t * 4.0).sin() * 0.25;
            let top = c0 + vec2(2.5 * u + sway * 2.0 * u, -4.5 * u);
            let bot = c0 + vec2(-1.0 * u, 2.0 * u);
            draw_line(top.x, top.y, bot.x, bot.y, 0.9 * u, c::WOOD_LIGHT);
            poly(
                &[
                    bot + vec2(-2.8 * u, 0.0),
                    bot + vec2(1.6 * u, 0.0),
                    bot + vec2(2.4 * u, 3.0 * u),
                    bot + vec2(-3.6 * u, 3.0 * u),
                ],
                c::AMBER_DEEP,
            );
        }
        PastimeIcon::Water => {
            poly(
                &[
                    c0 + vec2(-3.0 * u, -4.0 * u),
                    c0 + vec2(3.0 * u, -4.0 * u),
                    c0 + vec2(2.2 * u, 4.0 * u),
                    c0 + vec2(-2.2 * u, 4.0 * u),
                ],
                alpha(Color::from_hex(0xcfe9ff), 0.55),
            );
            poly(
                &[
                    c0 + vec2(-2.6 * u, -0.5 * u),
                    c0 + vec2(2.6 * u, -0.5 * u),
                    c0 + vec2(2.2 * u, 4.0 * u),
                    c0 + vec2(-2.2 * u, 4.0 * u),
                ],
                alpha(Color::from_hex(0x7cc4f2), 0.85),
            );
        }
        PastimeIcon::Chat => {
            rrect(
                c0.x - 4.5 * u,
                c0.y - 3.0 * u,
                9.0 * u,
                5.5 * u,
                2.2 * u,
                ink,
            );
            poly(
                &[
                    c0 + vec2(-2.0 * u, 2.3 * u),
                    c0 + vec2(-3.4 * u, 4.5 * u),
                    c0 + vec2(0.0, 2.3 * u),
                ],
                ink,
            );
            for i in 0..3 {
                let on = ((t * 2.5) as i32 % 3) == i;
                ellipse(
                    c0.x + (i as f32 - 1.0) * 2.4 * u,
                    c0.y - 0.3 * u,
                    0.8 * u,
                    0.8 * u,
                    if on { c::INDIGO } else { alpha(c::INDIGO, 0.5) },
                );
            }
        }
        PastimeIcon::Phone => {
            rrect(
                c0.x - 2.5 * u,
                c0.y - 4.2 * u,
                5.0 * u,
                8.4 * u,
                1.2 * u,
                c::GRAPHITE,
            );
            rrect(
                c0.x - 1.8 * u,
                c0.y - 3.4 * u,
                3.6 * u,
                6.4 * u,
                0.6 * u,
                Color::from_hex(0x8fd3ff),
            );
        }
        PastimeIcon::Window => {
            rrect(
                c0.x - 4.0 * u,
                c0.y - 4.0 * u,
                8.0 * u,
                8.0 * u,
                1.0 * u,
                c::WOOD_DARK,
            );
            rrect(
                c0.x - 3.2 * u,
                c0.y - 3.2 * u,
                6.4 * u,
                6.4 * u,
                0.6 * u,
                Color::from_hex(0x2c3d6e),
            );
            // a lit train sliding past
            let k = ((t * 0.8) % 1.0) * 9.0 - 4.5;
            rrect(
                c0.x + k * u - 2.0 * u,
                c0.y + 0.2 * u,
                4.0 * u,
                1.6 * u,
                0.5 * u,
                c::AMBER,
            );
            draw_line(c0.x, c0.y - 3.2 * u, c0.x, c0.y + 3.2 * u, 0.6 * u, c::WOOD_DARK);
        }
        PastimeIcon::Paw => {
            let col = Color::from_hex(0xf0c9a0);
            ellipse(c0.x, c0.y + 1.6 * u, 2.6 * u, 2.1 * u, col);
            for (dx, dy) in [(-2.6, -1.4), (-0.9, -2.9), (0.9, -2.9), (2.6, -1.4)] {
                ellipse(c0.x + dx * u, c0.y + dy * u, 1.0 * u, 1.2 * u, col);
            }
        }
    }
}

/// The prop of a pastime in the person's hands (drawn right after the person, sorted with
/// them). `feet` = the person's feet, `k` = their height factor.
pub fn draw_pastime_prop(feet: Vec2, facing: Facing, k: f32, icon: PastimeIcon, t: f32) {
    // hands in front of the waist (from behind they're hidden by the body)
    if facing == Facing::Up {
        return;
    }
    let side = match facing {
        Facing::Left => -1.0,
        Facing::Right => 1.0,
        _ => 0.0,
    };
    let hand = feet + vec2(side * 9.0 + if side == 0.0 { 7.0 } else { 0.0 }, -36.0 * k);
    match icon {
        PastimeIcon::Cup => {
            let wob = (t * 6.0).sin() * 0.8;
            rrect_v(
                hand.x - 3.0,
                hand.y - 4.0 + wob,
                6.0,
                5.5,
                1.6,
                c::CREAM,
                shade(c::CREAM, 0.75),
            );
            rrect(hand.x - 4.5, hand.y + 0.5, 9.0, 2.4, 1.0, Color::from_hex(0x9fd6e6));
        }
        PastimeIcon::Saucers => {
            for i in 0..3 {
                let y = hand.y + 1.0 - i as f32 * 1.6;
                ellipse(hand.x, y, 5.0, 1.2, shade(c::CREAM, 0.72));
                ellipse(hand.x, y - 0.4, 4.6, 0.9, c::CREAM);
            }
        }
        PastimeIcon::Cloth => {
            let k2 = (t * 5.0).sin() * 2.0;
            rrect(hand.x - 3.5 + k2, hand.y - 1.0, 7.0, 3.5, 1.2, Color::from_hex(0x9fd6e6));
        }
        PastimeIcon::Water => {
            poly(
                &[
                    hand + vec2(-2.4, -6.0),
                    hand + vec2(2.4, -6.0),
                    hand + vec2(1.8, 0.5),
                    hand + vec2(-1.8, 0.5),
                ],
                alpha(Color::from_hex(0xcfe9ff), 0.6),
            );
            poly(
                &[
                    hand + vec2(-2.1, -3.0),
                    hand + vec2(2.1, -3.0),
                    hand + vec2(1.8, 0.5),
                    hand + vec2(-1.8, 0.5),
                ],
                alpha(Color::from_hex(0x7cc4f2), 0.85),
            );
        }
        PastimeIcon::Phone => {
            let p = hand + vec2(0.0, -4.0);
            rrect(p.x - 2.4, p.y - 3.6, 4.8, 7.2, 1.0, c::GRAPHITE);
            rrect(p.x - 1.7, p.y - 2.9, 3.4, 5.6, 0.5, Color::from_hex(0x8fd3ff));
            glow(p.x, p.y - 2.0, 9.0, alpha(Color::from_hex(0x8fd3ff), 0.22));
        }
        PastimeIcon::Broom => {
            let sway = (t * 3.0).sin() * 4.0;
            let top = hand + vec2(2.0, -12.0);
            let bot = vec2(feet.x + side * 10.0 + sway + 6.0, feet.y - 2.0);
            draw_line(top.x, top.y, bot.x, bot.y, 1.6, c::WOOD_LIGHT);
            poly(
                &[
                    bot + vec2(-4.0, -1.0),
                    bot + vec2(4.0, -1.0),
                    bot + vec2(5.5, 3.0),
                    bot + vec2(-5.5, 3.0),
                ],
                c::AMBER_DEEP,
            );
        }
        PastimeIcon::Napkin | PastimeIcon::Sugar => {
            rrect_v(hand.x - 3.0, hand.y - 3.0, 6.0, 4.5, 1.2, WHITE, shade(WHITE, 0.8));
        }
        _ => {}
    }
}
