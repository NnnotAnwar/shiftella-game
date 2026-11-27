//! LatteArt (§2.2): a cup from above. Hold the mouse and drag the spout from the start dot
//! along the faint guide; release lifts. Keyboard: hold Space — the spout follows the guide
//! by itself — and tap Left/Right at the marked wobble points.
//! Mouse quality: `0.6 × coverage + 0.4 × (1 − stray)`; keyboard: `mean(1 − |Δt| / 0.35)`.

use macroquad::prelude::*;

use super::latte_shapes::{LatteShape, length, point_at};
use super::paint::{contact_shadow, hex, with_alpha};
use super::score;
use super::{Common, Ctx, Hint, LatteMilk, MgInput, MiniGameSound, MiniGameText, RegularTouch};
use crate::render::draw as rd;
use crate::ui::draw::{self, Paint, fill_circle, fill_rrect, stroke_circle};
use crate::ui::{Glass, Icon, Ui, approach, draw_glass, draw_icon, ease_out};

const C: Vec2 = Vec2::new(168.0, 124.0);
const CREMA_R: f32 = 76.0;
const CUP_IN: f32 = 84.0;
const CUP_OUT: f32 = 95.0;
const SAUCER: f32 = 116.0;
const MAX_T: f32 = 16.0;
const PANEL: Rect = Rect {
    x: 330.0,
    y: 12.0,
    w: 148.0,
    h: 224.0,
};
const START_DELAY: f32 = 0.6;

struct Blob {
    c: Vec2,
    r: f32,
    born: f32,
}

/// Keyboard / autoplay timeline piece; `stroke: None` = travelling, not pouring.
#[derive(Clone, Copy)]
struct Leg {
    t0: f32,
    t1: f32,
    stroke: Option<usize>,
    a: Vec2,
    b: Vec2,
}

/// The pattern's colour: dairy cream, oat beige, butterfly-pea blue.
pub(crate) fn milk_color(m: LatteMilk) -> Color {
    match m {
        LatteMilk::Dairy => hex(0xf8f2e8),
        LatteMilk::Oat => hex(0xf1e3c8),
        LatteMilk::Butterfly => hex(0x9fb6f2),
    }
}

pub(crate) struct Latte {
    shape: LatteShape,
    milk: LatteMilk,
    common: Common,
    strokes: Vec<Vec<Vec2>>,
    covered: Vec<Vec<bool>>,
    guide48: Vec<Vec2>,
    /// (position, time on the keyboard timeline, left?)
    marks: Vec<(Vec2, f32, bool)>,
    taps: Vec<Option<f32>>,
    legs: Vec<Leg>,
    kb_t: f32,
    kb_used: bool,
    trace: Vec<Vec2>,
    blobs: Vec<Blob>,
    flow: f32,
    mouse_hold: bool,
    pointer: Option<Vec2>,
    pour_point: Vec2,
    pouring: bool,
    stroke_last: Option<Vec2>,
    complete_at: Option<f32>,
    done: bool,
    cinnamon: bool,
    touch: Option<RegularTouch>,
    t: f32,
}

fn to_px(u: Vec2) -> Vec2 {
    C + u * CREMA_R
}

fn build_legs(strokes: &[Vec<Vec2>]) -> Vec<Leg> {
    let total: f32 = strokes.iter().map(|s| length(s)).sum();
    // about 8.5 s of pouring whatever the pattern
    let speed = (total / 8.5).clamp(40.0, 120.0);
    let mut legs = Vec::new();
    let mut t = 0.0;
    let mut pos: Option<Vec2> = None;
    for (i, s) in strokes.iter().enumerate() {
        if let Some(p) = pos {
            let d = p.distance(s[0]);
            let dt = d / (speed * 2.5) + 0.25;
            legs.push(Leg {
                t0: t,
                t1: t + dt,
                stroke: None,
                a: p,
                b: s[0],
            });
            t += dt;
        }
        let dt = length(s) / speed;
        legs.push(Leg {
            t0: t,
            t1: t + dt,
            stroke: Some(i),
            a: s[0],
            b: *s.last().unwrap_or(&s[0]),
        });
        t += dt;
        pos = s.last().copied();
    }
    legs
}

