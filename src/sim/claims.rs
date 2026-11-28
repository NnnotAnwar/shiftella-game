//! One claim rule for the work Karolina and the crew share (v0.2.5 play report: "the helper
//! and I both mop the same spot — he finishes, and I finish the same thing a second later").
//!
//! Every piece of shared work ([`Work`]: a party's order, an order item to make or carry, a
//! table's cups, a table's tip coins, the sink, a dirt blot, a bucket, a checklist item) has
//! **one holder** at a time:
//! * Karolina holds what she has queued — from the click on, so also while she walks there
//!   (`player::claim_act`);
//! * a helper holds what they picked (`staff::claim`), until it is done or dropped.
//!
//! Nobody takes what someone else holds:
//! * a helper never picks work anybody else holds — every candidate of `staff::find_work`
//!   goes through [`free_for`];
//! * Karolina's click on work a helper holds takes another free piece of the same kind when
//!   there is one (another dirt blot; a brew or a serve already picks a free cup of the
//!   party), else it is refused with `CommandError::InUse` and the UI says who has it
//!   ([`World::work_holder`]). The helper is never pulled off work they started;
//! * on arrival each side checks the work is still there (a blot already gone is not swiped).
//!
//! Most holders are the claim fields the sim always had (`Party::order_claim`,
//! `OrderItem::by`, `PlacedFurniture::claim`, `World::sink_claim`, the bucket's and the
//! checklist item's claim). Two are read off the acts and tasks themselves: a dirt blot (the
//! floor's dirt is one number; its blots are `World::dirt_spots`) and a table's tip coins (a
//! helper may clear the cups of a table she is going to for the coin: the coin stays for her).

use super::hands::PlayerAct;
use super::player::karolina_idx;
use super::world::*;

/// A piece of shared work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Work {
    /// Taking a party's order.
    Order(u32),
    /// Making or carrying an order item.
    Item(u32),
    /// Clearing a table's cups.
    Table(u32),
    /// Taking the tip coins off a table (whoever clears it takes them too).
    Tip(u32),
    /// Washing up at the sink.
    Sink,
    /// A dirt blot on the hall floor.
    Blot((i32, i32)),
    /// A bucket under a leak.
    Bucket((i32, i32)),
    /// A checklist item.
    Checklist(String),
}

/// What a helper's task works on (nothing once the dishes are picked up: the table is free).
pub(crate) fn of_task(t: &Task) -> Vec<Work> {
    match t {
        Task::TakeOrder { party, .. } => vec![Work::Order(*party)],
        Task::Prepare { item, .. } => vec![Work::Item(*item)],
        Task::Serve { item, extra, .. } => std::iter::once(item)
            .chain(extra)
            .map(|i| Work::Item(*i))
            .collect(),
        Task::ClearTable { table, picked, .. } if !picked => vec![Work::Table(*table)],
        Task::Wash { .. } => vec![Work::Sink],
        Task::Mop { tile } => vec![Work::Blot(*tile)],
        Task::Bucket { tile } => vec![Work::Bucket(*tile)],
        Task::Checklist { id, .. } => vec![Work::Checklist(id.clone())],
        _ => Vec::new(),
    }
}

/// What one of Karolina's acts works on (`carrying` = her dishes are on the way to the sink:
/// the table is free again). Appliance chores keep their own rule (`sim::equipment`).
pub(crate) fn of_act(a: &PlayerAct, carrying: bool) -> Vec<Work> {
    match a {
        PlayerAct::TakeOrder { party } => vec![Work::Order(*party)],
        PlayerAct::Brew { item, extra } | PlayerAct::Serve { item, extra } => {
            std::iter::once(item)
                .chain(extra)
                .map(|i| Work::Item(*i))
                .collect()
        }
        PlayerAct::ClearTable { table } if !carrying => {
            vec![Work::Table(*table), Work::Tip(*table)]
        }
        PlayerAct::CollectTip { table } => vec![Work::Tip(*table)],
        PlayerAct::Wash { .. } => vec![Work::Sink],
        PlayerAct::Mop { tile } => vec![Work::Blot(*tile)],
        PlayerAct::Checklist { id } => vec![Work::Checklist(id.clone())],
        _ => Vec::new(),
    }
}

/// Karolina's acts (the current one first) with whether her dishes are already on the way.
fn her_acts(w: &World) -> impl Iterator<Item = (&PlayerAct, bool)> {
    let p = &w.hands.player;
    p.current
        .iter()
        .map(|c| {
            (
                &c.act,
                matches!(c.act, PlayerAct::ClearTable { .. }) && c.stage >= 2,
            )
        })
        .chain(p.queue.iter().map(|a| (a, false)))
}

