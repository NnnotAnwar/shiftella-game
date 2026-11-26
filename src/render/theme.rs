//! Decor themes of the hall (`design/handson.md` §11.2): wall colours and a procedural
//! wall pattern, the floor variant, lamp shade styles, textile accents and curtains.
//!
//! No sprites per theme: the theme is a [`ThemeStyle`] — a small `Copy` bundle of colours
//! and style switches that walls, floors and furniture read while they draw. Set it with
//! [`set_decor`] (or scope it with [`with_decor`], e.g. to keep the kitchen in the default
//! look) before building the [`super::StaticLayer`] and before drawing furniture.
//! Every theme stays in the violet-dollhouse look: the night ambient multiplies over them.

use std::cell::Cell;

use macroquad::prelude::*;

use super::colors as c;
use super::draw::*;
use super::season::Season;
use crate::map::TILE;

/// A decor theme (ids as in `assets/data/themes.ron`: see [`DecorTheme::from_id`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DecorTheme {
    /// `grandma_original`: faded rose-stripe wallpaper, honey wood, lace, amber lamps.
    #[default]
    GrandmaOriginal,
    /// `art_nouveau`: Prague Secession — sage & gold mosaic band, brass tulip lamps.
    PragueArtNouveau,
    /// `station_retro`: a 1979 railway buffet — cream & bottle-green paint, enamel lamps.
    StationRetro,
    /// `nocturne_2039`: indigo holo accents, warm LED strips, Shiftella violet.
    Nocturne2039,
    /// `winter_lights`: white-gold fairy lights, red ribbons, spruce.
    WinterLights,
    /// Follows the season: autumn harvest colours in late autumn, winter lights later.
    Seasonal,
}

impl DecorTheme {
    pub const ALL: [DecorTheme; 6] = [
        DecorTheme::GrandmaOriginal,
        DecorTheme::PragueArtNouveau,
        DecorTheme::StationRetro,
        DecorTheme::Nocturne2039,
        DecorTheme::WinterLights,
        DecorTheme::Seasonal,
    ];

    /// The content id → theme (unknown → `None`).
    pub fn from_id(id: &str) -> Option<DecorTheme> {
        Some(match id {
            "grandma_original" => DecorTheme::GrandmaOriginal,
            "art_nouveau" | "prague_art_nouveau" => DecorTheme::PragueArtNouveau,
            "station_retro" => DecorTheme::StationRetro,
            "nocturne_2039" => DecorTheme::Nocturne2039,
            "winter_lights" => DecorTheme::WinterLights,
            "seasonal" => DecorTheme::Seasonal,
            _ => return None,
        })
    }

    pub fn id(self) -> &'static str {
        match self {
            DecorTheme::GrandmaOriginal => "grandma_original",
            DecorTheme::PragueArtNouveau => "art_nouveau",
            DecorTheme::StationRetro => "station_retro",
            DecorTheme::Nocturne2039 => "nocturne_2039",
            DecorTheme::WinterLights => "winter_lights",
            DecorTheme::Seasonal => "seasonal",
        }
    }

    /// The style for this theme in `season` (only `Seasonal` depends on it).
    pub fn style(self, season: Season) -> ThemeStyle {
        match self {
            DecorTheme::GrandmaOriginal => ThemeStyle::GRANDMA,
            DecorTheme::PragueArtNouveau => ThemeStyle::ART_NOUVEAU,
            DecorTheme::StationRetro => ThemeStyle::STATION_RETRO,
            DecorTheme::Nocturne2039 => ThemeStyle::NOCTURNE,
            DecorTheme::WinterLights => ThemeStyle::WINTER_LIGHTS,
            DecorTheme::Seasonal => match season {
                Season::LateAutumn => ThemeStyle::HARVEST,
                _ => ThemeStyle::WINTER_LIGHTS,
            },
        }
    }
}

/// A procedural pattern on the plaster part of the hall walls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WallPattern {
    Plain,
    /// Faded vertical stripes with tiny roses.
    RoseStripe,
    /// A Secession mosaic band with arches.
    MosaicBand,
    /// A tall painted dado above the wooden panels (railway buffet), enamel plaques.
    Wainscot,
    /// Thin indigo holo lines and a warm LED strip.
    HoloLines,
    /// Stencilled snowflakes and red ribbon swags.
    Snowflakes,
    /// Stencilled autumn leaves.
    Leaves,
}

/// The planks of a wooden floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FloorVariant {
    /// Long staggered planks (the default).
    Planks,
    /// Basket-weave parquet blocks.
    Parquet,
}

