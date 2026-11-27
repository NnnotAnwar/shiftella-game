//! Karolina behind the counter (design/handson.md §1.1–§1.2): the player actor. The sim never
//! assigns her work — she only does what the player queues (FIFO, at most 8; reorderable): take orders,
//! brew (quick or by hand), serve, clear, wash (hold), mop (swipe), collect tips, tick
//! checklist items, put records on. She walks there herself (×1.4 staff speed) and works at a
//! skill-independent speed.

use super::grid;
use super::guests;
use super::hands::*;
use super::orders;
use super::staff::{self, STAFF_SPEED, WORK_SPEED};
use super::types::*;
use super::world::*;
use crate::content::Content;
use crate::content::handson::match_score;

/// Work ticks (at speed 1) of her quick acts.
const TAKE_ORDER_WORK: f32 = 8.0;
const HAND_OVER_WORK: f32 = 3.0;
const CLEAR_WORK: f32 = 3.0;
const MOP_WORK: f32 = 12.0;
const TIP_WORK: f32 = 3.0;
const CHECKLIST_WORK: f32 = 6.0;
const NEEDLE_WORK: f32 = 7.0;
/// Hall dirt one mop swipe takes away.
pub const MOP_AMOUNT: f32 = 0.1;
/// Dishes washed per real second of holding.
pub const WASH_PER_SECOND: f32 = 2.0;
/// She gives up waiting at the sink without a hold after this many ticks (sooner when more
/// acts are queued), and at a busy station after `STATION_PATIENCE`.
const SINK_PATIENCE: u32 = 60;
const STATION_PATIENCE: u32 = 600;

pub(crate) fn karolina_idx(w: &World) -> Option<usize> {
    w.staff.iter().position(|s| s.owner)
}

fn me(w: &World) -> u32 {
    karolina_idx(w).map(|i| w.staff[i].id).unwrap_or(0)
}

/// Guest id (or party id) → party index.
pub(crate) fn party_of(w: &World, guest: u32) -> Option<usize> {
    if let Some(g) = w.guests.iter().find(|g| g.id == guest) {
        return w.parties.iter().position(|p| p.id == g.party);
    }
    w.parties.iter().position(|p| p.id == guest)
}

fn night_only(w: &World) -> Result<(), CommandError> {
    if w.phase != Phase::Night {
        return Err(CommandError::WrongPhase);
    }
    if karolina_idx(w).is_none_or(|i| !w.staff[i].on_duty()) {
        return Err(CommandError::Invalid(
            "Karolina isn't behind the counter".into(),
        ));
    }
    Ok(())
}

