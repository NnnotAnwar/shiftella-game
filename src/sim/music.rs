//! Grandma's jukebox (design/handson.md §10): the house playlist or a record, hall moods,
//! regulars' likes and dislikes (trust at most once per visit, never a loss), unlocks.

use super::hands::*;
use super::types::*;
use super::world::*;
use crate::content::Content;
use crate::content::handson::{HallMood, MoodEffect, Unlock};

/// Uid of grandma's jukebox, if it stands in the café.
pub(crate) fn jukebox(w: &World) -> Option<u32> {
    w.furniture
        .iter()
        .find(|p| p.def == "jukebox")
        .map(|p| p.uid)
}

/// The record currently on (not during a blackout).
pub(crate) fn current_record(w: &World) -> Option<&str> {
    match &w.hands.music.source {
        MusicSource::Record { id, .. } => Some(id),
        MusicSource::House => None,
    }
}

/// The hall mood: an effect's mood while it lasts, else the playing record's mood.
pub(crate) fn hall_mood(w: &World, content: &Content) -> Option<HallMood> {
    if w.phase != Phase::Night {
        return None;
    }
    if let Some(m) = w.hands.hall_mood.mood
        && w.hands.hall_mood.until.is_none_or(|u| u > w.minute())
    {
        return Some(m);
    }
    if w.blackout_until.is_some() {
        return None;
    }
    current_record(w)
        .and_then(|id| content.handson.record(id))
        .map(|r| r.mood)
}

/// The effect row of the current hall mood.
pub(crate) fn mood_effect<'a>(w: &World, content: &'a Content) -> Option<&'a MoodEffect> {
    hall_mood(w, content).and_then(|m| content.handson.records.mood(m))
}

/// Starter records (new game / migrated save).
pub(crate) fn init(w: &mut World, content: &Content) {
    for r in &content.handson.records.records {
        if r.is_starter() {
            w.hands.music.unlocked.insert(r.id.clone());
        }
    }
}

/// `Unlock::When` records (and the sandbox fallback nights). Checked at day start and every
/// game minute; an unlock stays.
pub(crate) fn check_unlocks(w: &mut World, content: &Content) {
    for r in &content.handson.records.records {
        if w.hands.music.unlocked.contains(&r.id) {
            continue;
        }
        let ok = match &r.unlock {
            Unlock::Starter => true,
            Unlock::Granted => false,
            Unlock::When(c) => super::xlogic::holds(w, content, c),
        } || (w.mode == Mode::Sandbox && r.sandbox_night.is_some_and(|n| w.night >= n));
        if ok {
            w.hands.music.unlocked.insert(r.id.clone());
            w.emit(SimEvent::RecordUnlocked { id: r.id.clone() });
        }
    }
}

/// Checks a `PlayRecord` request.
pub(crate) fn validate(w: &World, id: &Option<String>) -> Result<(), CommandError> {
    if let Some(id) = id
        && !w.hands.music.unlocked.contains(id)
    {
        return Err(CommandError::Locked);
    }
    if jukebox(w).is_none() {
        return Err(CommandError::NoJukebox);
    }
    if w.phase == Phase::Night && w.blackout_until.is_some() {
        return Err(CommandError::NoPower);
    }
    Ok(())
}

/// Puts the record on now (Karolina lifted the needle).
pub(crate) fn set_record(w: &mut World, content: &Content, id: Option<String>, repeat: bool) {
    let before = hall_mood(w, content);
    w.hands.music.source = match &id {
        Some(r) => MusicSource::Record {
            id: r.clone(),
            repeat,
        },
        None => MusicSource::House,
    };
    w.hands.music.record_minutes = 0;
    w.hands.music.requested_by = None;
    for p in w.parties.iter_mut() {
        p.record_minutes = 0;
    }
    if id.is_some() {
        if let Some(uid) = jukebox(w)
            && let Some(p) = w.furniture.iter_mut().find(|p| p.uid == uid)
        {
            p.on = true;
        }
        if !w.music_on {
            w.music_on = true;
            w.emit(SimEvent::MusicChanged { on: true });
        }
    }
    w.emit(SimEvent::RecordChanged { record: id, repeat });
    let after = hall_mood(w, content);
    if before != after {
        w.emit(SimEvent::HallMoodChanged { mood: after });
    }
}

pub(crate) fn on_night_start(w: &mut World, content: &Content) {
    w.hands.music.record_minutes = 0;
    w.hands.music.requested_by = None;
    w.hands.music.source = MusicSource::House;
    if let Some((id, repeat)) = w.hands.music.opening.clone()
        && w.hands.music.unlocked.contains(&id)
        && jukebox(w).is_some()
    {
        set_record(w, content, Some(id), repeat);
    }
}

pub(crate) fn on_night_end(w: &mut World) {
    w.hands.music.source = MusicSource::House;
    w.hands.music.record_minutes = 0;
    w.hands.hall_mood = HallMoodState::default();
}

/// `Command::RecordFinished`: a play-once pass ended → the house playlist.
pub(crate) fn record_finished(
    w: &mut World,
    content: &Content,
    id: &str,
) -> Result<(), CommandError> {
    match &w.hands.music.source {
        MusicSource::Record { id: cur, repeat } if cur == id => {
            if !*repeat {
                set_record(w, content, None, false);
            }
            Ok(())
        }
        _ => Err(CommandError::Invalid("that record isn't playing".into())),
    }
}

