//! The sandbox «Challenge» mode (owner, 2026-11-27): a sandbox that gets harder every season
//! so a café can't run for ever. Chosen in the sandbox setup (Endless — the default, calm as
//! before — or Challenge) and kept in the town options (`town.options["challenge"]`), so it
//! survives saves. Tuning: `sandbox.ron` → `challenge` ([`ChallengeDef`]).
//!
//! Nothing new to learn — the existing systems turn harder:
//! * **Money.** Every season after the first: rent, supply prices and wages rise
//!   (`*_inflation`, compounding; no `rent_growth_max` cap — the difficulty's 10-night rent
//!   growth is replaced by this). A **season goal** (revenue + tips over the season's 30
//!   nights, scaled by the town size and the season) shows on the planning card, the morning
//!   summary and in a bot message; a missed goal is a strike, `strikes_to_close` in a row
//!   close the café (as does `debt_nights` nights below −`debt_limit_rents` × the rent).
//! * **Rivals.** From `competitor_from_season` a rival café opens nearby each season (a name
//!   from the town's list, text only), each taking `competitor_share` of the walk-ins and
//!   wearing a little reputation off; the café's strengths (coziness level, four stars, its
//!   own drink, regulars) each win `counter_cut` of that back. One morning line a night.
//! * **Nights.** The storyteller rolls more often, Bad events weigh more, the good share,
//!   breathing room and tension budget loosen ([`storyteller`]); appliances wear faster.
//!
//! The closing goes through the sandbox's `CafeClosed` (story::on_night_end); the final page
//! reads [`World::challenge_report`]. Endless never touches any of it: every hook here is the
//! identity while `ChallengeState::season` is 0, nothing draws from the world RNG (rival names
//! use a derived RNG), and the state is `#[serde(default)]`.

use serde::{Deserialize, Serialize};

use super::purchases::PayoffLine;
use super::rng::{Rng, hash_str};
use super::types::*;
use super::world::*;
use crate::content::{ChallengeDef, Content};

/// The town option that turns the mode on (`SandboxSetup::options`).
pub const OPTION: &str = "challenge";

/// Why a Challenge run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChallengeEnd {
    /// `strikes_to_close` season goals missed in a row.
    Strikes,
    /// Too deep in the red for `debt_nights` nights.
    Debt,
}

impl ChallengeEnd {
    pub fn id(self) -> &'static str {
        match self {
            ChallengeEnd::Strikes => "strikes",
            ChallengeEnd::Debt => "debt",
        }
    }
}

/// A strength that wins back walk-ins from the rivals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Strength {
    /// Coziness at `counter_coziness_level` or more.
    Coziness,
    /// Reputation ≥ `counter_reputation` (four stars).
    Reputation,
    /// The café's own drink is on the menu.
    Signature,
    /// `counter_regulars` regulars who trust the café.
    Regulars,
}

/// One finished season.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeasonResult {
    pub season: u32,
    pub goal: i64,
    pub earned: i64,
    pub met: bool,
    pub money_end: i64,
    pub reputation_end: f32,
    pub rent: i64,
    pub rivals: u32,
}

/// Challenge progress (`World::challenge`). `season == 0`: Endless (or not started).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ChallengeState {
    /// The running season, 1-based.
    pub season: u32,
    /// This season's goal and what the café has earned towards it (revenue + tips).
    pub goal: i64,
    pub earned: i64,
    /// Missed goals in a row.
    pub strikes: u32,
    /// Supply prices and wages × this (1.0 in season 1).
    pub supply_index: f32,
    pub wage_index: f32,
    /// Rival cafés open nearby (names, oldest first).
    pub rivals: Vec<String>,
    /// Tonight's share of the walk-ins the rivals take (after the strengths).
    pub loss: f32,
    /// Tonight's strengths against the rivals.
    pub strengths: Vec<Strength>,
    /// Nights in a row below the debt limit.
    pub debt_nights: u32,
    pub history: Vec<SeasonResult>,
    /// The best night's revenue + tips, all guests served, all revenue + tips.
    pub best_night: i64,
    pub guests: u64,
    pub earned_total: i64,
    pub ended: Option<ChallengeEnd>,
}

