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
    // Mirek's first scene plays as he walks in; the rail works once it's over
    if g.cutscene_playing() {
        g.finish_cutscene_now(&c, &loc);
    }
    let t = g.world.ticket_rail(&c)[0].clone();
    g.act_on_rail(&c);
    let claimed = g
        .world
        .parties()
        .iter()
        .find(|p| p.id == t.party)
        .is_some_and(|p| p.order_claim.is_some());
    assert!(claimed, "Enter on the rail sent Karolina to take the order: {t:?}");
}

// ───────────────────────── T8: the world's systems on screen ─────────────────────────

/// A moment's marker → its card → an option: Karolina walks over (her queue shows the walk)
/// and the moment resolves; the result line becomes a toast.
#[test]
fn moments_open_a_card_and_resolve_through_the_screen() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, crate::i18n::Lang::En);
    let mut g = GameScreen::new_story(&c, "cozy", true);
    g.start_night(&c);
    g.set_speed(3);
    g.play_headless(&c, &loc, 1500.0, &|g| {
        g.world
            .active_moments()
            .iter()
            .any(|m| m.resolving.is_none())
    });
    let m = g
        .world
        .active_moments()
        .iter()
        .find(|m| m.resolving.is_none())
        .cloned()
        .expect("a moment appeared on night 1");
    assert!(g.open_moment(&m.id));
    assert!(g.hands.overlay(), "a moment card slows the night");
    let opt = g.world.moment_options(&c, &m.id)[0].clone();
    assert!(g.moment_option(&c, &m.id, &opt));
    assert!(g.hands.moment.is_none(), "the card closes at once");
    let resolving = g
        .world
        .active_moments()
        .iter()
        .any(|x| x.id == m.id && x.resolving.is_some());
    assert!(resolving, "Karolina is on her way");
    let id = m.id.clone();
    g.play_headless(&c, &loc, 600.0, &move |g| {
        !g.world.active_moments().iter().any(|x| x.id == id)
    });
    assert!(
        g.world.systems().moments.resolved_ever.contains(&m.id),
        "{} resolved",
        m.id
    );
}

/// The morning shows the night's polaroids; closing the summary keeps the chosen one; the
/// journal's snapshots map onto the render's scene snaps.
#[test]
fn the_morning_polaroid_goes_into_the_journal() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, crate::i18n::Lang::En);
    let mut g = GameScreen::new_story(&c, "cozy", true);
    g.start_night(&c);
    g.set_speed(3);
    g.play_headless(&c, &loc, 3600.0, &|_| false);
    let (night, cands) = g
        .world
        .pending_polaroids()
        .expect("candidates in the morning");
    assert_eq!(night, 1);
    let snap = journal_ui::snap_of(&c, &cands[0].snap);
    assert_eq!(
        snap.tiles.len(),
        snap.size.0 as usize * snap.size.1 as usize
    );
    assert!(!journal_ui::caption(&loc, &cands[0]).contains("journal.caption"));
    g.photos.pick = cands.len().saturating_sub(1);
    journal_ui::keep_morning(&mut g, &c);
    assert!(g.world.pending_polaroids().is_none());
    assert_eq!(g.world.journal().len(), 1);
    assert_eq!(
        journal_ui::night_date(&c, &g.world, 1),
        g.world.journal()[0].date
    );
}

/// The book's pantry: Try sends the experiment; by day the result shows on the page.
#[test]
fn a_day_experiment_from_the_pantry() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, crate::i18n::Lang::En);
    let mut g = GameScreen::new_story(&c, "cozy", true);
    g.open_book(None);
    let jars: Vec<String> = g.world.pantry().iter().take(2).cloned().collect();
    assert!(
        g.command(&c, sim::Command::TryExperiment { ingredients: jars })
            .is_ok()
    );
    // the next frame handles the result (it carries text)
    g.drain_and_handle(&c, Some(&loc));
    let note = g
        .hands
        .book
        .as_ref()
        .and_then(|b| b.note.clone())
        .expect("the result is on the page");
    assert!(note.key.starts_with("experiment."), "{}", note.key);
}

