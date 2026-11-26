//! The walk home and the back step (design/v0.2.md F, `sim::walk`): once a day / once a night,
//! the route's modifier only for the next night, the message pool follows the story acts,
//! nothing story-relevant changes, old saves load.

use super::*;
use crate::content::Content;
use crate::content::handson::HallMood;

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

/// Story night `n`, doors open, `minutes` after 22:00.
fn story_night_at(c: &Content, n: u32, minutes: u32) -> World {
    let mut w = story_day(c, n);
    w.start_night(c);
    bot::finish_cutscenes(&mut w, c);
    w.skip_prep(c);
    for _ in 0..minutes * 10 {
        w.tick(c);
        bot::finish_cutscenes(&mut w, c);
    }
    w.drain_events();
    w
}

#[test]
fn the_data_parses_and_every_key_has_all_four_languages() {
    let d = walk::parse(include_str!("../../assets/data/walk_home.ron")).unwrap();
    assert_eq!(d.routes.len(), 4);
    let ids: Vec<&str> = d.routes.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, ["embankment", "park", "bakery", "tram"]);
    assert!(walk::route(&d.default_route).is_some());
    let i18n = crate::i18n::I18n::load(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/i18n"))
        .unwrap_or_else(|e| panic!("{e}"));
    let mut keys = vec![
        "walk.title".to_string(),
        "walk.skip".to_string(),
        "walk.home".to_string(),
        "walk.backstep.chip".to_string(),
        "walk.backstep.chip_tip".to_string(),
        "walk.backstep.back".to_string(),
    ];
    for r in &d.routes {
        keys.push(r.name_key());
        keys.push(r.hint_key());
        keys.push(format!("walk.route.{}.tonight", r.id));
        assert_eq!(r.beats.len(), 2, "{}: two lines on the way", r.id);
        for beat in &r.beats {
            let last = beat.last().unwrap();
            assert!(
                last.seasons.is_empty() && last.weather.is_empty(),
                "{}: every beat ends with a plain line",
                r.id
            );
            keys.extend(beat.iter().map(|v| v.key.clone()));
        }
    }
    keys.push("walk.route.tram.arrived".into());
    for m in &d.backstep.messages {
        keys.push(m.text_key());
        keys.push(m.from.name_key());
    }
    for lang in crate::i18n::Lang::ALL {
        for k in &keys {
            assert!(i18n.has(lang, k), "{k} missing in {}", lang.code());
        }
    }
    // every act has its own messages, and some are for any night
    for act in 1..=4 {
        assert!(
            d.backstep.messages.iter().any(|m| m.acts == [act]),
            "act {act}"
        );
    }
    assert!(
        d.backstep
            .messages
            .iter()
            .filter(|m| m.acts.is_empty())
            .count()
            >= 3
    );
}

#[test]
fn walking_home_is_once_a_morning_and_only_in_the_day() {
    let c = content();
    let mut w = story_day(&c, 4);
    assert_eq!(w.usual_route(), "embankment", "the default way home");
    assert!(!w.walked_home_today());
    assert_eq!(
        w.apply(
            &c,
            Command::WalkHome {
                route: "nowhere".into()
            }
        ),
        Err(CommandError::UnknownId("nowhere".into()))
    );
    w.apply(
        &c,
        Command::WalkHome {
            route: "park".into(),
        },
    )
    .unwrap();
    assert!(w.walked_home_today());
    assert_eq!(w.usual_route(), "park", "the pick becomes her usual way");
    assert!(matches!(
        w.apply(
            &c,
            Command::WalkHome {
                route: "tram".into()
            }
        ),
        Err(CommandError::Invalid(_))
    ));
    w.start_night(&c);
    bot::finish_cutscenes(&mut w, &c);
    assert_eq!(
        w.apply(
            &c,
            Command::WalkHome {
                route: "park".into()
            }
        ),
        Err(CommandError::WrongPhase)
    );
    assert_eq!(w.walk().walks.get("park"), Some(&1));
}

