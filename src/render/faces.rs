//! Little round face avatars (crew lists, the Shiftella app, the phone header): each look's
//! portrait is rendered once into a small texture and drawn as a textured circle, so the
//! face is clipped cleanly and costs one quad per frame.

use std::cell::RefCell;
use std::collections::HashMap;

use macroquad::prelude::*;

use super::{Expression, PersonLook, draw_portrait};

/// Side of a cached face in px (sharp up to ~48 pt avatars on Retina).
const SIDE: u32 = 128;

thread_local! {
    static FACES: RefCell<HashMap<String, (PersonLook, RenderTarget)>> = RefCell::new(HashMap::new());
}

fn bake(look: &PersonLook) -> RenderTarget {
    let rt = render_target(SIDE, SIDE);
    crate::profile::count(crate::profile::Counter::RenderTargets, 1);
    crate::profile::count(crate::profile::Counter::FaceBakes, 1);
    rt.texture.set_filter(FilterMode::Linear);
    let s = SIDE as f32;
    let mut cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, s, s));
    cam.render_target = Some(rt.clone());
    set_camera(&cam);
    clear_background(Color::new(0.0, 0.0, 0.0, 0.0));
    // frame the head: the bust is drawn a little larger than the circle, face centred
    let k = 1.32;
    let r = Rect::new(-(k - 1.0) * s / 2.0, -0.02 * s, s * k, s * k);
    draw_portrait(look, Expression::Smile, r, 0.0);
    // render it now: a later `clear_background` in this frame would drop queued draw calls
    // SAFETY: flushes macroquad's own batcher between frames' draw calls.
    unsafe { get_internal_gl() }.flush();
    // keep the whole target alive: dropping it frees its render pass and texture
    rt
}

/// Bakes the face for `key` if it is missing or the look changed. Call at the very start of
/// a frame, before anything is drawn (baking clears a render pass, which drops the draw
/// calls queued so far, and must not run under a UI scissor). Returns true if it baked.
pub fn prepare(key: &str, look: &PersonLook) -> bool {
    FACES.with(|f| {
        let mut f = f.borrow_mut();
        if f.get(key).is_some_and(|(l, _)| l == look) {
            return false;
        }
        let t = bake(look);
        f.insert(key.to_string(), (*look, t));
        true
    })
}

/// The baked face for `key`, if [`prepare`] has made it.
pub fn face_texture(key: &str) -> Option<Texture2D> {
    FACES.with(|f| f.borrow().get(key).map(|(_, t)| t.texture.clone()))
}

/// Draws `tex` as a circle of diameter `d` centred at `c` (render targets are upside down).
pub fn draw_face_circle(tex: &Texture2D, c: Vec2, d: f32, tint: Color) {
    let r = d / 2.0;
    let n = 40;
    let mut vertices = Vec::with_capacity(n + 1);
    vertices.push(Vertex::new(c.x, c.y, 0.0, 0.5, 0.5, tint));
    for i in 0..n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        let (dx, dy) = (a.cos(), a.sin());
        vertices.push(Vertex::new(
            c.x + dx * r,
            c.y + dy * r,
            0.0,
            0.5 + dx * 0.5,
            0.5 - dy * 0.5,
            tint,
        ));
    }
    let mut indices = Vec::with_capacity(n * 3);
    for i in 0..n {
        indices.extend_from_slice(&[0, (i + 1) as u16, ((i + 1) % n + 1) as u16]);
    }
    draw_mesh(&Mesh {
        vertices,
        indices,
        texture: Some(tex.clone()),
    });
}
