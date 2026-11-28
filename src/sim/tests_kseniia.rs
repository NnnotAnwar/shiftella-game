//! The Kseniia cameo (`guest_kseniia`): she comes only on her nights (story 3, 15, 20 and the
//! same sandbox nights), has a talk each visit and is flavour only. The night-15 choice sets
//! `kseniia_ex` (answered | silent), which picks her night-20 variant (unset: the neutral one)
//! and switches one line of Anwar's night-24 talk. She is gone before Karolina's back-step
//! call on night 3 (04:20), and she never meets Anwar.

use super::tests_story_large::visible;
use super::tests_t10::{content, night_of, play_story_with};
use super::*;
use crate::content::{Condition, Content, CutsceneStep, CutsceneTrigger, Effect};

const ID: &str = "guest_kseniia";
const VAR: &str = "kseniia_ex";
/// Anwar's night-24 easter-egg line (only when she answered her ex).
const EGG: &str = "scene.anwar_talk_24.07";
const EGG_AMBIENT: &str = "regular.guest_anwar.ambient.recommended";
const TALKS: [(u32, &str); 3] = [
    (3, "kseniia_talk_3"),
    (15, "kseniia_talk_15"),
    (20, "kseniia_talk_20"),
];

fn visit_nights(w: &World, id: &str) -> Vec<u32> {
    let mut v: Vec<u32> = w
        .story()
        .regulars
        .get(id)
        .map(|s| s.visits.iter().map(|v| v.night).collect())
        .unwrap_or_default();
    v.dedup();
    v
}

/// The night-15 answer (`None`: the talk never opened — the var's default "none", or a save
/// from before the cameo without the var at all).
fn var(w: &World) -> Option<&str> {
    w.story()
        .vars
        .get(VAR)
        .map(String::as_str)
        .filter(|v| *v != "none")
}

fn has(keys: &[String], k: &str) -> bool {
    keys.iter().any(|x| x == k)
}

