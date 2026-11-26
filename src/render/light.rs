//! Night lighting: a light map in a render target, laid over the world as a "2× multiply".
//!
//! How it works:
//! 1. `begin` remembers the world camera and the overall darkness colour (`ambient`).
//! 2. `add_light` / `add_quad` collect sources (no GPU calls).
//! 3. `apply` draws the light map (half resolution) = ambient/2 + Σ lights (additive),
//!    then lays it on the screen with the blend `src·dst + dst·src` = 2·world·light.
//!    So 0.5 in the map = the world as is, less = darker, more = overexposed
//!    (warm lamp pools really "glow" instead of just being "less dark").
//! 4. A soft additive glow (bloom-lite) around bright lamps on top.
//!
//! Works on macOS GL (GLSL 100, like macroquad's own shaders).

use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation, PipelineParams};
use macroquad::prelude::*;

use super::colors as c;
use super::draw::{alpha, glow, hash01, mix, noise1, radial};
use crate::map::TILE;

/// A point light (a pool on the floor / wall) in world pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Light {
    pub pos: Vec2,
    pub radius: f32,
    pub color: Color,
    /// 1.0 in the centre ≈ +100 % of the world's brightness over the darkness.
    pub intensity: f32,
    /// Flicker amplitude 0..1 (candle ~0.12, lamp ~0.03).
    pub flicker: f32,
}

impl Light {
    pub fn new(pos: Vec2, radius: f32, color: Color, intensity: f32) -> Light {
        Light {
            pos,
            radius,
            color,
            intensity,
            flicker: 0.0,
        }
    }
    pub fn flicker(self, amount: f32) -> Light {
        Light {
            flicker: amount,
            ..self
        }
    }
    /// The glow of a phone screen (for the blackout).
    pub fn phone(pos: Vec2) -> Light {
        Light::new(pos, 0.9 * TILE, c::INDIGO_SOFT, 0.45).flicker(0.05)
    }
    /// A candle on a table (for the blackout).
    pub fn candle(pos: Vec2) -> Light {
        Light::new(pos, 1.5 * TILE, c::AMBER_DEEP, 0.6).flicker(0.12)
    }
    /// A soft lavender "spotlight" on a cutscene actor.
    pub fn actor_spot(feet: Vec2) -> Light {
        Light::new(feet - vec2(0.0, 30.0), 1.8 * TILE, c::LAVENDER_RIM, 0.55)
    }
}

/// Darkness by night progress: 0.0 — 22:00 (violet dusk),
/// ~0.55 — 3:00 (the deepest indigo night), 1.0 — 6:00 (blue pre-dawn).
/// The value multiplies the world's brightness (1,1,1 = no darkening).
pub fn ambient_for(night_progress: f32) -> Color {
    let p = night_progress.clamp(0.0, 1.0);
    let dusk = Color::new(0.56, 0.44, 0.78, 1.0);
    let night = Color::new(0.30, 0.28, 0.54, 1.0);
    let deep = Color::new(0.25, 0.24, 0.48, 1.0);
    let predawn = Color::new(0.48, 0.56, 0.82, 1.0);
    if p < 0.3 {
        mix(dusk, night, p / 0.3)
    } else if p < 0.6 {
        mix(night, deep, (p - 0.3) / 0.3)
    } else if p < 0.85 {
        mix(deep, mix(deep, predawn, 0.5), (p - 0.6) / 0.25)
    } else {
        mix(mix(deep, predawn, 0.5), predawn, (p - 0.85) / 0.15)
    }
}

/// Blackout darkness: only candles and phones give light.
pub fn ambient_blackout() -> Color {
    Color::new(0.13, 0.12, 0.25, 1.0)
}

const VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying lowp vec2 uv;
varying lowp vec4 color;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    uv = texcoord;
    color = color0 / 255.0;
}"#;

/// Additive light: colour × alpha is added to the map.
const FRAG_ADD: &str = r#"#version 100
precision mediump float;
varying lowp vec2 uv;
varying lowp vec4 color;
uniform sampler2D Texture;
void main() {
    vec4 c = color * texture2D(Texture, uv);
    gl_FragColor = vec4(c.rgb * c.a, c.a);
}"#;

