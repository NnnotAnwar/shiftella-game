//! Stage directions: purely visual beats woven around the data-driven steps of a few
//! cutscenes (the content only says *what* happens; this says how it looks). Today only
//! the opening is staged: the train stops at Mlhov, you step out with a suitcase, walk to the
//! dark café, grandma's letter is written on a paper card, you go in with a warm light,
//! switch the lamps on one by one, and Shiftella greets you on the phone.
//!
//! Matching is by step kind / text key, so the content can be edited freely; a missing match
//! simply drops that beat.

use macroquad::prelude::*;

use crate::content::CutsceneStep;
use crate::map::TILE;

/// Train progress at which a stopping train stands in front of the platform spot.
pub const TRAIN_STOP: f32 = 0.492;

/// One item of the play queue.
#[derive(Debug, Clone)]
pub enum Op {
    Step(CutsceneStep),
    Stage(Stage),
}

/// A visual beat that isn't in the content data.
#[derive(Debug, Clone, PartialEq)]
pub enum Stage {
    /// Lamps off: the closed café.
    CafeDark,
    /// A train rolls in and stops at the platform (blocks until it stands).
    TrainArrive,
    /// The standing train leaves (non-blocking).
    TrainDepart,
    /// An actor appears at a world point.
    Appear {
        actor: String,
        at: Vec2,
        suitcase: bool,
    },
    /// Non-blocking walk.
    Walk {
        actor: String,
        to: Vec2,
    },
    /// The actor leaves the stage instantly (went through a door).
    Hide {
        actor: String,
    },
    /// Warm handheld light around the actor (also for its next appearance).
    HandLight {
        actor: String,
        on: bool,
    },
    /// Grandma's lines are written on a letter card instead of the dialogue box.
    LetterOn,
    LetterOff,
    /// Lamps switch on one by one (blocks).
    LightsCascade {
        seconds: f32,
    },
    /// The Shiftella app slides up with a bot message (text key); waits for a click.
    Phone {
        message_key: String,
    },
    Pause(f32),
}

fn is_text(step: &CutsceneStep, key: &str) -> bool {
    match step {
        CutsceneStep::Narration { text_key } | CutsceneStep::Line { text_key, .. } => {
            text_key == key
        }
        _ => false,
    }
}

/// Builds the play queue for `steps` of cutscene `id`. `top` = the whole scene (adds the
/// opening/closing beats); `false` for `If` branches and `Call`ed scenes.
pub fn expand(id: &str, steps: &[CutsceneStep], top: bool) -> Vec<Op> {
    match id {
        "opening" => opening(steps, top),
        _ => steps.iter().cloned().map(Op::Step).collect(),
    }
}

fn opening(steps: &[CutsceneStep], top: bool) -> Vec<Op> {
    let player = || "player".to_string();
    let mut out = Vec::new();
    if top {
        out.push(Op::Stage(Stage::CafeDark));
    }
    let mut train_done = false;
    for step in steps {
        // ── before
        if is_text(step, "scene.opening.03") {
            out.push(Op::Stage(Stage::TrainDepart));
        }
        if is_text(step, "scene.opening.04") {
            out.push(Op::Stage(Stage::Walk {
                actor: player(),
                to: vec2(6.4, -2.15) * TILE,
            }));
        }
        if matches!(step, CutsceneStep::Sfx { id } if id == "sfx_door_bell") {
            out.push(Op::Stage(Stage::Hide { actor: player() }));
            out.push(Op::Stage(Stage::HandLight {
                actor: player(),
                on: true,
            }));
        }
        // ── the step itself
        if matches!(step, CutsceneStep::TrainPass) && !train_done {
            train_done = true;
            out.push(Op::Stage(Stage::TrainArrive));
            out.push(Op::Stage(Stage::Pause(0.5)));
            out.push(Op::Stage(Stage::Appear {
                actor: player(),
                at: vec2(14.6, -4.92) * TILE,
                suitcase: true,
            }));
            out.push(Op::Stage(Stage::Pause(0.6)));
        } else {
            out.push(Op::Step(step.clone()));
        }
        // ── after
        if is_text(step, "scene.opening.06") {
            out.push(Op::Stage(Stage::LetterOn));
        }
        if is_text(step, "scene.opening.11") {
            out.push(Op::Stage(Stage::LetterOff));
        }
        if is_text(step, "scene.opening.14") {
            out.push(Op::Stage(Stage::HandLight {
                actor: player(),
                on: false,
            }));
            out.push(Op::Stage(Stage::LightsCascade { seconds: 4.5 }));
            out.push(Op::Stage(Stage::Pause(0.8)));
        }
    }
    if top {
        out.push(Op::Stage(Stage::Phone {
            message_key: "app.bot.hello".into(),
        }));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn other_scenes_are_untouched() {
        let steps = vec![CutsceneStep::TrainPass, CutsceneStep::Wait { seconds: 1.0 }];
        let ops = expand("mirek_first", &steps, true);
        assert_eq!(ops.len(), 2);
        assert!(matches!(ops[0], Op::Step(CutsceneStep::TrainPass)));
    }

    #[test]
    fn opening_gets_its_beats() {
        let steps = vec![
            CutsceneStep::TrainPass,
            CutsceneStep::Narration {
                text_key: "scene.opening.06".into(),
            },
            CutsceneStep::Narration {
                text_key: "scene.opening.14".into(),
            },
        ];
        let ops = expand("opening", &steps, true);
        let stages: Vec<&Stage> = ops
            .iter()
            .filter_map(|o| match o {
                Op::Stage(s) => Some(s),
                _ => None,
            })
            .collect();
        assert_eq!(stages[0], &Stage::CafeDark);
        assert!(stages.contains(&&Stage::TrainArrive));
        assert!(stages.contains(&&Stage::LetterOn));
        assert!(matches!(stages.last(), Some(Stage::Phone { .. })));
        assert!(
            !ops.iter()
                .any(|o| matches!(o, Op::Step(CutsceneStep::TrainPass))),
            "the opening train stops instead of passing"
        );
    }
}
