//! The little ritual gestures over a piece (`content::handson::RitualGesture`): pull the
//! lamp's cord, hold the machine's needle in the green, count the float into the tray, a few
//! cloth strokes, tick off the shelf, drop the record's needle, sort the till's coins, pick a
//! line for the handover note, slide the bottle / the tray / the bag, stir the pot.
//!
//! Pure logic ([`Gesture::step`] with [`MgInput`] in stage-local coordinates, [`STAGE`]) plus
//! its drawing. Calm by design: nothing can fail, a mistake only means "not perfect", and
//! every kind plays with the mouse or with Space / the arrows alone. The screen glue (when a
//! gesture shows, the commands it sends) lives in `screens::game::rituals_ui`.

use std::f32::consts::{PI, TAU};

use macroquad::prelude::*;

use super::MgInput;
use super::paint::{hex, sparkle, with_alpha};
use crate::content::handson::{RitualGesture, ShelfLook, SlideLook, StrokeLook};
use crate::ui::draw::{arc, fill_circle, fill_rrect, glow_circle, line, stroke_circle, stroke_rrect};
use crate::ui::{Align, Ui, ease_out};

/// The gesture's stage (local px; the card adds its frame around it).
pub const STAGE: Vec2 = Vec2::new(176.0, 92.0);

/// Seconds between scripted autoplay actions.
const AUTO_EVERY: f32 = 0.32;

/// A gesture's result for this step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GestureStatus {
    Running,
    Done { perfect: bool },
}

#[derive(Debug, Clone, PartialEq)]
enum State {
    Cord {
        pull: f32,
        grab: Option<f32>,
        sprang: bool,
    },
    Needle {
        v: f32,
        held: bool,
        settle: Option<f32>,
        released: bool,
    },
    Count {
        target: u32,
        tray: Vec<u32>,
        sel: usize,
        over: f32,
    },
    Strokes {
        done: u32,
        travel: f32,
        last: Option<f32>,
        cloth: Vec2,
        /// Keyboard: the last arrow (−1 / +1) and the stroke animation (0..1).
        dir: i8,
        anim: f32,
    },
    Shelf {
        ticked: u32,
        wobble: Option<(usize, f32)>,
    },
    Drop {
        misses: u32,
        miss_t: f32,
        dropped: Option<f32>,
    },
    Sort {
        coins: Vec<usize>,
        at: usize,
        fly: f32,
        wobble: f32,
    },
    Lines {
        sel: usize,
        chosen: Option<usize>,
        ink: f32,
    },
    Slide {
        pos: f32,
        grab: Option<f32>,
        grabs: u32,
    },
    Stir {
        angle: f32,
        last: Option<f32>,
    },
}

/// One ritual gesture in play.
#[derive(Debug, Clone, PartialEq)]
pub struct Gesture {
    pub kind: RitualGesture,
    /// Accessibility "relaxed mini-games": wider bands, shorter strokes.
    pub relaxed: bool,
    /// Lines on offer (the handover note).
    pub lines: usize,
    /// The stage's width (the handover lines may use a wider card; [`STAGE`] otherwise).
    pub width: f32,
    age: f32,
    started: bool,
    flawless: bool,
    finished: Option<bool>,
    /// Seconds since the gesture finished (the card's little "done" beat).
    pub since_done: f32,
    cool: f32,
    st: State,
}

