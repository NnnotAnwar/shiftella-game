//! Story director: night beats, cutscene triggers & queue, regulars' visits and arcs,
//! Lída's notes, payments, endings; weather rolls; talk & ambient lines.

use super::guests;
use super::logic;
use super::types::*;
use super::world::*;
use crate::content::{BeatKind, Content, CutsceneStep, CutsceneTrigger, Weather};

// ---------------------------------------------------------------------------------------------
// Weather
// ---------------------------------------------------------------------------------------------

pub(crate) fn roll_weather(w: &mut World, content: &Content) {
    let forced = match w.mode {
        Mode::Story => content.story.night(w.night).and_then(|n| n.weather),
        Mode::Sandbox => w
            .town
            .options
            .get("rain_every_night")
            .copied()
            .unwrap_or(false)
            .then_some(Weather::Rain),
    };
    let weather = forced.unwrap_or_else(|| {
        let table: [(Weather, f32); 4] = match w.town.season {
            0 => [
                (Weather::Clear, 0.5),
                (Weather::Rain, 0.4),
                (Weather::Fog, 0.1),
                (Weather::Snow, 0.0),
            ],
            1 => [
                (Weather::Clear, 0.65),
                (Weather::Rain, 0.3),
                (Weather::Fog, 0.05),
                (Weather::Snow, 0.0),
            ],
            2 => [
                (Weather::Clear, 0.4),
                (Weather::Rain, 0.45),
                (Weather::Fog, 0.15),
                (Weather::Snow, 0.0),
            ],
            _ => [
                (Weather::Clear, 0.45),
                (Weather::Rain, 0.15),
                (Weather::Fog, 0.15),
                (Weather::Snow, 0.25),
            ],
        };
        let ws: Vec<f32> = table.iter().map(|t| t.1).collect();
        table[w.rng.weighted(&ws).unwrap_or(0)].0
    });
    w.weather = weather;
    w.emit(SimEvent::WeatherChanged { weather });
}

// ---------------------------------------------------------------------------------------------
// Day / night hooks
// ---------------------------------------------------------------------------------------------

/// Day phase of `w.night` begins: pre-22:00 beats fire (hints, bot messages, flags).
pub(crate) fn on_day_start(w: &mut World, content: &Content) {
    if w.mode != Mode::Story {
        return;
    }
    let Some(night) = content.story.night(w.night) else {
        return;
    };
    for (i, b) in night.beats.iter().enumerate() {
        if b.at.minutes_since_open() < 0 && !w.story.beats_done.contains(&i) {
            run_beat(w, content, i);
        }
    }
    let done: Vec<usize> = night
        .beats
        .iter()
        .enumerate()
        .filter(|(_, b)| b.at.minutes_since_open() < 0)
        .map(|(i, _)| i)
        .collect();
    // beats_done is reset at night start; keep the day beats marked through the night
    w.story.beats_done.extend(done);
}

pub(crate) fn on_night_start(w: &mut World, content: &Content) {
    // who visits tonight
    let night = w.night;
    let mut tonight: Vec<String> = Vec::new();
    for r in &content.regulars {
        let comes = match w.mode {
            Mode::Story => r.visits_story_night(night),
            Mode::Sandbox => match r.sandbox_visit_rule(night) {
                Some(b) => b,
                None => w.rng.chance(r.sandbox_chance),
            },
        };
        let weather_ok = r.weather.is_empty() || r.weather.contains(&w.weather);
        if comes && weather_ok {
            tonight.push(r.id.clone());
        }
    }
    for id in &tonight {
        w.story.regulars.entry(id.clone()).or_default();
    }
    for (id, st) in w.story.regulars.iter_mut() {
        st.spawned.clear();
        if !tonight.contains(id) {
            // mark every visit as done so nobody spawns
            st.spawned = (0..8).collect();
        }
    }
    // pre-22:00 beats were run in the day phase
    if w.mode == Mode::Story
        && let Some(n) = content.story.night(night)
    {
        for (i, b) in n.beats.iter().enumerate() {
            if b.at.minutes_since_open() < 0 {
                w.story.beats_done.insert(i);
            }
        }
    }
    if w.mode == Mode::Story {
        let trig = CutsceneTrigger::NightStart(night);
        let ids: Vec<String> = content
            .cutscenes
            .triggered_by(&trig)
            .map(|c| c.id.clone())
            .collect();
        for id in ids {
            queue_cutscene(w, content, &id);
        }
    }
}

