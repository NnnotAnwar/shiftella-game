//! Public sim types: phases, poses, commands in, events out, read-only views.

use serde::{Deserialize, Serialize};

use crate::content::{Job, Weather};

/// Serde helpers for content enums that only derive `Deserialize`.
pub(crate) mod ser {
    pub mod weather {
        use crate::content::Weather;
        use serde::{Deserialize, Deserializer, Serializer};
        pub fn serialize<S: Serializer>(w: &Weather, s: S) -> Result<S::Ok, S::Error> {
            s.serialize_u8(Weather::ALL.iter().position(|x| x == w).unwrap_or(0) as u8)
        }
        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Weather, D::Error> {
            let i = u8::deserialize(d)? as usize;
            Ok(Weather::ALL[i.min(Weather::ALL.len() - 1)])
        }
    }
}

/// Difficulty preset (`difficulty.ron` ids `cozy` / `classic`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Difficulty {
    #[default]
    Cozy,
    Classic,
}

impl Difficulty {
    pub fn id(self) -> &'static str {
        match self {
            Difficulty::Cozy => "cozy",
            Difficulty::Classic => "classic",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Story,
    Sandbox,
}

/// Day = planning (clock paused); Night = 22:00–06:00 running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Day,
    Night,
}

/// Mirrors `render::Facing`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Facing {
    #[default]
    Down,
    Up,
    Left,
    Right,
}

impl Facing {
    pub fn from_delta(dx: f32, dy: f32) -> Facing {
        if dx.abs() > dy.abs() {
            if dx > 0.0 {
                Facing::Right
            } else {
                Facing::Left
            }
        } else if dy < 0.0 {
            Facing::Up
        } else {
            Facing::Down
        }
    }
    /// Furniture rotation (0..4) → facing: 0 Down, 1 Left, 2 Up, 3 Right.
    pub fn from_rot(rot: u8) -> Facing {
        match rot % 4 {
            0 => Facing::Down,
            1 => Facing::Left,
            2 => Facing::Up,
            _ => Facing::Right,
        }
    }
    pub fn to_rot(self) -> u8 {
        match self {
            Facing::Down => 0,
            Facing::Left => 1,
            Facing::Up => 2,
            Facing::Right => 3,
        }
    }
}

/// Mirrors `render::Pose`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Pose {
    #[default]
    Idle,
    Walk,
    Sit,
    Carry,
    Work,
}

/// Mirrors `render::Emote` (speech-bubble icon over a head).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimEmote {
    Coffee,
    Heart,
    Zzz,
    Angry,
    Note,
    Music,
    Exclamation,
    Money,
}

/// A week shift in the Shiftella rozpis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Shift {
    Off,
    /// 22:00–06:00 (8 h).
    #[default]
    Full,
    /// 22:00–02:00 (4 h).
    Early,
    /// 02:00–06:00 (4 h).
    Late,
}

impl Shift {
    pub const ALL: [Shift; 4] = [Shift::Off, Shift::Full, Shift::Early, Shift::Late];
    /// (start, end) in minutes since opening.
    pub fn span(self) -> Option<(u32, u32)> {
        match self {
            Shift::Off => None,
            Shift::Full => Some((0, 480)),
            Shift::Early => Some((0, 240)),
            Shift::Late => Some((240, 480)),
        }
    }
    pub fn hours(self) -> f32 {
        self.span()
            .map(|(a, b)| (b - a) as f32 / 60.0)
            .unwrap_or(0.0)
    }
    pub fn id(self) -> &'static str {
        match self {
            Shift::Off => "off",
            Shift::Full => "full",
            Shift::Early => "early",
            Shift::Late => "late",
        }
    }
}

/// Kind of time off (Shiftella: dovolená / náhradní volno).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LeaveKind {
    /// Dovolená — paid vacation.
    Vacation,
    /// Náhradní volno — time off in lieu of overtime hours.
    Toil,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestStatus {
    Pending,
    Approved,
    Declined,
}

/// Reference to anything with a body on the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentRef {
    Staff(u32),
    Guest(u32),
}

