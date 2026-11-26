//! 0.2 package J (access): the Carefree difficulty (`relaxed`) — Cozy with no money pressure.

use super::*;
use crate::content::{Content, PaymentRule};

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

fn content() -> Content {
    Content::load(DATA).unwrap_or_else(|e| panic!("{e}"))
}

/// The whole story with the attentive player-bot (as in `tests_t10`); returns the world and
/// every payment event on the way.
fn play_story(
    c: &Content,
    seed: u64,
    difficulty: Difficulty,
    broke: bool,
) -> (World, Vec<SimEvent>) {
    let mut w = World::new_story_seeded(c, difficulty, seed);
    let mut b = bot::PlayerBot::new(bot::BotConfig {
        seed,
        ..Default::default()
    });
    let mut pays = Vec::new();
    for _ in 1..=30 {
        if w.is_closed() {
            break;
        }
        if w.phase() == Phase::Day {
            b.day(&mut w, c);
            answer(&mut w, c);
            w.start_night(c);
        }
        let mut guard = 0;
        while w.phase() == Phase::Night && !w.is_closed() && guard < 100_000 {
            answer(&mut w, c);
            b.step(&mut w, c);
            // a player who never keeps money aside: the till is empty at every payment
            if broke {
                w.econ.money = w.econ.money.min(0);
            }
            w.tick(c);
            for e in w.drain_events() {
                if matches!(
                    e,
                    SimEvent::PaymentMade { .. } | SimEvent::PaymentMissed { .. }
                ) {
                    pays.push(e);
                }
            }
            guard += 1;
        }
        answer(&mut w, c);
    }
    (w, pays)
}

fn answer(w: &mut World, c: &Content) {
    for _ in 0..64 {
        let Some(id) = w.playing_cutscene().map(str::to_string) else {
            return;
        };
        let choices = bot::attentive_choices(w, c, &id);
        w.apply(c, Command::CutsceneFinished { id, choices })
            .unwrap();
    }
}

#[test]
fn carefree_is_a_preset_that_forgives_and_is_otherwise_cozy() {
    let c = content();
    let d = c.difficulty("relaxed").expect("the relaxed preset");
    let cozy = c.difficulty("cozy").unwrap();
    assert_eq!(d.payments, PaymentRule::Forgiven);
    assert!(!d.default, "Cozy stays the default");
    assert!(d.close_after_missed.is_none() && d.bankrupt_after.is_none());
    assert_eq!(d.start_money, cozy.start_money);
    assert_eq!(d.guest_patience_mul, cozy.guest_patience_mul);
    assert_eq!(d.income_mul, cozy.income_mul);
    assert!(Difficulty::Relaxed.gentle() && Difficulty::Cozy.gentle());
    assert!(!Difficulty::Classic.gentle());
    assert_eq!(Difficulty::Relaxed.id(), "relaxed");
    let w = World::new_story_seeded(&c, Difficulty::Relaxed, 3);
    let z = World::new_story_seeded(&c, Difficulty::Cozy, 3);
    assert_eq!(w.overlay_scale(&c), z.overlay_scale(&c));
    assert_eq!(
        w.debt(),
        z.debt(),
        "the story still tells of grandma's debt"
    );
}

#[test]
fn a_payment_night_covers_the_instalment_even_with_an_empty_till() {
    let c = content();
    let p = c.story.payments.first().expect("a payment").clone();
    let mut w = World::new_story_seeded(&c, Difficulty::Relaxed, 8);
    w.night = p.night;
    w.econ.money = 0;
    let debt = w.debt();
    economy::collect_payment(&mut w, &c);
    let ev = w.drain_events();
    assert!(
        !ev.iter()
            .any(|e| matches!(e, SimEvent::PaymentMissed { .. })),
        "nothing can be missed"
    );
    assert_eq!(w.money(), 0, "the till keeps its money");
    assert_eq!(w.debt(), debt - p.amount, "the debt shrinks");
    assert!(w.story().payments.iter().any(|x| x.0 == p.night && x.2));
    assert!(!w.tonight().payment_missed);
    assert!(logic::payments_all_made(&w));
    // the same night on Cozy with an empty till grumbles
    let mut z = World::new_story_seeded(&c, Difficulty::Cozy, 8);
    z.night = p.night;
    z.econ.money = 0;
    economy::collect_payment(&mut z, &c);
    assert!(z.tonight().payment_missed);
}

#[test]
fn a_carefree_story_reaches_the_light_in_the_window_without_paying() {
    let c = content();
    // a player whose till is always empty: on Carefree nothing is ever missed
    let (w, pays) = play_story(&c, 4242, Difficulty::Relaxed, true);
    assert!(pays.is_empty(), "no payment taken or missed: {pays:?}");
    assert_eq!(w.ending(), Some("light"), "Light in the Window");
    assert_eq!(w.debt(), 0, "the debt is gone by the story's end");
    assert!(
        w.story().payments.iter().all(|x| x.2),
        "every instalment covered"
    );
    assert_eq!(
        w.story()
            .counters
            .get("missed_payments")
            .copied()
            .unwrap_or(0),
        0
    );
}

#[test]
fn an_old_save_without_the_variant_still_loads_as_cozy() {
    // saves store the difficulty by variant name; the new variant is at the end
    let back: Difficulty = ron::from_str("Cozy").unwrap();
    assert_eq!(back, Difficulty::Cozy);
    let r: Difficulty = ron::from_str(&ron::to_string(&Difficulty::Relaxed).unwrap()).unwrap();
    assert_eq!(r, Difficulty::Relaxed);
}
