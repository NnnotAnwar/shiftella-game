//! Music, ambience and sound effects (owner: integration).
//!
//! Audio never changes the game — it only reacts: sim events, cutscene requests
//! ([`crate::cutscene::AudioRequest`]), UI sounds ([`crate::ui::UiSound`]).
//! Missing files are not fatal: a warning is printed and that sound stays silent.
//!
//! * Music: nine looping tracks (menu, calm night, rush and grandma's six jukebox records)
//!   that **crossfade** (2.5 s); the game plays what the sim's `World::music_track` says
//!   ([`Audio::set_music_vol`]). Ducked while dialogue runs.
//! * Ambience: the rain-on-the-window loop ([`Audio::set_rain`]) and the CC0 loops ([`AmbLoop`]:
//!   the soup pot, rain on the awning, the storm wind, the cat's purr, wet steps). A loop plays
//!   while someone asks for it each frame ([`Audio::want_amb`]) or a cutscene holds it
//!   ([`Audio::hold_amb`]); it fades in and out, and each one plays once however many ask.
//! * Mini-game loops (design §14): steam (`sfx_coffee_machine` looped) and a soft pour
//!   trickle (a second `amb_rain_window_loop` instance), see [`Audio::set_loops`].
//! * One-shots: [`Sfx`] by enum or by content id (`content::SFX_IDS`).
//! * Volumes come from the settings ([`Audio::set_volumes`]); `M` mutes everything.
//!
//! Files: the generated pack (`assets/audio/sfx/`, `music/`) is not in the repository
//! (ElevenLabs licence). The CC0 sounds are (`assets/audio/cc0/`, credits in
//! `assets/audio/CREDITS.md`). A sound id loads from the generated pack first and falls back to
//! `cc0/`, so a generated file can still replace a CC0 one; one-shots may be `.wav` or `.ogg`.
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
/// Seconds for a full crossfade (design §10.3: 2.5 s).
const FADE_SECONDS: f32 = 2.5;
/// Number of music tracks.
pub const TRACKS: usize = 9;
/// Music level while a dialogue line is on screen.
const DUCK: f32 = 0.33;
/// Ambience (rain) under dialogue: a little lower, still clearly audible.
const AMB_DUCK: f32 = 0.75;
/// Seconds to duck down / to come back up (never an abrupt cut).
const DUCK_DOWN_SECONDS: f32 = 0.4;
const DUCK_UP_SECONDS: f32 = 0.8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Track {
    Menu,
    Calm,
    Rush,
    /// Grandma's records (design §10.2).
    PragueSwing,
    PianoTrio,
    BossaRain,
    FolkLullaby,
    Synth2039,
    CafeWaltz,
}

impl Track {
    pub const ALL: [Track; TRACKS] = [
        Track::Menu,
        Track::Calm,
        Track::Rush,
        Track::PragueSwing,
        Track::PianoTrio,
        Track::BossaRain,
        Track::FolkLullaby,
        Track::Synth2039,
        Track::CafeWaltz,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Track::Menu => "mus_menu_theme",
            Track::Calm => "mus_night_calm_loop",
            Track::Rush => "mus_rush_loop",
            Track::PragueSwing => "jukebox_prague_swing",
            Track::PianoTrio => "jukebox_piano_trio",
            Track::BossaRain => "jukebox_bossa_rain",
            Track::FolkLullaby => "jukebox_folk_lullaby",
            Track::Synth2039 => "jukebox_synth_2039",
            Track::CafeWaltz => "jukebox_cafe_waltz",
        }
    }

    pub fn from_id(id: &str) -> Option<Track> {
        Track::ALL.into_iter().find(|t| t.id() == id)
    }

    fn index(self) -> usize {
        self as usize
    }
}

