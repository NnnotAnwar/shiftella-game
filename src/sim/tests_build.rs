//! Package C (design/v0.2.md items 4, 6, 7, 9, 10): build requests, corners, grandma's attic,
//! the back room and the before / after polaroid — and that none of it is needed for the
//! story (a bot that never builds still ends in the light).

use super::*;
use crate::content::Content;
use crate::content::handson::HighlightKind;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

fn content() -> Content {
    Content::load(DATA).unwrap_or_else(|e| panic!("{e}"))
}

/// The day of story night `n` (cutscenes done, plenty of money for building).
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

fn ticks(w: &mut World, c: &Content, n: u32) -> Vec<SimEvent> {
    let mut out = Vec::new();
    for _ in 0..n {
        w.tick(c);
        bot::finish_cutscenes(w, c);
        out.extend(w.drain_events());
    }
    out
}

fn to_day(w: &mut World, c: &Content) -> Vec<SimEvent> {
    let mut out = Vec::new();
    while w.phase() == Phase::Night {
        w.tick(c);
        bot::finish_cutscenes(w, c);
        out.extend(w.drain_events());
    }
    bot::finish_cutscenes(w, c);
    out.extend(w.drain_events());
    out
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

fn trust(w: &World, r: &str) -> f32 {
    w.story().trust.get(r).copied().unwrap_or(0.0)
}

// ---------------------------------------------------------------------------------------------
// Requests (item 4)
// ---------------------------------------------------------------------------------------------

#[test]
fn a_regular_asks_once_sees_it_built_and_thanks_once() {
    let c = content();
    let mut w = day(&c, 7);
    assert!(
        w.build_wishes(&c).iter().all(|b| b.who != "filip"),
        "Filip's request waits until he has been here"
    );
    open(&mut w, &c);
    guests::spawn_regular(&mut w, &c, "filip", None);
    let ev = ticks(&mut w, &c, 8 * TICKS_PER_MINUTE);
    let wish = w
        .build_wishes(&c)
        .into_iter()
        .find(|b| b.who == "filip")
        .expect("Filip's socket request is open");
    assert_eq!(wish.tag.as_deref(), Some("power_socket"));
    assert_eq!(wish.reason_key, "build_wish.filip_socket.reason");
    let asked = ev
        .iter()
        .filter(
            |e| matches!(e, SimEvent::Speech { key, .. } if key == "build_wish.filip_socket.ask"),
        )
        .count();
    assert_eq!(asked, 1, "he says it once");
    let req = w
        .build_requests(&c)
        .into_iter()
        .find(|r| r.id == "filip_socket")
        .unwrap();
    assert_eq!(req.who, BuildWho::Regular("filip".into()));
    assert_eq!(req.status, BuildRequestStatus::Waiting);
    assert!(req.arc);
    // building it: "built", then he reacts while he's here
    let before = trust(&w, "filip");
    place(&mut w, &c, "table_socket", 8, 12, 0);
    let st = |w: &World| {
        w.build_requests(&c)
            .into_iter()
            .find(|r| r.id == "filip_socket")
            .unwrap()
            .status
    };
    assert_eq!(st(&w), BuildRequestStatus::Built);
    assert!(
        w.build_wishes(&c).iter().all(|b| b.who != "filip"),
        "no badge once it stands"
    );
    let ev = ticks(&mut w, &c, 3 * TICKS_PER_MINUTE);
    let thanked = ev
        .iter()
        .filter(|e| matches!(e, SimEvent::BuildWishFulfilled { id, .. } if id == "filip_socket"))
        .count();
    assert_eq!(thanked, 1);
    assert!(ev.iter().any(
        |e| matches!(e, SimEvent::Speech { key, .. } if key == "build_wish.filip_socket.thanks")
    ));
    assert!(trust(&w, "filip") >= before + 3.0 - 0.01, "the reward ran");
    assert_eq!(st(&w), BuildRequestStatus::Thanked(7));
    let again = ticks(&mut w, &c, 20 * TICKS_PER_MINUTE);
    assert!(
        !again
            .iter()
            .any(|e| matches!(e, SimEvent::BuildWishFulfilled { .. })),
        "only once"
    );
}

#[test]
fn a_request_already_standing_never_shows() {
    let c = content();
    // the sofa stands before the nurses' request would show (night 6)
    let mut w = day(&c, 5);
    place(&mut w, &c, "sofa", 6, 12, 0);
    w.night = 6;
    w.begin_day(&c);
    assert!(
        w.build_requests(&c).iter().all(|r| r.id != "nurses_sofa"),
        "the nurses don't ask for a sofa that's there"
    );
    // guests' requests show without a visit; staff ones need a helper
    let mut w = day(&c, 9);
    assert!(
        w.build_requests(&c)
            .iter()
            .any(|r| r.id == "insomniacs_rug")
    );
    assert!(w.build_requests(&c).iter().all(|r| r.id != "crew_coats"));
    w.night = 10;
    w.begin_day(&c);
    assert!(
        w.build_requests(&c)
            .iter()
            .any(|r| r.id == "insomniacs_rug")
    );
}

#[test]
fn a_helper_asks_for_a_coat_rack_and_cheers_up() {
    let c = content();
    let mut w = day(&c, 4);
    let cand = w.candidates()[0].id;
    w.apply(&c, Command::Hire { candidate: cand }).unwrap();
    let helper = w.staff().iter().find(|s| !s.owner).unwrap().id;
    let req = w
        .build_requests(&c)
        .into_iter()
        .find(|r| r.id == "crew_coats")
        .expect("the crew's request opens when someone is hired");
    assert_eq!(req.who, BuildWho::Staff(helper));
    assert_eq!(
        w.build_wishes(&c)
            .iter()
            .find(|b| b.tag.as_deref() == Some("coats"))
            .map(|b| b.who.clone()),
        Some(helper.to_string())
    );
    place(&mut w, &c, "coat_rack", 6, 12, 0);
    open(&mut w, &c);
    let i = w.staff.iter().position(|s| s.id == helper).unwrap();
    w.staff[i].status = StaffStatus::OnDuty;
    let mood = w.staff[i].mood;
    super::build_wishes::on_minute(&mut w, &c);
    let ev = w.drain_events();
    assert!(ev.iter().any(
        |e| matches!(e, SimEvent::BuildWishFulfilled { id, who } if id == "crew_coats" && *who == helper.to_string())
    ));
    assert!(w.staff[i].mood > mood, "TargetStaffMood");
}

// ---------------------------------------------------------------------------------------------
// Corners (item 6)
// ---------------------------------------------------------------------------------------------

#[test]
fn grandmas_default_cafe_forms_no_corner() {
    let c = content();
    let w = World::new_story(&c, Difficulty::Cozy);
    assert!(w.corners(&c).is_empty(), "{:?}", w.corners(&c));
    assert_eq!(w.corners_bonus(&c), 0.0);
}

#[test]
fn an_armchair_a_lamp_and_the_bookshelf_make_a_reading_nook() {
    let c = content();
    let mut w = day(&c, 9);
    let base = w.coziness().value;
    let chair = place(&mut w, &c, "armchair", 27, 6, 1);
    let ev = w.drain_events();
    assert!(
        !ev.iter()
            .any(|e| matches!(e, SimEvent::CornerFormed { id, .. } if id == "reading_nook")),
        "no lamp yet"
    );
    let lamp = place(&mut w, &c, "floor_lamp", 27, 4, 0);
    let ev = w.drain_events();
    let formed: Vec<_> = ev
        .iter()
        .filter_map(|e| match e {
            SimEvent::CornerFormed { id, first, .. } => Some((id.clone(), *first)),
            _ => None,
        })
        .collect();
    assert_eq!(formed, vec![("reading_nook".to_string(), true)]);
    // the armchair next to grandma's jukebox made a listening corner too
    assert!(w.corners(&c).iter().any(|k| k.id == "listening_corner"));
    let nook = w
        .corners(&c)
        .into_iter()
        .find(|k| k.id == "reading_nook")
        .unwrap();
    assert_eq!(nook.name_key, "corner.reading_nook.name");
    assert_eq!(nook.members.len(), 3);
    assert!(nook.members.contains(&chair) && nook.members.contains(&lamp));
    assert!(
        nook.tiles.contains(&(28, 5)),
        "grandma's bookshelf is part of it"
    );
    assert_eq!(nook.bonus, 3.0);
    assert!(
        w.coziness().value > base + 5.0 - 0.01,
        "{} → {}",
        base,
        w.coziness().value
    );
    // move the lamp away: gone; back: formed again (not the first any more)
    w.apply(
        &c,
        Command::MoveFurniture {
            uid: lamp,
            x: 12,
            y: 12,
            rot: 0,
        },
    )
    .unwrap();
    assert!(w.corners(&c).iter().all(|k| k.id != "reading_nook"));
    w.drain_events();
    w.undo(&c).unwrap();
    let ev = w.drain_events();
    assert!(ev.iter().any(
        |e| matches!(e, SimEvent::CornerFormed { id, first: false, .. } if id == "reading_nook")
    ));
}

#[test]
fn a_corner_draws_its_guests_and_the_bonus_is_capped() {
    let c = content();
    let mut w = day(&c, 9);
    place(&mut w, &c, "armchair", 27, 6, 1);
    place(&mut w, &c, "floor_lamp", 27, 4, 0);
    let att = super::corners::attraction(&w, &c);
    assert!(att.iter().any(|(a, m)| a == "insomniacs" && *m > 1.0));
    assert!(w.corners_bonus(&c) <= c.build.corners.max_bonus);
}

// ---------------------------------------------------------------------------------------------
// Grandma's attic (item 7)
// ---------------------------------------------------------------------------------------------

#[test]
fn the_attic_opens_on_night_3_restocks_on_saturdays_and_brings_pieces_down() {
    let c = content();
    let w = day(&c, 2);
    assert!(!w.attic(&c).open);
    assert_eq!(w.attic(&c).next_night, 3);
    let mut w = day(&c, 3);
    let a = w.attic(&c);
    assert!(a.open);
    assert_eq!(a.offers.len(), 3);
    assert!(a.offers.iter().all(|o| o.price < o.new_price));
    assert_eq!(a.next_night, 6, "the first Saturday");
    // the same seed offers the same pieces
    assert_eq!(day(&c, 3).attic(&c), a);
    let id = a.offers[0].id.clone();
    let money = w.money();
    w.apply(&c, Command::BuyAtticPiece { id: id.clone() })
        .unwrap();
    assert_eq!(w.money(), money - a.offers[0].price as i64);
    let piece = w.furniture().iter().find(|p| p.def == id).cloned().unwrap();
    assert_eq!(piece.paid, 0, "taken away it goes back upstairs");
    assert!(
        w.drain_events()
            .iter()
            .any(|e| matches!(e, SimEvent::AtticPieceBought { id: x, .. } if *x == id))
    );
    assert_eq!(
        w.apply(&c, Command::BuyAtticPiece { id: id.clone() }),
        Err(CommandError::AlreadyPlaced)
    );
    // upstairs again, then down for free
    w.apply(&c, Command::RemoveFurniture { uid: piece.uid })
        .unwrap();
    assert_eq!(w.money(), money - a.offers[0].price as i64, "no refund");
    assert_eq!(w.attic(&c).upstairs.len(), 1);
    w.apply(&c, Command::BuyAtticPiece { id: id.clone() })
        .unwrap();
    assert_eq!(w.money(), money - a.offers[0].price as i64, "free");
    // nights: day only; a piece not on offer is locked
    let other = c
        .build
        .attic
        .pieces
        .iter()
        .find(|p| !a.offers.iter().any(|o| o.id == p.id))
        .unwrap()
        .id
        .clone();
    assert_eq!(
        w.apply(&c, Command::BuyAtticPiece { id: other }),
        Err(CommandError::Locked)
    );
    // Saturday: a new offer without the piece already bought
    let mut w6 = w.clone();
    w6.night = 6;
    w6.begin_day(&c);
    let a6 = w6.attic(&c);
    assert_eq!(w6.build.attic_restocked, 6);
    assert!(a6.offers.iter().all(|o| o.id != id));
    // a weekday that isn't Saturday keeps the offer
    let mut w4 = w.clone();
    w4.night = 4;
    w4.begin_day(&c);
    assert_eq!(w4.build.attic_restocked, 3);
    open(&mut w6, &c);
    let id6 = a6.offers[0].id.clone();
    assert_eq!(
        w6.apply(&c, Command::BuyAtticPiece { id: id6 }),
        Err(CommandError::WrongPhase)
    );
}

#[test]
fn attic_pieces_stay_out_of_the_build_catalogue() {
    let c = content();
    let w = day(&c, 20);
    let buildable = w.buildable(&c);
    for p in &c.build.attic.pieces {
        assert!(!buildable.contains(&p.id), "{}", p.id);
    }
}

// ---------------------------------------------------------------------------------------------
// The back room (item 9)
// ---------------------------------------------------------------------------------------------

#[test]
fn the_back_room_opens_from_night_15_and_keeps_the_old_cafe_in_place() {
    let c = content();
    let mut w = day(&c, 14);
    assert!(!w.back_room(&c).available);
    assert_eq!(
        w.apply(&c, Command::OpenBackRoom),
        Err(CommandError::NotUnlocked)
    );
    let mut w = day(&c, 15);
    let old = w.grid().clone();
    let seats_before = super::guests::seats(&w, &c).len();
    let coz_before = w.coziness().value;
    let money = w.money();
    w.apply(&c, Command::OpenBackRoom).unwrap();
    let price = c.build.attic.back_room.price as i64;
    assert_eq!(w.money(), money - price);
    assert_eq!(w.grid().width, old.width + BACK_COLS);
    assert_eq!(w.grid().height, old.height);
    let room = w.build.back_room.clone().unwrap();
    for y in 0..old.height {
        for x in 0..old.width {
            if room.door.contains(&(x, y)) {
                continue;
            }
            assert_eq!(w.grid().tile(x, y), old.tile(x, y), "tile {x},{y}");
            assert_eq!(
                w.grid().object_at(x, y),
                old.object_at(x, y),
                "object {x},{y}"
            );
        }
    }
    // the street continues outside
    assert!(
        w.grid()
            .tile(w.grid().width - 1, w.grid().height - 1)
            .is_some_and(|t| t.kind == TileKind::Street)
    );
    // grandma's tables: 4 more seats, every one reachable by a guest from the door
    let seats = super::guests::seats(&w, &c);
    assert_eq!(seats.len(), seats_before + 4);
    let from = w.grid().entrance();
    for s in seats.iter().filter(|s| room.contains(s.tile)) {
        assert!(
            grid::find_path(w.grid(), from, s.tile, true).is_some(),
            "seat {:?}",
            s.tile
        );
    }
    // extra floor doesn't thin the hall out
    assert!(
        w.coziness().value >= coz_before - 0.5,
        "{coz_before} → {}",
        w.coziness().value
    );
    assert_eq!(
        w.apply(&c, Command::OpenBackRoom),
        Err(CommandError::AlreadyPlaced)
    );
    // saves keep it; the night runs with guests in the bigger café
    let text = w.save_to_string().unwrap();
    let mut back = World::load_from_str(&text).unwrap();
    assert_eq!(back.grid().width, w.grid().width);
    assert_eq!(back.build.back_room, w.build.back_room);
    let mut bt = bot::PlayerBot::new(bot::BotConfig::default());
    bt.run_night(&mut back, &c);
    assert_eq!(back.phase(), Phase::Day);
    assert_eq!(back.night(), 16);
}

#[test]
fn a_cozy_player_affords_the_back_room_by_night_20() {
    let c = content();
    let mut w = World::new_story_seeded(&c, Difficulty::Cozy, 4242);
    let mut b = bot::PlayerBot::new(bot::BotConfig {
        seed: 4242,
        build: false,
        ..Default::default()
    });
    while w.night() < 20 {
        b.run_night(&mut w, &c);
    }
    assert!(w.back_room(&c).available);
    assert!(
        w.money() >= c.build.attic.back_room.price as i64,
        "money {} on night 20",
        w.money()
    );
}

// ---------------------------------------------------------------------------------------------
// Before / after (item 10)
// ---------------------------------------------------------------------------------------------

#[test]
fn a_new_corner_brings_a_before_after_polaroid_next_morning() {
    let c = content();
    let mut w = day(&c, 8);
    open(&mut w, &c);
    to_day(&mut w, &c);
    let _ = w.apply(&c, Command::PickPolaroid { index: 0 });
    // night 9: nothing changed → no before / after
    open(&mut w, &c);
    assert!(w.build.before_after.is_none());
    to_day(&mut w, &c);
    let (_, list) = w.pending_polaroids().unwrap();
    assert!(list.iter().all(|h| h.kind != HighlightKind::BeforeAfter));
    let _ = w.apply(&c, Command::PickPolaroid { index: 0 });
    // day 10: a reading nook → night 10's morning offers before / after
    place(&mut w, &c, "armchair", 27, 6, 1);
    place(&mut w, &c, "floor_lamp", 27, 4, 0);
    open(&mut w, &c);
    let plan = w
        .build
        .before_after
        .clone()
        .expect("planned at night start");
    assert_eq!(plan.what, "@corner.reading_nook.what");
    to_day(&mut w, &c);
    let (_, list) = w.pending_polaroids().unwrap();
    let h = list
        .iter()
        .find(|h| h.kind == HighlightKind::BeforeAfter)
        .expect("the morning includes the before / after");
    assert_eq!(h.caption_key, "journal.caption.before_after");
    let before = h.before.as_ref().unwrap();
    assert_eq!(before.origin, h.snap.origin);
    assert_eq!(before.size, h.snap.size);
    let has = |s: &SceneSnapData, def: &str| s.props.iter().any(|p| p.def == def);
    assert!(has(&h.snap, "armchair") && !has(before, "armchair"));
    assert!(list.len() <= c.handson.journal.keep_per_night as usize);
    // keeping it puts both photos in the journal (and in the save)
    let i = list
        .iter()
        .position(|h| h.kind == HighlightKind::BeforeAfter)
        .unwrap();
    w.apply(&c, Command::PickPolaroid { index: i }).unwrap();
    let text = w.save_to_string().unwrap();
    let back = World::load_from_str(&text).unwrap();
    assert!(back.journal().last().unwrap().highlight.before.is_some());
}

// ---------------------------------------------------------------------------------------------
// Saves & the story
// ---------------------------------------------------------------------------------------------

#[test]
fn old_saves_load_without_package_c_state() {
    let c = content();
    for text in [
        include_str!("testdata/save_v1_story_day3.ron"),
        include_str!("testdata/save_v1_sandbox_night.ron"),
    ] {
        let mut w = World::load_from_str(text).unwrap();
        w.rebuild_caches(&c);
        assert_eq!(w.build, BuildState::default());
        assert!(w.back_room(&c).tiles.is_empty());
        let _ = w.corners(&c);
        let _ = w.build_requests(&c);
        for _ in 0..50 {
            w.tick(&c);
        }
    }
}

/// Plays the whole story with the attentive bot that never builds anything.
#[test]
fn a_bot_that_never_builds_still_reaches_the_light_ending() {
    let c = content();
    let start = World::new_story_seeded(&c, Difficulty::Cozy, 4242);
    let defs = |w: &World| {
        let mut v: Vec<String> = w.furniture().iter().map(|p| p.def.clone()).collect();
        v.sort();
        v
    };
    let mut w = start.clone();
    let mut b = bot::PlayerBot::new(bot::BotConfig {
        seed: 4242,
        build: false,
        ..Default::default()
    });
    for _ in 1..=30 {
        if w.is_closed() {
            break;
        }
        if w.phase() == Phase::Day {
            b.day(&mut w, &c);
            w.start_night(&c);
        }
        let mut guard = 0;
        while w.phase() == Phase::Night && !w.is_closed() && guard < 100_000 {
            if let Some(id) = w.playing_cutscene().map(str::to_string) {
                let choices = bot::attentive_choices(&w, &c, &id);
                w.apply(&c, Command::CutsceneFinished { id, choices })
                    .unwrap();
            }
            b.step(&mut w, &c);
            w.tick(&c);
            w.drain_events();
            guard += 1;
        }
        while let Some(id) = w.playing_cutscene().map(str::to_string) {
            let choices = bot::attentive_choices(&w, &c, &id);
            w.apply(&c, Command::CutsceneFinished { id, choices })
                .unwrap();
        }
    }
    assert_eq!(w.ending(), Some("light"));
    assert_eq!(defs(&w), defs(&start), "nothing was built");
    assert!(w.build.back_room.is_none() && w.build.attic_owned.is_empty());
    // the requests told the player about Filip's socket all along
    assert!(
        w.build_requests(&c)
            .iter()
            .any(|r| r.id == "filip_socket" && r.status == BuildRequestStatus::Waiting)
    );
}
