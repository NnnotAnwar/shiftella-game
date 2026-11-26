//! Side panels: Crew (C), Priorities (P), Log (L).

use macroquad::prelude::*;

use super::world_view::{self as wv, sim};
use super::{Ctx, GameScreen, Panel};
use crate::content::Job;
use crate::i18n::format_money;
use crate::settings::Locale;
use crate::ui::draw::{fill_rrect, stroke_rrect, with_a};
use crate::ui::{Align, Button, Glass, Icon, ListRow, Need, RectExt, TextStyle, Tone, Ui, Weight};
use sim::{AgentRef, Command, Phase, Staff, World};

/// The standard left panel rect.
pub fn left_panel(ui: &Ui, w: f32) -> Rect {
    let s = ui.screen();
    Rect::new(12.0, 88.0, w, (s.h - 88.0 - 60.0).max(200.0))
}

fn need_color(ui: &Ui, v: f32) -> Option<Color> {
    let t = &ui.theme;
    Some(if v < 25.0 {
        t.red_400
    } else if v < 50.0 {
        t.amber
    } else {
        t.emerald_400
    })
}

fn sorted_staff(w: &World, sort: usize) -> Vec<&Staff> {
    let mut v: Vec<&Staff> = w.staff().iter().collect();
    match sort {
        1 => v.sort_by(|a, b| a.energy.total_cmp(&b.energy)),
        2 => v.sort_by(|a, b| a.mood.total_cmp(&b.mood)),
        _ => v.sort_by_key(|s| (!s.owner, s.name.clone())),
    }
    v
}

