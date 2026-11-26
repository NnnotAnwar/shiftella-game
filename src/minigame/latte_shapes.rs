//! Latte-art guide patterns (`design/handson.md` §2.2): each pattern is a list of strokes
//! (polylines) in the unit disc — `(0,0)` = cup centre, radius 1 = crema edge, y down.
//! The player traces the strokes in order from each start dot; keyboard players tap
//! Left/Right at the marked wobble points while the spout follows the guide.

use std::f32::consts::{PI, TAU};

use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

/// Latte-art pattern. Ids match `minigames.ron` `latte_patterns`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LatteShape {
    Heart,
    Tulip,
    Rosetta,
    Leaf,
    Swan,
    Snowflake,
    /// Crescent moon with a little star (moon latte).
    Moon,
    /// A tiny night train with a puff of steam.
    Train,
    /// Cat face (station cat affection ≥ 3).
    Cat,
    /// Music note (when Lori is present).
    Note,
    /// A heart with an "L" (Vera's memory night).
    HeartL,
}

/// A wobble point for keyboard play: arc position along a stroke and the tap side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mark {
    pub stroke: usize,
    /// Arc length from the stroke start (unit-disc units).
    pub s: f32,
    /// `true` = Left, `false` = Right (either key is accepted; this is the arrow shown).
    pub left: bool,
}

const SPACING: f32 = 0.015;

impl LatteShape {
    pub const ALL: [LatteShape; 11] = [
        LatteShape::Heart,
        LatteShape::Tulip,
        LatteShape::Rosetta,
        LatteShape::Leaf,
        LatteShape::Swan,
        LatteShape::Snowflake,
        LatteShape::Moon,
        LatteShape::Train,
        LatteShape::Cat,
        LatteShape::Note,
        LatteShape::HeartL,
    ];
    /// The night-unlocked base patterns (heart … swan).
    pub const BASIC: [LatteShape; 5] = [
        LatteShape::Heart,
        LatteShape::Tulip,
        LatteShape::Rosetta,
        LatteShape::Leaf,
        LatteShape::Swan,
    ];

