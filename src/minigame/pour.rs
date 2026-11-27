//! PourOver (§2.3): a dripper from above with three faint rings. Phase 1 *bloom*: pour a
//! little (~1 s), then "let it bloom…" (don't pour). Phase 2: hold and pour in slow circles
//! with the guide light while a soft metronome ticks. Keyboard: hold Space (the spout
//! circles by itself) and tap Left/Right on the ticks.
//! Quality: `0.5 × rhythm + 0.3 × centred + 0.2 × bloom` for the dripper. Each coffee is
//! itself: turek pours straight onto the grounds in a glass, then three stirs on the beat and a
//! still wait while the grounds settle (`0.45 rhythm + 0.25 centred + 0.3 stirs`); the espresso
//! tonic pours slowly over ice (`0.6 rhythm + 0.4 centred`, a slower band); the night train ends
//! with a cocoa-and-spice dust tap when the light passes the top (15 %).

use std::f32::consts::{PI, TAU};

use macroquad::prelude::*;

use super::paint::{contact_shadow, hex, with_alpha};
use super::score::{self, widen};
use super::{
    Common, Ctx, Hint, MgInput, MiniGameSound, MiniGameText, PourStyle, PourTuning, RegularTouch,
    Rng,
};
use crate::render::draw as rd;
use crate::ui::draw::{self, Paint, fill_circle, fill_rrect, glow_circle, stroke_circle};
use crate::ui::{Align, Ui, approach, ease_out};

const C: Vec2 = Vec2::new(172.0, 124.0);
const RIM: f32 = 110.0;
const CONE: f32 = 96.0;
const BED: f32 = 68.0;
const RINGS: [f32; 3] = [24.0, 40.0, 56.0];
const RING: f32 = 40.0;
const MAX_T: f32 = 22.0;
const BLOOM_POUR: f32 = 1.0;
const CARAFE: Rect = Rect {
    x: 392.0,
    y: 58.0,
    w: 66.0,
    h: 136.0,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Bloom,
    Wait,
    Circles,
    /// Turek: stir on the ticks.
    Stir,
    /// Turek: hands off while the grounds sink.
    Settle,
    /// Night train: dust when the light passes the top.
    Dust,
}

/// The night train's dust mark (12 o'clock).
const DUST_ANGLE: f32 = -PI / 2.0;

struct Ripple {
    p: Vec2,
    age: f32,
}

pub(crate) struct Pour {
    tu: PourTuning,
    common: Common,
    phase: Phase,
    phase_t: f32,
    mouse_hold: bool,
    point: Vec2,
    pouring: bool,
    flow: f32,
    bloom_poured: f32,
    bloom_ok: bool,
    water: f32,
    need: f32,
    in_rhythm_t: f32,
    centred_t: f32,
    pour_t: f32,
    prev_ang: Option<f32>,
    omega: f32,
    kb_ang: f32,
    kb_used: bool,
    ticks: u32,
    beat_hits: u32,
    tapped_tick: Option<u32>,
    wet: [f32; 24],
    bloom_puff: f32,
    ripples: Vec<Ripple>,
    ripple_acc: f32,
    phase0: f32,
    drip: f32,
    stir_acc: Vec<f32>,
    stir_tick: Option<u32>,
    dust_acc: Option<f32>,
    t: f32,
    rng: Rng,
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

impl Pour {
    pub fn new(
        rng: &mut Rng,
        mut tu: PourTuning,
        touch: Option<RegularTouch>,
        common: Common,
    ) -> Pour {
        let base = match tu.style {
            PourStyle::Dripper => 8.5,
            PourStyle::Turek => 5.5,
            PourStyle::Tonic => 6.5,
        };
        let need = if touch == Some(RegularTouch::Strong) {
            base * 1.18
        } else {
            base
        };
        // a slightly different tempo every play
        tu.bpm += rng.range(-2.5, 2.5);
        let bloom = tu.bloom_s > 0.0;
        Pour {
            tu,
            common,
            phase: if bloom { Phase::Bloom } else { Phase::Circles },
            phase_t: 0.0,
            mouse_hold: false,
            point: C + vec2(0.0, -RING),
            pouring: false,
            flow: 0.0,
            bloom_poured: 0.0,
            bloom_ok: true,
            water: 0.0,
            need,
            in_rhythm_t: 0.0,
            centred_t: 0.0,
            pour_t: 0.0,
            prev_ang: None,
            omega: 0.0,
            kb_ang: if bloom { 0.0 } else { -0.3 },
            kb_used: false,
            ticks: 0,
            beat_hits: 0,
            tapped_tick: None,
            wet: [0.0; 24],
            bloom_puff: 0.0,
            ripples: Vec::new(),
            ripple_acc: 0.0,
            phase0: rng.range(0.0, TAU),
            drip: 0.0,
            stir_acc: Vec::new(),
            stir_tick: None,
            dust_acc: None,
            t: 0.0,
            rng: Rng::new(rng.next_u64()),
        }
    }

