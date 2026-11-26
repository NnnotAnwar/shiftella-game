//! Render demo: the night café from `map::CAFE_LAYOUT`, guests and staff, rain,
//! a train, light. Camera: `WorldCamera` (WASD / arrows, right button, wheel).
//!
//! Environment variables:
//! - `SHIFTELLA_SHOT=path.png` — render ~40 frames, save a PNG and exit;
//! - `SHIFTELLA_ZOOM=2.5`, `SHIFTELLA_CENTER=12.5,5` (in tiles), `SHIFTELLA_T=7.0` (time, s);
//! - `SHIFTELLA_NIGHT=0.55` (night progress 0..1), `SHIFTELLA_BLACKOUT=1`, `SHIFTELLA_TRAIN=0.5`;
//! - `SHIFTELLA_STRESS=1` — 40+ people and ~150 objects (performance check);
//! - `SHIFTELLA_DEMO=portraits|people|furniture` — galleries of portraits, people, furniture;
//! - `SHIFTELLA_DEMO=season_autumn|season_frost|season_festive|season_snow` — the café in a
//!   season (`SHIFTELLA_GARLANDS=0..1` garlands progress, `SHIFTELLA_THEME=<theme id>`,
//!   `SHIFTELLA_COZY=0..1` coziness);
//! - `SHIFTELLA_DEMO=handson` — tip coins, wish bubbles, order marks, queue chips, cup glyphs, emotes;
//! - `SHIFTELLA_DEMO=moments` — night-moment markers (with `SHIFTELLA_BLACKOUT=1`: candles being lit);
//! - `SHIFTELLA_DEMO=themes` — the decor themes side by side;
//! - `SHIFTELLA_DEMO=polaroid` — snapshots re-rendered from data in polaroid frames.

use std::time::Instant;

use macroquad::prelude::*;

use shiftella_game::camera::WorldCamera;
use shiftella_game::map::{CAFE_LAYOUT, Map, TILE, Tile};
use shiftella_game::render::cat::{CatPose, draw_cat};
use shiftella_game::render::colors as pal;
use shiftella_game::render::draw::{alpha, radial};
use shiftella_game::render::*;

fn window_conf() -> Conf {
    shiftella_game::platform::hide_for_automation();
    Conf {
        window_title: "Shiftella — render demo".to_owned(),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        sample_count: 4,
        ..Default::default()
    }
}

fn env_f(name: &str) -> Option<f32> {
    std::env::var(name).ok().and_then(|v| v.parse().ok())
}

struct Piece {
    sprite: &'static str,
    origin: Vec2,
    size: (u8, u8),
    facing: Facing,
    dirty: f32,
}

impl Piece {
    fn at(sprite: &'static str, x: f32, y: f32, size: (u8, u8)) -> Piece {
        Piece {
            sprite,
            origin: vec2(x, y) * TILE,
            size,
            facing: Facing::Down,
            dirty: 0.0,
        }
    }
    fn facing(mut self, f: Facing) -> Piece {
        self.facing = f;
        self
    }
    fn dirty(mut self, d: f32) -> Piece {
        self.dirty = d;
        self
    }
}

/// A person's movement in the demo.
enum Motion {
    Still(Vec2, Facing, Pose),
    Path(Vec<Vec2>, f32, Pose),
}

struct Actor {
    look: PersonLook,
    motion: Motion,
    emote: Option<(Emote, f32)>,
    selected: bool,
}

fn tile_c(x: f32, y: f32) -> Vec2 {
    vec2(x + 0.5, y + 0.5) * TILE
}

impl Actor {
    fn pose(&self, t: f32) -> PersonPose {
        match &self.motion {
            Motion::Still(feet, facing, pose) => PersonPose {
                feet: *feet,
                facing: *facing,
                pose: *pose,
                t,
            },
            Motion::Path(pts, speed, pose) => {
                // path length around the loop
                let n = pts.len();
                let lens: Vec<f32> = (0..n).map(|i| pts[i].distance(pts[(i + 1) % n])).collect();
                let total: f32 = lens.iter().sum();
                let mut d = (t * speed).rem_euclid(total);
                let mut i = 0;
                while d > lens[i] {
                    d -= lens[i];
                    i += 1;
                }
                let a = pts[i];
                let b = pts[(i + 1) % n];
                let dir = (b - a).normalize_or_zero();
                PersonPose {
                    feet: a + dir * d,
                    facing: Facing::from_dir(dir),
                    pose: *pose,
                    t,
                }
            }
        }
    }
}

fn is_wall(map: &Map, x: i32, y: i32) -> bool {
    if x < 0 || y < 0 {
        return false;
    }
    matches!(
        map.get(x as usize, y as usize),
        Some(Tile::Wall | Tile::Window | Tile::Door)
    )
}

fn is_floor(map: &Map, x: i32, y: i32) -> bool {
    if x < 0 || y < 0 {
        return false;
    }
    matches!(map.get(x as usize, y as usize), Some(t) if !matches!(t, Tile::Wall | Tile::Window | Tile::Door))
}

fn wall_kind(t: Tile) -> WallKind {
    match t {
        Tile::Window => WallKind::Window,
        Tile::Door => WallKind::Door,
        _ => WallKind::Plain,
    }
}

fn floor_kind(x: usize, y: usize) -> &'static str {
    if (11..=14).contains(&y) && x >= 15 {
        "floor_tile"
    } else if (1..=5).contains(&y) && x >= 22 {
        "floor_carpet"
    } else {
        "floor_wood"
    }
}

fn build_furniture(map: &Map, stress: bool) -> Vec<Piece> {
    let mut v = Vec::new();
    for y in 0..map.height {
        for x in 0..map.width {
            let (fx, fy) = (x as f32, y as f32);
            match map.get(x, y).unwrap() {
                Tile::Table => {
                    v.push(
                        Piece::at("table_round_2", fx, fy, (1, 1)).dirty(if x == 11 && y == 5 {
                            0.8
                        } else {
                            0.0
                        }),
                    );
                    v.push(Piece::at("pendant_lamp", fx, fy, (1, 1)));
                }
                Tile::Chair => {
                    let right_is_table = map.get(x + 1, y) == Some(Tile::Table);
                    let f = if right_is_table {
                        Facing::Right
                    } else {
                        Facing::Left
                    };
                    v.push(Piece::at("chair", fx, fy, (1, 1)).facing(f));
                }
                Tile::Counter => {
                    let left = map.get(x - 1, y) != Some(Tile::Counter);
                    let right = map.get(x + 1, y) != Some(Tile::Counter);
                    let id = if x == 19 {
                        "register"
                    } else if left || right {
                        "counter_corner"
                    } else {
                        "bar_counter"
                    };
                    v.push(Piece::at(id, fx, fy, (1, 1)));
                }
                Tile::CoffeeMachine => {
                    let id = if map.get(x - 1, y) == Some(Tile::CoffeeMachine) {
                        "espresso_grinder"
                    } else {
                        "coffee_machine"
                    };
                    v.push(Piece::at(id, fx, fy, (1, 1)));
                }
                Tile::Stove => {
                    let id = if map.get(x - 1, y) == Some(Tile::Stove) {
                        "oven"
                    } else {
                        "stove"
                    };
                    v.push(Piece::at(id, fx, fy, (1, 1)));
                }
                Tile::Sink => {
                    let id = if map.get(x - 1, y) == Some(Tile::Sink) {
                        "dish_rack"
                    } else {
                        "sink"
                    };
                    v.push(Piece::at(id, fx, fy, (1, 1)).dirty(0.6));
                }
                _ => {}
            }
        }
    }
    // decor
    v.extend([
        Piece::at("fridge", 22.0, 13.0, (1, 1)),
        Piece::at("trash_bin", 23.0, 13.0, (1, 1)).dirty(0.7),
        Piece::at("drone_dock", 27.0, 13.0, (1, 1)),
        Piece::at("menu_holo_board", 15.0, 10.0, (1, 1)),
        Piece::at("rug", 10.0, 4.0, (5, 3)),
        Piece::at("rug", 1.0, 12.0, (3, 2)),
        Piece::at("sofa", 24.0, 2.0, (2, 1)),
        Piece::at("armchair", 23.0, 4.0, (1, 1)),
        Piece::at("armchair", 26.0, 4.0, (1, 1)),
        Piece::at("table_square_4", 24.0, 4.0, (2, 1)),
        Piece::at("floor_lamp", 27.0, 2.0, (1, 1)),
        Piece::at("floor_lamp", 22.0, 2.0, (1, 1)),
        Piece::at("plant_tall", 28.0, 1.0, (1, 1)),
        Piece::at("plant_tall", 1.0, 1.0, (1, 1)),
        Piece::at("plant_small", 28.0, 7.0, (1, 1)),
        Piece::at("plant_small", 8.0, 9.0, (1, 1)),
        Piece::at("plant_small", 14.0, 13.0, (1, 1)),
        Piece::at("bookshelf", 9.0, 1.0, (2, 1)),
        Piece::at("grandma_photo", 8.0, 1.0, (1, 1)),
        Piece::at("radio", 8.0, 1.0, (1, 1)),
        Piece::at("jukebox", 7.0, 14.0, (1, 1)),
        Piece::at("window_seat", 14.0, 1.0, (4, 1)),
        Piece::at("coat_rack", 4.0, 14.0, (1, 1)),
        Piece::at("candle", 27.0, 9.0, (1, 1)),
        Piece::at("bucket", 6.0, 12.0, (1, 1)).dirty(0.5),
        Piece::at("pendant_lamp", 17.0, 13.0, (1, 1)),
        Piece::at("pendant_lamp", 20.0, 13.0, (1, 1)),
        Piece::at("pendant_lamp", 18.0, 10.0, (1, 1)),
        Piece::at("pendant_lamp", 23.0, 10.0, (1, 1)),
        Piece::at("mystery_thing", 1.0, 9.0, (1, 1)),
    ]);
    for x in [17.0, 19.0, 21.0, 23.0, 25.0] {
        v.push(Piece::at("stool", x, 9.0, (1, 1)));
    }
    if stress {
        // top up to ~150 objects
        let mut i = 0;
        while v.len() < 150 {
            let x = 2.0 + (i % 26) as f32;
            let y = 11.0 + (i / 26) as f32 % 3.0;
            v.push(Piece::at(
                if i % 2 == 0 { "plant_small" } else { "stool" },
                x,
                y,
                (1, 1),
            ));
            i += 1;
        }
    }
    v
}

