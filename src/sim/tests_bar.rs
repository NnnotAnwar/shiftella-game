//! The bar: the default café's counter, back line and appliance roles (sim::grid
//! `DEFAULT_LAYOUT`, sim::companions) — every station reachable from the staff side, the
//! register queue on walkable hall tiles, nobody walking through the counter, no two pieces
//! with the same job, the helpers' speed-ups, and old saves with removed pieces.

use super::*;
use crate::content::Content;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

fn content() -> Content {
    Content::load(DATA).unwrap_or_else(|e| panic!("{e}"))
}

fn uid_of(w: &World, def: &str) -> u32 {
    w.furniture()
        .iter()
        .find(|p| p.def == def)
        .unwrap_or_else(|| panic!("no {def} in the default café"))
        .uid
}

/// Every tile of the café's walls / counter / appliances: nothing may walk over these.
fn solid(w: &World, c: &Content, t: (i32, i32)) -> bool {
    !w.grid().walkable(t.0, t.1)
        && w.grid()
            .object_at(t.0, t.1)
            .and_then(|u| w.furniture_by_uid(u))
            .and_then(|p| c.furniture_def(&p.def))
            .is_some_and(|d| !d.walkable)
}

#[test]
fn every_station_is_reachable_from_the_staff_side() {
    let c = content();
    let w = World::new_story(&c, Difficulty::Cozy);
    let k = w.karolina().expect("Karolina").tile();
    let reach = grid::reachable(w.grid(), w.grid().entrance());
    for p in w.furniture() {
        let d = c.furniture_def(&p.def).unwrap();
        let station = d.tags.iter().any(|t| t.starts_with("station:"))
            || ["register", "sink", "trash", "drone"]
                .iter()
                .any(|t| d.has_tag(t));
        if !station {
            continue;
        }
        let at = staff::station_access(&w, &c, p.uid)
            .unwrap_or_else(|| panic!("{} has no access tile", p.def));
        assert!(
            w.grid().room(at.0, at.1).staff_only(),
            "{} is worked from the hall at {at:?}",
            p.def
        );
        let i = w.grid().idx(at.0, at.1).unwrap();
        assert!(reach[i], "{} at {at:?} can't be reached", p.def);
        assert!(
            grid::find_path(w.grid(), k, at, false).is_some() || k == at,
            "Karolina can't walk to the {}",
            p.def
        );
    }
}

#[test]
fn the_register_queue_stands_on_walkable_hall_tiles() {
    let c = content();
    let w = World::new_story(&c, Difficulty::Cozy);
    let reg = uid_of(&w, "register");
    let slots = guests::queue_slots(&w, &c, reg);
    assert!(slots.len() >= 6, "a queue of at least six: {slots:?}");
    let reach = grid::reachable(w.grid(), w.grid().entrance());
    for t in &slots {
        assert!(w.grid().guest_ok(t.0, t.1), "queue slot {t:?} isn't a hall tile");
        assert!(reach[w.grid().idx(t.0, t.1).unwrap()], "queue slot {t:?} unreachable");
    }
    // the front of the queue faces the register across the counter
    let p = w.furniture_by_uid(reg).unwrap();
    assert_eq!(slots[0], (p.x, p.y - 1));
    // and the guest path from the door to it never enters the bar
    let path = grid::find_path(w.grid(), w.grid().outside_door(), slots[0], true).unwrap();
    assert!(path.iter().all(|t| !w.grid().room(t.0, t.1).staff_only()));
}

