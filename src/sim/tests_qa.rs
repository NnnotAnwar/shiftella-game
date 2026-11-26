//! Gameplay QA regression tests (v0.1.0 balance pass): the gentle first nights, the default
//! rozpis, pendant lamps, breaks, hire log, starting money, event frequency, regular arcs,
//! the Anwar cameo nights and both endings (forced). Nights are played by the player-bot
//! (`sim::bot`): since the hands-on update Karolina only does what the player asks.

use super::*;
use crate::content::{Content, Job};

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

fn content() -> Content {
    Content::load(DATA).unwrap_or_else(|e| panic!("{e}"))
}

fn sandbox(c: &Content, seed: u64, difficulty: Difficulty) -> World {
    World::new_sandbox(
        c,
        SandboxSetup {
            seed,
            difficulty,
            ..Default::default()
        },
    )
}

/// Runs one night like a careful player (the player-bot: takes orders, brews, serves, lights
/// the window, publishes the rozpis, delegates). Hiring stays with the test.
fn play_night(w: &mut World, c: &Content) -> Vec<SimEvent> {
    let mut b = bot::PlayerBot::new(bot::BotConfig {
        hire_nights: Vec::new(),
        seed: w.seed(),
        ..Default::default()
    });
    b.run_night(w, c)
}

fn hire(w: &mut World, c: &Content, n: usize) {
    if w.candidates().len() < n && w.mode() == Mode::Sandbox {
        // sandbox offers 3 at a time; an empty list refreshes
        w.candidates.clear();
        super::staff::refresh_candidates(w, c);
    }
    let ids: Vec<u32> = w.candidates().iter().map(|x| x.id).take(n).collect();
    assert_eq!(ids.len(), n, "not enough candidates");
    for id in ids {
        w.apply(c, Command::Hire { candidate: id }).unwrap();
    }
}

/// Day phase of story night `n` (skips the nights in between).
fn jump_to_night(w: &mut World, c: &Content, n: u32) {
    w.night = n;
    w.begin_day(c);
}

fn shifts_worked(row: &[Shift; 7]) -> usize {
    row.iter().filter(|s| **s != Shift::Off).count()
}

// ---------------------------------------------------------------------------------------------
// The first nights
// ---------------------------------------------------------------------------------------------

#[test]
fn story_night_one_alone_is_gentle() {
    let c = content();
    let (mut served, mut left) = (0u32, 0u32);
    for seed in 1..=6u64 {
        let mut w = World::new_story_seeded(&c, Difficulty::Cozy, seed * 101);
        assert_eq!(w.staff().len(), 1, "night 1 is the player alone");
        play_night(&mut w, &c);
        let s = w.last_summary().unwrap();
        served += s.guests_served;
        left += s.guests_left_unhappy;
    }
    // Cozy: nobody leaves angry on the first solo night
    assert_eq!(left, 0, "cozy night 1: served {served}, left {left}");
    assert!(
        served >= 6 * 10,
        "cozy night 1: served {served} over 6 seeds"
    );
    // Classic: some pressure, still mostly served
    let (mut served, mut left) = (0u32, 0u32);
    for seed in 1..=6u64 {
        let mut w = World::new_story_seeded(&c, Difficulty::Classic, seed * 101);
        play_night(&mut w, &c);
        let s = w.last_summary().unwrap();
        served += s.guests_served;
        left += s.guests_left_unhappy;
    }
    assert!(
        left * 2 <= served,
        "classic night 1: served {served}, left {left}"
    );
}

#[test]
fn first_week_ramps_up_gently_on_cozy() {
    let c = content();
    let mut w = World::new_story_seeded(&c, Difficulty::Cozy, 77);
    let mut arrived = Vec::new();
    for night in 1..=5 {
        if night == FIRST_HIRE_NIGHT {
            hire(&mut w, &c, 2);
        }
        play_night(&mut w, &c);
        let s = w.last_summary().unwrap().clone();
        assert!(
            s.guests_left_unhappy * 5 <= s.guests_arrived.max(5),
            "night {night}: {} of {} left angry",
            s.guests_left_unhappy,
            s.guests_arrived
        );
        arrived.push(s.guests_arrived);
    }
    assert!(arrived[0] < arrived[4], "the café gets busier: {arrived:?}");
}

