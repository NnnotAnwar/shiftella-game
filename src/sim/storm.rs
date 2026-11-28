//! The storm night (design/v0.2.md H.4): a snowstorm closes the line and the strangers in the
//! café stay till morning.
//!
//! * **When:** story night 29 (`stories.ron` → `storm.story_nights`), and in the sandbox a
//!   rare snow night (`sandbox_chance`, from `sandbox_from_night`, at most every
//!   `sandbox_gap` nights; rolled with a derived stream, never the world RNG). The night is
//!   snowy from the start (`StormForecast`).
//! * **`cancel_at` (00:50):** the station announces it and @shiftella_bot writes
//!   (`StormCancelled`). From then on the line is closed: trains don't come, nobody new walks in
//!   (people already on the platform still come over for `platform_minutes`); regulars still
//!   walk over — they live nearby. Every stranger inside stays till `stay_until` (06:00, the doors close): they
//!   don't run out of patience and don't leave after their meal.
//! * **Karolina's care (one click each):** `Command::StormBlankets` (she fetches the blankets
//!   from behind the counter and tucks everyone in) and `Command::StormSoup` (a big pot on the
//!   stove; a bowl for everyone, the pot bubbles). Both are walks with work at the end.
//! * **`talk_at` (03:00):** with at least `talk_min` stranded strangers they talk — a short
//!   shared scene (`StormTalk { talk, speakers }`; the screens play it with their persona
//!   names, which Karolina now knows). A special polaroid is captured (`polaroid_score`).
//! * **06:00:** `morning` effects (a little reputation) when she looked after them
//!   (`StormMorning { cared }`). No story flags, no money; both endings are untouched.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::guests::PendingSpawn;
use super::systems::{WalkKind, XTarget};
use super::types::*;
use super::world::*;
use crate::content::handson::{Fx, HighlightKind};
use crate::content::{ClockTime, Content, Job, Weather};

// ---------------------------------------------------------------------------------------------
// Data (`stories.ron` → `storm`)
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct StormDef {
    pub story_nights: Vec<u32>,
    pub sandbox_chance: f32,
    pub sandbox_from_night: u32,
    pub sandbox_gap: u32,
    pub cancel_at: ClockTime,
    pub talk_at: ClockTime,
    pub stay_until: ClockTime,
    pub platform_minutes: u32,
    pub talk_min: u32,
    pub polaroid_score: u32,
    pub blankets: CareDef,
    pub soup: CareDef,
    #[serde(default)]
    pub morning: Vec<Fx>,
    pub talks: Vec<TalkDef>,
}

/// One of Karolina's storm chores.
#[derive(Debug, Clone, Deserialize)]
pub struct CareDef {
    /// Work ticks at the counter / the stove.
    pub work: f32,
    /// + ticks per stranded guest.
    #[serde(default)]
    pub per_guest: f32,
    /// Run once (no target).
    #[serde(default)]
    pub effects: Vec<Fx>,
    /// Satisfaction + for every stranded party.
    pub satisfaction: f32,
}

/// The shared talk at 03:00.
#[derive(Debug, Clone, Deserialize)]
pub struct TalkDef {
    pub id: String,
    /// Stranded strangers it needs.
    pub min: u32,
    pub lines: Vec<TalkLine>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TalkLine {
    /// Speaker index into `StormTalk::speakers`; `None` = Karolina.
    #[serde(default)]
    pub who: Option<u32>,
    /// `{a}` `{b}` `{c}` = the speakers' names.
    pub key: String,
}

fn def() -> &'static StormDef {
    &super::stories::data().storm
}

/// A talk by id.
pub fn talk(id: &str) -> Option<&'static TalkDef> {
    def().talks.iter().find(|t| t.id == id)
}

// ---------------------------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------------------------

/// Karolina's storm chores.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StormAct {
    Blankets,
    Soup,
}

