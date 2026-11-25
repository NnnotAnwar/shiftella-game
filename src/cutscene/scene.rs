//! The standalone stage for cutscenes: café «U Výhybky» (`map::CAFE_LAYOUT`) with the rainy
//! street, platform and tracks behind it. Lamps can be switched off (the dark café of the
//! opening) and back on one by one; trains pass with the light sweep.
//!
//! The integration can use it as-is, or draw the live game world with the player's camera
//! ([`super::CutscenePlayer::world_camera`]) and actors instead.

use macroquad::prelude::*;

use crate::camera::WorldCamera;
use crate::map::{CAFE_LAYOUT, Map, TILE, Tile};
use crate::render::colors as pal;
use crate::render::draw::{alpha, mix, radial, rrect_v, soft_shadow};
use crate::render::*;

use super::actors::Actor;

/// World size the default (zoom 1.0) camera fits on screen: café + a strip of street.
pub const OVERVIEW_SIZE: Vec2 = Vec2::new(32.0 * TILE, 21.6 * TILE);
/// World point of the overview camera.
pub const OVERVIEW_CENTER: Vec2 = Vec2::new(15.0 * TILE, 5.4 * TILE);

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
    fn is_lamp(&self) -> bool {
        is_lamp_sprite(self.sprite)
    }
}

/// Furniture that goes dark with the closed café and lights up in the opening cascade.
pub fn is_lamp_sprite(sprite: &str) -> bool {
    matches!(
        sprite,
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
            | "candle"
            | "table_round_2"
            | "table_square_4"
    )
}

/// Where the opening's lamp cascade starts: the lamp in the window.
const CASCADE_FIRST: Vec2 = Vec2::new(4.5 * TILE, 2.5 * TILE);

/// The café + street stage.
pub struct CafeScene {
    pub map: Map,
    furniture: Vec<Piece>,
    /// Per furniture piece: brightness 0..1 (only lamps use it).
    lamp: Vec<f32>,
    /// Per furniture piece: when it switches on during a cascade (0..1), `None` = not a lamp.
    lamp_order: Vec<Option<f32>>,
    /// Target: 1 = every lamp on, 0 = dark café.
    lights_target: f32,
    /// Cascade progress 0..1 (`None` = lamps follow `lights_target` directly).
    cascade: Option<(f32, f32)>,
    /// GPU resources, created on the first draw (keeps the logic usable without a window).
    statics: Option<StaticLayer>,
    low_walls: Vec<(Vec2, WallMask, WallKind)>,
    glass: Vec<Rect>,
    lighting: Option<Lighting>,
    pub street: Rect,
    street_lights: Vec<Light>,
    pub weather: Weather,
    /// Train progress 0..1 (right → left), `None` = no train.
    pub train: Option<f32>,
    /// Distance of the farthest lamp from the cascade start (normalises `lamp_at`).
    cascade_max_d: f32,
    /// 0..1 how "lived-in night" the café looks (ambient when lit).
    pub night_progress: f32,
}

impl Default for CafeScene {
    fn default() -> Self {
        Self::new()
    }
}

impl CafeScene {
    pub fn new() -> CafeScene {
        let map = Map::from_layout(CAFE_LAYOUT);
        let furniture = build_furniture(&map);
        let mut glass = Vec::new();
        for y in 0..map.height {
            for x in 0..map.width {
                if map.get(x, y) == Some(Tile::Window) {
                    glass.push(window_glass_rect(vec2(x as f32, y as f32)));
                }
            }
        }
        let street = Rect::new(
            -8.0 * TILE,
            -10.0 * TILE,
            (map.width as f32 + 16.0) * TILE,
            8.55 * TILE,
        );
        // cascade order: the lamp in the window first, then outward
        let first = CASCADE_FIRST;
        let lamps: Vec<(usize, f32)> = furniture
            .iter()
            .enumerate()
            .filter(|(_, p)| p.is_lamp())
            .map(|(i, p)| (i, p.origin.distance(first)))
            .collect();
        let max_d = lamps.iter().map(|l| l.1).fold(1.0, f32::max);
        let mut lamp_order = vec![None; furniture.len()];
        for (i, d) in lamps {
            lamp_order[i] = Some(if d < TILE {
                0.0
            } else {
                0.18 + 0.82 * d / max_d
            });
        }
        CafeScene {
            lamp: vec![1.0; furniture.len()],
            lamp_order,
            lights_target: 1.0,
            cascade: None,
            map,
            furniture,
            statics: None,
            low_walls: Vec::new(),
            glass,
            lighting: None,
            street_lights: outside_lights(street),
            street,
            weather: Weather::default(),
            train: None,
            night_progress: 0.15,
            cascade_max_d: max_d,
        }
    }

