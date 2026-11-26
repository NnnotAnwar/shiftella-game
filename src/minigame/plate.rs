//! Plate (§2.5): a plate on a wooden board, 3–5 components beside it. "Lida's way" shows
//! the layout as a ghost for 1.5 s (Relaxed: it stays). Drag the components onto the plate
//! (they snap to the nearest free slot when close), then click the garnish dots. Keyboard:
//! Tab cycles the pieces, arrows move, Space drops (snaps, scored 0.85) / adds garnish.
//! Quality: `0.7 × mean(slot_score) + 0.3 × garnish`.

use macroquad::prelude::*;

use super::paint::{contact_shadow, hex, with_alpha};
use super::score;
use super::{Common, Ctx, Hint, MgInput, MiniGameSound, PlateSet, RegularTouch, Rng};
use crate::render::draw as rd;
use crate::ui::draw::{self, Paint, fill_circle, fill_convex, fill_rrect, stroke_circle};
use crate::ui::{Ui, approach, ease};

const C: Vec2 = Vec2::new(168.0, 124.0);
const PLATE_R: f32 = 110.0;
const WELL_R: f32 = 80.0;
const TRAY: Rect = Rect {
    x: 346.0,
    y: 10.0,
    w: 132.0,
    h: 228.0,
};
const SNAP: f32 = 26.0;
const GHOST_T: f32 = 1.5;
const MAX_T: f32 = 20.0;
const KB_SCORE: f32 = 0.85;

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
    Egg,
    Toast,
    Garlic,
    Cheese,
    Pickle,
}

