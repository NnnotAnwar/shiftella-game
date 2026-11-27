//! The in-game Shiftella app: weekly rozpis (draft/published), swap & time-off requests,
//! checklists, handover notes, announcements, @shiftella_bot inbox, digest, payroll basis.
//! Never clock-in/out through the bot.

use std::collections::BTreeMap;

use super::logic;
use super::types::*;
use super::world::*;
use crate::content::{BotKind, Content};

pub(crate) fn week_of(night: u32) -> u32 {
    (night.max(1) - 1) / 7 + 1
}

fn staff_name(w: &World, id: Option<u32>) -> String {
    id.and_then(|id| w.staff_by_id(id))
        .map(|s| s.name.clone())
        .unwrap_or_default()
}

/// Sends a bot message from a template (placeholders filled from context).
pub(crate) fn bot(w: &mut World, content: &Content, template: &str, ctx: &EffectCtx) {
    bot_with(w, content, template, ctx, Vec::new());
}

pub(crate) fn bot_with(
    w: &mut World,
    content: &Content,
    template: &str,
    ctx: &EffectCtx,
    extra: Vec<(String, String)>,
) {
    bot_full(w, content, template, ctx, extra, None);
}

/// A bot message about a request the sim already created (rozpis prefs, life events).
pub(crate) fn bot_request(
    w: &mut World,
    content: &Content,
    template: &str,
    ctx: &EffectCtx,
    request: u32,
    extra: Vec<(String, String)>,
) {
    bot_full(w, content, template, ctx, extra, Some(request));
}

fn bot_full(
    w: &mut World,
    content: &Content,
    template: &str,
    ctx: &EffectCtx,
    extra: Vec<(String, String)>,
    preset: Option<u32>,
) {
    let Some(t) = content.app.bot_template(template) else {
        return;
    };
    let id = w.alloc_id();
    let mut request = preset;
    let mut subject = ctx.staff;
    // requests created by the message
    match t.kind {
        _ if preset.is_some() => {}
        BotKind::SwapRequest => {
            let ids: Vec<u32> = w.staff.iter().filter(|s| !s.owner).map(|s| s.id).collect();
            if ids.len() >= 2 {
                let a = subject.unwrap_or(ids[w.rng.index(ids.len())]);
                let others: Vec<u32> = ids.iter().copied().filter(|x| *x != a).collect();
                let b = others[w.rng.index(others.len())];
                let wd = ((w.weekday as u32 + 1 + w.rng.range_u32(0, 5)) % 7) as u8;
                let rid = w.alloc_id();
                w.app.swaps.push(SwapRequest {
                    id: rid,
                    from: a,
                    to: b,
                    weekday: wd,
                    status: RequestStatus::Pending,
                });
                request = Some(rid);
                subject = Some(a);
                w.emit(SimEvent::RequestCreated {
                    id: rid,
                    kind: crate::content::handson::RequestKind::Swap,
                    staff: a,
                });
            }
        }
        BotKind::TimeOffRequest => {
            let ids: Vec<u32> = w.staff.iter().filter(|s| !s.owner).map(|s| s.id).collect();
            if !ids.is_empty() {
                let a = subject.unwrap_or(ids[w.rng.index(ids.len())]);
                let toil = template.contains("toil");
                let rid = w.alloc_id();
                let hours = if toil {
                    w.staff_by_id(a)
                        .map(|s| s.toil_hours)
                        .unwrap_or(0.0)
                        .max(4.0)
                } else {
                    8.0
                };
                w.app.time_off.push(TimeOffRequest {
                    id: rid,
                    staff: a,
                    kind: if toil {
                        LeaveKind::Toil
                    } else {
                        LeaveKind::Vacation
                    },
                    night: w.night + 1,
                    hours,
                    status: RequestStatus::Pending,
                });
                request = Some(rid);
                subject = Some(a);
                w.emit(SimEvent::RequestCreated {
                    id: rid,
                    kind: if toil {
                        crate::content::handson::RequestKind::Toil
                    } else {
                        crate::content::handson::RequestKind::TimeOff
                    },
                    staff: a,
                });
            }
        }
        _ => {}
    }
    let mut args: Vec<(String, String)> = Vec::new();
    for ph in &t.placeholders {
        if extra.iter().any(|(k, _)| k == ph) {
            continue;
        }
        let v = match ph.as_str() {
            "name" => staff_name(w, subject),
            "night" => w.night.to_string(),
            "week" => week_of(w.night).to_string(),
            "debt" => w.econ.debt.to_string(),
            "amount" => content
                .story
                .payments
                .iter()
                .find(|p| p.night >= w.night)
                .map(|p| p.amount.to_string())
                .unwrap_or_default(),
            "guests" => w.tonight.guests_served.to_string(),
            "revenue" => w.tonight.revenue.to_string(),
            "other" => request
                .and_then(|r| w.app.swaps.iter().find(|s| s.id == r))
                .map(|s| staff_name(w, Some(s.to)))
                .unwrap_or_default(),
            "shift" => request
                .and_then(|r| w.app.swaps.iter().find(|s| s.id == r))
                .map(|s| {
                    crate::content::Weekday::ALL[s.weekday as usize % 7]
                        .id()
                        .to_string()
                })
                .unwrap_or_default(),
            "dates" => (w.night + 1).to_string(),
            "hours" => request
                .and_then(|r| w.app.time_off.iter().find(|x| x.id == r))
                .map(|x| format!("{:.0}", x.hours))
                .unwrap_or_default(),
            "minutes" => String::new(),
            "title" => String::new(),
            "train" => String::new(),
            "total" => String::new(),
            _ => String::new(),
        };
        args.push((ph.clone(), v));
    }
    args.extend(extra);
    let minute = (w.phase == Phase::Night).then(|| w.minute());
    w.app.inbox.push(BotMsg {
        id,
        template: template.to_string(),
        args: args.clone(),
        night: w.night,
        minute,
        answered: None,
        request,
    });
    if w.app.inbox.len() > 200 {
        w.app.inbox.remove(0);
    }
    w.emit(SimEvent::BotMessage {
        msg: id,
        template: template.to_string(),
        key: format!("app.bot.{template}"),
        args,
    });
}

