//! "The helper does the same job as me" (v0.2.5 play report): Karolina and a helper both went
//! to mop the same spot — he finished, and she finished the same thing a second later. One
//! claim rule now covers every piece of shared work (`sim::claims`): whoever has it — Karolina
//! from the moment she queues it, a helper from the moment they pick it — holds it alone.
//! Helpers never take what she holds; her click on what a helper holds takes another free one
//! of the same kind (another dirt blot, another cup of the party) or is refused (`InUse`, the
//! UI names who has it). Nothing is done, credited or rewarded twice.

use super::grid;
use super::guests;
use super::hands::PlayerAct;
use super::*;
use crate::content::Content;
use crate::content::handson::Duty;
use std::collections::BTreeSet;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

fn content() -> Content {
    Content::load(DATA).unwrap_or_else(|e| panic!("{e}"))
}

/// A quiet sandbox night (no walk-ins) right after the preparation, with Karolina behind the
/// counter and one experienced helper (no traits) holding exactly `duties`, on the floor at
/// the door with nothing to do; a clean café. Returns (world, helper index).
fn night_with(c: &Content, duties: &[Duty]) -> (World, usize) {
    let mut w = World::new_sandbox(c, SandboxSetup::default());
    bot::finish_cutscenes(&mut w, c);
    w.sys.tutorial_off = true;
    if !w.staff.iter().any(|s| !s.owner) {
        let s = staff::generate_staff(&mut w, c, None);
        w.push_staff(s);
    }
    let i = w.staff.iter().rposition(|s| !s.owner).expect("a helper");
    for (k, s) in w.staff.iter_mut().enumerate().filter(|(_, s)| !s.owner) {
        s.duties = if k == i { duties.to_vec() } else { Vec::new() };
        s.traits.clear();
        s.nights_worked = 5;
        s.skills = [7.0; 5];
    }
    w.start_night(c);
    bot::finish_cutscenes(&mut w, c);
    w.skip_prep(c);
    w.mods.push(Modifier {
        kind: ModKind::GuestRate,
        mul: 0.0,
        until: None,
        event: None,
    });
    let door = grid::tile_center(w.grid.entrance());
    for (k, s) in w.staff.iter_mut().enumerate() {
        if s.owner {
            continue;
        }
        if k == i {
            s.status = StaffStatus::OnDuty;
            s.shift = Shift::Full;
            s.leave_at = NIGHT_MINUTES;
            s.pos = door;
        } else {
            s.status = StaffStatus::OffDuty;
        }
        s.path.clear();
        s.task = None;
    }
    assert!(w.karolina().is_some_and(|k| k.on_duty()), "Karolina is in");
    w.dirt = 0.0;
    w.sink_dishes = 0;
    for p in w.furniture.iter_mut() {
        p.dishes = 0;
        p.dirt = 0.0;
        p.claim = None;
    }
    w.drain_events();
    (w, i)
}

fn queue(w: &mut World, c: &Content, act: PlayerAct) -> Result<(), CommandError> {
    w.apply(c, Command::PlayerQueue { act, front: false })
}

fn karolina_busy(w: &World) -> bool {
    w.player().current.is_some() || !w.player().queue.is_empty()
}

/// Every act of Karolina's (the current one first).
fn her_acts(w: &World) -> Vec<PlayerAct> {
    let p = w.player();
    p.current
        .iter()
        .map(|c| c.act.clone())
        .chain(p.queue.iter().cloned())
        .collect()
}

fn helper_mop(w: &World, i: usize) -> Option<(i32, i32)> {
    match &w.staff[i].task {
        Some(Task::Mop { tile }) => Some(*tile),
        _ => None,
    }
}

fn her_mop(w: &World) -> Option<(i32, i32)> {
    her_acts(w).iter().find_map(|a| match a {
        PlayerAct::Mop { tile } => Some(*tile),
        _ => None,
    })
}

fn floor_credit(w: &World, i: usize) -> u32 {
    w.staff[i].duty_done[duties::duty_index(Duty::Floor)]
}

// ---------------------------------------------------------------------------------------------
// Mop: the same dirt blot
// ---------------------------------------------------------------------------------------------