fn build_actors(stress: bool) -> Vec<Actor> {
    let still = |feet: Vec2, f: Facing, p: Pose| Motion::Still(feet, f, p);
    let chair = |x: f32, y: f32, f: Facing| sit_anchor(vec2(x, y) * TILE, f);
    let mut v = vec![
        // staff
        Actor {
            look: PersonLook::random(101, true),
            motion: still(
                tile_c(16.0, 12.0) + vec2(0.0, 10.0),
                Facing::Down,
                Pose::Work,
            ),
            emote: Some((Emote::Coffee, 0.0)),
            selected: false,
        },
        Actor {
            look: PersonLook::random(102, true),
            motion: still(
                tile_c(18.0, 12.0) + vec2(0.0, 10.0),
                Facing::Down,
                Pose::Work,
            ),
            emote: None,
            selected: false,
        },
        Actor {
            look: PersonLook::preset("player"),
            motion: still(tile_c(19.0, 11.0) + vec2(0.0, -4.0), Facing::Up, Pose::Idle),
            emote: Some((Emote::Money, 1.5)),
            selected: true,
        },
        Actor {
            look: PersonLook::random(104, true),
            motion: Motion::Path(
                vec![
                    tile_c(15.0, 9.0),
                    tile_c(7.0, 9.0),
                    tile_c(7.0, 3.6),
                    tile_c(15.0, 3.6),
                ],
                70.0,
                Pose::Carry,
            ),
            emote: None,
            selected: false,
        },
        // guests at the tables
        Actor {
            look: PersonLook::preset("mirek"),
            motion: still(chair(3.0, 2.0, Facing::Right), Facing::Right, Pose::Sit),
            emote: Some((Emote::Coffee, 0.4)),
            selected: false,
        },
        Actor {
            look: PersonLook::preset("vera"),
            motion: still(chair(5.0, 2.0, Facing::Left), Facing::Left, Pose::Sit),
            emote: Some((Emote::Note, 2.0)),
            selected: false,
        },
        Actor {
            look: PersonLook::preset("filip"),
            motion: still(chair(13.0, 5.0, Facing::Left), Facing::Left, Pose::Sit),
            emote: Some((Emote::Music, 0.9)),
            selected: false,
        },
        Actor {
            look: PersonLook::preset("hana"),
            motion: still(chair(11.0, 2.0, Facing::Right), Facing::Right, Pose::Sit),
            emote: Some((Emote::Zzz, 0.2)),
            selected: false,
        },
        Actor {
            look: PersonLook::preset("guest_333"),
            motion: still(chair(19.0, 2.0, Facing::Right), Facing::Right, Pose::Sit),
            emote: None,
            selected: false,
        },
        Actor {
            look: PersonLook::preset("lori"),
            motion: still(vec2(24.62, 2.86) * TILE, Facing::Down, Pose::Sit),
            emote: Some((Emote::Heart, 1.1)),
            selected: false,
        },
        Actor {
            look: PersonLook::preset("radek"),
            motion: still(
                tile_c(20.0, 9.0) + vec2(0.0, -10.0),
                Facing::Down,
                Pose::Idle,
            ),
            emote: Some((Emote::Exclamation, 2.6)),
            selected: false,
        },
        Actor {
            look: PersonLook::random(7, false),
            motion: Motion::Path(
                vec![
                    tile_c(2.0, 14.0),
                    tile_c(2.0, 11.0),
                    tile_c(8.0, 11.0),
                    tile_c(8.0, 7.0),
                    tile_c(2.0, 7.0),
                ],
                55.0,
                Pose::Walk,
            ),
            emote: None,
            selected: false,
        },
        Actor {
            look: PersonLook::random(8, false),
            motion: still(chair(3.0, 5.0, Facing::Right), Facing::Right, Pose::Sit),
            emote: Some((Emote::Angry, 3.0)),
            selected: false,
        },
        // Anwar with his laptop (the laptop is drawn separately, see main)
        Actor {
            look: PersonLook::preset("anwar"),
            motion: still(chair(11.0, 8.0, Facing::Right), Facing::Right, Pose::Sit),
            emote: Some((Emote::Coffee, 1.9)),
            selected: false,
        },
        Actor {
            look: PersonLook::random(9, false),
            motion: still(chair(5.0, 8.0, Facing::Left), Facing::Left, Pose::Sit),
            emote: None,
            selected: false,
        },
        Actor {
            look: PersonLook::preset("shiftella_bot"),
            motion: still(tile_c(27.0, 12.0), Facing::Down, Pose::Idle),
            emote: None,
            selected: false,
        },
    ];
    if stress {
        for i in 0..32u64 {
            let y = 6.3 + (i % 4) as f32 * 0.9;
            let x0 = 2.0 + (i / 4) as f32 * 3.0;
            v.push(Actor {
                look: PersonLook::random(500 + i, i % 5 == 0),
                motion: Motion::Path(
                    vec![tile_c(x0, y), tile_c(x0 + 6.0, y)],
                    40.0 + i as f32,
                    Pose::Walk,
                ),
                emote: if i % 3 == 0 {
                    Some((Emote::Coffee, i as f32))
                } else {
                    None
                },
                selected: false,
            });
        }
    }
    v
}

