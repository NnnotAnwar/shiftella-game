//! Validation and required i18n keys of the hands-on content (child of `validate`, so it can
//! reuse the core reference index).

use std::collections::{BTreeSet, HashSet};

use super::{Index, check_placeholders, duplicates};
use crate::content::handson::keys::UI_KEYS;
use crate::content::handson::logic::{XRef, visit_conds, visit_fx, walk_fx};
use crate::content::handson::*;
use crate::content::logic::Ref;
use crate::content::{Content, MUSIC_TRACKS};
use crate::i18n::I18n;

/// Ids of everything hands-on, for resolving [`XRef`]s.
struct HIndex<'a> {
    recipes: HashSet<&'a str>,
    ingredients: HashSet<&'a str>,
    moments: HashSet<&'a str>,
    records: HashSet<&'a str>,
    themes: HashSet<&'a str>,
    phases: HashSet<&'a str>,
    lattes: HashSet<&'a str>,
    napkins: HashSet<&'a str>,
}

impl<'a> HIndex<'a> {
    fn build(h: &'a HandsOn) -> HIndex<'a> {
        HIndex {
            // + the café's own drink, composed in the game (`sim::signature`)
            recipes: h
                .recipes
                .recipes
                .iter()
                .map(|r| r.id.as_str())
                .chain(std::iter::once(crate::content::handson::SIGNATURE_RECIPE))
                .collect(),
            ingredients: h
                .recipes
                .ingredients
                .iter()
                .map(|r| r.id.as_str())
                .collect(),
            moments: h.moments.iter().map(|m| m.id.as_str()).collect(),
            records: h.records.records.iter().map(|r| r.id.as_str()).collect(),
            themes: h.decor.themes.iter().map(|t| t.id.as_str()).collect(),
            phases: h.seasons.phases.iter().map(|p| p.id.as_str()).collect(),
            lattes: h
                .minigames
                .latte_patterns
                .iter()
                .map(|p| p.id.as_str())
                .collect(),
            napkins: h.extras.napkins.iter().map(|n| n.id.as_str()).collect(),
        }
    }
}

struct Ctx<'a, 'b> {
    ix: &'b Index<'a>,
    hx: &'b HIndex<'a>,
    i18n: &'b I18n,
    out: &'b mut Vec<String>,
}

impl Ctx<'_, '_> {
    fn xref(&mut self, r: XRef<'_>, ctx: &str) {
        let ok = match r {
            XRef::Core(core) => {
                self.ix.check(core, self.i18n, ctx, self.out);
                return;
            }
            XRef::Recipe(x) => self.hx.recipes.contains(x),
            XRef::Ingredient(x) => self.hx.ingredients.contains(x),
            XRef::Moment(x) => self.hx.moments.contains(x),
            XRef::Record(x) => self.hx.records.contains(x),
            XRef::Theme(x) => self.hx.themes.contains(x),
            XRef::SeasonPhase(x) => self.hx.phases.contains(x),
            XRef::Latte(x) => self.hx.lattes.contains(x),
            XRef::MusicTrack(x) => MUSIC_TRACKS.contains(&x),
            XRef::Napkin(x) => self.hx.napkins.contains(x),
        };
        if !ok {
            self.out.push(format!("{ctx}: unknown reference {r:?}"));
        }
    }

    fn conds(&mut self, list: &[Cond], ctx: &str) {
        let mut refs = Vec::new();
        visit_conds(list, &mut |r| refs.push(r));
        refs.into_iter().for_each(|r| self.xref(r, ctx));
    }

    fn fx(&mut self, list: &[Fx], ctx: &str) {
        let mut refs = Vec::new();
        visit_fx(list, &mut |r| refs.push(r));
        refs.into_iter().for_each(|r| self.xref(r, ctx));
    }

    fn push(&mut self, msg: String) {
        self.out.push(msg);
    }

    fn regular(&mut self, id: &str, ctx: &str) {
        if !self.ix.regulars.contains_key(id) {
            self.push(format!("{ctx}: unknown regular `{id}`"));
        }
    }

    fn staff(&mut self, id: &str, ctx: &str) {
        if !self.ix.staff.contains(id) {
            self.push(format!("{ctx}: unknown staff archetype `{id}`"));
        }
    }

    fn guest_or_regular(&mut self, id: &str, ctx: &str) {
        if !self.ix.guests.contains(id) && !self.ix.regulars.contains_key(id) {
            self.push(format!(
                "{ctx}: `{id}` is neither a guest archetype nor a regular"
            ));
        }
    }

    fn furniture(&mut self, id: &str, ctx: &str) {
        if !self.ix.furniture.contains(id) {
            self.push(format!("{ctx}: unknown furniture `{id}`"));
        }
    }

    fn tag(&mut self, tag: &str, ctx: &str) {
        if !self.ix.tags.contains(tag) {
            self.push(format!("{ctx}: unknown furniture tag `{tag}`"));
        }
    }

    fn unit(&mut self, v: f32, what: &str, ctx: &str) {
        if !(0.0..=1.0).contains(&v) {
            self.push(format!("{ctx}: {what} must be 0–1, got {v}"));
        }
    }
}

fn hex(c: &str) -> bool {
    c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|ch| ch.is_ascii_hexdigit())
}

/// Furniture tags written `theme:<id>` are valid when the theme exists.
pub(super) fn theme_tag_ok(c: &Content, tag: &str) -> bool {
    tag.strip_prefix("theme:")
        .is_some_and(|t| c.handson.theme(t).is_some())
}

