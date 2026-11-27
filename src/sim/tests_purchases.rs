//! Purchases that matter (`sim::purchases`): seats cap and draw guests, comfort lengthens the
//! stay and raises tips, coziness tiers unlock their perks, a second machine brews in
//! parallel, the appliances' roles act by tag, the morning's payoff lines add up — and a
//! building player earns noticeably more than one who never builds, while the story still
//! ends in the light without building.

use super::*;
use crate::content::Content;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

fn content() -> Content {
    Content::load(DATA).unwrap_or_else(|e| panic!("{e}"))
}

/// The day of story night `n` (cutscenes done, money for building).
fn day(c: &Content, n: u32) -> World {
    let mut w = World::new_story(c, Difficulty::Cozy);
    w.sys.tutorial_off = true;
    if n > 1 {
        w.night = n;
        w.begin_day(c);
    }
    bot::finish_cutscenes(&mut w, c);
    w.econ.money = 100_000;
    w.drain_events();
    w
}

/// Day → the doors open, no random walk-ins.
fn open(w: &mut World, c: &Content) {
    w.start_night(c);
    bot::finish_cutscenes(w, c);
    w.skip_prep(c);
    w.mods.push(Modifier {
        kind: ModKind::GuestRate,
        mul: 0.0,
        until: None,
        event: None,
    });
    w.drain_events();
}

fn place(w: &mut World, c: &Content, def: &str, x: i32, y: i32, rot: u8) -> u32 {
    w.apply(
        c,
        Command::PlaceFurniture {
            def: def.into(),
            x,
            y,
            rot,
        },
    )
    .unwrap_or_else(|e| panic!("{def} at {x},{y}: {e:?}"));
    w.furniture().last().unwrap().uid
}

fn spawn(w: &mut World, c: &Content, size: u32) -> u32 {
    guests::spawn_party(w, c, "students", size, None, 1.0, 1.0, &[], None, false).unwrap()
}

fn pi_of(w: &World, pid: u32) -> usize {
    w.parties.iter().position(|p| p.id == pid).unwrap()
}

#[test]
fn the_data_parses_and_grandmas_cafe_is_the_neutral_point() {
    let c = content();
    let d = purchases::parse(include_str!("../../assets/data/purchases.ron")).unwrap();
    assert_eq!(d.tiers.len(), 5);
    let w = World::new_story(&c, Difficulty::Cozy);
    assert_eq!(w.hall_seats(&c), d.seats.base);
    assert_eq!(w.seat_draw(&c), 1.0);
    let chair = purchases::comfort_at(c.furniture_def("chair").unwrap().comfort);
    assert_eq!(
        (chair.stay, chair.tips, chair.round, chair.satisfaction),
        (1.0, 1.0, 0.0, 0.0)
    );
    // every role hangs on a tag some piece carries
    for a in &d.appliances {
        assert!(
            c.furniture.iter().any(|f| f.has_tag(&a.tag)),
            "no piece has the tag {}",
            a.tag
        );
    }
    // the low tiers are plain: a café that never builds loses nothing
    assert!(purchases::tier_of(1).is_plain() && purchases::tier_of(2).is_plain());
}

// ---------------------------------------------------------------------------------------------
// Seats → guests
// ---------------------------------------------------------------------------------------------

#[test]
fn more_seats_draw_more_guests_with_diminishing_returns() {
    let c = content();
    let (a, b, d) = (
        purchases::draw_for(16),
        purchases::draw_for(24),
        purchases::draw_for(32),
    );
    assert!(a < b && b < d, "{a} {b} {d}");
    assert!(d - b < b - a, "diminishing: {a} {b} {d}");
    assert!(purchases::draw_for(200) <= purchases::data().seats.max_draw);
    let mut w = day(&c, 6);
    place(&mut w, &c, "sofa", 24, 3, 0);
    place(&mut w, &c, "window_seat", 9, 1, 0);
    assert_eq!(w.hall_seats(&c), 20);
    assert!(w.seat_draw(&c) > 1.05);
    // the card says so before buying
    let e = w.piece_effect(&c, c.furniture_def("sofa").unwrap());
    let (now, with, more) = e.seats.unwrap();
    assert_eq!((now, with), (20, 22));
    assert!(more >= 1, "about +{more} guests");
}

