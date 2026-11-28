//! Cross-reference and i18n validation of the whole content set.

use std::collections::{BTreeSet, HashMap, HashSet};

use super::defs::{
    BuildCategory, FURNITURE_TAGS, Job, Need, RelTarget, SPRITES, TraitEffect, Weather, Weekday,
};
use super::logic::{Ref, visit_conditions, visit_effects};
use super::story::{CutsceneStep, CutsceneTrigger, Focus, Place, walk_steps};
use super::world::EventTrigger;
use super::{Content, MUSIC_TRACKS, SFX_IDS};
use crate::i18n::{I18n, Lang, placeholders};

#[path = "validate_handson.rs"]
mod hands;

#[path = "validate_build.rs"]
mod build_v;

/// UI keys the content module itself relies on (units, enums).
fn builtin_keys() -> Vec<String> {
    let mut keys: Vec<String> = Job::ALL
        .iter()
        .flat_map(|j| [j.name_key().to_string(), j.skill_key().to_string()])
        .collect();
    keys.extend(Need::ALL.iter().map(|n| n.name_key().to_string()));
    keys.extend(Weather::ALL.iter().map(|w| w.name_key().to_string()));
    keys.extend(BuildCategory::ALL.iter().map(|b| b.name_key().to_string()));
    for d in Weekday::ALL {
        keys.push(d.short_key());
        keys.push(d.long_key());
    }
    for k in [
        "ui.unit.minutes_short",
        "ui.unit.hours_short",
        "ui.unit.hours_minutes_short",
        "story_ui.journal.notes",
        "story_ui.journal.not_found",
    ] {
        keys.push(k.to_string());
    }
    keys.extend(crate::cutscene::locked::KEYS.iter().map(|k| k.to_string()));
    keys
}

pub(super) fn required_keys(c: &Content) -> Vec<String> {
    let mut keys = builtin_keys();
    for t in &c.traits {
        keys.push(t.name_key());
        keys.push(t.desc_key());
    }
    for s in &c.staff {
        keys.extend([s.role_key(), s.bio_key(), s.likes_key(), s.dislikes_key()]);
    }
    for m in &c.menu {
        keys.push(m.name_key());
        keys.push(m.desc_key());
    }
    for g in &c.guests {
        keys.push(g.name_key());
        keys.push(g.desc_key());
    }
    for r in &c.regulars {
        keys.push(r.name_key());
        keys.push(r.bio_key());
        for st in &r.stages {
            keys.extend(st.lines.iter().cloned());
        }
        keys.extend(r.ambient_lines.iter().map(|l| l.key.clone()));
    }
    for f in &c.furniture {
        keys.push(f.name_key());
        keys.push(f.desc_key());
    }
    for e in &c.events {
        keys.push(e.title_key());
        keys.push(e.text_key());
        for ch in &e.choices {
            keys.push(e.choice_key(&ch.id));
            keys.push(e.result_key(&ch.id));
        }
    }
    let s = &c.story;
    for n in &s.nights {
        keys.push(n.title_key());
        keys.push(n.hint_key());
    }
    for f in &s.night_fallbacks {
        keys.extend(f.title.iter().chain(&f.hint).cloned());
    }
    for a in &s.acts {
        keys.push(format!("story.act.{}.title", a.act));
    }
    for n in &s.notes {
        keys.push(format!("note.{}.text", n.id));
        keys.push(format!("note.{}.where", n.id));
    }
    for e in &s.endings {
        keys.push(format!("ending.{}.title", e.id));
        keys.push(format!("ending.{}.summary", e.id));
        for v in &e.variants {
            keys.push(format!("ending.{}.variant.{}", e.id, v.id));
        }
    }
    keys.push(format!("ending.extra.{}.title", s.ending_extra.id));
    for i in &s.items {
        keys.push(format!("item.{i}.name"));
        keys.push(format!("item.{i}.desc"));
    }
    for m in &c.cutscenes.cast {
        keys.push(format!("cast.{}.name", m.id));
    }
    for d in &c.difficulties {
        keys.push(d.name_key());
        keys.push(d.desc_key());
    }
    for ch in &c.sandbox.characters {
        keys.push(format!("sandbox.character.{}.name", ch.id));
        keys.push(format!("sandbox.character.{}.desc", ch.id));
    }
    for i in &c.sandbox.institutions {
        keys.push(format!("sandbox.institution.{}", i.id));
    }
    let app = &c.app;
    for a in &app.announcements {
        keys.push(format!("app.announcement.{}.title", a.id));
        keys.push(format!("app.announcement.{}.body", a.id));
    }
    for i in &app.checklists {
        keys.push(format!("app.checklist.{}", i.id));
    }
    for b in &app.bot {
        keys.push(format!("app.bot.{}", b.id));
        for a in &b.actions {
            keys.push(format!("app.bot.{}.action.{}", b.id, a.id));
        }
    }
    for l in &app.digest {
        keys.push(format!("app.digest.{}", l.id));
    }
    for l in &app.payroll {
        keys.push(format!("app.payroll.{}", l.id));
    }
    for l in &app.handover {
        keys.push(format!("app.handover.{}", l.id));
    }
    keys.push(c.trains.announce_arrival.clone());
    keys.push(c.trains.announce_delay.clone());
    for t in &c.trains.trains {
        keys.push(train_key(&t.id));
    }
    // Keys written in data.
    for n in &s.nights {
        for b in &n.beats {
            match &b.what {
                super::story::BeatKind::Line { text_key, .. } => keys.push(text_key.clone()),
                super::story::BeatKind::Hint(k) => keys.push(k.clone()),
                _ => {}
            }
        }
    }
    for cs in &c.cutscenes.cutscenes {
        walk_steps(&cs.steps, &mut |step| match step {
            CutsceneStep::Narration { text_key } | CutsceneStep::Line { text_key, .. } => {
                keys.push(text_key.clone())
            }
            CutsceneStep::Choice { options } => {
                keys.extend(options.iter().map(|o| o.label_key.clone()));
                keys.extend(options.iter().filter_map(|o| o.locked_hint.clone()));
            }
            _ => {}
        });
    }
    keys.extend(hands::required_keys(c));
    keys.extend(build_v::required_keys(c));
    let mut seen = HashSet::new();
    keys.retain(|k| seen.insert(k.clone()));
    keys
}

