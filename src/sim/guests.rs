//! Guests: arrivals, seating, queueing, patience, eating, paying, reviews, leaving.

use serde::{Deserialize, Serialize};

use super::grid::{self, TILE, TileKind};
use super::rng::hash_str;
use super::types::*;
use super::world::*;
use crate::content::{Content, MenuItem, Pays, TraitEffect};

/// Guest walking speed, tiles per tick.
pub const GUEST_SPEED: f32 = 0.35;
/// Parties per opening hour at reputation 25 in a normal town (before hour weights).
pub const BASE_PARTIES_PER_HOUR: f32 = 1.7;
/// Waiting for food allows this many × the archetype patience.
pub const FOOD_PATIENCE: f32 = 2.4;
/// Cozy: food waits are longer still — Karolina makes most things herself (handson.md §1.4).
pub const COZY_FOOD_PATIENCE: f32 = 3.0;
/// Chance (× spend_mult) that a guest orders a second thing.
pub const SECOND_ITEM_CHANCE: f32 = 0.75;
/// Chance (× spend_mult) of a third thing (usually dessert).
pub const THIRD_ITEM_CHANCE: f32 = 0.2;
/// Reputation per satisfaction point above/below 55 per guest review.
pub const REVIEW_WEIGHT: f32 = 0.003;
/// Regulars' trust per item served (within 8 min of the order / later) and lost when they
/// leave unserved.
pub const TRUST_FAST: f32 = 4.0;
pub const TRUST_SERVED: f32 = 2.0;
pub const TRUST_ANGRY: f32 = 2.0;
/// No new walk-ins after this minute (05:40).
pub const LAST_CALL: u32 = 460;
/// Hall dirt above this is worth a mop.
pub const MOP_DIRT: f32 = 0.12;
/// Key of the line a forgiving Cozy guest leaves with ("I'll come back tomorrow.").
pub const FORGIVEN_KEY: &str = "ui.guest.forgiven";

/// How a party leaves (`finish_party`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Exit {
    /// Done (or closing time).
    Normal,
    /// Ran out of patience (Classic): a mild review.
    Angry,
    /// Cozy: waited too long, but they leave content — no review penalty.
    Forgiven,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingSpawn {
    pub minute: u32,
    pub archetype: String,
    pub size: u32,
    pub patience_mul: f32,
    pub spend_mul: f32,
    pub orders: Vec<String>,
    /// The leader comes with a wish (lull-director walk-ins).
    #[serde(default)]
    pub wish: bool,
}

// ---------------------------------------------------------------------------------------------
// Menu availability
// ---------------------------------------------------------------------------------------------

/// Sandbox unlock tier from reputation and nights played (stands in for `unlock_night`).
pub(crate) fn sandbox_tier(w: &World) -> u32 {
    2 + (w.econ.reputation / 7.0) as u32 + (w.night.saturating_sub(1)) / 4
}

pub(crate) fn item_unlocked(w: &World, m: &MenuItem) -> bool {
    if w.story.unlocked_menu.contains(&m.id) {
        return true;
    }
    match w.mode {
        Mode::Story => {
            m.unlock_night <= w.night
                && m.unlock_flag
                    .as_ref()
                    .is_none_or(|f| w.story.flags.contains(f))
        }
        Mode::Sandbox => m.unlock_night <= sandbox_tier(w),
    }
}

pub(crate) fn station_ok(w: &World, content: &Content, tag: &str) -> bool {
    if w.mods
        .iter()
        .any(|md| matches!(&md.kind, ModKind::StationDown(t) if t == tag))
    {
        return false;
    }
    w.furniture.iter().any(|p| {
        content
            .furniture_def(&p.def)
            .is_some_and(|d| d.has_tag(tag))
    })
}

pub(crate) fn item_orderable(w: &World, content: &Content, m: &MenuItem) -> bool {
    if !item_unlocked(w, m) || w.econ.menu_off.contains(&m.id) {
        return false;
    }
    if w.mods
        .iter()
        .any(|md| matches!(&md.kind, ModKind::MenuUnavailable(i) if *i == m.id))
    {
        return false;
    }
    if m.needs_power && w.blackout_until.is_some() {
        return false;
    }
    if w.econ.supplies < m.cost as i64 {
        return false;
    }
    if !station_ok(w, content, &m.station) {
        return false;
    }
    // Somebody tonight must be able to make it (Karolina can make anything).
    if w.phase == Phase::Night {
        let able = w.staff.iter().any(|s| {
            matches!(s.status, StaffStatus::OnDuty | StaffStatus::Expected { .. })
                && (s.owner || s.skill(m.job, w.night) >= m.min_skill)
                && w.mods.iter().all(|md| {
                    !(md.kind == ModKind::WorkSpeed(Some(m.job.index() as u8)) && md.mul <= 0.0)
                })
        });
        if !able {
            return false;
        }
    }
    true
}

// ---------------------------------------------------------------------------------------------
// Seats & queue spots
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(crate) struct SeatInfo {
    pub uid: u32,
    pub idx: u8,
    pub tile: (i32, i32),
    pub table: Option<u32>,
    pub free: bool,
    pub facing: Facing,
    pub comfort: f32,
}

pub(crate) fn seat_tile(w: &World, uid: u32, idx: u8) -> Option<(i32, i32)> {
    let p = w.furniture_by_uid(uid)?;
    // footprint tiles in row-major order
    let (sw, sh) = if p.rot % 2 == 1 { (1, 2) } else { (2, 1) };
    let _ = (sw, sh);
    let mut tiles = Vec::new();
    // we don't have the def here; seats ≤ 2 and footprints ≤ 2 wide/tall
    for dy in 0..2 {
        for dx in 0..2 {
            if w.grid.object_at(p.x + dx, p.y + dy) == Some(uid) {
                tiles.push((p.x + dx, p.y + dy));
            }
        }
    }
    tiles.get(idx as usize).copied().or(Some((p.x, p.y)))
}

pub(crate) fn seats(w: &World, content: &Content) -> Vec<SeatInfo> {
    let tables: Vec<(u32, Vec<(i32, i32)>)> = w
        .furniture
        .iter()
        .filter_map(|p| {
            let d = content.furniture_def(&p.def)?;
            d.has_tag("table")
                .then(|| (p.uid, grid::footprint(d, p.x, p.y, p.rot)))
        })
        .collect();
    let mut out = Vec::new();
    for p in &w.furniture {
        let Some(d) = content.furniture_def(&p.def) else {
            continue;
        };
        if d.seats == 0 {
            continue;
        }
        let fp = grid::footprint(d, p.x, p.y, p.rot);
        for i in 0..d.seats as usize {
            let tile = fp.get(i).copied().unwrap_or((p.x, p.y));
            let table = tables
                .iter()
                .filter(|(_, t)| t.iter().any(|tt| grid::chebyshev(*tt, tile) <= 1))
                .min_by_key(|(_, t)| {
                    t.iter()
                        .map(|tt| grid::manhattan(*tt, tile))
                        .min()
                        .unwrap_or(9)
                })
                .map(|(u, _)| *u);
            // Face the table if there is one, else the furniture's facing.
            let facing = match table.and_then(|t| tables.iter().find(|x| x.0 == t)) {
                Some((_, tt)) => {
                    let c = tt[0];
                    Facing::from_delta((c.0 - tile.0) as f32, (c.1 - tile.1) as f32)
                }
                None => p.facing(),
            };
            out.push(SeatInfo {
                uid: p.uid,
                idx: i as u8,
                tile,
                table,
                free: p.seat_by.get(i).copied().flatten().is_none(),
                facing,
                comfort: d.comfort,
            });
        }
    }
    out
}

