//! Side panels: Log (L) and the shared command-error toast. The crew panel (C) lives in
//! `crew_ui.rs`, the duties board (H) in `duties.rs`.

use macroquad::prelude::*;

use super::world_view as wv;
use super::{Ctx, GameScreen, Panel};
use crate::settings::Locale;
use crate::ui::draw::fill_rrect;
use crate::ui::{Align, Glass, Icon, RectExt, Ui};

/// Shows a failed command as a short warning toast.
pub fn command_error(g: &mut GameScreen, loc: &Locale, e: &str) {
    // a bot button that can't act now says why (`sim::tool::handle`)
    let key = if e.contains("night only") {
        "ui.error.night_only"
    } else if e.contains("nobody can come in") {
        "ui.error.nobody_free"
    } else if e.contains("nobody free to cover") || e.contains("nobody is late") {
        "ui.error.nobody_cover"
    } else if e.contains("no free seat") {
        "ui.error.no_seat"
    } else if e.contains("no drone dock") {
        "ui.error.no_dock"
    } else if e.starts_with("NotEnoughMoney") {
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
    let (body, close) = ui.panel_header(
        pr,
        loc.tr("ui.log.title"),
        Some(&crate::settings::keys::label(
            crate::settings::keys::Action::Log,
        )),
        None,
    );
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
