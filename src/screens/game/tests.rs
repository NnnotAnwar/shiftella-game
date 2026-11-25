//! Headless smoke tests of the game flow (no window): new story → opening → Day 1 →
//! night → morning summary → Day 2 → save → load.

use std::rc::Rc;

use super::*;
use crate::content::Content;
use crate::save::{SaveStore, Slot};
use crate::settings::load_i18n;

fn content() -> Rc<Content> {
    let root = env!("CARGO_MANIFEST_DIR");
    Rc::new(Content::load(&format!("{root}/assets/data")).expect("content"))
}

fn i18n() -> crate::i18n::I18n {
    let root = env!("CARGO_MANIFEST_DIR");
    load_i18n(&format!("{root}/assets/i18n")).expect("i18n")
}

#[test]
fn story_flow_day_night_morning_save_load() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, crate::i18n::Lang::En);
    let mut g = GameScreen::new_story(&c, "cozy", false);
    // the opening plays before Day 1
    assert!(g.cutscene_playing());
    g.finish_cutscene_now(&c, &loc);
    assert!(!g.cutscene_playing());
    assert!(g.extra.opening_seen.is_some());
    assert_eq!(g.world.phase(), Phase::Day);
    assert_eq!(g.world.night(), 1);

    // night 1: the sim asks for the opening again — it is answered at once
    g.start_night(&c);
    assert_eq!(g.world.phase(), Phase::Night);
    let mut guard = 0;
    while g.summary.is_none() && guard < 20_000 {
        if g.cutscene_playing() {
            g.finish_cutscene_now(&c, &loc);
        }
        g.advance(&c, TICK_SECONDS * 3.0, Some(&loc));
        guard += 1;
    }
    let s = g.summary.clone().expect("morning summary after 06:00");
    assert_eq!(s.night, 1);
    assert_eq!(g.world.phase(), Phase::Day);
    assert_eq!(g.world.night(), 2);
    assert!(!g.ticker.is_empty(), "the night produced ticker lines");

    // save → load keeps the day
    let dir = std::env::temp_dir().join(format!("shiftella-flow-test-{}", crate::save::now()));
    let store = SaveStore::at(&dir);
    g.save(&c, &store, Slot::Manual(1)).unwrap();
    let f = store.read(Slot::Manual(1)).unwrap();
    assert_eq!(f.meta.night, 2);
    assert_eq!(f.meta.clock, "day");
    let g2 = GameScreen::from_save(&c, &f).unwrap();
    assert_eq!(g2.world.night(), 2);
    assert_eq!(g2.world.money(), g.world.money());
    assert!(g2.extra.opening_seen.is_some());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn speed_and_pause() {
    let c = content();
    let mut g = GameScreen::new_story(&c, "cozy", true);
    assert!(g.frozen(), "day phase is frozen");
    g.start_night(&c);
    g.set_speed(3);
    assert_eq!(g.speed, 3);
    g.toggle_pause();
    assert_eq!(g.speed, 0);
    assert!(g.frozen());
    g.toggle_pause();
    assert_eq!(g.speed, 3);
}

#[test]
fn fast_forward_reaches_night_and_minute() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, crate::i18n::Lang::En);
    let mut g = GameScreen::new_story(&c, "cozy", true);
    g.fast_forward(&c, &loc, 3, Some(90));
    assert_eq!(g.world.night(), 3);
    assert_eq!(g.world.phase(), Phase::Night);
    assert!(g.world.minute() >= 90);
}

#[test]
fn sandbox_starts_in_the_day() {
    let c = content();
    let g = GameScreen::new_sandbox(
        &c,
        sim::SandboxSetup {
            seed: 7,
            ..Default::default()
        },
    );
    assert_eq!(g.world.mode(), sim::Mode::Sandbox);
    assert_eq!(g.world.phase(), Phase::Day);
}
