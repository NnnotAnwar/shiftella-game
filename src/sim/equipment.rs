//! Hands-on appliances (design/v0.2.md §B): the coffee machine, the stove, the oven, the
//! fridge, the register and the trash bin can be used by hand during the preparation half
//! hour and the night. Every chore is one click (or one hold), can't be failed, shows in the
//! café and makes a sound (UI). Lamps simply toggle (`Command::SetFurnitureOn`).
//!
//! Karolina does a chore as a queued act (`PlayerAct::Use { uid, action }`, sugar
//! `Command::UseEquipment`): she walks to the piece, works a moment (holding the mouse / **E**
//! on a hold chore speeds it up, `Command::EquipmentHold`) and the effect applies. Helpers
//! with the kitchen duty put the soup on, take the milk out and bake when she didn't (at
//! 22:00, with smaller effects); the bar / register chores stay checklist items they do as
//! before. Effects are deliberately small (tuning: `handson.ron` → `equipment`): a player who
//! never touches an appliance still finishes the story.

use serde::{Deserialize, Serialize};

use super::grid::TileKind;
use super::player::karolina_idx;
use super::types::*;
use super::world::*;
use crate::content::Content;
use crate::content::handson::Duty;

/// What a click on an appliance does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EquipAction {
    /// Coffee machine, opening (hold): it hisses, its lamp lights — `coffee_warmup` by hand.
    WarmUp,
    /// Coffee machine, night: the next few plain coffees are made almost at once.
    PullTray,
    /// Any worn appliance (hold): purge the machine / defrost the fridge / scrub the stove or
    /// the oven — the wear resets, a slow-down it caused ends (no repair cost).
    Service,
    /// Coffee machine, closing (hold): clean the group head — `clean_machine` by hand.
    CleanUp,
    /// Stove: a pot of soup goes on (soups quicker tonight; it bubbles now and then).
    SoupOn,
    /// Stove: stir the bubbling pot (soups much quicker for a while).
    Stir,
    /// Oven: grandma's medovník goes in (a timer dings).
    Bake,
    /// Oven: take the cake out after the ding (slices without waiting tonight).
    TakeOut,
    /// Fridge: take the milk out (the first lattes are quicker) + the stock check.
    Milk,
    /// Register, closing: count the till (`cash_up` by hand, tonight's numbers).
    CountTill,
    /// Trash bin, closing: take the trash out (a step outside, sometimes the cat).
    Trash,
}

impl EquipAction {
    pub const ALL: [EquipAction; 11] = [
        EquipAction::WarmUp,
        EquipAction::PullTray,
        EquipAction::Service,
        EquipAction::CleanUp,
        EquipAction::SoupOn,
        EquipAction::Stir,
        EquipAction::Bake,
        EquipAction::TakeOut,
        EquipAction::Milk,
        EquipAction::CountTill,
        EquipAction::Trash,
    ];

    /// i18n id (`equip.act.<id>`).
    pub fn id(self) -> &'static str {
        match self {
            EquipAction::WarmUp => "warm_up",
            EquipAction::PullTray => "pull_tray",
            EquipAction::Service => "service",
            EquipAction::CleanUp => "clean_up",
            EquipAction::SoupOn => "soup_on",
            EquipAction::Stir => "stir",
            EquipAction::Bake => "bake",
            EquipAction::TakeOut => "take_out",
            EquipAction::Milk => "milk",
            EquipAction::CountTill => "count_till",
            EquipAction::Trash => "trash",
        }
    }

    /// A hold chore (the progress ring fills faster while the mouse / **E** is held).
    pub fn hold(self) -> bool {
        matches!(
            self,
            EquipAction::WarmUp | EquipAction::Service | EquipAction::CleanUp
        )
    }

    /// The checklist item this chore ticks (if still open).
    pub fn checklist(self) -> Option<&'static str> {
        match self {
            EquipAction::WarmUp => Some("coffee_warmup"),
            EquipAction::CleanUp => Some("clean_machine"),
            EquipAction::Milk => Some("check_stock"),
            EquipAction::CountTill => Some("cash_up"),
            _ => None,
        }
    }
}

