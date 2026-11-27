//! Events, narrator pacing, the train line, the in-game Shiftella app, difficulty and sandbox.

use std::collections::HashMap;

use serde::Deserialize;

use super::defs::{ClockTime, Job, Modes};
use super::handson::{
    BotHandler, ChecklistOwner, Duty, Fx, Gesture, HandoverOption, InsightDef, PayrollRules, Ritual,
};
use super::logic::{Condition, Effect};

// ---------------------------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------------------------

/// For the narrator's balance and the toast colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum EventKind {
    Good,
    Bad,
    Neutral,
    Uncanny,
}

impl EventKind {
    /// Toast category: `good`, `bad`, `info`, `warn` (UI kit colours).
    pub fn toast_category(self) -> &'static str {
        match self {
            EventKind::Good => "good",
            EventKind::Bad => "bad",
            EventKind::Neutral => "info",
            EventKind::Uncanny => "info",
        }
    }
}

/// How an event can fire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
pub enum EventTrigger {
    /// Rolled by the narrator (weights, cooldowns, tension budget).
    #[default]
    Random,
    /// Fired by the sim when its conditions become true (walkout, broken dishes, 3:33 clock).
    /// Checked once per game minute; at most once per night unless `cooldown` is 0.
    Auto,
    /// Only from story beats / effects (`Beat::Event`, `Effect::StartEvent`).
    Scripted,
}

/// A choice answered in a non-blocking toast (buttons work while paused, too).
/// If the player ignores the toast, `default_choice` (or nothing) applies when it expires.
#[derive(Debug, Clone, Deserialize)]
pub struct EventChoice {
    pub id: String,
    #[serde(default)]
    pub effects: Vec<Effect>,
    #[serde(default)]
    pub set_flag: Option<String>,
    /// Money shown on the button ("−150 Kč"); informational, the cost is in `effects`.
    #[serde(default)]
    pub cost: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EventDef {
    pub id: String,
    pub kind: EventKind,
    /// 1–3: tension cost (`severity × 10`, negative for Good events).
    pub severity: u32,
    #[serde(default)]
    pub trigger: EventTrigger,
    /// Base weight for the random roll (ignored for Auto / Scripted).
    #[serde(default)]
    pub weight: f32,
    /// Nights before it can repeat.
    #[serde(default)]
    pub cooldown: u32,
    #[serde(default)]
    pub modes: Modes,
    #[serde(default = "one")]
    pub min_night: u32,
    #[serde(default)]
    pub conditions: Vec<Condition>,
    /// Applied when the event fires (before any choice).
    #[serde(default)]
    pub effects: Vec<Effect>,
    #[serde(default)]
    pub choices: Vec<EventChoice>,
    /// Choice applied if the toast times out (`None` = nothing more happens).
    #[serde(default)]
    pub default_choice: Option<String>,
}

fn one() -> u32 {
    1
}

impl EventDef {
    pub fn title_key(&self) -> String {
        format!("event.{}.title", self.id)
    }
    pub fn text_key(&self) -> String {
        format!("event.{}.text", self.id)
    }
    pub fn choice_key(&self, choice_id: &str) -> String {
        format!("event.{}.choice.{choice_id}", self.id)
    }
    /// Short result line shown after a choice ("Candles lit · −150 Kč").
    pub fn result_key(&self, choice_id: &str) -> String {
        format!("event.{}.result.{choice_id}", self.id)
    }
}

/// Narrator pacing (events.md §6).
#[derive(Debug, Clone, Deserialize)]
pub struct NarratorDef {
    /// Piecewise-constant intensity curve: from each time until the next entry.
    pub night_curve: Vec<(ClockTime, f32)>,
    /// Uncanny events weight multiplier between these times (witching hour).
    pub witching_hour: (ClockTime, ClockTime, f32),
    pub story_acts: Vec<ActPacing>,
    /// No severity-3 bad events the night before a payment.
    pub calm_before_payment: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ActPacing {
    pub nights: (u32, u32),
    pub act_mult: f32,
    pub max_severity: u32,
    /// At least one Good event per night in this range.
    #[serde(default)]
    pub warm_event_guaranteed: bool,
}

impl NarratorDef {
    /// Curve value at a time.
    pub fn curve_at(&self, t: ClockTime) -> f32 {
        let mut value = self.night_curve.first().map(|p| p.1).unwrap_or(1.0);
        for (from, v) in &self.night_curve {
            if *from <= t {
                value = *v;
            }
        }
        value
    }

