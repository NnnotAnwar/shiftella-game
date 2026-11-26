//! Shared pieces of the hands-on widgets: taste tags, duties, procedural dish glyphs,
//! wait rings, capacity dots, rotated / slanted drawing and a few tiny chips.

use macroquad::prelude::*;

use crate::ui::draw::{
    self, Paint, arc, fill_circle, fill_convex, fill_rrect, line, mix, stroke_circle, stroke_path,
    stroke_rrect, with_a,
};
use crate::ui::layout::RectExt;
use crate::ui::{Align, Icon, TextStyle, Theme, Ui, Weight, draw_icon};

// ---------------------------------------------------------------------------
// Taste tags
// ---------------------------------------------------------------------------

/// The 12 taste tags of a wish or recipe (mirrors the content's `Taste`, design §3.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TasteTag {
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

impl TasteTag {
    pub const ALL: [TasteTag; 12] = [
        TasteTag::Warm,
        TasteTag::Cold,
        TasteTag::Sweet,
        TasteTag::Bitter,
        TasteTag::Strong,
        TasteTag::Mild,
        TasteTag::Comforting,
        TasteTag::Fancy,
        TasteTag::Quick,
        TasteTag::Filling,
        TasteTag::Spiced,
        TasteTag::Fresh,
    ];

    /// The `taste.<id>` i18n suffix.
    pub fn id(self) -> &'static str {
        match self {
            TasteTag::Warm => "warm",
            TasteTag::Cold => "cold",
            TasteTag::Sweet => "sweet",
            TasteTag::Bitter => "bitter",
            TasteTag::Strong => "strong",
            TasteTag::Mild => "mild",
            TasteTag::Comforting => "comforting",
            TasteTag::Fancy => "fancy",
            TasteTag::Quick => "quick",
            TasteTag::Filling => "filling",
            TasteTag::Spiced => "spiced",
            TasteTag::Fresh => "fresh",
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            TasteTag::Warm => Icon::Flame,
            TasteTag::Cold => Icon::Snowflake,
            TasteTag::Sweet => Icon::Candy,
            TasteTag::Bitter => Icon::Bean,
            TasteTag::Strong => Icon::Bolt,
            TasteTag::Mild => Icon::Feather,
            TasteTag::Comforting => Icon::Heart,
            TasteTag::Fancy => Icon::Gem,
            TasteTag::Quick => Icon::Stopwatch,
            TasteTag::Filling => Icon::Bowl,
            TasteTag::Spiced => Icon::Anise,
            TasteTag::Fresh => Icon::Leaf,
        }
    }

    /// Soft identity colour of the tag (icons, chips). Never red.
    pub fn color(self) -> Color {
        Color::from_hex(match self {
            TasteTag::Warm => 0xF9A15A,
            TasteTag::Cold => 0x8FD8FF,
            TasteTag::Sweet => 0xF6A9CF,
            TasteTag::Bitter => 0xD2AE86,
            TasteTag::Strong => 0xFFD666,
            TasteTag::Mild => 0xC9BDFF,
            TasteTag::Comforting => 0xFFADA8,
            TasteTag::Fancy => 0xB79CFF,
            TasteTag::Quick => 0x7FE6F2,
            TasteTag::Filling => 0xEAC77A,
            TasteTag::Spiced => 0xF29A6E,
            TasteTag::Fresh => 0x79E3B6,
        })
    }
}

/// Localised names of the 12 tags, in [`TasteTag::ALL`] order (tooltips, legends).
#[derive(Clone, Debug, Default)]
pub struct TasteNames {
    pub names: [String; 12],
}

impl TasteNames {
    pub fn get(&self, t: TasteTag) -> &str {
        let i = TasteTag::ALL.iter().position(|x| *x == t).unwrap_or(0);
        &self.names[i]
    }
}

/// How a taste icon is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TasteMark {
    /// Wanted by the wish (full colour).
    Want,
    /// The wish avoids it (dim, struck through).
    Avoid,
    /// A recipe tag that matches the wish (lit, with a glow).
    Match,
    /// A recipe tag that does not matter for this wish (faint).
    Plain,
}

/// One taste icon in a 18 pt round well. Returns its rect (for tooltips).
pub fn taste_dot(c: Vec2, d: f32, tag: TasteTag, mark: TasteMark) -> Rect {
    let col = tag.color();
    let r = Rect::new(c.x - d / 2.0, c.y - d / 2.0, d, d);
    match mark {
        TasteMark::Want => {
            fill_circle(c, d / 2.0, with_a(col, 0.18));
            stroke_circle(c, d / 2.0 - 0.5, 1.0, with_a(col, 0.42));
            draw_icon(tag.icon(), c, d * 0.62, col);
        }
        TasteMark::Match => {
            draw::glow_circle(c, d * 0.9, with_a(col, 0.35));
            fill_circle(c, d / 2.0, with_a(col, 0.28));
            stroke_circle(c, d / 2.0 - 0.5, 1.0, with_a(col, 0.7));
            draw_icon(tag.icon(), c, d * 0.62, mix(col, WHITE, 0.25));
        }
        TasteMark::Avoid => {
            fill_circle(c, d / 2.0, Color::new(1.0, 1.0, 1.0, 0.05));
            let dim = mix(col, Color::from_hex(0x6B6890), 0.55);
            draw_icon(tag.icon(), c, d * 0.6, dim);
            line(
                c + vec2(-d * 0.34, d * 0.34),
                c + vec2(d * 0.34, -d * 0.34),
                1.4,
                Color::from_hex(0xA9A7C9),
            );
        }
        TasteMark::Plain => {
            draw_icon(
                tag.icon(),
                c,
                d * 0.58,
                mix(col, Color::from_hex(0x5E5B84), 0.55),
            );
        }
    }
    r
}

