//! 0.2.7 scene fixes on the sim side: a regular's walking-in line is varied (never the same
//! twice in a row, a new arc-stage line first) and a scene hands its regulars back sitting or
//! walking — never standing on a chair.

use super::*;
use crate::content::Content;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

fn content() -> Content {
    Content::load(DATA).unwrap_or_else(|e| panic!("{e}"))
}

/// Story night `n`, doors open, `minutes` after 22:00, nobody walking in by chance.
fn night_at(c: &Content, n: u32, minutes: u32) -> World {
    let mut w = World::new_story(c, Difficulty::Cozy);
    w.sys.tutorial_off = true;
    if n > 1 {
        w.night = n;
        w.begin_day(c);
    }
    bot::finish_cutscenes(&mut w, c);
    w.start_night(c);
    bot::finish_cutscenes(&mut w, c);
    w.skip_prep(c);
    w.mods.push(Modifier {
        kind: ModKind::GuestRate,
        mul: 0.0,
        until: None,
        event: None,
    });
    for _ in 0..minutes * 10 {
        w.tick(c);
        bot::finish_cutscenes(&mut w, c);
    }
    w.drain_events();
    w
}

fn arrival_lines(w: &mut World, c: &Content, regular: &str, visits: usize) -> Vec<String> {
    (0..visits)
        .map(|_| {
            story::regular_line(w, c, regular, 0);
            w.drain_events()
                .into_iter()
                .find_map(|e| match e {
                    SimEvent::Speech { key, .. } => Some(key),
                    _ => None,
                })
                .expect("a line on walking in")
        })
        .collect()
}

/// "Every time the regulars come in with one and the same phrase": a stage with one line
/// said it every visit. Now the new stage line comes first, then it takes turns with the
/// regular's everyday hellos, never twice in a row.
#[test]
fn a_regular_never_greets_with_the_same_line_twice_in_a_row() {
    let c = content();
    for r in c.regulars.iter().filter(|r| !r.greetings.is_empty()) {
        for stage in r.stages.iter().filter(|s| !s.lines.is_empty()) {
            let mut w = World::new_story(&c, Difficulty::Cozy);
            w.story.counters.insert(r.arc_counter(), stage.stage as i32);
            let lines = arrival_lines(&mut w, &c, &r.id, 16);
            let ctx = format!("{} stage {}", r.id, stage.stage);
            for pair in lines.windows(2) {
                assert_ne!(pair[0], pair[1], "{ctx}: the same line twice in a row");
            }
            // the stage's own lines are all said before any hello
            let n = stage.lines.len();
            let mut first: Vec<&String> = lines.iter().take(n).collect();
            first.sort();
            let mut want: Vec<&String> = stage.lines.iter().collect();
            want.sort();
            assert_eq!(first, want, "{ctx}: the stage's lines first");
            let distinct: std::collections::BTreeSet<&String> = lines.iter().collect();
            assert!(distinct.len() >= 4, "{ctx}: varied ({distinct:?})");
            for l in &lines {
                assert!(
                    stage.lines.contains(l) || r.greetings.contains(l),
                    "{ctx}: {l} isn't theirs"
                );
            }
        }
    }
}

/// The lines go on from where they were after a save (the last one is remembered).
#[test]
fn the_last_greeting_is_remembered_across_a_save() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    let first = arrival_lines(&mut w, &c, "vera", 3);
    let saved = w.save_to_string().expect("save");
    let mut w2 = World::load_from_str(&saved).expect("load");
    let next = arrival_lines(&mut w2, &c, "vera", 1);
    assert_ne!(first.last(), next.first());
}

/// A scene that sat a regular down on a chair (or left them beside a table) hands them back:
/// with somewhere to go they get up and go; with nowhere to go they stand on the floor beside
/// it — never on the chair.
#[test]
fn a_scene_never_hands_a_regular_back_standing_on_a_chair() {
    let c = content();
    let mut w = night_at(&c, 5, 150);
    let pid = guests::spawn_party(
        &mut w,
        &c,
        "students",
        1,
        Some("vera"),
        1.0,
        1.0,
        &[],
        None,
        true,
    )
    .expect("Vera comes in");
    let leader = w
        .parties
        .iter()
        .find(|p| p.id == pid)
        .and_then(|p| p.leader())
        .expect("leader");
    // a free chair of grandma's corner table, nobody on it
    let chair = guests::seats(&w, &c)
        .into_iter()
        .find(|s| s.free && s.tile.1 == 8 && s.tile.0 >= 11)
        .map(|s| s.tile)
        .expect("a free chair at the corner table");
    assert!(w.grid.is_seat(chair.0, chair.1));
    // nowhere to go (a scene played her in before her party chose a seat)
    if let Some(g) = w.guests.iter_mut().find(|g| g.id == leader) {
        g.goal = None;
        g.path.clear();
    }
    guests::scene_handoff(&mut w, "vera", chair);
    let g = w.guests.iter().find(|g| g.id == leader).expect("guest");
    let t = g.tile();
    assert!(
        w.grid.walkable(t.0, t.1) && !w.grid.is_seat(t.0, t.1),
        "stands on the floor, not on the chair ({t:?})"
    );
    assert!(
        (t.0 - chair.0).abs() <= 1 && (t.1 - chair.1).abs() <= 1,
        "beside it"
    );
    // somewhere to go: she gets up and walks there
    let goal = (2, 12);
    if let Some(g) = w.guests.iter_mut().find(|g| g.id == leader) {
        g.goal = Some(goal);
    }
    guests::scene_handoff(&mut w, "vera", chair);
    let g = w.guests.iter().find(|g| g.id == leader).expect("guest");
    assert_eq!(g.pose, Pose::Walk);
    assert_eq!(g.goal, Some(goal));
}
