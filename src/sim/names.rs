//! Strangers: a look, a description, then a name (design/v0.2.md §E, owner's «option C»).
//!
//! Every non-regular guest gets a [`Persona`] at spawn: a presentation (woman / man), maybe
//! elderly, a top and an extra they wear, a first name (Latin for cs/en, Cyrillic for uk/ru) and
//! a whole-phrase description (`stranger.<phrase>.<f|m>`). Until Karolina gets to know them
//! (a greeting or serving them by hand) they are shown by the description; then they say their
//! name (`SimEvent::GuestIntroduced`, `stranger.intro.<greet|serve>.<n>`) and are shown by it.
//!
//! Determinism without touching the balance: everything is derived from the guest's look seed
//! (already drawn from the world RNG at spawn) with a separate local SplitMix stream — no draws
//! from the world RNG. The renderer derives the same [`StrangerLook`] from the same seed, so
//! "a woman in a flat cap" is drawn as one. Data: `assets/data/guest_names.ron` (compiled in).

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use super::rng::Rng;
use super::types::SimEvent;
use super::world::World;

const DATA_SRC: &str = include_str!("../../assets/data/guest_names.ron");

/// Lines a guest says when they introduce themselves (`stranger.intro.greet.1..=N`).
pub const GREET_LINES: u64 = 3;
/// … and after Karolina served them (`stranger.intro.serve.1..=N`).
pub const SERVE_LINES: u64 = 4;

/// What a stranger wears on top (mirrors `render::TopStyle`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Garment {
    Tee,
    Sweater,
    Hoodie,
    Shirt,
    Cardigan,
    Jacket,
    Coat,
    Scrubs,
}

impl Garment {
    pub const ALL: [Garment; 8] = [
        Garment::Tee,
        Garment::Sweater,
        Garment::Hoodie,
        Garment::Shirt,
        Garment::Cardigan,
        Garment::Jacket,
        Garment::Coat,
        Garment::Scrubs,
    ];
    pub fn id(self) -> &'static str {
        match self {
            Garment::Tee => "tee",
            Garment::Sweater => "sweater",
            Garment::Hoodie => "hoodie",
            Garment::Shirt => "shirt",
            Garment::Cardigan => "cardigan",
            Garment::Jacket => "jacket",
            Garment::Coat => "coat",
            Garment::Scrubs => "scrubs",
        }
    }
}

/// The visible extra (mirrors `render::Accessory`; `Nothing` = none).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Extra {
    Nothing,
    Glasses,
    Scarf,
    Beanie,
    Headphones,
    Earrings,
    FlatCap,
    Hat,
    Hairclip,
}

impl Extra {
    pub const ALL: [Extra; 9] = [
        Extra::Nothing,
        Extra::Glasses,
        Extra::Scarf,
        Extra::Beanie,
        Extra::Headphones,
        Extra::Earrings,
        Extra::FlatCap,
        Extra::Hat,
        Extra::Hairclip,
    ];
    pub fn id(self) -> &'static str {
        match self {
            Extra::Nothing => "nothing",
            Extra::Glasses => "glasses",
            Extra::Scarf => "scarf",
            Extra::Beanie => "beanie",
            Extra::Headphones => "headphones",
            Extra::Earrings => "earrings",
            Extra::FlatCap => "flat_cap",
            Extra::Hat => "hat",
            Extra::Hairclip => "hairclip",
        }
    }
    /// Drawn on women only (the look generator never puts them on a man).
    pub fn women_only(self) -> bool {
        matches!(self, Extra::Earrings | Extra::Hairclip)
    }
    /// What elderly guests wear (calm extras).
    pub fn elder_ok(self) -> bool {
        matches!(
            self,
            Extra::Nothing | Extra::Glasses | Extra::Scarf | Extra::Hat | Extra::FlatCap
        )
    }
}

/// A feature a phrase needs to be true of the look.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Feature {
    Top(Garment),
    Extra(Extra),
}

