//! QA chaos testing (tests only): world invariants checked after every step, and a command
//! fuzzer that throws random — mostly valid-looking, sometimes absurd — commands at the world
//! at random times. Every command must either succeed or return a `CommandError`; it must never
//! panic or leave the world inconsistent.
//!
//! The screen-level harness (`screens::game::tests_chaos`) drives `GameScreen` with the same
//! invariants. Heavy runs are `#[ignore]`d:
//! `CHAOS_SEEDS=6 CHAOS_NIGHTS=30 cargo test --release --lib chaos_heavy -- --ignored --nocapture`
//! (`CHAOS_SEED0` shifts the seeds, `CHAOS_STEPS` sets the raw fuzz length).

use super::grid::{footprint, is_rug_layer};
use super::rng::Rng;
use super::*;
use crate::content::handson::{Duty, MiniGameKind, TipSplit};
use crate::content::{BuildCategory, Content, Job};

/// Agents stuck longer than this many ticks count as a violation (the sim's own safety resets
/// guests at 600 and staff at 300 — both should never trigger).
pub(crate) const STUCK_LIMIT: u32 = 300;

/// Hard invariants of a world. Returns the broken ones (empty = fine).
pub(crate) fn check(w: &World, c: &Content) -> Vec<String> {
    let mut bad: Vec<String> = Vec::new();
    macro_rules! fail {
        ($($t:tt)*) => { bad.push(format!($($t)*)) };
    }
    // ── money and other numbers ──
    let e = &w.econ;
    for (n, v) in [
        ("money", e.money),
        ("debt", e.debt),
        ("supplies", e.supplies),
        ("rent", e.rent),
    ] {
        if v.abs() > 1_000_000_000 {
            fail!("{n} out of range: {v}");
        }
    }
    if !e.reputation.is_finite() || !(-1.0..=101.0).contains(&e.reputation) {
        fail!("reputation {}", e.reputation);
    }
    if !w.dirt.is_finite() {
        fail!("dirt {}", w.dirt);
    }
    if w.tick > NIGHT_TICKS {
        fail!("tick {} past the night", w.tick);
    }
    if w.phase == Phase::Day && w.prep_left != 0 {
        fail!("day with prep_left {}", w.prep_left);
    }
    // ── positions ──
    let t = TILE;
    let (gw, gh) = (w.grid.width as f32 * t, w.grid.height as f32 * t);
    let inside = |p: (f32, f32)| {
        p.0.is_finite()
            && p.1.is_finite()
            && p.0 > -2.0 * t
            && p.1 > -2.0 * t
            && p.0 < gw + 2.0 * t
            && p.1 < gh + 2.0 * t
    };
    for s in &w.staff {
        if !inside(s.pos) {
            fail!("staff {} at {:?}", s.id, s.pos);
        }
        for (n, v) in [
            ("energy", s.energy),
            ("mood", s.mood),
            ("hunger", s.hunger),
            ("work_left", s.work_left),
            ("hours_week", s.hours_week),
            ("toil", s.toil_hours),
        ] {
            if !v.is_finite() {
                fail!("staff {} {n} = {v}", s.id);
            }
        }
        if s.stuck_ticks > STUCK_LIMIT {
            fail!(
                "staff {} stuck >{STUCK_LIMIT} ticks at {:?}",
                s.id,
                s.tile()
            );
        }
    }
    if w.tonight.stuck_resets > 0 {
        fail!("stuck resets tonight: {}", w.tonight.stuck_resets);
    }
    // ── guests, parties, items, queues ──
    for g in &w.guests {
        if !inside(g.pos) {
            fail!("guest {} at {:?}", g.id, g.pos);
        }
        if g.stuck_ticks > STUCK_LIMIT {
            let t = g.tile();
            let around: String = (-1..=1)
                .flat_map(|dy| (-1..=1).map(move |dx| (dx, dy)))
                .map(|(dx, dy)| {
                    let (x, y) = (t.0 + dx, t.1 + dy);
                    if w.grid.is_taken(x, y) {
                        'S'
                    } else if w.grid.walkable(x, y) {
                        '.'
                    } else {
                        '#'
                    }
                })
                .collect();
            let party = w.parties.iter().find(|p| p.id == g.party).map(|p| p.state);
            fail!(
                "guest {} stuck >{STUCK_LIMIT} ticks at {t:?} | {:?}, {party:?}, goal {:?}, \
                 seat {:?}, path {:?}, around {around}",
                g.id,
                g.pose,
                g.goal,
                g.seat,
                g.path.iter().take(3).collect::<Vec<_>>()
            );
        }
        if !w.parties.iter().any(|p| p.id == g.party) {
            fail!("guest {} of a missing party {}", g.id, g.party);
        }
        if let Some((uid, k)) = g.seat {
            match w.furniture.iter().find(|f| f.uid == uid) {
                None => fail!("guest {} on a missing seat {uid}", g.id),
                Some(f) => {
                    if f.seat_by.get(k as usize).copied().flatten() != Some(g.id) {
                        fail!(
                            "guest {} sits on {uid}/{k} but the seat says {:?}",
                            g.id,
                            f.seat_by.get(k as usize)
                        );
                    }
                }
            }
        }
    }
    let staff_ok = |id: u32| w.staff.iter().any(|s| s.id == id);
    for p in &w.parties {
        for m in &p.members {
            match w.guests.iter().find(|g| g.id == *m) {
                None if p.state != PartyState::Leaving => {
                    fail!("party {} ({:?}) member {m} missing", p.id, p.state)
                }
                None => {}
                Some(g) if g.party != p.id => fail!("guest {m} in two parties",),
                _ => {}
            }
        }
        for i in &p.items {
            if !w.items.iter().any(|x| x.id == *i) {
                fail!("party {} item {i} missing", p.id);
            }
        }
        for (n, v) in [
            ("patience", p.patience),
            ("patience_max", p.patience_max),
            ("satisfaction", p.satisfaction),
            ("stay_left", p.stay_left),
            ("wait_clock", p.wait_clock),
            ("spend_mul", p.spend_mul),
        ] {
            if !v.is_finite() {
                fail!("party {} {n} = {v}", p.id);
            }
        }
        if let Some(s) = p.order_claim
            && !staff_ok(s)
        {
            fail!("party {} claimed by missing staff {s}", p.id);
        }
        if let Some(r) = p.register
            && !w.furniture.iter().any(|f| f.uid == r)
        {
            fail!("party {} at a missing register {r}", p.id);
        }
    }
    for i in &w.items {
        // served / cancelled items stay as the night's history after their party left
        let done = matches!(i.state, ItemState::Served | ItemState::Cancelled);
        if !done && !w.parties.iter().any(|p| p.id == i.party) {
            fail!(
                "item {} ({:?}) of a missing party {}",
                i.id,
                i.state,
                i.party
            );
        }
        if !done && !w.guests.iter().any(|g| g.id == i.guest) {
            fail!(
                "item {} ({:?}) for a missing guest {}",
                i.id,
                i.state,
                i.guest
            );
        }
        if !i.prep_left.is_finite() || !i.quality.is_finite() {
            fail!("item {} prep {} q {}", i.id, i.prep_left, i.quality);
        }
        // (`by` stays on a served item as who made it; a fired maker is fine then)
        if let Some(s) = i.by
            && !done
            && !staff_ok(s)
        {
            fail!("item {} ({:?}) by missing staff {s}", i.id, i.state);
        }
    }
    for (reg, q) in &w.queues {
        // (an emptied line may keep its key)
        if !q.is_empty() && !w.furniture.iter().any(|f| f.uid == *reg) {
            fail!("queue at a missing register {reg}");
        }
        for (k, pid) in q.iter().enumerate() {
            if !w.parties.iter().any(|p| p.id == *pid) {
                fail!("queue {reg} holds a missing party {pid}");
            }
            if q[..k].contains(pid) {
                fail!("queue {reg} holds party {pid} twice");
            }
        }
    }
    if w.phase == Phase::Day && (!w.parties.is_empty() || !w.guests.is_empty()) {
        fail!(
            "day with {} parties / {} guests inside",
            w.parties.len(),
            w.guests.len()
        );
    }
    // ── furniture and the grid ──
    for (k, f) in w.furniture.iter().enumerate() {
        if w.furniture[..k].iter().any(|o| o.uid == f.uid) {
            fail!("furniture uid {} twice", f.uid);
        }
        let Some(def) = c.furniture_def(&f.def) else {
            fail!("furniture {} has an unknown def {}", f.uid, f.def);
            continue;
        };
        if def.category == BuildCategory::Floors {
            fail!("a floor {} placed as furniture", f.def);
            continue;
        }
        if !f.dirt.is_finite() || !f.wear.is_finite() {
            fail!("furniture {} dirt {} wear {}", f.uid, f.dirt, f.wear);
        }
        let rug = is_rug_layer(def);
        for (x, y) in footprint(def, f.x, f.y, f.rot) {
            let Some(i) = w.grid.idx(x, y) else {
                fail!("furniture {} ({}) out of the grid at {x},{y}", f.uid, f.def);
                continue;
            };
            let layer = if rug {
                w.grid.rugs[i]
            } else {
                w.grid.objects[i]
            };
            if layer != Some(f.uid) {
                fail!(
                    "grid at {x},{y} says {layer:?}, furniture {} ({}) is there",
                    f.uid,
                    f.def
                );
            }
        }
        if f.seat_by.len() != def.seats as usize && def.seats > 0 {
            fail!(
                "furniture {} has {} seat slots for {} seats",
                f.uid,
                f.seat_by.len(),
                def.seats
            );
        }
        for (k, s) in f.seat_by.iter().enumerate() {
            if let Some(g) = s {
                match w.guests.iter().find(|x| x.id == *g) {
                    None => fail!("seat {}/{k} held by a missing guest {g}", f.uid),
                    Some(x) if x.seat != Some((f.uid, k as u8)) => {
                        fail!(
                            "seat {}/{k} held by guest {g} who sits at {:?}",
                            f.uid,
                            x.seat
                        )
                    }
                    _ => {}
                }
            }
        }
    }
    for y in 0..w.grid.height {
        for x in 0..w.grid.width {
            let i = w.grid.idx(x, y).unwrap();
            for (layer, rug) in [(w.grid.objects[i], false), (w.grid.rugs[i], true)] {
                let Some(uid) = layer else { continue };
                let Some(f) = w.furniture.iter().find(|f| f.uid == uid) else {
                    fail!("grid at {x},{y} holds a removed piece {uid}");
                    continue;
                };
                let Some(def) = c.furniture_def(&f.def) else {
                    continue;
                };
                if is_rug_layer(def) != rug || !footprint(def, f.x, f.y, f.rot).contains(&(x, y)) {
                    fail!("grid at {x},{y} holds {uid} ({}) which isn't there", f.def);
                }
            }
        }
    }
    // the walk masks match a fresh rebuild (when they are built)
    let mut fresh = w.grid.clone();
    fresh.rebuild(&w.furniture, c);
    let built = (0..w.grid.width).any(|x| (0..w.grid.height).any(|y| w.grid.walkable(x, y)));
    if built {
        'outer: for y in 0..w.grid.height {
            for x in 0..w.grid.width {
                if fresh.walkable(x, y) != w.grid.walkable(x, y)
                    || fresh.guest_ok(x, y) != w.grid.guest_ok(x, y)
                {
                    fail!("stale walk mask at {x},{y}");
                    break 'outer;
                }
            }
        }
    }
    // ── Karolina ──
    let pl = &w.hands.player;
    if pl.queue.len() > PLAYER_QUEUE_MAX {
        fail!("Karolina's queue {} > {PLAYER_QUEUE_MAX}", pl.queue.len());
    }
    if w.staff.iter().filter(|s| s.owner).count() != 1 {
        fail!("{} owners", w.staff.iter().filter(|s| s.owner).count());
    }
    let acts: Vec<&PlayerAct> = pl
        .current
        .iter()
        .map(|a| &a.act)
        .chain(pl.queue.iter())
        .collect();
    if w.phase == Phase::Day && pl.current.is_some() {
        fail!("Karolina busy by day: {:?}", pl.current);
    }
    let piece = |uid: u32| w.furniture.iter().any(|f| f.uid == uid);
    // (her dishes on the way to the sink: the table may be gone by then, that's fine)
    let carrying_dishes = |a: &PlayerAct| {
        pl.current
            .as_ref()
            .is_some_and(|c| &c.act == a && c.stage >= 2)
    };
    for a in acts {
        let ok = match a {
            PlayerAct::ClearTable { .. } if carrying_dishes(a) => true,
            PlayerAct::TakeOrder { party } => w.parties.iter().any(|p| p.id == *party),
            PlayerAct::Brew { item, extra } | PlayerAct::Serve { item, extra } => {
                std::iter::once(item)
                    .chain(extra.iter())
                    .all(|i| w.items.iter().any(|x| x.id == *i))
            }
            PlayerAct::ClearTable { table } | PlayerAct::CollectTip { table } => piece(*table),
            PlayerAct::Wash { sink } => piece(*sink),
            PlayerAct::Use { uid, .. } => piece(*uid),
            PlayerAct::Checklist { id } => w.app.checklist.iter().any(|x| x.id == *id),
            PlayerAct::Mop { tile } | PlayerAct::GoTo { tile } => w.grid.in_bounds(tile.0, tile.1),
            PlayerAct::PlayRecord { .. } => true,
        };
        if !ok {
            fail!("Karolina's act points at nothing: {a:?}");
        }
    }
    if let Some(cur) = &pl.current
        && (!cur.work_left.is_finite() || !cur.work_total.is_finite())
    {
        fail!("current act work {} / {}", cur.work_left, cur.work_total);
    }
    // ── shared work: one holder at a time (sim::claims) ──
    if let Some(x) = super::claims::clashes(w).first() {
        fail!("work done twice: {x}");
    }
    // ── the app ──
    for (k, x) in w.app.checklist.iter().enumerate() {
        if w.app.checklist[..k].iter().any(|o| o.id == x.id) {
            fail!("checklist item {} twice", x.id);
        }
        if let Some(s) = x.claim
            && !staff_ok(s)
        {
            fail!("checklist {} claimed by missing staff {s}", x.id);
        }
        if x.done && x.claim.is_some() {
            fail!("checklist {} done and still claimed", x.id);
        }
    }
    // ── story ──
    if w.story.ending.is_some() && w.mode == Mode::Story && w.night < 30 && !w.closed {
        fail!(
            "ending {:?} on night {} without closing",
            w.story.ending,
            w.night
        );
    }
    if let Some(p) = &w.story.playing
        && c.cutscene(p).is_none()
    {
        fail!("playing an unknown cutscene {p}");
    }
    // ── journal & photos ──
    let nights: Vec<u32> = w.journal().iter().map(|j| j.night).collect();
    for (k, n) in nights.iter().enumerate() {
        if nights[..k].contains(n) {
            fail!("journal has night {n} twice");
        }
        if *n > w.night {
            fail!("journal has a future night {n}");
        }
    }
    let per = super::signature::data().photo.per_night;
    for n in 1..=w.night {
        let k = w.photos().iter().filter(|p| p.night == n).count() as u32;
        if k > per {
            fail!("night {n}: {k} extra photos (max {per})");
        }
    }
    bad.extend(people(w, c));
    bad
}

