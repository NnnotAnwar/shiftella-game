//! Hands-on rituals (the owner: "more interesting than just clicking a tick"). When Karolina
//! herself does an opening / closing checklist item — or an appliance chore — and the player is
//! watching, the UI plays a tiny gesture over the piece (`screens::game::rituals_ui`; the
//! gestures are data: `handson.ron` → `rituals`). The gesture enters the sim only through
//! [`Command::Ritual`](super::Command::Ritual):
//!
//! * `Start` — the player has begun the gesture: once she stands at the piece she waits for
//!   it (at most `rituals.patience_ticks` ticks of waiting, then she finishes on her own);
//! * `Done { perfect }` — the gesture is finished: the item completes at once (a little sooner
//!   than her own work would), and a perfect one is remembered for tonight's rituals;
//! * `LetGo` — Esc / right-click: she simply finishes it herself as she always did.
//!
//! Without these commands nothing here changes a thing: ignored gestures, ×2 / ×3, the
//! player-bot, headless runs and helpers all complete the item with the same ticks as before.
//! A list done with ≥ `rituals.perfect_min` perfect gestures in it gets its ritual's small
//! `perfect` reward (`shiftella.ron`, `SimEvent::RitualPerfect`), never needed for progress.
//!
//! What doing it by hand gives (0.2.6, the owner: "doing the gestures has no visible
//! effect"). Never a punishment: an item she does herself works exactly as before.
//!
//! * **Stronger** — an item done by hand runs its `hand_done` effects on top of its usual
//!   ones, a perfect gesture `perfect_done` as well (`shiftella.ron`; the ✋ in the checklist
//!   says what, `ritual.hand.<id>`).
//! * **The evening mood** — one point per opening item done by hand (0–7; a perfect gesture
//!   counts the same, it already strengthens the item). Each point done before 23:00
//!   (`rituals.mood.minutes`) makes the guests × `mood.patience` more patient and tip
//!   × `mood.tips` more until then (a chip by the clock). Each closing item done by hand puts
//!   `rituals.crew_mood` on the crew's mood when the next night starts.
//! * **Grandma's habits** — an item done by hand on `rituals.habit_nights` nights (in total)
//!   becomes one of Lída's habits: her note (`habit.<id>.note`, a card) and its small perk
//!   for good (`habit`, applied whenever the item is done, by anyone). One a night at most:
//!   another one due waits for the next time its item is done. Kept in the save
//!   ([`Habits`]); the player-bot never does a gesture, so it never learns one.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::hands::PlayerAct;
use super::types::*;
use super::world::{ModKind, Modifier, World};
use crate::content::handson::RitualGesture;
use crate::content::{ChecklistKind, Content};

/// What the player did with a ritual gesture (`Command::Ritual`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RitualStep {
    /// The gesture has begun: she waits for it at the piece.
    Start,
    /// The gesture is finished (`perfect` = done well).
    Done { perfect: bool },
    /// Let go (Esc / right-click): she finishes on her own.
    LetGo,
}

/// The gesture in the player's hands now.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RitualHold {
    /// The gesture key ([`ritual_key`]).
    pub key: String,
    /// Ticks she has stood waiting at the piece for it.
    pub waited: u32,
    /// The gesture is finished (the item completes on her next work tick).
    pub done: bool,
    pub perfect: bool,
}

/// Tonight's rituals (reset when the night starts).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RitualState {
    pub hold: Option<RitualHold>,
    /// Items (checklist id, or the gesture key of a chore without one) Karolina did by hand
    /// tonight → perfect.
    pub by_hand: BTreeMap<String, bool>,
    /// The `perfect` reward of [opening, closing] was given tonight.
    pub rewarded: [bool; 2],
    /// Points of the evening mood that act tonight (opening items by hand before
    /// `rituals.mood.minutes`).
    pub mood_acting: u32,
    /// Kč of tonight's tips that came from the evening mood.
    pub mood_tips: f32,
    /// Crew mood the closing done by hand tonight puts on the next night.
    pub crew: f32,
    /// Crew mood last night's closing put on tonight's crew.
    pub crew_from_last: f32,
    /// Habits learned tonight.
    pub learned: Vec<String>,
}