pub(super) fn validate(c: &Content, i18n: &I18n, ix: &Index<'_>, out: &mut Vec<String>) {
    let h = &c.handson;
    let hx = HIndex::build(h);
    let mut v = Ctx {
        ix,
        hx: &hx,
        i18n,
        out,
    };

    // --- unique ids -------------------------------------------------------------------------
    duplicates(
        "ingredients",
        h.recipes.ingredients.iter().map(|x| x.id.as_str()),
        v.out,
    );
    duplicates(
        "recipes",
        h.recipes.recipes.iter().map(|x| x.id.as_str()),
        v.out,
    );
    duplicates(
        "wishes",
        h.wishes.wishes.iter().map(|x| x.id.as_str()),
        v.out,
    );
    duplicates("moments", h.moments.iter().map(|x| x.id.as_str()), v.out);
    duplicates(
        "records",
        h.records.records.iter().map(|x| x.id.as_str()),
        v.out,
    );
    duplicates(
        "themes",
        h.decor.themes.iter().map(|x| x.id.as_str()),
        v.out,
    );
    duplicates(
        "season phases",
        h.seasons.phases.iter().map(|x| x.id.as_str()),
        v.out,
    );
    duplicates(
        "latte patterns",
        h.minigames.latte_patterns.iter().map(|x| x.id.as_str()),
        v.out,
    );
    duplicates("tutorial", h.tutorial.iter().map(|x| x.id.as_str()), v.out);
    duplicates(
        "napkins",
        h.extras.napkins.iter().map(|x| x.id.as_str()),
        v.out,
    );
    for kind in MiniGameKind::ALL {
        duplicates(
            &format!("minigame {} variants", kind.id()),
            h.minigames.variants(kind).into_iter(),
            v.out,
        );
    }

    tuning(&mut v, h);
    recipes(&mut v, c);
    wishes(&mut v, c);
    moments(&mut v, c);
    minigames(&mut v, c);
    records(&mut v, c);
    seasons(&mut v, c);
    decor(&mut v, c);
    journal(&mut v, c);
    tutorial(&mut v, c);
    extras(&mut v, c);
    staff_and_guests(&mut v, c);
    app(&mut v, c);
    calendar(&mut v, c);

    // placeholders of fixed UI keys and data-declared templates
    for (key, ph) in UI_KEYS {
        let declared: Vec<String> = ph.iter().map(|s| s.to_string()).collect();
        check_placeholders(i18n, key, &declared, v.out);
    }
    for r in &h.journal.rules {
        if !r.kind.has_sub_captions() {
            check_placeholders(i18n, &r.kind.caption_key(None), &r.placeholders, v.out);
        }
    }
    for e in &h.tuning.rozpis.life_events {
        check_placeholders(
            i18n,
            &format!("app.request.{}", e.id),
            &e.placeholders,
            v.out,
        );
    }
    for o in &c.app.handover_options {
        check_placeholders(i18n, &o.key(), &o.placeholders, v.out);
    }
    for o in &c.app.insights {
        check_placeholders(i18n, &o.key(), &o.placeholders, v.out);
    }
    for k in PrefKind::IDS {
        let ph: &[&str] = match k {
            "day_off" => &["day"],
            "shift_pref" => &["shift"],
            "max_nights" | "min_nights" => &["n"],
            "works_with" | "avoid_with" => &["name"],
            _ => &[],
        };
        let declared: Vec<String> = ph.iter().map(|s| s.to_string()).collect();
        check_placeholders(i18n, &format!("pref.kind.{k}"), &declared, v.out);
    }
}

fn tuning(v: &mut Ctx, h: &HandsOn) {
    let t = &h.tuning;
    let ctx = "handson.ron";
    if !(0.05..=1.0).contains(&t.real_seconds_per_tick_x1) {
        v.push(format!("{ctx}: real_seconds_per_tick_x1 out of range"));
    }
    if !(0.3..=1.5).contains(&t.prep_tick_scale) {
        v.push(format!("{ctx}: prep_tick_scale out of range (0.3–1.5)"));
    }
    for (what, x) in [
        ("overlay_scale.cozy", t.overlay_scale.cozy),
        ("overlay_scale.classic", t.overlay_scale.classic),
        ("quick_quality", t.quick_quality),
        ("practised_quick_quality", t.practised_quick_quality),
        ("practised_min_quality", t.practised_min_quality),
        ("by_hand_work_share", t.by_hand_work_share),
        ("hand_off_match_cap", t.hand_off_match_cap),
        ("helper_quality.max", t.helper_quality.max),
        ("sandbox_wish_chance", t.sandbox_wish_chance),
        ("claimed_wait_rate", t.claimed_wait_rate),
        ("cozy_rush_wait_rate", t.cozy_rush_wait_rate),
        ("cozy_forgiveness.refill", t.cozy_forgiveness.refill),
        ("rhythm.max", t.rhythm.max),
        ("thanks_chance.guest", t.thanks_chance.guest),
        ("thanks_chance.regular", t.thanks_chance.regular),
        ("quiet_hour.wish_share", t.quiet_hour.wish_share),
        ("quiet_hour.lamp_dim", t.quiet_hour.lamp_dim),
    ] {
        v.unit(x, what, ctx);
    }
    let m = &t.match_tiers;
    if !(m.perfect > m.good && m.good > m.okay && m.okay > 0.0) {
        v.push(format!(
            "{ctx}: match tiers must be perfect > good > okay > 0"
        ));
    }
    let mut last = 0;
    for w in &t.wish_chance {
        v.unit(w.chance, "wish_chance", ctx);
        if w.from_night <= last && last != 0 {
            v.push(format!("{ctx}: wish_chance must be sorted by from_night"));
        }
        last = w.from_night;
    }
    if t.wish_chance.first().map(|w| w.from_night) != Some(1) {
        v.push(format!("{ctx}: wish_chance must start at night 1"));
    }
    let acts: BTreeSet<u32> = t.moments_per_night.iter().map(|b| b.act).collect();
    let story_acts: BTreeSet<u32> = (1..=4).collect();
    if acts != story_acts {
        v.push(format!("{ctx}: moments_per_night needs acts 1–4"));
    }
    for b in &t.moments_per_night {
        if b.range.0 > b.range.1 || b.range.1 > t.moments_max_per_night {
            v.push(format!("{ctx}: bad moment budget for act {}", b.act));
        }
    }
    if !v.ix.traits.contains(t.capacity.trait_bonus.as_str()) {
        v.push(format!(
            "{ctx}: capacity trait `{}` unknown",
            t.capacity.trait_bonus
        ));
    }
    if t.capacity.base == 0 || t.capacity.base > t.capacity.max {
        v.push(format!("{ctx}: bad capacity base/max"));
    }
    if t.quiet_hour.from >= t.quiet_hour.to {
        v.push(format!("{ctx}: quiet hour must end after it starts"));
    }
    // rozpis
    let r = &t.rozpis;
    let rctx = "handson.ron rozpis";
    if r.rings.windows(2).any(|w| w[0].min <= w[1].min) || r.rings.is_empty() {
        v.push(format!(
            "{rctx}: rings must be sorted by min, highest first"
        ));
    }
    if r.week_icons.windows(2).any(|w| w[0] >= w[1]) {
        v.push(format!("{rctx}: week_icons must ascend"));
    }
    for (tr, _) in &r.pref_rules.from_traits {
        if !v.ix.traits.contains(tr.as_str()) {
            v.push(format!("{rctx}: unknown trait `{tr}`"));
        }
    }
    v.unit(r.pref_rules.need_chance, "need_chance", rctx);
    for e in &r.life_events {
        let ectx = format!("{rctx} life event `{}`", e.id);
        v.staff(&e.staff, &ectx);
        if !(1..=30).contains(&e.night) {
            v.push(format!("{ectx}: night out of 1..=30"));
        }
        if let Some(p) = &e.pref {
            pref(v, p, &ectx);
        }
    }
}

