//! `Condition` and `Effect` — the small scripting language shared by events, arcs, story beats,
//! cutscene choices, notes and endings. Exact semantics are documented here and in `docs/CONTENT.md`.
//!
//! **State the sim must keep** for these to work:
//! * `flags: HashSet<String>` — booleans (declared in `story.ron` → `flags`; `arc.<id>.good` /
//!   `arc.<id>.bad` are implicit for every regular).
//! * `vars: HashMap<String, String>` — small enums (declared in `story.ron` → `vars`, e.g. `letter`).
//! * `counters: HashMap<String, i32>` — numbers (declared in `story.ron` → `counters`;
//!   `arc.<regular>` is implicit for every regular).
//! * `items: HashSet<String>` — story items (declared in `story.ron` → `items`).
//! * `notes_found: HashSet<String>`.

use serde::Deserialize;

use super::defs::{ClockTime, GameMode, Job, Season, Weather, Weekday};

/// A predicate over game state. A list `Vec<Condition>` means **all** must hold (empty = true).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum Condition {
    // --- calendar & clock -----------------------------------------------------------------
    /// Night number ≥ n (story: 1..=30; sandbox: nights played).
    NightAtLeast(u32),
    NightAtMost(u32),
    /// Inclusive range.
    NightBetween(u32, u32),
    /// `from <= now < to` in night order (works across midnight; see `ClockTime`).
    TimeBetween(ClockTime, ClockTime),
    Weekday(Vec<Weekday>),
    Weather(Weather),
    /// Any wet weather (rain, downpour, storm).
    Raining,
    Season(Season),
    Mode(GameMode),
    /// Difficulty preset id from `difficulty.ron` (`cozy`, `classic`).
    Difficulty(String),

    // --- economy & reputation ------------------------------------------------------------
    MoneyAtLeast(i64),
    MoneyBelow(i64),
    /// Reputation 0–100.
    ReputationAtLeast(f32),
    ReputationBelow(f32),
    /// Every payment due so far has been paid.
    AllPaymentsMade,

    // --- story state ---------------------------------------------------------------------
    Flag(String),
    NotFlag(String),
    /// `vars[var] == value`.
    VarIs(String, String),
    /// `counters[name] >= n` (missing counter = 0). `arc.<regular>` is the arc stage.
    CounterAtLeast(String, i32),
    CounterBelow(String, i32),
    /// Number of Lida's notes found ≥ n.
    NotesFoundAtLeast(u32),
    NoteFound(String),
    HasItem(String),
    /// Arc stage of a regular ≥ n (same as `CounterAtLeast("arc.<id>", n)`).
    ArcAtLeast(String, u32),
    /// Number of regulars whose arc ended well (`arc.<id>.good` flags) ≥ n.
    ArcsGoodAtLeast(u32),
    /// The regular is in the café right now.
    RegularPresent(String),
    /// Regular-specific streak counter, e.g. "served within N minutes, K visits in a row":
    /// `ServedFastStreak { regular, minutes, visits }`. The sim keeps the streak per regular.
    ServedFastStreak {
        regular: String,
        minutes: u32,
        visits: u32,
    },
    /// The regular's preferred seat was free on their last N visits.
    SeatFreeStreak {
        regular: String,
        visits: u32,
    },
    /// Hall cleanliness ≥ fraction on the regular's last N visits.
    CleanStreak {
        regular: String,
        clean: f32,
        visits: u32,
    },
    /// The regular has been served this menu item at least once (any night).
    ServedItem {
        regular: String,
        item: String,
    },
    /// Relationship of the player/café with a regular (0–100, sim-tracked trust) ≥ n.
    TrustAtLeast(String, f32),
    /// The window light (lamp at the front window) is on.
    WindowLightOn,
    /// A train arrives at the café's station within ±N minutes of now.
    TrainNear(u32),

    // --- café & crew ----------------------------------------------------------------------
    /// At least one placed furniture piece with this tag.
    HasFurnitureTag(String),
    /// At least `n` pieces with this tag.
    FurnitureTagCount(String, u32),
    /// A specific furniture id is placed.
    HasFurniture(String),
    /// The menu item is unlocked and on the menu.
    MenuHas(String),
    /// Someone on shift has the trait.
    StaffWithTrait(String),
    /// Someone on shift has this archetype (story person).
    StaffArchetype(String),
    /// Staff on shift ≥ n.
    StaffOnShiftAtLeast(u32),
    /// Any staff member on shift has mood below n.
    StaffMoodBelow(f32),
    /// Any staff member on shift has mood above n and a skill ≥ `skill`.
    StaffInspired {
        mood: f32,
        skill: f32,
    },
    /// Two staff on shift with relationship below n.
    RelationshipBelow(f32),
    /// Two staff on shift with relationship above n (romantics use their own threshold).
    RelationshipAbove(f32),
    /// Guests waiting in the queue ≥ n.
    QueueAtLeast(u32),
    /// Guests in the café ≥ n.
    GuestsAtLeast(u32),
    /// Hall dirt ≥ fraction (0–1).
    DirtAtLeast(f32),
    /// Wear of any furniture with this tag ≥ fraction (0–1).
    WearAtLeast(String, f32),
    /// Sandbox town has this institution (`hospital`, `college`, `taxi_rank`, …).
    Institution(String),
    /// Another event is active right now / happened earlier tonight.
    EventActive(String),
    EventHappenedTonight(String),
    /// Previous N nights were "hard" (many bad events) — narrator safety valve.
    HardNightsInRow(u32),
    /// Random roll, 0–1. Evaluated once per check.
    Chance(f32),

    // --- combinators ----------------------------------------------------------------------
    All(Vec<Condition>),
    Any(Vec<Condition>),
    Not(Box<Condition>),
}

