//! QA combos: where features meet (build mode at night × seated guests × appliances, the
//! storm × stories × names, the house drink × the pantry × saves, helpers × Karolina's
//! chores, the back step × rushes, the attic × undo, difficulties × saves). Each scenario
//! sets the situation up, runs it through the sim and checks that nothing breaks: no panic,
//! the state stays consistent, the night goes on and a save round-trips.

use super::guests;
use super::*;
use crate::content::Content;
use crate::content::handson::MiniGameKind;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

fn content() -> Content {
    Content::load(DATA).unwrap_or_else(|e| panic!("{e}"))
}

/// The day before story night `n` (cutscenes done, the tutorial off).
fn story_day(c: &Content, n: u32, diff: Difficulty) -> World {
    let mut w = World::new_story(c, diff);
    w.sys.tutorial_off = true;
    if n > 1 {
        w.night = n;
        w.begin_day(c);
    }
    bot::finish_cutscenes(&mut w, c);
    w.drain_events();
    w
}

/// Into the night, doors open; `quiet` = no random walk-ins.
fn into_night(w: &mut World, c: &Content, quiet: bool) {
    w.start_night(c);
    bot::finish_cutscenes(w, c);
    w.skip_prep(c);
    if quiet {
        w.mods.push(Modifier {
            kind: ModKind::GuestRate,
            mul: 0.0,
            until: None,
            event: None,
        });
    }
    w.drain_events();
}

fn night(c: &Content, n: u32, quiet: bool) -> World {
    let mut w = story_day(c, n, Difficulty::Cozy);
    into_night(&mut w, c, quiet);
    w
}

fn ticks(w: &mut World, c: &Content, n: u32) -> Vec<SimEvent> {
    let mut out = Vec::new();
    for _ in 0..n {
        bot::finish_cutscenes(w, c);
        w.tick(c);
        out.extend(w.drain_events());
        check(w, c);
    }
    out
}

fn to_minute(w: &mut World, c: &Content, m: u32) -> Vec<SimEvent> {
    let mut out = Vec::new();
    let mut guard = 0;
    while w.phase() == Phase::Night && w.minute() < m && guard < NIGHT_TICKS * 2 {
        out.extend(ticks(w, c, 1));
        guard += 1;
    }
    out
}

/// Runs the rest of the night; the next day must start.
fn to_morning(w: &mut World, c: &Content) -> Vec<SimEvent> {
    let mut out = Vec::new();
    let mut guard = 0;
    while w.phase() == Phase::Night && guard < NIGHT_TICKS * 3 {
        out.extend(ticks(w, c, 1));
        guard += 1;
    }
    bot::finish_cutscenes(w, c);
    out.extend(w.drain_events());
    assert_eq!(w.phase(), Phase::Day, "the night ended");
    assert!(w.parties().is_empty(), "nobody left inside in the morning");
    out
}

/// A seated stranger party (`size`): (party, leader).
fn seated(w: &mut World, c: &Content, arch: &str, size: u32) -> (u32, u32) {
    let pid =
        guests::spawn_party(w, c, arch, size, None, 4.0, 1.0, &[], None, true).expect("spawned");
    for _ in 0..900 {
        let sat = w
            .parties
            .iter()
            .find(|p| p.id == pid)
            .is_some_and(|p| p.state == PartyState::WaitOrder)
            && w.guests
                .iter()
                .filter(|g| g.party == pid)
                .all(|g| g.seat.is_some() && g.pose == Pose::Sit);
        if sat {
            break;
        }
        ticks(w, c, 1);
    }
    let leader = w.parties.iter().find(|p| p.id == pid).unwrap().members[0];
    (pid, leader)
}

