//! Music, ambience and sound effects (owner: integration).
//!
//! Audio never changes the game — it only reacts: sim events, cutscene requests
//! ([`crate::cutscene::AudioRequest`]), UI sounds ([`crate::ui::UiSound`]).
//! Missing files are not fatal: a warning is printed and that sound stays silent.
//!
//! * Music: three looping tracks (menu, calm night, rush) that **crossfade**; the game
//!   picks calm/rush by the guest load ([`Audio::set_music`]). Ducked while dialogue runs.
//! * Ambience: the rain-on-the-window loop ([`Audio::set_rain`]).
//! * One-shots: [`Sfx`] by enum or by content id (`content::SFX_IDS`).
//! * Volumes come from the settings ([`Audio::set_volumes`]); `M` mutes everything.
//!
//! Call [`Audio::update`] once per frame (it moves the fades).

use macroquad::audio::{
    PlaySoundParams, Sound, load_sound, play_sound, set_sound_volume, stop_sound,
};

use crate::cutscene::AudioRequest;
use crate::ui::{ToastKind, UiSound};

/// Base mix (multiplied by the player's volume settings).
const MUSIC_BASE: f32 = 0.55;
const AMBIENCE_BASE: f32 = 0.32;
const SFX_BASE: f32 = 0.85;
/// Seconds for a full crossfade.
const FADE_SECONDS: f32 = 2.4;
/// Music level while a dialogue line is on screen.
const DUCK: f32 = 0.45;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Track {
    Menu,
    Calm,
    Rush,
}

impl Track {
    pub const ALL: [Track; 3] = [Track::Menu, Track::Calm, Track::Rush];

    pub fn id(self) -> &'static str {
        match self {
            Track::Menu => "mus_menu_theme",
            Track::Calm => "mus_night_calm_loop",
            Track::Rush => "mus_rush_loop",
        }
    }

    pub fn from_id(id: &str) -> Option<Track> {
        Track::ALL.into_iter().find(|t| t.id() == id)
    }

    fn index(self) -> usize {
        self as usize
    }
}

/// Short sounds. The number is the index into `Audio::sfx`.
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
    pub const ALL: [Sfx; 9] = [
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

    /// Content id (`content::SFX_IDS`).
    pub fn id(self) -> &'static str {
        match self {
            Sfx::DoorBell => "sfx_door_bell",
            Sfx::CashRegister => "sfx_cash_register",
            Sfx::CoffeeMachine => "sfx_coffee_machine",
            Sfx::CupSaucer => "sfx_cup_saucer",
            Sfx::Footsteps => "sfx_footsteps_tile",
            Sfx::PowerOutage => "sfx_power_outage",
            Sfx::UiClick => "ui_click",
            Sfx::EventAlert => "ui_alert_event",
            Sfx::TrainPass => "amb_train_pass_far",
        }
    }

    pub fn from_id(id: &str) -> Option<Sfx> {
        Sfx::ALL.into_iter().find(|s| s.id() == id)
    }

    fn path(self) -> String {
        let ext = if self.id().starts_with("amb_") {
            "ogg"
        } else {
            "wav"
        };
        format!("assets/audio/sfx/{}.{ext}", self.id())
    }

    /// Per-sound mix so the loud ones sit nicely under the music.
    fn gain(self) -> f32 {
        match self {
            Sfx::UiClick => 0.35,
            Sfx::Footsteps => 0.35,
            Sfx::CoffeeMachine => 0.45,
            Sfx::CupSaucer => 0.55,
            Sfx::CashRegister => 0.5,
            Sfx::DoorBell => 0.6,
            Sfx::TrainPass => 0.8,
            Sfx::EventAlert => 0.6,
            Sfx::PowerOutage => 0.9,
        }
    }
}

/// The mixer state that does not need a sound device (testable).
#[derive(Debug, Clone)]
pub struct Mix {
    /// Current level 0..1 per track.
    pub level: [f32; 3],
    /// Wanted track (`None` = silence).
    pub target: Option<Track>,
    pub duck: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub muted: bool,
    pub rain: bool,
    pub rain_level: f32,
}

impl Default for Mix {
    fn default() -> Self {
        Mix {
            level: [0.0; 3],
            target: None,
            duck: 1.0,
            music_volume: 0.7,
            sfx_volume: 0.8,
            muted: false,
            rain: false,
            rain_level: 0.0,
        }
    }
}