/// Number of [`Sfx`] sounds.
pub const SFX_COUNT: usize = 38;

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
    // ── CC0 (assets/audio/cc0/, v0.2 package K) ──
    /// The coffee machine warming up: a short, soft steam hiss.
    MachineHiss,
    /// A steam-wand purge (servicing a worn machine).
    MachinePurge,
    /// The end-of-steaming sputter (a worn appliance asks for attention).
    MachineSputter,
    /// A shot pulled ahead (the tray).
    ShotPull,
    /// Cleaning the group head: a knock, then a blank shot.
    GroupClean,
    /// The portafilter knocked out on the knock box.
    KnockBox,
    /// A pot lid set on / lifted.
    PotLid,
    /// A spoon against the pot (stirring).
    PotStir,
    /// The oven timer's chime.
    OvenDing,
    /// The oven door opening and closing.
    OvenDoor,
    /// The fridge door opening and closing.
    FridgeDoor,
    /// Bottles clinking (the milk comes out).
    BottleClink,
    /// Coins counted out of the till.
    TillCoins,
    /// A trash bag picked up.
    TrashBag,
    /// A wooden door to the outside, open and shut.
    DoorOutside,
    /// A lamp's switch.
    LampSwitch,
    /// Keys locking the café door.
    KeysLock,
    /// Pigeons taking off.
    Pigeons,
    /// A Czech tram's bell.
    TramBell,
    /// A tram passing by.
    TramPass,
    /// The back door's creak.
    DoorCreak,
    /// A phone buzzing twice (a message).
    PhoneBuzz,
    /// A small, soft meow.
    CatMeow,
    /// A book opened.
    BookOpen,
    /// A page turned.
    PageFlip,
    /// A blanket handed over / unfolded.
    Blanket,
    /// One chalk stroke on a board. Not played yet: no UI writes on the menu board (the
    /// specials are set by `Command::SetSpecials`, the signature drink is package I) — play it
    /// there with `audio.play_sfx_vol(Sfx::Chalk, 0.7)`.
    Chalk,
    /// A polaroid's shutter (the morning pick; ready for photo mode).
    PhotoShutter,
    /// A polaroid's print coming out (the morning pick; ready for photo mode).
    PhotoEject,
}