/// A button on a bot message.
pub(crate) fn bot_action(
    w: &mut World,
    content: &Content,
    msg: u32,
    action: &str,
) -> Result<(), CommandError> {
    let Some(i) = w.app.inbox.iter().position(|m| m.id == msg) else {
        return Err(CommandError::UnknownId(msg.to_string()));
    };
    if w.app.inbox[i].answered.is_some() {
        return Err(CommandError::Invalid("already answered".into()));
    }
    let template = w.app.inbox[i].template.clone();
    let t = content
        .app
        .bot_template(&template)
        .ok_or_else(|| CommandError::UnknownId(template.clone()))?;
    let a = t
        .actions
        .iter()
        .find(|a| a.id == action)
        .ok_or_else(|| CommandError::UnknownId(action.to_string()))?;
    let handler = a.handler.clone();
    if let Some(h) = &handler {
        // a button that can't act now leaves the message open
        let msg_copy = w.app.inbox[i].clone();
        super::tool::handle(w, content, &msg_copy, h)?;
    }
    w.app.inbox[i].answered = Some(action.to_string());
    let request = w.app.inbox[i].request;
    let mut ctx = EffectCtx::default();
    logic::run(
        w,
        content,
        &EffectSrc::BotAction(template.clone(), action.to_string()),
        &a.effects,
        &mut ctx,
    );
    if let Some(r) = request
        && matches!(action, "approve" | "decline")
    {
        let approve = action == "approve";
        if w.app.swaps.iter().any(|s| s.id == r) {
            answer_swap(w, r, approve)?;
            answered(w, content, r, approve);
        } else if w.app.time_off.iter().any(|x| x.id == r) {
            answer_time_off(w, r, approve)?;
            crew_changed(w, content);
            answered(w, content, r, approve);
        } else if w.sys.rozpis.early_leave.iter().any(|x| x.id == r) {
            super::rozpis::answer_early_leave(w, content, r, approve)?;
        }
    }
    if action == "get_well" {
        // the sick colleague feels remembered
        if let Some(name) = w.app.inbox[i]
            .args
            .iter()
            .find(|a| a.0 == "name")
            .map(|a| a.1.clone())
            && let Some(s) = w.staff.iter_mut().find(|s| s.name == name)
        {
            s.mood = (s.mood + 5.0).min(100.0);
        }
    }
    Ok(())
}