/// The soups plate their own bowl; the singular guest name; deep links open the panels.
#[test]
fn soups_names_and_links() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, crate::i18n::Lang::En);
    let soup = c.handson.recipe("r_garlic_soup").unwrap();
    let (kind, v) = hands::playable(soup, 1);
    assert_eq!(kind, crate::minigame::MiniGameKind::Plate);
    assert_eq!(v, "garlic_soup");
    let (_, v) = hands::playable(c.handson.recipe("r_borscht").unwrap(), 1);
    assert_eq!(v, "borscht");
    assert_eq!(world_view::guest_one(&loc, "travelers"), "Traveller");
    let mut g = GameScreen::new_story(&c, "cozy", true);
    g.follow_link(crate::content::handson::AppLink::Journal);
    assert_eq!(g.panel, Some(Panel::Journal));
    g.follow_link(crate::content::handson::AppLink::Rozpis);
    assert_eq!((g.panel, g.ui_state.app_tab), (Some(Panel::App), 0));
    g.follow_link(crate::content::handson::AppLink::Pantry);
    assert_eq!(g.hands.book.as_ref().map(|b| b.state.tab), Some(3));
}

/// Three nights through the screen's autoplayer (its day planned by the player-bot): the
/// rituals, the handover, experiments, a polaroid every morning.
#[test]
fn three_autoplayed_nights_touch_the_systems() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, crate::i18n::Lang::En);
    let mut g = GameScreen::new_story(&c, "cozy", true);
    g.hands.autoplay = Some(hands::Autoplay {
        cooldown: 0.0,
        keyboard: false,
        skip_minigames: false,
    });
    for _ in 0..3 {
        let mut guard = 0;
        while g.world.phase() == Phase::Day && guard < 20 {
            if g.cutscene_playing() {
                g.finish_cutscene_now(&c, &loc);
            }
            g.autoplay_day(&c, 1.0, 0.0);
            guard += 1;
        }
        g.set_speed(3);
        g.play_headless(&c, &loc, 3600.0, &|_| false);
        assert_eq!(
            g.world.systems().tool.rituals,
            [true, true],
            "night {}: opening and closing; open items {:?}",
            g.world.night(),
            g.world.app().checklist.iter().filter(|c| !c.done).map(|c| c.id.clone()).collect::<Vec<_>>()
        );
    }
    g.autoplay_day(&c, 1.0, 0.0);
    assert_eq!(g.world.journal().len(), 3, "a polaroid a morning");
    assert!(
        !g.world.knowledge().tried.is_empty(),
        "experiments in the lulls"
    );
}

/// T10: a full story's journal (one polaroid per night) survives a save and every entry can
/// be re-drawn from the loaded data (the save keeps data, never images).
#[test]
fn thirty_polaroids_survive_a_save_and_rebuild_their_scenes() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, crate::i18n::Lang::En);
    let mut g = GameScreen::new_story(&c, "cozy", true);
    g.fast_forward(&c, &loc, 30, None, true);
    g.start_night(&c);
    let mut guard = 0;
    while g.world.ending().is_none() && guard < 40_000 {
        if g.cutscene_playing() {
            g.finish_cutscene_now(&c, &loc);
        }
        g.summary = None;
        g.advance(&c, TICK_SECONDS * 4.0, Some(&loc));
        guard += 1;
    }
    // the last morning's polaroid is kept when the ending closes the story
    let nights: Vec<u32> = g.world.journal().iter().map(|e| e.night).collect();
    let pending = g.world.pending_polaroids().map(|(n, _)| n);
    let mut all = nights.clone();
    all.extend(pending);
    all.sort();
    all.dedup();
    assert_eq!(all, (1..=30).collect::<Vec<_>>(), "one polaroid per night");
    assert_eq!(
        nights.len() + usize::from(pending.is_some()),
        30,
        "exactly one each"
    );
    let file = g.save_file(&c).unwrap();
    let back = GameScreen::from_save(&c, &file).unwrap();
    assert_eq!(back.world.journal(), g.world.journal());
    for e in back.world.journal() {
        let snap = journal_ui::snap_of(&c, &e.highlight.snap);
        assert!(!snap.tiles.is_empty(), "night {}: an empty photo", e.night);
        assert!(
            !e.highlight.caption_key.is_empty()
                && loc.tr(&e.highlight.caption_key) != e.highlight.caption_key,
            "night {}: caption {}",
            e.night,
            e.highlight.caption_key
        );
    }
}