    /// All lamps on (`true`) or the dark, closed café (`false`), immediately.
    pub fn set_lights(&mut self, on: bool) {
        self.lights_target = if on { 1.0 } else { 0.0 };
        self.cascade = None;
        for (i, l) in self.lamp.iter_mut().enumerate() {
            if self.lamp_order[i].is_some() {
                *l = self.lights_target;
            }
        }
    }

    /// Switch the lamps on one by one over `seconds` (window lamp first).
    pub fn lights_cascade(&mut self, seconds: f32) {
        self.lights_target = 1.0;
        self.cascade = Some((0.0, seconds.max(0.1)));
    }

    /// Brightness 0..1 of a lamp standing at `origin` (world px). Lets the live café follow
    /// the stage lights: dark in the opening, then lit one by one from the window outward.
    pub fn lamp_at(&self, origin: Vec2) -> f32 {
        match self.cascade {
            Some((p, dur)) => {
                let d = origin.distance(CASCADE_FIRST);
                let at = if d < TILE {
                    0.0
                } else {
                    0.18 + 0.82 * (d / self.cascade_max_d).min(1.0)
                };
                ((p - at) * dur / 0.35).clamp(0.0, 1.0)
            }
            None => self.light_level(),
        }
    }

    pub fn cascade_done(&self) -> bool {
        self.cascade.is_none()
    }

    /// Overall light level 0..1 (average lamp brightness).
    pub fn light_level(&self) -> f32 {
        let (mut sum, mut n) = (0.0, 0.0);
        for (i, l) in self.lamp.iter().enumerate() {
            if self.lamp_order[i].is_some() {
                sum += *l;
                n += 1.0;
            }
        }
        if n > 0.0 { sum / n } else { 1.0 }
    }

    pub fn update(&mut self, dt: f32) {
        if let Some((p, dur)) = &mut self.cascade {
            *p += dt / *dur;
            let p = *p;
            for (i, l) in self.lamp.iter_mut().enumerate() {
                if let Some(at) = self.lamp_order[i]
                    && p >= at
                {
                    *l = (*l + dt / 0.35).min(1.0);
                }
            }
            if p >= 1.0 && self.lamp.iter().all(|l| *l >= 1.0) {
                self.cascade = None;
            }
        } else {
            for (i, l) in self.lamp.iter_mut().enumerate() {
                if self.lamp_order[i].is_some() {
                    let step = dt / 0.4;
                    *l = if *l < self.lights_target {
                        (*l + step).min(self.lights_target)
                    } else {
                        (*l - step).max(self.lights_target)
                    };
                }
            }
        }
    }

    /// The camera for `center` (world px) and `zoom` (1.0 = overview zoom).
    pub fn camera(&self, center: Vec2, zoom: f32) -> Camera2D {
        let screen = Rect::new(0.0, 0.0, screen_width(), screen_height());
        let mut cam = WorldCamera::fit(OVERVIEW_SIZE, screen);
        cam.zoom *= zoom;
        // keep the view on the stage (no black void beside the café)
        let half = vec2(screen.w, screen.h) / cam.zoom / 2.0;
        let b = self.bounds();
        let clamp = |c: f32, lo: f32, hi: f32, h: f32| {
            if hi - lo <= 2.0 * h {
                (lo + hi) / 2.0
            } else {
                c.clamp(lo + h, hi - h)
            }
        };
        cam.center = vec2(
            clamp(center.x, b.x, b.x + b.w, half.x),
            clamp(center.y, b.y, b.y + b.h, half.y),
        );
        cam.to_camera2d()
    }

    /// World area worth looking at: the café and the street strip above it.
    pub fn bounds(&self) -> Rect {
        let ms = self.map.size_px();
        Rect::new(
            -0.5 * TILE,
            self.street.y + 1.5 * TILE,
            ms.x + TILE,
            // extra floor below the café: room under the dialogue box when framing low actors
            ms.y + 3.5 * TILE - (self.street.y + 1.5 * TILE),
        )
    }

