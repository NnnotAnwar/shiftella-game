//! T9 balance locks (design/handson.md §9): short, deterministic player-bot slices of
//! `examples/sim_balance` with loose ranges, so a later content or sim change can't silently
//! push the evening out of shape. The full numbers come from
//! `cargo run --release --example sim_balance -- 30 story cozy 12`.

use super::*;
use crate::content::Content;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

fn content() -> Content {
    Content::load(DATA).expect("content")
}

/// What one bot run of a few nights adds up to.
#[derive(Default, Debug)]
struct Run {
    nights: u32,
    parties: u32,
    forgiven: u32,
    unhappy: u32,
    minigames: Vec<u32>,
    /// The night with the doors open (the quiet halves left out: `quiet_minutes`).
    real_minutes: Vec<f32>,
    quiet_minutes: Vec<f32>,
    peaks: Vec<u32>,
    medians: Vec<u32>,
    idle: u32,
    payments: u32,
    missed: u32,
}

fn play(w: &mut World, c: &Content, seed: u64, nights: u32, plan: Vec<u32>) -> Run {
    let mut b = bot::PlayerBot::new(bot::BotConfig {
        seed,
        hire_nights: plan,
        ..Default::default()
    });
    let mut r = Run::default();
    for _ in 0..nights {
        if w.is_closed() {
            break;
        }
        let ev = b.run_night(w, c);
        let s = w.last_summary().unwrap().clone();
        let bn = b.nights.last().unwrap();
        r.nights += 1;
        r.parties += ev
            .iter()
            .filter(|e| matches!(e, SimEvent::GuestArrived { .. }))
            .count() as u32;
        r.forgiven += ev
            .iter()
            .filter(|e| matches!(e, SimEvent::GuestForgiven { .. }))
            .count() as u32;
        r.unhappy += s.guests_left_unhappy;
        r.minigames.push(s.minigames);
        r.real_minutes.push(bn.real_minutes - bn.quiet_minutes);
        r.quiet_minutes.push(bn.quiet_minutes);
        r.peaks.push(s.todo_peak);
        r.medians.push(s.todo_median);
        r.idle += s.karolina_idle_minutes;
        r.payments += u32::from(s.payment > 0);
        r.missed += u32::from(s.payment_missed);
    }
    r
}

fn mean_u(v: &[u32]) -> f32 {
    v.iter().sum::<u32>() as f32 / v.len().max(1) as f32
}

fn mean_f(v: &[f32]) -> f32 {
    v.iter().sum::<f32>() / v.len().max(1) as f32
}

#[test]
fn story_act_one_on_cozy_is_a_calm_full_evening() {
    let c = content();
    for seed in [7932u64, 23770] {
        let mut w = World::new_story_seeded(&c, Difficulty::Cozy, seed);
        let r = play(&mut w, &c, seed, 10, vec![3, 5, 9]);
        assert_eq!(r.nights, 10);
        assert_eq!(r.unhappy, 0, "Cozy: nobody walks out angry");
        let forgiven = r.forgiven as f32 / r.parties.max(1) as f32;
        assert!(
            forgiven <= 0.12,
            "seed {seed}: forgiven {forgiven:.2} ({r:?})"
        );
        // mini-games: 6–14 a night on average, never a dry or a flooded night
        let mg = mean_u(&r.minigames);
        assert!((6.0..=14.0).contains(&mg), "seed {seed}: mini-games {r:?}");
        // (a night with very few wish guests is rare but real: ~1 in 120 nights has 2)
        assert!(
            r.minigames.iter().all(|n| (2..=16).contains(n)),
            "seed {seed}: {:?}",
            r.minigames
        );
        // 12–14 real minutes at ×1 with the doors open (a little slack per night); the quiet
        // halves (0.2.1: slower on purpose, ~1.6 min each at most; the bot opens and closes
        // up as soon as its lists are done) add about 1–3.5 minutes
        let quiet = mean_f(&r.quiet_minutes);
        assert!(
            (0.7..=3.6).contains(&quiet),
            "seed {seed}: quiet halves {quiet:.1} min"
        );
        let len = mean_f(&r.real_minutes);
        assert!((11.8..=14.2).contains(&len), "seed {seed}: {len:.1} min");
        assert!(
            r.real_minutes.iter().all(|m| (11.0..=15.0).contains(m)),
            "seed {seed}: {:?}",
            r.real_minutes
        );
        // pending things: a few at a time, rushes stay readable
        assert!(mean_u(&r.medians) <= 3.0, "seed {seed}: {:?}", r.medians);
        assert!(mean_u(&r.peaks) <= 8.5, "seed {seed}: peaks {:?}", r.peaks);
        assert!(
            r.peaks.iter().all(|p| *p <= 16),
            "seed {seed}: {:?}",
            r.peaks
        );
        let idle = r.idle as f32 / (r.nights * 420) as f32;
        assert!(idle <= 0.15, "seed {seed}: idle {idle:.2}");
        assert_eq!(r.missed, 0, "the first payment (night 7) is made on Cozy");
    }
}

