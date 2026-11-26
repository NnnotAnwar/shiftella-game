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
    /// "Replay tutorial" was clicked (the game restarts its coach marks).
    pub replay_tutorial: bool,
}

/// Preferred size of the panel.
pub const PANEL_SIZE: Vec2 = Vec2::new(500.0, 592.0);

/// The panel's rect on a screen: centred, as tall as [`PANEL_SIZE`] or the screen allows
/// (a large UI scale on a 720p screen leaves ~550 pt: the rows tighten up to fit).
pub fn panel_rect(screen: Vec2) -> Rect {
    let h = PANEL_SIZE.y.min(screen.y - 24.0).max(420.0);
    Rect::new(
        (screen.x - PANEL_SIZE.x) / 2.0,
        ((screen.y - h) / 2.0).max(12.0),
        PANEL_SIZE.x,
        h,
    )
}

/// Draws the settings panel into `r` (use [`PANEL_SIZE`]) and edits `s` in place.
/// The caller saves (`Settings::save`) and applies the values when `changed`.
pub fn settings_panel(ui: &mut Ui, loc: &Locale, s: &mut Settings, r: Rect) -> SettingsResponse {
    let mut out = SettingsResponse::default();
    let t = ui.theme.clone();
    ui.glass(r, &Glass::panel().solid().radius(14.0));
    let (body, closed) = ui.panel_header(r, loc.tr("ui.settings.title"), Some("Esc"), None);
    out.close |= closed;

    // a short screen (large UI scale on 720p) tightens the rows instead of running off it
    let k = (r.h / PANEL_SIZE.y).min(1.0);
    let row_h = (32.0 * k).max(27.0);
    let row_gap = (4.0 * k).max(1.0);
    let section_h = (26.0 * k).max(21.0);
    let mut y = body.y + 10.0 * k;
    let x0 = body.x + 18.0;
    let w = body.w - 36.0;
    let label_st = t.body();
    let value_st = t.small_medium().color(t.text_dim);

    // a labelled row: returns the control rect. The label takes what it needs (120–184 pt)
    // so a segmented control gets the rest; a toggle row (`control_w`) leaves the label
    // almost the whole width.
    let row = |ui: &mut Ui, y: &mut f32, label: &str, control_w: Option<f32>| -> Rect {
        let rr = Rect::new(x0, *y, w, row_h);
        let label_w = match control_w {
            Some(cw) => w - cw - 8.0,
            None => (ui.fonts.measure(label, &label_st) + 16.0).clamp(120.0, 184.0),
        };
        ui.fonts.draw_fit(
            label,
            Rect::new(rr.x, rr.y, label_w - 8.0, rr.h),
            &label_st,
            Align::Left,
            0.85,
        );
        *y += row_h + row_gap;
        Rect::new(rr.x + label_w, rr.y, rr.w - label_w, rr.h)
    };
    let section = |ui: &mut Ui, y: &mut f32, title: &str| {
        ui.section(Rect::new(x0, *y + 4.0 * k, w, 16.0), title);
        *y += section_h;
    };

    // ── language
    section(ui, &mut y, loc.tr("mainmenu.settings.section.language"));
    let c = row(ui, &mut y, loc.lang.native_name(), None);
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
        let c = row(ui, &mut y, loc.tr(label), None);
        let (val, sl) = c.split_right(44.0, 8.0);
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
    let c = row(ui, &mut y, loc.tr("ui.settings.ui_scale"), None);
    let (val, sl) = c.split_right(44.0, 8.0);
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
    let c = row(ui, &mut y, loc.tr("ui.settings.fullscreen"), Some(TOGGLE_W));
    if ui.toggle("set_full", c, &mut s.fullscreen).changed {
        out.changed = true;
    }

    // ── story
    section(ui, &mut y, loc.tr("mainmenu.settings.section.story"));
    let c = row(ui, &mut y, loc.tr("mainmenu.settings.text_speed"), None);
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
    let c = row(
        ui,
        &mut y,
        loc.tr("mainmenu.settings.auto_advance"),
        Some(TOGGLE_W),
    );
    if ui.toggle("set_auto", c, &mut s.auto_advance).changed {
        out.changed = true;
    }
    let c = row(ui, &mut y, loc.tr("ui.tutorial.setting"), None);
    let (tog, rest) = c.split_right(52.0, 8.0);
    if ui.toggle("set_tutorial", tog, &mut s.show_tutorial).changed {
        out.changed = true;
    }
    let replay = Button::new(loc.tr("ui.tutorial.replay"))
        .ghost()
        .icon(Icon::Undo);
    let rw = ui.button_width(&replay).min(rest.w);
    if ui
        .button(
            "set_tutorial_replay",
            Rect::new(rest.x, rest.y + 3.0, rw, rest.h - 6.0),
            &replay,
        )
        .clicked
    {
        s.show_tutorial = true;
        out.changed = true;
        out.replay_tutorial = true;
    }
    let c = row(
        ui,
        &mut y,
        loc.tr("ui.settings.pause_on_events"),
        Some(TOGGLE_W),
    );
    if ui.toggle("set_pause", c, &mut s.pause_on_events).changed {
        out.changed = true;
    }
    ui.fonts.draw_fit(
        loc.tr("mainmenu.settings.pause_hint"),
        Rect::new(x0, y - 6.0, w, 14.0),
        &t.small(),
        Align::Left,
        0.9,
    );
    y += 8.0 * k;
    let c = row(
        ui,
        &mut y,
        loc.tr("ui.settings.relaxed_minigames"),
        Some(TOGGLE_W),
    );
    if ui
        .toggle("set_relaxed", c, &mut s.relaxed_minigames)
        .changed
    {
        out.changed = true;
    }

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

/// Width a toggle row keeps for its switch.
const TOGGLE_W: f32 = 52.0;

fn val_rect(r: Rect) -> Rect {
    r.inset_xy(0.0, 5.0)
}