/// `train.os_2141.name` style key for a train's display line ("Os 2141 Adamov → Praha-Holešovice").
pub fn train_key(id: &str) -> String {
    format!("train.{}.name", id.to_ascii_lowercase().replace(' ', "_"))
}

struct Index<'a> {
    menu: HashSet<&'a str>,
    guests: HashSet<&'a str>,
    regulars: HashMap<&'a str, BTreeSet<u32>>,
    events: HashSet<&'a str>,
    cutscenes: HashSet<&'a str>,
    notes: HashSet<&'a str>,
    traits: HashSet<&'a str>,
    staff: HashSet<&'a str>,
    furniture: HashSet<&'a str>,
    tags: HashSet<&'a str>,
    flags: HashSet<String>,
    vars: HashMap<&'a str, HashSet<&'a str>>,
    counters: HashSet<String>,
    items: HashSet<&'a str>,
    difficulties: HashSet<&'a str>,
    institutions: HashSet<&'a str>,
    bots: HashSet<&'a str>,
    speakers: HashSet<&'a str>,
    spots: HashSet<&'a str>,
    trains: HashSet<&'a str>,
}

fn duplicates<'a>(what: &str, ids: impl Iterator<Item = &'a str>, out: &mut Vec<String>) {
    let mut seen = HashSet::new();
    for id in ids {
        if !seen.insert(id) {
            out.push(format!("{what}: duplicate id `{id}`"));
        }
    }
}

