//! The live café, drawn from the sim state with the render module (its documented frame
//! order): street + train + rain behind the back wall, the cached static layer (floors, tall
//! walls), a y-sorted queue (furniture, low front walls, people, props), lighting (ambient by
//! night progress, lamps, windows, blackout, train sweep, phone lights) and emotes on top.

use std::collections::HashMap;

use macroquad::prelude::*;

use super::world_view::{self as wv, sim};
use crate::camera::WorldCamera;
use crate::content::Content;
use crate::cutscene::CutscenePlayer;
use crate::cutscene::scene::{draw_suitcase, is_lamp_sprite};
use crate::map::TILE;
use crate::render::colors as pal;
use crate::render::draw::{alpha, mix, radial};
use crate::render::*;
use sim::{AgentRef, AgentView, World};

/// Seconds a train needs to cross the strip behind the café.
const TRAIN_SECONDS: f32 = 7.5;

fn akey(a: AgentRef) -> (u8, u32) {
    match a {
        AgentRef::Staff(i) => (0, i),
        AgentRef::Guest(i) => (1, i),
    }
}

/// Hit-test result in the world.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pick {
    Agent(AgentRef),
    Furniture(u32),
    Tile(i32, i32),
}

pub struct WorldScene {
    statics: Option<StaticLayer>,
    statics_key: u64,
    low_walls: Vec<(Vec2, WallMask, WallKind)>,
    glass: Vec<Rect>,
    lighting: Option<Lighting>,
    pub street: Rect,
    street_lights: Vec<Light>,
    looks: HashMap<(u64, bool, Option<String>), PersonLook>,
    /// Train crossing progress 0..1 (`None` = no train).
    pub train: Option<f32>,
    /// Emote start times (for the pop-in animation).
    emote_started: HashMap<(u8, u32), (Emote, f32)>,
    /// Cached views of this frame.
    pub people: Vec<AgentView>,
    size: (i32, i32),
    /// Per agent: (feet at the previous tick, feet at the last tick, that tick).
    tracks: HashMap<(u8, u32), (Vec2, Vec2, u32)>,
}

impl Default for WorldScene {
    fn default() -> Self {
        Self::new()
    }
}

/// Kind of a grid tile for walls.
fn is_wall_kind(k: sim::TileKind) -> bool {
    matches!(
        k,
        sim::TileKind::Wall | sim::TileKind::Window | sim::TileKind::Door
    )
}

impl WorldScene {
    pub fn new() -> WorldScene {
        WorldScene {
            statics: None,
            statics_key: 0,
            low_walls: Vec::new(),
            glass: Vec::new(),
            lighting: None,
            street: Rect::new(0.0, 0.0, 1.0, 1.0),
            street_lights: Vec::new(),
            looks: HashMap::new(),
            train: None,
            emote_started: HashMap::new(),
            people: Vec::new(),
            size: (30, 18),
            tracks: HashMap::new(),
        }
    }

    /// Café interior size in world px (without the sidewalk rows).
    pub fn cafe_px(&self) -> Vec2 {
        vec2(self.size.0 as f32, self.indoor_rows() as f32) * TILE
    }

    fn indoor_rows(&self) -> i32 {
        // the sim adds two street rows below the café frame
        (self.size.1 - 2).max(1)
    }

    /// The world area worth looking at (camera clamp): street strip, café, sidewalk.
    pub fn bounds(&self) -> Rect {
        let w = self.size.0 as f32 * TILE;
        let top = self.street.y + 1.5 * TILE;
        let bottom = self.size.1 as f32 * TILE + 1.0 * TILE;
        Rect::new(-TILE, top, w + 2.0 * TILE, bottom - top)
    }

    /// A camera that shows the whole café (+ the sidewalk) between the HUD bars.
    pub fn overview_camera(&self, screen: Rect) -> WorldCamera {
        let view = Rect::new(
            -0.3 * TILE,
            -TILE,
            (self.size.0 as f32 + 0.6) * TILE,
            (self.size.1 as f32 + 0.6) * TILE,
        );
        let area = Rect::new(
            screen.x,
            screen.y + screen.h * 0.11,
            screen.w,
            screen.h * 0.80,
        );
        let mut cam = WorldCamera::fit(view.size(), area);
        cam.center += view.point();
        cam
    }

