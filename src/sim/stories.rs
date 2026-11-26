//! Little stories of strangers (design/v0.2.md H.3).
//!
//! Now and then a described stranger (never a regular) carries a two-line vignette — waiting
//! for a call, a birthday alone, a nurse after a hard shift. Karolina may notice it (the
//! screens draw a small amber bubble over them) and answer it by hand: `Command::AnswerStory
//! { guest, answer }` → she walks over, works a moment, and the answer's effects run with the
//! guest's party as the target. A *good* answer (the one that fits) also writes a night-log
//! line and a polaroid candidate; every answer gets a kind reaction. Never punishing: an
//! unanswered story fades after a while or when the guest leaves.
//!
//! At most `per_night` a night (one at a time), never during a rush, never the same vignette
//! twice in a story (the sandbox starts the pool over once every one was told). Everything
//! is rolled with a stream derived from the seed + night + minute — **no draws from the world
//! RNG** — so a player who never answers (and the player-bot) plays the very same night.
//! Data: `assets/data/stories.ron` (compiled in); the storm night lives in `sim::storm`.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use super::rng::{Rng, hash_str};
use super::systems::{Highlight, WalkKind, XTarget};
use super::types::*;
use super::world::*;
use crate::content::handson::{Fx, HighlightKind};
use crate::content::{ClockTime, Content};

// ---------------------------------------------------------------------------------------------
// Data
// ---------------------------------------------------------------------------------------------

/// `assets/data/stories.ron`.
#[derive(Debug, Clone, Deserialize)]
pub struct StoriesData {
    pub from_night: u32,
    /// Weights of a night's budget being 0, 1, 2… stories.
    pub per_night: Vec<f32>,
    pub from_minute: u32,
    pub until_minute: u32,
    pub gap_minutes: u32,
    pub expires_minutes: u32,
    pub settled_minutes: u32,
    pub work: f32,
    pub journal_score: u32,
    pub stories: Vec<StoryDef>,
    pub storm: super::storm::StormDef,
}

fn any_party() -> (u32, u32) {
    (1, 6)
}

fn one() -> f32 {
    1.0
}

/// One vignette.
#[derive(Debug, Clone, Deserialize)]
pub struct StoryDef {
    pub id: String,
    /// Guest archetypes it fits (empty = any stranger).
    #[serde(default)]
    pub archetypes: Vec<String>,
    #[serde(default)]
    pub female: Option<bool>,
    #[serde(default)]
    pub elder: Option<bool>,
    /// Party size (min, max).
    #[serde(default = "any_party")]
    pub party: (u32, u32),
    #[serde(default)]
    pub from: Option<ClockTime>,
    #[serde(default)]
    pub until: Option<ClockTime>,
    /// Only on a wet / snowy night.
    #[serde(default)]
    pub wet: bool,
    #[serde(default = "one")]
    pub weight: f32,
    pub answers: Vec<AnswerDef>,
}

impl StoryDef {
    /// What Karolina notices (`story.<id>.text`).
    pub fn text_key(&self) -> String {
        format!("story.{}.text", self.id)
    }
    /// The night-log line of a good answer (`{name}`).
    pub fn log_key(&self) -> String {
        format!("story.{}.log", self.id)
    }
    /// The polaroid caption of a good answer (`{name}`).
    pub fn caption_key(&self) -> String {
        format!("journal.caption.story.{}", self.id)
    }
    pub fn answer(&self, id: &str) -> Option<&AnswerDef> {
        self.answers.iter().find(|a| a.id == id)
    }
}

/// One answer on the card.
#[derive(Debug, Clone, Deserialize)]
pub struct AnswerDef {
    pub id: String,
    /// The answer that fits: log line + polaroid + the bigger reward.
    #[serde(default)]
    pub good: bool,
    #[serde(default)]
    pub effects: Vec<Fx>,
    /// Work ticks at the guest (default `StoriesData::work`).
    #[serde(default)]
    pub work: Option<f32>,
}