#[macroquad::main(window_conf)]
async fn main() {
    let demo = std::env::var("SHIFTELLA_DEMO").unwrap_or_default();
    let shot = std::env::var("SHIFTELLA_SHOT").ok();
    let t0 = env_f("SHIFTELLA_T").unwrap_or(6.0);
    match demo.as_str() {
        "portraits" => return portraits_demo(shot, t0).await,
        "people" => return people_demo(shot, t0).await,
        "furniture" => return furniture_demo(shot, t0).await,
        "themes" => return themes_demo(shot, t0).await,
        "polaroid" => return polaroid_demo(shot, t0).await,
        _ => {}
    }
    let season = match demo.as_str() {
        "season_autumn" => Some(Season::LateAutumn),
        "season_frost" => Some(Season::Frost),
        "season_festive" => Some(Season::Festive),
        "season_snow" => Some(Season::FirstSnow),
        _ => None,
    };
    let look = season.map_or(SeasonLook::NONE, |s| s.look());
    let handson = demo == "handson";
    let moments = demo == "moments";
    let theme = std::env::var("SHIFTELLA_THEME")
        .ok()
        .and_then(|id| DecorTheme::from_id(&id))
        .map(|th| th.style(season.unwrap_or_default()))
        .unwrap_or(if season.is_some() || handson || moments {
            ThemeStyle::GRANDMA
        } else {
            ThemeStyle::DEFAULT
        });
    set_decor(theme);
    let winter_lights = theme == ThemeStyle::WINTER_LIGHTS;
    let garlands_p =
        env_f("SHIFTELLA_GARLANDS").unwrap_or(if look.festive > 0.5 { 1.0 } else { 0.0 });
    let cozy = env_f("SHIFTELLA_COZY").unwrap_or(if season.is_some() { 0.85 } else { 0.5 });

    let stress = std::env::var("SHIFTELLA_STRESS").is_ok();
    let blackout = std::env::var("SHIFTELLA_BLACKOUT").is_ok();
    let night = env_f("SHIFTELLA_NIGHT");
    let train_fixed = env_f("SHIFTELLA_TRAIN");

    let map = Map::from_layout(CAFE_LAYOUT);
    let mut furniture = build_furniture(&map, stress);
    if look.festive > 0.01 {
        furniture.retain(|p| !(p.sprite == "plant_small" && p.origin == vec2(28.0, 7.0) * TILE));
        furniture.push(Piece::at("spruce_tree", 27.0, 6.0, (1, 1)));
        furniture.push(Piece::at("advent_wreath", 22.0, 10.0, (1, 1)).dirty(
            if season == Some(Season::FirstSnow) {
                0.5
            } else {
                0.25
            },
        ));
        if garlands_p < 1.0 {
            furniture.push(Piece::at("garland_box", 14.0, 9.0, (1, 1)));
        }
    }
    if season.is_some() || handson {
        furniture.push(Piece::at("tip_jar", 18.0, 10.0, (1, 1)).dirty(0.45));
    }
    let mut actors = build_actors(stress);
    if handson {
        let emotes = [
            (4, Emote::Sparkle),
            (5, Emote::Smile),
            (7, Emote::Sleepy),
            (9, Emote::LikedRecord),
            (13, Emote::WarmHeart),
        ];
        for a in actors.iter_mut() {
            a.emote = None;
        }
        for (i, e) in emotes {
            actors[i].emote = Some((e, 0.0));
        }
    }
    // passers-by on the front pavement (breath in the cold)
    let mut outside: Vec<Actor> = Vec::new();
    if season.is_some() {
        outside.push(Actor {
            look: PersonLook::random(41, false),
            motion: Motion::Path(
                vec![tile_c(-0.5, 16.3), tile_c(30.0, 16.3)],
                46.0,
                Pose::Walk,
            ),
            emote: None,
            selected: false,
        });
        outside.push(Actor {
            look: PersonLook::random(57, false),
            motion: Motion::Still(tile_c(5.0, 16.4), Facing::Right, Pose::Idle),
            emote: None,
            selected: false,
        });
        outside.push(Actor {
            look: PersonLook::random(63, false),
            motion: Motion::Still(tile_c(6.0, 16.45), Facing::Left, Pose::Idle),
            emote: None,
            selected: false,
        });
    }
    // and on the far sidewalk behind the back wall
    let street_walkers: Vec<Actor> = if season.is_some() {
        vec![
            Actor {
                look: PersonLook::random(71, false),
                motion: Motion::Path(
                    vec![
                        vec2(32.0 * TILE, -2.25 * TILE),
                        vec2(-4.0 * TILE, -2.25 * TILE),
                    ],
                    40.0,
                    Pose::Walk,
                ),
                emote: None,
                selected: false,
            },
            Actor {
                look: PersonLook::random(88, false),
                motion: Motion::Still(vec2(9.6 * TILE, -2.1 * TILE), Facing::Down, Pose::Idle),
                emote: None,
                selected: false,
            },
        ]
    } else {
        Vec::new()
    };
    let weather = Weather {
        intensity: match season {
            Some(Season::FirstSnow) => 0.0,
            Some(Season::Festive) => 0.5,
            Some(Season::Frost) => 0.7,
            _ => 1.0,
        },
        ..Default::default()
    };
    let snowfall = Snowfall::from_look(&look);

    // window glass for rain and light
    let mut glass = Vec::new();
    for y in 0..map.height {
        for x in 0..map.width {
            if map.get(x, y) == Some(Tile::Window) {
                glass.push(window_glass_rect(vec2(x as f32, y as f32)));
            }
        }
    }
    let mut lighting = Lighting::new();
    lighting.set_windows(glass.clone());
    lighting.coziness = cozy;
    // festive garlands: over the windows, along the counter; fairy lights on the back wall
    let window_garlands = Garlands::over_windows(&glass, GarlandStyle::WarmBulbs);
    let counter_garlands = Garlands::new(
        Garlands::along_counter(16.0 * TILE, 27.0 * TILE, 11.0 * TILE, 5),
        GarlandStyle::Bough,
    );
    let fairy = Garlands::new(
        (0..7)
            .map(|i| {
                let x = 1.0 * TILE + i as f32 * 4.0 * TILE;
                GarlandSpan::new(vec2(x, -26.0), vec2(x + 4.0 * TILE, -26.0), 10.0)
            })
            .collect(),
        GarlandStyle::FairyLights,
    );
    let all_garlands = [&window_garlands, &counter_garlands];
    let counter_garlands_q = counter_garlands.clone();

    // cache of the floor and the tall walls
    let mut statics = StaticLayer::new();
    let static_rect = Rect::new(
        -TILE,
        -2.0 * TILE,
        (map.width as f32 + 2.0) * TILE,
        (map.height as f32 + 4.0) * TILE,
    );
    let mut low_walls = Vec::new();
    statics.build(static_rect, 2.0, || {
        // wet pavement in front of the facade and the door
        for y in map.height..map.height + 2 {
            for x in -1..=map.width as i32 {
                draw_floor("pavement", vec2(x as f32, y as f32));
                draw_floor_season("pavement", vec2(x as f32, y as f32), &look);
            }
        }
        for y in 0..map.height {
            for x in 0..map.width {
                let tile = map.get(x, y).unwrap();
                if !matches!(tile, Tile::Wall | Tile::Window) {
                    let kind = floor_kind(x, y);
                    // the hall wears the theme, the bar & kitchen keep the base look
                    if kind == "floor_tile" {
                        with_decor(ThemeStyle::DEFAULT, || {
                            draw_floor(kind, vec2(x as f32, y as f32))
                        });
                    } else {
                        draw_floor(kind, vec2(x as f32, y as f32));
                    }
                }
            }
        }
        for x in [1.0, 2.0] {
            draw_doorstep_season(vec2(x, 14.0), &look);
        }
        for y in 0..map.height {
            for x in 0..map.width {
                let tile = map.get(x, y).unwrap();
                if matches!(tile, Tile::Wall | Tile::Window | Tile::Door) {
                    let m = WallMask::compute(
                        x as i32,
                        y as i32,
                        |a, b| is_wall(&map, a, b),
                        |a, b| is_floor(&map, a, b),
                    );
                    let pos = vec2(x as f32, y as f32);
                    if m.is_low() {
                        low_walls.push((pos, m, wall_kind(tile)));
                    } else {
                        draw_wall(pos, m, wall_kind(tile));
                    }
                }
            }
        }
    });

    let street = Rect::new(
        -8.0 * TILE,
        -10.0 * TILE,
        (map.width as f32 + 16.0) * TILE,
        8.55 * TILE,
    );
    let street_lights = outside_lights(street);

    let screen = Rect::new(0.0, 0.0, screen_width(), screen_height());
    // overview: the café + a strip of street above the back wall
    let mut cam = WorldCamera::fit(vec2(32.0, 21.6) * TILE, screen);
    cam.center = vec2(15.0, 5.4) * TILE;
    if let Some(z) = env_f("SHIFTELLA_ZOOM") {
        cam.zoom = z;
    }
    if let Ok(c) = std::env::var("SHIFTELLA_CENTER") {
        let p: Vec<f32> = c.split(',').filter_map(|s| s.trim().parse().ok()).collect();
        if p.len() == 2 {
            cam.center = vec2(p[0], p[1]) * TILE;
        }
    }

    let mut frame = 0u32;
    let mut stats = (0.0f64, 0u32, Instant::now());
    loop {
        let t = if shot.is_some() {
            t0 + frame as f32 / 60.0
        } else {
            t0 + get_time() as f32
        };
        cam.update();
        let cpu = Instant::now();

        let world_cam = cam.to_camera2d();
        set_camera(&world_cam);
        clear_background(pal::NIGHT_0);
        // "dollhouse stage": a soft violet pool under the café
        let ms = map.size_px();
        radial(
            ms.x / 2.0,
            ms.y / 2.0 + TILE,
            ms.x * 0.75,
            ms.y * 0.9,
            pal::VIOLET_NIGHT_1,
            alpha(pal::NIGHT_0, 0.0),
        );

        let train_p = train_fixed.unwrap_or(((t % 16.0) - 3.0) / 7.0);
        draw_outside(street, t);
        draw_outside_season(street, &look, t);
        draw_train(street, train_p, t);
        let sign_pos = vec2(21.0 * TILE, street.y + street.h - 0.35 * TILE);
        draw_holo_sign(sign_pos, "Café Nocturne", t);
        draw_sign_season(sign_pos, "Café Nocturne", &look, t);
        let street_poses: Vec<PersonPose> = street_walkers.iter().map(|a| a.pose(t)).collect();
        for (a, pose) in street_walkers.iter().zip(&street_poses) {
            draw_person(&a.look, pose);
        }
        weather.draw_rain(street, t);
        snowfall.draw(street, t);
        draw_falling_leaves(street, look.leaves * 0.7, 0.25, t);
        statics.draw();
        for g in &glass {
            weather.draw_window_rain(*g, t);
            draw_window_season(*g, &look, t);
        }
        if winter_lights {
            fairy.draw(1.0, t);
        }
        // over the windows: right after the static walls; the counter's: in the queue
        window_garlands.draw(garlands_p, t);

        let mut q = RenderQueue::new();
        let mut states = Vec::with_capacity(furniture.len());
        for p in &furniture {
            let lamp = matches!(
                p.sprite,
                "floor_lamp"
                    | "pendant_lamp"
                    | "jukebox"
                    | "menu_holo_board"
                    | "register"
                    | "coffee_machine"
                    | "radio"
                    | "stove"
                    | "oven"
                    | "fridge"
            );
            let st = FurnitureState {
                active: if blackout {
                    !lamp && !(moments && p.sprite.starts_with("table"))
                } else {
                    p.sprite != "drone_dock" || (t % 10.0) < 6.0
                },
                dirty: p.dirty,
                t,
                facing: p.facing,
            };
            states.push(st);
            q.push_furniture(p.sprite, p.origin, p.size, st);
        }
        // Anwar's laptop on table (12, 8): after the table, before Anwar himself
        let laptop_at = vec2(12.42, 8.5) * TILE - vec2(0.0, 24.0);
        q.push_custom(9.0 * TILE + 0.01, move || {
            draw_laptop(laptop_at, Facing::Right, t)
        });
        for (pos, m, k) in &low_walls {
            q.push_wall(*pos, *m, *k);
        }
        let poses: Vec<PersonPose> = actors.iter().map(|a| a.pose(t)).collect();
        for (a, pose) in actors.iter().zip(&poses) {
            if a.selected {
                q.push_selection(pose);
            }
            q.push_person(&a.look, *pose);
        }
        let cg = &counter_garlands_q;
        q.push_custom(11.0 * TILE + 0.6, move || cg.draw(garlands_p, t));
        let out_poses: Vec<PersonPose> = outside.iter().map(|a| a.pose(t)).collect();
        for (a, pose) in outside.iter().zip(&out_poses) {
            q.push_person(&a.look, *pose);
        }
        if handson {
            // served cups with latte art on the tables
            for (tx, ty, g, art) in [
                (4.0, 2.0, CupGlyph::Latte, Some(LatteArt::Heart)),
                (12.0, 2.0, CupGlyph::Tea, None),
                (12.0, 5.0, CupGlyph::Latte, Some(LatteArt::Rosetta)),
                (4.0, 8.0, CupGlyph::Chocolate, None),
                (20.0, 2.0, CupGlyph::MulledWine, None),
            ] {
                let p = vec2(tx * TILE + 26.0, ty * TILE + 34.0 - 28.0);
                q.push_custom(ty * TILE + TILE + 0.02, move || {
                    draw_served_cup(p, g, art, t)
                });
            }
        }
        if moments {
            // the station cat by the door
            q.push_custom(13.75 * TILE, move || {
                draw_cat(vec2(2.4, 13.7) * TILE, CatPose::Sit, t, 0.6)
            });
        }
        // candles being lit table by table in the blackout
        let lighting_up: Vec<(Vec2, f32)> = if moments && blackout {
            furniture
                .iter()
                .filter_map(|p| table_candle_anchor(p.sprite, p.origin, p.size))
                .enumerate()
                .map(|(i, a)| (a, (t - 5.0) * 1.0 - i as f32 * 0.45))
                .collect()
        } else {
            Vec::new()
        };
        for (a, age) in &lighting_up {
            let (a, age) = (*a, *age);
            if age > 0.0 {
                q.push_custom(a.y + 70.0, move || draw_candle_lighting(a, age, t));
            }
        }
        q.flush();

        // light
        let ambient = if blackout {
            ambient_blackout()
        } else {
            ambient_for(night.unwrap_or(((t / 90.0) % 1.0) * 0.9 + 0.05))
        };
        lighting.begin(&world_cam, ambient);
        lighting.set_time(t);
        lighting.add_window_light(if blackout { 0.6 } else { 1.0 });
        lighting.train_sweep(train_p, street);
        for (p, st) in furniture.iter().zip(&states) {
            for l in furniture_lights(p.sprite, p.origin, p.size, st) {
                lighting.add_light(l);
            }
        }
        if !blackout {
            // the lamp over the door lights the wet pavement
            lighting.add_light(
                Light::new(vec2(2.0, 16.4) * TILE, 2.6 * TILE, pal::AMBER, 0.8).flicker(0.03),
            );
            for l in &street_lights {
                lighting.add_light(*l);
            }
            lighting.add_light(
                Light::new(
                    vec2(21.0 * TILE, street.y + street.h - 1.8 * TILE),
                    3.0 * TILE,
                    pal::INDIGO_GLOW,
                    0.7,
                )
                .flicker(0.1),
            );
        }
        for g in all_garlands {
            for l in g.lights(garlands_p) {
                lighting.add_light(l);
            }
        }
        if winter_lights {
            for l in fairy.lights(1.0) {
                lighting.add_light(l);
            }
        }
        for (a, age) in &lighting_up {
            if *age > 0.0 {
                lighting.add_light(candle_lighting_light(*a, *age));
            }
        }
        for (a, pose) in actors.iter().zip(&poses) {
            if blackout && pose.pose == Pose::Sit {
                lighting.add_light(Light::phone(pose.feet - vec2(0.0, 40.0)));
            }
            if a.selected {
                lighting.add_light(Light::new(pose.feet, 0.8 * TILE, pal::INDIGO_GLOW, 0.5));
            }
            if a.look == PersonLook::preset("anwar") && pose.pose == Pose::Sit {
                lighting.add_light(Light::phone(vec2(12.2, 8.5) * TILE - vec2(0.0, 34.0)));
            }
        }
        let markers = if moments {
            demo_markers(&poses, t)
        } else {
            Vec::new()
        };
        for m in &markers {
            lighting.add_light(moment_marker_light(m, t));
        }
        lighting.apply();
        lighting.draw_dust_motes(t);
        for g in all_garlands {
            g.draw_glow(garlands_p, t);
        }
        if winter_lights {
            fairy.draw_glow(1.0, t);
        }
        // hang spots while the garlands go up
        if garlands_p < 1.0 && look.festive > 0.5 {
            for (gi, g) in all_garlands.iter().enumerate() {
                for (i, spot) in g.hang_spots().iter().enumerate() {
                    let k = g.span_progress(i, garlands_p);
                    if k <= 0.0 && (gi * 31 + i) % 5 == 0 {
                        let m = MomentMarker::new(
                            *spot + vec2(0.0, MARKER_LIFT - 4.0),
                            MomentMarkerKind::Garland,
                            0.0,
                        );
                        draw_moment_marker(&m, t);
                    }
                }
            }
        }
        for (a, pose) in street_walkers.iter().zip(&street_poses) {
            draw_breath(&a.look, pose, t, look.breath);
        }
        for (a, pose) in outside.iter().zip(&out_poses) {
            draw_breath(&a.look, pose, t, look.breath);
        }
        draw_falling_leaves(
            Rect::new(-TILE, 15.6 * TILE, 32.0 * TILE, 2.4 * TILE),
            look.leaves * 0.6,
            0.2,
            t + 4.0,
        );
        for m in &markers {
            draw_moment_marker(m, t);
        }
        if handson {
            handson_overlays(&actors, &poses, t);
        }

        // emotes over the light
        for (a, pose) in actors.iter().zip(&poses) {
            if let Some((e, phase)) = a.emote {
                let cycle = 6.0;
                let age = (t + phase * 1.7) % cycle;
                if age < 3.5 {
                    draw_emote_ex(emote_anchor(&a.look, pose), e, age, t);
                }
            }
        }
        let cpu_ms = cpu.elapsed().as_secs_f64() * 1000.0;

        set_default_camera();
        if handson {
            handson_legend(t);
        }
        if shot.is_none() {
            stats.0 += cpu_ms;
            stats.1 += 1;
            if stats.2.elapsed().as_secs_f32() > 2.0 {
                println!(
                    "people {} furniture {}: {:.1} fps, render CPU {:.2} ms/frame",
                    actors.len(),
                    furniture.len(),
                    stats.1 as f32 / stats.2.elapsed().as_secs_f32(),
                    stats.0 / stats.1 as f64
                );
                stats = (0.0, 0, Instant::now());
            }
        }

        frame += 1;
        if let Some(path) = &shot
            && frame == 40
        {
            get_screen_data().export_png(path);
            println!("saved {path} (render CPU {cpu_ms:.2} ms)");
            break;
        }
        next_frame().await;
    }
}

