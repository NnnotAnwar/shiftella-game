//! The Shiftella app as a real tool (design/handson.md §12–§13): the rozpis puzzle
//! (preference chips, wellbeing rings, the week's fairness, coverage, auto-fill), the
//! předávka picker at closing, the digest's insights with deep links and the weekly payroll
//! decision (tip split, star of the week, náhradní volno, send).

use macroquad::prelude::*;

use super::world_view::{self as wv, sim};
use super::{Ctx, GameScreen, Panel};
use crate::content::Weekday;
use crate::content::handson::{AppLink, PrefKind, PreferredShift, Strength};
use crate::i18n::{format_decimal, format_money};
use crate::settings::Locale;
use crate::ui::draw::{fill_rrect, stroke_rrect, with_a};
use crate::ui::handson::{
    CoverageNight, FairnessView, Insight, PAYROLL_SIZE, PayRow, PayrollAction, PayrollLabels,
    PayrollView, PrefChip, PrefStatus, RozpisAction, RozpisLabels, RozpisRow, RozpisView,
    ShiftCell, SplitOption, TipSplit, WeekWeather, payroll_screen, rozpis_board,
};
use crate::ui::{Align, Icon, RectExt, Ui};
use sim::{Command, Shift};

fn cell_of(s: Shift) -> ShiftCell {
    match s {
        Shift::Off => ShiftCell::Off,
        Shift::Full => ShiftCell::Full,
        Shift::Early => ShiftCell::Early,
        Shift::Late => ShiftCell::Late,
    }
}

fn shift_of(c: ShiftCell) -> Shift {
    match c {
        ShiftCell::Off | ShiftCell::Leave => Shift::Off,
        ShiftCell::Full => Shift::Full,
        ShiftCell::Early => Shift::Early,
        ShiftCell::Late => Shift::Late,
    }
}

fn pref_shift_cell(p: PreferredShift) -> ShiftCell {
    match p {
        PreferredShift::Early => ShiftCell::Early,
        PreferredShift::Late => ShiftCell::Late,
        PreferredShift::Full => ShiftCell::Full,
    }
}

fn shift_key(p: PreferredShift) -> &'static str {
    match p {
        PreferredShift::Early => "rozpis.shift.early",
        PreferredShift::Late => "rozpis.shift.late",
        PreferredShift::Full => "rozpis.shift.full",
    }
}

/// A colleague by staff archetype (WorksWith / AvoidWith name the archetype).
fn colleague(g: &GameScreen, loc: &Locale, arch: &str) -> Option<String> {
    g.world
        .staff()
        .iter()
        .find(|s| s.archetype == arch && !s.owner)
        .map(|s| wv::staff_name(loc, s))
}

