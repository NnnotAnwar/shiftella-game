//! The live café, drawn from the sim state with the render module (its documented frame
//! order): street + train + rain behind the back wall, the cached static layer (floors, tall
//! walls), a y-sorted queue (furniture, low front walls, people, props), lighting (ambient by
//! night progress, lamps, windows, blackout, train sweep, phone lights) and emotes on top.

use std::collections::HashMap;

use macroquad::prelude::*;

use super::scene_fx::{self as fx, Visuals};
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
    tracks: HashMap<(u8, u32), (Vec2, Vec2, u64)>,
    /// Sim steps run so far (counted by the screen). People's feet are interpolated between
    /// the last two *steps*, not night ticks: the night tick stands still during the
    /// preparation half hour while everyone walks.
    pub sim_steps: u64,
    /// Guests served by Karolina: their cup glyph (+ latte art) on the table.
    pub served: HashMap<u32, (CupGlyph, Option<LatteArt>)>,
    /// Dirt blots being mopped: tile → `t` when the mop went over it (a little shine).
    pub shine: Vec<((i32, i32), f32)>,
    /// Tonight's season / theme / garlands (read from the sim each frame).
    pub vis: Visuals,
    /// Garlands over the back windows and along the counter, fairy lights (winter theme).
    garlands: Option<(crate::render::Garlands, crate::render::Garlands)>,
    fairy: Option<crate::render::Garlands>,
    /// Moment markers drawn last frame (id, marker): the screen hit-tests their orbs.
    pub markers: Vec<(String, MomentMarker)>,
    marker_born: HashMap<String, f32>,
    /// The moment marker under the mouse (it grows a little).
    pub hover_moment: Option<String>,
    /// The coziness meter is hovered: its weakest part (index into `CozyComponent::ALL`).
    pub cozy_hint: Option<usize>,
    /// `t` when the blackout candles were lit (they light up table by table).
    candles_since: Option<f32>,
    /// Real seconds per game minute at ×1 (marker fade).
    pub sec_per_minute: f32,
    /// `t` when the doors opened tonight (the door sign flips).
    pub doors_opened_at: Option<f32>,
    /// Placed pieces not drawn this frame (build mode: the group being dragged).
    pub hidden_furniture: Vec<u32>,
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
            sim_steps: 0,
            served: HashMap::new(),
            shine: Vec::new(),
            vis: Visuals::default(),
            garlands: None,
            fairy: None,
            markers: Vec::new(),
            marker_born: HashMap::new(),
            hover_moment: None,
            cozy_hint: None,
            candles_since: None,
            sec_per_minute: 1.5,
            doors_opened_at: None,
            hidden_furniture: Vec::new(),
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
        // automation: `SHIFTELLA_TRAIN_P=0.4` freezes a passing train at that point
        static TRAIN_P: std::sync::OnceLock<Option<f32>> = std::sync::OnceLock::new();
        if let Some(p) = *TRAIN_P.get_or_init(|| {
            std::env::var("SHIFTELLA_TRAIN_P").ok().and_then(|v| v.parse().ok())
        }) {
            self.train = Some(p);
            return;
        }
        if let Some(p) = &mut self.train {
            *p += dt / TRAIN_SECONDS;
            if *p > 1.05 {
                self.train = None;
            }
        }
    }

    fn statics_key(world: &World, vis: &Visuals) -> u64 {
        let g = world.grid();
        let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ (g.width as u64) << 32 ^ g.height as u64;
        h ^= vis.static_key();
        for t in &g.tiles {
            h = (h ^ (t.kind as u64 * 7 + t.floor as u64 + 1)).wrapping_mul(0x0100_0000_01b3);
        }
        h
    }

    fn rebuild_statics(&mut self, world: &World, content: &Content) {
        let g = world.grid();
        let vis = self.vis;
        let look = vis.look;
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
        // the hall wears the theme; the bar and the kitchen (tiles) keep the base look
        set_decor(vis.style);
        statics.build(rect, 2.0, || {
            for y in 0..g.height {
                for x in 0..g.width {
                    let Some(t) = tile(x, y) else { continue };
                    let p = vec2(x as f32, y as f32);
                    match t.kind {
                        sim::TileKind::Wall | sim::TileKind::Window => {}
                        _ => {
                            let kind = t.floor.id();
                            if kind == "floor_tile" {
                                with_decor(ThemeStyle::DEFAULT, || draw_floor(kind, p));
                            } else {
                                draw_floor(kind, p);
                            }
                            draw_floor_season(kind, p, &look);
                            // the doorstep: a wet leaf, slushy footprints
                            if tile(x, y + 1).is_some_and(|b| b.kind == sim::TileKind::Door) {
                                draw_doorstep_season(p, &look);
                            }
                        }
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
        self.garlands = Some(fx::garlands(&self.glass, fx::counter_span(world, content)));
        self.fairy = Some(fx::fairy_lights(g.width));
    }

    /// Refreshes cached people views (call once per frame before drawing / picking).
    /// `alpha` = fraction of the next tick already elapsed: feet are interpolated between
    /// the last two sim ticks so walking is smooth at 60 fps.
    pub fn refresh(&mut self, world: &World, now: f32, alpha: f32) {
        self.people = world.staff_views();
        self.people.extend(world.guest_views());
        let tick = self.sim_steps;
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
        self.vis = fx::visuals(world, content);
        let vis = self.vis;
        let look = vis.look;
        let counter = fx::counter_span(world, content);
        let ckey = counter.map_or(0, |c| (c.0 as u64) << 40 ^ (c.1 as u64) << 20 ^ c.2 as u64);
        let key = Self::statics_key(world, &vis) ^ ckey;
        if key != self.statics_key || !self.statics.as_ref().is_some_and(|s| s.is_built()) {
            self.rebuild_statics(world, content);
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
        // the preparation half hour: the café lights come up over its first minutes
        let warm_up = if cs.is_none() && world.in_prep() {
            let k = (world.prep_progress() / 0.2).clamp(0.0, 1.0);
            0.55 + 0.45 * k * k * (3.0 - 2.0 * k)
        } else {
            1.0
        };
        let level = cs.map_or(warm_up, |p| {
            let l = p.scene.light_level();
            // closing time in the endings: dawn already greys the room, it never goes black
            if p.current_id().starts_with("ending") {
                0.3 + 0.7 * l
            } else {
                l
            }
        });
        let lamp_k = |sprite: &str, origin: Vec2| match cs {
            Some(p) if is_lamp_sprite(sprite) => p.scene.lamp_at(origin),
            _ => 1.0,
        };
        set_decor(vis.style);
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
        draw_outside_season(self.street, &look, t);
        draw_train(self.street, train_p, t);
        let sign = vec2(21.0 * TILE, self.street.y + self.street.h - 0.35 * TILE);
        draw_holo_sign(sign, world.cafe_name(), t);
        draw_sign_season(sign, world.cafe_name(), &look, t);
        weather.draw_rain(self.street, t);
        Snowfall::from_look(&look).draw(self.street, t);
        draw_falling_leaves(self.street, look.leaves * 0.7, 0.25 + weather.wind, t);
        if let Some(st) = &self.statics {
            st.draw();
        }
        for g in &self.glass {
            weather.draw_window_rain(*g, t);
            draw_window_season(*g, &look, t);
        }
        if vis.fairy
            && let Some(f) = &self.fairy
        {
            f.draw(1.0, t);
        }
        // grandma's garlands over the windows (right after the static walls)
        if look.festive > 0.01
            && let Some((win, _)) = &self.garlands
        {
            win.draw(vis.garlands, t);
        }
        // dirt blots on the hall floor (mop them with a swipe), and the shine where she mopped
        if cs.is_none() {
            for (x, y) in world.dirt_spots() {
                let c = vec2(x as f32 + 0.5, y as f32 + 0.55) * TILE;
                draw_dirt_blot(c, x * 31 + y * 17);
                // colour-blind friendly: a dashed "mop here" ring (shape, not only a tint)
                if crate::render::marker_shapes() {
                    dashed_ellipse(c, 24.0, 10.5, Color::new(0.88, 0.88, 1.0, 0.55));
                }
            }
            for (tile, t0) in &self.shine {
                let age = t - t0;
                if (0.0..1.4).contains(&age) {
                    let c = vec2(tile.0 as f32 + 0.5, tile.1 as f32 + 0.55) * TILE;
                    let k = age / 1.4;
                    draw_ellipse(
                        c.x,
                        c.y,
                        22.0 + 8.0 * k,
                        9.0 + 3.0 * k,
                        0.0,
                        alpha(pal::INDIGO_GLOW, 0.22 * (1.0 - k)),
                    );
                }
            }
        }
        // rain over the sidewalk in front of the door
        let walk = Rect::new(
            -TILE,
            self.indoor_rows() as f32 * TILE,
            ms.x + 2.0 * TILE,
            2.0 * TILE,
        );
        weather.draw_rain(walk, t + 3.1);
        Snowfall::from_look(&look).draw(walk, t + 1.7);

        // furniture
        let placed = world.furniture();
        let views = world.furniture_views(content);
        let mut pieces: Vec<(String, Vec2, (u8, u8), FurnitureState)> =
            Vec::with_capacity(placed.len());
        for (p, v) in placed.iter().zip(&views) {
            if self.hidden_furniture.contains(&p.uid) {
                continue;
            }
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
        // the bar's own pendant lamps: a warm row over the counter (part of the café, not
        // buildable furniture), lit whenever the power is on
        if let Some((x0, x1, front)) = counter {
            let row = ((front + 8.0) / TILE).floor() - 1.0;
            for x in bar_pendant_columns(x0, x1) {
                let origin = vec2(x, row * TILE);
                let st = FurnitureState {
                    active: !blackout && lamp_k("pendant_lamp", origin) > 0.5,
                    t,
                    ..Default::default()
                };
                pieces.push(("pendant_lamp".to_string(), origin, (1, 1), st));
            }
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
        // the appliances' evening (v0.2): the warm lamp, the pot, the cake, the milk, the bag
        for (o, size, look) in super::hands_view::appliance_looks(world, content) {
            q.push_custom(crate::render::appliance_sort_key(o, size), move || {
                crate::render::draw_appliance_state(o, size, &look, t)
            });
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
                        // Karolina's own cups show their glyph (and the latte art)
                        let served = match v.agent {
                            AgentRef::Guest(id) => self.served.get(&id).copied(),
                            AgentRef::Staff(_) => None,
                        };
                        match served {
                            Some((glyph, art)) => {
                                q.push_custom(table.y * TILE + TILE + 0.02, move || {
                                    draw_served_cup(at + vec2(0.0, 6.0), glyph, art, t + phase)
                                });
                            }
                            None => q.push_custom(table.y * TILE + TILE + 0.02, move || {
                                draw_cup(at, 0.85, t + phase, true)
                            }),
                        }
                    }
                }
            }
        }
        // the helpers' pastime props (a cup and cloth, a broom, a glass of water…)
        if cs.is_none() {
            super::staff_life_view::push_props(world, people, &mut q, t);
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
        // grandma's tip jar beside the register (glows with the rush rhythm)
        if cs.is_none()
            && let Some(reg) = world.furniture().iter().find(|p| {
                content
                    .furniture_def(&p.def)
                    .is_some_and(|d| d.has_tag("register"))
            })
        {
            let at = vec2(reg.x as f32 * TILE - 10.0, reg.y as f32 * TILE + 34.0);
            let fill = (world.tonight().tips as f32 / 800.0).clamp(0.05, 1.0);
            let rhythm = (world.rhythm() as f32 / 5.0).clamp(0.0, 1.0);
            q.push_custom(reg.y as f32 * TILE + TILE + 0.5, move || {
                draw_tip_jar(at, fill, rhythm, t)
            });
        }
        // the season on the counter: the spruce garland, the Advent wreath, grandma's box
        if let Some((x0, x1, front)) = counter {
            if look.festive > 0.01
                && let Some((_, cg)) = &self.garlands
            {
                let cg = cg.clone();
                let p = vis.garlands;
                q.push_custom(front + 8.0 + 0.6, move || cg.draw(p, t));
            }
            if let Some(lit) = vis.wreath {
                let at = vec2(x0 + (x1 - x0) * 0.28, front - 52.0);
                q.push_custom(front + 8.0 + 0.7, move || draw_advent_wreath(at, lit, t));
            }
            let boxed = cs.is_none()
                && world
                    .active_moments()
                    .iter()
                    .any(|m| m.id == "m_hang_garlands")
                && vis.garlands < 0.05;
            if boxed {
                let o = vec2(x0 + (x1 - x0) * 0.45, front - TILE);
                let st = FurnitureState {
                    active: true,
                    t,
                    ..Default::default()
                };
                q.push_custom(front + 8.0 + 0.8, move || {
                    draw_furniture("garland_box", o, (1, 1), &st)
                });
            }
        }
        // the station cat
        if cs.is_none()
            && let Some((feet, pose, aff)) = fx::cat_at(world, content, people, t)
        {
            q.push_custom(feet.y, move || fx::draw_the_cat(feet, pose, t, aff));
        }
        // blackout candles: lit table by table once Karolina lights them
        let candles: Vec<(Vec2, f32)> = if world.candles_lit() {
            let since = *self.candles_since.get_or_insert(t);
            pieces
                .iter()
                .filter_map(|(sprite, origin, size, _)| table_candle_anchor(sprite, *origin, *size))
                .enumerate()
                .map(|(i, a)| (a, t - since - i as f32 * 0.45))
                .collect()
        } else {
            self.candles_since = None;
            Vec::new()
        };
        for (a, age) in &candles {
            let (a, age) = (*a, *age);
            if age > 0.0 {
                q.push_custom(a.y + 70.0, move || draw_candle_lighting(a, age, t));
            }
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
        // moment markers (world anchors; drawn after the light, their glow goes in it)
        self.markers = if cs.is_none() && world.phase() == sim::Phase::Night {
            fx::moment_markers(
                world,
                content,
                people,
                &mut self.marker_born,
                self.hover_moment.as_deref(),
                t,
                self.sec_per_minute,
            )
        } else {
            Vec::new()
        };
        let room = self.cafe_px();
        let lighting = self.lighting.as_mut().expect("created above");
        lighting.coziness = if cs.is_some() { 0.6 } else { vis.coziness };
        lighting.begin(cam, ambient);
        lighting.set_time(t);
        lighting.add_window_light(if blackout { 0.55 } else { 0.35 + 0.65 * level });
        if !blackout {
            // a soft warm fill over the whole interior: the café reads as a lit room against
            // the night street, the corners are dim but never murky
            let fill = alpha(INTERIOR_FILL, interior_fill_strength(ambient) * level);
            lighting.add_quad(
                [vec2(0.0, 0.0), vec2(room.x, 0.0), room, vec2(0.0, room.y)],
                [fill; 4],
            );
            // the back of the bar: a warm wash over the work area behind the counter
            if let Some((x0, x1, front)) = counter {
                let depth = (room.y - front).max(TILE);
                let mut x = x0 + TILE;
                while x < x1 {
                    lighting.add_light(Light::new(
                        vec2(x, front + depth * 0.45),
                        2.6 * TILE,
                        pal::AMBER,
                        0.24 * interior_fill_strength(ambient) / INTERIOR_FILL_STRENGTH * level,
                    ));
                    x += 2.5 * TILE;
                }
            }
        }
        lighting.train_sweep(train_p, self.street);
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
        if look.festive > 0.01
            && let Some((win, cg)) = &self.garlands
        {
            for l in win
                .lights(vis.garlands)
                .into_iter()
                .chain(cg.lights(vis.garlands))
            {
                lighting.add_light(l);
            }
        }
        if vis.fairy
            && let Some(f) = &self.fairy
        {
            for l in f.lights(1.0) {
                lighting.add_light(l);
            }
        }
        if let (Some(lit), Some((x0, x1, front))) = (vis.wreath, counter) {
            let at = vec2(x0 + (x1 - x0) * 0.28, front - 52.0);
            for l in advent_wreath_lights(at, lit) {
                lighting.add_light(l);
            }
        }
        for (a, age) in &candles {
            if *age > 0.0 {
                lighting.add_light(candle_lighting_light(*a, *age));
            }
        }
        for (_, m) in &self.markers {
            lighting.add_light(moment_marker_light(m, t));
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
        lighting.draw_dust_motes(t);
        if look.festive > 0.01
            && let Some((win, cg)) = &self.garlands
        {
            win.draw_glow(vis.garlands, t);
            cg.draw_glow(vis.garlands, t);
        }
        if vis.fairy
            && let Some(f) = &self.fairy
        {
            f.draw_glow(1.0, t);
        }
        // breath puffs for people out on the cold pavement
        if look.breath > 0.01 {
            let street_y = self.indoor_rows() as f32 * TILE;
            for ((v, lk), pose) in people.iter().zip(&looks).zip(&poses) {
                if v.feet.1 > street_y {
                    draw_breath(lk, pose, t, look.breath);
                }
            }
        }
        draw_falling_leaves(
            Rect::new(
                -TILE,
                self.indoor_rows() as f32 * TILE - 0.4 * TILE,
                ms.x + 2.0 * TILE,
                2.4 * TILE,
            ),
            look.leaves * 0.6,
            0.2,
            t + 4.0,
        );

        // hover ring + emotes on top of the light (not in cutscenes)
        if cs.is_some() {
            self.people = all_people;
            return;
        }
        if let Some(w) = self.cozy_hint {
            fx::draw_cozy_hints(world, content, w, t);
        }
        for (_, m) in &self.markers {
            draw_moment_marker(m, t);
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

/// A dashed ellipse ring (14 dashes) — the colour-blind "mop here" cue round a blot.
fn dashed_ellipse(c: Vec2, rx: f32, ry: f32, col: Color) {
    let n = 14;
    for i in 0..n {
        let a0 = i as f32 / n as f32 * std::f32::consts::TAU;
        let a1 = a0 + std::f32::consts::TAU / n as f32 * 0.55;
        draw_line(
            c.x + a0.cos() * rx,
            c.y + a0.sin() * ry,
            c.x + a1.cos() * rx,
            c.y + a1.sin() * ry,
            1.3,
            col,
        );
    }
}

/// A soft muddy blot on the floor (rain from the street, spilled coffee).
fn draw_dirt_blot(c: Vec2, seed: i32) {
    let col = Color::new(0.24, 0.17, 0.13, 0.42);
    let j = |k: i32| ((seed * 37 + k * 101) % 97) as f32 / 97.0 - 0.5;
    draw_ellipse(c.x, c.y, 17.0, 7.5, 0.0, col);
    draw_ellipse(c.x + 9.0 + j(1) * 6.0, c.y + 2.0, 8.0, 4.0, 0.0, col);
    draw_ellipse(c.x - 10.0 + j(2) * 5.0, c.y - 2.5, 7.0, 3.2, 0.0, col);
    draw_ellipse(
        c.x - 2.0,
        c.y - 1.5,
        10.0,
        3.5,
        0.0,
        Color::new(0.3, 0.22, 0.17, 0.3),
    );
    // two footprints of whoever walked through
    for (dx, dy) in [(4.0, -6.0), (12.0, -2.0)] {
        draw_ellipse(c.x + dx + j(3) * 4.0, c.y + dy, 2.6, 1.6, 0.0, col);
    }
}

/// Warm interior fill colour and its strength (alpha into the light map, × the power level).
const INTERIOR_FILL: Color = Color::new(1.0, 0.86, 0.74, 1.0);
const INTERIOR_FILL_STRENGTH: f32 = 0.34;

/// The fill follows the night: strongest in the deep indigo hours, gentle at dusk and dawn
/// (when the ambient is already light), so early evening never looks washed out.
fn interior_fill_strength(ambient: Color) -> f32 {
    let lum = 0.3 * ambient.r + 0.59 * ambient.g + 0.11 * ambient.b;
    let dark = ((0.6 - lum) / 0.32).clamp(0.3, 1.0);
    INTERIOR_FILL_STRENGTH * dark
}

/// Tile-aligned x positions (world px) for the bar's pendant lamps: one every ~2.5 tiles,
/// inset from the counter's ends.
fn bar_pendant_columns(x0: f32, x1: f32) -> Vec<f32> {
    let tiles = ((x1 - x0) / TILE).round() as i32;
    if tiles < 2 {
        return vec![x0];
    }
    let n = ((tiles as f32 / 2.5).round() as i32).clamp(1, 6);
    (0..n)
        .map(|i| {
            let f = (i as f32 + 0.5) / n as f32;
            x0 + (f * tiles as f32).floor() * TILE
        })
        .collect()
}

#[cfg(test)]
mod bar_light_tests {
    use super::*;

    #[test]
    fn bar_pendants_are_spread_over_the_counter_and_stay_on_it() {
        let cols = bar_pendant_columns(10.0 * TILE, 21.0 * TILE);
        assert!(cols.len() >= 3 && cols.len() <= 6, "{cols:?}");
        assert!(cols.windows(2).all(|w| w[1] - w[0] >= 2.0 * TILE), "{cols:?}");
        assert!(cols.iter().all(|x| *x >= 10.0 * TILE && *x < 21.0 * TILE));
        assert_eq!(bar_pendant_columns(4.0 * TILE, 5.0 * TILE), vec![4.0 * TILE]);
    }

    #[test]
    fn the_interior_fill_is_stronger_in_the_deep_night_than_at_dusk() {
        let dusk = interior_fill_strength(ambient_for(0.0));
        let deep = interior_fill_strength(ambient_for(0.55));
        assert!(deep > dusk * 1.5, "dusk {dusk}, deep {deep}");
        assert!(deep <= INTERIOR_FILL_STRENGTH && dusk > 0.0);
    }
}