/// The tile a seat is drawn on (`guests::seats`: the piece's footprint, seat by seat).
fn seat_tile(w: &World, c: &Content, uid: u32, idx: u8) -> Option<(i32, i32)> {
    let f = w.furniture.iter().find(|f| f.uid == uid)?;
    let d = c.furniture_def(&f.def)?;
    Some(
        footprint(d, f.x, f.y, f.rot)
            .get(idx as usize)
            .copied()
            .unwrap_or((f.x, f.y)),
    )
}

/// Where people are (checked every tick by the chaos runs): nobody walks through furniture,
/// a path never jumps, and nobody is drawn on a seat someone else sits on.
///
/// * A guest stands on a walkable tile, or on their own seat (sitting down / seated), or on
///   a seat tile they just stood up from (leaving).
/// * Staff stand on walkable tiles (they work from the access tiles).
/// * Every path step is a neighbour of the one before (8-way), starting next to the walker.
/// * Nobody else stands on the tile of a seated guest.
pub(crate) fn people(w: &World, c: &Content) -> Vec<String> {
    let mut out = Vec::new();
    let seat_tiles: Vec<(i32, i32)> = w
        .furniture
        .iter()
        .flat_map(|f| {
            let n = f.seat_by.len() as u8;
            (0..n).filter_map(move |k| seat_tile(w, c, f.uid, k))
        })
        .collect();
    let mut sitting: Vec<(u32, (i32, i32))> = Vec::new();
    for g in &w.guests {
        let t = g.tile();
        let own = g.seat.and_then(|(u, k)| seat_tile(w, c, u, k));
        if g.pose == Pose::Sit
            && let Some(st) = own
        {
            if st != t {
                out.push(format!(
                    "guest {} sits at {t:?}, their seat is at {st:?}",
                    g.id
                ));
            }
            sitting.push((g.id, st));
        }
        let ok = w.grid.walkable(t.0, t.1)
            || own == Some(t)
            || (g.seat.is_none() && seat_tiles.contains(&t));
        if !ok {
            out.push(format!(
                "guest {} walks through furniture at {t:?} | {:?}, goal {:?}, seat {:?}, path {:?}, \
                 tick {}, around {}",
                g.id,
                g.pose,
                g.goal,
                g.seat,
                g.path.iter().take(4).collect::<Vec<_>>(),
                w.tick,
                around(w, t)
            ));
        }
        if let Some(j) = jump(t, &g.path) {
            out.push(format!(
                "guest {} path jumps {j} ({:?}, goal {:?})",
                g.id, g.pose, g.goal
            ));
        }
    }
    for s in w.staff.iter().filter(|s| s.present()) {
        let t = s.tile();
        if !w.grid.walkable(t.0, t.1) {
            out.push(format!(
                "staff {} walks through furniture at {t:?} | {}, task {:?}, path {:?}, pos {:?}, \
                 tick {}, around {}",
                s.id,
                staff_doing(w, s),
                s.task.as_ref().map(|t| t.id()),
                s.path.iter().take(4).collect::<Vec<_>>(),
                s.pos,
                w.tick,
                around(w, t)
            ));
        }
        if let Some(j) = jump(t, &s.path) {
            out.push(format!("staff {} path jumps {j}", s.id));
        }
    }
    for (gid, st) in &sitting {
        for g in w.guests.iter().filter(|g| g.id != *gid) {
            if g.tile() == *st {
                out.push(format!(
                    "guest {} stands on the seat {st:?} guest {gid} sits on | {:?}, seat {:?}, \
                     goal {:?}, path {:?}, pos {:?}, tick {}, around {}",
                    g.id,
                    g.pose,
                    g.seat,
                    g.goal,
                    g.path.iter().take(4).collect::<Vec<_>>(),
                    g.pos,
                    w.tick,
                    around(w, *st)
                ));
            }
        }
        for s in w.staff.iter().filter(|s| s.present()) {
            if s.tile() == *st {
                out.push(format!(
                    "staff {} stands on the seat {st:?} guest {gid} sits on | {}, task {:?}, \
                     path {:?}, pos {:?}, tick {}, around {}",
                    s.id,
                    staff_doing(w, s),
                    s.task.as_ref().map(|t| t.id()),
                    s.path.iter().take(4).collect::<Vec<_>>(),
                    s.pos,
                    w.tick,
                    around(w, *st)
                ));
            }
        }
    }
    out
}

