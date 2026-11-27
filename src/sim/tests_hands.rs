//! Hands-on tests (design/handson.md §1, §3, §10): Karolina's queue and acts, wishes & the
//! match, plain rounds & the tray, delegation & capacity, hand-off, the wait clock & Cozy
//! forgiveness, tips on tables, rush & rhythm, the lull director, the jukebox, recipe
//! knowledge, save v1 → v2, determinism, and the player-bot's first-week targets.

use super::guests::{self, Exit};
use super::*;
use crate::content::Content;
use crate::content::handson::{Duty, HallMood, MatchContext, RecipeUnlock, match_score};

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

fn content() -> Content {
    Content::load(DATA).unwrap_or_else(|e| panic!("{e}"))
}

/// A story night `n` running, cutscenes done, no random walk-ins.
fn story_night(c: &Content, n: u32, d: Difficulty) -> World {
    let mut w = World::new_story(c, d);
    // the tutorial's scripted guests would break the quiet
    w.sys.tutorial_off = true;
    if n > 1 {
        w.night = n;
        w.begin_day(c);
    }
    bot::finish_cutscenes(&mut w, c);
    w.start_night(c);
    bot::finish_cutscenes(&mut w, c);
    w.skip_prep(c);
    quiet(&mut w);
    w.drain_events();
    w
}

/// No random walk-ins tonight (scheduled ones still come).
fn quiet(w: &mut World) {
    w.mods.push(Modifier {
        kind: ModKind::GuestRate,
        mul: 0.0,
        until: None,
        event: None,
    });
}

/// A party walks in now; `wish` forces a wish leader (else a plain party).
fn spawn(w: &mut World, c: &Content, archetype: &str, size: u32, wish: bool) -> u32 {
    let pid = guests::spawn_party(w, c, archetype, size, None, 1.0, 1.0, &[], None, wish)
        .expect("spawned");
    if !wish && let Some(p) = w.parties.iter_mut().find(|p| p.id == pid) {
        p.order_kind = OrderKind::Plain;
        p.wish_open = false;
    }
    pid
}

fn party(w: &World, pid: u32) -> &Party {
    w.parties().iter().find(|p| p.id == pid).expect("party")
}

/// Ticks until `f` matches an event (or `max` ticks); returns all events seen.
fn tick_until(
    w: &mut World,
    c: &Content,
    max: u32,
    f: impl Fn(&SimEvent) -> bool,
) -> (bool, Vec<SimEvent>) {
    let mut all = Vec::new();
    for _ in 0..max {
        w.tick(c);
        bot::finish_cutscenes(w, c);
        let ev = w.drain_events();
        let hit = ev.iter().any(&f);
        all.extend(ev);
        if hit {
            return (true, all);
        }
    }
    (false, all)
}

/// The first helper, experienced (capacity 3), with exactly these duties.
fn helper_with(w: &mut World, c: &Content, duties: &[Duty]) -> u32 {
    let i = w.staff.iter().position(|s| !s.owner).expect("a helper");
    let id = w.staff[i].id;
    w.staff[i].nights_worked = 5;
    w.staff[i].skills = [7.0; 5];
    for d in Duty::ALL {
        w.apply(
            c,
            Command::SetDuty {
                staff: id,
                duty: d,
                on: false,
            },
        )
        .unwrap();
    }
    for d in duties {
        w.apply(
            c,
            Command::SetDuty {
                staff: id,
                duty: *d,
                on: true,
            },
        )
        .unwrap();
    }
    id
}

// ---------------------------------------------------------------------------------------------
// Karolina: order → wish → brew → serve
// ---------------------------------------------------------------------------------------------

#[test]
fn karolina_takes_a_wish_brews_by_hand_and_serves_it() {
    let c = content();
    let mut w = story_night(&c, 1, Difficulty::Cozy);
    let pid = spawn(&mut w, &c, "travelers", 1, true);
    let wish = party(&w, pid).order_kind.wish().unwrap().to_string();
    assert!(party(&w, pid).wish_open);
    let leader = party(&w, pid).leader().unwrap();
    // wishes are told at the table, never in the register queue
    assert_eq!(party(&w, pid).state, PartyState::WaitOrder);
    w.apply(&c, Command::PlayerTakeOrder { guest: leader })
        .unwrap();
    assert_eq!(
        w.apply(&c, Command::PlayerTakeOrder { guest: leader }),
        Err(CommandError::Invalid("already on it".into()))
    );
    let (ok, ev) = tick_until(&mut w, &c, 900, |e| {
        matches!(e, SimEvent::PlayerOrderTaken { .. })
    });
    assert!(ok, "she takes the order");
    assert!(
        ev.iter().any(|e| matches!(e,
        SimEvent::PlayerOrderTaken { guest, wish: Some(x), .. } if *guest == leader && *x == wish))
    );
    // the order card: up to 4 chips, best first; never a wish the café can't answer
    let chips = w.best_matches(&c, leader);
    assert!(!chips.is_empty() && chips.len() <= 4);
    assert!(chips.windows(2).all(|p| p[0].1 >= p[1].1));
    assert!(chips[0].1 >= c.handson.tuning.fair_wishes.skip_below);
    let (recipe, m) = chips[0].clone();
    let expect = match_score(
        c.handson.wish(&wish).unwrap(),
        c.handson.recipe(&recipe).unwrap(),
        MatchContext::default(),
    );
    assert_eq!(m, expect);
    // an unknown recipe is refused
    assert_eq!(
        w.apply(
            &c,
            Command::PlayerBrew {
                guest: leader,
                recipe: "r_moon_latte".into(),
                quality: 1.0,
                mode: BrewMode::ByHand,
            },
        ),
        Err(CommandError::NotUnlocked)
    );
    let sat0 = party(&w, pid).satisfaction;
    w.apply(
        &c,
        Command::PlayerBrew {
            guest: leader,
            recipe: recipe.clone(),
            quality: 0.9,
            mode: BrewMode::ByHand,
        },
    )
    .unwrap();
    assert!(!party(&w, pid).wish_open);
    let (ok, ev) = tick_until(&mut w, &c, 1500, |e| {
        matches!(e, SimEvent::PlayerServed { .. })
    });
    assert!(ok, "she brews and serves");
    assert!(ev.iter().any(|e| matches!(e,
        SimEvent::PlayerServed { guest, recipe: r, match_score, quality }
            if *guest == leader && *r == recipe && *match_score == m && *quality == 0.9)));
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::GuestReaction { guest, .. } if *guest == leader))
    );
    let p = party(&w, pid);
    assert!(
        p.satisfaction > sat0,
        "match and quality raise satisfaction"
    );
    assert_eq!(p.tip_factors.len(), 1);
    let s = c.handson.tuning.serve;
    let want = s.tip_base + s.tip_span * m * (0.5 + 0.5 * 0.9);
    assert!((p.tip_factors[0] - want).abs() < 1e-4);
    assert_eq!(w.hands_tally().wishes_served, 1);
    assert_eq!(w.hands_tally().by_hand, 1);
}

