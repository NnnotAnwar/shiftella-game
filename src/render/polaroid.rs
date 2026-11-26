//! The polaroid journal (`design/handson.md` §11.4): a stored mini-scene re-drawn from
//! compact data ([`SceneSnap`]) into a render target with a photo grade, and the
//! polaroid frame around it ([`draw_polaroid`]).
//!
//! Saves store the *data*, never images: [`SnapRenderer::render`] is deterministic —
//! the same snap gives the same picture, even after the café has been rebuilt.
//!
//! ```ignore
//! let mut snaps = SnapRenderer::new();                 // once
//! let photo = snaps.render(&snap, 720, 540);           // when a journal page opens; keep it
//! // in the UI camera, every frame:
//! let lay = draw_polaroid(rect, Some(&photo.texture), &PolaroidStyle {
//!     tilt: polaroid_tilt(seed), develop: 1.0, caption: Some(&caption), date: Some("7. 11. 2039"),
//!     ..Default::default() });
//! ```

use macroquad::prelude::*;

use super::cat::{CatLook, CatPose, draw_cat_ex};
use super::colors as c;
use super::draw::*;
use super::festive::{GarlandStyle, Garlands};
use super::furniture::{FurnitureState, furniture_lights};
use super::handson::{CupGlyph, LatteArt, draw_served_cup};
use super::light::{Light, Lighting, ambient_blackout, ambient_for};
use super::look::{Emote, Expression, Facing, PersonLook, PersonPose, Pose};
use super::outside::{Weather, draw_holo_sign, draw_outside, draw_train, world_text_rot};
use super::person::{draw_emote_ex, emote_anchor};
use super::queue::RenderQueue;
use super::season::{
    Season, Snowfall, draw_breath, draw_floor_season, draw_outside_season, draw_sign_season,
    draw_window_season,
};
use super::theme::{DecorTheme, with_decor};
use super::tiles::{WallKind, WallMask, draw_floor, draw_wall, window_glass_rect};
use crate::map::TILE;

/// The warm white of the instant camera's flash.
const FLASH: Color = Color::new(1.0, 0.93, 0.84, 1.0);

/// A floor in a snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SnapFloor {
    #[default]
    Wood,
    Tile,
    Carpet,
    Pavement,
}

impl SnapFloor {
    pub fn id(self) -> &'static str {
        match self {
            SnapFloor::Wood => "floor_wood",
            SnapFloor::Tile => "floor_tile",
            SnapFloor::Carpet => "floor_carpet",
            SnapFloor::Pavement => "pavement",
        }
    }

    pub fn from_id(id: &str) -> SnapFloor {
        match id {
            "floor_tile" => SnapFloor::Tile,
            "floor_carpet" => SnapFloor::Carpet,
            "pavement" => SnapFloor::Pavement,
            _ => SnapFloor::Wood,
        }
    }
}

/// One tile of the snapshot's map window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SnapTile {
    /// Outside the café (nothing is drawn: the night shows through).
    #[default]
    Void,
    Floor(SnapFloor),
    Wall,
    Window,
    Door(SnapFloor),
}

impl SnapTile {
    fn is_wall(self) -> bool {
        matches!(self, SnapTile::Wall | SnapTile::Window | SnapTile::Door(_))
    }
}

/// A furniture piece (map tile coordinates, like the sim's placed furniture).
#[derive(Debug, Clone, PartialEq)]
pub struct SnapProp {
    pub sprite: String,
    pub tile: (i32, i32),
    pub size: (u8, u8),
    pub facing: Facing,
    pub on: bool,
    pub dirty: f32,
}

/// A person in the snapshot (world px in map coordinates).
#[derive(Debug, Clone, PartialEq)]
pub struct SnapActor {
    pub look: PersonLook,
    pub feet: Vec2,
    pub facing: Facing,
    pub pose: Pose,
    pub expression: Expression,
    pub emote: Option<Emote>,
}

