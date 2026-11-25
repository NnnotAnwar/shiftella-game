//! The settings screen — a compact glass panel used by the main menu and in-game.

use macroquad::prelude::*;

use super::{Locale, Settings, TextSpeed, UI_SCALE_MAX, UI_SCALE_MIN};
use crate::i18n::Lang;
use crate::ui::{Align, Button, Glass, Icon, RectExt, Ui};

/// What happened in the panel this frame.
#[derive(Debug, Clone, Copy, Default)]
pub struct SettingsResponse {
    /// Some value changed (save + apply: volumes, `ui.set_scale`, fullscreen).
    pub changed: bool,
    /// The language changed (texts must be re-read).
    pub language_changed: bool,
    /// Back / close / Esc.
    pub close: bool,
}

/// Preferred size of the panel.
pub const PANEL_SIZE: Vec2 = Vec2::new(460.0, 486.0);

/// Draws the settings panel into `r` (use [`PANEL_SIZE`]) and edits `s` in place.
/// The caller saves (`Settings::save`) and applies the values when `changed`.
pub fn settings_panel(ui: &mut Ui, loc: &Locale, s: &mut Settings, r: Rect) -> SettingsResponse {
    let mut out = SettingsResponse::default();
    let t = ui.theme.clone();
    ui.glass(r, &Glass::panel().solid().radius(14.0));
    let (body, closed) = ui.panel_header(r, loc.tr("ui.settings.title"), Some("Esc"), None);
    out.close |= closed;

    let label_w = 168.0;
    let row_h = 34.0;
    let mut y = body.y + 10.0;
    let x0 = body.x + 18.0;
    let w = body.w - 36.0;
    let label_st = t.body();
    let value_st = t.small_medium().color(t.text_dim);

    // a labelled row: returns the control rect
    let row = |ui: &mut Ui, y: &mut f32, label: &str| -> Rect {
        let rr = Rect::new(x0, *y, w, row_h);
        ui.fonts.draw_in(
            label,
            Rect::new(rr.x, rr.y, label_w - 8.0, rr.h),
            &label_st,
            Align::Left,
        );
        *y += row_h + 4.0;
        Rect::new(rr.x + label_w, rr.y, rr.w - label_w, rr.h)
    };
    let section = |ui: &mut Ui, y: &mut f32, title: &str| {
        ui.section(Rect::new(x0, *y + 4.0, w, 16.0), title);
        *y += 26.0;
    };

    // ── language
    section(ui, &mut y, loc.tr("mainmenu.settings.section.language"));
    let c = row(ui, &mut y, loc.lang.native_name());
    let labels: Vec<&str> = Lang::ALL.iter().map(|l| l.short_label()).collect();
    let mut sel = s.lang().index();
    if ui
        .segmented("set_lang", c.inset_xy(0.0, 3.0), &labels, &mut sel)
        .changed
    {
        s.language = Some(Lang::ALL[sel]);
        out.changed = true;
        out.language_changed = true;
    }

    // ── sound
    section(ui, &mut y, loc.tr("mainmenu.settings.section.sound"));
    for (key, label, v) in [
        ("set_music", "ui.settings.volume_music", &mut s.music_volume),
        ("set_sfx", "ui.settings.volume_sfx", &mut s.sfx_volume),
    ] {
        let c = row(ui, &mut y, loc.tr(label));
        let (sl, val) = c.split_right(44.0, 8.0);
        if ui
            .slider(key, val_rect(sl), v, 0.0, 1.0, Some(0.05))
            .changed
        {
            out.changed = true;
        }
        let pct = format!("{} %", (*v * 100.0).round() as i32);
        ui.fonts.draw_in(&pct, val, &value_st, Align::Right);
    }

    // ── display
    section(ui, &mut y, loc.tr("mainmenu.settings.section.display"));
    let c = row(ui, &mut y, loc.tr("ui.settings.ui_scale"));
    let (sl, val) = c.split_right(44.0, 8.0);
    if ui
        .slider(
            "set_scale",
            val_rect(sl),
            &mut s.ui_scale,
            UI_SCALE_MIN,
            UI_SCALE_MAX,
            Some(0.05),
        )
        .changed
    {
        out.changed = true;
    }
    let pct = format!("{} %", (s.ui_scale * 100.0).round() as i32);
    ui.fonts.draw_in(&pct, val, &value_st, Align::Right);
    let c = row(ui, &mut y, loc.tr("ui.settings.fullscreen"));
    if ui.toggle("set_full", c, &mut s.fullscreen).changed {
        out.changed = true;
    }

    // ── story
    section(ui, &mut y, loc.tr("mainmenu.settings.section.story"));
    let c = row(ui, &mut y, loc.tr("mainmenu.settings.text_speed"));
    let labels: Vec<&str> = TextSpeed::ALL
        .iter()
        .map(|x| loc.tr(x.label_key()))
        .collect();
    let mut sel = s.text_speed.index();
    if ui
        .segmented("set_speed", c.inset_xy(0.0, 3.0), &labels, &mut sel)
        .changed
    {
        s.text_speed = TextSpeed::ALL[sel];
        out.changed = true;
    }
    let c = row(ui, &mut y, loc.tr("mainmenu.settings.auto_advance"));
    if ui.toggle("set_auto", c, &mut s.auto_advance).changed {
        out.changed = true;
    }
    let c = row(ui, &mut y, loc.tr("ui.settings.pause_on_events"));
    if ui.toggle("set_pause", c, &mut s.pause_on_events).changed {
        out.changed = true;
    }
    ui.fonts.draw_in(
        loc.tr("mainmenu.settings.pause_hint"),
        Rect::new(x0, y - 8.0, w - 48.0, 14.0),
        &t.small(),
        Align::Left,
    );

    // ── footer
    let back = Button::new(loc.tr("ui.common.back"))
        .secondary()
        .icon(Icon::Back)
        .shortcut("Esc");
    let bw = ui.button_width(&back);
    let br = Rect::new(r.x + 18.0, r.bottom() - 18.0 - 28.0, bw, 28.0);
    if ui.button("set_back", br, &back).clicked {
        out.close = true;
    }
    let path_st = t.small().color(t.text_disabled);
    ui.fonts.draw_in(
        loc.tr("mainmenu.settings.saved_auto"),
        Rect::new(br.right() + 12.0, br.y, r.right() - br.right() - 30.0, br.h),
        &path_st,
        Align::Right,
    );
    if ui.esc() {
        out.close = true;
    }
    if out.changed {
        s.sanitize();
    }
    out
}

fn val_rect(r: Rect) -> Rect {
    r.inset_xy(0.0, 5.0)
}