/// Grandma's habits, for the whole game (never reset; old saves load empty).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Habits {
    /// Checklist item → nights Karolina did it by hand (in total).
    pub nights: BTreeMap<String, u32>,
    /// The items that became her habits, in the order learned.
    pub learned: Vec<String>,
    /// Crew mood waiting for the next night (the closing done by hand).
    pub crew_next: f32,
}

/// The evening mood now (the HUD chip).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoodView {
    /// Opening items done by hand tonight (0–`max`).
    pub points: u32,
    pub max: u32,
    /// Guests' patience and tips × these until `until` (minutes after 22:00).
    pub patience: f32,
    pub tips: f32,
    pub until: u32,
    /// The mood still acts (21:30 → `until`).
    pub active: bool,
}

/// The checklist item an act of Karolina's ticks (the item itself, the appliance chore that
/// ticks it, the record put on for `record_on`).
fn item_of(act: &PlayerAct) -> Option<&str> {
    match act {
        PlayerAct::Checklist { id } => Some(id),
        PlayerAct::Use { action, .. } => action.checklist(),
        PlayerAct::PlayRecord { .. } => Some("record_on"),
        _ => None,
    }
}

fn item_open(w: &World, id: &str) -> bool {
    w.app.checklist.iter().any(|c| c.id == id && !c.done)
}

/// The gesture key of an act of Karolina's, if it has a gesture now: a checklist item's id,
/// `equip.<action>` for an appliance chore with its own gesture (else the item it ticks), the
/// record put on while `record_on` is still open. `None` for everything else.
pub fn ritual_key(w: &World, content: &Content, act: &PlayerAct) -> Option<String> {
    let g = &content.handson.tuning.rituals;
    match act {
        PlayerAct::Checklist { id } => {
            if !item_open(w, id) || g.gesture(id).is_none() {
                return None;
            }
            Some(id.clone())
        }
        PlayerAct::Use { action, .. } => {
            let own = format!("equip.{}", action.id());
            if g.gesture(&own).is_some() {
                return Some(own);
            }
            let id = action.checklist()?;
            (item_open(w, id) && g.gesture(id).is_some()).then(|| id.to_string())
        }
        PlayerAct::PlayRecord { .. } => (item_open(w, "record_on")
            && g.gesture("record_on").is_some())
        .then(|| "record_on".to_string()),
        _ => None,
    }
}

/// The gesture of a key (`handson.ron` → `rituals.gestures`).
pub fn ritual_gesture<'a>(content: &'a Content, key: &str) -> Option<&'a RitualGesture> {
    content.handson.tuning.rituals.gesture(key)
}

/// `Command::Ritual`.
pub(crate) fn command(
    w: &mut World,
    content: &Content,
    key: String,
    step: RitualStep,
) -> Result<(), CommandError> {
    if w.phase != Phase::Night {
        return Err(CommandError::WrongPhase);
    }
    let Some(cur) = w.hands.player.current.as_ref() else {
        return Err(CommandError::Invalid("nothing in her hands".into()));
    };
    if ritual_key(w, content, &cur.act).as_deref() != Some(key.as_str()) {
        return Err(CommandError::Invalid("not this ritual".into()));
    }
    let r = &mut w.hands.rituals;
    match step {
        RitualStep::Start => {
            if r.hold.as_ref().is_none_or(|h| h.key != key) {
                r.hold = Some(RitualHold {
                    key,
                    ..Default::default()
                });
            }
        }
        RitualStep::Done { perfect } => {
            let waited = r
                .hold
                .as_ref()
                .filter(|h| h.key == key)
                .map_or(0, |h| h.waited);
            r.hold = Some(RitualHold {
                key,
                waited,
                done: true,
                perfect,
            });
        }
        RitualStep::LetGo => r.hold = None,
    }
    Ok(())
}