    fn build_statics(&mut self) {
        let map = &self.map;
        let static_rect = Rect::new(
            -TILE,
            -2.0 * TILE,
            (map.width as f32 + 2.0) * TILE,
            (map.height as f32 + 3.0) * TILE,
        );
        let mut low = Vec::new();
        let statics = self.statics.get_or_insert_with(StaticLayer::new);
        statics.build(static_rect, 2.0, || {
            for y in 0..map.height {
                for x in 0..map.width {
                    let tile = map.get(x, y).unwrap_or(Tile::Floor);
                    if !matches!(tile, Tile::Wall | Tile::Window) {
                        draw_floor(floor_kind(x, y), vec2(x as f32, y as f32));
                    }
                }
            }
            for y in 0..map.height {
                for x in 0..map.width {
                    let tile = map.get(x, y).unwrap_or(Tile::Floor);
                    if matches!(tile, Tile::Wall | Tile::Window | Tile::Door) {
                        let m = WallMask::compute(
                            x as i32,
                            y as i32,
                            |a, b| is_wall(map, a, b),
                            |a, b| is_floor(map, a, b),
                        );
                        let pos = vec2(x as f32, y as f32);
                        if m.is_low() {
                            low.push((pos, m, wall_kind(tile)));
                        } else {
                            draw_wall(pos, m, wall_kind(tile));
                        }
                    }
                }
            }
        });
        self.low_walls = low;
    }

