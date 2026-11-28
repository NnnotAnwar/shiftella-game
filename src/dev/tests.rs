//! Headless tests of the developer build (`cargo test --features dev`).

use std::rc::Rc;

use super::cheats::{self, Cheat};
use super::jump::{Jump, STORY_SEED, Start};
use super::{catalog, panel};
use crate::app::{Built, SceneBuilder};
use crate::content::Content;
use crate::i18n::{I18n, Lang};
use crate::screens::game::GameScreen;
use crate::settings::{Locale, Settings};
use crate::sim::Phase;

fn content() -> Rc<Content> {
    Rc::new(Content::load("assets/data").expect("content"))
}

fn i18n() -> I18n {
    crate::settings::load_i18n("assets/i18n").expect("i18n")
}

fn settings() -> Settings {
    Settings {
        language: Some(Lang::En),
        ..Default::default()
    }
}

#[test]
fn dev_build_keeps_its_own_save_and_settings_folder() {
    use crate::platform::{DATA_DIR, DATA_DIR_LOWER, RELEASE_DATA_DIR};
    assert_ne!(DATA_DIR, RELEASE_DATA_DIR);
    assert_ne!(DATA_DIR_LOWER, RELEASE_DATA_DIR.to_lowercase());
    let saves = crate::save::platform_dir().expect("a save folder");
    let config = crate::settings::config_dir().expect("a settings folder");
    for p in [&saves, &config] {
        let last = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        assert!(
            last == DATA_DIR || last == DATA_DIR_LOWER,
            "{p:?} is not the dev folder"
        );
        assert_ne!(last, RELEASE_DATA_DIR);
        assert_ne!(last, RELEASE_DATA_DIR.to_lowercase());
    }
}

#[test]
fn jump_lands_on_the_night_deterministically() {
    let c = content();
    let start = Start::Story {
        difficulty: "cozy".into(),
        seed: STORY_SEED,
    };
    let play = || {
        let mut j = Jump::new(&c, &start, 6, true);
        let mut steps = 0;
        while !j.step(&c, std::time::Duration::ZERO) {
            steps += 1;
        }
        // one night per step with no time budget: the progress is visible night by night
        assert_eq!(steps, 4, "nights 1–5 are played one per step");
        let g = j.finish(&c).expect("the day screen");
        assert_eq!(g.world.night(), 6);
        assert_eq!(g.world.phase(), Phase::Day);
        assert!(g.summary.is_none());
        g.world.save_to_string().expect("save")
    };
    let a = play();
    let b = play();
    assert!(a == b, "the same seed lands on the same café");

    // …the very café `fast_forward` (every screenshot scene) reaches
    let loc_i18n = i18n();
    let loc = Locale::new(&loc_i18n, Lang::En);
    let mut ff = GameScreen::new_story(&c, "cozy", true);
    ff.fast_forward(&c, &loc, 6, None, true);
    assert!(
        a == ff.world.save_to_string().unwrap(),
        "same world as fast_forward"
    );

    // another seed, another café
    let mut j = Jump::new(
        &c,
        &Start::Story {
            difficulty: "cozy".into(),
            seed: 7,
        },
        6,
        true,
    );
    j.run(&c);
    let other = j.finish(&c).unwrap().world.save_to_string().unwrap();
    assert!(other != a);
}

#[test]
fn jump_each_difficulty_and_the_sandbox() {
    let c = content();
    for (_, d) in panel::DIFFICULTIES {
        let mut j = Jump::new(
            &c,
            &Start::Story {
                difficulty: d.into(),
                seed: STORY_SEED,
            },
            3,
            true,
        );
        j.run(&c);
        let g = j.finish(&c).unwrap();
        assert_eq!(g.world.night(), 3, "{d}");
        assert_eq!(g.world.difficulty().id(), d);
    }
    for challenge in [false, true] {
        let mut j = Jump::new(
            &c,
            &Start::Sandbox {
                difficulty: "cozy".into(),
                seed: 2039,
                challenge,
            },
            3,
            true,
        );
        j.run(&c);
        let g = j.finish(&c).unwrap();
        assert_eq!(g.world.night(), 3);
        assert_eq!(g.world.mode(), crate::sim::Mode::Sandbox);
    }
    // a running story goes on from where it is
    let mut j = Jump::new(
        &c,
        &Start::Story {
            difficulty: "cozy".into(),
            seed: STORY_SEED,
        },
        2,
        true,
    );
    j.run(&c);
    let g = j.finish(&c).unwrap();
    let mut j = Jump::from_world(g.world.clone(), g.extra.clone(), 4, true);
    assert_eq!(j.from, 2);
    j.run(&c);
    assert_eq!(j.finish(&c).unwrap().world.night(), 4);
}

