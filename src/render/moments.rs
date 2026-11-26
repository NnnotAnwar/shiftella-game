//! Night-moment markers (`design/handson.md` §5): a soft glowing indigo orb floating over
//! the anchor (a tile, a furniture piece, a guest) with a small icon, pulsing every 1.2 s.
//! Plus the candle-lighting animation for the blackout moment.
//!
//! ```ignore
//! // into the light map:          lighting.add_light(moment_marker_light(&m, now));
//! // after lighting (on top):     draw_moment_marker(&m, now);
//! ```

use std::f32::consts::{PI, TAU};

use macroquad::prelude::*;

use super::cat::{CatLook, CatPose, draw_cat_ex};
use super::colors as c;
use super::draw::*;
use super::handson::{heart, sparkle};
use super::light::Light;
use super::season::star;
use super::theme::snowflake;
use crate::map::TILE;

/// The pulse period of a marker (s).
pub const MARKER_PULSE: f32 = 1.2;
/// How high the orb floats over its anchor (world px).
pub const MARKER_LIFT: f32 = 46.0;

/// The icon of a moment marker (one per moment kind in content).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MomentMarkerKind {
    /// `m_station_cat`.
    Cat,
    /// `m_blackout_candles`, `m_advent_candle`.
    Candle,
    /// `m_leak_bucket`, `m_leak_empty`.
    Bucket,
    /// `m_stuffy_window`, `m_fogged_window`.
    Window,
    /// `m_busker`.
    Busker,
    /// `m_drone_wrong_address`.
    Drone,
    /// `m_lost_traveller`.
    Traveller,
    /// `m_sleeping_guest`, `m_tired_parent`.
    Blanket,
    /// `m_record_request`, `m_radio_tuning`.
    Jukebox,
    /// `m_hang_garlands` hang spots.
    Garland,
    /// `m_first_snow`.
    Snow,
    /// `m_parcel_lviv`, `m_forgotten_item`.
    Parcel,
    /// `m_coffee_for_helper`.
    Coffee,
    /// `m_dead_phone`, `m_delay_board`.
    Phone,
    /// `m_bulb_flicker`.
    Bulb,
    /// `m_umbrellas`.
    Umbrella,
    /// `m_mikulas`.
    Star,
}

impl MomentMarkerKind {
    pub const ALL: [MomentMarkerKind; 17] = [
        MomentMarkerKind::Cat,
        MomentMarkerKind::Candle,
        MomentMarkerKind::Bucket,
        MomentMarkerKind::Window,
        MomentMarkerKind::Busker,
        MomentMarkerKind::Drone,
        MomentMarkerKind::Traveller,
        MomentMarkerKind::Blanket,
        MomentMarkerKind::Jukebox,
        MomentMarkerKind::Garland,
        MomentMarkerKind::Snow,
        MomentMarkerKind::Parcel,
        MomentMarkerKind::Coffee,
        MomentMarkerKind::Phone,
        MomentMarkerKind::Bulb,
        MomentMarkerKind::Umbrella,
        MomentMarkerKind::Star,
    ];

    /// The marker for a content moment id (`m_station_cat` → `Cat`); unknown → `Star`.
    pub fn for_moment(id: &str) -> MomentMarkerKind {
        use MomentMarkerKind::*;
        match id {
            "m_station_cat" => Cat,
            "m_blackout_candles" | "m_advent_candle" => Candle,
            "m_leak_bucket" | "m_leak_empty" => Bucket,
            "m_stuffy_window" | "m_fogged_window" => Window,
            "m_busker" => Busker,
            "m_drone_wrong_address" => Drone,
            "m_lost_traveller" => Traveller,
            "m_sleeping_guest" | "m_tired_parent" => Blanket,
            "m_record_request" | "m_radio_tuning" => Jukebox,
            "m_hang_garlands" => Garland,
            "m_first_snow" => Snow,
            "m_parcel_lviv" | "m_forgotten_item" => Parcel,
            "m_coffee_for_helper" => Coffee,
            "m_dead_phone" | "m_delay_board" => Phone,
            "m_bulb_flicker" => Bulb,
            "m_umbrellas" => Umbrella,
            _ => Star,
        }
    }
}

