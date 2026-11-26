//! Design tokens of the Shiftella glass UI (purple / indigo night palette).
//!
//! Colours used for text and icons are **opaque** on purpose: strokes overlap at joints
//! and translucent ink would show seams. Translucent tokens are for fills and edges.

use macroquad::prelude::Color;

use super::draw::Shadow;
use super::text::{TextStyle, Weight};

const fn hex(v: u32) -> Color {
    Color::from_hex(v)
}

const fn rgba(v: u32, a: f32) -> Color {
    let c = Color::from_hex(v);
    Color::new(c.r, c.g, c.b, a)
}

/// Spacing scale (pt): 2, 4, 6, 8, 12, 16, 24.
pub mod space {
    pub const XXS: f32 = 2.0;
    pub const XS: f32 = 4.0;
    pub const S: f32 = 6.0;
    pub const M: f32 = 8.0;
    pub const L: f32 = 12.0;
    pub const XL: f32 = 16.0;
    pub const XXL: f32 = 24.0;
    /// Screen edge → HUD block.
    pub const EDGE: f32 = 12.0;
}

/// Corner radii (pt).
pub mod radius {
    pub const XS: f32 = 4.0;
    pub const S: f32 = 6.0;
    pub const M: f32 = 8.0;
    pub const L: f32 = 10.0;
    pub const XL: f32 = 12.0;
    pub const XXL: f32 = 14.0;
}

/// Standard control heights (pt).
pub mod size {
    pub const BUTTON: f32 = 28.0;
    pub const BUTTON_SM: f32 = 24.0;
    pub const BUTTON_LG: f32 = 32.0;
    pub const INPUT: f32 = 32.0;
    pub const ROW: f32 = 44.0;
    pub const ROW_SM: f32 = 34.0;
    pub const ICON: f32 = 18.0;
    pub const CHIP: f32 = 22.0;
}

/// All colours and shadows. `Theme::SHIFTELLA` is the only theme for now.
#[derive(Clone, Debug)]
pub struct Theme {
    // canvas
    pub canvas: [Color; 3],
    pub violet_night: [Color; 2],
    pub ink: Color,
    // glass
    pub glass_top: Color,
    pub glass_bottom: Color,
    pub glass_solid_top: Color,
    pub glass_solid_bottom: Color,
    pub edge_top: Color,
    pub edge_bottom: Color,
    pub sheen: Color,
    pub divider: Color,
    pub raised: Color,
    pub raised_hover: Color,
    pub pressed: Color,
    pub well: Color,
    // accents
    pub indigo: Color,
    pub indigo_600: Color,
    pub indigo_400: Color,
    pub indigo_300: Color,
    pub violet: Color,
    pub violet_400: Color,
    pub amber: Color,
    pub amber_deep: Color,
    pub emerald: Color,
    pub emerald_400: Color,
    pub red: Color,
    pub red_400: Color,
    // text
    pub text: Color,
    pub text_dim: Color,
    pub text_disabled: Color,
    pub text_on_accent: Color,
    // needs
    pub need_energy: Color,
    pub need_mood: Color,
    pub need_hunger: Color,
    // shadows
    pub shadow_sm: Shadow,
    pub shadow_md: Shadow,
    pub shadow_lg: Shadow,
    pub shadow_ambient: Shadow,
    /// Settings → Text size (1.0 / 1.15 / 1.3): multiplies every text style below, never
    /// the panels or the UI scale (`Ui::set_text_scale`).
    pub text_scale: f32,
}

impl Theme {
    pub const SHIFTELLA: Theme = Theme {
        canvas: [hex(0x0D0C17), hex(0x121022), hex(0x1A1730)],
        violet_night: [hex(0x120C1F), hex(0x201634)],
        ink: hex(0x1B1A2E),

        glass_top: rgba(0x2A2850, 0.74),
        glass_bottom: rgba(0x17162C, 0.84),
        glass_solid_top: rgba(0x26244A, 0.94),
        glass_solid_bottom: rgba(0x15142A, 0.96),
        edge_top: rgba(0xFFFFFF, 0.24),
        edge_bottom: rgba(0xFFFFFF, 0.05),
        sheen: rgba(0xC7C9FF, 0.075),
        divider: rgba(0xFFFFFF, 0.075),
        raised: rgba(0xFFFFFF, 0.065),
        raised_hover: rgba(0xFFFFFF, 0.11),
        pressed: rgba(0x000000, 0.22),
        well: rgba(0x07061A, 0.55),

        indigo: hex(0x6366F1),
        indigo_600: hex(0x4F46E5),
        indigo_400: hex(0x818CF8),
        indigo_300: hex(0xA5B4FC),
        violet: hex(0x7C3AED),
        violet_400: hex(0xA78BFA),
        amber: hex(0xFFC46B),
        amber_deep: hex(0xF99C4E),
        emerald: hex(0x10B981),
        emerald_400: hex(0x34D399),
        red: hex(0xEF4444),
        red_400: hex(0xF87171),

        text: hex(0xEEEDF7),
        text_dim: hex(0xA9A7C9),
        text_disabled: hex(0x67658C),
        text_on_accent: hex(0xFFFFFF),

        need_energy: hex(0xFFC46B),
        need_mood: hex(0xA78BFA),
        need_hunger: hex(0x34D399),

        shadow_sm: Shadow::new(0.0, 2.0, 6.0, 0.0, rgba(0x000000, 0.30)),
        shadow_md: Shadow::new(0.0, 6.0, 18.0, 0.0, rgba(0x05040C, 0.42)),
        shadow_lg: Shadow::new(0.0, 14.0, 36.0, 2.0, rgba(0x05040C, 0.48)),
        shadow_ambient: Shadow::new(0.0, 1.0, 3.0, 0.0, rgba(0x000000, 0.35)),
        text_scale: 1.0,
    };

