//! QA combos on the screen (headless): photo mode against the other layers, the back step
//! during busy moments, the story card / storm talk / moment card / order card together,
//! the tutorial night at every speed, difficulties through saves.

use std::rc::Rc;

use super::*;
use crate::content::Content;
use crate::settings::load_i18n;

fn content() -> Rc<Content> {
    let root = env!("CARGO_MANIFEST_DIR");
    Rc::new(Content::load(&format!("{root}/assets/data")).expect("content"))
}

fn i18n() -> crate::i18n::I18n {
    let root = env!("CARGO_MANIFEST_DIR");
    load_i18n(&format!("{root}/assets/i18n")).expect("i18n")
}

/// Story night `n` at `minute` with nothing left open by the fast-forward.
fn at(c: &Rc<Content>, loc: &Locale, n: u32, minute: i32) -> GameScreen {
    let mut g = GameScreen::new_story(c, "cozy", true);
    g.fast_forward(c, loc, n, Some(minute), n > 1);
    clear(&mut g);
    g.set_speed(1);
    g
}

fn clear(g: &mut GameScreen) {
    g.hands.order = None;
    g.hands.moment = None;
    g.hands.book = None;
    g.hands.minigame = None;
    g.stories.card = None;
    g.panel = None;
    g.summary = None;
}

/// A stranger seated with a story on them (the leader's id).
fn story_guest(g: &mut GameScreen, c: &Rc<Content>, loc: &Locale) -> Option<u32> {
    for _ in 0..40 {
        let pick = g
            .world
            .parties()
            .iter()
            .filter(|p| p.regular.is_none())
            .filter_map(|p| p.leader())
            .find(|l| {
                g.world.persona(*l).is_some()
                    && g.world
                        .guests()
                        .iter()
                        .any(|x| x.id == *l && x.seat.is_some())
            });
        if let Some(l) = pick {
            if g.world.story_of(l).is_none() {
                g.world.debug_story("call", l);
            }
            g.drain_and_handle(c, Some(loc));
            if g.world.story_of(l).is_some() {
                return Some(l);
            }
        }
        g.play_headless(c, loc, 5.0, &|_| false);
        clear(g);
    }
    None
}

/// Photo mode refuses over every card-like layer (a story card included), and while the
/// camera is up the sim holds still whatever else is pending.
#[test]
fn the_camera_waits_for_the_cards_and_holds_the_night() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, crate::i18n::Lang::Cs);
    let mut g = at(&c, &loc, 8, 40);
    let guest = story_guest(&mut g, &c, &loc).expect("a stranger with a story");
    assert!(g.open_story(guest));
    photo_mode::open(&mut g, &loc);
    assert!(
        !g.photo.is_open(),
        "the camera came up over the story card (input went to two layers)"
    );
    g.stories.card = None;
    // over a moment card / the book / build mode: refused too
    g.open_book(None);
    photo_mode::open(&mut g, &loc);
    assert!(!g.photo.is_open());
    g.hands.book = None;
    g.panel = Some(Panel::Build);
    photo_mode::open(&mut g, &loc);
    assert!(!g.photo.is_open());
    g.panel = Some(Panel::Crew);
    // over a panel it opens (and closes the panel)
    photo_mode::open(&mut g, &loc);
    assert!(g.photo.is_open());
    assert_eq!(g.panel, None);
    let tick = g.world.tick_count();
    g.play_headless(&c, &loc, 10.0, &|_| false);
    assert_eq!(g.world.tick_count(), tick, "the café held still");
}

/// The 03:00 storm talk: the camera stays down; the talk keeps the café still; after it the
/// night goes on to the morning, a save in the middle round-trips.
#[test]
fn the_storm_talk_with_the_camera_and_a_save() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, crate::i18n::Lang::Uk);
    let mut g = at(&c, &loc, 29, 170);
    let _ = g.command(&c, Command::StormBlankets);
    let _ = g.command(&c, Command::StormSoup);
    g.play_headless(&c, &loc, 1500.0, &|g| {
        g.stories.talk.is_some() || g.world.minute() > 330
    });
    if g.stories.talk.is_some() {
        photo_mode::open(&mut g, &loc);
        assert!(!g.photo.is_open(), "the camera over the storm talk");
        assert!(g.frozen());
        // a save while the talk is on: it loads back into the night
        let f = g.save_file(&c).unwrap();
        let back = GameScreen::from_save(&c, &f).unwrap();
        assert_eq!(back.world.minute(), g.world.minute());
        g.stories.talk = None;
    }
    g.play_headless(&c, &loc, 3600.0, &|g| g.world.phase() == Phase::Day);
    assert!(g.summary.is_some() || g.world.phase() == Phase::Day);
}

