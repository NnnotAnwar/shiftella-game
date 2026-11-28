//! Orders, wishes and recipes (design/handson.md §1.2, §1.7, §3, §4.3): which order kind a
//! party's leader has, fair wish picking, Karolina's recipe knowledge, the serve formulas
//! (satisfaction, trust, tips, reactions), and the ticket rail.

use super::hands::*;
use super::types::*;
use super::world::*;
use super::xlogic;
use crate::content::handson::{
    Duty, MatchContext, MatchTier, RecipeDef, RecipeUnlock, Unlock, WishDef, WishKind, match_score,
};
use crate::content::{Content, Job, RegularDef};

/// A plain order made by Karolina: they got exactly what they asked for, but nothing
/// personal — this stands in for the match term of the serve formulas.
pub const PLAIN_CREDIT: f32 = 0.5;

// ---------------------------------------------------------------------------------------------
// Knowledge
// ---------------------------------------------------------------------------------------------

pub(crate) fn known(w: &World, recipe: &str) -> bool {
    w.hands.knowledge.recipes.contains_key(recipe)
}

/// Karolina knows it and its base item can be made tonight.
pub(crate) fn recipe_available(w: &World, content: &Content, r: &RecipeDef) -> bool {
    known(w, &r.id)
        && content
            .menu_item(&r.base_item)
            .is_some_and(|m| super::guests::item_orderable(w, content, m))
}

/// Can the café make what this line names tonight? (`wishes.ron` → `line_items`: a menu item
/// — any recipe on it known, the item on the menu, its station there — or one recipe.) A line
/// that names nothing is always fine. Guests only say a line naming a dish while this holds
/// (wishes, regulars' arrival and hover lines): the owner was asked for a koláč with no oven.
pub(crate) fn line_ok(w: &World, content: &Content, key: &str) -> bool {
    let Some(items) = content.handson.wishes.line_items.get(key) else {
        return true;
    };
    items.iter().any(|id| named_available(w, content, id))
}

/// A named dish (menu item or recipe id) can be made tonight.
fn named_available(w: &World, content: &Content, id: &str) -> bool {
    if let Some(r) = w.recipe_def(content, id) {
        return recipe_available(w, content, r);
    }
    w.all_recipes(content)
        .any(|r| r.base_item == id && recipe_available(w, content, r))
}

/// Does recipe `r` answer what the wish's line names (`line_items`)? `None`: it names nothing.
pub(crate) fn names_recipe(content: &Content, wish: &WishDef, r: &RecipeDef) -> Option<bool> {
    let items = content.handson.wishes.line_items.get(&wish.key())?;
    Some(items.iter().any(|id| *id == r.id || *id == r.base_item))
}

pub(crate) fn learn(w: &mut World, recipe: &str, source: String) {
    if known(w, recipe) {
        return;
    }
    let night = w.night;
    w.hands.knowledge.recipes.insert(
        recipe.to_string(),
        RecipeStats {
            learned_night: night,
            ..Default::default()
        },
    );
    w.emit(SimEvent::RecipeLearned {
        recipe: recipe.to_string(),
        source,
    });
}

/// New game / migrated save: starter recipes, grandma's cards of unlocked items, the
/// starter pantry (no events).
pub(crate) fn init_knowledge(w: &mut World, content: &Content) {
    let night = w.night;
    for r in &content.handson.recipes.recipes {
        let ok = match &r.unlock {
            RecipeUnlock::Starter => true,
            RecipeUnlock::WithMenu => content
                .menu_item(&r.base_item)
                .is_some_and(|m| super::guests::item_unlocked(w, m)),
            _ => false,
        };
        if ok {
            w.hands
                .knowledge
                .recipes
                .entry(r.id.clone())
                .or_insert(RecipeStats {
                    learned_night: night,
                    ..Default::default()
                });
        }
    }
    for i in &content.handson.recipes.ingredients {
        if i.unlock == Unlock::Starter {
            w.hands.knowledge.ingredients.insert(i.id.clone());
        }
    }
}

