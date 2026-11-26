//! The Shiftella app as a real tool: checklist rituals, bot action handlers, handover notes,
//! insights and the payroll decision (handson.md §13). Stored in `shiftella.ron`.

use serde::{Deserialize, Serialize};

use super::logic::{CameraTarget, Cond, Fx};
use super::rozpis::Duty;
use crate::content::defs::ClockTime;
use crate::content::world::ChecklistKind;

/// Who does a checklist item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum ChecklistOwner {
    /// Only Karolina (tapping it in the app queues it for her).
    Karolina,
    /// Karolina, or a helper holding this duty when free.
    Either(Duty),
}

/// The hands-on gesture of a checklist item or micro-action.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub enum Gesture {
    Click,
    /// Hold for N seconds.
    Hold(f32),
    Swipe,
    /// Pick something in a small card (specials, a record, handover notes).
    Pick,
}

/// "The café is ready." / "Closed properly." — a whole list done (`app.ritual.<list>`).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ritual {
    pub list: ChecklistKind,
    /// Must be complete by this time (`None` = by the end of the night).
    #[serde(default)]
    pub by: Option<ClockTime>,
    pub effects: Vec<Fx>,
}

impl Ritual {
    pub fn toast_key(&self) -> String {
        match self.list {
            ChecklistKind::Opening => "app.ritual.opening".into(),
            ChecklistKind::Closing => "app.ritual.closing".into(),
        }
    }
}

/// Deep links from insights and bot buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum AppLink {
    Rozpis,
    RecipeBook,
    Pantry,
    Build,
    Duties,
    Money,
    Journal,
    Board,
}

/// What a bot button does in the world (§13.4). Never clock-in/out.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum BotHandler {
    /// The late helper's duty tasks go to Karolina's rail until they arrive.
    CoverSelf,
    /// Temporarily moves the duty to an on-shift helper within capacity (−2 mood for them).
    CoverOther,
    /// An Off helper with no Need that night comes in (+60 min; wb −8 unless MinNights).
    CallIn,
    /// Reserve the regular's seat (SHOULD §6.3).
    ReserveSeat,
    /// A rail chip + a soft ping `minutes` before the subject time.
    RemindBefore(u32),
    /// Order the low ingredient group by drone (needs `drone_dock`; arrives in 30 min).
    OrderDrone {
        minutes: u32,
    },
    /// Take the subject item off tonight's menu.
    MenuOff,
    /// Spawn / apply a moment now.
    ApplyMoment(String),
    Camera(CameraTarget),
    Open(AppLink),
}

/// A sim-generated handover suggestion (`app.handover.option.<id>`), one tap each, ≤ 2 a night.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HandoverOption {
    pub id: String,
    /// Offered when these hold at closing.
    #[serde(default)]
    pub when: Vec<Cond>,
    /// What the note concerns; target effects act on it.
    pub subject: HandoverSubject,
    pub next_night: Vec<Fx>,
    #[serde(default)]
    pub placeholders: Vec<String>,
}

impl HandoverOption {
    pub fn key(&self) -> String {
        format!("app.handover.option.{}", self.id)
    }
}

/// The subject the sim fills a handover suggestion with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum HandoverSubject {
    None,
    /// The lowest-stock ingredient / item.
    LowItem,
    /// The most worn coffee machine.
    Machine,
    /// A regular with an arc hint.
    Regular,
    /// The train after which a rush happened.
    Train,
    /// A worn table.
    Table,
    Crew,
}

/// A digest insight (`app.insight.<id>`), at most 4 shown, each with a deep link.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InsightDef {
    pub id: String,
    pub link: AppLink,
    #[serde(default)]
    pub placeholders: Vec<String>,
}

impl InsightDef {
    pub fn key(&self) -> String {
        format!("app.insight.{}", self.id)
    }
}

/// Tip split choice on the payroll screen (`app.payroll.split.<id>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum TipSplit {
    Cafe,
    Half,
    Crew,
}

impl TipSplit {
    pub fn id(self) -> &'static str {
        match self {
            TipSplit::Cafe => "cafe",
            TipSplit::Half => "half",
            TipSplit::Crew => "crew",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TipSplitOption {
    pub split: TipSplit,
    /// Share of the week's tips that goes to the crew (0–1).
    pub crew_share: f32,
    /// Crew mood next week.
    pub crew_mood: f32,
    /// Crew wellbeing next week.
    pub crew_wellbeing: f32,
    #[serde(default)]
    pub default: bool,
}

/// The weekly payroll decision (§13.7).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PayrollRules {
    pub splits: Vec<TipSplitOption>,
    /// "Star of the week": CZK and mood.
    pub star_bonus: u32,
    pub star_mood: f32,
    /// Granting a night off for owed náhradní volno: wellbeing.
    pub toil_grant_wellbeing: f32,
}

impl PayrollRules {
    pub fn default_split(&self) -> Option<&TipSplitOption> {
        self.splits.iter().find(|s| s.default)
    }
}
