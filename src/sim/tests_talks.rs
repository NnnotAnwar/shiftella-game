//! The regulars' talk dialogues (`talks.ron`): one per act for each regular who comes in that
//! act, offered only while that regular is in the café and only once per act; Anwar's
//! night-pinned talks keep working; every text exists in all four languages.

use super::*;
use crate::content::{Condition, Content, CutsceneDef, CutsceneStep, CutsceneTrigger, GameMode};

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");
const REGULARS: [&str; 6] = ["mirek", "hana", "vera", "filip", "radek", "lori"];

fn content() -> Content {
    Content::load(DATA).unwrap_or_else(|e| panic!("{e}"))
}

/// The act talks: `Talk { night: None }` cutscenes of the six story regulars.
fn act_talks(c: &Content) -> Vec<(&str, &CutsceneDef)> {
    c.cutscenes
        .cutscenes
        .iter()
        .filter_map(|cs| match &cs.trigger {
            CutsceneTrigger::Talk {
                regular,
                night: None,
            } if REGULARS.contains(&regular.as_str()) => Some((regular.as_str(), cs)),
            _ => None,
        })
        .collect()
}

fn window(cs: &CutsceneDef) -> (u32, u32) {
    cs.conditions
        .iter()
        .find_map(|c| match c {
            Condition::NightBetween(a, b) => Some((*a, *b)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{}: no NightBetween window", cs.id))
}

fn finish_cutscenes(w: &mut World, c: &Content) {
    while let Some(id) = w.playing_cutscene().map(str::to_string) {
        w.apply(
            c,
            Command::CutsceneFinished {
                id,
                choices: vec![],
            },
        )
        .unwrap();
    }
}

fn guest_of(w: &World, regular: &str) -> Option<u32> {
    let pid = w.story().regulars.get(regular)?.party?;
    let p = w.parties().iter().find(|p| p.id == pid)?;
    p.members
        .iter()
        .copied()
        .find(|m| w.guests().iter().any(|g| g.id == *m && g.leader && !g.gone))
}

/// Plays the night on (nobody behind the counter) until `regular` is in the café.
fn until_present(w: &mut World, c: &Content, regular: &str) -> u32 {
    if w.phase() == Phase::Day {
        finish_cutscenes(w, c);
        w.start_night(c);
    }
    for _ in 0..NIGHT_TICKS * 2 {
        finish_cutscenes(w, c);
        if let Some(g) = guest_of(w, regular) {
            return g;
        }
        if w.waiting_for_doors() {
            w.open_doors(c);
        }
        w.tick(c);
        let _ = w.drain_events();
    }
    panic!("{regular} never came in on night {}", w.night());
}

/// Headless nights until night `n` is the next one to start.
fn skip_to_night(w: &mut World, c: &Content, n: u32) {
    if w.phase() == Phase::Night {
        w.run_night_headless(c);
    }
    while w.night() < n {
        w.run_night_headless(c);
    }
}

fn trust(w: &World, r: &str) -> f32 {
    w.story().trust.get(r).copied().unwrap_or(0.0)
}

#[test]
fn every_regular_has_one_talk_per_act_they_come_in() {
    let c = content();
    let talks = act_talks(&c);
    let mut total = 0;
    for r in REGULARS {
        let first = c.regular(r).expect("regular").first_night;
        for act in &c.story.acts {
            let (from, to) = act.nights;
            let in_act: Vec<&str> = talks
                .iter()
                .filter(|(reg, cs)| {
                    let (a, b) = window(cs);
                    *reg == r && a >= from && b <= to
                })
                .map(|(_, cs)| cs.id.as_str())
                .collect();
            let expected = usize::from(first <= to);
            assert_eq!(in_act.len(), expected, "{r}, act {}: {in_act:?}", act.act);
            total += in_act.len();
        }
    }
    // Radek (night 9) and Lori (night 11) don't come in act I
    assert_eq!(total, 22);
    assert_eq!(talks.len(), total, "every act talk sits inside one act");
    for (r, cs) in &talks {
        assert!(!cs.repeat, "{}: once only", cs.id);
        assert!(
            cs.conditions.contains(&Condition::Mode(GameMode::Story)),
            "{}: story only",
            cs.id
        );
        // short: 3–6 spoken lines on any path, a little trust
        let lines = count_lines(&cs.steps);
        assert!((3..=8).contains(&lines), "{}: {lines} lines", cs.id);
        assert!(gives_trust(&cs.steps, r), "{}: no trust for {r}", cs.id);
    }
}

fn count_lines(steps: &[CutsceneStep]) -> usize {
    steps
        .iter()
        .map(|s| match s {
            CutsceneStep::Line { .. } | CutsceneStep::Narration { .. } => 1,
            CutsceneStep::If {
                then, otherwise, ..
            } => count_lines(then).max(count_lines(otherwise)),
            _ => 0,
        })
        .sum()
}

fn gives_trust(steps: &[CutsceneStep], r: &str) -> bool {
    let trust = |e: &crate::content::Effect| matches!(e, crate::content::Effect::Trust(who, v) if who == r && *v > 0.0 && *v <= 5.0);
    steps.iter().any(|s| match s {
        CutsceneStep::Apply { effects } => effects.iter().any(trust),
        CutsceneStep::Choice { options } => options.iter().all(|o| o.effects.iter().any(trust)),
        _ => false,
    })
}

fn text_keys(steps: &[CutsceneStep], out: &mut Vec<String>) {
    for s in steps {
        match s {
            CutsceneStep::Line { text_key, .. } | CutsceneStep::Narration { text_key } => {
                out.push(text_key.clone())
            }
            CutsceneStep::Choice { options } => {
                out.extend(options.iter().map(|o| o.label_key.clone()))
            }
            CutsceneStep::If {
                then, otherwise, ..
            } => {
                text_keys(then, out);
                text_keys(otherwise, out);
            }
            _ => {}
        }
    }
}

#[test]
fn talk_texts_exist_in_four_languages() {
    let c = content();
    let root = env!("CARGO_MANIFEST_DIR");
    let i18n = crate::settings::load_i18n(&format!("{root}/assets/i18n")).expect("i18n");
    for (_, cs) in act_talks(&c) {
        let mut keys = Vec::new();
        text_keys(&cs.steps, &mut keys);
        assert!(!keys.is_empty());
        for k in keys {
            assert!(i18n.has_all(&k), "{}: `{k}` missing a language", cs.id);
            for lang in crate::i18n::Lang::ALL {
                let t = i18n.tr(lang, &k);
                assert!(
                    !t.trim().is_empty() && t != k,
                    "{}: `{k}` empty in {lang:?}",
                    cs.id
                );
            }
        }
    }
}

#[test]
fn a_talk_is_offered_to_its_present_regular_once_per_act() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    w.start_night(&c);
    // act I, night 1: Mirek walks in with his act I talk; nobody else is offered his
    let g = until_present(&mut w, &c, "mirek");
    assert_eq!(w.talk_available(&c, g).as_deref(), Some("talk_mirek_1"));
    let party = w.guests().iter().find(|y| y.id == g).map(|y| y.party);
    for other in w.guests().iter().filter(|x| Some(x.party) != party) {
        if let Some(t) = w.talk_available(&c, other.id) {
            assert!(
                !t.starts_with("talk_mirek"),
                "{t} offered to guest {}",
                other.id
            );
        }
    }
    let before = trust(&w, "mirek");
    w.apply(&c, Command::Talk { guest: g }).expect("talk");
    assert_eq!(w.playing_cutscene(), Some("talk_mirek_1"));
    finish_cutscenes(&mut w, &c);
    assert!(
        (trust(&w, "mirek") - before - 3.0).abs() < 0.01,
        "a little trust"
    );
    assert!(w.story().talks.contains("talk_mirek_1"));
    // not twice: the rest of tonight, nor the next night of the same act
    assert_eq!(w.talk_available(&c, g), None);
    assert!(w.apply(&c, Command::Talk { guest: g }).is_err());
    w.run_night_headless(&c);
    let g = until_present(&mut w, &c, "mirek");
    assert_eq!(w.night(), 2);
    assert_eq!(w.talk_available(&c, g), None, "one talk per act");
    // act II: the next one, only now
    skip_to_night(&mut w, &c, 8);
    let g = until_present(&mut w, &c, "mirek");
    assert_eq!(w.talk_available(&c, g).as_deref(), Some("talk_mirek_2"));
    w.apply(&c, Command::Talk { guest: g }).expect("talk");
    finish_cutscenes(&mut w, &c);
    assert_eq!(w.talk_available(&c, g), None);
}

#[test]
fn a_missed_talk_stays_in_its_act() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    // Vera's act I talk never opened: on night 8 she has act II's, not act I's
    skip_to_night(&mut w, &c, 8);
    assert!(!w.story().played.contains("talk_vera_1"));
    let g = until_present(&mut w, &c, "vera");
    assert_eq!(w.talk_available(&c, g).as_deref(), Some("talk_vera_2"));
    // and Anwar's talk on night 9 still works (pinned to the night, untouched by the acts)
    skip_to_night(&mut w, &c, 9);
    let g = until_present(&mut w, &c, "guest_anwar");
    assert_eq!(w.talk_available(&c, g).as_deref(), Some("anwar_talk_9"));
    w.apply(&c, Command::Talk { guest: g }).expect("anwar talk");
    assert_eq!(w.playing_cutscene(), Some("anwar_talk_9"));
    finish_cutscenes(&mut w, &c);
    assert_eq!(w.talk_available(&c, g), None);
}

#[test]
fn no_act_talks_in_the_sandbox() {
    let c = content();
    let mut w = World::new_sandbox(
        &c,
        SandboxSetup {
            seed: 3,
            ..Default::default()
        },
    );
    for (_, cs) in act_talks(&c) {
        let mut ctx = EffectCtx::default();
        assert!(
            !super::logic::all(&mut w, &c, &cs.conditions, &mut ctx),
            "{} would play in the sandbox",
            cs.id
        );
    }
}