pub fn crew(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let t = ui.theme.clone();
    let loc = cx.loc;
    let content = cx.content;
    let pr = left_panel(ui, 320.0);
    g.anchors.push(("panel", pr));
    ui.glass(pr, &Glass::panel());
    let on = g.world.staff().iter().filter(|s| s.on_duty()).count();
    let sub = loc.trf(
        "ui.crew.on_shift",
        &[
            ("on", &on.to_string()),
            ("total", &g.world.staff().len().to_string()),
        ],
    );
    let (content_r, close) = ui.panel_header(pr, loc.tr("ui.crew.title"), Some("C"), Some(&sub));
    if close {
        g.panel = None;
        return;
    }
    let inner = content_r.inset_xy(8.0, 8.0);
    // sort
    let (top, rest) = inner.split_top(26.0, 6.0);
    let opts = [
        loc.tr("ui.crew.sort_name"),
        loc.tr("ui.crew.sort_energy"),
        loc.tr("ui.crew.sort_mood"),
    ];
    ui.segmented("crew_sort", top, &opts, &mut g.ui_state.crew_sort);

    let staff: Vec<Staff> = sorted_staff(&g.world, g.ui_state.crew_sort)
        .into_iter()
        .cloned()
        .collect();
    if g.ui_state.crew_selected.is_none()
        || !staff.iter().any(|s| Some(s.id) == g.ui_state.crew_selected)
    {
        g.ui_state.crew_selected = staff.first().map(|s| s.id);
    }
    let cands = g.world.candidates().to_vec();
    let detail_h = 262.0;
    let list_h = (rest.h - detail_h - 8.0).max(120.0);
    let (list_r, detail_r) = rest.split_top(list_h, 8.0);
    let row_h = 44.0;
    let content_h = staff.len() as f32 * (row_h + 2.0)
        + if cands.is_empty() {
            0.0
        } else {
            28.0 + cands.len() as f32 * (row_h + 2.0)
        };
    let sa = ui.begin_scroll("crew_list", list_r, content_h);
    let mut y = sa.top();
    let w = list_r.w - 8.0;
    for s in &staff {
        let r = Rect::new(list_r.x, y, w, row_h);
        y += row_h + 2.0;
        if !sa.visible(r.y, r.h) {
            continue;
        }
        let name = wv::staff_name(&loc, s);
        let doing = wv::staff_doing(&loc, s);
        let status = match s.status {
            sim::StaffStatus::OnDuty => Some(t.emerald_400),
            sim::StaffStatus::Expected { .. } => Some(t.amber),
            sim::StaffStatus::Sick | sim::StaffStatus::WalkedOut => Some(t.red_400),
            _ => None,
        };
        if ui
            .list_row(
                ("crew", s.id),
                r,
                &ListRow::new(&name)
                    .subtitle(&doing)
                    .avatar(&name, status)
                    .selected(g.ui_state.crew_selected == Some(s.id)),
            )
            .clicked
        {
            g.ui_state.crew_selected = Some(s.id);
            g.selected = Some(AgentRef::Staff(s.id));
        }
        let bx = r.right() - 92.0;
        for (k, (need, v)) in [
            (Need::Energy, s.energy),
            (Need::Mood, s.mood),
            (Need::Hunger, s.hunger),
        ]
        .iter()
        .enumerate()
        {
            ui.need_bar(
                Rect::new(bx + k as f32 * 28.0, r.center_y() - 2.0, 24.0, 4.0),
                *need,
                v / 100.0,
            );
        }
    }
    if !cands.is_empty() {
        let hr = Rect::new(list_r.x + 4.0, y + 4.0, w, 20.0);
        ui.fonts.draw_in(
            &loc.trf("ui.crew.candidates", &[("n", &cands.len().to_string())]),
            hr,
            &t.micro(),
            Align::Left,
        );
        y += 28.0;
        for c in &cands {
            let r = Rect::new(list_r.x, y, w, row_h);
            y += row_h + 2.0;
            if !sa.visible(r.y, r.h) {
                continue;
            }
            let role = wv::staff_role(&loc, content, &c.staff);
            let sub = format!(
                "{role} · {}",
                loc.trf(
                    "ui.crew.wage",
                    &[("money", &format_money(loc.lang, c.staff.wage as i64))]
                )
            );
            let b = Button::new(loc.tr("ui.crew.hire"))
                .primary()
                .icon(Icon::Plus);
            let bw = ui.button_width(&b);
            ui.list_row(
                ("cand", c.id),
                Rect::new(r.x, r.y, r.w - bw - 10.0, r.h),
                &ListRow::new(&c.staff.name)
                    .subtitle(&sub)
                    .avatar(&c.staff.name, None),
            );
            let br = Rect::new(r.right() - bw - 6.0, r.center_y() - 12.0, bw, 24.0);
            if ui.button(("hire", c.id), br, &b).clicked {
                match g.command(content, Command::Hire { candidate: c.id }) {
                    Ok(()) => g.toast(
                        crate::ui::ToastKind::Success,
                        Icon::Crew,
                        loc.trf("ui.crew.hired_toast", &[("name", &c.staff.name)]),
                        None,
                    ),
                    Err(e) => command_error(g, &loc, &e),
                }
            }
        }
    }
    ui.end_scroll(sa);

    // details
    let Some(s) = staff
        .iter()
        .find(|s| Some(s.id) == g.ui_state.crew_selected)
        .cloned()
    else {
        return;
    };
    ui.glass(detail_r, &Glass::card());
    let cr = detail_r.inset(12.0);
    let name = wv::staff_name(&loc, &s);
    ui.avatar(vec2(cr.x + 20.0, cr.y + 20.0), 40.0, &name, None, true);
    ui.fonts.draw_in(
        &name,
        Rect::new(cr.x + 50.0, cr.y + 2.0, cr.w - 50.0, 18.0),
        &t.h2(),
        Align::Left,
    );
    let sub = if s.owner {
        wv::staff_role(&loc, content, &s)
    } else {
        loc.trf(
            "ui.crew.subtitle",
            &[
                ("role", &wv::staff_role(&loc, content, &s)),
                ("age", &s.age.to_string()),
                ("wage", &format_money(loc.lang, s.wage as i64)),
            ],
        )
    };
    ui.fonts.draw_in(
        &sub,
        Rect::new(cr.x + 50.0, cr.y + 22.0, cr.w - 50.0, 16.0),
        &t.small(),
        Align::Left,
    );
    // traits
    let (mut x, mut cy) = (cr.x, cr.y + 46.0);
    for tr in &s.traits {
        let txt = loc.tr_owned(&format!("trait.{tr}.name"));
        let tone = content
            .trait_def(tr)
            .map(trait_tone)
            .unwrap_or(Tone::Neutral);
        if x + ui.chip_width(&txt, false) > cr.right() {
            x = cr.x;
            cy += 26.0;
        }
        let r = ui.chip(vec2(x, cy), &txt, tone, None);
        let resp = ui.interact(ui.id(("trait", tr.as_str())), r);
        ui.tooltip(&resp, &loc.tr_owned(&format!("trait.{tr}.desc")));
        x = r.right() + 6.0;
    }
    // needs + skills side by side
    let mut yy = cy + 32.0;
    let half = (cr.w - 12.0) / 2.0;
    let ny = yy;
    for (need, key, v) in [
        (Need::Energy, "ui.need.energy", s.energy),
        (Need::Mood, "ui.need.mood", s.mood),
        (Need::Hunger, "ui.need.hunger", s.hunger),
    ] {
        ui.label(
            Rect::new(cr.x, yy, 64.0, 14.0),
            loc.tr(key),
            &t.small().color(t.text),
            Align::Left,
        );
        ui.need_bar(
            Rect::new(cr.x + 64.0, yy + 5.0, half - 64.0, 5.0),
            need,
            v / 100.0,
        );
        yy += 18.0;
    }
    let _ = need_color(ui, 0.0);
    let mut sy = ny;
    for (i, j) in Job::ALL.iter().enumerate() {
        let x0 = cr.x + half + 12.0;
        ui.label(
            Rect::new(x0, sy - 2.0, 70.0, 14.0),
            loc.tr(j.name_key()),
            &t.small().size(11.0),
            Align::Left,
        );
        let v = s.skills[i] / 10.0;
        ui.progress(
            Rect::new(x0 + 70.0, sy + 3.0, half - 92.0, 4.0),
            v,
            t.indigo_300,
        );
        ui.label(
            Rect::new(cr.right() - 18.0, sy - 2.0, 18.0, 14.0),
            &format!("{:.0}", s.skills[i]),
            &t.number().size(11.0),
            Align::Right,
        );
        sy += 14.0;
    }
    yy = yy.max(sy) + 6.0;
    // doing + relationships
    ui.fonts.draw_in(
        &wv::staff_doing(&loc, &s),
        Rect::new(cr.x, yy, cr.w, 16.0),
        &t.small_medium().color(t.indigo_300),
        Align::Left,
    );
    yy += 18.0;
    let mut rels: Vec<(f32, String)> = g
        .world
        .staff()
        .iter()
        .filter(|o| o.id != s.id)
        .map(|o| (g.world.relationship(s.id, o.id), wv::staff_name(&loc, o)))
        .collect();
    rels.sort_by(|a, b| b.0.abs().total_cmp(&a.0.abs()));
    let mut rx = cr.x;
    for (v, n) in rels.iter().take(2) {
        let (key, tone) = if *v >= 40.0 {
            ("ui.crew.rel_friend", Tone::Ok)
        } else if *v <= -25.0 {
            ("ui.crew.rel_rival", Tone::Danger)
        } else {
            ("ui.crew.rel_neutral", Tone::Neutral)
        };
        let txt = format!("{} · {}", n.split(' ').next().unwrap_or(n), loc.tr(key));
        if rx + ui.chip_width(&txt, false) > cr.right() {
            break;
        }
        let r = ui.chip(vec2(rx, yy), &txt, tone, Some(Icon::Heart));
        rx = r.right() + 6.0;
    }
    // actions
    let br = Rect::new(cr.x, cr.bottom() - 26.0, cr.w, 26.0);
    let (a, b) = if s.owner {
        (br, Rect::new(0.0, 0.0, 0.0, 0.0))
    } else {
        br.split_left(br.w / 2.0 - 3.0, 6.0)
    };
    if ui
        .button(
            "follow",
            a,
            &Button::new(loc.tr("ui.crew.follow")).icon(Icon::Search),
        )
        .clicked
        && let Some(head) = g.scene.head_of(AgentRef::Staff(s.id), g.time)
    {
        g.cam.center = head;
        g.selected = Some(AgentRef::Staff(s.id));
    }
    if !s.owner
        && ui
            .button(
                "fire",
                b,
                &Button::new(loc.tr("ui.crew.fire"))
                    .danger()
                    .disabled(g.world.phase() != Phase::Day)
                    .tooltip(loc.tr("ui.crew.fire_day_only")),
            )
            .clicked
    {
        match g.command(content, Command::Fire { staff: s.id }) {
            Ok(()) => g.toast(
                crate::ui::ToastKind::Info,
                Icon::Crew,
                loc.trf("ui.crew.fired_toast", &[("name", &name)]),
                None,
            ),
            Err(e) => command_error(g, &loc, &e),
        }
    }
}

