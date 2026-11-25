//! Staff: generation & hiring, shifts, needs, traits, relationships, job selection and the work loop.

use super::grid::{self, Room};
use super::guests;
use super::rng::{Rng, hash_str};
use super::types::*;
use super::world::*;
use crate::content::{
    ChecklistKind, Content, Gender, Job, Need, SandboxDef, Situation, TraitEffect,
};

/// Staff walking speed, tiles per tick.
pub const STAFF_SPEED: f32 = 0.5;
/// Items a server carries on one tray.
pub const TRAY: usize = 4;

// ---------------------------------------------------------------------------------------------
// Generation
// ---------------------------------------------------------------------------------------------

pub(crate) fn town_name(rng: &mut Rng, sb: &SandboxDef) -> String {
    let t = &sb.town_names;
    for _ in 0..20 {
        let root = rng.pick(&t.roots).cloned().unwrap_or_else(|| "Ml".into());
        let suffix = rng
            .pick(&t.suffixes)
            .cloned()
            .unwrap_or_else(|| "ov".into());
        let core = format!("{root}{suffix}");
        if t.stoplist.iter().any(|s| s.eq_ignore_ascii_case(&core)) {
            continue;
        }
        let mut name = String::new();
        if rng.chance(t.prefix_chance)
            && let Some(p) = rng.pick(&t.prefixes)
        {
            name.push_str(p);
            name.push(' ');
        }
        name.push_str(&core);
        if rng.chance(t.river_chance)
            && let Some(r) = rng.pick(&t.rivers)
        {
            name.push_str(" nad ");
            name.push_str(r);
        }
        return name;
    }
    "Mlžná Lhota".into()
}

fn blank_staff(id: u32) -> Staff {
    Staff {
        id,
        name: String::new(),
        archetype: String::new(),
        owner: false,
        story_person: false,
        age: 25,
        female: false,
        traits: Vec::new(),
        skills: [3.0; 5],
        wage: 1200,
        priorities: [3; 5],
        energy: 100.0,
        mood: 65.0,
        hunger: 90.0,
        look_seed: 0,
        personal_number: 0,
        hired_night: 0,
        status: StaffStatus::OffDuty,
        pos: (0.0, 0.0),
        facing: Facing::Down,
        pose: Pose::Idle,
        path: Vec::new(),
        task: None,
        work_left: 0.0,
        shift: Shift::Full,
        leave_at: NIGHT_MINUTES,
        late_minutes: 0,
        minutes_worked: 0,
        tasks_done: 0,
        hours_week: 0.0,
        toil_hours: 0.0,
        vacation_days: 4,
        skill_boost: None,
        romance_with: None,
        romance_ended_night: None,
        emote: None,
        carrying: None,
        stuck_ticks: 0,
        breaks_tonight: 0,
        met_333: false,
        leave_nights: Vec::new(),
    }
}

fn default_priorities(role: Job) -> [u8; 5] {
    let mut p = [3u8; 5];
    p[role.index()] = 1;
    if role != Job::Serving {
        p[Job::Serving.index()] = 2;
    }
    p
}

/// The fixed story person of an archetype.
pub(crate) fn story_person(w: &mut World, content: &Content, arch_id: &str) -> Option<Staff> {
    let a = content.staff_archetype(arch_id)?;
    let id = w.alloc_id();
    let mut s = blank_staff(id);
    s.name = a.story_name.clone();
    s.archetype = a.id.clone();
    s.story_person = true;
    s.age = a.story_age;
    s.female = a.gender == Gender::Female;
    s.traits = a.story_traits.clone();
    for job in Job::ALL {
        s.skills[job.index()] = a.skills.get(job);
    }
    apply_skill_traits(&mut s, content);
    s.wage = a.wage;
    s.priorities = default_priorities(a.role);
    s.look_seed = hash_str(&a.id);
    Some(s)
}

fn apply_skill_traits(s: &mut Staff, content: &Content) {
    for t in s.traits.clone() {
        if let Some(def) = content.trait_def(&t) {
            for e in &def.effects {
                if let TraitEffect::Skill { job, add } = e {
                    s.skills[job.index()] = (s.skills[job.index()] + add).clamp(0.0, 10.0);
                }
            }
        }
    }
}

/// A random person (sandbox, story extras). `arch` = fixed archetype.
pub(crate) fn generate_staff(w: &mut World, content: &Content, arch: Option<&str>) -> Staff {
    let a = match arch.and_then(|id| content.staff_archetype(id)) {
        Some(a) => a,
        None => {
            let i = w.rng.index(content.staff.len());
            &content.staff[i]
        }
    };
    let id = w.alloc_id();
    let mut s = blank_staff(id);
    s.archetype = a.id.clone();
    s.female = match a.gender {
        Gender::Female => w.rng.chance(0.7),
        Gender::Male => w.rng.chance(0.3),
        Gender::Any => w.rng.chance(0.5),
    };
    s.age = w.rng.range_u32(a.age.0, a.age.1);
    // name
    let pool_id = w.rng.pick(&a.name_pools).cloned().unwrap_or_default();
    if let Some(pool) = content.name_pools.iter().find(|p| p.id == pool_id) {
        let firsts = if s.female {
            &pool.female_first
        } else {
            &pool.male_first
        };
        let first = w.rng.pick(firsts).cloned().unwrap_or_default();
        let li = w.rng.index(pool.male_last.len().max(1));
        let last = if s.female && !pool.female_last.is_empty() {
            pool.female_last
                .get(li)
                .or(pool.female_last.first())
                .cloned()
        } else {
            pool.male_last.get(li).cloned()
        }
        .unwrap_or_default();
        s.name = format!("{first} {last}").trim().to_string();
    }
    for job in Job::ALL {
        let base = a.skills.get(job);
        let v = base + w.rng.range_f32(-a.skill_spread, a.skill_spread);
        s.skills[job.index()] = (v * 2.0).round() / 2.0;
        s.skills[job.index()] = s.skills[job.index()].clamp(0.0, 10.0);
    }
    // two traits, never conflicting
    let mut pool = a.trait_pool.clone();
    while s.traits.len() < 2 && !pool.is_empty() {
        let i = w.rng.index(pool.len());
        let t = pool.remove(i);
        let conflicts = content
            .trait_def(&t)
            .map(|d| d.conflicts.clone())
            .unwrap_or_default();
        if s.traits.iter().any(|x| conflicts.contains(x)) {
            continue;
        }
        s.traits.push(t);
    }
    apply_skill_traits(&mut s, content);
    let base_avg: f32 = Job::ALL.iter().map(|j| a.skills.get(*j)).sum::<f32>() / 5.0;
    let avg: f32 = s.skills.iter().sum::<f32>() / 5.0;
    s.wage = ((a.wage as f32 * (1.0 + 0.06 * (avg - base_avg))) / 10.0).round() as u32 * 10;
    s.priorities = default_priorities(a.role);
    s.look_seed = w.rng.fork();
    s
}