fn pref(v: &mut Ctx, p: &Pref, ctx: &str) {
    match &p.kind {
        PrefKind::WorksWith(s) | PrefKind::AvoidWith(s) => v.staff(s, ctx),
        PrefKind::MaxNights(n) | PrefKind::MinNights(n) if *n == 0 || *n > 7 => {
            v.push(format!("{ctx}: nights must be 1–7"))
        }
        _ => {}
    }
}

fn recipes(v: &mut Ctx, c: &Content) {
    let h = &c.handson;
    let rf = &h.recipes;
    let mut used: HashSet<&str> = HashSet::new();
    let mut sets: Vec<(BTreeSet<&str>, &str)> = Vec::new();
    for i in &rf.ingredients {
        let ctx = format!("ingredient `{}`", i.id);
        v.conds(i.unlock.conditions(), &ctx);
    }
    for r in &rf.recipes {
        let ctx = format!("recipe `{}`", r.id);
        if c.menu_item(&r.base_item).is_none() {
            v.push(format!("{ctx}: unknown base item `{}`", r.base_item));
        }
        if !(1..=3).contains(&r.ingredients.len()) {
            v.push(format!("{ctx}: needs 1–3 ingredients"));
        }
        if r.tags.is_empty() {
            v.push(format!("{ctx}: no taste tags"));
        }
        let tags: BTreeSet<_> = r.tags.iter().collect();
        if tags.len() != r.tags.len() {
            v.push(format!("{ctx}: duplicate taste tag"));
        }
        for ing in &r.ingredients {
            used.insert(ing);
            if !v.hx.ingredients.contains(ing.as_str()) {
                v.push(format!("{ctx}: unknown ingredient `{ing}`"));
            }
        }
        if r.ingredient_set().len() != r.ingredients.len() {
            v.push(format!("{ctx}: duplicate ingredient"));
        }
        for rev in &r.revealed {
            if !r.ingredients.contains(rev) {
                v.push(format!(
                    "{ctx}: revealed `{rev}` is not one of its ingredients"
                ));
            }
        }
        let set = r.ingredient_set();
        if let Some((_, other)) = sets.iter().find(|(s, _)| *s == set) {
            v.push(format!("{ctx}: same ingredient set as `{other}`"));
        }
        sets.push((set, &r.id));
        if r.variants.is_empty() {
            v.push(format!("{ctx}: needs at least one mini-game variant"));
        }
        for var in &r.variants {
            if !h.minigames.has_variant(r.minigame, var) {
                v.push(format!("{ctx}: {} has no variant `{var}`", r.minigame.id()));
            }
        }
        if r.minigame.tier() != Tier::Must {
            match (&r.fallback_minigame, &r.fallback_variant) {
                (Some(k), Some(var)) if k.tier() == Tier::Must => {
                    if !h.minigames.has_variant(*k, var) {
                        v.push(format!("{ctx}: fallback {} has no variant `{var}`", k.id()));
                    }
                }
                _ => v.push(format!(
                    "{ctx}: {} is SHOULD — needs a MUST fallback_minigame + fallback_variant",
                    r.minigame.id()
                )),
            }
        }
        match &r.unlock {
            RecipeUnlock::Taught { by, when } => {
                if !v.ix.regulars.contains_key(by.as_str()) && !v.ix.staff.contains(by.as_str()) {
                    v.push(format!("{ctx}: taught by unknown `{by}`"));
                }
                v.conds(when, &ctx);
            }
            RecipeUnlock::Story { when } => v.conds(when, &ctx),
            RecipeUnlock::WithMenu => {
                if c.menu_item(&r.base_item)
                    .is_some_and(|m| m.unlock_night <= 1 && m.unlock_flag.is_none())
                {
                    v.push(format!(
                        "{ctx}: WithMenu but the base item is on the menu from night 1"
                    ));
                }
            }
            RecipeUnlock::Starter | RecipeUnlock::Experiment => {}
        }
        if !r.experimentable && r.gated_hint.is_none() {
            v.push(format!("{ctx}: not experimentable — needs a gated_hint"));
        }
        if r.unlock == RecipeUnlock::Experiment && !r.experimentable {
            v.push(format!(
                "{ctx}: Experiment unlock but experimentable: false"
            ));
        }
        if !matches!(r.unlock, RecipeUnlock::Starter | RecipeUnlock::WithMenu) && r.hint.is_none() {
            v.push(format!(
                "{ctx}: needs a source hint for its silhouette card"
            ));
        }
    }
    for i in &rf.ingredients {
        if !used.contains(i.id.as_str()) {
            v.push(format!("ingredient `{}`: used by no recipe", i.id));
        }
    }
    let e = &rf.experiment;
    if e.min < 1 || e.min > e.max || e.nothing_lines == 0 {
        v.push("recipes.ron: bad experiment rules".into());
    }
}

