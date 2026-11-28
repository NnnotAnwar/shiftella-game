//! State of the T5 systems (design/handson.md §4–§6, §11–§13): hands-on effects in flight,
//! night moments, the station cat, napkins and the tip jar, coziness / seasons / themes, the
//! polaroid journal, the rozpis puzzle and the app as a real tool. Everything is
//! `#[serde(default)]`, so saves of version 2 made before T5 load unchanged.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::types::AgentRef;
use crate::content::Job;
use crate::content::handson::logic::CameraTarget;
use crate::content::handson::{
    AppLink, Fx, HighlightKind, Pref, PrefKind, RequestKind, Taste, TipSplit,
};

// ---------------------------------------------------------------------------------------------
// Effects in flight
// ---------------------------------------------------------------------------------------------

/// Who / what a hands-on effect list is about: the moment's anchored agent, the subject of a
/// handover note or announcement.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct XTarget {
    /// The guest party (target guest effects).
    pub party: Option<u32>,
    /// The helper (target staff effects).
    pub staff: Option<u32>,
    /// The anchor tile (near-guest effects, buckets, camera hints).
    pub tile: Option<(i32, i32)>,
    /// The anchor furniture (a table, a window lamp, the radio).
    pub furniture: Option<u32>,
    /// The moment that ran the effects (journal captions).
    pub moment: Option<String>,
    /// A regular (handover "remember", requested records).
    pub regular: Option<String>,
    /// A menu item (spotlight, low stock, menu off).
    pub item: Option<String>,
    /// A record (the regular's request).
    pub record: Option<String>,
    /// A train's arrival minute (handover "busy after the train").
    pub train_minute: Option<u32>,
}

/// A hands-on effect list waiting to run (`Later`, `NextNight`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingFx {
    /// Game minute (`Later`); unused for `NextNight`.
    pub due: u32,
    pub effects: Vec<Fx>,
    pub target: XTarget,
}

/// Tonight's hands-on modifiers (all cleared at night start).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FxState {
    pub later: Vec<PendingFx>,
    pub next_night: Vec<PendingFx>,
    /// (add, until minute).
    pub quick_quality: Vec<(f32, Option<u32>)>,
    pub coziness: Vec<(f32, Option<u32>)>,
    /// Guest patience × (mul, until).
    pub patience: Vec<(f32, Option<u32>)>,
    /// Wish weights × mul for wishes wanting any of the tags.
    pub wish_boosts: Vec<(Vec<Taste>, f32, Option<u32>)>,
    /// "The machine is cold": the first `count` items of `job` take × mul.
    pub slow_first: Vec<(Job, u32, f32)>,
    /// Candles on the tables while the power is out.
    pub candles: bool,
    /// The radio: (track, volume, until minute).
    pub radio: Option<(String, f32, u32)>,
    pub street_music_until: Option<u32>,
    pub window_open_until: Option<u32>,
    /// A "we're out of X" moment later tonight: (minute, minutes off).
    pub stockout: Option<(u32, u32)>,
    pub stock_checked: bool,
    pub cashed_up: bool,
    /// Checklist work × mul tonight (`[Opening, Closing]`).
    pub checklist_speed: [f32; 2],
    /// Helpers wait at their stations from these minutes (handover "busy after the train").
    pub helpers_ready: Vec<u32>,
    /// The spotlight item tonight (announcement `new_item`): prep × mul.
    pub spotlight: Option<(String, f32)>,
    /// Regulars with a "remember" mark tonight and their extra trust when served.
    pub remember: BTreeMap<String, f32>,
    /// Open swaps are more likely to be picked up by colleagues tonight.
    pub swap_board: bool,
    /// The stock was counted by hand: the low-stock warning comes earlier tonight.
    pub stock_watch: bool,
    /// Beauty added by fixed tables tonight (coziness beauty).
    pub table_beauty: f32,
    /// The trainspotting logbook (LATER): train ids logged.
    pub logbook: Vec<(u32, String)>,
}

