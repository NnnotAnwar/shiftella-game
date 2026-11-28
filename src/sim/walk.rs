//! Karolina's own small life (design/v0.2.md F): the walk home after the morning summary and
//! one break on the back step a night.
//!
//! * **The walk home** — `Command::WalkHome { route }` in the day after a night (once a day).
//!   The route's small cozy modifier runs when the next night starts (`on_night_start`), e.g.
//!   the bakery's koláč (coziness), the park (a calm hall), the embankment (steady hands), the
//!   tram (a nurse she met drops by with a wish). Never required: no walk = no modifier.
//! * **The back step** — `Command::StepOutBack` at night (doors open, not in a rush, within
//!   `from_minute..until_minute`, once a night): Karolina steps out for a breath and a message
//!   from mum, a friend or grandma's old neighbour arrives (a rotating pool by story act).
//!   No stats, no RNG: the UI holds the café while the scene is on screen.
//!
//! Neither touches story flags, trust, money or the world RNG, so the story plays out the same
//! (the player-bot never walks or steps out). Data: `assets/data/walk_home.ron` (compiled in).

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use super::guests::PendingSpawn;
use super::systems::XTarget;
use super::types::{CommandError, Mode, Phase};
use super::world::World;
use crate::content::handson::Fx;
use crate::content::{Content, Weather};

// ---------------------------------------------------------------------------------------------
// Data
// ---------------------------------------------------------------------------------------------

/// `assets/data/walk_home.ron`.
#[derive(Debug, Clone, Deserialize)]
pub struct WalkData {
    pub default_route: String,
    pub routes: Vec<RouteDef>,
    pub backstep: BackstepDef,
}

/// One way home.
#[derive(Debug, Clone, Deserialize)]
pub struct RouteDef {
    pub id: String,
    /// Run when the next night starts (21:30).
    #[serde(default)]
    pub effects: Vec<Fx>,
    /// A guest who comes in tonight because of this morning (the tram).
    #[serde(default)]
    pub walk_in: Option<WalkIn>,
    /// Lines on the way: per beat, the first matching variant.
    #[serde(default)]
    pub beats: Vec<Vec<LineVariant>>,
}

impl RouteDef {
    pub fn name_key(&self) -> String {
        format!("walk.route.{}", self.id)
    }
    /// The chip's hover: what the route gives tonight.
    pub fn hint_key(&self) -> String {
        format!("walk.route.{}.hint", self.id)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct WalkIn {
    pub archetype: String,
    /// Minutes after 22:00.
    pub minute: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LineVariant {
    pub key: String,
    /// Season phase ids (`seasons.ron`); empty = any.
    #[serde(default)]
    pub seasons: Vec<String>,
    /// Empty = any.
    #[serde(default)]
    pub weather: Vec<Weather>,
}

impl LineVariant {
    fn matches(&self, season: Option<&str>, weather: Weather) -> bool {
        (self.seasons.is_empty() || season.is_some_and(|s| self.seasons.iter().any(|x| x == s)))
            && (self.weather.is_empty() || self.weather.contains(&weather))
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct BackstepDef {
    /// Minutes after 22:00.
    pub from_minute: u32,
    pub until_minute: u32,
    pub messages: Vec<MessageDef>,
}

/// Who writes to Karolina on the back step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Sender {
    Mum,
    /// Terka, her friend from school.
    Friend,
    /// Marta, grandma Lida's old neighbour.
    Neighbour,
}

impl Sender {
    pub fn id(self) -> &'static str {
        match self {
            Sender::Mum => "mum",
            Sender::Friend => "friend",
            Sender::Neighbour => "neighbour",
        }
    }
    /// The sender's name (`walk.from.<id>`).
    pub fn name_key(self) -> String {
        format!("walk.from.{}", self.id())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageDef {
    pub id: String,
    pub from: Sender,
    /// Story acts (empty = any night, also the sandbox).
    #[serde(default)]
    pub acts: Vec<u32>,
}

impl MessageDef {
    /// The message text (`walk.msg.<id>`).
    pub fn text_key(&self) -> String {
        format!("walk.msg.{}", self.id)
    }
}

const DATA: &str = include_str!("../../assets/data/walk_home.ron");

/// The walk-home data (parsed once; a broken file is a test failure, at runtime an empty set).
pub fn data() -> &'static WalkData {
    static D: OnceLock<WalkData> = OnceLock::new();
    D.get_or_init(|| {
        parse(DATA).unwrap_or_else(|e| {
            eprintln!("error: walk_home.ron: {e}");
            WalkData {
                default_route: String::new(),
                routes: Vec::new(),
                backstep: BackstepDef {
                    from_minute: u32::MAX,
                    until_minute: 0,
                    messages: Vec::new(),
                },
            }
        })
    })
}

pub(crate) fn parse(text: &str) -> Result<WalkData, String> {
    ron::from_str(text).map_err(|e| e.to_string())
}

pub fn route(id: &str) -> Option<&'static RouteDef> {
    data().routes.iter().find(|r| r.id == id)
}

pub fn message(id: &str) -> Option<&'static MessageDef> {
    data().backstep.messages.iter().find(|m| m.id == id)
}

// ---------------------------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------------------------

/// Saved in `World.walk` (every field defaults: 0.1.x saves load with none of it).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct WalkState {
    /// The route walked this morning and the night it is for (applies when that night starts).
    pub planned: Option<(u32, String)>,
    /// Tonight's route modifier (set at night start, cleared at the next).
    pub tonight: Option<String>,
    /// The last route picked — her usual way home (the scene's default).
    pub usual: Option<String>,
    /// Walks per route.
    pub walks: BTreeMap<String, u32>,
    /// The night of the last back-step break (0 = never).
    pub backstep_night: u32,
    /// The message of that break.
    pub message: Option<String>,
    /// How often each message was shown.
    pub seen: BTreeMap<String, u32>,
    /// Breaks taken (all nights).
    pub breaks: u32,
}

/// Why the back step isn't open right now (the chip hides itself).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackstepBlock {
    NotNight,
    DoorsClosed,
    TooEarly,
    TooLate,
    Rush,
    AlreadyTonight,
    Scene,
}

