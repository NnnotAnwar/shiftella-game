//! The weekly "podklady pro mzdy" as a small, warm decision screen (design §13.7):
//! the crew's hours, wages and tips; the tip split (All to the café · Half · All to the
//! crew) with real numbers; a star of the week; náhradní volno rows with "grant a night
//! off"; and last night's digest insights with deep links (§13.6). Never failable.

use macroquad::prelude::*;

use crate::ui::draw::{
    self, Paint, fill_circle, fill_rrect, mix, shadow_rrect, stroke_circle, stroke_rrect, with_a,
};
use crate::ui::layout::RectExt;
use crate::ui::{Align, Button, Glass, Icon, TextStyle, Ui, UiSound, Weight, draw_icon};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TipSplit {
    Cafe,
    #[default]
    Half,
    Crew,
}

impl TipSplit {
    pub const ALL: [TipSplit; 3] = [TipSplit::Cafe, TipSplit::Half, TipSplit::Crew];
}

#[derive(Clone, Debug)]
pub struct PayRow {
    pub name: String,
    pub role: String,
    /// Already formatted ("32 h", "4 280 Kč").
    pub hours: String,
    pub wage: String,
    /// Their tips under the current split.
    pub tips: String,
    /// Náhradní volno owed ("6 h owed").
    pub toil: Option<String>,
    pub toil_granted: bool,
}

/// One tip-split choice.
#[derive(Clone, Debug)]
pub struct SplitOption {
    pub title: String,
    /// The real numbers ("1 240 Kč to the debt").
    pub detail: String,
    /// The gentle effect ("crew mood +4 next week").
    pub effect: String,
}

/// A digest insight with a deep link into the app ("8 wishes, 5 perfect" → Recipe book).
#[derive(Clone, Debug)]
pub struct Insight {
    pub icon: Icon,
    pub text: String,
    pub link: String,
}

#[derive(Clone, Debug)]
pub struct PayrollView {
    /// "Week 2 · 14–20 Nov".
    pub week: String,
    pub rows: Vec<PayRow>,
    pub wages_total: String,
    pub tips_total: String,
    pub split: TipSplit,
    pub options: [SplitOption; 3],
    pub star: Option<usize>,
    pub insights: Vec<Insight>,
    pub sent: bool,
}