/// Data for the morning summary / night digest.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NightSummary {
    pub night: u32,
    pub guests_arrived: u32,
    pub guests_served: u32,
    /// Parties/guests who left without being served (impatient, no seat…).
    pub guests_left_unhappy: u32,
    pub guests_turned_away: u32,
    pub orders_served: u32,
    pub revenue: i64,
    pub tips: i64,
    pub wages: i64,
    pub supplies: i64,
    pub rent: i64,
    pub other_income: i64,
    pub other_expenses: i64,
    pub payment: i64,
    pub payment_missed: bool,
    pub net: i64,
    pub money_end: i64,
    pub reputation_start: f32,
    pub reputation_end: f32,
    pub avg_satisfaction: f32,
    pub events: Vec<String>,
    pub bad_events: u32,
    pub best_seller: Option<String>,
    /// Staff id with the most finished tasks.
    pub star_of_night: Option<u32>,
    pub dishes_broken: u32,
    pub notes_found: Vec<String>,
    pub max_queue: u32,
    /// Staff-minutes spent below 15 energy.
    pub exhausted_minutes: u32,
    /// Agents that were stuck (no progress) for too long and got reset.
    pub stuck_resets: u32,
    pub checklist_done: u32,
    pub checklist_total: u32,
}

/// Everything the sim tells audio / UI / cutscenes. Drain with `World::drain_events`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SimEvent {
    DayStarted {
        night: u32,
    },
    NightStarted {
        night: u32,
    },
    NightEnded {
        summary: Box<NightSummary>,
    },
    GuestArrived {
        party: u32,
        archetype: String,
        size: u32,
        regular: Option<String>,
    },
    /// A party looked in, found no seat and left.
    GuestTurnedAway {
        party: u32,
    },
    GuestLeft {
        party: u32,
        satisfaction: f32,
        served: bool,
    },
    OrderTaken {
        party: u32,
        staff: u32,
        items: Vec<String>,
    },
    OrderReady {
        item: String,
        staff: u32,
    },
    OrderServed {
        party: u32,
        staff: u32,
        item: String,
    },
    Payment {
        amount: i64,
    },
    TipReceived {
        amount: i64,
        staff: Option<u32>,
    },
    DishBroken {
        staff: u32,
    },
    StaffLate {
        staff: u32,
        minutes: u32,
    },
    StaffArrived {
        staff: u32,
    },
    StaffLeft {
        staff: u32,
    },
    StaffSick {
        staff: u32,
    },
    StaffWalkedOut {
        staff: u32,
    },
    StaffBreak {
        staff: u32,
    },
    StaffHired {
        staff: u32,
    },
    StaffFired {
        staff: u32,
    },
    TrainAnnounced {
        train: String,
    },
    TrainPassing {
        train: String,
        /// Origin station id (`trains.ron` → `stations`).
        station: String,
    },
    Blackout {
        on: bool,
    },
    WeatherChanged {
        #[serde(with = "ser::weather")]
        weather: Weather,
    },
    MusicChanged {
        on: bool,
    },
    EventStarted {
        id: String,
    },
    /// A toast with buttons (`event.<id>.choice.<choice>`). Answer with `Command::AnswerEvent`.
    ChoiceNeeded {
        id: String,
        choices: Vec<String>,
        /// Game minutes until `default_choice` applies.
        expires_in: u32,
    },
    ChoiceResolved {
        id: String,
        choice: String,
    },
    /// Foresight trait: an event is coming in `minutes`.
    EventHint {
        id: String,
        minutes: u32,
    },
    Toast {
        key: String,
    },
    /// Tutorial / story hint (text key).
    Hint {
        key: String,
    },
    /// @shiftella_bot message: `key` = `app.bot.<template>`; `msg` = inbox id for actions.
    BotMessage {
        msg: u32,
        template: String,
        key: String,
        args: Vec<(String, String)>,
    },
    /// Speech bubble (`key` text) over a speaker (regular id, staff archetype, cast id).
    Speech {
        speaker: String,
        agent: Option<AgentRef>,
        key: String,
    },
    PlayCutscene(String),
    EndingReached(String),
    NoteFound(String),
    ItemGained(String),
    ItemLost(String),
    FlagSet(String),
    RegularArrived {
        regular: String,
        party: u32,
    },
    ArcAdvanced {
        regular: String,
        stage: u32,
    },
    /// Freeze clock and animations for this many real seconds (3:33).
    ClockStop {
        seconds: f32,
    },
    PaymentMade {
        amount: i64,
        debt_left: i64,
    },
    PaymentMissed {
        amount: i64,
    },
    /// Sandbox bankruptcy / Classic closure.
    CafeClosed,
    SchedulePublished,
    ChecklistTicked {
        id: String,
    },
    MenuUnlocked(String),
    FurnitureUnlocked(String),
    FurniturePlaced {
        uid: u32,
    },
    FurnitureRemoved {
        uid: u32,
    },
}