/// The world's cross references hold.
fn check(w: &World, c: &Content) {
    let uids: Vec<u32> = w.furniture.iter().map(|p| p.uid).collect();
    for g in &w.guests {
        if let Some((uid, idx)) = g.seat {
            let p = w.furniture_by_uid(uid);
            assert!(p.is_some(), "guest {} sits on a missing piece {uid}", g.id);
            let p = p.unwrap();
            assert!(
                p.seat_by.get(idx as usize).copied().flatten() == Some(g.id) || g.gone,
                "guest {} at {uid}/{idx} but the seat says {:?}",
                g.id,
                p.seat_by
            );
        }
        assert!(
            w.parties.iter().any(|p| p.id == g.party) || g.gone,
            "guest {} of a missing party",
            g.id
        );
    }
    for p in &w.furniture {
        for s in p.seat_by.iter().flatten() {
            assert!(
                w.guests.iter().any(|g| g.id == *s && g.seat.map(|x| x.0) == Some(p.uid)),
                "seat of {} ({}) held by guest {s} who isn't there",
                p.uid,
                p.def
            );
        }
    }
    for s in &w.stories.active {
        assert!(
            w.guests.iter().any(|g| g.id == s.guest),
            "a story on a guest who left"
        );
    }
    // Karolina's acts name things that exist
    let acts = w
        .hands
        .player
        .queue
        .iter()
        .chain(w.hands.player.current.iter().map(|c| &c.act));
    for a in acts {
        match a {
            // (an act at a piece removed meanwhile is dropped when she gets to it)
            PlayerAct::Use { uid, .. } | PlayerAct::Wash { sink: uid } if !uids.contains(uid) => {}
            PlayerAct::TakeOrder { party } => {
                assert!(
                    w.parties.iter().any(|p| p.id == *party),
                    "an order of a party that left: {a:?}"
                )
            }
            _ => {}
        }
    }
    let _ = c;
}

fn first_of(w: &World, c: &Content, tag: &str) -> Option<u32> {
    w.furniture
        .iter()
        .find(|p| c.furniture_def(&p.def).is_some_and(|d| d.has_tag(tag)))
        .map(|p| p.uid)
}

fn round_trip(w: &World, c: &Content) -> World {
    let s = w.save_to_string().unwrap();
    let mut back = World::load_from_str(&s).unwrap();
    back.rebuild_caches(c);
    assert_eq!(back.save_to_string().unwrap(), s, "a save round-trips");
    back
}

// ───────────────────────── build mode at night ─────────────────────────

/// Seated guests: their chairs refuse to move or go; the table's group drag refuses; the
/// night goes on.
#[test]
fn build_at_night_refuses_pieces_in_use() {
    let c = content();
    let mut w = night(&c, 4, true);
    let (pid, leader) = seated(&mut w, &c, "travelers", 2);
    let seat = w.guests.iter().find(|g| g.id == leader).unwrap().seat.unwrap().0;
    assert_eq!(w.in_use_reason(seat), Some(InUseReason::Seated));
    assert_eq!(
        w.apply(&c, Command::RemoveFurniture { uid: seat }),
        Err(CommandError::InUse)
    );
    assert_eq!(
        w.apply(&c, Command::RotateFurniture { uid: seat }),
        Err(CommandError::InUse)
    );
    let p = w.furniture_by_uid(seat).unwrap().clone();
    assert_eq!(
        w.apply(
            &c,
            Command::MoveFurniture {
                uid: seat,
                x: p.x + 1,
                y: p.y,
                rot: p.rot
            }
        ),
        Err(CommandError::InUse)
    );
    // the table of that chair: its group (with the taken chair) can't move
    let table = guests::seats(&w, &c)
        .iter()
        .find(|s| s.uid == seat)
        .and_then(|s| s.table);
    if let Some(t) = table {
        let group = w.move_group(&c, t);
        assert!(group.contains(&seat), "the chair belongs to its table's group");
        let tp = w.furniture_by_uid(t).unwrap().clone();
        let moves = w.group_moves(&c, t, (tp.x + 1, tp.y), 0);
        assert_eq!(
            w.apply(&c, Command::MoveGroup { moves }),
            Err(CommandError::InUse)
        );
    }
    check(&w, &c);
    assert!(w.parties.iter().any(|p| p.id == pid));
    to_morning(&mut w, &c);
}

/// The table where a party sits (the table itself has no seat): moving or removing it under
/// them must not break the night — dishes / the coin go to the counter, the party finishes.
#[test]
fn removing_the_table_under_a_seated_party() {
    let c = content();
    let mut w = night(&c, 4, true);
    let (pid, leader) = seated(&mut w, &c, "travelers", 2);
    let seat = w.guests.iter().find(|g| g.id == leader).unwrap().seat.unwrap().0;
    let table = guests::seats(&w, &c)
        .iter()
        .find(|s| s.uid == seat)
        .and_then(|s| s.table)
        .expect("a chair at a table");
    // a story on the leader too
    w.debug_story("call", leader);
    let r = w.apply(&c, Command::RemoveFurniture { uid: table });
    // either refused (in use) or the night copes
    if r.is_ok() {
        check(&w, &c);
        w.undo(&c).expect("undo puts it back");
        assert!(w.furniture_by_uid(table).is_some());
    }
    check(&w, &c);
    // take the order, serve, let them leave with the table gone again
    w.apply(&c, Command::PlayerTakeOrder { guest: leader }).unwrap();
    let _ = w.apply(&c, Command::RemoveFurniture { uid: table });
    ticks(&mut w, &c, 1500);
    let _ = pid;
    to_morning(&mut w, &c);
}