/// Laying the light map on: output its colour (+ dithering against banding); the blend does 2·src·dst.
const FRAG_MUL: &str = r#"#version 100
precision mediump float;
varying lowp vec2 uv;
varying lowp vec4 color;
uniform sampler2D Texture;
void main() {
    vec3 l = texture2D(Texture, uv).rgb;
    float n = fract(sin(dot(gl_FragCoord.xy, vec2(12.9898, 78.233))) * 43758.5453);
    l += (n - 0.5) / 255.0;
    gl_FragColor = vec4(l, 1.0);
}"#;

/// `Camera2D` isn't `Clone` — copy it by hand (no render target needed).
fn copy_cam(v: &Camera2D) -> Camera2D {
    Camera2D {
        target: v.target,
        zoom: v.zoom,
        offset: v.offset,
        rotation: v.rotation,
        render_target: None,
        viewport: v.viewport,
    }
}

/// Any light shape (a window, a train's beam).
#[derive(Debug, Clone, Copy)]
struct LightQuad {
    p: [Vec2; 4],
    c: [Color; 4],
}

/// The lighting system. Create it once (`Lighting::new()`), then every frame:
/// `begin` → `add_light`… → `apply`.
pub struct Lighting {
    rt: Option<RenderTarget>,
    rt_size: (u32, u32),
    add_mat: Material,
    mul_mat: Material,
    lights: Vec<Light>,
    quads: Vec<LightQuad>,
    windows: Vec<Rect>,
    ambient: Color,
    view: Option<Camera2D>,
    time: f32,
    /// Light map resolution relative to the screen (0.5 = half, soft and fast).
    pub resolution: f32,
    /// A soft additive glow of bright sources on top (0 = off).
    pub bloom: f32,
}

impl Default for Lighting {
    fn default() -> Self {
        Self::new()
    }
}