#[test]
fn a_quick_plain_brew_makes_the_round_and_carries_a_tray() {
    let c = content();
    let mut w = story_night(&c, 1, Difficulty::Cozy);
    // a plain party of three with only coffees: everything at one station
    let pid = spawn(&mut w, &c, "students", 3, false);
    w.parties.iter_mut().find(|p| p.id == pid).unwrap().wishes = vec![
        "cappuccino".into(),
        "black_coffee".into(),
        "cappuccino".into(),
    ];
    w.apply(&c, Command::PlayerTakeOrder { guest: pid })
        .unwrap();
    let (ok, _) = tick_until(&mut w, &c, 900, |e| {
        matches!(e, SimEvent::PlayerOrderTaken { wish: None, .. })
    });
    assert!(ok);
    let items = party(&w, pid).items.clone();
    assert_eq!(items.len(), 3);
    let first = w.items.iter().find(|i| i.id == items[0]).unwrap().guest;
    let chips = w.best_matches(&c, first);
    assert!(
        !chips.is_empty() && chips.iter().all(|x| x.1 == 1.0),
        "plain chips: the item's recipes"
    );
    w.apply(
        &c,
        Command::PlayerBrew {
            guest: first,
            recipe: "r_cappuccino".into(),
            quality: 0.0,
            mode: BrewMode::Quick,
        },
    )
    .unwrap();
    // one act, the whole round claimed
    assert_eq!(w.player().queue.len(), 1);
    let me = w.karolina().unwrap().id;
    assert!(
        items
            .iter()
            .all(|id| w.items.iter().find(|i| i.id == *id).unwrap().by == Some(me))
    );
    let (_, ev) = tick_until(
        &mut w,
        &c,
        3000,
        |e| matches!(e, SimEvent::GuestLeft { party, .. } if *party == pid),
    );
    let served: Vec<&SimEvent> = ev
        .iter()
        .filter(|e| matches!(e, SimEvent::PlayerServed { .. }))
        .collect();
    assert_eq!(served.len(), 3, "the tray served all three");
    assert!(served.iter().all(|e| matches!(e,
        SimEvent::PlayerServed { match_score, quality, .. }
            if *match_score == 1.0 && *quality == c.handson.tuning.quick_quality)));
}

#[test]
fn the_queue_is_fifo_capped_at_five_and_can_be_reordered_and_cancelled() {
    let c = content();
    let mut w = story_night(&c, 1, Difficulty::Cozy);
    let spots: Vec<(i32, i32)> = (0..6).map(|k| (3 + k, 4)).collect();
    // the day refuses acts
    let mut day = World::new_story(&c, Difficulty::Cozy);
    assert_eq!(
        day.apply(
            &c,
            Command::PlayerQueue {
                act: PlayerAct::GoTo { tile: spots[0] },
                front: false,
            },
        ),
        Err(CommandError::WrongPhase)
    );
    for t in &spots[..5] {
        w.apply(
            &c,
            Command::PlayerQueue {
                act: PlayerAct::GoTo { tile: *t },
                front: false,
            },
        )
        .unwrap();
    }
    assert_eq!(
        w.apply(
            &c,
            Command::PlayerQueue {
                act: PlayerAct::GoTo { tile: spots[5] },
                front: true,
            },
        ),
        Err(CommandError::QueueFull)
    );
    w.apply(&c, Command::PlayerUnqueue { index: 4 }).unwrap();
    w.apply(
        &c,
        Command::PlayerQueue {
            act: PlayerAct::GoTo { tile: spots[5] },
            front: true,
        },
    )
    .unwrap();
    assert_eq!(
        w.player().queue.front(),
        Some(&PlayerAct::GoTo { tile: spots[5] })
    );
    w.tick(&c);
    assert_eq!(
        w.player().current.as_ref().map(|a| a.act.clone()),
        Some(PlayerAct::GoTo { tile: spots[5] }),
        "shift+click = do next"
    );
    w.apply(&c, Command::PlayerCancel).unwrap();
    assert!(w.player().current.is_none());
    assert_eq!(w.player().queue.len(), 4);
    assert!(w.apply(&c, Command::PlayerUnqueue { index: 9 }).is_err());
    assert!(w.apply(&c, Command::PlayerCancel).is_err());
}

