//! MilkFroth: hold to froth, the foam rises in a glass pitcher; let go while the
//! level sits in the softly glowing sweet-spot band.

use macroquad::prelude::*;

use super::paint::{contact_shadow, ellipse_arc, with_alpha};
use super::score::froth_perf;
use super::{Ctx, Hint, MgInput, MiniGameSound, RegularTouch, Rng};
use crate::render::draw as rd;
use crate::ui::draw::{self, Paint, fill_convex, fill_rrect, glow_circle, stroke_rrect};
use crate::ui::{Icon, Ui, approach, draw_icon};

/// Jug geometry in stage units.
const JUG_X: f32 = 232.0;
const JUG_BOTTOM: f32 = 252.0;
const JUG_H: f32 = 170.0;
const HW_BOTTOM: f32 = 64.0;
const HW_TOP: f32 = 55.0;
/// Inner wall thickness of the glass.
const WALL: f32 = 5.0;
const BASE: f32 = 9.0;
const MILK0: f32 = 0.4;
const GAUGE: Rect = Rect {
    x: 394.0,
    y: 58.0,
    w: 16.0,
    h: 178.0,
};
const MAX_T: f32 = 18.0;

struct Puff {
    p: Vec2,
    v: Vec2,
    age: f32,
    life: f32,
    r: f32,
}

pub(crate) struct Froth {
    level: f32,
    band_c: f32,
    band_h: f32,
    mouse_hold: bool,
    holding: bool,
    momentum: f32,
    released_at: Option<f32>,
    hiss: f32,
    in_band: f32,
    noise_seed: u32,
    puffs: Vec<Puff>,
    spawn_acc: f32,
    rng: Rng,
    t: f32,
}

fn inner_hw(h: f32) -> f32 {
    HW_BOTTOM - WALL + (HW_TOP - HW_BOTTOM) * h
}

fn y_of(level: f32) -> f32 {
    JUG_BOTTOM - BASE - (JUG_H - BASE - 6.0) * level
}

impl Froth {
    pub fn new(rng: &mut Rng, touch: Option<RegularTouch>) -> Froth {
        let mut c = rng.range(0.66, 0.78);
        match touch {
            Some(RegularTouch::ExtraFoam) => c += 0.08,
            Some(RegularTouch::LightFoam) => c -= 0.09,
            _ => {}
        }
        Froth {
            level: MILK0,
            band_c: c.clamp(0.56, 0.86),
            band_h: 0.065,
            mouse_hold: false,
            holding: false,
            momentum: 0.0,
            released_at: None,
            hiss: 0.0,
            in_band: 0.0,
            noise_seed: rng.next_u64() as u32,
            puffs: Vec::new(),
            spawn_acc: 0.0,
            rng: Rng::new(rng.next_u64()),
            t: 0.0,
        }
    }

    pub fn hiss(&self) -> f32 {
        self.hiss
    }

    pub fn focus(&self) -> Vec2 {
        vec2(JUG_X, y_of(self.level) - 10.0)
    }

    fn band_contains(&self, l: f32, relaxed: bool) -> bool {
        (l - self.band_c).abs() <= self.band_h * super::score::widen(relaxed)
    }

    pub fn step(&mut self, ctx: &mut Ctx, input: &MgInput, dt: f32) -> Option<f32> {
        self.t = ctx.t;
        if input.pressed {
            self.mouse_hold = true;
        }
        if input.released || !input.down {
            self.mouse_hold = false;
        }
        let want = (self.mouse_hold || input.hold) && self.released_at.is_none();
        if want {
            if !self.holding {
                ctx.sounds.push(MiniGameSound::Froth);
            }
            self.holding = true;
            let n = rd::noise1(ctx.t * 0.8, self.noise_seed);
            let near_top = ((self.level - 0.86) / 0.14).clamp(0.0, 1.0);
            let rate = 0.056 * (0.8 + 0.4 * n) * (1.0 - 0.35 * near_top);
            self.level += rate * dt;
            self.momentum = rate;
        } else if self.holding {
            self.holding = false;
            if self.level > MILK0 + 0.1 {
                self.released_at = Some(ctx.t);
            }
        }
        if !self.holding {
            // the foam keeps stretching for a moment after the wand is lifted
            self.level += self.momentum * dt;
            self.momentum = approach(self.momentum, 0.0, dt, 0.12);
        }
        self.level = self.level.min(1.02);
        self.hiss = approach(self.hiss, if self.holding { 1.0 } else { 0.0 }, dt, 0.12);
        let inside = self.band_contains(self.level, ctx.relaxed);
        self.in_band = approach(self.in_band, if inside { 1.0 } else { 0.0 }, dt, 0.08);
        self.step_puffs(dt);

        let overflow = self.level >= 1.0 && self.released_at.is_none();
        if overflow {
            self.released_at = Some(ctx.t);
            self.holding = false;
        }
        let finish = match self.released_at {
            Some(r) => ctx.t - r > 0.55,
            None => ctx.t > MAX_T,
        };
        finish.then(|| froth_perf(self.level, self.band_c, self.band_h, ctx.relaxed))
    }