impl PieceKind {
    fn radius(self) -> f32 {
        match self {
            PieceKind::Slice | PieceKind::Toast => 32.0,
            PieceKind::Cream | PieceKind::Honey | PieceKind::Croutons | PieceKind::Egg => 19.0,
            _ => 17.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GarnishKind {
    SugarDust,
    DillFlecks,
    Onion,
}

struct Piece {
    kind: PieceKind,
    home: Vec2,
    pos: Vec2,
    goal: Vec2,
    on_plate: bool,
    kb_dropped: bool,
    lift: f32,
    bounce: f32,
    rot: f32,
}

struct Garnish {
    at: Vec2,
    done: Option<f32>,
}

pub(crate) struct Plate {
    set: PlateSet,
    common: Common,
    pieces: Vec<Piece>,
    slots: Vec<Vec2>,
    garnish_kind: GarnishKind,
    garnish: Vec<Garnish>,
    dragging: Option<usize>,
    grab: Vec2,
    selected: Option<usize>,
    idle_since: f32,
    t: f32,
}

impl Plate {
    pub fn new(rng: &mut Rng, set: PlateSet, touch: Option<RegularTouch>, common: Common) -> Plate {
        let (mut spec, garnish_kind, dots): (Vec<(PieceKind, Vec2)>, GarnishKind, Vec<Vec2>) =
            match set {
                PlateSet::Medovnik => (
                    vec![
                        (PieceKind::Slice, C + vec2(-10.0, 8.0)),
                        (PieceKind::Cream, C + vec2(52.0, -26.0)),
                        (PieceKind::Berries, C + vec2(-56.0, -32.0)),
                        (PieceKind::Mint, C + vec2(48.0, 40.0)),
                    ],
                    GarnishKind::SugarDust,
                    vec![
                        C + vec2(-58.0, 38.0),
                        C + vec2(8.0, -54.0),
                        C + vec2(66.0, 6.0),
                    ],
                ),
                PlateSet::Soup => (
                    vec![
                        (PieceKind::Swirl, C + vec2(-14.0, 6.0)),
                        (PieceKind::Egg, C + vec2(26.0, -18.0)),
                        (PieceKind::Croutons, C + vec2(30.0, 30.0)),
                        (PieceKind::Dill, C + vec2(-38.0, -30.0)),
                    ],
                    GarnishKind::DillFlecks,
                    vec![
                        C + vec2(-50.0, 8.0),
                        C + vec2(-30.0, 40.0),
                        C + vec2(4.0, -46.0),
                    ],
                ),
                PlateSet::Topinka => (
                    vec![
                        (PieceKind::Toast, C + vec2(-8.0, 6.0)),
                        (PieceKind::Garlic, C + vec2(-58.0, -34.0)),
                        (PieceKind::Cheese, C + vec2(-6.0, 2.0) + vec2(0.0, -4.0)),
                        (PieceKind::Pickle, C + vec2(58.0, 28.0)),
                    ],
                    GarnishKind::Onion,
                    vec![
                        C + vec2(54.0, -34.0),
                        C + vec2(-52.0, 40.0),
                        C + vec2(10.0, 56.0),
                    ],
                ),
            };
        // the cheese goes on the toast: give it its own spot slightly up
        if set == PlateSet::Topinka {
            spec[2].1 = C + vec2(-6.0, -6.0);
            spec[0].1 = C + vec2(-8.0, 12.0);
        }
        if touch == Some(RegularTouch::Honey) && set == PlateSet::Medovnik {
            spec.push((PieceKind::Honey, C + vec2(-46.0, 52.0)));
        }
        let n = spec.len();
        let slots: Vec<Vec2> = spec
            .iter()
            .map(|(_, p)| *p + vec2(rng.range(-4.0, 4.0), rng.range(-4.0, 4.0)))
            .collect();
        let pieces = spec
            .into_iter()
            .enumerate()
            .map(|(i, (kind, _))| {
                let gap = TRAY.h / n as f32;
                let home = vec2(TRAY.center().x, TRAY.y + gap * (i as f32 + 0.5));
                Piece {
                    kind,
                    home,
                    pos: home,
                    goal: home,
                    on_plate: false,
                    kb_dropped: false,
                    lift: 0.0,
                    bounce: 0.0,
                    rot: rng.range(-0.25, 0.25),
                }
            })
            .collect();
        Plate {
            set,
            common,
            pieces,
            slots,
            garnish_kind,
            garnish: dots
                .into_iter()
                .map(|at| Garnish { at, done: None })
                .collect(),
            dragging: None,
            grab: Vec2::ZERO,
            selected: None,
            idle_since: 0.0,
            t: 0.0,
        }
    }

    pub fn focus(&self) -> Vec2 {
        C
    }

    fn piece_at(&self, p: Vec2) -> Option<usize> {
        let mut best: Option<(usize, f32)> = None;
        for (i, pc) in self.pieces.iter().enumerate() {
            let d = pc.pos.distance(p);
            if d < pc.kind.radius() + 6.0 && best.is_none_or(|(_, bd)| d < bd) {
                best = Some((i, d));
            }
        }
        best.map(|b| b.0)
    }

    /// Nearest slot not taken by another placed piece.
    fn nearest_free_slot(&self, me: usize, at: Vec2) -> Option<Vec2> {
        self.slots
            .iter()
            .filter(|s| {
                !self
                    .pieces
                    .iter()
                    .enumerate()
                    .any(|(j, p)| j != me && p.on_plate && p.goal.distance(**s) < 3.0)
            })
            .min_by(|a, b| a.distance(at).total_cmp(&b.distance(at)))
            .copied()
    }

    fn place(&mut self, i: usize, at: Vec2, ctx: &mut Ctx) {
        let on = at.distance(C) < PLATE_R - self.pieces[i].kind.radius() * 0.4;
        let snap = self
            .nearest_free_slot(i, at)
            .filter(|s| s.distance(at) < SNAP * score::widen(ctx.relaxed));
        let pc = &mut self.pieces[i];
        if on {
            pc.goal = snap.unwrap_or(at);
            pc.on_plate = true;
            pc.bounce = 1.0;
            ctx.sounds.push(MiniGameSound::Clink);
        } else {
            pc.goal = pc.home;
            pc.on_plate = false;
            pc.kb_dropped = false;
        }
    }

    fn all_placed(&self) -> bool {
        self.pieces.iter().all(|p| p.on_plate)
    }

    fn garnish_done(&self) -> usize {
        self.garnish.iter().filter(|g| g.done.is_some()).count()
    }

    pub fn finished_gesture(&self) -> bool {
        self.all_placed() && self.garnish_done() == self.garnish.len() && self.dragging.is_none()
    }

    fn quality(&self, relaxed: bool) -> f32 {
        let placed: Vec<Option<Vec2>> = self
            .pieces
            .iter()
            .map(|p| p.on_plate.then_some(p.goal))
            .collect();
        let mut slots = score::plate_slots(&placed, &self.slots, relaxed);
        for (s, p) in slots.iter_mut().zip(&self.pieces) {
            if p.on_plate && p.kb_dropped {
                *s = KB_SCORE;
            }
        }
        score::plate(&slots, self.garnish_done(), self.garnish.len())
    }

    pub fn step(&mut self, ctx: &mut Ctx, input: &MgInput, dt: f32) -> Option<f32> {
        self.t = ctx.t;
        if let Some(p) = input.pointer {
            if input.pressed && self.dragging.is_none() {
                if let Some(i) = self.piece_at(p) {
                    self.dragging = Some(i);
                    self.selected = None;
                    self.grab = self.pieces[i].pos - p;
                    self.pieces[i].on_plate = false;
                    self.pieces[i].kb_dropped = false;
                    self.idle_since = ctx.t;
                } else if let Some(g) = self.garnish.iter_mut().find(|g| {
                    g.done.is_none() && g.at.distance(p) < 16.0 * score::widen(ctx.relaxed)
                }) {
                    g.done = Some(ctx.t);
                    ctx.sounds.push(MiniGameSound::Cue);
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

        // keyboard: Tab picks the next piece, arrows move it, Space drops / garnishes
        if input.cycle && self.dragging.is_none() {
            let start = self.selected.map_or(0, |s| s + 1);
            let n = self.pieces.len();
            self.selected = (0..n)
                .map(|k| (start + k) % n)
                .find(|i| !self.pieces[*i].on_plate);
            if let Some(i) = self.selected {
                self.pieces[i].goal = C;
            }
            self.idle_since = ctx.t;
        }
        if let Some(i) = self.selected
            && input.axis != Vec2::ZERO
        {
            let g = self.pieces[i].goal + input.axis.normalize_or_zero() * 120.0 * dt;
            let off = g - C;
            self.pieces[i].goal = C + off.clamp_length_max(PLATE_R - 20.0);
        }
        if input.hold_pressed && self.dragging.is_none() {
            if self.selected.is_none() {
                self.selected = self.pieces.iter().position(|p| !p.on_plate);
                if let Some(i) = self.selected {
                    self.pieces[i].goal = C;
                }
            }
            if let Some(i) = self.selected.take() {
                let at = self.pieces[i].goal;
                if let Some(s) = self.nearest_free_slot(i, at) {
                    let pc = &mut self.pieces[i];
                    pc.goal = s;
                    pc.on_plate = true;
                    pc.kb_dropped = true;
                    pc.bounce = 1.0;
                    ctx.sounds.push(MiniGameSound::Clink);
                }
            } else if let Some(g) = self.garnish.iter_mut().find(|g| g.done.is_none()) {
                g.done = Some(ctx.t);
                ctx.sounds.push(MiniGameSound::Cue);
            }
            self.idle_since = ctx.t;
        }

        for (i, pc) in self.pieces.iter_mut().enumerate() {
            let held = self.dragging == Some(i) || self.selected == Some(i);
            let tau = if held { 0.035 } else { 0.09 };
            pc.pos += (pc.goal - pc.pos) * (1.0 - (-dt / tau).exp());
            pc.lift = approach(pc.lift, if held { 1.0 } else { 0.0 }, dt, 0.06);
            pc.bounce = (pc.bounce - dt * 2.6).max(0.0);
        }
        let finish = if self.common.relaxed {
            input.confirm
        } else {
            let done = self.finished_gesture() && ctx.t - self.idle_since > 1.2;
            done || ctx.t > MAX_T || (input.confirm && self.all_placed())
        };
        if finish {
            ctx.sounds.push(MiniGameSound::Clink);
            return Some(self.quality(ctx.relaxed));
        }
        None
    }

    pub fn autoplay(&self, t: f32, keyboard: bool) -> MgInput {
        let lt = t - GHOST_T - 0.2;
        if lt < 0.0 {
            return MgInput::default();
        }
        if keyboard {
            // one action every 0.6 s: Tab, Space per piece, then Space per garnish
            let step = (lt / 0.6) as usize;
            let frac = lt / 0.6 - step as f32;
            let fire = frac < 0.03;
            let n = self.pieces.len();
            let mut i = MgInput::default();
            if fire {
                if step < n * 2 {
                    if step.is_multiple_of(2) {
                        i.cycle = true;
                    } else {
                        i.hold_pressed = true;
                    }
                } else if step < n * 2 + self.garnish.len() {
                    i.hold_pressed = true;
                }
            }
            return i;
        }
        let per = 1.3;
        let idx = (lt / per) as usize;
        let k = lt - idx as f32 * per;
        if let Some(pc) = self.pieces.get(idx) {
            let target = self.slots[idx];
            return if k < 0.2 {
                MgInput {
                    pointer: Some(pc.home),
                    ..Default::default()
                }
            } else if k < 1.0 {
                MgInput {
                    pointer: Some(pc.home.lerp(target, ease((k - 0.2) / 0.75))),
                    pressed: true,
                    down: true,
                    ..Default::default()
                }
            } else {
                MgInput {
                    pointer: Some(target),
                    released: k < 1.03,
                    ..Default::default()
                }
            };
        }
        let gt = lt - self.pieces.len() as f32 * per;
        let gi = (gt / 0.45) as usize;
        match self.garnish.get(gi) {
            Some(g) => MgInput {
                pointer: Some(g.at),
                pressed: gt - gi as f32 * 0.45 < 0.03,
                ..Default::default()
            },
            None => MgInput::default(),
        }
    }

    pub fn hint(&self) -> Hint<'static> {
        let done = self.common.relaxed || self.all_placed();
        let key = if self.t < GHOST_T {
            "minigame.plate.prompt.ghost"
        } else if !self.all_placed() {
            "minigame.plate.prompt.drag"
        } else if self.garnish_done() < self.garnish.len() {
            "minigame.plate.prompt.garnish"
        } else {
            "minigame.plate.prompt.done"
        };
        Hint {
            key,
            emphasis: false,
            keys: if self.all_placed() {
                &["Space"]
            } else {
                &["Tab", "Space"]
            },
            done,
        }
    }

    pub fn draw(&self, _ui: &mut Ui, o: Vec2, t: f32) {
        let c = o + C;
        // board under the plate + the tray
        let tray = TRAY.offset(o);
        contact_shadow(
            vec2(tray.center().x + 4.0, tray.bottom() - 2.0),
            tray.w * 0.6,
            12.0,
            0.5,
        );
        fill_rrect(
            Rect::new(tray.x, tray.y + 4.0, tray.w, tray.h),
            14.0,
            hex(0x3e2419),
        );
        fill_rrect(
            tray,
            14.0,
            Paint::vertical(tray, hex(0x7a5038), hex(0x543424)),
        );
        for i in 0..7u32 {
            let y = tray.y + 14.0 + i as f32 * 32.0 + rd::hash01(i * 9) * 8.0;
            draw::hline(
                tray.x + 10.0,
                tray.right() - 10.0,
                y,
                Color::new(0.25, 0.12, 0.06, 0.18),
            );
        }
        draw::stroke_rrect(tray, 14.0, 1.0, Color::new(1.0, 0.85, 0.65, 0.16));
        for pc in &self.pieces {
            let h = o + pc.home;
            rd::radial(
                h.x,
                h.y + 2.0,
                pc.kind.radius() + 6.0,
                pc.kind.radius() * 0.75,
                Color::new(0.2, 0.1, 0.05, 0.28),
                Color::new(0.2, 0.1, 0.05, 0.0),
            );
        }

        // plate
        contact_shadow(c + vec2(6.0, 12.0), PLATE_R + 14.0, PLATE_R * 0.95, 0.55);
        let sq = |r: f32| Rect::new(c.x - r, c.y - r, r * 2.0, r * 2.0);
        fill_circle(
            c,
            PLATE_R,
            Paint::diagonal(sq(PLATE_R), hex(0xfbf8fe), hex(0xb5a8cf)),
        );
        stroke_circle(c, PLATE_R - 1.0, 1.2, Color::new(1.0, 1.0, 1.0, 0.45));
        fill_circle(
            c,
            WELL_R + 2.0,
            Paint::diagonal(sq(WELL_R), hex(0xcfc4e2), hex(0xf6f2fb)),
        );
        stroke_circle(c, PLATE_R - 11.0, 1.4, with_alpha(hex(0x8b7fb8), 0.35));
        if self.set == PlateSet::Soup {
            rd::soft_shadow(c.x + 5.0, c.y + 7.0, 78.0, 74.0, 0.35);
            fill_circle(
                c,
                76.0,
                Paint::diagonal(sq(76.0), hex(0xfdfbff), hex(0xbdb1d6)),
            );
            fill_circle(
                c,
                65.0,
                Paint::diagonal(sq(65.0), hex(0xbfb4d4), hex(0xece6f5)),
            );
            rd::radial(c.x, c.y, 60.0, 60.0, hex(0xf1e4b8), hex(0xd8c48a));
            for i in 0..26u32 {
                let a = rd::hash01(i * 7 + 2) * std::f32::consts::TAU;
                let r = rd::hash01(i * 13 + 5).sqrt() * 52.0;
                let p = c + vec2(a.cos(), a.sin()) * r;
                let col = if i % 4 == 0 {
                    Color::new(0.98, 0.95, 0.85, 0.9)
                } else {
                    Color::new(0.85, 0.7, 0.36, 0.45)
                };
                fill_circle(p, 1.0 + rd::hash01(i) * 1.4, col);
            }
            draw::arc(c, 63.0, 3.6, 5.2, 2.0, Color::new(1.0, 1.0, 1.0, 0.6));
        } else {
            draw::arc(
                c,
                PLATE_R - 3.0,
                3.5,
                5.2,
                2.0,
                Color::new(1.0, 1.0, 1.0, 0.7),
            );
        }

        // "Lida's way": the ghost layout fades after 1.5 s (Relaxed: stays faint)
        let ghost = if self.common.relaxed {
            0.18_f32.max(1.0 - (t / GHOST_T).powi(2))
        } else {
            (1.0 - ((t - GHOST_T + 0.4) / 0.4).clamp(0.0, 1.0)) * ease(t / 0.3)
        };
        let dragging = self.dragging.or(self.selected).map(|i| &self.pieces[i]);
        for (i, s) in self.slots.iter().enumerate() {
            let taken = self
                .pieces
                .iter()
                .any(|p| p.on_plate && p.goal.distance(*s) < 3.0);
            if taken {
                continue;
            }
            let near = dragging
                .map(|d| 1.0 - ((d.pos.distance(*s) - 10.0) / 50.0).clamp(0.0, 1.0))
                .unwrap_or(0.0);
            let tp = o + *s;
            let pulse = 0.5 + 0.5 * (t * 2.4 + i as f32).sin();
            let r = self.pieces[i].kind.radius() * 0.85 + pulse * 2.0;
            crate::ui::draw::glow_circle(
                tp,
                r + 14.0,
                Color::new(0.65, 0.6, 1.0, 0.08 + 0.25 * near),
            );
            let segs = 14;
            for k in 0..segs {
                let a0 = k as f32 / segs as f32 * std::f32::consts::TAU + t * 0.3;
                draw::arc(
                    tp,
                    r,
                    a0,
                    a0 + 0.24,
                    1.6,
                    Color::new(0.55, 0.5, 0.95, 0.3 + 0.4 * near),
                );
            }
            if ghost > 0.01 {
                draw_piece(
                    self.pieces[i].kind,
                    tp,
                    0.95,
                    self.pieces[i].rot,
                    0.45 * ghost,
                    t,
                );
            }
            if self.common.practised {
                let tw = ((t * 1.7 + i as f32).sin() * 0.5 + 0.5).powi(2);
                super::paint::sparkle(tp + vec2(r, -r) * 0.7, 3.0, 1.0 - tw * 0.8, true);
            }
        }
        // garnish dots
        for g in &self.garnish {
            let p = o + g.at;
            match g.done {
                None => {
                    let pulse = 0.5 + 0.5 * (t * 3.0 + g.at.x).sin();
                    crate::ui::draw::glow_circle(
                        p,
                        12.0,
                        Color::new(1.0, 0.82, 0.5, 0.18 + 0.15 * pulse),
                    );
                    fill_circle(p, 3.0, Color::new(1.0, 0.9, 0.7, 0.75));
                }
                Some(at) => draw_garnish(self.garnish_kind, p, ease((t - at) / 0.3)),
            }
        }

        // pieces: placed ones first, the held one last
        let mut order: Vec<usize> = (0..self.pieces.len()).collect();
        order.sort_by_key(|i| {
            let held = self.dragging == Some(*i) || self.selected == Some(*i);
            let base = matches!(
                self.pieces[*i].kind,
                PieceKind::Slice | PieceKind::Toast | PieceKind::Swirl
            );
            (held, !base, (self.pieces[*i].pos.y * 10.0) as i32)
        });
        for i in order {
            let pc = &self.pieces[i];
            let p = o + pc.pos;
            let lift = pc.lift;
            let b = pc.bounce;
            let s = 1.0 + 0.1 * lift + 0.12 * (b * std::f32::consts::PI).sin() * b;
            rd::soft_shadow(
                p.x + 2.0 + lift * 6.0,
                p.y + 4.0 + lift * 10.0,
                pc.kind.radius() * (1.0 + lift * 0.2),
                pc.kind.radius() * 0.6,
                0.35 - lift * 0.12,
            );
            draw_piece(pc.kind, p - vec2(0.0, lift * 8.0), s, pc.rot, 1.0, t);
            if self.selected == Some(i) {
                stroke_circle(
                    p - vec2(0.0, lift * 8.0),
                    pc.kind.radius() + 6.0,
                    1.4,
                    Color::new(1.0, 0.85, 0.55, 0.8),
                );
            }
        }
    }
}

fn draw_garnish(kind: GarnishKind, p: Vec2, k: f32) {
    for i in 0..9u32 {
        let a0 = rd::hash01(i * 7 + 1) * std::f32::consts::TAU;
        let r = rd::hash01(i * 3 + 2).sqrt() * 9.0 * (0.4 + 0.6 * k);
        let q = p + vec2(a0.cos(), a0.sin()) * r;
        match kind {
            GarnishKind::SugarDust => fill_circle(q, 1.1, Color::new(1.0, 1.0, 1.0, 0.9 * k)),
            GarnishKind::DillFlecks => {
                let d = vec2(a0.sin(), a0.cos()) * 2.2;
                draw::line(q - d, q + d, 1.0, Color::new(0.33, 0.6, 0.35, 0.9 * k));
            }
            GarnishKind::Onion => {
                if i % 3 == 0 {
                    stroke_circle(q, 2.6, 1.0, Color::new(0.98, 0.95, 0.9, 0.9 * k));
                }
            }
        }
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
                let col = if i % 2 == 0 {
                    hex(0xe8cc96)
                } else {
                    hex(0xa45f2c)
                };
                fill_convex(
                    &[
                        q(tip + vec2(0.0, y0)),
                        q(b1 + vec2(0.0, y0)),
                        q(b1 + vec2(0.0, y1)),
                        q(tip + vec2(0.0, y1)),
                    ],
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
                fill_circle(
                    q(pt),
                    1.4 * s,
                    a(
                        if i % 3 == 0 {
                            hex(0xe9c48a)
                        } else {
                            hex(0x7f4a22)
                        },
                        k,
                    ),
                );
            }
            draw::line(q(tip), q(b1), 1.0, a(Color::new(1.0, 0.92, 0.75, 0.5), k));
        }
        PieceKind::Cream => {
            for (i, (rr, dy)) in [(19.0, 0.0), (15.0, -5.0), (10.5, -9.5), (6.0, -13.0)]
                .iter()
                .enumerate()
            {
                let c0 = q(vec2(0.0, *dy));
                let col = draw::mix(hex(0xe9e1d6), hex(0xfffdf8), i as f32 / 3.0);
                fill_circle(c0, rr * s, a(col, k));
            }
            let c0 = q(vec2(0.0, 0.0));
            for i in 0..3 {
                let rr = (7.0 + i as f32 * 4.5) * s;
                draw::arc(
                    c0 + vec2(0.0, -(9.0 - i as f32 * 4.0) * s),
                    rr,
                    3.4 + r,
                    5.0 + r,
                    1.2,
                    a(Color::new(1.0, 1.0, 1.0, 0.8), k),
                );
            }
            fill_circle(q(vec2(-1.0, -16.0)), 2.2 * s, a(WHITE, k));
        }
        PieceKind::Berries => {
            let leaf_pts: Vec<Vec2> = leaf(vec2(2.0, -4.0), vec2(16.0, -16.0), 5.0)
                .into_iter()
                .map(q)
                .collect();
            fill_convex(&leaf_pts, Paint::Solid(a(hex(0x4f9a6b), k)));
            for (c0, rr, col) in [
                (vec2(-8.0, 2.0), 8.5, hex(0x4a3f8f)),
                (vec2(7.0, 6.0), 8.0, hex(0x3d3478)),
            ] {
                let cc = q(c0);
                fill_circle(cc, rr * s, a(col, k));
                fill_circle(
                    cc + vec2(-2.5, -2.5) * s,
                    rr * 0.45 * s,
                    a(Color::new(0.75, 0.72, 0.95, 0.45), k),
                );
                fill_circle(cc + vec2(1.5, 1.5) * s, 1.4 * s, a(hex(0x231c45), k));
            }
            // raspberry
            let rc = q(vec2(-1.0, -9.0));
            fill_circle(rc, 8.0 * s, a(hex(0xc2304f), k));
            for i in 0..7 {
                let ang = i as f32 / 7.0 * std::f32::consts::TAU;
                fill_circle(
                    rc + vec2(ang.cos(), ang.sin()) * 4.6 * s,
                    2.6 * s,
                    a(hex(0xe0566f), k),
                );
            }
            fill_circle(
                rc + vec2(-2.0, -2.5) * s,
                1.6 * s,
                a(Color::new(1.0, 0.85, 0.9, 0.7), k),
            );
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
                draw::line(
                    q(base),
                    q(base + (rot(tip, sway) - base) * 0.85),
                    0.9,
                    a(hex(0x9ed8a8), k),
                );
            }
            draw::line(
                q(vec2(0.0, 8.0)),
                q(vec2(2.0, 15.0)),
                1.6,
                a(hex(0x2f6a4f), k),
            );
        }
        PieceKind::Honey => {
            let mut pts = Vec::new();
            for i in 0..=24 {
                let u = i as f32 / 24.0;
                pts.push(q(vec2(-18.0 + u * 36.0, (u * 18.0).sin() * 9.0)));
            }
            draw::stroke_path(&pts, false, 3.4 * s, Paint::Solid(a(hex(0xe39a2c), k)));
            let hl: Vec<Vec2> = pts.iter().map(|v| *v + vec2(-0.6, -0.8)).collect();
            draw::stroke_path(
                &hl,
                false,
                1.0,
                Paint::Solid(a(Color::new(1.0, 0.92, 0.6, 0.8), k)),
            );
            fill_circle(q(vec2(18.0, 0.0)), 3.6 * s, a(hex(0xf0b043), k));
        }
        PieceKind::Croutons => {
            for (i, (c0, ang)) in [
                (vec2(-8.0, -5.0), 0.3),
                (vec2(7.0, -2.0), -0.4),
                (vec2(-1.0, 9.0), 0.9),
                (vec2(11.0, 11.0), 0.1),
            ]
            .iter()
            .enumerate()
            {
                let sz = if i == 3 { 5.0 } else { 6.5 };
                let corners: Vec<Vec2> = [
                    vec2(-1.0, -1.0),
                    vec2(1.0, -1.0),
                    vec2(1.0, 1.0),
                    vec2(-1.0, 1.0),
                ]
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
        PieceKind::Toast => {
            let r0 = [
                vec2(-30.0, -20.0),
                vec2(30.0, -22.0),
                vec2(32.0, 20.0),
                vec2(-31.0, 22.0),
            ];
            let crust: Vec<Vec2> = r0.iter().map(|v| q(*v)).collect();
            fill_convex(&crust, Paint::Solid(a(hex(0xa8642e), k)));
            let inner: Vec<Vec2> = r0.iter().map(|v| q(*v * 0.84)).collect();
            fill_convex(&inner, Paint::Solid(a(hex(0xe0a95c), k)));
            for i in 0..16u32 {
                let p = vec2(
                    rd::hash01(i * 3) * 44.0 - 22.0,
                    rd::hash01(i * 5 + 1) * 30.0 - 15.0,
                );
                fill_circle(q(p), 1.3 * s, a(Color::new(0.65, 0.38, 0.16, 0.6), k));
            }
            draw::line(
                q(vec2(-24.0, -14.0)),
                q(vec2(20.0, -16.0)),
                1.2,
                a(Color::new(1.0, 0.9, 0.7, 0.5), k),
            );
        }
        PieceKind::Garlic => {
            for (c0, ang) in [(vec2(-6.0, 2.0), -0.5), (vec2(6.0, 0.0), 0.4)] {
                let pts: Vec<Vec2> = leaf(
                    c0 + rot(vec2(0.0, 9.0), ang),
                    c0 + rot(vec2(0.0, -9.0), ang),
                    6.0,
                )
                .into_iter()
                .map(q)
                .collect();
                fill_convex(&pts, Paint::Solid(a(hex(0xf6efe0), k)));
                draw::line(
                    q(c0 + rot(vec2(0.0, 6.0), ang)),
                    q(c0 + rot(vec2(0.0, -6.0), ang)),
                    0.8,
                    a(hex(0xd9cdb2), k),
                );
            }
        }
        PieceKind::Cheese => {
            for i in 0..14u32 {
                let p = vec2(
                    rd::hash01(i * 7) * 30.0 - 15.0,
                    rd::hash01(i * 11 + 3) * 22.0 - 11.0,
                );
                let ang = rd::hash01(i * 13) * std::f32::consts::PI;
                let d = rot(vec2(5.0, 0.0), ang);
                draw::line(q(p - d), q(p + d), 2.4 * s, a(hex(0xf4c64e), k));
            }
        }
        PieceKind::Pickle => {
            for c0 in [vec2(-8.0, -4.0), vec2(7.0, -6.0), vec2(0.0, 8.0)] {
                fill_circle(q(c0), 8.0 * s, a(hex(0x5f8f3a), k));
                fill_circle(q(c0), 6.2 * s, a(hex(0xb8cf6a), k));
                for j in 0..5 {
                    let ang = j as f32 / 5.0 * std::f32::consts::TAU;
                    fill_circle(
                        q(c0 + vec2(ang.cos(), ang.sin()) * 3.2),
                        0.9 * s,
                        a(hex(0xe9f0c2), k),
                    );
                }
            }
        }
        PieceKind::Egg => {
            let pts: Vec<Vec2> = (0..24)
                .map(|i| {
                    let ang = i as f32 / 24.0 * std::f32::consts::TAU;
                    q(vec2(ang.cos() * 16.0, ang.sin() * 12.0))
                })
                .collect();
            fill_convex(&pts, Paint::Solid(a(hex(0xfdfbf4), k)));
            fill_circle(q(vec2(1.0, 0.0)), 7.0 * s, a(hex(0xf7b52c), k));
            fill_circle(
                q(vec2(-1.0, -2.0)),
                2.2 * s,
                a(Color::new(1.0, 0.92, 0.7, 0.8), k),
            );
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
                    draw::line(
                        q(base.lerp(tip, 0.5)),
                        q(base.lerp(tip, 0.5) + vec2(side * 2.0, -3.0)),
                        0.8,
                        a(hex(0x7cc47d), k),
                    );
                }
            }
        }
    }
}
