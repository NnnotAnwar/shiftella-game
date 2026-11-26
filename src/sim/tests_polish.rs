//! 0.2 polish (design/v0.2.md G): the Cozy rush peak. On Cozy a crowd (train passengers, an
//! event rush, a walk-in while she is swamped) lingers outside while Karolina's hands are full
//! (`guests::COZY_HOLD_*`), so her pending things stay readable after a train; Classic keeps
//! its walls. A Cozy train no longer brings passengers in after last call. The full numbers: `cargo run --release --example sim_balance -- 10 story cozy 6
//! --player-bot` (worst avg peak ≤ 7).

use super::guests::{COZY_HOLD_LAST, COZY_HOLD_MAX};
use super::*;
use crate::content::Content;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

fn content() -> Content {
    Content::load(DATA).expect("content")
}

fn bot(seed: u64) -> bot::PlayerBot {
    bot::PlayerBot::new(bot::BotConfig {
        seed,
        hire_nights: vec![3, 5, 9],
        ..Default::default()
    })
}

#[test]
fn the_cozy_rush_peak_stays_readable() {
    let c = content();
    // the sim_balance seeds 1–12 (`seed * 7919 + 13`); nights 1–8 are the busiest stretch for
    // Karolina alone or with one or two helpers (the worst average peak was 8.6–10.2 there)
    let seeds: Vec<u64> = (1..=12u64).map(|s| s * 7919 + 13).collect();
    let nights = 8;
    let mut by_night = vec![0u32; nights];
    for &seed in &seeds {
        let mut w = World::new_story_seeded(&c, Difficulty::Cozy, seed);
        let mut b = bot(seed);
        for slot in by_night.iter_mut() {
            b.run_night(&mut w, &c);
            let s = w.last_summary().unwrap();
            assert!(
                s.todo_peak <= 16,
                "seed {seed} night {}: {}",
                s.night,
                s.todo_peak
            );
            *slot += s.todo_peak;
        }
    }
    let avg: Vec<f32> = by_night
        .iter()
        .map(|p| *p as f32 / seeds.len() as f32)
        .collect();
    let worst = avg.iter().copied().fold(0.0, f32::max);
    assert!(worst <= 7.0, "worst average peak {worst:.2}: {avg:?}");
}

/// Plays a night tick by tick and returns the most minutes any pending party lingered, and
/// whether every crowd came in before the doors closed.
fn linger_night(w: &mut World, c: &Content, b: &mut bot::PlayerBot) -> (u32, bool) {
    if w.phase() == Phase::Day {
        b.day(w, c);
        b.finish_scenes(w, c);
        w.drain_events();
        w.start_night(c);
    }
    let mut most = 0;
    let mut late = false;
    while w.phase() == Phase::Night && !w.is_closed() {
        b.finish_scenes(w, c);
        b.step(w, c);
        w.tick(c);
        w.drain_events();
        for s in &w.spawns {
            most = most.max(s.held);
            late |= s.held > 0 && s.minute > COZY_HOLD_LAST;
        }
    }
    (most, !late)
}

#[test]
fn a_cozy_crowd_lingers_a_little_and_still_comes_in() {
    let c = content();
    let seed = 23770;
    let mut w = World::new_story_seeded(&c, Difficulty::Cozy, seed);
    let mut b = bot(seed);
    let mut lingered = 0;
    for _ in 0..4 {
        let (most, in_time) = linger_night(&mut w, &c, &mut b);
        // (the ones who waited come in a party a minute: a few minutes past the cap at most)
        assert!(most <= COZY_HOLD_MAX + 20, "lingered {most} min");
        assert!(in_time, "a lingering party was still outside after 05:20");
        lingered = lingered.max(most);
    }
    assert!(lingered > 0, "a busy Cozy night makes a crowd wait outside");
}

#[test]
fn classic_crowds_never_linger() {
    let c = content();
    let seed = 23770;
    let mut w = World::new_story_seeded(&c, Difficulty::Classic, seed);
    let mut b = bot(seed);
    for _ in 0..4 {
        let (most, _) = linger_night(&mut w, &c, &mut b);
        assert_eq!(most, 0, "Classic: the passengers walk straight in");
    }
}

#[test]
fn old_pending_spawns_load_without_the_new_fields() {
    let s: guests::PendingSpawn = ron::from_str(
        "(minute: 40, archetype: \"travelers\", size: 2, patience_mul: 1.0, spend_mul: 1.0, orders: [])",
    )
    .unwrap();
    assert!(!s.crowd && s.held == 0 && !s.wish);
}
