//! Little stories of strangers and the storm night (design/v0.2.md H, `sim::stories`,
//! `sim::storm`): the data and every text in four languages, the director's limits (a
//! budget, one at a time, never twice in a story, strangers only, no world-RNG draws),
//! answering by hand (a reaction, the name, a polaroid), fading, the storm's closed line, the
//! stranded staying till morning, blankets and soup, the 03:00 talk, saves.

use std::collections::BTreeSet;

use super::guests;
use super::*;
use crate::content::Content;
use crate::i18n::{I18n, Lang};

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");
const I18N: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/i18n");

fn content() -> Content {
    Content::load(DATA).unwrap_or_else(|e| panic!("{e}"))
}

/// Story night `n`, the doors just open; `quiet` = no random arrivals.
fn night(c: &Content, n: u32, quiet: bool) -> World {
    let mut w = World::new_story(c, Difficulty::Cozy);
    w.sys.tutorial_off = true;
    if n > 1 {
        w.night = n;
        w.begin_day(c);
    }
    bot::finish_cutscenes(&mut w, c);
    w.start_night(c);
    bot::finish_cutscenes(&mut w, c);
    w.skip_prep(c);
    if quiet {
        w.mods.push(Modifier {
            kind: ModKind::GuestRate,
            mul: 0.0,
            until: None,
            event: None,
        });
    }
    w.drain_events();
    w
}

fn ticks(w: &mut World, c: &Content, n: u32) -> Vec<SimEvent> {
    let mut out = Vec::new();
    for _ in 0..n {
        bot::finish_cutscenes(w, c);
        w.tick(c);
        out.extend(w.drain_events());
    }
    out
}

fn to_minute(w: &mut World, c: &Content, m: u32) -> Vec<SimEvent> {
    let mut out = Vec::new();
    let mut guard = 0;
    while w.phase() == Phase::Night && w.minute() < m && guard < NIGHT_TICKS {
        out.extend(ticks(w, c, 1));
        guard += 1;
    }
    out
}

/// A seated stranger party (`size`), ready for a story: (party, leader).
fn seated(w: &mut World, c: &Content, arch: &str, size: u32) -> (u32, u32) {
    let pid =
        guests::spawn_party(w, c, arch, size, None, 4.0, 1.0, &[], None, true).expect("spawned");
    for _ in 0..600 {
        let sat = w
            .parties
            .iter()
            .find(|p| p.id == pid)
            .is_some_and(|p| p.state == PartyState::WaitOrder)
            && w.guests
                .iter()
                .filter(|g| g.party == pid)
                .all(|g| g.seat.is_some() && g.pose == Pose::Sit);
        if sat {
            break;
        }
        ticks(w, c, 1);
    }
    let leader = w.parties.iter().find(|p| p.id == pid).unwrap().members[0];
    (pid, leader)
}

