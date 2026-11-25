//! Состояние игры и один шаг симуляции (`tick`).
//!
//! Важное правило: вся логика живёт в `tick`, а не в отрисовке и не в звуке.
//! Тогда пауза и ускорение ×2/×3 работают сами собой.
//!
//! О том, что случилось, игра сообщает событиями (`GameEvent`).
//! Звук, а потом и интерфейс, их забирают через `drain_events()` и реагируют.

use crate::clock::Clock;
use crate::map::{CAFE_LAYOUT, Map};

/// Расписание ночных поездов (час, минута). Поезд в 3:33 — не случайность.
const TRAIN_TIMES: [(u32, u32); 5] = [(22, 47), (0, 15), (1, 52), (3, 33), (5, 20)];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Speed {
    Paused,
    Normal,
    Fast,
    Faster,
}

impl Speed {
    pub fn multiplier(self) -> f32 {
        match self {
            Speed::Paused => 0.0,
            Speed::Normal => 1.0,
            Speed::Fast => 2.0,
            Speed::Faster => 3.0,
        }
    }
}

/// Что произошло в мире. Сюда же потом добавишь `GuestArrived`, `OrderServed` и т.д.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEvent {
    NightStarted,
    NightEnded,
    TrainPassing,
}

pub struct Game {
    pub night: u32,
    pub clock: Clock,
    pub map: Map,
    pub speed: Speed,
    pub night_over: bool,
    /// Какая скорость была до паузы — чтобы пробел возвращал её обратно.
    speed_before_pause: Speed,
    events: Vec<GameEvent>,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    pub fn new() -> Self {
        Self {
            night: 1,
            clock: Clock::new(),
            map: Map::from_layout(CAFE_LAYOUT),
            speed: Speed::Normal,
            night_over: false,
            speed_before_pause: Speed::Normal,
            events: vec![GameEvent::NightStarted],
        }
    }

    /// Забрать все события, накопившиеся с прошлого раза.
    pub fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn set_speed(&mut self, speed: Speed) {
        if !self.night_over {
            self.speed = speed;
        }
    }

    pub fn toggle_pause(&mut self) {
        if self.night_over {
            return;
        }
        if self.speed == Speed::Paused {
            self.speed = self.speed_before_pause;
        } else {
            self.speed_before_pause = self.speed;
            self.speed = Speed::Paused;
        }
    }

    /// Один шаг мира. Вызывается 10 раз в секунду при скорости ×1.
    pub fn tick(&mut self) {
        if self.night_over {
            return;
        }

        self.clock.advance();

        if self.clock.is_minute_start() && TRAIN_TIMES.contains(&self.clock.hour_minute()) {
            self.events.push(GameEvent::TrainPassing);
        }

        // TODO(механики): здесь будут жить гости и сотрудники:
        //   - спавн гостей у двери
        //   - сотрудники выбирают работу и идут к ней
        //   - потребности (энергия, настроение, голод) меняются со временем

        if self.clock.is_night_over() {
            self.night_over = true;
            self.speed = Speed::Paused;
            self.events.push(GameEvent::NightEnded);
        }
    }

    pub fn start_next_night(&mut self) {
        self.night += 1;
        self.clock = Clock::new();
        self.night_over = false;
        self.speed = Speed::Normal;
        self.events.push(GameEvent::NightStarted);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::{NIGHT_MINUTES, TICKS_PER_MINUTE};

    fn run_whole_night(game: &mut Game) -> Vec<GameEvent> {
        let mut all = game.drain_events();
        for _ in 0..NIGHT_MINUTES * TICKS_PER_MINUTE {
            game.tick();
            all.extend(game.drain_events());
        }
        all
    }

    #[test]
    fn a_night_starts_has_five_trains_and_ends() {
        let mut game = Game::new();
        let events = run_whole_night(&mut game);
        assert_eq!(events.first(), Some(&GameEvent::NightStarted));
        assert_eq!(events.last(), Some(&GameEvent::NightEnded));
        let trains = events
            .iter()
            .filter(|e| **e == GameEvent::TrainPassing)
            .count();
        assert_eq!(trains, TRAIN_TIMES.len());
    }

    #[test]
    fn next_night_starts_again() {
        let mut game = Game::new();
        run_whole_night(&mut game);
        assert!(game.night_over);
        game.start_next_night();
        assert_eq!(game.night, 2);
        assert_eq!(game.drain_events(), vec![GameEvent::NightStarted]);
    }
}
