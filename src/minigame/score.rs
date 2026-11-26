//! Pure, deterministic scoring for the brewing mini-games.
//!
//! Every game measures a *performance* `p` in 0..1 and the shared mapping
//! [`quality_from_perf`] turns it into the quality the sim sees (0.55..1.0).
//! Nothing here is punishing: an empty attempt still yields 0.55, skipping yields
//! [`SKIP_QUALITY`], and the `relaxed` flag only ever widens tolerances.

/// Quality reported when the player skips a mini-game.
pub const SKIP_QUALITY: f32 = 0.7;
/// Lowest quality a played mini-game can return.
pub const MIN_QUALITY: f32 = 0.55;
/// Highest quality.
pub const MAX_QUALITY: f32 = 1.0;

/// Maps performance 0..1 onto the quality range 0.55..1.0 (eased so "decent" feels good).
pub fn quality_from_perf(perf: f32) -> f32 {
    let p = if perf.is_finite() {
        perf.clamp(0.0, 1.0)
    } else {
        0.0
    };
    // Slight ease-out: the upper half of the range is reached a little earlier.
    let eased = 1.0 - (1.0 - p) * (1.0 - p) * 0.35 - (1.0 - p) * 0.65;
    MIN_QUALITY + (MAX_QUALITY - MIN_QUALITY) * eased.clamp(0.0, 1.0)
}

/// Tolerance multiplier for the relaxed setting.
pub fn widen(relaxed: bool) -> f32 {
    if relaxed { 1.6 } else { 1.0 }
}

/// Smooth falloff: 1 inside `inner`, easing to 0 at `inner + outer`.
fn falloff(d: f32, inner: f32, outer: f32) -> f32 {
    let d = d.abs();
    if d <= inner {
        return 1.0;
    }
    let x = ((d - inner) / outer.max(1e-4)).clamp(0.0, 1.0);
    let s = 1.0 - x;
    s * s * (3.0 - 2.0 * s)
}

/// How the result reads to the player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rating {
    /// Still nice — never a failure word.
    Cosy,
    Nice,
    Lovely,
    Perfect,
}

impl Rating {
    pub fn from_quality(q: f32) -> Rating {
        if q >= 0.94 {
            Rating::Perfect
        } else if q >= 0.84 {
            Rating::Lovely
        } else if q >= 0.7 {
            Rating::Nice
        } else {
            Rating::Cosy
        }
    }
    /// Filled stars (1..=3) for the result chip.
    pub fn stars(self) -> u32 {
        match self {
            Rating::Cosy | Rating::Nice => 1,
            Rating::Lovely => 2,
            Rating::Perfect => 3,
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Rating::Cosy => "minigame.result.cosy",
            Rating::Nice => "minigame.result.nice",
            Rating::Lovely => "minigame.result.lovely",
            Rating::Perfect => "minigame.result.perfect",
        }
    }
}

// --- MilkFroth --------------------------------------------------------------

/// Froth: final foam `level` against the sweet-spot band (`center ± half`).
pub fn froth_perf(level: f32, center: f32, half: f32, relaxed: bool) -> f32 {
    let w = widen(relaxed);
    let d = (level - center).abs();
    let half = half * w;
    if d <= half {
        // Inside the band: 1.0 in the middle, 0.9 at its edge.
        1.0 - 0.1 * (d / half.max(1e-4))
    } else {
        0.9 * falloff(d - half, 0.0, 0.22 * w)
    }
}

// --- LatteArt ---------------------------------------------------------------

/// Soft intersection-over-union of two equally sized 0..1 grids.
pub fn soft_iou(milk: &[f32], target: &[f32]) -> f32 {
    let mut inter = 0.0;
    let mut union = 0.0;
    for (m, t) in milk.iter().zip(target) {
        let m = m.clamp(0.0, 1.0);
        let t = t.clamp(0.0, 1.0);
        inter += m.min(t);
        union += m.max(t);
    }
    if union <= 1e-4 { 0.0 } else { inter / union }
}

/// Latte art: similarity (soft IoU) → performance. A hand-drawn shape that clearly
/// reads as the target lands around 0.5–0.6 IoU, so that already counts as great.
pub fn latte_perf(iou: f32, relaxed: bool) -> f32 {
    let full = if relaxed { 0.42 } else { 0.58 };
    let x = (iou / full).clamp(0.0, 1.0);
    // ease-out so the first strokes matter most
    1.0 - (1.0 - x) * (1.0 - x)
}

// --- PourOver ---------------------------------------------------------------

/// One pour sample: radial distance from the ring (in ring radii) and the angular
/// distance to the guide light (radians). Returns 0..1.
pub fn pour_sample(radial_err: f32, angle_err: f32, relaxed: bool) -> f32 {
    let w = widen(relaxed);
    let ring = falloff(radial_err, 0.12 * w, 0.45 * w);
    let a = angle_err.abs().min(std::f32::consts::PI);
    let follow = falloff(a, 0.55 * w, 1.6 * w);
    ring * (0.45 + 0.55 * follow)
}

/// Mean of pour samples weighted by smoothness: `jerk` is the mean absolute change of
/// the pour point's angular speed (rad/s per sample), 0 = perfectly smooth.
pub fn pour_perf(mean_sample: f32, jerk: f32, relaxed: bool) -> f32 {
    let smooth = falloff(jerk, 0.6 * widen(relaxed), 2.5 * widen(relaxed));
    (mean_sample.clamp(0.0, 1.0) * (0.8 + 0.2 * smooth)).clamp(0.0, 1.0)
}

// --- TeaSteep ---------------------------------------------------------------

