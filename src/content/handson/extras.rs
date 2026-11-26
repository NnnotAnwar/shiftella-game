//! Station cat friendship, tip-jar goals, thank-you napkins, regulars' rituals and Karolina's
//! mug (`assets/data/extras.ron`, handson.md §6).

use serde::{Deserialize, Serialize};

use super::logic::Fx;

/// What a cat level brings (render poses are SHOULD).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum CatPerk {
    /// Sits on the doormat.
    Doormat,
    /// Comes inside, sleeps under the radiator.
    Inside,
    /// Its own spot on the window seat: guests nearby + satisfaction.
    WindowSeat { satisfaction: f32 },
    /// Follows Karolina; staff mood + per night.
    WorksHere { staff_mood: f32 },
    /// The naming moment.
    Naming { moment: String },
    /// A latte pattern unlocks.
    Latte(String),
}

/// A friendship level (`cat.level.<level>`): reached at `cat_affection` ≥ `level`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CatLevel {
    pub level: u32,
    #[serde(default)]
    pub perks: Vec<CatPerk>,
}

/// Station cat (`cat_affection` counter 0..=max).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CatDef {
    pub counter: String,
    pub max: i32,
    pub levels: Vec<CatLevel>,
    /// Name choices of the naming moment (`cat.name.<id>`); var `cat_name`.
    pub names: Vec<String>,
}

impl CatDef {
    /// The highest level reached at an affection value.
    pub fn level_at(&self, affection: i32) -> u32 {
        self.levels
            .iter()
            .filter(|l| affection >= l.level as i32)
            .map(|l| l.level)
            .max()
            .unwrap_or(0)
    }
}

/// A lifetime tip-jar goal: a "Count the jar" chip, then a choice of decor (`ui.tipjar.goal`).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TipGoal {
    pub lifetime: u32,
    /// Furniture ids offered (pick one; the other stays for the next goal).
    pub choices: Vec<String>,
}

/// A doodle on a napkin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum Doodle {
    Cat,
    Train,
    Heart,
    Moon,
    Cup,
    Star,
    Raindrop,
}

/// A thank-you napkin (`thanks.<id>`). `who` empty = any guest; otherwise only those regulars.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Napkin {
    pub id: String,
    #[serde(default)]
    pub who: Vec<String>,
    #[serde(default)]
    pub doodle: Option<Doodle>,
}

/// Rewards for collecting napkins.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NapkinReward {
    pub count: u32,
    pub effects: Vec<Fx>,
}

/// A regular's small tell (`ritual.<regular>`), rewarded when answered in time (SHOULD).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RitualDef {
    pub regular: String,
    pub response: RitualResponse,
}

/// What answers a regular's tell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum RitualResponse {
    Greet,
    WipeTable,
    StartByHand,
    GlassOfWater,
    SocketTableFree,
    CoatRackNear,
    WindowLightOn,
}

/// Rules shared by the rituals.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RitualRules {
    pub window_minutes: u32,
    pub trust: f32,
}

/// `assets/data/extras.ron`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExtrasFile {
    pub cat: CatDef,
    pub tip_goals: Vec<TipGoal>,
    pub napkins: Vec<Napkin>,
    pub napkin_rewards: Vec<NapkinReward>,
    pub ritual_rules: RitualRules,
    pub rituals: Vec<RitualDef>,
    /// Number of `karolina.mug.<n>` lines (1-based).
    pub mug_lines: u32,
}