/// T10: three manual slots and the autosave live side by side, Continue takes the newest,
/// and a save of a finished story loads straight to the credits.
#[test]
fn manual_slots_continue_and_a_finished_story() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, crate::i18n::Lang::En);
    let dir = std::env::temp_dir().join(format!(
        "shiftella-slots-test-{}-{}",
        std::process::id(),
        crate::save::now()
    ));
    let store = SaveStore::at(&dir);
    let mut g = GameScreen::new_story(&c, "cozy", true);
    for (k, slot) in [
        Slot::Auto,
        Slot::Manual(1),
        Slot::Manual(2),
        Slot::Manual(3),
    ]
    .into_iter()
    .enumerate()
    {
        g.fast_forward(&c, &loc, k as u32 + 2, None, true);
        let mut f = g.save_file(&c).unwrap();
        f.meta.saved_at = 1_000 + k as u64; // a clock tick apart
        store.write(slot, &f).unwrap();
    }
    let list = store.list();
    assert_eq!(list.len(), 4, "auto + three manual slots");
    let (latest, meta) = store.latest().unwrap();
    assert_eq!((latest, meta.night), (Slot::Manual(3), 5));
    let back = GameScreen::from_save(&c, &store.read(latest).unwrap()).unwrap();
    assert_eq!(back.world.night(), 5);
    // a finished story: the meta says so (the menu hides Continue) and it loads to the credits
    let mut done = GameScreen::new_story(&c, "cozy", true);
    done.fast_forward(&c, &loc, 30, None, true);
    done.start_night(&c);
    let mut guard = 0;
    while done.world.ending().is_none() && guard < 40_000 {
        if done.cutscene_playing() {
            done.finish_cutscene_now(&c, &loc);
        }
        done.summary = None;
        done.advance(&c, TICK_SECONDS * 4.0, Some(&loc));
        guard += 1;
    }
    let f = done.save_file(&c).unwrap();
    assert!(f.meta.finished);
    let mut again = GameScreen::from_save(&c, &f).unwrap();
    let mut exit = None;
    for _ in 0..200 {
        if again.cutscene_playing() {
            again.finish_cutscene_now(&c, &loc);
        }
        again.summary = None;
        again.advance(&c, 0.1, Some(&loc));
        exit = again.exit();
        if exit.is_some() {
            break;
        }
    }
    assert!(matches!(exit, Some(GameExit::Credits { .. })), "{exit:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Night 1's first coach marks (the window, the machine, the opening list) belong to the
/// preparation half hour: they can show from 21:30, well before Mirek at 22:04.
#[test]
fn night_one_tutorial_starts_in_the_preparation_half_hour() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, crate::i18n::Lang::En);
    let mut g = GameScreen::new_story(&c, "cozy", false);
    g.finish_cutscene_now(&c, &loc);
    g.start_tutorial();
    g.start_night(&c);
    while g.cutscene_playing() || g.world.playing_cutscene().is_some() {
        g.finish_cutscene_now(&c, &loc);
    }
    assert!(g.world.in_prep());
    assert_eq!(g.world.clock_label(), "21:30");
    assert!(super::coach::trigger(&g, &c, "light_window"));
    let mark = |g: &mut GameScreen, id: &str| {
        if let Some(p) = &mut g.extra.tutorial {
            p.coach_done.push(id.to_string());
        }
    };
    // she lights the window; the machine step follows, then the opening list
    g.command(
        &c,
        Command::TickChecklist {
            id: "light_window".into(),
        },
    )
    .unwrap();
    mark(&mut g, "light_window");
    assert!(super::coach::trigger(&g, &c, "machine"));
    mark(&mut g, "machine");
    assert!(super::coach::trigger(&g, &c, "checklist_open"));
    // a few real seconds later it's still before the doors open (and before 21:35)
    g.play_headless(&c, &loc, 4.0, &|_| false);
    assert!(g.world.in_prep());
    assert!(
        g.world.minutes_since_open() <= -25,
        "{}",
        g.world.clock_label()
    );
    // Mirek isn't there yet; he comes at 22:04 as before
    assert!(
        !g.world
            .parties()
            .iter()
            .any(|p| p.regular.as_deref() == Some("mirek"))
    );
}