/// A marker in the world.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MomentMarker {
    /// The anchor on the floor (world px): the marker floats `MARKER_LIFT` above it.
    pub anchor: Vec2,
    pub kind: MomentMarkerKind,
    /// When it appeared (it pops in).
    pub born: f32,
    /// Hover 0..1 (ease it in the caller for a smooth grow).
    pub hover: f32,
    /// Seconds left before it expires (the last 10 s it fades gently), `None` = no limit.
    pub expires_in: Option<f32>,
    /// How high the orb floats over the anchor (`MARKER_LIFT`; higher over people).
    pub lift: f32,
}

impl MomentMarker {
    pub fn new(anchor: Vec2, kind: MomentMarkerKind, born: f32) -> MomentMarker {
        MomentMarker {
            anchor,
            kind,
            born,
            hover: 0.0,
            expires_in: None,
            lift: MARKER_LIFT,
        }
    }

    /// The same marker floating `lift` px over its anchor (tall furniture: the jukebox,
    /// a window, the counter).
    pub fn with_lift(self, lift: f32) -> MomentMarker {
        MomentMarker { lift, ..self }
    }

    /// A marker by a person (`feet` — their feet): the orb floats beside the head (to the
    /// right), so it never covers the face or an emote bubble. `sitting` lowers it a little.
    pub fn over_person(
        feet: Vec2,
        kind: MomentMarkerKind,
        born: f32,
        sitting: bool,
    ) -> MomentMarker {
        let anchor = feet + vec2(30.0, 0.0);
        MomentMarker {
            lift: if sitting { 84.0 } else { 92.0 },
            ..MomentMarker::new(anchor, kind, born)
        }
    }

    /// The orb's centre at `now` (for hit tests: radius `marker_radius`).
    pub fn orb(&self, now: f32) -> Vec2 {
        let bob = ((now - self.born) * 1.6).sin() * 2.0;
        self.anchor - vec2(0.0, self.lift + bob)
    }

    fn appear(&self, now: f32) -> f32 {
        let a = ((now - self.born) / 0.4).clamp(0.0, 1.0);
        let c1 = 1.70158;
        (1.0 + (c1 + 1.0) * (a - 1.0).powi(3) + c1 * (a - 1.0).powi(2)).max(0.0)
    }

    fn fade(&self) -> f32 {
        self.expires_in.map_or(1.0, |s| (s / 10.0).clamp(0.25, 1.0))
    }
}

/// The clickable radius of a marker's orb (world px).
pub fn marker_radius(m: &MomentMarker) -> f32 {
    14.0 + 2.5 * m.hover
}

/// A soft indigo pool under the marker for the light map.
pub fn moment_marker_light(m: &MomentMarker, now: f32) -> Light {
    let pulse = 0.5 + 0.5 * ((now - m.born) * TAU / MARKER_PULSE).sin();
    Light::new(
        m.anchor - vec2(0.0, 10.0),
        (1.0 + 0.2 * pulse) * TILE,
        c::INDIGO_GLOW,
        (0.35 + 0.15 * pulse) * m.fade(),
    )
}

