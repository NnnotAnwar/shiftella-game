//! Build-mode tests (design/v0.2.md §A): dragging a table with its chairs, one undo step,
//! pieces in use stay put, the coziness preview of a held piece, the build-wishes stub.

use super::grid::Room;
use super::*;
use crate::content::Content;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

fn content() -> Content {
    Content::load(DATA).unwrap_or_else(|e| panic!("{e}"))
}

fn first(w: &World, def: &str) -> u32 {
    w.furniture().iter().find(|p| p.def == def).unwrap().uid
}

fn pos(w: &World, uid: u32) -> (i32, i32, u8) {
    let p = w.furniture_by_uid(uid).unwrap();
    (p.x, p.y, p.rot)
}

fn place(def: &str, x: i32, y: i32) -> Command {
    Command::PlaceFurniture {
        def: def.into(),
        x,
        y,
        rot: 0,
    }
}

/// The first hall spot where the group of `uid` (turned `turns`) can go, away from its tile.
fn free_spot(w: &World, c: &Content, uid: u32, turns: u8) -> Vec<PieceMove> {
    let (x0, y0, _) = pos(w, uid);
    for y in 0..w.grid().height {
        for x in 0..w.grid().width {
            if (x - x0).abs() + (y - y0).abs() < 3 || w.grid().room(x, y) != Room::Hall {
                continue;
            }
            let moves = w.group_moves(c, uid, (x, y), turns);
            if w.can_move_pieces(c, &moves).is_ok() {
                return moves;
            }
        }
    }
    panic!("no free spot for the group");
}

#[test]
fn a_table_brings_its_chairs() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    let table = first(&w, "table_round");
    let group = w.move_group(&c, table);
    assert_eq!(group[0], table);
    assert!(
        group.len() >= 2,
        "the default tables have chairs: {group:?}"
    );
    for uid in &group[1..] {
        let p = w.furniture_by_uid(*uid).unwrap();
        assert!(c.furniture_def(&p.def).unwrap().seats > 0);
    }
    // a chair alone is just itself
    assert_eq!(w.move_group(&c, group[1]), vec![group[1]]);

    let before: Vec<_> = group.iter().map(|u| pos(&w, *u)).collect();
    let money = w.money();
    let moves = free_spot(&w, &c, table, 0);
    w.apply(
        &c,
        Command::MoveGroup {
            moves: moves.clone(),
        },
    )
    .unwrap();
    assert_eq!(w.money(), money, "moving is free");
    // everyone moved by the same offset and still belongs to the table
    let (dx, dy) = (moves[0].x - before[0].0, moves[0].y - before[0].1);
    for (u, b) in group.iter().zip(&before) {
        assert_eq!(pos(&w, *u), (b.0 + dx, b.1 + dy, b.2));
    }
    let mut again = w.move_group(&c, table);
    let mut want = group.clone();
    again.sort();
    want.sort();
    assert_eq!(again, want, "the chairs still face their table");
    // one Ctrl+Z puts the whole group back
    w.undo(&c).unwrap();
    for (u, b) in group.iter().zip(&before) {
        assert_eq!(pos(&w, *u), *b);
    }
    assert_eq!(w.grid().object_at(before[0].0, before[0].1), Some(table));
}

#[test]
fn a_turned_table_turns_its_chairs_around_it() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    let table = first(&w, "table_round");
    let group = w.move_group(&c, table);
    let (tx, ty, _) = pos(&w, table);
    let offsets: Vec<_> = group[1..]
        .iter()
        .map(|u| {
            let (x, y, _) = pos(&w, *u);
            (x - tx, y - ty)
        })
        .collect();
    let moves = free_spot(&w, &c, table, 1);
    w.apply(&c, Command::MoveGroup { moves }).unwrap();
    let (nx, ny, _) = pos(&w, table);
    for (u, (dx, dy)) in group[1..].iter().zip(&offsets) {
        let (x, y, _) = pos(&w, *u);
        // a quarter turn: (dx, dy) → (−dy, dx)
        assert_eq!((x - nx, y - ny), (-dy, *dx));
    }
    assert_eq!(w.move_group(&c, table).len(), group.len());
}

