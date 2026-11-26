//! `World`: the whole game state, constructors, the tick loop, phases, queries and saves.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use super::grid::{self, Grid, PathCache, Placed};
use super::hands::*;
use super::rng::{Rng, hash_str};
use super::types::*;
use crate::content::handson::Duty;
use crate::content::{ClockTime, Content, Job, Weather, Weekday};

/// Save format version. 2 = the hands-on update (Karolina, duties, wishes, jukebox); version 1
/// saves still load (`World::load_from_str` migrates them).
pub const SAVE_VERSION: u32 = 2;
pub const TICKS_PER_MINUTE: u32 = 10;
pub const NIGHT_MINUTES: u32 = 480;
pub const NIGHT_TICKS: u32 = NIGHT_MINUTES * TICKS_PER_MINUTE;
/// The quiet half hour before the doors open (21:30–22:00): Karolina and the early crew get
/// the café ready, no guests come in. The night clock (`minute()`, `tick_count()`) stands at
/// 0 meanwhile; `minutes_since_open()` / `clock()` show the real 21:30→22:00 time.
pub const PREP_MINUTES: u32 = 30;
pub const PREP_TICKS: u32 = PREP_MINUTES * TICKS_PER_MINUTE;
/// Helpers on tonight's rozpis who start at 22:00 walk in during the preparation from this
/// many prep minutes (21:38) on, a couple of minutes apart (all in by ~21:45).
pub const PREP_CREW_ARRIVES: u32 = 8;
/// Story café name (proper noun, never translated; the sign's subtitle "kavárna u nádraží"
/// is UI text).
pub const STORY_CAFE: &str = "Café Nocturne";
/// Story setting: the café is next to this station in Prague.
pub const STORY_TOWN: &str = "Praha-Holešovice";
/// Story institutions around Praha-Holešovice (guest pools besides the always-present ones):
/// the taxi rank by the station, the hospital (Bulovka) shift change, the college campus.
pub const STORY_INSTITUTIONS: &[&str] = &["taxi_rank", "hospital", "college"];
/// Where `DEFAULT_LAYOUT` puts grandma's jukebox (`j`).
pub const JUKEBOX_TILE: (i32, i32) = (28, 8);
/// Story nightly utilities (the story's "rent" is the debt payments).
pub const STORY_UTILITIES: i64 = 350;

// ---------------------------------------------------------------------------------------------
// State structs
// ---------------------------------------------------------------------------------------------

/// What a staff member is doing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Task {
    /// Take a party's order at a register (`Some(uid)`) or at their table (`None`).
    TakeOrder {
        party: u32,
        register: Option<u32>,
    },
    Prepare {
        item: u32,
        station: u32,
    },
    /// `extra` = more items for the same party carried on the tray.
    Serve {
        item: u32,
        picked: bool,
        extra: Vec<u32>,
    },
    ClearTable {
        table: u32,
        picked: bool,
        dishes: u32,
    },
    Wash {
        sink: u32,
    },
    Mop {
        tile: (i32, i32),
    },
    Checklist {
        id: String,
        tile: (i32, i32),
    },
    Bucket {
        tile: (i32, i32),
    },
    Break {
        until: u32,
        eat: bool,
    },
    Idle {
        tile: (i32, i32),
    },
    Leave,
}