impl Lighting {
    pub fn new() -> Lighting {
        let add_mat = load_material(
            ShaderSource::Glsl {
                vertex: VERTEX,
                fragment: FRAG_ADD,
            },
            MaterialParams {
                pipeline_params: PipelineParams {
                    color_blend: Some(BlendState::new(
                        Equation::Add,
                        BlendFactor::One,
                        BlendFactor::One,
                    )),
                    alpha_blend: Some(BlendState::new(
                        Equation::Add,
                        BlendFactor::Zero,
                        BlendFactor::One,
                    )),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .expect("light add shader");
        let mul_mat = load_material(
            ShaderSource::Glsl {
                vertex: VERTEX,
                fragment: FRAG_MUL,
            },
            MaterialParams {
                pipeline_params: PipelineParams {
                    color_blend: Some(BlendState::new(
                        Equation::Add,
                        BlendFactor::Value(BlendValue::DestinationColor),
                        BlendFactor::Value(BlendValue::SourceColor),
                    )),
                    alpha_blend: Some(BlendState::new(
                        Equation::Add,
                        BlendFactor::Zero,
                        BlendFactor::One,
                    )),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .expect("light multiply shader");
        Lighting {
            rt: None,
            rt_size: (0, 0),
            add_mat,
            mul_mat,
            lights: Vec::with_capacity(128),
            quads: Vec::with_capacity(32),
            windows: Vec::new(),
            ambient: WHITE,
            view: None,
            time: 0.0,
            resolution: 0.5,
            bloom: 1.0,
        }
    }

    /// Start a frame. `view` — the camera the world is drawn to the screen with
    /// (`WorldCamera::to_camera2d()`), `ambient` — from `ambient_for` / `ambient_blackout`.
    pub fn begin(&mut self, view: &Camera2D, ambient: Color) {
        self.view = Some(copy_cam(view));
        self.ambient = ambient;
        self.lights.clear();
        self.quads.clear();
        self.time = get_time() as f32;
    }

    /// Set the time used for flicker (default: `get_time()`); useful for screenshots.
    pub fn set_time(&mut self, t: f32) {
        self.time = t;
    }

    pub fn add_light(&mut self, light: Light) {
        self.lights.push(light);
    }

    /// Any convex light quad with a colour per vertex (alpha = strength).
    pub fn add_quad(&mut self, p: [Vec2; 4], c: [Color; 4]) {
        self.quads.push(LightQuad { p, c });
    }

    /// Window panes (world rects of the glass, see `window_glass_rect`).
    /// Needed for the cool light from the windows and for `train_sweep`.
    pub fn set_windows(&mut self, glass: Vec<Rect>) {
        self.windows = glass;
    }

    pub fn windows(&self) -> &[Rect] {
        &self.windows
    }

    /// Cool moon-and-streetlamp light from the windows: trapezoids on the floor. `strength` 0..1.
    pub fn add_window_light(&mut self, strength: f32) {
        let col = Color::from_hex(0x8ea6ff);
        for i in 0..self.windows.len() {
            let g = self.windows[i];
            let floor = g.y + g.h + 0.9 * TILE; // bottom of the wall
            let a = alpha(col, 0.32 * strength);
            // the glass itself glows
            self.add_quad(
                [
                    vec2(g.x, g.y),
                    vec2(g.x + g.w, g.y),
                    vec2(g.x + g.w, g.y + g.h),
                    vec2(g.x, g.y + g.h),
                ],
                [alpha(col, 0.5 * strength); 4],
            );
            self.add_soft_beam(
                g.x + 2.0,
                g.x + g.w - 2.0,
                floor,
                2.4 * TILE,
                0.45 * TILE,
                a,
            );
        }
    }

    /// Light of a passing train: a bright band runs across the windows and the room.
    /// `progress` 0..1 — from appearing to leaving (outside the range — nothing).
    pub fn train_sweep(&mut self, progress: f32) {
        if !(0.0..=1.0).contains(&progress) || self.windows.is_empty() {
            return;
        }
        let (mut x0, mut x1) = (f32::MAX, f32::MIN);
        for w in &self.windows {
            x0 = x0.min(w.x);
            x1 = x1.max(w.x + w.w);
        }
        let span = x1 - x0 + 6.0 * TILE;
        // carriage windows: several bands in a row
        let fade = (progress * (1.0 - progress) * 4.0).clamp(0.0, 1.0).sqrt();
        let col = Color::from_hex(0xd8e0ff);
        let head = x0 - 3.0 * TILE + span * progress;
        for car in 0..5 {
            let bx = head - car as f32 * 1.1 * TILE;
            let bw = 0.55 * TILE;
            for wi in 0..self.windows.len() {
                let g = self.windows[wi];
                let l = bx.max(g.x);
                let r = (bx + bw).min(g.x + g.w);
                if r <= l {
                    continue;
                }
                let k = 0.55 * fade;
                let floor = g.y + g.h + 0.9 * TILE;
                // a band on the glass and a stretched pool on the floor (shifted — the light falls at an angle)
                self.add_quad(
                    [
                        vec2(l, g.y),
                        vec2(r, g.y),
                        vec2(r, g.y + g.h),
                        vec2(l, g.y + g.h),
                    ],
                    [alpha(col, k * 1.2); 4],
                );
                self.add_soft_beam(l, r, floor, 3.2 * TILE, 0.9 * TILE, alpha(col, k));
            }
        }
    }

    /// A beam of light onto the floor from the segment [x0, x1] on line `y`: widens and fades downwards,
    /// soft edges (fading bands at the sides), `skew` — drift to the right.
    fn add_soft_beam(&mut self, x0: f32, x1: f32, y: f32, len: f32, skew: f32, col: Color) {
        let z = alpha(col, 0.0);
        let feather = 0.35 * TILE;
        let spread = 0.35 * TILE;
        let (bl, br) = (x0 - spread + skew, x1 + spread + skew);
        let y1 = y + len;
        self.add_quad(
            [vec2(x0, y), vec2(x1, y), vec2(br, y1), vec2(bl, y1)],
            [col, col, z, z],
        );
        self.add_quad(
            [
                vec2(x0 - feather * 0.4, y),
                vec2(x0, y),
                vec2(bl, y1),
                vec2(bl - feather, y1),
            ],
            [z, col, z, z],
        );
        self.add_quad(
            [
                vec2(x1, y),
                vec2(x1 + feather * 0.4, y),
                vec2(br + feather, y1),
                vec2(br, y1),
            ],
            [col, z, z, z],
        );
    }

    fn ensure_rt(&mut self) -> RenderTarget {
        let w = ((screen_width() * self.resolution) as u32).max(1);
        let h = ((screen_height() * self.resolution) as u32).max(1);
        if self.rt.is_none() || self.rt_size != (w, h) {
            let rt = render_target(w, h);
            rt.texture.set_filter(FilterMode::Linear);
            self.rt = Some(rt);
            self.rt_size = (w, h);
        }
        self.rt.clone().expect("light rt")
    }

    fn flicker_k(&self, l: &Light, i: usize) -> f32 {
        if l.flicker <= 0.0 {
            return 1.0;
        }
        let seed = (l.pos.x as i32 as u32).wrapping_mul(7) ^ (l.pos.y as i32 as u32) ^ i as u32;
        let n = noise1(self.time * 9.0 + hash01(seed) * 50.0, seed) * 0.7
            + noise1(self.time * 23.0, seed.wrapping_add(5)) * 0.3;
        1.0 - l.flicker + l.flicker * 2.0 * n
    }

    /// Draw the light map and lay it over the world already drawn.
    /// Afterwards the world camera `view` is active again (emotes / selection can be drawn).
    pub fn apply(&mut self) {
        let Some(view) = self.view.as_ref().map(copy_cam) else {
            return;
        };
        let rt = self.ensure_rt();
        // 1) the light map: the same world camera, but into the render target
        //    (on this macOS GL the RT texture and the screen agree in Y, no flip)
        let light_cam = Camera2D {
            target: view.target,
            zoom: view.zoom,
            offset: view.offset,
            rotation: view.rotation,
            render_target: Some(rt.clone()),
            viewport: None,
        };
        set_camera(&light_cam);
        let a = self.ambient;
        clear_background(Color::new(a.r * 0.5, a.g * 0.5, a.b * 0.5, 1.0));
        gl_use_material(&self.add_mat);
        for (i, l) in self.lights.iter().enumerate() {
            let k = self.flicker_k(l, i) * l.intensity * 0.5;
            let col = Color::new(l.color.r, l.color.g, l.color.b, k.min(1.0));
            radial(
                l.pos.x,
                l.pos.y,
                l.radius,
                l.radius * 0.92,
                col,
                alpha(col, 0.0),
            );
        }
        for q in &self.quads {
            let mut cc = q.c;
            for col in &mut cc {
                col.a *= 0.5;
            }
            super::draw::quad(q.p, cc);
        }
        gl_use_default_material();

        // 2) onto the screen: 2 × world × light
        set_default_camera();
        gl_use_material(&self.mul_mat);
        draw_texture_ex(
            &rt.texture,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(screen_width(), screen_height())),
                ..Default::default()
            },
        );
        gl_use_default_material();

        // 3) a soft glow around bright warm sources
        set_camera(&view);
        if self.bloom > 0.0 {
            for (i, l) in self.lights.iter().enumerate() {
                if l.intensity < 0.5 {
                    continue;
                }
                let k = self.flicker_k(l, i);
                glow(
                    l.pos.x,
                    l.pos.y,
                    l.radius * 0.45,
                    alpha(l.color, 0.05 * l.intensity * k * self.bloom),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn night_is_darkest_in_the_middle() {
        let lum = |c: Color| c.r + c.g + c.b;
        let dusk = lum(ambient_for(0.0));
        let deep = lum(ambient_for(0.55));
        let dawn = lum(ambient_for(1.0));
        assert!(deep < dusk && deep < dawn);
        assert!(lum(ambient_blackout()) < deep);
        // pre-dawn bluer, dusk more violet
        let d = ambient_for(1.0);
        assert!(d.b > d.r);
        let v = ambient_for(0.0);
        assert!(v.r > v.g);
    }
}
