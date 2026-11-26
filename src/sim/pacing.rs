//! Waiting & pacing (design/handson.md §1.4–§1.6): the per-party wait clock rate, rushes after
//! trains and crowds, Karolina's rhythm, the quiet hour, her pending things (the "todo"), and
//! the lull director that keeps 1–3 meaningful things to do at ×1 without flooding.

use super::duties::duty_on_floor;
use super::guests;
use super::hands::*;
use super::types::*;
use super::world::*;
use crate::content::handson::Duty;
use crate::content::{Content, Season};

/// Karolina's world id (the owner).
pub(crate) fn owner_id(w: &World) -> Option<u32> {
    w.staff.iter().find(|s| s.owner).map(|s| s.id)
}

/// 03:00–04:00 (`handson.ron` → `quiet_hour`), nights only.
pub(crate) fn quiet_hour(w: &World, content: &Content) -> bool {
    let q = &content.handson.tuning.quiet_hour;
    w.phase == Phase::Night && w.clock().is_between(q.from, q.to)
}

/// The season phase id tonight (`seasons.ron`).
pub(crate) fn season_phase<'a>(w: &World, content: &'a Content) -> Option<&'a str> {
    let s = &content.handson.seasons;
    match w.mode {
        Mode::Story => s.story_phase(w.night).map(|p| p.id.as_str()),
        Mode::Sandbox => {
            let season = match w.town.season {
                0 => Season::Spring,
                1 => Season::Summer,
                2 => Season::Autumn,
                _ => Season::Winter,
            };
            let n = (w.night.max(1) - 1) % 30 + 1;
            s.sandbox_phase(season, n).map(|p| p.id.as_str())
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Wait clock
// ---------------------------------------------------------------------------------------------

/// Is Karolina on this party (walking over, brewing or carrying for them)?
pub(crate) fn claimed_by_karolina(w: &World, pi: usize) -> bool {
    let Some(me) = owner_id(w) else {
        return false;
    };
    let p = &w.parties[pi];
    p.order_claim == Some(me)
        || p.items.iter().any(|id| {
            w.items.iter().any(|i| {
                i.id == *id
                    && i.by == Some(me)
                    && !matches!(i.state, ItemState::Served | ItemState::Cancelled)
            })
        })
}

/// Wait clock (and patience) rate for a waiting party (§1.4): 1.0 unclaimed, ×0.5 while
/// Karolina is on it, ×0.67 more during a Cozy rush; hall-mood patience perks divide it.
pub(crate) fn wait_rate(w: &World, content: &Content, pi: usize) -> f32 {
    let t = &content.handson.tuning;
    let mut rate = 1.0;
    if claimed_by_karolina(w, pi) {
        rate *= t.claimed_wait_rate;
    }
    if w.difficulty == Difficulty::Cozy && w.hands.rush.until.is_some() {
        rate *= t.cozy_rush_wait_rate;
    }
    if let Some(m) = super::music::mood_effect(w, content) {
        let arch = &w.parties[pi].archetype;
        for p in &m.patience {
            if p.archetypes.contains(arch) && p.value > 0.0 {
                rate /= p.value;
            }
        }
    }
    // moment effects (the station board on the café screen)
    let now = w.minute();
    for (mul, until) in &w.sys.fx.patience {
        if *mul > 0.0 && until.is_none_or(|u| u > now) {
            rate /= mul;
        }
    }
    rate
}

// ---------------------------------------------------------------------------------------------
// Rush & rhythm
// ---------------------------------------------------------------------------------------------

/// A party walked in: ≥ `rush.parties` arrivals within `rush.window_minutes` start a rush
/// (story: not before `rush.from_night`).
pub(crate) fn note_arrival(w: &mut World, content: &Content) {
    let r = &content.handson.tuning.rush;
    let m = w.minute();
    w.hands.rush.arrivals.push(m);
    w.hands
        .rush
        .arrivals
        .retain(|a| m.saturating_sub(*a) < r.window_minutes);
    let allowed = w.mode == Mode::Sandbox || w.night >= r.from_night;
    if allowed && w.hands.rush.arrivals.len() as u32 >= r.parties {
        let until = m + r.minutes;
        if w.hands.rush.until.is_none() {
            w.hands.tally.rushes += 1;
            w.sys.tool.rush_at.get_or_insert(m);
            w.emit(SimEvent::RushStarted);
        }
        w.hands.rush.until = Some(w.hands.rush.until.map_or(until, |u| u.max(until)));
    }
}

/// Karolina served something: the rhythm streak grows within the window (else it starts over,
/// silently). Returns the tip multiplier.
pub(crate) fn rhythm_on_serve(w: &mut World, content: &Content) -> f32 {
    let r = &content.handson.tuning.rhythm;
    let m = w.minute();
    let max_steps = (r.max / r.step.max(0.001)).round() as u8;
    let p = &mut w.hands.player;
    let before = p.rhythm;
    p.rhythm = match p.last_serve_minute {
        Some(last) if m.saturating_sub(last) <= r.window_minutes => (p.rhythm + 1).min(max_steps),
        _ => 0,
    };
    p.last_serve_minute = Some(m);
    let step = p.rhythm;
    w.hands.tally.best_rhythm = w.hands.tally.best_rhythm.max(step);
    if step != before {
        w.emit(SimEvent::RhythmChanged { step });
    }
    1.0 + step as f32 * r.step
}

// ---------------------------------------------------------------------------------------------
// Karolina's pending things
// ---------------------------------------------------------------------------------------------

/// Things waiting on Karolina right now (no helper on the floor holds that duty): waiting
/// orders, open wishes, unowned rounds (a party's items at one station; a tray to carry),
/// dishes, the floor. The ticket rail + world markers + dirt, as one number (§9: median 1–3).
/// Tip coins are not counted (they never expire), but they keep her from being idle.
pub(crate) fn todo(w: &World, content: &Content) -> u32 {
    let reg = duty_on_floor(w, Duty::Register);
    let serving = duty_on_floor(w, Duty::Serving);
    let bar = duty_on_floor(w, Duty::Bar);
    let kitchen = duty_on_floor(w, Duty::Kitchen);
    let dishes = duty_on_floor(w, Duty::Dishes);
    let floor = duty_on_floor(w, Duty::Floor);
    let mut n = 0u32;
    for p in &w.parties {
        match p.state {
            PartyState::Queue | PartyState::WaitOrder if p.order_claim.is_none() => {
                let helper_can = (!p.order_kind.is_wish() || p.handed_off) && p.regular.is_none();
                if !(helper_can && reg) {
                    n += 1;
                }
            }
            PartyState::WaitFood if p.wish_open && !p.handed_off => n += 1,
            _ => {}
        }
    }
    // one thing per round: a party's unowned items at one station are one quick brew, its
    // ready items one tray
    let mut rounds: Vec<(u32, &str)> = Vec::new();
    for i in &w.items {
        if i.by.is_some() {
            continue;
        }
        match i.state {
            ItemState::Pending => {
                let Some(mi) = content.menu_item(&i.menu) else {
                    continue;
                };
                let helper = match mi.job {
                    crate::content::Job::Kitchen => kitchen,
                    _ => bar,
                };
                if !helper && !rounds.contains(&(i.party, mi.station.as_str())) {
                    rounds.push((i.party, mi.station.as_str()));
                }
            }
            ItemState::Ready if !serving && !rounds.contains(&(i.party, "tray")) => {
                rounds.push((i.party, "tray"));
            }
            _ => {}
        }
    }
    n += rounds.len() as u32;
    if !dishes {
        n += w
            .furniture
            .iter()
            .filter(|p| p.dishes > 0 && p.claim.is_none())
            .count() as u32;
        if w.sink_dishes > 0 && w.sink_claim.is_none() {
            n += 1;
        }
    }
    if !floor && w.dirt > guests::MOP_DIRT {
        n += 1;
    }
    n
}

// ---------------------------------------------------------------------------------------------
// Per minute: tallies, quiet hour, rush end, rhythm fade, the lull director
// ---------------------------------------------------------------------------------------------

pub(crate) fn on_minute(w: &mut World, content: &Content) {
    let m = w.minute();
    let t = &content.handson.tuning;
    // quiet hour edges
    let q = &t.quiet_hour;
    let from = q.from.minutes_since_open();
    let to = q.to.minutes_since_open();
    if from >= 0 && m == from as u32 {
        w.emit(SimEvent::QuietHour { on: true });
        w.emit(SimEvent::Toast {
            key: q.toast.clone(),
        });
    } else if to >= 0 && m == to as u32 {
        w.emit(SimEvent::QuietHour { on: false });
    }
    // rush end
    if let Some(u) = w.hands.rush.until
        && m >= u
    {
        w.hands.rush.until = None;
        w.emit(SimEvent::RushEnded);
    }
    // rhythm fades with a gap (no message, no penalty)
    if w.hands.player.rhythm > 0
        && w.hands
            .player
            .last_serve_minute
            .is_some_and(|l| m.saturating_sub(l) > t.rhythm.window_minutes)
    {
        w.hands.player.rhythm = 0;
        w.emit(SimEvent::RhythmChanged { step: 0 });
    }
    // effect hall mood ends
    if let Some(u) = w.hands.hall_mood.until
        && m >= u
    {
        w.hands.hall_mood = HallMoodState::default();
        let mood = super::music::hall_mood(w, content);
        w.emit(SimEvent::HallMoodChanged { mood });
    }
    // Karolina idle & the todo histogram
    let todo = todo(w, content);
    let busy = !w.hands.player.queue.is_empty() || w.hands.player.current.is_some();
    // tip coins never expire (nobody waits on them), but picking them up is something to do
    let idle = !busy && todo == 0 && w.hands.tips.is_empty();
    let quiet = quiet_hour(w, content);
    let tally = &mut w.hands.tally;
    let k = (todo as usize).min(15);
    if tally.todo_hist.len() < 16 {
        tally.todo_hist.resize(16, 0);
    }
    tally.todo_hist[k] += 1;
    tally.todo_peak = tally.todo_peak.max(todo);
    if quiet {
        tally.quiet_minutes += 1;
        if idle {
            tally.idle_quiet_minutes += 1;
        }
    } else if idle {
        tally.idle_minutes += 1;
    }
    if idle {
        w.hands.player.idle_minutes += 1;
    } else {
        w.hands.player.idle_minutes = 0;
    }
    lull_director(w, content, quiet);
}

/// §1.6: after `lull_minutes` idle (15 in the quiet hour) suggest the first that applies:
/// a regular's Talk → a night moment (within the night's maximum) → a walk-in → rain mud by
/// the door → the ×2 chip.
fn lull_director(w: &mut World, content: &Content, quiet: bool) {
    let t = &content.handson.tuning;
    let m = w.minute();
    let threshold = if quiet {
        t.quiet_hour_lull_minutes
    } else {
        t.lull_minutes
    };
    let idle = w.hands.player.idle_minutes;
    if idle < threshold || m >= guests::LAST_CALL {
        return;
    }
    if w.hands
        .lull
        .last_fire
        .is_some_and(|l| m.saturating_sub(l) < threshold)
    {
        return;
    }
    w.hands.lull.last_fire = Some(m);
    w.hands.player.idle_minutes = 0;
    w.hands.tally.lull_fired += 1;
    w.emit(SimEvent::KarolinaIdle { minutes: idle });
    // 1. a regular has something to say
    let talk = w
        .guests
        .iter()
        .filter(|g| g.leader && !g.gone)
        .map(|g| g.id)
        .find(|&g| super::story::talk_available(w, content, g).is_some());
    let hint = if let Some(guest) = talk {
        LullHint::Talk { guest }
    } else if super::moments::lull_spawn(w, content).is_some() {
        // a small moment appears: its marker is the suggestion
        return;
    } else if walk_in(w, content) {
        LullHint::WalkIn
    } else if w.weather.is_wet() && w.dirt < 0.2 && !duty_on_floor(w, Duty::Floor) {
        w.dirt = (w.dirt + 0.06).min(1.0);
        LullHint::Mud
    } else {
        LullHint::Speed
    };
    w.emit(SimEvent::LullSuggestion { hint });
}

/// A walk-in with a wish arrives in 1–3 minutes (only when there's room and the hall isn't
/// full); from story night `lull_walk_in.2` (sandbox: always) the guest rate also rises.
fn walk_in(w: &mut World, content: &Content) -> bool {
    let m = w.minute();
    let seats = guests::seats(w, content);
    let free = seats.iter().filter(|s| s.free).count();
    if free < 2 || w.guests_inside() * 10 >= seats.len() * 6 || m + 20 >= guests::LAST_CALL {
        return false;
    }
    let Some(hour) = crate::content::ClockTime::from_minutes_since_open(m).open_hour_index() else {
        return false;
    };
    let Some(arch) = guests::pick_archetype(w, content, hour) else {
        return false;
    };
    let at = m + w.rng.range_u32(1, 3);
    w.spawns.push(guests::PendingSpawn {
        minute: at,
        archetype: arch,
        size: 1,
        patience_mul: 1.0,
        spend_mul: 1.0,
        orders: Vec::new(),
        wish: true,
        crowd: false,
        held: 0,
    });
    let (mul, minutes, from) = content.handson.tuning.lull_walk_in;
    if w.mode == Mode::Sandbox || w.night >= from {
        w.mods.push(Modifier {
            kind: ModKind::GuestRate,
            mul,
            until: Some(m + minutes),
            event: None,
        });
    }
    w.hands.lull.walk_ins += 1;
    true
}
