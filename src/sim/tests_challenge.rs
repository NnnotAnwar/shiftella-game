//! The sandbox «Challenge» mode (`sim::challenge`): Endless stays as it was; Challenge raises
//! rent / supplies / wages each season, opens rivals who cut the walk-ins (the café's
//! strengths win some back), leans the storyteller harder, and two missed season goals (or
//! deep debt) close the café with a final report. The balance curve is an ignored sweep:
//!
//! ```text
//! SBX_SEEDS=1-12 SBX_DIFF=cozy cargo test --release --lib sim::tests_challenge::curve \
//!   -- --ignored --nocapture
//! ```

use super::challenge::{self, ChallengeEnd, Strength};
use super::tests_sandbox::{bot_for, diff_of, seeds_from_env, setup};
use super::*;
use crate::content::Content;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

fn content() -> Content {
    Content::load(DATA).expect("content")
}

fn challenge_world(c: &Content, seed: u64, diff: Difficulty) -> World {
    World::new_sandbox(c, setup(c, seed, diff, "challenge"))
}

/// Per season of one run: (season, goal, earned, met, money_end, rep_end, rent, rivals,
/// avg loss %, avg strengths, bad events).
type SeasonRow = (u32, i64, i64, bool, i64, f32, i64, u32, f32, f32, u32);

/// Plays a Challenge run with the player-bot until the café closes (or `max` nights).
fn play(c: &Content, seed: u64, diff: Difficulty, max: u32) -> (World, Vec<SeasonRow>) {
    let mut w = challenge_world(c, seed, diff);
    let mut b = bot_for(seed, diff);
    let mut rows = Vec::new();
    let (mut loss, mut st, mut bad, mut n) = (0.0f32, 0.0f32, 0u32, 0u32);
    while !w.is_closed() && w.night() <= max {
        b.run_night(&mut w, c);
        let s = w.last_summary().cloned().unwrap_or_default();
        loss += w.challenge.loss;
        st += w.challenge.strengths.len() as f32;
        bad += s.bad_events;
        n += 1;
        if let Some(h) = w.challenge.history.last()
            && h.season as usize > rows.len()
        {
            rows.push((
                h.season,
                h.goal,
                h.earned,
                h.met,
                h.money_end,
                h.reputation_end,
                h.rent,
                h.rivals,
                loss / n.max(1) as f32 * 100.0,
                st / n.max(1) as f32,
                bad,
            ));
            (loss, st, bad, n) = (0.0, 0.0, 0, 0);
        }
    }
    (w, rows)
}

