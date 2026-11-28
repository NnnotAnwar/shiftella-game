//! The Floor duty (`duty.floor`, "Mops, empties buckets…"): a helper who holds it notices the
//! dirt blots on the hall floor (`World::dirt_spots`), walks to one and mops it away, the
//! same way her other duties are picked up; a helper without it never mops.

use super::*;
use crate::content::Content;
use crate::content::handson::Duty;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

fn content() -> Content {
    Content::load(DATA).expect("content")
}

/// No random walk-ins tonight.
fn quiet(w: &mut World) {
    w.mods.push(Modifier {
        kind: ModKind::GuestRate,
        mul: 0.0,
        until: None,
        event: None,
    });
}

/// A quiet sandbox night with one helper on the floor holding exactly `duties` (no traits,
/// so no `AutoClean`), a clean café; returns the world and the helper's index.
fn night_with(c: &Content, duties: &[Duty]) -> (World, usize) {
    let mut w = World::new_sandbox(c, SandboxSetup::default());
    bot::finish_cutscenes(&mut w, c);
    w.sys.tutorial_off = true;
    let id = w.candidates().first().expect("a candidate").id;
    w.apply(c, Command::Hire { candidate: id }).unwrap();
    // the one just hired holds `duties`; anyone else on the books holds none
    let i = w.staff.iter().rposition(|s| !s.owner).expect("helper");
    for (k, s) in w.staff.iter_mut().enumerate().filter(|(_, s)| !s.owner) {
        s.duties = if k == i { duties.to_vec() } else { Vec::new() };
        s.traits.clear();
    }
    w.start_night(c);
    bot::finish_cutscenes(&mut w, c);
    w.skip_prep(c);
    quiet(&mut w);
    w.drain_events();
    w.staff[i].status = StaffStatus::Expected { minute: 0 };
    w.staff[i].late_minutes = 0;
    for _ in 0..600 {
        w.tick(c);
        bot::finish_cutscenes(&mut w, c);
        if w.staff[i].on_duty()
            && w.staff[i]
                .task
                .as_ref()
                .is_none_or(|t| matches!(t, Task::Idle { .. }))
        {
            break;
        }
    }
    assert!(w.staff[i].on_duty(), "the helper is on the floor");
    // a calm, clean café to start from
    w.dirt = 0.0;
    w.sink_dishes = 0;
    for p in w.furniture.iter_mut() {
        p.dishes = 0;
        p.dirt = 0.0;
    }
    (w, i)
}

fn mop_tile(w: &World, i: usize) -> Option<(i32, i32)> {
    match &w.staff[i].task {
        Some(Task::Mop { tile }) => Some(*tile),
        _ => None,
    }
}

#[test]
fn floor_helper_mops_a_lone_dirt_blot() {
    let c = content();
    let (mut w, i) = night_with(&c, &[Duty::Floor]);
    // a single blot: the player sees dirt on the floor
    w.dirt = 0.08;
    assert_eq!(w.dirt_spots().len(), 1);
    let mut mopped = false;
    for _ in 0..1200 {
        w.tick(&c);
        if mop_tile(&w, i).is_some() {
            mopped = true;
        }
        if w.dirt_spots().is_empty() {
            break;
        }
    }
    assert!(mopped, "she went to mop: {:?}", w.staff[i].task);
    assert!(
        w.dirt_spots().is_empty(),
        "the blot is gone (dirt {})",
        w.dirt
    );
}

#[test]
fn floor_helper_walks_to_the_dirt_and_cleans_the_whole_floor() {
    let c = content();
    let (mut w, i) = night_with(&c, &[Duty::Floor]);
    w.dirt = 0.35;
    assert_eq!(w.dirt_spots().len(), 4);
    let mut targets = Vec::new();
    let mut last: Option<(i32, i32)> = None;
    for _ in 0..4000 {
        let spots = w.dirt_spots();
        w.tick(&c);
        let t = mop_tile(&w, i);
        if let Some(t) = t.filter(|_| t != last) {
            assert!(
                spots.contains(&t),
                "she mops a blot ({t:?}), not a clean tile ({spots:?})"
            );
            targets.push(t);
        }
        last = t;
        if w.dirt_spots().is_empty() && mop_tile(&w, i).is_none() {
            break;
        }
    }
    assert!(targets.len() >= 4, "one trip per blot: {targets:?}");
    assert!(
        w.dirt_spots().is_empty(),
        "the floor is clean (dirt {})",
        w.dirt
    );
}

