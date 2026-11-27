//! Hands-on rituals (`sim::rituals`): a gesture the player starts holds Karolina at the piece
//! until it is done (or a while passes, or the player lets go), a finished one completes the
//! item at once, perfect gestures earn a list's small extra — and without the commands
//! nothing changes: the bot, helpers, ×2 and ignored gestures finish with the same ticks.

use super::*;
use crate::content::Content;
use crate::content::world::ChecklistKind;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

fn content() -> Content {
    Content::load(DATA).unwrap_or_else(|e| panic!("{e}"))
}

/// Night 1 at 21:30, the tutorial off, the clock waiting at 21:59.
fn evening(c: &Content) -> World {
    let mut w = World::new_story(c, Difficulty::Cozy);
    w.sys.tutorial_off = true;
    bot::finish_cutscenes(&mut w, c);
    w.start_night(c);
    bot::finish_cutscenes(&mut w, c);
    w.drain_events();
    w.auto_doors = false;
    w
}

fn done(w: &World, id: &str) -> bool {
    matches!(w.checklist_mark(id), ChecklistMark::Done { .. })
}

/// Ticks until `id` is done (or `max`), returns the ticks it took.
fn until_done(w: &mut World, c: &Content, id: &str, max: u32) -> Option<u32> {
    for t in 0..max {
        if done(w, id) {
            return Some(t);
        }
        w.tick(c);
        bot::finish_cutscenes(w, c);
        w.drain_events();
    }
    done(w, id).then_some(max)
}

fn at_piece(w: &mut World, c: &Content) {
    for _ in 0..600 {
        if w.player().current.as_ref().is_some_and(|a| a.stage >= 1) {
            return;
        }
        w.tick(c);
        w.drain_events();
    }
}

/// Queues `id` for her; a tick later it is her current act.
fn queue(w: &mut World, c: &Content, id: &str) {
    w.apply(c, Command::TickChecklist { id: id.into() })
        .unwrap();
    w.tick(c);
    w.drain_events();
}

fn ritual(w: &mut World, c: &Content, key: &str, step: RitualStep) -> Result<(), CommandError> {
    w.apply(
        c,
        Command::Ritual {
            key: key.into(),
            step,
        },
    )
}

#[test]
fn every_item_karolina_can_do_has_its_gesture_and_her_act_finds_it() {
    let c = content();
    let w = evening(&c);
    for item in &c.app.checklists {
        // every owner includes her (Karolina, or Karolina or a helper)
        assert!(
            ritual_gesture(&c, &item.id).is_some(),
            "{} has no gesture",
            item.id
        );
        let act = PlayerAct::Checklist {
            id: item.id.clone(),
        };
        if item.id != "record_on" || w.hands.music.source == MusicSource::House {
            assert_eq!(ritual_key(&w, &c, &act).as_deref(), Some(item.id.as_str()));
        }
    }
    // appliance chores: their own gesture, or the item they tick
    for a in EquipAction::ALL {
        let act = PlayerAct::Use { uid: 1, action: a };
        let own = format!("equip.{}", a.id());
        let key = ritual_key(&w, &c, &act);
        if ritual_gesture(&c, &own).is_some() {
            assert_eq!(key.as_deref(), Some(own.as_str()));
        } else if let Some(id) = a.checklist() {
            assert_eq!(key.as_deref(), Some(id), "{a:?}");
        }
    }
    for k in c.handson.tuning.rituals.gestures.keys() {
        if let Some(a) = k.strip_prefix("equip.") {
            assert!(
                EquipAction::ALL.iter().any(|x| x.id() == a),
                "unknown chore `{k}`"
            );
        }
    }
    // a record put on for the opening's record item drops the needle
    let rec = PlayerAct::PlayRecord {
        record: None,
        repeat: true,
    };
    assert_eq!(ritual_key(&w, &c, &rec).as_deref(), Some("record_on"));
}

#[test]
fn without_a_gesture_she_finishes_with_the_same_ticks_as_before() {
    let c = content();
    let mut a = evening(&c);
    let mut b = evening(&c);
    for id in ["light_window", "count_float", "check_stock", "record_on"] {
        a.apply(&c, Command::TickChecklist { id: id.into() })
            .unwrap();
        b.apply(&c, Command::TickChecklist { id: id.into() })
            .unwrap();
    }
    // b: a gesture that was offered but never touched sends nothing — the same world
    for _ in 0..3000 {
        a.tick(&c);
        b.tick(&c);
        a.drain_events();
        b.drain_events();
    }
    assert_eq!(a.save_to_string().ok(), b.save_to_string().ok());
    for id in ["light_window", "count_float", "check_stock", "record_on"] {
        assert!(done(&a, id), "{id}");
        assert_eq!(a.ritual_by_hand(id), None);
    }
    assert!(a.ritual_hold().is_none());
}

