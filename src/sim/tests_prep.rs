//! The preparation half hour before the doors open (21:30–22:00): no guests, the opening
//! ritual is doable in peace, the crew walks in at once and helps, the clock waits at 21:59
//! until she opens the doors (or the opening list is done: they open by themselves), saves
//! mid-prep, older saves.

use super::*;
use crate::content::Content;
use crate::content::world::ChecklistKind;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

fn content() -> Content {
    Content::load(DATA).unwrap_or_else(|e| panic!("{e}"))
}

fn ticks(w: &mut World, c: &Content, n: u32) -> Vec<SimEvent> {
    let mut out = Vec::new();
    for _ in 0..n {
        w.tick(c);
        bot::finish_cutscenes(w, c);
        out.extend(w.drain_events());
    }
    out
}

/// Game minutes of the preparation as ticks.
const QM: u32 = QUIET_TICKS_PER_MINUTE;

/// A story night `n` at 21:30 (cutscenes done), the tutorial off.
fn story_evening(c: &Content, n: u32) -> World {
    let mut w = World::new_story(c, Difficulty::Cozy);
    w.sys.tutorial_off = true;
    if n > 1 {
        w.night = n;
        w.begin_day(c);
    }
    bot::finish_cutscenes(&mut w, c);
    w.start_night(c);
    bot::finish_cutscenes(&mut w, c);
    w.drain_events();
    // the clock waits at 21:59 for her (as in the game)
    w.auto_doors = false;
    w
}

fn opening_ids(c: &Content) -> Vec<String> {
    c.app
        .checklists
        .iter()
        .filter(|d| d.list == ChecklistKind::Opening)
        .map(|d| d.id.clone())
        .collect()
}

#[test]
fn the_night_begins_at_half_past_nine_with_the_doors_closed() {
    let c = content();
    let w = story_evening(&c, 1);
    assert!(w.in_prep());
    assert!(!w.doors_open());
    assert_eq!(w.clock_label(), "21:30");
    assert_eq!(w.minutes_since_open(), -30);
    assert_eq!(w.minute(), 0, "the night clock waits at 22:00");
    assert_eq!(w.evening_progress(), 0.0);
    let base = c.handson.tuning.real_seconds_per_tick_x1;
    let prep = w.real_seconds_per_tick(&c);
    assert!((prep - base * c.handson.tuning.prep_tick_scale).abs() < 1e-6);
    // the owner: "time runs too fast for opening" — the half hour is 1.5–2 real minutes at ×1
    let real = PREP_TICKS as f32 * prep / 60.0;
    assert!((1.4..=2.0).contains(&real), "the preparation takes {real:.2} real minutes");
}

#[test]
fn nobody_comes_in_before_the_doors_open_at_ten() {
    let c = content();
    // a busy night with helpers, trains, regulars and events — all of it after 22:00
    for n in [1u32, 9, 18] {
        let mut w = story_evening(&c, n);
        // the preparation runs to 21:59, then the clock waits for her (nobody did the list)
        let ev = ticks(&mut w, &c, PREP_TICKS + 40 * QM);
        assert!(w.in_prep(), "night {n}");
        assert!(w.waiting_for_doors(), "night {n}: the clock waits at 21:59");
        assert_eq!(w.clock_label(), "21:59");
        for e in &ev {
            assert!(
                !matches!(
                    e,
                    SimEvent::GuestArrived { .. }
                        | SimEvent::RegularArrived { .. }
                        | SimEvent::RushStarted
                        | SimEvent::EventStarted { .. }
                        | SimEvent::ChoiceNeeded { .. }
                        | SimEvent::MomentAvailable { .. }
                        | SimEvent::PlayCutscene(_)
                        | SimEvent::DoorsOpened { .. }
                ),
                "night {n}: {e:?} before 22:00"
            );
        }
        assert!(w.parties().is_empty() && w.guests().is_empty());
        assert_eq!(w.minute(), 0, "night {n}: the night clock still waits at 22:00");
        w.apply(&c, Command::OpenDoors).unwrap();
        assert!(
            w.drain_events()
                .contains(&SimEvent::DoorsOpened { early_minutes: 0 }),
            "night {n}: she opens the doors at 21:59"
        );
        assert!(w.doors_open());
        assert_eq!(w.clock_label(), "22:00");
        // …and the night goes on as before
        let ev = ticks(&mut w, &c, 60 * TICKS_PER_MINUTE);
        assert!(
            ev.iter()
                .any(|e| matches!(e, SimEvent::GuestArrived { .. })),
            "night {n}: guests after 22:00"
        );
    }
}

