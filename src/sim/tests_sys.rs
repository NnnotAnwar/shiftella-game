//! T5 systems tests (design/handson.md §4–§6, §11–§13): the hands-on effect interpreter,
//! experiments & margin notes, the moment director and moments, the cat, napkins and the tip
//! jar, coziness / seasons / themes, the journal, the rozpis puzzle and requests, the app as
//! a tool (rituals, handover, bot buttons, insights, payroll), determinism, and a 30-night
//! player-bot run that touches every system.

use super::systems::XTarget;
use super::*;
use crate::content::handson::{Cond, Fx, HighlightKind, PrefKind, Strength, TipSplit, XCondition};
use crate::content::{Content, Effect, Weekday};

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

fn content() -> Content {
    Content::load(DATA).unwrap_or_else(|e| panic!("{e}"))
}

/// A story night `n` running, cutscenes done, no random walk-ins.
fn night(c: &Content, n: u32) -> World {
    let mut w = World::new_story(c, Difficulty::Cozy);
    w.sys.tutorial_off = true;
    if n > 1 {
        w.night = n;
        w.begin_day(c);
    }
    bot::finish_cutscenes(&mut w, c);
    w.start_night(c);
    bot::finish_cutscenes(&mut w, c);
    w.skip_prep(c);
    quiet(&mut w);
    w.drain_events();
    w
}

fn quiet(w: &mut World) {
    w.mods.push(Modifier {
        kind: ModKind::GuestRate,
        mul: 0.0,
        until: None,
        event: None,
    });
}

/// No random moments tonight (the tests spawn them).
fn no_director(w: &mut World) {
    w.sys.moments.budget = 0;
    w.sys.moments.cap = 0;
}

fn ticks(w: &mut World, c: &Content, n: u32) -> Vec<SimEvent> {
    let mut out = Vec::new();
    for _ in 0..n {
        w.tick(c);
        bot::finish_cutscenes(w, c);
        out.extend(w.drain_events());
    }
    out
}

fn to_day(w: &mut World, c: &Content) -> Vec<SimEvent> {
    let mut out = Vec::new();
    while w.phase() == Phase::Night {
        w.tick(c);
        bot::finish_cutscenes(w, c);
        out.extend(w.drain_events());
    }
    bot::finish_cutscenes(w, c);
    out.extend(w.drain_events());
    out
}

fn fx(text: &str) -> Vec<Fx> {
    ron::from_str(&format!("#![enable(implicit_some)]\n{text}")).unwrap_or_else(|e| panic!("{e}"))
}

// ---------------------------------------------------------------------------------------------
// The Fx / Cond interpreter
// ---------------------------------------------------------------------------------------------

#[test]
fn fx_later_next_night_if_maybe_and_targets() {
    let c = content();
    let mut w = night(&c, 2);
    no_director(&mut w);
    let t = XTarget::default();
    xlogic::run(
        &mut w,
        &c,
        &fx(r#"[Later(minutes: 5, effects: [SetFlag("t5_later")]),
                NextNight([SetFlag("t5_next")]),
                If(conditions: [NightAtLeast(2)], then: [SetFlag("t5_if")], otherwise: [SetFlag("t5_else")]),
                Maybe(chance: 1.0, effects: [UnlockIngredient("cocoa")]),
                LearnRecipe("r_turek"),
                Coziness(add: 5.0, minutes: None),
                WishBoost(tags: [Cold], mul: 2.0, minutes: 30)]"#),
        &t,
    );
    assert!(w.flag("t5_if") && !w.flag("t5_else"));
    assert!(w.pantry().contains("cocoa"));
    assert!(w.known_recipes().any(|r| r == "r_turek"));
    assert_eq!(w.sys.fx.wish_boosts.len(), 1);
    assert!(!w.flag("t5_later"));
    ticks(&mut w, &c, 6 * TICKS_PER_MINUTE);
    assert!(w.flag("t5_later"), "Later ran after its minutes");
    assert!(!w.flag("t5_next"));
    to_day(&mut w, &c);
    w.start_night(&c);
    assert!(w.flag("t5_next"), "NextNight ran at the next night start");
    // (guests come once the doors are open)
    w.skip_prep(&c);
    // a target guest
    let pid =
        guests::spawn_party(&mut w, &c, "travelers", 1, None, 1.0, 1.0, &[], None, false).unwrap();
    let before = w.parties.iter().find(|p| p.id == pid).unwrap().satisfaction;
    let target = XTarget {
        party: Some(pid),
        ..Default::default()
    };
    xlogic::run(&mut w, &c, &fx("[TargetSatisfaction(7.0)]"), &target);
    let after = w.parties.iter().find(|p| p.id == pid).unwrap().satisfaction;
    assert!((after - (before + 7.0).min(100.0)).abs() < 0.01);
}

#[test]
fn extension_conditions_read_the_t5_state() {
    let c = content();
    let mut w = night(&c, 2);
    let holds = |w: &World, x: XCondition| xlogic::holds(w, &c, &[Cond::X(x)]);
    assert!(!holds(
        &w,
        XCondition::MomentResolvedEver("m_station_cat".into())
    ));
    w.sys.moments.resolved_ever.insert("m_station_cat".into());
    assert!(holds(
        &w,
        XCondition::MomentResolvedEver("m_station_cat".into())
    ));
    w.sys.atmos.coziness.value = 70.0;
    assert!(holds(&w, XCondition::CozinessAtLeast(65.0)));
    assert!(!holds(&w, XCondition::CozinessAtLeast(75.0)));
    assert!(holds(&w, XCondition::ThemeOwned("grandma_original".into())));
    assert!(!holds(&w, XCondition::ThemeOwned("art_nouveau".into())));
    assert!(!holds(&w, XCondition::NapkinsAtLeast(1)));
    w.sys.extras.napkins.push("g01".into());
    assert!(holds(&w, XCondition::NapkinsAtLeast(1)));
    w.sys
        .rozpis
        .shared
        .insert(("barista".into(), "server".into()), 3);
    assert!(holds(
        &w,
        XCondition::SharedShifts {
            a: "server".into(),
            b: "barista".into(),
            shifts: 3
        }
    ));
}

