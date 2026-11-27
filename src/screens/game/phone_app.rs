//! The in-game **Shiftella** app (T) on a tablet: rozpis editor (draft → publish), requests
//! (swaps, dovolená / náhradní volno), @shiftella_bot messages, the board (announcements +
//! předávka), checklists, the night digest and the payroll basis (podklady pro mzdy).

use macroquad::prelude::*;

use super::world_view::{self as wv, sim};
use super::{Ctx, GameScreen};
use crate::i18n::{format_decimal, format_money};
use crate::settings::Locale;
use crate::ui::draw::{self, Paint, fill_rrect, stroke_rrect, with_a};
use crate::ui::{
    Align, Button, Device, Flow, Glass, Icon, PhoneFrame, RectExt, TextStyle, Tone, Ui, Weight,
};
use sim::{Command, Phase, RequestStatus, Shift};

const TABS: [(Icon, &str); 7] = [
    (Icon::Calendar, "ui.app.tab.schedule"),
    (Icon::Swap, "ui.app.tab.requests"),
    (Icon::Bell, "ui.app.tab.bot"),
    (Icon::Megaphone, "ui.app.tab.board"),
    (Icon::ClipboardCheck, "ui.app.tab.checklists"),
    (Icon::Chart, "ui.app.tab.digest"),
    (Icon::FileExport, "ui.app.tab.payroll"),
];

pub fn draw(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let loc = cx.loc;
    let s = ui.screen();
    let h = (s.h - 150.0).clamp(380.0, 600.0);
    let mut size = Device::Tablet.size_for_height(h);
    if size.x > s.w - 40.0 {
        size = vec2(s.w - 40.0, h);
    }
    let r = Rect::new(
        (s.w - size.x) / 2.0,
        84.0 + (s.h - 140.0 - h) / 2.0,
        size.x,
        size.y,
    );
    g.anchors.push(("panel", r));
    ui.region(r);
    let labels: Vec<(Icon, &str)> = TABS.iter().map(|(i, k)| (*i, loc.tr(k))).collect();
    let time = g.world.clock_label();
    let unread = super::hud::pending_requests(&g.world)
        + g.world
            .app()
            .inbox
            .iter()
            .filter(|m| m.answered.is_none() && has_actions(cx, &m.template))
            .count() as u32;
    let sub = format!("{} · {}", g.world.cafe_name(), g.world.town().name);
    let out = ui.phone_frame(
        "shiftella_tablet",
        r,
        &PhoneFrame::new(&time, &labels)
            .tablet()
            .subtitle(&sub)
            .selected(g.ui_state.app_tab)
            .badge(unread)
            .user(loc.tr("cast.player.name")),
    );
    if let Some(i) = out.tab_clicked {
        g.ui_state.app_tab = i;
    }
    if out.bell_clicked {
        g.ui_state.app_tab = 2;
    }
    let c = out.content.inset(4.0);
    ui.push_clip(out.content);
    match g.ui_state.app_tab {
        0 => super::app_tool::rozpis_tab(g, ui, cx, c),
        1 => requests(g, ui, cx, c),
        2 => messages(g, ui, cx, c),
        3 => board(g, ui, cx, c),
        4 => checklists(g, ui, cx, c),
        5 => digest(g, ui, cx, c),
        _ => payroll(g, ui, cx, c),
    }
    ui.pop_clip();
    // close button beside the device
    let mut cr = Rect::new(r.right() + 8.0, r.y, 28.0, 28.0);
    // clear of the tip jar chip (it reaches this far at a large UI scale / text size)
    if let Some(jar) = g.anchors.iter().find(|a| a.0 == "hud.tipjar").map(|a| a.1)
        && jar.overlaps(&cr)
    {
        cr.y = jar.bottom() + 6.0;
    }
    if ui
        .icon_button(
            "app_close",
            cr,
            Icon::Close,
            &crate::ui::IconButton::secondary().tooltip(loc.tr("ui.common.close")),
        )
        .clicked
    {
        g.panel = None;
    }
}

fn has_actions(cx: &Ctx, template: &str) -> bool {
    cx.content
        .app
        .bot_template(template)
        .is_some_and(|t| !t.actions.is_empty())
}

fn shift_label(s: Shift) -> &'static str {
    match s {
        Shift::Off => "—",
        Shift::Full => "22–06",
        Shift::Early => "22–02",
        Shift::Late => "02–06",
    }
}

fn next_shift(s: Shift, back: bool) -> Shift {
    let all = Shift::ALL;
    let i = all.iter().position(|x| *x == s).unwrap_or(0);
    if back {
        all[(i + all.len() - 1) % all.len()]
    } else {
        all[(i + 1) % all.len()]
    }
}

fn empty(ui: &mut Ui, r: Rect, loc: &Locale) {
    let t = ui.theme.clone();
    ui.fonts.draw_in(
        loc.tr("ui.app.empty"),
        r.with_h(60.0),
        &t.small(),
        Align::Center,
    );
}

