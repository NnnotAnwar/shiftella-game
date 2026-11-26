//! Plate: drag 2–4 garnish pieces from a little wooden tray onto the plate. Soft
//! targets glow where each piece looks best; pieces snap gently when close.

use macroquad::prelude::*;

use super::paint::{contact_shadow, hex, with_alpha};
use super::score::plate_perf;
use super::{Ctx, Hint, MgInput, MiniGameSound, PlateSet, RegularTouch, Rng};
use crate::render::draw as rd;
use crate::ui::draw::{self, Paint, fill_circle, fill_convex, fill_rrect, stroke_circle};
use crate::ui::{Ui, approach, ease};

const C: Vec2 = Vec2::new(172.0, 141.0);
const PLATE_R: f32 = 120.0;
const WELL_R: f32 = 86.0;
const TRAY: Rect = Rect {
    x: 346.0,
    y: 14.0,
    w: 130.0,
    h: 254.0,
};
const SNAP: f32 = 24.0;
const MAX_T: f32 = 20.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PieceKind {
    Slice,
    Cream,
    Berries,
    Mint,
    Honey,
    Croutons,
    Dill,
    /// Flat cream spiral on soup.
    Swirl,
}

impl PieceKind {
    fn radius(self) -> f32 {
        match self {
            PieceKind::Slice => 34.0,
            PieceKind::Cream => 20.0,
            PieceKind::Berries => 18.0,
            PieceKind::Mint => 16.0,
            PieceKind::Honey => 20.0,
            PieceKind::Croutons => 20.0,
            PieceKind::Dill => 18.0,
            PieceKind::Swirl => 18.0,
        }
    }
}

struct Piece {
    kind: PieceKind,
    home: Vec2,
    target: Vec2,
    pos: Vec2,
    goal: Vec2,
    on_plate: bool,
    lift: f32,
    bounce: f32,
    rot: f32,
}

pub(crate) struct Plate {
    set: PlateSet,
    pieces: Vec<Piece>,
    dragging: Option<usize>,
    grab: Vec2,
    idle_since: f32,
    all_placed_at: Option<f32>,
    t: f32,
}

impl Plate {
    pub fn new(rng: &mut Rng, set: PlateSet, touch: Option<RegularTouch>) -> Plate {
        let mut spec: Vec<(PieceKind, Vec2)> = match set {
            PlateSet::Medovnik => vec![
                (PieceKind::Slice, C + vec2(-8.0, 10.0)),
                (PieceKind::Cream, C + vec2(54.0, -24.0)),
                (PieceKind::Berries, C + vec2(-58.0, -30.0)),
                (PieceKind::Mint, C + vec2(48.0, 44.0)),
            ],
            PlateSet::Soup => vec![
                (PieceKind::Swirl, C + vec2(0.0, 2.0)),
                (PieceKind::Dill, C + vec2(-4.0, -38.0)),
                (PieceKind::Croutons, C + vec2(40.0, 26.0)),
            ],
        };
        if touch == Some(RegularTouch::Honey) && set == PlateSet::Medovnik {
            spec.push((PieceKind::Honey, C + vec2(-46.0, 52.0)));
        }
        // a touch of variety: targets drift a few points per seed
        let n = spec.len();
        let pieces = spec
            .into_iter()
            .enumerate()
            .map(|(i, (kind, target))| {
                let gap = TRAY.h / n as f32;
                let home = vec2(TRAY.center().x, TRAY.y + gap * (i as f32 + 0.5));
                Piece {
                    kind,
                    home,
                    target: target + vec2(rng.range(-5.0, 5.0), rng.range(-5.0, 5.0)),
                    pos: home,
                    goal: home,
                    on_plate: false,
                    lift: 0.0,
                    bounce: 0.0,
                    rot: rng.range(-0.25, 0.25),
                }
            })
            .collect();
        Plate {
            set,
            pieces,
            dragging: None,
            grab: Vec2::ZERO,
            idle_since: 0.0,
            all_placed_at: None,
            t: 0.0,
        }
    }

    pub fn focus(&self) -> Vec2 {
        C
    }