/// What the final page shows.
#[derive(Debug, Clone, PartialEq)]
pub struct ChallengeReport {
    pub seed: u64,
    /// Seasons the café finished and stayed open after.
    pub seasons_survived: u32,
    pub nights: u32,
    pub best_night: i64,
    pub guests: u64,
    pub earned: i64,
    pub reason: Option<ChallengeEnd>,
}

/// The planning card's / HUD's view of the running season.
#[derive(Debug, Clone, PartialEq)]
pub struct ChallengeView {
    pub season: u32,
    pub goal: i64,
    pub earned: i64,
    pub nights_left: u32,
    pub strikes: u32,
    pub strikes_max: u32,
    pub rivals: Vec<String>,
    pub loss: f32,
    pub strengths: Vec<Strength>,
    pub debt_nights: u32,
    pub debt_limit: i64,
}

/// The setup chose Challenge (sandbox only).
pub(crate) fn enabled(w: &World) -> bool {
    w.mode == Mode::Sandbox && w.town.options.get(OPTION).copied().unwrap_or(false)
}

/// A Challenge season is running.
pub(crate) fn active(w: &World) -> bool {
    w.challenge.season > 0
}

fn def(content: &Content) -> &ChallengeDef {
    &content.sandbox.challenge
}

/// The season of a night (1-based).
pub(crate) fn season_of(def: &ChallengeDef, night: u32) -> u32 {
    (night.max(1) - 1) / def.season_nights.max(1) + 1
}

/// Seasons past the first (0 in Endless).
fn steps(w: &World) -> f32 {
    w.challenge.season.saturating_sub(1) as f32
}

fn pct(p: f32) -> f32 {
    1.0 + p / 100.0
}

/// The goal of season `s` for this town and difficulty, whole 1 000 Kč.
pub(crate) fn goal_for(w: &World, content: &Content, s: u32) -> i64 {
    let d = def(content);
    let size = 1.0 + d.goal_size_weight * (w.town.size - 1.0);
    let diff = d.goal_mul.get(w.difficulty.id()).copied().unwrap_or(1.0);
    let g = d.goal_per_night
        * d.season_nights as f32
        * size
        * diff
        * pct(d.goal_growth).powi(s.saturating_sub(1) as i32);
    ((g / 1000.0).round() * 1000.0) as i64
}

/// Whole 10 Kč.
fn round10(v: f32) -> i64 {
    ((v / 10.0).round() * 10.0) as i64
}

// ---------------------------------------------------------------------------------------------
// Hooks
// ---------------------------------------------------------------------------------------------

/// Day start: a new season begins (night 1, 31, 61, …).
pub(crate) fn on_day_start(w: &mut World, content: &Content) {
    if !enabled(w) || w.challenge.ended.is_some() {
        return;
    }
    let d = def(content).clone();
    let s = season_of(&d, w.night);
    if s == w.challenge.season {
        return;
    }
    let first = w.challenge.season == 0;
    if first {
        w.challenge.supply_index = 1.0;
        w.challenge.wage_index = 1.0;
    } else {
        // inflation (compounding, no cap)
        w.econ.rent = round10(w.econ.rent as f32 * pct(d.rent_inflation));
        w.challenge.supply_index *= pct(d.supply_inflation);
        w.challenge.wage_index *= pct(d.wage_inflation);
    }
    w.challenge.season = s;
    w.challenge.goal = goal_for(w, content, s);
    w.challenge.earned = 0;
    let ctx = EffectCtx::default();
    let season = ("season".to_string(), s.to_string());
    let goal = ("goal".to_string(), w.challenge.goal.to_string());
    if first {
        let strikes = ("max".to_string(), d.strikes_to_close.to_string());
        let nights = ("nights".to_string(), d.season_nights.to_string());
        super::app::bot_with(
            w,
            content,
            "challenge_start",
            &ctx,
            vec![goal, nights, strikes],
        );
    } else {
        let rent = ("amount".to_string(), w.econ.rent.to_string());
        let up = (
            "percent".to_string(),
            format!("{:.0}", d.supply_inflation.max(d.wage_inflation)),
        );
        super::app::bot_with(
            w,
            content,
            "challenge_season",
            &ctx,
            vec![season, goal, rent, up],
        );
    }
    w.log_key(
        "challenge.log.season",
        vec![
            ("season".into(), s.to_string()),
            ("goal".into(), w.challenge.goal.to_string()),
        ],
    );
    if s >= d.competitor_from_season && (w.challenge.rivals.len() as u32) < d.competitors_max {
        let name = rival_name(w, &d, s);
        w.challenge.rivals.push(name.clone());
        super::app::bot_with(
            w,
            content,
            "challenge_rival",
            &ctx,
            vec![("name".into(), name.clone())],
        );
        w.log_key("challenge.log.rival", vec![("name".into(), name)]);
    }
}