/// Regression: a tip coin on a table that is then removed stayed on the missing table till
/// 06:00 (invisible, unreachable, and the lull director waited on it). Now it goes into the
/// jar with the removal.
#[test]
fn a_coin_on_a_removed_table() {
    let c = content();
    let mut w = night(&c, 4, true);
    let table = first_of(&w, &c, "table").unwrap();
    let id = w.alloc_id();
    w.hands.tips.push(super::hands::TipCoin {
        id,
        table,
        amount: 40,
        staff: None,
        minute: w.minute(),
        napkin: None,
    });
    let jar = w.tips_lifetime();
    let removed = w.apply(&c, Command::RemoveFurniture { uid: table });
    assert!(removed.is_ok(), "{removed:?}");
    assert_eq!(w.tips_lifetime(), jar + 40, "the coin went into the jar");
    if removed.is_ok() {
        let lifetime = w.tips_lifetime();
        // the coin doesn't stay on a table that's gone
        let stuck = w.tips_on_tables().iter().any(|t| t.table == table);
        let collect = w.apply(&c, Command::PlayerQueue {
            act: PlayerAct::CollectTip { table },
            front: false,
        });
        ticks(&mut w, &c, 400);
        assert!(
            !w.tips_on_tables().iter().any(|t| t.table == table),
            "the coin of a removed table is stuck (stuck={stuck}, collect={collect:?})"
        );
        assert!(w.tips_lifetime() >= lifetime);
    }
    to_morning(&mut w, &c);
}

/// The stove goes while the soup pot is on, the oven while the cake bakes, the fridge after
/// the milk: nothing panics, the views stay sane, the undo puts them back.
#[test]
fn removing_appliances_mid_chore() {
    let c = content();
    let mut w = story_day(&c, 6, Difficulty::Cozy);
    w.debug_set_money(200_000);
    // an oven
    'place: for y in [13, 14] {
        for x in (22..28).rev() {
            if w
                .apply(
                    &c,
                    Command::PlaceFurniture {
                        def: "oven".into(),
                        x,
                        y,
                        rot: 2,
                    },
                )
                .is_ok()
            {
                break 'place;
            }
        }
    }
    into_night(&mut w, &c, true);
    let stove = first_of(&w, &c, "station:stove").expect("a stove");
    let oven = first_of(&w, &c, "station:oven").expect("an oven");
    let fridge = first_of(&w, &c, "station:fridge").expect("a fridge");
    for (uid, a) in [
        (stove, super::equipment::EquipAction::SoupOn),
        (oven, super::equipment::EquipAction::Bake),
        (fridge, super::equipment::EquipAction::Milk),
    ] {
        let r = w.apply(&c, Command::UseEquipment { uid, action: a });
        assert!(r.is_ok(), "{a:?}: {r:?}");
    }
    // she walks off to do them; remove each piece while she's on her way / at it
    ticks(&mut w, &c, 30);
    for uid in [stove, oven, fridge] {
        let r = w.apply(&c, Command::RemoveFurniture { uid });
        check(&w, &c);
        if r.is_ok() {
            let _ = w.equipment_views(&c);
            assert_eq!(w.equipment_action(&c, uid), None);
        }
    }
    ticks(&mut w, &c, 600);
    let _ = w.equipment_views(&c);
    // all back
    while w.undo(&c).is_ok() {}
    check(&w, &c);
    for uid in [stove, oven, fridge] {
        assert!(w.furniture_by_uid(uid).is_some(), "{uid} is back");
    }
    let _ = round_trip(&w, &c);
    to_morning(&mut w, &c);
}

/// The pot and the cake done first, then the pieces go: soups/cakes aren't offered from a
/// stove/oven that isn't there, the night goes on, a save round-trips.
#[test]
fn the_pot_on_a_stove_that_left() {
    let c = content();
    let mut w = night(&c, 6, true);
    let stove = first_of(&w, &c, "station:stove").expect("a stove");
    w.apply(
        &c,
        Command::UseEquipment {
            uid: stove,
            action: super::equipment::EquipAction::SoupOn,
        },
    )
    .unwrap();
    ticks(&mut w, &c, 500);
    assert!(w.equipment().pot.is_some(), "the pot is on");
    let r = w.apply(&c, Command::RemoveFurniture { uid: stove });
    if r.is_ok() {
        // no stove: no stir either
        assert!(
            w.equipment_views(&c)
                .iter()
                .all(|v| v.kind != super::equipment::EquipKind::Stove)
        );
    }
    let back = round_trip(&w, &c);
    let mut w = back;
    to_morning(&mut w, &c);
}

