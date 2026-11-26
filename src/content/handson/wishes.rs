//! Guest wishes, the match score and reactions (`assets/data/wishes.ron`, handson.md §3).

use serde::{Deserialize, Serialize};

use super::logic::Cond;
use super::recipes::{RecipeDef, Taste};

/// Narrows which recipes can answer a wish.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
pub enum WishKind {
    #[default]
    Any,
    Drink,
    Food,
}

/// Multiplies a wish's pick weight while `when` holds.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WeightMod {
    pub when: Vec<Cond>,
    pub mul: f32,
}

fn one() -> f32 {
    1.0
}

fn one_u() -> u32 {
    1
}

/// A wish (`wish.<id>`). Personal ids carry the regular prefix: `mirek.like_lida` →
/// `wish.mirek.like_lida`, and then `who` must be exactly that regular.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WishDef {
    pub id: String,
    /// 1–3 tags the guest wants (empty only with `any: true`).
    #[serde(default)]
    pub want: Vec<Taste>,
    /// 0–2 tags that cost 0.5 of the match.
    #[serde(default)]
    pub avoid: Vec<Taste>,
    /// "I don't know what I want": every recipe scores 0.6 before avoid / favourites.
    #[serde(default)]
    pub any: bool,
    /// Exact favourite recipes → match 1.0.
    #[serde(default)]
    pub favourite: Vec<String>,
    /// Favourite menu items → +0.15 for any recipe riding on them.
    #[serde(default)]
    pub favourite_items: Vec<String>,
    /// Guest archetype ids and/or regular ids (empty = anyone).
    #[serde(default)]
    pub who: Vec<String>,
    #[serde(default)]
    pub when: Vec<Cond>,
    /// Smallest party size of the leader's party.
    #[serde(default = "one_u")]
    pub min_party: u32,
    #[serde(default = "one")]
    pub weight: f32,
    #[serde(default)]
    pub weight_mods: Vec<WeightMod>,
    #[serde(default)]
    pub kind: WishKind,
    /// Narrower mini-game band for "properly made" (TeaSteep tolerance ÷ 1.5).
    #[serde(default)]
    pub strict: bool,
    /// Latte pattern pre-selected for the LatteArt finish.
    #[serde(default)]
    pub latte: Option<String>,
    /// Uses per night (`None` = 2 for generic, 1 for personal wishes).
    #[serde(default)]
    pub max_per_night: Option<u32>,
}

impl WishDef {
    pub fn key(&self) -> String {
        format!("wish.{}", self.id)
    }

    /// The regular a personal wish belongs to (`mirek` for `mirek.like_lida`).
    pub fn regular(&self) -> Option<&str> {
        self.id.split_once('.').map(|(r, _)| r)
    }

    pub fn uses_per_night(&self) -> u32 {
        self.max_per_night
            .unwrap_or(if self.regular().is_some() { 1 } else { 2 })
    }
}

/// Match-score tiers (`handson.ron` → `match_tiers`).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MatchTiers {
    pub perfect: f32,
    pub good: f32,
    pub okay: f32,
}

/// Reaction tier of a served item (`react.<tier>.<n>`, `react.<regular>.<tier>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum MatchTier {
    Perfect,
    Good,
    Okay,
    NotQuite,
}

impl MatchTier {
    pub const ALL: [MatchTier; 4] = [
        MatchTier::Perfect,
        MatchTier::Good,
        MatchTier::Okay,
        MatchTier::NotQuite,
    ];

    pub fn id(self) -> &'static str {
        match self {
            MatchTier::Perfect => "perfect",
            MatchTier::Good => "good",
            MatchTier::Okay => "okay",
            MatchTier::NotQuite => "not_quite",
        }
    }

    pub fn of(score: f32, tiers: &MatchTiers) -> MatchTier {
        if score >= tiers.perfect {
            MatchTier::Perfect
        } else if score >= tiers.good {
            MatchTier::Good
        } else if score >= tiers.okay {
            MatchTier::Okay
        } else {
            MatchTier::NotQuite
        }
    }

    /// Generic line `react.<tier>.<n>` (n = 1-based).
    pub fn generic_key(self, n: u32) -> String {
        format!("react.{}.{n}", self.id())
    }

    /// Personal line `react.<regular>.<tier>`.
    pub fn regular_key(self, regular: &str) -> String {
        format!("react.{regular}.{}", self.id())
    }
}

/// Bonuses on top of the tag overlap.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MatchContext {
    /// The recipe is on tonight's specials board (+0.1, SHOULD).
    pub on_specials_board: bool,
}

/// The match score of `recipe` for `wish` (0..=1), handson.md §3.2:
///
/// ```text
/// base  = |want ∩ tags| / |want|        (any → 0.6)
/// avoid = 0.5 if avoid ∩ tags ≠ ∅
/// fav   = exact favourite → 1.0 · base item in favourite_items → +0.15
/// board = +0.1 when on tonight's specials board
/// ```
pub fn match_score(wish: &WishDef, recipe: &RecipeDef, ctx: MatchContext) -> f32 {
    if wish.favourite.iter().any(|f| f == &recipe.id) {
        return 1.0;
    }
    let base = if wish.any || wish.want.is_empty() {
        0.6
    } else {
        let hits = wish.want.iter().filter(|t| recipe.tags.contains(t)).count();
        hits as f32 / wish.want.len() as f32
    };
    let avoid = if wish.avoid.iter().any(|t| recipe.tags.contains(t)) {
        0.5
    } else {
        0.0
    };
    let fav = if wish.favourite_items.iter().any(|i| i == &recipe.base_item) {
        0.15
    } else {
        0.0
    };
    let board = if ctx.on_specials_board { 0.1 } else { 0.0 };
    (base - avoid + fav + board).clamp(0.0, 1.0)
}

/// Generic and personal reaction lines.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Reactions {
    /// `react.<tier>.1 ..= n` exist for every tier.
    pub generic_per_tier: u32,
    /// Regulars with personal `react.<regular>.<tier>` lines (one per tier).
    pub regulars: Vec<String>,
}

/// `assets/data/wishes.ron`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WishesFile {
    pub wishes: Vec<WishDef>,
    pub reactions: Reactions,
}
