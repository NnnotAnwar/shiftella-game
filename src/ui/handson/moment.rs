//! The small night-moment card next to its world anchor (design §5.1): a title, one line
//! of text and 1–3 one-click options with cost chips. Non-blocking; keys **1–3**, **Esc**.

use macroquad::prelude::*;

use super::common::{corner_key, mini_chip, mini_chip_width, place_near, pointer, ring};
use crate::ui::draw::{
    self, Paint, fill_circle, fill_rrect, mix, shadow_rrect, stroke_rrect, with_a,
};
use crate::ui::layout::RectExt;
use crate::ui::{Align, Glass, Icon, IconButton, Ui, UiSound, draw_icon, ease_out};

/// One option button.
#[derive(Clone, Debug)]
pub struct MomentOption {
    pub label: String,
    /// "−15 Kč", "free", "+♥"… (already formatted).
    pub cost: Option<String>,
    /// The chip is a gain (emerald) rather than a spend (amber).
    pub gain: bool,
    pub icon: Option<Icon>,
    /// e.g. "Needs a stage" — the option is shown but can't be picked.
    pub disabled: bool,
    pub tooltip: Option<String>,
    /// Why it's disabled, drawn small on the right of the row ("Needs a stage").
    pub reason: Option<String>,
}

impl MomentOption {
    pub fn new(label: impl Into<String>) -> Self {
        MomentOption {
            label: label.into(),
            cost: None,
            gain: false,
            icon: None,
            disabled: false,
            tooltip: None,
            reason: None,
        }
    }
    /// Disabled with a visible one-line reason (also the tooltip).
    pub fn locked(mut self, reason: impl Into<String>) -> Self {
        let r = reason.into();
        self.disabled = true;
        self.tooltip = Some(r.clone());
        self.reason = Some(r);
        self.cost = None;
        self
    }
    pub fn cost(mut self, c: impl Into<String>) -> Self {
        self.cost = Some(c.into());
        self
    }
    /// A gain chip ("+♥", "+2 rep").
    pub fn gain(mut self, c: impl Into<String>) -> Self {
        self.cost = Some(c.into());
        self.gain = true;
        self
    }
    pub fn icon(mut self, i: Icon) -> Self {
        self.icon = Some(i);
        self
    }
    pub fn disabled(mut self, d: bool) -> Self {
        self.disabled = d;
        self
    }
    pub fn tooltip(mut self, t: impl Into<String>) -> Self {
        self.tooltip = Some(t.into());
        self
    }
}