impl AnswerDef {
    /// The button (`story.<story>.opt.<id>`).
    pub fn label_key(&self, story: &str) -> String {
        format!("story.{story}.opt.{}", self.id)
    }
    /// The guest's reaction (`story.<story>.react.<id>`).
    pub fn react_key(&self, story: &str) -> String {
        format!("story.{story}.react.{}", self.id)
    }
}

const DATA: &str = include_str!("../../assets/data/stories.ron");

/// Parses a stories file (tests).
pub fn parse(text: &str) -> Result<StoriesData, String> {
    ron::from_str(text).map_err(|e| e.to_string())
}

/// The stories data (parsed once; a broken file is a test failure).
pub fn data() -> &'static StoriesData {
    static D: OnceLock<StoriesData> = OnceLock::new();
    D.get_or_init(|| parse(DATA).unwrap_or_else(|e| panic!("assets/data/stories.ron: {e}")))
}

/// A vignette by id.
pub fn def(id: &str) -> Option<&'static StoryDef> {
    data().stories.iter().find(|s| s.id == id)
}

// ---------------------------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------------------------

/// Stories and the storm (`World.stories`, `#[serde(default)]`: older saves start empty).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StoriesState {
    /// Vignettes told in this game (noticed, answered or not).
    pub told: BTreeSet<String>,
    /// Tonight's budget and how many appeared.
    pub budget: u32,
    pub tonight: u32,
    pub last_minute: Option<u32>,
    pub active: Vec<ActiveStory>,
    /// Tonight's answers: (story, answer, good).
    pub answered: Vec<(String, String, bool)>,
    /// Good answers over the whole game.
    pub warm_ever: u32,
    /// Guests who carried a story tonight (one story per stranger).
    pub guests_tonight: Vec<u32>,
    pub storm: super::storm::StormState,
}

/// A vignette on someone in the café now.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveStory {
    pub story: String,
    pub guest: u32,
    pub party: u32,
    pub spawned: u32,
    pub expires: u32,
    /// Karolina is on her way with this answer.
    #[serde(default)]
    pub resolving: Option<String>,
}

