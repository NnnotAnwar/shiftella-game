//! Simulation — pure logic behind [`World`]: café grid & furniture, A*, staff & guests,
//! economy, the Night storyteller, the story director, the in-game Shiftella app, saves, undo.
//!
//! Commands in ([`Command`] via [`World::apply`]), events out ([`SimEvent`] via
//! [`World::drain_events`]). 10 ticks = 1 game minute; the night runs 22:00–06:00.
//! Deterministic: same seed + same commands → same result. No rendering, no macroquad.
//! API & integration guide: `docs/SIM.md`.

mod app;
mod atmos;
mod attention;
mod attic;
mod backroom;
pub mod bot;
mod build_content;
mod build_ux;
mod build_wishes;
pub mod challenge;
mod commands;
pub mod companions;
mod corners;
mod crew;
mod duties;
mod economy;
mod equipment;
pub mod fridge;
pub mod grid;
mod guests;
mod hands;
mod journal;
mod knowledge;
mod life;
mod logic;
mod moments;
mod music;
mod names;
mod narrator;
mod orders;
mod pacing;
mod player;
pub mod purchases;
mod rituals;
pub mod rng;
mod rozpis;
pub mod signature;
mod staff;
mod stories;
mod storm;
mod story;
mod systems;
mod to_go;
mod tool;
mod types;
pub mod walk;
mod world;
mod xlogic;

#[cfg(test)]
pub(crate) mod chaos;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_access;
#[cfg(test)]
mod tests_balance;
#[cfg(test)]
mod tests_bar;
#[cfg(test)]
mod tests_build;
#[cfg(test)]
mod tests_build_ux;
#[cfg(test)]
mod tests_by_hand;
#[cfg(test)]
mod tests_chairs;
#[cfg(test)]
mod tests_challenge;
#[cfg(test)]
mod tests_closing;
#[cfg(test)]
mod tests_combos;
#[cfg(test)]
mod tests_equipment;
#[cfg(test)]
mod tests_floor;
#[cfg(test)]
mod tests_fridge;
#[cfg(test)]
mod tests_hands;
#[cfg(test)]
mod tests_life;
#[cfg(test)]
mod tests_names;
#[cfg(test)]
mod tests_polish;
#[cfg(test)]
mod tests_prep;
#[cfg(test)]
mod tests_purchases;
#[cfg(test)]
mod tests_qa;
#[cfg(test)]
mod tests_rituals;
#[cfg(test)]
mod tests_sandbox;
#[cfg(test)]
mod tests_seats;
#[cfg(test)]
mod tests_signature;
#[cfg(test)]
mod tests_stories;
#[cfg(test)]
mod tests_sys;
#[cfg(test)]
mod tests_t10;
#[cfg(test)]
mod tests_walk;

pub use app::{ChecklistMark, act_ticks, digest_lines};
pub use attention::PATIENCE_NEARLY_OUT;
pub use attic::{AtticOffer, AtticView};
pub use backroom::{BACK_COLS, BackRoomView};
pub use build_content::{BackRoomState, BuildState, WishProgress};
pub use build_ux::{InUseReason, PieceMove};
pub use build_wishes::{BuildRequest, BuildRequestStatus, BuildWho, BuildWish};
pub use challenge::{ChallengeEnd, ChallengeReport, ChallengeView, Strength};
pub use corners::CornerView;
pub use crew::{Activity, Feeling, Feelings, IdleWhy, ShiftTonight, TaskFor, TodoParts};
pub use equipment::{
    Cake, CakeView, EquipAction, EquipCue, EquipKind, EquipState, EquipView, Pot, PotView,
};
pub use grid::{Floor, Grid, Placed, Room, TILE, TileInfo, TileKind};
pub use guests::{FORGIVEN_KEY, LAST_CALL};
pub use hands::*;
pub use life::{HelperDoing, HelperWork, IdleLife, Pastime};
pub use moments::CatView;
pub use names::{Extra, Garment, Persona, StrangerLook, stranger_look};
pub use orders::PLAIN_CREDIT;
pub use player::{MOP_AMOUNT, NOT_IN_YET, WASH_PER_SECOND};
pub use rituals::{Habits, MoodView, RitualHold, RitualStep, ritual_gesture, ritual_key};
pub use signature::{NightPhoto, SIGNATURE_RECIPE, SignatureDrink};
pub use staff::FIRST_HIRE_NIGHT;
pub use stories::{ActiveStory, AnswerDef, GUEST_LEFT, StoriesState, StoryDef, persona_arg};
pub use storm::{StormAct, StormState, TalkDef, TalkLine};
pub use story::{ambient_line_for, talk_available};
pub use systems::*;
pub use types::*;
pub use world::*;

