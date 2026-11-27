//! Hands-on appliances (design/v0.2.md §B, `sim::equipment`): every chore by hand, its
//! effect, the helpers' smaller version, the timing windows, saves, and the player-bot.

use super::equipment::{self, Cake, EquipAction};
use super::*;
use crate::content::Content;
use crate::content::handson::Duty;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

fn content() -> Content {
    Content::load(DATA).unwrap_or_else(|e| panic!("{e}"))
}

fn ticks(w: &mut World, c: &Content, n: u32) -> Vec<SimEvent> {
    let mut out = Vec::new();
    for _ in 0..n {
        w.tick(c);
        bot::finish_cutscenes(w, c);
        out.extend(w.drain_events());
    }
    out
}

/// A story night `n` at 21:30 (cutscenes done), the tutorial off.
fn story_evening(c: &Content, n: u32) -> World {
    let mut w = World::new_story(c, Difficulty::Cozy);
    w.sys.tutorial_off = true;
    if n > 1 {
        w.night = n;
        w.begin_day(c);
    }
    bot::finish_cutscenes(&mut w, c);
    w.start_night(c);
    bot::finish_cutscenes(&mut w, c);
    w.drain_events();
    w
}

/// The same night with the doors open (22:00, the preparation played through).
fn story_night(c: &Content, n: u32) -> World {
    let mut w = story_evening(c, n);
    w.skip_prep(c);
    bot::finish_cutscenes(&mut w, c);
    w.drain_events();
    w
}

fn uid_with(w: &World, c: &Content, tag: &str) -> u32 {
    w.furniture()
        .iter()
        .find(|p| c.furniture_def(&p.def).is_some_and(|d| d.has_tag(tag)))
        .map(|p| p.uid)
        .unwrap_or_else(|| panic!("no {tag}"))
}

fn idle(w: &World) -> bool {
    w.player().current.is_none() && w.player().queue.is_empty()
}

/// Ticks until Karolina's hands are empty; returns the events.
fn until_done(w: &mut World, c: &Content, limit: u32) -> Vec<SimEvent> {
    let mut out = Vec::new();
    for _ in 0..limit {
        if idle(w) {
            return out;
        }
        out.extend(ticks(w, c, 1));
    }
    panic!("Karolina is still busy after {limit} ticks");
}

fn used(ev: &[SimEvent], action: EquipAction) -> Option<Option<u32>> {
    ev.iter().find_map(|e| match e {
        SimEvent::EquipmentUsed { action: a, by, .. } if *a == action => Some(*by),
        _ => None,
    })
}

fn cue(ev: &[SimEvent], which: EquipCue) -> bool {
    ev.iter()
        .any(|e| matches!(e, SimEvent::EquipmentCue { cue, .. } if *cue == which))
}

fn checklist_done(w: &World, id: &str) -> Option<Option<u32>> {
    w.app()
        .checklist
        .iter()
        .find(|x| x.id == id && x.done)
        .map(|x| x.by)
}

fn use_it(w: &mut World, c: &Content, uid: u32, action: EquipAction) -> Result<(), CommandError> {
    w.apply(c, Command::UseEquipment { uid, action })
}

/// The night clock to this night minute (ticks through guests and all).
fn to_minute(w: &mut World, c: &Content, minute: u32) {
    let mut guard = 0;
    while w.minute() < minute && w.phase() == Phase::Night && guard < NIGHT_TICKS * 2 {
        ticks(w, c, 1);
        guard += 1;
    }
}

fn place_oven(w: &mut World, c: &Content) -> u32 {
    w.econ.money += 20_000;
    let (gw, gh) = (w.grid().width, w.grid().height);
    for y in (10..gh).rev() {
        for x in 1..gw - 1 {
            let cmd = Command::PlaceFurniture {
                def: "oven".into(),
                x,
                y,
                rot: 0,
            };
            if w.apply(c, cmd).is_ok() {
                return uid_with(w, c, "station:oven");
            }
        }
    }
    panic!("no room for an oven");
}

