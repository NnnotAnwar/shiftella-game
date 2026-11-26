//! The polaroid journal (design/handson.md §11.4): highlight capture during the night
//! (deterministic scene snapshots), the morning pick, the kept entries.

use super::grid::{self, TileKind};
use super::systems::*;
use super::types::*;
use super::world::*;
use crate::content::Content;
use crate::content::handson::HighlightKind;

/// The snapshot of the scene around `focus` (a tile; `None` = Karolina, else the window).
pub(crate) fn snap(w: &World, content: &Content, focus: Option<(i32, i32)>) -> SceneSnapData {
    let jf = &content.handson.journal;
    let (sw, sh) = (jf.snap_tiles.0.max(4) as i32, jf.snap_tiles.1.max(4) as i32);
    let focus = focus
        .or_else(|| super::player::karolina_idx(w).map(|k| w.staff[k].tile()))
        .unwrap_or((w.grid.width / 2, w.grid.height / 2));
    let ox = (focus.0 - sw / 2).clamp(0, (w.grid.width - sw).max(0));
    let oy = (focus.1 - sh / 2).clamp(0, (w.grid.height - sh).max(0));
    let inside = |t: (i32, i32)| t.0 >= ox && t.0 < ox + sw && t.1 >= oy && t.1 < oy + sh;
    let mut tiles = Vec::with_capacity((sw * sh) as usize);
    for y in oy..oy + sh {
        for x in ox..ox + sw {
            let t = match w.grid.tile(x, y) {
                None => SnapTileData::Void,
                Some(i) => match i.kind {
                    TileKind::Floor => SnapTileData::Floor(i.floor.id().to_string()),
                    TileKind::Street => SnapTileData::Floor("pavement".into()),
                    TileKind::Wall => SnapTileData::Wall,
                    TileKind::Window => SnapTileData::Window,
                    TileKind::Door => SnapTileData::Door(i.floor.id().to_string()),
                },
            };
            tiles.push(t);
        }
    }
    let mut props = Vec::new();
    for v in w.furniture_views(content) {
        let Some(p) = w.furniture_by_uid(v.uid) else {
            continue;
        };
        let touches = (0..v.size.0 as i32)
            .any(|dx| (0..v.size.1 as i32).any(|dy| inside((v.tile.0 + dx, v.tile.1 + dy))));
        if !touches {
            continue;
        }
        let sprite = content
            .furniture_def(&p.def)
            .map(|d| d.sprite.clone())
            .unwrap_or_else(|| p.def.clone());
        props.push(SnapPropData {
            def: p.def.clone(),
            sprite,
            tile: v.tile,
            size: (v.size.0 as u8, v.size.1 as u8),
            facing: v.facing,
            on: v.active,
            dirty: v.dirty,
        });
    }
    let mut actors = Vec::new();
    let mut cups = Vec::new();
    for v in w.staff_views().into_iter().chain(w.guest_views()) {
        let t = grid::px_to_tile(v.feet);
        if !inside(t) {
            continue;
        }
        let expression = match v.agent {
            AgentRef::Staff(id) => match w.staff_by_id(id) {
                Some(s) if s.owner => SnapExpression::Smile,
                Some(s) if s.energy < 30.0 => SnapExpression::Tired,
                Some(s) if s.mood >= 75.0 => SnapExpression::Smile,
                _ => SnapExpression::Neutral,
            },
            AgentRef::Guest(id) => {
                let sat = w
                    .guests
                    .iter()
                    .find(|g| g.id == id)
                    .and_then(|g| w.parties.iter().find(|p| p.id == g.party))
                    .map(|p| p.satisfaction)
                    .unwrap_or(60.0);
                if sat >= 90.0 {
                    SnapExpression::Laugh
                } else if sat >= 72.0 {
                    SnapExpression::Smile
                } else if sat < 35.0 {
                    SnapExpression::Sad
                } else {
                    SnapExpression::Neutral
                }
            }
        };
        if let (AgentRef::Guest(gid), Some(item)) = (v.agent, v.holding.clone()) {
            let recipe = w
                .items
                .iter()
                .filter(|i| i.guest == gid && i.state == ItemState::Served && i.menu == item)
                .find_map(|i| match &i.origin {
                    super::hands::ItemOrigin::Player { recipe, .. } => Some(recipe.clone()),
                    _ => None,
                });
            cups.push(SnapCupData {
                pos: v.feet,
                item,
                recipe,
                latte: None,
            });
        }
        actors.push(SnapActorData {
            look: v.look,
            feet: v.feet,
            facing: v.facing,
            pose: v.pose,
            expression,
            emote: v.emote,
            seat_tile: v.seat_tile,
            holding: v.holding,
        });
    }
    let cat = super::moments::cat_view(w, content)
        .filter(|c| inside(c.tile))
        .map(|c| grid::tile_center(c.tile));
    let phase = super::atmos::phase(w, content);
    let rain = match w.weather {
        crate::content::Weather::Rain => 1.0,
        crate::content::Weather::Downpour | crate::content::Weather::Storm => 2.0,
        _ => 0.0,
    };
    let train = w.phase == Phase::Night
        && super::logic::train_minutes(content)
            .iter()
            .any(|t| (*t as i64 - w.minute() as i64).abs() <= 1);
    let seed = super::rng::hash_str(&format!("{}:{}:{}", w.seed, w.night, w.tick)) as u32;
    SceneSnapData {
        origin: (ox, oy),
        size: (sw as u8, sh as u8),
        tiles,
        props,
        actors,
        cups,
        cat,
        night_progress: w.night_progress(),
        weather: format!("{:?}", w.weather).to_lowercase(),
        rain,
        season: phase.map(|p| p.id.clone()).unwrap_or_default(),
        theme: super::atmos::theme_id(w, content).to_string(),
        garlands: super::atmos::garlands_up(w, content),
        snow: phase.is_some_and(|p| p.visuals.snow) || w.weather == crate::content::Weather::Snow,
        blackout: w.blackout_until.is_some(),
        window_light: w.window_light_on(content),
        coziness: w.sys.atmos.coziness.value,
        train,
        seed,
        focus: grid::tile_center(focus),
        zoom: 1.0,
    }
}

