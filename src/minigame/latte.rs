//! LatteArt: a cup from above; drag to pour steamed milk onto the crema. Milk
//! spreads as soft blobs, a faint ghost shows the pattern, the score is the soft
//! overlap between a 48×48 milk grid and the rasterised target.

use macroquad::prelude::*;

use super::paint::{contact_shadow, hex, with_alpha};
use super::score::{latte_perf, soft_iou};
use super::shapes::{LatteShape, cell_center};
use super::{Ctx, Hint, MgInput, MiniGameSound, MiniGameText, RegularTouch};
use crate::render::draw as rd;
use crate::ui::draw::{self, Paint, fill_circle, fill_rrect, stroke_circle};
use crate::ui::{Align, Glass, RectExt, Ui, approach, draw_glass};

const C: Vec2 = Vec2::new(178.0, 141.0);
const CREMA_R: f32 = 84.0;
const CUP_IN: f32 = 92.0;
const CUP_OUT: f32 = 104.0;
const SAUCER: f32 = 128.0;
const GRID: usize = 48;
const BUDGET: f32 = 9.0;
const MAX_T: f32 = 20.0;
const SPEED: f32 = 80.0;
const PANEL: Rect = Rect {
    x: 334.0,
    y: 22.0,
    w: 142.0,
    h: 238.0,
};

struct Blob {
    c: Vec2,
    r: f32,
    born: f32,
}

/// Scripted stroke for autoplay: `(t0, t1, a, b, pour)`.
type Seg = (f32, f32, Vec2, Vec2, bool);

pub(crate) struct Latte {
    shape: LatteShape,
    target: Vec<f32>,
    runs: Vec<[f32; 4]>,
    milk: Vec<f32>,
    blobs: Vec<Blob>,
    flow: f32,
    mouse_hold: bool,
    cursor: Vec2,
    pointer: Option<Vec2>,
    pour_point: Vec2,
    pouring: bool,
    stroke_last: Option<Vec2>,
    milk_left: f32,
    poured: f32,
    empty_at: Option<f32>,
    done: bool,
    cinnamon: bool,
    script: Vec<Seg>,
    t: f32,
}

fn to_unit(p: Vec2) -> Vec2 {
    (p - C) / CREMA_R
}

impl Latte {
    pub fn new(shape: LatteShape) -> Latte {
        let target = shape.rasterize(GRID);
        let script = build_script(&target);
        Latte {
            shape,
            target,
            runs: shape.runs(110),
            milk: vec![0.0; GRID * GRID],
            blobs: Vec::new(),
            flow: 0.0,
            mouse_hold: false,
            cursor: C + vec2(0.0, 30.0),
            pointer: None,
            pour_point: C,
            pouring: false,
            stroke_last: None,
            milk_left: 1.0,
            poured: 0.0,
            empty_at: None,
            done: false,
            cinnamon: false,
            script,
            t: 0.0,
        }
    }

    pub fn flow(&self) -> f32 {
        self.flow
    }

    pub fn focus(&self) -> Vec2 {
        C
    }

    fn stamp(&mut self, c: Vec2, r: f32) {
        let u = to_unit(c);
        let ur = r / CREMA_R;
        let cell = 2.0 / GRID as f32;
        let i0 = (((u.x - ur + 1.0) / cell).floor().max(0.0)) as usize;
        let i1 = ((((u.x + ur + 1.0) / cell).ceil()) as usize).min(GRID);
        let j0 = (((u.y - ur + 1.0) / cell).floor().max(0.0)) as usize;
        let j1 = ((((u.y + ur + 1.0) / cell).ceil()) as usize).min(GRID);
        for j in j0..j1 {
            for i in i0..i1 {
                let p = cell_center(i, j, GRID);
                if p.length() > 1.0 {
                    continue;
                }
                let d = p.distance(u);
                // soft edge one cell wide
                let v = ((ur - d) / cell + 0.5).clamp(0.0, 1.0);
                let m = &mut self.milk[j * GRID + i];
                *m = m.max(v);
            }
        }
    }