    /// Draws the whole stage with `cam` (sets it). Leaves `cam` active.
    pub fn draw(&mut self, cam: &Camera2D, t: f32, actors: &[Actor]) {
        if self.lighting.is_none() {
            let mut l = Lighting::new();
            l.set_windows(self.glass.clone());
            self.lighting = Some(l);
        }
        if !self.statics.as_ref().is_some_and(|s| s.is_built()) {
            self.build_statics();
        }
        set_camera(cam);
        clear_background(pal::NIGHT_0);
        let ms = self.map.size_px();
        radial(
            ms.x / 2.0,
            ms.y / 2.0 + TILE,
            ms.x * 0.75,
            ms.y * 0.9,
            pal::VIOLET_NIGHT_1,
            alpha(pal::NIGHT_0, 0.0),
        );
        let train_p = self.train.unwrap_or(-1.0);
        draw_outside(self.street, t);
        draw_train(self.street, train_p, t);
        draw_holo_sign(
            vec2(21.0 * TILE, self.street.y + self.street.h - 0.35 * TILE),
            "U Výhybky",
            t,
        );
        self.weather.draw_rain(self.street, t);
        if let Some(st) = &self.statics {
            st.draw();
        }
        for g in &self.glass {
            self.weather.draw_window_rain(*g, t);
        }

        let level = self.light_level();
        let mut q = RenderQueue::new();
        let mut states = Vec::with_capacity(self.furniture.len());
        for (i, p) in self.furniture.iter().enumerate() {
            let lamp_on = self.lamp[i] > 0.5;
            let st = FurnitureState {
                active: if p.is_lamp() {
                    lamp_on
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
        for (pos, m, k) in &self.low_walls {
            q.push_wall(*pos, *m, *k);
        }
        let poses: Vec<PersonPose> = actors.iter().map(|a| a.pose(t)).collect();
        for (a, pose) in actors.iter().zip(&poses) {
            if a.gone || a.alpha <= 0.01 {
                continue;
            }
            if a.suitcase {
                let feet = pose.feet;
                let side = if pose.facing == Facing::Left {
                    16.0
                } else {
                    -16.0
                };
                q.push_custom(feet.y + 0.5, move || draw_suitcase(feet + vec2(side, 0.0)));
            }
            q.push_person(&a.look, *pose);
        }
        q.flush();

        // light
        let lit = ambient_for(self.night_progress);
        let ambient = mix(ambient_blackout(), lit, level);
        let lighting = self.lighting.as_mut().expect("created above");
        lighting.begin(cam, ambient);
        lighting.set_time(t);
        lighting.add_window_light(0.35 + 0.65 * level);
        lighting.train_sweep(train_p);
        for (i, (p, st)) in self.furniture.iter().zip(&states).enumerate() {
            let k = if p.is_lamp() { self.lamp[i] } else { 1.0 };
            for mut l in furniture_lights(p.sprite, p.origin, p.size, st) {
                // switching on: a short warm flicker
                if k < 1.0 {
                    l.intensity *= k * (0.75 + 0.25 * (t * 41.0 + i as f32).sin().abs());
                }
                lighting.add_light(l);
            }
        }
        for l in &self.street_lights {
            lighting.add_light(*l);
        }
        lighting.add_light(
            Light::new(
                vec2(21.0 * TILE, self.street.y + self.street.h - 1.8 * TILE),
                3.0 * TILE,
                pal::INDIGO_GLOW,
                0.7,
            )
            .flicker(0.1),
        );
        for (a, pose) in actors.iter().zip(&poses) {
            if a.gone {
                continue;
            }
            if a.hand_light {
                lighting.add_light(
                    Light::new(pose.feet - vec2(0.0, 30.0), 3.0 * TILE, pal::AMBER, 1.1)
                        .flicker(0.04),
                );
            } else if level < 0.5 {
                lighting.add_light(Light::actor_spot(pose.feet));
            }
        }
        lighting.apply();
    }
}

/// The heir's suitcase beside the feet (opening).
pub fn draw_suitcase(p: Vec2) {
    soft_shadow(p.x, p.y, 11.0, 3.5, 0.5);
    rrect_v(
        p.x - 9.0,
        p.y - 22.0,
        18.0,
        21.0,
        3.5,
        Color::from_hex(0x8a5a3c),
        Color::from_hex(0x5e3a28),
    );
    rrect_v(
        p.x - 9.0,
        p.y - 22.0,
        18.0,
        3.0,
        1.5,
        Color::from_hex(0xb07a52),
        Color::from_hex(0x8a5a3c),
    );
    draw_rectangle(p.x - 4.0, p.y - 27.0, 8.0, 2.0, Color::from_hex(0x3a2a22));
    draw_rectangle(p.x - 4.0, p.y - 27.0, 2.0, 5.0, Color::from_hex(0x3a2a22));
    draw_rectangle(p.x + 2.0, p.y - 27.0, 2.0, 5.0, Color::from_hex(0x3a2a22));
    draw_rectangle(p.x - 1.0, p.y - 18.0, 2.0, 15.0, alpha(pal::CREAM, 0.35));
}

fn is_wall(map: &Map, x: i32, y: i32) -> bool {
    x >= 0
        && y >= 0
        && matches!(
            map.get(x as usize, y as usize),
            Some(Tile::Wall | Tile::Window | Tile::Door)
        )
}

fn is_floor(map: &Map, x: i32, y: i32) -> bool {
    x >= 0
        && y >= 0
        && matches!(map.get(x as usize, y as usize), Some(t) if !matches!(t, Tile::Wall | Tile::Window | Tile::Door))
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

fn build_furniture(map: &Map) -> Vec<Piece> {
    let mut v = Vec::new();
    for y in 0..map.height {
        for x in 0..map.width {
            let (fx, fy) = (x as f32, y as f32);
            match map.get(x, y).unwrap_or(Tile::Floor) {
                Tile::Table => {
                    v.push(Piece::at("table_round_2", fx, fy, (1, 1)));
                    v.push(Piece::at("pendant_lamp", fx, fy, (1, 1)));
                }
                Tile::Chair => {
                    let f = if map.get(x + 1, y) == Some(Tile::Table) {
                        Facing::Right
                    } else {
                        Facing::Left
                    };
                    v.push(Piece::at("chair", fx, fy, (1, 1)).facing(f));
                }
                Tile::Counter => {
                    let left = map.get(x.wrapping_sub(1), y) != Some(Tile::Counter);
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
                    let id = if map.get(x.wrapping_sub(1), y) == Some(Tile::CoffeeMachine) {
                        "espresso_grinder"
                    } else {
                        "coffee_machine"
                    };
                    v.push(Piece::at(id, fx, fy, (1, 1)));
                }
                Tile::Stove => {
                    let id = if map.get(x.wrapping_sub(1), y) == Some(Tile::Stove) {
                        "oven"
                    } else {
                        "stove"
                    };
                    v.push(Piece::at(id, fx, fy, (1, 1)));
                }
                Tile::Sink => {
                    let id = if map.get(x.wrapping_sub(1), y) == Some(Tile::Sink) {
                        "dish_rack"
                    } else {
                        "sink"
                    };
                    v.push(Piece::at(id, fx, fy, (1, 1)));
                }
                _ => {}
            }
        }
    }
    v.extend([
        Piece::at("fridge", 22.0, 13.0, (1, 1)),
        Piece::at("trash_bin", 23.0, 13.0, (1, 1)),
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
        Piece::at("pendant_lamp", 17.0, 13.0, (1, 1)),
        Piece::at("pendant_lamp", 20.0, 13.0, (1, 1)),
        Piece::at("pendant_lamp", 18.0, 10.0, (1, 1)),
        Piece::at("pendant_lamp", 23.0, 10.0, (1, 1)),
    ]);
    for x in [17.0, 19.0, 21.0, 23.0, 25.0] {
        v.push(Piece::at("stool", x, 9.0, (1, 1)));
    }
    v
}