#[test]
fn a_full_cafe_turns_parties_away_with_one_gentle_toast_and_more_seats_keep_them() {
    let c = content();
    let run = |extra: bool| {
        let mut w = day(&c, 6);
        if extra {
            place(&mut w, &c, "sofa", 24, 3, 0);
            place(&mut w, &c, "sofa", 24, 6, 0);
            place(&mut w, &c, "window_seat", 9, 1, 0);
        }
        open(&mut w, &c);
        let mut away = 0;
        let mut toasts = 0;
        for _ in 0..14 {
            spawn(&mut w, &c, 2);
            for e in w.drain_events() {
                match e {
                    SimEvent::GuestTurnedAway { .. } => away += 1,
                    SimEvent::Toast { key } if key == "purchases.toast.no_seat" => toasts += 1,
                    _ => {}
                }
            }
        }
        let extra_parties = w.parties.iter().filter(|p| p.extra_seat).count();
        (away, toasts, w.payoff_tonight().turned_away, extra_parties)
    };
    let (away, toasts, tally, extra) = run(false);
    assert!(away >= 5, "8 tables for 14 pairs: {away} went on");
    assert_eq!(toasts, 1, "one toast, not one per party");
    assert_eq!(tally, away);
    assert_eq!(extra, 0);
    let (away2, _, _, extra2) = run(true);
    assert_eq!(away2 + 3, away, "three more pieces seat three more pairs");
    // (the first pairs pick the comfy pieces while grandma's tables are still free; the ones
    // seated once hers were full are the extra guests)
    assert!(
        (1..=3).contains(&extra2),
        "{extra2} found room only on what was bought"
    );
}

// ---------------------------------------------------------------------------------------------
// Comfort → stay, tips, a second round
// ---------------------------------------------------------------------------------------------

#[test]
fn comfy_seats_lengthen_the_stay_and_raise_tips_hard_stools_the_opposite() {
    let c = content();
    let sofa = purchases::comfort_at(6.0);
    let stool = purchases::comfort_at(1.0);
    assert!(sofa.stay > 1.2 && sofa.tips > 1.05 && sofa.round > 0.2 && sofa.satisfaction > 0.0);
    assert!(stool.stay < 1.0 && stool.tips < 1.0 && stool.round == 0.0 && stool.satisfaction < 0.0);
    // a pair walks in: the sofa beats grandma's chairs
    let mut w = day(&c, 6);
    let uid = place(&mut w, &c, "sofa", 24, 3, 0);
    open(&mut w, &c);
    let pid = spawn(&mut w, &c, 2);
    let pi = pi_of(&w, pid);
    let seats: Vec<u32> = w.parties[pi]
        .members
        .iter()
        .filter_map(|m| w.guests.iter().find(|g| g.id == *m)?.seat.map(|s| s.0))
        .collect();
    assert_eq!(seats, vec![uid, uid], "they take the sofa");
    let leader = w.parties[pi].members[0];
    assert_eq!(
        w.seat_feel(&c, leader).map(|f| f.0),
        Some(purchases::SeatFeel::Comfy)
    );
    let stay = purchases::stay_mul(&mut w, &c, pi, 20.0);
    assert!(stay > 1.2, "stay ×{stay}");
    assert!(purchases::tips_mul(&w, &c, pi) > 1.05);
    let t = &w.payoff_tonight().by_seat["sofa"];
    assert!(t.parties == 1 && t.stay_minutes > 4.0, "{t:?}");
    // the next pair sits at grandma's chairs: neutral
    let pid2 = spawn(&mut w, &c, 2);
    let pi2 = pi_of(&w, pid2);
    assert_eq!(
        purchases::stay_mul(&mut w, &c, pi2, 20.0),
        1.0 + w.coziness_tier().stay
    );
    // stools at a table: shorter, smaller tips
    let mut w = day(&c, 6);
    let table = place(&mut w, &c, "table_round", 24, 4, 0);
    let _ = table;
    place(&mut w, &c, "stool", 23, 4, 1);
    place(&mut w, &c, "stool", 25, 4, 3);
    // grandma's tables are taken first (chairs beat stools), so fill them
    open(&mut w, &c);
    let mut last = 0;
    for _ in 0..9 {
        last = spawn(&mut w, &c, 2);
    }
    let pi = pi_of(&w, last);
    let lead = w.parties[pi].members[0];
    assert_eq!(
        w.seat_feel(&c, lead).map(|f| f.0),
        Some(purchases::SeatFeel::Hard)
    );
    assert!(purchases::stay_mul(&mut w, &c, pi, 20.0) < 1.0);
    assert!(purchases::tips_mul(&w, &c, pi) < 1.0);
}