/// Grandma's cards (menu unlocks), recipes taught by regulars / helpers who are present,
/// story recipes, and pantry unlocks. Day start + every 5 game minutes.
pub(crate) fn check_learning(w: &mut World, content: &Content) {
    let mut learned: Vec<(String, String)> = Vec::new();
    for r in &content.handson.recipes.recipes {
        if known(w, &r.id) {
            continue;
        }
        match &r.unlock {
            RecipeUnlock::Starter => learned.push((r.id.clone(), "starter".into())),
            RecipeUnlock::WithMenu => {
                if content
                    .menu_item(&r.base_item)
                    .is_some_and(|m| super::guests::item_unlocked(w, m))
                {
                    learned.push((r.id.clone(), "card".into()));
                }
            }
            RecipeUnlock::Taught { by, when } => {
                let present = super::logic::regular_present(w, by)
                    || w.staff
                        .iter()
                        .any(|s| !s.owner && s.on_duty() && &s.archetype == by);
                if present && xlogic::holds(w, content, when) {
                    learned.push((r.id.clone(), format!("taught:{by}")));
                }
            }
            RecipeUnlock::Story { when } => {
                if xlogic::holds(w, content, when) {
                    learned.push((r.id.clone(), "story".into()));
                }
            }
            RecipeUnlock::Experiment => {}
        }
    }
    for (id, src) in learned {
        if let Some(key) = content.handson.recipe(&id).and_then(|r| r.teach_key())
            && let Some(by) = src.strip_prefix("taught:")
        {
            let agent = w
                .story
                .regulars
                .get(by)
                .and_then(|r| r.party)
                .and_then(|p| w.parties.iter().find(|x| x.id == p))
                .and_then(|p| p.leader())
                .map(AgentRef::Guest);
            w.emit(SimEvent::Speech {
                speaker: by.to_string(),
                agent,
                key,
            });
        }
        learn(w, &id, src);
    }
    let mut unlocked = Vec::new();
    for i in &content.handson.recipes.ingredients {
        if w.hands.knowledge.ingredients.contains(&i.id) {
            continue;
        }
        let ok = match &i.unlock {
            Unlock::Starter => true,
            Unlock::Granted => false,
            Unlock::When(c) => xlogic::holds(w, content, c),
        } || (w.mode == Mode::Sandbox && i.sandbox_night.is_some_and(|n| w.night >= n));
        if ok {
            unlocked.push(i.id.clone());
        }
    }
    for id in unlocked {
        w.hands.knowledge.ingredients.insert(id.clone());
        w.emit(SimEvent::IngredientUnlocked { id });
    }
}

// ---------------------------------------------------------------------------------------------
// Wishes
// ---------------------------------------------------------------------------------------------

fn kind_ok(content: &Content, kind: WishKind, r: &RecipeDef) -> bool {
    match kind {
        WishKind::Any => true,
        WishKind::Drink => content
            .menu_item(&r.base_item)
            .is_some_and(|m| m.job == Job::Bar),
        WishKind::Food => content
            .menu_item(&r.base_item)
            .is_some_and(|m| m.job == Job::Kitchen),
    }
}

/// Recipes for a wish among Karolina's available ones, best first (ties: data order).
pub(crate) fn matches_for(w: &World, content: &Content, wish: &WishDef) -> Vec<(String, f32)> {
    // the café's own drink answers wishes like any recipe (`sim::signature`)
    let mut v: Vec<(String, f32)> = w
        .all_recipes(content)
        .filter(|r| kind_ok(content, wish.kind, r) && recipe_available(w, content, r))
        .map(|r| (r.id.clone(), match_score(wish, r, match_ctx(w, &r.id))))
        .collect();
    v.sort_by(|a, b| b.1.total_cmp(&a.1));
    v
}

/// Tonight's specials board gives its recipes a little extra match (§6.8).
pub(crate) fn match_ctx(w: &World, recipe: &str) -> MatchContext {
    MatchContext {
        on_specials_board: w.sys.extras.specials.iter().any(|r| r == recipe),
    }
}

fn best_match(w: &World, content: &Content, wish: &WishDef) -> f32 {
    matches_for(w, content, wish)
        .first()
        .map(|x| x.1)
        .unwrap_or(0.0)
}