impl FxState {
    pub(crate) fn tonight_reset(&mut self) {
        let next = std::mem::take(&mut self.next_night);
        let logbook = std::mem::take(&mut self.logbook);
        *self = FxState {
            next_night: next,
            logbook,
            checklist_speed: [1.0, 1.0],
            ..Default::default()
        };
    }
}

// ---------------------------------------------------------------------------------------------
// Moments
// ---------------------------------------------------------------------------------------------

/// Where a moment's glowing marker sits (world tile; the furniture or agent it is about).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MomentPos {
    pub tile: (i32, i32),
    pub furniture: Option<u32>,
    pub agent: Option<AgentRef>,
}

/// A moment on the map (`World::active_moments`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveMoment {
    pub id: String,
    pub anchor: MomentPos,
    /// Game minute it appeared.
    pub spawned: u32,
    /// Game minute it fades (`on_expire` applies).
    pub expires: u32,
    pub target: XTarget,
    /// The option Karolina is walking over to apply (the card closed at once).
    pub resolving: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MomentState {
    pub active: Vec<ActiveMoment>,
    /// Moments that appeared tonight (budgeted ones; follow-ups excluded).
    pub tonight: u32,
    /// Every id that appeared tonight (per-night limits).
    pub tonight_ids: Vec<String>,
    pub last_minute: Option<u32>,
    /// Tonight's budget, rolled at night start, and the night's cap (the lull director may
    /// add up to it).
    pub budget: u32,
    pub cap: u32,
    /// Moment id → (times appeared, last night).
    pub history: BTreeMap<String, (u32, u32)>,
    pub resolved_ever: BTreeSet<String>,
    /// Event moments waiting for room (nights 1–3: never two at once): (id, target, until).
    pub waiting: Vec<(String, XTarget, u32)>,
    /// What Karolina does when a queued `GoTo` arrives (moments, night experiments), FIFO by
    /// tile; the one she is doing now.
    pub walks: Vec<WalkJob>,
    pub walk_now: Option<WalkJob>,
}

/// Work at the end of one of Karolina's `GoTo` acts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WalkJob {
    pub tile: (i32, i32),
    pub kind: WalkKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WalkKind {
    /// A moment's option (`Command::ResolveMoment`).
    Moment { id: String, option: String },
    /// A pantry experiment at the counter (`Command::Experiment` at night).
    Experiment { ingredients: Vec<String> },
    /// An answer to a stranger's little story (`Command::AnswerStory`).
    Story { guest: u32, answer: String },
    /// A storm chore: `blankets` / `soup` (`Command::StormBlankets` / `StormSoup`).
    Storm { act: String },
}

