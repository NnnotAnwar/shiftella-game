//! The game clock. The café is open from 22:00 to 06:00.
//!
//! Time is counted in simulation "ticks": 10 ticks = 1 game minute.
//! At ×1 speed a tick happens every 0.1 seconds,
//! so 1 game minute = 1 real second and the whole night ≈ 8 minutes.

pub const TICKS_PER_MINUTE: u32 = 10;
pub const OPEN_AT_MINUTES: u32 = 22 * 60; // 22:00
pub const NIGHT_MINUTES: u32 = 8 * 60; // until 06:00

#[derive(Debug, Clone, Default)]
pub struct Clock {
    ticks: u32,
}

impl Clock {
    pub fn new() -> Self {
        Self { ticks: 0 }
    }

    /// One simulation step.
    pub fn advance(&mut self) {
        self.ticks += 1;
    }

    /// Did a new minute just begin? (Handy for doing something once a minute.)
    pub fn is_minute_start(&self) -> bool {
        self.ticks.is_multiple_of(TICKS_PER_MINUTE)
    }

    pub fn minutes_since_open(&self) -> u32 {
        self.ticks / TICKS_PER_MINUTE
    }

    /// Current time of day: (hours, minutes).
    pub fn hour_minute(&self) -> (u32, u32) {
        let total = (OPEN_AT_MINUTES + self.minutes_since_open()) % (24 * 60);
        (total / 60, total % 60)
    }

    /// A string like "03:33".
    pub fn label(&self) -> String {
        let (h, m) = self.hour_minute();
        format!("{h:02}:{m:02}")
    }

    /// How much of the night has passed: 0.0 to 1.0.
    pub fn progress(&self) -> f32 {
        (self.minutes_since_open() as f32 / NIGHT_MINUTES as f32).min(1.0)
    }

    pub fn is_night_over(&self) -> bool {
        self.minutes_since_open() >= NIGHT_MINUTES
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clock_at(minutes: u32) -> Clock {
        let mut c = Clock::new();
        for _ in 0..minutes * TICKS_PER_MINUTE {
            c.advance();
        }
        c
    }

    #[test]
    fn opens_at_22() {
        assert_eq!(Clock::new().label(), "22:00");
    }

    #[test]
    fn wraps_past_midnight() {
        assert_eq!(clock_at(120).label(), "00:00");
        assert_eq!(clock_at(5 * 60 + 33).label(), "03:33");
    }

    #[test]
    fn night_ends_at_6() {
        assert!(!clock_at(NIGHT_MINUTES - 1).is_night_over());
        let end = clock_at(NIGHT_MINUTES);
        assert!(end.is_night_over());
        assert_eq!(end.label(), "06:00");
    }
}
