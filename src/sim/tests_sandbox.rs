//! Long sandbox runs (QA): the player-bot plays a sandbox café for many nights and every
//! night is checked for drift the 30-night story never reaches (money, rent, crew, content
//! pools, save size, the calendar). The default tests are short; the sweeps are `#[ignore]`d:
//!
//! ```text
//! SBX_NIGHTS=100 SBX_SEEDS=1-30 SBX_DIFF=cozy SBX_VARIANT=default \
//!   cargo test --release --lib sim::tests_sandbox::sweep -- --ignored --nocapture > sweep.csv
//! ```

use super::*;
use crate::content::Content;
use std::collections::BTreeMap;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

fn content() -> Content {
    Content::load(DATA).expect("content")
}

fn ron_len<T: serde::Serialize>(v: &T) -> usize {
    ron::to_string(v).map(|s| s.len()).unwrap_or(0)
}

fn setup(c: &Content, seed: u64, diff: Difficulty, variant: &str) -> SandboxSetup {
    let places = &c.sandbox.town_names.places;
    let mut s = SandboxSetup {
        seed,
        difficulty: diff,
        town_name: Some(places[seed as usize % places.len()].clone()),
        ..Default::default()
    };
    match variant {
        "poor" => s.start_money = Some(10_000),
        "rich" => s.start_money = Some(50_000),
        "nodebt" => {
            s.start_money = Some(10_000);
            s.options.insert("allow_debt".into(), false);
        }
        "noinst" => s.institutions = Some(vec![]),
        "allinst" => {
            s.institutions = Some(c.sandbox.institutions.iter().map(|i| i.id.clone()).collect())
        }
        "chaotic" | "quiet" | "stormy" => s.character = Some(variant.into()),
        "norain" => {
            s.options.insert("rain_every_night".into(), false);
        }
        _ => {}
    }
    s
}

/// One line per night (CSV) with everything worth watching over a long game.
fn row(w: &World, c: &Content, ev: &[SimEvent], tick_us: f64, head: &str) -> String {
    let s = w.last_summary().cloned().unwrap_or_default();
    let helpers: Vec<&Staff> = w.staff.iter().filter(|s| !s.owner).collect();
    let n = helpers.len().max(1) as f32;
    let min_mood = helpers.iter().map(|s| s.mood).fold(100.0f32, f32::min);
    let avg_mood = helpers.iter().map(|s| s.mood).sum::<f32>() / n;
    let min_en = helpers.iter().map(|s| s.energy).fold(100.0f32, f32::min);
    let avg_en = helpers.iter().map(|s| s.energy).sum::<f32>() / n;
    let count = |f: &dyn Fn(&SimEvent) -> bool| ev.iter().filter(|e| f(e)).count();
    let walked = count(&|e| matches!(e, SimEvent::StaffWalkedOut { .. }));
    let hired = count(&|e| matches!(e, SimEvent::StaffHired { .. }));
    let fired = count(&|e| matches!(e, SimEvent::StaffFired { .. }));
    let started = count(&|e| matches!(e, SimEvent::EventStarted { .. }));
    let wear: Vec<f32> = w.furniture.iter().map(|f| f.wear).collect();
    let max_wear = wear.iter().copied().fold(0.0f32, f32::max);
    let avg_wear = wear.iter().sum::<f32>() / wear.len().max(1) as f32;
    let save = w.save_to_string().unwrap_or_default();
    let nan = save.contains("NaN") || save.contains("inf");
    let phase = super::pacing::season_phase(w, c).unwrap_or("-");
    format!(
        "{head},{},{},{},{},{:.1},{:.1},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.0},{:.0},{:.0},{:.0},{},{},{},{},{},{:.2},{:.2},{:.2},{},{},{:?},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.1},{}",
        s.night,
        w.money(),
        w.econ.debt,
        w.econ.rent,
        w.reputation(),
        w.sys.atmos.coziness.value,
        s.guests_arrived,
        s.guests_served,
        s.guests_left_unhappy,
        s.guests_forgiven,
        s.guests_turned_away,
        s.max_queue,
        s.karolina_idle_minutes,
        s.revenue,
        s.tips,
        s.wages,
        s.supplies,
        w.econ.supplies,
        helpers.len(),
        s.exhausted_minutes,
        min_mood,
        avg_mood,
        min_en,
        avg_en,
        walked,
        hired,
        fired,
        s.stuck_resets,
        started,
        max_wear,
        avg_wear,
        w.dirt,
        phase,
        w.town.season,
        w.weather,
        u8::from(w.stories.storm.nights.last() == Some(&s.night)),
        w.stories.told.len(),
        w.build.attic_offer.len(),
        w.build.attic_owned.len(),
        u8::from(w.build.back_room.is_some()),
        u8::from(w.sig.drink.is_some()),
        w.sig.photos.len(),
        w.sys.journal.entries.len(),
        w.log.len(),
        w.summaries.len(),
        save.len(),
        ron_len(&w.sys),
        ron_len(&w.hands),
        ron_len(&w.build),
        ron_len(&w.stories),
        ron_len(&w.story),
        ron_len(&w.narrator),
        ron_len(&w.app),
        ron_len(&w.staff),
        ron_len(&w.walk),
        ron_len(&w.candidates),
        tick_us,
        u8::from(nan),
    )
}

