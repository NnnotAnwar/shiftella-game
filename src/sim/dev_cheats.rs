//! Developer-build cheats on the world (`--features dev` only; the dev panel's "Cheats"
//! section, `crate::dev`). They poke the state directly, the way the `debug_*` helpers do —
//! nothing here is reachable from a normal build.

use crate::content::{Content, Weather};

use super::world::World;
use super::{guests, knowledge, orders, story};

impl World {
    /// Adds (or takes) money.
    pub fn dev_add_money(&mut self, delta: i64) {
        self.econ.money += delta;
    }

    /// Adds (or takes) reputation (0–100).
    pub fn dev_add_reputation(&mut self, delta: f32) {
        self.econ.reputation = (self.econ.reputation + delta).clamp(0.0, 100.0);
    }

    pub fn dev_set_weather(&mut self, weather: Weather) {
        self.weather = weather;
    }

    /// Hall dirt 0–1.
    pub fn dev_set_dirt(&mut self, dirt: f32) {
        self.dirt = dirt.clamp(0.0, 1.0);
    }

    /// Fills the fridge to its room (`full`) or empties the supplies.
    pub fn dev_fridge(&mut self, content: &Content, full: bool) {
        if !full {
            self.econ.supplies = 0;
            return;
        }
        let room = self.fridge(content).capacity.max(1) as f32;
        let share = super::fridge::fresh_of(10_000).max(1) as f32 / 10_000.0;
        self.econ.supplies = (room / share).ceil() as i64;
    }

    /// A regular (or a cameo guest) walks in now. False when the id is unknown, it is day or
    /// they are already inside.
    pub fn dev_spawn_regular(&mut self, content: &Content, id: &str) -> bool {
        if content.regular(id).is_none() || self.phase != super::Phase::Night {
            return false;
        }
        let before = self.parties.len();
        guests::spawn_regular(self, content, id, None);
        self.parties.len() > before
    }

    /// A regular's arc stage (`arc.<id>` counter), moved by `delta` (never below 0).
    pub fn dev_arc_stage(&mut self, id: &str, delta: i32) -> i32 {
        let key = format!("arc.{id}");
        let v = self.story.counters.get(&key).copied().unwrap_or(0) + delta;
        let v = v.max(0);
        self.story.counters.insert(key, v);
        v
    }

    /// A regular's trust, moved by `delta`.
    pub fn dev_trust(&mut self, id: &str, delta: f32) -> f32 {
        let t = self.story.trust.entry(id.to_string()).or_insert(0.0);
        *t += delta;
        *t
    }

    /// Sets (`on`) or clears a story flag.
    pub fn dev_flag(&mut self, flag: &str, on: bool) {
        if on {
            self.story.flags.insert(flag.to_string());
        } else {
            self.story.flags.remove(flag);
        }
    }

    pub fn dev_set_var(&mut self, name: &str, value: &str) {
        self.story.vars.insert(name.to_string(), value.to_string());
    }

    /// Finds every one of Lida's notes not found yet (with their effects). Returns how many.
    pub fn dev_find_notes(&mut self, content: &Content) -> usize {
        let ids: Vec<String> = content
            .story
            .notes
            .iter()
            .map(|n| n.id.clone())
            .filter(|id| !self.story.notes.contains(id))
            .collect();
        for id in &ids {
            story::find_note(self, content, id);
        }
        ids.len()
    }

    /// Every recipe learned and every ingredient in the pantry. Returns the recipes learned.
    pub fn dev_unlock_recipes(&mut self, content: &Content) -> usize {
        for i in &content.handson.recipes.ingredients {
            knowledge::unlock_ingredient(self, content, &i.id);
        }
        let before = self.hands.knowledge.recipes.len();
        for r in &content.handson.recipes.recipes {
            orders::learn(self, &r.id, "dev".into());
        }
        for m in &content.menu {
            self.story.unlocked_menu.insert(m.id.clone());
        }
        self.hands.knowledge.recipes.len() - before
    }

    /// Every piece of furniture may be bought.
    pub fn dev_unlock_furniture(&mut self, content: &Content) -> usize {
        let before = self.story.unlocked_furniture.len();
        for f in &content.furniture {
            self.story.unlocked_furniture.insert(f.id.clone());
        }
        self.story.unlocked_furniture.len() - before
    }

    /// Every decor theme unlocked and owned (switch freely by day).
    pub fn dev_unlock_themes(&mut self, content: &Content) -> usize {
        let before = self.sys.atmos.owned.len();
        for t in &content.handson.decor.themes {
            self.sys.atmos.unlocked.insert(t.id.clone());
            self.sys.atmos.owned.insert(t.id.clone());
        }
        self.sys.atmos.owned.len() - before
    }
}
