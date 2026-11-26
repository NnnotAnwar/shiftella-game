//! The "Shiftella" wordmark: Inter Bold with an indigo → violet gradient and a soft glow.

use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation, PipelineParams};
use macroquad::prelude::*;

use crate::ui::draw::{glow_circle, mix, with_a};
use crate::ui::{TextStyle, Ui, Weight};

const WORD: &str = "Shiftella";

const VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying lowp vec2 uv;
varying lowp vec4 color;
varying highp vec2 pos;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    color = color0 / 255.0;
    uv = texcoord;
    pos = position.xy;
}
"#;

const FRAGMENT: &str = r#"#version 100
precision highp float;
varying lowp vec2 uv;
varying lowp vec4 color;
varying highp vec2 pos;
uniform sampler2D Texture;
uniform vec4 area;
uniform vec4 lt;
uniform vec4 lb;
uniform vec4 rt;
uniform vec4 rb;
void main() {
    float fx = clamp((pos.x - area.x) / (area.z - area.x), 0.0, 1.0);
    float fy = clamp((pos.y - area.y) / (area.w - area.y), 0.0, 1.0);
    fy = fy * fy * (3.0 - 2.0 * fy);
    vec3 c = mix(mix(lt.rgb, rt.rgb, fx), mix(lb.rgb, rb.rgb, fx), fy);
    gl_FragColor = vec4(c, color.a * texture2D(Texture, uv).a);
}
"#;

thread_local! {
    static MATERIAL: std::cell::OnceCell<Option<Material>> = const { std::cell::OnceCell::new() };
}

/// The gradient material for the wordmark text (created on first use; `None` if the
/// shader can't compile, then a flat colour is used).
fn wordmark_material() -> Option<Material> {
    MATERIAL.with(|m| {
        m.get_or_init(|| {
            let blend = BlendState::new(
                Equation::Add,
                BlendFactor::Value(BlendValue::SourceAlpha),
                BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
            );
            load_material(
                ShaderSource::Glsl {
                    vertex: VERTEX,
                    fragment: FRAGMENT,
                },
                MaterialParams {
                    pipeline_params: PipelineParams {
                        color_blend: Some(blend),
                        alpha_blend: Some(BlendState::new(
                            Equation::Add,
                            BlendFactor::Zero,
                            BlendFactor::One,
                        )),
                        ..Default::default()
                    },
                    uniforms: vec![
                        UniformDesc::new("area", UniformType::Float4),
                        UniformDesc::new("lt", UniformType::Float4),
                        UniformDesc::new("lb", UniformType::Float4),
                        UniformDesc::new("rt", UniformType::Float4),
                        UniformDesc::new("rb", UniformType::Float4),
                    ],
                    ..Default::default()
                },
            )
            .map_err(|e| eprintln!("warning: wordmark shader: {e:?}"))
            .ok()
        })
        .clone()
    })
}

fn style(size: f32) -> TextStyle {
    TextStyle::new(size, Weight::Bold, WHITE).tracking(-size * 0.02)
}

pub fn wordmark_width(ui: &Ui, size: f32) -> f32 {
    ui.fonts.measure(WORD, &style(size))
}

/// Draws the wordmark with its baseline at `y`; returns the width.
/// The gradient runs left → right (indigo → violet) and top → bottom (light → deep),
/// computed per pixel by [`wordmark_material`].
pub fn draw_wordmark(ui: &mut Ui, x: f32, y: f32, size: f32, t: f32) -> f32 {
    let st = style(size);
    let w = ui.fonts.measure(WORD, &st);
    let cap = ui.fonts.cap_height(&st);
    let th = ui.theme.clone();
    // glow: a breathing violet haze behind the letters
    let breathe = 0.85 + 0.15 * (t * 0.8).sin();
    glow_circle(
        vec2(x + w * 0.5, y - cap * 0.5),
        w * 0.62,
        with_a(th.violet, 0.16 * breathe),
    );
    glow_circle(
        vec2(x + w * 0.3, y - cap * 0.55),
        w * 0.35,
        with_a(th.indigo, 0.18 * breathe),
    );
    // soft shadow + glow copies
    let mut sh = st;
    sh.color = Color::new(0.02, 0.01, 0.08, 0.55);
    ui.fonts.draw(WORD, x, y + size * 0.05, &sh);
    for (dx, dy) in [(-1.5, 0.0), (1.5, 0.0), (0.0, -1.5), (0.0, 1.5)] {
        let mut g = st;
        g.color = with_a(th.indigo_400, 0.10 * breathe);
        ui.fonts.draw(WORD, x + dx, y + dy, &g);
    }
    // the gradient: one smooth per-pixel blend in a tiny shader (no bands)
    let top = y - cap - size * 0.04;
    let bottom = y + size * 0.06;
    let (lt, lb) = (Color::from_hex(0xC7D2FE), th.indigo);
    let (rt, rb) = (Color::from_hex(0xDDD6FE), th.violet);
    match wordmark_material() {
        Some(mat) => {
            gl_use_material(&mat);
            mat.set_uniform("area", (x, top, x + w, bottom));
            mat.set_uniform("lt", (lt.r, lt.g, lt.b, 1.0f32));
            mat.set_uniform("lb", (lb.r, lb.g, lb.b, 1.0f32));
            mat.set_uniform("rt", (rt.r, rt.g, rt.b, 1.0f32));
            mat.set_uniform("rb", (rb.r, rb.g, rb.b, 1.0f32));
            ui.fonts.draw(WORD, x, y, &st);
            gl_use_default_material();
        }
        None => {
            let mut s = st;
            s.color = mix(mix(lt, rt, 0.5), mix(lb, rb, 0.5), 0.45);
            ui.fonts.draw(WORD, x, y, &s);
        }
    }
    // a thin highlight sweeping across now and then
    let sweep = ((t * 0.12) % 1.0) * 3.0 - 1.0;
    if (0.0..=1.0).contains(&sweep) {
        let sx = x + w * sweep;
        ui.push_clip(Rect::new(sx - 10.0, top, 20.0, bottom - top));
        let mut s = st;
        s.color = with_a(WHITE, 0.35 * (1.0 - (sweep * 2.0 - 1.0).abs()));
        ui.fonts.draw(WORD, x, y, &s);
        ui.pop_clip();
    }
    w
}
