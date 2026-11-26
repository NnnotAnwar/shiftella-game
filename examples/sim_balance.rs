//! Balance + QA sweep: plays several seeds headless with the player-bot (`sim::bot`) and
//! prints per-night averages, the design/handson.md §9 targets and sanity checks (stuck
//! agents, NaN, events, regular arcs, cameo nights, endings).
//!
//! ```text
//! cargo run --release --example sim_balance -- [nights] [story|sandbox] [cozy|classic] [seeds]
//!     [--player-bot [quality] [react_seconds]] [--no-bot]
//! SIM_PLAN=3,5,9   # nights on which the bot hires one more helper (default 3,5,9; Classic 3,8,15)
//! SIM_VISITS=1     # print each regular's last visits (night:served minutes) and trust
//! ```
//! `--player-bot` is the default; `--no-bot` = nobody behind the counter (Karolina steps back).
//! `--inattentive`: the bot leaves every story choice on its default and builds nothing for
//! the regulars (before T10 it always played like that).

use std::collections::BTreeMap;

use shiftella_game::content::Content;
use shiftella_game::sim::bot::{BotConfig, PlayerBot, finish_cutscenes};
use shiftella_game::sim::{Difficulty, Mode, SandboxSetup, SimEvent, World};

#[derive(Default, Clone)]
struct Row {
    arrived: f32,
    served: f32,
    forgiven: f32,
    left: f32,
    parties: f32,
    forgiven_parties: f32,
    k_items: f32,
    h_items: f32,
    reg_items: f32,
    reg_k: f32,
    wishes: f32,
    minigames: f32,
    same_kind: f32,
    idle: f32,
    todo_med: f32,
    todo_peak: f32,
    revenue: f32,
    tips: f32,
    money: f32,
    rep: f32,
    sat: f32,
    real_min: f32,
    crew: f32,
    overlay_min: f32,
    turned: f32,
    wages: f32,
    supplies: f32,
    n: f32,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let flag_at = args
        .iter()
        .position(|a| a.starts_with("--"))
        .unwrap_or(args.len());
    let pos: Vec<&String> = args.iter().take(flag_at).skip(1).collect();
    let nights: u32 = pos.first().and_then(|s| s.parse().ok()).unwrap_or(10);
    let mode = pos.get(1).map(|s| s.as_str()).unwrap_or("story");
    let difficulty = match pos.get(2).map(|s| s.as_str()) {
        Some("classic") => Difficulty::Classic,
        _ => Difficulty::Cozy,
    };
    let seeds: u64 = pos.get(3).and_then(|s| s.parse().ok()).unwrap_or(6);
    let no_bot = args.iter().any(|a| a == "--no-bot");
    let thrifty = args.iter().any(|a| a == "--thrifty");
    let attentive = !args.iter().any(|a| a == "--inattentive");
    let (quality, react) = match args.iter().position(|a| a == "--player-bot") {
        Some(i) => (
            args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(0.75),
            args.get(i + 2).and_then(|s| s.parse().ok()).unwrap_or(3.0),
        ),
        None => (0.75, 3.0),
    };
    let plan: Vec<u32> = std::env::var("SIM_PLAN")
        .ok()
        .map(|s| s.split(',').filter_map(|x| x.trim().parse().ok()).collect())
        .unwrap_or_else(|| {
            if difficulty == Difficulty::Classic {
                vec![3, 8, 15]
            } else {
                vec![3, 5, 9]
            }
        });
    let content = Content::load("assets/data").expect("content");

    let mut rows: Vec<Row> = vec![Row::default(); nights as usize + 1];
    let mut event_nights: BTreeMap<String, u32> = BTreeMap::new();
    let mut nights_played = 0u32;
    let mut endings: BTreeMap<String, u32> = BTreeMap::new();
    let mut problems: Vec<String> = Vec::new();
    let mut arcs: BTreeMap<String, Vec<i32>> = BTreeMap::new();
    let mut anwar_nights: Vec<u32> = Vec::new();
    let mut breaks = 0u32;
    let mut helper_nights = 0u32;
    let mut stuck = 0u32;
    let mut unhappy_cozy = 0u32;
    // T5 systems: counts over every played night
    let mut t5: BTreeMap<&'static str, f32> = BTreeMap::new();
    let mut moments_by_night: Vec<u32> = Vec::new();
    let mut two_open_early = 0u32;
    let mut journal_ok = 0u32;
    // T9: payments, the cat, money at the end, peaks
    let (mut paid, mut missed) = (0u32, 0u32);
    let mut cat_aff: Vec<i32> = Vec::new();
    let mut money_end: Vec<i64> = Vec::new();
    let mut lights = 0u32;
    let mut peaks: Vec<u32> = Vec::new();
    let mut missed_runs: Vec<String> = Vec::new();

