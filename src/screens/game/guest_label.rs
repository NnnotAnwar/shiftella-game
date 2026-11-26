//! How guests are named on screen (design/v0.2.md §E): regulars by name; strangers by a short
//! description ("Woman in a flat cap") until Karolina gets to know them (a greeting, serving
//! them by hand), then by the name they told her. Also the look that makes the description
//! true, the introduction bubble and the night-log line. The sim side is `sim::names`.

use macroquad::prelude::*;

use super::world_view::sim;
use super::{Bubble, GameScreen};
use crate::render::{Accessory, FacialHair, HairStyle, PersonLook, TopStyle};
use crate::settings::Locale;
use crate::ui::{Icon, TextStyle, Ui, Weight};
use sim::{AgentRef, Extra, Garment, SimEvent, World};

/// Width of a ticket-rail label (`ui::handson::order` chip width − 4) at the smallest size
/// `Fonts::draw_in` shrinks to (9 pt): a short form wider than this is not used.
pub const RAIL_LABEL_W: f32 = 56.0;
const RAIL_LABEL_PT: f32 = 9.0;

/// The guest's label: a regular's name, a known stranger's name, or the description.
pub fn label(w: &World, loc: &Locale, guest: u32) -> String {
    let party = w.party_of_guest(guest);
    if let Some(r) = party.and_then(|p| p.regular.as_ref()) {
        return loc.tr_owned(&format!("regular.{r}.name"));
    }
    match w.persona(guest) {
        Some(p) if p.known => p.name_in(loc.lang.code()).to_string(),
        Some(p) => loc.tr_owned(&p.desc_key()),
        None => {
            let arch = party.map(|p| p.archetype.as_str()).unwrap_or_default();
            super::world_view::guest_one(loc, arch)
        }
    }
}

/// [`label`] with the rest of a stranger's party for its leader: "Tereza + 2".
pub fn party_label(w: &World, loc: &Locale, guest: u32) -> String {
    let base = label(w, loc, guest);
    let Some(p) = w.party_of_guest(guest) else {
        return base;
    };
    if p.regular.is_some() || p.leader() != Some(guest) {
        return base;
    }
    let others = p
        .members
        .iter()
        .filter(|m| **m != guest && w.guests().iter().any(|g| g.id == **m && !g.gone))
        .count();
    if others == 0 {
        base
    } else {
        loc.trf(
            "stranger.party",
            &[("name", &base), ("n", &others.to_string())],
        )
    }
}

/// The 1–2 word form for a ticket chip: the name once known, else the description's short
/// form — or the plain visual one ("V kabátě") when that doesn't fit the chip.
pub fn rail_label(ui: &Ui, w: &World, loc: &Locale, guest: u32) -> String {
    let Some(p) = w.persona(guest).filter(|p| !p.known) else {
        return label(w, loc, guest);
    };
    let style = TextStyle::new(RAIL_LABEL_PT, Weight::Medium, WHITE);
    let mut shortest: Option<(f32, String)> = None;
    for key in [p.short_key(), p.alt_short_key()] {
        let s = loc.tr_owned(&key);
        let wd = ui.fonts.measure(&s, &style);
        if wd <= RAIL_LABEL_W {
            return s;
        }
        if shortest.as_ref().is_none_or(|(b, _)| wd < *b) {
            shortest = Some((wd, s));
        }
    }
    shortest.map(|s| s.1).unwrap_or_default()
}

/// A description in the middle of a sentence (the log line): lower-case first letter.
fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_lowercase().chain(c).collect(),
        None => String::new(),
    }
}

// ───────────────────────── the look ─────────────────────────

const HAIR_NATURAL: [u32; 7] = [
    0x2a1f22, 0x3e2a24, 0x5b3b2a, 0x8a5a36, 0xc79e68, 0xa4462f, 0x4a3530,
];
const HAIR_GREY: [u32; 4] = [0x8d8899, 0xc9c4cf, 0xe4e0e8, 0x6e6773];

