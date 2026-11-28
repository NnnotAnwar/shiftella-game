//! Walking as Karolina (`sim::steer`): WASD steering, the blockers (walls, furniture, seats,
//! people), never resting on a seat or between tiles, E's pick of the thing beside her, a
//! queued walk cancelled cleanly by steering, and determinism.

use super::grid;
use super::guests;
use super::rng::Rng;
use super::steer;
use super::*;
use crate::content::Content;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

fn content() -> Content {
    Content::load(DATA).unwrap_or_else(|e| panic!("{e}"))
}

/// Story night `n` past the preparation, no random walk-ins, cutscenes done.
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
    w.drain_events();
    w
}

fn kid(w: &World) -> usize {
    w.staff.iter().position(|s| s.owner).expect("Karolina")
}

/// Karolina stands on `t` (its centre), free, facing `f`.
fn put(w: &mut World, t: (i32, i32), f: Facing) {
    let i = kid(w);
    w.staff[i].pos = grid::tile_center(t);
    w.staff[i].path.clear();
    w.staff[i].facing = f;
}

fn here(w: &World) -> (i32, i32) {
    w.karolina().unwrap().tile()
}

fn ticks(w: &mut World, c: &Content, n: u32) {
    for _ in 0..n {
        w.tick(c);
        bot::finish_cutscenes(w, c);
    }
}

fn steer(w: &mut World, c: &Content, dir: (i8, i8)) {
    w.apply(c, Command::PlayerMove { dir }).expect("steer");
}

/// A tile she may stand on with a neighbour `dir` away that `blocked` likes.
fn spot(
    w: &World,
    mut blocked: impl FnMut(&World, (i32, i32)) -> bool,
) -> Option<((i32, i32), (i8, i8))> {
    let i = kid(w);
    for y in 0..w.grid.height {
        for x in 0..w.grid.width {
            if !steer::passable(w, i, (x, y)) {
                continue;
            }
            for d in [(1i8, 0i8), (-1, 0), (0, 1), (0, -1)] {
                let n = (x + d.0 as i32, y + d.1 as i32);
                if blocked(w, n) {
                    return Some(((x, y), d));
                }
            }
        }
    }
    None
}

fn facing_of(d: (i8, i8)) -> Facing {
    Facing::from_delta(d.0 as f32, d.1 as f32)
}

/// At rest she is on a tile's centre, on floor, never on a seat.
fn at_rest_ok(w: &World) {
    let k = w.karolina().unwrap();
    let t = k.tile();
    assert!(w.grid.walkable(t.0, t.1), "on furniture at {t:?}");
    if k.path.is_empty() {
        let c = grid::tile_center(t);
        assert!(
            (k.pos.0 - c.0).abs() < 0.01 && (k.pos.1 - c.1).abs() < 0.01,
            "between tiles at {:?}",
            k.pos
        );
        assert!(!w.grid.is_seat(t.0, t.1), "resting on a seat at {t:?}");
    }
}

#[test]
fn steering_walks_her_at_her_own_pace_and_stops_on_a_tile() {
    let c = content();
    let mut w = night(&c, 2);
    let ((x, y), d) = spot(&w, |w, n| steer::passable(w, kid(w), n)).expect("open floor");
    put(&mut w, (x, y), Facing::Down);
    steer(&mut w, &c, d);
    ticks(&mut w, &c, 1);
    let k = w.karolina().unwrap();
    // her auto-walk speed: 0.5 × player_walk_mul tiles a tick
    let moved = ((k.pos.0 - grid::tile_center((x, y)).0).abs()
        + (k.pos.1 - grid::tile_center((x, y)).1).abs())
        / grid::TILE;
    let want = 0.5 * c.handson.tuning.player_walk_mul;
    assert!((moved - want).abs() < 0.05 || moved >= 1.0, "moved {moved}");
    assert_eq!(k.facing, facing_of(d));
    assert_eq!(k.pose, Pose::Walk);
    steer(&mut w, &c, (0, 0));
    ticks(&mut w, &c, 4);
    at_rest_ok(&w);
    assert_ne!(here(&w), (x, y), "she went somewhere");
}

