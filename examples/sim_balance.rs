//! Balance + QA sweep: plays several seeds headless and prints per-night averages plus
//! sanity checks (stuck agents, NaN, staff never resting, events firing every night,
//! regular arcs, cameo nights, endings).
//!
//! ```text
//! cargo run --release --example sim_balance -- [nights] [story|sandbox] [cozy|classic] [seeds] [character]
//! SIM_PLAN=2,2,5 …   # nights on which the bot hires one more person (default: like sim_headless)
//! SIM_VISITS=1        # print each regular's last visits (night:served minutes)
//! SIM_FULL=1          # everybody 22–06 every night instead of the default rozpis
//! ```
//! The bot player lights the window, ticks its checklist items, hires by plan, publishes the
//! default rozpis and lets every choice time out to its default.

use std::collections::BTreeMap;

use shiftella_game::content::Content;
use shiftella_game::sim::{
    Command, Difficulty, PartyState, Phase, SandboxSetup, SimEvent, StaffStatus, World,
};

#[derive(Default, Clone)]
struct Row {
    arrived: f32,
    served: f32,
    left: f32,
    away: f32,
    revenue: f32,
    wages: f32,
    net: f32,
    money: f32,
    rep: f32,
    sat: f32,
    crew: f32,
    n: f32,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let nights: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(30);
    let mode = args.get(2).map(String::as_str).unwrap_or("story");
    let difficulty = match args.get(3).map(String::as_str) {
        Some("classic") => Difficulty::Classic,
        _ => Difficulty::Cozy,
    };
    let seeds: u64 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(6);
    let character = args.get(5).cloned();
    let plan: Vec<u32> = std::env::var("SIM_PLAN")
        .ok()
        .map(|s| s.split(',').filter_map(|x| x.trim().parse().ok()).collect())
        .unwrap_or_else(|| {
            if difficulty == Difficulty::Classic {
                vec![2, 8, 15]
            } else {
                vec![2, 5, 9]
            }
        });
    let content = Content::load("assets/data").expect("content");

    let mut rows: Vec<Row> = vec![Row::default(); nights as usize + 1];
    let mut angry_by_state: BTreeMap<String, u32> = BTreeMap::new();
    let mut event_nights: BTreeMap<String, u32> = BTreeMap::new();
    let mut nights_played = 0u32;
    let mut endings: BTreeMap<String, u32> = BTreeMap::new();
    let mut problems: Vec<String> = Vec::new();
    let mut arcs: BTreeMap<String, Vec<i32>> = BTreeMap::new();
    let mut anwar_nights: Vec<u32> = Vec::new();
    let mut breaks = 0u32;
    let mut staff_nights = 0u32;
    let mut no_break_tired = 0u32;
    let mut stuck = 0u32;
    let mut exhausted = 0u32;
    let mut day1: Vec<(i64, i64, i64)> = Vec::new();