/// A served cup with its latte art.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapCup {
    pub pos: Vec2,
    pub glyph: CupGlyph,
    pub art: Option<LatteArt>,
}

/// A compact scene snapshot (~1–3 KB once serialized by the sim's own type).
#[derive(Debug, Clone, PartialEq)]
pub struct SceneSnap {
    /// The top-left tile of the window in map coordinates.
    pub origin: (i32, i32),
    /// The window in tiles (10×8 by default).
    pub size: (u8, u8),
    /// Row-major, `size.0 * size.1` tiles (missing ones are `Void`).
    pub tiles: Vec<SnapTile>,
    pub props: Vec<SnapProp>,
    pub actors: Vec<SnapActor>,
    pub cups: Vec<SnapCup>,
    /// The station cat, if it was there.
    pub cat: Option<(Vec2, CatPose)>,
    /// Night progress 0..1 (22:00 → 06:00).
    pub night: f32,
    /// Rain 0..2.
    pub rain: f32,
    pub season: Season,
    pub theme: DecorTheme,
    /// Garlands up 0..1 (over the windows in the frame).
    pub garlands: f32,
    pub blackout: bool,
    pub window_light: bool,
    pub coziness: f32,
    /// A train passing (0..1), if any.
    pub train: Option<f32>,
    /// The frozen animation time (flames, steam, rain).
    pub time: f32,
    /// Grain, tilt and small variations.
    pub seed: u32,
    /// Where the camera looks (world px); `None` = the middle of the window.
    pub focus: Option<Vec2>,
    /// How close the photo is: 1 = the whole window width, 1.5 = a closer portrait.
    pub zoom: f32,
}

impl Default for SceneSnap {
    fn default() -> Self {
        SceneSnap {
            origin: (0, 0),
            size: (10, 8),
            tiles: Vec::new(),
            props: Vec::new(),
            actors: Vec::new(),
            cups: Vec::new(),
            cat: None,
            night: 0.5,
            rain: 1.0,
            season: Season::LateAutumn,
            theme: DecorTheme::GrandmaOriginal,
            garlands: 0.0,
            blackout: false,
            window_light: true,
            coziness: 0.6,
            train: None,
            time: 3.0,
            seed: 1,
            focus: None,
            zoom: 1.35,
        }
    }
}

impl SceneSnap {
    /// The tile at map coordinates (x, y), `Void` outside the window.
    pub fn tile(&self, x: i32, y: i32) -> SnapTile {
        let (lx, ly) = (x - self.origin.0, y - self.origin.1);
        if lx < 0 || ly < 0 || lx >= self.size.0 as i32 || ly >= self.size.1 as i32 {
            return SnapTile::Void;
        }
        self.tiles
            .get((ly * self.size.0 as i32 + lx) as usize)
            .copied()
            .unwrap_or_default()
    }

    /// The world rect the photo frames (4:3, the walls' height included).
    pub fn view_rect(&self) -> Rect {
        let w = self.size.0 as f32 * TILE / self.zoom.clamp(0.5, 4.0);
        let h = w * 0.75;
        let c = self.focus.unwrap_or_else(|| {
            let region_h = self.size.1 as f32 * TILE;
            // the middle of the region, nudged up to show the tall back wall
            vec2(
                (self.origin.0 as f32 + self.size.0 as f32 / 2.0) * TILE,
                self.origin.1 as f32 * TILE + region_h / 2.0 - 0.55 * TILE,
            )
        });
        Rect::new(c.x - w / 2.0, c.y - h / 2.0, w, h)
    }
}

/// The deterministic tilt of a polaroid (radians, −3…+3°).
pub fn polaroid_tilt(seed: u32) -> f32 {
    (hash01(seed.wrapping_mul(2_246_822_519)) * 6.0 - 3.0).to_radians()
}

/// Renders snapshots. Create once and reuse (it owns a `Lighting`).
pub struct SnapRenderer {
    lighting: Lighting,
    opaque: Material,
}