/// Pick weight of a wish right now (0 = not eligible): fair wishes (never one the café
/// can't answer), boosts, weight mods, the hall mood and the season.
fn wish_weight(w: &World, content: &Content, wish: &WishDef) -> f32 {
    let t = &content.handson.tuning;
    if w.hands.wish_uses.get(&wish.id).copied().unwrap_or(0) >= wish.uses_per_night() {
        return 0.0;
    }
    if !xlogic::holds(w, content, &wish.when) {
        return 0.0;
    }
    // a wish that names a dish ("show me your koláč") only while the café can make one
    if !line_ok(w, content, &wish.key()) {
        return 0.0;
    }
    let best = best_match(w, content, wish);
    if best < t.fair_wishes.skip_below {
        return 0.0;
    }
    let mut weight = wish.weight;
    if best >= t.fair_wishes.boost_at {
        weight *= t.fair_wishes.boost_mul;
    }
    for m in &wish.weight_mods {
        if xlogic::holds(w, content, &m.when) {
            weight *= m.mul;
        }
    }
    let mut tag_mul = |list: &[(crate::content::handson::Taste, f32)]| {
        for (tag, mul) in list {
            if wish.want.contains(tag) {
                weight *= mul;
            }
        }
    };
    if let Some(m) = super::music::mood_effect(w, content) {
        tag_mul(&m.wish_mul);
    }
    if let Some(p) =
        super::pacing::season_phase(w, content).and_then(|id| content.handson.seasons.phase(id))
    {
        tag_mul(&p.wish_mul);
    }
    // moment boosts (the open window: Cold / Fresh wishes)
    let now = w.minute();
    for (tags, mul, until) in &w.sys.fx.wish_boosts {
        if until.is_none_or(|u| u > now) && wish.want.iter().any(|t| tags.contains(t)) {
            weight *= mul;
        }
    }
    weight.max(0.0)
}

fn pick(w: &mut World, list: Vec<(&WishDef, f32)>) -> Option<String> {
    let weights: Vec<f32> = list.iter().map(|x| x.1).collect();
    if weights.iter().all(|x| *x <= 0.0) {
        return None;
    }
    let i = w.rng.weighted(&weights)?;
    let id = list[i].0.id.clone();
    *w.hands.wish_uses.entry(id.clone()).or_insert(0) += 1;
    Some(id)
}

/// The leader's order kind for a new party (§1.2): regulars always wish (their personal
/// wishes first; "the usual" once trusted), others wish with `wish_chance` (quiet hour:
/// `quiet_hour.wish_share`), or always when `force_wish` (lull walk-ins).
pub(crate) fn pick_order_kind(
    w: &mut World,
    content: &Content,
    archetype: &str,
    regular: Option<&RegularDef>,
    size: u32,
    force_wish: bool,
) -> OrderKind {
    let t = &content.handson.tuning;
    let wishes = &content.handson.wishes.wishes;
    if let Some(r) = regular {
        let personal: Vec<(&WishDef, f32)> = wishes
            .iter()
            .filter(|x| x.regular() == Some(r.id.as_str()))
            .map(|x| (x, wish_weight(w, content, x)))
            .filter(|x| x.1 > 0.0)
            .collect();
        let id = pick(w, personal).or_else(|| {
            let generic: Vec<(&WishDef, f32)> = wishes
                .iter()
                .filter(|x| {
                    x.regular().is_none()
                        && (x.who.is_empty() || x.who.iter().any(|a| a == &r.base))
                        && x.min_party <= size
                })
                .map(|x| (x, wish_weight(w, content, x)))
                .filter(|x| x.1 > 0.0)
                .collect();
            pick(w, generic)
        });
        let Some(id) = id else {
            return OrderKind::Plain;
        };
        let trust = w.story.trust.get(&r.id).copied().unwrap_or(0.0);
        if trust >= t.usual_trust
            && let Some(recipe) = usual_recipe(w, content, &id)
        {
            return OrderKind::Usual { wish: id, recipe };
        }
        return OrderKind::Wish { wish: id };
    }
    let chance = if super::pacing::quiet_hour(w, content) {
        t.quiet_hour.wish_share
    } else if w.mode == Mode::Sandbox {
        t.sandbox_wish_chance
    } else {
        t.wish_chance_on(w.night)
    };
    if !force_wish && !w.rng.chance(chance) {
        return OrderKind::Plain;
    }
    let generic: Vec<(&WishDef, f32)> = wishes
        .iter()
        .filter(|x| {
            x.regular().is_none()
                && (x.who.is_empty() || x.who.iter().any(|a| a == archetype))
                && x.min_party <= size
        })
        .map(|x| (x, wish_weight(w, content, x)))
        .filter(|x| x.1 > 0.0)
        .collect();
    match pick(w, generic) {
        Some(wish) => OrderKind::Wish { wish },
        None => OrderKind::Plain,
    }
}

