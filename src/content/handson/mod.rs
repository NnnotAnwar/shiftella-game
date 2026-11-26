//! Hands-on content (handson.md): Karolina behind the counter, recipes, wishes, moments,
//! mini-game tuning, the jukebox, seasons, decor & coziness, the journal, the rozpis puzzle,
//! the app as a tool, the tutorial and small extras.
//!
//! Loaded as part of [`Content`](crate::content::Content) into `content.handson`;
//! validated by `Content::validate`. Schemas and semantics: `docs/CONTENT.md` §10–§12.

pub mod app;
pub mod extras;
pub mod journal;
pub mod keys;
pub mod logic;
pub mod minigames;
pub mod moments;
pub mod recipes;
pub mod records;
pub mod rozpis;
pub mod seasons;
pub mod tuning;
pub mod tutorial;
pub mod wishes;

#[cfg(test)]
mod tests;

pub use app::*;
pub use extras::*;
pub use journal::*;
pub use logic::{Cond, Fx, XCondition, XEffect, XRef};
pub use minigames::*;
pub use moments::*;
pub use recipes::*;
pub use records::*;
pub use rozpis::*;
pub use seasons::*;
pub use tuning::*;
pub use tutorial::*;
pub use wishes::*;

use serde::{Deserialize, Serialize};

/// Design priority of a definition (handson.md tags). SHOULD content ships as data; the sim /
/// UI may ignore it until the feature exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
pub enum Tier {
    #[default]
    Must,
    Should,
    Later,
}

/// How an ingredient, record or theme becomes available.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub enum Unlock {
    /// Available from the start.
    #[default]
    Starter,
    /// Becomes available once all conditions hold (checked at day start and each game minute;
    /// stays unlocked).
    When(Vec<Cond>),
    /// Only through an effect (`UnlockIngredient`, `UnlockRecord`, `UnlockTheme`).
    Granted,
}

impl Unlock {
    pub fn conditions(&self) -> &[Cond] {
        match self {
            Unlock::When(c) => c,
            _ => &[],
        }
    }
}

/// Every hands-on data file, loaded together.
#[derive(Debug, Clone, PartialEq)]
pub struct HandsOn {
    pub tuning: HandsonTuning,
    pub recipes: RecipesFile,
    pub wishes: WishesFile,
    pub moments: Vec<MomentDef>,
    pub minigames: MiniGamesFile,
    pub records: RecordsFile,
    pub seasons: SeasonsFile,
    pub decor: DecorFile,
    pub journal: JournalFile,
    pub tutorial: Vec<TutorialStep>,
    pub extras: ExtrasFile,
}

/// The hands-on data files, in load order.
pub const HANDSON_FILES: &[&str] = &[
    "handson.ron",
    "recipes.ron",
    "wishes.ron",
    "moments.ron",
    "minigames.ron",
    "records.ron",
    "seasons.ron",
    "decor.ron",
    "journal.ron",
    "tutorial.ron",
    "extras.ron",
];

impl HandsOn {
    /// Reads every file of [`HANDSON_FILES`] from `dir`.
    pub fn load(dir: &str) -> Result<HandsOn, String> {
        use crate::content::read;
        Ok(HandsOn {
            tuning: read(dir, "handson.ron")?,
            recipes: read(dir, "recipes.ron")?,
            wishes: read(dir, "wishes.ron")?,
            moments: read(dir, "moments.ron")?,
            minigames: read(dir, "minigames.ron")?,
            records: read(dir, "records.ron")?,
            seasons: read(dir, "seasons.ron")?,
            decor: read(dir, "decor.ron")?,
            journal: read(dir, "journal.ron")?,
            tutorial: read(dir, "tutorial.ron")?,
            extras: read(dir, "extras.ron")?,
        })
    }

    pub fn recipe(&self, id: &str) -> Option<&RecipeDef> {
        self.recipes.recipes.iter().find(|r| r.id == id)
    }
    pub fn ingredient(&self, id: &str) -> Option<&IngredientDef> {
        self.recipes.ingredients.iter().find(|i| i.id == id)
    }
    pub fn wish(&self, id: &str) -> Option<&WishDef> {
        self.wishes.wishes.iter().find(|w| w.id == id)
    }
    pub fn moment(&self, id: &str) -> Option<&MomentDef> {
        self.moments.iter().find(|m| m.id == id)
    }
    pub fn record(&self, id: &str) -> Option<&RecordDef> {
        self.records.record(id)
    }
    pub fn theme(&self, id: &str) -> Option<&ThemeDef> {
        self.decor.theme(id)
    }
    pub fn phase(&self, id: &str) -> Option<&PhaseDef> {
        self.seasons.phase(id)
    }
    pub fn tutorial_step(&self, id: &str) -> Option<&TutorialStep> {
        self.tutorial.iter().find(|s| s.id == id)
    }

    /// Recipes riding on a menu item (the base recipe first).
    pub fn recipes_for_item<'a>(&'a self, item: &'a str) -> impl Iterator<Item = &'a RecipeDef> {
        self.recipes
            .recipes
            .iter()
            .filter(move |r| r.base_item == item)
    }

    /// Recipes known on night 1.
    pub fn starter_recipes(&self) -> impl Iterator<Item = &RecipeDef> {
        self.recipes
            .recipes
            .iter()
            .filter(|r| r.unlock == RecipeUnlock::Starter)
    }

    /// Personal wishes of a regular.
    pub fn wishes_of<'a>(&'a self, regular: &'a str) -> impl Iterator<Item = &'a WishDef> {
        self.wishes
            .wishes
            .iter()
            .filter(move |w| w.regular() == Some(regular))
    }

    /// The best `n` recipes for a wish among `available` ones, highest match first (ties: data order).
    pub fn best_matches<'a>(
        &'a self,
        wish: &WishDef,
        available: &dyn Fn(&RecipeDef) -> bool,
        ctx: &dyn Fn(&RecipeDef) -> MatchContext,
        n: usize,
    ) -> Vec<(&'a RecipeDef, f32)> {
        let mut v: Vec<(&RecipeDef, f32)> = self
            .recipes
            .recipes
            .iter()
            .filter(|r| available(r))
            .map(|r| (r, match_score(wish, r, ctx(r))))
            .collect();
        v.sort_by(|a, b| b.1.total_cmp(&a.1));
        v.truncate(n);
        v
    }

    /// Counts for logs / `content_check`.
    pub fn summary(&self) -> String {
        let must_moments = self.moments.iter().filter(|m| m.tier == Tier::Must).count();
        format!(
            "ingredients {} · recipes {} · wishes {} · moments {} ({} MUST) · records {} · \
             season phases {} · themes {} · latte patterns {} · tutorial steps {} · napkins {}",
            self.recipes.ingredients.len(),
            self.recipes.recipes.len(),
            self.wishes.wishes.len(),
            self.moments.len(),
            must_moments,
            self.records.records.len(),
            self.seasons.phases.len(),
            self.decor.themes.len(),
            self.minigames.latte_patterns.len(),
            self.tutorial.len(),
            self.extras.napkins.len(),
        )
    }
}
