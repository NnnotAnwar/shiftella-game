//! Interpreter for the hands-on scripting extension (`docs/CONTENT.md` §10.0):
//! conditions `Cond = Core(Condition) | X(XCondition)` and effects `Fx = Core(Effect) |
//! X(XEffect)`. Core conditions go to `logic::cond`, core effects to `logic::run` (they never
//! carry a `Later` / `Maybe` / `If` of their own: in `Fx` lists those are the extension's).
//! "Target" effects act on the [`XTarget`] (the agent a moment is anchored to, the subject
//! of a handover note or an announcement).

use super::logic;
use super::rng::Rng;
use super::systems::{PendingFx, XTarget};
use super::types::*;
use super::world::*;
use crate::content::handson::{Cond, Fx, XCondition, XEffect};
use crate::content::{Content, Effect};

// ---------------------------------------------------------------------------------------------
// Conditions
// ---------------------------------------------------------------------------------------------

/// All hold, with an explicit RNG (read-only world).
pub(crate) fn all_with(
    w: &World,
    content: &Content,
    conds: &[Cond],
    ctx: &mut EffectCtx,
    rng: &mut Rng,
) -> bool {
    conds.iter().all(|c| cond(w, content, c, ctx, rng))
}

/// Read-only check that never consumes the world RNG (a hash roll for `Chance`).
pub(crate) fn holds(w: &World, content: &Content, conds: &[Cond]) -> bool {
    let mut rng = Rng::new((w.tick as u64) ^ ((w.night as u64) << 32) ^ 0x5EED);
    all_with(w, content, conds, &mut EffectCtx::default(), &mut rng)
}

/// All hold; `Chance` consumes the world RNG (effects' `If`).
pub(crate) fn all_mut(w: &mut World, content: &Content, conds: &[Cond]) -> bool {
    let mut rng = w.rng.clone();
    let ok = all_with(w, content, conds, &mut EffectCtx::default(), &mut rng);
    w.rng = rng;
    ok
}

pub(crate) fn cond(
    w: &World,
    content: &Content,
    c: &Cond,
    ctx: &mut EffectCtx,
    rng: &mut Rng,
) -> bool {
    use XCondition::*;
    let x = match c {
        Cond::Core(core) => return logic::cond(w, content, core, ctx, rng),
        Cond::X(x) => x,
    };
    match x {
        All(list) => list.iter().all(|c| cond(w, content, c, ctx, rng)),
        Any(list) => list.iter().any(|c| cond(w, content, c, ctx, rng)),
        Not(c) => !cond(w, content, c, &mut ctx.clone(), rng),
        RecipeKnown(r) => w.hands.knowledge.recipes.contains_key(r),
        ServedRecipe { regular, recipe } => w
            .story
            .regulars
            .get(regular)
            .is_some_and(|r| r.served_recipes.contains(recipe)),
        WishMatchStreak {
            regular,
            min,
            visits,
        } => {
            let Some(r) = w.story.regulars.get(regular) else {
                return false;
            };
            // completed visits only (the current one may still get its coffee)
            let present = r
                .party
                .is_some_and(|pid| w.parties.iter().any(|p| p.id == pid));
            let mut v: Vec<&VisitRecord> = r.visits.iter().collect();
            if present && v.last().is_some_and(|l| l.best_match.is_none()) {
                v.pop();
            }
            v.len() >= *visits as usize
                && v.iter()
                    .rev()
                    .take(*visits as usize)
                    .all(|r| r.best_match.is_some_and(|m| m >= *min))
        }
        HallMood(m) => w.hall_mood(content) == Some(*m),
        QuietHour => super::pacing::quiet_hour(w, content),
        Blackout => w.blackout_until.is_some(),
        StaffLowEnergy(e) => w
            .staff
            .iter()
            .any(|s| !s.owner && s.on_duty() && s.energy < *e),
        MomentResolvedEver(m) => w.sys.moments.resolved_ever.contains(m),
        IngredientUnlocked(i) => w.hands.knowledge.ingredients.contains(i),
        RecordUnlocked(r) => w.hands.music.unlocked.contains(r),
        SeasonPhase(p) => super::pacing::season_phase(w, content).is_some_and(|x| x == p),
        CozinessAtLeast(n) => w.sys.atmos.coziness.value >= *n,
        HiredArchetype(a) => w.staff.iter().any(|s| !s.owner && &s.archetype == a),
        ShiftsWorked(a, n) => w
            .staff
            .iter()
            .any(|s| !s.owner && &s.archetype == a && s.nights_worked >= *n),
        SharedShifts { a, b, shifts } => super::rozpis::shared_shifts(w, a, b) >= *shifts,
        TipsLifetimeAtLeast(czk) => w.hands.tips_lifetime >= *czk as i64,
        NapkinsAtLeast(n) => w.sys.extras.napkins.len() as u32 >= *n,
        ThemeOwned(t) => super::atmos::theme_owned(w, content, t),
    }
}