#[test]
fn warming_the_machine_up_by_hand_in_the_preparation() {
    let c = content();
    let mut w = story_evening(&c, 1);
    let machine = uid_with(&w, &c, "station:coffee");
    assert_eq!(
        w.equipment_action(&c, machine),
        Some(EquipAction::WarmUp),
        "the opening item is the first thing the machine offers"
    );
    use_it(&mut w, &c, machine, EquipAction::WarmUp).unwrap();
    // queued twice is refused, and the item is claimed (helpers leave it alone)
    assert!(use_it(&mut w, &c, machine, EquipAction::WarmUp).is_err());
    let me = w.karolina().unwrap().id;
    assert!(
        w.app()
            .checklist
            .iter()
            .any(|x| x.id == "coffee_warmup" && x.claim == Some(me))
    );
    let ev = until_done(&mut w, &c, 600);
    assert_eq!(used(&ev, EquipAction::WarmUp), Some(Some(me)));
    assert_eq!(checklist_done(&w, "coffee_warmup"), Some(Some(me)));
    assert!(w.equipment().warm, "its little lamp is lit");
    assert!(w.in_prep());
    assert_eq!(w.equipment_action(&c, machine), None);
}

#[test]
fn holding_makes_a_hold_chore_quicker() {
    let c = content();
    let mut took = Vec::new();
    for hold in [false, true] {
        let mut w = story_evening(&c, 1);
        let machine = uid_with(&w, &c, "station:coffee");
        use_it(&mut w, &c, machine, EquipAction::WarmUp).unwrap();
        let mut n = 0;
        while !idle(&w) {
            if hold {
                let _ = w.apply(&c, Command::EquipmentHold { seconds: 0.1 });
            }
            ticks(&mut w, &c, 1);
            n += 1;
            assert!(n < 600);
        }
        took.push(n);
    }
    assert!(took[1] < took[0], "held {} vs {}", took[1], took[0]);
    // not at a hold chore: refused gently
    let mut w = story_evening(&c, 1);
    assert!(
        w.apply(&c, Command::EquipmentHold { seconds: 1.0 })
            .is_err()
    );
}

#[test]
fn a_tray_pulled_ahead_makes_the_next_coffees_at_once() {
    let c = content();
    let t = c.handson.tuning.equipment.tray.clone();
    // before the doors open there's no tray to pull (nobody to drink it)
    let mut early = story_evening(&c, 3);
    let machine = uid_with(&early, &c, "station:coffee");
    assert!(use_it(&mut early, &c, machine, EquipAction::PullTray).is_err());
    let mut w = story_night(&c, 3);
    // the warm-up is done by now (a helper or not): the night click pulls a tray
    for x in w.app.checklist.iter_mut() {
        if x.id == "coffee_warmup" {
            x.done = true;
        }
    }
    while !idle(&w) {
        let _ = w.apply(&c, Command::PlayerCancel);
        let _ = w.apply(&c, Command::PlayerUnqueue { index: 0 });
    }
    assert_eq!(w.equipment_action(&c, machine), Some(EquipAction::PullTray));
    use_it(&mut w, &c, machine, EquipAction::PullTray).unwrap();
    let ev = until_done(&mut w, &c, 600);
    assert!(used(&ev, EquipAction::PullTray).is_some());
    let left = w.equipment().tray;
    assert!(left > 0 && left <= t.count);
    w.hands.equip.tray = t.count;
    let muls: Vec<f32> = (0..t.count + 1)
        .map(|_| equipment::item_prep_mul(&mut w, &c, "black_coffee", true))
        .collect();
    assert!(
        muls[..t.count as usize].iter().all(|m| *m < 0.1),
        "{muls:?}"
    );
    assert_eq!(muls[t.count as usize], 1.0, "only the tray is instant");
    // a wish brew or a tea doesn't take from the tray
    w.hands.equip.tray = 2;
    assert_eq!(
        equipment::item_prep_mul(&mut w, &c, "black_coffee", false),
        1.0
    );
    assert_eq!(equipment::item_prep_mul(&mut w, &c, "lemon_tea", true), 1.0);
    assert_eq!(w.equipment().tray, 2);
    // it goes cold for the next train
    let until = w.equipment().tray_until.max(0) as u32;
    to_minute(&mut w, &c, until + 1);
    assert_eq!(w.equipment().tray, 0);
}