// ───────────────────────── galleries ─────────────────────────

/// A camera in logical screen points (for portraits, like the UI).
fn ui_camera() -> Camera2D {
    let s = screen_dpi_scale();
    let (w, h) = (screen_width() / s, screen_height() / s);
    Camera2D {
        target: vec2(w / 2.0, h / 2.0),
        zoom: vec2(2.0 / w, 2.0 / h),
        ..Default::default()
    }
}

async fn finish_frame(shot: &Option<String>, frame: &mut u32) -> bool {
    *frame += 1;
    if let Some(path) = shot
        && *frame == 40
    {
        get_screen_data().export_png(path);
        println!("saved {path}");
        return true;
    }
    next_frame().await;
    false
}

async fn portraits_demo(shot: Option<String>, t0: f32) {
    let mut frame = 0;
    loop {
        let t = if shot.is_some() {
            t0 + frame as f32 / 60.0
        } else {
            t0 + get_time() as f32
        };
        clear_background(pal::NIGHT_1);
        set_camera(&ui_camera());
        let s = screen_dpi_scale();
        let (w, h) = (screen_width() / s, screen_height() / s);
        let page = env_f("SHIFTELLA_PAGE").unwrap_or(0.0) as usize;
        let all = PersonLook::PRESETS;
        let ids = &all[(page * 6).min(all.len())..(page * 6 + 6).min(all.len())];
        let cols = Expression::ALL.len() as f32;
        let label_w = 96.0;
        let cell = ((w - label_w - 16.0) / cols).min((h - 34.0) / ids.len() as f32);
        for (ci, e) in Expression::ALL.iter().enumerate() {
            world_text(
                &format!("{e:?}"),
                vec2(label_w + (ci as f32 + 0.5) * cell, 18.0),
                11.0,
                pal::INDIGO_SOFT,
                true,
            );
        }
        for (ri, id) in ids.iter().enumerate() {
            let look = PersonLook::preset(id);
            let y = 26.0 + ri as f32 * cell;
            world_text(
                id,
                vec2(8.0, y + cell * 0.55),
                12.0,
                Color::from_hex(0xece8f7),
                false,
            );
            for (ci, e) in Expression::ALL.iter().enumerate() {
                let r = Rect::new(
                    label_w + ci as f32 * cell + 2.0,
                    y + 2.0,
                    cell - 4.0,
                    cell - 4.0,
                );
                // a "glass" frame, like in the UI
                draw::rrect_v(
                    r.x,
                    r.y,
                    r.w,
                    r.h,
                    8.0,
                    Color::from_hex(0x2a2550),
                    Color::from_hex(0x1a1733),
                );
                draw::rrect_v(
                    r.x + 1.0,
                    r.y + 1.0,
                    r.w - 2.0,
                    6.0,
                    3.0,
                    alpha(WHITE, 0.08),
                    alpha(WHITE, 0.0),
                );
                draw_portrait(&look, *e, r, t + ci as f32 * 0.37);
            }
        }
        if finish_frame(&shot, &mut frame).await {
            break;
        }
    }
}

