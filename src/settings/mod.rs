//! Player settings: language, volumes, UI scale, text speed, fullscreen, pausing.
//! Owner: cutscene agent.
//!
//! Stored as a small RON file in the user's config dir:
//! * macOS — `~/Library/Application Support/Shiftella/settings.ron`
//! * Windows — `%APPDATA%\Shiftella\settings.ron`
//! * Linux — `$XDG_CONFIG_HOME/shiftella/settings.ron` (or `~/.config/shiftella/…`)
//! * fallback — `./settings.ron`. `SHIFTELLA_SETTINGS=<path>` overrides everything (tests, demos).
//!
//! ```ignore
//! let mut settings = Settings::load();          // never fails: defaults on any problem
//! if settings.language.is_none() { /* first launch → language picker */ }
//! settings.music_volume = 0.5;
//! settings.save().ok();
//! ```
//! In-game the same screen is [`settings_panel`] (see `panel.rs`).

mod panel;

pub use panel::{PANEL_SIZE, SettingsResponse, panel_rect, settings_panel};

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::i18n::{I18n, Lang};

/// The translations plus the current language — what every screen needs to show text.
#[derive(Clone, Copy)]
pub struct Locale<'a> {
    pub i18n: &'a I18n,
    pub lang: Lang,
}

impl<'a> Locale<'a> {
    pub fn new(i18n: &'a I18n, lang: Lang) -> Self {
        Locale { i18n, lang }
    }
    /// Text for `key` (falls back to English, then the key itself).
    pub fn tr(&self, key: &'a str) -> &'a str {
        self.i18n.tr(self.lang, key)
    }
    /// Like [`Locale::tr`] for keys built at runtime.
    pub fn tr_owned(&self, key: &str) -> String {
        self.i18n
            .get(self.lang, key)
            .or_else(|| self.i18n.get(Lang::En, key))
            .unwrap_or(key)
            .to_string()
    }
    pub fn trf(&self, key: &str, args: &[(&str, &str)]) -> String {
        self.i18n.trf(self.lang, key, args)
    }
    /// The key exists in this language or in English.
    pub fn has(&self, key: &str) -> bool {
        self.i18n.has(self.lang, key) || self.i18n.has(Lang::En, key)
    }
    /// `key` if it exists, else `fallback` (e.g. a content key with a UI-side fallback).
    pub fn tr_or(&self, key: &'a str, fallback: &'a str) -> &'a str {
        if self.has(key) {
            self.tr(key)
        } else {
            self.tr(fallback)
        }
    }
}

/// Loads `assets/i18n/{cs,en,uk,ru}.ron`; if those are not generated yet, compiles the
/// side-by-side sources in `<dir>/src/*.tr` in memory.
pub fn load_i18n(dir: &str) -> Result<I18n, String> {
    match I18n::load(dir) {
        Ok(i) => Ok(i),
        Err(first) => {
            let tables = crate::i18n::source::compile_dir(&format!("{dir}/src"))
                .map_err(|e| format!("{first}; {e}"))?;
            Ok(I18n::from_tables(tables.map(|t| {
                t.into_iter().collect::<std::collections::HashMap<_, _>>()
            })))
        }
    }
}

/// Typewriter speed of dialogue text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TextSpeed {
    Slow,
    #[default]
    Normal,
    Fast,
    /// Whole line at once.
    Instant,
}

impl TextSpeed {
    pub const ALL: [TextSpeed; 4] = [
        TextSpeed::Slow,
        TextSpeed::Normal,
        TextSpeed::Fast,
        TextSpeed::Instant,
    ];

    /// Characters per second for [`crate::ui::DialogueBox::chars_per_sec`] (0 = instant).
    pub fn chars_per_sec(self) -> f32 {
        match self {
            TextSpeed::Slow => 30.0,
            TextSpeed::Normal => 55.0,
            TextSpeed::Fast => 90.0,
            TextSpeed::Instant => 0.0,
        }
    }

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|s| *s == self).unwrap_or(1)
    }

    /// i18n key of the label (`mainmenu.settings.text_speed.<id>`).
    pub fn label_key(self) -> &'static str {
        match self {
            TextSpeed::Slow => "mainmenu.settings.text_speed.slow",
            TextSpeed::Normal => "mainmenu.settings.text_speed.normal",
            TextSpeed::Fast => "mainmenu.settings.text_speed.fast",
            TextSpeed::Instant => "mainmenu.settings.text_speed.instant",
        }
    }
}

