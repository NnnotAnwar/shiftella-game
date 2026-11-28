//! Guest timers and what guests ask for (the owner's play-test, 0.2.x):
//!
//! * the ticket's wait bar is the truth: whatever makes a guest leave is what the bar shows —
//!   a regular's visit window no longer walks a waiting guest out with a full bar, and the bar
//!   after the order is taken drains from the wait the guest really has;
//! * Radek (and every regular) stays a fair wait after ordering, however late in his short
//!   window he walked in;
//! * the patience clock starts when the order can be taken (the leader has sat down);
//! * a wish or a regular's line never names a dish the café can't make tonight.

use super::guests::{self, FAIR_FOOD_WAIT, WRAP_UP};
use super::systems::{ActiveMoment, MomentPos, XTarget};
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

/// Story night `n` on difficulty `d`, doors open, no random walk-ins and no regular coming
/// in on their own (the test spawns them).
fn quiet_night(c: &Content, n: u32, d: Difficulty) -> World {
    night_with(c, n, d, false)
}

/// … with an oven bought by day (the koláč needs one; grandma's café has none).
fn night_with(c: &Content, n: u32, d: Difficulty, oven: bool) -> World {
    let mut w = World::new_story(c, d);
    w.sys.tutorial_off = true;
    if n > 1 {
        w.night = n;
        w.begin_day(c);
    }
    if oven {
        w.debug_set_money(200_000);
        let placed = (12..16).rev().any(|y| {
            (18..30).rev().any(|x| {
                w.apply(
                    c,
                    Command::PlaceFurniture {
                        def: "oven".into(),
                        x,
                        y,
                        rot: 2,
                    },
                )
                .is_ok()
            })
        });
        assert!(placed, "an oven");
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
    for r in &c.regulars {
        w.story.regulars.entry(r.id.clone()).or_default().spawned = (0..8).collect();
    }
    w.drain_events();
    w
}

fn tick_to(w: &mut World, c: &Content, minute: u32) {
    while w.minute() < minute && w.phase() == Phase::Night {
        w.tick(c);
        bot::finish_cutscenes(w, c);
        w.drain_events();
    }
}

fn wait_frac(w: &World, c: &Content, pid: u32) -> f32 {
    w.ticket_rail(c)
        .iter()
        .filter(|t| t.party == pid)
        .map(|t| t.wait_frac)
        .fold(0.0, f32::max)
}

fn alive(w: &World, pid: u32) -> bool {
    w.parties()
        .iter()
        .any(|p| p.id == pid && p.state != PartyState::Leaving)
}

/// Ticks (nobody serves) until the party leaves; returns (minutes it stayed from now, the
/// ticket's last wait share before it left).
fn wait_out(w: &mut World, c: &Content, pid: u32) -> (u32, f32) {
    let start = w.minute();
    let mut last = 0.0;
    while alive(w, pid) && w.phase() == Phase::Night {
        last = wait_frac(w, c, pid);
        w.tick(c);
        bot::finish_cutscenes(w, c);
        w.drain_events();
    }
    (w.minute() - start, last)
}

/// The regular walks in now, sits down at once and Karolina takes the order straight away.
fn regular_orders_now(w: &mut World, c: &Content, id: &str, visit: Option<usize>) -> u32 {
    guests::spawn_regular(w, c, id, visit);
    let pid = w.story.regulars[id].party.expect("spawned");
    // walk them in: a few ticks for the seat / queue spot, then settle on it
    for _ in 0..3 {
        w.tick(c);
        bot::finish_cutscenes(w, c);
    }
    guests::settle_now(w, pid);
    let me = pacing::owner_id(w).unwrap();
    guests::order_taken(w, c, pid, me);
    w.drain_events();
    pid
}

// ---------------------------------------------------------------------------------------------
// 1. The visible timer is the truth
// ---------------------------------------------------------------------------------------------

/// The owner: "I took the order, the timer refreshed and showed a lot of time — and then he
/// suddenly left." Radek's visit window (01:00–01:25) walked him out at 01:25 with his ticket
/// still nearly full: a second, hidden timer.
#[test]
fn radek_doesnt_walk_out_with_time_left_on_his_ticket() {
    let c = content();
    for d in [Difficulty::Classic, Difficulty::Cozy] {
        let mut w = quiet_night(&c, 9, d);
        // the latest he walks in for his 01:00–01:25 window (half the window late)
        tick_to(&mut w, &c, 193);
        let pid = regular_orders_now(&mut w, &c, "radek", Some(0));
        let p = w.parties().iter().find(|p| p.id == pid).unwrap();
        assert_eq!(p.state, PartyState::WaitFood, "{d:?}");
        let leave_by = p.leave_by.unwrap();
        let (stayed, last) = wait_out(&mut w, &c, pid);
        assert!(
            w.minute() > leave_by,
            "{d:?}: he still left at his window's end ({leave_by}) after {stayed} min"
        );
        assert!(
            last >= 0.95,
            "{d:?}: he left after {stayed} min with {:.0} % of his wait still on the ticket",
            (1.0 - last) * 100.0
        );
    }
}

/// Every regular, every visit, walking in as late as the story lets them: once the order is
/// taken they wait at least a fair food wait, and leave only once their ticket ran out.
#[test]
fn every_regular_gets_a_fair_wait_after_ordering() {
    let c = content();
    for d in [Difficulty::Classic, Difficulty::Cozy] {
        for r in &c.regulars {
            for (vi, v) in r.visits.iter().enumerate() {
                let mut w = quiet_night(&c, 12, d);
                let from = v.from.minutes_since_open().max(0) as u32;
                let to = (v.to.minutes_since_open().max(0) as u32).max(from);
                let latest = (from + (to - from) / 2 + 1).min(guests::LAST_CALL - 1);
                tick_to(&mut w, &c, latest);
                let pid = regular_orders_now(&mut w, &c, &r.id, Some(vi));
                let ordered = w.minute();
                let (stayed, last) = wait_out(&mut w, &c, pid);
                if w.minute() >= WRAP_UP {
                    // closing time wraps everybody up (05:45 / 05:50): not a walk-out
                    continue;
                }
                assert!(
                    stayed as f32 >= FAIR_FOOD_WAIT,
                    "{d:?} {} visit {vi}: ordered at {ordered}, gone after {stayed} min",
                    r.id
                );
                assert!(
                    last >= 0.95,
                    "{d:?} {} visit {vi}: left after {stayed} min with {:.0} % of the wait \
                     still on the ticket",
                    r.id,
                    (1.0 - last) * 100.0
                );
            }
        }
    }
}

/// The ticket drains from the wait the guest really has: with the fair minimum above three
/// times their patience, the bar used to sit full for minutes and then race to the end.
#[test]
fn the_ticket_drains_evenly_from_the_order() {
    let c = content();
    let mut w = quiet_night(&c, 9, Difficulty::Classic);
    tick_to(&mut w, &c, 30);
    let pid = guests::spawn_party(&mut w, &c, "taxi_drivers", 1, None, 0.3, 1.0, &[], None, false)
        .expect("party");
    for _ in 0..3 {
        w.tick(&c);
    }
    guests::settle_now(&mut w, pid);
    let me = pacing::owner_id(&w).unwrap();
    guests::order_taken(&mut w, &c, pid, me);
    let p = w.parties().iter().find(|p| p.id == pid).unwrap();
    assert_eq!(p.state, PartyState::WaitFood);
    let full = p.patience;
    assert!(
        full >= FAIR_FOOD_WAIT && full > p.patience_max * guests::food_patience(&w),
        "the fair minimum applies here: {full} vs {}",
        p.patience_max
    );
    // (the bar runs between the once-a-minute patience steps: at most one minute's worth)
    let step = 1.0 / full;
    assert!(
        wait_frac(&w, &c, pid) <= step + 0.01,
        "a fresh order shows a full bar"
    );
    let half = w.minute() + (full / 2.0).round() as u32;
    tick_to(&mut w, &c, half);
    let p = w.parties().iter().find(|p| p.id == pid).unwrap();
    let truth = 1.0 - p.patience / full;
    let shown = wait_frac(&w, &c, pid);
    assert!(
        (shown - truth).abs() <= step + 0.02,
        "half-way the ticket shows {shown:.2} used, the guest has used {truth:.2}"
    );
}

/// Bot-played nights, Cozy and Classic: nobody walks out unserved (before closing time) while
/// their ticket still shows time left.
#[test]
fn walkouts_only_happen_when_the_ticket_runs_out() {
    let c = content();
    for d in [Difficulty::Classic, Difficulty::Cozy] {
        for seed in [1u64, 3] {
            let mut w = World::new_story_seeded(&c, d, seed * 7919 + 13);
            let mut b = bot(seed);
            for _ in 0..12 {
                if w.phase() == Phase::Day {
                    b.day(&mut w, &c);
                    b.finish_scenes(&mut w, &c);
                    w.drain_events();
                    w.start_night(&c);
                }
                let mut last: std::collections::BTreeMap<u32, (f32, bool)> = Default::default();
                while w.phase() == Phase::Night && !w.is_closed() {
                    b.finish_scenes(&mut w, &c);
                    b.step(&mut w, &c);
                    let rail = w.ticket_rail(&c);
                    for p in w.parties() {
                        if p.state == PartyState::Leaving {
                            continue;
                        }
                        let wf = rail
                            .iter()
                            .filter(|t| t.party == p.id)
                            .map(|t| t.wait_frac)
                            .fold(0.0, f32::max);
                        let waiting = matches!(
                            p.state,
                            PartyState::Queue | PartyState::WaitOrder | PartyState::WaitFood
                        );
                        last.insert(p.id, (wf, waiting));
                    }
                    w.tick(&c);
                    for e in w.drain_events() {
                        if let SimEvent::GuestLeft { party, served, .. } = e
                            && !served
                            && w.minute() < WRAP_UP
                            && let Some(&(wf, waiting)) = last.get(&party)
                        {
                            assert!(
                                !waiting || wf >= 0.9,
                                "{d:?} seed {seed} night {} minute {}: party {party} walked \
                                 out with {:.0} % of its wait still on the ticket",
                                w.night(),
                                w.minute(),
                                (1.0 - wf) * 100.0
                            );
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// 4. One patience rule: the clock starts when the order can be taken
// ---------------------------------------------------------------------------------------------

/// A party at a table: while the leader is still walking to their chair the order can't be
/// taken — and their patience doesn't run either (it used to start once anyone sat down).
#[test]
fn patience_waits_for_the_leader_to_sit_down() {
    let c = content();
    let mut w = quiet_night(&c, 9, Difficulty::Classic);
    tick_to(&mut w, &c, 30);
    let mut pid = None;
    for _ in 0..40 {
        let Some(id) =
            guests::spawn_party(&mut w, &c, "students", 2, None, 1.0, 1.0, &[], None, false)
        else {
            continue;
        };
        w.tick(&c);
        if w
            .parties()
            .iter()
            .any(|p| p.id == id && p.state == PartyState::WaitOrder)
        {
            pid = Some(id);
            break;
        }
        guests::finish_party(&mut w, &c, id, guests::Exit::Normal);
    }
    let pid = pid.expect("a party ordering at its table");
    guests::settle_now(&mut w, pid);
    // the leader stands up again (still on the way to the chair); the friend is seated
    let members = w.parties().iter().find(|p| p.id == pid).unwrap().members.clone();
    assert_eq!(members.len(), 2);
    for (k, gid) in members.iter().enumerate() {
        let g = w.guests.iter_mut().find(|g| g.id == *gid).unwrap();
        g.pose = if k == 0 { Pose::Idle } else { Pose::Sit };
    }
    let p = w.parties().iter().find(|p| p.id == pid).unwrap().clone();
    assert!(!guests::settled(&w, &p), "the order can't be taken yet");
    let before = p.patience;
    // one on_minute, without the guests moving on
    guests::on_minute(&mut w, &c);
    let after = w.parties().iter().find(|p| p.id == pid).unwrap().patience;
    assert_eq!(before, after, "patience ran before the order could be taken");
}

// ---------------------------------------------------------------------------------------------
// 3. Guests only ask for what the café makes tonight
// ---------------------------------------------------------------------------------------------

/// Vera's "Your grandmother's koláč was better — show me yours" while the café can't bake a
/// koláč (here: switched off on the menu; also: no oven yet): the owner found no koláč, nor
/// anything like it, among the choices.
#[test]
fn vera_never_asks_for_a_kolac_the_cafe_cant_make() {
    let c = content();
    let mut saw_kolac_wish = false;
    for (oven, off) in [(false, false), (true, true), (true, false)] {
        let mut w = night_with(&c, 12, Difficulty::Cozy, oven);
        if off {
            w.econ.menu_off.insert("kolac".into());
        }
        let kolac_ok = c
            .menu_item("kolac")
            .is_some_and(|m| guests::item_orderable(&w, &c, m));
        assert_eq!(kolac_ok, oven && !off, "oven {oven}, off {off}");
        for _ in 0..80 {
            w.hands.wish_uses.clear();
            let k = orders::pick_order_kind(&mut w, &c, "insomniacs", c.regular("vera"), 1, true);
            if k.wish() == Some("vera.lidas_kolac") {
                saw_kolac_wish = true;
                assert!(kolac_ok, "oven {oven}, off {off}: she asked for a koláč");
            }
        }
    }
    assert!(saw_kolac_wish, "with the koláč on the menu she still asks for it");
}

/// When a wish names a dish, the order card's chips (the top four) include a recipe for it.
#[test]
fn the_choices_include_the_dish_the_wish_names() {
    let c = content();
    let mut w = night_with(&c, 12, Difficulty::Cozy, true);
    // Vera with her koláč wish
    let pid = regular_orders_now(&mut w, &c, "vera", Some(0));
    let pi = w.parties.iter().position(|p| p.id == pid).unwrap();
    w.parties[pi].order_kind = OrderKind::Wish {
        wish: "vera.lidas_kolac".into(),
    };
    w.parties[pi].wish_open = true;
    let leader = w.parties[pi].members[0];
    assert!(
        c.menu_item("kolac")
            .is_some_and(|m| guests::item_orderable(&w, &c, m)),
        "the koláč is on tonight"
    );
    let chips = w.best_matches(&c, leader);
    assert!(
        chips.iter().any(|(r, _)| w
            .recipe_def(&c, r)
            .is_some_and(|r| r.base_item == "kolac")),
        "no koláč among {chips:?}"
    );
}

/// A regular's arrival line that orders a dish by name ("Cappuccino, please") isn't said on
/// a night the café can't make it.
#[test]
fn arrival_lines_only_name_what_the_cafe_makes() {
    let c = content();
    let mut w = quiet_night(&c, 12, Difficulty::Cozy);
    w.econ.menu_off.insert("cappuccino".into());
    let mut said = Vec::new();
    for _ in 0..40 {
        let pid = w.alloc_id();
        story::regular_line(&mut w, &c, "hana", pid);
        for e in w.drain_events() {
            if let SimEvent::Speech { key, .. } = e {
                said.push(key);
            }
        }
    }
    assert!(!said.is_empty(), "she still says something");
    assert!(
        !said.iter().any(|k| k == "regular.hana.line.0a"),
        "Hana ordered a cappuccino the café can't make tonight"
    );
}

/// Every dish a line names is a real menu item or recipe.
#[test]
fn named_lines_name_real_dishes() {
    let c = content();
    let named = &c.handson.wishes.line_items;
    assert!(named.contains_key("wish.vera.lidas_kolac"));
    assert!(named.contains_key("regular.hana.line.0a"));
    for (key, items) in named {
        assert!(!items.is_empty(), "{key}: names nothing");
        for i in items {
            assert!(
                c.menu_item(i).is_some()
                    || c.handson.recipe(i).is_some()
                    || i == signature::SIGNATURE_RECIPE,
                "{key}: {i} is neither a menu item nor a recipe"
            );
        }
    }
}

/// Mikuláš's "A koláč for each" option isn't offered while the café can't make a koláč.
#[test]
fn a_moment_never_offers_a_dish_the_cafe_cant_make() {
    let c = content();
    let mut w = night_with(&c, 12, Difficulty::Cozy, true);
    let m = w.minute();
    let active = ActiveMoment {
        id: "m_mikulas".into(),
        anchor: MomentPos {
            tile: (0, 0),
            furniture: None,
            agent: None,
        },
        spawned: m,
        expires: m + 45,
        target: XTarget::default(),
        resolving: None,
    };
    w.sys.moments.active.push(active);
    let on = moments::options_now(&w, &c, "m_mikulas");
    assert!(on.contains(&"kolac".to_string()), "{on:?}");
    w.econ.menu_off.insert("kolac".into());
    let off = moments::options_now(&w, &c, "m_mikulas");
    assert!(!off.contains(&"kolac".to_string()), "{off:?}");
    assert!(off.contains(&"applause".to_string()), "{off:?}");
}

/// Every dish on tonight's menu has a recipe Karolina can choose for a plain order.
#[test]
fn every_orderable_dish_has_a_recipe_to_choose() {
    let c = content();
    for n in [1, 5, 12, 20] {
        let w = quiet_night(&c, n, Difficulty::Cozy);
        for m in &c.menu {
            if guests::item_orderable(&w, &c, m) {
                assert!(
                    c.handson
                        .recipes_for_item(&m.id)
                        .any(|r| orders::recipe_available(&w, &c, r)),
                    "night {n}: {} is on the menu with no recipe to choose",
                    m.id
                );
            }
        }
    }
}

/// A short wait (a nine-minute bar queue on release/0.3.0, Classic seed 3 night 7): patience
/// drops once a minute, so the ticket used to show 10 % left until the guest got up. The bar
/// runs between the steps and reads empty as they leave.
#[test]
fn a_short_wait_reads_empty_when_the_guest_gets_up() {
    let c = content();
    let mut w = quiet_night(&c, 9, Difficulty::Classic);
    tick_to(&mut w, &c, 30);
    let pid = guests::spawn_party(&mut w, &c, "taxi_drivers", 1, None, 1.0, 1.0, &[], None, false)
        .expect("party");
    for _ in 0..3 {
        w.tick(&c);
    }
    guests::settle_now(&mut w, pid);
    let pi = w.parties.iter().position(|p| p.id == pid).unwrap();
    w.parties[pi].patience = 8.9;
    w.parties[pi].wait_full = 8.9;
    let (stayed, last) = wait_out(&mut w, &c, pid);
    assert!(stayed >= 8, "gone after {stayed} min");
    assert!(last >= 0.97, "left with {:.0} % on the ticket", (1.0 - last) * 100.0);
}