/// The rozpis side of an answered swap / time-off request (§13.3).
pub(crate) fn answered(w: &mut World, content: &Content, id: u32, approve: bool) {
    let partner = w.app.swaps.iter().find(|s| s.id == id).map(|s| s.to);
    super::rozpis::on_answer(w, content, id, approve, partner);
}

pub(crate) fn answer_swap(w: &mut World, id: u32, approve: bool) -> Result<(), CommandError> {
    let Some(i) = w.app.swaps.iter().position(|s| s.id == id) else {
        return Err(CommandError::UnknownId(id.to_string()));
    };
    if w.app.swaps[i].status != RequestStatus::Pending {
        return Err(CommandError::Invalid("already answered".into()));
    }
    let s = w.app.swaps[i].clone();
    w.app.swaps[i].status = if approve {
        RequestStatus::Approved
    } else {
        RequestStatus::Declined
    };
    let wd = s.weekday as usize % 7;
    if approve {
        // a hand-made change: the app stops re-planning the rozpis on its own
        w.app.manual_rozpis = true;
        for sched in [&mut w.app.published, &mut w.app.draft] {
            let a = sched.get(&s.from).map(|x| x[wd]).unwrap_or_default();
            let b = sched.get(&s.to).map(|x| x[wd]).unwrap_or_default();
            if let Some(x) = sched.get_mut(&s.from) {
                x[wd] = b;
            }
            if let Some(x) = sched.get_mut(&s.to) {
                x[wd] = a;
            }
        }
    }
    if let Some(st) = w.staff.iter_mut().find(|x| x.id == s.from) {
        st.mood = (st.mood + if approve { 4.0 } else { -3.0 }).clamp(0.0, 100.0);
    }
    Ok(())
}

pub(crate) fn answer_time_off(w: &mut World, id: u32, approve: bool) -> Result<(), CommandError> {
    let Some(i) = w.app.time_off.iter().position(|s| s.id == id) else {
        return Err(CommandError::UnknownId(id.to_string()));
    };
    if w.app.time_off[i].status != RequestStatus::Pending {
        return Err(CommandError::Invalid("already answered".into()));
    }
    let r = w.app.time_off[i].clone();
    w.app.time_off[i].status = if approve {
        RequestStatus::Approved
    } else {
        RequestStatus::Declined
    };
    if let Some(st) = w.staff.iter_mut().find(|x| x.id == r.staff) {
        if approve {
            st.leave_nights.push((r.night, r.kind));
            match r.kind {
                LeaveKind::Vacation => st.vacation_days = st.vacation_days.saturating_sub(1),
                LeaveKind::Toil => st.toil_hours = (st.toil_hours - r.hours).max(0.0),
            }
            st.mood = (st.mood + 8.0).min(100.0);
        } else {
            st.mood = (st.mood - 6.0).max(0.0);
        }
    }
    Ok(())
}

/// Where a checklist item stands, as every screen shows it (the app, the ritual chip over its
/// furniture, Karolina's card): open → taken (queued / on the way) → done.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecklistMark {
    /// Nobody has it yet: a click queues it for Karolina.
    Open,
    /// Someone has it: Karolina (`karolina`; `doing` = it is her current act) or a helper on
    /// their way. A click on Karolina's cancels it.
    Taken {
        by: u32,
        karolina: bool,
        doing: bool,
    },
    /// Ticked: by whom and at which night minute.
    Done {
        by: Option<u32>,
        minute: Option<u32>,
    },
}