/// Everything the player can change. Unknown / missing fields fall back to defaults,
/// so old files keep loading after new options appear.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// `None` until the first-launch language picker has been answered.
    pub language: Option<Lang>,
    /// 0..1
    pub music_volume: f32,
    /// 0..1 (sound effects + ambience)
    pub sfx_volume: f32,
    /// UI scale, 0.9–1.3 (applied with `Ui::set_scale`).
    pub ui_scale: f32,
    pub text_speed: TextSpeed,
    /// Cutscene lines advance on their own after a reading pause.
    pub auto_advance: bool,
    pub fullscreen: bool,
    /// Pause the clock when an important event pops up (off by default — cozy flow).
    pub pause_on_events: bool,
    /// Coach marks on the first day and night of a new game (turned off once finished).
    pub show_tutorial: bool,
    /// Relaxed mini-games (design §2.0): wider sweet spots, a quality floor, a Done button.
    pub relaxed_minigames: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            language: None,
            music_volume: 0.7,
            sfx_volume: 0.8,
            ui_scale: 1.0,
            text_speed: TextSpeed::Normal,
            auto_advance: false,
            fullscreen: false,
            pause_on_events: false,
            show_tutorial: true,
            relaxed_minigames: false,
        }
    }
}

pub const UI_SCALE_MIN: f32 = 0.9;
pub const UI_SCALE_MAX: f32 = 1.3;

impl Settings {
    /// The chosen language, or English before the picker was answered.
    pub fn lang(&self) -> Lang {
        self.language.unwrap_or(Lang::En)
    }

    /// First launch: the language picker must be shown.
    pub fn needs_language(&self) -> bool {
        self.language.is_none()
    }

    /// Clamps every value into its valid range (after loading or editing).
    pub fn sanitize(&mut self) {
        let fix = |v: f32, lo: f32, hi: f32, d: f32| {
            if v.is_finite() { v.clamp(lo, hi) } else { d }
        };
        self.music_volume = fix(self.music_volume, 0.0, 1.0, 0.7);
        self.sfx_volume = fix(self.sfx_volume, 0.0, 1.0, 0.8);
        self.ui_scale = fix(self.ui_scale, UI_SCALE_MIN, UI_SCALE_MAX, 1.0);
    }

    /// Where settings live on this machine (see the module docs).
    pub fn path() -> PathBuf {
        if let Ok(p) = std::env::var("SHIFTELLA_SETTINGS")
            && !p.trim().is_empty()
        {
            return PathBuf::from(p);
        }
        config_dir()
            .map(|d| d.join("settings.ron"))
            .unwrap_or_else(|| PathBuf::from("settings.ron"))
    }

    /// Loads from [`Settings::path`], then `./settings.ron`; defaults when both are missing
    /// or broken. Never fails.
    pub fn load() -> Settings {
        let primary = Self::path();
        if let Some(s) = Self::load_from(&primary) {
            return s;
        }
        Self::load_from(Path::new("settings.ron")).unwrap_or_default()
    }

