//! SoupStir: a pot from above. Hold the mouse in the pot and stir in slow circles with the
//! guide light (keyboard: hold Space and the ladle circles by itself). The soup cooks while you
//! stir; when the pot glows, add the next ingredient (click its chip, or press ↑). Česnečka:
//! garlic, then croutons; kulajda: mushrooms, dill, then a cream swirl; borscht: beetroot, then
//! smetana. The cue points and the stirring direction change per play.
//! Quality: `0.6 × time stirring in the calm band + 0.4 × mean(add timing)`.

use std::f32::consts::{PI, TAU};

use macroquad::prelude::*;

use super::paint::{contact_shadow, hex, with_alpha};
use super::score::{self, widen};
use super::{Common, Ctx, Hint, MgInput, MiniGameSound, MiniGameText, Rng, SoupSet};
use crate::render::draw as rd;
use crate::ui::draw::{self, Paint, fill_circle, glow_circle, stroke_circle};
use crate::ui::{Align, Glass, Ui, approach, draw_glass, ease_out};

const C: Vec2 = Vec2::new(176.0, 126.0);
const POT_R: f32 = 108.0;
const BROTH_R: f32 = 92.0;
const RING: f32 = 52.0;
/// Seconds of stirring to cook the soup.
const COOK_S: f32 = 9.5;
/// Simmering on its own (an idle player still gets soup).
const SIMMER_S: f32 = 30.0;
/// The calm stirring band, revolutions per second.
const BAND: (f32, f32) = (0.35, 0.95);
const GUIDE_REV: f32 = 0.62;
const MAX_T: f32 = 36.0;
const TRAY_X: f32 = 404.0;
const ADD_AUTO_S: f32 = 4.0;

struct Add {
    id: &'static str,
    /// Cook progress at which the pot glows for it.
    mark: f32,
    cue_at: Option<f32>,
    done: Option<(f32, f32)>,
}

pub(crate) struct Soup {
    set: SoupSet,
    common: Common,
    adds: Vec<Add>,
    cook: f32,
    point: Vec2,
    mouse_hold: bool,
    stirring: bool,
    stir_t: f32,
    band_t: f32,
    cook_t: f32,
    prev_ang: Option<f32>,
    omega: f32,
    kb_ang: f32,
    dir: f32,
    phase0: f32,
    wiggle: Option<f32>,
    finished_at: Option<f32>,
    t: f32,
    rng: Rng,
    bubbles: Vec<(Vec2, f32)>,
    bubble_acc: f32,
}

fn wrap(a: f32) -> f32 {
    let mut a = a % TAU;
    if a > PI {
        a -= TAU;
    }
    if a < -PI {
        a += TAU;
    }
    a
}

/// `minigame.soup_stir.add.<id>`
pub(crate) fn add_key(id: &str) -> String {
    format!("minigame.soup_stir.add.{id}")
}

impl Soup {
    pub fn new(rng: &mut Rng, set: SoupSet, common: Common) -> Soup {
        let mut ids: Vec<&'static str> = set.adds().to_vec();
        if set.finish_tap() {
            ids.push("cream");
        }
        let n = ids.len();
        let adds = ids
            .into_iter()
            .enumerate()
            .map(|(i, id)| {
                let base = 0.22 + 0.66 * (i as f32 + 0.5) / n as f32;
                Add {
                    id,
                    mark: (base + rng.range(-0.05, 0.05)).clamp(0.12, 0.94),
                    cue_at: None,
                    done: None,
                }
            })
            .collect();
        Soup {
            set,
            common,
            adds,
            cook: 0.0,
            point: C + vec2(0.0, -RING),
            mouse_hold: false,
            stirring: false,
            stir_t: 0.0,
            band_t: 0.0,
            cook_t: 0.0,
            prev_ang: None,
            omega: 0.0,
            kb_ang: 0.0,
            dir: if rng.f() < 0.5 { 1.0 } else { -1.0 },
            phase0: rng.range(0.0, TAU),
            wiggle: None,
            finished_at: None,
            t: 0.0,
            rng: Rng::new(rng.next_u64()),
            bubbles: Vec::new(),
            bubble_acc: 0.0,
        }
    }

