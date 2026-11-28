//! Game content loaded from `assets/data/*.ron`: traits, staff, menu, guests, regulars, furniture,
//! events, story, cutscenes, the Shiftella app, trains, difficulty, sandbox and narrator pacing.
//!
//! ```no_run
//! use shiftella_game::{content::Content, i18n::I18n};
//! let i18n = I18n::load("assets/i18n").unwrap();
//! let content = Content::load("assets/data").unwrap();
//! content.validate(&i18n).unwrap();
//! ```
//!
//! Schemas and semantics: `docs/CONTENT.md`.

pub mod build;
pub mod defs;
pub mod handson;
pub mod logic;
pub mod story;
mod validate;
pub mod world;

pub use defs::*;
pub use logic::{Condition, Effect, Ref};
pub use story::*;
pub use world::*;

use serde::de::DeserializeOwned;

/// Music track ids (files `assets/audio/music/<id>.ogg`).
pub const MUSIC_TRACKS: &[&str] = &[
    "mus_menu_theme",
    "mus_night_calm_loop",
    "mus_rush_loop",
    "jukebox_prague_swing",
    "jukebox_piano_trio",
    "jukebox_bossa_rain",
    "jukebox_folk_lullaby",
    "jukebox_synth_2039",
    "jukebox_cafe_waltz",
];

/// Sound ids usable in cutscenes (files in `assets/audio/sfx/` — the generated pack, not in
/// the repository — or `assets/audio/cc0/`, committed). `amb_` ids loop: a cutscene step's
/// volume holds the loop at that level, `0` lets it fade out.
pub const SFX_IDS: &[&str] = &[
    "amb_rain_window_loop",
    "amb_train_pass_far",
    "sfx_cash_register",
    "sfx_coffee_machine",
    "sfx_cup_saucer",
    "sfx_door_bell",
    "sfx_footsteps_tile",
    "sfx_power_outage",
    "ui_alert_event",
    "ui_click",
    // CC0 (assets/audio/cc0/, assets/audio/CREDITS.md)
    "sfx_machine_hiss",
    "sfx_machine_purge",
    "sfx_machine_sputter",
    "sfx_shot_pull",
    "sfx_group_clean",
    "sfx_knock_box",
    "sfx_pot_lid",
    "sfx_pot_stir",
    "sfx_oven_ding",
    "sfx_oven_door",
    "sfx_fridge_door",
    "sfx_bottle_clink",
    "sfx_till_coins",
    "sfx_trash_bag",
    "sfx_door_outside",
    "sfx_lamp_switch",
    "sfx_keys_lock",
    "sfx_pigeons",
    "sfx_tram_bell",
    "sfx_tram_pass",
    "sfx_door_creak",
    "sfx_phone_buzz",
    "sfx_cat_meow",
    "sfx_book_open",
    "sfx_page_flip",
    "sfx_blanket",
    "sfx_chalk",
    "photo_shutter",
    "photo_eject",
    "amb_soup_simmer",
    "amb_rain_awning",
    "amb_storm",
    "amb_cat_purr",
    "amb_wet_steps",
];

/// Everything in `assets/data`.
#[derive(Debug, Clone)]
pub struct Content {
    pub traits: Vec<TraitDef>,
    pub staff: Vec<StaffArchetype>,
    pub name_pools: Vec<NamePool>,
    pub menu: Vec<MenuItem>,
    pub guests: Vec<GuestArchetype>,
    pub regulars: Vec<RegularDef>,
    pub furniture: Vec<FurnitureDef>,
    pub events: Vec<EventDef>,
    pub narrator: NarratorDef,
    pub story: StoryDef,
    pub cutscenes: CutsceneLib,
    pub app: ShiftellaApp,
    pub trains: TrainLine,
    pub difficulties: Vec<DifficultyDef>,
    pub sandbox: SandboxDef,
    /// Hands-on content (`handson::HANDSON_FILES`).
    pub handson: handson::HandsOn,
    /// Reasons to build: requests, corners, the attic, the back room (`build::BUILD_FILES`).
    pub build: build::BuildContent,
}

/// The data files, in load order (plus [`handson::HANDSON_FILES`]).
pub const DATA_FILES: &[&str] = &[
    "traits.ron",
    "staff.ron",
    "menu.ron",
    "guests.ron",
    "regulars.ron",
    "furniture.ron",
    "events.ron",
    "narrator.ron",
    "story.ron",
    "cutscenes.ron",
    "talks.ron",
    "shiftella.ron",
    "trains.ron",
    "difficulty.ron",
    "sandbox.ron",
];

