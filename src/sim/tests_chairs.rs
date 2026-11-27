//! Chairs and standing spots (0.2.3, chaos finds of the 0.2.1 QA run): nobody stands on,
//! walks across or sits down on a chair someone else is using, and no act sends anyone into
//! a piece. One test per cause.

use super::guests;
use super::*;
use crate::content::Content;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

fn content() -> Content {
    Content::load(DATA).unwrap_or_else(|e| panic!("{e}"))
}

/// A story night `n` running, cutscenes done, no random walk-ins.
fn night(c: &Content, n: u32) -> World {
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
    w.econ.money = 1_000_000;
    w.drain_events();
    w
}

/// A party of one that sits down at a table (a wish party always sits).
fn sitter(w: &mut World, c: &Content) -> u32 {
    let pid =
        guests::spawn_party(w, c, "students", 1, None, 1.0, 1.0, &[], None, true).expect("spawned");
    w.parties.iter().find(|p| p.id == pid).unwrap().members[0]
}

fn place(w: &mut World, c: &Content, def: &str, t: (i32, i32)) -> Result<(), CommandError> {
    w.apply(
        c,
        Command::PlaceFurniture {
            def: def.into(),
            x: t.0,
            y: t.1,
            rot: 0,
        },
    )
}

fn guest(w: &World, id: u32) -> &Guest {
    w.guests.iter().find(|g| g.id == id).unwrap()
}

fn karolina(w: &World) -> usize {
    player::karolina_idx(w).unwrap()
}

/// Cause 1: a guest sat down on whoever stood on their chair (a helper idling on it,
/// Karolina crossing it, the guest who had just got up). They wait on it until it's free.
#[test]
fn nobody_sits_down_on_someone_standing_on_the_chair() {
    let c = content();
    let mut w = night(&c, 3);
    let gid = sitter(&mut w, &c);
    let seat = staff::guest_seat_tile(&w, gid).expect("a seat");
    let k = karolina(&w);
    let hold = |w: &mut World| {
        w.staff[k].pos = grid::tile_center(seat);
        w.staff[k].path.clear();
    };
    let mut there = 0;
    for _ in 0..1500 {
        hold(&mut w);
        w.tick(&c);
        assert_ne!(guest(&w, gid).pose, Pose::Sit, "sat down on Karolina");
        if guest(&w, gid).tile() == seat {
            there += 1;
            if there > 30 {
                break;
            }
        }
    }
    assert!(there > 30, "the guest reached the chair and waited on it");
    // she steps off: they sit
    let off = staff::stand_next_to(&w, seat, seat).expect("a spot beside");
    w.staff[k].pos = grid::tile_center(off);
    for _ in 0..3 {
        w.tick(&c);
    }
    assert_eq!(guest(&w, gid).pose, Pose::Sit);
    assert!(w.grid.is_taken(seat.0, seat.1));
}

/// Cause 2: a way planned over a free chair still led over it once a guest sat there (and
/// the walk masks saw the guest a tick late). The walker now goes around at once.
#[test]
fn a_way_over_a_chair_goes_around_once_someone_sits_on_it() {
    let c = content();
    let mut w = night(&c, 3);
    let gid = sitter(&mut w, &c);
    let seat = staff::guest_seat_tile(&w, gid).expect("a seat");
    // a straight line across the chair (floor on both sides, a way around it), Karolina
    // two steps before it (she walks 0.75 tiles a tick)
    let (a0, a, b) = [(0, -1), (0, 1), (-1, 0), (1, 0)]
        .iter()
        .map(|d| {
            let at = |k: i32| (seat.0 + k * d.0, seat.1 + k * d.1);
            (at(2), at(1), at(-1))
        })
        .find(|(a0, a, b)| {
            [a0, a, b]
                .iter()
                .all(|t| w.grid.walkable(t.0, t.1) && !w.grid.is_seat(t.0, t.1))
        })
        .expect("floor on two sides of the chair");
    // the guest arrives on the chair this tick; Karolina is a step away, about to cross it
    let g = w.guests.iter_mut().find(|g| g.id == gid).unwrap();
    g.pos = grid::tile_center(seat);
    g.path.clear();
    g.goal = Some(seat);
    g.pose = Pose::Idle;
    let k = karolina(&w);
    w.staff[k].pos = grid::tile_center(a0);
    w.staff[k].path = vec![a, seat, b];
    w.tick(&c);
    assert_eq!(guest(&w, gid).pose, Pose::Sit);
    assert!(
        w.grid.is_taken(seat.0, seat.1),
        "the chair is taken at once"
    );
    assert!(
        !w.staff[k].path.contains(&seat),
        "her way still crosses the chair: {:?}",
        w.staff[k].path
    );
    assert_eq!(w.staff[k].path.last(), Some(&b));
    for _ in 0..60 {
        w.tick(&c);
        assert_ne!(
            w.staff[k].tile(),
            seat,
            "she walked across the seated guest"
        );
    }
}