#[derive(Clone, Debug, Default)]
pub struct PayrollLabels {
    pub title: String,
    /// Column heads: name, hours, wage, tips.
    pub cols: [String; 4],
    pub total: String,
    pub split_head: String,
    pub star_head: String,
    /// "+200 Kč bonus · mood +10" (tooltip on the star buttons).
    pub star_tip: String,
    pub grant: String,
    pub granted: String,
    pub insights_head: String,
    pub send: String,
    pub sent: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayrollAction {
    SetSplit(TipSplit),
    /// Toggle the star of the week (None = no bonus).
    Star(Option<usize>),
    GrantToil(usize),
    /// Follow insight `i`'s deep link.
    Insight(usize),
    Send,
    Close,
}

pub const PAYROLL_SIZE: Vec2 = Vec2::new(760.0, 500.0);

pub fn payroll_screen(
    ui: &mut Ui,
    key: impl std::hash::Hash,
    r: Rect,
    v: &PayrollView,
    l: &PayrollLabels,
) -> Option<PayrollAction> {
    let t = ui.theme.clone();
    let id = ui.id(key);
    ui.glass(r, &Glass::panel().solid());
    let (body, closed) = ui.panel_header(r, &l.title, None, Some(&v.week));
    let mut action = closed.then_some(PayrollAction::Close);
    let inner = body.inset_ltrb(14.0, 12.0, 14.0, 14.0);
    let side_w = 270.0_f32.min(inner.w * 0.42);
    let (side, table) = inner.split_right(side_w, 16.0);

    // --- the crew table ----------------------------------------------------
    let cols = [table.w - 58.0 - 88.0 - 80.0 - 34.0, 58.0, 88.0, 80.0];
    let mut x = table.x;
    let head = table.with_h(18.0);
    for (i, c) in l.cols.iter().enumerate() {
        let align = if i == 0 { Align::Left } else { Align::Right };
        ui.fonts.draw_in(
            &c.to_uppercase(),
            Rect::new(x + 4.0, head.y, cols[i] - 8.0, head.h),
            &t.micro(),
            align,
        );
        x += cols[i];
    }
    // rows with owed toil get a third line for the "grant a night off" button
    let star_head = Rect::new(table.right() - 36.0, head.y - 2.0, 26.0, 22.0);
    let sresp = ui.interact(id.with("star_head"), star_head);
    draw_icon(Icon::Star, star_head.center(), 11.0, t.text_disabled);
    ui.tooltip(&sresp, &l.star_head);
    let row_h = |row: &PayRow| if row.toil.is_some() { 66.0 } else { 46.0 };
    let rows_view = Rect::new(
        table.x,
        head.bottom() + 6.0,
        table.w,
        table.h - 18.0 - 6.0 - 40.0,
    );
    let content_h: f32 = v.rows.iter().map(|r| row_h(r) + 4.0).sum();
    let sa = ui.begin_scroll(id.with("rows"), rows_view, content_h);
    let mut ry = sa.top();
    for (ri, row) in v.rows.iter().enumerate() {
        let rr = Rect::new(table.x, ry, table.w - 6.0, row_h(row));
        ry += rr.h + 4.0;
        if !sa.visible(rr.y, rr.h) {
            continue;
        }
        let star = v.star == Some(ri);
        fill_rrect(
            rr,
            10.0,
            if star {
                Paint::horizontal(rr, with_a(t.amber, 0.12), with_a(t.amber, 0.03))
            } else {
                Paint::Solid(Color::new(1.0, 1.0, 1.0, 0.035))
            },
        );
        if star {
            stroke_rrect(rr, 10.0, 1.0, with_a(t.amber, 0.3));
        }
        ui.avatar(vec2(rr.x + 20.0, rr.y + 23.0), 28.0, &row.name, None, false);
        let nx = rr.x + 40.0;
        let nw = cols[0] - 44.0;
        ui.fonts.draw_in(
            &row.name,
            Rect::new(nx, rr.y + 5.0, nw, 18.0),
            &t.strong().size(12.5),
            Align::Left,
        );
        // second line: role, or the toil row with its button
        let sub = match &row.toil {
            Some(toil) => format!("{} · {toil}", row.role),
            None => row.role.clone(),
        };
        ui.fonts.draw_in(
            &sub,
            Rect::new(nx, rr.y + 24.0, nw, 16.0),
            &t.small().color(if row.toil.is_some() {
                t.amber
            } else {
                t.text_dim
            }),
            Align::Left,
        );
        if row.toil.is_some() {
            let bl = if row.toil_granted {
                &l.granted
            } else {
                &l.grant
            };
            let bs = TextStyle::new(
                11.0,
                Weight::SemiBold,
                if row.toil_granted {
                    t.emerald_400
                } else {
                    t.indigo_300
                },
            );
            let bw = (ui.fonts.measure(bl, &bs) + 30.0).min(rr.right() - 40.0 - nx);
            let br = Rect::new(nx, rr.y + 43.0, bw, 18.0);
            let resp = ui.interact(id.with(("toil", ri)), br);
            fill_rrect(
                br,
                9.0,
                if row.toil_granted {
                    with_a(t.emerald, 0.14)
                } else {
                    with_a(t.indigo, 0.18 + 0.12 * resp.hover_t)
                },
            );
            draw_icon(
                if row.toil_granted {
                    Icon::Check
                } else {
                    Icon::Moon
                },
                vec2(br.x + 11.0, br.center_y()),
                11.0,
                bs.color,
            );
            ui.fonts
                .draw_in(bl, br.inset_ltrb(20.0, 0.0, 8.0, 0.0), &bs, Align::Left);
            if resp.clicked && !row.toil_granted {
                ui.sound(UiSound::Click);
                action = Some(PayrollAction::GrantToil(ri));
            }
        }
        let mut cx = rr.x + cols[0];
        for (ci, val) in [&row.hours, &row.wage, &row.tips].iter().enumerate() {
            let color = if ci == 2 { t.amber } else { t.text };
            ui.fonts.draw_in(
                val,
                Rect::new(cx, rr.y, cols[ci + 1] - 8.0, 46.0),
                &t.number().color(color),
                Align::Right,
            );
            cx += cols[ci + 1];
        }
        // star of the week toggle
        let sr = Rect::new(rr.right() - 30.0, rr.y + 10.0, 26.0, 26.0);
        let resp = ui.interact(id.with(("star", ri)), sr);
        if resp.hover_t > 0.01 {
            fill_circle(
                sr.center(),
                12.0,
                Color::new(1.0, 1.0, 1.0, 0.07 * resp.hover_t),
            );
        }
        if star {
            draw::glow_circle(sr.center(), 16.0, with_a(t.amber, 0.45));
        }
        draw_icon(
            Icon::Star,
            sr.center(),
            15.0,
            if star {
                t.amber
            } else {
                mix(t.text_disabled, t.amber, resp.hover_t * 0.6)
            },
        );
        ui.tooltip(&resp, &l.star_tip);
        if resp.clicked {
            ui.sound(UiSound::Toggle);
            action = Some(PayrollAction::Star(if star { None } else { Some(ri) }));
        }
    }
    ui.end_scroll(sa);
    // totals
    let tr = Rect::new(table.x, table.bottom() - 34.0, table.w - 6.0, 34.0);
    fill_rrect(tr, 10.0, Color::new(0.03, 0.02, 0.09, 0.32));
    ui.fonts.draw_in(
        &l.total.to_uppercase(),
        Rect::new(tr.x + 12.0, tr.y, cols[0], tr.h),
        &t.micro(),
        Align::Left,
    );
    let wx = tr.x + cols[0] + cols[1];
    ui.fonts.draw_in(
        &v.wages_total,
        Rect::new(wx, tr.y, cols[2] - 8.0, tr.h),
        &t.number(),
        Align::Right,
    );
    ui.fonts.draw_in(
        &v.tips_total,
        Rect::new(wx + cols[2], tr.y, cols[3] - 8.0, tr.h),
        &t.number().color(t.amber),
        Align::Right,
    );

    // --- side: tip split, insights, send ----------------------------------
    let mut y = side.y;
    ui.section(Rect::new(side.x, y, side.w, 14.0), &l.split_head);
    y += 22.0;
    for (i, opt) in v.options.iter().enumerate() {
        let which = TipSplit::ALL[i];
        let on = v.split == which;
        let dh = ui
            .fonts
            .wrapped_height(&opt.detail, side.w - 46.0, &t.small(), Some(2));
        let cr = Rect::new(side.x, y, side.w, 24.0 + dh + 22.0);
        y += cr.h + 6.0;
        let resp = ui.interact(id.with(("split", i)), cr);
        let k = ui.ease_value(id.with(("split_on", i)), if on { 1.0 } else { 0.0 }, 0.07);
        if k > 0.01 {
            shadow_rrect(
                cr,
                10.0,
                draw::Shadow::new(0.0, 2.0, 12.0, -1.0, with_a(t.indigo, 0.3 * k)),
            );
        }
        fill_rrect(
            cr,
            10.0,
            Paint::horizontal(
                cr,
                mix(
                    Color::new(1.0, 1.0, 1.0, 0.05 + 0.03 * resp.hover_t),
                    with_a(t.indigo, 0.28),
                    k,
                ),
                mix(Color::new(1.0, 1.0, 1.0, 0.03), with_a(t.violet, 0.16), k),
            ),
        );
        stroke_rrect(
            cr,
            10.0,
            1.0,
            mix(
                Color::new(1.0, 1.0, 1.0, 0.08 + 0.08 * resp.hover_t),
                with_a(t.indigo_300, 0.8),
                k,
            ),
        );
        // radio
        let rc = vec2(cr.x + 16.0, cr.y + 16.0);
        stroke_circle(rc, 6.5, 1.3, mix(t.text_dim, t.indigo_300, k));
        if k > 0.01 {
            fill_circle(rc, 3.6 * k, t.indigo_300);
        }
        ui.fonts.draw_in(
            &opt.title,
            Rect::new(cr.x + 30.0, cr.y + 7.0, cr.w - 36.0, 18.0),
            &t.strong().size(12.5),
            Align::Left,
        );
        ui.fonts.draw_wrapped(
            &opt.detail,
            Rect::new(cr.x + 30.0, cr.y + 25.0, cr.w - 40.0, dh),
            &t.small().color(t.text),
            Align::Left,
            Some(2),
        );
        ui.fonts.draw_in(
            &opt.effect,
            Rect::new(cr.x + 30.0, cr.y + 25.0 + dh, cr.w - 40.0, 14.0),
            &t.small().size(11.0).color(t.emerald_400),
            Align::Left,
        );
        if resp.clicked && !on {
            ui.sound(UiSound::Toggle);
            action = Some(PayrollAction::SetSplit(which));
        }
    }
    y += 6.0;
    let send_y = side.bottom() - 32.0;
    if !v.insights.is_empty() && y + 40.0 < send_y {
        ui.section(Rect::new(side.x, y, side.w, 14.0), &l.insights_head);
        y += 20.0;
        for (i, ins) in v.insights.iter().enumerate() {
            let ls = TextStyle::new(11.0, Weight::SemiBold, t.indigo_300);
            let lw = ui.fonts.measure(&ins.link, &ls) + 18.0;
            let th =
                ui.fonts
                    .wrapped_height(&ins.text, side.w - 26.0 - lw - 6.0, &t.small(), Some(2));
            let h = th.max(18.0) + 8.0;
            if y + h > send_y - 8.0 {
                break;
            }
            draw_icon(ins.icon, vec2(side.x + 8.0, y + 11.0), 13.0, t.text_dim);
            ui.fonts.draw_wrapped(
                &ins.text,
                Rect::new(side.x + 22.0, y + 2.0, side.w - 26.0 - lw - 6.0, th),
                &t.small().color(t.text),
                Align::Left,
                Some(2),
            );
            let lr = Rect::new(side.right() - lw, y + 2.0, lw, 18.0);
            let resp = ui.interact(id.with(("ins", i)), lr);
            fill_rrect(lr, 9.0, with_a(t.indigo, 0.16 + 0.14 * resp.hover_t));
            ui.fonts
                .draw(&ins.link, lr.x + 7.0, ui.fonts.baseline_in(lr, &ls), &ls);
            draw_icon(
                Icon::ChevronRight,
                vec2(lr.right() - 7.0, lr.center_y()),
                9.0,
                t.indigo_300,
            );
            if resp.clicked {
                ui.sound(UiSound::Click);
                action = Some(PayrollAction::Insight(i));
            }
            y += h;
        }
    }
    let send = Button::new(if v.sent { &l.sent } else { &l.send })
        .primary()
        .icon(if v.sent {
            Icon::Check
        } else {
            Icon::FileExport
        })
        .disabled(v.sent);
    if ui
        .button(
            id.with("send"),
            Rect::new(side.x, send_y, side.w, 32.0),
            &send,
        )
        .clicked
    {
        action = Some(PayrollAction::Send);
    }
    if ui.esc() {
        action = Some(PayrollAction::Close);
    }
    action
}