pub(crate) fn read<T: DeserializeOwned>(dir: &str, file: &str) -> Result<T, String> {
    let path = format!("{dir}/{file}");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
    ron::from_str(&text).map_err(|e| format!("{path}: {e}"))
}

impl Content {
    /// Reads every file of [`DATA_FILES`] from `dir` (e.g. `"assets/data"`).
    pub fn load(dir: &str) -> Result<Content, String> {
        let staff: StaffFile = read(dir, "staff.ron")?;
        Ok(Content {
            traits: read(dir, "traits.ron")?,
            staff: staff.archetypes,
            name_pools: staff.name_pools,
            menu: read(dir, "menu.ron")?,
            guests: read(dir, "guests.ron")?,
            regulars: read(dir, "regulars.ron")?,
            furniture: read(dir, "furniture.ron")?,
            events: read(dir, "events.ron")?,
            narrator: read(dir, "narrator.ron")?,
            story: read(dir, "story.ron")?,
            cutscenes: story::with_talks(read(dir, "cutscenes.ron")?, read(dir, "talks.ron")?),
            app: read(dir, "shiftella.ron")?,
            trains: read(dir, "trains.ron")?,
            difficulties: read(dir, "difficulty.ron")?,
            sandbox: read(dir, "sandbox.ron")?,
            handson: handson::HandsOn::load(dir)?,
            build: build::BuildContent::load(dir)?,
        })
    }

    /// Checks that every referenced id exists and every i18n key exists in all four languages.
    /// Returns all problems at once.
    pub fn validate(&self, i18n: &crate::i18n::I18n) -> Result<(), Vec<String>> {
        let problems = validate::validate(self, i18n);
        if problems.is_empty() {
            Ok(())
        } else {
            Err(problems)
        }
    }

    /// Every i18n key the content needs (derived keys + keys written in data).
    pub fn required_keys(&self) -> Vec<String> {
        validate::required_keys(self)
    }