    pub fn focus(&self) -> Vec2 {
        C
    }

    pub fn steps(&self) -> Vec<&'static str> {
        if self.set.finish_tap() {
            vec!["soup_stir.swirl"]
        } else {
            Vec::new()
        }
    }

    fn guide_angle(&self, t: f32) -> f32 {
        self.phase0 + self.dir * t * GUIDE_REV * TAU
    }

    fn guide_pos(&self, t: f32) -> Vec2 {
        let a = self.guide_angle(t);
        C + vec2(a.cos(), a.sin()) * RING
    }

    fn chip(&self, i: usize) -> Rect {
        Rect::new(TRAY_X, 34.0 + i as f32 * 58.0, 76.0, 50.0)
    }

    /// The add whose cue is live.
    fn live(&self) -> Option<usize> {
        self.adds
            .iter()
            .position(|a| a.cue_at.is_some() && a.done.is_none())
    }

    fn zone(&self) -> f32 {
        if self.cook_t <= 0.0 {
            0.0
        } else {
            (self.band_t / self.cook_t).clamp(0.0, 1.0)
        }
    }

    fn add_score(&self, pending: f32) -> f32 {
        let n = self.adds.len().max(1) as f32;
        self.adds
            .iter()
            .map(|a| a.done.map_or(pending, |(acc, _)| acc))
            .sum::<f32>()
            / n
    }

    fn quality_with(&self, pending: f32) -> f32 {
        0.6 * self.zone() + 0.4 * self.add_score(pending)
    }

    pub fn preview(&self, _relaxed: bool) -> f32 {
        self.quality_with(1.0)
    }

    pub fn advice(&self, _relaxed: bool) -> &'static str {
        if 0.4 * (1.0 - self.add_score(0.4)) > 0.6 * (1.0 - self.zone()) {
            "minigame.why.soup.adds"
        } else {
            "minigame.why.soup.rhythm"
        }
    }

    pub fn finished_gesture(&self) -> bool {
        self.cook >= 1.0 && self.adds.iter().all(|a| a.done.is_some())
    }

    fn do_add(&mut self, i: usize, ctx: &mut Ctx, auto: bool) {
        let Some(at) = self.adds[i].cue_at else {
            return;
        };
        let delay = ctx.t - at;
        let acc = if auto {
            0.4
        } else {
            (1.0 - (delay - 0.6 * widen(ctx.relaxed)).max(0.0) / 2.4).clamp(0.4, 1.0)
        };
        self.adds[i].done = Some((acc, ctx.t));
        ctx.sounds.push(MiniGameSound::Pour);
        if acc > 0.8 {
            ctx.sounds.push(MiniGameSound::Cue);
        }
    }

    pub fn step(&mut self, ctx: &mut Ctx, input: &MgInput, dt: f32) -> Option<f32> {
        self.t = ctx.t;
        let on_chip = input
            .pointer
            .and_then(|p| (0..self.adds.len()).find(|i| self.chip(*i).contains(p)));
        if input.pressed && on_chip.is_none() {
            self.mouse_hold = true;
        }
        if input.released || !input.down {
            self.mouse_hold = false;
        }
        // adding: the glowing chip, or ↑ / → / ↓
        let key_add = input.up || input.right || input.down_tap;
        if let Some(i) = self.live() {
            let clicked = input.pressed && on_chip == Some(i);
            let auto =
                !self.common.relaxed && ctx.t - self.adds[i].cue_at.unwrap_or(ctx.t) > ADD_AUTO_S;
            if clicked || key_add {
                self.do_add(i, ctx, false);
            } else if auto {
                self.do_add(i, ctx, true);
            }
        } else if (input.pressed && on_chip.is_some()) || key_add {
            self.wiggle = Some(ctx.t);
        }

        let keyboard = input.hold && !self.mouse_hold;
        let target = if keyboard {
            self.kb_ang += self.dir * GUIDE_REV * TAU * dt;
            let a = self.phase0 + self.kb_ang;
            C + vec2(a.cos(), a.sin()) * RING
        } else {
            input.pointer.unwrap_or(self.point)
        };
        let k = 1.0 - (-dt / 0.06).exp();
        self.point += (target - self.point) * k;
        let rel = self.point - C;
        let in_pot = rel.length() < BROTH_R;
        self.stirring = (self.mouse_hold || input.hold) && in_pot;
        if self.cook < 1.0 {
            self.cook_t += dt;
            if self.stirring {
                let ang = rel.y.atan2(rel.x);
                if let Some(pa) = self.prev_ang {
                    let w = wrap(ang - pa) / dt.max(1e-4) / TAU;
                    self.omega = approach(self.omega, w, dt, 0.15);
                }
                self.prev_ang = Some(ang);
                self.stir_t += dt;
                if score::in_band(self.omega, BAND, ctx.relaxed) && rel.length() > 14.0 {
                    self.band_t += dt;
                }
                self.cook += dt / COOK_S;
            } else {
                self.prev_ang = None;
                self.omega = approach(self.omega, 0.0, dt, 0.3);
                self.cook += dt / SIMMER_S;
            }
            self.cook = self.cook.min(1.0);
        }
        // cues: the pot glows when the soup reaches the next mark (one at a time)
        if self.live().is_none()
            && let Some(i) = self.adds.iter().position(|a| a.done.is_none())
            && self.cook >= self.adds[i].mark
            && self.adds[i].cue_at.is_none()
        {
            self.adds[i].cue_at = Some(ctx.t);
            ctx.sounds.push(MiniGameSound::Cue);
        }
        // a cue past the end still waits for its add
        if self.cook >= 1.0
            && self.live().is_none()
            && let Some(i) = self
                .adds
                .iter()
                .position(|a| a.done.is_none() && a.cue_at.is_none())
        {
            self.adds[i].cue_at = Some(ctx.t);
        }
        // simmer bubbles
        self.bubble_acc += dt * (4.0 + 10.0 * self.cook);
        while self.bubble_acc >= 1.0 {
            self.bubble_acc -= 1.0;
            let a = self.rng.range(0.0, TAU);
            let r = self.rng.f().sqrt() * (BROTH_R - 10.0);
            self.bubbles.push((C + vec2(a.cos(), a.sin()) * r, 0.0));
        }
        for b in &mut self.bubbles {
            b.1 += dt;
        }
        self.bubbles.retain(|b| b.1 < 0.8);

        if self.finished_gesture() {
            self.finished_at.get_or_insert(ctx.t);
        }
        let finish = if self.common.relaxed {
            input.confirm
        } else {
            self.finished_at.is_some_and(|f| ctx.t - f > 0.6)
                || ctx.t > MAX_T
                || (input.confirm && self.cook > 0.5)
        };
        if finish {
            self.stirring = false;
            ctx.sounds.push(MiniGameSound::Clink);
            return Some(self.quality_with(0.4));
        }
        None
    }

    pub fn autoplay(&self, t: f32, keyboard: bool) -> MgInput {
        let live = self.live();
        let add_now = live.is_some_and(|i| {
            self.adds[i]
                .cue_at
                .is_some_and(|c| t - c > 0.25 && t - c < 0.5)
        });
        let stir = t > 0.5 && self.cook < 1.0;
        if keyboard {
            return MgInput {
                hold: stir,
                up: add_now,
                ..Default::default()
            };
        }
        if add_now && let Some(i) = live {
            return MgInput {
                pointer: Some(self.chip(i).center()),
                pressed: true,
                ..Default::default()
            };
        }
        let wob = vec2((t * 1.3).sin(), (t * 1.7).cos()) * 2.0;
        MgInput {
            pointer: Some(self.guide_pos(t + 0.06) + wob),
            pressed: stir,
            down: stir,
            ..Default::default()
        }
    }

    pub fn hint(&self) -> Hint<'static> {
        let done = self.common.relaxed;
        if let Some(i) = self.live() {
            return Hint {
                key: if self.adds[i].id == "cream" {
                    "minigame.soup_stir.prompt.swirl"
                } else {
                    "minigame.soup_stir.prompt.add"
                },
                emphasis: true,
                keys: &["↑"],
                done,
            };
        }
        if self.cook >= 1.0 {
            return Hint {
                key: "minigame.soup_stir.prompt.done",
                emphasis: false,
                keys: &[],
                done,
            };
        }
        Hint {
            key: "minigame.soup_stir.prompt.stir",
            emphasis: false,
            keys: &["Space"],
            done,
        }
    }

    fn broth(&self) -> (Color, Color) {
        let (a, b) = match self.set {
            SoupSet::Garlic => (hex(0xf3d98a), hex(0xc89a3e)),
            SoupSet::Kulajda => (hex(0xf1e4b8), hex(0xcdb880)),
            SoupSet::Borscht => (hex(0xc43a4c), hex(0x861c30)),
        };
        // it deepens as it cooks
        let k = 0.3 + 0.7 * self.cook;
        (
            draw::mix(draw::mix(a, WHITE, 0.25), a, k),
            draw::mix(draw::mix(b, WHITE, 0.2), b, k),
        )
    }

    pub fn draw(&self, ui: &mut Ui, o: Vec2, t: f32, text: &MiniGameText) {
        let th = ui.theme.clone();
        let c = o + C;
        // the stove glow and the pot
        rd::radial(
            c.x,
            c.y,
            POT_R + 24.0,
            POT_R + 20.0,
            Color::new(1.0, 0.55, 0.25, 0.16),
            Color::new(1.0, 0.55, 0.25, 0.0),
        );
        contact_shadow(c + vec2(8.0, 12.0), POT_R + 12.0, POT_R * 0.95, 0.55);
        let sq = |r: f32| Rect::new(c.x - r, c.y - r, r * 2.0, r * 2.0);
        for side in [-1.0f32, 1.0] {
            rd::capsule(
                c + vec2(side * (POT_R - 6.0), 0.0),
                c + vec2(side * (POT_R + 14.0), 0.0),
                7.0,
                hex(0x3d3a4a),
            );
        }
        fill_circle(
            c,
            POT_R,
            Paint::diagonal(sq(POT_R), hex(0xc9cbd8), hex(0x5f6276)),
        );
        fill_circle(c, POT_R - 7.0, hex(0x2e2c38));
        let (b0, b1) = self.broth();
        rd::radial(c.x, c.y, BROTH_R, BROTH_R, b0, b1);
        // the swirl of the stirring
        let spin = self.guide_angle(t) * 0.5;
        for k in 0..4 {
            let r = 22.0 + k as f32 * 17.0;
            let a0 = spin + k as f32 * 1.3;
            draw::arc(
                c,
                r,
                a0,
                a0 + 1.4,
                2.0,
                Color::new(1.0, 1.0, 1.0, 0.08 + if self.stirring { 0.08 } else { 0.0 }),
            );
        }
        for (p, age) in &self.bubbles {
            let k = age / 0.8;
            stroke_circle(
                o + *p,
                1.5 + k * 3.0,
                1.0,
                Color::new(1.0, 1.0, 1.0, 0.35 * (1.0 - k)),
            );
        }
        // what went in
        for (i, a) in self.adds.iter().enumerate() {
            if let Some((_, at)) = a.done {
                let k = ease_out((t - at) / 0.5);
                for j in 0..10u32 {
                    let ang = rd::hash01(j * 7 + i as u32 * 31) * TAU + t * 0.3 * self.dir;
                    let r = rd::hash01(j * 13 + i as u32 * 5).sqrt() * (BROTH_R - 18.0) * k;
                    draw_add(a.id, c + vec2(ang.cos(), ang.sin()) * r, 0.55);
                }
            }
        }
        stroke_circle(c, BROTH_R + 1.0, 2.0, Color::new(1.0, 1.0, 1.0, 0.18));
        // the pot glows when it wants the next ingredient
        if let Some(i) = self.live() {
            let pulse = 0.5 + 0.5 * (t * 5.0).sin();
            glow_circle(c, BROTH_R + 18.0, with_alpha(th.amber, 0.12 + 0.12 * pulse));
            stroke_circle(
                c,
                BROTH_R + 4.0,
                3.0,
                with_alpha(th.amber, 0.6 + 0.3 * pulse),
            );
            let _ = i;
        }
        // the pace light
        if self.cook < 1.0 {
            let g = o + self.guide_pos(t);
            for k in 1..8 {
                let a = self.guide_angle(t) - self.dir * k as f32 * 0.09;
                let p = c + vec2(a.cos(), a.sin()) * RING;
                fill_circle(
                    p,
                    3.4 - k as f32 * 0.35,
                    with_alpha(th.amber, 0.35 * (1.0 - k as f32 / 8.0)),
                );
            }
            glow_circle(g, 18.0, with_alpha(th.amber, 0.35));
            fill_circle(g, 4.0, hex(0xfff3d6));
            // the direction arrow
            let a = self.guide_angle(t) + self.dir * 0.5;
            let p = c + vec2(a.cos(), a.sin()) * (RING + 22.0);
            let tang = vec2(-a.sin(), a.cos()) * self.dir;
            draw::line(
                p - tang * 6.0,
                p + tang * 6.0,
                1.6,
                with_alpha(th.text, 0.4),
            );
        }
        if self.common.practised {
            let tw = ((t * 1.7).sin() * 0.5 + 0.5).powi(2);
            super::paint::sparkle(c + vec2(0.0, -RING), 4.0, 1.0 - tw * 0.8, true);
        }
        // the ladle
        let p = o + self.point;
        let hand = p + vec2(70.0, -90.0);
        rd::capsule(p + vec2(6.0, -8.0), hand, 4.0, hex(0x8b6a4e));
        rd::capsule(
            p + vec2(5.0, -9.0),
            hand + vec2(-1.0, -1.0),
            1.4,
            hex(0xc59a72),
        );
        fill_circle(
            p,
            13.0,
            Paint::diagonal(
                Rect::new(p.x - 13.0, p.y - 13.0, 26.0, 26.0),
                hex(0xd6d8e4),
                hex(0x7a7d92),
            ),
        );
        fill_circle(p, 9.0, with_alpha(b1, 0.9));

        self.draw_side(ui, o, t, text);
    }

    fn draw_side(&self, ui: &mut Ui, o: Vec2, t: f32, text: &MiniGameText) {
        let th = ui.theme.clone();
        // cooking progress (a thin arc around the pot)
        let c = o + C;
        draw::arc(
            c,
            POT_R + 6.0,
            -PI / 2.0,
            -PI / 2.0 + TAU * self.cook,
            3.0,
            with_alpha(th.indigo_300, 0.7),
        );
        for a in &self.adds {
            let ang = -PI / 2.0 + TAU * a.mark;
            let p = c + vec2(ang.cos(), ang.sin()) * (POT_R + 6.0);
            fill_circle(
                p,
                3.0,
                if a.done.is_some() {
                    th.emerald_400
                } else {
                    th.amber
                },
            );
        }
        // the ingredient chips
        let col = Rect::new(o.x + TRAY_X - 8.0, o.y + 10.0, 92.0, 228.0);
        draw_glass(&th, col, &Glass::card());
        ui.fonts.draw_in(
            &text.get("minigame.soup_stir.adds").to_uppercase(),
            Rect::new(col.x, col.y + 6.0, col.w, 14.0),
            &th.micro(),
            Align::Center,
        );
        let live = self.live();
        for (i, a) in self.adds.iter().enumerate() {
            let mut r = self.chip(i).offset(o);
            if live.is_none()
                && a.done.is_none()
                && let Some(w) = self.wiggle
                && t - w < 0.4
            {
                r.x += ((t - w) * 40.0).sin() * 3.0;
            }
            let is_live = live == Some(i);
            if is_live {
                let pulse = 0.5 + 0.5 * (t * 5.0).sin();
                glow_circle(r.center(), 40.0, with_alpha(th.amber, 0.15 + 0.15 * pulse));
                draw::stroke_rrect(r, 8.0, 1.6, with_alpha(th.amber, 0.9));
            }
            crate::ui::draw::fill_rrect(
                r,
                8.0,
                Color::new(1.0, 1.0, 1.0, if a.done.is_some() { 0.03 } else { 0.08 }),
            );
            draw_add(a.id, vec2(r.center().x, r.y + 18.0), 1.0);
            let key = add_key(a.id);
            let name = text.get(&key);
            let st = ui.fonts.fit_width(name, r.w - 6.0, &th.small_medium());
            ui.fonts.draw_in(
                name,
                Rect::new(r.x + 3.0, r.y + 32.0, r.w - 6.0, 14.0),
                &st,
                Align::Center,
            );
            if a.done.is_some() {
                crate::ui::draw::stroke_round(
                    &[
                        vec2(r.right() - 14.0, r.y + 10.0),
                        vec2(r.right() - 11.0, r.y + 13.0),
                        vec2(r.right() - 6.0, r.y + 7.0),
                    ],
                    false,
                    1.6,
                    th.emerald_400,
                );
            }
        }
    }
}

