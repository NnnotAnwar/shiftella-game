//! Day planning card (non-blocking, right side) and the warm, short morning summary.

use macroquad::prelude::*;

use super::world_view::{self as wv, sim};
use super::{Ctx, GameScreen};
use crate::i18n::{format_decimal, format_money, format_money_delta};
use crate::ui::draw::{fill_rrect, hline, with_a};
use crate::ui::{Align, Button, Glass, Icon, RectExt, Ui, draw_icon};
use sim::Mode;

pub fn planning(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    if g.hands.book.is_some()
        || matches!(
            g.panel,
            Some(
                super::Panel::App
                    | super::Panel::Duties
                    | super::Panel::Build
                    | super::Panel::Journal
                    | super::Panel::Payroll
            )
        )
    {
        // keep the screen clear for the big panels; the Start button lives in the dock row
        start_button_only(g, ui, cx);
        return;
    }
    let t = ui.theme.clone();
    let loc = cx.loc;
    let content = cx.content;
    let s = ui.screen();
    let w = 320.0;
    let x = if g.panel == Some(super::Panel::Log) {
        s.w - 12.0 - 360.0 - 12.0 - w
    } else {
        s.w - 12.0 - w
    };
    let night = g.world.night();
    let story = g.world.mode() == Mode::Story;
    let h = (s.h - 88.0 - 60.0).min(470.0);
    let pr = Rect::new(x, 88.0, w, h);
    g.anchors.push(("day.panel", pr));
    ui.glass(pr, &Glass::panel());
    let c = pr.inset(16.0);
    let mut y = c.y;
    // title
    ui.fonts.draw(
        &loc.trf("ui.plan.title", &[("day", &night.to_string())]),
        c.x,
        y + 16.0,
        &t.h1(),
    );
    y += 24.0;
    let wd = loc.tr_owned(&g.world.weekday().long_key());
    ui.fonts.draw_in(
        &loc.trf("ui.plan.next_night", &[("weekday", &wd)]),
        Rect::new(c.x, y, c.w, 16.0),
        &t.small(),
        Align::Left,
    );
    y += 22.0;
    if story && let Some(n) = content.story.night(night) {
        let title = loc.tr_owned(&n.title_key());
        let hint = loc.tr_owned(&n.hint_key());
        let card = Rect::new(c.x, y, c.w, 76.0);
        fill_rrect(card, 10.0, with_a(t.violet, 0.14));
        draw_icon(
            Icon::Moon,
            vec2(card.x + 16.0, card.y + 16.0),
            14.0,
            t.violet_400,
        );
        ui.fonts.draw_in(
            &format!(
                "{} · {}",
                loc.trf(
                    "ui.hud.night_of",
                    &[
                        ("night", &night.to_string()),
                        ("total", &content.story.nights_total.to_string())
                    ]
                ),
                title
            ),
            Rect::new(card.x + 30.0, card.y + 6.0, card.w - 36.0, 20.0),
            &t.strong(),
            Align::Left,
        );
        ui.fonts.draw_wrapped(
            &hint,
            Rect::new(card.x + 12.0, card.y + 30.0, card.w - 24.0, 44.0),
            &t.small().color(t.text),
            Align::Left,
            Some(2),
        );
        y += 86.0;
    }
    // money
    let row = |ui: &mut Ui, y: f32, label: &str, value: &str, color: Color| {
        ui.fonts.draw_in(
            label,
            Rect::new(c.x, y, c.w * 0.6, 18.0),
            &t.small(),
            Align::Left,
        );
        ui.fonts.draw_in(
            value,
            Rect::new(c.x + c.w * 0.4, y, c.w * 0.6, 18.0),
            &t.strong().color(color).tabular(),
            Align::Right,
        );
    };
    row(
        ui,
        y,
        loc.tr("ui.hud.money"),
        &format_money(loc.lang, g.world.money()),
        t.amber,
    );
    y += 20.0;
    if g.world.debt() > 0 {
        row(
            ui,
            y,
            loc.tr("ui.summary.debt_left"),
            &format_money(loc.lang, g.world.debt()),
            t.text,
        );
        y += 20.0;
    }
    if story
        && let Some(p) = content
            .story
            .payments
            .iter()
            .filter(|p| p.night >= night)
            .min_by_key(|p| p.night)
    {
        let txt = loc.trf(
            "ui.hud.next_payment",
            &[
                ("money", &format_money(loc.lang, p.amount)),
                ("night", &p.night.to_string()),
            ],
        );
        let color = if p.night == night {
            t.amber
        } else {
            t.text_dim
        };
        ui.fonts.draw_in(
            &txt,
            Rect::new(c.x, y, c.w, 18.0),
            &t.small_medium().color(color),
            Align::Left,
        );
        y += 20.0;
    } else if !story {
        row(
            ui,
            y,
            loc.tr("ui.plan.rent"),
            &format_money(loc.lang, g.world.rent()),
            t.text,
        );
        y += 20.0;
    }
    hline(c.x, c.right(), y + 4.0, t.divider);
    y += 12.0;
    // yesterday
    if let Some(sm) = g.world.last_summary().cloned() {
        ui.fonts
            .draw(loc.tr("ui.plan.yesterday"), c.x, y + 12.0, &t.micro());
        y += 18.0;
        let txt = loc.trf(
            "ui.plan.yesterday_line",
            &[
                ("served", &sm.guests_served.to_string()),
                ("money", &format_money_delta(loc.lang, sm.net)),
            ],
        );
        ui.fonts.draw_in(
            &txt,
            Rect::new(c.x, y, c.w, 18.0),
            &t.small().color(t.text),
            Align::Left,
        );
        y += 24.0;
    }
    // regulars expected (story knows the schedule)
    if story {
        let regs: Vec<String> = content
            .regulars
            .iter()
            .filter(|r| r.visits_story_night(night) && !r.id.starts_with("guest_anwar"))
            .map(|r| loc.tr_owned(&r.name_key()))
            .collect();
        if !regs.is_empty() {
            ui.fonts
                .draw(loc.tr("ui.plan.regulars"), c.x, y + 12.0, &t.micro());
            y += 18.0;
            // at most two rows of chips; the rest fold into "+n" (the card has a fixed height)
            let mut x = c.x;
            let mut row = 0;
            let total = regs.len();
            for (i, n) in regs.iter().enumerate() {
                let cw = ui.chip_width(n, true);
                let more_w = ui.chip_width("+9", false) + 6.0;
                let last_row = row == 1;
                let fits = x + cw <= c.right()
                    && (!last_row || i + 1 == total || x + cw + more_w <= c.right());
                if !fits {
                    if last_row {
                        let rest = format!("+{}", total - i);
                        ui.chip(vec2(x, y), &rest, crate::ui::Tone::Neutral, None);
                        break;
                    }
                    x = c.x;
                    y += 26.0;
                    row += 1;
                }
                let r = ui.chip(vec2(x, y), n, crate::ui::Tone::Accent, Some(Icon::Heart));
                x = r.right() + 6.0;
            }
            y += 30.0;
        }
    }
    // trains
    let times: Vec<String> = content
        .trains
        .trains
        .iter()
        .filter(|tr| tr.announce_if.is_empty())
        .map(|tr| tr.arrives.to_string())
        .collect();
    ui.fonts.draw_in(
        &loc.trf("ui.plan.forecast_trains", &[("times", &times.join(", "))]),
        Rect::new(c.x, y, c.w, 18.0),
        &t.small(),
        Align::Left,
    );
    y += 22.0;
    // staff on tonight
    let on = g
        .world
        .staff()
        .iter()
        .filter(|s| {
            g.world
                .app()
                .published
                .get(&s.id)
                .is_some_and(|w| w[g.world.weekday() as usize] != sim::Shift::Off)
                || s.owner
        })
        .count();
    ui.fonts.draw_in(
        &loc.trf("ui.plan.crew_tonight", &[("n", &on.to_string())]),
        Rect::new(c.x, y, c.w, 18.0),
        &t.small(),
        Align::Left,
    );
    let _ = y;
    // shortcuts to the panels
    let br = Rect::new(c.x, c.bottom() - 72.0, c.w, 28.0);
    let (a, b) = br.split_left(br.w / 2.0 - 3.0, 6.0);
    if ui
        .button(
            "plan_app",
            a,
            &Button::new(loc.tr("ui.toolbar.app"))
                .icon(Icon::Calendar)
                .shortcut("T"),
        )
        .clicked
    {
        g.panel = Some(super::Panel::App);
    }
    if ui
        .button(
            "plan_build",
            b,
            &Button::new(loc.tr("ui.toolbar.build"))
                .icon(Icon::Build)
                .shortcut("B"),
        )
        .clicked
    {
        g.panel = Some(super::Panel::Build);
    }
    let sr = Rect::new(c.x, c.bottom() - 36.0, c.w, 36.0);
    g.anchors.push(("day.start", sr));
    if ui
        .button(
            "start_night",
            sr,
            &Button::new(loc.tr("ui.plan.start_night"))
                .primary()
                .icon(Icon::Moon)
                .shortcut("Enter"),
        )
        .clicked
    {
        g.start_night(content);
    }
}

