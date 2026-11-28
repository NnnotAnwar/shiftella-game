//! T10 QA: every regular's story can be finished by an attentive player, both endings and the
//! optional "Answer" scene are reachable by playing (not by setting variables), and a hold-Esc
//! skipper doesn't lose the arcs. Nights are played by the player-bot (`sim::bot`), which
//! answers the story choices kindly and builds what the regulars ask for.

use super::*;
use crate::content::Content;
use crate::content::handson::Duty;

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

/// The cutscenes in the order they played, each with its night.
type SceneLog = Vec<(u32, String)>;

/// Plays a whole story with the attentive player-bot; `pick` may override a scene's choices.
fn play_story(
    c: &Content,
    seed: u64,
    difficulty: Difficulty,
    pick: &dyn Fn(&str) -> Option<Vec<usize>>,
) -> World {
    play_story_with(c, seed, difficulty, pick, &mut |_| {}).0
}

/// [`play_story`] with a hook run every day (after the bot's day) and the log of scenes.
fn play_story_with(
    c: &Content,
    seed: u64,
    difficulty: Difficulty,
    pick: &dyn Fn(&str) -> Option<Vec<usize>>,
    day: &mut dyn FnMut(&mut World),
) -> (World, SceneLog) {
    let mut log = SceneLog::new();
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
            day(&mut w);
            answer(&mut w, c, pick, &mut log);
            w.start_night(c);
        }
        let mut guard = 0;
        while w.phase() == Phase::Night && !w.is_closed() && guard < 100_000 {
            answer(&mut w, c, pick, &mut log);
            b.step(&mut w, c);
            w.tick(c);
            w.drain_events();
            guard += 1;
        }
        answer(&mut w, c, pick, &mut log);
    }
    (w, log)
}

fn answer(
    w: &mut World,
    c: &Content,
    pick: &dyn Fn(&str) -> Option<Vec<usize>>,
    log: &mut SceneLog,
) {
    for _ in 0..64 {
        let Some(id) = w.playing_cutscene().map(str::to_string) else {
            return;
        };
        log.push((w.night(), id.clone()));
        let choices = pick(&id).unwrap_or_else(|| bot::attentive_choices(w, c, &id));
        w.apply(c, Command::CutsceneFinished { id, choices })
            .unwrap();
    }
}

/// The night scene `id` played on.
fn night_of(log: &SceneLog, id: &str) -> Option<u32> {
    log.iter().find(|(_, x)| x == id).map(|(n, _)| *n)
}

fn letter(w: &World) -> &str {
    w.story()
        .vars
        .get("letter")
        .map_or("hidden", String::as_str)
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
    let (w, log) = play_story_with(&c, 4242, Difficulty::Cozy, &|_| None, &mut |_| {});
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
    // the reveal order: Vera's story is the setup, the franchise decision the temptation, then
    // Mirek's confession (the storm night at the earliest), then the letter
    let n = |id| night_of(&log, id).unwrap_or_else(|| panic!("{id} played"));
    assert!(
        n("vera_jindrich") < n("mirek_confession"),
        "Vera's story (night {}) before Mirek's confession (night {})",
        n("vera_jindrich"),
        n("mirek_confession")
    );
    assert!((19..=24).contains(&n("vera_jindrich")));
    assert!(n("franchise_decision") < n("mirek_confession"));
    assert!((25..=29).contains(&n("mirek_confession")));
    assert!(n("mirek_confession") < n("mirek_letter"));
    assert!((23..=25).contains(&n("radek_last_shift")));
    assert!(n("filip_cake") >= 26);
}

#[test]
fn saying_nothing_at_the_near_miss_still_leads_to_the_confession() {
    // "Say nothing" used to end Mirek's arc (and lose the letter) for good: now the accident
    // on the storm night makes Jana take his keys, and he confesses a few nights later
    let c = content();
    let (w, log) = play_story_with(
        &c,
        4242,
        Difficulty::Cozy,
        &|id| (id == "mirek_pedestrian").then(|| vec![2]),
        &mut |_| {},
    );
    assert!(w.flag("mirek_silence"));
    assert!(w.flag("mirek_jana_keys"), "the accident: Jana has his keys");
    let night = night_of(&log, "mirek_confession").expect("Mirek still confesses");
    assert!(night >= 26, "after the accident (night {night})");
    assert!(w.story().played.contains("mirek_letter"));
    assert!(
        !["hidden", "confessed"].contains(&letter(&w)),
        "the letter was decided: {}",
        letter(&w)
    );
    // the near-miss still costs him the good ending of his arc
    assert!(!w.flag("arc.mirek.good"));
}