/// Records a highlight candidate (night only; a day-phase one waits for the next night).
pub(crate) fn capture(
    w: &mut World,
    content: &Content,
    kind: HighlightKind,
    sub: Option<&str>,
    args: Vec<(String, String)>,
    focus: Option<(i32, i32)>,
) {
    if w.phase != Phase::Night {
        let mut args = args;
        if let Some(s) = sub {
            args.push(("__sub".into(), s.to_string()));
        }
        w.sys.journal.deferred.push((kind, args));
        return;
    }
    let jf = &content.handson.journal;
    let Some(rule) = jf.rule(kind) else {
        return;
    };
    // sub-captions must exist (moments / scenes listed in journal.ron)
    let sub = match kind {
        HighlightKind::MomentWarm => match sub {
            Some(m) if jf.moments.iter().any(|x| x == m) => Some(m.to_string()),
            _ => return,
        },
        HighlightKind::StoryScene => match sub {
            Some(s) if jf.scenes.iter().any(|x| x == s) => Some(s.to_string()),
            _ => return,
        },
        HighlightKind::ArcBeat => match sub {
            Some(s) => Some(s.to_string()),
            None => return,
        },
        _ => None,
    };
    let once_key = format!("{}:{}", kind.id(), sub.as_deref().unwrap_or(""));
    let mut score = rule.score;
    match kind {
        HighlightKind::Guest333 | HighlightKind::AnwarCameo => {
            if !w.sys.journal.firsts.insert(once_key.clone()) {
                score = rule.repeat_score.unwrap_or(score);
            }
        }
        HighlightKind::ArcBeat => {
            let regular = sub
                .as_deref()
                .and_then(|s| s.split('.').next())
                .unwrap_or("");
            if w.story.flags.contains(&format!("arc.{regular}.good")) {
                score += jf.arc_good_bonus;
            }
        }
        _ => {}
    }
    // one per kind + sub a night (a second perfect serve doesn't crowd the morning)
    if !w.sys.journal.tonight_kinds.insert(once_key) {
        return;
    }
    if w.sys.journal.last_kinds.contains(&kind) {
        score = score.saturating_sub(jf.repeat_kind_penalty);
    }
    let caption_key = kind.caption_key(sub.as_deref());
    let snap = snap(w, content, focus);
    let h = Highlight {
        night: w.night,
        minute: w.minute(),
        kind,
        score,
        caption_key,
        args,
        snap,
    };
    let keep = jf.keep_per_night.max(1) as usize;
    let list = &mut w.sys.journal.tonight;
    let pos = list
        .iter()
        .position(|x| x.score < score)
        .unwrap_or(list.len());
    if pos >= keep {
        return;
    }
    list.insert(pos, h);
    list.truncate(keep);
    w.emit(SimEvent::HighlightCaptured { kind, score });
}