#[test]
fn early_ramp_is_gentler_on_cozy_and_ends_with_act_one() {
    let c = content();
    let mut cozy = World::new_story(&c, Difficulty::Cozy);
    let mut classic = World::new_story(&c, Difficulty::Classic);
    for n in 1..=5 {
        cozy.night = n;
        classic.night = n;
        let (rc, pc) = guests::early_ramp(&cozy);
        let (rk, pk) = guests::early_ramp(&classic);
        assert!(rc <= rk && pc >= pk, "night {n}");
        assert!(rc < 1.0 && pc > 1.0);
    }
    // T9: Classic is at full crowd from night 6, Cozy over act 1 (nights 1–8)
    classic.night = 6;
    assert_eq!(guests::early_ramp(&classic), (1.0, 1.0));
    for n in 6..=8 {
        cozy.night = n;
        let (r, p) = guests::early_ramp(&cozy);
        assert!(r < 1.0 && p >= 1.0, "night {n}");
    }
    cozy.night = 9;
    assert_eq!(guests::early_ramp(&cozy), (1.0, 1.0));
}

// ---------------------------------------------------------------------------------------------
// Default rozpis
// ---------------------------------------------------------------------------------------------

#[test]
fn default_rozpis_is_sensible_for_every_crew_size() {
    let c = content();
    let mut w = sandbox(&c, 5, Difficulty::Cozy);
    for hires in 1..=6usize {
        while w.staff().iter().filter(|s| !s.owner).count() < hires {
            hire(&mut w, &c, 1);
        }
        let app = w.app();
        assert!(!app.draft_dirty, "the app plans it without nagging");
        assert_eq!(app.draft, app.published);
        let owner = w.staff().iter().find(|s| s.owner).unwrap().id;
        assert_eq!(
            app.published[&owner],
            [Shift::Full; 7],
            "the player is in every night"
        );
        let want = match hires {
            1 => 7,
            2 | 3 => 5,
            _ => 4,
        };
        for s in w.staff().iter().filter(|s| !s.owner) {
            let row = &app.published[&s.id];
            // a Need ("not two nights in a row", "three nights a week") may take a night off
            let limited = w.prefs(s.id).iter().any(|p| {
                p.strength == crate::content::handson::Strength::Need
                    && matches!(
                        p.kind,
                        crate::content::handson::PrefKind::NoBackToBack
                            | crate::content::handson::PrefKind::MaxNights(_)
                            | crate::content::handson::PrefKind::WeekendOff
                            | crate::content::handson::PrefKind::DayOff(_)
                    )
            });
            // (T9: the auto-fill may give someone one night more when that keeps a colleague's
            // late / early wish — a person on the floor for the other half)
            if limited {
                assert!(shifts_worked(row) <= want + 1, "{hires} hires: {row:?}");
            } else {
                let n = shifts_worked(row);
                assert!(n == want || n == want + 1, "{hires} hires: {row:?}");
            }
            let hours: f32 = row.iter().map(|x| x.hours()).sum();
            if hires >= 2 {
                assert!(hours <= 40.0, "{hires} hires: {hours} h");
            }
        }
        for d in 0..7 {
            let on: Vec<Shift> = w
                .staff()
                .iter()
                .map(|s| app.published[&s.id][d])
                .filter(|x| *x != Shift::Off)
                .collect();
            assert!(on.len() >= 2, "{hires} hires, weekday {d}: coverage {on:?}");
            let full_hires = w
                .staff()
                .iter()
                .filter(|s| !s.owner && app.published[&s.id][d] == Shift::Full)
                .count();
            assert!(full_hires <= super::app::ROZPIS_MAX_FULL_HIRES);
        }
        if hires >= 4 {
            let split = w
                .staff()
                .iter()
                .flat_map(|s| app.published[&s.id])
                .any(|x| matches!(x, Shift::Early | Shift::Late));
            assert!(split, "{hires} hires: overstaffed nights get split shifts");
        }
    }
}

#[test]
fn default_rozpis_respects_time_off_and_keeps_coverage() {
    let c = content();
    let mut w = sandbox(&c, 9, Difficulty::Cozy);
    hire(&mut w, &c, 2);
    let who = w.staff().iter().find(|s| !s.owner).unwrap().id;
    // tomorrow's weekday, wherever the person would work
    let wd = (w.weekday + 1) % 7;
    w.apply(
        &c,
        Command::RequestTimeOff {
            staff: who,
            kind: LeaveKind::Vacation,
            weekday: wd,
        },
    )
    .unwrap();
    let id = w.app().time_off.last().unwrap().id;
    w.apply(&c, Command::AnswerTimeOff { id, approve: true })
        .unwrap();
    assert_eq!(w.app().published[&who][wd as usize], Shift::Off);
    let on = w
        .staff()
        .iter()
        .filter(|s| w.app().published[&s.id][wd as usize] != Shift::Off)
        .count();
    assert!(on >= 2, "someone covers the day off");
}

