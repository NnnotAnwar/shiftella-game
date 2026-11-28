//! What doing a ritual by hand gives (0.2.6, `sim::rituals`): the item's own effect a little
//! stronger (a perfect gesture a bit more), the evening mood in the first hour, the crew's
//! mood the next night, and grandma's habits for good — while an item she does herself works
//! exactly as before (`tests_rituals` checks the same ticks and the same world).

use super::*;
use crate::content::Content;
use crate::content::world::ChecklistKind;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

fn content() -> Content {
    Content::load(DATA).unwrap_or_else(|e| panic!("{e}"))
}

/// Night 1 at 21:30, the tutorial off.
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

/// Queues `id` for Karolina and finishes it: `gesture` = by hand (`Some(perfect)`) or
/// herself (`None`). Returns the events on the way.
fn do_item(w: &mut World, c: &Content, id: &str, gesture: Option<bool>) -> Vec<SimEvent> {
    let mut events = Vec::new();
    w.apply(c, Command::TickChecklist { id: id.into() })
        .unwrap();
    w.tick(c);
    events.extend(w.drain_events());
    if let Some(perfect) = gesture {
        assert_eq!(w.ritual_key_now(c).as_deref(), Some(id), "her act is {id}");
        w.apply(
            c,
            Command::Ritual {
                key: id.into(),
                step: RitualStep::Done { perfect },
            },
        )
        .unwrap();
    }
    for _ in 0..3000 {
        if done(w, id) {
            break;
        }
        w.tick(c);
        bot::finish_cutscenes(w, c);
        events.extend(w.drain_events());
    }
    assert!(done(w, id), "{id} done");
    events.extend(w.drain_events());
    events
}

fn tips_mods(w: &World) -> Vec<(f32, Option<u32>)> {
    w.mods
        .iter()
        .filter(|m| m.kind == ModKind::Tips)
        .map(|m| (m.mul, m.until))
        .collect()
}

#[test]
fn by_hand_an_item_is_stronger_and_a_perfect_gesture_a_little_more() {
    let c = content();
    let mut rep = Vec::new();
    for gesture in [None, Some(false), Some(true)] {
        let mut w = evening(&c);
        let before = w.econ.reputation;
        do_item(&mut w, &c, "light_window", gesture);
        let rate: Vec<f32> = w
            .mods
            .iter()
            .filter(|m| m.kind == ModKind::GuestRate)
            .map(|m| m.mul)
            .collect();
        assert_eq!(
            rate.len(),
            gesture.map_or(0, |_| 1),
            "{gesture:?}: {rate:?}"
        );
        rep.push(w.econ.reputation - before);
    }
    assert!(rep[0].abs() < 1e-4, "herself: as before ({})", rep[0]);
    assert!((rep[1] - 0.5).abs() < 1e-4, "by hand: +0.5 ({})", rep[1]);
    assert!((rep[2] - 0.75).abs() < 1e-4, "perfect: +0.75 ({})", rep[2]);
}

#[test]
fn the_warm_up_and_the_float_by_hand_last_longer() {
    let c = content();
    let bar = Some(Job::Bar.index() as u8);
    let till = Some(Job::Register.index() as u8);
    let speed = |w: &World, job: Option<u8>| -> Vec<(f32, Option<u32>)> {
        w.mods
            .iter()
            .filter(|m| m.kind == ModKind::WorkSpeed(job))
            .map(|m| (m.mul, m.until))
            .collect()
    };
    let mut herself = evening(&c);
    do_item(&mut herself, &c, "coffee_warmup", None);
    do_item(&mut herself, &c, "count_float", None);
    assert_eq!(speed(&herself, bar).len(), 1, "the usual ×1.25");
    assert_eq!(speed(&herself, till).len(), 1, "the usual ×1.2");
    assert!(herself.sys.fx.later.is_empty());
    let mut hand = evening(&c);
    do_item(&mut hand, &c, "coffee_warmup", Some(false));
    do_item(&mut hand, &c, "count_float", Some(false));
    let b = speed(&hand, bar);
    assert_eq!(b.len(), 2, "{b:?}");
    let total: f32 = b.iter().map(|x| x.0).product();
    assert!((total - 1.35).abs() < 0.01, "bar ×1.35 first: {total}");
    // the usual hour, then ×1.25 more until 90 minutes; the till's 2 h, then all night
    let later: Vec<u32> = hand.sys.fx.later.iter().map(|l| l.due).collect();
    assert_eq!(later.len(), 2, "{later:?}");
    assert!(
        hand.sys
            .fx
            .quick_quality
            .iter()
            .any(|q| (q.0 - 0.03).abs() < 1e-4)
    );
}

