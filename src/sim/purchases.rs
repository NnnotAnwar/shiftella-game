//! Purchases that matter: what the pieces the player buys do, in numbers the player feels and
//! sees (data: `assets/data/purchases.ron`, compiled in).
//!
//! * **Seats → guests.** Hall seats set how many walk in (`seat_draw`, diminishing returns)
//!   and how many fit (a full café turns parties away: toast `purchases.toast.no_seat`).
//! * **Comfort → stay & tips.** A party's mean seat comfort scales their stay and tips, adds a
//!   chance of a second round and a little satisfaction (hard stools the opposite).
//! * **Coziness tiers.** The meter's five levels each carry perks (tips, stay, reviews, a rare
//!   night guest, regulars' "like home" wishes).
//! * **Appliances by tag.** A second machine brews in parallel, a grinder lifts quick brews, a
//!   dish rack / second sink washes faster, an oven puts cakes on the menu.
//! * **The payoff.** `lines` → `NightSummary.payoff`, "What the café gave you tonight".
//!
//! Grandma's café is the neutral point of every curve: a player who never builds plays the old
//! numbers (and draws nothing extra from the world RNG).

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use super::grid::{self, Room};
use super::types::{SimEmote, SimEvent};
use super::world::*;
use crate::content::{Content, FurnitureDef, MenuCategory};

// ---------------------------------------------------------------------------------------------
// Data
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct PurchasesData {
    pub seats: SeatsDef,
    pub comfort: Vec<ComfortPoint>,
    pub comfy_at: f32,
    pub hard_below: f32,
    pub round_stay: (u32, u32),
    pub round_until: u32,
    pub tiers: Vec<TierDef>,
    pub appliances: Vec<ApplianceDef>,
    pub summary_max: usize,
    #[serde(default)]
    pub to_go: ToGoDef,
}

/// No free seat → a paper cup to go (`guests::to_go`).
#[derive(Debug, Clone, Deserialize)]
pub struct ToGoDef {
    /// The share of seatless plain parties who take it to go (Cozy / Carefree; Classic).
    pub chance: f32,
    pub classic_chance: f32,
    /// What goes in a paper cup / bag.
    pub categories: Vec<MenuCategory>,
    /// Their tips ×.
    pub tips: f32,
    /// They don't join a register line this long.
    pub max_queue: usize,
    /// Guests waiting by the register for their cups at once.
    pub max_waiting: usize,
}