/// The old plain rozpis grid (kept for reference; the tab shows the rozpis puzzle).
#[allow(dead_code)]
fn schedule(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx, r: Rect) {
    let t = ui.theme.clone();
    let loc = cx.loc;
    let content = cx.content;
    let (head, body) = r.split_top(30.0, 8.0);
    let app = g.world.app().clone();
    let title = format!(
        "{} · {}",
        loc.tr("ui.app.schedule_title"),
        loc.trf("ui.app.week", &[("n", &app.week.to_string())])
    );
    let title_w = ui.fonts.measure(&title, &t.h1());
    ui.fonts.draw_in(
        &title,
        Rect::new(head.x + 2.0, head.y, head.w * 0.55, head.h),
        &t.h1(),
        Align::Left,
    );
    // the controls on the right take what is left next to the title; the less important
    // ones (mode chip, auto-fill label) give way first
    let room = head.w - title_w.min(head.w * 0.55) - 16.0;
    let mut fr = Flow::row_rev(head, 6.0);
    let b = Button::new(loc.tr("ui.app.publish"))
        .primary()
        .icon(Icon::Check)
        .disabled(!app.draft_dirty);
    let w = ui.button_width(&b);
    if ui
        .button("publish", fr.next(w).inset_xy(0.0, 1.0), &b)
        .clicked
    {
        match g.command(content, Command::PublishSchedule) {
            Ok(()) => g.toast(
                crate::ui::ToastKind::Success,
                Icon::Calendar,
                loc.tr_owned("ui.app.published"),
                None,
            ),
            Err(e) => super::panels::command_error(g, &loc, &e),
        }
    }
    // hand the rozpis back to the app (undoable with Ctrl+Z / the toast)
    let auto = Button::new(loc.tr("ui.app.autofill"))
        .ghost()
        .icon(Icon::Sparkle)
        .tooltip(loc.tr("ui.app.autofill_tip"));
    let (chip_txt, _, _) = if app.draft_dirty {
        (loc.tr("ui.app.draft"), Tone::Warn, Icon::Note)
    } else {
        (loc.tr("ui.app.is_published"), Tone::Ok, Icon::Check)
    };
    let mode_txt = if app.manual_rozpis {
        loc.tr("ui.app.mode_manual")
    } else {
        loc.tr("ui.app.mode_auto")
    };
    let full = w
        + ui.button_width(&auto)
        + ui.chip_width(chip_txt, true)
        + ui.chip_width(mode_txt, false)
        + 18.0;
    let compact_auto = full > room;
    let show_mode = full - ui.button_width(&auto) + 30.0 <= room;
    let clicked_auto = if compact_auto {
        ui.icon_button(
            "autofill",
            fr.next(28.0),
            Icon::Sparkle,
            &crate::ui::IconButton::ghost().tooltip(loc.tr("ui.app.autofill_tip")),
        )
        .clicked
    } else {
        let aw = ui.button_width(&auto);
        ui.button("autofill", fr.next(aw).inset_xy(0.0, 1.0), &auto)
            .clicked
    };
    if clicked_auto {
        match g.command(content, Command::AutoSchedule) {
            Ok(()) => g.toast(
                crate::ui::ToastKind::Info,
                Icon::Sparkle,
                loc.tr_owned("ui.app.autofilled"),
                Some(loc.tr_owned("ui.app.autofilled_body")),
            ),
            Err(e) => super::panels::command_error(g, &loc, &e),
        }
    }
    let (mode, mode_tone) = if app.manual_rozpis {
        (loc.tr("ui.app.mode_manual"), Tone::Neutral)
    } else {
        (loc.tr("ui.app.mode_auto"), Tone::Accent)
    };
    if show_mode {
        let mw = ui.chip_width(mode, false);
        let mr = fr.next(mw);
        ui.chip(vec2(mr.x, mr.center_y() - 11.0), mode, mode_tone, None);
    }
    let (chip, tone, icon) = if app.draft_dirty {
        (loc.tr("ui.app.draft"), Tone::Warn, Icon::Note)
    } else {
        (loc.tr("ui.app.is_published"), Tone::Ok, Icon::Check)
    };
    let cw = ui.chip_width(chip, true);
    // the state chip only when it fits beside the title (the Publish button shows it too)
    if w + 28.0 + cw + 18.0 <= room {
        let cr = fr.next(cw);
        ui.chip(vec2(cr.x, cr.center_y() - 11.0), chip, tone, Some(icon));
    }

    let staff: Vec<sim::Staff> = g.world.staff().to_vec();
    let name_w = 150.0;
    let sum_w = 44.0;
    ui.glass(body, &Glass::card().radius(10.0));
    let inner = body.inset(8.0);
    let col_w = (inner.w - name_w - sum_w) / 7.0;
    let hy = inner.y;
    ui.fonts
        .draw(loc.tr("ui.app.staff"), inner.x + 4.0, hy + 13.0, &t.micro());
    let today = g.world.weekday() as usize;
    for i in 0..7 {
        let x = inner.x + name_w + i as f32 * col_w;
        let wd = crate::content::Weekday::ALL[i];
        let st = t.key().color(if i == today {
            t.indigo_300
        } else if i >= 5 {
            t.violet_400
        } else {
            t.text_dim
        });
        ui.fonts.draw_centered(
            &loc.tr_owned(&wd.short_key()),
            x + col_w / 2.0,
            hy + 13.0,
            &st,
        );
        if i == today {
            fill_rrect(
                Rect::new(x + col_w / 2.0 - 10.0, hy + 18.0, 20.0, 2.0),
                1.0,
                t.indigo_400,
            );
        }
    }
    ui.fonts
        .draw_centered("Σ h", inner.right() - sum_w / 2.0, hy + 13.0, &t.micro());
    let rows_h = inner.bottom() - hy - 28.0 - 26.0;
    let row_h = (rows_h / staff.len().max(1) as f32).clamp(28.0, 44.0);
    let mut coverage = [0u32; 7];
    let sa = ui.begin_scroll(
        "sched_rows",
        Rect::new(inner.x, hy + 26.0, inner.w, rows_h),
        staff.len() as f32 * row_h,
    );
    let mut y = sa.top();
    for (ri, st) in staff.iter().enumerate() {
        let rr = Rect::new(inner.x, y, inner.w - 6.0, row_h - 2.0);
        y += row_h;
        let week = app.draft.get(&st.id).copied().unwrap_or([Shift::Off; 7]);
        for (i, s) in week.iter().enumerate() {
            if *s != Shift::Off {
                coverage[i] += 1;
            }
        }
        if !sa.visible(rr.y, rr.h) {
            continue;
        }
        if ri % 2 == 0 {
            fill_rrect(rr, 6.0, Color::new(1.0, 1.0, 1.0, 0.025));
        }
        let name = wv::staff_name(&loc, st);
        ui.avatar(vec2(rr.x + 14.0, rr.center_y()), 22.0, &name, None, false);
        ui.fonts.draw_in(
            &name,
            Rect::new(rr.x + 30.0, rr.y, name_w - 34.0, rr.h),
            &t.strong().size(12.0),
            Align::Left,
        );
        let mut hours = 0.0;
        for (ci, s) in week.iter().enumerate() {
            hours += s.hours();
            let cell = Rect::new(
                inner.x + name_w + ci as f32 * col_w + 3.0,
                rr.y + 4.0,
                col_w - 6.0,
                rr.h - 8.0,
            );
            let leave = st.leave_nights.iter().any(|(n, _)| {
                // nights of this week: map by weekday of that night
                crate::content::Weekday::ALL[((*n as usize) + 6) % 7] as usize == ci
                    && *n / 7 == g.world.night() / 7
            });
            let resp = ui.interact(ui.id(("shift", st.id, ci)), cell);
            if *s != Shift::Off {
                let (c0, c1) = (t.indigo, t.violet);
                draw::shadow_rrect(
                    cell,
                    6.0,
                    draw::Shadow::new(0.0, 2.0, 8.0, -1.0, with_a(c0, 0.35)),
                );
                fill_rrect(cell, 6.0, Paint::diagonal(cell, c0, c1));
                stroke_rrect(
                    cell,
                    6.0,
                    1.0,
                    Paint::vertical(
                        cell,
                        Color::new(1.0, 1.0, 1.0, 0.32),
                        Color::new(1.0, 1.0, 1.0, 0.04),
                    ),
                );
                // shrink the label to fit narrow cells (large UI scale) instead of "22-…"
                let mut st = TextStyle::new(11.0, Weight::SemiBold, WHITE).tabular();
                let lw = ui.fonts.measure(shift_label(*s), &st);
                if lw > cell.w - 4.0 {
                    st = st.size((11.0 * (cell.w - 4.0) / lw).max(7.5));
                }
                ui.fonts.draw_in(shift_label(*s), cell, &st, Align::Center);
            } else if leave {
                fill_rrect(cell, 6.0, with_a(t.amber, 0.12));
                ui.fonts.draw_in(
                    loc.tr("ui.app.leave_short"),
                    cell,
                    &t.key().color(t.amber),
                    Align::Center,
                );
            } else {
                ui.fonts
                    .draw_in("—", cell, &t.small().color(t.text_disabled), Align::Center);
            }
            if resp.hovered {
                stroke_rrect(cell, 6.0, 1.0, t.indigo_300);
            }
            ui.tooltip(&resp, loc.tr("ui.app.cell_tip"));
            if resp.clicked || resp.right_clicked {
                let n = next_shift(*s, resp.right_clicked);
                let _ = g.command(
                    content,
                    Command::SetShift {
                        staff: st.id,
                        weekday: ci as u8,
                        shift: n,
                    },
                );
            }
        }
        let sr = Rect::new(inner.right() - sum_w, rr.y, sum_w, rr.h);
        let over = hours > 40.0;
        ui.fonts.draw_in(
            &format_decimal(loc.lang, hours, 0),
            sr,
            &t.number().color(if over { t.amber } else { t.text }),
            Align::Center,
        );
    }
    ui.end_scroll(sa);
    // coverage row
    let cy = inner.bottom() - 22.0;
    ui.fonts.draw_in(
        loc.tr("ui.app.coverage"),
        Rect::new(inner.x + 4.0, cy, name_w, 20.0),
        &t.micro(),
        Align::Left,
    );
    for (i, n) in coverage.iter().enumerate() {
        let x = inner.x + name_w + i as f32 * col_w;
        let c = if *n < 2 { t.amber } else { t.emerald_400 };
        ui.fonts.draw_in(
            &n.to_string(),
            Rect::new(x, cy, col_w, 20.0),
            &t.key().color(c),
            Align::Center,
        );
    }
}