fn seat_has_tag(w: &World, content: &Content, s: &SeatInfo, tag: &str) -> bool {
    if tag == "window_spot" {
        for dy in -2..=2 {
            for dx in -2..=2 {
                if w.grid
                    .tile(s.tile.0 + dx, s.tile.1 + dy)
                    .is_some_and(|t| t.kind == TileKind::Window)
                {
                    return true;
                }
            }
        }
    }
    w.furniture.iter().any(|p| {
        content.furniture_def(&p.def).is_some_and(|d| {
            d.has_tag(tag)
                && grid::footprint(d, p.x, p.y, p.rot)
                    .iter()
                    .any(|t| grid::chebyshev(*t, s.tile) <= 1)
        })
    })
}

/// Picks `n` free seats close together (prefers one table, then `prefer` tag).
/// (furniture uid, seat index, seat tile).
type SeatPick = (u32, u8, (i32, i32));

fn choose_seats(
    w: &World,
    content: &Content,
    n: usize,
    prefer: Option<&str>,
) -> Option<Vec<SeatPick>> {
    let all = seats(w, content);
    let free: Vec<&SeatInfo> = all.iter().filter(|s| s.free).collect();
    if free.len() < n {
        return None;
    }
    let reach = grid::reachable(&w.grid, w.grid.entrance());
    let reachable = |t: (i32, i32)| -> bool {
        // seat tile itself (chairs) or any neighbour reachable
        [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)]
            .iter()
            .any(|(dx, dy)| {
                w.grid
                    .idx(t.0 + dx, t.1 + dy)
                    .is_some_and(|i| reach[i] && w.grid.guest_ok(t.0 + dx, t.1 + dy))
            })
    };
    let free: Vec<&SeatInfo> = free.into_iter().filter(|s| reachable(s.tile)).collect();
    if free.len() < n {
        return None;
    }
    let score = |s: &SeatInfo| -> f32 {
        let mut v = s.comfort;
        if s.table.is_some() {
            v += 3.0;
        }
        if let Some(tag) = prefer
            && seat_has_tag(w, content, s, tag)
        {
            v += 50.0;
        }
        // tables already holding dishes are less nice
        if let Some(t) = s.table.and_then(|t| w.furniture_by_uid(t)) {
            v -= t.dishes as f32;
        }
        v
    };
    // A table with enough free seats.
    let mut best: Option<(f32, Vec<&SeatInfo>)> = None;
    let mut tables: Vec<u32> = free.iter().filter_map(|s| s.table).collect();
    tables.sort();
    tables.dedup();
    for t in tables {
        let at: Vec<&SeatInfo> = free
            .iter()
            .filter(|s| s.table == Some(t))
            .copied()
            .collect();
        let total_at = all.iter().filter(|s| s.table == Some(t)).count();
        let busy = total_at - at.len();
        if at.len() >= n {
            // strangers don't share small tables
            if busy > 0 && n + busy > 2 && total_at <= 2 {
                continue;
            }
            if busy > 0 && total_at <= 2 {
                continue;
            }
            let sc: f32 = at.iter().take(n).map(|s| score(s)).sum::<f32>() / n as f32
                - if busy > 0 { 4.0 } else { 0.0 };
            if best.as_ref().is_none_or(|b| sc > b.0) {
                best = Some((sc, at.into_iter().take(n).collect()));
            }
        }
    }
    if best.is_none() {
        // Seats near each other (sofas, window seats, two tables).
        let first = free
            .iter()
            .max_by(|a, b| score(a).total_cmp(&score(b)))
            .copied()?;
        let mut v = vec![first];
        let mut rest: Vec<&SeatInfo> = free
            .iter()
            .filter(|s| !std::ptr::eq(*s, &first))
            .copied()
            .collect();
        rest.sort_by_key(|s| grid::manhattan(s.tile, first.tile));
        for s in rest {
            if v.len() >= n {
                break;
            }
            if grid::manhattan(s.tile, first.tile) <= 4 {
                v.push(s);
            }
        }
        if v.len() < n {
            return None;
        }
        best = Some((0.0, v));
    }
    best.map(|(_, v)| v.iter().map(|s| (s.uid, s.idx, s.tile)).collect())
}

/// Registers with their guest-side front tile.
pub(crate) fn registers(w: &World, content: &Content) -> Vec<(u32, (i32, i32))> {
    let mut v = Vec::new();
    for p in &w.furniture {
        let Some(d) = content.furniture_def(&p.def) else {
            continue;
        };
        if !d.has_tag("register") {
            continue;
        }
        if let Some(t) = grid::access_tiles(&w.grid, d, p)
            .into_iter()
            .find(|t| w.grid.guest_ok(t.0, t.1))
        {
            v.push((p.uid, t));
        }
    }
    v
}

/// Queue tiles for a register, front first.
pub(crate) fn queue_slots(w: &World, content: &Content, register: u32) -> Vec<(i32, i32)> {
    let Some(front) = registers(w, content)
        .into_iter()
        .find(|r| r.0 == register)
        .map(|r| r.1)
    else {
        return Vec::new();
    };
    let seat_tiles: Vec<(i32, i32)> = seats(w, content).iter().map(|s| s.tile).collect();
    let ok = |t: (i32, i32)| w.grid.guest_ok(t.0, t.1) && !seat_tiles.contains(&t);
    let mut v = vec![front];
    let mut cur = front;
    while v.len() < 8 {
        let next = (cur.0 - 1, cur.1);
        if ok(next) {
            v.push(next);
            cur = next;
        } else {
            break;
        }
    }
    while v.len() < 12 {
        let next = (cur.0, cur.1 - 1);
        if ok(next) {
            v.push(next);
            cur = next;
        } else {
            break;
        }
    }
    v
}

// ---------------------------------------------------------------------------------------------
// Spawning
// ---------------------------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub(crate) fn schedule_rush(
    w: &mut World,
    content: &Content,
    archetype: &str,
    count: u32,
    within: u32,
    patience_mul: f32,
    spend_mul: f32,
    orders: &[String],
) {
    let Some(arch) = content.guest(archetype) else {
        return;
    };
    let m = w.minute();
    let mut left = count;
    while left > 0 {
        let size = w
            .rng
            .range_u32(arch.group.0.max(1), arch.group.1.max(1))
            .min(left);
        let at = m + if within > 0 {
            w.rng.range_u32(0, within)
        } else {
            0
        };
        w.spawns.push(PendingSpawn {
            minute: at,
            archetype: archetype.to_string(),
            size,
            patience_mul,
            spend_mul,
            orders: orders.to_vec(),
            wish: false,
        });
        left -= size;
    }
}