/// Cause 3: a helper's idle (and break) spot could be a stool built on the staff floor —
/// a guest then sat down on her.
#[test]
fn helpers_never_idle_or_rest_on_a_stool_built_on_the_staff_floor() {
    let c = content();
    let mut w = night(&c, 3);
    let crew = w.staff.len();
    let spots_of = |w: &World| -> Vec<(i32, i32)> {
        (0..crew)
            .map(|i| staff::idle_tile(w, &c, i))
            .chain((0..8).map(|i| staff::break_tile(w, i)))
            .collect()
    };
    let mut spots = spots_of(&w);
    spots.extend([(27, 10), (28, 10)]);
    spots.sort_unstable();
    spots.dedup();
    let mut stools = 0;
    for t in spots {
        if w.grid.room(t.0, t.1).staff_only() && place(&mut w, &c, "stool", t).is_ok() {
            stools += 1;
        }
    }
    assert!(stools >= 2, "stools on the staff floor ({stools})");
    for t in spots_of(&w) {
        assert!(
            !w.grid.is_seat(t.0, t.1),
            "a helper's spot on the stool at {t:?}"
        );
    }
}

/// Cause 4: a station's access tile could be a chair somebody sits on (a table with its
/// chairs all round: the helper clearing it stood on a seated guest).
#[test]
fn a_table_is_never_worked_from_a_taken_chair() {
    let c = content();
    let mut w = night(&c, 3);
    // a free hall spot with room for a table and four chairs round it
    let ok = |w: &World, t: (i32, i32)| {
        let mut w = w.clone();
        place(&mut w, &c, "table_round", t).is_ok()
            && [(0, -1), (0, 1), (-1, 0), (1, 0)]
                .iter()
                .all(|(dx, dy)| place(&mut w, &c, "chair", (t.0 + dx, t.1 + dy)).is_ok())
    };
    let t = (2..26)
        .flat_map(|x| (2..8).map(move |y| (x, y)))
        .find(|&t| ok(&w, t))
        .expect("room for a table");
    place(&mut w, &c, "table_round", t).unwrap();
    let chairs: Vec<(i32, i32)> = [(0, -1), (0, 1), (-1, 0), (1, 0)]
        .iter()
        .map(|(dx, dy)| (t.0 + dx, t.1 + dy))
        .collect();
    for &ch in &chairs {
        place(&mut w, &c, "chair", ch).unwrap();
    }
    let uid = w.grid.object_at(t.0, t.1).unwrap();
    for n in 1..=4 {
        w.grid.set_taken(chairs[..n].to_vec());
        let a = staff::station_access(&w, &c, uid).expect("a spot");
        assert!(
            !w.grid.is_taken(a.0, a.1),
            "{n} taken: works from a taken chair"
        );
        assert!(w.grid.walkable(a.0, a.1) && grid::chebyshev(a, t) == 1);
    }
    // all four taken: a corner beside it, never a chair
    let a = staff::station_access(&w, &c, uid).unwrap();
    assert!(!w.grid.is_seat(a.0, a.1));
}

/// Cause 4b: the masks know which floor is reached without crossing a chair (a nook behind
/// a chair isn't): standing spots, the line and the to-go spots prefer those.
#[test]
fn a_nook_behind_a_chair_is_not_clear() {
    let c = content();
    let mut next = 1;
    let (mut g, f) = grid::parse_layout(
        "######\n#..h.#\n#..###\n#....#\n#DD###\n======",
        &c,
        &mut next,
    );
    g.rebuild(&f, &c);
    assert!(g.is_seat(3, 1));
    assert!(g.reached(4, 1) && !g.clear(4, 1) && !g.guest_clear(4, 1));
    assert!(g.clear(1, 1) && g.guest_clear(1, 1));
    assert!(!g.clear(3, 1), "a chair is never clear floor");
}