impl Default for ToGoDef {
    /// Off (a broken or old data file: they walk on as before).
    fn default() -> Self {
        ToGoDef {
            chance: 0.0,
            classic_chance: 0.0,
            categories: Vec::new(),
            tips: 1.0,
            max_queue: 0,
            max_waiting: 0,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SeatsDef {
    pub base: u32,
    pub exponent: f32,
    pub min_draw: f32,
    pub max_draw: f32,
    pub toast_gap: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct ComfortPoint {
    pub at: f32,
    pub stay: f32,
    pub tips: f32,
    pub round: f32,
    pub satisfaction: f32,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TierDef {
    pub level: u32,
    #[serde(default)]
    pub tips: f32,
    #[serde(default)]
    pub stay: f32,
    #[serde(default)]
    pub review: f32,
    #[serde(default)]
    pub round: f32,
    #[serde(default)]
    pub rare: Vec<(String, f32)>,
    #[serde(default)]
    pub wishes: bool,
}

impl TierDef {
    /// Nothing extra at this tier.
    pub fn is_plain(&self) -> bool {
        self.tips == 0.0
            && self.stay == 0.0
            && self.review == 0.0
            && self.round == 0.0
            && self.rare.is_empty()
            && !self.wishes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub enum ApplianceRole {
    /// Two (or more) pieces with the tag work at once.
    Parallel,
    /// The tag's menu items exist only with the piece (the oven's cakes).
    Unlocks,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApplianceDef {
    pub tag: String,
    pub role: ApplianceRole,
    pub free: u32,
    pub max: u32,
}

const DATA: &str = include_str!("../../assets/data/purchases.ron");

/// The purchases data (parsed once; a broken file is a test failure, at runtime neutral).
pub fn data() -> &'static PurchasesData {
    static D: OnceLock<PurchasesData> = OnceLock::new();
    D.get_or_init(|| {
        parse(DATA).unwrap_or_else(|e| {
            eprintln!("error: purchases.ron: {e}");
            PurchasesData {
                seats: SeatsDef {
                    base: 16,
                    exponent: 0.0,
                    min_draw: 1.0,
                    max_draw: 1.0,
                    toast_gap: 60,
                },
                comfort: Vec::new(),
                comfy_at: 99.0,
                hard_below: -1.0,
                round_stay: (0, 0),
                round_until: 0,
                tiers: Vec::new(),
                appliances: Vec::new(),
                summary_max: 0,
                to_go: ToGoDef::default(),
            }
        })
    })
}

pub(crate) fn parse(text: &str) -> Result<PurchasesData, String> {
    ron::from_str(text).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------------------------

/// What one kind of seat did tonight (by furniture def id).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SeatTally {
    pub parties: u32,
    /// Minutes of stay the seat's comfort added (negative: a hard seat).
    pub stay_minutes: f32,
    /// Kč of tips the comfort added (negative: a hard seat).
    pub tips: i64,
    pub rounds: u32,
}

/// Tonight's tally (reset at night start).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PayoffTally {
    /// Parties that only found room on a bought piece, their guests and what they paid.
    pub extra_parties: u32,
    pub extra_guests: u32,
    pub extra_money: i64,
    /// Walk-ins the seat draw added (fractional, summed per minute).
    pub drawn: f32,
    /// Parties that went on because every seat was taken.
    pub turned_away: u32,
    /// Parties (and their guests) who found no seat and took their order to go, and what
    /// they paid (bill + tips).
    pub to_go: u32,
    pub to_go_guests: u32,
    pub to_go_money: i64,
    pub by_seat: BTreeMap<String, SeatTally>,
    /// Items started at a station while another piece of the same tag was busy.
    pub parallel: BTreeMap<String, u32>,
    /// Items of an `Unlocks` tag served, and their menu price.
    pub made: BTreeMap<String, (u32, i64)>,
    /// Tip Kč the coziness tier added.
    pub tier_tips: i64,
    /// Night minutes spent at each meter level (index level − 1).
    pub tier_minutes: [u32; 5],
    /// Rare guests the tier drew in.
    pub rare_guests: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PayoffState {
    pub tonight: PayoffTally,
    /// Night minute of the last "No free seat" toast.
    pub last_toast: Option<u32>,
}

/// One line of "What the café gave you tonight": `purchases.line.<kind>` + its args.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PayoffLine {
    pub key: String,
    /// Named args; a value starting with `@` is another i18n key (a furniture / level name).
    pub args: Vec<(String, String)>,
    /// Kč this line stands for (sorting, "the lines add up"); 0 when it isn't money.
    pub money: i64,
}

pub(crate) fn on_night_start(w: &mut World) {
    w.pay.tonight = PayoffTally::default();
    w.pay.last_toast = None;
}

pub(crate) fn on_minute(w: &mut World) {
    if !w.doors_open() {
        return;
    }
    let l = w.sys.atmos.coziness.level.clamp(1, 5) as usize;
    w.pay.tonight.tier_minutes[l - 1] += 1;
}

// ---------------------------------------------------------------------------------------------
// Seats → guests
// ---------------------------------------------------------------------------------------------

/// Hall seats right now (tables, sofas, window seats…; the stools at the bar hold a guest
/// when the tables are full but draw nobody in, so grandma's 16 stay the neutral point).
pub(crate) fn hall_seats(w: &World, content: &Content) -> u32 {
    super::guests::seats(w, content)
        .iter()
        .filter(|s| !s.bar && w.grid.room(s.tile.0, s.tile.1) == Room::Hall)
        .count() as u32
}

/// The walk-in multiplier of `seats` hall seats (1.0 at grandma's count).
pub fn draw_for(seats: u32) -> f32 {
    let d = &data().seats;
    if d.base == 0 || seats == d.base {
        return 1.0;
    }
    (seats as f32 / d.base as f32)
        .powf(d.exponent)
        .clamp(d.min_draw, d.max_draw)
}

/// Walk-ins × now.
pub(crate) fn seat_draw(w: &World, content: &Content) -> f32 {
    draw_for(hall_seats(w, content))
}

/// A walk-in rate was rolled with `draw` (the share it added counts as drawn guests).
pub(crate) fn note_draw(w: &mut World, rate: f32, draw: f32) {
    if draw > 1.0 {
        w.pay.tonight.drawn += rate * (1.0 - 1.0 / draw);
    }
}

/// The last uid of grandma's own furniture (pieces with a larger uid were bought / brought).
fn grandma_last_uid(content: &Content) -> u32 {
    static N: OnceLock<u32> = OnceLock::new();
    *N.get_or_init(|| {
        let mut n = 1;
        let _ = grid::default_cafe(content, &mut n);
        n - 1
    })
}

pub(crate) fn bought(content: &Content, uid: u32) -> bool {
    uid > grandma_last_uid(content)
}

/// Would grandma's own seats have room for a party of `size` (a free table of hers with all
/// its seats free, or that many free seats of hers without a table)?
pub(crate) fn grandma_has_room(w: &World, content: &Content, size: usize) -> bool {
    let all = super::guests::seats(w, content);
    let mut tables: BTreeMap<u32, (usize, bool)> = BTreeMap::new();
    let mut loose = 0;
    for s in all.iter().filter(|s| !bought(content, s.uid)) {
        match s.table {
            Some(t) if !bought(content, t) => {
                let e = tables.entry(t).or_insert((0, true));
                e.0 += 1;
                e.1 &= s.free;
            }
            _ if s.free => loose += 1,
            _ => {}
        }
    }
    loose >= size || tables.values().any(|(n, free)| *free && *n >= size)
}

/// A party went on: every seat was taken (the toast, at most once per `toast_gap`).
pub(crate) fn on_no_seat(w: &mut World) {
    w.pay.tonight.turned_away += 1;
    seat_toast(w, "purchases.toast.no_seat");
}

/// A party found no seat and takes it to go (the same gentle toast clock as `on_no_seat`).
pub(crate) fn on_to_go(w: &mut World, size: u32) {
    w.pay.tonight.to_go += 1;
    w.pay.tonight.to_go_guests += size;
    w.tonight.guests_to_go += size;
    seat_toast(w, "purchases.toast.to_go");
}

/// "No free seat" toasts share one clock: at most one per `toast_gap` minutes.
fn seat_toast(w: &mut World, key: &str) {
    let m = w.minute();
    let gap = data().seats.toast_gap;
    if w.pay.last_toast.is_none_or(|t| m >= t + gap) {
        w.pay.last_toast = Some(m);
        w.emit(SimEvent::Toast { key: key.into() });
    }
}

// ---------------------------------------------------------------------------------------------
// Comfort → stay, tips, a second round
// ---------------------------------------------------------------------------------------------

/// The comfort curve at `c` (linear between the points; neutral without data).
pub fn comfort_at(c: f32) -> ComfortPoint {
    let pts = &data().comfort;
    let neutral = ComfortPoint {
        at: c,
        stay: 1.0,
        tips: 1.0,
        round: 0.0,
        satisfaction: 0.0,
    };
    let (Some(first), Some(last)) = (pts.first(), pts.last()) else {
        return neutral;
    };
    if c <= first.at {
        return *first;
    }
    if c >= last.at {
        return *last;
    }
    for p in pts.windows(2) {
        let (a, b) = (p[0], p[1]);
        if c >= a.at && c <= b.at {
            let k = if b.at > a.at {
                (c - a.at) / (b.at - a.at)
            } else {
                0.0
            };
            let l = |x: f32, y: f32| x + (y - x) * k;
            return ComfortPoint {
                at: c,
                stay: l(a.stay, b.stay),
                tips: l(a.tips, b.tips),
                round: l(a.round, b.round),
                satisfaction: l(a.satisfaction, b.satisfaction),
            };
        }
    }
    neutral
}

/// The seat pieces under a party (furniture uids, one per seated member).
fn party_seats(w: &World, p: &Party) -> Vec<u32> {
    p.members
        .iter()
        .filter_map(|m| w.guests.iter().find(|g| g.id == *m)?.seat.map(|s| s.0))
        .collect()
}

/// (mean seat comfort, the leader's seat def id) of a seated party.
pub(crate) fn party_comfort(w: &World, content: &Content, pi: usize) -> Option<(f32, String)> {
    let p = w.parties.get(pi)?;
    let uids = party_seats(w, p);
    let defs: Vec<&FurnitureDef> = uids
        .iter()
        .filter_map(|u| content.furniture_def(&w.furniture_by_uid(*u)?.def))
        .collect();
    let first = defs.first()?;
    let mean = defs.iter().map(|d| d.comfort).sum::<f32>() / defs.len() as f32;
    Some((mean, first.id.clone()))
}

/// The seat def a party's comfort effect is booked on in the morning's lines — only a piece
/// the player bought (grandma's own bar stools are part of the café, not a purchase).
fn booked(w: &World, content: &Content, pi: usize, seat: Option<(f32, String)>) -> Option<String> {
    let leader = w.parties.get(pi)?.leader()?;
    let (uid, _) = w.guests.iter().find(|g| g.id == leader)?.seat?;
    bought(content, uid).then_some(seat?.1)
}

fn party_point(w: &World, content: &Content, pi: usize) -> ComfortPoint {
    match party_comfort(w, content, pi) {
        Some((c, _)) => comfort_at(c),
        None => comfort_at(2.0),
    }
}

/// The comfort's satisfaction add once seated & served.
pub(crate) fn comfort_satisfaction(c: f32) -> f32 {
    comfort_at(c).satisfaction
}

/// The tier perks right now.
pub fn tier_of(level: u32) -> TierDef {
    data()
        .tiers
        .iter()
        .find(|t| t.level == level)
        .cloned()
        .unwrap_or_default()
}

pub(crate) fn tier(w: &World) -> TierDef {
    tier_of(w.sys.atmos.coziness.level)
}

/// Stay × from the seat and the tier; `base` minutes → the added minutes are booked on the
/// seat's tally. Returns the multiplier.
pub(crate) fn stay_mul(w: &mut World, content: &Content, pi: usize, base: f32) -> f32 {
    let seat = party_comfort(w, content, pi);
    let cs = seat.as_ref().map_or(1.0, |(c, _)| comfort_at(*c).stay);
    let ts = 1.0 + tier(w).stay;
    if let Some(def) = booked(w, content, pi, seat)
        && (cs - 1.0).abs() > 1e-4
    {
        let t = w.pay.tonight.by_seat.entry(def).or_default();
        t.parties += 1;
        t.stay_minutes += base * (cs - 1.0);
    }
    cs * ts
}

/// Tips × from the seat and the tier.
pub(crate) fn tips_mul(w: &World, content: &Content, pi: usize) -> f32 {
    party_point(w, content, pi).tips * (1.0 + tier(w).tips)
}

/// A tip of `tip` Kč was left: books what the seat and the tier added.
pub(crate) fn note_tip(w: &mut World, content: &Content, pi: usize, tip: i64) {
    let seat = party_comfort(w, content, pi);
    let cs = seat.as_ref().map_or(1.0, |(c, _)| comfort_at(*c).tips);
    let ts = 1.0 + tier(w).tips;
    let total = cs * ts;
    if total <= 0.0 {
        return;
    }
    let base = tip as f32 / total;
    // the seat's share first, then the tier's on top of it
    let seat_part = base * (cs - 1.0);
    let tier_part = base * cs * (ts - 1.0);
    if let Some(def) = booked(w, content, pi, seat)
        && (cs - 1.0).abs() > 1e-4
    {
        w.pay.tonight.by_seat.entry(def).or_default().tips += seat_part.round() as i64;
    }
    w.pay.tonight.tier_tips += tier_part.round() as i64;
}

/// Positive reviews × (the tier).
pub(crate) fn review_mul(w: &World) -> f32 {
    1.0 + tier(w).review
}

/// Extra arrival weight of an archetype at this tier.
pub(crate) fn rare_weight(w: &World, archetype: &str) -> f32 {
    if !w.doors_open() {
        return 0.0;
    }
    tier(w)
        .rare
        .iter()
        .filter(|(a, _)| a == archetype)
        .map(|(_, x)| *x)
        .sum()
}

/// A party arrived: a rare tier guest is counted.
pub(crate) fn on_arrived(w: &mut World, archetype: &str) {
    if tier(w).rare.iter().any(|(a, _)| a == archetype) {
        w.pay.tonight.rare_guests += 1;
    }
}

/// The chance that an eating party orders a second round now.
pub(crate) fn round_chance(w: &World, content: &Content, pi: usize) -> f32 {
    (party_point(w, content, pi).round + tier(w).round).clamp(0.0, 0.95)
}

/// An eating party's stay ran out: maybe one more round (a drink like their first one).
/// True = they stay and order (nothing is drawn from the RNG when the chance is 0, so
/// grandma's chairs play exactly as before).
pub(crate) fn second_round(w: &mut World, content: &Content, pid: u32) -> bool {
    let m = w.minute();
    let Some(pi) = w.parties.iter().position(|p| p.id == pid) else {
        return false;
    };
    {
        let p = &w.parties[pi];
        if p.rounds > 0
            || p.to_go
            || !p.served
            || p.wish_open
            || p.regular.is_some()
            || p.state != PartyState::Eating
            || m >= data().round_until
            || m >= super::guests::LAST_CALL
            || p.leave_by.is_some_and(|lb| m + 10 >= lb)
            || super::storm::line_closed(w)
        {
            return false;
        }
    }
    let chance = round_chance(w, content, pi);
    if chance <= 0.0 || !w.rng.chance(chance) {
        return false;
    }
    // the same again: their first drink, else a favourite drink, else any drink
    let served: Vec<String> = w.parties[pi]
        .items
        .iter()
        .filter_map(|i| w.items.iter().find(|x| x.id == *i))
        .filter(|x| x.state == ItemState::Served)
        .map(|x| x.menu.clone())
        .collect();
    let arch = content.guest(&w.parties[pi].archetype);
    let drink = |id: &str| {
        content.menu_item(id).filter(|mi| {
            matches!(mi.category, MenuCategory::Drink)
                && super::guests::item_orderable(w, content, mi)
        })
    };
    let pick = served
        .iter()
        .find_map(|s| drink(s))
        .or_else(|| arch.and_then(|a| a.favourites.iter().find_map(|f| drink(f))))
        .or_else(|| content.menu.iter().find_map(|mi| drink(&mi.id)))
        .map(|mi| (mi.id.clone(), mi.cost, mi.prep_minutes, mi.job));
    let Some((menu, cost, prep_minutes, job)) = pick else {
        return false;
    };
    let Some(leader) = w.parties[pi].leader() else {
        return false;
    };
    let iid = w.alloc_id();
    w.econ.supplies -= cost as i64;
    let prep = prep_minutes
        * TICKS_PER_MINUTE as f32
        * super::guests::prep_mul(w, job, &menu)
        * super::equipment::item_prep_mul(w, content, &menu, true);
    w.items.push(OrderItem {
        id: iid,
        party: pid,
        guest: leader,
        menu: menu.clone(),
        state: ItemState::Pending,
        prep_left: prep,
        by: None,
        ready_tile: None,
        quality: 0.0,
        created: m,
        origin: Default::default(),
        price_add: 0,
        hand_off_match: None,
        wish: false,
    });
    let food_patience = super::guests::food_patience(w);
    let def = party_comfort(w, content, pi).map(|x| x.1);
    let p = &mut w.parties[pi];
    p.items.push(iid);
    p.rounds += 1;
    p.state = PartyState::WaitFood;
    p.ordered_at = Some(m);
    p.wait_at_order = p.wait_clock;
    p.patience = (p.patience_max * food_patience).max(20.0);
    let tick = w.tick;
    if let Some(g) = w.guests.iter_mut().find(|g| g.id == leader) {
        g.emote = Some((SimEmote::Coffee, tick + 30));
    }
    if let Some(d) = def {
        w.pay.tonight.by_seat.entry(d).or_default().rounds += 1;
    }
    let _ = menu;
    true
}

// ---------------------------------------------------------------------------------------------
// Appliances by tag
// ---------------------------------------------------------------------------------------------

fn count_tag(w: &World, content: &Content, tag: &str) -> u32 {
    w.furniture
        .iter()
        .filter(|p| {
            content
                .furniture_def(&p.def)
                .is_some_and(|d| d.has_tag(tag))
        })
        .count() as u32
}

/// Someone starts work at station `uid`: another piece of the same `Parallel` tag busy at the
/// same time = a drink brewed side by side.
pub(crate) fn note_station_start(w: &mut World, content: &Content, uid: u32) {
    let Some(d) = w
        .furniture_by_uid(uid)
        .and_then(|p| content.furniture_def(&p.def))
    else {
        return;
    };
    for a in &data().appliances {
        if a.role != ApplianceRole::Parallel || !d.has_tag(&a.tag) {
            continue;
        }
        let other_busy = w.furniture.iter().any(|p| {
            p.uid != uid
                && p.user.is_some()
                && content
                    .furniture_def(&p.def)
                    .is_some_and(|x| x.has_tag(&a.tag))
        });
        if other_busy {
            *w.pay.tonight.parallel.entry(a.tag.clone()).or_default() += 1;
        }
    }
}

/// An item was served: one of an `Unlocks` tag counts (the oven's cakes).
pub(crate) fn on_item_served(w: &mut World, content: &Content, menu: &str) {
    let Some(mi) = content.menu_item(menu) else {
        return;
    };
    if data()
        .appliances
        .iter()
        .any(|a| a.role == ApplianceRole::Unlocks && a.tag == mi.station)
    {
        let price = w.price_of(content, menu) as i64;
        let e = w.pay.tonight.made.entry(mi.station.clone()).or_default();
        e.0 += 1;
        e.1 += price;
    }
}

/// A party paid and left: the extra-seat tally.
pub(crate) fn on_party_paid(w: &mut World, pi: usize, served: bool, money: i64) {
    let p = &w.parties[pi];
    if p.to_go && served {
        w.pay.tonight.to_go_money += money;
    }
    if p.extra_seat && served {
        let size = p.members.len() as u32;
        let t = &mut w.pay.tonight;
        t.extra_parties += 1;
        t.extra_guests += size;
        t.extra_money += money;
    }
}

// ---------------------------------------------------------------------------------------------
// The morning: "What the café gave you tonight"
// ---------------------------------------------------------------------------------------------

/// The first placed piece with `tag` (for its name in a line).
fn piece_with_tag(w: &World, content: &Content, tag: &str, skip: u32) -> Option<String> {
    w.furniture
        .iter()
        .filter(|p| {
            content
                .furniture_def(&p.def)
                .is_some_and(|d| d.has_tag(tag))
        })
        .nth(skip as usize)
        .map(|p| p.def.clone())
}

fn arg(k: &str, v: impl ToString) -> (String, String) {
    (k.to_string(), v.to_string())
}

/// The full-café hints (always the last line, kept when the list is cut).
pub const SEAT_HINTS: [&str; 3] = [
    "purchases.line.no_seat",
    "purchases.line.to_go",
    "purchases.line.to_go_some_left",
];

/// The payoff lines of tonight (sorted by what they stand for, ≤ `summary_max`).
pub(crate) fn lines(w: &World, content: &Content) -> Vec<PayoffLine> {
    let t = &w.pay.tonight;
    let mut out: Vec<(f32, PayoffLine)> = Vec::new();
    // new seats
    let drawn = t.drawn.round() as u32;
    if t.extra_parties > 0 || drawn > 0 {
        out.push((
            t.extra_money as f32 + drawn as f32 * 60.0,
            PayoffLine {
                key: "purchases.line.seats".into(),
                args: vec![
                    arg("guests", t.extra_guests),
                    arg("drawn", drawn),
                    arg("money", t.extra_money),
                ],
                money: t.extra_money,
            },
        ));
    }
    // comfortable (or hard) seats
    for (def, s) in &t.by_seat {
        if s.parties == 0 && s.rounds == 0 {
            continue;
        }
        let comfy = s.stay_minutes >= 0.0;
        let per = if s.parties > 0 {
            s.stay_minutes / s.parties as f32
        } else {
            0.0
        };
        let key = match (comfy, s.rounds > 0, s.tips > 0) {
            (false, _, _) => "purchases.line.hard_seat",
            (true, true, true) => "purchases.line.comfy_rounds",
            (true, true, false) => "purchases.line.comfy_rounds_stay",
            (true, false, true) => "purchases.line.comfy",
            (true, false, false) => "purchases.line.comfy_stay",
        };
        out.push((
            s.tips.abs() as f32 + s.stay_minutes.abs() * 2.0 + s.rounds as f32 * 70.0,
            PayoffLine {
                key: key.into(),
                args: vec![
                    arg("piece", format!("@furniture.{def}.name")),
                    arg("min", per.abs().round() as i32),
                    arg("money", s.tips.abs()),
                    arg("rounds", s.rounds),
                ],
                money: s.tips,
            },
        ));
    }
    // a second machine, a second stove
    for (tag, n) in &t.parallel {
        let piece = piece_with_tag(w, content, tag, 1).unwrap_or_default();
        out.push((
            *n as f32 * 40.0,
            PayoffLine {
                key: "purchases.line.parallel".into(),
                args: vec![
                    arg("piece", format!("@furniture.{piece}.name")),
                    arg("n", n),
                ],
                money: 0,
            },
        ));
    }
    for (tag, (n, money)) in &t.made {
        let piece = piece_with_tag(w, content, tag, 0).unwrap_or_default();
        out.push((
            *money as f32,
            PayoffLine {
                key: "purchases.line.made".into(),
                args: vec![
                    arg("piece", format!("@furniture.{piece}.name")),
                    arg("n", n),
                    arg("money", money),
                ],
                money: *money,
            },
        ));
    }
    // the tier the hall held most of the night
    let (li, _) = t
        .tier_minutes
        .iter()
        .enumerate()
        .max_by_key(|(i, m)| (**m, *i))
        .unwrap_or((0, &0));
    let level = li as u32 + 1;
    let td = tier_of(level);
    if t.tier_tips > 0 || t.rare_guests > 0 {
        let key = if t.rare_guests > 0 {
            "purchases.line.tier_rare"
        } else {
            "purchases.line.tier"
        };
        out.push((
            t.tier_tips as f32 + t.rare_guests as f32 * 80.0,
            PayoffLine {
                key: key.into(),
                args: vec![
                    arg("level", format!("@atmo.level.{level}")),
                    arg("pct", (td.tips * 100.0).round() as i32),
                    arg("money", t.tier_tips),
                    arg("n", t.rare_guests),
                ],
                money: t.tier_tips,
            },
        ));
    }
    // a soft hint, always last: they took it to go (and maybe a few still went on)
    // (a hint, not a purchase's payoff: `money` stays 0, the cups' Kč are only in the text)
    let hint = match (t.to_go, t.turned_away) {
        (0, 0) => None,
        (0, n) => Some(("purchases.line.no_seat", vec![arg("n", n)])),
        (g, 0) => Some((
            "purchases.line.to_go",
            vec![arg("n", g), arg("money", t.to_go_money)],
        )),
        (g, n) => Some((
            "purchases.line.to_go_some_left",
            vec![arg("n", g), arg("left", n), arg("money", t.to_go_money)],
        )),
    };
    if let Some((key, args)) = hint {
        out.push((
            -1.0,
            PayoffLine {
                key: key.into(),
                args,
                money: 0,
            },
        ));
    }
    out.sort_by(|a, b| b.0.total_cmp(&a.0));
    let max = data().summary_max;
    let mut lines: Vec<PayoffLine> = out.into_iter().map(|x| x.1).collect();
    if lines.len() > max {
        // keep the full-café hint if it was there
        let hint = lines
            .iter()
            .position(|l| SEAT_HINTS.contains(&l.key.as_str()))
            .map(|i| lines.remove(i));
        lines.truncate(max.saturating_sub(usize::from(hint.is_some())));
        lines.extend(hint);
    }
    lines
}

// ---------------------------------------------------------------------------------------------
// Queries for the screens
// ---------------------------------------------------------------------------------------------

/// How a seated guest's seat feels (the hover card).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeatFeel {
    Comfy,
    Hard,
}

/// What a piece would do if bought now (the build card's hover numbers).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PieceEffect {
    /// Hall seats now / with the piece, and about how many more guests a night.
    pub seats: Option<(u32, u32, i32)>,
    /// The seat's comfort point (stay ×, tips ×, second-round chance) when not neutral.
    pub comfort: Option<ComfortPoint>,
    /// The piece's appliance role and whether it would act now (beyond grandma's pieces).
    pub role: Option<(ApplianceRole, bool)>,
    /// Menu items the piece puts on the menu (an `Unlocks` tag).
    pub unlocks: Vec<String>,
}

impl World {
    /// Walk-ins × from the hall's seats (1.0 at grandma's 16).
    pub fn seat_draw(&self, content: &Content) -> f32 {
        seat_draw(self, content)
    }

    /// Hall seats right now.
    pub fn hall_seats(&self, content: &Content) -> u32 {
        hall_seats(self, content)
    }

    /// Tonight's purchases tally so far.
    pub fn payoff_tonight(&self) -> &PayoffTally {
        &self.pay.tonight
    }

    /// The tier perks of the meter's current level.
    pub fn coziness_tier(&self) -> TierDef {
        tier(self)
    }

    /// A seated guest's seat: comfy / hard (None = an ordinary chair or not seated).
    pub fn seat_feel(&self, content: &Content, guest: u32) -> Option<(SeatFeel, String)> {
        let g = self.guests.iter().find(|g| g.id == guest)?;
        let (uid, _) = g.seat?;
        let d = content.furniture_def(&self.furniture_by_uid(uid)?.def)?;
        let dd = data();
        if d.comfort >= dd.comfy_at {
            Some((SeatFeel::Comfy, d.id.clone()))
        } else if d.comfort < dd.hard_below {
            Some((SeatFeel::Hard, d.id.clone()))
        } else {
            None
        }
    }

    /// What buying `def` would do now (pure).
    pub fn piece_effect(&self, content: &Content, def: &FurnitureDef) -> PieceEffect {
        let mut e = PieceEffect::default();
        if def.seats > 0 {
            let now = hall_seats(self, content);
            let with = now + def.seats;
            // about this many guests a night (last night's walk-ins, else a usual night)
            let guests = self
                .summaries
                .last()
                .map(|s| s.guests_arrived)
                .filter(|n| *n > 0)
                .unwrap_or(45) as f32;
            let more = guests * (draw_for(with) / draw_for(now) - 1.0);
            e.seats = Some((now, with, more.round() as i32));
            let p = comfort_at(def.comfort);
            if (p.stay - 1.0).abs() > 0.01 || p.round > 0.0 {
                e.comfort = Some(p);
            }
        }
        for a in &data().appliances {
            if def.has_tag(&a.tag) {
                let acts = count_tag(self, content, &a.tag) + 1 > a.free;
                e.role = Some((a.role, acts));
                if a.role == ApplianceRole::Unlocks {
                    e.unlocks = content
                        .menu
                        .iter()
                        .filter(|m| m.station == a.tag && super::guests::item_unlocked(self, m))
                        .map(|m| m.id.clone())
                        .collect();
                }
                break;
            }
        }
        e
    }
}