impl Latte {
    pub fn new(
        shape: LatteShape,
        milk: LatteMilk,
        touch: Option<RegularTouch>,
        common: Common,
    ) -> Latte {
        let strokes: Vec<Vec<Vec2>> = shape
            .strokes()
            .into_iter()
            .map(|s| s.into_iter().map(to_px).collect())
            .collect();
        let legs = build_legs(&strokes);
        let marks = shape
            .marks()
            .into_iter()
            .map(|m| {
                let s = &strokes[m.stroke];
                let d = m.s * CREMA_R;
                let leg = legs
                    .iter()
                    .find(|l| l.stroke == Some(m.stroke))
                    .copied()
                    .expect("every stroke has a leg");
                let frac = d / length(s).max(1e-3);
                (point_at(s, d), leg.t0 + (leg.t1 - leg.t0) * frac, m.left)
            })
            .collect::<Vec<_>>();
        Latte {
            shape,
            milk,
            common,
            covered: strokes.iter().map(|s| vec![false; s.len()]).collect(),
            guide48: shape.guide_points(48).into_iter().map(to_px).collect(),
            taps: vec![None; marks.len()],
            marks,
            legs,
            strokes,
            kb_t: 0.0,
            kb_used: false,
            trace: Vec::new(),
            blobs: Vec::new(),
            flow: 0.0,
            mouse_hold: false,
            pointer: None,
            pour_point: C,
            pouring: false,
            stroke_last: None,
            complete_at: None,
            done: false,
            cinnamon: false,
            touch,
            t: 0.0,
        }
    }

    pub fn flow(&self) -> f32 {
        self.flow
    }

    pub fn focus(&self) -> Vec2 {
        C
    }

    fn timeline_end(&self) -> f32 {
        self.legs.last().map_or(0.0, |l| l.t1)
    }

    fn timeline_at(&self, t: f32) -> (Vec2, Option<usize>) {
        for l in &self.legs {
            if t >= l.t0 && t < l.t1 {
                let k = (t - l.t0) / (l.t1 - l.t0).max(1e-4);
                return match l.stroke {
                    Some(si) => {
                        let s = &self.strokes[si];
                        (point_at(s, length(s) * k), Some(si))
                    }
                    None => (l.a.lerp(l.b, k), None),
                };
            }
        }
        (self.legs.last().map_or(C, |l| l.b), None)
    }

    fn stroke_coverage(&self, i: usize) -> f32 {
        let c = &self.covered[i];
        c.iter().filter(|x| **x).count() as f32 / c.len().max(1) as f32
    }

    /// The first stroke that is not traced yet (its start dot is shown).
    fn next_stroke(&self) -> Option<usize> {
        (0..self.strokes.len()).find(|i| self.stroke_coverage(*i) < 0.8)
    }

    pub fn finished_gesture(&self) -> bool {
        self.next_stroke().is_none() || self.kb_t >= self.timeline_end()
    }

    fn deposit(&mut self, p: Vec2) {
        let r = 3.6 + 2.0 * self.flow;
        let last = self.stroke_last.unwrap_or(p);
        let d = last.distance(p);
        if self.stroke_last.is_some() && d < 2.0 {
            return;
        }
        let n = (d / 2.5).ceil().max(1.0) as usize;
        for k in 1..=n {
            let q = last.lerp(p, k as f32 / n as f32);
            self.blobs.push(Blob {
                c: q,
                r,
                born: self.t,
            });
        }
        self.stroke_last = Some(p);
    }

    fn add_trace(&mut self, p: Vec2, relaxed: bool) {
        if self.trace.last().is_some_and(|q| q.distance(p) < 1.5) {
            return;
        }
        self.trace.push(p);
        let r2 = (12.0 * score::widen(relaxed)).powi(2);
        for (s, cov) in self.strokes.iter().zip(self.covered.iter_mut()) {
            for (q, c) in s.iter().zip(cov.iter_mut()) {
                if !*c && q.distance_squared(p) <= r2 {
                    *c = true;
                }
            }
        }
    }