fn status_chip(ui: &mut Ui, loc: &Locale, pos: Vec2, s: RequestStatus) {
    let (k, tone) = match s {
        RequestStatus::Pending => ("ui.app.pending", Tone::Warn),
        RequestStatus::Approved => ("ui.app.approved", Tone::Ok),
        RequestStatus::Declined => ("ui.app.declined", Tone::Danger),
    };
    ui.chip(pos, loc.tr(k), tone, None);
}

fn card_h_early() -> f32 {
    74.0
}

fn requests(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx, r: Rect) {
    let t = ui.theme.clone();
    let loc = cx.loc;
    let content = cx.content;
    let app = g.world.app().clone();
    let names: std::collections::HashMap<u32, String> = g
        .world
        .staff()
        .iter()
        .map(|s| (s.id, wv::staff_name(&loc, s)))
        .collect();
    let name = |id: u32| names.get(&id).cloned().unwrap_or_default();
    let mut items: Vec<(bool, u32)> = Vec::new();
    for s in app.swaps.iter().rev() {
        items.push((true, s.id));
    }
    for s in app.time_off.iter().rev() {
        items.push((false, s.id));
    }
    let early: Vec<sim::EarlyLeaveRequest> = g.world.early_leave_requests().to_vec();
    // package C: what people would like built (on top, it's what the café is about)
    let build_h = super::build_content_ui::requests_height(g, content);
    if items.is_empty() && early.is_empty() && build_h == 0.0 {
        empty(ui, r, &loc);
        return;
    }
    let r = if build_h > 0.0 {
        let only = items.is_empty() && early.is_empty();
        let h = if only { r.h } else { build_h.min(r.h * 0.5) };
        super::build_content_ui::requests_section(g, ui, cx, Rect::new(r.x, r.y, r.w, h));
        Rect::new(r.x, r.y + h + 6.0, r.w, (r.h - h - 6.0).max(0.0))
    } else {
        r
    };
    // "Can I go at 2?" — tonight's early-leave requests first
    let early_h = early.len() as f32 * (card_h_early() + 8.0);
    let mut ey = r.y;
    for e in early.iter().rev() {
        let cr = Rect::new(r.x, ey, r.w - 10.0, card_h_early());
        ey += card_h_early() + 8.0;
        ui.glass(cr, &Glass::card().tint(t.amber));
        let c = cr.inset(12.0);
        crate::ui::draw_icon(Icon::Clock, vec2(c.x + 9.0, c.y + 8.0), 16.0, t.amber);
        let txt = loc.trf(
            "app.request.early_leave",
            &[
                ("name", &name(e.staff)),
                ("time", &wv::clock_of_minute(e.minute)),
            ],
        );
        ui.fonts.draw_in(
            &txt,
            Rect::new(c.x + 24.0, c.y, c.w - 130.0, 18.0),
            &t.strong(),
            Align::Left,
        );
        status_chip(ui, &loc, vec2(c.right() - 100.0, c.y - 2.0), e.status);
        if e.status == RequestStatus::Pending {
            let br = Rect::new(c.x + 24.0, c.bottom() - 26.0, (c.w - 24.0).min(300.0), 26.0);
            let (a, b) = br.split_left(br.w / 2.0 - 3.0, 6.0);
            let ok = ui
                .button(
                    ("early_ok", e.id),
                    a,
                    &Button::new(loc.tr("app.bot.early_leave.action.approve"))
                        .primary()
                        .icon(Icon::Check),
                )
                .clicked;
            let no = ui
                .button(
                    ("early_no", e.id),
                    b,
                    &Button::new(loc.tr("app.bot.early_leave.action.decline")).ghost(),
                )
                .clicked;
            if (ok || no)
                && let Err(err) = g.command(
                    content,
                    Command::AnswerEarlyLeave {
                        id: e.id,
                        approve: ok,
                    },
                )
            {
                super::panels::command_error(g, &loc, &err);
            }
        }
    }
    let r = Rect::new(r.x, r.y + early_h, r.w, (r.h - early_h).max(0.0));
    if items.is_empty() {
        return;
    }
    let card_h = 92.0;
    let sa = ui.begin_scroll("req_scroll", r, items.len() as f32 * (card_h + 8.0));
    let mut y = sa.top();
    let w = r.w - 10.0;
    for (swap, id) in items {
        let cr = Rect::new(r.x, y, w, card_h);
        y += card_h + 8.0;
        if !sa.visible(cr.y, cr.h) {
            continue;
        }
        ui.glass(cr, &Glass::card());
        let c = cr.inset(12.0);
        let (title, when, status) = if swap {
            let s = app.swaps.iter().find(|s| s.id == id).cloned().unwrap();
            let wd = loc.tr_owned(&crate::content::Weekday::ALL[s.weekday as usize % 7].long_key());
            (
                loc.trf(
                    "ui.app.swap_line",
                    &[("from", &name(s.from)), ("to", &name(s.to))],
                ),
                wd,
                s.status,
            )
        } else {
            let s = app.time_off.iter().find(|s| s.id == id).cloned().unwrap();
            let kind = match s.kind {
                sim::LeaveKind::Vacation => loc.tr("ui.app.time_off.vacation"),
                sim::LeaveKind::Toil => loc.tr("ui.app.time_off.toil"),
            };
            (
                format!("{} · {kind}", name(s.staff)),
                format!(
                    "{} · {}",
                    loc.trf("ui.hud.night", &[("night", &s.night.to_string())]),
                    loc.trf(
                        "ui.unit.hours_short",
                        &[("h", &format_decimal(loc.lang, s.hours, 0))]
                    )
                ),
                s.status,
            )
        };
        crate::ui::draw_icon(
            if swap { Icon::Swap } else { Icon::Plane },
            vec2(c.x + 9.0, c.y + 8.0),
            16.0,
            t.violet_400,
        );
        ui.fonts.draw_in(
            &title,
            Rect::new(c.x + 24.0, c.y, c.w - 140.0, 18.0),
            &t.strong(),
            Align::Left,
        );
        status_chip(ui, &loc, vec2(c.right() - 100.0, c.y - 2.0), status);
        // why they ask (a life event, a Need the draft breaks)
        let reason = g
            .world
            .request_meta(id)
            .and_then(|m| m.reason_key.clone())
            .map(|k| {
                let who = g
                    .world
                    .request_meta(id)
                    .map(|m| name(m.staff))
                    .unwrap_or_default();
                loc.trf(&k, &[("name", &who), ("day", &when), ("time", "02:00")])
            });
        let when = match reason {
            Some(r) => format!("{when} · {r}"),
            None => when,
        };
        ui.fonts.draw_in(
            &when,
            Rect::new(c.x + 24.0, c.y + 20.0, c.w - 24.0, 16.0),
            &t.small_medium().color(t.indigo_300),
            Align::Left,
        );
        if status == RequestStatus::Pending {
            let br = Rect::new(c.x, c.bottom() - 26.0, c.w.min(320.0), 26.0);
            let (a, b) = br.split_left(br.w / 2.0 - 3.0, 6.0);
            let ok = ui
                .button(
                    ("req_ok", swap, id),
                    a,
                    &Button::new(loc.tr("ui.app.approve"))
                        .primary()
                        .icon(Icon::Check),
                )
                .clicked;
            let no = ui
                .button(
                    ("req_no", swap, id),
                    b,
                    &Button::new(loc.tr("ui.app.decline")).ghost(),
                )
                .clicked;
            if ok || no {
                let cmd = if swap {
                    Command::AnswerSwap { id, approve: ok }
                } else {
                    Command::AnswerTimeOff { id, approve: ok }
                };
                if let Err(e) = g.command(content, cmd) {
                    super::panels::command_error(g, &loc, &e);
                }
            }
        }
    }
    ui.end_scroll(sa);
}

