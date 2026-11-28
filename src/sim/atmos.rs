//! The café changes over time (design/handson.md §11.1–§11.3): the hall's coziness (light,
//! beauty, comfort, cleanliness, music, theme, touches) and how it feeds guests' mood,
//! patience, stay and tips; the season phase per night (story calendar / slow sandbox
//! cycle); decor themes (unlock, buy, switch by day, perks); latte patterns.

use super::grid;
use super::grid::Room;
use super::systems::Coziness;
use super::types::*;
use super::world::*;
use crate::content::Content;
use crate::content::handson::{
    CozyComponent, HighlightKind, PhaseDef, ThemeDef, ThemePerk, Touch, Unlock,
};

// ---------------------------------------------------------------------------------------------
// Themes
// ---------------------------------------------------------------------------------------------

/// The hall's theme id (the default one until another is set).
pub(crate) fn theme_id<'a>(w: &'a World, content: &'a Content) -> &'a str {
    if !w.sys.atmos.theme.is_empty() {
        return &w.sys.atmos.theme;
    }
    content
        .handson
        .decor
        .default_theme()
        .map(|t| t.id.as_str())
        .unwrap_or("")
}

pub(crate) fn theme<'a>(w: &World, content: &'a Content) -> Option<&'a ThemeDef> {
    content.handson.theme(theme_id(w, content))
}

pub(crate) fn theme_owned(w: &World, content: &Content, id: &str) -> bool {
    w.sys.atmos.owned.contains(id)
        || content
            .handson
            .decor
            .default_theme()
            .is_some_and(|d| d.id == id)
}

/// `UnlockTheme`: owned for free.
pub(crate) fn give_theme(w: &mut World, content: &Content, id: &str) {
    if content.handson.theme(id).is_none() || theme_owned(w, content, id) {
        return;
    }
    w.sys.atmos.unlocked.insert(id.to_string());
    w.sys.atmos.owned.insert(id.to_string());
    w.emit(SimEvent::ThemeUnlocked {
        id: id.to_string(),
        owned: true,
    });
}

/// `Unlock::When` themes (day start + each minute); a free one is owned at once.
pub(crate) fn check_themes(w: &mut World, content: &Content) {
    for t in &content.handson.decor.themes {
        if w.sys.atmos.unlocked.contains(&t.id) || t.default {
            continue;
        }
        let ok = match &t.unlock {
            Unlock::Starter => true,
            Unlock::Granted => false,
            Unlock::When(c) => super::xlogic::holds(w, content, c),
        } || (w.mode == Mode::Sandbox && t.sandbox_night.is_some_and(|n| w.night >= n));
        if ok {
            w.sys.atmos.unlocked.insert(t.id.clone());
            let free = t.price == 0;
            if free {
                w.sys.atmos.owned.insert(t.id.clone());
            }
            w.emit(SimEvent::ThemeUnlocked {
                id: t.id.clone(),
                owned: free,
            });
        }
    }
}

/// `Command::BuyTheme`.
pub(crate) fn buy_theme(w: &mut World, content: &Content, id: &str) -> Result<(), CommandError> {
    let t = content
        .handson
        .theme(id)
        .ok_or_else(|| CommandError::UnknownId(id.to_string()))?;
    if theme_owned(w, content, id) {
        return Err(CommandError::AlreadyPlaced);
    }
    if !w.sys.atmos.unlocked.contains(id) {
        return Err(CommandError::Locked);
    }
    let price = t.price as i64;
    if w.econ.money < price {
        return Err(CommandError::NotEnoughMoney {
            need: price,
            have: w.econ.money,
        });
    }
    super::economy::spend_on_cafe(w, price);
    w.sys.atmos.owned.insert(id.to_string());
    w.emit(SimEvent::ThemeUnlocked {
        id: id.to_string(),
        owned: true,
    });
    Ok(())
}

/// `Command::SetTheme` (Day only). Returns the previous theme id for undo.
pub(crate) fn set_theme(
    w: &mut World,
    content: &Content,
    id: &str,
) -> Result<String, CommandError> {
    if content.handson.theme(id).is_none() {
        return Err(CommandError::UnknownId(id.to_string()));
    }
    if w.phase != Phase::Day {
        return Err(CommandError::WrongPhase);
    }
    if !theme_owned(w, content, id) {
        return Err(CommandError::Locked);
    }
    let old = theme_id(w, content).to_string();
    w.sys.atmos.theme = id.to_string();
    if old != id {
        w.emit(SimEvent::ThemeChanged { id: id.to_string() });
    }
    Ok(old)
}