    pub fn steps(&self) -> Vec<&'static str> {
        Vec::new()
    }

    pub fn preview(&self, relaxed: bool) -> f32 {
        self.quality(relaxed)
    }

    pub fn advice(&self, relaxed: bool) -> &'static str {
        if self.kb_used && self.trace.len() < 10 {
            return "minigame.why.latte.taps";
        }
        let w = score::widen(relaxed);
        let cov = score::coverage(&self.guide48, &self.trace, 12.0 * w);
        let stray = score::stray(&self.strokes, &self.trace, 24.0 * w);
        if 0.6 * (1.0 - cov) >= 0.4 * stray {
            "minigame.why.latte.coverage"
        } else {
            "minigame.why.latte.stray"
        }
    }

    fn quality(&self, relaxed: bool) -> f32 {
        if self.kb_used && self.trace.len() < 10 {
            return score::latte_taps(&self.taps, relaxed);
        }
        let w = score::widen(relaxed);
        let cov = score::coverage(&self.guide48, &self.trace, 12.0 * w);
        let stray = score::stray(&self.strokes, &self.trace, 24.0 * w);
        score::latte_trace(cov, stray)
    }

    pub fn step(&mut self, ctx: &mut Ctx, input: &MgInput, dt: f32) -> Option<f32> {
        self.t = ctx.t;
        if input.pressed {
            self.mouse_hold = true;
        }
        if input.released || !input.down {
            self.mouse_hold = false;
        }
        self.pointer = input.pointer;
        let keyboard = input.hold && !self.mouse_hold && ctx.t > 0.2;
        let mut want = false;
        if keyboard {
            self.kb_used = true;
            if self.kb_t < self.timeline_end() {
                self.kb_t += dt;
            }
            let (p, stroke) = self.timeline_at(self.kb_t);
            self.pour_point = p;
            want = stroke.is_some();
        } else if self.mouse_hold {
            if let Some(p) = input.pointer {
                self.pour_point = p;
                want = p.distance(C) < CREMA_R - 2.0;
            }
        } else if let Some(p) = input.pointer {
            self.pour_point = p;
        }
        // wobble taps (keyboard path)
        if self.kb_used && (input.left || input.right) {
            let best = self
                .marks
                .iter()
                .enumerate()
                .filter(|(i, _)| self.taps[*i].is_none())
                .map(|(i, m)| (i, self.kb_t - m.1))
                .filter(|(_, d)| d.abs() <= 0.5)
                .min_by(|a, b| a.1.abs().total_cmp(&b.1.abs()));
            if let Some((i, d)) = best {
                self.taps[i] = Some(d);
                if d.abs() < 0.2 {
                    ctx.sounds.push(MiniGameSound::Cue);
                }
            }
        }
        if want && !self.pouring {
            ctx.sounds.push(MiniGameSound::Pour);
        }
        self.pouring = want;
        self.flow = approach(self.flow, if want { 1.0 } else { 0.0 }, dt, 0.1);
        if want {
            let p = self.pour_point;
            self.deposit(p);
            if !keyboard {
                self.add_trace(p, ctx.relaxed);
            }
        } else {
            self.stroke_last = None;
        }

        let complete = self.finished_gesture() && !self.pouring;
        if complete {
            self.complete_at.get_or_insert(ctx.t);
        } else {
            self.complete_at = None;
        }
        let something = !self.trace.is_empty() || self.kb_used;
        let finish = if self.common.relaxed {
            input.confirm
        } else {
            (input.confirm && something)
                || self.complete_at.is_some_and(|c| ctx.t - c > 0.4)
                || ctx.t > MAX_T
        };
        if finish {
            self.done = true;
            self.pouring = false;
            self.flow = 0.0;
            self.cinnamon = self.touch == Some(RegularTouch::Cinnamon);
            ctx.sounds.push(MiniGameSound::Clink);
            return Some(self.quality(ctx.relaxed));
        }
        None
    }

    /// The scripted pour has run its whole timeline (relaxed autoplay presses Done after it).
    pub fn autoplay_finished(&self, t: f32) -> bool {
        t - START_DELAY >= self.timeline_end() || self.kb_t >= self.timeline_end()
    }

    pub fn autoplay(&self, t: f32, keyboard: bool) -> MgInput {
        let lt = t - START_DELAY;
        if keyboard {
            let hold = lt > 0.0 && self.kb_t < self.timeline_end();
            let next = self
                .marks
                .iter()
                .enumerate()
                .find(|(i, _)| self.taps[*i].is_none())
                .map(|(_, m)| m.1);
            let tap = next.is_some_and(|mt| self.kb_t >= mt && self.kb_t - mt < 0.05);
            return MgInput {
                hold,
                left: tap,
                ..Default::default()
            };
        }
        if lt < 0.0 {
            return MgInput {
                pointer: self.strokes.first().map(|s| s[0]),
                ..Default::default()
            };
        }
        let (p, stroke) = self.timeline_at(lt);
        let wobble = vec2((t * 2.1).sin(), (t * 1.7).cos()) * 1.5;
        MgInput {
            pointer: Some(p + wobble),
            pressed: stroke.is_some(),
            down: stroke.is_some(),
            ..Default::default()
        }
    }

    pub fn hint(&self) -> Hint<'static> {
        let something = !self.trace.is_empty() || self.kb_used;
        let done = self.common.relaxed || (something && !self.pouring);
        if self.kb_used {
            Hint {
                key: "minigame.latte_art.prompt.keys",
                emphasis: false,
                keys: &["Space", "←", "→"],
                done,
            }
        } else if something && !self.pouring {
            Hint {
                key: "minigame.latte_art.prompt.done",
                emphasis: false,
                keys: &[],
                done,
            }
        } else {
            Hint {
                key: "minigame.latte_art.prompt.trace",
                emphasis: false,
                keys: &[],
                done,
            }
        }
    }

    pub fn draw(&self, ui: &mut Ui, o: Vec2, t: f32, text: &MiniGameText) {
        let th = ui.theme.clone();
        let c = o + C;
        // saucer
        contact_shadow(c + vec2(6.0, 10.0), SAUCER + 12.0, SAUCER * 0.92, 0.5);
        let sq = |r: f32| Rect::new(c.x - r, c.y - r, r * 2.0, r * 2.0);
        fill_circle(
            c,
            SAUCER,
            Paint::diagonal(sq(SAUCER), hex(0xe9e2f4), hex(0xa89cc4)),
        );
        fill_circle(
            c + vec2(2.0, 3.0),
            SAUCER - 15.0,
            with_alpha(hex(0x8f82ad), 0.35),
        );
        stroke_circle(c, SAUCER - 1.0, 1.2, Color::new(1.0, 1.0, 1.0, 0.35));
        // cup: shadow, handle, body, inner wall
        rd::soft_shadow(c.x + 7.0, c.y + 9.0, CUP_OUT + 8.0, CUP_OUT + 6.0, 0.4);
        let handle = Rect::new(c.x + CUP_OUT - 10.0, c.y - 11.0, 38.0, 22.0);
        fill_rrect(
            handle,
            11.0,
            Paint::vertical(handle, hex(0xf6f1f8), hex(0xc9bfd8)),
        );
        fill_rrect(
            Rect::new(
                handle.x + 15.0,
                handle.y + 7.0,
                handle.w - 23.0,
                handle.h - 14.0,
            ),
            4.0,
            hex(0xb3a7c8),
        );
        fill_circle(
            c,
            CUP_OUT,
            Paint::diagonal(sq(CUP_OUT), hex(0xfcf9fd), hex(0xc4b9d6)),
        );
        fill_circle(
            c,
            CUP_IN + 1.0,
            Paint::diagonal(sq(CUP_IN), hex(0xc9bfd9), hex(0xf4effa)),
        );
        // crema
        rd::radial(
            c.x,
            c.y,
            CREMA_R + 5.0,
            CREMA_R + 5.0,
            hex(0xb07a46),
            hex(0x5e3520),
        );
        rd::glow(
            c.x - 12.0,
            c.y - 14.0,
            46.0,
            Color::new(0.85, 0.58, 0.32, 0.35),
        );
        for i in 0..60u32 {
            let a = rd::hash01(i * 11 + 5) * std::f32::consts::TAU;
            let r = rd::hash01(i * 23 + 9).sqrt() * (CREMA_R - 6.0);
            let p = c + vec2(a.cos(), a.sin()) * r;
            let s = 0.8 + rd::hash01(i * 5 + 2) * 1.4;
            let col = if i % 3 == 0 {
                Color::new(0.95, 0.72, 0.42, 0.25)
            } else {
                Color::new(0.25, 0.12, 0.06, 0.25)
            };
            fill_circle(p, s, col);
        }
        draw::stroke_circle(c, CREMA_R + 4.0, 9.0, hex(0x4e2b19));

        // the faint guide: dotted, the current stroke a little brighter
        let next = self.next_stroke();
        if !self.done || self.common.relaxed {
            for (si, s) in self.strokes.iter().enumerate() {
                let current = next == Some(si) && !self.kb_used;
                let a = if current { 0.62 } else { 0.34 } + 0.08 * (t * 2.2).sin();
                let mut acc = 0.0;
                for w in s.windows(2) {
                    acc += w[0].distance(w[1]);
                    if acc >= 6.0 {
                        acc = 0.0;
                        fill_circle(o + w[1], 1.6, Color::new(1.0, 0.96, 0.9, a));
                    }
                }
            }
            if self.common.practised {
                for (i, g) in self.guide48.iter().enumerate().step_by(6) {
                    let tw = ((t * 1.5 + i as f32 * 0.37).sin() * 0.5 + 0.5).powi(3);
                    super::paint::sparkle(o + *g, 3.0, 1.0 - tw * 0.8, true);
                }
            }
        }

        // milk: halo pass, then cores
        let grow = |b: &Blob| b.r * (0.55 + 0.45 * ease_out((t - b.born) / 0.4));
        for b in &self.blobs {
            fill_circle(o + b.c, grow(b) + 1.6, Color::new(0.83, 0.66, 0.48, 0.9));
        }
        let mc = milk_color(self.milk);
        for b in &self.blobs {
            fill_circle(o + b.c, grow(b), mc);
        }
        if self.cinnamon {
            for i in 0..140u32 {
                let a = rd::hash01(i * 13 + 1) * std::f32::consts::TAU;
                let r = rd::hash01(i * 29 + 4).sqrt() * (CREMA_R - 8.0);
                let p = c + vec2(a.cos(), a.sin()) * r;
                fill_circle(
                    p,
                    0.7 + rd::hash01(i) * 0.8,
                    Color::new(0.55, 0.3, 0.14, 0.55),
                );
            }
        }
        draw::arc(
            c,
            CUP_OUT - 2.0,
            3.5,
            5.3,
            2.0,
            Color::new(1.0, 1.0, 1.0, 0.6),
        );

        if !self.done {
            // wobble marks (keyboard) / start dot of the next stroke (mouse)
            if self.kb_used {
                for (i, (p, _, left)) in self.marks.iter().enumerate() {
                    let p = o + *p;
                    match self.taps[i] {
                        Some(d) => {
                            let good = 1.0 - (d.abs() / 0.35).min(1.0);
                            crate::ui::draw::glow_circle(
                                p,
                                10.0,
                                with_alpha(th.amber, 0.25 + 0.4 * good),
                            );
                        }
                        None => {
                            let ic = if *left {
                                Icon::Back
                            } else {
                                Icon::ChevronRight
                            };
                            fill_circle(p, 6.0, Color::new(0.1, 0.06, 0.16, 0.55));
                            draw_icon(ic, p, 9.0, with_alpha(th.amber, 0.9));
                        }
                    }
                }
            } else if let Some(si) = next
                && !self.pouring
            {
                let p = o + self.strokes[si][0];
                let pulse = 0.5 + 0.5 * (t * 3.0).sin();
                crate::ui::draw::glow_circle(p, 16.0 + pulse * 4.0, with_alpha(th.amber, 0.35));
                fill_circle(p, 4.0, hex(0xfff3d6));
                stroke_circle(p, 8.0 + pulse * 2.0, 1.4, with_alpha(th.amber, 0.8));
            }
            // pour stream + ripples
            let p = o + self.pour_point;
            if self.flow > 0.02 {
                let wob = (t * 9.0).sin() * 1.2;
                let top = vec2(p.x + 14.0 + wob, o.y - 8.0);
                rd::glow(
                    p.x,
                    p.y,
                    14.0,
                    Color::new(1.0, 0.98, 0.94, 0.25 * self.flow),
                );
                rd::capsule(
                    top,
                    p,
                    2.6 * self.flow + 0.5,
                    Color::new(0.98, 0.95, 0.9, 0.85 * self.flow),
                );
                for k in 0..2 {
                    let ph = ((t * 1.8) + k as f32 * 0.5).fract();
                    stroke_circle(
                        p,
                        5.0 + ph * 16.0,
                        1.1,
                        Color::new(1.0, 0.96, 0.9, 0.35 * (1.0 - ph) * self.flow),
                    );
                }
            } else if self.pointer.is_some_and(|q| q.distance(C) < CUP_IN) {
                stroke_circle(p, 8.0, 1.4, Color::new(0.85, 0.84, 1.0, 0.45));
            }
        }
        self.draw_panel(ui, o, t, text);
    }

    fn draw_panel(&self, ui: &mut Ui, o: Vec2, t: f32, text: &MiniGameText) {
        let th = ui.theme.clone();
        let r = PANEL.offset(o);
        draw_glass(&th, r, &Glass::card());
        let micro = th.micro();
        ui.fonts.draw(
            &text.get("minigame.target").to_uppercase(),
            r.x + 12.0,
            r.y + 20.0,
            &micro,
        );
        let key = self.shape.key();
        let st = ui.fonts.fit_width(text.get(&key), r.w - 24.0, &th.strong());
        let name = ui
            .fonts
            .ellipsize(text.get(&key), r.w - 24.0, &st)
            .into_owned();
        ui.fonts.draw(&name, r.x + 12.0, r.y + 38.0, &st);
        // mini cup with the pattern as a milk line
        let mc = vec2(r.center().x, r.y + 104.0);
        let mr = 46.0;
        crate::ui::draw::glow_circle(
            mc,
            mr + 16.0,
            Color::new(1.0, 0.75, 0.45, 0.08 + 0.03 * (t * 2.0).sin()),
        );
        let sq = Rect::new(mc.x - mr, mc.y - mr, mr * 2.0, mr * 2.0);
        fill_circle(
            mc,
            mr + 6.0,
            Paint::diagonal(sq, hex(0xfaf6fc), hex(0xbfb3d3)),
        );
        fill_circle(mc, mr, Paint::diagonal(sq, hex(0xb27a45), hex(0x5a321e)));
        for s in self.strokes.iter() {
            let pts: Vec<Vec2> = s.iter().map(|p| mc + (*p - C) * (mr / CREMA_R)).collect();
            draw::stroke_path(&pts, false, 3.2, Paint::Solid(hex(0xf6efe3)));
        }
        // progress
        let bar = Rect::new(r.x + 12.0, r.bottom() - 30.0, r.w - 24.0, 8.0);
        let prog = if self.kb_used {
            (self.kb_t / self.timeline_end().max(1e-3)).min(1.0)
        } else {
            let total: usize = self.covered.iter().map(Vec::len).sum();
            let done: usize = self
                .covered
                .iter()
                .map(|c| c.iter().filter(|x| **x).count())
                .sum();
            done as f32 / total.max(1) as f32
        };
        fill_rrect(bar, 4.0, Color::new(0.02, 0.01, 0.07, 0.55));
        let fw = (bar.w - 2.0) * prog;
        if fw > 1.0 {
            let f = Rect::new(bar.x + 1.0, bar.y + 1.0, fw, bar.h - 2.0);
            fill_rrect(f, 3.0, Paint::horizontal(f, hex(0xd9c6ad), hex(0xfffaf3)));
        }
    }
}
