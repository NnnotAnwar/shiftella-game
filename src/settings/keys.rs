//! Key rebinding for the main actions (Settings → Controls).
//!
//! The bindings are saved in the settings file as a small map of *changes* from the defaults
//! (`keys: {"crew": "V"}`), so new actions and new defaults reach old files untouched. The
//! game reads the live bindings through [`key`] / [`pressed`] / [`label`] (installed once a
//! frame by the app from the settings, see [`install`]); texts that name a key use tokens
//! (`{key.crew}`, `{Key.pause}` capitalised) that [`crate::i18n::I18n::apply_keys`] fills.
//!
//! ```ignore
//! if keys::pressed(ui, Action::Crew) { /* toggle the crew panel */ }
//! Button::new("Crew").shortcut(&keys::label(Action::Crew));
//! ```

use std::cell::RefCell;
use std::collections::BTreeMap;

use macroquad::prelude::KeyCode;
use serde::{Deserialize, Serialize};

/// A rebindable action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Action {
    Crew,
    Duties,
    App,
    Build,
    Log,
    Book,
    Journal,
    Photo,
    Jukebox,
    Moment,
    Chore,
    Greet,
    Queue,
    /// "Open the doors" in the preparation, "Close up" in the closing half hour.
    Doors,
    Pause,
    Speed1,
    Speed2,
    Speed3,
    Help,
    Mute,
}

impl Action {
    /// In the order of the Controls list.
    pub const ALL: [Action; 20] = [
        Action::Crew,
        Action::Duties,
        Action::App,
        Action::Build,
        Action::Log,
        Action::Book,
        Action::Journal,
        Action::Photo,
        Action::Jukebox,
        Action::Moment,
        Action::Chore,
        Action::Greet,
        Action::Queue,
        Action::Doors,
        Action::Pause,
        Action::Speed1,
        Action::Speed2,
        Action::Speed3,
        Action::Help,
        Action::Mute,
    ];

    /// Stable id (settings file, i18n tokens `{key.<id>}`, label `access.action.<id>`).
    pub fn id(self) -> &'static str {
        match self {
            Action::Crew => "crew",
            Action::Duties => "duties",
            Action::App => "app",
            Action::Build => "build",
            Action::Log => "log",
            Action::Book => "book",
            Action::Journal => "journal",
            Action::Photo => "photo",
            Action::Jukebox => "jukebox",
            Action::Moment => "moment",
            Action::Chore => "chore",
            Action::Greet => "greet",
            Action::Queue => "queue",
            Action::Doors => "doors",
            Action::Pause => "pause",
            Action::Speed1 => "speed1",
            Action::Speed2 => "speed2",
            Action::Speed3 => "speed3",
            Action::Help => "help",
            Action::Mute => "mute",
        }
    }

    pub fn from_id(id: &str) -> Option<Action> {
        Action::ALL.iter().copied().find(|a| a.id() == id)
    }

    /// i18n key of the row label in Settings → Controls.
    pub fn label_key(self) -> String {
        format!("access.action.{}", self.id())
    }

    pub fn default_key(self) -> KeyCode {
        match self {
            Action::Crew => KeyCode::C,
            Action::Duties => KeyCode::H,
            Action::App => KeyCode::T,
            Action::Build => KeyCode::B,
            Action::Log => KeyCode::L,
            Action::Book => KeyCode::K,
            Action::Journal => KeyCode::J,
            Action::Photo => KeyCode::P,
            Action::Jukebox => KeyCode::V,
            Action::Moment => KeyCode::F,
            Action::Chore => KeyCode::E,
            Action::Greet => KeyCode::G,
            Action::Queue => KeyCode::Q,
            Action::Doors => KeyCode::O,
            Action::Pause => KeyCode::Space,
            Action::Speed1 => KeyCode::Key1,
            Action::Speed2 => KeyCode::Key2,
            Action::Speed3 => KeyCode::Key3,
            Action::Help => KeyCode::F1,
            Action::Mute => KeyCode::M,
        }
    }
}

