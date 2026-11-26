//! Night moments (design/handson.md §5) and the small extras that ride on them (§6): the
//! moment director (2–4 a night, conditions, cooldowns, anchors, event replacements, the
//! lull director's nudge), resolving an option (Karolina walks to the anchor, the option
//! applies on arrival), the station cat's friendship, thank-you napkins and tip-jar goals.

use super::grid::{self, TileKind};

use super::staff;
use super::systems::{ActiveMoment, MomentPos, XTarget};
use super::types::*;
use super::world::*;
use super::xlogic;
use crate::content::handson::{
    AgentSelector, CatPerk, GuestFilter, HighlightKind, Interaction, MomentAnchor, MomentDef,
};
use crate::content::{Content, Effect, Job};

/// No moment appears after this minute (05:30): the night winds down.
const LAST_MOMENT_MINUTE: u32 = 450;
/// Nights on which never two moments are open at once (§9).
const ONE_AT_A_TIME_UNTIL: u32 = 3;
/// At most this many moments open at once later.
const MAX_OPEN: usize = 2;
/// Event moments wait at most this long for room.
const WAIT_MINUTES: u32 = 20;
/// Weight × in the quiet hour for `quiet` moments, and for the others.
const QUIET_PREF: (f32, f32) = (3.0, 0.3);
/// Phase moments (the season's own) are picked first while eligible.
const PHASE_PRIORITY: f32 = 6.0;

// ---------------------------------------------------------------------------------------------
// Night bookkeeping
// ---------------------------------------------------------------------------------------------

fn act_of(w: &World, content: &Content) -> u32 {
    match w.mode {
        Mode::Story => content
            .story
            .acts
            .iter()
            .find(|a| (a.nights.0..=a.nights.1).contains(&w.night))
            .map(|a| a.act)
            .unwrap_or(1),
        Mode::Sandbox => 2,
    }
}

pub(crate) fn on_night_start(w: &mut World, content: &Content) {
    let t = &content.handson.tuning;
    let (lo, hi) = t.moment_budget(act_of(w, content));
    let budget = w.rng.range_u32(lo, hi.max(lo)).min(t.moments_max_per_night);
    w.sys.moments.cap = hi.max(lo).min(t.moments_max_per_night);
    let ms = &mut w.sys.moments;
    ms.active.clear();
    ms.waiting.clear();
    ms.tonight = 0;
    ms.tonight_ids.clear();
    ms.last_minute = None;
    ms.budget = budget;
    let ex = &content.handson.extras;
    w.sys.extras.cat_here = w.sys.extras.cat_level >= 4;
    // the cat "works here": the crew likes it
    for lv in &ex.cat.levels {
        if lv.level > w.sys.extras.cat_level {
            continue;
        }
        for p in &lv.perks {
            if let CatPerk::WorksHere { staff_mood } = p {
                for s in w.staff.iter_mut().filter(|s| !s.owner) {
                    s.mood = (s.mood + staff_mood).min(100.0);
                }
            }
        }
    }
}

/// 06:00: moments still open fade (their `on_expire` applies, e.g. the crew hangs the
/// garlands in the morning).
pub(crate) fn on_night_end(w: &mut World, content: &Content) {
    let open: Vec<String> = w.sys.moments.active.iter().map(|a| a.id.clone()).collect();
    for id in open {
        expire(w, content, &id);
    }
    w.sys.moments.waiting.clear();
}

// ---------------------------------------------------------------------------------------------
// Anchors
// ---------------------------------------------------------------------------------------------

fn tagged(w: &World, content: &Content, tag: &str) -> Option<u32> {
    w.furniture
        .iter()
        .filter(|p| {
            content
                .furniture_def(&p.def)
                .is_some_and(|d| d.has_tag(tag))
        })
        .map(|p| p.uid)
        .min()
}

fn counter_tile(w: &World, content: &Content) -> (i32, i32) {
    staff::checklist_tile(w, content, Job::Register)
}

/// An indoor walkable tile next to the first window (the window anchor fallback).
fn window_tile(w: &World) -> Option<(i32, i32)> {
    for y in 0..w.grid.height {
        for x in 0..w.grid.width {
            if w.grid
                .tile(x, y)
                .is_some_and(|t| t.kind == TileKind::Window)
            {
                for (dx, dy) in [(0, 1), (0, -1), (1, 0), (-1, 0)] {
                    let n = (x + dx, y + dy);
                    if w.grid.walkable(n.0, n.1) && w.grid.is_indoor_floor(n.0, n.1) {
                        return Some(n);
                    }
                }
            }
        }
    }
    None
}