/// A regular came in: the theme's `RegularTrustOnEnter` (once per night).
pub(crate) fn on_regular_arrived(w: &mut World, content: &Content, regular: &str) {
    let Some(t) = theme(w, content) else { return };
    for p in &t.perks {
        if let ThemePerk::RegularTrustOnEnter { regulars, add } = p
            && regulars.iter().any(|r| r == regular)
            && w.sys.atmos.theme_trust_tonight.insert(regular.to_string())
        {
            let v = w.story.trust.entry(regular.to_string()).or_insert(0.0);
            *v = (*v + add).min(100.0);
        }
    }
}

/// The theme's satisfaction perk for an archetype (at a serve).
pub(crate) fn archetype_bonus(w: &World, content: &Content, archetype: &str) -> f32 {
    theme(w, content)
        .map(|t| {
            t.perks
                .iter()
                .map(|p| match p {
                    ThemePerk::ArchetypeSatisfaction { archetypes, add }
                        if archetypes.iter().any(|a| a == archetype) =>
                    {
                        *add
                    }
                    _ => 0.0,
                })
                .sum()
        })
        .unwrap_or(0.0)
}

/// The ambient line a regular may say while another theme is on (`MissedBy`).
pub(crate) fn missed_line(w: &World, content: &Content, regular: &str) -> Option<String> {
    let cur = theme_id(w, content);
    content.handson.decor.themes.iter().find_map(|t| {
        if t.id == cur {
            return None;
        }
        t.perks.iter().find_map(|p| match p {
            ThemePerk::MissedBy {
                regular: r,
                key,
                until_night,
            } if r == regular && w.night <= *until_night => Some(key.clone()),
            _ => None,
        })
    })
}

// ---------------------------------------------------------------------------------------------
// Seasons & latte patterns
// ---------------------------------------------------------------------------------------------

pub(crate) fn phase<'a>(w: &World, content: &'a Content) -> Option<&'a PhaseDef> {
    super::pacing::season_phase(w, content).and_then(|id| content.handson.seasons.phase(id))
}

pub(crate) fn unlock_latte(w: &mut World, id: &str) {
    if w.sys.atmos.lattes.insert(id.to_string()) {
        w.emit(SimEvent::LatteUnlocked { id: id.to_string() });
    }
}

/// Day start: a new season phase → `SeasonPhaseChanged` (+ its toast), its latte patterns.
pub(crate) fn on_day_start(w: &mut World, content: &Content) {
    let ph = phase(w, content).map(|p| (p.id.clone(), p.toast_key(), p.latte.clone()));
    if let Some((id, toast, lattes)) = ph {
        if w.sys.atmos.phase.as_deref() != Some(id.as_str()) {
            let first = w.sys.atmos.phase.is_none();
            w.sys.atmos.phase = Some(id.clone());
            w.emit(SimEvent::SeasonPhaseChanged { id: id.clone() });
            if !first {
                w.emit(SimEvent::Toast { key: toast });
            }
        }
        for l in lattes {
            unlock_latte(w, &l);
        }
    }
    // latte patterns that open by night
    for p in &content.handson.minigames.latte_patterns {
        if w.sys.atmos.lattes.contains(&p.id) || p.granted {
            continue;
        }
        let night_ok = p.from_night.is_none_or(|n| w.night >= n);
        let when_ok = super::xlogic::holds(w, content, &p.when);
        let recipe_ok = p
            .recipe
            .as_ref()
            .is_none_or(|r| w.hands.knowledge.recipes.contains_key(r));
        if night_ok && when_ok && recipe_ok {
            let id = p.id.clone();
            unlock_latte(w, &id);
        }
    }
    check_themes(w, content);
}

/// Garlands render now: the phase allows them and they're up.
pub(crate) fn garlands_up(w: &World, content: &Content) -> bool {
    w.story.flags.contains("garlands_up") && phase(w, content).is_some_and(|p| p.visuals.garlands)
}

// ---------------------------------------------------------------------------------------------
// Coziness
// ---------------------------------------------------------------------------------------------

fn in_hall(w: &World, t: (i32, i32)) -> bool {
    w.grid.room(t.0, t.1) == Room::Hall
}

fn hall_tiles(w: &World) -> usize {
    (0..w.grid.height)
        .flat_map(|y| (0..w.grid.width).map(move |x| (x, y)))
        .filter(|t| in_hall(w, *t) && w.grid.is_indoor_floor(t.0, t.1))
        .count()
}

