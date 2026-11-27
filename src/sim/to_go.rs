//! To go: a party that finds no free seat may take its order with it instead of walking on
//! (0.2.2, data `purchases.ron` → `to_go`).
//!
//! Plain parties only (never a regular, never a wish party), and only what fits a paper cup
//! or a paper bag (drinks, desserts). The leader queues at the register like anyone; the
//! others — and the leader once the order is taken — wait on the spots beside the register,
//! away from the line and the staff's flap. A helper or Karolina hands the cups over there;
//! they walk out with them (`Party::to_go`, the paper cup in `AgentView::to_go`), tip a
//! little less, leave no dishes. Classic: fewer of them bother (`classic_chance`). A long
//! line, or too many already waiting, and they go on as before.
//!
//! The choice is rolled from a derived stream: the world RNG is never touched.

use super::grid;
use super::purchases;
use super::rng::Rng;
use super::types::Difficulty;
use super::world::*;
use crate::content::Content;

/// The share of seatless plain parties who take it to go here.
fn chance(w: &World) -> f32 {
    let d = &purchases::data().to_go;
    if w.difficulty == Difficulty::Classic {
        d.classic_chance
    } else {
        d.chance
    }
}

/// A derived roll in 0..1 for party `pid` (never `World::rng`).
fn roll(w: &World, pid: u32) -> f32 {
    let mix = |x: u64, k: u64| x.wrapping_mul(k).rotate_left(29);
    Rng::new(
        w.seed
            ^ mix(pid as u64 + 1, 0x9E37_79B9_7F4A_7C15)
            ^ mix(w.night as u64 + 3, 0xC2B2_AE3D_27D4_EB4F)
            ^ 0x746f_5f67_6f5f_7631,
    )
    .f32()
}

/// Guests of to-go parties still inside (waiting or walking to their spot).
fn waiting_guests(w: &World) -> usize {
    w.parties
        .iter()
        .filter(|p| p.to_go && p.state != PartyState::Leaving)
        .map(|p| p.members.len())
        .sum()
}

/// What a seatless party of `size` would take to go: their register and the order cut to
/// what fits a paper cup (`None`: they go on). Nothing is rolled unless everything else fits.
pub(crate) fn plan(
    w: &World,
    content: &Content,
    pid: u32,
    size: u32,
    wishes: &[String],
    wish_party: bool,
    regular: bool,
) -> Option<(u32, Vec<String>)> {
    let d = &purchases::data().to_go;
    if wish_party || regular || chance(w) <= 0.0 {
        return None;
    }
    let items: Vec<String> = wishes
        .iter()
        .filter(|id| {
            content
                .menu_item(id)
                .is_some_and(|m| d.categories.contains(&m.category))
        })
        .cloned()
        .collect();
    if items.is_empty() {
        return None;
    }
    // the shortest line, and not a long one
    let (reg, line) = super::guests::guest_registers(w, content)
        .iter()
        .map(|(u, _)| (*u, w.queues.get(u).map_or(0, |q| q.len())))
        .min_by_key(|(_, n)| *n)?;
    if line >= d.max_queue {
        return None;
    }
    // room to wait by the register, without crowding the hall
    let waiting = waiting_guests(w) + size as usize;
    if waiting > d.max_waiting || spots(w, content, reg).len() < waiting {
        return None;
    }
    (roll(w, pid) < chance(w)).then_some((reg, items))
}

/// The spots beside register `reg` where to-go guests wait for their cups, best first: indoor
/// hall tiles within 4 steps of the register's front, never the line, a seat, or a tile beside an
/// opening to the staff side (the flap stays clear); the side away from the line first.
pub(crate) fn spots(w: &World, content: &Content, reg: u32) -> Vec<(i32, i32)> {
    let Some(front) = super::guests::registers(w, content)
        .into_iter()
        .find(|r| r.0 == reg)
        .map(|r| r.1)
    else {
        return Vec::new();
    };
    let line = super::guests::queue_slots(w, content, reg);
    let seat_tiles: Vec<(i32, i32)> = super::guests::seats(w, content)
        .iter()
        .map(|s| s.tile)
        .collect();
    let by_staff_door = |t: (i32, i32)| {
        [(0, 1), (0, -1), (1, 0), (-1, 0)].iter().any(|(dx, dy)| {
            let (x, y) = (t.0 + dx, t.1 + dy);
            w.grid.walkable(x, y) && w.grid.room(x, y).staff_only()
        })
    };
    let mut out: Vec<(i32, i32, i32)> = Vec::new();
    for dy in -4..=4 {
        for dx in -4..=4 {
            let t = (front.0 + dx, front.1 + dy);
            let d = grid::manhattan(t, front);
            if d == 0
                || d > 4
                || !w.grid.guest_ok(t.0, t.1)
                // (indoor floor only: 0.2.4 — a register by the outer wall had its spots
                // reach through the wall onto the sidewalk)
                || !w.grid.is_indoor_floor(t.0, t.1)
                // (one gets to without crossing a chair: chaos find — a spot in a nook
                // behind a chair had the to-go guest climb over whoever sat down there)
                || !w.grid.guest_clear(t.0, t.1)
                || seat_tiles.contains(&t)
                || line.contains(&t)
                || by_staff_door(t)
            {
                continue;
            }
            // the queue runs to the west of the front: wait on the other side
            let side = if dx > 0 { 0 } else { 3 };
            out.push((d + side, t.0, t.1));
        }
    }
    // nearest first; at a tie the tile closer to the counter, then from the west
    out.sort_by_key(|&(score, x, y)| (score, -y, x));
    out.into_iter().map(|(_, x, y)| (x, y)).collect()
}