/// The shape of the lamp shades.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LampStyle {
    /// A plain cone (the default indigo shade).
    Cone,
    /// A fabric shade with a fringe (grandma).
    Fringed,
    /// A brass stem with a glass tulip (Art Nouveau).
    Tulip,
    /// A white enamel dome with a coloured rim (railway buffet).
    Enamel,
    /// A slim LED ring (2039).
    LedRing,
    /// A cone tied with a red ribbon bow (winter).
    Ribbon,
}

/// Curtains: a valance across the top of the window panes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CurtainStyle {
    None,
    /// Cream lace with a scalloped edge.
    Lace,
    /// A velvet pelmet with a fringe.
    Velvet,
    /// A half-drawn roller blind.
    Blind,
}

/// Everything a theme changes. `Copy`; read by walls, floors and furniture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemeStyle {
    pub wall_top: Color,
    pub wall_bottom: Color,
    pub wainscot_top: Color,
    pub wainscot_bottom: Color,
    pub trim: Color,
    pub pattern: WallPattern,
    /// The main colour of the wall pattern.
    pub pattern_color: Color,
    /// A second pattern colour (mosaic tesserae, ribbons, the LED strip).
    pub pattern_accent: Color,
    pub floor: FloorVariant,
    /// The wooden floor is mixed towards this colour by `floor_mix`.
    pub floor_tint: Color,
    pub floor_mix: f32,
    /// Chair cushions, stools, sofas.
    pub textile: Color,
    /// Armchairs.
    pub textile_alt: Color,
    /// Throw cushions, the window seat, small accents.
    pub accent: Color,
    pub rug_base: Color,
    pub rug_border: Color,
    pub rug_field: Color,
    pub rug_motif: Color,
    pub lamp: LampStyle,
    pub lampshade: Color,
    /// Lamp light colour shift: −1 cooler (white LED) … 0 as is … +1 warmer (deep amber).
    pub lamp_warmth: f32,
    pub curtains: CurtainStyle,
    pub curtain_color: Color,
    /// Lace doilies on the tables.
    pub lace: bool,
}

impl Default for ThemeStyle {
    fn default() -> Self {
        ThemeStyle::DEFAULT
    }
}

const fn hex(v: u32) -> Color {
    Color::from_hex(v)
}

impl ThemeStyle {
    /// The base look (as the café was drawn before themes existed).
    pub const DEFAULT: ThemeStyle = ThemeStyle {
        wall_top: c::WALL_PLASTER_TOP,
        wall_bottom: c::WALL_PLASTER_BOTTOM,
        wainscot_top: c::WAINSCOT_TOP,
        wainscot_bottom: c::WAINSCOT_BOTTOM,
        trim: c::WALL_TRIM,
        pattern: WallPattern::Plain,
        pattern_color: c::WALL_TRIM,
        pattern_accent: c::AMBER,
        floor: FloorVariant::Planks,
        floor_tint: c::WOOD,
        floor_mix: 0.0,
        textile: c::FABRIC_PLUM,
        textile_alt: c::FABRIC_TEAL,
        accent: c::AMBER_DEEP,
        rug_base: hex(0x3b2f7a),
        rug_border: hex(0xc98a4b),
        rug_field: hex(0x5a3f8f),
        rug_motif: hex(0x7c3aed),
        lamp: LampStyle::Cone,
        lampshade: hex(0x4c45a8),
        lamp_warmth: 0.0,
        curtains: CurtainStyle::None,
        curtain_color: c::CREAM,
        lace: false,
    };

    /// Grandma Lida's original café.
    pub const GRANDMA: ThemeStyle = ThemeStyle {
        wall_top: hex(0xa98aae),
        wall_bottom: hex(0x8a6c94),
        wainscot_top: hex(0x7a4f3a),
        wainscot_bottom: hex(0x5a3828),
        trim: hex(0xe6d2c4),
        pattern: WallPattern::RoseStripe,
        pattern_color: hex(0xc8a2bc),
        pattern_accent: hex(0xd77a8c),
        floor: FloorVariant::Planks,
        floor_tint: hex(0xb07a4e),
        floor_mix: 0.35,
        textile: hex(0xb06a7e),
        textile_alt: hex(0x8a6a9e),
        accent: hex(0xe0a35a),
        rug_base: hex(0x6a3456),
        rug_border: hex(0xd9a86a),
        rug_field: hex(0x8a4a6a),
        rug_motif: hex(0xe8c58a),
        lamp: LampStyle::Fringed,
        lampshade: hex(0xc9867e),
        lamp_warmth: 0.35,
        curtains: CurtainStyle::Lace,
        curtain_color: hex(0xf3ead8),
        lace: true,
    };