fn trait_tone(d: &crate::content::TraitDef) -> Tone {
    use crate::content::TraitGroup as G;
    match d.group {
        G::Positive => Tone::Ok,
        G::Negative => Tone::Danger,
        _ => Tone::Neutral,
    }
}

/// Shows a failed command as a short warning toast.
pub fn command_error(g: &mut GameScreen, loc: &Locale, e: &str) {
    let key = if e.starts_with("NotEnoughMoney") {
        "ui.error.money"
    } else if e.starts_with("WrongPhase") {
        "ui.error.phase"
    } else if e.starts_with("Occupied") || e.starts_with("OutOfBounds") {
        "ui.error.occupied"
    } else if e.starts_with("BlocksPath") {
        "ui.error.blocks_path"
    } else if e.starts_with("InUse") {
        "ui.error.in_use"
    } else if e.starts_with("NotUnlocked") {
        "ui.error.locked"
    } else if e.starts_with("NothingToUndo") {
        "ui.error.nothing_to_undo"
    } else {
        "ui.error.generic"
    };
    g.toast(
        crate::ui::ToastKind::Warning,
        Icon::Warning,
        loc.tr_owned(key),
        None,
    );
}

pub fn priorities(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let t = ui.theme.clone();
    let loc = cx.loc;
    let content = cx.content;
    let s = ui.screen();
    let staff: Vec<Staff> = sorted_staff(&g.world, 0).into_iter().cloned().collect();
    let name_w = 170.0;
    let col_w = 74.0;
    let w = name_w + col_w * 5.0 + 32.0;
    let h = (96.0 + staff.len() as f32 * 38.0 + 54.0).min(s.h - 160.0);
    let pr = Rect::new(((s.w - w) / 2.0).max(12.0), 92.0, w, h);
    g.anchors.push(("panel", pr));
    ui.glass(pr, &Glass::panel());
    let (body, close) = ui.panel_header(pr, loc.tr("ui.priorities.title"), Some("P"), None);
    if close {
        g.panel = None;
        return;
    }
    let inner = body.inset(12.0);
    let hy = inner.y;
    for (i, j) in Job::ALL.iter().enumerate() {
        let x = inner.x + name_w + i as f32 * col_w;
        let icon = match j {
            Job::Bar => Icon::Coffee,
            Job::Kitchen => Icon::ChefHat,
            Job::Register => Icon::CashRegister,
            Job::Cleaning => Icon::Broom,
            Job::Serving => Icon::Cup,
        };
        crate::ui::draw_icon(icon, vec2(x + col_w / 2.0, hy + 8.0), 16.0, t.indigo_300);
        ui.fonts
            .draw_centered(loc.tr(j.name_key()), x + col_w / 2.0, hy + 30.0, &t.key());
    }
    let list = Rect::new(
        inner.x,
        hy + 40.0,
        inner.w,
        inner.bottom() - hy - 40.0 - 34.0,
    );
    let sa = ui.begin_scroll("prio_scroll", list, staff.len() as f32 * 38.0);
    let mut y = sa.top();
    for (ri, st) in staff.iter().enumerate() {
        let rr = Rect::new(list.x, y, list.w - 8.0, 36.0);
        y += 38.0;
        if !sa.visible(rr.y, rr.h) {
            continue;
        }
        if ri % 2 == 0 {
            fill_rrect(rr, 6.0, Color::new(1.0, 1.0, 1.0, 0.025));
        }
        let name = wv::staff_name(&loc, st);
        ui.avatar(vec2(rr.x + 16.0, rr.center_y()), 24.0, &name, None, false);
        ui.fonts.draw_in(
            &name,
            Rect::new(rr.x + 34.0, rr.y, name_w - 40.0, rr.h),
            &t.strong().size(12.0),
            Align::Left,
        );
        for (ci, j) in Job::ALL.iter().enumerate() {
            let cell = Rect::new(
                rr.x + name_w + ci as f32 * col_w + 8.0,
                rr.y + 4.0,
                col_w - 16.0,
                rr.h - 8.0,
            );
            let p = st.priorities[ci];
            let resp = ui.interact(ui.id(("prio", st.id, ci)), cell);
            let skill = st.skills[ci];
            let (bg, fg) = match p {
                1 => (t.indigo, WHITE),
                2 => (with_a(t.indigo, 0.55), WHITE),
                3 => (with_a(t.indigo, 0.28), t.text),
                4 => (with_a(t.indigo, 0.14), t.text_dim),
                _ => (Color::new(1.0, 1.0, 1.0, 0.03), t.text_disabled),
            };
            fill_rrect(cell, 6.0, bg);
            if resp.hovered {
                stroke_rrect(cell, 6.0, 1.0, t.indigo_300);
            }
            let txt = if p == 0 {
                "—".to_string()
            } else {
                p.to_string()
            };
            ui.fonts.draw_in(
                &txt,
                cell,
                &TextStyle::new(13.0, Weight::SemiBold, fg).tabular(),
                Align::Center,
            );
            // skill dots under the number
            let dots = (skill / 2.5).round() as i32;
            for d in 0..dots.min(4) {
                crate::ui::draw::fill_circle(
                    vec2(cell.x + 8.0 + d as f32 * 5.0, cell.bottom() - 5.0),
                    1.5,
                    with_a(fg, 0.7),
                );
            }
            ui.tooltip(
                &resp,
                &loc.trf("ui.priorities.cell_tip", &[("n", &format!("{skill:.0}"))]),
            );
            let wheel = if resp.hovered { ui.wheel().y } else { 0.0 };
            let next = if resp.clicked || wheel > 0.0 {
                Some(match p {
                    0 => 4,
                    1 => 1,
                    n => n - 1,
                })
            } else if resp.right_clicked || wheel < 0.0 {
                Some(match p {
                    0 => 0,
                    4 => 0,
                    n => n + 1,
                })
            } else {
                None
            };
            if let Some(n) = next
                && n != p
            {
                let _ = g.command(
                    content,
                    Command::SetPriority {
                        staff: st.id,
                        job: *j,
                        priority: n,
                    },
                );
            }
        }
    }
    ui.end_scroll(sa);
    ui.fonts.draw_in(
        loc.tr("ui.priorities.legend_short"),
        Rect::new(inner.x, inner.bottom() - 24.0, inner.w, 20.0),
        &t.small(),
        Align::Left,
    );
}