pub(super) fn chip_of(
    g: &GameScreen,
    loc: &Locale,
    p: &crate::content::handson::Pref,
    st: sim::PrefStatus,
) -> PrefChip {
    let day = |d: &Weekday| loc.tr_owned(&d.short_key());
    let (icon, args): (Icon, Vec<(&str, String)>) = match &p.kind {
        PrefKind::DayOff(d) => (Icon::Calendar, vec![("day", day(d))]),
        PrefKind::ShiftPref(s) => (
            match s {
                PreferredShift::Late => Icon::Moon,
                PreferredShift::Early => Icon::Sunrise,
                PreferredShift::Full => Icon::Clock,
            },
            vec![("shift", loc.tr_owned(shift_key(*s)))],
        ),
        PrefKind::WeekendOff => (Icon::Home, Vec::new()),
        PrefKind::NoBackToBack => (Icon::LinkBroken, Vec::new()),
        PrefKind::MaxNights(n) => (Icon::Minus, vec![("n", n.to_string())]),
        PrefKind::MinNights(n) => (Icon::Money, vec![("n", n.to_string())]),
        PrefKind::WorksWith(a) => (
            Icon::Heart,
            vec![("name", colleague(g, loc, a).unwrap_or_default())],
        ),
        PrefKind::AvoidWith(a) => (
            Icon::Cloud,
            vec![("name", colleague(g, loc, a).unwrap_or_default())],
        ),
    };
    let args: Vec<(&str, &str)> = args.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let portrait = match &p.kind {
        PrefKind::WorksWith(a) | PrefKind::AvoidWith(a) => colleague(g, loc, a),
        _ => None,
    };
    // the chip shows the colleague's face; the label keeps it short
    let label = match &p.kind {
        PrefKind::WorksWith(_) | PrefKind::AvoidWith(_) => portrait
            .as_deref()
            .map(|n| n.split(' ').next().unwrap_or(n).to_string())
            .unwrap_or_default(),
        _ => loc.trf(&p.kind.chip_key(), &args),
    };
    let strength = match p.strength {
        Strength::Need => loc.tr("rozpis.need"),
        Strength::Wish => loc.tr("rozpis.wish"),
    };
    PrefChip {
        icon,
        label,
        quote: format!(
            "{} — {}\n{strength}",
            loc.trf(&p.kind.chip_key(), &args),
            loc.tr(&p.quote)
        ),
        status: match st {
            sim::PrefStatus::Honoured => PrefStatus::Honoured,
            sim::PrefStatus::Violated => PrefStatus::Violated,
            sim::PrefStatus::Pending => PrefStatus::Pending,
        },
        need: p.strength == Strength::Need,
        portrait,
    }
}

/// Days on the row a violated preference is about (the amber corner dots).
fn conflicts(
    cells: &[ShiftCell; 7],
    prefs: &[(crate::content::handson::Pref, sim::PrefStatus)],
) -> [bool; 7] {
    let mut c = [false; 7];
    let on = |i: usize| !matches!(cells[i], ShiftCell::Off | ShiftCell::Leave);
    for (p, st) in prefs {
        if *st != sim::PrefStatus::Violated {
            continue;
        }
        match &p.kind {
            PrefKind::DayOff(d) => c[*d as usize] = true,
            PrefKind::ShiftPref(s) => {
                let want = pref_shift_cell(*s);
                for (i, x) in c.iter_mut().enumerate() {
                    if on(i) && cells[i] != want {
                        *x = true;
                    }
                }
            }
            PrefKind::WeekendOff => {
                for (i, x) in c.iter_mut().enumerate().skip(4).take(2) {
                    if on(i) {
                        *x = true;
                    }
                }
            }
            PrefKind::NoBackToBack => {
                for (i, x) in c.iter_mut().enumerate().skip(1) {
                    if on(i) && on(i - 1) {
                        *x = true;
                    }
                }
            }
            _ => {}
        }
    }
    c
}