use std::collections::BTreeSet;

use crate::content::handson::{Duty, HallMood};
use crate::content::{Content, Job};

/// The order card proposes Quick for wishes once tonight had this many mini-games (T9: the
/// §9 range is 6–14 a night; regulars' and calm wishes before that are By hand).
pub const BY_HAND_SOFT_CAP: u32 = 12;

/// Cozy: train passengers walk in within this many minutes (Classic: 6; T9: 14 → 24, so a
/// train is a busy stretch, not a wall of tickets).
pub const COZY_TRAIN_SPREAD: u32 = 24;

/// Trains: arrivals by timetable (if `announce_if` holds), passengers come in.
pub(crate) fn trains_tick(w: &mut World, content: &Content) {
    // the snowstorm closed the line (`sim::storm`)
    if storm::line_closed(w) {
        return;
    }
    let m = w.minute();
    for t in &content.trains.trains {
        let at = t.arrives.minutes_since_open();
        if at < 0 || at as u32 != m {
            continue;
        }
        let mut ctx = EffectCtx::default();
        if !logic::all(w, content, &t.announce_if, &mut ctx) {
            continue;
        }
        w.emit(SimEvent::TrainPassing {
            train: t.id.clone(),
            station: t.from.clone(),
        });
        let n =
            (w.rng.range_u32(t.guests.0, t.guests.1) as f32 * guests::story_ramp(w)).round() as u32;
        // after last call (05:15) the train only passes: nobody gets off for the café
        if n > 0 && m + 3 < guests::LAST_CALL {
            let start = w.rng.range_u32(3, 6);
            let mut left = n;
            let group = content.guest(&t.guest).map(|g| g.group).unwrap_or((1, 2));
            // Cozy: the passengers drift in over a longer stretch (a busy moment, not a wall),
            // but not after last call (the 05:20 train used to bring people in at 05:50, right
            // into the closing checklist)
            let spread = if w.difficulty.gentle() {
                COZY_TRAIN_SPREAD.min(guests::LAST_CALL.saturating_sub(m + start + 1).max(1))
            } else {
                6
            };
            while left > 0 {
                let size = w.rng.range_u32(group.0.max(1), group.1.max(1)).min(left);
                let minute = m + start + w.rng.range_u32(0, spread);
                w.spawns.push(guests::PendingSpawn {
                    minute,
                    archetype: t.guest.clone(),
                    size,
                    patience_mul: 1.0,
                    spend_mul: 1.0,
                    orders: Vec::new(),
                    wish: false,
                    crowd: true,
                    held: 0,
                });
                left -= size;
            }
        }
    }
}

/// The preparation half hour (21:30–22:00): a train timetabled before 22:00 only passes by
/// outside — the doors are still closed, nobody gets off for the café. `m` = minutes since
/// 22:00 (negative).
pub(crate) fn prep_trains(w: &mut World, content: &Content, m: i32) {
    for t in &content.trains.trains {
        if t.arrives.minutes_since_open() != m || m >= 0 {
            continue;
        }
        let mut ctx = EffectCtx::default();
        if !logic::all(w, content, &t.announce_if, &mut ctx) {
            continue;
        }
        w.emit(SimEvent::TrainPassing {
            train: t.id.clone(),
            station: t.from.clone(),
        });
    }
}

impl World {
    /// Hover bubble line for a guest (regulars' `ambient_lines`).
    pub fn ambient_line_for(&self, content: &Content, guest: u32) -> Option<String> {
        story::ambient_line_for(self, content, guest)
    }
    /// Talk cutscene id available for this guest tonight (then send `Command::Talk`).
    pub fn talk_available(&self, content: &Content, guest: u32) -> Option<String> {
        story::talk_available(self, content, guest)
    }
}

