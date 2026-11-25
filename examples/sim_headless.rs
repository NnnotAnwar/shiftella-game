//! Runs the simulation headless (no window) and prints one line per night — used for balancing.
//!
//! ```text
//! cargo run --release --example sim_headless -- [nights] [story|sandbox] [cozy|classic] [seed]
//! ```
//! The "player" here is a simple bot: lights the window, ticks jobless checklist items,
//! hires the first candidates up to 3 staff, answers choices with their default.

use std::time::Instant;

use shiftella_game::content::Content;
use shiftella_game::sim::{Command, Difficulty, Phase, SandboxSetup, Shift, SimEvent, World};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let nights: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(5);
    let mode = args.get(2).map(String::as_str).unwrap_or("story");
    let difficulty = match args.get(3).map(String::as_str) {
        Some("classic") => Difficulty::Classic,
        _ => Difficulty::Cozy,
    };
    let seed: u64 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(7);
    let content = Content::load("assets/data").expect("content");
    let mut world = if mode == "sandbox" {
        World::new_sandbox(
            &content,
            SandboxSetup {
                seed,
                difficulty,
                ..Default::default()
            },
        )
    } else {
        World::new_story_seeded(&content, difficulty, seed)
    };
    println!(
        "{} · {} · {:?} · town {} · money {}",
        mode,
        world.cafe_name(),
        difficulty,
        world.town().name,
        world.money()
    );
    println!(
        "night  guests served left away | revenue  tips  wages supply  rent  pay |   money   rep  sat | ev bad | exh stuck tick-µs"
    );
    let mut total_ticks = 0u64;
    let mut total_time = 0f64;
    let mut max_agents = 0usize;
    for _ in 0..nights {
        if world.is_closed() {
            break;
        }
        play_day(&mut world, &content);
        // finish day cutscenes, start the night
        finish_cutscenes(&mut world, &content);
        world.start_night(&content);
        let t0 = Instant::now();
        let mut ticks = 0u64;
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
            ticks += 1;
            max_agents = max_agents.max(world.guests().len() + world.staff().len());
            for e in world.drain_events() {
                if let SimEvent::ChoiceNeeded { .. } = e {
                    // let it time out → default choice
                }
            }
        }
        let dt = t0.elapsed().as_secs_f64();
        total_ticks += ticks;
        total_time += dt;
        finish_cutscenes(&mut world, &content);
        if let Some(s) = world.last_summary() {
            println!(
                "{:>5}  {:>6} {:>6} {:>4} {:>4} | {:>7} {:>5} {:>6} {:>6} {:>5} {:>5} | {:>7} {:>5.1} {:>4.0} | {:>2} {:>3} | {:>3} {:>5} {:>6.1}  {}",
                s.night,
                s.guests_arrived,
                s.guests_served,
                s.guests_left_unhappy,
                s.guests_turned_away,
                s.revenue,
                s.tips,
                s.wages,
                s.supplies,
                s.rent,
                s.payment,
                s.money_end,
                s.reputation_end,
                s.avg_satisfaction,
                s.events.len(),
                s.bad_events,
                s.exhausted_minutes,
                s.stuck_resets,
                dt * 1e6 / ticks.max(1) as f64,
                s.events.join(","),
            );
        }
    }
    if let Some(e) = world.ending() {
        println!("ending: {e} ({:?})", world.ending_variant());
    }
    println!(
        "notes {:?} · staff {} · max agents {} · avg tick {:.1} µs",
        world.notes_found(),
        world.staff().len(),
        max_agents,
        total_time * 1e6 / total_ticks.max(1) as f64
    );
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

fn play_day(world: &mut World, content: &Content) {
    finish_cutscenes(world, content);
    let crew = world.staff().len();
    // owner + 1 hire from night 2, +1 from night 5, +1 from night 9
    let plan: &[u32] = if world.difficulty() == Difficulty::Classic {
        &[2, 8, 15]
    } else {
        &[2, 5, 9]
    };
    let want = 1 + plan.iter().filter(|n| world.night() >= **n).count();
    if crew < want {
        let ids: Vec<u32> = world.candidates().iter().map(|c| c.id).collect();
        for id in ids.into_iter().take(want - crew) {
            let _ = world.apply(content, Command::Hire { candidate: id });
        }
    }
    // the newest of 3+ hires covers only the busy first half of the night
    let hires: Vec<u32> = world
        .staff()
        .iter()
        .filter(|s| !s.owner)
        .map(|s| s.id)
        .collect();
    if hires.len() >= 3 {
        let id = *hires.last().unwrap();
        for d in 0..7 {
            let _ = world.apply(
                content,
                Command::SetShift {
                    staff: id,
                    weekday: d,
                    shift: Shift::Early,
                },
            );
        }
    }
    let _ = world.apply(content, Command::PublishSchedule);
    let _ = world.drain_events();
}
