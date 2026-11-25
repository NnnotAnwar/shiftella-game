//! Interpreter for `content::Condition` / `content::Effect` (semantics: `docs/CONTENT.md` §3).

use super::grid::{self, TileKind};
use super::rng::Rng;
use super::types::*;
use super::world::*;
use crate::content::{ClockTime, Condition, Content, CutsceneStep, Effect, GameMode};

// ---------------------------------------------------------------------------------------------
// Conditions
// ---------------------------------------------------------------------------------------------

/// All conditions hold (mutable world: `Chance` consumes the world RNG).
pub(crate) fn all(
    w: &mut World,
    content: &Content,
    conds: &[Condition],
    ctx: &mut EffectCtx,
) -> bool {
    let mut rng = w.rng.clone();
    let ok = all_with(w, content, conds, ctx, &mut rng);
    w.rng = rng;
    ok
}

pub(crate) fn all_with(
    w: &World,
    content: &Content,
    conds: &[Condition],
    ctx: &mut EffectCtx,
    rng: &mut Rng,
) -> bool {
    conds.iter().all(|c| cond(w, content, c, ctx, rng))
}

fn on_duty(w: &World) -> impl Iterator<Item = &Staff> {
    w.staff.iter().filter(|s| s.on_duty())
}

/// The lamp that counts as "the light in the window": a `light` piece next to a window tile.
pub(crate) fn window_light(w: &World, content: &Content) -> Option<u32> {
    let mut best: Option<u32> = None;
    for p in &w.furniture {
        let Some(def) = content.furniture_def(&p.def) else {
            continue;
        };
        if !def.has_tag("light") || def.has_tag("music") {
            continue;
        }
        let near_window = grid::footprint(def, p.x, p.y, p.rot).iter().any(|&(x, y)| {
            (-1..=1).any(|dy| {
                (-1..=1).any(|dx| {
                    w.grid
                        .tile(x + dx, y + dy)
                        .is_some_and(|t| t.kind == TileKind::Window)
                })
            })
        });
        if near_window && best.is_none_or(|b| p.uid < b) {
            best = Some(p.uid);
        }
    }
    best
}

fn regular_present(w: &World, id: &str) -> bool {
    w.story
        .regulars
        .get(id)
        .and_then(|r| r.party)
        .is_some_and(|pid| w.parties.iter().any(|p| p.id == pid))
}

/// Completed visit records (the current, unfinished visit is skipped).
fn finished_visits<'a>(w: &'a World, regular: &str) -> Vec<&'a VisitRecord> {
    let Some(r) = w.story.regulars.get(regular) else {
        return Vec::new();
    };
    let present = regular_present(w, regular);
    let mut v: Vec<&VisitRecord> = r.visits.iter().collect();
    if present
        && let Some(last) = v.last()
        && last.served_minutes.is_none()
    {
        v.pop();
    }
    v
}

pub(crate) fn train_minutes(content: &Content) -> Vec<u32> {
    content
        .trains
        .trains
        .iter()
        .filter_map(|t| {
            let m = t.arrives.minutes_since_open();
            (0..480).contains(&m).then_some(m as u32)
        })
        .collect()
}

pub(crate) fn payments_all_made(w: &World) -> bool {
    w.econ.debt <= 0
        || w.story
            .payments
            .iter()
            .all(|(night, _, paid)| *paid || *night > w.story.payments_processed)
}