#[test]
fn the_answer_plays_once_the_letter_is_delivered_even_with_few_notes() {
    // eight notes are a bonus inside the scene (the postscript), not a lock on it
    let c = content();
    let (w, _) = play_story_with(&c, 4242, Difficulty::Cozy, &|_| None, &mut |w| {
        if w.night() == 30 {
            w.story
                .notes
                .retain(|n| ["note_01", "note_04", "note_07"].contains(&n.as_str()));
        }
    });
    assert!(w.story().notes.len() < 8, "{} notes", w.story().notes.len());
    assert!(
        ["delivered", "delivered_by_mirek"].contains(&letter(&w)),
        "letter {}",
        letter(&w)
    );
    assert!(w.story().played.contains("answer"));
}

#[test]
fn lori_sent_away_gets_a_second_chance_in_the_storm() {
    let c = content();
    let (w, log) = play_story_with(
        &c,
        4242,
        Difficulty::Cozy,
        &|id| (id == "lori_platform").then(|| vec![1]),
        &mut |_| {},
    );
    assert_eq!(night_of(&log, "lori_storm"), Some(25));
    assert!(w.flag("lori_invited") && !w.flag("lori_sent_away"));
    assert!(w.flag("arc.lori.good"));
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
    // the 3:33 guest doesn't come in any more: the sold night shows him on the platform, and
    // Mirek's letter goes to the platform bench instead of "at 3:33"
    assert!(w.story().played.contains("last_333_sold"));
    assert!(!w.story().played.contains("last_333"));
    assert!(w.story().played.contains("mirek_letter"));
    assert_eq!(letter(&w), "bench");
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

// ---------------------------------------------------------------------------------------------
// The Shiftella app as a tool: every @shiftella_bot button
// ---------------------------------------------------------------------------------------------

/// Night 12 with three helpers (one late, one off tonight), a drone dock and a regular due.
fn busy_night(c: &Content) -> World {
    let mut w = World::new_story_seeded(c, Difficulty::Cozy, 31);
    w.sys.tutorial_off = true;
    w.night = 12;
    w.begin_day(c);
    bot::finish_cutscenes(&mut w, c);
    w.debug_set_money(40_000);
    for _ in 0..3 {
        if let Some(id) = w.candidates().first().map(|x| x.id) {
            w.apply(c, Command::Hire { candidate: id }).unwrap();
        }
    }
    let placed = (2..14).any(|y| {
        (2..28).any(|x| {
            w.apply(
                c,
                Command::PlaceFurniture {
                    def: "drone_dock".into(),
                    x,
                    y,
                    rot: 0,
                },
            )
            .is_ok()
        })
    });
    assert!(placed, "a drone dock fits");
    w.start_night(c);
    bot::finish_cutscenes(&mut w, c);
    w.drain_events();
    let helpers: Vec<usize> = (0..w.staff.len()).filter(|&i| !w.staff[i].owner).collect();
    assert!(helpers.len() >= 3, "three helpers");
    w.staff[helpers[0]].status = StaffStatus::Expected { minute: 90 };
    w.staff[helpers[0]].duties = vec![Duty::Dishes];
    w.staff[helpers[1]].status = StaffStatus::OffDuty;
    w.staff[helpers[2]].status = StaffStatus::OnDuty;
    w.staff[helpers[2]].duties.retain(|d| *d != Duty::Dishes);
    w
}

fn bot_args(w: &World) -> Vec<(String, String)> {
    let helpers: Vec<&Staff> = w.staff.iter().filter(|s| !s.owner).collect();
    [
        ("name", helpers[0].name.clone()),
        ("other", helpers[2].name.clone()),
        ("shift", "Fri".into()),
        ("dates", "14. 11.".into()),
        ("hours", "8".into()),
        ("minutes", "15".into()),
        ("item", "cappuccino".into()),
        ("n", "3".into()),
        ("time", "01:00".into()),
        ("train", "Rn 333".into()),
        ("regular", "vera".into()),
        ("minute", "150".into()),
        ("amount", "15000".into()),
        ("night", "14".into()),
        ("week", "2".into()),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect()
}

/// The world without its inbox and pending events (what a button changed besides closing
/// its own message).
fn fingerprint(w: &World) -> String {
    let mut x = w.clone();
    x.app.inbox.clear();
    x.events.clear();
    x.save_to_string().unwrap()
}

#[test]
fn every_bot_button_acts_or_says_why_and_stays_open() {
    let c = content();
    let mut checked = 0;
    for t in &c.app.bot {
        for a in &t.actions {
            // approve / decline answer a linked request: `life_events_become_requests…`
            if matches!(a.id.as_str(), "approve" | "decline") {
                continue;
            }
            let mut w = busy_night(&c);
            let args = bot_args(&w);
            app::bot_with(&mut w, &c, &t.id, &EffectCtx::default(), args);
            let msg = w.app.inbox.last().unwrap().id;
            w.drain_events();
            let before = fingerprint(&w);
            let res = w.apply(
                &c,
                Command::BotAction {
                    msg,
                    action: a.id.clone(),
                },
            );
            let events = w.drain_events();
            let answered = w
                .app
                .inbox
                .iter()
                .find(|m| m.id == msg)
                .unwrap()
                .answered
                .clone();
            let what = format!("{}.{}", t.id, a.id);
            match res {
                Ok(()) => {
                    assert_eq!(answered.as_deref(), Some(a.id.as_str()), "{what}");
                    // "Noted" only closes the message; everything else does something
                    if a.id != "noted" {
                        assert!(
                            fingerprint(&w) != before || !events.is_empty(),
                            "{what}: pressed, nothing happened"
                        );
                    }
                }
                Err(e) => panic!("{what}: refused on a night where it can act: {e:?}"),
            }
            checked += 1;
        }
    }
    assert!(checked >= 15, "{checked} buttons");
}

#[test]
fn bot_buttons_that_cant_act_say_why_and_the_message_stays_open() {
    let c = content();
    let mut w = World::new_story_seeded(&c, Difficulty::Cozy, 5);
    w.sys.tutorial_off = true;
    bot::finish_cutscenes(&mut w, &c);
    w.start_night(&c);
    bot::finish_cutscenes(&mut w, &c);
    // night 1: no dock for a drone, nobody to call in
    for (tpl, action, args) in [
        ("low_stock", "drone", vec![("item", "milk"), ("n", "2")]),
        ("sick_call", "call_in", vec![("name", "Nobody")]),
    ] {
        let args = args
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        app::bot_with(&mut w, &c, tpl, &EffectCtx::default(), args);
        let msg = w.app.inbox.last().unwrap().id;
        let r = w.apply(
            &c,
            Command::BotAction {
                msg,
                action: action.into(),
            },
        );
        assert!(r.is_err(), "{tpl}.{action}");
        assert!(w.app.inbox.last().unwrap().answered.is_none());
    }
}

#[test]
fn a_reserved_seat_waits_for_its_regular_and_a_day_reminder_rings_at_night() {
    let c = content();
    let mut w = World::new_story_seeded(&c, Difficulty::Cozy, 12);
    w.sys.tutorial_off = true;
    w.night = 9;
    w.begin_day(&c);
    bot::finish_cutscenes(&mut w, &c);
    // the day's message about a regular: reserve their seat (Radek's is "to go": remind me)
    let msg = w
        .app
        .inbox
        .iter()
        .rev()
        .find(|m| m.template == "regular_tonight" || m.template == "regular_to_go")
        .cloned()
        .expect("a regular's message by day");
    let action = if msg.template == "regular_tonight" {
        "reserve"
    } else {
        "remind"
    };
    w.apply(
        &c,
        Command::BotAction {
            msg: msg.id,
            action: action.into(),
        },
    )
    .unwrap();
    let regular = msg
        .args
        .iter()
        .find(|a| a.0 == "regular")
        .map(|a| a.1.clone())
        .unwrap();
    let seat = w.reserved_seats().get(&regular).copied();
    w.start_night(&c);
    let mut reminded = false;
    let mut sat_there = false;
    for _ in 0..NIGHT_TICKS {
        bot::finish_cutscenes(&mut w, &c);
        w.tick(&c);
        for e in w.drain_events() {
            reminded |= matches!(e, SimEvent::Reminder { .. });
        }
        if let Some(uid) = seat {
            sat_there |= w
                .guests
                .iter()
                .any(|g| g.source == regular && g.seat.is_some_and(|(u, _)| u == uid));
        }
        if w.phase() != Phase::Night {
            break;
        }
    }
    if action == "reserve" {
        assert!(seat.is_some(), "a seat is reserved");
        assert!(sat_there, "{regular} sat at the reserved seat");
    } else {
        assert!(reminded, "the reminder rang");
    }
    assert!(
        w.reserved_seats().is_empty(),
        "reservations end with the night"
    );
}
