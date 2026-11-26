//! TeaSteep: a tea bag in a glass mug, the colour slowly deepens; lift the bag
//! (click / Space / Enter) when it matches the swatch. The timer is generous.

use macroquad::prelude::*;

use super::paint::{contact_shadow, ellipse_arc, hex, with_alpha};
use super::score::{tea_perf, widen};
use super::{Ctx, Hint, MgInput, MiniGameSound, MiniGameText, RegularTouch, Rng, TeaKind};
use crate::render::draw as rd;
use crate::ui::draw::{self, Paint, fill_circle, fill_convex, fill_rrect, glow_circle, stroke_circle};
use crate::ui::{Glass, Ui, approach, draw_glass, ease, ease_out};

const MX: f32 = 176.0;
const BOTTOM: f32 = 256.0;
const H: f32 = 178.0;
const HW: f32 = 68.0;
const WATER: f32 = 0.8;
const DUNK_T: f32 = 0.7;
const LIFT_ANIM: f32 = 0.7;
const PANEL: Rect = Rect {
    x: 334.0,
    y: 30.0,
    w: 142.0,
    h: 222.0,
};

struct Wisp {
    p: Vec2,
    v: Vec2,
    age: f32,
    life: f32,
    r: f32,
}

pub(crate) struct Tea {
    kind: TeaKind,
    target_t: f32,
    tau: f32,
    lifted_at: Option<f32>,
    perf: Option<f32>,
    wisps: Vec<Wisp>,
    acc: f32,
    rng: Rng,
    match_t: f32,
    t: f32,
}

/// (deep colour, glow tint)
fn tea_colors(k: TeaKind) -> Color {
    match k {
        TeaKind::Black => hex(0x6e2410),
        TeaKind::Green => hex(0x7f9a2e),
        TeaKind::Herbal => hex(0xc07d10),
        TeaKind::Fruit => hex(0x8f1530),
    }
}

const CLEAR: Color = Color::new(0.86, 0.85, 0.96, 1.0);

fn y_of(l: f32) -> f32 {
    BOTTOM - 10.0 - (H - 16.0) * l
}

impl Tea {
    pub fn new(rng: &mut Rng, kind: TeaKind, touch: Option<RegularTouch>) -> Tea {
        let mut target = rng.range(7.5, 10.0);
        if kind == TeaKind::Green {
            target -= 1.0;
        }
        match touch {
            Some(RegularTouch::Strong) => target += 2.5,
            Some(RegularTouch::Mild) => target -= 2.0,
            _ => {}
        }
        Tea {
            kind,
            target_t: target,
            tau: target / 1.35,
            lifted_at: None,
            perf: None,
            wisps: Vec::new(),
            acc: 0.0,
            rng: Rng::new(rng.next_u64()),
            match_t: 0.0,
            t: 0.0,
        }
    }

    pub fn focus(&self) -> Vec2 {
        vec2(MX, y_of(WATER) - 20.0)
    }

    fn steep_time(&self, t: f32) -> f32 {
        let end = self.lifted_at.unwrap_or(t);
        (end.min(t) - DUNK_T).max(0.0)
    }

    fn strength(&self, steep: f32) -> f32 {
        1.0 - (-steep / self.tau).exp()
    }

    fn color_at(&self, s: f32) -> Color {
        let deep = tea_colors(self.kind);
        let c = draw::mix(CLEAR, deep, s.clamp(0.0, 1.0).powf(0.8));
        Color { a: 1.0, ..c }
    }

    fn bag_lift(&self, t: f32) -> f32 {
        match self.lifted_at {
            Some(l) => ease((t - l) / LIFT_ANIM),
            None => 0.0,
        }
    }

