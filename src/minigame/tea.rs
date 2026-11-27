//! TeaSteep (§2.4): a tea bag in a glass mug; the colour spreads in soft plumes. Lift the
//! bag (click / Space) when it matches the swatch — a 1–5 strength scale and a tiny sand
//! timer make it colour-blind safe. Drag the bag up and down (or hold Up/Down) to dunk and
//! steep faster. Then the add-ons: drop the swinging lemon slice, add sugar cubes.
//! Quality: `clamp(1 − |strength − target| / tol) × mean(add-on accuracies)`.
//!
//! Each tea is itself: the colour and the steeping speed follow the tea kind (linden opens
//! slowly and stays pale gold, uzvar turns deep amber); linden, Tatra tea and uzvar take a
//! spoonful of honey; masala chai simmers in a little pan — first the spices in the order the
//! recipe card shows (the order changes per play), then the tea, then warm milk poured to the
//! caramel mark. The target, the lemon's swing and the card drift a little every play.

use std::f32::consts::PI;

use macroquad::prelude::*;

use super::paint::{contact_shadow, ellipse_arc, hex, with_alpha};
use super::score::{self, widen};
use super::{Common, Ctx, Hint, MgInput, MiniGameSound, MiniGameText, Rng, TeaKind, TeaTuning};
use crate::render::draw as rd;
use crate::ui::draw::{
    self, Paint, fill_circle, fill_convex, fill_rrect, glow_circle, stroke_circle,
};
use crate::ui::{Align, Glass, RectExt, Ui, approach, draw_glass, ease, ease_out};

const MX: f32 = 188.0;
const BOTTOM: f32 = 228.0;
const H: f32 = 160.0;
const HW: f32 = 60.0;
const WATER: f32 = 0.8;
const DUNK_T: f32 = 0.7;
const LIFT_ANIM: f32 = 1.0;
/// Chai's pan: the liquid's surface and half-widths.
const PAN_SURF: f32 = 168.0;
const PAN_HW: f32 = 96.0;
const PAN_RY: f32 = 17.0;
/// The spice tray (chai), stage-local column on the left.
const TRAY_X: f32 = 40.0;
const TRAY_Y0: f32 = 46.0;
const TRAY_STEP: f32 = 46.0;
const MILK_RATE: f32 = 0.34;
const BOWL: Vec2 = Vec2::new(46.0, 226.0);
const PANEL: Rect = Rect {
    x: 330.0,
    y: 12.0,
    w: 148.0,
    h: 224.0,
};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    /// Chai: the spices in the card's order.
    Spices,
    Steep,
    Lemon,
    Honey,
    Sugar,
    /// Chai: warm milk to the caramel mark.
    Milk,
    Finished,
}

/// Chai spices on the tray.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Spice {
    Cinnamon,
    Cardamom,
    Ginger,
    Clove,
}

impl Spice {
    const ALL: [Spice; 4] = [
        Spice::Cinnamon,
        Spice::Cardamom,
        Spice::Ginger,
        Spice::Clove,
    ];
    fn id(self) -> &'static str {
        match self {
            Spice::Cinnamon => "cinnamon",
            Spice::Cardamom => "cardamom",
            Spice::Ginger => "ginger",
            Spice::Clove => "clove",
        }
    }
    /// `minigame.spice.<id>`
    pub fn key(self) -> String {
        format!("minigame.spice.{}", self.id())
    }
    fn color(self) -> Color {
        match self {
            Spice::Cinnamon => hex(0xa0582a),
            Spice::Cardamom => hex(0x8fae5a),
            Spice::Ginger => hex(0xe0b368),
            Spice::Clove => hex(0x4a2a1c),
        }
    }
}

/// A swinging drop over the cup (lemon slice, honey spoon).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Drop {
    at: Option<(f32, f32)>,
    acc: f32,
}

impl Drop {
    const NEW: Drop = Drop { at: None, acc: 1.0 };
}

struct Wisp {
    p: Vec2,
    v: Vec2,
    age: f32,
    life: f32,
    r: f32,
}

struct Cube {
    x: f32,
    born: f32,
}

pub(crate) struct Tea {
    kind: TeaKind,
    tu: TeaTuning,
    common: Common,
    phase: Phase,
    phase_t: f32,
    tau: f32,
    swing_speed: f32,
    swing_phase: f32,
    /// The recipe card's order (chai).
    card: Vec<Spice>,
    /// The tray's order (chai).
    tray: Vec<Spice>,
    added: Vec<Spice>,
    wiggle: Option<(usize, f32)>,
    spice_sel: usize,
    spice_miss: u32,
    last_spice: f32,
    honey: Drop,
    milk: f32,
    milk_target: f32,
    milk_started: bool,
    milk_pouring: bool,
    milk_released: f32,
    flow: f32,
    steep: f32,
    dunk: f32,
    bag_y: f32,
    dragging: Option<(f32, f32)>,
    strength_at_lift: f32,
    lifted_at: Option<f32>,
    lemon_drop: Option<(f32, f32)>,
    lemon_acc: f32,
    cubes: Vec<Cube>,
    last_cube: f32,
    wisps: Vec<Wisp>,
    acc: f32,
    rng: Rng,
    match_t: f32,
    t: f32,
}

fn deep_color(k: TeaKind) -> Color {
    match k {
        TeaKind::Black => hex(0x7a300c),
        TeaKind::Green => hex(0x7f9a2e),
        TeaKind::Herbal => hex(0xc07d10),
        TeaKind::Fruit => hex(0x8f1530),
        TeaKind::Linden => hex(0xd09a2a),
        TeaKind::Mountain => hex(0x9a7a1c),
        TeaKind::Uzvar => hex(0x8a3a14),
    }
}

const CLEAR: Color = Color::new(0.93, 0.9, 0.86, 1.0);

fn y_of(l: f32) -> f32 {
    BOTTOM - 10.0 - (H - 16.0) * l
}

/// Strength on the 1–5 scale after `steep` seconds with time constant `tau`.
fn strength_tau(steep: f32, tau: f32) -> f32 {
    1.0 + 4.0 * (1.0 - (-steep / tau).exp())
}

/// Seconds of steeping to reach `s`.
fn time_for_tau(s: f32, tau: f32) -> f32 {
    let k = ((s - 1.0) / 4.0).clamp(0.0, 0.98);
    -tau * (1.0 - k).ln()
}

const MILK_C: Color = Color::new(0.96, 0.93, 0.87, 1.0);

impl Tea {
    pub fn new(rng: &mut Rng, kind: TeaKind, mut tu: TeaTuning, common: Common) -> Tea {
        // a little drift every play
        tu.target = (tu.target + rng.range(-0.25, 0.25)).clamp(1.4, 4.6);
        let mut pool = Spice::ALL.to_vec();
        let mut card = Vec::new();
        for _ in 0..(tu.spices as usize).min(pool.len()) {
            let i = (rng.next_u64() % pool.len() as u64) as usize;
            card.push(pool.remove(i));
        }
        let mut tray = Spice::ALL.to_vec();
        for i in (1..tray.len()).rev() {
            let j = (rng.next_u64() % (i as u64 + 1)) as usize;
            tray.swap(i, j);
        }
        Tea {
            kind,
            tu,
            common,
            phase: if tu.spices > 0 {
                Phase::Spices
            } else {
                Phase::Steep
            },
            phase_t: 0.0,
            tau: kind.tau(),
            swing_speed: rng.range(1.45, 1.95),
            swing_phase: rng.range(0.0, std::f32::consts::TAU),
            card,
            tray,
            added: Vec::new(),
            wiggle: None,
            spice_sel: 0,
            spice_miss: 0,
            last_spice: 0.0,
            honey: Drop::NEW,
            milk: 0.0,
            milk_target: rng.range(0.42, 0.58),
            milk_started: false,
            milk_pouring: false,
            milk_released: 0.0,
            flow: 0.0,
            steep: 0.0,
            dunk: 0.0,
            bag_y: 0.32,
            dragging: None,
            strength_at_lift: 1.0,
            lifted_at: None,
            lemon_drop: None,
            lemon_acc: 1.0,
            cubes: Vec::new(),
            last_cube: 0.0,
            wisps: Vec::new(),
            acc: 0.0,
            rng: Rng::new(rng.next_u64()),
            match_t: 0.0,
            t: 0.0,
        }
    }