    fn piece_at(&self, p: Vec2) -> Option<usize> {
        // topmost first (drawn last)
        let mut best: Option<(usize, f32)> = None;
        for (i, pc) in self.pieces.iter().enumerate() {
            let d = pc.pos.distance(p);
            if d < pc.kind.radius() + 6.0 && best.is_none_or(|(_, bd)| d < bd) {
                best = Some((i, d));
            }
        }
        best.map(|b| b.0)
    }

    fn place(&mut self, i: usize, at: Vec2, ctx: &mut Ctx) {
        let pc = &mut self.pieces[i];
        let on = at.distance(C) < PLATE_R - pc.kind.radius() * 0.4;
        if on {
            let snapped = if at.distance(pc.target) < SNAP * super::score::widen(ctx.relaxed) {
                pc.target
            } else {
                at
            };
            pc.goal = snapped;
            pc.on_plate = true;
            pc.bounce = 1.0;
            ctx.sounds.push(MiniGameSound::Clink);
        } else {
            pc.goal = pc.home;
            pc.on_plate = false;
        }
    }

    pub fn step(&mut self, ctx: &mut Ctx, input: &MgInput, dt: f32) -> Option<f32> {
        self.t = ctx.t;
        if let Some(p) = input.pointer {
            if input.pressed && self.dragging.is_none() {
                if let Some(i) = self.piece_at(p) {
                    self.dragging = Some(i);
                    self.grab = self.pieces[i].pos - p;
                    self.pieces[i].on_plate = false;
                    self.idle_since = ctx.t;
                }
            }
            if let Some(i) = self.dragging {
                self.pieces[i].goal = p + self.grab * 0.6;
                if input.released || !input.down {
                    let at = self.pieces[i].goal;
                    self.place(i, at, ctx);
                    self.dragging = None;
                    self.idle_since = ctx.t;
                }
            }
        } else if let Some(i) = self.dragging.take() {
            let at = self.pieces[i].pos;
            self.place(i, at, ctx);
        }
        // keyboard: place the next piece for the player (assisted)
        if (input.hold_pressed || input.confirm) && self.dragging.is_none() {
            if let Some(i) = self.pieces.iter().position(|p| !p.on_plate) {
                let j = vec2(((i as f32) * 2.3).sin(), ((i as f32) * 1.7).cos()) * 6.0;
                let at = self.pieces[i].target + j;
                self.place(i, at, ctx);
                self.idle_since = ctx.t;
            } else if self.all_placed_at.is_some() {
                return Some(self.score(ctx.relaxed));
            }
        }
        for (i, pc) in self.pieces.iter_mut().enumerate() {
            let dragged = self.dragging == Some(i);
            let tau = if dragged { 0.035 } else { 0.09 };
            pc.pos += (pc.goal - pc.pos) * (1.0 - (-dt / tau).exp());
            pc.lift = approach(pc.lift, if dragged { 1.0 } else { 0.0 }, dt, 0.06);
            pc.bounce = (pc.bounce - dt * 2.6).max(0.0);
        }
        let all = self.pieces.iter().all(|p| p.on_plate) && self.dragging.is_none();
        if all {
            self.all_placed_at.get_or_insert(ctx.t);
        } else {
            self.all_placed_at = None;
        }
        let auto = all && ctx.t - self.idle_since > 1.8;
        if auto || ctx.t > MAX_T {
            return Some(self.score(ctx.relaxed));
        }
        None
    }

    fn score(&self, relaxed: bool) -> f32 {
        let dists: Vec<f32> = self
            .pieces
            .iter()
            .map(|p| if p.on_plate { p.goal.distance(p.target) } else { 200.0 })
            .collect();
        let mut overlaps = 0;
        for i in 0..self.pieces.len() {
            for j in i + 1..self.pieces.len() {
                let (a, b) = (&self.pieces[i], &self.pieces[j]);
                if a.on_plate && b.on_plate && a.goal.distance(b.goal) < (a.kind.radius() + b.kind.radius()) * 0.55 {
                    overlaps += 1;
                }
            }
        }
        let off = self.pieces.iter().filter(|p| !p.on_plate).count();
        plate_perf(&dists, overlaps, off, relaxed)
    }