#[test]
fn nobody_walks_through_the_counter() {
    let c = content();
    let w = World::new_story(&c, Difficulty::Cozy);
    let g = w.grid();
    // staff from every hall table to every station, and back
    let tables: Vec<(i32, i32)> = w
        .furniture()
        .iter()
        .filter(|p| p.def == "table_round")
        .map(|p| (p.x, p.y + 1))
        .collect();
    let stations: Vec<(i32, i32)> = w
        .furniture()
        .iter()
        .filter_map(|p| staff::station_access(&w, &c, p.uid).filter(|t| g.room(t.0, t.1).staff_only()))
        .collect();
    assert!(!tables.is_empty() && stations.len() >= 6);
    for &a in &tables {
        for &b in &stations {
            for path in [grid::find_path(g, a, b, false), grid::find_path(g, b, a, false)] {
                let path = path.unwrap_or_else(|| panic!("no staff path {a:?} ↔ {b:?}"));
                for t in &path {
                    assert!(!solid(&w, &c, *t), "path {a:?} → {b:?} crosses {t:?}");
                }
            }
        }
    }
    // the front counter closes the bar off the hall: guests can't step behind it
    for x in 16..=26 {
        assert!(!g.walkable(x, 10), "the front counter has a hole at x {x}");
    }
}

#[test]
fn the_default_cafe_has_no_two_pieces_with_the_same_job() {
    let c = content();
    let w = World::new_story(&c, Difficulty::Cozy);
    // a piece's job: its station / role tags (decor, seats and lights may repeat)
    let job = |d: &crate::content::FurnitureDef| -> Option<String> {
        let roles: Vec<&str> = d
            .tags
            .iter()
            .map(String::as_str)
            .filter(|t| {
                t.starts_with("station:")
                    || t.starts_with("helps:")
                    || ["register", "sink", "trash", "drone", "music"].contains(t)
            })
            .collect();
        (!roles.is_empty()).then(|| roles.join("+"))
    };
    let mut seen: std::collections::BTreeMap<String, String> = Default::default();
    for p in w.furniture() {
        let d = c.furniture_def(&p.def).unwrap();
        let Some(j) = job(d) else { continue };
        if let Some(other) = seen.insert(j.clone(), p.def.clone()) {
            assert_eq!(
                other, p.def,
                "{} and {} both do `{j}` in the default café",
                other, p.def
            );
        }
    }
    // and across the catalogue: two different appliances never share a job
    let mut by_job: std::collections::BTreeMap<String, Vec<&str>> = Default::default();
    for d in c.furniture.iter().filter(|d| d.skin.is_none()) {
        if let Some(j) = job(d) {
            by_job.entry(j).or_default().push(&d.id);
        }
    }
    for (j, ids) in &by_job {
        if j == "music" {
            continue; // jukebox vs radio: grandma's two music pieces are a story choice
        }
        assert_eq!(ids.len(), 1, "`{j}` is done by {ids:?}");
    }
    // the helpers stand beside what they help
    let rack = uid_of(&w, "dish_rack");
    let grinder = uid_of(&w, "espresso_grinder");
    assert!(companions::helper_working(&w, &c, rack));
    assert!(companions::helper_working(&w, &c, grinder));
    assert!(companions::station_mul(&w, &c, uid_of(&w, "sink")) > 1.0);
    assert!(companions::station_mul(&w, &c, uid_of(&w, "coffee_machine")) > 1.0);
    assert_eq!(companions::station_mul(&w, &c, uid_of(&w, "stove")), 1.0);
    // the old corner counter is gone from the catalogue
    assert!(c.furniture_def("counter_corner").is_none());
}

#[test]
fn the_bar_reads_as_a_bar() {
    let c = content();
    let w = World::new_story(&c, Difficulty::Cozy);
    let at = |def: &str| {
        let p = w.furniture_by_uid(uid_of(&w, def)).unwrap();
        (p.x, p.y)
    };
    // the register and the coffee machine stand on the front counter, the grinder beside it
    let (rx, ry) = at("register");
    let (mx, my) = at("coffee_machine");
    let (gx, gy) = at("espresso_grinder");
    assert_eq!((ry, my, gy), (10, 10, 10));
    assert_eq!((gx - mx).abs(), 1);
    assert!(rx > mx);
    // the kitchen stands against the front wall, nothing floats in the middle of the floor
    for def in ["stove", "fridge", "sink", "dish_rack", "trash_bin"] {
        let (x, y) = at(def);
        assert_eq!(y, 14, "{def} at ({x}, {y}) isn't on the back line");
    }
    let (sx, _) = at("sink");
    let (dx, _) = at("dish_rack");
    assert_eq!((sx - dx).abs(), 1, "the rack stands beside the sink");
    // the walkway behind the counter is clear (rows 11–13), open to the hall at the left end
    for y in 11..=13 {
        for x in 16..=28 {
            assert!(w.grid().walkable(x, y), "the walkway is blocked at ({x}, {y})");
        }
    }
}

