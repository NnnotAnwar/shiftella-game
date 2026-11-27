//! MilkFroth (§2.1): hold LMB / Space to steam the milk in a glass pitcher. The
//! temperature needle climbs a dial; foam rises while steaming and gets an extra puff of
//! air on every re-press (pulsing). Let go when the needle sits in the amber band.
//! Quality: `w × clamp(1 − |T − T_target| / tol) + (1 − w) × clamp(1 − |foam − target| / foam_tol)`
//! (w = 0.75). Each drink is itself: espresso or cocoa in the cup, milk or cream in the jug,
//! a thick / silky / light foam line; Raf gets a vanilla drop first, Lori's encore a chili pinch
//! over the cup last (a swinging drop, its share of the quality from the content row). The
//! target drifts a little every play.

use std::f32::consts::PI;

use macroquad::prelude::*;

use super::paint::{contact_shadow, ellipse_arc, with_alpha};
use super::score::{self, widen};
use super::{
    Common, Ctx, FrothDrink, FrothDrop, FrothTuning, Hint, MgInput, MiniGameSound, MiniGameText,
    Rng,
};
use crate::render::draw as rd;
use crate::ui::draw::{self, Paint, fill_convex, fill_rrect, glow_circle};
use crate::ui::{Align, Ui, approach};

/// Jug geometry in stage units.
const JUG_X: f32 = 210.0;
const JUG_BOTTOM: f32 = 228.0;
const JUG_H: f32 = 150.0;
const HW_BOTTOM: f32 = 58.0;
const HW_TOP: f32 = 50.0;
/// Inner wall thickness of the glass.
const WALL: f32 = 5.0;
const BASE: f32 = 9.0;
/// Milk level before foaming (0..1 of the jug).
const MILK0: f32 = 0.4;
/// Foam 1.0 adds this much to the level.
const FOAM_RISE: f32 = 0.5;
const DIAL_C: Vec2 = Vec2::new(404.0, 118.0);
const DIAL_R: f32 = 54.0;
const T_START: f32 = 18.0;
const T_RATE: f32 = 6.5;
const FOAM_RATE: f32 = 0.05;
const FOAM_PULSE: f32 = 0.06;
const MAX_T: f32 = 14.0;

struct Puff {
    p: Vec2,
    v: Vec2,
    age: f32,
    life: f32,
    r: f32,
}

/// The swinging drop (vanilla over the jug first, chili over the cup last).
#[derive(Clone, Copy, Debug, PartialEq)]
enum DropState {
    /// Not yet swinging (chili waits for the milk).
    Waiting,
    Swinging {
        since: f32,
    },
    Dropped {
        acc: f32,
        at: f32,
    },
}

pub(crate) struct Froth {
    tu: FrothTuning,
    drop: Option<(FrothDrop, f32)>,
    drop_state: DropState,
    swing_phase: f32,
    swing_speed: f32,
    cup_x: f32,
    common: Common,
    temp: f32,
    foam: f32,
    mouse_hold: bool,
    holding: bool,
    last_press: f32,
    started: bool,
    peak: f32,
    released_for: f32,
    hiss: f32,
    in_band: f32,
    was_in_band: bool,
    puffs: Vec<Puff>,
    spawn_acc: f32,
    rng: super::Rng,
    t: f32,
}

fn inner_hw(h: f32) -> f32 {
    HW_BOTTOM - WALL + (HW_TOP - HW_BOTTOM) * h
}

fn y_of(level: f32) -> f32 {
    JUG_BOTTOM - BASE - (JUG_H - BASE - 6.0) * level
}

fn dial_angle(temp: f32) -> f32 {
    let u = ((temp - 20.0) / 60.0).clamp(0.0, 1.0);
    (160.0 + 220.0 * u).to_radians()
}

const DROP_SWING: f32 = 50.0;
const DROP_Y: f32 = 34.0;

