//! Season phases (`assets/data/seasons.ron`), decor themes and coziness
//! (`assets/data/decor.ron`), handson.md §11.1–11.3.

use serde::{Deserialize, Serialize};

use super::recipes::Taste;
use super::records::HallMood;
use super::{Tier, Unlock};
use crate::content::defs::Season;

// ---------------------------------------------------------------------------------------------
// Seasons
// ---------------------------------------------------------------------------------------------

/// What the render shows in a phase (all procedural).
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SeasonVisuals {
    /// Wet leaves on the pavement and doorstep.
    pub leaves: bool,
    /// Frost in the window corners, 0–1.
    pub frost: f32,
    /// Breath puffs for people outside.
    pub breath: bool,
    /// Garlands are *allowed* (they render once flag `garlands_up` is set).
    pub garlands: bool,
    /// Advent wreath on the counter.
    pub wreath: bool,
    /// Snowfall, snow caps on the sign and the street edge.
    pub snow: bool,
    /// Pink petals in light rain (sandbox spring).
    pub petals: bool,
    /// A star on the holo sign.
    pub sign_star: bool,
    /// Extra steam from cups, 0–1.
    pub steam: f32,
}

/// A season phase (`season.<id>.name`, `season.<id>.toast` on `SeasonPhaseChanged`).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PhaseDef {
    pub id: String,
    /// Inclusive story nights (`None` = sandbox only).
    #[serde(default)]
    pub story_nights: Option<(u32, u32)>,
    #[serde(default)]
    pub visuals: SeasonVisuals,
    /// Wish pick weight × mul for wishes wanting the tag.
    #[serde(default)]
    pub wish_mul: Vec<(Taste, f32)>,
    /// Latte patterns unlocked while in (or after reaching) this phase.
    #[serde(default)]
    pub latte: Vec<String>,
    /// Seasonal moments that become eligible in this phase.
    #[serde(default)]
    pub moments: Vec<String>,
    /// Moment weight × mul.
    #[serde(default)]
    pub moment_weight: Vec<(String, f32)>,
    /// Specials-board suggestions (recipe ids, SHOULD §6.8).
    #[serde(default)]
    pub specials: Vec<String>,
}

impl PhaseDef {
    pub fn name_key(&self) -> String {
        format!("season.{}.name", self.id)
    }
    pub fn toast_key(&self) -> String {
        format!("season.{}.toast", self.id)
    }
}

/// Sandbox: which phase a `town.season` (30 nights each) shows on nights `nights` of it.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SandboxPhase {
    pub season: Season,
    /// Inclusive night range within the 30-night season.
    pub nights: (u32, u32),
    pub phase: String,
}

/// `assets/data/seasons.ron`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SeasonsFile {
    pub phases: Vec<PhaseDef>,
    pub sandbox: Vec<SandboxPhase>,
}

impl SeasonsFile {
    pub fn phase(&self, id: &str) -> Option<&PhaseDef> {
        self.phases.iter().find(|p| p.id == id)
    }

    /// The story phase of a night.
    pub fn story_phase(&self, night: u32) -> Option<&PhaseDef> {
        self.phases.iter().find(|p| {
            p.story_nights
                .is_some_and(|(a, b)| (a..=b).contains(&night))
        })
    }

    /// The sandbox phase for night `night_of_season` (1..=30) of `season`.
    pub fn sandbox_phase(&self, season: Season, night_of_season: u32) -> Option<&PhaseDef> {
        self.sandbox
            .iter()
            .find(|s| s.season == season && (s.nights.0..=s.nights.1).contains(&night_of_season))
            .and_then(|s| self.phase(&s.phase))
    }
}

// ---------------------------------------------------------------------------------------------
// Decor themes
// ---------------------------------------------------------------------------------------------

/// Procedural wall pattern of a theme (render).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum WallPattern {
    /// Faded rose-stripe wallpaper.
    RoseStripe,
    /// Prague Secession mosaic band.
    SecessionMosaic,
    /// Cream-over-green railway wainscot.
    Wainscot,
    /// Thin indigo holo lines.
    HoloLines,
    /// Plain wall with a garland swag.
    GarlandSwag,
}

/// Colours of a theme (`#RRGGBB`); `lamp_warmth` shifts lamp light (−1 cool … +1 warm).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeTints {
    pub wall: String,
    pub wall_trim: String,
    pub floor: String,
    pub textile: String,
    pub lampshade: String,
    pub lamp_warmth: f32,
}

/// A small extra of a theme.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum ThemePerk {
    /// + beauty per lamp in the hall.
    BeautyPerLamp(f32),
    /// These regulars get + trust once per night when they come in.
    RegularTrustOnEnter { regulars: Vec<String>, add: f32 },
    /// + satisfaction for these archetypes.
    ArchetypeSatisfaction { archetypes: Vec<String>, add: f32 },
    /// While *another* theme is active, `regular` may say `key` (ambient) up to `until_night`.
    MissedBy {
        regular: String,
        key: String,
        until_night: u32,
    },
}