    pub fn autoplay(&self, t: f32) -> MgInput {
        let lt = t - 0.8;
        if lt < 0.0 {
            return MgInput::default();
        }
        let i = (lt / 1.4) as usize;
        let Some(pc) = self.pieces.get(i) else {
            return MgInput::default();
        };
        let k = lt - i as f32 * 1.4;
        if k < 0.25 {
            MgInput {
                pointer: Some(pc.home),
                down: false,
                ..Default::default()
            }
        } else if k < 1.1 {
            let e = ease((k - 0.25) / 0.8);
            // grab offset is 0 when grabbing at the centre, so aim straight at the target
            MgInput {
                pointer: Some(pc.home.lerp(pc.target, e)),
                pressed: true,
                down: true,
                ..Default::default()
            }
        } else {
            MgInput {
                pointer: Some(pc.target),
                released: k < 1.13,
                down: false,
                ..Default::default()
            }
        }
    }

    pub fn hint(&self) -> Hint<'static> {
        if self.all_placed_at.is_some() {
            Hint {
                key: "minigame.hint.plate_serve",
                emphasis: false,
                keys: &[],
                serve: true,
            }
        } else {
            Hint {
                key: "minigame.hint.plate",
                emphasis: false,
                keys: &["Space"],
                serve: false,
            }
        }
    }

    pub fn draw(&self, _ui: &mut Ui, o: Vec2, t: f32) {
        let c = o + C;
        // tray
        let tray = TRAY.offset(o);
        contact_shadow(vec2(tray.center().x + 4.0, tray.bottom() - 2.0), tray.w * 0.6, 12.0, 0.5);
        fill_rrect(tray.translate_y(4.0), 14.0, hex(0x4a2c22));
        fill_rrect(tray, 14.0, Paint::vertical(tray, hex(0x7a5038), hex(0x543424)));
        for i in 0..7u32 {
            let y = tray.y + 14.0 + i as f32 * 36.0 + rd::hash01(i * 9) * 8.0;
            draw::hline(tray.x + 10.0, tray.right() - 10.0, y, Color::new(0.25, 0.12, 0.06, 0.18));
        }
        draw::stroke_rrect(tray, 14.0, 1.0, Color::new(1.0, 0.85, 0.65, 0.18));
        for pc in &self.pieces {
            let h = o + pc.home;
            rd::radial(h.x, h.y + 2.0, pc.kind.radius() + 6.0, pc.kind.radius() * 0.75, Color::new(0.2, 0.1, 0.05, 0.28), Color::new(0.2, 0.1, 0.05, 0.0));
        }

        // plate
        contact_shadow(c + vec2(6.0, 12.0), PLATE_R + 14.0, PLATE_R * 0.95, 0.55);
        fill_circle(c, PLATE_R, Paint::diagonal(Rect::new(c.x - PLATE_R, c.y - PLATE_R, PLATE_R * 2.0, PLATE_R * 2.0), hex(0xfbf8fe), hex(0xb5a8cf)));
        stroke_circle(c, PLATE_R - 1.0, 1.2, Color::new(1.0, 1.0, 1.0, 0.45));
        fill_circle(c, WELL_R + 2.0, Paint::diagonal(Rect::new(c.x - WELL_R, c.y - WELL_R, WELL_R * 2.0, WELL_R * 2.0), hex(0xcfc4e2), hex(0xf6f2fb)));
        stroke_circle(c, PLATE_R - 12.0, 1.4, with_alpha(hex(0x8b7fb8), 0.35));
        if self.set == PlateSet::Soup {
            // a deep bowl of kulajda
            rd::soft_shadow(c.x + 5.0, c.y + 7.0, 82.0, 78.0, 0.35);
            fill_circle(c, 80.0, Paint::diagonal(Rect::new(c.x - 80.0, c.y - 80.0, 160.0, 160.0), hex(0xfdfbff), hex(0xbdb1d6)));
            fill_circle(c, 68.0, Paint::diagonal(Rect::new(c.x - 68.0, c.y - 68.0, 136.0, 136.0), hex(0xbfb4d4), hex(0xece6f5)));
            rd::radial(c.x, c.y, 63.0, 63.0, hex(0xf1e4b8), hex(0xd8c48a));
            for i in 0..26u32 {
                let a = rd::hash01(i * 7 + 2) * std::f32::consts::TAU;
                let r = rd::hash01(i * 13 + 5).sqrt() * 54.0;
                let p = c + vec2(a.cos(), a.sin()) * r;
                fill_circle(p, 1.0 + rd::hash01(i) * 1.4, if i % 4 == 0 { Color::new(0.98, 0.95, 0.85, 0.9) } else { Color::new(0.85, 0.7, 0.36, 0.45) });
            }
            draw::arc(c, 66.0, 3.6, 5.2, 2.0, Color::new(1.0, 1.0, 1.0, 0.6));
        } else {
            draw::arc(c, PLATE_R - 3.0, 3.5, 5.2, 2.0, Color::new(1.0, 1.0, 1.0, 0.7));
        }

        // soft targets with a ghost of the piece
        let dragging = self.dragging.map(|i| &self.pieces[i]);
        for (i, pc) in self.pieces.iter().enumerate() {
            let occupied = self.pieces.iter().any(|q| q.on_plate && q.goal.distance(pc.target) < 4.0);
            if occupied {
                continue;
            }
            let tp = o + pc.target;
            let near = dragging
                .filter(|d| std::ptr::eq(*d, pc))
                .map(|d| 1.0 - ((d.pos.distance(pc.target) - 10.0) / 50.0).clamp(0.0, 1.0))
                .unwrap_or(0.0);
            let pulse = 0.5 + 0.5 * (t * 2.4 + i as f32).sin();
            let r = pc.kind.radius() * 0.85 + pulse * 2.0;
            crate::ui::draw::glow_circle(tp, r + 14.0, Color::new(0.65, 0.6, 1.0, 0.12 + 0.25 * near));
            let segs = 14;
            for s in 0..segs {
                let a0 = s as f32 / segs as f32 * std::f32::consts::TAU + t * 0.3;
                draw::arc(tp, r, a0, a0 + 0.24, 1.6, Color::new(0.55, 0.5, 0.95, 0.4 + 0.4 * near));
            }
            draw_piece(pc.kind, tp, 0.85, pc.rot, 0.14 + 0.1 * near, t);
        }

        // pieces: placed ones first, the dragged one last
        let mut order: Vec<usize> = (0..self.pieces.len()).collect();
        order.sort_by_key(|i| (self.dragging == Some(*i), (self.pieces[*i].pos.y * 10.0) as i32));
        for i in order {
            let pc = &self.pieces[i];
            let p = o + pc.pos;
            let lift = pc.lift;
            let b = pc.bounce;
            let s = 1.0 + 0.1 * lift + 0.12 * (b * std::f32::consts::PI).sin() * b;
            rd::soft_shadow(p.x + 2.0 + lift * 6.0, p.y + 4.0 + lift * 10.0, pc.kind.radius() * (1.0 + lift * 0.2), pc.kind.radius() * 0.6, 0.35 - lift * 0.12);
            draw_piece(pc.kind, p - vec2(0.0, lift * 8.0), s, pc.rot, 1.0, t);
        }
    }
}