fn finish(w: &mut World, c: &Content, choices: Vec<usize>) {
    if let Some(id) = w.playing_cutscene().map(str::to_string) {
        w.apply(c, Command::CutsceneFinished { id, choices })
            .unwrap();
    }
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
        finish(w, c, vec![]);
        w.start_night(c);
    }
    for _ in 0..NIGHT_TICKS * 2 {
        finish(w, c, vec![]);
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

fn skip_to_night(w: &mut World, c: &Content, n: u32) {
    if w.phase() == Phase::Night {
        w.run_night_headless(c);
    }
    while w.night() < n {
        w.run_night_headless(c);
    }
}

/// Walks a story to night 15 and opens her talk there (the talks of nights 3 and 15 checked
/// on the way); `choice` answers the night-15 question (`None`: no answer given).
fn story_to_night_15(c: &Content, choice: Option<usize>) -> World {
    let mut w = World::new_story(c, Difficulty::Cozy);
    skip_to_night(&mut w, c, 3);
    let g = until_present(&mut w, c, ID);
    assert_eq!(w.talk_available(c, g).as_deref(), Some("kseniia_talk_3"));
    w.apply(c, Command::Talk { guest: g }).expect("talk 3");
    finish(&mut w, c, vec![]);
    assert_eq!(w.talk_available(c, g), None, "one talk per visit");
    skip_to_night(&mut w, c, 15);
    let g = until_present(&mut w, c, ID);
    assert_eq!(w.talk_available(c, g).as_deref(), Some("kseniia_talk_15"));
    w.apply(c, Command::Talk { guest: g }).expect("talk 15");
    assert_eq!(w.playing_cutscene(), Some("kseniia_talk_15"));
    finish(&mut w, c, choice.into_iter().collect());
    w
}

#[test]
fn she_comes_on_her_nights_only_with_a_talk_each_visit() {
    let c = content();
    let r = c.regular(ID).expect("guest_kseniia");
    assert!(r.is_cameo() && r.look_preset.as_deref() == Some("kseniia"));
    for n in 1..=30 {
        let hers = [3, 15, 20].contains(&n);
        assert_eq!(r.visits_story_night(n), hers, "story night {n}");
        assert_eq!(r.sandbox_visit_rule(n), Some(hers), "sandbox night {n}");
        // she and Anwar never meet
        assert!(!(hers && c.regular("guest_anwar").unwrap().visits_story_night(n)));
    }
    // a whole attentive story: she visits on 3, 15 and 20 and talks each time
    let (w, log) = play_story_with(&c, 4242, Difficulty::Cozy, &|_| None, &mut |_| {});
    assert_eq!(visit_nights(&w, ID), vec![3, 15, 20]);
    assert_eq!(visit_nights(&w, "guest_anwar"), vec![9, 24]);
    // (the bot takes a talk pinned to tonight at once, as the night auto-pauses for it)
    for (n, talk) in TALKS {
        assert_eq!(night_of(&log, talk), Some(n), "{talk}");
    }
    assert_eq!(night_of(&log, "anwar_talk_9"), Some(9));
    assert_eq!(night_of(&log, "anwar_talk_24"), Some(24));
    assert!(log.iter().all(|(n, id)| !id.starts_with("kseniia") || [3, 15, 20].contains(n)));
    assert_eq!(w.ending(), Some("light"));
}

#[test]
fn the_night_15_choice_sets_the_var_once() {
    let c = content();
    let mut w = story_to_night_15(&c, Some(0));
    assert_eq!(var(&w), Some("answered"));
    // her third visit: the night-20 talk is offered and plays the answered variant
    skip_to_night(&mut w, &c, 20);
    let g = until_present(&mut w, &c, ID);
    assert_eq!(w.talk_available(&c, g).as_deref(), Some("kseniia_talk_20"));
    assert!(visible(&w, &c, "kseniia_talk_20").iter().any(|k| k == "scene.kseniia_talk_20.01"));
    w.apply(&c, Command::Talk { guest: g }).expect("talk 20");
    finish(&mut w, &c, vec![]);
    assert_eq!(w.talk_available(&c, g), None);
    assert_eq!(var(&w), Some("answered"), "night 20 doesn't touch the var");
    let w = story_to_night_15(&c, Some(1));
    assert_eq!(var(&w), Some("silent"));
    // no answer given (the bot's default finish): unset, which reads as the neutral variant
    let w = story_to_night_15(&c, None);
    assert_eq!(var(&w), None);
}

#[test]
fn night_20_and_anwars_night_24_follow_the_var() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    let answered = ["scene.kseniia_talk_20.01", "scene.kseniia_talk_20.03"];
    let silent = ["scene.kseniia_talk_20.06", "scene.kseniia_talk_20.07"];
    let neutral = ["scene.kseniia_talk_20.10", "scene.kseniia_talk_20.11"];
    let egg_line = c
        .regular("guest_anwar")
        .unwrap()
        .ambient_lines
        .iter()
        .find(|l| l.key == EGG_AMBIENT)
        .expect("Anwar's easter-egg ambient line")
        .when
        .clone();
    for (value, want) in [
        (Some("answered"), answered),
        (Some("silent"), silent),
        (Some("none"), neutral),
        // an old save, from before the var existed
        (None, neutral),
    ] {
        match value {
            Some(v) => w.debug_set_var(VAR, v),
            None => {
                w.story.vars.remove(VAR);
            }
        }
        let keys = visible(&w, &c, "kseniia_talk_20");
        for set in [answered, silent, neutral] {
            for k in set {
                assert_eq!(has(&keys, k), set == want, "{value:?}: {k} in {keys:?}");
            }
        }
        // the common close plays in every variant
        assert!(has(&keys, "scene.kseniia_talk_20.12"), "{keys:?}");
        // Anwar: the "not just someone" line only when she answered
        let egg = value == Some("answered");
        let anwar = visible(&w, &c, "anwar_talk_24");
        assert_eq!(has(&anwar, EGG), egg, "{value:?}: {anwar:?}");
        assert!(has(&anwar, "scene.anwar_talk_24.06"), "the rest of his talk stays");
        assert_eq!(w.eval_conditions(&c, &egg_line), egg, "{value:?}: ambient");
    }
}