/// "The usual": the wish's favourite recipe if she can make it, else her best good match.
pub(crate) fn usual_recipe(w: &World, content: &Content, wish: &str) -> Option<String> {
    let def = content.handson.wish(wish)?;
    for f in &def.favourite {
        if let Some(r) = w.recipe_def(content, f)
            && recipe_available(w, content, r)
        {
            return Some(f.clone());
        }
    }
    let good = content.handson.tuning.match_tiers.good;
    let matches = matches_for(w, content, def);
    // the usual for a wish that names a dish is that dish
    let named = matches.iter().find(|(id, m)| {
        *m >= good
            && w.recipe_def(content, id)
                .and_then(|r| names_recipe(content, def, r))
                .unwrap_or(false)
    });
    named
        .or_else(|| matches.iter().find(|(_, m)| *m >= good))
        .map(|(r, _)| r.clone())
}

/// The best plain recipe for a handed-off wish (starter / grandma's-card recipes only).
pub(crate) fn hand_off_recipe(w: &World, content: &Content, wish: &str) -> Option<(String, f32)> {
    let def = content.handson.wish(wish)?;
    matches_for(w, content, def).into_iter().find(|(r, _)| {
        content
            .handson
            .recipe(r)
            .is_some_and(|r| matches!(r.unlock, RecipeUnlock::Starter | RecipeUnlock::WithMenu))
    })
}

// ---------------------------------------------------------------------------------------------
// Serving
// ---------------------------------------------------------------------------------------------

fn reaction_key(content: &Content, regular: Option<&str>, tier: MatchTier, item: u32) -> String {
    let r = &content.handson.wishes.reactions;
    if let Some(reg) = regular
        && r.regulars.iter().any(|x| x == reg)
    {
        return tier.regular_key(reg);
    }
    tier.generic_key(1 + item % r.generic_per_tier.max(1))
}

fn emote_for(tier: MatchTier) -> SimEmote {
    match tier {
        MatchTier::Perfect => SimEmote::Sparkle,
        MatchTier::Good => SimEmote::Heart,
        MatchTier::Okay => SimEmote::Smile,
        MatchTier::NotQuite => SimEmote::Coffee,
    }
}

