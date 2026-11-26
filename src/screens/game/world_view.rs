//! The one place where the game screens meet the simulation.
//!
//! Every screen imports sim types from here (`world_view::sim::…`), so switching the backend
//! is a one-line change: Phase A used a copy of the sim agent's WIP (`crate::simwip`);
//! Phase B points this at the real `crate::sim`.
//!
//! Also: small, pure conversions from sim view types to render types, and the text helpers
//! that turn sim ids into player-visible strings (always through i18n).

use macroquad::prelude::*;

pub use crate::sim;

use crate::content::{Content, Job, Weather};
use crate::i18n::format_clock;
use crate::map::TILE;
use crate::render::{self, Emote, Facing, PersonLook, PersonPose, Pose};
use crate::settings::Locale;
use sim::{AgentView, LookRef, Staff, World};

pub fn facing(f: sim::Facing) -> Facing {
    match f {
        sim::Facing::Down => Facing::Down,
        sim::Facing::Up => Facing::Up,
        sim::Facing::Left => Facing::Left,
        sim::Facing::Right => Facing::Right,
    }
}

pub fn pose(p: sim::Pose) -> Pose {
    match p {
        sim::Pose::Idle => Pose::Idle,
        sim::Pose::Walk => Pose::Walk,
        sim::Pose::Sit => Pose::Sit,
        sim::Pose::Carry => Pose::Carry,
        sim::Pose::Work => Pose::Work,
    }
}

pub fn emote(e: sim::SimEmote) -> Emote {
    match e {
        sim::SimEmote::Coffee => Emote::Coffee,
        sim::SimEmote::Heart => Emote::Heart,
        sim::SimEmote::Zzz => Emote::Zzz,
        sim::SimEmote::Angry => Emote::Angry,
        sim::SimEmote::Note => Emote::Note,
        sim::SimEmote::Music => Emote::Music,
        sim::SimEmote::Exclamation => Emote::Exclamation,
        sim::SimEmote::Money => Emote::Money,
        // hands-on (design §1.7): a perfect serve, an okay serve / a greeting
        sim::SimEmote::Sparkle => Emote::Sparkle,
        sim::SimEmote::Smile => Emote::Smile,
    }
}

/// The render look for a sim person.
pub fn look_of(l: &LookRef) -> PersonLook {
    match &l.preset {
        Some(p) => PersonLook::preset(p),
        None => PersonLook::random(l.seed, l.staff),
    }
}

/// Rain strength for the renderer.
pub fn rain_of(w: Weather) -> render::Weather {
    let (intensity, wind) = match w {
        Weather::Clear | Weather::Fog | Weather::Snow => (0.0, 0.1),
        Weather::Rain => (1.0, 0.18),
        Weather::Downpour => (1.7, 0.26),
        Weather::Storm => (2.0, 0.42),
    };
    render::Weather { intensity, wind }
}

pub fn weather_key(w: Weather) -> &'static str {
    match w {
        Weather::Clear => "ui.weather.clear",
        Weather::Rain => "ui.weather.rain",
        Weather::Downpour => "ui.weather.downpour",
        Weather::Fog => "ui.weather.fog",
        Weather::Storm => "ui.weather.storm",
        Weather::Snow => "ui.weather.snow",
    }
}

/// Pose for drawing (sitting people sit on their chair).
pub fn person_pose(v: &AgentView, t: f32) -> PersonPose {
    let f = facing(v.facing);
    let mut feet = vec2(v.feet.0, v.feet.1);
    if let Some((x, y)) = v.seat_tile {
        feet = render::sit_anchor(vec2(x as f32, y as f32) * TILE, f);
    }
    PersonPose {
        feet,
        facing: f,
        pose: pose(v.pose),
        t,
    }
}

/// "03:33" for minutes since 22:00.
pub fn clock_of_minute(minute: u32) -> String {
    let total = 22 * 60 + minute;
    format_clock((total / 60) % 24, total % 60)
}

/// The visible name of a staff member in lists (the owner is "Karolina (you)").
pub fn staff_name(loc: &Locale, s: &Staff) -> String {
    if s.owner || s.name.is_empty() {
        loc.tr_owned("ui.staff.you_row")
    } else {
        s.name.clone()
    }
}

