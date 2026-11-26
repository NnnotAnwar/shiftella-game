//! The station cat — a small original tabby in the violet-dollhouse style
//! (3/4 view, soft gradients, no outlines, lavender rim light). Used for the
//! "feed the station cat" night moment.
//!
//! Coordinates: world pixels, `feet` is the floor point under the cat (between the
//! front paws when sitting). At scale 1 a sitting cat is ~34 px tall (a person is ~83).

use std::f32::consts::PI;

use macroquad::prelude::*;

use super::colors as c;
use super::draw::{alpha, capsule, ellipse, ellipse_v, glow, mix, poly, poly_v, shade, soft_shadow};

/// What the cat is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CatPose {
    Sit,
    Walk,
    /// Curled up, breathing slowly.
    Sleep,
    /// Crouched over a little bowl (the bowl is drawn too).
    Eat,
}

/// Coat colours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CatCoat {
    pub light: Color,
    pub base: Color,
    pub dark: Color,
    pub stripe: Color,
    pub cream: Color,
    pub eye: Color,
}

impl CatCoat {
    /// Smoky lavender tabby with a cream chest and amber eyes (the station cat).
    pub const STATION: CatCoat = CatCoat {
        light: Color::from_hex(0xaaa1c4),
        base: Color::from_hex(0x8d84a8),
        dark: Color::from_hex(0x655c84),
        stripe: Color::from_hex(0x5a5178),
        cream: Color::from_hex(0xf1e8de),
        eye: Color::from_hex(0xf3c15a),
    };
    /// Warm ginger.
    pub const GINGER: CatCoat = CatCoat {
        light: Color::from_hex(0xf2bc7e),
        base: Color::from_hex(0xdf9858),
        dark: Color::from_hex(0xb06a36),
        stripe: Color::from_hex(0xa65d2a),
        cream: Color::from_hex(0xf7ecdf),
        eye: Color::from_hex(0x9fd36a),
    };
}

/// Look of one cat.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CatLook {
    pub scale: f32,
    /// Face left instead of right.
    pub flip: bool,
    pub coat: CatCoat,
}

impl Default for CatLook {
    fn default() -> Self {
        CatLook {
            scale: 1.0,
            flip: false,
            coat: CatCoat::STATION,
        }
    }
}

/// Station cat at world scale, facing right.
pub fn draw_cat(feet: Vec2, pose: CatPose, t: f32, affection: f32) {
    draw_cat_ex(feet, &CatLook::default(), pose, t, affection);
}

/// Local → world transform.
#[derive(Clone, Copy)]
struct Xf {
    o: Vec2,
    s: f32,
    sx: f32,
}

impl Xf {
    fn p(&self, x: f32, y: f32) -> Vec2 {
        self.o + vec2(x * self.sx, y) * self.s
    }
    fn v(&self, v: Vec2) -> Vec2 {
        self.p(v.x, v.y)
    }
    fn ell(&self, x: f32, y: f32, rx: f32, ry: f32, top: Color, bottom: Color) {
        let q = self.p(x, y);
        ellipse_v(q.x, q.y, rx * self.s, ry * self.s, top, bottom);
    }
    fn cap(&self, a: Vec2, b: Vec2, r: f32, col: Color) {
        capsule(self.v(a), self.v(b), r * self.s, col);
    }
    fn tri(&self, a: Vec2, b: Vec2, cc: Vec2, col: Color) {
        let mut pts = [self.v(a), self.v(b), self.v(cc)];
        if self.sx < 0.0 {
            pts.swap(1, 2);
        }
        poly(&pts, col);
    }
    /// Thin stroke through points.
    fn stroke(&self, pts: &[Vec2], w: f32, col: Color) {
        for s in pts.windows(2) {
            let a = self.v(s[0]);
            let b = self.v(s[1]);
            draw_line(a.x, a.y, b.x, b.y, w * self.s, col);
        }
    }
}

fn bezier(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, u: f32) -> Vec2 {
    let v = 1.0 - u;
    p0 * v * v * v + p1 * 3.0 * v * v * u + p2 * 3.0 * v * u * u + p3 * u * u * u
}

