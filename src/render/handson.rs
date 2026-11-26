//! Hands-on feedback in the world (`design/handson.md` §1–§3): tip coins on tables,
//! wish bubbles with taste icons, order marks, Karolina's action-queue chips, cup glyphs
//! per recipe family (with latte art) and served cups on tables.
//!
//! All of it is stateless — positions and start times come from the caller (sim / view),
//! the picture is a function of `now`. Draw these **after** `Lighting::apply` (like the
//! emotes), so they stay readable at night; `tip_coin_light` is optional extra glint.

use std::f32::consts::{PI, TAU};

use macroquad::prelude::*;

use super::colors as c;
use super::draw::*;
use super::light::Light;
use super::moments::{MomentMarkerKind, draw_moment_icon};
use super::outside::world_text;
use super::person::draw_steam;
use super::season::star;
use super::theme::snowflake;
use crate::map::TILE;

// ───────────────────────── taste icons ─────────────────────────

/// The 12 taste tags (`Taste` in content, §3.1), each with a 12 px icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TasteIcon {
    Warm,
    Cold,
    Sweet,
    Bitter,
    Strong,
    Mild,
    Comforting,
    Fancy,
    Quick,
    Filling,
    Spiced,
    Fresh,
}

impl TasteIcon {
    pub const ALL: [TasteIcon; 12] = [
        TasteIcon::Warm,
        TasteIcon::Cold,
        TasteIcon::Sweet,
        TasteIcon::Bitter,
        TasteIcon::Strong,
        TasteIcon::Mild,
        TasteIcon::Comforting,
        TasteIcon::Fancy,
        TasteIcon::Quick,
        TasteIcon::Filling,
        TasteIcon::Spiced,
        TasteIcon::Fresh,
    ];

    /// From the content name (`"Warm"`, `"warm"` …).
    pub fn from_name(name: &str) -> Option<TasteIcon> {
        let n = name.to_ascii_lowercase();
        TasteIcon::ALL
            .iter()
            .copied()
            .find(|t| format!("{t:?}").to_ascii_lowercase() == n)
    }
}

/// A taste icon centred at `c0`, `size` px tall (12 = the design size). Any camera.
pub fn draw_taste_icon(c0: Vec2, size: f32, taste: TasteIcon, t: f32) {
    let u = size / 12.0;
    match taste {
        TasteIcon::Warm => {
            // three rising wisps over a small ember
            for i in 0..3 {
                let x = c0.x + (i as f32 - 1.0) * 3.2 * u;
                let mut prev = vec2(x, c0.y + 4.5 * u);
                for j in 1..=5 {
                    let k = j as f32 / 5.0;
                    let p = vec2(
                        x + ((k * 5.0 + t * 3.0 + i as f32).sin()) * 1.2 * u,
                        c0.y + 4.5 * u - k * 9.5 * u,
                    );
                    draw_line(
                        prev.x,
                        prev.y,
                        p.x,
                        p.y,
                        1.4 * u,
                        alpha(c::AMBER_DEEP, 1.0 - k * 0.5),
                    );
                    prev = p;
                }
            }
        }
        TasteIcon::Cold => {
            snowflake(c0, 5.2 * u, Color::from_hex(0x7fb6e8), 1.3 * u);
        }
        TasteIcon::Sweet => {
            // a wrapped sweet
            let pink = Color::from_hex(0xe87aa0);
            ellipse_v(c0.x, c0.y, 3.6 * u, 3.0 * u, tint_light(pink, 0.3), pink);
            for s in [-1.0_f32, 1.0] {
                poly(
                    &[
                        c0 + vec2(s * 3.0 * u, 0.0),
                        c0 + vec2(s * 6.0 * u, -2.8 * u),
                        c0 + vec2(s * 6.0 * u, 2.8 * u),
                    ],
                    shade(pink, 0.85),
                );
            }
            draw_line(
                c0.x - 1.5 * u,
                c0.y - 2.5 * u,
                c0.x + 1.0 * u,
                c0.y + 2.5 * u,
                0.9 * u,
                alpha(WHITE, 0.7),
            );
        }
        TasteIcon::Bitter => {
            // a coffee bean
            let bean = Color::from_hex(0x6b3e26);
            let mut pts = [Vec2::ZERO; 14];
            for (i, p) in pts.iter_mut().enumerate() {
                let a = i as f32 / 14.0 * TAU;
                let r = vec2(a.cos() * 3.6, a.sin() * 5.0);
                let rot = 0.5_f32;
                *p = c0
                    + vec2(
                        r.x * rot.cos() - r.y * rot.sin(),
                        r.x * rot.sin() + r.y * rot.cos(),
                    ) * u;
            }
            poly_v(&pts, tint_light(bean, 0.2), shade(bean, 0.8));
            arc(
                c0 + vec2(1.4, 0.0) * u,
                1.8 * u,
                4.2 * u,
                PI * 0.6,
                PI * 1.4,
                1.0 * u,
                shade(bean, 0.5),
            );
        }
        TasteIcon::Strong => {
            let col = Color::from_hex(0xf2a33a);
            poly(
                &[
                    c0 + vec2(1.5, -6.0) * u,
                    c0 + vec2(-3.5, 1.0) * u,
                    c0 + vec2(0.0, 1.0) * u,
                ],
                col,
            );
            poly(
                &[
                    c0 + vec2(0.0, -0.5) * u,
                    c0 + vec2(3.5, -0.5) * u,
                    c0 + vec2(-1.5, 6.0) * u,
                ],
                col,
            );
        }
        TasteIcon::Mild => {
            // a soft cloud
            let col = Color::from_hex(0xb9b2e8);
            for (dx, dy, r) in [(-3.0, 1.0, 2.8), (0.0, -1.2, 3.6), (3.2, 0.8, 2.8)] {
                ellipse(c0.x + dx * u, c0.y + dy * u, r * u, r * u, col);
            }
            rrect(
                c0.x - 5.8 * u,
                c0.y + 0.5 * u,
                11.6 * u,
                3.2 * u,
                1.6 * u,
                col,
            );
            ellipse(
                c0.x - 1.0 * u,
                c0.y - 2.4 * u,
                1.4 * u,
                0.9 * u,
                alpha(WHITE, 0.7),
            );
        }
        TasteIcon::Comforting => {
            // a heart in a small warm halo
            glow(c0.x, c0.y, 7.0 * u, alpha(c::AMBER, 0.35));
            heart(c0, 4.4 * u, Color::from_hex(0xe8707a));
        }
        TasteIcon::Fancy => {
            let col = c::VIOLET_SOFT;
            sparkle(c0, 5.8 * u, col);
            sparkle(c0 + vec2(4.2, -3.8) * u, 2.0 * u, tint_light(col, 0.4));
        }
        TasteIcon::Quick => {
            // a stopwatch
            let col = Color::from_hex(0x5b6f9a);
            ellipse(c0.x, c0.y + 0.8 * u, 4.6 * u, 4.6 * u, col);
            ellipse(
                c0.x,
                c0.y + 0.8 * u,
                3.4 * u,
                3.4 * u,
                Color::from_hex(0xf4f1ff),
            );
            rrect(
                c0.x - 1.2 * u,
                c0.y - 5.6 * u,
                2.4 * u,
                1.8 * u,
                0.6 * u,
                col,
            );
            let a = -PI / 2.0 + t * 2.0;
            draw_line(
                c0.x,
                c0.y + 0.8 * u,
                c0.x + a.cos() * 2.6 * u,
                c0.y + 0.8 * u + a.sin() * 2.6 * u,
                1.0 * u,
                c::INDIGO_600,
            );
        }
        TasteIcon::Filling => {
            // a bowl with steam
            let bowl = Color::from_hex(0xd98a5a);
            poly_v(
                &[
                    c0 + vec2(-5.6, -0.5) * u,
                    c0 + vec2(5.6, -0.5) * u,
                    c0 + vec2(3.0, 4.5) * u,
                    c0 + vec2(-3.0, 4.5) * u,
                ],
                tint_light(bowl, 0.15),
                shade(bowl, 0.75),
            );
            ellipse(
                c0.x,
                c0.y - 0.5 * u,
                5.6 * u,
                1.6 * u,
                Color::from_hex(0xf0c060),
            );
            for i in 0..2 {
                let x = c0.x + (i as f32 * 3.0 - 1.5) * u;
                arc(
                    vec2(x, c0.y - 3.6 * u),
                    1.0 * u,
                    1.6 * u,
                    -PI * 0.5,
                    PI * 0.5,
                    0.9 * u,
                    alpha(c::INDIGO_SOFT, 0.9),
                );
            }
        }
        TasteIcon::Spiced => {
            // a star anise and a cinnamon stick
            let brown = Color::from_hex(0x9a5a34);
            capsule(
                c0 + vec2(-5.0, 4.0) * u,
                c0 + vec2(3.0, -4.0) * u,
                1.3 * u,
                shade(brown, 0.9),
            );
            for i in 0..8 {
                let a = i as f32 / 8.0 * TAU;
                let p = c0 + vec2(2.0, 1.5) * u + vec2(a.cos(), a.sin()) * 2.6 * u;
                ellipse(p.x, p.y, 1.3 * u, 1.3 * u, brown);
            }
            ellipse(
                c0.x + 2.0 * u,
                c0.y + 1.5 * u,
                1.0 * u,
                1.0 * u,
                Color::from_hex(0xe0a060),
            );
        }
        TasteIcon::Fresh => {
            // a mint leaf and a lemon slice
            let lemon = Color::from_hex(0xf2d34a);
            ellipse(c0.x + 2.0 * u, c0.y + 1.5 * u, 3.6 * u, 3.6 * u, lemon);
            ellipse(
                c0.x + 2.0 * u,
                c0.y + 1.5 * u,
                2.6 * u,
                2.6 * u,
                tint_light(lemon, 0.5),
            );
            for i in 0..4 {
                let a = i as f32 / 4.0 * PI;
                let d = vec2(a.cos(), a.sin()) * 2.6 * u;
                let cc = c0 + vec2(2.0, 1.5) * u;
                draw_line(
                    cc.x - d.x,
                    cc.y - d.y,
                    cc.x + d.x,
                    cc.y + d.y,
                    0.6 * u,
                    lemon,
                );
            }
            super::theme::leaf_shape(
                c0 + vec2(-2.4, -2.0) * u,
                -0.9,
                8.0 * u,
                Color::from_hex(0x4fae74),
            );
        }
    }
}