    pub fn focus(&self) -> Vec2 {
        if self.tu.pan {
            vec2(MX, PAN_SURF - 20.0)
        } else {
            vec2(MX, y_of(WATER) - 20.0)
        }
    }

    pub fn flow(&self) -> f32 {
        self.flow
    }

    fn strength(&self, steep: f32) -> f32 {
        strength_tau(steep, self.tau)
    }

    fn time_for(&self, s: f32) -> f32 {
        time_for_tau(s, self.tau)
    }

    fn strength_now(&self) -> f32 {
        if self.lifted_at.is_some() {
            self.strength_at_lift
        } else {
            self.strength(self.steep)
        }
    }

    fn color_at(&self, s: f32) -> Color {
        let k = ((s - 1.0) / 4.0).clamp(0.0, 1.0);
        let c = draw::mix(CLEAR, deep_color(self.kind), k.powf(0.8));
        let c = draw::mix(c, MILK_C, (self.milk * 1.1).min(0.85));
        Color { a: 1.0, ..c }
    }

    fn lemon_x(&self, t: f32) -> f32 {
        MX + (t * self.swing_speed + self.swing_phase).sin() * 58.0
    }

    /// Where the tray chip `i` sits (chai).
    fn tray_pos(&self, i: usize) -> Vec2 {
        vec2(TRAY_X, TRAY_Y0 + i as f32 * TRAY_STEP)
    }

    fn spice_score(&self) -> f32 {
        (1.0 - 0.15 * self.spice_miss as f32).max(0.4)
    }

    fn milk_acc(&self, relaxed: bool) -> f32 {
        (1.0 - (self.milk - self.milk_target).abs() / (0.22 * widen(relaxed))).clamp(0.0, 1.0)
    }

    #[cfg(test)]
    pub fn card_ids(&self) -> String {
        self.card
            .iter()
            .map(|s| s.id())
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn steps(&self) -> Vec<&'static str> {
        let mut v = Vec::new();
        if self.tu.spices > 0 {
            v.push("tea_steep.spices");
        }
        if self.tu.honey {
            v.push("tea_steep.honey");
        }
        if self.tu.milk {
            v.push("tea_steep.milk");
        }
        v
    }

    pub fn preview(&self, relaxed: bool) -> f32 {
        self.quality_with(relaxed, true)
    }