#[test]
fn a_drying_rack_speeds_up_washing_only_beside_the_sink() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    let sink = uid_of(&w, "sink");
    let rack = uid_of(&w, "dish_rack");
    let with = companions::station_mul(&w, &c, sink);
    assert!((with - companions::RACK_WASH_MUL).abs() < 1e-6);
    // carry it away from the sink: no more speed-up, and it shows idle
    let p = w.furniture_by_uid(rack).unwrap().clone();
    w.furniture.retain(|f| f.uid != rack);
    let mut moved = p.clone();
    moved.x = 27;
    moved.y = 14;
    w.furniture.push(moved);
    assert_eq!(companions::station_mul(&w, &c, sink), 1.0);
    assert!(!companions::helper_working(&w, &c, rack));
    let view = w
        .furniture_views(&c)
        .into_iter()
        .find(|v| v.uid == rack)
        .unwrap();
    assert!(!view.active);
}

#[test]
fn karolina_washes_faster_beside_the_drying_rack() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    w.sys.tutorial_off = true;
    bot::finish_cutscenes(&mut w, &c);
    w.start_night(&c);
    bot::finish_cutscenes(&mut w, &c);
    w.skip_prep(&c);
    w.mods.push(Modifier {
        kind: ModKind::GuestRate,
        mul: 0.0,
        until: None,
        event: None,
    });
    let sink = uid_of(&w, "sink");
    w.sink_dishes = 9;
    w.apply(
        &c,
        Command::PlayerQueue {
            act: PlayerAct::Wash { sink },
            front: false,
        },
    )
    .unwrap();
    for _ in 0..600 {
        w.tick(&c);
        if w.player().current.as_ref().is_some_and(|a| a.stage == 1) {
            break;
        }
    }
    // 2 dishes a second × 1.5 with the rack: 2 s wash 6
    w.apply(&c, Command::PlayerWashProgress { sink, seconds: 2.0 })
        .unwrap();
    assert_eq!(w.sink_dishes(), 3);
}

#[test]
fn old_saves_with_a_corner_counter_load_it_as_a_bar_counter() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    let n = w.furniture().len();
    let ends: Vec<u32> = w
        .furniture()
        .iter()
        .filter(|p| p.def == "bar_counter")
        .take(2)
        .map(|p| p.uid)
        .collect();
    for p in w.furniture.iter_mut().filter(|p| ends.contains(&p.uid)) {
        p.def = "counter_corner".into();
    }
    w.story.unlocked_furniture.insert("counter_corner".into());
    let text = w.save_to_string().unwrap();
    let mut back = World::load_from_str(&text).unwrap();
    back.rebuild_caches(&c);
    assert_eq!(back.furniture().len(), n);
    assert!(back.furniture().iter().all(|p| p.def != "counter_corner"));
    for uid in ends {
        assert_eq!(back.furniture_by_uid(uid).unwrap().def, "bar_counter");
    }
    assert!(back.story().unlocked_furniture.contains("bar_counter"));
    assert!(!back.story().unlocked_furniture.contains("counter_corner"));
    // every placed piece resolves in the catalogue
    for p in back.furniture() {
        assert!(c.furniture_def(&p.def).is_some(), "unknown piece {}", p.def);
    }
}