impl Froth {
    pub fn new(rng: &mut Rng, mut tu: FrothTuning, common: Common) -> Froth {
        // a little drift every play: the band and the foam line are never in the same place
        tu.t_target = (tu.t_target + rng.range(-1.5, 1.5)).round();
        tu.foam = (tu.foam + rng.range(-0.04, 0.04)).clamp(0.15, 0.9);
        let drop = tu.drop;
        Froth {
            tu,
            drop,
            drop_state: match drop {
                Some((FrothDrop::Vanilla, _)) => DropState::Swinging { since: 0.3 },
                _ => DropState::Waiting,
            },
            swing_phase: rng.range(0.0, std::f32::consts::TAU),
            swing_speed: rng.range(1.4, 1.9),
            cup_x: 78.0 + rng.range(-8.0, 8.0),
            common,
            temp: T_START,
            foam: 0.0,
            mouse_hold: false,
            holding: false,
            last_press: -10.0,
            started: false,
            peak: T_START,
            released_for: 0.0,
            hiss: 0.0,
            in_band: 0.0,
            was_in_band: false,
            puffs: Vec::new(),
            spawn_acc: 0.0,
            rng: Rng::new(rng.next_u64()),
            t: 0.0,
        }
    }

    /// Where the drop swings (x), and over what (the jug for vanilla, the cup for chili).
    fn drop_center(&self) -> f32 {
        match self.drop {
            Some((FrothDrop::Chili, _)) => self.cup_x,
            _ => JUG_X,
        }
    }

    fn drop_x(&self, t: f32) -> f32 {
        let amp = if matches!(self.drop, Some((FrothDrop::Chili, _))) {
            DROP_SWING * 0.8
        } else {
            DROP_SWING
        };
        self.drop_center() + (t * self.swing_speed + self.swing_phase).sin() * amp
    }

    fn drop_acc(&self) -> f32 {
        match self.drop_state {
            DropState::Dropped { acc, .. } => acc,
            _ => 1.0,
        }
    }

    fn drop_pending(&self) -> bool {
        self.drop.is_some() && !matches!(self.drop_state, DropState::Dropped { .. })
    }

    fn temp_score(&self, relaxed: bool) -> f32 {
        (1.0 - (self.temp - self.tu.t_target).abs() / (self.tu.tol * widen(relaxed)))
            .clamp(0.0, 1.0)
    }

    fn foam_score(&self, relaxed: bool) -> f32 {
        (1.0 - (self.foam - self.tu.foam).abs() / (self.tu.foam_tol * widen(relaxed)))
            .clamp(0.0, 1.0)
    }

