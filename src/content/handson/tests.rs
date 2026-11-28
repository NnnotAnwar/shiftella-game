//! Tests over the real hands-on data files.

use std::collections::BTreeSet;

use super::*;
use crate::content::defs::{MenuCategory, Season, Weekday};
use crate::content::story::Date;
use crate::content::{Content, MUSIC_TRACKS};
use crate::i18n::I18n;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");
const I18N: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/i18n");

fn load() -> Content {
    Content::load(DATA).unwrap_or_else(|e| panic!("{e}"))
}

fn must(r: &RecipeDef) -> bool {
    r.tier == Tier::Must
}

#[test]
fn the_promised_amounts() {
    let c = load();
    let h = &c.handson;
    assert_eq!(Taste::ALL.len(), 12);
    assert_eq!(h.recipes.ingredients.len(), 28);
    assert_eq!(
        h.recipes.recipes.iter().filter(|r| must(r)).count(),
        23,
        "17 base (incl. the double espresso) + 6 specials"
    );
    assert_eq!(
        h.recipes
            .recipes
            .iter()
            .filter(|r| r.tier == Tier::Should)
            .count(),
        6
    );
    // + the double espresso (Kseniia's order)
    assert_eq!(h.starter_recipes().count(), 7);
    // handson.md §3.4 lists 27 generic + 17 personal rows (its "43" footnote miscounts);
    // v0.2 package I adds the by-name house-drink wishes (1 generic + 6 personal)
    let by_name = |w: &&super::WishDef| w.favourite.iter().any(|f| f == super::SIGNATURE_RECIPE);
    assert_eq!(h.wishes.wishes.iter().filter(by_name).count(), 7);
    // purchases: + 3 "like home" wishes of the top coziness tier; + Kseniia's double espresso
    assert_eq!(h.wishes.wishes.len(), 44 + 7 + 3 + 1);
    assert_eq!(
        h.wishes
            .wishes
            .iter()
            .filter(|w| w.regular().is_some() && !by_name(w))
            .count(),
        17 + 3 + 1
    );
    let must_moments: Vec<&str> = h
        .moments
        .iter()
        .filter(|m| m.tier == Tier::Must && m.kind != MomentKind::Season)
        .map(|m| m.id.as_str())
        .collect();
    assert_eq!(must_moments.len(), 10, "{must_moments:?}");
    assert!(
        h.moment("m_hang_garlands")
            .is_some_and(|m| m.tier == Tier::Must)
    );
    assert_eq!(h.records.records.len(), 6);
    assert_eq!(
        h.records.records.iter().filter(|r| r.is_starter()).count(),
        3
    );
    assert_eq!(h.decor.themes.len(), 5);
    assert_eq!(h.minigames.latte_patterns.len(), 11);
    let tut_nights: BTreeSet<u32> = h.tutorial.iter().map(|s| s.night).collect();
    assert!((1..=4).all(|n| tut_nights.contains(&n)));
    assert_eq!(h.extras.napkins.len(), 40);
    assert_eq!(c.app.handover_options.len(), 6);
    assert!(
        c.staff
            .iter()
            .all(|s| s.default_duty.is_some() && !s.prefs.is_empty())
    );
}

#[test]
fn every_handson_file_round_trips_through_ron() {
    fn rt<T>(what: &str, value: &T)
    where
        T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
    {
        let text = ron::to_string(value).unwrap_or_else(|e| panic!("{what}: {e}"));
        let back: T = ron::from_str(&text).unwrap_or_else(|e| panic!("{what}: {e}\n{text}"));
        assert!(&back == value, "{what} changed in a round trip");
    }
    let h = load().handson;
    rt("handson.ron", &h.tuning);
    rt("recipes.ron", &h.recipes);
    rt("wishes.ron", &h.wishes);
    rt("moments.ron", &h.moments);
    rt("minigames.ron", &h.minigames);
    rt("records.ron", &h.records);
    rt("seasons.ron", &h.seasons);
    rt("decor.ron", &h.decor);
    rt("journal.ron", &h.journal);
    rt("tutorial.ron", &h.tutorial);
    rt("extras.ron", &h.extras);
    let c = load();
    for s in &c.staff {
        rt("staff prefs", &s.prefs);
    }
    for o in &c.app.handover_options {
        rt("handover option", o);
    }
}

/// Can `recipe` answer `wish` at all (drink vs food)?
fn fits(c: &Content, w: &WishDef, r: &RecipeDef) -> bool {
    let food = c
        .menu_item(&r.base_item)
        .is_some_and(|m| matches!(m.category, MenuCategory::Food | MenuCategory::Dessert));
    match w.kind {
        WishKind::Any => true,
        WishKind::Food => food,
        WishKind::Drink => !food,
    }
}