#[test]
fn the_data_parses_and_every_text_has_all_four_languages() {
    let d = stories::parse(include_str!("../../assets/data/stories.ron")).unwrap();
    assert!(d.stories.len() >= 20, "{} vignettes", d.stories.len());
    let ids: BTreeSet<&str> = d.stories.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids.len(), d.stories.len(), "unique ids");
    let c = content();
    let archetypes: Vec<&str> = c.guests.iter().map(|g| g.id.as_str()).collect();
    let i18n = I18n::load(I18N).unwrap_or_else(|e| panic!("{e}"));
    let mut keys: Vec<String> = vec!["story.toast".into()];
    for s in &d.stories {
        assert!((2..=3).contains(&s.answers.len()), "{}: 2–3 answers", s.id);
        assert!(s.answers.iter().any(|a| a.good), "{}: a good answer", s.id);
        let a: BTreeSet<&str> = s.answers.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(a.len(), s.answers.len(), "{}: unique answers", s.id);
        for arch in &s.archetypes {
            assert!(archetypes.contains(&arch.as_str()), "{}: {arch}", s.id);
        }
        assert!(s.party.0 >= 1 && s.party.0 <= s.party.1, "{}", s.id);
        keys.push(s.text_key());
        keys.push(s.log_key());
        keys.push(s.caption_key());
        for ans in &s.answers {
            keys.push(ans.label_key(&s.id));
            keys.push(ans.react_key(&s.id));
        }
    }
    let st = &d.storm;
    assert!(st.story_nights.iter().all(|n| (25..30).contains(n)));
    for n in &st.story_nights {
        // never on a payment night (nor the last one, which is one)
        assert!(c.story.payments.iter().all(|p| p.night != *n), "night {n}");
    }
    for k in [
        "storm.forecast.title",
        "storm.forecast.text",
        "storm.cancel.title",
        "storm.cancel.text",
        "storm.cancel.stay",
        "app.bot.storm_cancel",
        "app.bot.storm_cancel.action.noted",
        "storm.chip.title",
        "storm.chip.stranded.one",
        "storm.chip.stranded.few",
        "storm.chip.stranded.many",
        "storm.act.blankets",
        "storm.act.soup",
        "storm.act.blankets.tip",
        "storm.act.soup.tip",
        "storm.act.on_way",
        "storm.done.blankets",
        "storm.done.soup",
        "storm.cared.blankets",
        "storm.cared.soup",
        "storm.morning.title",
        "storm.morning.cared",
        "storm.morning.plain",
        "journal.caption.storm",
        "storm.talk.intro",
    ] {
        keys.push(k.into());
    }
    assert!(c.app.bot_template("storm_cancel").is_some());
    for t in &st.talks {
        assert!(t.min >= st.talk_min);
        let need = t.lines.iter().filter_map(|l| l.who).max().unwrap_or(0) + 1;
        assert!(need <= t.min, "{}: lines for {need}, needs {}", t.id, t.min);
        keys.extend(t.lines.iter().map(|l| l.key.clone()));
    }
    for lang in Lang::ALL {
        for k in &keys {
            assert!(i18n.has(lang, k), "{k} missing in {lang:?}");
        }
    }
    // {name} in the log lines; the noticing text is about what Karolina sees
    for s in &d.stories {
        for lang in Lang::ALL {
            assert!(i18n.tr(lang, &s.log_key()).contains("{name}"), "{}", s.id);
            assert!(!i18n.tr(lang, &s.text_key()).contains('{'), "{}", s.id);
        }
    }
}

#[test]
fn stories_come_to_strangers_one_at_a_time_and_never_twice() {
    let c = content();
    let mut w = World::new_story(&c, Difficulty::Cozy);
    let mut b = bot::PlayerBot::new(bot::BotConfig {
        hire_nights: vec![3, 5, 9],
        ..Default::default()
    });
    let mut told: Vec<String> = Vec::new();
    let mut total = 0;
    for _ in 0..12 {
        let n = w.night();
        let ev = b.run_night(&mut w, &c);
        let mut open = 0i32;
        let mut tonight = 0;
        for e in &ev {
            match e {
                SimEvent::StoryNoticed { story, .. } => {
                    open += 1;
                    tonight += 1;
                    assert!(open <= 1, "night {n}: one at a time");
                    assert!(!told.contains(story), "night {n}: {story} again");
                    told.push(story.clone());
                }
                SimEvent::StoryFaded { .. } | SimEvent::StoryAnswered { .. } => open -= 1,
                _ => {}
            }
        }
        assert!(tonight <= 2, "night {n}: {tonight} stories");
        if n == 1 {
            assert_eq!(tonight, 0, "night 1 belongs to the tutorial");
        }
        total += tonight;
    }
    assert!(total >= 5, "{total} stories in 11 nights");
    assert!(total <= 16, "{total}: the pool has to last the whole story");
    assert_eq!(w.stories().told.len(), told.len());
}

#[test]
fn noticing_and_fading_never_touch_the_world_rng() {
    let c = content();
    let mut a = night(&c, 6, true);
    let (_, g) = seated(&mut a, &c, "travelers", 1);
    let mut b = a.clone();
    assert!(b.debug_story("call", g));
    let ea = ticks(&mut a, &c, 600);
    let eb = ticks(&mut b, &c, 600);
    assert_eq!(a.rng, b.rng, "the same draws");
    assert!(eb.iter().any(|e| matches!(e, SimEvent::StoryFaded { .. })) || b.story_of(g).is_some());
    let arrivals = |ev: &[SimEvent]| {
        ev.iter()
            .filter(|e| matches!(e, SimEvent::GuestArrived { .. }))
            .count()
    };
    assert_eq!(arrivals(&ea), arrivals(&eb));
}