// ---------------------------------------------------------------------------------------------
// Effects
// ---------------------------------------------------------------------------------------------

/// Runs a hands-on effect list now.
pub(crate) fn run(w: &mut World, content: &Content, effects: &[Fx], target: &XTarget) {
    run_depth(w, content, effects, target, 0);
}

fn run_depth(w: &mut World, content: &Content, effects: &[Fx], target: &XTarget, depth: u32) {
    if depth > 8 {
        return;
    }
    for e in effects {
        match e {
            Fx::Core(core) => run_core(w, content, core, target),
            Fx::X(x) => apply_x(w, content, x, target, depth),
        }
    }
}

/// A core effect inside an `Fx` list (`StaffMood`, `Money`, `SetFlag`, …).
fn run_core(w: &mut World, content: &Content, e: &Effect, target: &XTarget) {
    let mut ctx = EffectCtx {
        staff: target.staff,
        ..Default::default()
    };
    logic::run(
        w,
        content,
        &EffectSrc::None,
        std::slice::from_ref(e),
        &mut ctx,
    );
    // effects that change what the sim tracks right away
    if let Effect::AddCounter(c, _) = e
        && c == &content.handson.extras.cat.counter
    {
        super::moments::check_cat(w, content);
    }
}

/// Due `Later` lists (each game minute).
pub(crate) fn run_later(w: &mut World, content: &Content) {
    let m = w.minute();
    let mut due = Vec::new();
    w.sys.fx.later.retain(|l| {
        if l.due <= m {
            due.push(l.clone());
            false
        } else {
            true
        }
    });
    for l in due {
        run(w, content, &l.effects, &l.target);
    }
}

/// Night start: last night's `NextNight` lists.
pub(crate) fn run_next_night(w: &mut World, content: &Content) {
    let list = std::mem::take(&mut w.sys.fx.next_night);
    for l in list {
        run(w, content, &l.effects, &l.target);
    }
}

fn until(w: &World, minutes: &Option<u32>) -> Option<u32> {
    minutes.map(|m| w.minute() + m)
}

fn target_party(w: &World, t: &XTarget) -> Option<usize> {
    t.party
        .and_then(|p| w.parties.iter().position(|x| x.id == p))
}

fn target_staff(w: &World, t: &XTarget) -> Option<usize> {
    t.staff.and_then(|s| w.staff.iter().position(|x| x.id == s))
}

fn target_regular(w: &World, t: &XTarget) -> Option<String> {
    t.regular
        .clone()
        .or_else(|| target_party(w, t).and_then(|pi| w.parties[pi].regular.clone()))
}

