//! The fridge keeps the stock (`sim::fridge`): fresh stock beyond what the fridges hold goes
//! off overnight, a second fridge cuts it (and the morning says what it saved), a worn or
//! failed fridge holds less, the closing fridge check helps, grandma's fridge in good repair
//! holds a usual morning's stock, the state survives a save, and every new key has its four
//! languages.

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

/// A night under way (the checklist fresh, nothing done yet).
fn night(c: &Content, n: u32) -> World {
    let mut w = day(c, n);
    w.start_night(c);
    bot::finish_cutscenes(&mut w, c);
    w.drain_events();
    w
}

fn set_check(w: &mut World, done: bool) {
    let id = purchases::data().fridge.check_item.clone();
    for x in w.app.checklist.iter_mut().filter(|x| x.id == id) {
        x.done = done;
    }
}

/// 06:00 for the fridge alone, with `supplies` Kč in stock; returns what went off.
fn morning(w: &mut World, c: &Content, supplies: i64) -> fridge::FridgeNight {
    w.econ.supplies = supplies;
    fridge::on_night_end(w, c);
    w.fridge_last_night().clone()
}

/// A free kitchen / bar floor tile on a row with a station (for a second fridge).
fn free_bar_tile(w: &World, c: &Content) -> (i32, i32) {
    let g = w.grid();
    for y in (10..g.height - 2).rev() {
        let machines = (0..g.width).any(|x| {
            g.object_at(x, y).is_some_and(|u| {
                w.furniture_by_uid(u)
                    .and_then(|p| c.furniture_def(&p.def))
                    .is_some_and(|d| d.tags.iter().any(|t| t.starts_with("station:")))
            })
        });
        if !machines {
            continue;
        }
        for x in 16..g.width - 1 {
            if g.object_at(x, y).is_none()
                && g.tile(x, y).is_some_and(|t| {
                    t.kind == TileKind::Floor && matches!(t.room, Room::Bar | Room::Kitchen)
                })
            {
                return (x, y);
            }
        }
    }
    panic!("no free tile behind the bar");
}

fn second_fridge(w: &mut World, c: &Content) {
    let (x, y) = free_bar_tile(w, c);
    w.apply(
        c,
        Command::PlaceFurniture {
            def: "fridge".into(),
            x,
            y,
            rot: 2,
        },
    )
    .unwrap_or_else(|e| panic!("fridge at {x},{y}: {e:?}"));
}

#[test]
fn the_data_parses_and_grandmas_fridge_holds_a_usual_morning() {
    let c = content();
    let d = purchases::parse(include_str!("../../assets/data/purchases.ron"))
        .unwrap()
        .fridge;
    assert_eq!(d.tag, "station:fridge");
    assert!(d.fresh_share > 0.0 && d.fresh_share < 1.0);
    assert!(d.spoil > 0.0 && d.spoil <= 1.0);
    assert!(d.check_mul > 0.0 && d.check_mul < 1.0);
    assert!(d.failed_mul > 0.0 && d.failed_mul < 1.0);
    // the fail event and the check item exist
    assert!(c.events.iter().any(|e| e.id == d.fail_event));
    assert!(c.app.checklists.iter().any(|x| x.id == d.check_item));
    // "worn" is when the fridge can be defrosted (the appliances' service)
    assert_eq!(d.worn_at, c.handson.tuning.equipment.service_wear);
    // grandma's café: one fridge in good repair
    let w = World::new_story(&c, Difficulty::Cozy);
    let f = w.fridge(&c);
    assert_eq!(f.pieces, 1);
    assert_eq!(f.capacity, d.per_piece.round() as i64);
    assert!(!f.worn());
    // the auto-restock target less a light night's use fits — nothing goes off
    let target = w.econ.auto_restock.unwrap();
    assert_eq!(
        fridge::spoil_of(fridge::fresh_of(target - 300), f.capacity, false),
        0
    );
    // the fridge's build card says what it does
    let def = c.furniture_def("fridge").unwrap();
    let room = d.per_piece.round() as i64;
    assert_eq!(w.piece_effect(&c, def).fridge, Some((room, room * 2)));
    assert_eq!(
        w.piece_effect(&c, c.furniture_def("chair").unwrap()).fridge,
        None
    );
}