#[test]
fn purging_a_worn_machine_resets_the_wear_for_free() {
    let c = content();
    let mut w = story_night(&c, 12);
    let machine = uid_with(&w, &c, "station:coffee");
    for x in w.app.checklist.iter_mut() {
        x.done = true;
    }
    while !idle(&w) {
        let _ = w.apply(&c, Command::PlayerCancel);
        let _ = w.apply(&c, Command::PlayerUnqueue { index: 0 });
    }
    w.debug_set_wear(&c, "station:coffee", 0.45);
    assert_ne!(w.equipment_action(&c, machine), Some(EquipAction::Service));
    w.debug_set_wear(&c, "station:coffee", 0.62);
    // the worn machine's event: a slow-down and a question
    super::narrator::fire(&mut w, &c, "coffee_machine", EffectCtx::default(), true);
    let slow = |w: &World| {
        w.mods
            .iter()
            .any(|m| matches!(&m.kind, ModKind::StationSlow(t) if t == "station:coffee"))
    };
    assert!(slow(&w));
    assert!(
        w.pending_choices()
            .iter()
            .any(|p| p.event == "coffee_machine")
    );
    assert_eq!(w.equipment_action(&c, machine), Some(EquipAction::Service));
    let money = w.money();
    use_it(&mut w, &c, machine, EquipAction::Service).unwrap();
    let ev = until_done(&mut w, &c, 900);
    assert!(used(&ev, EquipAction::Service).is_some());
    assert_eq!(w.furniture_by_uid(machine).unwrap().wear, 0.0);
    assert!(!slow(&w));
    assert!(
        !w.pending_choices()
            .iter()
            .any(|p| p.event == "coffee_machine")
    );
    assert!(ev.iter().any(|e| matches!(
        e,
        SimEvent::ChoiceResolved { id, choice } if id == "coffee_machine" && choice == "purged"
    )));
    assert!(w.money() >= money - 200, "no repair bill");
}

#[test]
fn a_worn_machine_sputters_now_and_then() {
    let c = content();
    let mut w = story_night(&c, 12);
    w.debug_set_wear(&c, "station:coffee", 0.7);
    let every = c.handson.tuning.equipment.sputter_every;
    let mut ev = Vec::new();
    while w.minute() <= every + 1 {
        ev.extend(ticks(&mut w, &c, 1));
    }
    assert!(cue(&ev, EquipCue::Sputter));
}

