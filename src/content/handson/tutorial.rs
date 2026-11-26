//! Data-driven coach marks for nights 1–4 (`assets/data/tutorial.ron`, handson.md §8).
//! Coach marks never block input or pause the clock; each has "Got it"; the whole tutorial
//! is a setting.

use serde::{Deserialize, Serialize};

use super::logic::Fx;
use crate::content::defs::ClockTime;

/// UI anchor ids the UI kit / integration must expose for coach marks (contract).
pub const TUTORIAL_WIDGETS: &[&str] = &[
    "order.card",
    "order.chip.0",
    "order.mode",
    "order.usual",
    "minigame",
    "minigame.skip",
    "rail",
    "hud.tipjar",
    "hud.lull",
    "hud.atmosphere",
    "book",
    "book.pantry",
    "duties.board",
    "hire.card",
    "rozpis.prefs",
    "rozpis.wellbeing",
    "app.checklist",
    "app.handover",
    "summary.polaroid",
    "jukebox.picker",
];

/// Something that happened (sim event or UI action) a step can wait for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum TutEvent {
    NightStarted,
    GuestArrived,
    Greeted,
    PlayerOrderTaken,
    BrewChosen,
    QuickBrewChosen,
    PlayerServed,
    TipLeft,
    TipCollected,
    TableCleared,
    DishesStacked,
    Washed,
    DirtBlot,
    Mopped,
    MomentAvailable,
    MomentResolved,
    LullChip,
    SpeedChanged,
    WindowLightOn,
    MachineWarmed,
    HandoverWritten,
    PolaroidShown,
    JournalOpened,
    BookOpened,
    RecipeDiscovered,
    JukeboxGlow,
    RecordChanged,
    HireCardShown,
    DutySet,
    RozpisOpened,
    SchedulePublished,
    UnownedTicket,
    CatArrived,
    UsualOffered,
    UsualChosen,
    AtmosphereLevel,
    WellbeingShown,
    GarlandsBox,
}

/// When a step appears.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum TutTrigger {
    /// At the given clock time on its night (day times = planning phase).
    At(ClockTime),
    /// The first time this happens on (or after) its night.
    Event(TutEvent),
    /// When the regular enters.
    RegularArrives(String),
    /// Right after another step is done or dismissed — for an `optional` step also once its
    /// mark has been shown for a few seconds without an answer.
    After(String),
}

/// Which guest a coach mark points at.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum GuestPick {
    First,
    Regular(String),
}

/// What a coach mark points at.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum TutAnchor {
    Guest(GuestPick),
    /// A UI widget id from [`TUTORIAL_WIDGETS`].
    Widget(String),
    /// The nearest placed furniture with this tag.
    Tag(String),
    Furniture(String),
    Moment(String),
    /// No arrow (a floating tip).
    Free,
}

/// When the step counts as done (besides "Got it").
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum TutDone {
    Event(TutEvent),
    /// Only "Got it" (or a timeout of `seconds` real seconds).
    GotIt {
        seconds: f32,
    },
}

/// Whose voice the tip is in: Karolina's thoughts, or Lida's note (amber italics).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
pub enum Voice {
    #[default]
    Karolina,
    Lida,
}

/// A coach mark (`tutorial.<id>`).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TutorialStep {
    pub id: String,
    /// Story night it belongs to (sandbox: the same nights played).
    pub night: u32,
    pub trigger: TutTrigger,
    pub anchor: TutAnchor,
    pub done_when: TutDone,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub voice: Voice,
    /// Applied when the step appears (a scripted early visit, a forced moment).
    #[serde(default)]
    pub on_show: Vec<Fx>,
}

impl TutorialStep {
    pub fn key(&self) -> String {
        format!("tutorial.{}", self.id)
    }
}