fn messages(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx, r: Rect) {
    let t = ui.theme.clone();
    let loc = cx.loc;
    let content = cx.content;
    let inbox = g.world.app().inbox.clone();
    if inbox.is_empty() {
        empty(ui, r, &loc);
        return;
    }
    let style = t.small().color(t.text);
    let wrap_w = r.w - 70.0;
    let mut heights = Vec::new();
    for m in inbox.iter().rev() {
        let txt = wv::tr_args(&loc, &format!("app.bot.{}", m.template), &m.args);
        let lines = ui.fonts.wrap(&txt, wrap_w, &style).len().max(1) as f32;
        // room for the buttons, or for the chip of the answer once one was chosen
        let acts = has_actions(cx, &m.template);
        let foot = match (acts, m.answered.is_some()) {
            (true, false) => 32.0,
            (true, true) => 28.0,
            _ => 0.0,
        };
        heights.push(lines * style.line_h().max(16.0) + 40.0 + foot);
    }
    let total: f32 = heights.iter().sum::<f32>() + heights.len() as f32 * 8.0;
    let sa = ui.begin_scroll("inbox", r, total);
    let mut y = sa.top();
    for (m, h) in inbox.iter().rev().zip(heights) {
        let cr = Rect::new(r.x, y, r.w - 10.0, h);
        y += h + 8.0;
        if !sa.visible(cr.y, cr.h) {
            continue;
        }
        ui.glass(cr, &Glass::card());
        let c = cr.inset(12.0);
        crate::render::draw_shiftella_logo(vec2(c.x + 12.0, c.y + 12.0), 22.0);
        ui.fonts
            .draw(loc.tr("cast.bot.name"), c.x + 32.0, c.y + 13.0, &t.strong());
        let when = match m.minute {
            Some(min) => format!(
                "{} · {}",
                loc.trf("ui.hud.night", &[("night", &m.night.to_string())]),
                wv::clock_of_minute(min)
            ),
            None => loc.trf("ui.hud.night", &[("night", &m.night.to_string())]),
        };
        ui.fonts
            .draw_right(&when, c.right(), c.y + 13.0, &t.small());
        let txt = wv::tr_args(&loc, &format!("app.bot.{}", m.template), &m.args);
        ui.fonts.draw_wrapped(
            &txt,
            Rect::new(c.x + 32.0, c.y + 22.0, wrap_w, h - 30.0),
            &style,
            Align::Left,
            None,
        );
        if m.answered.is_none()
            && let Some(tpl) = content.app.bot_template(&m.template)
            && !tpl.actions.is_empty()
        {
            let mut fr = Flow::row(
                Rect::new(c.x + 32.0, c.bottom() - 26.0, c.w - 32.0, 26.0),
                6.0,
            );
            for (i, a) in tpl.actions.iter().enumerate() {
                let label = loc.tr_owned(&format!("app.bot.{}.action.{}", m.template, a.id));
                let b = if i == 0 {
                    Button::new(&label).primary()
                } else {
                    Button::new(&label).ghost()
                };
                let bw = ui.button_width(&b);
                if ui.button(("bot", m.id, i), fr.next(bw), &b).clicked {
                    // a button that can't act now says so; the message stays open
                    if let Err(e) = g.command(
                        content,
                        Command::BotAction {
                            msg: m.id,
                            action: a.id.clone(),
                        },
                    ) {
                        super::panels::command_error(g, &loc, &e);
                    } else {
                        g.hands.tut.bump("bot_button");
                    }
                }
            }
        } else if let Some(a) = &m.answered {
            let label = loc.tr_owned(&format!("app.bot.{}.action.{a}", m.template));
            ui.chip(
                vec2(c.x + 32.0, c.bottom() - 22.0),
                &label,
                Tone::Ok,
                Some(Icon::Check),
            );
        }
    }
    ui.end_scroll(sa);
}