    pub fn steps(&self) -> Vec<&'static str> {
        let mut v = Vec::new();
        match self.tu.style {
            PourStyle::Turek => v.push("pour_over.stir"),
            PourStyle::Tonic => v.push("pour_over.slow"),
            PourStyle::Dripper => {}
        }
        if self.tu.dust {
            v.push("pour_over.dust");
        }
        v
    }

    pub fn preview(&self, relaxed: bool) -> f32 {
        self.quality_with(relaxed, 1.0)
    }

    fn parts(&self, relaxed: bool) -> (f32, f32) {
        let _ = relaxed;
        let fill = (self.water / self.need).clamp(0.0, 1.0);
        let (rhythm, centred) = if self.kb_used {
            let ticks = self.ticks.max(1) as f32;
            ((self.beat_hits as f32 / ticks).min(1.0), 1.0)
        } else if self.pour_t > 0.0 {
            (self.in_rhythm_t / self.pour_t, self.centred_t / self.pour_t)
        } else {
            (0.0, 0.0)
        };
        (rhythm * fill, centred * fill)
    }

    fn stir_score(&self, pending: f32) -> f32 {
        let n = self.tu.stirs.max(1) as usize;
        let done: f32 = self.stir_acc.iter().sum();
        let missing = n.saturating_sub(self.stir_acc.len()) as f32;
        (done + missing * pending) / n as f32
    }

    /// The quality; steps not reached yet count `pending` (1 for the live preview).
    fn quality_with(&self, relaxed: bool, pending: f32) -> f32 {
        let (rhythm, centred) = self.parts(relaxed);
        let base = match self.tu.style {
            PourStyle::Dripper => {
                score::pour(rhythm, centred, if self.bloom_ok { 1.0 } else { 0.0 })
            }
            PourStyle::Turek => 0.45 * rhythm + 0.25 * centred + 0.3 * self.stir_score(pending),
            PourStyle::Tonic => 0.6 * rhythm + 0.4 * centred,
        };
        if self.tu.dust {
            0.85 * base + 0.15 * self.dust_acc.unwrap_or(pending)
        } else {
            base
        }
    }

