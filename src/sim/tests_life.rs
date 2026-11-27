//! Staff idle life (`sim::life`): pastimes start only when the duties have nothing to do,
//! end the moment work appears, never touch the world RNG, never lower the helpers' real
//! work, and survive saves.

use super::life::{self, OFF, Pastime};
use super::*;
use crate::content::Content;
use crate::content::handson::Duty;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

fn content() -> Content {
    Content::load(DATA).expect("content")
}

/// A sandbox night with two helpers on the floor, played by the bot up to `minutes`.
fn night_with_helpers(c: &Content, seed0: u64, minutes: u32) -> (World, bot::PlayerBot) {
    for seed in seed0..seed0 + 40 {
        let mut w = World::new_sandbox(
            c,
            SandboxSetup {
                seed,
                difficulty: Difficulty::Cozy,
                ..Default::default()
            },
        );
        let ids: Vec<u32> = w.candidates().iter().map(|x| x.id).take(2).collect();
        if ids.len() < 2 {
            continue;
        }
        for id in ids {
            w.apply(c, Command::Hire { candidate: id }).unwrap();
        }
        w.start_night(c);
        w.skip_prep(c);
        let mut b = bot::PlayerBot::new(bot::BotConfig {
            hire_nights: Vec::new(),
            seed,
            ..Default::default()
        });
        while w.minute() < minutes && w.phase() == Phase::Night {
            b.step(&mut w, c);
            w.tick(c);
        }
        let _ = w.drain_events();
        if w.staff().iter().filter(|s| !s.owner && s.on_duty()).count() == 2 {
            return (w, b);
        }
    }
    panic!("no seed with both helpers on the floor");
}

fn helpers(w: &World) -> Vec<usize> {
    (0..w.staff.len())
        .filter(|&i| !w.staff[i].owner && w.staff[i].on_duty())
        .collect()
}

/// Ticks until some helper is at a pastime (they come after a short pause).
fn until_pastime(w: &mut World, c: &Content, b: &mut bot::PlayerBot, max: u32) -> Option<usize> {
    for _ in 0..max {
        b.step(w, c);
        w.tick(c);
        if let Some(i) = helpers(w).into_iter().find(|&i| w.staff[i].life.is_some()) {
            return Some(i);
        }
    }
    None
}

#[test]
fn pastimes_only_when_nothing_to_do_and_dropped_at_once() {
    let c = content();
    let (mut w, mut b) = night_with_helpers(&c, 1, 60);
    let mut seen = std::collections::BTreeSet::new();
    // how long a helper keeps a pastime while their duties offer a task
    let mut overdue = vec![0u32; w.staff.len()];
    for _ in 0..(300 * TICKS_PER_MINUTE) {
        if w.phase() != Phase::Night {
            break;
        }
        b.step(&mut w, &c);
        w.tick(&c);
        for i in helpers(&w) {
            let s = &w.staff[i];
            if let Some(l) = &s.life {
                seen.insert(l.kind);
                assert!(
                    matches!(s.task, Some(Task::Idle { .. })),
                    "a pastime only in Task::Idle: {:?}",
                    s.task
                );
                if staff::peek_work(&w, &c, i).is_some() {
                    overdue[i] += 1;
                    assert!(
                        overdue[i] <= 3,
                        "{} keeps {:?} while there is work",
                        s.name,
                        l.kind
                    );
                } else {
                    overdue[i] = 0;
                }
            } else {
                overdue[i] = 0;
            }
        }
    }
    assert!(seen.len() >= 3, "a few different pastimes in a night: {seen:?}");
}

