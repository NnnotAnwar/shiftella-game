//! The "large" story tier: Karolina's own arc (K1–K4), Šindelář's storm visit, the landlord's
//! thread and the endings by variant B — the franchise offer stands from night 17, "Sign now" /
//! "Not yet" on night 22 (and again on Classic night 28 when money is short), the last word on
//! night 30 at 05:30, after 3:33 and the letter. Every ending path and every letter overlay is
//! reached here; most of them by playing whole stories with the attentive player-bot.

use super::tests_t10::{SceneLog, content, letter, night_of, play_story, play_story_with};
use super::*;
use crate::content::{Content, CutsceneStep};

/// The text keys (and `call:<id>` markers) a cutscene shows for the world as it is now: `If`
/// steps follow the current state, `Call` steps are walked into. For the ending scenes, which
/// have no choices, this is exactly what the player read.
fn visible(w: &World, c: &Content, id: &str) -> Vec<String> {
    fn walk(w: &World, c: &Content, steps: &[CutsceneStep], out: &mut Vec<String>, depth: u32) {
        if depth > 6 {
            return;
        }
        for s in steps {
            match s {
                CutsceneStep::Narration { text_key } | CutsceneStep::Line { text_key, .. } => {
                    out.push(text_key.clone())
                }
                CutsceneStep::If {
                    conditions,
                    then,
                    otherwise,
                } => {
                    let b = if w.eval_conditions(c, conditions) {
                        then
                    } else {
                        otherwise
                    };
                    walk(w, c, b, out, depth + 1);
                }
                CutsceneStep::Call { cutscene } => {
                    out.push(format!("call:{cutscene}"));
                    if let Some(cs) = c.cutscene(cutscene) {
                        walk(w, c, &cs.steps, out, depth + 1);
                    }
                }
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    if let Some(cs) = c.cutscene(id) {
        walk(w, c, &cs.steps, &mut out, 0);
    }
    out
}

fn has(keys: &[String], k: &str) -> bool {
    keys.iter().any(|x| x == k)
}

fn ending_scene(w: &World) -> String {
    format!("ending_{}", w.ending().expect("an ending"))
}

fn offer(w: &World) -> &str {
    w.story().vars.get("offer").map_or("none", String::as_str)
}

/// The scenes of one night, in the order they played.
fn scenes_on(log: &SceneLog, night: u32) -> Vec<&str> {
    log.iter()
        .filter(|(n, _)| *n == night)
        .map(|(_, id)| id.as_str())
        .collect()
}

fn pos(list: &[&str], id: &str) -> usize {
    list.iter()
        .position(|x| *x == id)
        .unwrap_or_else(|| panic!("{id} not in {list:?}"))
}

// ---------------------------------------------------------------------------------------------
// Karolina (K1–K4) and the storm night
// ---------------------------------------------------------------------------------------------

#[test]
fn karolinas_arc_plays_from_the_back_step_to_the_storm() {
    let c = content();
    let (w, log) = play_story_with(&c, 4242, Difficulty::Cozy, &|_| None, &mut |_| {});
    // K1: mum's call on the back step, night 3
    assert_eq!(night_of(&log, "karolina_backstep"), Some(3));
    // K2: Vera, night 13 — the attentive bot takes the first option (Lida's way)
    assert_eq!(night_of(&log, "karolina_not_lida"), Some(13));
    assert_eq!(
        w.story().vars.get("karolina_way").map(String::as_str),
        Some("lida")
    );
    // K3 lives inside the franchise visit; K4 opens Mirek's confession
    assert_eq!(night_of(&log, "franchise_visit"), Some(17));
    let k4 = log
        .iter()
        .position(|(_, id)| id == "karolina_storm")
        .expect("K4 played");
    assert_eq!(
        log[k4 + 1].1,
        "mirek_confession",
        "K4 leads straight into the confession"
    );
    // the ending: Vera's crossword and the board in Lida's hand
    let keys = visible(&w, &c, "ending_light");
    assert!(
        has(&keys, "scene.ending_light.21"),
        "the crossword: {keys:?}"
    );
    assert!(has(&keys, "scene.ending_light.22"));
    assert!(!has(&keys, "scene.ending_light.16") && !has(&keys, "scene.ending_light.23"));
}

#[test]
fn karolinas_own_way_changes_her_last_line() {
    let c = content();
    let (w, _) = play_story_with(
        &c,
        4242,
        Difficulty::Cozy,
        &|id| (id == "karolina_not_lida").then(|| vec![1]),
        &mut |_| {},
    );
    assert_eq!(
        w.story().vars.get("karolina_way").map(String::as_str),
        Some("own")
    );
    let keys = visible(&w, &c, "ending_light");
    assert!(has(&keys, "scene.ending_light.23"), "{keys:?}");
    assert!(!has(&keys, "scene.ending_light.22"));
}

#[test]
fn the_storm_night_plays_one_scene_at_a_time_in_order() {
    let c = content();
    // Lori sent away on night 11: the fullest storm night
    let (_, log) = play_story_with(
        &c,
        4242,
        Difficulty::Cozy,
        &|id| (id == "lori_platform").then(|| vec![1]),
        &mut |_| {},
    );
    let n25 = scenes_on(&log, 25);
    let order = [
        "act4_storm",
        "lori_storm",
        "karolina_storm",
        "mirek_confession",
        "sindelar_storm",
    ];
    for pair in order.windows(2) {
        assert!(
            pos(&n25, pair[0]) < pos(&n25, pair[1]),
            "{} before {}: {n25:?}",
            pair[0],
            pair[1]
        );
    }
    // the night isn't one wall of scenes: the confession and Šindelář are hours apart
    assert!(n25.len() <= 7, "{n25:?}");
}

#[test]
fn the_storm_chain_still_plays_when_mirek_is_behind() {
    // "Say nothing" at the near-miss: no confession on the storm night (Jana takes his keys
    // after the accident), but the storm and Šindelář still come, and K4 waits for the
    // confession on a later night
    let c = content();
    let (_, log) = play_story_with(
        &c,
        4242,
        Difficulty::Cozy,
        &|id| (id == "mirek_pedestrian").then(|| vec![2]),
        &mut |_| {},
    );
    let n25 = scenes_on(&log, 25);
    assert!(
        n25.contains(&"act4_storm") && n25.contains(&"sindelar_storm"),
        "{n25:?}"
    );
    assert!(!n25.contains(&"karolina_storm"), "{n25:?}");
    let k4 = night_of(&log, "karolina_storm").expect("K4 later");
    assert_eq!(Some(k4), night_of(&log, "mirek_confession"));
    assert!(k4 >= 26);
}

// ---------------------------------------------------------------------------------------------
// The ending matrix (variant B)
// ---------------------------------------------------------------------------------------------

#[test]
fn cozy_not_signed_keeps_the_light_with_the_last_word_on_the_last_morning() {
    let c = content();
    let (w, log) = play_story_with(&c, 4242, Difficulty::Cozy, &|_| None, &mut |_| {});
    // night 22: "Not yet" keeps the offer open; the last word comes at 05:30 on night 30,
    // after 3:33
    assert_eq!(night_of(&log, "franchise_decision"), Some(22));
    let n30 = scenes_on(&log, 30);
    assert!(
        pos(&n30, "last_333") < pos(&n30, "franchise_last"),
        "{n30:?}"
    );
    assert_eq!(offer(&w), "rejected");
    assert!(!w.flag("signed_last_morning"));
    assert_eq!(w.ending(), Some("light"));
    assert_eq!(w.ending_variant(), Some("full"));
    // the letter delivered: the answer plays right before the window, no coda
    let keys = visible(&w, &c, "ending_light");
    let answer = keys
        .iter()
        .position(|k| k == "call:answer")
        .expect("answer");
    let window = keys
        .iter()
        .position(|k| k == "scene.ending_light.14")
        .unwrap();
    assert!(answer < window);
    assert!(!has(&keys, "call:letter_coda"));
    // the landlord's thread ends on forty-two messages
    assert!(has(&keys, "scene.ending_light.19") || has(&keys, "scene.ending_light.03"));
    assert!(has(&keys, "scene.ending_light.20"));
}

#[test]
fn signing_on_night_22_sells_the_cafe_and_the_333_guest_stays_on_the_platform() {
    let c = content();
    let (w, log) = play_story_with(
        &c,
        77,
        Difficulty::Cozy,
        &|id| (id == "franchise_decision").then(|| vec![0]),
        &mut |_| {},
    );
    assert_eq!(offer(&w), "accepted");
    assert_eq!(w.ending(), Some("last_train"));
    assert_eq!(w.ending_variant(), Some("sold"));
    assert!(w.story().played.contains("last_333_sold"));
    assert!(!w.story().played.contains("last_333"));
    assert!(
        !w.story().played.contains("franchise_last"),
        "nothing left to sign"
    );
    assert!(!w.story().played.contains("answer"));
    // Šindelář in the storm knows the café is his now
    assert_eq!(night_of(&log, "sindelar_storm"), Some(25));
    let keys = visible(&w, &c, "ending_last_train");
    assert!(
        has(&keys, "scene.ending_last_train.01"),
        "the sign on the 05:20"
    );
    assert!(!has(&keys, "scene.ending_last_train.21"));
    assert!(has(&keys, "scene.ending_last_train.02"));
}

#[test]
fn signing_on_the_last_morning_sells_after_the_answer() {
    let c = content();
    let (w, log) = play_story_with(
        &c,
        4242,
        Difficulty::Cozy,
        &|id| (id == "franchise_last").then(|| vec![0]),
        &mut |_| {},
    );
    assert_eq!(night_of(&log, "franchise_last"), Some(30));
    assert!(w.flag("signed_last_morning"));
    assert_eq!(w.ending(), Some("last_train"));
    assert_eq!(w.ending_variant(), Some("sold"));
    // 3:33 happened as usual before the signature: the letter went, the answer plays, and the
    // sold night's platform scene doesn't
    assert!(w.story().played.contains("last_333"));
    assert!(!w.story().played.contains("last_333_sold"));
    assert!(["delivered", "delivered_by_mirek"].contains(&letter(&w)));
    assert!(w.story().played.contains("answer"));
    let keys = visible(&w, &c, "ending_last_train");
    assert!(has(&keys, "call:answer"));
    assert!(
        has(&keys, "scene.ending_last_train.21"),
        "the sign comes tomorrow"
    );
    assert!(!has(&keys, "scene.ending_last_train.01"));
}

#[test]
fn classic_with_a_missed_last_payment_ends_on_the_last_train() {
    let c = content();
    let (w, log) = play_story_with(&c, 4242, Difficulty::Classic, &|_| None, &mut |w| {
        if w.night() == 30 {
            w.debug_set_money(0);
        }
    });
    // she keeps the light, knowing the till won't cover it
    assert_eq!(night_of(&log, "franchise_last"), Some(30));
    let fl = visible(&w, &c, "franchise_last");
    assert!(has(&fl, "scene.franchise_last.06") || w.money() >= 30_000);
    assert_eq!(offer(&w), "rejected");
    assert!(w.flag("final_payment_missed"));
    assert_eq!(w.ending(), Some("last_train"));
    assert!(
        matches!(w.ending_variant(), Some("farewell") | Some("closed")),
        "{:?}",
        w.ending_variant()
    );
    // the landlord, not Šindelář; forty-two messages anyway
    let keys = visible(&w, &c, "ending_last_train");
    assert!(has(&keys, "scene.ending_last_train.22"));
    assert!(!has(&keys, "scene.ending_last_train.02"));
}

#[test]
fn classic_money_short_on_night_28_brings_the_offer_back() {
    let c = content();
    let (w, log) = play_story_with(
        &c,
        4242,
        Difficulty::Classic,
        &|id| (id == "franchise_again").then(|| vec![0]),
        &mut |w| {
            if w.night() == 28 {
                w.debug_set_money(0);
            }
        },
    );
    assert_eq!(night_of(&log, "franchise_again"), Some(28));
    assert_eq!(offer(&w), "accepted");
    assert_eq!(w.ending(), Some("last_train"));
    assert_eq!(w.ending_variant(), Some("sold"));
    assert!(w.story().played.contains("last_333_sold"));
}

#[test]
fn classic_with_the_money_keeps_the_light() {
    let c = content();
    let (w, log) = play_story_with(&c, 4242, Difficulty::Classic, &|_| None, &mut |w| {
        // a well-run café: the last payment is safe
        if w.night() >= 28 {
            w.debug_set_money(w.money().max(60_000));
        }
    });
    assert_eq!(
        night_of(&log, "franchise_again"),
        None,
        "no reason to come back"
    );
    assert_eq!(night_of(&log, "franchise_last"), Some(30));
    assert_eq!(w.ending(), Some("light"));
}

/// Night 30 alone (the nights before skipped), for the variant thresholds.
fn last_night(c: &Content, setup: impl Fn(&mut World)) -> World {
    let mut w = World::new_story(c, Difficulty::Cozy);
    w.night = 30;
    w.begin_day(c);
    w.story.vars.insert("offer".into(), "pending".into());
    setup(&mut w);
    let mut b = bot::PlayerBot::new(bot::BotConfig {
        hire_nights: Vec::new(),
        seed: w.seed(),
        ..Default::default()
    });
    b.run_night(&mut w, c);
    w
}

#[test]
fn light_is_full_with_three_good_stories_and_reputation_fifty() {
    let c = content();
    let three = |w: &mut World| {
        for r in ["mirek", "vera", "radek"] {
            w.story.flags.insert(format!("arc.{r}.good"));
        }
    };
    let w = last_night(&c, |w| {
        three(w);
        w.econ.reputation = 55.0;
    });
    assert_eq!(w.ending(), Some("light"));
    assert_eq!(w.ending_variant(), Some("full"), "rep {}", w.reputation());
    let w = last_night(&c, |w| {
        three(w);
        w.econ.reputation = 20.0;
    });
    assert_eq!(w.ending_variant(), Some("quiet"));
    // no good stories, low reputation: on Cozy still the light (Karolina decides, not the books)
    let w = last_night(&c, |w| w.econ.reputation = 5.0);
    assert_eq!(w.ending(), Some("light"));
    assert_eq!(w.ending_variant(), Some("quiet"));
}

// ---------------------------------------------------------------------------------------------
// The letter in both endings
// ---------------------------------------------------------------------------------------------

#[test]
fn a_burned_letter_leaves_mirek_s_echo_in_the_ending() {
    let c = content();
    let (w, _) = play_story_with(
        &c,
        4242,
        Difficulty::Cozy,
        &|id| (id == "mirek_letter").then(|| vec![2]),
        &mut |_| {},
    );
    assert_eq!(letter(&w), "burned");
    let keys = visible(&w, &c, &ending_scene(&w));
    assert!(
        has(&keys, "call:letter_coda") && has(&keys, "scene.letter_coda.02"),
        "{keys:?}"
    );
    assert!(!has(&keys, "call:answer"));
}

#[test]
fn a_letter_left_with_mirek_gets_his_coda() {
    let c = content();
    let (w, _) = play_story_with(
        &c,
        4242,
        Difficulty::Cozy,
        &|id| (id == "mirek_letter").then(|| vec![1]),
        &mut |_| {},
    );
    assert_eq!(letter(&w), "with_mirek");
    let keys = visible(&w, &c, &ending_scene(&w));
    assert!(has(&keys, "scene.letter_coda.04"), "{keys:?}");
}

#[test]
fn a_letter_kept_in_the_pocket_never_finds_the_platform() {
    let c = content();
    let (w, _) = play_story_with(
        &c,
        4242,
        Difficulty::Cozy,
        &|id| match id {
            "mirek_letter" => Some(vec![0]),
            "last_333" => Some(vec![1]),
            _ => None,
        },
        &mut |_| {},
    );
    assert_eq!(letter(&w), "with_player");
    let keys = visible(&w, &c, &ending_scene(&w));
    assert!(has(&keys, "scene.letter_coda.06"), "{keys:?}");
}

#[test]
fn no_confession_means_karolinas_line_in_both_endings() {
    let c = content();
    // light: the letter stays hidden
    let w = last_night(&c, |_| {});
    assert_eq!(letter(&w), "hidden");
    let keys = visible(&w, &c, "ending_light");
    assert!(has(&keys, "scene.letter_coda.06"), "{keys:?}");
    // last train, signed on the last morning (no letter either)
    let w = last_night(&c, |w| {
        w.story.vars.insert("offer".into(), "accepted".into());
        w.story.flags.insert("signed_last_morning".into());
    });
    assert_eq!(w.ending(), Some("last_train"));
    let keys = visible(&w, &c, "ending_last_train");
    assert!(has(&keys, "scene.letter_coda.06"), "{keys:?}");
}

#[test]
fn the_bench_after_the_sale_shows_at_3_33_not_in_the_ending() {
    let c = content();
    let (w, _) = play_story_with(
        &c,
        77,
        Difficulty::Cozy,
        &|id| (id == "franchise_decision").then(|| vec![0]),
        &mut |_| {},
    );
    assert_eq!(letter(&w), "bench");
    let sold = visible(&w, &c, "last_333_sold");
    assert!(has(&sold, "scene.last_333_sold.03"));
    let keys = visible(&w, &c, "ending_last_train");
    for k in [
        "scene.letter_coda.02",
        "scene.letter_coda.04",
        "scene.letter_coda.06",
    ] {
        assert!(!has(&keys, k), "{k}: {keys:?}");
    }
}

#[test]
fn the_landlord_counts_the_messages_at_every_payment() {
    let c = content();
    let (_, log) = play_story_with(&c, 4242, Difficulty::Cozy, &|_| None, &mut |_| {});
    for p in ["payment_1", "payment_2", "payment_3", "payment_4"] {
        assert!(night_of(&log, p).is_some(), "{p}");
    }
    for (scene, key) in [
        ("payment_2", "scene.payment_2.04"),
        ("payment_3", "scene.payment_3.03"),
        ("payment_4", "scene.payment_4.03"),
    ] {
        let cs = c.cutscene(scene).unwrap();
        let mut keys = Vec::new();
        crate::content::story::walk_steps(&cs.steps, &mut |s| {
            if let CutsceneStep::Line { text_key, .. } = s {
                keys.push(text_key.clone());
            }
        });
        assert!(keys.contains(&key.to_string()), "{scene}");
    }
}

#[test]
fn old_saves_without_the_new_vars_still_end() {
    // a save from before karolina_way / signed_last_morning: offer already "rejected" on 22
    let c = content();
    let w = last_night(&c, |w| {
        w.story.vars.insert("offer".into(), "rejected".into());
        w.story.vars.remove("karolina_way");
    });
    assert!(!w.story().played.contains("franchise_last"));
    assert_eq!(w.ending(), Some("light"));
    let keys = visible(&w, &c, "ending_light");
    assert!(has(&keys, "scene.ending_light.16"), "the plain last line");
}

#[test]
fn skipping_every_choice_leaves_the_offer_open_and_keeps_the_light() {
    // no answer on night 22 nor at 05:30: the offer is never signed
    let c = content();
    let w = play_story(&c, 9, Difficulty::Cozy, &|_| Some(Vec::new()));
    assert_eq!(offer(&w), "pending");
    assert_eq!(w.ending(), Some("light"));
}