    pub fn step(&mut self, ctx: &mut Ctx, input: &MgInput, dt: f32) -> Option<f32> {
        self.t = ctx.t;
        let steep = self.steep_time(ctx.t);
        let close = (steep - self.target_t).abs() < 0.8 * widen(ctx.relaxed);
        self.match_t = approach(self.match_t, if close && self.lifted_at.is_none() { 1.0 } else { 0.0 }, dt, 0.2);
        // colour wisps drifting from the bag while it is in the water
        if self.lifted_at.is_none() && ctx.t > DUNK_T {
            let fresh = (1.0 - self.strength(steep)).max(0.15);
            self.acc += dt * 14.0 * fresh;
            while self.acc >= 1.0 {
                self.acc -= 1.0;
                let b = self.bag_pos(ctx.t);
                self.wisps.push(Wisp {
                    p: b + vec2(self.rng.range(-12.0, 12.0), self.rng.range(0.0, 16.0)),
                    v: vec2(self.rng.range(-16.0, 16.0), self.rng.range(4.0, 22.0)),
                    age: 0.0,
                    life: self.rng.range(1.6, 2.8),
                    r: self.rng.range(8.0, 15.0),
                });
            }
        }
        for w in &mut self.wisps {
            w.age += dt;
            w.p += w.v * dt;
            w.v *= (-0.8 * dt).exp();
            w.p.x = w.p.x.clamp(MX - HW + 14.0, MX + HW - 14.0);
            w.p.y = w.p.y.min(y_of(0.05));
        }
        self.wisps.retain(|w| w.age < w.life);

        let act = input.pressed || input.hold_pressed || input.confirm;
        let auto = steep > self.target_t + 7.0;
        if self.lifted_at.is_none() && ((act && ctx.t > DUNK_T + 0.3) || auto) {
            self.lifted_at = Some(ctx.t);
            self.perf = Some(tea_perf(steep, self.target_t, ctx.relaxed));
            ctx.sounds.push(MiniGameSound::Pour);
        }
        match self.lifted_at {
            Some(l) if ctx.t - l > LIFT_ANIM + 0.35 => {
                ctx.sounds.push(MiniGameSound::Clink);
                self.perf
            }
            _ => None,
        }
    }

    pub fn autoplay(&self, t: f32) -> MgInput {
        let steep = self.steep_time(t);
        MgInput {
            pressed: self.lifted_at.is_none() && steep >= self.target_t + 0.1,
            ..Default::default()
        }
    }