fn street_tile(w: &World) -> (i32, i32) {
    let e = w.grid.entrance();
    let s = (e.0, e.1 + 1);
    if w.grid.walkable(s.0, s.1) { s } else { e }
}

/// Where the window shots look: the window furniture or the first window tile.
pub(crate) fn anchor_window(w: &World, content: &Content) -> Option<(i32, i32)> {
    let tag = &content.handson.journal.fallback.focus_tag;
    tagged(w, content, tag)
        .and_then(|u| w.furniture_by_uid(u).map(|p| (p.x, p.y)))
        .or_else(|| window_tile(w))
}

/// A present regular's liked record that is unlocked and not playing.
pub(crate) fn wanted_record(w: &World, content: &Content, regular: &str) -> Option<String> {
    let playing = super::music::current_record(w);
    content
        .handson
        .records
        .records
        .iter()
        .find(|r| {
            r.liked_by.iter().any(|x| x == regular)
                && w.hands.music.unlocked.contains(&r.id)
                && playing != Some(r.id.as_str())
        })
        .map(|r| r.id.clone())
}

fn targeted(w: &World, party: u32) -> bool {
    w.sys
        .moments
        .active
        .iter()
        .any(|a| a.target.party == Some(party))
}

fn party_matches(w: &World, content: &Content, pi: usize, f: &GuestFilter) -> bool {
    let p = &w.parties[pi];
    if p.state == PartyState::Leaving {
        return false;
    }
    let m = w.minute();
    if let Some(r) = &p.regular
        && f.not_regulars.contains(r)
    {
        return false;
    }
    let any = f.archetypes.is_empty() && f.regulars.is_empty();
    let who = any
        || f.archetypes.contains(&p.archetype)
        || p.regular.as_ref().is_some_and(|r| f.regulars.contains(r));
    if !who {
        return false;
    }
    if f.at_door && !(p.state == PartyState::Arriving && m.saturating_sub(p.arrived) <= 4) {
        return false;
    }
    if f.fed && p.state != PartyState::Eating {
        return false;
    }
    if let Some(n) = f.seated_minutes {
        let since = if f.fed {
            p.served_at.unwrap_or(m)
        } else {
            p.arrived
        };
        let seated = p
            .leader()
            .is_some_and(|l| w.guests.iter().any(|g| g.id == l && g.seat.is_some()));
        if !seated || m.saturating_sub(since) < n {
            return false;
        }
    }
    if let Some(n) = f.party_min
        && (p.members.len() as u32) < n
    {
        return false;
    }
    if let Some(acc) = &f.accessory {
        // the sim doesn't draw looks: a "child" rides with a party of 2+ (deterministic)
        let carries = p.members.len() >= 2
            && super::rng::hash_str(&format!("{acc}:{}", p.id)).is_multiple_of(3);
        if !carries {
            return false;
        }
    }
    let _ = content;
    true
}

fn leader_tile(w: &World, party: u32) -> Option<(i32, i32)> {
    let p = w.parties.iter().find(|p| p.id == party)?;
    let l = p.leader()?;
    w.guests.iter().find(|g| g.id == l).map(|g| g.tile())
}