fn apply_x(w: &mut World, content: &Content, x: &XEffect, t: &XTarget, depth: u32) {
    use XEffect::*;
    match x {
        Maybe { chance, effects } => {
            if w.rng.chance(*chance) {
                run_depth(w, content, effects, t, depth + 1);
            }
        }
        Later { minutes, effects } => {
            if w.phase == Phase::Night {
                let due = w.minute() + *minutes;
                w.sys.fx.later.push(PendingFx {
                    due,
                    effects: effects.clone(),
                    target: t.clone(),
                });
            } else {
                run_depth(w, content, effects, t, depth + 1);
            }
        }
        If {
            conditions,
            then,
            otherwise,
        } => {
            if all_mut(w, content, conditions) {
                run_depth(w, content, then, t, depth + 1);
            } else {
                run_depth(w, content, otherwise, t, depth + 1);
            }
        }
        NextNight(effects) => w.sys.fx.next_night.push(PendingFx {
            due: 0,
            effects: effects.clone(),
            target: t.clone(),
        }),
        LearnRecipe(r) => super::orders::learn(w, r, "effect".into()),
        UnlockIngredient(i) => super::knowledge::unlock_ingredient(w, content, i),
        RevealIngredient(r) => {
            super::knowledge::reveal(w, content, r);
        }
        UnlockRecord(r) => {
            if content.handson.record(r).is_some() && w.hands.music.unlocked.insert(r.clone()) {
                w.emit(SimEvent::RecordUnlocked { id: r.clone() });
            }
        }
        UnlockTheme(id) => super::atmos::give_theme(w, content, id),
        UnlockLatte(id) => super::atmos::unlock_latte(w, id),
        HallMood { mood, minutes } => {
            let before = w.hall_mood(content);
            w.hands.hall_mood.mood = Some(*mood);
            w.hands.hall_mood.until = until(w, minutes);
            if before != Some(*mood) {
                w.emit(SimEvent::HallMoodChanged { mood: Some(*mood) });
            }
        }
        PlayRequestedRecord => {
            let rec = t.record.clone().or_else(|| {
                t.regular
                    .as_deref()
                    .and_then(|r| super::moments::wanted_record(w, content, r))
            });
            if let Some(rec) = rec
                && super::music::validate(w, &Some(rec.clone())).is_ok()
            {
                super::music::set_record(w, content, Some(rec), true);
                w.hands.music.requested_by = target_regular(w, t);
            }
        }
        RadioTrack {
            track,
            volume,
            minutes,
        } => {
            let until = w.minute() + *minutes;
            w.sys.fx.radio = Some((track.clone(), *volume, until));
            w.emit(SimEvent::RadioTrack {
                track: track.clone(),
                volume: *volume,
                minutes: *minutes,
            });
        }
        StreetMusic(minutes) => {
            w.sys.fx.street_music_until = Some(w.minute() + *minutes);
            w.emit(SimEvent::StreetMusic { minutes: *minutes });
        }
        SpawnMoment(id) => {
            super::moments::spawn(w, content, id, t.clone(), true);
        }
        CameraHint(target) => {
            w.sys.tool.last_camera = Some(*target);
            w.emit(SimEvent::CameraHint {
                target: *target,
                tile: t.tile,
            });
        }
        LogTrain => {
            let m = w.minute() as i64;
            let train = content
                .trains
                .trains
                .iter()
                .find(|tr| {
                    let at = tr.arrives.minutes_since_open() as i64;
                    at >= 0 && (at - m).abs() <= 5 && !tr.id.contains("333")
                })
                .map(|tr| tr.id.clone());
            if let Some(train) = train {
                let night = w.night;
                w.sys.fx.logbook.push((night, train.clone()));
                w.emit(SimEvent::TrainLogged { train });
            }
        }
        Highlight(kind) => {
            let sub = t.moment.clone();
            super::journal::capture(w, content, *kind, sub.as_deref(), Vec::new(), t.tile);
        }
        ThankYouNote(id) => {
            let regular = target_regular(w, t);
            super::moments::give_napkin(w, content, id.as_deref(), regular.as_deref());
        }
        TargetTrust(x) => {
            if let Some(r) = target_regular(w, t) {
                let v = w.story.trust.entry(r).or_insert(0.0);
                *v = (*v + x).clamp(0.0, 100.0);
            }
        }
        TargetSatisfaction(x) => {
            if let Some(pi) = target_party(w, t) {
                let p = &mut w.parties[pi];
                p.satisfaction = (p.satisfaction + x).clamp(0.0, 100.0);
            }
        }
        TargetStay(min) => {
            if let Some(pi) = target_party(w, t) {
                let p = &mut w.parties[pi];
                p.stay_left = (p.stay_left + *min as f32).max(1.0);
                if let Some(lb) = p.leave_by.as_mut() {
                    *lb = (*lb as i64 + *min as i64).clamp(0, NIGHT_MINUTES as i64) as u32;
                }
            }
        }
        TargetLeave => {
            if let Some(pi) = target_party(w, t) {
                let pid = w.parties[pi].id;
                if w.parties[pi].state == PartyState::Eating {
                    w.parties[pi].stay_left = 0.5;
                } else {
                    super::guests::finish_party(w, content, pid, super::guests::Exit::Normal);
                }
            }
        }
        TargetStaffMood(x) => {
            if let Some(i) = target_staff(w, t) {
                logic::change_mood(w, content, i, *x, false);
            }
        }
        TargetStaffEnergy(x) => {
            if let Some(i) = target_staff(w, t) {
                let s = &mut w.staff[i];
                s.energy = (s.energy + x).clamp(0.0, 100.0);
            }
        }
        TargetStaffBreak => {
            if let Some(i) = target_staff(w, t)
                && w.phase == Phase::Night
                && w.staff[i].on_duty()
                && !w.staff[i].owner
                && !matches!(w.staff[i].task, Some(Task::Break { .. }))
            {
                let task = super::staff::break_task(w, content, i, super::staff::BREAK_MINUTES);
                super::staff::start_break(w, i, task);
            }
        }
        TargetRelationship(x) => {
            if let (Some(s), Some(me)) = (t.staff, super::pacing::owner_id(w)) {
                super::staff::add_relationship(w, s, me, *x);
            }
        }
        ArchetypeMood { staff, add } => {
            for i in 0..w.staff.len() {
                if !w.staff[i].owner && &w.staff[i].archetype == staff {
                    logic::change_mood(w, content, i, *add, false);
                }
            }
        }
        GuestSatisfactionNear { add, tiles } => {
            let Some(at) = t.tile else { return };
            let near: Vec<u32> = w
                .guests
                .iter()
                .filter(|g| !g.gone && super::grid::chebyshev(g.tile(), at) <= *tiles as i32)
                .map(|g| g.party)
                .collect();
            for p in w.parties.iter_mut() {
                if near.contains(&p.id) && p.state != PartyState::Leaving {
                    p.satisfaction = (p.satisfaction + add).clamp(0.0, 100.0);
                }
            }
        }
        WishBoost { tags, mul, minutes } => {
            let u = until(w, minutes);
            w.sys.fx.wish_boosts.push((tags.clone(), *mul, u));
        }
        Coziness { add, minutes } => {
            let u = until(w, minutes);
            w.sys.fx.coziness.push((*add, u));
            super::atmos::recompute(w, content);
        }
        Patience { mul, minutes } => {
            let u = until(w, minutes);
            w.sys.fx.patience.push((*mul, u));
        }
        LightCandles => {
            w.sys.fx.candles = true;
            super::atmos::recompute(w, content);
        }
        PlaceBucket => {
            let tile = t.tile.or_else(|| w.buckets.last().map(|b| b.0));
            let Some(tile) = tile else { return };
            let m = w.minute();
            if let Some(b) = w.buckets.iter_mut().find(|b| b.0 == tile) {
                // Karolina's bucket stays until it is emptied (m_leak_empty)
                b.1 = b.1.max(m + 600);
            } else if w.grid.walkable(tile.0, tile.1)
                && w.grid.object_at(tile.0, tile.1).is_none()
                // not under someone standing there or walking to it: they'd be inside it
                && !super::staff::people_at(w, tile)
            {
                if let Some(i) = w.grid.idx(tile.0, tile.1) {
                    w.grid.blocked[i] = true;
                }
                w.grid.rebuild(&w.furniture, content);
                w.buckets.push((tile, m + 600, None));
            }
        }
        EmptyBucket => {
            let tiles: Vec<(i32, i32)> = match t.tile {
                Some(tt) if w.buckets.iter().any(|b| b.0 == tt) => vec![tt],
                _ => w.buckets.iter().map(|b| b.0).collect(),
            };
            for tile in tiles {
                w.buckets.retain(|b| b.0 != tile);
                if let Some(i) = w.grid.idx(tile.0, tile.1) {
                    w.grid.blocked[i] = false;
                }
            }
            w.grid.rebuild(&w.furniture, content);
        }
        WindowOpen(minutes) => {
            w.sys.fx.window_open_until = Some(w.minute() + *minutes);
            w.emit(SimEvent::WindowOpened { minutes: *minutes });
        }
        Tip { min, max } => {
            let amount = w.rng.range_u32(*min, (*max).max(*min)) as i64;
            if amount > 0 {
                let id = w.alloc_id();
                let coin = super::hands::TipCoin {
                    id,
                    table: 0,
                    amount,
                    staff: None,
                    minute: w.minute(),
                    napkin: None,
                };
                let me = super::pacing::owner_id(w);
                super::player::collect_coins(w, content, vec![coin], me);
            }
        }
        QuickQuality { add, minutes } => {
            let u = until(w, minutes);
            w.sys.fx.quick_quality.push((*add, u));
        }
        SlowFirstItems { job, count, mul } => w.sys.fx.slow_first.push((*job, *count, *mul)),
        StockoutRisk { chance, minutes } => {
            if !w.sys.fx.stock_checked && w.rng.chance(*chance) {
                let at = w.minute() + w.rng.range_u32(60, 300);
                w.sys.fx.stockout = Some((at, *minutes));
            }
        }
        StockChecked => {
            w.sys.fx.stock_checked = true;
            w.sys.fx.stockout = None;
            if w.phase == Phase::Night
                && w.app
                    .checklist
                    .iter()
                    .any(|c| c.id == "check_stock" && !c.done)
            {
                super::app::tick_checklist(w, content, "check_stock", None);
            }
        }
        RestockFirst => super::tool::restock_first(w, content, t.item.as_deref()),
        Wear { tag, add } => {
            for p in w.furniture.iter_mut() {
                if content
                    .furniture_def(&p.def)
                    .is_some_and(|d| d.has_tag(tag))
                {
                    p.wear = (p.wear + add).clamp(0.0, 1.0);
                }
            }
        }
        CashUp => w.sys.fx.cashed_up = true,
        ChecklistSpeed { list, mul } => {
            let i = match list {
                crate::content::world::ChecklistKind::Opening => 0,
                crate::content::world::ChecklistKind::Closing => 1,
            };
            // (they add up: the closing ritual's quicker opening and a handover by hand)
            let cur = w.sys.fx.checklist_speed[i];
            w.sys.fx.checklist_speed[i] = if cur > 0.0 { cur * mul } else { *mul };
        }
        HelpersReady { minutes } => {
            if let Some(tm) = t.train_minute {
                w.sys.fx.helpers_ready.push(tm.saturating_sub(*minutes));
            }
        }
        RememberRegular { trust } => {
            if let Some(r) = target_regular(w, t) {
                w.sys.fx.remember.insert(r, *trust);
            }
        }
        FixTable { beauty } => {
            if let Some(uid) = t.furniture
                && let Some(p) = w.furniture.iter_mut().find(|p| p.uid == uid)
            {
                p.wear = 0.0;
            }
            w.sys.fx.table_beauty += beauty;
            super::atmos::recompute(w, content);
        }
        ItemSpotlight { prep_mul } => {
            if let Some(item) = t.item.clone() {
                w.sys.fx.spotlight = Some((item, *prep_mul));
            }
        }
        SwapBoard => w.sys.fx.swap_board = true,
        StockWatch => {
            if w.sys.fx.stock_watch {
                return;
            }
            w.sys.fx.stock_watch = true;
            let ctx = EffectCtx::default();
            let n = (w.econ.supplies / 25).max(0).to_string();
            super::app::bot_with(w, content, "stock_check", &ctx, vec![("n".into(), n)]);
        }
        SpecialsBoost { mul, minutes } => {
            let mut tags: Vec<crate::content::handson::Taste> = Vec::new();
            for r in &w.sys.extras.specials {
                if let Some(tag) = w
                    .all_recipes(content)
                    .find(|d| &d.id == r)
                    .and_then(|d| d.tags.first())
                    && !tags.contains(tag)
                {
                    tags.push(*tag);
                }
            }
            if !tags.is_empty() {
                let u = until(w, minutes);
                w.sys.fx.wish_boosts.push((tags, *mul, u));
            }
        }
    }
}