fn wishes(v: &mut Ctx, c: &Content) {
    let h = &c.handson;
    for (key, items) in &h.wishes.line_items {
        let ctx = format!("wishes.ron line_items `{key}`");
        if items.is_empty() {
            v.push(format!("{ctx}: names no dish"));
        }
        for i in items {
            if c.menu_item(i).is_none() && !v.hx.recipes.contains(i.as_str()) {
                v.push(format!("{ctx}: `{i}` is neither a menu item nor a recipe"));
            }
        }
        if let Some(id) = key.strip_prefix("wish.")
            && !h.wishes.wishes.iter().any(|w| w.id == id)
        {
            v.push(format!("{ctx}: no such wish"));
        }
    }
    for w in &h.wishes.wishes {
        let ctx = format!("wish `{}`", w.id);
        if w.any != w.want.is_empty() {
            v.push(format!("{ctx}: either `any: true` or 1–3 `want` tags"));
        }
        if w.want.len() > 3 || w.avoid.len() > 2 {
            v.push(format!("{ctx}: at most 3 want / 2 avoid tags"));
        }
        if w.want.iter().any(|t| w.avoid.contains(t)) {
            v.push(format!("{ctx}: a tag is both wanted and avoided"));
        }
        for f in &w.favourite {
            if !v.hx.recipes.contains(f.as_str()) {
                v.push(format!("{ctx}: unknown favourite recipe `{f}`"));
            }
        }
        for f in &w.favourite_items {
            if c.menu_item(f).is_none() {
                v.push(format!("{ctx}: unknown favourite item `{f}`"));
            }
        }
        for who in &w.who {
            v.guest_or_regular(who, &ctx);
        }
        if let Some(reg) = w.regular() {
            v.regular(reg, &ctx);
            if w.who != [reg.to_string()] {
                v.push(format!("{ctx}: a personal wish must have who: [\"{reg}\"]"));
            }
        } else if w.who.iter().any(|x| v.ix.regulars.contains_key(x.as_str())) {
            v.push(format!(
                "{ctx}: a wish for a regular needs the `<regular>.` id prefix"
            ));
        }
        if let Some(l) = &w.latte
            && !v.hx.lattes.contains(l.as_str())
        {
            v.push(format!("{ctx}: unknown latte pattern `{l}`"));
        }
        if w.weight <= 0.0 || w.min_party == 0 {
            v.push(format!("{ctx}: weight must be > 0 and min_party ≥ 1"));
        }
        v.conds(&w.when, &ctx);
        for m in &w.weight_mods {
            v.conds(&m.when, &ctx);
        }
    }
    for r in &h.wishes.reactions.regulars {
        v.regular(r, "reactions");
    }
}

fn moments(v: &mut Ctx, c: &Content) {
    let h = &c.handson;
    let mut lists: Vec<&[Fx]> = Vec::new();
    for m in &h.moments {
        lists.extend(m.options.iter().map(|o| o.effects.as_slice()));
        lists.push(&m.on_expire);
    }
    lists.extend(h.tutorial.iter().map(|s| s.on_show.as_slice()));
    let mut spawned: HashSet<&str> = HashSet::new();
    for list in lists {
        walk_fx(list, &mut |e| {
            if let Fx::X(XEffect::SpawnMoment(m)) = e {
                spawned.insert(m.as_str());
            }
        });
    }
    for m in &h.moments {
        let ctx = format!("moment `{}`", m.id);
        if m.options.is_empty() || m.options.len() > 3 {
            v.push(format!("{ctx}: needs 1–3 options"));
        }
        duplicates(&ctx, m.options.iter().map(|o| o.id.as_str()), v.out);
        if let Some(e) = &m.from_event {
            v.ix.check(Ref::Event(e), v.i18n, &ctx, v.out);
        }
        if m.follow_up && !spawned.contains(m.id.as_str()) {
            v.push(format!("{ctx}: follow-up moment never spawned"));
        }
        if m.weight <= 0.0 || m.per_night_max == 0 || m.expires_minutes == 0 {
            v.push(format!(
                "{ctx}: weight, per_night_max and expires_minutes must be > 0"
            ));
        }
        match &m.anchor {
            MomentAnchor::Tag(t) => v.tag(t, &ctx),
            MomentAnchor::Furniture(f) => v.furniture(f, &ctx),
            MomentAnchor::Agent(AgentSelector::Guest(g)) => {
                for a in &g.archetypes {
                    if !v.ix.guests.contains(a.as_str()) {
                        v.push(format!("{ctx}: unknown guest archetype `{a}`"));
                    }
                }
                for r in g.regulars.iter().chain(&g.not_regulars) {
                    v.regular(r, &ctx);
                }
            }
            _ => {}
        }
        v.conds(&m.conditions, &ctx);
        v.fx(&m.on_expire, &ctx);
        for o in &m.options {
            let octx = format!("{ctx} option `{}`", o.id);
            v.fx(&o.effects, &octx);
            v.conds(&o.show_if, &octx);
            if o.locked_hint.is_some() && o.show_if.is_empty() {
                v.push(format!("{octx}: `locked_hint` without `show_if`"));
            }
            if let Some(Interaction::QuickBrews { recipe, .. }) = &o.interaction
                && !v.hx.recipes.contains(recipe.as_str())
            {
                v.push(format!("{octx}: unknown recipe `{recipe}`"));
            }
        }
    }
}

fn minigames(v: &mut Ctx, c: &Content) {
    let mg = &c.handson.minigames;
    let ctx = "minigames.ron";
    let q = &mg.quality_tiers;
    if !(q.beautiful > q.lovely && q.lovely > q.good && q.good > 0.0) {
        v.push(format!("{ctx}: quality tiers must descend"));
    }
    v.unit(mg.common.skip_quality, "skip_quality", ctx);
    v.unit(mg.common.relaxed_floor, "relaxed_floor", ctx);
    for p in &mg.latte_patterns {
        let pctx = format!("latte pattern `{}`", p.id);
        v.conds(&p.when, &pctx);
        if let Some(r) = &p.recipe
            && !v.hx.recipes.contains(r.as_str())
        {
            v.push(format!("{pctx}: unknown recipe `{r}`"));
        }
        if p.from_night.is_none() && p.when.is_empty() && p.recipe.is_none() && !p.granted {
            v.push(format!("{pctx}: no way to unlock it"));
        }
    }
    for lv in &mg.latte_art.variants {
        if let Some(p) = &lv.pattern
            && !v.hx.lattes.contains(p.as_str())
        {
            v.push(format!(
                "{ctx}: latte variant `{}` uses unknown pattern `{p}`",
                lv.variant
            ));
        }
    }
    for t in &mg.tea_steep {
        if !(1.0..=5.0).contains(&t.target) {
            v.push(format!(
                "{ctx}: tea `{}` target must be on the 1–5 scale",
                t.variant
            ));
        }
        if !TEA_KINDS.contains(&t.tea_kind.as_str()) {
            v.push(format!(
                "{ctx}: tea `{}` kind `{}` unknown",
                t.variant, t.tea_kind
            ));
        }
    }
    for p in &mg.plate {
        if let Some(d) = &p.dish
            && !PLATE_DISHES.contains(&d.as_str())
        {
            v.push(format!("{ctx}: plate `{}` dish `{d}` unknown", p.variant));
        }
    }
    for m in &mg.mulled_spice {
        if m.spices.iter().any(|s| !m.tray.contains(s)) {
            v.push(format!(
                "{ctx}: mulled `{}` spices must be on the tray",
                m.variant
            ));
        }
    }
}