fn start_button_only(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let loc = cx.loc;
    let s = ui.screen();
    let b = Button::new(loc.tr("ui.plan.start_night"))
        .primary()
        .icon(Icon::Moon)
        .shortcut("Enter");
    let w = ui.button_width(&b) + 12.0;
    let r = Rect::new(s.w - 12.0 - w, s.h - 12.0 - 36.0, w, 36.0);
    g.anchors.push(("day.start", r));
    ui.glass(r.inset(-4.0), &Glass::hud());
    if ui.button("start_night_small", r, &b).clicked {
        g.start_night(cx.content);
    }
}

pub fn summary(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let Some(sm) = g.summary.clone() else { return };
    let t = ui.theme.clone();
    let loc = cx.loc;
    let s = ui.screen();
    // dim the world a little
    let dim = ui.ease_value(ui.id("summary_dim"), 1.0, 0.25);
    draw_rectangle(0.0, 0.0, s.w, s.h, Color::new(0.03, 0.02, 0.08, 0.45 * dim));
    // the night's polaroid develops beside the numbers (pick one of the three)
    let photo = g.world.pending_polaroids().is_some();
    let pw = if photo { 300.0 } else { 0.0 };
    let (w, h) = (440.0 + pw, 430.0_f32.min(s.h - 24.0));
    let pr = Rect::new((s.w - w) / 2.0, (s.h - h) / 2.0 - 10.0, w, h);
    ui.glass(pr, &Glass::panel().radius(14.0));
    let c = Rect::new(pr.x, pr.y, 440.0, pr.h).inset(22.0);
    if photo {
        let area = Rect::new(pr.x + 440.0 - 6.0, pr.y + 16.0, pw - 16.0, pr.h - 32.0);
        crate::ui::draw::vline(area.x - 6.0, pr.y + 22.0, pr.bottom() - 22.0, t.divider);
        super::journal_ui::morning_pick(g, ui, cx, area);
    }
    draw_icon(Icon::Sun, vec2(c.x + 10.0, c.y + 12.0), 20.0, t.amber);
    ui.fonts.draw(
        &loc.trf("ui.summary.title", &[("night", &sm.night.to_string())]),
        c.x + 28.0,
        c.y + 19.0,
        &t.title(),
    );
    let mut y = c.y + 40.0;
    let line = |ui: &mut Ui, y: f32, label: &str, value: String, color: Color| {
        ui.fonts.draw_in(
            label,
            Rect::new(c.x, y, c.w * 0.6, 18.0),
            &t.body(),
            Align::Left,
        );
        ui.fonts.draw_in(
            &value,
            Rect::new(c.x + c.w * 0.4, y, c.w * 0.6, 18.0),
            &t.strong().color(color).tabular(),
            Align::Right,
        );
    };
    line(
        ui,
        y,
        loc.tr("ui.summary.served"),
        sm.guests_served.to_string(),
        t.text,
    );
    y += 22.0;
    if sm.guests_left_unhappy > 0 {
        line(
            ui,
            y,
            loc.tr("ui.summary.left_unhappy"),
            sm.guests_left_unhappy.to_string(),
            t.text_dim,
        );
        y += 22.0;
    }
    hline(c.x, c.right(), y + 4.0, t.divider);
    y += 12.0;
    line(
        ui,
        y,
        loc.tr("ui.summary.revenue"),
        format_money(loc.lang, sm.revenue),
        t.emerald_400,
    );
    y += 22.0;
    line(
        ui,
        y,
        loc.tr("ui.summary.tips"),
        format_money(loc.lang, sm.tips),
        t.emerald_400,
    );
    y += 22.0;
    line(
        ui,
        y,
        loc.tr("ui.summary.wages"),
        format_money(loc.lang, -sm.wages),
        t.text,
    );
    y += 22.0;
    let other = sm.supplies + sm.rent + sm.other_expenses;
    line(
        ui,
        y,
        loc.tr("ui.summary.expenses"),
        format_money(loc.lang, -other),
        t.text,
    );
    y += 22.0;
    if sm.payment != 0 || sm.payment_missed {
        let label = if sm.payment_missed {
            loc.tr("ui.summary.payment_missed")
        } else {
            loc.tr("ui.summary.payment")
        };
        line(
            ui,
            y,
            label,
            format_money(loc.lang, -sm.payment.abs()),
            if sm.payment_missed { t.red_400 } else { t.text },
        );
        y += 22.0;
    }
    hline(c.x, c.right(), y + 4.0, t.divider);
    y += 12.0;
    line(
        ui,
        y,
        loc.tr("ui.summary.profit"),
        format_money_delta(loc.lang, sm.net),
        if sm.net >= 0 { t.amber } else { t.red_400 },
    );
    y += 26.0;
    let delta = (sm.reputation_end - sm.reputation_start) / 20.0;
    let rep = loc.trf(
        "ui.summary.reputation",
        &[
            (
                "value",
                &format_decimal(loc.lang, sm.reputation_end / 20.0, 1),
            ),
            (
                "delta",
                &format!(
                    "{}{}",
                    if delta >= 0.0 { "+" } else { "−" },
                    format_decimal(loc.lang, delta.abs(), 1)
                ),
            ),
        ],
    );
    draw_icon(Icon::Star, vec2(c.x + 8.0, y + 9.0), 14.0, t.amber);
    ui.fonts.draw_in(
        &rep,
        Rect::new(c.x + 20.0, y, c.w - 20.0, 18.0),
        &t.body(),
        Align::Left,
    );
    y += 24.0;
    if g.world.mode() == Mode::Story {
        let notes = g.world.notes_found().len();
        draw_icon(Icon::Note, vec2(c.x + 8.0, y + 9.0), 14.0, t.violet_400);
        let mut txt = loc.trf("ui.summary.notes_found", &[("n", &notes.to_string())]);
        if !sm.notes_found.is_empty() {
            txt.push_str(&format!(" · {}", loc.tr("ui.summary.new_note")));
        }
        ui.fonts.draw_in(
            &txt,
            Rect::new(c.x + 20.0, y, c.w - 20.0, 18.0),
            &t.body(),
            Align::Left,
        );
        y += 24.0;
    }
    // a warm closing line
    let mood = if sm.avg_satisfaction >= 70.0 {
        "ui.summary.mood_good"
    } else if sm.avg_satisfaction >= 45.0 || sm.guests_served == 0 {
        "ui.summary.mood_ok"
    } else {
        "ui.summary.mood_hard"
    };
    ui.fonts.draw_wrapped(
        loc.tr(mood),
        Rect::new(c.x, y + 4.0, c.w, 40.0),
        &t.small().color(t.indigo_300),
        Align::Left,
        Some(2),
    );
    let b = Button::new(loc.tr("ui.summary.continue"))
        .primary()
        .icon(Icon::Sun)
        .shortcut("Enter");
    let bw = ui.button_width(&b).max(160.0);
    if ui
        .button(
            "summary_ok",
            Rect::new(c.right() - bw, c.bottom() - 34.0, bw, 34.0),
            &b,
        )
        .clicked
    {
        super::journal_ui::keep_morning(g, cx.content);
        g.summary = None;
        super::walk_home::begin(g, cx.content, None);
    }
    let _ = wv::clock_of_minute(0);
}