impl Sfx {
    pub const ALL: [Sfx; SFX_COUNT] = [
        Sfx::DoorBell,
        Sfx::CashRegister,
        Sfx::CoffeeMachine,
        Sfx::CupSaucer,
        Sfx::Footsteps,
        Sfx::PowerOutage,
        Sfx::UiClick,
        Sfx::EventAlert,
        Sfx::TrainPass,
        Sfx::MachineHiss,
        Sfx::MachinePurge,
        Sfx::MachineSputter,
        Sfx::ShotPull,
        Sfx::GroupClean,
        Sfx::KnockBox,
        Sfx::PotLid,
        Sfx::PotStir,
        Sfx::OvenDing,
        Sfx::OvenDoor,
        Sfx::FridgeDoor,
        Sfx::BottleClink,
        Sfx::TillCoins,
        Sfx::TrashBag,
        Sfx::DoorOutside,
        Sfx::LampSwitch,
        Sfx::KeysLock,
        Sfx::Pigeons,
        Sfx::TramBell,
        Sfx::TramPass,
        Sfx::DoorCreak,
        Sfx::PhoneBuzz,
        Sfx::CatMeow,
        Sfx::BookOpen,
        Sfx::PageFlip,
        Sfx::Blanket,
        Sfx::Chalk,
        Sfx::PhotoShutter,
        Sfx::PhotoEject,
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
            Sfx::MachineHiss => "sfx_machine_hiss",
            Sfx::MachinePurge => "sfx_machine_purge",
            Sfx::MachineSputter => "sfx_machine_sputter",
            Sfx::ShotPull => "sfx_shot_pull",
            Sfx::GroupClean => "sfx_group_clean",
            Sfx::KnockBox => "sfx_knock_box",
            Sfx::PotLid => "sfx_pot_lid",
            Sfx::PotStir => "sfx_pot_stir",
            Sfx::OvenDing => "sfx_oven_ding",
            Sfx::OvenDoor => "sfx_oven_door",
            Sfx::FridgeDoor => "sfx_fridge_door",
            Sfx::BottleClink => "sfx_bottle_clink",
            Sfx::TillCoins => "sfx_till_coins",
            Sfx::TrashBag => "sfx_trash_bag",
            Sfx::DoorOutside => "sfx_door_outside",
            Sfx::LampSwitch => "sfx_lamp_switch",
            Sfx::KeysLock => "sfx_keys_lock",
            Sfx::Pigeons => "sfx_pigeons",
            Sfx::TramBell => "sfx_tram_bell",
            Sfx::TramPass => "sfx_tram_pass",
            Sfx::DoorCreak => "sfx_door_creak",
            Sfx::PhoneBuzz => "sfx_phone_buzz",
            Sfx::CatMeow => "sfx_cat_meow",
            Sfx::BookOpen => "sfx_book_open",
            Sfx::PageFlip => "sfx_page_flip",
            Sfx::Blanket => "sfx_blanket",
            Sfx::Chalk => "sfx_chalk",
            Sfx::PhotoShutter => "photo_shutter",
            Sfx::PhotoEject => "photo_eject",
        }
    }

    /// A CC0 sound shipped in the repository (`assets/audio/cc0/`), not the generated pack.
    pub fn is_cc0(self) -> bool {
        self as usize >= Sfx::MachineHiss as usize
    }

    pub fn from_id(id: &str) -> Option<Sfx> {
        Sfx::ALL.into_iter().find(|s| s.id() == id)
    }

    fn paths(self) -> Vec<String> {
        sound_paths(self.id())
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
            // the CC0 files are all cut to the same loudness (-24 LUFS, like the coffee machine)
            Sfx::MachineHiss | Sfx::MachinePurge | Sfx::ShotPull => 0.5,
            Sfx::MachineSputter => 0.45,
            Sfx::GroupClean | Sfx::KnockBox => 0.5,
            Sfx::PotLid | Sfx::PotStir => 0.45,
            Sfx::OvenDing => 0.55,
            Sfx::OvenDoor | Sfx::FridgeDoor | Sfx::DoorOutside => 0.5,
            Sfx::BottleClink => 0.4,
            Sfx::TillCoins => 0.55,
            Sfx::TrashBag => 0.45,
            Sfx::LampSwitch => 0.4,
            Sfx::KeysLock => 0.55,
            Sfx::Pigeons => 0.5,
            Sfx::TramBell => 0.55,
            Sfx::TramPass => 0.7,
            Sfx::DoorCreak => 0.5,
            Sfx::PhoneBuzz => 0.5,
            Sfx::CatMeow => 0.5,
            Sfx::BookOpen | Sfx::PageFlip => 0.45,
            Sfx::Blanket => 0.45,
            Sfx::Chalk => 0.4,
            Sfx::PhotoShutter | Sfx::PhotoEject => 0.55,
        }
    }
}

/// Where a sound id may live: the generated pack first, then the CC0 folder; one-shots prefer
/// `.wav`, `amb_` loops `.ogg`, and either extension is accepted.
pub fn sound_paths(id: &str) -> Vec<String> {
    let exts = if id.starts_with("amb_") {
        ["ogg", "wav"]
    } else {
        ["wav", "ogg"]
    };
    let mut v = Vec::with_capacity(4);
    for dir in ["assets/audio/sfx", CC0_DIR] {
        for ext in exts {
            v.push(format!("{dir}/{id}.{ext}"));
        }
    }
    v
}

/// The CC0 sounds committed with the source (credits in `assets/audio/CREDITS.md`).
pub const CC0_DIR: &str = "assets/audio/cc0";

/// Number of [`AmbLoop`]s.
pub const AMB_COUNT: usize = 5;

/// The CC0 ambience loops (`assets/audio/cc0/<id>.ogg`). Each plays at most once, at the
/// loudest level asked for this frame ([`Audio::want_amb`]) or held by a cutscene
/// ([`Audio::hold_amb`]), and fades in and out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmbLoop {
    /// A pot of soup simmering on the stove.
    Soup,
    /// Light rain on the awning over the back step.
    RainAwning,
    /// Wind howling through a window crack (the storm night).
    Storm,
    /// The station cat purring close by.
    CatPurr,
    /// Steps on the wet pavement (the walk home).
    WetSteps,
}