/// A heart (`r` ≈ the half-width).
pub fn heart(c0: Vec2, r: f32, col: Color) {
    let v = r / 5.9;
    ellipse(c0.x - 2.7 * v, c0.y - 1.5 * v, 3.4 * v, 3.4 * v, col);
    ellipse(c0.x + 2.7 * v, c0.y - 1.5 * v, 3.4 * v, 3.4 * v, col);
    poly(
        &[
            c0 + vec2(-5.9 * v, -0.4 * v),
            c0 + vec2(5.9 * v, -0.4 * v),
            c0 + vec2(0.0, 6.0 * v),
        ],
        col,
    );
    ellipse(
        c0.x - 3.4 * v,
        c0.y - 2.6 * v,
        1.2 * v,
        0.9 * v,
        alpha(WHITE, 0.55),
    );
}

/// A four-pointed sparkle.
pub fn sparkle(c0: Vec2, r: f32, col: Color) {
    let w = r * 0.28;
    poly(
        &[
            c0 + vec2(0.0, -r),
            c0 + vec2(w, -w),
            c0 + vec2(r, 0.0),
            c0 + vec2(w, w),
            c0 + vec2(0.0, r),
            c0 + vec2(-w, w),
            c0 + vec2(-r, 0.0),
            c0 + vec2(-w, -w),
        ],
        col,
    );
}

// ───────────────────────── cup glyphs ─────────────────────────

/// A recipe family's cup (`CupGlyph` per recipe in content; food families included).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CupGlyph {
    /// Black coffee, turek, night train: a demitasse on a saucer.
    #[default]
    Espresso,
    /// Cappuccino, flat white, moonlight latte: a wide cup with latte art.
    Latte,
    /// Lemon tea, chai, linden: a glass mug with a tag and a lemon slice.
    Tea,
    /// Hot chocolate, Lori's encore: a mug with cream.
    Chocolate,
    /// Svařák: a glass with a handle, an orange slice and a cinnamon stick.
    MulledWine,
    /// Recipe-book specials: an indigo-glazed cup with a sparkle.
    Special,
    /// Česnečka, kulajda, borscht: a soup bowl.
    Soup,
    /// Koláč, medovník, buchty: a pastry on a plate.
    Pastry,
    /// Topinka, chlebíček, pirohy: a savoury plate.
    Plate,
}

impl CupGlyph {
    pub const ALL: [CupGlyph; 9] = [
        CupGlyph::Espresso,
        CupGlyph::Latte,
        CupGlyph::Tea,
        CupGlyph::Chocolate,
        CupGlyph::MulledWine,
        CupGlyph::Special,
        CupGlyph::Soup,
        CupGlyph::Pastry,
        CupGlyph::Plate,
    ];

    pub fn from_id(id: &str) -> Option<CupGlyph> {
        Some(match id {
            "espresso" | "coffee" => CupGlyph::Espresso,
            "latte" | "milk" => CupGlyph::Latte,
            "tea" => CupGlyph::Tea,
            "chocolate" => CupGlyph::Chocolate,
            "mulled_wine" | "svarak" => CupGlyph::MulledWine,
            "special" => CupGlyph::Special,
            "soup" => CupGlyph::Soup,
            "pastry" | "sweet" => CupGlyph::Pastry,
            "plate" | "food" => CupGlyph::Plate,
            _ => return None,
        })
    }

    /// Hot drinks steam when served.
    pub fn steams(self) -> bool {
        !matches!(self, CupGlyph::Pastry | CupGlyph::Plate)
    }
}