/// Where a moment anchors right now (and whom it is about); `None` = not possible now.
fn anchor(
    w: &World,
    content: &Content,
    def: &MomentDef,
    hint: &XTarget,
) -> Option<(MomentPos, XTarget)> {
    let mut t = hint.clone();
    t.moment = Some(def.id.clone());
    let pos = |tile: (i32, i32), furniture: Option<u32>, agent: Option<AgentRef>| MomentPos {
        tile,
        furniture,
        agent,
    };
    let at_furniture =
        |uid: u32| -> Option<(i32, i32)> { w.furniture_by_uid(uid).map(|p| (p.x, p.y)) };
    let r = match &def.anchor {
        MomentAnchor::Tile(x, y) => pos((*x, *y), None, None),
        MomentAnchor::Door => pos(w.grid.entrance(), None, None),
        MomentAnchor::Street => pos(street_tile(w), None, None),
        MomentAnchor::Counter => pos(counter_tile(w, content), None, None),
        MomentAnchor::Tag(tag) => match tagged(w, content, tag) {
            Some(uid) => {
                t.furniture = Some(uid);
                pos(at_furniture(uid)?, Some(uid), None)
            }
            None if tag.starts_with("window") => pos(window_tile(w)?, None, None),
            None => pos(counter_tile(w, content), None, None),
        },
        MomentAnchor::Furniture(id) => {
            match w
                .furniture
                .iter()
                .filter(|p| &p.def == id)
                .map(|p| p.uid)
                .min()
            {
                Some(uid) => {
                    t.furniture = Some(uid);
                    pos(at_furniture(uid)?, Some(uid), None)
                }
                None => pos(counter_tile(w, content), None, None),
            }
        }
        MomentAnchor::Leak | MomentAnchor::Bucket => {
            let b = w.buckets.last()?.0;
            pos(b, None, None)
        }
        MomentAnchor::Agent(sel) => match sel {
            AgentSelector::RegularWantsRecord => {
                let (pi, rec) = w.parties.iter().enumerate().find_map(|(i, p)| {
                    let r = p.regular.as_ref()?;
                    if p.state == PartyState::Leaving || p.state == PartyState::Arriving {
                        return None;
                    }
                    if targeted(w, p.id) {
                        return None;
                    }
                    wanted_record(w, content, r).map(|rec| (i, rec))
                })?;
                let p = &w.parties[pi];
                t.party = Some(p.id);
                t.regular = p.regular.clone();
                t.record = Some(rec);
                let l = p.leader()?;
                pos(leader_tile(w, p.id)?, None, Some(AgentRef::Guest(l)))
            }
            AgentSelector::StaffLowEnergy(e) => {
                let s = w.staff.iter().find(|s| {
                    !s.owner
                        && s.on_duty()
                        && s.energy < *e
                        && !matches!(s.task, Some(Task::Break { .. }))
                })?;
                t.staff = Some(s.id);
                pos(s.tile(), None, Some(AgentRef::Staff(s.id)))
            }
            AgentSelector::Guest(f) => {
                let pi = match hint
                    .party
                    .and_then(|p| w.parties.iter().position(|x| x.id == p))
                {
                    Some(pi) => pi,
                    None => (0..w.parties.len()).find(|&i| {
                        !targeted(w, w.parties[i].id) && party_matches(w, content, i, f)
                    })?,
                };
                let p = &w.parties[pi];
                t.party = Some(p.id);
                t.regular = p.regular.clone();
                let l = p.leader()?;
                pos(leader_tile(w, p.id)?, None, Some(AgentRef::Guest(l)))
            }
        },
    };
    if t.tile.is_none() {
        t.tile = Some(r.tile);
    }
    Some((r, t))
}

// ---------------------------------------------------------------------------------------------
// The director
// ---------------------------------------------------------------------------------------------

fn eligible(w: &World, content: &Content, def: &MomentDef) -> bool {
    let ms = &w.sys.moments;
    if def.follow_up || ms.active.iter().any(|a| a.id == def.id) {
        return false;
    }
    let tonight = ms.tonight_ids.iter().filter(|x| **x == def.id).count() as u32;
    if tonight >= def.per_night_max.max(1) {
        return false;
    }
    if let Some((count, last)) = ms.history.get(&def.id) {
        if def.max_per_game.is_some_and(|m| *count >= m) {
            return false;
        }
        if def.cooldown_nights > 0 && w.night.saturating_sub(*last) <= def.cooldown_nights {
            return false;
        }
    }
    // event-replacing moments only come from their event
    if def.from_event.is_some() {
        return false;
    }
    xlogic::holds(w, content, &def.conditions)
}

fn weight(w: &World, content: &Content, def: &MomentDef, quiet: bool) -> f32 {
    let mut v = def.weight.max(0.0);
    if let Some(p) =
        super::pacing::season_phase(w, content).and_then(|id| content.handson.seasons.phase(id))
    {
        for (id, mul) in &p.moment_weight {
            if *id == def.id {
                v *= mul;
            }
        }
        if p.moments.contains(&def.id) {
            v *= PHASE_PRIORITY;
        }
    }
    if let Some(m) = super::music::mood_effect(w, content) {
        for (id, mul) in &m.moment_weight {
            if *id == def.id {
                v *= mul;
            }
        }
    }
    if quiet {
        v *= if def.quiet {
            QUIET_PREF.0
        } else {
            QUIET_PREF.1
        };
    }
    v
}

fn room_for_one(w: &World) -> bool {
    let open = w.sys.moments.active.len();
    if w.mode == Mode::Story && w.night <= ONE_AT_A_TIME_UNTIL {
        open == 0
    } else {
        open < MAX_OPEN
    }
}