#[test]
fn arc_stages_accept_hands_on_conditions() {
    let c = content();
    let mirek = c.regular("mirek").unwrap();
    let st = mirek.stage(1).unwrap();
    assert!(
        format!("{:?}", st.when).contains("WishMatchStreak"),
        "Mirek's first stage counts well-matched coffees"
    );
    let vera3 = c.regular("vera").unwrap().stage(3).unwrap();
    assert!(format!("{:?}", vera3.when).contains("ServedRecipe"));
    // three good matches in a row hold the condition
    let mut w = night(&c, 3);
    let rs = w.story.regulars.entry("mirek".into()).or_default();
    for n in 1..=3 {
        rs.visits.push(VisitRecord {
            night: n,
            served_minutes: Some(30),
            seat_free: true,
            clean: 1.0,
            best_match: Some(0.9),
        });
    }
    assert!(xlogic::holds(&w, &c, &st.when));
}

// ---------------------------------------------------------------------------------------------
// Recipes: experiments & margin notes
// ---------------------------------------------------------------------------------------------

fn try_jars(w: &mut World, c: &Content, jars: &[&str]) -> ExperimentResult {
    w.apply(
        c,
        Command::Experiment {
            ingredients: jars.iter().map(|s| s.to_string()).collect(),
        },
    )
    .unwrap();
    w.drain_events()
        .into_iter()
        .find_map(|e| match e {
            SimEvent::ExperimentDone { result } => Some(result),
            _ => None,
        })
        .expect("a result")
}

#[test]
fn experiments_know_discover_hint_fairly_and_leave_margin_notes() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    bot::finish_cutscenes(&mut w, &c);
    w.drain_events();
    // validation
    assert!(matches!(
        w.apply(
            &c,
            Command::Experiment {
                ingredients: vec!["espresso".into()]
            }
        ),
        Err(CommandError::Invalid(_))
    ));
    assert_eq!(
        w.apply(
            &c,
            Command::Experiment {
                ingredients: vec!["espresso".into(), "chili".into()]
            }
        ),
        Err(CommandError::Locked)
    );
    // a known one
    assert_eq!(
        try_jars(&mut w, &c, &["milk", "espresso"]),
        ExperimentResult::Known {
            recipe: "r_cappuccino".into()
        }
    );
    // so close: Mirek's turek wants something sweet instead of the cream
    match try_jars(&mut w, &c, &["espresso", "cream"]) {
        ExperimentResult::Close { key, family } => {
            assert_eq!(key, "experiment.close.swap");
            assert!(family.is_some());
        }
        r => panic!("{r:?}"),
    }
    // discovered (free on Cozy early nights)
    let supplies = w.supplies();
    assert_eq!(
        try_jars(&mut w, &c, &["espresso", "sugar"]),
        ExperimentResult::Discovered {
            recipe: "r_turek".into()
        }
    );
    assert_eq!(w.supplies(), supplies, "free on Cozy nights 1–7");
    assert!(w.known_recipes().any(|r| r == "r_turek"));
    // three failed tries → grandma's margin note on a fair silhouette
    let before: usize = w.knowledge().revealed.values().map(Vec::len).sum();
    for jars in [["lemon", "bread"], ["garlic", "honey"], ["ice", "potato"]] {
        try_jars(&mut w, &c, &jars);
    }
    let after: usize = w.knowledge().revealed.values().map(Vec::len).sum();
    assert_eq!(after, before + 1, "one ingredient revealed");
    let recipe = w.knowledge().revealed.keys().next().unwrap().clone();
    let r = c.handson.recipe(&recipe).unwrap();
    assert!(
        r.ingredients.iter().all(|i| w.pantry().contains(i)),
        "margin notes only point at recipes she can make: {recipe}"
    );
    // after the free nights an experiment costs supplies
    w.night = 9;
    let s = w.supplies();
    try_jars(&mut w, &c, &["milk", "espresso"]);
    assert_eq!(w.supplies(), s - c.handson.tuning.experiment_cost as i64);
}

#[test]
fn a_night_experiment_is_karolinas_act_at_the_counter() {
    let c = content();
    let mut w = night(&c, 2);
    no_director(&mut w);
    w.apply(
        &c,
        Command::Experiment {
            ingredients: vec!["espresso".into(), "sugar".into()],
        },
    )
    .unwrap();
    assert!(
        w.player()
            .queue
            .iter()
            .any(|a| matches!(a, PlayerAct::GoTo { .. }))
    );
    assert!(matches!(
        w.walk_jobs().first().map(|j| &j.kind),
        Some(WalkKind::Experiment { .. })
    ));
    let ev = ticks(&mut w, &c, 900);
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::ExperimentDone { .. }))
    );
    assert!(ev.iter().any(|e| matches!(e,
        SimEvent::RecipeLearned { recipe, source } if recipe == "r_turek" && source == "experiment")));
}

// ---------------------------------------------------------------------------------------------
// Moments
// ---------------------------------------------------------------------------------------------

#[test]
fn the_director_gives_a_few_moments_a_night_and_one_at_a_time_early() {
    let c = content();
    let mut totals = Vec::new();
    for n in [2u32, 3, 12, 18] {
        let mut w = World::new_story(&c, Difficulty::Cozy);
        if n > 1 {
            w.night = n;
            w.begin_day(&c);
        }
        bot::finish_cutscenes(&mut w, &c);
        w.start_night(&c);
        let mut appeared = 0;
        while w.phase() == Phase::Night {
            w.tick(&c);
            bot::finish_cutscenes(&mut w, &c);
            for e in w.drain_events() {
                if let SimEvent::MomentAvailable { id, .. } = e
                    && id != "m_leak_empty"
                {
                    appeared += 1;
                }
            }
            if n <= 3 {
                assert!(w.active_moments().len() <= 1, "night {n}: one at a time");
            }
        }
        assert!(appeared <= 6, "night {n}: {appeared} moments");
        totals.push(appeared);
    }
    assert!(totals.iter().sum::<u32>() >= 4, "{totals:?}");
}