pub(crate) fn cond(
    w: &World,
    content: &Content,
    c: &Condition,
    ctx: &mut EffectCtx,
    rng: &mut Rng,
) -> bool {
    use Condition::*;
    let now = w.clock();
    match c {
        NightAtLeast(n) => w.night >= *n,
        NightAtMost(n) => w.night <= *n,
        NightBetween(a, b) => (*a..=*b).contains(&w.night),
        TimeBetween(a, b) => {
            let t = if w.phase == Phase::Day {
                ClockTime::new(21, 59)
            } else {
                now
            };
            t.is_between(*a, *b)
        }
        Weekday(days) => days.contains(&w.weekday()),
        Weather(x) => w.weather == *x,
        Raining => w.weather.is_wet(),
        Season(s) => {
            let cur = match w.town.season {
                0 => crate::content::Season::Spring,
                1 => crate::content::Season::Summer,
                2 => crate::content::Season::Autumn,
                _ => crate::content::Season::Winter,
            };
            cur == *s
        }
        Mode(m) => match m {
            GameMode::Story => w.mode == super::types::Mode::Story,
            GameMode::Sandbox => w.mode == super::types::Mode::Sandbox,
        },
        Difficulty(d) => w.difficulty.id() == d,
        MoneyAtLeast(n) => w.econ.money >= *n,
        MoneyBelow(n) => w.econ.money < *n,
        ReputationAtLeast(r) => w.econ.reputation >= *r,
        ReputationBelow(r) => w.econ.reputation < *r,
        AllPaymentsMade => payments_all_made(w),
        Flag(f) => w.story.flags.contains(f),
        NotFlag(f) => !w.story.flags.contains(f),
        VarIs(v, val) => w.story.vars.get(v).is_some_and(|x| x == val),
        CounterAtLeast(name, n) => counter(w, name) >= *n,
        CounterBelow(name, n) => counter(w, name) < *n,
        NotesFoundAtLeast(n) => w.story.notes.len() as u32 >= *n,
        NoteFound(id) => w.story.notes.iter().any(|x| x == id),
        HasItem(i) => w.story.items.contains(i),
        ArcAtLeast(r, n) => counter(w, &format!("arc.{r}")) >= *n as i32,
        ArcsGoodAtLeast(n) => {
            let good = content
                .regulars
                .iter()
                .filter(|r| w.story.flags.contains(&format!("arc.{}.good", r.id)))
                .count();
            good as u32 >= *n
        }
        RegularPresent(r) => regular_present(w, r),
        ServedFastStreak {
            regular,
            minutes,
            visits,
        } => {
            let v = finished_visits(w, regular);
            v.len() >= *visits as usize
                && v.iter()
                    .rev()
                    .take(*visits as usize)
                    .all(|r| r.served_minutes.is_some_and(|m| m <= *minutes))
        }
        SeatFreeStreak { regular, visits } => {
            let r = w.story.regulars.get(regular);
            let v: Vec<&VisitRecord> = r.map(|r| r.visits.iter().collect()).unwrap_or_default();
            v.len() >= *visits as usize
                && v.iter().rev().take(*visits as usize).all(|r| r.seat_free)
        }
        CleanStreak {
            regular,
            clean,
            visits,
        } => {
            let r = w.story.regulars.get(regular);
            let v: Vec<&VisitRecord> = r.map(|r| r.visits.iter().collect()).unwrap_or_default();
            v.len() >= *visits as usize
                && v.iter()
                    .rev()
                    .take(*visits as usize)
                    .all(|r| r.clean >= *clean)
        }
        ServedItem { regular, item } => w
            .story
            .regulars
            .get(regular)
            .is_some_and(|r| r.served_items.contains(item)),
        TrustAtLeast(r, n) => w.story.trust.get(r).copied().unwrap_or(0.0) >= *n,
        WindowLightOn => w.window_light_on(content),
        TrainNear(n) => {
            w.phase == Phase::Night
                && train_minutes(content)
                    .iter()
                    .any(|t| (*t as i64 - w.minute() as i64).unsigned_abs() <= *n as u64)
        }
        HasFurnitureTag(t) => w
            .furniture
            .iter()
            .any(|p| content.furniture_def(&p.def).is_some_and(|d| d.has_tag(t))),
        FurnitureTagCount(t, n) => {
            w.furniture
                .iter()
                .filter(|p| content.furniture_def(&p.def).is_some_and(|d| d.has_tag(t)))
                .count() as u32
                >= *n
        }
        HasFurniture(id) => w.furniture.iter().any(|p| &p.def == id),
        MenuHas(item) => content
            .menu_item(item)
            .is_some_and(|m| super::guests::item_unlocked(w, m) && !w.econ.menu_off.contains(item)),
        StaffWithTrait(t) => w.staff.iter().any(|s| s.present() && s.has_trait(t)),
        StaffArchetype(a) => w.staff.iter().any(|s| s.present() && &s.archetype == a),
        StaffOnShiftAtLeast(n) => {
            w.staff
                .iter()
                .filter(|s| matches!(s.status, StaffStatus::OnDuty | StaffStatus::Expected { .. }))
                .count() as u32
                >= *n
        }
        StaffMoodBelow(m) => {
            if let Some(s) = on_duty(w)
                .filter(|s| !s.owner && s.mood < *m)
                .min_by(|a, b| a.mood.total_cmp(&b.mood))
            {
                ctx.staff = Some(s.id);
                true
            } else {
                false
            }
        }
        StaffInspired { mood, skill } => {
            if let Some(s) = on_duty(w)
                .find(|s| s.mood > *mood && s.skills.iter().cloned().fold(0.0, f32::max) >= *skill)
            {
                ctx.staff = Some(s.id);
                true
            } else {
                false
            }
        }
        RelationshipBelow(x) => {
            if let Some(p) = find_pair(w, |a, b, r| {
                let _ = (a, b);
                r < *x
            }) {
                ctx.pair = Some(p);
                true
            } else {
                false
            }
        }
        RelationshipAbove(x) => {
            if let Some(p) = find_pair(w, |a, b, r| {
                let t = romance_threshold(w, content, a)
                    .min(romance_threshold(w, content, b))
                    .min(*x);
                r > t
            }) {
                ctx.pair = Some(p);
                true
            } else {
                false
            }
        }
        QueueAtLeast(n) => w.queue_len() as u32 >= *n,
        GuestsAtLeast(n) => w.guests_inside() as u32 >= *n,
        DirtAtLeast(f) => w.dirt >= *f,
        WearAtLeast(tag, f) => w.furniture.iter().any(|p| {
            p.wear >= *f
                && content
                    .furniture_def(&p.def)
                    .is_some_and(|d| d.has_tag(tag))
        }),
        Institution(i) => w.town.institutions.iter().any(|x| x == i),
        EventActive(e) => {
            w.narrator.active.iter().any(|(x, _)| x == e)
                || w.narrator.pending.iter().any(|p| &p.event == e)
        }
        EventHappenedTonight(e) => w.narrator.fired_tonight.iter().any(|x| x == e),
        HardNightsInRow(n) => w.narrator.hard_nights >= *n,
        Chance(p) => rng.chance(*p),
        All(list) => list.iter().all(|c| cond(w, content, c, ctx, rng)),
        Any(list) => list.iter().any(|c| cond(w, content, c, ctx, rng)),
        Not(c) => !cond(w, content, c, &mut ctx.clone(), rng),
    }
}