#[test]
fn herself_nothing_of_it_happens() {
    let c = content();
    let mut w = evening(&c);
    for id in ["light_window", "check_stock", "count_float", "record_on"] {
        do_item(&mut w, &c, id, None);
    }
    let m = w.ritual_mood(&c);
    assert_eq!(m.points, 0);
    assert_eq!(m.patience, 1.0);
    assert!(tips_mods(&w).is_empty());
    assert!(!w.sys.fx.stock_watch);
    assert!(w.habits().nights.is_empty() && w.habits().learned.is_empty());
    assert!(super::rituals::summary_lines(&w, &c).is_empty());
}

#[test]
fn the_evening_mood_counts_the_opening_by_hand_and_acts_in_the_first_hour() {
    let c = content();
    let mut w = evening(&c);
    let max = c
        .app
        .checklists
        .iter()
        .filter(|i| i.list == ChecklistKind::Opening)
        .count() as u32;
    assert_eq!(w.ritual_mood(&c).max, max);
    do_item(&mut w, &c, "light_window", Some(true));
    do_item(&mut w, &c, "count_float", Some(false));
    do_item(&mut w, &c, "check_stock", None);
    let m = w.ritual_mood(&c);
    assert_eq!(m.points, 2, "two by hand (perfect or not), one herself");
    assert!(m.active);
    assert!(
        (m.patience - 1.04f32.powi(2)).abs() < 1e-4,
        "{}",
        m.patience
    );
    assert!((m.tips - 1.03f32.powi(2)).abs() < 1e-4, "{}", m.tips);
    let until = c.handson.tuning.rituals.mood.minutes;
    assert_eq!(tips_mods(&w), vec![(1.03, Some(until)); 2]);
    let pat: Vec<_> = w
        .sys
        .fx
        .patience
        .iter()
        .filter(|p| p.1 == Some(until))
        .collect();
    assert_eq!(pat.len(), 2);
    // a guest waits more calmly: the wait clock runs slower by the mood
    // (and a tip in the first hour is partly the mood's)
    super::rituals::note_tip(&mut w, &c, 100);
    let share = w.hands.rituals.mood_tips;
    assert!(
        (share - 100.0 * (1.0 - 1.0 / 1.03f32.powi(2))).abs() < 0.01,
        "{share}"
    );
    // after 23:00 it's over: the mods go, a late item by hand adds a point but no effect
    w.skip_prep(&c);
    while w.minute() < until + 1 {
        w.tick(&c);
        bot::finish_cutscenes(&mut w, &c);
        w.drain_events();
    }
    assert!(tips_mods(&w).is_empty(), "the mood's tips mods expired");
    assert!(!w.ritual_mood(&c).active);
    do_item(&mut w, &c, "record_on", Some(false));
    assert_eq!(w.ritual_mood(&c).points, 3);
    assert!(tips_mods(&w).is_empty());
    super::rituals::note_tip(&mut w, &c, 100);
    assert!((w.hands.rituals.mood_tips - share).abs() < 1e-4);
}

#[test]
fn the_mood_patience_reaches_waiting_guests() {
    let c = content();
    let mut w = evening(&c);
    w.sys.fx.patience.clear();
    do_item(&mut w, &c, "light_window", Some(false));
    // the same multiplier list the wait clock reads (`pacing::wait_rate`)
    let now = w.minute();
    let mul: f32 = w
        .sys
        .fx
        .patience
        .iter()
        .filter(|(m, u)| *m > 0.0 && u.is_none_or(|u| u > now))
        .map(|p| p.0)
        .product();
    assert!((mul - 1.04).abs() < 1e-4, "{mul}");
}