impl Default for SnapRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl SnapRenderer {
    pub fn new() -> SnapRenderer {
        let mut lighting = Lighting::new();
        lighting.resolution = 0.5;
        SnapRenderer {
            lighting,
            opaque: opaque_material(),
        }
    }

    /// Render `snap` into a new `w`×`h` target (render at 2× the display size for smooth
    /// edges, e.g. 720×540 for a 360×270 photo). Leaves the default camera active.
    pub fn render(&mut self, snap: &SceneSnap, w: u32, h: u32) -> RenderTarget {
        let rt = render_target(w.max(8), h.max(6));
        crate::profile::count(crate::profile::Counter::RenderTargets, 1);
        rt.texture.set_filter(FilterMode::Linear);
        self.draw_scene_snap(snap, &rt);
        rt
    }

    /// Draw `snap` into an existing `target` (cleared first).
    pub fn draw_scene_snap(&mut self, snap: &SceneSnap, target: &RenderTarget) {
        let view = snap.view_rect();
        let mut cam = Camera2D::from_display_rect(view);
        cam.render_target = Some(target.clone());
        let t = snap.time;
        let look = snap.season.look();
        let style = snap.theme.style(snap.season);
        let (ox, oy) = snap.origin;
        let (sw, sh) = (snap.size.0 as i32, snap.size.1 as i32);

        set_camera(&cam);
        clear_background(c::NIGHT_0);
        radial(
            view.x + view.w / 2.0,
            view.y + view.h / 2.0,
            view.w * 0.8,
            view.h * 0.9,
            c::VIOLET_NIGHT_1,
            alpha(c::NIGHT_0, 0.0),
        );

        // the street behind the back wall (the same band as in the game)
        let street = Rect::new(
            (ox - 8) as f32 * TILE,
            -10.0 * TILE,
            (sw + 16) as f32 * TILE,
            8.55 * TILE,
        );
        let weather = Weather {
            intensity: snap.rain,
            ..Default::default()
        };
        let snow = Snowfall::from_look(&look);
        if view.y < street.y + street.h {
            draw_outside(street, t);
            draw_outside_season(street, &look, t);
            draw_train(street, snap.train.unwrap_or(-1.0), t);
            let sign = vec2(21.0 * TILE, street.y + street.h - 0.35 * TILE);
            draw_holo_sign(sign, "Café Nocturne", t);
            draw_sign_season(sign, "Café Nocturne", &look, t);
            weather.draw_rain(street, t);
            snow.draw(street, t);
        }

        let is_wall = |x: i32, y: i32| snap.tile(x, y).is_wall();
        let is_floor = |x: i32, y: i32| matches!(snap.tile(x, y), SnapTile::Floor(_));
        let mut glass = Vec::new();
        let mut low = Vec::new();
        with_decor(style, || {
            // floors
            for y in oy..oy + sh {
                for x in ox..ox + sw {
                    let p = vec2(x as f32, y as f32);
                    match snap.tile(x, y) {
                        SnapTile::Floor(f) | SnapTile::Door(f) => {
                            draw_floor(f.id(), p);
                            draw_floor_season(f.id(), p, &look);
                        }
                        _ => {}
                    }
                }
            }
            // tall walls now, low ones through the queue
            for y in oy..oy + sh {
                for x in ox..ox + sw {
                    let tile = snap.tile(x, y);
                    if !tile.is_wall() {
                        continue;
                    }
                    let m = WallMask::compute(x, y, is_wall, is_floor);
                    let kind = match tile {
                        SnapTile::Window => WallKind::Window,
                        SnapTile::Door(_) => WallKind::Door,
                        _ => WallKind::Plain,
                    };
                    let pos = vec2(x as f32, y as f32);
                    if kind == WallKind::Window {
                        glass.push(window_glass_rect(pos));
                    }
                    if m.is_low() {
                        low.push((pos, m, kind));
                    } else {
                        draw_wall(pos, m, kind);
                    }
                }
            }
            for g in &glass {
                weather.draw_window_rain(*g, t);
                draw_window_season(*g, &look, t);
            }
        });
        let garlands = Garlands::over_windows(&glass, GarlandStyle::WarmBulbs);
        with_decor(style, || {
            garlands.draw(snap.garlands, t);
            let mut q = RenderQueue::new();
            let states: Vec<FurnitureState> = snap
                .props
                .iter()
                .map(|p| FurnitureState {
                    active: if snap.blackout {
                        !is_lamp(&p.sprite)
                    } else {
                        p.on
                    },
                    dirty: p.dirty,
                    t,
                    facing: p.facing,
                })
                .collect();
            for (p, st) in snap.props.iter().zip(&states) {
                let o = vec2(p.tile.0 as f32, p.tile.1 as f32) * TILE;
                q.push_furniture(&p.sprite, o, p.size, *st);
            }
            for (pos, m, k) in &low {
                q.push_wall(*pos, *m, *k);
            }
            for a in &snap.actors {
                let pose = PersonPose {
                    feet: a.feet,
                    facing: a.facing,
                    pose: a.pose,
                    t,
                };
                q.push_person_ex(&a.look, pose, a.expression);
            }
            for cup in &snap.cups {
                let cup = *cup;
                q.push_custom(cup.pos.y + 0.5 * TILE, move || {
                    draw_served_cup(cup.pos, cup.glyph, cup.art, t)
                });
            }
            if let Some((feet, pose)) = snap.cat {
                q.push_custom(feet.y, move || {
                    draw_cat_ex(feet, &CatLook::default(), pose, t, 0.8)
                });
            }
            q.flush();

            // light (into the target)
            // an instant camera's flash lifts the darkness a little
            let ambient = mix(
                if snap.blackout {
                    ambient_blackout()
                } else {
                    ambient_for(snap.night)
                },
                WHITE,
                if snap.blackout { 0.22 } else { 0.1 },
            );
            let l = &mut self.lighting;
            l.set_windows(glass.clone());
            l.coziness = snap.coziness;
            l.begin(&cam, ambient);
            l.set_time(t);
            if snap.window_light || snap.blackout {
                l.add_window_light(if snap.blackout { 0.6 } else { 1.0 });
            }
            if let Some(p) = snap.train {
                l.train_sweep(p, street);
            }
            for (p, st) in snap.props.iter().zip(&states) {
                let o = vec2(p.tile.0 as f32, p.tile.1 as f32) * TILE;
                for li in furniture_lights(&p.sprite, o, p.size, st) {
                    l.add_light(li);
                }
            }
            for li in garlands.lights(snap.garlands) {
                l.add_light(li);
            }
            if snap.blackout {
                for a in &snap.actors {
                    if a.pose == Pose::Sit {
                        l.add_light(Light::phone(a.feet - vec2(0.0, 40.0)));
                    }
                }
            }
            // the instant camera's flash: a soft pool in the middle of the frame and a little
            // fill on the people in it, so a moment by the door (no lamp there) still reads
            let centre = vec2(view.x + view.w * 0.5, view.y + view.h * 0.55);
            l.add_light(Light::new(
                centre,
                view.w.max(view.h) * 0.62,
                FLASH,
                if snap.blackout { 0.5 } else { 0.42 },
            ));
            for a in &snap.actors {
                l.add_light(Light::new(
                    a.feet - vec2(0.0, 0.55 * TILE),
                    1.5 * TILE,
                    FLASH,
                    0.28,
                ));
            }
            l.apply();
            garlands.draw_glow(snap.garlands, t);
            l.draw_dust_motes(t);
        });
        // after the light: emotes, breath outside
        for a in &snap.actors {
            let pose = PersonPose {
                feet: a.feet,
                facing: a.facing,
                pose: a.pose,
                t,
            };
            let ty = (a.feet.y / TILE).floor() as i32;
            let tx = (a.feet.x / TILE).floor() as i32;
            if matches!(
                snap.tile(tx, ty),
                SnapTile::Floor(SnapFloor::Pavement) | SnapTile::Void
            ) {
                draw_breath(&a.look, &pose, t, look.breath);
            }
            if let Some(e) = a.emote {
                draw_emote_ex(emote_anchor(&a.look, &pose), e, 1.0, t);
            }
        }

        photo_grade(target, snap.seed);
        self.make_opaque(target);
        set_default_camera();
    }

