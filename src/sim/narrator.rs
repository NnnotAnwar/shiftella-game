//! The Night storyteller: picks `EventDef`s by weight, conditions and pacing (tension budget,
//! breathing room, act curve, witching hour), fires Auto events, resolves choices.

use super::logic;
use super::types::*;
use super::world::*;
use crate::content::{Content, EventDef, EventKind, EventTrigger, GameMode, NightCharacter};

/// Game minutes a choice toast stays before `default_choice` applies.
pub const CHOICE_MINUTES: u32 = 20;
/// Auto events with `cooldown: 0` re-fire at most this often.
pub const AUTO_REPEAT_MINUTES: u32 = 30;

fn game_mode(w: &World) -> GameMode {
    match w.mode {
        Mode::Story => GameMode::Story,
        Mode::Sandbox => GameMode::Sandbox,
    }
}

fn character<'a>(w: &World, content: &'a Content) -> Option<&'a NightCharacter> {
    content.sandbox.character(&w.town.character).or_else(|| {
        content
            .sandbox
            .character(&content.sandbox.default_character)
    })
}

fn on_cooldown(w: &World, e: &EventDef) -> bool {
    if w.narrator.fired_tonight.contains(&e.id) {
        return true;
    }
    w.narrator
        .last_fired
        .get(&e.id)
        .is_some_and(|last| w.night < last + e.cooldown.max(1))
}

fn weight_mul(w: &World, id: &str) -> f32 {
    w.narrator
        .weights
        .iter()
        .filter(|(e, _, until)| e == id && *until >= w.night)
        .map(|(_, m, _)| *m)
        .product()
}

fn is_staff_event(e: &EventDef) -> bool {
    e.id.starts_with("staff_")
}

pub(crate) fn on_minute(w: &mut World, content: &Content) {
    let m = w.minute();
    logic::run_later(w, content);
    // expired choices
    let expired: Vec<PendingChoice> = w
        .narrator
        .pending
        .iter()
        .filter(|p| p.expires <= m)
        .cloned()
        .collect();
    for p in expired {
        match p.default.clone() {
            Some(c) => {
                let _ = answer(w, content, &p.event, &c);
            }
            None => w.narrator.pending.retain(|x| x.event != p.event),
        }
    }
    // foreseen events arrive
    let due: Vec<String> = w
        .narrator
        .foreseen
        .iter()
        .filter(|f| f.1 <= m)
        .map(|f| f.0.clone())
        .collect();
    w.narrator.foreseen.retain(|f| f.1 > m);
    for id in due {
        fire(w, content, &id, EffectCtx::default(), false);
    }
    // tension decays 10 per hour
    w.narrator.tension = (w.narrator.tension - 10.0 / 60.0).max(0.0);
    auto_events(w, content);
    random_events(w, content);
}

fn auto_events(w: &mut World, content: &Content) {
    let m = w.minute();
    let mode = game_mode(w);
    for e in &content.events {
        if e.trigger != EventTrigger::Auto || e.conditions.is_empty() {
            continue;
        }
        if !e.modes.allows(mode) || w.night < e.min_night {
            continue;
        }
        let fired = w.narrator.fired_tonight.contains(&e.id);
        if fired {
            if e.cooldown > 0 {
                continue;
            }
            if w.narrator
                .auto_last
                .get(&e.id)
                .is_some_and(|last| m < last + AUTO_REPEAT_MINUTES)
            {
                continue;
            }
        } else if e.cooldown > 0
            && w.narrator
                .last_fired
                .get(&e.id)
                .is_some_and(|last| w.night < last + e.cooldown)
        {
            continue;
        }
        let mut ctx = EffectCtx::default();
        if logic::all(w, content, &e.conditions, &mut ctx) {
            w.narrator.auto_last.insert(e.id.clone(), m);
            fire(w, content, &e.id, ctx, false);
        }
    }
}