fn records(v: &mut Ctx, c: &Content) {
    let rf = &c.handson.records;
    let ctx = "records.ron";
    for t in [
        &rf.house.calm,
        &rf.house.rush,
        &rf.house.quiet,
        &rf.house.day,
    ] {
        if !MUSIC_TRACKS.contains(&t.as_str()) {
            v.push(format!("{ctx}: unknown house track `{t}`"));
        }
    }
    let moods: Vec<HallMood> = rf.moods.iter().map(|m| m.mood).collect();
    for mood in HallMood::ALL {
        if moods.iter().filter(|m| **m == mood).count() != 1 {
            v.push(format!(
                "{ctx}: mood {mood:?} must have exactly one effect entry"
            ));
        }
    }
    for m in &rf.moods {
        for a in m.satisfaction.iter().chain(&m.patience) {
            for g in &a.archetypes {
                if !v.ix.guests.contains(g.as_str()) {
                    v.push(format!("{ctx}: mood {:?}: unknown archetype `{g}`", m.mood));
                }
            }
        }
        for (mo, _) in &m.moment_weight {
            if !v.hx.moments.contains(mo.as_str()) {
                v.push(format!("{ctx}: mood {:?}: unknown moment `{mo}`", m.mood));
            }
        }
    }
    let starters = rf.records.iter().filter(|r| r.is_starter()).count();
    if starters != 3 {
        v.push(format!(
            "{ctx}: exactly 3 starter records, found {starters}"
        ));
    }
    for r in &rf.records {
        let rctx = format!("record `{}`", r.id);
        if !MUSIC_TRACKS.contains(&r.track.as_str()) {
            v.push(format!("{rctx}: track `{}` not in MUSIC_TRACKS", r.track));
        }
        if r.seconds <= 0.0 {
            v.push(format!("{rctx}: seconds must be > 0"));
        }
        if !r.is_starter() && r.sandbox_night.is_none() {
            v.push(format!("{rctx}: non-starters need a sandbox_night"));
        }
        if !hex(&r.sleeve.color) {
            v.push(format!("{rctx}: sleeve colour must be #RRGGBB"));
        }
        for reg in r.liked_by.iter().chain(&r.disliked_by) {
            v.regular(reg, &rctx);
        }
        if r.liked_by.iter().any(|x| r.disliked_by.contains(x)) {
            v.push(format!("{rctx}: a regular both likes and dislikes it"));
        }
        v.conds(r.unlock.conditions(), &rctx);
    }
}

fn seasons(v: &mut Ctx, c: &Content) {
    let s = &c.handson.seasons;
    let total = c.story.nights_total;
    let mut covered = vec![0u32; total as usize + 1];
    for p in &s.phases {
        let ctx = format!("season phase `{}`", p.id);
        if let Some((a, b)) = p.story_nights {
            if a == 0 || a > b || b > total {
                v.push(format!("{ctx}: bad story_nights"));
            } else {
                (a..=b).for_each(|n| covered[n as usize] += 1);
            }
        }
        v.unit(p.visuals.frost, "frost", &ctx);
        for l in &p.latte {
            if !v.hx.lattes.contains(l.as_str()) {
                v.push(format!("{ctx}: unknown latte `{l}`"));
            }
        }
        for m in p
            .moments
            .iter()
            .chain(p.moment_weight.iter().map(|(m, _)| m))
        {
            if !v.hx.moments.contains(m.as_str()) {
                v.push(format!("{ctx}: unknown moment `{m}`"));
            }
        }
        for r in &p.specials {
            if !v.hx.recipes.contains(r.as_str()) {
                v.push(format!("{ctx}: unknown special `{r}`"));
            }
        }
    }
    if covered[1..].iter().any(|n| *n != 1) {
        v.push(format!(
            "seasons.ron: story phases must cover nights 1..={total} exactly once"
        ));
    }
    for sp in &s.sandbox {
        if !v.hx.phases.contains(sp.phase.as_str()) {
            v.push(format!("seasons.ron: sandbox phase `{}` unknown", sp.phase));
        }
        if sp.nights.0 == 0 || sp.nights.0 > sp.nights.1 || sp.nights.1 > 30 {
            v.push(format!("seasons.ron: sandbox {:?} bad nights", sp.season));
        }
    }
}

fn decor(v: &mut Ctx, c: &Content) {
    let d = &c.handson.decor;
    if d.themes.iter().filter(|t| t.default).count() != 1 {
        v.push("decor.ron: exactly one default theme".into());
    }
    for t in &d.themes {
        let ctx = format!("theme `{}`", t.id);
        if t.default && (t.unlock != Unlock::Starter || t.price != 0) {
            v.push(format!("{ctx}: the default theme is a free starter"));
        }
        if let Some(tag) = &t.accent_tag
            && tag != &format!("theme:{}", t.id)
        {
            v.push(format!("{ctx}: accent_tag must be `theme:{}`", t.id));
        }
        let tn = &t.tints;
        for col in [
            &tn.wall,
            &tn.wall_trim,
            &tn.floor,
            &tn.textile,
            &tn.lampshade,
        ] {
            if !hex(col) {
                v.push(format!("{ctx}: colour `{col}` must be #RRGGBB"));
            }
        }
        for (p, _) in &t.coziness_in {
            if !v.hx.phases.contains(p.as_str()) {
                v.push(format!("{ctx}: unknown phase `{p}`"));
            }
        }
        if !(0.0..=10.0).contains(&t.coziness) {
            v.push(format!("{ctx}: coziness must be 0–10"));
        }
        for perk in &t.perks {
            match perk {
                ThemePerk::RegularTrustOnEnter { regulars, .. } => {
                    regulars.iter().for_each(|r| v.regular(r, &ctx))
                }
                ThemePerk::ArchetypeSatisfaction { archetypes, .. } => {
                    for a in archetypes {
                        if !v.ix.guests.contains(a.as_str()) {
                            v.push(format!("{ctx}: unknown archetype `{a}`"));
                        }
                    }
                }
                ThemePerk::MissedBy { regular, .. } => v.regular(regular, &ctx),
                ThemePerk::BeautyPerLamp(_) => {}
            }
        }
        v.conds(t.unlock.conditions(), &ctx);
        if t.unlock != Unlock::Starter && t.sandbox_night.is_none() && !t.default {
            // seasonal themes unlock via flags in both modes
            if t.unlock.conditions().is_empty() && t.unlock != Unlock::Granted {
                v.push(format!("{ctx}: needs a sandbox_night"));
            }
        }
    }
    let cz = &d.coziness;
    let sum: f32 = cz.weights.iter().map(|(_, w)| w).sum();
    if (sum - 100.0).abs() > 0.01 {
        v.push(format!("decor.ron: coziness weights sum to {sum}, not 100"));
    }
    for comp in CozyComponent::ALL {
        if cz.weights.iter().filter(|(x, _)| *x == comp).count() != 1 {
            v.push(format!(
                "decor.ron: coziness weight for {comp:?} missing or repeated"
            ));
        }
    }
    if cz.levels.windows(2).any(|w| w[0] >= w[1]) {
        v.push("decor.ron: coziness levels must ascend".into());
    }
    for (t, _) in &cz.touches {
        if let Touch::Furniture(f) = t {
            v.furniture(f, "decor.ron touches");
        }
    }
}