/// What her work at the piece does this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Gate {
    /// As always (no gesture in the player's hands).
    Normal,
    /// The player is doing the gesture: she waits for it.
    Wait,
    /// The gesture is done: the item completes now.
    Finish,
}

/// Called by her act at the piece (stage 1) before each work tick.
pub(crate) fn gate(w: &mut World, content: &Content) -> Gate {
    let Some(hold) = w.hands.rituals.hold.as_ref() else {
        return Gate::Normal;
    };
    let key = w
        .hands
        .player
        .current
        .as_ref()
        .and_then(|c| ritual_key(w, content, &c.act));
    if key.as_deref() != Some(hold.key.as_str()) {
        w.hands.rituals.hold = None;
        return Gate::Normal;
    }
    if hold.done {
        let item = w
            .hands
            .player
            .current
            .as_ref()
            .and_then(|c| item_of(&c.act))
            .map(str::to_string)
            .unwrap_or_else(|| hold.key.clone());
        let perfect = hold.perfect;
        w.hands.rituals.by_hand.insert(item, perfect);
        w.hands.rituals.hold = None;
        return Gate::Finish;
    }
    let patience = content.handson.tuning.rituals.patience_ticks;
    let Some(h) = w.hands.rituals.hold.as_mut() else {
        return Gate::Normal;
    };
    if h.waited >= patience {
        w.hands.rituals.hold = None;
        return Gate::Normal;
    }
    h.waited += 1;
    Gate::Wait
}

/// A gesture held for an act that is no longer hers (cancelled, done by a helper meanwhile)
/// lets go by itself.
pub(crate) fn tidy(w: &mut World, content: &Content) {
    let Some(hold) = w.hands.rituals.hold.as_ref() else {
        return;
    };
    let key = w
        .hands
        .player
        .current
        .as_ref()
        .and_then(|c| ritual_key(w, content, &c.act));
    if key.as_deref() != Some(hold.key.as_str()) {
        w.hands.rituals.hold = None;
    }
}

/// The needle dropped by hand on the record she put on: that is the opening's record item
/// too (the queued tick that would follow goes).
pub(crate) fn record_dropped(w: &mut World, content: &Content, me: u32) {
    if !w.hands.rituals.by_hand.contains_key("record_on") || !item_open(w, "record_on") {
        return;
    }
    let before = w.hands.player.queue.len();
    w.hands
        .player
        .queue
        .retain(|a| !matches!(a, PlayerAct::Checklist { id } if id == "record_on"));
    if w.hands.player.queue.len() != before {
        w.emit(SimEvent::PlayerQueueChanged);
    }
    super::app::tick_checklist(w, content, "record_on", Some(me));
}

/// Perfect gestures of Karolina's among a list's items tonight.
pub(crate) fn perfect_count(w: &World, content: &Content, list: ChecklistKind) -> u32 {
    content
        .app
        .checklists
        .iter()
        .filter(|c| c.list == list)
        .filter(|c| w.hands.rituals.by_hand.get(&c.id) == Some(&true))
        .count() as u32
}

/// The list's ritual is complete: its small `perfect` extra when enough gestures were perfect.
pub(crate) fn on_ritual_complete(
    w: &mut World,
    content: &Content,
    ritual: &crate::content::handson::Ritual,
) {
    let i = match ritual.list {
        ChecklistKind::Opening => 0,
        ChecklistKind::Closing => 1,
    };
    let min = content.handson.tuning.rituals.perfect_min.max(1);
    if ritual.perfect.is_empty()
        || w.hands.rituals.rewarded[i]
        || perfect_count(w, content, ritual.list) < min
    {
        return;
    }
    w.hands.rituals.rewarded[i] = true;
    let fx = ritual.perfect.clone();
    super::xlogic::run(w, content, &fx, &super::systems::XTarget::default());
    w.emit(SimEvent::RitualPerfect { list: ritual.list });
    w.emit(SimEvent::Toast {
        key: ritual.perfect_key(),
    });
}