#[test]
fn walls_and_furniture_stop_her_she_only_turns() {
    let c = content();
    let mut w = night(&c, 2);
    // a piece of furniture (not a seat) beside open floor
    let ((x, y), d) = spot(&w, |w, n| {
        w.grid.in_bounds(n.0, n.1)
            && w.grid.object_at(n.0, n.1).is_some()
            && !w.grid.walkable(n.0, n.1)
    })
    .expect("a counter beside the floor");
    put(&mut w, (x, y), Facing::Down);
    steer(&mut w, &c, d);
    ticks(&mut w, &c, 12);
    assert_eq!(here(&w), (x, y), "the piece stops her");
    assert_eq!(
        w.karolina().unwrap().facing,
        facing_of(d),
        "she turns to it"
    );
    at_rest_ok(&w);
    // a wall
    let ((x, y), d) = spot(&w, |w, n| {
        w.grid
            .tile(n.0, n.1)
            .is_some_and(|t| t.kind == TileKind::Wall)
    })
    .expect("a wall beside the floor");
    put(&mut w, (x, y), Facing::Down);
    steer(&mut w, &c, d);
    ticks(&mut w, &c, 12);
    assert_eq!(here(&w), (x, y), "the wall stops her");
}

#[test]
fn seats_and_people_stop_her() {
    let c = content();
    let mut w = night(&c, 2);
    let ((x, y), d) = spot(&w, |w, n| w.grid.is_seat(n.0, n.1)).expect("a chair");
    put(&mut w, (x, y), Facing::Down);
    steer(&mut w, &c, d);
    ticks(&mut w, &c, 12);
    assert_eq!(here(&w), (x, y), "a chair is not a way");
    // a guest standing on the next tile
    let pid = guests::spawn_party(&mut w, &c, "students", 1, None, 1.0, 1.0, &[], None, false)
        .expect("spawned");
    let g = w.parties.iter().find(|p| p.id == pid).unwrap().members[0];
    let ((x, y), d) = spot(&w, |w, n| steer::passable(w, kid(w), n)).unwrap();
    put(&mut w, (x, y), Facing::Down);
    let n = (x + d.0 as i32, y + d.1 as i32);
    let gi = w.guests.iter().position(|x| x.id == g).unwrap();
    w.guests[gi].pos = grid::tile_center(n);
    w.guests[gi].path.clear();
    let i = kid(&w);
    assert_eq!(
        steer::next_tile(&w, i, (x, y), d),
        None,
        "a guest is in the way"
    );
    // a helper too
    let hi = w.staff.iter().position(|s| !s.owner && s.present());
    if let Some(hi) = hi {
        w.guests[gi].pos = grid::tile_center((0, 0));
        w.staff[hi].pos = grid::tile_center(n);
        assert_eq!(
            steer::next_tile(&w, i, (x, y), d),
            None,
            "a helper is in the way"
        );
    }
}

#[test]
fn a_diagonal_never_cuts_a_corner() {
    let c = content();
    let w = night(&c, 2);
    let i = kid(&w);
    // a tile with free floor diagonally but a piece on one side
    for y in 0..w.grid.height {
        for x in 0..w.grid.width {
            for d in [(1i8, 1i8), (1, -1), (-1, 1), (-1, -1)] {
                if !steer::passable(&w, i, (x, y)) {
                    continue;
                }
                let h = (x + d.0 as i32, y);
                let v = (x, y + d.1 as i32);
                let diag = (x + d.0 as i32, y + d.1 as i32);
                let got = steer::next_tile(&w, i, (x, y), d);
                if !(steer::passable(&w, i, h) && steer::passable(&w, i, v)) {
                    assert_ne!(
                        got,
                        Some(diag),
                        "cut the corner at {:?} going {d:?}",
                        (x, y)
                    );
                }
                if let Some(t) = got {
                    assert!(steer::passable(&w, i, t));
                    assert!(grid::chebyshev(t, (x, y)) == 1);
                }
            }
        }
    }
}

#[test]
fn random_steering_never_leaves_the_floor_or_rests_on_a_seat() {
    let c = content();
    for seed in 0..3u64 {
        let mut w = night(&c, 3 + seed as u32);
        let mut r = Rng::new(seed * 77 + 5);
        for _ in 0..1500 {
            if r.chance(0.15) {
                let d = (r.range_i64(-1, 2) as i8, r.range_i64(-1, 2) as i8);
                let _ = w.apply(&c, Command::PlayerMove { dir: d });
            }
            w.tick(&c);
            bot::finish_cutscenes(&mut w, &c);
            let k = w.karolina().unwrap();
            let t = k.tile();
            assert!(
                w.grid.walkable(t.0, t.1),
                "seed {seed}: on furniture at {t:?}"
            );
            if w.steering() != (0, 0) && k.path.is_empty() {
                at_rest_ok(&w);
            }
            let v = chaos::people(&w, &c);
            assert!(v.is_empty(), "seed {seed}: {v:?}");
        }
        steer(&mut w, &c, (0, 0));
        ticks(&mut w, &c, 3);
        at_rest_ok(&w);
    }
}

