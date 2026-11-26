//! Interpreter for the hands-on conditions `Cond = Core(Condition) | X(XCondition)`
//! (`docs/CONTENT.md` §10.0). Core conditions go to `logic::cond`.

use super::logic;
use super::rng::Rng;
use super::world::*;
use crate::content::Content;
use crate::content::handson::{Cond, XCondition};

/// All hold, with an explicit RNG (read-only world).
pub(crate) fn all_with(
    w: &World,
    content: &Content,
    conds: &[Cond],
    ctx: &mut EffectCtx,
    rng: &mut Rng,
) -> bool {
    conds.iter().all(|c| cond(w, content, c, ctx, rng))
}

/// Read-only check that never consumes the world RNG (a hash roll for `Chance`).
pub(crate) fn holds(w: &World, content: &Content, conds: &[Cond]) -> bool {
    let mut rng = Rng::new((w.tick as u64) ^ ((w.night as u64) << 32) ^ 0x5EED);
    all_with(w, content, conds, &mut EffectCtx::default(), &mut rng)
}

pub(crate) fn cond(
    w: &World,
    content: &Content,
    c: &Cond,
    ctx: &mut EffectCtx,
    rng: &mut Rng,
) -> bool {
    use XCondition::*;
    let x = match c {
        Cond::Core(core) => return logic::cond(w, content, core, ctx, rng),
        Cond::X(x) => x,
    };
    match x {
        All(list) => list.iter().all(|c| cond(w, content, c, ctx, rng)),
        Any(list) => list.iter().any(|c| cond(w, content, c, ctx, rng)),
        Not(c) => !cond(w, content, c, &mut ctx.clone(), rng),
        RecipeKnown(r) => w.hands.knowledge.recipes.contains_key(r),
        ServedRecipe { regular, recipe } => w
            .story
            .regulars
            .get(regular)
            .is_some_and(|r| r.served_recipes.contains(recipe)),
        WishMatchStreak {
            regular,
            min,
            visits,
        } => {
            let Some(r) = w.story.regulars.get(regular) else {
                return false;
            };
            // completed visits only (the current one may still get its coffee)
            let present = r
                .party
                .is_some_and(|pid| w.parties.iter().any(|p| p.id == pid));
            let mut v: Vec<&VisitRecord> = r.visits.iter().collect();
            if present && v.last().is_some_and(|l| l.best_match.is_none()) {
                v.pop();
            }
            v.len() >= *visits as usize
                && v.iter()
                    .rev()
                    .take(*visits as usize)
                    .all(|r| r.best_match.is_some_and(|m| m >= *min))
        }
        HallMood(m) => w.hall_mood(content) == Some(*m),
        QuietHour => super::pacing::quiet_hour(w, content),
        Blackout => w.blackout_until.is_some(),
        StaffLowEnergy(e) => w
            .staff
            .iter()
            .any(|s| !s.owner && s.on_duty() && s.energy < *e),
        // moments are T5: nothing has been resolved yet
        MomentResolvedEver(_) => false,
        IngredientUnlocked(i) => w.hands.knowledge.ingredients.contains(i),
        RecordUnlocked(r) => w.hands.music.unlocked.contains(r),
        SeasonPhase(p) => super::pacing::season_phase(w, content).is_some_and(|x| x == p),
        // the coziness meter arrives with T5; until then the hall counts as "plain" (30)
        CozinessAtLeast(n) => 30.0 >= *n,
        HiredArchetype(a) => w.staff.iter().any(|s| !s.owner && &s.archetype == a),
        ShiftsWorked(a, n) => w
            .staff
            .iter()
            .any(|s| !s.owner && &s.archetype == a && s.nights_worked >= *n),
        // shared-shift bookkeeping arrives with the rozpis puzzle (T5)
        SharedShifts { .. } => false,
        TipsLifetimeAtLeast(czk) => w.hands.tips_lifetime >= *czk as i64,
        // napkins are T5
        NapkinsAtLeast(n) => *n == 0,
        ThemeOwned(t) => content
            .handson
            .decor
            .default_theme()
            .is_some_and(|d| &d.id == t),
    }
}