    pub fn act_for_night(&self, night: u32) -> Option<&ActPacing> {
        self.story_acts
            .iter()
            .find(|a| (a.nights.0..=a.nights.1).contains(&night))
    }
}

// ---------------------------------------------------------------------------------------------
// Trains
// ---------------------------------------------------------------------------------------------

/// A station on the line. Names are proper nouns (not translated); `name` is shown as-is.
#[derive(Debug, Clone, Deserialize)]
pub struct Station {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum TrainKind {
    /// Os — local.
    Regional,
    /// R — fast.
    Express,
    /// EN / night express with sleeping cars.
    Night,
    /// Autonomous freight — no passengers, just light and sound.
    Freight,
    /// Rn 333 — in no timetable since 1979.
    Phantom,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TrainDef {
    /// Train number as shown on the board: `Os 2141`, `Rn 333`.
    pub id: String,
    pub kind: TrainKind,
    pub from: String,
    pub to: String,
    /// Arrival at the café's station (Praha-Holešovice).
    pub arrives: ClockTime,
    /// Passengers who come to the café (inclusive range).
    pub guests: (u32, u32),
    /// Guest archetype of those passengers.
    pub guest: String,
    /// Board announcement is made only if these hold (empty = always).
    #[serde(default)]
    pub announce_if: Vec<Condition>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TrainLine {
    /// In line order.
    pub stations: Vec<Station>,
    /// The café's station id (`holesovice`).
    pub home: String,
    pub trains: Vec<TrainDef>,
    /// Announcement template keys (placeholders `{train}`, `{from}`, `{to}`, `{time}`, `{platform}`).
    pub announce_arrival: String,
    pub announce_delay: String,
}

impl TrainLine {
    pub fn station(&self, id: &str) -> Option<&Station> {
        self.stations.iter().find(|s| s.id == id)
    }
}

// ---------------------------------------------------------------------------------------------
// Shiftella app
// ---------------------------------------------------------------------------------------------

/// A pinned announcement on the board (`app.announcement.<id>.title` / `.body`).
#[derive(Debug, Clone, Deserialize)]
pub struct Announcement {
    pub id: String,
    /// Who posted it: `grandma`, `player`, a cast or staff id.
    pub author: String,
    /// Story: posted at the start of this night. Sandbox: `None` = available as a template.
    #[serde(default)]
    pub night: Option<u32>,
    /// Template placeholders the integration fills.
    #[serde(default)]
    pub placeholders: Vec<String>,
    /// Hands-on: real effects when posted (target = the person / item named).
    #[serde(default)]
    pub effects: Vec<Fx>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, serde::Serialize)]
pub enum ChecklistKind {
    Opening,
    Closing,
}

/// One checklist line (`app.checklist.<id>`). Ticked automatically by the sim when done;
/// the player can also tick it in the app. Unticked items at 06:00 apply `missed`.
#[derive(Debug, Clone, Deserialize)]
pub struct ChecklistItem {
    pub id: String,
    pub list: ChecklistKind,
    /// Job that does it (sim queues it as a work item).
    #[serde(default)]
    pub job: Option<Job>,
    #[serde(default)]
    pub minutes: u32,
    /// Applied when ticked.
    #[serde(default)]
    pub done: Vec<Effect>,
    /// Applied at 06:00 when not ticked.
    #[serde(default)]
    pub missed: Vec<Effect>,
    /// Hands-on: who does it (`None` = derived from `job`, see [`ChecklistItem::owner`]).
    #[serde(default)]
    pub owner: Option<ChecklistOwner>,
    /// Hands-on: furniture tag the checkbox chip sits on in the world.
    #[serde(default)]
    pub anchor: Option<String>,
    #[serde(default)]
    pub gesture: Option<Gesture>,
    /// Hands-on effects applied with `done` (once the sim runs [`Fx`]).
    #[serde(default)]
    pub extra_done: Vec<Fx>,
    /// Hands-on effects applied with `missed`.
    #[serde(default)]
    pub extra_missed: Vec<Fx>,
}

impl ChecklistItem {
    /// Explicit owner, else `Either(duty of job)`, else Karolina.
    pub fn owner(&self) -> ChecklistOwner {
        self.owner.unwrap_or(match self.job {
            Some(job) => ChecklistOwner::Either(Duty::for_job(job)),
            None => ChecklistOwner::Karolina,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum BotKind {
    SchedulePublished,
    LateAlert,
    SwapRequest,
    TimeOffRequest,
    SickCall,
    Handover,
    NightDigest,
    PayrollSummary,
    PaymentReminder,
    Announcement,
    Story,
}

/// A button on a bot message (`app.bot.<template>.action.<id>`). Never clock-in/out.
#[derive(Debug, Clone, Deserialize)]
pub struct BotAction {
    pub id: String,
    #[serde(default)]
    pub effects: Vec<Effect>,
    /// Hands-on: what the button does in the world (`None` = just effects / dismiss).
    #[serde(default)]
    pub handler: Option<BotHandler>,
}

/// An @shiftella_bot message template: `app.bot.<id>` with `{placeholders}`.
#[derive(Debug, Clone, Deserialize)]
pub struct BotTemplate {
    pub id: String,
    pub kind: BotKind,
    /// Placeholders the text uses (validated against every language).
    #[serde(default)]
    pub placeholders: Vec<String>,
    #[serde(default)]
    pub actions: Vec<BotAction>,
}

/// One line of a summary screen (digest or payroll): `app.<screen>.<id>`.
#[derive(Debug, Clone, Deserialize)]
pub struct SummaryLine {
    pub id: String,
    #[serde(default)]
    pub placeholders: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ShiftellaApp {
    pub bot_handle: String,
    pub announcements: Vec<Announcement>,
    pub checklists: Vec<ChecklistItem>,
    pub bot: Vec<BotTemplate>,
    /// Night digest lines (`app.digest.<id>`).
    pub digest: Vec<SummaryLine>,
    /// Payroll basis ("podklady pro mzdy") lines (`app.payroll.<id>`).
    pub payroll: Vec<SummaryLine>,
    /// Handover note templates (`app.handover.<id>`).
    pub handover: Vec<SummaryLine>,
    /// Hands-on: opening / closing rituals (`app.ritual.<list>`).
    #[serde(default)]
    pub rituals: Vec<Ritual>,
    /// Hands-on: one-tap handover suggestions with next-night effects.
    #[serde(default)]
    pub handover_options: Vec<HandoverOption>,
    /// Hands-on: digest insights with deep links (`app.insight.<id>`).
    #[serde(default)]
    pub insights: Vec<InsightDef>,
    /// Hands-on: the weekly payroll decision.
    #[serde(default)]
    pub payroll_rules: Option<PayrollRules>,
}

impl ShiftellaApp {
    pub fn bot_template(&self, id: &str) -> Option<&BotTemplate> {
        self.bot.iter().find(|b| b.id == id)
    }
}

// ---------------------------------------------------------------------------------------------
// Difficulty & sandbox
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum PaymentRule {
    /// Missing a payment applies `missed_payment_effects` only; the café never closes.
    Soft,
    /// Missing `close_after_missed` payments (or the final one) leads to the Last Train ending.
    Hard,
    /// Relaxed (no money pressure): an instalment is covered when it comes due — the debt
    /// shrinks, the till keeps its money, nothing can be missed.
    Forgiven,
}

/// `difficulty.<id>.name` / `.desc`.
#[derive(Debug, Clone, Deserialize)]
pub struct DifficultyDef {
    pub id: String,
    #[serde(default)]
    pub default: bool,
    pub start_money: i64,
    pub payments: PaymentRule,
    #[serde(default)]
    pub close_after_missed: Option<u32>,
    /// Applied when a payment is missed.
    #[serde(default)]
    pub missed_payment_effects: Vec<Effect>,
    pub guest_patience_mul: f32,
    pub bad_review_mul: f32,
    pub wear_mul: f32,
    pub need_decay_mul: f32,
    pub income_mul: f32,
    /// Sandbox rent growth: +percent every N nights (0 = none).
    pub rent_growth_percent: f32,
    pub rent_growth_every: u32,
    /// Sandbox: the rent stops growing at this multiple of the highest starting rent of a town
    /// that size (`sim::World::sandbox_rent_cap`; `None` = no cap). Without it +5 % every 10
    /// nights compounds past what a full café earns: every long game went broke by night 300.
    #[serde(default)]
    pub rent_growth_max: Option<f32>,
    /// Sandbox: nights in a row below zero before the café closes (`None` = never).
    #[serde(default)]
    pub bankrupt_after: Option<u32>,
}

impl DifficultyDef {
    pub fn name_key(&self) -> String {
        format!("difficulty.{}.name", self.id)
    }
    pub fn desc_key(&self) -> String {
        format!("difficulty.{}.desc", self.id)
    }
}

/// Sandbox "night character" slider step (`sandbox.character.<id>.name` / `.desc`).
#[derive(Debug, Clone, Deserialize)]
pub struct NightCharacter {
    pub id: String,
    /// Event chance per game minute before curve multipliers.
    pub base_rate: f32,
    pub max_tension: f32,
    /// Share of Good events among picks (0–1).
    pub good_share: f32,
    /// Minutes without new Bad events after one.
    pub breath_minutes: u32,
    pub max_bad_in_row: u32,
    pub uncanny_mul: f32,
    /// `lonely_night` safety valve after N hard nights (`None` = off).
    #[serde(default)]
    pub lonely_valve_after: Option<u32>,
    /// Good events ×2 when money is below one night of rent.
    pub poverty_help: bool,
    /// Max staff events per night (`None` = unlimited).
    #[serde(default)]
    pub staff_events_cap: Option<u32>,
    /// Expected events per night shown in the setup summary.
    pub events_per_night: (u32, u32),
}

#[derive(Debug, Clone, Deserialize)]
pub struct TownNameParts {
    /// Optional first word: "Horní", "Dolní", "Nová", "Stará".
    pub prefixes: Vec<String>,
    /// Chance of a prefix (0–1).
    pub prefix_chance: f32,
    pub roots: Vec<String>,
    pub suffixes: Vec<String>,
    /// Optional "nad <river>" tail.
    pub rivers: Vec<String>,
    pub river_chance: f32,
    /// Real town names that must not be generated.
    pub stoplist: Vec<String>,
    /// Real places (cities / city districts) a sandbox town is picked from. When non-empty
    /// the prefix/root/suffix generator is not used.
    #[serde(default)]
    pub places: Vec<String>,
}

/// `sandbox.institution.<id>` — enables guest pools.
#[derive(Debug, Clone, Deserialize)]
pub struct InstitutionDef {
    pub id: String,
    pub weight: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SandboxDef {
    pub characters: Vec<NightCharacter>,
    pub default_character: String,
    pub start_money_options: Vec<i64>,
    pub default_start_money: i64,
    pub town_names: TownNameParts,
    pub institutions: Vec<InstitutionDef>,
    /// How many institutions a generated town gets (inclusive).
    pub institutions_per_town: (u32, u32),
    /// Default café names (proper nouns, not translated).
    pub cafe_names: Vec<String>,
    /// Rent per night, CZK (inclusive range by town size).
    pub rent: (u32, u32),
    /// Defaults for the setup checkboxes.
    pub options: HashMap<String, bool>,
}

impl SandboxDef {
    pub fn character(&self, id: &str) -> Option<&NightCharacter> {
        self.characters.iter().find(|c| c.id == id)
    }
}