#[test]
fn a_started_gesture_holds_her_at_the_piece_until_it_is_done() {
    let c = content();
    let mut w = evening(&c);
    let mut base = evening(&c);
    for g in [&mut w, &mut base] {
        queue(g, &c, "count_float");
    }
    let normal = until_done(&mut base, &c, "count_float", 2000).unwrap();
    assert_eq!(w.ritual_key_now(&c).as_deref(), Some("count_float"));
    ritual(&mut w, &c, "count_float", RitualStep::Start).unwrap();
    // she walks there and waits: well past her own few ticks it is still open
    assert!(until_done(&mut w, &c, "count_float", normal + 40).is_none());
    assert!(w.ritual_hold().is_some_and(|h| h.waited > 0));
    ritual(
        &mut w,
        &c,
        "count_float",
        RitualStep::Done { perfect: true },
    )
    .unwrap();
    assert_eq!(until_done(&mut w, &c, "count_float", 3), Some(1), "at once");
    assert_eq!(w.ritual_by_hand("count_float"), Some(true));
    assert!(w.ritual_hold().is_none());
}

#[test]
fn a_gesture_done_on_her_way_finishes_the_item_as_she_arrives() {
    let c = content();
    let mut w = evening(&c);
    let mut base = evening(&c);
    for g in [&mut w, &mut base] {
        queue(g, &c, "light_window");
    }
    let normal = until_done(&mut base, &c, "light_window", 2000).unwrap();
    ritual(&mut w, &c, "light_window", RitualStep::Start).unwrap();
    ritual(
        &mut w,
        &c,
        "light_window",
        RitualStep::Done { perfect: false },
    )
    .unwrap();
    let quick = until_done(&mut w, &c, "light_window", 2000).unwrap();
    assert!(quick < normal, "a little faster: {quick} < {normal}");
    assert_eq!(w.ritual_by_hand("light_window"), Some(false));
    assert!(w.window_light_on(&c));
}

#[test]
fn letting_go_or_a_forgotten_gesture_never_holds_her() {
    let c = content();
    // Esc: she finishes it herself
    let mut w = evening(&c);
    queue(&mut w, &c, "check_stock");
    at_piece(&mut w, &c);
    ritual(&mut w, &c, "check_stock", RitualStep::Start).unwrap();
    for _ in 0..30 {
        w.tick(&c);
    }
    assert!(!done(&w, "check_stock"));
    ritual(&mut w, &c, "check_stock", RitualStep::LetGo).unwrap();
    assert!(until_done(&mut w, &c, "check_stock", 60).is_some());
    assert_eq!(w.ritual_by_hand("check_stock"), None);
    // forgotten: after `patience_ticks` of waiting she carries on
    let mut w = evening(&c);
    queue(&mut w, &c, "check_stock");
    at_piece(&mut w, &c);
    ritual(&mut w, &c, "check_stock", RitualStep::Start).unwrap();
    let patience = c.handson.tuning.rituals.patience_ticks;
    let t = until_done(&mut w, &c, "check_stock", patience + 60).expect("not held for good");
    assert!(t >= patience, "{t}");
    assert!(w.ritual_hold().is_none());
}

#[test]
fn a_ritual_command_must_match_what_she_is_doing() {
    let c = content();
    let mut w = evening(&c);
    assert!(matches!(
        ritual(&mut w, &c, "light_window", RitualStep::Start),
        Err(CommandError::Invalid(_))
    ));
    queue(&mut w, &c, "light_window");
    assert!(matches!(
        ritual(&mut w, &c, "count_float", RitualStep::Start),
        Err(CommandError::Invalid(_))
    ));
    assert!(ritual(&mut w, &c, "light_window", RitualStep::Start).is_ok());
    // cancelled: the held gesture goes with the act
    w.apply(&c, Command::PlayerCancel).unwrap();
    w.tick(&c);
    assert!(w.ritual_hold().is_none());
    let mut day = World::new_story(&c, Difficulty::Cozy);
    assert_eq!(
        ritual(&mut day, &c, "light_window", RitualStep::Start),
        Err(CommandError::WrongPhase)
    );
}

