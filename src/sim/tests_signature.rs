//! The café's own drink and photo mode (design/v0.2.md I, `sim::signature`): the data is
//! sound, composing follows the rules (pantry, styles, names in any script, once a week), the
//! drink answers wishes and is asked for by name, it brews and pays, photos have slots per
//! night and never touch the kept polaroid, saves keep it all and old saves load.

use super::signature::{self, SIGNATURE_RECIPE};
use super::*;
use crate::content::Content;
use crate::content::handson::{MatchContext, MiniGameKind, Taste, match_score};

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

fn content() -> Content {
    Content::load(DATA).unwrap_or_else(|e| panic!("{e}"))
}

/// The day before story night `n` (cutscenes done, the tutorial off).
fn story_day(c: &Content, n: u32) -> World {
    let mut w = World::new_story(c, Difficulty::Cozy);
    w.sys.tutorial_off = true;
    if n > 1 {
        w.night = n;
        w.begin_day(c);
    }
    bot::finish_cutscenes(&mut w, c);
    w.drain_events();
    w
}

/// From the day into the night: doors open, no random walk-ins.
fn into_night(w: &mut World, c: &Content) {
    w.start_night(c);
    bot::finish_cutscenes(w, c);
    w.skip_prep(c);
    w.mods.push(Modifier {
        kind: ModKind::GuestRate,
        mul: 0.0,
        until: None,
        event: None,
    });
    w.drain_events();
}

fn story_night(c: &Content, n: u32) -> World {
    let mut w = story_day(c, n);
    into_night(&mut w, c);
    w
}