/// Picks one eligible moment by weight (the season's own first) and spawns it.
fn pick_and_spawn(w: &mut World, content: &Content) -> Option<String> {
    let quiet = super::pacing::quiet_hour(w, content);
    let mut cands: Vec<(&MomentDef, f32)> = content
        .handson
        .moments
        .iter()
        .filter(|d| eligible(w, content, d))
        .filter(|d| anchor(w, content, d, &XTarget::default()).is_some())
        .map(|d| (d, weight(w, content, d, quiet)))
        .filter(|(_, wt)| *wt > 0.0)
        .collect();
    if cands.is_empty() {
        return None;
    }
    let weights: Vec<f32> = cands.iter().map(|c| c.1).collect();
    let i = w.rng.weighted(&weights)?;
    let id = cands.swap_remove(i).0.id.clone();
    spawn(w, content, &id, XTarget::default(), false).then_some(id)
}

/// Spawns a moment now (`forced` = effects / events / bot buttons: no budget check, but it
/// still needs its anchor). Returns false if it couldn't appear.
pub(crate) fn spawn(
    w: &mut World,
    content: &Content,
    id: &str,
    hint: XTarget,
    forced: bool,
) -> bool {
    if w.phase != Phase::Night {
        return false;
    }
    let Some(def) = content.handson.moment(id) else {
        return false;
    };
    if w.sys.moments.active.iter().any(|a| a.id == id) {
        return false;
    }
    if !forced && !room_for_one(w) {
        return false;
    }
    let Some((pos, target)) = anchor(w, content, def, &hint) else {
        return false;
    };
    let m = w.minute();
    let ms = &mut w.sys.moments;
    ms.active.push(ActiveMoment {
        id: id.to_string(),
        anchor: pos.clone(),
        spawned: m,
        expires: m + def.expires_minutes.max(1),
        target,
        resolving: None,
    });
    ms.tonight_ids.push(id.to_string());
    if !def.follow_up {
        ms.tonight += 1;
        ms.last_minute = Some(m);
    }
    let night = w.night;
    let h = ms.history.entry(id.to_string()).or_insert((0, 0));
    h.0 += 1;
    h.1 = night;
    w.tonight.moments += 1;
    w.emit(SimEvent::MomentAvailable {
        id: id.to_string(),
        anchor: pos.clone(),
    });
    // the cat at the door while Karolina is far away: the bot pings
    if def.kind == crate::content::handson::MomentKind::Cat
        && let Some(k) = super::player::karolina_idx(w)
        && grid::manhattan(w.staff[k].tile(), pos.tile) > 8
    {
        let ctx = EffectCtx::default();
        super::app::bot(w, content, "cat_waiting", &ctx);
    }
    true
}

/// The moment that replaces this event's choice toast, if any.
pub(crate) fn for_event<'a>(content: &'a Content, event: &str) -> Option<&'a MomentDef> {
    content
        .handson
        .moments
        .iter()
        .find(|m| m.from_event.as_deref() == Some(event))
}

/// An event fired: spawn its moment instead of the choice toast (nights 1–3: it waits for
/// room). True = the event's choices are skipped.
pub(crate) fn on_event(w: &mut World, content: &Content, event: &str) -> bool {
    let Some(def) = for_event(content, event) else {
        return false;
    };
    if w.phase != Phase::Night {
        return false;
    }
    let id = def.id.clone();
    if room_for_one(w) && spawn(w, content, &id, XTarget::default(), true) {
        return true;
    }
    if w.sys.moments.active.iter().any(|a| a.id == id) {
        return true;
    }
    let until = w.minute() + WAIT_MINUTES;
    w.sys.moments.waiting.push((id, XTarget::default(), until));
    true
}

/// Each game minute: expiry, waiting event moments, the budgeted director.
pub(crate) fn on_minute(w: &mut World, content: &Content) {
    let m = w.minute();
    // expiry (not while Karolina is on her way)
    let due: Vec<String> = w
        .sys
        .moments
        .active
        .iter()
        .filter(|a| a.expires <= m && a.resolving.is_none())
        .map(|a| a.id.clone())
        .collect();
    for id in due {
        expire(w, content, &id);
    }
    // an agent anchor that left (the guest went home, the helper went on a break)
    let gone: Vec<String> = w
        .sys
        .moments
        .active
        .iter()
        .filter(|a| a.resolving.is_none())
        .filter(|a| match a.anchor.agent {
            Some(AgentRef::Guest(g)) => !w.guests.iter().any(|x| x.id == g && !x.gone),
            Some(AgentRef::Staff(s)) => !w.staff.iter().any(|x| x.id == s && x.on_duty()),
            None => false,
        })
        .map(|a| a.id.clone())
        .collect();
    for id in gone {
        w.sys.moments.active.retain(|a| a.id != id);
        w.emit(SimEvent::MomentExpired { id });
    }
    // event moments waiting for room
    let waiting = std::mem::take(&mut w.sys.moments.waiting);
    for (id, hint, until) in waiting {
        if room_for_one(w) && spawn(w, content, &id, hint.clone(), true) {
            continue;
        }
        if m >= until {
            // never failable: the old default choice applies
            if let Some(def) = content.handson.moment(&id) {
                let on_expire = def.on_expire.clone();
                xlogic::run(w, content, &on_expire, &hint);
            }
        } else {
            w.sys.moments.waiting.push((id, hint, until));
        }
    }
    // the director
    if !(10..LAST_MOMENT_MINUTE).contains(&m) {
        return;
    }
    let t = &content.handson.tuning;
    let ms = &w.sys.moments;
    if ms.tonight >= ms.budget {
        return;
    }
    if ms
        .last_minute
        .is_some_and(|l| m.saturating_sub(l) < t.moment_gap_minutes)
    {
        return;
    }
    if !room_for_one(w) {
        return;
    }
    // spread the budget over the night: hazard = moments left / slots left (late in the
    // night it catches up, so a quiet night still gets its share)
    let left = (ms.budget - ms.tonight) as f32;
    let slots = ((LAST_MOMENT_MINUTE - m) as f32 / 20.0).max(1.0);
    let p = (left / slots * 1.2).clamp(0.03, 1.0);
    if w.rng.chance(p) {
        pick_and_spawn(w, content);
    }
}