    /// Prague Secession.
    pub const ART_NOUVEAU: ThemeStyle = ThemeStyle {
        wall_top: hex(0x8a9a98),
        wall_bottom: hex(0x6a7a82),
        wainscot_top: hex(0x4a3a52),
        wainscot_bottom: hex(0x33283c),
        trim: hex(0xd9b45a),
        pattern: WallPattern::MosaicBand,
        pattern_color: hex(0xd9b45a),
        pattern_accent: hex(0x3f8a8c),
        floor: FloorVariant::Parquet,
        floor_tint: hex(0x9a6a44),
        floor_mix: 0.25,
        textile: hex(0x2f6e72),
        textile_alt: hex(0x8a6a2e),
        accent: hex(0xd9b45a),
        rug_base: hex(0x23404a),
        rug_border: hex(0xd9b45a),
        rug_field: hex(0x2f5a62),
        rug_motif: hex(0xc96a4a),
        lamp: LampStyle::Tulip,
        lampshade: hex(0xe8c27a),
        lamp_warmth: 0.15,
        curtains: CurtainStyle::Velvet,
        curtain_color: hex(0x2f6e72),
        lace: false,
    };

    /// The 1979 railway buffet (tints from `handson.md` §11.2).
    pub const STATION_RETRO: ThemeStyle = ThemeStyle {
        wall_top: hex(0xd8cfb6),
        wall_bottom: hex(0xbdb194),
        wainscot_top: hex(0x2f5d4a),
        wainscot_bottom: hex(0x22463a),
        trim: hex(0x2f5d4a),
        pattern: WallPattern::Wainscot,
        pattern_color: hex(0x2f5d4a),
        pattern_accent: hex(0x3a5aa8),
        floor: FloorVariant::Planks,
        floor_tint: hex(0x8a6a4a),
        floor_mix: 0.45,
        textile: hex(0x2f5d4a),
        textile_alt: hex(0xa8553e),
        accent: hex(0xd9b04a),
        rug_base: hex(0x2a4a3e),
        rug_border: hex(0xe9e1c8),
        rug_field: hex(0x3a5f4e),
        rug_motif: hex(0xa8553e),
        lamp: LampStyle::Enamel,
        lampshade: hex(0xf3e7c9),
        lamp_warmth: 0.1,
        curtains: CurtainStyle::Blind,
        curtain_color: hex(0xe9e1c8),
        lace: false,
    };

    /// Café Nocturne, 2039.
    pub const NOCTURNE: ThemeStyle = ThemeStyle {
        wall_top: hex(0x6a62b0),
        wall_bottom: hex(0x4c4590),
        wainscot_top: hex(0x2e2a5e),
        wainscot_bottom: hex(0x211d48),
        trim: hex(0x818cf8),
        pattern: WallPattern::HoloLines,
        pattern_color: hex(0x818cf8),
        pattern_accent: hex(0xffc46b),
        floor: FloorVariant::Planks,
        floor_tint: hex(0x5a4a6a),
        floor_mix: 0.3,
        textile: hex(0x5b3fb0),
        textile_alt: hex(0x3f6aa8),
        accent: hex(0xa78bfa),
        rug_base: hex(0x1f1d4a),
        rug_border: hex(0x818cf8),
        rug_field: hex(0x2c2870),
        rug_motif: hex(0xa78bfa),
        lamp: LampStyle::LedRing,
        lampshade: hex(0x2c2f3e),
        lamp_warmth: -0.15,
        curtains: CurtainStyle::None,
        curtain_color: hex(0x4f46e5),
        lace: false,
    };

    /// Winter lights (seasonal): white-gold, red ribbons, spruce green.
    pub const WINTER_LIGHTS: ThemeStyle = ThemeStyle {
        wall_top: hex(0xa79ac8),
        wall_bottom: hex(0x857aaa),
        wainscot_top: hex(0x6a3a44),
        wainscot_bottom: hex(0x4a2632),
        trim: hex(0xf0e2c8),
        pattern: WallPattern::Snowflakes,
        pattern_color: hex(0xf4f0ff),
        pattern_accent: hex(0xc23a4a),
        floor: FloorVariant::Planks,
        floor_tint: hex(0x9a6a4a),
        floor_mix: 0.2,
        textile: hex(0xa8343e),
        textile_alt: hex(0x2f6a4f),
        accent: hex(0xf0e2c8),
        rug_base: hex(0x6a2430),
        rug_border: hex(0xf0e2c8),
        rug_field: hex(0x8a2e3a),
        rug_motif: hex(0x2f6a4f),
        lamp: LampStyle::Ribbon,
        lampshade: hex(0xf0e2c8),
        lamp_warmth: 0.25,
        curtains: CurtainStyle::Velvet,
        curtain_color: hex(0xa8343e),
        lace: true,
    };