#[test]
fn a_train_passes_by_while_the_doors_are_closed() {
    let c = content();
    let mut w = story_evening(&c, 3);
    let ev = ticks(&mut w, &c, PREP_TICKS);
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::TrainPassing { train, .. } if train == "Nex 9047")),
        "the 21:47 freight"
    );
    assert!(w.parties().is_empty(), "nobody gets off for the café");
}

#[test]
fn karolina_can_finish_the_opening_ritual_before_the_first_guests() {
    let c = content();
    // night 1: solo, every item is hers
    let mut w = story_evening(&c, 1);
    // she queues the items one after another as her hands free up (a few at a time)
    let mut todo = opening_ids(&c);
    let mut ready = None;
    let mut open = None;
    for _ in 0..PREP_TICKS * 2 {
        if w.doors_open() {
            break;
        }
        if let Some(id) = todo.first().cloned() {
            let r = if id == "menu_board" {
                let s = w.specials().to_vec();
                w.apply(&c, Command::SetSpecials { recipes: s })
            } else {
                w.apply(&c, Command::TickChecklist { id })
            };
            match r {
                Ok(()) => {
                    todo.remove(0);
                }
                Err(CommandError::QueueFull) => {}
                Err(e) => panic!("{e:?}"),
            }
        }
        w.tick(&c);
        for e in w.drain_events() {
            match e {
                SimEvent::RitualComplete {
                    list: ChecklistKind::Opening,
                } => ready = Some(w.minutes_since_open()),
                SimEvent::DoorsOpened { .. } => open = Some(w.minutes_since_open()),
                _ => {}
            }
        }
    }
    let ready = ready.expect("\"The café is ready.\" before the doors open");
    assert!(ready < 0, "ready at {ready} (before 22:00)");
    // the list is done: at 21:59 the doors open by themselves (the clock jumps to 22:00)
    assert_eq!(open, Some(0));
    // the ritual's mood/coziness bonus is on for the night
    assert!(w.sys.tool.rituals[0]);
}

#[test]
fn the_crew_walks_in_at_half_past_nine_and_takes_their_items() {
    let c = content();
    let mut w = World::new_sandbox(&c, SandboxSetup::default());
    bot::finish_cutscenes(&mut w, &c);
    let ids: Vec<u32> = w.candidates().iter().map(|x| x.id).take(2).collect();
    for id in ids {
        w.apply(&c, Command::Hire { candidate: id }).unwrap();
    }
    // both new helpers work tonight (their own day-off wishes hash off their ids)
    let wd = w.weekday;
    let hired: Vec<u32> = w.staff().iter().rev().take(2).map(|s| s.id).collect();
    for id in hired {
        w.apply(
            &c,
            Command::SetShift {
                staff: id,
                weekday: wd,
                shift: Shift::Full,
            },
        )
        .unwrap();
    }
    w.apply(&c, Command::PublishSchedule).unwrap();
    for s in w.staff.iter_mut().filter(|s| !s.owner) {
        s.duties = vec![
            crate::content::handson::Duty::Bar,
            crate::content::handson::Duty::Kitchen,
            crate::content::handson::Duty::Register,
        ];
    }
    w.start_night(&c);
    bot::finish_cutscenes(&mut w, &c);
    w.drain_events();
    // (a late helper would come after 22:00: make them punctual for the test)
    for s in w.staff.iter_mut().filter(|s| !s.owner) {
        if matches!(s.status, StaffStatus::Expected { .. }) {
            s.status = StaffStatus::Expected { minute: 0 };
            s.late_minutes = 0;
        }
    }
    let helpers: Vec<u32> = w
        .staff()
        .iter()
        .filter(|s| !s.owner && s.status == (StaffStatus::Expected { minute: 0 }))
        .map(|s| s.id)
        .collect();
    assert_eq!(helpers.len(), 2, "both helpers are on tonight's rozpis");
    let k = w.karolina().unwrap();
    assert!(k.on_duty(), "Karolina is inside at 21:30");
    let ev = ticks(&mut w, &c, 5 * QM);
    assert_eq!(w.clock_label(), "21:35");
    for id in &helpers {
        assert!(
            ev.contains(&SimEvent::StaffArrived { staff: *id }),
            "helper {id} came in during the preparation"
        );
        assert_eq!(
            w.staff_by_id(*id).unwrap().minutes_worked,
            0,
            "the preparation isn't on the timesheet"
        );
    }
    // with nobody to serve, helpers do the opening items of their duties
    ticks(&mut w, &c, 14 * QM);
    let by_helper = w
        .app()
        .checklist
        .iter()
        .filter(|s| s.done && s.by.is_some_and(|b| helpers.contains(&b)))
        .count();
    assert!(
        by_helper > 0,
        "a helper ticked an opening item before 22:00"
    );
    assert!(w.in_prep());
}