fn jars(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

fn compose(
    w: &mut World,
    c: &Content,
    base: &str,
    j: &[&str],
    style: MiniGameKind,
    name: &str,
) -> Result<(), CommandError> {
    w.apply(
        c,
        Command::ComposeSignature {
            base: base.into(),
            jars: jars(j),
            style,
            name: name.into(),
        },
    )
}

fn asks_by_name(c: &Content, wish: &str) -> bool {
    c.handson
        .wish(wish)
        .is_some_and(|d| d.favourite.iter().any(|f| f == SIGNATURE_RECIPE))
}

#[test]
fn the_data_is_sound_against_the_content() {
    let c = content();
    let d = signature::parse(include_str!("../../assets/data/signature.ron")).unwrap();
    assert!(d.jars.0 >= 2 && d.jars.1 <= 3 && d.jars.0 <= d.jars.1);
    assert_eq!(d.name_max, 24);
    for b in &d.bases {
        assert!(c.handson.ingredient(&b.ingredient).is_some(), "{}", b.id);
        let m = c
            .menu_item(&b.base_item)
            .unwrap_or_else(|| panic!("{}", b.base_item));
        assert_eq!(m.job, crate::content::Job::Bar, "{}: a drink", b.id);
        assert!(!b.styles.is_empty());
        for s in &b.styles {
            assert!(d.style(*s).is_some(), "{}: style {s:?}", b.id);
        }
    }
    let mg = &c.handson.minigames;
    for s in &d.styles {
        let ok = match s.kind {
            MiniGameKind::MilkFroth => mg.milk_froth.iter().any(|v| v.variant == s.variant),
            MiniGameKind::PourOver => mg.pour_over.iter().any(|v| v.variant == s.variant),
            MiniGameKind::TeaSteep => mg.tea_steep.iter().any(|v| v.variant == s.variant),
            MiniGameKind::LatteArt => mg.latte_art.variants.iter().any(|v| v.variant == s.variant),
            _ => false,
        };
        assert!(ok, "{:?}: variant {} exists", s.kind, s.variant);
    }
    for (j, tags) in &d.ingredient_tags {
        assert!(c.handson.ingredient(j).is_some(), "{j}");
        assert!(!tags.is_empty());
    }
    // every key in all four languages
    let i18n = crate::i18n::I18n::load(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/i18n"))
        .unwrap_or_else(|e| panic!("{e}"));
    let mut keys: Vec<String> = d.bases.iter().map(|b| b.name_key()).collect();
    keys.extend(
        d.styles
            .iter()
            .map(|s| format!("signature.style.{}", s.kind.id())),
    );
    keys.extend((1..=d.suggestions).map(|n| format!("signature.suggest.{n}")));
    keys.extend((1..=signature::PHOTO_CAPTIONS).map(|n| format!("photo.caption.{n}")));
    let by_name: Vec<_> = c
        .handson
        .wishes
        .wishes
        .iter()
        .filter(|w| asks_by_name(&c, &w.id))
        .collect();
    assert!(by_name.len() >= 2, "strangers and regulars ask by name");
    for w in by_name {
        keys.push(signature::wish_line_key(&w.id));
    }
    for k in &keys {
        assert!(i18n.has_all(k), "{k} in all four languages");
    }
    // the name ideas fit the name field
    for lang in crate::i18n::Lang::ALL {
        for n in 1..=d.suggestions {
            let k = format!("signature.suggest.{n}");
            let s = i18n.tr(lang, &k);
            assert!(signature::clean_name(s).is_some(), "{lang:?} {s}");
        }
    }
}

#[test]
fn tastes_come_from_the_base_the_jars_and_the_style() {
    let d = signature::data();
    let coffee = d.base("coffee").unwrap();
    let t = signature::tags_for(
        d,
        coffee,
        &jars(&["milk", "honey"]),
        MiniGameKind::MilkFroth,
    );
    assert!(
        t.contains(&Taste::Warm) && t.contains(&Taste::Comforting),
        "{t:?}"
    );
    assert!(t.len() <= d.max_tags);
    // ice makes it a cold drink
    let t = signature::tags_for(d, coffee, &jars(&["ice", "tonic"]), MiniGameKind::PourOver);
    assert!(
        t.contains(&Taste::Cold) && !t.contains(&Taste::Warm),
        "{t:?}"
    );
    // deterministic, order-free
    let a = signature::tags_for(
        d,
        coffee,
        &jars(&["honey", "spice"]),
        MiniGameKind::LatteArt,
    );
    let b = signature::tags_for(
        d,
        coffee,
        &jars(&["spice", "honey"]),
        MiniGameKind::LatteArt,
    );
    assert_eq!(a, b);
}

#[test]
fn names_are_trimmed_and_count_letters_not_bytes() {
    assert_eq!(
        signature::clean_name("  Holešovická   noc "),
        Some("Holešovická noc".into())
    );
    assert_eq!(
        signature::clean_name("Голешовицкая ночь"),
        Some("Голешовицкая ночь".into())
    );
    // 24 Cyrillic letters are fine (48 bytes), 25 are not
    let ok: String = "ж".repeat(24);
    assert_eq!(signature::clean_name(&ok), Some(ok.clone()));
    assert_eq!(signature::clean_name(&"ж".repeat(25)), None);
    assert_eq!(signature::clean_name("   "), None);
    assert_eq!(signature::clean_name("«…»"), None);
    assert_eq!(signature::clean_name("Rn\t333"), Some("Rn 333".into()));
}

#[test]
fn composing_follows_the_pantry_the_rules_and_the_calendar() {
    let c = content();
    let froth = MiniGameKind::MilkFroth;
    // too early
    let mut w = story_day(&c, 4);
    assert!(!w.signature_unlocked());
    assert_eq!(
        compose(&mut w, &c, "coffee", &["milk", "honey"], froth, "Test"),
        Err(CommandError::NotUnlocked)
    );
    let mut w = story_day(&c, 6);
    assert!(w.signature_unlocked());
    let bad = |r: Result<(), CommandError>| matches!(r, Err(CommandError::Invalid(_)));
    // one jar is not a drink; the base's own jar doesn't count twice
    assert!(bad(compose(&mut w, &c, "coffee", &["milk"], froth, "Test")));
    assert!(bad(compose(
        &mut w,
        &c,
        "coffee",
        &["espresso", "milk"],
        froth,
        "Test"
    )));
    // garlic never goes in a cup; two milks never
    assert!(bad(compose(
        &mut w,
        &c,
        "coffee",
        &["garlic", "milk"],
        froth,
        "Test"
    )));
    assert!(bad(compose(
        &mut w,
        &c,
        "coffee",
        &["milk", "cream"],
        froth,
        "Test"
    )));
    // a style that doesn't suit the base
    assert!(bad(compose(
        &mut w,
        &c,
        "coffee",
        &["milk", "honey"],
        MiniGameKind::TeaSteep,
        "Test"
    )));
    // a jar not on the shelf yet (vanilla from night 8)
    assert_eq!(
        compose(&mut w, &c, "coffee", &["milk", "vanilla"], froth, "Test"),
        Err(CommandError::Locked)
    );
    // a bad name
    assert!(bad(compose(
        &mut w,
        &c,
        "coffee",
        &["milk", "honey"],
        froth,
        "  "
    )));
    assert!(w.signature().is_none());
    // the real thing
    compose(
        &mut w,
        &c,
        "coffee",
        &["milk", "honey"],
        froth,
        "Holešovická noc",
    )
    .unwrap();
    let ev = w.drain_events();
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::SignatureComposed { first: true, .. }))
    );
    assert!(ev.iter().any(
        |e| matches!(e, SimEvent::RecipeLearned { recipe, .. } if recipe == SIGNATURE_RECIPE)
    ));
    let s = w.signature().unwrap();
    assert_eq!(s.name, "Holešovická noc");
    assert_eq!(s.def.id, SIGNATURE_RECIPE);
    assert_eq!(s.def.base_item, "cappuccino");
    assert_eq!(s.def.ingredients, jars(&["espresso", "milk", "honey"]));
    assert_eq!(s.def.price_add, 20, "10 + 5 a jar");
    assert!(w.known_recipes().any(|r| r == SIGNATURE_RECIPE));
    assert!(w.recipe_def(&c, SIGNATURE_RECIPE).is_some());
    assert_eq!(
        w.all_recipes(&c).count(),
        c.handson.recipes.recipes.len() + 1
    );
    // the same night it can still be tweaked (no change counted)
    compose(
        &mut w,
        &c,
        "coffee",
        &["milk", "honey", "spice"],
        MiniGameKind::LatteArt,
        "Holešovická noc",
    )
    .unwrap();
    assert_eq!(w.signature().unwrap().changes, 0);
    assert_eq!(w.signature().unwrap().def.price_add, 25);
    // next night: locked for the week
    w.night = 7;
    assert_eq!(w.signature_next_change(), Some(13));
    assert!(bad(compose(
        &mut w,
        &c,
        "tea",
        &["lemon", "honey"],
        MiniGameKind::TeaSteep,
        "Lídin čaj"
    )));
    w.night = 13;
    assert_eq!(w.signature_next_change(), None);
    w.drain_events();
    compose(
        &mut w,
        &c,
        "tea",
        &["lemon", "honey"],
        MiniGameKind::TeaSteep,
        "Лидин чай",
    )
    .unwrap();
    let s = w.signature().unwrap();
    assert_eq!((s.changes, s.made_night, s.changed_night), (1, 6, 13));
    assert_eq!(s.def.base_item, "lemon_tea");
    let ev = w.drain_events();
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::SignatureComposed { first: false, .. }))
    );
    assert!(
        !ev.iter()
            .any(|e| matches!(e, SimEvent::RecipeLearned { .. })),
        "learned once"
    );
}