/// Draw a marker (after lighting).
pub fn draw_moment_marker(m: &MomentMarker, now: f32) {
    let k = m.appear(now) * (1.0 + 0.12 * m.hover);
    if k <= 0.01 {
        return;
    }
    let fade = m.fade();
    let age = now - m.born;
    let pulse = (age % MARKER_PULSE) / MARKER_PULSE;
    let o = m.orb(now);
    let r = 13.0 * k;

    // the floor ring: expands and fades every pulse
    for (i, d) in [0.0_f32, 0.5].iter().enumerate() {
        let p = (pulse + d) % 1.0;
        let rr = 8.0 + p * 18.0;
        draw_ellipse_lines(
            m.anchor.x,
            m.anchor.y,
            rr,
            rr * 0.38,
            0.0,
            1.4,
            alpha(
                c::INDIGO_SOFT,
                0.55 * (1.0 - p) * fade / (1.0 + i as f32 * 0.3),
            ),
        );
    }
    radial(
        m.anchor.x,
        m.anchor.y,
        16.0,
        6.0,
        alpha(c::INDIGO_GLOW, 0.35 * fade),
        alpha(c::INDIGO_GLOW, 0.0),
    );
    // a dotted tether
    let n = 5;
    for i in 1..n {
        let p = m.anchor.lerp(o + vec2(0.0, r), i as f32 / n as f32);
        ellipse(p.x, p.y, 0.9, 0.9, alpha(c::INDIGO_SOFT, 0.4 * fade));
    }
    // the orb: glow, glass body, rim, highlight
    let beat = 1.0 + 0.06 * (1.0 - (pulse * TAU).cos()) * 0.5;
    glow(
        o.x,
        o.y,
        r * 2.4,
        alpha(c::INDIGO_GLOW, (0.28 + 0.2 * m.hover) * fade),
    );
    ellipse_v(
        o.x,
        o.y,
        r * beat,
        r * beat,
        alpha(Color::from_hex(0x6d6af5), 0.95 * fade),
        alpha(Color::from_hex(0x3b2fa8), 0.95 * fade),
    );
    draw_circle_lines(
        o.x,
        o.y,
        r * beat,
        1.2 + 0.6 * m.hover,
        alpha(c::INDIGO_SOFT, (0.8 + 0.2 * m.hover) * fade),
    );
    ellipse(
        o.x - r * 0.35,
        o.y - r * 0.45,
        r * 0.38,
        r * 0.22,
        alpha(WHITE, 0.35 * fade),
    );
    draw_moment_icon(o, 16.5 * k, m.kind, now);
    // two small orbiting sparkles
    for i in 0..2 {
        let a = age * 1.3 + i as f32 * PI;
        let p = o + vec2(a.cos() * (r + 5.0), a.sin() * (r + 5.0) * 0.5);
        sparkle(p, 2.2 * k, alpha(Color::from_hex(0xfff0c8), 0.85 * fade));
    }
}