/// A checklist item was ticked (after its `done` / `extra_done`): by hand it gets stronger,
/// adds to the evening mood or the crew's next night and counts towards a habit; a habit
/// learned acts whoever does it.
pub(crate) fn on_item_done(w: &mut World, content: &Content, id: &str) {
    let Some(def) = content.app.checklists.iter().find(|c| c.id == id) else {
        return;
    };
    let t = &content.handson.tuning.rituals;
    let fx_target = super::systems::XTarget::default();
    if let Some(perfect) = w.hands.rituals.by_hand.get(id).copied() {
        let fx = def.hand_done.clone();
        super::xlogic::run(w, content, &fx, &fx_target);
        if perfect {
            let fx = def.perfect_done.clone();
            super::xlogic::run(w, content, &fx, &fx_target);
        }
        match def.list {
            ChecklistKind::Opening => {
                if w.phase == Phase::Night && (w.in_prep() || w.minute() < t.mood.minutes) {
                    let until = Some(t.mood.minutes);
                    w.sys.fx.patience.push((t.mood.patience, until));
                    w.mods.push(Modifier {
                        kind: ModKind::Tips,
                        mul: t.mood.tips,
                        until,
                        event: None,
                    });
                    w.hands.rituals.mood_acting += 1;
                }
            }
            ChecklistKind::Closing => {
                w.hands.rituals.crew += t.crew_mood;
                w.hands.habits.crew_next += t.crew_mood;
            }
        }
        *w.hands.habits.nights.entry(id.to_string()).or_insert(0) += 1;
    }
    // one habit a night at most (a diligent player would learn all thirteen on night 5): a
    // waiting one comes the next time the item is done on a night without another
    let nights = w.hands.habits.nights.get(id).copied().unwrap_or(0);
    if nights >= t.habit_nights.max(1)
        && !def.habit.is_empty()
        && w.hands.rituals.learned.is_empty()
        && !w.hands.habits.learned.iter().any(|h| h == id)
    {
        w.hands.habits.learned.push(id.to_string());
        w.hands.rituals.learned.push(id.to_string());
        w.emit(SimEvent::HabitLearned {
            item: id.to_string(),
        });
    }
    if w.hands.habits.learned.iter().any(|h| h == id) {
        let fx = def.habit.clone();
        super::xlogic::run(w, content, &fx, &fx_target);
    }
}

/// Night start: last night's closing done by hand — the crew comes in in a better mood.
pub(crate) fn on_night_start(w: &mut World) {
    let add = std::mem::take(&mut w.hands.habits.crew_next);
    if add > 0.0 {
        for s in w.staff.iter_mut().filter(|s| !s.owner) {
            s.mood = (s.mood + add).min(100.0);
        }
    }
    w.hands.rituals.crew_from_last = add;
}

/// A tip was given: the evening mood's share of it.
pub(crate) fn note_tip(w: &mut World, content: &Content, tip: i64) {
    let m = content.handson.tuning.rituals.mood;
    if w.hands.rituals.mood_acting == 0 || tip <= 0 || w.minute() >= m.minutes {
        return;
    }
    let r = &mut w.hands.rituals;
    let mul = m.tips.powi(r.mood_acting as i32);
    if mul > 1.0 {
        r.mood_tips += tip as f32 * (1.0 - 1.0 / mul);
    }
}

fn list_counts(w: &World, content: &Content, list: ChecklistKind) -> (u32, u32) {
    let items = content.app.checklists.iter().filter(|c| c.list == list);
    let total = items.clone().count() as u32;
    let by_hand = items
        .filter(|c| w.hands.rituals.by_hand.contains_key(&c.id))
        .count() as u32;
    (by_hand, total)
}