impl AmbLoop {
    pub const ALL: [AmbLoop; AMB_COUNT] = [
        AmbLoop::Soup,
        AmbLoop::RainAwning,
        AmbLoop::Storm,
        AmbLoop::CatPurr,
        AmbLoop::WetSteps,
    ];

    pub fn id(self) -> &'static str {
        match self {
            AmbLoop::Soup => "amb_soup_simmer",
            AmbLoop::RainAwning => "amb_rain_awning",
            AmbLoop::Storm => "amb_storm",
            AmbLoop::CatPurr => "amb_cat_purr",
            AmbLoop::WetSteps => "amb_wet_steps",
        }
    }

    pub fn from_id(id: &str) -> Option<AmbLoop> {
        AmbLoop::ALL.into_iter().find(|a| a.id() == id)
    }

    /// Mix level (the files are all -24 LUFS, like the rain on the window).
    fn gain(self) -> f32 {
        match self {
            AmbLoop::Soup => 0.7,
            AmbLoop::RainAwning => 1.5,
            AmbLoop::Storm => 1.3,
            AmbLoop::CatPurr => 1.6,
            AmbLoop::WetSteps => 1.2,
        }
    }
}

/// The mixer state that does not need a sound device (testable).
#[derive(Debug, Clone)]
pub struct Mix {
    /// Current level 0..1 per track.
    pub level: [f32; TRACKS],
    /// Wanted track (`None` = silence).
    pub target: Option<Track>,
    /// Volume of each track while it plays (the quiet-hour piano runs at 0.7).
    pub track_volume: [f32; TRACKS],
    /// Mini-game loops: wanted and current level (steam, trickle).
    pub loop_target: [f32; 2],
    pub loop_level: [f32; 2],
    /// Music multiplier while dialogue / narration text is on screen (1 = none).
    pub duck: f32,
    /// Ambience multiplier while text is on screen.
    pub amb_duck: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub muted: bool,
    pub rain: bool,
    pub rain_level: f32,
    /// CC0 loops: asked for this frame (cleared by [`Audio::update`]), held by a cutscene,
    /// and the current (fading) level.
    pub amb_want: [f32; AMB_COUNT],
    pub amb_hold: [f32; AMB_COUNT],
    pub amb_level: [f32; AMB_COUNT],
}