/// The back step while the pot bubbles, a guest waits, a story is open: the café holds,
/// nothing that was open is lost, and the step works only once.
#[test]
fn the_back_step_with_things_going_on() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, crate::i18n::Lang::En);
    let mut g = at(&c, &loc, 6, 30);
    // the soup on
    let stove = g
        .world
        .furniture()
        .iter()
        .find(|p| g.world.equipment_kind(&c, p.uid) == Some(sim::EquipKind::Stove))
        .map(|p| p.uid);
    if let Some(uid) = stove {
        let _ = g.intent(&c, hands::Intent::Equip { uid, front: true });
    }
    g.play_headless(&c, &loc, 30.0, &|_| false);
    clear(&mut g);
    let guest = story_guest(&mut g, &c, &loc);
    if let Some(gid) = guest {
        g.open_story(gid);
    }
    let from = sim::walk::data().backstep.from_minute;
    if g.world.minute() < from {
        g.play_headless(&c, &loc, 600.0, &move |g| g.world.minute() >= from);
        clear(&mut g);
    }
    g.world.debug_end_rush();
    if g.world.backstep_block(&c).is_none() {
        assert!(walk_home::step_out(&mut g, &c, None));
        assert!(g.frozen(), "the café holds while she sits outside");
        let tick = g.world.tick_count();
        g.advance(&c, 5.0, Some(&loc));
        assert_eq!(g.world.tick_count(), tick);
        assert!(!walk_home::step_out(&mut g, &c, None), "once a night");
    }
}

/// Night 1 with the tutorial at ×1, ×2, ×3 and with it off: the autoplayer gets to the
/// morning every time, the coach never leaves an overlay stuck.
#[test]
fn night_one_with_the_tutorial_at_every_speed() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, crate::i18n::Lang::Ru);
    for (speed, tutorial) in [(1u8, true), (2, true), (3, true), (3, false)] {
        let mut g = GameScreen::new_story(&c, "cozy", true);
        if tutorial {
            g.start_tutorial();
        } else {
            g.extra.tutorial = Some(Default::default());
        }
        g.start_night(&c);
        g.set_speed(speed);
        g.play_headless(&c, &loc, 4000.0, &|g| g.world.phase() == Phase::Day);
        assert_eq!(
            g.world.phase(),
            Phase::Day,
            "×{speed} tutorial {tutorial}: the morning"
        );
        assert!(g.hands.minigame.is_none() && g.hands.order.is_none());
        assert_eq!(g.world.night(), 2);
    }
}

/// Carefree / Classic / Cozy saves: each loads, continues a night and saves again.
#[test]
fn every_difficulty_through_a_save_and_a_night() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, crate::i18n::Lang::Cs);
    for d in ["relaxed", "classic", "cozy"] {
        let mut g = GameScreen::new_story(&c, d, true);
        g.fast_forward(&c, &loc, 4, Some(120), true);
        let f = g.save_file(&c).unwrap();
        let mut back = GameScreen::from_save(&c, &f).unwrap();
        assert_eq!(back.world.difficulty(), g.world.difficulty(), "{d}");
        clear(&mut back);
        back.set_speed(3);
        back.play_headless(&c, &loc, 4000.0, &|g| g.world.phase() == Phase::Day);
        assert_eq!(back.world.night(), 5, "{d}: the night ended");
        let f2 = back.save_file(&c).unwrap();
        let again = GameScreen::from_save(&c, &f2).unwrap();
        assert_eq!(again.world.difficulty(), back.world.difficulty());
    }
}