/// Latte-art patterns (`LatteArt` shapes of the mini-game, §2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LatteArt {
    Heart,
    Tulip,
    Rosetta,
    Snowflake,
    Moon,
    Star,
}

impl LatteArt {
    pub const ALL: [LatteArt; 6] = [
        LatteArt::Heart,
        LatteArt::Tulip,
        LatteArt::Rosetta,
        LatteArt::Snowflake,
        LatteArt::Moon,
        LatteArt::Star,
    ];

    pub fn from_id(id: &str) -> Option<LatteArt> {
        LatteArt::ALL
            .iter()
            .copied()
            .find(|a| format!("{a:?}").eq_ignore_ascii_case(id))
    }
}

/// Latte art on a coffee surface ellipse (centre `c0`, radii `rx`, `ry`).
pub fn draw_latte_art(c0: Vec2, rx: f32, ry: f32, art: LatteArt) {
    let milk = Color::from_hex(0xf6ead6);
    let s = vec2(rx, ry);
    let p = |x: f32, y: f32| c0 + vec2(x * s.x, y * s.y);
    match art {
        LatteArt::Heart => {
            let v = rx / 7.0;
            ellipse(
                c0.x - 2.2 * v,
                c0.y - 0.8 * ry / 7.0 * 2.0,
                2.8 * v,
                2.6 * ry / rx * v,
                milk,
            );
            ellipse(
                c0.x + 2.2 * v,
                c0.y - 0.8 * ry / 7.0 * 2.0,
                2.8 * v,
                2.6 * ry / rx * v,
                milk,
            );
            poly(&[p(-0.68, -0.15), p(0.68, -0.15), p(0.0, 0.75)], milk);
        }
        LatteArt::Tulip => {
            for (k, y) in [(1.0_f32, 0.35_f32), (0.8, -0.05), (0.6, -0.42)] {
                ellipse(c0.x, c0.y + y * ry, 0.5 * k * rx, 0.28 * ry, milk);
            }
            draw_line(
                p(0.0, -0.55).x,
                p(0.0, -0.55).y,
                p(0.0, 0.75).x,
                p(0.0, 0.75).y,
                rx * 0.08,
                milk,
            );
        }
        LatteArt::Rosetta => {
            for i in 0..5 {
                let y = -0.6 + i as f32 * 0.3;
                let w = 0.25 + i as f32 * 0.1;
                arc(
                    p(0.0, y),
                    w * rx,
                    0.2 * ry,
                    0.15,
                    PI - 0.15,
                    rx * 0.09,
                    milk,
                );
            }
            draw_line(
                p(0.0, -0.7).x,
                p(0.0, -0.7).y,
                p(0.0, 0.75).x,
                p(0.0, 0.75).y,
                rx * 0.07,
                milk,
            );
        }
        LatteArt::Snowflake => {
            snowflake(c0, rx * 0.62, milk, rx * 0.09);
        }
        LatteArt::Moon => {
            ellipse(c0.x - rx * 0.1, c0.y, rx * 0.5, ry * 0.55, milk);
            ellipse(
                c0.x + rx * 0.12,
                c0.y - ry * 0.1,
                rx * 0.42,
                ry * 0.48,
                Color::from_hex(0x9a6a4a),
            );
            sparkle(p(0.5, -0.45), rx * 0.16, milk);
        }
        LatteArt::Star => {
            let mut pts = [Vec2::ZERO; 10];
            for (i, q) in pts.iter_mut().enumerate() {
                let a = -PI / 2.0 + i as f32 * PI / 5.0;
                let r = if i % 2 == 0 { 0.62 } else { 0.27 };
                *q = c0 + vec2(a.cos() * r * rx, a.sin() * r * ry);
            }
            fan_pub(c0, &pts, c0.y - ry, c0.y + ry, milk, milk);
        }
    }
}