/// Cause 5: a seat whose only guests' side was another chair (a stool by the counter's end,
/// staff floor round it) was handed out — its guest climbed over whoever sat on the other.
#[test]
fn a_seat_only_reached_over_another_chair_is_not_offered() {
    let c = content();
    let mut w = night(&c, 3);
    // the counter's open end: hall at x 15, staff floor from x 16 (rows 11–13)
    let (outer, inner) = ((15, 12), (16, 12));
    assert!(w.grid.room(inner.0, inner.1).staff_only());
    assert!(!w.grid.room(outer.0, outer.1).staff_only());
    place(&mut w, &c, "chair", outer).unwrap();
    place(&mut w, &c, "stool", inner).unwrap();
    let inner_uid = w.grid.object_at(inner.0, inner.1).unwrap();
    // every other seat full
    for s in guests::seats(&w, &c) {
        if s.uid != inner_uid
            && let Some(p) = w.furniture.iter_mut().find(|p| p.uid == s.uid)
        {
            p.seat_by[s.idx as usize] = Some(u32::MAX);
        }
    }
    let gid = sitter(&mut w, &c);
    assert_ne!(
        guest(&w, gid).seat.map(|s| s.0),
        Some(inner_uid),
        "got the stool behind the chair"
    );
}

/// Cause 5b: a chair right inside the door (everyone's only way in) was allowed: the checks
/// started past it. They start from the street now, so such a build is refused.
#[test]
fn a_chair_that_would_be_the_only_way_in_is_refused() {
    let c = content();
    let mut w = night(&c, 3);
    // grandma's door: (1,15) and (2,15); a table inside the first leaves the second
    place(&mut w, &c, "table_round", (1, 14)).unwrap();
    assert_eq!(
        place(&mut w, &c, "chair", (2, 14)),
        Err(CommandError::BlocksPath)
    );
}

/// Cause 5c: a register walled in by chairs had its line and its to-go guests climb over
/// the seated guests: guests order there only when its front is clear.
#[test]
fn a_register_boxed_in_by_chairs_takes_no_bar_orders() {
    let c = content();
    let mut w = night(&c, 3);
    let (reg, front) = guests::registers(&w, &c)[0];
    assert_eq!(guests::guest_registers(&w, &c).len(), 1);
    for d in [(-1, 0), (1, 0), (0, -1)] {
        let t = (front.0 + d.0, front.1 + d.1);
        if w.grid.guest_ok(t.0, t.1) {
            place(&mut w, &c, "chair", t).unwrap();
        }
    }
    assert!(!w.grid.guest_clear(front.0, front.1));
    assert!(guests::guest_registers(&w, &c).is_empty());
    assert!(
        super::to_go::spots(&w, &c, reg)
            .iter()
            .all(|t| w.grid.guest_clear(t.0, t.1))
    );
    for _ in 0..12 {
        let pid = guests::spawn_party(&mut w, &c, "students", 1, None, 1.0, 1.0, &[], None, false)
            .expect("spawned");
        let p = w.parties.iter().find(|p| p.id == pid).unwrap();
        assert_ne!(
            p.state,
            PartyState::Queue,
            "a party lined up behind the chairs"
        );
    }
}

/// Cause 6 (the QA's "walked into a piece at (26,9)"): the leak's bucket was put on any hall
/// tile but a guest's — also under Karolina or where she was walking to; she walked into it.
#[test]
fn a_bucket_never_lands_where_someone_stands_or_walks_to() {
    let c = content();
    let mut w = night(&c, 3);
    let mut probe = w.clone();
    staff::block_random_tile(&mut probe, &c);
    let t = probe.buckets.last().expect("a bucket").0;
    // the same draw, with Karolina heading there
    let k = karolina(&w);
    w.staff[k].path = vec![t];
    staff::block_random_tile(&mut w, &c);
    let b = w.buckets.last().expect("a bucket elsewhere").0;
    assert_ne!(b, t, "the bucket went on her goal");
    assert_eq!(
        w.rng.clone().next_u64(),
        probe.rng.clone().next_u64(),
        "one draw"
    );
    // …or right under her
    let mut w2 = night(&c, 3);
    let k = karolina(&w2);
    w2.staff[k].path.clear();
    w2.staff[k].pos = grid::tile_center(t);
    staff::block_random_tile(&mut w2, &c);
    assert_ne!(w2.buckets.last().map(|b| b.0), Some(t));
}