#[test]
fn grandma_hints_at_it_once_when_it_opens() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    w.sys.tutorial_off = true;
    let mut invites = 0;
    for n in 2..=9 {
        w.night = n;
        w.begin_day(&c);
        invites += w
            .drain_events()
            .iter()
            .filter(|e| matches!(e, SimEvent::SignatureInvite))
            .count();
        if n < signature::data().from_night {
            assert_eq!(invites, 0);
        }
    }
    assert_eq!(invites, 1);
}

#[test]
fn the_drink_answers_wishes_and_is_asked_for_by_name_now_and_then() {
    let c = content();
    let mut w = story_day(&c, 6);
    into_night(&mut w, &c);
    // before it exists nobody asks for it by name
    let mut before = 0;
    for _ in 0..300 {
        w.hands.wish_uses.clear();
        if let OrderKind::Wish { wish } =
            orders::pick_order_kind(&mut w, &c, "travelers", None, 1, true)
            && asks_by_name(&c, &wish)
        {
            before += 1;
        }
    }
    assert_eq!(before, 0);
    compose(
        &mut w,
        &c,
        "coffee",
        &["milk", "honey"],
        MiniGameKind::MilkFroth,
        "Noc",
    )
    .unwrap();
    let hs = c.handson.wish("house_special").unwrap().clone();
    let m = orders::matches_for(&w, &c, &hs);
    assert_eq!(
        m.first().map(|x| (x.0.as_str(), x.1)),
        Some((SIGNATURE_RECIPE, 1.0))
    );
    // a wish by taste scores it by its tastes like any recipe
    let wd = c.handson.wish("kind_coffee").unwrap();
    let def = w.signature().unwrap().def.clone();
    let mine = orders::matches_for(&w, &c, wd)
        .into_iter()
        .find(|x| x.0 == SIGNATURE_RECIPE)
        .unwrap()
        .1;
    assert_eq!(mine, match_score(wd, &def, MatchContext::default()));
    assert!(mine >= 0.5);
    // strangers: some, not most
    let mut strangers = 0;
    for _ in 0..600 {
        w.hands.wish_uses.clear();
        if let OrderKind::Wish { wish } =
            orders::pick_order_kind(&mut w, &c, "travelers", None, 1, true)
            && asks_by_name(&c, &wish)
        {
            strangers += 1;
        }
    }
    assert!(
        (15..=150).contains(&strangers),
        "{strangers} of 600 stranger wishes by name"
    );
    // a regular: now and then
    let mirek = c.regular("mirek").unwrap().clone();
    let mut by_mirek = 0;
    for _ in 0..600 {
        w.hands.wish_uses.clear();
        let k = orders::pick_order_kind(&mut w, &c, &mirek.base, Some(&mirek), 1, false);
        if k.wish().is_some_and(|x| asks_by_name(&c, x)) {
            by_mirek += 1;
        }
    }
    assert!(
        (20..=300).contains(&by_mirek),
        "{by_mirek} of 600 of Mirek's wishes by name"
    );
}