#[test]
fn fresh_stock_beyond_the_fridge_goes_off_overnight() {
    let c = content();
    let d = &purchases::data().fridge;
    let mut w = night(&c, 4);
    set_check(&mut w, false);
    let rng = w.rng.clone();
    // a usual morning: nothing
    let f = morning(&mut w, &c, 2500);
    assert_eq!((f.spoiled, f.saved), (0, 0));
    assert_eq!(w.econ.supplies, 2500);
    assert!(fridge::lines(&w).is_empty());
    // bulk: 5 000 Kč in stock → 2 000 fresh, 1 100 room → half of the 900 over goes off
    let f = morning(&mut w, &c, 5000);
    let fresh = (5000.0 * d.fresh_share).round() as i64;
    let over = fresh - d.per_piece.round() as i64;
    assert_eq!(f.fresh, fresh);
    assert_eq!(f.spoiled, (over as f32 * d.spoil).round() as i64);
    assert!(f.spoiled > 0);
    assert_eq!(w.econ.supplies, 5000 - f.spoiled);
    assert_eq!(f.cause, fridge::SpoilCause::Full);
    // the morning's line: litres, the money as a loss
    let lines = fridge::lines(&w);
    assert_eq!(lines.len(), 1);
    let l = &lines[0].1;
    assert_eq!(l.key, "purchases.line.spoiled_full");
    assert_eq!(l.money, -f.spoiled);
    let litres: i64 = l
        .args
        .iter()
        .find(|a| a.0 == "l")
        .unwrap()
        .1
        .parse()
        .unwrap();
    assert_eq!(litres, (f.spoiled as f32 / d.litre).round() as i64);
    // nothing random
    assert_eq!(w.rng, rng);
}

#[test]
fn a_second_fridge_cuts_it_and_the_morning_says_what_it_saved() {
    let c = content();
    let mut one = night(&c, 6);
    set_check(&mut one, false);
    let alone = morning(&mut one, &c, 5000);
    assert!(alone.spoiled > 0);

    let mut w = day(&c, 6);
    second_fridge(&mut w, &c);
    assert_eq!(w.fridge(&c).pieces, 2);
    w.start_night(&c);
    bot::finish_cutscenes(&mut w, &c);
    set_check(&mut w, false);
    let two = morning(&mut w, &c, 5000);
    assert_eq!(two.spoiled, 0, "2 000 fresh fit into two fridges");
    assert_eq!(two.saved, alone.spoiled);
    let lines = fridge::lines(&w);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].1.key, "purchases.line.fridge_saved");
    assert_eq!(lines[0].1.money, alone.spoiled);
    // even more stock: some still goes off, the second fridge kept the rest
    let more = morning(&mut w, &c, 9000);
    assert!(more.spoiled > 0 && more.saved > 0);
    let keys: Vec<String> = fridge::lines(&w).into_iter().map(|l| l.1.key).collect();
    assert_eq!(
        keys,
        vec![
            "purchases.line.fridge_saved_part".to_string(),
            "purchases.line.spoiled_full".to_string()
        ]
    );
}