impl Default for Mix {
    fn default() -> Self {
        Mix {
            level: [0.0; TRACKS],
            target: None,
            track_volume: [1.0; TRACKS],
            loop_target: [0.0; 2],
            loop_level: [0.0; 2],
            duck: 1.0,
            amb_duck: 1.0,
            music_volume: 0.7,
            sfx_volume: 0.8,
            muted: false,
            rain: false,
            rain_level: 0.0,
            amb_want: [0.0; AMB_COUNT],
            amb_hold: [0.0; AMB_COUNT],
            amb_level: [0.0; AMB_COUNT],
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
        // linear ramps: down in DUCK_DOWN_SECONDS, back up in DUCK_UP_SECONDS
        let ramp = |v: f32, low: f32| {
            if ducked {
                (v - dt * (1.0 - low) / DUCK_DOWN_SECONDS).max(low)
            } else {
                (v + dt * (1.0 - low) / DUCK_UP_SECONDS).min(1.0)
            }
        };
        self.duck = ramp(self.duck, DUCK);
        self.amb_duck = ramp(self.amb_duck, AMB_DUCK);
        // mini-game loops follow quickly (a held steam wand, a pour)
        for i in 0..2 {
            let (l, want) = (self.loop_level[i], self.loop_target[i]);
            let k = dt / 0.18;
            self.loop_level[i] = if l < want {
                (l + k).min(want)
            } else {
                (l - k).max(want)
            };
        }
        let rain_target = if self.rain { 1.0 } else { 0.0 };
        self.rain_level = if self.rain_level < rain_target {
            (self.rain_level + dt / 1.5).min(1.0)
        } else {
            (self.rain_level - dt / 2.0).max(0.0)
        };
        // CC0 loops: in over 1.2 s, out over 1.6 s (never an abrupt cut)
        for i in 0..AMB_COUNT {
            let want = self.amb_want[i].max(self.amb_hold[i]).clamp(0.0, 1.0);
            let l = self.amb_level[i];
            self.amb_level[i] = if l < want {
                (l + dt / 1.2).min(want)
            } else {
                (l - dt / 1.6).max(want)
            };
        }
    }

    #[cfg(test)]
    fn amb_gain_full(&self, a: AmbLoop) -> f32 {
        AMBIENCE_BASE * self.sfx_volume * self.amb_duck * a.gain()
    }

    /// Volume of a CC0 ambience loop.
    pub fn amb_gain(&self, a: AmbLoop) -> f32 {
        if self.muted {
            return 0.0;
        }
        AMBIENCE_BASE * self.sfx_volume * self.amb_duck * a.gain() * self.amb_level[a as usize]
    }

    /// Final playback volume of a music track.
    pub fn music_gain(&self, t: Track) -> f32 {
        if self.muted {
            return 0.0;
        }
        MUSIC_BASE
            * self.music_volume
            * self.duck
            * self.level[t.index()]
            * self.track_volume[t.index()]
    }

    /// Volume of a mini-game loop (0 = steam, 1 = trickle).
    pub fn loop_gain(&self, i: usize) -> f32 {
        if self.muted {
            return 0.0;
        }
        SFX_BASE * self.sfx_volume * self.loop_level[i] * if i == 0 { 0.5 } else { 0.9 }
    }

    pub fn rain_gain(&self) -> f32 {
        if self.muted {
            return 0.0;
        }
        AMBIENCE_BASE
            * self.sfx_volume.max(self.music_volume * 0.6)
            * self.rain_level
            * self.amb_duck
    }

    pub fn sfx_gain(&self, s: Sfx, volume: f32) -> f32 {
        if self.muted {
            return 0.0;
        }
        SFX_BASE * self.sfx_volume * s.gain() * volume
    }
}

pub struct Audio {
    music: [Option<Sound>; TRACKS],
    /// A looping track is currently started (so `update` can stop silent ones).
    playing: [bool; TRACKS],
    /// Mini-game loops: steam (the coffee machine) and a pour trickle (rain, second instance).
    loops: [Option<Sound>; 2],
    loop_playing: [bool; 2],
    rain: Option<Sound>,
    rain_playing: bool,
    sfx: Vec<Option<Sound>>,
    amb: [Option<Sound>; AMB_COUNT],
    amb_playing: [bool; AMB_COUNT],
    pub mix: Mix,
    /// Small cooldowns so a busy café does not machine-gun the same sound.
    cooldown: [f32; SFX_COUNT],
}

/// The first of `paths` that exists (see [`sound_paths`]).
async fn load_first(paths: &[String]) -> Option<Sound> {
    let path = paths
        .iter()
        .find(|p| std::path::Path::new(p.as_str()).exists())?;
    load(path).await
}

async fn load(path: &str) -> Option<Sound> {
    // release builds ship the audio; a source checkout may not have it — stay silent
    if !std::path::Path::new(path).exists() {
        return None;
    }
    match load_sound(path).await {
        Ok(sound) => Some(sound),
        Err(e) => {
            eprintln!("warning: can't load sound {path}: {e}");
            None
        }
    }
}

/// Some audio is present: the generated pack (not in the source repository) or the CC0
/// sounds (committed).
pub fn audio_available() -> bool {
    ["assets/audio/music", "assets/audio/sfx", CC0_DIR]
        .iter()
        .any(|d| std::path::Path::new(d).is_dir())
}

/// The generated pack (music, the original SFX) is present — release builds only.
pub fn generated_pack_available() -> bool {
    std::path::Path::new("assets/audio/music").is_dir()
}

impl Audio {
    /// Silent mixer (headless tests, `SHIFTELLA_SHOT` runs).
    pub fn silent() -> Audio {
        Audio {
            music: Default::default(),
            playing: [false; TRACKS],
            loops: [None, None],
            loop_playing: [false; 2],
            rain: None,
            rain_playing: false,
            sfx: Sfx::ALL.iter().map(|_| None).collect(),
            amb: Default::default(),
            amb_playing: [false; AMB_COUNT],
            mix: Mix::default(),
            cooldown: [0.0; SFX_COUNT],
        }
    }

