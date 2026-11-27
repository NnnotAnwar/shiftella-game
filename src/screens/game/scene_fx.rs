//! The café changing over time, in the live world (design/handson.md §5, §6.2, §11): the
//! season phase's look (leaves, frost, breath, snow, the star on the sign), the hall's decor
//! theme, grandma's garlands and the Advent wreath, the station cat, the night moments'
//! glowing markers and the coziness meter's world hints. Pure helpers: `scene.rs` calls them
//! at the documented places of the render frame order.

use std::collections::HashMap;

use macroquad::prelude::*;

use super::world_view::{self as wv, sim};
use crate::content::Content;
use crate::map::TILE;
use crate::render::cat::{CatPose, draw_cat};
use crate::render::{
    DecorTheme, FurnitureState, GarlandSpan, GarlandStyle, Garlands, Light, MomentMarker,
    MomentMarkerKind, Season, SeasonLook, ThemeStyle, furniture_lights,
};
use sim::{AgentRef, AgentView, World};

/// Everything seasonal / decorative the frame needs, read once from the sim.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Visuals {
    pub look: SeasonLook,
    pub season: Season,
    pub style: ThemeStyle,
    /// Garlands hung 0..1 (animated while Karolina hangs them).
    pub garlands: f32,
    /// Candles lit on the Advent wreath (0 = no wreath).
    pub wreath: Option<u8>,
    /// Coziness 0..1 (the lighting grade).
    pub coziness: f32,
    /// The winter-lights theme: fairy lights along the back wall.
    pub fairy: bool,
}

impl Default for Visuals {
    fn default() -> Self {
        Visuals {
            look: SeasonLook::NONE,
            season: Season::LateAutumn,
            style: ThemeStyle::GRANDMA,
            garlands: 0.0,
            wreath: None,
            coziness: 0.6,
            fairy: false,
        }
    }
}

impl Visuals {
    /// The cache key of the static layer (theme + the static season parts).
    pub fn static_key(&self) -> u64 {
        self.style.key() ^ self.look.static_key().wrapping_mul(0x9E37_79B9_7F4A_7C15)
    }
}

/// The season look of tonight's phase (`seasons.ron` visuals).
pub fn season_of(world: &World, content: &Content) -> (SeasonLook, Season) {
    let Some(p) = world.season_phase(content) else {
        return (Season::LateAutumn.look(), Season::LateAutumn);
    };
    let v = &p.visuals;
    let mut look = SeasonLook::from_flags(
        v.leaves,
        v.frost,
        v.breath,
        v.snow,
        v.garlands || v.sign_star,
        v.wreath,
    );
    // snow only falls when the sky says so (the phase keeps the snow lying around)
    if v.snow
        && world.weather() != crate::content::Weather::Snow
        && world.phase() == sim::Phase::Night
    {
        look.snowfall = 0.35;
    }
    // the storm night (`sim::storm`): a flurry all night
    if world.storm().tonight && world.phase() == sim::Phase::Night {
        look.snowfall = look.snowfall.max(1.8);
    }
    let season = Season::from_id(&p.id).unwrap_or(match p.id.as_str() {
        "summer" | "spring" => Season::LateAutumn,
        _ => Season::LateAutumn,
    });
    (look, season)
}

pub fn visuals(world: &World, content: &Content) -> Visuals {
    let (look, season) = season_of(world, content);
    let theme = DecorTheme::from_id(world.theme(content)).unwrap_or_default();
    let style = theme.style(season);
    let wreath = (look.wreath > 0.0).then(|| {
        // the Advent Sundays of the story calendar: night 21 and night 28
        let n = world.night();
        let mut lit: u8 = if world.mode() == sim::Mode::Story {
            if n >= 28 {
                2
            } else if n >= 21 {
                1
            } else {
                0
            }
        } else {
            1
        };
        // tonight's candle waits for her
        if world
            .active_moments()
            .iter()
            .any(|m| m.id == "m_advent_candle")
        {
            lit = lit.saturating_sub(1);
        }
        lit
    });
    Visuals {
        look,
        season,
        style,
        garlands: garland_progress(world, content),
        wreath,
        coziness: (world.coziness().value / 100.0).clamp(0.0, 1.0),
        fairy: theme == DecorTheme::WinterLights,
    }
}