impl StormAct {
    pub fn id(self) -> &'static str {
        match self {
            StormAct::Blankets => "blankets",
            StormAct::Soup => "soup",
        }
    }
    pub fn from_id(id: &str) -> Option<StormAct> {
        match id {
            "blankets" => Some(StormAct::Blankets),
            "soup" => Some(StormAct::Soup),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StormState {
    /// Tonight is a storm night.
    pub tonight: bool,
    /// The line closed at this minute (after 22:00).
    pub cancelled_at: Option<u32>,
    /// Guests under one of grandma's blankets.
    pub blanketed: Vec<u32>,
    /// Guests with a bowl of the big pot's soup.
    pub fed: Vec<u32>,
    pub blankets_done: bool,
    pub soup_done: bool,
    /// Chores Karolina is on her way to do (queued).
    pub busy: Vec<StormAct>,
    /// Tonight's talk (id, speakers) once it played.
    pub talk: Option<(String, Vec<u32>)>,
    /// Talks heard in this game.
    pub heard: BTreeSet<String>,
    /// Storm nights so far (the sandbox gap).
    pub nights: Vec<u32>,
}

// ---------------------------------------------------------------------------------------------
// Night flow
// ---------------------------------------------------------------------------------------------

fn decide(w: &World) -> bool {
    let d = def();
    match w.mode {
        Mode::Story => d.story_nights.contains(&w.night),
        Mode::Sandbox => {
            let gap_ok = w
                .stories
                .storm
                .nights
                .last()
                .is_none_or(|n| w.night >= n + d.sandbox_gap);
            w.night >= d.sandbox_from_night
                && w.weather == Weather::Snow
                && gap_ok
                && super::stories::roll(w, 7).chance(d.sandbox_chance)
        }
    }
}

pub(crate) fn on_night_start(w: &mut World, _content: &Content) {
    let tonight = decide(w);
    let s = &mut w.stories.storm;
    s.tonight = tonight;
    s.cancelled_at = None;
    s.blanketed.clear();
    s.fed.clear();
    s.blankets_done = false;
    s.soup_done = false;
    s.busy.clear();
    s.talk = None;
    if !tonight {
        return;
    }
    s.nights.push(w.night);
    if w.weather != Weather::Snow {
        w.weather = Weather::Snow;
        w.emit(SimEvent::WeatherChanged {
            weather: Weather::Snow,
        });
    }
    w.emit(SimEvent::StormForecast);
}

pub(crate) fn on_night_end(w: &mut World, content: &Content) {
    let s = &w.stories.storm;
    if !s.tonight || s.cancelled_at.is_none() {
        return;
    }
    let cared = s.blankets_done || s.soup_done;
    if cared {
        let fx = def().morning.clone();
        super::xlogic::run(w, content, &fx, &XTarget::default());
    }
    w.emit(SimEvent::StormMorning { cared });
}

/// The line is closed tonight (no trains, no new strangers).
pub(crate) fn line_closed(w: &World) -> bool {
    w.phase == Phase::Night && w.stories.storm.cancelled_at.is_some()
}

/// Strangers who stay the night (guest ids; every non-regular party inside, members too).
pub(crate) fn stranded(w: &World) -> Vec<u32> {
    if w.stories.storm.cancelled_at.is_none() {
        return Vec::new();
    }
    w.parties
        .iter()
        .filter(|p| p.regular.is_none() && p.state != PartyState::Leaving)
        .flat_map(|p| p.members.iter().copied())
        .filter(|g| w.guests.iter().any(|x| x.id == *g && !x.gone))
        .collect()
}

fn stranded_parties(w: &World) -> Vec<usize> {
    (0..w.parties.len())
        .filter(|i| {
            let p = &w.parties[*i];
            p.regular.is_none() && p.state != PartyState::Leaving
        })
        .collect()
}

/// Each minute (called before the trains and guests): the cancellation, the closed line, the
/// stranded staying, the 03:00 talk.
pub(crate) fn on_minute(w: &mut World, content: &Content) {
    if !w.stories.storm.tonight {
        return;
    }
    let d = def();
    let m = w.minute();
    let cancel = d.cancel_at.minutes_since_open().max(0) as u32;
    if w.stories.storm.cancelled_at.is_none() && m >= cancel {
        w.stories.storm.cancelled_at = Some(m);
        // the people already on the platform hurry over
        for s in w.spawns.iter_mut() {
            s.minute = s.minute.min(m + 6);
        }
        let n = stranded(w).len() as u32;
        super::app::bot(w, content, "storm_cancel", &EffectCtx::default());
        w.emit(SimEvent::StormCancelled { stranded: n });
    }
    let Some(at) = w.stories.storm.cancelled_at else {
        return;
    };
    // nobody new: only the ones from the platform, within a few minutes
    let window = at + d.platform_minutes;
    w.spawns
        .retain(|s: &PendingSpawn| s.crowd && s.minute <= window);
    // the stranded stay till morning
    let until = d.stay_until.minutes_since_open().max(0) as u32;
    if m < until {
        for i in stranded_parties(w) {
            let p = &mut w.parties[i];
            p.leave_by = None;
            match p.state {
                PartyState::Eating => {
                    // they go out together into the morning, a few minutes apart
                    let spread = (p.id % 6) as f32;
                    p.stay_left = p.stay_left.max((until - m) as f32 + spread);
                }
                PartyState::Queue | PartyState::WaitOrder | PartyState::WaitFood => {
                    p.patience = p.patience.max(p.patience_max * 0.3).max(2.0);
                    p.wait_full = p.wait_full.max(p.patience);
                }
                _ => {}
            }
        }
    }
    let talk_at = d.talk_at.minutes_since_open().max(0) as u32;
    if m >= talk_at && w.stories.storm.talk.is_none() {
        start_talk(w, content);
    }
}

/// Speakers for the talk: stranded strangers seated, one per party first (by seat, left to
/// right), then their companions.
fn speakers(w: &World) -> Vec<u32> {
    let mut lead: Vec<(i32, i32, u32)> = Vec::new();
    let mut rest: Vec<(i32, i32, u32)> = Vec::new();
    for i in stranded_parties(w) {
        let p = &w.parties[i];
        for (k, g) in p.members.iter().enumerate() {
            let Some(x) = w.guests.iter().find(|x| x.id == *g && !x.gone) else {
                continue;
            };
            if x.persona.is_none() || x.seat.is_none() {
                continue;
            }
            let t = x.tile();
            if k == 0 {
                lead.push((t.0, t.1, x.id));
            } else {
                rest.push((t.0, t.1, x.id));
            }
        }
    }
    lead.sort();
    rest.sort();
    lead.into_iter().chain(rest).map(|x| x.2).collect()
}

fn start_talk(w: &mut World, content: &Content) {
    let d = def();
    let people = speakers(w);
    let n = people.len() as u32;
    let heard = &w.stories.storm.heard;
    // the biggest talk that fits, one not heard yet first
    let pick = d
        .talks
        .iter()
        .filter(|t| t.min <= n && n >= d.talk_min)
        .max_by_key(|t| (!heard.contains(&t.id), t.min))
        .cloned();
    let Some(t) = pick else {
        w.stories.storm.talk = Some((String::new(), Vec::new()));
        // still a photo of the snowed-in night
        polaroid(w, content, &[]);
        return;
    };
    let need = t
        .lines
        .iter()
        .filter_map(|l| l.who)
        .max()
        .map_or(0, |x| x as usize + 1);
    let who: Vec<u32> = people.into_iter().take(need.max(t.min as usize)).collect();
    // they say their names to each other (and to Karolina)
    for g in w.guests.iter_mut().filter(|g| who.contains(&g.id)) {
        if let Some(p) = g.persona.as_mut() {
            p.known = true;
        }
    }
    w.stories.storm.heard.insert(t.id.clone());
    w.stories.storm.talk = Some((t.id.clone(), who.clone()));
    polaroid(w, content, &who);
    w.emit(SimEvent::StormTalk {
        talk: t.id.clone(),
        speakers: who,
    });
}

fn polaroid(w: &mut World, content: &Content, who: &[u32]) {
    let tiles: Vec<(i32, i32)> = who
        .iter()
        .filter_map(|g| w.guests.iter().find(|x| x.id == *g).map(|x| x.tile()))
        .collect();
    let focus = if tiles.is_empty() {
        super::moments::anchor_window(w, content)
    } else {
        let n = tiles.len() as i32;
        Some((
            tiles.iter().map(|t| t.0).sum::<i32>() / n,
            tiles.iter().map(|t| t.1).sum::<i32>() / n,
        ))
    };
    let score = def().polaroid_score;
    super::stories::polaroid(
        w,
        content,
        HighlightKind::StoryScene,
        "storm",
        "journal.caption.storm".into(),
        Vec::new(),
        focus,
        score,
    );
}

// ---------------------------------------------------------------------------------------------
// Karolina's chores
// ---------------------------------------------------------------------------------------------

fn stove_tile(w: &World, content: &Content) -> Option<(i32, i32)> {
    let uid = w
        .furniture
        .iter()
        .find(|p| {
            content
                .furniture_def(&p.def)
                .is_some_and(|d| d.has_tag("station:stove"))
        })
        .map(|p| p.uid)?;
    super::staff::station_access(w, content, uid)
}

/// What a chore needs right now (`Ok(tile)` = where she goes).
pub(crate) fn can(w: &World, content: &Content, act: StormAct) -> Result<(i32, i32), CommandError> {
    if w.phase != Phase::Night || !w.doors_open() {
        return Err(CommandError::WrongPhase);
    }
    let s = &w.stories.storm;
    if s.cancelled_at.is_none() {
        return Err(CommandError::Invalid("the trains are running".into()));
    }
    if s.busy.contains(&act) {
        return Err(CommandError::Invalid("Karolina is on her way".into()));
    }
    let done = match act {
        StormAct::Blankets => s.blankets_done,
        StormAct::Soup => s.soup_done,
    };
    if done {
        return Err(CommandError::Invalid("done already".into()));
    }
    if stranded(w).is_empty() {
        return Err(CommandError::Invalid("nobody stayed".into()));
    }
    Ok(match act {
        StormAct::Blankets => super::staff::checklist_tile(w, content, Job::Register),
        StormAct::Soup => stove_tile(w, content)
            .unwrap_or_else(|| super::staff::checklist_tile(w, content, Job::Kitchen)),
    })
}

/// `Command::StormBlankets` / `Command::StormSoup`.
pub(crate) fn command(w: &mut World, content: &Content, act: StormAct) -> Result<(), CommandError> {
    let tile = can(w, content, act)?;
    super::player::queue_walk(
        w,
        content,
        tile,
        WalkKind::Storm {
            act: act.id().to_string(),
        },
    )?;
    w.stories.storm.busy.push(act);
    Ok(())
}

pub(crate) fn walk_work(w: &World, _content: &Content, act: &str) -> Option<f32> {
    let act = StormAct::from_id(act)?;
    let d = def();
    let n = stranded(w).len() as f32;
    Some(match act {
        StormAct::Blankets => d.blankets.work + d.blankets.per_guest * n,
        StormAct::Soup => d.soup.work + d.soup.per_guest * n,
    })
}

pub(crate) fn release(w: &mut World, act: &str) {
    if let Some(a) = StormAct::from_id(act) {
        w.stories.storm.busy.retain(|b| *b != a);
    }
}

/// The soup the bowls hold (the menu's soup if it's on tonight, else the first soup).
fn soup_item(w: &World, content: &Content) -> String {
    let items = &content.handson.tuning.equipment.soup.items;
    let on = w.menu_available(content);
    items
        .iter()
        .rev()
        .find(|i| on.contains(i))
        .or_else(|| items.first())
        .cloned()
        .unwrap_or_else(|| "garlic_soup".into())
}

pub(crate) fn arrive(w: &mut World, content: &Content, act: &str) {
    let Some(act) = StormAct::from_id(act) else {
        return;
    };
    w.stories.storm.busy.retain(|b| *b != act);
    let d = def();
    let care = match act {
        StormAct::Blankets => &d.blankets,
        StormAct::Soup => &d.soup,
    };
    let guests = stranded(w);
    for i in stranded_parties(w) {
        let p = &mut w.parties[i];
        p.satisfaction = (p.satisfaction + care.satisfaction).clamp(0.0, 100.0);
    }
    let tick = w.tick;
    match act {
        StormAct::Blankets => {
            w.stories.storm.blankets_done = true;
            w.stories.storm.blanketed = guests.clone();
            for g in w.guests.iter_mut().filter(|g| guests.contains(&g.id)) {
                g.emote = Some((SimEmote::Heart, tick + 40));
            }
            w.emit(SimEvent::StormCared {
                act: StormAct::Blankets,
                guests: guests.len() as u32,
            });
        }
        StormAct::Soup => {
            w.stories.storm.soup_done = true;
            w.stories.storm.fed = guests.clone();
            let item = soup_item(w, content);
            for g in w.guests.iter_mut().filter(|g| guests.contains(&g.id)) {
                if g.seat.is_some() {
                    g.holding = Some(item.clone());
                }
                g.emote = Some((SimEmote::Smile, tick + 40));
            }
            // the big pot bubbles on the stove
            let now = w.minutes_since_open();
            let e = &mut w.hands.equip;
            let pot = e.pot.get_or_insert_with(|| super::equipment::Pot {
                by_hand: true,
                since: now,
                next_bubble: now + 35,
                bubbling_until: None,
                stirred_until: None,
            });
            pot.bubbling_until = Some(now + 12);
            w.emit(SimEvent::StormCared {
                act: StormAct::Soup,
                guests: guests.len() as u32,
            });
        }
    }
    let fx = care.effects.clone();
    super::xlogic::run(w, content, &fx, &XTarget::default());
}

// ---------------------------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------------------------

impl World {
    /// The storm tonight (the line closes at `cancel_at`; see `sim::storm`).
    pub fn storm(&self) -> &StormState {
        &self.stories.storm
    }

    /// The line is closed by the snowstorm right now.
    pub fn storm_line_closed(&self) -> bool {
        line_closed(self)
    }

    /// A storm talk by id (`stories.ron` → `storm.talks`).
    pub fn storm_talk_def(&self, id: &str) -> Option<&'static TalkDef> {
        talk(id)
    }

    /// Strangers staying the night (guest ids).
    pub fn storm_stranded(&self) -> Vec<u32> {
        stranded(self)
    }

    /// Whether a storm chore can be done now (the chip's buttons).
    pub fn storm_can(&self, content: &Content, act: StormAct) -> bool {
        can(self, content, act).is_ok()
    }

    /// Tests / screenshot scenes: makes tonight a storm night (call before the cancellation
    /// minute; the snow starts now).
    pub fn debug_storm(&mut self) {
        let s = &mut self.stories.storm;
        if s.tonight {
            return;
        }
        s.tonight = true;
        let n = self.night;
        s.nights.push(n);
        if self.weather != Weather::Snow {
            self.weather = Weather::Snow;
            self.emit(SimEvent::WeatherChanged {
                weather: Weather::Snow,
            });
        }
    }
}