    /// Translucent strokes leave alpha < 1 in the target; set it to 1 everywhere so the
    /// photo never lets the paper show through.
    fn make_opaque(&self, target: &RenderTarget) {
        let (w, h) = (target.texture.width(), target.texture.height());
        let mut cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, w, h));
        cam.render_target = Some(target.clone());
        set_camera(&cam);
        gl_use_material(&self.opaque);
        draw_rectangle(0.0, 0.0, w, h, WHITE);
        gl_use_default_material();
    }
}

const OPAQUE_VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying lowp vec4 color;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    color = color0 / 255.0;
}"#;
const OPAQUE_FRAGMENT: &str = r#"#version 100
precision mediump float;
varying lowp vec4 color;
void main() {
    gl_FragColor = vec4(0.0, 0.0, 0.0, 1.0);
}"#;

fn opaque_material() -> Material {
    use macroquad::miniquad::{BlendFactor, BlendState, Equation, PipelineParams};
    load_material(
        ShaderSource::Glsl {
            vertex: OPAQUE_VERTEX,
            fragment: OPAQUE_FRAGMENT,
        },
        MaterialParams {
            pipeline_params: PipelineParams {
                // colour: keep the destination; alpha: write 1
                color_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::Zero,
                    BlendFactor::One,
                )),
                alpha_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::One,
                    BlendFactor::Zero,
                )),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .expect("opaque snapshot shader")
}