#[test]
fn floor_helper_still_does_her_other_duty() {
    let c = content();
    let (mut w, i) = night_with(&c, &[Duty::Dishes, Duty::Floor]);
    w.dirt = 0.25;
    w.sink_dishes = 6;
    let (mut washed, mut mopped) = (false, false);
    for _ in 0..4000 {
        w.tick(&c);
        washed |= matches!(w.staff[i].task, Some(Task::Wash { .. }));
        mopped |= mop_tile(&w, i).is_some();
        if w.sink_dishes == 0 && w.dirt_spots().is_empty() {
            break;
        }
    }
    assert!(washed && mopped, "washed {washed}, mopped {mopped}");
    assert_eq!(w.sink_dishes, 0, "the sink is empty");
    assert!(w.dirt_spots().is_empty(), "the floor is clean ({})", w.dirt);
}

#[test]
fn helper_without_the_floor_duty_never_mops() {
    let c = content();
    let (mut w, i) = night_with(&c, &[Duty::Dishes]);
    w.dirt = 0.35;
    for _ in 0..2000 {
        w.tick(&c);
        assert!(
            mop_tile(&w, i).is_none(),
            "no floor duty, no mop: {:?}",
            w.staff[i].task
        );
    }
    assert!(w.dirt >= 0.35 - 1e-4, "nobody mopped ({})", w.dirt);
}

/// Over a played night the floor helper never idles (a pastime, standing about) while a
/// blot is on the floor and nobody else is mopping.
#[test]
fn floor_helper_never_idles_beside_a_dirty_floor() {
    let c = content();
    let mut w = World::new_sandbox(
        &c,
        SandboxSetup {
            seed: 7,
            difficulty: Difficulty::Cozy,
            ..Default::default()
        },
    );
    bot::finish_cutscenes(&mut w, &c);
    let id = w.candidates().first().expect("a candidate").id;
    w.apply(&c, Command::Hire { candidate: id }).unwrap();
    let i = w.staff.iter().rposition(|s| !s.owner).expect("helper");
    for (k, s) in w.staff.iter_mut().enumerate().filter(|(_, s)| !s.owner) {
        s.duties = if k == i {
            vec![Duty::Dishes, Duty::Floor]
        } else {
            Vec::new()
        };
        s.traits.clear();
    }
    w.start_night(&c);
    bot::finish_cutscenes(&mut w, &c);
    w.skip_prep(&c);
    // she is on tonight's rozpis, the whole night, punctual
    w.staff[i].status = StaffStatus::Expected { minute: 0 };
    w.staff[i].shift = Shift::Full;
    w.staff[i].leave_at = NIGHT_MINUTES;
    w.staff[i].late_minutes = 0;
    let mut b = bot::PlayerBot::new(bot::BotConfig {
        hire_nights: Vec::new(),
        seed: 7,
        ..Default::default()
    });
    let (mut idle_dirty, mut run) = (0u32, 0u32);
    let mut worst = 0u32;
    while w.phase() == Phase::Night && !w.in_closing() {
        b.step(&mut w, &c);
        w.tick(&c);
        let s = &w.staff[i];
        let idle =
            s.on_duty() && s.path.is_empty() && matches!(s.task, None | Some(Task::Idle { .. }));
        if idle && !w.dirt_spots().is_empty() && w.mop_claim.is_none() {
            idle_dirty += 1;
            run += 1;
            worst = worst.max(run);
        } else {
            run = 0;
        }
    }
    assert!(
        worst <= 4,
        "she stood idle beside a dirty floor for {worst} ticks in a row ({idle_dirty} in all)"
    );
}