trait TranslateY {
    fn translate_y(self, d: f32) -> Rect;
}
impl TranslateY for Rect {
    fn translate_y(self, d: f32) -> Rect {
        Rect::new(self.x, self.y + d, self.w, self.h)
    }
}

fn rot(v: Vec2, a: f32) -> Vec2 {
    let (s, c) = a.sin_cos();
    vec2(v.x * c - v.y * s, v.x * s + v.y * c)
}

fn a(c: Color, k: f32) -> Color {
    Color { a: c.a * k, ..c }
}

/// Leaf (lens) polygon from `base` towards `tip`.
fn leaf(base: Vec2, tip: Vec2, width: f32) -> Vec<Vec2> {
    let d = tip - base;
    let n = vec2(-d.y, d.x).normalize_or_zero();
    let mut pts = Vec::new();
    for i in 0..=8 {
        let u = i as f32 / 8.0;
        let w = (u * std::f32::consts::PI).sin() * width;
        pts.push(base + d * u + n * w);
    }
    for i in (1..8).rev() {
        let u = i as f32 / 8.0;
        let w = (u * std::f32::consts::PI).sin() * width;
        pts.push(base + d * u - n * w);
    }
    pts
}

/// Draws one garnish piece centred at `p`, scaled by `s`, alpha `k`.
fn draw_piece(kind: PieceKind, p: Vec2, s: f32, r: f32, k: f32, t: f32) {
    let q = |v: Vec2| p + rot(v * s, r);
    match kind {
        PieceKind::Slice => {
            let h = 13.0;
            let tip = vec2(-36.0, 8.0);
            let b0 = vec2(26.0, -20.0);
            let b1 = vec2(32.0, 12.0);
            // front face with honey-cake layers
            let layers = 6;
            for i in 0..layers {
                let y0 = i as f32 / layers as f32 * h;
                let y1 = (i + 1) as f32 / layers as f32 * h;
                let col = if i % 2 == 0 { hex(0xe8cc96) } else { hex(0xa45f2c) };
                fill_convex(
                    &[q(tip + vec2(0.0, y0)), q(b1 + vec2(0.0, y0)), q(b1 + vec2(0.0, y1)), q(tip + vec2(0.0, y1))],
                    Paint::Solid(a(col, k)),
                );
            }
            fill_convex(
                &[q(b0), q(b1), q(b1 + vec2(0.0, h)), q(b0 + vec2(0.0, h))],
                Paint::Solid(a(hex(0x8a4f26), k)),
            );
            // top with crumbs
            fill_convex(&[q(tip), q(b0), q(b1)], Paint::Solid(a(hex(0xc48a4c), k)));
            for i in 0..14u32 {
                let u = rd::hash01(i * 3 + 1);
                let v = rd::hash01(i * 7 + 2) * (1.0 - u);
                let pt = tip + (b0 - tip) * u + (b1 - tip) * v;
                fill_circle(q(pt), 1.4 * s, a(if i % 3 == 0 { hex(0xe9c48a) } else { hex(0x7f4a22) }, k));
            }
            draw::line(q(tip), q(b1), 1.0, a(Color::new(1.0, 0.92, 0.75, 0.5), k));
        }
        PieceKind::Cream => {
            for (i, (rr, dy)) in [(19.0, 0.0), (15.0, -5.0), (10.5, -9.5), (6.0, -13.0)].iter().enumerate() {
                let c0 = q(vec2(0.0, *dy));
                let col = draw::mix(hex(0xe9e1d6), hex(0xfffdf8), i as f32 / 3.0);
                fill_circle(c0, rr * s, a(col, k));
            }
            let c0 = q(vec2(0.0, 0.0));
            for i in 0..3 {
                let rr = (7.0 + i as f32 * 4.5) * s;
                draw::arc(c0 + vec2(0.0, -(9.0 - i as f32 * 4.0) * s), rr, 3.4 + r, 5.0 + r, 1.2, a(Color::new(1.0, 1.0, 1.0, 0.8), k));
            }
            fill_circle(q(vec2(-1.0, -16.0)), 2.2 * s, a(WHITE, k));
        }
        PieceKind::Berries => {
            let leaf_pts: Vec<Vec2> = leaf(vec2(2.0, -4.0), vec2(16.0, -16.0), 5.0).into_iter().map(q).collect();
            fill_convex(&leaf_pts, Paint::Solid(a(hex(0x4f9a6b), k)));
            for (c0, rr, col) in [(vec2(-8.0, 2.0), 8.5, hex(0x4a3f8f)), (vec2(7.0, 6.0), 8.0, hex(0x3d3478))] {
                let cc = q(c0);
                fill_circle(cc, rr * s, a(col, k));
                fill_circle(cc + vec2(-2.5, -2.5) * s, rr * 0.45 * s, a(Color::new(0.75, 0.72, 0.95, 0.45), k));
                fill_circle(cc + vec2(1.5, 1.5) * s, 1.4 * s, a(hex(0x231c45), k));
            }
            // raspberry
            let rc = q(vec2(-1.0, -9.0));
            fill_circle(rc, 8.0 * s, a(hex(0xc2304f), k));
            for i in 0..7 {
                let ang = i as f32 / 7.0 * std::f32::consts::TAU;
                fill_circle(rc + vec2(ang.cos(), ang.sin()) * 4.6 * s, 2.6 * s, a(hex(0xe0566f), k));
            }
            fill_circle(rc + vec2(-2.0, -2.5) * s, 1.6 * s, a(Color::new(1.0, 0.85, 0.9, 0.7), k));
        }
        PieceKind::Mint => {
            let sway = (t * 1.2).sin() * 0.04;
            for (base, tip, w) in [
                (vec2(0.0, 8.0), vec2(-14.0, -10.0), 6.5),
                (vec2(0.0, 8.0), vec2(13.0, -11.0), 6.5),
                (vec2(0.0, 8.0), vec2(0.0, -16.0), 7.0),
            ] {
                let pts: Vec<Vec2> = leaf(base, rot(tip, sway), w).into_iter().map(q).collect();
                fill_convex(&pts, Paint::Solid(a(hex(0x3f8f5e), k)));
                draw::line(q(base), q(base + (rot(tip, sway) - base) * 0.85), 0.9, a(hex(0x9ed8a8), k));
            }
            draw::line(q(vec2(0.0, 8.0)), q(vec2(2.0, 15.0)), 1.6, a(hex(0x2f6a4f), k));
        }
        PieceKind::Honey => {
            let mut pts = Vec::new();
            for i in 0..=24 {
                let u = i as f32 / 24.0;
                pts.push(q(vec2(-18.0 + u * 36.0, (u * 18.0).sin() * 9.0)));
            }
            draw::stroke_path(&pts, false, 3.4 * s, Paint::Solid(a(hex(0xe39a2c), k)));
            let hl: Vec<Vec2> = pts.iter().map(|v| *v + vec2(-0.6, -0.8)).collect();
            draw::stroke_path(&hl, false, 1.0, Paint::Solid(a(Color::new(1.0, 0.92, 0.6, 0.8), k)));
            fill_circle(q(vec2(18.0, 0.0)), 3.6 * s, a(hex(0xf0b043), k));
        }
        PieceKind::Croutons => {
            for (i, (c0, ang)) in [(vec2(-8.0, -5.0), 0.3), (vec2(7.0, -2.0), -0.4), (vec2(-1.0, 9.0), 0.9), (vec2(11.0, 11.0), 0.1)].iter().enumerate() {
                let sz = if i == 3 { 5.0 } else { 6.5 };
                let corners: Vec<Vec2> = [vec2(-1.0, -1.0), vec2(1.0, -1.0), vec2(1.0, 1.0), vec2(-1.0, 1.0)]
                    .iter()
                    .map(|v| q(*c0 + rot(*v * sz, *ang)))
                    .collect();
                fill_convex(&corners, Paint::Solid(a(hex(0xc98a3e), k)));
                fill_circle(q(*c0 + vec2(-1.5, -1.5)), 2.0 * s, a(hex(0xf0c27a), k));
            }
        }
        PieceKind::Swirl => {
            let pts: Vec<Vec2> = (0..=48)
                .map(|i| {
                    let u = i as f32 / 48.0;
                    let ang = u * std::f32::consts::TAU * 2.2;
                    let rr = 2.0 + u * 14.0;
                    q(vec2(ang.cos() * rr, ang.sin() * rr * 0.92))
                })
                .collect();
            draw::stroke_path(&pts, false, 5.6 * s, Paint::Solid(a(hex(0xe4d9c2), k)));
            draw::stroke_path(&pts, false, 3.6 * s, Paint::Solid(a(hex(0xfffcf4), k)));
            fill_circle(q(vec2(0.0, 0.0)), 2.6 * s, a(hex(0xfffcf4), k));
        }
        PieceKind::Dill => {
            let stem = [vec2(0.0, 14.0), vec2(-2.0, 0.0), vec2(1.0, -14.0)];
            let pts: Vec<Vec2> = stem.iter().map(|v| q(*v)).collect();
            draw::stroke_path(&pts, false, 1.4 * s, Paint::Solid(a(hex(0x3f7d4a), k)));
            for i in 0..9 {
                let u = i as f32 / 8.0;
                let base = vec2(-2.0 + u * 3.0, 12.0 - u * 26.0);
                for side in [-1.0f32, 1.0] {
                    let tip = base + vec2(side * (9.0 - u * 4.0), -5.0);
                    draw::line(q(base), q(tip), 1.0, a(hex(0x5fae64), k));
                    draw::line(q(base.lerp(tip, 0.5)), q(base.lerp(tip, 0.5) + vec2(side * 2.0, -3.0)), 0.8, a(hex(0x7cc47d), k));
                }
            }
        }
    }
}