const HEADER: &str = "seed,diff,variant,night,money,debt,rent,rep,cozy,arrived,served,left,forgiven,turned,maxq,idle,revenue,tips,wages,sup_cost,sup_stock,helpers,exh_min,min_mood,avg_mood,min_en,avg_en,walked,hired,fired,stuck,events,max_wear,avg_wear,dirt,phase,season,weather,storm,told,attic_offer,attic_owned,backroom,sig,photos,journal,log,summaries,save,b_sys,b_hands,b_build,b_stories,b_story,b_narr,b_app,b_staff,b_walk,b_cand,tick_us,nan";

fn diff_of(s: &str) -> Difficulty {
    match s {
        "classic" => Difficulty::Classic,
        "relaxed" | "carefree" => Difficulty::Relaxed,
        _ => Difficulty::Cozy,
    }
}

fn bot_for(seed: u64, diff: Difficulty) -> bot::PlayerBot {
    bot::PlayerBot::new(bot::BotConfig {
        seed,
        // SBX_NO_DELEGATE=1: a player who never touches the duties (hire-card duty only)
        delegate: std::env::var("SBX_NO_DELEGATE").is_err(),
        hire_nights: if diff == Difficulty::Classic {
            vec![3, 8, 15]
        } else {
            vec![3, 5, 9]
        },
        ..Default::default()
    })
}

/// Plays `nights` and prints one CSV row a night.
fn run(
    c: &Content,
    seed: u64,
    diff: Difficulty,
    variant: &str,
    nights: u32,
    csv: bool,
) -> (World, bot::PlayerBot) {
    let mut w = World::new_sandbox(c, setup(c, seed, diff, variant));
    let mut b = bot_for(seed, diff);
    let head = format!("{seed},{},{variant}", diff.id());
    for _ in 0..nights {
        if w.is_closed() {
            if csv {
                println!("{head},CLOSED at night {}", w.night());
            }
            break;
        }
        let t0 = std::time::Instant::now();
        let ev = b.run_night(&mut w, c);
        let ticks = b.nights.last().map(|n| n.ticks).unwrap_or(1).max(1);
        let us = t0.elapsed().as_secs_f64() * 1e6 / ticks as f64;
        if csv {
            println!("{}", row(&w, c, &ev, us, &head));
        }
    }
    (w, b)
}

fn seeds_from_env() -> Vec<u64> {
    let s = std::env::var("SBX_SEEDS").unwrap_or_else(|_| "1-30".into());
    match s.split_once('-') {
        Some((a, b)) => (a.parse().unwrap()..=b.parse().unwrap()).collect(),
        None => s.split(',').filter_map(|x| x.parse().ok()).collect(),
    }
}