    /// Autumn harvest (the `Seasonal` theme in late autumn).
    pub const HARVEST: ThemeStyle = ThemeStyle {
        wall_top: hex(0x9e84a8),
        wall_bottom: hex(0x7e6890),
        wainscot_top: hex(0x6a4232),
        wainscot_bottom: hex(0x4a2c22),
        trim: hex(0xe8c89a),
        pattern: WallPattern::Leaves,
        pattern_color: hex(0xe08a4a),
        pattern_accent: hex(0xc9a24a),
        floor: FloorVariant::Planks,
        floor_tint: hex(0xa86a42),
        floor_mix: 0.3,
        textile: hex(0xb8603e),
        textile_alt: hex(0x8a7a3a),
        accent: hex(0xe0a35a),
        rug_base: hex(0x5a2e2a),
        rug_border: hex(0xe0a35a),
        rug_field: hex(0x7a3e30),
        rug_motif: hex(0xc9a24a),
        lamp: LampStyle::Fringed,
        lampshade: hex(0xd98a4a),
        lamp_warmth: 0.4,
        curtains: CurtainStyle::Velvet,
        curtain_color: hex(0x8a3e2e),
        lace: false,
    };

    /// A stable hash of the style (for the static-layer cache key).
    pub fn key(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut eat = |v: u32| {
            h = (h ^ v as u64).wrapping_mul(0x0100_0000_01b3);
        };
        for col in [
            self.wall_top,
            self.wall_bottom,
            self.wainscot_top,
            self.wainscot_bottom,
            self.trim,
            self.pattern_color,
            self.pattern_accent,
            self.floor_tint,
            self.curtain_color,
        ] {
            eat(u32::from_be_bytes(<[u8; 4]>::from(col)));
        }
        eat(self.pattern as u32);
        eat(self.floor as u32 + 16);
        eat(self.curtains as u32 + 32);
        eat((self.floor_mix * 1000.0) as u32);
        h
    }

    /// Build a style from content tints (`ThemeTints` hex strings, `#RRGGBB`): the
    /// fields not given keep the values of `base`. Unparsable strings are ignored.
    pub fn with_tints(
        base: ThemeStyle,
        wall: Option<&str>,
        wall_trim: Option<&str>,
        floor: Option<&str>,
        textile: Option<&str>,
        lampshade: Option<&str>,
        lamp_warmth: Option<f32>,
    ) -> ThemeStyle {
        let mut s = base;
        if let Some(col) = wall.and_then(parse_hex) {
            s.wall_top = col;
            s.wall_bottom = shade(col, 0.85);
        }
        if let Some(col) = wall_trim.and_then(parse_hex) {
            s.trim = col;
            s.wainscot_top = col;
            s.wainscot_bottom = shade(col, 0.75);
        }
        if let Some(col) = floor.and_then(parse_hex) {
            s.floor_tint = col;
            s.floor_mix = s.floor_mix.max(0.35);
        }
        if let Some(col) = textile.and_then(parse_hex) {
            s.textile = col;
        }
        if let Some(col) = lampshade.and_then(parse_hex) {
            s.lampshade = col;
        }
        if let Some(w) = lamp_warmth {
            s.lamp_warmth = w.clamp(-1.0, 1.0);
        }
        s
    }

    /// A lamp's light colour in this theme.
    pub fn lamp_light(&self, base: Color) -> Color {
        let w = self.lamp_warmth;
        if w >= 0.0 {
            mix(base, c::AMBER_DEEP, w * 0.6)
        } else {
            mix(base, Color::from_hex(0xfff3e0), -w)
        }
    }
}

/// `#RRGGBB` / `RRGGBB` → a colour.
pub fn parse_hex(s: &str) -> Option<Color> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    u32::from_str_radix(s, 16).ok().map(Color::from_hex)
}