#[test]
fn dishes_and_dirt_interrupt_a_pastime_within_a_few_ticks() {
    let c = content();
    for (duty, kick) in [(Duty::Dishes, 0), (Duty::Floor, 1)] {
        let (mut w, mut b) = night_with_helpers(&c, 3, 200);
        // one helper holds only this duty, the other none (nobody else takes it)
        let hs = helpers(&w);
        let (me, other) = (hs[0], hs[1]);
        for (i, on) in [(me, Some(duty)), (other, None)] {
            let id = w.staff[i].id;
            for d in w.staff[i].duties.clone() {
                w.apply(&c, Command::SetDuty { staff: id, duty: d, on: false })
                    .unwrap();
            }
            if let Some(d) = on {
                w.apply(&c, Command::SetDuty { staff: id, duty: d, on: true })
                    .unwrap();
            }
        }
        // a calm café: nothing to wash, a clean floor
        w.sink_dishes = 0;
        w.dirt = 0.0;
        for p in w.furniture.iter_mut() {
            p.dishes = 0;
        }
        let mut got = false;
        for _ in 0..2000 {
            b.step(&mut w, &c);
            w.tick(&c);
            w.sink_dishes = 0;
            w.dirt = 0.0;
            for p in w.furniture.iter_mut() {
                p.dishes = 0;
            }
            if w.staff[me].life.is_some() && w.staff[me].path.is_empty() {
                got = true;
                break;
            }
        }
        assert!(got, "{duty:?}: the helper found a pastime");
        // work appears
        if kick == 0 {
            w.sink_dishes = 6;
        } else {
            w.dirt = 0.9;
        }
        let mut picked = None;
        for t in 1..=4 {
            w.tick(&c);
            if !matches!(w.staff[me].task, Some(Task::Idle { .. }) | None) {
                picked = Some(t);
                break;
            }
        }
        let t = picked.unwrap_or_else(|| panic!("{duty:?}: work not picked up: {:?}", w.staff[me]));
        assert!(t <= 3, "{duty:?}: picked up after {t} ticks");
        assert!(w.staff[me].life.is_none(), "the pastime is dropped");
        match duty {
            Duty::Dishes => assert!(matches!(w.staff[me].task, Some(Task::Wash { .. }))),
            _ => assert!(matches!(w.staff[me].task, Some(Task::Mop { .. }))),
        }
    }
}

#[test]
fn pastimes_never_touch_the_world_rng() {
    let c = content();
    let (mut w, mut b) = night_with_helpers(&c, 5, 120);
    let mut steps = 0;
    for _ in 0..3000 {
        b.step(&mut w, &c);
        w.tick(&c);
        for i in helpers(&w) {
            if matches!(w.staff[i].task, Some(Task::Idle { .. })) && w.staff[i].path.is_empty() {
                let before = w.rng.clone();
                life::step(&mut w, &c, i);
                assert_eq!(before, w.rng, "life::step rolled the world RNG");
                steps += 1;
            }
        }
    }
    assert!(steps > 100, "idle helpers were stepped ({steps})");
}

/// Real-work ticks of the helpers and served items over whole bot nights.
fn work_over_nights(c: &Content, off: bool, seed: u64, nights: u32) -> (u64, u32, u32, u32) {
    OFF.with(|x| x.set(off));
    let mut w = World::new_sandbox(
        c,
        SandboxSetup {
            seed,
            difficulty: Difficulty::Cozy,
            ..Default::default()
        },
    );
    let ids: Vec<u32> = w.candidates().iter().map(|x| x.id).take(2).collect();
    for id in ids {
        w.apply(c, Command::Hire { candidate: id }).unwrap();
    }
    let mut b = bot::PlayerBot::new(bot::BotConfig {
        hire_nights: Vec::new(),
        seed,
        ..Default::default()
    });
    let (mut busy, mut served, mut parties) = (0u64, 0u32, 0u32);
    for _ in 0..nights {
        b.day(&mut w, c);
        w.start_night(c);
        let mut guard = 0;
        while w.phase() == Phase::Night && guard < 20_000 {
            guard += 1;
            b.step(&mut w, c);
            w.tick(c);
            if let Some(id) = w.playing_cutscene().map(str::to_string) {
                let _ = w.apply(
                    c,
                    Command::CutsceneFinished {
                        id,
                        choices: Vec::new(),
                    },
                );
            }
            busy += w
                .staff
                .iter()
                .filter(|s| {
                    !s.owner
                        && s.on_duty()
                        && !matches!(
                            s.task,
                            None | Some(Task::Idle { .. } | Task::Break { .. } | Task::Leave)
                        )
                })
                .count() as u64;
        }
        if let Some(s) = w.last_summary() {
            served += s.orders_served;
            parties += s.guests_served;
        }
        let _ = w.drain_events();
    }
    OFF.with(|x| x.set(false));
    let done = w.staff.iter().filter(|s| !s.owner).map(|s| s.tasks_done).sum();
    (busy, served, parties, done)
}

