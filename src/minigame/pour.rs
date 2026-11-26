//! PourOver: a dripper seen from above; hold to pour from the gooseneck kettle and
//! trace slow circles with the calm guide light. Scored by staying near the ring,
//! following the light and moving smoothly.

use macroquad::prelude::*;

use super::paint::{contact_shadow, hex, with_alpha};
use super::score::{pour_perf, pour_sample};
use super::{Ctx, Hint, MgInput, MiniGameSound, MiniGameText, RegularTouch, Rng};
use crate::render::draw as rd;
use crate::ui::draw::{self, Paint, fill_circle, fill_rrect, glow_circle, stroke_circle};
use crate::ui::{Align, Ui, approach};

const C: Vec2 = Vec2::new(186.0, 141.0);
const RIM: f32 = 122.0;
const CONE: f32 = 106.0;
const BED: f32 = 70.0;
const RING: f32 = 44.0;
/// Guide light: one turn every `PERIOD` seconds.
const PERIOD: f32 = 3.4;
const MAX_T: f32 = 22.0;
const CARAFE: Rect = Rect {
    x: 384.0,
    y: 70.0,
    w: 70.0,
    h: 150.0,
};

struct Ripple {
    p: Vec2,
    age: f32,
}

pub(crate) struct Pour {
    mouse_hold: bool,
    point: Vec2,
    pouring: bool,
    flow: f32,
    water: f32,
    need: f32,
    acc_sum: f32,
    acc_n: f32,
    jerk_sum: f32,
    prev_ang: Option<f32>,
    prev_w: Option<f32>,
    wet: [f32; 24],
    ripples: Vec<Ripple>,
    ripple_acc: f32,
    phase0: f32,
    drip: f32,
    t: f32,
    rng: Rng,
}

fn guide_angle(t: f32, phase0: f32) -> f32 {
    phase0 + t / PERIOD * std::f32::consts::TAU
}

fn wrap(a: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    let mut a = a % tau;
    if a > std::f32::consts::PI {
        a -= tau;
    }
    if a < -std::f32::consts::PI {
        a += tau;
    }
    a
}

impl Pour {
    pub fn new(rng: &mut Rng, touch: Option<RegularTouch>) -> Pour {
        let need = if touch == Some(RegularTouch::Strong) { 10.5 } else { 8.5 };
        Pour {
            mouse_hold: false,
            point: C + vec2(0.0, -RING),
            pouring: false,
            flow: 0.0,
            water: 0.0,
            need,
            acc_sum: 0.0,
            acc_n: 0.0,
            jerk_sum: 0.0,
            prev_ang: None,
            prev_w: None,
            wet: [0.0; 24],
            ripples: Vec::new(),
            ripple_acc: 0.0,
            phase0: rng.range(0.0, std::f32::consts::TAU),
            drip: 0.0,
            t: 0.0,
            rng: Rng::new(rng.next_u64()),
        }
    }

    pub fn flow(&self) -> f32 {
        self.flow
    }

    pub fn focus(&self) -> Vec2 {
        C
    }

    fn guide_pos(&self, t: f32) -> Vec2 {
        let a = guide_angle(t, self.phase0);
        C + vec2(a.cos(), a.sin()) * RING
    }