impl World {
    /// Debug / stress helper: `n` guests of an archetype arrive within `within` minutes.
    pub fn debug_spawn_guests(&mut self, content: &Content, archetype: &str, n: u32, within: u32) {
        guests::schedule_rush(self, content, archetype, n, within, 1.0, 1.0, &[]);
    }
    /// Debug helper: set money (cheats / tests).
    pub fn debug_set_money(&mut self, money: i64) {
        self.econ.money = money;
    }
    /// Debug helper: set the supplies pool (Kč; screenshots of the fridge).
    pub fn debug_set_supplies(&mut self, supplies: i64) {
        self.econ.supplies = supplies.max(0);
    }
    /// Debug helper: pay `amount` for something for the café, as a purchase would (screenshots
    /// of the summary's «bought for the café» line).
    pub fn debug_buy_for_cafe(&mut self, amount: i64) {
        economy::spend_on_cafe(self, amount);
    }
    /// Debug helper: fire an event now (ignores conditions).
    pub fn debug_fire_event(&mut self, content: &Content, id: &str) {
        narrator::fire(self, content, id, EffectCtx::default(), true);
    }
}

// -------------------------------------------------------------------------------------------------
// Hands-on queries (design/handson.md §7.2)
// -------------------------------------------------------------------------------------------------

