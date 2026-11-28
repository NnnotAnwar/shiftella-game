//! The end of the night (0.2.1 night flow): last call at 05:15, the café empties by 06:00,
//! the doors close and the quiet closing half hour (06:00–06:30) is for the closing list —
//! the till, the group head, the trash, the handover note, the window light. The night ends
//! when she closes up (or by itself once the list is done at 06:29); the morning (payments,
//! the digest, the summary) comes then, once.

use super::*;
use crate::content::Content;
use crate::content::world::ChecklistKind;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");
const QM: u32 = QUIET_TICKS_PER_MINUTE;

fn content() -> Content {
    Content::load(DATA).unwrap_or_else(|e| panic!("{e}"))
}

/// Plays with the bot until `stop` holds (or the night is over); returns the events with the
/// minute since 22:00 each came at.
fn play_until(
    w: &mut World,
    c: &Content,
    b: &mut bot::PlayerBot,
    stop: impl Fn(&World) -> bool,
) -> Vec<(i32, SimEvent)> {
    let mut out = Vec::new();
    let mut guard = 0;
    while w.phase() == Phase::Night && !stop(w) && guard < 60_000 {
        b.finish_scenes(w, c);
        b.step(w, c);
        if w.phase() != Phase::Night {
            let m = w.minutes_since_open();
            out.extend(w.drain_events().into_iter().map(|e| (m, e)));
            break;
        }
        w.tick(c);
        let m = w.minutes_since_open();
        out.extend(w.drain_events().into_iter().map(|e| (m, e)));
        guard += 1;
    }
    out
}

fn bot(seed: u64) -> bot::PlayerBot {
    bot::PlayerBot::new(bot::BotConfig {
        seed,
        ..Default::default()
    })
}

/// A night `n` started (story, Cozy) and played to `until` minutes since 22:00 by the bot.
fn story_at(c: &Content, n: u32, until: i32) -> (World, bot::PlayerBot, Vec<(i32, SimEvent)>) {
    let mut w = World::new_story_seeded(c, Difficulty::Cozy, 77);
    let mut b = bot(77);
    while w.night() < n {
        b.run_night(&mut w, c);
    }
    b.day(&mut w, c);
    b.finish_scenes(&mut w, c);
    w.start_night(c);
    let ev = play_until(&mut w, c, &mut b, |w| w.minutes_since_open() >= until);
    (w, b, ev)
}