#[test]
fn open_the_doors_now_jumps_to_ten() {
    let c = content();
    let mut w = story_evening(&c, 2);
    ticks(&mut w, &c, 12 * QM);
    assert_eq!(w.clock_label(), "21:42");
    w.apply(&c, Command::OpenDoors).unwrap();
    assert!(
        w.drain_events()
            .contains(&SimEvent::DoorsOpened { early_minutes: 18 })
    );
    assert!(w.doors_open());
    assert_eq!(w.clock_label(), "22:00");
    assert_eq!(
        w.apply(&c, Command::OpenDoors),
        Err(CommandError::WrongPhase),
        "only before 22:00"
    );
    // everything keyed to 22:00 still happens: the first minute runs on the next tick
    ticks(&mut w, &c, 5 * TICKS_PER_MINUTE);
    assert_eq!(w.clock_label(), "22:05");
}

#[test]
fn a_save_in_the_preparation_continues_identically() {
    let c = content();
    let mut w = story_evening(&c, 4);
    ticks(&mut w, &c, 10 * QM + 3);
    assert_eq!(w.clock_label(), "21:40");
    let text = w.save_to_string().unwrap();
    let mut loaded = World::load_from_str(&text).unwrap();
    loaded.auto_doors = false;
    assert!(loaded.in_prep());
    assert_eq!(loaded.clock_label(), "21:40");
    for x in [&mut w, &mut loaded] {
        ticks(x, &c, 30 * QM);
        assert!(x.waiting_for_doors());
        x.apply(&c, Command::OpenDoors).unwrap();
        ticks(x, &c, 20 * TICKS_PER_MINUTE);
    }
    assert_eq!(w.clock_label(), "22:20");
    assert_eq!(w.save_to_string(), loaded.save_to_string());
}

#[test]
fn a_save_from_before_the_preparation_loads_with_the_doors_open() {
    let c = content();
    // a 0.1.0 (v2) save mid-night has no `prep_left`: it loads with the doors open
    let mut w = story_evening(&c, 3);
    w.skip_prep(&c);
    ticks(&mut w, &c, 20 * TICKS_PER_MINUTE);
    let text = w.save_to_string().unwrap();
    assert!(text.contains("prep_left:0"), "the field is saved");
    let old = text.replace("prep_left:0,", "");
    assert!(!old.contains("prep_left"));
    let mut loaded = World::load_from_str(&old).unwrap();
    assert!(loaded.doors_open());
    assert_eq!(loaded.clock_label(), "22:20");
    ticks(&mut loaded, &c, TICKS_PER_MINUTE);
    assert_eq!(loaded.clock_label(), "22:21");
    // and a v1 fixture still loads (its next night starts with the preparation)
    let v1 = include_str!("testdata/save_v1_story_day3.ron");
    let mut d = World::load_from_str(v1).unwrap();
    d.rebuild_caches(&c);
    bot::finish_cutscenes(&mut d, &c);
    d.start_night(&c);
    assert!(d.in_prep());
}

