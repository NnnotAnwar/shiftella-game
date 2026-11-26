//! The settings screen — a compact glass panel used by the main menu and in-game.
//! Three tabs: General (language, sound, display, story), Accessibility (text size,
//! colour-blind markers, relaxed mini-games) and Controls (key rebinding).

use std::cell::Cell;

use macroquad::prelude::*;

use super::keys::{self, Action};
use super::{Locale, Settings, TextSize, TextSpeed, UI_SCALE_MAX, UI_SCALE_MIN};
use crate::i18n::Lang;
use crate::ui::{Align, Button, Glass, Icon, RectExt, Tab, Ui};

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

thread_local! {
    /// The open tab (0 General, 1 Accessibility, 2 Controls) — kept while the game runs.
    static TAB: Cell<usize> = const { Cell::new(0) };
    /// Controls: the action waiting for its new key.
    static CAPTURE: Cell<Option<Action>> = const { Cell::new(None) };
    /// Controls: the last refused key (shown under the list) and when.
    static REFUSED: Cell<Option<(KeyCode, f64)>> = const { Cell::new(None) };
}

/// Opens the panel on a tab (`SHIFTELLA_SETTINGS_TAB` in screenshot runs).
pub fn set_tab(i: usize) {
    TAB.with(|t| t.set(i.min(2)));
}

/// Row metrics: a short screen (large UI scale on 720p) tightens the rows.
struct Rows {
    k: f32,
    h: f32,
    gap: f32,
    section_h: f32,
    x0: f32,
    w: f32,
}

impl Rows {
    /// A labelled row: returns the control rect. The label takes what it needs (120–184 pt)
    /// so a segmented control gets the rest; a toggle row (`control_w`) leaves the label
    /// almost the whole width.
    fn row(&self, ui: &mut Ui, y: &mut f32, label: &str, control_w: Option<f32>) -> Rect {
        let st = ui.theme.body();
        let rr = Rect::new(self.x0, *y, self.w, self.h);
        let label_w = match control_w {
            Some(cw) => self.w - cw - 8.0,
            None => (ui.fonts.measure(label, &st) + 16.0).clamp(120.0, 184.0),
        };
        ui.fonts.draw_fit(
            label,
            Rect::new(rr.x, rr.y, label_w - 8.0, rr.h),
            &st,
            Align::Left,
            0.85,
        );
        *y += self.h + self.gap;
        Rect::new(rr.x + label_w, rr.y, rr.w - label_w, rr.h)
    }

    fn section(&self, ui: &mut Ui, y: &mut f32, title: &str) {
        ui.section(Rect::new(self.x0, *y + 4.0 * self.k, self.w, 16.0), title);
        *y += self.section_h;
    }

    /// A dim hint under the row above (wraps onto a second line when it must).
    fn hint(&self, ui: &mut Ui, y: &mut f32, text: &str) {
        let st = ui.theme.small().line_height(1.3);
        let used = ui.fonts.draw_wrapped(
            text,
            Rect::new(self.x0, *y - 6.0, self.w, st.line_h() * 2.0),
            &st,
            Align::Left,
            Some(2),
        );
        *y += used.max(14.0) - 6.0 + 8.0 * self.k;
    }
}

