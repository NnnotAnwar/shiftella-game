//! Simulation — pure logic behind [`World`]: café grid & furniture, A*, staff & guests,
//! economy, the Night storyteller, the story director, the in-game Shiftella app, saves, undo.
//!
//! Commands in ([`Command`] via [`World::apply`]), events out ([`SimEvent`] via
//! [`World::drain_events`]). 10 ticks = 1 game minute; the night runs 22:00–06:00.
//! Deterministic: same seed + same commands → same result. No rendering, no macroquad.
//! API & integration guide: `docs/SIM.md`.

mod app;
mod commands;
mod economy;
pub mod grid;
mod guests;
mod logic;
mod narrator;
pub mod rng;
mod staff;
mod story;
mod types;
mod world;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_qa;

pub use app::digest_lines;
pub use grid::{Floor, Grid, Placed, Room, TILE, TileInfo, TileKind};
pub use story::{ambient_line_for, talk_available};
pub use types::*;
pub use world::*;

use crate::content::Content;

/// Trains: arrivals by timetable (if `announce_if` holds), passengers come in.
pub(crate) fn trains_tick(w: &mut World, content: &Content) {
    let m = w.minute();
    for t in &content.trains.trains {
        let at = t.arrives.minutes_since_open();
        if at < 0 || at as u32 != m {
            continue;
        }
        let mut ctx = EffectCtx::default();
        if !logic::all(w, content, &t.announce_if, &mut ctx) {
            continue;
        }
        w.emit(SimEvent::TrainPassing {
            train: t.id.clone(),
            station: t.from.clone(),
        });
        let n =
            (w.rng.range_u32(t.guests.0, t.guests.1) as f32 * guests::story_ramp(w)).round() as u32;
        if n > 0 {
            let start = w.rng.range_u32(3, 6);
            let mut left = n;
            let group = content.guest(&t.guest).map(|g| g.group).unwrap_or((1, 2));
            while left > 0 {
                let size = w.rng.range_u32(group.0.max(1), group.1.max(1)).min(left);
                let minute = m + start + w.rng.range_u32(0, 6);
                w.spawns.push(guests::PendingSpawn {
                    minute,
                    archetype: t.guest.clone(),
                    size,
                    patience_mul: 1.0,
                    spend_mul: 1.0,
                    orders: Vec::new(),
                });
                left -= size;
            }
        }
    }
}

impl World {
    /// Hover bubble line for a guest (regulars' `ambient_lines`).
    pub fn ambient_line_for(&self, content: &Content, guest: u32) -> Option<String> {
        story::ambient_line_for(self, content, guest)
    }
    /// Talk cutscene id available for this guest tonight (then send `Command::Talk`).
    pub fn talk_available(&self, content: &Content, guest: u32) -> Option<String> {
        story::talk_available(self, content, guest)
    }
}

impl World {
    /// Debug / stress helper: `n` guests of an archetype arrive within `within` minutes.
    pub fn debug_spawn_guests(&mut self, content: &Content, archetype: &str, n: u32, within: u32) {
        guests::schedule_rush(self, content, archetype, n, within, 1.0, 1.0, &[]);
    }
    /// Debug helper: set money (cheats / tests).
    pub fn debug_set_money(&mut self, money: i64) {
        self.econ.money = money;
    }
    /// Debug helper: fire an event now (ignores conditions).
    pub fn debug_fire_event(&mut self, content: &Content, id: &str) {
        narrator::fire(self, content, id, EffectCtx::default(), true);
    }
}