fn is_lamp(sprite: &str) -> bool {
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
    )
}

/// The photo look over the finished picture: a warm wash, a soft vignette, a slight
/// flash highlight and fine procedural grain (seeded — deterministic).
fn photo_grade(target: &RenderTarget, seed: u32) {
    let (w, h) = (target.texture.width(), target.texture.height());
    let mut cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, w, h));
    cam.render_target = Some(target.clone());
    set_camera(&cam);
    // a warm, slightly faded wash (instant film)
    draw_rectangle(0.0, 0.0, w, h, Color::new(1.0, 0.78, 0.5, 0.07));
    draw_rectangle(0.0, 0.0, w, h, Color::new(0.35, 0.3, 0.55, 0.05));
    // the flash: a soft bright bloom in the middle-top
    glow(
        w * 0.5,
        h * 0.42,
        w * 0.55,
        Color::new(1.0, 0.95, 0.88, 0.09),
    );
    // vignette: darkened edges
    let e = w.min(h) * 0.28;
    let dark = Color::new(0.06, 0.03, 0.08, 0.55);
    let none = alpha(dark, 0.0);
    rect_v(0.0, 0.0, w, e, dark, none);
    rect_v(0.0, h - e, w, e, none, dark);
    rect_h(0.0, 0.0, e, h, dark, none);
    rect_h(w - e, 0.0, e, h, none, dark);
    // grain
    let n = (w * h / 260.0) as u32;
    for i in 0..n {
        let k = seed
            .wrapping_mul(747_796_405)
            .wrapping_add(i.wrapping_mul(2_891_336_453));
        let x = hash01(k) * w;
        let y = hash01(k ^ 0x9e37_79b9) * h;
        let v = hash01(k.wrapping_add(17));
        let col = if v > 0.5 {
            Color::new(1.0, 0.96, 0.9, 0.06 + (v - 0.5) * 0.12)
        } else {
            Color::new(0.1, 0.06, 0.14, 0.05 + v * 0.12)
        };
        draw_rectangle(x, y, 1.6, 1.6, col);
    }
}