fn counter(w: &World, name: &str) -> i32 {
    w.story.counters.get(name).copied().unwrap_or(0)
}

pub(crate) fn romance_threshold(w: &World, content: &Content, staff: u32) -> f32 {
    let mut t = 80.0f32;
    if let Some(s) = w.staff_by_id(staff) {
        for tr in &s.traits {
            if let Some(def) = content.trait_def(tr) {
                for e in &def.effects {
                    if let crate::content::TraitEffect::RomanceThreshold(x) = e {
                        t = t.min(*x);
                    }
                }
            }
        }
    }
    t
}

fn find_pair(w: &World, mut pred: impl FnMut(u32, u32, f32) -> bool) -> Option<(u32, u32)> {
    let ids: Vec<u32> = on_duty(w).filter(|s| !s.owner).map(|s| s.id).collect();
    for i in 0..ids.len() {
        for j in i + 1..ids.len() {
            let r = w.relationship(ids[i], ids[j]);
            if pred(ids[i], ids[j], r) {
                return Some((ids[i], ids[j]));
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------------------------
// Effects
// ---------------------------------------------------------------------------------------------

/// Root effect list of a source.
fn root<'a>(content: &'a Content, src: &EffectSrc) -> Option<&'a [Effect]> {
    match src {
        EffectSrc::Event(id) => content.event(id).map(|e| e.effects.as_slice()),
        EffectSrc::EventChoice(id, c) => content
            .event(id)
            .and_then(|e| e.choices.iter().find(|x| &x.id == c))
            .map(|c| c.effects.as_slice()),
        EffectSrc::Checklist(id, done) => {
            content
                .app
                .checklists
                .iter()
                .find(|x| &x.id == id)
                .map(|c| {
                    if *done {
                        c.done.as_slice()
                    } else {
                        c.missed.as_slice()
                    }
                })
        }
        EffectSrc::Beat(night, idx) => content.story.night(*night).and_then(|n| {
            n.beats.get(*idx).and_then(|b| match &b.what {
                crate::content::BeatKind::Apply(list) => Some(list.as_slice()),
                _ => None,
            })
        }),
        EffectSrc::Note(id) => content.story.note(id).map(|n| n.effects.as_slice()),
        EffectSrc::Arc(r, stage) => content
            .regular(r)
            .and_then(|r| r.stage(*stage))
            .map(|s| s.effects.as_slice()),
        EffectSrc::MissedPayment(d) => content
            .difficulty(d)
            .map(|d| d.missed_payment_effects.as_slice()),
        EffectSrc::BotAction(t, a) => content
            .app
            .bot_template(t)
            .and_then(|b| b.actions.iter().find(|x| &x.id == a))
            .map(|a| a.effects.as_slice()),
        EffectSrc::Cutscene(id, steps, opt) => {
            let cs = content.cutscene(id)?;
            let mut list: &[CutsceneStep] = &cs.steps;
            let mut step: Option<&CutsceneStep> = None;
            for (k, (i, b)) in steps.iter().enumerate() {
                let s = list.get(*i as usize)?;
                step = Some(s);
                if k + 1 < steps.len() {
                    match s {
                        CutsceneStep::If {
                            then, otherwise, ..
                        } => list = if *b == 0 { then } else { otherwise },
                        _ => return None,
                    }
                }
            }
            match (step?, opt) {
                (CutsceneStep::Apply { effects }, _) => Some(effects.as_slice()),
                (CutsceneStep::Choice { options }, Some(o)) => {
                    options.get(*o as usize).map(|o| o.effects.as_slice())
                }
                _ => None,
            }
        }
        EffectSrc::None => None,
    }
}

fn resolve<'a>(content: &'a Content, src: &EffectSrc, path: &[(u16, u8)]) -> Option<&'a [Effect]> {
    let mut list = root(content, src)?;
    for (i, b) in path {
        list = match list.get(*i as usize)? {
            Effect::Maybe { effects, .. } | Effect::Later { effects, .. } => effects.as_slice(),
            Effect::If {
                then, otherwise, ..
            } => {
                if *b == 0 {
                    then.as_slice()
                } else {
                    otherwise.as_slice()
                }
            }
            _ => return None,
        };
    }
    Some(list)
}