#[test]
fn karolina_brews_the_house_drink_and_it_pays_its_price() {
    let c = content();
    let mut w = story_day(&c, 6);
    compose(
        &mut w,
        &c,
        "coffee",
        &["milk", "honey"],
        MiniGameKind::MilkFroth,
        "Noc",
    )
    .unwrap();
    into_night(&mut w, &c);
    let pid = guests::spawn_party(&mut w, &c, "travelers", 1, None, 1.0, 1.0, &[], None, true)
        .expect("spawned");
    let p = w.parties.iter_mut().find(|p| p.id == pid).unwrap();
    p.order_kind = OrderKind::Wish {
        wish: "house_special".into(),
    };
    p.wish_open = true;
    let leader = p.leader().unwrap();
    w.apply(&c, Command::PlayerTakeOrder { guest: leader })
        .unwrap();
    let mut taken = false;
    for _ in 0..900 {
        w.tick(&c);
        if w.drain_events()
            .iter()
            .any(|e| matches!(e, SimEvent::PlayerOrderTaken { .. }))
        {
            taken = true;
            break;
        }
    }
    assert!(taken);
    let chips = w.best_matches(&c, leader);
    assert_eq!(chips[0], (SIGNATURE_RECIPE.to_string(), 1.0));
    w.apply(
        &c,
        Command::PlayerBrew {
            guest: leader,
            recipe: SIGNATURE_RECIPE.into(),
            quality: 0.9,
            mode: BrewMode::ByHand,
        },
    )
    .unwrap();
    let item = w.items.iter().find(|i| i.party == pid && i.wish).unwrap();
    assert_eq!(item.price_add, 20);
    assert_eq!(item.menu, "cappuccino");
    let mut served = false;
    for _ in 0..1500 {
        w.tick(&c);
        if w.drain_events().iter().any(|e| {
            matches!(e, SimEvent::PlayerServed { recipe, match_score, .. }
                if recipe == SIGNATURE_RECIPE && *match_score == 1.0)
        }) {
            served = true;
            break;
        }
    }
    assert!(served);
    assert_eq!(w.signature_asked_served(), 1);
    assert_eq!(w.knowledge().recipes[SIGNATURE_RECIPE].served, 1);
}