/// The player's bindings: only the actions moved away from their default key.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct KeyBindings {
    /// action id → key name ([`key_name`]); unknown ids / names are ignored on read.
    changed: BTreeMap<String, String>,
}

impl KeyBindings {
    /// The key bound to `a`.
    pub fn key(&self, a: Action) -> KeyCode {
        self.changed
            .get(a.id())
            .and_then(|n| key_from_name(n))
            .filter(|k| bindable(*k))
            .unwrap_or_else(|| a.default_key())
    }

    /// Binds `a` to `k` (back to the default drops the entry). Reserved keys are refused.
    pub fn set(&mut self, a: Action, k: KeyCode) -> bool {
        if !bindable(k) {
            return false;
        }
        if k == a.default_key() {
            self.changed.remove(a.id());
        } else if let Some(n) = key_name(k) {
            self.changed.insert(a.id().to_string(), n.to_string());
        } else {
            return false;
        }
        true
    }

    /// Everything back to the defaults.
    pub fn reset(&mut self) {
        self.changed.clear();
    }

    pub fn is_default(&self) -> bool {
        Action::ALL.iter().all(|a| self.key(*a) == a.default_key())
    }

    /// Another action on the same key as `a` (the first one in [`Action::ALL`] order).
    pub fn conflict_of(&self, a: Action) -> Option<Action> {
        let k = self.key(a);
        Action::ALL
            .iter()
            .copied()
            .find(|b| *b != a && self.key(*b) == k)
    }

    /// Every pair of actions that share a key (each pair once).
    pub fn conflicts(&self) -> Vec<(Action, Action)> {
        let mut out = Vec::new();
        for (i, a) in Action::ALL.iter().enumerate() {
            for b in &Action::ALL[i + 1..] {
                if self.key(*a) == self.key(*b) {
                    out.push((*a, *b));
                }
            }
        }
        out
    }

    /// Action id → keycap badge, for every action (what
    /// [`crate::i18n::I18n::apply_keys`] fills the `{key.<id>}` tokens with).
    pub fn badges(&self) -> BTreeMap<&'static str, String> {
        Action::ALL
            .iter()
            .map(|a| (a.id(), badge(self.key(*a))))
            .collect()
    }
}

/// Keys that keep a fixed meaning and can't be bound: Esc / Enter, the camera (WASD,
/// arrows, Home), the ticket rail ([ ]), Backspace (cancel), build mode (R, X, Tab) and the
/// modifiers. `None` = free to bind; otherwise the i18n key of the reason.
pub fn reserved_reason(k: KeyCode) -> Option<&'static str> {
    use KeyCode::*;
    Some(match k {
        Escape | Enter | KpEnter => "access.reserved.menu",
        W | A | S | D | Up | Down | Left | Right | Home => "access.reserved.camera",
        LeftBracket | RightBracket | Backspace => "access.reserved.rail",
        R | X | Tab => "access.reserved.build",
        LeftShift | RightShift | LeftControl | RightControl | LeftAlt | RightAlt | LeftSuper
        | RightSuper | CapsLock | F3 => "access.reserved.other",
        _ => return None,
    })
}

/// A key the player may bind an action to.
pub fn bindable(k: KeyCode) -> bool {
    reserved_reason(k).is_none() && key_name(k).is_some()
}

/// Saved name of a key (`"C"`, `"F1"`, `"Space"`, `"1"`); `None` for keys we don't offer.
pub fn key_name(k: KeyCode) -> Option<&'static str> {
    KEY_NAMES.iter().find(|(c, _)| *c == k).map(|(_, n)| *n)
}

pub fn key_from_name(name: &str) -> Option<KeyCode> {
    KEY_NAMES
        .iter()
        .find(|(_, n)| n.eq_ignore_ascii_case(name))
        .map(|(c, _)| *c)
}