impl<'a> Index<'a> {
    fn build(c: &'a Content) -> Index<'a> {
        let mut flags: HashSet<String> = c.story.flags.iter().cloned().collect();
        let mut counters: HashSet<String> = c.story.counters.iter().cloned().collect();
        for r in &c.regulars {
            flags.insert(format!("arc.{}.good", r.id));
            flags.insert(format!("arc.{}.bad", r.id));
            counters.insert(format!("arc.{}", r.id));
        }
        let mut speakers: HashSet<&str> = HashSet::new();
        speakers.extend(c.regulars.iter().map(|r| r.id.as_str()));
        speakers.extend(c.staff.iter().map(|s| s.id.as_str()));
        speakers.extend(c.cutscenes.cast.iter().map(|m| m.id.as_str()));
        Index {
            menu: c.menu.iter().map(|m| m.id.as_str()).collect(),
            guests: c.guests.iter().map(|g| g.id.as_str()).collect(),
            regulars: c
                .regulars
                .iter()
                .map(|r| (r.id.as_str(), r.stages.iter().map(|s| s.stage).collect()))
                .collect(),
            events: c.events.iter().map(|e| e.id.as_str()).collect(),
            cutscenes: c
                .cutscenes
                .cutscenes
                .iter()
                .map(|x| x.id.as_str())
                .collect(),
            notes: c.story.notes.iter().map(|n| n.id.as_str()).collect(),
            traits: c.traits.iter().map(|t| t.id.as_str()).collect(),
            staff: c.staff.iter().map(|s| s.id.as_str()).collect(),
            furniture: c.furniture.iter().map(|f| f.id.as_str()).collect(),
            tags: FURNITURE_TAGS.iter().copied().collect(),
            flags,
            vars: c
                .story
                .vars
                .iter()
                .map(|v| (v.id.as_str(), v.values.iter().map(String::as_str).collect()))
                .collect(),
            counters,
            items: c.story.items.iter().map(String::as_str).collect(),
            difficulties: c.difficulties.iter().map(|d| d.id.as_str()).collect(),
            institutions: c
                .sandbox
                .institutions
                .iter()
                .map(|i| i.id.as_str())
                .collect(),
            bots: c.app.bot.iter().map(|b| b.id.as_str()).collect(),
            speakers,
            spots: c.cutscenes.spots.keys().map(String::as_str).collect(),
            trains: c.trains.trains.iter().map(|t| t.id.as_str()).collect(),
        }
    }

    fn check(&self, r: Ref<'_>, i18n: &I18n, ctx: &str, out: &mut Vec<String>) {
        let ok = match r {
            Ref::Menu(x) => self.menu.contains(x),
            Ref::Guest(x) => self.guests.contains(x),
            Ref::Regular(x) => self.regulars.contains_key(x),
            Ref::Event(x) => self.events.contains(x),
            Ref::Cutscene(x) => self.cutscenes.contains(x),
            Ref::Note(x) => self.notes.contains(x),
            Ref::Trait(x) => self.traits.contains(x),
            Ref::Staff(x) => self.staff.contains(x),
            Ref::Furniture(x) => self.furniture.contains(x),
            Ref::FurnitureTag(x) => self.tags.contains(x),
            Ref::Flag(x) => self.flags.contains(x),
            Ref::Var(v, val) => self.vars.get(v).is_some_and(|vals| vals.contains(val)),
            Ref::Counter(x) => self.counters.contains(x),
            Ref::Item(x) => self.items.contains(x),
            Ref::Difficulty(x) => self.difficulties.contains(x),
            Ref::Institution(x) => self.institutions.contains(x),
            Ref::Bot(x) => self.bots.contains(x),
            Ref::TextKey(k) => i18n.has_all(k),
        };
        if !ok {
            out.push(format!("{ctx}: unknown reference {r:?}"));
        }
    }

    fn flag(&self, flag: &Option<String>, ctx: &str, out: &mut Vec<String>) {
        if let Some(f) = flag
            && !self.flags.contains(f.as_str())
        {
            out.push(format!("{ctx}: flag `{f}` is not declared in story.ron"));
        }
    }
}

pub(super) fn validate(c: &Content, i18n: &I18n) -> Vec<String> {
    let mut out = Vec::new();
    let ix = Index::build(c);

    // --- unique ids ---------------------------------------------------------------------
    duplicates("traits", c.traits.iter().map(|x| x.id.as_str()), &mut out);
    duplicates("staff", c.staff.iter().map(|x| x.id.as_str()), &mut out);
    duplicates(
        "name pools",
        c.name_pools.iter().map(|x| x.id.as_str()),
        &mut out,
    );
    duplicates("menu", c.menu.iter().map(|x| x.id.as_str()), &mut out);
    duplicates("guests", c.guests.iter().map(|x| x.id.as_str()), &mut out);
    duplicates(
        "regulars",
        c.regulars.iter().map(|x| x.id.as_str()),
        &mut out,
    );
    duplicates(
        "furniture",
        c.furniture.iter().map(|x| x.id.as_str()),
        &mut out,
    );
    duplicates("events", c.events.iter().map(|x| x.id.as_str()), &mut out);
    duplicates(
        "cutscenes",
        c.cutscenes.cutscenes.iter().map(|x| x.id.as_str()),
        &mut out,
    );
    duplicates(
        "cast",
        c.cutscenes.cast.iter().map(|x| x.id.as_str()),
        &mut out,
    );
    duplicates(
        "notes",
        c.story.notes.iter().map(|x| x.id.as_str()),
        &mut out,
    );
    duplicates(
        "trains",
        c.trains.trains.iter().map(|x| x.id.as_str()),
        &mut out,
    );
    duplicates("bot", c.app.bot.iter().map(|x| x.id.as_str()), &mut out);
    duplicates("flags", c.story.flags.iter().map(String::as_str), &mut out);
    let all_speakers = c
        .regulars
        .iter()
        .map(|r| r.id.as_str())
        .chain(c.staff.iter().map(|s| s.id.as_str()))
        .chain(c.cutscenes.cast.iter().map(|m| m.id.as_str()));
    duplicates("speaker ids (regulars/staff/cast)", all_speakers, &mut out);

    // --- traits -------------------------------------------------------------------------
    for t in &c.traits {
        let ctx = format!("trait `{}`", t.id);
        for other in t.conflicts.iter().chain(&t.irritated_by) {
            if !ix.traits.contains(other.as_str()) {
                out.push(format!("{ctx}: unknown trait `{other}`"));
            }
        }
        for other in &t.conflicts {
            if let Some(o) = c.trait_def(other)
                && !o.conflicts.contains(&t.id)
            {
                out.push(format!("{ctx}: conflict with `{other}` is not symmetric"));
            }
        }
        for e in &t.effects {
            if let TraitEffect::Relationship {
                with: RelTarget::Trait(x),
                ..
            } = e
                && !ix.traits.contains(x.as_str())
            {
                out.push(format!("{ctx}: relationship target `{x}` unknown"));
            }
        }
    }

    // --- staff --------------------------------------------------------------------------
    for s in &c.staff {
        let ctx = format!("staff `{}`", s.id);
        for t in s.trait_pool.iter().chain(&s.story_traits) {
            if !ix.traits.contains(t.as_str()) {
                out.push(format!("{ctx}: unknown trait `{t}`"));
            }
        }
        for (i, a) in s.story_traits.iter().enumerate() {
            for b in &s.story_traits[i + 1..] {
                if c.trait_def(a).is_some_and(|t| t.conflicts.contains(b)) {
                    out.push(format!("{ctx}: story traits `{a}` and `{b}` conflict"));
                }
            }
        }
        if s.trait_pool.len() < 2 {
            out.push(format!("{ctx}: trait pool needs at least 2 traits"));
        }
        for p in &s.name_pools {
            if !c.name_pools.iter().any(|n| &n.id == p) {
                out.push(format!("{ctx}: unknown name pool `{p}`"));
            }
        }
        for job in Job::ALL {
            if !(0.0..=10.0).contains(&s.skills.get(job)) {
                out.push(format!("{ctx}: skill {job:?} out of 0–10"));
            }
        }
    }
    for p in &c.name_pools {
        if p.female_first.is_empty() || p.male_first.is_empty() || p.male_last.is_empty() {
            out.push(format!("name pool `{}`: empty list", p.id));
        }
        if !p.female_last.is_empty() && p.female_last.len() != p.male_last.len() {
            out.push(format!(
                "name pool `{}`: female_last must pair 1:1 with male_last",
                p.id
            ));
        }
    }

    // --- menu & furniture ---------------------------------------------------------------
    for m in &c.menu {
        let ctx = format!("menu `{}`", m.id);
        if !m.station.starts_with("station:") || !ix.tags.contains(m.station.as_str()) {
            out.push(format!("{ctx}: bad station tag `{}`", m.station));
        }
        if c.furniture_with_tag(&m.station).next().is_none() {
            out.push(format!("{ctx}: no furniture provides `{}`", m.station));
        }
        ix.flag(&m.unlock_flag, &ctx, &mut out);
    }
    for f in &c.furniture {
        let ctx = format!("furniture `{}`", f.id);
        if !SPRITES.contains(&f.sprite.as_str()) {
            out.push(format!(
                "{ctx}: sprite `{}` not in the render contract",
                f.sprite
            ));
        }
        for t in &f.tags {
            if !ix.tags.contains(t.as_str()) && !t.starts_with("theme:") {
                out.push(format!("{ctx}: unknown tag `{t}`"));
            }
        }
        if f.size.0 == 0 || f.size.1 == 0 {
            out.push(format!("{ctx}: zero size"));
        }
        ix.flag(&f.unlock_flag, &ctx, &mut out);
    }

    // --- guests -------------------------------------------------------------------------
    for g in &c.guests {
        let ctx = format!("guest `{}`", g.id);
        if g.hour_weights.len() != 8 {
            out.push(format!("{ctx}: hour_weights needs 8 values (22:00..06:00)"));
        }
        if !g.weekday_mult.is_empty() && g.weekday_mult.len() != 7 {
            out.push(format!("{ctx}: weekday_mult needs 7 values"));
        }
        for m in &g.favourites {
            if !ix.menu.contains(m.as_str()) {
                out.push(format!("{ctx}: unknown favourite `{m}`"));
            }
        }
        if let Some(i) = &g.institution
            && !ix.institutions.contains(i.as_str())
        {
            out.push(format!("{ctx}: unknown institution `{i}`"));
        }
        if g.group.0 == 0 || g.group.0 > g.group.1 {
            out.push(format!("{ctx}: bad group range"));
        }
    }

    // --- regulars -----------------------------------------------------------------------
    for r in &c.regulars {
        let ctx = format!("regular `{}`", r.id);
        if !ix.guests.contains(r.base.as_str()) {
            out.push(format!("{ctx}: unknown base guest `{}`", r.base));
        }
        for o in &r.orders {
            if !ix.menu.contains(o.as_str()) {
                out.push(format!("{ctx}: unknown order `{o}`"));
            }
        }
        if let Some(seat) = &r.seat
            && !ix.tags.contains(seat.as_str())
        {
            out.push(format!("{ctx}: unknown seat tag `{seat}`"));
        }
        visit_conditions(&r.absent_if, &mut |x| ix.check(x, i18n, &ctx, &mut out));
        if r.story_nights
            .iter()
            .any(|n| !(1..=c.story.nights_total).contains(n))
        {
            out.push(format!(
                "{ctx}: story_nights outside 1..={}",
                c.story.nights_total
            ));
        }
        for l in &r.ambient_lines {
            visit_conditions(&l.when, &mut |x| ix.check(x, i18n, &ctx, &mut out));
        }
        if r.sandbox_nights.contains(&0) {
            out.push(format!("{ctx}: sandbox_nights start at 1"));
        }
        if !(0.0..=1.0).contains(&r.sandbox_chance) {
            out.push(format!("{ctx}: sandbox_chance must be 0–1"));
        }
        for (i, st) in r.stages.iter().enumerate() {
            let sctx = format!("{ctx} stage {}", st.stage);
            if st.stage != i as u32 {
                out.push(format!("{sctx}: stages must be numbered 0,1,2… in order"));
            }
            if st.nights.0 > st.nights.1 {
                out.push(format!("{sctx}: bad night range"));
            }
            visit_conditions(&st.conditions, &mut |x| ix.check(x, i18n, &sctx, &mut out));
            visit_effects(&st.effects, &mut |x| ix.check(x, i18n, &sctx, &mut out));
        }
    }

    // --- events -------------------------------------------------------------------------
    for e in &c.events {
        let ctx = format!("event `{}`", e.id);
        if !(1..=3).contains(&e.severity) {
            out.push(format!("{ctx}: severity must be 1–3"));
        }
        if e.trigger == EventTrigger::Random && e.weight <= 0.0 {
            out.push(format!("{ctx}: random events need weight > 0"));
        }
        visit_conditions(&e.conditions, &mut |x| ix.check(x, i18n, &ctx, &mut out));
        visit_effects(&e.effects, &mut |x| ix.check(x, i18n, &ctx, &mut out));
        for ch in &e.choices {
            let cctx = format!("{ctx} choice `{}`", ch.id);
            visit_effects(&ch.effects, &mut |x| ix.check(x, i18n, &cctx, &mut out));
            ix.flag(&ch.set_flag, &cctx, &mut out);
        }
        if let Some(d) = &e.default_choice
            && !e.choices.iter().any(|ch| &ch.id == d)
        {
            out.push(format!("{ctx}: default_choice `{d}` is not a choice"));
        }
    }

    // --- narrator -----------------------------------------------------------------------
    if c.narrator.night_curve.is_empty() {
        out.push("narrator: empty night curve".into());
    }

    // --- story --------------------------------------------------------------------------
    let s = &c.story;
    let nights: BTreeSet<u32> = s.nights.iter().map(|n| n.night).collect();
    let want: BTreeSet<u32> = (1..=s.nights_total).collect();
    if nights != want || s.nights.len() != s.nights_total as usize {
        out.push(format!(
            "story: nights must be exactly 1..={} once each",
            s.nights_total
        ));
    }
    let paid: i64 = s.payments.iter().map(|p| p.amount).sum();
    if paid != s.debt {
        out.push(format!("story: payments sum {paid} ≠ debt {}", s.debt));
    }
    if s.payments.iter().filter(|p| p.is_final).count() != 1 {
        out.push("story: exactly one payment must be final".into());
    }
    for v in &s.vars {
        if !v.values.contains(&v.default) {
            out.push(format!("story var `{}`: default not among values", v.id));
        }
    }
    for n in &s.nights {
        let ctx = format!("story night {}", n.night);
        if !s
            .acts
            .iter()
            .any(|a| a.act == n.act && (a.nights.0..=a.nights.1).contains(&n.night))
        {
            out.push(format!("{ctx}: act {} doesn't cover this night", n.act));
        }
        let mut last = None;
        for b in &n.beats {
            if last.is_some_and(|t| b.at < t) {
                out.push(format!("{ctx}: beats not sorted by time ({})", b.at));
            }
            last = Some(b.at);
            visit_conditions(&b.conditions, &mut |x| ix.check(x, i18n, &ctx, &mut out));
            use super::story::BeatKind::*;
            match &b.what {
                Line { speaker, .. } => {
                    if !ix.speakers.contains(speaker.as_str()) {
                        out.push(format!("{ctx}: unknown speaker `{speaker}`"));
                    }
                }
                Event(e) => ix.check(Ref::Event(e), i18n, &ctx, &mut out),
                Note(nid) => ix.check(Ref::Note(nid), i18n, &ctx, &mut out),
                PaymentDue => {
                    if s.payment_on(n.night).is_none() {
                        out.push(format!("{ctx}: PaymentDue but no payment that night"));
                    }
                }
                Train(t) => {
                    if !ix.trains.contains(t.as_str()) {
                        out.push(format!("{ctx}: unknown train `{t}`"));
                    }
                }
                Bot(b) => ix.check(Ref::Bot(b), i18n, &ctx, &mut out),
                Apply(effects) => {
                    visit_effects(effects, &mut |x| ix.check(x, i18n, &ctx, &mut out))
                }
                Hint(_) | Weather(_) => {}
            }
        }
    }
    for p in &s.payments {
        if !nights.contains(&p.night) {
            out.push(format!("story: payment on unknown night {}", p.night));
        } else if !s.night(p.night).is_some_and(|n| {
            n.beats
                .iter()
                .any(|b| matches!(b.what, super::story::BeatKind::PaymentDue))
        }) {
            out.push(format!(
                "story night {}: payment has no PaymentDue beat",
                p.night
            ));
        }
    }
    let numbers: BTreeSet<u32> = s.notes.iter().map(|n| n.number).collect();
    if numbers != (1..=s.notes.len() as u32).collect() {
        out.push("story: notes must be numbered 1..N".into());
    }
    for f in &s.night_fallbacks {
        let ctx = format!("story night {} fallback", f.night);
        if !nights.contains(&f.night) {
            out.push(format!("{ctx}: unknown night"));
        }
        if f.needs.is_empty() {
            out.push(format!("{ctx}: `needs` is empty (it would never apply)"));
        }
        if f.title.is_none() && f.hint.is_none() {
            out.push(format!("{ctx}: neither `title` nor `hint` is set"));
        }
        visit_conditions(&f.needs, &mut |x| ix.check(x, i18n, &ctx, &mut out));
    }
    for n in &s.notes {
        let ctx = format!("note `{}`", n.id);
        visit_conditions(&n.found_when, &mut |x| ix.check(x, i18n, &ctx, &mut out));
        visit_effects(&n.effects, &mut |x| ix.check(x, i18n, &ctx, &mut out));
    }
    if s.endings.len() != 2 {
        out.push(format!(
            "story: exactly 2 endings, found {}",
            s.endings.len()
        ));
    }
    let order = s.endings_in_order();
    if let Some(last) = order.last()
        && !last.conditions.is_empty()
    {
        out.push(format!(
            "ending `{}`: the last ending must be the fallback (no conditions)",
            last.id
        ));
    }
    for e in &s.endings {
        let ctx = format!("ending `{}`", e.id);
        visit_conditions(&e.conditions, &mut |x| ix.check(x, i18n, &ctx, &mut out));
        for v in &e.variants {
            visit_conditions(&v.conditions, &mut |x| ix.check(x, i18n, &ctx, &mut out));
        }
        match c.cutscene(&e.cutscene) {
            None => out.push(format!("{ctx}: unknown cutscene `{}`", e.cutscene)),
            Some(cs) if cs.trigger != CutsceneTrigger::Ending(e.id.clone()) => out.push(format!(
                "{ctx}: cutscene `{}` must have trigger Ending(\"{}\")",
                cs.id, e.id
            )),
            _ => {}
        }
    }
    let ex = &s.ending_extra;
    visit_conditions(&ex.conditions, &mut |x| {
        ix.check(x, i18n, "ending extra", &mut out)
    });
    ix.check(Ref::Cutscene(&ex.cutscene), i18n, "ending extra", &mut out);

    // --- cutscenes ----------------------------------------------------------------------
    for cs in &c.cutscenes.cutscenes {
        let ctx = format!("cutscene `{}`", cs.id);
        visit_conditions(&cs.conditions, &mut |x| ix.check(x, i18n, &ctx, &mut out));
        match &cs.trigger {
            CutsceneTrigger::NightStart(n)
            | CutsceneTrigger::NightEnd(n)
            | CutsceneTrigger::At { night: n, .. } => {
                if !nights.contains(n) {
                    out.push(format!("{ctx}: trigger night {n} not in story"));
                }
            }
            CutsceneTrigger::FlagSet(f) => ix.check(Ref::Flag(f), i18n, &ctx, &mut out),
            CutsceneTrigger::ArcStage { regular, stage } => match ix.regulars.get(regular.as_str())
            {
                Some(stages) if stages.contains(stage) => {}
                _ => out.push(format!("{ctx}: unknown arc stage {regular}/{stage}")),
            },
            CutsceneTrigger::Ending(e) => {
                if !s.endings.iter().any(|x| &x.id == e) {
                    out.push(format!("{ctx}: unknown ending `{e}`"));
                }
            }
            CutsceneTrigger::Talk { regular, night } => {
                if !ix.regulars.contains_key(regular.as_str()) {
                    out.push(format!("{ctx}: unknown regular `{regular}`"));
                }
                if let Some(n) = night
                    && !nights.contains(n)
                {
                    out.push(format!("{ctx}: talk night {n} not in story"));
                }
            }
            CutsceneTrigger::Manual => {}
        }
        if cs.steps.is_empty() {
            out.push(format!("{ctx}: no steps"));
        }
        let actor_ok = |a: &str| ix.speakers.contains(a) || ix.guests.contains(a);
        let place_ok = |p: &Place| match p {
            Place::Tile(..) => true,
            Place::Spot(s) => ix.spots.contains(s.as_str()),
        };
        walk_steps(&cs.steps, &mut |step| match step {
            CutsceneStep::Line { speaker, .. } => {
                if !ix.speakers.contains(speaker.as_str()) {
                    out.push(format!("{ctx}: unknown speaker `{speaker}`"));
                }
            }
            CutsceneStep::Choice { options } => {
                if options.len() < 2 {
                    out.push(format!("{ctx}: a choice needs at least 2 options"));
                }
                for o in options {
                    visit_effects(&o.effects, &mut |x| ix.check(x, i18n, &ctx, &mut out));
                    visit_conditions(&o.conditions, &mut |x| ix.check(x, i18n, &ctx, &mut out));
                    ix.flag(&o.set_flag, &ctx, &mut out);
                }
            }
            CutsceneStep::Camera { focus, .. } => match focus {
                Focus::Spot(sp) if !ix.spots.contains(sp.as_str()) => {
                    out.push(format!("{ctx}: unknown spot `{sp}`"))
                }
                Focus::Actor(a) if !actor_ok(a) => out.push(format!("{ctx}: unknown actor `{a}`")),
                _ => {}
            },
            CutsceneStep::ActorEnter { actor, to_tile }
            | CutsceneStep::ActorMove { actor, to_tile }
            | CutsceneStep::ActorExit { actor, to_tile } => {
                if !actor_ok(actor) {
                    out.push(format!("{ctx}: unknown actor `{actor}`"));
                }
                if !place_ok(to_tile) {
                    out.push(format!("{ctx}: unknown place {to_tile:?}"));
                }
            }
            CutsceneStep::Music { track: Some(t) } if !MUSIC_TRACKS.contains(&t.as_str()) => {
                out.push(format!("{ctx}: unknown music `{t}`"))
            }
            CutsceneStep::Sfx { id } if !SFX_IDS.contains(&id.as_str()) => {
                out.push(format!("{ctx}: unknown sfx `{id}`"))
            }
            CutsceneStep::ShowNote { note_id } => {
                ix.check(Ref::Note(note_id), i18n, &ctx, &mut out)
            }
            CutsceneStep::If { conditions, .. } => {
                visit_conditions(conditions, &mut |x| ix.check(x, i18n, &ctx, &mut out))
            }
            CutsceneStep::Call { cutscene } => {
                if cutscene == &cs.id {
                    out.push(format!("{ctx}: calls itself"));
                }
                ix.check(Ref::Cutscene(cutscene), i18n, &ctx, &mut out)
            }
            CutsceneStep::Apply { effects } => {
                visit_effects(effects, &mut |x| ix.check(x, i18n, &ctx, &mut out))
            }
            _ => {}
        });
    }

    // --- Shiftella app ------------------------------------------------------------------
    for a in &c.app.announcements {
        if !ix.speakers.contains(a.author.as_str()) {
            out.push(format!(
                "announcement `{}`: unknown author `{}`",
                a.id, a.author
            ));
        }
        check_placeholders(
            i18n,
            &format!("app.announcement.{}.body", a.id),
            &a.placeholders,
            &mut out,
        );
    }
    for item in &c.app.checklists {
        let ctx = format!("checklist `{}`", item.id);
        visit_effects(&item.done, &mut |x| ix.check(x, i18n, &ctx, &mut out));
        visit_effects(&item.missed, &mut |x| ix.check(x, i18n, &ctx, &mut out));
    }
    for b in &c.app.bot {
        let ctx = format!("bot `{}`", b.id);
        check_placeholders(
            i18n,
            &format!("app.bot.{}", b.id),
            &b.placeholders,
            &mut out,
        );
        for a in &b.actions {
            if a.id.contains("clock") {
                out.push(format!("{ctx}: clock-in/out through the bot is forbidden"));
            }
            visit_effects(&a.effects, &mut |x| ix.check(x, i18n, &ctx, &mut out));
        }
    }
    for (screen, lines) in [
        ("digest", &c.app.digest),
        ("payroll", &c.app.payroll),
        ("handover", &c.app.handover),
    ] {
        for l in lines {
            check_placeholders(
                i18n,
                &format!("app.{screen}.{}", l.id),
                &l.placeholders,
                &mut out,
            );
        }
    }

    // --- trains -------------------------------------------------------------------------
    let stations: HashSet<&str> = c.trains.stations.iter().map(|s| s.id.as_str()).collect();
    if !stations.contains(c.trains.home.as_str()) {
        out.push(format!("trains: home station `{}` unknown", c.trains.home));
    }
    for t in &c.trains.trains {
        let ctx = format!("train `{}`", t.id);
        for st in [&t.from, &t.to] {
            if !stations.contains(st.as_str()) {
                out.push(format!("{ctx}: unknown station `{st}`"));
            }
        }
        if !ix.guests.contains(t.guest.as_str()) {
            out.push(format!("{ctx}: unknown guest `{}`", t.guest));
        }
        visit_conditions(&t.announce_if, &mut |x| ix.check(x, i18n, &ctx, &mut out));
    }

    // --- difficulty & sandbox -----------------------------------------------------------
    if c.difficulties.iter().filter(|d| d.default).count() != 1 {
        out.push("difficulty: exactly one preset must be default".into());
    }
    for d in &c.difficulties {
        let ctx = format!("difficulty `{}`", d.id);
        visit_effects(&d.missed_payment_effects, &mut |x| {
            ix.check(x, i18n, &ctx, &mut out)
        });
    }
    if c.sandbox.character(&c.sandbox.default_character).is_none() {
        out.push(format!(
            "sandbox: default character `{}` unknown",
            c.sandbox.default_character
        ));
    }

    // --- hands-on -------------------------------------------------------------------------
    hands::validate(c, i18n, &ix, &mut out);
    build_v::validate(c, i18n, &ix, &mut out);

    // --- i18n -----------------------------------------------------------------------------
    for key in required_keys(c) {
        if !i18n.has_all(&key) {
            let missing: Vec<&str> = Lang::ALL
                .iter()
                .filter(|l| !i18n.has(**l, &key))
                .map(|l| l.code())
                .collect();
            out.push(format!("i18n: `{key}` missing in {}", missing.join(", ")));
        }
    }
    out.extend(i18n.check());
    out
}

fn check_placeholders(i18n: &I18n, key: &str, declared: &[String], out: &mut Vec<String>) {
    let want: BTreeSet<&str> = declared.iter().map(String::as_str).collect();
    for lang in Lang::ALL {
        if let Some(text) = i18n.get(lang, key) {
            let got = placeholders(text);
            if got != want {
                out.push(format!(
                    "i18n: `{key}` ({}) uses placeholders {got:?}, data declares {want:?}",
                    lang.code()
                ));
            }
        }
    }
}