/// The rozpis tab: the board fills the tablet.
pub fn rozpis_tab(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx, r: Rect) {
    let t = ui.theme.clone();
    let loc = cx.loc;
    let content = cx.content;
    let app = g.world.app().clone();
    // the board takes the whole tablet (its own header names the days)
    let body = r;
    let wd = g.world.weekday() as usize;
    let _ = &t;
    let staff: Vec<sim::Staff> = g.world.staff().to_vec();
    let mut rows = Vec::new();
    let mut ids = Vec::new();
    for s in &staff {
        let week = app.draft.get(&s.id).copied().unwrap_or([Shift::Off; 7]);
        let mut cells = week.map(cell_of);
        for (ci, c) in cells.iter_mut().enumerate() {
            let leave = s.leave_nights.iter().any(|(n, _)| {
                Weekday::ALL[((*n as usize) + 6) % 7] as usize == ci
                    && *n / 7 == g.world.night() / 7
            });
            if leave && *c == ShiftCell::Off {
                *c = ShiftCell::Leave;
            }
        }
        let name = wv::staff_name(&loc, s);
        if s.owner {
            rows.push(RozpisRow {
                name,
                prefs: Vec::new(),
                cells: [ShiftCell::Full; 7],
                conflicts: [false; 7],
                wellbeing: 100.0,
                locked: true,
            });
        } else {
            let ps = g.world.pref_status(content, s.id);
            rows.push(RozpisRow {
                name,
                prefs: ps.iter().map(|(p, st)| chip_of(g, &loc, p, *st)).collect(),
                cells,
                conflicts: conflicts(&cells, &ps),
                wellbeing: g.world.wellbeing(s.id).unwrap_or(80.0),
                locked: false,
            });
        }
        ids.push(s.id);
    }
    // Karolina first
    if let Some(k) = rows.iter().position(|r| r.locked) {
        let row = rows.remove(k);
        let id = ids.remove(k);
        rows.insert(0, row);
        ids.insert(0, id);
    }
    let cov = g.world.coverage(content);
    let coverage: [CoverageNight; 7] = std::array::from_fn(|i| CoverageNight {
        on: cov[i].on.min(255) as u8,
        suggested: cov[i].suggested.min(255) as u8,
        cups: cov[i].cups,
    });
    let fair = g.world.week_fairness();
    let weather = WeekWeather::from_score(fair);
    let wkey = match weather {
        WeekWeather::Sunny => "rozpis.week.4",
        WeekWeather::MostlySunny => "rozpis.week.3",
        WeekWeather::Cloudy => "rozpis.week.2",
        WeekWeather::CosyRain => "rozpis.week.1",
    };
    let rep = g.world.autofill_report().clone();
    let autofill_line = (rep.total > 0).then(|| {
        let mut l = loc.trf(
            "rozpis.autofill_result",
            &[
                ("kept", &rep.kept.to_string()),
                ("total", &rep.total.to_string()),
            ],
        );
        if let Some((sid, kind)) = rep.broken.first()
            && let Some(s) = g.world.staff_by_id(*sid)
        {
            let pref = crate::content::handson::Pref {
                kind: kind.clone(),
                strength: Strength::Wish,
                quote: String::new(),
            };
            let chip = chip_of(g, &loc, &pref, sim::PrefStatus::Violated);
            l.push(' ');
            l.push_str(&loc.trf(
                "rozpis.autofill_missed",
                &[
                    (
                        "name",
                        wv::staff_name(&loc, s).split(' ').next().unwrap_or(""),
                    ),
                    ("pref", &chip.label),
                    ("day", ""),
                ],
            ));
        }
        l.trim_end_matches([' ', '.']).to_string() + "."
    });
    let view = RozpisView {
        days: std::array::from_fn(|i| loc.tr_owned(&Weekday::ALL[i].short_key())),
        today: Some(wd),
        rows,
        coverage,
        fairness: FairnessView {
            score: fair,
            weather,
            label: loc.tr_owned(wkey),
            detail: autofill_line.clone().unwrap_or_else(|| loc.tr_owned(wkey)),
        },
        autofill_line,
        dirty: app.draft_dirty,
    };
    let labels = RozpisLabels {
        shift: [
            "—".to_string(),
            loc.tr_owned("rozpis.shift.full"),
            loc.tr_owned("rozpis.shift.early"),
            loc.tr_owned("rozpis.shift.late"),
            loc.tr_owned("ui.app.leave_short"),
        ],
        staff: loc.tr_owned("ui.app.staff"),
        wellbeing: loc.tr_owned("ui.rozpis.wellbeing"),
        coverage: loc.tr_owned("ui.app.coverage"),
        autofill: loc.tr_owned("rozpis.autofill"),
        clear: loc.tr_owned("rozpis.clear"),
        publish: loc.tr_owned("ui.app.publish"),
        cell_tip: loc.tr_owned("ui.app.cell_tip"),
        locked_tip: loc.tr_owned("ui.rozpis.locked"),
        coverage_tip: loc.tr_owned("ui.rozpis.coverage_tip"),
    };
    g.anchors.push((
        "rozpis.prefs",
        Rect::new(body.x, body.y + 24.0, 232.0, 56.0),
    ));
    g.anchors.push((
        "rozpis.wellbeing",
        Rect::new(body.right() - 56.0, body.y + 24.0, 50.0, 56.0),
    ));
    g.hands.tut.bump("rozpis_shown");
    let action = rozpis_board(ui, "rozpis", body, &view, &labels, &mut g.ui_state.rozpis);
    match action {
        Some(RozpisAction::SetCell { row, day, shift }) => {
            if let Some(id) = ids.get(row).copied() {
                let _ = g.command(
                    content,
                    Command::SetShift {
                        staff: id,
                        weekday: day as u8,
                        shift: shift_of(shift),
                    },
                );
            }
        }
        Some(RozpisAction::AutoFill) => {
            if let Err(e) = g.command(content, Command::AutoFillRozpis) {
                super::panels::command_error(g, &loc, &e);
            }
            g.ui_state.rozpis.flash_until = ui.time() + 1.2;
        }
        Some(RozpisAction::Clear) => {
            for (i, id) in ids.iter().enumerate() {
                if i == 0 {
                    continue;
                }
                for d in 0..7u8 {
                    let _ = g.world.apply(
                        content,
                        Command::SetShift {
                            staff: *id,
                            weekday: d,
                            shift: Shift::Off,
                        },
                    );
                }
            }
            g.drain_and_handle(content, None);
        }
        Some(RozpisAction::Publish) => match g.command(content, Command::PublishSchedule) {
            Ok(()) => {
                g.hands.tut.bump("published");
                g.toast(
                    crate::ui::ToastKind::Success,
                    Icon::Calendar,
                    loc.tr_owned("ui.app.published"),
                    None,
                )
            }
            Err(sim::CommandError::Locked) => {
                let next = (g.world.night().max(1) - 1) / 7 * 7 + 8;
                g.toast(
                    crate::ui::ToastKind::Warning,
                    Icon::Calendar,
                    loc.trf("ui.app.rozpis_weekly", &[("night", &next.to_string())]),
                    None,
                )
            }
            Err(e) => super::panels::command_error(g, &loc, &e),
        },
        Some(RozpisAction::FlashAutofill) => g.ui_state.rozpis.flash_until = ui.time() + 1.2,
        None => {}
    }
}