impl World {
    /// Real seconds per tick at ×1 (`handson.ron`; 0.13 → 10.4 minutes + overlays). Integration:
    /// `acc += dt × speed × overlay; while acc >= this { tick }`.
    /// During the preparation half hour (doors closed) a tick is `prep_tick_scale` of that.
    pub fn real_seconds_per_tick(&self, content: &Content) -> f32 {
        let t = &content.handson.tuning;
        let v = if t.real_seconds_per_tick_x1 > 0.0 {
            t.real_seconds_per_tick_x1
        } else {
            REAL_SECONDS_PER_TICK_X1
        };
        if self.in_prep() || self.in_closing() {
            v * t.prep_tick_scale.clamp(0.3, 1.5)
        } else {
            v
        }
    }
    /// Sim dt multiplier while an overlay (order card, mini-game, book, moment card, the phone
    /// app) is open: Cozy 0.25, Classic 0.35; clamp the speed to ×1 meanwhile.
    pub fn overlay_scale(&self, content: &Content) -> f32 {
        let o = content.handson.tuning.overlay_scale;
        match self.difficulty {
            Difficulty::Cozy | Difficulty::Relaxed => o.cozy,
            Difficulty::Classic => o.classic,
        }
    }
    /// Karolina's queue, current act (with `progress()`), rhythm, idle clock.
    pub fn player(&self) -> &PlayerState {
        &self.hands.player
    }
    /// Karolina's body (position, pose, carrying) — the owner in `staff()`.
    pub fn karolina(&self) -> Option<&Staff> {
        self.staff.iter().find(|s| s.owner)
    }
    pub fn knowledge(&self) -> &Knowledge {
        &self.hands.knowledge
    }
    pub fn known_recipes(&self) -> impl Iterator<Item = &str> {
        self.hands.knowledge.recipes.keys().map(String::as_str)
    }
    /// Karolina knows it and its base item can be made tonight.
    pub fn recipe_available(&self, content: &Content, recipe: &str) -> bool {
        self.recipe_def(content, recipe)
            .is_some_and(|r| orders::recipe_available(self, content, r))
    }
    /// The party a guest (or party id) belongs to.
    pub fn party_of_guest(&self, guest: u32) -> Option<&Party> {
        player::party_of(self, guest).map(|i| &self.parties[i])
    }
    /// Is party `party` in and settled — seated at their table, or standing in the register
    /// line — so its order can be taken? (Walking in from the street: not yet.)
    pub fn party_settled(&self, party: u32) -> bool {
        self.parties
            .iter()
            .find(|p| p.id == party)
            .is_some_and(|p| guests::settled(self, p))
    }
    /// What the guest's party leader ordered.
    pub fn order_kind(&self, guest: u32) -> Option<&OrderKind> {
        self.party_of_guest(guest).map(|p| &p.order_kind)
    }
    /// The order card's chips: the top 4 recipes for the guest's wish (recipe id, match).
    /// Plain orders: the recipes on their pending item (match 1.0).
    pub fn best_matches(&self, content: &Content, guest: u32) -> Vec<(String, f32)> {
        let Some(p) = self.party_of_guest(guest) else {
            return Vec::new();
        };
        if p.wish_open
            && let Some(wish) = p.order_kind.wish().and_then(|w| content.handson.wish(w))
        {
            let mut v = orders::matches_for(self, content, wish);
            v.truncate(4);
            return v;
        }
        let item = self
            .items
            .iter()
            .filter(|i| i.party == p.id && i.state == ItemState::Pending && i.by.is_none())
            .min_by_key(|i| (i.guest != guest, i.id));
        match item {
            Some(i) => content
                .handson
                .recipes_for_item(&i.menu)
                .filter(|r| orders::recipe_available(self, content, r))
                .map(|r| (r.id.clone(), 1.0))
                .take(4)
                .collect(),
            None => Vec::new(),
        }
    }
    /// Quick-brew quality for a recipe (0.6; 0.8 once practised).
    pub fn quick_quality(&self, content: &Content, recipe: &str) -> f32 {
        let t = &content.handson.tuning;
        if self
            .hands
            .knowledge
            .recipes
            .get(recipe)
            .is_some_and(|s| s.practised)
        {
            t.practised_quick_quality
        } else {
            t.quick_quality
        }
    }
    /// The order card's default mode: plain = the last plain choice (Quick at first), wishes
    /// = By hand — but Quick after `by_hand_streak_max` same-kind mini-games in a row (§2.0),
    /// and Quick once tonight already had [`BY_HAND_SOFT_CAP`] mini-games (busy late-story
    /// nights stay 12–14 real minutes; the player can still pick By hand).
    pub fn default_brew_mode(&self, content: &Content, wish: bool, recipe: &str) -> BrewMode {
        if !wish {
            return self.hands.player.last_mode_plain;
        }
        if self.hands.tally.by_hand >= BY_HAND_SOFT_CAP {
            return BrewMode::Quick;
        }
        self.by_hand_streak_mode(content, recipe)
    }
    /// The anti-repetition part of [`World::default_brew_mode`] alone: By hand unless the last
    /// `by_hand_streak_max` mini-games were this recipe's kind.
    pub fn by_hand_streak_mode(&self, content: &Content, recipe: &str) -> BrewMode {
        let kind = content
            .handson
            .recipe(recipe)
            .map(|r| r.playable_minigame(false).0);
        match (self.hands.player.minigame_streak, kind) {
            (Some((k, n)), Some(kind))
                if k == kind && n >= content.handson.tuning.by_hand_streak_max =>
            {
                BrewMode::Quick
            }
            _ => BrewMode::ByHand,
        }
    }
    /// "The usual?" is offered (a trusted regular whose usual she can make).
    pub fn usual_available(&self, content: &Content, guest: u32) -> Option<String> {
        let p = self.party_of_guest(guest)?;
        let r = p.order_kind.usual()?;
        (p.wish_open && self.recipe_available(content, r)).then(|| r.to_string())
    }
    /// The ticket rail: every open order / wish / item, with its owner.
    pub fn ticket_rail(&self, content: &Content) -> Vec<Ticket> {
        orders::ticket_rail(self, content)
    }
    /// Dirty dishes waiting at the sink.
    pub fn sink_dishes(&self) -> u32 {
        self.sink_dishes
    }
    pub fn tips_on_tables(&self) -> &[TipCoin] {
        &self.hands.tips
    }
    /// Lifetime tips in the jar (CZK).
    pub fn tips_lifetime(&self) -> i64 {
        self.hands.tips_lifetime
    }
    pub fn hall_mood(&self, content: &Content) -> Option<HallMood> {
        music::hall_mood(self, content)
    }
    /// A rush is on (trains, crowds): the rail sorts by wait, the house music shifts.
    pub fn rush(&self) -> bool {
        self.hands.rush.until.is_some()
    }
    /// Karolina's rhythm streak (tips +4 % a step).
    pub fn rhythm(&self) -> u8 {
        self.hands.player.rhythm
    }
    pub fn staff_duties(&self, staff: u32) -> &[Duty] {
        self.staff_by_id(staff)
            .map(|s| s.duties.as_slice())
            .unwrap_or(&[])
    }
    /// How many duties this helper can hold (1–3).
    pub fn capacity(&self, content: &Content, staff: u32) -> u32 {
        self.staff_by_id(staff)
            .map(|s| {
                if s.owner {
                    0
                } else {
                    duties::capacity(content, s, self.night, None)
                }
            })
            .unwrap_or(0)
    }
    /// The duty board's "Suggest": the best free duty for this helper by skill.
    pub fn suggest_duty(&self, staff: u32) -> Option<Duty> {
        let s = self.staff_by_id(staff)?;
        Duty::ALL
            .iter()
            .copied()
            .filter(|d| !s.duties.contains(d) && !duties::duty_tonight(self, *d))
            .max_by(|a, b| {
                s.skill(a.job(), self.night)
                    .total_cmp(&s.skill(b.job(), self.night))
            })
    }
    pub fn music(&self) -> &MusicState {
        &self.hands.music
    }
    /// The track the café plays now and its volume (`None` = silence: a blackout).
    pub fn music_track(&self, content: &Content) -> Option<(String, f32)> {
        music::track(self, content)
    }
    pub fn records_unlocked(&self) -> &BTreeSet<String> {
        &self.hands.music.unlocked
    }
    /// Things only Karolina will do right now (rail + markers + dirt).
    pub fn karolina_todo(&self, content: &Content) -> u32 {
        pacing::todo(self, content)
    }
    /// The quiet hour (03:00–04:00).
    pub fn quiet_hour(&self, content: &Content) -> bool {
        pacing::quiet_hour(self, content)
    }
    /// Tonight's hands-on tallies (balance harness, digest).
    pub fn hands_tally(&self) -> &HandsTally {
        &self.hands.tally
    }
    pub fn assist_wishes(&self) -> bool {
        self.hands.assist_wishes
    }
    /// Dirt blots to render and mop: one per 0.1 of hall dirt (max 6), on fixed hall tiles.
    pub fn dirt_spots(&self) -> Vec<(i32, i32)> {
        let n = ((self.dirt - 0.02) / MOP_AMOUNT).ceil().clamp(0.0, 6.0) as usize;
        if n == 0 {
            return Vec::new();
        }
        let hall: Vec<(i32, i32)> = (0..self.grid.height)
            .flat_map(|y| (0..self.grid.width).map(move |x| (x, y)))
            .filter(|t| {
                self.grid.room(t.0, t.1) == Room::Hall
                    && self.grid.walkable(t.0, t.1)
                    && self.grid.object_at(t.0, t.1).is_none()
            })
            .collect();
        if hall.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut k = 0u64;
        while out.len() < n && k < 64 {
            let h = rng::hash_str(&format!("dirt{}:{k}", self.night));
            let t = hall[(h % hall.len() as u64) as usize];
            if !out.contains(&t) {
                out.push(t);
            }
            k += 1;
        }
        out
    }
    /// The job that makes an order item.
    pub(crate) fn item_job(&self, content: &Content, item: &OrderItem) -> Option<Job> {
        content.menu_item(&item.menu).map(|m| m.job)
    }
}

