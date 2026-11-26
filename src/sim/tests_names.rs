//! Strangers' descriptions and names (design/v0.2.md §E, `sim::names`): determinism without
//! world-RNG draws, phrases for every archetype × presentation in all four languages, the
//! name revealed by a greeting / a serve by hand (never by a helper), regulars untouched,
//! unique names tonight, saves (round trip, old saves get personas on load).

use std::collections::{BTreeMap, BTreeSet};

use super::guests;
use super::names::{self, Extra, Garment};
use super::*;
use crate::content::Content;
use crate::i18n::{I18n, Lang};

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");
const I18N: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/i18n");

fn content() -> Content {
    Content::load(DATA).unwrap_or_else(|e| panic!("{e}"))
}

fn night(c: &Content, n: u32) -> World {
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
    w.mods.push(Modifier {
        kind: ModKind::GuestRate,
        mul: 0.0,
        until: None,
        event: None,
    });
    w.drain_events();
    w
}

fn spawn(w: &mut World, c: &Content, arch: &str, size: u32, wish: bool) -> u32 {
    let pid =
        guests::spawn_party(w, c, arch, size, None, 1.0, 1.0, &[], None, wish).expect("spawned");
    if !wish && let Some(p) = w.parties.iter_mut().find(|p| p.id == pid) {
        p.order_kind = OrderKind::Plain;
        p.wish_open = false;
    }
    pid
}

fn leader(w: &World, pid: u32) -> u32 {
    w.parties()
        .iter()
        .find(|p| p.id == pid)
        .and_then(|p| p.leader())
        .expect("leader")
}

fn tick_until(
    w: &mut World,
    c: &Content,
    max: u32,
    f: impl Fn(&SimEvent) -> bool,
) -> Vec<SimEvent> {
    let mut all = Vec::new();
    for _ in 0..max {
        w.tick(c);
        bot::finish_cutscenes(w, c);
        let ev = w.drain_events();
        let hit = ev.iter().any(&f);
        all.extend(ev);
        if hit {
            break;
        }
    }
    all
}

#[test]
fn personas_come_from_the_look_seed_not_the_world_rng() {
    let c = content();
    let mut a = night(&c, 2);
    let mut b = night(&c, 2);
    for arch in ["travelers", "nurses", "students", "taxi_drivers"] {
        spawn(&mut a, &c, arch, 3, false);
        spawn(&mut b, &c, arch, 3, false);
    }
    let pa: Vec<_> = a.guests().iter().map(|g| g.persona.clone()).collect();
    let pb: Vec<_> = b.guests().iter().map(|g| g.persona.clone()).collect();
    assert_eq!(pa, pb, "same seed, same strangers");
    assert!(pa.iter().all(|p| p.is_some()));
    // naming never draws from the world RNG: the stream is where it would be without it
    let before = a.rng.clone();
    let seed = a.guests()[0].look_seed;
    let _ = names::new_persona(&a, seed, "travelers");
    assert_eq!(a.rng, before);
    assert_eq!(a.save_to_string().unwrap(), b.save_to_string().unwrap());
}