#[test]
fn greeting_and_the_micro_acts_wash_and_mop() {
    let c = content();
    let mut w = story_night(&c, 1, Difficulty::Cozy);
    let pid = spawn(&mut w, &c, "travelers", 1, true);
    let sat = party(&w, pid).satisfaction;
    w.apply(&c, Command::PlayerGreet { guest: pid }).unwrap();
    assert!(party(&w, pid).satisfaction > sat);
    assert!(w.apply(&c, Command::PlayerGreet { guest: pid }).is_err());
    // wash: she walks to the sink, then each hold washes 2 dishes per second (without the
    // drying rack beside it — that one is `tests_bar`)
    w.furniture.retain(|p| p.def != "dish_rack");
    w.sink_dishes = 9;
    let sink = w
        .furniture
        .iter()
        .find(|p| p.def == "sink")
        .map(|p| p.uid)
        .unwrap();
    assert!(
        w.apply(&c, Command::PlayerWashProgress { sink, seconds: 1.0 })
            .is_err(),
        "not at the sink yet"
    );
    w.apply(
        &c,
        Command::PlayerQueue {
            act: PlayerAct::Wash { sink },
            front: false,
        },
    )
    .unwrap();
    for _ in 0..300 {
        w.tick(&c);
        if w.player().current.as_ref().is_some_and(|a| a.stage == 1) {
            break;
        }
    }
    w.apply(
        &c,
        Command::PlayerWashProgress {
            sink,
            seconds: 2.25,
        },
    )
    .unwrap();
    assert_eq!(w.sink_dishes(), 5, "4.5 dishes: 4 now, the half is kept");
    w.apply(
        &c,
        Command::PlayerWashProgress {
            sink,
            seconds: 0.25,
        },
    )
    .unwrap();
    assert_eq!(w.sink_dishes(), 4);
    w.apply(&c, Command::PlayerWashProgress { sink, seconds: 2.0 })
        .unwrap();
    assert_eq!(w.sink_dishes(), 0);
    assert!(w.player().current.is_none(), "an empty sink ends the act");
    // mop: one swipe takes 0.1 dirt
    w.dirt = 0.35;
    assert_eq!(w.dirt_spots().len(), 4);
    let spot = w.dirt_spots()[0];
    w.apply(
        &c,
        Command::PlayerQueue {
            act: PlayerAct::Mop { tile: spot },
            front: false,
        },
    )
    .unwrap();
    for _ in 0..300 {
        w.tick(&c);
        if w.player().current.is_none() && w.player().queue.is_empty() {
            break;
        }
    }
    assert!((w.dirt() - 0.25).abs() < 0.02, "dirt {}", w.dirt());
}

// ---------------------------------------------------------------------------------------------
// Delegation
// ---------------------------------------------------------------------------------------------

#[test]
fn capacity_limits_duties_and_grows_with_nights_and_skill() {
    let c = content();
    let mut w = World::new_sandbox(&c, SandboxSetup::default());
    let id = w.staff[1].id;
    w.staff[1].skills = [5.0; 5];
    w.staff[1].traits.clear();
    w.staff[1].nights_worked = 0;
    assert_eq!(w.capacity(&c, id), 1);
    assert_eq!(w.staff_duties(id).len(), 1, "the hire card's default duty");
    let other = Duty::ALL
        .iter()
        .copied()
        .find(|d| !w.staff_duties(id).contains(d) && *d != Duty::Floor)
        .unwrap();
    assert_eq!(
        w.apply(
            &c,
            Command::SetDuty {
                staff: id,
                duty: other,
                on: true,
            },
        ),
        Err(CommandError::Capacity)
    );
    w.staff[1].nights_worked = 3;
    assert_eq!(w.capacity(&c, id), 2);
    w.staff[1].skills[Duty::Floor.job().index()] = 8.0;
    assert_eq!(w.capacity(&c, id), 2, "skill counts for held duties only");
    w.apply(
        &c,
        Command::SetDuty {
            staff: id,
            duty: Duty::Floor,
            on: true,
        },
    )
    .unwrap();
    assert_eq!(w.capacity(&c, id), 3);
    assert_eq!(w.capacity(&c, w.staff[0].id), 0, "Karolina: none");
    // priorities mirror the duties; undo takes the duty back
    assert_eq!(w.staff[1].priorities[Duty::Floor.job().index()], 1);
    w.undo(&c).unwrap();
    assert!(!w.staff_duties(id).contains(&Duty::Floor));
    assert!(w.suggest_duty(id).is_some());
}

