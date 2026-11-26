//! Pure, deterministic scoring (`design/handson.md` §2). Every game computes a raw
//! quality 0..1 with the formula of its section; [`finalize`] clamps it and applies the
//! Relaxed floor. Relaxed tolerances are ×[`RELAXED_WIDEN`].

use macroquad::prelude::Vec2;

/// Quality reported when the player skips a mini-game.
pub const SKIP_QUALITY: f32 = 0.7;
/// Relaxed setting: lowest quality.
pub const RELAXED_FLOOR: f32 = 0.5;
/// Relaxed setting: tolerance windows are this much wider.
pub const RELAXED_WIDEN: f32 = 1.6;

/// Tolerance multiplier.
pub fn widen(relaxed: bool) -> f32 {
    if relaxed { RELAXED_WIDEN } else { 1.0 }
}

fn clamp01(x: f32) -> f32 {
    if x.is_finite() {
        x.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Final quality: clamped to 0..1, Relaxed never below 0.5.
pub fn finalize(q: f32, relaxed: bool) -> f32 {
    let q = clamp01(q);
    if relaxed { q.max(RELAXED_FLOOR) } else { q }
}

/// Quality label (never negative, §2.0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rating {
    Homely,
    Good,
    Lovely,
    Beautiful,
}

impl Rating {
    pub fn from_quality(q: f32) -> Rating {
        if q >= 0.9 {
            Rating::Beautiful
        } else if q >= 0.7 {
            Rating::Lovely
        } else if q >= 0.4 {
            Rating::Good
        } else {
            Rating::Homely
        }
    }
    /// Filled stars (1..=3) for the toast.
    pub fn stars(self) -> u32 {
        match self {
            Rating::Homely | Rating::Good => 1,
            Rating::Lovely => 2,
            Rating::Beautiful => 3,
        }
    }
    /// `minigame.quality.<tier>`
    pub fn key(self) -> &'static str {
        match self {
            Rating::Homely => "minigame.quality.homely",
            Rating::Good => "minigame.quality.good",
            Rating::Lovely => "minigame.quality.lovely",
            Rating::Beautiful => "minigame.quality.beautiful",
        }
    }
}

// --- §2.1 MilkFroth -----------------------------------------------------------

/// `0.75 × clamp(1 − |T − T_target| / tol) + 0.25 × clamp(1 − |foam − foam_target| / 0.3)`.
pub fn froth(
    temp: f32,
    t_target: f32,
    tol: f32,
    foam: f32,
    foam_target: f32,
    relaxed: bool,
) -> f32 {
    let w = widen(relaxed);
    0.75 * clamp01(1.0 - (temp - t_target).abs() / (tol * w))
        + 0.25 * clamp01(1.0 - (foam - foam_target).abs() / (0.3 * w))
}

// --- §2.2 LatteArt ------------------------------------------------------------

/// Share of `guide` points with a trace sample within `radius`.
pub fn coverage(guide: &[Vec2], trace: &[Vec2], radius: f32) -> f32 {
    if guide.is_empty() {
        return 0.0;
    }
    let r2 = radius * radius;
    let hit = guide
        .iter()
        .filter(|g| trace.iter().any(|t| t.distance_squared(**g) <= r2))
        .count();
    hit as f32 / guide.len() as f32
}

