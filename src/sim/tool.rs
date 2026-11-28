//! The Shiftella app as a real tool (design/handson.md §13): checklist rituals with real
//! effects, bot buttons whose handlers act in the world, handover notes the next night acts
//! on, digest insights with deep links, and the weekly payroll decision (tip split, star of
//! the week, granted náhradní volno). Never clock-in/out.

use super::app::{self, week_of};
use super::systems::*;
use super::types::*;
use super::world::*;
use super::xlogic;
use crate::content::handson::{
    BotHandler, ChecklistOwner, HandoverSubject, HighlightKind, TipSplit,
};
use crate::content::world::ChecklistKind;
use crate::content::{ClockTime, Content};

fn list_index(l: ChecklistKind) -> usize {
    match l {
        ChecklistKind::Opening => 0,
        ChecklistKind::Closing => 1,
    }
}

// ---------------------------------------------------------------------------------------------
// Checklists & rituals
// ---------------------------------------------------------------------------------------------

/// Work × for a checklist item tonight (closing properly halves tomorrow's opening).
pub(crate) fn checklist_speed(w: &World, content: &Content, id: &str) -> f32 {
    content
        .app
        .checklists
        .iter()
        .find(|c| c.id == id)
        .map(|c| w.sys.fx.checklist_speed[list_index(c.list)])
        .unwrap_or(1.0)
        .max(0.1)
}

/// An item was ticked: its hands-on `extra_done`, the handover note prefill, rituals.
pub(crate) fn on_checklist_done(w: &mut World, content: &Content, id: &str) {
    let Some(def) = content.app.checklists.iter().find(|c| c.id == id) else {
        return;
    };
    let fx = def.extra_done.clone();
    xlogic::run(w, content, &fx, &XTarget::default());
    // by hand: stronger, the evening mood, grandma's habits (`sim::rituals`)
    super::rituals::on_item_done(w, content, id);
    if id == "handover_note" && w.sys.tool.picks.is_empty() {
        w.sys.tool.suggestions = suggestions(w, content);
        w.sys.tool.picks = w
            .sys
            .tool
            .suggestions
            .iter()
            .take(2)
            .map(|s| s.id.clone())
            .collect();
    }
    if id == "record_on" && super::music::current_record(w).is_none() {
        let rec = w.hands.music.unlocked.iter().next().cloned();
        if rec.is_some() && super::music::validate(w, &rec).is_ok() {
            super::music::set_record(w, content, rec, true);
        }
    }
    check_rituals(w, content);
}

pub(crate) fn list_done(w: &World, content: &Content, list: ChecklistKind) -> bool {
    content
        .app
        .checklists
        .iter()
        .filter(|c| c.list == list)
        .all(|c| w.app.checklist.iter().any(|s| s.id == c.id && s.done))
}

fn check_rituals(w: &mut World, content: &Content) {
    let m = w.minute();
    for r in &content.app.rituals {
        let i = list_index(r.list);
        if w.sys.tool.rituals[i] {
            continue;
        }
        let in_time = r
            .by
            .is_none_or(|by| by.minutes_since_open() < 0 || m <= by.minutes_since_open() as u32);
        if in_time && list_done(w, content, r.list) {
            w.sys.tool.rituals[i] = true;
            let fx = r.effects.clone();
            xlogic::run(w, content, &fx, &XTarget::default());
            w.emit(SimEvent::RitualComplete { list: r.list });
            w.emit(SimEvent::Toast { key: r.toast_key() });
            super::rituals::on_ritual_complete(w, content, r);
        }
    }
}

/// Missed items' hands-on `extra_missed` of a list (Opening at its ritual deadline, Closing
/// at 06:00).
fn apply_missed(w: &mut World, content: &Content, list: ChecklistKind) {
    let missed: Vec<Vec<crate::content::handson::Fx>> = content
        .app
        .checklists
        .iter()
        .filter(|c| c.list == list && !c.extra_missed.is_empty())
        .filter(|c| !w.app.checklist.iter().any(|s| s.id == c.id && s.done))
        .map(|c| c.extra_missed.clone())
        .collect();
    for fx in missed {
        xlogic::run(w, content, &fx, &XTarget::default());
    }
}

/// Who may do a checklist item: Karolina only, or a helper holding the duty (the floor's
/// items are the dishes helper's too: wiping the tables before the doors open).
pub(crate) fn helper_may(content: &Content, id: &str, duty: crate::content::handson::Duty) -> bool {
    use crate::content::handson::Duty;
    content.app.checklists.iter().find(|c| c.id == id).is_some_and(|c| {
        matches!(c.owner(), ChecklistOwner::Either(d)
            if d == duty || (d == Duty::Floor && duty == Duty::Dishes && c.list == ChecklistKind::Opening))
    })
}

/// Closing-list items that wait for closing time ([`World::closing_time`]: the closing half
/// hour, or after last call in an empty café) — the till, the group head, the window light.
/// The others (the dishes, the fridge, the handover note) are open from 05:00.
pub const CLOSING_TIME_ITEMS: &[&str] = &["cash_up", "clean_machine", "light_off_at_six"];