/// Something that happens. Durations in **game minutes**; `None` = until the end of the night.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum Effect {
    // --- money & reputation --------------------------------------------------------------
    /// Add (or subtract) CZK.
    Money(i64),
    /// Random amount in the inclusive range.
    MoneyRange(i64, i64),
    /// Change the remaining story debt (negative = paid off; clamped at 0). When the debt
    /// reaches 0, remaining payments count as made.
    Debt(i64),
    /// Reputation ± (0–100 scale, clamped).
    Reputation(f32),
    /// Next review of the given guest archetype (or any with `None`) counts × mult.
    ReviewWeight {
        guest: Option<String>,
        mult: f32,
        reviews: u32,
    },

    // --- guests --------------------------------------------------------------------------
    /// Multiply the guest arrival rate.
    GuestRate {
        mul: f32,
        minutes: Option<u32>,
    },
    /// A group of guests arrives within `within` minutes.
    GuestRush {
        guest: String,
        count: (u32, u32),
        within: u32,
        #[serde(default = "one")]
        patience_mul: f32,
        #[serde(default = "one")]
        spend_mul: f32,
        /// They order only these items (empty = their favourites).
        #[serde(default)]
        orders: Vec<String>,
    },
    /// Guests stay longer / shorter (minutes added to stay).
    GuestStay(i32),
    /// Satisfaction ± for every guest present (0–100).
    GuestSatisfaction(f32),
    /// Multiply tips.
    Tips {
        mul: f32,
        minutes: Option<u32>,
    },
    SpawnRegular(String),
    /// One-off uncanny or special guest of an archetype.
    SpawnGuest(String),

    // --- staff ---------------------------------------------------------------------------
    /// Mood ± for every staff member on shift.
    StaffMood(f32),
    /// Mood ± only for staff with this trait.
    TraitMood(String, f32),
    /// One random on-shift staff member is out for the night (calls in sick).
    StaffSick,
    /// The staff member with the lowest mood walks out until the end of the night.
    StaffWalkout,
    /// The two staff members in the triggering pair (quarrel / romance) get mood ±.
    PairMood(f32),
    /// Relationship ± between everyone on shift (per hour while the effect lasts, once if `minutes` is None).
    Relationship {
        add: f32,
        minutes: Option<u32>,
    },
    /// Work speed multiplier (`job: None` = all jobs).
    WorkSpeed {
        job: Option<Job>,
        mul: f32,
        minutes: Option<u32>,
    },
    /// Temporary skill bonus for the best skill of the triggering staff member.
    SkillBoost {
        add: f32,
        nights: u32,
    },

    // --- café ----------------------------------------------------------------------------
    /// Dirt ± (fraction 0–1 of the hall).
    Dirt(f32),
    DirtRate {
        mul: f32,
        minutes: Option<u32>,
    },
    /// Power is out: stations needing power stop, lights dim.
    PowerOutage {
        minutes: (u32, u32),
    },
    /// Furniture with this tag doesn't work for `minutes` (or until repaired if `None`).
    StationDown {
        tag: String,
        minutes: Option<u32>,
    },
    /// Furniture with this tag works slower until repaired.
    StationSlow {
        tag: String,
        mul: f32,
    },
    /// A menu item can't be ordered until the given time (`None` = rest of the night).
    MenuUnavailable {
        item: String,
        until: Option<ClockTime>,
    },
    /// Lose a share (0–1) of food stock.
    LoseStock(f32),
    /// Block one random hall tile (a bucket under the leak) until cleaned.
    BlockTile,
    /// Freeze the clock & animations for N real seconds (3:33).
    ClockStop {
        seconds: f32,
    },
    SetWeather(Weather),

    // --- story state ---------------------------------------------------------------------
    SetFlag(String),
    ClearFlag(String),
    SetVar(String, String),
    AddCounter(String, i32),
    GiveItem(String),
    TakeItem(String),
    /// Mark a note as found and show it.
    FindNote(String),
    /// Advance a regular's arc by one stage.
    AdvanceArc(String),
    /// Change the café's trust with a regular (0–100, see `Condition::TrustAtLeast`).
    Trust(String, f32),
    /// Mark a regular's arc as ended well / badly (sets `arc.<id>.good` / `arc.<id>.bad`).
    ArcGood(String),
    ArcBad(String),
    UnlockMenu(String),
    UnlockFurniture(String),

    // --- chaining & presentation ---------------------------------------------------------
    /// Fire another event now.
    StartEvent(String),
    /// Multiply an event's weight for N nights (0 = disable).
    EventWeight {
        event: String,
        mul: f32,
        nights: u32,
    },
    StartCutscene(String),
    /// Show a plain toast with this text key.
    Toast(String),
    /// Send an @shiftella_bot message from a template id in `shiftella.ron`.
    Bot(String),

    // --- control flow --------------------------------------------------------------------
    /// With probability `chance` (0–1) apply `effects`.
    Maybe {
        chance: f32,
        effects: Vec<Effect>,
    },
    /// Apply `effects` after `minutes` game minutes (dropped if the night ends first).
    Later {
        minutes: u32,
        effects: Vec<Effect>,
    },
    /// Evaluate `conditions` when this effect runs; apply `then` or `otherwise`.
    If {
        conditions: Vec<Condition>,
        then: Vec<Effect>,
        #[serde(default)]
        otherwise: Vec<Effect>,
    },
}

