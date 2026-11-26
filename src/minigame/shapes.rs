//! Latte-art target shapes as unions of simple primitives in a unit disc
//! (`(0,0)` = cup centre, radius 1 = crema edge, y down), plus helpers to
//! rasterise them into a scoring grid and into horizontal runs for drawing.

use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

/// Latte-art pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LatteShape {
    Heart,
    Rosetta,
    Tulip,
    /// Crescent moon with a little star.
    Moon,
    /// A tiny night train with a puff of steam.
    Train,
}

impl LatteShape {
    pub const ALL: [LatteShape; 5] = [
        LatteShape::Heart,
        LatteShape::Rosetta,
        LatteShape::Tulip,
        LatteShape::Moon,
        LatteShape::Train,
    ];

    pub fn id(self) -> &'static str {
        match self {
            LatteShape::Heart => "heart",
            LatteShape::Rosetta => "rosetta",
            LatteShape::Tulip => "tulip",
            LatteShape::Moon => "moon",
            LatteShape::Train => "train",
        }
    }

    pub fn from_id(id: &str) -> Option<LatteShape> {
        LatteShape::ALL.into_iter().find(|s| s.id() == id)
    }

    pub fn key(self) -> &'static str {
        match self {
            LatteShape::Heart => "minigame.shape.heart",
            LatteShape::Rosetta => "minigame.shape.rosetta",
            LatteShape::Tulip => "minigame.shape.tulip",
            LatteShape::Moon => "minigame.shape.moon",
            LatteShape::Train => "minigame.shape.train",
        }
    }

    /// Is the unit-disc point inside the pattern?
    pub fn contains(self, p: Vec2) -> bool {
        let (add, sub) = self.prims();
        add.iter().any(|q| q.contains(p)) && !sub.iter().any(|q| q.contains(p))
    }

    fn prims(self) -> (Vec<Prim>, Vec<Prim>) {
        use Prim::*;
        match self {
            LatteShape::Heart => (
                vec![
                    Circle(vec2(-0.27, -0.17), 0.32),
                    Circle(vec2(0.27, -0.17), 0.32),
                    Tri(vec2(-0.575, -0.08), vec2(0.575, -0.08), vec2(0.0, 0.6)),
                ],
                vec![],
            ),
            LatteShape::Rosetta => (
                vec![
                    Ellipse(vec2(0.0, 0.42), 0.56, 0.12),
                    Ellipse(vec2(0.0, 0.19), 0.5, 0.12),
                    Ellipse(vec2(0.0, -0.03), 0.43, 0.115),
                    Ellipse(vec2(0.0, -0.24), 0.34, 0.105),
                    Ellipse(vec2(0.0, -0.43), 0.24, 0.095),
                    Circle(vec2(0.0, -0.6), 0.1),
                    Capsule(vec2(0.0, 0.66), vec2(0.0, -0.58), 0.04),
                ],
                vec![],
            ),
            LatteShape::Tulip => (
                vec![
                    Ellipse(vec2(0.0, 0.33), 0.44, 0.19),
                    Ellipse(vec2(0.0, 0.03), 0.37, 0.17),
                    Circle(vec2(-0.13, -0.3), 0.18),
                    Circle(vec2(0.13, -0.3), 0.18),
                    Tri(vec2(-0.3, -0.25), vec2(0.3, -0.25), vec2(0.0, -0.02)),
                    Capsule(vec2(0.0, 0.66), vec2(0.0, 0.4), 0.045),
                ],
                vec![],
            ),
            LatteShape::Moon => (
                vec![
                    Circle(vec2(-0.08, 0.04), 0.56),
                    Tri(vec2(0.47, -0.62), vec2(0.53, -0.44), vec2(0.41, -0.44)),
                    Tri(vec2(0.47, -0.3), vec2(0.53, -0.44), vec2(0.41, -0.44)),
                    Tri(vec2(0.33, -0.44), vec2(0.47, -0.49), vec2(0.47, -0.39)),
                    Tri(vec2(0.61, -0.44), vec2(0.47, -0.49), vec2(0.47, -0.39)),
                ],
                vec![Circle(vec2(0.2, -0.12), 0.46)],
            ),
            LatteShape::Train => (
                vec![
                    RRect(vec2(0.06, 0.07), vec2(0.44, 0.17), 0.08),
                    RRect(vec2(-0.27, -0.17), vec2(0.15, 0.15), 0.04),
                    RRect(vec2(-0.27, -0.33), vec2(0.21, 0.04), 0.03),
                    RRect(vec2(0.3, -0.2), vec2(0.065, 0.11), 0.03),
                    Circle(vec2(-0.24, 0.31), 0.12),
                    Circle(vec2(0.07, 0.31), 0.12),
                    Circle(vec2(0.36, 0.31), 0.12),
                    Circle(vec2(0.4, -0.45), 0.08),
                    Circle(vec2(0.55, -0.6), 0.1),
                ],
                vec![],
            ),
        }
    }

    /// `n × n` grid over the square `[-1,1]²` (row-major), each cell 0..1 coverage
    /// (2×2 supersampled), cells outside the unit disc are 0.
    pub fn rasterize(self, n: usize) -> Vec<f32> {
        let (add, sub) = self.prims();
        let inside = |p: Vec2| add.iter().any(|q| q.contains(p)) && !sub.iter().any(|q| q.contains(p));
        let mut out = vec![0.0; n * n];
        for j in 0..n {
            for i in 0..n {
                let c = cell_center(i, j, n);
                if c.length() > 1.0 {
                    continue;
                }
                let h = 0.5 / n as f32;
                let mut k = 0.0;
                for (dx, dy) in [(-h, -h), (h, -h), (-h, h), (h, h)] {
                    if inside(c + vec2(dx, dy)) {
                        k += 0.25;
                    }
                }
                out[j * n + i] = k;
            }
        }
        out
    }

    /// Horizontal runs `(y0, y1, x0, x1)` covering the shape (unit-disc coords),
    /// `rows` scanlines over `[-1,1]`. Runs never overlap, so a translucent fill is even.
    pub fn runs(self, rows: usize) -> Vec<[f32; 4]> {
        let (add, sub) = self.prims();
        let inside = |p: Vec2| add.iter().any(|q| q.contains(p)) && !sub.iter().any(|q| q.contains(p));
        let mut out = Vec::new();
        let cols = rows * 2;
        let dy = 2.0 / rows as f32;
        let dx = 2.0 / cols as f32;
        for j in 0..rows {
            let y0 = -1.0 + j as f32 * dy;
            let y = y0 + dy * 0.5;
            let mut start: Option<f32> = None;
            let mut prev_x = -1.0;
            for i in 0..=cols {
                let x = -1.0 + i as f32 * dx;
                let on = i < cols && inside(vec2(x + dx * 0.5, y));
                match (on, start) {
                    (true, None) => {
                        // refine the left edge
                        start = Some(refine(&inside, y, x - dx * 0.5, x + dx * 0.5, true));
                    }
                    (false, Some(s)) => {
                        let e = refine(&inside, y, prev_x + dx * 0.5, x + dx * 0.5, false);
                        out.push([y0, y0 + dy, s, e]);
                        start = None;
                    }
                    _ => {}
                }
                prev_x = x;
            }
        }
        out
    }
}