    fn step_puffs(&mut self, dt: f32) {
        let rate = 4.0 + 30.0 * self.hiss;
        self.spawn_acc += rate * dt;
        while self.spawn_acc >= 1.0 {
            self.spawn_acc -= 1.0;
            let from_wand = self.rng.f() < 0.5 * self.hiss;
            let p = if from_wand {
                vec2(JUG_X + 18.0 + self.rng.range(-6.0, 6.0), y_of(self.level) - 2.0)
            } else {
                vec2(
                    JUG_X + self.rng.range(-HW_TOP * 0.8, HW_TOP * 0.8),
                    y_of(1.0) + self.rng.range(-4.0, 8.0),
                )
            };
            self.puffs.push(Puff {
                p,
                v: vec2(self.rng.range(-6.0, 14.0), -self.rng.range(18.0, 36.0)),
                age: 0.0,
                life: self.rng.range(1.2, 2.2),
                r: self.rng.range(7.0, 13.0),
            });
        }
        for p in &mut self.puffs {
            p.age += dt;
            p.p += p.v * dt;
            p.v.x += (p.age * 3.0 + p.r).sin() * 6.0 * dt;
        }
        self.puffs.retain(|p| p.age < p.life);
    }

    pub fn autoplay(&self, t: f32) -> MgInput {
        let hold = t > 0.8 && self.released_at.is_none() && self.level < self.band_c - 0.012;
        MgInput {
            hold,
            ..Default::default()
        }
    }