#[test]
fn a_comfy_party_orders_a_second_round_grandmas_chairs_never_draw_the_rng() {
    let c = content();
    let mut w = day(&c, 8);
    place(&mut w, &c, "armchair", 24, 3, 0);
    open(&mut w, &c);
    let pid = spawn(&mut w, &c, 1);
    let pi = pi_of(&w, pid);
    let item = w.alloc_id();
    let leader = w.parties[pi].members[0];
    w.items.push(OrderItem {
        id: item,
        party: pid,
        guest: leader,
        menu: "cappuccino".into(),
        state: ItemState::Served,
        prep_left: 0.0,
        by: None,
        ready_tile: None,
        quality: 0.7,
        created: 0,
        origin: Default::default(),
        price_add: 0,
        hand_off_match: None,
        wish: false,
    });
    {
        let p = &mut w.parties[pi];
        p.items = vec![item];
        p.served = true;
        p.wish_open = false;
        p.state = PartyState::Eating;
        p.stay_left = 0.0;
    }
    assert!(purchases::round_chance(&w, &c, pi) > 0.3);
    let mut ordered = false;
    for _ in 0..40 {
        if purchases::second_round(&mut w, &c, pid) {
            ordered = true;
            break;
        }
    }
    assert!(ordered, "a 40 % chance came up within 40 tries");
    let p = &w.parties[pi_of(&w, pid)];
    assert_eq!((p.rounds, p.state), (1, PartyState::WaitFood));
    let round = w
        .items
        .iter()
        .find(|i| i.id == *p.items.last().unwrap())
        .unwrap();
    assert_eq!(
        (round.menu.as_str(), round.state),
        ("cappuccino", ItemState::Pending)
    );
    assert_eq!(w.payoff_tonight().by_seat["armchair"].rounds, 1);
    // only one round per visit
    w.parties[pi].state = PartyState::Eating;
    assert!(!purchases::second_round(&mut w, &c, pid));
    // a pair on grandma's chairs: no chance, and the RNG is untouched
    let pid2 = spawn(&mut w, &c, 2);
    let pi2 = pi_of(&w, pid2);
    w.parties[pi2].served = true;
    w.parties[pi2].wish_open = false;
    w.parties[pi2].state = PartyState::Eating;
    let tier_round = w.coziness_tier().round;
    if tier_round == 0.0 {
        let rng = w.rng.clone();
        assert!(!purchases::second_round(&mut w, &c, pid2));
        assert_eq!(w.rng, rng);
    }
}

// ---------------------------------------------------------------------------------------------
// Coziness tiers
// ---------------------------------------------------------------------------------------------

#[test]
fn coziness_tiers_unlock_their_perks() {
    let c = content();
    let t3 = purchases::tier_of(3);
    let t4 = purchases::tier_of(4);
    let t5 = purchases::tier_of(5);
    assert!(0.0 < t3.tips && t3.tips < t4.tips && t4.tips < t5.tips);
    assert!(t4.stay > 0.0 && t5.stay > t4.stay && t5.review > t4.review);
    assert!(t5.wishes && !t5.rare.is_empty());
    let mut w = day(&c, 8);
    open(&mut w, &c);
    let wanderer = &t5.rare[0].0;
    w.sys.atmos.coziness.level = 4;
    assert_eq!(purchases::rare_weight(&w, wanderer), 0.0);
    let r4 = purchases::review_mul(&w);
    w.sys.atmos.coziness.level = 5;
    assert!(purchases::rare_weight(&w, wanderer) > 0.0);
    assert!(purchases::review_mul(&w) > r4);
    // regulars' "like home" wishes wait for the top tier's threshold
    let top = *c.handson.decor.coziness.levels.last().unwrap();
    let like_home: Vec<_> = c
        .handson
        .wishes
        .wishes
        .iter()
        .filter(|x| x.id.ends_with(".like_home"))
        .collect();
    assert!(like_home.len() >= 2);
    for wish in like_home {
        assert!(
            format!("{:?}", wish.when).contains(&format!("CozinessAtLeast({top:?})")),
            "{}: {:?}",
            wish.id,
            wish.when
        );
    }
    // tips: the tier's share is booked
    let pid = spawn(&mut w, &c, 1);
    let pi = pi_of(&w, pid);
    let mul = purchases::tips_mul(&w, &c, pi);
    assert!((mul - (1.0 + t5.tips)).abs() < 1e-4, "{mul}");
    let tip = (100.0 * (1.0 + t5.tips)).round() as i64;
    purchases::note_tip(&mut w, &c, pi, tip);
    assert_eq!(w.payoff_tonight().tier_tips, tip - 100);
}