fn top_style(g: Garment) -> TopStyle {
    match g {
        Garment::Tee => TopStyle::Tee,
        Garment::Sweater => TopStyle::Sweater,
        Garment::Hoodie => TopStyle::Hoodie,
        Garment::Shirt => TopStyle::Shirt,
        Garment::Cardigan => TopStyle::Cardigan,
        Garment::Jacket => TopStyle::Jacket,
        Garment::Coat => TopStyle::Coat,
        Garment::Scrubs => TopStyle::Scrubs,
    }
}

fn accessory(e: Extra) -> Accessory {
    match e {
        Extra::Nothing => Accessory::None,
        Extra::Glasses => Accessory::Glasses,
        Extra::Scarf => Accessory::Scarf,
        Extra::Beanie => Accessory::Beanie,
        Extra::Headphones => Accessory::Headphones,
        Extra::Earrings => Accessory::Earrings,
        Extra::FlatCap => Accessory::FlatCap,
        Extra::Hat => Accessory::Hat,
        Extra::Hairclip => Accessory::Hairclip,
    }
}

/// A stranger's look: `PersonLook::random(seed)` dressed so that their description is true
/// (presentation → hair and beard, elderly → grey hair, the top and the extra).
pub fn stranger_look(seed: u64, archetype: &str) -> PersonLook {
    let mut l = PersonLook::random(seed, false);
    let s = sim::stranger_look(seed, archetype);
    let b = s.bits;
    l.top_style = top_style(s.top);
    l.accessory = accessory(s.extra);
    if s.elder != l.elder {
        l.hair = if s.elder {
            Color::from_hex(HAIR_GREY[(b % 4) as usize])
        } else {
            Color::from_hex(HAIR_NATURAL[(b % 7) as usize])
        };
        l.height += if s.elder { -0.03 } else { 0.03 };
        l.elder = s.elder;
    }
    let styles: &[HairStyle] = match (s.female, s.elder) {
        (true, false) => &[
            HairStyle::Bob,
            HairStyle::Long,
            HairStyle::Ponytail,
            HairStyle::Bun,
            HairStyle::Curly,
        ],
        (true, true) => &[HairStyle::Bun, HairStyle::Bob, HairStyle::Curly],
        (false, false) => &[
            HairStyle::Buzz,
            HairStyle::Short,
            HairStyle::Side,
            HairStyle::Curly,
        ],
        (false, true) => &[HairStyle::Short, HairStyle::Bald, HairStyle::Side],
    };
    if !styles.contains(&l.hair_style) {
        l.hair_style = styles[((b >> 8) % styles.len() as u64) as usize];
    }
    if s.female {
        l.facial_hair = FacialHair::None;
    }
    l
}

// ───────────────────────── events ─────────────────────────

impl GameScreen {
    /// `SimEvent::GuestIntroduced`: the line over their head ("I'm Tereza, thank you!") — after
    /// a reaction shown a moment ago — and a line in tonight's log.
    pub(super) fn names_event(&mut self, e: &SimEvent, loc: Option<&Locale>) {
        let SimEvent::GuestIntroduced { guest, key } = e else {
            return;
        };
        let (Some(loc), Some(p)) = (loc, self.world.persona(*guest).cloned()) else {
            return;
        };
        let name = p.name_in(loc.lang.code()).to_string();
        let line = loc.trf(key, &[("name", &name)]);
        let agent = AgentRef::Guest(*guest);
        let pinned = self.bubble.as_ref().is_some_and(|b| b.pinned);
        if !pinned {
            let text = match &self.bubble {
                Some(b) if b.agent == agent && b.until - self.time > 4.0 => {
                    format!("{} {line}", super::resolve_text(loc, &b.text))
                }
                _ => line,
            };
            self.bubble = Some(Bubble {
                agent,
                text,
                talk: None,
                pinned: false,
                until: self.time + 6.0,
            });
        }
        let desc = lower_first(loc.tr(&p.desc_key()));
        let log = loc.trf("stranger.log.met", &[("name", &name), ("desc", &desc)]);
        self.push_ticker(Icon::User, log);
    }
}