fn random_events(w: &mut World, content: &Content) {
    let m = w.minute();
    let Some(ch) = character(w, content) else {
        return;
    };
    let now = w.clock();
    let nar = &content.narrator;
    let (act_mult, max_sev, warm) = match w.mode {
        Mode::Story => nar
            .act_for_night(w.night)
            .map(|a| (a.act_mult, a.max_severity, a.warm_event_guaranteed))
            .unwrap_or((1.0, 3, false)),
        Mode::Sandbox => (1.0, 3, false),
    };
    // Challenge: the nights get harder each season (the identity in Endless)
    let hard = super::challenge::storyteller(w, content);
    let mut p = ch.base_rate * nar.curve_at(now) * act_mult * hard.rate;
    // fewer random rolls once the night already had plenty
    if w.narrator.random_tonight >= ch.events_per_night.1 {
        p *= 0.15;
    }
    let force_good = warm && w.narrator.good_tonight == 0 && m == 300;
    if !force_good && !w.rng.chance(p) {
        return;
    }
    let mode = game_mode(w);
    let tomorrow_payment = w.mode == Mode::Story && content.story.payment_on(w.night + 1).is_some();
    let witching = {
        let (a, b, mul) = nar.witching_hour;
        if now.is_between(a, b) { mul } else { 1.0 }
    };
    let poverty = ch.poverty_help && w.econ.money < w.econ.rent.max(1000);
    let breath = w
        .narrator
        .last_bad
        .is_some_and(|lb| m < lb + (ch.breath_minutes as f32 * hard.breath).round() as u32);
    let mut cands: Vec<(usize, f32, EffectCtx)> = Vec::new();
    for (i, e) in content.events.iter().enumerate() {
        if e.trigger != EventTrigger::Random || e.weight <= 0.0 {
            continue;
        }
        if !e.modes.allows(mode) || w.night < e.min_night || e.severity > max_sev {
            continue;
        }
        if on_cooldown(w, e) {
            continue;
        }
        if force_good && e.kind != EventKind::Good {
            continue;
        }
        if e.kind == EventKind::Bad
            && (breath
                || w.narrator.bad_in_row >= ch.max_bad_in_row + hard.bad_in_row
                || w.narrator.tension + e.severity as f32 * 10.0 > ch.max_tension + hard.tension
                || (nar.calm_before_payment && tomorrow_payment && e.severity >= 3))
        {
            continue;
        }
        if is_staff_event(e)
            && ch
                .staff_events_cap
                .is_some_and(|cap| w.narrator.staff_events >= cap)
        {
            continue;
        }
        let mut ctx = EffectCtx::default();
        let mut rng = w.rng.clone();
        if !logic::all_with(w, content, &e.conditions, &mut ctx, &mut rng) {
            w.rng = rng;
            continue;
        }
        w.rng = rng;
        let mut weight = e.weight * weight_mul(w, &e.id);
        if e.kind == EventKind::Uncanny {
            weight *= ch.uncanny_mul * witching;
        }
        if e.kind == EventKind::Good && poverty {
            weight *= 2.0;
        }
        if e.kind == EventKind::Bad || hard.also.contains(&e.id) {
            weight *= hard.bad;
        }
        if weight > 0.0 {
            cands.push((i, weight, ctx));
        }
    }
    if cands.is_empty() {
        return;
    }
    // good share
    if !force_good && w.rng.chance(ch.good_share * hard.good_share) {
        let goods: Vec<(usize, f32, EffectCtx)> = cands
            .iter()
            .filter(|c| content.events[c.0].kind == EventKind::Good)
            .cloned()
            .collect();
        if !goods.is_empty() {
            cands = goods;
        }
    }
    let weights: Vec<f32> = cands.iter().map(|c| c.1).collect();
    let Some(k) = w.rng.weighted(&weights) else {
        return;
    };
    let (i, _, ctx) = cands.swap_remove(k);
    let id = content.events[i].id.clone();
    w.narrator.random_tonight += 1;
    // Foresight: someone senses it coming.
    if let Some(minutes) = foresight(w, content) {
        w.narrator.foreseen.push((id.clone(), m + minutes));
        w.narrator.fired_tonight.push(id.clone()); // reserve (no double pick)
        w.emit(SimEvent::EventHint { id, minutes });
        return;
    }
    fire(w, content, &id, ctx, false);
}

fn foresight(w: &mut World, content: &Content) -> Option<u32> {
    let mut found = None;
    for s in w.staff.iter().filter(|s| s.on_duty()) {
        for t in &s.traits {
            if let Some(def) = content.trait_def(t) {
                for e in &def.effects {
                    if let crate::content::TraitEffect::Foresight { chance, minutes } = e {
                        found = Some((*chance, *minutes));
                    }
                }
            }
        }
    }
    let (chance, minutes) = found?;
    w.rng.chance(chance).then_some(minutes)
}

