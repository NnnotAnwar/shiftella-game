//! "Served twice" (v0.2.5 play report): with a barista and a floor helper on shift, a guest
//! whose order Karolina took got one cup from the crew (the barista brewed, the floor helper
//! carried) and then Karolina's own cup for the same order. Nothing is ever served twice: a
//! wish is the leader's whole order (the crew gets no plain ticket for them), and a round
//! gives each guest one drink (and at most one thing to eat).

use super::grid;
use super::guests;
use super::*;
use crate::content::handson::Duty;
use crate::content::{Content, MenuCategory};
use std::collections::{BTreeMap, BTreeSet};

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

fn content() -> Content {
    Content::load(DATA).unwrap_or_else(|e| panic!("{e}"))
}

fn is_drink(c: &Content, menu: &str) -> bool {
    c.menu_item(menu)
        .is_some_and(|m| matches!(m.category, MenuCategory::Drink | MenuCategory::Special))
}

/// The leader's wish is answered with a drink (a Food wish with a dish).
fn wish_is_drink(c: &Content, kind: &OrderKind) -> bool {
    !kind
        .wish()
        .and_then(|id| c.handson.wish(id))
        .is_some_and(|d| d.kind == crate::content::handson::WishKind::Food)
}

/// Helper `id`, experienced, with exactly these duties.
fn set_duties(w: &mut World, c: &Content, id: u32, duties: &[Duty]) {
    let i = w.staff.iter().position(|s| s.id == id).unwrap();
    w.staff[i].nights_worked = 5;
    w.staff[i].skills = [7.0; 5];
    for d in Duty::ALL {
        let _ = w.apply(
            c,
            Command::SetDuty {
                staff: id,
                duty: d,
                on: false,
            },
        );
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
}

/// A sandbox night with a barista (bar + kitchen) and a floor helper (serving + register) on
/// the floor; `quiet` = no random walk-ins. Returns (world, barista, floor helper).
fn crew_night(c: &Content, seed: u64, quiet: bool) -> (World, u32, u32) {
    let mut w = World::new_sandbox(
        c,
        SandboxSetup {
            seed,
            ..Default::default()
        },
    );
    while w.staff.iter().filter(|s| !s.owner).count() < 2 {
        let s = staff::generate_staff(&mut w, c, None);
        w.push_staff(s);
    }
    let helpers: Vec<u32> = w
        .staff
        .iter()
        .filter(|s| !s.owner)
        .map(|s| s.id)
        .take(2)
        .collect();
    set_duties(&mut w, c, helpers[0], &[Duty::Bar, Duty::Kitchen]);
    set_duties(&mut w, c, helpers[1], &[Duty::Serving, Duty::Register]);
    bot::finish_cutscenes(&mut w, c);
    w.start_night(c);
    bot::finish_cutscenes(&mut w, c);
    w.skip_prep(c);
    if quiet {
        w.mods.push(Modifier {
            kind: ModKind::GuestRate,
            mul: 0.0,
            until: None,
            event: None,
        });
    }
    // on the floor now, at the door
    let door = grid::tile_center(w.grid.entrance());
    for s in w.staff.iter_mut() {
        if helpers.contains(&s.id) {
            s.status = StaffStatus::OnDuty;
            s.pos = door;
            s.path.clear();
        }
    }
    w.drain_events();
    (w, helpers[0], helpers[1])
}

/// Items served since the last call: (item id, guest, menu).
fn newly_served(w: &World, seen: &mut BTreeSet<u32>) -> Vec<(u32, u32, String)> {
    w.items
        .iter()
        .filter(|i| i.state == ItemState::Served && seen.insert(i.id))
        .map(|i| (i.id, i.guest, i.menu.clone()))
        .collect()
}

#[test]
fn a_wish_takes_the_place_of_the_leaders_drink_so_the_crew_gets_no_cup_for_them() {
    let c = content();
    let (mut w, _, _) = crew_night(&c, 3, true);
    let mut kept_sides = 0;
    for size in [1u32, 1, 1, 1, 1, 1, 2, 2, 3, 3, 4] {
        let pid = guests::spawn_party(
            &mut w,
            &c,
            "travelers",
            size,
            None,
            1.0,
            1.0,
            &[],
            None,
            true,
        )
        .expect("spawned");
        let p = w.parties.iter().find(|p| p.id == pid).unwrap();
        assert!(p.order_kind.is_wish());
        let wish_drink = wish_is_drink(&c, &p.order_kind);
        // the others: one drink and maybe something to eat each; the leader: the wish and
        // maybe something of the other kind with it
        let same = p
            .wishes
            .iter()
            .filter(|m| is_drink(&c, m) == wish_drink)
            .count();
        assert!(
            same < size as usize,
            "size {size}: the wish is the leader's {}: {:?}",
            if wish_drink { "drink" } else { "dish" },
            p.wishes
        );
        assert!(p.wishes.len() < 2 * size as usize, "{:?}", p.wishes);
        if size == 1 {
            kept_sides += p.wishes.len();
        }
    }
    assert!(
        kept_sides > 0,
        "a solo wish guest may still eat something with it"
    );
}

#[test]
fn a_round_gives_each_guest_one_drink() {
    let c = content();
    let (mut w, _, _) = crew_night(&c, 5, true);
    let pid = guests::spawn_party(&mut w, &c, "students", 2, None, 1.0, 1.0, &[], None, false)
        .expect("spawned");
    {
        let p = w.parties.iter_mut().find(|p| p.id == pid).unwrap();
        p.order_kind = OrderKind::Plain;
        p.wish_open = false;
        // what pick_wishes makes for two: a toast with a coffee, and a coffee
        p.wishes = vec![
            "topinka".into(),
            "black_coffee".into(),
            "black_coffee".into(),
        ];
    }
    guests::settle_now(&mut w, pid);
    let me = w.karolina().unwrap().id;
    guests::order_taken(&mut w, &c, pid, me);
    let members = w
        .parties
        .iter()
        .find(|p| p.id == pid)
        .unwrap()
        .members
        .clone();
    let mut per: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    for i in w.items.iter().filter(|i| i.party == pid) {
        per.entry(i.guest).or_default().push(i.menu.clone());
    }
    assert_eq!(per.values().map(Vec::len).sum::<usize>(), 3);
    for g in &members {
        let v = per.get(g).cloned().unwrap_or_default();
        let drinks = v.iter().filter(|m| is_drink(&c, m)).count();
        assert_eq!(drinks, 1, "guest {g} gets one coffee: {per:?}");
    }
}

/// The play report, step by step: Karolina takes a wish order while the barista and the floor
/// helper stand idle and eager, then brews the wish. The guest gets exactly one cup: hers.
#[test]
fn karolina_brews_a_wish_and_the_crew_never_brings_a_second_cup() {
    let c = content();
    let (mut w, barista, floor) = crew_night(&c, 7, true);
    let me = w.karolina().unwrap().id;
    let mut seen = BTreeSet::new();
    for round in 0..6 {
        let pid = guests::spawn_party(&mut w, &c, "travelers", 1, None, 1.0, 1.0, &[], None, true)
            .expect("spawned");
        guests::settle_now(&mut w, pid);
        let leader = w
            .parties
            .iter()
            .find(|p| p.id == pid)
            .unwrap()
            .leader()
            .unwrap();
        w.apply(&c, Command::PlayerTakeOrder { guest: leader })
            .unwrap();
        let mut brewed = false;
        let mut served: Vec<(u32, u32, String)> = Vec::new();
        let mut crew_served = 0;
        for _ in 0..4000 {
            w.tick(&c);
            bot::finish_cutscenes(&mut w, &c);
            for e in w.drain_events() {
                if let SimEvent::OrderServed { party, staff, .. } = e
                    && party == pid
                    && (staff == barista || staff == floor)
                {
                    crew_served += 1;
                }
            }
            served.extend(newly_served(&w, &mut seen));
            let p = w.parties.iter().find(|p| p.id == pid).unwrap();
            let (state, wish_open) = (p.state, p.wish_open);
            if !brewed && state == PartyState::WaitFood && wish_open {
                let (recipe, _) = w.best_matches(&c, leader).first().cloned().unwrap();
                w.apply(
                    &c,
                    Command::PlayerBrew {
                        guest: leader,
                        recipe,
                        quality: 0.9,
                        mode: BrewMode::ByHand,
                    },
                )
                .unwrap();
                brewed = true;
            }
            if brewed && matches!(state, PartyState::Eating | PartyState::Leaving) {
                // a little longer: a second cup would be on its way by now
                for _ in 0..600 {
                    w.tick(&c);
                    bot::finish_cutscenes(&mut w, &c);
                    for e in w.drain_events() {
                        if let SimEvent::OrderServed { party, staff, .. } = e
                            && party == pid
                            && (staff == barista || staff == floor)
                        {
                            crew_served += 1;
                        }
                    }
                    served.extend(newly_served(&w, &mut seen));
                }
                break;
            }
        }
        assert!(brewed, "round {round}: she brewed the wish");
        // (served items stay as the night's history after the party left)
        let wish = w.items.iter().find(|i| i.party == pid && i.wish).unwrap();
        assert!(wish.origin.by_player() && wish.by == Some(me));
        assert_eq!(wish.state, ItemState::Served);
        let wish_drink = is_drink(&c, &wish.menu);
        let mine: Vec<_> = served.iter().filter(|s| s.1 == leader).collect();
        let drinks = mine.iter().filter(|s| is_drink(&c, &s.2)).count();
        assert!(drinks <= 1, "round {round}: one cup: {served:?}");
        // the crew may bring something to eat with it, never a second cup of the wish's kind
        // nor the same dish again
        for s in &mine {
            if s.0 != wish.id {
                assert_ne!(s.2, wish.menu, "round {round}: {served:?}");
                if wish_drink {
                    assert!(!is_drink(&c, &s.2), "round {round}: {served:?}");
                }
            }
        }
        assert!(crew_served <= 1, "round {round}: crew served {crew_served}");
    }
}

/// An order that still has a coffee for the leader beside the wish (a save from before the
/// fix, or the leader's drink side): the barista picks it up, then Karolina brews the wish
/// with a coffee — the barista drops theirs, the guest gets one cup, its supplies come back.
#[test]
fn karolina_brewing_the_wish_takes_the_leaders_cup_off_the_crew() {
    let c = content();
    let (mut w, barista, _) = crew_night(&c, 11, true);
    let pid = guests::spawn_party(&mut w, &c, "travelers", 1, None, 1.0, 1.0, &[], None, true)
        .expect("spawned");
    {
        let p = w.parties.iter_mut().find(|p| p.id == pid).unwrap();
        p.order_kind = OrderKind::Wish {
            wish: "long_day".into(),
        };
        p.wishes = vec!["cappuccino".into()];
    }
    guests::settle_now(&mut w, pid);
    let leader = w
        .parties
        .iter()
        .find(|p| p.id == pid)
        .unwrap()
        .leader()
        .unwrap();
    w.apply(&c, Command::PlayerTakeOrder { guest: leader })
        .unwrap();
    // the order is taken and the barista is on the cappuccino
    let mut on_it = false;
    for _ in 0..3000 {
        w.tick(&c);
        bot::finish_cutscenes(&mut w, &c);
        if w.items
            .iter()
            .any(|i| i.party == pid && i.menu == "cappuccino" && i.by == Some(barista))
        {
            on_it = true;
            break;
        }
    }
    assert!(
        on_it,
        "the barista took the leader's cappuccino: {:?} {:?} {:?}",
        w.items
            .iter()
            .filter(|i| i.party == pid)
            .collect::<Vec<_>>(),
        w.staff
            .iter()
            .map(|s| (s.id, s.status, s.task.clone()))
            .collect::<Vec<_>>(),
        w.parties.iter().find(|p| p.id == pid).map(|p| p.state)
    );
    let side = w
        .items
        .iter()
        .find(|i| i.party == pid && i.menu == "cappuccino")
        .unwrap()
        .id;
    let supplies = w.econ.supplies;
    w.apply(
        &c,
        Command::PlayerBrew {
            guest: leader,
            recipe: "r_black".into(),
            quality: 0.9,
            mode: BrewMode::Quick,
        },
    )
    .unwrap();
    let it = w.items.iter().find(|i| i.id == side).unwrap();
    assert_eq!(it.state, ItemState::Cancelled, "the crew's cup is off");
    let black = c.menu_item("black_coffee").unwrap().cost as i64;
    let capp = c.menu_item("cappuccino").unwrap().cost as i64;
    assert_eq!(w.econ.supplies, supplies - black + capp);
    let mut seen = BTreeSet::new();
    let mut served = Vec::new();
    for _ in 0..3000 {
        w.tick(&c);
        bot::finish_cutscenes(&mut w, &c);
        served.extend(newly_served(&w, &mut seen));
    }
    let mine: Vec<_> = served.iter().filter(|s| s.1 == leader).collect();
    assert_eq!(mine.len(), 1, "one cup: {served:?}");
    assert_eq!(mine[0].2, "black_coffee");
    let bi = w.staff.iter().position(|s| s.id == barista).unwrap();
    assert!(
        !matches!(w.staff[bi].task, Some(Task::Prepare { item, .. }) if item == side),
        "the barista let it go"
    );
}

/// A whole night with an eager Karolina (she takes every order and brews every card) beside a
/// barista and a floor helper: no guest ever gets two drinks from one round.
#[test]
fn a_busy_night_with_the_crew_serves_nobody_twice() {
    let c = content();
    for seed in 1..=3u64 {
        let (mut w, _, _) = crew_night(&c, seed, false);
        let mut seen = BTreeSet::new();
        // (guest, party) → drinks served
        let mut drinks: BTreeMap<(u32, u32), u32> = BTreeMap::new();
        let mut rounds: BTreeMap<u32, u32> = BTreeMap::new();
        let mut crew_made = 0;
        for t in 0..6000u32 {
            if w.phase() != Phase::Night {
                break;
            }
            w.tick(&c);
            bot::finish_cutscenes(&mut w, &c);
            w.drain_events();
            for (id, g, menu) in newly_served(&w, &mut seen) {
                let it = w.items.iter().find(|i| i.id == id).unwrap();
                if matches!(it.origin, ItemOrigin::Helper { .. }) {
                    crew_made += 1;
                }
                if is_drink(&c, &menu) {
                    let party = it.party;
                    *drinks.entry((g, party)).or_default() += 1;
                }
            }
            for p in &w.parties {
                rounds.insert(p.id, p.rounds as u32);
            }
            if !t.is_multiple_of(5) {
                continue;
            }
            let parties: Vec<(u32, PartyState, Vec<u32>)> = w
                .parties
                .iter()
                .map(|p| (p.id, p.state, p.members.clone()))
                .collect();
            for (pid, st, members) in parties {
                match st {
                    PartyState::WaitOrder | PartyState::Queue => {
                        let _ = w.apply(&c, Command::PlayerTakeOrder { guest: pid });
                    }
                    PartyState::WaitFood => {
                        for g in members {
                            if let Some((r, _)) = w.best_matches(&c, g).first().cloned() {
                                let mode = if t.is_multiple_of(2) {
                                    BrewMode::Quick
                                } else {
                                    BrewMode::ByHand
                                };
                                let _ = w.apply(
                                    &c,
                                    Command::PlayerBrew {
                                        guest: g,
                                        recipe: r,
                                        quality: 0.8,
                                        mode,
                                    },
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        assert!(!drinks.is_empty(), "seed {seed}: drinks were served");
        assert!(crew_made > 0, "seed {seed}: the crew worked too");
        for ((g, party), n) in &drinks {
            // "the same again" rounds bring the leader another cup
            let extra = rounds.get(party).copied().unwrap_or(0);
            assert!(
                *n <= 1 + extra,
                "seed {seed}: guest {g} (party {party}) got {n} drinks in {} round(s)",
                1 + extra
            );
        }
    }
}
