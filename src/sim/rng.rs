//! Small seeded RNG (SplitMix64). Deterministic across platforms, serialisable, no crates.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng {
            state: seed ^ 0x9E37_79B9_7F4A_7C15,
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[0, 1)`.
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// `true` with probability `p` (clamped to 0–1).
    pub fn chance(&mut self, p: f32) -> bool {
        p > 0.0 && self.f32() < p
    }

    /// Uniform integer in the inclusive range (order-insensitive).
    pub fn range_u32(&mut self, lo: u32, hi: u32) -> u32 {
        let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
        lo + (self.next_u64() % (hi - lo + 1) as u64) as u32
    }

    pub fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
        lo + (self.next_u64() % (hi - lo + 1) as u64) as i64
    }

    /// Uniform float in `[lo, hi)`.
    pub fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f32()
    }

    /// Uniform index `0..len` (`len` must be > 0).
    pub fn index(&mut self, len: usize) -> usize {
        (self.next_u64() % len.max(1) as u64) as usize
    }

    pub fn pick<'a, T>(&mut self, list: &'a [T]) -> Option<&'a T> {
        if list.is_empty() {
            None
        } else {
            Some(&list[self.index(list.len())])
        }
    }

    /// Weighted pick; returns the index or `None` if all weights are ≤ 0.
    pub fn weighted(&mut self, weights: &[f32]) -> Option<usize> {
        let total: f32 = weights.iter().filter(|w| **w > 0.0).sum();
        if total <= 0.0 {
            return None;
        }
        let mut roll = self.f32() * total;
        let mut last = None;
        for (i, w) in weights.iter().enumerate() {
            if *w <= 0.0 {
                continue;
            }
            last = Some(i);
            if roll < *w {
                return Some(i);
            }
            roll -= *w;
        }
        last
    }

    /// A derived generator (for look seeds etc.) that doesn't disturb this one much.
    pub fn fork(&mut self) -> u64 {
        self.next_u64()
    }
}

/// Stable 64-bit hash of a string (FNV-1a) — used for seeds derived from ids.
pub fn hash_str(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_numbers() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
        let mut c = Rng::new(43);
        assert_ne!(Rng::new(42).next_u64(), c.next_u64());
    }

    #[test]
    fn ranges_stay_in_bounds() {
        let mut r = Rng::new(7);
        for _ in 0..1000 {
            let v = r.range_u32(3, 5);
            assert!((3..=5).contains(&v));
            let f = r.f32();
            assert!((0.0..1.0).contains(&f));
        }
        assert_eq!(r.weighted(&[0.0, 0.0]), None);
        assert_eq!(r.weighted(&[0.0, 1.0]), Some(1));
    }
}