/// Can a closing-list item be done now?
pub(crate) fn closing_item_open(w: &World, id: &str) -> bool {
    if w.phase != Phase::Night || w.in_prep() {
        return false;
    }
    if CLOSING_TIME_ITEMS.contains(&id) {
        w.closing_time()
    } else {
        w.in_closing() || w.minute() >= 420
    }
}

impl World {
    /// Is this checklist item open now? Opening items until they're done (before 05:00);
    /// closing items by [`CLOSING_TIME_ITEMS`] (closing time) or from 05:00.
    pub fn checklist_item_open(&self, content: &Content, id: &str) -> bool {
        let Some(def) = content.app.checklists.iter().find(|c| c.id == id) else {
            return false;
        };
        match def.list {
            ChecklistKind::Opening => self.phase == Phase::Night && !self.in_closing(),
            ChecklistKind::Closing => closing_item_open(self, id),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Tutorial `on_show` (tutorial.ron)
// ---------------------------------------------------------------------------------------------

/// Runs a tutorial step's `on_show` effects once (`asked` = the UI showed it).
pub(crate) fn tutorial_show(w: &mut World, content: &Content, id: &str, asked: bool) {
    if w.sys.tutorial_off || w.sys.tutorial_shown.contains(id) {
        return;
    }
    let Some(step) = content.handson.tutorial_step(id) else {
        return;
    };
    if !asked && (w.mode != Mode::Story || step.night != w.night) {
        return;
    }
    w.sys.tutorial_shown.insert(id.to_string());
    let fx = step.on_show.clone();
    if !fx.is_empty() {
        xlogic::run(w, content, &fx, &XTarget::default());
        w.emit(SimEvent::TutorialEffects { id: id.to_string() });
    }
}

/// Story nights: steps triggered at a clock time run their `on_show` then.
fn tutorial_minute(w: &mut World, content: &Content) {
    let m = w.minute() as i32;
    tutorial_at(w, content, m);
}

/// …at `m` minutes since 22:00 (negative in the preparation half hour, 21:30 = −30).
pub(crate) fn tutorial_at(w: &mut World, content: &Content, m: i32) {
    if w.mode != Mode::Story || w.sys.tutorial_off {
        return;
    }
    let due: Vec<String> = content
        .handson
        .tutorial
        .iter()
        .filter(|s| s.night == w.night && !s.on_show.is_empty())
        .filter(|s| match &s.trigger {
            crate::content::handson::TutTrigger::At(t) => t.minutes_since_open() == m,
            _ => false,
        })
        .map(|s| s.id.clone())
        .collect();
    for id in due {
        tutorial_show(w, content, &id, false);
    }
}

// ---------------------------------------------------------------------------------------------
// Night flow
// ---------------------------------------------------------------------------------------------

pub(crate) fn on_night_start(w: &mut World, content: &Content) {
    let closed = std::mem::take(&mut w.sys.tool.closed_properly);
    w.sys.tool.rituals = [false; 2];
    w.sys.tool.rush_at = None;
    w.sys.tool.hour_arrivals = [0; 8];
    w.sys.tool.unmet.clear();
    w.sys.tool.loved = None;
    w.sys.tool.low_stock_sent = false;
    w.sys.tool.suggestions.clear();
    w.sys.tool.picks.clear();
    // (reminders set by day — "remind me before Radek's train" — are for tonight: they're
    // cleared when the night ends)
    w.sys.tool.covers.clear();
    w.sys.tool.drones.clear();
    w.sys.tool.trust_start = w.story.trust.clone();
    let _ = closed;
    // last night's handover notes act now
    let notes = std::mem::take(&mut w.sys.tool.next_notes);
    if !notes.is_empty() {
        let mut ids = Vec::new();
        for (id, target) in notes {
            if let Some(o) = content.app.handover_options.iter().find(|o| o.id == id) {
                let fx = o.next_night.clone();
                xlogic::run(w, content, &fx, &target);
                ids.push(id);
            }
        }
        w.emit(SimEvent::HandoverApplied { ids });
    }
    // last week's tip split: the crew feels it
    if let Some((mood, _, week)) = w.sys.tool.split_effect
        && week == week_of(w.night)
    {
        for s in w.staff.iter_mut().filter(|s| !s.owner) {
            s.mood = (s.mood + mood * 0.5).min(100.0);
        }
    }
    // bad weather coming: the bot warns (and offers the umbrella stand)
    if matches!(
        w.weather,
        crate::content::Weather::Downpour | crate::content::Weather::Storm
    ) {
        let ctx = EffectCtx::default();
        app::bot_with(
            w,
            content,
            "weather_alert",
            &ctx,
            vec![("time".into(), "22:00".into())],
        );
    }
}

pub(crate) fn on_minute(w: &mut World, content: &Content) {
    tutorial_minute(w, content);
    let m = w.minute();
    // opening ritual deadline
    for r in &content.app.rituals {
        if r.list == ChecklistKind::Opening
            && let Some(by) = r.by
            && by.minutes_since_open() >= 0
            && m == by.minutes_since_open() as u32 + 1
        {
            apply_missed(w, content, ChecklistKind::Opening);
        }
    }
    check_rituals(w, content);
    // day-phase bot buttons that asked for a moment
    if m == 5 {
        let ids = std::mem::take(&mut w.sys.tool.moments_tonight);
        for id in ids {
            super::moments::spawn(w, content, &id, XTarget::default(), true);
        }
    }
    // reminders
    let due: Vec<Reminder> = w
        .sys
        .tool
        .reminders
        .iter()
        .filter(|r| r.minute == m)
        .cloned()
        .collect();
    for r in due {
        w.emit(SimEvent::Reminder {
            key: r.key,
            args: r.args,
        });
    }
    // drones
    if w.sys.tool.drones.contains(&m) {
        w.sys.tool.drones.retain(|x| *x != m);
        w.econ.supplies += 600;
        w.emit(SimEvent::Toast {
            key: "app.bot.low_stock.drone_arrived".into(),
        });
    }
    // "we're out of X" (the stock wasn't checked)
    if let Some((at, minutes)) = w.sys.fx.stockout
        && at == m
    {
        w.sys.fx.stockout = None;
        let items: Vec<String> = w.menu_available(content);
        if !items.is_empty() {
            let item = items[w.rng.index(items.len())].clone();
            w.mods.push(Modifier {
                kind: ModKind::MenuUnavailable(item.clone()),
                mul: 0.0,
                until: Some(m + minutes),
                event: None,
            });
            w.emit(SimEvent::Toast {
                key: "app.checklist.check_stock.out".into(),
            });
        }
    }
    // helpers wait at their stations before a train
    if w.sys.fx.helpers_ready.contains(&m) {
        w.mods.push(Modifier {
            kind: ModKind::WorkSpeed(None),
            mul: 1.15,
            until: Some(m + 30),
            event: None,
        });
    }
    // a late helper arrived: the colleague's cover ends
    let covers = w.sys.tool.covers.clone();
    for (late, duty, cover) in covers {
        if w.staff_by_id(late).is_some_and(|s| s.on_duty()) {
            w.sys.tool.covers.retain(|c| c.0 != late);
            if let Some(s) = w.staff.iter_mut().find(|s| s.id == cover) {
                s.duties.retain(|d| *d != duty);
                super::duties::sync_priorities(s);
            }
        }
    }
    // the radio fades
    if w.sys.fx.radio.as_ref().is_some_and(|r| r.2 <= m) {
        w.sys.fx.radio = None;
    }
    // low stock: once a night
    // (counted by hand: she warns while there is still time to order — `StockWatch`)
    let low = if w.sys.fx.stock_watch { 700 } else { 400 };
    if !w.sys.tool.low_stock_sent && w.econ.supplies < low && m < 420 {
        w.sys.tool.low_stock_sent = true;
        let item = w
            .econ
            .sold
            .iter()
            .max_by_key(|(_, n)| **n)
            .map(|(k, _)| k.clone())
            .unwrap_or_default();
        let ctx = EffectCtx::default();
        app::bot_with(
            w,
            content,
            "low_stock",
            &ctx,
            vec![
                ("item".into(), item),
                ("n".into(), (w.econ.supplies / 25).max(1).to_string()),
            ],
        );
    }
    // the handover suggestions are ready from 05:00
    if m == 420 {
        w.sys.tool.suggestions = suggestions(w, content);
    }
}

/// Each minute of the closing half hour (06:00–06:30): the closing ritual can complete, the
/// handover suggestions stay fresh.
pub(crate) fn on_closing_minute(w: &mut World, content: &Content) {
    check_rituals(w, content);
    if w.sys.tool.suggestions.is_empty() {
        w.sys.tool.suggestions = suggestions(w, content);
    }
}

pub(crate) fn note_arrival_hour(w: &mut World) {
    let h = (w.minute() / 60).min(7) as usize;
    w.sys.tool.hour_arrivals[h] += 1;
}

/// 06:00: closing extras, the closing ritual, handover notes for tomorrow.
pub(crate) fn on_night_end(w: &mut World, content: &Content) {
    check_rituals(w, content);
    apply_missed(w, content, ChecklistKind::Closing);
    w.sys.tool.closed_properly = w.sys.tool.rituals[1];
    if w.sys.tool.suggestions.is_empty() {
        w.sys.tool.suggestions = suggestions(w, content);
    }
    let picks = std::mem::take(&mut w.sys.tool.picks);
    let mut notes = Vec::new();
    for id in picks.iter().take(2) {
        if let Some(s) = w.sys.tool.suggestions.iter().find(|s| &s.id == id) {
            notes.push((s.id.clone(), s.target.clone()));
        }
    }
    w.sys.tool.next_notes = notes;
    // tonight's reminders and the reservations nobody came for end with the night
    w.sys.tool.reminders.clear();
    w.sys.tool.reserved.clear();
}

// ---------------------------------------------------------------------------------------------
// Handover (předávka)
// ---------------------------------------------------------------------------------------------

/// Sim-generated handover suggestions right now.
pub(crate) fn suggestions(w: &World, content: &Content) -> Vec<HandoverSuggestion> {
    let mut out = Vec::new();
    for o in &content.app.handover_options {
        if !xlogic::holds(w, content, &o.when) {
            continue;
        }
        let mut t = XTarget::default();
        let mut args: Vec<(String, String)> = Vec::new();
        let ok = match o.subject {
            HandoverSubject::None => true,
            HandoverSubject::LowItem => {
                let target = w.econ.auto_restock.unwrap_or(3000);
                let item = w
                    .econ
                    .sold
                    .iter()
                    .max_by_key(|(_, n)| **n)
                    .map(|(k, _)| k.clone());
                match item {
                    Some(item) if w.econ.supplies * 2 < target => {
                        args.push(("item".into(), item.clone()));
                        t.item = Some(item);
                        true
                    }
                    _ => false,
                }
            }
            HandoverSubject::Machine => {
                let m = w
                    .furniture
                    .iter()
                    .filter(|p| {
                        content
                            .furniture_def(&p.def)
                            .is_some_and(|d| d.has_tag("station:coffee"))
                    })
                    .max_by(|a, b| a.wear.total_cmp(&b.wear));
                m.map(|p| t.furniture = Some(p.uid)).is_some()
            }
            HandoverSubject::Regular => {
                let r = w
                    .story
                    .regulars
                    .iter()
                    .filter(|(_, s)| s.visits.last().is_some_and(|v| v.night == w.night))
                    .map(|(id, _)| id.clone())
                    .find(|id| content.regular(id).is_some());
                match r {
                    Some(r) => {
                        args.push(("name".into(), format!("regular.{r}.name")));
                        t.regular = Some(r);
                        true
                    }
                    None => false,
                }
            }
            HandoverSubject::Train => {
                let Some(at) = w.sys.tool.rush_at else {
                    continue;
                };
                let train = super::logic::train_minutes(content)
                    .into_iter()
                    .filter(|tm| *tm <= at + 5)
                    .max();
                match train {
                    Some(tm) => {
                        args.push((
                            "time".into(),
                            ClockTime::from_minutes_since_open(tm).to_string(),
                        ));
                        t.train_minute = Some(tm);
                        true
                    }
                    None => false,
                }
            }
            HandoverSubject::Table => {
                let tb = w
                    .furniture
                    .iter()
                    .filter(|p| {
                        content
                            .furniture_def(&p.def)
                            .is_some_and(|d| d.has_tag("table"))
                    })
                    .max_by(|a, b| a.wear.total_cmp(&b.wear));
                tb.map(|p| t.furniture = Some(p.uid)).is_some()
            }
            HandoverSubject::Crew => {
                let sat = if w.satisfaction_n > 0 {
                    w.satisfaction_sum / w.satisfaction_n as f32
                } else {
                    0.0
                };
                sat >= 70.0
            }
        };
        if ok {
            out.push(HandoverSuggestion {
                id: o.id.clone(),
                args,
                target: t,
            });
        }
    }
    out
}

/// `Command::PickHandover`.
pub(crate) fn pick_handover(
    w: &mut World,
    content: &Content,
    picks: Vec<String>,
) -> Result<(), CommandError> {
    if picks.len() > 2 {
        return Err(CommandError::Invalid("at most two notes".into()));
    }
    if w.sys.tool.suggestions.is_empty() {
        w.sys.tool.suggestions = suggestions(w, content);
    }
    for p in &picks {
        if !w.sys.tool.suggestions.iter().any(|s| &s.id == p) {
            return Err(CommandError::UnknownId(p.clone()));
        }
    }
    w.sys.tool.picks = picks;
    Ok(())
}

/// `RestockFirst`: the subject item (or the whole stock) is topped up at once.
pub(crate) fn restock_first(w: &mut World, content: &Content, item: Option<&str>) {
    let target = w.econ.auto_restock.unwrap_or(3000);
    let need = (target - w.econ.supplies).max(0);
    let buy = if item.is_some() { need.max(300) } else { need };
    if buy > 0 && w.econ.money >= buy {
        let _ = super::economy::buy_supplies(w, buy);
    }
    let _ = content;
}

// ---------------------------------------------------------------------------------------------
// Bot buttons
// ---------------------------------------------------------------------------------------------

fn subject_staff(w: &World, msg: &BotMsg) -> Option<u32> {
    let name = msg
        .args
        .iter()
        .find(|a| a.0 == "name")
        .map(|a| a.1.clone())?;
    w.staff
        .iter()
        .find(|s| !s.owner && s.name == name)
        .map(|s| s.id)
}

fn arg<'a>(msg: &'a BotMsg, k: &str) -> Option<&'a str> {
    msg.args.iter().find(|a| a.0 == k).map(|a| a.1.as_str())
}

/// A bot button's handler acts in the world (§13.4).
pub(crate) fn handle(
    w: &mut World,
    content: &Content,
    msg: &BotMsg,
    h: &BotHandler,
) -> Result<(), CommandError> {
    match h {
        BotHandler::CoverSelf => {
            // her rail carries the late helper's duty tasks until they arrive (nobody else
            // holds them): a thank-you
            if let Some(id) = subject_staff(w, msg)
                && let Some(i) = w.staff.iter().position(|s| s.id == id)
            {
                super::logic::change_mood(w, content, i, 2.0, false);
            }
            Ok(())
        }
        BotHandler::CoverOther => {
            let Some(late) = subject_staff(w, msg) else {
                return Err(CommandError::Invalid("nobody is late".into()));
            };
            let mut covered = false;
            let duties = w
                .staff_by_id(late)
                .map(|s| s.duties.clone())
                .unwrap_or_default();
            let night = w.night;
            for d in duties {
                let pick = w
                    .staff
                    .iter()
                    .filter(|s| !s.owner && s.id != late && s.on_duty() && !s.duties.contains(&d))
                    .filter(|s| {
                        (s.duties.len() as u32) < super::duties::capacity(content, s, night, None)
                    })
                    .max_by(|a, b| a.skill(d.job(), night).total_cmp(&b.skill(d.job(), night)))
                    .map(|s| s.id);
                if let Some(c) = pick
                    && let Some(i) = w.staff.iter().position(|s| s.id == c)
                {
                    w.staff[i].duties.push(d);
                    super::duties::sync_priorities(&mut w.staff[i]);
                    super::logic::change_mood(w, content, i, -2.0, false);
                    w.sys.tool.covers.push((late, d, c));
                    covered = true;
                    break;
                }
            }
            if !covered {
                // nobody on shift has room for it: the message stays open, she can cover
                return Err(CommandError::Invalid("nobody free to cover".into()));
            }
            Ok(())
        }
        BotHandler::CallIn => {
            if w.phase != Phase::Night {
                return Err(CommandError::Invalid("night only".into()));
            }
            let wd = w.weekday as usize;
            let m = w.minute();
            let rozpis = &content.handson.tuning.rozpis;
            let pick = w
                .staff
                .iter()
                .filter(|s| !s.owner && s.status == StaffStatus::OffDuty)
                .filter(|s| {
                    !super::rozpis::prefs_of(w, s.id).iter().any(|p| {
                        p.strength == crate::content::handson::Strength::Need
                            && matches!(&p.kind,
                                crate::content::handson::PrefKind::DayOff(d)
                                    if crate::content::Weekday::ALL[wd] == *d)
                    })
                })
                .map(|s| s.id)
                .next();
            let Some(id) = pick else {
                return Err(CommandError::Invalid("nobody can come in".into()));
            };
            let min_nights = super::rozpis::prefs_of(w, id)
                .iter()
                .any(|p| matches!(p.kind, crate::content::handson::PrefKind::MinNights(_)));
            if let Some(s) = w.staff.iter_mut().find(|s| s.id == id) {
                s.shift = Shift::Late;
                s.leave_at = NIGHT_MINUTES;
                s.status = StaffStatus::Expected {
                    minute: (m + 60).min(NIGHT_MINUTES - 30),
                };
            }
            if !min_nights {
                let week = week_of(w.night);
                w.sys
                    .rozpis
                    .bonus
                    .entry(id)
                    .or_default()
                    .push((-rozpis.need_penalty * 0.32, week));
            }
            super::rozpis::score(w, content);
            Ok(())
        }
        BotHandler::ReserveSeat => {
            let Some(r) = arg(msg, "regular").map(str::to_string) else {
                return Err(CommandError::Invalid("no regular to reserve for".into()));
            };
            // their favourite kind of seat (window, socket…) if there is one, else any
            // table seat nobody else has reserved
            let prefer = content.regular(&r).and_then(|d| d.seat.clone());
            let taken: Vec<u32> = w.sys.tool.reserved.values().copied().collect();
            let seats = super::guests::seats(w, content);
            let ok = |s: &&super::guests::SeatInfo| {
                s.free && s.table.is_some() && !taken.contains(&s.uid)
            };
            let seat = prefer
                .as_deref()
                .and_then(|tag| {
                    seats
                        .iter()
                        .filter(ok)
                        .find(|s| super::guests::seat_has_tag(w, content, s, tag))
                })
                .or_else(|| seats.iter().find(ok))
                .map(|s| s.uid);
            let Some(seat) = seat else {
                return Err(CommandError::Invalid("no free seat".into()));
            };
            w.sys.tool.reserved.insert(r, seat);
            Ok(())
        }
        BotHandler::RemindBefore(min) => {
            let at = arg(msg, "minute").and_then(|x| x.parse::<u32>().ok());
            if let Some(at) = at {
                let mut args = msg.args.clone();
                args.retain(|a| a.0 != "minute");
                w.sys.tool.reminders.push(Reminder {
                    minute: at.saturating_sub(*min),
                    key: format!("app.bot.{}.reminder", msg.template),
                    args,
                });
            }
            Ok(())
        }
        BotHandler::OrderDrone { minutes } => {
            let dock = w.furniture.iter().any(|p| {
                content
                    .furniture_def(&p.def)
                    .is_some_and(|d| d.has_tag("drone"))
            });
            if !dock {
                return Err(CommandError::Invalid("no drone dock".into()));
            }
            let cost = 300;
            if w.econ.money < cost {
                return Err(CommandError::NotEnoughMoney {
                    need: cost,
                    have: w.econ.money,
                });
            }
            super::logic::add_money(w, -cost);
            let at = w.minute() + minutes;
            w.sys.tool.drones.push(at);
            Ok(())
        }
        BotHandler::MenuOff => {
            if let Some(item) = arg(msg, "item").map(str::to_string)
                && content.menu_item(&item).is_some()
            {
                w.mods.push(Modifier {
                    kind: ModKind::MenuUnavailable(item),
                    mul: 0.0,
                    until: None,
                    event: None,
                });
            }
            Ok(())
        }
        BotHandler::ApplyMoment(id) => {
            if w.phase == Phase::Night {
                if !super::moments::spawn(w, content, id, XTarget::default(), true)
                    && let Some(def) = content.handson.moment(id)
                {
                    // can't anchor now: its first option applies (never a dead button)
                    if let Some(o) = def.options.first() {
                        let fx = o.effects.clone();
                        xlogic::run(w, content, &fx, &XTarget::default());
                    }
                }
            } else {
                w.sys.tool.moments_tonight.push(id.clone());
            }
            Ok(())
        }
        BotHandler::Camera(t) => {
            w.sys.tool.last_camera = Some(*t);
            w.emit(SimEvent::CameraHint {
                target: *t,
                tile: None,
            });
            Ok(())
        }
        BotHandler::Open(link) => {
            w.emit(SimEvent::OpenApp { link: *link });
            Ok(())
        }
    }
}

/// Day start: a regular likely tonight → `regular_tonight` / `regular_to_go` (one message).
pub(crate) fn on_day_start(w: &mut World, content: &Content) {
    if w.mode != Mode::Story || w.night < 2 {
        return;
    }
    // payroll weeks waiting a day: they book themselves
    let weeks: Vec<u32> = w.app.payroll.iter().map(|p| p.week).collect();
    for week in weeks {
        if !w.sys.tool.payroll_sent.contains(&week) && w.night > week * 7 + 1 {
            let _ = send_payroll(w, content, week);
        }
    }
    let regs: Vec<(String, u32)> = content
        .regulars
        .iter()
        .filter(|r| r.first_night <= w.night && r.id != "guest_333" && r.look_preset.is_none())
        .filter_map(|r| {
            let v = r.visits.first()?;
            let m = v.from.minutes_since_open();
            (m >= 0 && v.chance >= 0.5).then(|| (r.id.clone(), m as u32))
        })
        .collect();
    if regs.is_empty() {
        return;
    }
    let (id, minute) = regs[(w.night as usize) % regs.len()].clone();
    let time = ClockTime::from_minutes_since_open(minute).to_string();
    let to_go = id == "radek";
    let ctx = EffectCtx::default();
    app::bot_with(
        w,
        content,
        if to_go {
            "regular_to_go"
        } else {
            "regular_tonight"
        },
        &ctx,
        vec![
            ("name".into(), format!("regular.{id}.name")),
            ("time".into(), time),
            ("regular".into(), id),
            ("minute".into(), minute.to_string()),
        ],
    );
}

/// A regular walked in: their reserved seat (trust +1, the seat-free streak counts).
pub(crate) fn on_regular_arrived(w: &mut World, regular: &str) {
    if w.sys.tool.reserved.remove(regular).is_some() {
        let t = w.story.trust.entry(regular.to_string()).or_insert(0.0);
        *t = (*t + 1.0).min(100.0);
        if let Some(v) = w
            .story
            .regulars
            .get_mut(regular)
            .and_then(|s| s.visits.last_mut())
        {
            v.seat_free = true;
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Digest insights (§13.6)
// ---------------------------------------------------------------------------------------------

pub(crate) fn insights(w: &World, content: &Content) -> Vec<Insight> {
    let mut out = Vec::new();
    let has = |id: &str| content.app.insights.iter().find(|i| i.id == id);
    let push = |out: &mut Vec<Insight>, id: &str, args: Vec<(String, String)>| {
        if let Some(d) = has(id) {
            out.push(Insight {
                key: d.key(),
                args,
                link: d.link,
            });
        }
    };
    let t = &w.hands.tally;
    // the busiest hour
    if let Some((h, n)) = w
        .sys
        .tool
        .hour_arrivals
        .iter()
        .enumerate()
        .max_by_key(|(i, n)| (**n, std::cmp::Reverse(*i)))
        && *n >= 3
    {
        push(
            &mut out,
            "busiest_hour",
            vec![
                (
                    "time".into(),
                    ClockTime::from_minutes_since_open(h as u32 * 60).to_string(),
                ),
                ("weekday".into(), w.weekday().id().to_string()),
            ],
        );
    }
    if let Some((r, recipe, _)) = &w.sys.tool.loved {
        push(
            &mut out,
            "wish_loved",
            vec![
                ("name".into(), format!("regular.{r}.name")),
                ("recipe".into(), format!("recipe.{recipe}.name")),
            ],
        );
    } else if t.wishes_served >= 3 {
        push(
            &mut out,
            "wishes",
            vec![
                ("n".into(), t.wishes_served.to_string()),
                ("perfect".into(), t.tiers[0].to_string()),
            ],
        );
    }
    if let Some((taste, n)) = w.sys.tool.unmet.iter().max_by_key(|(_, n)| **n)
        && *n >= 2
    {
        push(
            &mut out,
            "unmet_taste",
            vec![
                ("n".into(), n.to_string()),
                ("taste".into(), format!("taste.{}", taste.id())),
            ],
        );
    }
    let avg = super::atmos::night_avg(w);
    let def = &content.handson.decor.coziness;
    let weakest = crate::content::handson::CozyComponent::ALL[w.sys.atmos.coziness.weakest];
    push(
        &mut out,
        "coziness",
        vec![
            ("level".into(), format!("atmo.level.{}", def.level(avg))),
            ("weakest".into(), weakest.tip_key()),
        ],
    );
    let credits = super::duties::helper_credits(w);
    if !credits.is_empty() {
        let summary = credits
            .iter()
            .take(2)
            .map(|(n, d, k)| format!("{n}:{d}:{k}"))
            .collect::<Vec<_>>()
            .join(",");
        push(&mut out, "helpers", vec![("summary".into(), summary)]);
    }
    push(
        &mut out,
        "tips",
        vec![
            ("tips".into(), w.tonight.tips.to_string()),
            ("streak".into(), t.best_rhythm.to_string()),
        ],
    );
    let gain = w
        .story
        .trust
        .iter()
        .map(|(r, v)| (r, v - w.sys.tool.trust_start.get(r).copied().unwrap_or(0.0)))
        .filter(|(_, d)| *d >= 3.0)
        .max_by(|a, b| a.1.total_cmp(&b.1));
    if let Some((r, _)) = gain {
        push(
            &mut out,
            "trust",
            vec![("name".into(), format!("regular.{r}.name"))],
        );
    }
    out.truncate(4);
    out
}

// ---------------------------------------------------------------------------------------------
// Payroll (§13.7)
// ---------------------------------------------------------------------------------------------

pub(crate) fn tip_split(w: &World, content: &Content) -> TipSplit {
    w.sys.tool.tip_split.unwrap_or_else(|| {
        content
            .app
            .payroll_rules
            .as_ref()
            .and_then(|r| r.default_split())
            .map(|s| s.split)
            .unwrap_or(TipSplit::Half)
    })
}

/// `Command::SendPayroll`: books the week — the tip split leaves the café's money, the crew
/// feels it next week; the star of the week gets the bonus.
pub(crate) fn send_payroll(
    w: &mut World,
    content: &Content,
    week: u32,
) -> Result<(), CommandError> {
    if w.sys.tool.payroll_sent.contains(&week) {
        return Err(CommandError::Invalid("already sent".into()));
    }
    let Some(p) = w.app.payroll.iter().find(|p| p.week == week).cloned() else {
        return Err(CommandError::UnknownId(week.to_string()));
    };
    let Some(rules) = content.app.payroll_rules.clone() else {
        return Err(CommandError::Invalid("no payroll rules".into()));
    };
    let split = tip_split(w, content);
    let opt = rules.splits.iter().find(|s| s.split == split);
    let share = opt.map(|s| s.crew_share).unwrap_or(0.5);
    let tips_to_crew = (p.tips as f32 * share).round() as i64;
    if tips_to_crew > 0 {
        w.econ.money -= tips_to_crew;
    }
    if let Some(o) = opt {
        w.sys.tool.split_effect = Some((o.crew_mood, o.crew_wellbeing, week + 1));
        if o.crew_wellbeing != 0.0 {
            let ids: Vec<u32> = w.staff.iter().filter(|s| !s.owner).map(|s| s.id).collect();
            for id in ids {
                w.sys
                    .rozpis
                    .bonus
                    .entry(id)
                    .or_default()
                    .push((o.crew_wellbeing, week + 1));
            }
        }
    }
    let mut total = p.total + tips_to_crew;
    if let Some((bw, staff)) = w.sys.tool.bonus
        && bw == week
    {
        let b = rules.star_bonus as i64;
        w.econ.money -= b;
        total += b;
        if let Some(i) = w.staff.iter().position(|s| s.id == staff) {
            super::logic::change_mood(w, content, i, rules.star_mood, false);
        }
        w.emit(SimEvent::Toast {
            key: "app.payroll.star_card".into(),
        });
    }
    w.sys.tool.payroll_sent.insert(week);
    w.emit(SimEvent::PayrollSent {
        week,
        total,
        tips_to_crew,
    });
    Ok(())
}

/// The newest payroll week not booked yet.
fn open_week(w: &World) -> Option<u32> {
    w.app
        .payroll
        .iter()
        .rev()
        .map(|p| p.week)
        .find(|wk| !w.sys.tool.payroll_sent.contains(wk))
}

pub(crate) fn pay_bonus(w: &mut World, staff: u32) -> Result<(), CommandError> {
    if !w.staff.iter().any(|s| s.id == staff && !s.owner) {
        return Err(CommandError::UnknownId(staff.to_string()));
    }
    let week = open_week(w).ok_or_else(|| CommandError::Invalid("no open week".into()))?;
    w.sys.tool.bonus = Some((week, staff));
    Ok(())
}

pub(crate) fn grant_toil(
    w: &mut World,
    content: &Content,
    staff: u32,
    weekday: u8,
) -> Result<(), CommandError> {
    let grant_wb = content
        .app
        .payroll_rules
        .as_ref()
        .map(|r| r.toil_grant_wellbeing)
        .unwrap_or(10.0);
    let Some(i) = w.staff.iter().position(|s| s.id == staff && !s.owner) else {
        return Err(CommandError::UnknownId(staff.to_string()));
    };
    if w.staff[i].toil_hours < 4.0 {
        return Err(CommandError::Invalid("no náhradní volno owed".into()));
    }
    let ahead = (weekday as u32 + 7 - w.weekday as u32) % 7;
    let night = w.night + if ahead == 0 { 7 } else { ahead };
    let s = &mut w.staff[i];
    s.toil_hours = (s.toil_hours - 8.0).max(0.0);
    s.leave_nights.push((night, LeaveKind::Toil));
    let week = week_of(night);
    w.sys
        .rozpis
        .bonus
        .entry(staff)
        .or_default()
        .push((grant_wb, week));
    app::crew_changed(w, content);
    Ok(())
}

/// Tallies from a served wish (unmet tastes, the night's loved wish).
pub(crate) fn note_wish(
    w: &mut World,
    content: &Content,
    wish: Option<&str>,
    recipe: &str,
    regular: Option<&str>,
    m: f32,
) {
    let Some(wd) = wish.and_then(|x| content.handson.wish(x)) else {
        return;
    };
    let tiers = &content.handson.tuning.match_tiers;
    if m < tiers.good
        && let Some(tags) = w.recipe_def(content, recipe).map(|r| r.tags.clone())
    {
        for tag in &wd.want {
            if !tags.contains(tag) {
                *w.sys.tool.unmet.entry(*tag).or_insert(0) += 1;
            }
        }
    }
    if let Some(reg) = regular
        && w.sys.tool.loved.as_ref().is_none_or(|l| m > l.2)
        && m >= tiers.perfect
    {
        w.sys.tool.loved = Some((reg.to_string(), recipe.to_string(), m));
    }
}

/// Highlights from a serve (a perfect one for a regular, a beautiful latte).
#[allow(clippy::too_many_arguments)]
pub(crate) fn serve_highlights(
    w: &mut World,
    content: &Content,
    regular: Option<&str>,
    recipe: &str,
    m: f32,
    q: f32,
    by_hand: bool,
    guest: u32,
) {
    let tiers = &content.handson.tuning.match_tiers;
    let tile = w.guests.iter().find(|g| g.id == guest).map(|g| g.tile());
    if let Some(r) = regular
        && m >= tiers.perfect
    {
        super::journal::capture(
            w,
            content,
            HighlightKind::PerfectServe,
            None,
            vec![("name".into(), format!("regular.{r}.name"))],
            tile,
        );
    }
    let latte = content
        .handson
        .recipe(recipe)
        .filter(|r| r.minigame == crate::content::handson::MiniGameKind::LatteArt);
    if by_hand && q >= content.handson.journal.beautiful_latte_quality && latte.is_some() {
        let pattern = latte
            .and_then(|r| r.variants.first())
            .map(|v| format!("latte.{v}"))
            .unwrap_or_default();
        let name = regular
            .map(|r| format!("regular.{r}.name"))
            .unwrap_or_default();
        super::journal::capture(
            w,
            content,
            HighlightKind::BeautifulLatte,
            None,
            vec![("pattern".into(), pattern), ("name".into(), name)],
            tile,
        );
    }
}