impl Mix {
    /// Moves the fades by `dt` seconds.
    pub fn step(&mut self, dt: f32, ducked: bool) {
        let k = dt / FADE_SECONDS;
        for t in Track::ALL {
            let want = if self.target == Some(t) { 1.0 } else { 0.0 };
            let l = &mut self.level[t.index()];
            *l = if *l < want {
                (*l + k).min(want)
            } else {
                (*l - k * 1.3).max(want)
            };
        }
        let duck_target = if ducked { DUCK } else { 1.0 };
        self.duck += (duck_target - self.duck) * (dt * 4.0).min(1.0);
        let rain_target = if self.rain { 1.0 } else { 0.0 };
        self.rain_level = if self.rain_level < rain_target {
            (self.rain_level + dt / 1.5).min(1.0)
        } else {
            (self.rain_level - dt / 2.0).max(0.0)
        };
    }

    /// Final playback volume of a music track.
    pub fn music_gain(&self, t: Track) -> f32 {
        if self.muted {
            return 0.0;
        }
        MUSIC_BASE * self.music_volume * self.duck * self.level[t.index()]
    }

    pub fn rain_gain(&self) -> f32 {
        if self.muted {
            return 0.0;
        }
        AMBIENCE_BASE * self.sfx_volume.max(self.music_volume * 0.6) * self.rain_level
    }

    pub fn sfx_gain(&self, s: Sfx, volume: f32) -> f32 {
        if self.muted {
            return 0.0;
        }
        SFX_BASE * self.sfx_volume * s.gain() * volume
    }
}

pub struct Audio {
    music: [Option<Sound>; 3],
    /// A looping track is currently started (so `update` can stop silent ones).
    playing: [bool; 3],
    rain: Option<Sound>,
    rain_playing: bool,
    sfx: Vec<Option<Sound>>,
    pub mix: Mix,
    /// Small cooldowns so a busy café does not machine-gun the same sound.
    cooldown: [f32; 9],
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

impl Audio {
    /// Silent mixer (headless tests, `SHIFTELLA_SHOT` runs).
    pub fn silent() -> Audio {
        Audio {
            music: [None, None, None],
            playing: [false; 3],
            rain: None,
            rain_playing: false,
            sfx: Sfx::ALL.iter().map(|_| None).collect(),
            mix: Mix::default(),
            cooldown: [0.0; 9],
        }
    }

    /// Number of files [`Audio::load_step`] goes through.
    pub const FILES: usize = 4 + Sfx::ALL.len();

    /// Loads file number `i` (0..[`Audio::FILES`]) — lets the boot screen draw between files.
    pub async fn load_step(&mut self, i: usize) {
        if i < 3 {
            let t = Track::ALL[i];
            self.music[i] = load(&format!("assets/audio/music/{}.ogg", t.id())).await;
        } else if i == 3 {
            self.rain = load("assets/audio/sfx/amb_rain_window_loop.ogg").await;
        } else if let Some(s) = Sfx::ALL.get(i - 4) {
            self.sfx[i - 4] = load(&s.path()).await;
        }
    }

    pub fn set_volumes(&mut self, music: f32, sfx: f32) {
        self.mix.music_volume = music.clamp(0.0, 1.0);
        self.mix.sfx_volume = sfx.clamp(0.0, 1.0);
    }

    /// Crossfade to `track` (`None` = fade out).
    pub fn set_music(&mut self, track: Option<Track>) {
        self.mix.target = track;
    }

    pub fn music(&self) -> Option<Track> {
        self.mix.target
    }

    pub fn set_rain(&mut self, on: bool) {
        self.mix.rain = on;
    }

    pub fn toggle_mute(&mut self) -> bool {
        self.mix.muted = !self.mix.muted;
        self.mix.muted
    }

    pub fn play_sfx(&mut self, sfx: Sfx) {
        self.play_sfx_vol(sfx, 1.0);
    }

    pub fn play_sfx_vol(&mut self, sfx: Sfx, volume: f32) {
        let i = sfx as usize;
        if self.cooldown[i] > 0.0 {
            return;
        }
        self.cooldown[i] = match sfx {
            Sfx::UiClick => 0.04,
            Sfx::Footsteps => 2.5,
            _ => 0.35,
        };
        let v = self.mix.sfx_gain(sfx, volume);
        if v <= 0.001 {
            return;
        }
        if let Some(s) = &self.sfx[i] {
            play_sound(
                s,
                PlaySoundParams {
                    looped: false,
                    volume: v,
                },
            );
        }
    }

