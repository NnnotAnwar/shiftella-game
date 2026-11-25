//! Музыка, фоновые звуки (дождь) и эффекты.
//!
//! Звук не меняет игру — он только реагирует на события (`GameEvent`).
//! Если какого-то файла нет, игра не падает: пишет предупреждение и молчит на этом месте.

use macroquad::audio::{PlaySoundParams, Sound, load_sound, play_sound, stop_sound};

use crate::game::GameEvent;

const MUSIC_VOLUME: f32 = 0.45;
const AMBIENCE_VOLUME: f32 = 0.3;
const SFX_VOLUME: f32 = 0.8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Track {
    Calm,
    /// Для часа пик — включишь, когда появятся гости.
    #[allow(dead_code)]
    Rush,
}

/// Короткие звуки. Номер в enum = место в массиве `Audio::sfx`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sfx {
    DoorBell,
    CashRegister,
    CoffeeMachine,
    CupSaucer,
    Footsteps,
    PowerOutage,
    UiClick,
    EventAlert,
    TrainPass,
}

impl Sfx {
    const ALL: [Sfx; 9] = [
        Sfx::DoorBell,
        Sfx::CashRegister,
        Sfx::CoffeeMachine,
        Sfx::CupSaucer,
        Sfx::Footsteps,
        Sfx::PowerOutage,
        Sfx::UiClick,
        Sfx::EventAlert,
        Sfx::TrainPass,
    ];

    fn path(self) -> &'static str {
        match self {
            Sfx::DoorBell => "assets/audio/sfx/sfx_door_bell.wav",
            Sfx::CashRegister => "assets/audio/sfx/sfx_cash_register.wav",
            Sfx::CoffeeMachine => "assets/audio/sfx/sfx_coffee_machine.wav",
            Sfx::CupSaucer => "assets/audio/sfx/sfx_cup_saucer.wav",
            Sfx::Footsteps => "assets/audio/sfx/sfx_footsteps_tile.wav",
            Sfx::PowerOutage => "assets/audio/sfx/sfx_power_outage.wav",
            Sfx::UiClick => "assets/audio/sfx/ui_click.wav",
            Sfx::EventAlert => "assets/audio/sfx/ui_alert_event.wav",
            Sfx::TrainPass => "assets/audio/sfx/amb_train_pass_far.ogg",
        }
    }
}

pub struct Audio {
    calm: Option<Sound>,
    rush: Option<Sound>,
    rain: Option<Sound>,
    sfx: Vec<Option<Sound>>,
    /// Какой трек должен играть (даже если звук выключен — чтобы включить обратно).
    current: Option<Track>,
    rain_on: bool,
    muted: bool,
}

async fn load(path: &str) -> Option<Sound> {
    match load_sound(path).await {
        Ok(sound) => Some(sound),
        Err(e) => {
            eprintln!("warning: can't load sound {path}: {e}");
            None
        }
    }
}

fn play(sound: &Option<Sound>, looped: bool, volume: f32) {
    if let Some(s) = sound {
        play_sound(s, PlaySoundParams { looped, volume });
    }
}

fn stop(sound: &Option<Sound>) {
    if let Some(s) = sound {
        stop_sound(s);
    }
}

impl Audio {
    pub async fn load() -> Self {
        let mut sfx = Vec::with_capacity(Sfx::ALL.len());
        for s in Sfx::ALL {
            sfx.push(load(s.path()).await);
        }
        Self {
            calm: load("assets/audio/music/mus_night_calm_loop.ogg").await,
            rush: load("assets/audio/music/mus_rush_loop.ogg").await,
            rain: load("assets/audio/sfx/amb_rain_window_loop.ogg").await,
            sfx,
            current: None,
            rain_on: false,
            muted: false,
        }
    }

    fn track_sound(&self, track: Track) -> &Option<Sound> {
        match track {
            Track::Calm => &self.calm,
            Track::Rush => &self.rush,
        }
    }

    /// Включить трек по кругу. Если он уже играет — ничего не делаем.
    /// Пригодится: когда гостей много — `play_music(Track::Rush)`, мало — `Track::Calm`.
    pub fn play_music(&mut self, track: Track) {
        if self.current == Some(track) {
            return;
        }
        if let Some(old) = self.current {
            stop(self.track_sound(old));
        }
        self.current = Some(track);
        if !self.muted {
            play(self.track_sound(track), true, MUSIC_VOLUME);
        }
    }

    pub fn stop_music(&mut self) {
        if let Some(old) = self.current.take() {
            stop(self.track_sound(old));
        }
    }

    fn set_rain(&mut self, on: bool) {
        if on == self.rain_on {
            return;
        }
        self.rain_on = on;
        if on && !self.muted {
            play(&self.rain, true, AMBIENCE_VOLUME);
        } else {
            stop(&self.rain);
        }
    }

    pub fn play_sfx(&self, sfx: Sfx) {
        if !self.muted {
            play(&self.sfx[sfx as usize], false, SFX_VOLUME);
        }
    }

    /// Клавиша M: выключить/включить весь звук.
    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        if self.muted {
            if let Some(t) = self.current {
                stop(self.track_sound(t));
            }
            stop(&self.rain);
        } else {
            if let Some(t) = self.current {
                play(self.track_sound(t), true, MUSIC_VOLUME);
            }
            if self.rain_on {
                play(&self.rain, true, AMBIENCE_VOLUME);
            }
        }
    }

    /// Реакция на событие игры.
    pub fn handle(&mut self, event: GameEvent) {
        match event {
            GameEvent::NightStarted => {
                self.play_music(Track::Calm);
                self.set_rain(true);
            }
            GameEvent::NightEnded => {
                self.stop_music();
                self.set_rain(false);
                self.play_sfx(Sfx::EventAlert);
            }
            GameEvent::TrainPassing => self.play_sfx(Sfx::TrainPass),
        }
    }
}