    pub fn step(&mut self, ctx: &mut Ctx, input: &MgInput, dt: f32) -> Option<f32> {
        self.t = ctx.t;
        if input.pressed {
            self.mouse_hold = true;
        }
        if input.released || !input.down {
            self.mouse_hold = false;
        }
        let keyboard = input.hold && !self.mouse_hold;
        let target = if keyboard {
            // keyboard: the spout drifts along the ring a little behind the light
            let a = guide_angle(ctx.t, self.phase0) - 0.35;
            C + vec2(a.cos(), a.sin()) * RING * 1.04
        } else {
            input.pointer.unwrap_or(self.point)
        };
        // the kettle is heavy: the pour point follows with a little lag (smooth)
        let k = 1.0 - (-dt / 0.07).exp();
        self.point += (target - self.point) * k;

        let want = self.mouse_hold || input.hold;
        if want && !self.pouring {
            ctx.sounds.push(MiniGameSound::Pour);
        }
        self.pouring = want;
        self.flow = approach(self.flow, if want { 1.0 } else { 0.0 }, dt, 0.15);

        let rel = self.point - C;
        let ang = rel.y.atan2(rel.x);
        if self.pouring && rel.length() < CONE {
            self.water += dt * self.flow;
            let radial = (rel.length() - RING).abs() / RING;
            let ga = guide_angle(ctx.t, self.phase0);
            let s = pour_sample(radial, wrap(ang - ga), ctx.relaxed);
            self.acc_sum += s * dt;
            self.acc_n += dt;
            if let Some(pa) = self.prev_ang {
                let w = wrap(ang - pa) / dt.max(1e-4);
                if let Some(pw) = self.prev_w {
                    self.jerk_sum += (w - pw).abs().min(30.0) * dt;
                }
                self.prev_w = Some(w);
            }
            self.prev_ang = Some(ang);
            // wet the bed sector under the stream
            let sector = (((ang + std::f32::consts::PI) / std::f32::consts::TAU * 24.0) as usize).min(23);
            self.wet[sector] = (self.wet[sector] + dt * 1.4).min(1.0);
            self.ripple_acc += dt * 7.0;
            while self.ripple_acc >= 1.0 {
                self.ripple_acc -= 1.0;
                let j = vec2(self.rng.range(-2.0, 2.0), self.rng.range(-2.0, 2.0));
                self.ripples.push(Ripple {
                    p: self.point + j,
                    age: 0.0,
                });
            }
        } else {
            self.prev_ang = None;
            self.prev_w = None;
        }
        for r in &mut self.ripples {
            r.age += dt;
        }
        self.ripples.retain(|r| r.age < 0.9);
        for w in &mut self.wet {
            *w = (*w - dt * 0.05).max(0.0);
        }
        // coffee drips into the carafe a little behind the pour
        self.drip = approach(self.drip, (self.water / self.need).min(1.0), dt, 1.2);

        let full = self.water >= self.need;
        if full || ctx.t > MAX_T {
            let mean = if self.acc_n > 0.0 { self.acc_sum / self.acc_n } else { 0.0 };
            // an unfinished pour weighs down gently
            let fill = (self.water / self.need).clamp(0.0, 1.0);
            let jerk = if self.acc_n > 0.0 { self.jerk_sum / self.acc_n } else { 0.0 };
            let perf = pour_perf(mean, jerk, ctx.relaxed) * (0.4 + 0.6 * fill);
            ctx.sounds.push(MiniGameSound::Clink);
            return Some(perf);
        }
        None
    }

    pub fn autoplay(&self, t: f32) -> MgInput {
        if t < 0.7 {
            return MgInput {
                pointer: Some(self.guide_pos(t)),
                ..Default::default()
            };
        }
        let wobble = vec2((t * 1.3).sin(), (t * 1.7).cos()) * 2.5;
        MgInput {
            pointer: Some(self.guide_pos(t + 0.05) + wobble),
            pressed: true,
            down: true,
            ..Default::default()
        }
    }