#[test]
fn helpers_never_take_wishes_and_karolina_can_hand_one_off() {
    let c = content();
    let mut w = World::new_sandbox(&c, SandboxSetup::default());
    let helper = helper_with(&mut w, &c, &[Duty::Register, Duty::Bar, Duty::Serving]);
    bot::finish_cutscenes(&mut w, &c);
    w.start_night(&c);
    w.skip_prep(&c);
    quiet(&mut w);
    let hi = w.staff.iter().position(|s| s.id == helper).unwrap();
    w.staff[hi].status = StaffStatus::OnDuty;
    let wish = spawn(&mut w, &c, "travelers", 1, true);
    let wish_leader = party(&w, wish).leader().unwrap();
    // a drink wish (the helper holds the bar, not the kitchen)
    let drink = c
        .handson
        .wishes
        .wishes
        .iter()
        .find(|d| {
            d.regular().is_none()
                && super::orders::hand_off_recipe(&w, &c, &d.id)
                    .and_then(|(r, _)| c.handson.recipe(&r))
                    .and_then(|r| c.menu_item(&r.base_item))
                    .is_some_and(|m| m.job == crate::content::Job::Bar)
        })
        .map(|d| d.id.clone())
        .unwrap();
    w.parties
        .iter_mut()
        .find(|p| p.id == wish)
        .unwrap()
        .order_kind = OrderKind::Wish { wish: drink };
    let plain = spawn(&mut w, &c, "travelers", 1, false);
    w.parties.iter_mut().find(|p| p.id == plain).unwrap().wishes = vec!["black_coffee".into()];
    let (_, ev) = tick_until(
        &mut w,
        &c,
        900,
        |e| matches!(e, SimEvent::OrderTaken { party, .. } if *party == plain),
    );
    assert!(ev.iter().any(|e| matches!(e,
        SimEvent::OrderTaken { party, staff, .. } if *party == plain && *staff == helper)));
    for _ in 0..150 {
        w.tick(&c);
    }
    assert!(
        party(&w, wish).ordered_at.is_none(),
        "nobody but Karolina takes a wish"
    );
    // hand it off: the helpers take it as a plain ticket, the match capped
    w.apply(&c, Command::PlayerHandOff { guest: wish }).unwrap();
    assert!(party(&w, wish).handed_off);
    let (ok, ev) = tick_until(
        &mut w,
        &c,
        3000,
        |e| matches!(e, SimEvent::GuestReaction { guest, .. } if *guest == wish_leader),
    );
    assert!(ok, "the guest reacts to the handed-off wish");
    assert!(ev.iter().any(|e| matches!(e,
        SimEvent::OrderServed { party, staff, .. } if *party == wish && *staff == helper)));
    let item = w.items.iter().find(|i| i.party == wish && i.wish).unwrap();
    assert!(item.hand_off_match.unwrap() <= c.handson.tuning.hand_off_match_cap);
    assert!(matches!(item.origin, ItemOrigin::Helper { quality } if quality <= 0.75));
}

// ---------------------------------------------------------------------------------------------
// Waiting
// ---------------------------------------------------------------------------------------------

#[test]
fn the_wait_clock_runs_at_half_speed_while_karolina_is_on_it() {
    let c = content();
    let mut w = story_night(&c, 1, Difficulty::Cozy);
    let a = spawn(&mut w, &c, "insomniacs", 1, true);
    let b = spawn(&mut w, &c, "insomniacs", 1, true);
    for _ in 0..400 {
        w.tick(&c);
    }
    // she claims a only, behind a walk so she doesn't take it yet
    w.apply(
        &c,
        Command::PlayerQueue {
            act: PlayerAct::GoTo {
                tile: w.grid.entrance(),
            },
            front: false,
        },
    )
    .unwrap();
    w.apply(&c, Command::PlayerTakeOrder { guest: a }).unwrap();
    let (wa, wb) = (party(&w, a).wait_clock, party(&w, b).wait_clock);
    for _ in 0..(4 * TICKS_PER_MINUTE) {
        w.tick(&c);
    }
    let da = party(&w, a).wait_clock - wa;
    let db = party(&w, b).wait_clock - wb;
    assert!((db - 4.0).abs() < 1.01, "unclaimed: {db}");
    assert!((da - 2.0).abs() < 1.01, "claimed: {da}");
}

#[test]
fn cozy_guests_forgive_and_classic_guests_leave_angry() {
    let c = content();
    // nobody behind the counter on a solo night
    let mut cozy = World::new_story(&c, Difficulty::Cozy);
    let ev = cozy.run_night_headless(&c);
    let s = cozy.last_summary().unwrap();
    assert_eq!(s.guests_left_unhappy, 0, "Cozy: nobody leaves angry");
    assert!(s.guests_forgiven > 0);
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::GuestPatienceRefilled { .. }))
    );
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::GuestForgiven { key, .. } if key == FORGIVEN_KEY))
    );
    // forgiven guests write no review: only the dark window (missed checklist, −1) and
    // guests who found no seat cost a little
    let walked_on = 0.05 * s.guests_turned_away as f32;
    assert!(
        s.reputation_end >= s.reputation_start - 1.0 - walked_on - 0.25,
        "{} → {} ({} turned away)",
        s.reputation_start,
        s.reputation_end,
        s.guests_turned_away
    );
    let mut classic = World::new_story(&c, Difficulty::Classic);
    classic.run_night_headless(&c);
    let s = classic.last_summary().unwrap();
    assert!(s.guests_left_unhappy > 0);
    assert_eq!(s.guests_forgiven, 0);
}

#[test]
fn regulars_wait_on_cozy_until_their_window_ends() {
    let c = content();
    // (after act 1: on night 1 the gentle start's patience outlasts Mirek's whole window)
    let mut w = story_night(&c, 9, Difficulty::Cozy);
    guests::spawn_regular(&mut w, &c, "mirek", Some(0));
    let pid = w.story.regulars["mirek"].party.unwrap();
    let p = party(&w, pid);
    assert!(p.order_kind.is_wish(), "regulars always wish");
    assert!(p.patience_max >= c.handson.tuning.cozy_regular_patience_floor);
    let leave_by = p.leave_by.unwrap();
    let mut refills = 0;
    while w
        .parties()
        .iter()
        .any(|p| p.id == pid && p.state != PartyState::Leaving)
        && w.phase() == Phase::Night
    {
        w.tick(&c);
        bot::finish_cutscenes(&mut w, &c);
        for e in w.drain_events() {
            if matches!(e, SimEvent::GuestPatienceRefilled { party } if party == pid) {
                refills += 1;
            }
        }
    }
    assert!(w.minute() >= leave_by, "Mirek stayed until {leave_by}");
    assert!(refills >= 1);
}