/// The předávka picker at closing: up to two notes for tomorrow, one tap each.
/// Returns the height it used (0 when there's nothing to pick).
pub fn handover_picker(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx, r: Rect) -> f32 {
    let content = cx.content;
    let loc = cx.loc;
    let t = ui.theme.clone();
    let night = g.world.phase() == sim::Phase::Night;
    if !night || g.world.minute() < 7 * 60 {
        return 0.0;
    }
    let sugg = g.world.handover_suggestions(content);
    if sugg.is_empty() {
        return 0.0;
    }
    let picks: Vec<String> = g.world.handover_picks().to_vec();
    ui.fonts.draw_in(
        loc.tr("ui.app.handover_pick"),
        Rect::new(r.x, r.y, r.w, 18.0),
        &t.micro(),
        Align::Left,
    );
    g.anchors.push((
        "app.handover",
        Rect::new(r.x, r.y, r.w, 22.0 + sugg.len() as f32 * 30.0),
    ));
    let mut y = r.y + 22.0;
    for s in &sugg {
        let rr = Rect::new(r.x, y, r.w, 26.0);
        y += 30.0;
        let on = picks.contains(&s.id);
        let text = wv::tr_args(
            &loc,
            &format!("app.handover.option.{}", s.id),
            &super::journal_ui::sys_args(&loc, &s.args),
        );
        let resp = ui.interact(ui.id(("handover_pick", s.id.as_str())), rr);
        fill_rrect(
            rr,
            8.0,
            if on {
                with_a(t.indigo, 0.3)
            } else {
                Color::new(1.0, 1.0, 1.0, 0.04 + 0.04 * resp.hover_t)
            },
        );
        if on {
            stroke_rrect(rr, 8.0, 1.0, t.indigo_300);
        }
        crate::ui::draw_icon(
            if on { Icon::Check } else { Icon::Note },
            vec2(rr.x + 13.0, rr.center_y()),
            13.0,
            if on { t.emerald_400 } else { t.text_dim },
        );
        ui.fonts.draw_in(
            &text,
            Rect::new(rr.x + 26.0, rr.y, rr.w - 30.0, rr.h),
            &t.small().color(t.text),
            Align::Left,
        );
        if resp.clicked {
            let mut p = picks.clone();
            if on {
                p.retain(|x| x != &s.id);
            } else {
                if p.len() >= 2 {
                    p.remove(0);
                }
                p.push(s.id.clone());
            }
            if g.command(content, Command::PickHandover { picks: p })
                .is_ok()
            {
                g.hands.tut.bump("handover_written");
            }
        }
    }
    y - r.y
}