pub(crate) fn on_night_start(w: &mut World, content: &Content) {
    // the morning closed without a pick: the best one is kept
    if w.sys.journal.pending.is_some() {
        let _ = keep(w, content, 0);
    }
    w.sys.journal.tonight.clear();
    w.sys.journal.tonight_kinds.clear();
}

/// Day-phase highlights (a happy rozpis) get their snapshot early in the night.
pub(crate) fn on_minute(w: &mut World, content: &Content) {
    let m = w.minute();
    if m == 2 {
        let d = std::mem::take(&mut w.sys.journal.deferred);
        for (kind, mut args) in d {
            let sub = args
                .iter()
                .position(|a| a.0 == "__sub")
                .map(|i| args.remove(i).1);
            capture(w, content, kind, sub.as_deref(), args, None);
        }
    }
    let fb = content.handson.journal.fallback.at.minutes_since_open();
    if fb >= 0 && m == fb as u32 {
        let focus = super::moments::anchor_window(w, content);
        capture(
            w,
            content,
            HighlightKind::QuietNight,
            None,
            Vec::new(),
            focus,
        );
    }
    // a full house (once a night)
    let seats = super::guests::seats(w, content);
    if !seats.is_empty() && m.is_multiple_of(5) {
        let taken = seats.iter().filter(|s| !s.free).count() as f32;
        if taken / seats.len() as f32 >= content.handson.journal.full_house_share {
            capture(w, content, HighlightKind::FullHouse, None, Vec::new(), None);
        }
    }
}

/// 06:00: the candidates go to the morning (a fallback if the night had none).
pub(crate) fn on_night_end(w: &mut World, content: &Content) {
    if w.sys.journal.tonight.is_empty() {
        let focus = super::moments::anchor_window(w, content);
        capture(
            w,
            content,
            HighlightKind::QuietNight,
            None,
            Vec::new(),
            focus,
        );
    }
    let list = std::mem::take(&mut w.sys.journal.tonight);
    let n = list.len() as u32;
    let night = w.night;
    w.sys.journal.pending = Some((night, list));
    w.emit(SimEvent::PolaroidsReady { night, count: n });
}

/// `Command::PickPolaroid` (the best is index 0).
pub(crate) fn keep(w: &mut World, content: &Content, index: usize) -> Result<(), CommandError> {
    let Some((night, list)) = w.sys.journal.pending.as_ref() else {
        return Err(CommandError::Invalid("no polaroids to pick".into()));
    };
    let Some(h) = list.get(index).cloned() else {
        return Err(CommandError::UnknownId(index.to_string()));
    };
    let night = *night;
    w.sys.journal.pending = None;
    let date = (w.mode == Mode::Story)
        .then_some(content.story.calendar.as_ref())
        .flatten()
        .map(|c| {
            let d = c.date_of_night(night);
            (d.year, d.month, d.day)
        });
    w.sys.journal.last_kinds = vec![h.kind];
    w.sys.journal.entries.push(JournalEntry {
        night,
        date,
        highlight: h,
    });
    w.emit(SimEvent::JournalUpdated { night });
    Ok(())
}