/// Which appliance a piece is (from its tags).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EquipKind {
    Machine,
    Stove,
    Oven,
    Fridge,
    Register,
    Trash,
    /// A lamp (not the jukebox): a click toggles it.
    Lamp,
}

impl EquipKind {
    pub fn of(def: &crate::content::FurnitureDef) -> Option<EquipKind> {
        if def.has_tag("station:coffee") {
            Some(EquipKind::Machine)
        } else if def.has_tag("station:stove") {
            Some(EquipKind::Stove)
        } else if def.has_tag("station:oven") {
            Some(EquipKind::Oven)
        } else if def.has_tag("station:fridge") {
            Some(EquipKind::Fridge)
        } else if def.has_tag("register") {
            Some(EquipKind::Register)
        } else if def.has_tag("trash") {
            Some(EquipKind::Trash)
        } else if def.has_tag("light") && !def.has_tag("music") {
            Some(EquipKind::Lamp)
        } else {
            None
        }
    }

    /// The chores of this kind, the most useful first (what a click picks).
    pub fn actions(self) -> &'static [EquipAction] {
        use EquipAction::*;
        match self {
            EquipKind::Machine => &[Service, WarmUp, CleanUp, PullTray],
            EquipKind::Stove => &[Stir, Service, SoupOn],
            EquipKind::Oven => &[TakeOut, Service, Bake],
            EquipKind::Fridge => &[Service, Milk],
            EquipKind::Register => &[CountTill],
            EquipKind::Trash => &[Trash],
            EquipKind::Lamp => &[],
        }
    }
}

/// A small call for attention (sound + glint).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EquipCue {
    /// The soup pot bubbles: a stir would help now.
    Bubbling,
    /// The oven timer: the cake is ready to come out.
    Ding,
    /// A worn appliance sputters: purge / service it.
    Sputter,
}

/// The soup pot on the stove tonight.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Pot {
    /// Karolina put it on herself (else a kitchen helper did).
    pub by_hand: bool,
    /// Minutes since opening (negative in the preparation).
    pub since: i32,
    pub next_bubble: i32,
    pub bubbling_until: Option<i32>,
    pub stirred_until: Option<i32>,
}

/// Grandma's medovník in the oven.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum Cake {
    #[default]
    None,
    Baking {
        since: i32,
        ready_at: i32,
        by_hand: bool,
    },
    Ready {
        since: i32,
    },
    Out {
        slices: u32,
    },
}

/// Tonight's appliance state (`HandsState::equip`; reset when a night starts).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EquipState {
    /// Karolina warmed the machine up by hand tonight (its lamp is lit).
    pub warm: bool,
    /// Plain coffees left on the tray pulled ahead, and when they go cold (minutes since open).
    pub tray: u32,
    pub tray_until: i32,
    pub pot: Option<Pot>,
    pub cake: Cake,
    /// Milky drinks left that are quicker (the milk is out).
    pub milk_left: u32,
    pub milk_done: bool,
    pub till_counted: bool,
    pub trash_out: bool,
    /// Chores done by hand tonight (the bot report, the digest later).
    pub by_hand: u32,
}