thread_local! {
    static DECOR: Cell<ThemeStyle> = const { Cell::new(ThemeStyle::DEFAULT) };
}

/// Set the decor style that walls, floors and furniture use from now on.
pub fn set_decor(style: ThemeStyle) {
    DECOR.with(|d| d.set(style));
}

/// The current decor style.
pub fn decor() -> ThemeStyle {
    DECOR.with(|d| d.get())
}

/// Draw `f` with `style`, then restore the previous style.
pub fn with_decor<R>(style: ThemeStyle, f: impl FnOnce() -> R) -> R {
    let prev = decor();
    set_decor(style);
    let r = f();
    set_decor(prev);
    r
}

// ───────────────────────── wall patterns ─────────────────────────

/// The pattern over the plaster face of a hall wall tile.
/// `x` — the tile's left edge, `y0..y1` — the plaster band (above the wainscot).
pub(super) fn draw_wall_pattern(s: &ThemeStyle, tile: Vec2, x: f32, y0: f32, y1: f32) {
    let (tx, ty) = (tile.x as i32, tile.y as i32);
    let h = y1 - y0;
    match s.pattern {
        WallPattern::Plain => {}
        WallPattern::RoseStripe => {
            // 4 stripes per tile: a wide pale band + a thin cream pinstripe
            let band = TILE / 4.0;
            for i in 0..4 {
                let bx = x + i as f32 * band;
                if i % 2 == 0 {
                    rect_v(
                        bx,
                        y0,
                        band,
                        h,
                        alpha(s.pattern_color, 0.22),
                        alpha(s.pattern_color, 0.12),
                    );
                } else {
                    draw_line(
                        bx + band / 2.0,
                        y0,
                        bx + band / 2.0,
                        y1,
                        1.0,
                        alpha(s.trim, 0.22),
                    );
                }
            }
            // tiny faded roses in the pale bands
            for i in 0..2 {
                let bx = x + (i as f32 * 2.0 + 0.5) * band;
                let n = 3;
                for k in 0..n {
                    let jitter = hash2(tx * 4 + i, ty * 9 + k) * 6.0;
                    let ry = y0 + 9.0 + k as f32 * (h - 14.0) / (n as f32 - 0.5) + jitter;
                    let rose = alpha(s.pattern_accent, 0.42);
                    ellipse(bx - 1.4, ry, 1.6, 1.4, rose);
                    ellipse(bx + 1.4, ry, 1.6, 1.4, rose);
                    ellipse(bx, ry - 1.2, 1.6, 1.4, rose);
                    ellipse(bx, ry, 0.8, 0.8, alpha(s.trim, 0.5));
                    ellipse(
                        bx + 2.6,
                        ry + 2.2,
                        1.6,
                        0.8,
                        alpha(Color::from_hex(0x6a9a6b), 0.35),
                    );
                }
            }
        }
        WallPattern::MosaicBand => {
            // a band of tesserae between two gold lines + arches below it
            let by = y0 + h * 0.18;
            let bh = 9.0;
            rect_v(
                x,
                by,
                TILE,
                bh,
                alpha(s.pattern_accent, 0.55),
                alpha(shade(s.pattern_accent, 0.8), 0.55),
            );
            let cell = TILE / 8.0;
            for i in 0..8 {
                let v = hash2(tx * 8 + i, ty);
                let col = if (i + tx) % 4 == 0 {
                    s.pattern_color
                } else if v > 0.5 {
                    tint_light(s.pattern_accent, 0.25)
                } else {
                    shade(s.pattern_accent, 0.75)
                };
                rrect(
                    x + i as f32 * cell + 1.0,
                    by + 1.5,
                    cell - 2.0,
                    bh - 3.0,
                    1.0,
                    alpha(col, 0.85),
                );
            }
            draw_line(x, by, x + TILE, by, 1.5, alpha(s.pattern_color, 0.9));
            draw_line(
                x,
                by + bh,
                x + TILE,
                by + bh,
                1.5,
                alpha(s.pattern_color, 0.9),
            );
            // Secession arches with a small gold "sun" inside
            for k in 0..2 {
                let cx = x + TILE * (0.25 + 0.5 * k as f32);
                let ay = by + bh + 15.0;
                arc(
                    vec2(cx, ay),
                    TILE * 0.2,
                    12.0,
                    std::f32::consts::PI,
                    std::f32::consts::TAU,
                    1.2,
                    alpha(s.pattern_color, 0.45),
                );
                draw_line(
                    cx - TILE * 0.2,
                    ay,
                    cx - TILE * 0.2,
                    y1 - 4.0,
                    1.0,
                    alpha(s.pattern_color, 0.3),
                );
                glow(cx, ay - 5.0, 4.0, alpha(s.pattern_color, 0.5));
                ellipse(cx, ay - 5.0, 1.6, 1.6, alpha(s.pattern_color, 0.8));
            }
        }
        WallPattern::Wainscot => {
            // a painted dado: bottle green up to ~45 % of the plaster, a cream pinstripe
            let dy = y0 + h * 0.52;
            rect_v(
                x,
                dy,
                TILE,
                y1 - dy,
                alpha(s.pattern_color, 0.92),
                alpha(shade(s.pattern_color, 0.85), 0.92),
            );
            draw_line(x, dy, x + TILE, dy, 2.0, alpha(s.wall_top, 0.9));
            draw_line(x, dy + 3.0, x + TILE, dy + 3.0, 0.8, alpha(s.wall_top, 0.5));
            // an enamel plaque on some tiles
            if hash2(tx * 13 + 7, ty * 3) > 0.72 {
                let (pw, ph) = (26.0, 14.0);
                let px = x + (TILE - pw) / 2.0;
                let py = y0 + 8.0;
                rrect(
                    px - 1.0,
                    py - 1.0,
                    pw + 2.0,
                    ph + 2.0,
                    3.5,
                    alpha(BLACK, 0.18),
                );
                rrect_v(
                    px,
                    py,
                    pw,
                    ph,
                    3.0,
                    s.pattern_accent,
                    shade(s.pattern_accent, 0.8),
                );
                rrect(
                    px + 2.0,
                    py + 2.0,
                    pw - 4.0,
                    ph - 4.0,
                    2.0,
                    alpha(s.wall_top, 0.25),
                );
                // a tiny cup pictogram
                rrect(
                    px + pw / 2.0 - 4.0,
                    py + 4.0,
                    8.0,
                    6.0,
                    1.5,
                    tint_light(s.wall_top, 0.2),
                );
                draw_circle_lines(
                    px + pw / 2.0 + 5.0,
                    py + 6.5,
                    1.6,
                    0.8,
                    tint_light(s.wall_top, 0.2),
                );
            }
        }
        WallPattern::HoloLines => {
            let ly = y0 + 10.0;
            draw_line(x, ly, x + TILE, ly, 2.4, alpha(s.pattern_color, 0.25));
            draw_line(x, ly, x + TILE, ly, 0.9, alpha(s.pattern_color, 0.8));
            // a short dashed "data" line
            let mut dx = x + hash2(tx, ty) * 10.0;
            while dx < x + TILE - 6.0 {
                draw_line(
                    dx,
                    ly + 5.0,
                    dx + 4.0,
                    ly + 5.0,
                    0.8,
                    alpha(s.pattern_color, 0.4),
                );
                dx += 9.0;
            }
            // warm LED strip just above the moulding
            let sy = y1 - 6.0;
            rect_v(
                x,
                sy - 8.0,
                TILE,
                8.0,
                alpha(s.pattern_accent, 0.0),
                alpha(s.pattern_accent, 0.25),
            );
            draw_line(x, sy, x + TILE, sy, 1.6, alpha(s.pattern_accent, 0.9));
        }
        WallPattern::Snowflakes => {
            for k in 0..3 {
                let fx = x + 8.0 + hash2(tx * 3 + k, ty + 5) * (TILE - 16.0);
                let fy = y0 + 14.0 + hash2(tx + 11, ty * 3 + k) * (h - 24.0);
                let r = 3.0 + hash2(tx * 7, k) * 2.5;
                snowflake(vec2(fx, fy), r, alpha(s.pattern_color, 0.35), 0.9);
            }
            // red ribbon swags along the top
            let sy = y0 + 2.0;
            for k in 0..2 {
                let ax = x + k as f32 * TILE / 2.0;
                arc(
                    vec2(ax + TILE / 4.0, sy),
                    TILE / 4.0,
                    6.0,
                    0.0,
                    std::f32::consts::PI,
                    2.0,
                    alpha(s.pattern_accent, 0.75),
                );
                ellipse(ax, sy + 1.0, 2.2, 2.2, alpha(s.pattern_accent, 0.9));
            }
        }
        WallPattern::Leaves => {
            for k in 0..3 {
                let fx = x + 8.0 + hash2(tx * 5 + k, ty + 2) * (TILE - 16.0);
                let fy = y0 + 12.0 + hash2(tx + 3, ty * 5 + k) * (h - 22.0);
                let ang = hash2(tx, k * 3) * std::f32::consts::TAU;
                let col = if k % 2 == 0 {
                    s.pattern_color
                } else {
                    s.pattern_accent
                };
                leaf_shape(vec2(fx, fy), ang, 6.0, alpha(col, 0.32));
            }
        }
    }
}