/// How a key is written on a keycap badge (short, the same in every language: "Space").
pub fn badge(k: KeyCode) -> String {
    match key_name(k) {
        Some(n) if n.starts_with("Num") => format!("Num {}", &n[3..]),
        Some("Comma") => ",".into(),
        Some("Period") => ".".into(),
        Some("Minus") => "-".into(),
        Some("Equal") => "=".into(),
        Some("Slash") => "/".into(),
        Some("Semicolon") => ";".into(),
        Some("Apostrophe") => "'".into(),
        Some("Backslash") => "\\".into(),
        Some("Grave") => "`".into(),
        Some("PageUp") => "PgUp".into(),
        Some("PageDown") => "PgDn".into(),
        Some("Delete") => "Del".into(),
        Some("Insert") => "Ins".into(),
        Some(n) => n.to_string(),
        None => "?".into(),
    }
}

const KEY_NAMES: &[(KeyCode, &str)] = {
    use KeyCode::*;
    &[
        (A, "A"),
        (B, "B"),
        (C, "C"),
        (D, "D"),
        (E, "E"),
        (F, "F"),
        (G, "G"),
        (H, "H"),
        (I, "I"),
        (J, "J"),
        (K, "K"),
        (L, "L"),
        (M, "M"),
        (N, "N"),
        (O, "O"),
        (P, "P"),
        (Q, "Q"),
        (R, "R"),
        (S, "S"),
        (T, "T"),
        (U, "U"),
        (V, "V"),
        (W, "W"),
        (X, "X"),
        (Y, "Y"),
        (Z, "Z"),
        (Key0, "0"),
        (Key1, "1"),
        (Key2, "2"),
        (Key3, "3"),
        (Key4, "4"),
        (Key5, "5"),
        (Key6, "6"),
        (Key7, "7"),
        (Key8, "8"),
        (Key9, "9"),
        (F1, "F1"),
        (F2, "F2"),
        (F4, "F4"),
        (F5, "F5"),
        (F6, "F6"),
        (F7, "F7"),
        (F8, "F8"),
        (F9, "F9"),
        (F10, "F10"),
        (F11, "F11"),
        (F12, "F12"),
        (Space, "Space"),
        (Comma, "Comma"),
        (Period, "Period"),
        (Minus, "Minus"),
        (Equal, "Equal"),
        (Slash, "Slash"),
        (Semicolon, "Semicolon"),
        (Apostrophe, "Apostrophe"),
        (Backslash, "Backslash"),
        (GraveAccent, "Grave"),
        (Insert, "Insert"),
        (Delete, "Delete"),
        (PageUp, "PageUp"),
        (PageDown, "PageDown"),
        (End, "End"),
        (Kp0, "Num0"),
        (Kp1, "Num1"),
        (Kp2, "Num2"),
        (Kp3, "Num3"),
        (Kp4, "Num4"),
        (Kp5, "Num5"),
        (Kp6, "Num6"),
        (Kp7, "Num7"),
        (Kp8, "Num8"),
        (Kp9, "Num9"),
        // reserved (named so a saved file can mention them; never bindable)
        (Escape, "Escape"),
        (Enter, "Enter"),
        (Tab, "Tab"),
        (Backspace, "Backspace"),
        (Home, "Home"),
        (Up, "Up"),
        (Down, "Down"),
        (Left, "Left"),
        (Right, "Right"),
        (LeftBracket, "LeftBracket"),
        (RightBracket, "RightBracket"),
    ]
};

thread_local! {
    static CURRENT: RefCell<KeyBindings> = RefCell::new(KeyBindings::default());
}

/// Makes `b` the live bindings (the app does this from the settings every frame).
pub fn install(b: &KeyBindings) {
    CURRENT.with(|c| {
        if *c.borrow() != *b {
            *c.borrow_mut() = b.clone();
        }
    });
}

/// The live key of `a`.
pub fn key(a: Action) -> KeyCode {
    CURRENT.with(|c| c.borrow().key(a))
}

/// The live key of `a` as a keycap badge ("C", "F1", "Space").
pub fn label(a: Action) -> String {
    badge(key(a))
}

/// `a`'s key was pressed this frame (and no widget took it).
pub fn pressed(ui: &crate::ui::Ui, a: Action) -> bool {
    ui.key(key(a))
}