/// A rival's name from the town's list (derived RNG: the world's stays untouched).
fn rival_name(w: &World, d: &ChallengeDef, s: u32) -> String {
    let mut rng = Rng::new(w.seed ^ hash_str(&format!("challenge:rival:{s}")));
    let taken = |n: &&String| !w.challenge.rivals.iter().any(|r| r == *n);
    let any = d.competitor_names.get("*");
    let town: Vec<&String> = d
        .competitor_names
        .get(&w.town.name)
        .into_iter()
        .flatten()
        .filter(taken)
        .collect();
    let pool: Vec<&String> = if town.is_empty() {
        any.into_iter().flatten().filter(taken).collect()
    } else {
        town
    };
    rng.pick(&pool)
        .map(|n| (*n).clone())
        .unwrap_or_else(|| format!("Café {}", w.challenge.rivals.len() + 2))
}

/// The café's strengths against the rivals right now.
pub(crate) fn strengths(w: &World, content: &Content) -> Vec<Strength> {
    let d = def(content);
    let mut v = Vec::new();
    if w.sys.atmos.coziness.level >= d.counter_coziness_level {
        v.push(Strength::Coziness);
    }
    if w.econ.reputation >= d.counter_reputation {
        v.push(Strength::Reputation);
    }
    if w.sig.drink.is_some() {
        v.push(Strength::Signature);
    }
    let regulars = w
        .story
        .trust
        .values()
        .filter(|t| **t >= d.counter_regular_trust)
        .count() as u32;
    if regulars >= d.counter_regulars {
        v.push(Strength::Regulars);
    }
    v
}

/// The rivals' share of the walk-ins with these strengths.
pub(crate) fn rival_loss(d: &ChallengeDef, rivals: usize, strengths: usize) -> f32 {
    let kept = (1.0 - d.counter_cut * strengths as f32).clamp(0.0, 1.0);
    (d.competitor_share * rivals as f32 * kept).clamp(0.0, 0.9)
}

/// Night start: tonight's strengths and what the rivals take.
pub(crate) fn on_night_start(w: &mut World, content: &Content) {
    if !active(w) {
        return;
    }
    let st = strengths(w, content);
    w.challenge.loss = rival_loss(def(content), w.challenge.rivals.len(), st.len());
    w.challenge.strengths = st;
}

/// Walk-in multiplier (exactly 1.0 in Endless).
pub(crate) fn demand_mul(w: &World) -> f32 {
    1.0 - w.challenge.loss
}

/// Supply price index (`None` in Endless: the old code path).
pub(crate) fn supply_index(w: &World) -> Option<f32> {
    (active(w) && w.challenge.supply_index > 0.0).then_some(w.challenge.supply_index)
}

/// Wage index (`None` in Endless).
pub(crate) fn wage_index(w: &World) -> Option<f32> {
    (active(w) && w.challenge.wage_index > 0.0).then_some(w.challenge.wage_index)
}

/// Appliance wear multiplier (1.0 in Endless).
pub(crate) fn wear_mul(w: &World, content: &Content) -> f32 {
    if !active(w) {
        return 1.0;
    }
    1.0 + def(content).wear / 100.0 * steps(w)
}