    pub fn advice(&self, relaxed: bool) -> &'static str {
        let (rhythm, centred) = self.parts(relaxed);
        if self.tu.style == PourStyle::Dripper && !self.bloom_ok {
            return "minigame.why.pour.bloom";
        }
        if self.tu.style == PourStyle::Turek && self.stir_score(0.5) < 0.75 {
            return "minigame.why.pour.stir";
        }
        if self.tu.dust && self.dust_acc.unwrap_or(0.5) < 0.6 {
            return "minigame.why.pour.dust";
        }
        if self.kb_used {
            return "minigame.why.pour.beat";
        }
        if rhythm <= centred {
            "minigame.why.pour.rhythm"
        } else {
            "minigame.why.pour.centre"
        }
    }

    pub fn flow(&self) -> f32 {
        self.flow
    }

    pub fn focus(&self) -> Vec2 {
        C
    }

    fn freq(&self) -> f32 {
        self.tu.bpm / 60.0
    }

    /// Guide light angle: one turn per beat.
    fn guide_angle(&self, t: f32) -> f32 {
        self.phase0 + t * self.freq() * TAU
    }

    fn guide_pos(&self, t: f32) -> Vec2 {
        let a = self.guide_angle(t);
        C + vec2(a.cos(), a.sin()) * RING
    }

    /// Seconds since the last metronome tick (Circles phase).
    fn since_tick(&self) -> f32 {
        self.phase_t % (1.0 / self.freq())
    }

    pub fn finished_gesture(&self) -> bool {
        self.water >= self.need
            && !matches!(self.phase, Phase::Stir | Phase::Dust)
            && (self.phase != Phase::Settle || self.phase_t >= self.tu.settle_s)
    }

    fn quality(&self, relaxed: bool) -> f32 {
        self.quality_with(relaxed, 0.5)
    }

    /// The pour is done: turek stirs, the night train dusts, the rest is served.
    fn after_pour(&mut self, ctx: &mut Ctx) -> bool {
        let next = if self.tu.style == PourStyle::Turek && self.tu.stirs > 0 {
            Some(Phase::Stir)
        } else if self.tu.dust {
            Some(Phase::Dust)
        } else {
            None
        };
        match next {
            Some(p) => {
                self.phase = p;
                self.phase_t = 0.0;
                self.pouring = false;
                ctx.sounds.push(MiniGameSound::Cue);
                false
            }
            None => true,
        }
    }

    fn tap(input: &MgInput) -> bool {
        input.pressed || input.hold_pressed || input.left || input.right
    }

    pub fn step(&mut self, ctx: &mut Ctx, input: &MgInput, dt: f32) -> Option<f32> {
        self.t = ctx.t;
        self.phase_t += dt;
        if input.pressed {
            self.mouse_hold = true;
        }
        if input.released || !input.down {
            self.mouse_hold = false;
        }
        let keyboard = input.hold && !self.mouse_hold;
        let target = if keyboard {
            self.kb_used = true;
            if self.phase == Phase::Circles {
                self.kb_ang += self.freq() * TAU * dt;
            }
            let a = self.guide_angle(ctx.t) - 0.3;
            let a = if self.phase == Phase::Circles {
                self.kb_ang + self.phase0
            } else {
                a
            };
            C + vec2(a.cos(), a.sin()) * RING
        } else {
            input.pointer.unwrap_or(self.point)
        };
        // the kettle is heavy: the pour point follows with a little lag
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
        let on_bed = rel.length() < CONE;

        match self.phase {
            Phase::Bloom => {
                if self.pouring && on_bed {
                    self.bloom_poured += dt;
                    self.water += dt * 0.3;
                }
                let enough = self.bloom_poured >= BLOOM_POUR;
                if enough && (!self.pouring || self.bloom_poured > BLOOM_POUR + 0.5) {
                    self.phase = Phase::Wait;
                    self.phase_t = 0.0;
                    ctx.sounds.push(MiniGameSound::Cue);
                }
            }
            Phase::Wait => {
                self.bloom_puff = approach(self.bloom_puff, 1.0, dt, 0.5);
                // a short grace so letting go late is fine
                if self.pouring && self.phase_t > 0.35 {
                    self.bloom_ok = false;
                }
                if self.phase_t >= self.tu.bloom_s {
                    self.phase = Phase::Circles;
                    self.phase_t = 0.0;
                    self.kb_ang = self.guide_angle(ctx.t) - self.phase0 - 0.3;
                    ctx.sounds.push(MiniGameSound::Cue);
                }
            }
            Phase::Circles => {
                self.bloom_puff = approach(self.bloom_puff, 0.0, dt, 2.0);
                // metronome
                let tick = (self.phase_t * self.freq()) as u32;
                if tick + 1 > self.ticks {
                    if self.ticks > 0 || self.phase_t < dt * 1.5 {
                        ctx.sounds.push(MiniGameSound::Tick);
                    }
                    if self.pouring {
                        self.ticks = tick + 1;
                    } else {
                        self.ticks = self.ticks.max(tick);
                    }
                }
                if self.pouring && on_bed {
                    self.water += dt * self.flow;
                    self.pour_t += dt;
                    if let Some(pa) = self.prev_ang {
                        let w = wrap(ang - pa) / dt.max(1e-4) / TAU;
                        self.omega = approach(self.omega, w, dt, 0.15);
                    }
                    if score::in_band(self.omega, self.tu.rev, ctx.relaxed) {
                        self.in_rhythm_t += dt;
                    }
                    let m = 6.0 * widen(ctx.relaxed);
                    let r = rel.length();
                    if r >= RINGS[0] - m && r <= RINGS[2] + m {
                        self.centred_t += dt;
                    }
                    self.prev_ang = Some(ang);
                } else {
                    self.prev_ang = None;
                }
                // beat taps (keyboard path)
                if keyboard && (input.left || input.right) {
                    let s = self.since_tick();
                    let period = 1.0 / self.freq();
                    let d = if s > period * 0.5 { s - period } else { s };
                    let n = ((self.phase_t + period * 0.5) * self.freq()) as u32;
                    if score::on_beat(d, ctx.relaxed) && self.tapped_tick != Some(n) {
                        self.beat_hits += 1;
                        self.tapped_tick = Some(n);
                        ctx.sounds.push(MiniGameSound::Cue);
                    }
                }
            }
            Phase::Stir => {
                self.pouring = false;
                let period = 1.0 / self.freq();
                let tick = (self.phase_t * self.freq()) as u32;
                if tick + 1 > self.ticks {
                    ctx.sounds.push(MiniGameSound::Tick);
                    self.ticks = tick + 1;
                }
                let idle = self.phase_t > 3.0 + self.stir_acc.len() as f32 * 2.5;
                if Self::tap(input) || (!self.common.relaxed && idle) {
                    let s = self.phase_t % period;
                    let d = if s > period * 0.5 { s - period } else { s };
                    let n = ((self.phase_t + period * 0.5) * self.freq()) as u32;
                    if self.stir_tick != Some(n) || idle {
                        let acc = if idle {
                            0.5
                        } else {
                            (1.0 - d.abs() / (0.3 * widen(ctx.relaxed))).clamp(0.3, 1.0)
                        };
                        self.stir_acc.push(acc);
                        self.stir_tick = Some(n);
                        ctx.sounds.push(MiniGameSound::Clink);
                        self.bloom_puff = 1.0;
                    }
                }
                if self.stir_acc.len() >= self.tu.stirs as usize {
                    self.phase = if self.tu.dust {
                        Phase::Dust
                    } else {
                        Phase::Settle
                    };
                    self.phase_t = 0.0;
                }
            }
            Phase::Settle => {
                self.pouring = false;
                self.bloom_puff = approach(self.bloom_puff, 0.0, dt, 0.8);
            }
            Phase::Dust => {
                self.pouring = false;
                let auto = !self.common.relaxed && self.phase_t > 5.0;
                if self.dust_acc.is_none() && self.phase_t > 0.2 && (Self::tap(input) || auto) {
                    let d = wrap(self.guide_angle(ctx.t) - DUST_ANGLE).abs();
                    let acc = if auto {
                        0.5
                    } else {
                        (1.0 - d / (0.7 * widen(ctx.relaxed))).clamp(0.0, 1.0)
                    };
                    self.dust_acc = Some(acc);
                    self.phase_t = 0.0;
                    ctx.sounds.push(MiniGameSound::Pour);
                    if acc > 0.6 {
                        ctx.sounds.push(MiniGameSound::Cue);
                    }
                }
            }
        }

        if self.pouring && on_bed {
            let sector = (((ang + PI) / TAU * 24.0) as usize).min(23);
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
        }
        for r in &mut self.ripples {
            r.age += dt;
        }
        self.ripples.retain(|r| r.age < 0.9);
        for w in &mut self.wet {
            *w = (*w - dt * 0.05).max(0.0);
        }
        self.drip = approach(self.drip, (self.water / self.need).min(1.0), dt, 1.2);

        if self.phase == Phase::Circles && self.water >= self.need && !self.common.relaxed {
            let served = self.after_pour(ctx);
            if !served {
                return None;
            }
        }
        if self.common.relaxed && self.phase == Phase::Circles && self.water >= self.need {
            self.after_pour(ctx);
        }
        let extra = match self.phase {
            Phase::Settle => self.phase_t >= self.tu.settle_s,
            Phase::Dust => self.dust_acc.is_some() && self.phase_t > 0.5,
            _ => false,
        };
        let max_t = MAX_T
            + if self.tu.stirs > 0 || self.tu.dust {
                8.0
            } else {
                0.0
            };
        let finish = if self.common.relaxed {
            input.confirm
        } else {
            (self.water >= self.need
                && !matches!(self.phase, Phase::Stir | Phase::Dust | Phase::Settle))
                || extra
                || ctx.t > max_t
                || (input.confirm && self.phase == Phase::Circles)
        };
        if finish {
            ctx.sounds.push(MiniGameSound::Clink);
            return Some(self.quality(ctx.relaxed));
        }
        None
    }

    pub fn autoplay(&self, t: f32, keyboard: bool) -> MgInput {
        let tap = match self.phase {
            Phase::Stir => {
                let period = 1.0 / self.freq();
                let n = ((self.phase_t + period * 0.5) * self.freq()) as u32;
                Some(
                    self.phase_t % period < 0.03 && self.stir_tick != Some(n) && self.phase_t > 0.2,
                )
            }
            Phase::Dust => Some(
                self.dust_acc.is_none()
                    && self.phase_t > 0.3
                    && wrap(self.guide_angle(t) - DUST_ANGLE).abs() < 0.12,
            ),
            Phase::Settle => Some(false),
            _ => None,
        };
        if let Some(tap) = tap {
            return if keyboard {
                MgInput {
                    hold_pressed: tap,
                    ..Default::default()
                }
            } else {
                MgInput {
                    pointer: Some(C),
                    pressed: tap,
                    ..Default::default()
                }
            };
        }
        let pour = match self.phase {
            Phase::Bloom => t > 0.6 && self.bloom_poured < BLOOM_POUR + 0.1,
            Phase::Wait => false,
            Phase::Circles => self.water < self.need,
            _ => false,
        };
        if keyboard {
            let s = self.since_tick();
            let n = (self.phase_t * self.freq()) as u32;
            let tap = self.phase == Phase::Circles
                && s < 0.05
                && self.tapped_tick != Some(n)
                && self.phase_t > 0.1;
            return MgInput {
                hold: pour,
                left: tap && n.is_multiple_of(2),
                right: tap && n % 2 == 1,
                ..Default::default()
            };
        }
        let p = match self.phase {
            Phase::Circles => {
                let wobble = vec2((t * 1.3).sin(), (t * 1.7).cos()) * 2.5;
                self.guide_pos(t + 0.06) + wobble
            }
            _ => C + vec2((t * 1.1).cos(), (t * 1.1).sin()) * 16.0,
        };
        MgInput {
            pointer: Some(p),
            pressed: pour,
            down: pour,
            ..Default::default()
        }
    }

    pub fn hint(&self) -> Hint<'static> {
        let done = self.common.relaxed;
        match self.phase {
            Phase::Bloom => Hint {
                key: "minigame.pour_over.prompt.bloom_pour",
                emphasis: false,
                keys: &["Space"],
                done,
            },
            Phase::Wait => Hint {
                key: "minigame.pour_over.prompt.bloom",
                emphasis: true,
                keys: &[],
                done,
            },
            Phase::Circles if self.kb_used => Hint {
                key: "minigame.pour_over.prompt.keys",
                emphasis: false,
                keys: &["Space", "←", "→"],
                done,
            },
            Phase::Circles if self.tu.style == PourStyle::Tonic => Hint {
                key: "minigame.pour_over.prompt.slow",
                emphasis: false,
                keys: &[],
                done,
            },
            Phase::Circles => Hint {
                key: "minigame.pour_over.prompt.circles",
                emphasis: false,
                keys: &[],
                done,
            },
            Phase::Stir => Hint {
                key: "minigame.pour_over.prompt.stir",
                emphasis: self.since_tick() * self.freq() < 0.25,
                keys: &["Space"],
                done,
            },
            Phase::Settle => Hint {
                key: "minigame.pour_over.prompt.settle",
                emphasis: false,
                keys: &[],
                done,
            },
            Phase::Dust => Hint {
                key: "minigame.pour_over.prompt.dust",
                emphasis: wrap(self.guide_angle(self.t) - DUST_ANGLE).abs() < 0.5,
                keys: &["Space"],
                done,
            },
        }
    }

    pub fn draw(&self, ui: &mut Ui, o: Vec2, t: f32, text: &MiniGameText) {
        let th = ui.theme.clone();
        let c = o + C;
        match self.tu.style {
            PourStyle::Dripper => self.draw_dripper(c, t),
            PourStyle::Turek => self.draw_turek(c, t),
            PourStyle::Tonic => self.draw_tonic(c, t),
        }

        // three faint rings; the guide light and the beat pulse (Circles)
        let beat = if self.phase == Phase::Circles {
            1.0 - (self.since_tick() * self.freq() * 3.0).min(1.0)
        } else {
            0.0
        };
        for (i, r) in RINGS.iter().enumerate() {
            let a = if i == 1 { 0.3 } else { 0.16 } + 0.2 * beat;
            stroke_circle(c, *r, 1.4, with_alpha(th.indigo_300, a));
        }
        if self.common.practised {
            let tw = ((t * 1.7).sin() * 0.5 + 0.5).powi(2);
            super::paint::sparkle(c + vec2(0.0, -RINGS[2]), 4.0, 1.0 - tw * 0.8, true);
        }
        if self.phase == Phase::Dust {
            let m = c + vec2(DUST_ANGLE.cos(), DUST_ANGLE.sin()) * RING;
            let pulse = 0.5 + 0.5 * (t * 4.0).sin();
            glow_circle(m, 18.0, with_alpha(th.amber, 0.2 + 0.2 * pulse));
            stroke_circle(m, 9.0, 1.6, with_alpha(th.amber, 0.8));
        }
        if let Some(acc) = self.dust_acc {
            // cocoa and spice dust over the cup
            let k = ease_out(self.phase_t / 0.4);
            for i in 0..70u32 {
                let a = rd::hash01(i * 13 + 5) * TAU;
                let r = rd::hash01(i * 7 + 1).sqrt() * (24.0 + 30.0 * acc) * k;
                fill_circle(
                    c + vec2(a.cos(), a.sin()) * r,
                    1.1,
                    Color::new(0.33, 0.18, 0.1, 0.8),
                );
            }
        }
        if self.phase == Phase::Stir {
            let beat = 1.0 - (self.since_tick() * self.freq() * 3.0).min(1.0);
            glow_circle(
                c,
                40.0 + beat * 12.0,
                with_alpha(th.amber, 0.12 + 0.25 * beat),
            );
            for k in 0..self.tu.stirs {
                let done = (k as usize) < self.stir_acc.len();
                let p = c + vec2(-24.0 + k as f32 * 24.0, RINGS[2] + 18.0);
                fill_circle(
                    p,
                    5.0,
                    if done {
                        th.amber
                    } else {
                        Color::new(1.0, 1.0, 1.0, 0.25)
                    },
                );
            }
        }
        if self.phase == Phase::Settle {
            let k = (self.phase_t / self.tu.settle_s.max(0.1)).min(1.0);
            draw::arc(
                c,
                BED + 12.0,
                -PI / 2.0,
                -PI / 2.0 + TAU * k,
                2.0,
                with_alpha(th.amber, 0.7),
            );
        }
        if matches!(self.phase, Phase::Circles | Phase::Dust) || self.common.relaxed {
            let g = o + self.guide_pos(t);
            for k in 1..8 {
                let a = self.guide_angle(t) - k as f32 * 0.09;
                let p = c + vec2(a.cos(), a.sin()) * RING;
                fill_circle(
                    p,
                    3.6 - k as f32 * 0.35,
                    with_alpha(th.amber, 0.4 * (1.0 - k as f32 / 8.0)),
                );
            }
            glow_circle(g, 20.0 + beat * 8.0, with_alpha(th.amber, 0.4 + 0.3 * beat));
            fill_circle(g, 4.5, hex(0xfff3d6));
        }
        if self.phase == Phase::Wait {
            // a soft countdown ring around the bed while it blooms
            let k = (self.phase_t / self.tu.bloom_s).min(1.0);
            draw::arc(
                c,
                BED + 12.0,
                -PI / 2.0,
                -PI / 2.0 + TAU * k,
                2.0,
                with_alpha(th.amber, 0.7),
            );
        }

        for r in &self.ripples {
            let a = r.age / 0.9;
            stroke_circle(
                o + r.p,
                3.0 + a * 13.0,
                1.2,
                Color::new(0.95, 0.9, 0.82, 0.4 * (1.0 - a)),
            );
        }

        // gooseneck kettle spout coming from the top-right
        let p = o + self.point;
        let tip = p + vec2(10.0, -12.0);
        // kettle body peeking in from the top edge (top view: lid + knob)
        let kc = vec2(o.x + 300.0, o.y - 34.0);
        rd::soft_shadow(kc.x + 8.0, kc.y + 14.0, 74.0, 66.0, 0.45);
        fill_circle(
            kc,
            64.0,
            Paint::diagonal(
                Rect::new(kc.x - 64.0, kc.y - 64.0, 128.0, 128.0),
                hex(0x5a5f74),
                hex(0x23252f),
            ),
        );
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
            rd::capsule(
                w[0] + vec2(-0.6, -0.6),
                w[1] + vec2(-0.6, -0.6),
                3.6 - 1.4 * u,
                hex(0x7d849c),
            );
        }
        let hl: Vec<Vec2> = pts.iter().map(|q| *q + vec2(-1.6, -1.6)).collect();
        draw::stroke_path(
            &hl,
            false,
            1.2,
            Paint::Solid(Color::new(1.0, 1.0, 1.0, 0.5)),
        );
        fill_circle(tip, 3.4, hex(0x30333f));
        if self.flow > 0.02 {
            rd::capsule(
                tip,
                p,
                1.8 * self.flow + 0.3,
                Color::new(0.86, 0.92, 1.0, 0.8 * self.flow),
            );
            glow_circle(p, 12.0, Color::new(0.86, 0.92, 1.0, 0.3 * self.flow));
        }

        self.draw_carafe(ui, o, t, text);
    }

    /// The paper dripper from above: rim, filter folds, the coffee bed and its wet sectors.
    fn draw_dripper(&self, c: Vec2, t: f32) {
        contact_shadow(c + vec2(8.0, 12.0), RIM + 14.0, RIM * 0.95, 0.55);
        let handle = Rect::new(c.x - 18.0, c.y + RIM - 14.0, 36.0, 26.0);
        fill_rrect(
            handle,
            12.0,
            Paint::vertical(handle, hex(0xf2ecf7), hex(0xb9aecd)),
        );
        let sq = |r: f32| Rect::new(c.x - r, c.y - r, r * 2.0, r * 2.0);
        fill_circle(
            c,
            RIM,
            Paint::diagonal(sq(RIM), hex(0xfbf8fd), hex(0xb8acd0)),
        );
        fill_circle(c, CONE + 3.0, with_alpha(hex(0x8c7fae), 0.5));
        for i in 0..6 {
            let k = 1.0 - i as f32 / 6.0;
            let col = draw::mix(hex(0xd9cdb8), hex(0xf6efe2), 1.0 - k);
            fill_circle(c, CONE * (0.62 + 0.38 * k), col);
        }
        for i in 0..16 {
            let a = i as f32 / 16.0 * TAU;
            let a2 = a + 0.25;
            let p0 = c + vec2(a.cos(), a.sin()) * (BED + 4.0);
            let p1 = c + vec2(a2.cos(), a2.sin()) * (CONE - 3.0);
            draw::line(p0, p1, 1.0, Color::new(0.55, 0.45, 0.35, 0.18));
        }
        // coffee bed; it puffs up and lightens while blooming
        let puff = self.bloom_puff;
        let bed_r = BED + 3.0 + puff * 3.0;
        rd::radial(
            c.x,
            c.y,
            bed_r,
            bed_r,
            draw::mix(hex(0x6a4128), hex(0x9a6a44), puff * 0.6),
            hex(0x3a2216),
        );
        for i in 0..140u32 {
            let a = rd::hash01(i * 17 + 3) * TAU;
            let r = rd::hash01(i * 31 + 7).sqrt() * (BED - 2.0);
            let g = c + vec2(a.cos(), a.sin()) * r;
            let col = if i % 2 == 0 {
                Color::new(0.55, 0.36, 0.22, 0.5)
            } else {
                Color::new(0.16, 0.08, 0.04, 0.45)
            };
            fill_circle(g, 0.7 + rd::hash01(i) * 0.9, col);
        }
        if puff > 0.02 {
            for i in 0..40u32 {
                let a = rd::hash01(i * 7 + 11) * TAU;
                let r = rd::hash01(i * 13 + 2).sqrt() * (BED - 10.0);
                let bp = c + vec2(a.cos(), a.sin()) * r;
                let wob = ((t * 2.0 + i as f32).sin() * 0.5 + 0.5) * puff;
                fill_circle(bp, 1.0 + 2.2 * wob, Color::new(0.88, 0.7, 0.5, 0.45 * wob));
            }
        }
        for (i, w) in self.wet.iter().enumerate() {
            if *w <= 0.01 {
                continue;
            }
            let a0 = i as f32 / 24.0 * TAU - PI;
            let mid = a0 + TAU / 48.0;
            let p = c + vec2(mid.cos(), mid.sin()) * RING;
            rd::glow(p.x, p.y, 22.0, Color::new(0.16, 0.08, 0.04, 0.45 * w));
            for k in 0..3u32 {
                let h = rd::hash01(i as u32 * 7 + k * 3);
                let bp = p + vec2((h * 6.3).cos(), (h * 6.3).sin()) * (6.0 + h * 10.0);
                let pulse = ((t * 1.5 + h * 4.0).sin() * 0.5 + 0.5) * w;
                fill_circle(
                    bp,
                    1.2 + 1.5 * pulse,
                    Color::new(0.86, 0.68, 0.48, 0.45 * pulse),
                );
            }
        }
        draw::stroke_circle(c, BED + 4.0, 6.0, hex(0x4a2e1d));
    }

    /// Turek: a glass from above, the grounds floating, then sinking while it settles.
    fn draw_turek(&self, c: Vec2, t: f32) {
        contact_shadow(c + vec2(8.0, 12.0), RIM + 4.0, RIM * 0.9, 0.55);
        let sq = |r: f32| Rect::new(c.x - r, c.y - r, r * 2.0, r * 2.0);
        // saucer and the glass with its metal holder
        fill_circle(
            c,
            RIM,
            Paint::diagonal(sq(RIM), hex(0xf6f1fa), hex(0xada1c8)),
        );
        fill_circle(
            c,
            CONE,
            Paint::diagonal(sq(CONE), hex(0xb9bccb), hex(0x6d7086)),
        );
        fill_circle(c, CONE - 6.0, with_alpha(hex(0xe9e6ff), 0.35));
        let fill = (self.water / self.need).clamp(0.0, 1.0);
        let settle = if self.phase == Phase::Settle {
            (self.phase_t / self.tu.settle_s.max(0.1)).min(1.0)
        } else {
            0.0
        };
        // dark coffee; grounds swirl on top until they sink
        let liquid = draw::mix(hex(0x3a2216), hex(0x5a3420), 0.4 * fill);
        fill_circle(c, CONE - 10.0, draw::mix(hex(0x2b1a12), liquid, fill));
        let swirl = t * (0.6 + 2.0 * self.bloom_puff);
        let floating = (1.0 - settle) * (0.35 + 0.65 * fill);
        for i in 0..120u32 {
            let a = rd::hash01(i * 17 + 3) * TAU + swirl * (0.5 + rd::hash01(i) * 0.5);
            let r = rd::hash01(i * 31 + 7).sqrt() * (CONE - 16.0);
            let g = c + vec2(a.cos(), a.sin()) * r;
            fill_circle(
                g,
                0.8 + rd::hash01(i * 3) * 1.0,
                Color::new(0.62, 0.42, 0.26, 0.55 * floating),
            );
        }
        if settle > 0.0 {
            // a thin caramel ring forms as it clears
            stroke_circle(
                c,
                CONE - 14.0,
                3.0,
                Color::new(0.75, 0.5, 0.3, 0.4 * settle),
            );
        }
        draw::arc(
            c,
            CONE - 2.0,
            3.6,
            5.2,
            2.0,
            Color::new(1.0, 1.0, 1.0, 0.45),
        );
        for (i, w) in self.wet.iter().enumerate() {
            if *w > 0.01 {
                let a0 = i as f32 / 24.0 * TAU - PI + TAU / 48.0;
                let p = c + vec2(a0.cos(), a0.sin()) * RING;
                rd::glow(p.x, p.y, 18.0, Color::new(0.7, 0.5, 0.32, 0.25 * w));
            }
        }
    }

    /// Espresso tonic: a tall glass from above, tonic with ice cubes; the espresso clouds in.
    fn draw_tonic(&self, c: Vec2, t: f32) {
        contact_shadow(c + vec2(8.0, 12.0), RIM, RIM * 0.9, 0.5);
        let sq = |r: f32| Rect::new(c.x - r, c.y - r, r * 2.0, r * 2.0);
        fill_circle(
            c,
            CONE + 6.0,
            Paint::diagonal(sq(CONE + 6.0), hex(0xf4f2ff), hex(0x9f97c4)),
        );
        fill_circle(
            c,
            CONE,
            Paint::diagonal(sq(CONE), hex(0xf2e6b8), hex(0xd9c57e)),
        );
        let fill = (self.water / self.need).clamp(0.0, 1.0);
        // espresso clouds where it was poured
        for (i, w) in self.wet.iter().enumerate() {
            let a0 = i as f32 / 24.0 * TAU - PI + TAU / 48.0;
            let p = c + vec2(a0.cos(), a0.sin()) * RING;
            rd::glow(
                p.x,
                p.y,
                30.0,
                Color::new(0.4, 0.2, 0.1, 0.35 * w.max(fill * 0.6)),
            );
        }
        rd::glow(
            c.x,
            c.y,
            CONE * 0.8,
            Color::new(0.35, 0.18, 0.08, 0.35 * fill),
        );
        // ice cubes
        for k in 0..4u32 {
            let a = k as f32 / 4.0 * TAU + 0.5 + (t * 0.2 + k as f32).sin() * 0.05;
            let p = c + vec2(a.cos(), a.sin()) * (34.0 + rd::hash01(k * 5) * 18.0);
            let rot = a + 0.4;
            let corners: Vec<Vec2> = [
                vec2(-1.0, -1.0),
                vec2(1.0, -1.0),
                vec2(1.0, 1.0),
                vec2(-1.0, 1.0),
            ]
            .iter()
            .map(|v| {
                let (sn, cs) = rot.sin_cos();
                p + vec2(v.x * cs - v.y * sn, v.x * sn + v.y * cs) * 13.0
            })
            .collect();
            draw::fill_convex(&corners, Paint::Solid(Color::new(0.93, 0.96, 1.0, 0.55)));
            fill_circle(p + vec2(-4.0, -4.0), 3.0, Color::new(1.0, 1.0, 1.0, 0.7));
        }
        // tonic fizz
        for i in 0..24u32 {
            let ph = (t * (0.6 + rd::hash01(i) * 0.6) + rd::hash01(i * 3)).fract();
            let a = rd::hash01(i * 11 + 2) * TAU;
            let r = rd::hash01(i * 7 + 9).sqrt() * (CONE - 8.0);
            fill_circle(
                c + vec2(a.cos(), a.sin()) * r,
                0.8 + ph * 1.6,
                Color::new(1.0, 1.0, 1.0, 0.5 * (1.0 - ph)),
            );
        }
        draw::arc(c, CONE + 3.0, 3.6, 5.2, 2.0, Color::new(1.0, 1.0, 1.0, 0.6));
    }

    fn draw_carafe(&self, ui: &mut Ui, o: Vec2, t: f32, text: &MiniGameText) {
        let th = ui.theme.clone();
        let r = CARAFE.offset(o);
        contact_shadow(
            vec2(r.center().x + 3.0, r.bottom() + 2.0),
            r.w * 0.7,
            9.0,
            0.5,
        );
        let body = Rect::new(r.x, r.y + 26.0, r.w, r.h - 26.0);
        fill_rrect(body, 24.0, Color::new(0.8, 0.78, 1.0, 0.1));
        let h = (body.h - 10.0) * self.drip;
        if h > 1.0 {
            let f = Rect::new(body.x + 4.0, body.bottom() - 5.0 - h, body.w - 8.0, h);
            fill_rrect(
                f,
                20.0f32.min(h / 2.0),
                Paint::vertical(f, hex(0x8a5530), hex(0x3d1f12)),
            );
            draw::hline(
                f.x + 8.0,
                f.right() - 8.0,
                f.y + 1.0,
                Color::new(1.0, 0.85, 0.6, 0.4),
            );
        }
        let neck = Rect::new(r.x + 18.0, r.y + 4.0, r.w - 36.0, 26.0);
        fill_rrect(neck, 6.0, Color::new(0.8, 0.78, 1.0, 0.12));
        let collar = Rect::new(r.x + 13.0, r.y + 20.0, r.w - 26.0, 13.0);
        fill_rrect(
            collar,
            5.0,
            Paint::vertical(collar, hex(0xa8764f), hex(0x6e4532)),
        );
        draw::stroke_rrect(body, 24.0, 1.2, Color::new(0.9, 0.88, 1.0, 0.35));
        draw::line(
            vec2(body.x + 10.0, body.y + 18.0),
            vec2(body.x + 10.0, body.bottom() - 24.0),
            2.0,
            Color::new(1.0, 1.0, 1.0, 0.25),
        );
        if self.flow > 0.05 || self.drip < (self.water / self.need).min(1.0) - 0.01 {
            let ph = (t * 1.7).fract();
            let dy = r.y + 6.0 + ph * 40.0;
            fill_circle(
                vec2(r.center().x, dy),
                2.0,
                with_alpha(hex(0x6b3e22), 1.0 - ph),
            );
        }
        let pct = format!("{}%", (self.drip * 100.0).round() as i32);
        let st = th.small_medium().tabular();
        ui.fonts.draw_in(
            &pct,
            Rect::new(r.x - 10.0, r.bottom() + 8.0, r.w + 20.0, 16.0),
            &st,
            Align::Center,
        );
        let micro = th.micro();
        ui.fonts.draw_in(
            &text.get("minigame.water").to_uppercase(),
            Rect::new(r.x - 20.0, r.y - 20.0, r.w + 40.0, 14.0),
            &micro,
            Align::Center,
        );
    }
}
