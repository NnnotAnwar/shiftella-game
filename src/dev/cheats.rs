//! The "Cheats" section's actions on a running game (headless: tests apply every one).

use std::rc::Rc;

use crate::content::{Content, Weather};
use crate::screens::game::GameScreen;
use crate::settings::Locale;
use crate::sim::{self, Command, Phase};

/// One cheat.
#[derive(Debug, Clone, PartialEq)]
pub enum Cheat {
    Money(i64),
    Reputation(f32),
    /// Jump the clock to this many minutes since 22:00 (negative: the preparation half hour).
    Time(i32),
    /// Run the night to its morning summary.
    Morning,
    /// 0 = paused, 1–3.
    Speed(u8),
    Weather(Weather),
    Event(String),
    Spawn(String),
    Arc(String, i32),
    Trust(String, f32),
    Flag(String, bool),
    Var(String, String),
    Counter(String, i32),
    Notes,
    Recipes,
    Furniture,
    Themes,
    Dirt(f32),
    Fridge(bool),
    Hire,
    Fire,
}

/// The clock times the panel offers (label, minutes since 22:00).
pub const TIMES: &[(&str, i32)] = &[
    ("21:30", -30),
    ("22:00", 0),
    ("00:30", 150),
    ("03:30", 330),
    ("05:15", 435),
    ("06:00", 480),
];

/// Applies `c`; the line for the panel's status bar.
pub fn apply(g: &mut GameScreen, content: &Rc<Content>, loc: &Locale, c: &Cheat) -> String {
    let w = &mut g.world;
    let msg = match c {
        Cheat::Money(d) => {
            w.dev_add_money(*d);
            format!("money {} Kč", w.money())
        }
        Cheat::Reputation(d) => {
            w.dev_add_reputation(*d);
            format!("reputation {:.0}", w.reputation())
        }
        Cheat::Time(m) => return jump_time(g, content, loc, *m),
        Cheat::Morning => {
            if g.world.phase() != Phase::Night {
                return "not a night".into();
            }
            g.run_to_morning(content, loc);
            return "morning".into();
        }
        Cheat::Speed(s) => {
            g.set_speed(*s);
            return if *s == 0 {
                "paused".into()
            } else {
                format!("speed ×{s}")
            };
        }
        Cheat::Weather(x) => {
            w.dev_set_weather(*x);
            format!("weather {x:?}")
        }
        Cheat::Event(id) => {
            if content.events.iter().all(|e| &e.id != id) {
                return format!("no event `{id}`");
            }
            w.debug_fire_event(content, id);
            g.drain_and_handle(content, Some(loc));
            return format!("event {id}");
        }
        Cheat::Spawn(id) => {
            let ok = w.dev_spawn_regular(content, id);
            g.drain_and_handle(content, Some(loc));
            return if ok {
                format!("{id} walks in")
            } else {
                format!("{id} can't come now (day, unknown or inside)")
            };
        }
        Cheat::Arc(id, d) => format!("arc.{id} = {}", w.dev_arc_stage(id, *d)),
        Cheat::Trust(id, d) => format!("trust {id} = {:.2}", w.dev_trust(id, *d)),
        Cheat::Flag(f, on) => {
            if f.trim().is_empty() {
                return "flag name?".into();
            }
            w.dev_flag(f.trim(), *on);
            format!("flag {} {}", f.trim(), if *on { "set" } else { "cleared" })
        }
        Cheat::Var(k, v) => {
            if k.trim().is_empty() {
                return "var name?".into();
            }
            w.dev_set_var(k.trim(), v.trim());
            format!("var {} = {}", k.trim(), v.trim())
        }
        Cheat::Counter(k, v) => {
            if k.trim().is_empty() {
                return "counter name?".into();
            }
            w.debug_set_counter(k.trim(), *v);
            format!("counter {} = {v}", k.trim())
        }
        Cheat::Notes => format!("{} notes found", w.dev_find_notes(content)),
        Cheat::Recipes => format!("{} recipes learned", w.dev_unlock_recipes(content)),
        Cheat::Furniture => format!("{} pieces unlocked", w.dev_unlock_furniture(content)),
        Cheat::Themes => format!("{} themes owned", w.dev_unlock_themes(content)),
        Cheat::Dirt(d) => {
            w.dev_set_dirt(*d);
            format!("dirt {d:.1}")
        }
        Cheat::Fridge(full) => {
            w.dev_fridge(content, *full);
            let f = w.fridge(content);
            format!("fridge {} / {} Kč", f.fresh, f.capacity)
        }
        Cheat::Hire => {
            let Some(id) = w.candidates().first().map(|c| c.id) else {
                return "no candidates".into();
            };
            return match g.command(content, Command::Hire { candidate: id }) {
                Ok(()) => "hired".into(),
                Err(e) => format!("can't hire: {e}"),
            };
        }
        Cheat::Fire => {
            let Some(id) = w.staff().iter().rev().find(|s| !s.owner).map(|s| s.id) else {
                return "no helpers".into();
            };
            return match g.command(content, Command::Fire { staff: id }) {
                Ok(()) => "fired".into(),
                Err(e) => format!("can't fire: {e}"),
            };
        }
    };
    g.drain_and_handle(content, Some(loc));
    msg
}

/// The clock to `minute` since 22:00 the way the screenshot scenes get there (the bot plays
/// the minutes between; from the day the night starts first).
fn jump_time(g: &mut GameScreen, content: &Rc<Content>, loc: &Locale, minute: i32) -> String {
    if g.world.is_closed() || g.world.ending().is_some() {
        return "the run is over".into();
    }
    if g.world.phase() == Phase::Night && g.world.minutes_since_open() > minute {
        return "can't go back in time (jump from the day)".into();
    }
    if g.world.phase() == Phase::Day {
        let _ = g.command(content, Command::StartNight);
    }
    let night = g.world.night();
    g.fast_forward(content, loc, night, Some(minute), false);
    format!("{} · night {night}", g.world.clock_label())
}

/// The weathers the panel offers.
pub const WEATHERS: [Weather; 6] = Weather::ALL;

/// Short text of the game state for the panel's header.
pub fn summary(g: &GameScreen) -> String {
    let w = &g.world;
    let when = if w.phase() == Phase::Day {
        "day".to_string()
    } else {
        w.clock_label()
    };
    let mode = match w.mode() {
        sim::Mode::Story => w.difficulty().id(),
        sim::Mode::Sandbox => "sandbox",
    };
    format!(
        "night {} · {when} · {mode} · {} Kč · rep {:.0} · {:?}",
        w.night(),
        w.money(),
        w.reputation(),
        w.weather()
    )
}