#[test]
fn a_worn_or_failed_fridge_holds_less() {
    let c = content();
    let d = &purchases::data().fridge;
    let mut w = night(&c, 8);
    set_check(&mut w, false);
    let good = morning(&mut w, &c, 2700);
    assert_eq!(good.spoiled, 0);
    // a little wear: a little less room, not yet "worn"
    w.debug_set_wear(&c, &d.tag, 0.3);
    let f = w.fridge(&c);
    assert!(!f.worn() && f.capacity < d.per_piece as i64);
    let some = morning(&mut w, &c, 2700);
    assert_eq!(some.cause, fridge::SpoilCause::Full);
    // worn (it can be defrosted now): the room shrinks more, the line blames the wear
    w.debug_set_wear(&c, &d.tag, 0.5);
    let f = w.fridge(&c);
    assert!(f.worn());
    assert_eq!(
        f.capacity,
        (d.per_piece * fridge::condition(0.5)).round() as i64
    );
    assert!(f.capacity < d.per_piece as i64);
    let worn = morning(&mut w, &c, 2700);
    assert!(worn.spoiled > some.spoiled);
    assert_eq!(worn.cause, fridge::SpoilCause::Worn);
    assert_eq!(fridge::lines(&w)[0].1.key, "purchases.line.spoiled_worn");
    // it failed tonight: half the room again for the morning
    w.tonight.events.push(d.fail_event.clone());
    let failed = morning(&mut w, &c, 2700);
    assert_eq!(failed.cause, fridge::SpoilCause::Failed);
    assert_eq!(
        failed.capacity,
        (d.per_piece * fridge::condition(0.5) * d.failed_mul).round() as i64
    );
    assert!(failed.spoiled > worn.spoiled);
    // the condition never drops below its floor
    assert!(fridge::condition(1.0) >= d.min_condition);
}

#[test]
fn the_closing_fridge_check_helps_a_little() {
    let c = content();
    let mut w = night(&c, 5);
    set_check(&mut w, false);
    let skipped = morning(&mut w, &c, 5000);
    set_check(&mut w, true);
    let checked = morning(&mut w, &c, 5000);
    assert!(checked.checked && !skipped.checked);
    assert!(checked.spoiled > 0 && checked.spoiled < skipped.spoiled);
    assert_eq!(
        checked.spoiled,
        (skipped.spoiled as f32 * purchases::data().fridge.check_mul).round() as i64
    );
}

/// A whole night: overfull stock at the doors, the morning's summary carries the line and the
/// supplies are lower; the tally survives a save.
#[test]
fn a_night_with_too_much_stock_ends_with_the_line_and_survives_a_save() {
    let c = content();
    let mut w = night(&c, 6);
    w.mods.push(Modifier {
        kind: ModKind::GuestRate,
        mul: 0.0,
        until: None,
        event: None,
    });
    w.econ.supplies = 6000;
    w.run_night_headless(&c);
    assert_eq!(w.phase(), Phase::Day);
    let f = w.fridge_last_night().clone();
    assert!(f.spoiled > 0, "{f:?}");
    let s = w.last_summary().unwrap();
    assert!(
        s.payoff
            .iter()
            .any(|l| l.key.starts_with("purchases.line.spoiled") && l.money == -f.spoiled),
        "{:?}",
        s.payoff
    );
    assert!(w.supplies() <= 6000 - f.spoiled);
    let text = w.save_to_string().unwrap();
    let back = World::load_from_str(&text).unwrap();
    assert_eq!(back.fridge_last_night(), &f);
    assert_eq!(back.pay, w.pay);
    // the next night starts with a clean tally
    let mut next = back;
    next.start_night(&c);
    assert_eq!(next.fridge_last_night(), &fridge::FridgeNight::default());
}

/// A save from before the fridge (no `fridge` in the tally) loads with an empty one.
#[test]
fn an_old_tally_without_the_fridge_loads() {
    let t: purchases::PayoffTally = ron::from_str("(extra_parties: 2)").unwrap();
    assert_eq!(t.extra_parties, 2);
    assert_eq!(t.fridge, fridge::FridgeNight::default());
}

#[test]
fn every_fridge_key_has_four_languages() {
    let i18n = crate::i18n::I18n::load(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/i18n"))
        .unwrap_or_else(|e| panic!("{e}"));
    let keys = [
        "purchases.line.fridge_saved",
        "purchases.line.fridge_saved_part",
        "purchases.line.spoiled_full",
        "purchases.line.spoiled_worn",
        "purchases.line.spoiled_failed",
        "purchases.card.fridge",
        "fridge.plan.label",
        "fridge.plan.worn",
        "fridge.pantry",
    ];
    for lang in crate::i18n::Lang::ALL {
        for k in keys {
            assert!(i18n.has(lang, k), "{k} missing in {}", lang.code());
        }
    }
}
