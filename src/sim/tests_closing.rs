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