/// Tea: lift time vs. the target steep time, in seconds.
pub fn tea_perf(lift_t: f32, target_t: f32, relaxed: bool) -> f32 {
    let w = widen(relaxed);
    falloff(lift_t - target_t, 0.6 * w, 4.0 * w)
}

// --- Plate ------------------------------------------------------------------

/// Plate: `dists` = distance of each piece from its soft target (stage units),
/// `overlaps` = number of piece pairs that overlap untidily, `off_plate` = pieces
/// left outside the plate rim.
pub fn plate_perf(dists: &[f32], overlaps: usize, off_plate: usize, relaxed: bool) -> f32 {
    if dists.is_empty() {
        return 0.0;
    }
    let w = widen(relaxed);
    let place: f32 =
        dists.iter().map(|d| falloff(*d, 10.0 * w, 46.0 * w)).sum::<f32>() / dists.len() as f32;
    let tidy = 1.0 - 0.12 * overlaps as f32 / w - 0.25 * off_plate as f32 / w;
    (place * tidy.clamp(0.0, 1.0)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn in_range(q: f32) -> bool {
        (MIN_QUALITY..=MAX_QUALITY).contains(&q)
    }

    #[test]
    fn quality_mapping_is_bounded_and_monotonic() {
        assert_eq!(quality_from_perf(0.0), MIN_QUALITY);
        assert_eq!(quality_from_perf(1.0), MAX_QUALITY);
        assert_eq!(quality_from_perf(-3.0), MIN_QUALITY);
        assert_eq!(quality_from_perf(7.0), MAX_QUALITY);
        assert_eq!(quality_from_perf(f32::NAN), MIN_QUALITY);
        let mut prev = 0.0;
        for i in 0..=100 {
            let q = quality_from_perf(i as f32 / 100.0);
            assert!(in_range(q));
            assert!(q >= prev);
            prev = q;
        }
        // "decent" (0.5) already feels good
        assert!(quality_from_perf(0.5) > 0.8);
    }

    #[test]
    fn skip_is_seven_tenths() {
        assert_eq!(SKIP_QUALITY, 0.7);
        assert!(in_range(SKIP_QUALITY));
    }

    #[test]
    fn froth_scores() {
        assert_eq!(froth_perf(0.7, 0.7, 0.07, false), 1.0);
        assert!(froth_perf(0.76, 0.7, 0.07, false) >= 0.9);
        assert!(froth_perf(0.95, 0.7, 0.07, false) < 0.5);
        assert!(froth_perf(0.1, 0.7, 0.07, false) >= 0.0);
        // relaxed widens
        for l in [0.5, 0.6, 0.8, 0.85, 0.9] {
            assert!(froth_perf(l, 0.7, 0.07, true) >= froth_perf(l, 0.7, 0.07, false));
        }
        assert!(froth_perf(0.8, 0.7, 0.07, true) > froth_perf(0.8, 0.7, 0.07, false));
    }

    #[test]
    fn latte_scores() {
        let t = [1.0, 1.0, 0.0, 0.0];
        assert_eq!(soft_iou(&t, &t), 1.0);
        assert_eq!(soft_iou(&[0.0; 4], &t), 0.0);
        assert_eq!(soft_iou(&[0.0; 4], &[0.0; 4]), 0.0);
        assert!((soft_iou(&[1.0, 0.0, 0.0, 0.0], &t) - 0.5).abs() < 1e-6);
        assert_eq!(latte_perf(0.0, false), 0.0);
        assert_eq!(latte_perf(0.9, false), 1.0);
        assert!(latte_perf(0.3, true) > latte_perf(0.3, false));
    }

    #[test]
    fn pour_scores() {
        assert_eq!(pour_sample(0.0, 0.0, false), 1.0);
        assert!(pour_sample(2.0, 0.0, false) < 0.01);
        assert!(pour_sample(0.3, 1.0, true) > pour_sample(0.3, 1.0, false));
        assert!(pour_perf(1.0, 0.0, false) > 0.99);
        assert!(pour_perf(1.0, 10.0, false) >= 0.8);
        assert!(pour_perf(0.7, 1.2, true) >= pour_perf(0.7, 1.2, false));
    }

    #[test]
    fn tea_scores() {
        assert_eq!(tea_perf(9.0, 9.0, false), 1.0);
        assert!(tea_perf(11.0, 9.0, false) < 1.0);
        assert!(tea_perf(30.0, 9.0, false) == 0.0);
        assert!(tea_perf(11.0, 9.0, true) > tea_perf(11.0, 9.0, false));
    }

    #[test]
    fn plate_scores() {
        assert_eq!(plate_perf(&[0.0, 0.0, 0.0], 0, 0, false), 1.0);
        assert_eq!(plate_perf(&[], 0, 0, false), 0.0);
        assert!(plate_perf(&[30.0, 5.0], 1, 0, false) < 1.0);
        assert!(plate_perf(&[30.0, 5.0], 1, 0, true) > plate_perf(&[30.0, 5.0], 1, 0, false));
        assert!(plate_perf(&[500.0; 4], 6, 4, false) >= 0.0);
    }

    #[test]
    fn deterministic() {
        for i in 0..50 {
            let x = i as f32 * 0.037;
            assert_eq!(froth_perf(x, 0.7, 0.07, false), froth_perf(x, 0.7, 0.07, false));
            assert_eq!(pour_sample(x, x * 2.0, true), pour_sample(x, x * 2.0, true));
        }
    }

    #[test]
    fn ratings() {
        assert_eq!(Rating::from_quality(SKIP_QUALITY), Rating::Nice);
        assert_eq!(Rating::from_quality(0.55), Rating::Cosy);
        assert_eq!(Rating::from_quality(1.0), Rating::Perfect);
        assert!(Rating::from_quality(0.9).stars() >= 2);
    }
}