/// Builds every opener of `group` headless; what each built.
fn open_group(group: &str) {
    let c = content();
    let i18n = i18n();
    let s = settings();
    let mut n = 0;
    for (i, o) in catalog::all().iter().enumerate() {
        if o.group != group {
            continue;
        }
        let t0 = std::time::Instant::now();
        eprintln!("opening {}", o.label);
        let mut b = SceneBuilder {
            content: c.clone(),
            i18n: &i18n,
            settings: &s,
        };
        let built = super::build_opener(&mut b, i);
        let ok = matches!(
            (&built, o.scene),
            (
                Built::Menu { .. },
                "menu" | "language" | "newgame" | "sandbox_setup" | "settings"
            ) | (Built::Credits { .. }, "credits" | "challenge_final")
                | (Built::Game(_) | Built::GameInstantText(..), _)
        );
        assert!(ok, "{}: built the wrong kind of screen", o.label);
        if let Built::Menu { challenge, .. } = built {
            assert_eq!(challenge, o.opts.iter().any(|(_, v)| *v == "challenge"));
        }
        n += 1;
        eprintln!("opened {:<40} {:>6} ms", o.label, t0.elapsed().as_millis());
    }
    assert!(n > 0, "no openers in {group}");
}

#[test]
fn openers_menu_and_start() {
    open_group("Menu & settings");
    open_group("Story start & tutorial");
}

#[test]
fn openers_night() {
    open_group("Night");
}

#[test]
fn openers_panels() {
    open_group("Panels & cards");
}

#[test]
fn openers_app_journal_rituals() {
    open_group("Shiftella app");
    open_group("Journal & photos");
    open_group("Rituals & habits");
}

#[test]
fn openers_minigames() {
    open_group("Mini-games");
}

#[test]
fn openers_morning_and_walk_home() {
    open_group("Morning & walk home");
}

#[test]
fn openers_endings_and_sandbox() {
    open_group("Endings & credits");
    open_group("Sandbox");
}

#[test]
fn every_opener_is_tested_and_labelled_once() {
    let tested = [
        "Menu & settings",
        "Story start & tutorial",
        "Night",
        "Panels & cards",
        "Shiftella app",
        "Journal & photos",
        "Rituals & habits",
        "Mini-games",
        "Morning & walk home",
        "Endings & credits",
        "Sandbox",
    ];
    let mut labels = std::collections::BTreeSet::new();
    for o in catalog::all() {
        assert!(tested.contains(&o.group), "group {} has no test", o.group);
        assert!(labels.insert((o.group, o.label)), "{} twice", o.label);
    }
    assert_eq!(catalog::groups().len(), tested.len());
    assert_eq!(
        catalog::all()
            .iter()
            .filter(|o| o.group == "Mini-games")
            .count(),
        catalog::MINIGAMES.len()
    );
}

#[test]
fn cutscenes_play_fresh_at_their_night() {
    let c = content();
    let i18n = i18n();
    let s = settings();
    // one scene of each kind of trigger (a talk, a night start, a clock time, an arc stage, an
    // ending), played and stopped at a choice
    let mut picked: Vec<(String, bool)> = Vec::new();
    let mut kinds = std::collections::HashSet::new();
    for def in &c.cutscenes.cutscenes {
        let kind = std::mem::discriminant(&def.trigger);
        if kinds.insert(kind) {
            picked.push((def.id.clone(), false));
            if panel::has_choice(def) {
                picked.push((def.id.clone(), true));
            }
        }
    }
    assert!(picked.len() >= 5, "{picked:?}");
    for (id, choice) in picked {
        let (scene, opts) = super::cutscene_scene(&c, &id, choice);
        let mut b = SceneBuilder {
            content: c.clone(),
            i18n: &i18n,
            settings: &s,
        };
        match super::build_with(&mut b, &scene, &opts) {
            Built::Game(g) | Built::GameInstantText(g, _) => {
                assert!(
                    g.cutscene_playing() || g.world.ending().is_some(),
                    "{id} ({scene}, choice {choice}) isn't playing"
                );
            }
            _ => panic!("{id}: no game screen"),
        }
    }
}