#[test]
fn a_blot_karolina_is_on_the_way_to_is_left_to_her() {
    let c = content();
    let (mut w, i) = night_with(&c, &[Duty::Floor]);
    w.dirt = 0.1;
    let spots = w.dirt_spots();
    assert_eq!(spots.len(), 1, "{spots:?}");
    let t = spots[0];
    queue(&mut w, &c, PlayerAct::Mop { tile: t }).unwrap();
    let mut mopped = Vec::new();
    for _ in 0..1500 {
        w.tick(&c);
        assert_ne!(
            helper_mop(&w, i),
            Some(t),
            "the helper went to mop the blot Karolina is on"
        );
        for e in w.drain_events() {
            if let SimEvent::Mopped { tile } = e {
                mopped.push(tile);
            }
        }
        if !karolina_busy(&w) && w.staff[i].task.as_ref().is_none_or(|t| t.id() == "idle") {
            break;
        }
    }
    assert_eq!(mopped, vec![t], "she mopped her blot once");
    assert!(w.dirt_spots().is_empty() && w.dirt < 0.02, "{}", w.dirt);
    assert_eq!(floor_credit(&w, i), 0, "the helper mopped nothing");
}

#[test]
fn two_blots_are_mopped_once_each_and_each_by_whoever_holds_it() {
    let c = content();
    let (mut w, i) = night_with(&c, &[Duty::Floor]);
    w.dirt = 0.2;
    let spots = w.dirt_spots();
    assert_eq!(spots.len(), 2, "{spots:?}");
    // she takes the one a single swipe would clear (the last listed): the helper's favourite
    let hers = spots[1];
    queue(&mut w, &c, PlayerAct::Mop { tile: hers }).unwrap();
    let mut helper_tile = None;
    let mut her_swipes = Vec::new();
    for _ in 0..3000 {
        w.tick(&c);
        if let Some(t) = helper_mop(&w, i) {
            assert_ne!(t, hers, "the helper took Karolina's blot");
            assert!(her_mop(&w) != Some(t), "both on {t:?}");
            helper_tile = Some(t);
        }
        // the helper's swipe never takes away the blot Karolina is still heading for
        if her_mop(&w) == Some(hers) {
            assert!(
                w.dirt_spots().contains(&hers),
                "her blot {hers:?} vanished under her: {:?}",
                w.dirt_spots()
            );
        }
        for e in w.drain_events() {
            if let SimEvent::Mopped { tile } = e {
                her_swipes.push(tile);
            }
        }
        if !karolina_busy(&w) && helper_mop(&w, i).is_none() && w.dirt_spots().is_empty() {
            break;
        }
    }
    assert_eq!(her_swipes, vec![hers], "her one swipe, on her blot");
    assert_eq!(helper_tile, Some(spots[0]), "the helper took the other blot");
    assert_eq!(floor_credit(&w, i), 1, "the helper mopped one blot");
    assert!(w.dirt_spots().is_empty() && w.dirt < 0.02, "{}", w.dirt);
}

#[test]
fn a_click_on_the_helpers_blot_takes_another_free_blot_or_is_refused() {
    let c = content();
    let (mut w, i) = night_with(&c, &[Duty::Floor]);
    w.dirt = 0.2;
    for _ in 0..10 {
        w.tick(&c);
        if helper_mop(&w, i).is_some() {
            break;
        }
    }
    let his = helper_mop(&w, i).expect("the helper went to mop");
    let other = *w.dirt_spots().iter().find(|t| **t != his).unwrap();
    // a click on his blot: she takes the other one
    queue(&mut w, &c, PlayerAct::Mop { tile: his }).unwrap();
    assert_eq!(her_mop(&w), Some(other), "she goes to the free blot");
    // nothing else is free: a second click is refused, not a second swipe on his blot
    assert_eq!(
        queue(&mut w, &c, PlayerAct::Mop { tile: his }),
        Err(CommandError::InUse)
    );
    assert_eq!(her_acts(&w).len(), 1);
}

