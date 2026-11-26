//! Helpers' duties, staff preferences and the rozpis puzzle (handson.md §1.3, §12).
//! Story people's prefs live in `staff.ron`; tuning and sandbox rules in `handson.ron` → `rozpis`.

use serde::{Deserialize, Serialize};

use crate::content::defs::{Job, Weekday};

/// A delegable duty (`duty.<id>.name/.desc`). **Canonical** — the sim's `Staff.duties` uses it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize)]
pub enum Duty {
    /// ClearTable, Wash; puts left tips in the jar.
    Dishes,
    /// Mop, bucket, cleaning checklist items.
    Floor,
    /// TakeOrder for Plain orders, payments.
    Register,
    /// Plain Bar items, base recipes only.
    Bar,
    /// Plain Kitchen items, base recipes only.
    Kitchen,
    /// Serve items prepared by helpers (never Karolina's).
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

    /// The skill a duty uses.
    pub fn job(self) -> Job {
        match self {
            Duty::Dishes | Duty::Floor => Job::Cleaning,
            Duty::Register => Job::Register,
            Duty::Bar => Job::Bar,
            Duty::Kitchen => Job::Kitchen,
            Duty::Serving => Job::Serving,
        }
    }

    /// The default duty for a job (checklist items written with `job:`).
    pub fn for_job(job: Job) -> Duty {
        match job {
            Job::Bar => Duty::Bar,
            Job::Kitchen => Duty::Kitchen,
            Job::Register => Duty::Register,
            Job::Cleaning => Duty::Floor,
            Job::Serving => Duty::Serving,
        }
    }

    pub fn name_key(self) -> String {
        format!("duty.{}.name", self.id())
    }
    pub fn desc_key(self) -> String {
        format!("duty.{}.desc", self.id())
    }
    /// Morning digest credit line (`app.digest.helper_<duty>`, `{name}`, `{n}`).
    pub fn digest_key(self) -> String {
        format!("app.digest.helper_{}", self.id())
    }
}

/// A shift kind a person prefers (maps to the sim's `Shift::{Early, Late, Full}`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum PreferredShift {
    Early,
    Late,
    Full,
}

/// What a preference asks for (`pref.kind.<id>` chip label).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum PrefKind {
    /// That weekday is Off.
    DayOff(Weekday),
    /// Every working night uses this shift.
    ShiftPref(PreferredShift),
    /// Need: Fri and Sat Off · Wish: at least one of them.
    WeekendOff,
    /// No two consecutive working nights (incl. last week's Sunday).
    NoBackToBack,
    MaxNights(u32),
    MinNights(u32),
    /// ≥ 4 h shared on ≥ 2 nights with that staff archetype.
    WorksWith(String),
    /// Never shares a shift (dynamic: relationship < −30, gone above −10).
    AvoidWith(String),
}

impl PrefKind {
    pub fn id(&self) -> &'static str {
        match self {
            PrefKind::DayOff(_) => "day_off",
            PrefKind::ShiftPref(_) => "shift_pref",
            PrefKind::WeekendOff => "weekend_off",
            PrefKind::NoBackToBack => "no_back_to_back",
            PrefKind::MaxNights(_) => "max_nights",
            PrefKind::MinNights(_) => "min_nights",
            PrefKind::WorksWith(_) => "works_with",
            PrefKind::AvoidWith(_) => "avoid_with",
        }
    }

    pub const IDS: [&'static str; 8] = [
        "day_off",
        "shift_pref",
        "weekend_off",
        "no_back_to_back",
        "max_nights",
        "min_nights",
        "works_with",
        "avoid_with",
    ];

    /// Chip label key (`pref.kind.max_nights` — placeholders `{day}`, `{shift}`, `{n}`, `{name}`).
    pub fn chip_key(&self) -> String {
        format!("pref.kind.{}", self.id())
    }
}

/// Need = breaking it hurts (−25 wellbeing); Wish = nice to honour (−12 / +4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum Strength {
    Need,
    Wish,
}

/// One preference of a person (`quote` = i18n key of their line, shown on hover).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pref {
    pub kind: PrefKind,
    pub strength: Strength,
    pub quote: String,
}

/// Sandbox preference template (resolved by the generator).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum SandboxPref {
    Shift(PreferredShift),
    MaxNights(u32),
    MinNights(u32),
    WeekendOff,
    NoBackToBack,
    /// A random weekday off.
    DayOffRandom,
    /// `WorksWith` their romance partner (only when a romance exists).
    WorksWithPartner,
}