/// Is this act of Karolina's the checklist item `id` (the item itself, or the appliance chore
/// that ticks it)?
pub fn act_ticks(act: &super::hands::PlayerAct, id: &str) -> bool {
    match act {
        super::hands::PlayerAct::Checklist { id: i } => i == id,
        super::hands::PlayerAct::Use { action, .. } => action.checklist() == Some(id),
        _ => false,
    }
}

impl World {
    /// The state of checklist item `id` (`Open` for an unknown id or by day before the night).
    pub fn checklist_mark(&self, id: &str) -> ChecklistMark {
        let Some(c) = self.app.checklist.iter().find(|c| c.id == id) else {
            return ChecklistMark::Open;
        };
        if c.done {
            return ChecklistMark::Done {
                by: c.by,
                minute: c.minute,
            };
        }
        let me = super::player::karolina_idx(self).map(|i| self.staff[i].id);
        let slot = self.karolina_checklist_slot(id);
        match (c.claim, slot) {
            (_, Some(slot)) => ChecklistMark::Taken {
                by: me.unwrap_or(0),
                karolina: true,
                doing: slot == 0 && self.hands.player.current.is_some(),
            },
            (Some(by), None) if Some(by) != me => ChecklistMark::Taken {
                by,
                karolina: false,
                doing: false,
            },
            _ => ChecklistMark::Open,
        }
    }

    /// Where checklist item `id` sits in Karolina's acts (0 = her current act when she has
    /// one, then the queue — the order of the chips over her head), if she has it.
    pub fn karolina_checklist_slot(&self, id: &str) -> Option<usize> {
        let p = &self.hands.player;
        p.current
            .iter()
            .map(|c| &c.act)
            .chain(p.queue.iter())
            .position(|a| act_ticks(a, id))
    }
}

/// Satisfaction bonus for guests from opening checklist items done tonight.
pub(crate) fn checklist_bonus(w: &World) -> f32 {
    w.app.checklist.iter().filter(|c| c.done).count() as f32 * 0.6
}

pub(crate) fn tick_checklist(w: &mut World, content: &Content, id: &str, by: Option<u32>) {
    let Some(i) = w.app.checklist.iter().position(|c| c.id == id) else {
        return;
    };
    if w.app.checklist[i].done {
        return;
    }
    let minute = (w.phase == Phase::Night).then(|| w.minute());
    let c = &mut w.app.checklist[i];
    c.done = true;
    c.by = by;
    c.minute = minute;
    c.claim = None;
    w.tonight.checklist_done += 1;
    if let Some(def) = content.app.checklists.iter().find(|c| c.id == id) {
        let mut ctx = EffectCtx {
            staff: by,
            ..Default::default()
        };
        logic::run(
            w,
            content,
            &EffectSrc::Checklist(id.to_string(), true),
            &def.done,
            &mut ctx,
        );
    }
    // little extras
    match id {
        "light_window" => {
            if let Some(uid) = logic::window_light(w, content)
                && let Some(p) = w.furniture.iter_mut().find(|p| p.uid == uid)
            {
                p.on = true;
            }
        }
        "light_off_at_six" => {
            if let Some(uid) = logic::window_light(w, content)
                && let Some(p) = w.furniture.iter_mut().find(|p| p.uid == uid)
            {
                p.on = false;
            }
        }
        _ => {}
    }
    for s in w.staff.iter_mut().filter(|s| s.on_duty()) {
        s.mood = (s.mood + 0.5).min(100.0);
    }
    w.emit(SimEvent::ChecklistTicked { id: id.to_string() });
    super::tool::on_checklist_done(w, content, id);
}

