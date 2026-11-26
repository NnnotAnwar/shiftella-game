//! T10 QA: every regular's story can be finished by an attentive player, both endings and the
//! optional "Answer" scene are reachable by playing (not by setting variables), and a hold-Esc
//! skipper doesn't lose the arcs. Nights are played by the player-bot (`sim::bot`), which
//! answers the story choices kindly and builds what the regulars ask for.

use super::*;
use crate::content::Content;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

const STORY_REGULARS: [&str; 7] = [
    "mirek",
    "hana",
    "vera",
    "filip",
    "radek",
    "lori",
    "guest_333",
];

fn content() -> Content {
    Content::load(DATA).unwrap_or_else(|e| panic!("{e}"))
}

/// Plays a whole story with the attentive player-bot; `pick` may override a scene's choices.
fn play_story(
    c: &Content,
    seed: u64,
    difficulty: Difficulty,
    pick: &dyn Fn(&str) -> Option<Vec<usize>>,
) -> World {
    let mut w = World::new_story_seeded(c, difficulty, seed);
    let mut b = bot::PlayerBot::new(bot::BotConfig {
        seed,
        ..Default::default()
    });
    for _ in 1..=30 {
        if w.is_closed() {
            break;
        }
        if w.phase() == Phase::Day {
            b.day(&mut w, c);
            answer(&mut w, c, pick);
            w.start_night(c);
        }
        let mut guard = 0;
        while w.phase() == Phase::Night && !w.is_closed() && guard < 100_000 {
            answer(&mut w, c, pick);
            b.step(&mut w, c);
            w.tick(c);
            w.drain_events();
            guard += 1;
        }
        answer(&mut w, c, pick);
    }
    w
}

fn answer(w: &mut World, c: &Content, pick: &dyn Fn(&str) -> Option<Vec<usize>>) {
    for _ in 0..64 {
        let Some(id) = w.playing_cutscene().map(str::to_string) else {
            return;
        };
        let choices = pick(&id).unwrap_or_else(|| bot::attentive_choices(w, c, &id));
        w.apply(c, Command::CutsceneFinished { id, choices })
            .unwrap();
    }
}

fn arc(w: &World, id: &str) -> i64 {
    w.story()
        .counters
        .get(&format!("arc.{id}"))
        .copied()
        .unwrap_or(0) as i64
}

#[test]
fn an_attentive_cozy_story_finishes_every_arc_and_hears_the_answer() {
    let c = content();
    let w = play_story(&c, 4242, Difficulty::Cozy, &|_| None);
    for r in STORY_REGULARS {
        let last = c
            .regular(r)
            .unwrap()
            .stages
            .iter()
            .map(|s| s.stage)
            .max()
            .unwrap();
        assert_eq!(arc(&w, r), last as i64, "{r}: arc {} of {last}", arc(&w, r));
        assert!(w.flag(&format!("arc.{r}.good")), "{r}: no good ending");
    }
    assert!(w.flag("filip_talked") || w.flag("filip_free_coffee"));
    assert!(w.flag("lori_invited") && w.flag("hana_stays"));
    assert_eq!(w.ending(), Some("light"));
    assert_eq!(w.ending_variant(), Some("full"));
    assert!(
        w.story().played.contains("answer"),
        "the 3:33 guest's answer plays (letter {:?}, notes {})",
        w.story().vars.get("letter"),
        w.story().notes.len()
    );
}

#[test]
fn accepting_the_franchise_offer_ends_on_the_last_train() {
    let c = content();
    let w = play_story(&c, 77, Difficulty::Cozy, &|id| {
        (id == "franchise_decision").then(|| vec![0])
    });
    assert_eq!(
        w.ending(),
        Some("last_train"),
        "offer {:?}, franchise scene played {}",
        w.story().vars.get("offer"),
        w.story().played.contains("franchise_decision")
    );
    assert_eq!(w.ending_variant(), Some("sold"));
    assert!(!w.story().played.contains("answer"), "no answer once sold");
}

#[test]
fn skipping_every_scene_keeps_the_arcs_that_need_no_choice() {
    // a player who skips every scene and never answers a choice still meets the regulars;
    // the arcs that hang on a choice (Lori's invitation, Filip's exam) wait, nothing breaks
    let c = content();
    let w = play_story(&c, 9, Difficulty::Cozy, &|_| Some(Vec::new()));
    for r in ["vera", "radek"] {
        assert!(w.flag(&format!("arc.{r}.good")), "{r}");
    }
    for r in ["mirek", "hana", "guest_333"] {
        assert!(arc(&w, r) >= 3, "{r}: arc {}", arc(&w, r));
    }
    assert!(arc(&w, "filip") >= 2 && arc(&w, "lori") == 0);
    assert!(
        !w.story().played.contains("answer"),
        "the answer needs the letter"
    );
    assert_eq!(w.ending(), Some("light"));
}