/// Applies an effect list.
pub(crate) fn run(
    w: &mut World,
    content: &Content,
    src: &EffectSrc,
    effects: &[Effect],
    ctx: &mut EffectCtx,
) {
    run_at(w, content, src, &[], effects, ctx, 0);
}

/// Runs due `Later` effects.
pub(crate) fn run_later(w: &mut World, content: &Content) {
    let m = w.minute();
    let mut due = Vec::new();
    w.narrator.later.retain(|l| {
        if l.due <= m {
            due.push(l.clone());
            false
        } else {
            true
        }
    });
    for l in due {
        if let Some(list) = resolve(content, &l.src, &l.path) {
            let mut ctx = l.ctx.clone();
            run_at(w, content, &l.src, &l.path, list, &mut ctx, 0);
        }
    }
}

fn run_at(
    w: &mut World,
    content: &Content,
    src: &EffectSrc,
    base: &[(u16, u8)],
    effects: &[Effect],
    ctx: &mut EffectCtx,
    depth: u32,
) {
    if depth > 8 {
        return;
    }
    for (i, e) in effects.iter().enumerate() {
        let path = |b: u8| -> Vec<(u16, u8)> {
            let mut p = base.to_vec();
            p.push((i as u16, b));
            p
        };
        match e {
            Effect::Maybe { chance, effects } => {
                if w.rng.chance(*chance) {
                    run_at(w, content, src, &path(0), effects, ctx, depth + 1);
                }
            }
            Effect::Later { minutes, effects } => {
                if *src == EffectSrc::None || w.phase == Phase::Day {
                    run_at(w, content, src, &path(0), effects, ctx, depth + 1);
                } else {
                    let due = w.minute() + *minutes;
                    w.narrator.later.push(PendingLater {
                        due,
                        src: src.clone(),
                        path: path(0),
                        ctx: ctx.clone(),
                    });
                }
            }
            Effect::If {
                conditions,
                then,
                otherwise,
            } => {
                if all(w, content, conditions, ctx) {
                    run_at(w, content, src, &path(0), then, ctx, depth + 1);
                } else {
                    run_at(w, content, src, &path(1), otherwise, ctx, depth + 1);
                }
            }
            Effect::StartEvent(id) => {
                if depth < 4 {
                    super::narrator::fire(w, content, id, ctx.clone(), true);
                }
            }
            other => apply_one(w, content, other, ctx),
        }
    }
}

