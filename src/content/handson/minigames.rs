//! Mini-game tuning per kind and variant, quality tiers and latte patterns
//! (`assets/data/minigames.ron`, handson.md §2). Guide polylines live in `src/minigame`.

use serde::{Deserialize, Serialize};

use super::Tier;
use super::logic::Cond;
use super::recipes::MiniGameKind;

/// Tea colour families understood by `src/minigame` (`TeaKind::from_id`).
pub const TEA_KINDS: &[&str] = &[
    "black", "green", "herbal", "fruit", "linden", "mountain", "uzvar",
];

/// Plate dish sets understood by `src/minigame` (`PlateSet::from_id`).
pub const PLATE_DISHES: &[&str] = &[
    "medovnik",
    "soup",
    "topinka",
    "garlic_soup",
    "borscht",
    "chlebicek",
    "kolac",
    "vera_kolac",
    "varenyky",
    "pirohy",
    "buchty",
];

/// Shared rules for every mini-game (§2.0).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommonTuning {
    /// Quality when skipped (Tab).
    pub skip_quality: f32,
    /// Relaxed setting: tolerance windows × this.
    pub relaxed_tolerance_mul: f32,
    /// Relaxed setting: quality never below this.
    pub relaxed_floor: f32,
    /// Overall duration bounds in seconds.
    pub seconds: (f32, f32),
    /// Quality toast after a game, seconds.
    pub toast_seconds: f32,
}

/// Quality labels (never negative): `minigame.quality.<tier>`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QualityTiers {
    pub beautiful: f32,
    pub lovely: f32,
    pub good: f32,
}

impl QualityTiers {
    /// `beautiful` / `lovely` / `good` / `homely`.
    pub fn label(&self, q: f32) -> &'static str {
        if q >= self.beautiful {
            "beautiful"
        } else if q >= self.lovely {
            "lovely"
        } else if q >= self.good {
            "good"
        } else {
            "homely"
        }
    }

    pub const LABELS: [&'static str; 4] = ["beautiful", "lovely", "good", "homely"];
}

/// A timed tap inside a game (Raf's vanilla drop, Lori's chili pinch, a cocoa dust tap).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TapCue {
    pub id: String,
    /// ± seconds around the cue.
    pub window_s: f32,
    /// Share of the quality it is worth.
    pub weight: f32,
}

/// MilkFroth: `q = t_weight × clamp(1 − |T − t_target| / tol) + (1 − t_weight) × clamp(1 − |foam − foam| / foam_tol)`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MilkFrothTuning {
    pub variant: String,
    pub t_target: f32,
    pub tol: f32,
    pub foam: f32,
    pub foam_tol: f32,
    pub t_weight: f32,
    #[serde(default)]
    pub tap: Option<TapCue>,
    /// Cosmetic swirl on first-snow nights.
    #[serde(default)]
    pub seasonal_dust: bool,
    pub seconds: (f32, f32),
}

/// One LatteArt variant (the pattern is chosen on the order card / by the wish).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LatteVariant {
    pub variant: String,
    /// Fixed pattern (`moon` for the moonlight latte); `None` = the player's pick.
    #[serde(default)]
    pub pattern: Option<String>,
}

/// LatteArt: `q = coverage_weight × coverage + (1 − coverage_weight) × (1 − stray)`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LatteArtTuning {
    pub samples: u32,
    pub cover_px: f32,
    pub stray_px: f32,
    pub coverage_weight: f32,
    /// Keyboard mode: ± seconds per wobble tap.
    pub wobble_window_s: f32,
    pub seconds: (f32, f32),
    pub variants: Vec<LatteVariant>,
}

/// PourOver: `q = w.0 × rhythm + w.1 × centred + w.2 × bloom` (variants change the steps).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PourOverTuning {
    pub variant: String,
    /// Metronome (48 bpm = 0.8 Hz, in time with the rain).
    pub bpm: f32,
    /// Phase 1 pour, then "let it bloom" for `bloom_s` (0 = no bloom step).
    pub bloom_hold_s: f32,
    pub bloom_s: f32,
    /// Angular speed band, revolutions per second.
    pub rev_per_s: (f32, f32),
    /// Keyboard: ± seconds for an on-beat tap.
    pub beat_window_s: f32,
    pub weights: (f32, f32, f32),
    /// Turek: stir clicks, then a stillness wait.
    #[serde(default)]
    pub stirs: u32,
    #[serde(default)]
    pub settle_s: f32,
    /// Espresso tonic: a slow pour over ice (fast pours fizz up).
    #[serde(default)]
    pub slow_pour: bool,
    /// Night train: a final cocoa dust tap.
    #[serde(default)]
    pub dust_tap: bool,
    pub seconds: (f32, f32),
}

/// TeaSteep: `q = clamp(1 − |strength − target| / tol) × addons` on a 1–5 strength scale.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TeaSteepTuning {
    pub variant: String,
    pub target: f32,
    pub tol: f32,
    #[serde(default)]
    pub lemon: bool,
    #[serde(default)]
    pub lemon_px: f32,
    /// Exact sugar cubes (0 = no sugar step); ±1 scores 0.8.
    #[serde(default)]
    pub sugar: u32,
    /// Chai: "pour the warm milk" hold at the end.
    #[serde(default)]
    pub milk_hold: bool,
    /// A spoonful of honey dropped like the lemon (linden, Tatra tea, uzvar).
    #[serde(default)]
    pub honey: bool,
    /// Chai: spices added in the recipe card's order before steeping.
    #[serde(default)]
    pub spices: u32,
    /// Chai simmers in a pan.
    #[serde(default)]
    pub pan: bool,
    /// Colour family (`src/minigame` `TeaKind`): one of [`TEA_KINDS`].
    pub tea_kind: String,
    pub seconds: (f32, f32),
}