/// A stream for this night and `salt` (never the world RNG).
pub(crate) fn roll(w: &World, salt: u64) -> Rng {
    Rng::new(
        w.seed
            ^ hash_str("night_stories")
            ^ (w.night as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ salt.wrapping_mul(0xC2B2_AE3D_27D4_EB4F),
    )
}

/// A persona's name for a sim arg: `persona:<Latin>|<uk>|<ru>` (the screens pick the
/// language, `screens::game::stories_ui::persona_arg_text`).
pub fn persona_arg(p: &super::names::Persona) -> String {
    format!("persona:{}|{}|{}", p.name, p.name_in("uk"), p.name_in("ru"))
}

// ---------------------------------------------------------------------------------------------
// The director
// ---------------------------------------------------------------------------------------------

pub(crate) fn on_night_start(w: &mut World, content: &Content) {
    let d = data();
    let on = match w.mode {
        Mode::Story => w.night >= d.from_night,
        Mode::Sandbox => true,
    };
    let budget = if on {
        roll(w, 1).weighted(&d.per_night).unwrap_or(1) as u32
    } else {
        0
    };
    let st = &mut w.stories;
    st.active.clear();
    st.answered.clear();
    st.guests_tonight.clear();
    st.tonight = 0;
    st.last_minute = None;
    st.budget = budget;
    super::storm::on_night_start(w, content);
}

pub(crate) fn on_night_end(w: &mut World, content: &Content) {
    let open: Vec<ActiveStory> = std::mem::take(&mut w.stories.active);
    for a in open {
        w.emit(SimEvent::StoryFaded {
            guest: a.guest,
            story: a.story,
        });
    }
    super::storm::on_night_end(w, content);
}

fn guest_here(w: &World, guest: u32) -> bool {
    w.guests.iter().any(|g| g.id == guest && !g.gone)
        && w.party_of_guest(guest)
            .is_some_and(|p| p.state != PartyState::Leaving)
}

/// Each game minute (doors open): fading, then maybe a new story.
pub(crate) fn on_minute(w: &mut World, content: &Content) {
    let m = w.minute();
    let fade: Vec<usize> = w
        .stories
        .active
        .iter()
        .enumerate()
        .filter(|(_, a)| a.resolving.is_none() && (a.expires <= m || !guest_here(w, a.guest)))
        .map(|(i, _)| i)
        .collect();
    for i in fade.into_iter().rev() {
        let a = w.stories.active.remove(i);
        w.emit(SimEvent::StoryFaded {
            guest: a.guest,
            story: a.story,
        });
    }
    super::storm::on_minute(w, content);
    let d = data();
    let st = &w.stories;
    if !(d.from_minute..d.until_minute).contains(&m)
        || st.tonight >= st.budget
        || !st.active.is_empty()
        || st
            .last_minute
            .is_some_and(|l| m.saturating_sub(l) < d.gap_minutes)
        || w.hands.rush.until.is_some()
        || w.karolina_todo(content) > 4
    {
        return;
    }
    // spread the budget over the night (late in the night it catches up)
    let left = (st.budget - st.tonight) as f32;
    let slots = ((d.until_minute - m) as f32 / 20.0).max(1.0);
    let p = (left / slots * 1.2).clamp(0.03, 1.0);
    let mut r = roll(w, 1000 + m as u64);
    if !r.chance(p) {
        return;
    }
    let cands = candidates(w);
    if cands.is_empty() {
        return;
    }
    let weights: Vec<f32> = cands.iter().map(|c| c.2).collect();
    let Some(i) = r.weighted(&weights) else {
        return;
    };
    let (story, guest, _) = cands[i].clone();
    spawn(w, &story, guest);
}

/// Every (story, guest, weight) that could appear now.
fn candidates(w: &World) -> Vec<(String, u32, f32)> {
    let d = data();
    let m = w.minute();
    let told = &w.stories.told;
    // the sandbox tells the pool again once every story was told
    let fresh =
        |id: &str| !told.contains(id) || (w.mode == Mode::Sandbox && told.len() >= d.stories.len());
    let wet = w.weather.is_wet() || w.weather == crate::content::Weather::Snow;
    let mut out = Vec::new();
    for p in &w.parties {
        if p.regular.is_some()
            || !matches!(
                p.state,
                PartyState::WaitOrder | PartyState::WaitFood | PartyState::Eating
            )
            || m.saturating_sub(p.arrived) < d.settled_minutes
            || (p.state == PartyState::Eating && p.stay_left < 12.0)
        {
            continue;
        }
        let Some(leader) = p.leader() else { continue };
        if w.stories.guests_tonight.contains(&leader) {
            continue;
        }
        let Some(g) = w.guests.iter().find(|g| g.id == leader && !g.gone) else {
            continue;
        };
        let Some(persona) = g.persona.as_ref() else {
            continue;
        };
        if g.seat.is_none() {
            continue;
        }
        // a moment is already about them
        let busy = w.sys.moments.active.iter().any(|a| {
            a.target.party == Some(p.id) || a.anchor.agent == Some(AgentRef::Guest(leader))
        });
        if busy {
            continue;
        }
        let size = p
            .members
            .iter()
            .filter(|x| w.guests.iter().any(|g| g.id == **x && !g.gone))
            .count() as u32;
        for s in &d.stories {
            let fits = fresh(&s.id)
                && (s.archetypes.is_empty() || s.archetypes.contains(&p.archetype))
                && s.female.is_none_or(|f| f == persona.female)
                && s.elder.is_none_or(|e| e == persona.elder)
                && (s.party.0..=s.party.1).contains(&size)
                && s.from.is_none_or(|t| m as i32 >= t.minutes_since_open())
                && s.until.is_none_or(|t| (m as i32) < t.minutes_since_open())
                && (!s.wet || wet);
            if fits {
                out.push((s.id.clone(), leader, s.weight.max(0.01)));
            }
        }
    }
    out
}

/// A story appears on `guest` (tests and screenshot scenes may force one).
pub(crate) fn spawn(w: &mut World, story: &str, guest: u32) -> bool {
    let Some(_def) = def(story) else {
        return false;
    };
    let Some(party) = w.party_of_guest(guest).map(|p| p.id) else {
        return false;
    };
    if w.stories.active.iter().any(|a| a.guest == guest) {
        return false;
    }
    let m = w.minute();
    let d = data();
    let st = &mut w.stories;
    if w.mode == Mode::Sandbox && st.told.len() >= d.stories.len() {
        st.told.clear();
    }
    st.told.insert(story.to_string());
    st.guests_tonight.push(guest);
    st.tonight += 1;
    st.last_minute = Some(m);
    st.active.push(ActiveStory {
        story: story.to_string(),
        guest,
        party,
        spawned: m,
        expires: m + d.expires_minutes.max(1),
        resolving: None,
    });
    w.emit(SimEvent::StoryNoticed {
        guest,
        story: story.to_string(),
    });
    true
}

// ---------------------------------------------------------------------------------------------
// Answering
// ---------------------------------------------------------------------------------------------

fn stand_tile(w: &World, guest: u32) -> Option<(i32, i32)> {
    let g = w.guests.iter().find(|g| g.id == guest)?;
    let from = super::player::karolina_idx(w)
        .map(|k| w.staff[k].tile())
        .unwrap_or_else(|| w.grid.entrance());
    super::staff::stand_next_to(w, g.tile(), from).or(Some(g.tile()))
}

/// `Command::AnswerStory`: Karolina walks to the guest; the answer applies when she's there.
pub(crate) fn answer(
    w: &mut World,
    content: &Content,
    guest: u32,
    answer: &str,
) -> Result<(), CommandError> {
    if w.phase != Phase::Night || !w.doors_open() {
        return Err(CommandError::WrongPhase);
    }
    let a = w
        .stories
        .active
        .iter()
        .find(|a| a.guest == guest)
        .ok_or_else(|| CommandError::Invalid("no story here".into()))?;
    if a.resolving.is_some() {
        return Err(CommandError::Invalid("Karolina is on her way".into()));
    }
    let d = def(&a.story).ok_or_else(|| CommandError::UnknownId(a.story.clone()))?;
    if d.answer(answer).is_none() {
        return Err(CommandError::UnknownId(answer.to_string()));
    }
    let tile =
        stand_tile(w, guest).ok_or_else(|| CommandError::Invalid("can't get there".into()))?;
    super::player::queue_walk(
        w,
        content,
        tile,
        WalkKind::Story {
            guest,
            answer: answer.to_string(),
        },
    )?;
    if let Some(a) = w.stories.active.iter_mut().find(|a| a.guest == guest) {
        a.resolving = Some(answer.to_string());
    }
    Ok(())
}

/// Work ticks at the end of a story / storm walk (`None` = nothing to do there any more).
pub(crate) fn walk_work(w: &World, content: &Content, kind: &WalkKind) -> Option<f32> {
    match kind {
        WalkKind::Story { guest, answer } => {
            let a = w.stories.active.iter().find(|a| a.guest == *guest)?;
            let d = def(&a.story)?;
            let ans = d.answer(answer)?;
            Some(ans.work.unwrap_or(data().work).max(1.0))
        }
        WalkKind::Storm { act } => super::storm::walk_work(w, content, act),
        _ => None,
    }
}

/// Karolina got there.
pub(crate) fn arrive(w: &mut World, content: &Content, kind: &WalkKind) {
    match kind {
        WalkKind::Story { guest, answer } => resolve(w, content, *guest, answer),
        WalkKind::Storm { act } => super::storm::arrive(w, content, act),
        _ => {}
    }
}

/// Her act was cancelled / unqueued: the card can be answered again.
pub(crate) fn release(w: &mut World, kind: &WalkKind) {
    match kind {
        WalkKind::Story { guest, .. } => {
            if let Some(a) = w.stories.active.iter_mut().find(|a| a.guest == *guest) {
                a.resolving = None;
            }
        }
        WalkKind::Storm { act } => super::storm::release(w, act),
        _ => {}
    }
}

fn resolve(w: &mut World, content: &Content, guest: u32, answer: &str) {
    let Some(i) = w.stories.active.iter().position(|a| a.guest == guest) else {
        return;
    };
    let a = w.stories.active.remove(i);
    let Some(d) = def(&a.story) else {
        return;
    };
    let Some(ans) = d.answer(answer) else {
        return;
    };
    let tile = w.guests.iter().find(|g| g.id == guest).map(|g| g.tile());
    let target = XTarget {
        party: Some(a.party),
        tile,
        ..Default::default()
    };
    let effects = ans.effects.clone();
    super::xlogic::run(w, content, &effects, &target);
    let tick = w.tick;
    if let Some(g) = w.guests.iter_mut().find(|g| g.id == guest) {
        let e = if ans.good {
            SimEmote::Sparkle
        } else {
            SimEmote::Smile
        };
        g.emote = Some((e, tick + 40));
    }
    w.stories
        .answered
        .push((a.story.clone(), answer.to_string(), ans.good));
    if ans.good {
        w.stories.warm_ever += 1;
    }
    w.emit(SimEvent::StoryAnswered {
        guest,
        story: a.story.clone(),
        answer: answer.to_string(),
        good: ans.good,
    });
    if ans.good {
        let args = w
            .persona(guest)
            .map(|p| vec![("name".to_string(), persona_arg(p))])
            .unwrap_or_default();
        let score = data().journal_score;
        polaroid(
            w,
            content,
            HighlightKind::MomentWarm,
            &format!("story.{}", a.story),
            d.caption_key(),
            args,
            tile,
            score,
        );
    }
    // after a word like that, they tell her their name
    super::names::introduce(w, guest, super::names::Intro::Greet);
}

/// A polaroid candidate with its own caption (the journal's insert rules: one per kind + sub
/// a night, the repeat-kind penalty, the top `keep_per_night`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn polaroid(
    w: &mut World,
    content: &Content,
    kind: HighlightKind,
    once: &str,
    caption_key: String,
    args: Vec<(String, String)>,
    focus: Option<(i32, i32)>,
    score: u32,
) {
    if w.phase != Phase::Night {
        return;
    }
    let jf = &content.handson.journal;
    if !w
        .sys
        .journal
        .tonight_kinds
        .insert(format!("{}:{once}", kind.id()))
    {
        return;
    }
    let mut score = score;
    // (the storm's photo is once a story: no "same kind as yesterday" penalty)
    if w.sys.journal.last_kinds.contains(&kind) && once != "storm" {
        score = score.saturating_sub(jf.repeat_kind_penalty);
    }
    let keep = jf.keep_per_night.max(1) as usize;
    let snap = super::journal::snap(w, content, focus);
    let h = Highlight {
        night: w.night,
        minute: w.minute(),
        kind,
        score,
        caption_key,
        args,
        snap,
        before: None,
    };
    let list = &mut w.sys.journal.tonight;
    let pos = list
        .iter()
        .position(|x| x.score < score)
        .unwrap_or(list.len());
    if pos >= keep {
        return;
    }
    list.insert(pos, h);
    list.truncate(keep);
    w.emit(SimEvent::HighlightCaptured { kind, score });
}

// ---------------------------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------------------------

impl World {
    /// The little stories on people in the café now (markers; `resolving` = she's on her way).
    pub fn stories_active(&self) -> &[ActiveStory] {
        &self.stories.active
    }

    /// A vignette's data (`stories.ron`).
    pub fn story_def(&self, id: &str) -> Option<&'static StoryDef> {
        def(id)
    }

    /// The story on this guest, if any.
    pub fn story_of(&self, guest: u32) -> Option<&ActiveStory> {
        self.stories.active.iter().find(|a| a.guest == guest)
    }

    /// Stories and the storm (told, tonight's answers, the storm state).
    pub fn stories(&self) -> &StoriesState {
        &self.stories
    }

    /// Tests / screenshot scenes: puts `story` on `guest` now (no budget or filter check).
    pub fn debug_story(&mut self, story: &str, guest: u32) -> bool {
        spawn(self, story, guest)
    }
}