#[test]
fn the_bot_finishes_the_opening_ritual_in_the_preparation() {
    let c = content();
    let mut w = World::new_story_seeded(&c, Difficulty::Cozy, 4242);
    let mut b = bot::PlayerBot::new(bot::BotConfig {
        seed: 4242,
        ..Default::default()
    });
    let mut opened = 0;
    for _ in 0..6 {
        let ev = b.run_night(&mut w, &c);
        opened += ev
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    SimEvent::RitualComplete {
                        list: ChecklistKind::Opening
                    }
                )
            })
            .count();
    }
    assert_eq!(opened, 6, "every night is ready by 22:30");
}

/// Night 1: Mirek is the first guest at 22:04, his "so the light's on again" scene plays as he
/// walks in, and he doesn't walk in a second time at 23:00 (owner bug report, 0.1.1).
#[test]
fn mirek_comes_in_once_on_night_one_and_his_scene_plays_as_he_arrives() {
    let c = content();
    for tutorial_off in [true, false] {
        let mut w = World::new_story(&c, Difficulty::Cozy);
        w.sys.tutorial_off = tutorial_off;
        bot::finish_cutscenes(&mut w, &c);
        w.start_night(&c);
        bot::finish_cutscenes(&mut w, &c);
        w.drain_events();
        let mut mirek_parties = std::collections::BTreeSet::new();
        let mut scene_at = None;
        let mut guard = 0;
        while w.minutes_since_open() < 180 && guard < 200_000 {
            if w.waiting_for_doors() {
                w.apply(&c, Command::OpenDoors).unwrap();
            }
            w.tick(&c);
            if w.playing_cutscene() == Some("mirek_first") && scene_at.is_none() {
                scene_at = Some(w.minutes_since_open());
            }
            bot::finish_cutscenes(&mut w, &c);
            w.drain_events();
            for p in w.parties.iter().filter(|p| p.regular.as_deref() == Some("mirek")) {
                mirek_parties.insert(p.id);
            }
            guard += 1;
        }
        let at = scene_at.unwrap_or_else(|| panic!("mirek_first never played (tutorial_off {tutorial_off})"));
        // with the tutorial he is the first guest at 22:04, without it he comes at 23:00
        let when = if tutorial_off { 59..=61 } else { 3..=6 };
        assert!(when.contains(&at), "scene at {at} (tutorial_off {tutorial_off})");
        assert_eq!(mirek_parties.len(), 1, "Mirek came in {} times by 01:00", mirek_parties.len());
    }
}

/// 0.2.0 saves taken in the preparation counted its ticks at the night's rate (10 a minute):
/// they load at the same clock time and the clock then waits at 21:59 as now.
#[test]
fn a_preparation_save_from_0_2_0_keeps_its_clock_time() {
    let c = content();
    let mut w = story_evening(&c, 4);
    ticks(&mut w, &c, 10 * QM);
    assert_eq!(w.clock_label(), "21:40");
    let text = w.save_to_string().unwrap();
    assert!(text.contains(&format!("prep_left:{},", PREP_TICKS - 10 * QM)));
    // what 0.2.0 wrote at 21:40: 200 ticks left of 300, no `close_tick` / `flow`
    let old = text
        .replace(&format!("prep_left:{},", PREP_TICKS - 10 * QM), "prep_left:200,")
        .replace("close_tick:0,", "")
        .replace("flow:1,", "");
    assert!(!old.contains("flow:") && !old.contains("close_tick"));
    let mut loaded = World::load_from_str(&old).unwrap();
    loaded.auto_doors = false;
    assert!(loaded.in_prep());
    assert_eq!(loaded.clock_label(), "21:40");
    ticks(&mut loaded, &c, 25 * QM);
    assert!(loaded.waiting_for_doors(), "{}", loaded.clock_label());
}

