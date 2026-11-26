//! Render demo: the night café from `map::CAFE_LAYOUT`, guests and staff, rain,
//! a train, light. Camera: `WorldCamera` (WASD / arrows, right button, wheel).
//!
//! Environment variables:
//! - `SHIFTELLA_SHOT=path.png` — render ~40 frames, save a PNG and exit;
//! - `SHIFTELLA_ZOOM=2.5`, `SHIFTELLA_CENTER=12.5,5` (in tiles), `SHIFTELLA_T=7.0` (time, s);
//! - `SHIFTELLA_NIGHT=0.55` (night progress 0..1), `SHIFTELLA_BLACKOUT=1`, `SHIFTELLA_TRAIN=0.5`;
//! - `SHIFTELLA_STRESS=1` — 40+ people and ~150 objects (performance check);
//! - `SHIFTELLA_DEMO=portraits|people|furniture` — galleries of portraits, people, furniture.

use std::time::Instant;

use macroquad::prelude::*;

use shiftella_game::camera::WorldCamera;
use shiftella_game::map::{CAFE_LAYOUT, Map, TILE, Tile};
use shiftella_game::render::colors as pal;
use shiftella_game::render::draw::{alpha, radial};
use shiftella_game::render::*;

fn window_conf() -> Conf {
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
        _ => {}
    }

    let stress = std::env::var("SHIFTELLA_STRESS").is_ok();
    let blackout = std::env::var("SHIFTELLA_BLACKOUT").is_ok();
    let night = env_f("SHIFTELLA_NIGHT");
    let train_fixed = env_f("SHIFTELLA_TRAIN");

    let map = Map::from_layout(CAFE_LAYOUT);
    let furniture = build_furniture(&map, stress);
    let actors = build_actors(stress);
    let weather = Weather::default();

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
            }
        }
        for y in 0..map.height {
            for x in 0..map.width {
                let tile = map.get(x, y).unwrap();
                if !matches!(tile, Tile::Wall | Tile::Window) {
                    draw_floor(floor_kind(x, y), vec2(x as f32, y as f32));
                }
            }
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
        draw_train(street, train_p, t);
        draw_holo_sign(
            vec2(21.0 * TILE, street.y + street.h - 0.35 * TILE),
            "Café Nocturne",
            t,
        );
        weather.draw_rain(street, t);
        statics.draw();
        for g in &glass {
            weather.draw_window_rain(*g, t);
        }

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
                    !lamp
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
        lighting.train_sweep(train_p);
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
        lighting.apply();

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