#[test]
fn resolving_a_moment_walks_karolina_there_and_applies_on_arrival() {
    let c = content();
    let mut w = night(&c, 4);
    no_director(&mut w);
    // the cat replaces the event's toast
    w.debug_fire_event(&c, "station_cat");
    assert!(w.active_moments().iter().any(|m| m.id == "m_station_cat"));
    assert!(w.pending_choices().is_empty());
    let opts = w.moment_options(&c, "m_station_cat");
    assert!(opts.contains(&"ham".to_string()));
    let money = w.money();
    w.apply(
        &c,
        Command::ResolveMoment {
            id: "m_station_cat".into(),
            option: "ham".into(),
        },
    )
    .unwrap();
    assert!(
        w.active_moments()[0].resolving.is_some(),
        "the card closed, she's on her way"
    );
    assert!(
        w.apply(
            &c,
            Command::ResolveMoment {
                id: "m_station_cat".into(),
                option: "milk".into()
            }
        )
        .is_err()
    );
    let ev = ticks(&mut w, &c, 1200);
    assert!(ev.iter().any(|e| matches!(e,
        SimEvent::MomentResolved { id, option } if id == "m_station_cat" && option == "ham")));
    assert_eq!(w.money(), money - 15);
    assert_eq!(w.story().counters.get("cat_affection"), Some(&2));
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::CatLevel { level: 2 }))
    );
    assert_eq!(w.cat_level(), 2);
    assert!(w.active_moments().is_empty());
}

#[test]
fn an_unanswered_moment_fades_with_its_gentle_default() {
    let c = content();
    let mut w = night(&c, 4);
    no_director(&mut w);
    w.debug_fire_event(&c, "station_cat");
    let money = w.money();
    let ev = ticks(&mut w, &c, 30 * TICKS_PER_MINUTE);
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::MomentExpired { id } if id == "m_station_cat"))
    );
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::Toast { key } if key == "moment.m_station_cat.expired"))
    );
    assert_eq!(w.money(), money - 10, "on_expire: the crew gave it milk");
}

#[test]
fn the_leak_bucket_follow_up_and_garlands_in_advent() {
    let c = content();
    let mut w = night(&c, 5);
    no_director(&mut w);
    w.debug_fire_event(&c, "roof_leak");
    assert!(w.active_moments().iter().any(|m| m.id == "m_leak_bucket"));
    w.apply(
        &c,
        Command::ResolveMoment {
            id: "m_leak_bucket".into(),
            option: "bucket".into(),
        },
    )
    .unwrap();
    let ev = ticks(&mut w, &c, 70 * TICKS_PER_MINUTE);
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::MomentAvailable { id, .. } if id == "m_leak_empty"))
    );
    // advent: the garland box
    let mut w = night(&c, 21);
    assert!(w.season_phase(&c).is_some_and(|p| p.id == "advent"));
    if !w.active_moments().iter().any(|m| m.id == "m_hang_garlands") {
        assert!(moments::spawn(
            &mut w,
            &c,
            "m_hang_garlands",
            XTarget::default(),
            true
        ));
    }
    w.apply(
        &c,
        Command::ResolveMoment {
            id: "m_hang_garlands".into(),
            option: "hang".into(),
        },
    )
    .unwrap();
    let ev = ticks(&mut w, &c, 600);
    assert!(w.flag("garlands_up") && w.garlands_up(&c));
    assert!(w.theme_owned(&c, "winter_lights"));
    assert!(ev.iter().any(|e| matches!(
        e,
        SimEvent::HighlightCaptured {
            kind: HighlightKind::GarlandsUp,
            ..
        }
    )));
}

#[test]
fn the_lull_director_offers_a_moment_when_karolina_is_idle() {
    let c = content();
    let mut w = night(&c, 6);
    // nothing random, but the night's cap allows a nudge
    w.sys.moments.budget = 0;
    w.sys.moments.cap = 4;
    w.weather = crate::content::Weather::Rain;
    // 22:40: the busker's window (rain, before 01:00)
    w.tick = 40 * TICKS_PER_MINUTE;
    let ev = ticks(&mut w, &c, 60 * TICKS_PER_MINUTE);
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::KarolinaIdle { .. }))
    );
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::MomentAvailable { .. })),
        "an idle stretch brings a moment"
    );
}

// ---------------------------------------------------------------------------------------------
// The cat, napkins, the tip jar
// ---------------------------------------------------------------------------------------------

#[test]
fn the_cat_grows_fond_unlocks_a_latte_and_asks_for_a_name() {
    let c = content();
    let mut w = night(&c, 6);
    no_director(&mut w);
    xlogic::run(
        &mut w,
        &c,
        &[Fx::Core(Effect::AddCounter("cat_affection".into(), 3))],
        &XTarget::default(),
    );
    assert_eq!(w.cat_level(), 3);
    assert!(w.lattes().contains("cat"), "a cat-face latte at level 3");
    xlogic::run(
        &mut w,
        &c,
        &[Fx::Core(Effect::AddCounter("cat_affection".into(), 9))],
        &XTarget::default(),
    );
    assert_eq!(w.story().counters["cat_affection"], 10, "clamped to max");
    assert_eq!(w.cat_level(), 10);
    assert!(w.active_moments().iter().any(|m| m.id == "m_cat_name"));
    w.apply(
        &c,
        Command::ResolveMoment {
            id: "m_cat_name".into(),
            option: "kofi".into(),
        },
    )
    .unwrap();
    ticks(&mut w, &c, 1200);
    assert_eq!(w.story().vars["cat_name"], "kofi");
    assert_eq!(
        w.station_cat(&c).and_then(|v| v.name),
        Some("cat.name.kofi".into())
    );
}

#[test]
fn napkins_come_with_lovely_serves_and_reward_the_collection() {
    let c = content();
    let mut w = night(&c, 3);
    for i in 0..10 {
        moments::give_napkin(&mut w, &c, None, if i == 0 { Some("mirek") } else { None });
    }
    assert_eq!(w.napkins().len(), 10);
    assert_eq!(w.napkins()[0], "mirek.1", "regulars' napkins are personal");
    assert!(w.story().unlocked_furniture.contains("napkin_wall"));
    // a napkin lies next to the coin and is collected with it
    let pid =
        guests::spawn_party(&mut w, &c, "travelers", 1, None, 1.0, 1.0, &[], None, false).unwrap();
    w.parties.iter_mut().find(|p| p.id == pid).unwrap().napkin = Some("g20".into());
    guests::finish_party(&mut w, &c, pid, guests::Exit::Normal);
    player::collect_all_tips(&mut w, &c);
    assert!(w.napkins().iter().any(|n| n == "g20"));
}

