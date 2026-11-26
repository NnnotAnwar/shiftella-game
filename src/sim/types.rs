//! Public sim types: phases, poses, commands in, events out, read-only views.

use serde::{Deserialize, Serialize};

use super::hands::{BrewMode, LullHint, OrderKind, PlayerAct};
use super::systems::{Insight, MomentPos};
use crate::content::handson::logic::CameraTarget;
use crate::content::handson::{
    AppLink, Duty, HallMood, HighlightKind, MatchTier, PrefKind, RequestKind, TipSplit,
};
use crate::content::world::ChecklistKind;
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
    /// A perfect serve (hands-on; render mirrors it).
    Sparkle,
    /// A greeting, an okay serve.
    Smile,
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
    // --- hands-on (save v2) -------------------------------------------------------------------
    /// Guests who waited too long on Cozy and left content (no review penalty).
    #[serde(default)]
    pub guests_forgiven: u32,
    /// Items made by Karolina / by helpers.
    #[serde(default)]
    pub karolina_items: u32,
    #[serde(default)]
    pub helper_items: u32,
    /// Karolina's by-hand (mini-game) brews.
    #[serde(default)]
    pub minigames: u32,
    #[serde(default)]
    pub wishes_served: u32,
    #[serde(default)]
    pub wishes_perfect: u32,
    /// Regulars' items, and how many of them Karolina made.
    #[serde(default)]
    pub regular_items: u32,
    #[serde(default)]
    pub regular_items_karolina: u32,
    /// Night minutes Karolina had nothing meaningful to do (quiet hour excluded).
    #[serde(default)]
    pub karolina_idle_minutes: u32,
    /// Median / peak of her pending things per minute.
    #[serde(default)]
    pub todo_median: u32,
    #[serde(default)]
    pub todo_peak: u32,
    #[serde(default)]
    pub best_rhythm: u32,
    /// Helpers' credits for the digest: (name, duty id, tasks).
    #[serde(default)]
    pub helper_credits: Vec<(String, String, u32)>,
    // --- T5 ---------------------------------------------------------------------------------
    /// At most 4 digest insights with deep links (handson.md §13.6).
    #[serde(default)]
    pub insights: Vec<Insight>,
    /// Moments that appeared / were resolved tonight.
    #[serde(default)]
    pub moments: u32,
    #[serde(default)]
    pub moments_resolved: u32,
    /// Mean coziness tonight and the meter level of that mean.
    #[serde(default)]
    pub coziness_avg: f32,
    #[serde(default)]
    pub experiments: u32,
    #[serde(default)]
    pub napkins: u32,
}