fn one() -> f32 {
    1.0
}

/// A reference from a condition / effect to another definition (for validation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ref<'a> {
    Menu(&'a str),
    Guest(&'a str),
    Regular(&'a str),
    Event(&'a str),
    Cutscene(&'a str),
    Note(&'a str),
    Trait(&'a str),
    Staff(&'a str),
    Furniture(&'a str),
    FurnitureTag(&'a str),
    Flag(&'a str),
    Var(&'a str, &'a str),
    Counter(&'a str),
    Item(&'a str),
    Difficulty(&'a str),
    Institution(&'a str),
    Bot(&'a str),
    TextKey(&'a str),
}

impl Condition {
    /// Calls `f` for every id this condition refers to.
    pub fn visit_refs<'a>(&'a self, f: &mut dyn FnMut(Ref<'a>)) {
        use Condition::*;
        match self {
            Difficulty(d) => f(Ref::Difficulty(d)),
            Flag(x) | NotFlag(x) => f(Ref::Flag(x)),
            VarIs(v, val) => f(Ref::Var(v, val)),
            CounterAtLeast(c, _) | CounterBelow(c, _) => f(Ref::Counter(c)),
            NoteFound(n) => f(Ref::Note(n)),
            HasItem(i) => f(Ref::Item(i)),
            ArcAtLeast(r, _) | RegularPresent(r) | TrustAtLeast(r, _) => f(Ref::Regular(r)),
            ServedFastStreak { regular, .. }
            | SeatFreeStreak { regular, .. }
            | CleanStreak { regular, .. } => f(Ref::Regular(regular)),
            ServedItem { regular, item } => {
                f(Ref::Regular(regular));
                f(Ref::Menu(item));
            }
            HasFurnitureTag(t) | FurnitureTagCount(t, _) | WearAtLeast(t, _) => {
                f(Ref::FurnitureTag(t))
            }
            HasFurniture(id) => f(Ref::Furniture(id)),
            MenuHas(m) => f(Ref::Menu(m)),
            StaffWithTrait(t) => f(Ref::Trait(t)),
            StaffArchetype(s) => f(Ref::Staff(s)),
            Institution(i) => f(Ref::Institution(i)),
            EventActive(e) | EventHappenedTonight(e) => f(Ref::Event(e)),
            All(list) | Any(list) => list.iter().for_each(|c| c.visit_refs(f)),
            Not(c) => c.visit_refs(f),
            NightAtLeast(_)
            | NightAtMost(_)
            | NightBetween(..)
            | TimeBetween(..)
            | Weekday(_)
            | Weather(_)
            | Raining
            | Season(_)
            | Mode(_)
            | MoneyAtLeast(_)
            | MoneyBelow(_)
            | ReputationAtLeast(_)
            | ReputationBelow(_)
            | AllPaymentsMade
            | NotesFoundAtLeast(_)
            | ArcsGoodAtLeast(_)
            | WindowLightOn
            | StaffOnShiftAtLeast(_)
            | StaffMoodBelow(_)
            | StaffInspired { .. }
            | RelationshipBelow(_)
            | RelationshipAbove(_)
            | QueueAtLeast(_)
            | GuestsAtLeast(_)
            | DirtAtLeast(_)
            | HardNightsInRow(_)
            | TrainNear(_)
            | Chance(_) => {}
        }
    }
}

impl Effect {
    /// Calls `f` for every id this effect refers to.
    pub fn visit_refs<'a>(&'a self, f: &mut dyn FnMut(Ref<'a>)) {
        use Effect::*;
        match self {
            ReviewWeight { guest: Some(g), .. } => f(Ref::Guest(g)),
            GuestRush { guest, orders, .. } => {
                f(Ref::Guest(guest));
                orders.iter().for_each(|o| f(Ref::Menu(o)));
            }
            SpawnRegular(r) | AdvanceArc(r) | ArcGood(r) | ArcBad(r) | Trust(r, _) => {
                f(Ref::Regular(r))
            }
            SpawnGuest(g) => f(Ref::Guest(g)),
            TraitMood(t, _) => f(Ref::Trait(t)),
            StationDown { tag, .. } | StationSlow { tag, .. } => f(Ref::FurnitureTag(tag)),
            MenuUnavailable { item, .. } | UnlockMenu(item) => f(Ref::Menu(item)),
            SetFlag(x) | ClearFlag(x) => f(Ref::Flag(x)),
            SetVar(v, val) => f(Ref::Var(v, val)),
            AddCounter(c, _) => f(Ref::Counter(c)),
            GiveItem(i) | TakeItem(i) => f(Ref::Item(i)),
            FindNote(n) => f(Ref::Note(n)),
            UnlockFurniture(id) => f(Ref::Furniture(id)),
            StartEvent(e) | EventWeight { event: e, .. } => f(Ref::Event(e)),
            StartCutscene(c) => f(Ref::Cutscene(c)),
            Toast(k) => f(Ref::TextKey(k)),
            Bot(b) => f(Ref::Bot(b)),
            Maybe { effects, .. } | Later { effects, .. } => {
                effects.iter().for_each(|e| e.visit_refs(f))
            }
            If {
                conditions,
                then,
                otherwise,
            } => {
                conditions.iter().for_each(|c| c.visit_refs(f));
                then.iter().chain(otherwise).for_each(|e| e.visit_refs(f));
            }
            Money(_)
            | MoneyRange(..)
            | Debt(_)
            | Reputation(_)
            | ReviewWeight { .. }
            | GuestRate { .. }
            | GuestStay(_)
            | GuestSatisfaction(_)
            | Tips { .. }
            | StaffMood(_)
            | StaffSick
            | StaffWalkout
            | PairMood(_)
            | Relationship { .. }
            | WorkSpeed { .. }
            | SkillBoost { .. }
            | Dirt(_)
            | DirtRate { .. }
            | PowerOutage { .. }
            | LoseStock(_)
            | BlockTile
            | ClockStop { .. }
            | SetWeather(_) => {}
        }
    }
}

/// Convenience: visit refs of a whole list.
pub fn visit_conditions<'a>(list: &'a [Condition], f: &mut dyn FnMut(Ref<'a>)) {
    list.iter().for_each(|c| c.visit_refs(f));
}

pub fn visit_effects<'a>(list: &'a [Effect], f: &mut dyn FnMut(Ref<'a>)) {
    list.iter().for_each(|e| e.visit_refs(f));
}