#[test]
fn pieces_in_use_stay_put_and_failed_moves_change_nothing() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    let table = first(&w, "table_round");
    let group = w.move_group(&c, table);
    let chair = group[1];
    let moves = free_spot(&w, &c, table, 0);
    let before: Vec<_> = group.iter().map(|u| pos(&w, *u)).collect();
    // a guest sits on one of the chairs
    w.furniture
        .iter_mut()
        .find(|p| p.uid == chair)
        .unwrap()
        .seat_by[0] = Some(99);
    assert_eq!(w.in_use_reason(chair), Some(InUseReason::Seated));
    assert_eq!(w.in_use_reason(table), None);
    assert_eq!(
        w.apply(
            &c,
            Command::MoveGroup {
                moves: moves.clone()
            }
        ),
        Err(CommandError::InUse)
    );
    let now: Vec<_> = group.iter().map(|u| pos(&w, *u)).collect();
    assert_eq!(now, before);
    w.furniture
        .iter_mut()
        .find(|p| p.uid == chair)
        .unwrap()
        .seat_by[0] = None;
    // onto another table: refused, nothing moved, the grid is intact
    let other = w
        .furniture()
        .iter()
        .find(|p| p.def == "table_round" && p.uid != table)
        .unwrap()
        .clone();
    let onto = w.group_moves(&c, table, (other.x, other.y), 0);
    assert_eq!(
        w.apply(&c, Command::MoveGroup { moves: onto }),
        Err(CommandError::Occupied)
    );
    let now: Vec<_> = group.iter().map(|u| pos(&w, *u)).collect();
    assert_eq!(now, before);
    assert_eq!(w.grid().object_at(other.x, other.y), Some(other.uid));
    for u in &group {
        let (x, y, _) = pos(&w, *u);
        assert_eq!(w.grid().object_at(x, y), Some(*u));
    }
    assert!(!w.can_undo(), "a refused move leaves no undo step");
    // a helper at a station: busy
    let machine = first(&w, "coffee_machine");
    w.furniture
        .iter_mut()
        .find(|p| p.uid == machine)
        .unwrap()
        .user = Some(1);
    assert_eq!(w.in_use_reason(machine), Some(InUseReason::Busy));
}

#[test]
fn coziness_preview_matches_the_real_change() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    let cozy = |w: &World| atmos::compute(w, &c).value;
    // a chair in a dark hall corner, far from every lamp: the hall's lit share drops
    let mut dark = None;
    'find: for y in 0..w.grid().height {
        for x in 0..w.grid().width {
            if w.grid().room(x, y) != Room::Hall {
                continue;
            }
            let p = w.coziness_preview(&c, "chair", x, y, 0);
            if p < -0.05 && w.clone().apply(&c, place("chair", x, y)).is_ok() {
                dark = Some((x, y, p));
                break 'find;
            }
        }
    }
    let (x, y, p) = dark.expect("a dark corner for a chair");
    let before = cozy(&w);
    w.apply(&c, place("chair", x, y)).unwrap();
    assert!(
        (cozy(&w) - before - p).abs() < 1e-3,
        "preview {p} = real change"
    );
    // a floor lamp right beside it brings the light back
    let (lx, ly) = [(1, 0), (-1, 0), (0, 1), (0, -1)]
        .iter()
        .map(|(dx, dy)| (x + dx, y + dy))
        .find(|(lx, ly)| w.clone().apply(&c, place("floor_lamp", *lx, *ly)).is_ok())
        .expect("room for a lamp");
    let up = w.coziness_preview(&c, "floor_lamp", lx, ly, 0);
    assert!(up > 0.0, "a lamp by the dark chair helps: {up}");
    let before = cozy(&w);
    w.apply(&c, place("floor_lamp", lx, ly)).unwrap();
    assert!((cozy(&w) - before - up).abs() < 1e-3);
    // floors don't change coziness; the preview never mutates the world
    assert_eq!(w.coziness_preview(&c, "floor_carpet", lx, ly, 0), 0.0);
    let n = w.furniture().len();
    let _ = w.coziness_preview(&c, "plant_tall", 3, 3, 0);
    assert_eq!(w.furniture().len(), n);
}

#[test]
fn coziness_preview_of_a_drag() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    let table = first(&w, "table_round");
    let moves = free_spot(&w, &c, table, 0);
    let p = w.coziness_preview_moves(&c, &moves);
    let before = atmos::compute(&w, &c).value;
    w.apply(&c, Command::MoveGroup { moves }).unwrap();
    let after = atmos::compute(&w, &c).value;
    assert!((after - before - p).abs() < 1e-3, "drag preview {p}");
}

#[test]
fn build_wishes_stub_lists_the_story_facts() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    assert!(w.build_wishes(&c).is_empty());
    w.story.flags.insert("lori_invited".into());
    assert!(
        w.build_wishes(&c)
            .iter()
            .any(|b| b.who == "lori" && b.tag.as_deref() == Some("stage"))
    );
    // fulfilled → gone
    let spot = (0..w.grid().height)
        .flat_map(|y| (0..w.grid().width).map(move |x| (x, y)))
        .find(|&(x, y)| {
            w.grid().room(x, y) == Room::Hall
                && w.clone().apply(&c, place("stage_corner", x, y)).is_ok()
        })
        .expect("room for a stage");
    w.apply(&c, place("stage_corner", spot.0, spot.1)).unwrap();
    assert!(!w.build_wishes(&c).iter().any(|b| b.who == "lori"));
}