impl SandboxPref {
    /// Generic quote for generated people (`pref.quote.<id>`).
    pub fn quote_key(&self) -> String {
        let id = match self {
            SandboxPref::Shift(PreferredShift::Early) => "early",
            SandboxPref::Shift(PreferredShift::Late) => "late",
            SandboxPref::Shift(PreferredShift::Full) => "full",
            SandboxPref::MaxNights(_) => "max_nights",
            SandboxPref::MinNights(_) => "min_nights",
            SandboxPref::WeekendOff => "weekend_off",
            SandboxPref::NoBackToBack => "no_back_to_back",
            SandboxPref::DayOffRandom => "day_off",
            SandboxPref::WorksWithPartner => "partner",
        };
        format!("pref.quote.{id}")
    }
}

/// Sandbox generation: trait → pref, plus one random extra.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PrefRules {
    pub from_traits: Vec<(String, SandboxPref)>,
    pub random: Vec<SandboxPref>,
    /// Chance a generated pref is a Need.
    pub need_chance: f32,
    pub max: u32,
}

/// Wellbeing ring (`rozpis.ring.<id>`), from the highest threshold down; never red.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ring {
    pub id: String,
    pub min: f32,
}

/// Request kinds the rozpis creates from preferences / life events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum RequestKind {
    Swap,
    TimeOff,
    /// "Can I go at 2?" (new; `AnswerEarlyLeave`).
    EarlyLeave,
    /// Náhradní volno after > `toil_hours` overtime.
    Toil,
}

/// A one-week preference + request from a life event (`app.request.<id>` reason line).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LifeEvent {
    pub id: String,
    pub staff: String,
    /// Story night the week starts; the pref lasts that week.
    pub night: u32,
    #[serde(default)]
    pub pref: Option<Pref>,
    #[serde(default)]
    pub request: Option<RequestKind>,
    /// Placeholders of `app.request.<id>`.
    #[serde(default)]
    pub placeholders: Vec<String>,
}

/// Answer consequences (§13.3).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequestRules {
    pub approve_wellbeing: f32,
    pub approve_relationship: f32,
    /// A swap partner's wellbeing unless the swap suits their prefs.
    pub partner_wellbeing: f32,
    pub decline_mood: f32,
    /// Declined Need: chance they come in late.
    pub decline_need_late: f32,
    /// N declines within `tired_window_nights` → the bot's "{name} looks tired".
    pub tired_after: u32,
    pub tired_window_nights: u32,
    pub no_answer_mood: f32,
    /// Overtime hours that create a náhradní volno request.
    pub toil_hours: f32,
}

/// Rozpis puzzle tuning (`handson.ron` → `rozpis`), §12.3–12.5.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RozpisTuning {
    pub need_penalty: f32,
    pub wish_penalty: f32,
    /// Nights above this count cost `nights_penalty` each.
    pub nights_soft_max: u32,
    pub nights_penalty: f32,
    pub wish_bonus: f32,
    pub wish_bonus_max: f32,
    /// Rings, highest first: Happy ≥ 85, Fine ≥ 65, Tired ≥ 45, else Stretched.
    pub rings: Vec<Ring>,
    /// `fairness = mean(wb) − stdev_weight × stdev(wb)`; week icon thresholds (`rozpis.week.<n>`).
    pub stdev_weight: f32,
    pub week_icons: Vec<f32>,
    /// Effects of the published week.
    pub mood_baseline_center: f32,
    pub mood_baseline_per: f32,
    pub energy_loss_per: f32,
    pub energy_min: f32,
    pub late_mul: (f32, f32),
    pub sick_mul: (f32, f32),
    pub need_violated_swap_chance: f32,
    pub need_violated_mood: f32,
    pub works_with_relationship_per_hour: f32,
    pub works_with_speed: f32,
    pub happy_week_mood: f32,
    /// Auto-fill costs.
    pub autofill_wish_cost: f32,
    pub autofill_balance_cost: f32,
    pub autofill_partner_bonus: f32,
    pub autofill_min_nights_bonus: f32,
    /// A Need is broken only when coverage would fall this far below the suggestion.
    pub autofill_need_gap: u32,
    pub requests: RequestRules,
    pub pref_rules: PrefRules,
    pub life_events: Vec<LifeEvent>,
}

impl RozpisTuning {
    /// The ring for a wellbeing value (falls back to the last ring).
    pub fn ring(&self, wb: f32) -> Option<&Ring> {
        self.rings
            .iter()
            .find(|r| wb >= r.min)
            .or_else(|| self.rings.last())
    }
}