fn queue_room(w: &World) -> Result<(), CommandError> {
    if w.hands.player.queue.len() >= PLAYER_QUEUE_MAX {
        return Err(CommandError::QueueFull);
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Claims
// ---------------------------------------------------------------------------------------------

/// Validates an act and claims what it works on (so helpers leave it alone). Returns the act
/// as queued (a serve gets the party's other ready items on its tray).
fn claim_act(w: &mut World, content: &Content, act: PlayerAct) -> Result<PlayerAct, CommandError> {
    let me = me(w);
    match &act {
        PlayerAct::TakeOrder { party } => {
            let p = w
                .parties
                .iter_mut()
                .find(|p| p.id == *party)
                .ok_or_else(|| CommandError::UnknownId(party.to_string()))?;
            if !matches!(p.state, PartyState::Queue | PartyState::WaitOrder) {
                return Err(CommandError::Invalid("they already ordered".into()));
            }
            match p.order_claim {
                Some(x) if x == me => return Err(CommandError::Invalid("already on it".into())),
                Some(_) => return Err(CommandError::InUse),
                None => {}
            }
            p.order_claim = Some(me);
        }
        PlayerAct::Brew { .. } => {
            return Err(CommandError::Invalid("use Command::PlayerBrew".into()));
        }
        PlayerAct::Serve { item, .. } => {
            let i = w
                .items
                .iter_mut()
                .find(|i| i.id == *item)
                .ok_or_else(|| CommandError::UnknownId(item.to_string()))?;
            if i.state != ItemState::Ready {
                return Err(CommandError::Invalid("not ready".into()));
            }
            if i.by.is_some() {
                return Err(CommandError::InUse);
            }
            i.by = Some(me);
            let (party, at) = (i.party, i.ready_tile);
            let mut extra = Vec::new();
            for o in w.items.iter_mut() {
                if extra.len() + 1 >= staff::TRAY {
                    break;
                }
                if o.id != *item
                    && o.party == party
                    && o.state == ItemState::Ready
                    && o.by.is_none()
                    && o.ready_tile
                        .zip(at)
                        .is_some_and(|(a, b)| grid::chebyshev(a, b) <= 3)
                {
                    o.by = Some(me);
                    extra.push(o.id);
                }
            }
            return Ok(PlayerAct::Serve { item: *item, extra });
        }
        PlayerAct::ClearTable { table } => {
            let f = w
                .furniture
                .iter_mut()
                .find(|f| f.uid == *table)
                .ok_or_else(|| CommandError::UnknownId(table.to_string()))?;
            if f.dishes == 0 && !w.hands.tips.iter().any(|t| t.table == *table) {
                return Err(CommandError::Invalid("nothing on that table".into()));
            }
            if f.claim.is_some_and(|c| c != me) {
                return Err(CommandError::InUse);
            }
            f.claim = Some(me);
        }
        PlayerAct::Wash { sink } => {
            if w.furniture_by_uid(*sink).is_none() {
                return Err(CommandError::UnknownId(sink.to_string()));
            }
            if w.sink_dishes == 0 {
                return Err(CommandError::Invalid("the sink is empty".into()));
            }
            if w.sink_claim.is_some_and(|c| c != me) {
                return Err(CommandError::InUse);
            }
            w.sink_claim = Some(me);
        }
        PlayerAct::Mop { tile } => {
            if !w.grid.walkable(tile.0, tile.1) || w.grid.room(tile.0, tile.1).staff_only() {
                return Err(CommandError::OutOfBounds);
            }
            if w.dirt <= 0.0 {
                return Err(CommandError::Invalid("the floor is clean".into()));
            }
        }
        PlayerAct::CollectTip { table } => {
            if !w.hands.tips.iter().any(|t| t.table == *table) {
                return Err(CommandError::Invalid("no tip on that table".into()));
            }
        }
        PlayerAct::Checklist { id } => {
            let c = w
                .app
                .checklist
                .iter_mut()
                .find(|c| &c.id == id)
                .ok_or_else(|| CommandError::UnknownId(id.clone()))?;
            if c.done {
                return Err(CommandError::Invalid("already done".into()));
            }
            if c.claim.is_some_and(|x| x != me) {
                return Err(CommandError::InUse);
            }
            c.claim = Some(me);
        }
        PlayerAct::PlayRecord { record, .. } => super::music::validate(w, record)?,
        PlayerAct::GoTo { tile } => {
            if !w.grid.walkable(tile.0, tile.1) {
                return Err(CommandError::OutOfBounds);
            }
        }
        PlayerAct::Use { uid, action } => super::equipment::claim(w, content, *uid, *action)?,
    }
    Ok(act)
}

/// Releases what an unfinished act claimed (`carrying` = she had picked it up).
fn release_act(w: &mut World, content: &Content, act: &PlayerAct, stage: u8, count: u32) {
    let me = me(w);
    let here = karolina_idx(w).map(|i| w.staff[i].tile());
    match act {
        PlayerAct::TakeOrder { party } => {
            if let Some(p) = w.parties.iter_mut().find(|p| p.id == *party)
                && p.order_claim == Some(me)
            {
                p.order_claim = None;
            }
        }
        PlayerAct::Brew { item, extra } => {
            for x in extra {
                release_item(w, *x, me, here);
            }
            let Some(ii) = w.items.iter().position(|i| i.id == *item) else {
                return;
            };
            if w.items[ii].by != Some(me) {
                return;
            }
            let it = &w.items[ii];
            if matches!(it.state, ItemState::Served | ItemState::Cancelled) {
                return;
            }
            if it.wish && it.state != ItemState::Carrying && it.state != ItemState::Ready {
                // back to the order card: the wish is open again
                let it = w.items.remove(ii);
                if let Some(mi) = content.menu_item(&it.menu) {
                    w.econ.supplies += mi.cost as i64;
                }
                if let Some(p) = w.parties.iter_mut().find(|p| p.id == it.party) {
                    p.items.retain(|x| *x != it.id);
                    p.wish_open = true;
                }
            } else {
                let i = &mut w.items[ii];
                i.by = None;
                if matches!(i.state, ItemState::Preparing) {
                    i.state = ItemState::Pending;
                }
                if matches!(i.state, ItemState::Carrying) {
                    i.state = ItemState::Ready;
                    i.ready_tile = here;
                }
                if i.state == ItemState::Pending && !i.wish {
                    i.origin = ItemOrigin::Unassigned;
                    i.price_add = 0;
                }
            }
        }
        PlayerAct::Serve { item, extra } => {
            for x in std::iter::once(item).chain(extra) {
                release_item(w, *x, me, here);
            }
        }
        PlayerAct::ClearTable { table } => {
            if let Some(f) = w.furniture.iter_mut().find(|f| f.uid == *table)
                && f.claim == Some(me)
            {
                f.claim = None;
            }
            if stage >= 2 {
                w.sink_dishes += count;
            }
        }
        PlayerAct::Wash { .. } => {
            if w.sink_claim == Some(me) {
                w.sink_claim = None;
            }
        }
        PlayerAct::Checklist { id } => {
            if let Some(c) = w.app.checklist.iter_mut().find(|c| &c.id == id)
                && c.claim == Some(me)
            {
                c.claim = None;
            }
        }
        PlayerAct::Use { action, .. } => super::equipment::release(w, *action),
        PlayerAct::GoTo { tile } => {
            // a queued moment / experiment walk was dropped
            if stage == 0
                && let Some(i) = w.sys.moments.walks.iter().position(|j| j.tile == *tile)
            {
                let j = w.sys.moments.walks.remove(i);
                if let super::systems::WalkKind::Moment { id, .. } = &j.kind {
                    super::moments::release(w, id);
                }
                super::stories::release(w, &j.kind);
            }
            if stage > 0
                && let Some(j) = w.sys.moments.walk_now.take()
            {
                if let super::systems::WalkKind::Moment { id, .. } = &j.kind {
                    super::moments::release(w, id);
                }
                super::stories::release(w, &j.kind);
            }
        }
        _ => {}
    }
}

/// Gives a claimed plain item back to the rail (progress is kept).
fn release_item(w: &mut World, item: u32, me: u32, here: Option<(i32, i32)>) {
    if let Some(i) = w.items.iter_mut().find(|i| i.id == item)
        && i.by == Some(me)
        && !matches!(i.state, ItemState::Served | ItemState::Cancelled)
    {
        i.by = None;
        match i.state {
            ItemState::Preparing => i.state = ItemState::Pending,
            ItemState::Carrying => {
                i.state = ItemState::Ready;
                i.ready_tile = here;
            }
            _ => {}
        }
        if i.state == ItemState::Pending && !i.wish {
            i.origin = ItemOrigin::Unassigned;
            i.price_add = 0;
        }
    }
}

fn push(w: &mut World, act: PlayerAct, front: bool) {
    if front {
        w.hands.player.queue.push_front(act);
    } else {
        w.hands.player.queue.push_back(act);
    }
    w.emit(SimEvent::PlayerQueueChanged);
}

// ---------------------------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------------------------

/// `Command::PlayerQueue`.
pub(crate) fn queue(
    w: &mut World,
    content: &Content,
    act: PlayerAct,
    front: bool,
) -> Result<(), CommandError> {
    if let PlayerAct::PlayRecord { record, repeat } = &act
        && w.phase == Phase::Day
    {
        super::music::validate(w, record)?;
        w.hands.music.opening = record.clone().map(|r| (r, *repeat));
        return Ok(());
    }
    night_only(w)?;
    queue_room(w)?;
    let act = claim_act(w, content, act)?;
    push(w, act, front);
    Ok(())
}

/// `Command::PlayerTakeOrder`.
pub(crate) fn take_order(w: &mut World, content: &Content, guest: u32) -> Result<(), CommandError> {
    night_only(w)?;
    let pi = party_of(w, guest).ok_or_else(|| CommandError::UnknownId(guest.to_string()))?;
    let party = w.parties[pi].id;
    queue(w, content, PlayerAct::TakeOrder { party }, false)
}

/// The quality of a brew: the mini-game's for by hand, the (practised) quick quality else.
fn brew_quality(w: &World, content: &Content, recipe: &str, mode: BrewMode, q: f32) -> f32 {
    let t = &content.handson.tuning;
    match mode {
        BrewMode::ByHand => {
            if q.is_finite() {
                q.clamp(0.0, 1.0)
            } else {
                content.handson.minigames.common.skip_quality
            }
        }
        BrewMode::Quick => {
            let base = if w
                .hands
                .knowledge
                .recipes
                .get(recipe)
                .is_some_and(|s| s.practised)
            {
                t.practised_quick_quality
            } else {
                t.quick_quality
            };
            // the warmed-up machine (opening checklist)
            let m = w.minute();
            let add: f32 = w
                .sys
                .fx
                .quick_quality
                .iter()
                .filter(|(_, u)| u.is_none_or(|u| u > m))
                .map(|(a, _)| a)
                .sum();
            (base + add).clamp(0.0, 1.0)
        }
    }
}

/// `Command::PlayerBrew`.
pub(crate) fn brew(
    w: &mut World,
    content: &Content,
    guest: u32,
    recipe: &str,
    quality: f32,
    mode: BrewMode,
) -> Result<(), CommandError> {
    night_only(w)?;
    let pi = party_of(w, guest).ok_or_else(|| CommandError::UnknownId(guest.to_string()))?;
    // a clone: the café's own drink lives in the world (`sim::signature`)
    let r = w
        .recipe_def(content, recipe)
        .cloned()
        .ok_or_else(|| CommandError::UnknownId(recipe.to_string()))?;
    let r = &r;
    if !orders::known(w, recipe) {
        return Err(CommandError::NotUnlocked);
    }
    if !orders::recipe_available(w, content, r) {
        return Err(CommandError::Invalid(
            "that can't be made tonight (menu, stock, station or power)".into(),
        ));
    }
    let mi = content
        .menu_item(&r.base_item)
        .ok_or_else(|| CommandError::UnknownId(r.base_item.clone()))?;
    queue_room(w)?;
    let me = me(w);
    let t = &content.handson.tuning;
    let q = brew_quality(w, content, recipe, mode, quality);
    let leader = w.parties[pi].leader().unwrap_or(0);
    let party_id = w.parties[pi].id;
    let for_wish = w.parties[pi].wish_open
        && !w.parties[pi].handed_off
        && w.parties[pi].state == PartyState::WaitFood
        && (guest == leader || guest == party_id);
    let share = if mode == BrewMode::ByHand {
        t.by_hand_work_share
    } else {
        1.0
    };
    let mut full = mi.prep_minutes * TICKS_PER_MINUTE as f32;
    if for_wish {
        // the pot on the stove, the cake out of the oven, the milk out (v0.2 equipment)
        full *= super::equipment::item_prep_mul(w, content, &r.base_item, false);
    }
    let iid = if for_wish {
        let wish = w.parties[pi]
            .order_kind
            .wish()
            .unwrap_or_default()
            .to_string();
        let m = content
            .handson
            .wish(&wish)
            .map(|wd| match_score(wd, r, orders::match_ctx(w, &r.id)))
            .unwrap_or(0.6);
        let iid = w.alloc_id();
        w.econ.supplies -= (mi.cost + r.cost_add) as i64;
        let now = w.minute();
        w.items.push(OrderItem {
            id: iid,
            party: party_id,
            guest: leader,
            menu: r.base_item.clone(),
            state: ItemState::Pending,
            prep_left: full * share,
            by: Some(me),
            ready_tile: None,
            quality: 0.0,
            created: now,
            origin: ItemOrigin::Player {
                recipe: recipe.to_string(),
                quality: q,
                match_score: m,
                mode,
            },
            price_add: r.price_add,
            hand_off_match: None,
            wish: true,
        });
        let p = &mut w.parties[pi];
        p.items.push(iid);
        p.wish_open = false;
        w.hands.player.last_mode_wish = mode;
        iid
    } else {
        let pick = w
            .items
            .iter()
            .filter(|i| {
                i.party == party_id
                    && i.state == ItemState::Pending
                    && i.by.is_none()
                    && i.menu == r.base_item
                    && !i.wish
            })
            .min_by_key(|i| (i.guest != guest, i.id))
            .map(|i| i.id);
        let Some(iid) = pick else {
            return Err(CommandError::Invalid(
                "nothing of that to make for them".into(),
            ));
        };
        let i = w.items.iter_mut().find(|i| i.id == iid).unwrap();
        i.by = Some(me);
        i.origin = ItemOrigin::Player {
            recipe: recipe.to_string(),
            quality: q,
            match_score: 1.0,
            mode,
        };
        i.price_add = r.price_add;
        i.prep_left = i.prep_left.min(full * share);
        w.hands.player.last_mode_plain = mode;
        iid
    };
    // a quick plain brew makes the party's whole round at that station
    let mut extra = Vec::new();
    if mode == BrewMode::Quick && !for_wish {
        let tag = mi.station.clone();
        let others: Vec<(u32, String)> = w
            .items
            .iter()
            .filter(|i| {
                i.party == party_id
                    && i.id != iid
                    && i.state == ItemState::Pending
                    && i.by.is_none()
                    && !i.wish
                    && content.menu_item(&i.menu).is_some_and(|m| m.station == tag)
            })
            .map(|i| (i.id, i.menu.clone()))
            .take(staff::TRAY - 1)
            .collect();
        for (oid, menu) in others {
            let Some(rr) = content
                .handson
                .recipes_for_item(&menu)
                .find(|r| orders::recipe_available(w, content, r))
                .map(|r| r.id.clone())
            else {
                continue;
            };
            let oq = brew_quality(w, content, &rr, BrewMode::Quick, 0.0);
            let price_add = content
                .handson
                .recipe(&rr)
                .map(|r| r.price_add)
                .unwrap_or(0);
            if let Some(i) = w.items.iter_mut().find(|i| i.id == oid) {
                i.by = Some(me);
                i.origin = ItemOrigin::Player {
                    recipe: rr,
                    quality: oq,
                    match_score: 1.0,
                    mode: BrewMode::Quick,
                };
                i.price_add = price_add;
                extra.push(oid);
            }
        }
    }
    if mode == BrewMode::ByHand {
        let (kind, _) = r.playable_minigame(false);
        let streak = &mut w.hands.player.minigame_streak;
        *streak = match *streak {
            Some((k, n)) if k == kind => Some((k, n + 1)),
            _ => Some((kind, 1)),
        };
    }
    // a regular's wish goes first: their visit windows are short (T9 balance)
    let front = for_wish && w.parties[pi].regular.is_some();
    push(w, PlayerAct::Brew { item: iid, extra }, front);
    Ok(())
}

/// `Command::PlayerUsual`.
pub(crate) fn usual(
    w: &mut World,
    content: &Content,
    guest: u32,
    quality: f32,
) -> Result<(), CommandError> {
    night_only(w)?;
    let pi = party_of(w, guest).ok_or_else(|| CommandError::UnknownId(guest.to_string()))?;
    let recipe = w.parties[pi]
        .order_kind
        .usual()
        .map(str::to_string)
        .ok_or_else(|| CommandError::Invalid("no usual yet".into()))?;
    let leader = w.parties[pi].leader().unwrap_or(guest);
    brew(w, content, leader, &recipe, quality, BrewMode::ByHand)
}

/// Turns a wish into a helper ticket (match capped at `hand_off_match_cap`).
pub(crate) fn hand_off_item(w: &mut World, content: &Content, pi: usize) -> bool {
    let Some(wish) = w.parties[pi].order_kind.wish().map(str::to_string) else {
        return false;
    };
    let Some((recipe, m)) = orders::hand_off_recipe(w, content, &wish) else {
        return false;
    };
    let Some(r) = content.handson.recipe(&recipe) else {
        return false;
    };
    let Some(mi) = content.menu_item(&r.base_item) else {
        return false;
    };
    let cap = content.handson.tuning.hand_off_match_cap;
    let iid = w.alloc_id();
    w.econ.supplies -= mi.cost as i64;
    let now = w.minute();
    let leader = w.parties[pi].leader().unwrap_or(0);
    let party = w.parties[pi].id;
    w.items.push(OrderItem {
        id: iid,
        party,
        guest: leader,
        menu: r.base_item.clone(),
        state: ItemState::Pending,
        prep_left: mi.prep_minutes * TICKS_PER_MINUTE as f32,
        by: None,
        ready_tile: None,
        quality: 0.0,
        created: now,
        origin: ItemOrigin::Unassigned,
        price_add: 0,
        hand_off_match: Some(m.min(cap)),
        wish: true,
    });
    let p = &mut w.parties[pi];
    p.items.push(iid);
    p.wish_open = false;
    p.handed_off = true;
    true
}

/// `Command::PlayerHandOff`.
pub(crate) fn hand_off(w: &mut World, content: &Content, guest: u32) -> Result<(), CommandError> {
    night_only(w)?;
    let pi = party_of(w, guest).ok_or_else(|| CommandError::UnknownId(guest.to_string()))?;
    if !w.parties[pi].order_kind.is_wish() || w.parties[pi].handed_off {
        return Err(CommandError::Invalid("no wish to hand off".into()));
    }
    use crate::content::handson::Duty;
    let makers = super::duties::duty_on_floor(w, Duty::Bar)
        || super::duties::duty_on_floor(w, Duty::Kitchen);
    if !makers {
        return Err(CommandError::Invalid(
            "no helper on the bar or kitchen".into(),
        ));
    }
    let me = me(w);
    match w.parties[pi].state {
        PartyState::Queue | PartyState::WaitOrder => {
            if !super::duties::duty_on_floor(w, Duty::Register) {
                return Err(CommandError::Invalid("no helper on the register".into()));
            }
            let party = w.parties[pi].id;
            if w.parties[pi].order_claim.is_some_and(|c| c != me) {
                return Err(CommandError::InUse);
            }
            // drop her own pending take-order for them
            w.hands
                .player
                .queue
                .retain(|a| *a != PlayerAct::TakeOrder { party });
            if w.hands
                .player
                .current
                .as_ref()
                .is_some_and(|c| c.act == PlayerAct::TakeOrder { party })
            {
                cancel(w, content)?;
            }
            let p = &mut w.parties[pi];
            p.order_claim = None;
            p.handed_off = true;
            w.emit(SimEvent::PlayerQueueChanged);
            Ok(())
        }
        PartyState::WaitFood if w.parties[pi].wish_open => {
            if hand_off_item(w, content, pi) {
                Ok(())
            } else {
                Err(CommandError::Invalid("nothing on the menu fits".into()))
            }
        }
        _ => Err(CommandError::Invalid("no wish to hand off".into())),
    }
}

fn finish(w: &mut World) {
    w.hands.player.current = None;
    if let Some(i) = karolina_idx(w) {
        w.staff[i].pose = Pose::Idle;
    }
    w.emit(SimEvent::PlayerQueueChanged);
}

/// Drops the current act, releasing its claims.
fn drop_current(w: &mut World, content: &Content) {
    if let Some(cur) = w.hands.player.current.take() {
        release_act(w, content, &cur.act, cur.stage, cur.count);
        // the station she was holding
        let me = me(w);
        if let Some(st) = cur.target
            && let Some(f) = w.furniture.iter_mut().find(|f| f.uid == st)
            && f.user == Some(me)
        {
            f.user = None;
        }
    }
    if let Some(i) = karolina_idx(w) {
        w.staff[i].path.clear();
        w.staff[i].carrying = None;
        w.staff[i].pose = Pose::Idle;
    }
    w.emit(SimEvent::PlayerQueueChanged);
}

/// `Command::PlayerCancel`.
pub(crate) fn cancel(w: &mut World, content: &Content) -> Result<(), CommandError> {
    if w.hands.player.current.is_none() {
        return Err(CommandError::Invalid("nothing to cancel".into()));
    }
    drop_current(w, content);
    Ok(())
}

/// `Command::PlayerUnqueue`.
pub(crate) fn unqueue(w: &mut World, content: &Content, index: usize) -> Result<(), CommandError> {
    let act = w
        .hands
        .player
        .queue
        .remove(index)
        .ok_or_else(|| CommandError::Invalid("no such queued act".into()))?;
    release_act(w, content, &act, 0, 0);
    w.emit(SimEvent::PlayerQueueChanged);
    Ok(())
}

/// `Command::PlayerReorder`.
pub(crate) fn reorder(w: &mut World, from: usize, to: usize) -> Result<(), CommandError> {
    let q = &mut w.hands.player.queue;
    if from >= q.len() || to >= q.len() {
        return Err(CommandError::Invalid("no such queued act".into()));
    }
    if from == to {
        return Ok(());
    }
    if let Some(act) = q.remove(from) {
        q.insert(to, act);
    }
    w.emit(SimEvent::PlayerQueueChanged);
    Ok(())
}

/// `Command::PlayerClearQueue`.
pub(crate) fn clear_queue(w: &mut World, content: &Content) -> Result<(), CommandError> {
    if w.hands.player.queue.is_empty() {
        return Err(CommandError::Invalid("nothing queued".into()));
    }
    while let Some(act) = w.hands.player.queue.pop_back() {
        release_act(w, content, &act, 0, 0);
    }
    w.emit(SimEvent::PlayerQueueChanged);
    Ok(())
}

/// Drops whatever Karolina has queued or is doing for a story / moment walk that matches
/// `pred` (its guest left): the queued walk goes, the current one is cancelled.
pub(crate) fn drop_walks(
    w: &mut World,
    content: &Content,
    pred: impl Fn(&super::systems::WalkKind) -> bool,
) {
    // the current act first: its job is `walk_now` (working) or still in `walks` (walking)
    let cur_tile = w.hands.player.current.as_ref().and_then(|c| match c.act {
        PlayerAct::GoTo { tile } => Some((tile, c.stage)),
        _ => None,
    });
    if let Some((tile, stage)) = cur_tile {
        let hit = if stage == 0 {
            w.sys
                .moments
                .walks
                .iter()
                .any(|j| j.tile == tile && pred(&j.kind))
        } else {
            w.sys
                .moments
                .walk_now
                .as_ref()
                .is_some_and(|j| pred(&j.kind))
        };
        if hit {
            drop_current(w, content);
        }
    }
    let tiles: Vec<(i32, i32)> = w
        .sys
        .moments
        .walks
        .iter()
        .filter(|j| pred(&j.kind))
        .map(|j| j.tile)
        .collect();
    for tile in tiles {
        if let Some(i) = w
            .hands
            .player
            .queue
            .iter()
            .position(|a| *a == PlayerAct::GoTo { tile })
        {
            let _ = unqueue(w, content, i);
        }
    }
}

/// `Command::PlayerWashProgress`: she must be standing at that sink.
pub(crate) fn wash_progress(w: &mut World, sink: u32, seconds: f32) -> Result<(), CommandError> {
    let at_sink = w
        .hands
        .player
        .current
        .as_ref()
        .is_some_and(|c| c.act == PlayerAct::Wash { sink } && c.stage == 1);
    if !at_sink {
        return Err(CommandError::Invalid(
            "Karolina isn't at the sink yet".into(),
        ));
    }
    if !(seconds.is_finite() && seconds > 0.0) {
        return Err(CommandError::Invalid("seconds".into()));
    }
    let p = &mut w.hands.player;
    p.wash_bank += seconds.min(30.0) * WASH_PER_SECOND;
    let n = (p.wash_bank.floor() as u32).min(w.sink_dishes);
    p.wash_bank -= n as f32;
    w.sink_dishes -= n;
    if n > 0 {
        w.emit(SimEvent::Washed { dishes: n });
    }
    if let Some(c) = w.hands.player.current.as_mut() {
        c.waited = 0;
    }
    if w.sink_dishes == 0 {
        w.hands.player.wash_bank = 0.0;
        if w.sink_claim == Some(me(w)) {
            w.sink_claim = None;
        }
        finish(w);
    }
    Ok(())
}

/// `Command::PlayerGreet`: a wave from where she stands (no walk).
pub(crate) fn greet(w: &mut World, content: &Content, guest: u32) -> Result<(), CommandError> {
    night_only(w)?;
    let pi = party_of(w, guest).ok_or_else(|| CommandError::UnknownId(guest.to_string()))?;
    let g = &content.handson.tuning.greeting;
    let p = &w.parties[pi];
    if p.greeted || p.state == PartyState::Leaving {
        return Err(CommandError::Invalid("already greeted".into()));
    }
    if w.minute().saturating_sub(p.arrived) > g.window_minutes {
        return Err(CommandError::Invalid("they came in a while ago".into()));
    }
    let leader = p.leader();
    let regular = p.regular.clone();
    let p = &mut w.parties[pi];
    p.greeted = true;
    p.satisfaction = (p.satisfaction + g.satisfaction).min(100.0);
    if let Some(r) = regular {
        let t = w.story.trust.entry(r).or_insert(0.0);
        *t = (*t + g.trust).min(100.0);
    }
    let tick = w.tick;
    if let Some(gl) = leader.and_then(|l| w.guests.iter_mut().find(|g| g.id == l)) {
        gl.emote = Some((SimEmote::Smile, tick + 25));
    }
    if let Some(i) = karolina_idx(w) {
        w.staff[i].emote = Some((SimEmote::Smile, tick + 15));
    }
    w.emit(SimEvent::PlayerGreeted {
        guest: leader.unwrap_or(guest),
    });
    Ok(())
}

/// Night start / end: an empty queue.
pub(crate) fn reset(w: &mut World) {
    w.hands.player.queue.clear();
    w.hands.player.current = None;
    w.hands.player.idle_minutes = 0;
    w.hands.player.rhythm = 0;
    w.hands.player.last_serve_minute = None;
    w.hands.player.wash_bank = 0.0;
    w.sys.moments.walks.clear();
    w.sys.moments.walk_now = None;
}

/// Queues a walk with work at its end (a moment's option, a night experiment).
pub(crate) fn queue_walk(
    w: &mut World,
    content: &Content,
    tile: (i32, i32),
    kind: super::systems::WalkKind,
) -> Result<(), CommandError> {
    queue(w, content, PlayerAct::GoTo { tile }, false)?;
    w.sys
        .moments
        .walks
        .push(super::systems::WalkJob { tile, kind });
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Ticking
// ---------------------------------------------------------------------------------------------

fn work_speed(w: &World, content: &Content, job: Option<crate::content::Job>) -> f32 {
    let mut v = content.handson.tuning.player_work_mul * WORK_SPEED;
    for m in &w.mods {
        if let ModKind::WorkSpeed(j) = m.kind
            && job.is_some_and(|job| j.is_none_or(|j| j as usize == job.index()))
        {
            v *= m.mul;
        }
    }
    v.max(0.05)
}

enum Goal {
    At((i32, i32)),
    Wait,
    Fail,
}

fn checklist_anchor(w: &World, content: &Content, id: &str, from: (i32, i32)) -> (i32, i32) {
    if matches!(id, "light_window" | "light_off_at_six")
        && let Some(uid) = super::logic::window_light(w, content)
        && let Some(p) = w.furniture_by_uid(uid)
        && let Some(t) = staff::stand_next_to(w, (p.x, p.y), from)
    {
        return t;
    }
    let job = content
        .app
        .checklists
        .iter()
        .find(|c| c.id == id)
        .and_then(|c| c.job)
        .unwrap_or(crate::content::Job::Register);
    staff::checklist_tile(w, content, job)
}

/// Where the act's first stage happens.
fn first_goal(w: &World, content: &Content, idx: usize, act: &PlayerAct) -> Goal {
    let here = w.staff[idx].tile();
    let at = |t: Option<(i32, i32)>| t.map_or(Goal::Fail, Goal::At);
    match act {
        PlayerAct::TakeOrder { party } => {
            let Some(p) = w.parties.iter().find(|p| p.id == *party) else {
                return Goal::Fail;
            };
            if p.state == PartyState::Queue
                && let Some(r) = p.register
            {
                return at(staff::station_access(w, content, r));
            }
            let seated = p
                .leader()
                .is_some_and(|l| w.guests.iter().any(|g| g.id == l && g.pose == Pose::Sit));
            let seat = p.leader().and_then(|l| staff::guest_seat_tile(w, l));
            match seat {
                Some(s) if seated => at(staff::stand_next_to(w, s, here)),
                // still walking in: wait at the counter, not by an empty chair
                Some(_) => Goal::At(staff::checklist_tile(
                    w,
                    content,
                    crate::content::Job::Register,
                )),
                None => Goal::Wait,
            }
        }
        PlayerAct::Brew { item, .. } => {
            let Some(i) = w.items.iter().find(|i| i.id == *item) else {
                return Goal::Fail;
            };
            let Some(mi) = content.menu_item(&i.menu) else {
                return Goal::Fail;
            };
            match staff::station_free(w, content, &mi.station, me(w)) {
                Some(st) => at(staff::station_access(w, content, st)),
                None => Goal::Wait,
            }
        }
        PlayerAct::Serve { item, .. } => at(w
            .items
            .iter()
            .find(|i| i.id == *item)
            .and_then(|i| i.ready_tile)),
        PlayerAct::ClearTable { table } | PlayerAct::CollectTip { table } => {
            at(staff::station_access(w, content, *table))
        }
        PlayerAct::Wash { sink } => at(staff::station_access(w, content, *sink)),
        PlayerAct::Mop { tile } | PlayerAct::GoTo { tile } => Goal::At(*tile),
        PlayerAct::Checklist { id } => Goal::At(checklist_anchor(w, content, id, here)),
        PlayerAct::PlayRecord { .. } => {
            at(super::music::jukebox(w).and_then(|j| staff::station_access(w, content, j)))
        }
        PlayerAct::Use { uid, .. } => at(super::equipment::goal(w, content, *uid)),
    }
}

fn walk(w: &mut World, content: &Content, idx: usize) {
    let speed = STAFF_SPEED * content.handson.tuning.player_walk_mul;
    let carrying = w.staff[idx].carrying.is_some();
    let s = &mut w.staff[idx];
    let (vel, _arrived, blocked) =
        guests::step_along(&w.grid, &mut s.pos, &mut s.path, speed, false);
    s.pose = if carrying { Pose::Carry } else { Pose::Walk };
    if vel.0.abs() + vel.1.abs() > 0.01 {
        s.facing = Facing::from_delta(vel.0, vel.1);
        s.stuck_ticks = 0;
    } else {
        s.stuck_ticks += 1;
    }
    if blocked {
        let goal = *w.staff[idx].path.last().unwrap_or(&w.staff[idx].tile());
        if !staff::go(w, idx, goal) {
            drop_current(w, content);
        }
    }
    if w.staff[idx].stuck_ticks > 300 {
        if let Some(g) = w.staff[idx].path.last().copied() {
            w.staff[idx].pos = grid::tile_center(g);
            w.staff[idx].path.clear();
        }
        w.staff[idx].stuck_ticks = 0;
        w.tonight.stuck_resets += 1;
    }
}

/// Starts the next queued act (or keeps waiting for a busy station / a guest still walking in).
fn start_next(w: &mut World, content: &Content, idx: usize) {
    while w.hands.player.current.is_none() {
        let Some(act) = w.hands.player.queue.pop_front() else {
            return;
        };
        w.hands.player.current = Some(ActiveAct {
            act,
            stage: 0,
            work_left: 0.0,
            work_total: 0.0,
            waited: 0,
            target: None,
            count: 0,
        });
        w.emit(SimEvent::PlayerQueueChanged);
        if !enter_stage0(w, content, idx) {
            drop_current(w, content);
        }
    }
}

/// Heads for the act's first spot. False = impossible (the act is dropped).
fn enter_stage0(w: &mut World, content: &Content, idx: usize) -> bool {
    let Some(cur) = w.hands.player.current.clone() else {
        return false;
    };
    match first_goal(w, content, idx, &cur.act) {
        Goal::At(t) => {
            if let PlayerAct::Brew { item, .. } = &cur.act
                && let Some(mi) = w
                    .items
                    .iter()
                    .find(|i| i.id == *item)
                    .and_then(|i| content.menu_item(&i.menu))
                && let Some(st) = staff::station_free(w, content, &mi.station, me(w))
            {
                let me = me(w);
                if let Some(f) = w.furniture.iter_mut().find(|f| f.uid == st) {
                    f.user = Some(me);
                }
                if let Some(c) = w.hands.player.current.as_mut() {
                    c.target = Some(st);
                }
            }
            if let Some(c) = w.hands.player.current.as_mut() {
                c.stage = 0;
                c.waited = 0;
            }
            w.staff[idx].tile() == t || staff::go(w, idx, t)
        }
        Goal::Wait => true,
        Goal::Fail => false,
    }
}

fn set_work(w: &mut World, stage: u8, work: f32) {
    if let Some(c) = w.hands.player.current.as_mut() {
        c.stage = stage;
        c.work_left = work;
        c.work_total = work;
    }
}

/// Picks up the tray (her ready items of it) and heads to the first guest's seat (stage 2).
/// False = they're gone.
fn carry_tray(w: &mut World, idx: usize, tray: &[u32]) -> bool {
    let me = w.staff[idx].id;
    let mine: Vec<(u32, u32, String)> = w
        .items
        .iter()
        .filter(|i| {
            tray.contains(&i.id)
                && i.by == Some(me)
                && matches!(i.state, ItemState::Ready | ItemState::Carrying)
        })
        .map(|i| (i.id, i.guest, i.menu.clone()))
        .collect();
    let here = w.staff[idx].tile();
    let Some(t) = mine.iter().find_map(|(_, g, _)| {
        staff::guest_seat_tile(w, *g).and_then(|s| staff::stand_next_to(w, s, here))
    }) else {
        return false;
    };
    for i in w.items.iter_mut() {
        if mine.iter().any(|m| m.0 == i.id) {
            i.state = ItemState::Carrying;
        }
    }
    w.staff[idx].carrying = mine.first().map(|m| m.2.clone());
    set_work(w, 2, 0.0);
    w.staff[idx].tile() == t || staff::go(w, idx, t)
}

/// Serves everything on the tray.
fn hand_over(w: &mut World, content: &Content, idx: usize, tray: &[u32]) {
    let me = w.staff[idx].id;
    w.staff[idx].carrying = None;
    let carried: Vec<u32> = w
        .items
        .iter()
        .filter(|i| tray.contains(&i.id) && i.by == Some(me) && i.state == ItemState::Carrying)
        .map(|i| i.id)
        .collect();
    for id in carried {
        guests::item_served(w, content, id, me);
    }
    finish(w);
}

pub(crate) fn tick(w: &mut World, content: &Content, idx: usize) {
    if !w.staff[idx].path.is_empty() {
        walk(w, content, idx);
        return;
    }
    if w.hands.player.current.is_none() {
        start_next(w, content, idx);
        if w.hands.player.current.is_none() {
            w.staff[idx].pose = Pose::Idle;
            return;
        }
        if !w.staff[idx].path.is_empty() {
            return;
        }
    }
    step(w, content, idx);
}

/// One tick of the current act, standing at its spot.
fn step(w: &mut World, content: &Content, idx: usize) {
    let Some(cur) = w.hands.player.current.clone() else {
        return;
    };
    let me = w.staff[idx].id;
    let here = w.staff[idx].tile();
    match cur.act.clone() {
        PlayerAct::TakeOrder { party } => {
            let Some(p) = w.parties.iter().find(|p| p.id == party) else {
                return drop_current(w, content);
            };
            if !matches!(p.state, PartyState::Queue | PartyState::WaitOrder)
                || p.order_claim != Some(me)
            {
                return drop_current(w, content);
            }
            match cur.stage {
                0 => {
                    // at a table she waits for them to sit down
                    let seated = p.state == PartyState::Queue
                        || p.leader().is_some_and(|l| {
                            w.guests.iter().any(|g| g.id == l && g.pose == Pose::Sit)
                        });
                    match first_goal(w, content, idx, &cur.act) {
                        Goal::At(t) if t != here => {
                            if !staff::go(w, idx, t) {
                                drop_current(w, content);
                            }
                        }
                        Goal::At(_) if !seated => wait_a_tick(w, content, STATION_PATIENCE),
                        Goal::At(_) => set_work(w, 1, TAKE_ORDER_WORK),
                        Goal::Wait => wait_a_tick(w, content, STATION_PATIENCE),
                        Goal::Fail => drop_current(w, content),
                    }
                }
                _ => {
                    w.staff[idx].pose = Pose::Work;
                    if work_tick(w, work_speed(w, content, None)) {
                        guests::order_taken(w, content, party, me);
                        finish(w);
                    }
                }
            }
        }
        PlayerAct::Brew { item, extra } => {
            let Some(it) = w.items.iter().find(|i| i.id == item) else {
                return drop_current(w, content);
            };
            if it.by != Some(me) || matches!(it.state, ItemState::Served | ItemState::Cancelled) {
                return drop_current(w, content);
            }
            let tray: Vec<u32> = std::iter::once(item).chain(extra.iter().copied()).collect();
            match cur.stage {
                0 => {
                    if cur.target.is_none() {
                        // the station was busy: try again
                        match first_goal(w, content, idx, &cur.act) {
                            Goal::At(_) => {
                                if !enter_stage0(w, content, idx) {
                                    drop_current(w, content);
                                }
                            }
                            Goal::Wait => wait_a_tick(w, content, STATION_PATIENCE),
                            Goal::Fail => drop_current(w, content),
                        }
                        return;
                    }
                    let prep: f32 = w
                        .items
                        .iter()
                        .filter(|i| tray.contains(&i.id) && i.by == Some(me))
                        .map(|i| i.prep_left.max(0.0))
                        .sum();
                    set_work(w, 1, prep);
                }
                1 => {
                    // the round, one item after another
                    let Some(next) = w
                        .items
                        .iter()
                        .find(|i| {
                            tray.contains(&i.id)
                                && i.by == Some(me)
                                && matches!(i.state, ItemState::Pending | ItemState::Preparing)
                        })
                        .map(|i| (i.id, i.menu.clone()))
                    else {
                        // a by-hand brew started early waits for its mini-game's quality
                        if let Some(&(wid, since)) =
                            w.sys.brew_waiting.iter().find(|(i, _)| tray.contains(i))
                        {
                            w.staff[idx].pose = Pose::Work;
                            if w.tick.saturating_sub(since) < BREW_QUALITY_PATIENCE {
                                if cur.waited == 0 {
                                    let guest = w
                                        .items
                                        .iter()
                                        .find(|i| i.id == wid)
                                        .map(|i| i.guest)
                                        .unwrap_or(0);
                                    w.emit(SimEvent::BrewAwaitsQuality { guest, item: wid });
                                }
                                if let Some(c) = w.hands.player.current.as_mut() {
                                    c.waited += 1;
                                }
                                return;
                            }
                            // the mini-game was closed: the skip quality
                            let q = content.handson.minigames.common.skip_quality;
                            set_brew_quality(w, wid, q);
                        }
                        // all made: release the station, carry the tray over
                        if let Some(st) = cur.target
                            && let Some(f) = w.furniture.iter_mut().find(|f| f.uid == st)
                            && f.user == Some(me)
                        {
                            f.user = None;
                        }
                        if let Some(c) = w.hands.player.current.as_mut() {
                            c.target = None;
                        }
                        if !carry_tray(w, idx, &tray) {
                            // they left: the cups stay on the counter
                            w.staff[idx].carrying = None;
                            for x in &tray {
                                release_item(w, *x, me, Some(here));
                            }
                            finish(w);
                        }
                        return;
                    };
                    let mi = content.menu_item(&next.1);
                    let powered =
                        !(mi.is_some_and(|m| m.needs_power) && w.blackout_until.is_some());
                    w.staff[idx].pose = if powered { Pose::Work } else { Pose::Idle };
                    if let Some(st) = cur.target.and_then(|t| w.furniture_by_uid(t)) {
                        let d = (st.x - here.0, st.y - here.1);
                        w.staff[idx].facing = Facing::from_delta(d.0 as f32, d.1 as f32);
                    }
                    if !powered {
                        return;
                    }
                    let mut speed = work_speed(w, content, mi.map(|m| m.job));
                    if let Some(tag) = cur
                        .target
                        .and_then(|t| w.furniture_by_uid(t))
                        .and_then(|p| content.furniture_def(&p.def))
                        .and_then(|d| d.tags.iter().find(|t| t.starts_with("station:")).cloned())
                    {
                        for md in &w.mods {
                            if matches!(&md.kind, ModKind::StationSlow(x) if *x == tag) {
                                speed *= md.mul;
                            }
                        }
                    }
                    let mut done = false;
                    if let Some(i) = w.items.iter_mut().find(|i| i.id == next.0) {
                        i.state = ItemState::Preparing;
                        i.prep_left -= speed;
                        if i.prep_left <= 0.0 {
                            i.state = ItemState::Ready;
                            i.ready_tile = Some(here);
                            done = true;
                        }
                    }
                    if let Some(c) = w.hands.player.current.as_mut() {
                        c.work_left = (c.work_left - speed).max(0.0);
                    }
                    if done {
                        w.emit(SimEvent::OrderReady {
                            item: next.1,
                            staff: me,
                        });
                    }
                }
                2 => set_work(w, 3, HAND_OVER_WORK),
                _ => {
                    if work_tick(w, 1.0) {
                        hand_over(w, content, idx, &tray);
                    }
                }
            }
        }
        PlayerAct::Serve { item, extra } => {
            let Some(it) = w.items.iter().find(|i| i.id == item) else {
                return drop_current(w, content);
            };
            if it.by != Some(me) || matches!(it.state, ItemState::Served | ItemState::Cancelled) {
                return drop_current(w, content);
            }
            let tray: Vec<u32> = std::iter::once(item).chain(extra.iter().copied()).collect();
            match cur.stage {
                0 | 1 => {
                    if !carry_tray(w, idx, &tray) {
                        drop_current(w, content);
                    }
                }
                2 => set_work(w, 3, HAND_OVER_WORK),
                _ => {
                    if work_tick(w, 1.0) {
                        hand_over(w, content, idx, &tray);
                    }
                }
            }
        }
        PlayerAct::ClearTable { table } => match cur.stage {
            0 => {
                let n = w.furniture_by_uid(table).map(|f| f.dishes).unwrap_or(0);
                set_work(w, 1, CLEAR_WORK + n as f32);
            }
            1 => {
                w.staff[idx].pose = Pose::Work;
                if work_tick(w, work_speed(w, content, None)) {
                    let mut n = 0;
                    if let Some(f) = w.furniture.iter_mut().find(|f| f.uid == table) {
                        n = f.dishes;
                        f.dishes = 0;
                        f.dirt = 0.0;
                        f.claim = None;
                    }
                    collect_tips_on(w, content, table, Some(me));
                    w.emit(SimEvent::TableCleared {
                        table,
                        dishes: n,
                        by: me,
                    });
                    let sink = w
                        .furniture
                        .iter()
                        .find(|p| {
                            content
                                .furniture_def(&p.def)
                                .is_some_and(|d| d.has_tag("dishes"))
                        })
                        .map(|p| p.uid);
                    let goal = sink.and_then(|s| staff::station_access(w, content, s));
                    if n == 0 {
                        return finish(w);
                    }
                    w.staff[idx].carrying = Some("dishes".into());
                    if let Some(c) = w.hands.player.current.as_mut() {
                        c.stage = 2;
                        c.count = n;
                    }
                    let moved = goal.is_some_and(|g| here == g || staff::go(w, idx, g));
                    if !moved {
                        w.sink_dishes += n;
                        w.staff[idx].carrying = None;
                        finish(w);
                    }
                }
            }
            _ => {
                w.sink_dishes += cur.count;
                w.staff[idx].carrying = None;
                finish(w);
            }
        },
        PlayerAct::Wash { .. } => {
            if w.sink_dishes == 0 {
                if w.sink_claim == Some(me) {
                    w.sink_claim = None;
                }
                return finish(w);
            }
            if cur.stage == 0 {
                set_work(w, 1, 0.0);
                return;
            }
            w.staff[idx].pose = Pose::Work;
            let more = !w.hands.player.queue.is_empty();
            let limit = if more {
                SINK_PATIENCE / 2
            } else {
                SINK_PATIENCE
            };
            if let Some(c) = w.hands.player.current.as_mut() {
                c.waited += 1;
                if c.waited > limit {
                    drop_current(w, content);
                }
            }
        }
        PlayerAct::Mop { .. } => match cur.stage {
            0 => set_work(w, 1, MOP_WORK),
            _ => {
                w.staff[idx].pose = Pose::Work;
                if work_tick(w, work_speed(w, content, None)) {
                    w.dirt = (w.dirt - MOP_AMOUNT).max(0.0);
                    if let PlayerAct::Mop { tile } = cur.act {
                        w.emit(SimEvent::Mopped { tile });
                    }
                    finish(w);
                }
            }
        },
        PlayerAct::CollectTip { table } => match cur.stage {
            0 => set_work(w, 1, TIP_WORK),
            _ => {
                if work_tick(w, 1.0) {
                    collect_tips_on(w, content, table, Some(me));
                    finish(w);
                }
            }
        },
        PlayerAct::Checklist { id } => match cur.stage {
            0 => set_work(
                w,
                1,
                CHECKLIST_WORK * super::tool::checklist_speed(w, content, &id),
            ),
            _ => {
                w.staff[idx].pose = Pose::Work;
                if work_tick(w, 1.0) {
                    super::app::tick_checklist(w, content, &id, Some(me));
                    finish(w);
                }
            }
        },
        PlayerAct::PlayRecord { record, repeat } => match cur.stage {
            0 => set_work(w, 1, NEEDLE_WORK),
            _ => {
                w.staff[idx].pose = Pose::Work;
                if work_tick(w, 1.0) {
                    if super::music::validate(w, &record).is_ok() {
                        super::music::set_record(w, content, record, repeat);
                    }
                    finish(w);
                }
            }
        },
        PlayerAct::Use { uid, action } => use_step(w, content, idx, &cur, uid, action),
        PlayerAct::GoTo { tile } => match cur.stage {
            0 => {
                // a moment's option or a night experiment waits at the end of this walk
                let pos = w.sys.moments.walks.iter().position(|j| j.tile == tile);
                let Some(pos) = pos else {
                    return finish(w);
                };
                let job = w.sys.moments.walks.remove(pos);
                let work = match &job.kind {
                    super::systems::WalkKind::Moment { id, option } => {
                        if !w.sys.moments.active.iter().any(|a| &a.id == id) {
                            return finish(w);
                        }
                        super::moments::option_work(content, id, option, w)
                    }
                    super::systems::WalkKind::Experiment { .. } => {
                        content.handson.tuning.experiment_minutes.max(1) as f32
                            * TICKS_PER_MINUTE as f32
                    }
                    k @ (super::systems::WalkKind::Story { .. }
                    | super::systems::WalkKind::Storm { .. }) => {
                        match super::stories::walk_work(w, content, k) {
                            Some(work) => work,
                            None => {
                                super::stories::release(w, k);
                                return finish(w);
                            }
                        }
                    }
                };
                w.sys.moments.walk_now = Some(job);
                set_work(w, 1, work);
            }
            _ => {
                w.staff[idx].pose = Pose::Work;
                if work_tick(w, 1.0) {
                    match w.sys.moments.walk_now.take().map(|j| j.kind) {
                        Some(super::systems::WalkKind::Moment { id, option }) => {
                            super::moments::arrive(w, content, &id, &option);
                        }
                        Some(super::systems::WalkKind::Experiment { ingredients })
                            if super::knowledge::validate(w, content, &ingredients).is_ok() =>
                        {
                            super::knowledge::run(w, content, &ingredients);
                        }
                        Some(
                            k @ (super::systems::WalkKind::Story { .. }
                            | super::systems::WalkKind::Storm { .. }),
                        ) => super::stories::arrive(w, content, &k),
                        _ => {}
                    }
                    finish(w);
                }
            }
        },
    }
}

/// One tick of an appliance chore (`sim::equipment`): work at the piece (a hold chore goes
/// faster while the player holds, `Command::EquipmentHold`); the trash run then carries the
/// bag out of the door and works there a moment.
fn use_step(
    w: &mut World,
    content: &Content,
    idx: usize,
    cur: &ActiveAct,
    uid: u32,
    action: super::equipment::EquipAction,
) {
    use super::equipment::{self as eq, EquipAction};
    let me = w.staff[idx].id;
    match cur.stage {
        0 => set_work(w, 1, eq::work_ticks(content, action)),
        1 => {
            w.staff[idx].pose = Pose::Work;
            if let Some(p) = w.furniture_by_uid(uid) {
                let here = w.staff[idx].tile();
                let d = (p.x - here.0, p.y - here.1);
                w.staff[idx].facing = Facing::from_delta(d.0 as f32, d.1 as f32);
            }
            if !work_tick(w, work_speed(w, content, None)) {
                return;
            }
            // (a helper may have done it meanwhile, the pot stopped bubbling…: then nothing)
            if !eq::possible(w, content, uid, action) {
                eq::release(w, action);
                return finish(w);
            }
            if action == EquipAction::Trash
                && let Some(t) = eq::outside_spot(w)
            {
                set_work(w, 2, 0.0);
                if w.staff[idx].tile() == t || staff::go(w, idx, t) {
                    return;
                }
            }
            eq::complete(w, content, uid, action, Some(me));
            finish(w);
        }
        2 => set_work(w, 3, eq::trash_ticks(content)),
        _ => {
            w.staff[idx].pose = Pose::Idle;
            if work_tick(w, 1.0) {
                if eq::possible(w, content, uid, action) {
                    eq::complete(w, content, uid, action, Some(me));
                }
                finish(w);
            }
        }
    }
}

/// Ticks Karolina holds a finished by-hand cup waiting for `PlayerBrewQuality` (the
/// mini-game overlay slows the sim, so this is a long real wait).
const BREW_QUALITY_PATIENCE: u32 = 200;

/// Sets the quality of a brew started early (`PlayerBrewStart`).
fn set_brew_quality(w: &mut World, item: u32, q: f32) {
    w.sys.brew_waiting.retain(|(i, _)| *i != item);
    if let Some(it) = w.items.iter_mut().find(|i| i.id == item)
        && let ItemOrigin::Player { quality, .. } = &mut it.origin
    {
        *quality = q.clamp(0.0, 1.0);
    }
}

/// `Command::PlayerBrewStart`: brew now, the quality later.
pub(crate) fn brew_start(
    w: &mut World,
    content: &Content,
    guest: u32,
    recipe: &str,
    mode: BrewMode,
) -> Result<(), CommandError> {
    let q = content.handson.minigames.common.skip_quality;
    let brews = |w: &World| -> Vec<u32> {
        w.hands
            .player
            .queue
            .iter()
            .filter_map(|a| match a {
                PlayerAct::Brew { item, .. } => Some(*item),
                _ => None,
            })
            .collect()
    };
    let before = brews(w);
    brew(w, content, guest, recipe, q, mode)?;
    // the brew just queued (a regular's goes to the front, the others to the back)
    let added = brews(w).into_iter().find(|i| !before.contains(i));
    if mode == BrewMode::ByHand
        && let Some(item) = added
    {
        let tick = w.tick;
        w.sys.brew_waiting.push((item, tick));
    }
    Ok(())
}

/// `Command::PlayerBrewQuality`.
pub(crate) fn brew_quality_in(w: &mut World, guest: u32, quality: f32) -> Result<(), CommandError> {
    if !quality.is_finite() {
        return Err(CommandError::Invalid("quality".into()));
    }
    let pi = party_of(w, guest).ok_or_else(|| CommandError::UnknownId(guest.to_string()))?;
    let party = w.parties[pi].id;
    let item = w
        .sys
        .brew_waiting
        .iter()
        .map(|(i, _)| *i)
        .find(|i| w.items.iter().any(|x| x.id == *i && x.party == party))
        .ok_or_else(|| CommandError::Invalid("no brew waits for a quality".into()))?;
    set_brew_quality(w, item, quality);
    Ok(())
}

/// Works one tick; true when the stage's work is done.
fn work_tick(w: &mut World, speed: f32) -> bool {
    let Some(c) = w.hands.player.current.as_mut() else {
        return false;
    };
    c.work_left -= speed;
    c.work_left <= 0.0
}

fn wait_a_tick(w: &mut World, content: &Content, limit: u32) {
    if let Some(i) = karolina_idx(w) {
        w.staff[i].pose = Pose::Idle;
    }
    let over = w.hands.player.current.as_mut().is_some_and(|c| {
        c.waited += 1;
        c.waited > limit
    });
    if over {
        drop_current(w, content);
    }
}

// ---------------------------------------------------------------------------------------------
// Tips
// ---------------------------------------------------------------------------------------------

/// Books coins into the jar (`by` = who picked them up; `None` = the night end).
pub(crate) fn collect_coins(
    w: &mut World,
    content: &Content,
    coins: Vec<TipCoin>,
    by: Option<u32>,
) {
    if coins.is_empty() {
        return;
    }
    let mut total = 0;
    let mut napkins = Vec::new();
    for c in coins {
        if let Some(n) = c.napkin.clone() {
            napkins.push(n);
        }
        total += c.amount;
        w.econ.money += c.amount;
        w.tonight.tips += c.amount;
        w.app.week_tips += c.amount;
        w.emit(SimEvent::TipReceived {
            amount: c.amount,
            staff: c.staff,
        });
    }
    w.hands.tips_lifetime += total;
    w.hands.tally.tips_collected += total;
    w.emit(SimEvent::TipCollected { amount: total, by });
    for n in napkins {
        super::moments::collect_napkin(w, content, &n);
    }
    super::moments::check_tip_goals(w, content);
}

pub(crate) fn collect_tips_on(w: &mut World, content: &Content, table: u32, by: Option<u32>) {
    let mut coins = Vec::new();
    w.hands.tips.retain(|t| {
        if t.table == table {
            coins.push(t.clone());
            false
        } else {
            true
        }
    });
    collect_coins(w, content, coins, by);
}

/// 06:00: nothing is ever lost.
pub(crate) fn collect_all_tips(w: &mut World, content: &Content) {
    let coins = std::mem::take(&mut w.hands.tips);
    collect_coins(w, content, coins, None);
}
