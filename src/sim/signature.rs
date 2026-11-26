//! The café's own drink and the night's extra photos (design/v0.2.md I.5–I.6).
//!
//! * **The signature drink** — from night `from_night` (story; sandbox `sandbox_from_night`)
//!   Karolina composes the café's drink in the recipe book: a base (`coffee`, `tea`, `cocoa`,
//!   `linden`), 2–3 jars from the pantry and a mini-game style, then a name (≤ `name_max`
//!   characters). `Command::ComposeSignature`. It becomes the recipe [`SIGNATURE_RECIPE`]
//!   (a world-side [`RecipeDef`] riding on the base's menu item, at + `price_add` +
//!   `price_per_jar` × jars): its tastes come from the base, the jars and the style, so the
//!   wish matcher scores it like any recipe, and the `house_special` wishes (strangers and
//!   regulars, `wishes.ron`) ask for it by name once it exists (an exact favourite → match 1).
//!   It can be reworked / renamed once every `rework_every` nights; the night it was made (or
//!   last changed) it may still be tweaked for free.
//! * **Photo mode** — `Command::TakePhoto { focus, width }` at night: a [`SceneSnapData`] of
//!   the framed part of the café goes into the journal as an extra photo of that night
//!   (`photo.per_night` slots; never replaces the night's kept polaroid).
//!
//! Data: `assets/data/signature.ron` (compiled in). Nothing here touches the world RNG.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use super::grid::{self, TILE};
use super::systems::SceneSnapData;
use super::types::*;
use super::world::World;
use crate::content::Content;
use crate::content::handson::{Family, MiniGameKind, RecipeDef, RecipeUnlock, Taste, Tier};

/// The recipe id of the café's own drink (`recipe.r_signature.name` is overridden with the
/// player's name by the game's i18n layer).
pub use crate::content::handson::SIGNATURE_RECIPE;

/// The journal draws a snapshot this much closer than the data's `zoom`
/// (`screens::game::journal_ui::snap_of`); a photo's zoom is stored divided by it.
pub const SNAP_CLOSER: f32 = 1.55;

// ---------------------------------------------------------------------------------------------
// Data
// ---------------------------------------------------------------------------------------------

/// `assets/data/signature.ron`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignatureData {
    pub from_night: u32,
    pub sandbox_from_night: u32,
    pub rework_every: u32,
    /// How many jars besides the base (min, max).
    pub jars: (usize, usize),
    pub max_tags: usize,
    pub price_add: u32,
    pub price_per_jar: u32,
    pub cost_per_jar: u32,
    /// Characters, not bytes (Cyrillic and Czech letters count one each).
    pub name_max: usize,
    /// Number of `signature.suggest.<n>` name ideas (1-based).
    pub suggestions: u32,
    pub bases: Vec<BaseDef>,
    pub styles: Vec<StyleDef>,
    /// The jars allowed in a cup and the tastes they bring.
    pub ingredient_tags: Vec<(String, Vec<Taste>)>,
    /// Families a cup may hold at most one of (two milks never).
    #[serde(default)]
    pub one_of_family: Vec<Family>,
    pub photo: PhotoRules,
}

/// A base of the drink (`signature.base.<id>`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaseDef {
    pub id: String,
    /// The pantry jar it needs (and puts in the cup).
    pub ingredient: String,
    /// The menu item it rides on (station, prep, price, availability).
    pub base_item: String,
    /// Each counts twice.
    pub tags: Vec<Taste>,
    pub styles: Vec<MiniGameKind>,
}

impl BaseDef {
    pub fn name_key(&self) -> String {
        format!("signature.base.{}", self.id)
    }
}

/// A mini-game style (`signature.style.<kind id>`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StyleDef {
    pub kind: MiniGameKind,
    pub variant: String,
    #[serde(default)]
    pub tag: Option<Taste>,
}

/// Photo mode limits.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhotoRules {
    /// Extra photos a night.
    pub per_night: u32,
    /// How far the viewfinder zooms (× the café camera's zoom when it opened).
    pub zoom: (f32, f32),
    /// How far it pans from where it opened (tiles).
    pub pan_tiles: f32,
}

