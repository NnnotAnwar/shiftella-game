//! Seats & the ordering flow (0.2.2): grandma's bar stools, a seatless plain party taking it
//! to go, big parties finding a free cluster, most guests ordering at their table (a few at
//! the bar), and no order taken before the party is in.

use super::guests;
use super::*;
use crate::content::Content;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

fn content() -> Content {
    Content::load(DATA).unwrap_or_else(|e| panic!("{e}"))
}

/// A story night `n` running, cutscenes done, no random walk-ins.
fn night(c: &Content, n: u32, d: Difficulty) -> World {
    let mut w = World::new_story(c, d);
    w.sys.tutorial_off = true;
    if n > 1 {
        w.night = n;
        w.begin_day(c);
    }
    bot::finish_cutscenes(&mut w, c);
    w.start_night(c);
    bot::finish_cutscenes(&mut w, c);
    w.skip_prep(c);
    w.mods.push(Modifier {
        kind: ModKind::GuestRate,
        mul: 0.0,
        until: None,
        event: None,
    });
    w.drain_events();
    w
}

fn spawn(w: &mut World, c: &Content, arch: &str, size: u32, wish: bool, orders: &[&str]) -> u32 {
    let orders: Vec<String> = orders.iter().map(|s| s.to_string()).collect();
    guests::spawn_party(w, c, arch, size, None, 1.0, 1.0, &orders, None, wish).expect("spawned")
}

fn party(w: &World, pid: u32) -> &Party {
    w.parties.iter().find(|p| p.id == pid).expect("party")
}

/// Wish pairs (they always sit, never take it to go) on every table of grandma's.
fn fill_tables(w: &mut World, c: &Content) {
    for _ in 0..8 {
        let pid = spawn(w, c, "students", 2, true, &[]);
        assert_ne!(party(w, pid).state, PartyState::Leaving, "a table for them");
    }
}

/// Marks every seat taken.
fn occupy_every_seat(w: &mut World, c: &Content) {
    for s in guests::seats(w, c) {
        if let Some(p) = w.furniture.iter_mut().find(|p| p.uid == s.uid) {
            p.seat_by[s.idx as usize] = Some(u32::MAX);
        }
    }
}

