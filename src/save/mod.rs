//! Saved games (owner: integration).
//!
//! One RON file per slot: `auto.ron` (autosave at the start of every day and on quit) and
//! `slot1.ron` … `slot3.ron` (manual). Each file carries a format [`SAVE_VERSION`], a small
//! [`SaveMeta`] for the menus, a few presentation-side values ([`SessionExtra`]) and the
//! sim's own save string (`World::save_to_string`, versioned by the sim).
//!
//! Folder: `SHIFTELLA_SAVES=<dir>` (tests) → `~/Library/Application Support/Shiftella/saves/`
//! (macOS; `%APPDATA%\Shiftella\saves`, `~/.config/shiftella/saves` elsewhere) → `./saves/`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Version of this file format (not of the sim state inside).
pub const SAVE_VERSION: u32 = 1;
/// Manual slots 1..=3.
pub const MANUAL_SLOTS: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Slot {
    Auto,
    Manual(u8),
}

impl Slot {
    pub fn all() -> Vec<Slot> {
        let mut v = vec![Slot::Auto];
        v.extend((1..=MANUAL_SLOTS).map(Slot::Manual));
        v
    }

    pub fn file_name(self) -> String {
        match self {
            Slot::Auto => "auto.ron".into(),
            Slot::Manual(n) => format!("slot{n}.ron"),
        }
    }
}

/// What the menus show about a save without loading the world.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveMeta {
    /// `"story"` / `"sandbox"`.
    pub mode: String,
    pub night: u32,
    /// `"day"` or the clock (`"01:40"`).
    pub clock: String,
    pub cafe: String,
    pub difficulty: String,
    /// Unix seconds.
    pub saved_at: u64,
    /// The story reached an ending (the save stays loadable).
    pub finished: bool,
    pub money: i64,
}

/// Presentation-side state that the sim does not keep.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SessionExtra {
    /// The opening was shown before Day 1; its choices go to the sim when it asks.
    pub opening_seen: Option<Vec<usize>>,
    /// The ending scene already played (credits next).
    pub ending_played: bool,
    /// Camera centre / zoom (tiles, multiplier).
    pub camera: Option<(f32, f32, f32)>,
    /// First-time coach marks (`None` = not decided yet for this game).
    #[serde(default)]
    pub tutorial: Option<TutorialProgress>,
}

/// Where the first-time tutorial is (see `screens::game::tutorial`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TutorialProgress {
    pub active: bool,
    /// Index into `tutorial::STEPS`.
    pub step: u32,
    /// The panel of the current step has been opened (it finishes when closed again).
    #[serde(default)]
    pub panel_seen: bool,
    /// Night coach marks (`tutorial.ron` ids) already done or dismissed.
    #[serde(default)]
    pub coach_done: Vec<String>,
    /// The night coach marks were skipped for good.
    #[serde(default)]
    pub coach_skipped: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveFile {
    pub version: u32,
    pub meta: SaveMeta,
    pub extra: SessionExtra,
    /// `World::save_to_string()`.
    pub world: String,
}

impl SaveFile {
    pub fn new(meta: SaveMeta, extra: SessionExtra, world: String) -> SaveFile {
        SaveFile {
            version: SAVE_VERSION,
            meta,
            extra,
            world,
        }
    }

    pub fn to_ron(&self) -> Result<String, String> {
        let cfg = ron::ser::PrettyConfig::new().depth_limit(2);
        ron::ser::to_string_pretty(self, cfg).map_err(|e| e.to_string())
    }

    pub fn from_ron(text: &str) -> Result<SaveFile, String> {
        #[derive(Deserialize)]
        struct VersionOnly {
            version: u32,
        }
        // Check the version first so an old file gives a clear message.
        if let Ok(v) = ron::from_str::<VersionOnly>(text)
            && v.version != SAVE_VERSION
        {
            return Err(format!(
                "save format {} not supported (expected {SAVE_VERSION})",
                v.version
            ));
        }
        ron::from_str(text).map_err(|e| e.to_string())
    }
}

/// Unix time now (seconds).
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// The save folder (see module docs). Not created until something is written.
pub fn saves_dir() -> PathBuf {
    if let Ok(p) = std::env::var("SHIFTELLA_SAVES")
        && !p.trim().is_empty()
    {
        return PathBuf::from(p);
    }
    platform_dir()
        .map(|d| d.join("saves"))
        .unwrap_or_else(|| PathBuf::from("saves"))
}

