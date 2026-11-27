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
                    | super::Panel::Attic
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
    let h = (s.h - 88.0 - 60.0).min(510.0);
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
        // Relaxed: the instalment will be covered (no money pressure)
        let key = if g.world.difficulty() == sim::Difficulty::Relaxed {
            "access.day.payment_covered"
        } else {
            "ui.hud.next_payment"
        };
        let txt = loc.trf(
            key,
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
    // sandbox «Challenge»: the season goal, nights left and strikes, the rivals
    if let Some(ch) = g.world.challenge(content) {
        let on_track = ch.earned >= ch.goal;
        row(
            ui,
            y,
            &loc.trf("challenge.plan.goal", &[("season", &ch.season.to_string())]),
            &format!(
                "{} / {}",
                crate::i18n::format_int(loc.lang, ch.earned),
                format_money(loc.lang, ch.goal)
            ),
            if on_track { t.emerald_400 } else { t.amber },
        );
        y += 20.0;
        let mut txt = loc.trf(
            "challenge.plan.left",
            &[
                ("nights", &ch.nights_left.to_string()),
                ("strikes", &ch.strikes.to_string()),
                ("max", &ch.strikes_max.to_string()),
            ],
        );
        if !ch.rivals.is_empty() {
            txt.push_str(" · ");
            txt.push_str(&loc.trf(
                "challenge.plan.rivals",
                &[
                    ("n", &ch.rivals.len().to_string()),
                    ("percent", &format!("{:.0}", ch.loss * 100.0)),
                ],
            ));
        }
        ui.fonts.draw_fit(
            &txt,
            Rect::new(c.x, y, c.w, 16.0),
            &t.small()
                .color(if ch.strikes > 0 { t.amber } else { t.text_dim }),
            Align::Left,
            0.75,
        );
        y += 20.0;
    }
    // the fridge: fresh stock / the room for it (sim::fridge)
    let fr = g.world.fridge(content);
    if fr.pieces > 0 {
        let label = if fr.worn() {
            "fridge.plan.worn"
        } else {
            "fridge.plan.label"
        };
        row(
            ui,
            y,
            loc.tr(label),
            &format!(
                "{} / {}",
                crate::i18n::format_int(loc.lang, fr.fresh),
                format_money(loc.lang, fr.capacity)
            ),
            if fr.over() || fr.worn() {
                t.amber
            } else {
                t.text
            },
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
        // what still has to fit below: the header, the trains and crew lines, the buttons
        let room = c.bottom() - 72.0 - 4.0 - y - 18.0 - 44.0;
        let rows_max = if room >= 26.0 + 30.0 {
            2
        } else if room >= 30.0 {
            1
        } else {
            0
        };
        if !regs.is_empty() && rows_max > 0 {
            ui.fonts
                .draw(loc.tr("ui.plan.regulars"), c.x, y + 12.0, &t.micro());
            y += 18.0;
            // at most two rows of chips (one on a short card); the rest fold into "+n" (the
            // card has a fixed height)
            let mut x = c.x;
            let mut row = 0;
            let total = regs.len();
            for (i, n) in regs.iter().enumerate() {
                let cw = ui.chip_width(n, true);
                let more_w = ui.chip_width("+9", false) + 6.0;
                let last_row = row + 1 == rows_max;
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
        } else if !regs.is_empty() {
            // a short card: one line of names
            draw_icon(Icon::Heart, vec2(c.x + 6.0, y + 9.0), 12.0, t.violet_400);
            ui.fonts.draw_fit(
                &regs.join(", "),
                Rect::new(c.x + 18.0, y, c.w - 18.0, 18.0),
                &t.small().color(t.text),
                Align::Left,
                0.8,
            );
            y += 22.0;
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
    // grandma's attic (and the back room): new things on Saturdays — a compact button at the
    // right end of the crew line, so it never covers the text above the shortcuts
    let attic = g.world.attic(content);
    let attic_label = (attic.open || g.world.back_room(content).available).then(|| {
        let fresh = attic.offers.iter().filter(|o| !o.owned).count();
        if fresh > 0 {
            format!("{} · {fresh}", loc.tr("ui.attic.title"))
        } else {
            loc.tr_owned("ui.attic.title")
        }
    });
    let attic_b = attic_label
        .as_deref()
        .map(|l| Button::new(l).ghost().icon(Icon::Home));
    let attic_w = attic_b
        .as_ref()
        .map_or(0.0, |b| (ui.button_width(b) + 4.0).min(c.w * 0.55));
    ui.fonts.draw_in(
        &loc.trf("ui.plan.crew_tonight", &[("n", &on.to_string())]),
        Rect::new(c.x, y, c.w - attic_w - 6.0, 18.0),
        &t.small(),
        Align::Left,
    );
    if let Some(b) = attic_b {
        let ar = Rect::new(c.right() - attic_w, y - 5.0, attic_w, 26.0);
        if ui.button("plan_attic", ar, &b).clicked {
            super::attic_ui::toggle(g);
        }
    }
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
                .shortcut(&crate::settings::keys::label(
                    crate::settings::keys::Action::App,
                )),
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
                .shortcut(&crate::settings::keys::label(
                    crate::settings::keys::Action::Build,
                )),
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
    // "What the café gave you tonight": a heading + one row per line
    let pay_h = if sm.payoff.is_empty() {
        0.0
    } else {
        24.0 + 18.0 * sm.payoff.len() as f32
    } + if sm.challenge.is_empty() {
        0.0
    } else {
        24.0 + 18.0 * sm.challenge.len() as f32
    };
    let (w, h) = (440.0 + pw, (430.0 + pay_h).min(s.h - 24.0));
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
    if !sm.payoff.is_empty() {
        hline(c.x, c.right(), y + 2.0, t.divider);
        y += 8.0;
        ui.fonts.draw(
            &loc.tr("purchases.summary.title").to_uppercase(),
            c.x,
            y + 12.0,
            &t.micro(),
        );
        y += 18.0;
        for l in &sm.payoff {
            // the fridge's lines carry a snowflake; what went off is dimmed
            let (icon, color) = if l.key.starts_with("purchases.line.spoiled") {
                (Icon::Snowflake, t.text_dim)
            } else if l.key.starts_with("purchases.line.fridge") {
                (Icon::Snowflake, t.amber)
            } else {
                (Icon::Sparkle, t.amber)
            };
            draw_icon(icon, vec2(c.x + 7.0, y + 8.0), 11.0, color);
            ui.fonts.draw_fit(
                &payoff_text(&loc, l),
                Rect::new(c.x + 18.0, y, c.w - 18.0, 16.0),
                &t.small().color(t.text),
                Align::Left,
                0.8,
            );
            y += 18.0;
        }
    }
    // sandbox «Challenge»: the season goal, the rivals' toll, a strike
    if !sm.challenge.is_empty() {
        hline(c.x, c.right(), y + 2.0, t.divider);
        y += 8.0;
        // (the night's own season: after its last night the next one has already begun)
        let season = (sm.night.max(1) - 1) / cx.content.sandbox.challenge.season_nights.max(1) + 1;
        ui.fonts.draw(
            &loc.trf(
                "challenge.summary.title",
                &[("season", &season.to_string())],
            )
            .to_uppercase(),
            c.x,
            y + 12.0,
            &t.micro(),
        );
        y += 18.0;
        for l in &sm.challenge {
            let (icon, color) = if l.key.ends_with("missed") || l.key.ends_with("debt") {
                (Icon::Warning, t.red_400)
            } else if l.key.ends_with("goal_met") {
                (Icon::Star, t.emerald_400)
            } else if l.key.contains("rival") {
                (Icon::Door, t.amber)
            } else if l.key.ends_with("strengths") {
                (Icon::Heart, t.violet_400)
            } else {
                (Icon::Calendar, t.indigo_300)
            };
            draw_icon(icon, vec2(c.x + 7.0, y + 8.0), 11.0, color);
            ui.fonts.draw_fit(
                &wv::tr_args(&loc, &l.key, &l.args),
                Rect::new(c.x + 18.0, y, c.w - 18.0, 16.0),
                &t.small().color(t.text),
                Align::Left,
                0.8,
            );
            y += 18.0;
        }
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

/// One "What the café gave you tonight" line in words: `@key` args are translated (piece and
/// level names), `money` is formatted.
pub fn payoff_text(loc: &crate::settings::Locale, l: &sim::purchases::PayoffLine) -> String {
    let args: Vec<(String, String)> = l
        .args
        .iter()
        .map(|(k, v)| {
            let v = if let Some(key) = v.strip_prefix('@') {
                loc.tr_owned(key)
            } else if k == "money" {
                format_money(loc.lang, v.parse().unwrap_or(0))
            } else {
                v.clone()
            };
            (k.clone(), v)
        })
        .collect();
    let refs: Vec<(&str, &str)> = args.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    loc.trf(&l.key, &refs)
}