/// The digest's insights (≤ 4) as rows with a deep link each. Returns the height used.
pub fn insights(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx, r: Rect) -> f32 {
    let loc = cx.loc;
    let t = ui.theme.clone();
    let Some(s) = g.world.last_summary().cloned() else {
        return 0.0;
    };
    if s.insights.is_empty() {
        return 0.0;
    }
    let mut y = r.y;
    for (i, ins) in s.insights.iter().take(4).enumerate() {
        let text = wv::tr_args(
            &loc,
            &ins.key,
            &super::journal_ui::sys_args(&loc, &ins.args),
        );
        let link = loc.tr_owned(&link_key(ins.link));
        let lw = ui.fonts.measure(&link, &t.key()) + 22.0;
        let lines = ui
            .fonts
            .wrap(&text, r.w - lw - 34.0, &t.small())
            .len()
            .clamp(1, 3);
        let h = 12.0 + lines as f32 * 16.0;
        let rr = Rect::new(r.x, y, r.w, h);
        y += h + 6.0;
        let resp = ui.interact(ui.id(("insight", i)), rr);
        fill_rrect(
            rr,
            8.0,
            Color::new(1.0, 1.0, 1.0, 0.04 + 0.05 * resp.hover_t),
        );
        crate::ui::draw_icon(
            link_icon(ins.link),
            vec2(rr.x + 14.0, rr.y + 14.0),
            13.0,
            t.violet_400,
        );
        ui.fonts.draw_wrapped(
            &text,
            Rect::new(rr.x + 28.0, rr.y + 5.0, rr.w - lw - 34.0, h),
            &t.small().color(t.text),
            Align::Left,
            Some(3),
        );
        ui.fonts.draw_in(
            &format!("{link} ›"),
            Rect::new(rr.right() - lw, rr.y, lw - 8.0, 24.0),
            &t.key().color(t.indigo_300),
            Align::Right,
        );
        if resp.clicked {
            g.follow_link(ins.link);
        }
    }
    y - r.y
}

pub fn link_key(l: AppLink) -> String {
    let id = match l {
        AppLink::Rozpis => "rozpis",
        AppLink::RecipeBook => "book",
        AppLink::Pantry => "pantry",
        AppLink::Build => "build",
        AppLink::Duties => "duties",
        AppLink::Money => "money",
        AppLink::Journal => "journal",
        AppLink::Board => "board",
    };
    format!("ui.link.{id}")
}

fn link_icon(l: AppLink) -> Icon {
    match l {
        AppLink::Rozpis => Icon::Calendar,
        AppLink::RecipeBook | AppLink::Pantry => Icon::Book,
        AppLink::Build => Icon::Build,
        AppLink::Duties => Icon::Priorities,
        AppLink::Money => Icon::Money,
        AppLink::Journal => Icon::Camera,
        AppLink::Board => Icon::Megaphone,
    }
}