// ---------------------------------------------------------------------------------------------
// Rules
// ---------------------------------------------------------------------------------------------

/// The story act of this night (sandbox: 0 — only general messages).
fn act(w: &World, content: &Content) -> u32 {
    if w.mode != Mode::Story {
        return 0;
    }
    content.story.night(w.night).map_or(0, |n| n.act)
}

pub(crate) fn walk_home(w: &mut World, route_id: &str) -> Result<(), CommandError> {
    if w.phase != Phase::Day {
        return Err(CommandError::WrongPhase);
    }
    if route(route_id).is_none() {
        return Err(CommandError::UnknownId(route_id.to_string()));
    }
    if w.walk.planned.as_ref().is_some_and(|p| p.0 == w.night) {
        return Err(CommandError::Invalid("already home".into()));
    }
    w.walk.planned = Some((w.night, route_id.to_string()));
    w.walk.usual = Some(route_id.to_string());
    *w.walk.walks.entry(route_id.to_string()).or_default() += 1;
    Ok(())
}

pub(crate) fn backstep_block(w: &World, content: &Content) -> Option<BackstepBlock> {
    let d = &data().backstep;
    if w.phase != Phase::Night || w.closed {
        return Some(BackstepBlock::NotNight);
    }
    if w.story.playing.is_some() {
        return Some(BackstepBlock::Scene);
    }
    if !w.doors_open() {
        return Some(BackstepBlock::DoorsClosed);
    }
    if w.walk.backstep_night == w.night {
        return Some(BackstepBlock::AlreadyTonight);
    }
    let m = w.minute();
    if m < d.from_minute {
        return Some(BackstepBlock::TooEarly);
    }
    if m >= d.until_minute {
        return Some(BackstepBlock::TooLate);
    }
    if w.rush() {
        return Some(BackstepBlock::Rush);
    }
    let _ = content;
    None
}

pub(crate) fn step_out_back(w: &mut World, content: &Content) -> Result<(), CommandError> {
    match backstep_block(w, content) {
        None => {}
        Some(BackstepBlock::NotNight | BackstepBlock::DoorsClosed) => {
            return Err(CommandError::WrongPhase);
        }
        Some(b) => return Err(CommandError::Invalid(format!("{b:?}"))),
    }
    let msg = pick_message(w, content).map(|m| m.id.clone());
    w.walk.backstep_night = w.night;
    w.walk.breaks += 1;
    if let Some(id) = &msg {
        *w.walk.seen.entry(id.clone()).or_default() += 1;
    }
    w.walk.message = msg;
    Ok(())
}

