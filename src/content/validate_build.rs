//! Validation and required i18n keys of package C's content (`build_wishes.ron`,
//! `corners.ron`, `attic.ron`). Child of `validate`, so it reuses the core reference index.

use std::collections::HashSet;

use super::{Index, check_placeholders, duplicates};
use crate::content::build::{ATTIC_FLAG, WishWho};
use crate::content::handson::logic::{XRef, visit_conds, visit_fx};
use crate::content::handson::{Cond, Fx};
use crate::content::{BuildCategory, Content};
use crate::i18n::I18n;

fn xrefs(
    c: &Content,
    i18n: &I18n,
    ix: &Index<'_>,
    refs: Vec<XRef<'_>>,
    ctx: &str,
    out: &mut Vec<String>,
) {
    let h = &c.handson;
    for r in refs {
        let ok = match r {
            XRef::Core(core) => {
                ix.check(core, i18n, ctx, out);
                continue;
            }
            XRef::Recipe(x) => h.recipe(x).is_some(),
            XRef::Ingredient(x) => h.ingredient(x).is_some(),
            XRef::Moment(x) => h.moment(x).is_some(),
            XRef::Record(x) => h.record(x).is_some(),
            XRef::Theme(x) => h.theme(x).is_some(),
            XRef::SeasonPhase(x) => h.phase(x).is_some(),
            XRef::Napkin(x) => h.extras.napkins.iter().any(|n| n.id == x),
            XRef::Latte(_) | XRef::MusicTrack(_) => true,
        };
        if !ok {
            out.push(format!("{ctx}: unknown reference {r:?}"));
        }
    }
}

fn conds(
    c: &Content,
    i18n: &I18n,
    ix: &Index<'_>,
    list: &[Cond],
    ctx: &str,
    out: &mut Vec<String>,
) {
    let mut refs = Vec::new();
    visit_conds(list, &mut |r| refs.push(r));
    xrefs(c, i18n, ix, refs, ctx, out);
}

fn fx(c: &Content, i18n: &I18n, ix: &Index<'_>, list: &[Fx], ctx: &str, out: &mut Vec<String>) {
    let mut refs = Vec::new();
    visit_fx(list, &mut |r| refs.push(r));
    xrefs(c, i18n, ix, refs, ctx, out);
}