/// Every player action.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    PlaceFurniture {
        def: String,
        x: i32,
        y: i32,
        rot: u8,
    },
    MoveFurniture {
        uid: u32,
        x: i32,
        y: i32,
        rot: u8,
    },
    RotateFurniture {
        uid: u32,
    },
    RemoveFurniture {
        uid: u32,
    },
    /// Paint a floor (`floor_wood` / `floor_tile` / `floor_carpet`).
    SetFloor {
        x: i32,
        y: i32,
        floor: String,
    },
    /// Toggle a lamp / jukebox / radio.
    SetFurnitureOn {
        uid: u32,
        on: bool,
    },
    Hire {
        candidate: u32,
    },
    Fire {
        staff: u32,
    },
    /// 0 = off, 1 (highest) … 4.
    SetPriority {
        staff: u32,
        job: Job,
        priority: u8,
    },
    /// Edits the **draft** schedule (weekday 0 = Mon).
    SetShift {
        staff: u32,
        weekday: u8,
        shift: Shift,
    },
    PublishSchedule,
    AnswerSwap {
        id: u32,
        approve: bool,
    },
    AnswerTimeOff {
        id: u32,
        approve: bool,
    },
    RequestTimeOff {
        staff: u32,
        kind: LeaveKind,
        weekday: u8,
    },
    AnswerEvent {
        event: String,
        choice: String,
    },
    /// A button on a bot message (`app.bot.<template>.action.<id>`).
    BotAction {
        msg: u32,
        action: String,
    },
    /// The cutscene player is done. `choices` = chosen option index (into the full `options`
    /// list) for every `Choice` step in the order they were shown.
    CutsceneFinished {
        id: String,
        choices: Vec<usize>,
    },
    /// "Talk" on a regular: plays their talk cutscene for tonight (see `World::talk_available`).
    Talk {
        guest: u32,
    },
    /// Buy ingredients for this many CZK.
    BuySupplies {
        amount: i64,
    },
    SetAutoRestock {
        target: Option<i64>,
    },
    SetPrice {
        item: String,
        price: u32,
    },
    /// Put an unlocked item on / off the menu.
    SetMenuItem {
        item: String,
        on: bool,
    },
    RenameCafe {
        name: String,
    },
    TickChecklist {
        id: String,
    },
    PostAnnouncement {
        id: String,
        args: Vec<(String, String)>,
    },
    WriteHandover {
        text: String,
    },
    SetMusic {
        on: bool,
    },
    /// Day → Night (same as `World::start_night`).
    StartNight,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    UnknownId(String),
    NotEnoughMoney { need: i64, have: i64 },
    OutOfBounds,
    Occupied,
    BlocksPath,
    NotUnlocked,
    AlreadyPlaced,
    InUse,
    WrongPhase,
    CutscenePlaying,
    Invalid(String),
    NothingToUndo,
    GameOver,
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for CommandError {}

/// How the render agent should build the person's look.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LookRef {
    /// Seed for `PersonLook::random(seed, staff)`.
    pub seed: u64,
    pub staff: bool,
    /// Story preset for `PersonLook::preset(id)` (regulars, `player`, `guest_333`…).
    pub preset: Option<String>,
    /// Content source of the look hints (staff/guest archetype id, regular id, cast id).
    pub source: String,
}

/// One person, ready for `render::draw_person`.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentView {
    pub agent: AgentRef,
    pub look: LookRef,
    /// Feet in world px (for `Sit` the seat tile centre; render may use `sit_anchor`).
    pub feet: (f32, f32),
    pub facing: Facing,
    pub pose: Pose,
    pub emote: Option<SimEmote>,
    /// Seat tile when sitting (for `render::sit_anchor`).
    pub seat_tile: Option<(i32, i32)>,
    /// Menu item id carried / being drunk.
    pub holding: Option<String>,
}

/// Render state of a placed furniture piece (maps onto `render::FurnitureState`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FurnitureView {
    pub uid: u32,
    pub active: bool,
    pub dirty: f32,
    pub facing: Facing,
    /// Tile origin (top-left of the rotated footprint) and rotated size.
    pub tile: (i32, i32),
    pub size: (u32, u32),
}