/// What the renderer must draw so the description is true.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrangerLook {
    pub female: bool,
    pub elder: bool,
    pub top: Garment,
    pub extra: Extra,
    /// Spare random bits for the renderer's own small choices (hair style).
    pub bits: u64,
}

/// A stranger's identity (stored on `Guest`; `None` for regulars and story guests).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Persona {
    pub female: bool,
    #[serde(default)]
    pub elder: bool,
    /// First name, Latin (cs, en).
    pub name: String,
    /// Cyrillic (uk).
    #[serde(default)]
    pub name_uk: String,
    /// Cyrillic (ru).
    #[serde(default)]
    pub name_ru: String,
    /// Phrase id of the description (`stranger.<desc>.<f|m>`).
    pub desc: String,
    /// The plain visual phrase (extra or top) — the short label when the phrase's is too long.
    #[serde(default)]
    pub alt: String,
    /// Karolina knows their name (greeted / served by her).
    #[serde(default)]
    pub known: bool,
}

impl Persona {
    fn sex(&self) -> &'static str {
        if self.female { "f" } else { "m" }
    }
    /// `stranger.<desc>.<f|m>` — the full description ("Woman in a flat cap").
    pub fn desc_key(&self) -> String {
        format!("stranger.{}.{}", self.desc, self.sex())
    }
    /// The 1–2 word form for the ticket rail.
    pub fn short_key(&self) -> String {
        format!("stranger.{}.{}.short", self.desc, self.sex())
    }
    /// The visual short form (fallback when `short_key` doesn't fit).
    pub fn alt_short_key(&self) -> String {
        let alt = if self.alt.is_empty() {
            &self.desc
        } else {
            &self.alt
        };
        format!("stranger.{alt}.{}.short", self.sex())
    }
    /// The first name in a language (`cs`, `en`, `uk`, `ru`).
    pub fn name_in(&self, lang: &str) -> &str {
        let cyr = match lang {
            "uk" => &self.name_uk,
            "ru" if !self.name_ru.is_empty() => &self.name_ru,
            "ru" => &self.name_uk,
            _ => &self.name,
        };
        if cyr.is_empty() { &self.name } else { cyr }
    }
}

/// How Karolina got to know someone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro {
    Greet,
    Serve,
}

impl Intro {
    pub fn id(self) -> &'static str {
        match self {
            Intro::Greet => "greet",
            Intro::Serve => "serve",
        }
    }
    pub fn lines(self) -> u64 {
        match self {
            Intro::Greet => GREET_LINES,
            Intro::Serve => SERVE_LINES,
        }
    }
}

// ───────────────────────── data ─────────────────────────

#[derive(Debug, Deserialize)]
struct NameData {
    female: Vec<Vec<String>>,
    male: Vec<Vec<String>>,
    styles: Vec<Style>,
}

#[derive(Debug, Deserialize)]
struct Style {
    id: String,
    female: f32,
    elder: f32,
    tops: Vec<(Garment, u32)>,
    extras: Vec<(Extra, u32)>,
    phrases: Vec<Phrase>,
}

#[derive(Debug, Deserialize)]
struct Phrase {
    id: String,
    #[serde(default)]
    needs: Option<Feature>,
}

fn data() -> &'static NameData {
    static DATA: OnceLock<NameData> = OnceLock::new();
    DATA.get_or_init(|| ron::from_str(DATA_SRC).expect("assets/data/guest_names.ron"))
}

fn style(archetype: &str) -> &'static Style {
    let d = data();
    d.styles
        .iter()
        .find(|s| s.id == archetype)
        .or_else(|| d.styles.iter().find(|s| s.id == "default"))
        .unwrap_or(&d.styles[0])
}

fn weighted<T: Copy>(r: &mut Rng, items: &[(T, u32)], ok: impl Fn(T) -> bool) -> Option<T> {
    let w: Vec<f32> = items
        .iter()
        .map(|(t, w)| if ok(*t) { *w as f32 } else { 0.0 })
        .collect();
    r.weighted(&w).map(|i| items[i].0)
}