pub(crate) fn on_minute(w: &mut World, content: &Content) {
    let m = w.minute();
    let now = w.clock();
    if w.mode == Mode::Story {
        // beats at this minute
        if let Some(n) = content.story.night(w.night) {
            for (i, b) in n.beats.iter().enumerate() {
                let at = b.at.minutes_since_open();
                if at >= 0 && at as u32 <= m && at < 480 && !w.story.beats_done.contains(&i) {
                    run_beat(w, content, i);
                }
            }
        }
        // timed cutscenes
        let ids: Vec<String> = content
            .cutscenes
            .cutscenes
            .iter()
            .filter(|c| matches!(&c.trigger, CutsceneTrigger::At { night, time } if *night == w.night && *time == now))
            .map(|c| c.id.clone())
            .collect();
        for id in ids {
            queue_cutscene(w, content, &id);
        }
    }
    regular_visits(w, content);
    arc_checks(w, content);
    if m.is_multiple_of(5) {
        check_notes(w, content);
    }
}

fn run_beat(w: &mut World, content: &Content, idx: usize) {
    let Some(night) = content.story.night(w.night) else {
        return;
    };
    let Some(b) = night.beats.get(idx) else {
        return;
    };
    w.story.beats_done.insert(idx);
    let mut ctx = EffectCtx::default();
    if !logic::all(w, content, &b.conditions, &mut ctx) {
        return;
    }
    match &b.what {
        BeatKind::Line { speaker, text_key } => {
            let agent = agent_for_speaker(w, speaker);
            w.emit(SimEvent::Speech {
                speaker: speaker.clone(),
                agent,
                key: text_key.clone(),
            });
        }
        BeatKind::Event(id) => super::narrator::fire(w, content, id, ctx, true),
        BeatKind::Note(id) => find_note(w, content, id),
        BeatKind::PaymentDue => super::economy::collect_payment(w, content),
        BeatKind::Train(id) => w.emit(SimEvent::TrainAnnounced { train: id.clone() }),
        BeatKind::Hint(k) => w.emit(SimEvent::Hint { key: k.clone() }),
        BeatKind::Bot(t) => super::app::bot(w, content, t, &ctx),
        BeatKind::Weather(x) => {
            w.weather = *x;
            w.emit(SimEvent::WeatherChanged { weather: *x });
        }
        BeatKind::Apply(list) => {
            let n = w.night;
            logic::run(w, content, &EffectSrc::Beat(n, idx), list, &mut ctx);
        }
    }
}

fn agent_for_speaker(w: &World, speaker: &str) -> Option<AgentRef> {
    if let Some(s) = w
        .staff
        .iter()
        .find(|s| s.present() && s.archetype == speaker)
    {
        return Some(AgentRef::Staff(s.id));
    }
    if speaker == "player"
        && let Some(s) = w.staff.iter().find(|s| s.owner && s.present())
    {
        return Some(AgentRef::Staff(s.id));
    }
    let pid = w.story.regulars.get(speaker).and_then(|r| r.party)?;
    let p = w.parties.iter().find(|p| p.id == pid)?;
    p.members.first().map(|g| AgentRef::Guest(*g))
}

/// 06:00 beats (payments) and last note checks, before the economy closes the night.
pub(crate) fn on_night_end_early(w: &mut World, content: &Content) {
    if w.mode == Mode::Story
        && let Some(n) = content.story.night(w.night)
    {
        for (i, b) in n.beats.iter().enumerate() {
            if !w.story.beats_done.contains(&i) && b.at.minutes_since_open() >= 0 {
                run_beat(w, content, i);
            }
        }
    }
    check_notes(w, content);
}

/// After the summary. Returns true when the game ended (ending / closure).
pub(crate) fn on_night_end(w: &mut World, content: &Content) -> bool {
    match w.mode {
        Mode::Story => {
            let night = w.night;
            let trig = CutsceneTrigger::NightEnd(night);
            let ids: Vec<String> = content
                .cutscenes
                .triggered_by(&trig)
                .map(|c| c.id.clone())
                .collect();
            for id in ids {
                queue_cutscene(w, content, &id);
            }
            let classic_close = content.difficulty(w.difficulty.id()).is_some_and(|d| {
                d.payments == crate::content::PaymentRule::Hard
                    && (d.close_after_missed.is_some_and(|n| {
                        w.story
                            .counters
                            .get("missed_payments")
                            .copied()
                            .unwrap_or(0)
                            >= n as i32
                    }) || w.story.flags.contains("final_payment_missed"))
            });
            if night >= content.story.nights_total || classic_close {
                reach_ending(w, content);
                return true;
            }
            false
        }
        Mode::Sandbox => {
            let bankrupt = content
                .difficulty(w.difficulty.id())
                .and_then(|d| d.bankrupt_after)
                .is_some_and(|n| w.econ.nights_in_red >= n);
            if bankrupt {
                w.closed = true;
                w.emit(SimEvent::CafeClosed);
                return true;
            }
            false
        }
    }
}