    pub fn hint(&self) -> Hint<'static> {
        if self.holding && self.in_band > 0.5 {
            Hint {
                key: "minigame.hint.froth_release",
                emphasis: true,
                keys: &[],
                serve: false,
            }
        } else {
            Hint {
                key: "minigame.hint.froth",
                emphasis: false,
                keys: &["Space"],
                serve: false,
            }
        }
    }

    pub fn draw(&self, ui: &mut Ui, o: Vec2, t: f32) {
        let th = ui.theme.clone();
        let x = o.x + JUG_X;
        let bottom = o.y + JUG_BOTTOM;
        let top = bottom - JUG_H;

        draw_espresso(o + vec2(96.0, JUG_BOTTOM), t);
        contact_shadow(vec2(x + 4.0, bottom + 2.0), HW_BOTTOM + 26.0, 14.0, 0.55);

        // back of the glass
        let back = [
            vec2(x - HW_BOTTOM, bottom - 4.0),
            vec2(x - HW_TOP, top),
            vec2(x + HW_TOP, top),
            vec2(x + HW_BOTTOM, bottom - 4.0),
        ];
        fill_convex(
            &back,
            Paint::vertical(
                Rect::new(x, top, 1.0, JUG_H),
                Color::new(0.75, 0.72, 1.0, 0.05),
                Color::new(0.75, 0.72, 1.0, 0.1),
            ),
        );

        // steam wand (behind the milk, in front of the back wall)
        let tip = vec2(x + 18.0, o.y + y_of(MILK0 - 0.1));
        let root = vec2(x + 150.0, o.y - 30.0);
        rd::capsule(root, tip, 4.2, hex(0x8e96ad));
        rd::capsule(root + vec2(-1.4, 0.4), tip + vec2(-1.4, 0.0), 1.4, with_alpha(hex(0xe4e8f5), 0.8));
        rd::capsule(root, root + (tip - root) * 0.12, 6.0, hex(0x4a5068));

        // milk + foam
        let level = self.level.max(MILK0);
        let foam_amount = (level - MILK0).max(0.0);
        let milk_top = MILK0 - foam_amount * 0.22;
        let wob = 1.2 + 2.4 * self.hiss;
        let surface = |lv: f32, phase: f32, amp: f32| -> Vec<Vec2> {
            let y = o.y + y_of(lv);
            let hw = inner_hw(lv);
            let n = 18;
            (0..=n)
                .map(|i| {
                    let u = i as f32 / n as f32;
                    let xx = x - hw + 2.0 * hw * u;
                    let w = (u * 9.0 + t * 5.5 + phase).sin() * amp * (u * std::f32::consts::PI).sin();
                    vec2(xx, y + w)
                })
                .collect()
        };
        let body = |top_pts: Vec<Vec2>, lv0: f32| -> Vec<Vec2> {
            let y0 = o.y + y_of(lv0);
            let hw0 = inner_hw(lv0);
            let mut pts = vec![vec2(x - hw0, y0)];
            pts.extend(top_pts);
            pts.push(vec2(x + hw0, y0));
            pts
        };
        // milk
        let milk_poly = body(surface(milk_top, 0.0, 0.8), 0.0);
        fill_convex(
            &milk_poly,
            Paint::vertical(
                Rect::new(x, o.y + y_of(milk_top), 1.0, y_of(0.0) - y_of(milk_top)),
                hex(0xf3e9db),
                hex(0xdcc9b3),
            ),
        );
        // foam
        if level > milk_top + 0.005 {
            let foam_poly = body(surface(level, 1.3, wob), milk_top);
            let fy = o.y + y_of(level);
            fill_convex(
                &foam_poly,
                Paint::vertical(
                    Rect::new(x, fy, 1.0, (y_of(milk_top) - y_of(level)).max(1.0)),
                    hex(0xfffcf6),
                    hex(0xf4ece0),
                ),
            );
            // micro-bubbles: fine when polished
            let area_h = y_of(milk_top) - y_of(level);
            let count = (area_h * 0.9) as u32;
            for i in 0..count.min(90) {
                let u = rd::hash01(i * 31 + 7);
                let v = rd::hash01(i * 17 + 3);
                let yy = fy + 3.0 + v * (area_h - 5.0).max(0.0);
                let lv = (y_of(0.0) - (yy - o.y)) / (y_of(0.0) - y_of(1.0));
                let hw = inner_hw(lv) - 4.0;
                let xx = x - hw + 2.0 * hw * u;
                let r = 0.7 + rd::hash01(i * 7 + 1) * 1.6 * (1.0 - v * 0.5);
                draw::fill_circle(vec2(xx, yy), r, Color::new(0.85, 0.79, 0.72, 0.35));
                draw::fill_circle(vec2(xx - r * 0.3, yy - r * 0.3), r * 0.4, Color::new(1.0, 1.0, 1.0, 0.7));
            }
            // soft milk/foam seam
            let sy = o.y + y_of(milk_top);
            rd::rect_v(x - inner_hw(milk_top), sy - 3.0, inner_hw(milk_top) * 2.0, 6.0, Color::new(1.0, 1.0, 1.0, 0.0), Color::new(0.86, 0.78, 0.68, 0.25));
        }
        // whirl where the wand works
        if self.hiss > 0.05 {
            let sy = o.y + y_of(level);
            for k in 0..3 {
                let ph = t * 7.0 + k as f32 * 2.1;
                let rx = 10.0 + k as f32 * 7.0;
                rd::arc(
                    vec2(tip.x - 4.0, sy + 3.0),
                    rx,
                    3.0,
                    ph,
                    ph + 1.6,
                    1.3,
                    Color::new(0.78, 0.7, 0.62, 0.45 * self.hiss),
                );
            }
        }

        // sweet-spot band across the glass
        let b0 = o.y + y_of(self.band_c + self.band_h);
        let b1 = o.y + y_of(self.band_c - self.band_h);
        let glow_c = draw::mix(th.indigo_300, th.amber, self.in_band);
        let pulse = 0.5 + 0.5 * (t * 2.6).sin();
        let band_a = 0.09 + 0.05 * pulse + 0.14 * self.in_band;
        let hw_b = inner_hw(self.band_c) + WALL + 2.0;
        let mid = (b0 + b1) * 0.5;
        let hh = (b1 - b0) * 0.5 + 7.0;
        let on = with_alpha(glow_c, band_a * 1.6);
        let off = with_alpha(glow_c, 0.0);
        rd::rect_v(x - hw_b, mid - hh, hw_b * 2.0, hh, off, on);
        rd::rect_v(x - hw_b, mid, hw_b * 2.0, hh, on, off);
        for side in [-1.0f32, 1.0] {
            for y in [b0, b1] {
                let x0 = x + side * (inner_hw(self.band_c) + WALL + 3.0);
                draw::line(vec2(x0, y), vec2(x0 + side * 9.0, y), 1.6, with_alpha(glow_c, 0.55 + 0.4 * self.in_band));
            }
        }
        if self.in_band > 0.02 {
            rd::glow(x, (b0 + b1) * 0.5, hw_b * 1.3, with_alpha(th.amber, 0.12 * self.in_band));
        }

        // front glass
        self.draw_glass_front(x, top, bottom, t);

        // steam
        for p in &self.puffs {
            let a = p.age / p.life;
            let fade = (a * (1.0 - a) * 4.0).clamp(0.0, 1.0);
            rd::glow(o.x + p.p.x, o.y + p.p.y, p.r * (1.0 + a * 1.4), Color::new(0.94, 0.93, 1.0, 0.2 * fade));
        }

        // gauge
        let g = GAUGE.offset(o);
        fill_rrect(g, g.w / 2.0, Color::new(0.02, 0.01, 0.07, 0.55));
        stroke_rrect(g, g.w / 2.0, 1.0, Color::new(1.0, 1.0, 1.0, 0.09));
        let inner = Rect::new(g.x + 3.0, g.y + 3.0, g.w - 6.0, g.h - 6.0);
        let gy = |l: f32| inner.bottom() - inner.h * ((l - MILK0 + 0.08) / (1.0 - MILK0 + 0.08)).clamp(0.0, 1.0);
        let by0 = gy(self.band_c + self.band_h);
        let by1 = gy(self.band_c - self.band_h);
        let band_r = Rect::new(g.x - 3.0, by0, g.w + 6.0, by1 - by0);
        fill_rrect(band_r, 4.0, with_alpha(glow_c, 0.22 + 0.25 * self.in_band));
        let ly = gy(level);
        let f = Rect::new(inner.x, ly, inner.w, inner.bottom() - ly);
        if f.h > 1.0 {
            fill_rrect(f, inner.w / 2.0, Paint::vertical(f, hex(0xfffaf2), hex(0xcdbca8)));
        }
        glow_circle(vec2(g.center().x, ly), 12.0, with_alpha(draw::mix(th.indigo_300, th.amber, self.in_band), 0.5));
        draw::fill_circle(vec2(g.center().x, ly), 3.0, WHITE);
        let hc = vec2(g.right() + 16.0, (by0 + by1) * 0.5);
        glow_circle(hc, 14.0, with_alpha(glow_c, 0.25 + 0.3 * self.in_band));
        draw_icon(Icon::Heart, hc, 14.0, with_alpha(glow_c, 0.75 + 0.25 * self.in_band));
    }

    fn draw_glass_front(&self, x: f32, top: f32, bottom: f32, t: f32) {
        // base: thick glass foot
        let base = Rect::new(x - HW_BOTTOM, bottom - BASE - 2.0, HW_BOTTOM * 2.0, BASE + 2.0);
        fill_rrect(base, 6.0, Paint::vertical(base, Color::new(0.82, 0.8, 1.0, 0.22), Color::new(0.6, 0.56, 0.9, 0.32)));
        draw::hline(base.x + 8.0, base.right() - 8.0, base.y + 1.0, Color::new(1.0, 1.0, 1.0, 0.35));
        // walls
        for side in [-1.0f32, 1.0] {
            let wall = [
                vec2(x + side * HW_BOTTOM, bottom - 4.0),
                vec2(x + side * HW_TOP, top),
                vec2(x + side * (HW_TOP - WALL), top),
                vec2(x + side * (HW_BOTTOM - WALL), bottom - 4.0),
            ];
            let c = if side < 0.0 {
                Color::new(0.92, 0.92, 1.0, 0.28)
            } else {
                Color::new(0.78, 0.74, 1.0, 0.4)
            };
            fill_convex(&wall, Paint::Solid(c));
        }
        // specular streak
        let s = [
            vec2(x - HW_BOTTOM + 13.0, bottom - 22.0),
            vec2(x - HW_TOP + 11.0, top + 14.0),
            vec2(x - HW_TOP + 18.0, top + 14.0),
            vec2(x - HW_BOTTOM + 21.0, bottom - 22.0),
        ];
        fill_convex(
            &s,
            Paint::vertical(Rect::new(x, top, 1.0, bottom - top), Color::new(1.0, 1.0, 1.0, 0.3), Color::new(1.0, 1.0, 1.0, 0.04)),
        );
        // rim
        rd::ellipse(x, top, HW_TOP + 1.0, 4.0, Color::new(0.9, 0.9, 1.0, 0.22));
        draw::stroke_path(
            &ellipse_arc(vec2(x, top), HW_TOP, 4.0, 0.0, std::f32::consts::PI),
            false,
            1.6,
            Paint::Solid(Color::new(1.0, 1.0, 1.0, 0.5)),
        );
        // spout
        fill_convex(
            &[vec2(x - HW_TOP - 14.0, top - 7.0), vec2(x - HW_TOP + 6.0, top - 2.0), vec2(x - HW_TOP + 2.0, top + 9.0)],
            Paint::Solid(Color::new(0.88, 0.88, 1.0, 0.32)),
        );
        // handle
        let hc = vec2(x + HW_TOP + 16.0, top + 60.0);
        draw::arc(hc, 30.0, -1.45, 1.45, 8.0, Color::new(0.8, 0.77, 1.0, 0.3));
        draw::arc(hc + vec2(1.0, 0.0), 32.0, -1.0, 0.4, 1.6, Color::new(1.0, 1.0, 1.0, 0.35));
        // lavender rim light on the right edge
        let shimmer = 0.75 + 0.25 * (t * 1.3).sin();
        draw::line(
            vec2(x + HW_BOTTOM - 1.5, bottom - 10.0),
            vec2(x + HW_TOP - 1.5, top + 6.0),
            1.5,
            Color::new(0.8, 0.76, 1.0, 0.55 * shimmer),
        );
    }
}