#[test]
fn every_archetype_and_presentation_has_phrases_in_all_languages() {
    let c = content();
    let i18n = I18n::load(I18N).unwrap_or_else(|e| panic!("{e}"));
    let ids = names::phrase_ids();
    let reach: BTreeMap<String, (bool, bool)> = ids
        .iter()
        .map(|(id, f, m)| (id.clone(), (*f, *m)))
        .collect();
    // every reachable (phrase, presentation): description + short form, four languages
    for (id, f, m) in &ids {
        for (sex, on) in [("f", *f), ("m", *m)] {
            if !on {
                continue;
            }
            for key in [
                format!("stranger.{id}.{sex}"),
                format!("stranger.{id}.{sex}.short"),
            ] {
                for lang in Lang::ALL {
                    let t = i18n.get(lang, &key).unwrap_or_default();
                    assert!(!t.trim().is_empty(), "{key} missing in {lang:?}");
                    assert!(!t.contains('{'), "{key}: no placeholders in a label");
                }
            }
        }
    }
    let styles: BTreeSet<&str> = names::style_ids().into_iter().collect();
    for g in &c.guests {
        assert!(styles.contains(g.id.as_str()), "no style for {}", g.id);
        let mut seen: BTreeSet<(bool, String)> = BTreeSet::new();
        for seed in 0..3000u64 {
            let s = seed.wrapping_mul(0x2545_F491_4F6C_DD1D) ^ 77;
            let look = names::stranger_look(s, &g.id);
            for p in names::phrases_for(s, &g.id) {
                let (f, m) = reach
                    .get(&p)
                    .copied()
                    .unwrap_or_else(|| panic!("{p} unknown"));
                assert!(
                    if look.female { f } else { m },
                    "{p} for the wrong presentation"
                );
                seen.insert((look.female, p));
            }
        }
        for female in [true, false] {
            assert!(
                seen.iter().any(|(f, _)| *f == female),
                "{}: nobody of one presentation",
                g.id
            );
        }
        // the archetype's own phrases do show up
        assert!(
            seen.iter()
                .any(|(_, p)| !p.starts_with("x_") && !p.starts_with("t_")),
            "{}: only generic phrases",
            g.id
        );
    }
    for key in ["stranger.party", "stranger.log.met"] {
        assert!(i18n.has_all(key), "{key}");
    }
    for (kind, n) in [("greet", names::GREET_LINES), ("serve", names::SERVE_LINES)] {
        for i in 1..=n {
            let key = format!("stranger.intro.{kind}.{i}");
            assert!(i18n.has_all(&key), "{key}");
            for lang in Lang::ALL {
                assert!(i18n.tr(lang, &key).contains("{name}"), "{key} {lang:?}");
            }
        }
    }
}

#[test]
fn looks_are_true_to_the_description() {
    for arch in names::style_ids() {
        for seed in 0..2000u64 {
            let l = names::stranger_look(seed * 7919 + 3, arch);
            if !l.female {
                assert!(!l.extra.women_only(), "{arch}: earrings on a man");
            }
            if l.elder {
                assert!(l.extra.elder_ok());
                assert!(!matches!(l.top, Garment::Hoodie | Garment::Tee));
            }
            if l.top == Garment::Scrubs {
                assert_eq!(arch, "nurses", "only nurses wear scrubs");
            }
            for p in names::phrases_for(seed * 7919 + 3, arch) {
                if let Some(x) = p.strip_prefix("x_") {
                    assert_eq!(x, l.extra.id());
                    assert_ne!(l.extra, Extra::Nothing);
                }
                if let Some(t) = p.strip_prefix("t_") {
                    assert_eq!(t, l.top.id());
                }
                if p.starts_with("elder") {
                    assert!(l.elder);
                }
            }
        }
    }
}

