//! Read-only queries for "Pause on important events" (Settings → Gameplay, the sandbox setup's
//! box): the screens pause the night when one of these turns true. Nothing here changes the
//! world.

use super::guests;
use super::{Mode, PartyState, World};
use crate::content::Content;

/// A waiting party counts as "about to leave" below this share of its patience.
pub const PATIENCE_NEARLY_OUT: f32 = 0.15;

impl World {
    /// Classic only (gentle difficulties forgive, nobody storms out): parties waiting in the
    /// queue / at a table whose patience is nearly out (`PATIENCE_NEARLY_OUT` of their base
    /// patience, at least 2 game minutes) — they leave unhappy soon. Party ids, oldest first.
    pub fn parties_losing_patience(&self) -> Vec<u32> {
        if self.difficulty.gentle() {
            return Vec::new();
        }
        self.parties
            .iter()
            .filter(|p| {
                matches!(
                    p.state,
                    PartyState::Queue | PartyState::WaitOrder | PartyState::WaitFood
                ) && !p.angry
                    && p.patience > 0.0
                    && p.patience < (p.patience_max * PATIENCE_NEARLY_OUT).max(2.0)
            })
            .map(|p| p.id)
            .collect()
    }

    /// The café can't make anything on tonight's menu for lack of supplies (the cheapest
    /// unlocked item on the menu costs more than what's left). `false` with an empty menu.
    pub fn out_of_supplies(&self, content: &Content) -> bool {
        let cheapest = content
            .menu
            .iter()
            .filter(|m| guests::item_unlocked(self, m) && !self.econ.menu_off.contains(&m.id))
            .map(|m| m.cost as i64)
            .min();
        cheapest.is_some_and(|c| self.econ.supplies < c)
    }

    /// The sandbox setup asked to pause on important events (`town.options`, kept in the
    /// save). Always `false` in the story (it follows the global setting only).
    pub fn sandbox_pauses_on_events(&self) -> bool {
        self.mode == Mode::Sandbox
            && self
                .town
                .options
                .get("pause_on_events")
                .copied()
                .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::content::Content;

    const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

    fn night(c: &Content, diff: Difficulty, pause: bool) -> World {
        let mut s = SandboxSetup {
            seed: 11,
            difficulty: diff,
            ..Default::default()
        };
        s.options.insert("pause_on_events".into(), pause);
        let mut w = World::new_sandbox(c, s);
        w.start_night(c);
        let _ = w.apply(c, Command::OpenDoors);
        w
    }

    /// Ticks until a party waits inside (queue / table).
    fn waiting_party(w: &mut World, c: &Content) -> u32 {
        w.debug_spawn_guests(c, "students", 2, 1);
        for _ in 0..20_000 {
            w.tick(c);
            let _ = w.drain_events();
            if let Some(p) = w.parties.iter().find(|p| {
                matches!(
                    p.state,
                    PartyState::Queue | PartyState::WaitOrder | PartyState::WaitFood
                )
            }) {
                return p.id;
            }
        }
        panic!("no party sat down");
    }

    #[test]
    fn classic_guests_losing_patience_are_listed_gentle_never() {
        let c = Content::load(DATA).expect("content");
        let mut w = night(&c, Difficulty::Classic, false);
        let id = waiting_party(&mut w, &c);
        assert!(
            w.parties_losing_patience().is_empty(),
            "fresh guests are patient"
        );
        let p = w.parties.iter_mut().find(|p| p.id == id).unwrap();
        p.patience = 1.0;
        assert_eq!(w.parties_losing_patience(), vec![id]);

        let mut cozy = night(&c, Difficulty::Cozy, false);
        let id = waiting_party(&mut cozy, &c);
        cozy.parties
            .iter_mut()
            .find(|p| p.id == id)
            .unwrap()
            .patience = 1.0;
        assert!(
            cozy.parties_losing_patience().is_empty(),
            "Cozy forgives: nobody leaves unhappy"
        );
    }

    #[test]
    fn out_of_supplies_when_nothing_on_the_menu_can_be_made() {
        let c = Content::load(DATA).expect("content");
        let mut w = night(&c, Difficulty::Cozy, false);
        w.econ.supplies = 5_000;
        assert!(!w.out_of_supplies(&c));
        w.econ.supplies = 0;
        assert!(w.out_of_supplies(&c));
    }

    #[test]
    fn the_sandbox_option_survives_save_and_load() {
        let c = Content::load(DATA).expect("content");
        let w = night(&c, Difficulty::Cozy, true);
        assert!(w.sandbox_pauses_on_events());
        let mut back = World::load_from_str(&w.save_to_string().unwrap()).unwrap();
        back.rebuild_caches(&c);
        assert!(back.sandbox_pauses_on_events(), "kept in the save");
        assert!(!night(&c, Difficulty::Cozy, false).sandbox_pauses_on_events());
        let story = World::new_story(&c, Difficulty::Cozy);
        assert!(
            !story.sandbox_pauses_on_events(),
            "the story follows the setting only"
        );
    }
}