    for seed in 1..=seeds {
        let seed = seed * 7919 + 13;
        let mut world = if mode == "sandbox" {
            World::new_sandbox(
                &content,
                SandboxSetup {
                    seed,
                    difficulty,
                    character: character.clone(),
                    ..Default::default()
                },
            )
        } else {
            World::new_story_seeded(&content, difficulty, seed)
        };
        day1.push((world.money(), world.debt(), world.rent()));
        for _ in 0..nights {
            if world.is_closed() {
                break;
            }
            play_day(&mut world, &content, &plan);
            finish_cutscenes(&mut world, &content);
            world.start_night(&content);
            let mut last_state: BTreeMap<u32, PartyState> = BTreeMap::new();
            let mut counted: std::collections::BTreeSet<u32> = Default::default();
            let mut rested: std::collections::BTreeSet<u32> = Default::default();
            let mut low_energy: std::collections::BTreeSet<u32> = Default::default();
            let mut fired: std::collections::BTreeSet<String> = Default::default();
            while world.phase() == Phase::Night && !world.is_closed() {
                finish_cutscenes(&mut world, &content);
                if world.minute() == 1 {
                    let _ = world.apply(
                        &content,
                        Command::TickChecklist {
                            id: "light_window".into(),
                        },
                    );
                }
                if world.minute() == 475 {
                    for id in ["handover_note", "light_off_at_six"] {
                        let _ = world.apply(&content, Command::TickChecklist { id: id.into() });
                    }
                }
                world.tick(&content);
                for p in world.parties() {
                    if p.angry && counted.insert(p.id) {
                        let st = last_state.get(&p.id).copied().unwrap_or(p.state);
                        *angry_by_state.entry(format!("{st:?}")).or_default() +=
                            p.members.len() as u32;
                    }
                    if p.state != PartyState::Leaving {
                        last_state.insert(p.id, p.state);
                    }
                    if !p.patience.is_finite() || !p.satisfaction.is_finite() {
                        problems.push(format!("seed {seed} night {}: NaN party", world.night()));
                    }
                }
                for s in world.staff() {
                    if !(s.energy.is_finite() && s.mood.is_finite() && s.hunger.is_finite()) {
                        problems.push(format!("seed {seed} night {}: NaN staff", world.night()));
                    }
                    if s.status == StaffStatus::OnDuty && s.energy < 25.0 {
                        low_energy.insert(s.id);
                    }
                }
                if !world.reputation().is_finite() {
                    problems.push(format!("seed {seed}: NaN reputation"));
                }
                for e in world.drain_events() {
                    match e {
                        SimEvent::StaffBreak { staff } => {
                            breaks += 1;
                            rested.insert(staff);
                        }
                        SimEvent::EventStarted { id } => {
                            fired.insert(id);
                        }
                        SimEvent::RegularArrived { regular, .. } if regular == "guest_anwar" => {
                            anwar_nights.push(world.night());
                        }
                        _ => {}
                    }
                }
            }
            finish_cutscenes(&mut world, &content);
            for id in fired {
                *event_nights.entry(id).or_default() += 1;
            }
            staff_nights += world
                .staff()
                .iter()
                .filter(|s| s.minutes_worked > 0)
                .count() as u32;
            no_break_tired += low_energy.difference(&rested).count() as u32;
            nights_played += 1;
            if let Some(s) = world.last_summary() {
                let r = &mut rows[s.night.min(nights) as usize];
                r.arrived += s.guests_arrived as f32;
                r.served += s.guests_served as f32;
                r.left += s.guests_left_unhappy as f32;
                r.away += s.guests_turned_away as f32;
                r.revenue += s.revenue as f32;
                r.wages += s.wages as f32;
                r.net += s.net as f32;
                r.money += s.money_end as f32;
                r.rep += s.reputation_end;
                r.sat += s.avg_satisfaction;
                r.crew += world.staff().len() as f32;
                r.n += 1.0;
                stuck += s.stuck_resets;
                exhausted += s.exhausted_minutes;
                if s.money_end < -50_000 {
                    problems.push(format!(
                        "seed {seed} night {}: money {}",
                        s.night, s.money_end
                    ));
                }
            }
        }
        *endings
            .entry(
                world
                    .ending()
                    .map(|e| format!("{e}/{}", world.ending_variant().unwrap_or("-")))
                    .unwrap_or_else(|| "none".into()),
            )
            .or_default() += 1;
        for (id, st) in &world.story().regulars {
            if std::env::var("SIM_VISITS").is_ok() {
                let v: Vec<String> = st
                    .visits
                    .iter()
                    .map(|v| format!("{}:{:?}", v.night, v.served_minutes))
                    .collect();
                let trust = world.story().trust.get(id).copied().unwrap_or(0.0);
                println!("  {id} trust {trust:.0} visits {}", v.join(" "));
            }
            let arc = world
                .story()
                .counters
                .get(&format!("arc.{id}"))
                .copied()
                .unwrap_or(0);
            arcs.entry(id.clone()).or_default().push(arc);
        }
    }