#[test]
fn the_tip_jar_counts_goals_into_decor() {
    let c = content();
    let mut w = night(&c, 3);
    let coin = TipCoin {
        id: 1,
        table: 0,
        amount: 600,
        staff: None,
        minute: 0,
        napkin: None,
    };
    player::collect_coins(&mut w, &c, vec![coin], None);
    let (goal, choices) = w.tip_goal_pending(&c).expect("count the jar");
    assert_eq!(goal, 0);
    assert_eq!(choices.len(), 2);
    w.apply(
        &c,
        Command::PickTipGoal {
            choice: choices[1].clone(),
        },
    )
    .unwrap();
    assert!(w.story().unlocked_furniture.contains(&choices[1]));
    assert!(w.tip_goal_pending(&c).is_none());
}

// ---------------------------------------------------------------------------------------------
// Coziness, seasons, themes
// ---------------------------------------------------------------------------------------------

#[test]
fn chores_and_lamps_move_the_coziness_meter() {
    let c = content();
    let mut w = night(&c, 4);
    atmos::recompute(&mut w, &c);
    let clean = w.coziness().clone();
    w.dirt = 0.9;
    for p in w.furniture.iter_mut() {
        if c.furniture_def(&p.def).is_some_and(|d| d.has_tag("light")) {
            p.on = false;
        }
    }
    atmos::recompute(&mut w, &c);
    let dark = w.coziness().clone();
    assert!(
        dark.level < clean.level,
        "a dirty dark hall drops at least a level: {} → {}",
        clean.value,
        dark.value
    );
    // mopping and the lamps bring it back
    w.dirt = 0.0;
    for p in w.furniture.iter_mut() {
        if c.furniture_def(&p.def).is_some_and(|d| d.has_tag("light")) {
            p.on = true;
        }
    }
    atmos::recompute(&mut w, &c);
    assert!(w.coziness().level >= clean.level);
    // a cosier hall = more patience, more satisfaction once seated
    let hi = atmos::satisfaction_term(&w, &c);
    w.sys.atmos.coziness = dark;
    assert!(hi > atmos::satisfaction_term(&w, &c) + 4.0);
    w.sys.atmos.coziness.value = 80.0;
    assert!(atmos::patience_mul(&w, &c) > 1.0);
}

#[test]
fn seasons_follow_the_story_calendar_and_cycle_in_sandbox() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    let mut phases = Vec::new();
    for n in [1u32, 13, 14, 21, 27, 30] {
        w.night = n;
        w.begin_day(&c);
        bot::finish_cutscenes(&mut w, &c);
        phases.push(w.season_phase(&c).map(|p| p.id.clone()).unwrap_or_default());
    }
    assert_eq!(
        phases,
        [
            "late_autumn",
            "late_autumn",
            "frost",
            "advent",
            "first_snow",
            "first_snow"
        ]
    );
    assert!(
        w.lattes().contains("snowflake"),
        "frost brings the snowflake latte"
    );
    let mut s = World::new_sandbox(&c, SandboxSetup::default());
    let mut seen = std::collections::BTreeSet::new();
    for n in 1..=120 {
        s.night = n;
        s.begin_day(&c);
        if let Some(p) = s.season_phase(&c) {
            seen.insert(p.id.clone());
        }
    }
    assert!(seen.len() >= 4, "sandbox cycles: {seen:?}");
}

#[test]
fn themes_unlock_buy_and_switch_by_day_with_undo() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    bot::finish_cutscenes(&mut w, &c);
    assert_eq!(w.theme(&c), "grandma_original");
    assert_eq!(
        w.apply(
            &c,
            Command::BuyTheme {
                id: "station_retro".into()
            }
        ),
        Err(CommandError::Locked)
    );
    w.night = 18;
    w.begin_day(&c);
    bot::finish_cutscenes(&mut w, &c);
    assert!(w.themes_unlocked().contains("station_retro"));
    w.debug_set_money(20_000);
    w.apply(
        &c,
        Command::BuyTheme {
            id: "station_retro".into(),
        },
    )
    .unwrap();
    assert_eq!(w.money(), 16_000);
    w.apply(
        &c,
        Command::SetTheme {
            id: "station_retro".into(),
        },
    )
    .unwrap();
    assert_eq!(w.theme(&c), "station_retro");
    w.undo(&c).unwrap();
    assert_eq!(w.theme(&c), "grandma_original");
    w.start_night(&c);
    assert_eq!(
        w.apply(
            &c,
            Command::SetTheme {
                id: "station_retro".into()
            }
        ),
        Err(CommandError::WrongPhase)
    );
}

// ---------------------------------------------------------------------------------------------
// The journal
// ---------------------------------------------------------------------------------------------

#[test]
fn every_night_leaves_a_polaroid_to_keep_and_it_survives_a_save() {
    let c = content();
    let mut w = night(&c, 2);
    to_day(&mut w, &c);
    let (n, list) = w.pending_polaroids().expect("candidates");
    assert_eq!(n, 2);
    assert!(!list.is_empty() && list.len() <= 3);
    assert!(list.windows(2).all(|p| p[0].score >= p[1].score));
    let snap = &list[0].snap;
    assert_eq!(
        snap.tiles.len(),
        snap.size.0 as usize * snap.size.1 as usize
    );
    assert!(!snap.props.is_empty());
    w.apply(&c, Command::PickPolaroid { index: 0 }).unwrap();
    assert_eq!(w.journal().len(), 1);
    assert_eq!(w.journal()[0].date, Some((2039, 11, 8)));
    // closing the morning without a pick keeps the best one
    bot::finish_cutscenes(&mut w, &c);
    w.start_night(&c);
    to_day(&mut w, &c);
    bot::finish_cutscenes(&mut w, &c);
    w.start_night(&c);
    assert_eq!(w.journal().len(), 2);
    let text = w.save_to_string().unwrap();
    let back = World::load_from_str(&text).unwrap();
    assert_eq!(back.journal(), w.journal());
    let one = ron::to_string(&w.journal()[0]).unwrap();
    assert!(one.len() < 12_000, "a polaroid is a few KB: {}", one.len());
}

// ---------------------------------------------------------------------------------------------
// The rozpis puzzle
// ---------------------------------------------------------------------------------------------

