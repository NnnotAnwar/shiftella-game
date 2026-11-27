//! MulledSpice (svařák): a pot of red wine from above, a tray of six spices and the recipe
//! card above it. Drop in the card's spices (click a chip; keyboard ←/→ and Space) — the
//! others float back out with a little shake — and keep the wine warm, never boiling: set the
//! flame (drag the knob, or hold ↑/↓) so the thermometer stays in the amber band. The tray is
//! shuffled and the band drifts a little each play.
//! Quality: `0.5 × spices + 0.5 × min(1, time in the band / band_s)`; a spice not on the card
//! costs `wrong_penalty` of the spice half.

use std::f32::consts::{PI, TAU};

use macroquad::prelude::*;

use super::paint::{contact_shadow, hex, with_alpha};
use super::score::widen;
use super::{Common, Ctx, Hint, MgInput, MiniGameSound, MiniGameText, MulledSet, Rng};
use crate::render::draw as rd;
use crate::ui::draw::{self, Paint, fill_circle, fill_rrect, glow_circle, stroke_circle};
use crate::ui::{Align, Glass, RectExt, Ui, approach, draw_glass, ease_out};

const C: Vec2 = Vec2::new(122.0, 128.0);
const POT_R: f32 = 98.0;
const WINE_R: f32 = 84.0;
const WRONG_PENALTY: f32 = 0.15;
const BAND_S: f32 = 3.0;
const MAX_T: f32 = 22.0;
/// The flame knob (a vertical slider) and the thermometer.
const KNOB: Rect = Rect {
    x: 440.0,
    y: 40.0,
    w: 30.0,
    h: 170.0,
};
const T_MIN: f32 = 30.0;
const T_MAX: f32 = 92.0;

struct Chip {
    id: &'static str,
    on_card: bool,
    added: Option<f32>,
    wiggle: Option<f32>,
}

pub(crate) struct Mulled {
    set: MulledSet,
    common: Common,
    chips: Vec<Chip>,
    sel: usize,
    flame: f32,
    temp: f32,
    band: (f32, f32),
    band_t: f32,
    wrong: u32,
    dragging_knob: bool,
    finished_at: Option<f32>,
    last_add: f32,
    t: f32,
}

/// `minigame.spice.<id>`
pub(crate) fn spice_key(id: &str) -> String {
    format!("minigame.spice.{id}")
}

impl Mulled {
    pub fn new(rng: &mut Rng, set: MulledSet, common: Common) -> Mulled {
        let mut tray: Vec<&'static str> = set.tray().to_vec();
        for i in (1..tray.len()).rev() {
            let j = (rng.next_u64() % (i as u64 + 1)) as usize;
            tray.swap(i, j);
        }
        let chips = tray
            .into_iter()
            .map(|id| Chip {
                id,
                on_card: set.spices().contains(&id),
                added: None,
                wiggle: None,
            })
            .collect();
        let mid = 73.0 + rng.range(-1.5, 1.5);
        Mulled {
            set,
            common,
            chips,
            sel: 0,
            flame: rng.range(0.1, 0.3),
            temp: T_MIN + rng.range(0.0, 6.0),
            band: (mid - 5.0, mid + 5.0),
            band_t: 0.0,
            wrong: 0,
            dragging_knob: false,
            finished_at: None,
            last_add: 0.0,
            t: 0.0,
        }
    }

    pub fn focus(&self) -> Vec2 {
        C
    }