#[test]
fn a_good_answer_warms_the_guest_tells_the_name_and_makes_a_polaroid() {
    let c = content();
    let mut w = night(&c, 6, true);
    let (pid, g) = seated(&mut w, &c, "travelers", 1);
    assert!(w.debug_story("call", g));
    assert!(w.story_of(g).is_some());
    let before = w
        .parties()
        .iter()
        .find(|p| p.id == pid)
        .unwrap()
        .satisfaction;
    assert!(!w.persona(g).unwrap().known);
    // a wrong answer id, then the real one; a second click while she walks is refused
    assert!(matches!(
        w.apply(
            &c,
            Command::AnswerStory {
                guest: g,
                answer: "nope".into()
            }
        ),
        Err(CommandError::UnknownId(_))
    ));
    w.apply(
        &c,
        Command::AnswerStory {
            guest: g,
            answer: "charger".into(),
        },
    )
    .unwrap();
    assert!(
        w.apply(
            &c,
            Command::AnswerStory {
                guest: g,
                answer: "tea".into()
            }
        )
        .is_err()
    );
    let ev = ticks(&mut w, &c, 900);
    let answered = ev.iter().find_map(|e| match e {
        SimEvent::StoryAnswered {
            guest,
            answer,
            good,
            ..
        } if *guest == g => Some((answer.clone(), *good)),
        _ => None,
    });
    assert_eq!(answered, Some(("charger".into(), true)));
    assert!(w.story_of(g).is_none());
    let after = w
        .parties()
        .iter()
        .find(|p| p.id == pid)
        .unwrap()
        .satisfaction;
    assert!(after > before, "{before} → {after}");
    assert!(w.persona(g).unwrap().known, "they tell her their name");
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::GuestIntroduced { guest, .. } if *guest == g))
    );
    let h = w
        .systems()
        .journal
        .tonight
        .iter()
        .find(|h| h.caption_key == "journal.caption.story.call")
        .expect("a polaroid candidate");
    assert!(
        h.args
            .iter()
            .any(|(k, v)| k == "name" && v.starts_with("guest.") && v.contains(".role."))
    );
    assert_eq!(
        w.stories().answered,
        vec![("call".into(), "charger".into(), true)]
    );
}

#[test]
fn a_kind_but_unfitting_answer_is_fine_and_an_ignored_story_fades() {
    let c = content();
    let mut w = night(&c, 6, true);
    let (_, g) = seated(&mut w, &c, "students", 1);
    assert!(w.debug_story("exam", g));
    w.apply(
        &c,
        Command::AnswerStory {
            guest: g,
            answer: "luck".into(),
        },
    )
    .unwrap();
    let ev = ticks(&mut w, &c, 900);
    assert!(
        ev.iter().any(
            |e| matches!(e, SimEvent::StoryAnswered { good: false, guest, .. } if *guest == g)
        )
    );
    assert!(
        !w.systems()
            .journal
            .tonight
            .iter()
            .any(|h| h.caption_key.starts_with("journal.caption.story")),
        "no polaroid for a plain answer"
    );
    // another one, left alone: it fades quietly
    let (_, g2) = seated(&mut w, &c, "insomniacs", 1);
    assert!(w.debug_story("letter", g2));
    let minutes = stories::data().expires_minutes + 2;
    let ev = ticks(&mut w, &c, minutes * TICKS_PER_MINUTE);
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::StoryFaded { guest, .. } if *guest == g2))
    );
    assert!(w.stories_active().is_empty());
}

#[test]
fn regulars_never_carry_a_story() {
    let c = content();
    let mut b = bot::PlayerBot::new(bot::BotConfig::default());
    let mut w = night(&c, 6, false);
    let mut seen = 0;
    for _ in 0..3000 {
        b.step(&mut w, &c);
        ticks(&mut w, &c, 1);
        for a in w.stories_active() {
            let p = w.party_of_guest(a.guest).unwrap();
            assert!(p.regular.is_none());
            assert!(w.persona(a.guest).is_some());
            seen += 1;
        }
    }
    let _ = seen;
}