#[test]
fn a_greeting_reveals_the_name_once() {
    let c = content();
    let mut w = night(&c, 2);
    let pid = spawn(&mut w, &c, "travelers", 2, false);
    let g = leader(&w, pid);
    let p = w.persona(g).expect("a stranger").clone();
    assert!(!p.known && !p.name.is_empty() && !p.name_uk.is_empty());
    assert!(p.desc_key().starts_with("stranger."));
    w.apply(&c, Command::PlayerGreet { guest: g }).unwrap();
    let ev = w.drain_events();
    let intro: Vec<_> = ev
        .iter()
        .filter_map(|e| match e {
            SimEvent::GuestIntroduced { guest, key } => Some((*guest, key.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(intro.len(), 1);
    assert_eq!(intro[0].0, g);
    assert!(intro[0].1.starts_with("stranger.intro.greet."));
    assert!(w.persona(g).unwrap().known);
    // the rest of the party is still unknown
    let other = w.parties().iter().find(|p| p.id == pid).unwrap().members[1];
    assert!(!w.persona(other).unwrap().known);
    assert!(w.apply(&c, Command::PlayerGreet { guest: g }).is_err());
    assert!(
        !w.drain_events()
            .iter()
            .any(|e| matches!(e, SimEvent::GuestIntroduced { .. }))
    );
}

#[test]
fn karolina_serving_by_hand_reveals_the_name() {
    let c = content();
    let mut w = night(&c, 1);
    let pid = spawn(&mut w, &c, "nurses", 1, true);
    let g = leader(&w, pid);
    assert!(!w.persona(g).unwrap().known);
    w.apply(&c, Command::PlayerTakeOrder { guest: g }).unwrap();
    tick_until(&mut w, &c, 900, |e| {
        matches!(e, SimEvent::PlayerOrderTaken { .. })
    });
    let (recipe, _) = w.best_matches(&c, g)[0].clone();
    w.apply(
        &c,
        Command::PlayerBrew {
            guest: g,
            recipe,
            quality: 0.9,
            mode: BrewMode::ByHand,
        },
    )
    .unwrap();
    let ev = tick_until(&mut w, &c, 1500, |e| {
        matches!(e, SimEvent::PlayerServed { .. })
    });
    let served = ev
        .iter()
        .position(|e| matches!(e, SimEvent::PlayerServed { .. }))
        .expect("served");
    let intro = ev
        .iter()
        .position(|e| {
            matches!(e, SimEvent::GuestIntroduced { guest, key }
            if *guest == g && key.starts_with("stranger.intro.serve."))
        })
        .expect("introduced");
    assert!(intro > served, "they say their name after the cup");
    assert!(w.persona(g).unwrap().known);
}

#[test]
fn regulars_keep_their_names_and_helpers_dont_introduce() {
    let c = content();
    let mut w = night(&c, 2);
    let pid = guests::spawn_party(
        &mut w,
        &c,
        "insomniacs",
        1,
        Some("mirek"),
        1.0,
        1.0,
        &[],
        None,
        false,
    )
    .expect("mirek");
    let g = leader(&w, pid);
    assert!(w.persona(g).is_none(), "regulars have no persona");
    // a plain party left to the helpers: served, still strangers
    let pid = spawn(&mut w, &c, "taxi_drivers", 1, false);
    let g = leader(&w, pid);
    let mut ev = w.run_night_headless(&c);
    ev.extend(w.drain_events());
    assert!(
        !ev.iter()
            .any(|e| matches!(e, SimEvent::GuestIntroduced { guest, .. } if *guest == g))
    );
}

#[test]
fn names_are_unique_tonight_while_the_pool_lasts() {
    let c = content();
    let mut w = night(&c, 2);
    for _ in 0..8 {
        spawn(&mut w, &c, "students", 2, false);
    }
    let mut seen = BTreeSet::new();
    for g in w.guests() {
        let p = g.persona.as_ref().unwrap();
        assert!(seen.insert(p.name.clone()), "{} twice", p.name);
    }
}

#[test]
fn saves_keep_personas_and_old_saves_get_them() {
    let c = content();
    let mut w = night(&c, 2);
    let pid = spawn(&mut w, &c, "clubbers", 3, false);
    let g = leader(&w, pid);
    w.apply(&c, Command::PlayerGreet { guest: g }).unwrap();
    let text = w.save_to_string().unwrap();
    let back = World::load_from_str(&text).unwrap();
    assert_eq!(back.save_to_string().unwrap(), text);
    assert!(back.persona(g).unwrap().known);
    // a 0.1.x save has no `persona` on its guests: they get one on load (not yet known)
    let mut old = w.clone();
    for g in old.guests.iter_mut() {
        g.persona = None;
    }
    let loaded = World::load_from_str(&old.save_to_string().unwrap()).unwrap();
    for (a, b) in loaded.guests().iter().zip(w.guests()) {
        let (pa, pb) = (a.persona.as_ref().unwrap(), b.persona.as_ref().unwrap());
        assert_eq!((pa.female, &pa.desc), (pb.female, &pb.desc));
        assert!(!pa.known);
    }
    // the v1 fixtures still load
    for f in ["save_v1_sandbox_night.ron", "save_v1_story_day3.ron"] {
        let path = format!("{}/src/sim/testdata/{f}", env!("CARGO_MANIFEST_DIR"));
        let w = World::load_from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        for g in w.guests() {
            assert_eq!(g.persona.is_some(), g.preset.is_none(), "{f}");
        }
    }
}