const DATA: &str = include_str!("../../assets/data/signature.ron");

/// The signature / photo data (parsed once; a broken file is a test failure, at runtime the
/// feature stays shut).
pub fn data() -> &'static SignatureData {
    static D: OnceLock<SignatureData> = OnceLock::new();
    D.get_or_init(|| {
        parse(DATA).unwrap_or_else(|e| {
            eprintln!("error: signature.ron: {e}");
            SignatureData {
                from_night: u32::MAX,
                sandbox_from_night: u32::MAX,
                rework_every: 7,
                jars: (2, 3),
                max_tags: 4,
                price_add: 0,
                price_per_jar: 0,
                cost_per_jar: 0,
                name_max: 24,
                suggestions: 0,
                bases: Vec::new(),
                styles: Vec::new(),
                ingredient_tags: Vec::new(),
                one_of_family: Vec::new(),
                photo: PhotoRules {
                    per_night: 0,
                    zoom: (1.0, 1.0),
                    pan_tiles: 0.0,
                },
            }
        })
    })
}

pub(crate) fn parse(text: &str) -> Result<SignatureData, String> {
    ron::from_str(text).map_err(|e| e.to_string())
}

impl SignatureData {
    pub fn base(&self, id: &str) -> Option<&BaseDef> {
        self.bases.iter().find(|b| b.id == id)
    }
    pub fn style(&self, kind: MiniGameKind) -> Option<&StyleDef> {
        self.styles.iter().find(|s| s.kind == kind)
    }
    pub fn jar_tags(&self, id: &str) -> Option<&[Taste]> {
        self.ingredient_tags
            .iter()
            .find(|(i, _)| i == id)
            .map(|(_, t)| t.as_slice())
    }
}

// ---------------------------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------------------------

/// The café's own drink.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignatureDrink {
    /// The player's name for it (trimmed, ≤ `name_max` characters).
    pub name: String,
    pub base: String,
    pub jars: Vec<String>,
    pub style: MiniGameKind,
    /// The night it was first made.
    pub made_night: u32,
    /// The night it was last changed (made, reworked or renamed).
    pub changed_night: u32,
    /// Reworks / renames after the first.
    pub changes: u32,
    /// The recipe the sim brews ([`SIGNATURE_RECIPE`]); built from the fields above.
    pub def: RecipeDef,
}

/// An extra photo of a night (photo mode).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NightPhoto {
    pub night: u32,
    /// Minutes since 22:00 (negative in the preparation half hour is stored as 0).
    pub minute: u32,
    /// Story calendar date, if any.
    pub date: Option<(i32, u32, u32)>,
    /// `photo.caption.<n>` (1-based).
    pub caption: u32,
    pub snap: SceneSnapData,
}

impl NightPhoto {
    pub fn caption_key(&self) -> String {
        format!("photo.caption.{}", self.caption)
    }
}

/// The by-name line of a wish that asks for the house drink (`{drink}` is filled in by the
/// game): `signature.wish.<regular>` for a personal one, `signature.wish.<id>` otherwise.
pub fn wish_line_key(wish: &str) -> String {
    match wish.split_once('.') {
        Some((regular, _)) => format!("signature.wish.{regular}"),
        None => format!("signature.wish.{wish}"),
    }
}

/// Number of `photo.caption.<n>` lines.
pub const PHOTO_CAPTIONS: u32 = 8;

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SigState {
    pub drink: Option<SignatureDrink>,
    /// Grandma's soft prompt was shown (once).
    pub invited: bool,
    /// Every extra photo, oldest first.
    pub photos: Vec<NightPhoto>,
    /// Times a guest got the house drink they asked for by name.
    pub asked_served: u32,
}

// ---------------------------------------------------------------------------------------------
// Composing
// ---------------------------------------------------------------------------------------------

/// Trims and tidies a name (single spaces, no control characters). `None` = not a name
/// (empty, no letter or digit, or too long).
pub fn clean_name(raw: &str) -> Option<String> {
    let s: String = raw
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .filter(|c| !c.is_control())
        .collect();
    let n = s.chars().count();
    if n == 0 || n > data().name_max || !s.chars().any(char::is_alphanumeric) {
        return None;
    }
    Some(s)
}