#[test]
fn closing_by_hand_puts_the_crew_in_a_better_mood_next_night() {
    let c = content();
    let make = |crew: f32| {
        let mut w = World::new_sandbox(
            &c,
            SandboxSetup {
                seed: 7,
                ..Default::default()
            },
        );
        let ids: Vec<u32> = w.candidates().iter().map(|x| x.id).take(2).collect();
        for id in ids {
            w.apply(&c, Command::Hire { candidate: id }).unwrap();
        }
        w.hands.habits.crew_next = crew;
        w.start_night(&c);
        w
    };
    let base = make(0.0);
    let happy = make(3.0);
    let helpers = |w: &World| -> Vec<f32> {
        w.staff()
            .iter()
            .filter(|s| !s.owner)
            .map(|s| s.mood)
            .collect()
    };
    let (a, b) = (helpers(&base), helpers(&happy));
    assert!(!a.is_empty(), "hired");
    for (x, y) in a.iter().zip(&b) {
        assert!((y - x - 3.0).abs() < 1e-3 || *y >= 100.0, "{x} → {y}");
    }
    assert_eq!(happy.habits().crew_next, 0.0, "used up");
    assert_eq!(happy.crew_mood_from_rituals(), 3.0);
    // the closing items by hand fill it: one `crew_mood` each
    let mut w = evening(&c);
    w.debug_by_hand(&c, "wash_dishes", false);
    w.debug_by_hand(&c, "handover_note", true);
    let per = c.handson.tuning.rituals.crew_mood;
    assert!((w.habits().crew_next - 2.0 * per).abs() < 1e-4);
    let lines = super::rituals::summary_lines(&w, &c);
    assert!(
        lines
            .iter()
            .any(|l| l.key == "rituals.line.closing" && l.args.contains(&("n".into(), "2".into())))
    );
}

#[test]
fn the_stock_counted_by_hand_tells_what_is_on_the_shelves() {
    let c = content();
    let mut w = evening(&c);
    let events = do_item(&mut w, &c, "check_stock", Some(false));
    assert!(w.sys.fx.stock_watch);
    let n = events
        .iter()
        .filter(|e| matches!(e, SimEvent::BotMessage { template, .. } if template == "stock_check"))
        .count();
    assert_eq!(n, 1, "one stock message");
    let mut herself = evening(&c);
    let events = do_item(&mut herself, &c, "check_stock", None);
    assert!(!herself.sys.fx.stock_watch);
    assert!(
        !events.iter().any(
            |e| matches!(e, SimEvent::BotMessage { template, .. } if template == "stock_check")
        )
    );
}

#[test]
fn the_specials_board_by_hand_boosts_the_specials_tastes() {
    let c = content();
    let mut w = evening(&c);
    let known: Vec<String> = w
        .hands
        .knowledge
        .recipes
        .keys()
        .filter(|r| c.handson.recipe(r).is_some_and(|d| !d.tags.is_empty()))
        .take(2)
        .cloned()
        .collect();
    assert!(!known.is_empty());
    w.sys.extras.specials = known.clone();
    let before = w.sys.fx.wish_boosts.len();
    w.debug_by_hand(&c, "menu_board", false);
    let boosts = &w.sys.fx.wish_boosts[before..];
    assert_eq!(boosts.len(), 1, "{boosts:?}");
    let first = c.handson.recipe(&known[0]).unwrap().tags[0];
    assert!(boosts[0].0.contains(&first));
    assert!((boosts[0].1 - 1.2).abs() < 1e-4);
}