fn platform_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if cfg!(target_os = "macos") {
        return home.map(|h| h.join("Library/Application Support/Shiftella"));
    }
    if cfg!(target_os = "windows") {
        return std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("Shiftella"));
    }
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| home.map(|h| h.join(".config")))
        .map(|d| d.join("shiftella"))
}

/// A folder of save slots.
#[derive(Debug, Clone)]
pub struct SaveStore {
    pub dir: PathBuf,
}

impl Default for SaveStore {
    fn default() -> Self {
        SaveStore { dir: saves_dir() }
    }
}

impl SaveStore {
    pub fn at(dir: impl Into<PathBuf>) -> SaveStore {
        SaveStore { dir: dir.into() }
    }

    pub fn path(&self, slot: Slot) -> PathBuf {
        self.dir.join(slot.file_name())
    }

    /// Writes a slot; falls back to `./saves/` if the main folder can't be written.
    pub fn write(&self, slot: Slot, file: &SaveFile) -> Result<PathBuf, String> {
        let text = file.to_ron()?;
        match write_to(&self.path(slot), &text) {
            Ok(()) => Ok(self.path(slot)),
            Err(e) => {
                let local = Path::new("saves").join(slot.file_name());
                write_to(&local, &text)
                    .map(|_| local)
                    .map_err(|e2| format!("{e}; {e2}"))
            }
        }
    }

    pub fn read(&self, slot: Slot) -> Result<SaveFile, String> {
        let p = self.path(slot);
        let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        SaveFile::from_ron(&text)
    }

    /// Meta of every slot that holds a readable save.
    pub fn list(&self) -> Vec<(Slot, SaveMeta)> {
        Slot::all()
            .into_iter()
            .filter_map(|s| self.read(s).ok().map(|f| (s, f.meta)))
            .collect()
    }

    /// The most recently written slot (for Continue).
    pub fn latest(&self) -> Option<(Slot, SaveMeta)> {
        self.list().into_iter().max_by_key(|(_, m)| m.saved_at)
    }

    pub fn delete(&self, slot: Slot) -> Result<(), String> {
        let p = self.path(slot);
        if p.exists() {
            std::fs::remove_file(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        }
        Ok(())
    }
}

fn write_to(path: &Path, text: &str) -> Result<(), String> {
    if let Some(dir) = path.parent()
        && !dir.as_os_str().is_empty()
    {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    // write next to it, then rename: a crash never leaves half a save
    let tmp = path.with_extension("ron.tmp");
    std::fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(night: u32, at: u64) -> SaveMeta {
        SaveMeta {
            mode: "story".into(),
            night,
            clock: "day".into(),
            cafe: "Café Nocturne".into(),
            difficulty: "cozy".into(),
            saved_at: at,
            finished: false,
            money: 12_480,
        }
    }

    fn temp_store(name: &str) -> SaveStore {
        let dir = std::env::temp_dir().join(format!("shiftella-save-test-{name}-{}", now()));
        let _ = std::fs::remove_dir_all(&dir);
        SaveStore::at(dir)
    }

    #[test]
    fn round_trip_and_latest() {
        let store = temp_store("rt");
        let a = SaveFile::new(meta(3, 100), SessionExtra::default(), "(world)".into());
        let b = SaveFile::new(
            meta(5, 200),
            SessionExtra {
                opening_seen: Some(vec![1]),
                ..Default::default()
            },
            "(w2)".into(),
        );
        store.write(Slot::Auto, &a).unwrap();
        store.write(Slot::Manual(2), &b).unwrap();
        assert_eq!(store.read(Slot::Auto).unwrap(), a);
        assert_eq!(store.list().len(), 2);
        assert_eq!(store.latest().unwrap().0, Slot::Manual(2));
        store.delete(Slot::Manual(2)).unwrap();
        assert_eq!(store.latest().unwrap().0, Slot::Auto);
        let _ = std::fs::remove_dir_all(&store.dir);
    }

    #[test]
    fn rejects_other_versions() {
        let mut f = SaveFile::new(meta(1, 1), SessionExtra::default(), String::new());
        f.version = 99;
        let text = f.to_ron().unwrap();
        let err = SaveFile::from_ron(&text).unwrap_err();
        assert!(err.contains("99"), "{err}");
    }
}