/// A six-armed snowflake.
pub(super) fn snowflake(c0: Vec2, r: f32, col: Color, thick: f32) {
    for i in 0..3 {
        let a = i as f32 * std::f32::consts::PI / 3.0 + 0.3;
        let d = vec2(a.cos(), a.sin()) * r;
        draw_line(c0.x - d.x, c0.y - d.y, c0.x + d.x, c0.y + d.y, thick, col);
        for sgn in [-1.0_f32, 1.0] {
            let tip = c0 + d * sgn * 0.6;
            let n = vec2(-d.y, d.x) * 0.25;
            draw_line(
                tip.x,
                tip.y,
                tip.x + n.x + d.x * sgn * 0.2,
                tip.y + n.y + d.y * sgn * 0.2,
                thick * 0.8,
                col,
            );
            draw_line(
                tip.x,
                tip.y,
                tip.x - n.x + d.x * sgn * 0.2,
                tip.y - n.y + d.y * sgn * 0.2,
                thick * 0.8,
                col,
            );
        }
    }
}

/// A small pointed leaf (a lens) centred at `c0`.
pub(super) fn leaf_shape(c0: Vec2, ang: f32, len: f32, col: Color) {
    let dir = vec2(ang.cos(), ang.sin());
    let n = vec2(-dir.y, dir.x);
    let mut pts = [Vec2::ZERO; 8];
    for (i, p) in pts.iter_mut().enumerate().take(4) {
        let t = i as f32 / 3.0;
        *p = c0 - dir * len * 0.5
            + dir * len * t
            + n * len * 0.32 * (t * std::f32::consts::PI).sin();
    }
    for i in 0..4 {
        let t = 1.0 - i as f32 / 3.0;
        pts[4 + i] = c0 - dir * len * 0.5 + dir * len * t
            - n * len * 0.32 * (t * std::f32::consts::PI).sin();
    }
    poly(&pts, col);
}