    pub fn advice(&self, relaxed: bool) -> &'static str {
        let s = self.strength_now();
        let steep = 1.0 - ((s - self.tu.target).abs() / (self.tu.tol * widen(relaxed))).min(1.0);
        let mut worst = (
            1.0 - steep,
            if s < self.tu.target {
                "minigame.why.tea.weak"
            } else {
                "minigame.why.tea.strong"
            },
        );
        let mut consider = |v: f32, k: &'static str| {
            if 1.0 - v > worst.0 + 0.05 {
                worst = (1.0 - v, k);
            }
        };
        if self.tu.spices > 0 {
            consider(self.spice_score(), "minigame.why.tea.spices");
        }
        if self.tu.lemon {
            consider(self.lemon_acc, "minigame.why.drop");
        }
        if self.tu.honey {
            consider(self.honey.acc, "minigame.why.drop");
        }
        if self.tu.sugar > 0 {
            consider(
                score::sugar(self.cubes.len().min(255) as u8, self.tu.sugar),
                "minigame.why.tea.sugar",
            );
        }
        if self.tu.milk {
            consider(self.milk_acc(relaxed), "minigame.why.tea.milk");
        }
        worst.1
    }

    fn bag_pos(&self, t: f32) -> Vec2 {
        let dunk_in = ease_out(t / DUNK_T);
        let sway = (t * 0.9).sin() * 4.0;
        let rest = vec2(MX - 6.0 + sway, y_of(self.bag_y));
        let above = vec2(MX + 4.0, y_of(1.12));
        let up = vec2(MX + 10.0, y_of(1.22));
        let rest_plate = vec2(MX + 98.0, y_of(0.2));
        let p = above.lerp(rest, dunk_in);
        match self.lifted_at {
            // up out of the water, then over to the little saucer
            Some(l) => {
                let k = ((t - l) / LIFT_ANIM).clamp(0.0, 1.0);
                if k < 0.5 {
                    p.lerp(up, ease(k * 2.0))
                } else {
                    up.lerp(rest_plate, ease(k * 2.0 - 1.0))
                }
            }
            None => p,
        }
    }

    pub fn finished_gesture(&self) -> bool {
        match self.phase {
            Phase::Finished => true,
            Phase::Sugar => self.cubes.len() >= self.tu.sugar as usize && !self.tu.milk,
            Phase::Milk => self.milk_started && !self.milk_pouring,
            _ => false,
        }
    }

    /// Mean add-on accuracy; `pending` steps count as perfect (the live preview).
    fn addons(&self, relaxed: bool, pending: bool) -> f32 {
        let past = |p: Phase| -> bool {
            let order = [
                Phase::Spices,
                Phase::Steep,
                Phase::Lemon,
                Phase::Honey,
                Phase::Sugar,
                Phase::Milk,
                Phase::Finished,
            ];
            let i = order.iter().position(|x| *x == self.phase).unwrap_or(0);
            let j = order.iter().position(|x| *x == p).unwrap_or(0);
            i > j || (i == j && !pending)
        };
        let mut v = Vec::new();
        if self.tu.spices > 0 {
            v.push(self.spice_score());
        }
        if self.tu.lemon {
            v.push(if past(Phase::Lemon) {
                self.lemon_acc
            } else {
                1.0
            });
        }
        if self.tu.honey {
            v.push(if past(Phase::Honey) {
                self.honey.acc
            } else {
                1.0
            });
        }
        if self.tu.sugar > 0 {
            v.push(if past(Phase::Sugar) || !self.cubes.is_empty() {
                score::sugar(self.cubes.len().min(255) as u8, self.tu.sugar)
            } else {
                1.0
            });
        }
        if self.tu.milk {
            v.push(if past(Phase::Milk) || self.milk_started {
                self.milk_acc(relaxed)
            } else if pending {
                1.0
            } else {
                0.5
            });
        }
        if v.is_empty() {
            1.0
        } else {
            v.iter().sum::<f32>() / v.len() as f32
        }
    }

    fn quality_with(&self, relaxed: bool, pending: bool) -> f32 {
        score::tea(
            self.strength_now(),
            self.tu.target,
            self.tu.tol,
            self.addons(relaxed, pending),
            relaxed,
        )
    }

    fn quality(&self, relaxed: bool) -> f32 {
        self.quality_with(relaxed, false)
    }

    fn lift(&mut self, ctx: &mut Ctx) {
        self.strength_at_lift = self.strength(self.steep);
        self.lifted_at = Some(ctx.t);
        self.dragging = None;
        ctx.sounds.push(MiniGameSound::Pour);
    }

    fn next_phase(&mut self, ctx: &mut Ctx) {
        self.phase_t = 0.0;
        let order = [
            (Phase::Steep, true),
            (Phase::Lemon, self.tu.lemon),
            (Phase::Honey, self.tu.honey),
            (Phase::Sugar, self.tu.sugar > 0),
            (Phase::Milk, self.tu.milk),
            (Phase::Finished, true),
        ];
        let from = order
            .iter()
            .position(|(p, _)| *p == self.phase)
            .map_or(0, |i| i + 1);
        self.phase = order[from..]
            .iter()
            .find(|(_, on)| *on)
            .map_or(Phase::Finished, |(p, _)| *p);
        if self.phase == Phase::Finished {
            ctx.sounds.push(MiniGameSound::Clink);
        }
    }

    /// A spice from the tray goes into the pan (right one in the card's order, or a wiggle).
    fn add_spice(&mut self, i: usize, ctx: &mut Ctx) {
        let Some(sp) = self.tray.get(i).copied() else {
            return;
        };
        if self.added.contains(&sp) {
            return;
        }
        let next = self.card.get(self.added.len()).copied();
        if next == Some(sp) {
            self.added.push(sp);
            self.last_spice = ctx.t;
            ctx.sounds.push(MiniGameSound::Pour);
            ctx.sounds.push(MiniGameSound::Cue);
            if self.added.len() >= self.card.len() {
                self.phase = Phase::Steep;
                self.phase_t = 0.0;
                self.steep = 0.0;
            }
        } else {
            self.spice_miss += 1;
            self.wiggle = Some((i, ctx.t));
            self.last_spice = ctx.t;
            ctx.sounds.push(MiniGameSound::Clink);
        }
    }

    pub fn step(&mut self, ctx: &mut Ctx, input: &MgInput, dt: f32) -> Option<f32> {
        self.t = ctx.t;
        self.phase_t += dt;
        let act = input.pressed || input.hold_pressed;
        self.flow = approach(
            self.flow,
            if self.milk_pouring { 1.0 } else { 0.0 },
            dt,
            0.12,
        );
        // the bag dunks in when steeping starts (after the spices for chai)
        let st = if self.tu.spices > 0 {
            self.phase_t
        } else {
            ctx.t
        };
        match self.phase {
            Phase::Spices => {
                let n = self.tray.len();
                if input.left || input.up {
                    self.spice_sel = (self.spice_sel + n - 1) % n;
                }
                if input.right || input.down_tap {
                    self.spice_sel = (self.spice_sel + 1) % n;
                }
                if input.pressed
                    && let Some(p) = input.pointer
                    && let Some(i) = (0..n).find(|i| p.distance(self.tray_pos(*i)) < 20.0)
                {
                    self.spice_sel = i;
                    self.add_spice(i, ctx);
                } else if input.hold_pressed && self.phase_t > 0.2 {
                    self.add_spice(self.spice_sel, ctx);
                } else if !self.common.relaxed && ctx.t - self.last_spice > 6.0 {
                    // nobody is rushed: grandma's hand adds the next one
                    if let Some(next) = self.card.get(self.added.len())
                        && let Some(i) = self.tray.iter().position(|s| s == next)
                    {
                        self.spice_miss += 1;
                        self.add_spice(i, ctx);
                    }
                }
            }
            Phase::Steep => {
                if self.lifted_at.is_none() {
                    // dunking: drag the bag or hold Up/Down
                    let mut speed = 1.0;
                    if input.axis.y != 0.0 {
                        self.bag_y = (self.bag_y - input.axis.y * dt * 0.8).clamp(0.12, 0.6);
                        speed = 1.8;
                    }
                    if let (Some(p), Some((y0, b0))) = (input.pointer, self.dragging)
                        && input.down
                    {
                        let ny = (b0 - (p.y - y0) / (H - 16.0)).clamp(0.12, 0.6);
                        if (ny - self.bag_y).abs() > 1e-3 {
                            speed = 1.8;
                        }
                        self.bag_y = ny;
                    }
                    self.dunk = approach(self.dunk, speed - 1.0, dt, 0.2);
                    if st > DUNK_T {
                        self.steep += dt * (1.0 + self.dunk);
                    }
                    let close = (self.strength(self.steep) - self.tu.target).abs()
                        < 0.35 * widen(ctx.relaxed);
                    self.match_t = approach(self.match_t, if close { 1.0 } else { 0.0 }, dt, 0.2);
                    // click on the bag starts a dunk drag; a click elsewhere (or Space) lifts
                    if input.pressed && st > DUNK_T {
                        let on_bag = input
                            .pointer
                            .is_some_and(|p| p.distance(self.bag_pos(ctx.t)) < 22.0);
                        if on_bag {
                            self.dragging = Some((input.pointer.map_or(0.0, |p| p.y), self.bag_y));
                        } else {
                            self.lift(ctx);
                        }
                    } else if input.hold_pressed && st > DUNK_T {
                        self.lift(ctx);
                    }
                    if self.dragging.is_some() && (input.released || !input.down) {
                        self.dragging = None;
                    }
                    let auto =
                        !self.common.relaxed && self.steep > self.time_for(self.tu.target) + 6.0;
                    if auto {
                        self.lift(ctx);
                    }
                } else {
                    self.match_t = approach(self.match_t, 0.0, dt, 0.2);
                    if ctx.t - self.lifted_at.unwrap_or(ctx.t) > LIFT_ANIM {
                        self.next_phase(ctx);
                    }
                }
            }
            Phase::Lemon => {
                if let Some((_, at)) = self.lemon_drop {
                    if ctx.t - at > 0.6 {
                        self.next_phase(ctx);
                    }
                } else if act || (!self.common.relaxed && self.phase_t > 8.0) {
                    let x = self.lemon_x(ctx.t);
                    self.lemon_acc = score::lemon(x - MX, ctx.relaxed);
                    self.lemon_drop = Some((x, ctx.t));
                    ctx.sounds.push(MiniGameSound::Pour);
                    if self.lemon_acc > 0.6 {
                        ctx.sounds.push(MiniGameSound::Cue);
                    }
                }
            }
            Phase::Honey => {
                if let Some((_, at)) = self.honey.at {
                    if ctx.t - at > 0.6 {
                        self.next_phase(ctx);
                    }
                } else if act || (!self.common.relaxed && self.phase_t > 8.0) {
                    let x = self.lemon_x(ctx.t);
                    self.honey.acc = score::lemon(x - MX, ctx.relaxed);
                    self.honey.at = Some((x, ctx.t));
                    ctx.sounds.push(MiniGameSound::Pour);
                    if self.honey.acc > 0.6 {
                        ctx.sounds.push(MiniGameSound::Cue);
                    }
                }
            }
            Phase::Milk => {
                let want =
                    (input.down && input.pointer.is_some_and(|p| p.x < PANEL.x)) || input.hold;
                if want && self.milk < 1.0 {
                    if !self.milk_pouring {
                        ctx.sounds.push(MiniGameSound::Pour);
                    }
                    self.milk_pouring = true;
                    self.milk_started = true;
                    self.milk = (self.milk + MILK_RATE * dt).min(1.0);
                    self.milk_released = 0.0;
                } else {
                    self.milk_pouring = false;
                    self.milk_released += dt;
                }
                let settled = self.milk_started && self.milk_released > 0.8;
                let idle = !self.milk_started && self.phase_t > 7.0;
                if !self.common.relaxed && (settled || idle || self.milk >= 1.0) {
                    self.next_phase(ctx);
                }
            }
            Phase::Sugar => {
                let on_bowl = input
                    .pointer
                    .is_some_and(|p| p.distance(BOWL + vec2(0.0, -12.0)) < 34.0);
                let add = (input.pressed
                    && (on_bowl || input.pointer.is_some_and(|p| p.x < 330.0)))
                    || input.hold_pressed;
                if add && self.cubes.len() < 9 {
                    self.cubes.push(Cube {
                        x: MX + self.rng.range(-26.0, 26.0),
                        born: ctx.t,
                    });
                    self.last_cube = ctx.t;
                    ctx.sounds.push(MiniGameSound::Pour);
                }
                let idle = ctx.t - self.last_cube.max(ctx.t - self.phase_t);
                let settled =
                    (self.cubes.len() == self.tu.sugar as usize && idle > 1.2) || idle > 5.0;
                if !self.common.relaxed && (settled || input.confirm) {
                    self.next_phase(ctx);
                }
                if self.common.relaxed && self.tu.milk && settled {
                    self.next_phase(ctx);
                }
            }
            Phase::Finished => {}
        }
        // colour plumes from the bag while it is in the water
        if self.lifted_at.is_none() && self.phase == Phase::Steep && st > DUNK_T {
            let fresh =
                (1.0 - (self.strength(self.steep) - 1.0) / 4.0).max(0.15) * (1.0 + self.dunk);
            self.acc += dt * 12.0 * fresh;
            while self.acc >= 1.0 {
                self.acc -= 1.0;
                let b = self.bag_pos(ctx.t);
                self.wisps.push(Wisp {
                    p: b + vec2(self.rng.range(-12.0, 12.0), self.rng.range(0.0, 16.0)),
                    v: vec2(self.rng.range(-16.0, 16.0), self.rng.range(4.0, 22.0)),
                    age: 0.0,
                    life: self.rng.range(1.6, 2.8),
                    r: self.rng.range(8.0, 14.0),
                });
            }
        }
        let (xl, xr, yb) = if self.tu.pan {
            (MX - PAN_HW + 20.0, MX + PAN_HW - 20.0, PAN_SURF + 10.0)
        } else {
            (MX - HW + 14.0, MX + HW - 14.0, y_of(0.05))
        };
        for w in &mut self.wisps {
            w.age += dt;
            w.p += w.v * dt;
            w.v *= (-0.8 * dt).exp();
            w.p.x = w.p.x.clamp(xl, xr);
            w.p.y = w.p.y.min(yb);
        }
        self.wisps.retain(|w| w.age < w.life);

        let finish = if self.common.relaxed {
            input.confirm
        } else {
            self.phase == Phase::Finished && self.phase_t > 0.3
        };
        if finish {
            if self.lifted_at.is_none() {
                self.strength_at_lift = self.strength(self.steep);
                self.lifted_at = Some(ctx.t);
            }
            self.milk_pouring = false;
            return Some(self.quality(ctx.relaxed));
        }
        None
    }

    pub fn autoplay(&self, t: f32, keyboard: bool) -> MgInput {
        if self.phase == Phase::Spices {
            let next = self.card.get(self.added.len()).copied();
            let target = next.and_then(|n| self.tray.iter().position(|s| *s == n));
            let ready = t - self.last_spice > 0.55 && self.phase_t > 0.5;
            if keyboard {
                return match target {
                    Some(i) if i != self.spice_sel => MgInput {
                        right: ready && ((t * 60.0).round() as u32).is_multiple_of(20),
                        ..Default::default()
                    },
                    Some(_) => MgInput {
                        hold_pressed: ready,
                        ..Default::default()
                    },
                    None => MgInput::default(),
                };
            }
            return MgInput {
                pointer: target.map(|i| self.tray_pos(i)),
                pressed: ready && target.is_some(),
                ..Default::default()
            };
        }
        if self.phase == Phase::Milk {
            let pour = self.milk < self.milk_target - 0.004 && self.phase_t > 0.4;
            return if keyboard {
                MgInput {
                    hold: pour,
                    ..Default::default()
                }
            } else {
                MgInput {
                    pointer: Some(vec2(MX, 60.0)),
                    pressed: pour && !self.milk_pouring,
                    down: pour,
                    ..Default::default()
                }
            };
        }
        let st = if self.tu.spices > 0 { self.phase_t } else { t };
        let act = match self.phase {
            Phase::Steep => {
                self.lifted_at.is_none()
                    && st > DUNK_T + 0.3
                    && self.strength(self.steep) >= self.tu.target
            }
            Phase::Lemon => {
                self.lemon_drop.is_none()
                    && self.phase_t > 0.4
                    && (self.lemon_x(t) - MX).abs() < 4.0
            }
            Phase::Honey => {
                self.honey.at.is_none() && self.phase_t > 0.4 && (self.lemon_x(t) - MX).abs() < 4.0
            }
            Phase::Sugar => {
                self.cubes.len() < self.tu.sugar as usize
                    && t - self.last_cube > 0.45
                    && self.phase_t > 0.3
            }
            _ => false,
        };
        let finished = self.finished_gesture() && self.phase == Phase::Finished;
        if keyboard {
            MgInput {
                hold_pressed: act,
                confirm: finished,
                ..Default::default()
            }
        } else {
            let p = if self.phase == Phase::Sugar {
                BOWL + vec2(0.0, -12.0)
            } else {
                vec2(MX, 40.0)
            };
            MgInput {
                pointer: Some(p),
                pressed: act,
                ..Default::default()
            }
        }
    }

    pub fn hint(&self) -> Hint<'static> {
        let done = self.common.relaxed || (self.phase == Phase::Sugar && !self.tu.milk);
        let near = (self.lemon_x(self.t) - MX).abs() < 10.0;
        let (key, emphasis) = match self.phase {
            Phase::Spices => ("minigame.tea_steep.prompt.spices", false),
            Phase::Steep if self.match_t > 0.5 => ("minigame.tea_steep.prompt.ready", true),
            Phase::Steep if self.tu.pan => ("minigame.tea_steep.prompt.lift_pan", false),
            Phase::Steep | Phase::Finished => ("minigame.tea_steep.prompt.lift", false),
            Phase::Lemon => ("minigame.tea_steep.prompt.lemon", near),
            Phase::Honey => ("minigame.tea_steep.prompt.honey", near),
            Phase::Sugar => ("minigame.tea_steep.prompt.sugar", false),
            Phase::Milk => (
                "minigame.tea_steep.prompt.milk",
                self.milk_pouring && (self.milk - self.milk_target).abs() < 0.06,
            ),
        };
        Hint {
            key,
            emphasis,
            keys: if self.phase == Phase::Spices {
                &["←", "→", "Space"]
            } else {
                &["Space"]
            },
            done,
        }
    }

    pub fn draw(&self, ui: &mut Ui, o: Vec2, t: f32, text: &MiniGameText) {
        if self.tu.pan {
            self.draw_pan(o, t);
        } else {
            self.draw_mug(o, t);
        }
        self.draw_honey(o, t);
        if self.tu.spices > 0 {
            self.draw_tray(ui, o, t, text);
        }
        if self.phase == Phase::Milk || self.flow > 0.02 {
            self.draw_milk_jug(o, t);
        }
        let col = self.color_at(self.strength_now());
        self.draw_panel(ui, o, t, text, col);
    }

    fn draw_mug(&self, o: Vec2, t: f32) {
        let x = o.x + MX;
        let bottom = o.y + BOTTOM;
        let top = bottom - H;
        let s = self.strength_now();
        let col = self.color_at(s);
        let k = (s - 1.0) / 4.0;

        self.draw_bowl(o + BOWL, t);
        // the little saucer for the used tea bag
        let sp = o + vec2(MX + 98.0, y_of(0.2) + 20.0);
        contact_shadow(sp + vec2(2.0, 1.0), 26.0, 5.0, 0.4);
        rd::ellipse_v(sp.x, sp.y - 3.0, 24.0, 5.5, hex(0xe9e2f4), hex(0xa99dc6));
        contact_shadow(vec2(x + 4.0, bottom + 2.0), HW + 28.0, 13.0, 0.55);
        rd::radial(
            x + 18.0,
            bottom + 4.0,
            HW + 20.0,
            12.0,
            with_alpha(col, 0.35 * k),
            with_alpha(col, 0.0),
        );
        fill_rrect(
            Rect::new(x - HW, top, HW * 2.0, H),
            18.0,
            Color::new(0.78, 0.75, 1.0, 0.07),
        );

        let wy = o.y + y_of(WATER);
        let wb = bottom - 10.0;
        let water = Rect::new(x - HW + 6.0, wy, HW * 2.0 - 12.0, wb - wy);
        let top_c = self.color_at(1.0 + (s - 1.0) * 0.82);
        let bot_c = self.color_at((1.0 + (s - 1.0) * 1.12).min(5.0));
        let a = 0.38 + 0.5 * k;
        fill_rrect(
            water,
            13.0,
            Paint::vertical(water, with_alpha(top_c, a), with_alpha(bot_c, a + 0.06)),
        );
        for w in &self.wisps {
            let kk = w.age / w.life;
            let fade = (kk * (1.0 - kk) * 4.0).clamp(0.0, 1.0);
            rd::glow(
                o.x + w.p.x,
                o.y + w.p.y,
                w.r * (0.8 + kk * 1.8),
                with_alpha(deep_color(self.kind), 0.32 * fade),
            );
        }
        // sugar cubes sink and dissolve
        for c in &self.cubes {
            let age = t - c.born;
            let fall = ease_out(age / 0.5);
            let y = o.y + y_of(1.15) + (y_of(0.15) - y_of(1.15)) * fall;
            let dis = (1.0 - (age - 1.0).max(0.0) / 1.5).clamp(0.0, 1.0);
            if dis > 0.0 {
                let sz = 9.0 * (0.5 + 0.5 * dis);
                fill_rrect(
                    Rect::new(o.x + c.x - sz / 2.0, y - sz / 2.0, sz, sz),
                    2.0,
                    Color::new(1.0, 1.0, 1.0, 0.9 * dis),
                );
            }
        }
        rd::ellipse(
            x,
            wy,
            HW - 6.0,
            5.0,
            with_alpha(draw::mix(top_c, WHITE, 0.25), a),
        );
        draw::hline(
            x - HW + 14.0,
            x + HW - 14.0,
            wy - 1.0,
            Color::new(1.0, 1.0, 1.0, 0.25),
        );

        // lemon: swinging on a thread, then floating
        if self.tu.lemon && (self.phase == Phase::Lemon || self.lemon_drop.is_some()) {
            let (lx, ly) = match self.lemon_drop {
                Some((dx, at)) => {
                    let f = ease((t - at) / 0.5);
                    (dx, (o.y + 26.0) + (wy - 2.0 - (o.y + 26.0)) * f)
                }
                None => {
                    let lx = self.lemon_x(t);
                    draw::line(
                        vec2(o.x + MX, o.y - 4.0),
                        vec2(o.x + lx, o.y + 20.0),
                        1.0,
                        Color::new(1.0, 0.98, 0.9, 0.6),
                    );
                    (lx, o.y + 26.0)
                }
            };
            let lc = vec2(o.x + lx, ly);
            fill_circle(lc, 12.0, hex(0xf7d34a));
            fill_circle(lc, 10.0, hex(0xfbeaa0));
            for i in 0..8 {
                let a0 = i as f32 / 8.0 * 2.0 * PI;
                let d = vec2(a0.cos(), a0.sin());
                draw::line(lc, lc + d * 9.5, 1.0, Color::new(0.97, 0.85, 0.35, 0.8));
            }
            if self.lemon_drop.is_none() {
                // drop marker straight down over the mug's centre
                let pulse = 0.5 + 0.5 * (t * 3.0).sin();
                draw::line(
                    vec2(x, top - 18.0),
                    vec2(x, top - 6.0),
                    2.0,
                    Color::new(1.0, 0.85, 0.5, 0.4 + 0.3 * pulse),
                );
            }
        }

        // tea bag + string + tag
        let rim_pt = vec2(x + HW - 10.0, top + 2.0);
        let hang = vec2(x + HW + 14.0, top + 46.0 + (t * 1.1).sin() * 1.5);
        self.draw_bag(o, t, wy, rim_pt, hang, k);

        // front glass
        for (side, c) in [
            (-1.0f32, Color::new(0.92, 0.92, 1.0, 0.24)),
            (1.0, Color::new(0.78, 0.74, 1.0, 0.36)),
        ] {
            let wx = x + side * (HW - 3.0);
            fill_rrect(Rect::new(wx - 3.0, top + 6.0, 6.0, H - 22.0), 3.0, c);
        }
        let base = Rect::new(x - HW + 2.0, bottom - 12.0, HW * 2.0 - 4.0, 12.0);
        fill_rrect(
            base,
            8.0,
            Paint::vertical(
                base,
                Color::new(0.85, 0.82, 1.0, 0.25),
                Color::new(0.6, 0.56, 0.9, 0.35),
            ),
        );
        rd::ellipse(x, top + 2.0, HW, 5.0, Color::new(0.9, 0.9, 1.0, 0.18));
        draw::stroke_path(
            &ellipse_arc(vec2(x, top + 2.0), HW - 1.0, 5.0, 0.0, PI),
            false,
            1.5,
            Paint::Solid(Color::new(1.0, 1.0, 1.0, 0.45)),
        );
        let streak = Rect::new(x - HW + 14.0, top + 20.0, 8.0, H - 60.0);
        fill_rrect(
            streak,
            4.0,
            Paint::vertical(
                streak,
                Color::new(1.0, 1.0, 1.0, 0.3),
                Color::new(1.0, 1.0, 1.0, 0.03),
            ),
        );
        let hc = vec2(x - HW - 12.0, top + 74.0);
        draw::arc(hc, 26.0, 1.75, 4.55, 9.0, Color::new(0.8, 0.77, 1.0, 0.28));
        draw::arc(
            hc + vec2(-1.0, 0.0),
            28.0,
            2.4,
            3.9,
            1.6,
            Color::new(1.0, 1.0, 1.0, 0.35),
        );
        for i in 0..4u32 {
            let life = 2.4;
            let off = i as f32 / 4.0 * life;
            let aa = ((t + off) % life) / life;
            let sx = x - 30.0 + i as f32 * 20.0 + (t * 1.4 + i as f32).sin() * 6.0;
            let sy = top - 6.0 - aa * 44.0;
            let fade = (aa * (1.0 - aa) * 4.0).clamp(0.0, 1.0);
            rd::glow(
                sx,
                sy,
                11.0 + aa * 14.0,
                Color::new(0.95, 0.94, 1.0, 0.15 * fade),
            );
        }
    }

    /// The tea bag on its string (the tag hangs over the rim), lifted to the saucer.
    fn draw_bag(&self, o: Vec2, t: f32, wy: f32, rim_pt: Vec2, hang: Vec2, k: f32) {
        let bp = o + self.bag_pos(t);
        let lp = self
            .lifted_at
            .map_or(0.0, |l| ease(((t - l) / LIFT_ANIM - 0.4) / 0.6));
        let tag_c = hang.lerp(bp + vec2(24.0, 8.0), lp);
        let string = Color::new(0.95, 0.93, 0.88, 0.85);
        if lp < 0.5 {
            draw::line(bp + vec2(0.0, -16.0), rim_pt, 1.0, string);
            draw::line(rim_pt, tag_c + vec2(0.0, -12.0), 1.0, string);
        } else {
            draw::line(bp + vec2(0.0, -16.0), tag_c + vec2(0.0, -12.0), 1.0, string);
        }
        let bag = Rect::new(bp.x - 15.0, bp.y - 18.0, 30.0, 36.0);
        let submerged = bp.y > wy;
        let bag_top = draw::mix(
            hex(0xf1e6cf),
            hex(0xc9b48f),
            if submerged { 0.4 + 0.4 * k } else { 0.1 },
        );
        fill_rrect(
            bag,
            5.0,
            Paint::vertical(
                bag,
                bag_top,
                draw::mix(bag_top, deep_color(self.kind), 0.35),
            ),
        );
        for i in 0..10u32 {
            let lp = vec2(
                bag.x + 6.0 + rd::hash01(i * 3 + 1) * 18.0,
                bag.y + 14.0 + rd::hash01(i * 5 + 2) * 18.0,
            );
            fill_circle(
                lp,
                1.6 + rd::hash01(i) * 1.2,
                with_alpha(draw::mix(deep_color(self.kind), hex(0x2a160c), 0.4), 0.55),
            );
        }
        fill_convex(
            &[
                bag.point(),
                vec2(bag.right(), bag.y),
                vec2(bag.center().x, bag.y + 7.0),
            ],
            Paint::Solid(with_alpha(WHITE, 0.25)),
        );
        if self.dragging.is_some() || self.dunk > 0.1 {
            stroke_circle(bp, 24.0, 1.2, Color::new(0.85, 0.84, 1.0, 0.35));
        }
        if let Some(l) = self.lifted_at {
            let age = t - l;
            for kk in 0..3 {
                let ph = ((age * 1.6) + kk as f32 * 0.33).fract();
                let dp = vec2(
                    bag.center().x + (kk as f32 - 1.0) * 6.0,
                    bag.bottom() + ph * (wy - bag.bottom()).max(4.0),
                );
                fill_circle(dp, 1.8, with_alpha(deep_color(self.kind), 0.8 * (1.0 - ph)));
            }
        }
        let tag = Rect::new(tag_c.x - 9.0, tag_c.y - 12.0, 18.0, 22.0);
        fill_rrect(tag, 3.0, Paint::vertical(tag, hex(0xfaf3e6), hex(0xe3d3b8)));
        fill_circle(
            vec2(tag.center().x, tag.y + 13.0),
            4.0,
            with_alpha(deep_color(self.kind), 0.85),
        );
    }

    /// Chai's little pan (3/4 front): copper body, the tea surface, spices floating.
    fn draw_pan(&self, o: Vec2, t: f32) {
        let x = o.x + MX;
        let sy = o.y + PAN_SURF;
        let bottom = o.y + BOTTOM - 4.0;
        let s = self.strength_now();
        let col = self.color_at(s);
        // the stove's warm glow under it
        rd::radial(
            x,
            bottom + 2.0,
            PAN_HW + 26.0,
            16.0,
            Color::new(1.0, 0.55, 0.25, 0.25),
            Color::new(1.0, 0.55, 0.25, 0.0),
        );
        contact_shadow(vec2(x + 4.0, bottom + 2.0), PAN_HW + 20.0, 12.0, 0.55);
        // the used bag's saucer
        let sp = o + vec2(MX + 98.0, y_of(0.2) + 20.0);
        rd::ellipse_v(
            sp.x + 30.0,
            sp.y - 3.0,
            22.0,
            5.0,
            hex(0xe9e2f4),
            hex(0xa99dc6),
        );
        // back rim and the inside
        rd::ellipse(x, sy - 2.0, PAN_HW + 4.0, PAN_RY + 4.0, hex(0x6b4a3a));
        rd::ellipse(x, sy, PAN_HW, PAN_RY, hex(0x3b2a24));
        let a = 0.55 + 0.4 * ((s - 1.0) / 4.0).max(self.milk);
        rd::ellipse(x, sy + 2.0, PAN_HW - 6.0, PAN_RY - 3.0, with_alpha(col, a));
        for w in &self.wisps {
            let kk = w.age / w.life;
            let fade = (kk * (1.0 - kk) * 4.0).clamp(0.0, 1.0);
            rd::glow(
                o.x + w.p.x,
                sy + (w.p.y - PAN_SURF).clamp(-6.0, 8.0),
                w.r * (0.8 + kk * 1.8),
                with_alpha(deep_color(self.kind), 0.3 * fade),
            );
        }
        // the spices bob on the surface
        for (i, sp) in self.added.iter().enumerate() {
            let ang = i as f32 * 2.1 + t * 0.4;
            let p = vec2(x + ang.cos() * 46.0, sy + 2.0 + ang.sin() * 7.0);
            draw_spice(*sp, p, 0.8);
        }
        // milk swirl while it pours
        if self.milk > 0.01 {
            for k in 0..3 {
                let ph = t * 1.3 + k as f32 * 2.0;
                rd::arc(
                    vec2(x, sy + 2.0),
                    20.0 + k as f32 * 16.0,
                    6.0 - k as f32,
                    ph,
                    ph + 2.2,
                    2.0,
                    with_alpha(MILK_C, 0.45 * self.milk.min(0.6)),
                );
            }
        }
        // the bag (its string over the rim)
        let rim_pt = vec2(x + PAN_HW - 16.0, sy - 4.0);
        let hang = vec2(x + PAN_HW + 12.0, sy + 26.0 + (t * 1.1).sin() * 1.5);
        if self.phase != Phase::Spices {
            self.draw_bag(o, t, sy, rim_pt, hang, (s - 1.0) / 4.0);
        }
        // the front of the pan: copper, a dark band, a lit rim
        let body = [
            vec2(x - PAN_HW - 4.0, sy),
            vec2(x + PAN_HW + 4.0, sy),
            vec2(x + PAN_HW - 4.0, bottom - 6.0),
            vec2(x - PAN_HW + 4.0, bottom - 6.0),
        ];
        fill_convex(
            &body,
            Paint::vertical(
                Rect::new(x, sy, 1.0, bottom - sy),
                hex(0xd98a56),
                hex(0x8a4a2a),
            ),
        );
        rd::ellipse_v(
            x,
            bottom - 6.0,
            PAN_HW - 4.0,
            6.0,
            hex(0x8a4a2a),
            hex(0x5e301c),
        );
        draw::stroke_path(
            &ellipse_arc(vec2(x, sy), PAN_HW + 4.0, PAN_RY + 4.0, 0.0, PI),
            false,
            3.0,
            Paint::Solid(hex(0xf0b38a)),
        );
        draw::hline(
            x - PAN_HW + 10.0,
            x + PAN_HW - 10.0,
            sy + 30.0,
            Color::new(0.35, 0.16, 0.08, 0.35),
        );
        let streak = Rect::new(x - PAN_HW + 18.0, sy + 10.0, 10.0, bottom - sy - 22.0);
        fill_rrect(
            streak,
            5.0,
            Paint::vertical(
                streak,
                Color::new(1.0, 0.9, 0.8, 0.35),
                Color::new(1.0, 0.9, 0.8, 0.03),
            ),
        );
        // the handle, out to the left over the counter
        let h0 = vec2(x - PAN_HW - 2.0, sy + 14.0);
        rd::capsule(h0, h0 + vec2(-14.0, 24.0), 6.0, hex(0x3a2a24));
        rd::capsule(
            h0 + vec2(-1.0, -1.0),
            h0 + vec2(-13.0, 22.0),
            2.0,
            hex(0x6b5348),
        );
        // steam
        for i in 0..4u32 {
            let life = 2.4;
            let off = i as f32 / 4.0 * life;
            let aa = ((t + off) % life) / life;
            let sx = x - 40.0 + i as f32 * 26.0 + (t * 1.4 + i as f32).sin() * 6.0;
            let fade = (aa * (1.0 - aa) * 4.0).clamp(0.0, 1.0);
            rd::glow(
                sx,
                sy - 10.0 - aa * 44.0,
                11.0 + aa * 14.0,
                Color::new(0.95, 0.94, 1.0, 0.16 * fade),
            );
        }
    }

    /// The honey spoon: swings like the lemon, then a golden thread into the cup.
    fn draw_honey(&self, o: Vec2, t: f32) {
        if !self.tu.honey || !(self.phase == Phase::Honey || self.honey.at.is_some()) {
            return;
        }
        let wy = o.y + if self.tu.pan { PAN_SURF } else { y_of(WATER) };
        let (lx, ly) = match self.honey.at {
            Some((dx, at)) => {
                let f = ease((t - at) / 0.5);
                if f >= 1.0 {
                    rd::glow(o.x + dx, wy, 10.0, Color::new(1.0, 0.75, 0.25, 0.35));
                    return;
                }
                (dx, (o.y + 26.0) + (wy - 2.0 - (o.y + 26.0)) * f)
            }
            None => {
                let lx = self.lemon_x(t);
                draw::line(
                    vec2(o.x + MX, o.y - 4.0),
                    vec2(o.x + lx, o.y + 18.0),
                    1.0,
                    Color::new(1.0, 0.98, 0.9, 0.6),
                );
                let pulse = 0.5 + 0.5 * (t * 3.0).sin();
                let top = wy - 40.0;
                draw::line(
                    vec2(o.x + MX, top - 18.0),
                    vec2(o.x + MX, top - 6.0),
                    2.0,
                    Color::new(1.0, 0.85, 0.5, 0.4 + 0.3 * pulse),
                );
                (lx, o.y + 26.0)
            }
        };
        let c = vec2(o.x + lx, ly);
        rd::capsule(
            c + vec2(0.0, -16.0),
            c + vec2(0.0, -2.0),
            1.6,
            hex(0xb9b2cc),
        );
        rd::ellipse_v(c.x, c.y + 2.0, 9.0, 6.0, hex(0xc9c2dc), hex(0x8f88a8));
        rd::ellipse(c.x, c.y + 1.0, 7.0, 4.0, hex(0xf0b043));
        fill_circle(c + vec2(-2.0, 0.0), 1.6, Color::new(1.0, 0.95, 0.7, 0.8));
        if self.honey.at.is_none() {
            let drip = ((t * 1.5).fract()) * 6.0;
            fill_circle(c + vec2(0.0, 7.0 + drip), 1.5, hex(0xe39a2c));
        }
    }

    /// Chai: the spice tray on the left (selected chip ringed; a wrong one wiggles).
    fn draw_tray(&self, ui: &mut Ui, o: Vec2, t: f32, text: &MiniGameText) {
        let th = ui.theme.clone();
        let col = Rect::new(o.x + 8.0, o.y + 16.0, 64.0, 6.0 + TRAY_STEP * 4.0);
        draw_glass(&th, col, &Glass::card());
        for (i, sp) in self.tray.iter().enumerate() {
            let mut p = o + self.tray_pos(i);
            if let Some((wi, at)) = self.wiggle
                && wi == i
                && t - at < 0.45
            {
                p.x += ((t - at) * 40.0).sin() * 4.0 * (1.0 - (t - at) / 0.45);
            }
            let used = self.added.contains(sp);
            let sel = self.phase == Phase::Spices && i == self.spice_sel;
            if sel {
                stroke_circle(p, 21.0, 1.6, with_alpha(th.amber, 0.9));
            }
            fill_circle(
                p,
                17.0,
                Color::new(1.0, 1.0, 1.0, if used { 0.04 } else { 0.1 }),
            );
            if !used {
                draw_spice(*sp, p, 1.0);
            } else {
                draw_check(p, th.emerald_400);
            }
            let key = sp.key();
            let name = text.get(&key);
            let st = ui.fonts.fit_width(name, 62.0, &th.micro());
            ui.fonts.draw_in(
                name,
                Rect::new(p.x - 31.0, p.y + 13.0, 62.0, 12.0),
                &st,
                Align::Center,
            );
        }
    }

    /// The little milk jug tilting over the pan.
    fn draw_milk_jug(&self, o: Vec2, t: f32) {
        let x = o.x + MX + 46.0;
        let y = o.y + 40.0 - self.flow * 6.0;
        let tilt = self.flow * 0.5;
        let body = Rect::new(x - 16.0, y - 20.0, 32.0, 36.0);
        fill_rrect(
            body,
            8.0,
            Paint::vertical(body, hex(0xf4f0fa), hex(0xb7abd0)),
        );
        fill_convex(
            &[
                vec2(x - 16.0, y - 18.0),
                vec2(x - 26.0 - tilt * 8.0, y - 22.0 + tilt * 6.0),
                vec2(x - 14.0, y - 10.0),
            ],
            Paint::Solid(hex(0xe7e1f2)),
        );
        draw::arc(vec2(x + 18.0, y - 2.0), 9.0, -1.3, 1.3, 3.5, hex(0xd7cfe6));
        if self.flow > 0.02 {
            let from = vec2(x - 25.0 - tilt * 8.0, y - 20.0 + tilt * 6.0);
            let to_y = o.y + if self.tu.pan { PAN_SURF } else { y_of(WATER) };
            rd::capsule(
                from,
                vec2(from.x + 2.0, to_y),
                2.2 * self.flow + 0.4,
                with_alpha(MILK_C, 0.9),
            );
            let _ = t;
        }
    }

    fn draw_bowl(&self, b: Vec2, t: f32) {
        if self.tu.sugar == 0 {
            return;
        }
        contact_shadow(b + vec2(2.0, 1.0), 34.0, 7.0, 0.45);
        let body = [
            b + vec2(-26.0, -18.0),
            b + vec2(26.0, -18.0),
            b + vec2(18.0, 0.0),
            b + vec2(-18.0, 0.0),
        ];
        fill_convex(
            &body,
            Paint::vertical(
                Rect::new(b.x, b.y - 18.0, 1.0, 18.0),
                hex(0xf4f0fa),
                hex(0xb7abd0),
            ),
        );
        rd::ellipse(b.x, b.y - 18.0, 26.0, 6.0, hex(0xe7e1f2));
        rd::ellipse(b.x, b.y - 17.0, 22.0, 4.5, hex(0xcfc6e2));
        for i in 0..5u32 {
            let cx = b.x - 14.0 + i as f32 * 7.0;
            let cy = b.y - 20.0 - rd::hash01(i) * 3.0;
            fill_rrect(
                Rect::new(cx - 3.5, cy - 3.5, 7.0, 7.0),
                1.5,
                Color::new(1.0, 1.0, 1.0, 0.95),
            );
        }
        if self.phase == Phase::Sugar {
            let pulse = 0.5 + 0.5 * (t * 3.0).sin();
            glow_circle(
                b + vec2(0.0, -14.0),
                36.0,
                Color::new(1.0, 0.85, 0.55, 0.12 + 0.1 * pulse),
            );
        }
    }

    fn draw_panel(&self, ui: &mut Ui, o: Vec2, t: f32, text: &MiniGameText, now: Color) {
        let th = ui.theme.clone();
        let r = PANEL.offset(o);
        draw_glass(&th, r, &Glass::card());
        let micro = th.micro();
        if self.phase == Phase::Spices {
            // the recipe card: the spices in their order, ticked as they go in
            ui.fonts.draw(
                &text.get("minigame.tea_steep.card").to_uppercase(),
                r.x + 12.0,
                r.y + 20.0,
                &micro,
            );
            for (i, sp) in self.card.iter().enumerate() {
                let y = r.y + 44.0 + i as f32 * 40.0;
                let done = i < self.added.len();
                let next = i == self.added.len();
                if next {
                    let pulse = 0.5 + 0.5 * (t * 3.0).sin();
                    fill_rrect(
                        Rect::new(r.x + 6.0, y - 16.0, r.w - 12.0, 34.0),
                        8.0,
                        with_alpha(th.amber, 0.08 + 0.06 * pulse),
                    );
                }
                ui.fonts.draw(
                    &format!("{}.", i + 1),
                    r.x + 12.0,
                    y + 5.0,
                    &th.strong().tabular(),
                );
                draw_spice(*sp, vec2(r.x + 42.0, y), 0.9);
                let st = th.small_medium();
                let key = sp.key();
                let name = text.get(&key);
                let w = r.right() - (r.x + 58.0) - 22.0;
                let st = ui.fonts.fit_width(name, w, &st);
                ui.fonts.draw(name, r.x + 58.0, y + 5.0, &st);
                if done {
                    draw_check(vec2(r.right() - 14.0, y), th.emerald_400);
                }
            }
            let hint = text.get("minigame.tea_steep.card_hint");
            let hs = th.small().size(11.5).color(th.text_dim);
            let lines = ui.fonts.wrap(hint, r.w - 24.0, &hs);
            ui.fonts.draw_lines(
                &lines,
                Rect::new(r.x + 12.0, r.bottom() - 44.0, r.w - 24.0, 40.0),
                &hs,
                Align::Left,
                Some(3),
                usize::MAX,
            );
            return;
        }
        ui.fonts.draw(
            &text.get("minigame.target").to_uppercase(),
            r.x + 12.0,
            r.y + 20.0,
            &micro,
        );
        let key = self.kind.key();
        let st = ui.fonts.fit_width(text.get(&key), r.w - 24.0, &th.strong());
        let name = ui
            .fonts
            .ellipsize(text.get(&key), r.w - 24.0, &st)
            .into_owned();
        ui.fonts.draw(&name, r.x + 12.0, r.y + 38.0, &st);
        // swatch: target in the middle, the current colour as a ring
        let c = vec2(r.center().x, r.y + 84.0);
        let target = self.color_at(self.tu.target);
        glow_circle(c, 50.0, with_alpha(target, 0.25 + 0.2 * self.match_t));
        fill_circle(c, 32.0, now);
        stroke_circle(c, 32.0, 1.2, Color::new(1.0, 1.0, 1.0, 0.25));
        fill_circle(c, 21.0, Color::new(0.06, 0.03, 0.1, 0.4));
        fill_circle(c, 19.0, target);
        rd::glow(c.x - 7.0, c.y - 8.0, 14.0, Color::new(1.0, 1.0, 1.0, 0.18));
        if self.match_t > 0.02 {
            let pulse = 0.5 + 0.5 * (t * 5.0).sin();
            stroke_circle(
                c,
                37.0 + pulse * 3.0,
                2.0,
                with_alpha(th.amber, 0.7 * self.match_t),
            );
        }
        // 1–5 strength scale with the target and current markers (colour-blind safe)
        let bar = Rect::new(r.x + 14.0, r.y + 160.0, r.w - 28.0, 6.0);
        ui.fonts.draw(
            &text.get("minigame.strength").to_uppercase(),
            bar.x,
            bar.y - 15.0,
            &micro,
        );
        fill_rrect(
            bar,
            3.0,
            Paint::horizontal(bar, self.color_at(1.0), self.color_at(5.0)),
        );
        let xs = |v: f32| bar.x + bar.w * ((v - 1.0) / 4.0).clamp(0.0, 1.0);
        for n in 1..=5 {
            let nx = xs(n as f32);
            ui.fonts.draw_in(
                &n.to_string(),
                Rect::new(nx - 8.0, bar.bottom() + 3.0, 16.0, 14.0),
                &th.small().tabular(),
                Align::Center,
            );
        }
        let tol = self.tu.tol * widen(self.common.relaxed) * 0.35;
        let band = Rect::new(
            xs(self.tu.target - tol),
            bar.y - 3.0,
            xs(self.tu.target + tol) - xs(self.tu.target - tol),
            bar.h + 6.0,
        );
        draw::stroke_rrect(band, 4.0, 1.4, with_alpha(th.amber, 0.9));
        let cx = xs(self.strength_now());
        fill_convex(
            &[
                vec2(cx - 5.0, bar.y - 9.0),
                vec2(cx + 5.0, bar.y - 9.0),
                vec2(cx, bar.y - 2.0),
            ],
            Paint::Solid(th.text),
        );
        // tiny sand timer
        let hg = vec2(r.right() - 22.0, r.y + 26.0);
        let flow = (self.steep / (self.time_for(self.tu.target) * 1.6)).clamp(0.0, 1.0);
        fill_convex(
            &[
                hg + vec2(-6.0, -9.0),
                hg + vec2(6.0, -9.0),
                hg + vec2(0.0, 0.0),
            ],
            Paint::Solid(Color::new(1.0, 1.0, 1.0, 0.14)),
        );
        fill_convex(
            &[
                hg + vec2(0.0, 0.0),
                hg + vec2(6.0, 9.0),
                hg + vec2(-6.0, 9.0),
            ],
            Paint::Solid(Color::new(1.0, 1.0, 1.0, 0.14)),
        );
        let top_k = 1.0 - flow;
        if top_k > 0.02 {
            fill_convex(
                &[
                    hg + vec2(-6.0 * top_k, -9.0 * top_k),
                    hg + vec2(6.0 * top_k, -9.0 * top_k),
                    hg,
                ],
                Paint::Solid(th.amber),
            );
        }
        if flow > 0.02 {
            let h = 9.0 * flow;
            let w = 6.0 * (h / 9.0).sqrt();
            fill_convex(
                &[
                    hg + vec2(-6.0, 9.0),
                    hg + vec2(6.0, 9.0),
                    hg + vec2(w, 9.0 - h),
                    hg + vec2(-w, 9.0 - h),
                ],
                Paint::Solid(th.amber),
            );
        }
        draw::hline(
            hg.x - 7.0,
            hg.x + 7.0,
            hg.y - 9.5,
            Color::new(1.0, 1.0, 1.0, 0.4),
        );
        draw::hline(
            hg.x - 7.0,
            hg.x + 7.0,
            hg.y + 9.0,
            Color::new(1.0, 1.0, 1.0, 0.4),
        );
        // add-ons: lemon, sugar ×n
        let mut ax = r.x + 12.0;
        let ay = r.bottom() - 22.0;
        if self.tu.lemon {
            let lc = vec2(ax + 8.0, ay);
            fill_circle(lc, 7.0, hex(0xf7d34a));
            fill_circle(lc, 5.5, hex(0xfbeaa0));
            if self.lemon_drop.is_some() {
                draw_check(lc + vec2(10.0, -5.0), th.emerald_400);
            }
            ax += 30.0;
        }
        if self.tu.honey {
            let hc = vec2(ax + 8.0, ay);
            fill_circle(hc, 7.0, hex(0xe39a2c));
            fill_circle(hc + vec2(-2.0, -2.0), 2.5, hex(0xf6cf78));
            if self.honey.at.is_some() {
                draw_check(hc + vec2(10.0, -5.0), th.emerald_400);
            }
            ax += 30.0;
        }
        if self.tu.milk {
            // the milk bar with its caramel mark
            let bar = Rect::new(ax + 2.0, ay + 4.0, r.right() - ax - 16.0, 6.0);
            ui.fonts.draw(
                &text.get("minigame.milk").to_uppercase(),
                bar.x,
                bar.y - 4.0,
                &micro,
            );
            fill_rrect(bar, 3.0, Color::new(1.0, 1.0, 1.0, 0.12));
            let f = bar.with_w((bar.w * self.milk).max(0.0));
            if f.w > 1.0 {
                fill_rrect(f, 3.0, MILK_C);
            }
            let tol = 0.22 * widen(self.common.relaxed) * 0.35;
            let band = Rect::new(
                bar.x + bar.w * (self.milk_target - tol),
                bar.y - 3.0,
                bar.w * tol * 2.0,
                bar.h + 6.0,
            );
            draw::stroke_rrect(band, 4.0, 1.4, with_alpha(th.amber, 0.9));
        }
        if self.tu.sugar > 0 {
            let label = format!(
                "{} {}/{}",
                text.get("minigame.sugar"),
                self.cubes.len(),
                self.tu.sugar
            );
            let s = th.small_medium();
            ui.fonts.draw_in(
                &label,
                Rect::new(ax, ay - 8.0, r.right() - ax - 8.0, 16.0),
                &s,
                Align::Left,
            );
        }
    }
}