/// Distance from `p` to a polyline.
pub fn dist_to_polyline(p: Vec2, line: &[Vec2]) -> f32 {
    match line.len() {
        0 => f32::INFINITY,
        1 => p.distance(line[0]),
        _ => line
            .windows(2)
            .map(|w| {
                let ab = w[1] - w[0];
                let t = ((p - w[0]).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
                p.distance(w[0] + ab * t)
            })
            .fold(f32::INFINITY, f32::min),
    }
}

/// Share of trace samples farther than `far` from every stroke (no trace = 1).
pub fn stray(strokes: &[Vec<Vec2>], trace: &[Vec2], far: f32) -> f32 {
    if trace.is_empty() {
        return 1.0;
    }
    let n = trace
        .iter()
        .filter(|t| {
            strokes
                .iter()
                .map(|s| dist_to_polyline(**t, s))
                .fold(f32::INFINITY, f32::min)
                > far
        })
        .count();
    n as f32 / trace.len() as f32
}

/// Mouse mode: `0.6 × coverage + 0.4 × (1 − stray)`.
pub fn latte_trace(coverage: f32, stray: f32) -> f32 {
    0.6 * clamp01(coverage) + 0.4 * (1.0 - clamp01(stray))
}

/// Keyboard mode: `mean(1 − |Δt| / 0.35)` over the wobble taps (a missed tap counts 0).
pub fn latte_taps(deltas: &[Option<f32>], relaxed: bool) -> f32 {
    if deltas.is_empty() {
        return 0.0;
    }
    let w = 0.35 * widen(relaxed);
    deltas
        .iter()
        .map(|d| d.map_or(0.0, |d| clamp01(1.0 - d.abs() / w)))
        .sum::<f32>()
        / deltas.len() as f32
}

// --- §2.3 PourOver ------------------------------------------------------------

/// `0.5 × rhythm + 0.3 × centred + 0.2 × bloom`, each 0..1.
pub fn pour(rhythm: f32, centred: f32, bloom: f32) -> f32 {
    0.5 * clamp01(rhythm) + 0.3 * clamp01(centred) + 0.2 * clamp01(bloom)
}

/// Is an angular speed (rev/s) inside the calm band 0.6–1.2 (widened when relaxed)?
pub fn in_rhythm(rev_per_s: f32, relaxed: bool) -> bool {
    let w = widen(relaxed);
    let (lo, hi) = (0.9 - 0.3 * w, 0.9 + 0.3 * w);
    (lo..=hi).contains(&rev_per_s.abs())
}

/// Beat tap within ±0.2 s (widened when relaxed)?
pub fn on_beat(dt: f32, relaxed: bool) -> bool {
    dt.abs() <= 0.2 * widen(relaxed)
}

// --- §2.4 TeaSteep ------------------------------------------------------------

/// `clamp(1 − |strength − target| / tol) × addons`.
pub fn tea(strength: f32, target: f32, tol: f32, addons: f32, relaxed: bool) -> f32 {
    clamp01(1.0 - (strength - target).abs() / (tol * widen(relaxed))) * clamp01(addons)
}

/// Lemon drop accuracy, ±25 px.
pub fn lemon(dx: f32, relaxed: bool) -> f32 {
    clamp01(1.0 - dx.abs() / (25.0 * widen(relaxed)))
}

/// Sugar: exact = 1, ±1 = 0.8, further = 0.5.
pub fn sugar(count: u8, target: u8) -> f32 {
    match count.abs_diff(target) {
        0 => 1.0,
        1 => 0.8,
        _ => 0.5,
    }
}

// --- §2.5 Plate ---------------------------------------------------------------

/// Greedy nearest-free-slot scores `1 − dist / 40 px` per placed piece (`None` = not placed → 0).
pub fn plate_slots(pieces: &[Option<Vec2>], slots: &[Vec2], relaxed: bool) -> Vec<f32> {
    let w = widen(relaxed);
    let mut free: Vec<bool> = vec![true; slots.len()];
    // pair the closest piece/slot pairs first
    let mut pairs: Vec<(f32, usize, usize)> = Vec::new();
    for (i, p) in pieces.iter().enumerate() {
        if let Some(p) = p {
            for (j, s) in slots.iter().enumerate() {
                pairs.push((p.distance(*s), i, j));
            }
        }
    }
    pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out = vec![0.0; pieces.len()];
    let mut done = vec![false; pieces.len()];
    for (d, i, j) in pairs {
        if done[i] || !free[j] {
            continue;
        }
        done[i] = true;
        free[j] = false;
        out[i] = clamp01(1.0 - d / (40.0 * w));
    }
    out
}

/// `0.7 × mean(slot_score) + 0.3 × garnish` (placed / needed, extras free).
pub fn plate(slot_scores: &[f32], garnish_placed: usize, garnish_needed: usize) -> f32 {
    let slots = if slot_scores.is_empty() {
        0.0
    } else {
        slot_scores.iter().sum::<f32>() / slot_scores.len() as f32
    };
    let g = if garnish_needed == 0 {
        1.0
    } else {
        (garnish_placed as f32 / garnish_needed as f32).min(1.0)
    };
    0.7 * slots + 0.3 * g
}

#[cfg(test)]
mod tests {
    use super::*;
    use macroquad::prelude::vec2;

    #[test]
    fn finalize_and_floor() {
        assert_eq!(finalize(-1.0, false), 0.0);
        assert_eq!(finalize(2.0, false), 1.0);
        assert_eq!(finalize(f32::NAN, false), 0.0);
        assert_eq!(finalize(0.1, true), 0.5);
        assert_eq!(finalize(0.8, true), 0.8);
        assert_eq!(SKIP_QUALITY, 0.7);
    }

    #[test]
    fn labels() {
        assert_eq!(Rating::from_quality(0.95), Rating::Beautiful);
        assert_eq!(Rating::from_quality(0.7), Rating::Lovely);
        assert_eq!(Rating::from_quality(SKIP_QUALITY), Rating::Lovely);
        assert_eq!(Rating::from_quality(0.4), Rating::Good);
        assert_eq!(Rating::from_quality(0.1), Rating::Homely);
    }

    #[test]
    fn froth_formula() {
        assert_eq!(froth(62.0, 62.0, 4.0, 0.6, 0.6, false), 1.0);
        assert!((froth(64.0, 62.0, 4.0, 0.6, 0.6, false) - (0.75 * 0.5 + 0.25)).abs() < 1e-5);
        assert_eq!(froth(90.0, 62.0, 4.0, 1.0, 0.3, false), 0.0);
        assert!(froth(65.0, 62.0, 4.0, 0.8, 0.6, true) > froth(65.0, 62.0, 4.0, 0.8, 0.6, false));
    }

    #[test]
    fn latte_formulas() {
        let guide = [
            vec2(0.0, 0.0),
            vec2(10.0, 0.0),
            vec2(20.0, 0.0),
            vec2(30.0, 0.0),
        ];
        let trace = [vec2(0.0, 3.0), vec2(10.0, 3.0)];
        assert_eq!(coverage(&guide, &trace, 6.0), 0.5);
        let strokes = vec![guide.to_vec()];
        assert_eq!(stray(&strokes, &trace, 24.0), 0.0);
        assert_eq!(stray(&strokes, &[vec2(0.0, 100.0)], 24.0), 1.0);
        assert_eq!(stray(&strokes, &[], 24.0), 1.0);
        assert!((latte_trace(1.0, 0.0) - 1.0).abs() < 1e-6);
        assert!((latte_trace(0.0, 1.0)).abs() < 1e-6);
        assert_eq!(latte_taps(&[Some(0.0), Some(0.0)], false), 1.0);
        assert_eq!(latte_taps(&[Some(0.0), None], false), 0.5);
        assert!(latte_taps(&[Some(0.3)], true) > latte_taps(&[Some(0.3)], false));
        assert!((dist_to_polyline(vec2(5.0, 4.0), &guide) - 4.0).abs() < 1e-5);
    }

    #[test]
    fn pour_formula() {
        assert_eq!(pour(1.0, 1.0, 1.0), 1.0);
        assert!((pour(0.0, 1.0, 1.0) - 0.5).abs() < 1e-6);
        assert!(in_rhythm(0.8, false));
        assert!(!in_rhythm(2.0, false));
        assert!(!in_rhythm(0.5, false) && in_rhythm(0.5, true));
        assert!(on_beat(0.15, false) && !on_beat(0.25, false) && on_beat(0.25, true));
    }

    #[test]
    fn tea_formula() {
        assert_eq!(tea(3.0, 3.0, 1.5, 1.0, false), 1.0);
        assert!((tea(3.75, 3.0, 1.5, 1.0, false) - 0.5).abs() < 1e-5);
        assert!((tea(3.0, 3.0, 1.5, 0.8, false) - 0.8).abs() < 1e-6);
        assert!(tea(4.0, 3.0, 1.5, 1.0, true) > tea(4.0, 3.0, 1.5, 1.0, false));
        assert_eq!(lemon(0.0, false), 1.0);
        assert_eq!(lemon(30.0, false), 0.0);
        assert_eq!((sugar(2, 2), sugar(3, 2), sugar(0, 2)), (1.0, 0.8, 0.5));
    }

    #[test]
    fn plate_formula() {
        let slots = [vec2(0.0, 0.0), vec2(100.0, 0.0)];
        let s = plate_slots(
            &[Some(vec2(100.0, 0.0)), Some(vec2(20.0, 0.0))],
            &slots,
            false,
        );
        assert_eq!(s, vec![1.0, 0.5]);
        // both near the same slot: the second gets the far one
        let s = plate_slots(&[Some(vec2(0.0, 0.0)), Some(vec2(1.0, 0.0))], &slots, false);
        assert_eq!(s[0], 1.0);
        assert_eq!(s[1], 0.0);
        assert_eq!(plate_slots(&[None], &slots, false), vec![0.0]);
        assert!((plate(&[1.0, 1.0], 2, 2) - 1.0).abs() < 1e-6);
        assert!((plate(&[1.0], 0, 2) - 0.7).abs() < 1e-6);
        assert!((plate(&[0.5], 3, 2) - (0.35 + 0.3)).abs() < 1e-6);
        let a = plate_slots(&[Some(vec2(30.0, 0.0))], &slots, false)[0];
        let b = plate_slots(&[Some(vec2(30.0, 0.0))], &slots, true)[0];
        assert!(b > a);
    }

    #[test]
    fn deterministic() {
        for i in 0..50 {
            let x = i as f32 * 0.37;
            assert_eq!(
                froth(60.0 + x, 62.0, 4.0, 0.5, 0.6, false),
                froth(60.0 + x, 62.0, 4.0, 0.5, 0.6, false)
            );
        }
    }
}