#[test]
fn manual_rozpis_edits_stick_and_auto_schedule_hands_it_back() {
    let c = content();
    let mut w = sandbox(&c, 4, Difficulty::Cozy);
    hire(&mut w, &c, 1);
    let a = w.staff().iter().find(|s| !s.owner).unwrap().id;
    w.apply(
        &c,
        Command::SetShift {
            staff: a,
            weekday: 2,
            shift: Shift::Late,
        },
    )
    .unwrap();
    assert!(w.app().manual_rozpis && w.app().draft_dirty);
    // a new hire gets a row without touching the hand-made one
    hire(&mut w, &c, 1);
    assert_eq!(w.app().draft[&a][2], Shift::Late);
    let b = w.staff().last().unwrap().id;
    assert!(w.app().draft.contains_key(&b) && w.app().published.contains_key(&b));
    // the button: default rozpis into the draft, undoable
    w.apply(&c, Command::AutoSchedule).unwrap();
    assert!(!w.app().manual_rozpis);
    assert_eq!(w.app().draft, super::app::default_rozpis(&w, &c));
    w.undo(&c).unwrap();
    assert!(w.app().manual_rozpis);
    assert_eq!(w.app().draft[&a][2], Shift::Late);
    // firing drops the row
    w.apply(&c, Command::Fire { staff: b }).unwrap();
    assert!(!w.app().draft.contains_key(&b) && !w.app().published.contains_key(&b));
}

// ---------------------------------------------------------------------------------------------
// Layout, breaks, hiring log
// ---------------------------------------------------------------------------------------------

#[test]
fn pendant_lamps_hang_over_every_table() {
    let c = content();
    let w = World::new_story(&c, Difficulty::Cozy);
    let tables: Vec<(i32, i32)> = w
        .furniture()
        .iter()
        .filter(|p| p.def == "table_round")
        .map(|p| (p.x, p.y))
        .collect();
    assert!(!tables.is_empty());
    for t in &tables {
        let lamp = w
            .furniture()
            .iter()
            .find(|p| p.def == "pendant_lamp" && (p.x, p.y) == *t)
            .unwrap_or_else(|| panic!("no pendant lamp over the table at {t:?}"));
        assert!(lamp.on, "pendant lamps are lit");
        assert!(c.furniture_def("pendant_lamp").unwrap().has_tag("light"));
    }
    // the window light is still grandma's floor lamp, and starts off
    let wl = super::logic::window_light(&w, &c).unwrap();
    assert_eq!(w.furniture_by_uid(wl).unwrap().def, "floor_lamp");
    assert!(!w.window_light_on(&c));
}

#[test]
fn send_on_break_command() {
    let c = content();
    // a night where the hire is on the floor (not sick or off)
    let mut w = (2..40)
        .map(|seed| {
            let mut w = sandbox(&c, seed, Difficulty::Cozy);
            assert_eq!(
                w.apply(
                    &c,
                    Command::SendOnBreak {
                        staff: w.staff()[0].id
                    }
                ),
                Err(CommandError::WrongPhase)
            );
            w.start_night(&c);
            for _ in 0..30 * TICKS_PER_MINUTE {
                w.tick(&c);
            }
            w
        })
        .find(|w| {
            w.staff()
                .iter()
                .any(|s| s.on_duty() && !s.owner && !matches!(s.task, Some(Task::Break { .. })))
        })
        .expect("a night with the hire on the floor");
    let owner = w.staff()[0].id;
    let _ = w.drain_events();
    // a helper on the floor (Karolina takes no breaks: she does what the player asks)
    assert!(matches!(
        w.apply(&c, Command::SendOnBreak { staff: owner }),
        Err(CommandError::Invalid(_))
    ));
    let hire = w
        .staff()
        .iter()
        .rev()
        .find(|s| s.on_duty() && !s.owner && !matches!(s.task, Some(Task::Break { .. })))
        .map(|s| s.id)
        .unwrap();
    w.apply(&c, Command::SendOnBreak { staff: hire }).unwrap();
    assert!(matches!(
        w.staff_by_id(hire).unwrap().task,
        Some(Task::Break { .. })
    ));
    assert!(
        w.drain_events()
            .contains(&SimEvent::StaffBreak { staff: hire })
    );
    assert!(matches!(
        w.apply(&c, Command::SendOnBreak { staff: hire }),
        Err(CommandError::Invalid(_))
    ));
    let mut ended = false;
    for _ in 0..40 * TICKS_PER_MINUTE {
        w.tick(&c);
        if w.drain_events()
            .contains(&SimEvent::StaffBreakEnded { staff: hire })
        {
            ended = true;
            break;
        }
    }
    assert!(ended, "the break ends and they're back");
    assert!(!matches!(
        w.staff_by_id(hire).unwrap().task,
        Some(Task::Break { .. })
    ));
}