    for seed in 1..=seeds {
        let seed = seed * 7919 + 13;
        let mut missed_nights: Vec<u32> = Vec::new();
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
        let mut bot = PlayerBot::new(BotConfig {
            quality,
            react_seconds: react,
            hire_nights: plan.clone(),
            seed,
            thrifty,
            attentive,
            ..Default::default()
        });
        for _ in 0..nights {
            if world.is_closed() {
                break;
            }
            let night = world.night();
            let events = if no_bot {
                finish_cutscenes(&mut world, &content);
                world.run_night_headless(&content)
            } else {
                bot.run_night(&mut world, &content)
            };
            let mut fired: std::collections::BTreeSet<String> = Default::default();
            let (mut parties, mut forgiven) = (0u32, 0u32);
            let (mut open, mut budgeted) = (0i32, 0u32);
            for e in events {
                match &e {
                    SimEvent::MomentAvailable { id, .. } => {
                        open += 1;
                        if id != "m_leak_empty" {
                            budgeted += 1;
                        }
                        if open >= 2 && night <= 3 && world.mode() == Mode::Story {
                            two_open_early += 1;
                        }
                    }
                    SimEvent::MomentExpired { .. } => open -= 1,
                    SimEvent::MomentResolved { .. } => {
                        open -= 1;
                        *t5.entry("moments resolved").or_default() += 1.0;
                    }
                    SimEvent::ExperimentDone { .. } => *t5.entry("experiments").or_default() += 1.0,
                    SimEvent::RecipeLearned { source, .. } if source == "experiment" => {
                        *t5.entry("recipes discovered").or_default() += 1.0
                    }
                    SimEvent::MarginNote { .. } => *t5.entry("margin notes").or_default() += 1.0,
                    SimEvent::ThankYouNote { .. } => *t5.entry("napkins").or_default() += 1.0,
                    SimEvent::CatLevel { .. } => *t5.entry("cat levels").or_default() += 1.0,
                    SimEvent::TipGoalReached { .. } => *t5.entry("tip goals").or_default() += 1.0,
                    SimEvent::ThemeChanged { .. } => {
                        *t5.entry("theme switches").or_default() += 1.0
                    }
                    SimEvent::SeasonPhaseChanged { .. } => {
                        *t5.entry("season changes").or_default() += 1.0
                    }
                    SimEvent::HighlightCaptured { .. } => {
                        *t5.entry("highlights").or_default() += 1.0
                    }
                    SimEvent::RozpisHappy => *t5.entry("happy weeks").or_default() += 1.0,
                    SimEvent::RequestCreated { .. } => *t5.entry("requests").or_default() += 1.0,
                    SimEvent::RitualComplete { .. } => *t5.entry("rituals").or_default() += 1.0,
                    SimEvent::HandoverApplied { .. } => *t5.entry("handovers").or_default() += 1.0,
                    SimEvent::PayrollSent { .. } => *t5.entry("payrolls").or_default() += 1.0,
                    SimEvent::PrefViolated { .. } => *t5.entry("prefs broken").or_default() += 1.0,
                    _ => {}
                }
                match e {
                    SimEvent::StaffBreak { .. } => breaks += 1,
                    SimEvent::EventStarted { id } => {
                        fired.insert(id);
                    }
                    SimEvent::GuestArrived { .. } => parties += 1,
                    SimEvent::GuestForgiven { .. } => forgiven += 1,
                    SimEvent::RegularArrived { regular, .. } if regular == "guest_anwar" => {
                        anwar_nights.push(night);
                    }
                    _ => {}
                }
            }
            for id in fired {
                *event_nights.entry(id).or_default() += 1;
            }
            moments_by_night.push(budgeted);
            *t5.entry("bot buttons").or_default() +=
                bot.nights.last().map(|n| n.buttons).unwrap_or(0) as f32;
            if let Some(s) = world.last_summary() {
                *t5.entry("coziness avg").or_default() += s.coziness_avg;
                *t5.entry("insights").or_default() += s.insights.len() as f32;
            }
            *t5.entry("fairness").or_default() += world.week_fairness();
            helper_nights += world
                .staff()
                .iter()
                .filter(|s| s.minutes_worked > 0 && !s.owner)
                .count() as u32;
            nights_played += 1;
            for s in world.staff() {
                if !(s.energy.is_finite() && s.mood.is_finite() && s.hunger.is_finite()) {
                    problems.push(format!("seed {seed} night {night}: NaN staff"));
                }
            }
            let Some(s) = world.last_summary().cloned() else {
                continue;
            };
            let bn = bot.nights.last().cloned().unwrap_or_default();
            let r = &mut rows[s.night.min(nights) as usize];
            r.arrived += s.guests_arrived as f32;
            r.served += s.guests_served as f32;
            r.forgiven += s.guests_forgiven as f32;
            r.left += s.guests_left_unhappy as f32;
            r.parties += parties as f32;
            r.forgiven_parties += forgiven as f32;
            r.k_items += s.karolina_items as f32;
            r.h_items += s.helper_items as f32;
            r.reg_items += s.regular_items as f32;
            r.reg_k += s.regular_items_karolina as f32;
            r.wishes += s.wishes_served as f32;
            r.minigames += s.minigames as f32;
            r.same_kind += bn.same_kind_max as f32;
            r.idle += s.karolina_idle_minutes as f32 / 420.0 * 100.0;
            r.todo_med += s.todo_median as f32;
            r.todo_peak += s.todo_peak as f32;
            r.revenue += s.revenue as f32;
            r.tips += s.tips as f32;
            r.money += s.money_end as f32;
            r.rep += s.reputation_end;
            r.sat += s.avg_satisfaction;
            r.real_min += if no_bot { 12.0 } else { bn.real_minutes };
            r.crew += world.staff().len() as f32;
            r.overlay_min += bn.overlay_seconds / 60.0;
            r.turned += s.guests_turned_away as f32;
            r.wages += s.wages as f32;
            r.supplies += s.supplies as f32;
            r.n += 1.0;
            peaks.push(s.todo_peak);
            if s.payment > 0 {
                paid += 1;
            }
            if s.payment_missed {
                missed += 1;
                missed_nights.push(s.night);
            }
            stuck += s.stuck_resets;
            if difficulty == Difficulty::Cozy {
                unhappy_cozy += s.guests_left_unhappy;
            }
            if !s.avg_satisfaction.is_finite() || !s.reputation_end.is_finite() {
                problems.push(format!("seed {seed} night {night}: NaN summary"));
            }
            if s.money_end < -50_000 {
                problems.push(format!("seed {seed} night {night}: money {}", s.money_end));
            }
        }
        // every played night left one kept polaroid (the last one may still wait for a pick)
        let kept = world.journal().len() as u32 + u32::from(world.pending_polaroids().is_some());
        if kept >= nights.min(world.night()) {
            journal_ok += 1;
        }
        *t5.entry("themes owned").or_default() += content
            .handson
            .decor
            .themes
            .iter()
            .filter(|t| world.theme_owned(&content, &t.id))
            .count() as f32;
        *t5.entry("recipes known").or_default() += world.known_recipes().count() as f32;
        *t5.entry("cat level").or_default() += world.cat_level() as f32;
        cat_aff.push(
            world
                .story()
                .counters
                .get("cat_affection")
                .copied()
                .unwrap_or(0),
        );
        money_end.push(world.money());
        if std::env::var("SIM_TOWN").is_ok() {
            println!(
                "  seed {seed}: town size {} rent {} institutions {:?} money {}",
                world.town().size,
                world.rent(),
                world.town().institutions,
                world.money()
            );
        }
        if !missed_nights.is_empty() {
            missed_runs.push(format!("{missed_nights:?}"));
        }
        if world.ending() == Some("light") {
            lights += 1;
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
        "{mode} · {difficulty:?} · {seeds} seeds · {} · hires on {plan:?}",
        if no_bot {
            "nobody behind the counter".to_string()
        } else {
            format!("player-bot q {quality} react {react}s")
        }
    );
    println!(
        "night arrived served forgv left | K-made% reg-K% wish   mg same | idle% todo med/pk | revenue   tips |   money   rep  sat | real-min crew"
    );
    let mut sums = Row::default();
    for (n, r) in rows.iter().enumerate().skip(1) {
        if r.n == 0.0 {
            continue;
        }
        let k = r.n;
        let made = (r.k_items + r.h_items).max(1.0);
        println!(
            "{n:>5} {:>7.1} {:>6.1} {:>5.1} {:>4.1} | {:>6.0}% {:>5.0}% {:>4.1} {:>4.1} {:>4.1} | {:>5.1} {:>4.1}/{:<4.1} | {:>7.0} {:>6.0} | {:>7.0} {:>5.1} {:>4.0} | {:>8.1} {:>4.1}",
            r.arrived / k,
            r.served / k,
            r.forgiven / k,
            r.left / k,
            r.k_items / made * 100.0,
            r.reg_k / r.reg_items.max(1.0) * 100.0,
            r.wishes / k,
            r.minigames / k,
            r.same_kind / k,
            r.idle / k,
            r.todo_med / k,
            r.todo_peak / k,
            r.revenue / k,
            r.tips / k,
            r.money / k,
            r.rep / k,
            r.sat / k,
            r.real_min / k,
            r.crew / k,
        );
        sums.parties += r.parties;
        sums.forgiven_parties += r.forgiven_parties;
        sums.idle += r.idle;
        sums.todo_med += r.todo_med;
        sums.todo_peak = sums.todo_peak.max(r.todo_peak / k);
        sums.n += r.n;
        sums.minigames += r.minigames;
        sums.real_min += r.real_min;
    }
    let k = sums.n.max(1.0);
    println!(
        "§9: idle {:.1}% (≤ 15) · todo median {:.1} (1–3), worst avg peak {:.1} (≤ 7) · forgiven parties {:.1}% (≤ 10) · unhappy on Cozy {} (0) · mini-games {:.1}/night (6–14) · night {:.1} real min (12 ±1 + 1–2 overlays)",
        sums.idle / k,
        sums.todo_med / k,
        sums.todo_peak,
        sums.forgiven_parties / sums.parties.max(1.0) * 100.0,
        unhappy_cozy,
        sums.minigames / k,
        sums.real_min / k,
    );
    let n = nights_played.max(1) as f32;
    let s = seeds as f32;
    let per_night = [
        "moments resolved",
        "experiments",
        "highlights",
        "insights",
        "bot buttons",
        "coziness avg",
        "fairness",
        "rituals",
        "prefs broken",
    ];
    let mut line = Vec::new();
    for (k, v) in &t5 {
        if per_night.contains(k) {
            line.push(format!("{k} {:.1}/night", v / n));
        } else if ["themes owned", "recipes known", "cat level"].contains(k) {
            line.push(format!("{k} {:.1}", v / s));
        } else {
            line.push(format!("{k} {:.1}/run", v / s));
        }
    }
    let mut mb = moments_by_night.clone();
    mb.sort();
    let mean = mb.iter().sum::<u32>() as f32 / mb.len().max(1) as f32;
    println!(
        "moments/night: mean {mean:.1}, min {}, max {} · two open at once on nights 1–3: {two_open_early} · journal complete {journal_ok}/{seeds}",
        mb.first().copied().unwrap_or(0),
        mb.last().copied().unwrap_or(0)
    );
    println!("T5: {}", line.join(" · "));
    peaks.sort();
    let pct = |v: &[u32], q: f32| {
        v.get(((v.len() as f32 - 1.0) * q) as usize)
            .copied()
            .unwrap_or(0)
    };
    let turned: f32 = rows.iter().map(|r| r.turned).sum();
    let overlay: f32 = rows.iter().map(|r| r.overlay_min).sum();
    let wages: f32 = rows.iter().map(|r| r.wages).sum();
    let supplies: f32 = rows.iter().map(|r| r.supplies).sum();
    money_end.sort();
    println!(
        "T9: todo peak p50 {} p90 {} max {} · overlays {:.1} real min/night · turned away {:.1}/night · wages {:.0} supplies {:.0}/night · payments made {paid} missed {missed} · light endings {lights}/{seeds} · missed payments by run {missed_runs:?} · money at the end {money_end:?} · cat affection {cat_aff:?}",
        pct(&peaks, 0.5),
        pct(&peaks, 0.9),
        peaks.last().copied().unwrap_or(0),
        overlay / k,
        turned / k,
        wages / k,
        supplies / k,
    );
    println!("endings: {endings:?}");
    println!(
        "breaks per helper-night {:.2} · stuck resets {stuck}",
        breaks as f32 / helper_nights.max(1) as f32
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
