//! Tastes, the pantry, recipes, the canonical [`MiniGameKind`] and experiment logic
//! (`assets/data/recipes.ron`, handson.md §3.1, §4).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::logic::Cond;
use super::{Tier, Unlock};
use crate::content::defs::{MenuCategory, MenuItem};

/// The twelve taste tags of wishes and recipes (`taste.<id>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize)]
pub enum Taste {
    Warm,
    Cold,
    Sweet,
    Bitter,
    Strong,
    Mild,
    Comforting,
    Fancy,
    Quick,
    Filling,
    Spiced,
    Fresh,
}

impl Taste {
    pub const ALL: [Taste; 12] = [
        Taste::Warm,
        Taste::Cold,
        Taste::Sweet,
        Taste::Bitter,
        Taste::Strong,
        Taste::Mild,
        Taste::Comforting,
        Taste::Fancy,
        Taste::Quick,
        Taste::Filling,
        Taste::Spiced,
        Taste::Fresh,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Taste::Warm => "warm",
            Taste::Cold => "cold",
            Taste::Sweet => "sweet",
            Taste::Bitter => "bitter",
            Taste::Strong => "strong",
            Taste::Mild => "mild",
            Taste::Comforting => "comforting",
            Taste::Fancy => "fancy",
            Taste::Quick => "quick",
            Taste::Filling => "filling",
            Taste::Spiced => "spiced",
            Taste::Fresh => "fresh",
        }
    }

    /// `taste.warm` — the chip / icon label.
    pub fn key(self) -> String {
        format!("taste.{}", self.id())
    }
}

/// Ingredient family; drives the experiment hints ("swap the {family}").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum Family {
    Coffee,
    Milk,
    Tea,
    Sweet,
    Spice,
    Fruit,
    Cold,
    Savoury,
    Dough,
}

impl Family {
    pub const ALL: [Family; 9] = [
        Family::Coffee,
        Family::Milk,
        Family::Tea,
        Family::Sweet,
        Family::Spice,
        Family::Fruit,
        Family::Cold,
        Family::Savoury,
        Family::Dough,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Family::Coffee => "coffee",
            Family::Milk => "milk",
            Family::Tea => "tea",
            Family::Sweet => "sweet",
            Family::Spice => "spice",
            Family::Fruit => "fruit",
            Family::Cold => "cold",
            Family::Savoury => "savoury",
            Family::Dough => "dough",
        }
    }

    /// `ingredient.family.tea` — used inside hint sentences ("something {family}").
    pub fn key(self) -> String {
        format!("ingredient.family.{}", self.id())
    }
}

/// Which pantry shelf a jar stands on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum Shelf {
    Bar,
    Kitchen,
}

/// A pantry ingredient (`ingredient.<id>.name`).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IngredientDef {
    pub id: String,
    pub family: Family,
    pub shelf: Shelf,
    /// `Starter` (on the shelf from night 1), `When(conds)` (checked at day start and each game
    /// minute) or `Granted` (only via `UnlockIngredient`).
    #[serde(default)]
    pub unlock: Unlock,
    /// Sandbox: also unlocked from this night (`None` = only by `unlock`).
    #[serde(default)]
    pub sandbox_night: Option<u32>,
}

impl IngredientDef {
    pub fn name_key(&self) -> String {
        format!("ingredient.{}.name", self.id)
    }
}

/// The mini-game a recipe is brewed with. **Canonical** id set — `src/minigame` maps its own
/// enum to this one. `SoupStir` and `MulledSpice` are SHOULD: recipes using them carry a
/// MUST `fallback_minigame`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize)]
pub enum MiniGameKind {
    MilkFroth,
    LatteArt,
    PourOver,
    TeaSteep,
    Plate,
    SoupStir,
    MulledSpice,
}

impl MiniGameKind {
    pub const ALL: [MiniGameKind; 7] = [
        MiniGameKind::MilkFroth,
        MiniGameKind::LatteArt,
        MiniGameKind::PourOver,
        MiniGameKind::TeaSteep,
        MiniGameKind::Plate,
        MiniGameKind::SoupStir,
        MiniGameKind::MulledSpice,
    ];

    /// Snake-case id: `milk_froth`, `latte_art`, `pour_over`, `tea_steep`, `plate`,
    /// `soup_stir`, `mulled_spice` (i18n `minigame.<id>.*`, tuning lists in `minigames.ron`).
    pub fn id(self) -> &'static str {
        match self {
            MiniGameKind::MilkFroth => "milk_froth",
            MiniGameKind::LatteArt => "latte_art",
            MiniGameKind::PourOver => "pour_over",
            MiniGameKind::TeaSteep => "tea_steep",
            MiniGameKind::Plate => "plate",
            MiniGameKind::SoupStir => "soup_stir",
            MiniGameKind::MulledSpice => "mulled_spice",
        }
    }

    /// MUST for the first five, SHOULD for SoupStir / MulledSpice.
    pub fn tier(self) -> Tier {
        match self {
            MiniGameKind::SoupStir | MiniGameKind::MulledSpice => Tier::Should,
            _ => Tier::Must,
        }
    }

    /// `minigame.<id>.title` (defined by the mini-game agent in `minigame.tr`).
    pub fn title_key(self) -> String {
        format!("minigame.{}.title", self.id())
    }
}