/// Placing pieces at night opens / closes corners and the back room path mid-night; the
/// guests keep finding their way.
#[test]
fn corners_and_paths_change_mid_night() {
    let c = content();
    let mut w = story_day(&c, 16, Difficulty::Cozy);
    w.debug_set_money(300_000);
    let _ = w.apply(&c, Command::OpenBackRoom);
    w.start_night(&c);
    bot::finish_cutscenes(&mut w, &c);
    w.skip_prep(&c);
    w.drain_events();
    ticks(&mut w, &c, 200);
    // put a lamp and an armchair here and there, then take them away again
    let mut placed = Vec::new();
    for (def, x, y) in [
        ("floor_lamp", 3, 3),
        ("armchair", 4, 3),
        ("floor_lamp", 12, 10),
        ("plant_tall", 8, 8),
    ] {
        for dx in 0..4 {
            if w
                .apply(
                    &c,
                    Command::PlaceFurniture {
                        def: def.into(),
                        x: x + dx,
                        y,
                        rot: 0,
                    },
                )
                .is_ok()
            {
                placed.push(w.furniture.last().unwrap().uid);
                break;
            }
        }
        ticks(&mut w, &c, 50);
    }
    let _ = w.coziness();
    for uid in placed {
        let _ = w.apply(&c, Command::RemoveFurniture { uid });
        ticks(&mut w, &c, 40);
    }
    to_morning(&mut w, &c);
}

/// The attic: buy a piece, place it, undo — money and the attic list stay consistent.
#[test]
fn attic_purchase_then_undo() {
    let c = content();
    let mut w = story_day(&c, 14, Difficulty::Cozy);
    w.debug_set_money(100_000);
    let offers: Vec<String> = w.attic(&c).offers.iter().map(|o| o.id.clone()).collect();
    let Some(id) = offers.first().cloned() else {
        return;
    };
    let before = w.money();
    let n = w.furniture.len();
    w.apply(&c, Command::BuyAtticPiece { id: id.clone() }).unwrap();
    let after_buy = w.money();
    let r = w.undo(&c);
    // whatever undo does, it doesn't hand out free money or duplicate the piece
    assert!(w.money() <= before, "undo after the attic: {} > {before}", w.money());
    if r.is_ok() && w.furniture.len() < n + 1 {
        // the piece went back: the purchase must be refunded or offered again
        let again = w.attic(&c).upstairs.iter().any(|o| o.id == id);
        assert!(
            again || w.money() > after_buy,
            "the piece vanished: not refunded and not offered again"
        );
    }
    let _ = round_trip(&w, &c);
}

// ───────────────────────── equipment × orders × helpers ─────────────────────────