/// Tonight's message: unseen ones of this act first, then unseen general ones, then the least
/// seen of all eligible (ties: file order). Deterministic, no RNG.
pub(crate) fn pick_message(w: &World, content: &Content) -> Option<&'static MessageDef> {
    let act = act(w, content);
    let eligible: Vec<&MessageDef> = data()
        .backstep
        .messages
        .iter()
        .filter(|m| m.acts.is_empty() || (act > 0 && m.acts.contains(&act)))
        .collect();
    let seen = |m: &MessageDef| w.walk.seen.get(&m.id).copied().unwrap_or(0);
    eligible
        .iter()
        .find(|m| !m.acts.is_empty() && seen(m) == 0)
        .or_else(|| eligible.iter().find(|m| seen(m) == 0))
        .or_else(|| eligible.iter().min_by_key(|m| seen(m)))
        .copied()
}

/// Night start (after the handover notes): this morning's route applies tonight.
pub(crate) fn on_night_start(w: &mut World, content: &Content) {
    w.walk.tonight = None;
    let Some((night, id)) = w.walk.planned.take() else {
        return;
    };
    if night != w.night {
        return;
    }
    let Some(r) = route(&id) else { return };
    super::xlogic::run(w, content, &r.effects, &XTarget::default());
    if let Some(wi) = &r.walk_in
        && content.guest(&wi.archetype).is_some()
    {
        w.spawns.push(PendingSpawn {
            minute: wi.minute,
            archetype: wi.archetype.clone(),
            size: 1,
            patience_mul: 1.2,
            spend_mul: 1.1,
            orders: Vec::new(),
            wish: true,
            // the nurse Karolina met on the tram comes on her own, never held back
            crowd: false,
            held: 0,
        });
    }
    w.walk.tonight = Some(id);
}

/// The lines on the way for this season phase and weather (one key per beat).
pub fn lines<'a>(route: &'a RouteDef, season: Option<&str>, weather: Weather) -> Vec<&'a str> {
    route
        .beats
        .iter()
        .filter_map(|beat| {
            beat.iter()
                .find(|v| v.matches(season, weather))
                .map(|v| v.key.as_str())
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------------------------

impl World {
    pub fn walk(&self) -> &WalkState {
        &self.walk
    }
    /// Her usual way home (the last pick, else the data's default).
    pub fn usual_route(&self) -> &str {
        self.walk
            .usual
            .as_deref()
            .filter(|r| route(r).is_some())
            .unwrap_or(&data().default_route)
    }
    /// She already walked home this morning.
    pub fn walked_home_today(&self) -> bool {
        self.phase == Phase::Day
            && self
                .walk
                .planned
                .as_ref()
                .is_some_and(|p| p.0 == self.night)
    }
    /// The route whose modifier runs tonight.
    pub fn walk_tonight(&self) -> Option<&str> {
        self.walk.tonight.as_deref()
    }
    /// The lines of a route for the walk this morning (season phase of the coming night and
    /// the weather of the night that just ended).
    pub fn walk_lines(&self, content: &Content, route_id: &str) -> Vec<&'static str> {
        let Some(r) = route(route_id) else {
            return Vec::new();
        };
        let season = self.season_phase(content).map(|p| p.id.as_str());
        lines(r, season, self.weather)
    }
    /// `None` = the back step is open now (the HUD chip shows).
    pub fn backstep_block(&self, content: &Content) -> Option<BackstepBlock> {
        backstep_block(self, content)
    }
    /// Screenshot scenes only: ends a running rush so the back step can open on cue.
    #[doc(hidden)]
    pub fn debug_end_rush(&mut self) {
        self.hands.rush.until = None;
    }
    /// Tonight's back-step message (after `StepOutBack`).
    pub fn backstep_message(&self) -> Option<&'static MessageDef> {
        if self.walk.backstep_night != self.night || self.phase != Phase::Night {
            return None;
        }
        self.walk.message.as_deref().and_then(message)
    }
}