#[test]
fn the_storm_closes_the_line_and_the_strangers_stay_till_morning() {
    let c = content();
    let mut w = night(&c, 29, true);
    assert!(w.storm().tonight, "night 29 is the storm night");
    assert_eq!(w.weather(), crate::content::Weather::Snow);
    // strangers in the café before the line closes
    let (p1, _) = seated(&mut w, &c, "travelers", 2);
    let (p2, _) = seated(&mut w, &c, "students", 1);
    let (p3, _) = seated(&mut w, &c, "insomniacs", 1);
    let cancel = stories::data().storm.cancel_at.minutes_since_open() as u32;
    let ev = to_minute(&mut w, &c, cancel + 1);
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::StormCancelled { .. }))
    );
    assert!(
        ev.iter().any(
            |e| matches!(e, SimEvent::BotMessage { template, .. } if template == "storm_cancel")
        )
    );
    assert!(w.storm_line_closed());
    // blankets and soup by hand
    w.apply(&c, Command::StormBlankets).unwrap();
    assert!(
        w.apply(&c, Command::StormBlankets).is_err(),
        "on her way already"
    );
    let ev = ticks(&mut w, &c, 600);
    assert!(ev.iter().any(|e| matches!(
        e,
        SimEvent::StormCared {
            act: StormAct::Blankets,
            ..
        }
    )));
    w.apply(&c, Command::StormSoup).unwrap();
    let ev = ticks(&mut w, &c, 900);
    assert!(ev.iter().any(|e| matches!(
        e,
        SimEvent::StormCared {
            act: StormAct::Soup,
            ..
        }
    )));
    assert!(w.apply(&c, Command::StormBlankets).is_err(), "done already");
    assert!(!w.storm().blanketed.is_empty() && !w.storm().fed.is_empty());
    // the night goes on with arrivals back on: nobody new comes, no trains
    w.mods.retain(|m| m.kind != ModKind::GuestRate);
    let talk_at = stories::data().storm.talk_at.minutes_since_open() as u32;
    let ev = to_minute(&mut w, &c, talk_at + 1);
    let window = cancel + stories::data().storm.platform_minutes + 1;
    for e in &ev {
        assert!(!matches!(e, SimEvent::TrainPassing { .. }), "no trains");
        if let SimEvent::GuestArrived { party, .. } = e {
            let p = w.parties().iter().find(|p| p.id == *party);
            assert!(
                p.is_none_or(|p| p.regular.is_some() || p.arrived <= window),
                "a stranger walked in through the storm"
            );
        }
    }
    for pid in [p1, p2, p3] {
        assert!(
            w.parties()
                .iter()
                .any(|p| p.id == pid && p.state != PartyState::Leaving),
            "party {pid} stays"
        );
    }
    let (talk, speakers) = w.storm().talk.clone().expect("the 03:00 talk");
    assert!(!talk.is_empty(), "a talk for {speakers:?}");
    assert!(speakers.len() >= 2);
    for g in &speakers {
        assert!(w.persona(*g).unwrap().known, "names are known now");
    }
    assert!(
        w.systems()
            .journal
            .tonight
            .iter()
            .any(|h| h.caption_key == "journal.caption.storm"),
        "the storm polaroid"
    );
    // still here at 05:30, gone by the morning
    to_minute(&mut w, &c, 450);
    assert!(w.parties().iter().any(|p| p.id == p1));
    let mut out = Vec::new();
    while w.phase() == Phase::Night {
        out.extend(ticks(&mut w, &c, 1));
    }
    assert!(
        out.iter()
            .any(|e| matches!(e, SimEvent::StormMorning { cared: true }))
    );
    assert!(w.parties().is_empty());
}

#[test]
fn an_ordinary_night_has_no_storm_and_saves_keep_the_state() {
    let c = content();
    let mut w = night(&c, 12, true);
    assert!(!w.storm().tonight);
    assert!(!w.storm_line_closed());
    assert!(w.apply(&c, Command::StormBlankets).is_err());
    // a save round trip keeps the state; an old save without the field loads empty
    let (_, g) = seated(&mut w, &c, "travelers", 1);
    w.debug_story("call", g);
    let s = w.save_to_string().unwrap();
    let back = World::load_from_str(&s).unwrap();
    assert_eq!(back.stories(), w.stories());
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/sim/testdata/");
    for f in ["save_v1_story_day3.ron", "save_v1_sandbox_night.ron"] {
        let text = std::fs::read_to_string(format!("{dir}{f}")).unwrap();
        let old = World::load_from_str(&text).unwrap();
        assert!(old.stories().told.is_empty());
        assert!(!old.storm().tonight);
    }
}