/// The lull director's nudge: a moment now (within the night's max), ignoring the gap.
pub(crate) fn lull_spawn(w: &mut World, content: &Content) -> Option<String> {
    let t = &content.handson.tuning;
    let m = w.minute();
    let cap = if w.sys.moments.cap > 0 {
        w.sys.moments.cap
    } else {
        t.moments_max_per_night
    };
    if m >= LAST_MOMENT_MINUTE || w.sys.moments.tonight >= cap {
        return None;
    }
    if !room_for_one(w) {
        return None;
    }
    let id = pick_and_spawn(w, content)?;
    // it counts: keep the budget honest
    let ms = &mut w.sys.moments;
    ms.budget = ms.budget.max(ms.tonight);
    Some(id)
}

fn expire(w: &mut World, content: &Content, id: &str) {
    let Some(i) = w.sys.moments.active.iter().position(|a| a.id == id) else {
        return;
    };
    let a = w.sys.moments.active.remove(i);
    if let Some(def) = content.handson.moment(id)
        && !def.on_expire.is_empty()
    {
        xlogic::run(w, content, &def.on_expire, &a.target);
        w.emit(SimEvent::Toast {
            key: def.expired_key(),
        });
    }
    w.emit(SimEvent::MomentExpired { id: id.to_string() });
}

// ---------------------------------------------------------------------------------------------
// Resolving
// ---------------------------------------------------------------------------------------------

/// The options shown on a moment's card right now (`show_if`, `within_minutes`).
pub(crate) fn options_now(w: &World, content: &Content, id: &str) -> Vec<String> {
    let Some(a) = w.sys.moments.active.iter().find(|a| a.id == id) else {
        return Vec::new();
    };
    let Some(def) = content.handson.moment(id) else {
        return Vec::new();
    };
    let m = w.minute();
    def.options
        .iter()
        .filter(|o| xlogic::holds(w, content, &o.show_if))
        .filter(|o| {
            o.within_minutes
                .is_none_or(|n| m.saturating_sub(a.spawned) <= n)
        })
        .map(|o| o.id.clone())
        .collect()
}

/// `Command::ResolveMoment`: Karolina walks to the anchor; the option applies on arrival.
pub(crate) fn resolve(
    w: &mut World,
    content: &Content,
    id: &str,
    option: &str,
) -> Result<(), CommandError> {
    let def = content
        .handson
        .moment(id)
        .ok_or_else(|| CommandError::UnknownId(id.to_string()))?;
    if def.option(option).is_none() {
        return Err(CommandError::UnknownId(option.to_string()));
    }
    let Some(a) = w.sys.moments.active.iter().find(|a| a.id == id) else {
        return Err(CommandError::Invalid("that moment has passed".into()));
    };
    if a.resolving.is_some() {
        return Err(CommandError::Invalid("Karolina is on her way".into()));
    }
    if !options_now(w, content, id).iter().any(|o| o == option) {
        return Err(CommandError::Invalid("not now".into()));
    }
    let from = super::player::karolina_idx(w)
        .map(|k| w.staff[k].tile())
        .unwrap_or_else(|| w.grid.entrance());
    let tile =
        stand_tile(w, id, from).ok_or_else(|| CommandError::Invalid("can't get there".into()))?;
    super::player::queue_walk(
        w,
        content,
        tile,
        super::systems::WalkKind::Moment {
            id: id.to_string(),
            option: option.to_string(),
        },
    )?;
    if let Some(a) = w.sys.moments.active.iter_mut().find(|a| a.id == id) {
        a.resolving = Some(option.to_string());
    }
    Ok(())
}