pub(crate) fn on_day_start(w: &mut World, content: &Content) {
    w.app.week = week_of(w.night);
    crew_changed(w, content);
    // story announcements of this night
    for a in &content.app.announcements {
        if w.mode == Mode::Story
            && a.night == Some(w.night)
            && !w.app.announcements.iter().any(|x| x.id == a.id)
        {
            w.app.announcements.push(PostedAnnouncement {
                id: a.id.clone(),
                author: a.author.clone(),
                night: w.night,
                args: Vec::new(),
            });
        }
    }
    // sandbox: occasional swap / time-off requests
    if w.mode == Mode::Sandbox && w.staff.iter().filter(|s| !s.owner).count() >= 2 {
        let ctx = EffectCtx::default();
        if w.rng.chance(0.08) {
            bot(w, content, "swap_request", &ctx);
        } else if w.rng.chance(0.05) {
            let toil = w.staff.iter().any(|s| s.toil_hours >= 4.0);
            bot(
                w,
                content,
                if toil {
                    "time_off_toil"
                } else {
                    "time_off_vacation"
                },
                &ctx,
            );
        }
    }
}

pub(crate) fn on_night_start(w: &mut World, content: &Content) {
    w.app.checklist = content
        .app
        .checklists
        .iter()
        .map(|c| ChecklistState {
            id: c.id.clone(),
            done: false,
            by: None,
            minute: None,
            claim: None,
        })
        .collect();
    w.tonight.checklist_total = w.app.checklist.len() as u32;
    // a published, unchanged schedule makes the crew happy; last-minute edits don't
    let dirty = w.app.draft_dirty;
    for s in w.staff.iter_mut() {
        if !s.owner {
            s.mood = (s.mood + if dirty { -2.0 } else { 1.0 }).clamp(0.0, 100.0);
        }
    }
    // a handover note from last night helps
    if w.app
        .handovers
        .last()
        .is_some_and(|h| h.night + 1 == w.night)
    {
        for s in w.staff.iter_mut() {
            s.mood = (s.mood + 1.5).min(100.0);
        }
    }
}

pub(crate) fn on_minute(_w: &mut World, _content: &Content) {}

pub(crate) fn on_night_end(w: &mut World, content: &Content) {
    // missed checklist items
    let missed: Vec<String> = w
        .app
        .checklist
        .iter()
        .filter(|c| !c.done)
        .map(|c| c.id.clone())
        .collect();
    for id in missed {
        if let Some(def) = content.app.checklists.iter().find(|c| c.id == id)
            && !def.missed.is_empty()
        {
            let mut ctx = EffectCtx::default();
            logic::run(
                w,
                content,
                &EffectSrc::Checklist(id.clone(), false),
                &def.missed,
                &mut ctx,
            );
        }
    }
    // handover note (předávka)
    let handover_done = w
        .app
        .checklist
        .iter()
        .any(|c| c.id == "handover_note" && c.done);
    if handover_done || w.app.handover_text.is_some() {
        let mut lines = Vec::new();
        if w.tonight.guests_served < 15 {
            lines.push(("app.handover.quiet".to_string(), Vec::new()));
        } else if let Some(best) = w.econ.sold.iter().max_by_key(|(_, n)| **n) {
            lines.push((
                "app.handover.busy".to_string(),
                vec![("item".to_string(), format!("menu.{}.name", best.0))],
            ));
        }
        if let Some(broken) = w.furniture.iter().find(|p| p.wear >= 0.5) {
            lines.push((
                "app.handover.broken".to_string(),
                vec![(
                    "thing".to_string(),
                    format!("furniture.{}.name", broken.def),
                )],
            ));
        }
        if let Some(r) = w
            .story
            .regulars
            .iter()
            .find(|(_, s)| s.visits.last().is_some_and(|v| v.night == w.night))
            .map(|(id, _)| id.clone())
        {
            lines.push((
                "app.handover.regular".to_string(),
                vec![("name".to_string(), format!("regular.{r}.name"))],
            ));
        }
        // the helper who did the most tonight signs it; on a night alone, Karolina herself
        let star = w
            .staff
            .iter()
            .filter(|s| s.tasks_done > 0)
            .max_by_key(|s| s.tasks_done)
            .or_else(|| w.staff.iter().find(|s| s.owner))
            .map(|s| s.name.clone())
            .unwrap_or_default();
        lines.push((
            "app.handover.signature".to_string(),
            vec![("name".to_string(), star.clone())],
        ));
        let text = w.app.handover_text.take();
        w.app.handovers.push(Handover {
            night: w.night,
            lines,
            text,
        });
        if w.app.handovers.len() > 30 {
            w.app.handovers.remove(0);
        }
        let ctx = EffectCtx::default();
        bot_with(
            w,
            content,
            "handover_ready",
            &ctx,
            vec![("name".into(), star)],
        );
    }
    // weekly payroll basis
    if w.night.is_multiple_of(7) {
        payroll(w, content);
    }
}