// ---------------------------------------------------------------------------------------------
// Tips, rush, rhythm, lull
// ---------------------------------------------------------------------------------------------

#[test]
fn tips_lie_on_tables_until_collected_and_nothing_is_lost() {
    let c = content();
    // a served guest's tip becomes a coin, not money
    let mut w = story_night(&c, 1, Difficulty::Cozy);
    let mut coin = None;
    for _ in 0..20 {
        let pid = spawn(&mut w, &c, "travelers", 1, false);
        let leader = party(&w, pid).leader().unwrap();
        // served and happy
        let p = w.parties.iter_mut().find(|p| p.id == pid).unwrap();
        p.satisfaction = 100.0;
        p.served = true;
        let iid = w.alloc_id();
        w.items.push(OrderItem {
            id: iid,
            party: pid,
            guest: leader,
            menu: "cappuccino".into(),
            state: ItemState::Served,
            prep_left: 0.0,
            by: None,
            ready_tile: None,
            quality: 0.0,
            created: 0,
            origin: ItemOrigin::Unassigned,
            price_add: 0,
            hand_off_match: None,
            wish: false,
        });
        w.parties.iter_mut().find(|p| p.id == pid).unwrap().items = vec![iid];
        for _ in 0..200 {
            w.tick(&c);
            bot::finish_cutscenes(&mut w, &c);
        }
        let money = w.money();
        guests::finish_party(&mut w, &c, pid, Exit::Normal);
        let ev = w.drain_events();
        if let Some((table, amount)) = ev.iter().find_map(|e| match e {
            SimEvent::TipLeft { table, amount } => Some((*table, *amount)),
            _ => None,
        }) {
            let bill = ev
                .iter()
                .find_map(|e| match e {
                    SimEvent::Payment { amount } => Some(*amount),
                    _ => None,
                })
                .unwrap_or(0);
            assert_eq!(w.money(), money + bill, "the tip isn't money yet");
            coin = Some((table, amount));
            break;
        }
    }
    let (table, amount) = coin.expect("someone tipped");
    let before = w.money();
    w.apply(
        &c,
        Command::PlayerQueue {
            act: PlayerAct::CollectTip { table },
            front: false,
        },
    )
    .unwrap();
    let (ok, _) = tick_until(
        &mut w,
        &c,
        600,
        |e| matches!(e, SimEvent::TipCollected { amount: a, by: Some(_) } if *a == amount),
    );
    assert!(ok);
    assert!(w.tips_on_tables().is_empty());
    assert!(w.money() >= before + amount);
    assert_eq!(w.tips_lifetime(), amount);
    // a coin nobody picks up goes into the jar at six
    w.hands.tips.push(TipCoin {
        id: 999_999,
        table,
        amount: 40,
        staff: None,
        minute: w.minute(),
        napkin: None,
    });
    while w.phase() == Phase::Night {
        w.tick(&c);
        bot::finish_cutscenes(&mut w, &c);
    }
    assert!(w.tips_on_tables().is_empty());
    assert!(w.last_summary().unwrap().tips >= amount + 40);
}

#[test]
fn a_burst_of_arrivals_starts_a_rush_from_night_three() {
    let c = content();
    for (night, expect) in [(2, false), (3, true)] {
        let mut w = story_night(&c, night, Difficulty::Cozy);
        for _ in 0..3 {
            spawn(&mut w, &c, "travelers", 1, false);
        }
        let ev = w.drain_events();
        assert_eq!(ev.contains(&SimEvent::RushStarted), expect, "night {night}");
        assert_eq!(w.rush(), expect);
        if expect {
            // the house playlist shifts (a record the player chose would keep playing)
            assert_eq!(
                w.music_track(&c).map(|t| t.0),
                Some(c.handson.records.house.rush.clone())
            );
            let (ok, _) = tick_until(&mut w, &c, 25 * TICKS_PER_MINUTE, |e| {
                *e == SimEvent::RushEnded
            });
            assert!(ok);
        }
    }
}

#[test]
fn rhythm_builds_with_quick_serves_and_fades_without_a_word() {
    let c = content();
    let mut w = story_night(&c, 1, Difficulty::Cozy);
    let r = c.handson.tuning.rhythm;
    let mut factors = Vec::new();
    for m in [10u32, 12, 15, 17] {
        w.tick = m * TICKS_PER_MINUTE;
        factors.push(super::pacing::rhythm_on_serve(&mut w, &c));
    }
    assert_eq!(w.rhythm(), 3);
    assert_eq!(factors[0], 1.0);
    assert!((factors[3] - (1.0 + 3.0 * r.step)).abs() < 1e-5);
    for _ in 0..((r.window_minutes + 2) * TICKS_PER_MINUTE) {
        w.tick(&c);
    }
    assert_eq!(w.rhythm(), 0, "a gap simply ends it");
    assert!(
        w.drain_events()
            .contains(&SimEvent::RhythmChanged { step: 0 })
    );
}