#[test]
fn her_swipe_on_a_blot_already_gone_does_nothing() {
    let c = content();
    let (mut w, _) = night_with(&c, &[Duty::Register]);
    w.dirt = 0.1;
    let t = w.dirt_spots()[0];
    queue(&mut w, &c, PlayerAct::Mop { tile: t }).unwrap();
    // the trash taken out / a wipe-down clears the floor before she gets there
    w.dirt = 0.0;
    for _ in 0..1500 {
        w.tick(&c);
        if !karolina_busy(&w) {
            break;
        }
    }
    assert!(!karolina_busy(&w), "she gave the swipe up");
    let ev = w.drain_events();
    assert!(
        !ev.iter().any(|e| matches!(e, SimEvent::Mopped { .. })),
        "no swipe over a clean floor: {ev:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// Wash: the same sink, the same dishes
// ---------------------------------------------------------------------------------------------

fn sink_uid(w: &World, c: &Content) -> u32 {
    w.furniture
        .iter()
        .find(|p| c.furniture_def(&p.def).is_some_and(|d| d.has_tag("sink")))
        .expect("a sink")
        .uid
}

fn dishes_credit(w: &World, i: usize) -> u32 {
    w.staff[i].duty_done[duties::duty_index(Duty::Dishes)]
}

#[test]
fn the_sink_karolina_holds_is_left_to_her_and_each_dish_is_washed_once() {
    let c = content();
    let (mut w, i) = night_with(&c, &[Duty::Dishes]);
    let sink = sink_uid(&w, &c);
    w.sink_dishes = 6;
    queue(&mut w, &c, PlayerAct::Wash { sink }).unwrap();
    let mut washed = 0;
    for _ in 0..3000 {
        w.tick(&c);
        assert!(
            !matches!(w.staff[i].task, Some(Task::Wash { .. })),
            "the helper went to wash at Karolina's sink"
        );
        if w.player().current.as_ref().is_some_and(|c| c.stage == 1) {
            let _ = w.apply(
                &c,
                Command::PlayerWashProgress {
                    sink,
                    seconds: 0.5,
                },
            );
        }
        for e in w.drain_events() {
            if let SimEvent::Washed { dishes } = e {
                washed += dishes;
            }
        }
        if !karolina_busy(&w) {
            break;
        }
    }
    assert_eq!(washed, 6, "six dishes washed, each once");
    assert_eq!(w.sink_dishes, 0);
    assert_eq!(dishes_credit(&w, i), 0, "the helper washed nothing");
}

#[test]
fn a_click_on_the_sink_the_helper_washes_at_is_refused() {
    let c = content();
    let (mut w, i) = night_with(&c, &[Duty::Dishes]);
    let sink = sink_uid(&w, &c);
    w.sink_dishes = 6;
    for _ in 0..10 {
        w.tick(&c);
        if matches!(w.staff[i].task, Some(Task::Wash { .. })) {
            break;
        }
    }
    assert!(matches!(w.staff[i].task, Some(Task::Wash { .. })));
    assert_eq!(
        queue(&mut w, &c, PlayerAct::Wash { sink }),
        Err(CommandError::InUse)
    );
    assert_eq!(w.work_holder(&PlayerAct::Wash { sink }), Some(w.staff[i].id));
    for _ in 0..3000 {
        w.tick(&c);
        if w.sink_dishes == 0 && w.staff[i].task.as_ref().is_none_or(|t| t.id() == "idle") {
            break;
        }
    }
    assert_eq!(dishes_credit(&w, i), 6, "the helper washed all six, once");
    assert!(!karolina_busy(&w));
}

// ---------------------------------------------------------------------------------------------
// Orders: the same cup to make, the same plate to carry
// ---------------------------------------------------------------------------------------------

/// A seated party of two that ordered two black coffees (Karolina took the order).
fn two_coffees(w: &mut World, c: &Content) -> (u32, Vec<u32>) {
    let pid = guests::spawn_party(w, c, "students", 2, None, 1.0, 1.0, &[], None, false)
        .expect("spawned");
    {
        let p = w.parties.iter_mut().find(|p| p.id == pid).unwrap();
        p.order_kind = OrderKind::Plain;
        p.wish_open = false;
        p.wishes = vec!["black_coffee".into(), "black_coffee".into()];
    }
    guests::settle_now(w, pid);
    let me = w.karolina().unwrap().id;
    guests::order_taken(w, c, pid, me);
    let items: Vec<u32> = w
        .items
        .iter()
        .filter(|i| i.party == pid)
        .map(|i| i.id)
        .collect();
    assert_eq!(items.len(), 2);
    (pid, items)
}

fn coffee_recipe(w: &World, c: &Content) -> String {
    c.handson
        .recipes_for_item("black_coffee")
        .find(|r| orders::recipe_available(w, c, r) && orders::known(w, &r.id))
        .expect("a black coffee recipe")
        .id
        .clone()
}

#[test]
fn karolina_never_makes_the_cup_a_helper_is_making() {
    let c = content();
    let (mut w, i) = night_with(&c, &[Duty::Bar, Duty::Serving]);
    let (_, items) = two_coffees(&mut w, &c);
    let me = w.staff[i].id;
    for _ in 0..10 {
        w.tick(&c);
        if matches!(w.staff[i].task, Some(Task::Prepare { .. })) {
            break;
        }
    }
    let Some(Task::Prepare { item: his, .. }) = w.staff[i].task.clone() else {
        panic!("the barista started a cup: {:?}", w.staff[i].task);
    };
    let guest = w.items.iter().find(|x| x.id == his).unwrap().guest;
    let recipe = coffee_recipe(&w, &c);
    // a brew for that very guest: she makes the party's other, free coffee
    w.apply(
        &c,
        Command::PlayerBrew {
            guest,
            recipe: recipe.clone(),
            quality: 0.8,
            mode: BrewMode::Quick,
        },
    )
    .unwrap();
    let hers = match her_acts(&w).first() {
        Some(PlayerAct::Brew { item, extra }) => {
            assert!(!extra.contains(&his), "his cup on her tray");
            *item
        }
        a => panic!("{a:?}"),
    };
    assert_ne!(hers, his, "she took the cup he is making");
    assert!(items.contains(&hers));
    assert_eq!(w.items.iter().find(|x| x.id == his).unwrap().by, Some(me));
    // nothing left to make: a third brew is refused
    assert!(
        w.apply(
            &c,
            Command::PlayerBrew {
                guest,
                recipe,
                quality: 0.8,
                mode: BrewMode::Quick,
            },
        )
        .is_err()
    );
    let mut served = Vec::new();
    for _ in 0..6000 {
        w.tick(&c);
        for x in w.items.iter().filter(|x| x.state == ItemState::Served) {
            if !served.contains(&x.id) {
                served.push(x.id);
            }
        }
        if served.len() == 2 {
            break;
        }
    }
    served.sort();
    let mut want = items.clone();
    want.sort();
    assert_eq!(served, want, "each coffee made and served once");
    let made: Vec<_> = w.items.iter().filter(|x| x.party == items[0]).collect();
    assert!(made.len() <= 2);
}

/// A ready plate a helper made, on the counter, for one guest of a seated party.
fn ready_plate(w: &mut World, c: &Content) -> u32 {
    let (_, items) = two_coffees(w, c);
    let at = w.karolina().unwrap().tile();
    let x = w.items.iter_mut().find(|x| x.id == items[0]).unwrap();
    x.state = ItemState::Ready;
    x.prep_left = 0.0;
    x.by = None;
    x.ready_tile = Some(at);
    x.origin = ItemOrigin::Helper { quality: 0.8 };
    // the other cup isn't ready yet (and nobody makes it here)
    let y = w.items.iter_mut().find(|x| x.id == items[1]).unwrap();
    y.state = ItemState::Cancelled;
    items[0]
}

#[test]
fn a_plate_the_helper_carries_is_not_carried_by_karolina_too() {
    let c = content();
    let (mut w, i) = night_with(&c, &[Duty::Serving]);
    let plate = ready_plate(&mut w, &c);
    for _ in 0..10 {
        w.tick(&c);
        if matches!(w.staff[i].task, Some(Task::Serve { .. })) {
            break;
        }
    }
    assert!(
        matches!(w.staff[i].task, Some(Task::Serve { item, .. }) if item == plate),
        "{:?}",
        w.staff[i].task
    );
    let serve = PlayerAct::Serve {
        item: plate,
        extra: Vec::new(),
    };
    assert_eq!(queue(&mut w, &c, serve.clone()), Err(CommandError::InUse));
    assert_eq!(w.work_holder(&serve), Some(w.staff[i].id));
    let mut served = 0;
    for _ in 0..3000 {
        w.tick(&c);
        served = w
            .items
            .iter()
            .filter(|x| x.id == plate && x.state == ItemState::Served)
            .count();
        if served == 1 && !karolina_busy(&w) {
            break;
        }
    }
    assert_eq!(served, 1);
    assert_eq!(w.staff[i].duty_done[duties::duty_index(Duty::Serving)], 1);
}

#[test]
fn a_plate_karolina_carries_is_left_to_her() {
    let c = content();
    let (mut w, i) = night_with(&c, &[Duty::Serving]);
    let plate = ready_plate(&mut w, &c);
    queue(
        &mut w,
        &c,
        PlayerAct::Serve {
            item: plate,
            extra: Vec::new(),
        },
    )
    .unwrap();
    let mut seen = BTreeSet::new();
    for _ in 0..3000 {
        w.tick(&c);
        assert!(
            !matches!(w.staff[i].task, Some(Task::Serve { item, .. }) if item == plate),
            "the helper picked up her plate"
        );
        for x in w.items.iter().filter(|x| x.state == ItemState::Served) {
            seen.insert(x.id);
        }
        if !karolina_busy(&w) {
            break;
        }
    }
    assert_eq!(seen, BTreeSet::from([plate]));
    assert_eq!(w.staff[i].duty_done[duties::duty_index(Duty::Serving)], 0);
}

// ---------------------------------------------------------------------------------------------
// Checklist items (wipe the tables)
// ---------------------------------------------------------------------------------------------

/// `wipe_tables` open again (nobody on it).
fn reopen(w: &mut World, id: &str) {
    let x = w.app.checklist.iter_mut().find(|x| x.id == id).unwrap();
    x.done = false;
    x.claim = None;
}

fn ticked(ev: &[SimEvent], id: &str) -> usize {
    ev.iter()
        .filter(|e| matches!(e, SimEvent::ChecklistTicked { id: x } if x == id))
        .count()
}

#[test]
fn a_checklist_item_karolina_queued_is_left_to_her_and_done_once() {
    let c = content();
    let (mut w, i) = night_with(&c, &[Duty::Floor]);
    reopen(&mut w, "wipe_tables");
    w.dirt = 0.0;
    let done_before = w.tonight.checklist_done;
    w.apply(
        &c,
        Command::TickChecklist {
            id: "wipe_tables".into(),
        },
    )
    .unwrap();
    let mut ev = Vec::new();
    for _ in 0..3000 {
        w.tick(&c);
        bot::finish_cutscenes(&mut w, &c);
        assert!(
            !matches!(&w.staff[i].task, Some(Task::Checklist { id, .. }) if id == "wipe_tables"),
            "the helper went to wipe the tables Karolina is on"
        );
        ev.extend(w.drain_events());
        if !karolina_busy(&w) {
            break;
        }
    }
    assert_eq!(ticked(&ev, "wipe_tables"), 1);
    let x = w.app.checklist.iter().find(|x| x.id == "wipe_tables").unwrap();
    assert!(x.done && x.by == w.karolina().map(|k| k.id));
    assert_eq!(w.tonight.checklist_done, done_before + 1);
}

#[test]
fn a_checklist_item_the_helper_does_is_not_ticked_twice() {
    let c = content();
    let (mut w, i) = night_with(&c, &[Duty::Floor]);
    reopen(&mut w, "wipe_tables");
    let done_before = w.tonight.checklist_done;
    for _ in 0..10 {
        w.tick(&c);
        if matches!(&w.staff[i].task, Some(Task::Checklist { id, .. }) if id == "wipe_tables") {
            break;
        }
    }
    assert!(
        matches!(&w.staff[i].task, Some(Task::Checklist { id, .. }) if id == "wipe_tables"),
        "{:?}",
        w.staff[i].task
    );
    let tick = Command::TickChecklist {
        id: "wipe_tables".into(),
    };
    assert_eq!(w.apply(&c, tick), Err(CommandError::InUse));
    let act = PlayerAct::Checklist {
        id: "wipe_tables".into(),
    };
    assert_eq!(w.work_holder(&act), Some(w.staff[i].id));
    let mut ev = Vec::new();
    for _ in 0..3000 {
        w.tick(&c);
        bot::finish_cutscenes(&mut w, &c);
        ev.extend(w.drain_events());
        if w.app.checklist.iter().any(|x| x.id == "wipe_tables" && x.done) {
            break;
        }
    }
    assert_eq!(ticked(&ev, "wipe_tables"), 1);
    assert_eq!(w.tonight.checklist_done, done_before + 1);
    assert!(!karolina_busy(&w));
}

// ---------------------------------------------------------------------------------------------
// Tip coins on a table the dishes helper clears
// ---------------------------------------------------------------------------------------------

#[test]
fn the_tip_karolina_goes_for_is_hers_while_the_helper_may_clear_the_cups() {
    let c = content();
    let (mut w, i) = night_with(&c, &[Duty::Dishes]);
    let me = w.karolina().unwrap().id;
    let table = w
        .furniture
        .iter()
        .find(|p| c.furniture_def(&p.def).is_some_and(|d| d.has_tag("table")))
        .unwrap()
        .uid;
    w.furniture
        .iter_mut()
        .find(|p| p.uid == table)
        .unwrap()
        .dishes = 2;
    let id = w.alloc_id();
    w.hands.tips.push(hands::TipCoin {
        id,
        table,
        amount: 20,
        staff: None,
        minute: w.minute(),
        napkin: None,
    });
    queue(&mut w, &c, PlayerAct::CollectTip { table }).unwrap();
    let mut collected = Vec::new();
    for _ in 0..3000 {
        w.tick(&c);
        for e in w.drain_events() {
            if let SimEvent::TipCollected { by, .. } = e {
                collected.push(by);
            }
        }
        let cleared = w.furniture.iter().any(|p| p.uid == table && p.dishes == 0);
        if !karolina_busy(&w) && cleared {
            break;
        }
    }
    assert!(!karolina_busy(&w));
    assert_eq!(collected, vec![Some(me)], "the coin went into the jar once, by her");
    assert!(w.hands.tips.is_empty());
    // the cups didn't wait for her: the dishes helper cleared them meanwhile
    assert!(w.furniture.iter().any(|p| p.uid == table && p.dishes == 0));
    assert!(w.staff[i].carrying.is_some() || w.sink_dishes > 0 || dishes_credit(&w, i) > 0);
}

// ---------------------------------------------------------------------------------------------
// The whole night: nothing held by two people at once
// ---------------------------------------------------------------------------------------------

#[test]
fn a_busy_crew_night_never_has_two_people_on_the_same_work() {
    let c = content();
    for seed in [1u64, 2, 3] {
        let mut w = World::new_sandbox(
            &c,
            SandboxSetup {
                seed,
                ..Default::default()
            },
        );
        bot::finish_cutscenes(&mut w, &c);
        while w.staff.iter().filter(|s| !s.owner).count() < 3 {
            let s = staff::generate_staff(&mut w, &c, None);
            w.push_staff(s);
        }
        let ids: Vec<u32> = w.staff.iter().filter(|s| !s.owner).map(|s| s.id).collect();
        let plan: [&[Duty]; 3] = [
            &[Duty::Bar, Duty::Kitchen],
            &[Duty::Serving, Duty::Register],
            &[Duty::Floor, Duty::Dishes],
        ];
        for (k, id) in ids.iter().take(3).enumerate() {
            let s = w.staff.iter_mut().find(|s| s.id == *id).unwrap();
            s.duties = plan[k].to_vec();
            s.nights_worked = 5;
            s.skills = [7.0; 5];
        }
        w.start_night(&c);
        bot::finish_cutscenes(&mut w, &c);
        for s in w.staff.iter_mut().filter(|s| ids.contains(&s.id)) {
            s.status = StaffStatus::Expected { minute: 0 };
            s.shift = Shift::Full;
            s.leave_at = NIGHT_MINUTES;
            s.late_minutes = 0;
        }
        let mut b = bot::PlayerBot::new(bot::BotConfig {
            hire_nights: Vec::new(),
            seed,
            ..Default::default()
        });
        let mut served = 0;
        while w.phase() == Phase::Night {
            b.step(&mut w, &c);
            w.tick(&c);
            bot::finish_cutscenes(&mut w, &c);
            let clash = claims::clashes(&w);
            assert!(clash.is_empty(), "seed {seed}, {}: {clash:?}", w.clock());
            served += w
                .drain_events()
                .iter()
                .filter(|e| matches!(e, SimEvent::PlayerServed { .. }))
                .count();
        }
        assert!(served > 0, "seed {seed}: Karolina served too");
    }
}