// ---------------------------------------------------------------------------------------------
// Appliances by tag
// ---------------------------------------------------------------------------------------------

#[test]
fn a_second_coffee_machine_brews_in_parallel() {
    let c = content();
    let mut w = day(&c, 6);
    let first = w
        .furniture()
        .iter()
        .find(|p| p.def == "coffee_machine")
        .unwrap()
        .uid;
    open(&mut w, &c);
    let k = w.karolina().unwrap().id;
    // one machine: busy → nothing free, nothing parallel
    w.furniture
        .iter_mut()
        .find(|p| p.uid == first)
        .unwrap()
        .user = Some(9999);
    assert_eq!(staff::station_free(&w, &c, "station:coffee", k), None);
    purchases::note_station_start(&mut w, &c, first);
    assert!(w.payoff_tonight().parallel.is_empty());
    // a second machine: Karolina brews on it while the first is busy
    let mut w = day(&c, 6);
    let second = place(&mut w, &c, "coffee_machine", 22, 13, 2);
    open(&mut w, &c);
    assert!(staff::station_access(&w, &c, second).is_some());
    w.furniture
        .iter_mut()
        .find(|p| p.uid == first)
        .unwrap()
        .user = Some(9999);
    assert_eq!(
        staff::station_free(&w, &c, "station:coffee", k),
        Some(second)
    );
    purchases::note_station_start(&mut w, &c, second);
    assert_eq!(w.payoff_tonight().parallel["station:coffee"], 1);
    let e = w.piece_effect(&c, c.furniture_def("coffee_machine").unwrap());
    assert_eq!(e.role, Some((purchases::ApplianceRole::Parallel, true)));
}

#[test]
fn a_grinder_lifts_quick_brews_a_dish_rack_washes_faster_an_oven_bakes() {
    let c = content();
    let mut w = day(&c, 6);
    assert_eq!(purchases::quick_quality_bonus(&w, &c), 0.0);
    assert_eq!(purchases::wash_mul(&w, &c), 1.0);
    let kolac = c.menu_item("kolac").unwrap();
    assert!(!guests::item_orderable(&w, &c, kolac), "no oven, no cakes");
    let oven = c.furniture_def("oven").unwrap();
    assert_eq!(w.piece_effect(&c, oven).unlocks, vec!["kolac".to_string()]);
    place(&mut w, &c, "espresso_grinder", 22, 13, 2);
    place(&mut w, &c, "dish_rack", 23, 13, 2);
    place(&mut w, &c, "oven", 24, 13, 2);
    assert!((purchases::quick_quality_bonus(&w, &c) - 0.08).abs() < 1e-5);
    assert!((purchases::wash_mul(&w, &c) - 1.4).abs() < 1e-5);
    assert!(guests::item_orderable(&w, &c, kolac));
    // a second grinder adds nothing (max 1)
    place(&mut w, &c, "espresso_grinder", 25, 13, 2);
    assert!((purchases::quick_quality_bonus(&w, &c) - 0.08).abs() < 1e-5);
    // the counters
    open(&mut w, &c);
    purchases::note_washed(&mut w, 10, 1.4);
    assert_eq!(w.payoff_tonight().washed, 3);
    purchases::on_item_served(&mut w, &c, "kolac");
    purchases::on_item_served(&mut w, &c, "cappuccino");
    let price = w.price_of(&c, "kolac") as i64;
    assert_eq!(w.payoff_tonight().made["station:oven"], (1, price));
}