/// A recipe-family cup icon centred at `c0`, `size` px tall (16 = the design size).
/// `art` is shown on Latte / Special surfaces. For the recipe book, bubbles, queue chips.
pub fn draw_cup_glyph(c0: Vec2, size: f32, glyph: CupGlyph, art: Option<LatteArt>) {
    let u = size / 16.0;
    let cream = c::CREAM;
    match glyph {
        CupGlyph::Espresso => {
            ellipse(c0.x, c0.y + 5.5 * u, 7.5 * u, 2.2 * u, shade(cream, 0.85));
            ellipse(c0.x, c0.y + 5.0 * u, 6.5 * u, 1.6 * u, cream);
            draw_circle_lines(
                c0.x + 5.0 * u,
                c0.y + 0.5 * u,
                1.9 * u,
                1.1 * u,
                shade(cream, 0.8),
            );
            poly_v(
                &[
                    c0 + vec2(-4.5, -2.5) * u,
                    c0 + vec2(4.5, -2.5) * u,
                    c0 + vec2(3.0, 4.8) * u,
                    c0 + vec2(-3.0, 4.8) * u,
                ],
                cream,
                shade(cream, 0.78),
            );
            ellipse(c0.x, c0.y - 2.5 * u, 4.5 * u, 1.3 * u, shade(cream, 0.9));
            ellipse(
                c0.x,
                c0.y - 2.4 * u,
                3.6 * u,
                0.9 * u,
                Color::from_hex(0xa8683a),
            );
            ellipse(
                c0.x - 1.0 * u,
                c0.y - 2.6 * u,
                1.2 * u,
                0.4 * u,
                alpha(Color::from_hex(0xe0a870), 0.8),
            );
        }
        CupGlyph::Latte | CupGlyph::Special => {
            let body = if glyph == CupGlyph::Special {
                c::INDIGO
            } else {
                cream
            };
            ellipse(c0.x, c0.y + 6.0 * u, 8.0 * u, 2.0 * u, shade(cream, 0.85));
            draw_circle_lines(c0.x + 6.6 * u, c0.y, 2.4 * u, 1.3 * u, shade(body, 0.85));
            poly_v(
                &[
                    c0 + vec2(-7.0, -4.0) * u,
                    c0 + vec2(7.0, -4.0) * u,
                    c0 + vec2(4.5, 5.5) * u,
                    c0 + vec2(-4.5, 5.5) * u,
                ],
                tint_light(body, 0.1),
                shade(body, 0.75),
            );
            ellipse(c0.x, c0.y - 4.0 * u, 7.0 * u, 2.6 * u, shade(body, 0.92));
            ellipse(
                c0.x,
                c0.y - 3.9 * u,
                6.0 * u,
                2.0 * u,
                Color::from_hex(0x9a6a4a),
            );
            if let Some(a) = art {
                draw_latte_art(c0 - vec2(0.0, 3.9 * u), 6.0 * u, 2.0 * u, a);
            } else if glyph == CupGlyph::Latte {
                draw_latte_art(c0 - vec2(0.0, 3.9 * u), 6.0 * u, 2.0 * u, LatteArt::Heart);
            }
            if glyph == CupGlyph::Special {
                draw_line(
                    c0.x - 6.0 * u,
                    c0.y + 1.0 * u,
                    c0.x + 6.0 * u,
                    c0.y + 1.0 * u,
                    1.0 * u,
                    c::INDIGO_SOFT,
                );
                sparkle(c0 + vec2(5.5, -8.0) * u, 2.8 * u, c::AMBER);
                sparkle(c0 + vec2(-6.0, -7.0) * u, 1.6 * u, c::VIOLET_SOFT);
            }
        }
        CupGlyph::Tea => {
            let glass = Color::new(0.85, 0.88, 1.0, 0.45);
            let tea = Color::from_hex(0xd9822e);
            draw_circle_lines(c0.x + 5.6 * u, c0.y + 0.5 * u, 2.4 * u, 1.2 * u, glass);
            rrect_v(
                c0.x - 5.0 * u,
                c0.y - 2.5 * u,
                10.0 * u,
                9.0 * u,
                2.2 * u,
                tint_light(tea, 0.15),
                shade(tea, 0.75),
            );
            rrect(
                c0.x - 5.0 * u,
                c0.y - 6.5 * u,
                10.0 * u,
                13.0 * u,
                2.5 * u,
                glass,
            );
            ellipse(c0.x, c0.y - 2.5 * u, 5.0 * u, 1.3 * u, tint_light(tea, 0.3));
            // lemon on the rim, the tea-bag tag
            ellipse(
                c0.x - 4.0 * u,
                c0.y - 6.0 * u,
                3.0 * u,
                3.0 * u,
                Color::from_hex(0xf2d34a),
            );
            ellipse(
                c0.x - 4.0 * u,
                c0.y - 6.0 * u,
                2.1 * u,
                2.1 * u,
                Color::from_hex(0xfaeb9a),
            );
            draw_line(
                c0.x + 2.5 * u,
                c0.y - 6.0 * u,
                c0.x + 6.0 * u,
                c0.y - 2.0 * u,
                0.6 * u,
                WHITE,
            );
            rrect(
                c0.x + 5.0 * u,
                c0.y - 2.5 * u,
                3.0 * u,
                3.6 * u,
                0.6 * u,
                Color::from_hex(0xf3ead8),
            );
            draw_line(
                c0.x - 3.5 * u,
                c0.y - 4.5 * u,
                c0.x - 3.5 * u,
                c0.y + 4.5 * u,
                1.0 * u,
                alpha(WHITE, 0.5),
            );
        }
        CupGlyph::Chocolate => {
            let mug = Color::from_hex(0xc96a5a);
            draw_circle_lines(
                c0.x + 6.0 * u,
                c0.y + 1.0 * u,
                2.4 * u,
                1.4 * u,
                shade(mug, 0.85),
            );
            rrect_v(
                c0.x - 5.5 * u,
                c0.y - 3.5 * u,
                11.0 * u,
                10.5 * u,
                2.5 * u,
                tint_light(mug, 0.12),
                shade(mug, 0.7),
            );
            for (dx, dy, r) in [(-2.5, -4.0, 2.8), (2.0, -4.2, 3.0), (0.0, -6.2, 2.6)] {
                ellipse(
                    c0.x + dx * u,
                    c0.y + dy * u,
                    r * u,
                    r * 0.8 * u,
                    Color::from_hex(0xfbf3e6),
                );
            }
            for i in 0..4 {
                ellipse(
                    c0.x + (i as f32 - 1.5) * 1.6 * u,
                    c0.y - 5.0 * u + (i % 2) as f32 * u,
                    0.5 * u,
                    0.5 * u,
                    Color::from_hex(0x6b3e26),
                );
            }
            draw_line(
                c0.x - 3.0 * u,
                c0.y + 2.0 * u,
                c0.x + 3.0 * u,
                c0.y + 2.0 * u,
                1.0 * u,
                alpha(c::CREAM, 0.6),
            );
        }
        CupGlyph::MulledWine => {
            let glass = Color::new(0.85, 0.88, 1.0, 0.45);
            let wine = Color::from_hex(0x9a2a3e);
            draw_circle_lines(c0.x + 5.6 * u, c0.y + 0.5 * u, 2.4 * u, 1.2 * u, glass);
            poly_v(
                &[
                    c0 + vec2(-5.0, -3.0) * u,
                    c0 + vec2(5.0, -3.0) * u,
                    c0 + vec2(3.8, 5.0) * u,
                    c0 + vec2(-3.8, 5.0) * u,
                ],
                tint_light(wine, 0.2),
                shade(wine, 0.7),
            );
            rrect(
                c0.x - 5.4 * u,
                c0.y - 6.0 * u,
                10.8 * u,
                11.5 * u,
                2.0 * u,
                glass,
            );
            ellipse(c0.x, c0.y + 6.5 * u, 4.5 * u, 1.3 * u, glass);
            // an orange slice and a cinnamon stick
            ellipse(
                c0.x + 3.0 * u,
                c0.y - 5.5 * u,
                3.0 * u,
                3.0 * u,
                Color::from_hex(0xf2913a),
            );
            ellipse(
                c0.x + 3.0 * u,
                c0.y - 5.5 * u,
                2.2 * u,
                2.2 * u,
                Color::from_hex(0xf9c070),
            );
            capsule(
                c0 + vec2(-3.0, -9.0) * u,
                c0 + vec2(-1.0, -1.0) * u,
                0.9 * u,
                Color::from_hex(0x8a4a2a),
            );
        }
        CupGlyph::Soup => {
            let bowl = Color::from_hex(0xe8dcc6);
            ellipse(c0.x, c0.y + 4.5 * u, 6.0 * u, 1.8 * u, shade(bowl, 0.7));
            poly_v(
                &[
                    c0 + vec2(-8.0, -1.5) * u,
                    c0 + vec2(8.0, -1.5) * u,
                    c0 + vec2(4.5, 5.0) * u,
                    c0 + vec2(-4.5, 5.0) * u,
                ],
                bowl,
                shade(bowl, 0.75),
            );
            ellipse(c0.x, c0.y - 1.5 * u, 8.0 * u, 2.6 * u, shade(bowl, 0.9));
            ellipse(
                c0.x,
                c0.y - 1.4 * u,
                7.0 * u,
                2.0 * u,
                Color::from_hex(0xe8b04a),
            );
            for i in 0..3 {
                ellipse(
                    c0.x + (i as f32 - 1.0) * 3.0 * u,
                    c0.y - 1.6 * u,
                    0.9 * u,
                    0.6 * u,
                    Color::from_hex(0x7fae5a),
                );
            }
            draw_line(
                c0.x + 3.0 * u,
                c0.y - 2.0 * u,
                c0.x + 8.5 * u,
                c0.y - 6.5 * u,
                1.3 * u,
                c::STEEL_LIGHT,
            );
        }
        CupGlyph::Pastry => {
            let plate = c::CREAM;
            ellipse(c0.x, c0.y + 3.5 * u, 8.5 * u, 3.0 * u, shade(plate, 0.8));
            ellipse(c0.x, c0.y + 3.0 * u, 8.0 * u, 2.6 * u, plate);
            let dough = Color::from_hex(0xd9a050);
            ellipse_v(
                c0.x,
                c0.y,
                6.0 * u,
                3.4 * u,
                tint_light(dough, 0.2),
                shade(dough, 0.8),
            );
            ellipse(
                c0.x,
                c0.y - 0.6 * u,
                3.0 * u,
                1.6 * u,
                Color::from_hex(0x7a2e4a),
            );
            ellipse(
                c0.x - 0.8 * u,
                c0.y - 1.0 * u,
                0.9 * u,
                0.4 * u,
                alpha(WHITE, 0.5),
            );
            for i in 0..5 {
                let a = i as f32 / 5.0 * TAU;
                ellipse(
                    c0.x + a.cos() * 4.6 * u,
                    c0.y + a.sin() * 2.5 * u,
                    0.6 * u,
                    0.4 * u,
                    Color::from_hex(0xfbf3e6),
                );
            }
        }
        CupGlyph::Plate => {
            let plate = c::CREAM;
            ellipse(c0.x, c0.y + 3.5 * u, 8.5 * u, 3.0 * u, shade(plate, 0.8));
            ellipse(c0.x, c0.y + 3.0 * u, 8.0 * u, 2.6 * u, plate);
            let bread = Color::from_hex(0xc98a4a);
            rrect_v(
                c0.x - 6.0 * u,
                c0.y - 2.0 * u,
                12.0 * u,
                4.5 * u,
                1.5 * u,
                tint_light(bread, 0.15),
                shade(bread, 0.8),
            );
            rrect(
                c0.x - 5.0 * u,
                c0.y - 3.2 * u,
                10.0 * u,
                2.0 * u,
                1.0 * u,
                Color::from_hex(0xf3e0b0),
            );
            for i in 0..3 {
                ellipse(
                    c0.x + (i as f32 - 1.0) * 3.2 * u,
                    c0.y - 3.4 * u,
                    1.3 * u,
                    0.9 * u,
                    if i == 1 {
                        Color::from_hex(0xd2404e)
                    } else {
                        Color::from_hex(0x7fae5a)
                    },
                );
            }
        }
    }
}