#[test]
fn a_cozy_story_pays_off_grandmas_debt_with_room_to_spare() {
    let c = content();
    let seed = 15851;
    let mut w = World::new_story_seeded(&c, Difficulty::Cozy, seed);
    let r = play(&mut w, &c, seed, 30, vec![3, 5, 9]);
    assert_eq!((r.payments, r.missed), (5, 0), "{r:?}");
    assert_eq!(w.debt(), 0);
    assert!(
        w.money() >= 5_000,
        "money after the last payment {}",
        w.money()
    );
    assert_eq!(w.ending(), Some("light"));
    let forgiven = r.forgiven as f32 / r.parties.max(1) as f32;
    assert!(forgiven <= 0.12, "forgiven {forgiven:.2}");
    // (0.2.x fair timers: regulars no longer walk out of a wish at their window's end, so this
    // seed's Karolina makes ~1 more a night: 12.9 → 14.1; sim_balance's 12-seed mean 13.1 → 13.9)
    let mg = mean_u(&r.minigames);
    assert!((8.0..=14.5).contains(&mg), "mini-games {:?}", r.minigames);
    let len = mean_f(&r.real_minutes);
    assert!((12.0..=14.2).contains(&len), "{len:.1} min (doors open)");
    let quiet = mean_f(&r.quiet_minutes);
    assert!((0.7..=3.6).contains(&quiet), "quiet halves {quiet:.1} min");
    // a kind player befriends the station cat over a story (naming at 10)
    let cat = w
        .story()
        .counters
        .get("cat_affection")
        .copied()
        .unwrap_or(0);
    assert!(cat >= 8, "cat affection {cat}");
}

#[test]
fn classic_story_is_a_fair_challenge() {
    // the regular payments are made; the 30 000 final one is the real test (about half the
    // bot's runs make it — `sim_balance 30 story classic 12`), so it isn't locked here
    let c = content();
    let seed = 7932;
    let mut w = World::new_story_seeded(&c, Difficulty::Classic, seed);
    let r = play(&mut w, &c, seed, 29, vec![3, 8, 15]);
    assert_eq!(r.nights, 29, "the café stays open until the last night");
    assert_eq!((r.payments, r.missed), (4, 0), "{r:?}");
    // some guests walk out on Classic, but not a crowd
    let arrived: u32 = w.summaries().iter().map(|s| s.guests_arrived).sum();
    let share = r.unhappy as f32 / arrived.max(1) as f32;
    assert!((0.02..=0.2).contains(&share), "left unhappy {share:.2}");
    // within reach of the final payment, not far past it
    assert!(
        (12_000..=60_000).contains(&w.money()),
        "money before the final payment {}",
        w.money()
    );
}

#[test]
fn sandbox_money_grows_without_running_away() {
    let c = content();
    let seed = 23770;
    let mut w = World::new_sandbox(
        &c,
        SandboxSetup {
            seed,
            difficulty: Difficulty::Cozy,
            ..Default::default()
        },
    );
    let start = w.money();
    let r = play(&mut w, &c, seed, 30, vec![3, 5, 9]);
    assert_eq!(r.nights, 30);
    assert!(
        w.money() > start && w.money() < start + 90_000,
        "{start} → {}",
        w.money()
    );
}