/// A moment icon centred at `c0`, `size` px (15 in the marker). Also for the UI moment card.
pub fn draw_moment_icon(c0: Vec2, size: f32, kind: MomentMarkerKind, t: f32) {
    let u = size / 15.0;
    let cream = Color::from_hex(0xfff3dc);
    let amber = c::AMBER;
    match kind {
        MomentMarkerKind::Cat => {
            let look = CatLook {
                scale: 0.38 * u,
                ..Default::default()
            };
            draw_cat_ex(c0 + vec2(0.0, 6.0 * u), &look, CatPose::Sit, t, 1.0);
        }
        MomentMarkerKind::Candle => {
            rrect_v(
                c0.x - 2.2 * u,
                c0.y - 1.0 * u,
                4.4 * u,
                7.5 * u,
                1.2 * u,
                cream,
                shade(cream, 0.8),
            );
            let fl = 0.8 + 0.2 * (t * 9.0).sin();
            glow(c0.x, c0.y - 3.5 * u, 6.0 * u, alpha(amber, 0.7));
            ellipse(
                c0.x,
                c0.y - 3.8 * u,
                1.6 * u,
                3.0 * u * fl,
                Color::from_hex(0xffd27a),
            );
            ellipse(c0.x, c0.y - 3.2 * u, 0.7 * u, 1.3 * u, WHITE);
        }
        MomentMarkerKind::Bucket => {
            poly_v(
                &[
                    c0 + vec2(-5.0, -2.0) * u,
                    c0 + vec2(5.0, -2.0) * u,
                    c0 + vec2(3.8, 5.5) * u,
                    c0 + vec2(-3.8, 5.5) * u,
                ],
                c::STEEL_LIGHT,
                c::STEEL,
            );
            ellipse(
                c0.x,
                c0.y - 2.0 * u,
                5.0 * u,
                1.6 * u,
                Color::from_hex(0x3b3f8a),
            );
            arc(
                c0 + vec2(0.0, -2.0 * u),
                5.0 * u,
                4.0 * u,
                PI,
                TAU,
                0.8 * u,
                c::STEEL_LIGHT,
            );
            let drop = ((t * 1.2) % 1.0) * 6.0;
            ellipse(
                c0.x,
                c0.y - 9.0 * u + drop * u,
                0.9 * u,
                1.3 * u,
                Color::from_hex(0xbfe0ff),
            );
        }
        MomentMarkerKind::Window => {
            rrect(
                c0.x - 5.5 * u,
                c0.y - 5.5 * u,
                11.0 * u,
                11.0 * u,
                1.5 * u,
                cream,
            );
            rect_v(
                c0.x - 4.2 * u,
                c0.y - 4.2 * u,
                8.4 * u,
                8.4 * u,
                Color::from_hex(0x2a2a6a),
                Color::from_hex(0x4a3a8a),
            );
            draw_line(c0.x, c0.y - 4.2 * u, c0.x, c0.y + 4.2 * u, 1.0 * u, cream);
            draw_line(
                c0.x - 4.2 * u,
                c0.y - 0.5 * u,
                c0.x + 4.2 * u,
                c0.y - 0.5 * u,
                1.0 * u,
                cream,
            );
            ellipse(c0.x + 2.2 * u, c0.y - 2.4 * u, 1.0 * u, 1.0 * u, amber);
        }
        MomentMarkerKind::Busker => {
            // a guitar
            let wood = Color::from_hex(0xe0a35a);
            ellipse(c0.x - 1.5 * u, c0.y + 2.5 * u, 4.0 * u, 3.6 * u, wood);
            ellipse(c0.x + 1.2 * u, c0.y - 0.5 * u, 3.0 * u, 2.8 * u, wood);
            ellipse(
                c0.x - 0.5 * u,
                c0.y + 1.5 * u,
                1.2 * u,
                1.2 * u,
                Color::from_hex(0x3a2418),
            );
            capsule(
                c0 + vec2(1.5, -1.5) * u,
                c0 + vec2(5.5, -6.5) * u,
                0.9 * u,
                Color::from_hex(0x6b4532),
            );
            let note = c0 + vec2(-5.0, -5.0) * u + vec2(0.0, (t * 3.0).sin() * u);
            ellipse(note.x, note.y, 1.4 * u, 1.0 * u, cream);
            draw_line(
                note.x + 1.2 * u,
                note.y,
                note.x + 1.2 * u,
                note.y - 3.5 * u,
                0.8 * u,
                cream,
            );
        }
        MomentMarkerKind::Drone => {
            let body = Color::from_hex(0xd7d3ea);
            rrect(
                c0.x - 3.5 * u,
                c0.y - 1.5 * u,
                7.0 * u,
                4.0 * u,
                1.6 * u,
                body,
            );
            for s in [-1.0_f32, 1.0] {
                draw_line(
                    c0.x + s * 3.0 * u,
                    c0.y - 0.5 * u,
                    c0.x + s * 6.0 * u,
                    c0.y - 3.0 * u,
                    0.8 * u,
                    body,
                );
                let spin = (t * 30.0).sin().abs();
                ellipse(
                    c0.x + s * 6.0 * u,
                    c0.y - 3.5 * u,
                    2.8 * u * spin + 0.6 * u,
                    0.6 * u,
                    alpha(body, 0.8),
                );
            }
            rrect(
                c0.x - 2.0 * u,
                c0.y + 2.5 * u,
                4.0 * u,
                3.2 * u,
                0.6 * u,
                Color::from_hex(0xc8a070),
            );
            ellipse(c0.x, c0.y, 0.9 * u, 0.9 * u, c::INDIGO_GLOW);
        }
        MomentMarkerKind::Traveller => {
            // a suitcase with a ticket
            let case = Color::from_hex(0xc96a5a);
            rrect_v(
                c0.x - 5.0 * u,
                c0.y - 2.5 * u,
                10.0 * u,
                8.0 * u,
                1.6 * u,
                tint_light(case, 0.15),
                shade(case, 0.75),
            );
            arc(
                c0 + vec2(0.0, -2.5 * u),
                2.4 * u,
                2.4 * u,
                PI,
                TAU,
                1.0 * u,
                shade(case, 0.6),
            );
            draw_line(
                c0.x - 5.0 * u,
                c0.y + 1.0 * u,
                c0.x + 5.0 * u,
                c0.y + 1.0 * u,
                0.8 * u,
                shade(case, 0.6),
            );
            rrect(
                c0.x + 1.5 * u,
                c0.y - 6.5 * u,
                5.0 * u,
                3.2 * u,
                0.6 * u,
                cream,
            );
            draw_line(
                c0.x + 2.5 * u,
                c0.y - 5.0 * u,
                c0.x + 5.5 * u,
                c0.y - 5.0 * u,
                0.6 * u,
                c::INDIGO,
            );
        }
        MomentMarkerKind::Blanket => {
            // a folded checked blanket
            let wool = Color::from_hex(0xc96a7e);
            rrect_v(
                c0.x - 6.0 * u,
                c0.y - 3.0 * u,
                12.0 * u,
                7.5 * u,
                2.0 * u,
                tint_light(wool, 0.15),
                shade(wool, 0.8),
            );
            for i in 0..3 {
                let x = c0.x - 4.0 * u + i as f32 * 4.0 * u;
                draw_line(
                    x,
                    c0.y - 3.0 * u,
                    x,
                    c0.y + 4.5 * u,
                    0.8 * u,
                    alpha(cream, 0.6),
                );
            }
            draw_line(
                c0.x - 6.0 * u,
                c0.y + 0.5 * u,
                c0.x + 6.0 * u,
                c0.y + 0.5 * u,
                0.8 * u,
                alpha(cream, 0.6),
            );
            heart(c0 + vec2(4.0, -5.5) * u, 2.4 * u, c::ROSE);
        }
        MomentMarkerKind::Jukebox => {
            // a vinyl record with a note
            ellipse(
                c0.x,
                c0.y + 1.0 * u,
                5.5 * u,
                5.5 * u,
                Color::from_hex(0x1f1d2e),
            );
            draw_circle_lines(
                c0.x,
                c0.y + 1.0 * u,
                3.8 * u,
                0.5 * u,
                alpha(c::LAVENDER_RIM, 0.5),
            );
            ellipse(
                c0.x,
                c0.y + 1.0 * u,
                1.8 * u,
                1.8 * u,
                Color::from_hex(0xe0607a),
            );
            let a = t * 2.0;
            ellipse(
                c0.x + a.cos() * 3.0 * u,
                c0.y + 1.0 * u + a.sin() * 3.0 * u,
                0.8 * u,
                0.5 * u,
                alpha(WHITE, 0.6),
            );
            let note = c0 + vec2(4.5, -4.0) * u;
            ellipse(note.x, note.y, 1.5 * u, 1.1 * u, amber);
            draw_line(
                note.x + 1.3 * u,
                note.y,
                note.x + 1.3 * u,
                note.y - 4.0 * u,
                0.9 * u,
                amber,
            );
            draw_line(
                note.x + 1.3 * u,
                note.y - 4.0 * u,
                note.x + 3.0 * u,
                note.y - 3.0 * u,
                0.9 * u,
                amber,
            );
        }
        MomentMarkerKind::Garland => {
            let mut prev = c0 + vec2(-6.0, -3.0) * u;
            for i in 1..=6 {
                let x = -6.0 + i as f32 * 2.0;
                let y = -3.0 + (x / 6.0 * PI / 2.0).cos() * 4.0;
                let p = c0 + vec2(x, y) * u;
                draw_line(prev.x, prev.y, p.x, p.y, 0.9 * u, Color::from_hex(0x3f8a5f));
                prev = p;
            }
            for (i, x) in [-4.0_f32, 0.0, 4.0].iter().enumerate() {
                let y = -3.0 + (x / 6.0 * PI / 2.0).cos() * 4.0 + 2.2;
                let col = [amber, Color::from_hex(0xf0607a), Color::from_hex(0xfff0c0)][i];
                glow(c0.x + x * u, c0.y + y * u, 3.0 * u, alpha(col, 0.6));
                ellipse(c0.x + x * u, c0.y + y * u, 1.3 * u, 1.7 * u, col);
            }
        }
        MomentMarkerKind::Snow => snowflake(c0, 6.0 * u, cream, 1.2 * u),
        MomentMarkerKind::Parcel => {
            let card = Color::from_hex(0xd9b080);
            rrect_v(
                c0.x - 5.0 * u,
                c0.y - 4.0 * u,
                10.0 * u,
                9.0 * u,
                1.2 * u,
                tint_light(card, 0.1),
                shade(card, 0.8),
            );
            draw_line(c0.x, c0.y - 4.0 * u, c0.x, c0.y + 5.0 * u, 1.4 * u, c::ROSE);
            draw_line(c0.x - 5.0 * u, c0.y, c0.x + 5.0 * u, c0.y, 1.4 * u, c::ROSE);
            ellipse(c0.x - 1.5 * u, c0.y - 5.0 * u, 1.8 * u, 1.2 * u, c::ROSE);
            ellipse(c0.x + 1.5 * u, c0.y - 5.0 * u, 1.8 * u, 1.2 * u, c::ROSE);
        }
        MomentMarkerKind::Coffee => {
            super::handson::draw_cup_glyph(
                c0 + vec2(0.0, 1.0 * u),
                13.0 * u,
                super::handson::CupGlyph::Latte,
                None,
            );
        }
        MomentMarkerKind::Phone => {
            rrect(
                c0.x - 3.5 * u,
                c0.y - 6.0 * u,
                7.0 * u,
                12.0 * u,
                1.6 * u,
                Color::from_hex(0x2c2f3e),
            );
            rrect_v(
                c0.x - 2.6 * u,
                c0.y - 4.8 * u,
                5.2 * u,
                9.0 * u,
                0.8 * u,
                c::INDIGO_SOFT,
                c::INDIGO,
            );
            // a low battery bar
            rrect(
                c0.x - 1.6 * u,
                c0.y + 1.0 * u,
                3.2 * u,
                1.6 * u,
                0.4 * u,
                c::ROSE,
            );
        }
        MomentMarkerKind::Bulb => {
            let fl = if (t * 7.0).sin() > 0.6 { 0.4 } else { 1.0 };
            glow(c0.x, c0.y - 1.0 * u, 7.0 * u, alpha(amber, 0.6 * fl));
            ellipse_v(
                c0.x,
                c0.y - 1.5 * u,
                4.0 * u,
                4.4 * u,
                alpha(Color::from_hex(0xfff0c0), fl),
                amber,
            );
            rrect(
                c0.x - 2.0 * u,
                c0.y + 2.6 * u,
                4.0 * u,
                3.0 * u,
                0.6 * u,
                c::STEEL_LIGHT,
            );
        }
        MomentMarkerKind::Umbrella => {
            let col = Color::from_hex(0x5b6f9a);
            arc(c0 + vec2(0.0, 0.0), 6.5 * u, 5.0 * u, PI, TAU, 2.6 * u, col);
            poly(
                &[
                    c0 + vec2(-6.5, 0.0) * u,
                    c0 + vec2(0.0, -5.0) * u,
                    c0 + vec2(6.5, 0.0) * u,
                ],
                col,
            );
            draw_line(c0.x, c0.y - 5.0 * u, c0.x, c0.y + 5.0 * u, 0.9 * u, cream);
            arc(
                c0 + vec2(-1.2, 5.0) * u,
                1.2 * u,
                1.2 * u,
                0.0,
                PI,
                0.9 * u,
                cream,
            );
        }
        MomentMarkerKind::Star => {
            star(c0, 6.0 * u, 2.6 * u, 0.0, Color::from_hex(0xf2c860));
        }
    }
}