/// A row of taste icons from `x`, centred on `cy`. Returns the end x.
pub fn taste_row(
    ui: &mut Ui,
    key: impl std::hash::Hash,
    x: f32,
    cy: f32,
    d: f32,
    tags: &[(TasteTag, TasteMark)],
    names: Option<&TasteNames>,
) -> f32 {
    let mut cx = x + d / 2.0;
    let id = ui.id(key);
    for (i, (tag, mark)) in tags.iter().enumerate() {
        let r = taste_dot(vec2(cx, cy), d, *tag, *mark);
        if let Some(n) = names {
            let resp = ui.interact(id.with(i), r);
            ui.tooltip(&resp, n.get(*tag));
        }
        cx += d + 3.0;
    }
    cx - d / 2.0
}

// ---------------------------------------------------------------------------
// Duties
// ---------------------------------------------------------------------------

/// The six delegable duties (design §1.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Duty {
    Dishes,
    Floor,
    Register,
    Bar,
    Kitchen,
    Serving,
}

impl Duty {
    pub const ALL: [Duty; 6] = [
        Duty::Dishes,
        Duty::Floor,
        Duty::Register,
        Duty::Bar,
        Duty::Kitchen,
        Duty::Serving,
    ];
    /// The `duty.<id>` i18n suffix.
    pub fn id(self) -> &'static str {
        match self {
            Duty::Dishes => "dishes",
            Duty::Floor => "floor",
            Duty::Register => "register",
            Duty::Bar => "bar",
            Duty::Kitchen => "kitchen",
            Duty::Serving => "serving",
        }
    }
    pub fn icon(self) -> Icon {
        match self {
            Duty::Dishes => Icon::Dishes,
            Duty::Floor => Icon::Broom,
            Duty::Register => Icon::CashRegister,
            Duty::Bar => Icon::Coffee,
            Duty::Kitchen => Icon::ChefHat,
            Duty::Serving => Icon::Tray,
        }
    }
    pub fn index(self) -> usize {
        Duty::ALL.iter().position(|d| *d == self).unwrap_or(0)
    }
}

// ---------------------------------------------------------------------------
// Dish glyphs
// ---------------------------------------------------------------------------

/// What a recipe looks like as a tiny illustration (order chips, book cards, tickets).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DishGlyph {
    /// Small cup on a saucer (black coffee, turek).
    Espresso,
    /// Wide cup with foam (cappuccino, flat white, raf).
    Cup,
    /// Big mug (hot chocolate, chai).
    Mug,
    /// Tea glass in a metal holder with a lemon slice.
    TeaGlass,
    /// Tall cold glass with ice.
    ColdGlass,
    /// Glass mug with an orange slice (svařák).
    MulledGlass,
    /// Soup bowl.
    Bowl,
    /// Open toast / open sandwich on a plate.
    Plate,
    /// Round koláč with a filling.
    Pastry,
    /// A slice of layered cake.
    Cake,
    /// Dumplings on a plate.
    Dumplings,
}

/// A dish glyph with its main colour (liquid, filling, topping).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dish {
    pub glyph: DishGlyph,
    pub tint: Color,
    /// Draw rising steam (warm drinks and soups).
    pub steam: bool,
}

impl Dish {
    pub fn new(glyph: DishGlyph, tint: Color) -> Self {
        Dish {
            glyph,
            tint,
            steam: false,
        }
    }
    pub fn steam(mut self, s: bool) -> Self {
        self.steam = s;
        self
    }
}

/// Points of an ellipse arc (radians, y-down) — convex when the sweep ≤ π plus the chord.
fn ellipse_pts(c: Vec2, rx: f32, ry: f32, a0: f32, a1: f32, n: usize) -> Vec<Vec2> {
    (0..=n)
        .map(|i| {
            let a = a0 + (a1 - a0) * i as f32 / n as f32;
            c + vec2(a.cos() * rx, a.sin() * ry)
        })
        .collect()
}

/// `fill_convex` taking a colour or a paint.
fn fillp(pts: &[Vec2], paint: impl Into<Paint>) {
    fill_convex(pts, paint.into());
}

fn fill_ellipse(c: Vec2, rx: f32, ry: f32, paint: impl Into<Paint>) {
    let mut p = ellipse_pts(c, rx, ry, 0.0, std::f32::consts::TAU, 40);
    p.pop();
    fillp(&p, paint.into());
}