/// The balance curve: per seed (nights, seasons survived, why it closed) and the average per
/// season over the seeds that reached it.
#[test]
#[ignore = "QA sweep: minutes in release"]
fn curve() {
    let c = content();
    let diff = diff_of(&std::env::var("SBX_DIFF").unwrap_or_default());
    let max: u32 = std::env::var("SBX_NIGHTS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(450);
    let mut all: Vec<Vec<SeasonRow>> = Vec::new();
    let mut nights = Vec::new();
    println!("seed,size,nights,survived,reason");
    for seed in seeds_from_env() {
        let (w, rows) = play(&c, seed, diff, max);
        let r = w.challenge_report().unwrap();
        println!(
            "{seed},{},{},{},{:?}",
            w.town.size, r.nights, r.seasons_survived, r.reason
        );
        for x in &rows {
            println!("  {x:?}");
        }
        nights.push(r.nights);
        all.push(rows);
    }
    let avg = nights.iter().sum::<u32>() as f32 / nights.len().max(1) as f32;
    println!(
        "avg nights {avg:.0} (≈ {:.1} seasons) min {} max {}",
        avg / 30.0,
        nights.iter().min().unwrap_or(&0),
        nights.iter().max().unwrap_or(&0)
    );
    println!("season,runs,goal,earned,met%,money_end,rep_end,rent,rivals,loss%,strengths,bad");
    for s in 1..=15u32 {
        let rs: Vec<&SeasonRow> = all.iter().filter_map(|r| r.get(s as usize - 1)).collect();
        if rs.is_empty() {
            break;
        }
        let k = rs.len() as f32;
        let f = |g: &dyn Fn(&SeasonRow) -> f32| rs.iter().map(|r| g(r)).sum::<f32>() / k;
        println!(
            "{s},{},{:.0},{:.0},{:.0},{:.0},{:.1},{:.0},{:.1},{:.1},{:.1},{:.1}",
            rs.len(),
            f(&|r| r.1 as f32),
            f(&|r| r.2 as f32),
            f(&|r| if r.3 { 100.0 } else { 0.0 }),
            f(&|r| r.4 as f32),
            f(&|r| r.5),
            f(&|r| r.6 as f32),
            f(&|r| r.7 as f32),
            f(&|r| r.8),
            f(&|r| r.9),
            f(&|r| r.10 as f32),
        );
    }
}

/// A long Classic run ends by itself (strikes or debt) and leaves a report.
#[test]
#[ignore = "long: a full Challenge run with the player-bot (~30 s in release)"]
fn a_classic_challenge_run_ends() {
    let c = content();
    let (w, rows) = play(&c, 7, Difficulty::Classic, 400);
    assert!(w.is_closed(), "still open on night {}", w.night());
    let r = w.challenge_report().unwrap();
    assert!(
        r.reason.is_some() && r.nights < 400 && !rows.is_empty(),
        "{r:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// Endless stays as it was
// ---------------------------------------------------------------------------------------------

/// Endless (no option, or the option off) never starts a season: no goal, no rivals, no lines,
/// the difficulty's capped rent growth, the identity everywhere — and the same café either way.
#[test]
fn endless_never_starts_a_challenge() {
    let c = content();
    let mut a = World::new_sandbox(&c, setup(&c, 5, Difficulty::Cozy, "default"));
    let mut off = setup(&c, 5, Difficulty::Cozy, "default");
    off.options.insert(challenge::OPTION.into(), false);
    let mut b = World::new_sandbox(&c, off);
    for w in [&mut a, &mut b] {
        assert!(!challenge::enabled(w) && !challenge::active(w));
        assert!(w.challenge(&c).is_none() && w.challenge_report().is_none());
        assert_eq!(challenge::demand_mul(w), 1.0);
        assert_eq!(challenge::wear_mul(w, &c), 1.0);
        assert_eq!(challenge::supply_index(w), None);
        assert_eq!(challenge::wage_index(w), None);
        let s = challenge::storyteller(w, &c);
        assert_eq!(
            (s.rate, s.bad, s.good_share, s.tension),
            (1.0, 1.0, 1.0, 0.0)
        );
        assert_eq!((s.bad_in_row, s.breath), (0, 1.0));
        assert!(
            !w.app
                .inbox
                .iter()
                .any(|m| m.template.starts_with("challenge"))
        );
    }
    // the same café, the same night (the option's presence alone changes nothing)
    a.run_night_headless(&c);
    b.run_night_headless(&c);
    assert_eq!(a.money(), b.money());
    assert_eq!(a.rng, b.rng);
    assert!(a.last_summary().unwrap().challenge.is_empty());
    // rent: the difficulty's +5 % every 10 nights, capped as before
    let rent = a.econ.rent;
    a.night = 11;
    a.begin_day(&c);
    let grown = ((rent as f32 * 1.05 / 10.0).round() * 10.0) as i64;
    assert_eq!(a.econ.rent, grown);
    a.night = 31;
    a.begin_day(&c);
    assert_eq!(a.challenge, Default::default());
}

// ---------------------------------------------------------------------------------------------
// Money
// ---------------------------------------------------------------------------------------------

/// Season 1 starts with the goal and a bot message; each new season raises rent, supplies and
/// wages (compounding, no `rent_growth_max` cap) and the goal.
#[test]
fn each_season_raises_rent_supplies_wages_and_the_goal() {
    let c = content();
    let d = &c.sandbox.challenge;
    let mut w = challenge_world(&c, 3, Difficulty::Cozy);
    assert!(challenge::active(&w));
    let v = w.challenge(&c).unwrap();
    assert_eq!(
        (v.season, v.strikes, v.nights_left),
        (1, 0, d.season_nights)
    );
    assert_eq!(v.goal, challenge::goal_for(&w, &c, 1));
    assert!(v.goal > 0 && v.rivals.is_empty());
    assert!(w.app.inbox.iter().any(|m| m.template == "challenge_start"));
    // night 11 and 21: no 10-night growth in Challenge
    let rent1 = w.econ.rent;
    for n in [11, 21] {
        w.night = n;
        w.begin_day(&c);
        assert_eq!(w.econ.rent, rent1, "night {n}");
    }
    let mut rent = rent1;
    let mut goal = w.challenge.goal;
    for s in 2..=8u32 {
        w.night = (s - 1) * d.season_nights + 1;
        w.begin_day(&c);
        assert_eq!(w.challenge.season, s);
        let want = ((rent as f32 * (1.0 + d.rent_inflation / 100.0) / 10.0).round() * 10.0) as i64;
        assert_eq!(w.econ.rent, want, "season {s}");
        rent = want;
        assert!(w.challenge.goal > goal, "the goal grows");
        goal = w.challenge.goal;
        let k = (s - 1) as i32;
        let idx = |p: f32| (1.0 + p / 100.0).powi(k);
        assert!((w.challenge.supply_index - idx(d.supply_inflation)).abs() < 1e-3);
        assert!((w.challenge.wage_index - idx(d.wage_inflation)).abs() < 1e-3);
        assert!(w.app.inbox.iter().any(|m| m.template == "challenge_season"));
    }
    // uncapped: well past the Endless cap
    let mul = c.difficulty("cozy").unwrap().rent_growth_max.unwrap();
    assert!(
        w.econ.rent > w.sandbox_rent_cap(&c, mul),
        "rent {}",
        w.econ.rent
    );

    // supplies cost more: the restock pays `index` × the stock it tops up
    w.econ.supplies = 0;
    w.econ.auto_restock = Some(3000);
    let money = w.econ.money;
    super::economy::restock(&mut w, &c);
    let idx = w.challenge.supply_index;
    assert_eq!(w.econ.supplies, 3000);
    assert_eq!(money - w.econ.money, (3000.0 * idx).round() as i64);
    // …and a manual buy gets less stock for the money
    w.econ.money = 10_000;
    let before = w.econ.supplies;
    super::economy::buy_supplies(&mut w, 1000).unwrap();
    assert_eq!(w.econ.supplies - before, (1000.0 / idx).floor() as i64);

    // wages: the bill × the wage index
    let mut e = World::new_sandbox(&c, setup(&c, 3, Difficulty::Cozy, "default"));
    let mut x = challenge_world(&c, 3, Difficulty::Cozy);
    x.challenge.wage_index = 1.5;
    for w in [&mut e, &mut x] {
        for s in w.staff.iter_mut().filter(|s| !s.owner) {
            s.minutes_worked = 8 * 60;
        }
        super::economy::on_night_end(w, &c);
    }
    assert!(e.tonight.wages > 0);
    assert_eq!(
        x.tonight.wages,
        (e.tonight.wages as f32 * 1.5).round() as i64
    );
}

// ---------------------------------------------------------------------------------------------
// Rivals
// ---------------------------------------------------------------------------------------------

/// From season 2 a named rival opens each season (up to the cap); each takes a share of the
/// walk-ins and the café's strengths win part of it back.
#[test]
fn rivals_open_cut_the_walk_ins_and_strengths_win_them_back() {
    let c = content();
    let d = &c.sandbox.challenge;
    // the share: per rival, cut by each strength
    assert!((challenge::rival_loss(d, 1, 0) - d.competitor_share).abs() < 1e-6);
    assert!((challenge::rival_loss(d, 3, 0) - 3.0 * d.competitor_share).abs() < 1e-6);
    let all = challenge::rival_loss(d, 3, 4);
    assert!((all - 3.0 * d.competitor_share * (1.0 - 4.0 * d.counter_cut)).abs() < 1e-6);
    assert!(challenge::rival_loss(d, 3, 2) < challenge::rival_loss(d, 3, 1));

    let mut w = challenge_world(&c, 9, Difficulty::Cozy);
    for s in 2..=6u32 {
        w.night = (s - 1) * d.season_nights + 1;
        w.begin_day(&c);
    }
    assert_eq!(w.challenge.rivals.len() as u32, d.competitors_max);
    let names: std::collections::BTreeSet<_> = w.challenge.rivals.iter().collect();
    assert_eq!(
        names.len(),
        w.challenge.rivals.len(),
        "three different rivals"
    );
    let town = d
        .competitor_names
        .get(&w.town.name)
        .or(d.competitor_names.get("*"));
    assert!(w.challenge.rivals.iter().all(|r| town.unwrap().contains(r)));
    assert!(w.app.inbox.iter().any(|m| m.template == "challenge_rival"));
    // the same seed opens the same rivals (derived RNG, the world's untouched)
    let mut w2 = challenge_world(&c, 9, Difficulty::Cozy);
    for s in 2..=6u32 {
        w2.night = (s - 1) * d.season_nights + 1;
        w2.begin_day(&c);
    }
    assert_eq!(w.challenge.rivals, w2.challenge.rivals);
    assert_eq!(w.rng, w2.rng);

    // strengths: four stars, regulars (the counters the morning line reports)
    w.econ.reputation = 50.0;
    w.story.trust.clear();
    assert!(!challenge::strengths(&w, &c).contains(&Strength::Reputation));
    w.econ.reputation = d.counter_reputation + 1.0;
    for r in ["a", "b", "c"] {
        w.story
            .trust
            .insert(r.into(), d.counter_regular_trust + 1.0);
    }
    let st = challenge::strengths(&w, &c);
    assert!(st.contains(&Strength::Reputation) && st.contains(&Strength::Regulars));
    challenge::on_night_start(&mut w, &c);
    let with = w.challenge.loss;
    w.econ.reputation = 10.0;
    w.story.trust.clear();
    challenge::on_night_start(&mut w, &c);
    assert!(w.challenge.loss > with, "{} vs {with}", w.challenge.loss);
    assert!((challenge::demand_mul(&w) - (1.0 - w.challenge.loss)).abs() < 1e-6);

    // fewer walk-ins tonight than the same café without rivals, and the morning line says so
    let mut none = challenge_world(&c, 9, Difficulty::Cozy);
    let mut three = challenge_world(&c, 9, Difficulty::Cozy);
    three.challenge.rivals = vec!["Café Kompas".into(), "Bean Station".into(), "X".into()];
    let (mut a, mut b) = (0, 0);
    for _ in 0..2 {
        none.run_night_headless(&c);
        three.run_night_headless(&c);
        a += none.last_summary().unwrap().guests_arrived;
        b += three.last_summary().unwrap().guests_arrived;
    }
    assert!(b < a, "with rivals {b}, without {a}");
    let lines = &three.last_summary().unwrap().challenge;
    assert!(
        lines.iter().any(|l| l.key == "challenge.line.rivals"),
        "{lines:?}"
    );
    assert!(
        lines.iter().any(|l| l.key == "challenge.line.goal"),
        "{lines:?}"
    );
    let digest = super::app::digest_lines(three.last_summary().unwrap());
    assert!(digest.iter().any(|(k, _)| k == "challenge.line.rivals"));
}

// ---------------------------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------------------------

/// The storyteller and the wear lean harder each season (season 1 is the identity).
#[test]
fn the_storyteller_and_wear_scale_with_the_season() {
    let c = content();
    let d = &c.sandbox.challenge;
    let mut w = challenge_world(&c, 4, Difficulty::Cozy);
    let s1 = challenge::storyteller(&w, &c);
    assert_eq!((s1.rate, s1.bad, s1.tension), (1.0, 1.0, 0.0));
    assert_eq!(challenge::wear_mul(&w, &c), 1.0);
    let mut last = s1;
    let mut wear = 1.0;
    for s in 2..=5u32 {
        w.night = (s - 1) * d.season_nights + 1;
        w.begin_day(&c);
        let x = challenge::storyteller(&w, &c);
        assert!(x.rate > last.rate && x.bad > last.bad && x.tension > last.tension);
        assert!(x.good_share < last.good_share && x.breath < last.breath);
        assert!(x.bad_in_row >= last.bad_in_row);
        assert!(x.also.iter().any(|e| e == "health_inspection"));
        let wm = challenge::wear_mul(&w, &c);
        assert!(wm > wear);
        (last, wear) = (x, wm);
    }
    let k = 4.0; // season 5
    assert!((last.rate - (1.0 + d.event_rate / 100.0 * k)).abs() < 1e-5);
    assert!((last.bad - (1.0 + d.bad_weight / 100.0 * k)).abs() < 1e-5);
    assert_eq!(last.bad_in_row, 4 / d.bad_in_row_every);
    // more Bad events over the same nights than the same café in season 1
    let mut calm = challenge_world(&c, 4, Difficulty::Cozy);
    let mut late = challenge_world(&c, 4, Difficulty::Cozy);
    late.challenge.season = 6;
    let (mut a, mut b) = (0, 0);
    for _ in 0..4 {
        calm.run_night_headless(&c);
        late.run_night_headless(&c);
        a += calm.last_summary().unwrap().bad_events;
        b += late.last_summary().unwrap().bad_events;
    }
    assert!(b > a, "season 6: {b} bad events, season 1: {a}");
}

// ---------------------------------------------------------------------------------------------
// Strikes → closing → the final page
// ---------------------------------------------------------------------------------------------

/// Ends a season's last night by hand: tonight earned `earned`.
fn end_season(w: &mut World, c: &Content, night: u32, earned: i64) {
    w.night = night;
    w.tonight = NightSummary {
        night,
        revenue: earned,
        guests_served: 40,
        ..Default::default()
    };
    challenge::on_night_end(w, c);
}

#[test]
fn two_missed_goals_in_a_row_close_the_cafe_with_a_report() {
    let c = content();
    let d = &c.sandbox.challenge;
    let n = d.season_nights;
    let mut w = challenge_world(&c, 11, Difficulty::Cozy);
    // season 1: met (strikes stay 0)
    let goal = w.challenge.goal;
    end_season(&mut w, &c, n, goal + 1);
    assert_eq!((w.challenge.strikes, w.challenge.history.len()), (0, 1));
    assert!(w.challenge.history[0].met);
    assert!(
        w.tonight
            .challenge
            .iter()
            .any(|l| l.key == "challenge.line.goal_met")
    );
    // season 2: missed → strike 1, still open
    w.night = n + 1;
    w.begin_day(&c);
    end_season(&mut w, &c, 2 * n, 10);
    assert_eq!(w.challenge.strikes, 1);
    assert!(!challenge::closes(&w));
    assert!(
        w.app
            .inbox
            .iter()
            .any(|m| m.template == "challenge_goal_missed")
    );
    // season 3: met again → the strikes reset
    w.night = 2 * n + 1;
    w.begin_day(&c);
    let goal = w.challenge.goal;
    end_season(&mut w, &c, 3 * n, goal);
    assert_eq!(w.challenge.strikes, 0);
    // seasons 4 and 5 missed → closed
    for s in [4, 5] {
        w.night = (s - 1) * n + 1;
        w.begin_day(&c);
        end_season(&mut w, &c, s * n, 0);
    }
    assert_eq!(w.challenge.strikes, 2);
    assert_eq!(w.challenge.ended, Some(ChallengeEnd::Strikes));
    assert!(super::story::on_night_end(&mut w, &c));
    assert!(w.is_closed());
    assert!(
        w.drain_events()
            .iter()
            .any(|e| matches!(e, SimEvent::CafeClosed))
    );
    let r = w.challenge_report().unwrap();
    assert_eq!(
        r.seasons_survived, 4,
        "five seasons played, closed after the fifth"
    );
    assert_eq!(r.reason, Some(ChallengeEnd::Strikes));
    assert_eq!(r.guests, 5 * 40);
    assert!(r.best_night > 0 && r.earned >= r.best_night);
    // a closed run doesn't start another season
    w.night = 5 * n + 1;
    w.begin_day(&c);
    assert_eq!(w.challenge.season, 5);
}

/// Deep in the red for `debt_nights` nights closes the café too (Cozy with debt allowed).
#[test]
fn too_deep_in_debt_closes_the_cafe() {
    let c = content();
    let d = &c.sandbox.challenge;
    let mut w = challenge_world(&c, 12, Difficulty::Cozy);
    assert_eq!(w.town.options.get("allow_debt"), Some(&true));
    let limit = (d.debt_limit_rents * w.econ.rent as f32).round() as i64;
    for i in 1..=d.debt_nights {
        assert!(!challenge::closes(&w), "night {i}");
        w.econ.money = -limit - 1;
        end_season(&mut w, &c, i + 1, 0);
        assert_eq!(w.challenge.debt_nights, i);
        assert!(
            w.tonight
                .challenge
                .iter()
                .any(|l| l.key == "challenge.line.debt")
        );
    }
    assert_eq!(w.challenge.ended, Some(ChallengeEnd::Debt));
    assert!(w.app.inbox.iter().any(|m| m.template == "challenge_debt"));
    // a night back above the limit resets the count
    let mut x = challenge_world(&c, 12, Difficulty::Cozy);
    x.econ.money = -limit - 1;
    end_season(&mut x, &c, 2, 0);
    x.econ.money = 0;
    end_season(&mut x, &c, 3, 0);
    assert_eq!(x.challenge.debt_nights, 0);
    assert_eq!(x.challenge_report().unwrap().seasons_survived, 0);
}

/// The record per seed (settings): the best seasons survived, never lowered.
#[test]
fn the_seed_record_keeps_the_best() {
    let mut s = crate::settings::Settings::default();
    assert_eq!(s.record_challenge(42, 3), None);
    assert_eq!(s.record_challenge(42, 2), Some(3));
    assert_eq!(s.record_challenge(42, 5), Some(3));
    assert_eq!(s.challenge_records.get("42"), Some(&5));
    let back = crate::settings::Settings::from_ron(&s.to_ron()).unwrap();
    assert_eq!(back.challenge_records, s.challenge_records);
}

// ---------------------------------------------------------------------------------------------
// Saves and text
// ---------------------------------------------------------------------------------------------

#[test]
fn the_mode_and_the_season_survive_save_and_load() {
    let c = content();
    let d = &c.sandbox.challenge;
    let mut w = challenge_world(&c, 8, Difficulty::Classic);
    w.night = d.season_nights + 1;
    w.begin_day(&c);
    w.challenge.strikes = 1;
    let text = w.save_to_string().unwrap();
    let back = World::load_from_str(&text).unwrap();
    assert_eq!(back.town().options.get(challenge::OPTION), Some(&true));
    assert_eq!(back.challenge, w.challenge);
    assert_eq!(back.challenge(&c), w.challenge(&c));
    // an older save (no `challenge` state at all) loads as Endless
    let e = World::new_sandbox(&c, setup(&c, 8, Difficulty::Cozy, "default"));
    let back = World::load_from_str(&e.save_to_string().unwrap()).unwrap();
    assert_eq!(back.challenge, Default::default());
}

/// Every Challenge text exists in all four languages.
#[test]
fn challenge_texts_exist_in_four_languages() {
    let root = env!("CARGO_MANIFEST_DIR");
    let i = crate::settings::load_i18n(&format!("{root}/assets/i18n")).unwrap();
    let mut keys: Vec<String> = [
        "ui.sandbox.mode",
        "ui.sandbox.mode.endless",
        "ui.sandbox.mode.challenge",
        "ui.sandbox.mode.endless_desc",
        "ui.sandbox.mode.challenge_desc",
        "challenge.line.rival",
        "challenge.line.rivals",
        "challenge.line.strengths",
        "challenge.line.goal",
        "challenge.line.goal_met",
        "challenge.line.goal_missed",
        "challenge.line.debt",
        "challenge.log.season",
        "challenge.log.rival",
        "challenge.summary.title",
        "challenge.plan.goal",
        "challenge.plan.left",
        "challenge.plan.rivals",
        "challenge.final.kicker",
        "challenge.final.seasons",
        "challenge.final.nights",
        "challenge.final.best_night",
        "challenge.final.guests",
        "challenge.final.earned",
        "challenge.final.record",
        "challenge.final.new_record",
        "challenge.final.back",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    for e in [ChallengeEnd::Strikes, ChallengeEnd::Debt] {
        keys.push(format!("challenge.final.reason.{}", e.id()));
    }
    for t in [
        "challenge_start",
        "challenge_season",
        "challenge_rival",
        "challenge_goal_met",
        "challenge_goal_missed",
        "challenge_debt",
    ] {
        keys.push(format!("app.bot.{t}"));
    }
    for k in &keys {
        for lang in crate::i18n::Lang::ALL {
            assert!(i.has(lang, k), "{k} missing in {lang:?}");
        }
    }
}