impl WalkKind {
    /// Queue icon hint for the UI (`moment` / `experiment` / `story` / `storm`).
    pub fn id(&self) -> &'static str {
        match self {
            WalkKind::Moment { .. } => "moment",
            WalkKind::Experiment { .. } => "experiment",
            WalkKind::Story { .. } => "story",
            WalkKind::Storm { .. } => "storm",
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Extras: cat, napkins, tip jar
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ExtrasState {
    /// The station cat's friendship level (`extras.ron` → cat levels).
    pub cat_level: u32,
    /// The cat is in the café tonight (it came by or lives here).
    pub cat_here: bool,
    /// Napkins collected, in order (`thanks.<id>`).
    pub napkins: Vec<String>,
    /// Napkin rewards already given (counts).
    pub napkin_rewards: BTreeSet<u32>,
    /// Lifetime tip goals reached (indexes into `extras.ron` → tip_goals).
    pub tip_goals: BTreeSet<u32>,
    /// A goal waiting for "Count the jar" → pick one of two (`Command::PickTipGoal`).
    pub tip_goal_pending: Option<u32>,
    /// Tonight's specials board (recipes; match +0.1).
    pub specials: Vec<String>,
}

// ---------------------------------------------------------------------------------------------
// Atmosphere: coziness, seasons, themes
// ---------------------------------------------------------------------------------------------

/// The coziness meter (`World::coziness`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Coziness {
    /// 0–100.
    pub value: f32,
    /// Meter level 1..=5 (`atmo.level.<n>`).
    pub level: u32,
    /// Points per component (`CozyComponent::ALL` order), each out of its weight.
    pub parts: [f32; 7],
    /// The component furthest below its weight (`atmo.tip.<id>`), index into `CozyComponent::ALL`.
    pub weakest: usize,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AtmosState {
    pub coziness: Coziness,
    /// Tonight's coziness sum / samples (digest insight).
    pub night_sum: f32,
    pub night_n: u32,
    pub like_home_ever: bool,
    /// The hall's decor theme (`decor.ron`; empty = the default).
    pub theme: String,
    /// Themes bought or given (switch freely by day).
    pub owned: BTreeSet<String>,
    /// Themes whose unlock holds (may be bought).
    pub unlocked: BTreeSet<String>,
    /// The season phase of the current night (`SeasonPhaseChanged` on change).
    pub phase: Option<String>,
    /// Latte patterns unlocked by seasons, the cat, moments.
    pub lattes: BTreeSet<String>,
    /// Regulars who got tonight's theme trust (`RegularTrustOnEnter`).
    pub theme_trust_tonight: BTreeSet<String>,
}

// ---------------------------------------------------------------------------------------------
// Journal
// ---------------------------------------------------------------------------------------------

/// A tile of a snapshot (render `SnapTile`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SnapTileData {
    #[default]
    Void,
    /// Floor id (`floor_wood`, `floor_tile`, `floor_carpet`, `pavement`).
    Floor(String),
    Wall,
    Window,
    Door(String),
}

/// A furniture piece in a snapshot (render `SnapProp`: `sprite` from the def).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapPropData {
    pub def: String,
    pub sprite: String,
    pub tile: (i32, i32),
    pub size: (u8, u8),
    pub facing: super::types::Facing,
    pub on: bool,
    pub dirty: f32,
}

/// The face in a snapshot (render `Expression`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SnapExpression {
    #[default]
    Neutral,
    Smile,
    Sad,
    Surprised,
    Thinking,
    Laugh,
    Tired,
}

/// A person in a snapshot (render `SnapActor`; map `look` with `PersonLook` as for views).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapActorData {
    pub look: super::types::LookRef,
    /// World px.
    pub feet: (f32, f32),
    pub facing: super::types::Facing,
    pub pose: super::types::Pose,
    pub expression: SnapExpression,
    pub emote: Option<super::types::SimEmote>,
    pub seat_tile: Option<(i32, i32)>,
    pub holding: Option<String>,
}

/// A served cup (render `SnapCup`): the recipe's `CupGlyph`, the latte pattern.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapCupData {
    pub pos: (f32, f32),
    pub item: String,
    pub recipe: Option<String>,
    pub latte: Option<String>,
}

/// A compact scene snapshot the render's `SceneSnap` is built from (≈ 1–3 KB in a save).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SceneSnapData {
    /// Top-left tile of the window.
    pub origin: (i32, i32),
    /// Window size in tiles (`journal.ron` → snap_tiles).
    pub size: (u8, u8),
    /// Row-major `size.0 × size.1`.
    pub tiles: Vec<SnapTileData>,
    pub props: Vec<SnapPropData>,
    pub actors: Vec<SnapActorData>,
    pub cups: Vec<SnapCupData>,
    /// The station cat (world px), if it was there.
    pub cat: Option<(f32, f32)>,
    /// Night progress 0–1.
    pub night_progress: f32,
    /// Weather id (`clear`, `rain`, `downpour`, …) and rain strength 0–2.
    pub weather: String,
    pub rain: f32,
    /// Season phase id (`seasons.ron`) and theme id (`decor.ron`).
    pub season: String,
    pub theme: String,
    pub garlands: bool,
    pub snow: bool,
    pub blackout: bool,
    pub window_light: bool,
    pub coziness: f32,
    /// A train passing.
    pub train: bool,
    /// Grain / tilt seed.
    pub seed: u32,
    /// World px the camera looks at.
    pub focus: (f32, f32),
    /// 1 = the whole window, 1.5 = a closer portrait.
    pub zoom: f32,
}