#[test]
fn the_soup_pot_bubbles_and_a_stir_makes_soup_quicker() {
    let c = content();
    let s = c.handson.tuning.equipment.soup.clone();
    let mut w = story_evening(&c, 1);
    let stove = uid_with(&w, &c, "station:stove");
    assert_eq!(w.equipment_action(&c, stove), Some(EquipAction::SoupOn));
    use_it(&mut w, &c, stove, EquipAction::SoupOn).unwrap();
    until_done(&mut w, &c, 600);
    assert!(w.equipment().pot.as_ref().is_some_and(|p| p.by_hand));
    assert_eq!(
        equipment::item_prep_mul(&mut w, &c, "garlic_soup", true),
        s.warm_mul
    );
    assert_eq!(equipment::item_prep_mul(&mut w, &c, "topinka", true), 1.0);
    // no bubbles while the doors are closed; then now and then
    let ev = ticks(&mut w, &c, 50);
    assert!(!cue(&ev, EquipCue::Bubbling));
    w.skip_prep(&c);
    let mut guard = 0;
    while !cue(&ticks(&mut w, &c, 1), EquipCue::Bubbling) {
        guard += 1;
        assert!(guard < 10 * 60 * 2, "it never bubbled");
    }
    let first = w.minute();
    assert!(first >= 1 && first <= s.bubble_every, "{first}");
    while !idle(&w) {
        let _ = w.apply(&c, Command::PlayerCancel);
        let _ = w.apply(&c, Command::PlayerUnqueue { index: 0 });
    }
    assert_eq!(w.equipment_action(&c, stove), Some(EquipAction::Stir));
    use_it(&mut w, &c, stove, EquipAction::Stir).unwrap();
    until_done(&mut w, &c, 900);
    assert_eq!(
        equipment::item_prep_mul(&mut w, &c, "garlic_soup", true),
        s.stir_mul
    );
    // the next bubbling, left alone: it simply keeps warm (no penalty, no fuss)
    let mut bubbled = false;
    let end = first + s.bubble_every * 2 + s.bubble_minutes + 2;
    while w.minute() < end {
        bubbled |= cue(&ticks(&mut w, &c, 1), EquipCue::Bubbling);
    }
    assert!(bubbled);
    assert_ne!(w.equipment_action(&c, stove), Some(EquipAction::Stir));
    let m = equipment::item_prep_mul(&mut w, &c, "garlic_soup", true);
    assert!(m <= 1.0 && m >= s.stir_mul, "{m}");
}

#[test]
fn the_cake_dings_and_comes_out_in_slices() {
    let c = content();
    let k = c.handson.tuning.equipment.cake.clone();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    w.sys.tutorial_off = true;
    w.night = 3;
    w.begin_day(&c);
    bot::finish_cutscenes(&mut w, &c);
    let oven = place_oven(&mut w, &c);
    w.start_night(&c);
    bot::finish_cutscenes(&mut w, &c);
    w.drain_events();
    assert_eq!(w.equipment_action(&c, oven), Some(EquipAction::Bake));
    use_it(&mut w, &c, oven, EquipAction::Bake).unwrap();
    until_done(&mut w, &c, 600);
    assert!(matches!(
        w.equipment().cake,
        Cake::Baking { by_hand: true, .. }
    ));
    let mut ev = Vec::new();
    let mut guard = 0;
    while !matches!(w.equipment().cake, Cake::Ready { .. }) {
        ev.extend(ticks(&mut w, &c, 1));
        guard += 1;
        // (in the preparation a game minute is QUIET_TICKS_PER_MINUTE ticks)
        assert!(guard < QUIET_TICKS_PER_MINUTE * (k.bake_minutes + 2) * 2);
    }
    assert!(ev.iter().any(|e| matches!(
        e,
        SimEvent::EquipmentCue { cue: EquipCue::Ding, uid } if *uid == oven
    )));
    assert_eq!(w.equipment_action(&c, oven), Some(EquipAction::TakeOut));
    use_it(&mut w, &c, oven, EquipAction::TakeOut).unwrap();
    until_done(&mut w, &c, 600);
    assert_eq!(w.equipment().cake, Cake::Out { slices: k.slices });
    for _ in 0..k.slices {
        assert!(equipment::item_prep_mul(&mut w, &c, "medovnik", true) < 0.1);
    }
    assert_eq!(equipment::item_prep_mul(&mut w, &c, "medovnik", true), 1.0);
}