#[test]
fn the_lull_director_brings_something_to_do_without_flooding() {
    let c = content();
    let mut w = story_night(&c, 1, Difficulty::Cozy);
    let t = &c.handson.tuning;
    let (ok, ev) = tick_until(&mut w, &c, (t.lull_minutes + 3) * TICKS_PER_MINUTE, |e| {
        matches!(e, SimEvent::LullSuggestion { .. })
    });
    assert!(ok, "idle {} min", w.player().idle_minutes);
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::KarolinaIdle { minutes } if *minutes >= t.lull_minutes))
    );
    assert!(ev.iter().any(|e| matches!(
        e,
        SimEvent::LullSuggestion {
            hint: LullHint::WalkIn
        }
    )));
    assert!(
        w.spawns.iter().any(|s| s.wish),
        "a guest with a wish walks in"
    );
    let (again, _) = tick_until(&mut w, &c, 2 * TICKS_PER_MINUTE, |e| {
        matches!(e, SimEvent::LullSuggestion { .. })
    });
    assert!(!again, "not right away again");
}

// ---------------------------------------------------------------------------------------------
// The jukebox
// ---------------------------------------------------------------------------------------------

#[test]
fn records_change_the_music_and_the_hall_mood() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    assert!(super::music::jukebox(&w).is_some(), "grandma's jukebox");
    assert_eq!(w.records_unlocked().len(), 3, "three starters");
    assert_eq!(
        w.apply(
            &c,
            Command::PlayRecord {
                id: Some("rec_folk_lullaby".into()),
                repeat: true,
            },
        ),
        Err(CommandError::Locked)
    );
    // the day sets tonight's opening record
    w.apply(
        &c,
        Command::PlayRecord {
            id: Some("rec_prague_swing".into()),
            repeat: true,
        },
    )
    .unwrap();
    bot::finish_cutscenes(&mut w, &c);
    w.start_night(&c);
    let ev = w.drain_events();
    assert!(ev.iter().any(|e| matches!(e,
        SimEvent::RecordChanged { record: Some(r), repeat: true } if r == "rec_prague_swing")));
    assert_eq!(w.hall_mood(&c), Some(HallMood::Lively));
    assert_eq!(
        w.music_track(&c).map(|t| t.0),
        Some("jukebox_prague_swing".into())
    );
    // at night Karolina walks to the jukebox
    bot::finish_cutscenes(&mut w, &c);
    w.apply(
        &c,
        Command::PlayRecord {
            id: Some("rec_piano_trio".into()),
            repeat: false,
        },
    )
    .unwrap();
    let (ok, _) = tick_until(
        &mut w,
        &c,
        600,
        |e| matches!(e, SimEvent::RecordChanged { record: Some(r), .. } if r == "rec_piano_trio"),
    );
    assert!(ok);
    assert_eq!(w.hall_mood(&c), Some(HallMood::Calm));
    // play once: when the pass ends, the house playlist
    w.apply(
        &c,
        Command::RecordFinished {
            id: "rec_piano_trio".into(),
        },
    )
    .unwrap();
    assert_eq!(w.music().source, MusicSource::House);
    assert_eq!(w.hall_mood(&c), None);
    // a blackout silences it
    w.debug_fire_event(&c, "power_outage");
    assert!(w.blackout());
    assert_eq!(w.music_track(&c), None);
    assert_eq!(
        w.apply(
            &c,
            Command::PlayRecord {
                id: None,
                repeat: true,
            },
        ),
        Err(CommandError::NoPower)
    );
}

#[test]
fn a_liked_record_gives_trust_once_per_visit_and_a_disliked_one_never_takes_it() {
    let c = content();
    let rules = &c.handson.records.rules;
    for (record, liked) in [("rec_prague_swing", true), ("rec_bossa_rain", false)] {
        let mut w = story_night(&c, 1, Difficulty::Cozy);
        super::music::set_record(&mut w, &c, Some(record.into()), true);
        guests::spawn_regular(&mut w, &c, "mirek", Some(0));
        let pid = w.story.regulars["mirek"].party.unwrap();
        let trust0 = w.story.trust.get("mirek").copied().unwrap_or(0.0);
        let sat0 = party(&w, pid).satisfaction;
        let mut reactions = 0;
        for _ in 0..(3 * rules.like_minutes * TICKS_PER_MINUTE) {
            w.tick(&c);
            for e in w.drain_events() {
                if matches!(e, SimEvent::RecordReaction { liked: l, .. } if l == liked) {
                    reactions += 1;
                }
            }
        }
        assert_eq!(reactions, 1, "{record}: once per visit");
        let trust = w.story.trust.get("mirek").copied().unwrap_or(0.0);
        if liked {
            assert!(trust >= trust0 + rules.like_trust - 0.01, "{record}");
        } else {
            assert!(trust >= trust0, "{record}: never a trust loss");
            assert!(party(&w, pid).satisfaction <= sat0, "{record}");
        }
        assert!(
            w.music()
                .known_tastes
                .contains(&(record.to_string(), "mirek".to_string()))
        );
    }
}

#[test]
fn records_unlock_over_the_story_and_in_sandbox() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    assert!(!w.records_unlocked().contains("rec_folk_lullaby"));
    w.night = 13;
    w.begin_day(&c);
    assert!(w.records_unlocked().contains("rec_folk_lullaby"));
    assert!(w.drain_events().contains(&SimEvent::RecordUnlocked {
        id: "rec_folk_lullaby".into()
    }));
    let mut s = World::new_sandbox(&c, SandboxSetup::default());
    s.night = 4;
    s.begin_day(&c);
    assert!(
        s.records_unlocked().contains("rec_folk_lullaby"),
        "sandbox night 4"
    );
}