fn reach_ending(w: &mut World, content: &Content) {
    let mut ctx = EffectCtx::default();
    let endings = content.story.endings_in_order();
    let chosen = endings
        .iter()
        .find(|e| logic::all(w, content, &e.conditions, &mut ctx))
        .or(endings.last())
        .copied();
    let Some(e) = chosen else {
        w.closed = true;
        return;
    };
    let variant = e
        .variants
        .iter()
        .find(|v| logic::all(w, content, &v.conditions, &mut ctx))
        .map(|v| v.id.clone());
    w.story.ending = Some(e.id.clone());
    w.story.ending_variant = variant;
    w.closed = true;
    w.emit(SimEvent::EndingReached(e.id.clone()));
    if e.id == "last_train" {
        w.emit(SimEvent::CafeClosed);
    }
    queue_cutscene(w, content, &e.cutscene);
}

// ---------------------------------------------------------------------------------------------
// Cutscenes
// ---------------------------------------------------------------------------------------------

pub(crate) fn queue_cutscene(w: &mut World, content: &Content, id: &str) {
    let Some(cs) = content.cutscene(id) else {
        return;
    };
    if !cs.repeat && (w.story.played.contains(id) || w.story.queue.iter().any(|q| q == id)) {
        return;
    }
    let mut ctx = EffectCtx::default();
    if !logic::all(w, content, &cs.conditions, &mut ctx) {
        return;
    }
    w.story.played.insert(id.to_string());
    w.story.queue.push_back(id.to_string());
    start_next(w);
}

fn start_next(w: &mut World) {
    if w.story.playing.is_some() {
        return;
    }
    if let Some(next) = w.story.queue.pop_front() {
        w.story.playing = Some(next.clone());
        w.emit(SimEvent::PlayCutscene(next));
    }
}

/// `Command::CutsceneFinished`: replays `Apply` / `Choice` / `ShowNote` steps.
pub(crate) fn finish_cutscene(
    w: &mut World,
    content: &Content,
    id: &str,
    choices: &[usize],
) -> Result<(), CommandError> {
    if w.story.playing.as_deref() != Some(id) {
        return Err(CommandError::UnknownId(id.to_string()));
    }
    w.story.playing = None;
    let mut it = choices.iter().copied();
    if let Some(cs) = content.cutscene(id) {
        replay(w, content, id, &cs.steps, &[], &mut it, 0);
    }
    start_next(w);
    Ok(())
}

fn replay(
    w: &mut World,
    content: &Content,
    id: &str,
    steps: &[CutsceneStep],
    base: &[(u16, u8)],
    choices: &mut dyn Iterator<Item = usize>,
    depth: u32,
) {
    if depth > 6 {
        return;
    }
    for (i, step) in steps.iter().enumerate() {
        let path = |b: u8| -> Vec<(u16, u8)> {
            let mut p = base.to_vec();
            p.push((i as u16, b));
            p
        };
        match step {
            CutsceneStep::Apply { effects } => {
                let mut ctx = EffectCtx::default();
                logic::run(
                    w,
                    content,
                    &EffectSrc::Cutscene(id.to_string(), path(0), None),
                    effects,
                    &mut ctx,
                );
            }
            CutsceneStep::Choice { options } => {
                let Some(c) = choices.next() else { continue };
                let Some(opt) = options.get(c) else { continue };
                let mut ctx = EffectCtx::default();
                logic::run(
                    w,
                    content,
                    &EffectSrc::Cutscene(id.to_string(), path(0), Some(c as u16)),
                    &opt.effects,
                    &mut ctx,
                );
                if let Some(f) = &opt.set_flag {
                    logic::set_flag(w, content, f);
                }
            }
            CutsceneStep::ShowNote { note_id } => find_note(w, content, note_id),
            CutsceneStep::If {
                conditions,
                then,
                otherwise,
            } => {
                let mut ctx = EffectCtx::default();
                if logic::all(w, content, conditions, &mut ctx) {
                    replay(w, content, id, then, &path(0), choices, depth + 1);
                } else {
                    replay(w, content, id, otherwise, &path(1), choices, depth + 1);
                }
            }
            CutsceneStep::Call { cutscene } => {
                if let Some(cs) = content.cutscene(cutscene) {
                    w.story.played.insert(cutscene.clone());
                    replay(w, content, cutscene, &cs.steps, &[], choices, depth + 1);
                }
            }
            _ => {}
        }
    }
}