#[test]
fn each_route_gives_only_the_next_night_a_small_modifier() {
    let c = content();
    // bakery: coziness +3 for the first hours, a toast at 21:30
    let mut plain = story_day(&c, 5);
    let mut baked = plain.clone();
    baked
        .apply(
            &c,
            Command::WalkHome {
                route: "bakery".into(),
            },
        )
        .unwrap();
    plain.start_night(&c);
    baked.start_night(&c);
    let ev = baked.drain_events();
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::Toast { key } if key == "walk.route.bakery.tonight"))
    );
    assert_eq!(baked.walk_tonight(), Some("bakery"));
    assert_eq!(plain.walk_tonight(), None);
    let (a, b) = (plain.coziness().value, baked.coziness().value);
    assert!(b > a && b - a <= 3.01, "kolač on the counter: {a} → {b}");
    // the next night without a walk: nothing left over
    let mut next = baked.clone();
    next.run_night_headless(&c);
    bot::finish_cutscenes(&mut next, &c);
    next.start_night(&c);
    assert_eq!(next.walk_tonight(), None);

    // park: a calm hall
    let mut park = story_day(&c, 5);
    park.apply(
        &c,
        Command::WalkHome {
            route: "park".into(),
        },
    )
    .unwrap();
    park.start_night(&c);
    assert_eq!(park.hall_mood(&c), Some(HallMood::Calm));

    // tram: the nurse she met comes in tonight with a wish
    let mut tram = story_day(&c, 5);
    tram.apply(
        &c,
        Command::WalkHome {
            route: "tram".into(),
        },
    )
    .unwrap();
    tram.start_night(&c);
    let spawn = tram
        .spawns
        .iter()
        .find(|s| s.archetype == "nurses" && s.wish)
        .expect("a walk-in from the tram");
    assert_eq!(spawn.size, 1);

    // embankment: steadier quick brews
    let mut river = story_day(&c, 5);
    river
        .apply(
            &c,
            Command::WalkHome {
                route: "embankment".into(),
            },
        )
        .unwrap();
    river.start_night(&c);
    assert!(river.sys.fx.quick_quality.iter().any(|q| q.0 > 0.0));
}

#[test]
fn a_walk_never_touches_the_rng_money_or_the_story() {
    let c = content();
    let mut w = story_day(&c, 6);
    let before = (
        w.rng.clone(),
        w.money(),
        w.story().flags.clone(),
        w.story().counters.clone(),
    );
    w.apply(
        &c,
        Command::WalkHome {
            route: "tram".into(),
        },
    )
    .unwrap();
    let after = (
        w.rng.clone(),
        w.money(),
        w.story().flags.clone(),
        w.story().counters.clone(),
    );
    assert_eq!(format!("{before:?}"), format!("{after:?}"));
}

#[test]
fn the_back_step_is_once_a_night_after_half_past_ten_and_never_in_a_rush() {
    let c = content();
    let mut w = story_day(&c, 3);
    assert_eq!(w.backstep_block(&c), Some(walk::BackstepBlock::NotNight));
    assert_eq!(
        w.apply(&c, Command::StepOutBack),
        Err(CommandError::WrongPhase)
    );
    w.start_night(&c);
    bot::finish_cutscenes(&mut w, &c);
    assert_eq!(
        w.backstep_block(&c),
        Some(walk::BackstepBlock::DoorsClosed),
        "not in the preparation"
    );
    w.skip_prep(&c);
    assert_eq!(w.backstep_block(&c), Some(walk::BackstepBlock::TooEarly));

    let mut w = story_night_at(&c, 3, 60);
    // a rush holds it
    w.hands.rush.until = Some(w.minute() + 10);
    assert_eq!(w.backstep_block(&c), Some(walk::BackstepBlock::Rush));
    assert!(matches!(
        w.apply(&c, Command::StepOutBack),
        Err(CommandError::Invalid(_))
    ));
    w.hands.rush.until = None;
    assert_eq!(w.backstep_block(&c), None);
    w.apply(&c, Command::StepOutBack).unwrap();
    let m = w.backstep_message().expect("a message tonight");
    assert_eq!(m.acts, [1], "an act-1 message on night 3");
    assert_eq!(
        w.backstep_block(&c),
        Some(walk::BackstepBlock::AlreadyTonight)
    );
    assert!(w.apply(&c, Command::StepOutBack).is_err());
    assert_eq!(w.walk().breaks, 1);
}

#[test]
fn the_messages_follow_the_acts_and_rotate_without_repeats() {
    let c = content();
    let mut shown = Vec::new();
    let mut w = story_day(&c, 1);
    for night in [2, 3, 4, 5, 9, 10, 18, 26] {
        w.night = night;
        w.phase = Phase::Night;
        w.prep_left = 0;
        w.tick = 60 * 10;
        w.hands.rush.until = None;
        w.apply(&c, Command::StepOutBack).unwrap();
        let m = w.backstep_message().unwrap();
        shown.push((night, m.id.clone(), m.acts.clone()));
    }
    // act 1 (nights 2–5): its three messages first, then general ones
    assert_eq!(shown[0].2, [1]);
    assert_eq!(shown[1].2, [1]);
    assert_eq!(shown[2].2, [1]);
    assert!(shown[3].2.is_empty(), "act 1 used up → a general message");
    assert_eq!(shown[4].2, [2]);
    assert_eq!(shown[6].2, [3]);
    assert_eq!(shown[7].2, [4]);
    let ids: std::collections::BTreeSet<&str> = shown.iter().map(|s| s.1.as_str()).collect();
    assert_eq!(
        ids.len(),
        shown.len(),
        "no repeats while unseen ones are left"
    );
    // the sandbox only gets the general messages
    let mut s = World::new_sandbox(&c, SandboxSetup::default());
    s.phase = Phase::Night;
    s.prep_left = 0;
    s.tick = 90 * 10;
    s.apply(&c, Command::StepOutBack).unwrap();
    assert!(s.backstep_message().unwrap().acts.is_empty());
}