/// The moment card's option went back (her act was cancelled / unqueued).
pub(crate) fn release(w: &mut World, id: &str) {
    if let Some(a) = w.sys.moments.active.iter_mut().find(|a| a.id == id) {
        a.resolving = None;
    }
}

/// Where Karolina stands to resolve it.
pub(crate) fn stand_tile(w: &World, id: &str, from: (i32, i32)) -> Option<(i32, i32)> {
    let a = w.sys.moments.active.iter().find(|a| a.id == id)?;
    let tile = match a.anchor.agent {
        Some(AgentRef::Guest(g)) => w.guests.iter().find(|x| x.id == g)?.tile(),
        Some(AgentRef::Staff(s)) => w.staff_by_id(s)?.tile(),
        None => a.anchor.tile,
    };
    if w.grid.walkable(tile.0, tile.1)
        && w.grid.object_at(tile.0, tile.1).is_none()
        && !w.buckets.iter().any(|b| b.0 == tile)
    {
        return Some(tile);
    }
    staff::stand_next_to(w, tile, from).or_else(|| {
        // a street or wall anchor: the nearest inside spot by the door
        let e = w.grid.entrance();
        Some(e)
    })
}

/// Work ticks at the anchor for an option (the hands-on part beyond the click).
pub(crate) fn option_work(content: &Content, id: &str, option: &str, w: &World) -> f32 {
    let base = 6.0;
    let Some(o) = content.handson.moment(id).and_then(|d| d.option(option)) else {
        return base;
    };
    match &o.interaction {
        None => base,
        Some(Interaction::Swipe) | Some(Interaction::Hold(_)) => 8.0,
        Some(Interaction::TrainPicker { .. }) => 6.0,
        Some(Interaction::LightEachTable) => {
            let tables = w
                .furniture
                .iter()
                .filter(|p| {
                    content
                        .furniture_def(&p.def)
                        .is_some_and(|d| d.has_tag("table"))
                })
                .count();
            base + tables as f32 * 3.0
        }
        Some(Interaction::HangSpots(n)) => *n as f32 * 10.0,
        Some(Interaction::QuickBrews { recipe, count }) => {
            let prep = content
                .handson
                .recipe(recipe)
                .and_then(|r| content.menu_item(&r.base_item))
                .map(|m| m.prep_minutes)
                .unwrap_or(1.0);
            prep * TICKS_PER_MINUTE as f32 * (*count as f32) * 0.6
        }
        Some(Interaction::ByHandFavourite) => 12.0,
    }
}

/// Karolina arrived: the option applies.
pub(crate) fn arrive(w: &mut World, content: &Content, id: &str, option: &str) {
    let Some(i) = w.sys.moments.active.iter().position(|a| a.id == id) else {
        return;
    };
    let a = w.sys.moments.active.remove(i);
    let Some(def) = content.handson.moment(id) else {
        return;
    };
    let Some(o) = def.option(option) else {
        return;
    };
    let effects = o.effects.clone();
    xlogic::run(w, content, &effects, &a.target);
    w.sys.moments.resolved_ever.insert(id.to_string());
    w.tonight.moments_resolved += 1;
    if def.kind == crate::content::handson::MomentKind::Cat {
        w.sys.extras.cat_here = true;
    }
    // a QuickBrews option makes the drinks: they count as Karolina's items
    if let Some(Interaction::QuickBrews { count, .. }) = &o.interaction {
        w.hands.tally.karolina_items += count;
    }
    w.emit(SimEvent::MomentResolved {
        id: id.to_string(),
        option: option.to_string(),
    });
}

// ---------------------------------------------------------------------------------------------
// The station cat (§6.2)
// ---------------------------------------------------------------------------------------------

/// Where the cat is and what it does (render): its tile, a perch id, its level and name.
#[derive(Debug, Clone, PartialEq)]
pub struct CatView {
    pub tile: (i32, i32),
    /// `doormat`, `radiator`, `window_seat`, `follows`.
    pub perch: &'static str,
    pub level: u32,
    /// `cat.name.<id>` once named.
    pub name: Option<String>,
}