    pub fn id(self) -> &'static str {
        match self {
            LatteShape::Heart => "heart",
            LatteShape::Tulip => "tulip",
            LatteShape::Rosetta => "rosetta",
            LatteShape::Leaf => "leaf",
            LatteShape::Swan => "swan",
            LatteShape::Snowflake => "snowflake",
            LatteShape::Moon => "moon",
            LatteShape::Train => "train",
            LatteShape::Cat => "cat",
            LatteShape::Note => "note",
            LatteShape::HeartL => "heart_l",
        }
    }

    pub fn from_id(id: &str) -> Option<LatteShape> {
        LatteShape::ALL.into_iter().find(|s| s.id() == id)
    }

    /// `latte.<id>`
    pub fn key(self) -> String {
        format!("latte.{}", self.id())
    }

    /// Dense strokes (spacing ≈ 0.015) in the unit disc.
    pub fn strokes(self) -> Vec<Vec<Vec2>> {
        let raw: Vec<Vec<Vec2>> = match self {
            LatteShape::Heart => vec![heart(Vec2::ZERO, 1.0)],
            LatteShape::HeartL => vec![
                heart(Vec2::ZERO, 1.05),
                vec![vec2(-0.12, -0.12), vec2(-0.12, 0.18), vec2(0.15, 0.18)],
            ],
            LatteShape::Tulip => vec![
                arc(vec2(0.0, 0.2), 0.46, 0.26, PI, 0.0),
                arc(vec2(0.0, -0.06), 0.36, 0.2, PI, 0.0),
                heart(vec2(0.0, -0.36), 0.4),
                vec![vec2(0.0, -0.52), vec2(0.0, 0.64)],
            ],
            LatteShape::Rosetta => vec![
                zigzag(vec2(0.0, 0.48), vec2(0.0, -0.48), 0.48, 0.14, 10),
                vec![vec2(0.0, -0.62), vec2(0.0, 0.64)],
            ],
            LatteShape::Leaf => {
                let rot = |v: Vec2| rotate(v, -0.45);
                let mut lens: Vec<Vec2> = (0..=24)
                    .map(|i| {
                        let u = i as f32 / 24.0;
                        vec2(-0.32 * (PI * u).sin(), 0.6 - 1.2 * u)
                    })
                    .collect();
                lens.extend((1..=24).map(|i| {
                    let u = i as f32 / 24.0;
                    vec2(0.32 * (PI * u).sin(), -0.6 + 1.2 * u)
                }));
                vec![
                    lens.into_iter().map(rot).collect(),
                    vec![rot(vec2(0.0, -0.48)), rot(vec2(0.0, 0.74))],
                ]
            }
            LatteShape::Swan => vec![
                zigzag_h(vec2(0.42, 0.3), vec2(-0.44, 0.1), 0.2, 0.08, 8),
                cubic(
                    vec2(0.46, 0.34),
                    vec2(0.66, -0.06),
                    vec2(0.32, -0.62),
                    vec2(0.06, -0.48),
                ),
                vec![vec2(0.06, -0.48), vec2(-0.08, -0.42)],
            ],
            LatteShape::Snowflake => {
                let mut s = Vec::new();
                for k in 0..3 {
                    let a = PI / 2.0 + k as f32 * PI / 3.0;
                    let d = vec2(a.cos(), a.sin());
                    s.push(vec![-d * 0.58, d * 0.58]);
                }
                for k in 0..6 {
                    let a = PI / 2.0 + k as f32 * PI / 3.0;
                    let d = vec2(a.cos(), a.sin());
                    let p = d * 0.4;
                    let b1 = p + rotate(d, 0.75) * 0.15;
                    let b2 = p + rotate(d, -0.75) * 0.15;
                    s.push(vec![b1, p, b2]);
                }
                s
            }
            LatteShape::Moon => {
                let mut c = arc(vec2(-0.05, 0.02), 0.56, 0.56, -1.3, -(TAU - 1.3));
                c.extend(arc(vec2(0.25, 0.02), 0.56, 0.56, 1.84, TAU - 1.84));
                vec![
                    c,
                    vec![vec2(0.5, -0.62), vec2(0.5, -0.36)],
                    vec![vec2(0.38, -0.49), vec2(0.62, -0.49)],
                ]
            }
            LatteShape::Train => vec![
                rect_loop(-0.42, -0.08, 0.5, 0.22),
                vec![
                    vec2(-0.42, -0.08),
                    vec2(-0.42, -0.4),
                    vec2(-0.1, -0.4),
                    vec2(-0.1, -0.08),
                ],
                vec![
                    vec2(0.2, -0.08),
                    vec2(0.2, -0.3),
                    vec2(0.32, -0.3),
                    vec2(0.32, -0.08),
                ],
                circle(vec2(-0.26, 0.32), 0.09),
                circle(vec2(0.04, 0.32), 0.09),
                circle(vec2(0.34, 0.32), 0.09),
                circle(vec2(0.34, -0.43), 0.07),
                circle(vec2(0.5, -0.6), 0.09),
            ],
            LatteShape::Cat => vec![
                circle(vec2(0.0, 0.1), 0.42),
                vec![vec2(-0.36, -0.12), vec2(-0.34, -0.56), vec2(-0.08, -0.31)],
                vec![vec2(0.36, -0.12), vec2(0.34, -0.56), vec2(0.08, -0.31)],
                vec![vec2(-0.06, 0.13), vec2(0.0, 0.19), vec2(0.06, 0.13)],
                vec![vec2(-0.16, 0.2), vec2(-0.52, 0.12)],
                vec![vec2(-0.16, 0.26), vec2(-0.52, 0.32)],
                vec![vec2(0.16, 0.2), vec2(0.52, 0.12)],
                vec![vec2(0.16, 0.26), vec2(0.52, 0.32)],
            ],
            LatteShape::Note => vec![
                (0..=36)
                    .map(|i| {
                        let a = i as f32 / 36.0 * TAU;
                        vec2(-0.15, 0.36) + rotate(vec2(a.cos() * 0.19, a.sin() * 0.13), -0.4)
                    })
                    .collect(),
                vec![vec2(0.03, 0.32), vec2(0.03, -0.52)],
                cubic(
                    vec2(0.03, -0.52),
                    vec2(0.26, -0.38),
                    vec2(0.4, -0.26),
                    vec2(0.3, -0.04),
                ),
            ],
        };
        raw.into_iter().map(|s| densify(&s, SPACING)).collect()
    }

    /// The whole guide resampled to `n` points evenly by arc length (scoring, §2.2).
    pub fn guide_points(self, n: usize) -> Vec<Vec2> {
        let strokes = self.strokes();
        let total: f32 = strokes.iter().map(|s| length(s)).sum();
        let mut out = Vec::with_capacity(n);
        for k in 0..n {
            let mut want = (k as f32 + 0.5) / n as f32 * total;
            for s in &strokes {
                let l = length(s);
                if want <= l {
                    out.push(point_at(s, want));
                    break;
                }
                want -= l;
            }
        }
        out
    }

    /// Wobble points for keyboard play: later stroke starts and sharp turns (≥ 3 marks).
    pub fn marks(self) -> Vec<Mark> {
        let strokes = self.strokes();
        let mut marks = Vec::new();
        for (si, s) in strokes.iter().enumerate() {
            if si > 0 {
                marks.push(Mark {
                    stroke: si,
                    s: 0.0,
                    left: si % 2 == 1,
                });
            }
            let k = 3;
            let mut arc_s = vec![0.0; s.len()];
            for i in 1..s.len() {
                arc_s[i] = arc_s[i - 1] + s[i].distance(s[i - 1]);
            }
            let turn_at = |j: usize| {
                let a = (s[j] - s[j - k]).normalize_or_zero();
                let b = (s[j + k] - s[j]).normalize_or_zero();
                (a.dot(b).clamp(-1.0, 1.0).acos(), a.perp_dot(b))
            };
            let mut last = -1.0f32;
            let mut i = k;
            while i + k < s.len() {
                let (turn, _) = turn_at(i);
                if turn > 0.95 && arc_s[i] - last > 0.12 {
                    // the sharpest point of this corner
                    let mut best = i;
                    let mut best_turn = turn;
                    for j in i..(i + 4).min(s.len() - k) {
                        let (t, _) = turn_at(j);
                        if t > best_turn {
                            best = j;
                            best_turn = t;
                        }
                    }
                    let (_, side) = turn_at(best);
                    marks.push(Mark {
                        stroke: si,
                        s: arc_s[best],
                        left: side < 0.0,
                    });
                    last = arc_s[best];
                    i = best + k;
                }
                i += 1;
            }
        }
        if marks.len() < 3 {
            let (si, s) = strokes
                .iter()
                .enumerate()
                .max_by(|a, b| length(a.1).total_cmp(&length(b.1)))
                .expect("a pattern has strokes");
            let l = length(s);
            for q in 1..=3 {
                marks.push(Mark {
                    stroke: si,
                    s: l * q as f32 / 4.0,
                    left: q % 2 == 1,
                });
            }
        }
        marks.sort_by(|a, b| a.stroke.cmp(&b.stroke).then(a.s.total_cmp(&b.s)));
        marks
    }
}