    fn deposit(&mut self, p: Vec2, dt: f32) {
        let r_move = 5.5 + 3.5 * self.flow;
        match self.stroke_last {
            Some(last) if last.distance(p) < 3.0 => {
                // pouring on one spot: the circle grows
                if let Some(b) = self.blobs.last_mut() {
                    b.r = (b.r + 10.0 * dt * self.flow).min(30.0);
                    let (c, r) = (b.c, b.r);
                    self.stamp(c, r);
                }
            }
            Some(last) => {
                let d = last.distance(p);
                let n = (d / 3.0).ceil() as usize;
                for k in 1..=n {
                    let q = last.lerp(p, k as f32 / n as f32);
                    self.blobs.push(Blob {
                        c: q,
                        r: r_move,
                        born: self.t,
                    });
                    self.stamp(q, r_move);
                }
                self.stroke_last = Some(p);
            }
            None => {
                self.blobs.push(Blob {
                    c: p,
                    r: r_move,
                    born: self.t,
                });
                self.stamp(p, r_move);
                self.stroke_last = Some(p);
            }
        }
    }

    pub fn step(&mut self, ctx: &mut Ctx, input: &MgInput, dt: f32) -> Option<f32> {
        self.t = ctx.t;
        if input.pressed {
            self.mouse_hold = true;
        }
        if input.released || !input.down {
            self.mouse_hold = false;
        }
        if input.pointer != self.pointer {
            if let Some(p) = input.pointer {
                self.cursor = p;
            }
            self.pointer = input.pointer;
        }
        if input.axis != Vec2::ZERO {
            self.cursor += input.axis.normalize_or_zero() * 150.0 * dt;
        }
        let off = self.cursor - C;
        if off.length() > CREMA_R - 4.0 {
            self.cursor = C + off.normalize_or_zero() * (CREMA_R - 4.0);
        }
        let p = if self.mouse_hold {
            input.pointer.unwrap_or(self.cursor)
        } else {
            self.cursor
        };
        self.pour_point = p;
        let want = (self.mouse_hold || input.hold) && self.milk_left > 0.0;
        if want && !self.pouring {
            ctx.sounds.push(MiniGameSound::Pour);
        }
        self.pouring = want;
        self.flow = approach(self.flow, if want { 1.0 } else { 0.0 }, dt, 0.12);
        let inside = p.distance(C) < CREMA_R - 2.0;
        if want && inside {
            let budget = BUDGET * super::score::widen(ctx.relaxed).sqrt();
            self.milk_left = (self.milk_left - dt / budget).max(0.0);
            self.poured += dt;
            self.deposit(p, dt);
        } else {
            self.stroke_last = None;
        }
        if self.milk_left <= 0.0 && self.empty_at.is_none() {
            self.empty_at = Some(ctx.t);
        }
        let serve = input.confirm && self.poured > 0.8;
        let finish = serve
            || self.empty_at.is_some_and(|e| ctx.t - e > 0.5)
            || ctx.t > MAX_T;
        if finish {
            self.done = true;
            self.pouring = false;
            self.flow = 0.0;
            self.cinnamon = ctx.touch == Some(RegularTouch::Cinnamon);
            ctx.sounds.push(MiniGameSound::Clink);
            return Some(latte_perf(soft_iou(&self.milk, &self.target), ctx.relaxed));
        }
        None
    }

    pub fn autoplay(&self, t: f32) -> MgInput {
        let start = 0.8;
        let lt = t - start;
        if lt < 0.0 {
            return MgInput {
                pointer: Some(self.script.first().map(|s| s.2).unwrap_or(C)),
                ..Default::default()
            };
        }
        for &(t0, t1, a, b, pour) in &self.script {
            if lt >= t0 && lt < t1 {
                let k = ((lt - t0) / (t1 - t0).max(1e-4)).clamp(0.0, 1.0);
                return MgInput {
                    pointer: Some(a.lerp(b, k)),
                    pressed: pour,
                    down: pour,
                    ..Default::default()
                };
            }
        }
        MgInput {
            confirm: true,
            ..Default::default()
        }
    }