#[derive(Clone, Debug)]
pub struct MomentView {
    pub icon: Icon,
    pub title: String,
    pub text: Option<String>,
    pub options: Vec<MomentOption>,
    /// Time left before it fades, 1 → 0 (a thin ring by the title).
    pub expires: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MomentAction {
    Pick(usize),
    /// Closed (Esc / ×): the moment stays in the world until it expires.
    Dismiss,
}

const W: f32 = 272.0;
const OPT_H: f32 = 30.0;
/// A locked option's row: the label, its reason on a small line under it.
const LOCKED_H: f32 = 40.0;

fn opt_h(o: &MomentOption) -> f32 {
    if o.reason.is_some() { LOCKED_H } else { OPT_H }
}

/// Width left for the title (icon on the left; expiry ring and × on the right).
fn title_w(v: &MomentView) -> f32 {
    W - 24.0 - 34.0 - 24.0 - if v.expires.is_some() { 18.0 } else { 0.0 }
}

fn title_lines(ui: &Ui, v: &MomentView) -> usize {
    ui.fonts
        .wrap(&v.title, title_w(v), &ui.theme.h2())
        .len()
        .clamp(1, 2)
}

fn head_h(ui: &Ui, v: &MomentView) -> f32 {
    if title_lines(ui, v) > 1 { 42.0 } else { 28.0 }
}

pub fn moment_card_size(ui: &Ui, v: &MomentView) -> Vec2 {
    let mut h = 12.0 + head_h(ui, v);
    if let Some(tx) = &v.text {
        h += 4.0
            + ui.fonts
                .wrapped_height(tx, W - 24.0, &ui.theme.small(), Some(3));
    }
    let n = v.options.len().clamp(1, 3) as f32;
    let rows: f32 = v.options.iter().take(3).map(opt_h).sum::<f32>().max(OPT_H);
    h += 10.0 + rows + (n - 1.0) * 4.0 + 12.0;
    vec2(W, h.ceil())
}

/// Where [`moment_card`] will put itself (to keep later cards away from it).
pub fn moment_card_rect(ui: &Ui, anchor: Vec2, v: &MomentView, avoid: &[Rect]) -> Rect {
    place_near(ui.screen(), anchor, moment_card_size(ui, v), 18.0, avoid)
}

/// The moment card next to `anchor` (UI units), away from `avoid`. `close_tip` is the ×
/// tooltip. Returns the action; the card's rect is [`moment_card_rect`].
pub fn moment_card(
    ui: &mut Ui,
    key: impl std::hash::Hash,
    anchor: Vec2,
    v: &MomentView,
    close_tip: &str,
    avoid: &[Rect],
) -> Option<MomentAction> {
    let t = ui.theme.clone();
    let id = ui.id(key);
    let size = moment_card_size(ui, v);
    let base = place_near(ui.screen(), anchor, size, 18.0, avoid);
    let k = ui.ease_value(id.with("appear"), 1.0, 0.07);
    let r = base.translate(vec2(0.0, (1.0 - ease_out(k)) * 6.0));
    draw::push_alpha(k);
    // soft marker glow at the anchor so the card's origin is clear
    draw::glow_circle(anchor, 26.0, with_a(t.indigo_400, 0.35));
    fill_circle(anchor, 4.0, t.indigo_300);
    pointer(r, anchor, t.glass_solid_top);
    ui.glass(r, &Glass::panel().solid().radius(12.0));
    let mut action = None;
    let inner = r.inset(12.0);

    // header
    let ic = vec2(inner.x + 13.0, inner.y + head_h(ui, v) / 2.0);
    draw::glow_circle(ic, 20.0, with_a(t.amber, 0.22));
    fill_circle(
        ic,
        13.0,
        Paint::vertical(
            Rect::new(ic.x - 13.0, ic.y - 13.0, 26.0, 26.0),
            with_a(t.amber, 0.28),
            with_a(t.amber_deep, 0.14),
        ),
    );
    draw_icon(v.icon, ic, 15.0, t.amber);
    let close = Rect::new(inner.right() - 20.0, inner.y + 4.0, 20.0, 20.0);
    if let Some(e) = v.expires {
        let ec = vec2(close.x - 10.0, close.center_y());
        ring(
            ec,
            6.0,
            1.6,
            e,
            Color::new(1.0, 1.0, 1.0, 0.1),
            with_a(t.indigo_300, 0.9),
        );
    }
    let hh = head_h(ui, v);
    let ts = t.h2().line_height(1.3);
    let n = title_lines(ui, v);
    let title_r = Rect::new(
        inner.x + 34.0,
        inner.y + (hh - n as f32 * ts.line_h()) / 2.0,
        title_w(v),
        n as f32 * ts.line_h(),
    );
    ui.fonts
        .draw_wrapped(&v.title, title_r, &ts, Align::Left, Some(2));
    if ui
        .icon_button(
            id.with("close"),
            close,
            Icon::Close,
            &IconButton::ghost().size(13.0).tooltip(close_tip),
        )
        .clicked
    {
        action = Some(MomentAction::Dismiss);
    }
    let mut y = inner.y + hh;
    if let Some(tx) = &v.text {
        y += 4.0;
        y += ui.fonts.draw_wrapped(
            tx,
            Rect::new(inner.x, y, inner.w, 60.0),
            &t.small(),
            Align::Left,
            Some(3),
        );
    }
    y += 10.0;

    // options
    for (i, o) in v.options.iter().take(3).enumerate() {
        let br = Rect::new(inner.x, y, inner.w, opt_h(o));
        y += opt_h(o) + 4.0;
        let resp = ui.interact(id.with(("opt", i)), br);
        let (h, p) = if o.disabled {
            (0.0, 0.0)
        } else {
            (resp.hover_t, resp.press_t)
        };
        if o.disabled {
            draw::push_alpha(0.45);
        }
        if h > 0.01 {
            shadow_rrect(
                br,
                8.0,
                draw::Shadow::new(0.0, 2.0, 10.0, -1.0, with_a(t.indigo, 0.3 * h)),
            );
        }
        let first = i == 0;
        let (c0, c1) = if first {
            (
                mix(with_a(t.indigo, 0.34), with_a(t.indigo, 0.6), h),
                mix(with_a(t.violet, 0.26), with_a(t.violet, 0.5), h),
            )
        } else {
            (
                Color::new(1.0, 1.0, 1.0, 0.07 + 0.05 * h),
                Color::new(1.0, 1.0, 1.0, 0.04 + 0.04 * h),
            )
        };
        let br2 = br.translate(vec2(0.0, p * 0.5));
        fill_rrect(br2, 8.0, Paint::horizontal(br2, c0, c1));
        stroke_rrect(
            br2,
            8.0,
            1.0,
            mix(Color::new(1.0, 1.0, 1.0, 0.1), with_a(t.indigo_300, 0.8), h),
        );
        if o.reason.is_some() {
            // locked: a lock where the number key would be (the key does nothing)
            draw_icon(
                Icon::Lock,
                vec2(br2.x + 15.0, br2.y + 13.0),
                11.0,
                t.text_dim,
            );
        } else {
            corner_key(
                ui,
                vec2(br2.x + 8.0, br2.center_y() - 7.0),
                &(i + 1).to_string(),
            );
        }
        let mut x = br2.x + 30.0;
        if let Some(ic) = o.icon {
            draw_icon(ic, vec2(x + 7.0, br2.center_y()), 14.0, t.text_dim);
            x += 20.0;
        }
        let cost_w = o
            .cost
            .as_ref()
            .map_or(0.0, |c| mini_chip_width(ui, c, false) + 8.0);
        let why_st = t.small().size(11.0).color(t.text_dim);
        // a locked row: the label on top, the reason on a line under it
        let (label_y, label_h) = if o.reason.is_some() {
            (br2.y + 3.0, 20.0)
        } else {
            (br2.y, br2.h)
        };
        let why_r = Rect::new(x, br2.y + 21.0, br2.right() - x - 8.0, 15.0);
        ui.fonts.draw_in(
            &o.label,
            Rect::new(x, label_y, br2.right() - x - cost_w - 6.0, label_h),
            &t.strong().size(12.5),
            Align::Left,
        );

        if let Some(c) = &o.cost {
            let cw = mini_chip_width(ui, c, false);
            let fg = if o.gain { t.emerald_400 } else { t.amber };
            mini_chip(
                ui,
                vec2(br2.right() - cw - 6.0, br2.center_y() - 9.0),
                c,
                fg,
                Color::new(0.03, 0.02, 0.09, 0.4),
                None,
            );
        }
        if o.disabled {
            draw::pop_alpha();
        }
        // the reason stays legible (not dimmed with the row)
        if let Some(why) = &o.reason {
            ui.fonts.draw_in(why, why_r, &why_st, Align::Left);
        }
        if let Some(tip) = &o.tooltip {
            ui.tooltip(&resp, tip);
        }
        if resp.clicked && !o.disabled {
            ui.sound(UiSound::Click);
            action = Some(MomentAction::Pick(i));
        }
    }
    draw::pop_alpha();

    let keys = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3];
    for (i, kc) in keys.iter().enumerate().take(v.options.len().min(3)) {
        if !v.options[i].disabled && ui.take_key(*kc) {
            action = Some(MomentAction::Pick(i));
        }
    }
    if ui.esc() {
        action = Some(MomentAction::Dismiss);
    }
    action
}
