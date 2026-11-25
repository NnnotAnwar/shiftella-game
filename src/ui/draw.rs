//! Low-level vector painting for the UI kit.
//!
//! macroquad only gives us flat triangles, so every shape here is built as a small
//! mesh with an **anti-aliasing fringe**: one physical pixel wide ring whose outer
//! vertices are transparent. That makes edges smooth even without MSAA and keeps
//! 1 pt hairlines crisp on Retina. Gradients are per-vertex colours ([`Paint`]).
//!
//! All coordinates are UI units (logical points ÷ `ui_scale`), see [`px`].

use std::cell::{Cell, RefCell};

use macroquad::miniquad;
use macroquad::models::{Mesh, Vertex, draw_mesh};
use macroquad::prelude::*;

thread_local! {
    static PX: Cell<f32> = const { Cell::new(0.5) };
    static ALPHA: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
    static NOISE: RefCell<Option<Texture2D>> = const { RefCell::new(None) };
}

/// Size of one physical pixel in UI units (`1 / (dpi × ui_scale)`).
pub fn px() -> f32 {
    PX.with(|p| p.get())
}

/// Called by `Ui::begin_frame`. Only needed if you paint without a `Ui`.
pub fn set_px(v: f32) {
    PX.with(|p| p.set(v.max(0.05)));
}

/// Snap a coordinate to the physical pixel grid (crisp text and hairlines).
pub fn snap(v: f32) -> f32 {
    let p = px();
    (v / p).round() * p
}

/// Multiply the alpha of everything painted until [`pop_alpha`] (nests).
pub fn push_alpha(a: f32) {
    ALPHA.with(|s| {
        let mut s = s.borrow_mut();
        let cur = s.last().copied().unwrap_or(1.0);
        s.push(cur * a.clamp(0.0, 1.0));
    });
}

pub fn pop_alpha() {
    ALPHA.with(|s| {
        s.borrow_mut().pop();
    });
}

/// Current global alpha multiplier.
pub fn alpha() -> f32 {
    ALPHA.with(|s| s.borrow().last().copied().unwrap_or(1.0))
}

/// `color` with its alpha multiplied by the global alpha.
pub fn faded(c: Color) -> Color {
    Color {
        a: c.a * alpha(),
        ..c
    }
}

/// Colour with a new alpha.
pub fn with_a(c: Color, a: f32) -> Color {
    Color { a, ..c }
}