/// How the storyteller leans this season (the identity in Endless and season 1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Storyteller<'a> {
    /// Events weighted like Bad ones although they aren't (`scaled_events`: inspections).
    pub also: &'a [String],
    pub rate: f32,
    pub bad: f32,
    pub good_share: f32,
    pub tension: f32,
    pub bad_in_row: u32,
    pub breath: f32,
}

pub(crate) fn storyteller<'a>(w: &World, content: &'a Content) -> Storyteller<'a> {
    if !active(w) || w.challenge.season <= 1 {
        return Storyteller {
            also: &[],
            rate: 1.0,
            bad: 1.0,
            good_share: 1.0,
            tension: 0.0,
            bad_in_row: 0,
            breath: 1.0,
        };
    }
    let d = def(content);
    let k = steps(w);
    Storyteller {
        also: &d.scaled_events,
        rate: 1.0 + d.event_rate / 100.0 * k,
        bad: 1.0 + d.bad_weight / 100.0 * k,
        good_share: (1.0 - d.good_share_cut / 100.0 * k).max(0.25),
        tension: d.tension_add * k,
        bad_in_row: (w.challenge.season - 1)
            .checked_div(d.bad_in_row_every)
            .unwrap_or(0),
        breath: (1.0 - d.breath_cut / 100.0).max(0.1).powi(k as i32),
    }
}

/// 06:00, after the ledger: the season's tally, the rivals' toll on reputation, the debt
/// limit, the season's end (goal met / strike) and tonight's morning lines.
pub(crate) fn on_night_end(w: &mut World, content: &Content) {
    if !active(w) || w.challenge.ended.is_some() {
        return;
    }
    let d = def(content).clone();
    let earned = w.tonight.revenue + w.tonight.tips;
    let c = &mut w.challenge;
    c.earned += earned;
    c.earned_total += earned;
    c.guests += w.tonight.guests_served as u64;
    c.best_night = c.best_night.max(earned);
    let mut lines: Vec<PayoffLine> = Vec::new();
    let line = |key: &str, args: Vec<(&str, String)>| PayoffLine {
        key: key.into(),
        args: args.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        money: 0,
    };
    // the rivals: walk-ins they took tonight, a little reputation (guests compare)
    if !w.challenge.rivals.is_empty() {
        let loss = w.challenge.loss;
        let kept = rival_loss(&d, w.challenge.rivals.len(), 0);
        let arrived = w.tonight.guests_arrived as f32;
        let lost = (arrived * loss / (1.0 - loss).max(0.1)).round() as u32;
        let n = w.challenge.rivals.len();
        let name = w.challenge.rivals.last().cloned().unwrap_or_default();
        let key = if n == 1 {
            "challenge.line.rival"
        } else {
            "challenge.line.rivals"
        };
        lines.push(line(
            key,
            vec![
                ("name", name),
                ("more", (n - 1).to_string()),
                ("guests", lost.to_string()),
                ("percent", format!("{:.0}", loss * 100.0)),
            ],
        ));
        let held = w.challenge.strengths.len();
        if held > 0 && kept > 0.0 {
            lines.push(line(
                "challenge.line.strengths",
                vec![
                    ("n", held.to_string()),
                    ("percent", format!("{:.0}", (1.0 - loss / kept) * 100.0)),
                ],
            ));
        }
        let drain = d.rival_reputation_drain * n as f32 * (loss / kept.max(0.001));
        w.econ.reputation = (w.econ.reputation - drain).clamp(0.0, 100.0);
    }
    // the debt limit
    let limit = (d.debt_limit_rents * w.econ.rent as f32).round() as i64;
    if w.econ.money < -limit {
        w.challenge.debt_nights += 1;
        lines.push(line(
            "challenge.line.debt",
            vec![
                ("amount", (-limit).to_string()),
                ("n", w.challenge.debt_nights.to_string()),
                ("max", d.debt_nights.to_string()),
            ],
        ));
        if w.challenge.debt_nights == 1 {
            let ctx = EffectCtx::default();
            super::app::bot_with(
                w,
                content,
                "challenge_debt",
                &ctx,
                vec![
                    ("amount".into(), (-limit).to_string()),
                    ("nights".into(), d.debt_nights.to_string()),
                ],
            );
        }
        if w.challenge.debt_nights >= d.debt_nights {
            w.challenge.ended = Some(ChallengeEnd::Debt);
        }
    } else {
        w.challenge.debt_nights = 0;
    }
    // the season's end
    let s = w.challenge.season;
    let last = w.night.is_multiple_of(d.season_nights.max(1));
    let (goal, got) = (w.challenge.goal, w.challenge.earned);
    if last {
        let met = got >= goal;
        w.challenge.strikes = if met { 0 } else { w.challenge.strikes + 1 };
        let result = SeasonResult {
            season: s,
            goal,
            earned: got,
            met,
            money_end: w.econ.money,
            reputation_end: w.econ.reputation,
            rent: w.econ.rent,
            rivals: w.challenge.rivals.len() as u32,
        };
        w.challenge.history.push(result);
        let strikes = w.challenge.strikes.to_string();
        let max = d.strikes_to_close.to_string();
        let key = if met {
            "challenge.line.goal_met"
        } else {
            "challenge.line.goal_missed"
        };
        lines.push(line(
            key,
            vec![
                ("season", s.to_string()),
                ("money", got.to_string()),
                ("goal", goal.to_string()),
                ("strikes", strikes.clone()),
                ("max", max.clone()),
            ],
        ));
        let ctx = EffectCtx::default();
        super::app::bot_with(
            w,
            content,
            if met {
                "challenge_goal_met"
            } else {
                "challenge_goal_missed"
            },
            &ctx,
            vec![
                ("season".into(), s.to_string()),
                ("money".into(), got.to_string()),
                ("goal".into(), goal.to_string()),
                ("strikes".into(), strikes),
                ("max".into(), max),
            ],
        );
        if w.challenge.strikes >= d.strikes_to_close.max(1) && w.challenge.ended.is_none() {
            w.challenge.ended = Some(ChallengeEnd::Strikes);
        }
    } else {
        let left = d.season_nights - (w.night - 1) % d.season_nights.max(1) - 1;
        lines.push(line(
            "challenge.line.goal",
            vec![
                ("season", s.to_string()),
                ("money", got.to_string()),
                ("goal", goal.to_string()),
                ("nights", left.to_string()),
            ],
        ));
    }
    w.tonight.challenge = lines;
}