/// A tiny glyph per soup ingredient.
fn draw_add(id: &str, p: Vec2, s: f32) {
    match id {
        "garlic" => {
            for dx in [-4.0, 4.0] {
                rd::ellipse(p.x + dx * s, p.y, 4.0 * s, 6.0 * s, hex(0xf6efe0));
            }
        }
        "croutons" => {
            for (dx, dy) in [(-5.0, -2.0), (4.0, -4.0), (0.0, 4.0)] {
                crate::ui::draw::fill_rrect(
                    Rect::new(p.x + (dx - 3.0) * s, p.y + (dy - 3.0) * s, 6.0 * s, 6.0 * s),
                    1.5,
                    hex(0xc98a3e),
                );
            }
        }
        "mushrooms" => {
            rd::ellipse(p.x, p.y - 2.0 * s, 8.0 * s, 5.0 * s, hex(0x9a6a44));
            crate::ui::draw::fill_rrect(
                Rect::new(p.x - 2.5 * s, p.y, 5.0 * s, 7.0 * s),
                2.0,
                hex(0xead9bd),
            );
        }
        "dill" => {
            for k in 0..5 {
                let a = -PI / 2.0 + (k as f32 - 2.0) * 0.45;
                draw::line(p, p + vec2(a.cos(), a.sin()) * 8.0 * s, 1.0, hex(0x5fae64));
            }
        }
        "beetroot" => {
            fill_circle(p, 6.5 * s, hex(0x8e1f40));
            fill_circle(p + vec2(-2.0, -2.0) * s, 2.0 * s, hex(0xc44a6a));
        }
        "smetana" | "cream" => {
            fill_circle(p, 6.5 * s, hex(0xf4efe6));
            fill_circle(p + vec2(-1.5, -2.0) * s, 3.5 * s, WHITE);
        }
        _ => fill_circle(p, 5.0 * s, hex(0xd9cdb8)),
    }
}