fn board(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx, r: Rect) {
    let t = ui.theme.clone();
    let loc = cx.loc;
    let content = cx.content;
    let app = g.world.app().clone();
    let (left, right) = r.split_left(r.w * 0.56, 12.0);
    // announcements
    ui.section(left.with_h(24.0), loc.tr("ui.app.tab.announcements"));
    // post a thank-you to someone on the crew (lifts everyone's mood a little)
    let crew: Vec<(u32, String)> = g
        .world
        .staff()
        .iter()
        .filter(|s| !s.owner)
        .map(|s| (s.id, wv::staff_name(&loc, s)))
        .collect();
    let post_h = if crew.is_empty() { 0.0 } else { 36.0 };
    if !crew.is_empty() {
        let i = g.ui_state.thank_pick % crew.len();
        let first = crew[i]
            .1
            .split(' ')
            .next()
            .unwrap_or(&crew[i].1)
            .to_string();
        let pr = Rect::new(left.x, left.bottom() - 28.0, left.w - 10.0, 28.0);
        let label = loc.trf("ui.app.thank_post", &[("name", &first)]);
        let b = Button::new(&label).secondary().icon(Icon::Megaphone);
        let (next_r, br) = pr.split_right(28.0, 6.0);
        if ui
            .icon_button(
                "thank_next",
                next_r,
                Icon::Swap,
                &crate::ui::IconButton::ghost().tooltip(loc.tr("ui.app.thank_next")),
            )
            .clicked
        {
            g.ui_state.thank_pick = (i + 1) % crew.len();
        }
        if ui.button("thank_post", br, &b).clicked {
            let cmd = Command::PostAnnouncement {
                id: "thank_you".into(),
                args: vec![("name".into(), first.clone())],
            };
            match g.command(content, cmd) {
                Ok(()) => g.toast(
                    crate::ui::ToastKind::Success,
                    Icon::Megaphone,
                    loc.tr_owned("ui.app.posted"),
                    None,
                ),
                Err(e) => super::panels::command_error(g, &loc, &e),
            }
        }
    }
    let list = Rect::new(left.x, left.y + 28.0, left.w, left.h - 28.0 - post_h);
    if app.announcements.is_empty() {
        empty(ui, list, &loc);
    } else {
        let style = t.small().color(t.text);
        let hs: Vec<f32> = app
            .announcements
            .iter()
            .rev()
            .map(|a| {
                let body = wv::tr_args(&loc, &format!("app.announcement.{}.body", a.id), &a.args);
                ui.fonts.wrap(&body, list.w - 34.0, &style).len().max(1) as f32 * 16.0 + 46.0
            })
            .collect();
        let sa = ui.begin_scroll("ann", list, hs.iter().sum::<f32>() + hs.len() as f32 * 8.0);
        let mut y = sa.top();
        for (a, h) in app.announcements.iter().rev().zip(hs) {
            let cr = Rect::new(list.x, y, list.w - 10.0, h);
            y += h + 8.0;
            if !sa.visible(cr.y, cr.h) {
                continue;
            }
            ui.glass(cr, &Glass::card().tint(t.violet));
            let c = cr.inset(12.0);
            crate::ui::draw_icon(
                Icon::Megaphone,
                vec2(c.x + 8.0, c.y + 8.0),
                15.0,
                t.violet_400,
            );
            let title = wv::tr_args(&loc, &format!("app.announcement.{}.title", a.id), &a.args);
            ui.fonts.draw_in(
                &title,
                Rect::new(c.x + 22.0, c.y, c.w - 22.0, 16.0),
                &t.strong(),
                Align::Left,
            );
            let body = wv::tr_args(&loc, &format!("app.announcement.{}.body", a.id), &a.args);
            ui.fonts.draw_wrapped(
                &body,
                Rect::new(c.x, c.y + 22.0, c.w, h - 30.0),
                &style,
                Align::Left,
                None,
            );
        }
        ui.end_scroll(sa);
    }
    // handover
    ui.section(right.with_h(24.0), loc.tr("ui.app.tab.handover"));
    let mut y = right.y + 30.0;
    // at closing: pick up to two notes for tomorrow (each does something next night)
    let picked = super::app_tool::handover_picker(
        g,
        ui,
        cx,
        Rect::new(right.x, y, right.w, right.h - 110.0),
    );
    if picked > 0.0 {
        y += picked + 6.0;
    } else if let Some(h) = app.handovers.last() {
        let cr = Rect::new(right.x, y, right.w, right.h - 120.0);
        ui.glass(cr, &Glass::card());
        let c = cr.inset(12.0);
        ui.fonts.draw(
            &loc.trf("ui.hud.night", &[("night", &h.night.to_string())]),
            c.x,
            c.y + 10.0,
            &t.micro(),
        );
        let mut ly = c.y + 20.0;
        let style = t.small().color(t.text);
        for (k, args) in &h.lines {
            let txt = wv::tr_args(&loc, k, args);
            let room = ((c.bottom() - ly) / style.line_h()).floor() as usize;
            if room == 0 {
                break;
            }
            ly += ui.fonts.draw_wrapped(
                &txt,
                Rect::new(c.x, ly, c.w, c.bottom() - ly),
                &style,
                Align::Left,
                Some(room),
            ) + 4.0;
        }
        let room = ((c.bottom() - ly) / style.line_h()).floor() as usize;
        if let Some(txt) = &h.text
            && room > 0
        {
            ui.fonts.draw_wrapped(
                &format!("„{txt}“"),
                Rect::new(c.x, ly, c.w, c.bottom() - ly),
                &t.small().color(t.indigo_300),
                Align::Left,
                Some(room),
            );
        }
        y = cr.bottom() + 10.0;
    } else {
        empty(ui, Rect::new(right.x, y, right.w, 60.0), &loc);
        y += 70.0;
    }
    let ir = Rect::new(right.x, y, right.w, 30.0);
    let resp = ui.text_input(
        "handover_text",
        ir,
        &mut g.ui_state.handover_text,
        loc.tr("ui.app.handover_write"),
        140,
    );
    let b = Button::new(loc.tr("ui.app.handover_send"))
        .primary()
        .icon(Icon::Note);
    let bw = ui.button_width(&b);
    let br = Rect::new(right.right() - bw, y + 36.0, bw, 26.0);
    let send = ui
        .button(
            "handover_send",
            br,
            &b.disabled(g.ui_state.handover_text.trim().is_empty()),
        )
        .clicked
        || resp.submitted;
    if send && !g.ui_state.handover_text.trim().is_empty() {
        let text = std::mem::take(&mut g.ui_state.handover_text);
        match g.command(content, Command::WriteHandover { text }) {
            Ok(()) => g.toast(
                crate::ui::ToastKind::Success,
                Icon::Note,
                loc.tr_owned("ui.app.handover_saved"),
                None,
            ),
            Err(e) => super::panels::command_error(g, &loc, &e),
        }
    }
}