    pub fn hint(&self) -> Hint<'static> {
        if self.pouring && self.point.distance(C) > RING * 1.6 {
            Hint {
                key: "minigame.hint.pour_keep",
                emphasis: true,
                keys: &[],
                serve: false,
            }
        } else {
            Hint {
                key: "minigame.hint.pour",
                emphasis: false,
                keys: &["Space"],
                serve: false,
            }
        }
    }

    pub fn draw(&self, ui: &mut Ui, o: Vec2, t: f32, text: &MiniGameText) {
        let th = ui.theme.clone();
        let c = o + C;
        // dripper on a carafe: from above we see the ceramic rim and the cone
        contact_shadow(c + vec2(8.0, 12.0), RIM + 14.0, RIM * 0.95, 0.55);
        // the dripper's little handle
        let handle = Rect::new(c.x - 18.0, c.y + RIM - 14.0, 36.0, 30.0);
        fill_rrect(handle, 12.0, Paint::vertical(handle, hex(0xf2ecf7), hex(0xb9aecd)));
        fill_circle(c, RIM, Paint::diagonal(Rect::new(c.x - RIM, c.y - RIM, RIM * 2.0, RIM * 2.0), hex(0xfbf8fd), hex(0xb8acd0)));
        fill_circle(c, CONE + 3.0, with_alpha(hex(0x8c7fae), 0.5));
        // paper filter cone: concentric gradient rings
        for i in 0..6 {
            let k = 1.0 - i as f32 / 6.0;
            let col = draw::mix(hex(0xd9cdb8), hex(0xf6efe2), 1.0 - k);
            fill_circle(c, CONE * (0.62 + 0.38 * k), col);
        }
        // filter pleats / dripper ribs as faint spiral spokes
        for i in 0..16 {
            let a = i as f32 / 16.0 * std::f32::consts::TAU;
            let a2 = a + 0.25;
            let p0 = c + vec2(a.cos(), a.sin()) * (BED + 4.0);
            let p1 = c + vec2(a2.cos(), a2.sin()) * (CONE - 3.0);
            draw::line(p0, p1, 1.0, Color::new(0.55, 0.45, 0.35, 0.18));
        }
        // coffee bed
        rd::radial(c.x, c.y, BED + 3.0, BED + 3.0, hex(0x6a4128), hex(0x3a2216));
        for i in 0..140u32 {
            let a = rd::hash01(i * 17 + 3) * std::f32::consts::TAU;
            let r = rd::hash01(i * 31 + 7).sqrt() * (BED - 2.0);
            let g = c + vec2(a.cos(), a.sin()) * r;
            let col = if i % 2 == 0 {
                Color::new(0.55, 0.36, 0.22, 0.5)
            } else {
                Color::new(0.16, 0.08, 0.04, 0.45)
            };
            fill_circle(g, 0.7 + rd::hash01(i) * 0.9, col);
        }
        for (i, w) in self.wet.iter().enumerate() {
            if *w <= 0.01 {
                continue;
            }
            let a0 = i as f32 / 24.0 * std::f32::consts::TAU - std::f32::consts::PI;
            let mid = a0 + std::f32::consts::TAU / 48.0;
            let p = c + vec2(mid.cos(), mid.sin()) * RING;
            rd::glow(p.x, p.y, 24.0, Color::new(0.16, 0.08, 0.04, 0.45 * w));
            // blooming bubbles
            for k in 0..3u32 {
                let h = rd::hash01(i as u32 * 7 + k * 3);
                let bp = p + vec2((h * 6.3).cos(), (h * 6.3).sin()) * (6.0 + h * 10.0);
                let pulse = ((t * 1.5 + h * 4.0).sin() * 0.5 + 0.5) * w;
                fill_circle(bp, 1.2 + 1.5 * pulse, Color::new(0.86, 0.68, 0.48, 0.45 * pulse));
            }
        }
        draw::stroke_circle(c, BED + 4.0, 6.0, hex(0x4a2e1d));
        // the guide ring and the calm metronome light
        let pulse = 0.5 + 0.5 * (t / PERIOD * std::f32::consts::TAU * 2.0).sin();
        stroke_circle(c, RING, 2.0, with_alpha(th.indigo_300, 0.22 + 0.1 * pulse));
        glow_circle(c, RING + 10.0, with_alpha(th.indigo_400, 0.06));
        let g = o + self.guide_pos(t);
        for k in 1..8 {
            let a = guide_angle(t, self.phase0) - k as f32 * 0.09;
            let p = c + vec2(a.cos(), a.sin()) * RING;
            fill_circle(p, 3.6 - k as f32 * 0.35, with_alpha(th.amber, 0.4 * (1.0 - k as f32 / 8.0)));
        }
        glow_circle(g, 22.0, with_alpha(th.amber, 0.45 + 0.15 * pulse));
        fill_circle(g, 4.5, hex(0xfff3d6));

        // ripples where the water lands
        for r in &self.ripples {
            let a = r.age / 0.9;
            stroke_circle(o + r.p, 3.0 + a * 13.0, 1.2, Color::new(0.95, 0.9, 0.82, 0.4 * (1.0 - a)));
        }

        // gooseneck kettle spout coming from the top-right
        let p = o + self.point;
        let tip = p + vec2(10.0, -12.0);
        // kettle body peeking in from the top edge (top view: lid + knob)
        let kc = vec2(o.x + 300.0, o.y - 34.0);
        rd::soft_shadow(kc.x + 8.0, kc.y + 14.0, 74.0, 66.0, 0.45);
        fill_circle(kc, 64.0, Paint::diagonal(Rect::new(kc.x - 64.0, kc.y - 64.0, 128.0, 128.0), hex(0x5a5f74), hex(0x23252f)));
        stroke_circle(kc, 46.0, 2.0, Color::new(1.0, 1.0, 1.0, 0.12));
        fill_circle(kc + vec2(0.0, 6.0), 12.0, hex(0x9c6a47));
        draw::arc(kc, 62.0, 1.9, 2.9, 2.0, Color::new(0.8, 0.78, 1.0, 0.4));
        // gooseneck: a smooth cubic from the kettle to the spout tip, thicker at the base
        let base = kc + vec2(-34.0, 50.0);
        let c1 = base + vec2(-10.0, 40.0);
        let c2 = tip + vec2(24.0, -50.0);
        let n = 28;
        let pts: Vec<Vec2> = (0..=n)
            .map(|i| {
                let u = i as f32 / n as f32;
                let v = 1.0 - u;
                base * v * v * v + c1 * 3.0 * v * v * u + c2 * 3.0 * v * u * u + tip * u * u * u
            })
            .collect();
        rd::soft_shadow(p.x + 34.0, p.y + 20.0, 50.0, 16.0, 0.22);
        for (i, w) in pts.windows(2).enumerate() {
            let u = i as f32 / n as f32;
            rd::capsule(w[0], w[1], 5.6 - 2.2 * u, hex(0x40445a));
        }
        for (i, w) in pts.windows(2).enumerate() {
            let u = i as f32 / n as f32;
            rd::capsule(w[0] + vec2(-0.6, -0.6), w[1] + vec2(-0.6, -0.6), 3.6 - 1.4 * u, hex(0x7d849c));
        }
        let hl: Vec<Vec2> = pts.iter().map(|q| *q + vec2(-1.6, -1.6)).collect();
        draw::stroke_path(&hl, false, 1.2, Paint::Solid(Color::new(1.0, 1.0, 1.0, 0.5)));
        fill_circle(tip, 3.4, hex(0x30333f));
        if self.flow > 0.02 {
            rd::capsule(tip, p, 1.8 * self.flow + 0.3, Color::new(0.86, 0.92, 1.0, 0.8 * self.flow));
            glow_circle(p, 12.0, Color::new(0.86, 0.92, 1.0, 0.3 * self.flow));
        }

        self.draw_carafe(ui, o, t, text);
    }

    fn draw_carafe(&self, ui: &mut Ui, o: Vec2, t: f32, text: &MiniGameText) {
        let th = ui.theme.clone();
        let r = CARAFE.offset(o);
        contact_shadow(vec2(r.center().x + 3.0, r.bottom() + 2.0), r.w * 0.7, 9.0, 0.5);
        // glass body (side view)
        let body = Rect::new(r.x, r.y + 26.0, r.w, r.h - 26.0);
        fill_rrect(body, 24.0, Color::new(0.8, 0.78, 1.0, 0.1));
        let lvl = self.drip;
        let h = (body.h - 10.0) * lvl;
        if h > 1.0 {
            let f = Rect::new(body.x + 4.0, body.bottom() - 5.0 - h, body.w - 8.0, h);
            fill_rrect(f, 20.0f32.min(h / 2.0), Paint::vertical(f, hex(0x8a5530), hex(0x3d1f12)));
            draw::hline(f.x + 8.0, f.right() - 8.0, f.y + 1.0, Color::new(1.0, 0.85, 0.6, 0.4));
        }
        // neck and wooden collar
        let neck = Rect::new(r.x + 16.0, r.y, r.w - 32.0, 30.0);
        fill_rrect(neck, 6.0, Color::new(0.8, 0.78, 1.0, 0.12));
        let collar = Rect::new(r.x + 12.0, r.y + 22.0, r.w - 24.0, 14.0);
        fill_rrect(collar, 5.0, Paint::vertical(collar, hex(0xc08a5e), hex(0x7d5034)));
        // glass highlights
        draw::stroke_rrect(body, 24.0, 1.2, Color::new(0.9, 0.88, 1.0, 0.35));
        draw::line(vec2(body.x + 10.0, body.y + 18.0), vec2(body.x + 10.0, body.bottom() - 24.0), 2.0, Color::new(1.0, 1.0, 1.0, 0.25));
        // a drip falling
        if self.flow > 0.05 || self.drip < (self.water / self.need).min(1.0) - 0.01 {
            let ph = (t * 1.7).fract();
            let dy = r.y + 4.0 + ph * 40.0;
            fill_circle(vec2(r.center().x, dy), 2.0, with_alpha(hex(0x6b3e22), 1.0 - ph));
        }
        let pct = format!("{}%", (self.drip * 100.0).round() as i32);
        let st = th.small_medium().tabular();
        ui.fonts.draw_in(&pct, Rect::new(r.x - 10.0, r.bottom() + 10.0, r.w + 20.0, 16.0), &st, Align::Center);
        let micro = th.micro();
        ui.fonts.draw_in(&text.get("minigame.water").to_uppercase(), Rect::new(r.x - 20.0, r.y - 22.0, r.w + 40.0, 14.0), &micro, Align::Center);
    }
}