/// The HUD bar spans 21:30 → 06:00 and the door chip works ("Open now" → 22:00).
#[test]
fn open_the_doors_from_the_hud_and_the_door_toast() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, crate::i18n::Lang::En);
    let mut g = GameScreen::new_story(&c, "cozy", true);
    g.start_night(&c);
    while g.cutscene_playing() || g.world.playing_cutscene().is_some() {
        g.finish_cutscene_now(&c, &loc);
    }
    assert!(g.world.in_prep());
    g.command(&c, Command::OpenDoors).unwrap();
    g.drain_and_handle(&c, Some(&loc));
    assert!(g.world.doors_open());
    assert_eq!(g.world.clock_label(), "22:00");
    assert!(g.scene.doors_opened_at.is_some(), "the door sign flips");
    assert!(
        g.ticker
            .iter()
            .any(|t| t.text == loc.tr("prep.toast.doors_open.title")),
        "ticker: doors open"
    );
    let p = g.world.evening_progress();
    let want = sim::PREP_MINUTES as f32 / (sim::PREP_MINUTES + sim::NIGHT_MINUTES) as f32;
    assert!((p - want).abs() < 1e-4, "{p}");
}

/// v0.2 appliances: after the window, **E** reaches the appliances; a click on one queues
/// its chore; the hold speeds a hold chore; a lamp click toggles it; the fridge with nothing
/// to do opens the pantry.
#[test]
fn appliances_by_click_and_by_e() {
    let c = content();
    let mut g = GameScreen::new_story(&c, "cozy", true);
    g.start_night(&c);
    assert!(g.world.in_prep());
    assert_eq!(g.next_chore(&c), Some(hands::Intent::Window));
    assert!(g.intent(&c, hands::Intent::Chore { front: false }));
    // then the nearest appliance with something on offer
    let Some(hands::Intent::Equip { uid, .. }) = g.next_chore(&c) else {
        panic!("E should reach an appliance: {:?}", g.next_chore(&c));
    };
    assert!(g.world.equipment_action(&c, uid).is_some());
    // a click on the machine: warm it up (a hold chore)
    let machine = g
        .world
        .furniture()
        .iter()
        .find(|p| g.world.equipment_kind(&c, p.uid) == Some(sim::EquipKind::Machine))
        .map(|p| p.uid)
        .unwrap();
    assert!(g.intent(
        &c,
        hands::Intent::Equip {
            uid: machine,
            front: true
        }
    ));
    assert!(hands::karolina_acts(&g.world).iter().any(|a| matches!(
        a,
        sim::PlayerAct::Use {
            action: sim::EquipAction::WarmUp,
            ..
        }
    )));
    // she walks there; holding (mouse / E) finishes it quickly
    let mut guard = 0;
    while !g.at_hold_chore() && guard < 2000 {
        g.world.tick(&c);
        guard += 1;
    }
    assert!(g.at_hold_chore(), "she reached the machine");
    for _ in 0..20 {
        g.equip_input(&c, true, 0.1);
        g.world.tick(&c);
    }
    g.equip_input(&c, false, 0.0);
    let ev = g.world.drain_events();
    assert!(ev.iter().any(|e| matches!(
        e,
        sim::SimEvent::EquipmentUsed {
            action: sim::EquipAction::WarmUp,
            ..
        }
    )));
    // a pendant lamp toggles
    let lamp = g
        .world
        .furniture()
        .iter()
        .find(|p| p.def == "pendant_lamp")
        .map(|p| (p.uid, p.on))
        .unwrap();
    assert!(g.intent(&c, hands::Intent::Lamp { uid: lamp.0 }));
    assert_eq!(g.world.furniture_by_uid(lamp.0).unwrap().on, !lamp.1);
    // the fridge: the milk first, afterwards the pantry
    let fridge = g
        .world
        .furniture()
        .iter()
        .find(|p| g.world.equipment_kind(&c, p.uid) == Some(sim::EquipKind::Fridge))
        .map(|p| p.uid)
        .unwrap();
    g.world.hands.equip.milk_done = true;
    for x in g.world.app.checklist.iter_mut() {
        if x.id == "check_stock" {
            x.done = true;
        }
    }
    assert_eq!(g.world.equipment_action(&c, fridge), None);
    assert!(g.intent(
        &c,
        hands::Intent::Equip {
            uid: fridge,
            front: false
        }
    ));
    assert!(g.hands.book.is_some(), "the pantry opened");
}
/// 0.1.1 bug: during the preparation half hour the night tick stands still, so the drawn
/// people froze on the spot while they walked and worked, then jumped at 22:00.
#[test]
fn people_are_drawn_walking_during_the_preparation_half_hour() {
    let c = content();
    let mut g = GameScreen::new_story(&c, "cozy", true);
    g.start_night(&c);
    assert!(g.world.in_prep());
    // Karolina goes to light the window
    assert!(g.intent(&c, hands::Intent::Window));
    let karolina = |g: &GameScreen| {
        g.scene
            .people
            .iter()
            .find(|v| matches!(v.agent, sim::AgentRef::Staff(_)))
            .map(|v| v.feet)
            .expect("Karolina is drawn")
    };
    g.scene.refresh(&g.world, 0.0, 0.0);
    let start = karolina(&g);
    let mut moved = 0.0f32;
    let mut last = start;
    for i in 0..90 {
        g.advance(&c, 1.0 / 30.0, None);
        g.scene.refresh(&g.world, i as f32 / 30.0, 0.5);
        let now = karolina(&g);
        let step = ((now.0 - last.0).powi(2) + (now.1 - last.1).powi(2)).sqrt();
        assert!(step < 1.5 * crate::map::TILE, "a jump of {step} px in one frame");
        moved += step;
        last = now;
    }
    assert!(g.world.in_prep(), "still the preparation");
    assert!(moved > 0.5 * crate::map::TILE, "drawn Karolina walked {moved} px");
}