// ---------------------------------------------------------------------------------------------
// Wishes & knowledge
// ---------------------------------------------------------------------------------------------

#[test]
fn wishes_are_fair_personal_and_become_the_usual() {
    let c = content();
    let mut w = story_night(&c, 8, Difficulty::Cozy);
    let mut wishes = 0;
    for k in 0..300usize {
        let arch = c.guests[k % c.guests.len()].id.clone();
        let kind = super::orders::pick_order_kind(&mut w, &c, &arch, None, 2, true);
        if let Some(id) = kind.wish() {
            wishes += 1;
            let def = c.handson.wish(id).unwrap();
            assert!(def.regular().is_none(), "{id}");
            let best = super::orders::matches_for(&w, &c, def)[0].1;
            assert!(best >= c.handson.tuning.fair_wishes.skip_below, "{id}");
        }
        w.hands.wish_uses.clear();
    }
    assert!(wishes > 250);
    let mirek = c.regular("mirek").unwrap();
    let kind = super::orders::pick_order_kind(&mut w, &c, &mirek.base, Some(mirek), 1, false);
    assert!(
        kind.wish().is_some_and(|x| x.starts_with("mirek.")),
        "{kind:?}"
    );
    w.story.trust.insert("mirek".into(), 60.0);
    w.hands.wish_uses.clear();
    let kind = super::orders::pick_order_kind(&mut w, &c, &mirek.base, Some(mirek), 1, false);
    assert!(kind.usual().is_some(), "trusted: the usual? {kind:?}");
}

#[test]
fn knowledge_starts_with_the_starters_and_grandmas_cards_arrive_with_the_menu() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    for r in c.handson.starter_recipes() {
        assert!(w.knowledge().recipes.contains_key(&r.id), "{}", r.id);
    }
    assert!(!w.knowledge().recipes.contains_key("r_moon_latte"));
    let card = c
        .handson
        .recipes
        .recipes
        .iter()
        .find(|r| {
            r.unlock == RecipeUnlock::WithMenu
                && c.menu_item(&r.base_item)
                    .is_some_and(|m| m.unlock_night > 1)
        })
        .unwrap();
    let night = c.menu_item(&card.base_item).unwrap().unlock_night;
    assert!(!w.knowledge().recipes.contains_key(&card.id));
    w.night = night;
    w.begin_day(&c);
    assert!(w.knowledge().recipes.contains_key(&card.id));
    assert!(w.drain_events().iter().any(|e| matches!(e,
        SimEvent::RecipeLearned { recipe, source } if *recipe == card.id && source == "card")));
}

#[test]
fn ten_good_by_hand_plays_make_a_recipe_practised() {
    let c = content();
    let mut w = story_night(&c, 1, Difficulty::Cozy);
    let need = c.handson.tuning.practised_after;
    assert_eq!(
        w.quick_quality(&c, "r_black"),
        c.handson.tuning.quick_quality
    );
    for k in 0..need {
        let pid = spawn(&mut w, &c, "taxi_drivers", 1, false);
        w.parties.iter_mut().find(|p| p.id == pid).unwrap().wishes = vec!["black_coffee".into()];
        w.apply(&c, Command::PlayerTakeOrder { guest: pid })
            .unwrap();
        let (ok, _) = tick_until(
            &mut w,
            &c,
            1500,
            |e| matches!(e, SimEvent::PlayerOrderTaken { party, .. } if *party == pid),
        );
        assert!(ok, "order {k}");
        let guest = party(&w, pid).leader().unwrap();
        w.apply(
            &c,
            Command::PlayerBrew {
                guest,
                recipe: "r_black".into(),
                quality: 0.85,
                mode: BrewMode::ByHand,
            },
        )
        .unwrap();
        let (ok, ev) = tick_until(
            &mut w,
            &c,
            1500,
            |e| matches!(e, SimEvent::PlayerServed { guest: g, .. } if *g == guest),
        );
        assert!(ok, "serve {k}");
        if k + 1 == need {
            assert!(
                ev.iter().any(
                    |e| matches!(e, SimEvent::PractisedRecipe { recipe } if recipe == "r_black")
                )
            );
        }
    }
    assert_eq!(
        w.quick_quality(&c, "r_black"),
        c.handson.tuning.practised_quick_quality
    );
}

// ---------------------------------------------------------------------------------------------
// Saves & determinism
// ---------------------------------------------------------------------------------------------

#[test]
fn a_version_1_day_save_migrates_to_version_2() {
    let c = content();
    let text = include_str!("testdata/save_v1_story_day3.ron");
    assert!(text.starts_with("(version:1,"));
    let mut w = World::load_from_str(text).expect("v1 loads");
    assert_eq!(w.version, SAVE_VERSION);
    assert_eq!(w.phase(), Phase::Day);
    // duties from the old priority-1 jobs; Karolina has none
    for s in w.staff() {
        if s.owner {
            assert!(s.duties.is_empty());
            assert_eq!(s.priorities, [0; 5]);
        } else {
            assert!(!s.duties.is_empty(), "{} has a duty", s.name);
        }
    }
    w.rebuild_caches(&c);
    assert!(!w.knowledge().recipes.is_empty(), "the starters are known");
    assert_eq!(w.records_unlocked().len(), 3);
    assert!(
        super::music::jukebox(&w).is_some(),
        "grandma's jukebox moved in"
    );
    let mut b = bot::PlayerBot::new(bot::BotConfig {
        hire_nights: Vec::new(),
        ..Default::default()
    });
    b.run_night(&mut w, &c);
    assert!(w.last_summary().unwrap().karolina_items > 0);
    let again = w.save_to_string().unwrap();
    assert!(again.starts_with("(version:2,"));
    World::load_from_str(&again).unwrap();
    assert!(World::load_from_str("(version: 3, world: ())").is_err());
}