    pub fn steps(&self) -> Vec<&'static str> {
        Vec::new()
    }

    fn band(&self, relaxed: bool) -> (f32, f32) {
        let mid = (self.band.0 + self.band.1) * 0.5;
        let half = (self.band.1 - self.band.0) * 0.5 * widen(relaxed);
        (mid - half, mid + half)
    }

    /// Temperature the flame settles at.
    fn temp_for(flame: f32) -> f32 {
        T_MIN + (T_MAX - T_MIN) * flame
    }

    fn flame_for(temp: f32) -> f32 {
        ((temp - T_MIN) / (T_MAX - T_MIN)).clamp(0.0, 1.0)
    }

    fn chip_pos(&self, i: usize) -> Vec2 {
        let col = (i % 3) as f32;
        let row = (i / 3) as f32;
        vec2(262.0 + col * 62.0, 104.0 + row * 70.0)
    }

    fn needed(&self) -> usize {
        self.set.spices().len()
    }

    fn added_right(&self) -> usize {
        self.chips
            .iter()
            .filter(|c| c.on_card && c.added.is_some())
            .count()
    }

    fn spice_score(&self, pending: bool) -> f32 {
        let n = self.needed().max(1) as f32;
        let right = if pending {
            n
        } else {
            self.added_right() as f32
        };
        (right / n - WRONG_PENALTY * self.wrong as f32).clamp(0.0, 1.0)
    }

    fn heat_score(&self) -> f32 {
        (self.band_t / BAND_S).min(1.0)
    }

    pub fn preview(&self, _relaxed: bool) -> f32 {
        // the spices count as they go in; the warmth fills as it holds in the band
        let n = self.needed().max(1) as f32;
        let spices = (self.added_right() as f32 / n - WRONG_PENALTY * self.wrong as f32).max(0.0);
        0.5 * spices + 0.5 * self.heat_score()
    }

    fn quality(&self) -> f32 {
        0.5 * self.spice_score(false) + 0.5 * self.heat_score()
    }

    pub fn advice(&self, _relaxed: bool) -> &'static str {
        if 1.0 - self.spice_score(false) > 1.0 - self.heat_score() {
            "minigame.why.mulled.spices"
        } else if self.temp > self.band.1 {
            "minigame.why.mulled.hot"
        } else {
            "minigame.why.mulled.cool"
        }
    }

    pub fn finished_gesture(&self) -> bool {
        self.added_right() >= self.needed() && self.band_t >= BAND_S
    }

    fn drop_chip(&mut self, i: usize, ctx: &mut Ctx) {
        let Some(c) = self.chips.get_mut(i) else {
            return;
        };
        if c.added.is_some() {
            return;
        }
        self.last_add = ctx.t;
        if c.on_card {
            c.added = Some(ctx.t);
            ctx.sounds.push(MiniGameSound::Pour);
            ctx.sounds.push(MiniGameSound::Cue);
        } else if c.wiggle.is_none_or(|w| ctx.t - w > 0.5) {
            // not on the card: it bobs back out (once per tap)
            c.wiggle = Some(ctx.t);
            self.wrong += 1;
            ctx.sounds.push(MiniGameSound::Clink);
        }
    }

    pub fn step(&mut self, ctx: &mut Ctx, input: &MgInput, dt: f32) -> Option<f32> {
        self.t = ctx.t;
        let n = self.chips.len();
        if input.left {
            self.sel = (self.sel + n - 1) % n;
        }
        if input.right {
            self.sel = (self.sel + 1) % n;
        }
        let knob = KNOB.grow(10.0);
        if input.pressed
            && let Some(p) = input.pointer
        {
            self.dragging_knob = knob.contains(p);
            if !self.dragging_knob
                && let Some(i) = (0..n).find(|i| p.distance(self.chip_pos(*i)) < 22.0)
            {
                self.sel = i;
                self.drop_chip(i, ctx);
            }
        }
        if !input.down {
            self.dragging_knob = false;
        }
        if self.dragging_knob
            && let Some(p) = input.pointer
        {
            self.flame = (1.0 - (p.y - KNOB.y) / KNOB.h).clamp(0.0, 1.0);
        }
        if input.axis.y != 0.0 {
            self.flame = (self.flame - input.axis.y * dt * 0.45).clamp(0.0, 1.0);
        }
        if input.hold_pressed {
            self.drop_chip(self.sel, ctx);
        }
        // the wine follows the flame, slowly
        let goal = Self::temp_for(self.flame);
        self.temp = approach(self.temp, goal, dt, 2.0);
        let (lo, hi) = self.band(ctx.relaxed);
        if (lo..=hi).contains(&self.temp) {
            if self.band_t < BAND_S && self.band_t + dt >= BAND_S {
                ctx.sounds.push(MiniGameSound::Cue);
            }
            self.band_t += dt;
        }
        // grandma's hand: a forgotten spice goes in by itself after a while (counts half)
        if !self.common.relaxed
            && ctx.t - self.last_add > 7.0
            && ctx.t > 7.0
            && let Some(i) = self
                .chips
                .iter()
                .position(|c| c.on_card && c.added.is_none())
        {
            self.chips[i].added = Some(ctx.t);
            self.wrong += 1;
            self.last_add = ctx.t;
            ctx.sounds.push(MiniGameSound::Pour);
        }
        if self.finished_gesture() {
            self.finished_at.get_or_insert(ctx.t);
        }
        let finish = if self.common.relaxed {
            input.confirm
        } else {
            self.finished_at.is_some_and(|f| ctx.t - f > 0.6)
                || ctx.t > MAX_T
                || (input.confirm && self.added_right() > 0)
        };
        if finish {
            ctx.sounds.push(MiniGameSound::Clink);
            return Some(self.quality());
        }
        None
    }

    pub fn autoplay(&self, t: f32, keyboard: bool) -> MgInput {
        let want = Self::flame_for((self.band.0 + self.band.1) * 0.5);
        let next = self
            .chips
            .iter()
            .position(|c| c.on_card && c.added.is_none());
        let ready = t > 0.8 && t - self.last_add > 0.55;
        if keyboard {
            let axis = if self.flame < want - 0.015 {
                -1.0
            } else if self.flame > want + 0.015 {
                1.0
            } else {
                0.0
            };
            let (right, drop) = match next {
                Some(i) if i != self.sel => (
                    ready && ((t * 60.0).round() as u32).is_multiple_of(20),
                    false,
                ),
                Some(_) => (false, ready),
                None => (false, false),
            };
            return MgInput {
                axis: vec2(0.0, axis),
                right,
                hold_pressed: drop,
                ..Default::default()
            };
        }
        // mouse: set the flame first, then the spices one by one
        if (self.flame - want).abs() > 0.01 && (self.dragging_knob || t < 2.0) {
            let y = KNOB.y + (1.0 - want) * KNOB.h;
            let x = KNOB.center().x;
            return MgInput {
                pointer: Some(vec2(x, y)),
                pressed: !self.dragging_knob,
                down: true,
                ..Default::default()
            };
        }
        match next {
            Some(i) if ready => MgInput {
                pointer: Some(self.chip_pos(i)),
                pressed: true,
                ..Default::default()
            },
            _ => MgInput::default(),
        }
    }

    pub fn hint(&self) -> Hint<'static> {
        let done = self.common.relaxed;
        let (lo, hi) = self.band(self.common.relaxed);
        if self.temp > hi + 1.0 {
            return Hint {
                key: "minigame.mulled_spice.prompt.hot",
                emphasis: true,
                keys: &["↓"],
                done,
            };
        }
        if self.added_right() < self.needed() {
            return Hint {
                key: "minigame.mulled_spice.prompt.spices",
                emphasis: false,
                keys: &["←", "→", "Space"],
                done,
            };
        }
        if self.temp < lo {
            return Hint {
                key: "minigame.mulled_spice.prompt.warm",
                emphasis: false,
                keys: &["↑"],
                done,
            };
        }
        Hint {
            key: "minigame.mulled_spice.prompt.hold",
            emphasis: false,
            keys: &[],
            done,
        }
    }

    pub fn draw(&self, ui: &mut Ui, o: Vec2, t: f32, text: &MiniGameText) {
        let th = ui.theme.clone();
        let c = o + C;
        let heat = ((self.temp - T_MIN) / (T_MAX - T_MIN)).clamp(0.0, 1.0);
        rd::radial(
            c.x,
            c.y,
            POT_R + 20.0,
            POT_R + 18.0,
            Color::new(1.0, 0.5, 0.2, 0.08 + 0.18 * self.flame),
            Color::new(1.0, 0.5, 0.2, 0.0),
        );
        contact_shadow(c + vec2(8.0, 12.0), POT_R + 10.0, POT_R * 0.95, 0.55);
        let sq = |r: f32| Rect::new(c.x - r, c.y - r, r * 2.0, r * 2.0);
        fill_circle(
            c,
            POT_R,
            Paint::diagonal(sq(POT_R), hex(0xe0a070), hex(0x7a3d22)),
        );
        fill_circle(c, POT_R - 6.0, hex(0x3a1c14));
        rd::radial(
            c.x,
            c.y,
            WINE_R,
            WINE_R,
            draw::mix(hex(0x9a2a3a), hex(0xb8404a), heat * 0.4),
            hex(0x4a0c1c),
        );
        // the spices that went in, drifting
        for (i, ch) in self.chips.iter().enumerate() {
            if let Some(at) = ch.added {
                let k = ease_out((t - at) / 0.5);
                let a = i as f32 * 1.7 + t * 0.25;
                let r = 30.0 + (i % 3) as f32 * 14.0;
                let from = o + self.chip_pos(i);
                let to = c + vec2(a.cos(), a.sin()) * r;
                draw_spice(ch.id, from.lerp(to, k), 0.8);
            }
        }
        // a simmer shimmer; steam if it is too hot
        let bubbles = (heat * 18.0) as u32;
        for i in 0..bubbles {
            let ph = (t * (0.7 + rd::hash01(i) * 0.6) + rd::hash01(i * 3)).fract();
            let a = rd::hash01(i * 11 + 2) * TAU;
            let r = rd::hash01(i * 7 + 9).sqrt() * (WINE_R - 10.0);
            stroke_circle(
                c + vec2(a.cos(), a.sin()) * r,
                1.0 + ph * 3.0,
                1.0,
                Color::new(1.0, 0.85, 0.85, 0.35 * (1.0 - ph)),
            );
        }
        if self.temp > self.band.1 {
            for i in 0..4u32 {
                let life = 1.8;
                let aa = ((t + i as f32 * 0.45) % life) / life;
                rd::glow(
                    c.x - 30.0 + i as f32 * 20.0,
                    c.y - 20.0 - aa * 60.0,
                    12.0 + aa * 12.0,
                    Color::new(0.95, 0.94, 1.0, 0.18 * (1.0 - aa)),
                );
            }
        }
        draw::arc(
            c,
            POT_R - 3.0,
            3.6,
            5.2,
            2.0,
            Color::new(1.0, 0.9, 0.8, 0.45),
        );

        // the recipe card above the tray
        let card = Rect::new(o.x + 238.0, o.y + 10.0, 178.0, 48.0);
        draw_glass(&th, card, &Glass::card());
        ui.fonts.draw(
            &text.get("minigame.mulled_spice.card").to_uppercase(),
            card.x + 10.0,
            card.y + 15.0,
            &th.micro(),
        );
        for (k, id) in self.set.spices().iter().enumerate() {
            let p = vec2(card.x + 22.0 + k as f32 * 38.0, card.y + 33.0);
            let done = self.chips.iter().any(|c| c.id == *id && c.added.is_some());
            draw_spice(id, p, 0.75);
            if done {
                draw::stroke_round(
                    &[
                        p + vec2(8.0, -4.0),
                        p + vec2(10.5, -1.5),
                        p + vec2(15.0, -7.0),
                    ],
                    false,
                    1.6,
                    th.emerald_400,
                );
            }
        }
        // the tray
        for (i, ch) in self.chips.iter().enumerate() {
            let mut p = o + self.chip_pos(i);
            if let Some(w) = ch.wiggle
                && t - w < 0.5
            {
                let k = (t - w) / 0.5;
                p.x += ((t - w) * 38.0).sin() * 5.0 * (1.0 - k);
                p.y -= (k * PI).sin() * 8.0;
            }
            let sel = i == self.sel;
            if sel {
                stroke_circle(p, 22.0, 1.6, with_alpha(th.amber, 0.9));
            }
            fill_circle(
                p,
                19.0,
                Color::new(1.0, 1.0, 1.0, if ch.added.is_some() { 0.03 } else { 0.09 }),
            );
            if ch.added.is_none() {
                draw_spice(ch.id, p, 1.0);
            }
            let key = spice_key(ch.id);
            let name = text.get(&key);
            let st = ui.fonts.fit_width(name, 58.0, &th.micro());
            ui.fonts.draw_in(
                name,
                Rect::new(p.x - 29.0, p.y + 21.0, 58.0, 12.0),
                &st,
                Align::Center,
            );
        }

        // thermometer + flame knob
        let r = KNOB.offset(o);
        let (lo, hi) = self.band(self.common.relaxed);
        let y_of = |temp: f32| r.bottom() - (temp - T_MIN) / (T_MAX - T_MIN) * r.h;
        fill_rrect(r, 8.0, Color::new(1.0, 1.0, 1.0, 0.08));
        let band = Rect::new(r.x - 3.0, y_of(hi), r.w + 6.0, y_of(lo) - y_of(hi));
        let inside = (lo..=hi).contains(&self.temp);
        fill_rrect(
            band,
            5.0,
            with_alpha(th.amber, if inside { 0.35 } else { 0.18 }),
        );
        draw::stroke_rrect(band, 5.0, 1.2, with_alpha(th.amber, 0.8));
        let ty = y_of(self.temp.clamp(T_MIN, T_MAX));
        let fill = Rect::new(r.x + r.w / 2.0 - 3.0, ty, 6.0, r.bottom() - ty);
        fill_rrect(
            fill,
            3.0,
            Paint::vertical(fill, hex(0xf06a5a), hex(0xb02a3a)),
        );
        // the knob marker (flame level)
        let ky = r.bottom() - self.flame * r.h;
        let kr = Rect::new(r.x - 6.0, ky - 6.0, r.w + 12.0, 12.0);
        fill_rrect(kr, 6.0, Paint::vertical(kr, hex(0xf2ecf7), hex(0xa99dc6)));
        glow_circle(
            vec2(r.center().x, r.bottom() + 14.0),
            10.0 + 10.0 * self.flame,
            Color::new(1.0, 0.6, 0.25, 0.25 + 0.4 * self.flame),
        );
        fill_circle(
            vec2(r.center().x, r.bottom() + 14.0),
            3.0 + 4.0 * self.flame,
            Color::new(0.55, 0.65, 1.0, 0.9),
        );
        let txt = format!("{}°", self.temp.round() as i32);
        ui.fonts.draw_in(
            &txt,
            Rect::new(r.x - 30.0, r.y - 22.0, r.w + 60.0, 16.0),
            &th.small_medium().tabular(),
            Align::Center,
        );
        // warmth held: a little ring fills
        let k = (self.band_t / BAND_S).min(1.0);
        draw::arc(
            vec2(r.center().x - 44.0, r.y + 6.0),
            8.0,
            -PI / 2.0,
            -PI / 2.0 + TAU * k,
            2.5,
            with_alpha(th.amber, 0.9),
        );
    }
}