/// What an experiment in the pantry gave (handson.md §4.4); keys from `experiment.*`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExperimentResult {
    /// "You know this one: {recipe}." (`experiment.known`)
    Known { recipe: String },
    /// "Discovered!" (`experiment.discovered`) — also `RecipeLearned { source: "experiment" }`.
    Discovered { recipe: String },
    /// A story-gated recipe: its `gated_hint` key.
    Gated { recipe: String, key: String },
    /// "So close…" (`experiment.close.swap/.extra/.missing`); `family` = `ingredient.family.<id>`.
    Close { key: String, family: Option<String> },
    /// "It's… drinkable." (`experiment.nothing.<n>`); `cat` = the cat sniffs it.
    Nothing { key: String, cat: bool },
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
    /// 22:00: the preparation half hour is over, the door sign flips to "Open" and guests may
    /// come in. `early_minutes` > 0 when the player opened early (`Command::OpenDoors`).
    DoorsOpened {
        early_minutes: u32,
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
    /// Someone goes on a break (auto, or `Command::SendOnBreak`).
    StaffBreak {
        staff: u32,
    },
    /// …and is back on the floor.
    StaffBreakEnded {
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
    // --- hands-on -----------------------------------------------------------------------------
    /// Karolina took an order: open the order card (`wish` = `wish.<id>`, `None` = plain).
    PlayerOrderTaken {
        guest: u32,
        party: u32,
        wish: Option<String>,
        kind: OrderKind,
    },
    /// Karolina served something she made.
    PlayerServed {
        guest: u32,
        recipe: String,
        match_score: f32,
        quality: f32,
    },
    /// Her queue / current act changed (refresh the icons over her head).
    PlayerQueueChanged,
    PlayerGreeted {
        guest: u32,
    },
    /// Reaction bubble after a wish serve (`react.<tier>.<n>` / `react.<regular>.<tier>`).
    GuestReaction {
        guest: u32,
        tier: MatchTier,
        key: String,
    },
    /// Cozy: patience ran out — a glass of water, patience refills once.
    GuestPatienceRefilled {
        party: u32,
    },
    /// Cozy: they waited too long and leave content ("I'll come back tomorrow.").
    GuestForgiven {
        party: u32,
        key: String,
    },
    /// A coin glints on a table.
    TipLeft {
        table: u32,
        amount: i64,
    },
    /// Coins went into the tip jar (Karolina, a dishes helper, or the night end).
    TipCollected {
        amount: i64,
        by: Option<u32>,
    },
    /// Karolina learned a recipe (`source`: `starter`, `card`, `taught:<who>`, `story`).
    RecipeLearned {
        recipe: String,
        source: String,
    },
    IngredientUnlocked {
        id: String,
    },
    PractisedRecipe {
        recipe: String,
    },
    RushStarted,
    RushEnded,
    /// The rhythm streak (purely positive; 0 = it faded).
    RhythmChanged {
        step: u8,
    },
    /// Karolina has been idle this long; the lull director suggests something.
    KarolinaIdle {
        minutes: u32,
    },
    LullSuggestion {
        hint: LullHint,
    },
    /// A regular is crossing the street: portrait chip on the rail.
    RegularIncoming {
        regular: String,
        party: u32,
        minutes: u32,
    },
    /// The jukebox changed (`None` = the house playlist).
    RecordChanged {
        record: Option<String>,
        repeat: bool,
    },
    HallMoodChanged {
        mood: Option<HallMood>,
    },
    RecordUnlocked {
        id: String,
    },
    RecordReaction {
        guest: u32,
        record: String,
        liked: bool,
    },
    /// The quiet hour began (03:00).
    QuietHour {
        on: bool,
    },
    // --- T5: moments, recipes, extras -----------------------------------------------------------
    /// A moment's glowing marker appears (`moment.<id>.title`).
    MomentAvailable {
        id: String,
        anchor: MomentPos,
    },
    /// It faded (`on_expire` applied; toast `moment.<id>.expired` when it had effects).
    MomentExpired {
        id: String,
    },
    /// Karolina arrived and the option applied (`moment.<id>.result.<option>`).
    MomentResolved {
        id: String,
        option: String,
    },
    ExperimentDone {
        result: ExperimentResult,
    },
    /// Grandma's margin note revealed an ingredient on a silhouette (`book.margin_note`).
    MarginNote {
        recipe: String,
        ingredient: String,
    },
    LatteUnlocked {
        id: String,
    },
    /// The station cat's friendship reached a level (`cat.level.<n>`).
    CatLevel {
        level: u32,
    },
    /// A thank-you napkin was collected (`thanks.<id>`).
    ThankYouNote {
        id: String,
    },
    /// A lifetime tip-jar goal: "Count the jar" → `Command::PickTipGoal`.
    TipGoalReached {
        goal: u32,
        choices: Vec<String>,
    },
    // --- T5: atmosphere ---------------------------------------------------------------------
    /// The coziness meter changed level (1..=5, `atmo.level.<n>`).
    CozinessLevel {
        level: u32,
    },
    /// Day start of a new season phase (`season.<id>.toast`).
    SeasonPhaseChanged {
        id: String,
    },
    /// A theme can be bought (`owned` = it was given for free).
    ThemeUnlocked {
        id: String,
        owned: bool,
    },
    ThemeChanged {
        id: String,
    },
    /// Ease the camera somewhere (moment / bot button); `tile` = the anchor when known.
    CameraHint {
        target: CameraTarget,
        tile: Option<(i32, i32)>,
    },
    /// The radio plays a track for a while (not the jukebox).
    RadioTrack {
        track: String,
        volume: f32,
        minutes: u32,
    },
    StreetMusic {
        minutes: u32,
    },
    WindowOpened {
        minutes: u32,
    },
    TrainLogged {
        train: String,
    },
    // --- T5: journal --------------------------------------------------------------------------
    /// A polaroid candidate (a tiny white flash in the HUD corner).
    HighlightCaptured {
        kind: HighlightKind,
        score: u32,
    },
    /// The morning's candidates are ready (`World::pending_polaroids`).
    PolaroidsReady {
        night: u32,
        count: u32,
    },
    JournalUpdated {
        night: u32,
    },
    // --- T5: rozpis & the app ---------------------------------------------------------------
    /// The draft changed: wellbeing per person, the week's fairness.
    RozpisScored {
        wellbeing: Vec<(u32, f32)>,
        fairness: f32,
    },
    /// Night start: a preference is broken tonight.
    PrefViolated {
        staff: u32,
        pref: PrefKind,
    },
    /// Every Need and Wish honoured, coverage fine (`app.bot.rozpis_happy`).
    RozpisHappy,
    RequestCreated {
        id: u32,
        kind: RequestKind,
        staff: u32,
    },
    RequestAnswered {
        id: u32,
        approved: bool,
    },
    /// "The café is ready." / "Closed properly." (`app.ritual.<list>`).
    RitualComplete {
        list: ChecklistKind,
    },
    /// Last night's handover notes acted (`app.handover.option.<id>`).
    HandoverApplied {
        ids: Vec<String>,
    },
    PayrollSent {
        week: u32,
        total: i64,
        tips_to_crew: i64,
    },
    /// A bot button deep-links into the app.
    OpenApp {
        link: AppLink,
    },
    /// A soft reminder chip (`RemindBefore`).
    Reminder {
        key: String,
        args: Vec<(String, String)>,
    },
    /// Karolina's micro-acts finished (for coach marks and little sounds).
    TableCleared {
        table: u32,
        dishes: u32,
        by: u32,
    },
    Washed {
        dishes: u32,
    },
    Mopped {
        tile: (i32, i32),
    },
    /// A brew started with `PlayerBrewStart` is ready to be handed over but still waits for its
    /// mini-game quality (`PlayerBrewQuality`).
    BrewAwaitsQuality {
        guest: u32,
        item: u32,
    },
    /// A tutorial step's `on_show` effects ran (`tutorial.ron`).
    TutorialEffects {
        id: String,
    },
    // --- v0.2 equipment (`sim::equipment`) ------------------------------------------------------
    /// An appliance chore is done (`by` = Karolina's id by hand, a helper's id, or `None`).
    EquipmentUsed {
        uid: u32,
        action: super::equipment::EquipAction,
        by: Option<u32>,
    },
    /// An appliance wants a little attention (the pot bubbles, the oven dings, a worn
    /// machine sputters) — a sound and a glint, never a must.
    EquipmentCue {
        uid: u32,
        cue: super::equipment::EquipCue,
    },
    /// "Count the till" at closing: tonight's numbers for the little count card.
    TillCounted {
        revenue: i64,
        guests: u32,
        tips: i64,
    },
    /// Karolina took the trash out; `cat` = the station cat was waiting by the bins.
    TrashOut {
        cat: bool,
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
    /// Fill the **draft** with the default rozpis (2–3 people a night, rotating days off,
    /// split shifts when overstaffed, time off respected) and let the app keep it current
    /// again on hires / time off. Undoable; publish as usual.
    AutoSchedule,
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
    /// Night only: send an on-duty person on a break right now (regular length, they eat if
    /// hungry). Works even when the auto rule would wait; emits `StaffBreak`, then
    /// `StaffBreakEnded`. Errors: `WrongPhase` by day, `Invalid` if they aren't on the floor
    /// or are already resting.
    SendOnBreak {
        staff: u32,
    },
    /// Day → Night (same as `World::start_night`).
    StartNight,
    /// "Open the doors now": ends the preparation half hour early (only before 22:00; the
    /// clock jumps to 22:00, everything keyed to 22:00 happens then).
    OpenDoors,
    // --- hands-on: Karolina behind the counter --------------------------------------------------
    /// Karolina walks over and takes the order (`guest` = any member or the party id). The
    /// party is claimed at once; error `InUse` if a helper already took it.
    PlayerTakeOrder {
        guest: u32,
    },
    /// Brew for a guest whose order she took: the leader's wish (any known recipe) or a plain
    /// item (a recipe on that menu item). `quality` (0–1, from the mini-game) is ignored for
    /// `Quick`. She walks to the station, works, then serves automatically.
    PlayerBrew {
        guest: u32,
        recipe: String,
        quality: f32,
        mode: BrewMode,
    },
    /// "The usual?" = `PlayerBrew` with the regular's usual recipe, by hand.
    PlayerUsual {
        guest: u32,
        quality: f32,
    },
    /// The wish becomes a plain ticket for the helpers (match capped, no story credit).
    PlayerHandOff {
        guest: u32,
    },
    /// Cancels the current act (claims are released).
    PlayerCancel,
    /// Queue a micro-act (`front` = do next).
    PlayerQueue {
        act: PlayerAct,
        front: bool,
    },
    /// Remove a queued act (index into `player().queue`).
    PlayerUnqueue {
        index: usize,
    },
    /// Hold-to-wash released after `seconds` (real) at the sink: 2 dishes per second.
    PlayerWashProgress {
        sink: u32,
        seconds: f32,
    },
    /// Wave at a guest who came in within the last few minutes (no walk).
    PlayerGreet {
        guest: u32,
    },
    /// Delegate / take back a duty. Capacity-checked (`CommandError::Capacity`); undoable.
    SetDuty {
        staff: u32,
        duty: Duty,
        on: bool,
    },
    /// Grandma's jukebox (`None` = house playlist). Night: Karolina's act; Day: tonight's
    /// opening record. Errors `Locked`, `NoJukebox`, `NoPower`.
    PlayRecord {
        id: Option<String>,
        repeat: bool,
    },
    /// A play-once pass ended (the integration measures `RecordDef::seconds`).
    RecordFinished {
        id: String,
    },
    /// Accessibility: helpers may hand off wishes left unclaimed for a while.
    SetAssistWishes {
        on: bool,
    },
    // --- T5 -------------------------------------------------------------------------------------
    /// Pick an option on a moment's card: Karolina walks to the anchor, the option applies on
    /// arrival (the card closes at once). Errors `UnknownId`, `Invalid` (hidden / too late).
    ResolveMoment {
        id: String,
        option: String,
    },
    /// Try 2–3 unlocked pantry ingredients (Day: at once; Night: Karolina spends
    /// `experiment_minutes` at the counter). Result: `SimEvent::ExperimentDone`.
    Experiment {
        ingredients: Vec<String>,
    },
    /// "Count the jar": keep one of the two decor pieces of the pending goal.
    PickTipGoal {
        choice: String,
    },
    /// Tonight's two specials on the chalkboard (known recipes; match +0.1).
    SetSpecials {
        recipes: Vec<String>,
    },
    /// Buy an unlocked decor theme once (Build → Theme).
    BuyTheme {
        id: String,
    },
    /// Switch to an owned theme (Day only; undoable).
    SetTheme {
        id: String,
    },
    /// Keep one of the morning's polaroid candidates (index into `pending_polaroids`).
    PickPolaroid {
        index: usize,
    },
    /// Same as `AutoSchedule` (handson.md naming): the pref-aware auto-fill.
    AutoFillRozpis,
    AnswerEarlyLeave {
        id: u32,
        approve: bool,
    },
    /// Up to 2 handover notes for tomorrow (ids from `World::handover_suggestions`).
    PickHandover {
        picks: Vec<String>,
    },
    /// The payroll screen's tip split (remembered).
    SetTipSplit {
        split: TipSplit,
    },
    /// Star of the week (200 Kč, mood +10).
    PayBonus {
        staff: u32,
    },
    /// Grant owed náhradní volno as a night off next week.
    GrantToil {
        staff: u32,
        weekday: u8,
    },
    /// Book the week (confirms the tip split and bonus). Auto at the next day start.
    SendPayroll {
        week: u32,
    },
    /// Same as `Experiment` (the book's pantry "Try").
    TryExperiment {
        ingredients: Vec<String>,
    },
    /// Start a brew while the mini-game plays: Karolina walks to the station and works; the
    /// quality follows with `PlayerBrewQuality` (she waits at the station with the cup until
    /// it arrives; after a while the skip quality applies). `PlayerBrew` stays the shortcut.
    PlayerBrewStart {
        guest: u32,
        recipe: String,
        mode: BrewMode,
    },
    /// The mini-game's quality (0–1) for the guest's brew started with `PlayerBrewStart`.
    PlayerBrewQuality {
        guest: u32,
        quality: f32,
    },
    /// The UI showed a tutorial step: its `on_show` effects run once (steps with an
    /// `At(time)` trigger run by themselves in story mode while the tutorial is on).
    TutorialShown {
        id: String,
    },
    /// The tutorial setting (default on): off = no scripted `on_show` effects.
    SetTutorial {
        on: bool,
    },
    // --- v0.2 equipment (`sim::equipment`) ------------------------------------------------------
    /// Karolina does an appliance chore (queued: she walks there and works). Errors
    /// `Invalid` when that chore isn't available now (`World::equipment_action`), `QueueFull`.
    UseEquipment {
        uid: u32,
        action: super::equipment::EquipAction,
    },
    /// The player holds the mouse / **E** on a hold chore she is doing (warm up, purge, clean
    /// the group head): `seconds` real seconds of holding speed it up.
    EquipmentHold {
        seconds: f32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    UnknownId(String),
    NotEnoughMoney {
        need: i64,
        have: i64,
    },
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
    /// The helper has no free duty slot.
    Capacity,
    /// Karolina's queue is full.
    QueueFull,
    /// The record (or recipe) isn't unlocked yet.
    Locked,
    NoJukebox,
    /// The power is out.
    NoPower,
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