/// Hands-on part of `guests::item_served`: satisfaction / trust / tip factor / reaction for
/// the item `ii` of party `pi`. `by_karolina` = she carried it (rhythm).
pub(crate) fn on_served(w: &mut World, content: &Content, ii: usize, pi: usize, by_karolina: bool) {
    let t = &content.handson.tuning;
    let s = t.serve;
    let item = w.items[ii].clone();
    let regular = w.parties[pi].regular.clone();
    let rhythm = if by_karolina {
        super::pacing::rhythm_on_serve(w, content)
    } else {
        1.0
    };
    let classic = w.difficulty == Difficulty::Classic;
    let mut sat_add = 0.0;
    let factor;
    if regular.is_some() {
        w.hands.tally.regular_items += 1;
    }
    match item.origin.clone() {
        ItemOrigin::Player {
            recipe,
            quality: q,
            match_score: m,
            mode,
        } => {
            w.hands.tally.karolina_items += 1;
            if mode == BrewMode::ByHand {
                w.hands.tally.by_hand += 1;
            }
            if regular.is_some() {
                w.hands.tally.regular_items_karolina += 1;
            }
            // recipe stats & mastery
            let mut newly_practised = false;
            if let Some(st) = w.hands.knowledge.recipes.get_mut(&recipe) {
                st.served += 1;
                st.best_quality = st.best_quality.max(q);
                if mode == BrewMode::ByHand && q >= t.practised_min_quality {
                    st.good_by_hand += 1;
                    if !st.practised && st.good_by_hand >= t.practised_after {
                        st.practised = true;
                        newly_practised = true;
                    }
                }
            }
            if newly_practised {
                w.emit(SimEvent::PractisedRecipe {
                    recipe: recipe.clone(),
                });
            }
            let credit = if item.wish { m } else { PLAIN_CREDIT };
            let tier = MatchTier::of(m, &t.match_tiers);
            sat_add += if item.wish && classic && tier == MatchTier::NotQuite {
                s.classic_not_quite
            } else {
                s.satisfaction_match * credit + s.satisfaction_quality * q
            };
            factor = (s.tip_base + s.tip_span * credit * (0.5 + 0.5 * q)) * rhythm;
            let shown = if item.wish { m } else { 1.0 };
            super::moments::napkin_on_serve(w, content, pi, shown, q);
            if item.wish {
                let wish = w.parties[pi].order_kind.wish().map(str::to_string);
                super::tool::note_wish(w, content, wish.as_deref(), &recipe, regular.as_deref(), m);
                super::signature::on_served(w, content, wish.as_deref(), &recipe);
            }
            super::tool::serve_highlights(
                w,
                content,
                regular.as_deref(),
                &recipe,
                shown,
                q,
                mode == BrewMode::ByHand,
                item.guest,
            );
            if let Some(r) = &regular {
                // a handover note asked to remember them
                let remembered = w.sys.fx.remember.remove(r).unwrap_or(0.0);
                let add = s.trust_base
                    + if item.wish { s.trust_match * m } else { 0.0 }
                    + s.trust_quality * q
                    + remembered;
                let tr = w.story.trust.entry(r.clone()).or_insert(0.0);
                *tr = (*tr + add).min(100.0);
                let st = w.story.regulars.entry(r.clone()).or_default();
                st.served_recipes.insert(recipe.clone());
                if item.wish
                    && let Some(v) = st.visits.last_mut()
                {
                    v.best_match = Some(v.best_match.map_or(m, |b| b.max(m)));
                }
            }
            if item.wish {
                w.hands.tally.wishes_served += 1;
                w.hands.tally.tiers[tier_index(tier)] += 1;
                react(w, content, &item, regular.as_deref(), tier);
            }
            w.emit(SimEvent::PlayerServed {
                guest: item.guest,
                recipe,
                match_score: if item.wish { m } else { 1.0 },
                quality: q,
            });
        }
        other => {
            let q = match other {
                ItemOrigin::Helper { quality } => quality,
                _ => t.helper_quality.base,
            };
            w.hands.tally.helper_items += 1;
            let credit = item.hand_off_match.unwrap_or(PLAIN_CREDIT);
            if let Some(mc) = item.hand_off_match {
                // a handed-off wish: the guest still reacts, no story credit
                let tier = MatchTier::of(mc, &t.match_tiers);
                sat_add += s.satisfaction_match * mc + s.satisfaction_quality * q;
                w.hands.tally.wishes_served += 1;
                w.hands.tally.tiers[tier_index(tier)] += 1;
                react(w, content, &item, regular.as_deref(), tier);
            }
            factor = (s.tip_base + s.tip_span * credit * (0.5 + 0.5 * q)) * rhythm;
        }
    }
    let p = &mut w.parties[pi];
    p.satisfaction = (p.satisfaction + sat_add).clamp(0.0, 100.0);
    p.tip_factors.push(factor);
}

fn react(
    w: &mut World,
    content: &Content,
    item: &OrderItem,
    regular: Option<&str>,
    tier: MatchTier,
) {
    let key = reaction_key(content, regular, tier, item.id);
    if let Some(g) = w.guests.iter_mut().find(|g| g.id == item.guest) {
        g.emote = Some((emote_for(tier), w.tick + 30));
    }
    w.emit(SimEvent::GuestReaction {
        guest: item.guest,
        tier,
        key,
    });
}

// ---------------------------------------------------------------------------------------------
// The ticket rail
// ---------------------------------------------------------------------------------------------