/// The café closes tonight (story::on_night_end).
pub(crate) fn closes(w: &World) -> bool {
    w.challenge.ended.is_some()
}

impl World {
    /// The running Challenge season (`None` in Endless and the story).
    pub fn challenge(&self, content: &Content) -> Option<ChallengeView> {
        if !active(self) {
            return None;
        }
        let d = def(content);
        let c = &self.challenge;
        let n = d.season_nights.max(1);
        Some(ChallengeView {
            season: c.season,
            goal: c.goal,
            earned: c.earned,
            nights_left: n - (self.night.max(1) - 1) % n,
            strikes: c.strikes,
            strikes_max: d.strikes_to_close,
            rivals: c.rivals.clone(),
            loss: c.loss,
            strengths: c.strengths.clone(),
            debt_nights: c.debt_nights,
            debt_limit: (d.debt_limit_rents * self.econ.rent as f32).round() as i64,
        })
    }

    /// The Challenge's state (seasons played, rivals, history) — `season == 0` in Endless.
    pub fn challenge_state(&self) -> &ChallengeState {
        &self.challenge
    }

    /// The final page of a Challenge run (`None` unless the mode is on).
    pub fn challenge_report(&self) -> Option<ChallengeReport> {
        if !active(self) {
            return None;
        }
        let c = &self.challenge;
        let finished = c.history.len() as u32;
        let survived = if c.ended == Some(ChallengeEnd::Strikes) {
            finished.saturating_sub(1)
        } else {
            finished
        };
        Some(ChallengeReport {
            seed: self.seed,
            seasons_survived: survived,
            nights: self.night,
            best_night: c.best_night,
            guests: c.guests,
            earned: c.earned_total,
            reason: c.ended,
        })
    }
}