#[test]
fn the_milk_out_of_the_fridge_and_the_stock_check() {
    let c = content();
    let m = c.handson.tuning.equipment.milk.clone();
    let mut w = story_evening(&c, 2);
    let fridge = uid_with(&w, &c, "station:fridge");
    assert_eq!(w.equipment_action(&c, fridge), Some(EquipAction::Milk));
    use_it(&mut w, &c, fridge, EquipAction::Milk).unwrap();
    until_done(&mut w, &c, 600);
    let me = w.karolina().unwrap().id;
    assert_eq!(checklist_done(&w, "check_stock"), Some(Some(me)));
    assert_eq!(w.equipment().milk_left, m.count);
    for _ in 0..m.count {
        assert_eq!(
            equipment::item_prep_mul(&mut w, &c, "cappuccino", true),
            m.prep_mul
        );
    }
    assert_eq!(
        equipment::item_prep_mul(&mut w, &c, "cappuccino", true),
        1.0
    );
    assert_eq!(w.equipment_action(&c, fridge), None);
}

#[test]
fn closing_count_the_till_and_take_the_trash_out() {
    let mut c = content();
    c.handson.tuning.equipment.closing.cat_chance = 1.0;
    let mut w = story_night(&c, 2);
    w.auto_doors = false;
    let register = uid_with(&w, &c, "register");
    let bin = uid_with(&w, &c, "trash");
    // not while guests sit: closing time (the closing half hour from 06:00)
    to_minute(&mut w, &c, 120);
    assert_eq!(w.equipment_action(&c, bin), None);
    assert!(use_it(&mut w, &c, register, EquipAction::CountTill).is_err());
    to_minute(&mut w, &c, NIGHT_MINUTES);
    assert!(w.in_closing());
    while !idle(&w) {
        let _ = w.apply(&c, Command::PlayerCancel);
        let _ = w.apply(&c, Command::PlayerUnqueue { index: 0 });
    }
    assert_eq!(
        w.equipment_action(&c, register),
        Some(EquipAction::CountTill)
    );
    use_it(&mut w, &c, register, EquipAction::CountTill).unwrap();
    let ev = until_done(&mut w, &c, 900);
    let me = w.karolina().unwrap().id;
    let revenue = w.tonight().revenue;
    assert!(ev.iter().any(|e| matches!(
        e,
        SimEvent::TillCounted { revenue: r, .. } if *r <= revenue
    )));
    assert!(checklist_done(&w, "cash_up").is_some());
    assert_eq!(w.equipment_action(&c, register), None, "once a night");
    // the trash: out of the door, a moment outside, the cat by the bins
    let counter = c.handson.extras.cat.counter.clone();
    let cat_before = w.story().counters.get(&counter).copied().unwrap_or(0);
    let dirt = w.dirt();
    use_it(&mut w, &c, bin, EquipAction::Trash).unwrap();
    let mut outside = false;
    let mut ev = Vec::new();
    for _ in 0..1500 {
        if idle(&w) {
            break;
        }
        let k = w.karolina().unwrap().tile();
        outside |= w
            .grid()
            .tile(k.0, k.1)
            .is_some_and(|t| t.kind == TileKind::Street);
        ev.extend(ticks(&mut w, &c, 1));
    }
    assert!(outside, "she steps out of the door");
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::TrashOut { cat: true }))
    );
    assert_eq!(used(&ev, EquipAction::Trash), Some(Some(me)));
    assert!(w.equipment().trash_out);
    assert!(w.dirt() <= dirt);
    let after = w.story().counters.get(&counter).copied().unwrap_or(0);
    assert!(after > cat_before, "the cat came for a scratch");
    assert_eq!(w.equipment_action(&c, bin), None, "once a night");
}