    /// Keeps the camera on the stage (no black void beside the café).
    pub fn clamp_camera(&self, cam: &mut WorldCamera) {
        let fit = WorldCamera::fit(
            self.bounds().size(),
            Rect::new(0.0, 0.0, screen_width(), screen_height()),
        );
        cam.zoom = cam.zoom.max(fit.zoom * 0.8);
        let half = vec2(screen_width(), screen_height()) / cam.zoom / 2.0;
        let b = self.bounds();
        let clamp = |c: f32, lo: f32, hi: f32, h: f32| {
            if hi - lo <= 2.0 * h {
                (lo + hi) / 2.0
            } else {
                c.clamp(lo + h, hi - h)
            }
        };
        cam.center = vec2(
            clamp(cam.center.x, b.x, b.x + b.w, half.x),
            clamp(cam.center.y, b.y, b.y + b.h, half.y),
        );
    }

    pub fn look(&mut self, l: &sim::LookRef) -> PersonLook {
        let key = (l.seed, l.staff, l.preset.clone());
        *self.looks.entry(key).or_insert_with(|| wv::look_of(l))
    }

    /// Start a train crossing (called on `SimEvent::TrainPassing`).
    pub fn start_train(&mut self) {
        self.train = Some(0.0);
    }

    /// Animation step (real seconds, already scaled by the game speed if wanted).
    pub fn update(&mut self, dt: f32) {
        if let Some(p) = &mut self.train {
            *p += dt / TRAIN_SECONDS;
            if *p > 1.05 {
                self.train = None;
            }
        }
    }

    fn statics_key(world: &World) -> u64 {
        let g = world.grid();
        let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ (g.width as u64) << 32 ^ g.height as u64;
        for t in &g.tiles {
            h = (h ^ (t.kind as u64 * 7 + t.floor as u64 + 1)).wrapping_mul(0x0100_0000_01b3);
        }
        h
    }

    fn rebuild_statics(&mut self, world: &World) {
        let g = world.grid();
        self.size = (g.width, g.height);
        let w = g.width;
        self.street = Rect::new(
            -8.0 * TILE,
            -10.0 * TILE,
            (w as f32 + 16.0) * TILE,
            8.55 * TILE,
        );
        self.street_lights = outside_lights(self.street);
        let tile = |x: i32, y: i32| g.tile(x, y);
        let is_wall = |x: i32, y: i32| tile(x, y).is_some_and(|t| is_wall_kind(t.kind));
        let is_floor =
            |x: i32, y: i32| tile(x, y).is_some_and(|t| matches!(t.kind, sim::TileKind::Floor));
        self.glass.clear();
        for y in 0..g.height {
            for x in 0..g.width {
                if tile(x, y).is_some_and(|t| t.kind == sim::TileKind::Window) {
                    self.glass.push(window_glass_rect(vec2(x as f32, y as f32)));
                }
            }
        }
        let rect = Rect::new(
            -TILE,
            -2.0 * TILE,
            (g.width as f32 + 2.0) * TILE,
            (g.height as f32 + 3.0) * TILE,
        );
        let mut low = Vec::new();
        let statics = self.statics.get_or_insert_with(StaticLayer::new);
        statics.build(rect, 2.0, || {
            for y in 0..g.height {
                for x in 0..g.width {
                    let Some(t) = tile(x, y) else { continue };
                    let p = vec2(x as f32, y as f32);
                    match t.kind {
                        sim::TileKind::Wall | sim::TileKind::Window => {}
                        _ => draw_floor(t.floor.id(), p),
                    }
                }
            }
            for y in 0..g.height {
                for x in 0..g.width {
                    let Some(t) = tile(x, y) else { continue };
                    if !is_wall_kind(t.kind) {
                        continue;
                    }
                    let m = WallMask::compute(x, y, is_wall, is_floor);
                    let kind = match t.kind {
                        sim::TileKind::Window => WallKind::Window,
                        sim::TileKind::Door => WallKind::Door,
                        _ => WallKind::Plain,
                    };
                    let pos = vec2(x as f32, y as f32);
                    if m.is_low() {
                        low.push((pos, m, kind));
                    } else {
                        draw_wall(pos, m, kind);
                    }
                }
            }
        });
        self.low_walls = low;
        if let Some(l) = &mut self.lighting {
            l.set_windows(self.glass.clone());
        }
    }

