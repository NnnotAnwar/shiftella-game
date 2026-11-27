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

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::hands::PlayerAct;
use super::types::*;
use super::world::World;
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

impl World {
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