#[test]
fn the_kitchen_helper_does_the_chores_she_skipped_but_smaller() {
    let c = content();
    let e = c.handson.tuning.equipment.clone();
    let mut w = World::new_sandbox(&c, SandboxSetup::default());
    bot::finish_cutscenes(&mut w, &c);
    let ids: Vec<u32> = w.candidates().iter().map(|x| x.id).take(1).collect();
    for id in ids {
        w.apply(&c, Command::Hire { candidate: id }).unwrap();
    }
    for s in w.staff.iter_mut().filter(|s| !s.owner) {
        s.duties = vec![Duty::Kitchen];
    }
    w.start_night(&c);
    bot::finish_cutscenes(&mut w, &c);
    w.drain_events();
    // (a late helper would come after 22:00: make them punctual for the test)
    for s in w.staff.iter_mut().filter(|s| !s.owner) {
        if matches!(s.status, StaffStatus::Expected { .. }) {
            s.status = StaffStatus::Expected { minute: 0 };
            s.late_minutes = 0;
        }
    }
    w.skip_prep(&c);
    let ev = ticks(&mut w, &c, 2);
    let helper = w
        .staff()
        .iter()
        .find(|s| !s.owner && s.on_duty())
        .map(|s| s.id)
        .expect("a helper on shift");
    let pot = w
        .equipment()
        .pot
        .clone()
        .expect("the helper put the soup on");
    assert!(!pot.by_hand);
    assert_eq!(used(&ev, EquipAction::SoupOn), Some(Some(helper)));
    assert!(w.equipment().milk_done);
    assert!(w.equipment().milk_left <= e.milk.helper_count);
    assert!(e.milk.helper_count < e.milk.count && e.soup.helper_warm_mul > e.soup.warm_mul);
    assert_eq!(
        equipment::item_prep_mul(&mut w, &c, "garlic_soup", true),
        e.soup.helper_warm_mul
    );
}

#[test]
fn nothing_to_do_outside_the_night_or_at_the_wrong_moment() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    bot::finish_cutscenes(&mut w, &c);
    let stove = uid_with(&w, &c, "station:stove");
    let machine = uid_with(&w, &c, "station:coffee");
    assert_eq!(w.phase(), Phase::Day);
    assert_eq!(w.equipment_action(&c, stove), None);
    assert!(use_it(&mut w, &c, stove, EquipAction::SoupOn).is_err());
    let mut w = story_night(&c, 1);
    // stirring a pot that isn't there / isn't bubbling
    assert!(use_it(&mut w, &c, stove, EquipAction::Stir).is_err());
    // a chore that belongs to another appliance
    assert!(use_it(&mut w, &c, machine, EquipAction::Bake).is_err());
    // a lamp has no chore: it toggles
    let lamp = w
        .furniture()
        .iter()
        .find(|p| p.def == "pendant_lamp")
        .map(|p| p.uid)
        .unwrap();
    assert_eq!(w.equipment_kind(&c, lamp), Some(EquipKind::Lamp));
    assert_eq!(w.equipment_action(&c, lamp), None);
    // too late for a new pot after the until minute
    to_minute(&mut w, &c, c.handson.tuning.equipment.soup.until_minute + 1);
    if w.equipment().pot.is_none() {
        assert_ne!(w.equipment_action(&c, stove), Some(EquipAction::SoupOn));
    }
}

#[test]
fn a_lamp_click_changes_the_light_and_the_coziness() {
    let c = content();
    let mut w = story_night(&c, 4);
    ticks(&mut w, &c, 10);
    let before = w.coziness().value;
    let lamps: Vec<u32> = w
        .furniture()
        .iter()
        .filter(|p| p.def == "pendant_lamp" && p.on)
        .map(|p| p.uid)
        .collect();
    assert!(!lamps.is_empty());
    for uid in &lamps {
        w.apply(
            &c,
            Command::SetFurnitureOn {
                uid: *uid,
                on: false,
            },
        )
        .unwrap();
    }
    ticks(&mut w, &c, 10);
    let dark = w.coziness().value;
    assert!(dark < before, "{before} → {dark}");
    for uid in &lamps {
        w.apply(
            &c,
            Command::SetFurnitureOn {
                uid: *uid,
                on: true,
            },
        )
        .unwrap();
    }
    ticks(&mut w, &c, 10);
    assert!(w.coziness().value > dark);
}