fn ui_split(s: crate::content::handson::TipSplit) -> TipSplit {
    match s {
        crate::content::handson::TipSplit::Cafe => TipSplit::Cafe,
        crate::content::handson::TipSplit::Half => TipSplit::Half,
        crate::content::handson::TipSplit::Crew => TipSplit::Crew,
    }
}

fn sim_split(s: TipSplit) -> crate::content::handson::TipSplit {
    match s {
        TipSplit::Cafe => crate::content::handson::TipSplit::Cafe,
        TipSplit::Half => crate::content::handson::TipSplit::Half,
        TipSplit::Crew => crate::content::handson::TipSplit::Crew,
    }
}

/// The weekly payroll basis as a small decision screen (Panel::Payroll).
pub fn payroll_panel(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let content = cx.content;
    let loc = cx.loc;
    let s = ui.screen();
    let Some(week) = g.world.app().payroll.last().cloned() else {
        g.panel = Some(Panel::App);
        g.ui_state.app_tab = 6;
        return;
    };
    let rules = content.app.payroll_rules.clone();
    let split = ui_split(g.world.tip_split(content));
    let share = |sp: TipSplit| {
        rules
            .as_ref()
            .and_then(|r| r.splits.iter().find(|o| ui_split(o.split) == sp))
            .map_or(0.5, |o| o.crew_share)
    };
    let crew_hours: f32 = week.rows.iter().map(|r| r.hours).sum::<f32>().max(1.0);
    let to_crew = (week.tips as f32 * share(split)).round() as i64;
    let rows: Vec<PayRow> = week
        .rows
        .iter()
        .map(|row| {
            let st = g.world.staff_by_id(row.staff);
            let toil = st.map_or(0.0, |s| s.toil_hours);
            PayRow {
                name: st
                    .map(|s| wv::staff_name(&loc, s))
                    .unwrap_or(row.name.clone()),
                role: st
                    .map(|s| wv::staff_role(&loc, content, s))
                    .unwrap_or_default(),
                hours: loc.trf(
                    "ui.unit.hours_short",
                    &[("h", &format_decimal(loc.lang, row.hours, 0))],
                ),
                wage: format_money(loc.lang, row.wage),
                tips: format_money(
                    loc.lang,
                    (to_crew as f32 * row.hours / crew_hours).round() as i64,
                ),
                toil: (toil >= 4.0).then(|| {
                    loc.trf(
                        "ui.payroll.toil_owed",
                        &[("hours", &format_decimal(loc.lang, toil, 0))],
                    )
                }),
                toil_granted: g.ui_state.toil_granted.contains(&row.staff),
            }
        })
        .collect();
    let options: [SplitOption; 3] = TipSplit::ALL.map(|sp| {
        let crew = (week.tips as f32 * share(sp)).round() as i64;
        let cafe = week.tips - crew;
        let id = match sp {
            TipSplit::Cafe => "cafe",
            TipSplit::Half => "half",
            TipSplit::Crew => "crew",
        };
        SplitOption {
            title: loc.tr_owned(&format!("app.payroll.split.{id}")),
            detail: loc.trf(
                &format!("ui.payroll.detail.{id}"),
                &[
                    ("cafe", &format_money(loc.lang, cafe)),
                    ("crew", &format_money(loc.lang, crew)),
                ],
            ),
            effect: loc.tr_owned(&format!("ui.payroll.effect.{id}")),
        }
    });
    let star = g
        .world
        .systems()
        .tool
        .bonus
        .filter(|(w, _)| *w == week.week)
        .and_then(|(_, sid)| week.rows.iter().position(|r| r.staff == sid));
    let ins: Vec<Insight> = g
        .world
        .last_summary()
        .map(|sm| {
            sm.insights
                .iter()
                .take(4)
                .map(|i| Insight {
                    icon: link_icon(i.link),
                    text: wv::tr_args(&loc, &i.key, &super::journal_ui::sys_args(&loc, &i.args)),
                    link: loc.tr_owned(&link_key(i.link)),
                })
                .collect()
        })
        .unwrap_or_default();
    let sent = g.world.payroll_sent(week.week);
    let view = PayrollView {
        week: loc.trf("ui.app.week", &[("n", &week.week.to_string())]),
        rows,
        wages_total: format_money(loc.lang, week.total),
        tips_total: format_money(loc.lang, week.tips),
        split,
        options,
        star,
        insights: ins,
        sent,
    };
    let bonus = rules.as_ref().map_or(200, |r| r.star_bonus) as i64;
    let labels = PayrollLabels {
        title: loc.tr_owned("ui.app.tab.payroll"),
        cols: [
            loc.tr_owned("ui.app.staff"),
            loc.tr_owned("ui.app.hours"),
            loc.tr_owned("ui.app.wage"),
            loc.tr_owned("ui.summary.tips"),
        ],
        total: loc.tr_owned("ui.payroll.total"),
        split_head: loc.tr_owned("app.payroll.split.title"),
        star_head: loc.tr_owned("ui.payroll.star_head"),
        star_tip: loc.trf(
            "ui.payroll.star_tip",
            &[("amount", &format_money(loc.lang, bonus))],
        ),
        grant: loc.tr_owned("app.payroll.grant"),
        granted: loc.tr_owned("ui.payroll.granted"),
        insights_head: loc.tr_owned("ui.payroll.insights"),
        send: loc.tr_owned("app.payroll.send"),
        sent: loc.tr_owned("ui.payroll.sent"),
    };
    let size = vec2(
        PAYROLL_SIZE.x.min(s.w - 24.0),
        PAYROLL_SIZE.y.min(s.h - 150.0),
    );
    let r = Rect::new(
        (s.w - size.x) / 2.0,
        (84.0 + (s.h - 60.0 - 84.0 - size.y) / 2.0).max(84.0),
        size.x,
        size.y,
    );
    g.anchors.push(("panel", r));
    ui.region(r);
    match payroll_screen(ui, "payroll", r, &view, &labels) {
        Some(PayrollAction::SetSplit(sp)) => {
            let _ = g.command(
                content,
                Command::SetTipSplit {
                    split: sim_split(sp),
                },
            );
        }
        Some(PayrollAction::Star(Some(i))) => {
            if let Some(row) = week.rows.get(i) {
                let _ = g.command(content, Command::PayBonus { staff: row.staff });
            }
        }
        Some(PayrollAction::Star(None)) => {}
        Some(PayrollAction::GrantToil(i)) => {
            if let Some(row) = week.rows.get(i) {
                // the first night they're on next week becomes a night off
                let wd = g.world.weekday() as usize;
                let draft = g.world.app().draft.get(&row.staff).copied();
                let day = (1..=7)
                    .map(|k| (wd + k) % 7)
                    .find(|d| draft.is_some_and(|w| w[*d] != Shift::Off))
                    .unwrap_or((wd + 1) % 7);
                match g.command(
                    content,
                    Command::GrantToil {
                        staff: row.staff,
                        weekday: day as u8,
                    },
                ) {
                    Ok(()) => g.ui_state.toil_granted.push(row.staff),
                    Err(e) => super::panels::command_error(g, &loc, &e),
                }
            }
        }
        Some(PayrollAction::Insight(i)) => {
            if let Some(l) = g
                .world
                .last_summary()
                .and_then(|sm| sm.insights.get(i).map(|x| x.link))
            {
                g.follow_link(l);
            }
        }
        Some(PayrollAction::Send) => {
            if let Err(e) = g.command(content, Command::SendPayroll { week: week.week }) {
                super::panels::command_error(g, &loc, &e);
            }
        }
        Some(PayrollAction::Close) => g.panel = None,
        None => {}
    }
}
