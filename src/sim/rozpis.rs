//! The rozpis as a light puzzle (design/handson.md §12, §13.3): staff preferences, wellbeing
//! and fairness on the draft, the published week's effects (mood, energy, lateness, sick
//! days, `WorksWith`), the pref-aware auto-fill, shared shifts, and requests that come from
//! preferences and life events.

use std::collections::BTreeMap;

use super::app::{self, week_of};
use super::rng::{Rng, hash_str};
use super::systems::*;
use super::types::*;
use super::world::*;
use crate::content::handson::{
    LifeEvent, Pref, PrefKind, PreferredShift, RequestKind, SandboxPref, Strength,
};
use crate::content::{Content, Weekday};

type Rows = BTreeMap<u32, [Shift; 7]>;

// ---------------------------------------------------------------------------------------------
// Preferences
// ---------------------------------------------------------------------------------------------

fn weekday_index(d: Weekday) -> usize {
    Weekday::ALL.iter().position(|x| *x == d).unwrap_or(0)
}

/// Generated preferences for a sandbox person (deterministic by seed and id).
fn generate(w: &World, content: &Content, s: &Staff) -> Vec<Pref> {
    let rules = &content.handson.tuning.rozpis.pref_rules;
    let mut rng = Rng::new(hash_str(&format!("prefs:{}:{}", w.seed, s.id)));
    let mut out: Vec<Pref> = Vec::new();
    let push = |sp: &SandboxPref, rng: &mut Rng, out: &mut Vec<Pref>| {
        if out.len() >= rules.max as usize {
            return;
        }
        let strength = if rng.chance(rules.need_chance) {
            Strength::Need
        } else {
            Strength::Wish
        };
        let kind = match sp {
            SandboxPref::Shift(p) => PrefKind::ShiftPref(*p),
            SandboxPref::MaxNights(n) => PrefKind::MaxNights(*n),
            SandboxPref::MinNights(n) => PrefKind::MinNights(*n),
            SandboxPref::WeekendOff => PrefKind::WeekendOff,
            SandboxPref::NoBackToBack => PrefKind::NoBackToBack,
            SandboxPref::DayOffRandom => PrefKind::DayOff(Weekday::ALL[rng.index(7)]),
            SandboxPref::WorksWithPartner => {
                let Some(p) = s
                    .romance_with
                    .and_then(|id| w.staff.iter().find(|x| x.id == id))
                else {
                    return;
                };
                PrefKind::WorksWith(p.archetype.clone())
            }
        };
        if out.iter().any(|p| p.kind.id() == kind.id()) {
            return;
        }
        out.push(Pref {
            kind,
            strength,
            quote: sp.quote_key(),
        });
    };
    for (tr, sp) in &rules.from_traits {
        if s.has_trait(tr) {
            push(sp, &mut rng, &mut out);
        }
    }
    if !rules.random.is_empty() {
        let sp = &rules.random[rng.index(rules.random.len())];
        push(sp, &mut rng, &mut out);
    }
    out
}

fn prefs_for_new(w: &World, content: &Content, s: &Staff) -> Vec<Pref> {
    if s.owner {
        return Vec::new();
    }
    if s.story_person && w.mode == Mode::Story {
        return content
            .staff_archetype(&s.archetype)
            .map(|a| a.prefs.clone())
            .unwrap_or_default();
    }
    generate(w, content, s)
}

/// Everyone (and every candidate on the hire card) has their preferences.
pub(crate) fn sync_prefs(w: &mut World, content: &Content) {
    let mut add: Vec<(u32, Vec<Pref>)> = Vec::new();
    for s in w.staff.iter().chain(w.candidates.iter().map(|c| &c.staff)) {
        if !w.sys.rozpis.prefs.contains_key(&s.id) {
            add.push((s.id, prefs_for_new(w, content, s)));
        }
    }
    for (id, p) in add {
        w.sys.rozpis.prefs.insert(id, p);
    }
}

/// A person's preferences this week (their own + life-event ones).
pub(crate) fn prefs_of(w: &World, staff: u32) -> Vec<Pref> {
    let week = week_of(w.night);
    let mut v = w.sys.rozpis.prefs.get(&staff).cloned().unwrap_or_default();
    if let Some(t) = w.sys.rozpis.temp.get(&staff) {
        v.extend(
            t.iter()
                .filter(|(_, wk)| *wk == week)
                .map(|(p, _)| p.clone()),
        );
    }
    v
}

// ---------------------------------------------------------------------------------------------
// Evaluation
// ---------------------------------------------------------------------------------------------

fn overlap(a: Shift, b: Shift) -> f32 {
    match (a.span(), b.span()) {
        (Some((a0, a1)), Some((b0, b1))) => (a1.min(b1) as f32 - a0.max(b0) as f32).max(0.0) / 60.0,
        _ => 0.0,
    }
}

fn shift_of(p: PreferredShift) -> Shift {
    match p {
        PreferredShift::Early => Shift::Early,
        PreferredShift::Late => Shift::Late,
        PreferredShift::Full => Shift::Full,
    }
}

fn partner_row<'a>(w: &World, rows: &'a Rows, arch: &str, me: u32) -> Option<&'a [Shift; 7]> {
    w.staff
        .iter()
        .find(|s| !s.owner && s.id != me && s.archetype == arch)
        .and_then(|s| rows.get(&s.id))
}