/// Same seeds with and without pastimes: the helpers finish as many tasks and the café
/// serves as many items. (A single seed drifts ±20 % either way — a helper standing one tile
/// elsewhere picks another ticket first and the night unfolds differently — so the check is
/// on the sum of 8 seeds, within that noise.)
#[test]
fn helpers_do_no_less_real_work_with_pastimes() {
    let c = content();
    let (mut on, mut off) = ((0u64, 0u32, 0u32), (0u64, 0u32, 0u32));
    for seed in 11..19 {
        let a = work_over_nights(&c, true, seed, 3);
        let b = work_over_nights(&c, false, seed, 3);
        eprintln!("seed {seed}: off {a:?} on {b:?}");
        off = (off.0 + a.0, off.1 + a.1, off.2 + a.3);
        on = (on.0 + b.0, on.1 + b.1, on.2 + b.3);
    }
    eprintln!("real-work ticks / served / helper tasks done: off {off:?} → on {on:?}");
    assert!(on.2 as f64 >= off.2 as f64 * 0.95, "tasks done {on:?} vs {off:?}");
    assert!(on.1 as f64 >= off.1 as f64 * 0.95, "served {on:?} vs {off:?}");
}

#[test]
fn a_pastime_survives_a_save() {
    let c = content();
    let (mut w, mut b) = night_with_helpers(&c, 7, 120);
    let i = until_pastime(&mut w, &c, &mut b, 3000).expect("a pastime");
    assert!(w.staff[i].life.is_some());
    let text = w.save_to_string().unwrap();
    let mut back = World::load_from_str(&text).unwrap();
    back.rebuild_caches(&c);
    w.rebuild_caches(&c);
    assert_eq!(back.staff[i].life, w.staff[i].life);
    for _ in 0..400 {
        w.tick(&c);
        back.tick(&c);
    }
    assert_eq!(w.save_to_string().unwrap(), back.save_to_string().unwrap());
}

#[test]
fn the_card_says_what_they_do() {
    let c = content();
    let (mut w, mut b) = night_with_helpers(&c, 9, 120);
    let mut chat = false;
    let mut said = false;
    for _ in 0..6000 {
        b.step(&mut w, &c);
        w.tick(&c);
        for i in helpers(&w) {
            let s = &w.staff[i];
            let Some(l) = s.life.clone() else { continue };
            if s.duties.is_empty() {
                continue;
            }
            match w.activity(s.id).unwrap() {
                Activity::Pastime { kind, with } => {
                    assert_eq!(kind, l.kind);
                    said = true;
                    if kind == Pastime::Chat && with.is_some() {
                        chat = true;
                    }
                }
                a => panic!("{a:?} for a pastime"),
            }
        }
        if said && chat {
            break;
        }
    }
    assert!(said, "a helper's card shows a pastime");
    let _ = chat;
}

#[test]
fn rings_show_tasks_and_pastimes_but_never_karolina() {
    let c = content();
    let (mut w, mut b) = night_with_helpers(&c, 1, 60);
    let (mut task, mut pastime) = (false, false);
    let k = w.karolina().unwrap().id;
    for _ in 0..4000 {
        b.step(&mut w, &c);
        w.tick(&c);
        for hw in w.helper_work() {
            assert_ne!(hw.staff, k);
            assert!((0.0..=1.0).contains(&hw.progress));
            match hw.doing {
                HelperDoing::Task { .. } => task = true,
                HelperDoing::Pastime(_) => pastime = true,
            }
        }
        if task && pastime {
            break;
        }
    }
    assert!(task && pastime, "task ring {task}, pastime ring {pastime}");
}