fn splitmix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A stable seed for a gesture (world seed, night and the key: the same night shows the same
/// float target). Never touches the world's RNG.
pub fn seed_for(world_seed: u64, night: u32, key: &str) -> u64 {
    let mut h = splitmix(world_seed ^ ((night as u64) << 32));
    for b in key.bytes() {
        h = splitmix(h ^ b as u64);
    }
    h
}

// layout (stage px)
const COIN_Y: f32 = 20.0;
const TRAY: Rect = Rect {
    x: 28.0,
    y: 52.0,
    w: 120.0,
    h: 32.0,
};
const SURFACE: Rect = Rect {
    x: 16.0,
    y: 26.0,
    w: 144.0,
    h: 56.0,
};
const RECORD_C: Vec2 = Vec2::new(64.0, 50.0);
const RECORD_R: f32 = 36.0;
const POT_C: Vec2 = Vec2::new(88.0, 52.0);
const POT_R: f32 = 30.0;
const TRACK: (f32, f32, f32) = (26.0, 150.0, 58.0);
const SORT_VALUES: [u32; 3] = [1, 5, 10];

fn coin_x(i: usize, n: usize) -> f32 {
    let gap = (STAGE.x - 24.0) / n.max(1) as f32;
    12.0 + gap * (i as f32 + 0.5)
}
fn shelf_x(i: usize, n: usize) -> f32 {
    let gap = (STAGE.x - 32.0) / n.max(1) as f32;
    16.0 + gap * (i as f32 + 0.5)
}
fn stack_x(i: usize) -> f32 {
    [40.0, 88.0, 136.0][i.min(2)]
}
fn line_rect(i: usize, width: f32) -> Rect {
    Rect::new(4.0, 6.0 + i as f32 * 27.0, width - 8.0, 24.0)
}

impl Gesture {
    pub fn new(kind: &RitualGesture, seed: u64, relaxed: bool, lines: usize) -> Gesture {
        let st = match kind {
            RitualGesture::Cord => State::Cord {
                pull: 0.0,
                grab: None,
                sprang: false,
            },
            RitualGesture::Needle { .. } => State::Needle {
                v: 0.0,
                held: false,
                settle: None,
                released: false,
            },
            RitualGesture::Count { coins, total } => {
                let span = total.1.saturating_sub(total.0) as u64 + 1;
                let mut target = total.0 + (seed % span) as u32;
                // always reachable with the coins on offer (a 1 Kč coin makes any total)
                if !coins.contains(&1) && coins.iter().all(|c| !target.is_multiple_of(*c)) {
                    target = coins.iter().copied().max().unwrap_or(1) * 2;
                }
                State::Count {
                    target,
                    tray: Vec::new(),
                    sel: coins.len().saturating_sub(1),
                    over: 0.0,
                }
            }
            RitualGesture::Strokes { .. } => State::Strokes {
                done: 0,
                travel: 0.0,
                last: None,
                cloth: vec2(SURFACE.x + 18.0, SURFACE.center().y),
                dir: 1,
                anim: 1.0,
            },
            RitualGesture::Shelf { .. } => State::Shelf {
                ticked: 0,
                wobble: None,
            },
            RitualGesture::Drop { .. } => State::Drop {
                misses: 0,
                miss_t: 0.0,
                dropped: None,
            },
            RitualGesture::Sort { coins } => {
                let mut s = seed;
                let mut v = Vec::new();
                for i in 0..*coins {
                    s = splitmix(s);
                    let mut c = (s % 3) as usize;
                    // never three of a kind in a row (sorting, not tapping one stack)
                    if i >= 2 && v[i as usize - 1] == c && v[i as usize - 2] == c {
                        c = (c + 1) % 3;
                    }
                    v.push(c);
                }
                State::Sort {
                    coins: v,
                    at: 0,
                    fly: 1.0,
                    wobble: 0.0,
                }
            }
            RitualGesture::Lines { .. } => State::Lines {
                sel: 0,
                chosen: None,
                ink: 0.0,
            },
            RitualGesture::Slide { .. } => State::Slide {
                pos: 0.0,
                grab: None,
                grabs: 0,
            },
            RitualGesture::Stir => State::Stir {
                angle: 0.0,
                last: None,
            },
        };
        let lines = match kind {
            RitualGesture::Lines { count } => lines.clamp(1, *count as usize),
            _ => 0,
        };
        Gesture {
            kind: kind.clone(),
            relaxed,
            lines,
            width: STAGE.x,
            age: 0.0,
            started: false,
            flawless: true,
            finished: None,
            since_done: 0.0,
            cool: 0.25,
            st,
        }
    }

    /// The player has touched the gesture (Karolina then waits for it at the piece).
    pub fn started(&self) -> bool {
        self.started
    }
    pub fn finished(&self) -> Option<bool> {
        self.finished
    }
    /// The line chosen for the handover note.
    pub fn chosen_line(&self) -> Option<usize> {
        match &self.st {
            State::Lines { chosen, .. } => *chosen,
            _ => None,
        }
    }
    /// The float's target and what lies in the tray (Kč).
    pub fn count_totals(&self) -> Option<(u32, u32)> {
        match &self.st {
            State::Count { target, tray, .. } => Some((tray.iter().sum(), *target)),
            _ => None,
        }
    }
    /// The tray holds more than the float (a coin must go back).
    pub fn count_over(&self) -> bool {
        self.count_totals().is_some_and(|(n, t)| n > t)
    }
    /// The swinging arm just missed the groove (a calm "not yet").
    pub fn drop_missed(&self) -> bool {
        matches!(self.st, State::Drop { miss_t, .. } if miss_t > 0.0)
    }

    fn green(&self) -> (f32, f32) {
        let (a, b) = match self.kind {
            RitualGesture::Needle { green } => green,
            _ => (0.6, 0.8),
        };
        if self.relaxed { (a - 0.06, b + 0.06) } else { (a, b) }
    }

    /// The arm of the record player: 0 = outside the record, 1 = on the label.
    fn arm(&self) -> f32 {
        0.5 - 0.5 * (self.age * 1.45).cos()
    }
    fn band(&self) -> (f32, f32) {
        let (misses, base) = match (&self.st, &self.kind) {
            (State::Drop { misses, .. }, RitualGesture::Drop { band }) => (*misses, *band),
            _ => (0, 0.16),
        };
        let w = base * if self.relaxed { 1.5 } else { 1.0 } * if misses >= 3 { 1.6 } else { 1.0 };
        let c = 0.3;
        (c - w / 2.0, c + w / 2.0)
    }

    fn strokes_needed(&self) -> u32 {
        match self.kind {
            RitualGesture::Strokes { count, .. } => count.max(1),
            _ => 1,
        }
    }
    fn shelf_items(&self) -> usize {
        match self.kind {
            RitualGesture::Shelf { items, .. } => items.max(1) as usize,
            _ => 3,
        }
    }
    fn coins(&self) -> &[u32] {
        match &self.kind {
            RitualGesture::Count { coins, .. } => coins,
            _ => &[],
        }
    }

    fn finish(&mut self, perfect: bool) {
        if self.finished.is_none() {
            self.finished = Some(perfect && self.flawless);
        }
    }

    /// One step with explicit input (stage-local pointer).
    pub fn step(&mut self, input: &MgInput, dt: f32) -> GestureStatus {
        let dt = dt.clamp(0.0, 0.1);
        if let Some(p) = self.finished {
            self.since_done += dt;
            return GestureStatus::Done { perfect: p };
        }
        self.age += dt;
        self.cool -= dt;
        let width = self.width.max(STAGE.x);
        let inside =
            |p: Option<Vec2>| p.is_some_and(|p| Rect::new(0.0, 0.0, width, STAGE.y).contains(p));
        let mut touched = false;
        let mut done: Option<bool> = None;
        let relaxed = self.relaxed;
        let green = self.green();
        let band = self.band();
        let arm = self.arm();
        let needed = self.strokes_needed();
        let shelf_n = self.shelf_items();
        let coins: Vec<u32> = self.coins().to_vec();
        let lines = self.lines;
        match &mut self.st {
            State::Cord { pull, grab, sprang } => {
                if input.pressed && inside(input.pointer) {
                    *grab = input.pointer.map(|p| p.y - *pull * 30.0);
                    touched = true;
                }
                let key = input.hold || input.axis.y > 0.0;
                if let (Some(g), true, Some(p)) = (*grab, input.down, input.pointer) {
                    *pull = ((p.y - g) / 30.0).clamp(0.0, 1.0);
                } else if key {
                    touched = true;
                    *pull = (*pull + dt * 1.7).min(1.0);
                } else {
                    if grab.is_some() && *pull < 0.85 && *pull > 0.05 {
                        *sprang = true;
                    }
                    *grab = None;
                    if *pull >= 0.85 {
                        done = Some(!*sprang);
                    } else {
                        *pull = (*pull - dt * 3.0).max(0.0);
                    }
                }
                if *pull >= 0.98 {
                    done = Some(!*sprang);
                }
            }
            State::Needle {
                v,
                held,
                settle,
                released,
            } => {
                if let Some(s) = settle {
                    *s += dt;
                    if *s >= 0.45 {
                        done = Some(!*released);
                    }
                } else {
                    let now = (input.down && (*held || (input.pressed && inside(input.pointer))))
                        || input.hold;
                    if now {
                        touched = true;
                        // the pressure builds slower near the top
                        *v = (*v + dt * (0.5 - 0.25 * *v)).min(1.0);
                    } else {
                        if *held {
                            if *v >= green.0 && *v <= green.1 {
                                *settle = Some(0.0);
                            } else {
                                *released = true;
                            }
                        }
                        if settle.is_none() {
                            *v = (*v - dt * 0.35).max(0.0);
                        }
                    }
                    *held = now;
                }
            }
            State::Count {
                target,
                tray,
                sel,
                over,
            } => {
                *over = (*over - dt).max(0.0);
                let mut add: Option<u32> = None;
                let mut back = false;
                if input.pressed
                    && let Some(p) = input.pointer
                {
                    for (i, c) in coins.iter().enumerate() {
                        if p.distance(vec2(coin_x(i, coins.len()), COIN_Y)) < 15.0 {
                            add = Some(*c);
                            *sel = i;
                        }
                    }
                    if TRAY.contains(p) && !tray.is_empty() {
                        back = true;
                    }
                }
                if input.left {
                    *sel = sel.saturating_sub(1);
                    touched = true;
                }
                if input.right {
                    *sel = (*sel + 1).min(coins.len().saturating_sub(1));
                    touched = true;
                }
                if (input.hold_pressed || input.up) && !coins.is_empty() {
                    add = Some(coins[(*sel).min(coins.len() - 1)]);
                }
                if input.down_tap && !tray.is_empty() {
                    back = true;
                }
                if let Some(c) = add {
                    touched = true;
                    tray.push(c);
                    let sum: u32 = tray.iter().sum();
                    if sum > *target {
                        *over = 1.2;
                        self.flawless = false;
                    }
                } else if back {
                    touched = true;
                    tray.pop();
                }
                if tray.iter().sum::<u32>() == *target {
                    done = Some(true);
                }
            }
            State::Strokes {
                done: n,
                travel,
                last,
                cloth,
                dir,
                anim,
            } => {
                let span = SURFACE.w * if relaxed { 0.45 } else { 0.6 };
                if input.pressed && inside(input.pointer) {
                    *last = input.pointer.map(|p| p.x);
                    touched = true;
                }
                if input.down
                    && let (Some(lx), Some(p)) = (*last, input.pointer)
                {
                    let c = vec2(
                        p.x.clamp(SURFACE.x + 8.0, SURFACE.right() - 8.0),
                        p.y.clamp(SURFACE.y + 8.0, SURFACE.bottom() - 8.0),
                    );
                    *travel += (c.x - lx).abs();
                    *last = Some(c.x);
                    if (p.y - c.y).abs() > 14.0 {
                        // wandering off the table: still fine, just not perfect
                        self.flawless = false;
                    }
                    *cloth = c;
                    if *travel >= span {
                        *travel = 0.0;
                        *n += 1;
                    }
                } else if !input.down {
                    *last = None;
                }
                // keyboard: ← → in turn (or Space) sweep the cloth across
                let key_dir = if input.left {
                    -1
                } else if input.right {
                    1
                } else if input.hold_pressed {
                    -*dir
                } else {
                    0
                };
                if key_dir != 0 && *anim >= 0.6 {
                    touched = true;
                    if key_dir != *dir {
                        *dir = key_dir;
                        *anim = 0.0;
                        *n += 1;
                    }
                }
                if *anim < 1.0 {
                    *anim = (*anim + dt * 3.5).min(1.0);
                    let (a, b) = (SURFACE.x + 18.0, SURFACE.right() - 18.0);
                    let k = ease_out(*anim);
                    let x = if *dir > 0 { a + (b - a) * k } else { b - (b - a) * k };
                    *cloth = vec2(x, SURFACE.center().y + (k * PI).sin() * 6.0);
                }
                if *n >= needed && (*anim >= 1.0 || input.down) {
                    done = Some(true);
                }
            }
            State::Shelf { ticked, wobble } => {
                if let Some((_, t)) = wobble {
                    *t -= dt;
                }
                if wobble.is_some_and(|(_, t)| t <= 0.0) {
                    *wobble = None;
                }
                let mut hit: Option<usize> = None;
                if input.pressed
                    && let Some(p) = input.pointer
                {
                    for i in 0..shelf_n {
                        if Rect::new(shelf_x(i, shelf_n) - 20.0, 14.0, 40.0, 60.0).contains(p) {
                            hit = Some(i);
                        }
                    }
                }
                if input.hold_pressed || input.right {
                    hit = Some(*ticked as usize);
                }
                if let Some(i) = hit {
                    touched = true;
                    if i == *ticked as usize {
                        *ticked += 1;
                    } else if i > *ticked as usize {
                        *wobble = Some((i, 0.35));
                        self.flawless = false;
                    }
                }
                if *ticked as usize >= shelf_n {
                    done = Some(true);
                }
            }
            State::Drop {
                misses,
                miss_t,
                dropped,
            } => {
                *miss_t = (*miss_t - dt).max(0.0);
                if let Some(t) = dropped {
                    *t += dt;
                    if *t >= 0.4 {
                        done = Some(*misses == 0);
                    }
                } else if (input.pressed && inside(input.pointer)) || input.hold_pressed {
                    touched = true;
                    if arm >= band.0 && arm <= band.1 {
                        *dropped = Some(0.0);
                    } else if *miss_t <= 0.0 {
                        *misses += 1;
                        *miss_t = 0.6;
                    }
                }
            }
            State::Sort {
                coins: cs,
                at,
                fly,
                wobble,
            } => {
                *fly = (*fly + dt * 4.0).min(1.0);
                *wobble = (*wobble - dt).max(0.0);
                let mut pick: Option<usize> = None;
                if input.pressed
                    && let Some(p) = input.pointer
                {
                    for i in 0..3 {
                        if Rect::new(stack_x(i) - 22.0, 46.0, 44.0, 44.0).contains(p) {
                            pick = Some(i);
                        }
                    }
                }
                if input.left {
                    pick = Some(0);
                }
                if input.down_tap {
                    pick = Some(1);
                }
                if input.right {
                    pick = Some(2);
                }
                if let Some(j) = pick
                    && *at < cs.len()
                    && *fly >= 0.6
                {
                    touched = true;
                    if cs[*at] == j {
                        *at += 1;
                        *fly = 0.0;
                    } else {
                        *wobble = 0.35;
                        self.flawless = false;
                    }
                }
                if *at >= cs.len() && *fly >= 1.0 {
                    done = Some(true);
                }
            }
            State::Lines { sel, chosen, ink } => {
                if let Some(_c) = chosen {
                    *ink += dt;
                    if *ink >= 0.6 {
                        done = Some(true);
                    }
                } else {
                    if let Some(p) = input.pointer {
                        for i in 0..lines {
                            if line_rect(i, width).contains(p) {
                                *sel = i;
                                if input.pressed {
                                    *chosen = Some(i);
                                    touched = true;
                                }
                            }
                        }
                    }
                    if input.up {
                        *sel = sel.saturating_sub(1);
                        touched = true;
                    }
                    if input.down_tap {
                        *sel = (*sel + 1).min(lines.saturating_sub(1));
                        touched = true;
                    }
                    if input.hold_pressed || input.confirm {
                        *chosen = Some(*sel);
                        touched = true;
                    }
                }
            }
            State::Slide { pos, grab, grabs } => {
                let (a, b, _) = TRACK;
                if input.pressed && inside(input.pointer) {
                    *grab = input.pointer.map(|p| p.x - (a + (b - a) * *pos));
                    *grabs += 1;
                    touched = true;
                }
                if let (Some(g), true, Some(p)) = (*grab, input.down, input.pointer) {
                    *pos = ((p.x - g - a) / (b - a)).clamp(*pos - 0.02, 1.0).max(0.0);
                } else {
                    *grab = None;
                    if input.hold || input.axis.x > 0.0 {
                        touched = true;
                        if *grabs == 0 {
                            *grabs = 1;
                        }
                        *pos = (*pos + dt * 0.75).min(1.0);
                    }
                }
                if *pos >= 0.96 {
                    done = Some(*grabs <= 1);
                }
            }
            State::Stir { angle, last } => {
                if input.down
                    && let Some(p) = input.pointer
                    && (input.pressed || last.is_some())
                {
                    let d = p - POT_C;
                    if d.length() > 6.0 {
                        let a = d.y.atan2(d.x);
                        if let Some(l) = *last {
                            let mut da = a - l;
                            while da > PI {
                                da -= TAU;
                            }
                            while da < -PI {
                                da += TAU;
                            }
                            *angle += da;
                        }
                        *last = Some(a);
                        touched = true;
                    }
                } else {
                    *last = None;
                    if input.hold || input.axis.x > 0.0 {
                        touched = true;
                        *angle += dt * 3.4;
                    }
                }
                if angle.abs() >= TAU * if relaxed { 0.85 } else { 1.0 } {
                    done = Some(true);
                }
            }
        }
        if touched {
            self.started = true;
            self.cool = AUTO_EVERY;
        }
        if let Some(p) = done {
            self.started = true;
            self.finish(p);
            return GestureStatus::Done {
                perfect: self.finished.unwrap_or(p),
            };
        }
        GestureStatus::Running
    }

    /// Scripted mouse input that plays the gesture well (tests, the demo).
    pub fn autoplay_input(&self) -> MgInput {
        let mut i = MgInput::default();
        let ready = self.cool <= 0.0;
        match &self.st {
            State::Cord { pull, grab, .. } => {
                let knob = vec2(STAGE.x * 0.5, 34.0 + pull * 30.0);
                i.pointer = Some(knob + vec2(0.0, if grab.is_some() { 6.0 } else { 0.0 }));
                i.pressed = grab.is_none() && ready;
                i.down = grab.is_some() || i.pressed;
            }
            State::Needle { v, held, .. } => {
                let g = self.green();
                let want = (g.0 + g.1) / 2.0;
                i.pointer = Some(STAGE / 2.0);
                let hold = *v < want;
                i.down = hold && (*held || ready);
                i.pressed = hold && !*held && ready;
            }
            State::Count { target, tray, .. } => {
                let sum: u32 = tray.iter().sum();
                let coins = self.coins();
                if ready {
                    if sum > *target {
                        i.pointer = Some(TRAY.center());
                        i.pressed = true;
                    } else if let Some((k, _)) = coins
                        .iter()
                        .enumerate()
                        .filter(|(_, c)| **c <= target - sum)
                        .max_by_key(|(_, c)| **c)
                    {
                        i.pointer = Some(vec2(coin_x(k, coins.len()), COIN_Y));
                        i.pressed = true;
                    }
                }
            }
            State::Strokes { last, cloth, .. } => {
                // sweep left-right across the table
                let k = (self.age * 2.2).sin() * 0.5 + 0.5;
                let x = SURFACE.x + 10.0 + (SURFACE.w - 20.0) * k;
                i.pointer = Some(vec2(x, SURFACE.center().y));
                i.down = last.is_some() || ready;
                i.pressed = last.is_none() && ready;
                let _ = cloth;
            }
            State::Shelf { ticked, .. } => {
                let n = self.shelf_items();
                i.pointer = Some(vec2(shelf_x(*ticked as usize, n), 44.0));
                i.pressed = ready;
            }
            State::Drop { dropped, .. } => {
                let (a, b) = self.band();
                let arm = self.arm();
                let mid = (a + b) / 2.0;
                i.pointer = Some(RECORD_C);
                i.pressed = dropped.is_none() && (arm - mid).abs() < (b - a) * 0.3;
            }
            State::Sort { coins, at, fly, .. } => {
                if let Some(c) = coins.get(*at)
                    && *fly >= 0.6
                    && ready
                {
                    i.pointer = Some(vec2(stack_x(*c), 68.0));
                    i.pressed = true;
                }
            }
            State::Lines { chosen, .. } => {
                if chosen.is_none() && ready && self.age > 0.6 {
                    i.pointer = Some(line_rect(0, self.width).center());
                    i.pressed = true;
                }
            }
            State::Slide { pos, grab, .. } => {
                let (a, b, y) = TRACK;
                let at = a + (b - a) * pos;
                i.pointer = Some(vec2(at + if grab.is_some() { 6.0 } else { 0.0 }, y));
                i.pressed = grab.is_none() && ready;
                i.down = grab.is_some() || i.pressed;
            }
            State::Stir { last, .. } => {
                let a = self.age * 4.0;
                i.pointer = Some(POT_C + vec2(a.cos(), a.sin()) * 20.0);
                i.pressed = last.is_none() && ready;
                i.down = last.is_some() || i.pressed;
            }
        }
        i
    }

    /// Scripted keyboard-only input (Space / the arrows), for the keyboard tests.
    pub fn autoplay_keyboard_input(&self) -> MgInput {
        let mut i = MgInput::default();
        let ready = self.cool <= 0.0;
        match &self.st {
            State::Cord { .. } | State::Slide { .. } | State::Stir { .. } => i.hold = true,
            State::Needle { v, .. } => {
                let g = self.green();
                i.hold = *v < (g.0 + g.1) / 2.0;
            }
            State::Count {
                target, tray, sel, ..
            } => {
                let sum: u32 = tray.iter().sum();
                let coins = self.coins();
                if ready {
                    let want = coins
                        .iter()
                        .enumerate()
                        .filter(|(_, c)| **c <= target.saturating_sub(sum))
                        .max_by_key(|(_, c)| **c)
                        .map(|(k, _)| k);
                    match want {
                        _ if sum > *target => i.down_tap = true,
                        Some(k) if k < *sel => i.left = true,
                        Some(k) if k > *sel => i.right = true,
                        Some(_) => i.hold_pressed = true,
                        None => {}
                    }
                }
            }
            State::Strokes { dir, .. } => {
                if ready {
                    if *dir > 0 {
                        i.left = true;
                    } else {
                        i.right = true;
                    }
                }
            }
            State::Shelf { .. } | State::Lines { .. } => i.hold_pressed = ready && self.age > 0.4,
            State::Drop { dropped, .. } => {
                let (a, b) = self.band();
                let arm = self.arm();
                i.hold_pressed = dropped.is_none() && (arm - (a + b) / 2.0).abs() < (b - a) * 0.3;
            }
            State::Sort { coins, at, fly, .. } => {
                if let Some(c) = coins.get(*at)
                    && *fly >= 0.6
                    && ready
                {
                    match c {
                        0 => i.left = true,
                        1 => i.down_tap = true,
                        _ => i.right = true,
                    }
                }
            }
        }
        i
    }

    /// 0..1 how far along it is (the card's thin progress line).
    pub fn progress(&self) -> f32 {
        if self.finished.is_some() {
            return 1.0;
        }
        match &self.st {
            State::Cord { pull, .. } => *pull,
            State::Needle { v, .. } => (*v / self.green().0).min(1.0) * 0.9,
            State::Count { target, tray, .. } => {
                (tray.iter().sum::<u32>() as f32 / (*target).max(1) as f32).min(1.0)
            }
            State::Strokes { done, .. } => *done as f32 / self.strokes_needed() as f32,
            State::Shelf { ticked, .. } => *ticked as f32 / self.shelf_items() as f32,
            State::Drop { dropped, .. } => dropped.map_or(0.0, |t| (t / 0.4).min(1.0)),
            State::Sort { coins, at, .. } => *at as f32 / coins.len().max(1) as f32,
            State::Lines { ink, .. } => (*ink / 0.6).min(1.0),
            State::Slide { pos, .. } => *pos,
            State::Stir { angle, .. } => (angle.abs() / TAU).min(1.0),
        }
    }

    // ───────────────────────── drawing ─────────────────────────

    /// Draws the stage at `o` (UI px, top-left). `lines` = the handover lines on offer.
    pub fn draw(&self, ui: &mut Ui, o: Vec2, lines: &[String], time: f32) {
        let th = ui.theme.clone();
        let stage = Rect::new(o.x, o.y, self.width.max(STAGE.x), STAGE.y);
        fill_rrect(stage, 8.0, Color::new(0.05, 0.04, 0.12, 0.55));
        let at = |p: Vec2| o + p;
        let done = self.finished.is_some();
        match &self.st {
            State::Cord { pull, .. } => {
                let lit = self.finished.is_some() || *pull > 0.97;
                // a small lamp shade, the cord, the knob
                let shade = at(vec2(STAGE.x * 0.5, 14.0));
                if lit {
                    glow_circle(shade + vec2(0.0, 10.0), 46.0, with_alpha(hex(0xFFD27A), 0.35));
                }
                fill_rrect(
                    Rect::new(shade.x - 22.0, shade.y - 8.0, 44.0, 12.0),
                    5.0,
                    if lit { hex(0xF4C46A) } else { hex(0x6A5A7E) },
                );
                let knob = at(vec2(STAGE.x * 0.5, 34.0 + pull * 30.0));
                line(shade + vec2(0.0, 4.0), knob, 1.4, hex(0xD8CFC0));
                fill_circle(knob, 5.5, hex(0xC89B5A));
                stroke_circle(knob, 5.5, 1.0, with_alpha(WHITE, 0.4));
                // the pull mark
                let end = at(vec2(STAGE.x * 0.5, 34.0 + 30.0));
                if !lit {
                    for k in 0..3 {
                        let y = end.y + 6.0 + k as f32 * 5.0 - ((time * 2.0) % 1.0) * 5.0;
                        let a = 0.25 + 0.2 * (1.0 - k as f32 / 3.0);
                        line(
                            vec2(end.x - 4.0, y - 3.0),
                            vec2(end.x, y),
                            1.2,
                            with_alpha(th.indigo_300, a),
                        );
                        line(
                            vec2(end.x + 4.0, y - 3.0),
                            vec2(end.x, y),
                            1.2,
                            with_alpha(th.indigo_300, a),
                        );
                    }
                }
            }
            State::Needle { v, settle, .. } => {
                let c = at(vec2(STAGE.x * 0.5, 76.0));
                let r = 50.0;
                let g = self.green();
                let ang = |x: f32| PI + x * PI;
                arc(c, r, PI, TAU, 6.0, Color::new(1.0, 1.0, 1.0, 0.12));
                arc(c, r, ang(g.0), ang(g.1), 6.0, with_alpha(th.emerald_400, 0.9));
                arc(c, r, ang(g.1), TAU, 6.0, with_alpha(th.amber, 0.45));
                for k in 0..=10 {
                    let a = ang(k as f32 / 10.0);
                    let d = vec2(a.cos(), a.sin());
                    line(c + d * (r - 10.0), c + d * (r - 5.0), 1.0, with_alpha(WHITE, 0.35));
                }
                let a = ang(*v);
                let tip = c + vec2(a.cos(), a.sin()) * (r - 6.0);
                let ok = *v >= g.0 && *v <= g.1;
                line(c, tip, 2.4, if ok { th.emerald_400 } else { hex(0xF2E6D8) });
                fill_circle(c, 5.0, hex(0x8C8C9C));
                if settle.is_some() || done {
                    sparkle(tip, 7.0, self.since_done + settle.unwrap_or(0.0), true);
                }
            }
            State::Count {
                target,
                tray,
                sel,
                over,
            } => {
                let coins = self.coins();
                for (i, c) in coins.iter().enumerate() {
                    let p = at(vec2(coin_x(i, coins.len()), COIN_Y));
                    draw_coin(ui, p, *c, i == *sel);
                }
                let tr = Rect::new(o.x + TRAY.x, o.y + TRAY.y, TRAY.w, TRAY.h);
                fill_rrect(tr, 6.0, hex(0x3A2E4A));
                stroke_rrect(
                    tr,
                    6.0,
                    1.0,
                    if *over > 0.0 {
                        with_alpha(th.amber, 0.9)
                    } else {
                        with_alpha(WHITE, 0.18)
                    },
                );
                // notes first, then the coins, side by side (a second row when full)
                let mut sorted = tray.clone();
                sorted.sort_by(|a, b| b.cmp(a));
                let (mut x, mut y) = (tr.x + 11.0, tr.y + 10.0);
                for c in sorted {
                    let wide = if c >= 20 { 17.0 } else { 11.0 };
                    if x + wide / 2.0 > tr.right() - 4.0 {
                        x = tr.x + 11.0;
                        y += 9.0;
                    }
                    draw_coin_small(vec2(x + (wide - 11.0) / 2.0, y), c);
                    x += wide;
                }
                let sum: u32 = tray.iter().sum();
                let txt = format!("{sum} / {target} Kč");
                let style = th.small_medium().color(if sum == *target {
                    th.emerald_400
                } else if sum > *target {
                    th.amber
                } else {
                    th.text
                });
                ui.fonts.draw_in(
                    &txt,
                    Rect::new(tr.x, tr.bottom() - 15.0, tr.w - 6.0, 14.0),
                    &style,
                    Align::Right,
                );
            }
            State::Strokes { done: n, cloth, .. } => {
                let look = match self.kind {
                    RitualGesture::Strokes { look, .. } => look,
                    _ => StrokeLook::Cloth,
                };
                let s = Rect::new(o.x + SURFACE.x, o.y + SURFACE.y, SURFACE.w, SURFACE.h);
                let need = self.strokes_needed() as f32;
                let left = (1.0 - *n as f32 / need).clamp(0.0, 1.0);
                match look {
                    StrokeLook::Cloth => {
                        fill_rrect(s, 8.0, hex(0x8A5A3C));
                        stroke_rrect(s, 8.0, 1.0, hex(0xA87452));
                        // crumbs and a cup ring fade with each stroke
                        stroke_circle(
                            s.center() + vec2(26.0, -4.0),
                            9.0,
                            2.0,
                            with_alpha(hex(0x5A3A26), 0.7 * left),
                        );
                        for k in 0..9 {
                            let p = vec2(
                                s.x + s.w * (0.15 + 0.08 * k as f32),
                                s.y + s.h * (0.3 + 0.4 * ((k * 7 % 5) as f32 / 5.0)),
                            );
                            fill_circle(p, 1.6, with_alpha(hex(0xE6D2B0), 0.9 * left));
                        }
                    }
                    StrokeLook::Chalk => {
                        fill_rrect(s, 6.0, hex(0x24302A));
                        stroke_rrect(s, 6.0, 2.0, hex(0x7A5A3A));
                        // lines of writing appear, one per stroke
                        let k = 1.0 - left;
                        for row in 0..3 {
                            let w = (s.w - 28.0) * (k * 3.0 - row as f32).clamp(0.0, 1.0);
                            if w > 0.0 {
                                let y = s.y + 14.0 + row as f32 * 13.0;
                                for seg in 0..((w / 9.0) as i32) {
                                    let x = s.x + 14.0 + seg as f32 * 9.0;
                                    let dy = ((seg * 37 + row * 11) % 5) as f32 * 0.6;
                                    line(
                                        vec2(x, y + dy),
                                        vec2(x + 6.0, y + 2.0 - dy),
                                        1.4,
                                        with_alpha(hex(0xECEFE6), 0.85),
                                    );
                                }
                            }
                        }
                    }
                    StrokeLook::Sponge => {
                        let c = s.center();
                        fill_circle(c, 25.0, hex(0xEDE8F2));
                        stroke_circle(c, 18.0, 1.0, hex(0xC8C0D4));
                        for k in 0..7 {
                            let a = k as f32 * 0.9;
                            fill_circle(
                                c + vec2(a.cos(), a.sin()) * (8.0 + (k % 3) as f32 * 5.0),
                                2.4,
                                with_alpha(hex(0xA88A5A), 0.8 * left),
                            );
                        }
                    }
                    StrokeLook::Brush => {
                        let c = s.center();
                        fill_circle(c, 24.0, hex(0x9AA0AE));
                        stroke_circle(c, 24.0, 2.0, hex(0xC9CED8));
                        fill_circle(c, 15.0, hex(0x5E6472));
                        for k in 0..8 {
                            let a = k as f32 * 0.8;
                            fill_circle(
                                c + vec2(a.cos(), a.sin()) * (6.0 + (k % 3) as f32 * 3.0),
                                1.8,
                                with_alpha(hex(0x3A2418), 0.9 * left),
                            );
                        }
                    }
                }
                // the cloth / sponge / brush in her hand
                let p = o + *cloth;
                match look {
                    StrokeLook::Chalk => {
                        fill_rrect(Rect::new(p.x - 2.0, p.y - 9.0, 5.0, 14.0), 2.0, hex(0xF4F4EE));
                    }
                    StrokeLook::Brush => {
                        fill_rrect(Rect::new(p.x - 10.0, p.y - 4.0, 20.0, 8.0), 3.0, hex(0xC08A4A));
                        fill_rrect(Rect::new(p.x - 3.0, p.y - 14.0, 6.0, 11.0), 2.0, hex(0x7A5230));
                    }
                    _ => {
                        let col = if look == StrokeLook::Sponge { hex(0xF2D04A) } else { hex(0xD8E4F4) };
                        fill_rrect(Rect::new(p.x - 13.0, p.y - 8.0, 26.0, 16.0), 5.0, col);
                        stroke_rrect(
                            Rect::new(p.x - 13.0, p.y - 8.0, 26.0, 16.0),
                            5.0,
                            1.0,
                            with_alpha(WHITE, 0.5),
                        );
                    }
                }
                stroke_dots(ui, o, *n, self.strokes_needed());
            }
            State::Shelf { ticked, wobble } => {
                let look = match self.kind {
                    RitualGesture::Shelf { look, .. } => look,
                    _ => ShelfLook::Pantry,
                };
                let n = self.shelf_items();
                let board = Rect::new(o.x + 8.0, o.y + 70.0, STAGE.x - 16.0, 5.0);
                fill_rrect(board, 2.0, hex(0x8A6A4A));
                for i in 0..n {
                    let w = wobble.filter(|(j, _)| *j == i).map_or(0.0, |(_, t)| (t * 40.0).sin() * 2.5);
                    let base = at(vec2(shelf_x(i, n) + w, 70.0));
                    let glow = i == *ticked as usize && !done;
                    if glow {
                        glow_circle(base - vec2(0.0, 22.0), 26.0, with_alpha(th.indigo_300, 0.22 + 0.1 * (time * 3.0).sin()));
                    }
                    draw_shelf_item(base, look, i);
                    let bc = base - vec2(0.0, 52.0);
                    let ticked_it = i < *ticked as usize;
                    fill_rrect(
                        Rect::new(bc.x - 7.0, bc.y - 7.0, 14.0, 14.0),
                        4.0,
                        if ticked_it { th.indigo } else { Color::new(0.11, 0.1, 0.24, 0.9) },
                    );
                    stroke_rrect(Rect::new(bc.x - 7.0, bc.y - 7.0, 14.0, 14.0), 4.0, 1.0, with_alpha(th.indigo_300, 0.9));
                    if ticked_it {
                        crate::ui::draw_icon(crate::ui::Icon::Check, bc, 10.0, WHITE);
                    }
                }
            }
            State::Drop { dropped, miss_t, .. } => {
                let c = at(RECORD_C);
                fill_circle(c, RECORD_R + 3.0, hex(0x2A2234));
                fill_circle(c, RECORD_R, hex(0x15121C));
                let spin = time * if dropped.is_some() || done { 3.5 } else { 1.2 };
                for k in 1..5 {
                    stroke_circle(c, RECORD_R - k as f32 * 6.0, 0.8, with_alpha(WHITE, 0.08));
                }
                fill_circle(c, 10.0, hex(0xC85A4A));
                let d = vec2(spin.cos(), spin.sin());
                line(c + d * 12.0, c + d * (RECORD_R - 2.0), 1.0, with_alpha(WHITE, 0.12));
                // the lead-in groove (where the needle should land)
                let (a, b) = self.band();
                let rr = |x: f32| RECORD_R - x * (RECORD_R - 12.0);
                arc(c, (rr(a) + rr(b)) / 2.0, -PI * 0.15, PI * 0.45, (rr(a) - rr(b)).abs().max(2.0), with_alpha(th.emerald_400, 0.35));
                // the arm swings in from the right
                let pivot = at(vec2(158.0, 18.0));
                let arm = if let Some(_t) = dropped { self.band_mid() } else { self.arm() };
                let target = c + vec2(rr(arm) * (0.25f32).cos(), rr(arm) * (0.25f32).sin());
                let tip = target + vec2(0.0, if dropped.is_some() || done { 0.0 } else { -6.0 });
                line(pivot, tip, 3.0, hex(0xC9CED8));
                fill_circle(pivot, 6.0, hex(0x8C92A0));
                fill_rrect(Rect::new(tip.x - 4.0, tip.y - 3.0, 8.0, 6.0), 2.0, hex(0xE8ECF2));
                if *miss_t > 0.0 {
                    stroke_circle(tip, 8.0 + (0.6 - miss_t) * 10.0, 1.0, with_alpha(th.amber, *miss_t));
                }
                if dropped.is_some() || done {
                    sparkle(tip, 7.0, self.since_done + dropped.unwrap_or(0.0), true);
                }
            }
            State::Sort {
                coins,
                at: k,
                fly,
                wobble,
            } => {
                for (i, v) in SORT_VALUES.iter().enumerate() {
                    let base = at(vec2(stack_x(i), 82.0));
                    fill_rrect(Rect::new(base.x - 18.0, base.y - 4.0, 36.0, 8.0), 3.0, hex(0x3A2E4A));
                    let n = coins.iter().take(*k).filter(|c| **c == i).count();
                    for h in 0..n {
                        draw_coin_small(base - vec2(0.0, 5.0 + h as f32 * 4.0), *v);
                    }
                    ui.fonts.draw_in(
                        &format!("{v}"),
                        Rect::new(base.x - 18.0, base.y + 1.0, 36.0, 10.0),
                        &th.micro().color(th.text_dim),
                        Align::Center,
                    );
                }
                if let Some(c) = coins.get(*k) {
                    let w = (wobble * 40.0).sin() * 3.0 * (*wobble / 0.35);
                    draw_coin(ui, at(vec2(STAGE.x / 2.0 + w, 24.0)), SORT_VALUES[*c], true);
                }
                if *k > 0 && *fly < 1.0 {
                    let c = coins[*k - 1];
                    let from = at(vec2(STAGE.x / 2.0, 24.0));
                    let to = at(vec2(stack_x(c), 70.0));
                    draw_coin_small(from.lerp(to, ease_out(*fly)), SORT_VALUES[c]);
                }
            }
            State::Lines { sel, chosen, ink } => {
                for i in 0..self.lines.min(lines.len().max(1)) {
                    let r = line_rect(i, self.width.max(STAGE.x));
                    let rr = Rect::new(o.x + r.x, o.y + r.y, r.w, r.h);
                    let on = Some(i) == *chosen || (chosen.is_none() && i == *sel);
                    fill_rrect(
                        rr,
                        6.0,
                        if on {
                            with_alpha(th.indigo, 0.3)
                        } else {
                            Color::new(1.0, 1.0, 1.0, 0.05)
                        },
                    );
                    if on {
                        stroke_rrect(rr, 6.0, 1.0, th.indigo_300);
                    }
                    let text = lines.get(i).map(String::as_str).unwrap_or("…");
                    let style = th.small().color(th.text);
                    let tw = ui.fonts.draw_in(
                        text,
                        Rect::new(rr.x + 8.0, rr.y, rr.w - 16.0, rr.h),
                        &style,
                        Align::Left,
                    );
                    if Some(i) == *chosen {
                        let k = (*ink / 0.6).clamp(0.0, 1.0);
                        line(
                            vec2(rr.x + 8.0, rr.bottom() - 5.0),
                            vec2(rr.x + 8.0 + tw.min(rr.w - 16.0) * k, rr.bottom() - 5.0),
                            1.4,
                            th.indigo_300,
                        );
                    }
                }
            }
            State::Slide { pos, .. } => {
                let look = match self.kind {
                    RitualGesture::Slide { look } => look,
                    _ => SlideLook::Bottle,
                };
                let (a, b, y) = TRACK;
                let ta = at(vec2(a - 10.0, y + 12.0));
                fill_rrect(Rect::new(ta.x, ta.y, b - a + 20.0, 4.0), 2.0, with_alpha(WHITE, 0.14));
                // where it goes: the counter, the oven's mouth, the door
                let end = at(vec2(b + 8.0, y));
                match look {
                    SlideLook::Bottle => {
                        fill_rrect(Rect::new(o.x + 4.0, o.y + 18.0, 22.0, 58.0), 4.0, hex(0xDDE6EE));
                        fill_rrect(Rect::new(end.x - 18.0, end.y + 8.0, 30.0, 6.0), 2.0, hex(0x8A6A4A));
                    }
                    SlideLook::Tray => {
                        fill_rrect(Rect::new(end.x - 20.0, end.y - 22.0, 30.0, 40.0), 4.0, hex(0x4A3A3A));
                        fill_rrect(Rect::new(end.x - 16.0, end.y - 8.0, 22.0, 16.0), 3.0, with_alpha(hex(0xFF9A4A), 0.6 + 0.2 * (time * 4.0).sin()));
                    }
                    SlideLook::Bag => {
                        fill_rrect(Rect::new(end.x - 16.0, end.y - 34.0, 24.0, 48.0), 3.0, hex(0x5A4A3A));
                        fill_circle(vec2(end.x + 3.0, end.y - 10.0), 2.0, hex(0xD8B060));
                    }
                }
                let p = at(vec2(a + (b - a) * pos, y));
                match look {
                    SlideLook::Bottle => {
                        fill_rrect(Rect::new(p.x - 7.0, p.y - 14.0, 14.0, 24.0), 4.0, hex(0xF6F6F2));
                        fill_rrect(Rect::new(p.x - 4.0, p.y - 20.0, 8.0, 7.0), 2.0, hex(0x5A8AD8));
                    }
                    SlideLook::Tray => {
                        fill_rrect(Rect::new(p.x - 16.0, p.y + 4.0, 32.0, 5.0), 2.0, hex(0xA8AEB8));
                        fill_rrect(Rect::new(p.x - 12.0, p.y - 6.0, 24.0, 10.0), 3.0, hex(0xC88A4A));
                    }
                    SlideLook::Bag => {
                        fill_circle(p + vec2(0.0, 1.0), 11.0, hex(0x2E2A34));
                        fill_rrect(Rect::new(p.x - 3.0, p.y - 15.0, 6.0, 6.0), 2.0, hex(0x2E2A34));
                    }
                }
            }
            State::Stir { angle, .. } => {
                let c = at(POT_C);
                fill_circle(c, POT_R + 4.0, hex(0x6A6E7A));
                fill_circle(c, POT_R, hex(0xE8C67A));
                stroke_circle(c, POT_R * 0.6, 1.0, with_alpha(hex(0xFFF2C8), 0.35));
                // the swirl follows the spoon
                let k = (angle.abs() / TAU).min(1.0);
                if k > 0.02 {
                    arc(c, POT_R * 0.55, -PI / 2.0, -PI / 2.0 + TAU * k, 2.0, with_alpha(hex(0xFFF6DC), 0.7));
                }
                let a = *angle - PI / 2.0;
                let tip = c + vec2(a.cos(), a.sin()) * (POT_R * 0.55);
                line(tip, tip + vec2(18.0, -26.0), 3.0, hex(0xB0885A));
                fill_circle(tip, 4.0, hex(0xC8A070));
            }
        }
        if done {
            let p = self.finished.unwrap_or(false);
            sparkle(o + vec2(STAGE.x - 14.0, 12.0), if p { 9.0 } else { 6.0 }, self.since_done, p);
        }
    }

    fn band_mid(&self) -> f32 {
        let (a, b) = self.band();
        (a + b) / 2.0
    }
}

fn stroke_dots(_ui: &mut Ui, o: Vec2, n: u32, need: u32) {
    for k in 0..need {
        let p = o + vec2(STAGE.x / 2.0 + (k as f32 - (need as f32 - 1.0) / 2.0) * 10.0, 12.0);
        if k < n {
            fill_circle(p, 3.0, hex(0xA5B4FC));
        } else {
            stroke_circle(p, 3.0, 1.0, with_alpha(WHITE, 0.4));
        }
    }
}

fn draw_coin(ui: &mut Ui, p: Vec2, value: u32, sel: bool) {
    let th = ui.theme.clone();
    if value >= 20 {
        // a note
        let r = Rect::new(p.x - 15.0, p.y - 9.0, 30.0, 18.0);
        fill_rrect(r, 3.0, hex(0x7AAE8A));
        stroke_rrect(r, 3.0, if sel { 1.6 } else { 1.0 }, if sel { th.indigo_300 } else { with_alpha(WHITE, 0.4) });
    } else {
        let rad = 8.0 + (value as f32).sqrt() * 1.2;
        fill_circle(p, rad, if value >= 10 { hex(0xE2C46A) } else { hex(0xC8CCD4) });
        stroke_circle(p, rad, if sel { 1.6 } else { 1.0 }, if sel { th.indigo_300 } else { with_alpha(WHITE, 0.45) });
    }
    ui.fonts.draw_in(
        &value.to_string(),
        Rect::new(p.x - 14.0, p.y - 7.0, 28.0, 14.0),
        &th.micro().color(hex(0x2A2236)),
        Align::Center,
    );
}

fn draw_coin_small(p: Vec2, value: u32) {
    if value >= 20 {
        fill_rrect(Rect::new(p.x - 7.0, p.y - 4.0, 14.0, 8.0), 2.0, hex(0x7AAE8A));
    } else {
        fill_circle(p, 4.5, if value >= 10 { hex(0xE2C46A) } else { hex(0xC8CCD4) });
        stroke_circle(p, 4.5, 0.8, with_alpha(hex(0x2A2236), 0.5));
    }
}

/// Jars and bags (the pantry) or bottles and boxes (the fridge), sitting on `base`.
fn draw_shelf_item(base: Vec2, look: ShelfLook, i: usize) {
    match (look, i % 3) {
        (ShelfLook::Pantry, 0) => {
            fill_rrect(Rect::new(base.x - 10.0, base.y - 28.0, 20.0, 28.0), 4.0, with_alpha(hex(0xC8E0F0), 0.8));
            fill_rrect(Rect::new(base.x - 9.0, base.y - 18.0, 18.0, 17.0), 3.0, hex(0x6A3E22));
            fill_rrect(Rect::new(base.x - 11.0, base.y - 32.0, 22.0, 5.0), 2.0, hex(0xB08A5A));
        }
        (ShelfLook::Pantry, 1) => {
            fill_rrect(Rect::new(base.x - 12.0, base.y - 30.0, 24.0, 30.0), 5.0, hex(0xEEE2C8));
            fill_rrect(Rect::new(base.x - 8.0, base.y - 20.0, 16.0, 8.0), 2.0, hex(0xC85A4A));
        }
        (ShelfLook::Pantry, _) => {
            fill_rrect(Rect::new(base.x - 9.0, base.y - 26.0, 18.0, 26.0), 4.0, with_alpha(hex(0xC8E0F0), 0.8));
            fill_rrect(Rect::new(base.x - 8.0, base.y - 14.0, 16.0, 13.0), 3.0, hex(0xF0E6D2));
            fill_rrect(Rect::new(base.x - 10.0, base.y - 30.0, 20.0, 5.0), 2.0, hex(0x8A6A4A));
        }
        (ShelfLook::Fridge, 0) => {
            fill_rrect(Rect::new(base.x - 7.0, base.y - 30.0, 14.0, 30.0), 4.0, hex(0xF6F6F2));
            fill_rrect(Rect::new(base.x - 4.0, base.y - 36.0, 8.0, 7.0), 2.0, hex(0x5A8AD8));
        }
        (ShelfLook::Fridge, 1) => {
            fill_rrect(Rect::new(base.x - 13.0, base.y - 14.0, 26.0, 14.0), 3.0, hex(0xF2D46A));
        }
        (ShelfLook::Fridge, _) => {
            fill_rrect(Rect::new(base.x - 9.0, base.y - 20.0, 18.0, 20.0), 4.0, hex(0xE8EEF6));
            fill_rrect(Rect::new(base.x - 9.0, base.y - 20.0, 18.0, 5.0), 2.0, hex(0xD86A8A));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::handson::RitualGesture as G;

    fn kinds() -> Vec<G> {
        vec![
            G::Cord,
            G::Needle { green: (0.62, 0.8) },
            G::Count {
                coins: vec![1, 2, 5, 10, 20],
                total: (32, 68),
            },
            G::Strokes {
                count: 3,
                look: StrokeLook::Cloth,
            },
            G::Shelf {
                items: 3,
                look: ShelfLook::Pantry,
            },
            G::Drop { band: 0.16 },
            G::Sort { coins: 5 },
            G::Lines { count: 3 },
            G::Slide {
                look: SlideLook::Bag,
            },
            G::Stir,
        ]
    }

    fn play(g: &mut Gesture, keys: bool, max: f32) -> (Option<bool>, f32) {
        let dt = 1.0 / 30.0;
        let mut t = 0.0;
        while t < max {
            let i = if keys {
                g.autoplay_keyboard_input()
            } else {
                g.autoplay_input()
            };
            if let GestureStatus::Done { perfect } = g.step(&i, dt) {
                return (Some(perfect), t);
            }
            t += dt;
        }
        (None, t)
    }

    #[test]
    fn every_gesture_plays_well_by_mouse_and_by_keyboard_in_a_few_seconds() {
        for k in kinds() {
            for keys in [false, true] {
                for seed in [1u64, 7, 4242, 99_991] {
                    let mut g = Gesture::new(&k, seed, false, 3);
                    let (done, t) = play(&mut g, keys, 12.0);
                    assert_eq!(done, Some(true), "{} keys={keys} seed={seed}", k.id());
                    assert!(t <= 6.5, "{} keys={keys} took {t:.1} s", k.id());
                    assert!(g.started());
                }
            }
        }
    }

    #[test]
    fn untouched_a_gesture_never_finishes_or_starts() {
        for k in kinds() {
            let mut g = Gesture::new(&k, 3, false, 3);
            for _ in 0..600 {
                assert_eq!(g.step(&MgInput::default(), 1.0 / 30.0), GestureStatus::Running);
            }
            assert!(!g.started(), "{}", k.id());
        }
    }

    #[test]
    fn mistakes_are_never_failure_only_not_perfect() {
        // overshooting the float: take a coin back, still done, not perfect
        let k = G::Count {
            coins: vec![1, 2, 5, 10, 20],
            total: (32, 32),
        };
        let mut g = Gesture::new(&k, 0, false, 0);
        let tap = |g: &mut Gesture, p: Vec2| {
            g.step(
                &MgInput {
                    pointer: Some(p),
                    pressed: true,
                    ..Default::default()
                },
                0.05,
            )
        };
        for _ in 0..2 {
            tap(&mut g, vec2(coin_x(4, 5), COIN_Y));
        }
        assert!(g.count_over(), "40 > 32");
        let (done, _) = play(&mut g, false, 10.0);
        assert_eq!(done, Some(false));
        // a needle released low, then in the green: done, not perfect
        let mut g = Gesture::new(&G::Needle { green: (0.62, 0.8) }, 0, false, 0);
        let hold = MgInput {
            pointer: Some(STAGE / 2.0),
            pressed: true,
            down: true,
            ..Default::default()
        };
        g.step(&hold, 0.1);
        g.step(&MgInput::default(), 0.1);
        let (done, _) = play(&mut g, false, 10.0);
        assert_eq!(done, Some(false));
    }

    #[test]
    fn the_float_target_is_stable_per_night_and_never_touches_the_world_rng() {
        let k = G::Count {
            coins: vec![1, 2, 5, 10, 20],
            total: (32, 68),
        };
        let a = Gesture::new(&k, seed_for(4242, 3, "count_float"), false, 0);
        let b = Gesture::new(&k, seed_for(4242, 3, "count_float"), false, 0);
        assert_eq!(a.count_totals(), b.count_totals());
        let (_, t) = a.count_totals().unwrap();
        assert!((32..=68).contains(&t));
    }
}