/// The tastes of a cup: base ×2, each jar, the style; the top `max_tags` (ties: `Taste::ALL`
/// order). A Cold jar makes the drink cold (no Warm).
pub fn tags_for(
    d: &SignatureData,
    base: &BaseDef,
    jars: &[String],
    style: MiniGameKind,
) -> Vec<Taste> {
    let mut score: BTreeMap<Taste, u32> = BTreeMap::new();
    for t in &base.tags {
        *score.entry(*t).or_default() += 2;
    }
    for j in jars {
        for t in d.jar_tags(j).unwrap_or(&[]) {
            *score.entry(*t).or_default() += 1;
        }
    }
    if let Some(t) = d.style(style).and_then(|s| s.tag) {
        *score.entry(t).or_default() += 1;
    }
    if score.contains_key(&Taste::Cold) {
        score.remove(&Taste::Warm);
    }
    let mut v: Vec<(Taste, u32)> = score.into_iter().collect();
    let order = |t: Taste| Taste::ALL.iter().position(|x| *x == t).unwrap_or(99);
    v.sort_by(|a, b| b.1.cmp(&a.1).then(order(a.0).cmp(&order(b.0))));
    v.truncate(d.max_tags.max(1));
    let mut tags: Vec<Taste> = v.into_iter().map(|x| x.0).collect();
    tags.sort_by_key(|t| order(*t));
    tags
}

/// The recipe for a plan (no knowledge checks).
pub fn build_def(base: &BaseDef, jars: &[String], style: MiniGameKind) -> RecipeDef {
    let d = data();
    let variant = d
        .style(style)
        .map(|s| s.variant.clone())
        .unwrap_or_default();
    let mut ingredients = vec![base.ingredient.clone()];
    ingredients.extend(jars.iter().cloned());
    RecipeDef {
        id: SIGNATURE_RECIPE.to_string(),
        base_item: base.base_item.clone(),
        price_add: d.price_add + d.price_per_jar * jars.len() as u32,
        cost_add: d.cost_per_jar * jars.len() as u32,
        tags: tags_for(d, base, jars, style),
        minigame: style,
        variants: vec![variant],
        fallback_minigame: None,
        fallback_variant: None,
        ingredients,
        unlock: RecipeUnlock::Experiment,
        experimentable: false,
        hint: None,
        gated_hint: None,
        revealed: Vec::new(),
        art_finish: style == MiniGameKind::MilkFroth && base.id != "tea",
        tier: Tier::Must,
    }
}

fn base_ready(w: &World, content: &Content, b: &BaseDef) -> bool {
    w.hands.knowledge.ingredients.contains(&b.ingredient)
        && content
            .menu_item(&b.base_item)
            .is_some_and(|m| super::guests::item_unlocked(w, m))
}

/// Checks a plan (base, jars, style) against the pantry and the rules.
pub(crate) fn check_plan(
    w: &World,
    content: &Content,
    base: &str,
    jars: &[String],
    style: MiniGameKind,
) -> Result<RecipeDef, CommandError> {
    let d = data();
    let b = d
        .base(base)
        .ok_or_else(|| CommandError::UnknownId(base.to_string()))?;
    if !base_ready(w, content, b) {
        return Err(CommandError::Locked);
    }
    if !b.styles.contains(&style) || d.style(style).is_none() {
        return Err(CommandError::Invalid(
            "that style doesn't suit the base".into(),
        ));
    }
    let mut set: Vec<&String> = jars.iter().collect();
    set.sort();
    set.dedup();
    if set.len() != jars.len() || !(d.jars.0..=d.jars.1).contains(&jars.len()) {
        return Err(CommandError::Invalid(format!(
            "pick {}–{} different jars",
            d.jars.0, d.jars.1
        )));
    }
    for j in jars {
        if *j == b.ingredient || d.jar_tags(j).is_none() {
            return Err(CommandError::Invalid(format!(
                "`{j}` doesn't go in this cup"
            )));
        }
        if content.handson.ingredient(j).is_none() {
            return Err(CommandError::UnknownId(j.clone()));
        }
        if !w.hands.knowledge.ingredients.contains(j) {
            return Err(CommandError::Locked);
        }
    }
    for fam in &d.one_of_family {
        let n = std::iter::once(&b.ingredient)
            .chain(jars.iter())
            .filter(|i| {
                content
                    .handson
                    .ingredient(i)
                    .is_some_and(|x| x.family == *fam)
            })
            .count();
        if n > 1 {
            return Err(CommandError::Invalid(
                "one of those is enough in a cup".into(),
            ));
        }
    }
    Ok(build_def(b, jars, style))
}