    pub fn steps(&self) -> Vec<&'static str> {
        match self.drop {
            Some((FrothDrop::Vanilla, _)) => vec!["milk_froth.vanilla"],
            Some((FrothDrop::Chili, _)) => vec!["milk_froth.chili"],
            None => Vec::new(),
        }
    }

    pub fn preview(&self, relaxed: bool) -> f32 {
        self.quality(relaxed)
    }

    pub fn advice(&self, relaxed: bool) -> &'static str {
        let w = self.tu.t_weight;
        let dt = w * (1.0 - self.temp_score(relaxed));
        let df = (1.0 - w) * (1.0 - self.foam_score(relaxed));
        let dd = self
            .drop
            .map_or(0.0, |(_, dw)| dw * (1.0 - self.drop_acc()));
        if dd > dt && dd > df {
            "minigame.why.drop"
        } else if df > dt {
            if self.foam < self.tu.foam {
                "minigame.why.froth.foam_low"
            } else {
                "minigame.why.froth.foam_high"
            }
        } else if self.temp < self.tu.t_target {
            "minigame.why.froth.cold"
        } else {
            "minigame.why.froth.hot"
        }
    }

    /// `sfx_coffee_machine` loop volume: 0.3 → 0.8 with temperature while steaming.
    pub fn steam_volume(&self) -> f32 {
        self.hiss * (0.3 + 0.5 * ((self.temp - 20.0) / 60.0).clamp(0.0, 1.0))
    }

    pub fn focus(&self) -> Vec2 {
        vec2(JUG_X, y_of(self.level()) - 10.0)
    }

    fn level(&self) -> f32 {
        (MILK0 + self.foam * FOAM_RISE).min(1.02)
    }

    fn band(&self, relaxed: bool) -> (f32, f32) {
        let tol = self.tu.tol * widen(relaxed);
        (self.tu.t_target - tol, self.tu.t_target + tol)
    }

    fn quality(&self, relaxed: bool) -> f32 {
        let q = score::froth_weighted(
            self.temp,
            self.tu.t_target,
            self.tu.tol,
            self.foam,
            self.tu.foam,
            self.tu.foam_tol,
            self.tu.t_weight,
            relaxed,
        );
        match self.drop {
            Some((_, w)) => (1.0 - w) * q + w * self.drop_acc(),
            None => q,
        }
    }

    /// Released in or above the band (the gesture is over).
    pub fn finished_gesture(&self) -> bool {
        !self.holding && self.temp >= self.band(self.common.relaxed).0 && !self.drop_pending()
    }

    /// The swinging drop: click / Space while it hangs over the middle.
    fn step_drop(&mut self, ctx: &mut Ctx, input: &MgInput) {
        if let DropState::Swinging { since } = self.drop_state
            && ctx.t > since + 0.25
        {
            let auto = !self.common.relaxed && ctx.t > since + 7.0;
            if input.pressed || input.hold_pressed || auto {
                let x = self.drop_x(ctx.t);
                let acc = score::lemon(x - self.drop_center(), ctx.relaxed);
                self.drop_state = DropState::Dropped { acc, at: ctx.t };
                ctx.sounds.push(MiniGameSound::Pour);
                if acc > 0.6 {
                    ctx.sounds.push(MiniGameSound::Cue);
                }
            }
        }
    }

    pub fn step(&mut self, ctx: &mut Ctx, input: &MgInput, dt: f32) -> Option<f32> {
        self.t = ctx.t;
        let swinging = matches!(self.drop_state, DropState::Swinging { .. });
        if swinging {
            self.step_drop(ctx, input);
            // the vanilla goes in before the steam: no steaming while it swings
            if matches!(self.drop, Some((FrothDrop::Vanilla, _))) {
                self.temp = (self.temp - 0.4 * dt).max(T_START);
                self.step_puffs(dt);
                return None;
            }
        }
        if input.pressed && !swinging {
            self.mouse_hold = true;
        }
        if input.released || !input.down {
            self.mouse_hold = false;
        }
        let want = !swinging && (self.mouse_hold || input.hold);
        if want {
            if !self.holding {
                ctx.sounds.push(MiniGameSound::Froth);
                // every re-press pulls in a little air
                if ctx.t - self.last_press > 0.12 {
                    self.foam += FOAM_PULSE;
                }
                self.last_press = ctx.t;
            }
            self.holding = true;
            self.started = true;
            self.released_for = 0.0;
            self.temp += T_RATE * dt;
            self.foam += FOAM_RATE * dt;
        } else {
            self.holding = false;
            self.released_for += dt;
            // the jug waits on the counter while the chili swings: it keeps its warmth
            if self.drop_state == DropState::Waiting
                || matches!(self.drop, Some((FrothDrop::Vanilla, _)))
            {
                self.temp = (self.temp - 0.4 * dt).max(T_START);
            }
        }
        self.foam = self.foam.clamp(0.0, 1.2);
        self.peak = self.peak.max(self.temp);
        self.hiss = approach(self.hiss, if self.holding { 1.0 } else { 0.0 }, dt, 0.1);
        let (lo, hi) = self.band(ctx.relaxed);
        let inside = (lo..=hi).contains(&self.temp);
        if inside && !self.was_in_band {
            ctx.sounds.push(MiniGameSound::Cue);
        }
        self.was_in_band = inside;
        self.in_band = approach(self.in_band, if inside { 1.0 } else { 0.0 }, dt, 0.08);
        self.step_puffs(dt);

        // chili: once the milk is done, the pinch swings over the cup
        let milk_done = self.temp >= lo && self.released_for > 0.6;
        if self.drop_state == DropState::Waiting
            && self.drop.is_some()
            && (milk_done || (self.common.relaxed && input.confirm && self.temp >= lo))
        {
            self.drop_state = DropState::Swinging { since: ctx.t };
            return None;
        }
        let drop_wait = match self.drop_state {
            DropState::Dropped { at, .. } => ctx.t - at < 0.6,
            DropState::Waiting => false,
            DropState::Swinging { .. } => true,
        };
        let finish = if self.common.relaxed {
            input.confirm && !matches!(self.drop_state, DropState::Swinging { .. })
        } else {
            let scalded = self.temp > hi + 12.0;
            let max_t = MAX_T + if self.drop.is_some() { 5.0 } else { 0.0 };
            (milk_done && !drop_wait && !self.drop_pending())
                || (scalded && !self.drop_pending())
                || ctx.t > max_t
                || (input.confirm && self.temp > T_START + 5.0 && !self.drop_pending())
        };
        if finish {
            self.holding = false;
            return Some(self.quality(ctx.relaxed));
        }
        None
    }

    fn step_puffs(&mut self, dt: f32) {
        let rate = 3.0 + 28.0 * self.hiss;
        self.spawn_acc += rate * dt;
        while self.spawn_acc >= 1.0 {
            self.spawn_acc -= 1.0;
            let from_wand = self.rng.f() < 0.5 * self.hiss;
            let p = if from_wand {
                vec2(
                    JUG_X + 16.0 + self.rng.range(-6.0, 6.0),
                    y_of(self.level()) - 2.0,
                )
            } else {
                vec2(
                    JUG_X + self.rng.range(-HW_TOP * 0.8, HW_TOP * 0.8),
                    y_of(1.0) + self.rng.range(-4.0, 8.0),
                )
            };
            self.puffs.push(Puff {
                p,
                v: vec2(self.rng.range(-6.0, 14.0), -self.rng.range(18.0, 34.0)),
                age: 0.0,
                life: self.rng.range(1.1, 2.0),
                r: self.rng.range(6.0, 12.0),
            });
        }
        for p in &mut self.puffs {
            p.age += dt;
            p.p += p.v * dt;
            p.v.x += (p.age * 3.0 + p.r).sin() * 6.0 * dt;
        }
        self.puffs.retain(|p| p.age < p.life);
    }

    pub fn autoplay(&self, t: f32, keyboard: bool) -> MgInput {
        if let DropState::Swinging { since } = self.drop_state {
            let now = t > since + 0.35 && (self.drop_x(t) - self.drop_center()).abs() < 3.0;
            return if keyboard {
                MgInput {
                    hold_pressed: now,
                    ..Default::default()
                }
            } else {
                MgInput {
                    pointer: Some(vec2(self.drop_center(), 120.0)),
                    pressed: now,
                    ..Default::default()
                }
            };
        }
        if self.drop_state == DropState::Waiting
            && self.drop.is_some()
            && self.peak >= self.tu.t_target - 0.4
        {
            return MgInput::default();
        }
        let stop = self.tu.t_target - 0.4;
        let mut hold = t > 0.7 && self.peak < stop && (!self.started || self.released_for < 0.3);
        // pulse when the foam would fall short of the target
        let projected = self.foam + FOAM_RATE * (stop - self.temp).max(0.0) / T_RATE;
        if hold && self.holding && projected < self.tu.foam - 0.03 && t - self.last_press > 0.8 {
            hold = false;
        }
        if keyboard {
            MgInput {
                hold,
                ..Default::default()
            }
        } else {
            MgInput {
                pointer: Some(vec2(JUG_X, 120.0)),
                pressed: hold,
                down: hold,
                released: !hold,
                ..Default::default()
            }
        }
    }

    pub fn hint(&self) -> Hint<'static> {
        let done = self.common.relaxed;
        if matches!(self.drop_state, DropState::Swinging { .. }) {
            return Hint {
                key: match self.drop {
                    Some((FrothDrop::Chili, _)) => "minigame.milk_froth.prompt.chili",
                    _ => "minigame.milk_froth.prompt.vanilla",
                },
                emphasis: (self.drop_x(self.t) - self.drop_center()).abs() < 12.0,
                keys: &["Space"],
                done: false,
            };
        }
        if self.holding && self.in_band > 0.5 {
            Hint {
                key: "minigame.milk_froth.prompt.release",
                emphasis: true,
                keys: &[],
                done,
            }
        } else if !self.holding
            && self.temp > T_START + 3.0
            && self.temp < self.band(self.common.relaxed).0
        {
            Hint {
                key: "minigame.milk_froth.prompt.again",
                emphasis: false,
                keys: &["Space"],
                done,
            }
        } else {
            Hint {
                key: "minigame.milk_froth.prompt.hold",
                emphasis: false,
                keys: &["Space"],
                done,
            }
        }
    }

    pub fn draw(&self, ui: &mut Ui, o: Vec2, t: f32, _text: &MiniGameText) {
        let th = ui.theme.clone();
        let x = o.x + JUG_X;
        let bottom = o.y + JUG_BOTTOM;
        let top = bottom - JUG_H;

        draw_cup(
            o + vec2(self.cup_x, JUG_BOTTOM),
            t,
            self.tu.drink,
            self.chili_on(),
        );
        contact_shadow(vec2(x + 4.0, bottom + 2.0), HW_BOTTOM + 26.0, 13.0, 0.55);

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
        let tip = vec2(x + 16.0, o.y + y_of(MILK0 - 0.1));
        let root = vec2(x + 140.0, o.y - 30.0);
        rd::capsule(root, tip, 4.0, hex(0x8e96ad));
        rd::capsule(
            root + vec2(-1.4, 0.4),
            tip + vec2(-1.4, 0.0),
            1.3,
            with_alpha(hex(0xe4e8f5), 0.8),
        );
        rd::capsule(root, root + (tip - root) * 0.12, 6.0, hex(0x4a5068));

        // milk + foam
        let level = self.level();
        let milk_top = MILK0 - self.foam * 0.1;
        let wob = 1.0 + 2.2 * self.hiss;
        let surface = |lv: f32, phase: f32, amp: f32| -> Vec<Vec2> {
            let y = o.y + y_of(lv);
            let hw = inner_hw(lv);
            let n = 18;
            (0..=n)
                .map(|i| {
                    let u = i as f32 / n as f32;
                    let xx = x - hw + 2.0 * hw * u;
                    let w = (u * 9.0 + t * 5.5 + phase).sin() * amp * (u * PI).sin();
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
        // milk warms from cream to a slightly golden tone
        let warm = ((self.temp - 20.0) / 50.0).clamp(0.0, 1.0);
        let milk_poly = body(surface(milk_top, 0.0, 0.8), 0.0);
        fill_convex(
            &milk_poly,
            Paint::vertical(
                Rect::new(x, o.y + y_of(milk_top), 1.0, y_of(0.0) - y_of(milk_top)),
                draw::mix(self.milk_top(), hex(0xf3e2c8), warm),
                draw::mix(self.milk_bottom(), hex(0xd6bf9f), warm),
            ),
        );
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
            let area_h = y_of(milk_top) - y_of(level);
            let count = (area_h * 0.9) as u32;
            for i in 0..count.min(90) {
                let u = rd::hash01(i * 31 + 7);
                let v = rd::hash01(i * 17 + 3);
                let yy = fy + 3.0 + v * (area_h - 5.0).max(0.0);
                let lv = (y_of(0.0) - (yy - o.y)) / (y_of(0.0) - y_of(1.0));
                let hw = inner_hw(lv) - 4.0;
                let xx = x - hw + 2.0 * hw * u;
                let r = 0.7 + rd::hash01(i * 7 + 1) * 1.5 * (1.0 - v * 0.5);
                draw::fill_circle(vec2(xx, yy), r, Color::new(0.85, 0.79, 0.72, 0.35));
                draw::fill_circle(
                    vec2(xx - r * 0.3, yy - r * 0.3),
                    r * 0.4,
                    Color::new(1.0, 1.0, 1.0, 0.7),
                );
            }
            let sy = o.y + y_of(milk_top);
            rd::rect_v(
                x - inner_hw(milk_top),
                sy - 3.0,
                inner_hw(milk_top) * 2.0,
                6.0,
                Color::new(1.0, 1.0, 1.0, 0.0),
                Color::new(0.86, 0.78, 0.68, 0.22),
            );
        }
        if self.hiss > 0.05 {
            let sy = o.y + y_of(level);
            for k in 0..3 {
                let ph = t * 7.0 + k as f32 * 2.1;
                let rx = 9.0 + k as f32 * 6.0;
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

        // foam target: a soft lavender line across the glass
        let fy = o.y + y_of(MILK0 + self.tu.foam * FOAM_RISE);
        let hw_b = inner_hw(MILK0 + self.tu.foam * FOAM_RISE) + WALL + 2.0;
        let pulse = 0.5 + 0.5 * (t * 2.6).sin();
        let lav = th.indigo_300;
        rd::rect_v(
            x - hw_b,
            fy - 9.0,
            hw_b * 2.0,
            9.0,
            with_alpha(lav, 0.0),
            with_alpha(lav, 0.16 + 0.06 * pulse),
        );
        rd::rect_v(
            x - hw_b,
            fy,
            hw_b * 2.0,
            9.0,
            with_alpha(lav, 0.16 + 0.06 * pulse),
            with_alpha(lav, 0.0),
        );
        for side in [-1.0f32, 1.0] {
            let x0 = x + side * (hw_b + 1.0);
            draw::line(
                vec2(x0, fy),
                vec2(x0 + side * 9.0, fy),
                1.6,
                with_alpha(lav, 0.8),
            );
        }

        self.draw_glass_front(x, top, bottom, t);

        for p in &self.puffs {
            let a = p.age / p.life;
            let fade = (a * (1.0 - a) * 4.0).clamp(0.0, 1.0);
            rd::glow(
                o.x + p.p.x,
                o.y + p.p.y,
                p.r * (1.0 + a * 1.4),
                Color::new(0.94, 0.93, 1.0, 0.2 * fade),
            );
        }

        self.draw_dial(ui, o + DIAL_C, t);
        self.draw_drop(o, t);
    }

    fn chili_on(&self) -> bool {
        matches!(
            (self.drop, self.drop_state),
            (Some((FrothDrop::Chili, _)), DropState::Dropped { .. })
        )
    }

    fn milk_top(&self) -> Color {
        match self.tu.drink {
            // Raf: cream with vanilla — ivory
            FrothDrink::Raf => hex(0xf8eccb),
            _ => hex(0xf3ecdf),
        }
    }

    fn milk_bottom(&self) -> Color {
        match self.tu.drink {
            FrothDrink::Raf => hex(0xe6d2a6),
            _ => hex(0xdccfbd),
        }
    }

    /// The swinging vanilla bottle / chili pinch on its thread, then falling.
    fn draw_drop(&self, o: Vec2, t: f32) {
        let Some((kind, _)) = self.drop else {
            return;
        };
        let (x, y) = match self.drop_state {
            DropState::Waiting => return,
            DropState::Swinging { .. } => {
                let x = self.drop_x(t);
                draw::line(
                    vec2(o.x + self.drop_center(), o.y - 4.0),
                    vec2(o.x + x, o.y + DROP_Y - 10.0),
                    1.0,
                    Color::new(1.0, 0.98, 0.9, 0.6),
                );
                // the aim marker straight over the middle
                let pulse = 0.5 + 0.5 * (t * 3.0).sin();
                let cx = o.x + self.drop_center();
                draw::line(
                    vec2(cx, o.y + DROP_Y + 18.0),
                    vec2(cx, o.y + DROP_Y + 32.0),
                    2.0,
                    Color::new(1.0, 0.85, 0.5, 0.4 + 0.3 * pulse),
                );
                (x, DROP_Y)
            }
            DropState::Dropped { at, .. } => {
                let k = ((t - at) / 0.45).clamp(0.0, 1.0);
                if k >= 1.0 {
                    return;
                }
                let x = self.drop_center();
                (x, DROP_Y + 90.0 * k * k)
            }
        };
        let p = vec2(o.x + x, o.y + y);
        match kind {
            FrothDrop::Vanilla => {
                // a little amber bottle with a dark pod inside
                let b = Rect::new(p.x - 7.0, p.y - 10.0, 14.0, 20.0);
                fill_rrect(b, 4.0, Paint::vertical(b, hex(0xe9b766), hex(0xa8671f)));
                fill_rrect(
                    Rect::new(p.x - 4.0, p.y - 15.0, 8.0, 6.0),
                    2.0,
                    hex(0x5b3a2a),
                );
                draw::line(p + vec2(0.0, -6.0), p + vec2(1.0, 7.0), 2.0, hex(0x3a2216));
            }
            FrothDrop::Chili => {
                // a pinch of red flakes
                for i in 0..9u32 {
                    let a = rd::hash01(i * 7 + 1) * std::f32::consts::TAU;
                    let r = rd::hash01(i * 3 + 2).sqrt() * 7.0;
                    draw::fill_circle(p + vec2(a.cos(), a.sin()) * r, 1.8, hex(0xd2402a));
                }
                draw::fill_circle(p, 2.2, hex(0xf06a3a));
            }
        }
    }

    fn draw_dial(&self, ui: &mut Ui, c: Vec2, t: f32) {
        let th = ui.theme.clone();
        let (lo, hi) = self.band(self.common.relaxed);
        // face
        glow_circle(
            c,
            DIAL_R + 18.0,
            with_alpha(th.amber, 0.05 + 0.12 * self.in_band),
        );
        draw::fill_circle(
            c,
            DIAL_R + 8.0,
            Paint::vertical(
                Rect::new(c.x, c.y - DIAL_R, 1.0, DIAL_R * 2.0),
                hex(0x2c2940),
                hex(0x17142a),
            ),
        );
        draw::stroke_circle(c, DIAL_R + 8.0, 1.2, Color::new(1.0, 1.0, 1.0, 0.12));
        let a0 = dial_angle(20.0);
        let a1 = dial_angle(80.0);
        draw::arc(
            c,
            DIAL_R - 4.0,
            a0,
            a1,
            7.0,
            Color::new(1.0, 1.0, 1.0, 0.08),
        );
        // amber band
        let b0 = dial_angle(lo);
        let b1 = dial_angle(hi);
        draw::arc(
            c,
            DIAL_R - 4.0,
            b0,
            b1,
            9.0,
            with_alpha(th.amber, 0.55 + 0.4 * self.in_band),
        );
        if self.common.practised {
            let mid = (b0 + b1) * 0.5;
            let tw = ((t * 1.7).sin() * 0.5 + 0.5).powi(2);
            super::paint::sparkle(
                c + vec2(mid.cos(), mid.sin()) * (DIAL_R - 4.0),
                4.0,
                1.0 - tw * 0.8,
                true,
            );
        }
        // ticks every 10 °C
        for k in 0..=6 {
            let a = dial_angle(20.0 + k as f32 * 10.0);
            let d = vec2(a.cos(), a.sin());
            draw::line(
                c + d * (DIAL_R - 15.0),
                c + d * (DIAL_R - 11.0),
                1.2,
                Color::new(1.0, 1.0, 1.0, 0.35),
            );
        }
        // needle
        let a = dial_angle(self.temp);
        let d = vec2(a.cos(), a.sin());
        let col = draw::mix(th.text, th.amber, self.in_band);
        draw::line(c - d * 6.0, c + d * (DIAL_R - 10.0), 2.2, col);
        glow_circle(c + d * (DIAL_R - 10.0), 8.0, with_alpha(col, 0.4));
        draw::fill_circle(c, 4.5, hex(0x8e96ad));
        // readout
        let txt = format!("{}°", self.temp.round() as i32);
        let st = th.h2().tabular().color(col);
        ui.fonts.draw_in(
            &txt,
            Rect::new(c.x - 40.0, c.y + 14.0, 80.0, 22.0),
            &st,
            Align::Center,
        );
        let tgt = format!("{}°", self.tu.t_target.round() as i32);
        let small = th.small();
        ui.fonts.draw_in(
            &tgt,
            Rect::new(c.x - 40.0, c.y + DIAL_R + 12.0, 80.0, 16.0),
            &small,
            Align::Center,
        );
    }

    fn draw_glass_front(&self, x: f32, top: f32, bottom: f32, t: f32) {
        // base: thick glass foot
        let base = Rect::new(
            x - HW_BOTTOM,
            bottom - BASE - 2.0,
            HW_BOTTOM * 2.0,
            BASE + 2.0,
        );
        fill_rrect(
            base,
            6.0,
            Paint::vertical(
                base,
                Color::new(0.82, 0.8, 1.0, 0.22),
                Color::new(0.6, 0.56, 0.9, 0.32),
            ),
        );
        draw::hline(
            base.x + 8.0,
            base.right() - 8.0,
            base.y + 1.0,
            Color::new(1.0, 1.0, 1.0, 0.35),
        );
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
            Paint::vertical(
                Rect::new(x, top, 1.0, bottom - top),
                Color::new(1.0, 1.0, 1.0, 0.3),
                Color::new(1.0, 1.0, 1.0, 0.04),
            ),
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
            &[
                vec2(x - HW_TOP - 14.0, top - 7.0),
                vec2(x - HW_TOP + 6.0, top - 2.0),
                vec2(x - HW_TOP + 2.0, top + 9.0),
            ],
            Paint::Solid(Color::new(0.88, 0.88, 1.0, 0.32)),
        );
        // handle
        let hc = vec2(x + HW_TOP + 14.0, top + 56.0);
        draw::arc(hc, 28.0, -1.45, 1.45, 8.0, Color::new(0.8, 0.77, 1.0, 0.3));
        draw::arc(
            hc + vec2(1.0, 0.0),
            32.0,
            -1.0,
            0.4,
            1.6,
            Color::new(1.0, 1.0, 1.0, 0.35),
        );
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

/// The cup waiting for its milk (side view, 3/4 top): espresso crema, or cocoa.
fn draw_cup(base: Vec2, t: f32, drink: FrothDrink, chili: bool) {
    let cocoa = matches!(drink, FrothDrink::Chocolate | FrothDrink::Encore);
    let (c0, c1) = if cocoa {
        (hex(0x7a4126), hex(0x3c1c10))
    } else {
        (hex(0xb07540), hex(0x6a3a20))
    };
    contact_shadow(base + vec2(3.0, 1.0), 46.0, 9.0, 0.5);
    // saucer
    rd::ellipse_v(
        base.x,
        base.y - 3.0,
        42.0,
        8.0,
        hex(0xe9e2f4),
        hex(0xa99dc6),
    );
    rd::ellipse(base.x, base.y - 5.0, 30.0, 5.0, hex(0xcfc6e2));
    // cup body
    let w = 46.0;
    let h = 36.0;
    let body = Rect::new(base.x - w / 2.0, base.y - 6.0 - h, w, h);
    fill_rrect(
        body,
        12.0,
        Paint::horizontal(body, hex(0xfdfbff), hex(0xbcb0d4)),
    );
    // handle
    draw::arc(
        vec2(body.right() + 2.0, body.y + 14.0),
        9.0,
        -1.4,
        1.4,
        4.5,
        hex(0xd7cfe6),
    );
    // rim + crema
    rd::ellipse(base.x, body.y + 1.0, w / 2.0, 6.0, hex(0xf6f2fb));
    rd::ellipse_v(base.x, body.y + 1.5, w / 2.0 - 3.5, 4.2, c0, c1);
    if chili {
        for i in 0..8u32 {
            let px = base.x - 12.0 + rd::hash01(i * 5 + 3) * 24.0;
            let py = body.y + 0.5 + rd::hash01(i * 11 + 1) * 3.0;
            draw::fill_circle(vec2(px, py), 1.2, hex(0xd2402a));
        }
    }
    rd::ellipse(
        base.x - 5.0,
        body.y + 0.5,
        6.0,
        1.6,
        Color::new(0.95, 0.75, 0.5, 0.45),
    );
    // a thin wisp of steam
    for i in 0..2u32 {
        let life = 2.6;
        let a = ((t + i as f32 * 1.3) % life) / life;
        let fade = (a * (1.0 - a) * 4.0).clamp(0.0, 1.0);
        rd::glow(
            base.x + (t * 1.2 + i as f32).sin() * 4.0,
            body.y - 8.0 - a * 34.0,
            8.0 + a * 10.0,
            Color::new(0.95, 0.94, 1.0, 0.14 * fade),
        );
    }
}