fn payroll(w: &mut World, content: &Content) {
    let week = week_of(w.night);
    let mut rows = Vec::new();
    let mut total = 0i64;
    let mut hours = 0.0;
    for s in w.staff.iter_mut() {
        if s.owner {
            continue;
        }
        let vac = s
            .leave_nights
            .iter()
            .filter(|(n, k)| *n + 7 > w.night && *k == LeaveKind::Vacation)
            .count() as f32
            * 8.0;
        let toil = s
            .leave_nights
            .iter()
            .filter(|(n, k)| *n + 7 > w.night && *k == LeaveKind::Toil)
            .count() as f32
            * 8.0;
        let wage = (s.wage as f32 * s.hours_week / 8.0).round() as i64;
        // overtime over 40 h becomes náhradní volno
        if s.hours_week > 40.0 {
            s.toil_hours += s.hours_week - 40.0;
        }
        rows.push(PayrollRow {
            personal_number: s.personal_number,
            staff: s.id,
            name: s.name.clone(),
            hours: s.hours_week,
            night_hours: s.hours_week,
            wage,
            vacation_hours: vac,
            toil_hours: toil,
        });
        total += wage;
        hours += s.hours_week;
        s.hours_week = 0.0;
    }
    let tips = std::mem::take(&mut w.app.week_tips);
    w.app.payroll.push(PayrollWeek {
        week,
        rows,
        tips,
        total_hours: hours,
        total,
    });
    let ctx = EffectCtx::default();
    bot_with(
        w,
        content,
        "payroll_ready",
        &ctx,
        vec![
            ("week".into(), week.to_string()),
            ("hours".into(), format!("{hours:.0}")),
            ("total".into(), total.to_string()),
        ],
    );
}

impl World {
    /// The week in progress as a payroll sheet (hours and wages so far; nothing is booked).
    pub fn payroll_so_far(&self) -> PayrollWeek {
        let mut rows = Vec::new();
        let (mut total, mut hours) = (0i64, 0.0f32);
        for s in self.staff.iter().filter(|s| !s.owner) {
            let wage = (s.wage as f32 * s.hours_week / 8.0).round() as i64;
            rows.push(PayrollRow {
                personal_number: s.personal_number,
                staff: s.id,
                name: s.name.clone(),
                hours: s.hours_week,
                night_hours: s.hours_week,
                wage,
                vacation_hours: 0.0,
                toil_hours: 0.0,
            });
            total += wage;
            hours += s.hours_week;
        }
        PayrollWeek {
            week: week_of(self.night),
            rows,
            tips: self.app.week_tips,
            total_hours: hours,
            total,
        }
    }
}

pub(crate) fn digest(w: &mut World, content: &Content, summary: &NightSummary) {
    let ctx = EffectCtx::default();
    bot_with(
        w,
        content,
        "night_digest",
        &ctx,
        vec![
            ("night".into(), summary.night.to_string()),
            ("guests".into(), summary.guests_served.to_string()),
            ("revenue".into(), summary.revenue.to_string()),
        ],
    );
}