    pub fn hint(&self) -> Hint<'static> {
        if self.poured > 0.8 && !self.pouring {
            Hint {
                key: "minigame.hint.latte_serve",
                emphasis: false,
                keys: &[],
                serve: true,
            }
        } else {
            Hint {
                key: "minigame.hint.latte",
                emphasis: false,
                keys: &[],
                serve: self.poured > 0.8,
            }
        }
    }

    pub fn draw(&self, ui: &mut Ui, o: Vec2, t: f32, text: &MiniGameText) {
        let c = o + C;
        // saucer
        contact_shadow(c + vec2(6.0, 10.0), SAUCER + 12.0, SAUCER * 0.92, 0.5);
        fill_circle(c, SAUCER, Paint::diagonal(Rect::new(c.x - SAUCER, c.y - SAUCER, SAUCER * 2.0, SAUCER * 2.0), hex(0xe9e2f4), hex(0xa89cc4)));
        fill_circle(c + vec2(2.0, 3.0), SAUCER - 16.0, with_alpha(hex(0x8f82ad), 0.35));
        stroke_circle(c, SAUCER - 1.0, 1.2, Color::new(1.0, 1.0, 1.0, 0.35));
        // cup shadow on the saucer, handle, body
        rd::soft_shadow(c.x + 7.0, c.y + 9.0, CUP_OUT + 8.0, CUP_OUT + 6.0, 0.4);
        let handle = Rect::new(c.x + CUP_OUT - 12.0, c.y - 12.0, 42.0, 24.0);
        fill_rrect(handle, 12.0, Paint::vertical(handle, hex(0xf6f1f8), hex(0xc9bfd8)));
        fill_rrect(handle.inset_ltrb(16.0, 7.0, 9.0, 7.0), 5.0, hex(0xb3a7c8));
        fill_circle(c, CUP_OUT, Paint::diagonal(Rect::new(c.x - CUP_OUT, c.y - CUP_OUT, CUP_OUT * 2.0, CUP_OUT * 2.0), hex(0xfcf9fd), hex(0xc4b9d6)));
        fill_circle(c, CUP_IN + 1.0, Paint::diagonal(Rect::new(c.x - CUP_IN, c.y - CUP_IN, CUP_IN * 2.0, CUP_IN * 2.0), hex(0xc9bfd9), hex(0xf4effa)));
        // crema
        rd::radial(c.x, c.y, CREMA_R + 5.0, CREMA_R + 5.0, hex(0xb07a46), hex(0x5e3520));
        rd::glow(c.x - 14.0, c.y - 16.0, 52.0, Color::new(0.85, 0.58, 0.32, 0.35));
        // tiger speckles
        for i in 0..70u32 {
            let a = rd::hash01(i * 11 + 5) * std::f32::consts::TAU;
            let r = rd::hash01(i * 23 + 9).sqrt() * (CREMA_R - 6.0);
            let p = c + vec2(a.cos(), a.sin()) * r;
            let s = 0.8 + rd::hash01(i * 5 + 2) * 1.6;
            let col = if i % 3 == 0 {
                Color::new(0.95, 0.72, 0.42, 0.25)
            } else {
                Color::new(0.25, 0.12, 0.06, 0.25)
            };
            fill_circle(p, s, col);
        }
        // crema edge ring (the radial already darkens; this hides its polygon edge)
        draw::stroke_circle(c, CREMA_R + 4.0, 9.0, hex(0x4e2b19));

        // ghost of the target
        let ghost_a = if self.done { 0.0 } else { 0.11 + 0.05 * (t * 2.2).sin() };
        if ghost_a > 0.0 {
            for run in &self.runs {
                let y0 = c.y + run[0] * CREMA_R;
                let y1 = c.y + run[1] * CREMA_R;
                let x0 = c.x + run[2] * CREMA_R;
                let x1 = c.x + run[3] * CREMA_R;
                draw_rectangle(x0, y0, x1 - x0, y1 - y0, Color::new(1.0, 0.97, 0.92, ghost_a));
            }
        }

        // milk: halo pass, then cores
        let grow = |b: &Blob| b.r * (0.55 + 0.45 * crate::ui::ease_out((t - b.born) / 0.4));
        for b in &self.blobs {
            fill_circle(o + b.c, grow(b) + 1.8, Color::new(0.83, 0.66, 0.48, 0.9));
        }
        for b in &self.blobs {
            fill_circle(o + b.c, grow(b), hex(0xf8f2e8));
        }
        // a soft sheen across the milk
        if !self.blobs.is_empty() {
            rd::radial(c.x - 26.0, c.y - 30.0, 60.0, 50.0, Color::new(1.0, 1.0, 1.0, 0.08), Color::new(1.0, 1.0, 1.0, 0.0));
        }
        if self.cinnamon {
            for i in 0..140u32 {
                let a = rd::hash01(i * 13 + 1) * std::f32::consts::TAU;
                let r = rd::hash01(i * 29 + 4).sqrt() * (CREMA_R - 8.0);
                let p = c + vec2(a.cos(), a.sin()) * r;
                fill_circle(p, 0.7 + rd::hash01(i) * 0.8, Color::new(0.55, 0.3, 0.14, 0.55));
            }
        }
        // rim highlight
        draw::arc(c, CUP_OUT - 2.0, 3.5, 5.3, 2.0, Color::new(1.0, 1.0, 1.0, 0.6));

        // pour stream + ripples
        if !self.done {
            let p = o + self.pour_point;
            if self.flow > 0.02 {
                let wob = (t * 9.0).sin() * 1.2;
                let top = vec2(p.x + 14.0 + wob, o.y - 8.0);
                rd::glow(p.x, p.y, 16.0, Color::new(1.0, 0.98, 0.94, 0.25 * self.flow));
                rd::capsule(top, p, 3.0 * self.flow + 0.5, Color::new(0.98, 0.95, 0.9, 0.85 * self.flow));
                rd::capsule(top + vec2(-1.0, 0.0), p + vec2(-1.0, 0.0), 0.9, Color::new(1.0, 1.0, 1.0, 0.6 * self.flow));
                for k in 0..2 {
                    let ph = ((t * 1.8) + k as f32 * 0.5).fract();
                    stroke_circle(p, 6.0 + ph * 18.0, 1.2, Color::new(1.0, 0.96, 0.9, 0.35 * (1.0 - ph) * self.flow));
                }
            } else if self.pointer.is_some_and(|q| q.distance(C) < CUP_IN) || self.blobs.is_empty() {
                let pulse = 0.5 + 0.5 * (t * 3.0).sin();
                stroke_circle(p, 9.0 + pulse * 2.0, 1.4, Color::new(0.85, 0.84, 1.0, 0.45));
                fill_circle(p, 2.0, Color::new(0.95, 0.94, 1.0, 0.7));
            }
        }

        self.draw_panel(ui, o, t, text);
    }

    fn draw_panel(&self, ui: &mut Ui, o: Vec2, t: f32, text: &MiniGameText) {
        let th = ui.theme.clone();
        let r = PANEL.offset(o);
        draw_glass(&th, r, &Glass::card());
        let micro = th.micro();
        ui.fonts.draw(&text.get("minigame.target").to_uppercase(), r.x + 12.0, r.y + 20.0, &micro);
        let name = text.get(self.shape.key());
        let st = th.strong();
        let name = ui.fonts.ellipsize(name, r.w - 24.0, &st).into_owned();
        ui.fonts.draw(&name, r.x + 12.0, r.y + 38.0, &st);
        // mini cup with the pattern
        let mc = vec2(r.center().x, r.y + 100.0);
        let mr = 44.0;
        crate::ui::draw::glow_circle(mc, mr + 16.0, Color::new(1.0, 0.75, 0.45, 0.08 + 0.03 * (t * 2.0).sin()));
        fill_circle(mc, mr + 6.0, Paint::diagonal(Rect::new(mc.x - mr, mc.y - mr, mr * 2.0, mr * 2.0), hex(0xfaf6fc), hex(0xbfb3d3)));
        fill_circle(mc, mr, Paint::diagonal(Rect::new(mc.x - mr, mc.y - mr, mr * 2.0, mr * 2.0), hex(0xb27a45), hex(0x5a321e)));
        for run in &self.runs {
            draw_rectangle(
                mc.x + run[2] * mr,
                mc.y + run[0] * mr,
                (run[3] - run[2]) * mr,
                (run[1] - run[0]) * mr + 0.2,
                hex(0xf6efe3),
            );
        }
        // milk left
        let bar = Rect::new(r.x + 12.0, r.bottom() - 40.0, r.w - 24.0, 8.0);
        ui.fonts.draw(&text.get("minigame.milk").to_uppercase(), bar.x, bar.y - 8.0, &micro);
        fill_rrect(bar, 4.0, Color::new(0.02, 0.01, 0.07, 0.55));
        let fw = (bar.w - 2.0) * self.milk_left;
        if fw > 1.0 {
            let f = Rect::new(bar.x + 1.0, bar.y + 1.0, fw, bar.h - 2.0);
            fill_rrect(f, 3.0, Paint::horizontal(f, hex(0xd9c6ad), hex(0xfffaf3)));
        }
        let pct = format!("{}%", (self.milk_left * 100.0).round() as i32);
        let ps = th.small().tabular();
        ui.fonts.draw_in(&pct, Rect::new(bar.x, bar.bottom() + 2.0, bar.w, 16.0), &ps, Align::Right);
    }
}