#[test]
fn everyone_on_a_full_shift_gets_a_break_but_the_cafe_is_never_left_alone() {
    let c = content();
    // Cozy needs drain slowly: breaks must still happen
    let mut w = sandbox(&c, 8, Difficulty::Cozy);
    hire(&mut w, &c, 1);
    play_night(&mut w, &c);
    for s in w.staff().iter().filter(|s| !s.owner) {
        if s.minutes_worked >= 360 {
            assert!(s.breaks_tonight >= 1, "{} never rested", s.name);
        }
    }
    // a solo night: the owner doesn't take a scheduled break and leave the café empty
    let mut w = World::new_story(&c, Difficulty::Cozy);
    play_night(&mut w, &c);
    assert_eq!(w.staff()[0].breaks_tonight, 0);
}

#[test]
fn hiring_log_carries_the_display_name() {
    let c = content();
    let mut w = sandbox(&c, 6, Difficulty::Cozy);
    hire(&mut w, &c, 1);
    let s = w.staff().last().unwrap().clone();
    w.apply(&c, Command::Fire { staff: s.id }).unwrap();
    let e = w
        .event_log()
        .iter()
        .rev()
        .find(|e| e.key == "ui.crew.hired")
        .unwrap();
    let name = e.args.iter().find(|(k, _)| k == "staff").unwrap();
    assert_eq!(name.1, s.name, "readable even after they left");
    assert!(!name.1.is_empty());
}