/// Day of story night `n` with the first three story people hired.
fn crew_day(c: &Content, n: u32) -> World {
    let mut w = World::new_story(c, Difficulty::Cozy);
    bot::finish_cutscenes(&mut w, c);
    w.night = n;
    w.begin_day(c);
    bot::finish_cutscenes(&mut w, c);
    for arch in ["barista", "cook", "porter"] {
        let id = w
            .candidates()
            .iter()
            .find(|x| x.staff.archetype == arch)
            .map(|x| x.id)
            .expect(arch);
        w.apply(c, Command::Hire { candidate: id }).unwrap();
    }
    w
}

fn id_of(w: &World, arch: &str) -> u32 {
    w.staff().iter().find(|s| s.archetype == arch).unwrap().id
}

#[test]
fn story_people_bring_their_preferences_and_wellbeing_scores_the_draft() {
    let c = content();
    let mut w = crew_day(&c, 4);
    let cook = id_of(&w, "cook");
    assert!(
        w.prefs(cook)
            .iter()
            .any(|p| p.kind == PrefKind::NoBackToBack && p.strength == Strength::Need)
    );
    // the auto-fill keeps every Need
    w.apply(&c, Command::AutoFillRozpis).unwrap();
    for s in w.staff().iter().filter(|s| !s.owner) {
        for (p, st) in w.pref_status(&c, s.id) {
            if p.strength == Strength::Need {
                assert_ne!(st, PrefStatus::Violated, "{} {:?}", s.archetype, p.kind);
            }
        }
    }
    assert!(w.autofill_report().total > 0);
    // breaking a Need by hand costs wellbeing
    let wb = w.wellbeing(cook).unwrap();
    for d in 0..7 {
        w.apply(
            &c,
            Command::SetShift {
                staff: cook,
                weekday: d,
                shift: Shift::Full,
            },
        )
        .unwrap();
    }
    let wb2 = w.wellbeing(cook).unwrap();
    assert!(wb2 <= wb - 25.0, "{wb} → {wb2}");
    assert!(w.week_fairness() < 100.0);
    let cov = w.coverage(&c);
    assert!(cov.iter().all(|n| n.on >= 1 && (1..=3).contains(&n.cups)));
}

#[test]
fn a_happy_week_is_reachable_with_the_story_crew() {
    let c = content();
    let mut w = crew_day(&c, 4);
    let (b, k, p) = (id_of(&w, "barista"), id_of(&w, "cook"), id_of(&w, "porter"));
    use Shift::*;
    // Bára late (Sundays off), Zdeněk never two nights in a row, Kuba with Zdeněk
    let rows = [
        (b, [Late, Off, Late, Late, Late, Late, Off]),
        (k, [Full, Off, Full, Off, Full, Off, Off]),
        (p, [Full, Full, Off, Full, Full, Full, Full]),
    ];
    for (id, row) in rows {
        for (d, sh) in row.iter().enumerate() {
            w.apply(
                &c,
                Command::SetShift {
                    staff: id,
                    weekday: d as u8,
                    shift: *sh,
                },
            )
            .unwrap();
        }
    }
    for id in [b, k, p] {
        for (pref, st) in w.pref_status(&c, id) {
            assert_ne!(st, PrefStatus::Violated, "{id}: {:?}", pref.kind);
        }
    }
    // (T9: since the auto-fill keeps the story crew's wishes, the app's own plan may already
    // have made this week happy at the hire; then the event came then, once a week)
    let mut ev = w.drain_events();
    w.apply(&c, Command::PublishSchedule).unwrap();
    ev.extend(w.drain_events());
    let rows = rozpis::effective(&w, &c, &w.app().published);
    assert!(rozpis::happy(&w, &c, &rows));
    assert!(ev.contains(&SimEvent::RozpisHappy));
    assert!(
        ev.iter().any(
            |e| matches!(e, SimEvent::BotMessage { template, .. } if template == "rozpis_happy")
        )
    );
    // a well-planned week: fewer late arrivals and sick days
    let (late, sick) = rozpis::late_sick_mul(&w, &c, b);
    assert!(late < 1.0 && sick < 1.0);
}

fn to_next_day(w: &mut World, c: &Content) {
    bot::finish_cutscenes(w, c);
    w.start_night(c);
    to_day(w, c);
    bot::finish_cutscenes(w, c);
}

#[test]
fn life_events_become_requests_with_consequences() {
    let c = content();
    let mut w = crew_day(&c, 7);
    // night 8: Bára's exam week → a Need for Friday off + a time-off request
    to_next_day(&mut w, &c);
    let b = id_of(&w, "barista");
    assert!(
        w.prefs(b)
            .iter()
            .any(|p| p.kind == PrefKind::DayOff(Weekday::Fri) && p.strength == Strength::Need)
    );
    let req = w
        .app()
        .time_off
        .iter()
        .find(|r| r.staff == b && r.status == RequestStatus::Pending)
        .map(|r| r.id)
        .expect("a time-off request");
    assert!(
        w.request_meta(req)
            .is_some_and(|m| m.reason_key.as_deref() == Some("app.request.barista_exam"))
    );
    w.apply(
        &c,
        Command::AnswerTimeOff {
            id: req,
            approve: true,
        },
    )
    .unwrap();
    assert!(
        w.staff_by_id(b)
            .unwrap()
            .leave_nights
            .iter()
            .any(|(n, _)| *n == 12)
    );
    // declines add up: three in two weeks → "looks tired"
    let mut tired = false;
    for _ in 0..3 {
        let id = w.alloc_id();
        let k = id_of(&w, "cook");
        w.app.swaps.push(SwapRequest {
            id,
            from: k,
            to: b,
            weekday: 3,
            status: RequestStatus::Pending,
        });
        w.apply(&c, Command::AnswerSwap { id, approve: false })
            .unwrap();
        tired |= w.drain_events().iter().any(
            |e| matches!(e, SimEvent::BotMessage { template, .. } if template == "looks_tired"),
        );
    }
    assert!(tired);
}