/// How one preference fares on a row.
pub(crate) fn pref_status(
    w: &World,
    rows: &Rows,
    staff: u32,
    row: &[Shift; 7],
    p: &Pref,
) -> PrefStatus {
    let on = |d: usize| row[d % 7] != Shift::Off;
    let count = (0..7).filter(|d| on(*d)).count() as u32;
    let ok = match &p.kind {
        PrefKind::DayOff(d) => !on(weekday_index(*d)),
        PrefKind::ShiftPref(s) => {
            let want = shift_of(*s);
            row.iter().all(|x| *x == Shift::Off || *x == want)
        }
        PrefKind::WeekendOff => {
            let (fri, sat) = (on(4), on(5));
            match p.strength {
                Strength::Need => !fri && !sat,
                Strength::Wish => !(fri && sat),
            }
        }
        PrefKind::NoBackToBack => !(0..7).any(|d| on(d) && on(d + 1)),
        PrefKind::MaxNights(n) => count <= *n,
        PrefKind::MinNights(n) => count >= *n,
        PrefKind::WorksWith(a) => {
            let Some(pr) = partner_row(w, rows, a, staff) else {
                return PrefStatus::Pending;
            };
            (0..7).filter(|d| overlap(row[*d], pr[*d]) >= 4.0).count() >= 2
        }
        PrefKind::AvoidWith(a) => {
            let Some(pr) = partner_row(w, rows, a, staff) else {
                return PrefStatus::Pending;
            };
            !(0..7).any(|d| overlap(row[d], pr[d]) > 0.0)
        }
    };
    if ok {
        PrefStatus::Honoured
    } else {
        PrefStatus::Violated
    }
}

fn bonus(w: &World, staff: u32) -> f32 {
    let week = week_of(w.night);
    w.sys
        .rozpis
        .bonus
        .get(&staff)
        .map(|v| v.iter().filter(|(_, wk)| *wk == week).map(|(b, _)| b).sum())
        .unwrap_or(0.0)
}

/// Wellbeing of a person on a schedule (§12.3), 0–100.
pub(crate) fn wellbeing_on(w: &World, content: &Content, rows: &Rows, staff: u32) -> f32 {
    let t = &content.handson.tuning.rozpis;
    let Some(row) = rows.get(&staff) else {
        return 100.0;
    };
    let mut wb = 100.0;
    let mut honoured = 0.0f32;
    for p in prefs_of(w, staff) {
        match (pref_status(w, rows, staff, row, &p), p.strength) {
            (PrefStatus::Violated, Strength::Need) => wb -= t.need_penalty,
            (PrefStatus::Violated, Strength::Wish) => wb -= t.wish_penalty,
            (PrefStatus::Honoured, Strength::Wish) => honoured += t.wish_bonus,
            _ => {}
        }
    }
    wb += honoured.min(t.wish_bonus_max);
    let count = row.iter().filter(|x| **x != Shift::Off).count() as u32;
    wb -= count.saturating_sub(t.nights_soft_max) as f32 * t.nights_penalty;
    wb += bonus(w, staff);
    wb.clamp(0.0, 100.0)
}

fn helpers(w: &World) -> Vec<u32> {
    w.staff.iter().filter(|s| !s.owner).map(|s| s.id).collect()
}

/// `mean(wb) − 0.5 × stdev(wb)` over the helpers (100 with nobody).
fn fairness(content: &Content, wbs: &[f32]) -> f32 {
    if wbs.is_empty() {
        return 100.0;
    }
    let n = wbs.len() as f32;
    let mean = wbs.iter().sum::<f32>() / n;
    let var = wbs.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / n;
    (mean - content.handson.tuning.rozpis.stdev_weight * var.sqrt()).clamp(0.0, 100.0)
}

/// Re-scores the draft (`RozpisScored`) and the published week.
pub(crate) fn score(w: &mut World, content: &Content) {
    sync_prefs(w, content);
    let ids = helpers(w);
    let draft = effective(w, content, &w.app.draft);
    let published = effective(w, content, &w.app.published);
    let mut wb = BTreeMap::new();
    let mut pwb = BTreeMap::new();
    for id in &ids {
        wb.insert(*id, wellbeing_on(w, content, &draft, *id));
        pwb.insert(*id, wellbeing_on(w, content, &published, *id));
    }
    let vals: Vec<f32> = wb.values().copied().collect();
    let f = fairness(content, &vals);
    let changed = w.sys.rozpis.wellbeing != wb || (w.sys.rozpis.fairness - f).abs() > 0.01;
    w.sys.rozpis.wellbeing = wb.clone();
    w.sys.rozpis.fairness = f;
    w.sys.rozpis.published_wb = pwb;
    if changed {
        w.emit(SimEvent::RozpisScored {
            wellbeing: wb.into_iter().collect(),
            fairness: f,
        });
    }
}

// ---------------------------------------------------------------------------------------------
// Coverage & auto-fill
// ---------------------------------------------------------------------------------------------

fn per_person(n: usize) -> usize {
    match n {
        0 => 0,
        1 => 7,
        2 | 3 => 5,
        _ => 4,
    }
}