pub(crate) fn refresh_candidates(w: &mut World, content: &Content) {
    match w.mode {
        Mode::Story => {
            let hired: Vec<String> = w.staff.iter().map(|s| s.archetype.clone()).collect();
            let offered: Vec<String> = w
                .candidates
                .iter()
                .map(|c| c.staff.archetype.clone())
                .collect();
            let ids: Vec<String> = content
                .staff
                .iter()
                .filter(|a| a.hire_from_night <= w.night)
                .map(|a| a.id.clone())
                .collect();
            for a in ids {
                if !hired.contains(&a)
                    && !offered.contains(&a)
                    && let Some(s) = story_person(w, content, &a)
                {
                    let id = s.id;
                    w.candidates.push(Candidate { id, staff: s });
                }
            }
        }
        Mode::Sandbox => {
            if w.candidates.is_empty() || (w.night - 1).is_multiple_of(3) {
                w.candidates.clear();
                for _ in 0..3 {
                    let s = generate_staff(w, content, None);
                    let id = s.id;
                    w.candidates.push(Candidate { id, staff: s });
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Traits & situations
// ---------------------------------------------------------------------------------------------

fn effects_of<'a>(content: &'a Content, s: &Staff) -> Vec<&'a TraitEffect> {
    s.traits
        .iter()
        .filter_map(|t| content.trait_def(t))
        .flat_map(|d| d.effects.iter())
        .collect()
}

pub(crate) fn mood_floor(s: &Staff, content: &Content) -> f32 {
    let mut f = 0.0f32;
    for e in effects_of(content, s) {
        if let TraitEffect::MoodFloor(x) = e {
            f = f.max(*x);
        }
    }
    f
}

pub(crate) fn situation(w: &World, idx: usize, sit: &Situation) -> bool {
    let s = &w.staff[idx];
    let now = w.clock();
    match sit {
        Situation::TimeFrom(t) => now >= *t,
        Situation::TimeBefore(t) => now < *t,
        Situation::ShiftStart
        | Situation::BadEvent
        | Situation::MetGuest333
        | Situation::RomanceEnded => false,
        Situation::Serving => matches!(
            s.task,
            Some(Task::TakeOrder { .. }) | Some(Task::Serve { .. })
        ),
        Situation::QueueAtLeast(n) => w.queue_len() as u32 >= *n,
        Situation::QueueAtMost(n) => w.queue_len() as u32 <= *n,
        Situation::AloneInRoom => {
            let room = w.grid.room(s.tile().0, s.tile().1);
            !w.staff
                .iter()
                .any(|o| o.id != s.id && o.on_duty() && w.grid.room(o.tile().0, o.tile().1) == room)
        }
        Situation::StaffNearbyAtLeast(n) => {
            w.staff
                .iter()
                .filter(|o| o.id != s.id && o.on_duty() && grid::chebyshev(o.tile(), s.tile()) <= 3)
                .count() as u32
                >= *n
        }
        Situation::DirtAbove(f) => w.dirt > *f,
        Situation::MusicOn => w.music_on,
        Situation::MusicOff => !w.music_on,
        Situation::Raining => w.weather.is_wet(),
    }
}

fn when_ok(w: &World, idx: usize, when: &Option<Situation>) -> bool {
    when.as_ref().is_none_or(|s| situation(w, idx, s))
}

/// Work speed multiplier for a job (skill, traits, needs, events).
pub(crate) fn work_speed(w: &World, content: &Content, idx: usize, job: Job) -> f32 {
    let s = &w.staff[idx];
    let mut v = 0.5 + s.skill(job, w.night) * 0.1;
    for e in effects_of(content, s) {
        if let TraitEffect::Speed { job: j, mul, when } = e
            && j.is_none_or(|j| j == job)
            && when_ok(w, idx, when)
        {
            v *= mul;
        }
    }
    for m in &w.mods {
        if let ModKind::WorkSpeed(j) = m.kind
            && j.is_none_or(|j| j as usize == job.index())
        {
            v *= m.mul;
        }
    }
    if s.energy < 15.0 {
        v *= 0.65;
    } else if s.energy < 35.0 {
        v *= 0.85;
    }
    v *= 0.85 + s.mood / 100.0 * 0.3;
    if w.sink_dishes > 25 && matches!(job, Job::Bar | Job::Kitchen) {
        v *= 0.75;
    }
    v.max(0.0)
}

fn need_rate(w: &World, content: &Content, idx: usize, need: Need) -> f32 {
    let mut v = 1.0;
    for e in effects_of(content, &w.staff[idx]) {
        if let TraitEffect::NeedRate { need: n, mul, when } = e
            && *n == need
            && when_ok(w, idx, when)
        {
            v *= mul;
        }
    }
    v
}

/// One-shot trait moods (`ShiftStart`, `BadEvent`, `MetGuest333`).
pub(crate) fn one_shot_mood(w: &mut World, content: &Content, which: &Situation) {
    for i in 0..w.staff.len() {
        if !w.staff[i].on_duty() {
            continue;
        }
        let mut add = 0.0;
        for e in effects_of(content, &w.staff[i]) {
            if let TraitEffect::Mood { add: a, when } = e
                && when == which
            {
                add += a;
            }
        }
        if add != 0.0 {
            super::logic::change_mood(w, content, i, add, false);
        }
    }
}

pub(crate) fn add_relationship(w: &mut World, a: u32, b: u32, add: f32) {
    if a == b {
        return;
    }
    let key = if a < b { (a, b) } else { (b, a) };
    let r = w.relationships.entry(key).or_insert(0.0);
    *r = (*r + add).clamp(-100.0, 100.0);
}

// ---------------------------------------------------------------------------------------------
// Shifts
// ---------------------------------------------------------------------------------------------

fn staff_tiles(w: &World) -> Vec<(i32, i32)> {
    let mut v = Vec::new();
    for y in 0..w.grid.height {
        for x in 0..w.grid.width {
            if w.grid.room(x, y).staff_only() && w.grid.walkable(x, y) {
                v.push((x, y));
            }
        }
    }
    v
}

fn idle_tile(w: &World, content: &Content, idx: usize) -> (i32, i32) {
    let s = &w.staff[idx];
    if s.priorities[Job::Register.index()] == 1
        && let Some(t) = guests::registers(w, content).first().and_then(|(uid, _)| {
            let p = w.furniture_by_uid(*uid)?;
            let d = content.furniture_def(&p.def)?;
            grid::access_tiles(&w.grid, d, p)
                .into_iter()
                .find(|t| !w.grid.guest_ok(t.0, t.1))
        })
    {
        return t;
    }
    let tiles = staff_tiles(w);
    if tiles.is_empty() {
        return w.grid.entrance();
    }
    // spread along the bar side, near the counter
    let mut near: Vec<(i32, i32)> = tiles.clone();
    near.sort_by_key(|t| (t.1, t.0));
    let k = (idx * 3 + 2) % near.len().max(1);
    near[k]
}

fn break_tile(w: &World, idx: usize) -> (i32, i32) {
    let mut tiles = staff_tiles(w);
    tiles.sort_by_key(|t| (-t.0, -t.1));
    tiles
        .get(idx % tiles.len().max(1))
        .copied()
        .unwrap_or_else(|| w.grid.entrance())
}

pub(crate) fn on_night_start(w: &mut World, content: &Content) {
    let wd = w.weekday as usize;
    let cozy = w.difficulty == Difficulty::Cozy;
    let night = w.night;
    for i in 0..w.staff.len() {
        let id = w.staff[i].id;
        let mut shift = w
            .app
            .published
            .get(&id)
            .map(|s| s[wd])
            .unwrap_or(Shift::Full);
        if w.staff[i].leave_nights.iter().any(|(n, _)| *n == night) {
            shift = Shift::Off;
        }
        let s = &mut w.staff[i];
        s.shift = shift;
        s.minutes_worked = 0;
        s.tasks_done = 0;
        s.late_minutes = 0;
        s.breaks_tonight = 0;
        s.task = None;
        s.path.clear();
        s.carrying = None;
        s.met_333 = false;
        let Some((start, end)) = shift.span() else {
            s.status = StaffStatus::OffDuty;
            continue;
        };
        s.leave_at = end;
        s.status = StaffStatus::Expected { minute: start };
    }
    // sick calls & late arrivals
    for i in 0..w.staff.len() {
        if w.staff[i].owner || w.staff[i].status == StaffStatus::OffDuty {
            continue;
        }
        let sick = if cozy { 0.008 } else { 0.015 };
        let late = if cozy { 0.04 } else { 0.06 };
        if w.rng.chance(sick) {
            w.staff[i].status = StaffStatus::Sick;
            let id = w.staff[i].id;
            w.emit(SimEvent::StaffSick { staff: id });
            let ctx = EffectCtx {
                staff: Some(id),
                ..Default::default()
            };
            super::app::bot(w, content, "sick_call", &ctx);
        } else if w.rng.chance(late)
            && let StaffStatus::Expected { minute } = w.staff[i].status
        {
            let mins = w.rng.range_u32(5, 25);
            w.staff[i].status = StaffStatus::Expected {
                minute: minute + mins,
            };
            w.staff[i].late_minutes = mins;
            let id = w.staff[i].id;
            w.emit(SimEvent::StaffLate {
                staff: id,
                minutes: mins,
            });
            let ctx = EffectCtx {
                staff: Some(id),
                ..Default::default()
            };
            super::app::bot_with(
                w,
                content,
                "late_alert",
                &ctx,
                vec![("minutes".into(), mins.to_string())],
            );
        }
    }
    // those starting at opening are already inside
    for i in 0..w.staff.len() {
        if w.staff[i].status == (StaffStatus::Expected { minute: 0 }) {
            let t = idle_tile(w, content, i);
            let s = &mut w.staff[i];
            s.status = StaffStatus::OnDuty;
            s.pos = grid::tile_center(t);
            s.pose = Pose::Idle;
        }
    }
    one_shot_mood(w, content, &Situation::ShiftStart);
    // music lovers turn the radio on
    let has_music = w.furniture.iter().any(|p| {
        content
            .furniture_def(&p.def)
            .is_some_and(|d| d.has_tag("music"))
    });
    if has_music
        && !w.music_on
        && w.staff.iter().any(|s| {
            s.on_duty()
                && effects_of(content, s)
                    .iter()
                    .any(|e| matches!(e, TraitEffect::PlaysMusic))
        })
    {
        set_music(w, content, true);
    }
}

pub(crate) fn set_music(w: &mut World, content: &Content, on: bool) {
    w.music_on = on;
    for p in w.furniture.iter_mut() {
        if content
            .furniture_def(&p.def)
            .is_some_and(|d| d.has_tag("music"))
        {
            p.on = on;
        }
    }
    w.emit(SimEvent::MusicChanged { on });
}

/// Takes someone off the floor (sick, walkout). They walk out.
pub(crate) fn send_home(w: &mut World, id: u32, status: StaffStatus) {
    let Some(i) = w.staff.iter().position(|s| s.id == id) else {
        return;
    };
    abandon(w, i);
    let present = w.staff[i].present();
    w.staff[i].status = status;
    if present {
        // walk out visibly
        let edge = w.grid.street_edges()[0];
        let from = w.staff[i].tile();
        let path = w.paths.get(&w.grid, from, edge, false).unwrap_or_default();
        let s = &mut w.staff[i];
        s.path = path;
        s.task = Some(Task::Leave);
    }
}

// ---------------------------------------------------------------------------------------------
// Work finding
// ---------------------------------------------------------------------------------------------

struct Work {
    task: Task,
    goal: (i32, i32),
    work: f32,
    rank: u8,
}

fn station_access(w: &World, content: &Content, uid: u32) -> Option<(i32, i32)> {
    let p = w.furniture_by_uid(uid)?;
    let d = content.furniture_def(&p.def)?;
    let tiles = grid::access_tiles(&w.grid, d, p);
    tiles
        .iter()
        .find(|t| w.grid.room(t.0, t.1).staff_only())
        .or(tiles.first())
        .copied()
}

fn stand_next_to(w: &World, t: (i32, i32), from: (i32, i32)) -> Option<(i32, i32)> {
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
        if !w.grid.walkable(n.0, n.1) {
            continue;
        }
        // not on someone's seat
        if w.guests
            .iter()
            .any(|g| g.pose == Pose::Sit && g.tile() == n)
        {
            continue;
        }
        if best.is_none_or(|b| grid::manhattan(n, from) < grid::manhattan(b, from)) {
            best = Some(n);
        }
    }
    best
}

fn guest_seat_tile(w: &World, gid: u32) -> Option<(i32, i32)> {
    let g = w.guests.iter().find(|g| g.id == gid)?;
    let (uid, idx) = g.seat?;
    guests::seat_tile(w, uid, idx)
}

fn station_free(w: &World, content: &Content, tag: &str, staff: u32) -> Option<u32> {
    if w.mods
        .iter()
        .any(|m| matches!(&m.kind, ModKind::StationDown(t) if t == tag))
    {
        return None;
    }
    w.furniture
        .iter()
        .filter(|p| {
            content
                .furniture_def(&p.def)
                .is_some_and(|d| d.has_tag(tag))
                && p.user.is_none_or(|u| u == staff)
        })
        .map(|p| p.uid)
        .next()
}

fn works_for(w: &World, content: &Content, idx: usize, job: Job) -> Vec<Work> {
    let s = &w.staff[idx];
    let me = s.id;
    let here = s.tile();
    let m = w.minute();
    let mut out = Vec::new();
    if work_speed(w, content, idx, job) <= 0.01 {
        return out;
    }
    match job {
        Job::Register => {
            for (reg, _) in guests::registers(w, content) {
                let Some(q) = w.queues.get(&reg) else {
                    continue;
                };
                let Some(&pid) = q.first() else { continue };
                let Some(p) = w.parties.iter().find(|p| p.id == pid) else {
                    continue;
                };
                if p.order_claim.is_some() || p.state != PartyState::Queue {
                    continue;
                }
                let reg_free = w
                    .furniture_by_uid(reg)
                    .is_some_and(|f| f.user.is_none_or(|u| u == me));
                if !reg_free {
                    continue;
                }
                let Some(goal) = station_access(w, content, reg) else {
                    continue;
                };
                out.push(Work {
                    task: Task::TakeOrder {
                        party: pid,
                        register: Some(reg),
                    },
                    goal,
                    work: 10.0 + 3.0 * p.wishes.len() as f32,
                    rank: 0,
                });
            }
            for c in checklist_works(w, content, job, me) {
                out.push(c);
            }
        }
        Job::Serving => {
            for it in &w.items {
                if it.state == ItemState::Ready && it.by.is_none() {
                    let Some(goal) = it.ready_tile else { continue };
                    out.push(Work {
                        task: Task::Serve {
                            item: it.id,
                            picked: false,
                            extra: Vec::new(),
                        },
                        goal,
                        work: 3.0,
                        rank: 0,
                    });
                }
            }
            for p in &w.parties {
                if p.state != PartyState::WaitOrder || p.order_claim.is_some() {
                    continue;
                }
                let Some(&leader) = p.members.first() else {
                    continue;
                };
                let seated = w
                    .guests
                    .iter()
                    .any(|g| g.id == leader && g.pose == Pose::Sit);
                if !seated {
                    continue;
                }
                let Some(st) = guest_seat_tile(w, leader) else {
                    continue;
                };
                let Some(goal) = stand_next_to(w, st, here) else {
                    continue;
                };
                out.push(Work {
                    task: Task::TakeOrder {
                        party: p.id,
                        register: None,
                    },
                    goal,
                    work: 10.0 + 3.0 * p.wishes.len() as f32,
                    rank: 0,
                });
            }
            for c in checklist_works(w, content, job, me) {
                out.push(c);
            }
        }
        Job::Bar | Job::Kitchen => {
            for it in &w.items {
                if it.state != ItemState::Pending || it.by.is_some() {
                    continue;
                }
                let Some(mi) = content.menu_item(&it.menu) else {
                    continue;
                };
                if mi.job != job || s.skill(job, w.night) < mi.min_skill {
                    continue;
                }
                if mi.needs_power && w.blackout_until.is_some() {
                    continue;
                }
                let Some(st) = station_free(w, content, &mi.station, me) else {
                    continue;
                };
                let Some(goal) = station_access(w, content, st) else {
                    continue;
                };
                out.push(Work {
                    task: Task::Prepare {
                        item: it.id,
                        station: st,
                    },
                    goal,
                    work: it.prep_left,
                    rank: 1,
                });
                if out.len() > 6 {
                    break;
                }
            }
            // waiting items prepared earlier, oldest first
            out.sort_by_key(|w2| match &w2.task {
                Task::Prepare { item, .. } => *item,
                _ => 0,
            });
            for c in checklist_works(w, content, job, me) {
                out.push(c);
            }
        }
        Job::Cleaning => {
            let seat_info = guests::seats(w, content);
            for p in &w.furniture {
                if p.dishes == 0 || p.claim.is_some() {
                    continue;
                }
                let occupied = seat_info.iter().any(|s| s.table == Some(p.uid) && !s.free);
                if occupied && p.dishes < 4 {
                    continue;
                }
                let Some(goal) = station_access(w, content, p.uid) else {
                    continue;
                };
                out.push(Work {
                    task: Task::ClearTable {
                        table: p.uid,
                        picked: false,
                        dishes: 0,
                    },
                    goal,
                    work: 5.0 + 4.0 * p.dishes as f32,
                    rank: 0,
                });
            }
            if w.sink_dishes > 0
                && w.sink_claim.is_none()
                && let Some(sink) = w.furniture.iter().find(|p| {
                    content
                        .furniture_def(&p.def)
                        .is_some_and(|d| d.has_tag("sink"))
                })
                && let Some(goal) = station_access(w, content, sink.uid)
            {
                out.push(Work {
                    task: Task::Wash { sink: sink.uid },
                    goal,
                    work: 4.0 * w.sink_dishes.min(12) as f32,
                    rank: 1,
                });
            }
            for (k, b) in w.buckets.iter().enumerate() {
                if b.2.is_none() && b.1 <= m {
                    if let Some(goal) = stand_next_to(w, b.0, here) {
                        out.push(Work {
                            task: Task::Bucket { tile: b.0 },
                            goal,
                            work: 20.0,
                            rank: 2,
                        });
                    }
                    let _ = k;
                }
            }
            if w.dirt > 0.12 && w.mop_claim.is_none() {
                // a hall tile near the middle of the mess (deterministic by minute)
                let hall: Vec<(i32, i32)> = (0..w.grid.height)
                    .flat_map(|y| (0..w.grid.width).map(move |x| (x, y)))
                    .filter(|t| w.grid.room(t.0, t.1) == Room::Hall && w.grid.walkable(t.0, t.1))
                    .filter(|t| w.grid.object_at(t.0, t.1).is_none())
                    .collect();
                if !hall.is_empty() {
                    let t = hall[(m as usize * 7 + me as usize) % hall.len()];
                    out.push(Work {
                        task: Task::Mop { tile: t },
                        goal: t,
                        work: 40.0,
                        rank: 3,
                    });
                }
            }
            for c in checklist_works(w, content, job, me) {
                out.push(c);
            }
        }
    }
    out
}

fn checklist_works(w: &World, content: &Content, job: Job, _me: u32) -> Vec<Work> {
    let m = w.minute();
    let mut out = Vec::new();
    for c in &content.app.checklists {
        if c.job != Some(job) {
            continue;
        }
        let window = match c.list {
            ChecklistKind::Opening => m < 120,
            ChecklistKind::Closing => m >= 420,
        };
        if !window {
            continue;
        }
        let Some(st) = w.app.checklist.iter().find(|x| x.id == c.id) else {
            continue;
        };
        if st.done || st.claim.is_some() {
            continue;
        }
        let tile = checklist_tile(w, content, job);
        out.push(Work {
            task: Task::Checklist {
                id: c.id.clone(),
                tile,
            },
            goal: tile,
            work: c.minutes.max(1) as f32 * TICKS_PER_MINUTE as f32,
            rank: 5,
        });
    }
    out
}

fn checklist_tile(w: &World, content: &Content, job: Job) -> (i32, i32) {
    let tag = match job {
        Job::Bar => "station:coffee",
        Job::Kitchen => "station:fridge",
        Job::Register => "register",
        Job::Cleaning => "sink",
        Job::Serving => "table",
    };
    w.furniture
        .iter()
        .find(|p| {
            content
                .furniture_def(&p.def)
                .is_some_and(|d| d.has_tag(tag))
        })
        .and_then(|p| station_access(w, content, p.uid))
        .unwrap_or_else(|| w.grid.entrance())
}

fn effective_priority(w: &World, content: &Content, idx: usize, job: Job) -> u8 {
    let s = &w.staff[idx];
    let p = s.priorities[job.index()];
    if p > 0 {
        return p;
    }
    if job == Job::Cleaning
        && effects_of(content, s)
            .iter()
            .any(|e| matches!(e, TraitEffect::AutoClean))
    {
        return 4;
    }
    // Nobody on the floor does this job → anyone helps out (keeps the café from deadlocking).
    let covered = w
        .staff
        .iter()
        .any(|o| o.on_duty() && o.priorities[job.index()] > 0);
    if !covered { 4 } else { 0 }
}

fn find_work(w: &World, content: &Content, idx: usize) -> Option<Work> {
    let here = w.staff[idx].tile();
    for level in 1..=4u8 {
        let mut best: Option<(i32, Work)> = None;
        for job in Job::ALL {
            if effective_priority(w, content, idx, job) != level {
                continue;
            }
            for wk in works_for(w, content, idx, job) {
                let score = wk.rank as i32 * 40 + grid::manhattan(here, wk.goal);
                if best.as_ref().is_none_or(|b| score < b.0) {
                    best = Some((score, wk));
                }
            }
        }
        if let Some((_, wk)) = best {
            return Some(wk);
        }
    }
    None
}

fn claim(w: &mut World, task: &Task, me: u32) {
    match task {
        Task::TakeOrder { party, register } => {
            if let Some(p) = w.parties.iter_mut().find(|p| p.id == *party) {
                p.order_claim = Some(me);
            }
            if let Some(r) = register
                && let Some(f) = w.furniture.iter_mut().find(|f| f.uid == *r)
            {
                f.user = Some(me);
            }
        }
        Task::Prepare { item, station } => {
            if let Some(i) = w.items.iter_mut().find(|i| i.id == *item) {
                i.by = Some(me);
            }
            if let Some(f) = w.furniture.iter_mut().find(|f| f.uid == *station) {
                f.user = Some(me);
            }
        }
        Task::Serve { item, .. } => {
            if let Some(i) = w.items.iter_mut().find(|i| i.id == *item) {
                i.by = Some(me);
            }
        }
        Task::ClearTable { table, .. } => {
            if let Some(f) = w.furniture.iter_mut().find(|f| f.uid == *table) {
                f.claim = Some(me);
            }
        }
        Task::Wash { .. } => w.sink_claim = Some(me),
        Task::Mop { .. } => w.mop_claim = Some(me),
        Task::Bucket { tile } => {
            if let Some(b) = w.buckets.iter_mut().find(|b| b.0 == *tile) {
                b.2 = Some(me);
            }
        }
        Task::Checklist { id, .. } => {
            if let Some(c) = w.app.checklist.iter_mut().find(|c| &c.id == id) {
                c.claim = Some(me);
            }
        }
        _ => {}
    }
}

/// Drops the current task and releases its claims.
pub(crate) fn abandon(w: &mut World, idx: usize) {
    let me = w.staff[idx].id;
    let Some(task) = w.staff[idx].task.take() else {
        return;
    };
    w.staff[idx].path.clear();
    w.staff[idx].work_left = 0.0;
    match task {
        Task::TakeOrder { party, register } => {
            if let Some(p) = w.parties.iter_mut().find(|p| p.id == party)
                && p.order_claim == Some(me)
            {
                p.order_claim = None;
            }
            if let Some(r) = register
                && let Some(f) = w.furniture.iter_mut().find(|f| f.uid == r)
                && f.user == Some(me)
            {
                f.user = None;
            }
        }
        Task::Prepare { item, station } => {
            if let Some(i) = w.items.iter_mut().find(|i| i.id == item)
                && i.by == Some(me)
            {
                i.by = None;
                if i.state == ItemState::Preparing {
                    i.state = ItemState::Pending;
                }
            }
            if let Some(f) = w.furniture.iter_mut().find(|f| f.uid == station)
                && f.user == Some(me)
            {
                f.user = None;
            }
        }
        Task::Serve { item, extra, .. } => {
            let here = w.staff[idx].tile();
            for id in std::iter::once(item).chain(extra) {
                if let Some(i) = w.items.iter_mut().find(|i| i.id == id)
                    && i.by == Some(me)
                {
                    i.by = None;
                    if i.state == ItemState::Carrying {
                        i.state = ItemState::Ready;
                        i.ready_tile = Some(here);
                    }
                }
            }
            w.staff[idx].carrying = None;
        }
        Task::ClearTable {
            table,
            picked,
            dishes,
        } => {
            if let Some(f) = w.furniture.iter_mut().find(|f| f.uid == table)
                && f.claim == Some(me)
            {
                f.claim = None;
            }
            if picked {
                w.sink_dishes += dishes;
            }
            w.staff[idx].carrying = None;
        }
        Task::Wash { .. } => {
            if w.sink_claim == Some(me) {
                w.sink_claim = None;
            }
        }
        Task::Mop { .. } => {
            if w.mop_claim == Some(me) {
                w.mop_claim = None;
            }
        }
        Task::Bucket { tile } => {
            if let Some(b) = w.buckets.iter_mut().find(|b| b.0 == tile)
                && b.2 == Some(me)
            {
                b.2 = None;
            }
        }
        Task::Checklist { id, .. } => {
            if let Some(c) = w.app.checklist.iter_mut().find(|c| c.id == id)
                && c.claim == Some(me)
            {
                c.claim = None;
            }
        }
        _ => {}
    }
}

/// Is the task still meaningful?
fn task_valid(w: &World, idx: usize) -> bool {
    let me = w.staff[idx].id;
    match &w.staff[idx].task {
        Some(Task::TakeOrder { party, register }) => w.parties.iter().any(|p| {
            p.id == *party
                && p.order_claim == Some(me)
                && match register {
                    Some(_) => p.state == PartyState::Queue,
                    None => p.state == PartyState::WaitOrder,
                }
        }),
        Some(Task::Prepare { item, .. }) => w.items.iter().any(|i| {
            i.id == *item
                && i.by == Some(me)
                && matches!(i.state, ItemState::Pending | ItemState::Preparing)
        }),
        Some(Task::Serve { item, .. }) => w.items.iter().any(|i| {
            i.id == *item
                && i.by == Some(me)
                && matches!(i.state, ItemState::Ready | ItemState::Carrying)
        }),
        Some(Task::Checklist { id, .. }) => w.app.checklist.iter().any(|c| &c.id == id && !c.done),
        Some(_) => true,
        None => false,
    }
}

// ---------------------------------------------------------------------------------------------
// Ticking
// ---------------------------------------------------------------------------------------------

fn go(w: &mut World, idx: usize, goal: (i32, i32)) -> bool {
    let from = w.staff[idx].tile();
    match w.paths.get(&w.grid, from, goal, false) {
        Some(p) => {
            w.staff[idx].path = p;
            true
        }
        None => false,
    }
}

pub(crate) fn tick_staff(w: &mut World, content: &Content) {
    let m = w.minute();
    for idx in 0..w.staff.len() {
        match w.staff[idx].status {
            StaffStatus::Expected { minute } if m >= minute => {
                // arrives from the street
                let edge = w.grid.street_edges()[idx % 2];
                let t = idle_tile(w, content, idx);
                w.staff[idx].pos = grid::tile_center(edge);
                w.staff[idx].status = StaffStatus::OnDuty;
                w.staff[idx].task = Some(Task::Idle { tile: t });
                go(w, idx, t);
                let id = w.staff[idx].id;
                w.emit(SimEvent::StaffArrived { staff: id });
                continue;
            }
            StaffStatus::OnDuty => {}
            StaffStatus::Leaving | StaffStatus::Sick | StaffStatus::WalkedOut => {
                if w.staff[idx].task == Some(Task::Leave) {
                    let s = &mut w.staff[idx];
                    let (vel, arrived, _) =
                        guests::step_along(&w.grid, &mut s.pos, &mut s.path, STAFF_SPEED, false);
                    s.pose = Pose::Walk;
                    if vel.0.abs() + vel.1.abs() > 0.01 {
                        s.facing = Facing::from_delta(vel.0, vel.1);
                    }
                    if arrived {
                        s.task = None;
                        if s.status == StaffStatus::Leaving {
                            s.status = StaffStatus::OffDuty;
                        }
                        let id = s.id;
                        w.emit(SimEvent::StaffLeft { staff: id });
                    }
                }
                continue;
            }
            _ => continue,
        }
        // shift over
        if m >= w.staff[idx].leave_at && w.staff[idx].leave_at < NIGHT_MINUTES {
            abandon(w, idx);
            let edge = w.grid.street_edges()[0];
            w.staff[idx].status = StaffStatus::Leaving;
            w.staff[idx].task = Some(Task::Leave);
            go(w, idx, edge);
            continue;
        }
        if w.staff[idx].task.is_some()
            && !matches!(
                w.staff[idx].task,
                Some(Task::Idle { .. }) | Some(Task::Break { .. })
            )
            && !task_valid(w, idx)
        {
            abandon(w, idx);
        }
        // choose something to do
        let idle = matches!(w.staff[idx].task, None | Some(Task::Idle { .. }));
        if idle && (w.tick + idx as u32).is_multiple_of(2) {
            if let Some(b) = wants_break(w, content, idx) {
                abandon(w, idx);
                let t = break_tile(w, idx);
                w.staff[idx].task = Some(b);
                w.staff[idx].breaks_tonight += 1;
                go(w, idx, t);
                let id = w.staff[idx].id;
                w.emit(SimEvent::StaffBreak { staff: id });
            } else if let Some(wk) = find_work(w, content, idx) {
                w.staff[idx].task = None;
                let me = w.staff[idx].id;
                claim(w, &wk.task, me);
                w.staff[idx].task = Some(wk.task);
                w.staff[idx].work_left = wk.work;
                if !go(w, idx, wk.goal) {
                    abandon(w, idx);
                }
            } else if w.staff[idx].task.is_none() {
                let t = idle_tile(w, content, idx);
                w.staff[idx].task = Some(Task::Idle { tile: t });
                go(w, idx, t);
            }
        }
        step_staff(w, content, idx);
    }
}

fn wants_break(w: &World, content: &Content, idx: usize) -> Option<Task> {
    let s = &w.staff[idx];
    let m = w.minute();
    if !(s.energy < 32.0 || s.hunger < 28.0) || m > 460 {
        return None;
    }
    for e in effects_of(content, s) {
        if let TraitEffect::SkipsBreakIfQueue(n) = e
            && w.queue_len() as u32 > *n
        {
            return None;
        }
    }
    let on: Vec<&Staff> = w.staff.iter().filter(|o| o.on_duty()).collect();
    let on_break = on
        .iter()
        .filter(|o| matches!(o.task, Some(Task::Break { .. })))
        .count();
    if on.len() > 1 && on_break >= on.len().div_ceil(3) {
        return None;
    }
    let mut len = 15.0;
    for e in effects_of(content, s) {
        if let TraitEffect::BreakLength(x) = e {
            len *= x;
        }
    }
    let eat = s.hunger < 55.0;
    if eat {
        len += 5.0;
    }
    Some(Task::Break {
        until: m + len as u32,
        eat,
    })
}

fn step_staff(w: &mut World, content: &Content, idx: usize) {
    // movement
    if !w.staff[idx].path.is_empty() {
        let carrying = w.staff[idx].carrying.is_some();
        let s = &mut w.staff[idx];
        let before = s.pos;
        let (vel, _arrived, blocked) =
            guests::step_along(&w.grid, &mut s.pos, &mut s.path, STAFF_SPEED, false);
        s.pose = if carrying { Pose::Carry } else { Pose::Walk };
        if vel.0.abs() + vel.1.abs() > 0.01 {
            s.facing = Facing::from_delta(vel.0, vel.1);
            s.stuck_ticks = 0;
        } else {
            s.stuck_ticks += 1;
        }
        let _ = before;
        if blocked {
            let goal = *w.staff[idx].path.last().unwrap_or(&w.staff[idx].tile());
            if !go(w, idx, goal) {
                abandon(w, idx);
            }
        }
        if w.staff[idx].stuck_ticks > 300 {
            // reset a stuck agent
            let goal = w.staff[idx].path.last().copied();
            if let Some(g) = goal {
                w.staff[idx].pos = grid::tile_center(g);
                w.staff[idx].path.clear();
            }
            w.staff[idx].stuck_ticks = 0;
            w.tonight.stuck_resets += 1;
        }
        return;
    }
    // at the goal: work
    let me = w.staff[idx].id;
    let task = w.staff[idx].task.clone();
    match task {
        None => {
            w.staff[idx].pose = Pose::Idle;
        }
        Some(Task::Idle { .. }) => {
            w.staff[idx].pose = Pose::Idle;
            w.staff[idx].facing = Facing::Up;
        }
        Some(Task::Break { until, eat }) => {
            w.staff[idx].pose = Pose::Idle;
            if w.minute() >= until {
                if eat {
                    w.econ.supplies -= 15;
                }
                w.staff[idx].task = None;
            } else if w.tick.is_multiple_of(TICKS_PER_MINUTE) {
                let s = &mut w.staff[idx];
                s.energy = (s.energy + 2.2).min(100.0);
                s.mood = (s.mood + 0.4).min(100.0);
                if eat {
                    s.hunger = (s.hunger + 5.0).min(100.0);
                }
                if w.tick.is_multiple_of(60) {
                    s.emote = Some((SimEmote::Coffee, w.tick + 20));
                }
            }
        }
        Some(Task::Leave) => {}
        Some(t) => {
            let job = match &t {
                Task::Prepare { item, .. } => w
                    .items
                    .iter()
                    .find(|i| i.id == *item)
                    .and_then(|i| content.menu_item(&i.menu))
                    .map(|m| m.job)
                    .unwrap_or(Job::Bar),
                Task::Checklist { id, .. } => content
                    .app
                    .checklists
                    .iter()
                    .find(|c| &c.id == id)
                    .and_then(|c| c.job)
                    .unwrap_or(Job::Serving),
                other => other.job().unwrap_or(Job::Serving),
            };
            let mut speed = work_speed(w, content, idx, job);
            if let Task::Prepare { item, station } = &t {
                // station slow-downs
                if let Some(tag) = w
                    .furniture_by_uid(*station)
                    .and_then(|p| content.furniture_def(&p.def))
                    .and_then(|d| d.tags.iter().find(|t| t.starts_with("station:")).cloned())
                {
                    for md in &w.mods {
                        if matches!(&md.kind, ModKind::StationSlow(x) if *x == tag) {
                            speed *= md.mul;
                        }
                    }
                }
                if let Some(i) = w.items.iter_mut().find(|i| i.id == *item) {
                    i.state = ItemState::Preparing;
                    i.prep_left -= speed;
                    w.staff[idx].work_left = i.prep_left;
                }
                if let Some(f) = w.furniture_by_uid(*station) {
                    let dir = (f.x - w.staff[idx].tile().0, f.y - w.staff[idx].tile().1);
                    w.staff[idx].facing = Facing::from_delta(dir.0 as f32, dir.1 as f32);
                }
                w.staff[idx].pose = Pose::Work;
                if w.staff[idx].work_left <= 0.0 {
                    complete(w, content, idx, t, job);
                }
                return;
            }
            w.staff[idx].pose = match &t {
                Task::Serve { picked: true, .. } | Task::ClearTable { picked: true, .. } => {
                    Pose::Carry
                }
                _ => Pose::Work,
            };
            w.staff[idx].work_left -= speed;
            if w.staff[idx].work_left <= 0.0 {
                complete(w, content, idx, t, job);
            }
            let _ = me;
        }
    }
}

fn learn(w: &mut World, idx: usize, job: Job) {
    let s = &mut w.staff[idx];
    s.skills[job.index()] = (s.skills[job.index()] + 0.012).min(10.0);
    s.tasks_done += 1;
}

fn mishap(w: &mut World, content: &Content, idx: usize) {
    let mut chance = 0.003;
    let mut mul = 1.0;
    for i in 0..w.staff[idx].traits.len() {
        let t = w.staff[idx].traits[i].clone();
        if let Some(def) = content.trait_def(&t) {
            for e in &def.effects {
                match e {
                    TraitEffect::Mishap { chance: c, when } if when_ok(w, idx, when) => {
                        chance += c * 0.5
                    }
                    TraitEffect::ErrorChance { mul: m, when } if when_ok(w, idx, when) => mul *= m,
                    _ => {}
                }
            }
        }
    }
    if w.difficulty == Difficulty::Cozy {
        chance *= 0.6;
    }
    if w.rng.chance(chance * mul) {
        let id = w.staff[idx].id;
        w.tonight.dishes_broken += 1;
        w.emit(SimEvent::DishBroken { staff: id });
        let ctx = EffectCtx {
            staff: Some(id),
            ..Default::default()
        };
        super::narrator::fire(w, content, "broken_dishes", ctx, true);
    }
}

fn complete(w: &mut World, content: &Content, idx: usize, task: Task, job: Job) {
    let me = w.staff[idx].id;
    match task {
        Task::TakeOrder { party, register } => {
            w.staff[idx].task = None;
            if let Some(r) = register
                && let Some(f) = w.furniture.iter_mut().find(|f| f.uid == r)
            {
                f.user = None;
            }
            guests::order_taken(w, content, party, me);
            learn(w, idx, Job::Register);
        }
        Task::Prepare { item, station } => {
            w.staff[idx].task = None;
            if let Some(f) = w.furniture.iter_mut().find(|f| f.uid == station) {
                f.user = None;
            }
            let here = w.staff[idx].tile();
            let mut quality = 0.0;
            for e in effects_of(content, &w.staff[idx]) {
                if let TraitEffect::OrderQuality(q) = e {
                    quality += q;
                }
            }
            quality += w.staff[idx].skill(job, w.night) * 1.5;
            let mut name = String::new();
            if let Some(i) = w.items.iter_mut().find(|i| i.id == item) {
                i.state = ItemState::Ready;
                i.by = None;
                i.ready_tile = Some(here);
                i.quality = quality;
                name = i.menu.clone();
            }
            w.emit(SimEvent::OrderReady {
                item: name,
                staff: me,
            });
            learn(w, idx, job);
            mishap(w, content, idx);
        }
        Task::Serve {
            item,
            picked,
            extra,
        } => {
            if !picked {
                // pick up: put more ready things for the same party on the tray
                let Some((gid, menu, party, at)) = w
                    .items
                    .iter()
                    .find(|i| i.id == item)
                    .map(|i| (i.guest, i.menu.clone(), i.party, i.ready_tile))
                else {
                    abandon(w, idx);
                    return;
                };
                let here = w.staff[idx].tile();
                let mut tray = Vec::new();
                for i in w.items.iter_mut() {
                    if tray.len() >= TRAY - 1 {
                        break;
                    }
                    if i.id != item
                        && i.party == party
                        && i.state == ItemState::Ready
                        && i.by.is_none()
                        && i.ready_tile.is_some_and(|t| {
                            grid::chebyshev(t, here) <= 3
                                || at.is_some_and(|a| grid::chebyshev(t, a) <= 3)
                        })
                    {
                        i.by = Some(me);
                        i.state = ItemState::Carrying;
                        tray.push(i.id);
                    }
                }
                if let Some(i) = w.items.iter_mut().find(|i| i.id == item) {
                    i.state = ItemState::Carrying;
                }
                w.staff[idx].carrying = Some(menu);
                let target = guest_seat_tile(w, gid).and_then(|t| stand_next_to(w, t, here));
                w.staff[idx].task = Some(Task::Serve {
                    item,
                    picked: true,
                    extra: tray,
                });
                w.staff[idx].work_left = 3.0;
                match target {
                    Some(t) if go(w, idx, t) => {}
                    _ => abandon(w, idx),
                }
            } else {
                w.staff[idx].task = None;
                w.staff[idx].carrying = None;
                for id in std::iter::once(item).chain(extra) {
                    guests::item_served(w, content, id, me);
                }
                learn(w, idx, Job::Serving);
                mishap(w, content, idx);
            }
        }
        Task::ClearTable {
            table,
            picked,
            dishes,
        } => {
            if !picked {
                let dishes = w.furniture_by_uid(table).map(|f| f.dishes).unwrap_or(0);
                if let Some(f) = w.furniture.iter_mut().find(|f| f.uid == table) {
                    f.dishes = 0;
                    f.dirt = 0.0;
                    f.claim = None;
                }
                w.staff[idx].carrying = Some("dishes".into());
                let sink = w
                    .furniture
                    .iter()
                    .find(|p| {
                        content
                            .furniture_def(&p.def)
                            .is_some_and(|d| d.has_tag("dishes"))
                    })
                    .map(|p| p.uid);
                let goal = sink.and_then(|s| station_access(w, content, s));
                w.staff[idx].task = Some(Task::ClearTable {
                    table,
                    picked: true,
                    dishes,
                });
                w.staff[idx].work_left = 2.0;
                let moved = goal.is_some_and(|g| go(w, idx, g));
                if !moved {
                    w.sink_dishes += dishes;
                    w.staff[idx].task = None;
                    w.staff[idx].carrying = None;
                }
                learn(w, idx, Job::Cleaning);
                mishap(w, content, idx);
            } else {
                w.sink_dishes += dishes;
                w.staff[idx].task = None;
                w.staff[idx].carrying = None;
            }
        }
        Task::Wash { .. } => {
            let n = w.sink_dishes.min(12);
            w.sink_dishes -= n;
            w.sink_claim = None;
            w.staff[idx].task = None;
            learn(w, idx, Job::Cleaning);
            mishap(w, content, idx);
        }
        Task::Mop { .. } => {
            let s = 0.5 + w.staff[idx].skill(Job::Cleaning, w.night) * 0.1;
            w.dirt = (w.dirt - 0.07 * s).max(0.0);
            w.mop_claim = None;
            w.staff[idx].task = None;
            learn(w, idx, Job::Cleaning);
        }
        Task::Bucket { tile } => {
            w.buckets.retain(|b| b.0 != tile);
            if let Some(i) = w.grid.idx(tile.0, tile.1) {
                w.grid.blocked[i] = false;
            }
            w.grid.rebuild(&w.furniture, content);
            w.staff[idx].task = None;
            learn(w, idx, Job::Cleaning);
        }
        Task::Checklist { id, .. } => {
            w.staff[idx].task = None;
            super::app::tick_checklist(w, content, &id, Some(me));
            learn(w, idx, job);
        }
        _ => {
            w.staff[idx].task = None;
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Per-minute: needs, moods, relationships
// ---------------------------------------------------------------------------------------------

pub(crate) fn on_minute(w: &mut World, content: &Content) {
    let decay = content
        .difficulty(w.difficulty.id())
        .map(|d| d.need_decay_mul)
        .unwrap_or(1.0);
    let n = w.staff.len();
    for idx in 0..n {
        if !w.staff[idx].on_duty() {
            continue;
        }
        let working = !matches!(
            w.staff[idx].task,
            None | Some(Task::Idle { .. }) | Some(Task::Break { .. })
        );
        let on_break = matches!(w.staff[idx].task, Some(Task::Break { .. }));
        let er = need_rate(w, content, idx, Need::Energy);
        let hr = need_rate(w, content, idx, Need::Hunger);
        let mr = need_rate(w, content, idx, Need::Mood);
        // continuous trait moods
        let mut mood_add = 0.0;
        for e in effects_of(content, &w.staff[idx]) {
            if let TraitEffect::Mood { add, when } = e
                && !matches!(
                    when,
                    Situation::ShiftStart
                        | Situation::BadEvent
                        | Situation::MetGuest333
                        | Situation::RomanceEnded
                )
                && situation(w, idx, when)
            {
                mood_add += add / 60.0;
            }
        }
        // aura from kind colleagues nearby
        let me_tile = w.staff[idx].tile();
        let me_id = w.staff[idx].id;
        for o in w.staff.iter() {
            if o.id == me_id || !o.on_duty() || grid::chebyshev(o.tile(), me_tile) > 3 {
                continue;
            }
            for e in effects_of(content, o) {
                if let TraitEffect::AuraMood(x) = e {
                    mood_add += x / 60.0;
                }
            }
        }
        let floor = mood_floor(&w.staff[idx], content);
        let s = &mut w.staff[idx];
        s.minutes_worked += 1;
        if !on_break {
            let base = if working { 9.0 } else { 4.5 };
            s.energy = (s.energy - base / 60.0 * decay * er).max(0.0);
            s.hunger = (s.hunger - 9.0 / 60.0 * decay * hr).max(0.0);
        }
        let mut target = 62.0;
        if s.energy < 25.0 {
            target -= 12.0;
        }
        if s.hunger < 25.0 {
            target -= 10.0;
        }
        s.mood += (target - s.mood) * 0.08 / 60.0 * mr + mood_add;
        s.mood = s.mood.clamp(floor, 100.0);
        if s.energy < 15.0 {
            w.tonight.exhausted_minutes += 1;
        }
        if s.energy < 15.0 && w.tick.is_multiple_of(200) {
            s.emote = Some((SimEmote::Zzz, w.tick + 30));
        }
    }
    // relationships (per minute = per-hour / 60)
    let ids: Vec<usize> = (0..n)
        .filter(|&i| w.staff[i].on_duty() && !w.staff[i].owner)
        .collect();
    let rel_mod: f32 = w
        .mods
        .iter()
        .filter(|m| m.kind == ModKind::Relationship)
        .map(|m| m.mul)
        .sum();
    for a in 0..ids.len() {
        for b in a + 1..ids.len() {
            let (ia, ib) = (ids[a], ids[b]);
            let mut per_hour = 0.4 + rel_mod;
            for (x, y) in [(ia, ib), (ib, ia)] {
                for e in effects_of(content, &w.staff[x]) {
                    if let TraitEffect::Relationship { with, per_hour: ph } = e {
                        match with {
                            crate::content::RelTarget::All => per_hour += ph,
                            crate::content::RelTarget::Trait(t) => {
                                if w.staff[y].has_trait(t) {
                                    per_hour += ph;
                                }
                            }
                        }
                    }
                }
                if let Some(def) = w.staff[x]
                    .traits
                    .iter()
                    .filter_map(|t| content.trait_def(t))
                    .find(|d| d.irritated_by.iter().any(|t| w.staff[y].has_trait(t)))
                {
                    let _ = def;
                    per_hour -= 2.0;
                }
            }
            let (ida, idb) = (w.staff[ia].id, w.staff[ib].id);
            add_relationship(w, ida, idb, per_hour / 60.0);
        }
    }
}

pub(crate) fn block_random_tile(w: &mut World, content: &Content) {
    let entrance = w.grid.entrance();
    let seat_tiles: Vec<(i32, i32)> = guests::seats(w, content).iter().map(|s| s.tile).collect();
    let mut queue: Vec<(i32, i32)> = Vec::new();
    for (r, _) in guests::registers(w, content) {
        queue.extend(guests::queue_slots(w, content, r));
    }
    let cands: Vec<(i32, i32)> = (0..w.grid.height)
        .flat_map(|y| (0..w.grid.width).map(move |x| (x, y)))
        .filter(|t| {
            w.grid.room(t.0, t.1) == Room::Hall
                && w.grid.is_indoor_floor(t.0, t.1)
                && w.grid.walkable(t.0, t.1)
                && w.grid.object_at(t.0, t.1).is_none()
                && grid::manhattan(*t, entrance) > 3
                && !seat_tiles.contains(t)
                && !queue.contains(t)
                && w.guests.iter().all(|g| g.tile() != *t)
        })
        .collect();
    let Some(&t) = w.rng.pick(&cands) else { return };
    if let Some(i) = w.grid.idx(t.0, t.1) {
        w.grid.blocked[i] = true;
    }
    w.grid.rebuild(&w.furniture, content);
    // never cut the café in two
    let reach = grid::reachable(&w.grid, entrance);
    let ok = seat_tiles.iter().all(|s| {
        [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)]
            .iter()
            .any(|(dx, dy)| w.grid.idx(s.0 + dx, s.1 + dy).is_some_and(|i| reach[i]))
    });
    if !ok {
        if let Some(i) = w.grid.idx(t.0, t.1) {
            w.grid.blocked[i] = false;
        }
        w.grid.rebuild(&w.furniture, content);
        return;
    }
    let m = w.minute();
    w.buckets.push((t, m + 45, None));
}