#[test]
fn early_leave_approved_sends_them_home_at_two() {
    let c = content();
    let mut w = crew_day(&c, 4);
    let k = id_of(&w, "cook");
    let rid = w.alloc_id();
    let night = w.night();
    w.sys.rozpis.early_leave.push(EarlyLeaveRequest {
        id: rid,
        staff: k,
        night,
        minute: 240,
        status: RequestStatus::Pending,
    });
    w.apply(
        &c,
        Command::AnswerEarlyLeave {
            id: rid,
            approve: true,
        },
    )
    .unwrap();
    // the cook works tonight
    let wd = w.weekday() as usize;
    w.app.published.get_mut(&k).unwrap()[wd] = Shift::Full;
    w.start_night(&c);
    assert_eq!(w.staff_by_id(k).unwrap().leave_at, 240);
}

// ---------------------------------------------------------------------------------------------
// The app as a tool
// ---------------------------------------------------------------------------------------------

#[test]
fn rituals_handover_notes_and_the_next_night() {
    let c = content();
    let mut w = night(&c, 3);
    no_director(&mut w);
    // the whole opening list → "The café is ready." (crew mood, coziness)
    let list = |k| -> Vec<String> {
        c.app
            .checklists
            .iter()
            .filter(|x| x.list == k)
            .map(|x| x.id.clone())
            .collect()
    };
    let coz = w.coziness().value;
    for id in list(crate::content::world::ChecklistKind::Opening) {
        app::tick_checklist(&mut w, &c, &id, None);
    }
    let ev = w.drain_events();
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::RitualComplete { .. }))
    );
    atmos::recompute(&mut w, &c);
    assert!(
        w.coziness().value >= coz + 1.0,
        "the ritual's coziness touch"
    );
    // closing: the whole list → tomorrow's opening is quicker; the handover note prefills
    for id in list(crate::content::world::ChecklistKind::Closing) {
        app::tick_checklist(&mut w, &c, &id, None);
    }
    // the note is picked after 05:00, when the app has made its own suggestions (which would
    // replace this hand-made list; how many fit depends on who is still waiting)
    while w.minute() <= 420 {
        w.tick(&c);
        bot::finish_cutscenes(&mut w, &c);
    }
    w.drain_events();
    w.sys.tool.suggestions = vec![HandoverSuggestion {
        id: "thank_crew".into(),
        args: Vec::new(),
        target: XTarget::default(),
    }];
    w.apply(
        &c,
        Command::PickHandover {
            picks: vec!["thank_crew".into()],
        },
    )
    .unwrap();
    to_day(&mut w, &c);
    bot::finish_cutscenes(&mut w, &c);
    w.start_night(&c);
    let ev = w.drain_events();
    assert!(ev.iter().any(|e| matches!(e,
        SimEvent::HandoverApplied { ids } if ids.contains(&"thank_crew".to_string()))));
    assert!(
        (w.sys.fx.checklist_speed[0] - 0.5).abs() < 0.01,
        "closed properly"
    );
}

#[test]
fn bot_buttons_act_in_the_world() {
    let c = content();
    let mut w = night(&c, 6);
    no_director(&mut w);
    let ctx = EffectCtx::default();
    // the weather alert's umbrella stand
    app::bot_with(
        &mut w,
        &c,
        "weather_alert",
        &ctx,
        vec![("time".into(), "01:00".into())],
    );
    let msg = w.app().inbox.last().unwrap().id;
    w.apply(
        &c,
        Command::BotAction {
            msg,
            action: "umbrellas".into(),
        },
    )
    .unwrap();
    assert!(w.active_moments().iter().any(|m| m.id == "m_umbrellas"));
    // take an item off tonight's menu
    app::bot_with(
        &mut w,
        &c,
        "low_stock",
        &ctx,
        vec![
            ("item".into(), "cappuccino".into()),
            ("n".into(), "3".into()),
        ],
    );
    let msg = w.app().inbox.last().unwrap().id;
    w.apply(
        &c,
        Command::BotAction {
            msg,
            action: "menu_off".into(),
        },
    )
    .unwrap();
    assert!(!w.menu_available(&c).contains(&"cappuccino".to_string()));
    // the drone needs a dock: the button refuses and stays open
    app::bot_with(
        &mut w,
        &c,
        "low_stock",
        &ctx,
        vec![("item".into(), "milk".into()), ("n".into(), "3".into())],
    );
    let msg = w.app().inbox.last().unwrap().id;
    assert!(
        w.apply(
            &c,
            Command::BotAction {
                msg,
                action: "drone".into()
            }
        )
        .is_err()
    );
    assert!(w.app().inbox.last().unwrap().answered.is_none());
    // the cat button eases the camera to the door
    app::bot(&mut w, &c, "cat_waiting", &ctx);
    let msg = w.app().inbox.last().unwrap().id;
    w.apply(
        &c,
        Command::BotAction {
            msg,
            action: "look".into(),
        },
    )
    .unwrap();
    assert!(
        w.drain_events()
            .iter()
            .any(|e| matches!(e, SimEvent::CameraHint { .. }))
    );
}

#[test]
fn cover_other_moves_a_late_helpers_duty_until_they_arrive() {
    let c = content();
    let mut w = World::new_sandbox(&c, SandboxSetup::default());
    bot::finish_cutscenes(&mut w, &c);
    let ids: Vec<u32> = w.candidates().iter().map(|x| x.id).take(1).collect();
    for id in ids {
        w.apply(&c, Command::Hire { candidate: id }).unwrap();
    }
    w.start_night(&c);
    w.skip_prep(&c);
    let helpers: Vec<u32> = w
        .staff()
        .iter()
        .filter(|s| !s.owner)
        .map(|s| s.id)
        .collect();
    let (late, other) = (helpers[0], helpers[1]);
    for i in 0..w.staff.len() {
        if w.staff[i].id == other {
            w.staff[i].duties.clear();
            w.staff[i].nights_worked = 5;
            w.staff[i].status = StaffStatus::OnDuty;
        }
        if w.staff[i].id == late {
            w.staff[i].duties = vec![crate::content::handson::Duty::Bar];
            w.staff[i].status = StaffStatus::Expected { minute: 30 };
        }
    }
    let ctx = EffectCtx {
        staff: Some(late),
        ..Default::default()
    };
    app::bot_with(
        &mut w,
        &c,
        "late_alert",
        &ctx,
        vec![("minutes".into(), "20".into())],
    );
    let msg = w.app().inbox.last().unwrap().id;
    w.apply(
        &c,
        Command::BotAction {
            msg,
            action: "ask_other".into(),
        },
    )
    .unwrap();
    assert!(
        w.staff_duties(other)
            .contains(&crate::content::handson::Duty::Bar)
    );
    ticks(&mut w, &c, 40 * TICKS_PER_MINUTE);
    assert!(
        !w.staff_duties(other)
            .contains(&crate::content::handson::Duty::Bar),
        "the cover ends when they arrive"
    );
}