#[test]
fn the_needle_dropped_by_hand_is_the_record_item_too() {
    let c = content();
    let mut w = evening(&c);
    let rec = w.hands.music.unlocked.iter().next().cloned();
    w.apply(
        &c,
        Command::PlayRecord {
            id: rec,
            repeat: true,
        },
    )
    .unwrap();
    queue(&mut w, &c, "record_on");
    assert_eq!(w.ritual_key_now(&c).as_deref(), Some("record_on"));
    ritual(&mut w, &c, "record_on", RitualStep::Done { perfect: true }).unwrap();
    assert!(until_done(&mut w, &c, "record_on", 2000).is_some());
    assert_eq!(w.ritual_by_hand("record_on"), Some(true));
    assert!(
        super::music::current_record(&w).is_some(),
        "the record plays"
    );
    assert!(
        !w.player()
            .queue
            .iter()
            .any(|a| matches!(a, PlayerAct::Checklist { id } if id == "record_on")),
        "no second trip to the jukebox"
    );
}

/// The opening done with three perfect gestures: a perfect opening (+1 coziness tonight);
/// the same list done without them: none.
#[test]
fn a_perfect_opening_gives_its_small_extra_once() {
    let c = content();
    let mut runs = Vec::new();
    for by_hand in [true, false] {
        let mut w = evening(&c);
        let ids: Vec<String> = c
            .app
            .checklists
            .iter()
            .filter(|d| d.list == ChecklistKind::Opening)
            .map(|d| d.id.clone())
            .collect();
        let mut events = Vec::new();
        for (n, id) in ids.iter().enumerate() {
            if done(&w, id) {
                continue;
            }
            if id == "menu_board" {
                let s = w.specials().to_vec();
                w.apply(&c, Command::SetSpecials { recipes: s }).unwrap();
                continue;
            }
            if w.apply(&c, Command::TickChecklist { id: id.clone() })
                .is_err()
            {
                continue;
            }
            w.tick(&c);
            events.extend(w.drain_events());
            if by_hand && n < 5 && w.ritual_key_now(&c).as_deref() == Some(id.as_str()) {
                ritual(&mut w, &c, id, RitualStep::Done { perfect: true }).unwrap();
            }
            for _ in 0..2000 {
                if done(&w, id) {
                    break;
                }
                w.tick(&c);
                bot::finish_cutscenes(&mut w, &c);
                events.extend(w.drain_events());
            }
        }
        events.extend(w.drain_events());
        assert!(super::tool::list_done(&w, &c, ChecklistKind::Opening));
        let perfect = events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    SimEvent::RitualPerfect {
                        list: ChecklistKind::Opening
                    }
                )
            })
            .count();
        runs.push((perfect, w.coziness().value, w.ritual_rewarded()));
    }
    assert_eq!(runs[0].0, 1, "by hand: a perfect opening");
    assert_eq!(runs[1].0, 0, "by herself: none");
    assert!(runs[0].2[0] && !runs[1].2[0]);
}

#[test]
fn a_held_gesture_survives_a_save_and_old_saves_load_without_one() {
    let c = content();
    let mut w = evening(&c);
    queue(&mut w, &c, "count_float");
    ritual(&mut w, &c, "count_float", RitualStep::Start).unwrap();
    let s = w.save_to_string().unwrap();
    let mut back = World::load_from_str(&s).unwrap();
    back.rebuild_caches(&c);
    assert_eq!(back.ritual_hold(), w.ritual_hold());
    // a version 1 save (no rituals at all) still loads
    let v1 = include_str!("testdata/save_v1_story_day3.ron");
    let old = World::load_from_str(v1).unwrap();
    assert!(old.ritual_hold().is_none());
}

/// The player-bot never touches a gesture: the story still ends the same way on Cozy and
/// Classic (seed 4242), and nothing of the rituals' state ever fills up.
#[test]
fn the_bot_plays_the_story_as_before() {
    let c = content();
    for diff in [Difficulty::Cozy, Difficulty::Classic] {
        let mut w = World::new_story_seeded(&c, diff, 4242);
        let mut b = bot::PlayerBot::new(bot::BotConfig {
            seed: 4242,
            ..Default::default()
        });
        for _ in 0..30 {
            if w.is_closed() || w.ending().is_some() {
                break;
            }
            let _ = b.run_night(&mut w, &c);
            assert!(w.ritual_hold().is_none());
            assert!(w.hands.rituals.by_hand.is_empty());
        }
        let _ = b.run_night(&mut w, &c);
        assert!(w.ending().is_some(), "{diff:?}: an ending");
        eprintln!("{diff:?}: ending {:?}, debt {}", w.ending(), w.econ.debt);
    }
}