pub(crate) fn ticket_rail(w: &World, content: &Content) -> Vec<Ticket> {
    let me = super::pacing::owner_id(w);
    let helper_with = |d: Duty| -> Option<u32> {
        w.staff
            .iter()
            .find(|s| !s.owner && s.on_duty() && s.duties.contains(&d))
            .map(|s| s.id)
    };
    let owner_of = |by: Option<u32>, fallback: Option<u32>| -> TicketOwner {
        match by {
            Some(id) if Some(id) == me => TicketOwner::Karolina,
            Some(id) => TicketOwner::Helper(id),
            None => fallback.map_or(TicketOwner::Nobody, TicketOwner::Helper),
        }
    };
    let mut out = Vec::new();
    // ticks since `on_minute` last drained patience (1..=TICKS_PER_MINUTE): the bar runs on
    // between the once-a-minute steps, so it reads empty when the guest gets up (a 9-minute
    // queue wait used to jump the last 11 % at once — "left with time still on the ticket")
    let since_step = ((w.tick + TICKS_PER_MINUTE - 1) % TICKS_PER_MINUTE + 1) as f32
        / TICKS_PER_MINUTE as f32;
    for (pi, p) in w.parties.iter().enumerate() {
        if matches!(p.state, PartyState::Arriving | PartyState::Leaving) {
            continue;
        }
        let Some(leader) = p.leader() else { continue };
        let patience = if super::guests::wait_running(w, p) {
            p.patience - super::pacing::wait_rate(w, content, pi) * since_step
        } else {
            p.patience
        };
        // the guest's real timer: the patience the current wait started with (old saves:
        // the estimate from before `wait_full`)
        let full = if p.wait_full > 0.0 {
            p.wait_full.max(p.patience)
        } else if p.ordered_at.is_some() {
            p.patience_max * super::guests::food_patience(w)
        } else {
            p.patience_max
        };
        let wait_frac = if full > 0.0 {
            (1.0 - patience / full).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let base = |stage: TicketStage, guest: u32, owner: TicketOwner| Ticket {
            party: p.id,
            guest,
            stage,
            owner,
            wait_frac,
            regular: p.regular.clone(),
        };
        match p.state {
            PartyState::Queue | PartyState::WaitOrder => {
                let helper_can = (!p.order_kind.is_wish() || p.handed_off) && p.regular.is_none();
                let fb = if helper_can {
                    helper_with(Duty::Register)
                } else {
                    None
                };
                out.push(base(
                    TicketStage::Order,
                    leader,
                    owner_of(p.order_claim, fb),
                ));
            }
            PartyState::WaitFood | PartyState::Eating => {
                if p.wish_open
                    && let Some(wish) = p.order_kind.wish()
                {
                    out.push(base(
                        TicketStage::Wish {
                            wish: wish.to_string(),
                        },
                        leader,
                        TicketOwner::Karolina,
                    ));
                }
                for iid in &p.items {
                    let Some(i) = w.items.iter().find(|i| i.id == *iid) else {
                        continue;
                    };
                    match i.state {
                        ItemState::Pending | ItemState::Preparing => {
                            let fb = match w.item_job(content, i) {
                                Some(Job::Kitchen) => helper_with(Duty::Kitchen),
                                Some(_) => helper_with(Duty::Bar),
                                None => None,
                            };
                            out.push(base(
                                TicketStage::Make {
                                    item: i.id,
                                    menu: i.menu.clone(),
                                },
                                i.guest,
                                owner_of(i.by, fb),
                            ));
                        }
                        ItemState::Ready | ItemState::Carrying => {
                            let fb = helper_with(Duty::Serving);
                            out.push(base(
                                TicketStage::Serve {
                                    item: i.id,
                                    menu: i.menu.clone(),
                                },
                                i.guest,
                                owner_of(i.by, fb),
                            ));
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    if w.hands.rush.until.is_some() {
        out.sort_by(|a, b| b.wait_frac.total_cmp(&a.wait_frac));
    }
    out
}

/// Accessibility `assist_wishes` (default off): a wish nobody claimed for
/// `assist_wishes_after` wait-clock minutes is handed off to the helpers (match capped).
pub(crate) fn assist_wishes(w: &mut World, content: &Content) {
    if !w.hands.assist_wishes {
        return;
    }
    let after = content.handson.tuning.assist_wishes_after as f32;
    let makers = super::duties::duty_on_floor(w, Duty::Bar)
        || super::duties::duty_on_floor(w, Duty::Kitchen);
    if !makers {
        return;
    }
    let register = super::duties::duty_on_floor(w, Duty::Register);
    for pi in 0..w.parties.len() {
        let p = &w.parties[pi];
        if !p.order_kind.is_wish() || p.handed_off || p.wait_clock < after {
            continue;
        }
        match p.state {
            PartyState::WaitOrder | PartyState::Queue if p.order_claim.is_none() && register => {
                w.parties[pi].handed_off = true;
            }
            PartyState::WaitFood if p.wish_open => {
                super::player::hand_off_item(w, content, pi);
            }
            _ => {}
        }
    }
}