pub(super) fn validate(c: &Content, i18n: &I18n, ix: &Index<'_>, out: &mut Vec<String>) {
    let b = &c.build;
    duplicates(
        "build wishes",
        b.wishes.wishes.iter().map(|w| w.id.as_str()),
        out,
    );
    duplicates(
        "corners",
        b.corners.corners.iter().map(|w| w.id.as_str()),
        out,
    );
    duplicates("attic", b.attic.pieces.iter().map(|w| w.id.as_str()), out);
    if !ix.flags.contains(ATTIC_FLAG) {
        out.push(format!(
            "story.ron: flag `{ATTIC_FLAG}` (attic pieces) is not declared"
        ));
    }

    // --- requests ---------------------------------------------------------------------------
    for w in &b.wishes.wishes {
        let ctx = format!("build_wishes.ron `{}`", w.id);
        match &w.who {
            WishWho::Regular(r) => {
                if !ix.regulars.contains_key(r.as_str()) {
                    out.push(format!("{ctx}: unknown regular `{r}`"));
                }
            }
            WishWho::Staff(Some(s)) => {
                if !ix.staff.contains(s.as_str()) {
                    out.push(format!("{ctx}: unknown staff archetype `{s}`"));
                }
            }
            WishWho::Staff(None) => {}
            WishWho::Guests(g) => {
                if !ix.guests.contains(g.as_str()) {
                    out.push(format!("{ctx}: unknown guest archetype `{g}`"));
                }
            }
        }
        if w.def.is_none() && w.tag.is_none() {
            out.push(format!(
                "{ctx}: needs a `def` or a `tag` (the build card badge)"
            ));
        }
        if let Some(d) = &w.def
            && !ix.furniture.contains(d.as_str())
        {
            out.push(format!("{ctx}: unknown furniture `{d}`"));
        }
        if let Some(t) = &w.tag
            && !ix.tags.contains(t.as_str())
        {
            out.push(format!("{ctx}: unknown furniture tag `{t}`"));
        }
        if w.done.is_empty() {
            out.push(format!(
                "{ctx}: `done` is empty (it would be fulfilled at once)"
            ));
        }
        conds(c, i18n, ix, &w.appear, &ctx, out);
        conds(c, i18n, ix, &w.done, &ctx, out);
        fx(c, i18n, ix, &w.reward, &ctx, out);
    }

    // --- corners ----------------------------------------------------------------------------
    for k in &b.corners.corners {
        let ctx = format!("corners.ron `{}`", k.id);
        if k.needs.is_empty() {
            out.push(format!("{ctx}: no needs"));
        }
        if k.needs.first().is_some_and(|n| n.count != 1) {
            out.push(format!("{ctx}: the first need is the anchor (count 1)"));
        }
        for n in &k.needs {
            if n.defs.is_empty() && n.tags.is_empty() {
                out.push(format!("{ctx}: a need without defs or tags"));
            }
            if n.count == 0 {
                out.push(format!("{ctx}: a need with count 0"));
            }
            for d in &n.defs {
                if !ix.furniture.contains(d.as_str()) {
                    out.push(format!("{ctx}: unknown furniture `{d}`"));
                }
            }
            for t in &n.tags {
                if !ix.tags.contains(t.as_str()) {
                    out.push(format!("{ctx}: unknown furniture tag `{t}`"));
                }
            }
        }
        for a in &k.attracts {
            if !ix.guests.contains(a.as_str()) {
                out.push(format!("{ctx}: unknown guest archetype `{a}`"));
            }
        }
        if !(0.0..=10.0).contains(&k.bonus) {
            out.push(format!("{ctx}: bonus {} out of 0–10", k.bonus));
        }
        if !(1.0..=2.0).contains(&k.attract_mul) {
            out.push(format!("{ctx}: attract_mul {} out of 1–2", k.attract_mul));
        }
        if k.radius == 0 || k.radius > 4 {
            out.push(format!("{ctx}: radius {} out of 1–4", k.radius));
        }
    }
    if b.corners.max_bonus <= 0.0 {
        out.push("corners.ron: max_bonus must be > 0".into());
    }
    for (key, ph) in crate::content::build::UI_KEYS {
        let declared: Vec<String> = ph.iter().map(|s| s.to_string()).collect();
        check_placeholders(i18n, key, &declared, out);
    }

    // --- attic -----------------------------------------------------------------------------
    let a = &b.attic;
    if a.per_week == 0 {
        out.push("attic.ron: per_week must be > 0".into());
    }
    let mut seen = HashSet::new();
    for p in &a.pieces {
        let ctx = format!("attic.ron `{}`", p.id);
        seen.insert(p.id.as_str());
        let Some(d) = c.furniture_def(&p.id) else {
            out.push(format!("{ctx}: no furniture `{}` in furniture.ron", p.id));
            continue;
        };
        if d.unlock_flag.as_deref() != Some(ATTIC_FLAG) {
            out.push(format!(
                "{ctx}: the furniture must be hidden with `unlock_flag: \"{ATTIC_FLAG}\"`"
            ));
        }
        if !d.unique {
            out.push(format!("{ctx}: attic pieces are unique"));
        }
        if matches!(d.category, BuildCategory::Floors | BuildCategory::Structure) {
            out.push(format!(
                "{ctx}: floors / structure can't come from the attic"
            ));
        }
        if p.price == 0 || p.price >= d.price {
            out.push(format!(
                "{ctx}: second-hand price {} must be below the new price {}",
                p.price, d.price
            ));
        }
    }
    for f in &c.furniture {
        if f.unlock_flag.as_deref() == Some(ATTIC_FLAG) && !seen.contains(f.id.as_str()) {
            out.push(format!(
                "furniture.ron `{}`: hidden as an attic piece but not in attic.ron",
                f.id
            ));
        }
    }
    let br = &a.back_room;
    if br.price == 0 {
        out.push("attic.ron back_room: price must be > 0".into());
    }
    for (id, _, _, _) in &br.pieces {
        if !ix.furniture.contains(id.as_str()) {
            out.push(format!("attic.ron back_room: unknown furniture `{id}`"));
        }
    }
}

pub(super) fn required_keys(c: &Content) -> Vec<String> {
    let b = &c.build;
    let mut keys = Vec::new();
    for w in &b.wishes.wishes {
        keys.extend([w.reason_key(), w.ask_key(), w.thanks_key()]);
    }
    for k in &b.corners.corners {
        keys.extend([k.name_key(), k.desc_key(), format!("corner.{}.what", k.id)]);
    }
    for p in &b.attic.pieces {
        keys.push(p.story_key());
    }
    keys.extend(
        crate::content::build::UI_KEYS
            .iter()
            .map(|k| k.0.to_string()),
    );
    keys
}