/// Garlands: up (1), being hung by Karolina (her act's progress) or still in the box (0).
pub fn garland_progress(world: &World, content: &Content) -> f32 {
    if world.garlands_up(content) {
        return 1.0;
    }
    let hanging = world
        .active_moments()
        .iter()
        .any(|m| m.id == "m_hang_garlands" && m.resolving.is_some());
    let at_it = world.systems().moments.walk_now.as_ref().is_some_and(
        |j| matches!(&j.kind, sim::WalkKind::Moment { id, .. } if id == "m_hang_garlands"),
    );
    if hanging && at_it {
        world
            .player()
            .current
            .as_ref()
            .map(|c| c.progress())
            .unwrap_or(0.0)
    } else {
        0.0
    }
}

/// The bar counter run (x0, x1, front y in world px).
pub fn counter_span(world: &World, content: &Content) -> Option<(f32, f32, f32)> {
    // the front counter = the row of counter-height pieces with the register (else the one
    // nearest the hall); the back bar and a counter's return down the side don't count
    let mut rows: Vec<(i32, i32, i32, bool)> = Vec::new(); // (bottom row y, x0, x1, register)
    for p in world.furniture() {
        let Some(d) = content.furniture_def(&p.def) else {
            continue;
        };
        if !(d.has_tag("counter") || d.has_tag("register")) {
            continue;
        }
        let (w, h) = sim::grid::rotated_size(d, p.rot);
        let (x0, x1, y) = (p.x, p.x + w as i32, p.y + h as i32);
        let reg = d.has_tag("register");
        match rows.iter_mut().find(|r| r.0 == y) {
            Some(r) => {
                r.1 = r.1.min(x0);
                r.2 = r.2.max(x1);
                r.3 |= reg;
            }
            None => rows.push((y, x0, x1, reg)),
        }
    }
    // a return down the side is a column of 1-wide rows: prefer the widest row with the till
    rows.sort_by_key(|r| (!r.3, -(r.2 - r.1), r.0));
    rows.first()
        .map(|&(y, a, b, _)| (a as f32 * TILE, b as f32 * TILE, y as f32 * TILE - 8.0))
}

/// Grandma's garlands: over the back windows (warm bulbs) and along the counter (spruce).
pub fn garlands(glass: &[Rect], counter: Option<(f32, f32, f32)>) -> (Garlands, Garlands) {
    let windows = Garlands::over_windows(glass, GarlandStyle::WarmBulbs);
    let spans = counter
        .map(|(x0, x1, y)| {
            let swags = (((x1 - x0) / TILE / 2.2).round() as usize).clamp(2, 6);
            Garlands::along_counter(x0, x1, y, swags)
        })
        .unwrap_or_default();
    (windows, Garlands::new(spans, GarlandStyle::Bough))
}

/// The winter-lights theme's fairy lights along the back wall.
pub fn fairy_lights(width_tiles: i32) -> Garlands {
    let n = (width_tiles / 4).max(1);
    let spans = (0..n)
        .map(|i| {
            let x = 0.6 * TILE + i as f32 * 4.0 * TILE;
            GarlandSpan::new(vec2(x, -26.0), vec2(x + 4.0 * TILE, -26.0), 10.0)
        })
        .collect();
    Garlands::new(spans, GarlandStyle::FairyLights)
}

// ───────────────────────── the station cat ─────────────────────────