fn hex(h: u32) -> Color {
    Color::from_hex(h)
}

/// The espresso waiting for its milk (side view, 3/4 top).
fn draw_espresso(base: Vec2, t: f32) {
    contact_shadow(base + vec2(3.0, 1.0), 46.0, 9.0, 0.5);
    // saucer
    rd::ellipse_v(base.x, base.y - 3.0, 42.0, 8.0, hex(0xe9e2f4), hex(0xa99dc6));
    rd::ellipse(base.x, base.y - 5.0, 30.0, 5.0, hex(0xcfc6e2));
    // cup body
    let w = 46.0;
    let h = 36.0;
    let body = Rect::new(base.x - w / 2.0, base.y - 6.0 - h, w, h);
    fill_rrect(body, 12.0, Paint::horizontal(body, hex(0xfdfbff), hex(0xbcb0d4)));
    // handle
    draw::arc(vec2(body.right() + 2.0, body.y + 14.0), 9.0, -1.4, 1.4, 4.5, hex(0xd7cfe6));
    // rim + crema
    rd::ellipse(base.x, body.y + 1.0, w / 2.0, 6.0, hex(0xf6f2fb));
    rd::ellipse_v(base.x, body.y + 1.5, w / 2.0 - 3.5, 4.2, hex(0xb07540), hex(0x6a3a20));
    rd::ellipse(base.x - 5.0, body.y + 0.5, 6.0, 1.6, Color::new(0.95, 0.75, 0.5, 0.45));
    // a thin wisp of steam
    for i in 0..2u32 {
        let life = 2.6;
        let a = ((t + i as f32 * 1.3) % life) / life;
        let fade = (a * (1.0 - a) * 4.0).clamp(0.0, 1.0);
        rd::glow(base.x + (t * 1.2 + i as f32).sin() * 4.0, body.y - 8.0 - a * 34.0, 8.0 + a * 10.0, Color::new(0.95, 0.94, 1.0, 0.14 * fade));
    }
}