/// A tiny glyph per svařák spice.
fn draw_spice(id: &str, p: Vec2, s: f32) {
    match id {
        "cinnamon" => {
            rd::capsule(
                p + vec2(-9.0, 4.0) * s,
                p + vec2(9.0, -4.0) * s,
                3.6 * s,
                hex(0xa0582a),
            );
            rd::capsule(
                p + vec2(-8.0, 2.6) * s,
                p + vec2(8.0, -5.4) * s,
                1.2 * s,
                hex(0xd28a50),
            );
        }
        "clove" => {
            for k in 0..3 {
                let a = k as f32 * 2.1;
                let d = vec2(a.cos(), a.sin()) * 5.0 * s;
                rd::capsule(p, p + d, 1.3 * s, hex(0x4a2a1c));
                fill_circle(p + d, 2.4 * s, hex(0x6b3a22));
            }
        }
        "star_anise" => {
            for k in 0..8 {
                let a = k as f32 / 8.0 * TAU;
                let d = vec2(a.cos(), a.sin());
                draw::fill_convex(
                    &[
                        p + vec2(-d.y, d.x) * 2.2 * s,
                        p + d * 9.0 * s,
                        p - vec2(-d.y, d.x) * 2.2 * s,
                    ],
                    Paint::Solid(hex(0x7a3f22)),
                );
            }
            fill_circle(p, 2.4 * s, hex(0xa86a3a));
        }
        "orange" => {
            fill_circle(p, 9.0 * s, hex(0xf08a24));
            fill_circle(p, 7.2 * s, hex(0xffc56a));
            for k in 0..8 {
                let a = k as f32 / 8.0 * TAU;
                draw::line(p, p + vec2(a.cos(), a.sin()) * 6.8 * s, 1.0, hex(0xf29a3a));
            }
        }
        "honey_drop" => {
            draw::fill_convex(
                &[
                    p + vec2(0.0, -9.0) * s,
                    p + vec2(5.5, 1.0) * s,
                    p + vec2(0.0, 6.5) * s,
                    p + vec2(-5.5, 1.0) * s,
                ],
                Paint::Solid(hex(0xe39a2c)),
            );
            fill_circle(p + vec2(0.0, 2.0) * s, 5.4 * s, hex(0xe39a2c));
            fill_circle(p + vec2(-1.6, 0.5) * s, 1.6 * s, hex(0xf6cf78));
        }
        "ginger" => {
            for (dx, dy, r) in [(-3.0, 1.0, 5.5), (4.0, -2.0, 4.5), (1.0, 5.0, 3.5)] {
                fill_circle(p + vec2(dx, dy) * s, r * s, hex(0xe0b368));
            }
        }
        _ => fill_circle(p, 6.0 * s, hex(0xd9cdb8)),
    }
}