/// A journal candidate (handson.md §11.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Highlight {
    pub night: u32,
    pub minute: u32,
    pub kind: HighlightKind,
    pub score: u32,
    /// `journal.caption.*` and its placeholders.
    pub caption_key: String,
    pub args: Vec<(String, String)>,
    pub snap: SceneSnapData,
    /// A before / after polaroid: the hall as it was, same window (`snap` is the after).
    #[serde(default)]
    pub before: Option<SceneSnapData>,
}

/// A kept polaroid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalEntry {
    pub night: u32,
    /// Story calendar date (year, month, day), if any.
    pub date: Option<(i32, u32, u32)>,
    pub highlight: Highlight,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct JournalState {
    /// Tonight's best candidates (top `keep_per_night`).
    pub tonight: Vec<Highlight>,
    /// The morning's choice: (night, candidates best first) until `PickPolaroid`.
    pub pending: Option<(u32, Vec<Highlight>)>,
    pub entries: Vec<JournalEntry>,
    /// Kinds kept last night (−penalty for variety).
    pub last_kinds: Vec<HighlightKind>,
    /// One-time kinds already captured (`like_home`, `guest_333`, …).
    pub firsts: BTreeSet<String>,
    /// Night-once kinds captured tonight (full house…).
    pub tonight_kinds: BTreeSet<String>,
    /// A day-phase highlight (rozpis happy) to capture at the next night start.
    pub deferred: Vec<(HighlightKind, Vec<(String, String)>)>,
    /// Arrivals photographed once the guest sits down (the 3:33 guest, the cameo): at the
    /// door they'd be a dark figure at the edge of the frame; an arc beat on arrival, too.
    /// (guest id, kind, sub-caption)
    pub on_seat: Vec<(u32, HighlightKind, Option<String>)>,
}

// ---------------------------------------------------------------------------------------------
// Rozpis puzzle
// ---------------------------------------------------------------------------------------------

/// How one preference fares on a rozpis row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrefStatus {
    Honoured,
    Violated,
    /// Not decidable (e.g. the colleague isn't employed).
    Pending,
}

/// A request the rozpis created from a preference or a life event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestMeta {
    pub kind: RequestKind,
    pub staff: u32,
    /// `app.request.<id>` reason line (life events), if any.
    pub reason_key: Option<String>,
    /// Created because a Need would be broken.
    pub need: bool,
    /// The night it is about.
    pub night: u32,
}

/// "Can I go at 2?" (handson.md §13.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EarlyLeaveRequest {
    pub id: u32,
    pub staff: u32,
    pub night: u32,
    /// Minute since opening they would leave.
    pub minute: u32,
    pub status: super::types::RequestStatus,
}

/// One night of the coverage footer.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CoverageNight {
    pub weekday: u8,
    /// Helpers `weekday_demand` suggests.
    pub suggested: u32,
    /// Helpers on (any shift) in the draft.
    pub on: u32,
    /// Busy-ness 1–3 cups.
    pub cups: u8,
}

/// The auto-fill result line (`rozpis.autofill.result` {kept} {total}).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AutofillReport {
    pub kept: u32,
    pub total: u32,
    /// Wishes / Needs that didn't fit: (staff, pref).
    pub broken: Vec<(u32, PrefKind)>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RozpisState {
    /// Preferences per person (story people from `staff.ron`, generated people from traits).
    pub prefs: BTreeMap<u32, Vec<Pref>>,
    /// One-week preferences from life events: (pref, week).
    pub temp: BTreeMap<u32, Vec<(Pref, u32)>>,
    /// Wellbeing bonus per person this week: (add, week).
    pub bonus: BTreeMap<u32, Vec<(f32, u32)>>,
    /// Live wellbeing on the draft, and the week's fairness.
    pub wellbeing: BTreeMap<u32, f32>,
    pub fairness: f32,
    /// Wellbeing of the published week (feeds tonight's effects).
    pub published_wb: BTreeMap<u32, f32>,
    /// (archetype a, archetype b) sorted → shifts shared (≥ 4 h together).
    pub shared: BTreeMap<(String, String), u32>,
    /// Declined requests per person (nights).
    pub declines: BTreeMap<u32, Vec<u32>>,
    /// A happy week (every pref honoured, coverage fine): its week number.
    pub happy_week: Option<u32>,
    pub requests: BTreeMap<u32, RequestMeta>,
    pub early_leave: Vec<EarlyLeaveRequest>,
    /// Life events already started.
    pub life_events: BTreeSet<String>,
    pub last_report: AutofillReport,
    /// Someone declined a Need: they come in late tonight.
    pub late_tonight: BTreeSet<u32>,
}