fn checklists(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx, r: Rect) {
    let t = ui.theme.clone();
    let loc = cx.loc;
    let content = cx.content;
    let states = g.world.app().checklist.clone();
    let night = g.world.phase() == Phase::Night;
    let kinds = [
        crate::content::ChecklistKind::Opening,
        crate::content::ChecklistKind::Closing,
    ];
    let count = |k| {
        content
            .app
            .checklists
            .iter()
            .filter(|c| c.list == k)
            .count()
    };
    let hint_h = if night { 0.0 } else { 24.0 };
    // the lists scroll inside the tablet; the day hint stays fixed below them
    let view = Rect::new(r.x, r.y, r.w, r.h - hint_h);
    let content_h: f32 = kinds
        .iter()
        .map(|k| 30.0 + count(*k) as f32 * 30.0 + 8.0)
        .sum();
    let sa = ui.begin_scroll("checklists", view, content_h);
    let mut y = sa.top();
    for kind in kinds {
        let items: Vec<_> = content
            .app
            .checklists
            .iter()
            .filter(|c| c.list == kind)
            .collect();
        let done = items
            .iter()
            .filter(|c| states.iter().any(|s| s.id == c.id && s.done))
            .count();
        let key = match kind {
            crate::content::ChecklistKind::Opening => "ui.app.checklist.opening",
            crate::content::ChecklistKind::Closing => "ui.app.checklist.closing",
        };
        let title = format!(
            "{} · {}",
            loc.tr(key),
            loc.trf(
                "ui.app.checklist.progress",
                &[
                    ("done", &done.to_string()),
                    ("total", &items.len().to_string())
                ]
            )
        );
        if sa.visible(y, 26.0) {
            ui.section(Rect::new(r.x, y, r.w - 12.0, 26.0), &title);
        }
        y += 30.0;
        for it in items {
            let rr = Rect::new(r.x, y, r.w - 16.0, 28.0);
            y += 30.0;
            if !sa.visible(rr.y, rr.h) {
                continue;
            }
            let st = states.iter().find(|s| s.id == it.id);
            let mut checked = st.is_some_and(|s| s.done);
            let who = st
                .and_then(|s| s.by)
                .and_then(|id| g.world.staff_by_id(id))
                .map(|s| wv::staff_name(&loc, s))
                .or_else(|| {
                    // not done yet: whose it is (yours, or a helper with that duty)
                    (!checked).then(|| match it.owner() {
                        crate::content::handson::ChecklistOwner::Karolina => {
                            loc.tr_owned("ui.rail.karolina")
                        }
                        crate::content::handson::ChecklistOwner::Either(d) => {
                            format!("{} · {}", loc.tr("ui.rail.karolina"), loc.tr(&d.name_key()))
                        }
                    })
                });
            let label = loc.tr_owned(&format!("app.checklist.{}", it.id));
            let was = checked;
            ui.check_item(
                ("chk", it.id.as_str()),
                rr,
                &label,
                who.as_deref(),
                &mut checked,
            );
            if checked && !was {
                if night {
                    if let Err(e) = g.command(content, Command::TickChecklist { id: it.id.clone() })
                    {
                        super::panels::command_error(g, &loc, &e);
                    }
                } else {
                    g.toast(
                        crate::ui::ToastKind::Info,
                        Icon::ClipboardCheck,
                        loc.tr_owned("ui.app.night_only"),
                        None,
                    );
                }
            }
        }
        y += 8.0;
    }
    ui.end_scroll(sa);
    if !night {
        ui.fonts.draw_in(
            loc.tr("ui.app.checklist_hint"),
            Rect::new(r.x, r.bottom() - 20.0, r.w, 20.0),
            &t.small(),
            Align::Left,
        );
    }
}

