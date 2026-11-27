//! The fridge keeps the stock (purchases that matter, `assets/data/purchases.ron` → `fridge`).
//!
//! The supplies are one pool in Kč (`Economy::supplies`); a fixed share of it is *fresh* —
//! milk, cream, the cake, the chlebíček's fillings — and lives in the fridge. Each piece with
//! the fridge tag holds `per_piece` Kč of it, less when it is worn (`wear_loss` × wear) and
//! half for the morning after it failed (`fridge_fail`). At 06:00 the fresh stock beyond what
//! the fridges hold partly goes off (`spoil`); the closing fridge check (by hand or a helper)
//! makes that a little less (`check_mul`). A second fridge raises the room, so a café that
//! keeps more stock wastes less; the morning summary says what it saved or what went off.
//!
//! Grandma's café is the neutral point: with her one fridge in good repair the usual stock
//! (the auto-restock target) fits, so a player who never builds loses nothing. Pure, no RNG.

use serde::{Deserialize, Serialize};

use super::world::*;
use crate::content::{Content, FurnitureDef};

/// Tuning (`purchases.ron` → `fridge`); every field defaulted.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct FridgeDef {
    /// The furniture tag of a fridge.
    pub tag: String,
    /// The share of the supplies that is fresh (needs the fridge).
    pub fresh_share: f32,
    /// Kč of fresh stock one fridge in good repair holds.
    pub per_piece: f32,
    /// Capacity × (1 − `wear_loss` × wear), at least `min_condition`.
    pub wear_loss: f32,
    pub min_condition: f32,
    /// The morning after `fail_event`, the most worn fridge holds × this.
    pub failed_mul: f32,
    pub fail_event: String,
    /// The share of the fresh stock beyond the room that goes off overnight.
    pub spoil: f32,
    /// Spoilage × this when the closing `check_item` was done.
    pub check_mul: f32,
    pub check_item: String,
    /// From this wear a fridge counts as worn (the morning line blames it, the planning card
    /// says so): the appliances' `service_wear`, when it can be defrosted.
    pub worn_at: f32,
    /// Kč of fresh stock per litre in the morning line ("3 l of milk went off").
    pub litre: f32,
}

impl Default for FridgeDef {
    fn default() -> Self {
        Self {
            tag: "station:fridge".into(),
            fresh_share: 0.4,
            per_piece: 1100.0,
            wear_loss: 0.8,
            min_condition: 0.3,
            failed_mul: 0.5,
            fail_event: "fridge_fail".into(),
            spoil: 0.5,
            check_mul: 0.6,
            check_item: "fridge_check".into(),
            worn_at: 0.5,
            litre: 25.0,
        }
    }
}

fn def() -> &'static FridgeDef {
    &super::purchases::data().fridge
}

/// Why fresh stock went off.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpoilCause {
    /// More fresh stock than the fridges hold.
    #[default]
    Full,
    /// A worn fridge holds less.
    Worn,
    /// The fridge failed tonight.
    Failed,
}

/// Last morning's fridge (in `PayoffTally`, reset at night start).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FridgeNight {
    /// Fresh stock at 06:00, the room for it (Kč), the fridges counted.
    pub fresh: i64,
    pub capacity: i64,
    pub pieces: u32,
    /// Kč of fresh stock that went off.
    pub spoiled: i64,
    /// Kč the fridges beyond the first kept from going off.
    pub saved: i64,
    /// The closing fridge check was done.
    pub checked: bool,
    pub cause: SpoilCause,
}

/// The fridge as the screens show it.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FridgeView {
    /// Fresh stock now and the room for it (Kč).
    pub fresh: i64,
    pub capacity: i64,
    pub pieces: u32,
    /// The most worn fridge's wear (0 = good repair).
    pub wear: f32,
}

impl FridgeView {
    pub fn worn(&self) -> bool {
        self.pieces > 0 && self.wear >= def().worn_at
    }
    pub fn over(&self) -> bool {
        self.fresh > self.capacity
    }
}

/// A fridge's condition at `wear` (1 = good repair).
pub fn condition(wear: f32) -> f32 {
    let d = def();
    (1.0 - d.wear_loss * wear).clamp(d.min_condition.min(1.0), 1.0)
}

fn is_fridge(content: &Content, def_id: &str) -> bool {
    content
        .furniture_def(def_id)
        .is_some_and(|d| d.has_tag(&def().tag))
}

/// Each fridge's (room in Kč, wear) (placement order; the most worn one halved after a
/// failure).
fn rooms(w: &World, content: &Content, failed: bool) -> Vec<(f32, f32)> {
    let d = def();
    let mut v: Vec<(f32, f32)> = w
        .furniture
        .iter()
        .filter(|p| is_fridge(content, &p.def))
        .map(|p| (d.per_piece * condition(p.wear), p.wear))
        .collect();
    if failed && let Some(worst) = (0..v.len()).max_by(|a, b| v[*a].1.total_cmp(&v[*b].1)) {
        v[worst].0 *= d.failed_mul;
    }
    v
}