/// Helpers `weekday_demand` suggests per weekday.
pub(crate) fn suggested(w: &World, content: &Content) -> [u32; 7] {
    let n = helpers(w).len();
    let demand = app::weekday_demand(w, content);
    let total_d: f32 = demand.iter().sum::<f32>().max(0.01);
    let total = (n * per_person(n)) as f32;
    let mut out = [0u32; 7];
    for d in 0..7 {
        out[d] = if n == 0 {
            0
        } else {
            ((total * demand[d] / total_d).round() as u32).clamp(1, n as u32)
        };
    }
    out
}

/// The coverage footer of the draft.
pub(crate) fn coverage(w: &World, content: &Content) -> [CoverageNight; 7] {
    let sug = suggested(w, content);
    let demand = app::weekday_demand(w, content);
    let ids = helpers(w);
    std::array::from_fn(|d| {
        let on = ids
            .iter()
            .filter(|id| w.app.draft.get(id).is_some_and(|r| r[d] != Shift::Off))
            .count() as u32;
        let cups = if demand[d] < 0.9 {
            1
        } else if demand[d] < 1.15 {
            2
        } else {
            3
        };
        CoverageNight {
            weekday: d as u8,
            suggested: sug[d],
            on,
            cups,
        }
    })
}

/// How far a row is from keeping each preference (the search needs a slope: a broken Need
/// is one step, but "two of three back-to-back pairs fixed" is progress).
fn soft_cost(w: &World, rows: &Rows, id: u32) -> f32 {
    let Some(row) = rows.get(&id) else {
        return 0.0;
    };
    let on = |d: usize| row[d % 7] != Shift::Off;
    let count = (0..7).filter(|d| on(*d)).count() as i32;
    let mut c = 0.0;
    for p in prefs_of(w, id) {
        // a Need weighs like most of its wellbeing penalty, so fixing it step by step pays
        let k = if p.strength == Strength::Need {
            20.0
        } else {
            1.0
        };
        c += k * match &p.kind {
            PrefKind::NoBackToBack => (0..7).filter(|d| on(*d) && on(d + 1)).count() as f32,
            PrefKind::MaxNights(n) => (count - *n as i32).max(0) as f32,
            PrefKind::MinNights(n) => (*n as i32 - count).max(0) as f32,
            PrefKind::WorksWith(a) => match partner_row(w, rows, a, id) {
                Some(pr) => {
                    let shared = (0..7).filter(|d| overlap(row[*d], pr[*d]) >= 4.0).count();
                    2usize.saturating_sub(shared) as f32
                }
                None => 0.0,
            },
            _ => 0.0,
        };
    }
    c
}

/// The search cost of day placements: shift kinds applied as the auto-fill would.
fn total_cost(w: &World, content: &Content, rows: &Rows, ids: &[u32]) -> f32 {
    let rows = shift_pass(w, ids, rows.clone());
    let people: f32 = ids
        .iter()
        .map(|id| 100.0 - wellbeing_on(w, content, &rows, *id) + soft_cost(w, &rows, *id))
        .sum();
    // a night below the suggested coverage is busier for Karolina (fine, but not free)
    let sug = suggested(w, content);
    let short: u32 = (0..7)
        .map(|d| {
            let on = ids
                .iter()
                .filter(|id| rows.get(id).is_some_and(|r| r[d] != Shift::Off))
                .count() as u32;
            sug[d].saturating_sub(on)
        })
        .sum();
    people + short as f32 * COVERAGE_COST
}

/// Search cost of one helper-night below the suggested coverage.
const COVERAGE_COST: f32 = 8.0;

/// The schedule as it will be worked: approved leave in the coming week counts as Off.
pub(crate) fn effective(w: &World, content: &Content, rows: &Rows) -> Rows {
    let mut out = rows.clone();
    for (id, row) in out.iter_mut() {
        let away = away_days(w, content, *id);
        for d in 0..7 {
            if away[d] {
                row[d] = Shift::Off;
            }
        }
    }
    out
}

/// Days each helper can't work in the coming week (approved leave).
fn away_days(w: &World, content: &Content, id: u32) -> [bool; 7] {
    let mut a = [false; 7];
    if let Some(s) = w.staff_by_id(id) {
        for (night, _) in &s.leave_nights {
            if *night >= w.night && *night < w.night + 7 {
                a[app::weekday_of_night(w, content, *night)] = true;
            }
        }
    }
    a
}

/// Shift kinds by `ShiftPref` (a Need always; a Wish only while the other half of the night
/// stays covered by a colleague).
fn shift_pass(w: &World, ids: &[u32], mut rows: Rows) -> Rows {
    let full_on = |rows: &Rows, d: usize| {
        ids.iter()
            .filter(|id| rows.get(id).is_some_and(|r| r[d] == Shift::Full))
            .count()
    };
    for &id in ids {
        for p in prefs_of(w, id) {
            let PrefKind::ShiftPref(sp) = p.kind else {
                continue;
            };
            let want = shift_of(sp);
            for d in 0..7 {
                let Some(cur) = rows.get(&id).map(|r| r[d]) else {
                    continue;
                };
                if cur == Shift::Off || cur == want {
                    continue;
                }
                if want == Shift::Full && full_on(&rows, d) >= app::ROZPIS_MAX_FULL_HIRES {
                    continue;
                }
                let covered = |half: Shift| {
                    ids.iter().any(|o| {
                        *o != id && rows.get(o).is_some_and(|r| overlap(r[d], half) >= 4.0)
                    })
                };
                let ok = match want {
                    Shift::Early => covered(Shift::Late),
                    Shift::Late => covered(Shift::Early),
                    _ => true,
                } || p.strength == Strength::Need;
                if ok {
                    rows.get_mut(&id).unwrap()[d] = want;
                }
            }
        }
    }
    rows
}