/// Tapered tail along a cubic curve.
fn tail(x: &Xf, pts: [Vec2; 4], r0: f32, r1: f32, coat: &CatCoat, tip_dark: bool) {
    let n = 14;
    let mut prev = pts[0];
    for i in 1..=n {
        let u = i as f32 / n as f32;
        let q = bezier(pts[0], pts[1], pts[2], pts[3], u);
        let r = r0 + (r1 - r0) * u;
        let col = if tip_dark && u > 0.78 {
            coat.dark
        } else if (i / 3) % 2 == 1 {
            mix(coat.base, coat.stripe, 0.45)
        } else {
            coat.base
        };
        x.cap(prev, q, r, col);
        prev = q;
    }
}

/// Seconds-based blink: returns eye openness 0..1.
fn blink(t: f32, seed: f32) -> f32 {
    let period = 3.7 + seed;
    let ph = (t + seed * 1.3) % period;
    if ph < 0.14 {
        let k = ph / 0.14;
        (k * 2.0 - 1.0).abs()
    } else {
        1.0
    }
}

/// Head with ears, face and whiskers at local `h` (centre), `tilt` in local units.
fn head(x: &Xf, h: Vec2, coat: &CatCoat, t: f32, eyes_open: f32, content: bool, ear_twitch: f32) {
    let k = x.s;
    let _ = k;
    // ears (back one first, slightly darker)
    let ear = |base_l: Vec2, base_r: Vec2, tip: Vec2, inner: bool, col: Color| {
        x.tri(base_l, tip, base_r, col);
        if inner {
            let m = (base_l + base_r) * 0.5;
            x.tri(
                base_l.lerp(m, 0.35) + vec2(0.0, -0.4),
                tip.lerp(m, 0.3),
                base_r.lerp(m, 0.35) + vec2(0.0, -0.4),
                Color::from_hex(0xe7a9bb),
            );
        }
    };
    ear(h + vec2(-7.5, -3.0), h + vec2(-2.0, -6.5), h + vec2(-7.0, -12.5 + ear_twitch), true, coat.dark);
    ear(h + vec2(0.5, -6.8), h + vec2(6.8, -3.5), h + vec2(5.0, -13.0), true, coat.base);
    // head
    x.ell(h.x, h.y, 8.6, 7.4, coat.light, coat.base);
    // cheek fluff
    x.ell(h.x + 1.0, h.y + 3.0, 8.0, 4.6, coat.base, shade(coat.base, 0.92));
    // forehead stripes
    for (i, dx) in [-1.8f32, 0.6, 3.0].iter().enumerate() {
        let len = if i == 1 { 3.4 } else { 2.6 };
        x.stroke(&[h + vec2(*dx, -6.6), h + vec2(*dx + 0.2, -6.6 + len)], 0.9, alpha(coat.stripe, 0.8));
    }
    // muzzle (cream), towards the facing side
    let m = h + vec2(3.6, 2.6);
    x.ell(m.x, m.y, 4.6, 3.2, coat.cream, shade(coat.cream, 0.92));
    // eyes
    for (ex, big) in [(h.x - 1.2, 1.0f32), (h.x + 5.0, 0.92)] {
        let e = x.p(ex, h.y - 0.6);
        let s = x.s * big;
        if content || eyes_open < 0.25 {
            // happy closed arcs
            let arc_pts: Vec<Vec2> = (0..=6)
                .map(|i| {
                    let a = PI * (i as f32 / 6.0);
                    vec2(ex + a.cos() * 1.7 * big, h.y - 0.6 + -(a.sin()) * 1.1 * big + 0.6)
                })
                .collect();
            x.stroke(&arc_pts, 0.85, Color::from_hex(0x2a2238));
        } else {
            let ry = 2.0 * eyes_open;
            ellipse(e.x, e.y, 1.7 * s, ry * s, coat.eye);
            ellipse(e.x + 0.2 * x.sx * s, e.y, 0.65 * s, ry * 0.92 * s, Color::from_hex(0x2a2238));
            ellipse(e.x - 0.5 * x.sx * s, e.y - 0.7 * s * eyes_open, 0.45 * s, 0.45 * s, WHITE);
        }
    }
    // nose + mouth
    let n = h + vec2(4.4, 1.6);
    x.tri(n + vec2(-1.1, -0.6), n + vec2(1.1, -0.6), n + vec2(0.0, 0.7), Color::from_hex(0xe48c9f));
    x.stroke(&[n + vec2(0.0, 0.7), n + vec2(-0.9, 1.6), n + vec2(-1.8, 1.3)], 0.6, alpha(Color::from_hex(0x4a3a52), 0.8));
    x.stroke(&[n + vec2(0.0, 0.7), n + vec2(0.9, 1.6), n + vec2(1.8, 1.3)], 0.6, alpha(Color::from_hex(0x4a3a52), 0.8));
    // whiskers (twitch a little)
    let w = (t * 1.7).sin() * 0.3;
    for (dy, end_dy) in [(0.8f32, -0.6f32), (1.8, 1.6)] {
        x.stroke(&[n + vec2(1.5, dy), n + vec2(8.5, end_dy + w)], 0.35, Color::new(1.0, 1.0, 1.0, 0.55));
        x.stroke(&[n + vec2(-2.5, dy), n + vec2(-9.5, end_dy + 0.4 - w)], 0.35, Color::new(1.0, 1.0, 1.0, 0.35));
    }
    // lavender rim light on the back of the head
    let rim: Vec<Vec2> = (0..=6)
        .map(|i| {
            let a = PI * 0.9 + i as f32 / 6.0 * 0.9;
            h + vec2(a.cos() * 8.2, a.sin() * 7.0)
        })
        .collect();
    x.stroke(&rim, 0.9, alpha(c::LAVENDER_RIM, 0.55));
}