/// The owner: "let the colleagues help with the opening too" — with a full crew the opening
/// list is done well before 21:59 (Karolina only does her own three items), so she can open
/// early.
#[test]
fn a_full_crew_finishes_the_opening_list_in_the_preparation() {
    use crate::content::handson::Duty;
    let c = content();
    let mut w = World::new_sandbox(&c, SandboxSetup::default());
    bot::finish_cutscenes(&mut w, &c);
    w.sys.tutorial_off = true;
    let ids: Vec<u32> = w.candidates().iter().map(|x| x.id).take(3).collect();
    for id in ids {
        w.apply(&c, Command::Hire { candidate: id }).unwrap();
    }
    let crew: Vec<Vec<Duty>> = vec![
        vec![Duty::Bar, Duty::Serving],
        vec![Duty::Kitchen, Duty::Register],
        vec![Duty::Floor, Duty::Dishes],
    ];
    for (s, d) in w.staff.iter_mut().filter(|s| !s.owner).zip(crew) {
        s.duties = d;
    }
    w.start_night(&c);
    bot::finish_cutscenes(&mut w, &c);
    w.drain_events();
    // (the whole crew on tonight's rozpis, punctual)
    for s in w.staff.iter_mut().filter(|s| !s.owner) {
        s.status = StaffStatus::Expected { minute: 0 };
        s.late_minutes = 0;
    }
    let k = w.karolina().unwrap().id;
    // her own items: the window, the specials board, a record
    w.apply(&c, Command::TickChecklist { id: "light_window".into() }).unwrap();
    let sp = w.specials().to_vec();
    w.apply(&c, Command::SetSpecials { recipes: sp }).unwrap();
    w.apply(&c, Command::TickChecklist { id: "record_on".into() }).unwrap();
    let mut done_at = None;
    let mut kitchen = false;
    for _ in 0..PREP_TICKS {
        w.tick(&c);
        for e in w.drain_events() {
            if let SimEvent::EquipmentUsed { by: Some(b), .. } = e
                && b != k
            {
                kitchen = true;
            }
        }
        if done_at.is_none() && super::tool::list_done(&w, &c, ChecklistKind::Opening) {
            done_at = Some(w.minutes_since_open());
            break;
        }
        if w.waiting_for_doors() {
            break;
        }
    }
    let at = done_at.expect("the opening list is done in the preparation");
    assert!(at <= -10, "the crew had the café ready by 21:50 ({at})");
    let by_crew = w
        .app()
        .checklist
        .iter()
        .filter(|s| s.done && s.by.is_some_and(|b| b != k))
        .count();
    assert!(by_crew >= 3, "the crew did {by_crew} opening items");
    assert!(kitchen, "the kitchen helper put the soup on / the milk out in the preparation");
    // she can open early; nobody was paid for the preparation
    assert!(w.in_prep());
    w.apply(&c, Command::OpenDoors).unwrap();
    assert!(w.staff().iter().all(|s| s.minutes_worked == 0));
}

/// An item Karolina queued herself is never taken by a helper holding its duty.
#[test]
fn helpers_never_take_an_item_karolina_queued() {
    use crate::content::handson::Duty;
    let c = content();
    let mut w = World::new_sandbox(&c, SandboxSetup::default());
    bot::finish_cutscenes(&mut w, &c);
    w.sys.tutorial_off = true;
    let id = w.candidates()[0].id;
    w.apply(&c, Command::Hire { candidate: id }).unwrap();
    for s in w.staff.iter_mut().filter(|s| !s.owner) {
        s.duties = vec![Duty::Bar, Duty::Register, Duty::Kitchen];
    }
    w.start_night(&c);
    bot::finish_cutscenes(&mut w, &c);
    w.drain_events();
    for s in w.staff.iter_mut().filter(|s| !s.owner) {
        s.status = StaffStatus::Expected { minute: 0 };
        s.late_minutes = 0;
    }
    let k = w.karolina().unwrap().id;
    // she queues the helper's kinds of items herself, before the helper is even in
    let mine = ["coffee_warmup", "count_float", "check_stock"];
    for id in mine {
        w.apply(&c, Command::TickChecklist { id: id.into() }).unwrap();
    }
    for _ in 0..PREP_TICKS {
        w.tick(&c);
        if w.waiting_for_doors() {
            break;
        }
    }
    for id in mine {
        let st = w.app().checklist.iter().find(|s| s.id == id).unwrap();
        assert!(st.done, "{id} done");
        assert_eq!(st.by, Some(k), "{id} is hers (she queued it)");
    }
}