/// §12.5: the pref-aware auto-fill on top of the base rozpis (coverage, counts and split
/// shifts stay as the base plans them): days off are moved to where they hurt least, then
/// shift kinds follow `ShiftPref` where the other half of the night stays covered.
pub(crate) fn autofill(w: &World, content: &Content, base: Rows) -> (Rows, AutofillReport) {
    let ids = helpers(w);
    let base_count: BTreeMap<u32, usize> = base
        .iter()
        .map(|(id, r)| (*id, r.iter().filter(|x| **x != Shift::Off).count()))
        .collect();
    let mut rows = base;
    let away: BTreeMap<u32, [bool; 7]> = ids
        .iter()
        .map(|id| (*id, away_days(w, content, *id)))
        .collect();
    let full_on = |rows: &Rows, d: usize| {
        ids.iter()
            .filter(|id| rows.get(id).is_some_and(|r| r[d] == Shift::Full))
            .count()
    };
    let on_count = |rows: &Rows, d: usize| {
        ids.iter()
            .filter(|id| rows.get(id).is_some_and(|r| r[d] != Shift::Off))
            .count()
    };
    // 1. move days off, 1b. a night less where a Need can't be kept otherwise; repeat while
    // either helps
    let mut cost = total_cost(w, content, &rows, &ids);
    for _round in 0..4 {
        let before = cost;
        for _ in 0..64 {
            let mut best: Option<(Rows, f32)> = None;
            'search: for &id in &ids {
                let Some(row) = rows.get(&id).copied() else {
                    continue;
                };
                for a in 0..7 {
                    if row[a] == Shift::Off || on_count(&rows, a) <= 1 {
                        continue;
                    }
                    for b in 0..7 {
                        if row[b] != Shift::Off || away[&id][b] {
                            continue;
                        }
                        if row[a] == Shift::Full && full_on(&rows, b) >= app::ROZPIS_MAX_FULL_HIRES
                        {
                            continue;
                        }
                        let mut next = rows.clone();
                        let r = next.get_mut(&id).unwrap();
                        r[b] = r[a];
                        r[a] = Shift::Off;
                        let c = total_cost(w, content, &next, &ids);
                        if c + 0.01 < cost && best.as_ref().is_none_or(|(_, bc)| c < *bc) {
                            best = Some((next, c));
                            if c <= 0.01 {
                                break 'search;
                            }
                        }
                    }
                }
            }
            match best {
                Some((r, c)) => {
                    rows = r;
                    cost = c;
                }
                None => break,
            }
        }
        // 1b. a Need that no move can keep ("not two nights in a row", "three nights a week"):
        // a night less for that person, while someone else still covers it
        for _ in 0..16 {
            let mut best: Option<(Rows, f32)> = None;
            for &id in &ids {
                let Some(row) = rows.get(&id).copied() else {
                    continue;
                };
                let need_broken = prefs_of(w, id).iter().any(|p| {
                    p.strength == Strength::Need
                        && pref_status(w, &rows, id, &row, p) == PrefStatus::Violated
                });
                if !need_broken {
                    continue;
                }
                for (a, sh) in row.iter().enumerate() {
                    if *sh == Shift::Off || on_count(&rows, a) <= 1 {
                        continue;
                    }
                    let mut next = rows.clone();
                    next.get_mut(&id).unwrap()[a] = Shift::Off;
                    let c = total_cost(w, content, &next, &ids);
                    if c + 0.01 < cost && best.as_ref().is_none_or(|(_, bc)| c < *bc) {
                        best = Some((next, c));
                    }
                }
            }
            match best {
                Some((r, c)) => {
                    rows = r;
                    cost = c;
                }
                None => break,
            }
        }
        // 1c. a night more for someone below their planned count where it helps coverage
        for _ in 0..16 {
            let mut best: Option<(Rows, f32)> = None;
            for &id in &ids {
                let (Some(row), Some(planned)) = (rows.get(&id).copied(), base_count.get(&id))
                else {
                    continue;
                };
                if row.iter().filter(|x| **x != Shift::Off).count() >= *planned {
                    continue;
                }
                for b in 0..7 {
                    if row[b] != Shift::Off || away[&id][b] {
                        continue;
                    }
                    let mut next = rows.clone();
                    let shift = if full_on(&rows, b) >= app::ROZPIS_MAX_FULL_HIRES {
                        Shift::Late
                    } else {
                        Shift::Full
                    };
                    next.get_mut(&id).unwrap()[b] = shift;
                    let c = total_cost(w, content, &next, &ids);
                    if c + 0.01 < cost && best.as_ref().is_none_or(|(_, bc)| c < *bc) {
                        best = Some((next, c));
                    }
                }
            }
            match best {
                Some((r, c)) => {
                    rows = r;
                    cost = c;
                }
                None => break,
            }
        }
        if cost + 0.01 >= before {
            break;
        }
    }
    // 2. shift kinds by ShiftPref
    let rows = shift_pass(w, &ids, rows);
    // report
    let mut report = AutofillReport::default();
    for &id in &ids {
        let Some(row) = rows.get(&id) else { continue };
        for p in prefs_of(w, id) {
            match pref_status(w, &rows, id, row, &p) {
                PrefStatus::Honoured => {
                    report.kept += 1;
                    report.total += 1;
                }
                PrefStatus::Violated => {
                    report.total += 1;
                    report.broken.push((id, p.kind.clone()));
                }
                PrefStatus::Pending => {}
            }
        }
    }
    (rows, report)
}