async fn people_demo(shot: Option<String>, t0: f32) {
    let mut frame = 0;
    let looks: Vec<PersonLook> = PersonLook::PRESETS
        .iter()
        .map(|id| PersonLook::preset(id))
        .chain((0..10).map(|s| PersonLook::random(s * 7 + 3, s % 3 == 0)))
        .collect();
    let lit = std::env::var("SHIFTELLA_LIT").is_ok();
    let mut lighting = Lighting::new();
    loop {
        let t = if shot.is_some() {
            t0 + frame as f32 / 60.0
        } else {
            t0 + get_time() as f32
        };
        let cols = 10.0;
        let ww = cols * 1.2 * TILE;
        let wh = 7.0 * 1.6 * TILE;
        let zoom = (screen_width() / ww).min(screen_height() / wh);
        let cam = Camera2D {
            target: vec2(ww / 2.0, wh / 2.0),
            zoom: vec2(2.0 * zoom / screen_width(), 2.0 * zoom / screen_height()),
            ..Default::default()
        };
        set_camera(&cam);
        clear_background(pal::NIGHT_1);
        for y in 0..12 {
            for x in 0..13 {
                draw_floor("floor_wood", vec2(x as f32, y as f32));
            }
        }
        let rows: [(Facing, Pose); 7] = [
            (Facing::Down, Pose::Idle),
            (Facing::Down, Pose::Walk),
            (Facing::Right, Pose::Walk),
            (Facing::Up, Pose::Carry),
            (Facing::Left, Pose::Sit),
            (Facing::Down, Pose::Work),
            (Facing::Down, Pose::Sit),
        ];
        let mut q = RenderQueue::new();
        for (ri, (f, p)) in rows.iter().enumerate() {
            for (ci, look) in looks.iter().enumerate().skip((ri % 2) * 10).take(10) {
                let feet = vec2(
                    (ci % 10) as f32 * 1.2 * TILE + 0.6 * TILE,
                    ri as f32 * 1.6 * TILE + 1.45 * TILE,
                );
                if *p == Pose::Sit {
                    let o = feet - vec2(0.5 * TILE, 0.6 * TILE);
                    q.push_furniture(
                        "chair",
                        o,
                        (1, 1),
                        FurnitureState {
                            facing: *f,
                            ..Default::default()
                        },
                    );
                    let pose = PersonPose {
                        feet: sit_anchor(o, *f),
                        facing: *f,
                        pose: *p,
                        t,
                    };
                    q.push_person(look, pose);
                } else {
                    q.push_person(
                        look,
                        PersonPose {
                            feet,
                            facing: *f,
                            pose: *p,
                            t: t + ci as f32 * 0.21,
                        },
                    );
                }
            }
        }
        q.flush();
        if lit {
            lighting.begin(&cam, ambient_for(0.4));
            lighting.add_light(Light::new(
                vec2(ww * 0.3, wh * 0.4),
                5.0 * TILE,
                pal::AMBER,
                1.0,
            ));
            lighting.apply();
        }
        for (ci, look) in looks.iter().enumerate().take(8) {
            let feet = vec2(ci as f32 * 1.2 * TILE + 0.6 * TILE, 1.45 * TILE);
            let pose = PersonPose {
                feet,
                facing: Facing::Down,
                pose: Pose::Idle,
                t,
            };
            draw_emote_ex(emote_anchor(look, &pose), Emote::ALL[ci], 1.0, t);
        }
        if finish_frame(&shot, &mut frame).await {
            break;
        }
    }
}

async fn furniture_demo(shot: Option<String>, t0: f32) {
    let mut frame = 0;
    let lit = std::env::var("SHIFTELLA_LIT").is_ok();
    let mut lighting = Lighting::new();
    loop {
        let t = if shot.is_some() {
            t0 + frame as f32 / 60.0
        } else {
            t0 + get_time() as f32
        };
        let cols = 8;
        let (cw, ch) = (3.0 * TILE, 3.0 * TILE);
        let rows = FURNITURE_IDS.len().div_ceil(cols) + 1;
        let ww = cols as f32 * cw;
        let wh = rows as f32 * ch;
        let zoom = (screen_width() / ww).min(screen_height() / wh);
        let cam = Camera2D {
            target: vec2(ww / 2.0, wh / 2.0),
            zoom: vec2(2.0 * zoom / screen_width(), 2.0 * zoom / screen_height()),
            ..Default::default()
        };
        set_camera(&cam);
        clear_background(pal::NIGHT_1);
        for y in 0..(wh / TILE) as i32 {
            for x in 0..(ww / TILE) as i32 {
                let kinds = ["floor_wood", "floor_tile", "pavement", "floor_carpet"];
                draw_floor(
                    kinds[((x / 3 + y / 3) % 4) as usize],
                    vec2(x as f32, y as f32),
                );
            }
        }
        let mut q = RenderQueue::new();
        let mut placed = Vec::new();
        for (i, id) in FURNITURE_IDS
            .iter()
            .chain(["unknown_id"].iter())
            .enumerate()
        {
            let size: (u8, u8) = match *id {
                "sofa" | "bookshelf" | "window_seat" | "table_square_4" => (2, 1),
                "rug" => (3, 2),
                _ => (1, 1),
            };
            let cell = vec2((i % cols) as f32 * cw, (i / cols) as f32 * ch);
            let o = cell + vec2(0.5 * TILE, 1.6 * TILE);
            let st = FurnitureState {
                dirty: if i % 3 == 0 { 0.8 } else { 0.0 },
                t,
                ..Default::default()
            };
            q.push_furniture(id, o, size, st);
            placed.push((*id, o, size, st));
            let label = (*id).to_string();
            q.push_custom(1e9, move || {
                world_text(
                    &label,
                    cell + vec2(1.5 * TILE, 2.85 * TILE),
                    13.0,
                    Color::from_hex(0xece8f7),
                    true,
                )
            });
        }
        q.flush();
        if lit {
            lighting.begin(&cam, ambient_for(0.4));
            for (id, o, size, st) in &placed {
                for l in furniture_lights(id, *o, *size, st) {
                    lighting.add_light(l);
                }
            }
            lighting.apply();
        }
        if finish_frame(&shot, &mut frame).await {
            break;
        }
    }
}