/// Fires an event now: effects, toast, choices, bookkeeping. `forced` skips mode/conditions
/// (story beats, `StartEvent`, mishaps).
pub(crate) fn fire(w: &mut World, content: &Content, id: &str, mut ctx: EffectCtx, forced: bool) {
    let Some(e) = content.event(id) else {
        return;
    };
    if !forced && !e.modes.allows(game_mode(w)) {
        return;
    }
    let m = w.minute();
    let night = w.night;
    w.narrator.last_fired.insert(e.id.clone(), night);
    // a foreseen reservation is replaced by the real firing
    if !w.narrator.fired_tonight.contains(&e.id) {
        w.narrator.fired_tonight.push(e.id.clone());
    }
    w.narrator.active.push((e.id.clone(), m + 60));
    w.tonight.events.push(e.id.clone());
    // sim-fired consequences (mishaps) don't steer the pacing
    let paced = !(e.trigger == EventTrigger::Auto && e.conditions.is_empty());
    match e.kind {
        _ if !paced => {
            if e.kind == EventKind::Bad {
                w.tonight.bad_events += 1;
            }
        }
        EventKind::Bad => {
            w.narrator.tension += e.severity as f32 * 10.0;
            w.narrator.bad_tonight += 1;
            w.narrator.bad_in_row += 1;
            w.narrator.last_bad = Some(m);
            w.tonight.bad_events += 1;
        }
        EventKind::Good => {
            w.narrator.tension = (w.narrator.tension - e.severity as f32 * 10.0).max(0.0);
            w.narrator.good_tonight += 1;
            w.narrator.bad_in_row = 0;
        }
        _ => {
            w.narrator.tension += e.severity as f32 * 3.0;
            w.narrator.bad_in_row = 0;
        }
    }
    w.narrator.peak_tension = w.narrator.peak_tension.max(w.narrator.tension);
    if is_staff_event(e) {
        w.narrator.staff_events += 1;
    }
    w.emit(SimEvent::EventStarted { id: e.id.clone() });
    w.log_key(e.title_key(), Vec::new());
    ctx.event = Some(e.id.clone());
    logic::run(
        w,
        content,
        &EffectSrc::Event(e.id.clone()),
        &e.effects,
        &mut ctx,
    );
    if e.kind == EventKind::Bad {
        super::staff::one_shot_mood(w, content, &crate::content::Situation::BadEvent);
    }
    // a night moment replaces the choice toast (its default lives in `on_expire`)
    if !e.choices.is_empty() && super::moments::on_event(w, content, &e.id) {
        return;
    }
    if !e.choices.is_empty() {
        w.narrator.pending.retain(|p| p.event != e.id);
        w.narrator.pending.push(PendingChoice {
            event: e.id.clone(),
            choices: e.choices.iter().map(|c| c.id.clone()).collect(),
            default: e.default_choice.clone(),
            expires: m + CHOICE_MINUTES,
            ctx: ctx.clone(),
        });
        w.emit(SimEvent::ChoiceNeeded {
            id: e.id.clone(),
            choices: e.choices.iter().map(|c| c.id.clone()).collect(),
            expires_in: CHOICE_MINUTES,
        });
    }
}

/// Applies a choice of a pending event.
pub(crate) fn answer(
    w: &mut World,
    content: &Content,
    event: &str,
    choice: &str,
) -> Result<(), CommandError> {
    let Some(pos) = w.narrator.pending.iter().position(|p| p.event == event) else {
        return Err(CommandError::UnknownId(event.to_string()));
    };
    let def = content
        .event(event)
        .ok_or_else(|| CommandError::UnknownId(event.to_string()))?;
    let c = def
        .choices
        .iter()
        .find(|c| c.id == choice)
        .ok_or_else(|| CommandError::UnknownId(choice.to_string()))?;
    let pending = w.narrator.pending.remove(pos);
    // the problem is handled: slow-downs caused by this event end (and the piece is serviced)
    let fixed: Vec<String> = w
        .mods
        .iter()
        .filter(|m| m.event.as_deref() == Some(event))
        .filter_map(|m| match &m.kind {
            ModKind::StationSlow(t) => Some(t.clone()),
            _ => None,
        })
        .collect();
    for p in w.furniture.iter_mut() {
        if content
            .furniture_def(&p.def)
            .is_some_and(|d| fixed.iter().any(|t| d.has_tag(t)))
        {
            p.wear = (p.wear - 0.4).max(0.0);
        }
    }
    w.mods.retain(|m| {
        !(matches!(m.kind, ModKind::StationSlow(_)) && m.event.as_deref() == Some(event))
    });
    let mut ctx = pending.ctx;
    logic::run(
        w,
        content,
        &EffectSrc::EventChoice(event.to_string(), choice.to_string()),
        &c.effects,
        &mut ctx,
    );
    if let Some(f) = &c.set_flag {
        logic::set_flag(w, content, f);
    }
    w.emit(SimEvent::ChoiceResolved {
        id: event.to_string(),
        choice: choice.to_string(),
    });
    w.log_key(def.result_key(choice), Vec::new());
    Ok(())
}

pub(crate) fn on_night_end(w: &mut World, content: &Content) {
    let hard = w.narrator.bad_tonight >= 3 || w.narrator.peak_tension >= 60.0;
    w.narrator.hard_nights = if hard { w.narrator.hard_nights + 1 } else { 0 };
    let night = w.night;
    w.narrator.weights.retain(|(_, _, until)| *until > night);
    // equipment wear
    let wear_mul = content
        .difficulty(w.difficulty.id())
        .map(|d| d.wear_mul)
        .unwrap_or(1.0);
    let challenge_wear = super::challenge::wear_mul(w, content);
    for p in w.furniture.iter_mut() {
        if let Some(d) = content.furniture_def(&p.def) {
            p.wear = (p.wear + d.wear_per_night * wear_mul * challenge_wear).min(1.0);
        }
    }
}