#[test]
fn nobody_comes_in_after_last_call() {
    let c = content();
    assert_eq!(LAST_CALL, 7 * 60 + 15, "05:15");
    // story nights with late trains (the 05:20 R 815 on night 23), a sandbox with the hospital
    // shift change
    let mut worlds = Vec::new();
    for n in [2u32, 6] {
        let (mut w, mut b, mut ev) = story_at(&c, n, 0);
        ev.extend(play_until(&mut w, &c, &mut b, |w| w.in_closing()));
        worlds.push((format!("story {n}"), w, ev));
    }
    let mut s = World::new_sandbox(
        &c,
        SandboxSetup {
            seed: 9,
            ..Default::default()
        },
    );
    s.town.institutions.push("hospital".into());
    let mut b = bot(9);
    b.day(&mut s, &c);
    s.start_night(&c);
    let ev = play_until(&mut s, &c, &mut b, |w| w.in_closing());
    worlds.push(("sandbox".into(), s, ev));
    for (name, w, ev) in &worlds {
        assert!(w.in_closing(), "{name}");
        let arrivals: Vec<i32> = ev
            .iter()
            .filter(|(_, e)| matches!(e, SimEvent::GuestArrived { .. }))
            .map(|(m, _)| *m)
            .collect();
        assert!(!arrivals.is_empty(), "{name}: guests came");
        assert!(
            arrivals.iter().all(|m| (0..LAST_CALL as i32).contains(m)),
            "{name}: arrivals at {:?}",
            arrivals
                .iter()
                .filter(|m| **m >= LAST_CALL as i32)
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn the_cafe_is_empty_when_the_doors_close_at_six() {
    let c = content();
    for n in [3u32, 7] {
        let (mut w, mut b, _) = story_at(&c, n, 0);
        let ev = play_until(&mut w, &c, &mut b, |w| w.in_closing());
        let closed = ev
            .iter()
            .filter(|(_, e)| *e == SimEvent::DoorsClosed)
            .count();
        assert_eq!(closed, 1, "night {n}: the doors close once");
        assert_eq!(w.clock_label(), "06:00");
        assert!(!w.doors_open() && w.in_closing());
        assert!(
            w.parties().iter().all(|p| p.state == PartyState::Leaving),
            "night {n}: nobody is still waiting or eating at 06:00"
        );
        // the wrap-up from 05:45 never made anyone leave unhappy
        let left_unhappy = w.tonight().guests_left_unhappy;
        assert_eq!(left_unhappy, 0, "night {n}");
        // a couple of minutes later the last ones are out of the door
        let mut guard = 0;
        while w.guests_inside() > 0 && guard < 5 * QM {
            w.tick(&c);
            guard += 1;
        }
        assert_eq!(w.guests_inside(), 0, "night {n}: the café is empty");
        assert!(w.in_closing());
    }
}

#[test]
fn the_closing_chores_are_for_the_closing_half_hour() {
    let c = content();
    let (mut w, _, _) = story_at(&c, 4, 3 * 60);
    w.auto_doors = false;
    // 01:00 with guests inside: the till and the window light wait
    assert!(!w.closing_time());
    for id in ["light_off_at_six", "cash_up", "clean_machine"] {
        assert!(!w.checklist_item_open(&c, id), "{id} at 01:00");
        assert!(
            w.apply(&c, Command::TickChecklist { id: id.into() })
                .is_err(),
            "{id} can't be done while guests sit"
        );
    }
    assert!(
        w.apply(&c, Command::CloseUp).is_err(),
        "Close up only after 06:00"
    );
    // the doors close at 06:00 (nobody touches the closing list till then)
    let mut guard = 0;
    while !w.in_closing() && guard < 60_000 {
        bot::finish_cutscenes(&mut w, &c);
        w.tick(&c);
        guard += 1;
    }
    w.drain_events();
    assert!(w.closing_time());
    let k = w.karolina().unwrap().id;
    // Karolina does the closing list by hand: the till, the group head, the window light,
    // the note — and takes the trash out
    let mut todo: Vec<String> = c
        .app
        .checklists
        .iter()
        .filter(|d| d.list == ChecklistKind::Closing)
        .map(|d| d.id.clone())
        .collect();
    let bin = w
        .furniture()
        .iter()
        .find(|p| w.equipment_action(&c, p.uid) == Some(EquipAction::Trash))
        .map(|p| p.uid);
    let mut trash = bin.is_some();
    let mut ticked = Vec::new();
    for _ in 0..CLOSE_TICKS * 2 {
        if w.phase() != Phase::Night {
            break;
        }
        if trash
            && let Some(uid) = bin
            && w.apply(
                &c,
                Command::UseEquipment {
                    uid,
                    action: EquipAction::Trash,
                },
            )
            .is_ok()
        {
            trash = false;
        }
        if let Some(id) = todo.first().cloned() {
            let open = w
                .app()
                .checklist
                .iter()
                .any(|s| s.id == id && !s.done && s.claim.is_none());
            if !open
                || w.apply(&c, Command::TickChecklist { id: id.clone() })
                    .is_ok()
            {
                todo.remove(0);
            }
        }
        if let Some(cur) = &w.player().current
            && let PlayerAct::Wash { sink } = cur.act
            && cur.stage == 1
        {
            let _ = w.apply(&c, Command::PlayerWashProgress { sink, seconds: 2.0 });
        }
        if let Some(cur) = &w.player().current
            && let PlayerAct::Use { action, .. } = cur.act
            && action.hold()
            && cur.stage == 1
        {
            let _ = w.apply(&c, Command::EquipmentHold { seconds: 1.0 });
        }
        w.tick(&c);
        for e in w.drain_events() {
            if let SimEvent::ChecklistTicked { id } = &e
                && w.in_closing()
            {
                ticked.push(id.clone());
            }
            if let SimEvent::NightEnded { .. } = e {
                ticked.push("<end>".into());
            }
        }
    }
    for id in ["light_off_at_six", "handover_note"] {
        assert!(
            ticked.contains(&id.to_string()),
            "{id} in the closing half hour: {ticked:?}"
        );
    }
    let s = w.summaries().last().unwrap();
    assert_eq!(s.night, 4);
    // the whole closing list done (by her and the crew): the night ended by itself at 06:29
    assert_eq!(
        ticked.last().map(String::as_str),
        Some("<end>"),
        "{ticked:?}"
    );
    assert!(w.sys.tool.closed_properly, "the closing ritual");
    assert!(!w.window_light_on(&c), "the window light is off");
    let _ = k;
}

#[test]
fn close_up_ends_the_night_at_once_and_the_morning_comes_once() {
    let c = content();
    // night 7: a payment night in the story? whatever it is, the morning happens once, after
    // the doors closed
    let (mut w, mut b, mut ev) = story_at(&c, 7, 0);
    // (a full crew with two duties each can finish the closing list within minutes: close up
    // a minute in)
    ev.extend(play_until(&mut w, &c, &mut b, |w| w.in_closing()));
    ev.extend(play_until(&mut w, &c, &mut b, |w| {
        !w.in_closing() || w.close_minute() >= 1
    }));
    assert!(w.in_closing());
    let before = w.summaries().len();
    let closed_at = ev
        .iter()
        .position(|(_, e)| *e == SimEvent::DoorsClosed)
        .unwrap();
    assert!(
        !ev.iter().any(|(_, e)| matches!(
            e,
            SimEvent::NightEnded { .. }
                | SimEvent::PaymentMade { .. }
                | SimEvent::PaymentMissed { .. }
        )),
        "nothing of the morning before she closes up"
    );
    assert!(closed_at > 0);
    w.apply(&c, Command::CloseUp).unwrap();
    let after = w.drain_events();
    assert_eq!(
        after
            .iter()
            .filter(|e| matches!(e, SimEvent::NightEnded { .. }))
            .count(),
        1
    );
    assert!(
        after
            .iter()
            .any(|e| matches!(e, SimEvent::DayStarted { night: 8 }))
    );
    assert_eq!(w.summaries().len(), before + 1);
    assert_eq!(w.phase(), Phase::Day);
    // the digest went out once
    assert_eq!(
        after
            .iter()
            .filter(
                |e| matches!(e, SimEvent::BotMessage { template, .. } if template == "night_digest")
            )
            .count(),
        1
    );
    assert!(
        w.apply(&c, Command::CloseUp).is_err(),
        "only in the closing half hour"
    );
    // the shift on the timesheet is still 22:00–06:00
    let s = w.summaries().last().unwrap();
    assert_eq!(s.night, 7);
}

#[test]
fn the_clock_waits_at_half_past_six_until_she_closes_up() {
    let c = content();
    let (mut w, _, _) = story_at(&c, 2, 0);
    w.auto_doors = false;
    let mut guard = 0;
    while !w.in_closing() && guard < 60_000 {
        bot::finish_cutscenes(&mut w, &c);
        w.tick(&c);
        guard += 1;
    }
    // nobody does the closing list: 06:29, and the clock waits
    for _ in 0..CLOSE_TICKS + 40 * QM {
        w.tick(&c);
    }
    assert!(w.waiting_to_close());
    assert_eq!(w.clock_label(), "06:29");
    assert_eq!(w.phase(), Phase::Night);
    assert_eq!(w.minute(), NIGHT_MINUTES, "the night clock stands at 06:00");
    // the closing half hour isn't paid: nobody worked past 480 minutes
    assert!(w.staff().iter().all(|s| s.minutes_worked <= NIGHT_MINUTES));
    w.drain_events();
    w.apply(&c, Command::CloseUp).unwrap();
    assert_eq!(w.phase(), Phase::Day);
    // whatever was left on the closing list was missed: no proper close
    assert!(!w.sys.tool.closed_properly);
}

#[test]
fn the_quiet_halves_run_slower_and_the_night_length_stays_reasonable() {
    let c = content();
    let mut w = World::new_story_seeded(&c, Difficulty::Cozy, 5);
    let mut b = bot(5);
    for _ in 0..3 {
        b.run_night(&mut w, &c);
    }
    let tick_s = c.handson.tuning.real_seconds_per_tick_x1 * c.handson.tuning.prep_tick_scale;
    let half = PREP_TICKS as f32 * tick_s / 60.0;
    assert!(
        (1.4..=2.0).contains(&half),
        "a quiet half at ×1: {half:.2} real minutes"
    );
    assert_eq!(PREP_TICKS, CLOSE_TICKS);
    for n in &b.nights {
        assert!(
            n.closing_ticks > 0,
            "night {}: the closing half hour ran",
            n.night
        );
        assert!(
            (10.0..=19.0).contains(&n.real_minutes),
            "night {}: {:.1} real minutes at ×1",
            n.night,
            n.real_minutes
        );
    }
}

#[test]
fn the_storm_night_strangers_stay_till_six_then_walk_out() {
    let c = content();
    let mut w = World::new_sandbox(
        &c,
        SandboxSetup {
            seed: 21,
            ..Default::default()
        },
    );
    let mut b = bot(21);
    b.day(&mut w, &c);
    w.start_night(&c);
    w.stories.storm.tonight = true;
    play_until(&mut w, &c, &mut b, |w| {
        w.minutes_since_open() >= 7 * 60 + 55
    });
    let stranded = w.storm_stranded();
    assert!(
        !stranded.is_empty(),
        "strangers are still inside at 05:55 (the line is closed)"
    );
    let ev = play_until(&mut w, &c, &mut b, |w| w.in_closing());
    assert!(ev.iter().any(|(_, e)| *e == SimEvent::DoorsClosed));
    // the doors open for them at 06:00: they pay and walk out into the morning
    assert!(w.storm_stranded().is_empty());
    assert!(w.parties().iter().all(|p| p.state == PartyState::Leaving));
    let ev = play_until(&mut w, &c, &mut b, |w| w.phase() == Phase::Day);
    assert!(
        ev.iter()
            .any(|(_, e)| matches!(e, SimEvent::StormMorning { .. })),
        "the storm's morning at the night's real end"
    );
}

/// A 0.2.0 save taken mid-night (no `close_tick` / `flow`) loads, plays on through 06:00 into
/// the closing half hour and ends; a v1 fixture mid-night too.
#[test]
fn an_old_mid_night_save_loads_and_reaches_the_closing_half_hour() {
    let c = content();
    let (w, _, _) = story_at(&c, 3, 4 * 60);
    let text = w.save_to_string().unwrap();
    assert!(text.contains("close_tick:0,") && text.contains("flow:1,"));
    let old = text.replace("close_tick:0,", "").replace("flow:1,", "");
    let mut loaded = World::load_from_str(&old).unwrap();
    assert_eq!(loaded.clock_label(), w.clock_label());
    assert!(loaded.doors_open());
    let ev = loaded.run_night_headless(&c);
    assert!(ev.contains(&SimEvent::DoorsClosed));
    assert_eq!(
        ev.iter()
            .filter(|e| matches!(e, SimEvent::NightEnded { .. }))
            .count(),
        1
    );
    let v1 = include_str!("testdata/save_v1_sandbox_night.ron");
    let mut d = World::load_from_str(v1).unwrap();
    d.rebuild_caches(&c);
    let ev = d.run_night_headless(&c);
    assert!(ev.contains(&SimEvent::DoorsClosed));
    assert!(ev.iter().any(|e| matches!(e, SimEvent::NightEnded { .. })));
}

// ---------------------------------------------------------------------------------------------
// fix/closing (the owner: "at closing, with guests still in, the boxes for the coffee machine
// and the lamp show — I can't press them and Karolina does nothing"; "make counting the till
// the closing's last step"; "the lamp looks ON when I switch it off").
// ---------------------------------------------------------------------------------------------

/// The night `n` played by the bot to `until`, then left alone (the bot doesn't touch the
/// closing list) until `stop`.
fn story_then_alone(c: &Content, n: u32, until: i32, stop: impl Fn(&World) -> bool) -> World {
    let (mut w, _, _) = story_at(c, n, until);
    w.auto_doors = false;
    let mut guard = 0;
    while w.phase() == Phase::Night && !stop(&w) && guard < 60_000 {
        bot::finish_cutscenes(&mut w, c);
        w.tick(c);
        guard += 1;
    }
    w.drain_events();
    w
}

/// Every closing item at every moment is either doable (a click queues it for her) or locked
/// with a reason the UI shows — never an open box that does nothing.
fn assert_no_dead_items(w: &World, c: &Content, when: &str) {
    for d in c.app.checklists.iter() {
        let open = w.checklist_item_open(c, &d.id);
        let lock = w.checklist_lock(c, &d.id);
        let mark = w.checklist_mark(&d.id);
        if mark != ChecklistMark::Open {
            continue;
        }
        assert_eq!(
            open,
            lock.is_none(),
            "{when}: {} open {open} lock {lock:?}",
            d.id
        );
        // what the click does agrees with the lock (her hands free)
        let mut probe = w.clone();
        let _ = probe.apply(c, Command::PlayerClearQueue);
        let _ = probe.apply(c, Command::PlayerCancel);
        let r = probe.apply(c, Command::TickChecklist { id: d.id.clone() });
        assert_eq!(
            r.is_ok(),
            lock.is_none(),
            "{when}: {} — the click {:?} but the lock says {lock:?}",
            d.id,
            r
        );
    }
}

#[test]
fn closing_items_with_guests_still_in_are_doable_or_locked_with_a_reason() {
    let c = content();
    // after last call, guests still at their tables
    let w = story_then_alone(&c, 4, LAST_CALL as i32 + 3, |_| true);
    assert!(
        w.guests_inside() > 0,
        "guests are still in at {}",
        w.clock_label()
    );
    assert!(!w.closing_time());
    assert_no_dead_items(&w, &c, "after last call, guests in");
    // the ones that need an empty café / the closed doors say so
    assert_eq!(
        w.checklist_lock(&c, "light_off_at_six"),
        Some(ChecklistLock::GuestsInside)
    );
    assert_eq!(
        w.checklist_lock(&c, TILL_ITEM),
        Some(ChecklistLock::DoorsClosed)
    );
    // the dishes, the fridge, the handover note: she can do them while the last guests finish
    for id in ["wash_dishes", "fridge_check", "handover_note"] {
        assert!(w.checklist_item_open(&c, id), "{id} at {}", w.clock_label());
    }
    // the group head: once nobody waits for a coffee any more, even with guests still eating
    let mut w2 = w.clone();
    for p in w2.parties.iter_mut() {
        if matches!(
            p.state,
            PartyState::Arriving | PartyState::Queue | PartyState::WaitOrder
        ) {
            p.state = PartyState::Eating;
        }
    }
    for i in w2.items.iter_mut() {
        if matches!(i.state, ItemState::Pending | ItemState::Preparing) {
            i.state = ItemState::Served;
        }
    }
    if let Some(it) = w2
        .app
        .checklist
        .iter_mut()
        .find(|i| i.id == "clean_machine")
    {
        it.done = false;
        it.claim = None;
    }
    assert!(w2.guests_inside() > 0);
    assert_eq!(w2.checklist_lock(&c, "clean_machine"), None);
    let _ = w2.apply(&c, Command::PlayerClearQueue);
    let _ = w2.apply(&c, Command::PlayerCancel);
    let r = w2.apply(
        &c,
        Command::TickChecklist {
            id: "clean_machine".into(),
        },
    );
    assert!(
        r.is_ok(),
        "she cleans the group head after last call with the last guests still eating: {r:?}"
    );
    // a coffee still to be made: the group head waits for it, and says why
    let mut w3 = w2.clone();
    if let Some(i) = w3.items.iter_mut().find(|i| {
        c.menu_item(&i.menu)
            .is_some_and(|m| m.station == "station:coffee")
    }) {
        i.state = ItemState::Pending;
        assert_eq!(
            w3.checklist_lock(&c, "clean_machine"),
            Some(ChecklistLock::Brewing)
        );
    }
    // earlier in the night: the closing list is locked "from 05:00", never dead
    let early = story_then_alone(&c, 4, 3 * 60, |_| true);
    assert_no_dead_items(&early, &c, "01:00");
    assert_eq!(
        early.checklist_lock(&c, "wash_dishes"),
        Some(ChecklistLock::FromFive)
    );
    assert_eq!(
        early.checklist_lock(&c, "clean_machine"),
        Some(ChecklistLock::FromFive)
    );
    // the closing half hour: an opening item left undone is "too late", the closing list open
    let closing = story_then_alone(&c, 4, LAST_CALL as i32, |w| w.in_closing());
    assert!(closing.in_closing());
    assert_no_dead_items(&closing, &c, "06:00");
    assert_eq!(
        closing.checklist_lock(&c, "wipe_tables"),
        Some(ChecklistLock::TooLate)
    );
    for d in c
        .app
        .checklists
        .iter()
        .filter(|d| d.list == ChecklistKind::Closing)
    {
        assert_eq!(closing.checklist_lock(&c, &d.id), None, "{} at 06:00", d.id);
    }
}

#[test]
fn the_preparation_half_hour_has_no_dead_items() {
    let c = content();
    let mut w = World::new_story_seeded(&c, Difficulty::Cozy, 77);
    w.start_night(&c);
    w.auto_doors = false;
    for _ in 0..5 * QM {
        bot::finish_cutscenes(&mut w, &c);
        w.tick(&c);
    }
    assert!(w.in_prep());
    assert_no_dead_items(&w, &c, "21:35");
    for d in c
        .app
        .checklists
        .iter()
        .filter(|d| d.list == ChecklistKind::Opening)
    {
        assert!(
            w.checklist_item_open(&c, &d.id),
            "{} in the preparation",
            d.id
        );
    }
    for d in c
        .app
        .checklists
        .iter()
        .filter(|d| d.list == ChecklistKind::Closing)
    {
        assert_eq!(
            w.checklist_lock(&c, &d.id),
            Some(ChecklistLock::FromFive),
            "{}",
            d.id
        );
    }
}

#[test]
fn counting_the_till_ends_the_night_and_the_clock_waits_until_she_does() {
    let c = content();
    let mut w = story_then_alone(&c, 5, LAST_CALL as i32, |w| w.in_closing());
    assert!(w.in_closing());
    assert!(w.till_open(&c));
    // the rest of the closing list done (by her, one after the other): no end yet
    let rest: Vec<String> = c
        .app
        .checklists
        .iter()
        .filter(|d| d.list == ChecklistKind::Closing && d.id != TILL_ITEM)
        .map(|d| d.id.clone())
        .collect();
    let mut guard = 0;
    while !w.closing_rest_done(&c) && guard < CLOSE_TICKS * 3 {
        for id in &rest {
            if w.checklist_mark(id) == ChecklistMark::Open {
                let _ = w.apply(&c, Command::TickChecklist { id: id.clone() });
            }
        }
        if let Some(cur) = &w.player().current
            && let PlayerAct::Wash { sink } = cur.act
            && cur.stage == 1
        {
            let _ = w.apply(&c, Command::PlayerWashProgress { sink, seconds: 2.0 });
        }
        w.tick(&c);
        guard += 1;
    }
    assert!(w.closing_rest_done(&c), "the rest of the list is done");
    // nobody counts the till: 06:29, and the clock waits — the whole list isn't done
    for _ in 0..CLOSE_TICKS + 10 * QM {
        w.tick(&c);
    }
    assert!(w.waiting_to_close(), "{}", w.clock_label());
    assert_eq!(w.clock_label(), "06:29");
    assert_eq!(w.phase(), Phase::Night, "the clock waits for the till");
    assert!(!w.till_counted());
    w.drain_events();
    // she counts the till: the night's take, a short beat, then the morning — once
    w.apply(
        &c,
        Command::TickChecklist {
            id: TILL_ITEM.into(),
        },
    )
    .unwrap();
    let mut ev = Vec::new();
    let mut ticks = 0;
    while w.phase() == Phase::Night && ticks < 3_000 {
        w.tick(&c);
        ev.extend(w.drain_events());
        ticks += 1;
    }
    assert_eq!(w.phase(), Phase::Day, "counting the till ends the night");
    let till = ev
        .iter()
        .position(|e| matches!(e, SimEvent::TillCounted { .. }))
        .expect("the night's take");
    let end = ev
        .iter()
        .position(|e| matches!(e, SimEvent::NightEnded { .. }))
        .expect("the morning");
    assert!(till < end, "the take comes first, then the morning");
    assert_eq!(
        ev.iter()
            .filter(|e| matches!(e, SimEvent::NightEnded { .. }))
            .count(),
        1
    );
    if let Some(SimEvent::TillCounted { revenue, .. }) = ev.get(till) {
        assert_eq!(*revenue, w.summaries().last().unwrap().revenue);
    }
    // the whole list, the till last: a proper close
    let s = w.summaries().last().unwrap();
    assert_eq!(s.night, 5);
    assert!(
        w.sys.tool.closed_properly || s.checklist_done > 0,
        "the closing ritual"
    );
}

#[test]
fn the_till_is_hers_and_waits_for_the_closed_doors() {
    let c = content();
    let def = c.app.checklists.iter().find(|d| d.id == TILL_ITEM).unwrap();
    assert_eq!(def.list, ChecklistKind::Closing);
    assert_eq!(
        def.owner(),
        crate::content::handson::ChecklistOwner::Karolina,
        "the till is Karolina's own last step, never a helper's"
    );
    assert_eq!(
        c.app
            .checklists
            .iter()
            .rfind(|d| d.list == ChecklistKind::Closing)
            .map(|d| d.id.as_str()),
        Some(TILL_ITEM),
        "the last step is last on the list"
    );
    // an empty café after last call: still not the till (it ends the night)
    let mut w = story_then_alone(&c, 3, LAST_CALL as i32, |w| {
        w.closing_time() || w.in_closing()
    });
    if !w.in_closing() {
        assert!(w.closing_time());
        assert_eq!(
            w.checklist_lock(&c, TILL_ITEM),
            Some(ChecklistLock::DoorsClosed)
        );
        assert!(
            w.apply(
                &c,
                Command::TickChecklist {
                    id: TILL_ITEM.into()
                }
            )
            .is_err()
        );
    }
    // helpers with the register duty never count it: the closing half hour with nobody at
    // the till ends only when she does
    let mut guard = 0;
    while w.phase() == Phase::Night && !w.waiting_to_close() && guard < 60_000 {
        bot::finish_cutscenes(&mut w, &c);
        w.tick(&c);
        guard += 1;
    }
    assert!(w.waiting_to_close());
    assert!(!w.till_counted(), "nobody but her counts the till");
}

#[test]
fn the_window_lamp_follows_the_opening_and_the_closing() {
    let c = content();
    let mut w = World::new_story_seeded(&c, Difficulty::Cozy, 77);
    w.start_night(&c);
    w.auto_doors = false;
    bot::finish_cutscenes(&mut w, &c);
    assert!(!w.window_light_on(&c), "the window lamp starts off");
    w.apply(
        &c,
        Command::TickChecklist {
            id: "light_window".into(),
        },
    )
    .unwrap();
    for _ in 0..QM * 10 {
        if w.checklist_mark("light_window") != ChecklistMark::Open
            && matches!(w.checklist_mark("light_window"), ChecklistMark::Done { .. })
        {
            break;
        }
        w.tick(&c);
    }
    assert!(w.window_light_on(&c), "lit at the opening");
    // 03:33 with guests in: the lamp stays lit for the 333 guest, the closing item locked
    let mut guard = 0;
    while w.phase() == Phase::Night && w.minutes_since_open() < 5 * 60 + 33 && guard < 60_000 {
        bot::finish_cutscenes(&mut w, &c);
        if w.waiting_for_doors() {
            w.open_doors(&c);
        }
        w.tick(&c);
        guard += 1;
    }
    assert!(w.window_light_on(&c) || w.blackout(), "lit at 03:33");
    assert!(!w.checklist_item_open(&c, "light_off_at_six"));
    // the closing: off
    while w.phase() == Phase::Night && !w.in_closing() && guard < 120_000 {
        bot::finish_cutscenes(&mut w, &c);
        w.tick(&c);
        guard += 1;
    }
    bot::finish_cutscenes(&mut w, &c);
    let _ = w.apply(&c, Command::PlayerClearQueue);
    let _ = w.apply(&c, Command::PlayerCancel);
    w.apply(
        &c,
        Command::TickChecklist {
            id: "light_off_at_six".into(),
        },
    )
    .unwrap();
    for _ in 0..QM * 10 {
        if matches!(
            w.checklist_mark("light_off_at_six"),
            ChecklistMark::Done { .. }
        ) {
            break;
        }
        bot::finish_cutscenes(&mut w, &c);
        w.tick(&c);
    }
    assert!(matches!(
        w.checklist_mark("light_off_at_six"),
        ChecklistMark::Done { .. }
    ));
    assert!(!w.window_light_on(&c), "dark after the closing");
}