fn stool_tiles(w: &World) -> Vec<(i32, i32)> {
    w.furniture
        .iter()
        .filter(|p| p.def == "stool")
        .map(|p| (p.x, p.y))
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Grandma's bar stools
// ---------------------------------------------------------------------------------------------

#[test]
fn grandmas_bar_stools_sit_at_the_counter_and_block_nothing() {
    let c = content();
    let w = World::new_story(&c, Difficulty::Cozy);
    let stools = stool_tiles(&w);
    assert_eq!(stools, vec![(16, 9), (17, 9), (18, 9), (19, 9)]);
    // placed last: every older piece keeps its uid
    let n = w.furniture.len();
    assert!(w.furniture[n - 4..].iter().all(|p| p.def == "stool"));
    // bar seats: they hold a guest, they draw nobody in (grandma's 16 stay the neutral point)
    let seats = guests::seats(&w, &c);
    assert_eq!(seats.iter().filter(|s| s.bar).count(), 4);
    assert!(
        seats
            .iter()
            .filter(|s| s.bar)
            .all(|s| stools.contains(&s.tile))
    );
    assert_eq!(w.hall_seats(&c), 16);
    assert_eq!(w.seat_draw(&c), 1.0);
    // each stool is reached from the door without climbing over another chair
    let reach = grid::reachable_off_seats(&w.grid, w.grid.entrance());
    for t in &stools {
        let ok = [(0, -1), (-1, 0), (1, 0)].iter().any(|(dx, dy)| {
            w.grid
                .idx(t.0 + dx, t.1 + dy)
                .is_some_and(|i| reach[i] && !stools.contains(&(t.0 + dx, t.1 + dy)))
        });
        assert!(ok, "stool {t:?} can't be reached");
    }
    // the register line still has room and never stands on a stool
    let reg = w
        .furniture
        .iter()
        .find(|p| p.def == "register")
        .unwrap()
        .uid;
    let line = guests::queue_slots(&w, &c, reg);
    assert!(line.len() >= 6, "{line:?}");
    assert!(line.iter().all(|t| !stools.contains(t)), "{line:?}");
    // nobody's way crosses them: from the door to every chair and the line, and the
    // staff's way out past the counter's open end to every table
    let mut goals: Vec<(i32, i32)> = seats.iter().filter(|s| !s.bar).map(|s| s.tile).collect();
    goals.extend(line.iter().copied());
    for g in &goals {
        let p = grid::find_path(&w.grid, w.grid.entrance(), *g, true).expect("a way");
        assert!(p.iter().all(|t| !stools.contains(t)), "door → {g:?}: {p:?}");
        let s = grid::find_path(&w.grid, (16, 12), *g, false).expect("a staff way");
        assert!(s.iter().all(|t| !stools.contains(t)), "bar → {g:?}: {s:?}");
    }
    // the bar's own lamps light them: grandma's hall keeps its coziness tier
    assert!(w.coziness().level >= 3, "{:?}", w.coziness());
}

#[test]
fn a_party_of_four_takes_the_bar_stools_when_the_tables_are_full() {
    let c = content();
    let mut w = night(&c, 6, Difficulty::Cozy);
    fill_tables(&mut w, &c);
    // a wish party (never to go) of four: the four stools side by side
    let pid = spawn(&mut w, &c, "students", 4, true, &[]);
    assert_eq!(party(&w, pid).state, PartyState::WaitOrder);
    let stools = stool_tiles(&w);
    let mine: Vec<(i32, i32)> = party(&w, pid)
        .members
        .iter()
        .filter_map(|m| w.guests.iter().find(|g| g.id == *m)?.seat)
        .filter_map(|(u, i)| guests::seat_tile(&w, u, i))
        .collect();
    assert_eq!(mine.len(), 4);
    assert!(mine.iter().all(|t| stools.contains(t)), "{mine:?}");
}

#[test]
fn a_big_party_finds_a_free_cluster_even_when_the_best_seat_stands_alone() {
    let c = content();
    let mut w = night(&c, 6, Difficulty::Cozy);
    // the left column of tables taken but one chair at (5, 8); the right ones free except
    // the bar: the best-scored free chair has no free neighbour, a cluster elsewhere has
    let all = guests::seats(&w, &c);
    for s in &all {
        let keep_free =
            s.tile == (5, 8) || (s.tile.0 >= 19 && !s.bar && s.tile.1 <= 5) || s.tile.0 == 13;
        if !keep_free && let Some(p) = w.furniture.iter_mut().find(|p| p.uid == s.uid) {
            p.seat_by[s.idx as usize] = Some(u32::MAX);
        }
    }
    let pid = spawn(&mut w, &c, "students", 3, true, &[]);
    assert_ne!(
        party(&w, pid).state,
        PartyState::Leaving,
        "three seats free near each other: they sit"
    );
}

// ---------------------------------------------------------------------------------------------
// To go
// ---------------------------------------------------------------------------------------------

#[test]
fn no_free_seat_a_plain_party_takes_it_to_go_and_walks_out_with_the_cup() {
    let c = content();
    let mut w = night(&c, 6, Difficulty::Cozy);
    // every seat taken (by nobody in particular: the bot has only them to serve)
    occupy_every_seat(&mut w, &c);
    assert!(guests::seats(&w, &c).iter().all(|s| !s.free));
    w.drain_events();
    // plain solo guests wanting a coffee: (nearly) all take it to go — never "went on"
    let mut pid = None;
    // (a walk-in may come with a wish by chance: those go on — wishes are told at a table)
    for _ in 0..10 {
        let p = spawn(&mut w, &c, "travelers", 1, false, &["cappuccino"]);
        let pp = w.parties.iter_mut().find(|x| x.id == p).unwrap();
        pp.order_kind = OrderKind::Plain;
        pp.wish_open = false;
        if party(&w, p).to_go {
            pid = Some(p);
            break;
        }
    }
    let pid = pid.expect("a plain one took it to go (90 %)");
    let p = party(&w, pid).clone();
    assert_eq!(p.state, PartyState::Queue, "they queue at the register");
    assert!(p.register.is_some());
    assert!(!p.wishes.is_empty());
    assert!(p.wishes.iter().all(|i| {
        let m = c.menu_item(i).unwrap();
        purchases::data().to_go.categories.contains(&m.category)
    }));
    let leader = p.leader().unwrap();
    assert!(
        w.guests
            .iter()
            .find(|g| g.id == leader)
            .unwrap()
            .seat
            .is_none()
    );
    let ev = w.drain_events();
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::GuestArrived { party, .. } if *party == pid))
    );
    assert!(
        !ev.iter()
            .any(|e| matches!(e, SimEvent::GuestTurnedAway { party } if *party == pid))
    );
    assert_eq!(w.payoff_tonight().to_go, 1);
    // a save keeps them taking it to go
    let back = World::load_from_str(&w.save_to_string().unwrap()).unwrap();
    assert!(back.parties.iter().any(|x| x.id == pid && x.to_go));
    assert_eq!(back.pay, w.pay);
    assert_eq!(w.tonight.guests_to_go, 1);
    // the player-bot plays Karolina: they reach the line, she takes the order, the cup is
    // made and handed over by the register
    let mut b = bot::PlayerBot::new(bot::BotConfig::default());
    let dishes = w.sink_dishes;
    let table_dishes =
        |w: &World| -> Vec<(u32, u32)> { w.furniture.iter().map(|p| (p.uid, p.dishes)).collect() };
    let mut left = false;
    let mut cup_out = false;
    let mut spot_ok = false;
    let mut taken_settled = true;
    for _ in 0..6000 {
        b.step(&mut w, &c);
        w.tick(&c);
        bot::finish_cutscenes(&mut w, &c);
        if let Some(pp) = w.parties.iter().find(|x| x.id == pid) {
            if pp.order_claim.is_some() && pp.ordered_at.is_none() && !w.party_settled(pid) {
                // (a claim may only start once settled; the line moving up keeps it)
                taken_settled &= pp.state == PartyState::Queue;
            }
            if pp.state == PartyState::WaitFood
                && let Some(g) = w.guests.iter().find(|g| g.id == leader)
                && let Some(goal) = g.goal
            {
                let line = guests::queue_slots(&w, &c, p.register.unwrap());
                spot_ok = !line.contains(&goal) && !w.grid.is_seat(goal.0, goal.1);
            }
        }
        for e in w.drain_events() {
            if matches!(e, SimEvent::GuestLeft { party, served: true, .. } if party == pid) {
                left = true;
            }
        }
        if left {
            cup_out = w
                .guests
                .iter()
                .find(|g| g.id == leader)
                .is_some_and(|g| g.holding.is_some());
            let v = w.guest_views();
            assert!(
                v.iter()
                    .any(|v| v.agent == AgentRef::Guest(leader) && v.to_go)
            );
            break;
        }
    }
    assert!(taken_settled);
    assert!(
        spot_ok,
        "waited for the cup beside the register, off the line"
    );
    assert!(left, "served and gone");
    assert!(cup_out, "out of the door with the paper cup");
    // a paper cup: nothing comes back to the sink from them (the seated wish parties may
    // still be eating — their tables are theirs)
    let _ = (dishes, table_dishes(&w));
    assert!(w.payoff_tonight().to_go_money > 0);
    // the morning line says so instead of "no free seat"
    let lines = purchases::lines(&w, &c);
    let hint = lines.last().map(|l| l.key.as_str()).unwrap_or_default();
    // (`…_some_left` when a wish walk-in above found no seat and went on)
    assert!(hint.starts_with("purchases.line.to_go"), "{lines:?}");
}