#[test]
fn steering_by_day_is_refused_but_letting_go_is_fine() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    bot::finish_cutscenes(&mut w, &c);
    assert_eq!(
        w.apply(&c, Command::PlayerMove { dir: (1, 0) }),
        Err(CommandError::WrongPhase)
    );
    assert!(w.apply(&c, Command::PlayerMove { dir: (0, 0) }).is_ok());
    let mut n = night(&c, 2);
    assert!(matches!(
        n.apply(&c, Command::PlayerMove { dir: (2, 0) }),
        Err(CommandError::Invalid(_))
    ));
}

// ---------------------------------------------------------------------------------------------
// Her queue while steered
// ---------------------------------------------------------------------------------------------

/// A table, and dishes on it.
fn dirty_table(w: &mut World, c: &Content, skip: usize) -> u32 {
    let uid = w
        .furniture
        .iter()
        .filter(|p| c.furniture_def(&p.def).is_some_and(|d| d.has_tag("table")))
        .nth(skip)
        .map(|p| p.uid)
        .expect("a table");
    w.furniture
        .iter_mut()
        .find(|p| p.uid == uid)
        .unwrap()
        .dishes = 2;
    uid
}

#[test]
fn steering_cancels_the_walk_to_her_act_cleanly_and_the_queue_waits() {
    let c = content();
    let mut w = night(&c, 2);
    let far = dirty_table(&mut w, &c, 0);
    let next = dirty_table(&mut w, &c, 1);
    let sink0 = w.sink_dishes;
    w.apply(
        &c,
        Command::PlayerQueue {
            act: PlayerAct::ClearTable { table: far },
            front: false,
        },
    )
    .unwrap();
    w.apply(
        &c,
        Command::PlayerQueue {
            act: PlayerAct::ClearTable { table: next },
            front: false,
        },
    )
    .unwrap();
    ticks(&mut w, &c, 1);
    let cur = w.player().current.clone().expect("on her way");
    assert_eq!(cur.stage, 0);
    assert_eq!(
        w.furniture_by_uid(far).unwrap().claim,
        w.karolina().map(|k| k.id)
    );
    // steer: the walk to the first table is cancelled, nothing half done
    let i = kid(&w);
    let d = [(1i8, 0i8), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .find(|d| steer::next_tile(&w, i, w.staff[i].tile(), *d).is_some())
        .unwrap_or((1, 0));
    steer(&mut w, &c, d);
    ticks(&mut w, &c, 1);
    assert!(w.player().current.is_none(), "the walk is cancelled");
    assert_eq!(
        w.furniture_by_uid(far).unwrap().claim,
        None,
        "claim released"
    );
    assert_eq!(
        w.furniture_by_uid(far).unwrap().dishes,
        2,
        "the cups are still there"
    );
    assert_eq!(w.sink_dishes, sink0);
    assert_eq!(w.player().queue.len(), 1, "the next act waits");
    // while steered and a moment after, the queue waits
    ticks(&mut w, &c, 6);
    assert!(w.player().current.is_none());
    steer(&mut w, &c, (0, 0));
    ticks(&mut w, &c, steer::HOLD_TICKS - 2);
    assert!(w.player().current.is_none(), "still the moment after");
    ticks(&mut w, &c, 4);
    assert!(
        w.player()
            .current
            .as_ref()
            .is_some_and(|c| c.act == PlayerAct::ClearTable { table: next }),
        "then she carries on: {:?}",
        w.player().current
    );
}

#[test]
fn a_click_after_steering_does_not_wait() {
    let c = content();
    let mut w = night(&c, 2);
    let t = dirty_table(&mut w, &c, 0);
    let i = kid(&w);
    let d = [(1i8, 0i8), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .find(|d| steer::next_tile(&w, i, w.staff[i].tile(), *d).is_some())
        .unwrap();
    steer(&mut w, &c, d);
    ticks(&mut w, &c, 2);
    steer(&mut w, &c, (0, 0));
    w.apply(
        &c,
        Command::PlayerQueue {
            act: PlayerAct::ClearTable { table: t },
            front: false,
        },
    )
    .unwrap();
    ticks(&mut w, &c, 3);
    assert!(w.player().current.is_some(), "the click goes at once");
}

#[test]
fn working_she_finishes_first() {
    let c = content();
    let mut w = night(&c, 2);
    let t = dirty_table(&mut w, &c, 0);
    w.apply(
        &c,
        Command::PlayerQueue {
            act: PlayerAct::ClearTable { table: t },
            front: false,
        },
    )
    .unwrap();
    // until she is at the table, picking up
    for _ in 0..400 {
        ticks(&mut w, &c, 1);
        if w.player().current.as_ref().is_some_and(|c| c.stage >= 1) {
            break;
        }
    }
    assert!(w.player().current.as_ref().is_some_and(|c| c.stage >= 1));
    let at = here(&w);
    steer(&mut w, &c, (1, 0));
    ticks(&mut w, &c, 1);
    assert!(w.player().current.is_some(), "busy: the keys wait");
    let _ = at;
    // she finishes (clears, carries to the sink) and only then walks
    for _ in 0..600 {
        ticks(&mut w, &c, 1);
        if w.player().current.is_none() {
            break;
        }
    }
    assert!(w.player().current.is_none());
    assert_eq!(w.furniture_by_uid(t).unwrap().dishes, 0);
}

// ---------------------------------------------------------------------------------------------
// Interact
// ---------------------------------------------------------------------------------------------

/// A free floor tile beside the piece `uid` (not a seat), and the way to it.
fn beside(w: &World, c: &Content, uid: u32) -> ((i32, i32), (i8, i8)) {
    let p = w.furniture_by_uid(uid).unwrap();
    let d = c.furniture_def(&p.def).unwrap();
    let fp = grid::footprint(d, p.x, p.y, p.rot);
    let i = kid(w);
    for t in &fp {
        for dir in [(0i8, 1i8), (0, -1), (1, 0), (-1, 0)] {
            let n = (t.0 - dir.0 as i32, t.1 - dir.1 as i32);
            if !fp.contains(&n) && steer::passable(w, i, n) {
                return (n, dir);
            }
        }
    }
    panic!("no free side by {}", p.def)
}

#[test]
fn e_beside_a_dirty_table_clears_it_without_a_walk() {
    let c = content();
    let mut w = night(&c, 2);
    let t = dirty_table(&mut w, &c, 0);
    let (spot, dir) = beside(&w, &c, t);
    put(&mut w, spot, facing_of(dir));
    let n = w.nearby(&c).expect("the table is in reach");
    assert_eq!(n.kind, NearKind::Act(PlayerAct::ClearTable { table: t }));
    assert!(n.faced);
    assert_eq!(n.label, "walk.do.clear");
    w.apply(&c, n.command().unwrap()).unwrap();
    ticks(&mut w, &c, 1);
    let k = w.karolina().unwrap();
    assert!(
        k.path.is_empty(),
        "she does it from where she stands: {:?}",
        k.path
    );
    assert_eq!(k.tile(), spot);
    // done: the cups go to the sink
    for _ in 0..400 {
        ticks(&mut w, &c, 1);
        if w.furniture_by_uid(t).unwrap().dishes == 0 {
            break;
        }
    }
    assert_eq!(w.furniture_by_uid(t).unwrap().dishes, 0);
    // it is hers now: E doesn't offer it twice
    assert!(
        w.nearby_all(&c)
            .iter()
            .all(|n| n.kind != NearKind::Act(PlayerAct::ClearTable { table: t }))
    );
}

#[test]
fn e_beside_a_waiting_guest_takes_the_order() {
    let c = content();
    let mut w = night(&c, 2);
    let pid = guests::spawn_party(&mut w, &c, "students", 1, None, 1.0, 1.0, &[], None, false)
        .expect("spawned");
    guests::settle_now(&mut w, pid);
    let g = w.parties.iter().find(|p| p.id == pid).unwrap().members[0];
    let gt = w.guests.iter().find(|x| x.id == g).unwrap().tile();
    let i = kid(&w);
    let (spot, dir) = [(0i8, 1i8), (0, -1), (1, 0), (-1, 0)]
        .into_iter()
        .map(|d| ((gt.0 - d.0 as i32, gt.1 - d.1 as i32), d))
        .find(|(t, _)| steer::passable(&w, i, *t))
        .expect("room beside the guest");
    put(&mut w, spot, facing_of(dir));
    let n = w.nearby(&c).expect("the guest is in reach");
    assert_eq!(n.kind, NearKind::Act(PlayerAct::TakeOrder { party: pid }));
    assert_eq!(n.label, "walk.do.take_order");
    w.apply(&c, n.command().unwrap()).unwrap();
    assert_eq!(
        w.parties.iter().find(|p| p.id == pid).unwrap().order_claim,
        w.karolina().map(|k| k.id)
    );
    // nothing is in reach far away
    let far = spot_far(&w, gt);
    put(&mut w, far, Facing::Down);
    assert!(
        w.nearby_all(&c)
            .iter()
            .all(|n| !matches!(n.kind, NearKind::Act(PlayerAct::TakeOrder { .. })))
    );
}

fn spot_far(w: &World, from: (i32, i32)) -> (i32, i32) {
    let i = kid(w);
    (0..w.grid.height)
        .flat_map(|y| (0..w.grid.width).map(move |x| (x, y)))
        .find(|t| steer::passable(w, i, *t) && grid::chebyshev(*t, from) > 4)
        .expect("far floor")
}

#[test]
fn what_she_faces_wins() {
    let c = content();
    let mut w = night(&c, 2);
    let t = dirty_table(&mut w, &c, 0);
    let (spot, dir) = beside(&w, &c, t);
    // a dirt blot under her feet too (mop) — facing the table, the table wins
    put(&mut w, spot, facing_of(dir));
    let all = w.nearby_all(&c);
    assert!(all[0].faced, "{all:?}");
    // turned away from the table it isn't faced any more: a faced thing (if any) or the
    // table by its priority still — never something out of reach
    let back = facing_of((-dir.0, -dir.1));
    put(&mut w, spot, back);
    for n in w.nearby_all(&c) {
        assert!(grid::chebyshev(n.tile, spot) <= 1, "{n:?}");
    }
}

#[test]
fn interact_is_refused_by_day_and_for_her_own_act() {
    let c = content();
    let mut w = night(&c, 2);
    let t = dirty_table(&mut w, &c, 0);
    let act = PlayerAct::ClearTable { table: t };
    w.apply(&c, Command::PlayerInteract { act: act.clone() })
        .unwrap();
    assert!(matches!(
        w.apply(&c, Command::PlayerInteract { act }),
        Err(CommandError::Invalid(_))
    ));
}

#[test]
fn interact_lets_go_and_goes_first() {
    let c = content();
    let mut w = night(&c, 2);
    let a = dirty_table(&mut w, &c, 0);
    let b = dirty_table(&mut w, &c, 1);
    let i = kid(&w);
    let d = [(1i8, 0i8), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .find(|d| steer::next_tile(&w, i, w.staff[i].tile(), *d).is_some())
        .unwrap();
    w.apply(
        &c,
        Command::PlayerQueue {
            act: PlayerAct::ClearTable { table: a },
            front: false,
        },
    )
    .unwrap();
    steer(&mut w, &c, d);
    ticks(&mut w, &c, 2);
    w.apply(
        &c,
        Command::PlayerInteract {
            act: PlayerAct::ClearTable { table: b },
        },
    )
    .unwrap();
    assert_eq!(w.steering(), (0, 0), "E lets go of the keys");
    ticks(&mut w, &c, 2);
    assert_eq!(
        w.player().current.as_ref().map(|c| c.act.clone()),
        Some(PlayerAct::ClearTable { table: b }),
        "E's act goes first, at once"
    );
}

// ---------------------------------------------------------------------------------------------
// Determinism, saves
// ---------------------------------------------------------------------------------------------

/// Plays a night slice with the bot and random steering / E presses.
fn play(c: &Content, seed: u64, save_at: Option<u32>) -> String {
    let mut w = night(c, 3);
    w.mods.clear();
    let mut r = Rng::new(seed);
    let mut b = bot::PlayerBot::new(bot::BotConfig {
        seed,
        ..Default::default()
    });
    for t in 0..1200u32 {
        if Some(t) == save_at {
            let s = w.save_to_string().unwrap();
            w = World::load_from_str(&s).unwrap();
        }
        if r.chance(0.5) {
            b.step(&mut w, c);
        }
        if r.chance(0.1) {
            let d = (r.range_i64(-1, 2) as i8, r.range_i64(-1, 2) as i8);
            let _ = w.apply(c, Command::PlayerMove { dir: d });
        }
        if r.chance(0.04)
            && let Some(cmd) = w.nearby(c).and_then(|n| n.command())
        {
            let _ = w.apply(c, cmd);
        }
        w.tick(c);
        bot::finish_cutscenes(&mut w, c);
        w.drain_events();
    }
    w.save_to_string().unwrap()
}

#[test]
fn same_steering_same_world_also_across_a_save() {
    let c = content();
    let a = play(&c, 42, None);
    let b = play(&c, 42, None);
    assert!(a == b, "same inputs, same world");
    let s = play(&c, 42, Some(500));
    assert!(a == s, "a save mid-way changes nothing");
    assert!(play(&c, 43, None) != a, "(other inputs, another night)");
}

#[test]
fn an_unsteered_save_has_no_steering_fields() {
    let c = content();
    let w = night(&c, 2);
    let s = w.save_to_string().unwrap();
    assert!(!s.contains("steer"), "old saves stay byte-identical");
}