// ───────────────────────── candles in the blackout ─────────────────────────

/// How long lighting one candle takes (s): a match flare, then the flame grows.
pub const CANDLE_LIGHT_SECS: f32 = 0.9;

/// Where a table's candle stands (world px), or `None` for other pieces.
/// `origin`/`size` as for `draw_furniture`.
pub fn table_candle_anchor(sprite: &str, origin: Vec2, size: (u8, u8)) -> Option<Vec2> {
    let (w, d) = (size.0.max(1) as f32 * TILE, size.1.max(1) as f32 * TILE);
    let cx = origin.x + w / 2.0;
    let cy = origin.y + d / 2.0;
    let h = 28.0;
    match sprite {
        "table_round_2" => Some(vec2(cx + w * 0.38 * 0.35, cy - h - d * 0.32 * 0.15)),
        "table_square_4" => Some(vec2(cx, origin.y + 8.0 + (d - 18.0) * 0.45 - h)),
        "candle" => Some(vec2(cx, cy)),
        _ => None,
    }
}

/// A candle being lit: `age` seconds since Karolina touched the match to it.
/// Draw the table with `active: false` (no candle), then this on top with the next key.
/// After `CANDLE_LIGHT_SECS` it looks exactly like a lit table candle.
pub fn draw_candle_lighting(p: Vec2, age: f32, t: f32) {
    let k = (age / CANDLE_LIGHT_SECS).clamp(0.0, 1.0);
    // the glass and the wax
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
    // the match: a bright flare that moves in and fades
    if k < 0.55 {
        let m = k / 0.55;
        let mp = p + vec2(9.0 - 7.0 * m, -14.0 + 4.0 * m);
        draw_line(
            mp.x,
            mp.y,
            mp.x + 7.0,
            mp.y + 5.0,
            1.2,
            Color::from_hex(0xd8c09a),
        );
        glow(
            mp.x,
            mp.y,
            10.0 * (1.0 - m * 0.4),
            alpha(Color::from_hex(0xffd27a), 0.85),
        );
        ellipse(mp.x, mp.y, 1.6, 2.2, Color::from_hex(0xfff0c0));
    }
    // the flame grows with a little flicker
    let g = ((k - 0.35) / 0.65).clamp(0.0, 1.0);
    if g > 0.0 {
        let e = 1.0 - (1.0 - g).powi(3);
        let fl = noise1(t * 7.0, p.x as u32) * 0.4 + 0.6;
        glow(p.x, p.y - 9.0, 9.0 * fl * e, alpha(c::AMBER, 0.6 * e));
        ellipse(
            p.x,
            p.y - 9.0,
            1.4 * e,
            2.6 * fl * e,
            Color::from_hex(0xffd27a),
        );
        ellipse(p.x, p.y - 8.4, 0.7 * e, 1.2 * e, WHITE);
        // a wisp of smoke from the match as the flame takes
        if g < 1.0 {
            glow(
                p.x + 4.0,
                p.y - 18.0 - g * 8.0,
                4.0 + g * 4.0,
                Color::new(0.85, 0.85, 1.0, 0.25 * (1.0 - g)),
            );
        }
    }
}