/// How Karolina comes to know a recipe.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub enum RecipeUnlock {
    /// Known on night 1.
    #[default]
    Starter,
    /// Learned when its base menu item unlocks — a card in Lida's handwriting
    /// ("Found in the drawer: grandma's card").
    WithMenu,
    /// Learned while `by` (a regular in the café, or the staff archetype on shift) is present
    /// and `when` holds (checked each game minute); `recipe.<id>.teach` bubble + card.
    Taught {
        by: String,
        #[serde(default)]
        when: Vec<Cond>,
    },
    /// Learned as soon as `when` holds (any time).
    Story { when: Vec<Cond> },
    /// Only through experiments (or a `LearnRecipe` effect).
    Experiment,
}

/// Recipe-book tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum BookTab {
    Drinks,
    Food,
    Specials,
}

impl BookTab {
    pub const ALL: [BookTab; 3] = [BookTab::Drinks, BookTab::Food, BookTab::Specials];

    pub fn key(self) -> &'static str {
        match self {
            BookTab::Drinks => "book.tab.drinks",
            BookTab::Food => "book.tab.food",
            BookTab::Specials => "book.tab.specials",
        }
    }
}

fn yes() -> bool {
    true
}

/// A recipe: a menu item done Karolina's way (`recipe.<id>.name/.desc/.lore`).
///
/// `base_item` (menu.ron) supplies station, job, prep time, price, cost, power and availability;
/// `price_add` / `cost_add` sit on top.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeDef {
    pub id: String,
    pub base_item: String,
    #[serde(default)]
    pub price_add: u32,
    #[serde(default)]
    pub cost_add: u32,
    pub tags: Vec<Taste>,
    pub minigame: MiniGameKind,
    /// Mini-game variants (tuning in `minigames.ron`); one is picked with the seed.
    pub variants: Vec<String>,
    /// Played while `minigame` (a SHOULD game) isn't available.
    #[serde(default)]
    pub fallback_minigame: Option<MiniGameKind>,
    #[serde(default)]
    pub fallback_variant: Option<String>,
    /// 1–3 pantry ingredients; order never matters.
    pub ingredients: Vec<String>,
    #[serde(default)]
    pub unlock: RecipeUnlock,
    /// Can be found by an exact pantry match (default true; story-gated ones are false).
    #[serde(default = "yes")]
    pub experimentable: bool,
    /// Source hint on the silhouette card (`book.hint.<source>` key).
    #[serde(default)]
    pub hint: Option<String>,
    /// Shown when the exact set is tried before the story allows it (`book.gated.<recipe>`).
    #[serde(default)]
    pub gated_hint: Option<String>,
    /// Ingredients shown on the silhouette from the start (a grandma card or a parcel note).
    #[serde(default)]
    pub revealed: Vec<String>,
    /// A LatteArt finish is offered for Fancy wishes ("+ art" on the order card).
    #[serde(default)]
    pub art_finish: bool,
    #[serde(default)]
    pub tier: Tier,
}

impl RecipeDef {
    pub fn name_key(&self) -> String {
        format!("recipe.{}.name", self.id)
    }
    pub fn desc_key(&self) -> String {
        format!("recipe.{}.desc", self.id)
    }
    /// Lida's handwritten line on the right page.
    pub fn lore_key(&self) -> String {
        format!("recipe.{}.lore", self.id)
    }
    /// Bubble when a regular / helper teaches it (`Taught` only).
    pub fn teach_key(&self) -> Option<String> {
        matches!(self.unlock, RecipeUnlock::Taught { .. }).then(|| format!("recipe.{}.teach", self.id))
    }

    /// The ingredient set (order never matters).
    pub fn ingredient_set(&self) -> BTreeSet<&str> {
        self.ingredients.iter().map(String::as_str).collect()
    }

    /// The mini-game to play when SHOULD games are unavailable.
    pub fn playable_minigame(&self, should_games: bool) -> (MiniGameKind, &str) {
        let main = (
            self.minigame,
            self.variants.first().map(String::as_str).unwrap_or(""),
        );
        if should_games || self.minigame.tier() == Tier::Must {
            return main;
        }
        match (&self.fallback_minigame, &self.fallback_variant) {
            (Some(k), Some(v)) => (*k, v.as_str()),
            _ => main,
        }
    }