fn mod_until(w: &World, minutes: &Option<u32>) -> Option<u32> {
    minutes.map(|m| w.minute() + m)
}

pub(crate) fn add_money(w: &mut World, amount: i64) {
    w.econ.money += amount;
    if amount >= 0 {
        w.tonight.other_income += amount;
    } else {
        w.tonight.other_expenses += -amount;
    }
}

pub(crate) fn change_mood(w: &mut World, content: &Content, staff_idx: usize, add: f32, bad: bool) {
    let mut add = add;
    if bad && add < 0.0 {
        for t in w.staff[staff_idx].traits.clone() {
            if let Some(def) = content.trait_def(&t) {
                for e in &def.effects {
                    if let crate::content::TraitEffect::BadEventMoodMul(m) = e {
                        add *= m;
                    }
                }
            }
        }
    }
    let floor = super::staff::mood_floor(&w.staff[staff_idx], content);
    let s = &mut w.staff[staff_idx];
    s.mood = (s.mood + add).clamp(floor, 100.0);
}

fn apply_one(w: &mut World, content: &Content, e: &Effect, ctx: &mut EffectCtx) {
    let m = w.minute();
    let bad_event = ctx
        .event
        .as_ref()
        .and_then(|id| content.event(id))
        .is_some_and(|e| e.kind == crate::content::EventKind::Bad);
    match e {
        Effect::Money(x) => add_money(w, *x),
        Effect::MoneyRange(a, b) => {
            let x = w.rng.range_i64(*a, *b);
            add_money(w, x);
        }
        Effect::Debt(x) => {
            w.econ.debt = (w.econ.debt + x).max(0);
            if w.econ.debt == 0 {
                for p in w.story.payments.iter_mut() {
                    p.2 = true;
                }
            }
        }
        Effect::Reputation(x) => {
            w.econ.reputation = (w.econ.reputation + x).clamp(0.0, 100.0);
        }
        Effect::ReviewWeight {
            guest,
            mult,
            reviews,
        } => w
            .narrator
            .review_weights
            .push((guest.clone(), *mult, *reviews)),
        Effect::GuestRate { mul, minutes } => w.mods.push(Modifier {
            kind: ModKind::GuestRate,
            mul: *mul,
            until: mod_until(w, minutes),
            event: ctx.event.clone(),
        }),
        Effect::GuestRush {
            guest,
            count,
            within,
            patience_mul,
            spend_mul,
            orders,
        } => {
            let n = w.rng.range_u32(count.0, count.1);
            super::guests::schedule_rush(
                w,
                content,
                guest,
                n,
                *within,
                *patience_mul,
                *spend_mul,
                orders,
            );
        }
        Effect::GuestStay(x) => {
            w.guest_stay_add += x;
            for p in w.parties.iter_mut() {
                if p.state == PartyState::Eating {
                    p.stay_left = (p.stay_left + *x as f32).max(1.0);
                }
            }
        }
        Effect::GuestSatisfaction(x) => {
            for p in w.parties.iter_mut() {
                if p.state != PartyState::Leaving {
                    p.satisfaction = (p.satisfaction + x).clamp(0.0, 100.0);
                }
            }
        }
        Effect::Tips { mul, minutes } => w.mods.push(Modifier {
            kind: ModKind::Tips,
            mul: *mul,
            until: mod_until(w, minutes),
            event: ctx.event.clone(),
        }),
        Effect::SpawnRegular(r) => super::guests::spawn_regular(w, content, r, None),
        Effect::SpawnGuest(g) => {
            super::guests::schedule_rush(w, content, g, 1, 0, 1.0, 1.0, &[]);
        }
        Effect::StaffMood(x) => {
            for i in 0..w.staff.len() {
                if w.staff[i].on_duty() {
                    change_mood(w, content, i, *x, bad_event);
                }
            }
        }
        Effect::TraitMood(t, x) => {
            for i in 0..w.staff.len() {
                if w.staff[i].on_duty() && w.staff[i].has_trait(t) {
                    change_mood(w, content, i, *x, bad_event);
                }
            }
        }
        Effect::StaffSick => {
            let ids: Vec<u32> = w
                .staff
                .iter()
                .filter(|s| {
                    !s.owner
                        && matches!(s.status, StaffStatus::OnDuty | StaffStatus::Expected { .. })
                })
                .map(|s| s.id)
                .collect();
            if let Some(&id) = w.rng.pick(&ids) {
                ctx.staff = Some(id);
                super::staff::send_home(w, id, StaffStatus::Sick);
                w.emit(SimEvent::StaffSick { staff: id });
            }
        }
        Effect::StaffWalkout => {
            let pick = w
                .staff
                .iter()
                .filter(|s| s.on_duty() && !s.owner)
                .min_by(|a, b| a.mood.total_cmp(&b.mood))
                .map(|s| s.id);
            if let Some(id) = pick {
                ctx.staff = Some(id);
                super::staff::send_home(w, id, StaffStatus::WalkedOut);
                w.emit(SimEvent::StaffWalkedOut { staff: id });
            }
        }
        Effect::PairMood(x) => {
            if let Some((a, b)) = ctx.pair {
                for id in [a, b] {
                    if let Some(i) = w.staff.iter().position(|s| s.id == id) {
                        change_mood(w, content, i, *x, bad_event);
                    }
                }
            }
        }
        Effect::Relationship { add, minutes } => match minutes {
            None => {
                let ids: Vec<u32> = on_duty(w).map(|s| s.id).collect();
                for i in 0..ids.len() {
                    for j in i + 1..ids.len() {
                        super::staff::add_relationship(w, ids[i], ids[j], *add);
                    }
                }
            }
            Some(_) => w.mods.push(Modifier {
                kind: ModKind::Relationship,
                mul: *add,
                until: mod_until(w, minutes),
                event: ctx.event.clone(),
            }),
        },
        Effect::WorkSpeed { job, mul, minutes } => w.mods.push(Modifier {
            kind: ModKind::WorkSpeed(job.map(|j| j.index() as u8)),
            mul: *mul,
            until: mod_until(w, minutes),
            event: ctx.event.clone(),
        }),
        Effect::SkillBoost { add, nights } => {
            let target = ctx.staff.or_else(|| {
                on_duty(w)
                    .max_by(|a, b| a.mood.total_cmp(&b.mood))
                    .map(|s| s.id)
            });
            let night = w.night;
            if let Some(s) = target.and_then(|id| w.staff.iter_mut().find(|s| s.id == id)) {
                let best = (0..5)
                    .max_by(|a, b| s.skills[*a].total_cmp(&s.skills[*b]))
                    .unwrap_or(0);
                s.skill_boost = Some((best as u8, *add, night + nights.saturating_sub(1)));
            }
        }
        Effect::Dirt(x) => w.dirt = (w.dirt + x).clamp(0.0, 1.0),
        Effect::DirtRate { mul, minutes } => w.mods.push(Modifier {
            kind: ModKind::DirtRate,
            mul: *mul,
            until: mod_until(w, minutes),
            event: ctx.event.clone(),
        }),
        Effect::PowerOutage { minutes } => {
            if w.phase == Phase::Night {
                let len = w.rng.range_u32(minutes.0, minutes.1);
                let until = m + len;
                w.blackout_until = Some(w.blackout_until.map_or(until, |u| u.max(until)));
                w.mods.push(Modifier {
                    kind: ModKind::PowerOut,
                    mul: 0.0,
                    until: Some(until),
                    event: ctx.event.clone(),
                });
                w.emit(SimEvent::Blackout { on: true });
            }
        }
        Effect::StationDown { tag, minutes } => w.mods.push(Modifier {
            kind: ModKind::StationDown(tag.clone()),
            mul: 0.0,
            until: mod_until(w, minutes),
            event: ctx.event.clone(),
        }),
        Effect::StationSlow { tag, mul } => w.mods.push(Modifier {
            kind: ModKind::StationSlow(tag.clone()),
            mul: *mul,
            until: None,
            event: ctx.event.clone(),
        }),
        Effect::MenuUnavailable { item, until } => {
            let until = until.map(|t| t.minutes_since_open().max(0) as u32);
            w.mods.push(Modifier {
                kind: ModKind::MenuUnavailable(item.clone()),
                mul: 0.0,
                until,
                event: ctx.event.clone(),
            });
        }
        Effect::LoseStock(f) => {
            w.econ.supplies = (w.econ.supplies as f32 * (1.0 - f.clamp(0.0, 1.0))) as i64;
        }
        Effect::BlockTile => super::staff::block_random_tile(w, content),
        Effect::ClockStop { seconds } => w.emit(SimEvent::ClockStop { seconds: *seconds }),
        Effect::SetWeather(x) => {
            if w.weather != *x {
                w.weather = *x;
                w.emit(SimEvent::WeatherChanged { weather: *x });
            }
        }
        Effect::SetFlag(f) => set_flag(w, content, f),
        Effect::ClearFlag(f) => {
            w.story.flags.remove(f);
        }
        Effect::SetVar(v, val) => {
            w.story.vars.insert(v.clone(), val.clone());
        }
        Effect::AddCounter(c, n) => *w.story.counters.entry(c.clone()).or_insert(0) += n,
        Effect::GiveItem(i) => {
            if w.story.items.insert(i.clone()) {
                w.emit(SimEvent::ItemGained(i.clone()));
            }
        }
        Effect::TakeItem(i) => {
            if w.story.items.remove(i) {
                w.emit(SimEvent::ItemLost(i.clone()));
            }
        }
        Effect::FindNote(n) => super::story::find_note(w, content, n),
        Effect::AdvanceArc(r) => super::story::advance_arc(w, content, r),
        Effect::Trust(r, x) => {
            let t = w.story.trust.entry(r.clone()).or_insert(0.0);
            *t = (*t + x).clamp(0.0, 100.0);
        }
        Effect::ArcGood(r) => set_flag(w, content, &format!("arc.{r}.good")),
        Effect::ArcBad(r) => set_flag(w, content, &format!("arc.{r}.bad")),
        Effect::UnlockMenu(i) => {
            if w.story.unlocked_menu.insert(i.clone()) {
                w.emit(SimEvent::MenuUnlocked(i.clone()));
            }
        }
        Effect::UnlockFurniture(i) => {
            if w.story.unlocked_furniture.insert(i.clone()) {
                w.emit(SimEvent::FurnitureUnlocked(i.clone()));
            }
        }
        Effect::EventWeight { event, mul, nights } => {
            let until = w.night + nights.saturating_sub(1);
            w.narrator.weights.push((event.clone(), *mul, until));
        }
        Effect::StartCutscene(c) => super::story::queue_cutscene(w, content, c),
        Effect::Toast(k) => w.emit(SimEvent::Toast { key: k.clone() }),
        Effect::Bot(t) => super::app::bot(w, content, t, ctx),
        Effect::Maybe { .. } | Effect::Later { .. } | Effect::If { .. } | Effect::StartEvent(_) => {
        }
    }
}

pub(crate) fn set_flag(w: &mut World, content: &Content, f: &str) {
    if w.story.flags.insert(f.to_string()) {
        w.emit(SimEvent::FlagSet(f.to_string()));
        super::story::on_flag_set(w, content, f);
    }
}