// -------------------------------------------------------------------------------------------------
// T5 queries: moments, recipes, extras, atmosphere, journal, rozpis, the app as a tool
// -------------------------------------------------------------------------------------------------

impl World {
    /// Every T5 system's state (read-only).
    pub fn systems(&self) -> &SystemsState {
        &self.sys
    }
    /// What Karolina will do at the end of her queued `GoTo` acts (moment options, night
    /// experiments) — for queue icons (`WalkKind::id`).
    pub fn walk_jobs(&self) -> &[WalkJob] {
        &self.sys.moments.walks
    }
    /// Moments on the map now (markers; `resolving` = Karolina is on her way).
    pub fn active_moments(&self) -> &[ActiveMoment] {
        &self.sys.moments.active
    }
    /// The options a moment's card shows right now (`show_if`, `within_minutes`).
    pub fn moment_options(&self, content: &Content, id: &str) -> Vec<String> {
        moments::options_now(self, content, id)
    }
    /// Unlocked pantry jars.
    pub fn pantry(&self) -> &BTreeSet<String> {
        &self.hands.knowledge.ingredients
    }
    /// Ingredients shown on an unknown recipe's silhouette (data + margin notes).
    pub fn revealed_ingredients(&self, content: &Content, recipe: &str) -> Vec<String> {
        content
            .handson
            .recipe(recipe)
            .map(|r| knowledge::revealed_of(self, r))
            .unwrap_or_default()
    }
    /// Supplies one experiment costs now (0 on Cozy nights 1–7).
    pub fn experiment_cost(&self, content: &Content) -> i64 {
        knowledge::cost(self, content)
    }
    /// The recipe grandma's next margin note is about (the lull chip).
    pub fn margin_note_target(&self, content: &Content) -> Option<String> {
        knowledge::margin_note_target(self, content).map(|r| r.id.clone())
    }
    /// The hall's coziness meter (value 0–100, level 1–5, parts, weakest part).
    pub fn coziness(&self) -> &Coziness {
        &self.sys.atmos.coziness
    }
    /// Tonight's season phase (`seasons.ron`): visuals for render, wish multipliers.
    pub fn season_phase<'a>(
        &self,
        content: &'a Content,
    ) -> Option<&'a crate::content::handson::PhaseDef> {
        atmos::phase(self, content)
    }
    /// Garlands render now (allowed by the phase and hung).
    pub fn garlands_up(&self, content: &Content) -> bool {
        atmos::garlands_up(self, content)
    }
    /// The hall's decor theme id.
    pub fn theme<'a>(&'a self, content: &'a Content) -> &'a str {
        atmos::theme_id(self, content)
    }
    pub fn theme_owned(&self, content: &Content, id: &str) -> bool {
        atmos::theme_owned(self, content, id)
    }
    /// Themes that may be bought (or are owned).
    pub fn themes_unlocked(&self) -> &BTreeSet<String> {
        &self.sys.atmos.unlocked
    }
    /// Latte patterns unlocked by seasons, the cat and moments.
    pub fn lattes(&self) -> &BTreeSet<String> {
        &self.sys.atmos.lattes
    }
    /// A regular's ambient line when grandma's theme has been swapped away (`MissedBy`).
    pub fn theme_missed_line(&self, content: &Content, regular: &str) -> Option<String> {
        atmos::missed_line(self, content, regular)
    }
    /// The station cat (render pose by perch), if it is around.
    pub fn station_cat(&self, content: &Content) -> Option<CatView> {
        moments::cat_view(self, content)
    }
    pub fn cat_level(&self) -> u32 {
        self.sys.extras.cat_level
    }
    /// Thank-you napkins collected (`thanks.<id>`), in order.
    pub fn napkins(&self) -> &[String] {
        &self.sys.extras.napkins
    }
    /// "Count the jar": the pending goal and its two decor choices.
    pub fn tip_goal_pending(&self, content: &Content) -> Option<(u32, Vec<String>)> {
        let i = self.sys.extras.tip_goal_pending?;
        let g = content.handson.extras.tip_goals.get(i as usize)?;
        Some((i, g.choices.clone()))
    }
    /// Tonight's specials board.
    pub fn specials(&self) -> &[String] {
        &self.sys.extras.specials
    }
    /// Kept polaroids, oldest first.
    pub fn journal(&self) -> &[JournalEntry] {
        &self.sys.journal.entries
    }
    /// Tonight's best candidates so far.
    pub fn tonight_highlights(&self) -> &[Highlight] {
        &self.sys.journal.tonight
    }
    /// The morning's choice (night, candidates best first) until `Command::PickPolaroid`.
    pub fn pending_polaroids(&self) -> Option<(u32, &[Highlight])> {
        self.sys
            .journal
            .pending
            .as_ref()
            .map(|(n, v)| (*n, v.as_slice()))
    }
    /// A person's preferences this week (chips).
    pub fn prefs(&self, staff: u32) -> Vec<crate::content::handson::Pref> {
        rozpis::prefs_of(self, staff)
    }
    /// How each preference fares on the draft.
    pub fn pref_status(
        &self,
        content: &Content,
        staff: u32,
    ) -> Vec<(crate::content::handson::Pref, PrefStatus)> {
        let rows = rozpis::effective(self, content, &self.app.draft);
        let Some(row) = rows.get(&staff) else {
            return Vec::new();
        };
        rozpis::prefs_of(self, staff)
            .into_iter()
            .map(|p| {
                let st = rozpis::pref_status(self, &rows, staff, row, &p);
                (p, st)
            })
            .collect()
    }
    /// Wellbeing on the draft (0–100; rings from `handson.ron` → rozpis.rings).
    pub fn wellbeing(&self, staff: u32) -> Option<f32> {
        self.sys.rozpis.wellbeing.get(&staff).copied()
    }
    /// `mean(wb) − 0.5 × stdev(wb)` of the draft.
    pub fn week_fairness(&self) -> f32 {
        self.sys.rozpis.fairness
    }
    /// The coverage footer (suggested vs on, cups per night).
    pub fn coverage(&self, content: &Content) -> [CoverageNight; 7] {
        rozpis::coverage(self, content)
    }
    /// The last auto-fill's result line.
    pub fn autofill_report(&self) -> &AutofillReport {
        &self.sys.rozpis.last_report
    }
    /// "Can I go at 2?" requests.
    pub fn early_leave_requests(&self) -> &[EarlyLeaveRequest] {
        &self.sys.rozpis.early_leave
    }
    /// Why a request exists (life event reason, a Need).
    pub fn request_meta(&self, id: u32) -> Option<&RequestMeta> {
        self.sys.rozpis.requests.get(&id)
    }
    /// Shifts two staff archetypes have shared (≥ 4 h together).
    pub fn shared_shifts(&self, a: &str, b: &str) -> u32 {
        rozpis::shared_shifts(self, a, b)
    }
    /// Handover suggestions for tomorrow (pick ≤ 2 with `Command::PickHandover`).
    pub fn handover_suggestions(&self, content: &Content) -> Vec<HandoverSuggestion> {
        if self.sys.tool.suggestions.is_empty() {
            tool::suggestions(self, content)
        } else {
            self.sys.tool.suggestions.clone()
        }
    }
    pub fn handover_picks(&self) -> &[String] {
        &self.sys.tool.picks
    }
    /// The payroll screen's tip split (remembered; the content default at first).
    pub fn tip_split(&self, content: &Content) -> crate::content::handson::TipSplit {
        tool::tip_split(self, content)
    }
    pub fn payroll_sent(&self, week: u32) -> bool {
        self.sys.tool.payroll_sent.contains(&week)
    }
    /// The radio's track now: (track, volume) while a moment plays it.
    pub fn radio(&self) -> Option<(&str, f32)> {
        let m = self.minute();
        self.sys
            .fx
            .radio
            .as_ref()
            .filter(|r| self.phase == Phase::Night && r.2 > m)
            .map(|r| (r.0.as_str(), r.1))
    }
    /// The open window, the busker outside (render flavour).
    pub fn window_open(&self) -> bool {
        self.sys
            .fx
            .window_open_until
            .is_some_and(|u| self.phase == Phase::Night && u > self.minute())
    }
    pub fn street_music(&self) -> bool {
        self.sys
            .fx
            .street_music_until
            .is_some_and(|u| self.phase == Phase::Night && u > self.minute())
    }
    /// Candles on the tables (a blackout moment).
    pub fn candles_lit(&self) -> bool {
        self.sys.fx.candles && self.blackout_until.is_some()
    }
    /// The order card's "remember" mark for a regular tonight.
    pub fn remembered(&self, regular: &str) -> bool {
        self.sys.fx.remember.contains_key(regular)
    }
    /// Seats reserved for regulars (regular → seat uid).
    pub fn reserved_seats(&self) -> &std::collections::BTreeMap<String, u32> {
        &self.sys.tool.reserved
    }
}