/// Draw `dish` centred at `c` in a `size`-pt box. `silhouette` paints it as a flat shape
/// (undiscovered recipes). `t` animates the steam.
pub fn draw_dish(c: Vec2, size: f32, dish: Dish, silhouette: Option<Color>, t: f64) {
    use std::f32::consts::PI;
    let k = size / 24.0;
    let o = c - vec2(12.0, 12.0) * k;
    let p = |x: f32, y: f32| o + vec2(x, y) * k;
    let sil = silhouette;
    let ceramic_top = sil.unwrap_or(Color::from_hex(0xF4F0FA));
    let ceramic_bot = sil.unwrap_or(Color::from_hex(0xC9C1DC));
    let ceramic = |r: Rect| Paint::vertical(r, ceramic_top, ceramic_bot);
    let tint = sil.map(|s| mix(s, WHITE, 0.12)).unwrap_or(dish.tint);
    let shade = sil.unwrap_or(Color::from_hex(0x9E95B8));
    let saucer = |y: f32, rx: f32| {
        fill_ellipse(
            p(12.0, y),
            rx * k,
            2.3 * k,
            sil.unwrap_or(Color::from_hex(0xB7AECB)),
        );
        fill_ellipse(
            p(12.0, y - 0.5),
            (rx - 1.6) * k,
            1.4 * k,
            sil.unwrap_or(Color::from_hex(0xE7E1F0)),
        );
    };
    let plate = |y: f32| {
        fill_ellipse(
            p(12.0, y + 0.6),
            10.4 * k,
            3.6 * k,
            sil.unwrap_or(Color::from_hex(0x9F96B8)),
        );
        fill_ellipse(
            p(12.0, y),
            10.2 * k,
            3.4 * k,
            sil.unwrap_or(Color::from_hex(0xEEE9F6)),
        );
        fill_ellipse(
            p(12.0, y),
            7.0 * k,
            2.2 * k,
            sil.unwrap_or(Color::from_hex(0xDCD5E8)),
        );
    };
    let w = (1.6 * k).max(1.0);
    let mut steam_x: Vec<f32> = Vec::new();
    let mut steam_y = 6.0;
    match dish.glyph {
        DishGlyph::Espresso => {
            saucer(19.8, 8.4);
            stroke_path(
                &crate::ui::draw::arc_path(p(16.2, 15.0), 2.2 * k, -PI / 2.0, PI / 2.0),
                false,
                w,
                Paint::Solid(ceramic_bot),
            );
            let body = [
                p(6.6, 12.0),
                p(16.4, 12.0),
                p(15.0, 18.4),
                p(13.6, 19.4),
                p(9.4, 19.4),
                p(8.0, 18.4),
            ];
            fillp(
                &body,
                ceramic(Rect::new(p(6.6, 12.0).x, p(0.0, 12.0).y, 10.0 * k, 7.4 * k)),
            );
            fill_ellipse(p(11.5, 12.1), 4.7 * k, 1.1 * k, tint);
            steam_x = vec![10.0, 13.0];
            steam_y = 9.6;
        }
        DishGlyph::Cup => {
            saucer(19.6, 9.6);
            stroke_path(
                &crate::ui::draw::arc_path(p(18.4, 13.4), 2.6 * k, -PI / 2.0, PI / 2.0),
                false,
                w,
                Paint::Solid(ceramic_bot),
            );
            let mut body = ellipse_pts(p(11.4, 10.6), 7.6 * k, 8.2 * k, 0.0, PI, 24);
            body.reverse();
            fillp(
                &body,
                ceramic(Rect::new(p(3.8, 0.0).x, p(0.0, 10.6).y, 15.2 * k, 8.2 * k)),
            );
            fill_ellipse(p(11.4, 10.7), 7.3 * k, 1.9 * k, tint);
            if sil.is_none() {
                fill_ellipse(p(11.6, 10.6), 4.2 * k, 1.05 * k, Color::from_hex(0xF6E7D2));
                // a tiny heart of latte art
                fill_circle(p(11.0, 10.4), 0.75 * k, with_a(tint, 0.9));
                fill_circle(p(12.2, 10.4), 0.75 * k, with_a(tint, 0.9));
            }
            steam_x = vec![9.0, 12.0, 15.0];
            steam_y = 7.6;
        }
        DishGlyph::Mug => {
            stroke_path(
                &crate::ui::draw::arc_path(p(17.4, 14.2), 3.2 * k, -PI / 2.0, PI / 2.0),
                false,
                (2.2 * k).max(1.2),
                Paint::Solid(ceramic_bot),
            );
            let body = Rect::new(p(5.2, 8.2).x, p(5.2, 8.2).y, 12.4 * k, 12.6 * k);
            fill_rrect(body, 2.6 * k, ceramic(body));
            fill_ellipse(p(11.4, 8.6), 5.9 * k, 1.4 * k, tint);
            if sil.is_none() {
                fill_circle(p(9.8, 8.0), 1.6 * k, Color::from_hex(0xFFF6EA));
                fill_circle(p(12.2, 7.6), 1.9 * k, Color::from_hex(0xFFF6EA));
                // a band on the mug
                fill_rrect(
                    Rect::new(body.x, body.y + body.h * 0.55, body.w, 1.6 * k),
                    0.0,
                    with_a(Color::from_hex(0x8B83D6), 0.55),
                );
            }
            steam_x = vec![9.4, 13.0];
            steam_y = 5.4;
        }
        DishGlyph::TeaGlass => {
            let glass = [p(6.4, 6.6), p(16.6, 6.6), p(15.2, 19.6), p(7.8, 19.6)];
            fillp(&glass, with_a(tint, if sil.is_some() { 1.0 } else { 0.82 }));
            if sil.is_none() {
                fillp(
                    &[p(7.4, 7.4), p(9.0, 7.4), p(9.2, 18.4), p(8.4, 18.4)],
                    Color::new(1.0, 1.0, 1.0, 0.22),
                );
            }
            // metal holder
            let holder = Rect::new(p(6.8, 14.4).x, p(6.8, 14.4).y, 9.4 * k, 6.4 * k);
            fill_rrect(
                holder,
                1.6 * k,
                Paint::vertical(
                    holder,
                    sil.unwrap_or(Color::from_hex(0xE8C77A)),
                    sil.unwrap_or(Color::from_hex(0xA67C3C)),
                ),
            );
            stroke_path(
                &crate::ui::draw::arc_path(p(16.6, 16.6), 2.4 * k, -PI / 2.0, PI / 2.0),
                false,
                w,
                Paint::Solid(sil.unwrap_or(Color::from_hex(0xC9A060))),
            );
            // lemon
            fill_circle(
                p(16.0, 6.6),
                3.0 * k,
                sil.unwrap_or(Color::from_hex(0xF2D64B)),
            );
            fill_circle(
                p(16.0, 6.6),
                2.2 * k,
                sil.unwrap_or(Color::from_hex(0xFBEFA0)),
            );
            steam_x = vec![9.6, 12.4];
            steam_y = 4.2;
        }
        DishGlyph::ColdGlass => {
            let glass = [p(6.6, 4.4), p(17.4, 4.4), p(15.8, 21.0), p(8.2, 21.0)];
            fillp(&glass, with_a(tint, if sil.is_some() { 1.0 } else { 0.85 }));
            if sil.is_none() {
                for (x, y, a) in [(9.4, 8.6, 0.3_f32), (12.8, 11.4, -0.4), (10.4, 14.6, 0.6)] {
                    let (s, co) = a.sin_cos();
                    let h = 1.9 * k;
                    let q = p(x, y);
                    let pts: Vec<Vec2> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                        .iter()
                        .map(|(dx, dy)| q + vec2(dx * co - dy * s, dx * s + dy * co) * h)
                        .collect();
                    fillp(&pts, Color::new(1.0, 1.0, 1.0, 0.42));
                }
                fillp(
                    &[p(7.6, 5.2), p(9.0, 5.2), p(9.6, 19.6), p(8.8, 19.6)],
                    Color::new(1.0, 1.0, 1.0, 0.2),
                );
            }
            line(
                p(14.6, 1.4),
                p(12.6, 11.0),
                (1.4 * k).max(1.0),
                sil.unwrap_or(Color::from_hex(0xF2EEF8)),
            );
        }
        DishGlyph::MulledGlass => {
            stroke_path(
                &crate::ui::draw::arc_path(p(17.2, 13.6), 3.0 * k, -PI / 2.0, PI / 2.0),
                false,
                w,
                Paint::Solid(sil.unwrap_or(Color::from_hex(0xD9D2EA))),
            );
            let glass = [p(5.6, 7.4), p(17.6, 7.4), p(16.6, 20.4), p(6.6, 20.4)];
            fillp(&glass, with_a(tint, if sil.is_some() { 1.0 } else { 0.9 }));
            if sil.is_none() {
                fillp(
                    &[p(6.8, 8.2), p(8.2, 8.2), p(8.6, 19.0), p(7.6, 19.0)],
                    Color::new(1.0, 1.0, 1.0, 0.2),
                );
            }
            fill_circle(
                p(6.4, 7.6),
                3.0 * k,
                sil.unwrap_or(Color::from_hex(0xF59E4C)),
            );
            fill_circle(
                p(6.4, 7.6),
                2.1 * k,
                sil.unwrap_or(Color::from_hex(0xFFC98A)),
            );
            line(
                p(13.0, 3.0),
                p(15.0, 10.0),
                (1.6 * k).max(1.0),
                sil.unwrap_or(Color::from_hex(0x8B5A33)),
            );
            steam_x = vec![10.4, 13.6];
            steam_y = 4.4;
        }
        DishGlyph::Bowl => {
            let mut body = ellipse_pts(p(12.0, 11.8), 9.4 * k, 8.0 * k, 0.0, PI, 26);
            body.reverse();
            fillp(
                &body,
                ceramic(Rect::new(p(2.6, 0.0).x, p(0.0, 11.8).y, 18.8 * k, 8.0 * k)),
            );
            fill_ellipse(p(12.0, 11.9), 9.0 * k, 2.3 * k, tint);
            if sil.is_none() {
                fill_circle(p(9.0, 11.6), 0.9 * k, Color::from_hex(0x6FC46B));
                fill_circle(p(14.6, 12.2), 0.8 * k, Color::from_hex(0x6FC46B));
                fill_circle(p(12.0, 11.2), 1.3 * k, Color::from_hex(0xF2DDB0));
                fill_rrect(
                    Rect::new(p(6.0, 15.2).x, p(6.0, 15.2).y, 12.0 * k, 1.2 * k),
                    0.6 * k,
                    with_a(shade, 0.5),
                );
            }
            steam_x = vec![8.6, 12.0, 15.4];
            steam_y = 7.6;
        }
        DishGlyph::Plate => {
            plate(16.2);
            let toast = [p(5.6, 11.6), p(16.6, 9.4), p(18.6, 13.8), p(7.4, 16.0)];
            fillp(&toast, sil.unwrap_or(mix(tint, BLACK, 0.12)));
            let top = [p(6.0, 11.2), p(16.4, 9.0), p(18.0, 12.8), p(7.4, 15.0)];
            fillp(&top, tint);
            if sil.is_none() {
                for (x, y, c) in [
                    (10.0, 11.6, 0xF7F1E3_u32),
                    (13.0, 11.0, 0xE85D5D),
                    (14.8, 12.0, 0x7CC36A),
                ] {
                    fill_circle(p(x, y), 1.1 * k, Color::from_hex(c));
                }
            }
        }
        DishGlyph::Pastry => {
            plate(16.4);
            fill_ellipse(
                p(12.0, 13.4),
                7.4 * k,
                4.6 * k,
                sil.unwrap_or(Color::from_hex(0xC98A43)),
            );
            fill_ellipse(
                p(12.0, 12.8),
                7.0 * k,
                4.2 * k,
                sil.unwrap_or(Color::from_hex(0xE9B567)),
            );
            fill_ellipse(p(12.0, 12.6), 3.6 * k, 2.2 * k, tint);
            if sil.is_none() {
                for a in 0..6 {
                    let ang = a as f32 / 6.0 * std::f32::consts::TAU;
                    fill_circle(
                        p(12.0 + ang.cos() * 5.4, 12.8 + ang.sin() * 3.2),
                        0.55 * k,
                        Color::from_hex(0xFFF1C9),
                    );
                }
            }
        }
        DishGlyph::Cake => {
            plate(17.4);
            let side = [p(5.4, 11.2), p(18.4, 11.2), p(18.4, 17.0), p(5.4, 17.0)];
            fillp(&side, tint);
            let top = [p(5.4, 11.2), p(10.4, 7.6), p(18.4, 11.2)];
            fillp(&top, sil.unwrap_or(mix(tint, WHITE, 0.25)));
            if sil.is_none() {
                for y in [12.8, 14.6] {
                    fill_rrect(
                        Rect::new(p(5.4, y).x, p(0.0, y).y, 13.0 * k, 0.8 * k),
                        0.0,
                        Color::from_hex(0xFFF2D6),
                    );
                }
                fill_circle(p(10.6, 8.6), 1.1 * k, Color::from_hex(0xFFF2D6));
            }
        }
        DishGlyph::Dumplings => {
            plate(16.0);
            for (x, y) in [(8.0, 13.6), (12.6, 12.2), (15.8, 14.6)] {
                let mut half = ellipse_pts(
                    p(x, y + 1.6),
                    3.4 * k,
                    3.6 * k,
                    PI,
                    std::f32::consts::TAU,
                    16,
                );
                half.reverse();
                fillp(&half, sil.unwrap_or(Color::from_hex(0xF5E6C6)));
            }
            if sil.is_none() {
                for (x, y) in [(9.0, 13.6), (12.0, 12.0), (15.4, 14.2), (13.6, 15.6)] {
                    fill_circle(p(x, y), 0.9 * k, tint);
                }
            }
        }
    }
    if dish.steam && sil.is_none() {
        for (i, x) in steam_x.iter().enumerate() {
            let ph = (t * 1.3 + i as f64 * 0.7) as f32;
            let lift = (ph % 1.0) * 2.4;
            let a = 0.55 * (1.0 - (ph % 1.0));
            let pts: Vec<Vec2> = (0..=14)
                .map(|j| {
                    let f = j as f32 / 14.0;
                    p(
                        x + (f * 3.4 + ph * 2.0).sin() * 0.7,
                        steam_y - lift - f * 4.0,
                    )
                })
                .collect();
            stroke_path(
                &pts,
                false,
                (1.1 * k).max(0.9),
                Paint::Solid(Color::new(1.0, 1.0, 1.0, a * 0.7)),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Rings, dots, small chips
// ---------------------------------------------------------------------------

/// A thin progress ring around `c` (0 = empty, 1 = full), starting at 12 o'clock.
pub fn ring(c: Vec2, r: f32, width: f32, frac: f32, track: Color, fill: Color) {
    use std::f32::consts::{FRAC_PI_2, TAU};
    stroke_circle(c, r, width, track);
    let f = frac.clamp(0.0, 1.0);
    if f > 0.005 {
        arc(c, r, -FRAC_PI_2, -FRAC_PI_2 + TAU * f, width, fill);
    }
}

/// Wait colour: indigo → amber as the guest waits longer (never red on Cozy).
pub fn wait_color(t: &Theme, wait: f32) -> Color {
    let w = wait.clamp(0.0, 1.0);
    if w < 0.5 {
        mix(t.indigo_300, t.amber, w * 2.0)
    } else {
        mix(t.amber, t.amber_deep, (w - 0.5) * 2.0)
    }
}

/// Capacity dots: `used` filled of `cap` (max 3), left to right from `x`, centred on `cy`.
pub fn capacity_dots(t: &Theme, x: f32, cy: f32, used: usize, cap: usize) -> f32 {
    let mut cx = x + 3.5;
    for i in 0..cap.max(1) {
        let c = vec2(cx, cy);
        if i < used {
            fill_circle(
                c,
                3.2,
                Paint::vertical(
                    Rect::new(c.x - 3.2, c.y - 3.2, 6.4, 6.4),
                    t.indigo_300,
                    t.violet,
                ),
            );
        } else {
            stroke_circle(c, 2.8, 1.1, with_a(t.indigo_300, 0.55));
        }
        cx += 9.0;
    }
    cx - 3.5 - 2.0
}

/// A small pill with optional icon, for costs ("−15 Kč") and hints. Height 18.
pub fn mini_chip(
    ui: &mut Ui,
    pos: Vec2,
    text: &str,
    fg: Color,
    bg: Color,
    icon: Option<Icon>,
) -> Rect {
    let st = TextStyle::new(11.0, Weight::SemiBold, fg).tabular();
    let w = ui.fonts.measure(text, &st) + 12.0 + if icon.is_some() { 14.0 } else { 0.0 };
    let r = Rect::new(pos.x, pos.y, w.ceil(), 18.0);
    fill_rrect(r, 9.0, bg);
    let mut x = r.x + 6.0;
    if let Some(i) = icon {
        draw_icon(i, vec2(x + 5.0, r.center_y()), 11.0, fg);
        x += 14.0;
    }
    ui.fonts.draw(text, x, ui.fonts.baseline_in(r, &st), &st);
    r
}

/// Width of a [`mini_chip`].
pub fn mini_chip_width(ui: &Ui, text: &str, icon: bool) -> f32 {
    let st = TextStyle::new(11.0, Weight::SemiBold, WHITE).tabular();
    (ui.fonts.measure(text, &st) + 12.0 + if icon { 14.0 } else { 0.0 }).ceil()
}

/// Small 14 pt keycap-like hint ("1"…"4") with its top-left corner at `pos`.
pub fn corner_key(ui: &mut Ui, pos: Vec2, label: &str) {
    let st = TextStyle::new(10.0, Weight::Bold, ui.theme.text_dim);
    let k = Rect::new(pos.x, pos.y, 14.0, 14.0);
    fill_rrect(k, 4.0, Color::new(0.03, 0.02, 0.09, 0.45));
    stroke_rrect(k, 4.0, 1.0, Color::new(1.0, 1.0, 1.0, 0.1));
    ui.fonts.draw_in(label, k, &st, Align::Center);
}

/// A tiny pointer triangle from a card's edge towards `target` (for popovers anchored in
/// the world).
pub fn pointer(card: Rect, target: Vec2, color: Color) {
    let c = card.center();
    let dir = (target - c).normalize_or_zero();
    if dir == Vec2::ZERO || card.contains(target) {
        return;
    }
    // where the ray leaves the card (with the corner radius kept clear)
    let inner = card.inset(14.0);
    let tx = if dir.x.abs() > 1e-4 {
        ((if dir.x > 0.0 { card.right() } else { card.x }) - c.x) / dir.x
    } else {
        f32::MAX
    };
    let ty = if dir.y.abs() > 1e-4 {
        ((if dir.y > 0.0 { card.bottom() } else { card.y }) - c.y) / dir.y
    } else {
        f32::MAX
    };
    let mut base = c + dir * tx.min(ty);
    base.x = base.x.clamp(inner.x, inner.right());
    base.y = base.y.clamp(inner.y, inner.bottom());
    let horizontal = tx < ty;
    let (n, out) = if horizontal {
        (vec2(0.0, 1.0), vec2(dir.x.signum(), 0.0))
    } else {
        (vec2(1.0, 0.0), vec2(0.0, dir.y.signum()))
    };
    let base = base - out * 0.5;
    fillp(
        &[base + n * 7.0, base + out * 8.0, base - n * 7.0],
        Paint::Solid(color),
    );
}

/// Place a `size` popover next to the world point `at` (screen UI units): right, left,
/// above or below — always fully on screen, preferring the side that covers the least of
/// `avoid` (HUD blocks, other open cards).
pub fn place_near(screen: Rect, at: Vec2, size: Vec2, gap: f32, avoid: &[Rect]) -> Rect {
    let m = 10.0;
    let clamp = |r: Rect| {
        Rect::new(
            r.x.clamp(m, (screen.right() - m - r.w).max(m)),
            r.y.clamp(m, (screen.bottom() - m - r.h).max(m)),
            r.w,
            r.h,
        )
    };
    let cy = at.y - size.y * 0.45;
    let cx = at.x - size.x / 2.0;
    // beside the anchor (vertically centred, or slid just past each avoided block) …
    let mut ys = vec![cy];
    let mut xs = vec![cx];
    for a in avoid {
        ys.push(a.bottom() + 8.0);
        ys.push(a.y - 8.0 - size.y);
        xs.push(a.right() + 8.0);
        xs.push(a.x - 8.0 - size.x);
    }
    let mut candidates: Vec<(Rect, f32)> = Vec::new();
    for (k, y) in ys.iter().enumerate() {
        let slide = if k == 0 { 0.0 } else { 1.0 };
        candidates.push((Rect::new(at.x + gap, *y, size.x, size.y), slide));
        candidates.push((
            Rect::new(at.x - gap - size.x, *y, size.x, size.y),
            slide + 0.5,
        ));
    }
    // … or above / below it
    for (k, x) in xs.iter().enumerate() {
        let slide = if k == 0 { 1.0 } else { 2.0 };
        candidates.push((Rect::new(*x, at.y - gap - size.y, size.x, size.y), slide));
        candidates.push((Rect::new(*x, at.y + gap, size.x, size.y), slide + 0.5));
    }
    let covered = |r: &Rect| -> f32 {
        let anchor = if r.grow(4.0).contains(at) {
            size.x * size.y
        } else {
            0.0
        };
        anchor
            + avoid
                .iter()
                .map(|a| {
                    let i = a.intersect_or_empty(*r);
                    i.w * i.h
                })
                .sum::<f32>()
    };
    // least overlap first; then the least clamping, then the nicest side
    let mut best = clamp(candidates[0].0);
    let mut best_score = f32::MAX;
    for (c, pref) in &candidates {
        let cl = clamp(*c);
        let moved = (cl.x - c.x).abs() + (cl.y - c.y).abs();
        let score = covered(&cl) * 4.0 + moved * 3.0 + pref * 10.0;
        if score < best_score {
            best_score = score;
            best = cl;
        }
    }
    best
}

// ---------------------------------------------------------------------------
// Transformed drawing (rotation, slant) via the camera matrix
// ---------------------------------------------------------------------------

struct MatrixCamera(Mat4);

impl Camera for MatrixCamera {
    fn matrix(&self) -> Mat4 {
        self.0
    }
    fn depth_enabled(&self) -> bool {
        false
    }
    fn render_pass(&self) -> Option<macroquad::texture::RenderPass> {
        None
    }
    fn viewport(&self) -> Option<(i32, i32, i32, i32)> {
        None
    }
}

/// A 2D affine map in UI units: `p' = m · p + t`.
#[derive(Clone, Copy, Debug)]
pub struct Affine {
    pub m: Mat2,
    pub t: Vec2,
}

impl Affine {
    pub const IDENTITY: Affine = Affine {
        m: Mat2::IDENTITY,
        t: Vec2::ZERO,
    };
    /// Rotation by `deg` degrees (clockwise on screen) around `pivot`.
    pub fn rotate(pivot: Vec2, deg: f32) -> Self {
        let m = Mat2::from_angle(deg.to_radians());
        Affine {
            m,
            t: pivot - m * pivot,
        }
    }
    /// Horizontal slant around the line `y = baseline` (faux italic: `k ≈ 0.2`).
    pub fn slant(baseline: f32, k: f32) -> Self {
        let m = Mat2::from_cols(vec2(1.0, 0.0), vec2(-k, 1.0));
        Affine {
            m,
            t: vec2(k * baseline, 0.0),
        }
    }
    /// `self` applied after `inner`.
    pub fn then(self, inner: Affine) -> Self {
        Affine {
            m: self.m * inner.m,
            t: self.m * inner.t + self.t,
        }
    }
    pub fn apply(&self, p: Vec2) -> Vec2 {
        self.m * p + self.t
    }
    fn mat4(&self) -> Mat4 {
        Mat4::from_cols(
            vec4(self.m.x_axis.x, self.m.x_axis.y, 0.0, 0.0),
            vec4(self.m.y_axis.x, self.m.y_axis.y, 0.0, 0.0),
            vec4(0.0, 0.0, 1.0, 0.0),
            vec4(self.t.x, self.t.y, 0.0, 1.0),
        )
    }
}

thread_local! {
    /// The transform in effect (nested `with_transform` calls compose).
    static XF: std::cell::Cell<Option<Affine>> = const { std::cell::Cell::new(None) };
}

fn set_xf_camera(ui: &Ui, xf: Option<Affine>) {
    match xf {
        None => ui.set_camera(),
        Some(xf) => {
            let s = ui.screen();
            let mut cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, s.w, s.h));
            cam.zoom.y = -cam.zoom.y;
            set_camera(&MatrixCamera(cam.matrix() * xf.mat4()));
        }
    }
}

/// Run `f` with everything it paints transformed by `xf` (UI units), then restore the
/// previous transform (or the plain UI camera). Nested calls compose. Hit-testing is not
/// transformed: keep interactive parts axis-aligned.
pub fn with_transform<R>(ui: &mut Ui, xf: Affine, f: impl FnOnce(&mut Ui) -> R) -> R {
    let outer = XF.with(|c| c.get());
    let combined = match outer {
        Some(o) => o.then(xf),
        None => xf,
    };
    XF.with(|c| c.set(Some(combined)));
    set_xf_camera(ui, Some(combined));
    let out = f(ui);
    XF.with(|c| c.set(outer));
    set_xf_camera(ui, outer);
    out
}

/// "Handwritten" text: Inter with a faux-italic slant and a gentle baseline wobble.
/// Draws one line with its baseline at `y`; returns the width.
pub fn draw_handwritten(ui: &mut Ui, text: &str, x: f32, y: f32, style: &TextStyle) -> f32 {
    let w = ui.fonts.measure(text, style);
    with_transform(ui, Affine::slant(y, 0.21), |ui| {
        ui.fonts.draw(text, x, y, style)
    });
    w
}

/// Wrapped handwritten paragraph inside `r` (at most `max_lines`, last one ellipsised).
/// Returns the used height.
pub fn draw_handwritten_block(
    ui: &mut Ui,
    text: &str,
    r: Rect,
    style: &TextStyle,
    align: Align,
    max_lines: usize,
) -> f32 {
    // slanted glyphs lean right by ~0.21 × cap height: wrap a little narrower
    let lean = style.size * 0.16;
    let lines = ui.fonts.wrap(text, (r.w - lean).max(10.0), style);
    let n = lines.len().min(max_lines.max(1));
    let lh = style.line_h();
    let cap = ui.fonts.cap_height(style);
    for (i, l) in lines.iter().take(n).enumerate() {
        let mut l = l.clone();
        if i + 1 == n && n < lines.len() {
            l = format!("{}…", l.trim_end());
        }
        let l = ui.fonts.ellipsize(&l, r.w - lean, style).into_owned();
        let w = ui.fonts.measure(&l, style);
        let x = match align {
            Align::Left => r.x,
            Align::Center => r.x + (r.w - lean - w) / 2.0 + lean * 0.5,
            Align::Right => r.right() - lean - w,
        };
        let base = r.y + i as f32 * lh + (lh + cap) * 0.5;
        draw_handwritten(ui, &l, x, base, style);
    }
    n as f32 * lh
}

/// Height a handwritten block would take.
pub fn handwritten_height(ui: &Ui, text: &str, w: f32, style: &TextStyle, max_lines: usize) -> f32 {
    let lean = style.size * 0.16;
    let n = ui
        .fonts
        .wrap(text, (w - lean).max(10.0), style)
        .len()
        .min(max_lines.max(1));
    n as f32 * style.line_h()
}

/// A dashed rounded-rect outline (empty slots, missing polaroids).
pub fn dashed_rrect(r: Rect, rad: f32, width: f32, dash: f32, color: Color) {
    let path = crate::ui::draw::rounded_rect_path(r.inset(width * 0.5), rad);
    let n = path.len();
    let mut acc = 0.0;
    let mut on = true;
    let mut seg: Vec<Vec2> = Vec::new();
    for i in 0..=n {
        let a = path[i % n];
        let b = path[(i + 1) % n];
        let len = a.distance(b);
        let mut t = 0.0;
        while t < len {
            let step = (dash - acc).min(len - t);
            let p0 = a + (b - a) * (t / len.max(1e-4));
            if on {
                if seg.is_empty() {
                    seg.push(p0);
                }
                seg.push(a + (b - a) * ((t + step) / len.max(1e-4)));
            }
            t += step;
            acc += step;
            if acc >= dash - 1e-3 {
                if on && seg.len() > 1 {
                    stroke_path(&seg, false, width, Paint::Solid(color));
                }
                seg.clear();
                on = !on;
                acc = 0.0;
            }
        }
    }
    if on && seg.len() > 1 {
        stroke_path(&seg, false, width, Paint::Solid(color));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taste_ids_unique() {
        let mut ids: Vec<&str> = TasteTag::ALL.iter().map(|t| t.id()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 12);
        assert_eq!(Duty::Kitchen.index(), 4);
    }

    #[test]
    fn affine_rotation_and_slant() {
        let r = Affine::rotate(vec2(10.0, 10.0), 90.0);
        let p = r.apply(vec2(20.0, 10.0));
        assert!((p - vec2(10.0, 20.0)).length() < 1e-4);
        let s = Affine::slant(50.0, 0.25);
        // points on the baseline stay put, points above lean right
        assert!((s.apply(vec2(3.0, 50.0)) - vec2(3.0, 50.0)).length() < 1e-4);
        assert!(s.apply(vec2(0.0, 30.0)).x > 4.9);
        let id = r.then(Affine::IDENTITY);
        assert!((id.apply(vec2(20.0, 10.0)) - p).length() < 1e-4);
    }

    #[test]
    fn place_near_stays_on_screen() {
        let s = Rect::new(0.0, 0.0, 800.0, 600.0);
        for at in [vec2(5.0, 5.0), vec2(790.0, 590.0), vec2(400.0, 300.0)] {
            let r = place_near(s, at, vec2(300.0, 200.0), 16.0, &[]);
            assert!(r.x >= 0.0 && r.y >= 0.0 && r.right() <= 800.0 && r.bottom() <= 600.0);
        }
        // a HUD block on the right pushes the card to the left
        let hud = Rect::new(420.0, 0.0, 380.0, 600.0);
        let r = place_near(s, vec2(400.0, 300.0), vec2(300.0, 200.0), 16.0, &[hud]);
        assert!(r.right() <= 420.0);
    }

    #[test]
    fn wait_color_never_red() {
        let t = Theme::SHIFTELLA;
        for i in 0..=10 {
            let c = wait_color(&t, i as f32 / 10.0);
            assert!(c.g > 0.4, "wait colour too red at {i}");
        }
        let _ = RectExt::inset(Rect::new(0.0, 0.0, 1.0, 1.0), 0.0);
    }
}