// ───────────────────────── the polaroid frame ─────────────────────────

/// How to draw a polaroid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PolaroidStyle<'a> {
    /// Rotation in radians (see [`polaroid_tilt`]).
    pub tilt: f32,
    /// 0 = a fresh white print … 1 = fully developed (the morning "develop" animation).
    pub develop: f32,
    /// The handwritten-style caption in the bottom band.
    pub caption: Option<&'a str>,
    /// The date in the corner ("7. 11. 2039", localized by the caller).
    pub date: Option<&'a str>,
    /// A strip of washi tape at the top.
    pub tape: bool,
    /// Hover / selection lift 0..1 (a bigger, softer shadow).
    pub lift: f32,
}

impl Default for PolaroidStyle<'_> {
    fn default() -> Self {
        PolaroidStyle {
            tilt: 0.0,
            develop: 1.0,
            caption: None,
            date: None,
            tape: true,
            lift: 0.0,
        }
    }
}

/// Where the parts of a drawn polaroid are (unrotated, in the caller's camera).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PolaroidLayout {
    /// The whole print (unrotated; `tilt` turns it around its centre).
    pub frame: Rect,
    pub photo: Rect,
    /// The bottom band for the caption.
    pub caption: Rect,
    pub tilt: f32,
}

/// The layout of a polaroid `width` wide at `pos` (top-left). Proportions of a real
/// instant print: a 360×270 photo, a 14 px border, a 40 px bottom band.
pub fn polaroid_layout(pos: Vec2, width: f32, tilt: f32) -> PolaroidLayout {
    let k = width / 388.0;
    let b = 14.0 * k;
    let pw = width - 2.0 * b;
    let ph = pw * 0.75;
    let band = 46.0 * k;
    let frame = Rect::new(pos.x, pos.y, width, b + ph + band);
    PolaroidLayout {
        frame,
        photo: Rect::new(pos.x + b, pos.y + b, pw, ph),
        caption: Rect::new(pos.x + b, pos.y + b + ph, pw, band),
        tilt,
    }
}

fn rot_about(p: Vec2, pivot: Vec2, a: f32) -> Vec2 {
    let (s, c0) = a.sin_cos();
    let d = p - pivot;
    pivot + vec2(d.x * c0 - d.y * s, d.x * s + d.y * c0)
}

fn rot_rect(r: Rect, pivot: Vec2, a: f32) -> [Vec2; 4] {
    [
        rot_about(vec2(r.x, r.y), pivot, a),
        rot_about(vec2(r.x + r.w, r.y), pivot, a),
        rot_about(vec2(r.x + r.w, r.y + r.h), pivot, a),
        rot_about(vec2(r.x, r.y + r.h), pivot, a),
    ]
}