// ---------------------------------------------------------------------------------------------
// The app as a tool
// ---------------------------------------------------------------------------------------------

/// A handover suggestion at closing (`app.handover.option.<id>` + args).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HandoverSuggestion {
    pub id: String,
    pub args: Vec<(String, String)>,
    pub target: XTarget,
}

/// A digest insight (`app.insight.<id>`) with its deep link.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Insight {
    pub key: String,
    pub args: Vec<(String, String)>,
    pub link: AppLink,
}

/// A soft reminder chip (`RemindBefore`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reminder {
    pub minute: u32,
    pub key: String,
    pub args: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ToolState {
    /// Remembered tip split (`None` = the content default).
    pub tip_split: Option<TipSplit>,
    /// Weeks booked (`SendPayroll`, or auto at the next day start).
    pub payroll_sent: BTreeSet<u32>,
    /// Star-of-the-week bonus chosen for a week: (week, staff).
    pub bonus: Option<(u32, u32)>,
    /// Crew mood / wellbeing from last week's tip split: (mood, wb, week it applies).
    pub split_effect: Option<(f32, f32, u32)>,
    /// Tonight's handover suggestions (computed at closing) and Karolina's picks.
    pub suggestions: Vec<HandoverSuggestion>,
    pub picks: Vec<String>,
    /// Notes written for the next night: (option id, target).
    pub next_notes: Vec<(String, XTarget)>,
    /// Rituals done tonight (`[Opening, Closing]`).
    pub rituals: [bool; 2],
    /// Last night's closing ritual: tonight's opening is quicker.
    pub closed_properly: bool,
    pub reminders: Vec<Reminder>,
    /// Reserved seats: regular → seat furniture uid.
    pub reserved: BTreeMap<String, u32>,
    /// A late helper's duty covered by another: (late staff, duty, cover staff).
    pub covers: Vec<(u32, crate::content::handson::Duty, u32)>,
    /// Drone deliveries arriving at these minutes.
    pub drones: Vec<u32>,
    /// Camera hints and app links for the UI are events; the last one is kept for tests.
    pub last_camera: Option<CameraTarget>,
    /// Moments a day-phase bot button asked for (spawned early tonight).
    pub moments_tonight: Vec<String>,
    /// Tonight's tallies for the digest insights.
    pub rush_at: Option<u32>,
    pub hour_arrivals: [u32; 8],
    /// Wanted tastes a served wish didn't get (not-quite / okay serves).
    pub unmet: BTreeMap<Taste, u32>,
    /// Trust at night start (the digest's "trust" insight).
    pub trust_start: BTreeMap<String, f32>,
    /// The best wish of the night: (regular, recipe, match).
    pub loved: Option<(String, String, f32)>,
    pub low_stock_sent: bool,
}

// ---------------------------------------------------------------------------------------------
// All of it
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SystemsState {
    /// Content-derived state (cat var, themes, prefs) was initialised.
    pub ready: bool,
    pub fx: FxState,
    pub moments: MomentState,
    pub extras: ExtrasState,
    pub atmos: AtmosState,
    pub journal: JournalState,
    pub rozpis: RozpisState,
    pub tool: ToolState,
    /// Brews started with `PlayerBrewStart` still waiting for their quality: (item, tick).
    pub brew_waiting: Vec<(u32, u32)>,
    /// Tutorial steps whose `on_show` ran; the tutorial setting (off = none).
    pub tutorial_shown: BTreeSet<String>,
    pub tutorial_off: bool,
}
