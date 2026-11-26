//! Hands-on tuning constants (`assets/data/handson.ron`, handson.md §1, §7.1).

use serde::{Deserialize, Serialize};

use super::rozpis::RozpisTuning;
use super::wishes::MatchTiers;
use crate::content::defs::ClockTime;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayScale {
    pub cozy: f32,
    pub classic: f32,
}

/// Helper item quality `base + per_skill × skill`, capped at `max`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HelperQuality {
    pub base: f32,
    pub per_skill: f32,
    pub max: f32,
}

impl HelperQuality {
    pub fn at(self, skill: f32) -> f32 {
        (self.base + self.per_skill * skill).min(self.max)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WishChance {
    pub from_night: u32,
    pub chance: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FairWishes {
    /// Wishes whose best available match ≥ this get × `boost_mul` weight.
    pub boost_at: f32,
    pub boost_mul: f32,
    /// Wishes whose best available match < this are never picked.
    pub skip_below: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Forgiveness {
    /// Patience refills once to this share when it runs out (Cozy).
    pub refill: f32,
    /// After this many run-outs they leave content.
    pub runs: u32,
    pub leave_satisfaction: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RushRules {
    /// ≥ parties arriving within `window_minutes` start a rush.
    pub parties: u32,
    pub window_minutes: u32,
    pub minutes: u32,
    /// No rushes before this story night.
    pub from_night: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Rhythm {
    pub window_minutes: u32,
    pub step: f32,
    pub max: f32,
}

/// Serve formulas (§1.7).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServeFormula {
    /// Satisfaction += `match × satisfaction_match + quality × satisfaction_quality`.
    pub satisfaction_match: f32,
    pub satisfaction_quality: f32,
    /// Trust += `trust_base + trust_match × match + trust_quality × quality`.
    pub trust_base: f32,
    pub trust_match: f32,
    pub trust_quality: f32,
    /// Tip multiplier `tip_base + tip_span × match × (0.5 + 0.5 × quality)` × rhythm.
    pub tip_base: f32,
    pub tip_span: f32,
    /// Classic only: satisfaction for a `not_quite` serve (Cozy never negative).
    pub classic_not_quite: f32,
    /// Neighbours of a latte-art cup get this once.
    pub latte_neighbour: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Greeting {
    pub window_minutes: u32,
    pub satisfaction: f32,
    pub trust: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MomentBudget {
    pub act: u32,
    pub range: (u32, u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ThanksChance {
    pub guest: f32,
    pub regular: f32,
    pub min_match: f32,
    pub min_quality: f32,
}

/// Helper capacity `base + [nights ≥ after_nights] + [duty skill ≥ skill_bonus_at or trait]`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Capacity {
    pub base: u32,
    pub after_nights: u32,
    pub skill_bonus_at: f32,
    pub trait_bonus: String,
    pub max: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerEnergy {
    /// Classic: Karolina's energy drains at this share of the staff rate (Cozy: never).
    pub classic_drain_mul: f32,
    /// "Make yourself a coffee" chip below this energy.
    pub mug_below: f32,
    pub mug_restore: f32,
}

/// The quiet hour 03:00–04:00 (§6.4).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QuietHour {
    pub from: ClockTime,
    pub to: ClockTime,
    /// Lamps dim by this share.
    pub lamp_dim: f32,
    pub rain_volume_mul: f32,
    pub guest_rate_mul: f32,
    /// Wish share of new parties (with the personal pool).
    pub wish_share: f32,
    /// Toast key at the start.
    pub toast: String,
}

fn default_prep_tick_scale() -> f32 {
    0.7
}

/// `assets/data/handson.ron`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HandsonTuning {
    /// Real seconds per sim tick at ×1 (0.15 → a night is 12 real minutes).
    pub real_seconds_per_tick_x1: f32,
    /// Real seconds per tick × this during the preparation half hour (21:30–22:00, doors
    /// closed): 0.7 → the half hour is ~27 real seconds at ×1 instead of 39.
    #[serde(default = "default_prep_tick_scale")]
    pub prep_tick_scale: f32,
    pub overlay_scale: OverlayScale,
    pub player_walk_mul: f32,
    pub player_work_mul: f32,
    pub helper_speed: f32,
    pub helper_quality: HelperQuality,
    pub quick_quality: f32,
    pub practised_quick_quality: f32,
    /// By-hand plays at ≥ `practised_min_quality` before a recipe is practised.
    pub practised_after: u32,
    pub practised_min_quality: f32,
    /// By hand: after the mini-game she works only `prep × by_hand_work_share`.
    pub by_hand_work_share: f32,
    /// By hand is suggested for at most N same-kind mini-games in a row.
    pub by_hand_streak_max: u32,
    pub match_tiers: MatchTiers,
    pub wish_chance: Vec<WishChance>,
    pub sandbox_wish_chance: f32,
    pub fair_wishes: FairWishes,
    pub usual_trust: f32,
    pub hand_off_match_cap: f32,
    /// Accessibility `assist_wishes`: helpers hand off wishes after N unclaimed minutes.
    pub assist_wishes_after: u32,
    pub claimed_wait_rate: f32,
    pub cozy_rush_wait_rate: f32,
    pub cozy_forgiveness: Forgiveness,
    pub cozy_regular_patience_floor: f32,
    pub regular_announce_minutes: u32,
    pub greeting: Greeting,
    pub rush: RushRules,
    pub rhythm: Rhythm,
    pub serve: ServeFormula,
    pub lull_minutes: u32,
    pub quiet_hour_lull_minutes: u32,
    /// The lull director's walk-in boost: guest rate × mul for minutes, from story night.
    pub lull_walk_in: (f32, u32, u32),
    pub moments_per_night: Vec<MomentBudget>,
    pub moment_gap_minutes: u32,
    pub moments_max_per_night: u32,
    pub experiment_cost: u32,
    pub experiment_free_until_night: u32,
    /// Night: an experiment costs Karolina this many game minutes at the counter.
    pub experiment_minutes: u32,
    pub margin_note_every: u32,
    pub thanks_chance: ThanksChance,
    pub capacity: Capacity,
    pub player_energy: PlayerEnergy,
    pub quiet_hour: QuietHour,
    pub rozpis: RozpisTuning,
}

impl HandsonTuning {
    /// Chance that a non-regular party leader has a wish on a story night.
    pub fn wish_chance_on(&self, night: u32) -> f32 {
        self.wish_chance
            .iter()
            .filter(|w| night >= w.from_night)
            .map(|w| w.chance)
            .next_back()
            .unwrap_or(0.0)
    }

    /// The moment budget (min, max) for a story act.
    pub fn moment_budget(&self, act: u32) -> (u32, u32) {
        self.moments_per_night
            .iter()
            .find(|b| b.act == act)
            .map(|b| b.range)
            .unwrap_or((2, 4))
    }
}