/// Digest lines (`app.digest.<id>`, args) for the morning screen.
pub fn digest_lines(summary: &NightSummary) -> Vec<(String, Vec<(String, String)>)> {
    let a = |k: &str, v: String| (k.to_string(), v);
    let mut v = vec![
        (
            "app.digest.title".into(),
            vec![a("night", summary.night.to_string())],
        ),
        (
            "app.digest.guests".into(),
            vec![
                a("served", summary.guests_served.to_string()),
                a("left", summary.guests_left_unhappy.to_string()),
            ],
        ),
        (
            "app.digest.revenue".into(),
            vec![a("revenue", summary.revenue.to_string())],
        ),
        (
            "app.digest.tips".into(),
            vec![a("tips", summary.tips.to_string())],
        ),
    ];
    if let Some(b) = &summary.best_seller {
        v.push(("app.digest.best_seller".into(), vec![a("item", b.clone())]));
    }
    let stars = ((summary.avg_satisfaction / 20.0).round() as u32).clamp(1, 5);
    v.push((
        "app.digest.rating".into(),
        vec![a("stars", stars.to_string())],
    ));
    if summary.guests_served < 10 {
        v.push(("app.digest.quiet".into(), Vec::new()));
    }
    v
}

// ---------------------------------------------------------------------------------------------
// Default rozpis
// ---------------------------------------------------------------------------------------------

/// Full-shift hires per night before extra people get split shifts (22–02 / 02–06).
pub const ROZPIS_MAX_FULL_HIRES: usize = 2;

/// Weekday of a night (story nights carry their own weekday).
pub(crate) fn weekday_of_night(w: &World, content: &Content, night: u32) -> usize {
    match w.mode {
        Mode::Story => content
            .story
            .night(night)
            .map(|n| n.weekday as usize)
            .unwrap_or(((night.max(1) - 1) % 7) as usize),
        Mode::Sandbox => ((night.max(1) - 1) % 7) as usize,
    }
}

/// How busy each weekday is (sum of the town's guest pools × weekday multipliers).
pub(crate) fn weekday_demand(w: &World, content: &Content) -> [f32; 7] {
    let mut d = [0.0f32; 7];
    for g in &content.guests {
        if g.institution
            .as_ref()
            .is_some_and(|i| !w.town.institutions.iter().any(|x| x == i))
        {
            continue;
        }
        let avg: f32 = (0..8).map(|h| g.weight_at(h)).sum::<f32>() / 8.0;
        for (wd, x) in d.iter_mut().enumerate() {
            *x += avg * g.weekday_mult.get(wd).copied().unwrap_or(1.0);
        }
    }
    // normalised: the average night is 1.0
    let mean = (d.iter().sum::<f32>() / 7.0).max(0.01);
    for x in d.iter_mut() {
        *x = (*x / mean).max(0.05);
    }
    d
}

/// A sensible default rozpis for the current crew.
///
/// * The owner (the player) is in every night, 22–06.
/// * Hires get rotating days off: 5 shifts a week with up to three hires, 4 with more
///   (a lone hire works every night — the coverage rule wins: at least 2 people a night).
/// * Nights are filled by demand (Fri/Sat first get the second person), days off are
///   staggered so no night is left to the owner alone.
/// * Approved time off in the coming week (dovolená / náhradní volno) is respected.
/// * More than [`ROZPIS_MAX_FULL_HIRES`] hires on one night: the extra people get split
///   shifts, alternating 22–02 and 02–06.
pub(crate) fn default_rozpis(w: &World, content: &Content) -> BTreeMap<u32, [Shift; 7]> {
    super::rozpis::autofill(w, content, base_rozpis(w, content)).0
}