/// Every game minute: record time, regulars' likes / dislikes, archetype tastes, mood touches.
pub(crate) fn on_minute(w: &mut World, content: &Content) {
    check_unlocks(w, content);
    let rules = &content.handson.records.rules;
    let record = if w.blackout_until.is_none() {
        current_record(w).map(str::to_string)
    } else {
        None
    };
    if record.is_some() {
        w.hands.music.record_minutes += 1;
    }
    let mood = mood_effect(w, content).cloned();
    let tick = w.tick;
    for pi in 0..w.parties.len() {
        let p = &w.parties[pi];
        let inside = !matches!(p.state, PartyState::Arriving | PartyState::Leaving);
        if !inside {
            continue;
        }
        if record.is_some() {
            w.parties[pi].record_minutes += 1;
        }
        let ready = w.parties[pi].record_minutes >= rules.like_minutes;
        let leader = w.parties[pi].leader();
        // regulars' likes & dislikes, once per visit
        if let (Some(rec_id), Some(reg)) = (&record, w.parties[pi].regular.clone())
            && ready
            && !w.parties[pi].music_reacted
            && let Some(rec) = content.handson.record(rec_id)
        {
            let liked = rec.liked_by.contains(&reg);
            let disliked = rec.disliked_by.contains(&reg);
            if liked || disliked {
                w.parties[pi].music_reacted = true;
                w.hands
                    .music
                    .known_tastes
                    .insert((rec_id.clone(), reg.clone()));
                if liked {
                    let add = if w.hands.music.requested_by.as_deref() == Some(reg.as_str()) {
                        rules.request_trust
                    } else {
                        rules.like_trust
                    };
                    let t = w.story.trust.entry(reg.clone()).or_insert(0.0);
                    *t = (*t + add).min(100.0);
                } else {
                    let s = &mut w.parties[pi].satisfaction;
                    *s = (*s + rules.dislike_satisfaction).clamp(0.0, 100.0);
                }
                if let Some(g) = leader.and_then(|l| w.guests.iter_mut().find(|g| g.id == l)) {
                    g.emote = Some((
                        if liked {
                            SimEmote::Music
                        } else {
                            SimEmote::Note
                        },
                        tick + 30,
                    ));
                }
                let key = if liked {
                    rec.liked_key(&reg)
                } else {
                    rec.disliked_key(&reg)
                };
                w.emit(SimEvent::Speech {
                    speaker: reg.clone(),
                    agent: leader.map(AgentRef::Guest),
                    key,
                });
                if let Some(g) = leader {
                    w.emit(SimEvent::RecordReaction {
                        guest: g,
                        record: rec_id.clone(),
                        liked,
                    });
                }
            }
        }
        // archetype tastes and the hall mood's touches, once per visit
        if ready && !w.parties[pi].archetype_music_done {
            w.parties[pi].archetype_music_done = true;
            let arch = w.parties[pi].archetype.clone();
            let mut add = 0.0;
            if let (Some(rec_id), Some(a)) = (&record, content.guest(&arch)) {
                if a.music_likes.contains(rec_id) {
                    add += rules.archetype_like;
                } else if a.music_dislikes.contains(rec_id) {
                    add += rules.archetype_dislike;
                }
            }
            if let Some(m) = &mood {
                for s in &m.satisfaction {
                    if s.archetypes.contains(&arch) {
                        add += s.value;
                    }
                }
                if let (Some((age, trust)), Some(reg)) =
                    (m.elder_trust, w.parties[pi].regular.clone())
                    && content.regular(&reg).is_some_and(|r| r.age >= age)
                {
                    let t = w.story.trust.entry(reg).or_insert(0.0);
                    *t = (*t + trust).min(100.0);
                }
            }
            if add != 0.0 {
                let s = &mut w.parties[pi].satisfaction;
                *s = (*s + add).clamp(0.0, 100.0);
            }
        }
    }
}

/// The track the café should play now and its volume (`None` = silence: a blackout, or no
/// jukebox at night).
pub(crate) fn track(w: &World, content: &Content) -> Option<(String, f32)> {
    let house = &content.handson.records.house;
    if w.phase == Phase::Day {
        return Some((house.day.clone(), 1.0));
    }
    if w.blackout_until.is_some() {
        return None;
    }
    if let Some(id) = current_record(w)
        && let Some(r) = content.handson.record(id)
    {
        return Some((r.track.clone(), 1.0));
    }
    Some(match house_state(w, content) {
        HouseState::Rush => (house.rush.clone(), 1.0),
        HouseState::Quiet => (house.quiet.clone(), house.quiet_volume),
        HouseState::Calm => (house.calm.clone(), 1.0),
    })
}

pub(crate) fn house_state(w: &World, content: &Content) -> HouseState {
    if w.hands.rush.until.is_some() {
        HouseState::Rush
    } else if super::pacing::quiet_hour(w, content) {
        HouseState::Quiet
    } else {
        HouseState::Calm
    }
}
