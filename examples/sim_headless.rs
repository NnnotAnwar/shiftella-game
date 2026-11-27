//! Runs the simulation headless (no window) and prints one line per night — used for balancing.
//!
//! ```text
//! cargo run --release --example sim_headless -- [nights] [story|sandbox] [cozy|classic] [seed]
//!     [--player-bot [quality] [react_seconds]]
//! ```
//! Without `--player-bot` nobody works behind the counter: Karolina "steps back" (the helpers
//! do their duties, guests wait and — on Cozy — forgive). With it, `sim::bot::PlayerBot` plays:
//! takes orders, brews wishes by hand at `quality` (±0.2), quick-brews unowned plain tickets,
//! clears, washes, mops, collects tips, hires on nights 3/5/9 (Classic 3/8/15) and delegates.
//! `SIM_STRESS=1` adds 60 guests at 23:00 every night (perf check, without the bot).

use std::time::Instant;

use shiftella_game::content::Content;
use shiftella_game::sim::bot::{BotConfig, PlayerBot, finish_cutscenes};
use shiftella_game::sim::{Command, Difficulty, Phase, SandboxSetup, World};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let bot_at = args.iter().position(|a| a == "--player-bot");
    let pos: Vec<&String> = args
        .iter()
        .take(bot_at.unwrap_or(args.len()))
        .skip(1)
        .collect();
    let nights: u32 = pos.first().and_then(|s| s.parse().ok()).unwrap_or(5);
    let mode = pos.get(1).map(|s| s.as_str()).unwrap_or("story");
    let difficulty = match pos.get(2).map(|s| s.as_str()) {
        Some("classic") => Difficulty::Classic,
        _ => Difficulty::Cozy,
    };
    let seed: u64 = pos.get(3).and_then(|s| s.parse().ok()).unwrap_or(7);
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
    let mut bot = bot_at.map(|i| {
        let quality = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(0.75);
        let react = args.get(i + 2).and_then(|s| s.parse().ok()).unwrap_or(3.0);
        PlayerBot::new(BotConfig {
            quality,
            react_seconds: react,
            hire_nights: if difficulty == Difficulty::Classic {
                vec![3, 8, 15]
            } else {
                vec![3, 5, 9]
            },
            seed,
            ..Default::default()
        })
    });
    println!(
        "{} · {} · {:?} · town {} · money {} · {}",
        mode,
        world.cafe_name(),
        difficulty,
        world.town().name,
        world.money(),
        if bot.is_some() {
            "player-bot"
        } else {
            "nobody behind the counter"
        }
    );
    println!(
        "night  guests served forgv left | K-items help wish perf mg | idle% todo med/pk | revenue  tips  wages |   money   rep  sat | real-min | tick-µs"
    );
    let stress = std::env::var("SIM_STRESS").is_ok();
    let mut total_ticks = 0u64;
    let mut total_time = 0f64;
    for _ in 0..nights {
        if world.is_closed() {
            break;
        }
        let t0 = Instant::now();
        let ticks;
        let mut real_min = 12.0;
        match bot.as_mut() {
            Some(b) => {
                b.run_night(&mut world, &content);
                let n = b.nights.last().cloned().unwrap_or_default();
                ticks = n.ticks as u64;
                real_min = n.real_minutes;
            }
            None => {
                finish_cutscenes(&mut world, &content);
                world.start_night(&content);
                let mut t = 0u64;
                while world.phase() == Phase::Night && !world.is_closed() {
                    finish_cutscenes(&mut world, &content);
                    if stress && world.minute() == 60 && world.tick_count() % 10 == 0 {
                        world.debug_spawn_guests(&content, "students", 60, 10);
                    }
                    // nobody behind the counter: open at 21:59, close up at 06:29
                    if world.waiting_for_doors() {
                        let _ = world.apply(&content, Command::OpenDoors);
                    } else if world.waiting_to_close() {
                        let _ = world.apply(&content, Command::CloseUp);
                        continue;
                    }
                    world.tick(&content);
                    world.drain_events();
                    t += 1;
                }
                finish_cutscenes(&mut world, &content);
                ticks = t;
            }
        }
        let dt = t0.elapsed().as_secs_f64();
        total_ticks += ticks;
        total_time += dt;
        if let Some(s) = world.last_summary() {
            // idle share of the night minutes outside the quiet hour (420)
            let idle = s.karolina_idle_minutes as f32 / 420.0 * 100.0;
            println!(
                "{:>5}  {:>6} {:>6} {:>5} {:>4} | {:>7} {:>4} {:>4} {:>4} {:>2} | {:>5.1} {:>4}/{:<3} | {:>7} {:>5} {:>6} | {:>7} {:>5.1} {:>4.0} | {:>8.1} | {:>6.1}",
                s.night,
                s.guests_arrived,
                s.guests_served,
                s.guests_forgiven,
                s.guests_left_unhappy,
                s.karolina_items,
                s.helper_items,
                s.wishes_served,
                s.wishes_perfect,
                s.minigames,
                idle,
                s.todo_median,
                s.todo_peak,
                s.revenue,
                s.tips,
                s.wages,
                s.money_end,
                s.reputation_end,
                s.avg_satisfaction,
                real_min,
                dt * 1e6 / ticks.max(1) as f64,
            );
        }
    }
    if let Some(e) = world.ending() {
        println!("ending: {e} ({:?})", world.ending_variant());
    }
    match world.signature() {
        Some(d) => println!(
            "house drink: «{}» from night {} ({} {:?}, +{} Kč) · asked for by name and served {}× · photos {}",
            d.name,
            d.made_night,
            d.base,
            d.def.tags,
            d.def.price_add,
            world.signature_asked_served(),
            world.photos().len()
        ),
        None => println!("house drink: none"),
    }
    println!(
        "notes {:?} · staff {} · avg tick {:.1} µs",
        world.notes_found(),
        world.staff().len(),
        total_time * 1e6 / total_ticks.max(1) as f64,
    );
}