// ───────────────────────── hands-on & moments ─────────────────────────

/// Night-moment markers around the café (the `moments` demo).
fn demo_markers(poses: &[PersonPose], t: f32) -> Vec<MomentMarker> {
    let mk =
        |anchor: Vec2, kind: MomentMarkerKind, born: f32| MomentMarker::new(anchor, kind, born);
    let mut v = vec![
        mk(vec2(2.4, 13.7) * TILE, MomentMarkerKind::Cat, 0.0),
        mk(vec2(6.5, 12.62) * TILE, MomentMarkerKind::Bucket, 0.3),
        mk(vec2(14.5, 1.35) * TILE, MomentMarkerKind::Window, 0.6),
        mk(vec2(7.5, 14.6) * TILE, MomentMarkerKind::Jukebox, 0.9).with_lift(104.0),
        MomentMarker::over_person(poses[7].feet, MomentMarkerKind::Blanket, 0.2, true),
        mk(vec2(4.3, 12.4) * TILE, MomentMarkerKind::Traveller, 0.5),
        mk(vec2(9.6, -2.1) * TILE, MomentMarkerKind::Busker, 0.8),
        mk(vec2(25.5, 1.35) * TILE, MomentMarkerKind::Drone, 0.1),
        mk(vec2(24.5, 10.55) * TILE, MomentMarkerKind::Candle, 0.4).with_lift(70.0),
        MomentMarker::over_person(poses[1].feet, MomentMarkerKind::Coffee, 0.7, false),
        MomentMarker::over_person(poses[6].feet, MomentMarkerKind::Phone, 1.0, true),
    ];
    v[3].hover = 1.0;
    v[9].expires_in = Some(4.0);
    let _ = t;
    v
}

const TABLE_TOP: f32 = 4.0;

/// Coins, wishes, order marks and Karolina's queue (the `handson` demo, after lighting).
fn handson_overlays(actors: &[Actor], poses: &[PersonPose], t: f32) {
    // Karolina's queue
    let k = &actors[2];
    draw_queue_chips(
        emote_anchor(&k.look, &poses[2]),
        &[
            QueueIcon::Brew(CupGlyph::Latte),
            QueueIcon::Serve(CupGlyph::Latte),
            QueueIcon::CollectTip,
            QueueIcon::Clear,
            QueueIcon::Mop,
        ],
        (t * 0.25) % 1.0,
        Some(3),
        t,
    );
    // wishes and order marks
    for (i, tastes) in [
        (
            12usize,
            vec![TasteIcon::Warm, TasteIcon::Sweet, TasteIcon::Comforting],
        ),
        (6, vec![TasteIcon::Cold, TasteIcon::Fresh, TasteIcon::Quick]),
    ] {
        draw_wish_bubble(emote_anchor(&actors[i].look, &poses[i]), &tastes, 1.0, t);
    }
    for (i, mark) in [
        (10usize, OrderMark::Waiting),
        (8, OrderMark::Usual),
        (14, OrderMark::Claimed),
    ] {
        draw_order_mark(emote_anchor(&actors[i].look, &poses[i]), mark, 1.0, t);
    }
    // tip coins: dropping, resting, flying into the jar
    let jar = vec2(18.5 * TILE, 10.0 * TILE - 15.0);
    let table = |x: f32, y: f32| vec2(x * TILE + 24.0, y * TILE + TABLE_TOP + 4.0);
    let a = t % 4.0;
    draw_tip_coin(&TipCoin::new(table(4.0, 5.0), t - a, jar, 1), t);
    draw_tip_coin(&TipCoin::new(table(12.0, 8.0), t - 10.0, jar, 2), t);
    draw_tip_coin(
        &TipCoin::new(table(4.0, 8.0) + vec2(14.0, 2.0), t - 12.0, jar, 3),
        t,
    );
    let cyc = 2.4;
    let mut c = TipCoin::new(table(20.0, 5.0), t - 20.0, jar, 4);
    c.collected = Some(t - (t % cyc));
    draw_tip_coin(&c, t);
    let float_age = (t % cyc) - TIP_COLLECT_SECS;
    draw_tip_float(jar - vec2(0.0, 6.0), "+38 Kč", float_age);
    draw_rhythm_glow(jar + vec2(16.0, 10.0), 3, t);
}

/// A screen-space legend: every cup glyph, latte art, taste icon, order mark and queue icon.
fn handson_legend(t: f32) {
    set_camera(&ui_camera());
    let s = screen_dpi_scale();
    let h = screen_height() / s;
    let (x0, y0, pw, ph) = (10.0, h - 66.0, 616.0, 58.0);
    draw::rrect_v(
        x0 - 1.0,
        y0 - 1.0,
        pw + 2.0,
        ph + 2.0,
        11.0,
        alpha(pal::INDIGO_SOFT, 0.55),
        alpha(pal::INDIGO, 0.3),
    );
    draw::rrect_v(
        x0,
        y0,
        pw,
        ph,
        10.0,
        Color::from_hex(0x221f44),
        Color::from_hex(0x17152f),
    );
    let mut x = x0 + 14.0;
    let row1 = y0 + 17.0;
    for g in CupGlyph::ALL {
        draw_cup_glyph(vec2(x, row1), 17.0, g, None);
        x += 21.0;
    }
    for art in LatteArt::ALL {
        draw_cup_glyph(vec2(x, row1), 17.0, CupGlyph::Latte, Some(art));
        x += 21.0;
    }
    for m in [
        OrderMark::Waiting,
        OrderMark::Claimed,
        OrderMark::Usual,
        OrderMark::Water,
    ] {
        draw_order_mark(vec2(x + 4.0, row1 + 15.0), m, 1.0, t);
        x += 23.0;
    }
    for (i, e) in [
        Emote::Sparkle,
        Emote::Smile,
        Emote::WarmHeart,
        Emote::Sleepy,
        Emote::LikedRecord,
    ]
    .iter()
    .enumerate()
    {
        draw_emote_ex(vec2(x + 8.0 + i as f32 * 24.0, row1 + 12.0), *e, 1.0, t);
    }
    let row2 = y0 + 44.0;
    let mut x = x0 + 14.0;
    for ti in TasteIcon::ALL {
        draw::rrect(
            x - 8.0,
            row2 - 8.0,
            16.0,
            16.0,
            4.5,
            Color::from_hex(0xf4f1ff),
        );
        draw_taste_icon(vec2(x, row2), 11.0, ti, t);
        x += 19.0;
    }
    x += 6.0;
    for (i, k) in MomentMarkerKind::ALL.iter().enumerate() {
        let c = vec2(x + i as f32 * 19.0, row2);
        draw::ellipse_v(
            c.x,
            c.y,
            8.5,
            8.5,
            Color::from_hex(0x6d6af5),
            Color::from_hex(0x3b2fa8),
        );
        draw_moment_icon(c, 11.0, *k, t);
    }
}

// ───────────────────────── themes ─────────────────────────

const ROOM: [&str; 6] = [
    "#WWW##WW#",
    "#.......#",
    "#.......#",
    "#.......#",
    "#.......#",
    "#########",
];

fn room_tile(x: i32, y: i32) -> char {
    if x < 0 || y < 0 || y >= ROOM.len() as i32 || x >= 9 {
        return ' ';
    }
    ROOM[y as usize].chars().nth(x as usize).unwrap_or(' ')
}