    /// Reads one file. `None` if it is missing or unreadable.
    pub fn load_from(path: &Path) -> Option<Settings> {
        let text = std::fs::read_to_string(path).ok()?;
        match Self::from_ron(&text) {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!("warning: {}: {e} — using default settings", path.display());
                None
            }
        }
    }

    pub fn from_ron(text: &str) -> Result<Settings, String> {
        let mut s: Settings = ron::from_str(text).map_err(|e| e.to_string())?;
        s.sanitize();
        Ok(s)
    }

    pub fn to_ron(&self) -> String {
        let cfg = ron::ser::PrettyConfig::new().depth_limit(2);
        ron::ser::to_string_pretty(self, cfg).unwrap_or_default()
    }

    /// Writes to [`Settings::path`] (creating the folder); falls back to `./settings.ron`.
    /// Returns the path that was written.
    pub fn save(&self) -> Result<PathBuf, String> {
        let primary = Self::path();
        match self.save_to(&primary) {
            Ok(()) => Ok(primary),
            Err(e) => {
                let local = PathBuf::from("settings.ron");
                self.save_to(&local)
                    .map(|_| local)
                    .map_err(|e2| format!("{e}; {e2}"))
            }
        }
    }

    pub fn save_to(&self, path: &Path) -> Result<(), String> {
        if let Some(dir) = path.parent()
            && !dir.as_os_str().is_empty()
        {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(path, self.to_ron()).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Applies the window-related options (fullscreen) to the macroquad window.
    pub fn apply_window(&self) {
        // an invisible automation run never goes fullscreen (crate::platform)
        if crate::platform::automation_hidden() {
            return;
        }
        macroquad::window::set_fullscreen(self.fullscreen);
    }
}

fn config_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if cfg!(target_os = "macos") {
        return home.map(|h| h.join("Library/Application Support/Shiftella"));
    }
    if cfg!(target_os = "windows") {
        return std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("Shiftella"));
    }
    if let Some(x) = std::env::var_os("XDG_CONFIG_HOME").filter(|x| !x.is_empty()) {
        return Some(PathBuf::from(x).join("shiftella"));
    }
    home.map(|h| h.join(".config/shiftella"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_through_ron() {
        let s = Settings {
            language: Some(Lang::Uk),
            music_volume: 0.25,
            sfx_volume: 0.5,
            ui_scale: 1.2,
            text_speed: TextSpeed::Fast,
            auto_advance: true,
            fullscreen: true,
            pause_on_events: true,
            show_tutorial: false,
            relaxed_minigames: true,
        };
        let back = Settings::from_ron(&s.to_ron()).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn round_trip_through_a_file() {
        let dir = std::env::temp_dir().join(format!("shiftella-settings-{}", std::process::id()));
        let path = dir.join("nested/settings.ron");
        let s = Settings {
            language: Some(Lang::Ru),
            ..Default::default()
        };
        s.save_to(&path).unwrap();
        assert_eq!(Settings::load_from(&path), Some(s));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_fields_and_bad_values_fall_back() {
        let s = Settings::from_ron("(language: Some(Cs), ui_scale: 7.0)").unwrap();
        assert_eq!(s.language, Some(Lang::Cs));
        assert_eq!(s.ui_scale, UI_SCALE_MAX);
        assert_eq!(s.music_volume, Settings::default().music_volume);
        assert!(!s.pause_on_events, "pause on events is off by default");
        assert!(Settings::from_ron("not ron").is_err());
    }

    #[test]
    fn a_saved_slovak_choice_becomes_czech() {
        // Slovak was dropped (2026-11-26); older settings files still say `Some(Sk)`.
        let s = Settings::from_ron("(language: Some(Sk), music_volume: 0.3)").unwrap();
        assert_eq!(s.language, Some(Lang::Cs));
        assert_eq!(s.music_volume, 0.3);
        assert!(!s.needs_language(), "no second trip through the picker");
        let ru = Settings::from_ron("(language: Some(Ru))").unwrap();
        assert_eq!(ru.language, Some(Lang::Ru));
    }

    #[test]
    fn first_launch_needs_the_language_picker() {
        let s = Settings::default();
        assert!(s.needs_language());
        assert_eq!(s.lang(), Lang::En);
    }

    #[test]
    fn text_speeds_map_to_typewriter_rates() {
        assert_eq!(TextSpeed::Instant.chars_per_sec(), 0.0);
        assert!(TextSpeed::Slow.chars_per_sec() < TextSpeed::Fast.chars_per_sec());
        for s in TextSpeed::ALL {
            assert_eq!(TextSpeed::ALL[s.index()], s);
        }
    }
}