/// A tray of coffees pulled ahead while a wish waits; the purge while a brew runs; the
/// helper's kitchen chores after Karolina already did them; two pots never.
#[test]
fn chores_interleaved_with_orders() {
    let c = content();
    let mut w = story_day(&c, 9, Difficulty::Cozy);
    // a kitchen helper
    if let Some(cand) = w.candidates().first().map(|x| x.staff.id) {
        let _ = w.apply(&c, Command::Hire { candidate: cand });
    }
    let helper = w.staff().iter().find(|s| !s.owner).map(|s| s.id);
    if let Some(h) = helper {
        let _ = w.apply(
            &c,
            Command::SetDuty {
                staff: h,
                duty: crate::content::handson::Duty::Kitchen,
                on: true,
            },
        );
    }
    w.debug_set_wear(&c, "station:coffee", 0.95);
    w.start_night(&c);
    bot::finish_cutscenes(&mut w, &c);
    // Karolina puts the soup on in the preparation
    let stove = first_of(&w, &c, "station:stove").unwrap();
    let _ = w.apply(
        &c,
        Command::UseEquipment {
            uid: stove,
            action: super::equipment::EquipAction::SoupOn,
        },
    );
    w.skip_prep(&c);
    w.drain_events();
    ticks(&mut w, &c, 200);
    let pot_by_hand = w.equipment().pot.as_ref().map(|p| p.by_hand);
    // a wish guest and two plain guests
    let (_, wish_leader) = seated(&mut w, &c, "travelers", 1);
    if let Some(p) = w.parties.iter_mut().find(|p| p.leader() == Some(wish_leader)) {
        p.order_kind = OrderKind::Wish {
            wish: "kind_coffee".into(),
        };
        p.wish_open = true;
    }
    let machine = first_of(&w, &c, "station:coffee").unwrap();
    let _ = w.apply(
        &c,
        Command::UseEquipment {
            uid: machine,
            action: super::equipment::EquipAction::PullTray,
        },
    );
    w.apply(&c, Command::PlayerTakeOrder { guest: wish_leader })
        .unwrap();
    // purge (service) the worn machine queued behind
    let _ = w.apply(
        &c,
        Command::UseEquipment {
            uid: machine,
            action: super::equipment::EquipAction::Service,
        },
    );
    let mut brewed = false;
    for _ in 0..1200 {
        let ev = ticks(&mut w, &c, 1);
        if ev
            .iter()
            .any(|e| matches!(e, SimEvent::PlayerOrderTaken { .. }))
            && !brewed
        {
            let r = w.best_matches(&c, wish_leader);
            if let Some((rec, _)) = r.first() {
                let res = w.apply(
                    &c,
                    Command::PlayerBrew {
                        guest: wish_leader,
                        recipe: rec.clone(),
                        quality: 0.8,
                        mode: BrewMode::ByHand,
                    },
                );
                assert!(res.is_ok() || res == Err(CommandError::QueueFull), "{res:?}");
                brewed = true;
            }
        }
    }
    // only one pot, still marked as hers if she put it on
    if let Some(b) = pot_by_hand {
        assert_eq!(w.equipment().pot.as_ref().map(|p| p.by_hand), Some(b));
    }
    // a second SoupOn is refused while the pot is on
    if w.equipment().pot.is_some() {
        assert!(
            w.apply(
                &c,
                Command::UseEquipment {
                    uid: stove,
                    action: super::equipment::EquipAction::SoupOn,
                },
            )
            .is_err()
        );
    }
    to_morning(&mut w, &c);
}

// ───────────────────────── the house drink ─────────────────────────

fn compose(w: &mut World, c: &Content, jars: &[&str], name: &str) -> Result<(), CommandError> {
    w.apply(
        c,
        Command::ComposeSignature {
            base: "coffee".into(),
            jars: jars.iter().map(|s| s.to_string()).collect(),
            style: MiniGameKind::MilkFroth,
            name: name.into(),
        },
    )
}

/// Composed then renamed the same night; a week later reworked; a save between; asked for by
/// name while the supplies are gone — no panic, the wish is fair, the name survives.
#[test]
fn the_house_drink_renamed_saved_and_out_of_supplies() {
    let c = content();
    let mut w = story_day(&c, 6, Difficulty::Cozy);
    compose(&mut w, &c, &["milk", "honey"], "Noc").unwrap();
    compose(&mut w, &c, &["milk", "honey"], "Noční vlak č. 333 — pro Mirka").unwrap_or(());
    let long = "Наш нічний поїзд з медом і молоком";
    let r = compose(&mut w, &c, &["milk", "honey"], long);
    let name_now = w.signature().unwrap().name.clone();
    assert!(r.is_ok() || name_now != long);
    let mut w = round_trip(&w, &c);
    into_night(&mut w, &c, true);
    // the supplies run out: nothing is orderable, the house drink isn't available
    w.econ.supplies = 0;
    let mirek = c.regular("mirek").unwrap().base.clone();
    let pid = guests::spawn_party(&mut w, &c, &mirek, 1, Some("mirek"), 1.0, 1.0, &[], None, true)
        .expect("spawned");
    if let Some(p) = w.parties.iter_mut().find(|p| p.id == pid) {
        p.order_kind = OrderKind::Wish {
            wish: "house_special".into(),
        };
        p.wish_open = true;
    }
    let leader = w.parties.iter().find(|p| p.id == pid).unwrap().leader().unwrap();
    ticks(&mut w, &c, 200);
    let _ = w.apply(&c, Command::PlayerTakeOrder { guest: leader });
    for _ in 0..900 {
        let ev = ticks(&mut w, &c, 1);
        if ev
            .iter()
            .any(|e| matches!(e, SimEvent::PlayerOrderTaken { .. }))
        {
            break;
        }
    }
    let m = w.best_matches(&c, leader);
    for (r, _) in &m {
        assert!(w.recipe_available(&c, r), "an unavailable chip {r}");
    }
    let r = w.apply(
        &c,
        Command::PlayerBrew {
            guest: leader,
            recipe: SIGNATURE_RECIPE.into(),
            quality: 0.9,
            mode: BrewMode::ByHand,
        },
    );
    assert!(r.is_err(), "brewing the house drink with no supplies: {r:?}");
    // hand-off / cancel and the night goes on
    w.econ.supplies = 5_000;
    to_morning(&mut w, &c);
    // a week later it can change
    w.night += 7;
    w.begin_day(&c);
    bot::finish_cutscenes(&mut w, &c);
    compose(&mut w, &c, &["milk", "cinnamon"], "Noc II").unwrap_or(());
    let _ = round_trip(&w, &c);
}