// ---------------------------------------------------------------------------------------------
// The morning's lines, saves, the player-bot
// ---------------------------------------------------------------------------------------------

#[test]
fn the_payoff_lines_add_up_and_survive_a_save() {
    let c = content();
    let mut w = World::new_story_seeded(&c, Difficulty::Cozy, 4242);
    let mut b = bot::PlayerBot::new(bot::BotConfig {
        seed: 4242,
        ..Default::default()
    });
    let mut keys = std::collections::BTreeSet::new();
    for _ in 0..12 {
        b.run_night(&mut w, &c);
        let s = w.last_summary().unwrap().clone();
        assert!(s.payoff.len() <= purchases::data().summary_max);
        // the money lines are parts of the night's money, never more
        let tip_lines: i64 = s
            .payoff
            .iter()
            .filter(|l| {
                l.key.starts_with("purchases.line.comfy")
                    || l.key.starts_with("purchases.line.tier")
            })
            .map(|l| l.money.max(0))
            .sum();
        assert!(
            tip_lines <= s.tips,
            "night {}: {tip_lines} > tips {}",
            s.night,
            s.tips
        );
        let all: i64 = s.payoff.iter().map(|l| l.money.max(0)).sum();
        assert!(all <= s.revenue + s.tips, "night {}", s.night);
        // each line is its tally
        let t = w.payoff_tonight();
        for l in &s.payoff {
            let get = |k: &str| l.args.iter().find(|a| a.0 == k).map(|a| a.1.clone());
            match l.key.as_str() {
                "purchases.line.no_seat" => assert_eq!(get("n"), Some(t.turned_away.to_string())),
                "purchases.line.seats" => assert_eq!(l.money, t.extra_money),
                "purchases.line.tier" | "purchases.line.tier_rare" => {
                    assert_eq!(l.money, t.tier_tips)
                }
                "purchases.line.grinder" => assert_eq!(get("n"), Some(t.ground.to_string())),
                "purchases.line.washing" => assert_eq!(get("n"), Some(t.washed.to_string())),
                _ => {}
            }
            // every arg that names a key names a real one
            for (_, v) in &l.args {
                if let Some(k) = v.strip_prefix('@') {
                    assert!(!k.ends_with(".."), "{k}");
                }
            }
            keys.insert(l.key.clone());
        }
    }
    assert!(w.furniture().len() > World::new_story(&c, Difficulty::Cozy).furniture().len());
    assert!(
        keys.len() >= 3,
        "the attentive bot's purchases show: {keys:?}"
    );
    let text = w.save_to_string().unwrap();
    let back = World::load_from_str(&text).unwrap();
    assert_eq!(back.pay, w.pay);
    assert_eq!(back.last_summary(), w.last_summary());
}

/// The whole Cozy story twice: an attentive bot that furnishes the café vs one that never
/// builds. Both end in the light; the builder earns noticeably more by night 30.
#[test]
fn a_building_player_earns_noticeably_more_than_one_who_never_builds() {
    let c = content();
    let play = |build: bool| {
        let mut w = World::new_story_seeded(&c, Difficulty::Cozy, 4242);
        let mut b = bot::PlayerBot::new(bot::BotConfig {
            seed: 4242,
            build,
            ..Default::default()
        });
        let mut earned = 0i64;
        for _ in 0..30 {
            if w.is_closed() {
                break;
            }
            b.run_night(&mut w, &c);
            let s = w.last_summary().unwrap();
            earned += s.revenue + s.tips;
        }
        (earned, w.ending().map(str::to_string), w.debt())
    };
    let (built, end_b, debt_b) = play(true);
    let (plain, end_p, debt_p) = play(false);
    assert_eq!(
        end_p.as_deref(),
        Some("light"),
        "never building still ends in the light"
    );
    assert_eq!(end_b.as_deref(), Some("light"));
    assert_eq!((debt_b, debt_p), (0, 0));
    let gain = built as f32 / plain as f32 - 1.0;
    assert!(
        gain >= 0.15,
        "builder {built} vs {plain}: +{:.1} %",
        gain * 100.0
    );
}