fn heart(p: Vec2, s: f32, a: f32) {
    let col = alpha(c::ROSE, a);
    glow(p.x, p.y, 10.0 * s, alpha(c::ROSE, 0.3 * a));
    ellipse(p.x - 2.2 * s, p.y - 1.2 * s, 2.7 * s, 2.7 * s, col);
    ellipse(p.x + 2.2 * s, p.y - 1.2 * s, 2.7 * s, 2.7 * s, col);
    poly(&[p + vec2(-4.8, -0.4) * s, p + vec2(4.8, -0.4) * s, p + vec2(0.0, 5.2) * s], col);
    ellipse(p.x - 2.9 * s, p.y - 2.2 * s, 0.9 * s, 0.9 * s, alpha(WHITE, 0.75 * a));
}

fn bowl(x: &Xf, at: Vec2, t: f32) {
    let q = x.v(at);
    let s = x.s;
    soft_shadow(q.x, q.y + 0.5 * s, 9.0 * s, 2.6 * s, 0.4);
    // body (front face) + rim
    poly_v(
        &[
            q + vec2(-7.5, -4.0) * s,
            q + vec2(7.5, -4.0) * s,
            q + vec2(5.6, 0.0) * s,
            q + vec2(-5.6, 0.0) * s,
        ],
        c::INDIGO_SOFT,
        c::INDIGO_600,
    );
    ellipse(q.x, q.y - 4.0 * s, 7.5 * s, 2.0 * s, Color::from_hex(0xd9dcff));
    ellipse(q.x, q.y - 3.8 * s, 6.0 * s, 1.4 * s, Color::from_hex(0x8a5a3a));
    // a few kibble bits
    for i in 0..4 {
        let dx = (i as f32 - 1.5) * 2.6 + (t * 0.0).sin();
        ellipse(q.x + dx * s, q.y - 4.1 * s, 0.9 * s, 0.6 * s, Color::from_hex(0xb77b4c));
    }
}