// ───────────────────────── stories, names and the storm ─────────────────────────

/// A story on a guest whose name is known already; the answer walk while the guest leaves.
#[test]
fn a_story_on_a_known_guest_who_leaves_mid_walk() {
    let c = content();
    let mut w = night(&c, 10, true);
    let (pid, leader) = seated(&mut w, &c, "travelers", 1);
    if let Some(g) = w.guests.iter_mut().find(|g| g.id == leader)
        && let Some(p) = g.persona.as_mut()
    {
        p.known = true;
    }
    assert!(w.debug_story("call", leader));
    let a = w.story_of(leader).unwrap().clone();
    let def = w.story_def(&a.story).unwrap();
    let ans = def.answers[0].id.clone();
    // Karolina busy first, then the answer queued behind
    w.apply(&c, Command::AnswerStory { guest: leader, answer: ans })
        .unwrap();
    // they get up and leave before she arrives
    super::guests::finish_party(&mut w, &c, pid, super::guests::Exit::Normal);
    ticks(&mut w, &c, 600);
    assert!(w.story_of(leader).is_none(), "the story went with them");
    assert!(
        w.hands.player.current.is_none() || w.hands.player.queue.len() < PLAYER_QUEUE_MAX,
        "she isn't stuck"
    );
    to_morning(&mut w, &c);
}

/// Two parties with the same description; a story guest in the back room.
#[test]
fn two_alike_parties_and_a_story_in_the_back_room() {
    let c = content();
    let mut w = story_day(&c, 16, Difficulty::Cozy);
    w.debug_set_money(300_000);
    let opened = w.apply(&c, Command::OpenBackRoom);
    into_night(&mut w, &c, true);
    let (_, a) = seated(&mut w, &c, "students", 1);
    let (_, b) = seated(&mut w, &c, "students", 1);
    if let (Some(pa), Some(pb)) = (w.persona(a).cloned(), w.persona(b).cloned()) {
        let _ = (pa, pb);
    }
    // more guests until someone sits in the back room (if it's open)
    let mut back_guest = None;
    if opened.is_ok() {
        for _ in 0..12 {
            let (_, g) = seated(&mut w, &c, "travelers", 2);
            let tile = w.guests.iter().find(|x| x.id == g).map(|x| x.tile());
            let back = w.build.back_room.clone();
            if tile.is_some_and(|t| back.as_ref().is_some_and(|r| r.contains(t))) {
                back_guest = Some(g);
                break;
            }
        }
    }
    for g in [Some(a), Some(b), back_guest].into_iter().flatten() {
        w.debug_story("call", g);
        if let Some(s) = w.story_of(g).cloned() {
            let ans = w.story_def(&s.story).unwrap().answers[0].id.clone();
            let _ = w.apply(&c, Command::AnswerStory { guest: g, answer: ans });
        }
    }
    ticks(&mut w, &c, 1500);
    to_morning(&mut w, &c);
}