#[test]
fn appliance_state_survives_a_save_and_older_saves_load() {
    let c = content();
    let mut w = story_evening(&c, 1);
    let stove = uid_with(&w, &c, "station:stove");
    use_it(&mut w, &c, stove, EquipAction::SoupOn).unwrap();
    until_done(&mut w, &c, 600);
    w.hands.equip.tray = 2;
    let text = w.save_to_string().unwrap();
    let mut back = World::load_from_str(&text).unwrap();
    assert_eq!(back.equipment(), w.equipment());
    // the continuation is identical
    ticks(&mut w, &c, 200);
    ticks(&mut back, &c, 200);
    assert_eq!(back.equipment(), w.equipment());
    // 0.1.x hands-on state (no `equip` field) loads with the appliances idle
    let old: HandsState = ron::from_str("()").unwrap();
    assert_eq!(old.equip, EquipState::default());
}

#[test]
fn views_follow_the_evening() {
    let c = content();
    let mut w = story_evening(&c, 1);
    let stove = uid_with(&w, &c, "station:stove");
    let machine = uid_with(&w, &c, "station:coffee");
    let view = |w: &World, uid: u32| {
        w.equipment_views(&c)
            .into_iter()
            .find(|v| v.uid == uid)
            .unwrap()
    };
    assert_eq!(view(&w, stove).pot, PotView::None);
    assert_eq!(view(&w, stove).action, Some(EquipAction::SoupOn));
    use_it(&mut w, &c, machine, EquipAction::WarmUp).unwrap();
    let mut saw_working = false;
    for _ in 0..400 {
        saw_working |= view(&w, machine).working.is_some();
        ticks(&mut w, &c, 1);
    }
    assert!(saw_working);
    assert!(view(&w, machine).warm);
    w.hands.equip.pot = Some(Pot {
        by_hand: true,
        ..Default::default()
    });
    assert_eq!(view(&w, stove).pot, PotView::Warm);
}

#[test]
fn every_listed_item_is_on_the_menu() {
    let c = content();
    let e = &c.handson.tuning.equipment;
    for id in e
        .tray
        .items
        .iter()
        .chain(&e.milk.items)
        .chain(&e.soup.items)
        .chain(&e.cake.items)
    {
        assert!(c.menu_item(id).is_some(), "{id}");
    }
}

#[test]
fn the_player_bot_uses_the_appliances() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    let mut b = bot::PlayerBot::new(bot::BotConfig::default());
    let mut by_hand = 0;
    let mut kinds = std::collections::BTreeSet::new();
    for _ in 0..4 {
        let ev = b.run_night(&mut w, &c);
        let me = w.karolina().map(|k| k.id);
        for e in &ev {
            if let SimEvent::EquipmentUsed { action, by, .. } = e
                && *by == me
            {
                by_hand += 1;
                kinds.insert(action.id());
            }
        }
    }
    assert!(by_hand >= 4, "{by_hand}");
    assert!(kinds.len() >= 3, "{kinds:?}");
}

/// A player who never touches an appliance still reaches «Light in the Window» (Cozy).
#[test]
fn a_story_without_the_appliances_still_ends_in_the_light() {
    let c = content();
    let mut w = World::new_story_seeded(&c, Difficulty::Cozy, 4242);
    let mut b = bot::PlayerBot::new(bot::BotConfig {
        seed: 4242,
        equipment: false,
        ..Default::default()
    });
    let mut used = 0;
    for _ in 1..=30 {
        if w.is_closed() {
            break;
        }
        let ev = b.run_night(&mut w, &c);
        let me = w.karolina().map(|k| k.id);
        used += ev
            .iter()
            .filter(|e| matches!(e, SimEvent::EquipmentUsed { by, .. } if *by == me))
            .count();
    }
    bot::finish_cutscenes(&mut w, &c);
    assert_eq!(used, 0, "she never used an appliance");
    assert_eq!(w.ending(), Some("light"));
}