    pub fn trait_def(&self, id: &str) -> Option<&TraitDef> {
        self.traits.iter().find(|x| x.id == id)
    }
    pub fn staff_archetype(&self, id: &str) -> Option<&StaffArchetype> {
        self.staff.iter().find(|x| x.id == id)
    }
    pub fn menu_item(&self, id: &str) -> Option<&MenuItem> {
        self.menu.iter().find(|x| x.id == id)
    }
    pub fn guest(&self, id: &str) -> Option<&GuestArchetype> {
        self.guests.iter().find(|x| x.id == id)
    }
    pub fn regular(&self, id: &str) -> Option<&RegularDef> {
        self.regulars.iter().find(|x| x.id == id)
    }
    pub fn furniture_def(&self, id: &str) -> Option<&FurnitureDef> {
        self.furniture.iter().find(|x| x.id == id)
    }
    /// Furniture providing a tag (e.g. every `station:coffee`).
    pub fn furniture_with_tag<'a>(
        &'a self,
        tag: &'a str,
    ) -> impl Iterator<Item = &'a FurnitureDef> + 'a {
        self.furniture.iter().filter(move |f| f.has_tag(tag))
    }
    pub fn event(&self, id: &str) -> Option<&EventDef> {
        self.events.iter().find(|x| x.id == id)
    }
    pub fn cutscene(&self, id: &str) -> Option<&CutsceneDef> {
        self.cutscenes.get(id)
    }
    pub fn difficulty(&self, id: &str) -> Option<&DifficultyDef> {
        self.difficulties.iter().find(|x| x.id == id)
    }
    /// The preset marked `default: true` (Cozy).
    pub fn default_difficulty(&self) -> Option<&DifficultyDef> {
        self.difficulties.iter().find(|d| d.default)
    }
    pub fn train(&self, id: &str) -> Option<&TrainDef> {
        self.trains.trains.iter().find(|x| x.id == id)
    }
    /// Menu items unlocked by a story night (ignores `unlock_flag`).
    pub fn menu_for_night(&self, night: u32) -> impl Iterator<Item = &MenuItem> {
        self.menu.iter().filter(move |m| m.unlock_night <= night)
    }

    /// One-line counts for logs / the `content_check` example.
    pub fn summary(&self) -> String {
        format!(
            "traits {} · staff archetypes {} · name pools {} · menu {} · guest archetypes {} · regulars {} · \
             furniture {} · events {} · story nights {} · notes {} · endings {} · cutscenes {} · cast {} · \
             bot templates {} · checklist items {} · announcements {} · stations {} · trains {} · difficulties {} · night characters {}",
            self.traits.len(),
            self.staff.len(),
            self.name_pools.len(),
            self.menu.len(),
            self.guests.len(),
            self.regulars.len(),
            self.furniture.len(),
            self.events.len(),
            self.story.nights.len(),
            self.story.notes.len(),
            self.story.endings.len(),
            self.cutscenes.cutscenes.len(),
            self.cutscenes.cast.len(),
            self.app.bot.len(),
            self.app.checklists.len(),
            self.app.announcements.len(),
            self.trains.stations.len(),
            self.trains.trains.len(),
            self.difficulties.len(),
            self.sandbox.characters.len(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::I18n;

    const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");
    const I18N: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/i18n");

    fn load() -> Content {
        Content::load(DATA).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn real_content_loads_and_validates() {
        let content = load();
        let i18n = I18n::load(I18N).expect("i18n loads");
        if let Err(problems) = content.validate(&i18n) {
            panic!("{} problems:\n{}", problems.len(), problems.join("\n"));
        }
    }

    #[test]
    fn content_has_the_promised_amounts() {
        let c = load();
        assert_eq!(c.traits.len(), 16);
        assert!(c.menu.len() >= 12 && c.menu.len() <= 16, "{}", c.menu.len());
        assert!(c.guests.len() >= 10);
        assert_eq!(c.regulars.len(), 8, "7 story regulars + the Anwar cameo");
        assert!(c.events.len() >= 35);
        assert_eq!(c.story.nights.len(), 30);
        assert_eq!(c.story.notes.len(), 10);
        assert_eq!(c.story.endings.len(), 2, "exactly two endings");
        assert_eq!(c.difficulties.len(), 3, "Cozy, Carefree (relaxed), Classic");
        assert_eq!(c.default_difficulty().map(|d| d.id.as_str()), Some("cozy"));
        assert!(c.cutscenes.cutscenes.len() >= 20);
    }

    #[test]
    fn every_sprite_of_the_render_contract_is_used() {
        let c = load();
        for sprite in SPRITES {
            assert!(
                c.furniture.iter().any(|f| f.sprite == *sprite),
                "no furniture uses sprite `{sprite}`"
            );
        }
    }

    #[test]
    fn audio_ids_exist_on_disk() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/audio");
        // The generated audio is not in the source repository (licence): it ships only inside
        // release builds. A checkout without it (`assets/audio/music`) has nothing to check; one
        // with it (a release tree) must have every file. The CC0 sounds (`assets/audio/cc0/`) are
        // always there — `audio::tests::cc0_sounds_are_on_disk_and_credited` checks them.
        if !std::path::Path::new(&format!("{root}/music")).is_dir() {
            eprintln!("audio_ids_exist_on_disk: no generated pack (assets/audio/music) — skipped");
            return;
        }
        for m in MUSIC_TRACKS {
            assert!(
                std::path::Path::new(&format!("{root}/music/{m}.ogg")).exists(),
                "{m}"
            );
        }
        for s in SFX_IDS {
            let found = ["sfx", "cc0"].iter().any(|d| {
                ["ogg", "wav"]
                    .iter()
                    .any(|e| std::path::Path::new(&format!("{root}/{d}/{s}.{e}")).exists())
            });
            assert!(found, "{s}");
        }
    }

    #[test]
    fn trains_match_the_skeleton_timetable() {
        let c = load();
        let times: Vec<String> = c
            .trains
            .trains
            .iter()
            .map(|t| t.arrives.to_string())
            .collect();
        for t in ["22:47", "00:15", "01:52", "03:33", "05:20"] {
            assert!(times.iter().any(|x| x == t), "no train at {t}");
        }
        assert!(c.train("Rn 333").is_some());
    }

    #[test]
    fn validation_catches_a_broken_reference() {
        let mut c = load();
        let i18n = I18n::load(I18N).expect("i18n loads");
        c.guests[0].favourites.push("no_such_item".into());
        let problems = c.validate(&i18n).unwrap_err();
        assert!(problems.iter().any(|p| p.contains("no_such_item")));
    }
}