/// Full control: look, pose, time (s) and affection 0..1 (≥0.75 shows a heart).
pub fn draw_cat_ex(feet: Vec2, look: &CatLook, pose: CatPose, t: f32, affection: f32) {
    let x = Xf {
        o: feet,
        s: look.scale,
        sx: if look.flip { -1.0 } else { 1.0 },
    };
    let coat = &look.coat;
    let breathe = (t * 2.1).sin();
    let open = blink(t, feet.x.abs() % 1.7);
    let content = affection > 0.9;
    let twitch = if (t % 5.3) < 0.18 { -1.4 } else { 0.0 };
    let shadow_rx = match pose {
        CatPose::Walk => 16.0,
        CatPose::Sleep => 16.0,
        _ => 13.0,
    };
    let sp = x.p(0.0, 0.0);
    soft_shadow(sp.x, sp.y, shadow_rx * x.s, 4.0 * x.s, 0.45);

    let head_top: Vec2 = match pose {
        CatPose::Sit => {
            // tail curls around the paws, the tip swishes
            let swish = (t * 1.3).sin();
            tail(
                &x,
                [vec2(-9.0, -3.0), vec2(-14.0, 1.5), vec2(2.0, 2.5), vec2(10.0 + swish * 1.5, -1.0 - swish.max(0.0) * 3.0)],
                2.8,
                1.9,
                coat,
                true,
            );
            let b = 1.0 + 0.015 * breathe;
            // haunch, body, chest
            x.ell(-4.0, -8.0, 10.0, 8.6, coat.light, coat.dark);
            for i in 0..3 {
                let yy = -12.0 + i as f32 * 3.0;
                x.stroke(&[vec2(-11.0, yy), vec2(-7.0, yy - 1.0), vec2(-4.0, yy + 0.5)], 1.0, alpha(coat.stripe, 0.6));
            }
            x.ell(2.0, -14.0 * b, 7.8, 11.0 * b, coat.light, coat.base);
            x.ell(5.0, -13.0 * b, 4.2, 7.0 * b, coat.cream, shade(coat.cream, 0.9));
            // front legs + paws
            x.cap(vec2(2.6, -10.0), vec2(2.6, -1.2), 2.2, coat.base);
            x.cap(vec2(7.0, -10.0), vec2(7.2, -1.2), 2.2, coat.light);
            x.ell(2.8, -1.0, 2.6, 1.5, coat.cream, shade(coat.cream, 0.9));
            x.ell(7.6, -1.0, 2.6, 1.5, coat.cream, shade(coat.cream, 0.9));
            // rim light along the back
            let rim: Vec<Vec2> = (0..=7).map(|i| {
                let a = PI * 0.95 + i as f32 / 7.0 * 0.8;
                vec2(-4.0 + a.cos() * 9.6, -8.0 + a.sin() * 8.2)
            }).collect();
            x.stroke(&rim, 1.0, alpha(c::LAVENDER_RIM, 0.5));
            let h = vec2(4.5, -26.0 * b + (t * 0.6).sin() * 0.3);
            head(&x, h, coat, t, open, content, twitch);
            h
        }
        CatPose::Walk => {
            let gait = t * 6.0;
            let bob = (gait * 2.0).sin().abs() * 0.8;
            let sway = (t * 2.0).sin();
            // tail up with a curl at the tip
            tail(
                &x,
                [vec2(-12.0, -14.0 - bob), vec2(-19.0, -18.0), vec2(-18.0 + sway, -30.0), vec2(-13.0 + sway * 2.0, -32.0)],
                2.6,
                1.8,
                coat,
                true,
            );
            // far legs (darker), body, near legs
            let leg = |hip: Vec2, ph: f32, col: Color| {
                let sw = ph.sin() * 3.0;
                let lift = ph.cos().max(0.0) * 1.6;
                x.cap(hip, vec2(hip.x + sw, -1.2 - lift), 2.0, col);
                x.ell(hip.x + sw + 0.8, -1.0 - lift, 2.3, 1.3, shade(coat.cream, 0.85), shade(coat.cream, 0.8));
            };
            leg(vec2(-7.0, -10.0 - bob), gait + PI, coat.dark);
            leg(vec2(8.0, -10.0 - bob), gait, coat.dark);
            x.ell(0.0, -13.0 - bob, 13.5, 7.2, coat.light, coat.dark);
            x.ell(4.0, -10.5 - bob, 7.0, 3.5, mix(coat.base, coat.cream, 0.5), coat.cream);
            for i in 0..4 {
                let xx = -9.0 + i as f32 * 4.5;
                x.stroke(&[vec2(xx, -19.5 - bob), vec2(xx + 1.0, -15.5 - bob)], 1.1, alpha(coat.stripe, 0.65));
            }
            leg(vec2(-5.0, -9.5 - bob), gait, coat.base);
            leg(vec2(10.0, -9.5 - bob), gait + PI, coat.base);
            let rim: Vec<Vec2> = (0..=8).map(|i| {
                let a = PI * 1.1 + i as f32 / 8.0 * 0.9;
                vec2(a.cos() * 13.2, -13.0 - bob + a.sin() * 7.0)
            }).collect();
            x.stroke(&rim, 1.0, alpha(c::LAVENDER_RIM, 0.5));
            let h = vec2(15.0, -21.0 - bob);
            head(&x, h, coat, t, open, content, twitch);
            h
        }
        CatPose::Eat => {
            bowl(&x, vec2(17.0, 0.0), t);
            let chew = (t * 7.0).sin() * 0.6;
            let flick = ((t * 0.8).sin() * 4.0).sin();
            tail(
                &x,
                [vec2(-11.0, -4.0), vec2(-17.0, 0.0), vec2(-20.0, -2.0), vec2(-23.0 + flick, -6.0 - flick.max(0.0) * 2.0)],
                2.6,
                1.8,
                coat,
                true,
            );
            // loaf body
            x.ell(-2.0, -8.0, 12.5, 8.0, coat.light, coat.dark);
            for i in 0..3 {
                let xx = -9.0 + i as f32 * 4.5;
                x.stroke(&[vec2(xx, -15.0), vec2(xx + 1.0, -11.5)], 1.1, alpha(coat.stripe, 0.6));
            }
            x.cap(vec2(6.0, -6.0), vec2(9.0, -1.2), 2.1, coat.base);
            x.ell(9.6, -1.0, 2.6, 1.5, coat.cream, shade(coat.cream, 0.9));
            x.ell(5.0, -6.0, 5.0, 4.0, coat.cream, shade(coat.cream, 0.88));
            let h = vec2(12.0, -9.0 + chew);
            head(&x, h, coat, t, 0.0, true, twitch);
            h
        }
        CatPose::Sleep => {
            let slow = (t * 1.1).sin();
            let b = 1.0 + 0.04 * slow;
            // tail wraps around the front
            tail(
                &x,
                [vec2(-12.0, -3.0), vec2(-14.0, 2.0), vec2(4.0, 3.5), vec2(13.0, 0.0)],
                2.8,
                2.0,
                coat,
                true,
            );
            x.ell(-1.0, -7.0 * b, 14.5, 7.5 * b, coat.light, coat.dark);
            for i in 0..4 {
                let xx = -10.0 + i as f32 * 4.5;
                x.stroke(&[vec2(xx, -13.0 * b), vec2(xx + 1.0, -9.5 * b)], 1.1, alpha(coat.stripe, 0.6));
            }
            let rim: Vec<Vec2> = (0..=8).map(|i| {
                let a = PI * 1.05 + i as f32 / 8.0 * 1.0;
                vec2(-1.0 + a.cos() * 14.2, -7.0 * b + a.sin() * 7.2 * b)
            }).collect();
            x.stroke(&rim, 1.0, alpha(c::LAVENDER_RIM, 0.5));
            // front paws tucked under the chin
            x.ell(9.0, -2.2, 3.2, 1.7, coat.cream, shade(coat.cream, 0.9));
            let h = vec2(8.0, -8.5 * b);
            head(&x, h, coat, t, 0.0, true, 0.0);
            // floating z's
            for i in 0..3 {
                let ph = ((t * 0.45) + i as f32 / 3.0).fract();
                let zp = h + vec2(8.0 + ph * 6.0, -12.0 - ph * 14.0);
                let zs = 1.4 + ph * 1.2;
                let a = (ph * (1.0 - ph) * 4.0).clamp(0.0, 1.0) * 0.8;
                let col = alpha(c::INDIGO_SOFT, a);
                x.stroke(&[zp + vec2(-zs, -zs), zp + vec2(zs, -zs), zp + vec2(-zs, zs), zp + vec2(zs, zs)], 0.7, col);
            }
            h
        }
    };

    // affection heart: pops up, floats, fades, repeats
    if affection >= 0.75 {
        let cycle = 2.6;
        let ph = (t % cycle) / cycle;
        let pop = (ph / 0.15).clamp(0.0, 1.0);
        let c1 = 1.70158;
        let k = 1.0 + (c1 + 1.0) * (pop - 1.0).powi(3) + c1 * (pop - 1.0).powi(2);
        let fade = 1.0 - ((ph - 0.7) / 0.3).clamp(0.0, 1.0);
        let p = x.p(head_top.x + 1.0, head_top.y - 16.0 - ph * 6.0);
        let s = x.s * (0.8 + 0.2 * k) * (0.6 + 0.4 * affection);
        heart(p, s, fade * k.min(1.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blink_is_bounded() {
        for i in 0..400 {
            let o = blink(i as f32 * 0.013, 0.4);
            assert!((0.0..=1.0).contains(&o));
        }
    }

    #[test]
    fn bezier_endpoints() {
        let a = vec2(0.0, 0.0);
        let d = vec2(4.0, 2.0);
        assert_eq!(bezier(a, vec2(1.0, 1.0), vec2(2.0, 3.0), d, 0.0), a);
        assert!((bezier(a, vec2(1.0, 1.0), vec2(2.0, 3.0), d, 1.0) - d).length() < 1e-5);
    }
}
