//! "Pause on important events": the night pauses itself once for a bad event, keeps a
//! cooldown, waits while something holds the night, and never pauses when it is off, under
//! automation or while the bot plays; the sandbox option comes back from a save.

use std::rc::Rc;

use super::auto_pause::{COOLDOWN_MINUTES, Why};
use super::*;
use crate::content::Content;
use crate::i18n::Lang;
use crate::settings::load_i18n;

fn content() -> Rc<Content> {
    let root = env!("CARGO_MANIFEST_DIR");
    Rc::new(Content::load(&format!("{root}/assets/data")).expect("content"))
}

fn i18n() -> crate::i18n::I18n {
    let root = env!("CARGO_MANIFEST_DIR");
    load_i18n(&format!("{root}/assets/i18n")).expect("i18n")
}

fn setup(pause: bool) -> sim::SandboxSetup {
    let mut s = sim::SandboxSetup {
        seed: 21,
        ..Default::default()
    };
    s.options.insert("pause_on_events".into(), pause);
    s
}

/// A sandbox night after 22:00, running at ×1, nothing on screen.
fn night(c: &Rc<Content>, option: bool) -> GameScreen {
    let mut g = GameScreen::new_sandbox(c, setup(option));
    g.automation = false;
    g.start_night(c);
    let _ = g.command(c, Command::OpenDoors);
    let _ = g.world.drain_events();
    g.deferred.clear();
    g.set_speed(1);
    g
}

/// Runs `minutes` of the night quietly (events dropped: nothing else may pause).
fn skip_minutes(g: &mut GameScreen, c: &Rc<Content>, minutes: u32) {
    for _ in 0..minutes * 10 {
        g.world.tick(c);
        let _ = g.world.drain_events();
    }
}

fn fire(g: &mut GameScreen, c: &Rc<Content>, loc: &Locale, id: &str) {
    g.world.debug_fire_event(c, id);
    g.drain_and_handle(c, Some(loc));
}

#[test]
fn a_bad_event_pauses_once_and_play_goes_on() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, Lang::En);
    let mut g = night(&c, false);
    g.auto.setting = true;
    g.set_speed(2);
    fire(&mut g, &c, &loc, "storm");
    assert_eq!(g.speed, 0, "the storm paused the night");
    assert!(g.frozen());
    assert_eq!(g.auto.shown, Some(Why::Event("storm".into())));
    assert_eq!(g.auto.count, 1);
    // the chip's text: "Paused: Thunderstorm"
    let text = loc.trf(
        "ui.autopause.chip",
        &[("reason", &Why::Event("storm".into()).text(&g, &loc))],
    );
    assert_eq!(text, "Paused: Thunderstorm");
    // Space / Continue: the same speed as before, the chip goes
    g.toggle_pause();
    assert_eq!(g.speed, 2);
    g.drain_and_handle(&c, Some(&loc));
    assert!(g.auto.shown.is_none());
    assert_eq!(g.auto.count, 1, "paused once");
}

#[test]
fn the_cooldown_holds_for_fifteen_minutes() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, Lang::En);
    let mut g = night(&c, false);
    g.auto.setting = true;
    fire(&mut g, &c, &loc, "power_outage");
    assert_eq!(g.speed, 0);
    g.toggle_pause();
    // another bad thing a few minutes later: no second pause
    skip_minutes(&mut g, &c, 5);
    fire(&mut g, &c, &loc, "fridge_fail");
    assert_eq!(g.speed, 1, "inside the cooldown");
    assert_eq!(g.auto.count, 1);
    // after the cooldown it pauses again
    skip_minutes(&mut g, &c, COOLDOWN_MINUTES as u32);
    fire(&mut g, &c, &loc, "student_party");
    assert_eq!(g.speed, 0);
    assert_eq!(g.auto.count, 2);
    assert_eq!(g.auto.shown, Some(Why::Event("student_party".into())));
}

#[test]
fn off_never_pauses() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, Lang::En);
    let mut g = night(&c, false);
    assert!(!g.pause_on_events(), "off by default");
    for id in ["storm", "power_outage", "fridge_fail", "staff_walkout"] {
        fire(&mut g, &c, &loc, id);
        skip_minutes(&mut g, &c, COOLDOWN_MINUTES as u32 + 1);
    }
    assert_eq!(g.speed, 1);
    assert_eq!(g.auto.count, 0);
}

#[test]
fn small_and_good_events_do_not_pause() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, Lang::En);
    let mut g = night(&c, false);
    g.auto.setting = true;
    // severity-1 bad event without a question, a good one, a neutral one
    for id in ["dine_and_dash", "big_tip", "late_train"] {
        fire(&mut g, &c, &loc, id);
    }
    assert_eq!(g.speed, 1);
    assert_eq!(g.auto.count, 0);
}