/// Where the cat is and how it sits, in world px (feet), if it is around tonight.
pub fn cat_at(
    world: &World,
    content: &Content,
    people: &[AgentView],
    t: f32,
) -> Option<(Vec2, CatPose, f32)> {
    let cat = world.station_cat(content)?;
    let waiting = world
        .active_moments()
        .iter()
        .any(|m| m.id == "m_station_cat" || m.id == "m_cat_name");
    let affection = (cat.level as f32 / 10.0).clamp(0.0, 1.0);
    let tile_c = |t: (i32, i32)| vec2(t.0 as f32 + 0.5, t.1 as f32 + 0.75) * TILE;
    let (feet, pose) = match cat.perch {
        "follows" if !waiting => {
            // a few steps behind Karolina, sitting when she stands still
            let k = world.karolina().map(|k| k.id);
            let v = people
                .iter()
                .find(|v| Some(v.agent) == k.map(AgentRef::Staff));
            match v {
                Some(v) => {
                    let side = if v.facing == sim::Facing::Left {
                        34.0
                    } else {
                        -34.0
                    };
                    let walking = v.pose == sim::Pose::Walk;
                    (
                        vec2(v.feet.0 + side, v.feet.1 + 6.0),
                        if walking { CatPose::Walk } else { CatPose::Sit },
                    )
                }
                None => (tile_c(cat.tile), CatPose::Sit),
            }
        }
        "radiator" if !waiting => (tile_c(cat.tile), CatPose::Sleep),
        "window_seat" if !waiting => (
            tile_c(cat.tile),
            if (t * 0.05).sin() > 0.0 {
                CatPose::Sleep
            } else {
                CatPose::Sit
            },
        ),
        _ => (tile_c(cat.tile), CatPose::Sit),
    };
    Some((feet, pose, affection))
}

pub fn draw_the_cat(feet: Vec2, pose: CatPose, t: f32, affection: f32) {
    draw_cat(feet, pose, t, affection);
}

// ───────────────────────── moment markers ─────────────────────────

/// Tall pieces float their marker higher (the jukebox, the counter, a window).
fn lift_for(sprite: &str) -> f32 {
    match sprite {
        "jukebox" | "bookshelf" | "fridge" | "coat_rack" => 112.0,
        "bar_counter" | "register" | "coffee_machine" | "espresso_grinder" => 92.0,
        "radio" | "menu_holo_board" => 96.0,
        _ => crate::render::MARKER_LIFT,
    }
}

/// The glowing markers of tonight's moments (id, marker), oldest first. `born` keeps when
/// each first appeared (for the pop-in), `hover` = the id under the mouse.
pub fn moment_markers(
    world: &World,
    content: &Content,
    people: &[AgentView],
    born: &mut HashMap<String, f32>,
    hover: Option<&str>,
    t: f32,
    sec_per_minute: f32,
) -> Vec<(String, MomentMarker)> {
    let mut out = Vec::new();
    let now = world.minute();
    born.retain(|id, _| world.active_moments().iter().any(|m| &m.id == id));
    for m in world.active_moments() {
        let b = *born.entry(m.id.clone()).or_insert(t);
        let kind = MomentMarkerKind::for_moment(&m.id);
        let agent = m
            .anchor
            .agent
            .and_then(|a| people.iter().find(|v| v.agent == a));
        let mut marker = if let Some(v) = agent {
            MomentMarker::over_person(vec2(v.feet.0, v.feet.1), kind, b, v.pose == sim::Pose::Sit)
        } else if let Some(p) = m.anchor.furniture.and_then(|u| world.furniture_by_uid(u)) {
            let def = content.furniture_def(&p.def);
            let (w, h) = def
                .map(|d| sim::grid::rotated_size(d, p.rot))
                .unwrap_or((1, 1));
            let sprite = def.map(|d| d.sprite.as_str()).unwrap_or("");
            let anchor = vec2(
                (p.x as f32 + w as f32 / 2.0) * TILE,
                (p.y as f32 + h as f32) * TILE - 10.0,
            );
            MomentMarker::new(anchor, kind, b).with_lift(lift_for(sprite))
        } else {
            let (x, y) = m.anchor.tile;
            let on_wall = world
                .grid()
                .tile(x, y)
                .is_some_and(|i| matches!(i.kind, sim::TileKind::Window | sim::TileKind::Wall));
            let anchor = vec2(x as f32 + 0.5, y as f32 + if on_wall { 1.1 } else { 0.8 }) * TILE;
            let lift = if on_wall {
                92.0
            } else {
                crate::render::MARKER_LIFT
            };
            MomentMarker::new(anchor, kind, b).with_lift(lift)
        };
        let left = m.expires.saturating_sub(now) as f32 * sec_per_minute;
        marker.expires_in = Some(if m.resolving.is_some() { 1.5 } else { left });
        if hover == Some(m.id.as_str()) && m.resolving.is_none() {
            marker.hover = 1.0;
        }
        out.push((m.id.clone(), marker));
    }
    out
}