/// A served cup on a table in the world (`p` — the bottom point on the table top):
/// a little larger than `draw_cup`, so the latte-art glyph stays visible (§1.7).
pub fn draw_served_cup(p: Vec2, glyph: CupGlyph, art: Option<LatteArt>, t: f32) {
    soft_shadow(p.x + 1.0, p.y, 7.0, 2.2, 0.35);
    let size = 13.0;
    draw_cup_glyph(p - vec2(0.0, size * 0.36), size, glyph, art);
    if glyph.steams() {
        draw_steam(p - vec2(0.0, size * 0.9), t, 0.55);
    }
}

// ───────────────────────── bubbles ─────────────────────────

fn pop_scale(age: f32) -> f32 {
    let a = (age / 0.28).clamp(0.0, 1.0);
    let c1 = 1.70158;
    (1.0 + (c1 + 1.0) * (a - 1.0).powi(3) + c1 * (a - 1.0).powi(2)).max(0.0)
}

fn bubble_bg(x: f32, y: f32, w: f32, h: f32, r: f32) {
    soft_shadow(x + w / 2.0, y + h / 2.0 + 3.0, w * 0.62, h * 0.6, 0.32);
    rrect_v(
        x,
        y,
        w,
        h,
        r,
        Color::from_hex(0xf8f6ff),
        Color::from_hex(0xe3defa),
    );
    rrect_v(
        x + 1.5,
        y + 1.0,
        w - 3.0,
        h * 0.4,
        r * 0.8,
        alpha(WHITE, 0.55),
        alpha(WHITE, 0.0),
    );
}

/// A guest's wish: a thought bubble with 1–4 faint taste icons (`anchor` — `emote_anchor`).
/// `age` — seconds since it appeared (pops in), `t` — animation time.
pub fn draw_wish_bubble(anchor: Vec2, tastes: &[TasteIcon], age: f32, t: f32) {
    let k = pop_scale(age);
    if k <= 0.01 {
        return;
    }
    let n = tastes.len().clamp(1, 4) as f32;
    let bob = (t * 2.0).sin() * 1.2;
    let (w, h) = ((n * 14.0 + 8.0) * k, 20.0 * k);
    let c0 = anchor + vec2(0.0, -16.0 * k + bob);
    // the thought "tail": two small dots down to the head
    for (i, (dx, dy, r)) in [(-3.0, 13.0, 2.6), (-6.0, 19.0, 1.7)].iter().enumerate() {
        let _ = i;
        ellipse(
            c0.x + dx * k,
            c0.y + dy * k,
            r * k,
            r * k,
            Color::from_hex(0xeeeafd),
        );
    }
    bubble_bg(c0.x - w / 2.0, c0.y - h / 2.0, w, h, 9.0 * k);
    for (i, taste) in tastes.iter().take(4).enumerate() {
        let x = c0.x - w / 2.0 + (4.0 + 7.0 + i as f32 * 14.0) * k;
        draw_taste_icon(vec2(x, c0.y), 11.0 * k, *taste, t);
    }
}

/// What a guest's order mark says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderMark {
    /// Seated and waiting to order: a pulsing `!` (click to take the order).
    Waiting,
    /// Karolina is on it: a calm indigo ring with a check.
    Claimed,
    /// A trusted regular: "The usual?" — a heart over a cup.
    Usual,
    /// Patience ran out once (Cozy): a glass of water, waiting kindly.
    Water,
}