#[test]
fn the_digest_has_insights_and_payroll_books_the_tip_split() {
    let c = content();
    let mut w = night(&c, 7);
    w.app.week_tips = 1000;
    to_day(&mut w, &c);
    let s = w.last_summary().unwrap();
    assert!(!s.insights.is_empty() && s.insights.len() <= 4);
    assert!(s.insights.iter().all(|i| i.key.starts_with("app.insight.")));
    let week = w
        .app()
        .payroll
        .last()
        .map(|p| p.week)
        .expect("week 1 payroll");
    let tips = w.app().payroll.last().unwrap().tips;
    assert!(tips >= 1000);
    let money = w.money();
    w.apply(
        &c,
        Command::SetTipSplit {
            split: TipSplit::Crew,
        },
    )
    .unwrap();
    w.apply(&c, Command::SendPayroll { week }).unwrap();
    assert_eq!(w.money(), money - tips, "all tips to the crew");
    assert!(w.payroll_sent(week));
    assert!(w.apply(&c, Command::SendPayroll { week }).is_err());
    assert_eq!(w.tip_split(&c), TipSplit::Crew, "remembered");
}

#[test]
fn announcements_act_on_the_person_named() {
    let c = content();
    let mut w = crew_day(&c, 4);
    let b = id_of(&w, "barista");
    let name = w.staff_by_id(b).unwrap().name.clone();
    let mood = w.staff_by_id(b).unwrap().mood;
    w.apply(
        &c,
        Command::PostAnnouncement {
            id: "thank_you".into(),
            args: vec![("name".into(), name)],
        },
    )
    .unwrap();
    assert!(w.staff_by_id(b).unwrap().mood >= (mood + 6.0).min(100.0) - 0.01);
}

// ---------------------------------------------------------------------------------------------
// Determinism & the 30-night run
// ---------------------------------------------------------------------------------------------

#[test]
fn same_seed_same_t5_saves() {
    let c = content();
    let run = || {
        let mut w = World::new_story_seeded(&c, Difficulty::Cozy, 4242);
        let mut b = bot::PlayerBot::new(bot::BotConfig::default());
        for _ in 0..5 {
            b.run_night(&mut w, &c);
        }
        b.day(&mut w, &c);
        w.save_to_string().unwrap()
    };
    let (a, b) = (run(), run());
    assert_eq!(a, b);
    let w = World::load_from_str(&a).unwrap();
    assert!(!w.journal().is_empty());
    assert_eq!(
        w.save_to_string().unwrap(),
        a,
        "a loaded save writes the same text"
    );
}

#[test]
fn thirty_cozy_nights_touch_every_system() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    let mut b = bot::PlayerBot::new(bot::BotConfig::default());
    let mut seen: std::collections::BTreeMap<&'static str, u32> = Default::default();
    let mut nights = 0;
    while !w.is_closed() && nights < 30 {
        for e in b.run_night(&mut w, &c) {
            let k = match e {
                SimEvent::MomentResolved { .. } => "moment",
                SimEvent::ExperimentDone { .. } => "experiment",
                SimEvent::MarginNote { .. } => "margin_note",
                SimEvent::ThankYouNote { .. } => "napkin",
                SimEvent::CatLevel { .. } => "cat",
                SimEvent::TipGoalReached { .. } => "tip_goal",
                SimEvent::CozinessLevel { .. } => "coziness",
                SimEvent::SeasonPhaseChanged { .. } => "season",
                SimEvent::ThemeChanged { .. } => "theme",
                SimEvent::HighlightCaptured { .. } => "highlight",
                SimEvent::RozpisScored { .. } => "rozpis",
                SimEvent::RequestCreated { .. } => "request",
                SimEvent::RequestAnswered { .. } => "answer",
                SimEvent::RitualComplete { .. } => "ritual",
                SimEvent::HandoverApplied { .. } => "handover",
                SimEvent::PayrollSent { .. } => "payroll",
                SimEvent::LatteUnlocked { .. } => "latte",
                _ => continue,
            };
            *seen.entry(k).or_default() += 1;
        }
        nights += 1;
    }
    for k in [
        "moment",
        "experiment",
        "margin_note",
        "napkin",
        "cat",
        "tip_goal",
        "coziness",
        "season",
        "highlight",
        "rozpis",
        "request",
        "answer",
        "ritual",
        "handover",
        "payroll",
        "latte",
    ] {
        assert!(seen.contains_key(k), "{k} never happened: {seen:?}");
    }
    let kept = w.journal().len() + usize::from(w.pending_polaroids().is_some());
    assert_eq!(kept, 30, "one polaroid every night");
    assert!(w.ending().is_some(), "the story ends");
}

// ---------------------------------------------------------------------------------------------
// Integration requests (T7)
// ---------------------------------------------------------------------------------------------

#[test]
fn the_night_one_tutorial_brings_mirek_early() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    bot::finish_cutscenes(&mut w, &c);
    w.start_night(&c);
    bot::finish_cutscenes(&mut w, &c);
    // the preparation half hour first: nobody comes in before the doors open
    let prep = ticks(&mut w, &c, PREP_TICKS);
    assert!(
        prep.iter()
            .any(|e| matches!(e, SimEvent::DoorsOpened { .. }))
    );
    assert!(!prep.iter().any(|e| matches!(
        e,
        SimEvent::RegularArrived { .. } | SimEvent::GuestArrived { .. }
    )));
    assert_eq!(w.clock_label(), "22:00");
    // Mirek still comes at 22:04
    let ev = ticks(&mut w, &c, 6 * TICKS_PER_MINUTE);
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::TutorialEffects { id } if id == "greet"))
    );
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::RegularArrived { regular, .. } if regular == "mirek"))
    );
    // the UI may show a step itself; it runs once
    w.apply(&c, Command::TutorialShown { id: "greet".into() })
        .unwrap();
    assert!(
        !w.drain_events()
            .iter()
            .any(|e| matches!(e, SimEvent::TutorialEffects { .. }))
    );
    // off in the settings: nothing scripted
    let mut w = World::new_story(&c, Difficulty::Cozy);
    w.apply(&c, Command::SetTutorial { on: false }).unwrap();
    bot::finish_cutscenes(&mut w, &c);
    w.start_night(&c);
    let ev = ticks(&mut w, &c, 6 * TICKS_PER_MINUTE);
    assert!(
        !ev.iter()
            .any(|e| matches!(e, SimEvent::TutorialEffects { .. }))
    );
}