#[test]
fn the_sandbox_storm_is_rare_and_needs_snow() {
    let c = content();
    let mut storms = 0;
    let mut snow = 0;
    for seed in 0..60u64 {
        let mut w = World::new_sandbox(
            &c,
            SandboxSetup {
                seed,
                difficulty: Difficulty::Cozy,
                ..Default::default()
            },
        );
        // a winter town (season 3 has snow in the weather table)
        w.town.season = 3;
        w.town.options.insert("rain_every_night".into(), false);
        w.night = 20;
        w.begin_day(&c);
        w.start_night(&c);
        if w.weather() == crate::content::Weather::Snow {
            snow += 1;
        }
        if w.storm().tonight {
            storms += 1;
        }
    }
    assert!(snow > 0);
    assert!(
        storms <= snow && storms * 2 <= snow.max(2),
        "{storms} of {snow}"
    );
}

/// The party of `g` gets up to go (what `guests` does when they are done).
fn make_leave(w: &mut World, g: u32) {
    let pid = w.guests.iter().find(|x| x.id == g).unwrap().party;
    w.parties.iter_mut().find(|p| p.id == pid).unwrap().state = PartyState::Leaving;
}

#[test]
fn a_story_closes_the_moment_its_guest_gets_up_to_leave() {
    let c = content();
    let mut w = night(&c, 6, true);
    let (_, g) = seated(&mut w, &c, "travelers", 1);
    assert!(w.debug_story("call", g));
    make_leave(&mut w, g);
    // the sim refuses an answer that raced the guest out of the door
    assert_eq!(
        w.apply(
            &c,
            Command::AnswerStory {
                guest: g,
                answer: "charger".into(),
            },
        ),
        Err(CommandError::Invalid(GUEST_LEFT.into()))
    );
    assert!(
        w.player().queue.is_empty(),
        "nothing queued for a guest on the way out"
    );
    // and the very next tick closes it (card and marker with it), not a minute later
    let ev = ticks(&mut w, &c, 1);
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::StoryFaded { guest, .. } if *guest == g))
    );
    assert!(w.story_of(g).is_none());
}

#[test]
fn a_guest_leaving_while_karolina_walks_over_calls_her_walk_off() {
    let c = content();
    let mut w = night(&c, 6, true);
    let (_, g) = seated(&mut w, &c, "travelers", 1);
    assert!(w.debug_story("call", g));
    w.apply(
        &c,
        Command::AnswerStory {
            guest: g,
            answer: "charger".into(),
        },
    )
    .unwrap();
    ticks(&mut w, &c, 2);
    assert!(
        w.player().current.is_some() || !w.player().queue.is_empty(),
        "she is on her way"
    );
    make_leave(&mut w, g);
    let ev = ticks(&mut w, &c, 600);
    assert!(
        !ev.iter()
            .any(|e| matches!(e, SimEvent::StoryAnswered { guest, .. } if *guest == g)),
        "no charger for someone who has gone"
    );
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::StoryFaded { guest, .. } if *guest == g))
    );
    assert!(w.story_of(g).is_none());
    assert!(
        !w.player()
            .queue
            .iter()
            .any(|a| matches!(a, PlayerAct::GoTo { .. })),
        "the walk went with them"
    );
}

#[test]
fn a_moment_about_a_guest_closes_when_they_leave_too() {
    let c = content();
    let mut w = night(&c, 6, true);
    let (_, g) = seated(&mut w, &c, "insomniacs", 1);
    let tile = w.guests.iter().find(|x| x.id == g).unwrap().tile();
    let m = w.minute();
    w.sys.moments.active.push(super::systems::ActiveMoment {
        id: "m_sleeping_guest".into(),
        anchor: super::systems::MomentPos {
            tile,
            furniture: None,
            agent: Some(AgentRef::Guest(g)),
        },
        spawned: m,
        expires: m + 30,
        target: Default::default(),
        resolving: None,
    });
    w.apply(
        &c,
        Command::ResolveMoment {
            id: "m_sleeping_guest".into(),
            option: "blanket".into(),
        },
    )
    .unwrap();
    make_leave(&mut w, g);
    let ev = ticks(&mut w, &c, 1);
    assert!(
        ev.iter()
            .any(|e| matches!(e, SimEvent::MomentExpired { id } if id == "m_sleeping_guest"))
    );
    assert!(w.active_moments().is_empty());
    assert!(
        w.player().current.is_none() && w.player().queue.is_empty(),
        "her walk over was called off"
    );
    let ev = ticks(&mut w, &c, 300);
    assert!(
        !ev.iter()
            .any(|e| matches!(e, SimEvent::MomentResolved { .. })),
        "no blanket for an empty chair"
    );
}