#[test]
fn never_under_automation_the_autoplayer_or_the_bot() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, Lang::En);
    // screenshot automation
    let mut g = night(&c, true);
    g.auto.setting = true;
    g.automation = true;
    fire(&mut g, &c, &loc, "storm");
    assert_eq!(g.speed, 1);
    // the bot playing nights (fast_forward / run_to_morning)
    let mut g = GameScreen::new_sandbox(&c, setup(true));
    g.automation = false;
    g.auto.setting = true;
    g.fast_forward(&c, &loc, 2, Some(240), false);
    g.run_to_morning(&c, &loc);
    assert!(g.world.night() >= 2, "the bot played a night");
    assert_eq!(g.auto.count, 0, "the bot's nights never paused");
    assert!(!g.auto.driving);
}

#[test]
fn a_reason_waits_while_the_pause_menu_is_open() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, Lang::En);
    let mut g = night(&c, false);
    g.auto.setting = true;
    g.pause = Some(Default::default());
    fire(&mut g, &c, &loc, "storm");
    assert_eq!(g.auto.count, 0, "the menu holds the night");
    g.pause = None;
    g.drain_and_handle(&c, Some(&loc));
    assert_eq!(g.speed, 0, "paused once the menu closed");
    assert_eq!(g.auto.shown, Some(Why::Event("storm".into())));
}

#[test]
fn the_player_pause_is_left_alone() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, Lang::En);
    let mut g = night(&c, false);
    g.auto.setting = true;
    g.toggle_pause();
    fire(&mut g, &c, &loc, "storm");
    assert!(g.auto.shown.is_none(), "already paused: no chip");
    g.toggle_pause();
    assert_eq!(g.speed, 1);
}

#[test]
fn a_helper_walking_out_and_running_out_of_supplies_pause() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, Lang::En);
    let mut g = night(&c, false);
    g.auto.setting = true;
    g.handle_event(&c, SimEvent::StaffWalkedOut { staff: 1 }, Some(&loc));
    g.drain_and_handle(&c, Some(&loc));
    assert_eq!(g.auto.shown, Some(Why::StaffQuit(1)));
    g.toggle_pause();
    skip_minutes(&mut g, &c, COOLDOWN_MINUTES as u32);
    g.world.econ.supplies = 0;
    g.drain_and_handle(&c, Some(&loc));
    assert_eq!(g.speed, 0);
    assert_eq!(g.auto.shown, Some(Why::SuppliesOut));
    // still out a while later: no second pause for the same thing
    g.toggle_pause();
    skip_minutes(&mut g, &c, COOLDOWN_MINUTES as u32);
    g.world.econ.supplies = 0;
    g.drain_and_handle(&c, Some(&loc));
    assert_eq!(g.speed, 1);
}

#[test]
fn the_sandbox_option_survives_save_and_load() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, Lang::En);
    // the menu form → the sim setup keeps the box
    let form = crate::screens::menu::SandboxSetup {
        cafe_name: "Test".into(),
        seed_text: "1".into(),
        seed: 1,
        difficulty: "cozy".into(),
        character: "calm".into(),
        start_money: 25_000,
        rain_every_night: false,
        pause_on_events: true,
        allow_debt: true,
        challenge: false,
    };
    let s = wv::sandbox_setup(&form);
    assert_eq!(s.options.get("pause_on_events"), Some(&true));
    let mut g = GameScreen::new_sandbox(&c, s);
    g.automation = false;
    assert!(g.pause_on_events(), "the box alone turns it on");
    let file = g.save_file(&c).unwrap();
    let mut g2 = GameScreen::from_save(&c, &file).unwrap();
    g2.automation = false;
    assert!(!g2.auto.setting);
    assert!(g2.pause_on_events(), "kept in the save");
    g2.start_night(&c);
    let _ = g2.command(&c, Command::OpenDoors);
    let _ = g2.world.drain_events();
    g2.deferred.clear();
    fire(&mut g2, &c, &loc, "storm");
    assert_eq!(g2.speed, 0, "the loaded sandbox pauses too");
}

#[test]
fn every_auto_pause_key_is_in_four_languages() {
    let i = i18n();
    let keys = [
        "ui.autopause.chip",
        "ui.autopause.continue",
        "ui.autopause.regular",
        "ui.autopause.story",
        "ui.autopause.patience",
        "ui.autopause.staff_sick",
        "ui.autopause.staff_quit",
        "ui.autopause.message",
        "ui.autopause.low_stock",
        "ui.autopause.supplies_out",
        "ui.settings.pause_on_events",
        "ui.sandbox.pause_on_events",
    ];
    for lang in [Lang::Cs, Lang::En, Lang::Uk, Lang::Ru] {
        for k in keys {
            assert!(i.has(lang, k), "{k} missing in {lang:?}");
        }
    }
}