#[test]
fn a_version_1_mid_night_save_migrates_and_the_night_continues() {
    let c = content();
    let text = include_str!("testdata/save_v1_sandbox_night.ron");
    let mut w = World::load_from_str(text).expect("v1 loads");
    assert_eq!(w.phase(), Phase::Night);
    assert!(
        w.karolina().unwrap().task.is_none(),
        "the old autopilot task is dropped"
    );
    let mut b = bot::PlayerBot::new(bot::BotConfig {
        hire_nights: Vec::new(),
        ..Default::default()
    });
    let night = w.night();
    b.run_night(&mut w, &c);
    assert_eq!(w.phase(), Phase::Day);
    assert_eq!(w.night(), night + 1);
    assert!(
        super::music::jukebox(&w).is_some(),
        "the jukebox arrives the next day"
    );
    assert!(!w.knowledge().recipes.is_empty());
}

#[test]
fn same_seed_and_commands_give_the_same_save() {
    let c = content();
    let run = |seed: u64| -> (String, Vec<NightSummary>) {
        let mut w = World::new_story_seeded(&c, Difficulty::Cozy, seed);
        let mut b = bot::PlayerBot::new(bot::BotConfig {
            seed,
            hire_nights: vec![3],
            ..Default::default()
        });
        for _ in 0..3 {
            b.run_night(&mut w, &c);
        }
        (w.save_to_string().unwrap(), w.summaries().to_vec())
    };
    let a = run(5);
    assert_eq!(a, run(5));
    assert_ne!(a.1, run(6).1);
}

#[test]
fn a_mid_night_save_continues_identically_with_karolina_busy() {
    let c = content();
    let mut w = World::new_story_seeded(&c, Difficulty::Cozy, 11);
    let mut b = bot::PlayerBot::new(bot::BotConfig::default());
    b.day(&mut w, &c);
    bot::finish_cutscenes(&mut w, &c);
    w.start_night(&c);
    for _ in 0..1800 {
        bot::finish_cutscenes(&mut w, &c);
        b.step(&mut w, &c);
        w.tick(&c);
    }
    // a quiet moment is no test: go on until she's busy or guests are in
    for _ in 0..3000 {
        if w.player().current.is_some() || !w.parties().is_empty() {
            break;
        }
        bot::finish_cutscenes(&mut w, &c);
        b.step(&mut w, &c);
        w.tick(&c);
    }
    w.drain_events();
    assert!(w.player().current.is_some() || !w.parties().is_empty());
    let text = w.save_to_string().unwrap();
    let mut a = World::load_from_str(&text).unwrap();
    let mut x = World::load_from_str(&text).unwrap();
    for k in 0..1500u32 {
        if k == 100 {
            for v in [&mut a, &mut x] {
                v.apply(
                    &c,
                    Command::PlayerQueue {
                        act: PlayerAct::GoTo { tile: (8, 4) },
                        front: false,
                    },
                )
                .unwrap();
            }
        }
        a.tick(&c);
        x.tick(&c);
        bot::finish_cutscenes(&mut a, &c);
        bot::finish_cutscenes(&mut x, &c);
    }
    assert_eq!(a.save_to_string().unwrap(), x.save_to_string().unwrap());
}

// ---------------------------------------------------------------------------------------------
// The player-bot and the §9 targets (a short, deterministic slice of `sim_balance`)
// ---------------------------------------------------------------------------------------------

#[test]
fn the_player_bot_meets_the_first_week_targets_on_cozy() {
    let c = content();
    let (mut idle, mut minutes, mut parties, mut forgiven) = (0u32, 0u32, 0u32, 0u32);
    for seed in [7932u64, 15851] {
        let mut w = World::new_story_seeded(&c, Difficulty::Cozy, seed);
        let mut b = bot::PlayerBot::new(bot::BotConfig {
            seed,
            ..Default::default()
        });
        for night in 1..=5 {
            let ev = b.run_night(&mut w, &c);
            let s = w.last_summary().unwrap().clone();
            assert_eq!(s.guests_left_unhappy, 0, "night {night}");
            if night <= 2 {
                assert_eq!(s.helper_items, 0, "nights 1–2: Karolina alone");
                assert!(s.karolina_items >= 10, "night {night}: {s:?}");
            }
            assert_eq!(
                s.regular_items_karolina, s.regular_items,
                "regulars are Karolina's"
            );
            let bn = b.nights.last().unwrap();
            assert!(bn.same_kind_max <= 4, "{bn:?}");
            assert!(bn.real_minutes > 11.0 && bn.real_minutes < 17.0, "{bn:?}");
            idle += s.karolina_idle_minutes;
            minutes += 420;
            parties += ev
                .iter()
                .filter(|e| matches!(e, SimEvent::GuestArrived { .. }))
                .count() as u32;
            forgiven += ev
                .iter()
                .filter(|e| matches!(e, SimEvent::GuestForgiven { .. }))
                .count() as u32;
        }
    }
    let idle_share = idle as f32 / minutes as f32;
    assert!(idle_share <= 0.15, "idle share {idle_share}");
    let forgiven_share = forgiven as f32 / parties.max(1) as f32;
    assert!(forgiven_share <= 0.15, "forgiven share {forgiven_share}");
}