    /// Book tab: anything beyond the plain menu way (taught, story, experiment, or a surcharge)
    /// is a special; otherwise drinks vs food by the base item's category.
    pub fn book_tab(&self, base: &MenuItem) -> BookTab {
        let plain = matches!(self.unlock, RecipeUnlock::Starter | RecipeUnlock::WithMenu)
            && self.price_add == 0;
        match (plain, base.category) {
            (false, _) => BookTab::Specials,
            (true, MenuCategory::Drink) => BookTab::Drinks,
            (true, MenuCategory::Special) => BookTab::Specials,
            (true, _) => BookTab::Food,
        }
    }
}

/// Experiment settings (`recipes.ron` → `experiment`).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExperimentRules {
    /// Pantry slots: pick `min..=max` jars.
    pub min: usize,
    pub max: usize,
    /// Number of `experiment.nothing.<n>` lines (1-based).
    pub nothing_lines: u32,
}

/// `assets/data/recipes.ron`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecipesFile {
    pub ingredients: Vec<IngredientDef>,
    pub recipes: Vec<RecipeDef>,
    pub experiment: ExperimentRules,
}

/// Which way an experiment was "almost" right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseKind {
    /// Same size, one wrong: "swap the {family of the wrong one}" (`experiment.close.swap`).
    Swap,
    /// Contains the recipe plus extras: "one thing too many" (`experiment.close.extra`).
    Extra,
    /// One short: "it wants something {family of the missing one}" (`experiment.close.missing`).
    Missing,
}

impl CloseKind {
    pub fn key(self) -> &'static str {
        match self {
            CloseKind::Swap => "experiment.close.swap",
            CloseKind::Extra => "experiment.close.extra",
            CloseKind::Missing => "experiment.close.missing",
        }
    }
}

/// Result of trying a set of pantry ingredients (handson.md §4.4).
#[derive(Debug, Clone, PartialEq)]
pub enum ExperimentOutcome<'a> {
    /// Exact match of a recipe she knows (`experiment.known`, `{recipe}`).
    Known(&'a RecipeDef),
    /// Exact match of an unknown experimentable recipe (`experiment.discovered`, `{recipe}`).
    Discovered(&'a RecipeDef),
    /// Exact match of a story-gated recipe: show its `gated_hint`.
    Gated(&'a RecipeDef),
    /// Almost: `family` is the wrong (Swap) or missing (Missing) ingredient's family.
    Close {
        recipe: &'a RecipeDef,
        kind: CloseKind,
        family: Option<Family>,
    },
    /// Nothing: show one of `experiment.nothing.<n>`.
    Nothing,
}

/// Evaluates an experiment. `known(recipe_id)` says whether Karolina knows a recipe.
/// Pure and deterministic: ties prefer the fewest ingredients, then the data order.
pub fn experiment<'a>(
    file: &'a RecipesFile,
    chosen: &[&str],
    known: &dyn Fn(&str) -> bool,
) -> ExperimentOutcome<'a> {
    let s: BTreeSet<&str> = chosen.iter().copied().collect();
    if let Some(r) = file.recipes.iter().find(|r| r.ingredient_set() == s) {
        return if known(&r.id) {
            ExperimentOutcome::Known(r)
        } else if r.experimentable {
            ExperimentOutcome::Discovered(r)
        } else {
            ExperimentOutcome::Gated(r)
        };
    }
    let family = |id: &str| file.ingredients.iter().find(|i| i.id == id).map(|i| i.family);
    let best = file
        .recipes
        .iter()
        .filter(|r| r.experimentable && !known(&r.id))
        .map(|r| (r, r.ingredient_set().intersection(&s).count()))
        .filter(|(_, overlap)| *overlap > 0)
        .min_by_key(|(r, overlap)| (std::cmp::Reverse(*overlap), r.ingredients.len()));
    let Some((r, overlap)) = best else {
        return ExperimentOutcome::Nothing;
    };
    let rs = r.ingredient_set();
    let n = rs.len();
    if overlap + 1 == n && s.len() == n {
        let wrong = s.difference(&rs).next().copied();
        ExperimentOutcome::Close {
            recipe: r,
            kind: CloseKind::Swap,
            family: wrong.and_then(family),
        }
    } else if overlap == n && s.len() > n {
        ExperimentOutcome::Close {
            recipe: r,
            kind: CloseKind::Extra,
            family: None,
        }
    } else if overlap + 1 == n && s.len() < n {
        let missing = rs.difference(&s).next().copied();
        ExperimentOutcome::Close {
            recipe: r,
            kind: CloseKind::Missing,
            family: missing.and_then(family),
        }
    } else {
        ExperimentOutcome::Nothing
    }
}