    // --- text styles (pt @ ui_scale 1) ------------------------------------

    /// 52/Bold — menu logo.
    pub fn display(&self) -> TextStyle {
        // the logo keeps its size (it is art, not reading text)
        TextStyle::new(44.0, Weight::Bold, self.text).tracking(-0.6)
    }
    /// 22/Bold tabular — HUD clock.
    pub fn clock(&self) -> TextStyle {
        TextStyle::new(22.0, Weight::Bold, self.text)
            .tabular()
            .scaled(self.text_scale)
    }
    /// 18/Bold — screen title.
    pub fn title(&self) -> TextStyle {
        TextStyle::new(18.0, Weight::Bold, self.text)
            .tracking(-0.2)
            .scaled(self.text_scale)
    }
    /// 17/Bold — panel header.
    pub fn h1(&self) -> TextStyle {
        TextStyle::new(16.0, Weight::Bold, self.text)
            .tracking(-0.15)
            .scaled(self.text_scale)
    }
    /// 15/SemiBold — card titles, names in cards.
    pub fn h2(&self) -> TextStyle {
        TextStyle::new(14.0, Weight::SemiBold, self.text).scaled(self.text_scale)
    }
    /// 13/Regular — body text.
    pub fn body(&self) -> TextStyle {
        TextStyle::new(13.0, Weight::Regular, self.text).scaled(self.text_scale)
    }
    /// 13/Medium.
    pub fn body_medium(&self) -> TextStyle {
        TextStyle::new(13.0, Weight::Medium, self.text).scaled(self.text_scale)
    }
    /// 13/SemiBold — buttons, toast titles, list names.
    pub fn strong(&self) -> TextStyle {
        TextStyle::new(13.0, Weight::SemiBold, self.text).scaled(self.text_scale)
    }
    /// 12/Regular dim — captions.
    pub fn small(&self) -> TextStyle {
        TextStyle::new(12.0, Weight::Regular, self.text_dim).scaled(self.text_scale)
    }
    /// 12/Medium — chips.
    pub fn small_medium(&self) -> TextStyle {
        TextStyle::new(12.0, Weight::Medium, self.text).scaled(self.text_scale)
    }
    /// 11/Bold dim tracked — SECTION HEADERS (uppercase the string yourself).
    pub fn micro(&self) -> TextStyle {
        TextStyle::new(11.0, Weight::Bold, self.text_disabled)
            .tracking(0.6)
            .scaled(self.text_scale)
    }
    /// 11/SemiBold — keycaps, axis labels.
    pub fn key(&self) -> TextStyle {
        TextStyle::new(11.0, Weight::SemiBold, self.text_dim).scaled(self.text_scale)
    }
    /// Numbers in tables / money: 13 SemiBold tabular.
    pub fn number(&self) -> TextStyle {
        TextStyle::new(13.0, Weight::SemiBold, self.text)
            .tabular()
            .scaled(self.text_scale)
    }

    /// Avatar palette (deterministic by name, see `avatar_color`).
    pub const AVATARS: [Color; 8] = [
        hex(0x6366F1),
        hex(0x8B5CF6),
        hex(0xEC8FB5),
        hex(0xF59E6B),
        hex(0x10B981),
        hex(0x38BDF8),
        hex(0xE8B04B),
        hex(0xA78BFA),
    ];
}

impl Default for Theme {
    fn default() -> Self {
        Theme::SHIFTELLA
    }
}

/// Semantic tone for chips, toasts, banners.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tone {
    Neutral,
    Accent,
    Violet,
    Info,
    Ok,
    Warn,
    Danger,
}

impl Theme {
    /// Foreground colour of a tone (text/icon).
    pub fn tone_fg(&self, t: Tone) -> Color {
        match t {
            Tone::Neutral => self.text_dim,
            Tone::Accent => self.indigo_300,
            Tone::Violet => self.violet_400,
            Tone::Info => self.indigo_400,
            Tone::Ok => self.emerald_400,
            Tone::Warn => self.amber,
            Tone::Danger => self.red_400,
        }
    }
    /// Solid accent colour of a tone (bars, stripes).
    pub fn tone_solid(&self, t: Tone) -> Color {
        match t {
            Tone::Neutral => self.text_dim,
            Tone::Accent => self.indigo,
            Tone::Violet => self.violet,
            Tone::Info => self.indigo_400,
            Tone::Ok => self.emerald,
            Tone::Warn => self.amber,
            Tone::Danger => self.red,
        }
    }
    /// Tinted background of a tone (chips: ~16 %).
    pub fn tone_bg(&self, t: Tone) -> Color {
        let c = match t {
            Tone::Neutral => return Color::new(1.0, 1.0, 1.0, 0.08),
            _ => self.tone_solid(t),
        };
        Color::new(c.r, c.g, c.b, 0.17)
    }
}