/// Bisection for the edge between `a` and `b` on scanline `y`.
fn refine(inside: &impl Fn(Vec2) -> bool, y: f32, mut a: f32, mut b: f32, rising: bool) -> f32 {
    for _ in 0..6 {
        let m = (a + b) * 0.5;
        let on = inside(vec2(m, y));
        if on == rising {
            b = m;
        } else {
            a = m;
        }
    }
    (a + b) * 0.5
}

/// Centre of grid cell `(i, j)` in unit-disc coordinates.
pub fn cell_center(i: usize, j: usize, n: usize) -> Vec2 {
    vec2(
        -1.0 + (i as f32 + 0.5) * 2.0 / n as f32,
        -1.0 + (j as f32 + 0.5) * 2.0 / n as f32,
    )
}

#[derive(Clone, Copy, Debug)]
enum Prim {
    Circle(Vec2, f32),
    Ellipse(Vec2, f32, f32),
    Capsule(Vec2, Vec2, f32),
    Tri(Vec2, Vec2, Vec2),
    /// centre, half size, corner radius
    RRect(Vec2, Vec2, f32),
}

impl Prim {
    fn contains(&self, p: Vec2) -> bool {
        match *self {
            Prim::Circle(c, r) => p.distance_squared(c) <= r * r,
            Prim::Ellipse(c, rx, ry) => {
                let d = (p - c) / vec2(rx, ry);
                d.length_squared() <= 1.0
            }
            Prim::Capsule(a, b, r) => {
                let ab = b - a;
                let t = ((p - a).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
                p.distance_squared(a + ab * t) <= r * r
            }
            Prim::Tri(a, b, c) => {
                let s = |p: Vec2, q: Vec2, r: Vec2| (q - p).perp_dot(r - p);
                let d1 = s(a, b, p);
                let d2 = s(b, c, p);
                let d3 = s(c, a, p);
                let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
                let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
                !(neg && pos)
            }
            Prim::RRect(c, h, r) => {
                let q = (p - c).abs() - h + Vec2::splat(r);
                q.max(Vec2::ZERO).length() + q.x.max(q.y).min(0.0) - r <= 0.0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_have_reasonable_area() {
        for s in LatteShape::ALL {
            let g = s.rasterize(48);
            let inside: f32 = g.iter().sum();
            let disc = std::f32::consts::PI * 24.0 * 24.0;
            let frac = inside / disc;
            assert!((0.12..0.6).contains(&frac), "{s:?} covers {frac}");
            assert!(g.iter().all(|v| (0.0..=1.0).contains(v)));
        }
    }

    #[test]
    fn moon_is_a_crescent() {
        assert!(LatteShape::Moon.contains(vec2(-0.5, 0.1)));
        assert!(!LatteShape::Moon.contains(vec2(0.15, -0.1)));
        assert!(LatteShape::Heart.contains(vec2(0.0, 0.2)));
        assert!(!LatteShape::Heart.contains(vec2(0.0, 0.8)));
    }

    #[test]
    fn runs_do_not_overlap() {
        let runs = LatteShape::Train.runs(60);
        assert!(!runs.is_empty());
        for w in runs.windows(2) {
            if w[0][0] == w[1][0] {
                assert!(w[0][3] <= w[1][2]);
            }
        }
    }

    #[test]
    fn ids_round_trip() {
        for s in LatteShape::ALL {
            assert_eq!(LatteShape::from_id(s.id()), Some(s));
        }
    }
}
