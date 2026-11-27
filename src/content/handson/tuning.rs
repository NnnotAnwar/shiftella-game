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
    1.0
}

/// `assets/data/handson.ron`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HandsonTuning {
    /// Real seconds per sim tick at ×1 (0.15 → a night is 12 real minutes).
    pub real_seconds_per_tick_x1: f32,
    /// Real seconds per tick × this during the two quiet halves (the preparation 21:30–22:00
    /// and the closing half hour 06:00–06:30, doors closed; their clock already runs at
    /// `sim::QUIET_TICKS_PER_MINUTE`): 1.0 → ~1.6 real minutes per half at ×1.
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
    /// Hands-on appliances (design/v0.2.md §B): `handson.ron` → `equipment`.
    #[serde(default)]
    pub equipment: EquipmentTuning,
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

// ---------------------------------------------------------------------------------------------
// Hands-on appliances (design/v0.2.md §B, `sim::equipment`)
// ---------------------------------------------------------------------------------------------

/// Karolina's work at an appliance, in work ticks at speed 1 (10 ticks = 1 game minute).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct EquipWork {
    pub warm_up: f32,
    pub pull_tray: f32,
    pub service: f32,
    pub clean_up: f32,
    pub soup_on: f32,
    pub stir: f32,
    pub bake: f32,
    pub take_out: f32,
    pub milk: f32,
    pub count_till: f32,
    pub trash: f32,
}

impl Default for EquipWork {
    fn default() -> Self {
        EquipWork {
            warm_up: 30.0,
            pull_tray: 25.0,
            service: 40.0,
            clean_up: 30.0,
            soup_on: 25.0,
            stir: 8.0,
            bake: 25.0,
            take_out: 8.0,
            milk: 10.0,
            count_till: 30.0,
            trash: 25.0,
        }
    }
}

/// "Pull a tray ahead": the next `count` plain coffees are made almost at once.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct TrayTuning {
    pub count: u32,
    /// The shots go cold after this many game minutes (it's for the next train).
    pub minutes: u32,
    pub prep_mul: f32,
    /// Plain menu items that count as "a coffee".
    pub items: Vec<String>,
}

impl Default for TrayTuning {
    fn default() -> Self {
        TrayTuning {
            count: 3,
            minutes: 40,
            prep_mul: 0.05,
            items: vec![
                "black_coffee".into(),
                "cappuccino".into(),
                "oat_flat_white".into(),
            ],
        }
    }
}

/// "Take the milk out": the first lattes of the night are quicker.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct MilkTuning {
    pub count: u32,
    /// A kitchen helper does it at 22:00 when Karolina didn't (fewer cups).
    pub helper_count: u32,
    pub prep_mul: f32,
    /// Until this night minute (0 = 22:00) she can still take it out.
    pub until_minute: u32,
    pub items: Vec<String>,
}

impl Default for MilkTuning {
    fn default() -> Self {
        MilkTuning {
            count: 5,
            helper_count: 3,
            prep_mul: 0.7,
            until_minute: 60,
            items: vec![
                "cappuccino".into(),
                "oat_flat_white".into(),
                "hot_chocolate".into(),
                "chai".into(),
            ],
        }
    }
}

/// The soup pot on the stove: soups are a little quicker all night, much quicker for a
/// while after a stir. The pot bubbles now and then; untouched it just keeps warm.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SoupTuning {
    pub items: Vec<String>,
    pub warm_mul: f32,
    pub helper_warm_mul: f32,
    pub stir_mul: f32,
    pub stir_minutes: u32,
    pub bubble_every: u32,
    pub bubble_minutes: u32,
    /// Until this night minute she can still put the pot on.
    pub until_minute: u32,
}

impl Default for SoupTuning {
    fn default() -> Self {
        SoupTuning {
            items: vec!["garlic_soup".into(), "grandma_kulajda".into()],
            warm_mul: 0.9,
            helper_warm_mul: 0.95,
            stir_mul: 0.6,
            stir_minutes: 30,
            bubble_every: 35,
            bubble_minutes: 12,
            until_minute: 120,
        }
    }
}

/// Grandma's medovník in the oven: after the ding and "take it out", cake slices are served
/// without waiting (a limited number tonight).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct CakeTuning {
    pub items: Vec<String>,
    pub bake_minutes: u32,
    pub slices: u32,
    pub helper_slices: u32,
    pub prep_mul: f32,
    /// A kitchen helper takes a ready cake out after this many minutes.
    pub helper_after: u32,
    pub until_minute: u32,
}

impl Default for CakeTuning {
    fn default() -> Self {
        CakeTuning {
            items: vec!["medovnik".into(), "kolac".into()],
            bake_minutes: 12,
            slices: 8,
            helper_slices: 4,
            prep_mul: 0.05,
            helper_after: 10,
            until_minute: 120,
        }
    }
}

/// Closing chores: count the till, take the trash out.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ClosingTuning {
    /// From this night minute (435 = 05:15, last call) once the café is empty; always in the
    /// closing half hour (06:00–06:30).
    pub from_minute: u32,
    /// Hall dirt the trash run takes away.
    pub trash_dirt: f32,
    /// Chance the station cat waits by the bins (affection +1).
    pub cat_chance: f32,
}

impl Default for ClosingTuning {
    fn default() -> Self {
        ClosingTuning {
            from_minute: 435,
            trash_dirt: 0.03,
            cat_chance: 0.35,
        }
    }
}

/// `handson.ron` → `equipment` (design/v0.2.md §B).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct EquipmentTuning {
    pub work: EquipWork,
    /// Holding the mouse / **E** on a hold chore adds this many work ticks per real second.
    pub hold_ticks_per_second: f32,
    /// Appliances at or above this wear sputter; "purge / service" resets the wear (free).
    pub service_wear: f32,
    /// A worn machine sputters (a sound cue) every N minutes.
    pub sputter_every: u32,
    pub tray: TrayTuning,
    pub milk: MilkTuning,
    pub soup: SoupTuning,
    pub cake: CakeTuning,
    pub closing: ClosingTuning,
}

impl Default for EquipmentTuning {
    fn default() -> Self {
        EquipmentTuning {
            work: EquipWork::default(),
            hold_ticks_per_second: 40.0,
            service_wear: 0.5,
            sputter_every: 25,
            tray: TrayTuning::default(),
            milk: MilkTuning::default(),
            soup: SoupTuning::default(),
            cake: CakeTuning::default(),
            closing: ClosingTuning::default(),
        }
    }
}