/// The 5×5 tiles around `t`, row by row: `@` = `t`, `S` = a taken seat, `s` = a free
/// seat, `,` = staff-only floor, `.` = floor, `#` = not walkable.
fn around(w: &World, t: (i32, i32)) -> String {
    let mut rows = Vec::new();
    for y in t.1 - 2..=t.1 + 2 {
        let row: String = (t.0 - 2..=t.0 + 2)
            .map(|x| {
                if (x, y) == t {
                    '@'
                } else if w.grid.is_taken(x, y) {
                    'S'
                } else if w.grid.is_seat(x, y) {
                    's'
                } else if !w.grid.walkable(x, y) {
                    '#'
                } else if w.grid.room(x, y).staff_only() {
                    ','
                } else {
                    '.'
                }
            })
            .collect();
        rows.push(row);
    }
    rows.join("/")
}

/// The whole floor (as [`around`]; `R` = a register, `Q` = its line, `o` = a to-go spot,
/// `d` = the door) — `CHAOS_MAP=1` prints it with the first violation of a run.
pub(crate) fn map(w: &World, c: &Content) -> String {
    let regs = guests::registers(w, c);
    let mut line = Vec::new();
    let mut togo = Vec::new();
    for (r, _) in &regs {
        line.extend(guests::queue_slots(w, c, *r));
        togo.extend(super::to_go::spots(w, c, *r));
    }
    let mut rows = Vec::new();
    for y in 0..w.grid.height {
        let row: String = (0..w.grid.width)
            .map(|x| {
                let t = (x, y);
                if regs.iter().any(|(u, _)| w.grid.object_at(x, y) == Some(*u)) {
                    'R'
                } else if w.grid.tile(x, y).is_some_and(|i| i.kind == TileKind::Door) {
                    'd'
                } else if line.contains(&t) {
                    'Q'
                } else if togo.contains(&t) {
                    'o'
                } else if w.grid.is_taken(x, y) {
                    'S'
                } else if w.grid.is_seat(x, y) {
                    's'
                } else if !w.grid.walkable(x, y) {
                    '#'
                } else if w.grid.room(x, y).staff_only() {
                    ','
                } else {
                    '.'
                }
            })
            .collect();
        rows.push(row);
    }
    rows.join("\n")
}

/// Who a staff member is and what they do (Karolina: her current act), for the messages.
fn staff_doing(w: &World, s: &Staff) -> String {
    if s.owner {
        let act = w.hands.player.current.as_ref();
        format!(
            "Karolina, act {:?} stage {:?}",
            act.map(|a| &a.act),
            act.map(|a| a.stage)
        )
    } else {
        format!("helper {:?}", s.status)
    }
}

/// The first non-neighbouring step of a path walked from `from`, if any.
fn jump(from: (i32, i32), path: &[(i32, i32)]) -> Option<String> {
    let mut prev = from;
    for &p in path {
        if grid::chebyshev(prev, p) > 1 {
            return Some(format!("{prev:?} → {p:?}"));
        }
        prev = p;
    }
    None
}

// ---------------------------------------------------------------------------------------------
// The command fuzzer
// ---------------------------------------------------------------------------------------------

fn pick_str(r: &mut Rng, v: &[String]) -> String {
    r.pick(v).cloned().unwrap_or_else(|| "nope".into())
}

/// Some id from `v`, or now and then a junk one.
fn some_id(r: &mut Rng, v: &[u32]) -> u32 {
    if v.is_empty() || r.chance(0.1) {
        *r.pick(&[0, 1, 7, 999, u32::MAX]).unwrap()
    } else {
        *r.pick(v).unwrap()
    }
}

fn some_f32(r: &mut Rng) -> f32 {
    if r.chance(0.08) {
        *r.pick(&[
            -1.0,
            0.0,
            1.0,
            2.0,
            1e9,
            -1e9,
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
        ])
        .unwrap()
    } else {
        r.f32()
    }
}

fn some_tile(r: &mut Rng, w: &World) -> (i32, i32) {
    if r.chance(0.05) {
        *r.pick(&[(-1, -1), (999, 3), (i32::MAX, i32::MIN), (0, 0)])
            .unwrap()
    } else {
        (
            r.range_i64(0, w.grid.width as i64) as i32,
            r.range_i64(0, w.grid.height as i64) as i32,
        )
    }
}

/// A random player act, targets taken from the world.
pub(crate) fn random_act(w: &World, c: &Content, r: &mut Rng) -> PlayerAct {
    let parties: Vec<u32> = w.parties.iter().map(|p| p.id).collect();
    let items: Vec<u32> = w.items.iter().map(|i| i.id).collect();
    let uids: Vec<u32> = w.furniture.iter().map(|f| f.uid).collect();
    let checks: Vec<String> = c.app.checklists.iter().map(|x| x.id.clone()).collect();
    match r.index(10) {
        0 => PlayerAct::TakeOrder {
            party: some_id(r, &parties),
        },
        1 => PlayerAct::Serve {
            item: some_id(r, &items),
            extra: if r.chance(0.2) {
                vec![some_id(r, &items)]
            } else {
                Vec::new()
            },
        },
        2 => PlayerAct::ClearTable {
            table: some_id(r, &uids),
        },
        3 => PlayerAct::Wash {
            sink: some_id(r, &uids),
        },
        4 => PlayerAct::Mop {
            tile: some_tile(r, w),
        },
        5 => PlayerAct::CollectTip {
            table: some_id(r, &uids),
        },
        6 => PlayerAct::Checklist {
            id: pick_str(r, &checks),
        },
        7 => PlayerAct::PlayRecord {
            record: r
                .pick(&w.records_unlocked().iter().cloned().collect::<Vec<_>>())
                .cloned(),
            repeat: r.chance(0.5),
        },
        8 => PlayerAct::GoTo {
            tile: some_tile(r, w),
        },
        _ => PlayerAct::Use {
            uid: some_id(r, &uids),
            action: *r.pick(&super::equipment::EquipAction::ALL).unwrap(),
        },
    }
}