/// Draws the settings panel into `r` (use [`PANEL_SIZE`]) and edits `s` in place.
/// The caller saves (`Settings::save`) and applies the values when `changed`.
pub fn settings_panel(ui: &mut Ui, loc: &Locale, s: &mut Settings, r: Rect) -> SettingsResponse {
    let mut out = SettingsResponse::default();
    let t = ui.theme.clone();
    ui.glass(r, &Glass::panel().solid().radius(14.0));
    let (body, closed) = ui.panel_header(r, loc.tr("ui.settings.title"), Some("Esc"), None);
    out.close |= closed;

    let k = (r.h / PANEL_SIZE.y).min(1.0);
    let rows = Rows {
        k,
        h: (32.0 * k).max(27.0),
        gap: (4.0 * k).max(1.0),
        section_h: (26.0 * k).max(21.0),
        x0: body.x + 18.0,
        w: body.w - 36.0,
    };
    // ── tabs
    let mut tab = TAB.with(Cell::get);
    let tabs = [
        Tab::new(loc.tr("access.tab.general")),
        Tab::new(loc.tr("access.tab.access")),
        Tab::new(loc.tr("access.tab.controls")),
    ];
    let tab_h = (32.0 * k).max(28.0);
    if ui
        .tabs(
            "set_tabs",
            Rect::new(rows.x0 - 4.0, body.y + 2.0, rows.w + 8.0, tab_h),
            &tabs,
            &mut tab,
        )
        .changed
    {
        TAB.with(|c| c.set(tab));
        CAPTURE.with(|c| c.set(None));
    }
    let mut y = body.y + 2.0 + tab_h + 6.0 * k;
    match tab {
        1 => access_tab(ui, loc, s, &rows, &mut y, &mut out),
        2 => controls_tab(ui, loc, s, &rows, &mut y, &mut out),
        _ => general_tab(ui, loc, s, &rows, &mut y, &mut out),
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
    // the crash log's folder (crate::crashlog); the tooltip shows the path
    let tip = format!(
        "{} {}",
        loc.tr("ui.settings.open_log_tip"),
        crate::crashlog::log_dir().display()
    );
    let log = Button::new(loc.tr("ui.settings.open_log"))
        .ghost()
        .icon(Icon::Log)
        .tooltip(&tip);
    let lw = ui.button_width(&log);
    let lr = Rect::new(br.right() + 8.0, br.y, lw, br.h);
    if ui.button("set_open_log", lr, &log).clicked {
        let _ = crate::crashlog::open_log_folder();
    }
    let path_st = t.small().color(t.text_disabled);
    ui.fonts.draw_fit(
        loc.tr("mainmenu.settings.saved_auto"),
        Rect::new(
            lr.right() + 12.0,
            br.y - 4.0,
            r.right() - lr.right() - 30.0,
            br.h + 8.0,
        ),
        &path_st,
        Align::Right,
        0.85,
    );
    // Esc closes — unless it just cancelled a key capture
    if CAPTURE.with(Cell::get).is_none() && ui.esc() {
        out.close = true;
    }
    if out.close {
        CAPTURE.with(|c| c.set(None));
    }
    if out.changed {
        s.sanitize();
    }
    out
}

fn general_tab(
    ui: &mut Ui,
    loc: &Locale,
    s: &mut Settings,
    rows: &Rows,
    y: &mut f32,
    out: &mut SettingsResponse,
) {
    let t = ui.theme.clone();
    let value_st = t.small_medium().color(t.text_dim);
    // ── language
    rows.section(ui, y, loc.tr("mainmenu.settings.section.language"));
    let c = rows.row(ui, y, loc.lang.native_name(), None);
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
    rows.section(ui, y, loc.tr("mainmenu.settings.section.sound"));
    for (key, label, v) in [
        ("set_music", "ui.settings.volume_music", &mut s.music_volume),
        ("set_sfx", "ui.settings.volume_sfx", &mut s.sfx_volume),
    ] {
        let c = rows.row(ui, y, loc.tr(label), None);
        let (val, sl) = c.split_right(pct_w(ui), 8.0);
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
    rows.section(ui, y, loc.tr("mainmenu.settings.section.display"));
    let c = rows.row(ui, y, loc.tr("ui.settings.ui_scale"), None);
    let (val, sl) = c.split_right(pct_w(ui), 8.0);
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
    let c = rows.row(ui, y, loc.tr("ui.settings.fullscreen"), Some(TOGGLE_W));
    if ui.toggle("set_full", c, &mut s.fullscreen).changed {
        out.changed = true;
    }

    // ── story
    rows.section(ui, y, loc.tr("mainmenu.settings.section.story"));
    let c = rows.row(ui, y, loc.tr("mainmenu.settings.text_speed"), None);
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
    let c = rows.row(
        ui,
        y,
        loc.tr("mainmenu.settings.auto_advance"),
        Some(TOGGLE_W),
    );
    if ui.toggle("set_auto", c, &mut s.auto_advance).changed {
        out.changed = true;
    }
    let c = rows.row(ui, y, loc.tr("ui.tutorial.setting"), None);
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
    let c = rows.row(ui, y, loc.tr("ui.settings.pause_on_events"), Some(TOGGLE_W));
    if ui.toggle("set_pause", c, &mut s.pause_on_events).changed {
        out.changed = true;
    }
    rows.hint(ui, y, loc.tr("mainmenu.settings.pause_hint"));
}

fn access_tab(
    ui: &mut Ui,
    loc: &Locale,
    s: &mut Settings,
    rows: &Rows,
    y: &mut f32,
    out: &mut SettingsResponse,
) {
    rows.section(ui, y, loc.tr("access.section.reading"));
    let c = rows.row(ui, y, loc.tr("access.text_size"), None);
    let labels: Vec<String> = TextSize::ALL.iter().map(|x| x.label()).collect();
    let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
    let mut sel = s.text_size.index();
    if ui
        .segmented("set_text_size", c.inset_xy(0.0, 3.0), &labels, &mut sel)
        .changed
    {
        s.text_size = TextSize::ALL[sel];
        out.changed = true;
    }
    rows.hint(ui, y, loc.tr("access.text_size_hint"));

    rows.section(ui, y, loc.tr("access.section.seeing"));
    let c = rows.row(ui, y, loc.tr("access.colorblind"), Some(TOGGLE_W));
    if ui
        .toggle("set_colorblind", c, &mut s.colorblind_markers)
        .changed
    {
        out.changed = true;
    }
    rows.hint(ui, y, loc.tr("access.colorblind_hint"));

    rows.section(ui, y, loc.tr("access.section.pace"));
    let c = rows.row(
        ui,
        y,
        loc.tr("ui.settings.relaxed_minigames"),
        Some(TOGGLE_W),
    );
    if ui
        .toggle("set_relaxed", c, &mut s.relaxed_minigames)
        .changed
    {
        out.changed = true;
    }
    // the money side lives in the difficulty: a pointer to it, wrapped
    let st = ui.theme.small();
    let used = ui.fonts.draw_wrapped(
        loc.tr("access.relaxed_hint"),
        Rect::new(rows.x0, *y, rows.w, st.line_h() * 3.0),
        &st,
        Align::Left,
        Some(3),
    );
    *y += used + 6.0;
}

fn controls_tab(
    ui: &mut Ui,
    loc: &Locale,
    s: &mut Settings,
    rows: &Rows,
    y: &mut f32,
    out: &mut SettingsResponse,
) {
    let t = ui.theme.clone();
    // a key for the action waiting for one
    if let Some(a) = CAPTURE.with(Cell::get) {
        if ui.esc() {
            CAPTURE.with(|c| c.set(None));
        } else if let Some(k) = pressed_key() {
            ui.consume_key(k);
            if s.keys.set(a, k) {
                out.changed = true;
                CAPTURE.with(|c| c.set(None));
                REFUSED.with(|c| c.set(None));
            } else {
                REFUSED.with(|c| c.set(Some((k, ui.time()))));
            }
        }
    }
    let capture = CAPTURE.with(Cell::get);
    let hint = t.small();
    let hh = hint.line_h() * 2.0;
    ui.fonts.draw_wrapped(
        loc.tr("access.controls.hint"),
        Rect::new(rows.x0, *y + 4.0, rows.w, hh),
        &hint,
        Align::Left,
        Some(2),
    );
    *y += hh + 10.0;
    // two columns of rows: label · keycap button
    let n = Action::ALL.len();
    let per_col = n.div_ceil(2);
    let col_gap = 16.0;
    let col_w = (rows.w - col_gap) / 2.0;
    let row_h = (26.0 * rows.k).max(23.0).max(t.strong().line_h() + 6.0);
    let label_st = t.body();
    for (i, a) in Action::ALL.iter().copied().enumerate() {
        let col = i / per_col;
        let rr = Rect::new(
            rows.x0 + col as f32 * (col_w + col_gap),
            *y + (i % per_col) as f32 * (row_h + 2.0),
            col_w,
            row_h,
        );
        let key = s.keys.key(a);
        let conflict = s.keys.conflict_of(a);
        let waiting = capture == Some(a);
        let label = if waiting {
            loc.tr_owned("access.controls.press")
        } else {
            keys::badge(key)
        };
        let mut b = Button::new(&label).secondary();
        if waiting {
            b = b.primary();
        }
        let tip = conflict.map(|o| {
            loc.trf(
                "access.controls.conflict",
                &[("action", &loc.tr_owned(&o.label_key()))],
            )
        });
        if let Some(tp) = &tip {
            b = b.tooltip(tp);
        }
        let bw = ui.button_width(&b).clamp(44.0, col_w * 0.5);
        let br = Rect::new(rr.right() - bw, rr.y + 1.0, bw, rr.h - 2.0);
        let mut lr = Rect::new(rr.x, rr.y, rr.w - bw - 6.0, rr.h);
        if conflict.is_some() {
            // a warning glyph (shape + colour) before the label
            crate::ui::draw_icon(
                Icon::Warning,
                vec2(lr.x + 7.0, lr.center_y()),
                13.0,
                t.amber,
            );
            lr = Rect::new(lr.x + 18.0, lr.y, lr.w - 18.0, lr.h);
        }
        ui.fonts.draw_fit(
            &loc.tr_owned(&a.label_key()),
            lr,
            &label_st.color(if conflict.is_some() { t.amber } else { t.text }),
            Align::Left,
            0.85,
        );
        if ui.button(("set_key", a.id()), br, &b).clicked {
            CAPTURE.with(|c| c.set(if waiting { None } else { Some(a) }));
            REFUSED.with(|c| c.set(None));
        }
    }
    *y += per_col as f32 * (row_h + 2.0) + 6.0;
    // a refused key: why; else a word about conflicts
    let msg = REFUSED
        .with(Cell::get)
        .filter(|(_, at)| ui.time() - at < 4.0)
        .and_then(|(k, _)| {
            keys::reserved_reason(k).map(|r| {
                loc.trf(
                    "access.controls.reserved",
                    &[("key", &keys::badge(k)), ("reason", &loc.tr_owned(r))],
                )
            })
        })
        .or_else(|| {
            (!s.keys.conflicts().is_empty()).then(|| loc.tr_owned("access.controls.conflicts"))
        });
    let reset = Button::new(loc.tr("access.controls.reset"))
        .ghost()
        .icon(Icon::Undo);
    let rw = ui.button_width(&reset);
    let rr = Rect::new(rows.x0 + rows.w - rw, *y, rw, 26.0);
    if !s.keys.is_default() && ui.button("set_keys_reset", rr, &reset).clicked {
        s.keys.reset();
        CAPTURE.with(|c| c.set(None));
        out.changed = true;
    }
    if let Some(m) = msg {
        ui.fonts.draw_fit(
            &m,
            Rect::new(rows.x0, *y - 4.0, rows.w - rw - 10.0, 34.0),
            &t.small().color(t.amber),
            Align::Left,
            0.85,
        );
    }
    *y += 30.0;
}

/// The first key pressed this frame (the one to bind), modifiers aside.
fn pressed_key() -> Option<KeyCode> {
    let mut v: Vec<KeyCode> = get_keys_pressed().into_iter().collect();
    v.sort_by_key(|k| *k as u32);
    v.into_iter().find(|k| {
        !matches!(
            k,
            KeyCode::LeftShift
                | KeyCode::RightShift
                | KeyCode::LeftControl
                | KeyCode::RightControl
                | KeyCode::LeftAlt
                | KeyCode::RightAlt
                | KeyCode::LeftSuper
                | KeyCode::RightSuper
                | KeyCode::Unknown
        )
    })
}

/// Width a toggle row keeps for its switch.
const TOGGLE_W: f32 = 52.0;

/// Width of a "100 %" value next to a slider (grows with the text size).
fn pct_w(ui: &Ui) -> f32 {
    (ui.fonts.measure("100 %", &ui.theme.small_medium()) + 6.0).max(44.0)
}

fn val_rect(r: Rect) -> Rect {
    r.inset_xy(0.0, 5.0)
}