#[test]
fn photos_have_a_few_slots_a_night_and_leave_the_polaroid_alone() {
    let c = content();
    let mut w = story_day(&c, 3);
    let shot = |x: f32| Command::TakePhoto {
        focus: (x, 400.0),
        width: 640.0,
    };
    assert_eq!(w.apply(&c, shot(640.0)), Err(CommandError::WrongPhase));
    into_night(&mut w, &c);
    let per = signature::data().photo.per_night;
    assert!(per >= 2);
    assert_eq!(w.photos_left(), per);
    for i in 0..per {
        w.apply(&c, shot(640.0 + i as f32 * 64.0)).unwrap();
        let ev = w.drain_events();
        assert!(
            ev.iter().any(
                |e| matches!(e, SimEvent::PhotoTaken { night: 3, left } if *left == per - 1 - i)
            )
        );
    }
    assert_eq!(w.photos_left(), 0);
    assert!(matches!(
        w.apply(&c, shot(640.0)),
        Err(CommandError::Invalid(_))
    ));
    let p = &w.photos()[0];
    assert_eq!(p.night, 3);
    assert!(p.date.is_some());
    assert!((p.snap.focus.0 - 640.0).abs() < 0.01);
    assert!(p.snap.zoom > 0.0);
    assert_eq!(
        p.snap.tiles.len(),
        p.snap.size.0 as usize * p.snap.size.1 as usize
    );
    assert!((1..=signature::PHOTO_CAPTIONS).contains(&p.caption));
    // the night's own highlights are untouched: run to the morning and keep the best
    let mut guard = 0;
    while w.phase() == Phase::Night && guard < 6000 {
        w.tick(&c);
        bot::finish_cutscenes(&mut w, &c);
        guard += 1;
    }
    let (night, cands) = w.pending_polaroids().expect("the morning pick");
    assert_eq!(night, 3);
    assert!(!cands.is_empty());
    w.apply(&c, Command::PickPolaroid { index: 0 }).unwrap();
    assert!(w.journal().iter().any(|e| e.night == 3));
    assert_eq!(w.photos().len(), per as usize);
    // a new night, new film
    w.start_night(&c);
    bot::finish_cutscenes(&mut w, &c);
    assert_eq!(w.photos_left(), per);
}

#[test]
fn saves_keep_the_drink_and_the_photos_and_old_saves_load() {
    let c = content();
    let mut w = story_night(&c, 6);
    compose(
        &mut w,
        &c,
        "cocoa",
        &["milk", "spice"],
        MiniGameKind::MilkFroth,
        "Тихий час",
    )
    .unwrap();
    w.apply(
        &c,
        Command::TakePhoto {
            focus: (700.0, 420.0),
            width: 576.0,
        },
    )
    .unwrap();
    let text = w.save_to_string().unwrap();
    let back = World::load_from_str(&text).unwrap();
    assert_eq!(back.signature(), w.signature());
    assert_eq!(back.photos(), w.photos());
    assert_eq!(back.save_to_string().unwrap(), text);
    // a save from before package I has no drink and no photos
    let old = include_str!("testdata/save_v1_story_day3.ron");
    let w = World::load_from_str(old).unwrap();
    assert!(w.signature().is_none() && w.photos().is_empty());
}

#[test]
fn the_player_bot_sometimes_makes_one() {
    let c = content();
    let mut made = 0;
    let seeds = [1u64, 2, 3, 4, 5, 6];
    for seed in seeds {
        let mut w = World::new_story_seeded(&c, Difficulty::Cozy, seed);
        w.sys.tutorial_off = true;
        let mut b = bot::PlayerBot::new(bot::BotConfig {
            seed,
            ..Default::default()
        });
        // the days are enough: the bot decides in the day phase
        for n in 2..=9 {
            w.night = n;
            w.begin_day(&c);
            bot::finish_cutscenes(&mut w, &c);
            b.day(&mut w, &c);
        }
        if let Some(s) = w.signature() {
            made += 1;
            assert!(s.made_night >= signature::data().from_night);
            assert!(w.knowledge().recipes.contains_key(SIGNATURE_RECIPE));
        }
    }
    assert!(
        made >= 1 && made < seeds.len(),
        "sometimes, not always ({made} of {})",
        seeds.len()
    );
}