/// Total polyline length.
pub fn length(s: &[Vec2]) -> f32 {
    s.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// Point at arc length `d` along a polyline (clamped).
pub fn point_at(s: &[Vec2], mut d: f32) -> Vec2 {
    for w in s.windows(2) {
        let l = w[0].distance(w[1]);
        if d <= l {
            return w[0].lerp(w[1], if l > 0.0 { d / l } else { 0.0 });
        }
        d -= l;
    }
    *s.last().unwrap_or(&Vec2::ZERO)
}

fn densify(s: &[Vec2], step: f32) -> Vec<Vec2> {
    let mut out = vec![s[0]];
    for w in s.windows(2) {
        let n = (w[0].distance(w[1]) / step).ceil().max(1.0) as usize;
        for k in 1..=n {
            out.push(w[0].lerp(w[1], k as f32 / n as f32));
        }
    }
    out
}

fn rotate(v: Vec2, a: f32) -> Vec2 {
    let (s, c) = a.sin_cos();
    vec2(v.x * c - v.y * s, v.x * s + v.y * c)
}

/// The classic heart curve, starting and ending at the bottom tip.
fn heart(c: Vec2, scale: f32) -> Vec<Vec2> {
    let k = 0.034 * scale;
    (0..=96)
        .map(|i| {
            let t = PI + i as f32 / 96.0 * TAU;
            let x = 16.0 * t.sin().powi(3);
            let y =
                -(13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos());
            c + vec2(x, y - 2.5) * k
        })
        .collect()
}

fn arc(c: Vec2, rx: f32, ry: f32, a0: f32, a1: f32) -> Vec<Vec2> {
    let n = ((a1 - a0).abs() * 12.0).ceil().max(4.0) as usize;
    (0..=n)
        .map(|i| {
            let a = a0 + (a1 - a0) * i as f32 / n as f32;
            c + vec2(a.cos() * rx, a.sin() * ry)
        })
        .collect()
}

fn circle(c: Vec2, r: f32) -> Vec<Vec2> {
    arc(c, r, r, -PI / 2.0, -PI / 2.0 + TAU)
}

fn cubic(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2) -> Vec<Vec2> {
    (0..=24)
        .map(|i| {
            let u = i as f32 / 24.0;
            let v = 1.0 - u;
            p0 * v * v * v + p1 * 3.0 * v * v * u + p2 * 3.0 * v * u * u + p3 * u * u * u
        })
        .collect()
}

fn rect_loop(x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<Vec2> {
    vec![
        vec2(x0, y0),
        vec2(x1, y0),
        vec2(x1, y1),
        vec2(x0, y1),
        vec2(x0, y0),
    ]
}

/// Vertical zigzag from `a` to `b` (swings left/right), amplitude easing `amp0` → `amp1`.
fn zigzag(a: Vec2, b: Vec2, amp0: f32, amp1: f32, swings: usize) -> Vec<Vec2> {
    let mut pts = vec![a];
    for i in 1..=swings {
        let u = i as f32 / (swings + 1) as f32;
        let side = if i % 2 == 1 { -1.0 } else { 1.0 };
        let amp = amp0 + (amp1 - amp0) * u;
        let p = a.lerp(b, u);
        pts.push(vec2(p.x + side * amp, p.y));
    }
    pts.push(b);
    pts
}

/// Horizontal zigzag (swings up/down).
fn zigzag_h(a: Vec2, b: Vec2, amp0: f32, amp1: f32, swings: usize) -> Vec<Vec2> {
    let mut pts = vec![a];
    for i in 1..=swings {
        let u = i as f32 / (swings + 1) as f32;
        let side = if i % 2 == 1 { -1.0 } else { 1.0 };
        let amp = amp0 + (amp1 - amp0) * u;
        let p = a.lerp(b, u);
        pts.push(vec2(p.x, p.y + side * amp));
    }
    pts.push(b);
    pts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patterns_stay_in_the_cup() {
        for s in LatteShape::ALL {
            let strokes = s.strokes();
            assert!(!strokes.is_empty());
            for st in &strokes {
                assert!(st.len() >= 2, "{s:?}");
                for p in st {
                    assert!(p.length() < 0.95, "{s:?} point {p:?} outside the crema");
                }
            }
        }
    }

    #[test]
    fn guide_has_48_points() {
        for s in LatteShape::ALL {
            assert_eq!(s.guide_points(48).len(), 48, "{s:?}");
        }
    }

    #[test]
    fn every_pattern_has_marks() {
        for s in LatteShape::ALL {
            let m = s.marks();
            assert!(m.len() >= 3, "{s:?} has {} marks", m.len());
            assert!(m.len() <= 26, "{s:?} has {} marks", m.len());
        }
        // the rosetta's wobbles are its zigzag corners
        assert!(LatteShape::Rosetta.marks().len() >= 9);
    }

    #[test]
    fn ids_round_trip() {
        for s in LatteShape::ALL {
            assert_eq!(LatteShape::from_id(s.id()), Some(s));
        }
    }
}