fn journal(v: &mut Ctx, c: &Content) {
    let j = &c.handson.journal;
    for kind in HighlightKind::ALL {
        if j.rule(kind).is_none() {
            v.push(format!("journal.ron: no rule for {kind:?}"));
        }
    }
    v.tag(&j.fallback.focus_tag, "journal.ron fallback");
    for s in &j.scenes {
        v.ix.check(Ref::Cutscene(s), v.i18n, "journal.ron scenes", v.out);
    }
    for m in &j.moments {
        if !v.hx.moments.contains(m.as_str()) {
            v.push(format!("journal.ron: unknown moment `{m}`"));
        }
    }
    // every moment that captures a MomentWarm highlight needs its caption, and vice versa
    for m in &c.handson.moments {
        let mut warm = false;
        for o in &m.options {
            walk_fx(&o.effects, &mut |e| {
                if matches!(e, Fx::X(XEffect::Highlight(HighlightKind::MomentWarm))) {
                    warm = true;
                }
            });
        }
        if warm != j.moments.contains(&m.id) {
            v.push(format!(
                "journal.ron: moment `{}` {} a MomentWarm highlight but is {}listed in `moments`",
                m.id,
                if warm { "captures" } else { "doesn't capture" },
                if warm { "not " } else { "" }
            ));
        }
    }
    v.unit(j.full_house_share, "full_house_share", "journal.ron");
}

fn tutorial(v: &mut Ctx, c: &Content) {
    let mut seen: HashSet<&str> = HashSet::new();
    for s in &c.handson.tutorial {
        let ctx = format!("tutorial `{}`", s.id);
        if !(1..=c.story.nights_total).contains(&s.night) {
            v.push(format!("{ctx}: night out of range"));
        }
        match &s.trigger {
            TutTrigger::After(prev) if !seen.contains(prev.as_str()) => v.push(format!(
                "{ctx}: `After` must name an earlier step, not `{prev}`"
            )),
            TutTrigger::RegularArrives(r) => v.regular(r, &ctx),
            _ => {}
        }
        match &s.anchor {
            TutAnchor::Guest(GuestPick::Regular(r)) => v.regular(r, &ctx),
            TutAnchor::Widget(w) if !TUTORIAL_WIDGETS.contains(&w.as_str()) => {
                v.push(format!("{ctx}: widget `{w}` not in TUTORIAL_WIDGETS"))
            }
            TutAnchor::Tag(t) => v.tag(t, &ctx),
            TutAnchor::Furniture(f) => v.furniture(f, &ctx),
            TutAnchor::Moment(m) if !v.hx.moments.contains(m.as_str()) => {
                v.push(format!("{ctx}: unknown moment `{m}`"))
            }
            _ => {}
        }
        v.fx(&s.on_show, &ctx);
        seen.insert(&s.id);
    }
}

fn extras(v: &mut Ctx, c: &Content) {
    let e = &c.handson.extras;
    let ctx = "extras.ron";
    v.ix.check(Ref::Counter(&e.cat.counter), v.i18n, ctx, v.out);
    if e.cat.levels.windows(2).any(|w| w[0].level >= w[1].level)
        || e.cat.levels.first().map(|l| l.level) != Some(0)
        || e.cat
            .levels
            .last()
            .is_some_and(|l| l.level as i32 > e.cat.max)
    {
        v.push(format!("{ctx}: cat levels must ascend from 0 up to max"));
    }
    for l in &e.cat.levels {
        for p in &l.perks {
            match p {
                CatPerk::Naming { moment } if !v.hx.moments.contains(moment.as_str()) => {
                    v.push(format!("{ctx}: unknown naming moment `{moment}`"))
                }
                CatPerk::Latte(x) if !v.hx.lattes.contains(x.as_str()) => {
                    v.push(format!("{ctx}: unknown latte `{x}`"))
                }
                _ => {}
            }
        }
    }
    v.ix.check(Ref::Var("cat_name", "none"), v.i18n, ctx, v.out);
    for n in &e.cat.names {
        v.ix.check(Ref::Var("cat_name", n), v.i18n, ctx, v.out);
    }
    if e.tip_goals
        .windows(2)
        .any(|w| w[0].lifetime >= w[1].lifetime)
    {
        v.push(format!("{ctx}: tip goals must ascend"));
    }
    for g in &e.tip_goals {
        if g.choices.len() != 2 {
            v.push(format!("{ctx}: tip goal {} needs 2 choices", g.lifetime));
        }
        for f in &g.choices {
            v.furniture(f, ctx);
        }
    }
    for n in &e.napkins {
        for r in &n.who {
            v.regular(r, &format!("napkin `{}`", n.id));
        }
    }
    for r in &e.napkin_rewards {
        v.fx(&r.effects, ctx);
    }
    for r in &e.rituals {
        v.regular(&r.regular, ctx);
    }
}

