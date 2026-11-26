//! Story mode: 30 nights, Lida's notes, the two endings, story state declarations and cutscenes.

use std::collections::HashMap;

use serde::Deserialize;

use super::defs::{ClockTime, Look, Weather, Weekday};
use super::logic::{Condition, Effect};

// ---------------------------------------------------------------------------------------------
// Story nights
// ---------------------------------------------------------------------------------------------

/// What happens at a beat.
#[derive(Debug, Clone, Deserialize)]
pub enum BeatKind {
    /// A speech bubble over `speaker` (non-blocking). Speaker ids as in cutscenes.
    Line { speaker: String, text_key: String },
    /// Fire a (usually `Scripted`) event.
    Event(String),
    /// Lida's note is found (shows the note card).
    Note(String),
    /// The night's payment is collected now (amount from `StoryDef::payments`).
    PaymentDue,
    /// Station board announcement for a train id from `trains.ron`.
    Train(String),
    /// Tutorial / hint toast (text key).
    Hint(String),
    /// @shiftella_bot message (template id).
    Bot(String),
    /// Weather changes.
    Weather(Weather),
    /// Plain effects (flags, unlocks).
    Apply(Vec<Effect>),
}

/// A timed beat. `at` before 22:00 means the day phase before the night.
#[derive(Debug, Clone, Deserialize)]
pub struct Beat {
    pub at: ClockTime,
    pub what: BeatKind,
    /// Beat is skipped unless these hold.
    #[serde(default)]
    pub conditions: Vec<Condition>,
}

/// `story.night.<NN>.title` and `story.night.<NN>.hint` (planning-screen line), NN = 01..30.
#[derive(Debug, Clone, Deserialize)]
pub struct StoryNight {
    pub night: u32,
    pub act: u32,
    pub weekday: Weekday,
    /// Forced weather (`None` = rolled).
    #[serde(default)]
    pub weather: Option<Weather>,
    pub beats: Vec<Beat>,
}

impl StoryNight {
    pub fn title_key(&self) -> String {
        format!("story.night.{:02}.title", self.night)
    }
    pub fn hint_key(&self) -> String {
        format!("story.night.{:02}.hint", self.night)
    }
}