pub(crate) fn cat_view(w: &World, content: &Content) -> Option<CatView> {
    let ex = &w.sys.extras;
    if !(ex.cat_here || ex.cat_level >= 2) {
        return None;
    }
    let level = ex.cat_level;
    let name = w
        .story
        .vars
        .get("cat_name")
        .filter(|v| v.as_str() != "none")
        .map(|v| format!("cat.name.{v}"));
    let (tile, perch) = if level >= 8 && w.phase == Phase::Night {
        let k = super::player::karolina_idx(w)?;
        (w.staff[k].tile(), "follows")
    } else if level >= 6 {
        let t = tagged(w, content, "window_spot")
            .and_then(|u| w.furniture_by_uid(u).map(|p| (p.x, p.y)))
            .or_else(|| window_tile(w))?;
        (t, "window_seat")
    } else if level >= 4 {
        (
            window_tile(w).unwrap_or_else(|| w.grid.entrance()),
            "radiator",
        )
    } else {
        (w.grid.entrance(), "doormat")
    };
    Some(CatView {
        tile,
        perch,
        level,
        name,
    })
}

/// The affection counter moved: new levels → `CatLevel`, toast, perks, a polaroid.
pub(crate) fn check_cat(w: &mut World, content: &Content) {
    let cat = &content.handson.extras.cat;
    let aff = w.story.counters.get(&cat.counter).copied().unwrap_or(0);
    if aff > cat.max {
        w.story.counters.insert(cat.counter.clone(), cat.max);
    }
    let level = cat.level_at(aff.min(cat.max));
    let before = w.sys.extras.cat_level;
    if level <= before {
        return;
    }
    w.sys.extras.cat_level = level;
    for lv in cat
        .levels
        .iter()
        .filter(|l| l.level > before && l.level <= level)
    {
        w.emit(SimEvent::CatLevel { level: lv.level });
        w.emit(SimEvent::Toast {
            key: format!("cat.level.{}", lv.level),
        });
        for p in &lv.perks {
            match p {
                CatPerk::Latte(id) => super::atmos::unlock_latte(w, id),
                CatPerk::Inside => w.sys.extras.cat_here = true,
                CatPerk::Naming { moment } => {
                    let id = moment.clone();
                    if !spawn(w, content, &id, XTarget::default(), true) && w.phase == Phase::Day {
                        // next night it will be there
                    }
                }
                _ => {}
            }
        }
    }
    if level >= content.handson.journal.cat_level_min && level > before {
        let tile = cat_view(w, content).map(|c| c.tile);
        super::journal::capture(w, content, HighlightKind::CatLevel, None, Vec::new(), tile);
    }
}

/// The naming moment waits for a night (conditions are checked by the director too, but
/// it isn't on any random list: spawn it early each night until it is resolved).
pub(crate) fn naming_nudge(w: &mut World, content: &Content) {
    if w.minute() != 30 {
        return;
    }
    for lv in &content.handson.extras.cat.levels {
        if lv.level > w.sys.extras.cat_level {
            continue;
        }
        for p in &lv.perks {
            if let CatPerk::Naming { moment } = p {
                let id = moment.clone();
                if let Some(def) = content.handson.moment(&id)
                    && xlogic::holds(w, content, &def.conditions)
                    && !w.sys.moments.resolved_ever.contains(&id)
                {
                    spawn(w, content, &id, XTarget::default(), true);
                }
            }
        }
    }
}

/// Grants the cat-related latte and window-seat satisfaction at a serve (+2 near the cat).
pub(crate) fn window_seat_bonus(w: &World, content: &Content, pi: usize) -> f32 {
    let mut add = 0.0;
    let Some(cv) = cat_view(w, content) else {
        return 0.0;
    };
    for lv in &content.handson.extras.cat.levels {
        if lv.level > w.sys.extras.cat_level {
            continue;
        }
        for p in &lv.perks {
            if let CatPerk::WindowSeat { satisfaction } = p {
                let near = w.parties[pi].members.iter().any(|g| {
                    w.guests
                        .iter()
                        .any(|x| x.id == *g && grid::chebyshev(x.tile(), cv.tile) <= 3)
                });
                if near {
                    add += satisfaction;
                }
            }
        }
    }
    add
}

// ---------------------------------------------------------------------------------------------
// Thank-you napkins (§6.1)
// ---------------------------------------------------------------------------------------------

fn napkin_for(w: &World, content: &Content, regular: Option<&str>) -> Option<String> {
    let ex = &content.handson.extras;
    let have = &w.sys.extras.napkins;
    let pending = |id: &str| w.parties.iter().any(|p| p.napkin.as_deref() == Some(id));
    if let Some(r) = regular
        && let Some(n) = ex
            .napkins
            .iter()
            .find(|n| n.who.iter().any(|x| x == r) && !have.contains(&n.id) && !pending(&n.id))
    {
        return Some(n.id.clone());
    }
    ex.napkins
        .iter()
        .find(|n| n.who.is_empty() && !have.contains(&n.id) && !pending(&n.id))
        .map(|n| n.id.clone())
}