#[test]
fn the_lines_follow_the_season_and_the_weather() {
    let r = walk::route("embankment").unwrap();
    let plain = walk::lines(r, Some("late_autumn"), crate::content::Weather::Clear);
    assert_eq!(plain, ["walk.line.embankment.1", "walk.line.embankment.2"]);
    let rain = walk::lines(r, Some("late_autumn"), crate::content::Weather::Rain);
    assert_eq!(rain[0], "walk.line.embankment.1.rain");
    let snow = walk::lines(r, Some("first_snow"), crate::content::Weather::Snow);
    assert_eq!(
        snow,
        [
            "walk.line.embankment.1.snow",
            "walk.line.embankment.2.festive"
        ]
    );
    for route in &walk::data().routes {
        for season in ["late_autumn", "frost", "advent", "first_snow", "summer"] {
            for weather in crate::content::Weather::ALL {
                assert_eq!(walk::lines(route, Some(season), weather).len(), 2);
            }
        }
    }
}

#[test]
fn saves_keep_the_walk_and_old_saves_load_without_it() {
    let c = content();
    let mut w = story_day(&c, 7);
    w.apply(
        &c,
        Command::WalkHome {
            route: "bakery".into(),
        },
    )
    .unwrap();
    let text = w.save_to_string().unwrap();
    let back = World::load_from_str(&text).unwrap();
    assert_eq!(back.walk(), w.walk());
    // a save from before the walk (no `walk` field) loads with nothing planned
    let old = text.replace(&walk_field(&text), "");
    assert!(
        !old.contains("planned"),
        "the field is gone from the old save"
    );
    let old = World::load_from_str(&old).unwrap();
    assert_eq!(old.walk(), &walk::WalkState::default());
    // a 0.1.x fixture too
    for f in ["save_v1_story_day3.ron", "save_v1_sandbox_night.ron"] {
        let path = format!("{}/src/sim/testdata/{f}", env!("CARGO_MANIFEST_DIR"));
        let t = std::fs::read_to_string(&path).unwrap();
        let w = World::load_from_str(&t).unwrap();
        assert_eq!(w.walk(), &walk::WalkState::default());
    }
}

/// The `walk: (...)` entry of a RON save (balanced parentheses).
fn walk_field(text: &str) -> String {
    let start = text.find("walk:").expect("walk field");
    let open = start + text[start..].find('(').unwrap();
    let mut depth = 0;
    let mut end = open;
    for (i, ch) in text[open..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end = open + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    let mut s = text[start..end].to_string();
    if text[end..].starts_with(',') {
        s.push(',');
    }
    s
}

/// The whole Cozy story played by the attentive bot, walking home every morning (each route in
/// turn) and stepping out back every night: the same ending, every arc good.
#[test]
fn walking_home_and_stepping_out_every_night_keeps_the_light_ending() {
    let c = content();
    let seed = 4242;
    let mut w = World::new_story_seeded(&c, Difficulty::Cozy, seed);
    let mut b = bot::PlayerBot::new(bot::BotConfig {
        seed,
        ..Default::default()
    });
    let routes = ["bakery", "park", "tram", "embankment"];
    let mut breaks = 0;
    for n in 1..=30u32 {
        if w.is_closed() {
            break;
        }
        if w.phase() == Phase::Day {
            b.day(&mut w, &c);
            bot::finish_cutscenes_attentive(&mut w, &c);
            if n > 1 {
                w.apply(
                    &c,
                    Command::WalkHome {
                        route: routes[n as usize % 4].into(),
                    },
                )
                .unwrap();
            }
            w.start_night(&c);
        }
        let mut guard = 0;
        let mut stepped = false;
        while w.phase() == Phase::Night && !w.is_closed() && guard < 100_000 {
            bot::finish_cutscenes_attentive(&mut w, &c);
            if !stepped && w.minute() >= 90 && w.backstep_block(&c).is_none() {
                w.apply(&c, Command::StepOutBack).unwrap();
                stepped = true;
                breaks += 1;
            }
            b.step(&mut w, &c);
            w.tick(&c);
            w.drain_events();
            guard += 1;
        }
        bot::finish_cutscenes_attentive(&mut w, &c);
    }
    assert!(breaks >= 25, "a break most nights ({breaks})");
    assert_eq!(w.ending(), Some("light"));
    for r in ["mirek", "hana", "vera", "filip", "radek", "lori"] {
        assert!(w.flag(&format!("arc.{r}.good")), "{r}: no good ending");
    }
}