/// `story.act.<n>.title`.
#[derive(Debug, Clone, Deserialize)]
pub struct ActDef {
    pub act: u32,
    pub nights: (u32, u32),
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaymentDef {
    pub night: u32,
    /// CZK, collected at 06:00 of that night.
    pub amount: i64,
    /// The last one: missing it in Classic means the Last Train ending.
    #[serde(default)]
    pub is_final: bool,
}

/// One of Lida's ten notes: `note.<id>.text` (the note itself) and `note.<id>.where`
/// (where it was found). Found when `found_when` holds (checked each game minute and at
/// night end) or via `Effect::FindNote` / `Beat::Note`.
#[derive(Debug, Clone, Deserialize)]
pub struct GrandmaNote {
    pub id: String,
    pub number: u32,
    #[serde(default)]
    pub found_when: Vec<Condition>,
    #[serde(default)]
    pub effects: Vec<Effect>,
}

/// A declared small enum of story state, e.g. `letter` ∈ {hidden, confessed, …}.
#[derive(Debug, Clone, Deserialize)]
pub struct VarDef {
    pub id: String,
    pub values: Vec<String>,
    pub default: String,
}

/// One of the two endings (`ending.<id>.title`, `ending.<id>.summary`).
///
/// At the end of night 30 (or when a Classic-mode closure happens earlier) endings are checked
/// in ascending `order`; the first whose `conditions` all hold wins. The last one has no
/// conditions and is the fallback. The ending's `cutscene` then plays; the cutscene itself
/// branches with `If` steps (variants, farewell, the optional "Answer" scene).
#[derive(Debug, Clone, Deserialize)]
pub struct EndingDef {
    pub id: String,
    pub order: u32,
    #[serde(default)]
    pub conditions: Vec<Condition>,
    pub cutscene: String,
    /// Named variants for the summary screen / achievements: first match wins
    /// (`ending.<id>.variant.<variant>`).
    #[serde(default)]
    pub variants: Vec<EndingVariant>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EndingVariant {
    pub id: String,
    #[serde(default)]
    pub conditions: Vec<Condition>,
}

/// The optional extra scene inside either ending (the letter reaches the 3:33 guest).
#[derive(Debug, Clone, Deserialize)]
pub struct EndingExtra {
    pub id: String,
    pub conditions: Vec<Condition>,
    pub cutscene: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StoryDef {
    pub nights_total: u32,
    pub debt: i64,
    pub payments: Vec<PaymentDef>,
    pub acts: Vec<ActDef>,
    pub nights: Vec<StoryNight>,
    pub notes: Vec<GrandmaNote>,
    pub endings: Vec<EndingDef>,
    pub ending_extra: EndingExtra,
    /// Declared boolean flags.
    pub flags: Vec<String>,
    pub vars: Vec<VarDef>,
    pub counters: Vec<String>,
    /// Declared story items (`item.<id>.name` / `.desc`).
    pub items: Vec<String>,
}

impl StoryDef {
    pub fn night(&self, n: u32) -> Option<&StoryNight> {
        self.nights.iter().find(|x| x.night == n)
    }
    pub fn note(&self, id: &str) -> Option<&GrandmaNote> {
        self.notes.iter().find(|x| x.id == id)
    }
    pub fn payment_on(&self, night: u32) -> Option<&PaymentDef> {
        self.payments.iter().find(|p| p.night == night)
    }
    /// Endings sorted by check order.
    pub fn endings_in_order(&self) -> Vec<&EndingDef> {
        let mut v: Vec<&EndingDef> = self.endings.iter().collect();
        v.sort_by_key(|e| e.order);
        v
    }
}

// ---------------------------------------------------------------------------------------------
// Cutscenes
// ---------------------------------------------------------------------------------------------

/// When a cutscene starts. Every cutscene plays at most once per save unless `repeat`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum CutsceneTrigger {
    /// Before the night begins (day phase → night), story mode.
    NightStart(u32),
    /// After 06:00 of that night, before the morning summary.
    NightEnd(u32),
    /// At a clock time during a story night.
    At { night: u32, time: ClockTime },
    /// As soon as the flag gets set.
    FlagSet(String),
    /// When a regular's arc reaches this stage.
    ArcStage { regular: String, stage: u32 },
    /// Played as the ending with this id.
    Ending(String),
    /// The player chose "Talk" on this regular. `night: None` = any night.
    /// The UI offers "Talk" only while an unplayed talk cutscene matches the current night.
    Talk { regular: String, night: Option<u32> },
    /// Only via `Call`, `Effect::StartCutscene` or an arc stage's `cutscene`.
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
pub enum Expression {
    #[default]
    Neutral,
    Smile,
    Sad,
    Surprised,
    Thinking,
    Laugh,
    Tired,
}

/// Camera target.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum Focus {
    Tile(i32, i32),
    /// Named spot from `cutscenes.ron` → `spots`.
    Spot(String),
    Actor(String),
    /// Whole café.
    Overview,
}

/// A place on the map: a tile or a named spot.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum Place {
    Tile(i32, i32),
    Spot(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChoiceOption {
    pub label_key: String,
    #[serde(default)]
    pub effects: Vec<Effect>,
    #[serde(default)]
    pub set_flag: Option<String>,
    /// Only offered when these hold.
    #[serde(default)]
    pub conditions: Vec<Condition>,
}

/// One step of a cutscene. Steps run in order; a step finishes before the next starts, except
/// `Camera`, `Music`, `Sfx`, `Letterbox` which start and continue immediately.
#[derive(Debug, Clone, Deserialize)]
pub enum CutsceneStep {
    /// Text in the dialogue window without a speaker (waits for click).
    Narration {
        text_key: String,
    },
    /// Dialogue window with portrait + name of `speaker` (waits for click).
    /// Speaker: a regular id, staff archetype id (story person), a cast id, `player`, `grandma`, `bot`.
    Line {
        speaker: String,
        #[serde(default)]
        expression: Expression,
        text_key: String,
    },
    /// Blocking choice inside the scene. Applies the option's effects and flag.
    Choice {
        options: Vec<ChoiceOption>,
    },
    /// Smoothly move the camera (`zoom` 1.0 = default night zoom).
    Camera {
        focus: Focus,
        zoom: f32,
    },
    /// Actor appears at the door (or the spot `door`) and walks to `to_tile`.
    ActorEnter {
        actor: String,
        to_tile: Place,
    },
    ActorMove {
        actor: String,
        to_tile: Place,
    },
    /// Actor walks to `to_tile` and disappears.
    ActorExit {
        actor: String,
        to_tile: Place,
    },
    Wait {
        seconds: f32,
    },
    Fade {
        to_black: bool,
        seconds: f32,
    },
    /// Switch music (`None` = fade out).
    Music {
        track: Option<String>,
    },
    Sfx {
        id: String,
    },
    /// A train passes the windows (light sweep + `amb_train_pass_far`).
    TrainPass,
    Letterbox {
        on: bool,
    },
    /// Show a note card (marks it found).
    ShowNote {
        note_id: String,
    },
    /// Run `then` if all conditions hold, else `otherwise`.
    If {
        conditions: Vec<Condition>,
        then: Vec<CutsceneStep>,
        #[serde(default)]
        otherwise: Vec<CutsceneStep>,
    },
    /// Play another cutscene inline, then continue.
    Call {
        cutscene: String,
    },
    /// Apply effects silently.
    Apply {
        effects: Vec<Effect>,
    },
}

/// Someone who speaks in cutscenes but isn't a regular or a staff archetype
/// (`cast.<id>.name`). `player`, `grandma` and `bot` are cast members too.
#[derive(Debug, Clone, Deserialize)]
pub struct CastMember {
    pub id: String,
    #[serde(default)]
    pub look: Look,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CutsceneDef {
    pub id: String,
    pub trigger: CutsceneTrigger,
    /// Plays only if these hold when triggered.
    #[serde(default)]
    pub conditions: Vec<Condition>,
    #[serde(default = "yes")]
    pub skippable: bool,
    #[serde(default)]
    pub repeat: bool,
    pub steps: Vec<CutsceneStep>,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
pub struct CutsceneLib {
    /// Named tiles on the default café map.
    pub spots: HashMap<String, (i32, i32)>,
    pub cast: Vec<CastMember>,
    pub cutscenes: Vec<CutsceneDef>,
}

impl CutsceneLib {
    pub fn get(&self, id: &str) -> Option<&CutsceneDef> {
        self.cutscenes.iter().find(|c| c.id == id)
    }

    /// Cutscenes that should start for this trigger.
    pub fn triggered_by<'a>(
        &'a self,
        trigger: &'a CutsceneTrigger,
    ) -> impl Iterator<Item = &'a CutsceneDef> + 'a {
        self.cutscenes.iter().filter(move |c| &c.trigger == trigger)
    }
}

/// Visits every step recursively (including `If` branches).
pub fn walk_steps<'a>(steps: &'a [CutsceneStep], f: &mut dyn FnMut(&'a CutsceneStep)) {
    for step in steps {
        f(step);
        if let CutsceneStep::If {
            then, otherwise, ..
        } = step
        {
            walk_steps(then, f);
            walk_steps(otherwise, f);
        }
    }
}