    /// Refreshes cached people views (call once per frame before drawing / picking).
    /// `alpha` = fraction of the next tick already elapsed: feet are interpolated between
    /// the last two sim ticks so walking is smooth at 60 fps.
    pub fn refresh(&mut self, world: &World, now: f32, alpha: f32) {
        self.people = world.staff_views();
        self.people.extend(world.guest_views());
        let tick = world.tick_count();
        let a = alpha.clamp(0.0, 1.0);
        for v in self.people.iter_mut() {
            let k = akey(v.agent);
            let cur = vec2(v.feet.0, v.feet.1);
            let e = self.tracks.entry(k).or_insert((cur, cur, tick));
            if e.2 != tick {
                e.0 = e.1;
                e.1 = cur;
                e.2 = tick;
            }
            if e.0.distance(e.1) < 1.5 * TILE && v.seat_tile.is_none() {
                let p = e.0.lerp(e.1, a);
                v.feet = (p.x, p.y);
            }
        }
        let alive: Vec<(u8, u32)> = self.people.iter().map(|v| akey(v.agent)).collect();
        self.tracks.retain(|k, _| alive.contains(k));
        let mut seen = Vec::with_capacity(self.people.len());
        for v in &self.people {
            seen.push(akey(v.agent));
            match v.emote.map(wv::emote) {
                Some(e) => {
                    let entry = self.emote_started.entry(akey(v.agent)).or_insert((e, now));
                    if entry.0 != e {
                        *entry = (e, now);
                    }
                }
                None => {
                    self.emote_started.remove(&akey(v.agent));
                }
            }
        }
        self.emote_started.retain(|k, _| seen.contains(k));
    }