/// A random command; arguments mostly from the world, sometimes junk.
pub(crate) fn random_command(w: &World, c: &Content, r: &mut Rng) -> Command {
    let staff: Vec<u32> = w.staff.iter().map(|s| s.id).collect();
    let helpers: Vec<u32> = w.staff.iter().filter(|s| !s.owner).map(|s| s.id).collect();
    let cands: Vec<u32> = w.candidates.iter().map(|x| x.id).collect();
    let uids: Vec<u32> = w.furniture.iter().map(|f| f.uid).collect();
    let guests: Vec<u32> = w.guests.iter().map(|g| g.id).collect();
    let leaders: Vec<u32> = w.parties.iter().filter_map(|p| p.leader()).collect();
    let who = |r: &mut Rng| {
        if !leaders.is_empty() && r.chance(0.7) {
            *r.pick(&leaders).unwrap()
        } else {
            some_id(r, &guests)
        }
    };
    let defs: Vec<String> = c.furniture.iter().map(|d| d.id.clone()).collect();
    let floors: Vec<String> = c
        .furniture
        .iter()
        .filter(|d| d.category == BuildCategory::Floors)
        .map(|d| d.id.clone())
        .collect();
    let menu: Vec<String> = c.menu.iter().map(|m| m.id.clone()).collect();
    let recipes: Vec<String> = c
        .handson
        .recipes
        .recipes
        .iter()
        .map(|x| x.id.clone())
        .collect();
    let ingredients: Vec<String> = c
        .handson
        .recipes
        .ingredients
        .iter()
        .map(|x| x.id.clone())
        .collect();
    let themes: Vec<String> = c
        .handson
        .decor
        .themes
        .iter()
        .map(|x| x.id.clone())
        .collect();
    let records: Vec<String> = c
        .handson
        .records
        .records
        .iter()
        .map(|x| x.id.clone())
        .collect();
    let checks: Vec<String> = c.app.checklists.iter().map(|x| x.id.clone()).collect();
    let anns: Vec<String> = c.app.announcements.iter().map(|x| x.id.clone()).collect();
    let attic: Vec<String> = c.build.attic.pieces.iter().map(|x| x.id.clone()).collect();
    let moments: Vec<String> = w
        .active_moments()
        .iter()
        .map(|m| m.id.clone())
        .chain(c.handson.moments.iter().take(3).map(|m| m.id.clone()))
        .collect();
    let rot = r.range_u32(0, 6) as u8;
    let shift = *r
        .pick(&[Shift::Off, Shift::Full, Shift::Early, Shift::Late])
        .unwrap();
    let weekday = if r.chance(0.05) {
        200
    } else {
        r.range_u32(0, 7) as u8
    };
    match r.index(74) {
        0..=2 => {
            let (x, y) = some_tile(r, w);
            Command::PlaceFurniture {
                def: pick_str(r, &defs),
                x,
                y,
                rot,
            }
        }
        3 => {
            let (x, y) = some_tile(r, w);
            Command::MoveFurniture {
                uid: some_id(r, &uids),
                x,
                y,
                rot,
            }
        }
        4 => Command::RotateFurniture {
            uid: some_id(r, &uids),
        },
        5 => Command::RemoveFurniture {
            uid: some_id(r, &uids),
        },
        6 => {
            let (x, y) = some_tile(r, w);
            Command::SetFloor {
                x,
                y,
                floor: pick_str(r, &floors),
            }
        }
        7 => Command::SetFurnitureOn {
            uid: some_id(r, &uids),
            on: r.chance(0.5),
        },
        8 => Command::Hire {
            candidate: some_id(r, &cands),
        },
        9 => Command::Fire {
            staff: some_id(r, &staff),
        },
        10 => Command::SetPriority {
            staff: some_id(r, &staff),
            job: *r.pick(&Job::ALL).unwrap(),
            priority: r.range_u32(0, 6) as u8,
        },
        11 | 12 => Command::SetShift {
            staff: some_id(r, &staff),
            weekday,
            shift,
        },
        13 => Command::AutoSchedule,
        14 => Command::PublishSchedule,
        15 => Command::AnswerSwap {
            id: some_id(r, &w.app.swaps.iter().map(|s| s.id).collect::<Vec<_>>()),
            approve: r.chance(0.5),
        },
        16 => Command::AnswerTimeOff {
            id: some_id(r, &w.app.time_off.iter().map(|s| s.id).collect::<Vec<_>>()),
            approve: r.chance(0.5),
        },
        17 => Command::RequestTimeOff {
            staff: some_id(r, &staff),
            kind: *r.pick(&[LeaveKind::Vacation, LeaveKind::Toil]).unwrap(),
            weekday,
        },
        18 => match w.narrator.pending.first() {
            Some(p) if r.chance(0.8) => Command::AnswerEvent {
                event: p.event.clone(),
                choice: pick_str(r, &p.choices),
            },
            _ => Command::AnswerEvent {
                event: "nope".into(),
                choice: "nope".into(),
            },
        },
        19 => {
            let msg = r.pick(&w.app.inbox).cloned();
            let actions: Vec<String> = msg
                .as_ref()
                .and_then(|m| c.app.bot.iter().find(|b| b.id == m.template))
                .map(|b| b.actions.iter().map(|a| a.id.clone()).collect())
                .unwrap_or_default();
            Command::BotAction {
                msg: msg.map(|m| m.id).unwrap_or(999),
                action: pick_str(r, &actions),
            }
        }
        20 => Command::CutsceneFinished {
            id: w
                .story
                .playing
                .clone()
                .unwrap_or_else(|| pick_str(r, &["opening".to_string()])),
            choices: (0..r.index(4)).map(|_| r.index(5)).collect(),
        },
        21 => Command::Talk { guest: who(r) },
        22 => Command::BuySupplies {
            amount: *r
                .pick(&[-500, 0, 100, 1000, 50_000, i64::MAX, i64::MIN])
                .unwrap(),
        },
        23 => Command::SetAutoRestock {
            target: *r
                .pick(&[None, Some(-1), Some(0), Some(3000), Some(i64::MAX)])
                .unwrap(),
        },
        24 => Command::SetPrice {
            item: pick_str(r, &menu),
            price: *r.pick(&[0, 1, 45, 120, 9999, u32::MAX]).unwrap(),
        },
        25 => Command::SetMenuItem {
            item: pick_str(r, &menu),
            on: r.chance(0.5),
        },
        26 => Command::RenameCafe {
            name: r
                .pick(&[
                    String::new(),
                    "Café Chaos".to_string(),
                    "x".repeat(500),
                    "  ".to_string(),
                    "Kavárna ř 🌙".to_string(),
                ])
                .cloned()
                .unwrap(),
        },
        27 | 28 => Command::TickChecklist {
            id: pick_str(r, &checks),
        },
        29 => Command::PostAnnouncement {
            id: pick_str(r, &anns),
            args: Vec::new(),
        },
        30 => Command::WriteHandover {
            text: if r.chance(0.5) {
                "Mléko došlo".into()
            } else {
                "z".repeat(3000)
            },
        },
        31 => Command::SetMusic { on: r.chance(0.5) },
        32 => Command::SendOnBreak {
            staff: some_id(r, &staff),
        },
        33 => Command::OpenDoors,
        34 | 35 => Command::PlayerTakeOrder { guest: who(r) },
        36 | 37 => Command::PlayerBrew {
            guest: who(r),
            recipe: if r.chance(0.6) {
                pick_str(
                    r,
                    &w.known_recipes()
                        .map(str::to_string)
                        .collect::<Vec<String>>(),
                )
            } else {
                pick_str(r, &recipes)
            },
            quality: some_f32(r),
            mode: if r.chance(0.5) {
                BrewMode::Quick
            } else {
                BrewMode::ByHand
            },
        },
        38 => Command::PlayerUsual {
            guest: who(r),
            quality: some_f32(r),
        },
        39 => Command::PlayerHandOff { guest: who(r) },
        40 => Command::PlayerCancel,
        41..=43 => Command::PlayerQueue {
            act: random_act(w, c, r),
            front: r.chance(0.3),
        },
        44 => Command::PlayerUnqueue { index: r.index(7) },
        45 => Command::PlayerWashProgress {
            sink: some_id(r, &uids),
            seconds: some_f32(r) * 3.0,
        },
        46 => Command::PlayerGreet { guest: who(r) },
        47 => Command::SetDuty {
            staff: some_id(r, &helpers),
            duty: *r.pick(&Duty::ALL).unwrap(),
            on: r.chance(0.6),
        },
        48 => Command::PlayRecord {
            id: if r.chance(0.2) {
                None
            } else {
                Some(pick_str(r, &records))
            },
            repeat: r.chance(0.5),
        },
        49 => Command::RecordFinished {
            id: pick_str(r, &records),
        },
        50 => Command::SetAssistWishes { on: r.chance(0.5) },
        51 => {
            let id = pick_str(r, &moments);
            let opts = w.moment_options(c, &id);
            Command::ResolveMoment {
                option: if opts.is_empty() || r.chance(0.1) {
                    "nope".into()
                } else {
                    pick_str(r, &opts)
                },
                id,
            }
        }
        52 => {
            let n = r.index(5);
            let pantry: Vec<String> = w.pantry().iter().cloned().collect();
            let src = if r.chance(0.8) { &pantry } else { &ingredients };
            let ings = (0..n).map(|_| pick_str(r, src)).collect();
            if r.chance(0.5) {
                Command::TryExperiment { ingredients: ings }
            } else {
                Command::Experiment { ingredients: ings }
            }
        }
        53 => Command::PickTipGoal {
            choice: pick_str(r, &["jar".into(), "crew".into(), "nope".into()]),
        },
        54 => Command::SetSpecials {
            recipes: (0..r.index(4)).map(|_| pick_str(r, &recipes)).collect(),
        },
        55 => Command::BuyTheme {
            id: pick_str(r, &themes),
        },
        56 => Command::SetTheme {
            id: pick_str(r, &themes),
        },
        57 => Command::PickPolaroid { index: r.index(6) },
        58 => Command::AutoFillRozpis,
        59 => Command::AnswerEarlyLeave {
            id: some_id(
                r,
                &w.early_leave_requests()
                    .iter()
                    .map(|x| x.id)
                    .collect::<Vec<_>>(),
            ),
            approve: r.chance(0.5),
        },
        60 => {
            let sug: Vec<String> = w
                .handover_suggestions(c)
                .iter()
                .map(|s| s.id.clone())
                .collect();
            Command::PickHandover {
                picks: (0..r.index(4)).map(|_| pick_str(r, &sug)).collect(),
            }
        }
        61 => Command::SetTipSplit {
            split: *r
                .pick(&[TipSplit::Cafe, TipSplit::Half, TipSplit::Crew])
                .unwrap(),
        },
        62 => match r.index(3) {
            0 => Command::PayBonus {
                staff: some_id(r, &staff),
            },
            1 => Command::GrantToil {
                staff: some_id(r, &staff),
                weekday,
            },
            _ => Command::SendPayroll {
                week: r.range_u32(0, 8),
            },
        },
        63 => {
            let recipe = pick_str(r, &recipes);
            if r.chance(0.5) {
                Command::PlayerBrewStart {
                    guest: who(r),
                    recipe,
                    mode: BrewMode::ByHand,
                }
            } else {
                Command::PlayerBrewQuality {
                    guest: who(r),
                    quality: some_f32(r),
                }
            }
        }
        64 => {
            // a group move (sometimes with junk / duplicate pieces)
            let uid = some_id(r, &uids);
            let to = some_tile(r, w);
            let mut moves = w.group_moves(c, uid, to, r.range_u32(0, 4) as u8);
            if r.chance(0.15)
                && let Some(m) = moves.first().cloned()
            {
                moves.push(m);
            }
            Command::MoveGroup { moves }
        }
        65 => Command::BuyAtticPiece {
            id: pick_str(r, &attic),
        },
        66 => Command::OpenBackRoom,
        67 => Command::UseEquipment {
            uid: some_id(r, &uids),
            action: *r.pick(&super::equipment::EquipAction::ALL).unwrap(),
        },
        68 => Command::EquipmentHold {
            seconds: some_f32(r) * 4.0,
        },
        69 => match r.index(3) {
            0 => Command::SendHome {
                staff: some_id(r, &staff),
            },
            1 => Command::WalkHome {
                route: pick_str(
                    r,
                    &[
                        "embankment".into(),
                        "park".into(),
                        "bakery".into(),
                        "tram".into(),
                        "nope".into(),
                    ],
                ),
            },
            _ => Command::StepOutBack,
        },
        70 => {
            let bases: Vec<String> = w
                .signature_bases(c)
                .iter()
                .map(|(b, _)| b.id.clone())
                .collect();
            let jars: Vec<String> = w.signature_jars().iter().map(|s| s.to_string()).collect();
            Command::ComposeSignature {
                base: pick_str(r, &bases),
                jars: (0..r.index(4)).map(|_| pick_str(r, &jars)).collect(),
                style: *r.pick(&MiniGameKind::ALL).unwrap(),
                name: pick_str(r, &["Noční vlak".into(), String::new(), "q".repeat(200)]),
            }
        }
        72 => Command::PlayerMove {
            dir: if r.chance(0.05) {
                (*r.pick(&[2i8, -7, i8::MAX, i8::MIN]).unwrap(), 1)
            } else if r.chance(0.35) {
                (0, 0)
            } else {
                (r.range_i64(-1, 2) as i8, r.range_i64(-1, 2) as i8)
            },
        },
        73 => match w.nearby(c).and_then(|n| n.command()) {
            Some(cmd) if r.chance(0.7) => cmd,
            _ => Command::PlayerInteract {
                act: random_act(w, c, r),
            },
        },
        _ => match r.index(5) {
            0 => Command::TakePhoto {
                focus: (some_f32(r) * 1900.0, some_f32(r) * 1100.0),
                width: some_f32(r) * 1200.0,
            },
            1 => {
                let st = w.stories_active().first().cloned();
                let answers: Vec<String> = st
                    .as_ref()
                    .and_then(|a| w.story_def(&a.story))
                    .map(|d| d.answers.iter().map(|x| x.id.clone()).collect())
                    .unwrap_or_default();
                Command::AnswerStory {
                    guest: st.map(|a| a.guest).unwrap_or_else(|| who(r)),
                    answer: pick_str(r, &answers),
                }
            }
            2 => Command::StormBlankets,
            3 => Command::StormSoup,
            _ => {
                if r.chance(0.5) {
                    Command::SetTutorial { on: r.chance(0.5) }
                } else {
                    Command::TutorialShown { id: "nope".into() }
                }
            }
        },
    }
}