/// 0.2.0 bug: Mirek walked in in his scene, then popped back to the street and walked in a
/// second time. After a scene, a regular it played goes on from where the scene left them.
#[test]
fn a_regular_goes_on_from_where_their_scene_left_them() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, crate::i18n::Lang::En);
    let mut g = GameScreen::new_story(&c, "cozy", true);
    g.start_night(&c);
    let mut guard = 0;
    while g.world.playing_cutscene() != Some("mirek_first") && guard < 200_000 {
        g.advance(&c, 1.0 / 30.0, Some(&loc));
        guard += 1;
    }
    assert_eq!(g.world.playing_cutscene(), Some("mirek_first"));
    // play the scene through like a player would (actors walk to their spots)
    let mut guard = 0;
    while g.cutscene_playing() && guard < 20_000 {
        g.player.fast_forward(&loc, 0.5);
        if let Some(end) = g.player.actors.iter().find(|a| a.id == "mirek").map(|a| a.pos) {
            let _ = end;
        }
        g.finish_cutscene_now(&c, &loc);
        guard += 1;
    }
    let mirek = g
        .world
        .parties()
        .iter()
        .find(|p| p.regular.as_deref() == Some("mirek"))
        .and_then(|p| p.leader())
        .and_then(|l| g.world.guest_views().into_iter().find(|v| v.agent == sim::AgentRef::Guest(l)))
        .expect("Mirek is in the café");
    let tile = (
        (mirek.feet.0 / crate::map::TILE).floor() as i32,
        (mirek.feet.1 / crate::map::TILE).floor() as i32,
    );
    let rows = g.world.grid().height;
    assert!(
        tile.1 < rows - 3,
        "Mirek stands inside where the scene left him, not back at the street: {tile:?}"
    );
}