#[test]
fn a_habit_is_learned_once_after_five_nights_and_then_works_for_anyone() {
    let c = content();
    let nights = c.handson.tuning.rituals.habit_nights;
    assert_eq!(nights, 5);
    let mut w = evening(&c);
    w.hands.habits.nights.insert("record_on".into(), nights - 2);
    w.debug_by_hand(&c, "record_on", false);
    assert!(!w.habit_learned("record_on"), "four nights: not yet");
    let learned = |w: &mut World| {
        w.drain_events()
            .iter()
            .filter(|e| matches!(e, SimEvent::HabitLearned { item } if item == "record_on"))
            .count()
    };
    assert_eq!(learned(&mut w), 0);
    // the fifth night by hand
    let cozy_before = w.sys.fx.coziness.len();
    w.debug_by_hand(&c, "record_on", false);
    assert_eq!(w.habits().nights["record_on"], nights);
    assert!(w.habit_learned("record_on"));
    assert_eq!(learned(&mut w), 1, "the card, once");
    // hand_done +2 and the habit's +1 for the first hour
    assert_eq!(w.sys.fx.coziness.len(), cozy_before + 2);
    w.debug_by_hand(&c, "record_on", false);
    assert_eq!(learned(&mut w), 0, "never twice");
    assert_eq!(w.habits().learned, vec!["record_on".to_string()]);
    let lines = super::rituals::summary_lines(&w, &c);
    assert!(lines.iter().any(|l| {
        l.key == "rituals.line.habit"
            && l.args
                .contains(&("perk".into(), "@habit.record_on.perk".into()))
    }));
    // a later night: she puts the record on herself — the habit still acts
    let mut later = evening(&c);
    later.hands.habits = w.hands.habits.clone();
    later.sys.fx.coziness.clear();
    let rec = later.hands.music.unlocked.iter().next().cloned();
    later
        .apply(
            &c,
            Command::PlayRecord {
                id: rec,
                repeat: true,
            },
        )
        .unwrap();
    do_item(&mut later, &c, "record_on", None);
    assert!(
        later
            .sys
            .fx
            .coziness
            .iter()
            .any(|z| (z.0 - 1.0).abs() < 1e-4),
        "the habit's coziness: {:?}",
        later.sys.fx.coziness
    );
    assert_eq!(later.ritual_by_hand("record_on"), None);
}

#[test]
fn habits_survive_a_save_and_old_saves_load_without_them() {
    let c = content();
    let mut w = evening(&c);
    w.hands.habits.nights.insert("wipe_tables".into(), 3);
    w.hands.habits.learned.push("light_window".into());
    w.hands.habits.crew_next = 2.0;
    w.debug_by_hand(&c, "count_float", true);
    let s = w.save_to_string().unwrap();
    let mut back = World::load_from_str(&s).unwrap();
    back.rebuild_caches(&c);
    assert_eq!(back.habits(), w.habits());
    assert_eq!(back.habits().nights["count_float"], 1);
    assert_eq!(back.ritual_mood(&c).points, 1);
    let v1 = include_str!("testdata/save_v1_story_day3.ron");
    let old = World::load_from_str(v1).unwrap();
    assert_eq!(old.habits(), &Habits::default());
}

#[test]
fn the_morning_summary_lists_the_rituals_by_hand() {
    let c = content();
    let mut w = evening(&c);
    do_item(&mut w, &c, "light_window", Some(true));
    do_item(&mut w, &c, "wipe_tables", Some(false));
    let lines = super::rituals::summary_lines(&w, &c);
    let open = lines
        .iter()
        .find(|l| l.key.starts_with("rituals.line.opening"))
        .expect("the opening line");
    assert!(open.args.contains(&("n".into(), "2".into())));
    assert!(open.args.contains(&("total".into(), "7".into())));
    // the night's own summary carries them
    let mut b = bot::PlayerBot::new(bot::BotConfig {
        seed: 1,
        ..Default::default()
    });
    let _ = b.run_night(&mut w, &c);
    let sm = w.last_summary().expect("a summary");
    assert!(
        sm.rituals
            .iter()
            .any(|l| l.key.starts_with("rituals.line.opening"))
    );
}

#[test]
fn one_habit_a_night_the_next_one_waits_for_its_item() {
    let c = content();
    let mut w = evening(&c);
    w.debug_habit_nights(&c, 4);
    w.debug_by_hand(&c, "light_window", false);
    w.debug_by_hand(&c, "wipe_tables", false);
    assert!(w.habit_learned("light_window"));
    assert!(!w.habit_learned("wipe_tables"), "one a night");
    assert_eq!(w.habits().nights["wipe_tables"], 5);
    assert_eq!(
        super::rituals::summary_lines(&w, &c)
            .iter()
            .filter(|l| l.key == "rituals.line.habit")
            .count(),
        1
    );
    // the next night it comes the moment the tables are wiped — even by someone else
    let mut next = evening(&c);
    next.hands.habits = w.hands.habits.clone();
    next.drain_events();
    do_item(&mut next, &c, "wipe_tables", None);
    assert!(next.habit_learned("wipe_tables"));
    assert_eq!(
        next.habits().nights["wipe_tables"],
        5,
        "only by hand counts"
    );
}