// ───────────────────────── the coziness meter's hints ─────────────────────────

/// While the meter is hovered: the world shows the weakest part (dark seats glow faintly,
/// dirty tables pulse, a soft ring around the jukebox for music…).
pub fn draw_cozy_hints(world: &World, content: &Content, weakest: usize, t: f32) {
    let comp = crate::content::handson::CozyComponent::ALL
        .get(weakest)
        .copied();
    let pulse = 0.5 + 0.5 * (t * 3.2).sin();
    let glow = |c: Vec2, r: f32, col: Color, a: f32| {
        for i in 0..4 {
            let k = 1.0 - i as f32 / 4.0;
            draw_ellipse(
                c.x,
                c.y,
                r * (1.0 + i as f32 * 0.35),
                r * 0.45 * (1.0 + i as f32 * 0.35),
                0.0,
                Color::new(col.r, col.g, col.b, a * k * 0.35),
            );
        }
    };
    use crate::content::handson::CozyComponent as C;
    match comp {
        Some(C::Light) => {
            let lights: Vec<Light> = world
                .furniture()
                .iter()
                .zip(world.furniture_views(content))
                .flat_map(|(p, v)| {
                    let sprite = content
                        .furniture_def(&p.def)
                        .map(|d| d.sprite.clone())
                        .unwrap_or_default();
                    let st = FurnitureState {
                        active: v.active,
                        ..Default::default()
                    };
                    furniture_lights(
                        &sprite,
                        vec2(v.tile.0 as f32, v.tile.1 as f32) * TILE,
                        (v.size.0 as u8, v.size.1 as u8),
                        &st,
                    )
                })
                .collect();
            for p in world.furniture() {
                let Some(d) = content.furniture_def(&p.def) else {
                    continue;
                };
                if !d.has_tag("seat") {
                    continue;
                }
                let c = vec2(p.x as f32 + 0.5, p.y as f32 + 0.6) * TILE;
                let lit = lights
                    .iter()
                    .any(|l| l.pos.distance(c) < l.radius * 0.55 && l.intensity > 0.2);
                if !lit {
                    glow(c, 22.0, Color::new(0.65, 0.7, 1.0, 1.0), 0.5 + 0.4 * pulse);
                }
            }
        }
        Some(C::Clean) => {
            for p in world.furniture() {
                if p.dishes == 0 {
                    continue;
                }
                let c = vec2(p.x as f32 + 0.5, p.y as f32 + 0.7) * TILE;
                glow(c, 26.0, Color::new(1.0, 0.77, 0.42, 1.0), 0.5 + 0.5 * pulse);
            }
            for (x, y) in world.dirt_spots() {
                let c = vec2(x as f32 + 0.5, y as f32 + 0.55) * TILE;
                glow(c, 20.0, Color::new(1.0, 0.77, 0.42, 1.0), 0.4 + 0.4 * pulse);
            }
        }
        Some(C::Music) => {
            for p in world.furniture() {
                if content
                    .furniture_def(&p.def)
                    .is_some_and(|d| d.has_tag("music"))
                {
                    let c = vec2(p.x as f32 + 0.5, p.y as f32 + 0.8) * TILE;
                    glow(c, 30.0, Color::new(0.65, 0.7, 1.0, 1.0), 0.5 + 0.4 * pulse);
                }
            }
        }
        _ => {}
    }
    let _ = wv::clock_of_minute;
}