/// The register a to-go guest waits by (their party's, else the nearest one).
fn register_of(w: &World, content: &Content, gid: u32) -> Option<u32> {
    let g = w.guests.iter().find(|g| g.id == gid)?;
    if let Some(r) = w
        .parties
        .iter()
        .find(|p| p.id == g.party)
        .and_then(|p| p.register)
    {
        return Some(r);
    }
    let here = g.goal.unwrap_or_else(|| g.tile());
    super::guests::registers(w, content)
        .into_iter()
        .min_by_key(|(_, t)| grid::manhattan(*t, here))
        .map(|(u, _)| u)
}

/// Is `gid` one of a to-go party (still inside)?
pub(crate) fn is_to_go(w: &World, gid: u32) -> bool {
    w.guests
        .iter()
        .find(|g| g.id == gid)
        .and_then(|g| w.parties.iter().find(|p| p.id == g.party))
        .is_some_and(|p| p.to_go)
}

/// Is `gid` waiting on a spot (not the leader in the line, not leaving)?
fn waits_on_a_spot(w: &World, gid: u32) -> bool {
    let Some(g) = w.guests.iter().find(|g| g.id == gid) else {
        return false;
    };
    w.parties.iter().any(|p| {
        p.id == g.party
            && p.to_go
            && p.state != PartyState::Leaving
            && !(p.state == PartyState::Queue && p.leader() == Some(gid))
    })
}

/// Sends `gid` to a free spot beside the register.
pub(crate) fn send_to_spot(w: &mut World, content: &Content, gid: u32) {
    let Some(reg) = register_of(w, content, gid) else {
        return;
    };
    let taken: Vec<(i32, i32)> = w
        .guests
        .iter()
        .filter(|o| o.id != gid && waits_on_a_spot(w, o.id))
        .filter_map(|o| o.goal)
        .collect();
    let all = spots(w, content, reg);
    let spot = all
        .iter()
        .find(|t| !taken.contains(t))
        .or(all.last())
        .copied();
    if let Some(t) = spot {
        super::guests::set_goal(w, gid, t);
    }
}

/// Every now and then (`guests::refresh_queue_goals`): a to-go guest whose spot is gone (a
/// piece was put there, the register moved) or shared walks to a free one.
pub(crate) fn refresh(w: &mut World, content: &Content) {
    let waiting: Vec<u32> = w
        .guests
        .iter()
        .filter(|g| waits_on_a_spot(w, g.id))
        .map(|g| g.id)
        .collect();
    let mut kept: Vec<(i32, i32)> = Vec::new();
    for gid in waiting {
        let Some(reg) = register_of(w, content, gid) else {
            continue;
        };
        let goal = w.guests.iter().find(|g| g.id == gid).and_then(|g| g.goal);
        let ok = goal.is_some_and(|t| !kept.contains(&t) && spots(w, content, reg).contains(&t));
        if ok {
            kept.extend(goal);
        } else {
            send_to_spot(w, content, gid);
            if let Some(t) = w.guests.iter().find(|g| g.id == gid).and_then(|g| g.goal) {
                kept.push(t);
            }
        }
    }
}

/// Where a to-go guest is handed their cup (their spot; walking there: the spot).
pub(crate) fn hand_over_tile(w: &World, gid: u32) -> Option<(i32, i32)> {
    let g = w.guests.iter().find(|g| g.id == gid)?;
    is_to_go(w, gid).then(|| g.goal.unwrap_or_else(|| g.tile()))
}

/// The to-go tips ×.
pub(crate) fn tips_mul(w: &World, pi: usize) -> f32 {
    if w.parties.get(pi).is_some_and(|p| p.to_go) {
        purchases::data().to_go.tips
    } else {
        1.0
    }
}