/// The night the drink can be changed again (`None` = now).
pub(crate) fn next_change(w: &World) -> Option<u32> {
    let drink = w.sig.drink.as_ref()?;
    if w.night == drink.changed_night {
        return None;
    }
    let at = drink.changed_night + data().rework_every;
    (w.night < at).then_some(at)
}

pub(crate) fn unlocked(w: &World) -> bool {
    let d = data();
    let from = if w.mode == Mode::Sandbox {
        d.sandbox_from_night
    } else {
        d.from_night
    };
    w.night >= from
}

/// `Command::ComposeSignature`.
pub(crate) fn compose(
    w: &mut World,
    content: &Content,
    base: &str,
    jars: &[String],
    style: MiniGameKind,
    name: &str,
) -> Result<(), CommandError> {
    if !unlocked(w) {
        return Err(CommandError::NotUnlocked);
    }
    if next_change(w).is_some() {
        return Err(CommandError::Invalid(
            "the house drink changes once a week".into(),
        ));
    }
    let name = clean_name(name).ok_or_else(|| CommandError::Invalid("name".into()))?;
    let def = check_plan(w, content, base, jars, style)?;
    let night = w.night;
    let first = w.sig.drink.is_none();
    let changes = match &w.sig.drink {
        Some(d) if d.changed_night != night => d.changes + 1,
        Some(d) => d.changes,
        None => 0,
    };
    let made = w.sig.drink.as_ref().map_or(night, |d| d.made_night);
    w.sig.drink = Some(SignatureDrink {
        name: name.clone(),
        base: base.to_string(),
        jars: jars.to_vec(),
        style,
        made_night: made,
        changed_night: night,
        changes,
        def,
    });
    w.sig.invited = true;
    if first {
        super::orders::learn(w, SIGNATURE_RECIPE, "signature".into());
    }
    w.emit(SimEvent::SignatureComposed { name, first });
    Ok(())
}

/// Day start: grandma's soft prompt, once, on the first day it opens.
pub(crate) fn on_day_start(w: &mut World, _content: &Content) {
    if !w.sig.invited && w.sig.drink.is_none() && unlocked(w) {
        w.sig.invited = true;
        w.emit(SimEvent::SignatureInvite);
    }
}

/// A served house drink a guest asked for by name (the `house_special` wishes).
pub(crate) fn on_served(w: &mut World, content: &Content, wish: Option<&str>, recipe: &str) {
    if recipe != SIGNATURE_RECIPE {
        return;
    }
    let asked = wish
        .and_then(|id| content.handson.wish(id))
        .is_some_and(|wd| wd.favourite.iter().any(|f| f == SIGNATURE_RECIPE));
    if asked {
        w.sig.asked_served += 1;
    }
}

// ---------------------------------------------------------------------------------------------
// Photos
// ---------------------------------------------------------------------------------------------