fn staff_and_guests(v: &mut Ctx, c: &Content) {
    // regulars' arc stages may use hands-on conditions (`when`)
    for r in &c.regulars {
        for st in &r.stages {
            let ctx = format!("regular `{}` stage {}", r.id, st.stage);
            v.conds(&st.when, &ctx);
        }
    }
    for s in &c.staff {
        let ctx = format!("staff `{}`", s.id);
        if s.default_duty.is_none() {
            v.push(format!("{ctx}: needs a default_duty"));
        }
        if s.prefs.len() > 3 {
            v.push(format!("{ctx}: at most 3 prefs"));
        }
        for p in &s.prefs {
            pref(v, p, &ctx);
            if let PrefKind::WorksWith(x) | PrefKind::AvoidWith(x) = &p.kind
                && x == &s.id
            {
                v.push(format!("{ctx}: a pref can't name themselves"));
            }
        }
    }
    for g in &c.guests {
        let ctx = format!("guest `{}`", g.id);
        for r in g.music_likes.iter().chain(&g.music_dislikes) {
            if !v.hx.records.contains(r.as_str()) {
                v.push(format!("{ctx}: unknown record `{r}`"));
            }
        }
        if g.music_likes.iter().any(|r| g.music_dislikes.contains(r)) {
            v.push(format!("{ctx}: likes and dislikes the same record"));
        }
        let ws: BTreeSet<String> = g
            .weather_mult
            .iter()
            .map(|(w, _)| format!("{w:?}"))
            .collect();
        if ws.len() != g.weather_mult.len() {
            v.push(format!("{ctx}: weather listed twice"));
        }
        if g.weather_mult.iter().any(|(_, m)| *m <= 0.0) {
            v.push(format!("{ctx}: weather multipliers must be > 0"));
        }
    }
    for f in &c.furniture {
        for t in &f.tags {
            if t.starts_with("theme:") && !theme_tag_ok(c, t) {
                v.push(format!("furniture `{}`: unknown theme tag `{t}`", f.id));
            }
        }
    }
}

fn app(v: &mut Ctx, c: &Content) {
    let app = &c.app;
    for item in &app.checklists {
        let ctx = format!("checklist `{}`", item.id);
        if let Some(a) = &item.anchor {
            v.tag(a, &ctx);
        }
        v.fx(&item.extra_done, &ctx);
        v.fx(&item.extra_missed, &ctx);
        v.fx(&item.hand_done, &ctx);
        v.fx(&item.perfect_done, &ctx);
        v.fx(&item.habit, &ctx);
    }
    for r in &app.rituals {
        v.fx(&r.effects, &format!("ritual {:?}", r.list));
        v.fx(&r.perfect, &format!("ritual {:?} perfect", r.list));
    }
    // hands-on rituals: every item Karolina may do has its gesture (handson.ron → rituals)
    let rit = &c.handson.tuning.rituals;
    for item in &app.checklists {
        if rit.gesture(&item.id).is_none() {
            v.push(format!(
                "handson.ron rituals: checklist `{}` has no gesture",
                item.id
            ));
        }
    }
    for (key, g) in &rit.gestures {
        let known = app.checklists.iter().any(|i| &i.id == key) || key.starts_with("equip.");
        if !known {
            v.push(format!("handson.ron rituals: unknown key `{key}`"));
        }
        let bad = match g {
            RitualGesture::Needle { green: (a, b) } => {
                !(0.05..=0.95).contains(a) || b <= a || *b > 1.0
            }
            RitualGesture::Count { coins, total } => {
                coins.is_empty() || coins.contains(&0) || total.0 == 0 || total.1 < total.0
            }
            RitualGesture::Strokes { count, .. } => !(1..=8).contains(count),
            RitualGesture::Shelf { items, .. } => !(1..=5).contains(items),
            RitualGesture::Drop { band } => !(0.05..=0.5).contains(band),
            RitualGesture::Sort { coins } => !(1..=8).contains(coins),
            RitualGesture::Lines { count } => !(2..=4).contains(count),
            RitualGesture::Cord | RitualGesture::Slide { .. } | RitualGesture::Stir => false,
        };
        if bad {
            v.push(format!("handson.ron rituals: `{key}` has odd parameters"));
        }
    }
    if app.rituals.len() != 2 {
        v.push("shiftella.ron: one Opening and one Closing ritual".into());
    }
    for a in &app.announcements {
        v.fx(&a.effects, &format!("announcement `{}`", a.id));
    }
    for b in &app.bot {
        for a in &b.actions {
            if let Some(BotHandler::ApplyMoment(m)) = &a.handler
                && !v.hx.moments.contains(m.as_str())
            {
                v.push(format!("bot `{}`: unknown moment `{m}`", b.id));
            }
        }
    }
    duplicates(
        "handover options",
        app.handover_options.iter().map(|o| o.id.as_str()),
        v.out,
    );
    for o in &app.handover_options {
        let ctx = format!("handover option `{}`", o.id);
        v.conds(&o.when, &ctx);
        v.fx(&o.next_night, &ctx);
    }
    duplicates(
        "insights",
        app.insights.iter().map(|o| o.id.as_str()),
        v.out,
    );
    match &app.payroll_rules {
        None => v.push("shiftella.ron: payroll_rules missing".into()),
        Some(p) => {
            if p.splits.iter().filter(|s| s.default).count() != 1 {
                v.push("shiftella.ron: exactly one default tip split".into());
            }
            for s in &p.splits {
                v.unit(s.crew_share, "crew_share", "payroll");
            }
        }
    }
}