/// The candle's light while it is being lit (grows with the flame).
pub fn candle_lighting_light(p: Vec2, age: f32) -> Light {
    let k = ((age / CANDLE_LIGHT_SECS - 0.35) / 0.65).clamp(0.0, 1.0);
    let mut l = Light::candle(p - vec2(0.0, 8.0));
    l.intensity *= k.max(0.15);
    l.radius *= 0.5 + 0.5 * k;
    l
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moment_ids_map_to_icons() {
        assert_eq!(
            MomentMarkerKind::for_moment("m_station_cat"),
            MomentMarkerKind::Cat
        );
        assert_eq!(
            MomentMarkerKind::for_moment("m_leak_empty"),
            MomentMarkerKind::Bucket
        );
        assert_eq!(
            MomentMarkerKind::for_moment("m_new_one"),
            MomentMarkerKind::Star
        );
    }

    #[test]
    fn marker_floats_and_fades() {
        let mut m = MomentMarker::new(vec2(100.0, 300.0), MomentMarkerKind::Candle, 0.0);
        assert!(m.orb(1.0).y < 300.0 - MARKER_LIFT + 3.0);
        assert_eq!(m.fade(), 1.0);
        m.expires_in = Some(5.0);
        assert!(m.fade() < 1.0);
        let l0 = moment_marker_light(&m, 0.0).intensity;
        let l1 = moment_marker_light(&m, MARKER_PULSE * 0.25).intensity;
        assert!(l1 > l0, "pulses");
        m.hover = 1.0;
        assert!(marker_radius(&m) > 14.0);
        let p = MomentMarker::over_person(vec2(0.0, 500.0), MomentMarkerKind::Blanket, 0.0, true);
        assert!(
            p.orb(0.0).y < 500.0 - 80.0 && p.orb(0.0).x > 20.0,
            "beside the head"
        );
    }

    #[test]
    fn candles_sit_on_tables() {
        let o = vec2(64.0, 128.0);
        let a = table_candle_anchor("table_round_2", o, (1, 1)).unwrap();
        assert!(a.y < o.y + 64.0 && a.x > o.x && a.x < o.x + 64.0);
        assert!(table_candle_anchor("sofa", o, (2, 1)).is_none());
        assert!(candle_lighting_light(a, 0.0).intensity < candle_lighting_light(a, 2.0).intensity);
    }
}
