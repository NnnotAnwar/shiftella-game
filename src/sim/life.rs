//! Staff idle life: what a helper does with a free moment, so nobody stands at their post
//! like a bot when their duties have nothing to do.
//!
//! A helper whose duties offer no task (`Task::Idle`) picks a small plausible pastime by
//! their duties and where they stand — polish cups at the back bar, wipe a free table,
//! straighten chairs, sweep by the door, a sip of water, a chat with an idle colleague, a
//! look out of the window when a train passes, a pet for the station cat… — walks a few
//! tiles to it, does it for a few seconds to a minute, then waits a moment and picks the
//! next. The work loop in `staff::tick_staff` still looks for real work every other tick,
//! and the moment it finds some the pastime is dropped (`stop`): a pastime never claims
//! anything, never steals a task and never blocks one.
//!
//! **Balance & determinism.** Pastimes never touch the world RNG (`World::rng`): every roll
//! comes from a derived stream of `(seed, staff, night, beat)` (like `sim::names`). Their
//! only effect on the café is a sliver less dirt on a free table a helper wipes (no dishes
//! on it). Moods, energy (the idle drain stays), relationships, the cat's affection and
//! every counter are untouched. All the state is `#[serde(default)]` on `Staff`.

use serde::{Deserialize, Serialize};

use super::grid::{self, Room, TileKind};
use super::guests;
use super::rng::Rng;
use super::types::*;
use super::world::*;
use crate::content::Content;
use crate::content::handson::Duty;

/// A small thing a helper does while their duties have nothing for them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Pastime {
    /// Wipes the bar counter (staff side).
    WipeCounter,
    /// Polishes cups at the coffee machine / back bar.
    PolishCups,
    /// Stacks saucers by the sink.
    StackSaucers,
    /// Checks the pastries in the fridge / oven.
    CheckPastry,
    /// Refills the napkins at the counter.
    RefillNapkins,
    /// Wipes a free table (a sliver less dirt on it).
    WipeTable,
    /// Straightens the chairs of a free table.
    StraightenChairs,
    /// Refills the sugar on a free table.
    RefillSugar,
    /// Sweeps by the door.
    Sweep,
    /// A sip of water behind the counter.
    SipWater,
    /// Leans on the counter and chats with an idle colleague.
    Chat,
    /// Glances at the phone (rarely).
    Phone,
    /// Looks out of the window while a train passes.
    Window,
    /// Pets the station cat.
    PetCat,
}

impl Pastime {
    pub const ALL: [Pastime; 14] = [
        Pastime::WipeCounter,
        Pastime::PolishCups,
        Pastime::StackSaucers,
        Pastime::CheckPastry,
        Pastime::RefillNapkins,
        Pastime::WipeTable,
        Pastime::StraightenChairs,
        Pastime::RefillSugar,
        Pastime::Sweep,
        Pastime::SipWater,
        Pastime::Chat,
        Pastime::Phone,
        Pastime::Window,
        Pastime::PetCat,
    ];

    /// `life.do.<id>` (the card's "Now" line).
    pub fn id(self) -> &'static str {
        match self {
            Pastime::WipeCounter => "wipe_counter",
            Pastime::PolishCups => "polish_cups",
            Pastime::StackSaucers => "stack_saucers",
            Pastime::CheckPastry => "check_pastry",
            Pastime::RefillNapkins => "refill_napkins",
            Pastime::WipeTable => "wipe_table",
            Pastime::StraightenChairs => "straighten_chairs",
            Pastime::RefillSugar => "refill_sugar",
            Pastime::Sweep => "sweep",
            Pastime::SipWater => "sip_water",
            Pastime::Chat => "chat",
            Pastime::Phone => "phone",
            Pastime::Window => "window",
            Pastime::PetCat => "pet_cat",
        }
    }

    /// The hands are busy (the person is drawn working) — or they just stand (a sip, a
    /// chat, the phone, the window).
    pub fn hands(self) -> bool {
        !matches!(
            self,
            Pastime::SipWater | Pastime::Chat | Pastime::Phone | Pastime::Window
        )
    }

    /// Length range in ticks (10 ticks = 1 game minute ≈ 1.3 real seconds at ×1).
    fn length(self) -> (u32, u32) {
        match self {
            Pastime::WipeCounter => (60, 120),
            Pastime::PolishCups => (90, 220),
            Pastime::StackSaucers => (60, 140),
            Pastime::CheckPastry => (40, 80),
            Pastime::RefillNapkins => (50, 100),
            Pastime::WipeTable => (50, 100),
            Pastime::StraightenChairs => (40, 80),
            Pastime::RefillSugar => (40, 70),
            Pastime::Sweep => (80, 180),
            Pastime::SipWater => (30, 50),
            Pastime::Chat => (120, 360),
            Pastime::Phone => (50, 110),
            Pastime::Window => (60, 110),
            Pastime::PetCat => (50, 90),
        }
    }
}