/// Ends playing cutscenes with random (sometimes invalid) choices.
pub(crate) fn finish_cutscenes_random(w: &mut World, c: &Content, r: &mut Rng) {
    for _ in 0..64 {
        let Some(id) = w.playing_cutscene().map(str::to_string) else {
            return;
        };
        let choices = (0..r.index(4)).map(|_| r.index(5)).collect();
        if w.apply(c, Command::CutsceneFinished { id, choices })
            .is_err()
        {
            bot::finish_cutscenes(w, c);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Sim-level chaos runs
// ---------------------------------------------------------------------------------------------

/// What a run found.
#[derive(Debug, Default)]
pub(crate) struct Report {
    pub nights: u32,
    pub ticks: u64,
    pub commands: u64,
    pub ok: u64,
    pub errors: std::collections::BTreeMap<String, u64>,
    pub violations: Vec<String>,
    pub off_floor: u64,
    pub off_floor_examples: Vec<String>,
    pub save_loads: u32,
    /// Distinct violations by kind (numbers stripped), uncapped: the before/after counts.
    pub kinds: std::collections::BTreeMap<String, u64>,
    seen: std::collections::HashSet<String>,
}

/// A violation without its details (after ` | `): one incident (the same agent on the same
/// tile), however many checks saw it.
pub(crate) fn head_of(v: &str) -> &str {
    v.split(" | ").next().unwrap_or(v)
}

/// A violation's kind: the message up to its first detail, numbers stripped
/// ("staff # stands on the seat").
pub(crate) fn kind_of(v: &str) -> String {
    let head = head_of(v).split(['(', ':']).next().unwrap_or(v);
    let mut out = String::new();
    let mut digit = false;
    for ch in head.chars() {
        if ch.is_ascii_digit() {
            if !digit {
                out.push('#');
            }
            digit = true;
        } else {
            out.push(ch);
            digit = false;
        }
    }
    out.trim().to_string()
}

impl Report {
    fn violation(&mut self, at: &str, v: Vec<String>, last: &std::collections::VecDeque<String>) {
        for x in v {
            if !self.seen.insert(head_of(&x).to_string()) {
                continue;
            }
            *self.kinds.entry(kind_of(&x)).or_default() += 1;
            if self.violations.len() < 40 {
                let tail: Vec<&String> = last.iter().rev().take(6).collect();
                self.violations
                    .push(format!("{at} | recent: {tail:?} | {x}"));
            }
        }
    }
}

/// Plays `nights` nights of `w` with the player-bot, firing a random command now and then
/// (both phases), random undos, random story answers and save → load round trips (the loaded
/// copy must continue exactly like the original). Checks the invariants after every step.
pub(crate) fn sim_chaos(w: &mut World, c: &Content, seed: u64, nights: u32, rate: f32) -> Report {
    use std::collections::VecDeque;
    let mut r = Rng::new(seed ^ 0xC4A0_5EED);
    let mut rep = Report::default();
    let mut last: VecDeque<String> = VecDeque::new();
    let mut bot = bot::PlayerBot::new(bot::BotConfig {
        seed,
        ..Default::default()
    });
    let mut endings = 0u32;
    let start_night = w.night();
    let fire = |w: &mut World, r: &mut Rng, rep: &mut Report, last: &mut VecDeque<String>| {
        let cmd = if r.chance(0.04) {
            None
        } else {
            Some(random_command(w, c, r))
        };
        let label = format!(
            "n{} t{} {:?}",
            w.night(),
            w.tick_count(),
            cmd.as_ref()
                .map(|c| format!("{c:?}").chars().take(160).collect::<String>())
        );
        last.push_back(label);
        if last.len() > 12 {
            last.pop_front();
        }
        let res = match cmd {
            Some(cmd) => w.apply(c, cmd),
            None => w.undo(c),
        };
        rep.commands += 1;
        match res {
            Ok(()) => rep.ok += 1,
            Err(e) => {
                let k = format!("{e:?}");
                let k = k.split(['(', ' ']).next().unwrap_or("").to_string();
                *rep.errors.entry(k).or_insert(0) += 1;
            }
        }
    };
    while w.night() < start_night + nights && !w.is_closed() {
        // the day: a few random commands, the bot's plan, more chaos
        for _ in 0..r.range_u32(0, 12) {
            fire(w, &mut r, &mut rep, &mut last);
            let v = check(w, c);
            rep.violation("day", v, &last);
        }
        if r.chance(0.7) {
            bot.day(w, c);
        }
        for _ in 0..r.range_u32(0, 6) {
            fire(w, &mut r, &mut rep, &mut last);
        }
        if r.chance(0.5) {
            finish_cutscenes_random(w, c, &mut r);
        } else {
            bot.finish_scenes(w, c);
        }
        let v = check(w, c);
        rep.violation("day end", v, &last);
        if w.phase() == Phase::Day {
            if w.playing_cutscene().is_some() {
                rep.violation("day", vec!["a cutscene left playing".into()], &last);
                bot::finish_cutscenes(w, c);
            }
            w.start_night(c);
        }
        let night = w.night();
        let mut guard = 0u32;
        while w.phase() == Phase::Night && !w.is_closed() {
            if w.playing_cutscene().is_some() {
                if r.chance(0.5) {
                    finish_cutscenes_random(w, c, &mut r);
                } else {
                    bot.finish_scenes(w, c);
                }
            }
            if r.chance(0.85) {
                bot.step(w, c);
            }
            if r.chance(rate) {
                fire(w, &mut r, &mut rep, &mut last);
            }
            w.tick(c);
            rep.ticks += 1;
            for e in w.drain_events() {
                if matches!(e, SimEvent::EndingReached(_)) {
                    endings += 1;
                }
            }
            guard += 1;
            {
                let v = check(w, c);
                rep.violation(&format!("night {night} tick {}", w.tick_count()), v, &last);
                let off: Vec<String> = Vec::new();
                rep.off_floor += off.len() as u64;
                for o in off {
                    if rep.off_floor_examples.len() < 12 {
                        rep.off_floor_examples
                            .push(format!("night {night} t{} {o}", w.tick_count()));
                    }
                }
            }
            // a save → load round trip now and then: the copy continues identically
            if r.chance(0.0006) {
                rep.save_loads += 1;
                if let Some(v) = round_trip(w, c, 200 + r.index(400) as u32) {
                    rep.violation(&format!("night {night} save/load"), vec![v], &last);
                }
            }
            if guard > NIGHT_TICKS * 6 {
                rep.violation(
                    &format!("night {night}"),
                    vec![format!(
                        "the night never ends (tick {}, cutscene {:?}, prep {})",
                        w.tick_count(),
                        w.playing_cutscene(),
                        w.prep_left
                    )],
                    &last,
                );
                return rep;
            }
        }
        rep.nights += 1;
        if w.phase() == Phase::Day && !w.is_closed() && w.night() != night + 1 {
            rep.violation(
                "morning",
                vec![format!("night {night} → day {}", w.night())],
                &last,
            );
        }
        // a day save → load now and then
        if r.chance(0.2) {
            rep.save_loads += 1;
            if let Some(v) = round_trip_day(w, c) {
                rep.violation(&format!("day {} save/load", w.night()), vec![v], &last);
            }
        }
    }
    bot.finish_scenes(w, c);
    for e in w.drain_events() {
        if matches!(e, SimEvent::EndingReached(_)) {
            endings += 1;
        }
    }
    if endings > 1 {
        rep.violation("end", vec![format!("{endings} endings")], &last);
    }
    if w.mode() == Mode::Story && start_night + nights > 30 && w.ending().is_none() {
        rep.violation(
            "end",
            vec![format!("no ending after night {}", w.night())],
            &last,
        );
    }
    rep
}

/// Saves, loads twice and runs the loaded copy and the original `ticks` ticks side by side
/// (cutscenes finished the same way): their saves must match. `None` = fine.
pub(crate) fn round_trip(w: &mut World, c: &Content, ticks: u32) -> Option<String> {
    let text = match w.save_to_string() {
        Ok(t) => t,
        Err(e) => return Some(format!("save failed: {e}")),
    };
    let mut a = match World::load_from_str(&text) {
        Ok(a) => a,
        Err(e) => return Some(format!("load failed: {e}")),
    };
    let again = a.save_to_string().unwrap_or_default();
    if again != text {
        return Some(format!(
            "save → load → save differs at byte {}",
            first_diff(&text, &again)
        ));
    }
    let mut b = w.clone();
    for _ in 0..ticks {
        bot::finish_cutscenes(&mut a, c);
        bot::finish_cutscenes(&mut b, c);
        a.tick(c);
        b.tick(c);
    }
    let (sa, sb) = (a.save_to_string().unwrap(), b.save_to_string().unwrap());
    if sa != sb {
        let i = first_diff(&sa, &sb);
        let lo = i.saturating_sub(120);
        return Some(format!(
            "the loaded copy diverged after {ticks} ticks at byte {i}: …{}… vs …{}…",
            &sa[lo..(i + 80).min(sa.len())],
            &sb[lo..(i + 80).min(sb.len())]
        ));
    }
    None
}

fn round_trip_day(w: &mut World, c: &Content) -> Option<String> {
    let text = w.save_to_string().ok()?;
    let mut a = World::load_from_str(&text).ok()?;
    a.rebuild_caches(c);
    let mut b = w.clone();
    for x in [&mut a, &mut b] {
        bot::finish_cutscenes(x, c);
        x.start_night(c);
        for _ in 0..400 {
            bot::finish_cutscenes(x, c);
            x.tick(c);
        }
    }
    let (sa, sb) = (a.save_to_string().unwrap(), b.save_to_string().unwrap());
    (sa != sb).then(|| {
        let i = first_diff(&sa, &sb);
        let lo = i.saturating_sub(120);
        format!(
            "a day save diverged at byte {i}: …{}… vs …{}…",
            &sa[lo..(i + 80).min(sa.len())],
            &sb[lo..(i + 80).min(sb.len())]
        )
    })
}

fn first_diff(a: &str, b: &str) -> usize {
    let i = a
        .bytes()
        .zip(b.bytes())
        .position(|(x, y)| x != y)
        .unwrap_or(a.len().min(b.len()));
    // a char boundary for slicing
    let mut i = i;
    while i > 0 && (!a.is_char_boundary(i) || !b.is_char_boundary(i)) {
        i -= 1;
    }
    i
}

/// Raw command fuzz: only commands and ticks, no bot (the world mostly sits idle with chaos).
pub(crate) fn command_fuzz(w: &mut World, c: &Content, seed: u64, steps: u32) -> Report {
    let mut r = Rng::new(seed ^ 0xF022);
    let mut rep = Report::default();
    let mut last = std::collections::VecDeque::new();
    for step in 0..steps {
        let cmd = random_command(w, c, &mut r);
        let label = format!(
            "#{step} n{} {:?} t{} {}",
            w.night(),
            w.phase(),
            w.tick_count(),
            format!("{cmd:?}").chars().take(160).collect::<String>()
        );
        last.push_back(label);
        if last.len() > 12 {
            last.pop_front();
        }
        rep.commands += 1;
        match w.apply(c, cmd) {
            Ok(()) => rep.ok += 1,
            Err(e) => {
                let k = format!("{e:?}");
                let k = k.split(['(', ' ']).next().unwrap_or("").to_string();
                *rep.errors.entry(k).or_insert(0) += 1;
            }
        }
        let v = check(w, c);
        rep.violation(&format!("step {step}"), v, &last);
        // time passes in bursts; cutscenes end with random choices
        let n = r.range_u32(0, 40);
        for _ in 0..n {
            finish_cutscenes_random(w, c, &mut r);
            w.tick(c);
            rep.ticks += 1;
        }
        let _ = w.drain_events();
        if w.phase() == Phase::Day && r.chance(0.05) {
            finish_cutscenes_random(w, c, &mut r);
            w.start_night(c);
            rep.nights += 1;
        }
        if w.is_closed() {
            break;
        }
    }
    rep
}

pub(crate) fn content() -> Content {
    Content::load(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data")).expect("content")
}

pub(crate) fn print(name: &str, rep: &Report) {
    eprintln!(
        "[chaos] {name}: {} nights, {} ticks, {} commands ({} ok), {} save/loads, \
         off-floor {}, errors {:?}",
        rep.nights, rep.ticks, rep.commands, rep.ok, rep.save_loads, rep.off_floor, rep.errors
    );
    for v in &rep.off_floor_examples {
        eprintln!("    off floor: {v}");
    }
    for v in &rep.violations {
        eprintln!("  !! {v}");
    }
    for (k, n) in &rep.kinds {
        eprintln!("[tally] {n} × {k}");
    }
}

// ---------------------------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------------------------

fn world_for(c: &Content, kind: usize, seed: u64) -> World {
    match kind % 4 {
        0 => World::new_story_seeded(c, Difficulty::Cozy, seed),
        1 => World::new_story_seeded(c, Difficulty::Relaxed, seed),
        2 => World::new_story_seeded(c, Difficulty::Classic, seed),
        _ => World::new_sandbox(
            c,
            SandboxSetup {
                seed,
                difficulty: if seed.is_multiple_of(2) {
                    Difficulty::Cozy
                } else {
                    Difficulty::Classic
                },
                ..Default::default()
            },
        ),
    }
}

/// The owner saw guests walk through tables and two people on one seat (0.2.0): the plain
/// player-bot, no chaos, every tick checked.
#[test]
fn people_never_walk_through_furniture_or_share_a_seat() {
    let c = content();
    let mut bad = Vec::new();
    for (kind, seed) in [(0, 1u64), (3, 8), (3, 9)] {
        let mut w = world_for(&c, kind, seed);
        let mut bot = bot::PlayerBot::new(bot::BotConfig {
            seed,
            ..Default::default()
        });
        for _ in 0..3 {
            bot.day(&mut w, &c);
            bot.finish_scenes(&mut w, &c);
            w.start_night(&c);
            while w.phase() == Phase::Night {
                bot.finish_scenes(&mut w, &c);
                bot.step(&mut w, &c);
                w.tick(&c);
                for v in people(&w, &c) {
                    if bad.len() < 20 {
                        bad.push(format!(
                            "kind {kind} night {} {}: {v}",
                            w.night(),
                            w.clock_label()
                        ));
                    }
                }
            }
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
}

/// Quick default run (CI): a couple of nights of chaos per mode, and a raw command fuzz.
#[test]
fn chaos_quick_sim() {
    let c = content();
    let mut all = Vec::new();
    for kind in 0..4 {
        let mut w = world_for(&c, kind, 11 + kind as u64);
        let rep = sim_chaos(&mut w, &c, 11 + kind as u64, 2, 0.02);
        print(&format!("quick kind {kind}"), &rep);
        all.extend(rep.violations);
    }
    let mut w = world_for(&c, 3, 5);
    let rep = command_fuzz(&mut w, &c, 5, 1500);
    print("quick fuzz", &rep);
    all.extend(rep.violations);
    assert!(all.is_empty(), "{all:#?}");
}

fn env_u32(k: &str, d: u32) -> u32 {
    std::env::var(k)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(d)
}

/// Heavy: `CHAOS_SEEDS` seeds × every mode × `CHAOS_NIGHTS` nights
/// (`cargo test --release chaos_heavy_sim -- --ignored --nocapture`).
#[test]
#[ignore]
fn chaos_heavy_sim() {
    let c = content();
    let seeds = env_u32("CHAOS_SEEDS", 6);
    let nights = env_u32("CHAOS_NIGHTS", 30);
    let base = env_u32("CHAOS_SEED0", 100) as u64;
    let mut all = Vec::new();
    for s in 0..seeds as u64 {
        for kind in 0..4 {
            let seed = base + s * 7 + kind as u64;
            let mut w = world_for(&c, kind, seed);
            let rep = sim_chaos(&mut w, &c, seed, nights, 0.01);
            print(&format!("seed {seed} kind {kind}"), &rep);
            all.extend(
                rep.violations
                    .into_iter()
                    .map(|v| format!("seed {seed} kind {kind}: {v}")),
            );
        }
    }
    assert!(
        all.is_empty(),
        "{} violations:\n{}",
        all.len(),
        all.join("\n")
    );
}

/// Heavy raw fuzz: `CHAOS_SEEDS` × 20 000 random commands.
#[test]
#[ignore]
fn chaos_heavy_fuzz() {
    let c = content();
    let seeds = env_u32("CHAOS_SEEDS", 6);
    let steps = env_u32("CHAOS_STEPS", 20_000);
    let base = env_u32("CHAOS_SEED0", 100) as u64;
    let mut all = Vec::new();
    for s in 0..seeds as u64 {
        for kind in 0..4 {
            let seed = base + s * 13 + kind as u64;
            let mut w = world_for(&c, kind, seed);
            let rep = command_fuzz(&mut w, &c, seed, steps);
            print(&format!("fuzz seed {seed} kind {kind}"), &rep);
            all.extend(
                rep.violations
                    .into_iter()
                    .map(|v| format!("seed {seed} kind {kind}: {v}")),
            );
        }
    }
    assert!(
        all.is_empty(),
        "{} violations:\n{}",
        all.len(),
        all.join("\n")
    );
}

/// Chaos find: a party with no table to share (the "seats near each other" fallback) got the
/// best seat twice — two guests on one chair, the seat remembering only the last.
#[test]
fn a_party_without_a_table_never_shares_a_chair() {
    let c = content();
    let mut w = world_for(&c, 3, 40);
    // no tables: every party of two or more takes the fallback
    let tables: Vec<u32> = w
        .furniture
        .iter()
        .filter(|p| c.furniture_def(&p.def).is_some_and(|d| d.has_tag("table")))
        .map(|p| p.uid)
        .collect();
    assert!(!tables.is_empty());
    for uid in tables {
        w.apply(&c, Command::RemoveFurniture { uid }).unwrap();
    }
    w.start_night(&c);
    bot::finish_cutscenes(&mut w, &c);
    w.skip_prep(&c);
    w.debug_spawn_guests(&c, "students", 12, 2);
    let mut seated = 0;
    for _ in 0..600 {
        bot::finish_cutscenes(&mut w, &c);
        w.tick(&c);
        let seats: Vec<(u32, u8)> = w.guests.iter().filter_map(|g| g.seat).collect();
        for (k, s) in seats.iter().enumerate() {
            assert!(!seats[..k].contains(s), "two guests on seat {s:?}");
        }
        seated = seated.max(seats.len());
        let v: Vec<String> = check(&w, &c)
            .into_iter()
            .filter(|x| x.contains("seat") || x.contains("sits"))
            .collect();
        assert!(v.is_empty(), "{v:?}");
    }
    assert!(seated >= 2, "guests sat down ({seated})");
}

/// Chaos find: the register could be removed (or moved) at night while guests stood in its
/// line — they then waited at a till that wasn't there.
#[test]
fn a_register_with_a_line_stays_put() {
    let c = content();
    let mut w = world_for(&c, 3, 41);
    w.start_night(&c);
    bot::finish_cutscenes(&mut w, &c);
    w.skip_prep(&c);
    w.debug_spawn_guests(&c, "travelers", 8, 1);
    let mut guard = 0;
    while !w.queues.values().any(|q| !q.is_empty()) && guard < 3000 {
        bot::finish_cutscenes(&mut w, &c);
        w.tick(&c);
        guard += 1;
    }
    let (&reg, _) = w
        .queues
        .iter()
        .find(|(_, q)| !q.is_empty())
        .expect("someone queues at the register");
    assert_eq!(
        w.apply(&c, Command::RemoveFurniture { uid: reg }),
        Err(CommandError::InUse)
    );
    let p = w.furniture_by_uid(reg).unwrap().clone();
    assert_eq!(
        w.apply(
            &c,
            Command::MoveFurniture {
                uid: reg,
                x: p.x - 3,
                y: p.y,
                rot: p.rot
            }
        ),
        Err(CommandError::InUse)
    );
    assert_eq!(w.in_use_reason(reg), Some(InUseReason::Busy));
    assert!(check(&w, &c).is_empty(), "{:?}", check(&w, &c));
}

/// Chaos find: a queued act for a party that already went home kept its slot in Karolina's
/// queue (and a chip in the strip) until she reached it.
#[test]
fn acts_for_people_who_left_leave_her_queue() {
    let c = content();
    let mut w = world_for(&c, 3, 42);
    w.start_night(&c);
    bot::finish_cutscenes(&mut w, &c);
    w.skip_prep(&c);
    w.debug_spawn_guests(&c, "travelers", 3, 1);
    let mut guard = 0;
    while w.parties.len() < 2 && guard < 3000 {
        bot::finish_cutscenes(&mut w, &c);
        w.tick(&c);
        guard += 1;
    }
    let ids: Vec<u32> = w.parties.iter().map(|p| p.id).take(2).collect();
    assert_eq!(ids.len(), 2, "two parties came");
    // something long in her hands first, then two order takes behind it
    w.apply(
        &c,
        Command::PlayerQueue {
            act: PlayerAct::GoTo { tile: (2, 2) },
            front: false,
        },
    )
    .unwrap();
    for &party in &ids {
        super::guests::settle_now(&mut w, party);
        w.apply(
            &c,
            Command::PlayerQueue {
                act: PlayerAct::TakeOrder { party },
                front: false,
            },
        )
        .unwrap();
    }
    // the first party goes home (as if they had walked out of the door)
    let gone = ids[0];
    w.guests.retain(|g| g.party != gone);
    w.parties.retain(|p| p.id != gone);
    w.tick(&c);
    let q: Vec<&PlayerAct> = w.player().queue.iter().collect();
    assert!(
        !q.contains(&&PlayerAct::TakeOrder { party: gone }),
        "a stale act stays queued: {q:?}"
    );
    assert!(
        w.player().current.as_ref().map(|a| &a.act)
            == Some(&PlayerAct::TakeOrder { party: ids[1] })
            || q.contains(&&PlayerAct::TakeOrder { party: ids[1] }),
        "the other party's order is still hers: {q:?}"
    );
}

/// Owner report (0.2.0) / chaos find: a guest with no way to the goal walked there in a
/// straight line, through tables and the counter (`set_goal`'s old `vec![goal]` fallback).
/// Now they wait and try again.
#[test]
fn a_guest_with_no_way_waits_instead_of_walking_through_tables() {
    let c = content();
    let mut w = world_for(&c, 3, 43);
    w.start_night(&c);
    bot::finish_cutscenes(&mut w, &c);
    w.skip_prep(&c);
    w.debug_spawn_guests(&c, "travelers", 3, 1);
    let mut guard = 0;
    while !w.guests.iter().any(|g| g.tile().1 < 14) && guard < 3000 {
        bot::finish_cutscenes(&mut w, &c);
        w.tick(&c);
        guard += 1;
    }
    let gid = w
        .guests
        .iter()
        .find(|g| g.tile().1 < 14)
        .map(|g| g.id)
        .expect("a guest inside");
    // a floor tile walled in by buckets: no way there
    let target = (5, 3);
    for dy in -1..=1 {
        for dx in -1..=1 {
            if (dx, dy) != (0, 0)
                && let Some(i) = w.grid.idx(target.0 + dx, target.1 + dy)
            {
                w.grid.blocked[i] = true;
            }
        }
    }
    w.grid.rebuild(&w.furniture, &c);
    super::guests::set_goal(&mut w, gid, target);
    let g = w.guests.iter().find(|g| g.id == gid).unwrap();
    assert!(g.path.is_empty(), "no straight path: {:?}", g.path);
    for _ in 0..200 {
        w.tick(&c);
        let v: Vec<String> = people(&w, &c)
            .into_iter()
            .filter(|x| x.contains(&format!("guest {gid} ")))
            .collect();
        assert!(v.is_empty(), "{v:?}");
    }
}

/// Chaos find: at night a table could be put down (or moved) right where Karolina or a guest
/// stood — they were then inside it.
#[test]
fn no_table_lands_on_someone_standing_there() {
    let c = content();
    let mut w = world_for(&c, 3, 44);
    w.start_night(&c);
    bot::finish_cutscenes(&mut w, &c);
    let k = w.karolina().unwrap().tile();
    assert!(w.grid.walkable(k.0, k.1));
    let r = w.apply(
        &c,
        Command::PlaceFurniture {
            def: "table_round".into(),
            x: k.0,
            y: k.1,
            rot: 0,
        },
    );
    assert_eq!(r, Err(CommandError::Occupied));
    // …nor on the spot she is walking to (a path's last step may be entered even when it
    // isn't walkable)
    let place = |w: &mut World, t: (i32, i32)| {
        w.apply(
            &c,
            Command::PlaceFurniture {
                def: "table_round".into(),
                x: t.0,
                y: t.1,
                rot: 0,
            },
        )
    };
    let to = (1..w.grid.width)
        .flat_map(|x| (1..14).map(move |y| (x, y)))
        .find(|&t| place(&mut w.clone(), t).is_ok())
        .expect("a free spot for a table");
    let i = w.staff.iter().position(|s| s.owner).unwrap();
    w.staff[i].path = vec![to];
    assert_eq!(place(&mut w, to), Err(CommandError::Occupied));
}

/// Chaos find: a chair only reachable over another one (built so by night) trapped the
/// guests once someone sat on the outer chair — paths now cross a taken chair as a last
/// resort, and such a build is refused; the starting cafés never do it.
#[test]
fn a_chair_behind_a_taken_chair_is_still_reachable() {
    let c = content();
    let mut next = 1;
    let (mut g, f) = grid::parse_layout("######\n#Th.##\n#hh###\n#..###\n######", &c, &mut next);
    g.rebuild(&f, &c);
    assert!(g.is_seat(2, 1) && g.is_seat(2, 2));
    g.set_taken(vec![(2, 1)]);
    let p = grid::find_path(&g, (1, 3), (3, 1), true).expect("a way over the chairs");
    assert!(p.contains(&(2, 1)), "{p:?}");
    // with a free floor way, the taken chair is left alone
    let (mut g, f) = grid::parse_layout("######\n#....#\n#.hh.#\n#....#\n######", &c, &mut next);
    g.rebuild(&f, &c);
    g.set_taken(vec![(2, 2)]);
    let p = grid::find_path(&g, (1, 2), (4, 2), true).unwrap();
    assert!(!p.contains(&(2, 2)) && !p.contains(&(3, 2)), "{p:?}");
    // the starting cafés keep every seat reachable off the chairs
    for w in [world_for(&c, 0, 1), world_for(&c, 3, 2)] {
        assert!(super::commands::seat_access_ok(&w, &c));
    }
}