#[test]
fn she_is_gone_before_the_back_step_call_on_night_3() {
    let c = content();
    let backstep = c.cutscene("karolina_backstep").expect("K1");
    let CutsceneTrigger::At { night: 3, time } = &backstep.trigger else {
        panic!("karolina_backstep moved: {:?}", backstep.trigger);
    };
    let r = c.regular(ID).unwrap();
    for v in &r.visits {
        assert!(
            v.to.minutes_since_open() + 5 <= time.minutes_since_open(),
            "her visit ends at {} — too close to the back step at {time}",
            v.to
        );
    }
    // night 3 played by the player-bot: she comes, talks, is served, and has left by the
    // time the call comes. (Left unserved she would wait on: since 0.3.0 a waiting regular
    // goes only when her visible patience bar runs out, so only the served visit has an end.)
    let mut w = World::new_story_seeded(&c, Difficulty::Cozy, 4242);
    let mut b = bot::PlayerBot::new(bot::BotConfig {
        seed: 4242,
        ..Default::default()
    });
    while w.night() < 3 {
        b.run_night(&mut w, &c);
    }
    if w.phase() == Phase::Day {
        b.day(&mut w, &c);
        bot::finish_cutscenes_attentive(&mut w, &c);
        w.start_night(&c);
    }
    let mut seen = false;
    let mut talked = false;
    for _ in 0..NIGHT_TICKS * 4 {
        match w.playing_cutscene() {
            Some("karolina_backstep") => {
                seen = true;
                assert!(guest_of(&w, ID).is_none(), "Kseniia still in the café at 04:20");
                assert!(!w.story().queue.iter().any(|q| q.starts_with("kseniia")));
                break;
            }
            Some("kseniia_talk_3") => talked = true,
            _ => {}
        }
        bot::finish_cutscenes_attentive(&mut w, &c);
        if w.phase() != Phase::Night {
            break;
        }
        b.step(&mut w, &c);
        w.tick(&c);
        let _ = w.drain_events();
    }
    assert!(talked, "her night-3 talk played");
    assert!(seen, "the back step played on night 3");
}

#[test]
fn she_comes_on_her_sandbox_nights_too() {
    let c = content();
    let mut w = World::new_sandbox(
        &c,
        SandboxSetup {
            seed: 5,
            ..Default::default()
        },
    );
    skip_to_night(&mut w, &c, 3);
    let g = until_present(&mut w, &c, ID);
    assert_eq!(w.talk_available(&c, g).as_deref(), Some("kseniia_talk_3"));
    w.run_night_headless(&c);
    skip_to_night(&mut w, &c, 6);
    assert_eq!(visit_nights(&w, ID), vec![3]);
}

/// Every effect and var use of the cameo: a choice that sets `kseniia_ex`, read only by her
/// own talks and Anwar's — no arc, flag, trust, money or ending.
#[test]
fn she_is_flavour_only() {
    let c = content();
    let r = c.regular(ID).unwrap();
    assert!(r.absent_if.is_empty() && r.weather.is_empty());
    assert!(
        r.stages
            .iter()
            .all(|s| s.effects.is_empty() && s.lines.is_empty() && s.conditions.is_empty()),
        "no arc"
    );
    fn walk(steps: &[CutsceneStep], f: &mut dyn FnMut(&CutsceneStep)) {
        for s in steps {
            f(s);
            if let CutsceneStep::If {
                then, otherwise, ..
            } = s
            {
                walk(then, f);
                walk(otherwise, f);
            }
        }
    }
    fn reads_var(conds: &[Condition]) -> bool {
        conds.iter().any(|c| match c {
            Condition::VarIs(v, _) => v == VAR,
            Condition::Not(x) => reads_var(std::slice::from_ref(&**x)),
            Condition::Any(xs) | Condition::All(xs) => reads_var(xs),
            _ => false,
        })
    }
    for cs in &c.cutscenes.cutscenes {
        let hers = cs.id.starts_with("kseniia_talk_");
        let mut sets = Vec::new();
        let mut reads = reads_var(&cs.conditions);
        walk(&cs.steps, &mut |s| match s {
            CutsceneStep::Choice { options } => {
                for o in options {
                    reads |= reads_var(&o.conditions);
                    for e in &o.effects {
                        match e {
                            Effect::SetVar(v, val) if v == VAR => sets.push(val.clone()),
                            _ if hers => panic!("{}: effect {e:?}", cs.id),
                            _ => {}
                        }
                    }
                    assert!(!hers || o.set_flag.is_none(), "{}: a flag", cs.id);
                }
            }
            CutsceneStep::Apply { effects } => {
                assert!(!hers, "{}: effects {effects:?}", cs.id);
                assert!(
                    !effects
                        .iter()
                        .any(|e| matches!(e, Effect::SetVar(v, _) if v == VAR)),
                    "{}",
                    cs.id
                );
            }
            CutsceneStep::If { conditions, .. } => reads |= reads_var(conditions),
            _ => {}
        });
        if hers {
            assert!(
                matches!(&cs.trigger, CutsceneTrigger::Talk { regular, night: Some(_) } if regular == ID),
                "{}",
                cs.id
            );
        }
        if !sets.is_empty() {
            assert_eq!(cs.id, "kseniia_talk_15", "only night 15 sets {VAR}");
            assert_eq!(sets, ["answered", "silent"]);
        }
        if reads {
            assert!(
                ["kseniia_talk_15", "kseniia_talk_20", "anwar_talk_24"].contains(&cs.id.as_str()),
                "{} reads {VAR}",
                cs.id
            );
        }
    }
    // the endings and the night titles never read it
    for e in &c.story.endings {
        assert!(!reads_var(&e.conditions), "ending {}", e.id);
    }
    for reg in &c.regulars {
        assert!(!reads_var(&reg.absent_if), "{}", reg.id);
        for l in &reg.ambient_lines {
            if reads_var(&l.when) {
                assert_eq!(l.key, EGG_AMBIENT);
            }
        }
    }
}