pub(crate) fn on_flag_set(w: &mut World, content: &Content, flag: &str) {
    let trig = CutsceneTrigger::FlagSet(flag.to_string());
    let ids: Vec<String> = content
        .cutscenes
        .triggered_by(&trig)
        .map(|c| c.id.clone())
        .collect();
    for id in ids {
        queue_cutscene(w, content, &id);
    }
}

// ---------------------------------------------------------------------------------------------
// Notes
// ---------------------------------------------------------------------------------------------

pub(crate) fn find_note(w: &mut World, content: &Content, id: &str) {
    if w.story.notes.iter().any(|n| n == id) {
        return;
    }
    let Some(note) = content.story.note(id) else {
        return;
    };
    w.story.notes.push(id.to_string());
    w.tonight.notes_found.push(id.to_string());
    w.emit(SimEvent::NoteFound(id.to_string()));
    w.log_key(format!("note.{id}.where"), Vec::new());
    let mut ctx = EffectCtx::default();
    logic::run(
        w,
        content,
        &EffectSrc::Note(id.to_string()),
        &note.effects,
        &mut ctx,
    );
}

fn check_notes(w: &mut World, content: &Content) {
    if w.mode != Mode::Story {
        return;
    }
    for n in &content.story.notes {
        if n.found_when.is_empty() || w.story.notes.contains(&n.id) {
            continue;
        }
        let mut ctx = EffectCtx::default();
        if logic::all(w, content, &n.found_when, &mut ctx) {
            find_note(w, content, &n.id);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Regulars
// ---------------------------------------------------------------------------------------------

fn regular_visits(w: &mut World, content: &Content) {
    let m = w.minute();
    for r in &content.regulars {
        let Some(spawned) = w.story.regulars.get(&r.id).map(|s| s.spawned.clone()) else {
            continue;
        };
        for (vi, v) in r.visits.iter().enumerate() {
            if spawned.contains(&vi) {
                continue;
            }
            let from = v.from.minutes_since_open();
            if from < 0 || (from as u32) > m {
                continue;
            }
            // mark first (a failed roll still uses the visit)
            if let Some(s) = w.story.regulars.get_mut(&r.id) {
                s.spawned.push(vi);
            }
            if !w.rng.chance(v.chance) {
                continue;
            }
            let mut ctx = EffectCtx::default();
            if r.absent_if
                .iter()
                .any(|c| logic::all(w, content, std::slice::from_ref(c), &mut ctx))
            {
                continue;
            }
            // too late for a visit we missed by more than half its window
            let to = v.to.minutes_since_open().max(from) as u32;
            if m > from as u32 + (to - from as u32) / 2 + 1 {
                continue;
            }
            guests::spawn_regular(w, content, &r.id, Some(vi));
            if r.id == "guest_333" {
                super::staff::one_shot_mood(w, content, &crate::content::Situation::MetGuest333);
            }
            break;
        }
    }
}

fn arc_checks(w: &mut World, content: &Content) {
    let night = w.night;
    for r in &content.regulars {
        let present = w
            .story
            .regulars
            .get(&r.id)
            .and_then(|s| s.party)
            .is_some_and(|p| w.parties.iter().any(|x| x.id == p));
        if !present {
            continue;
        }
        if w.story
            .regulars
            .get(&r.id)
            .is_some_and(|s| s.advanced_night == night)
        {
            continue;
        }
        let cur = w.story.counters.get(&r.arc_counter()).copied().unwrap_or(0);
        let Some(next) = r.stage(cur as u32 + 1) else {
            continue;
        };
        if !(next.nights.0..=next.nights.1).contains(&night) {
            continue;
        }
        let mut ctx = EffectCtx::default();
        if logic::all(w, content, &next.conditions, &mut ctx) {
            advance_arc(w, content, &r.id);
        }
    }
}

pub(crate) fn advance_arc(w: &mut World, content: &Content, regular: &str) {
    let Some(r) = content.regular(regular) else {
        return;
    };
    let key = r.arc_counter();
    let stage = w.story.counters.get(&key).copied().unwrap_or(0) + 1;
    w.story.counters.insert(key, stage);
    let night = w.night;
    w.story
        .regulars
        .entry(regular.to_string())
        .or_default()
        .advanced_night = night;
    w.emit(SimEvent::ArcAdvanced {
        regular: regular.to_string(),
        stage: stage as u32,
    });
    if let Some(s) = r.stage(stage as u32) {
        let mut ctx = EffectCtx::default();
        logic::run(
            w,
            content,
            &EffectSrc::Arc(regular.to_string(), stage as u32),
            &s.effects,
            &mut ctx,
        );
    }
    let trig = CutsceneTrigger::ArcStage {
        regular: regular.to_string(),
        stage: stage as u32,
    };
    let ids: Vec<String> = content
        .cutscenes
        .triggered_by(&trig)
        .map(|c| c.id.clone())
        .collect();
    for id in ids {
        queue_cutscene(w, content, &id);
    }
}

/// Ambient arc-stage bubble when a regular arrives.
pub(crate) fn regular_line(w: &mut World, content: &Content, regular: &str, party: u32) {
    let Some(r) = content.regular(regular) else {
        return;
    };
    let stage = w.story.counters.get(&r.arc_counter()).copied().unwrap_or(0) as u32;
    let lines = r
        .stages
        .iter()
        .filter(|s| s.stage <= stage)
        .max_by_key(|s| s.stage)
        .map(|s| s.lines.clone())
        .unwrap_or_default();
    let Some(key) = w.rng.pick(&lines).cloned() else {
        return;
    };
    let agent = w
        .parties
        .iter()
        .find(|p| p.id == party)
        .and_then(|p| p.members.first().copied())
        .map(AgentRef::Guest);
    w.emit(SimEvent::Speech {
        speaker: regular.to_string(),
        agent,
        key,
    });
}

fn regular_of_guest<'a>(
    w: &World,
    content: &'a Content,
    guest: u32,
) -> Option<&'a crate::content::RegularDef> {
    let g = w.guests.iter().find(|g| g.id == guest)?;
    let p = w.parties.iter().find(|p| p.id == g.party)?;
    content.regular(p.regular.as_deref()?)
}

/// Hover bubble line for a guest (regulars only): context lines first, then fallbacks.
/// Deterministic per guest & game minute (doesn't consume the sim RNG).
pub fn ambient_line_for(w: &World, content: &Content, guest: u32) -> Option<String> {
    let r = regular_of_guest(w, content, guest)?;
    let mut rng =
        super::rng::Rng::new(guest as u64 ^ (w.minute() as u64 / 3) << 20 ^ w.night as u64);
    let mut ctx = EffectCtx::default();
    let ctxual: Vec<&String> = r
        .ambient_lines
        .iter()
        .filter(|l| !l.when.is_empty() && logic::all_with(w, content, &l.when, &mut ctx, &mut rng))
        .map(|l| &l.key)
        .collect();
    let pool: Vec<&String> = if ctxual.is_empty() {
        r.ambient_lines
            .iter()
            .filter(|l| l.when.is_empty())
            .map(|l| &l.key)
            .collect()
    } else {
        ctxual
    };
    rng.pick(&pool).map(|s| s.to_string())
}

/// An unplayed `Talk` cutscene for this guest's regular tonight.
pub fn talk_available(w: &World, content: &Content, guest: u32) -> Option<String> {
    let r = regular_of_guest(w, content, guest)?;
    content
        .cutscenes
        .cutscenes
        .iter()
        .find(|c| {
            matches!(&c.trigger, CutsceneTrigger::Talk { regular, night }
                if *regular == r.id && night.is_none_or(|n| n == w.night))
                && (c.repeat || !w.story.played.contains(&c.id))
                && {
                    let mut rng = w.rng.clone();
                    logic::all_with(
                        w,
                        content,
                        &c.conditions,
                        &mut EffectCtx::default(),
                        &mut rng,
                    )
                }
        })
        .map(|c| c.id.clone())
}

pub(crate) fn talk(w: &mut World, content: &Content, guest: u32) -> Result<(), CommandError> {
    let id = talk_available(w, content, guest)
        .ok_or_else(|| CommandError::Invalid("nothing to talk about".into()))?;
    w.story.talks.insert(id.clone());
    queue_cutscene(w, content, &id);
    Ok(())
}