#[test]
fn every_cutscene_has_a_place_to_play() {
    let c = content();
    for def in &c.cutscenes.cutscenes {
        let (scene, opts) = super::cutscene_scene(&c, &def.id, false);
        assert!(!scene.is_empty());
        if scene == "cutscene_choice" {
            let night: u32 = opts
                .iter()
                .find(|(k, _)| k == "SHIFTELLA_NIGHT")
                .and_then(|(_, v)| v.parse().ok())
                .expect("a night");
            assert!((1..=30).contains(&night), "{}: night {night}", def.id);
        }
        assert!(!panel::trigger_text(&def.trigger).is_empty());
    }
}

#[test]
fn every_cheat_applies_on_a_night() {
    let c = content();
    let i18n = i18n();
    let loc = Locale::new(&i18n, Lang::En);
    let mut g = GameScreen::new_story(&c, "cozy", true);
    g.fast_forward(&c, &loc, 3, None, true);
    let reg = c.regulars[0].id.clone();
    let ev = c.events[0].id.clone();
    let money = g.world.money();
    let arc = |g: &GameScreen| {
        g.world
            .story()
            .counters
            .get(&format!("arc.{reg}"))
            .copied()
            .unwrap_or(0)
    };
    let arc_before = arc(&g);
    cheats::apply(&mut g, &c, &loc, &Cheat::Money(1000));
    assert_eq!(g.world.money(), money + 1000);
    let list = vec![
        Cheat::Reputation(10.0),
        Cheat::Hire,
        Cheat::Fire,
        Cheat::Time(-30),
        Cheat::Time(0),
        Cheat::Spawn(reg.clone()),
        Cheat::Time(150),
        Cheat::Time(100),
        Cheat::Speed(0),
        Cheat::Speed(3),
        Cheat::Weather(crate::content::Weather::Snow),
        Cheat::Event(ev),
        Cheat::Event("no_such_event".into()),
        Cheat::Arc(reg.clone(), 1),
        Cheat::Trust(reg.clone(), 5.0),
        Cheat::Flag("dev_flag".into(), true),
        Cheat::Var("dev_var".into(), "x".into()),
        Cheat::Counter("dev_counter".into(), 3),
        Cheat::Notes,
        Cheat::Recipes,
        Cheat::Furniture,
        Cheat::Themes,
        Cheat::Dirt(0.5),
        Cheat::Fridge(true),
        Cheat::Fridge(false),
        Cheat::Time(330),
        Cheat::Time(435),
        Cheat::Time(480),
        Cheat::Morning,
    ];
    for ch in &list {
        let msg = cheats::apply(&mut g, &c, &loc, ch);
        assert!(!msg.is_empty(), "{ch:?}");
    }
    let w = &g.world;
    assert!(w.flag("dev_flag"));
    assert_eq!(w.story().vars.get("dev_var").map(String::as_str), Some("x"));
    assert_eq!(w.story().counters.get("dev_counter"), Some(&3));
    assert!(arc(&g) > arc_before, "the arc stage moved on");
    assert_eq!(w.notes_found().len(), c.story.notes.len());
    assert!(
        c.handson
            .recipes
            .recipes
            .iter()
            .all(|r| w.hands.knowledge.recipes.contains_key(&r.id))
    );
    assert!(g.summary.is_some(), "the morning summary shows");
    // the clock really moved: a fresh night jumped to 03:30
    let mut g = GameScreen::new_story(&c, "cozy", true);
    cheats::apply(&mut g, &c, &loc, &Cheat::Time(330));
    assert_eq!(g.world.phase(), Phase::Night);
    assert!(g.world.minutes_since_open() >= 330);
    assert!(cheats::summary(&g).contains("night 1"));
}

#[test]
fn panel_defaults_are_sane() {
    let p = panel::DevPanel::default();
    assert!(!p.open, "closed until ` / F1");
    assert_eq!(p.seed.parse::<u64>().ok(), Some(STORY_SEED));
    assert_eq!(panel::SECTIONS.len(), 6);
    assert!(!cheats::TIMES.is_empty());
}