fn draw_check(p: Vec2, c: Color) {
    draw::stroke_round(
        &[
            p + vec2(-3.0, 0.0),
            p + vec2(-0.5, 2.5),
            p + vec2(4.0, -3.0),
        ],
        false,
        1.6,
        c,
    );
}

/// A tiny spice glyph centred at `p` (stage art, no text).
fn draw_spice(sp: Spice, p: Vec2, s: f32) {
    let c = sp.color();
    match sp {
        Spice::Cinnamon => {
            // a rolled stick
            rd::capsule(p + vec2(-9.0, 4.0) * s, p + vec2(9.0, -4.0) * s, 3.6 * s, c);
            rd::capsule(
                p + vec2(-8.0, 2.6) * s,
                p + vec2(8.0, -5.4) * s,
                1.2 * s,
                hex(0xd28a50),
            );
        }
        Spice::Cardamom => {
            for (dx, dy) in [(-4.0, 1.0), (4.0, -1.0)] {
                rd::ellipse(p.x + dx * s, p.y + dy * s, 4.2 * s, 6.5 * s, c);
                fill_circle(p + vec2(dx - 1.0, dy - 2.0) * s, 1.4 * s, hex(0xc6dc96));
            }
        }
        Spice::Ginger => {
            for (dx, dy, r) in [(-3.0, 1.0, 5.5), (4.0, -2.0, 4.5), (1.0, 5.0, 3.5)] {
                fill_circle(p + vec2(dx, dy) * s, r * s, c);
            }
            fill_circle(p + vec2(-4.0, -1.0) * s, 1.6 * s, hex(0xf6dca0));
        }
        Spice::Clove => {
            for k in 0..3 {
                let a = k as f32 * 2.1;
                let d = vec2(a.cos(), a.sin()) * 5.0 * s;
                rd::capsule(p, p + d, 1.3 * s, c);
                fill_circle(p + d, 2.4 * s, hex(0x6b3a22));
            }
        }
    }
}