/// Raster sweep over the target: rows four cells apart, spans inset by a blob radius.
fn build_script(target: &[f32]) -> Vec<Seg> {
    let mut segs = Vec::new();
    let mut tcur = 0.0;
    let mut pos: Option<Vec2> = None;
    let mut flip = false;
    let inset = 6.0;
    for j in (2..GRID).step_by(4) {
        let mut spans: Vec<(f32, f32)> = Vec::new();
        let mut start: Option<usize> = None;
        for i in 0..=GRID {
            let on = i < GRID && target[j * GRID + i] > 0.5;
            match (on, start) {
                (true, None) => start = Some(i),
                (false, Some(s)) => {
                    let a = C.x + cell_center(s, j, GRID).x * CREMA_R;
                    let b = C.x + cell_center(i - 1, j, GRID).x * CREMA_R;
                    spans.push((a, b));
                    start = None;
                }
                _ => {}
            }
        }
        if flip {
            spans.reverse();
        }
        let y = C.y + cell_center(0, j, GRID).y * CREMA_R;
        for (a, b) in spans {
            let (a, b) = if flip { (b, a) } else { (a, b) };
            let (a, b) = if (b - a).abs() > inset * 2.0 {
                let s = (b - a).signum();
                (a + s * inset, b - s * inset)
            } else {
                let m = (a + b) * 0.5;
                (m, m)
            };
            let pa = vec2(a, y);
            let pb = vec2(b, y);
            if let Some(p) = pos {
                let d = p.distance(pa);
                segs.push((tcur, tcur + d / (SPEED * 2.0) + 0.05, p, pa, false));
                tcur += d / (SPEED * 2.0) + 0.05;
            }
            let d = pa.distance(pb).max(4.0);
            segs.push((tcur, tcur + d / SPEED + 0.08, pa, pb, true));
            tcur += d / SPEED + 0.08;
            pos = Some(pb);
        }
        flip = !flip;
    }
    segs
}