    /// Draws the whole café. Leaves `cam` active.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        world: &World,
        content: &Content,
        cam: &Camera2D,
        t: f32,
        selected: Option<AgentRef>,
        hovered: Option<AgentRef>,
        extra: &mut dyn FnMut(&mut RenderQueue),
    ) {
        self.draw_with(world, content, cam, t, selected, hovered, extra, None);
    }

    /// Draws the live café as the set of a story cutscene: the player's camera, actors,
    /// train and lamps (dark café, the lighting-up cascade) over the real map, furniture,
    /// weather and the frozen night. People the scene plays itself are hidden here.
    pub fn draw_cutscene(
        &mut self,
        world: &World,
        content: &Content,
        t: f32,
        player: &CutscenePlayer,
    ) {
        let cam = player.world_camera();
        self.draw_with(
            world,
            content,
            &cam,
            t,
            None,
            None,
            &mut |_| {},
            Some(player),
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_with(
        &mut self,
        world: &World,
        content: &Content,
        cam: &Camera2D,
        t: f32,
        selected: Option<AgentRef>,
        hovered: Option<AgentRef>,
        extra: &mut dyn FnMut(&mut RenderQueue),
        cs: Option<&CutscenePlayer>,
    ) {
        if self.lighting.is_none() {
            self.lighting = Some(Lighting::new());
        }
        let key = Self::statics_key(world);
        if key != self.statics_key || !self.statics.as_ref().is_some_and(|s| s.is_built()) {
            self.rebuild_statics(world);
            self.statics_key = key;
            if let Some(l) = &mut self.lighting {
                l.set_windows(self.glass.clone());
            }
        }
        let blackout = world.blackout();
        let mut weather = wv::rain_of(world.weather());
        // the opening always arrives in the rain
        if cs.is_some_and(|p| p.current_id() == "opening") && weather.intensity < 0.5 {
            weather = Weather::default();
        }
        // stage lights: 1 = the café as the sim has it, 0 = dark and closed
        let level = cs.map_or(1.0, |p| p.scene.light_level());
        let lamp_k = |sprite: &str, origin: Vec2| match cs {
            Some(p) if is_lamp_sprite(sprite) => p.scene.lamp_at(origin),
            _ => 1.0,
        };
        set_camera(cam);
        clear_background(pal::NIGHT_0);
        let ms = vec2(self.size.0 as f32, self.size.1 as f32) * TILE;
        radial(
            ms.x / 2.0,
            ms.y / 2.0 + TILE,
            ms.x * 0.75,
            ms.y * 0.9,
            pal::VIOLET_NIGHT_1,
            alpha(pal::NIGHT_0, 0.0),
        );
        let train_p = match cs {
            Some(p) => p.train_progress().unwrap_or(-1.0),
            None => self.train.unwrap_or(-1.0),
        };
        draw_outside(self.street, t);
        draw_train(self.street, train_p, t);
        draw_holo_sign(
            vec2(21.0 * TILE, self.street.y + self.street.h - 0.35 * TILE),
            world.cafe_name(),
            t,
        );
        weather.draw_rain(self.street, t);
        if let Some(st) = &self.statics {
            st.draw();
        }
        for g in &self.glass {
            weather.draw_window_rain(*g, t);
        }
        // rain over the sidewalk in front of the door
        let walk = Rect::new(
            -TILE,
            self.indoor_rows() as f32 * TILE,
            ms.x + 2.0 * TILE,
            2.0 * TILE,
        );
        weather.draw_rain(walk, t + 3.1);

        // furniture
        let placed = world.furniture();
        let views = world.furniture_views(content);
        let mut pieces: Vec<(String, Vec2, (u8, u8), FurnitureState)> =
            Vec::with_capacity(placed.len());
        for (p, v) in placed.iter().zip(&views) {
            let sprite = content
                .furniture_def(&p.def)
                .map(|d| d.sprite.clone())
                .unwrap_or_else(|| p.def.clone());
            let origin = vec2(v.tile.0 as f32, v.tile.1 as f32) * TILE;
            let st = FurnitureState {
                active: v.active && lamp_k(&sprite, origin) > 0.5,
                dirty: v.dirty,
                t,
                facing: wv::facing(v.facing),
            };
            pieces.push((sprite, origin, (v.size.0 as u8, v.size.1 as u8), st));
        }
        // people (in a cutscene: the frozen night minus the ones the scene plays itself)
        let mut looks = Vec::with_capacity(self.people.len());
        let all_people = std::mem::take(&mut self.people);
        let staged_people: Vec<AgentView>;
        let people: &[AgentView] = match cs {
            None => &all_people,
            Some(p) => {
                let staged: Vec<&str> = p.stage_cast().collect();
                staged_people = all_people
                    .iter()
                    .filter(|v| !staged.contains(&v.look.source.as_str()))
                    .cloned()
                    .map(|mut v| {
                        // the clock is stopped: nobody walks on the spot
                        if v.pose == sim::Pose::Walk {
                            v.pose = sim::Pose::Idle;
                        }
                        v
                    })
                    .collect();
                &staged_people
            }
        };
        for v in people {
            looks.push(self.look(&v.look));
        }
        let poses: Vec<PersonPose> = people.iter().map(|v| wv::person_pose(v, t)).collect();

        let mut q = RenderQueue::new();
        for (sprite, origin, size, st) in &pieces {
            q.push_furniture(sprite, *origin, *size, *st);
        }
        for (pos, m, k) in &self.low_walls {
            q.push_wall(*pos, *m, *k);
        }
        for ((v, look), pose) in people.iter().zip(&looks).zip(&poses) {
            if Some(v.agent) == selected {
                q.push_selection(pose);
            }
            q.push_person(look, *pose);
            // props: a cup in front of seated guests, the laptop of the quiet guy
            if v.pose == sim::Pose::Sit {
                let f = pose.facing;
                let dir = match f {
                    Facing::Left => vec2(-1.0, 0.0),
                    Facing::Right => vec2(1.0, 0.0),
                    Facing::Up => vec2(0.0, -1.0),
                    Facing::Down => vec2(0.0, 1.0),
                };
                if let Some((sx, sy)) = v.seat_tile {
                    let table = vec2(sx as f32, sy as f32) + dir;
                    if v.look.preset.as_deref() == Some("anwar") {
                        let side = if f == Facing::Left { 0.58 } else { 0.42 };
                        let at = (table + vec2(side, 0.5)) * TILE - vec2(0.0, 24.0);
                        q.push_custom(table.y * TILE + TILE + 0.01, move || draw_laptop(at, f, t));
                    } else if v.holding.is_some() {
                        let side = if f == Facing::Left { 0.7 } else { 0.3 };
                        let at = (table + vec2(side, 0.42)) * TILE - vec2(0.0, 30.0);
                        let phase = (sx * 7 + sy * 3) as f32;
                        q.push_custom(table.y * TILE + TILE + 0.02, move || {
                            draw_cup(at, 0.85, t + phase, true)
                        });
                    }
                }
            }
        }
        // the scene's own actors
        let actor_poses: Vec<PersonPose> = cs
            .map(|p| p.actors.iter().map(|a| a.pose(t)).collect())
            .unwrap_or_default();
        if let Some(p) = cs {
            for (a, pose) in p.actors.iter().zip(&actor_poses) {
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
        }
        // roof leaks: a bucket under the drip
        let buckets: Vec<(Vec2, FurnitureState)> = world
            .bucket_tiles()
            .into_iter()
            .map(|(x, y)| {
                let st = FurnitureState {
                    active: true,
                    dirty: 0.5,
                    t,
                    ..Default::default()
                };
                (vec2(x as f32, y as f32) * TILE, st)
            })
            .collect();
        for (o, st) in &buckets {
            q.push_furniture("bucket", *o, (1, 1), *st);
        }
        extra(&mut q);
        q.flush();

        // light
        let ambient = if blackout {
            ambient_blackout()
        } else {
            let p = if world.phase() == sim::Phase::Day {
                // a cutscene before the first night plays in the evening, not at noon
                if cs.is_some() { 0.15 } else { 0.02 }
            } else {
                world.night_progress().clamp(0.0, 1.0)
            };
            mix(ambient_blackout(), ambient_for(p), level)
        };
        let lighting = self.lighting.as_mut().expect("created above");
        lighting.begin(cam, ambient);
        lighting.set_time(t);
        lighting.add_window_light(if blackout { 0.55 } else { 0.35 + 0.65 * level });
        lighting.train_sweep(train_p);
        for (i, (sprite, origin, size, st)) in pieces.iter().enumerate() {
            let k = lamp_k(sprite, *origin);
            for mut l in furniture_lights(sprite, *origin, *size, st) {
                // switching on: a short warm flicker
                if k < 1.0 {
                    l.intensity *= k * (0.75 + 0.25 * (t * 41.0 + i as f32).sin().abs());
                }
                lighting.add_light(l);
            }
        }
        if !blackout {
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
        }
        for (v, pose) in people.iter().zip(&poses) {
            if blackout && v.pose == sim::Pose::Sit {
                lighting.add_light(Light::phone(pose.feet - vec2(0.0, 40.0)));
            }
            if Some(v.agent) == selected {
                lighting.add_light(Light::new(pose.feet, 0.8 * TILE, pal::INDIGO_GLOW, 0.5));
            }
            if v.look.preset.as_deref() == Some("anwar")
                && v.pose == sim::Pose::Sit
                && let Some((sx, sy)) = v.seat_tile
            {
                let dx = if pose.facing == Facing::Left {
                    -1.0
                } else {
                    1.0
                };
                lighting.add_light(Light::phone(
                    vec2(sx as f32 + dx + 0.5, sy as f32 + 0.5) * TILE - vec2(0.0, 34.0),
                ));
            }
        }
        if let Some(p) = cs {
            for (a, pose) in p.actors.iter().zip(&actor_poses) {
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
        }
        lighting.apply();

        // hover ring + emotes on top of the light (not in cutscenes)
        if cs.is_some() {
            self.people = all_people;
            return;
        }
        for ((v, look), pose) in people.iter().zip(&looks).zip(&poses) {
            if Some(v.agent) == hovered && Some(v.agent) != selected {
                draw_actor_highlight(pose.feet, t);
            }
            if let Some((e, start)) = self.emote_started.get(&akey(v.agent)) {
                let age = (t - start).max(0.0);
                draw_emote_ex(emote_anchor(look, pose), *e, age, t);
            }
        }
        self.people = all_people;
    }

    /// What is under the world point `p` (people first, then furniture, then the tile).
    pub fn pick(&self, world: &World, content: &Content, p: Vec2, t: f32) -> Option<Pick> {
        let mut best: Option<(f32, AgentRef)> = None;
        for v in &self.people {
            let pose = wv::person_pose(v, t);
            let f = pose.feet;
            let tall = if v.pose == sim::Pose::Sit { 62.0 } else { 84.0 };
            if (p.x - f.x).abs() < 20.0 && p.y < f.y + 8.0 && p.y > f.y - tall {
                // the one drawn on top (largest y) wins
                if best.is_none_or(|(y, _)| f.y > y) {
                    best = Some((f.y, v.agent));
                }
            }
        }
        if let Some((_, a)) = best {
            return Some(Pick::Agent(a));
        }
        let (tx, ty) = ((p.x / TILE).floor() as i32, (p.y / TILE).floor() as i32);
        let _ = content;
        if let Some(uid) = world.grid().object_at(tx, ty) {
            return Some(Pick::Furniture(uid));
        }
        world.grid().in_bounds(tx, ty).then_some(Pick::Tile(tx, ty))
    }

    /// Screen-independent world position of an agent's head (bubbles, follow camera).
    pub fn head_of(&self, a: AgentRef, t: f32) -> Option<Vec2> {
        let v = self.people.iter().find(|v| v.agent == a)?;
        let pose = wv::person_pose(v, t);
        let h = if v.pose == sim::Pose::Sit { 70.0 } else { 92.0 };
        Some(pose.feet - vec2(0.0, h))
    }
}