impl GameScreen {
    /// Strangers inside whom Karolina doesn't know yet (leaders first).
    pub fn unknown_strangers(&self) -> Vec<u32> {
        let w = &self.world;
        let mut out: Vec<(bool, u32)> = w
            .guests()
            .iter()
            .filter(|g| !g.gone && g.persona.as_ref().is_some_and(|p| !p.known))
            .filter(|g| {
                w.party_of_guest(g.id)
                    .is_some_and(|p| p.state != sim::PartyState::Leaving)
            })
            .map(|g| (!g.leader, g.id))
            .collect();
        out.sort();
        out.into_iter().map(|x| x.1).collect()
    }

    /// `SHIFTELLA_SCENE=guest_names`: the order card of a stranger she doesn't know yet (when
    /// one waits for it) and a pinned bubble over another one (verification shots).
    pub fn show_strangers(&mut self, content: &crate::content::Content, loc: &Locale) {
        use sim::PartyState as S;
        let unknown = self.unknown_strangers();
        let waiting = unknown.iter().copied().find(|g| {
            self.world.party_of_guest(*g).is_some_and(|p| {
                p.leader() == Some(*g) && matches!(p.state, S::Queue | S::WaitOrder | S::WaitFood)
            })
        });
        // else a stranger she already knows (the card then shows "Tereza + 2")
        let waiting = waiting.or_else(|| {
            self.world
                .parties()
                .iter()
                .filter(|p| p.regular.is_none() && matches!(p.state, S::WaitOrder | S::WaitFood))
                .filter_map(|p| p.leader())
                .find(|g| self.world.persona(*g).is_some())
        });
        if self.hands.order.is_none()
            && let Some(g) = waiting
        {
            self.open_card(content, g);
        }
        let card = self.hands.order.as_ref().map(|o| o.guest);
        if let Some(g) = unknown.iter().find(|g| Some(**g) != card) {
            let text = super::hud::guest_line(self, content, *g, loc);
            self.bubble = Some(Bubble {
                agent: AgentRef::Guest(*g),
                text,
                talk: None,
                pinned: true,
                until: self.time + 600.0,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_drawn_look_matches_the_description() {
        let women = [
            HairStyle::Bob,
            HairStyle::Long,
            HairStyle::Ponytail,
            HairStyle::Bun,
            HairStyle::Curly,
        ];
        for arch in [
            "travelers",
            "nurses",
            "students",
            "insomniacs",
            "truckers",
            "?",
        ] {
            for seed in 0..400u64 {
                let seed = seed.wrapping_mul(0x9E37_79B9) ^ 5;
                let s = sim::stranger_look(seed, arch);
                let l = stranger_look(seed, arch);
                assert_eq!(l.top_style, top_style(s.top));
                assert_eq!(l.accessory, accessory(s.extra));
                assert_eq!(l.elder, s.elder);
                if s.female {
                    assert_eq!(l.facial_hair, FacialHair::None);
                    assert!(women.contains(&l.hair_style), "{arch} {seed}");
                } else {
                    assert!(!matches!(
                        l.hair_style,
                        HairStyle::Long | HairStyle::Ponytail | HairStyle::Bun | HairStyle::Bob
                    ));
                }
                // deterministic
                assert_eq!(l, stranger_look(seed, arch));
            }
        }
    }

    #[test]
    fn lower_first_handles_every_script() {
        assert_eq!(lower_first("Žena v brýlích"), "žena v brýlích");
        assert_eq!(lower_first("Женщина в очках"), "женщина в очках");
        assert_eq!(lower_first(""), "");
    }
}