/// An extra gesture on a plate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum PlateGesture {
    /// Mirek's topinka: rub the garlic back and forth twice.
    GarlicRub,
    /// Hold over the plate to sprinkle (crispy onion, sugar dust).
    Sprinkle,
}

/// Plate: `q = 0.7 × mean(slot_score) + 0.3 × garnish` (slot score `1 − dist / slot_px`).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlateTuning {
    pub variant: String,
    /// The dish set the mini-game draws (`src/minigame` `PlateSet`, one of [`PLATE_DISHES`]);
    /// `None` = picked by seed until per-dish layouts exist.
    #[serde(default)]
    pub dish: Option<String>,
    pub slot_px: f32,
    /// Keyboard drop snaps to the nearest slot and scores this.
    pub keyboard_score: f32,
    /// "Lida's way" ghost layout, seconds.
    pub ghost_s: f32,
    #[serde(default)]
    pub gesture: Option<PlateGesture>,
    /// Advent nights: an extra spruce-twig slot.
    #[serde(default)]
    pub advent_twig: bool,
    pub seconds: (f32, f32),
}

/// SoupStir: `q = 0.6 × time_in_zone + 0.4 × mean(add timing)`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SoupStirTuning {
    pub variant: String,
    /// Ingredient chips clicked when the bubble ring glows.
    pub adds: Vec<String>,
    pub add_window_s: f32,
    /// Kulajda: a final cream-swirl tap.
    #[serde(default)]
    pub finish_tap: bool,
    pub seconds: (f32, f32),
}

/// MulledSpice: `q = 0.5 × spices + 0.5 × band_time / band_s`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MulledSpiceTuning {
    pub variant: String,
    /// The recipe card's spices (a subset of `tray`).
    pub spices: Vec<String>,
    /// Everything on the tray.
    pub tray: Vec<String>,
    pub wrong_penalty: f32,
    pub band_s: f32,
    pub seconds: (f32, f32),
}

/// A latte-art pattern (`latte.<id>`); available when every set rule holds.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LattePattern {
    pub id: String,
    /// Story night (sandbox: nights played) from which it is offered.
    #[serde(default)]
    pub from_night: Option<u32>,
    #[serde(default)]
    pub when: Vec<Cond>,
    /// Only for this recipe (and always for it).
    #[serde(default)]
    pub recipe: Option<String>,
    /// Only via `UnlockLatte` (e.g. the first snow), unless `from_night` / `when` also allow it.
    #[serde(default)]
    pub granted: bool,
    #[serde(default)]
    pub tier: Tier,
}

/// `assets/data/minigames.ron`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MiniGamesFile {
    pub common: CommonTuning,
    pub quality_tiers: QualityTiers,
    pub milk_froth: Vec<MilkFrothTuning>,
    pub latte_art: LatteArtTuning,
    pub pour_over: Vec<PourOverTuning>,
    pub tea_steep: Vec<TeaSteepTuning>,
    pub plate: Vec<PlateTuning>,
    pub soup_stir: Vec<SoupStirTuning>,
    pub mulled_spice: Vec<MulledSpiceTuning>,
    pub latte_patterns: Vec<LattePattern>,
}

impl MiniGamesFile {
    /// Variant ids defined for a kind.
    pub fn variants(&self, kind: MiniGameKind) -> Vec<&str> {
        match kind {
            MiniGameKind::MilkFroth => self.milk_froth.iter().map(|v| v.variant.as_str()).collect(),
            MiniGameKind::LatteArt => self
                .latte_art
                .variants
                .iter()
                .map(|v| v.variant.as_str())
                .collect(),
            MiniGameKind::PourOver => self.pour_over.iter().map(|v| v.variant.as_str()).collect(),
            MiniGameKind::TeaSteep => self.tea_steep.iter().map(|v| v.variant.as_str()).collect(),
            MiniGameKind::Plate => self.plate.iter().map(|v| v.variant.as_str()).collect(),
            MiniGameKind::SoupStir => self.soup_stir.iter().map(|v| v.variant.as_str()).collect(),
            MiniGameKind::MulledSpice => self
                .mulled_spice
                .iter()
                .map(|v| v.variant.as_str())
                .collect(),
        }
    }

    pub fn has_variant(&self, kind: MiniGameKind, variant: &str) -> bool {
        self.variants(kind).contains(&variant)
    }

    pub fn milk_froth(&self, variant: &str) -> Option<&MilkFrothTuning> {
        self.milk_froth.iter().find(|v| v.variant == variant)
    }
    pub fn pour_over(&self, variant: &str) -> Option<&PourOverTuning> {
        self.pour_over.iter().find(|v| v.variant == variant)
    }
    pub fn tea_steep(&self, variant: &str) -> Option<&TeaSteepTuning> {
        self.tea_steep.iter().find(|v| v.variant == variant)
    }
    pub fn plate(&self, variant: &str) -> Option<&PlateTuning> {
        self.plate.iter().find(|v| v.variant == variant)
    }
    pub fn soup_stir(&self, variant: &str) -> Option<&SoupStirTuning> {
        self.soup_stir.iter().find(|v| v.variant == variant)
    }
    pub fn mulled_spice(&self, variant: &str) -> Option<&MulledSpiceTuning> {
        self.mulled_spice.iter().find(|v| v.variant == variant)
    }
    pub fn latte_pattern(&self, id: &str) -> Option<&LattePattern> {
        self.latte_patterns.iter().find(|p| p.id == id)
    }
}