/// The storm talk with 1, 2 and 6 stranded strangers; blankets + soup + a story at once.
#[test]
fn the_storm_with_one_two_or_six_stranded() {
    let c = content();
    let cancel = stories::data().storm.cancel_at.minutes_since_open() as u32;
    let talk_at = stories::data().storm.talk_at.minutes_since_open() as u32;
    for n in [1u32, 2, 6] {
        let mut w = night(&c, 29, true);
        assert!(w.storm().tonight);
        let mut leaders = Vec::new();
        let mut left = n;
        while left > 0 {
            let size = left.min(2);
            let (_, l) = seated(&mut w, &c, "travelers", size);
            leaders.push(l);
            left -= size;
        }
        to_minute(&mut w, &c, cancel + 1);
        assert!(w.storm_line_closed());
        // a story, the blankets and the soup at once
        w.debug_story("call", leaders[0]);
        if let Some(s) = w.story_of(leaders[0]).cloned() {
            let ans = w.story_def(&s.story).unwrap().answers[0].id.clone();
            let _ = w.apply(
                &c,
                Command::AnswerStory {
                    guest: leaders[0],
                    answer: ans,
                },
            );
        }
        w.apply(&c, Command::StormBlankets).unwrap();
        w.apply(&c, Command::StormSoup).unwrap();
        let ev = to_minute(&mut w, &c, talk_at + 1);
        let talk = ev.iter().find_map(|e| match e {
            SimEvent::StormTalk { talk, speakers } => Some((talk.clone(), speakers.clone())),
            _ => None,
        });
        let stranded = w.storm_stranded().len();
        match &talk {
            Some((t, s)) => {
                assert!(!t.is_empty());
                assert!(s.len() >= 2 && s.len() <= stranded, "{n}: {s:?}");
                let def = w.storm_talk_def(t).unwrap();
                let need = def.lines.iter().filter_map(|l| l.who).max().map_or(0, |x| x + 1);
                assert!(s.len() >= need as usize, "{n}: {} speakers for {need}", s.len());
            }
            None => assert!(n < 2, "{n} stranded and no talk"),
        }
        assert!(w.storm().blankets_done && w.storm().soup_done, "{n}: cared");
        let mut w = round_trip(&w, &c);
        to_morning(&mut w, &c);
    }
}

// ───────────────────────── the back step ─────────────────────────

/// The back step while a pot bubbles / the oven dings / a guest waits / the storm closed the
/// line / in a rush (refused): the state stays sane and the night continues.
#[test]
fn the_back_step_in_busy_moments() {
    let c = content();
    // the storm night: the line closed
    let mut w = night(&c, 29, true);
    let stove = first_of(&w, &c, "station:stove").unwrap();
    let _ = w.apply(
        &c,
        Command::UseEquipment {
            uid: stove,
            action: super::equipment::EquipAction::SoupOn,
        },
    );
    let (_, g) = seated(&mut w, &c, "travelers", 1);
    let cancel = stories::data().storm.cancel_at.minutes_since_open() as u32;
    let from = walk::data().backstep.from_minute;
    to_minute(&mut w, &c, cancel.max(from) + 1);
    w.apply(&c, Command::PlayerTakeOrder { guest: g }).ok();
    let r = w.apply(&c, Command::StepOutBack);
    match w.backstep_block(&c) {
        Some(walk::BackstepBlock::AlreadyTonight) => assert!(r.is_ok()),
        b => assert!(r.is_err(), "{b:?}"),
    }
    assert!(w.apply(&c, Command::StepOutBack).is_err(), "once a night");
    to_morning(&mut w, &c);
}

// ───────────────────────── regressions ─────────────────────────

/// Regression: when no table had two free seats, a pair picked "seats near each other" and
/// the first seat was never taken out of the rest (the filter compared the slots holding
/// the references) — both sat on the same chair.
#[test]
fn a_pair_without_a_free_table_never_shares_one_chair() {
    let c = content();
    let mut w = night(&c, 4, true);
    // every table keeps a single free seat
    let seats = guests::seats(&w, &c);
    let mut kept: Vec<u32> = Vec::new();
    for s in &seats {
        let keep = match s.table {
            Some(t) if !kept.contains(&t) => {
                kept.push(t);
                true
            }
            _ => false,
        };
        if !keep && let Some(p) = w.furniture.iter_mut().find(|p| p.uid == s.uid) {
            p.seat_by[s.idx as usize] = Some(u32::MAX);
        }
    }
    let pid = guests::spawn_party(&mut w, &c, "students", 2, None, 1.0, 1.0, &[], None, false)
        .expect("spawned");
    let p = w.parties.iter().find(|p| p.id == pid).unwrap();
    if p.state == PartyState::Leaving {
        return; // turned away: fine, nothing shared
    }
    let taken: Vec<(u32, u8)> = p
        .members
        .iter()
        .filter_map(|g| w.guests.iter().find(|x| x.id == *g).and_then(|x| x.seat))
        .collect();
    assert_eq!(taken.len(), 2);
    assert_ne!(taken[0], taken[1], "two guests on one chair");
}

// ───────────────────────── difficulties × saves × endings ─────────────────────────