pub fn log(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let t = ui.theme.clone();
    let loc = cx.loc;
    let s = ui.screen();
    let pr = Rect::new(
        s.w - 12.0 - 360.0,
        88.0,
        360.0,
        (s.h - 88.0 - 60.0).max(200.0),
    );
    ui.glass(pr, &Glass::panel());
    let (body, close) = ui.panel_header(pr, loc.tr("ui.log.title"), Some("L"), None);
    if close {
        g.panel = None;
        return;
    }
    let inner = body.inset_xy(8.0, 8.0);
    let (top, list) = inner.split_top(26.0, 8.0);
    let opts = [loc.tr("ui.log.tonight"), loc.tr("ui.log.story")];
    ui.segmented("log_filter", top, &opts, &mut g.ui_state.log_filter);

    let lines: Vec<(Icon, String, String)> = if g.ui_state.log_filter == 0 {
        g.ticker
            .iter()
            .rev()
            .map(|tk| (tk.icon, tk.time.clone(), tk.text.clone()))
            .collect()
    } else {
        let w = &g.world;
        w.event_log()
            .iter()
            .rev()
            .map(|e| {
                let args: Vec<(String, String)> = e
                    .args
                    .iter()
                    .map(|(k, v)| {
                        let v = if k == "staff"
                            && let Ok(id) = v.parse::<u32>()
                            && let Some(st) = w.staff_by_id(id)
                        {
                            wv::staff_name(&loc, st)
                        } else {
                            v.clone()
                        };
                        (k.clone(), v)
                    })
                    .collect();
                let time = match e.minute {
                    Some(m) => format!(
                        "{} · {}",
                        loc.trf("ui.hud.night", &[("night", &e.night.to_string())]),
                        wv::clock_of_minute(m)
                    ),
                    None => loc.trf("ui.hud.night", &[("night", &e.night.to_string())]),
                };
                (Icon::Note, time, wv::tr_args(&loc, &e.key, &args))
            })
            .collect()
    };
    if lines.is_empty() {
        ui.fonts.draw_in(
            loc.tr("ui.log.empty"),
            list.with_h(40.0),
            &t.small(),
            Align::Center,
        );
        return;
    }
    let style = t.small().color(t.text);
    let wrap_w = list.w - 50.0;
    let heights: Vec<f32> = lines
        .iter()
        .map(|(_, _, txt)| ui.fonts.wrap(txt, wrap_w, &style).len().max(1) as f32 * 16.0 + 22.0)
        .collect();
    let total: f32 = heights.iter().sum();
    let sa = ui.begin_scroll("log_scroll", list, total);
    let mut y = sa.top();
    for ((icon, time, txt), h) in lines.iter().zip(&heights) {
        let r = Rect::new(list.x, y, list.w - 8.0, *h - 4.0);
        y += h;
        if !sa.visible(r.y, r.h) {
            continue;
        }
        fill_rrect(r, 8.0, Color::new(1.0, 1.0, 1.0, 0.03));
        crate::ui::draw_icon(*icon, vec2(r.x + 16.0, r.y + 16.0), 14.0, t.violet_400);
        ui.fonts.draw_in(
            time,
            Rect::new(r.x + 32.0, r.y + 4.0, r.w - 36.0, 14.0),
            &t.micro(),
            Align::Left,
        );
        ui.fonts.draw_wrapped(
            txt,
            Rect::new(r.x + 32.0, r.y + 18.0, wrap_w, r.h - 18.0),
            &style,
            Align::Left,
            None,
        );
    }
    ui.end_scroll(sa);
    let _ = Panel::Log;
}