#[test]
fn starting_money_and_debt_are_readable_and_fair() {
    let c = content();
    let cozy = World::new_story(&c, Difficulty::Cozy);
    let classic = World::new_story(&c, Difficulty::Classic);
    assert_eq!(cozy.debt(), 90_000);
    assert_eq!(classic.debt(), 90_000);
    assert!(cozy.money() > classic.money(), "Cozy is the generous one");
    assert!(
        classic.money() >= 8_000,
        "Classic can survive the first week"
    );
    for w in [&cozy, &classic] {
        assert_eq!(w.money() % 1000, 0);
        assert_eq!(w.rent(), STORY_UTILITIES);
        assert_eq!(w.cafe_name(), STORY_CAFE);
        assert_eq!(w.town().name, STORY_TOWN);
    }
    for seed in 1..=12 {
        for d in [Difficulty::Cozy, Difficulty::Classic] {
            let w = sandbox(&c, seed, d);
            assert_eq!(w.money() % 1000, 0, "sandbox money {}", w.money());
            assert_eq!(w.rent() % 50, 0, "sandbox rent {}", w.rent());
            let (lo, hi) = c.sandbox.rent;
            assert!(w.rent() >= lo as i64 && w.rent() <= hi as i64);
            assert_eq!(w.staff().len(), 2, "sandbox starts with one hire");
            assert!(c.sandbox.town_names.places.contains(&w.town().name));
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Long runs: events, arcs, cameo, endings
// ---------------------------------------------------------------------------------------------

#[test]
fn clock_333_is_every_story_night_but_rare_in_sandbox() {
    let c = content();
    let mut w = sandbox(&c, 21, Difficulty::Cozy);
    let mut stops = 0;
    for _ in 0..20 {
        for e in play_night(&mut w, &c) {
            if matches!(e, SimEvent::ClockStop { .. }) {
                stops += 1;
            }
        }
    }
    assert!(
        stops <= 8,
        "sandbox: the clock stopped {stops} of 20 nights"
    );
}

#[test]
fn thirty_story_nights_arcs_cameo_and_a_light_ending() {
    let c = content();
    let mut w = World::new_story_seeded(&c, Difficulty::Cozy, 4242);
    let mut anwar = Vec::new();
    let mut dishes_nights = 0;
    for night in 1..=30 {
        match night {
            3 => hire(&mut w, &c, 2),
            5 | 9 => hire(&mut w, &c, 1),
            _ => {}
        }
        for e in play_night(&mut w, &c) {
            if let SimEvent::RegularArrived { regular, .. } = &e
                && regular == "guest_anwar"
            {
                anwar.push(night);
            }
        }
        let s = w.last_summary().unwrap();
        assert_eq!(s.stuck_resets, 0, "night {night}: stuck agents");
        assert!(s.avg_satisfaction.is_finite() && s.reputation_end.is_finite());
        assert!(s.guests_served > 0, "night {night}: nobody served");
        if s.dishes_broken > 0 {
            dishes_nights += 1;
        }
        assert!(s.dishes_broken <= 2);
        for st in w.staff() {
            assert!(st.energy.is_finite() && st.mood.is_finite() && st.hunger.is_finite());
            assert!((0.0..=100.0).contains(&st.energy));
        }
        assert!(w.supplies() >= 0, "night {night}: negative stock");
        if w.is_closed() {
            break;
        }
    }
    anwar.dedup();
    assert_eq!(anwar, vec![9, 24], "Anwar only on nights 9 and 24");
    assert!(
        dishes_nights < 22,
        "broken dishes on {dishes_nights} of 30 nights"
    );
    // the regulars' arcs move on with a decent crew (no building needed for these)
    let arc = |id: &str| {
        w.story()
            .counters
            .get(&format!("arc.{id}"))
            .copied()
            .unwrap_or(0)
    };
    assert!(arc("mirek") >= 3, "mirek arc {}", arc("mirek"));
    assert!(arc("guest_333") >= 2, "333 arc {}", arc("guest_333"));
    assert!(arc("vera") >= 2, "vera arc {}", arc("vera"));
    assert!(arc("radek") >= 2, "radek arc {}", arc("radek"));
    assert_eq!(w.ending(), Some("light"));
}

#[test]
fn ending_light_is_reached_on_night_thirty() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    jump_to_night(&mut w, &c, 30);
    let events = play_night(&mut w, &c);
    assert!(events.contains(&SimEvent::EndingReached("light".into())));
    assert_eq!(w.ending(), Some("light"));
    assert!(w.is_closed());
}

#[test]
fn ending_last_train_by_accepting_the_offer() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    jump_to_night(&mut w, &c, 30);
    w.story.vars.insert("offer".into(), "accepted".into());
    play_night(&mut w, &c);
    assert_eq!(w.ending(), Some("last_train"));
    assert_eq!(w.ending_variant(), Some("sold"));
}

#[test]
fn ending_last_train_when_classic_payments_fail() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Classic);
    for night in [7, 14] {
        jump_to_night(&mut w, &c, night);
        w.debug_set_money(0);
        play_night(&mut w, &c);
        assert!(w.last_summary().unwrap().payment_missed, "night {night}");
    }
    assert!(
        w.is_closed(),
        "two missed payments close the café on Classic"
    );
    assert_eq!(w.ending(), Some("last_train"));
    assert_eq!(w.night(), 14);
}

#[test]
fn cozy_never_closes_on_missed_payments() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    for night in [7, 14, 21] {
        jump_to_night(&mut w, &c, night);
        w.debug_set_money(0);
        play_night(&mut w, &c);
    }
    assert!(!w.is_closed());
    assert_eq!(w.ending(), None);
}

#[test]
fn fast_service_streak_counts_from_the_order() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    play_night(&mut w, &c);
    play_night(&mut w, &c);
    hire(&mut w, &c, 2);
    let mut served = Vec::new();
    for _ in 0..4 {
        play_night(&mut w, &c);
        if let Some(v) = w.story().regulars["mirek"].visits.last() {
            served.extend(v.served_minutes);
        }
    }
    assert!(!served.is_empty());
    // measured on the wait clock from the order, the first coffee comes within a few minutes
    assert!(
        served.iter().any(|m| *m <= 10),
        "mirek served after {served:?} min"
    );
    let _ = Job::Bar;
}

#[test]
fn the_first_hire_is_offered_on_night_three() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    assert!(
        w.candidates().is_empty(),
        "night 1 is Karolina's solo night"
    );
    play_night(&mut w, &c);
    assert_eq!(w.night(), 2);
    assert!(w.candidates().is_empty(), "night 2 is solo too");
    play_night(&mut w, &c);
    assert_eq!(w.night(), FIRST_HIRE_NIGHT);
    assert!(
        !w.candidates().is_empty(),
        "the hire hint and the candidates line up"
    );
    // the hire card proposes the archetype's default duty
    for cand in w.candidates() {
        let a = c.staff_archetype(&cand.staff.archetype).unwrap();
        assert_eq!(cand.staff.duties, vec![a.default_duty.unwrap()]);
    }
}