/// A whole story played attentively on Carefree (Relaxed) and Classic, with a save → load
/// every morning and once mid-night: the endings come, Carefree hears the 3:33 guest's
/// answer (letter delivered + 8 notes), and the difficulty survives every save.
#[test]
fn whole_stories_through_saves_on_carefree_and_classic() {
    let c = content();
    for (diff, seed) in [(Difficulty::Relaxed, 4242u64), (Difficulty::Classic, 4242)] {
        let mut w = World::new_story_seeded(&c, diff, seed);
        let mut b = bot::PlayerBot::new(bot::BotConfig {
            seed,
            ..Default::default()
        });
        for n in 1..=30 {
            if w.is_closed() {
                break;
            }
            if w.phase() == Phase::Day {
                b.day(&mut w, &c);
                bot::finish_cutscenes_attentive(&mut w, &c);
                w = round_trip(&w, &c);
                assert_eq!(w.difficulty(), diff);
                w.start_night(&c);
            }
            let mut guard = 0;
            while w.phase() == Phase::Night && !w.is_closed() && guard < 100_000 {
                bot::finish_cutscenes_attentive(&mut w, &c);
                b.step(&mut w, &c);
                w.tick(&c);
                w.drain_events();
                guard += 1;
                if n == 15 && w.minute() == 200 && w.playing_cutscene().is_none() {
                    w = round_trip(&w, &c);
                }
            }
            bot::finish_cutscenes_attentive(&mut w, &c);
        }
        assert!(w.ending().is_some(), "{diff:?}: an ending");
        if diff == Difficulty::Relaxed {
            assert_eq!(w.ending(), Some("light"), "Carefree never closes");
            assert!(
                w.story().played.contains("answer"),
                "Carefree: the answer (letter {:?}, notes {})",
                w.story().vars.get("letter"),
                w.story().notes.len()
            );
        }
    }
}

/// Regression: at 22:00 the kitchen helper put the soup on / baked even while Karolina was
/// on her way to do it by hand (her chore queued or under way), so her by-hand effect was
/// lost to the helper's smaller one.
#[test]
fn the_helper_leaves_the_pot_to_karolina_when_shes_on_it() {
    let c = content();
    let mut w = World::new_story_seeded(&c, Difficulty::Cozy, 5);
    w.sys.tutorial_off = true;
    let mut b = bot::PlayerBot::new(bot::BotConfig {
        seed: 5,
        ..Default::default()
    });
    // a few nights in: a helper is hired
    for _ in 0..5 {
        b.run_night(&mut w, &c);
    }
    b.day(&mut w, &c);
    bot::finish_cutscenes(&mut w, &c);
    let helpers: Vec<u32> = w.staff().iter().filter(|s| !s.owner).map(|s| s.id).collect();
    assert!(!helpers.is_empty(), "a helper");
    for h in helpers {
        let _ = w.apply(
            &c,
            Command::SetDuty {
                staff: h,
                duty: crate::content::handson::Duty::Kitchen,
                on: true,
            },
        );
    }
    w.start_night(&c);
    bot::finish_cutscenes(&mut w, &c);
    // the helper has just walked in (before the kitchen chores of the preparation start),
    // Karolina far from the stove
    let kitchen_in = |w: &World| {
        w.staff().iter().any(|s| {
            !s.owner && s.on_duty() && s.duties.contains(&crate::content::handson::Duty::Kitchen)
        })
    };
    while w.in_prep() && !kitchen_in(&w) {
        w.tick(&c);
        bot::finish_cutscenes(&mut w, &c);
    }
    w.drain_events();
    let on = w
        .staff()
        .iter()
        .any(|s| !s.owner && s.on_duty() && s.duties.contains(&crate::content::handson::Duty::Kitchen));
    assert!(on, "a kitchen helper on shift in the preparation");
    let stove = first_of(&w, &c, "station:stove").unwrap();
    w.hands.player.queue.clear();
    w.hands.player.current = None;
    // somewhere away from the stove first, then the soup
    let door = w.grid().door_tiles()[0];
    w.apply(
        &c,
        Command::PlayerQueue {
            act: PlayerAct::GoTo {
                tile: (door.0, door.1 - 1),
            },
            front: false,
        },
    )
    .unwrap();
    w.apply(
        &c,
        Command::UseEquipment {
            uid: stove,
            action: super::equipment::EquipAction::SoupOn,
        },
    )
    .unwrap();
    for _ in 0..800 {
        w.tick(&c);
        bot::finish_cutscenes(&mut w, &c);
        let done = !w.hands.player.queue.iter().any(|a| matches!(a, PlayerAct::Use { .. }))
            && !w
                .hands
                .player
                .current
                .as_ref()
                .is_some_and(|c| matches!(c.act, PlayerAct::Use { .. }));
        if done {
            break;
        }
    }
    let pot = w.equipment().pot.clone().expect("the soup is on");
    assert!(pot.by_hand, "the helper took the pot she was on her way to");
}