/// What the renderer needs for one appliance (`World::equipment_views`).
#[derive(Debug, Clone, PartialEq)]
pub struct EquipView {
    pub uid: u32,
    pub kind: EquipKind,
    /// The machine was warmed up by hand (its little lamp glows).
    pub warm: bool,
    /// Karolina is working at it right now (0–1 progress of the chore).
    pub working: Option<(EquipAction, f32)>,
    /// Wear at or above the service threshold: it sputters.
    pub worn: bool,
    pub tray: u32,
    pub pot: PotView,
    pub cake: CakeView,
    pub milk_out: bool,
    pub counted: bool,
    /// The full bag peeks out of the bin (closing, not taken out yet).
    pub bag: bool,
    /// What a click does now.
    pub action: Option<EquipAction>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PotView {
    None,
    Warm,
    Bubbling,
    Stirred,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CakeView {
    None,
    /// 0–1 of the baking time.
    Baking(f32),
    Ready,
    Out(u32),
}

fn cfg(content: &Content) -> &crate::content::handson::EquipmentTuning {
    &content.handson.tuning.equipment
}

fn now(w: &World) -> i32 {
    w.minutes_since_open()
}

fn kind_of_uid(w: &World, content: &Content, uid: u32) -> Option<EquipKind> {
    let p = w.furniture_by_uid(uid)?;
    EquipKind::of(content.furniture_def(&p.def)?)
}

fn checklist_open(w: &World, id: &str, me: u32) -> bool {
    w.app
        .checklist
        .iter()
        .any(|c| c.id == id && !c.done && c.claim.is_none_or(|x| x == me))
}

fn any_on_menu(w: &World, content: &Content, items: &[String]) -> bool {
    let menu = w.menu_available(content);
    items.iter().any(|i| menu.contains(i))
}

fn in_hands(w: &World, uid: u32, action: EquipAction) -> bool {
    let p = &w.hands.player;
    let act = super::hands::PlayerAct::Use { uid, action };
    p.queue.iter().any(|a| *a == act) || p.current.as_ref().is_some_and(|c| c.act == act)
}

/// Karolina is behind the counter (night, on duty).
fn karolina_here(w: &World) -> Option<u32> {
    if w.phase != Phase::Night {
        return None;
    }
    let i = karolina_idx(w)?;
    w.staff[i].on_duty().then(|| w.staff[i].id)
}

/// Can she do `action` at `uid` now? (Ignores whether it is already queued.)
pub(crate) fn possible(w: &World, content: &Content, uid: u32, action: EquipAction) -> bool {
    let Some(me) = karolina_here(w) else {
        return false;
    };
    let Some(p) = w.furniture_by_uid(uid) else {
        return false;
    };
    let Some(def) = content.furniture_def(&p.def) else {
        return false;
    };
    let Some(kind) = EquipKind::of(def) else {
        return false;
    };
    if !kind.actions().contains(&action) {
        return false;
    }
    let c = cfg(content);
    let e = &w.hands.equip;
    let m = w.minute();
    let closing = m >= c.closing.from_minute;
    let open = w.doors_open();
    let power = w.blackout_until.is_none();
    use EquipAction::*;
    match action {
        Service => def.wear_per_night > 0.0 && p.wear >= c.service_wear,
        WarmUp => checklist_open(w, "coffee_warmup", me),
        CleanUp => closing && checklist_open(w, "clean_machine", me),
        PullTray => {
            open && !closing
                && power
                && (e.tray == 0 || now(w) >= e.tray_until)
                && any_on_menu(w, content, &c.tray.items)
        }
        SoupOn => {
            e.pot.is_none()
                && m <= c.soup.until_minute
                && power
                && any_on_menu(w, content, &c.soup.items)
        }
        Stir => e
            .pot
            .as_ref()
            .is_some_and(|p| p.bubbling_until.is_some_and(|u| now(w) < u)),
        Bake => {
            e.cake == Cake::None
                && m <= c.cake.until_minute
                && power
                && any_on_menu(w, content, &c.cake.items)
        }
        TakeOut => matches!(e.cake, Cake::Ready { .. }),
        Milk => {
            (!e.milk_done && m <= c.milk.until_minute)
                || checklist_open(w, "check_stock", me) && !closing
        }
        CountTill => closing && !e.till_counted,
        Trash => closing && !e.trash_out,
    }
}

/// What a click on this piece does now (`None` = nothing; lamps are toggled by the UI).
pub(crate) fn action_for(w: &World, content: &Content, uid: u32) -> Option<EquipAction> {
    let kind = kind_of_uid(w, content, uid)?;
    kind.actions()
        .iter()
        .copied()
        .find(|a| possible(w, content, uid, *a) && !in_hands(w, uid, *a))
}

/// Validates and claims a queued chore (`PlayerAct::Use`).
pub(crate) fn claim(
    w: &mut World,
    content: &Content,
    uid: u32,
    action: EquipAction,
) -> Result<(), CommandError> {
    if w.furniture_by_uid(uid).is_none() {
        return Err(CommandError::UnknownId(uid.to_string()));
    }
    if in_hands(w, uid, action) {
        return Err(CommandError::Invalid("already on it".into()));
    }
    if !possible(w, content, uid, action) {
        return Err(CommandError::Invalid(format!(
            "{} isn't possible here now",
            action.id()
        )));
    }
    // a checklist chore: helpers leave the item alone while it's in her hands
    let me = karolina_here(w).unwrap_or(0);
    if let Some(id) = action.checklist()
        && let Some(c) = w
            .app
            .checklist
            .iter_mut()
            .find(|c| c.id == id && !c.done && c.claim.is_none())
    {
        c.claim = Some(me);
    }
    Ok(())
}

/// Releases a dropped chore's checklist claim.
pub(crate) fn release(w: &mut World, action: EquipAction) {
    let me = karolina_idx(w).map(|i| w.staff[i].id);
    if let Some(id) = action.checklist()
        && let Some(c) = w.app.checklist.iter_mut().find(|c| c.id == id)
        && c.claim.is_some()
        && c.claim == me
    {
        c.claim = None;
    }
}

/// Where she stands for the chore (the bin's own spot first for the trash).
pub(crate) fn goal(w: &World, content: &Content, uid: u32) -> Option<(i32, i32)> {
    super::staff::station_access(w, content, uid)
}

/// Just outside the door (the trash run's second stop).
pub(crate) fn outside_spot(w: &World) -> Option<(i32, i32)> {
    let doors = w.grid.door_tiles();
    for (x, y) in doors {
        for (dx, dy) in [(0, 1), (1, 1), (-1, 1), (0, 2)] {
            let t = (x + dx, y + dy);
            if w.grid.walkable(t.0, t.1)
                && w.grid
                    .tile(t.0, t.1)
                    .is_some_and(|i| i.kind == TileKind::Street)
            {
                return Some(t);
            }
        }
    }
    None
}

/// Work ticks of the chore at the piece (the trash run: picking the bag up).
pub(crate) fn work_ticks(content: &Content, action: EquipAction) -> f32 {
    let w = &cfg(content).work;
    use EquipAction::*;
    match action {
        WarmUp => w.warm_up,
        PullTray => w.pull_tray,
        Service => w.service,
        CleanUp => w.clean_up,
        SoupOn => w.soup_on,
        Stir => w.stir,
        Bake => w.bake,
        TakeOut => w.take_out,
        Milk => w.milk,
        CountTill => w.count_till,
        Trash => w.take_out,
    }
    .max(1.0)
}

/// Work ticks outside the door (the trash run).
pub(crate) fn trash_ticks(content: &Content) -> f32 {
    cfg(content).work.trash.max(1.0)
}

/// `Command::EquipmentHold`: she is doing a hold chore at its piece.
pub(crate) fn hold(w: &mut World, content: &Content, seconds: f32) -> Result<(), CommandError> {
    if !(seconds.is_finite() && seconds > 0.0) {
        return Err(CommandError::Invalid("seconds".into()));
    }
    let rate = cfg(content).hold_ticks_per_second;
    let Some(c) = w.hands.player.current.as_mut() else {
        return Err(CommandError::Invalid("nothing in her hands".into()));
    };
    match c.act {
        super::hands::PlayerAct::Use { action, .. } if action.hold() && c.stage == 1 => {
            c.work_left = (c.work_left - seconds.min(5.0) * rate).max(0.0);
            Ok(())
        }
        _ => Err(CommandError::Invalid("not at a hold chore".into())),
    }
}

/// The chore is done (`by` = who did it; Karolina by hand gets the full effect).
pub(crate) fn complete(
    w: &mut World,
    content: &Content,
    uid: u32,
    action: EquipAction,
    by: Option<u32>,
) {
    let c = cfg(content).clone();
    let t = now(w);
    let by_hand = by.is_some_and(|id| w.staff_by_id(id).is_some_and(|s| s.owner));
    if let Some(id) = action.checklist()
        && w.app.checklist.iter().any(|x| x.id == id && !x.done)
    {
        super::app::tick_checklist(w, content, id, by);
    }
    use EquipAction::*;
    match action {
        WarmUp => w.hands.equip.warm = true,
        PullTray => {
            w.hands.equip.tray = c.tray.count;
            w.hands.equip.tray_until = t + c.tray.minutes as i32;
        }
        Service => service(w, content, uid),
        CleanUp => {}
        SoupOn => {
            w.hands.equip.pot = Some(Pot {
                by_hand,
                since: t,
                next_bubble: t.max(0) + (c.soup.bubble_every as i32 * 3 / 5).max(1),
                bubbling_until: None,
                stirred_until: None,
            });
        }
        Stir => {
            if let Some(p) = w.hands.equip.pot.as_mut() {
                p.bubbling_until = None;
                p.stirred_until = Some(t + c.soup.stir_minutes as i32);
            }
        }
        Bake => {
            w.hands.equip.cake = Cake::Baking {
                since: t,
                ready_at: t + c.cake.bake_minutes as i32,
                by_hand,
            };
        }
        TakeOut => {
            let slices = if by_hand {
                c.cake.slices
            } else {
                c.cake.helper_slices
            };
            w.hands.equip.cake = Cake::Out { slices };
        }
        Milk => {
            if !w.hands.equip.milk_done {
                w.hands.equip.milk_done = true;
                w.hands.equip.milk_left = if by_hand {
                    c.milk.count
                } else {
                    c.milk.helper_count
                };
            }
        }
        CountTill => {
            w.hands.equip.till_counted = true;
            let s = &w.tonight;
            let ev = SimEvent::TillCounted {
                revenue: s.revenue,
                guests: s.guests_served,
                tips: s.tips,
            };
            w.emit(ev);
        }
        Trash => {
            w.hands.equip.trash_out = true;
            w.dirt = (w.dirt - c.closing.trash_dirt).max(0.0);
            let cat = w.rng.chance(c.closing.cat_chance);
            if cat {
                let counter = content.handson.extras.cat.counter.clone();
                *w.story.counters.entry(counter).or_insert(0) += 1;
                super::moments::check_cat(w, content);
            }
            w.emit(SimEvent::TrashOut { cat });
        }
    }
    if by_hand {
        w.hands.equip.by_hand += 1;
    }
    w.emit(SimEvent::EquipmentUsed { uid, action, by });
}

/// Purge / service: the wear resets and what the wear caused tonight is over (the event's
/// slow-down ends, its question goes away) — for free.
fn service(w: &mut World, content: &Content, uid: u32) {
    let tag = w
        .furniture_by_uid(uid)
        .and_then(|p| content.furniture_def(&p.def))
        .and_then(|d| d.tags.iter().find(|t| t.starts_with("station:")).cloned());
    if let Some(p) = w.furniture.iter_mut().find(|p| p.uid == uid) {
        p.wear = 0.0;
    }
    let Some(tag) = tag else {
        return;
    };
    let events: Vec<String> = w
        .mods
        .iter()
        .filter(
            |m| matches!(&m.kind, ModKind::StationSlow(t) | ModKind::StationDown(t) if *t == tag),
        )
        .filter_map(|m| m.event.clone())
        .collect();
    w.mods.retain(
        |m| !matches!(&m.kind, ModKind::StationSlow(t) | ModKind::StationDown(t) if *t == tag),
    );
    for e in events {
        if let Some(i) = w.narrator.pending.iter().position(|p| p.event == e) {
            w.narrator.pending.remove(i);
            w.emit(SimEvent::ChoiceResolved {
                id: e,
                choice: "purged".into(),
            });
        }
    }
}

/// Prep × of a new order item (plain or Karolina's brew): the tray, the milk, the pot, the
/// cake. Consumes the tray / milk / slices it uses.
pub(crate) fn item_prep_mul(w: &mut World, content: &Content, menu: &str, plain: bool) -> f32 {
    let c = cfg(content);
    let t = now(w);
    let has = |v: &[String]| v.iter().any(|i| i == menu);
    let e = &mut w.hands.equip;
    if plain && e.tray > 0 && t < e.tray_until && has(&c.tray.items) {
        e.tray -= 1;
        return c.tray.prep_mul;
    }
    let mut mul = 1.0;
    if e.milk_left > 0 && has(&c.milk.items) {
        e.milk_left -= 1;
        mul *= c.milk.prep_mul;
    }
    if has(&c.soup.items)
        && let Some(p) = &e.pot
    {
        mul *= if p.stirred_until.is_some_and(|u| t < u) {
            c.soup.stir_mul
        } else if p.by_hand {
            c.soup.warm_mul
        } else {
            c.soup.helper_warm_mul
        };
    }
    if has(&c.cake.items)
        && let Cake::Out { slices } = &mut e.cake
        && *slices > 0
    {
        *slices -= 1;
        mul *= c.cake.prep_mul;
    }
    mul
}

/// A helper on shift with this duty (not Karolina).
fn helper_with(w: &World, d: Duty) -> Option<u32> {
    w.staff
        .iter()
        .find(|s| !s.owner && s.on_duty() && s.duties.contains(&d))
        .map(|s| s.id)
}

fn first_with(w: &World, content: &Content, kind: EquipKind) -> Option<u32> {
    w.furniture
        .iter()
        .find(|p| {
            content
                .furniture_def(&p.def)
                .and_then(EquipKind::of)
                .is_some_and(|k| k == kind)
        })
        .map(|p| p.uid)
}

/// Night start: a fresh evening for the appliances.
pub(crate) fn on_night_start(w: &mut World) {
    w.hands.equip = EquipState::default();
}

/// Every minute of the preparation and the night: the oven dings, the pot bubbles, the
/// tray goes cold, a worn machine sputters; at 22:00 the kitchen helper does what Karolina
/// didn't.
pub(crate) fn on_minute(w: &mut World, content: &Content) {
    if w.phase != Phase::Night {
        return;
    }
    let c = cfg(content).clone();
    let t = now(w);
    let m = w.minute();
    // 22:00: delegated kitchen chores
    if t == 0
        && let Some(helper) = helper_with(w, Duty::Kitchen)
    {
        let power = w.blackout_until.is_none();
        if w.hands.equip.pot.is_none()
            && power
            && any_on_menu(w, content, &c.soup.items)
            && let Some(stove) = first_with(w, content, EquipKind::Stove)
        {
            complete(w, content, stove, EquipAction::SoupOn, Some(helper));
        }
        if !w.hands.equip.milk_done
            && let Some(fridge) = first_with(w, content, EquipKind::Fridge)
        {
            complete(w, content, fridge, EquipAction::Milk, Some(helper));
        }
        if w.hands.equip.cake == Cake::None
            && power
            && any_on_menu(w, content, &c.cake.items)
            && let Some(oven) = first_with(w, content, EquipKind::Oven)
        {
            complete(w, content, oven, EquipAction::Bake, Some(helper));
        }
    }
    // the oven timer
    if let Cake::Baking { ready_at, .. } = w.hands.equip.cake
        && t >= ready_at
    {
        w.hands.equip.cake = Cake::Ready { since: t };
        if let Some(oven) = first_with(w, content, EquipKind::Oven) {
            w.emit(SimEvent::EquipmentCue {
                uid: oven,
                cue: EquipCue::Ding,
            });
        }
    }
    // nobody takes the cake out: the kitchen helper does after a while (fewer slices)
    if let Cake::Ready { since } = w.hands.equip.cake
        && t >= since + c.cake.helper_after as i32
        && let Some(helper) = helper_with(w, Duty::Kitchen)
        && let Some(oven) = first_with(w, content, EquipKind::Oven)
        && !in_hands(w, oven, EquipAction::TakeOut)
    {
        complete(w, content, oven, EquipAction::TakeOut, Some(helper));
    }
    // the pot bubbles now and then (doors open, before closing)
    if t >= 0
        && m < c.closing.from_minute
        && let Some(stove) = first_with(w, content, EquipKind::Stove)
        && let Some(p) = w.hands.equip.pot.as_mut()
    {
        if p.bubbling_until.is_some_and(|u| t >= u) {
            // untouched: it simply keeps warm
            p.bubbling_until = None;
        }
        if p.bubbling_until.is_none() && t >= p.next_bubble {
            p.bubbling_until = Some(t + c.soup.bubble_minutes as i32);
            p.next_bubble = t + c.soup.bubble_every.max(1) as i32;
            w.emit(SimEvent::EquipmentCue {
                uid: stove,
                cue: EquipCue::Bubbling,
            });
        }
    }
    // the tray goes cold
    if w.hands.equip.tray > 0 && t >= w.hands.equip.tray_until {
        w.hands.equip.tray = 0;
    }
    // a worn appliance sputters now and then (doors open)
    if t > 0 && c.sputter_every > 0 && (t as u32).is_multiple_of(c.sputter_every) {
        let worn: Vec<u32> = w
            .furniture
            .iter()
            .filter(|p| {
                content
                    .furniture_def(&p.def)
                    .is_some_and(|d| d.wear_per_night > 0.0 && EquipKind::of(d).is_some())
                    && p.wear >= c.service_wear
            })
            .map(|p| p.uid)
            .collect();
        if let Some(uid) = worn.first() {
            w.emit(SimEvent::EquipmentCue {
                uid: *uid,
                cue: EquipCue::Sputter,
            });
        }
    }
}

impl World {
    /// Tonight's appliance state.
    pub fn equipment(&self) -> &EquipState {
        &self.hands.equip
    }

    /// The appliance kind of a placed piece (lamps included).
    pub fn equipment_kind(&self, content: &Content, uid: u32) -> Option<EquipKind> {
        kind_of_uid(self, content, uid)
    }

    /// What a click on this appliance does now (`None` = nothing to do there; lamps are
    /// toggled with `Command::SetFurnitureOn`).
    pub fn equipment_action(&self, content: &Content, uid: u32) -> Option<EquipAction> {
        action_for(self, content, uid)
    }

    /// Render state of every appliance (warm lamp, pot, cake, tray, milk, bag, wear).
    pub fn equipment_views(&self, content: &Content) -> Vec<EquipView> {
        let c = cfg(content);
        let e = &self.hands.equip;
        let t = now(self);
        let night = self.phase == Phase::Night;
        let cur = self.hands.player.current.as_ref();
        let mut out = Vec::new();
        for p in &self.furniture {
            let Some(def) = content.furniture_def(&p.def) else {
                continue;
            };
            let Some(kind) = EquipKind::of(def) else {
                continue;
            };
            if kind == EquipKind::Lamp {
                continue;
            }
            let working = cur.and_then(|a| match a.act {
                super::hands::PlayerAct::Use { uid, action } if uid == p.uid && a.stage >= 1 => {
                    Some((action, a.progress()))
                }
                _ => None,
            });
            let pot = match (&e.pot, kind) {
                (Some(pt), EquipKind::Stove) if night => {
                    if pt.bubbling_until.is_some_and(|u| t < u) {
                        PotView::Bubbling
                    } else if pt.stirred_until.is_some_and(|u| t < u) {
                        PotView::Stirred
                    } else {
                        PotView::Warm
                    }
                }
                _ => PotView::None,
            };
            let cake = match (e.cake, kind) {
                (
                    Cake::Baking {
                        since, ready_at, ..
                    },
                    EquipKind::Oven,
                ) if night => {
                    let span = (ready_at - since).max(1) as f32;
                    CakeView::Baking(((t - since) as f32 / span).clamp(0.0, 1.0))
                }
                (Cake::Ready { .. }, EquipKind::Oven) if night => CakeView::Ready,
                (Cake::Out { slices }, EquipKind::Oven) if night => CakeView::Out(slices),
                _ => CakeView::None,
            };
            let closing = night && self.minute() >= c.closing.from_minute;
            out.push(EquipView {
                uid: p.uid,
                kind,
                warm: night && kind == EquipKind::Machine && e.warm,
                working,
                worn: def.wear_per_night > 0.0 && p.wear >= c.service_wear,
                tray: if night && kind == EquipKind::Machine && t < e.tray_until {
                    e.tray
                } else {
                    0
                },
                pot,
                cake,
                milk_out: night && kind == EquipKind::Fridge && e.milk_left > 0,
                counted: night && kind == EquipKind::Register && e.till_counted,
                bag: closing && kind == EquipKind::Trash && !e.trash_out,
                action: action_for(self, content, p.uid),
            });
        }
        out
    }

    /// Screenshots / QA: sets the wear of every piece with the tag (a worn machine).
    #[doc(hidden)]
    pub fn debug_set_wear(&mut self, content: &Content, tag: &str, wear: f32) {
        for p in self.furniture.iter_mut() {
            if content
                .furniture_def(&p.def)
                .is_some_and(|d| d.has_tag(tag))
            {
                p.wear = wear.clamp(0.0, 1.0);
            }
        }
    }
}