/// Role line: archetype role key or the generic job name.
pub fn staff_role(loc: &Locale, content: &Content, s: &Staff) -> String {
    if s.owner {
        return loc.tr_owned("ui.crew.owner");
    }
    let key = format!("staff.{}.role", s.archetype);
    if loc.has(&key) {
        return loc.tr_owned(&key);
    }
    let best = best_job(s);
    let _ = content;
    loc.tr_owned(best.name_key())
}

pub fn best_job(s: &Staff) -> Job {
    let mut best = Job::ALL[0];
    for (i, j) in Job::ALL.iter().enumerate() {
        if s.skills[i] > s.skills[best_index(best)] {
            best = *j;
        }
    }
    best
}

fn best_index(j: Job) -> usize {
    Job::ALL.iter().position(|x| *x == j).unwrap_or(0)
}

/// Translated status/task line for the crew list.
pub fn staff_doing(loc: &Locale, s: &Staff) -> String {
    use sim::StaffStatus as St;
    match s.status {
        St::OffDuty => loc.tr_owned("ui.crew.status_off"),
        St::Expected { minute } => loc.trf(
            "ui.crew.status_expected",
            &[("time", &clock_of_minute(minute))],
        ),
        St::Sick => loc.tr_owned("ui.crew.status_sick"),
        St::WalkedOut => loc.tr_owned("ui.crew.status_walked_out"),
        St::Leaving => loc.tr_owned("ui.crew.status_leaving"),
        St::OnDuty => match &s.task {
            Some(t) => loc.tr_owned(&format!("ui.task.{}", t.id())),
            None => loc.tr_owned("ui.task.idle"),
        },
    }
}

/// Text for a log / bot / toast key with string args. Arg values that are i18n keys
/// (`regular.mirek.name`, `recipe.r_chai.name` — the sim's own args) are translated too.
pub fn tr_args(loc: &Locale, key: &str, args: &[(String, String)]) -> String {
    let owned: Vec<(String, String)> = args
        .iter()
        .map(|(k, v)| {
            let v = if v.contains('.') && !v.contains(' ') && loc.has(v) {
                loc.tr_owned(v)
            } else {
                v.clone()
            };
            (k.clone(), v)
        })
        .collect();
    let a: Vec<(&str, &str)> = owned
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    loc.trf(key, &a)
}

/// Mode / difficulty ids for save meta.
pub fn mode_id(w: &World) -> &'static str {
    match w.mode() {
        sim::Mode::Story => "story",
        sim::Mode::Sandbox => "sandbox",
    }
}

/// The sim's difficulty from a content id.
pub fn difficulty_from_id(id: &str) -> sim::Difficulty {
    if id == "classic" {
        sim::Difficulty::Classic
    } else {
        sim::Difficulty::Cozy
    }
}

/// Menu sandbox form → sim setup.
pub fn sandbox_setup(s: &crate::screens::menu::SandboxSetup) -> sim::SandboxSetup {
    let mut options = std::collections::BTreeMap::new();
    options.insert("rain_every_night".to_string(), s.rain_every_night);
    options.insert("pause_on_events".to_string(), s.pause_on_events);
    options.insert("allow_debt".to_string(), s.allow_debt);
    sim::SandboxSetup {
        seed: s.seed,
        difficulty: difficulty_from_id(&s.difficulty),
        town_name: None,
        cafe_name: (!s.cafe_name.trim().is_empty()).then(|| s.cafe_name.trim().to_string()),
        character: Some(s.character.clone()),
        start_money: Some(s.start_money),
        institutions: None,
        options,
    }
}

/// One guest of an archetype ("Traveller", not "Travellers"): `guest.<archetype>.one`,
/// falling back to the archetype's plural name.
pub fn guest_one(loc: &Locale, archetype: &str) -> String {
    let one = format!("guest.{archetype}.one");
    if loc.has(&one) {
        loc.tr_owned(&one)
    } else {
        loc.tr_owned(&format!("guest.{archetype}.name"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_from_minutes() {
        assert_eq!(clock_of_minute(0), "22:00");
        assert_eq!(clock_of_minute(333 - 120), "01:33");
        assert_eq!(clock_of_minute(5 * 60 + 33), "03:33");
        assert_eq!(clock_of_minute(480), "06:00");
    }
}