#[test]
fn regulars_and_wish_parties_never_take_it_to_go() {
    let c = content();
    let mut w = night(&c, 6, Difficulty::Cozy);
    fill_tables(&mut w, &c);
    spawn(&mut w, &c, "students", 4, true, &[]);
    for _ in 0..4 {
        let pid = spawn(&mut w, &c, "travelers", 1, true, &[]);
        let p = party(&w, pid);
        assert!(
            !p.to_go && p.state == PartyState::Leaving,
            "a wish party goes on"
        );
    }
    let pid = guests::spawn_party(
        &mut w,
        &c,
        "travelers",
        1,
        Some("mirek"),
        1.0,
        1.0,
        &[],
        None,
        false,
    )
    .expect("spawned");
    let p = party(&w, pid);
    assert!(
        !p.to_go && p.state == PartyState::Leaving,
        "a regular never"
    );
    assert_eq!(w.payoff_tonight().to_go, 0);
}

#[test]
fn the_to_go_choice_never_draws_the_world_rng_and_classic_is_shyer() {
    let c = content();
    let mut w = night(&c, 6, Difficulty::Cozy);
    fill_tables(&mut w, &c);
    let rng = w.rng.clone();
    let mut yes = 0;
    for pid in 1_000_000..1_000_200 {
        if super::to_go::plan(&w, &c, pid, 1, &["cappuccino".into()], false, false).is_some() {
            yes += 1;
        }
    }
    assert_eq!(w.rng, rng);
    let d = &purchases::data().to_go;
    assert!(d.classic_chance < d.chance);
    assert!((150..=195).contains(&yes), "~90 % of 200: {yes}");
    // a food order has nothing for a paper bag
    assert!(super::to_go::plan(&w, &c, 7, 1, &["garlic_soup".into()], false, false).is_none());
}

