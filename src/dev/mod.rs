//! The developer / test build (`cargo run --features dev`; `docs/DEV-BUILD.md`): a panel over
//! every screen to jump to any night, open any screen, play any cutscene, switch the language,
//! cheat the running night and switch debug drawing on — plus a "DEV" badge and its own save
//! and settings folder (`platform::DATA_DIR`). None of it exists without the `dev` feature.
//!
//! The panel's texts are English only on purpose: it is a developer tool, never seen by
//! players. Everything player-facing it opens is the game's own (localized) UI.
//!
//! * [`panel`] — the panel UI ([`DevPanel`]) and the [`DevAction`]s it asks for;
//! * [`catalog`] — the screen openers (`SHIFTELLA_SCENE` scenes + their options);
//! * [`opts`] — the scene options the openers set for the scene code (`app::scene_env`);
//! * [`jump`] — the bot playing a run up to a night, a few nights per frame;
//! * [`cheats`] — the Cheats section on a running game;
//! * [`overlays`] — nav grid, seat claims, UI regions.
//!
//! The app side (keys, drawing over each screen, applying actions) is `app::dev`.

pub mod catalog;
pub mod cheats;
pub mod jump;
pub mod opts;
pub mod overlays;
pub mod panel;
#[cfg(test)]
mod tests;

pub use cheats::Cheat;
pub use panel::{DevAction, DevPanel};

use crate::content::{Content, CutsceneTrigger};

/// `SHIFTELLA_SCENE` + options that show cutscene `id` over a fresh café at its own night:
/// a talk at its regular's visit, an ending as the ending scenes do, anything else at the
/// night (and time) its trigger names, stopped at its choice when `choice`.
pub fn cutscene_scene(
    content: &Content,
    id: &str,
    choice: bool,
) -> (String, Vec<(String, String)>) {
    let mut opts = vec![("SHIFTELLA_CUTSCENE".to_string(), id.to_string())];
    // the opening plays before Day 1 (over a café it only hands its choices to the sim)
    if id == "opening" {
        return ("opening".into(), Vec::new());
    }
    let Some(def) = content.cutscene(id) else {
        return ("cutscene_choice".into(), opts);
    };
    let hhmm = |m: i32| {
        let t = (22 * 60 + m).rem_euclid(24 * 60);
        format!("{:02}:{:02}", t / 60, t % 60)
    };
    if let CutsceneTrigger::Ending(_) = &def.trigger
        && !choice
        && (id == "ending_light" || id == "ending_last_train")
    {
        return (id.to_string(), Vec::new());
    }
    let (night, minute) = match &def.trigger {
        CutsceneTrigger::Talk { .. } => {
            if !choice {
                return ("talk".into(), opts);
            }
            crate::screens::game::talk_shot::talk_defaults(content, id).unwrap_or((10, 180))
        }
        CutsceneTrigger::NightStart(n) => (*n, -25),
        CutsceneTrigger::NightEnd(n) => (*n, 470),
        CutsceneTrigger::At { night, time } => (*night, minutes_since_open(time)),
        CutsceneTrigger::ArcStage { regular, stage } => {
            let first = content.regular(regular).map_or(1, |r| r.first_night);
            ((first + stage * 3).min(29), 120)
        }
        _ => (12, 240),
    };
    opts.push(("SHIFTELLA_NIGHT".into(), night.clamp(1, 30).to_string()));
    opts.push(("SHIFTELLA_TIME".into(), hhmm(minute)));
    if !choice {
        opts.push(("SHIFTELLA_T".into(), "0.05".into()));
    }
    ("cutscene_choice".into(), opts)
}

/// Minutes since 22:00 of a clock time (21:30–21:59 negative).
fn minutes_since_open(t: &crate::content::ClockTime) -> i32 {
    let since = (t.minute_of_day as i32 - 22 * 60).rem_euclid(24 * 60);
    if since >= 24 * 60 - 30 {
        since - 24 * 60
    } else {
        since
    }
}

/// Builds opener `i` of [`catalog::all`] headless (what the panel's Screens section opens).
pub fn build_opener(b: &mut crate::app::SceneBuilder, i: usize) -> crate::app::Built {
    let o = catalog::all()[i];
    opts::with(o.opts, || b.scene(o.scene))
}

/// Builds a scene with owned options (cutscenes).
pub fn build_with(
    b: &mut crate::app::SceneBuilder,
    scene: &str,
    opts: &[(String, String)],
) -> crate::app::Built {
    let o: Vec<(&str, &str)> = opts.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    opts::with(&o, || b.scene(scene))
}