    /// Number of files [`Audio::load_step`] goes through.
    pub const FILES: usize = TRACKS + 3 + SFX_COUNT + AMB_COUNT;

    /// Loads file number `i` (0..[`Audio::FILES`]) — lets the boot screen draw between files.
    pub async fn load_step(&mut self, i: usize) {
        if i < TRACKS {
            let t = Track::ALL[i];
            self.music[i] = load(&format!("assets/audio/music/{}.ogg", t.id())).await;
        } else if i == TRACKS {
            self.rain = load("assets/audio/sfx/amb_rain_window_loop.ogg").await;
        } else if i == TRACKS + 1 {
            self.loops[0] = load_first(&Sfx::CoffeeMachine.paths()).await;
        } else if i == TRACKS + 2 {
            self.loops[1] = load("assets/audio/sfx/amb_rain_window_loop.ogg").await;
        } else if let Some(s) = Sfx::ALL.get(i - TRACKS - 3) {
            self.sfx[i - TRACKS - 3] = load_first(&s.paths()).await;
        } else if let Some(a) = AmbLoop::ALL.get(i - TRACKS - 3 - SFX_COUNT) {
            self.amb[*a as usize] = load_first(&sound_paths(a.id())).await;
        }
    }

    pub fn set_volumes(&mut self, music: f32, sfx: f32) {
        self.mix.music_volume = music.clamp(0.0, 1.0);
        self.mix.sfx_volume = sfx.clamp(0.0, 1.0);
    }

    /// Crossfade to `track` (`None` = fade out).
    pub fn set_music(&mut self, track: Option<Track>) {
        self.set_music_vol(track, 1.0);
    }

    /// Crossfade to `track`, played at `volume` (0..1) of the music level.
    pub fn set_music_vol(&mut self, track: Option<Track>, volume: f32) {
        self.mix.target = track;
        if let Some(t) = track {
            self.mix.track_volume[t.index()] = volume.clamp(0.0, 1.0);
        }
    }

    /// Mini-game loops this frame (0..1 each): steam hiss and pour trickle (§14).
    pub fn set_loops(&mut self, steam: f32, trickle: f32) {
        self.mix.loop_target = [steam.clamp(0.0, 1.0), trickle.clamp(0.0, 1.0)];
    }

    pub fn music(&self) -> Option<Track> {
        self.mix.target
    }

    pub fn set_rain(&mut self, on: bool) {
        self.mix.rain = on;
    }

    /// Asks for a CC0 loop this frame at `level` (0..1); call every frame it should play. The
    /// loudest ask wins, so several pots on the stove are still one soup loop.
    pub fn want_amb(&mut self, a: AmbLoop, level: f32) {
        let w = &mut self.mix.amb_want[a as usize];
        *w = w.max(level.clamp(0.0, 1.0));
    }