/// Curtains over the window pane `g` (drawn by `draw_window` with the current decor).
pub(super) fn draw_curtains(s: &ThemeStyle, g: Rect, tile: Vec2) {
    let col = s.curtain_color;
    match s.curtains {
        CurtainStyle::None => {}
        CurtainStyle::Lace => {
            // a lace café valance on the top fifth, a scalloped edge with little holes
            let h = g.h * 0.24;
            rect_v(
                g.x - 2.0,
                g.y - 2.0,
                g.w + 4.0,
                h,
                alpha(col, 0.78),
                alpha(col, 0.62),
            );
            let n = 4;
            let sw = (g.w + 4.0) / n as f32;
            for i in 0..n {
                let sx = g.x - 2.0 + (i as f32 + 0.5) * sw;
                ellipse(sx, g.y - 2.0 + h, sw / 2.0, 3.2, alpha(col, 0.62));
                ellipse(
                    sx,
                    g.y - 2.0 + h * 0.55,
                    1.4,
                    1.4,
                    alpha(Color::from_hex(0x2a2250), 0.45),
                );
            }
            for i in 0..(n * 2) {
                let hx = g.x + (i as f32 + 0.5) * (g.w / (n * 2) as f32);
                ellipse(
                    hx,
                    g.y + 2.0,
                    0.9,
                    0.9,
                    alpha(Color::from_hex(0x2a2250), 0.35),
                );
            }
        }
        CurtainStyle::Velvet => {
            let h = 9.0;
            rect_v(
                g.x - 4.0,
                g.y - 4.0,
                g.w + 8.0,
                h,
                tint_light(col, 0.15),
                shade(col, 0.75),
            );
            // folds
            for i in 0..4 {
                let fx = g.x - 4.0 + (i as f32 + 0.5) * (g.w + 8.0) / 4.0;
                draw_line(
                    fx,
                    g.y - 4.0,
                    fx,
                    g.y - 4.0 + h,
                    1.0,
                    alpha(shade(col, 0.6), 0.6),
                );
            }
            // a golden fringe
            let mut fx = g.x - 3.0;
            while fx < g.x + g.w + 3.0 {
                draw_line(
                    fx,
                    g.y - 4.0 + h,
                    fx,
                    g.y - 4.0 + h + 3.0,
                    0.9,
                    alpha(Color::from_hex(0xe0b86a), 0.85),
                );
                fx += 2.5;
            }
            // tie-backs at the outer edges of window runs (just a hint on every pane)
            let side = if (tile.x as i32) % 2 == 0 {
                g.x - 2.0
            } else {
                g.x + g.w - 3.0
            };
            rrect_v(
                side,
                g.y + 4.0,
                5.0,
                g.h * 0.55,
                2.5,
                alpha(col, 0.9),
                alpha(shade(col, 0.7), 0.9),
            );
        }
        CurtainStyle::Blind => {
            let h = g.h * 0.3;
            rect_v(
                g.x,
                g.y,
                g.w,
                h,
                alpha(col, 0.9),
                alpha(shade(col, 0.9), 0.9),
            );
            for k in 1..4 {
                let ly = g.y + h * k as f32 / 4.0;
                draw_line(g.x, ly, g.x + g.w, ly, 0.6, alpha(shade(col, 0.7), 0.5));
            }
            rrect(
                g.x - 1.0,
                g.y + h - 1.0,
                g.w + 2.0,
                3.0,
                1.0,
                shade(col, 0.6),
            );
            draw_line(
                g.x + g.w / 2.0,
                g.y + h + 2.0,
                g.x + g.w / 2.0,
                g.y + h + 7.0,
                0.8,
                shade(col, 0.5),
            );
            ellipse(g.x + g.w / 2.0, g.y + h + 8.0, 1.6, 1.6, shade(col, 0.55));
        }
    }
}

