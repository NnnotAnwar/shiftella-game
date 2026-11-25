//! # Cutscene player (owner: cutscene agent)
//!
//! Plays a [`crate::content::CutsceneDef`]: letterbox, fades, smooth camera moves over the café
//! and street, actors walking along grid paths, trains (with the light sweep), glass dialogue
//! with portraits + typewriter, narration, choices, Lída's note cards, hold-Esc-to-skip, the
//! history log (Log button) and auto-advance (`A`).
//!
//! * [`CutscenePlayer::start`] → per frame [`CutscenePlayer::update`] (returns
//!   [`CutsceneStatus`]), [`CutscenePlayer::draw_world`] (its own café stage, see
//!   [`scene::CafeScene`]) and [`CutscenePlayer::draw_ui`].
//! * Audio is owned by the integration: [`CutscenePlayer::drain_audio_requests`].
//! * Story state: [`CutscenePlayer::set_condition_check`] answers `If` / choice conditions;
//!   effects come out live via [`CutscenePlayer::drain_effects`] and in
//!   [`CutscenePlayer::outcome`]. `Finished { id, choices }` maps to
//!   `Command::CutsceneFinished`.
//! * Visual-only stage directions for some scenes (the opening) live in [`staging`].

pub mod actors;
pub mod cards;
pub mod cast;
mod player;
pub mod scene;
pub mod staging;

pub use cast::Cast;
pub use player::{
    AudioRequest, ChoiceMade, CutsceneOutcome, CutscenePlayer, CutsceneStatus, StepInput,
};
pub use scene::CafeScene;

pub use crate::settings::Locale;
