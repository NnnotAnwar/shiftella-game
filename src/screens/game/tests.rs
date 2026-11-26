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
    g.fast_forward(&c, &loc, 3, Some(90), true);
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

#[test]
fn story_reaches_an_ending_and_asks_for_credits() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, crate::i18n::Lang::En);
    let mut g = GameScreen::new_story(&c, "cozy", true);
    g.fast_forward(&c, &loc, 30, None, true);
    assert_eq!(g.world.night(), 30);
    g.start_night(&c);
    let mut exit = None;
    let mut guard = 0;
    while exit.is_none() && guard < 40_000 {
        if g.cutscene_playing() {
            g.finish_cutscene_now(&c, &loc);
        }
        // the morning summary is closed like the player would
        g.summary = None;
        g.advance(&c, TICK_SECONDS * 4.0, Some(&loc));
        exit = g.exit();
        guard += 1;
    }
    match exit {
        Some(GameExit::Credits { ending, .. }) => {
            let e = ending.expect("an ending id");
            assert!(e == "light" || e == "last_train", "{e}");
        }
        other => panic!("expected credits, got {other:?}"),
    }
    assert!(g.extra.ending_played);
}

/// Night 1 played end to end through the same intents as the mouse and keyboard
/// (the autoplayer): orders taken, mini-games played, cups served, tips collected, tables
/// cleared — and the morning comes.
#[test]
fn night1_hands_on_end_to_end() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, crate::i18n::Lang::En);
    let mut g = GameScreen::new_story(&c, "cozy", true);
    g.start_night(&c);
    g.set_speed(3);
    g.play_headless(&c, &loc, 3600.0, &|_| false);
    // the morning summary is waiting (or already shown)
    let s = g
        .summary
        .clone()
        .or_else(|| g.world.last_summary().cloned())
        .expect("night 1 ended");
    assert_eq!(s.night, 1);
    assert!(
        s.karolina_items >= 4,
        "Karolina served {}",
        s.karolina_items
    );
    assert!(s.minigames >= 1, "mini-games {}", s.minigames);
    assert!(g.world.tips_lifetime() > 0, "tips reached the jar");
    assert_eq!(s.guests_left_unhappy, 0, "Cozy: nobody leaves unhappy");
}

/// The order card opens on `PlayerOrderTaken`, Quick brews at once, By hand runs a
/// mini-game first; overlays slow the sim and clamp the speed.
#[test]
fn order_card_minigame_and_overlay_time() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, crate::i18n::Lang::En);
    let mut g = GameScreen::new_story(&c, "cozy", true);
    g.start_night(&c);
    g.set_speed(3);
    let (full, tick) = g.sim_rate(&c);
    assert_eq!(full, 3.0);
    assert!((tick - g.world.real_seconds_per_tick(&c)).abs() < 1e-6);
    g.play_headless(&c, &loc, 1200.0, &|g| g.hands.order.is_some());
    let o = g.hands.order.clone().expect("an order card opened");
    let (slow, _) = g.sim_rate(&c);
    assert!(
        slow <= g.world.overlay_scale(&c) + 1e-6,
        "overlay: ×1 at most, scaled"
    );
    let recipe = g.world.best_matches(&c, o.guest)[0].0.clone();
    g.brew_chosen(&c, o.guest, &recipe, crate::ui::handson::BrewMode::ByHand);
    assert!(
        g.hands.minigame.is_some(),
        "by hand plays the mini-game first"
    );
    assert!(g.hands.order.is_none());
    g.minigame_done(&c, 0.9);
    assert!(g.hands.minigame.is_none());
    let acts = hands::karolina_acts(&g.world);
    assert!(
        acts.iter()
            .any(|a| matches!(a, sim::PlayerAct::Brew { .. })),
        "she goes and brews: {acts:?}"
    );
}

/// Keyboard-only chores: E finds the nearest thing to do, Enter takes a rail ticket.
#[test]
fn keyboard_intents_reach_the_sim() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, crate::i18n::Lang::En);
    let mut g = GameScreen::new_story(&c, "cozy", true);
    g.start_night(&c);
    // the window light first (an opening checklist item)
    assert_eq!(g.next_chore(&c), Some(hands::Intent::Window));
    assert!(g.intent(&c, hands::Intent::Chore { front: false }));
    // a guest arrives: Enter on the rail takes the order (no autoplayer here)
    g.set_speed(3);
    let mut guard = 0;
    while guard < 40_000
        && !g
            .world
            .ticket_rail(&c)
            .iter()
            .any(|t| matches!(t.stage, sim::TicketStage::Order))
    {
        if g.cutscene_playing() {
            g.finish_cutscene_now(&c, &loc);
        }
        g.advance(&c, 0.05, Some(&loc));
        guard += 1;
    }
    let t = g.world.ticket_rail(&c)[0].clone();
    g.act_on_rail(&c);
    let claimed = g
        .world
        .parties()
        .iter()
        .find(|p| p.id == t.party)
        .is_some_and(|p| p.order_claim.is_some());
    assert!(claimed, "Enter on the rail sent Karolina to take the order");
}