async fn themes_demo(shot: Option<String>, t0: f32) {
    let themes = [
        (DecorTheme::GrandmaOriginal, Season::LateAutumn),
        (DecorTheme::PragueArtNouveau, Season::LateAutumn),
        (DecorTheme::StationRetro, Season::LateAutumn),
        (DecorTheme::Nocturne2039, Season::LateAutumn),
        (DecorTheme::WinterLights, Season::FirstSnow),
        (DecorTheme::Seasonal, Season::LateAutumn),
    ];
    let room_w = 9.0 * TILE;
    let room_h = 6.0 * TILE;
    let gap = 1.2 * TILE;
    let offset = |i: usize| {
        vec2(
            (i % 3) as f32 * (room_w + gap),
            (i / 3) as f32 * (room_h + gap + 0.8 * TILE),
        )
    };
    let is_wall = |x: i32, y: i32| matches!(room_tile(x, y), '#' | 'W');
    let is_floor = |x: i32, y: i32| room_tile(x, y) == '.';
    let mut layers = Vec::new();
    let mut glass_all = Vec::new();
    let mut lows: Vec<Vec<(Vec2, WallMask, WallKind)>> = Vec::new();
    for (i, (th, season)) in themes.iter().enumerate() {
        let style = th.style(*season);
        let o = offset(i) / TILE;
        let mut layer = StaticLayer::new();
        let mut low = Vec::new();
        let rect = Rect::new(
            o.x * TILE - 8.0,
            o.y * TILE - 2.0 * TILE,
            room_w + 16.0,
            room_h + 2.5 * TILE,
        );
        with_decor(style, || {
            layer.build(rect, 2.0, || {
                for y in 0..6 {
                    for x in 0..9 {
                        if room_tile(x, y) == '.' {
                            draw_floor("floor_wood", o + vec2(x as f32, y as f32));
                        }
                    }
                }
                for y in 0..6 {
                    for x in 0..9 {
                        let ch = room_tile(x, y);
                        if !matches!(ch, '#' | 'W') {
                            continue;
                        }
                        let m = WallMask::compute(x, y, is_wall, is_floor);
                        let k = if ch == 'W' {
                            WallKind::Window
                        } else {
                            WallKind::Plain
                        };
                        let pos = o + vec2(x as f32, y as f32);
                        if m.is_low() {
                            low.push((pos, m, k));
                        } else {
                            draw_wall(pos, m, k);
                        }
                    }
                }
            });
        });
        for y in 0..6 {
            for x in 0..9 {
                if room_tile(x, y) == 'W' {
                    glass_all.push((i, window_glass_rect(o + vec2(x as f32, y as f32))));
                }
            }
        }
        layers.push(layer);
        lows.push(low);
    }
    let mut lighting = Lighting::new();
    lighting.set_windows(glass_all.iter().map(|g| g.1).collect());
    lighting.coziness = 0.75;
    let weather = Weather::default();
    let guest = PersonLook::preset("vera");
    let guest2 = PersonLook::preset("mirek");
    let mut frame = 0;
    loop {
        let t = if shot.is_some() {
            t0 + frame as f32 / 60.0
        } else {
            t0 + get_time() as f32
        };
        let ww = 3.0 * room_w + 2.0 * gap;
        let wh = 2.0 * (room_h + gap + 0.8 * TILE) - gap;
        let (target, zoom) = match env_f("SHIFTELLA_ROOM") {
            // one room up close
            Some(i) => {
                let o = offset(i as usize % themes.len());
                let z = (screen_width() / room_w).min(screen_height() / (room_h + 1.4 * TILE));
                (o + vec2(room_w / 2.0, room_h / 2.0 - 0.6 * TILE), z)
            }
            None => (
                vec2(ww / 2.0, wh / 2.0 - 1.1 * TILE),
                (screen_width() / ww).min(screen_height() / (wh + 1.6 * TILE)) * 0.98,
            ),
        };
        let cam = Camera2D {
            target,
            zoom: vec2(2.0 * zoom / screen_width(), 2.0 * zoom / screen_height()),
            ..Default::default()
        };
        set_camera(&cam);
        clear_background(pal::NIGHT_0);
        for (i, (_, season)) in themes.iter().enumerate() {
            let o = offset(i);
            draw::rect_v(
                o.x - 6.0,
                o.y - 2.4 * TILE,
                room_w + 12.0,
                1.4 * TILE,
                pal::VIOLET_NIGHT_0,
                pal::VIOLET_NIGHT_1,
            );
            if *season == Season::FirstSnow {
                Snowfall::default().draw(Rect::new(o.x, o.y - 2.4 * TILE, room_w, 1.4 * TILE), t);
            } else {
                weather.draw_rain(Rect::new(o.x, o.y - 2.4 * TILE, room_w, 1.4 * TILE), t);
            }
            layers[i].draw();
        }
        for (i, g) in &glass_all {
            weather.draw_window_rain(*g, t);
            draw_window_season(*g, &themes[*i].1.look(), t);
        }
        let mut placed = Vec::new();
        let mut q = RenderQueue::new();
        for (i, (th, season)) in themes.iter().enumerate() {
            let style = th.style(*season);
            let o = offset(i);
            let at = |x: f32, y: f32| o + vec2(x, y) * TILE;
            let st = |f: Facing| FurnitureState {
                t,
                facing: f,
                ..Default::default()
            };
            let pieces: Vec<(&'static str, Vec2, (u8, u8), FurnitureState)> = vec![
                ("rug", at(2.0, 2.0), (3, 2), st(Facing::Down)),
                ("table_round_2", at(3.0, 3.0), (1, 1), st(Facing::Down)),
                ("chair", at(2.0, 3.0), (1, 1), st(Facing::Right)),
                ("chair", at(4.0, 3.0), (1, 1), st(Facing::Left)),
                ("pendant_lamp", at(3.0, 3.0), (1, 1), st(Facing::Down)),
                ("sofa", at(5.0, 1.0), (2, 1), st(Facing::Down)),
                ("floor_lamp", at(7.0, 1.0), (1, 1), st(Facing::Down)),
                ("armchair", at(6.0, 3.0), (1, 1), st(Facing::Down)),
                ("window_seat", at(1.0, 1.0), (3, 1), st(Facing::Down)),
                ("plant_small", at(7.0, 4.0), (1, 1), st(Facing::Down)),
                ("stool", at(5.0, 4.0), (1, 1), st(Facing::Down)),
            ];
            for (id, po, size, s) in pieces {
                let s2 = s;
                q.push_custom(furniture_sort_key(id, po, size), move || {
                    with_decor(style, || draw_furniture(id, po, size, &s2))
                });
                placed.push((style, id, po, size, s));
            }
            for (pos, m, k) in &lows[i] {
                let (pos, m, k) = (*pos, *m, *k);
                q.push_custom(wall_sort_key(pos), move || {
                    with_decor(style, || draw_wall(pos, m, k))
                });
            }
            q.push_person(
                &guest,
                PersonPose {
                    feet: sit_anchor(at(2.0, 3.0), Facing::Right),
                    facing: Facing::Right,
                    pose: Pose::Sit,
                    t,
                },
            );
            q.push_person(
                &guest2,
                PersonPose {
                    feet: sit_anchor(at(4.0, 3.0), Facing::Left),
                    facing: Facing::Left,
                    pose: Pose::Sit,
                    t: t + 0.4,
                },
            );
        }
        q.flush();
        lighting.begin(&cam, ambient_for(0.45));
        lighting.set_time(t);
        lighting.add_window_light(0.8);
        for (style, id, po, size, s) in &placed {
            with_decor(*style, || {
                for l in furniture_lights(id, *po, *size, s) {
                    lighting.add_light(l);
                }
            });
        }
        lighting.apply();
        lighting.draw_dust_motes(t);
        for (i, (th, season)) in themes.iter().enumerate() {
            let o = offset(i);
            let label = if *th == DecorTheme::Seasonal {
                format!("{} ({})", th.id(), season.id())
            } else {
                th.id().to_string()
            };
            world_text(
                &label,
                o + vec2(room_w / 2.0, room_h + 0.55 * TILE),
                18.0,
                Color::from_hex(0xece8f7),
                true,
            );
        }
        if finish_frame(&shot, &mut frame).await {
            break;
        }
    }
}

// ───────────────────────── polaroid ─────────────────────────