/// The morning summary's lines: the opening and the closing done by hand and what they
/// gave, the habits learned tonight.
pub(crate) fn summary_lines(w: &World, content: &Content) -> Vec<super::purchases::PayoffLine> {
    use super::purchases::PayoffLine;
    let r = &w.hands.rituals;
    let mut out = Vec::new();
    let s = |v: u32| v.to_string();
    let (n, total) = list_counts(w, content, ChecklistKind::Opening);
    if n > 0 {
        let tips = r.mood_tips.round() as i64;
        let (key, mut args) = if tips > 0 {
            (
                "rituals.line.opening_tips",
                vec![("money".to_string(), tips.to_string())],
            )
        } else {
            ("rituals.line.opening", Vec::new())
        };
        args.push(("n".into(), s(n)));
        args.push(("total".into(), s(total)));
        out.push(PayoffLine {
            key: key.into(),
            args,
            money: tips,
        });
    }
    let (n, total) = list_counts(w, content, ChecklistKind::Closing);
    if n > 0 {
        out.push(PayoffLine {
            key: "rituals.line.closing".into(),
            args: vec![
                ("n".into(), s(n)),
                ("total".into(), s(total)),
                ("mood".into(), format!("{}", r.crew.round() as i64)),
            ],
            money: 0,
        });
    }
    for id in &r.learned {
        out.push(PayoffLine {
            key: "rituals.line.habit".into(),
            args: vec![("perk".into(), format!("@habit.{id}.perk"))],
            money: 0,
        });
    }
    out
}

impl World {
    /// The evening mood now (`rituals.mood`): its points, what they do, until when.
    pub fn ritual_mood(&self, content: &Content) -> MoodView {
        let m = content.handson.tuning.rituals.mood;
        let (points, max) = list_counts(self, content, ChecklistKind::Opening);
        let acting = self.hands.rituals.mood_acting as i32;
        MoodView {
            points,
            max,
            patience: m.patience.powi(acting),
            tips: m.tips.powi(acting),
            until: m.minutes,
            active: self.phase == Phase::Night && (self.in_prep() || self.minute() < m.minutes),
        }
    }
    /// Grandma's habits: nights by hand per item, the ones learned.
    pub fn habits(&self) -> &Habits {
        &self.hands.habits
    }
    /// Has this item become one of grandma's habits?
    pub fn habit_learned(&self, id: &str) -> bool {
        self.hands.habits.learned.iter().any(|h| h == id)
    }
    /// Crew mood last night's closing by hand put on tonight's crew.
    pub fn crew_mood_from_rituals(&self) -> f32 {
        self.hands.rituals.crew_from_last
    }
    /// Screenshot staging: `id` done by Karolina by hand now (re-opened first if it was done).
    pub fn debug_by_hand(&mut self, content: &Content, id: &str, perfect: bool) {
        let Some(c) = self.app.checklist.iter_mut().find(|c| c.id == id) else {
            return;
        };
        if c.done {
            c.done = false;
            self.tonight.checklist_done = self.tonight.checklist_done.saturating_sub(1);
        }
        self.hands.rituals.by_hand.insert(id.to_string(), perfect);
        let me = super::pacing::owner_id(self);
        super::app::tick_checklist(self, content, id, me);
    }
    /// Screenshot staging: tonight's tips from the evening mood.
    pub fn debug_mood_tips(&mut self, kc: f32) {
        self.hands.rituals.mood_tips = kc;
    }
    /// Screenshot staging: every item done by hand on `n` nights so far.
    pub fn debug_habit_nights(&mut self, content: &Content, n: u32) {
        for c in &content.app.checklists {
            self.hands.habits.nights.insert(c.id.clone(), n);
        }
    }
    /// The gesture key of Karolina's current act, if it has one now ([`ritual_key`]).
    pub fn ritual_key_now(&self, content: &Content) -> Option<String> {
        let cur = self.hands.player.current.as_ref()?;
        ritual_key(self, content, &cur.act)
    }
    /// The gesture in the player's hands (`Command::Ritual`), if any.
    pub fn ritual_hold(&self) -> Option<&RitualHold> {
        self.hands.rituals.hold.as_ref()
    }
    /// Did Karolina do this item by hand tonight (`Some(perfect)`)?
    pub fn ritual_by_hand(&self, id: &str) -> Option<bool> {
        self.hands.rituals.by_hand.get(id).copied()
    }
    /// The ritual reward of [opening, closing] given tonight.
    pub fn ritual_rewarded(&self) -> [bool; 2] {
        self.hands.rituals.rewarded
    }
}