/// The coziness value and its parts right now (pure).
pub(crate) fn compute(w: &World, content: &Content) -> Coziness {
    let def = &content.handson.decor.coziness;
    let wt = |c| def.weight(c);
    let night = w.phase == Phase::Night;
    let blackout = w.blackout_until.is_some();
    let all_seats: Vec<super::guests::SeatInfo> = super::guests::seats(w, content)
        .into_iter()
        .filter(|s| in_hall(w, s.tile))
        .collect();
    let seats: Vec<(i32, i32, f32)> = all_seats
        .iter()
        .map(|s| (s.tile.0, s.tile.1, s.comfort))
        .collect();
    // light: hall seats within the radius of a lit light (candles in a blackout)
    let lights: Vec<((f32, f32), f32)> = w
        .furniture
        .iter()
        .filter_map(|p| {
            let d = content.furniture_def(&p.def)?;
            if d.light_radius <= 0.0 || !d.has_tag("light") {
                return None;
            }
            let lit = (p.on || !night) && !(blackout && d.needs_power);
            lit.then(|| {
                let fp = grid::footprint(d, p.x, p.y, p.rot);
                let n = fp.len().max(1) as f32;
                let cx = fp.iter().map(|t| t.0 as f32).sum::<f32>() / n;
                let cy = fp.iter().map(|t| t.1 as f32).sum::<f32>() / n;
                ((cx, cy), d.light_radius)
            })
        })
        .collect();
    let lit_share = if blackout && w.sys.fx.candles {
        1.0
    } else if seats.is_empty() {
        0.0
    } else {
        let lit = all_seats
            .iter()
            .map(|s| (s.tile.0, s.tile.1, s.bar))
            .filter(|(x, y, bar)| {
                // a seat at the bar sits under the bar's own pendant lamps (the renderer's
                // warm row over the counter), lit whenever the power is on
                (*bar && !blackout)
                    || lights.iter().any(|((cx, cy), r)| {
                        let dx = *x as f32 - cx;
                        let dy = *y as f32 - cy;
                        (dx * dx + dy * dy).sqrt() <= *r + 0.5
                    })
            })
            .count();
        lit as f32 / seats.len() as f32
    };
    let window_bonus = if w.window_light_on(content) {
        def.window_light_bonus
    } else {
        0.0
    };
    let light = (lit_share * wt(CozyComponent::Light) + window_bonus).min(wt(CozyComponent::Light));
    // beauty
    let th = theme(w, content);
    let per_lamp: f32 = th
        .map(|t| {
            t.perks
                .iter()
                .map(|p| match p {
                    ThemePerk::BeautyPerLamp(x) => *x,
                    _ => 0.0,
                })
                .sum()
        })
        .unwrap_or(0.0);
    let mut beauty_sum = w.sys.fx.table_beauty;
    let mut plants = 0u32;
    for p in &w.furniture {
        let Some(d) = content.furniture_def(&p.def) else {
            continue;
        };
        let fp = grid::footprint(d, p.x, p.y, p.rot);
        if !fp.iter().any(|t| in_hall(w, *t)) {
            continue;
        }
        beauty_sum += d.beauty;
        if d.has_tag("light") && d.light_radius > 0.0 {
            beauty_sum += per_lamp;
        }
        if d.sprite.starts_with("plant_") {
            plants += 1;
        }
    }
    // the back room is extra floor: it doesn't thin the hall's beauty out
    let tiles = hall_tiles(w)
        .saturating_sub(super::backroom::floor_tiles(w))
        .max(1) as f32;
    let beauty =
        (beauty_sum / (tiles * def.beauty_per_tile)).clamp(0.0, 1.0) * wt(CozyComponent::Beauty);
    // comfort
    let comfort = if seats.is_empty() {
        0.0
    } else {
        let mean = seats.iter().map(|s| s.2).sum::<f32>() / seats.len() as f32;
        (mean / def.comfort_div).clamp(0.0, 1.0) * wt(CozyComponent::Comfort)
    };
    // clean
    let dishes = w.furniture.iter().filter(|p| p.dishes > 0).count() as f32;
    let clean =
        (1.0 - w.dirt - def.dishes_penalty * dishes).clamp(0.0, 1.0) * wt(CozyComponent::Clean);
    // music
    let m = &def.music;
    let music_pts = if blackout && night {
        m.off
    } else if matches!(
        w.hands.music.source,
        super::hands::MusicSource::Record { .. }
    ) {
        m.record
    } else if super::music::jukebox(w).is_some() || w.music_on {
        m.house
    } else {
        m.off
    };
    let music = (music_pts / 10.0).clamp(0.0, 1.0) * wt(CozyComponent::Music);
    // theme
    let phase_id = super::pacing::season_phase(w, content).unwrap_or("");
    let theme_pts = th.map(|t| t.coziness_at(phase_id)).unwrap_or(0.0);
    let accents = th
        .and_then(|t| t.accent_tag.as_ref())
        .map(|tag| {
            w.furniture
                .iter()
                .filter(|p| {
                    content
                        .furniture_def(&p.def)
                        .is_some_and(|d| d.has_tag(tag))
                })
                .count() as f32
        })
        .unwrap_or(0.0);
    let accent = (accents * def.accent_bonus).min(def.accent_max);
    let theme_part =
        ((theme_pts + accent) / def.theme_div).clamp(0.0, 1.0) * wt(CozyComponent::Theme);
    // touches
    let mut touch = 0.0f32;
    for (t, pts) in &def.touches {
        let ok = match t {
            Touch::CatLevel(n) => w.sys.extras.cat_level >= *n,
            Touch::GarlandsInSeason => garlands_up(w, content),
            Touch::Plants(n) => plants >= *n,
            Touch::Furniture(id) => w.furniture.iter().any(|p| &p.def == id),
        };
        if ok {
            touch += pts;
        }
    }
    let touches = touch.min(wt(CozyComponent::Touches));
    let parts = [light, beauty, comfort, clean, music, theme_part, touches];
    let mut value: f32 = parts.iter().sum();
    // corners (pieces placed together: a reading nook, a window spot…)
    value += super::corners::coziness_bonus(w, content);
    if let Some(mood) = w.hall_mood(content) {
        for (m, add) in &def.mood_bonus {
            if *m == mood {
                value += add;
            }
        }
    }
    let now = w.minute();
    for (add, until) in &w.sys.fx.coziness {
        if until.is_none_or(|u| u > now) || !night {
            value += add;
        }
    }
    let value = value.clamp(0.0, 100.0);
    let weakest = CozyComponent::ALL
        .iter()
        .enumerate()
        .map(|(i, c)| (i, (wt(*c) - parts[i]) / wt(*c).max(0.01)))
        .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)))
        .map(|x| x.0)
        .unwrap_or(0);
    Coziness {
        value,
        level: def.level(value),
        parts,
        weakest,
    }
}