/// The local stream for a guest (never the world RNG).
fn stream(seed: u64, salt: u64) -> Rng {
    Rng::new(seed ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x6e61_6d65_735f_7631)
}

/// What a stranger of `archetype` with this look seed wears (pure; the renderer calls it too).
pub fn stranger_look(seed: u64, archetype: &str) -> StrangerLook {
    let s = style(archetype);
    let mut r = stream(seed, 1);
    let female = r.chance(s.female);
    let elder = r.chance(s.elder);
    let top = weighted(&mut r, &s.tops, |t| {
        !elder || !matches!(t, Garment::Hoodie | Garment::Tee)
    })
    .unwrap_or(if elder {
        Garment::Cardigan
    } else {
        Garment::Jacket
    });
    let extra = weighted(&mut r, &s.extras, |e| {
        (female || !e.women_only()) && (!elder || e.elder_ok())
    })
    .unwrap_or(Extra::Nothing);
    StrangerLook {
        female,
        elder,
        top,
        extra,
        bits: r.next_u64(),
    }
}

/// The visual phrase id for a look (`x_<extra>` or `t_<top>`).
fn visual_phrase(l: &StrangerLook) -> String {
    if l.extra != Extra::Nothing {
        format!("x_{}", l.extra.id())
    } else {
        format!("t_{}", l.top.id())
    }
}

/// Every phrase id a look can be described with (weights): archetype phrases (those needing a
/// feature the guest really has weigh more), the generic visual ones, the elderly ones.
fn phrase_candidates(l: &StrangerLook, archetype: &str) -> Vec<(String, f32)> {
    if l.elder {
        let id = match l.extra {
            Extra::Glasses => "elder_glasses",
            Extra::Scarf => "elder_scarf",
            Extra::Hat => "elder_hat",
            Extra::FlatCap => "elder_cap",
            _ => "elder",
        };
        return vec![(id.to_string(), 1.0)];
    }
    let mut out = Vec::new();
    for p in &style(archetype).phrases {
        let w = match p.needs {
            None => 1.0,
            Some(Feature::Top(t)) if t == l.top => 3.0,
            Some(Feature::Extra(e)) if e == l.extra => 3.0,
            Some(_) => continue,
        };
        out.push((p.id.clone(), w));
    }
    if l.extra != Extra::Nothing {
        out.push((format!("x_{}", l.extra.id()), 2.0));
    }
    if l.top != Garment::Scrubs {
        out.push((format!("t_{}", l.top.id()), 1.0));
    }
    out
}

fn mix(seed: u64, salt: u64) -> u64 {
    stream(seed, salt).next_u64()
}

/// A new stranger's persona. Names already used by tonight's other guests are skipped while
/// the pool allows.
pub(crate) fn new_persona(w: &World, seed: u64, archetype: &str) -> Persona {
    let look = stranger_look(seed, archetype);
    let cands = phrase_candidates(&look, archetype);
    let mut r = stream(seed, 2);
    let weights: Vec<f32> = cands.iter().map(|c| c.1).collect();
    let desc = r
        .weighted(&weights)
        .map(|i| cands[i].0.clone())
        .unwrap_or_else(|| visual_phrase(&look));
    let d = data();
    let pool = if look.female { &d.female } else { &d.male };
    let start = r.index(pool.len());
    let taken = |n: &str| {
        w.guests
            .iter()
            .filter_map(|g| g.persona.as_ref())
            .any(|p| p.name == n)
    };
    let pick = (0..pool.len())
        .map(|k| &pool[(start + k) % pool.len()])
        .find(|n| !taken(&n[0]))
        .unwrap_or(&pool[start]);
    let lat = pick[0].clone();
    let uk = pick.get(1).cloned().unwrap_or_else(|| lat.clone());
    let ru = pick.get(2).cloned().unwrap_or_else(|| uk.clone());
    Persona {
        female: look.female,
        elder: look.elder,
        name: lat,
        name_uk: uk,
        name_ru: ru,
        desc,
        alt: if look.elder {
            "elder".into()
        } else {
            visual_phrase(&look)
        },
        known: false,
    }
}