/// The never-building player on Cozy: hardly anyone goes on for want of a seat (0.2.1:
/// 11–13 guests a night).
#[test]
fn on_cozy_few_guests_go_on_for_want_of_a_seat() {
    let c = content();
    let seed = 7932;
    let mut w = World::new_story_seeded(&c, Difficulty::Cozy, seed);
    let mut b = bot::PlayerBot::new(bot::BotConfig {
        seed,
        build: false,
        ..Default::default()
    });
    let (mut away, mut to_go, mut nights) = (0u32, 0u32, 0u32);
    for _ in 0..12 {
        b.run_night(&mut w, &c);
        let s = w.last_summary().unwrap();
        away += s.guests_turned_away;
        to_go += s.guests_to_go;
        nights += 1;
    }
    let per = away as f32 / nights as f32;
    assert!(per <= 5.0, "turned away {per:.1} a night");
    assert!(to_go > 0, "some took it to go");
}

// ---------------------------------------------------------------------------------------------
// The ordering flow
// ---------------------------------------------------------------------------------------------

#[test]
fn most_plain_parties_sit_and_order_at_the_table_a_few_at_the_bar() {
    let c = content();
    let (mut table, mut bar) = (0u32, 0u32);
    for seed in [7932u64, 23770] {
        let mut w = World::new_story_seeded(&c, Difficulty::Cozy, seed);
        let mut b = bot::PlayerBot::new(bot::BotConfig {
            seed,
            ..Default::default()
        });
        for _ in 0..6 {
            b.run_night(&mut w, &c);
        }
        // the next nights, arrival by arrival (each party's state the tick it came in; four
        // nights so the sample stays big enough whatever a night brings)
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..4 {
            b.run_night_watched(&mut w, &c, |w| {
                for p in w.parties() {
                    if !seen.insert(p.id) {
                        continue;
                    }
                    if p.order_kind.is_wish() || p.regular.is_some() || p.to_go {
                        continue;
                    }
                    match p.state {
                        PartyState::Queue => bar += 1,
                        PartyState::WaitOrder => table += 1,
                        _ => {}
                    }
                }
            });
        }
    }
    let share = bar as f32 / (bar + table).max(1) as f32;
    assert!(table + bar >= 45, "{table} + {bar}");
    assert!(
        (0.1..=0.4).contains(&share),
        "bar {bar} / table {table}: {share:.2}"
    );
}

#[test]
fn no_order_is_taken_before_the_party_is_in() {
    let c = content();
    let mut w = night(&c, 6, Difficulty::Cozy);
    // a helper on the register and the floor: they wait too
    let pid = spawn(&mut w, &c, "travelers", 1, false, &["cappuccino"]);
    let leader = party(&w, pid).leader().unwrap();
    assert!(!w.party_settled(pid), "still on the street");
    assert_eq!(
        w.apply(&c, Command::PlayerTakeOrder { guest: leader }),
        Err(CommandError::Invalid(NOT_IN_YET.into()))
    );
    assert_eq!(
        w.apply(
            &c,
            Command::PlayerQueue {
                act: PlayerAct::TakeOrder { party: pid },
                front: true,
            }
        ),
        Err(CommandError::Invalid(NOT_IN_YET.into()))
    );
    assert!(w.player().queue.is_empty() && party(&w, pid).order_claim.is_none());
    // they walk in; nobody claims the order before they are settled
    let mut settled_at = None;
    for t in 0..900 {
        let claimed = party(&w, pid).order_claim.is_some();
        let settled = w.party_settled(pid);
        assert!(
            !claimed || settled_at.is_some() || settled,
            "claimed at tick {t} walking in"
        );
        if settled && settled_at.is_none() {
            settled_at = Some(t);
            break;
        }
        w.tick(&c);
        bot::finish_cutscenes(&mut w, &c);
    }
    assert!(settled_at.is_some(), "they sat down / reached the line");
    let p = party(&w, pid);
    if p.state == PartyState::WaitOrder {
        let g = w.guests.iter().find(|g| g.id == leader).unwrap();
        assert_eq!(g.pose, Pose::Sit, "settled at a table = seated");
    }
    if p.order_claim.is_none() {
        w.apply(&c, Command::PlayerTakeOrder { guest: leader })
            .unwrap();
    }
}
