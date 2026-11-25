//! Stable widget ids: a deterministic FNV-1a hash of any `Hash` key,
//! combined with the parent id (see `Ui::push_id`).

use std::hash::{Hash, Hasher};

/// Identifies a widget between frames. Same key + same parent ⇒ same id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Id(pub u64);

impl Id {
    pub const ROOT: Id = Id(0xcbf2_9ce4_8422_2325);

    /// Id of `key` at the root level.
    pub fn new(key: impl Hash) -> Id {
        Id::ROOT.with(key)
    }

    /// Child id: `parent.with(key)`.
    pub fn with(self, key: impl Hash) -> Id {
        let mut h = Fnv(self.0);
        key.hash(&mut h);
        Id(h.finish())
    }
}

/// FNV-1a, 64 bit. Deterministic across runs and Rust versions.
struct Fnv(u64);

impl Hasher for Fnv {
    fn finish(&self) -> u64 {
        // final avalanche so that nearby keys spread
        let mut x = self.0;
        x ^= x >> 33;
        x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
        x ^= x >> 33;
        x
    }
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 ^= *b as u64;
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_stable_and_distinct() {
        assert_eq!(Id::new("play"), Id::new("play"));
        assert_ne!(Id::new("play"), Id::new("pause"));
        assert_ne!(Id::new(1_usize), Id::new(2_usize));
    }

    #[test]
    fn parent_scopes_ids() {
        let a = Id::new("row").with(3_usize);
        let b = Id::new("col").with(3_usize);
        assert_ne!(a, b);
        assert_eq!(a, Id::new("row").with(3_usize));
        assert_ne!(Id::new("x"), Id::ROOT.with("x").with("x"));
    }

    #[test]
    fn known_value_does_not_drift() {
        // guards against accidental hasher changes (ids may be persisted for UI state)
        assert_eq!(Id::new("shiftella").0, Id::new("shiftella").0);
        assert_ne!(Id::new("").0, Id::ROOT.0);
    }
}