fn digest(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx, r: Rect) {
    let t = ui.theme.clone();
    let loc = cx.loc;
    let Some(s) = g.world.last_summary().cloned() else {
        empty(ui, r, &loc);
        return;
    };
    let cr = Rect::new(r.x + r.w * 0.15, r.y + 8.0, r.w * 0.7, r.h - 16.0);
    ui.glass(cr, &Glass::card().tint(t.indigo));
    let c = cr.inset(18.0);
    let mut y = c.y + 6.0;
    crate::render::draw_shiftella_logo(vec2(c.x + 14.0, y + 10.0), 26.0);
    for (i, (k, args)) in sim::digest_lines(&s).iter().enumerate() {
        let args: Vec<(String, String)> = args
            .iter()
            .map(|(k, v)| {
                let v = match k.as_str() {
                    "revenue" | "tips" => v
                        .parse::<i64>()
                        .map(|n| format_money(loc.lang, n))
                        .unwrap_or(v.clone()),
                    "item" => loc.tr_owned(&format!("menu.{v}.name")),
                    _ => v.clone(),
                };
                (k.clone(), v)
            })
            .collect();
        let txt = wv::tr_args(&loc, k, &args);
        let style = if i == 0 {
            t.h1()
        } else {
            t.body().color(t.text)
        };
        let x = if i == 0 { c.x + 36.0 } else { c.x };
        y += ui.fonts.draw_wrapped(
            &txt,
            Rect::new(x, y, c.w - (x - c.x), 60.0),
            &style,
            Align::Left,
            None,
        ) + if i == 0 { 16.0 } else { 8.0 };
    }
    // the insights: what last night says about the next one, each with a link
    let room = c.bottom() - y - 4.0;
    if room > 40.0 {
        super::app_tool::insights(g, ui, cx, Rect::new(c.x, y + 4.0, c.w, room));
    }
}