/// A guest loaded from a save without a persona gets one (regulars / story guests don't).
pub(crate) fn ensure_all(w: &mut World) {
    for i in 0..w.guests.len() {
        let g = &w.guests[i];
        if g.persona.is_some() || g.preset.is_some() {
            continue;
        }
        let (seed, source) = (g.look_seed, g.source.clone());
        let regular = w
            .parties
            .iter()
            .find(|p| p.id == g.party)
            .is_some_and(|p| p.regular.is_some());
        if regular {
            continue;
        }
        let p = new_persona(w, seed, &source);
        w.guests[i].persona = Some(p);
    }
}

/// The event that makes a guest known (greeting, a serve by Karolina's hand).
pub(crate) fn introduced_by(e: &SimEvent) -> Option<(u32, Intro)> {
    match e {
        SimEvent::PlayerGreeted { guest } => Some((*guest, Intro::Greet)),
        SimEvent::PlayerServed { guest, .. } => Some((*guest, Intro::Serve)),
        _ => None,
    }
}

/// They say their name (once): `known` + `SimEvent::GuestIntroduced`.
pub(crate) fn introduce(w: &mut World, guest: u32, how: Intro) {
    let Some(g) = w.guests.iter_mut().find(|g| g.id == guest) else {
        return;
    };
    let seed = g.look_seed;
    let Some(p) = g.persona.as_mut() else {
        return;
    };
    if p.known {
        return;
    }
    p.known = true;
    let n = mix(seed, 3 + how as u64) % how.lines() + 1;
    w.events.push(SimEvent::GuestIntroduced {
        guest,
        key: format!("stranger.intro.{}.{n}", how.id()),
    });
}

impl World {
    /// A stranger's persona (`None` for regulars, story guests and unknown ids).
    pub fn persona(&self, guest: u32) -> Option<&Persona> {
        self.guests
            .iter()
            .find(|g| g.id == guest)
            .and_then(|g| g.persona.as_ref())
    }
}

/// Every phrase id with the presentations it can be used for — for tests and validation.
#[cfg(test)]
pub(crate) fn phrase_ids() -> Vec<(String, bool, bool)> {
    let mut out: Vec<(String, bool, bool)> = Vec::new();
    let mut add = |id: String, f: bool, m: bool| {
        if let Some(e) = out.iter_mut().find(|e| e.0 == id) {
            e.1 |= f;
            e.2 |= m;
        } else {
            out.push((id, f, m));
        }
    };
    for e in Extra::ALL.iter().filter(|e| **e != Extra::Nothing) {
        add(format!("x_{}", e.id()), true, !e.women_only());
    }
    for t in Garment::ALL {
        add(format!("t_{}", t.id()), true, true);
    }
    for id in [
        "elder",
        "elder_glasses",
        "elder_scarf",
        "elder_hat",
        "elder_cap",
    ] {
        add(id.into(), true, true);
    }
    for s in &data().styles {
        for p in &s.phrases {
            let women_only = matches!(p.needs, Some(Feature::Extra(e)) if e.women_only());
            add(p.id.clone(), true, !women_only);
        }
    }
    out
}

/// The archetype ids that have a style (tests).
#[cfg(test)]
pub(crate) fn style_ids() -> Vec<&'static str> {
    data().styles.iter().map(|s| s.id.as_str()).collect()
}

/// Every phrase a look of this archetype could get (tests).
#[cfg(test)]
pub(crate) fn phrases_for(seed: u64, archetype: &str) -> Vec<String> {
    phrase_candidates(&stranger_look(seed, archetype), archetype)
        .into_iter()
        .map(|c| c.0)
        .collect()
}
