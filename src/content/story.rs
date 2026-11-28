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

/// A night's planning title / hint that depend on a floating arc stage get a fallback: when
/// `needs` doesn't hold (checked live on the day screen, recorded when the night starts),
/// `title` / `hint` replace `story.night.NN.title` / `.hint` (each optional: `None` keeps the
/// primary one). Several fallbacks per night: the first whose `needs` fails wins.
#[derive(Debug, Clone, Deserialize)]
pub struct NightFallback {
    pub night: u32,
    pub needs: Vec<Condition>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub hint: Option<String>,
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
    /// The story calendar (dates of the 30 nights, named days).
    #[serde(default)]
    pub calendar: Option<StoryCalendar>,
    /// Night title / hint fallbacks (see [`NightFallback`]).
    #[serde(default)]
    pub night_fallbacks: Vec<NightFallback>,
}

impl StoryDef {
    /// The fallbacks of `night`, in order (indexes into `night_fallbacks`).
    pub fn fallbacks_of(&self, night: u32) -> impl Iterator<Item = (usize, &NightFallback)> {
        self.night_fallbacks
            .iter()
            .enumerate()
            .filter(move |(_, f)| f.night == night)
    }

    /// The title and hint keys of `night` given the fallback in use (`None` = the primary).
    pub fn night_text_keys(&self, night: u32, fallback: Option<usize>) -> Option<(String, String)> {
        let n = self.night(night)?;
        let fb = fallback.and_then(|i| self.night_fallbacks.get(i));
        Some((
            fb.and_then(|f| f.title.clone())
                .unwrap_or_else(|| n.title_key()),
            fb.and_then(|f| f.hint.clone())
                .unwrap_or_else(|| n.hint_key()),
        ))
    }
}

/// A calendar date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, serde::Serialize)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl Date {
    fn is_leap(year: i32) -> bool {
        (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
    }

    fn days_in_month(year: i32, month: u32) -> u32 {
        match month {
            2 if Date::is_leap(year) => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        }
    }

    /// The date `days` later.
    pub fn plus_days(self, days: u32) -> Date {
        let mut d = self;
        for _ in 0..days {
            d.day += 1;
            if d.day > Date::days_in_month(d.year, d.month) {
                d.day = 1;
                d.month += 1;
                if d.month > 12 {
                    d.month = 1;
                    d.year += 1;
                }
            }
        }
        d
    }

    /// Weekday (Zeller-style, Monday first).
    pub fn weekday(self) -> Weekday {
        let (mut y, mut m) = (self.year, self.month as i32);
        if m < 3 {
            m += 12;
            y -= 1;
        }
        let k = y % 100;
        let j = y / 100;
        let h = (self.day as i32 + 13 * (m + 1) / 5 + k + k / 4 + j / 4 + 5 * j) % 7;
        // h: 0 = Saturday … 6 = Friday → Monday-first index.
        Weekday::ALL[((h + 5) % 7) as usize]
    }

    pub fn is_valid(self) -> bool {
        (1..=12).contains(&self.month)
            && self.day >= 1
            && self.day <= Date::days_in_month(self.year, self.month)
    }
}

/// A named day of the story (`calendar.<id>`), shown on the day card and polaroids.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CalendarDay {
    pub id: String,
    pub night: u32,
}

/// Night 1's date and the named days. Night n = `start + (n − 1)` days; the night's weekday
/// in `nights` must match (validated).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct StoryCalendar {
    pub start: Date,
    #[serde(default)]
    pub days: Vec<CalendarDay>,
}

impl StoryCalendar {
    pub fn date_of_night(&self, night: u32) -> Date {
        self.start.plus_days(night.saturating_sub(1))
    }

    /// Named days falling on a night.
    pub fn days_on(&self, night: u32) -> impl Iterator<Item = &CalendarDay> {
        self.days.iter().filter(move |d| d.night == night)
    }

    pub fn day_key(day: &CalendarDay) -> String {
        format!("calendar.{}", day.id)
    }
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
    /// Only offered when these hold. A failing option is still shown, greyed out and not
    /// selectable, with a one-line reason (see [`ChoiceOption::locked_hint`]).
    #[serde(default)]
    pub conditions: Vec<Condition>,
    /// Text key of the reason shown under a locked option. Without it the player uses
    /// `<label_key>.locked` when that key exists, else a reason derived from the failing
    /// condition ("Needs: Armchair"), else `story_ui.locked.generic`.
    #[serde(default)]
    pub locked_hint: Option<String>,
    /// Opt out of the greyed-out display: a failing option is hidden instead (alternatives
    /// that exclude each other, e.g. two endings of one scene — not content to unlock).
    #[serde(default)]
    pub hidden_when_locked: bool,
}

impl ChoiceOption {
    /// The conventional reason key (`<label_key>.locked`), used when `locked_hint` is unset.
    pub fn conventional_hint_key(&self) -> String {
        format!("{}.locked", self.label_key)
    }
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