/// Karolina's id, when she is behind the counter tonight.
fn karolina_on(w: &World) -> Option<u32> {
    karolina_idx(w)
        .filter(|&i| w.staff[i].on_duty())
        .map(|i| w.staff[i].id)
}

/// Does Karolina have an act on `work`?
pub(crate) fn hers(w: &World, work: &Work) -> bool {
    her_acts(w).any(|(a, carrying)| of_act(a, carrying).contains(work))
}

/// Does one of Karolina's acts still need checklist item `id` (ticking it, or an appliance
/// chore that ticks it)? Then dropping another act must not let go of the item's claim.
pub(crate) fn checklist_still_hers(w: &World, id: &str) -> bool {
    her_acts(w).any(|(a, _)| match a {
        PlayerAct::Checklist { id: x } => x == id,
        PlayerAct::Use { action, .. } => action.checklist() == Some(id),
        _ => false,
    })
}

/// Who holds `work` now (a staff id; Karolina's too), if anybody.
pub(crate) fn holder(w: &World, work: &Work) -> Option<u32> {
    match work {
        Work::Order(p) => w
            .parties
            .iter()
            .find(|x| x.id == *p)
            .and_then(|x| x.order_claim),
        Work::Item(i) => w
            .items
            .iter()
            .find(|x| x.id == *i && !matches!(x.state, ItemState::Served | ItemState::Cancelled))
            .and_then(|x| x.by),
        Work::Table(t) => w.furniture_by_uid(*t).and_then(|f| f.claim),
        // hers when she goes for them; else the helper about to clear the table
        Work::Tip(t) => karolina_on(w).filter(|_| hers(w, work)).or_else(|| {
            w.staff
                .iter()
                .find(|s| {
                    !s.owner
                        && s.on_duty()
                        && matches!(s.task, Some(Task::ClearTable { table, picked: false, .. }) if table == *t)
                })
                .map(|s| s.id)
        }),
        Work::Sink => w.sink_claim,
        Work::Blot(_) => karolina_on(w).filter(|_| hers(w, work)).or_else(|| {
            w.staff
                .iter()
                .find(|s| {
                    !s.owner
                        && s.on_duty()
                        && s.task.as_ref().is_some_and(|t| of_task(t).contains(work))
                })
                .map(|s| s.id)
        }),
        Work::Bucket(t) => w.buckets.iter().find(|b| b.0 == *t).and_then(|b| b.2),
        Work::Checklist(id) => w
            .app
            .checklist
            .iter()
            .find(|c| &c.id == id && !c.done)
            .and_then(|c| c.claim),
    }
}

/// Nobody but `me` holds `work`.
pub(crate) fn free_for(w: &World, work: &Work, me: u32) -> bool {
    holder(w, work).is_none_or(|h| h == me)
}

/// A dirt blot nobody holds.
pub(crate) fn blot_free(w: &World, t: (i32, i32)) -> bool {
    holder(w, &Work::Blot(t)).is_none()
}

#[cfg(test)]
/// Who (staff id) is on what right now: Karolina's acts and the helpers' tasks.
pub(crate) fn active(w: &World) -> Vec<(u32, Work)> {
    let mut out = Vec::new();
    if let Some(k) = karolina_on(w) {
        for (a, carrying) in her_acts(w) {
            out.extend(of_act(a, carrying).into_iter().map(|x| (k, x)));
        }
    }
    for s in w.staff.iter().filter(|s| !s.owner && s.on_duty()) {
        if let Some(t) = &s.task {
            out.extend(of_task(t).into_iter().map(|x| (s.id, x)));
        }
    }
    out
}

#[cfg(test)]
/// Work two people are on at once, or that someone works on while another holds it (the
/// chaos invariant; empty when the rule holds).
pub(crate) fn clashes(w: &World) -> Vec<String> {
    let on = active(w);
    let mut out = Vec::new();
    for (k, (who, work)) in on.iter().enumerate() {
        if let Some((other, _)) = on[..k].iter().find(|(o, x)| x == work && o != who) {
            out.push(format!("{work:?}: staff {other} and {who} both on it"));
        }
        if let Some(h) = holder(w, work)
            && h != *who
        {
            out.push(format!("{work:?}: staff {who} on it, held by {h}"));
        }
    }
    out
}

impl World {
    /// Who else already has the work this act of Karolina's would do — a helper's staff id
    /// (for "Tomáš is already on it"); `None` when it is free or already hers.
    pub fn work_holder(&self, act: &PlayerAct) -> Option<u32> {
        let me = karolina_idx(self).map(|i| self.staff[i].id);
        of_act(act, false)
            .iter()
            .filter_map(|x| holder(self, x))
            .find(|h| Some(*h) != me)
    }
}