    /// Holds a CC0 loop at `level` until it is set again (`0` = let it fade out) — cutscenes.
    pub fn hold_amb(&mut self, a: AmbLoop, level: f32) {
        self.mix.amb_hold[a as usize] = level.clamp(0.0, 1.0);
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
                } else if let Some(a) = AmbLoop::from_id(id) {
                    self.hold_amb(a, *volume);
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
        self.mix.amb_want = [0.0; AMB_COUNT];
        for a in AmbLoop::ALL {
            let i = a as usize;
            let g = self.mix.amb_gain(a);
            let Some(s) = &self.amb[i] else { continue };
            if g > 0.001 {
                if !self.amb_playing[i] {
                    play_sound(
                        s,
                        PlaySoundParams {
                            looped: true,
                            volume: g,
                        },
                    );
                    self.amb_playing[i] = true;
                } else {
                    set_sound_volume(s, g);
                }
            } else if self.amb_playing[i] {
                stop_sound(s);
                self.amb_playing[i] = false;
            }
        }
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
        for i in 0..2 {
            let g = self.mix.loop_gain(i);
            let Some(s) = &self.loops[i] else { continue };
            if g > 0.001 {
                if !self.loop_playing[i] {
                    play_sound(
                        s,
                        PlaySoundParams {
                            looped: true,
                            volume: g,
                        },
                    );
                    self.loop_playing[i] = true;
                } else {
                    set_sound_volume(s, g);
                }
            } else if self.loop_playing[i] {
                stop_sound(s);
                self.loop_playing[i] = false;
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
            level: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
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
    fn dialogue_ducks_music_smoothly_and_comes_back() {
        let mut m = Mix {
            target: Some(Track::Calm),
            level: [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            rain: true,
            rain_level: 1.0,
            ..Default::default()
        };
        let (music, rain) = (m.music_gain(Track::Calm), m.rain_gain());
        // a fraction of a second in: on its way down, not cut
        m.step(0.1, true);
        let g = m.music_gain(Track::Calm);
        assert!(g < music && g > music * 0.7, "{g} vs {music}");
        for _ in 0..30 {
            m.step(1.0 / 60.0, true);
        }
        let low = m.music_gain(Track::Calm) / music;
        assert!((0.3..=0.35).contains(&low), "ducked to {low}");
        let rain_low = m.rain_gain() / rain;
        assert!(rain_low > 0.6 && rain_low < 0.9, "rain {rain_low}");
        // back up slower than it went down
        m.step(0.4, false);
        assert!(m.music_gain(Track::Calm) < music * 0.8);
        m.step(0.5, false);
        assert!((m.music_gain(Track::Calm) - music).abs() < 1e-4);
        // the user's volume still scales everything
        m.music_volume = 0.0;
        assert_eq!(m.music_gain(Track::Calm), 0.0);
    }

    #[test]
    fn jukebox_record_crossfades_at_its_volume() {
        let mut m = Mix {
            target: Some(Track::Calm),
            level: [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            ..Default::default()
        };
        m.target = Some(Track::PianoTrio);
        m.track_volume[Track::PianoTrio.index()] = 0.7;
        // audible within 3 s (design §15 acceptance)
        for _ in 0..180 {
            m.step(1.0 / 60.0, false);
        }
        assert_eq!(m.level[Track::Calm.index()], 0.0);
        let full = MUSIC_BASE * m.music_volume;
        assert!((m.music_gain(Track::PianoTrio) - full * 0.7).abs() < 1e-4);
    }

    #[test]
    fn minigame_loops_follow_quickly() {
        let mut m = Mix {
            loop_target: [0.8, 0.0],
            ..Default::default()
        };
        m.step(0.25, false);
        assert!(m.loop_gain(0) > 0.0);
        m.loop_target = [0.0, 0.25];
        m.step(0.5, false);
        assert_eq!(m.loop_gain(0), 0.0);
        assert!(m.loop_gain(1) > 0.0);
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
        for a in AmbLoop::ALL {
            assert_eq!(AmbLoop::from_id(a.id()), Some(a));
            assert!(a.id().starts_with("amb_"));
            assert!(crate::content::SFX_IDS.contains(&a.id()), "{}", a.id());
        }
        for (i, s) in Sfx::ALL.iter().enumerate() {
            assert_eq!(*s as usize, i, "Sfx::ALL is in declaration order");
        }
    }

    /// Every CC0 id has its file in `assets/audio/cc0/` and a line in `assets/audio/CREDITS.md`,
    /// and nothing else sits in that folder (a stray file would ship uncredited).
    #[test]
    fn cc0_sounds_are_on_disk_and_credited() {
        let root = env!("CARGO_MANIFEST_DIR");
        let credits = std::fs::read_to_string(format!("{root}/assets/audio/CREDITS.md"))
            .expect("assets/audio/CREDITS.md");
        let ids: Vec<&str> = Sfx::ALL
            .iter()
            .filter(|s| s.is_cc0())
            .map(|s| s.id())
            .chain(AmbLoop::ALL.iter().map(|a| a.id()))
            .collect();
        assert!(ids.len() >= 30);
        for id in &ids {
            let found = sound_paths(id)
                .iter()
                .filter(|p| p.starts_with(CC0_DIR))
                .any(|p| std::path::Path::new(&format!("{root}/{p}")).exists());
            assert!(found, "{id}: no file in {CC0_DIR}");
            assert!(
                credits.contains(&format!("`{id}.")),
                "{id}: not in CREDITS.md"
            );
            assert!(
                crate::content::SFX_IDS.contains(id),
                "{id}: not in content::SFX_IDS"
            );
        }
        for e in std::fs::read_dir(format!("{root}/{CC0_DIR}")).expect("cc0 dir") {
            let name = e.expect("entry").file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue; // .DS_Store and friends (git-ignored)
            }
            let stem = name.rsplit_once('.').map_or(name.as_str(), |(s, _)| s);
            assert!(ids.contains(&stem), "{name}: no Sfx / AmbLoop uses it");
        }
        // the original sounds stay in the generated pack
        assert!(!Sfx::DoorBell.is_cc0() && !Sfx::TrainPass.is_cc0());
    }

    #[test]
    fn generated_pack_comes_before_cc0() {
        let p = sound_paths("sfx_oven_ding");
        assert_eq!(p[0], "assets/audio/sfx/sfx_oven_ding.wav");
        assert!(p.iter().position(|x| x.starts_with(CC0_DIR)) > Some(0));
        assert_eq!(
            sound_paths("amb_storm")[0],
            "assets/audio/sfx/amb_storm.ogg"
        );
    }

    #[test]
    fn ambience_loops_fade_and_never_stack() {
        let mut m = Mix::default();
        // three pots ask for the soup: still one loop, at the loudest ask
        for want in [0.3, 1.0, 0.5] {
            let w = &mut m.amb_want[AmbLoop::Soup as usize];
            *w = w.max(want);
        }
        m.step(0.3, false);
        let g = m.amb_gain(AmbLoop::Soup);
        assert!(
            g > 0.0 && g < m.amb_gain_full(AmbLoop::Soup),
            "fades in, {g}"
        );
        for _ in 0..120 {
            m.step(1.0 / 60.0, false);
        }
        assert!((m.amb_level[AmbLoop::Soup as usize] - 1.0).abs() < 1e-4);
        // nobody asks any more: it fades out, not cut
        m.amb_want = [0.0; AMB_COUNT];
        m.step(0.4, false);
        let l = m.amb_level[AmbLoop::Soup as usize];
        assert!(l > 0.5 && l < 1.0, "{l}");
        m.step(2.0, false);
        assert_eq!(m.amb_gain(AmbLoop::Soup), 0.0);
        // a cutscene holds the storm wind until it lets go
        m.amb_hold[AmbLoop::Storm as usize] = 0.8;
        m.step(3.0, false);
        assert!((m.amb_level[AmbLoop::Storm as usize] - 0.8).abs() < 1e-4);
        // the player's volume and the mute still rule
        m.sfx_volume = 0.0;
        assert_eq!(m.amb_gain(AmbLoop::Storm), 0.0);
        m.sfx_volume = 0.8;
        m.muted = true;
        assert_eq!(m.amb_gain(AmbLoop::Storm), 0.0);
    }

    #[test]
    fn cutscene_requests_hold_a_loop() {
        let mut a = Audio::silent();
        a.request(&AudioRequest::Sfx {
            id: "amb_storm".into(),
            volume: 0.7,
        });
        assert_eq!(a.mix.amb_hold[AmbLoop::Storm as usize], 0.7);
        a.want_amb(AmbLoop::CatPurr, 0.4);
        a.want_amb(AmbLoop::CatPurr, 0.2);
        assert_eq!(a.mix.amb_want[AmbLoop::CatPurr as usize], 0.4);
        a.update(0.5, false);
        // asks last one frame; a hold stays
        assert_eq!(a.mix.amb_want, [0.0; AMB_COUNT]);
        assert!(a.mix.amb_level[AmbLoop::CatPurr as usize] > 0.0);
        a.request(&AudioRequest::Sfx {
            id: "amb_storm".into(),
            volume: 0.0,
        });
        assert_eq!(a.mix.amb_hold[AmbLoop::Storm as usize], 0.0);
    }
}