/// The coverage plan before preferences (counts, coverage, split shifts).
pub(crate) fn base_rozpis(w: &World, content: &Content) -> BTreeMap<u32, [Shift; 7]> {
    let mut out: BTreeMap<u32, [Shift; 7]> = BTreeMap::new();
    let hires: Vec<&Staff> = w.staff.iter().filter(|s| !s.owner).collect();
    for s in w.staff.iter().filter(|s| s.owner) {
        out.insert(s.id, [Shift::Full; 7]);
    }
    let n = hires.len();
    if n == 0 {
        return out;
    }
    let per_person = match n {
        1 => 7,
        2 | 3 => 5,
        _ => 4,
    };
    let demand = weekday_demand(w, content);
    // weekdays each hire is away in the coming week
    let away: Vec<[bool; 7]> = hires
        .iter()
        .map(|s| {
            let mut a = [false; 7];
            for (night, _) in &s.leave_nights {
                if *night >= w.night && *night < w.night + 7 {
                    a[weekday_of_night(w, content, *night)] = true;
                }
            }
            a
        })
        .collect();
    // each person's main job (their best skill); people sharing it take their days off on
    // different nights, so the kitchen / the bar isn't left to the owner
    let main_job: Vec<usize> = hires
        .iter()
        .map(|s| {
            (0..5)
                .max_by(|a, b| s.skills[*a].total_cmp(&s.skills[*b]))
                .unwrap_or(0)
        })
        .collect();
    let mut on = vec![[false; 7]; n];
    let mut count = [0usize; 7];
    let mut job_count = [[0usize; 5]; 7];
    for k in 0..n {
        let mut days: Vec<usize> = (0..7).filter(|d| !away[k][*d]).collect();
        // least covered (relative to demand, and for this person's job) first; the rotation
        // staggers the days off
        let key =
            |d: usize| (count[d] + 1) as f32 / demand[d] + job_count[d][main_job[k]] as f32 * 0.8;
        days.sort_by(|a, b| {
            key(*a)
                .total_cmp(&key(*b))
                .then(((a + 7 - (k * 2) % 7) % 7).cmp(&((b + 7 - (k * 2) % 7) % 7)))
        });
        for d in days.into_iter().take(per_person) {
            on[k][d] = true;
            count[d] += 1;
            job_count[d][main_job[k]] += 1;
        }
    }
    // coverage: nobody but the owner on a night → the least-busy available hire steps in
    for d in 0..7 {
        if count[d] > 0 {
            continue;
        }
        let pick = (0..n)
            .filter(|&k| !away[k][d] && !on[k][d])
            .min_by_key(|&k| on[k].iter().filter(|x| **x).count());
        if let Some(k) = pick {
            on[k][d] = true;
            count[d] += 1;
        }
    }
    let mut rows: Vec<[Shift; 7]> = on
        .iter()
        .map(|r| r.map(|x| if x { Shift::Full } else { Shift::Off }))
        .collect();
    // overstaffed nights: extra people work half a night
    for d in 0..7 {
        if count[d] <= ROZPIS_MAX_FULL_HIRES {
            continue;
        }
        // the newest people get the split shifts, alternating early / late
        let mut extra = 0usize;
        for k in (0..n).rev() {
            if count[d] - extra <= ROZPIS_MAX_FULL_HIRES {
                break;
            }
            if rows[k][d] == Shift::Full {
                rows[k][d] = if extra.is_multiple_of(2) {
                    Shift::Early
                } else {
                    Shift::Late
                };
                extra += 1;
            }
        }
    }
    for (k, s) in hires.iter().enumerate() {
        out.insert(s.id, rows[k]);
    }
    out
}

/// The crew or their time off changed: keep the rozpis sensible.
///
/// While the player hasn't edited the rozpis by hand (`AppState::manual_rozpis`), the app
/// plans it: draft **and** published become the default rozpis (no "unpublished" grumbling).
/// After manual edits only rows for new people are added (their default row, published too so
/// they can start tonight) and rows of people who left are dropped.
pub(crate) fn crew_changed(w: &mut World, content: &Content) {
    super::rozpis::sync_prefs(w, content);
    let def = default_rozpis(w, content);
    if !w.app.manual_rozpis {
        if w.app.published != def || w.app.draft != def {
            w.app.draft = def.clone();
            w.app.published = def;
            w.app.draft_dirty = false;
            super::rozpis::on_published(w, content);
        }
        super::rozpis::score(w, content);
        return;
    }
    for sched in [&mut w.app.draft, &mut w.app.published] {
        sched.retain(|id, _| def.contains_key(id));
        for (id, row) in &def {
            sched.entry(*id).or_insert(*row);
        }
    }
}