fn calendar(v: &mut Ctx, c: &Content) {
    let Some(cal) = &c.story.calendar else {
        v.push("story.ron: calendar missing".into());
        return;
    };
    if !cal.start.is_valid() {
        v.push("story.ron: calendar start is not a date".into());
    }
    for n in &c.story.nights {
        let d = cal.date_of_night(n.night);
        if d.weekday() != n.weekday {
            v.push(format!(
                "story night {}: weekday {:?} but {}.{}.{} is a {:?}",
                n.night,
                n.weekday,
                d.day,
                d.month,
                d.year,
                d.weekday()
            ));
        }
    }
    for d in &cal.days {
        if !(1..=c.story.nights_total).contains(&d.night) {
            v.push(format!("calendar day `{}`: night out of range", d.id));
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Required keys
// ---------------------------------------------------------------------------------------------

pub(super) fn required_keys(c: &Content) -> Vec<String> {
    let h = &c.handson;
    let mut k: Vec<String> = UI_KEYS.iter().map(|(key, _)| key.to_string()).collect();
    k.extend(Taste::ALL.iter().map(|t| t.key()));
    k.extend(Family::ALL.iter().map(|f| f.key()));
    k.extend(BookTab::ALL.iter().map(|t| t.key().to_string()));
    for i in &h.recipes.ingredients {
        k.push(i.name_key());
    }
    for r in &h.recipes.recipes {
        k.extend([r.name_key(), r.desc_key(), r.lore_key()]);
        k.extend(r.teach_key());
        k.extend(r.hint.clone());
        k.extend(r.gated_hint.clone());
    }
    for n in 1..=h.recipes.experiment.nothing_lines {
        k.push(format!("experiment.nothing.{n}"));
    }
    for kind in MiniGameKind::ALL
        .into_iter()
        .filter(|k| k.tier() == Tier::Must)
    {
        k.push(kind.title_key());
    }
    k.extend(
        QualityTiers::LABELS
            .iter()
            .map(|l| format!("minigame.quality.{l}")),
    );
    for p in &h.minigames.latte_patterns {
        k.push(format!("latte.{}", p.id));
    }
    for w in &h.wishes.wishes {
        k.push(w.key());
    }
    for tier in MatchTier::ALL {
        for n in 1..=h.wishes.reactions.generic_per_tier {
            k.push(tier.generic_key(n));
        }
        for r in &h.wishes.reactions.regulars {
            k.push(tier.regular_key(r));
        }
    }
    for m in &h.moments {
        k.extend([m.title_key(), m.text_key()]);
        for o in &m.options {
            k.extend([m.option_key(&o.id), m.result_key(&o.id)]);
            k.extend(o.locked_hint.clone());
        }
        if !m.on_expire.is_empty() {
            k.push(m.expired_key());
        }
    }
    for d in Duty::ALL {
        k.extend([d.name_key(), d.desc_key(), d.digest_key()]);
    }
    k.extend(HallMood::ALL.iter().map(|m| m.key()));
    for r in &h.records.records {
        k.extend([r.title_key(), r.artist_key()]);
        if !r.is_starter() {
            k.extend([r.hint_key(), r.source_key()]);
        }
        k.extend(r.liked_by.iter().map(|x| r.liked_key(x)));
        k.extend(r.disliked_by.iter().map(|x| r.disliked_key(x)));
    }
    for p in &h.seasons.phases {
        k.extend([p.name_key(), p.toast_key()]);
    }
    if let Some(cal) = &c.story.calendar {
        k.extend(
            cal.days
                .iter()
                .map(crate::content::story::StoryCalendar::day_key),
        );
    }
    for t in &h.decor.themes {
        k.extend([t.name_key(), t.desc_key()]);
        for p in &t.perks {
            if let ThemePerk::MissedBy { key, .. } = p {
                k.push(key.clone());
            }
        }
    }
    for comp in CozyComponent::ALL {
        k.extend([comp.name_key(), comp.tip_key()]);
    }
    for n in 1..=h.decor.coziness.levels.len() + 1 {
        k.push(format!("atmo.level.{n}"));
    }
    // journal captions
    for r in &h.journal.rules {
        if !r.kind.has_sub_captions() {
            k.push(r.kind.caption_key(None));
        }
    }
    for reg in &c.regulars {
        if reg.id == "guest_anwar" {
            continue;
        }
        for st in reg.stages.iter().filter(|s| s.stage >= 1) {
            k.push(HighlightKind::ArcBeat.caption_key(Some(&format!("{}.{}", reg.id, st.stage))));
        }
    }
    for s in &h.journal.scenes {
        k.push(HighlightKind::StoryScene.caption_key(Some(s)));
    }
    for m in &h.journal.moments {
        k.push(HighlightKind::MomentWarm.caption_key(Some(m)));
    }
    for s in &h.tutorial {
        k.push(s.key());
    }
    let e = &h.extras;
    for l in &e.cat.levels {
        k.push(format!("cat.level.{}", l.level));
    }
    for n in &e.cat.names {
        k.push(format!("cat.name.{n}"));
    }
    for n in &e.napkins {
        k.push(format!("thanks.{}", n.id));
    }
    for r in &e.rituals {
        k.push(format!("ritual.{}", r.regular));
    }
    for n in 1..=e.mug_lines {
        k.push(format!("karolina.mug.{n}"));
    }
    // rozpis
    k.extend(PrefKind::IDS.iter().map(|id| format!("pref.kind.{id}")));
    let rz = &h.tuning.rozpis;
    for r in &rz.rings {
        k.push(format!("rozpis.ring.{}", r.id));
    }
    for n in 1..=rz.week_icons.len() + 1 {
        k.push(format!("rozpis.week.{n}"));
    }
    for (_, sp) in &rz.pref_rules.from_traits {
        k.push(sp.quote_key());
    }
    for sp in &rz.pref_rules.random {
        k.push(sp.quote_key());
    }
    for ev in &rz.life_events {
        k.push(format!("app.request.{}", ev.id));
        if let Some(p) = &ev.pref {
            k.push(p.quote.clone());
        }
    }
    for s in &c.staff {
        k.extend(s.prefs.iter().map(|p| p.quote.clone()));
    }
    k.push(h.tuning.quiet_hour.toast.clone());
    // app
    let app = &c.app;
    for r in &app.rituals {
        k.push(r.toast_key());
        if !r.perfect.is_empty() {
            k.push(r.perfect_key());
        }
    }
    // the ritual gestures' first-time hints
    for g in h.tuning.rituals.gestures.values() {
        k.push(format!("ritual.hint.{}", g.id()));
    }
    // by hand (what it adds) and grandma's habits (her note, the perk)
    for item in &app.checklists {
        if !item.hand_done.is_empty() || !item.perfect_done.is_empty() {
            k.push(format!("ritual.hand.{}", item.id));
        }
        if !item.habit.is_empty() {
            k.push(format!("habit.{}.note", item.id));
            k.push(format!("habit.{}.perk", item.id));
        }
    }
    for o in &app.handover_options {
        k.push(o.key());
    }
    for o in &app.insights {
        k.push(o.key());
    }
    if let Some(p) = &app.payroll_rules {
        for s in &p.splits {
            k.push(format!("app.payroll.split.{}", s.split.id()));
        }
    }
    // keys written inside effects (Toast)
    let mut lists: Vec<&[Fx]> = Vec::new();
    for m in &h.moments {
        lists.push(&m.on_expire);
        lists.extend(m.options.iter().map(|o| o.effects.as_slice()));
    }
    lists.extend(h.tutorial.iter().map(|s| s.on_show.as_slice()));
    lists.extend(app.handover_options.iter().map(|o| o.next_night.as_slice()));
    for list in lists {
        walk_fx(list, &mut |e| {
            if let Fx::Core(crate::content::Effect::Toast(key)) = e {
                k.push(key.clone());
            }
        });
    }
    k
}