    pub fn hint(&self) -> Hint<'static> {
        if self.match_t > 0.5 {
            Hint {
                key: "minigame.hint.tea_ready",
                emphasis: true,
                keys: &["Space"],
                serve: false,
            }
        } else {
            Hint {
                key: "minigame.hint.tea",
                emphasis: false,
                keys: &["Space"],
                serve: false,
            }
        }
    }

    fn bag_pos(&self, t: f32) -> Vec2 {
        let dunk = ease_out(t / DUNK_T);
        let sway = (t * 0.9).sin() * 4.0;
        let rest = vec2(MX - 6.0 + sway, y_of(0.32));
        let above = vec2(MX + 4.0, y_of(1.12));
        let lifted = vec2(MX + 10.0, y_of(1.18));
        let p = above.lerp(rest, dunk);
        p.lerp(lifted, self.bag_lift(t))
    }

    pub fn draw(&self, ui: &mut Ui, o: Vec2, t: f32, text: &MiniGameText) {
        let x = o.x + MX;
        let bottom = o.y + BOTTOM;
        let top = bottom - H;
        let steep = self.steep_time(t);
        let s = self.strength(steep);
        let col = self.color_at(s);

        contact_shadow(vec2(x + 4.0, bottom + 2.0), HW + 28.0, 13.0, 0.55);
        // warm light passing through the tea onto the counter
        rd::radial(x + 18.0, bottom + 4.0, HW + 20.0, 12.0, with_alpha(col, 0.35 * s), with_alpha(col, 0.0));

        // back of the glass
        let body = Rect::new(x - HW, top, HW * 2.0, H);
        fill_rrect(body, 18.0, Color::new(0.78, 0.75, 1.0, 0.07));

        // tea: denser at the bottom where it settles
        let wy = o.y + y_of(WATER);
        let wb = bottom - 10.0;
        let water = Rect::new(x - HW + 6.0, wy, HW * 2.0 - 12.0, wb - wy);
        let top_c = self.color_at(s * 0.82);
        let bot_c = self.color_at((s * 1.12).min(1.0));
        let a = 0.38 + 0.5 * s;
        fill_rrect(water, 13.0, Paint::vertical(water, with_alpha(top_c, a), with_alpha(bot_c, a + 0.06)));
        for w in &self.wisps {
            let k = w.age / w.life;
            let fade = (k * (1.0 - k) * 4.0).clamp(0.0, 1.0);
            let deep = tea_colors(self.kind);
            rd::glow(o.x + w.p.x, o.y + w.p.y, w.r * (0.8 + k * 1.8), with_alpha(deep, 0.32 * fade));
        }
        // surface
        rd::ellipse(x, wy, HW - 6.0, 5.0, with_alpha(draw::mix(top_c, WHITE, 0.25), a));
        draw::hline(x - HW + 14.0, x + HW - 14.0, wy - 1.0, Color::new(1.0, 1.0, 1.0, 0.25));

        // tea bag + string + tag
        let bp = o + self.bag_pos(t);
        let rim_pt = vec2(x + HW - 10.0, top + 2.0);
        let tag_c = vec2(x + HW + 14.0, top + 46.0 + (t * 1.1).sin() * 1.5);
        draw::line(bp + vec2(0.0, -16.0), rim_pt, 1.0, Color::new(0.95, 0.93, 0.88, 0.85));
        draw::line(rim_pt, tag_c + vec2(0.0, -12.0), 1.0, Color::new(0.95, 0.93, 0.88, 0.85));
        let bag = Rect::new(bp.x - 15.0, bp.y - 18.0, 30.0, 36.0);
        let submerged = bp.y > wy;
        let bag_top = draw::mix(hex(0xf1e6cf), hex(0xc9b48f), if submerged { 0.4 + 0.4 * s } else { 0.1 });
        fill_rrect(bag, 5.0, Paint::vertical(bag, bag_top, draw::mix(bag_top, tea_colors(self.kind), 0.35)));
        // leaves inside the bag
        for i in 0..10u32 {
            let lp = vec2(bag.x + 6.0 + rd::hash01(i * 3 + 1) * 18.0, bag.y + 14.0 + rd::hash01(i * 5 + 2) * 18.0);
            fill_circle(lp, 1.6 + rd::hash01(i) * 1.2, with_alpha(draw::mix(tea_colors(self.kind), hex(0x2a160c), 0.4), 0.55));
        }
        // fold at the top
        fill_convex(
            &[bag.point(), vec2(bag.right(), bag.y), vec2(bag.center().x, bag.y + 7.0)],
            Paint::Solid(with_alpha(WHITE, 0.25)),
        );
        // drips when lifted
        if let Some(l) = self.lifted_at {
            let age = t - l;
            for k in 0..3 {
                let ph = ((age * 1.6) + k as f32 * 0.33).fract();
                let dp = vec2(bag.center().x + (k as f32 - 1.0) * 6.0, bag.bottom() + ph * (wy - bag.bottom()).max(4.0));
                fill_circle(dp, 1.8, with_alpha(tea_colors(self.kind), 0.8 * (1.0 - ph)));
            }
        }
        // tag
        let tag = Rect::new(tag_c.x - 9.0, tag_c.y - 12.0, 18.0, 22.0);
        fill_rrect(tag, 3.0, Paint::vertical(tag, hex(0xfaf3e6), hex(0xe3d3b8)));
        fill_circle(vec2(tag.center().x, tag.y + 13.0), 4.0, with_alpha(tea_colors(self.kind), 0.85));

        // front glass
        for (side, c) in [(-1.0f32, Color::new(0.92, 0.92, 1.0, 0.24)), (1.0, Color::new(0.78, 0.74, 1.0, 0.36))] {
            let wx = x + side * (HW - 3.0);
            let r = Rect::new(wx - 3.0, top + 6.0, 6.0, H - 22.0);
            fill_rrect(r, 3.0, c);
        }
        let base = Rect::new(x - HW + 2.0, bottom - 12.0, HW * 2.0 - 4.0, 12.0);
        fill_rrect(base, 8.0, Paint::vertical(base, Color::new(0.85, 0.82, 1.0, 0.25), Color::new(0.6, 0.56, 0.9, 0.35)));
        rd::ellipse(x, top + 2.0, HW, 5.0, Color::new(0.9, 0.9, 1.0, 0.18));
        draw::stroke_path(&ellipse_arc(vec2(x, top + 2.0), HW - 1.0, 5.0, 0.0, std::f32::consts::PI), false, 1.5, Paint::Solid(Color::new(1.0, 1.0, 1.0, 0.45)));
        let streak = Rect::new(x - HW + 14.0, top + 20.0, 8.0, H - 60.0);
        fill_rrect(streak, 4.0, Paint::vertical(streak, Color::new(1.0, 1.0, 1.0, 0.3), Color::new(1.0, 1.0, 1.0, 0.03)));
        // handle (left, so the tag can hang on the right)
        let hc = vec2(x - HW - 14.0, top + 78.0);
        draw::arc(hc, 28.0, 1.75, 4.55, 9.0, Color::new(0.8, 0.77, 1.0, 0.28));
        draw::arc(hc + vec2(-1.0, 0.0), 30.0, 2.4, 3.9, 1.6, Color::new(1.0, 1.0, 1.0, 0.35));

        // steam
        for i in 0..4u32 {
            let life = 2.4;
            let off = i as f32 / 4.0 * life;
            let a = ((t + off) % life) / life;
            let sx = x - 30.0 + i as f32 * 20.0 + (t * 1.4 + i as f32).sin() * 6.0;
            let sy = top - 6.0 - a * 50.0;
            let fade = (a * (1.0 - a) * 4.0).clamp(0.0, 1.0);
            rd::glow(sx, sy, 12.0 + a * 16.0, Color::new(0.95, 0.94, 1.0, 0.16 * fade));
        }

        self.draw_panel(ui, o, t, text, col);
    }

    fn draw_panel(&self, ui: &mut Ui, o: Vec2, t: f32, text: &MiniGameText, now: Color) {
        let th = ui.theme.clone();
        let r = PANEL.offset(o);
        draw_glass(&th, r, &Glass::card());
        let micro = th.micro();
        ui.fonts.draw(&text.get("minigame.target").to_uppercase(), r.x + 12.0, r.y + 20.0, &micro);
        let st = th.strong();
        let name = ui.fonts.ellipsize(text.get(self.kind.key()), r.w - 24.0, &st).into_owned();
        ui.fonts.draw(&name, r.x + 12.0, r.y + 38.0, &st);
        // swatch: the target colour in the middle, the current colour as a ring
        let c = vec2(r.center().x, r.y + 120.0);
        let target = self.color_at(self.strength(self.target_t));
        glow_circle(c, 64.0, with_alpha(target, 0.25 + 0.2 * self.match_t));
        fill_circle(c, 46.0, now);
        stroke_circle(c, 46.0, 1.2, Color::new(1.0, 1.0, 1.0, 0.25));
        fill_circle(c, 30.0, Color::new(0.06, 0.03, 0.1, 0.4));
        fill_circle(c, 28.0, target);
        rd::glow(c.x - 8.0, c.y - 10.0, 18.0, Color::new(1.0, 1.0, 1.0, 0.18));
        if self.match_t > 0.02 {
            let pulse = 0.5 + 0.5 * (t * 5.0).sin();
            stroke_circle(c, 52.0 + pulse * 3.0, 2.0, with_alpha(th.amber, 0.7 * self.match_t));
        }
        // steep progress: soft dots (no countdown pressure)
        let steep = self.steep_time(t);
        let n = 8;
        let filled = ((steep / (self.target_t * 1.25)) * n as f32).clamp(0.0, n as f32);
        let y = r.bottom() - 28.0;
        for i in 0..n {
            let dx = r.x + 22.0 + i as f32 * ((r.w - 44.0) / (n - 1) as f32);
            let k = (filled - i as f32).clamp(0.0, 1.0);
            fill_circle(vec2(dx, y), 4.0, Color::new(1.0, 1.0, 1.0, 0.1));
            if k > 0.0 {
                fill_circle(vec2(dx, y), 4.0 * k, with_alpha(draw::mix(th.indigo_300, th.amber, k), 0.9));
            }
        }
    }
}