/// The payroll decision screen (opened from the tab, a digest link or a bot button).
pub fn payroll_panel(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    super::app_tool::payroll_panel(g, ui, cx);
}

fn payroll(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx, r: Rect) {
    let t = ui.theme.clone();
    let loc = cx.loc;
    // the decision lives on its own screen; the tab keeps the summary and opens it
    if g.world.app().payroll.last().is_some() {
        let sent = g
            .world
            .app()
            .payroll
            .last()
            .is_some_and(|w| g.world.payroll_sent(w.week));
        let b = Button::new(loc.tr(if sent {
            "ui.payroll.open_sent"
        } else {
            "ui.payroll.open"
        }))
        .primary()
        .icon(Icon::FileExport);
        let bw = ui.button_width(&b);
        let br = Rect::new(r.right() - bw - 12.0, r.bottom() - 34.0, bw, 28.0);
        if ui.button("payroll_open", br, &b).clicked {
            g.panel = Some(super::Panel::Payroll);
        }
    }
    let r = Rect::new(r.x, r.y, r.w, r.h - 36.0);
    let Some(week) = g.world.app().payroll.last().cloned() else {
        ui.fonts.draw_in(
            loc.tr("ui.app.payroll_empty"),
            r.with_h(60.0),
            &t.small(),
            Align::Center,
        );
        return;
    };
    let cr = Rect::new(r.x + r.w * 0.08, r.y + 4.0, r.w * 0.84, r.h - 8.0);
    ui.glass(cr, &Glass::card());
    let c = cr.inset(16.0);
    ui.fonts.draw(
        &loc.trf("app.payroll.title", &[("week", &week.week.to_string())]),
        c.x,
        c.y + 16.0,
        &t.h1(),
    );
    let mut y = c.y + 34.0;
    let cols = [0.0, 0.12, 0.5, 0.64, 0.8];
    let heads = [
        loc.tr("ui.app.payroll_number"),
        loc.tr("ui.app.staff"),
        loc.tr("ui.app.hours"),
        loc.tr("ui.app.night_hours"),
        loc.tr("ui.app.wage"),
    ];
    for (h, x) in heads.iter().zip(cols) {
        ui.fonts.draw(h, c.x + c.w * x, y + 10.0, &t.micro());
    }
    y += 18.0;
    draw::hline(c.x, c.right(), y, t.divider);
    y += 6.0;
    for (i, row) in week.rows.iter().enumerate() {
        if y > c.bottom() - 70.0 {
            break;
        }
        if i % 2 == 0 {
            fill_rrect(
                Rect::new(c.x - 4.0, y - 2.0, c.w + 8.0, 22.0),
                4.0,
                Color::new(1.0, 1.0, 1.0, 0.025),
            );
        }
        let vals = [
            format!("{:04}", row.personal_number),
            row.name.clone(),
            format_decimal(loc.lang, row.hours, 1),
            format_decimal(loc.lang, row.night_hours, 1),
            format_money(loc.lang, row.wage),
        ];
        for (v, x) in vals.iter().zip(cols) {
            ui.fonts.draw_in(
                v,
                Rect::new(c.x + c.w * x, y, c.w * 0.36, 18.0),
                &t.small().color(t.text).tabular(),
                Align::Left,
            );
        }
        y += 22.0;
    }
    let by = c.bottom() - 56.0;
    draw::hline(c.x, c.right(), by, t.divider);
    ui.fonts.draw(
        &loc.trf(
            "app.payroll.tips",
            &[("tips", &format_money(loc.lang, week.tips))],
        ),
        c.x,
        by + 18.0,
        &t.small().color(t.text),
    );
    ui.fonts.draw(
        &loc.trf(
            "app.payroll.total",
            &[
                ("hours", &format_decimal(loc.lang, week.total_hours, 1)),
                ("total", &format_money(loc.lang, week.total)),
            ],
        ),
        c.x,
        by + 38.0,
        &t.strong().color(t.amber),
    );
    ui.fonts.draw_right(
        loc.tr("app.payroll.footer"),
        c.right(),
        by + 38.0,
        &t.small(),
    );
}