/// An order mark over a guest (`anchor` — `emote_anchor`).
pub fn draw_order_mark(anchor: Vec2, mark: OrderMark, age: f32, t: f32) {
    let k = pop_scale(age);
    if k <= 0.01 {
        return;
    }
    let bob = (t * 2.4).sin() * 1.2;
    let c0 = anchor + vec2(0.0, -12.0 * k + bob);
    let (w, h) = (20.0 * k, 19.0 * k);
    match mark {
        OrderMark::Waiting => {
            let pulse = 0.5 + 0.5 * (t * TAU / 1.2).sin();
            glow(c0.x, c0.y, 18.0 * k, alpha(c::AMBER, 0.25 + 0.2 * pulse));
        }
        OrderMark::Claimed => {
            glow(c0.x, c0.y, 15.0 * k, alpha(c::INDIGO_GLOW, 0.3));
        }
        _ => {}
    }
    poly(
        &[
            c0 + vec2(-3.5 * k, h * 0.4),
            c0 + vec2(3.5 * k, h * 0.4),
            c0 + vec2(0.0, h * 0.5 + 5.0 * k),
        ],
        Color::from_hex(0xe9e5fb),
    );
    bubble_bg(c0.x - w / 2.0, c0.y - h / 2.0, w, h, 7.0 * k);
    match mark {
        OrderMark::Waiting => {
            let col = c::AMBER_DEEP;
            capsule(
                c0 + vec2(0.0, -6.0 * k),
                c0 + vec2(0.0, 1.5 * k),
                1.9 * k,
                col,
            );
            ellipse(c0.x, c0.y + 5.5 * k, 2.0 * k, 2.0 * k, col);
        }
        OrderMark::Claimed => {
            draw_circle_lines(c0.x, c0.y, 6.0 * k, 1.4 * k, c::INDIGO);
            draw_line(
                c0.x - 3.0 * k,
                c0.y,
                c0.x - 0.8 * k,
                c0.y + 2.4 * k,
                1.6 * k,
                c::INDIGO,
            );
            draw_line(
                c0.x - 0.8 * k,
                c0.y + 2.4 * k,
                c0.x + 3.4 * k,
                c0.y - 2.6 * k,
                1.6 * k,
                c::INDIGO,
            );
        }
        OrderMark::Usual => {
            draw_cup_glyph(c0 + vec2(-1.0, 2.5) * k, 11.0 * k, CupGlyph::Espresso, None);
            let beat = 1.0 + ((t * 4.0).sin().max(0.0)).powi(4) * 0.15;
            heart(c0 + vec2(4.5, -4.5) * k, 3.2 * k * beat, c::ROSE);
        }
        OrderMark::Water => {
            let glass = Color::new(0.6, 0.7, 1.0, 0.5);
            rrect_v(
                c0.x - 3.5 * k,
                c0.y - 5.5 * k,
                7.0 * k,
                11.0 * k,
                1.5 * k,
                alpha(Color::from_hex(0x9ec4f0), 0.7),
                alpha(Color::from_hex(0x6a8ad0), 0.8),
            );
            rrect(
                c0.x - 4.0 * k,
                c0.y - 6.5 * k,
                8.0 * k,
                12.5 * k,
                2.0 * k,
                glass,
            );
            draw_line(
                c0.x - 2.0 * k,
                c0.y - 4.0 * k,
                c0.x - 2.0 * k,
                c0.y + 3.0 * k,
                0.8 * k,
                alpha(WHITE, 0.7),
            );
        }
    }
}

// ───────────────────────── action queue ─────────────────────────

/// One action in Karolina's queue (up to 5 chips over her head, §1.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueueIcon {
    TakeOrder,
    Brew(CupGlyph),
    Serve(CupGlyph),
    Clear,
    Wash,
    Mop,
    CollectTip,
    Greet,
    /// Light the window / a lamp.
    Light,
    Moment(MomentMarkerKind),
    Walk,
}

/// Chip size and gap in world pixels.
pub const QUEUE_CHIP: f32 = 16.0;
const QUEUE_GAP: f32 = 3.0;

/// World rects of `n` queue chips over `anchor` (`emote_anchor` of Karolina), left to right.
/// Use for hit-testing ("click a queued icon to remove it").
pub fn queue_chip_rects(anchor: Vec2, n: usize) -> Vec<Rect> {
    let n = n.min(5);
    let total = n as f32 * QUEUE_CHIP + (n.max(1) - 1) as f32 * QUEUE_GAP;
    let x0 = anchor.x - total / 2.0;
    let y = anchor.y - 14.0 - QUEUE_CHIP;
    (0..n)
        .map(|i| {
            Rect::new(
                x0 + i as f32 * (QUEUE_CHIP + QUEUE_GAP),
                y,
                QUEUE_CHIP,
                QUEUE_CHIP,
            )
        })
        .collect()
}

/// Karolina's queue chips. `progress` 0..1 — the current (first) action's progress ring;
/// `hovered` — index of a chip under the mouse (it lifts and shows a small ×).
pub fn draw_queue_chips(
    anchor: Vec2,
    icons: &[QueueIcon],
    progress: f32,
    hovered: Option<usize>,
    t: f32,
) {
    let rects = queue_chip_rects(anchor, icons.len());
    for (i, (r, icon)) in rects.iter().zip(icons).enumerate() {
        let lift = if hovered == Some(i) { 2.0 } else { 0.0 };
        let r = Rect::new(r.x, r.y - lift, r.w, r.h);
        let c0 = r.center();
        let first = i == 0;
        soft_shadow(c0.x, c0.y + 4.0, 11.0, 8.0, 0.3);
        // glass chip with a lit 1 px edge
        rrect_v(
            r.x - 1.0,
            r.y - 1.0,
            r.w + 2.0,
            r.h + 2.0,
            5.5,
            alpha(c::INDIGO_SOFT, if first { 0.95 } else { 0.6 }),
            alpha(c::INDIGO, if first { 0.8 } else { 0.4 }),
        );
        rrect_v(
            r.x,
            r.y,
            r.w,
            r.h,
            5.0,
            Color::from_hex(0x2c2856),
            Color::from_hex(0x1d1a3c),
        );
        rrect_v(
            r.x + 1.0,
            r.y + 1.0,
            r.w - 2.0,
            5.0,
            3.0,
            alpha(WHITE, 0.14),
            alpha(WHITE, 0.0),
        );
        draw_queue_icon(c0, 11.0, *icon, t);
        if first && progress > 0.0 {
            let a0 = -PI / 2.0;
            arc(
                c0,
                r.w * 0.5 + 2.5,
                r.h * 0.5 + 2.5,
                a0,
                a0 + TAU * progress.clamp(0.0, 1.0),
                1.8,
                c::INDIGO_GLOW,
            );
            let a = a0 + TAU * progress.clamp(0.0, 1.0);
            glow(
                c0.x + a.cos() * (r.w * 0.5 + 2.5),
                c0.y + a.sin() * (r.h * 0.5 + 2.5),
                4.0,
                alpha(c::INDIGO_SOFT, 0.9),
            );
        }
        if hovered == Some(i) {
            let x = vec2(r.x + r.w - 1.0, r.y + 1.0);
            ellipse(x.x, x.y, 3.6, 3.6, c::ROSE);
            draw_line(x.x - 1.5, x.y - 1.5, x.x + 1.5, x.y + 1.5, 1.0, WHITE);
            draw_line(x.x - 1.5, x.y + 1.5, x.x + 1.5, x.y - 1.5, 1.0, WHITE);
        }
    }
    let _ = t;
}