#[test]
fn every_wish_can_be_answered() {
    let c = load();
    let h = &c.handson;
    let best = |w: &WishDef, only_must: bool| {
        h.recipes
            .recipes
            .iter()
            .filter(|r| (!only_must || must(r)) && fits(&c, w, r))
            .map(|r| match_score(w, r, MatchContext::default()))
            .fold(0.0f32, f32::max)
    };
    let mut problems = Vec::new();
    for w in &h.wishes.wishes {
        // Good (≥ 0.6) with the whole book; never skipped (≥ 0.3) with the MUST recipes alone.
        if best(w, false) < 0.6 || best(w, true) < 0.3 {
            problems.push(format!(
                "{}: best {} / MUST {}",
                w.id,
                best(w, false),
                best(w, true)
            ));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn every_taste_is_on_the_menu() {
    let h = load().handson;
    for t in Taste::ALL {
        let all = h
            .recipes
            .recipes
            .iter()
            .filter(|r| r.tags.contains(&t))
            .count();
        let must_n = h
            .recipes
            .recipes
            .iter()
            .filter(|r| must(r) && r.tags.contains(&t))
            .count();
        assert!(
            must_n >= 1 && all >= 2,
            "{t:?}: {must_n} MUST / {all} recipes"
        );
    }
}

#[test]
fn match_scores_follow_the_design() {
    let h = load().handson;
    let ctx = MatchContext::default();
    let w = h.wish("guest_333.two_sugars").unwrap();
    assert_eq!(match_score(w, h.recipe("r_rn333_tea").unwrap(), ctx), 1.0);
    let lemon = match_score(w, h.recipe("r_lemon_tea").unwrap(), ctx);
    assert!((lemon - 0.8167).abs() < 0.01, "lemon tea = {lemon}");
    let any = h.wish("three_am").unwrap();
    assert_eq!(match_score(any, h.recipe("r_black").unwrap(), ctx), 0.6);
    let long_day = h.wish("long_day").unwrap();
    // Warm but Sweet: 1.0 − 0.5
    assert_eq!(match_score(long_day, h.recipe("r_chai").unwrap(), ctx), 0.5);
    let board = MatchContext {
        on_specials_board: true,
    };
    assert_eq!(
        match_score(long_day, h.recipe("r_chai").unwrap(), board),
        0.6
    );
    let tiers = h.tuning.match_tiers;
    assert_eq!(MatchTier::of(1.0, &tiers), MatchTier::Perfect);
    assert_eq!(MatchTier::of(0.6, &tiers), MatchTier::Good);
    assert_eq!(MatchTier::of(0.29, &tiers), MatchTier::NotQuite);
    let top = h.best_matches(
        h.wish("mirek.like_lida").unwrap(),
        &|r| r.unlock == RecipeUnlock::Starter,
        &|_| ctx,
        4,
    );
    assert_eq!(top[0].0.id, "r_black");
}

#[test]
fn experiments_give_fair_hints() {
    let h = load().handson;
    let starters: BTreeSet<&str> = h.starter_recipes().map(|r| r.id.as_str()).collect();
    let known = |id: &str| starters.contains(id);
    let try_ = |set: &[&str]| experiment(&h.recipes, set, &known);
    assert!(
        matches!(try_(&["honey", "linden"]), ExperimentOutcome::Discovered(r) if r.id == "r_linden_tea")
    );
    assert!(
        matches!(try_(&["espresso", "milk"]), ExperimentOutcome::Known(r) if r.id == "r_cappuccino")
    );
    assert!(matches!(
        try_(&["potato", "mushroom", "cream"]),
        ExperimentOutcome::Gated(r) if r.gated_hint.is_some()
    ));
    // one right, one wrong, same size → "swap the … one" (ties: fewest ingredients, data order)
    assert!(matches!(
        try_(&["linden", "sugar"]),
        ExperimentOutcome::Close {
            kind: CloseKind::Swap,
            family: Some(_),
            ..
        }
    ));
    assert!(matches!(
        try_(&["oat_milk", "honey"]),
        ExperimentOutcome::Close { recipe, kind: CloseKind::Missing, family: Some(Family::Fruit) }
            if recipe.id == "r_moon_latte"
    ));
    assert!(matches!(
        try_(&["linden", "honey", "ice"]),
        ExperimentOutcome::Close {
            kind: CloseKind::Extra,
            ..
        }
    ));
    assert_eq!(try_(&["bread", "ice"]), ExperimentOutcome::Nothing);
    // every experimentable recipe can be found by its exact set, within the pantry slot limits
    let e = &h.recipes.experiment;
    for r in h
        .recipes
        .recipes
        .iter()
        .filter(|r| r.unlock == RecipeUnlock::Experiment)
    {
        assert!((e.min..=e.max).contains(&r.ingredients.len()), "{}", r.id);
    }
}

#[test]
fn story_calendar_matches_the_design() {
    let c = load();
    let cal = c.story.calendar.as_ref().expect("calendar");
    let d = |n| cal.date_of_night(n);
    assert_eq!(
        d(1),
        Date {
            year: 2039,
            month: 11,
            day: 7
        }
    );
    assert_eq!(d(1).weekday(), Weekday::Mon);
    assert_eq!(
        d(21),
        Date {
            year: 2039,
            month: 11,
            day: 27
        }
    );
    assert_eq!(d(21).weekday(), Weekday::Sun, "1st Advent Sunday");
    assert_eq!(
        d(29),
        Date {
            year: 2039,
            month: 12,
            day: 5
        },
        "Mikuláš"
    );
    assert_eq!(
        d(30),
        Date {
            year: 2039,
            month: 12,
            day: 6
        }
    );
    assert_eq!(
        Date {
            year: 2026,
            month: 11,
            day: 26
        }
        .weekday(),
        Weekday::Thu
    );
    assert_eq!(
        Date {
            year: 2024,
            month: 2,
            day: 28
        }
        .plus_days(1)
        .day,
        29
    );
    assert!(cal.days_on(27).any(|x| x.id == "first_snow"));
    let s = &c.handson.seasons;
    assert_eq!(s.story_phase(27).map(|p| p.id.as_str()), Some("first_snow"));
    assert_eq!(s.story_phase(21).map(|p| p.id.as_str()), Some("advent"));
    assert_eq!(
        s.sandbox_phase(Season::Winter, 25).map(|p| p.id.as_str()),
        Some("first_snow")
    );
}

#[test]
fn records_point_at_real_tracks() {
    let h = load().handson;
    for r in &h.records.records {
        assert!(MUSIC_TRACKS.contains(&r.track.as_str()));
        assert!((94.0..97.0).contains(&r.seconds), "{}", r.id);
    }
}

#[test]
fn small_formulas() {
    let h = load().handson;
    assert_eq!(h.tuning.helper_quality.at(10.0), 0.75);
    assert!((h.tuning.helper_quality.at(5.0) - 0.55).abs() < 1e-6);
    assert_eq!(h.tuning.wish_chance_on(1), 0.4);
    assert_eq!(h.tuning.wish_chance_on(12), 0.5);
    assert_eq!(h.tuning.moment_budget(1), (1, 3));
    let cz = &h.decor.coziness;
    assert_eq!(cz.level(10.0), 1);
    // purchases: the tiers sit at 30 / 50 / 70 / 85 (grandma's hall ≈ 64, Pleasant)
    assert_eq!(cz.level(80.0), 4);
    assert_eq!(cz.level(85.0), 5);
    assert!((cz.effects.patience.at(100.0) - 1.3).abs() < 1e-6);
    let cat = &h.extras.cat;
    assert_eq!(cat.level_at(5), 4);
    let soup = h.recipe("r_garlic_soup").unwrap();
    assert_eq!(soup.playable_minigame(false), (MiniGameKind::Plate, "soup"));
    assert_eq!(
        soup.playable_minigame(true),
        (MiniGameKind::SoupStir, "garlic")
    );
    let winter = h.theme("winter_lights").unwrap();
    assert_eq!(winter.coziness_at("advent"), 8.0);
    assert_eq!(winter.coziness_at("late_autumn"), 2.0);
    assert_eq!(
        h.tuning.rozpis.ring(90.0).map(|r| r.id.as_str()),
        Some("happy")
    );
    assert_eq!(
        h.tuning.rozpis.ring(10.0).map(|r| r.id.as_str()),
        Some("stretched")
    );
}

#[test]
fn validation_catches_broken_handson_references() {
    let i18n = I18n::load(I18N).expect("i18n loads");
    let mut c = load();
    c.handson.wishes.wishes[0].favourite.push("r_nope".into());
    c.handson.moments[0].options[0]
        .effects
        .push(Fx::X(XEffect::SpawnMoment("m_nope".into())));
    c.handson.recipes.recipes[0]
        .ingredients
        .push("unobtainium".into());
    c.handson.records.records[0].liked_by.push("nobody".into());
    let problems = c.validate(&i18n).unwrap_err();
    for needle in ["r_nope", "m_nope", "unobtainium", "nobody"] {
        assert!(
            problems.iter().any(|p| p.contains(needle)),
            "{needle}: {problems:?}"
        );
    }
}

#[test]
fn checklist_owner_falls_back_to_the_job() {
    let c = load();
    let item = |id: &str| c.app.checklists.iter().find(|i| i.id == id).unwrap();
    assert_eq!(item("light_window").owner(), ChecklistOwner::Karolina);
    assert_eq!(
        item("wash_dishes").owner(),
        ChecklistOwner::Either(Duty::Dishes)
    );
    assert_eq!(item("menu_board").owner(), ChecklistOwner::Karolina);
}
