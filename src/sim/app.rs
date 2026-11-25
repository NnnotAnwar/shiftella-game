//! The in-game Shiftella app: weekly rozpis (draft/published), swap & time-off requests,
//! checklists, handover notes, announcements, @shiftella_bot inbox, digest, payroll basis.
//! Never clock-in/out through the bot.

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
    let Some(t) = content.app.bot_template(template) else {
        return;
    };
    let id = w.alloc_id();
    let mut request = None;
    let mut subject = ctx.staff;
    // requests created by the message
    match t.kind {
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
    if let Some(r) = request {
        let approve = action == "approve";
        if w.app.swaps.iter().any(|s| s.id == r) {
            answer_swap(w, r, approve)?;
        } else if w.app.time_off.iter().any(|x| x.id == r) {
            answer_time_off(w, r, approve)?;
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
}

pub(crate) fn on_day_start(w: &mut World, content: &Content) {
    w.app.week = week_of(w.night);
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
                vec![("item".to_string(), best.0.clone())],
            ));
        }
        if let Some(broken) = w.furniture.iter().find(|p| p.wear >= 0.5) {
            lines.push((
                "app.handover.broken".to_string(),
                vec![("thing".to_string(), broken.def.clone())],
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
                vec![("name".to_string(), r)],
            ));
        }
        let star = w
            .staff
            .iter()
            .filter(|s| s.tasks_done > 0)
            .max_by_key(|s| s.tasks_done)
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