    /// A cutscene's sound request.
    pub fn request(&mut self, r: &AudioRequest) {
        match r {
            AudioRequest::Music { track } => {
                self.set_music(track.as_deref().and_then(Track::from_id));
            }
            AudioRequest::Sfx { id, volume } => {
                if id == "amb_rain_window_loop" {
                    self.set_rain(*volume > 0.0);
                } else if let Some(s) = Sfx::from_id(id) {
                    self.play_sfx_vol(s, *volume);
                }
            }
        }
    }

    pub fn ui_sound(&mut self, s: UiSound) {
        match s {
            UiSound::Click | UiSound::Toggle | UiSound::Advance => {
                self.play_sfx_vol(Sfx::UiClick, 0.8)
            }
            UiSound::Choice => self.play_sfx(Sfx::UiClick),
            UiSound::Toast(ToastKind::Event | ToastKind::Warning) => self.play_sfx(Sfx::EventAlert),
            UiSound::Toast(_) => {}
        }
    }

    /// Per frame: fades, ducking, loop start/stop.
    pub fn update(&mut self, dt: f32, ducked: bool) {
        for c in self.cooldown.iter_mut() {
            *c = (*c - dt).max(0.0);
        }
        self.mix.step(dt, ducked);
        for t in Track::ALL {
            let i = t.index();
            let g = self.mix.music_gain(t);
            let Some(s) = &self.music[i] else { continue };
            if g > 0.001 {
                if !self.playing[i] {
                    play_sound(
                        s,
                        PlaySoundParams {
                            looped: true,
                            volume: g,
                        },
                    );
                    self.playing[i] = true;
                } else {
                    set_sound_volume(s, g);
                }
            } else if self.playing[i] && (self.mix.level[i] <= 0.0 || self.mix.muted) {
                stop_sound(s);
                self.playing[i] = false;
            }
        }
        let g = self.mix.rain_gain();
        if let Some(s) = &self.rain {
            if g > 0.001 {
                if !self.rain_playing {
                    play_sound(
                        s,
                        PlaySoundParams {
                            looped: true,
                            volume: g,
                        },
                    );
                    self.rain_playing = true;
                } else {
                    set_sound_volume(s, g);
                }
            } else if self.rain_playing {
                stop_sound(s);
                self.rain_playing = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossfade_moves_between_tracks() {
        let mut m = Mix {
            target: Some(Track::Calm),
            ..Default::default()
        };
        for _ in 0..200 {
            m.step(0.05, false);
        }
        assert_eq!(m.level[Track::Calm as usize], 1.0);
        m.target = Some(Track::Rush);
        m.step(0.5, false);
        assert!(m.level[Track::Rush as usize] > 0.0);
        assert!(m.level[Track::Calm as usize] < 1.0);
        for _ in 0..200 {
            m.step(0.05, false);
        }
        assert_eq!(m.level[Track::Calm as usize], 0.0);
        assert_eq!(m.level[Track::Rush as usize], 1.0);
    }

    #[test]
    fn mute_and_duck() {
        let mut m = Mix {
            target: Some(Track::Menu),
            level: [1.0, 0.0, 0.0],
            ..Default::default()
        };
        let full = m.music_gain(Track::Menu);
        for _ in 0..60 {
            m.step(0.05, true);
        }
        assert!(m.music_gain(Track::Menu) < full * 0.6);
        m.muted = true;
        assert_eq!(m.music_gain(Track::Menu), 0.0);
        assert_eq!(m.sfx_gain(Sfx::DoorBell, 1.0), 0.0);
    }

    #[test]
    fn ids_round_trip() {
        for s in Sfx::ALL {
            assert_eq!(Sfx::from_id(s.id()), Some(s));
            assert!(crate::content::SFX_IDS.contains(&s.id()), "{}", s.id());
        }
        for t in Track::ALL {
            assert_eq!(Track::from_id(t.id()), Some(t));
            assert!(crate::content::MUSIC_TRACKS.contains(&t.id()));
        }
    }
}