/// Linear interpolation between two colours.
pub fn mix(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

/// How a shape is filled: a solid colour or a linear gradient between two points.
#[derive(Clone, Copy, Debug)]
pub enum Paint {
    Solid(Color),
    Linear {
        from: Vec2,
        to: Vec2,
        c0: Color,
        c1: Color,
    },
}

impl From<Color> for Paint {
    fn from(c: Color) -> Self {
        Paint::Solid(c)
    }
}

impl Paint {
    /// Top → bottom gradient over `r`.
    pub fn vertical(r: Rect, top: Color, bottom: Color) -> Self {
        Paint::Linear {
            from: vec2(r.x, r.y),
            to: vec2(r.x, r.y + r.h),
            c0: top,
            c1: bottom,
        }
    }
    /// Left → right gradient over `r`.
    pub fn horizontal(r: Rect, left: Color, right: Color) -> Self {
        Paint::Linear {
            from: vec2(r.x, r.y),
            to: vec2(r.x + r.w, r.y),
            c0: left,
            c1: right,
        }
    }
    /// Top-left → bottom-right gradient over `r`.
    pub fn diagonal(r: Rect, c0: Color, c1: Color) -> Self {
        Paint::Linear {
            from: vec2(r.x, r.y),
            to: vec2(r.x + r.w, r.y + r.h),
            c0,
            c1,
        }
    }
    pub fn at(&self, p: Vec2) -> Color {
        match *self {
            Paint::Solid(c) => c,
            Paint::Linear { from, to, c0, c1 } => {
                let d = to - from;
                let len2 = d.length_squared().max(1e-6);
                mix(c0, c1, (p - from).dot(d) / len2)
            }
        }
    }
    /// Same paint with every stop's alpha multiplied by `a`.
    pub fn alpha(self, a: f32) -> Self {
        match self {
            Paint::Solid(c) => Paint::Solid(with_a(c, c.a * a)),
            Paint::Linear { from, to, c0, c1 } => Paint::Linear {
                from,
                to,
                c0: with_a(c0, c0.a * a),
                c1: with_a(c1, c1.a * a),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Geometry helpers (pure)
// ---------------------------------------------------------------------------

/// Segments for a corner/arc of radius `r` (UI units) at current pixel density.
fn arc_segments(r: f32, sweep_deg: f32) -> usize {
    let r_px = (r / px()).max(0.0);
    let full = (r_px.sqrt() * 6.5).clamp(10.0, 72.0);
    ((full * sweep_deg / 360.0).ceil() as usize).max(2)
}

/// Closed clockwise (on screen) outline of a rounded rectangle.
pub fn rounded_rect_path(r: Rect, radius: f32) -> Vec<Vec2> {
    let rad = radius.min(r.w / 2.0).min(r.h / 2.0).max(0.0);
    if rad <= 0.01 {
        return vec![
            vec2(r.x, r.y),
            vec2(r.x + r.w, r.y),
            vec2(r.x + r.w, r.y + r.h),
            vec2(r.x, r.y + r.h),
        ];
    }
    let n = arc_segments(rad, 90.0).min(14);
    let corners = [
        (vec2(r.x + rad, r.y + rad), 180.0_f32),
        (vec2(r.x + r.w - rad, r.y + rad), 270.0),
        (vec2(r.x + r.w - rad, r.y + r.h - rad), 0.0),
        (vec2(r.x + rad, r.y + r.h - rad), 90.0),
    ];
    let mut pts = Vec::with_capacity(4 * (n + 1));
    for (c, start) in corners {
        for i in 0..=n {
            let a = (start + 90.0 * i as f32 / n as f32).to_radians();
            pts.push(c + vec2(a.cos(), a.sin()) * rad);
        }
    }
    pts
}

/// Closed circle outline.
pub fn circle_path(c: Vec2, r: f32) -> Vec<Vec2> {
    let n = arc_segments(r, 360.0);
    (0..n)
        .map(|i| {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            c + vec2(a.cos(), a.sin()) * r
        })
        .collect()
}

/// Open arc from `a0` to `a1` (radians, y-down: 0 = right, π/2 = down).
pub fn arc_path(c: Vec2, r: f32, a0: f32, a1: f32) -> Vec<Vec2> {
    let sweep = (a1 - a0).abs().to_degrees();
    let n = arc_segments(r, sweep).max(3);
    (0..=n)
        .map(|i| {
            let a = a0 + (a1 - a0) * i as f32 / n as f32;
            c + vec2(a.cos(), a.sin()) * r
        })
        .collect()
}

fn signed_area(pts: &[Vec2]) -> f32 {
    let mut a = 0.0;
    for i in 0..pts.len() {
        let p = pts[i];
        let q = pts[(i + 1) % pts.len()];
        a += p.x * q.y - q.x * p.y;
    }
    a * 0.5
}

/// Outward miter vectors for each vertex (length ≥ 1, clamped).
fn vertex_normals(pts: &[Vec2], closed: bool) -> Vec<Vec2> {
    let n = pts.len();
    let flip = if closed && signed_area(pts) < 0.0 {
        -1.0
    } else {
        1.0
    };
    let edge_n = |a: Vec2, b: Vec2| {
        let d = (b - a).normalize_or_zero();
        vec2(d.y, -d.x) * flip
    };
    (0..n)
        .map(|i| {
            let has_prev = closed || i > 0;
            let has_next = closed || i + 1 < n;
            let prev = pts[(i + n - 1) % n];
            let next = pts[(i + 1) % n];
            let p = pts[i];
            let n0 = if has_prev {
                edge_n(prev, p)
            } else {
                Vec2::ZERO
            };
            let n1 = if has_next {
                edge_n(p, next)
            } else {
                Vec2::ZERO
            };
            let (n0, n1) = match (has_prev, has_next) {
                (true, true) => (n0, n1),
                (true, false) => (n0, n0),
                _ => (n1, n1),
            };
            let m = (n0 + n1).normalize_or_zero();
            if m == Vec2::ZERO {
                return n0;
            }
            let cos = m.dot(n0).max(0.34);
            m / cos
        })
        .collect()
}

/// Clip a convex polygon by the half-plane `dot(p - origin, normal) <= 0`.
pub fn clip_convex(pts: &[Vec2], origin: Vec2, normal: Vec2) -> Vec<Vec2> {
    let mut out = Vec::with_capacity(pts.len() + 2);
    let d = |p: Vec2| (p - origin).dot(normal);
    for i in 0..pts.len() {
        let a = pts[i];
        let b = pts[(i + 1) % pts.len()];
        let (da, db) = (d(a), d(b));
        if da <= 0.0 {
            out.push(a);
        }
        if (da <= 0.0) != (db <= 0.0) {
            let t = da / (da - db);
            out.push(a + (b - a) * t);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Mesh building
// ---------------------------------------------------------------------------

struct Buf {
    v: Vec<Vertex>,
    i: Vec<u16>,
    a: f32,
    uv_scale: f32,
}

impl Buf {
    fn new(cap: usize, uv_scale: f32) -> Self {
        Buf {
            v: Vec::with_capacity(cap),
            i: Vec::with_capacity(cap * 3),
            a: alpha(),
            uv_scale,
        }
    }
    fn push(&mut self, p: Vec2, c: Color) -> u16 {
        let c = Color {
            a: c.a * self.a,
            ..c
        };
        let uv = p * self.uv_scale;
        self.v.push(Vertex::new(p.x, p.y, 0.0, uv.x, uv.y, c));
        (self.v.len() - 1) as u16
    }
    fn tri(&mut self, a: u16, b: u16, c: u16) {
        self.i.extend_from_slice(&[a, b, c]);
    }
    fn quad(&mut self, a: u16, b: u16, c: u16, d: u16) {
        // a-b along one row, c-d along the next row (same direction)
        self.i.extend_from_slice(&[a, b, c, b, d, c]);
    }
    fn flush(self, texture: Option<Texture2D>) {
        if self.v.is_empty() {
            return;
        }
        draw_mesh(&Mesh {
            vertices: self.v,
            indices: self.i,
            texture,
        });
    }
}

/// Fill a convex polygon with an AA fringe.
pub fn fill_convex(pts: &[Vec2], paint: Paint) {
    fill_convex_tex(pts, paint, None, 0.0);
}

fn fill_convex_tex(pts: &[Vec2], paint: Paint, tex: Option<Texture2D>, uv_scale: f32) {
    let n = pts.len();
    if n < 3 {
        return;
    }
    let f = px();
    let normals = vertex_normals(pts, true);
    let center = pts.iter().copied().sum::<Vec2>() / n as f32;
    let mut b = Buf::new(2 * n + 1, uv_scale);
    let c = b.push(center, paint.at(center));
    let inner: Vec<u16> = (0..n)
        .map(|i| {
            let p = pts[i] - normals[i] * f * 0.5;
            b.push(p, paint.at(pts[i]))
        })
        .collect();
    let outer: Vec<u16> = (0..n)
        .map(|i| {
            let p = pts[i] + normals[i] * f * 0.5;
            let col = paint.at(pts[i]);
            b.push(p, with_a(col, 0.0))
        })
        .collect();
    for i in 0..n {
        let j = (i + 1) % n;
        b.tri(c, inner[i], inner[j]);
        b.quad(inner[i], inner[j], outer[i], outer[j]);
    }
    b.flush(tex);
}

/// Stroke a path. `closed` joins the last point to the first; open paths get round caps.
pub fn stroke_path(pts: &[Vec2], closed: bool, width: f32, paint: Paint) {
    let n = pts.len();
    if n < 2 {
        return;
    }
    let f = px();
    let hw = width * 0.5;
    let core = (hw - f * 0.5).max(0.0);
    let fringe = hw + f * 0.5;
    let k = (width / f).min(1.0);
    let normals = vertex_normals(pts, closed);
    let mut b = Buf::new(n * 4, 0.0);
    let mut rows: [Vec<u16>; 4] = Default::default();
    for i in 0..n {
        let p = pts[i];
        let nn = normals[i];
        let col = paint.at(p);
        let col = with_a(col, col.a * k);
        rows[0].push(b.push(p + nn * fringe, with_a(col, 0.0)));
        rows[1].push(b.push(p + nn * core, col));
        rows[2].push(b.push(p - nn * core, col));
        rows[3].push(b.push(p - nn * fringe, with_a(col, 0.0)));
    }
    let segs = if closed { n } else { n - 1 };
    for s in 0..segs {
        let (i, j) = (s, (s + 1) % n);
        for r in 0..3 {
            b.quad(rows[r][i], rows[r][j], rows[r + 1][i], rows[r + 1][j]);
        }
    }
    b.flush(None);
    if !closed {
        cap(pts[0], pts[0] - pts[1], hw, paint.at(pts[0]));
        cap(
            pts[n - 1],
            pts[n - 1] - pts[n - 2],
            hw,
            paint.at(pts[n - 1]),
        );
    }
}

/// Half-disc round cap at `p`, bulging towards `dir`.
fn cap(p: Vec2, dir: Vec2, r: f32, color: Color) {
    let d = dir.normalize_or_zero();
    if d == Vec2::ZERO {
        return;
    }
    let base = d.y.atan2(d.x);
    let a0 = base - std::f32::consts::FRAC_PI_2;
    let mut pts = arc_path(p, r, a0, a0 + std::f32::consts::PI);
    pts.push(p);
    fill_convex(&pts, Paint::Solid(color));
}

/// Polyline stroke with round caps **and** round joins (icon style).
pub fn stroke_round(pts: &[Vec2], closed: bool, width: f32, color: Color) {
    stroke_path(pts, closed, width, Paint::Solid(color));
    let n = pts.len();
    if n < 3 {
        return;
    }
    let range = if closed { 0..n } else { 1..n - 1 };
    for i in range {
        let a = pts[(i + n - 1) % n];
        let p = pts[i];
        let c = pts[(i + 1) % n];
        let d0 = (p - a).normalize_or_zero();
        let d1 = (c - p).normalize_or_zero();
        if d0.dot(d1) < 0.9 {
            fill_convex(&circle_path(p, width * 0.5), Paint::Solid(color));
        }
    }
}

// ---------------------------------------------------------------------------
// Public shapes
// ---------------------------------------------------------------------------

pub fn fill_rrect(r: Rect, radius: f32, paint: impl Into<Paint>) {
    fill_convex(&rounded_rect_path(r, radius), paint.into());
}

/// Stroke drawn **inside** the rect edge.
pub fn stroke_rrect(r: Rect, radius: f32, width: f32, paint: impl Into<Paint>) {
    let h = width * 0.5;
    let inner = Rect::new(r.x + h, r.y + h, r.w - width, r.h - width);
    stroke_path(
        &rounded_rect_path(inner, (radius - h).max(0.0)),
        true,
        width,
        paint.into(),
    );
}

pub fn fill_circle(c: Vec2, r: f32, paint: impl Into<Paint>) {
    fill_convex(&circle_path(c, r), paint.into());
}

pub fn stroke_circle(c: Vec2, r: f32, width: f32, paint: impl Into<Paint>) {
    stroke_path(&circle_path(c, r), true, width, paint.into());
}

/// Round-capped line.
pub fn line(a: Vec2, b: Vec2, width: f32, color: Color) {
    stroke_path(&[a, b], false, width, Paint::Solid(color));
}

/// Round-capped arc (radians, y-down).
pub fn arc(c: Vec2, r: f32, a0: f32, a1: f32, width: f32, color: Color) {
    stroke_path(&arc_path(c, r, a0, a1), false, width, Paint::Solid(color));
}

/// Hairline that is exactly one physical pixel (or `w` units) thick, no caps.
pub fn hline(x0: f32, x1: f32, y: f32, color: Color) {
    let t = px().max(0.5);
    let y = snap(y);
    fill_convex(
        &[vec2(x0, y), vec2(x1, y), vec2(x1, y + t), vec2(x0, y + t)],
        Paint::Solid(color),
    );
}

pub fn vline(x: f32, y0: f32, y1: f32, color: Color) {
    let t = px().max(0.5);
    let x = snap(x);
    fill_convex(
        &[vec2(x, y0), vec2(x + t, y0), vec2(x + t, y1), vec2(x, y1)],
        Paint::Solid(color),
    );
}

/// A soft drop shadow / glow description.
#[derive(Clone, Copy, Debug)]
pub struct Shadow {
    pub offset: Vec2,
    pub blur: f32,
    pub spread: f32,
    pub color: Color,
}

impl Shadow {
    pub const fn new(dx: f32, dy: f32, blur: f32, spread: f32, color: Color) -> Self {
        Shadow {
            offset: Vec2::new(dx, dy),
            blur,
            spread,
            color,
        }
    }
}

/// Gaussian-looking soft shadow under a rounded rect (or a coloured glow).
pub fn shadow_rrect(r: Rect, radius: f32, s: Shadow) {
    if s.color.a <= 0.001 {
        return;
    }
    let r = Rect::new(
        r.x + s.offset.x - s.spread,
        r.y + s.offset.y - s.spread,
        r.w + s.spread * 2.0,
        r.h + s.spread * 2.0,
    );
    let blur = s.blur.max(px());
    let inset = (blur * 0.5).min(r.w.min(r.h) * 0.5 - 0.5).max(0.0);
    let base_r = (radius + s.spread).max(inset + 1.0);
    let path = rounded_rect_path(r, base_r);
    let normals = vertex_normals(&path, true);
    let n = path.len();
    const RINGS: [(f32, f32); 6] = [
        (0.0, 1.0),
        (0.18, 0.86),
        (0.38, 0.55),
        (0.6, 0.24),
        (0.8, 0.07),
        (1.0, 0.0),
    ];
    let mut b = Buf::new(n * RINGS.len() + 1, 0.0);
    let center = vec2(r.x + r.w / 2.0, r.y + r.h / 2.0);
    let c = b.push(center, s.color);
    let mut rows: Vec<Vec<u16>> = Vec::with_capacity(RINGS.len());
    for (t, a) in RINGS {
        let d = -inset + t * (inset + blur * 0.5);
        rows.push(
            (0..n)
                .map(|i| b.push(path[i] + normals[i] * d, with_a(s.color, s.color.a * a)))
                .collect(),
        );
    }
    for i in 0..n {
        let j = (i + 1) % n;
        b.tri(c, rows[0][i], rows[0][j]);
        for k in 0..RINGS.len() - 1 {
            b.quad(rows[k][i], rows[k][j], rows[k + 1][i], rows[k + 1][j]);
        }
    }
    b.flush(None);
}

/// Radial glow (soft circle of light) — e.g. behind a selected icon.
pub fn glow_circle(c: Vec2, r: f32, color: Color) {
    let path = circle_path(c, r);
    let n = path.len();
    let mut b = Buf::new(n * 4 + 1, 0.0);
    let ci = b.push(c, color);
    let stops = [(0.35, 0.7), (0.7, 0.25), (1.0, 0.0)];
    let rows: Vec<Vec<u16>> = stops
        .iter()
        .map(|&(t, a)| {
            (0..n)
                .map(|i| b.push(c + (path[i] - c) * t, with_a(color, color.a * a)))
                .collect()
        })
        .collect();
    for i in 0..n {
        let j = (i + 1) % n;
        b.tri(ci, rows[0][i], rows[0][j]);
        for k in 0..rows.len() - 1 {
            b.quad(rows[k][i], rows[k][j], rows[k + 1][i], rows[k + 1][j]);
        }
    }
    b.flush(None);
}

// ---------------------------------------------------------------------------
// Grain texture for frosted glass
// ---------------------------------------------------------------------------

const NOISE_SIZE: u16 = 128;

fn noise_texture() -> Texture2D {
    NOISE.with(|n| {
        n.borrow_mut()
            .get_or_insert_with(|| {
                let mut bytes = Vec::with_capacity(NOISE_SIZE as usize * NOISE_SIZE as usize * 4);
                let mut seed: u32 = 0x9E37_79B9;
                for _ in 0..(NOISE_SIZE as usize * NOISE_SIZE as usize) {
                    // xorshift32 — deterministic grain
                    seed ^= seed << 13;
                    seed ^= seed >> 17;
                    seed ^= seed << 5;
                    let v = 255 - (seed % 22) as u8;
                    bytes.extend_from_slice(&[v, v, v, 255]);
                }
                let tex = Texture2D::from_rgba8(NOISE_SIZE, NOISE_SIZE, &bytes);
                tex.set_filter(FilterMode::Nearest);
                let id = tex.raw_miniquad_id();
                // SAFETY: plain state change on a texture we own, on the main thread.
                unsafe {
                    get_internal_gl().quad_context.texture_set_wrap(
                        id,
                        miniquad::TextureWrap::Repeat,
                        miniquad::TextureWrap::Repeat,
                    );
                }
                tex
            })
            .clone()
    })
}

/// Rounded rect fill with a fine grain (1 texel = 1 physical pixel).
pub fn fill_rrect_grain(r: Rect, radius: f32, paint: Paint) {
    let tex = noise_texture();
    let uv = 1.0 / (px() * NOISE_SIZE as f32);
    fill_convex_tex(&rounded_rect_path(r, radius), paint, Some(tex), uv);
}

// ---------------------------------------------------------------------------
// Clipping (scissor)
// ---------------------------------------------------------------------------

/// Set the GPU scissor to `r` (UI units) or clear it. Prefer `Ui::push_clip`.
pub fn set_scissor(r: Option<Rect>) {
    let k = 1.0 / px();
    let clip = r.map(|r| {
        let x0 = (r.x * k).floor() as i32;
        let y0 = (r.y * k).floor() as i32;
        let x1 = ((r.x + r.w) * k).ceil() as i32;
        let y1 = ((r.y + r.h) * k).ceil() as i32;
        (x0, y0, (x1 - x0).max(0), (y1 - y0).max(0))
    });
    // SAFETY: only changes batched draw state of macroquad's own QuadGl.
    unsafe {
        get_internal_gl().quad_gl.scissor(clip);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_square_in_half() {
        let sq = [
            vec2(0.0, 0.0),
            vec2(10.0, 0.0),
            vec2(10.0, 10.0),
            vec2(0.0, 10.0),
        ];
        let half = clip_convex(&sq, vec2(5.0, 0.0), vec2(1.0, 0.0));
        assert_eq!(half.len(), 4);
        assert!(half.iter().all(|p| p.x <= 5.0 + 1e-4));
    }

    #[test]
    fn rounded_path_is_clockwise() {
        let p = rounded_rect_path(Rect::new(0.0, 0.0, 100.0, 40.0), 8.0);
        assert!(signed_area(&p) > 0.0);
        // outward normal of the top edge points up (y-down space)
        let n = vertex_normals(&p, true);
        let top = p
            .iter()
            .enumerate()
            .find(|(_, q)| (q.x - 50.0).abs() < 45.0 && q.y.abs() < 1e-3)
            .map(|(i, _)| i)
            .unwrap();
        assert!(n[top].y < -0.5);
    }

    #[test]
    fn paint_gradient_endpoints() {
        let r = Rect::new(0.0, 0.0, 10.0, 10.0);
        let p = Paint::vertical(r, WHITE, BLACK);
        assert!((p.at(vec2(3.0, 0.0)).r - 1.0).abs() < 1e-5);
        assert!(p.at(vec2(3.0, 10.0)).r.abs() < 1e-5);
        assert!((p.at(vec2(3.0, 5.0)).r - 0.5).abs() < 1e-5);
    }
}