/// Cause 6b: an item's pick-up spot (and a walk's goal) fell back to the piece's own tile
/// when nothing beside it was free — she walked into it. No spot now: the act is dropped.
#[test]
fn a_pick_up_spot_is_never_inside_a_piece() {
    let c = content();
    let mut w = night(&c, 3);
    // grandma's plant in the corner (1,1): wall it in
    for t in [(2, 1), (1, 2), (2, 2)] {
        place(&mut w, &c, "table_round", t).unwrap();
    }
    assert!(!w.grid.walkable(1, 1));
    assert_eq!(staff::pickup_tile(&w, (1, 1), (5, 5)), None);
    assert_eq!(staff::pickup_tile(&w, (5, 5), (5, 5)), Some((5, 5)));
}

/// 0.2.4: grandma's register stands by the outer wall, and the to-go spots within four steps
/// of its front reached through the wall onto the sidewalk — guests waited for their cups out
/// on the street. Spots are indoor floor only.
#[test]
fn to_go_spots_are_always_inside_the_cafe() {
    let c = content();
    let mut w = night(&c, 3);
    // a second register in the empty south-west of the hall, by the outer wall
    for rot in 0..4u8 {
        let ok = w.apply(
            &c,
            Command::PlaceFurniture {
                def: "register".into(),
                x: 6,
                y: 13,
                rot,
            },
        );
        if ok.is_ok() {
            break;
        }
    }
    assert_eq!(guests::registers(&w, &c).len(), 2, "the second register was placed");
    for (reg, _) in guests::registers(&w, &c) {
        let spots = super::to_go::spots(&w, &c, reg);
        assert!(!spots.is_empty(), "register {reg} has no to-go spot");
        for t in spots {
            assert!(
                w.grid.is_indoor_floor(t.0, t.1),
                "to-go spot {t:?} of register {reg} is not inside"
            );
        }
    }
}

/// 0.2.4 (the two leftover chaos finds): a nook's way in is the chair in front of it…
#[test]
fn a_nook_knows_the_chair_that_is_its_way_in() {
    let c = content();
    let mut next = 1;
    let (mut g, f) = grid::parse_layout(
        "######\n#..h.#\n#..###\n#....#\n#DD###\n======",
        &c,
        &mut next,
    );
    g.rebuild(&f, &c);
    assert_eq!(g.nook_gates((4, 1)), vec![(3, 1)]);
    assert!(g.nook_gates((1, 1)).is_empty(), "clear floor has no gate");
    assert!(g.nook_gates((3, 1)).is_empty(), "a chair is no nook");
}

/// …and while Karolina works in there, no guest sits down on it: she climbed out over the
/// guest who did.
#[test]
fn nobody_sits_down_in_the_way_out_of_a_nook_she_works_in() {
    let c = content();
    let build = |c: &Content| {
        let mut w = night(c, 3);
        // the north-east corner (28,1) walled in by two tables, its way in a chair
        place(&mut w, c, "table_round", (27, 1)).unwrap();
        place(&mut w, c, "table_round", (27, 2)).unwrap();
        place(&mut w, c, "chair", (28, 2)).unwrap();
        assert_eq!(w.grid.nook_gates((28, 1)), vec![(28, 2)]);
        w
    };
    let gate_taken = |w: &mut World, c: &Content| {
        for _ in 0..40 {
            let _ = guests::spawn_party(w, c, "students", 1, None, 1.0, 1.0, &[], None, true);
        }
        w.guests
            .iter()
            .filter_map(|g| g.seat)
            .any(|(u, k)| guests::seat_tile(w, u, k) == Some((28, 2)))
    };
    // without her in there the chair fills up like any other
    let mut w = build(&c);
    assert!(gate_taken(&mut w, &c), "the chair is a seat like any other");
    // with her in the nook nobody picks it
    let mut w = build(&c);
    let k = karolina(&w);
    w.staff[k].path.clear();
    w.staff[k].pos = grid::tile_center((28, 1));
    assert_eq!(staff::gates_in_use(&w), vec![(28, 2)]);
    assert!(!gate_taken(&mut w, &c), "a guest sat down in her way out");
    // …nor while she walks there
    let mut w = build(&c);
    let k = karolina(&w);
    w.staff[k].path = vec![(28, 1)];
    assert!(!gate_taken(&mut w, &c), "a guest sat down in her way in");
}