/// What a helper is doing with a free moment (`Staff::life`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IdleLife {
    pub kind: Pastime,
    /// Where they do it.
    pub spot: (i32, i32),
    /// What they look at while at it.
    pub look: (i32, i32),
    /// The colleague of a chat.
    pub with: Option<u32>,
    /// The table they wipe / straighten.
    pub piece: Option<u32>,
    /// Ticks it takes at the spot, and ticks done.
    pub len: u32,
    pub done: u32,
    /// Ticks spent walking to the spot (a long detour gives it up).
    pub walked: u32,
}

impl IdleLife {
    pub fn progress(&self) -> f32 {
        (self.done as f32 / self.len.max(1) as f32).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
thread_local! {
    /// Tests: no pastimes at all (the old "stand at the post" behaviour), for comparisons.
    pub(crate) static OFF: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Ticks between two pastimes (they just stand a moment: weight shifts, a glance).
const PAUSE: (u32, u32) = (25, 90);
/// A walk to a pastime longer than this is given up.
pub(crate) const MAX_WALK: u32 = 60;

/// A clock that runs through the preparation half hour too (the night tick stands still).
pub(crate) fn beat(w: &World) -> u32 {
    w.tick + PREP_TICKS.saturating_sub(w.prep_left)
}

/// A derived random stream: the world RNG is never touched.
fn roll(w: &World, idx: usize, salt: u64) -> Rng {
    let s = &w.staff[idx];
    let mix = |x: u64, k: u64| x.wrapping_mul(k).rotate_left(29);
    Rng::new(
        w.seed
            ^ mix(s.id as u64 + 1, 0x9E37_79B9_7F4A_7C15)
            ^ mix(w.night as u64 + 7, 0xC2B2_AE3D_27D4_EB4F)
            ^ mix(beat(w) as u64 + 13, 0x1656_67B1_9E37_79F9)
            ^ mix(salt, 0x27D4_EB2F_1656_67C5)
            ^ 0x6c69_6665_5f76_3031,
    )
}

/// The pause before the next pastime (after a task or a pastime ended).
pub(crate) fn rest(w: &mut World, idx: usize) {
    let mut r = roll(w, idx, 1);
    w.staff[idx].life_rest = beat(w) + r.range_u32(PAUSE.0, PAUSE.1);
}

/// Real work (or a break, the end of the shift) came: drop the pastime at once.
pub(crate) fn stop(w: &mut World, idx: usize) {
    if let Some(l) = w.staff[idx].life.take() {
        // the little chat bubbles go with it
        if matches!(
            w.staff[idx].emote,
            Some((SimEmote::Smile | SimEmote::Heart, _))
        ) {
            w.staff[idx].emote = None;
        }
        if l.kind == Pastime::Chat
            && let Some(o) = l
                .with
                .and_then(|id| w.staff.iter().position(|s| s.id == id))
            && w.staff[o]
                .life
                .as_ref()
                .is_some_and(|x| x.kind == Pastime::Chat)
        {
            w.staff[o].life = None;
            rest(w, o);
        }
    }
}

fn free_for_life(s: &Staff) -> bool {
    !s.owner && s.on_duty() && matches!(s.task, Some(Task::Idle { .. }))
}

/// One tick of an idle helper standing at their spot (`Task::Idle`, no path left).
pub(crate) fn step(w: &mut World, content: &Content, idx: usize) {
    let b = beat(w);
    let Some(mut l) = w.staff[idx].life.clone() else {
        w.staff[idx].pose = Pose::Idle;
        // behind the counter they face the hall
        let here = w.staff[idx].tile();
        if w.grid.room(here.0, here.1).staff_only() {
            w.staff[idx].facing = Facing::Up;
        }
        if b >= w.staff[idx].life_rest {
            start(w, content, idx);
        }
        return;
    };
    let me = w.staff[idx].id;
    let here = w.staff[idx].tile();
    if here != l.spot {
        // the path ended short of the spot (blocked): try once more, else give it up
        if l.walked < MAX_WALK && super::staff::go(w, idx, l.spot) {
            return;
        }
        w.staff[idx].life = None;
        rest(w, idx);
        return;
    }
    if l.kind == Pastime::Chat {
        let partner = l
            .with
            .and_then(|id| w.staff.iter().position(|s| s.id == id));
        let Some(o) = partner.filter(|&o| free_for_life(&w.staff[o])) else {
            finish(w, idx);
            return;
        };
        // the colleague joins (or is already chatting with me)
        match &w.staff[o].life {
            Some(x) if x.kind == Pastime::Chat && x.with == Some(me) => {}
            None if w.staff[o].path.is_empty() => {
                let ot = w.staff[o].tile();
                w.staff[o].life = Some(IdleLife {
                    kind: Pastime::Chat,
                    spot: ot,
                    look: here,
                    with: Some(me),
                    piece: None,
                    len: l.len.saturating_sub(l.done).max(20),
                    done: 0,
                    walked: 0,
                });
            }
            _ => {
                finish(w, idx);
                return;
            }
        }
        l.look = w.staff[o].tile();
        if (b + me * 37).is_multiple_of(70) && w.staff[idx].emote.is_none() {
            w.staff[idx].emote = Some((SimEmote::Smile, w.tick + 14));
        }
    }
    // look at it, do it
    let d = (l.look.0 - here.0, l.look.1 - here.1);
    if d != (0, 0) {
        w.staff[idx].facing = Facing::from_delta(d.0 as f32, d.1 as f32);
    }
    w.staff[idx].pose = if l.kind.hands() {
        Pose::Work
    } else {
        Pose::Idle
    };
    if l.kind == Pastime::Window && (b + me).is_multiple_of(90) && w.staff[idx].emote.is_none() {
        w.staff[idx].emote = Some((SimEmote::Music, w.tick + 12));
    }
    l.done += 1;
    let over = l.done >= l.len;
    w.staff[idx].life = Some(l);
    if over {
        finish(w, idx);
    }
}

/// The pastime is done: its tiny effect, a pause, then the next one.
fn finish(w: &mut World, idx: usize) {
    let Some(l) = w.staff[idx].life.take() else {
        return;
    };
    match l.kind {
        Pastime::WipeTable => {
            if let Some(p) = l
                .piece
                .and_then(|u| w.furniture.iter_mut().find(|p| p.uid == u))
                && p.dishes == 0
                && p.claim.is_none()
            {
                p.dirt = (p.dirt - 0.05).max(0.0);
            }
        }
        Pastime::PetCat if w.staff[idx].emote.is_none() => {
            w.staff[idx].emote = Some((SimEmote::Heart, w.tick + 16));
        }
        _ => {}
    }
    if l.kind == Pastime::Chat
        && let Some(o) = l
            .with
            .and_then(|id| w.staff.iter().position(|s| s.id == id))
        && w.staff[o]
            .life
            .as_ref()
            .is_some_and(|x| x.kind == Pastime::Chat && x.with == Some(w.staff[idx].id))
    {
        w.staff[o].life = None;
        rest(w, o);
    }
    rest(w, idx);
}

/// Weights of the pastimes a helper's duties suggest.
fn weights(duties: &[Duty]) -> [f32; 14] {
    use Pastime as P;
    let mut wt = [0.0f32; 14];
    let mut add = |p: P, x: f32| {
        let i = Pastime::ALL.iter().position(|q| *q == p).unwrap_or(0);
        wt[i] = wt[i].max(x);
    };
    if duties.is_empty() {
        add(P::WipeCounter, 2.0);
        add(P::PolishCups, 2.0);
        add(P::SipWater, 1.0);
    }
    for d in duties {
        match d {
            Duty::Register => {
                add(P::WipeCounter, 3.0);
                add(P::RefillNapkins, 3.0);
                add(P::PolishCups, 1.0);
            }
            Duty::Bar => {
                add(P::PolishCups, 4.0);
                add(P::WipeCounter, 2.0);
                add(P::StackSaucers, 1.0);
            }
            Duty::Kitchen => {
                add(P::CheckPastry, 3.0);
                add(P::WipeCounter, 2.0);
                add(P::StackSaucers, 1.5);
            }
            Duty::Dishes => {
                add(P::StackSaucers, 3.0);
                add(P::WipeTable, 3.0);
            }
            Duty::Floor => {
                add(P::Sweep, 4.0);
                add(P::StraightenChairs, 3.0);
                add(P::WipeTable, 1.0);
            }
            Duty::Serving => {
                add(P::RefillSugar, 3.0);
                add(P::WipeTable, 2.0);
                add(P::StraightenChairs, 2.0);
                add(P::PolishCups, 1.0);
            }
        }
    }
    add(P::SipWater, 1.0);
    add(P::Chat, 2.0);
    add(P::Phone, 0.35);
    add(P::PetCat, 1.2);
    // a passing train beats everything (checked when picking)
    add(P::Window, 8.0);
    wt
}

/// How far from their post a helper goes for it: the register stays by the queue, the bar
/// and the kitchen stay behind the counter, the hall duties roam the hall a little. The
/// window and the cat only for those whose post is near them.
fn radius(duties: &[Duty], kind: Pastime) -> i32 {
    let hall = duties.is_empty()
        || duties
            .iter()
            .all(|d| matches!(d, Duty::Dishes | Duty::Floor | Duty::Serving));
    let base = if duties.contains(&Duty::Register) {
        2
    } else if hall {
        7
    } else {
        4
    };
    match kind {
        Pastime::Window | Pastime::PetCat => base.min(6),
        Pastime::Chat => base.min(5),
        _ => base,
    }
}

/// Picks and starts the next pastime (or just waits a little longer).
fn start(w: &mut World, content: &Content, idx: usize) {
    #[cfg(test)]
    if OFF.with(|c| c.get()) {
        rest(w, idx);
        return;
    }
    let duties = w.staff[idx].duties.clone();
    let post = super::staff::idle_tile(w, content, idx);
    let mut wt = weights(&duties);
    let slot = |p: Pastime| Pastime::ALL.iter().position(|q| *q == p).unwrap_or(0);
    if !train_passing(w, content) {
        wt[slot(Pastime::Window)] = 0.0;
    }
    if super::moments::cat_view(w, content).is_none_or(|c| c.perch == "follows") {
        wt[slot(Pastime::PetCat)] = 0.0;
    }
    let mut r = roll(w, idx, 2);
    // a few tries: the pick may have no spot right now
    for _ in 0..4 {
        let Some(i) = r.weighted(&wt) else { break };
        let kind = Pastime::ALL[i];
        wt[i] = 0.0;
        let rad = radius(&duties, kind);
        if let Some((spot, look, with, piece)) = spot_for(w, content, idx, kind, post, rad, &mut r)
        {
            let (lo, hi) = kind.length();
            let len = r.range_u32(lo, hi);
            let goal = spot;
            w.staff[idx].life = Some(IdleLife {
                kind,
                spot,
                look,
                with,
                piece,
                len,
                done: 0,
                walked: 0,
            });
            w.staff[idx].task = Some(Task::Idle { tile: goal });
            if w.staff[idx].tile() != goal && !super::staff::go(w, idx, goal) {
                w.staff[idx].life = None;
                continue;
            }
            return;
        }
    }
    rest(w, idx);
}

/// Tiles other people stand on or are heading to (no two helpers on one spot).
fn taken(w: &World, idx: usize) -> Vec<(i32, i32)> {
    let mut v = Vec::new();
    for (i, s) in w.staff.iter().enumerate() {
        if i == idx || !s.present() {
            continue;
        }
        v.push(s.tile());
        if let Some(t) = s.path.last() {
            v.push(*t);
        }
        if let Some(l) = &s.life {
            v.push(l.spot);
        }
    }
    v.extend(w.guests.iter().filter(|g| !g.gone).map(|g| g.tile()));
    v
}

/// A free floor tile next to `t` (no object, nobody on it), nearest to `from`.
fn beside(w: &World, t: (i32, i32), from: (i32, i32), avoid: &[(i32, i32)]) -> Option<(i32, i32)> {
    let mut best: Option<(i32, i32)> = None;
    for (dx, dy) in [
        (0, 1),
        (0, -1),
        (1, 0),
        (-1, 0),
        (1, 1),
        (-1, 1),
        (1, -1),
        (-1, -1),
    ] {
        let n = (t.0 + dx, t.1 + dy);
        if !w.grid.walkable(n.0, n.1)
            || !w.grid.is_indoor_floor(n.0, n.1)
            || w.grid.object_at(n.0, n.1).is_some()
            || avoid.contains(&n)
        {
            continue;
        }
        if best.is_none_or(|b| grid::manhattan(n, from) < grid::manhattan(b, from)) {
            best = Some(n);
        }
    }
    best
}

/// Staff-side access tiles of the pieces with `tag`, with the piece tile to look at.
fn staff_side(w: &World, content: &Content, tag: &str) -> Vec<((i32, i32), (i32, i32))> {
    let mut out = Vec::new();
    for p in &w.furniture {
        let Some(d) = content.furniture_def(&p.def) else {
            continue;
        };
        if !d.has_tag(tag) {
            continue;
        }
        for t in grid::access_tiles(&w.grid, d, p) {
            if w.grid.room(t.0, t.1).staff_only() && w.grid.object_at(t.0, t.1).is_none() {
                out.push((t, (p.x, p.y)));
            }
        }
    }
    out
}

/// Tables nobody sits at, waits for or works on (no dishes on them).
fn free_tables(w: &World, content: &Content) -> Vec<u32> {
    let seats = guests::seats(w, content);
    w.furniture
        .iter()
        .filter(|p| {
            content
                .furniture_def(&p.def)
                .is_some_and(|d| d.has_tag("table"))
                && p.dishes == 0
                && p.claim.is_none()
                && !seats.iter().any(|s| s.table == Some(p.uid) && !s.free)
        })
        .map(|p| p.uid)
        .collect()
}

/// A train at the station within a couple of minutes (everyone glances out).
fn train_passing(w: &World, content: &Content) -> bool {
    if super::storm::line_closed(w) {
        return false;
    }
    let m = w.minutes_since_open();
    content.trains.trains.iter().any(|t| {
        let at = t.arrives.minutes_since_open();
        t.announce_if.is_empty() && (m - 1..=m + 3).contains(&at)
    })
}

type Spot = ((i32, i32), (i32, i32), Option<u32>, Option<u32>);

/// Where to do `kind` (spot, look-at tile, colleague, piece), within `rad` of the post.
fn spot_for(
    w: &World,
    content: &Content,
    idx: usize,
    kind: Pastime,
    post: (i32, i32),
    rad: i32,
    r: &mut Rng,
) -> Option<Spot> {
    let here = w.staff[idx].tile();
    let avoid = taken(w, idx);
    let near = |t: (i32, i32)| grid::manhattan(t, post) <= rad && !avoid.contains(&t);
    let pick = |r: &mut Rng, v: Vec<((i32, i32), (i32, i32))>| -> Option<Spot> {
        let v: Vec<_> = v.into_iter().filter(|(t, _)| near(*t)).collect();
        r.pick(&v).map(|(t, l)| (*t, *l, None, None))
    };
    match kind {
        Pastime::WipeCounter | Pastime::RefillNapkins => {
            let mut v = staff_side(w, content, "counter");
            if kind == Pastime::RefillNapkins {
                v.extend(staff_side(w, content, "register"));
            }
            pick(r, v)
        }
        Pastime::PolishCups => {
            let mut v = staff_side(w, content, "station:coffee");
            v.extend(staff_side(w, content, "counter"));
            pick(r, v)
        }
        Pastime::StackSaucers => {
            let mut v = staff_side(w, content, "dishes");
            if v.is_empty() {
                v = staff_side(w, content, "counter");
            }
            pick(r, v)
        }
        Pastime::CheckPastry => {
            let mut v = staff_side(w, content, "station:fridge");
            v.extend(staff_side(w, content, "station:oven"));
            pick(r, v)
        }
        Pastime::SipWater | Pastime::Phone => {
            let mut v = staff_side(w, content, "sink");
            if kind == Pastime::Phone || v.is_empty() {
                // a quiet staff tile, looking at the camera
                v = (0..w.grid.height)
                    .flat_map(|y| (0..w.grid.width).map(move |x| (x, y)))
                    .filter(|t| {
                        w.grid.room(t.0, t.1).staff_only()
                            && w.grid.walkable(t.0, t.1)
                            && w.grid.object_at(t.0, t.1).is_none()
                    })
                    .map(|t| (t, (t.0, t.1 + 1)))
                    .collect();
            }
            pick(r, v)
        }
        Pastime::WipeTable | Pastime::StraightenChairs | Pastime::RefillSugar => {
            let mut opts = Vec::new();
            for uid in free_tables(w, content) {
                let Some(p) = w.furniture_by_uid(uid) else {
                    continue;
                };
                let target = if kind == Pastime::StraightenChairs {
                    // stand by one of its chairs
                    let seats = guests::seats(w, content);
                    match seats.iter().find(|s| s.table == Some(uid)) {
                        Some(s) => s.tile,
                        None => continue,
                    }
                } else {
                    (p.x, p.y)
                };
                if let Some(t) = beside(w, target, here, &avoid)
                    && grid::manhattan(t, post) <= rad
                {
                    opts.push(((t, target), uid));
                }
            }
            r.pick(&opts)
                .map(|((t, l), uid)| (*t, *l, None, Some(*uid)))
        }
        Pastime::Sweep => {
            let door = w.grid.entrance();
            let mut queue = Vec::new();
            for (reg, _) in guests::registers(w, content) {
                queue.extend(guests::queue_slots(w, content, reg));
            }
            let seat_tiles: Vec<_> = guests::seats(w, content).iter().map(|s| s.tile).collect();
            let v: Vec<_> = (0..w.grid.height)
                .flat_map(|y| (0..w.grid.width).map(move |x| (x, y)))
                .filter(|t| {
                    w.grid.room(t.0, t.1) == Room::Hall
                        && w.grid.is_indoor_floor(t.0, t.1)
                        && w.grid.walkable(t.0, t.1)
                        && w.grid.object_at(t.0, t.1).is_none()
                        && grid::manhattan(*t, door) <= 3
                        && !queue.contains(t)
                        && !seat_tiles.contains(t)
                })
                .map(|t| (t, (t.0 + 1, t.1)))
                .collect();
            // only when they are over that side of the hall anyway
            let v: Vec<_> = v.into_iter().filter(|(t, _)| !avoid.contains(t)).collect();
            if v.is_empty() || grid::manhattan(door, here) > 12 {
                return None;
            }
            r.pick(&v).map(|(t, l)| (*t, *l, None, None))
        }
        Pastime::Chat => {
            let me = w.staff[idx].id;
            let mates: Vec<usize> = (0..w.staff.len())
                .filter(|&o| {
                    let s = &w.staff[o];
                    o != idx
                        && free_for_life(s)
                        && s.life.is_none()
                        && s.path.is_empty()
                        && grid::manhattan(s.tile(), post) <= rad
                        && !w.staff.iter().any(|x| {
                            x.life
                                .as_ref()
                                .is_some_and(|l| l.with == Some(s.id) && x.id != me)
                        })
                })
                .collect();
            let o = *r.pick(&mates)?;
            let ot = w.staff[o].tile();
            let t = beside(w, ot, here, &avoid)?;
            Some((t, ot, Some(w.staff[o].id), None))
        }
        Pastime::Window => {
            if !train_passing(w, content) {
                return None;
            }
            let mut best: Option<((i32, i32), (i32, i32))> = None;
            for y in 0..w.grid.height {
                for x in 0..w.grid.width {
                    if !w
                        .grid
                        .tile(x, y)
                        .is_some_and(|t| t.kind == TileKind::Window)
                    {
                        continue;
                    }
                    let n = (x, y + 1);
                    if !w.grid.walkable(n.0, n.1)
                        || !w.grid.is_indoor_floor(n.0, n.1)
                        || w.grid.object_at(n.0, n.1).is_some()
                        || avoid.contains(&n)
                        || grid::manhattan(n, post) > rad
                    {
                        continue;
                    }
                    if best.is_none_or(|b| grid::manhattan(n, here) < grid::manhattan(b.0, here)) {
                        best = Some((n, (x, y)));
                    }
                }
            }
            best.map(|(t, l)| (t, l, None, None))
        }
        Pastime::PetCat => {
            let cat = super::moments::cat_view(w, content)?;
            if cat.perch == "follows" {
                return None;
            }
            let t = beside(w, cat.tile, here, &avoid)?;
            (grid::manhattan(t, post) <= rad).then_some((t, cat.tile, None, None))
        }
    }
}

/// What a helper is doing right now, for the ring over their head.
#[derive(Debug, Clone, PartialEq)]
pub enum HelperDoing {
    /// A real task (`Task::id()`), with the menu item for a brew / a tray.
    Task {
        id: &'static str,
        menu: Option<String>,
    },
    Pastime(Pastime),
}

/// A helper at work (or at a pastime), with its progress 0–1 (`World::helper_work`).
#[derive(Debug, Clone, PartialEq)]
pub struct HelperWork {
    pub staff: u32,
    pub doing: HelperDoing,
    pub progress: f32,
}

/// Tasks shorter than this (work units) get no ring (a tray handed over, a quick pick-up).
const RING_MIN_WORK: f32 = 5.0;

fn helper_work(w: &World, s: &Staff) -> Option<HelperWork> {
    if s.owner || !s.on_duty() || !s.path.is_empty() {
        return None;
    }
    let (doing, progress) = match &s.task {
        Some(Task::Idle { .. }) => {
            let l = s.life.as_ref()?;
            if s.tile() != l.spot {
                return None;
            }
            (HelperDoing::Pastime(l.kind), l.progress())
        }
        Some(Task::Break { .. } | Task::Leave) | None => return None,
        Some(t) => {
            if s.work_total < RING_MIN_WORK {
                return None;
            }
            let menu = match t {
                Task::Prepare { item, .. } | Task::Serve { item, .. } => w
                    .items
                    .iter()
                    .find(|i| i.id == *item)
                    .map(|i| i.menu.clone()),
                _ => None,
            };
            let p = 1.0 - s.work_left.max(0.0) / s.work_total;
            (HelperDoing::Task { id: t.id(), menu }, p.clamp(0.0, 1.0))
        }
    };
    Some(HelperWork {
        staff: s.id,
        doing,
        progress,
    })
}

impl World {
    /// Helpers at work right now (at the spot, not walking), with the progress of the task or
    /// the pastime — the small rings over their heads. Karolina is never in it (her queue
    /// chips have their own ring).
    pub fn helper_work(&self) -> Vec<HelperWork> {
        self.staff
            .iter()
            .filter_map(|s| helper_work(self, s))
            .collect()
    }

    /// A helper's pastime (`None` while working, resting or away).
    pub fn pastime(&self, staff: u32) -> Option<&IdleLife> {
        self.staff
            .iter()
            .find(|s| s.id == staff)
            .filter(|s| free_for_life(s))
            .and_then(|s| s.life.as_ref())
    }
}
