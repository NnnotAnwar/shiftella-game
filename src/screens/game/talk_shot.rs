//! `SHIFTELLA_SCENE=talk` (verification shots): a regular's talk of `talks.ron` in progress over
//! the live café. `SHIFTELLA_CUTSCENE=<talk id>` (default `talk_vera_2`) picks it; the night is
//! the first of its act window (`SHIFTELLA_NIGHT`), the time a little after the regular's first
//! visit starts (`SHIFTELLA_TIME`), and `SHIFTELLA_T=<seconds>` plays that far into it.

use std::rc::Rc;

use super::GameScreen;
use crate::content::{ClockTime, Condition, Content, CutsceneTrigger};
use crate::settings::Locale;

/// The talk shown when `SHIFTELLA_CUTSCENE` is unset.
pub const DEFAULT_TALK: &str = "talk_vera_2";

/// Where a talk can be seen: (night, minutes since 22:00) — the first night of its window and
/// 20 minutes into the regular's first visit. `None` for an unknown id or a non-talk scene.
pub fn talk_defaults(content: &Content, id: &str) -> Option<(u32, i32)> {
    let cs = content.cutscene(id)?;
    let CutsceneTrigger::Talk { regular, night } = &cs.trigger else {
        return None;
    };
    let reg = content.regular(regular)?;
    let n = night
        .or_else(|| {
            cs.conditions.iter().find_map(|c| match c {
                Condition::NightBetween(a, _) => Some((*a).max(reg.first_night)),
                _ => None,
            })
        })
        .unwrap_or(reg.first_night);
    let from = reg.visits.first().map_or(ClockTime::OPEN, |v| v.from);
    let since_open =
        (from.minute_of_day as i32 - ClockTime::OPEN.minute_of_day as i32 + 1440) % 1440;
    Some((n, since_open + 20))
}

impl GameScreen {
    /// Plays talk `id` over the café (as "Talk" would) and `seconds` into it.
    pub fn stage_talk(&mut self, content: &Rc<Content>, loc: &Locale, id: &str, seconds: f32) {
        self.start_cutscene(content, id);
        if seconds > 0.0 {
            self.player.fast_forward(loc, seconds);
        }
    }
}