#[test]
fn a_brew_starts_with_the_minigame_and_takes_its_quality_after() {
    let c = content();
    let mut w = night(&c, 2);
    no_director(&mut w);
    let pid =
        guests::spawn_party(&mut w, &c, "travelers", 1, None, 1.0, 1.0, &[], None, true).unwrap();
    let guest = w
        .parties
        .iter()
        .find(|p| p.id == pid)
        .unwrap()
        .leader()
        .unwrap();
    guests::settle_now(&mut w, pid);
    w.apply(&c, Command::PlayerTakeOrder { guest }).unwrap();
    let mut ok = false;
    for _ in 0..3000 {
        w.tick(&c);
        bot::finish_cutscenes(&mut w, &c);
        if w.party_of_guest(guest)
            .is_some_and(|p| p.wish_open && p.ordered_at.is_some())
        {
            ok = true;
            break;
        }
    }
    assert!(ok, "the order was taken");
    // Karolina never waited by an empty chair: she was at the counter or the table
    let recipe = w.best_matches(&c, guest).first().unwrap().0.clone();
    w.apply(
        &c,
        Command::PlayerBrewStart {
            guest,
            recipe: recipe.clone(),
            mode: BrewMode::ByHand,
        },
    )
    .unwrap();
    let mut waiting = false;
    for _ in 0..1500 {
        w.tick(&c);
        bot::finish_cutscenes(&mut w, &c);
        let ev = w.drain_events();
        assert!(
            !ev.iter()
                .any(|e| matches!(e, SimEvent::PlayerServed { .. }))
        );
        if ev
            .iter()
            .any(|e| matches!(e, SimEvent::BrewAwaitsQuality { .. }))
        {
            waiting = true;
            break;
        }
    }
    assert!(waiting, "she holds the cup until the mini-game reports");
    let ev = ticks(&mut w, &c, 50);
    assert!(
        !ev.iter()
            .any(|e| matches!(e, SimEvent::PlayerServed { .. }))
    );
    w.apply(
        &c,
        Command::PlayerBrewQuality {
            guest,
            quality: 0.93,
        },
    )
    .unwrap();
    let ev = ticks(&mut w, &c, 600);
    assert!(ev.iter().any(|e| matches!(e,
        SimEvent::PlayerServed { quality, .. } if (*quality - 0.93).abs() < 0.001)));
}

#[test]
fn micro_acts_report_when_done() {
    let c = content();
    let mut w = night(&c, 2);
    no_director(&mut w);
    let table = w
        .furniture()
        .iter()
        .find(|p| c.furniture_def(&p.def).is_some_and(|d| d.has_tag("table")))
        .unwrap()
        .uid;
    w.furniture
        .iter_mut()
        .find(|p| p.uid == table)
        .unwrap()
        .dishes = 2;
    w.apply(
        &c,
        Command::PlayerQueue {
            act: PlayerAct::ClearTable { table },
            front: false,
        },
    )
    .unwrap();
    w.dirt = 0.5;
    let tile = w.dirt_spots()[0];
    w.apply(
        &c,
        Command::PlayerQueue {
            act: PlayerAct::Mop { tile },
            front: false,
        },
    )
    .unwrap();
    let ev = ticks(&mut w, &c, 1500);
    assert!(
        ev.iter().any(
            |e| matches!(e, SimEvent::TableCleared { table: t, dishes: 2, .. } if *t == table)
        )
    );
    assert!(ev.iter().any(|e| matches!(e, SimEvent::Mopped { .. })));
}

#[test]
fn the_auto_fill_alone_plans_a_happy_week_for_the_story_crew() {
    // T9 balance: Bára, Zdeněk and Kuba's wishes fit together, so one click on Auto-fill
    // (and Publish) is enough for a happy week, with no hand edits.
    let c = content();
    for night in [4, 8] {
        let mut w = crew_day(&c, night);
        w.apply(&c, Command::AutoFillRozpis).unwrap();
        let r = w.autofill_report().clone();
        assert!(
            r.broken.is_empty() && r.kept == r.total,
            "night {night}: {r:?}"
        );
        for s in w.staff().iter().filter(|s| !s.owner) {
            for (p, st) in w.pref_status(&c, s.id) {
                assert_ne!(
                    st,
                    PrefStatus::Violated,
                    "night {night}: {} {:?}",
                    s.archetype,
                    p.kind
                );
            }
        }
        assert!(w.coverage(&c).iter().all(|n| n.on + 1 >= n.suggested));
        w.apply(&c, Command::PublishSchedule).unwrap();
        let rows = super::rozpis::effective(&w, &c, &w.app().published);
        assert!(super::rozpis::happy(&w, &c, &rows), "night {night}");
        assert!(w.systems().rozpis.happy_week.is_some(), "night {night}");
    }
}

#[test]
fn a_locked_moment_option_is_on_the_card_but_cannot_be_resolved() {
    let c = content();
    let mut w = night(&c, 4);
    no_director(&mut w);
    assert!(
        super::moments::spawn(&mut w, &c, "m_busker", XTarget::default(), true),
        "the busker appears"
    );
    // no stage in the café: "Ask them in" is greyed out with its reason, not on offer
    assert!(
        !w.moment_options(&c, "m_busker")
            .contains(&"ask_in".to_string())
    );
    let card = w.moment_card_options(&c, "m_busker");
    assert!(card.contains(&(
        "ask_in".to_string(),
        Some("story_ui.moment.needs_stage".to_string())
    )));
    assert!(card.contains(&("tea".to_string(), None)));
    let r = w.apply(
        &c,
        Command::ResolveMoment {
            id: "m_busker".into(),
            option: "ask_in".into(),
        },
    );
    assert!(r.is_err(), "a locked option can't be resolved");
}