/// `Command::TakePhoto`: `focus` = the middle of the frame (world px), `width` = the framed
/// width (world px, 4:3).
pub(crate) fn take_photo(
    w: &mut World,
    content: &Content,
    focus: (f32, f32),
    width: f32,
) -> Result<(), CommandError> {
    if w.phase != Phase::Night {
        return Err(CommandError::WrongPhase);
    }
    let left = photos_left(w);
    if left == 0 {
        return Err(CommandError::Invalid("no film left tonight".into()));
    }
    if !focus.0.is_finite() || !focus.1.is_finite() || !width.is_finite() {
        return Err(CommandError::Invalid("frame".into()));
    }
    let width = width.clamp(3.0 * TILE, 16.0 * TILE);
    let sw = ((width / TILE).ceil() as i32 + 2).clamp(4, 20);
    let sh = ((width * 0.75 / TILE).ceil() as i32 + 3).clamp(4, 18);
    let tile = grid::px_to_tile(focus);
    let mut snap = super::journal::snap_sized(w, content, Some(tile), Some((sw, sh)));
    snap.focus = focus;
    snap.zoom = sw as f32 * TILE / (width * SNAP_CLOSER);
    let night = w.night;
    let date = (w.mode == Mode::Story)
        .then_some(content.story.calendar.as_ref())
        .flatten()
        .map(|c| {
            let d = c.date_of_night(night);
            (d.year, d.month, d.day)
        });
    let n = w.sig.photos.len() as u32;
    // the café holds still in photo mode (the same tick): each print still gets its own
    // grain, tilt and cache key
    snap.seed ^= (n + 1).wrapping_mul(0x9E37_79B9);
    let caption = 1
        + (super::rng::hash_str(&format!("{}:{night}:{n}", w.seed)) % PHOTO_CAPTIONS as u64) as u32;
    w.sig.photos.push(NightPhoto {
        night,
        minute: w.minute(),
        date,
        caption,
        snap,
    });
    w.emit(SimEvent::PhotoTaken {
        night,
        left: left - 1,
    });
    Ok(())
}

pub(crate) fn photos_left(w: &World) -> u32 {
    let taken = w.sig.photos.iter().filter(|p| p.night == w.night).count() as u32;
    data().photo.per_night.saturating_sub(taken)
}

// ---------------------------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------------------------

impl World {
    /// The café's own drink, once composed.
    pub fn signature(&self) -> Option<&SignatureDrink> {
        self.sig.drink.as_ref()
    }
    /// The recipe book offers "the house drink" (night ≥ `from_night`).
    pub fn signature_unlocked(&self) -> bool {
        unlocked(self)
    }
    /// `None` = it can be composed / changed now; else the night it can be changed again.
    pub fn signature_next_change(&self) -> Option<u32> {
        next_change(self)
    }
    /// Bases with "ready" (its jar on the shelf and its menu item unlocked), in data order.
    pub fn signature_bases(&self, content: &Content) -> Vec<(&'static BaseDef, bool)> {
        data()
            .bases
            .iter()
            .map(|b| (b, base_ready(self, content, b)))
            .collect()
    }
    /// The jars that may go in the cup and are on the shelf (data order).
    pub fn signature_jars(&self) -> Vec<&'static str> {
        data()
            .ingredient_tags
            .iter()
            .map(|(i, _)| i.as_str())
            .filter(|i| self.hands.knowledge.ingredients.contains(*i))
            .collect()
    }
    /// The recipe a plan would make (tastes, price add) or why it can't be made.
    pub fn signature_preview(
        &self,
        content: &Content,
        base: &str,
        jars: &[String],
        style: MiniGameKind,
    ) -> Result<RecipeDef, CommandError> {
        check_plan(self, content, base, jars, style)
    }
    /// A recipe by id: the content's, or the café's own drink.
    pub fn recipe_def<'a>(&'a self, content: &'a Content, id: &str) -> Option<&'a RecipeDef> {
        if id == SIGNATURE_RECIPE {
            return self.sig.drink.as_ref().map(|d| &d.def);
        }
        content.handson.recipe(id)
    }
    /// Every recipe: the content's, then the café's own drink.
    pub fn all_recipes<'a>(&'a self, content: &'a Content) -> impl Iterator<Item = &'a RecipeDef> {
        content
            .handson
            .recipes
            .recipes
            .iter()
            .chain(self.sig.drink.as_ref().map(|d| &d.def))
    }
    /// Every extra photo (photo mode), oldest first.
    pub fn photos(&self) -> &[NightPhoto] {
        &self.sig.photos
    }
    /// Extra photos still possible tonight.
    pub fn photos_left(&self) -> u32 {
        photos_left(self)
    }
    /// Times a guest got the house drink they asked for by name.
    pub fn signature_asked_served(&self) -> u32 {
        self.sig.asked_served
    }
}