/// A lace doily under a table's candle (drawn by tables when `lace` is on).
pub(super) fn draw_doily(cx: f32, cy: f32, rx: f32, ry: f32) {
    let col = alpha(Color::from_hex(0xcbbfae), 0.7);
    let n = 14;
    for i in 0..n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        ellipse(
            cx + a.cos() * rx,
            cy + a.sin() * ry,
            rx * 0.2,
            ry * 0.2,
            col,
        );
    }
    ellipse(cx, cy, rx, ry, col);
    draw_ellipse_lines(
        cx,
        cy,
        rx * 0.65,
        ry * 0.65,
        0.0,
        0.8,
        alpha(shade(col, 0.8), 0.7),
    );
    for i in 0..8 {
        let a = i as f32 / 8.0 * std::f32::consts::TAU;
        ellipse(
            cx + a.cos() * rx * 0.82,
            cy + a.sin() * ry * 0.82,
            0.8,
            0.6,
            alpha(Color::from_hex(0x8a6a5a), 0.45),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip() {
        for t in DecorTheme::ALL {
            assert_eq!(DecorTheme::from_id(t.id()), Some(t));
        }
        assert_eq!(DecorTheme::from_id("nope"), None);
    }

    #[test]
    fn theme_keys_differ() {
        let keys: Vec<u64> = DecorTheme::ALL
            .iter()
            .map(|t| t.style(Season::LateAutumn).key())
            .chain([ThemeStyle::DEFAULT.key()])
            .collect();
        for i in 0..keys.len() {
            for j in i + 1..keys.len() {
                assert_ne!(keys[i], keys[j], "{i} vs {j}");
            }
        }
        // the seasonal theme follows the season
        assert_ne!(
            DecorTheme::Seasonal.style(Season::LateAutumn),
            DecorTheme::Seasonal.style(Season::FirstSnow)
        );
    }

    #[test]
    fn decor_scope_restores() {
        set_decor(ThemeStyle::DEFAULT);
        with_decor(ThemeStyle::GRANDMA, || {
            assert_eq!(decor(), ThemeStyle::GRANDMA);
        });
        assert_eq!(decor(), ThemeStyle::DEFAULT);
    }

    #[test]
    fn tints_parse() {
        assert_eq!(parse_hex("#E9E1C8"), Some(Color::from_hex(0xe9e1c8)));
        assert_eq!(parse_hex("zz"), None);
        let s = ThemeStyle::with_tints(
            ThemeStyle::DEFAULT,
            Some("#E9E1C8"),
            None,
            None,
            Some("bad"),
            None,
            Some(3.0),
        );
        assert_eq!(s.wall_top, Color::from_hex(0xe9e1c8));
        assert_eq!(s.textile, ThemeStyle::DEFAULT.textile);
        assert_eq!(s.lamp_warmth, 1.0);
    }
}