/// Fresh stock in a supplies pool of `supplies` Kč.
pub fn fresh_of(supplies: i64) -> i64 {
    (supplies.max(0) as f32 * def().fresh_share).round() as i64
}

/// What goes off of `fresh` Kč with `room` Kč of fridge (the check done or not).
pub fn spoil_of(fresh: i64, room: i64, checked: bool) -> i64 {
    let d = def();
    let over = (fresh - room).max(0) as f32;
    let k = if checked { d.check_mul } else { 1.0 };
    (over * d.spoil * k).round() as i64
}

fn failed_tonight(w: &World) -> bool {
    w.tonight.events.iter().any(|e| *e == def().fail_event)
}

fn checked_tonight(w: &World) -> bool {
    let id = &def().check_item;
    w.app.checklist.iter().any(|c| &c.id == id && c.done)
}

/// 06:00 (before the morning's lines): the fresh stock beyond the fridges' room partly goes
/// off. Nothing random.
pub(crate) fn on_night_end(w: &mut World, content: &Content) {
    let failed = failed_tonight(w);
    let checked = checked_tonight(w);
    let rooms = rooms(w, content, failed);
    let capacity = rooms.iter().map(|r| r.0).sum::<f32>().round() as i64;
    let first = rooms.first().map_or(0, |r| r.0.round() as i64);
    let fresh = fresh_of(w.econ.supplies);
    let spoiled = spoil_of(fresh, capacity, checked).min(w.econ.supplies.max(0));
    let saved = if rooms.len() > 1 {
        (spoil_of(fresh, first, checked) - spoiled).max(0)
    } else {
        0
    };
    let worst = rooms.iter().map(|r| r.1).fold(0.0f32, f32::max);
    let cause = if failed {
        SpoilCause::Failed
    } else if worst >= def().worn_at {
        SpoilCause::Worn
    } else {
        SpoilCause::Full
    };
    w.econ.supplies -= spoiled;
    w.pay.tonight.fridge = FridgeNight {
        fresh,
        capacity,
        pieces: rooms.len() as u32,
        spoiled,
        saved,
        checked,
        cause,
    };
}

fn arg(k: &str, v: impl ToString) -> (String, String) {
    (k.to_string(), v.to_string())
}

/// The morning's fridge lines: what a second fridge saved, what went off. (sort weight, line)
pub(crate) fn lines(w: &World) -> Vec<(f32, super::purchases::PayoffLine)> {
    use super::purchases::PayoffLine;
    let f = &w.pay.tonight.fridge;
    let mut out = Vec::new();
    if f.saved > 0 {
        let key = if f.spoiled == 0 {
            "purchases.line.fridge_saved"
        } else {
            "purchases.line.fridge_saved_part"
        };
        out.push((
            f.saved as f32,
            PayoffLine {
                key: key.into(),
                args: vec![arg("money", f.saved)],
                money: f.saved,
            },
        ));
    }
    if f.spoiled > 0 {
        let key = match f.cause {
            SpoilCause::Full => "purchases.line.spoiled_full",
            SpoilCause::Worn => "purchases.line.spoiled_worn",
            SpoilCause::Failed => "purchases.line.spoiled_failed",
        };
        let litres = (f.spoiled as f32 / def().litre.max(1.0)).round().max(1.0) as i64;
        out.push((
            f.spoiled as f32,
            PayoffLine {
                key: key.into(),
                args: vec![arg("l", litres), arg("money", -f.spoiled)],
                money: -f.spoiled,
            },
        ));
    }
    out
}

impl World {
    /// The fridge now: fresh stock, the room for it, how worn (the planning card, the pantry).
    pub fn fridge(&self, content: &Content) -> FridgeView {
        let rooms = rooms(self, content, false);
        FridgeView {
            fresh: fresh_of(self.econ.supplies),
            capacity: rooms.iter().map(|r| r.0).sum::<f32>().round() as i64,
            pieces: rooms.len() as u32,
            wear: rooms.iter().map(|r| r.1).fold(0.0f32, f32::max),
        }
    }

    /// Last morning's fridge (what went off, what a second fridge saved).
    pub fn fridge_last_night(&self) -> &FridgeNight {
        &self.pay.tonight.fridge
    }
}

/// The build card: the room now → with `def` (Kč of fresh stock), when it is a fridge.
pub(crate) fn card_room(w: &World, content: &Content, d: &FurnitureDef) -> Option<(i64, i64)> {
    if !d.has_tag(&def().tag) {
        return None;
    }
    let now = w.fridge(content).capacity;
    Some((now, now + def().per_piece.round() as i64))
}