/// The icon of a queued action (any camera).
pub fn draw_queue_icon(c0: Vec2, size: f32, icon: QueueIcon, t: f32) {
    let u = size / 11.0;
    let ink = Color::from_hex(0xe9e6ff);
    match icon {
        QueueIcon::TakeOrder => {
            rrect(
                c0.x - 3.5 * u,
                c0.y - 4.5 * u,
                7.0 * u,
                9.0 * u,
                1.0 * u,
                Color::from_hex(0xfff3c9),
            );
            for i in 0..3 {
                let y = c0.y - 2.0 * u + i as f32 * 2.2 * u;
                draw_line(
                    c0.x - 2.2 * u,
                    y,
                    c0.x + (2.2 - i as f32 * 0.8) * u,
                    y,
                    0.7 * u,
                    c::INDIGO,
                );
            }
            draw_line(
                c0.x + 3.0 * u,
                c0.y + 4.5 * u,
                c0.x + 5.5 * u,
                c0.y - 1.0 * u,
                1.2 * u,
                c::AMBER_DEEP,
            );
        }
        QueueIcon::Brew(g) => draw_cup_glyph(c0 + vec2(0.0, 0.5 * u), 12.0 * u, g, None),
        QueueIcon::Serve(g) => {
            ellipse(c0.x, c0.y + 3.5 * u, 5.5 * u, 1.6 * u, c::STEEL_LIGHT);
            draw_cup_glyph(c0 + vec2(0.0, -0.5 * u), 9.5 * u, g, None);
        }
        QueueIcon::Clear => {
            for i in 0..3 {
                let y = c0.y + 3.0 * u - i as f32 * 2.6 * u;
                rrect_v(
                    c0.x - 3.5 * u,
                    y - 2.0 * u,
                    7.0 * u,
                    2.4 * u,
                    1.0 * u,
                    c::CREAM,
                    shade(c::CREAM, 0.75),
                );
            }
        }
        QueueIcon::Wash => {
            for (dx, dy, r) in [
                (-2.0, 1.0, 2.6),
                (2.2, -1.0, 2.0),
                (0.5, -3.6, 1.4),
                (3.0, 3.0, 1.2),
            ] {
                let w = (t * 2.0 + dx).sin() * 0.3 * u;
                draw_circle_lines(
                    c0.x + dx * u + w,
                    c0.y + dy * u,
                    r * u,
                    0.8 * u,
                    alpha(Color::from_hex(0xbfe0ff), 0.95),
                );
                ellipse(
                    c0.x + dx * u - r * 0.3 * u,
                    c0.y + dy * u - r * 0.3 * u,
                    0.5 * u,
                    0.5 * u,
                    WHITE,
                );
            }
        }
        QueueIcon::Mop => {
            draw_line(
                c0.x + 3.5 * u,
                c0.y - 5.0 * u,
                c0.x - 1.0 * u,
                c0.y + 2.5 * u,
                1.2 * u,
                c::WOOD_LIGHT,
            );
            for i in 0..5 {
                let x = c0.x - 4.5 * u + i as f32 * 1.4 * u;
                draw_line(
                    c0.x - 1.0 * u,
                    c0.y + 2.0 * u,
                    x,
                    c0.y + 5.0 * u,
                    1.0 * u,
                    ink,
                );
            }
        }
        QueueIcon::CollectTip => draw_coin(c0, 4.6 * u, 1.0, t),
        QueueIcon::Greet => {
            // a waving hand
            let skin = Color::from_hex(0xf0c6a6);
            let wave = (t * 6.0).sin() * 0.25;
            let rot = |v: Vec2| {
                vec2(
                    v.x * wave.cos() - v.y * wave.sin(),
                    v.x * wave.sin() + v.y * wave.cos(),
                )
            };
            rrect(
                c0.x - 3.0 * u,
                c0.y - 1.0 * u,
                6.0 * u,
                5.5 * u,
                2.0 * u,
                skin,
            );
            for i in 0..4 {
                let base = vec2((i as f32 - 1.5) * 1.5, -1.0) * u;
                let tip = base + rot(vec2(0.0, -4.0 - (i == 1 || i == 2) as i32 as f32)) * u;
                capsule(c0 + base, c0 + tip, 0.7 * u, skin);
            }
            capsule(
                c0 + vec2(3.0, 1.5) * u,
                c0 + vec2(5.0, -1.0) * u,
                0.7 * u,
                skin,
            );
        }
        QueueIcon::Light => {
            glow(c0.x, c0.y, 7.0 * u, alpha(c::AMBER, 0.5));
            ellipse_v(
                c0.x,
                c0.y - 0.5 * u,
                3.4 * u,
                3.8 * u,
                Color::from_hex(0xfff0c0),
                c::AMBER,
            );
            rrect(
                c0.x - 1.8 * u,
                c0.y + 3.0 * u,
                3.6 * u,
                2.4 * u,
                0.6 * u,
                c::STEEL_LIGHT,
            );
        }
        QueueIcon::Moment(k) => draw_moment_icon(c0, size, k, t),
        QueueIcon::Walk => {
            for (dx, dy) in [(-2.0_f32, 2.0_f32), (2.0, -2.0)] {
                ellipse(c0.x + dx * u, c0.y + dy * u, 1.6 * u, 2.6 * u, ink);
                ellipse(
                    c0.x + dx * u,
                    c0.y + dy * u - 3.4 * u,
                    1.0 * u,
                    0.9 * u,
                    ink,
                );
            }
        }
    }
}

// ───────────────────────── tip coins ─────────────────────────

/// How long a collected coin flies to the jar (s).
pub const TIP_COLLECT_SECS: f32 = 0.7;
/// How long a new coin drops and settles (s).
pub const TIP_SPAWN_SECS: f32 = 0.55;

/// A tip coin left on a table (`TipLeft { table, amount }` → a coin, click → collect).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TipCoin {
    /// Where it lies on the table top (world px).
    pub pos: Vec2,
    /// When it appeared (same clock as `now`).
    pub spawned: f32,
    /// When Karolina collected it (it then arcs to `jar`).
    pub collected: Option<f32>,
    /// The tip jar on the counter (world px of its mouth).
    pub jar: Vec2,
    pub seed: u32,
}

impl TipCoin {
    pub fn new(pos: Vec2, spawned: f32, jar: Vec2, seed: u32) -> TipCoin {
        TipCoin {
            pos,
            spawned,
            collected: None,
            jar,
            seed,
        }
    }

    /// Still on screen at `now` (false once it has landed in the jar).
    pub fn alive(&self, now: f32) -> bool {
        self.collected
            .is_none_or(|c0| now - c0 < TIP_COLLECT_SECS + 0.35)
    }

    /// The coin's current centre (for hit tests and the glint light).
    pub fn position(&self, now: f32) -> Vec2 {
        match self.collected {
            Some(c0) => {
                let k = ((now - c0) / TIP_COLLECT_SECS).clamp(0.0, 1.0);
                let e = 1.0 - (1.0 - k).powi(2);
                let p = self.pos.lerp(self.jar, e);
                let lift = (self.pos.distance(self.jar) * 0.35).clamp(30.0, 90.0);
                p - vec2(0.0, (k * PI).sin() * lift)
            }
            None => {
                let a = ((now - self.spawned) / TIP_SPAWN_SECS).clamp(0.0, 1.0);
                // drop from a little above with one small bounce
                let h = if a < 0.6 {
                    let k = a / 0.6;
                    (1.0 - k * k) * 18.0
                } else {
                    let k = (a - 0.6) / 0.4;
                    (k * PI).sin() * 4.0
                };
                self.pos - vec2(0.0, h + 3.0)
            }
        }
    }
}