    println!(
        "{mode} · {difficulty:?} · {seeds} seeds · plan {plan:?} · day 1 money/debt/rent {:?}",
        day1.first()
    );
    println!("night arrived served  left  away | revenue  wages    net |   money   rep  sat crew");
    for (n, r) in rows.iter().enumerate().skip(1) {
        if r.n == 0.0 {
            continue;
        }
        let k = r.n;
        println!(
            "{n:>5} {:>7.1} {:>6.1} {:>5.1} {:>5.1} | {:>7.0} {:>6.0} {:>6.0} | {:>7.0} {:>5.1} {:>4.0} {:>4.1}{}",
            r.arrived / k,
            r.served / k,
            r.left / k,
            r.away / k,
            r.revenue / k,
            r.wages / k,
            r.net / k,
            r.money / k,
            r.rep / k,
            r.sat / k,
            r.crew / k,
            if k < seeds as f32 {
                format!("  ({k} runs)")
            } else {
                String::new()
            }
        );
    }
    println!("angry guests by state they left from: {angry_by_state:?}");
    println!("endings: {endings:?}");
    println!(
        "breaks per staff-night {:.2} · tired staff without a break {no_break_tired} · stuck resets {stuck} · exhausted staff-min {exhausted}",
        breaks as f32 / staff_nights.max(1) as f32
    );
    let mut ev: Vec<(String, u32)> = event_nights.into_iter().collect();
    ev.sort_by_key(|e| std::cmp::Reverse(e.1));
    let ev_str: Vec<String> = ev
        .iter()
        .take(12)
        .map(|(id, n)| {
            format!(
                "{id} {:.0}%",
                *n as f32 * 100.0 / nights_played.max(1) as f32
            )
        })
        .collect();
    println!("events (share of nights): {}", ev_str.join(", "));
    let arc_str: Vec<String> = arcs.iter().map(|(id, v)| format!("{id} {v:?}")).collect();
    println!("arcs: {}", arc_str.join(" · "));
    anwar_nights.sort();
    anwar_nights.dedup();
    println!("anwar visited on nights {anwar_nights:?}");
    if problems.is_empty() {
        println!("problems: none");
    } else {
        problems.dedup();
        println!("problems ({}):", problems.len());
        for p in problems.iter().take(20) {
            println!("  {p}");
        }
    }
}

fn finish_cutscenes(world: &mut World, content: &Content) {
    while let Some(id) = world.playing_cutscene().map(str::to_string) {
        let _ = world.apply(
            content,
            Command::CutsceneFinished {
                id,
                choices: vec![0, 0, 0],
            },
        );
    }
}

fn play_day(world: &mut World, content: &Content, plan: &[u32]) {
    finish_cutscenes(world, content);
    let crew = world.staff().len();
    let want = 1 + plan.iter().filter(|n| world.night() >= **n).count();
    let base = if world.mode() == shiftella_game::sim::Mode::Sandbox {
        1
    } else {
        0
    };
    if crew < want + base {
        let ids: Vec<u32> = world.candidates().iter().map(|c| c.id).collect();
        for id in ids.into_iter().take(want + base - crew) {
            let _ = world.apply(content, Command::Hire { candidate: id });
        }
    }
    if std::env::var("SIM_FULL").is_ok() {
        // the old default: everybody 22–06 every night
        let ids: Vec<u32> = world.staff().iter().map(|s| s.id).collect();
        for id in ids {
            for d in 0..7 {
                let _ = world.apply(
                    content,
                    Command::SetShift {
                        staff: id,
                        weekday: d,
                        shift: shiftella_game::sim::Shift::Full,
                    },
                );
            }
        }
    }
    let _ = world.apply(content, Command::PublishSchedule);
    let _ = world.drain_events();
}