/// A snapshot of a region of the demo café.
fn snap_of(map: &Map, furniture: &[Piece], origin: (i32, i32), size: (u8, u8)) -> SceneSnap {
    let mut tiles = Vec::new();
    for y in origin.1..origin.1 + size.1 as i32 {
        for x in origin.0..origin.0 + size.0 as i32 {
            let tile = if x < 0 || y < 0 {
                None
            } else {
                map.get(x as usize, y as usize)
            };
            tiles.push(match tile {
                None => SnapTile::Void,
                Some(Tile::Wall) => SnapTile::Wall,
                Some(Tile::Window) => SnapTile::Window,
                Some(Tile::Door) => SnapTile::Door(SnapFloor::Wood),
                Some(_) => SnapTile::Floor(SnapFloor::from_id(floor_kind(x as usize, y as usize))),
            });
        }
    }
    let inside = |p: &Piece| {
        let tx = (p.origin.x / TILE) as i32;
        let ty = (p.origin.y / TILE) as i32;
        tx >= origin.0 - 1
            && tx < origin.0 + size.0 as i32 + 1
            && ty >= origin.1 - 1
            && ty < origin.1 + size.1 as i32 + 1
    };
    SceneSnap {
        origin,
        size,
        tiles,
        props: furniture
            .iter()
            .filter(|p| inside(p))
            .map(|p| SnapProp {
                sprite: p.sprite.to_string(),
                tile: ((p.origin.x / TILE) as i32, (p.origin.y / TILE) as i32),
                size: p.size,
                facing: p.facing,
                on: p.sprite != "drone_dock",
                dirty: p.dirty,
            })
            .collect(),
        ..Default::default()
    }
}

fn sitter(
    id: &str,
    chair: (f32, f32),
    facing: Facing,
    expr: Expression,
    emote: Option<Emote>,
) -> SnapActor {
    SnapActor {
        look: PersonLook::preset(id),
        feet: sit_anchor(vec2(chair.0, chair.1) * TILE, facing),
        facing,
        pose: Pose::Sit,
        expression: expr,
        emote,
    }
}

async fn polaroid_demo(shot: Option<String>, t0: f32) {
    let map = Map::from_layout(CAFE_LAYOUT);
    let furniture = build_furniture(&map, false);
    // 1) late autumn: Vera laughing over her tea with Mirek
    let mut a = snap_of(&map, &furniture, (1, 0), (8, 6));
    a.actors = vec![
        sitter("mirek", (3.0, 2.0), Facing::Right, Expression::Smile, None),
        sitter(
            "vera",
            (5.0, 2.0),
            Facing::Left,
            Expression::Laugh,
            Some(Emote::Sparkle),
        ),
    ];
    a.cups = vec![SnapCup {
        pos: vec2(4.0 * TILE + 26.0, 2.0 * TILE + 6.0),
        glyph: CupGlyph::Tea,
        art: None,
    }];
    a.seed = 11;
    a.focus = Some(vec2(4.5, 2.0) * TILE);
    a.zoom = 2.1;
    a.time = 2.0;
    // 2) first snow through grandma's window, garlands up, the cat stayed
    let mut b = snap_of(&map, &furniture, (10, 0), (10, 6));
    b.season = Season::FirstSnow;
    b.rain = 0.0;
    b.garlands = 1.0;
    b.coziness = 0.9;
    b.night = 0.7;
    b.actors = vec![
        sitter("hana", (11.0, 2.0), Facing::Right, Expression::Smile, None),
        sitter(
            "filip",
            (13.0, 2.0),
            Facing::Left,
            Expression::Surprised,
            None,
        ),
    ];
    b.cat = Some((vec2(16.2, 3.2) * TILE, CatPose::Sleep));
    b.cups = vec![SnapCup {
        pos: vec2(12.0 * TILE + 26.0, 2.0 * TILE + 6.0),
        glyph: CupGlyph::Latte,
        art: Some(LatteArt::Snowflake),
    }];
    b.seed = 27;
    b.focus = Some(vec2(14.0, 1.9) * TILE);
    b.zoom = 1.6;
    b.time = 5.0;
    // 3) Advent, blackout: candles and phones
    let mut c = snap_of(&map, &furniture, (1, 3), (10, 7));
    c.season = Season::Festive;
    c.theme = DecorTheme::WinterLights;
    c.blackout = true;
    c.rain = 0.4;
    c.actors = vec![
        sitter(
            "lori",
            (3.0, 5.0),
            Facing::Right,
            Expression::Smile,
            Some(Emote::WarmHeart),
        ),
        sitter("radek", (5.0, 5.0), Facing::Left, Expression::Neutral, None),
        sitter(
            "anwar",
            (11.0, 8.0),
            Facing::Right,
            Expression::Thinking,
            None,
        ),
    ];
    c.seed = 9;
    c.focus = Some(vec2(4.5, 4.9) * TILE);
    c.zoom = 2.2;
    c.time = 8.0;
    // 4) a train passing, 3:33
    let mut d = snap_of(&map, &furniture, (16, 0), (10, 6));
    d.train = Some(0.45);
    d.night = 0.55;
    d.theme = DecorTheme::StationRetro;
    d.actors = vec![sitter(
        "guest_333",
        (19.0, 2.0),
        Facing::Right,
        Expression::Neutral,
        None,
    )];
    d.seed = 333;
    d.focus = Some(vec2(20.0, 2.0) * TILE);
    d.zoom = 2.0;

    let mut renderer = SnapRenderer::new();
    let photos: Vec<RenderTarget> = [&a, &b, &c, &d]
        .iter()
        .map(|s| renderer.render(s, 720, 540))
        .collect();
    // determinism: the same snap renders the same pixels
    let again = renderer.render(&a, 720, 540);
    let same = photos[0].texture.get_texture_data().bytes == again.texture.get_texture_data().bytes;
    println!("snapshot deterministic: {same}");

    let captions = [
        (
            "Vera laughed tonight. First time I heard it.",
            "9. 11. 2039",
        ),
        ("The first snow, through grandma's window.", "3. 12. 2039"),
        ("Candles, and everyone stayed.", "27. 11. 2039"),
        (
            "3:33. The clock stopped, and he was there again.",
            "15. 11. 2039",
        ),
    ];
    let mut frame = 0;
    loop {
        let t = if shot.is_some() {
            t0 + frame as f32 / 60.0
        } else {
            t0 + get_time() as f32
        };
        clear_background(Color::from_hex(0x2a2238));
        set_camera(&ui_camera());
        let s = screen_dpi_scale();
        let (w, h) = (screen_width() / s, screen_height() / s);
        // a kraft-paper scrapbook page
        draw::rect_v(
            0.0,
            0.0,
            w,
            h,
            Color::from_hex(0x8a6a52),
            Color::from_hex(0x6e523f),
        );
        for i in 0..900u32 {
            let x = draw::hash01(i * 3) * w;
            let y = draw::hash01(i * 3 + 1) * h;
            let l = 3.0 + draw::hash01(i * 3 + 2) * 9.0;
            draw_line(
                x,
                y,
                x + l,
                y + l * 0.2,
                0.6,
                Color::new(0.25, 0.17, 0.12, 0.12),
            );
        }
        draw::rect_v(
            0.0,
            0.0,
            w,
            h,
            Color::new(0.2, 0.12, 0.3, 0.25),
            Color::new(0.1, 0.05, 0.2, 0.45),
        );
        let pw = (w - 120.0) / 4.0 * 0.82;
        let row_bottom = 70.0 + 40.0 + polaroid_layout(Vec2::ZERO, pw, 0.0).frame.h;
        let develop =
            env_f("SHIFTELLA_DEVELOP").unwrap_or(((t - t0) / 2.0).clamp(0.0, 1.0) * 0.0 + 0.45);
        for i in 0..4 {
            let x = 30.0 + i as f32 * (w - 60.0 - pw) / 3.0;
            let y = 70.0 + if i % 2 == 0 { 0.0 } else { 40.0 };
            let st = PolaroidStyle {
                tilt: polaroid_tilt(i as u32 + 3),
                develop: if i == 3 { develop } else { 1.0 },
                caption: Some(captions[i].0),
                date: Some(captions[i].1),
                tape: true,
                lift: if i == 1 { 1.0 } else { 0.0 },
            };
            draw_polaroid(Rect::new(x, y, pw, 0.0), Some(&photos[i].texture), &st);
        }
        world_text(
            "Journal — night 9 · 21 · 27 · 30 (re-rendered from data)",
            vec2(30.0, 40.0),
            15.0,
            Color::from_hex(0xf3ead8),
            false,
        );
        // an enlarged one
        let big_h_k = polaroid_layout(Vec2::ZERO, 1.0, 0.0).frame.h;
        let big = ((h - row_bottom - 30.0) / big_h_k).min(w * 0.5);
        draw_polaroid(
            Rect::new(w / 2.0 - big / 2.0, row_bottom + 22.0, big, 0.0),
            Some(&photos[1].texture),
            &PolaroidStyle {
                tilt: -0.02,
                caption: Some(captions[1].0),
                date: Some(captions[1].1),
                ..Default::default()
            },
        );
        if finish_frame(&shot, &mut frame).await {
            break;
        }
    }
}