/// A Karolina-made serve was lovely: maybe a napkin with the tip.
pub(crate) fn napkin_on_serve(
    w: &mut World,
    content: &Content,
    pi: usize,
    match_score: f32,
    quality: f32,
) {
    let tc = &content.handson.tuning.thanks_chance;
    if match_score < tc.min_match || quality < tc.min_quality || w.parties[pi].napkin.is_some() {
        return;
    }
    let regular = w.parties[pi].regular.clone();
    let chance = if regular.is_some() {
        tc.regular
    } else {
        tc.guest
    };
    if !w.rng.chance(chance) {
        return;
    }
    if let Some(n) = napkin_for(w, content, regular.as_deref()) {
        w.parties[pi].napkin = Some(n);
    }
}

/// `ThankYouNote` effect: a napkin into the book at once.
pub(crate) fn give_napkin(
    w: &mut World,
    content: &Content,
    id: Option<&str>,
    regular: Option<&str>,
) {
    let id = match id {
        Some(i) if content.handson.extras.napkins.iter().any(|n| n.id == i) => i.to_string(),
        Some(_) => return,
        None => match napkin_for(w, content, regular) {
            Some(n) => n,
            None => return,
        },
    };
    collect_napkin(w, content, &id);
}

/// A napkin was picked up with its coin.
pub(crate) fn collect_napkin(w: &mut World, content: &Content, id: &str) {
    if w.sys.extras.napkins.iter().any(|n| n == id) {
        return;
    }
    w.sys.extras.napkins.push(id.to_string());
    w.tonight.napkins += 1;
    w.emit(SimEvent::ThankYouNote { id: id.to_string() });
    let n = w.sys.extras.napkins.len() as u32;
    for r in &content.handson.extras.napkin_rewards {
        if n >= r.count && w.sys.extras.napkin_rewards.insert(r.count) {
            let fx = r.effects.clone();
            xlogic::run(w, content, &fx, &XTarget::default());
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Tip-jar goals (§6.5)
// ---------------------------------------------------------------------------------------------

fn furniture_owned(w: &World, id: &str) -> bool {
    w.story.unlocked_furniture.contains(id) || w.furniture.iter().any(|p| p.def == id)
}

/// Lifetime tips crossed a goal: "Count the jar" (one pick of two; if one is owned the other
/// is given at once).
pub(crate) fn check_tip_goals(w: &mut World, content: &Content) {
    if w.sys.extras.tip_goal_pending.is_some() {
        return;
    }
    let life = w.hands.tips_lifetime;
    for (i, g) in content.handson.extras.tip_goals.iter().enumerate() {
        let i = i as u32;
        if w.sys.extras.tip_goals.contains(&i) || life < g.lifetime as i64 {
            continue;
        }
        let open: Vec<String> = g
            .choices
            .iter()
            .filter(|c| !furniture_owned(w, c))
            .cloned()
            .collect();
        match open.len() {
            0 => {
                w.sys.extras.tip_goals.insert(i);
            }
            1 => {
                w.sys.extras.tip_goals.insert(i);
                give_furniture(w, content, &open[0]);
                w.emit(SimEvent::TipGoalReached {
                    goal: i,
                    choices: open,
                });
            }
            _ => {
                w.sys.extras.tip_goal_pending = Some(i);
                w.emit(SimEvent::TipGoalReached {
                    goal: i,
                    choices: open,
                });
                return;
            }
        }
    }
}

fn give_furniture(w: &mut World, content: &Content, id: &str) {
    let mut ctx = EffectCtx::default();
    super::logic::run(
        w,
        content,
        &EffectSrc::None,
        &[Effect::UnlockFurniture(id.to_string())],
        &mut ctx,
    );
}

/// `Command::PickTipGoal`.
pub(crate) fn pick_tip_goal(
    w: &mut World,
    content: &Content,
    choice: &str,
) -> Result<(), CommandError> {
    let Some(i) = w.sys.extras.tip_goal_pending else {
        return Err(CommandError::Invalid("no goal to count".into()));
    };
    let g = content
        .handson
        .extras
        .tip_goals
        .get(i as usize)
        .ok_or_else(|| CommandError::UnknownId(i.to_string()))?;
    if !g.choices.iter().any(|c| c == choice) {
        return Err(CommandError::UnknownId(choice.to_string()));
    }
    w.sys.extras.tip_goal_pending = None;
    w.sys.extras.tip_goals.insert(i);
    give_furniture(w, content, choice);
    check_tip_goals(w, content);
    Ok(())
}