#[test]
#[ignore = "QA sweep: prints CSV, minutes in release"]
fn sweep() {
    let c = content();
    let nights: u32 = std::env::var("SBX_NIGHTS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);
    let diff = diff_of(&std::env::var("SBX_DIFF").unwrap_or_default());
    let variant = std::env::var("SBX_VARIANT").unwrap_or_else(|_| "default".into());
    println!("{HEADER}");
    for seed in seeds_from_env() {
        run(&c, seed, diff, &variant, nights, true);
    }
}

#[test]
#[ignore = "QA: writes the save after SBX_NIGHTS nights to SBX_DUMP"]
fn dump() {
    let c = content();
    let nights: u32 = std::env::var("SBX_NIGHTS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);
    let diff = diff_of(&std::env::var("SBX_DIFF").unwrap_or_default());
    let variant = std::env::var("SBX_VARIANT").unwrap_or_else(|_| "default".into());
    let seed = seeds_from_env()[0];
    let (w, _) = run(&c, seed, diff, &variant, nights, false);
    let path = std::env::var("SBX_DUMP").unwrap_or_else(|_| "sbx_dump.ron".into());
    std::fs::write(path, w.save_to_string().unwrap()).unwrap();
}

#[test]
#[ignore = "QA: traces night SBX_NIGHTS of seed SBX_SEEDS every SBX_EVERY minutes"]
fn trace() {
    let c = content();
    let target: u32 = std::env::var("SBX_NIGHTS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);
    let every: u32 = std::env::var("SBX_EVERY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20);
    let diff = diff_of(&std::env::var("SBX_DIFF").unwrap_or_default());
    let variant = std::env::var("SBX_VARIANT").unwrap_or_else(|_| "default".into());
    let seed = seeds_from_env()[0];
    let (mut w, mut b) = run(&c, seed, diff, &variant, target - 1, false);
    b.day(&mut w, &c);
    b.finish_scenes(&mut w, &c);
    w.drain_events();
    w.start_night(&c);
    println!("night {} weather {:?} mods {:?}", w.night(), w.weather, w.mods);
    let mut last = u32::MAX;
    while w.phase() == Phase::Night && !w.is_closed() {
        b.finish_scenes(&mut w, &c);
        b.step(&mut w, &c);
        w.tick(&c);
        for e in w.drain_events() {
            if matches!(
                e,
                SimEvent::EventStarted { .. }
                    | SimEvent::Blackout { .. }
                    | SimEvent::ChoiceNeeded { .. }
                    | SimEvent::PlayCutscene(_)
                    | SimEvent::StaffWalkedOut { .. }
                    | SimEvent::StaffSick { .. }
            ) {
                println!("  {} {:?}", w.clock_label(), e);
            }
        }
        if w.minute() / every != last && !w.in_prep() {
            last = w.minute() / every;
            let st: Vec<String> = w
                .parties
                .iter()
                .map(|p| format!("{:?}{}", p.state, if p.regular.is_some() { "R" } else { "" }))
                .collect();
            let pend: Vec<String> = w
                .items
                .iter()
                .filter(|i| matches!(i.state, ItemState::Pending | ItemState::Preparing))
                .map(|i| format!("{}{:?}{}{}", i.menu, i.origin, if i.wish { "W" } else { "" }, if i.by.is_some() { "*" } else { "" }))
                .collect();
            let claimed: Vec<String> = w
                .furniture
                .iter()
                .filter(|f| f.user.is_some())
                .map(|f| format!("{}#{}={:?}", f.def, f.uid, f.user))
                .collect();
            let duties: Vec<String> = w
                .staff
                .iter()
                .filter(|s| s.on_duty() && !s.owner)
                .map(|s| format!("{}{:?}e{:.0}m{:.0}", s.id, s.duties, s.energy, s.mood))
                .collect();
            let mut paths = std::mem::take(&mut w.paths);
            for (idx, st) in w.staff.iter().enumerate() {
                if st.owner || !st.on_duty() {
                    continue;
                }
                let from = st.tile();
                let reach: Vec<String> = w
                    .furniture
                    .iter()
                    .filter(|f| {
                        c.furniture_def(&f.def).is_some_and(|d| {
                            d.has_tag("station:coffee") || d.has_tag("station:fridge") || d.has_tag("sink")
                        })
                    })
                    .map(|f| {
                        let acc = super::staff::station_access(&w, &c, f.uid);
                        let ok = acc.map(|a| paths.get(&w.grid, from, a, false).is_some());
                        format!("{}@{:?}->{:?}:{:?}", f.def, (f.x, f.y), acc, ok)
                    })
                    .collect();
                println!("   {} at {:?} walkable {} reach {:?}", st.name, from, w.grid.walkable(from.0, from.1), reach);
                let peek = super::staff::peek_work(&w, &c, idx);
                let probe: Vec<String> = w
                    .items
                    .iter()
                    .filter(|i| i.state == ItemState::Pending && matches!(i.origin, ItemOrigin::Unassigned))
                    .take(3)
                    .filter_map(|i| c.menu_item(&i.menu).map(|mi| (i, mi)))
                    .map(|(i, mi)| {
                        let free = super::staff::station_free(&w, &c, &mi.station, st.id);
                        let acc = free.and_then(|u| super::staff::station_access(&w, &c, u));
                        format!("{}:{:?}/{}/min{}:free{:?}:acc{:?}:sk{:.1}", i.menu, mi.job, mi.station, mi.min_skill, free, acc, st.skill(mi.job, w.night))
                    })
                    .collect();
                println!("   {} {:?} task {:?} peek {:?} probe {:?} speed {:.2}", st.name, st.duties, st.task.is_some(), peek.map(|t| format!("{t:?}").chars().take(40).collect::<String>()), probe, super::staff::work_speed(&w, &c, idx, Job::Bar));
            }
            w.paths = paths;
            println!("   claimed {claimed:?} duties {duties:?} mods {:?}", w.mods.iter().map(|m| format!("{:?}", m.kind)).collect::<Vec<_>>());
            println!(
                "{} q{} parties {} pend {:?} plan {:?} | cur {:?} queue {} | staff {:?}",
                w.clock_label(),
                w.queue_len(),
                w.parties.len(),
                pend,
                st,
                w.hands.player.current.as_ref().map(|a| format!("{:?}", a.act)),
                w.hands.player.queue.len(),
                w.staff
                    .iter()
                    .map(|s| format!("{}:{:?}/{:?}", s.name, s.status, s.task.as_ref().map(|t| format!("{t:?}").chars().take(30).collect::<String>())))
                    .collect::<Vec<_>>(),
            );
        }
    }
    let s = w.last_summary().unwrap();
    println!(
        "summary arrived {} served {} forgiven {} revenue {}",
        s.guests_arrived, s.guests_served, s.guests_forgiven, s.revenue
    );
}

#[test]
#[ignore = "QA: per-night crew/service diagnostics for one seed"]
fn diag() {
    let c = content();
    let nights: u32 = std::env::var("SBX_NIGHTS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);
    let diff = diff_of(&std::env::var("SBX_DIFF").unwrap_or_default());
    let variant = std::env::var("SBX_VARIANT").unwrap_or_else(|_| "default".into());
    let seed = seeds_from_env()[0];
    let mut w = World::new_sandbox(&c, setup(&c, seed, diff, &variant));
    let mut b = bot_for(seed, diff);
    for _ in 0..nights {
        if w.is_closed() {
            break;
        }
        let ev = b.run_night(&mut w, &c);
        let s = w.last_summary().unwrap().clone();
        let n = s.night;
        let wd = ((n - 1) % 7) as usize;
        let crew: Vec<String> = w
            .staff
            .iter()
            .filter(|st| !st.owner)
            .map(|st| {
                let sh = w.app.published.get(&st.id).map(|r| r[wd]);
                format!(
                    "{}:{:?}:{:?}:sk{:.1}",
                    &st.name[..3.min(st.name.len())],
                    sh,
                    st.duties,
                    st.skills.iter().cloned().fold(0.0f32, f32::max)
                )
            })
            .collect();
        let wish_orders = ev
            .iter()
            .filter(|e| matches!(e, SimEvent::PlayerOrderTaken { .. }))
            .count();
        println!(
            "n{n} arr {} srv {} left {} K {} H {} wish {} pot {} mg {} reg {}/{} idle {} rep {:.0} | {}",
            s.guests_arrived,
            s.guests_served,
            s.guests_left_unhappy,
            s.karolina_items,
            s.helper_items,
            s.wishes_served,
            wish_orders,
            s.minigames,
            s.regular_items,
            s.regular_items_karolina,
            s.karolina_idle_minutes,
            s.reputation_end,
            crew.join(" ")
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Regressions (fast)
// ---------------------------------------------------------------------------------------------

/// The setup's «Allow going into debt» did nothing: a Cozy café with it off ran on at
/// −40 000 Kč (sandbox seed 13, night 100). Off, three nights in a row in the red close it,
/// as on Classic; on (the default) Cozy never closes over money.
#[test]
fn allow_debt_off_closes_a_cafe_in_the_red() {
    let c = content();
    for (allow, closes) in [(false, true), (true, false)] {
        let mut s = SandboxSetup {
            seed: 13,
            difficulty: Difficulty::Cozy,
            ..Default::default()
        };
        s.options.insert("allow_debt".into(), allow);
        let mut w = World::new_sandbox(&c, s);
        let mut nights = 0;
        while !w.is_closed() && nights < 4 {
            w.econ.money = -50_000;
            w.run_night_headless(&c);
            nights += 1;
        }
        assert_eq!(w.is_closed(), closes, "allow_debt {allow}");
        if closes {
            assert_eq!(nights, super::story::NO_DEBT_BANKRUPT_AFTER as usize);
            assert!(w.econ.nights_in_red >= super::story::NO_DEBT_BANKRUPT_AFTER);
        }
    }
}

/// Helper utilisation tonight, sampled every tick after the doors open.
#[derive(Default, Debug, Clone)]
struct Util {
    /// helper-ticks on the floor (OnDuty)
    on: u32,
    busy: u32,
    brk: u32,
    /// no task / Idle while their duties offer nothing
    idle: u32,
    /// no task / Idle although `peek_work` has something (picked and dropped: the stall)
    stalled: u32,
    /// idle while plain tickets of a job wait that no helper on the floor holds a duty for
    uncovered: u32,
    /// idle while plain tickets of their own duty's job wait (offered nothing anyway)
    own_waiting: u32,
    by_kind: BTreeMap<&'static str, u32>,
    /// on-duty helpers with no duties at all (ticks)
    no_duties: u32,
    /// idle while helper-made items stand ready and nobody on the floor holds Serving
    ready_unserved: u32,
}

fn sample_util(w: &World, c: &Content, u: &mut Util) {
    if w.in_prep() || w.phase() != Phase::Night {
        return;
    }
    let waiting_jobs: Vec<Job> = w
        .items
        .iter()
        .filter(|i| i.state == ItemState::Pending && matches!(i.origin, ItemOrigin::Unassigned))
        .filter(|i| !i.wish || i.hand_off_match.is_some())
        .filter_map(|i| c.menu_item(&i.menu).map(|m| m.job))
        .collect();
    let held: Vec<crate::content::handson::Duty> = w
        .staff
        .iter()
        .filter(|s| !s.owner && s.on_duty())
        .flat_map(|s| s.duties.iter().copied())
        .collect();
    let serving_held = held.contains(&crate::content::handson::Duty::Serving);
    let ready_helper = w
        .items
        .iter()
        .any(|i| i.state == ItemState::Ready && matches!(i.origin, ItemOrigin::Helper { .. }));
    for (idx, s) in w.staff.iter().enumerate() {
        if s.owner || !s.on_duty() {
            continue;
        }
        u.on += 1;
        if s.duties.is_empty() {
            u.no_duties += 1;
        }
        let kind = match &s.task {
            Some(Task::TakeOrder { .. }) => "order",
            Some(Task::Prepare { .. }) => "prepare",
            Some(Task::Serve { .. }) => "serve",
            Some(Task::ClearTable { .. }) => "clear",
            Some(Task::Wash { .. }) => "wash",
            Some(Task::Mop { .. }) | Some(Task::Bucket { .. }) => "floor",
            Some(Task::Checklist { .. }) => "checklist",
            Some(Task::Break { .. }) => "break",
            Some(Task::Leave) => "leave",
            Some(Task::Idle { .. }) | None => "idle",
        };
        match kind {
            "break" => u.brk += 1,
            "idle" => {
                if super::staff::peek_work(w, c, idx).is_some() {
                    u.stalled += 1;
                } else {
                    u.idle += 1;
                    if ready_helper && !serving_held {
                        u.ready_unserved += 1;
                    }
                    if waiting_jobs.iter().any(|j| s.duties.iter().any(|d| d.job() == *j)) {
                        u.own_waiting += 1;
                    } else if waiting_jobs.iter().any(|j| !held.iter().any(|d| d.job() == *j)) {
                        u.uncovered += 1;
                    }
                }
            }
            _ => {
                u.busy += 1;
                *u.by_kind.entry(kind).or_insert(0) += 1;
            }
        }
    }
}

fn util_line(u: &Util) -> String {
    let p = |x: u32| x as f32 * 100.0 / u.on.max(1) as f32;
    let kinds: Vec<String> = u.by_kind.iter().map(|(k, v)| format!("{k} {:.0}", p(*v))).collect();
    format!(
        "on {:>5} min | busy {:>4.1}% break {:>4.1}% idle {:>4.1}% stalled {:>4.1}% | idle w/ own job waiting {:>4.1}% w/ uncovered job waiting {:>4.1}% w/ ready cups, no server {:>4.1}% | no duties {:.1}% | {}",
        u.on / 10,
        p(u.busy),
        p(u.brk),
        p(u.idle),
        p(u.stalled),
        p(u.own_waiting),
        p(u.uncovered),
        p(u.ready_unserved),
        p(u.no_duties),
        kinds.join(" ")
    )
}

/// One night of helper utilisation with the bot playing.
fn util_night(w: &mut World, b: &mut bot::PlayerBot, c: &Content) -> Util {
    let mut u = Util::default();
    b.run_night_watched(w, c, |w| sample_util(w, c, &mut u));
    u
}

#[test]
#[ignore = "QA: helper utilisation per night (SBX_SEEDS, SBX_NIGHTS, SBX_DIFF, SBX_VARIANT)"]
fn utilisation() {
    let c = content();
    let nights: u32 = std::env::var("SBX_NIGHTS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);
    let diff = diff_of(&std::env::var("SBX_DIFF").unwrap_or_default());
    let variant = std::env::var("SBX_VARIANT").unwrap_or_else(|_| "default".into());
    let story = variant == "story";
    for seed in seeds_from_env() {
        let mut w = if story {
            World::new_story_seeded(&c, diff, seed)
        } else {
            World::new_sandbox(&c, setup(&c, seed, diff, &variant))
        };
        let mut b = bot_for(seed, diff);
        for _ in 0..nights {
            if w.is_closed() {
                break;
            }
            let u = util_night(&mut w, &mut b, &c);
            let s = w.last_summary().unwrap();
            println!(
                "U,{seed},{},{variant},{},{},{},{},{},{},{},{},{} | {}",
                diff.id(),
                s.night,
                u.on,
                u.busy,
                u.brk,
                u.idle,
                u.stalled,
                u.own_waiting,
                u.uncovered,
                s.helper_items,
                util_line(&u)
            );
        }
    }
}

/// Sandbox seed 4 (Classic), from night ~50: grandma's attic pieces come down around the
/// same spot, and the bookcase, the window bench and the rubber plant boxed in the front tile
/// of her marble table (every piece still reachable, so the placement allowed it). The
/// table's access tile was that pocket: every helper with the dishes duty picked "clear the
/// marble table" (rank 0, the best score), found no path, dropped it and picked it again two
/// ticks later — all night, every night. Their other duties never got a turn (night 97: two
/// helpers on the floor, 0 items made, 28 guests walked out; 31 dishes on the table).
#[test]
fn a_boxed_in_front_tile_does_not_stall_the_helpers() {
    let c = content();
    let mut w = World::new_sandbox(
        &c,
        SandboxSetup {
            seed: 4,
            ..Default::default()
        },
    );
    for (def, x, y) in [
        ("attic_marble_table", 8, 10),
        ("attic_bookcase", 7, 11),
        ("attic_window_bench", 9, 11),
        ("attic_rubber_plant", 8, 12),
    ] {
        assert!(
            super::commands::place_free(&mut w, &c, def, x, y, 0),
            "{def} fits at ({x}, {y})"
        );
    }
    assert!(w.grid.walkable(8, 11) && !w.grid.reached(8, 11), "a walkable pocket");
    let table = w
        .furniture
        .iter()
        .find(|f| f.def == "attic_marble_table")
        .unwrap()
        .uid;
    let acc = super::staff::station_access(&w, &c, table).unwrap();
    assert!(w.grid.reached(acc.0, acc.1), "the table is cleared from a side one reaches");
    let helper = w.staff.iter().find(|s| !s.owner).unwrap().id;
    w.start_night(&c);
    w.skip_prep(&c);
    for s in w.staff.iter_mut().filter(|s| s.id == helper) {
        s.duties = vec![crate::content::handson::Duty::Dishes];
    }
    w.furniture.iter_mut().find(|f| f.uid == table).unwrap().dishes = 6;
    let mut cleared = false;
    for _ in 0..600 {
        w.tick(&c);
        w.drain_events();
        if w.furniture.iter().find(|f| f.uid == table).unwrap().dishes == 0 {
            cleared = true;
            break;
        }
    }
    assert!(cleared, "the helper clears the marble table");
}

/// The sandbox rent grew 5 % every 10 nights for ever: 2 200 Kč on night 1, ~3 600 on night
/// 100, ~9 500 on night 300, and every long Cozy / Classic game ended in the red (Cozy seed 43:
/// −445 000 Kč on night 300). It now stops at `rent_growth_max` × the town's top starting
/// rent (a fixed 4 200 Kč cap still sank a village café, seed 42: −104 000 by night 300).
#[test]
fn sandbox_rent_growth_stops_at_the_cap() {
    let c = content();
    let mul = c.difficulty("cozy").unwrap().rent_growth_max.unwrap();
    let mut w = World::new_sandbox(&c, SandboxSetup::default());
    let start = w.econ.rent;
    let cap = w.sandbox_rent_cap(&c, mul);
    assert!(start < cap && cap < start * 2, "start {start} cap {cap}");
    let (lo, hi) = (c.sandbox.rent.0 as i64, c.sandbox.rent.1 as i64);
    for (size, range) in [(0.6, (lo, lo + (hi - lo) / 5)), (1.5, (lo, hi))] {
        w.town.size = size;
        let top = w.sandbox_rent_cap(&c, 1.0);
        assert!(top >= range.0 && top <= range.1, "size {size}: top {top}");
    }
    w.town.size = 1.0;
    let cap = w.sandbox_rent_cap(&c, mul);
    w.econ.rent = cap - 100;
    w.night = 21;
    w.begin_day(&c);
    assert_eq!(w.econ.rent, cap);
    w.night = 31;
    w.begin_day(&c);
    assert_eq!(w.econ.rent, cap, "no further growth");
    // an old save already above the cap keeps its rent (never lowered)
    w.econ.rent = cap + 500;
    w.night = 41;
    w.begin_day(&c);
    assert_eq!(w.econ.rent, cap + 500);
}

/// 300 nights, one seed per difficulty: nothing panics, no NaN, the rent stays capped, the
/// helpers never stall on unreachable work, the season wraps.
#[test]
#[ignore = "long: ~1 min in release"]
fn three_hundred_nights_stay_sane() {
    let c = content();
    for diff in [Difficulty::Cozy, Difficulty::Relaxed, Difficulty::Classic] {
        let mut w = World::new_sandbox(&c, setup(&c, 43, diff, "norain"));
        let mut b = bot_for(43, diff);
        let mut seasons = std::collections::BTreeSet::new();
        let mut stalled = 0u32;
        let mut on = 0u32;
        for _ in 0..300 {
            if w.is_closed() {
                break;
            }
            let u = util_night(&mut w, &mut b, &c);
            stalled += u.stalled;
            on += u.on;
            seasons.insert(w.town.season);
            let cap = c
                .difficulty(diff.id())
                .unwrap()
                .rent_growth_max
                .map_or(i64::MAX, |m| w.sandbox_rent_cap(&c, m));
            assert!(w.econ.rent <= cap, "{diff:?} rent {}", w.econ.rent);
            assert!(w.reputation().is_finite() && w.dirt.is_finite());
        }
        let save = w.save_to_string().unwrap();
        assert!(!save.contains("NaN") && !save.contains("inf"));
        assert_eq!(seasons.len(), 4, "{diff:?}");
        // ~1.8 % is the normal pick-up beat (every other tick); the boxed-in table was 5–10 %
        // on average from night 40 and up to 87 % on a bad night
        assert!(
            (stalled as f32) < on as f32 * 0.04,
            "{diff:?}: helpers stalled {stalled} of {on} ticks"
        );
    }
}