/// A decor theme for the hall (`theme.<id>.name/.desc`).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeDef {
    pub id: String,
    /// The theme the café starts with (exactly one).
    #[serde(default)]
    pub default: bool,
    #[serde(default)]
    pub unlock: Unlock,
    /// Sandbox: unlocked from this night at the latest.
    #[serde(default)]
    pub sandbox_night: Option<u32>,
    /// CZK, paid once in Build; 0 = free once unlocked.
    pub price: u32,
    /// Theme coziness (0–10) for the coziness `theme` component.
    pub coziness: f32,
    /// Coziness overrides while in these season phases.
    #[serde(default)]
    pub coziness_in: Vec<(String, f32)>,
    pub wall_pattern: WallPattern,
    pub tints: ThemeTints,
    /// Furniture tag of the matching accent pieces (`theme:<id>`).
    #[serde(default)]
    pub accent_tag: Option<String>,
    #[serde(default)]
    pub perks: Vec<ThemePerk>,
    #[serde(default)]
    pub tier: Tier,
}

impl ThemeDef {
    pub fn name_key(&self) -> String {
        format!("theme.{}.name", self.id)
    }
    pub fn desc_key(&self) -> String {
        format!("theme.{}.desc", self.id)
    }
    /// Theme coziness in a season phase.
    pub fn coziness_at(&self, phase: &str) -> f32 {
        self.coziness_in
            .iter()
            .find(|(p, _)| p == phase)
            .map(|(_, c)| *c)
            .unwrap_or(self.coziness)
    }
}

// ---------------------------------------------------------------------------------------------
// Coziness
// ---------------------------------------------------------------------------------------------

/// A part of the coziness value (`atmo.component.<id>`, tip `atmo.tip.<id>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum CozyComponent {
    Light,
    Beauty,
    Comfort,
    Clean,
    Music,
    Theme,
    Touches,
}

impl CozyComponent {
    pub const ALL: [CozyComponent; 7] = [
        CozyComponent::Light,
        CozyComponent::Beauty,
        CozyComponent::Comfort,
        CozyComponent::Clean,
        CozyComponent::Music,
        CozyComponent::Theme,
        CozyComponent::Touches,
    ];

    pub fn id(self) -> &'static str {
        match self {
            CozyComponent::Light => "light",
            CozyComponent::Beauty => "beauty",
            CozyComponent::Comfort => "comfort",
            CozyComponent::Clean => "clean",
            CozyComponent::Music => "music",
            CozyComponent::Theme => "theme",
            CozyComponent::Touches => "touches",
        }
    }
    pub fn name_key(self) -> String {
        format!("atmo.component.{}", self.id())
    }
    pub fn tip_key(self) -> String {
        format!("atmo.tip.{}", self.id())
    }
}

/// A little touch that adds coziness points.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum Touch {
    /// Cat level ≥ n (`cat_affection`).
    CatLevel(u32),
    /// Flag `garlands_up` while the phase allows garlands.
    GarlandsInSeason,
    /// ≥ n plants in the hall (furniture with sprite plant_*).
    Plants(u32),
    /// The napkin wall is placed.
    Furniture(String),
}

/// Music component values (0–10).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MusicPoints {
    pub off: f32,
    pub house: f32,
    pub record: f32,
}

/// `value × (base + coz / div)` style multipliers.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CozyCurve {
    pub base: f32,
    pub div: f32,
}

impl CozyCurve {
    pub fn at(self, coziness: f32) -> f32 {
        self.base + coziness / self.div
    }
}

/// How coziness feeds back (replaces the separate beauty/dirt/music/comfort terms).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CozyEffects {
    /// Satisfaction at serve += `(coz − center) × per`.
    pub satisfaction_center: f32,
    pub satisfaction_per: f32,
    pub patience: CozyCurve,
    pub stay: CozyCurve,
    pub tips: CozyCurve,
}

/// The coziness formula (§11.3).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CozinessDef {
    /// Weight of each component (they sum to 100).
    pub weights: Vec<(CozyComponent, f32)>,
    /// Light: window light on adds this many points.
    pub window_light_bonus: f32,
    /// Beauty: Σ hall beauty / (hall tiles × this), capped at 1.
    pub beauty_per_tile: f32,
    /// Comfort: mean seat comfort / this.
    pub comfort_div: f32,
    /// Clean: − this per table with dishes.
    pub dishes_penalty: f32,
    pub music: MusicPoints,
    /// Theme: (theme coziness + accent bonus) / this.
    pub theme_div: f32,
    pub accent_bonus: f32,
    pub accent_max: f32,
    pub touches: Vec<(Touch, f32)>,
    /// Hall mood adds points.
    pub mood_bonus: Vec<(HallMood, f32)>,
    pub effects: CozyEffects,
    /// Meter level thresholds (`atmo.level.<n>`, n = 1..=len+1): Bare, Plain, Pleasant, Cosy, Like home.
    pub levels: Vec<f32>,
    /// Recompute interval, game minutes.
    pub every_minutes: u32,
}

impl CozinessDef {
    /// Meter level 1..=levels.len()+1 for a coziness value.
    pub fn level(&self, coziness: f32) -> u32 {
        1 + self.levels.iter().filter(|t| coziness >= **t).count() as u32
    }

    pub fn weight(&self, c: CozyComponent) -> f32 {
        self.weights
            .iter()
            .find(|(x, _)| *x == c)
            .map(|(_, w)| *w)
            .unwrap_or(0.0)
    }
}

/// `assets/data/decor.ron`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecorFile {
    pub themes: Vec<ThemeDef>,
    pub coziness: CozinessDef,
}

impl DecorFile {
    pub fn theme(&self, id: &str) -> Option<&ThemeDef> {
        self.themes.iter().find(|t| t.id == id)
    }
    pub fn default_theme(&self) -> Option<&ThemeDef> {
        self.themes.iter().find(|t| t.default)
    }
}