/// The duties board's old key P (the former priorities panel) still works while P isn't
/// given to anything but the camera (photo mode at night).
pub fn legacy_duties_p() -> bool {
    CURRENT.with(|c| {
        let b = c.borrow();
        Action::ALL
            .iter()
            .all(|a| *a == Action::Photo || b.key(*a) != KeyCode::P)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_have_no_conflicts_and_are_bindable() {
        let b = KeyBindings::default();
        assert!(b.conflicts().is_empty(), "{:?}", b.conflicts());
        for a in Action::ALL {
            assert!(bindable(a.default_key()), "{a:?}");
            assert_eq!(b.key(a), a.default_key());
            assert_eq!(Action::from_id(a.id()), Some(a));
        }
        assert!(b.is_default());
    }

    #[test]
    fn rebinding_round_trips_through_ron() {
        let mut b = KeyBindings::default();
        assert!(b.set(Action::Crew, KeyCode::N));
        assert!(b.set(Action::Pause, KeyCode::F5));
        assert!(b.set(Action::Help, KeyCode::F1), "back to default");
        let text = ron::to_string(&b).unwrap();
        assert!(text.contains("crew") && !text.contains("help"), "{text}");
        let back: KeyBindings = ron::from_str(&text).unwrap();
        assert_eq!(back, b);
        assert_eq!(back.key(Action::Crew), KeyCode::N);
        assert_eq!(back.key(Action::Pause), KeyCode::F5);
        assert_eq!(back.key(Action::Build), KeyCode::B);
        // setting the default again removes the change
        let mut c = back.clone();
        c.set(Action::Crew, KeyCode::C);
        c.set(Action::Pause, KeyCode::Space);
        assert!(c.is_default());
        b.reset();
        assert!(b.is_default());
    }

    #[test]
    fn conflicts_are_reported_both_ways() {
        let mut b = KeyBindings::default();
        assert!(b.set(Action::Log, KeyCode::C));
        assert_eq!(b.conflicts(), vec![(Action::Crew, Action::Log)]);
        assert_eq!(b.conflict_of(Action::Crew), Some(Action::Log));
        assert_eq!(b.conflict_of(Action::Log), Some(Action::Crew));
        assert_eq!(b.conflict_of(Action::Build), None);
        b.set(Action::Crew, KeyCode::L);
        assert!(b.conflicts().is_empty(), "a swap clears it");
    }

    #[test]
    fn reserved_keys_are_refused_and_junk_is_ignored() {
        let mut b = KeyBindings::default();
        for k in [
            KeyCode::Escape,
            KeyCode::W,
            KeyCode::Left,
            KeyCode::R,
            KeyCode::Enter,
        ] {
            assert!(!b.set(Action::Crew, k), "{k:?}");
        }
        assert!(b.is_default());
        let junk: KeyBindings =
            ron::from_str(r#"{"crew": "W", "nonsense": "Q", "build": "NoSuchKey", "log": "q"}"#)
                .unwrap();
        assert_eq!(
            junk.key(Action::Crew),
            KeyCode::C,
            "a reserved key falls back"
        );
        assert_eq!(junk.key(Action::Build), KeyCode::B);
        assert_eq!(
            junk.key(Action::Log),
            KeyCode::Q,
            "names are case-insensitive"
        );
    }

    #[test]
    fn badges_and_names() {
        assert_eq!(badge(KeyCode::Space), "Space");
        assert_eq!(badge(KeyCode::Key2), "2");
        assert_eq!(badge(KeyCode::Kp4), "Num 4");
        assert_eq!(badge(KeyCode::Comma), ",");
        for (k, n) in KEY_NAMES {
            assert_eq!(key_from_name(n), Some(*k));
        }
    }

    #[test]
    fn the_old_p_key_steps_aside() {
        install(&KeyBindings::default());
        assert!(legacy_duties_p());
        let mut b = KeyBindings::default();
        b.set(Action::Book, KeyCode::P);
        install(&b);
        assert!(!legacy_duties_p());
        install(&KeyBindings::default());
    }
}
