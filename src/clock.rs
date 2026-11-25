//! Игровые часы. Кафе работает с 22:00 до 06:00.
//!
//! Время считается в «тиках» симуляции: 10 тиков = 1 игровая минута.
//! При скорости ×1 один тик происходит каждые 0.1 секунды,
//! значит 1 игровая минута = 1 реальная секунда, а вся ночь ≈ 8 минут.

pub const TICKS_PER_MINUTE: u32 = 10;
pub const OPEN_AT_MINUTES: u32 = 22 * 60; // 22:00
pub const NIGHT_MINUTES: u32 = 8 * 60; // до 06:00

#[derive(Debug, Clone, Default)]
pub struct Clock {
    ticks: u32,
}

impl Clock {
    pub fn new() -> Self {
        Self { ticks: 0 }
    }

    /// Один шаг симуляции.
    pub fn advance(&mut self) {
        self.ticks += 1;
    }

    /// Только что началась новая минута? (Удобно, чтобы делать что-то раз в минуту.)
    pub fn is_minute_start(&self) -> bool {
        self.ticks.is_multiple_of(TICKS_PER_MINUTE)
    }

    pub fn minutes_since_open(&self) -> u32 {
        self.ticks / TICKS_PER_MINUTE
    }

    /// Текущее время суток: (часы, минуты).
    pub fn hour_minute(&self) -> (u32, u32) {
        let total = (OPEN_AT_MINUTES + self.minutes_since_open()) % (24 * 60);
        (total / 60, total % 60)
    }

    /// Строка вида "03:33".
    pub fn label(&self) -> String {
        let (h, m) = self.hour_minute();
        format!("{h:02}:{m:02}")
    }

    /// Сколько ночи прошло: от 0.0 до 1.0.
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