/// Draw a polaroid in the current (UI) camera: `rect.x/y/w` place it (the height follows
/// from the width). `photo` — a texture from [`SnapRenderer::render`] (`None` = an empty,
/// undeveloped print). Returns the layout so the UI can hit-test or add more.
pub fn draw_polaroid(rect: Rect, photo: Option<&Texture2D>, st: &PolaroidStyle) -> PolaroidLayout {
    let lay = polaroid_layout(vec2(rect.x, rect.y), rect.w, st.tilt);
    let f = lay.frame;
    let pivot = f.center();
    let a = st.tilt;
    let k = f.w / 388.0;

    // soft shadow: a few widening translucent layers, offset down-right
    let lift = st.lift.clamp(0.0, 1.0);
    for i in 0..5 {
        let g = (2.0 + i as f32 * 3.0) * k * (1.0 + lift);
        let off = vec2(4.0, 7.0) * k * (1.0 + lift * 1.5);
        let r = Rect::new(
            f.x - g + off.x,
            f.y - g + off.y,
            f.w + 2.0 * g,
            f.h + 2.0 * g,
        );
        let sc = Color::new(0.03, 0.02, 0.08, 0.07 - i as f32 * 0.01);
        quad(rot_rect(r, pivot + off, a), [sc; 4]);
    }
    // the paper
    let paper_t = Color::from_hex(0xfbf8f1);
    let paper_b = Color::from_hex(0xeee8dc);
    quad(rot_rect(f, pivot, a), [paper_t, paper_t, paper_b, paper_b]);
    // a faint paper texture
    let seed = (f.x as i32 as u32).wrapping_mul(31) ^ (f.y as i32 as u32);
    for i in 0..30 {
        let p = vec2(
            f.x + hash01(seed.wrapping_add(i * 3)) * f.w,
            f.y + hash01(seed.wrapping_add(i * 3 + 1)) * f.h,
        );
        let p = rot_about(p, pivot, a);
        draw_circle(p.x, p.y, 0.6 * k.max(0.5), Color::new(0.5, 0.45, 0.4, 0.08));
    }
    // the photo well (a hair darker) and the photo itself
    let well = Rect::new(
        lay.photo.x - 1.0,
        lay.photo.y - 1.0,
        lay.photo.w + 2.0,
        lay.photo.h + 2.0,
    );
    quad(
        rot_rect(well, pivot, a),
        [Color::new(0.2, 0.17, 0.2, 0.35); 4],
    );
    let dev = st.develop.clamp(0.0, 1.0);
    if let Some(tex) = photo {
        let ctr = lay.photo.center();
        let rc = rot_about(ctr, pivot, a);
        draw_texture_ex(
            tex,
            rc.x - lay.photo.w / 2.0,
            rc.y - lay.photo.h / 2.0,
            Color::new(1.0, 1.0, 1.0, 1.0),
            DrawTextureParams {
                dest_size: Some(lay.photo.size()),
                rotation: a,
                pivot: Some(rc),
                flip_y: true,
                ..Default::default()
            },
        );
    }
    if dev < 1.0 || photo.is_none() {
        // developing: a milky grey-white veil that clears, warm first
        let d = if photo.is_none() { 0.0 } else { dev };
        let veil = mix(
            Color::from_hex(0xd8d3cf),
            Color::from_hex(0x6a5a7a),
            (d * 1.6).min(1.0) * 0.3,
        );
        let va = (1.0 - d).powf(0.8);
        quad(rot_rect(lay.photo, pivot, a), [alpha(veil, va); 4]);
    }
    // a subtle glossy sheen across the photo
    let sheen = [
        alpha(WHITE, 0.0),
        alpha(WHITE, 0.07),
        alpha(WHITE, 0.0),
        alpha(WHITE, 0.0),
    ];
    let pr = lay.photo;
    quad(
        [
            rot_about(vec2(pr.x, pr.y + pr.h * 0.55), pivot, a),
            rot_about(vec2(pr.x + pr.w * 0.45, pr.y), pivot, a),
            rot_about(vec2(pr.x + pr.w * 0.7, pr.y), pivot, a),
            rot_about(vec2(pr.x, pr.y + pr.h * 0.95), pivot, a),
        ],
        sheen,
    );

    // caption & date
    let ink = Color::from_hex(0x3a3450);
    if let Some(text) = st.caption {
        // shrink to fit the band's width
        let mut size = (15.0 * k).clamp(3.0, 22.0);
        let avail = lay.caption.w - 6.0 * k;
        let w = super::outside::world_text_width(text, size);
        if w > avail {
            size = (size * avail / w).max(2.5);
        }
        let p = vec2(
            lay.caption.x + 3.0 * k,
            lay.caption.y + lay.caption.h * 0.46,
        );
        let p = rot_about(p, pivot, a);
        world_text_rot(text, p, size, ink, false, a);
    }
    if let Some(date) = st.date {
        let size = (10.0 * k).clamp(2.5, 14.0);
        let w = super::outside::world_text_width(date, size);
        let p = vec2(
            lay.caption.x + lay.caption.w - w - 2.0 * k,
            lay.caption.y + lay.caption.h * 0.86,
        );
        let p = rot_about(p, pivot, a);
        world_text_rot(date, p, size, Color::from_hex(0x8a8098), false, a);
    }

    // washi tape across the top edge
    if st.tape {
        let tw = f.w * 0.3;
        let th = 22.0 * k;
        let ta = a - 0.05 + (hash01(seed) - 0.5) * 0.1;
        let tc = vec2(
            f.x + f.w / 2.0 + (hash01(seed.wrapping_add(5)) - 0.5) * f.w * 0.15,
            f.y + 1.0 * k,
        );
        let tc = rot_about(tc, pivot, a);
        let r = Rect::new(tc.x - tw / 2.0, tc.y - th / 2.0, tw, th);
        let col = Color::new(0.78, 0.74, 0.95, 0.62);
        quad(
            rot_rect(r, tc, ta),
            [col, col, alpha(col, 0.55), alpha(col, 0.55)],
        );
        // faint stripes on the tape
        for i in 1..6 {
            let x = r.x + r.w * i as f32 / 6.0;
            let p0 = rot_about(vec2(x, r.y), tc, ta);
            let p1 = rot_about(vec2(x - 6.0 * k, r.y + r.h), tc, ta);
            draw_line(
                p0.x,
                p0.y,
                p1.x,
                p1.y,
                2.0 * k,
                Color::new(1.0, 1.0, 1.0, 0.18),
            );
        }
        // ragged ends
        for side in [0.0_f32, 1.0] {
            for j in 0..4 {
                let y = r.y + r.h * (j as f32 + 0.5) / 4.0;
                let x = r.x + side * r.w;
                let p = rot_about(vec2(x, y), tc, ta);
                draw_circle(p.x, p.y, 2.2 * k, Color::new(0.78, 0.74, 0.95, 0.4));
            }
        }
    }
    lay
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap() -> SceneSnap {
        let mut s = SceneSnap {
            origin: (2, 0),
            size: (4, 3),
            ..Default::default()
        };
        s.tiles = vec![
            SnapTile::Wall,
            SnapTile::Window,
            SnapTile::Window,
            SnapTile::Wall,
        ];
        s.tiles.extend([SnapTile::Floor(SnapFloor::Wood); 8]);
        s
    }

    #[test]
    fn tiles_are_looked_up_in_map_coordinates() {
        let s = snap();
        assert_eq!(s.tile(3, 0), SnapTile::Window);
        assert_eq!(s.tile(2, 2), SnapTile::Floor(SnapFloor::Wood));
        assert_eq!(s.tile(1, 0), SnapTile::Void);
        assert_eq!(s.tile(6, 0), SnapTile::Void);
        let v = s.view_rect();
        assert!((v.w / v.h - 4.0 / 3.0).abs() < 1e-4);
        assert!((v.center().x - 4.0 * TILE).abs() < 1e-3);
    }

    #[test]
    fn tilt_is_small_and_deterministic() {
        for seed in 0..200 {
            let t = polaroid_tilt(seed);
            assert!(t.abs() <= 3.0_f32.to_radians() + 1e-6);
            assert_eq!(t, polaroid_tilt(seed));
        }
        assert_ne!(polaroid_tilt(1), polaroid_tilt(2));
    }

    #[test]
    fn layout_has_real_polaroid_proportions() {
        let l = polaroid_layout(vec2(10.0, 20.0), 388.0, 0.0);
        assert_eq!(l.photo.w, 360.0);
        assert_eq!(l.photo.h, 270.0);
        assert!(l.caption.y >= l.photo.bottom());
        assert!(l.frame.bottom() >= l.caption.bottom() - 0.01);
        assert_eq!(SnapFloor::from_id(SnapFloor::Tile.id()), SnapFloor::Tile);
    }
}