/// The week is happy: every pref honoured, coverage ≥ suggested − 1 every night.
pub(crate) fn happy(w: &World, content: &Content, rows: &Rows) -> bool {
    let ids = helpers(w);
    if ids.is_empty() {
        return false;
    }
    let all_prefs = ids.iter().all(|id| {
        let Some(row) = rows.get(id) else {
            return false;
        };
        prefs_of(w, *id)
            .iter()
            .all(|p| pref_status(w, rows, *id, row, p) != PrefStatus::Violated)
    });
    let sug = suggested(w, content);
    let covered = (0..7).all(|d| {
        let on = ids
            .iter()
            .filter(|id| rows.get(id).is_some_and(|r| r[d] != Shift::Off))
            .count() as u32;
        on + 1 >= sug[d]
    });
    all_prefs && covered
}

/// After a publish (or the app planning the week): a happy week → `RozpisHappy`.
pub(crate) fn on_published(w: &mut World, content: &Content) {
    score(w, content);
    let week = week_of(w.night);
    if w.sys.rozpis.happy_week == Some(week) {
        return;
    }
    let rows = effective(w, content, &w.app.published);
    if happy(w, content, &rows) {
        w.sys.rozpis.happy_week = Some(week);
        w.emit(SimEvent::RozpisHappy);
        let ctx = EffectCtx::default();
        app::bot(w, content, "rozpis_happy", &ctx);
        super::journal::capture(
            w,
            content,
            crate::content::handson::HighlightKind::RozpisHappy,
            None,
            Vec::new(),
            None,
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Tonight's effects
// ---------------------------------------------------------------------------------------------

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

fn published_wb(w: &World, staff: u32) -> f32 {
    w.sys
        .rozpis
        .published_wb
        .get(&staff)
        .copied()
        .unwrap_or(70.0)
}

/// (late chance ×, sick chance ×) from the published week's wellbeing.
pub(crate) fn late_sick_mul(w: &World, content: &Content, staff: u32) -> (f32, f32) {
    let t = &content.handson.tuning.rozpis;
    let wb = published_wb(w, staff) / 100.0;
    (
        lerp(t.late_mul.0, t.late_mul.1, wb),
        lerp(t.sick_mul.0, t.sick_mul.1, wb),
    )
}

/// Night start (after the crew's statuses are set): mood baseline, shift-start energy,
/// broken Needs, unanswered requests, a happy week.
pub(crate) fn on_night_start(w: &mut World, content: &Content) {
    score(w, content);
    let t = content.handson.tuning.rozpis.clone();
    let wd = w.weekday as usize;
    let week = week_of(w.night);
    let happy = w.sys.rozpis.happy_week == Some(week);
    let published = effective(w, content, &w.app.published);
    for i in 0..w.staff.len() {
        if w.staff[i].owner {
            continue;
        }
        let id = w.staff[i].id;
        let wb = published_wb(w, id);
        let working = !matches!(w.staff[i].status, StaffStatus::OffDuty | StaffStatus::Sick);
        let s = &mut w.staff[i];
        s.mood = (s.mood + (wb - t.mood_baseline_center) * t.mood_baseline_per).clamp(0.0, 100.0);
        if happy {
            s.mood = (s.mood + t.happy_week_mood).min(100.0);
        }
        if working {
            let e = (100.0 - (100.0 - wb) * t.energy_loss_per).max(t.energy_min);
            s.energy = s.energy.min(e);
        }
        if !working {
            continue;
        }
        // a broken Need tonight (no request was made for it): they come in a bit low
        if let Some(row) = published.get(&id) {
            for p in prefs_of(w, id) {
                let breaks_tonight = match &p.kind {
                    PrefKind::DayOff(d) => weekday_index(*d) == wd,
                    PrefKind::WeekendOff => wd == 4 || wd == 5,
                    PrefKind::NoBackToBack => row[(wd + 6) % 7] != Shift::Off,
                    _ => false,
                };
                if breaks_tonight && pref_status(w, &published, id, row, &p) == PrefStatus::Violated
                {
                    w.emit(SimEvent::PrefViolated {
                        staff: id,
                        pref: p.kind.clone(),
                    });
                    if p.strength == Strength::Need {
                        super::logic::change_mood(w, content, i, t.need_violated_mood, false);
                    }
                }
            }
        }
    }
    // a declined Need: 25 % they come in late
    let late: Vec<u32> = std::mem::take(&mut w.sys.rozpis.late_tonight)
        .into_iter()
        .collect();
    for id in late {
        if let Some(s) = w.staff.iter_mut().find(|s| s.id == id)
            && let StaffStatus::Expected { minute } = s.status
        {
            s.status = StaffStatus::Expected {
                minute: minute + 15,
            };
            s.late_minutes += 15;
            w.emit(SimEvent::StaffLate {
                staff: id,
                minutes: 15,
            });
        } else if let Some(s) = w.staff.iter_mut().find(|s| s.id == id)
            && s.on_duty()
        {
            // already inside at opening: they slip out and back in
            s.status = StaffStatus::Expected { minute: 15 };
            s.late_minutes = 15;
            w.emit(SimEvent::StaffLate {
                staff: id,
                minutes: 15,
            });
        }
    }
    expire_requests(w, content);
    early_leave_tonight(w);
}

/// No answer by night start: "{name}: I'll manage." (mood −3), the request lapses.
fn expire_requests(w: &mut World, content: &Content) {
    let night = w.night;
    let mut lapsed: Vec<(u32, u32)> = Vec::new();
    for (rid, meta) in &w.sys.rozpis.requests {
        if meta.night > night {
            continue;
        }
        let pending = w
            .app
            .swaps
            .iter()
            .any(|s| s.id == *rid && s.status == RequestStatus::Pending)
            || w.app
                .time_off
                .iter()
                .any(|s| s.id == *rid && s.status == RequestStatus::Pending)
            || w.sys
                .rozpis
                .early_leave
                .iter()
                .any(|s| s.id == *rid && s.status == RequestStatus::Pending);
        if pending {
            lapsed.push((*rid, meta.staff));
        }
    }
    let no_answer = content.handson.tuning.rozpis.requests.no_answer_mood;
    for (rid, staff) in lapsed {
        // the swap board: a colleague picks it up
        if w.sys.fx.swap_board && w.app.swaps.iter().any(|s| s.id == rid) && w.rng.chance(0.6) {
            let _ = app::answer_swap(w, rid, true);
            w.emit(SimEvent::RequestAnswered {
                id: rid,
                approved: true,
            });
            continue;
        }
        for s in w.app.swaps.iter_mut().filter(|s| s.id == rid) {
            s.status = RequestStatus::Declined;
        }
        for s in w.app.time_off.iter_mut().filter(|s| s.id == rid) {
            s.status = RequestStatus::Declined;
        }
        for s in w.sys.rozpis.early_leave.iter_mut().filter(|s| s.id == rid) {
            s.status = RequestStatus::Declined;
        }
        if let Some(i) = w.staff.iter().position(|s| s.id == staff) {
            super::logic::change_mood(w, content, i, no_answer, false);
        }
        let ctx = EffectCtx {
            staff: Some(staff),
            ..Default::default()
        };
        app::bot(w, content, "manage_alone", &ctx);
    }
}

/// Approved early leave: they go home at the agreed minute tonight.
fn early_leave_tonight(w: &mut World) {
    let night = w.night;
    let leaves: Vec<(u32, u32)> = w
        .sys
        .rozpis
        .early_leave
        .iter()
        .filter(|r| r.night == night && r.status == RequestStatus::Approved)
        .map(|r| (r.staff, r.minute))
        .collect();
    for (id, minute) in leaves {
        if let Some(s) = w.staff.iter_mut().find(|s| s.id == id) {
            s.leave_at = s.leave_at.min(minute);
        }
    }
}

/// Each minute: an honoured `WorksWith` grows the friendship faster while together.
pub(crate) fn on_minute(w: &mut World, content: &Content) {
    let per_min = content
        .handson
        .tuning
        .rozpis
        .works_with_relationship_per_hour
        / 60.0;
    let mut pairs = Vec::new();
    for s in w.staff.iter().filter(|s| !s.owner && s.on_duty()) {
        for p in prefs_of(w, s.id) {
            if let PrefKind::WorksWith(a) = &p.kind
                && let Some(o) = w
                    .staff
                    .iter()
                    .find(|o| o.id != s.id && !o.owner && o.on_duty() && &o.archetype == a)
            {
                pairs.push((s.id, o.id));
            }
        }
    }
    for (a, b) in pairs {
        super::staff::add_relationship(w, a, b, per_min);
    }
}

/// Helper speed × for an honoured `WorksWith` pair on the floor together.
pub(crate) fn pair_speed(w: &World, content: &Content, staff: u32) -> f32 {
    let has = prefs_of(w, staff).iter().any(|p| match &p.kind {
        PrefKind::WorksWith(a) => w
            .staff
            .iter()
            .any(|o| o.id != staff && !o.owner && o.on_duty() && &o.archetype == a),
        _ => false,
    });
    if has {
        content.handson.tuning.rozpis.works_with_speed
    } else {
        1.0
    }
}

/// 06:00: shared shifts (≥ 4 h together) between archetypes.
pub(crate) fn on_night_end(w: &mut World) {
    let worked: Vec<(String, Shift)> = w
        .staff
        .iter()
        .filter(|s| !s.owner && s.minutes_worked >= 120)
        .map(|s| (s.archetype.clone(), s.shift))
        .collect();
    for i in 0..worked.len() {
        for j in i + 1..worked.len() {
            if overlap(worked[i].1, worked[j].1) >= 4.0 {
                let mut k = [worked[i].0.clone(), worked[j].0.clone()];
                k.sort();
                let [a, b] = k;
                *w.sys.rozpis.shared.entry((a, b)).or_insert(0) += 1;
            }
        }
    }
}

pub(crate) fn shared_shifts(w: &World, a: &str, b: &str) -> u32 {
    let mut k = [a.to_string(), b.to_string()];
    k.sort();
    let [a, b] = k;
    w.sys.rozpis.shared.get(&(a, b)).copied().unwrap_or(0)
}

// ---------------------------------------------------------------------------------------------
// Requests from preferences and life events (§13.3)
// ---------------------------------------------------------------------------------------------

fn note_request(
    w: &mut World,
    id: u32,
    kind: RequestKind,
    staff: u32,
    reason: Option<String>,
    need: bool,
    night: u32,
) {
    w.sys.rozpis.requests.insert(
        id,
        RequestMeta {
            kind,
            staff,
            reason_key: reason,
            need,
            night,
        },
    );
    w.emit(SimEvent::RequestCreated { id, kind, staff });
}

/// The next night (from tonight) on a weekday.
fn night_on(w: &World, content: &Content, weekday: usize) -> u32 {
    (w.night..w.night + 7)
        .find(|n| app::weekday_of_night(w, content, *n) == weekday)
        .unwrap_or(w.night)
}

/// Day start: life events, a swap request for a Need broken tonight, owed toil.
pub(crate) fn on_day_start(w: &mut World, content: &Content) {
    sync_prefs(w, content);
    let t = content.handson.tuning.rozpis.clone();
    if w.mode == Mode::Story {
        for ev in &t.life_events {
            life_event(w, content, ev);
        }
    }
    // a Need tonight's rozpis breaks: 50 % they ask for a swap
    let wd = w.weekday as usize;
    let published = w.app.published.clone();
    let ids = helpers(w);
    for &id in &ids {
        let Some(row) = published.get(&id) else {
            continue;
        };
        if row[wd] == Shift::Off {
            continue;
        }
        let need_broken = prefs_of(w, id).iter().any(|p| {
            p.strength == Strength::Need
                && pref_status(w, &published, id, row, p) == PrefStatus::Violated
                && match &p.kind {
                    PrefKind::DayOff(d) => weekday_index(*d) == wd,
                    PrefKind::WeekendOff => wd == 4 || wd == 5,
                    PrefKind::NoBackToBack => {
                        row[(wd + 6) % 7] != Shift::Off || row[(wd + 1) % 7] != Shift::Off
                    }
                    PrefKind::MaxNights(_) => true,
                    _ => false,
                }
        });
        let already = w
            .sys
            .rozpis
            .requests
            .values()
            .any(|m| m.staff == id && m.night == w.night);
        if !need_broken || already || !w.rng.chance(t.need_violated_swap_chance) {
            continue;
        }
        // a colleague who is off tonight and whose own Needs survive the swap
        let partner = ids.iter().copied().find(|o| {
            *o != id
                && published.get(o).is_some_and(|r| r[wd] == Shift::Off)
                && !away_days(w, content, *o)[wd]
                && {
                    let mut next = published.clone();
                    let (mine, theirs) = (row[wd], Shift::Off);
                    next.get_mut(o).unwrap()[wd] = mine;
                    next.get_mut(&id).unwrap()[wd] = theirs;
                    let r = next[o];
                    prefs_of(w, *o).iter().all(|p| {
                        p.strength != Strength::Need
                            || pref_status(w, &next, *o, &r, p) != PrefStatus::Violated
                    })
                }
        });
        let Some(to) = partner else { continue };
        let rid = w.alloc_id();
        w.app.swaps.push(SwapRequest {
            id: rid,
            from: id,
            to,
            weekday: wd as u8,
            status: RequestStatus::Pending,
        });
        let night = w.night;
        note_request(w, rid, RequestKind::Swap, id, None, true, night);
        let other = w
            .staff_by_id(to)
            .map(|s| s.name.clone())
            .unwrap_or_default();
        let ctx = EffectCtx {
            staff: Some(id),
            ..Default::default()
        };
        app::bot_request(
            w,
            content,
            "swap_request",
            &ctx,
            rid,
            vec![
                ("other".into(), other),
                ("shift".into(), Weekday::ALL[wd].id().to_string()),
            ],
        );
    }
    // owed náhradní volno: a request
    for &id in &ids {
        let owed = w.staff_by_id(id).map(|s| s.toil_hours).unwrap_or(0.0);
        let pending = w.app.time_off.iter().any(|r| {
            r.staff == id && r.kind == LeaveKind::Toil && r.status == RequestStatus::Pending
        });
        if owed >= t.requests.toil_hours && !pending {
            let rid = w.alloc_id();
            let night = w.night + 2;
            w.app.time_off.push(TimeOffRequest {
                id: rid,
                staff: id,
                kind: LeaveKind::Toil,
                night,
                hours: owed.min(8.0),
                status: RequestStatus::Pending,
            });
            note_request(w, rid, RequestKind::Toil, id, None, false, night);
            let ctx = EffectCtx {
                staff: Some(id),
                ..Default::default()
            };
            app::bot_request(
                w,
                content,
                "time_off_toil",
                &ctx,
                rid,
                vec![("hours".into(), format!("{:.0}", owed.min(8.0)))],
            );
        }
    }
}

fn life_event(w: &mut World, content: &Content, ev: &LifeEvent) {
    if w.sys.rozpis.life_events.contains(&ev.id) || w.night < ev.night || w.night > ev.night + 6 {
        return;
    }
    let Some(staff) = w
        .staff
        .iter()
        .find(|s| !s.owner && s.archetype == ev.staff)
        .map(|s| s.id)
    else {
        return;
    };
    w.sys.rozpis.life_events.insert(ev.id.clone());
    let week = week_of(w.night);
    if let Some(p) = &ev.pref {
        w.sys
            .rozpis
            .temp
            .entry(staff)
            .or_default()
            .push((p.clone(), week));
    }
    let reason = Some(format!("app.request.{}", ev.id));
    let name = w
        .staff_by_id(staff)
        .map(|s| s.name.clone())
        .unwrap_or_default();
    let ctx = EffectCtx {
        staff: Some(staff),
        ..Default::default()
    };
    match ev.request {
        Some(RequestKind::TimeOff) => {
            let wd = match ev.pref.as_ref().map(|p| &p.kind) {
                Some(PrefKind::DayOff(d)) => weekday_index(*d),
                _ => w.weekday as usize,
            };
            let night = night_on(w, content, wd);
            let rid = w.alloc_id();
            w.app.time_off.push(TimeOffRequest {
                id: rid,
                staff,
                kind: LeaveKind::Vacation,
                night,
                hours: 8.0,
                status: RequestStatus::Pending,
            });
            note_request(w, rid, RequestKind::TimeOff, staff, reason, true, night);
            app::bot_request(
                w,
                content,
                "time_off_vacation",
                &ctx,
                rid,
                vec![
                    ("dates".into(), night.to_string()),
                    ("day".into(), Weekday::ALL[wd].id().to_string()),
                    ("name".into(), name),
                ],
            );
        }
        Some(RequestKind::EarlyLeave) => {
            let rid = w.alloc_id();
            let night = w.night;
            w.sys.rozpis.early_leave.push(EarlyLeaveRequest {
                id: rid,
                staff,
                night,
                minute: 240,
                status: RequestStatus::Pending,
            });
            note_request(w, rid, RequestKind::EarlyLeave, staff, reason, false, night);
            app::bot_request(
                w,
                content,
                "early_leave",
                &ctx,
                rid,
                vec![("time".into(), "02:00".into()), ("name".into(), name)],
            );
        }
        _ => {}
    }
    score(w, content);
}

/// `Command::AnswerEarlyLeave`.
pub(crate) fn answer_early_leave(
    w: &mut World,
    content: &Content,
    id: u32,
    approve: bool,
) -> Result<(), CommandError> {
    let Some(r) = w.sys.rozpis.early_leave.iter_mut().find(|r| r.id == id) else {
        return Err(CommandError::UnknownId(id.to_string()));
    };
    if r.status != RequestStatus::Pending {
        return Err(CommandError::Invalid("already answered".into()));
    }
    r.status = if approve {
        RequestStatus::Approved
    } else {
        RequestStatus::Declined
    };
    let (staff, night, minute) = (r.staff, r.night, r.minute);
    if approve
        && night == w.night
        && w.phase == Phase::Night
        && let Some(s) = w.staff.iter_mut().find(|s| s.id == staff)
    {
        s.leave_at = s.leave_at.min(minute.max(w.tick / TICKS_PER_MINUTE));
    }
    on_answer(w, content, id, approve, None);
    Ok(())
}

/// Consequences of an answer (§13.3) for any rozpis request (swap, time off, early leave).
pub(crate) fn on_answer(
    w: &mut World,
    content: &Content,
    id: u32,
    approve: bool,
    partner: Option<u32>,
) {
    let rules = content.handson.tuning.rozpis.requests.clone();
    let staff = w
        .sys
        .rozpis
        .requests
        .get(&id)
        .map(|m| m.staff)
        .or_else(|| w.app.swaps.iter().find(|s| s.id == id).map(|s| s.from))
        .or_else(|| w.app.time_off.iter().find(|s| s.id == id).map(|s| s.staff));
    let need = w.sys.rozpis.requests.get(&id).is_some_and(|m| m.need);
    w.emit(SimEvent::RequestAnswered {
        id,
        approved: approve,
    });
    let Some(staff) = staff else { return };
    let week = week_of(w.night);
    if approve {
        w.sys
            .rozpis
            .bonus
            .entry(staff)
            .or_default()
            .push((rules.approve_wellbeing, week));
        if let Some(me) = super::pacing::owner_id(w) {
            super::staff::add_relationship(w, staff, me, rules.approve_relationship);
        }
        if let Some(p) = partner {
            let suits = prefs_of(w, p)
                .iter()
                .any(|x| matches!(x.kind, PrefKind::MinNights(_) | PrefKind::WorksWith(_)));
            if !suits {
                w.sys
                    .rozpis
                    .bonus
                    .entry(p)
                    .or_default()
                    .push((rules.partner_wellbeing, week));
            }
        }
    } else {
        if let Some(i) = w.staff.iter().position(|s| s.id == staff) {
            super::logic::change_mood(w, content, i, rules.decline_mood, false);
        }
        if need && w.rng.chance(rules.decline_need_late) {
            w.sys.rozpis.late_tonight.insert(staff);
        }
        let night = w.night;
        let list = w.sys.rozpis.declines.entry(staff).or_default();
        list.push(night);
        list.retain(|n| night.saturating_sub(*n) < rules.tired_window_nights);
        if list.len() as u32 >= rules.tired_after {
            list.clear();
            let ctx = EffectCtx {
                staff: Some(staff),
                ..Default::default()
            };
            app::bot(w, content, "looks_tired", &ctx);
        }
    }
    score(w, content);
}