fn pick_wishes(
    w: &mut World,
    content: &Content,
    favourites: &[String],
    orders: &[String],
    size: u32,
    spend_mult: f32,
) -> Vec<String> {
    let orderable: Vec<&MenuItem> = content
        .menu
        .iter()
        .filter(|m| item_orderable(w, content, m))
        .collect();
    if orderable.is_empty() {
        return Vec::new();
    }
    let pool: Vec<&MenuItem> = if orders.is_empty() {
        favourites
            .iter()
            .filter_map(|f| orderable.iter().find(|m| &m.id == f).copied())
            .collect()
    } else {
        orders
            .iter()
            .filter_map(|f| orderable.iter().find(|m| &m.id == f).copied())
            .collect()
    };
    let pool = if pool.is_empty() {
        orderable.clone()
    } else {
        pool
    };
    let weights: Vec<f32> = (0..pool.len()).map(|i| 1.0 / (i as f32 + 1.0)).collect();
    let mut out = Vec::new();
    for _ in 0..size {
        let i = w.rng.weighted(&weights).unwrap_or(0);
        out.push(pool[i].id.clone());
        if w.rng.chance((SECOND_ITEM_CHANCE * spend_mult).min(0.85)) {
            // a second thing from a different category
            let first_cat = pool[i].category;
            let other: Vec<&MenuItem> = orderable
                .iter()
                .filter(|m| m.category != first_cat)
                .copied()
                .collect();
            let fav_other: Vec<&MenuItem> = other
                .iter()
                .filter(|m| favourites.contains(&m.id))
                .copied()
                .collect();
            let src = if fav_other.is_empty() {
                &other
            } else {
                &fav_other
            };
            if let Some(m) = w.rng.pick(src) {
                out.push(m.id.clone());
            }
            if w.rng.chance((THIRD_ITEM_CHANCE * spend_mult).min(0.4))
                && let Some(m) = w.rng.pick(&orderable)
            {
                out.push(m.id.clone());
            }
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_party(
    w: &mut World,
    content: &Content,
    archetype: &str,
    size: u32,
    regular: Option<&str>,
    patience_mul: f32,
    spend_mul: f32,
    orders: &[String],
    leave_by: Option<u32>,
    force_wish: bool,
) -> Option<u32> {
    let arch = content.guest(archetype)?;
    let reg = regular.and_then(|r| content.regular(r));
    // the leader's order kind (regulars always wish; others by chance)
    let order_kind = super::orders::pick_order_kind(w, content, archetype, reg, size, force_wish);
    let wish_party = order_kind.is_wish();
    let favourites: Vec<String> = arch.favourites.clone();
    let mut order_list: Vec<String> = orders.to_vec();
    if let Some(r) = reg {
        order_list = r.orders.clone();
        // A story-unlocked dish they haven't tried yet.
        let tried = w
            .story
            .regulars
            .get(&r.id)
            .map(|s| s.served_items.clone())
            .unwrap_or_default();
        for u in w.story.unlocked_menu.clone() {
            if !tried.contains(&u) && w.rng.chance(0.6) {
                order_list.insert(0, u);
                break;
            }
        }
    }
    let wishes = if wish_party && reg.is_some() {
        // the regular's wish is Karolina's: no plain items
        Vec::new()
    } else if wish_party {
        // the leader's main order is the wish (it may still come with something on the
        // side); the others order plain items
        let mut v = pick_wishes(
            w,
            content,
            &favourites,
            &order_list,
            size,
            arch.spend_mult * spend_mul,
        );
        if !v.is_empty() {
            v.remove(0);
        }
        v
    } else if reg.is_some() {
        // regulars: their first orderable order (+ maybe the second)
        let ok: Vec<String> = order_list
            .iter()
            .filter(|o| {
                content
                    .menu_item(o)
                    .is_some_and(|m| item_orderable(w, content, m))
            })
            .cloned()
            .collect();
        let mut v: Vec<String> = ok.iter().take(1).cloned().collect();
        if ok.len() > 1 && w.rng.chance(0.5) {
            v.push(ok[1].clone());
        }
        if v.is_empty() {
            pick_wishes(w, content, &favourites, &[], 1, arch.spend_mult)
        } else {
            v
        }
    } else {
        pick_wishes(
            w,
            content,
            &favourites,
            &order_list,
            size,
            arch.spend_mult * spend_mul,
        )
    };
    let id = w.alloc_id();
    let edges = w.grid.street_edges();
    let side = (w.rng.next_u64() % 2) as usize;
    let start = edges[side];
    let diff = content.difficulty(w.difficulty.id());
    let pmul = diff.map(|d| d.guest_patience_mul).unwrap_or(1.0) * patience_mul * early_ramp(w).1;
    let mut patience = reg
        .map(|r| r.patience_minutes)
        .unwrap_or(arch.patience_minutes)
        * pmul;
    if reg.is_some() && w.difficulty == Difficulty::Cozy {
        patience = patience.max(content.handson.tuning.cozy_regular_patience_floor);
    }
    let mut party = Party {
        id,
        archetype: archetype.to_string(),
        regular: regular.map(|s| s.to_string()),
        members: Vec::new(),
        state: PartyState::Arriving,
        arrived: w.minute(),
        ordered_at: None,
        served_at: None,
        patience,
        patience_max: patience,
        wishes,
        items: Vec::new(),
        satisfaction: 60.0 + super::app::checklist_bonus(w),
        spend_mul,
        register: None,
        order_claim: None,
        stay_left: 0.0,
        bill: 0,
        served_by: None,
        leave_by,
        served: false,
        angry: false,
        wish_open: wish_party,
        order_kind,
        handed_off: false,
        wait_clock: 0.0,
        wait_at_order: 0.0,
        forgiven: 0,
        greeted: false,
        tip_factors: Vec::new(),
        record_minutes: 0,
        music_reacted: false,
        archetype_music_done: false,
    };
    let prefer = reg.and_then(|r| r.seat.clone());
    let seats_pick = if party.wishes.is_empty() && !wish_party {
        None
    } else {
        choose_seats(w, content, size as usize, prefer.as_deref())
    };
    // record regular visit
    if let Some(r) = reg {
        let seat_free = prefer.as_ref().is_none_or(|tag| {
            seats(w, content)
                .iter()
                .any(|s| s.free && seat_has_tag(w, content, s, tag))
        });
        let clean = 1.0 - w.dirt;
        let night = w.night;
        let st = w.story.regulars.entry(r.id.clone()).or_default();
        st.visits.push(VisitRecord {
            night,
            served_minutes: None,
            seat_free,
            clean,
            best_match: None,
        });
        if st.visits.len() > 12 {
            st.visits.remove(0);
        }
        st.party = Some(id);
    }
    let preset = reg.map(|r| r.look_preset.clone().unwrap_or_else(|| r.id.clone()));
    let exit = edges[1 - side];
    for k in 0..size {
        let gid = w.alloc_id();
        let seed = w.rng.fork() ^ hash_str(archetype);
        let offset = (k as f32 * 9.0 - 9.0, 0.0);
        let c = grid::tile_center(start);
        party.members.push(gid);
        w.guests.push(Guest {
            id: gid,
            party: id,
            pos: (c.0 + offset.0, c.1 + offset.1),
            path: Vec::new(),
            facing: if side == 0 {
                Facing::Right
            } else {
                Facing::Left
            },
            pose: Pose::Walk,
            seat: None,
            look_seed: seed,
            preset: preset.clone(),
            source: regular.unwrap_or(archetype).to_string(),
            leader: k == 0,
            holding: None,
            emote: None,
            goal: None,
            gone: false,
            stuck_ticks: 0,
        });
    }
    w.tonight.guests_arrived += size;
    match seats_pick {
        None => {
            // Looks in, walks on (the wish wasn't used).
            if let Some(wish) = party.order_kind.wish()
                && let Some(n) = w.hands.wish_uses.get_mut(wish)
            {
                *n = n.saturating_sub(1);
            }
            party.state = PartyState::Leaving;
            w.tonight.guests_turned_away += size;
            if !party.wishes.is_empty() {
                w.econ.reputation = (w.econ.reputation - 0.05 * size as f32).max(0.0);
            }
            for &gid in &party.members {
                set_goal(w, gid, exit);
            }
            w.parties.push(party);
            w.emit(SimEvent::GuestTurnedAway { party: id });
            return Some(id);
        }
        Some(seat_list) => {
            for (k, (uid, idx, _)) in seat_list.iter().enumerate() {
                let gid = party.members[k];
                if let Some(p) = w.furniture.iter_mut().find(|p| p.uid == *uid)
                    && let Some(s) = p.seat_by.get_mut(*idx as usize)
                {
                    *s = Some(gid);
                }
                if let Some(g) = w.guests.iter_mut().find(|g| g.id == gid) {
                    g.seat = Some((*uid, *idx));
                }
            }
            // order at the register, or at the table when the queue is long / no register
            let regs = registers(w, content);
            let best = regs
                .iter()
                .map(|(u, _)| (*u, w.queues.get(u).map(|q| q.len()).unwrap_or(0)))
                .min_by_key(|(_, n)| *n);
            // wishes are told to Karolina at the table, never in the register queue
            let table_order = wish_party
                || match best {
                    None => true,
                    Some((_, n)) => n >= 3 && w.rng.chance(0.7),
                };
            for (k, &gid) in party.members.clone().iter().enumerate() {
                let seat_t = seat_list[k].2;
                if k == 0 && !table_order {
                    continue;
                }
                set_goal(w, gid, seat_t);
            }
            if table_order {
                party.state = PartyState::WaitOrder;
                // sitting down is more comfortable than standing in line
                party.patience *= 1.5;
                party.patience_max *= 1.5;
            } else {
                let (reg_uid, _) = best.unwrap_or((0, 0));
                party.register = Some(reg_uid);
                party.state = PartyState::Queue;
                w.queues.entry(reg_uid).or_default().push(id);
            }
            let size_u = size;
            let reg_id = party.regular.clone();
            w.parties.push(party);
            if party_is_queue(w, id) {
                refresh_queue_goals(w, content);
            }
            w.emit(SimEvent::GuestArrived {
                party: id,
                archetype: archetype.to_string(),
                size: size_u,
                regular: reg_id.clone(),
            });
            w.hands.tally.parties += 1;
            super::pacing::note_arrival(w, content);
            if let Some(r) = reg_id {
                w.emit(SimEvent::RegularIncoming {
                    regular: r.clone(),
                    party: id,
                    minutes: content.handson.tuning.regular_announce_minutes,
                });
                w.emit(SimEvent::RegularArrived {
                    regular: r.clone(),
                    party: id,
                });
                super::story::regular_line(w, content, &r, id);
            }
        }
    }
    Some(id)
}

fn party_is_queue(w: &World, id: u32) -> bool {
    w.parties
        .iter()
        .any(|p| p.id == id && p.state == PartyState::Queue)
}

/// Story/sandbox regular arrives now.
pub(crate) fn spawn_regular(w: &mut World, content: &Content, id: &str, visit: Option<usize>) {
    let Some(r) = content.regular(id) else {
        return;
    };
    if w.story
        .regulars
        .get(id)
        .and_then(|s| s.party)
        .is_some_and(|p| w.parties.iter().any(|x| x.id == p))
    {
        return;
    }
    let leave_by = visit
        .and_then(|v| r.visits.get(v))
        .map(|v| v.to.minutes_since_open().clamp(0, 480) as u32);
    spawn_party(
        w,
        content,
        &r.base,
        1,
        Some(id),
        1.0,
        1.0,
        &[],
        leave_by,
        false,
    );
}

pub(crate) fn set_goal(w: &mut World, gid: u32, goal: (i32, i32)) {
    let Some(i) = w.guests.iter().position(|g| g.id == gid) else {
        return;
    };
    let from = w.guests[i].tile();
    let path = w.paths.get(&w.grid, from, goal, true).or_else(|| {
        // fall back to staff mask (e.g. standing on a seat tile next to the bar)
        w.paths.get(&w.grid, from, goal, false)
    });
    let g = &mut w.guests[i];
    g.goal = Some(goal);
    g.path = path.unwrap_or_else(|| vec![goal]);
    g.pose = Pose::Walk;
}

pub(crate) fn refresh_queue_goals(w: &mut World, content: &Content) {
    let regs: Vec<u32> = w.queues.keys().copied().collect();
    for r in regs {
        let slots = queue_slots(w, content, r);
        let q = w.queues.get(&r).cloned().unwrap_or_default();
        for (pos, pid) in q.iter().enumerate() {
            let Some(leader) = w
                .parties
                .iter()
                .find(|p| p.id == *pid)
                .and_then(|p| p.members.first().copied())
            else {
                continue;
            };
            let slot = slots
                .get(pos)
                .copied()
                .or_else(|| slots.last().copied())
                .unwrap_or_else(|| w.grid.entrance());
            if w.guests
                .iter()
                .any(|g| g.id == leader && g.goal != Some(slot))
            {
                set_goal(w, leader, slot);
            }
        }
    }
}

/// Gentle start: (arrival multiplier, patience multiplier) for the first nights.
///
/// Story night 1 is a solo shift (the player only), so it is a quiet, forgiving tutorial; the
/// crowd and the impatience ramp up over nights 1–5. Cozy is softer than Classic. Sandbox
/// starts with two people and gets a shorter ramp.
pub(crate) fn early_ramp(w: &World) -> (f32, f32) {
    let cozy = w.difficulty == Difficulty::Cozy;
    let n = w.night as usize;
    let (rate, patience): (&[f32], &[f32]) = match (w.mode, cozy) {
        (Mode::Story, true) => (&[0.28, 0.5, 0.62, 0.74, 0.86], &[2.8, 2.0, 1.6, 1.3, 1.1]),
        (Mode::Story, false) => (
            &[0.28, 0.58, 0.72, 0.84, 0.94],
            &[2.4, 1.5, 1.25, 1.1, 1.05],
        ),
        (Mode::Sandbox, true) => (&[0.6, 0.75, 0.9], &[1.5, 1.3, 1.15]),
        (Mode::Sandbox, false) => (&[0.75, 0.9], &[1.2, 1.1]),
    };
    let at = |v: &[f32]| v.get(n.wrapping_sub(1)).copied().unwrap_or(1.0);
    (at(rate), at(patience))
}

/// Arrival multiplier of the gentle start (see [`early_ramp`]).
pub(crate) fn story_ramp(w: &World) -> f32 {
    early_ramp(w).0
}

fn arrival_rate(w: &World, content: &Content, hour: usize) -> f32 {
    let allowed = |inst: &Option<String>| {
        inst.as_ref()
            .is_none_or(|i| w.town.institutions.iter().any(|x| x == i))
    };
    let wd = w.weekday as usize;
    let weight_at = |h: usize| -> f32 {
        content
            .guests
            .iter()
            .filter(|g| allowed(&g.institution))
            .map(|g| g.weight_at(h) * g.weekday_mult.get(wd).copied().unwrap_or(1.0))
            .sum()
    };
    let avg: f32 = (0..8).map(weight_at).sum::<f32>() / 8.0;
    if avg <= 0.0 {
        return 0.0;
    }
    let shape = weight_at(hour) / avg;
    let rep = 0.8 + w.econ.reputation / 125.0;
    let weather = match w.weather {
        crate::content::Weather::Rain => 1.1,
        crate::content::Weather::Downpour => 1.0,
        crate::content::Weather::Storm => 0.85,
        crate::content::Weather::Fog => 0.9,
        _ => 1.0,
    };
    let story_ramp = story_ramp(w);
    let quiet = if super::pacing::quiet_hour(w, content) {
        content.handson.tuning.quiet_hour.guest_rate_mul
    } else {
        1.0
    };
    let mods: f32 = w
        .mods
        .iter()
        .filter(|m| m.kind == ModKind::GuestRate)
        .map(|m| m.mul)
        .product();
    BASE_PARTIES_PER_HOUR / 60.0 * shape * rep * weather * story_ramp * mods * quiet * w.town.size
}

pub(crate) fn pick_archetype(w: &mut World, content: &Content, hour: usize) -> Option<String> {
    let wd = w.weekday as usize;
    let list: Vec<(&str, f32)> = content
        .guests
        .iter()
        .filter(|g| {
            g.institution
                .as_ref()
                .is_none_or(|i| w.town.institutions.iter().any(|x| x == i))
        })
        .map(|g| {
            (
                g.id.as_str(),
                g.weight_at(hour) * g.weekday_mult.get(wd).copied().unwrap_or(1.0),
            )
        })
        .collect();
    let weights: Vec<f32> = list.iter().map(|x| x.1).collect();
    w.rng.weighted(&weights).map(|i| list[i].0.to_string())
}

// ---------------------------------------------------------------------------------------------
// Per-minute logic
// ---------------------------------------------------------------------------------------------

pub(crate) fn on_minute(w: &mut World, content: &Content) {
    let m = w.minute();
    // Random walk-ins.
    if m < LAST_CALL
        && let Some(hour) = crate::content::ClockTime::from_minutes_since_open(m).open_hour_index()
    {
        let rate = arrival_rate(w, content, hour);
        if w.rng.chance(rate)
            && let Some(a) = pick_archetype(w, content, hour)
            && let Some(arch) = content.guest(&a)
        {
            let size = w.rng.range_u32(arch.group.0.max(1), arch.group.1.max(1));
            spawn_party(w, content, &a, size, None, 1.0, 1.0, &[], None, false);
        }
    }
    // Scheduled (rushes, trains).
    let due: Vec<PendingSpawn> = {
        let mut due = Vec::new();
        w.spawns.retain(|s| {
            if s.minute <= m {
                due.push(s.clone());
                false
            } else {
                true
            }
        });
        due
    };
    for s in due {
        if m < 478 {
            spawn_party(
                w,
                content,
                &s.archetype,
                s.size,
                None,
                s.patience_mul,
                s.spend_mul,
                &s.orders,
                None,
                s.wish,
            );
        }
    }
    // Patience & eating. The wait clock runs only once they're actually inside and waiting
    // (not while walking in), at the claimed / rush rates (handson.md §1.4).
    let cozy = w.difficulty == Difficulty::Cozy;
    let forgive = content.handson.tuning.cozy_forgiveness;
    let mut leave: Vec<(u32, Exit)> = Vec::new();
    let waiting: Vec<u32> = w
        .parties
        .iter()
        .filter(|p| match p.state {
            PartyState::Queue => p
                .members
                .first()
                .and_then(|l| w.guests.iter().find(|g| g.id == *l))
                .is_some_and(|g| g.path.is_empty()),
            PartyState::WaitOrder => p
                .members
                .iter()
                .any(|m| w.guests.iter().any(|g| g.id == *m && g.pose == Pose::Sit)),
            PartyState::WaitFood => true,
            _ => false,
        })
        .map(|p| p.id)
        .collect();
    let rates: Vec<f32> = (0..w.parties.len())
        .map(|pi| super::pacing::wait_rate(w, content, pi))
        .collect();
    let mut refilled: Vec<u32> = Vec::new();
    for (pi, p) in w.parties.iter_mut().enumerate() {
        match p.state {
            PartyState::Queue | PartyState::WaitOrder | PartyState::WaitFood
                if !waiting.contains(&p.id) => {}
            PartyState::Queue | PartyState::WaitOrder | PartyState::WaitFood => {
                let rate = rates[pi];
                p.wait_clock += rate;
                p.patience -= rate;
                if p.patience <= 0.0 {
                    if !cozy {
                        leave.push((p.id, Exit::Angry));
                    } else if p.forgiven + 1 < forgive.runs || p.regular.is_some() {
                        // a glass of water; regulars never leave before their window ends
                        p.forgiven += 1;
                        p.patience = p.patience_max * forgive.refill;
                        refilled.push(p.id);
                    } else {
                        p.forgiven += 1;
                        leave.push((p.id, Exit::Forgiven));
                    }
                } else if !cozy && p.patience < p.patience_max * 0.3 {
                    p.satisfaction = (p.satisfaction - 0.5).max(0.0);
                }
            }
            PartyState::Eating => {
                p.stay_left -= 1.0;
                if p.stay_left <= 0.0 {
                    leave.push((p.id, Exit::Normal));
                }
            }
            _ => {}
        }
        if let Some(lb) = p.leave_by
            && m >= lb
            && p.state != PartyState::Leaving
            && !leave.iter().any(|l| l.0 == p.id)
        {
            let exit = if p.state == PartyState::Eating {
                Exit::Normal
            } else if cozy {
                Exit::Forgiven
            } else {
                Exit::Angry
            };
            leave.push((p.id, exit));
        }
    }
    let tick = w.tick;
    for pid in refilled {
        if let Some(p) = w.parties.iter().find(|p| p.id == pid)
            && let Some(l) = p.leader()
            && let Some(g) = w.guests.iter_mut().find(|g| g.id == l)
        {
            g.emote = Some((SimEmote::Coffee, tick + 30));
        }
        w.emit(SimEvent::GuestPatienceRefilled { party: pid });
    }
    for (pid, exit) in leave {
        finish_party(w, content, pid, exit);
    }
    // Emotes for impatient guests.
    let tick = w.tick;
    // (never on Cozy: nothing there turns red)
    let impatient: Vec<u32> = w
        .parties
        .iter()
        .filter(|p| {
            !cozy
                && matches!(
                    p.state,
                    PartyState::Queue | PartyState::WaitOrder | PartyState::WaitFood
                )
                && p.patience < p.patience_max * 0.25
        })
        .map(|p| p.id)
        .collect();
    for g in w.guests.iter_mut() {
        if impatient.contains(&g.party) && g.emote.is_none() {
            g.emote = Some((SimEmote::Angry, tick + 30));
        }
    }
    // Drop guests who left the map.
    w.guests.retain(|g| !g.gone);
    let alive: Vec<u32> = w.guests.iter().map(|g| g.party).collect();
    w.parties
        .retain(|p| p.state != PartyState::Leaving || alive.contains(&p.id));
    if w.tick.is_multiple_of(50) {
        refresh_queue_goals(w, content);
    }
}

/// Called when an order is taken.
pub(crate) fn order_taken(w: &mut World, content: &Content, pid: u32, staff: u32) {
    let m = w.minute();
    let Some(pi) = w.parties.iter().position(|p| p.id == pid) else {
        return;
    };
    let wait = (m - w.parties[pi].arrived) as f32;
    let pmax = w.parties[pi].patience_max;
    let mut items = Vec::new();
    let wishes = w.parties[pi].wishes.clone();
    let members = w.parties[pi].members.clone();
    let wish_party = w.parties[pi].order_kind.is_wish() && w.parties[pi].wish_open;
    // plain items go to the members who didn't wish
    let eaters: Vec<u32> = if wish_party && members.len() > 1 {
        members[1..].to_vec()
    } else {
        members.clone()
    };
    let mut names = Vec::new();
    for (k, wish) in wishes.iter().enumerate() {
        let Some(mi) = content.menu_item(wish) else {
            continue;
        };
        if !item_orderable(w, content, mi) {
            continue;
        }
        let iid = w.alloc_id();
        w.econ.supplies -= mi.cost as i64;
        w.items.push(OrderItem {
            id: iid,
            party: pid,
            guest: eaters[k.min(eaters.len() - 1)],
            menu: wish.clone(),
            state: ItemState::Pending,
            prep_left: mi.prep_minutes * TICKS_PER_MINUTE as f32,
            by: None,
            ready_tile: None,
            quality: 0.0,
            created: m,
            origin: Default::default(),
            price_add: 0,
            hand_off_match: None,
            wish: false,
        });
        items.push(iid);
        names.push(wish.clone());
    }
    let reg = w.parties[pi].register;
    let food_patience = food_patience(w);
    {
        let p = &mut w.parties[pi];
        p.items = items;
        p.ordered_at = Some(m);
        p.wait_at_order = p.wait_clock;
        p.order_claim = None;
        p.satisfaction += 10.0 * (1.0 - (wait / pmax.max(1.0)).min(1.5));
        p.patience = (pmax * food_patience).max(15.0);
        p.state = if p.items.is_empty() && !p.wish_open {
            PartyState::Eating
        } else {
            PartyState::WaitFood
        };
        p.stay_left = 5.0;
    }
    // a wish handed off to the helpers becomes a plain ticket now
    let by_karolina = w.staff_by_id(staff).is_some_and(|s| s.owner);
    if wish_party && w.parties[pi].handed_off {
        super::player::hand_off_item(w, content, pi);
    }
    // Chatty / kind staff taking the order.
    let bonus = service_bonus(w, content, staff, pid);
    w.parties[pi].satisfaction = (w.parties[pi].satisfaction + bonus).clamp(0.0, 100.0);
    if let Some(r) = reg {
        if let Some(q) = w.queues.get_mut(&r) {
            q.retain(|x| *x != pid);
        }
        w.parties[pi].register = None;
        if let Some(&leader) = members.first()
            && let Some(seat) = w
                .guests
                .iter()
                .find(|g| g.id == leader)
                .and_then(|g| g.seat)
            && let Some(t) = seat_tile(w, seat.0, seat.1)
        {
            set_goal(w, leader, t);
        }
        refresh_queue_goals(w, content);
    }
    w.emit(SimEvent::OrderTaken {
        party: pid,
        staff,
        items: names,
    });
    if by_karolina {
        let p = &w.parties[pi];
        let ev = SimEvent::PlayerOrderTaken {
            guest: p.leader().unwrap_or(0),
            party: pid,
            wish: p.order_kind.wish().map(str::to_string),
            kind: p.order_kind.clone(),
        };
        w.emit(ev);
    }
}

/// Patience multiplier while waiting for food.
pub(crate) fn food_patience(w: &World) -> f32 {
    if w.difficulty == Difficulty::Cozy {
        COZY_FOOD_PATIENCE
    } else {
        FOOD_PATIENCE
    }
}

fn trait_effects<'a>(content: &'a Content, s: &Staff) -> Vec<&'a TraitEffect> {
    s.traits
        .iter()
        .filter_map(|t| content.trait_def(t))
        .flat_map(|d| d.effects.iter())
        .collect()
}

fn service_bonus(w: &World, content: &Content, staff: u32, pid: u32) -> f32 {
    let Some(s) = w.staff_by_id(staff) else {
        return 0.0;
    };
    let Some(p) = w.parties.iter().find(|p| p.id == pid) else {
        return 0.0;
    };
    let mut b = 0.0;
    for e in trait_effects(content, s) {
        match e {
            TraitEffect::GuestSatisfaction { add, .. } => b += add * 0.5,
            TraitEffect::ComfortGuest(x) if p.satisfaction < 55.0 => b += x,
            TraitEffect::PatientGuestSatisfaction(x) if p.patience_max >= 15.0 => b += x,
            _ => {}
        }
    }
    b
}

/// An item reached its guest.
pub(crate) fn item_served(w: &mut World, content: &Content, iid: u32, staff: u32) {
    let m = w.minute();
    let Some(ii) = w.items.iter().position(|i| i.id == iid) else {
        return;
    };
    w.items[ii].state = ItemState::Served;
    let pid = w.items[ii].party;
    let menu = w.items[ii].menu.clone();
    let quality = w.items[ii].quality;
    let gid = w.items[ii].guest;
    if let Some(g) = w.guests.iter_mut().find(|g| g.id == gid) {
        g.holding = Some(menu.clone());
        g.emote = Some((SimEmote::Heart, w.tick + 25));
    }
    *w.econ.sold.entry(menu.clone()).or_insert(0) += 1;
    w.tonight.orders_served += 1;
    let bonus = service_bonus(w, content, staff, pid);
    let mood_bonus = content
        .menu_item(&menu)
        .map(|m| m.mood_bonus)
        .unwrap_or(0.0);
    let Some(pi) = w.parties.iter().position(|p| p.id == pid) else {
        return;
    };
    let reg = w.parties[pi].regular.clone();
    let by_karolina = w.staff_by_id(staff).is_some_and(|s| s.owner);
    let karolina_made = w.items[ii].origin.by_player();
    let food_patience = food_patience(w);
    super::orders::on_served(w, content, ii, pi, by_karolina);
    {
        let p = &mut w.parties[pi];
        let wait = (m - p.ordered_at.unwrap_or(m)) as f32;
        let pm = p.patience_max.max(1.0);
        p.satisfaction +=
            8.0 * (1.0 - (wait / pm).min(1.5)) + mood_bonus + quality / 5.0 + bonus * 0.5;
        p.satisfaction = p.satisfaction.clamp(0.0, 100.0);
        p.served = true;
        p.served_by = Some(staff);
        p.patience = (p.patience + 4.0).min(p.patience_max * food_patience);
    }
    if let Some(r) = &reg {
        let st = w.story.regulars.entry(r.clone()).or_default();
        st.served_items.insert(menu.clone());
        // "Served fast" counts the wait clock from the order (walking in from the street and
        // a queue the player can't see from the counter would make the streaks unreachable).
        let p = &w.parties[pi];
        let since = (p.wait_clock - p.wait_at_order).max(0.0).round() as u32;
        if let Some(v) = st.visits.last_mut()
            && v.served_minutes.is_none()
        {
            v.served_minutes = Some(since);
        }
        // Karolina's items earn trust by match & quality (orders::on_served); helpers' the
        // old flat amount
        if !karolina_made {
            let fast = since <= 8;
            let t = w.story.trust.entry(r.clone()).or_insert(0.0);
            *t = (*t + if fast { TRUST_FAST } else { TRUST_SERVED }).min(100.0);
        }
    }
    w.emit(SimEvent::OrderServed {
        party: pid,
        staff,
        item: menu,
    });
    let all_done = w.parties[pi].items.iter().all(|i| {
        w.items
            .iter()
            .find(|x| x.id == *i)
            .is_none_or(|x| matches!(x.state, ItemState::Served | ItemState::Cancelled))
    });
    if all_done && !w.parties[pi].wish_open && w.parties[pi].state == PartyState::WaitFood {
        let arch = content.guest(&w.parties[pi].archetype);
        let (lo, hi) = arch.map(|a| a.stay_minutes).unwrap_or((15, 30));
        let stay_mul = super::music::mood_effect(w, content)
            .map(|m| m.stay_mul)
            .unwrap_or(1.0);
        let stay = (w.rng.range_u32(lo, hi) as f32 * stay_mul).round() as i32 + w.guest_stay_add;
        let p = &mut w.parties[pi];
        p.state = PartyState::Eating;
        p.served_at = Some(m);
        p.stay_left = stay.max(3) as f32;
        // ambience once seated & served
        let amb = ambience(w, content, pid);
        w.parties[pi].satisfaction = (w.parties[pi].satisfaction + amb).clamp(0.0, 100.0);
    }
}

/// Comfort, beauty, dirt, light and music around a party's seats.
fn ambience(w: &World, content: &Content, pid: u32) -> f32 {
    let Some(p) = w.parties.iter().find(|p| p.id == pid) else {
        return 0.0;
    };
    let mut comfort = 0.0;
    let mut beauty = 0.0f32;
    let mut n = 0.0;
    for gid in &p.members {
        let Some(g) = w.guests.iter().find(|g| g.id == *gid) else {
            continue;
        };
        let Some((uid, idx)) = g.seat else { continue };
        let Some(t) = seat_tile(w, uid, idx) else {
            continue;
        };
        n += 1.0;
        if let Some(d) = w
            .furniture_by_uid(uid)
            .and_then(|f| content.furniture_def(&f.def))
        {
            comfort += d.comfort;
        }
        for f in &w.furniture {
            if let Some(d) = content.furniture_def(&f.def)
                && d.beauty != 0.0
                && grid::chebyshev((f.x, f.y), t) <= 3
            {
                beauty += d.beauty;
            }
        }
    }
    if n == 0.0 {
        return 0.0;
    }
    let mut v = (comfort / n) * 1.2 + (beauty / n).min(10.0) * 0.6 - w.dirt * 30.0;
    if w.music_on {
        v += 3.0;
    }
    if w.blackout_until.is_some() {
        v -= 3.0;
    }
    v
}

/// Party pays (if served) and leaves.
pub(crate) fn finish_party(w: &mut World, content: &Content, pid: u32, exit: Exit) {
    let Some(pi) = w.parties.iter().position(|p| p.id == pid) else {
        return;
    };
    // Cozy: nobody leaves angry — they forgive
    let exit = if exit == Exit::Angry && w.difficulty == Difficulty::Cozy {
        Exit::Forgiven
    } else {
        exit
    };
    let angry = exit == Exit::Angry;
    let forgiven = exit == Exit::Forgiven;
    if w.parties[pi].state == PartyState::Leaving {
        return;
    }
    let diff = content.difficulty(w.difficulty.id());
    let income_mul = diff.map(|d| d.income_mul).unwrap_or(1.0);
    let bad_mul = diff.map(|d| d.bad_review_mul).unwrap_or(1.0);
    let arch = content.guest(&w.parties[pi].archetype);
    let reg = w.parties[pi]
        .regular
        .as_ref()
        .and_then(|r| content.regular(r));
    // cancel unserved items
    let items = w.parties[pi].items.clone();
    let mut served_value = 0i64;
    let mut served_n = 0u32;
    for iid in &items {
        if let Some(it) = w.items.iter_mut().find(|x| x.id == *iid) {
            if it.state == ItemState::Served {
                served_n += 1;
                served_value += 0; // computed below with prices
            } else {
                it.state = ItemState::Cancelled;
            }
        }
    }
    for iid in &items {
        if let Some(it) = w
            .items
            .iter()
            .find(|x| x.id == *iid && x.state == ItemState::Served)
        {
            served_value += w.price_of(content, &it.menu) as i64 + it.price_add as i64;
        }
    }
    let size = w.parties[pi].members.len() as u32;
    let mut sat = w.parties[pi].satisfaction;
    if angry {
        sat = (sat - 35.0).max(0.0);
        w.tonight.guests_left_unhappy += size;
        let r = &mut w.parties[pi];
        r.angry = true;
    }
    if forgiven {
        sat = sat.max(content.handson.tuning.cozy_forgiveness.leave_satisfaction);
        w.tonight.guests_forgiven += size;
        w.hands.tally.forgiven_parties += 1;
    }
    // payment
    let spend = arch.map(|a| a.spend_mult).unwrap_or(1.0) * w.parties[pi].spend_mul;
    let mut bill = (served_value as f32 * spend * income_mul).round() as i64;
    let pays = reg.map(|r| r.pays).unwrap_or(Pays::Normal);
    let mut tip = 0i64;
    match pays {
        Pays::OldCoins if served_n > 0 => {
            bill = 0;
            let mut ctx = EffectCtx::default();
            let fx = [
                crate::content::Effect::GiveItem("old_coin".into()),
                crate::content::Effect::AddCounter("coins_collected".into(), 1),
            ];
            super::logic::run(w, content, &EffectSrc::None, &fx, &mut ctx);
        }
        Pays::Song if served_n > 0 => {
            bill = 0;
            for p in w.parties.iter_mut() {
                p.satisfaction = (p.satisfaction + 5.0).min(100.0);
            }
            if !w.music_on {
                w.music_on = true;
                w.emit(SimEvent::MusicChanged { on: true });
            }
        }
        _ => {}
    }
    if bill > 0 {
        w.econ.money += bill;
        w.tonight.revenue += bill;
        w.emit(SimEvent::Payment { amount: bill });
        let tip_chance = arch.map(|a| a.tip_chance).unwrap_or(0.2);
        if pays == Pays::Normal && sat >= 55.0 && w.rng.chance(tip_chance * (sat / 100.0) * 1.6) {
            let tips_mul: f32 = w
                .mods
                .iter()
                .filter(|m| m.kind == ModKind::Tips)
                .map(|m| m.mul)
                .product();
            // Karolina's match & quality and her rhythm (handson.md §1.7), the hall mood
            let f = &w.parties[pi].tip_factors;
            let party_mul = if f.is_empty() {
                1.0
            } else {
                f.iter().sum::<f32>() / f.len() as f32
            };
            let mood_mul = super::music::mood_effect(w, content)
                .map(|m| m.tips_mul)
                .unwrap_or(1.0);
            tip = ((bill as f32 * (0.05 + 0.15 * ((sat - 55.0) / 45.0)))
                * tips_mul
                * party_mul
                * mood_mul)
                .round()
                .max(5.0) as i64;
            // a coin on the table: Karolina (or a dishes helper) puts it in the jar
            let by = w.parties[pi].served_by;
            let table = party_table(w, content, pi);
            let id = w.alloc_id();
            let coin = super::hands::TipCoin {
                id,
                table: table.unwrap_or(0),
                amount: tip,
                staff: by,
                minute: w.minute(),
            };
            match table {
                Some(t) if w.phase == Phase::Night && w.tick < NIGHT_TICKS => {
                    w.hands.tips.push(coin);
                    w.emit(SimEvent::TipLeft {
                        table: t,
                        amount: tip,
                    });
                }
                _ => super::player::collect_coins(w, vec![coin], by),
            }
        }
    }
    w.parties[pi].bill = bill + tip;
    if served_n > 0 && !angry {
        w.tonight.guests_served += size;
    }
    if served_n > 0 || angry || forgiven {
        w.satisfaction_sum += sat * size as f32;
        w.satisfaction_n += size;
    }
    // forgiven guests write no review (handson.md §1.4)
    if (served_n > 0 || angry) && !forgiven {
        // review → reputation
        let mut weight = arch.map(|a| a.review_mult).unwrap_or(1.0);
        let at = w.parties[pi].archetype.clone();
        if let Some(rw) = w
            .narrator
            .review_weights
            .iter_mut()
            .find(|r| r.2 > 0 && r.0.as_ref().is_none_or(|g| *g == at))
        {
            weight *= rw.1;
            rw.2 -= 1;
        }
        w.narrator.review_weights.retain(|r| r.2 > 0);
        let rep = w.econ.reputation;
        let mut delta = (sat - 55.0) * REVIEW_WEIGHT * weight;
        if delta > 0.0 {
            delta *= 1.0 - rep / 120.0;
        } else {
            delta *= bad_mul * (0.4 + rep / 100.0);
        }
        w.econ.reputation = (rep + delta).clamp(0.0, 100.0);
    }
    if let Some(r) = reg {
        let t = w.story.trust.entry(r.id.clone()).or_insert(0.0);
        if angry {
            *t = (*t - TRUST_ANGRY).max(0.0);
        }
    }
    if forgiven {
        w.emit(SimEvent::GuestForgiven {
            party: pid,
            key: FORGIVEN_KEY.into(),
        });
    }
    // free seats, dirty tables
    let members = w.parties[pi].members.clone();
    let wet = if w.weather.is_wet() { 1.5 } else { 1.0 };
    let dirt_mul: f32 = w
        .mods
        .iter()
        .filter(|m| m.kind == ModKind::DirtRate)
        .map(|m| m.mul)
        .product();
    w.dirt = (w.dirt + 0.005 * size as f32 * wet * dirt_mul).min(1.0);
    let mut table_dishes: Vec<(u32, u32)> = Vec::new();
    for gid in &members {
        if let Some(g) = w.guests.iter_mut().find(|g| g.id == *gid) {
            g.holding = None;
            if let Some((uid, idx)) = g.seat.take() {
                if let Some(p) = w.furniture.iter_mut().find(|p| p.uid == uid)
                    && let Some(s) = p.seat_by.get_mut(idx as usize)
                {
                    *s = None;
                }
                if served_n > 0 {
                    table_dishes.push((uid, 1));
                }
            }
        }
    }
    let seat_list = seats(w, content);
    for (seat_uid, n) in table_dishes {
        let table = seat_list
            .iter()
            .find(|s| s.uid == seat_uid)
            .and_then(|s| s.table);
        if let Some(t) = table
            && let Some(p) = w.furniture.iter_mut().find(|p| p.uid == t)
        {
            p.dishes += n;
            p.dirt = (p.dirt + 0.2 * dirt_mul).min(1.0);
        } else {
            w.sink_dishes += n; // no table: guest brings it to the counter
        }
    }
    // leave the queue
    if let Some(r) = w.parties[pi].register.take()
        && let Some(q) = w.queues.get_mut(&r)
    {
        q.retain(|x| *x != pid);
    }
    let regular = w.parties[pi].regular.clone();
    w.parties[pi].state = PartyState::Leaving;
    w.parties[pi].satisfaction = sat;
    let edges = w.grid.street_edges();
    for (k, gid) in members.iter().enumerate() {
        set_goal(w, *gid, edges[(pid as usize + k) % 2]);
        if angry && let Some(g) = w.guests.iter_mut().find(|g| g.id == *gid) {
            g.emote = Some((SimEmote::Angry, w.tick + 40));
        }
    }
    if let Some(r) = regular
        && let Some(st) = w.story.regulars.get_mut(&r)
    {
        st.party = None;
    }
    w.emit(SimEvent::GuestLeft {
        party: pid,
        satisfaction: sat,
        served: served_n > 0,
    });
    refresh_queue_goals(w, content);
}

/// The table at the leader's seat (where the tip coin goes).
fn party_table(w: &World, content: &Content, pi: usize) -> Option<u32> {
    let leader = w.parties[pi].leader()?;
    let (uid, _) = w.guests.iter().find(|g| g.id == leader)?.seat?;
    seats(w, content)
        .iter()
        .find(|s| s.uid == uid)
        .and_then(|s| s.table)
}

/// 06:00: everyone finishes up.
pub(crate) fn close_up(w: &mut World, content: &Content) {
    let ids: Vec<(u32, PartyState)> = w.parties.iter().map(|p| (p.id, p.state)).collect();
    for (id, st) in ids {
        match st {
            PartyState::Leaving => {}
            PartyState::Eating => finish_party(w, content, id, Exit::Normal),
            _ => {
                // not their fault: closing time, they just go
                finish_party(w, content, id, Exit::Normal);
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Movement
// ---------------------------------------------------------------------------------------------

/// Moves `pos` along `path`. Returns (velocity, arrived, blocked_next).
pub(crate) fn step_along(
    grid: &grid::Grid,
    pos: &mut (f32, f32),
    path: &mut Vec<(i32, i32)>,
    speed_tiles: f32,
    guest: bool,
) -> ((f32, f32), bool, bool) {
    let mut budget = speed_tiles * TILE;
    let start = *pos;
    while budget > 0.0 {
        let Some(&next) = path.first() else {
            break;
        };
        let is_last = path.len() == 1;
        let pass = if guest {
            grid.guest_ok(next.0, next.1) || grid.walkable(next.0, next.1)
        } else {
            grid.walkable(next.0, next.1)
        };
        if !pass && !is_last {
            return ((pos.0 - start.0, pos.1 - start.1), false, true);
        }
        let c = grid::tile_center(next);
        let dx = c.0 - pos.0;
        let dy = c.1 - pos.1;
        let d = (dx * dx + dy * dy).sqrt();
        if d <= budget {
            *pos = c;
            budget -= d;
            path.remove(0);
        } else {
            pos.0 += dx / d * budget;
            pos.1 += dy / d * budget;
            budget = 0.0;
        }
    }
    ((pos.0 - start.0, pos.1 - start.1), path.is_empty(), false)
}

pub(crate) fn tick_guests(w: &mut World, content: &Content) {
    let _ = content;
    let n = w.guests.len();
    let mut repath: Vec<u32> = Vec::new();
    let mut gone: Vec<u32> = Vec::new();
    let edges = w.grid.street_edges();
    for i in 0..n {
        let g = &mut w.guests[i];
        if g.gone {
            continue;
        }
        if g.path.is_empty() {
            g.stuck_ticks = 0;
            continue;
        }
        let (vel, arrived, blocked) =
            step_along(&w.grid, &mut g.pos, &mut g.path, GUEST_SPEED, true);
        if blocked {
            repath.push(g.id);
            g.stuck_ticks += 1;
            continue;
        }
        if vel.0.abs() + vel.1.abs() > 0.01 {
            g.facing = Facing::from_delta(vel.0, vel.1);
            g.stuck_ticks = 0;
        } else {
            g.stuck_ticks += 1;
        }
        if arrived {
            let tile = g.tile();
            if g.seat.is_some() && g.goal == Some(tile) {
                g.pose = Pose::Sit;
            } else if edges.contains(&tile) && g.goal == Some(tile) {
                gone.push(g.id);
            } else {
                g.pose = Pose::Idle;
            }
        }
    }
    // seated facing
    if !w.guests.iter().all(|g| g.pose != Pose::Sit) {
        let seat_info = seats(w, content);
        for g in w.guests.iter_mut() {
            if g.pose == Pose::Sit
                && let Some((uid, idx)) = g.seat
                && let Some(s) = seat_info.iter().find(|s| s.uid == uid && s.idx == idx)
            {
                g.facing = s.facing;
            }
        }
    }
    for id in repath {
        if let Some(goal) = w.guests.iter().find(|g| g.id == id).and_then(|g| g.goal) {
            set_goal(w, id, goal);
        }
    }
    let leaving: Vec<u32> = w
        .parties
        .iter()
        .filter(|p| p.state == PartyState::Leaving)
        .map(|p| p.id)
        .collect();
    for g in w.guests.iter_mut() {
        if gone.contains(&g.id) && leaving.contains(&g.party) {
            g.gone = true;
        }
        // safety: stuck way too long → teleport along
        if g.stuck_ticks > 600 {
            if let Some(goal) = g.goal {
                g.pos = grid::tile_center(goal);
                g.path.clear();
            }
            g.stuck_ticks = 0;
            w.tonight.stuck_resets += 1;
            if leaving.contains(&g.party) {
                g.gone = true;
            }
        }
    }
}
