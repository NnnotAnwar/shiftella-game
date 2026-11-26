//! The recipe book's pantry (design/handson.md §4.3–§4.4): experiments with fair hints,
//! grandma's margin notes, ingredient unlocks and revealed silhouette ingredients.

use super::types::*;
use super::world::*;
use crate::content::Content;
use crate::content::handson::{ExperimentOutcome, RecipeDef, experiment};

/// An ingredient joins the pantry (effect / parcel).
pub(crate) fn unlock_ingredient(w: &mut World, content: &Content, id: &str) {
    if content.handson.ingredient(id).is_some()
        && w.hands.knowledge.ingredients.insert(id.to_string())
    {
        w.emit(SimEvent::IngredientUnlocked { id: id.to_string() });
    }
}

/// One more ingredient shows on an unknown recipe's silhouette. False when there was nothing
/// left to reveal (or she knows it).
pub(crate) fn reveal(w: &mut World, content: &Content, recipe: &str) -> bool {
    let Some(r) = content.handson.recipe(recipe) else {
        return false;
    };
    if w.hands.knowledge.recipes.contains_key(recipe) {
        return false;
    }
    let shown = revealed_of(w, r);
    let Some(next) = r.ingredients.iter().find(|i| !shown.contains(i)).cloned() else {
        return false;
    };
    w.hands
        .knowledge
        .revealed
        .entry(recipe.to_string())
        .or_default()
        .push(next.clone());
    w.emit(SimEvent::MarginNote {
        recipe: recipe.to_string(),
        ingredient: next,
    });
    true
}

/// Ingredients shown on a silhouette: the data's `revealed` plus margin notes.
pub(crate) fn revealed_of(w: &World, r: &RecipeDef) -> Vec<String> {
    let mut v = r.revealed.clone();
    if let Some(x) = w.hands.knowledge.revealed.get(&r.id) {
        for i in x {
            if !v.contains(i) {
                v.push(i.clone());
            }
        }
    }
    v
}

fn usable(w: &World, r: &RecipeDef) -> bool {
    r.ingredients
        .iter()
        .all(|i| w.hands.knowledge.ingredients.contains(i))
}

/// The recipe the next margin note is about: undiscovered, experimentable, every jar on the
/// shelf (so the hint is fair), something left to reveal; the most revealed (one silhouette
/// fills in at a time), then data order.
pub(crate) fn margin_note_target<'a>(w: &World, content: &'a Content) -> Option<&'a RecipeDef> {
    content
        .handson
        .recipes
        .recipes
        .iter()
        .filter(|r| r.experimentable && !w.hands.knowledge.recipes.contains_key(&r.id))
        .filter(|r| usable(w, r))
        .filter(|r| revealed_of(w, r).len() < r.ingredients.len())
        // one silhouette at a time: the one grandma already wrote most about
        .min_by_key(|r| std::cmp::Reverse(revealed_of(w, r).len()))
}

/// Checks a `Command::Experiment` request.
pub(crate) fn validate(
    w: &World,
    content: &Content,
    ingredients: &[String],
) -> Result<(), CommandError> {
    let rules = &content.handson.recipes.experiment;
    let mut set: Vec<&String> = ingredients.iter().collect();
    set.sort();
    set.dedup();
    if set.len() != ingredients.len() || !(rules.min..=rules.max).contains(&set.len()) {
        return Err(CommandError::Invalid(format!(
            "pick {}–{} different jars",
            rules.min, rules.max
        )));
    }
    for i in ingredients {
        if content.handson.ingredient(i).is_none() {
            return Err(CommandError::UnknownId(i.clone()));
        }
        if !w.hands.knowledge.ingredients.contains(i) {
            return Err(CommandError::Locked);
        }
    }
    let cost = cost(w, content);
    if cost > 0 && w.econ.money < cost && w.difficulty == Difficulty::Classic {
        return Err(CommandError::NotEnoughMoney {
            need: cost,
            have: w.econ.money,
        });
    }
    Ok(())
}

/// Supplies for one try (free on Cozy early nights).
pub(crate) fn cost(w: &World, content: &Content) -> i64 {
    let t = &content.handson.tuning;
    if w.difficulty.gentle() && w.night <= t.experiment_free_until_night {
        0
    } else {
        t.experiment_cost as i64
    }
}

/// Runs one experiment (validated) and tells the UI (`ExperimentDone`).
pub(crate) fn run(w: &mut World, content: &Content, ingredients: &[String]) -> ExperimentResult {
    let c = cost(w, content);
    if c > 0 {
        w.econ.supplies = (w.econ.supplies - c).max(0);
        if w.phase == Phase::Night {
            w.tonight.supplies += c;
        }
    }
    let mut key: Vec<&str> = ingredients.iter().map(String::as_str).collect();
    key.sort();
    w.hands.knowledge.tried.insert(key.join("+"));
    w.hands.knowledge.tries += 1;
    w.tonight.experiments += 1;
    let chosen: Vec<&str> = ingredients.iter().map(String::as_str).collect();
    // fairness: recipes needing a jar that isn't on the shelf yet never steer the hints
    let known = |id: &str| {
        w.hands.knowledge.recipes.contains_key(id)
            || content.handson.recipe(id).is_some_and(|r| !usable(w, r))
    };
    let outcome = experiment(&content.handson.recipes, &chosen, &known);
    let family = |f: Option<crate::content::handson::Family>| {
        f.map(|f| format!("ingredient.family.{}", f.id()))
    };
    let result = match outcome {
        ExperimentOutcome::Known(r) => ExperimentResult::Known {
            recipe: r.id.clone(),
        },
        ExperimentOutcome::Discovered(r) => ExperimentResult::Discovered {
            recipe: r.id.clone(),
        },
        ExperimentOutcome::Gated(r) => ExperimentResult::Gated {
            recipe: r.id.clone(),
            key: r
                .gated_hint
                .clone()
                .unwrap_or_else(|| format!("book.gated.{}", r.id)),
        },
        ExperimentOutcome::Close {
            kind, family: f, ..
        } => ExperimentResult::Close {
            key: kind.key().to_string(),
            family: family(f),
        },
        ExperimentOutcome::Nothing => {
            let n = content.handson.recipes.experiment.nothing_lines.max(1);
            let k = 1 + w.hands.knowledge.tries % n;
            ExperimentResult::Nothing {
                key: format!("experiment.nothing.{k}"),
                cat: w.sys.extras.cat_here,
            }
        }
    };
    let failed = matches!(
        result,
        ExperimentResult::Close { .. } | ExperimentResult::Nothing { .. }
    );
    if let ExperimentResult::Discovered { recipe } = &result {
        super::orders::learn(w, recipe, "experiment".into());
        w.hands.knowledge.failed_tries = 0;
        let args = vec![("recipe".to_string(), format!("recipe.{recipe}.name"))];
        super::journal::capture(
            w,
            content,
            crate::content::handson::HighlightKind::RecipeDiscovered,
            None,
            args,
            None,
        );
    }
    w.emit(SimEvent::ExperimentDone {
        result: result.clone(),
    });
    if failed {
        w.hands.knowledge.failed_tries += 1;
        let every = content.handson.tuning.margin_note_every.max(1);
        if w.hands.knowledge.failed_tries >= every {
            w.hands.knowledge.failed_tries = 0;
            if let Some(r) = margin_note_target(w, content).map(|r| r.id.clone()) {
                reveal(w, content, &r);
            }
        }
    }
    result
}