/// The two answers end the same story: same ending, money, reputation, arcs and flags.
#[test]
fn answered_or_silent_the_story_is_the_same() {
    let c = content();
    let play = |pick: usize| {
        play_story_with(
            &c,
            4242,
            Difficulty::Cozy,
            &move |id| (id == "kseniia_talk_15").then(|| vec![pick]),
            &mut |_| {},
        )
        .0
    };
    let a = play(0);
    let s = play(1);
    assert_eq!(var(&a), Some("answered"));
    assert_eq!(var(&s), Some("silent"));
    assert_eq!(a.ending(), s.ending());
    assert_eq!(a.ending_variant(), s.ending_variant());
    assert_eq!(a.money(), s.money());
    assert_eq!(a.reputation(), s.reputation());
    assert_eq!(a.story().counters, s.story().counters);
    assert_eq!(a.story().flags, s.story().flags);
    assert_eq!(a.story().trust, s.story().trust);
}

#[test]
fn her_texts_exist_in_four_languages() {
    let c = content();
    let root = env!("CARGO_MANIFEST_DIR");
    let i18n = crate::settings::load_i18n(&format!("{root}/assets/i18n")).expect("i18n");
    fn keys(steps: &[CutsceneStep], out: &mut Vec<String>) {
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
                    keys(then, out);
                    keys(otherwise, out);
                }
                _ => {}
            }
        }
    }
    let r = c.regular(ID).unwrap();
    let mut all = vec![
        r.name_key(),
        r.bio_key(),
        "wish.guest_kseniia.espresso".to_string(),
        "journal.caption.kseniia".to_string(),
        EGG.to_string(),
        EGG_AMBIENT.to_string(),
    ];
    all.extend(r.ambient_lines.iter().map(|l| l.key.clone()));
    for (_, id) in TALKS {
        let cs = c.cutscene(id).unwrap_or_else(|| panic!("{id}"));
        keys(&cs.steps, &mut all);
    }
    assert!(all.len() > 40, "{}", all.len());
    for k in all {
        assert!(i18n.has_all(&k), "`{k}` missing a language");
        for lang in crate::i18n::Lang::ALL {
            let t = i18n.tr(lang, &k);
            assert!(!t.trim().is_empty() && t != k, "`{k}` empty in {lang:?}");
        }
    }
    // her name per language (uk/ru in Cyrillic)
    let name = |lang| i18n.tr(lang, &r.name_key()).to_string();
    use crate::i18n::Lang;
    assert_eq!(name(Lang::Cs), "Kseniia");
    assert_eq!(name(Lang::En), "Kseniia");
    assert_eq!(name(Lang::Uk), "Ксенія");
    assert_eq!(name(Lang::Ru), "Ксения");
}


/// Without her the same attentive story ends the same way: the same ending and variant, the
/// same arcs ended well (she only adds a guest's ordinary order on three nights).
#[test]
fn the_story_ends_the_same_without_her() {
    let c = content();
    let mut bare = c.clone();
    bare.regulars.retain(|r| r.id != ID);
    bare.cutscenes
        .cutscenes
        .retain(|cs| !cs.id.starts_with("kseniia_talk_"));
    let (with, _) = play_story_with(&c, 4242, Difficulty::Cozy, &|_| None, &mut |_| {});
    let (without, _) = play_story_with(&bare, 4242, Difficulty::Cozy, &|_| None, &mut |_| {});
    assert_eq!(with.ending(), without.ending());
    assert_eq!(with.ending_variant(), without.ending_variant());
    let arcs = |w: &World| -> Vec<String> {
        w.story()
            .flags
            .iter()
            .filter(|f| f.starts_with("arc.") && f.ends_with(".good"))
            .cloned()
            .collect()
    };
    assert_eq!(arcs(&with), arcs(&without));
}