impl Task {
    pub fn job(&self) -> Option<Job> {
        match self {
            Task::TakeOrder { .. } => Some(Job::Register),
            Task::Serve { .. } => Some(Job::Serving),
            Task::ClearTable { .. }
            | Task::Wash { .. }
            | Task::Mop { .. }
            | Task::Bucket { .. } => Some(Job::Cleaning),
            _ => None,
        }
    }
    /// Short id for UI (`ui.task.<id>` suggested).
    pub fn id(&self) -> &'static str {
        match self {
            Task::TakeOrder { .. } => "take_order",
            Task::Prepare { .. } => "prepare",
            Task::Serve { .. } => "serve",
            Task::ClearTable { .. } => "clear_table",
            Task::Wash { .. } => "wash",
            Task::Mop { .. } => "mop",
            Task::Checklist { .. } => "checklist",
            Task::Bucket { .. } => "bucket",
            Task::Break { .. } => "break",
            Task::Idle { .. } => "idle",
            Task::Leave => "leave",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StaffStatus {
    /// Not working tonight (schedule off, vacation, or day phase).
    OffDuty,
    /// Will arrive at `minute` (late arrivals / late shift).
    Expected {
        minute: u32,
    },
    OnDuty,
    /// Walking out at shift end.
    Leaving,
    Sick,
    WalkedOut,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Staff {
    pub id: u32,
    /// Proper noun (not translated). Empty for the owner (use `cast.player.name`).
    pub name: String,
    /// Staff archetype id, or `"owner"` for the player.
    pub archetype: String,
    pub owner: bool,
    /// The fixed story person of the archetype.
    pub story_person: bool,
    pub age: u32,
    pub female: bool,
    pub traits: Vec<String>,
    /// Skills 0–10 indexed by `Job::index()`.
    pub skills: [f32; 5],
    /// CZK per 8 h night.
    pub wage: u32,
    /// 0 off, 1 (highest) … 4, indexed by `Job::index()`.
    pub priorities: [u8; 5],
    pub energy: f32,
    pub mood: f32,
    pub hunger: f32,
    pub look_seed: u64,
    /// Osobní číslo for the payroll export.
    pub personal_number: u32,
    pub hired_night: u32,
    pub status: StaffStatus,
    pub pos: (f32, f32),
    pub facing: Facing,
    pub pose: Pose,
    pub path: Vec<(i32, i32)>,
    pub task: Option<Task>,
    pub work_left: f32,
    pub shift: Shift,
    pub leave_at: u32,
    pub late_minutes: u32,
    pub minutes_worked: u32,
    pub tasks_done: u32,
    pub hours_week: f32,
    /// Náhradní volno balance (hours).
    pub toil_hours: f32,
    /// Remaining paid vacation days.
    pub vacation_days: u32,
    /// (job index, add, last night).
    pub skill_boost: Option<(u8, f32, u32)>,
    pub romance_with: Option<u32>,
    pub romance_ended_night: Option<u32>,
    pub emote: Option<(SimEmote, u32)>,
    pub carrying: Option<String>,
    pub stuck_ticks: u32,
    pub breaks_tonight: u32,
    pub met_333: bool,
    /// Night the staff is on approved leave.
    pub leave_nights: Vec<(u32, LeaveKind)>,
    /// Duties the player delegated (helpers only; Karolina has none). Capacity 1–3
    /// (`World::capacity`). `priorities` is kept in sync (duty on → 1, off → 0).
    #[serde(default)]
    pub duties: Vec<Duty>,
    /// Shifts worked so far (capacity growth, `ShiftsWorked`).
    #[serde(default)]
    pub nights_worked: u32,
    /// Tasks finished tonight per duty (`Duty::ALL` order) — the morning digest credits.
    #[serde(default)]
    pub duty_done: [u32; 6],
}

impl Staff {
    pub fn tile(&self) -> (i32, i32) {
        grid::px_to_tile(self.pos)
    }
    pub fn present(&self) -> bool {
        matches!(self.status, StaffStatus::OnDuty | StaffStatus::Leaving)
    }
    pub fn on_duty(&self) -> bool {
        self.status == StaffStatus::OnDuty
    }
    pub fn has_trait(&self, t: &str) -> bool {
        self.traits.iter().any(|x| x == t)
    }
    pub fn skill(&self, job: Job, night: u32) -> f32 {
        let mut s = self.skills[job.index()];
        if let Some((j, add, until)) = self.skill_boost
            && j as usize == job.index()
            && night <= until
        {
            s += add;
        }
        s.clamp(0.0, 10.0)
    }
}

/// Someone you can hire.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Candidate {
    pub id: u32,
    pub staff: Staff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartyState {
    /// Walking in (to the queue or to the seats).
    Arriving,
    /// Leader stands in the register queue.
    Queue,
    /// Seated, waiting for someone to take the order at the table.
    WaitOrder,
    WaitFood,
    Eating,
    Leaving,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Party {
    pub id: u32,
    pub archetype: String,
    pub regular: Option<String>,
    pub members: Vec<u32>,
    pub state: PartyState,
    pub arrived: u32,
    pub ordered_at: Option<u32>,
    pub served_at: Option<u32>,
    /// Minutes of patience left in the current wait.
    pub patience: f32,
    pub patience_max: f32,
    pub wishes: Vec<String>,
    pub items: Vec<u32>,
    pub satisfaction: f32,
    pub spend_mul: f32,
    pub register: Option<u32>,
    pub order_claim: Option<u32>,
    pub stay_left: f32,
    pub bill: i64,
    pub served_by: Option<u32>,
    /// Leaves the café no later than this minute (regular visit windows).
    pub leave_by: Option<u32>,
    pub served: bool,
    pub angry: bool,
    /// What the leader ordered (wish parties are Karolina's).
    #[serde(default)]
    pub order_kind: OrderKind,
    /// The leader's wish still has no item (taken, waiting for Karolina's recipe).
    #[serde(default)]
    pub wish_open: bool,
    /// Karolina handed the wish off to the helpers (match capped, no story credit).
    #[serde(default)]
    pub handed_off: bool,
    /// Wait clock (game minutes, scaled: ×0.5 while Karolina is on it, ×0.67 Cozy rush).
    #[serde(default)]
    pub wait_clock: f32,
    /// Wait clock when the order was taken (`ServedFastStreak` counts from here).
    #[serde(default)]
    pub wait_at_order: f32,
    /// Cozy: patience ran out this many times (a glass of water, then they leave content).
    #[serde(default)]
    pub forgiven: u32,
    #[serde(default)]
    pub greeted: bool,
    /// Tip factor of every served item (Karolina's match & quality, rhythm).
    #[serde(default)]
    pub tip_factors: Vec<f32>,
    /// Game minutes the current record has played while they were inside.
    #[serde(default)]
    pub record_minutes: u32,
    /// A record reaction (like / dislike) happened this visit.
    #[serde(default)]
    pub music_reacted: bool,
    /// Archetype music like / dislike applied this visit.
    #[serde(default)]
    pub archetype_music_done: bool,
    /// A thank-you napkin they will leave with the tip (`thanks.<id>`).
    #[serde(default)]
    pub napkin: Option<String>,
}

impl Party {
    /// The leader (wish guest).
    pub fn leader(&self) -> Option<u32> {
        self.members.first().copied()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Guest {
    pub id: u32,
    pub party: u32,
    pub pos: (f32, f32),
    pub path: Vec<(i32, i32)>,
    pub facing: Facing,
    pub pose: Pose,
    /// (furniture uid, seat index).
    pub seat: Option<(u32, u8)>,
    pub look_seed: u64,
    pub preset: Option<String>,
    pub source: String,
    pub leader: bool,
    pub holding: Option<String>,
    pub emote: Option<(SimEmote, u32)>,
    /// Final destination tile (queue slot, seat, exit).
    pub goal: Option<(i32, i32)>,
    pub gone: bool,
    pub stuck_ticks: u32,
}

impl Guest {
    pub fn tile(&self) -> (i32, i32) {
        grid::px_to_tile(self.pos)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemState {
    Pending,
    Preparing,
    Ready,
    Carrying,
    Served,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderItem {
    pub id: u32,
    pub party: u32,
    pub guest: u32,
    pub menu: String,
    pub state: ItemState,
    pub prep_left: f32,
    pub by: Option<u32>,
    pub ready_tile: Option<(i32, i32)>,
    pub quality: f32,
    pub created: u32,
    /// Who made it (Karolina's recipe & quality, or a helper).
    #[serde(default)]
    pub origin: ItemOrigin,
    /// Recipe surcharge on top of the menu price (Karolina's specials).
    #[serde(default)]
    pub price_add: u32,
    /// The leader's wish handed off to the helpers: this capped match applies when served.
    #[serde(default)]
    pub hand_off_match: Option<f32>,
    /// This is the leader's wish (made by Karolina, or handed off).
    #[serde(default)]
    pub wish: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModKind {
    GuestRate,
    Tips,
    DirtRate,
    /// Job index (`None` = all jobs).
    WorkSpeed(Option<u8>),
    StationDown(String),
    StationSlow(String),
    MenuUnavailable(String),
    /// Relationship per hour for everyone on shift.
    Relationship,
    PowerOut,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Modifier {
    pub kind: ModKind,
    pub mul: f32,
    /// Minute since opening when it ends (`None` = end of night).
    pub until: Option<u32>,
    pub event: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VisitRecord {
    pub night: u32,
    pub served_minutes: Option<u32>,
    pub seat_free: bool,
    pub clean: f32,
    /// Best match of a Karolina-made item this visit (`WishMatchStreak`).
    #[serde(default)]
    pub best_match: Option<f32>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RegularState {
    pub visits: Vec<VisitRecord>,
    pub served_items: BTreeSet<String>,
    pub advanced_night: u32,
    pub party: Option<u32>,
    /// Visit-window indexes already spawned tonight.
    pub spawned: Vec<usize>,
    /// Recipes Karolina has served them (`ServedRecipe`).
    #[serde(default)]
    pub served_recipes: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StoryState {
    pub flags: BTreeSet<String>,
    pub vars: BTreeMap<String, String>,
    pub counters: BTreeMap<String, i32>,
    pub items: BTreeSet<String>,
    /// Notes in the order found.
    pub notes: Vec<String>,
    pub trust: BTreeMap<String, f32>,
    pub regulars: BTreeMap<String, RegularState>,
    pub played: BTreeSet<String>,
    pub queue: VecDeque<String>,
    pub playing: Option<String>,
    pub ending: Option<String>,
    pub ending_variant: Option<String>,
    pub unlocked_menu: BTreeSet<String>,
    pub unlocked_furniture: BTreeSet<String>,
    pub beats_done: BTreeSet<usize>,
    /// (night, amount, paid).
    pub payments: Vec<(u32, i64, bool)>,
    /// Last night whose payment was collected (paid or missed).
    pub payments_processed: u32,
    /// Talk cutscenes played (also in `played`).
    pub talks: BTreeSet<String>,
}

/// How a pending `Later` effect list is found again in content (effects aren't serialisable).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EffectSrc {
    Event(String),
    EventChoice(String, String),
    Checklist(String, bool),
    Beat(u32, usize),
    Note(String),
    Arc(String, u32),
    MissedPayment(String),
    BotAction(String, String),
    /// Cutscene step path (`(index, branch)` through `If` steps) + choice option.
    Cutscene(String, Vec<(u16, u8)>, Option<u16>),
    /// Not addressable: `Later` inside runs immediately.
    None,
}

/// Who triggered an effect list (pair for quarrels, a person for `SkillBoost`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EffectCtx {
    pub pair: Option<(u32, u32)>,
    pub staff: Option<u32>,
    pub event: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingLater {
    pub due: u32,
    pub src: EffectSrc,
    /// Path inside the root list to the `Later` effect's inner list.
    pub path: Vec<(u16, u8)>,
    pub ctx: EffectCtx,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingChoice {
    pub event: String,
    pub choices: Vec<String>,
    pub default: Option<String>,
    pub expires: u32,
    pub ctx: EffectCtx,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NarratorState {
    pub tension: f32,
    pub last_bad: Option<u32>,
    pub bad_in_row: u32,
    /// Event id → last night fired.
    pub last_fired: BTreeMap<String, u32>,
    pub fired_tonight: Vec<String>,
    pub hard_nights: u32,
    pub staff_events: u32,
    pub bad_tonight: u32,
    pub good_tonight: u32,
    pub random_tonight: u32,
    /// (event, mul, last night).
    pub weights: Vec<(String, f32, u32)>,
    pub pending: Vec<PendingChoice>,
    /// (event, until minute) — `EventActive`.
    pub active: Vec<(String, u32)>,
    pub later: Vec<PendingLater>,
    /// (guest archetype filter, mult, reviews left).
    pub review_weights: Vec<(Option<String>, f32, u32)>,
    /// Foresight: (event, minute it fires).
    pub foreseen: Vec<(String, u32)>,
    /// Auto events: last minute fired tonight.
    pub auto_last: BTreeMap<String, u32>,
    pub peak_tension: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwapRequest {
    pub id: u32,
    pub from: u32,
    pub to: u32,
    pub weekday: u8,
    pub status: RequestStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeOffRequest {
    pub id: u32,
    pub staff: u32,
    pub kind: LeaveKind,
    /// Story night the leave applies to.
    pub night: u32,
    pub hours: f32,
    pub status: RequestStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Handover {
    pub night: u32,
    /// (text key `app.handover.<id>`, args).
    pub lines: Vec<(String, Vec<(String, String)>)>,
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostedAnnouncement {
    pub id: String,
    pub author: String,
    pub night: u32,
    pub args: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChecklistState {
    pub id: String,
    pub done: bool,
    pub by: Option<u32>,
    pub minute: Option<u32>,
    pub claim: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BotMsg {
    pub id: u32,
    pub template: String,
    pub args: Vec<(String, String)>,
    pub night: u32,
    pub minute: Option<u32>,
    /// Action chosen (buttons collapse afterwards).
    pub answered: Option<String>,
    /// Linked swap / time-off request id.
    pub request: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PayrollRow {
    pub personal_number: u32,
    pub staff: u32,
    pub name: String,
    pub hours: f32,
    pub night_hours: f32,
    pub wage: i64,
    pub vacation_hours: f32,
    pub toil_hours: f32,
}

/// Weekly "podklady pro mzdy".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PayrollWeek {
    pub week: u32,
    pub rows: Vec<PayrollRow>,
    pub tips: i64,
    pub total_hours: f32,
    pub total: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AppState {
    pub draft: BTreeMap<u32, [Shift; 7]>,
    pub published: BTreeMap<u32, [Shift; 7]>,
    pub draft_dirty: bool,
    /// The player edited the rozpis by hand (`SetShift`); until then the app keeps the
    /// default rozpis current (`Command::AutoSchedule` hands it back to the app).
    #[serde(default)]
    pub manual_rozpis: bool,
    pub week: u32,
    pub swaps: Vec<SwapRequest>,
    pub time_off: Vec<TimeOffRequest>,
    pub handovers: Vec<Handover>,
    pub handover_text: Option<String>,
    pub announcements: Vec<PostedAnnouncement>,
    pub checklist: Vec<ChecklistState>,
    pub inbox: Vec<BotMsg>,
    pub payroll: Vec<PayrollWeek>,
    pub week_tips: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Economy {
    pub money: i64,
    pub debt: i64,
    pub reputation: f32,
    /// Ingredient stock value, CZK.
    pub supplies: i64,
    pub auto_restock: Option<i64>,
    pub prices: BTreeMap<String, u32>,
    pub menu_off: BTreeSet<String>,
    pub rent: i64,
    pub nights_in_red: u32,
    /// Day-phase purchases, folded into the next night's ledger.
    pub day_supplies: i64,
    pub sold: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Town {
    pub name: String,
    pub institutions: Vec<String>,
    /// Night character id (`sandbox.ron`).
    pub character: String,
    /// Arrival multiplier (village 0.6 … district town 1.5).
    pub size: f32,
    pub options: BTreeMap<String, bool>,
    /// 0 Spring … 3 Winter.
    pub season: u8,
}

/// Sandbox setup screen result.
#[derive(Debug, Clone, PartialEq)]
pub struct SandboxSetup {
    pub seed: u64,
    pub difficulty: Difficulty,
    pub town_name: Option<String>,
    pub cafe_name: Option<String>,
    pub character: Option<String>,
    pub start_money: Option<i64>,
    pub institutions: Option<Vec<String>>,
    pub options: BTreeMap<String, bool>,
}

impl Default for SandboxSetup {
    fn default() -> Self {
        SandboxSetup {
            seed: 1,
            difficulty: Difficulty::Cozy,
            town_name: None,
            cafe_name: None,
            character: None,
            start_money: None,
            institutions: None,
            options: BTreeMap::new(),
        }
    }
}

/// One line of the event log screen (`L`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogEntry {
    pub night: u32,
    pub minute: Option<u32>,
    /// i18n key (event title, bot template, …).
    pub key: String,
    pub args: Vec<(String, String)>,
}

/// A reversible player action (inverse data).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UndoOp {
    /// Undo a placement: remove `uid`, refund `refund`.
    Unplace {
        uid: u32,
        refund: i64,
    },
    /// Undo a removal: put `piece` back, charge `charge`.
    Replace {
        piece: Placed,
        charge: i64,
    },
    /// Undo a move/rotate: back to x, y, rot.
    MoveBack {
        uid: u32,
        x: i32,
        y: i32,
        rot: u8,
    },
    Floor {
        x: i32,
        y: i32,
        floor: grid::Floor,
        refund: i64,
    },
    Priority {
        staff: u32,
        job: u8,
        old: u8,
    },
    /// `SetDuty` / the `SetPriority` shim.
    Duty {
        staff: u32,
        duty: Duty,
        was_on: bool,
    },
    Shift {
        staff: u32,
        weekday: u8,
        old: Shift,
        was_dirty: bool,
    },
    /// `Command::AutoSchedule`: the whole previous draft.
    Draft {
        old: BTreeMap<u32, [Shift; 7]>,
        was_dirty: bool,
        was_manual: bool,
    },
    Price {
        item: String,
        old: Option<u32>,
    },
    MenuItem {
        item: String,
        was_off: bool,
    },
    FurnitureOn {
        uid: u32,
        old: bool,
    },
    /// `Command::SetTheme`: the previous theme.
    Theme {
        old: String,
    },
    /// `Command::MoveGroup`: where each piece stood.
    MoveGroupBack {
        moves: Vec<super::build_ux::PieceMove>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct World {
    pub(crate) version: u32,
    pub(crate) mode: Mode,
    pub(crate) difficulty: Difficulty,
    pub(crate) seed: u64,
    pub(crate) rng: Rng,
    pub(crate) phase: Phase,
    pub(crate) night: u32,
    /// Ticks since 22:00 of the current night.
    pub(crate) tick: u32,
    /// Ticks left in the preparation half hour (21:30–22:00) before the doors open; 0 = the
    /// doors are open (and in the day). Older saves have none (doors open).
    #[serde(default)]
    pub(crate) prep_left: u32,
    pub(crate) cafe_name: String,
    pub(crate) town: Town,
    /// 0 = Mon.
    pub(crate) weekday: u8,
    pub(crate) grid: Grid,
    pub(crate) furniture: Vec<Placed>,
    pub(crate) econ: Economy,
    pub(crate) staff: Vec<Staff>,
    pub(crate) candidates: Vec<Candidate>,
    pub(crate) parties: Vec<Party>,
    pub(crate) guests: Vec<Guest>,
    pub(crate) items: Vec<OrderItem>,
    /// Register uid → party ids in queue order.
    pub(crate) queues: BTreeMap<u32, Vec<u32>>,
    pub(crate) story: StoryState,
    pub(crate) narrator: NarratorState,
    pub(crate) mods: Vec<Modifier>,
    #[serde(with = "ser::weather")]
    pub(crate) weather: Weather,
    pub(crate) blackout_until: Option<u32>,
    pub(crate) music_on: bool,
    /// Hall dirt 0–1.
    pub(crate) dirt: f32,
    pub(crate) sink_dishes: u32,
    pub(crate) sink_claim: Option<u32>,
    pub(crate) mop_claim: Option<u32>,
    /// (tile, removable from minute, claim).
    pub(crate) buckets: Vec<((i32, i32), u32, Option<u32>)>,
    pub(crate) guest_stay_add: i32,
    pub(crate) spawns: Vec<super::guests::PendingSpawn>,
    pub(crate) app: AppState,
    pub(crate) undo: Vec<UndoOp>,
    pub(crate) log: Vec<LogEntry>,
    pub(crate) tonight: NightSummary,
    pub(crate) summaries: Vec<NightSummary>,
    pub(crate) next_id: u32,
    pub(crate) closed: bool,
    pub(crate) satisfaction_sum: f32,
    pub(crate) satisfaction_n: u32,
    /// Sandbox legend / misc counters.
    pub(crate) relationships: BTreeMap<(u32, u32), f32>,
    /// Hands-on state: Karolina, knowledge, the jukebox, tips, rush, tallies (save v2).
    #[serde(default)]
    pub(crate) hands: HandsState,
    /// T5 systems: moments, extras, atmosphere, journal, rozpis puzzle, the app as a tool.
    #[serde(default)]
    pub(crate) sys: super::systems::SystemsState,
    /// Package C: build requests, corners, grandma's attic, the back room, before / after.
    #[serde(default)]
    pub(crate) build: super::build_content::BuildState,
    #[serde(skip)]
    pub(crate) events: Vec<SimEvent>,
    #[serde(skip)]
    pub(crate) paths: PathCache,
}

#[derive(Serialize, Deserialize)]
struct SaveFile {
    version: u32,
    world: World,
}

// ---------------------------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------------------------

impl World {
    fn blank(content: &Content, mode: Mode, difficulty: Difficulty, seed: u64) -> World {
        let mut next_id = 1;
        let (grid, furniture) = grid::default_cafe(content, &mut next_id);
        let diff = content.difficulty(difficulty.id());
        let money = diff.map(|d| d.start_money).unwrap_or(10_000);
        World {
            version: SAVE_VERSION,
            mode,
            difficulty,
            seed,
            rng: Rng::new(seed),
            phase: Phase::Day,
            night: 1,
            tick: 0,
            prep_left: 0,
            cafe_name: STORY_CAFE.into(),
            town: Town {
                name: STORY_TOWN.into(),
                institutions: STORY_INSTITUTIONS.iter().map(|s| s.to_string()).collect(),
                character: content.sandbox.default_character.clone(),
                size: 1.0,
                options: BTreeMap::new(),
                season: 2,
            },
            weekday: 0,
            grid,
            furniture,
            econ: Economy {
                money,
                debt: 0,
                reputation: 25.0,
                supplies: 2500,
                auto_restock: Some(3000),
                ..Default::default()
            },
            staff: Vec::new(),
            candidates: Vec::new(),
            parties: Vec::new(),
            guests: Vec::new(),
            items: Vec::new(),
            queues: BTreeMap::new(),
            story: StoryState::default(),
            narrator: NarratorState::default(),
            mods: Vec::new(),
            weather: Weather::Clear,
            blackout_until: None,
            music_on: false,
            dirt: 0.0,
            sink_dishes: 0,
            sink_claim: None,
            mop_claim: None,
            buckets: Vec::new(),
            guest_stay_add: 0,
            spawns: Vec::new(),
            app: AppState::default(),
            undo: Vec::new(),
            log: Vec::new(),
            tonight: NightSummary::default(),
            summaries: Vec::new(),
            next_id,
            closed: false,
            satisfaction_sum: 0.0,
            satisfaction_n: 0,
            relationships: BTreeMap::new(),
            hands: HandsState::default(),
            sys: Default::default(),
            build: Default::default(),
            events: Vec::new(),
            paths: PathCache::default(),
        }
    }

    /// Story mode, 30 nights. Deterministic (fixed seed).
    pub fn new_story(content: &Content, difficulty: Difficulty) -> World {
        Self::new_story_seeded(content, difficulty, 0x5111_F7E1_1A00_2039)
    }

    pub fn new_story_seeded(content: &Content, difficulty: Difficulty, seed: u64) -> World {
        let mut w = World::blank(content, Mode::Story, difficulty, seed);
        w.econ.debt = content.story.debt;
        w.econ.rent = STORY_UTILITIES;
        w.story.payments = content
            .story
            .payments
            .iter()
            .map(|p| (p.night, p.amount, false))
            .collect();
        for v in &content.story.vars {
            w.story.vars.insert(v.id.clone(), v.default.clone());
        }
        w.town.character = if difficulty == Difficulty::Classic {
            "changeable".into()
        } else {
            "calm".into()
        };
        w.add_owner(content);
        w.ensure_hands(content);
        w.begin_day(content);
        w
    }

    pub fn new_sandbox(content: &Content, setup: SandboxSetup) -> World {
        let mut w = World::blank(content, Mode::Sandbox, setup.difficulty, setup.seed);
        let sb = &content.sandbox;
        w.town.name = setup
            .town_name
            .clone()
            .unwrap_or_else(|| super::staff::town_name(&mut w.rng, sb));
        if let Some(n) = setup.cafe_name.clone() {
            w.cafe_name = n;
        } else if let Some(n) = w.rng.pick(&sb.cafe_names) {
            w.cafe_name = n.clone();
        }
        w.town.character = setup
            .character
            .clone()
            .filter(|c| sb.character(c).is_some())
            .unwrap_or_else(|| sb.default_character.clone());
        w.town.institutions = match setup.institutions.clone() {
            Some(v) => v,
            None => {
                let n = w
                    .rng
                    .range_u32(sb.institutions_per_town.0, sb.institutions_per_town.1)
                    as usize;
                let mut pool: Vec<(String, f32)> = sb
                    .institutions
                    .iter()
                    .map(|i| (i.id.clone(), i.weight))
                    .collect();
                let mut out = Vec::new();
                while out.len() < n && !pool.is_empty() {
                    let ws: Vec<f32> = pool.iter().map(|p| p.1).collect();
                    let Some(i) = w.rng.weighted(&ws) else { break };
                    out.push(pool.remove(i).0);
                }
                out.sort();
                out
            }
        };
        w.town.size = [0.6, 1.0, 1.5][w.rng.index(3)];
        w.town.season = w.rng.index(4) as u8;
        let mut options: BTreeMap<String, bool> =
            sb.options.iter().map(|(k, v)| (k.clone(), *v)).collect();
        options.extend(setup.options.clone());
        w.town.options = options;
        let (lo, hi) = sb.rent;
        let t = (w.town.size - 0.6) / 0.9;
        let rent =
            lo as f32 + (hi - lo) as f32 * t * 0.6 + w.rng.range_f32(0.0, 0.2) * (hi - lo) as f32;
        // readable: whole 50 Kč
        w.econ.rent = ((rent / 50.0).round() * 50.0) as i64;
        if let Some(m) = setup.start_money {
            w.econ.money = m;
        } else {
            w.econ.money = sb.default_start_money.min(
                content
                    .difficulty(setup.difficulty.id())
                    .map(|d| d.start_money * 2)
                    .unwrap_or(sb.default_start_money),
            );
        }
        w.add_owner(content);
        // One starting hire so the first night isn't a solo shift.
        let mut first = super::staff::generate_staff(&mut w, content, Some("barista"));
        first.id = w.alloc_id();
        first.hired_night = 1;
        w.push_staff(first);
        w.ensure_hands(content);
        w.begin_day(content);
        w
    }

    /// A version 1 café gets grandma's jukebox where the default layout has it (Day only:
    /// nobody walks through that tile).
    fn place_jukebox(&mut self, content: &Content) {
        if std::mem::take(&mut self.hands.place_jukebox) && super::music::jukebox(self).is_none() {
            let (x, y) = JUKEBOX_TILE;
            super::commands::place_free(self, content, "jukebox", x, y, 1);
        }
    }

    /// Hands-on state from content, once (new games; saves of version 1 on their first
    /// tick / command / `rebuild_caches`): starter & menu recipes, the starter pantry and
    /// records, default duties for helpers without any.
    pub(crate) fn ensure_hands(&mut self, content: &Content) {
        self.ensure_sys(content);
        if self.hands.ready {
            return;
        }
        self.hands.ready = true;
        super::orders::init_knowledge(self, content);
        super::music::init(self, content);
        if self.phase == Phase::Day {
            self.place_jukebox(content);
        }

        for s in self.staff.iter_mut() {
            if s.owner {
                s.duties.clear();
                s.priorities = [0; 5];
            } else if s.duties.is_empty() {
                s.duties = super::duties::default_duties(content, &s.archetype);
                super::duties::sync_priorities(s);
            }
        }
        for c in self.candidates.iter_mut() {
            if c.staff.duties.is_empty() {
                c.staff.duties = super::duties::default_duties(content, &c.staff.archetype);
                super::duties::sync_priorities(&mut c.staff);
            }
        }
    }

    /// T5 state from content, once (new games; older saves on their first tick / command).
    pub(crate) fn ensure_sys(&mut self, content: &Content) {
        if self.sys.ready {
            return;
        }
        self.sys.ready = true;
        self.sys.fx.checklist_speed = [1.0, 1.0];
        self.story
            .vars
            .entry("cat_name".into())
            .or_insert_with(|| "none".into());
        if let Some(d) = content.handson.decor.default_theme() {
            if self.sys.atmos.theme.is_empty() {
                self.sys.atmos.theme = d.id.clone();
            }
            self.sys.atmos.owned.insert(d.id.clone());
            self.sys.atmos.unlocked.insert(d.id.clone());
        }
        let cat = &content.handson.extras.cat;
        let aff = self.story.counters.get(&cat.counter).copied().unwrap_or(0);
        self.sys.extras.cat_level = cat.level_at(aff.min(cat.max));
        super::rozpis::sync_prefs(self, content);
        self.sys.atmos.coziness = super::atmos::compute(self, content);
    }

    pub(crate) fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub(crate) fn emit(&mut self, e: SimEvent) {
        self.events.push(e);
    }

    pub(crate) fn log_key(&mut self, key: impl Into<String>, args: Vec<(String, String)>) {
        let minute = (self.phase == Phase::Night).then(|| self.minute());
        self.log.push(LogEntry {
            night: self.night,
            minute,
            key: key.into(),
            args,
        });
        if self.log.len() > 600 {
            self.log.drain(0..100);
        }
    }

    fn add_owner(&mut self, _content: &Content) {
        let id = self.alloc_id();
        let entrance = self.grid.entrance();
        let s = Staff {
            id,
            name: String::new(),
            archetype: "owner".into(),
            owner: true,
            story_person: true,
            age: 30,
            female: false,
            traits: Vec::new(),
            skills: [4.0, 3.5, 4.0, 4.0, 4.0],
            wage: 0,
            // Karolina has no job priorities: she only does what the player asks.
            priorities: [0; 5],
            energy: 100.0,
            mood: 70.0,
            hunger: 90.0,
            look_seed: hash_str("player"),
            personal_number: 1,
            hired_night: 1,
            status: StaffStatus::OffDuty,
            pos: grid::tile_center(entrance),
            facing: Facing::Down,
            pose: Pose::Idle,
            path: Vec::new(),
            task: None,
            work_left: 0.0,
            shift: Shift::Full,
            leave_at: NIGHT_MINUTES,
            late_minutes: 0,
            minutes_worked: 0,
            tasks_done: 0,
            hours_week: 0.0,
            toil_hours: 0.0,
            vacation_days: 0,
            skill_boost: None,
            romance_with: None,
            romance_ended_night: None,
            emote: None,
            carrying: None,
            stuck_ticks: 0,
            breaks_tonight: 0,
            met_333: false,
            leave_nights: Vec::new(),
            duties: Vec::new(),
            nights_worked: 0,
            duty_done: [0; 6],
        };
        self.push_staff(s);
    }

    pub(crate) fn push_staff(&mut self, mut s: Staff) {
        if s.personal_number == 0 {
            s.personal_number = 100 + self.staff.len() as u32;
        }
        let id = s.id;
        self.app.draft.insert(id, [Shift::Full; 7]);
        self.app.published.insert(id, [Shift::Full; 7]);
        self.staff.push(s);
    }

    // -----------------------------------------------------------------------------------------
    // Time & phases
    // -----------------------------------------------------------------------------------------

    /// Ticks since 22:00 (10 per game minute).
    pub fn tick_count(&self) -> u32 {
        self.tick
    }

    /// Minutes since 22:00.
    pub fn minute(&self) -> u32 {
        self.tick / TICKS_PER_MINUTE
    }

    pub fn clock(&self) -> ClockTime {
        if self.phase == Phase::Day {
            ClockTime::new(21, 0)
        } else if self.in_prep() {
            ClockTime::new(21, 30 + self.prep_minute() as u16)
        } else {
            ClockTime::from_minutes_since_open(self.minute().min(NIGHT_MINUTES))
        }
    }

    /// Minutes since 22:00, negative before the doors open: −30 at 21:30 (the start of the
    /// preparation), 0 at 22:00, 480 at 06:00. The day phase reads −60 (21:00).
    pub fn minutes_since_open(&self) -> i32 {
        if self.phase == Phase::Day {
            -60
        } else if self.in_prep() {
            self.prep_minute() as i32 - PREP_MINUTES as i32
        } else {
            self.minute() as i32
        }
    }

    /// The quiet half hour before the doors open (21:30–22:00): no guests yet.
    pub fn in_prep(&self) -> bool {
        self.phase == Phase::Night && self.prep_left > 0
    }

    /// Night and the doors are open (22:00 on).
    pub fn doors_open(&self) -> bool {
        self.phase == Phase::Night && self.prep_left == 0
    }

    /// Ticks left before the doors open (0 = open).
    pub fn prep_left_ticks(&self) -> u32 {
        self.prep_left
    }

    /// Whole minutes of the preparation gone (0 at 21:30 … 29 at 21:59).
    pub fn prep_minute(&self) -> u32 {
        PREP_TICKS.saturating_sub(self.prep_left) / TICKS_PER_MINUTE
    }

    /// 0–1 through the preparation (1 once the doors are open).
    pub fn prep_progress(&self) -> f32 {
        if self.in_prep() {
            1.0 - self.prep_left as f32 / PREP_TICKS as f32
        } else {
            1.0
        }
    }

    /// 0–1 through the whole evening shown on the HUD bar: 21:30 → 06:00.
    pub fn evening_progress(&self) -> f32 {
        if self.phase == Phase::Day {
            return 0.0;
        }
        let gone = PREP_TICKS.saturating_sub(self.prep_left) + self.tick.min(NIGHT_TICKS);
        gone as f32 / (PREP_TICKS + NIGHT_TICKS) as f32
    }

    /// `"03:33"`.
    pub fn clock_label(&self) -> String {
        self.clock().to_string()
    }

    /// 0–1 through the night.
    pub fn night_progress(&self) -> f32 {
        if self.phase == Phase::Day {
            0.0
        } else {
            (self.tick as f32 / NIGHT_TICKS as f32).min(1.0)
        }
    }

    pub fn weekday(&self) -> Weekday {
        Weekday::ALL[self.weekday as usize % 7]
    }

    /// Clock paused (day phase, cutscene playing, café closed / ending reached).
    pub fn is_paused(&self) -> bool {
        self.phase == Phase::Day || self.story.playing.is_some() || self.closed
    }

    /// Called when a new day (planning phase) for `self.night` begins.
    pub(crate) fn begin_day(&mut self, content: &Content) {
        self.phase = Phase::Day;
        self.tick = 0;
        self.prep_left = 0;
        self.undo.clear();
        // last night's beat indices must not block today's day beats (e.g. night 17's 21:59
        // franchise offer shares index 0 with night 16's first beat)
        self.story.beats_done.clear();
        self.weekday = match self.mode {
            Mode::Story => content
                .story
                .night(self.night)
                .map(|n| n.weekday as u8)
                .unwrap_or(((self.night - 1) % 7) as u8),
            Mode::Sandbox => ((self.night - 1) % 7) as u8,
        };
        if self.mode == Mode::Sandbox && self.night > 1 && (self.night - 1).is_multiple_of(30) {
            self.town.season = (self.town.season + 1) % 4;
        }
        // Rent growth (sandbox).
        if self.mode == Mode::Sandbox
            && let Some(d) = content.difficulty(self.difficulty.id())
            && d.rent_growth_every > 0
            && self.night > 1
            && (self.night - 1).is_multiple_of(d.rent_growth_every)
        {
            // readable: whole 10 Kč
            let grown = self.econ.rent as f32 * (1.0 + d.rent_growth_percent / 100.0);
            self.econ.rent = ((grown / 10.0).round() * 10.0) as i64;
        }
        // Repair overnight: slowdowns end, very worn stuff is serviced a little.
        for s in self.staff.iter_mut() {
            s.status = StaffStatus::OffDuty;
            s.task = None;
            s.path.clear();
            s.carrying = None;
            s.energy = 100.0;
            s.hunger = 90.0;
            s.mood = (s.mood * 0.6 + 60.0 * 0.4).clamp(0.0, 100.0);
            if let Some((_, _, until)) = s.skill_boost
                && until < self.night
            {
                s.skill_boost = None;
            }
        }
        super::staff::refresh_candidates(self, content);
        super::player::reset(self);
        self.place_jukebox(content);
        super::orders::check_learning(self, content);
        super::music::check_unlocks(self, content);
        super::atmos::on_day_start(self, content);
        super::app::on_day_start(self, content);
        super::rozpis::on_day_start(self, content);
        super::tool::on_day_start(self, content);
        super::rozpis::score(self, content);
        super::story::on_day_start(self, content);
        super::build_content::on_day_start(self, content);
        self.emit(SimEvent::DayStarted { night: self.night });
    }

    /// Day → Night. No-op outside the day phase.
    pub fn start_night(&mut self, content: &Content) {
        if self.phase != Phase::Day || self.closed {
            return;
        }
        self.phase = Phase::Night;
        self.tick = 0;
        // the night begins at 21:30 with the doors closed (the preparation half hour)
        self.prep_left = PREP_TICKS;
        self.undo.clear();
        self.tonight = NightSummary {
            night: self.night,
            reputation_start: self.econ.reputation,
            ..Default::default()
        };
        self.satisfaction_sum = 0.0;
        self.satisfaction_n = 0;
        self.parties.clear();
        self.guests.clear();
        self.items.clear();
        self.queues.clear();
        self.mods.clear();
        self.blackout_until = None;
        self.guest_stay_add = 0;
        self.spawns.clear();
        self.buckets.clear();
        for i in 0..self.grid.blocked.len() {
            self.grid.blocked[i] = false;
        }
        for p in self.furniture.iter_mut() {
            p.user = None;
            p.claim = None;
            for s in p.seat_by.iter_mut() {
                *s = None;
            }
        }
        self.grid.rebuild(&self.furniture, content);
        self.sink_claim = None;
        self.mop_claim = None;
        self.econ.sold.clear();
        let n = &mut self.narrator;
        n.fired_tonight.clear();
        n.staff_events = 0;
        n.bad_tonight = 0;
        n.good_tonight = 0;
        n.random_tonight = 0;
        n.pending.clear();
        n.active.clear();
        n.later.clear();
        n.foreseen.clear();
        n.auto_last.clear();
        n.tension = 0.0;
        n.peak_tension = 0.0;
        n.last_bad = None;
        n.bad_in_row = 0;
        for r in self.story.regulars.values_mut() {
            r.party = None;
            r.spawned.clear();
        }
        self.story.beats_done.clear();
        // hands-on night state
        super::player::reset(self);
        let h = &mut self.hands;
        h.tally = HandsTally::default();
        h.wish_uses.clear();
        h.rush = RushState::default();
        h.lull = LullState::default();
        h.hall_mood = HallMoodState::default();
        let left = std::mem::take(&mut h.tips);
        super::player::collect_coins(self, content, left, None);
        self.sys.fx.tonight_reset();
        super::journal::on_night_start(self, content);
        super::moments::on_night_start(self, content);
        super::economy::restock(self, content);
        super::story::roll_weather(self, content);
        super::app::on_night_start(self, content);
        super::staff::on_night_start(self, content);
        super::music::on_night_start(self, content);
        super::tool::on_night_start(self, content);
        super::xlogic::run_next_night(self, content);
        super::rozpis::on_night_start(self, content);
        super::atmos::on_night_start(self, content);
        super::build_content::on_night_start(self, content);
        self.emit(SimEvent::NightStarted { night: self.night });
        super::story::on_night_start(self, content);
    }

    /// One tick (10 per game minute). Does nothing while paused.
    pub fn tick(&mut self, content: &Content) {
        if self.is_paused() {
            return;
        }
        self.ensure_hands(content);
        self.grid.ensure_masks(&self.furniture, content);
        if self.prep_left > 0 {
            self.prep_tick(content);
            return;
        }
        if self.tick.is_multiple_of(TICKS_PER_MINUTE) {
            self.on_minute(content);
            if self.is_paused() {
                return;
            }
        }
        self.tick += 1;
        super::staff::tick_staff(self, content);
        super::guests::tick_guests(self, content);
        for s in self.staff.iter_mut() {
            if let Some((_, until)) = s.emote
                && self.tick >= until
            {
                s.emote = None;
            }
        }
        for g in self.guests.iter_mut() {
            if let Some((_, until)) = g.emote
                && self.tick >= until
            {
                g.emote = None;
            }
        }
        if self.tick >= NIGHT_TICKS {
            self.end_night(content);
        }
    }

    /// One tick of the preparation half hour (21:30–22:00). The night clock stands at 22:00
    /// meanwhile, so only the quiet systems run: Karolina's queue, the early crew walking in
    /// and doing their checklist items, the tutorial's clock steps, trains passing outside.
    /// No guests, rushes, regulars, moments, story beats, narrator events or wages — those
    /// all start at 22:00 as before. The last prep tick opens the doors.
    fn prep_tick(&mut self, content: &Content) {
        let gone = PREP_TICKS - self.prep_left;
        if gone.is_multiple_of(TICKS_PER_MINUTE) {
            self.on_prep_minute(content);
            if self.is_paused() {
                return;
            }
        }
        self.prep_left -= 1;
        super::staff::tick_staff(self, content);
        // emotes count down by themselves (their `until` is in night ticks, which stand still)
        for s in self.staff.iter_mut() {
            if let Some((_, until)) = &mut s.emote {
                if *until <= self.tick + 1 {
                    s.emote = None;
                } else {
                    *until -= 1;
                }
            }
        }
        if self.prep_left == 0 {
            self.open_doors(content);
        }
    }

    /// Tests: plays the preparation half hour through (the crew walks in, nothing else
    /// happens), then the doors are open at 22:00. Stops early (and opens the doors) if a
    /// cutscene holds the clock.
    #[cfg(test)]
    pub(crate) fn skip_prep(&mut self, content: &Content) {
        let mut guard = 0;
        while self.in_prep() && !self.is_paused() && guard < PREP_TICKS * 2 {
            self.tick(content);
            guard += 1;
        }
        if self.in_prep() {
            self.open_doors(content);
        }
    }

    fn on_prep_minute(&mut self, content: &Content) {
        let m = self.minutes_since_open();
        super::tool::tutorial_at(self, content, m);
        super::prep_trains(self, content, m);
    }

    /// 22:00: the door sign flips to "Open" and guests may come (`SimEvent::DoorsOpened`).
    /// Also `Command::OpenDoors` ("Open the doors now") during the preparation.
    pub(crate) fn open_doors(&mut self, _content: &Content) {
        if self.phase != Phase::Night {
            return;
        }
        let early = self.prep_left;
        self.prep_left = 0;
        self.emit(SimEvent::DoorsOpened {
            early_minutes: early.div_ceil(TICKS_PER_MINUTE),
        });
    }

    /// Runs whole ticks until the night is over or a cutscene/choice pauses it (headless helper).
    /// Cutscenes are auto-finished with no choices; event choices resolve by timeout.
    pub fn run_night_headless(&mut self, content: &Content) -> Vec<SimEvent> {
        let mut out = Vec::new();
        if self.phase == Phase::Day {
            self.auto_finish_cutscenes(content, &mut out);
            self.start_night(content);
        }
        let mut guard = 0u32;
        while self.phase == Phase::Night && !self.closed && guard < NIGHT_TICKS * 4 {
            self.auto_finish_cutscenes(content, &mut out);
            self.tick(content);
            out.extend(self.drain_events());
            guard += 1;
        }
        self.auto_finish_cutscenes(content, &mut out);
        out
    }

    fn auto_finish_cutscenes(&mut self, content: &Content, out: &mut Vec<SimEvent>) {
        let mut guard = 0;
        while let Some(id) = self.story.playing.clone() {
            out.extend(self.drain_events());
            let _ = self.apply(
                content,
                Command::CutsceneFinished {
                    id,
                    choices: Vec::new(),
                },
            );
            guard += 1;
            if guard > 64 {
                break;
            }
        }
        out.extend(self.drain_events());
    }

    fn on_minute(&mut self, content: &Content) {
        let m = self.minute();
        // Expire modifiers.
        let before = self.blackout_until;
        self.mods.retain(|md| md.until.is_none_or(|u| u > m));
        if let Some(b) = before
            && b <= m
        {
            self.blackout_until = None;
            self.mods.retain(|md| md.kind != ModKind::PowerOut);
            self.emit(SimEvent::Blackout { on: false });
        }
        self.narrator.active.retain(|(_, u)| *u > m);
        super::story::on_minute(self, content);
        if self.is_paused() {
            return;
        }
        super::narrator::on_minute(self, content);
        super::trains_tick(self, content);
        super::guests::on_minute(self, content);
        super::staff::on_minute(self, content);
        super::app::on_minute(self, content);
        super::music::on_minute(self, content);
        if m.is_multiple_of(5) {
            super::orders::check_learning(self, content);
        }
        super::orders::assist_wishes(self, content);
        super::pacing::on_minute(self, content);
        super::xlogic::run_later(self, content);
        super::moments::on_minute(self, content);
        super::moments::naming_nudge(self, content);
        super::atmos::on_minute(self, content);
        super::journal::on_minute(self, content);
        super::build_content::on_minute(self, content);
        super::tool::on_minute(self, content);
        super::rozpis::on_minute(self, content);
        let q = self.queue_len() as u32;
        self.tonight.max_queue = self.tonight.max_queue.max(q);
    }

    fn end_night(&mut self, content: &Content) {
        self.tick = NIGHT_TICKS;
        // Everybody still inside pays and leaves; uncollected tips go into the jar.
        super::guests::close_up(self, content);
        super::player::collect_all_tips(self, content);
        super::player::reset(self);
        super::moments::on_night_end(self, content);
        super::tool::on_night_end(self, content);
        super::rozpis::on_night_end(self);
        super::build_content::on_night_end(self, content);
        super::journal::on_night_end(self, content);
        self.tonight.insights = super::tool::insights(self, content);
        self.tonight.coziness_avg = super::atmos::night_avg(self);
        for s in self.staff.iter_mut() {
            if !s.owner && s.minutes_worked > 0 {
                s.nights_worked += 1;
            }
        }
        super::story::on_night_end_early(self, content);
        super::app::on_night_end(self, content);
        super::economy::on_night_end(self, content);
        self.narrator.later.clear();
        self.narrator.pending.clear();
        super::narrator::on_night_end(self, content);
        for s in self.staff.iter_mut() {
            if s.present() {
                s.status = StaffStatus::OffDuty;
            }
            s.task = None;
            s.path.clear();
        }
        self.guests.clear();
        self.parties.clear();
        self.items.clear();
        self.queues.clear();
        super::music::on_night_end(self);
        let mut summary = self.tonight.clone();
        let t = &self.hands.tally;
        summary.karolina_items = t.karolina_items;
        summary.helper_items = t.helper_items;
        summary.minigames = t.by_hand;
        summary.wishes_served = t.wishes_served;
        summary.wishes_perfect = t.tiers[0];
        summary.regular_items = t.regular_items;
        summary.regular_items_karolina = t.regular_items_karolina;
        summary.karolina_idle_minutes = t.idle_minutes;
        summary.todo_peak = t.todo_peak;
        summary.todo_median = {
            let total: u32 = t.todo_hist.iter().sum();
            let mut acc = 0;
            let mut med = 0;
            for (k, n) in t.todo_hist.iter().enumerate() {
                acc += n;
                if acc * 2 >= total {
                    med = k as u32;
                    break;
                }
            }
            med
        };
        summary.best_rhythm = t.best_rhythm as u32;
        summary.helper_credits = super::duties::helper_credits(self);
        summary.money_end = self.econ.money;
        summary.reputation_end = self.econ.reputation;
        summary.avg_satisfaction = if self.satisfaction_n > 0 {
            self.satisfaction_sum / self.satisfaction_n as f32
        } else {
            0.0
        };
        summary.best_seller = self
            .econ
            .sold
            .iter()
            .max_by_key(|(_, n)| **n)
            .map(|(k, _)| k.clone());
        summary.star_of_night = self
            .staff
            .iter()
            .filter(|s| s.tasks_done > 0)
            .max_by_key(|s| s.tasks_done)
            .map(|s| s.id);
        self.tonight = summary.clone();
        super::app::digest(self, content, &summary);
        self.summaries.push(summary.clone());
        if self.summaries.len() > 60 {
            self.summaries.remove(0);
        }
        self.emit(SimEvent::NightEnded {
            summary: Box::new(summary),
        });
        let ended = super::story::on_night_end(self, content);
        self.phase = Phase::Day;
        if ended || self.closed {
            return;
        }
        self.night += 1;
        self.begin_day(content);
    }

    // -----------------------------------------------------------------------------------------
    // Events out
    // -----------------------------------------------------------------------------------------

    pub fn drain_events(&mut self) -> Vec<SimEvent> {
        std::mem::take(&mut self.events)
    }

    // -----------------------------------------------------------------------------------------
    // Saves
    // -----------------------------------------------------------------------------------------

    pub fn save_to_string(&self) -> Result<String, String> {
        #[derive(Serialize)]
        struct SaveRef<'a> {
            version: u32,
            world: &'a World,
        }
        ron::ser::to_string(&SaveRef {
            version: SAVE_VERSION,
            world: self,
        })
        .map_err(|e| e.to_string())
    }

    /// Loads a save. Caches (walk masks, paths) rebuild on the next tick.
    ///
    /// Version 1 saves (before the hands-on update) are migrated: helpers' duties come from
    /// their priority-1 jobs, Karolina drops any autopilot task, and the recipe knowledge and
    /// records are filled from content on the first tick / command / `rebuild_caches`.
    pub fn load_from_str(text: &str) -> Result<World, String> {
        let file: SaveFile = ron::from_str(text).map_err(|e| e.to_string())?;
        let mut world = file.world;
        match file.version {
            SAVE_VERSION => {}
            1 => world.migrate_v1(),
            v => {
                return Err(format!(
                    "save version {v} not supported (expected {SAVE_VERSION})"
                ));
            }
        }
        world.version = SAVE_VERSION;
        Ok(world)
    }

    fn migrate_v1(&mut self) {
        self.hands = HandsState {
            place_jukebox: true,
            ..Default::default()
        };
        for i in 0..self.staff.len() {
            if self.staff[i].owner {
                // the old job loop may have left her mid-task
                super::staff::abandon(self, i);
                self.staff[i].priorities = [0; 5];
                self.staff[i].duties.clear();
            } else {
                let p = self.staff[i].priorities;
                self.staff[i].duties = super::duties::duties_from_priorities(&p);
                super::duties::sync_priorities(&mut self.staff[i]);
            }
        }
        for c in self.candidates.iter_mut() {
            let p = c.staff.priorities;
            c.staff.duties = super::duties::duties_from_priorities(&p);
        }
        // the shift loop's "nights worked" didn't exist: count the nights since the hire
        let night = self.night;
        for s in self.staff.iter_mut().filter(|s| !s.owner) {
            s.nights_worked = night.saturating_sub(s.hired_night.max(1));
        }
    }

    /// Call once after loading if you need queries before the first tick.
    pub fn rebuild_caches(&mut self, content: &Content) {
        self.ensure_hands(content);
        self.grid.rebuild(&self.furniture, content);
    }

    // -----------------------------------------------------------------------------------------
    // Read-only queries
    // -----------------------------------------------------------------------------------------

    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn mode(&self) -> Mode {
        self.mode
    }
    pub fn difficulty(&self) -> Difficulty {
        self.difficulty
    }
    pub fn seed(&self) -> u64 {
        self.seed
    }
    /// Story: 1..=30. Sandbox: nights played + 1.
    pub fn night(&self) -> u32 {
        self.night
    }
    pub fn money(&self) -> i64 {
        self.econ.money
    }
    pub fn debt(&self) -> i64 {
        self.econ.debt
    }
    pub fn reputation(&self) -> f32 {
        self.econ.reputation
    }
    pub fn supplies(&self) -> i64 {
        self.econ.supplies
    }
    pub fn rent(&self) -> i64 {
        self.econ.rent
    }
    pub fn cafe_name(&self) -> &str {
        &self.cafe_name
    }
    pub fn town(&self) -> &Town {
        &self.town
    }
    pub fn weather(&self) -> Weather {
        self.weather
    }
    pub fn blackout(&self) -> bool {
        self.blackout_until.is_some()
    }
    pub fn music_on(&self) -> bool {
        self.music_on
    }
    /// Hall dirt 0–1.
    pub fn dirt(&self) -> f32 {
        self.dirt
    }
    pub fn grid(&self) -> &Grid {
        &self.grid
    }
    pub fn furniture(&self) -> &[Placed] {
        &self.furniture
    }
    pub fn furniture_by_uid(&self, uid: u32) -> Option<&Placed> {
        self.furniture.iter().find(|p| p.uid == uid)
    }
    pub fn staff(&self) -> &[Staff] {
        &self.staff
    }
    pub fn staff_by_id(&self, id: u32) -> Option<&Staff> {
        self.staff.iter().find(|s| s.id == id)
    }
    pub fn candidates(&self) -> &[Candidate] {
        &self.candidates
    }
    pub fn parties(&self) -> &[Party] {
        &self.parties
    }
    pub fn guests(&self) -> &[Guest] {
        &self.guests
    }
    pub fn order_items(&self) -> &[OrderItem] {
        &self.items
    }
    pub fn story(&self) -> &StoryState {
        &self.story
    }
    pub fn flag(&self, f: &str) -> bool {
        self.story.flags.contains(f)
    }
    pub fn notes_found(&self) -> &[String] {
        &self.story.notes
    }
    pub fn ending(&self) -> Option<&str> {
        self.story.ending.as_deref()
    }
    pub fn ending_variant(&self) -> Option<&str> {
        self.story.ending_variant.as_deref()
    }
    /// Cutscene currently waiting for `Command::CutsceneFinished`.
    pub fn playing_cutscene(&self) -> Option<&str> {
        self.story.playing.as_deref()
    }
    pub fn is_closed(&self) -> bool {
        self.closed
    }
    pub fn app(&self) -> &AppState {
        &self.app
    }
    pub fn event_log(&self) -> &[LogEntry] {
        &self.log
    }
    pub fn pending_choices(&self) -> &[PendingChoice] {
        &self.narrator.pending
    }
    /// Running night ledger (finalised at 06:00).
    pub fn tonight(&self) -> &NightSummary {
        &self.tonight
    }
    pub fn summaries(&self) -> &[NightSummary] {
        &self.summaries
    }
    pub fn last_summary(&self) -> Option<&NightSummary> {
        self.summaries.last()
    }
    pub fn relationship(&self, a: u32, b: u32) -> f32 {
        let key = if a < b { (a, b) } else { (b, a) };
        self.relationships.get(&key).copied().unwrap_or(0.0)
    }
    pub fn tension(&self) -> f32 {
        self.narrator.tension
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    /// Tiles blocked by a bucket (render a bucket there).
    pub fn bucket_tiles(&self) -> Vec<(i32, i32)> {
        self.buckets.iter().map(|b| b.0).collect()
    }
    /// The window lamp is on (`Condition::WindowLightOn`).
    pub fn window_light_on(&self, content: &Content) -> bool {
        super::logic::window_light(self, content).is_some_and(|uid| {
            self.furniture_by_uid(uid).is_some_and(|p| p.on) && self.blackout_until.is_none()
        })
    }
    /// Price currently charged for a menu item.
    pub fn price_of(&self, content: &Content, item: &str) -> u32 {
        self.econ
            .prices
            .get(item)
            .copied()
            .or_else(|| content.menu_item(item).map(|m| m.price))
            .unwrap_or(0)
    }
    /// Menu items that can be ordered right now.
    pub fn menu_available(&self, content: &Content) -> Vec<String> {
        content
            .menu
            .iter()
            .filter(|m| super::guests::item_orderable(self, content, m))
            .map(|m| m.id.clone())
            .collect()
    }
    /// Unlocked menu items (on or off the menu).
    pub fn menu_unlocked(&self, content: &Content) -> Vec<String> {
        content
            .menu
            .iter()
            .filter(|m| super::guests::item_unlocked(self, m))
            .map(|m| m.id.clone())
            .collect()
    }
    /// Furniture the build menu offers.
    pub fn buildable(&self, content: &Content) -> Vec<String> {
        content
            .furniture
            .iter()
            .filter(|f| super::commands::furniture_unlocked(self, f))
            .map(|f| f.id.clone())
            .collect()
    }
    pub fn queue_len(&self) -> usize {
        self.queues.values().map(|q| q.len()).sum()
    }
    pub fn guests_inside(&self) -> usize {
        self.guests.iter().filter(|g| !g.gone).count()
    }

    /// Render view of every present staff member.
    pub fn staff_views(&self) -> Vec<AgentView> {
        self.staff
            .iter()
            .filter(|s| s.present())
            .map(|s| AgentView {
                agent: AgentRef::Staff(s.id),
                look: LookRef {
                    seed: s.look_seed,
                    staff: !s.owner,
                    preset: s.owner.then(|| "player".to_string()),
                    source: s.archetype.clone(),
                },
                feet: s.pos,
                facing: s.facing,
                pose: s.pose,
                emote: s.emote.map(|e| e.0),
                seat_tile: None,
                holding: s.carrying.clone(),
            })
            .collect()
    }

    /// Render view of every guest in or around the café.
    pub fn guest_views(&self) -> Vec<AgentView> {
        self.guests
            .iter()
            .filter(|g| !g.gone)
            .map(|g| {
                let seat_tile = g.seat.and_then(|(uid, i)| {
                    (g.pose == Pose::Sit)
                        .then(|| super::guests::seat_tile(self, uid, i))
                        .flatten()
                });
                AgentView {
                    agent: AgentRef::Guest(g.id),
                    look: LookRef {
                        seed: g.look_seed,
                        staff: false,
                        preset: g.preset.clone(),
                        source: g.source.clone(),
                    },
                    feet: g.pos,
                    facing: g.facing,
                    pose: g.pose,
                    emote: g.emote.map(|e| e.0),
                    seat_tile,
                    holding: g.holding.clone(),
                }
            })
            .collect()
    }

    /// Render state of every furniture piece.
    pub fn furniture_views(&self, content: &Content) -> Vec<FurnitureView> {
        self.furniture
            .iter()
            .map(|p| {
                let def = content.furniture_def(&p.def);
                let powered = def.is_none_or(|d| !d.tags.iter().any(|t| t.starts_with("station:")))
                    || self.blackout_until.is_none();
                let active = if def.is_some_and(|d| d.has_tag("light") || d.has_tag("music")) {
                    p.on && self.blackout_until.is_none()
                } else if def.is_some_and(|d| d.has_tag("table")) {
                    p.on
                } else {
                    powered
                };
                let size = def.map(|d| grid::rotated_size(d, p.rot)).unwrap_or((1, 1));
                FurnitureView {
                    uid: p.uid,
                    active,
                    dirty: (p.dirt + p.dishes as f32 * 0.2).min(1.0),
                    facing: p.facing(),
                    tile: (p.x, p.y),
                    size,
                }
            })
            .collect()
    }

    /// Condition evaluation for UI / cutscene `If` steps (read-only; `Chance` uses a hash roll).
    pub fn eval_conditions(&self, content: &Content, conds: &[crate::content::Condition]) -> bool {
        let mut scratch = self.clone();
        scratch.events.clear();
        super::logic::all(&mut scratch, content, conds, &mut EffectCtx::default())
    }
}