/// Recomputes the meter (each `every_minutes`, and after effects that change it).
pub(crate) fn recompute(w: &mut World, content: &Content) {
    let c = compute(w, content);
    let before = w.sys.atmos.coziness.level;
    let level = c.level;
    w.sys.atmos.coziness = c;
    if w.phase == Phase::Night && before != 0 && level != before {
        w.emit(SimEvent::CozinessLevel { level });
        let top = content.handson.decor.coziness.levels.len() as u32 + 1;
        if level == top && !w.sys.atmos.like_home_ever {
            w.sys.atmos.like_home_ever = true;
            super::journal::capture(
                w,
                content,
                HighlightKind::CozyLikeHome,
                None,
                Vec::new(),
                None,
            );
        }
    }
}

pub(crate) fn on_minute(w: &mut World, content: &Content) {
    let every = content.handson.decor.coziness.every_minutes.max(1);
    if w.minute().is_multiple_of(every) {
        recompute(w, content);
    }
    let v = w.sys.atmos.coziness.value;
    w.sys.atmos.night_sum += v;
    w.sys.atmos.night_n += 1;
    check_themes(w, content);
}

pub(crate) fn on_night_start(w: &mut World, content: &Content) {
    w.sys.atmos.night_sum = 0.0;
    w.sys.atmos.night_n = 0;
    w.sys.atmos.theme_trust_tonight.clear();
    recompute(w, content);
}

/// Mean coziness tonight.
pub(crate) fn night_avg(w: &World) -> f32 {
    if w.sys.atmos.night_n == 0 {
        w.sys.atmos.coziness.value
    } else {
        w.sys.atmos.night_sum / w.sys.atmos.night_n as f32
    }
}

fn coz(w: &World) -> f32 {
    w.sys.atmos.coziness.value
}

/// Satisfaction once seated and served: `(coz − 50) × 0.12` (replaces the old ambience terms).
pub(crate) fn satisfaction_term(w: &World, content: &Content) -> f32 {
    let e = &content.handson.decor.coziness.effects;
    (coz(w) - e.satisfaction_center) * e.satisfaction_per
}

pub(crate) fn patience_mul(w: &World, content: &Content) -> f32 {
    content.handson.decor.coziness.effects.patience.at(coz(w))
}

pub(crate) fn stay_mul(w: &World, content: &Content) -> f32 {
    content.handson.decor.coziness.effects.stay.at(coz(w))
}

pub(crate) fn tips_mul(w: &World, content: &Content) -> f32 {
    content.handson.decor.coziness.effects.tips.at(coz(w))
}