/// A gold coin seen from the side-top; `spin` 0..1 squashes it (turning).
fn draw_coin(c0: Vec2, r: f32, spin: f32, _t: f32) {
    let gold = Color::from_hex(0xf2b84b);
    let w = r * spin.clamp(0.15, 1.0);
    ellipse(c0.x, c0.y + r * 0.25, w, r * 0.9, shade(gold, 0.7));
    ellipse_v(
        c0.x,
        c0.y,
        w,
        r * 0.9,
        tint_light(gold, 0.25),
        shade(gold, 0.85),
    );
    if spin > 0.5 {
        draw_ellipse_lines(c0.x, c0.y, w * 0.68, r * 0.6, 0.0, 0.8, shade(gold, 0.65));
        ellipse(
            c0.x - w * 0.35,
            c0.y - r * 0.4,
            w * 0.25,
            r * 0.16,
            alpha(WHITE, 0.75),
        );
    }
}

/// Draw a tip coin at `now`: drop → glint on the table → collect arc into the jar.
pub fn draw_tip_coin(coin: &TipCoin, now: f32) {
    if !coin.alive(now) {
        return;
    }
    let p = coin.position(now);
    match coin.collected {
        None => {
            let age = now - coin.spawned;
            let a = (age / TIP_SPAWN_SECS).clamp(0.0, 1.0);
            let spin = if a < 1.0 {
                ((age * 14.0).cos()).abs()
            } else {
                1.0
            };
            soft_shadow(coin.pos.x, coin.pos.y, 6.0 * (0.6 + 0.4 * a), 2.2, 0.35);
            // a soft "come and get me" halo
            let breathe = 0.5 + 0.5 * (now * 2.2 + coin.seed as f32).sin();
            glow(p.x, p.y, 11.0, alpha(c::AMBER, 0.18 + 0.12 * breathe));
            draw_coin(p, 4.6, spin, now);
            // a glint every ~2.4 s
            let period = 2.4;
            let g = ((now + hash01(coin.seed) * period) % period) / period;
            if a >= 1.0 && g < 0.18 {
                let k = (g / 0.18 * PI).sin();
                sparkle(p + vec2(2.2, -2.5), 4.5 * k, alpha(WHITE, 0.95));
            }
        }
        Some(c0) => {
            let k = ((now - c0) / TIP_COLLECT_SECS).clamp(0.0, 1.0);
            if k < 1.0 {
                // a short trail of sparkles behind it
                for i in 1..4 {
                    let back = coin_trail(coin, now - i as f32 * 0.05);
                    glow(back.x, back.y, 4.0, alpha(c::AMBER, 0.35 / i as f32));
                }
                let spin = ((now - c0) * 18.0).cos().abs();
                draw_coin(p, 4.6 * (1.0 - k * 0.3), spin, now);
            } else {
                // landed: a soft ring at the jar's mouth
                let r = ((now - c0 - TIP_COLLECT_SECS) / 0.35).clamp(0.0, 1.0);
                draw_ellipse_lines(
                    coin.jar.x,
                    coin.jar.y,
                    4.0 + r * 10.0,
                    1.6 + r * 3.0,
                    0.0,
                    1.2,
                    alpha(c::AMBER, 0.8 * (1.0 - r)),
                );
            }
        }
    }
}

fn coin_trail(coin: &TipCoin, at: f32) -> Vec2 {
    coin.position(at)
}

/// The glint as a light (optional, for the light map).
pub fn tip_coin_light(coin: &TipCoin, now: f32) -> Option<Light> {
    if !coin.alive(now) || coin.collected.is_some() {
        return None;
    }
    Some(Light::new(coin.position(now), 0.45 * TILE, c::AMBER, 0.35))
}

/// A floating amount ("+38 Kč", text from i18n) rising from `p`. `age` 0..~1.2 s.
pub fn draw_tip_float(p: Vec2, label: &str, age: f32) {
    if !(0.0..1.2).contains(&age) {
        return;
    }
    let k = age / 1.2;
    let fade = (1.0 - k).powf(0.7) * (age / 0.12).min(1.0);
    let y = p.y - 8.0 - 22.0 * (1.0 - (1.0 - k).powi(2));
    world_text(
        label,
        vec2(p.x + 0.8, y + 0.8),
        11.0,
        alpha(Color::from_hex(0x3a2410), 0.5 * fade),
        true,
    );
    world_text(
        label,
        vec2(p.x, y),
        11.0,
        alpha(Color::from_hex(0xffd88a), fade),
        true,
    );
}

/// The rhythm glow badge (a little amber flame + `×n`) for rush streaks, drawn next to the jar.
pub fn draw_rhythm_glow(p: Vec2, steps: u32, t: f32) {
    if steps == 0 {
        return;
    }
    let k = (steps as f32 / 5.0).min(1.0);
    let flick = 0.85 + 0.15 * noise1(t * 6.0, 3);
    glow(p.x, p.y, 10.0 + 8.0 * k, alpha(c::AMBER, 0.45 * flick));
    ellipse_v(
        p.x,
        p.y - 1.0,
        2.6 + k,
        (4.0 + 2.0 * k) * flick,
        Color::from_hex(0xfff0c0),
        c::AMBER_DEEP,
    );
    star(
        p + vec2(0.0, -9.0 - 3.0 * k),
        2.0,
        0.9,
        t,
        alpha(WHITE, 0.6 * k),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coin_flies_into_the_jar_and_disappears() {
        let mut c0 = TipCoin::new(vec2(100.0, 200.0), 0.0, vec2(300.0, 150.0), 7);
        assert!(c0.alive(10.0));
        // settled on the table after the drop
        assert!(c0.position(5.0).distance(c0.pos) < 4.0);
        c0.collected = Some(5.0);
        let mid = c0.position(5.0 + TIP_COLLECT_SECS / 2.0);
        assert!(mid.y < c0.pos.y.min(c0.jar.y), "arcs upwards");
        assert!(c0.position(5.0 + TIP_COLLECT_SECS).distance(c0.jar) < 0.01);
        assert!(!c0.alive(7.0));
        assert!(tip_coin_light(&c0, 5.1).is_none());
    }

    #[test]
    fn queue_chips_are_centred_and_capped() {
        let r = queue_chip_rects(vec2(100.0, 100.0), 3);
        assert_eq!(r.len(), 3);
        let mid = (r[0].x + r[2].x + r[2].w) / 2.0;
        assert!((mid - 100.0).abs() < 0.01);
        assert!(r.iter().all(|c| c.bottom() < 100.0));
        assert_eq!(queue_chip_rects(Vec2::ZERO, 9).len(), 5);
    }

    #[test]
    fn names_parse() {
        assert_eq!(TasteIcon::from_name("spiced"), Some(TasteIcon::Spiced));
        assert_eq!(TasteIcon::from_name("Warm"), Some(TasteIcon::Warm));
        assert_eq!(CupGlyph::from_id("mulled_wine"), Some(CupGlyph::MulledWine));
        assert_eq!(LatteArt::from_id("rosetta"), Some(LatteArt::Rosetta));
        assert!(!CupGlyph::Plate.steams());
    }
}
