//! Widgets as `Ui` methods. Every widget takes a hashable `key` (stable id under
//! the current `push_id` scope) and a rect in UI units, and returns a [`Response`].

use std::hash::Hash;

use macroquad::miniquad::CursorIcon;
use macroquad::prelude::*;

use super::context::{Response, Ui, UiSound, approach, ease_out};
use super::draw::{
    self, Paint, fill_circle, fill_rrect, glow_circle, hline, mix, shadow_rrect, stroke_circle,
    stroke_rrect, with_a,
};
use super::glass::{Glass, draw_glass};
use super::icons::{Icon, draw_icon};
use super::id::Id;
use super::layout::RectExt;
use super::text::{Align, TextStyle, initials};
use super::theme::{Theme, Tone, radius, size};

// ---------------------------------------------------------------------------
// Buttons
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonKind {
    /// Indigo → violet gradient with glow. One per screen.
    Primary,
    /// Raised glass.
    Secondary,
    /// Text only, faint hover.
    Ghost,
    /// Red tint, for "fire", "demolish" (no confirm — use Undo).
    Danger,
}

/// Button description (builder).
#[derive(Clone, Copy, Debug)]
pub struct Button<'a> {
    pub label: &'a str,
    pub kind: ButtonKind,
    pub icon: Option<Icon>,
    pub trailing_icon: Option<Icon>,
    pub shortcut: Option<&'a str>,
    /// Shows the "active" look (speed buttons, open screen in the toolbar).
    pub selected: bool,
    pub disabled: bool,
    pub tooltip: Option<&'a str>,
    pub align: Align,
}

impl<'a> Button<'a> {
    pub fn new(label: &'a str) -> Self {
        Button {
            label,
            kind: ButtonKind::Secondary,
            icon: None,
            trailing_icon: None,
            shortcut: None,
            selected: false,
            disabled: false,
            tooltip: None,
            align: Align::Center,
        }
    }
    pub fn primary(mut self) -> Self {
        self.kind = ButtonKind::Primary;
        self
    }
    pub fn secondary(mut self) -> Self {
        self.kind = ButtonKind::Secondary;
        self
    }
    pub fn ghost(mut self) -> Self {
        self.kind = ButtonKind::Ghost;
        self
    }
    pub fn danger(mut self) -> Self {
        self.kind = ButtonKind::Danger;
        self
    }
    pub fn icon(mut self, i: Icon) -> Self {
        self.icon = Some(i);
        self
    }
    pub fn trailing(mut self, i: Icon) -> Self {
        self.trailing_icon = Some(i);
        self
    }
    pub fn shortcut(mut self, k: &'a str) -> Self {
        self.shortcut = Some(k);
        self
    }
    pub fn selected(mut self, s: bool) -> Self {
        self.selected = s;
        self
    }
    pub fn disabled(mut self, d: bool) -> Self {
        self.disabled = d;
        self
    }
    pub fn tooltip(mut self, t: &'a str) -> Self {
        self.tooltip = Some(t);
        self
    }
    pub fn align(mut self, a: Align) -> Self {
        self.align = a;
        self
    }
}

/// Square icon button description.
#[derive(Clone, Copy, Debug)]
pub struct IconButton<'a> {
    pub kind: ButtonKind,
    pub size: f32,
    pub selected: bool,
    pub disabled: bool,
    pub tooltip: Option<&'a str>,
    pub color: Option<Color>,
    pub badge: Option<u32>,
}

impl<'a> IconButton<'a> {
    pub fn ghost() -> Self {
        IconButton {
            kind: ButtonKind::Ghost,
            size: size::ICON,
            selected: false,
            disabled: false,
            tooltip: None,
            color: None,
            badge: None,
        }
    }
    pub fn secondary() -> Self {
        IconButton {
            kind: ButtonKind::Secondary,
            ..IconButton::ghost()
        }
    }
    pub fn primary() -> Self {
        IconButton {
            kind: ButtonKind::Primary,
            ..IconButton::ghost()
        }
    }
    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
    pub fn selected(mut self, s: bool) -> Self {
        self.selected = s;
        self
    }
    pub fn disabled(mut self, d: bool) -> Self {
        self.disabled = d;
        self
    }
    pub fn tooltip(mut self, t: &'a str) -> Self {
        self.tooltip = Some(t);
        self
    }
    pub fn color(mut self, c: Color) -> Self {
        self.color = Some(c);
        self
    }
    pub fn badge(mut self, n: u32) -> Self {
        self.badge = Some(n);
        self
    }
}

/// Paint the background of a button-like surface in a given state.
pub fn paint_surface(
    t: &Theme,
    r: Rect,
    rad: f32,
    kind: ButtonKind,
    selected: bool,
    hover: f32,
    press: f32,
) {
    let accent = kind == ButtonKind::Primary || selected;
    if accent {
        let glow = 0.22 + 0.18 * hover;
        shadow_rrect(
            r,
            rad,
            draw::Shadow::new(
                0.0,
                3.0,
                14.0,
                -1.0,
                with_a(t.indigo, glow * (1.0 - press * 0.5)),
            ),
        );
        let lift = 0.10 * hover - 0.14 * press;
        let c0 = mix(t.indigo, WHITE, lift.max(0.0));
        let c1 = mix(t.violet, WHITE, lift.max(0.0) * 0.6);
        let (c0, c1) = if lift < 0.0 {
            (mix(c0, BLACK, -lift), mix(c1, BLACK, -lift))
        } else {
            (c0, c1)
        };
        fill_rrect(r, rad, Paint::diagonal(r, c0, c1));
        stroke_rrect(
            r,
            rad,
            1.0,
            Paint::vertical(
                r,
                Color::new(1.0, 1.0, 1.0, 0.34),
                Color::new(1.0, 1.0, 1.0, 0.06),
            ),
        );
        return;
    }
    match kind {
        ButtonKind::Secondary => {
            let a = 0.065 + 0.05 * hover;
            let fill = mix(
                Color::new(1.0, 1.0, 1.0, a),
                Color::new(0.0, 0.0, 0.0, 0.2),
                press,
            );
            fill_rrect(
                r,
                rad,
                Paint::vertical(r, with_a(fill, fill.a * 1.25), fill),
            );
            stroke_rrect(
                r,
                rad,
                1.0,
                Paint::vertical(
                    r,
                    Color::new(1.0, 1.0, 1.0, 0.13 + 0.06 * hover),
                    Color::new(1.0, 1.0, 1.0, 0.04),
                ),
            );
        }
        ButtonKind::Ghost => {
            let a = 0.065 * hover + 0.04 * press;
            if a > 0.002 {
                fill_rrect(r, rad, Color::new(1.0, 1.0, 1.0, a));
            }
        }
        ButtonKind::Danger => {
            let a = 0.12 + 0.08 * hover - 0.04 * press;
            fill_rrect(r, rad, with_a(t.red, a));
            stroke_rrect(r, rad, 1.0, with_a(t.red_400, 0.32 + 0.15 * hover));
        }
        ButtonKind::Primary => unreachable!(),
    }
}

impl Ui {
    /// Natural width of a button (padding + icon + label + keycap).
    pub fn button_width(&self, b: &Button) -> f32 {
        let st = self.theme.strong();
        let mut w = 20.0 + self.fonts.measure(b.label, &st);
        if b.icon.is_some() {
            w += 16.0 + if b.label.is_empty() { 0.0 } else { 6.0 };
        }
        if b.trailing_icon.is_some() {
            w += 20.0;
        }
        if let Some(k) = b.shortcut {
            w += self.keycap_width(k) + 8.0;
        }
        w.ceil()
    }

    pub fn button(&mut self, key: impl Hash, r: Rect, b: &Button) -> Response {
        let id = self.id(key);
        let resp = self.interact_with(id, r, !b.disabled);
        let t = self.theme.clone();
        let rad = radius::S.min(r.h / 2.0);
        if b.disabled {
            draw::push_alpha(0.45);
        }
        let press = resp.press_t;
        paint_surface(&t, r, rad, b.kind, b.selected, resp.hover_t, press);
        let accent = b.kind == ButtonKind::Primary || b.selected;
        let fg = if b.disabled {
            t.text_disabled
        } else if accent {
            t.text_on_accent
        } else {
            match b.kind {
                ButtonKind::Ghost => mix(t.text_dim, t.text, resp.hover_t),
                ButtonKind::Danger => t.red_400,
                _ => t.text,
            }
        };
        let st = t.strong().color(fg);
        let content_w = self.button_width(b) - 20.0;
        let inner = r.inset_xy(10.0, 0.0).translate(vec2(0.0, press * 0.5));
        let mut x = match b.align {
            Align::Left => inner.x,
            Align::Center => inner.x + ((inner.w - content_w) / 2.0).max(0.0),
            Align::Right => inner.right() - content_w,
        };
        let cy = inner.center_y();
        if let Some(i) = b.icon {
            draw_icon(i, vec2(x + 8.0, cy), 16.0, fg);
            x += 16.0 + if b.label.is_empty() { 0.0 } else { 6.0 };
        }
        let mut text_w = self.fonts.measure(b.label, &st);
        let mut right_reserve = 0.0;
        if let Some(k) = b.shortcut {
            right_reserve += self.keycap_width(k) + 8.0;
        }
        if b.trailing_icon.is_some() {
            right_reserve += 20.0;
        }
        let max_text = (inner.right() - right_reserve - x).max(0.0);
        if text_w > max_text {
            text_w = max_text;
        }
        self.fonts.draw_in(
            b.label,
            Rect::new(x, inner.y, text_w + 1.0, inner.h),
            &st,
            Align::Left,
        );
        x += text_w;
        if let Some(k) = b.shortcut {
            let kx = if b.align == Align::Center {
                x + 8.0
            } else {
                inner.right()
                    - self.keycap_width(k)
                    - if b.trailing_icon.is_some() { 20.0 } else { 0.0 }
            };
            self.keycap(vec2(kx, cy), k, accent);
            x = kx + self.keycap_width(k);
        }
        if let Some(i) = b.trailing_icon {
            let tx = if b.align == Align::Center {
                x + 12.0
            } else {
                inner.right() - 8.0
            };
            draw_icon(i, vec2(tx, cy), 14.0, with_a(fg, fg.a));
        }
        if b.disabled {
            draw::pop_alpha();
        }
        if resp.clicked {
            self.sound(UiSound::Click);
        }
        if let Some(tip) = b.tooltip {
            self.tooltip(&resp, tip);
        }
        resp
    }

    /// Square icon button.
    pub fn icon_button(&mut self, key: impl Hash, r: Rect, icon: Icon, b: &IconButton) -> Response {
        let id = self.id(key);
        let resp = self.interact_with(id, r, !b.disabled);
        let t = self.theme.clone();
        let rad = radius::S.min(r.h / 2.0);
        if b.disabled {
            draw::push_alpha(0.45);
        }
        paint_surface(&t, r, rad, b.kind, b.selected, resp.hover_t, resp.press_t);
        let accent = b.kind == ButtonKind::Primary || b.selected;
        let fg = b.color.unwrap_or(if accent {
            t.text_on_accent
        } else if b.kind == ButtonKind::Ghost {
            mix(t.text_dim, t.text, resp.hover_t)
        } else {
            t.text
        });
        let c = vec2(r.x + r.w / 2.0, r.center_y() + resp.press_t * 0.5);
        draw_icon(icon, c, b.size, fg);
        if let Some(n) = b.badge.filter(|n| *n > 0) {
            self.badge(vec2(r.right() - 4.0, r.y + 4.0), n);
        }
        if b.disabled {
            draw::pop_alpha();
        }
        if resp.clicked {
            self.sound(UiSound::Click);
        }
        if let Some(tip) = b.tooltip {
            self.tooltip(&resp, tip);
        }
        resp
    }

    /// Small red/indigo count bubble centred at `c`.
    pub fn badge(&mut self, c: Vec2, n: u32) {
        let t = &self.theme;
        let label = if n > 99 {
            "99+".to_string()
        } else {
            n.to_string()
        };
        let st = TextStyle::new(11.0, super::text::Weight::Bold, WHITE).tabular();
        let w = (self.fonts.measure(&label, &st) + 8.0).max(17.0);
        let r = Rect::new(c.x - w / 2.0, c.y - 8.5, w, 17.0);
        shadow_rrect(
            r,
            8.0,
            draw::Shadow::new(0.0, 1.0, 4.0, 0.0, Color::new(0.0, 0.0, 0.0, 0.4)),
        );
        fill_rrect(r, 8.0, Paint::vertical(r, mix(t.red, WHITE, 0.12), t.red));
        self.fonts.draw_in(&label, r, &st, Align::Center);
    }

    pub fn keycap_width(&self, label: &str) -> f32 {
        (self.fonts.measure(label, &self.theme.key()) + 10.0).max(18.0)
    }

    /// Keyboard hint chip, left edge at `left.x`, vertically centred on `left.y`.
    pub fn keycap(&mut self, left: Vec2, label: &str, on_accent: bool) -> Rect {
        let t = &self.theme;
        let w = self.keycap_width(label);
        let r = Rect::new(left.x, left.y - 9.0, w, 18.0);
        let (bg, edge, fg) = if on_accent {
            (
                Color::new(1.0, 1.0, 1.0, 0.16),
                Color::new(1.0, 1.0, 1.0, 0.22),
                Color::new(1.0, 1.0, 1.0, 0.92),
            )
        } else {
            (
                Color::new(0.03, 0.02, 0.09, 0.45),
                Color::new(1.0, 1.0, 1.0, 0.11),
                t.text_dim,
            )
        };
        fill_rrect(r, radius::XS, bg);
        stroke_rrect(r, radius::XS, 1.0, edge);
        // bottom lip like a real key
        hline(
            r.x + 2.0,
            r.right() - 2.0,
            r.bottom() - 1.0,
            Color::new(0.0, 0.0, 0.0, 0.25),
        );
        let st = t.key().color(fg);
        self.fonts
            .draw_in(label, r.translate(vec2(0.0, -0.5)), &st, Align::Center);
        r
    }

    // -----------------------------------------------------------------------
    // Toggles, checkboxes, sliders
    // -----------------------------------------------------------------------

    /// iOS-style switch (34×20) centred vertically in `r`, aligned right.
    pub fn toggle(&mut self, key: impl Hash, r: Rect, value: &mut bool) -> Response {
        let id = self.id(key);
        let track = Rect::new(r.right() - 34.0, r.center_y() - 10.0, 34.0, 20.0);
        let mut resp = self.interact(id, r);
        if resp.clicked {
            *value = !*value;
            resp.changed = true;
            self.sound(UiSound::Toggle);
        }
        let k = {
            let dt = self.dt();
            let a = self.anim(id);
            a.v = approach(a.v, if *value { 1.0 } else { 0.0 }, dt, 0.06);
            a.v
        };
        let t = self.theme.clone();
        if k > 0.01 {
            shadow_rrect(
                track,
                10.0,
                draw::Shadow::new(0.0, 2.0, 10.0, -1.0, with_a(t.indigo, 0.35 * k)),
            );
        }
        fill_rrect(track, 10.0, Color::new(0.03, 0.02, 0.09, 0.5));
        stroke_rrect(
            track,
            10.0,
            1.0,
            Color::new(1.0, 1.0, 1.0, 0.12 + 0.06 * resp.hover_t),
        );
        if k > 0.01 {
            draw::push_alpha(k);
            fill_rrect(track, 10.0, Paint::diagonal(track, t.indigo, t.violet));
            stroke_rrect(
                track,
                10.0,
                1.0,
                Paint::vertical(
                    track,
                    Color::new(1.0, 1.0, 1.0, 0.3),
                    Color::new(1.0, 1.0, 1.0, 0.05),
                ),
            );
            draw::pop_alpha();
        }
        let kx = track.x + 10.0 + 14.0 * k;
        let c = vec2(kx, track.center_y());
        shadow_rrect(
            Rect::new(c.x - 8.0, c.y - 8.0, 16.0, 16.0),
            8.0,
            t.shadow_sm,
        );
        fill_circle(
            c,
            8.0 + resp.press_t * 0.6,
            Paint::vertical(
                Rect::new(c.x - 8.0, c.y - 8.0, 16.0, 16.0),
                WHITE,
                mix(WHITE, t.indigo_300, 0.35),
            ),
        );
        resp
    }

    /// Toggle with a label on the left filling `r`.
    pub fn toggle_row(
        &mut self,
        key: impl Hash,
        r: Rect,
        label: &str,
        value: &mut bool,
    ) -> Response {
        let st = self.theme.body();
        self.fonts
            .draw_in(label, r.inset_ltrb(0.0, 0.0, 44.0, 0.0), &st, Align::Left);
        self.toggle(key, r, value)
    }

    /// Checkbox (18 pt) with a label. `strike` dims and strikes the label when checked
    /// (checklists).
    pub fn checkbox(
        &mut self,
        key: impl Hash,
        r: Rect,
        label: &str,
        value: &mut bool,
        strike: bool,
    ) -> Response {
        let id = self.id(key);
        let mut resp = self.interact(id, r);
        if resp.clicked {
            *value = !*value;
            resp.changed = true;
            self.sound(UiSound::Toggle);
        }
        let k = self.ease_value(id, if *value { 1.0 } else { 0.0 }, 0.05);
        let t = self.theme.clone();
        let b = Rect::new(r.x, r.center_y() - 9.0, 18.0, 18.0);
        fill_rrect(b, 5.0, Color::new(0.03, 0.02, 0.09, 0.5));
        stroke_rrect(
            b,
            5.0,
            1.0,
            mix(
                Color::new(1.0, 1.0, 1.0, 0.22),
                t.indigo_300,
                resp.hover_t * 0.6,
            ),
        );
        if k > 0.01 {
            draw::push_alpha(k);
            shadow_rrect(
                b,
                5.0,
                draw::Shadow::new(0.0, 2.0, 8.0, -1.0, with_a(t.indigo, 0.4)),
            );
            fill_rrect(b, 5.0, Paint::diagonal(b, t.indigo, t.violet));
            stroke_rrect(
                b,
                5.0,
                1.0,
                Paint::vertical(
                    b,
                    Color::new(1.0, 1.0, 1.0, 0.3),
                    Color::new(1.0, 1.0, 1.0, 0.05),
                ),
            );
            draw_icon(
                Icon::Check,
                vec2(b.x + 9.0, b.y + 9.0),
                14.0 * (0.6 + 0.4 * ease_out(k)),
                WHITE,
            );
            draw::pop_alpha();
        }
        if !label.is_empty() {
            let checked = *value && strike;
            let st = t.body().color(if checked { t.text_dim } else { t.text });
            let lr = Rect::new(
                b.right() + 10.0,
                r.y,
                (r.right() - b.right() - 10.0).max(0.0),
                r.h,
            );
            let w = self.fonts.draw_in(label, lr, &st, Align::Left);
            if checked && k > 0.0 {
                let y = lr.center_y() - 0.5;
                hline(lr.x - 1.0, lr.x + (w + 2.0) * k, y, with_a(t.text_dim, 0.8));
            }
        }
        resp
    }

    /// Horizontal slider. `step` snaps the value; ←/→ nudge it while hovered.
    pub fn slider(
        &mut self,
        key: impl Hash,
        r: Rect,
        value: &mut f32,
        min: f32,
        max: f32,
        step: Option<f32>,
    ) -> Response {
        let id = self.id(key);
        let mut resp = self.interact(id, r);
        let span = (max - min).max(1e-6);
        let track = Rect::new(r.x + 8.0, r.center_y() - 2.0, (r.w - 16.0).max(1.0), 4.0);
        let old = *value;
        if resp.pressed {
            let f = ((self.mouse().x - track.x) / track.w).clamp(0.0, 1.0);
            *value = min + f * span;
        }
        if resp.hovered {
            let nudge = step.unwrap_or(span / 20.0);
            if self.key_repeat(KeyCode::Left) {
                *value -= nudge;
            }
            if self.key_repeat(KeyCode::Right) {
                *value += nudge;
            }
        }
        if let Some(s) = step.filter(|s| *s > 0.0) {
            *value = min + ((*value - min) / s).round() * s;
        }
        *value = value.clamp(min, max);
        if (*value - old).abs() > 1e-6 {
            resp.changed = true;
        }
        let t = self.theme.clone();
        let f = (*value - min) / span;
        fill_rrect(track, 2.0, Color::new(1.0, 1.0, 1.0, 0.10));
        if let Some(s) = step.filter(|s| span / *s <= 12.0) {
            let n = (span / s).round() as usize;
            for i in 0..=n {
                let x = track.x + track.w * i as f32 / n as f32;
                fill_circle(
                    vec2(x, track.center_y()),
                    1.6,
                    Color::new(1.0, 1.0, 1.0, 0.25),
                );
            }
        }
        let filled = Rect::new(track.x, track.y, track.w * f, track.h);
        if filled.w > 0.5 {
            fill_rrect(
                filled,
                2.0,
                Paint::horizontal(track, t.indigo, t.violet_400),
            );
        }
        let c = vec2(track.x + track.w * f, track.center_y());
        let act = resp.hover_t.max(resp.press_t);
        if act > 0.01 {
            glow_circle(c, 16.0, with_a(t.indigo_400, 0.28 * act));
        }
        shadow_rrect(
            Rect::new(c.x - 8.0, c.y - 8.0, 16.0, 16.0),
            8.0,
            t.shadow_sm,
        );
        fill_circle(
            c,
            8.0,
            Paint::vertical(
                Rect::new(c.x, c.y - 8.0, 1.0, 16.0),
                WHITE,
                mix(WHITE, t.indigo_300, 0.3),
            ),
        );
        stroke_circle(c, 8.0, 1.0, with_a(t.indigo_400, 0.4 + 0.4 * act));
        resp
    }

    // -----------------------------------------------------------------------
    // Segmented control & tabs
    // -----------------------------------------------------------------------

    /// Equal-width segments; the selected one carries the accent with a sliding pill.
    pub fn segmented(
        &mut self,
        key: impl Hash,
        r: Rect,
        options: &[&str],
        selected: &mut usize,
    ) -> Response {
        let id = self.id(key);
        let n = options.len().max(1);
        let t = self.theme.clone();
        let rad = radius::M.min(r.h / 2.0);
        fill_rrect(r, rad, Color::new(0.03, 0.02, 0.09, 0.42));
        stroke_rrect(r, rad, 1.0, Color::new(1.0, 1.0, 1.0, 0.07));
        let inner = r.inset(2.0);
        let cell_w = inner.w / n as f32;
        let mut out = Response {
            id: Some(id),
            rect: r,
            ..Default::default()
        };
        let sel_x = {
            let dt = self.dt();
            let a = self.anim(id);
            if a.n == 0.0 {
                a.v = *selected as f32;
                a.n = 1.0;
            }
            a.v = approach(a.v, *selected as f32, dt, 0.06);
            a.v
        };
        let pill = Rect::new(inner.x + sel_x * cell_w, inner.y, cell_w, inner.h);
        paint_surface(
            &t,
            pill,
            (rad - 2.0).max(2.0),
            ButtonKind::Primary,
            true,
            0.0,
            0.0,
        );
        for (i, label) in options.iter().enumerate() {
            let cell = Rect::new(inner.x + i as f32 * cell_w, inner.y, cell_w, inner.h);
            let resp = self.interact(id.with(i), cell);
            if resp.hovered {
                out.hovered = true;
            }
            if resp.clicked && *selected != i {
                *selected = i;
                out.changed = true;
                out.clicked = true;
                self.sound(UiSound::Click);
            }
            let on = i == *selected;
            if !on && resp.hover_t > 0.01 {
                fill_rrect(
                    cell,
                    (rad - 2.0).max(2.0),
                    Color::new(1.0, 1.0, 1.0, 0.05 * resp.hover_t),
                );
            }
            let st = t.strong().size(12.0).color(if on {
                WHITE
            } else {
                mix(t.text_dim, t.text, resp.hover_t)
            });
            self.fonts
                .draw_in(label, cell.inset_xy(4.0, 0.0), &st, Align::Center);
        }
        out
    }

    /// Underlined tabs with optional icons and count badges.
    pub fn tabs(
        &mut self,
        key: impl Hash,
        r: Rect,
        tabs: &[Tab],
        selected: &mut usize,
    ) -> Response {
        let id = self.id(key);
        let t = self.theme.clone();
        let st = t.strong();
        let mut out = Response {
            id: Some(id),
            rect: r,
            ..Default::default()
        };
        // natural widths
        let widths: Vec<f32> = tabs
            .iter()
            .map(|tab| {
                self.fonts.measure(tab.label, &st)
                    + 20.0
                    + if tab.icon.is_some() { 22.0 } else { 0.0 }
                    + if tab.badge.is_some() { 22.0 } else { 0.0 }
            })
            .collect();
        let mut x = r.x;
        let mut cells = Vec::with_capacity(tabs.len());
        for w in &widths {
            cells.push(Rect::new(x, r.y, *w, r.h));
            x += w + 2.0;
        }
        hline(r.x, r.right(), r.bottom() - 1.0, t.divider);
        for (i, tab) in tabs.iter().enumerate() {
            let cell = cells[i];
            let resp = self.interact(id.with(i), cell);
            if resp.clicked && *selected != i {
                *selected = i;
                out.changed = true;
                out.clicked = true;
                self.sound(UiSound::Click);
            }
            let on = i == *selected;
            let fg = if on {
                t.text
            } else {
                mix(t.text_dim, t.text, resp.hover_t * 0.7)
            };
            if resp.hover_t > 0.01 && !on {
                fill_rrect(
                    cell.inset_xy(2.0, 4.0),
                    radius::S,
                    Color::new(1.0, 1.0, 1.0, 0.04 * resp.hover_t),
                );
            }
            let mut cx = cell.x + 10.0;
            let cy = cell.center_y();
            if let Some(ic) = tab.icon {
                draw_icon(
                    ic,
                    vec2(cx + 8.0, cy),
                    16.0,
                    if on { t.indigo_300 } else { fg },
                );
                cx += 22.0;
            }
            cx += self.fonts.draw(
                tab.label,
                cx,
                self.fonts.baseline_in(cell, &st),
                &st.color(fg),
            );
            if let Some(n) = tab.badge {
                let label = n.to_string();
                let bs = t.key().color(if on { t.indigo_300 } else { t.text_dim });
                let bw = (self.fonts.measure(&label, &bs) + 10.0).max(18.0);
                let br = Rect::new(cx + 6.0, cy - 8.0, bw, 16.0);
                fill_rrect(
                    br,
                    8.0,
                    if on {
                        with_a(t.indigo, 0.25)
                    } else {
                        Color::new(1.0, 1.0, 1.0, 0.08)
                    },
                );
                self.fonts.draw_in(&label, br, &bs, Align::Center);
            }
        }
        // sliding indicator
        if let Some(target) = cells.get(*selected).copied() {
            let (ix, iw) = {
                let dt = self.dt();
                let a = self.anim(id);
                if a.n == 0.0 {
                    a.v = target.x;
                    a.t = target.w;
                    a.n = 1.0;
                }
                a.v = approach(a.v, target.x, dt, 0.06);
                a.t = approach(a.t, target.w, dt, 0.06);
                (a.v, a.t)
            };
            let bar = Rect::new(ix + 8.0, r.bottom() - 2.0, iw - 16.0, 2.0);
            shadow_rrect(
                bar,
                1.0,
                draw::Shadow::new(0.0, 0.0, 8.0, 0.0, with_a(t.indigo_400, 0.7)),
            );
            fill_rrect(bar, 1.0, Paint::horizontal(bar, t.indigo_400, t.violet_400));
        }
        out
    }

    // -----------------------------------------------------------------------
    // Lists
    // -----------------------------------------------------------------------

    /// Two-line list row with hover/selected states.
    pub fn list_row(&mut self, key: impl Hash, r: Rect, row: &ListRow) -> Response {
        let id = self.id(key);
        let resp = self.interact(id, r);
        let t = self.theme.clone();
        let sel = self.ease_value(id, if row.selected { 1.0 } else { 0.0 }, 0.06);
        if resp.hover_t > 0.01 {
            fill_rrect(
                r,
                radius::M,
                Color::new(1.0, 1.0, 1.0, 0.045 * resp.hover_t),
            );
        }
        if sel > 0.01 {
            fill_rrect(
                r,
                radius::M,
                Paint::horizontal(
                    r,
                    with_a(t.indigo, 0.24 * sel),
                    with_a(t.indigo, 0.08 * sel),
                ),
            );
            stroke_rrect(r, radius::M, 1.0, with_a(t.indigo_400, 0.22 * sel));
            let bar = Rect::new(r.x + 1.0, r.y + 8.0, 3.0, r.h - 16.0);
            shadow_rrect(
                bar,
                1.5,
                draw::Shadow::new(0.0, 0.0, 8.0, 0.0, with_a(t.indigo_400, 0.8 * sel)),
            );
            fill_rrect(bar, 1.5, with_a(t.indigo_300, sel));
        }
        let mut x = r.x + 10.0;
        let cy = r.center_y();
        match row.leading {
            Leading::None => {}
            Leading::Icon(ic, c) => {
                let cr = Rect::new(x, cy - 15.0, 30.0, 30.0);
                fill_rrect(cr, radius::M, with_a(c, 0.14));
                draw_icon(ic, vec2(cr.x + 15.0, cy), 16.0, c);
                x += 40.0;
            }
            Leading::Avatar {
                name,
                color,
                status,
            } => {
                let d = (r.h - 12.0).min(32.0);
                self.avatar(vec2(x + d / 2.0, cy), d, name, color, row.selected);
                if let Some(sc) = status {
                    let p = vec2(x + d - 3.0, cy + d / 2.0 - 3.0);
                    fill_circle(p, 5.0, t.ink);
                    fill_circle(p, 3.5, sc);
                }
                x += d + 10.0;
            }
        }
        let mut right = r.right() - 12.0;
        if row.chevron {
            draw_icon(
                Icon::ChevronRight,
                vec2(right - 6.0, cy),
                14.0,
                t.text_disabled,
            );
            right -= 20.0;
        }
        if let Some(v) = row.trailing {
            let tone = row.trailing_tone.unwrap_or(Tone::Neutral);
            let st = t.number().color(if tone == Tone::Neutral {
                t.text
            } else {
                t.tone_fg(tone)
            });
            let w = self.fonts.measure(v, &st);
            self.fonts.draw(
                v,
                right - w,
                self.fonts.baseline_in(Rect::new(0.0, r.y, 0.0, r.h), &st),
                &st,
            );
            right -= w + 12.0;
        }
        let text_w = (right - x).max(0.0);
        let title_st = t.strong().color(if row.dim { t.text_dim } else { t.text });
        match row.subtitle {
            Some(sub) => {
                self.fonts.draw_in(
                    row.title,
                    Rect::new(x, cy - 17.0, text_w, 18.0),
                    &title_st,
                    Align::Left,
                );
                self.fonts.draw_in(
                    sub,
                    Rect::new(x, cy + 1.0, text_w, 16.0),
                    &t.small(),
                    Align::Left,
                );
            }
            None => {
                self.fonts.draw_in(
                    row.title,
                    Rect::new(x, r.y, text_w, r.h),
                    &title_st,
                    Align::Left,
                );
            }
        }
        if resp.clicked {
            self.sound(UiSound::Click);
        }
        resp
    }

    /// Checklist row: checkbox + label + optional meta on the right (e.g. "22:05 · Lída").
    pub fn check_item(
        &mut self,
        key: impl Hash,
        r: Rect,
        label: &str,
        meta: Option<&str>,
        value: &mut bool,
    ) -> Response {
        let id = self.id(&key);
        let hover = self.hovering(r);
        let h = self.ease_value(id.with("bg"), if hover { 1.0 } else { 0.0 }, 0.05);
        if h > 0.01 {
            fill_rrect(r, radius::S, Color::new(1.0, 1.0, 1.0, 0.04 * h));
        }
        let mut label_r = r.inset_xy(8.0, 0.0);
        if let Some(m) = meta {
            let st = self.theme.small();
            let w = self.fonts.measure(m, &st);
            self.fonts
                .draw(m, label_r.right() - w, self.fonts.baseline_in(r, &st), &st);
            label_r.w -= w + 10.0;
        }
        self.checkbox(key, label_r, label, value, true)
    }

    // -----------------------------------------------------------------------
    // Scroll area
    // -----------------------------------------------------------------------

    /// Start a clipped, scrollable area. Draw content at `y = scroll.top()`.
    pub fn begin_scroll(&mut self, key: impl Hash, view: Rect, content_h: f32) -> ScrollArea {
        let id = self.id(key);
        let max = (content_h - view.h).max(0.0);
        let hovered = self.hovering(view);
        let wheel = self.wheel().y;
        let dt = self.dt();
        let a = self.anim(id);
        if hovered && wheel != 0.0 {
            let step = if wheel.abs() < 4.0 {
                wheel * 40.0
            } else {
                wheel
            };
            a.t -= step;
        }
        a.t = a.t.clamp(0.0, max);
        a.v = approach(a.v, a.t, dt, 0.05).clamp(0.0, max);
        let offset = a.v;
        self.push_clip(view);
        ScrollArea {
            id,
            view,
            offset,
            content_h,
        }
    }

    /// Finish the scroll area: pop the clip and draw/drag the scrollbar.
    pub fn end_scroll(&mut self, s: ScrollArea) {
        self.pop_clip();
        if s.content_h <= s.view.h + 0.5 {
            return;
        }
        let max = s.content_h - s.view.h;
        let track = Rect::new(s.view.right() - 5.0, s.view.y + 3.0, 3.0, s.view.h - 6.0);
        let th = (track.h * s.view.h / s.content_h).max(24.0);
        let ty = track.y + (track.h - th) * (s.offset / max);
        let thumb = Rect::new(track.x - 2.0, ty, 7.0, th);
        let resp = self.interact(s.id.with("thumb"), thumb.grow(2.0));
        if resp.pressed {
            let dy = self.mouse_delta().y;
            let a = self.anim(s.id);
            a.t = (a.t + dy * max / (track.h - th).max(1.0)).clamp(0.0, max);
            a.v = a.t;
        }
        let a = 0.16 + 0.22 * resp.hover_t.max(resp.press_t);
        fill_rrect(
            Rect::new(track.x, ty, 3.0, th),
            1.5,
            Color::new(1.0, 1.0, 1.0, a),
        );
    }

    /// Scroll so that `y_in_content` (0 = top of content) is visible.
    pub fn scroll_to(&mut self, s: &ScrollArea, y_in_content: f32, h: f32) {
        let a = self.anim(s.id);
        if y_in_content < a.t {
            a.t = y_in_content;
        } else if y_in_content + h > a.t + s.view.h {
            a.t = y_in_content + h - s.view.h;
        }
    }

    /// Jump to the bottom (chat logs).
    pub fn scroll_to_end(&mut self, key: impl Hash, content_h: f32, view_h: f32) {
        let id = self.id(key);
        let a = self.anim(id);
        a.t = (content_h - view_h).max(0.0);
    }

    // -----------------------------------------------------------------------
    // Bars, chips, avatars, labels
    // -----------------------------------------------------------------------

    /// Generic progress bar (pill). `color` fills with a soft gradient.
    pub fn progress(&mut self, r: Rect, value: f32, color: Color) {
        let rad = r.h / 2.0;
        fill_rrect(r, rad, Color::new(1.0, 1.0, 1.0, 0.08));
        let v = value.clamp(0.0, 1.0);
        if v > 0.0 {
            let f = Rect::new(r.x, r.y, (r.w * v).max(r.h), r.h);
            fill_rrect(
                f,
                rad,
                Paint::horizontal(f, mix(color, BLACK, 0.12), mix(color, WHITE, 0.12)),
            );
            if r.h >= 5.0 {
                hline(
                    f.x + rad,
                    f.right() - rad,
                    f.y + 0.5,
                    Color::new(1.0, 1.0, 1.0, 0.22),
                );
            }
        }
    }

    /// Need bar coloured per need; turns red and pulses below 25 %.
    pub fn need_bar(&mut self, r: Rect, need: Need, value: f32) {
        let t = &self.theme;
        let base = match need {
            Need::Energy => t.need_energy,
            Need::Mood => t.need_mood,
            Need::Hunger => t.need_hunger,
        };
        let (c, pulse) = if value < 0.25 {
            let p = 0.75 + 0.25 * ((self.time() * std::f64::consts::TAU).sin() as f32);
            (t.red_400, p)
        } else {
            (base, 1.0)
        };
        draw::push_alpha(pulse);
        self.progress(r, value, c);
        draw::pop_alpha();
    }

    pub fn chip_width(&self, text: &str, icon: bool) -> f32 {
        self.fonts.measure(text, &self.theme.small_medium()) + 20.0 + if icon { 18.0 } else { 0.0 }
    }

    /// Pill chip (h 22) with tone tint; returns its rect so chips can flow.
    pub fn chip(&mut self, pos: Vec2, text: &str, tone: Tone, icon: Option<Icon>) -> Rect {
        let t = &self.theme;
        let w = self.chip_width(text, icon.is_some());
        let r = Rect::new(pos.x, pos.y, w, size::CHIP);
        let fg = if tone == Tone::Neutral {
            t.text
        } else {
            t.tone_fg(tone)
        };
        fill_rrect(r, r.h / 2.0, t.tone_bg(tone));
        stroke_rrect(
            r,
            r.h / 2.0,
            1.0,
            if tone == Tone::Neutral {
                Color::new(1.0, 1.0, 1.0, 0.06)
            } else {
                with_a(t.tone_solid(tone), 0.22)
            },
        );
        let mut x = r.x + 10.0;
        if let Some(i) = icon {
            draw_icon(i, vec2(x + 6.0, r.center_y()), 13.0, fg);
            x += 18.0;
        }
        let st = t.small_medium().color(fg);
        self.fonts
            .draw(text, x, self.fonts.baseline_in(r, &st), &st);
        r
    }

    /// Circle avatar with initials. `color` defaults to [`avatar_color`] of the name.
    pub fn avatar(&mut self, c: Vec2, d: f32, name: &str, color: Option<Color>, ring: bool) {
        let t = &self.theme;
        let col = color.unwrap_or_else(|| avatar_color(name));
        let r = d / 2.0;
        let bb = Rect::new(c.x - r, c.y - r, d, d);
        if ring {
            stroke_circle(c, r + 2.5, 1.5, t.indigo_300);
        }
        fill_circle(
            c,
            r,
            Paint::vertical(bb, mix(col, WHITE, 0.18), mix(col, BLACK, 0.28)),
        );
        stroke_circle(
            c,
            r - 0.5,
            1.0,
            Paint::vertical(
                bb,
                Color::new(1.0, 1.0, 1.0, 0.35),
                Color::new(1.0, 1.0, 1.0, 0.0),
            ),
        );
        if let Some(tex) = crate::render::faces::face_texture(name) {
            // the person's face, cut to the circle, with the lit rim on top
            crate::render::faces::draw_face_circle(&tex, c, d - 1.0, super::draw::faded(WHITE));
            stroke_circle(
                c,
                r - 0.5,
                1.0,
                Paint::vertical(
                    bb,
                    Color::new(1.0, 1.0, 1.0, 0.40),
                    Color::new(1.0, 1.0, 1.0, 0.06),
                ),
            );
            return;
        }
        let st = TextStyle::new(
            (d * 0.38).clamp(9.0, 22.0),
            super::text::Weight::SemiBold,
            WHITE,
        );
        let ini = initials(name);
        self.fonts
            .draw_in(&ini, bb.translate(vec2(0.0, 0.0)), &st, Align::Center);
    }

    /// UPPERCASE section header (micro style) with a hairline to the right.
    pub fn section(&mut self, r: Rect, title: &str) {
        let st = self.theme.micro();
        let up = title.to_uppercase();
        let w = self
            .fonts
            .draw(&up, r.x, self.fonts.baseline_in(r, &st), &st);
        hline(r.x + w + 8.0, r.right(), r.center_y(), self.theme.divider);
    }

    /// Hairline divider.
    pub fn divider(&mut self, x0: f32, x1: f32, y: f32) {
        hline(x0, x1, y, self.theme.divider);
    }

    /// Text label, vertically centred in `r`, ellipsised.
    pub fn label(&mut self, r: Rect, text: &str, style: &TextStyle, align: Align) -> f32 {
        self.fonts.draw_in(text, r, style, align)
    }

    // -----------------------------------------------------------------------
    // Text input
    // -----------------------------------------------------------------------

    /// Single-line text field (h 32). Click to focus, Enter submits, Esc unfocuses.
    pub fn text_input(
        &mut self,
        key: impl Hash,
        r: Rect,
        text: &mut String,
        placeholder: &str,
        max_chars: usize,
    ) -> TextResponse {
        let id = self.id(key);
        let resp = self.interact(id, r);
        if resp.hovered {
            self.set_hot_cursor(CursorIcon::Text);
        }
        let mut out = TextResponse {
            response: resp,
            ..Default::default()
        };
        let t = self.theme.clone();
        let st = t.body();
        let inner = r.inset_xy(10.0, 0.0);
        if self.mouse_pressed() && resp.hovered {
            self.focus = Some(id);
            self.focus_claimed = true;
            // caret at click position
            let local = self.mouse().x - inner.x + self.text_edit.scroll;
            let mut acc = 0.0;
            let mut idx = 0;
            for c in text.chars() {
                let a = self.fonts.advance(c, &st);
                if acc + a * 0.5 > local {
                    break;
                }
                acc += a;
                idx += 1;
            }
            self.text_edit.caret = idx;
            self.text_edit.blink = self.time();
            self.text_edit.owner = Some(id);
        }
        let focused = self.focus == Some(id);
        if focused {
            self.focus_seen = true;
            if self.text_edit.owner != Some(id) {
                self.text_edit = TextEdit {
                    owner: Some(id),
                    caret: text.chars().count(),
                    ..Default::default()
                };
            }
            let n = text.chars().count();
            let mut caret = self.text_edit.caret.min(n);
            let before = text.clone();
            let chars: Vec<char> = self.typed_chars().to_vec();
            for c in chars {
                if c.is_control() || text.chars().count() >= max_chars {
                    continue;
                }
                let byte = byte_at(text, caret);
                text.insert(byte, c);
                caret += 1;
            }
            if self.raw_repeat(KeyCode::Backspace) && caret > 0 {
                let b = byte_at(text, caret - 1);
                text.remove(b);
                caret -= 1;
            }
            if self.raw_repeat(KeyCode::Delete) && caret < text.chars().count() {
                let b = byte_at(text, caret);
                text.remove(b);
            }
            if self.raw_repeat(KeyCode::Left) {
                caret = caret.saturating_sub(1);
            }
            if self.raw_repeat(KeyCode::Right) {
                caret = (caret + 1).min(text.chars().count());
            }
            if is_key_pressed(KeyCode::Home) {
                caret = 0;
            }
            if is_key_pressed(KeyCode::End) {
                caret = text.chars().count();
            }
            if self.ctrl()
                && is_key_pressed(KeyCode::V)
                && let Some(clip) = macroquad::miniquad::window::clipboard_get()
            {
                for c in clip.chars().filter(|c| !c.is_control()) {
                    if text.chars().count() >= max_chars {
                        break;
                    }
                    let b = byte_at(text, caret);
                    text.insert(b, c);
                    caret += 1;
                }
            }
            if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
                out.submitted = true;
                self.focus = None;
            }
            if *text != before || caret != self.text_edit.caret {
                self.text_edit.blink = self.time();
            }
            out.changed = *text != before;
            self.text_edit.caret = caret;
        }
        out.focused = focused;
        let f = self.ease_value(id, if focused { 1.0 } else { 0.0 }, 0.06);
        fill_rrect(r, radius::S, Color::new(0.03, 0.02, 0.09, 0.55));
        stroke_rrect(
            r,
            radius::S,
            1.0,
            mix(
                Color::new(1.0, 1.0, 1.0, 0.12 + 0.06 * resp.hover_t),
                t.indigo_400,
                f,
            ),
        );
        if f > 0.01 {
            shadow_rrect(
                r,
                radius::S,
                draw::Shadow::new(0.0, 0.0, 10.0, 0.0, with_a(t.indigo, 0.35 * f)),
            );
            stroke_rrect(
                r.grow(2.0),
                radius::S + 2.0,
                1.5,
                with_a(t.indigo, 0.35 * f),
            );
        }
        self.push_clip(inner.inset_xy(-2.0, 1.0));
        let base = self.fonts.baseline_in(r, &st);
        if text.is_empty() {
            self.fonts
                .draw(placeholder, inner.x, base, &st.color(t.text_disabled));
        }
        let caret_x_local = if focused {
            let prefix: String = text.chars().take(self.text_edit.caret).collect();
            self.fonts.measure(&prefix, &st)
        } else {
            0.0
        };
        if focused {
            let s = &mut self.text_edit.scroll;
            if caret_x_local - *s > inner.w - 2.0 {
                *s = caret_x_local - inner.w + 2.0;
            }
            if caret_x_local - *s < 0.0 {
                *s = caret_x_local;
            }
        }
        let scroll = if focused { self.text_edit.scroll } else { 0.0 };
        self.fonts.draw(text, inner.x - scroll, base, &st);
        if focused && ((self.time() - self.text_edit.blink) % 1.0) < 0.55 {
            let x = inner.x - scroll + caret_x_local;
            fill_rrect(
                Rect::new(x, r.center_y() - 8.0, 1.5, 16.0),
                0.75,
                t.indigo_300,
            );
        }
        self.pop_clip();
        out
    }

    // -----------------------------------------------------------------------
    // Dropdown
    // -----------------------------------------------------------------------

    /// Small dropdown. The list opens in an overlay above everything else.
    pub fn dropdown(
        &mut self,
        key: impl Hash,
        r: Rect,
        options: &[&str],
        selected: &mut usize,
    ) -> Response {
        let id = self.id(key);
        let mut resp = self.interact(id, r);
        if let Some((pid, i)) = self.popup_pick
            && pid == id
        {
            self.popup_pick = None;
            if i != *selected {
                *selected = i;
                resp.changed = true;
            }
        }
        let open = self.popup.as_ref().is_some_and(|p| p.owner == id);
        if resp.clicked {
            if open {
                self.popup = None;
            } else {
                self.popup = Some(Popup {
                    owner: id,
                    anchor: r,
                    options: options.iter().map(|s| s.to_string()).collect(),
                    selected: *selected,
                    hover: *selected,
                    opened: self.time(),
                });
            }
            self.sound(UiSound::Click);
        }
        if let Some(p) = self.popup.as_mut().filter(|p| p.owner == id) {
            p.anchor = r;
            p.selected = *selected;
        }
        let t = self.theme.clone();
        paint_surface(
            &t,
            r,
            radius::S,
            ButtonKind::Secondary,
            false,
            resp.hover_t.max(if open { 1.0 } else { 0.0 }),
            resp.press_t,
        );
        let st = t.body_medium();
        let label = options.get(*selected).copied().unwrap_or("");
        self.fonts
            .draw_in(label, r.inset_ltrb(10.0, 0.0, 28.0, 0.0), &st, Align::Left);
        draw_icon(
            if open {
                Icon::ChevronUp
            } else {
                Icon::ChevronDown
            },
            vec2(r.right() - 15.0, r.center_y()),
            14.0,
            t.text_dim,
        );
        resp
    }

    pub(crate) fn draw_popup(&mut self) {
        let Some(mut p) = self.popup.take() else {
            return;
        };
        let t = self.theme.clone();
        let row_h = 28.0;
        let h = p.options.len() as f32 * row_h + 8.0;
        let w = p.anchor.w.max(140.0);
        let below = p.anchor.bottom() + 4.0;
        let y = if below + h > self.screen().h - 8.0 {
            p.anchor.y - 4.0 - h
        } else {
            below
        };
        let r = Rect::new(p.anchor.x, y, w, h);
        let k = ease_out(((self.time() - p.opened) / 0.12) as f32);
        self.region(r);
        draw::push_alpha(k);
        draw_glass(
            &t,
            r.translate(vec2(0.0, (1.0 - k) * -4.0)),
            &Glass::tooltip(),
        );
        let mut picked = None;
        if self.key_repeat(KeyCode::Down) {
            p.hover = (p.hover + 1).min(p.options.len().saturating_sub(1));
        }
        if self.key_repeat(KeyCode::Up) {
            p.hover = p.hover.saturating_sub(1);
        }
        if self.take_key(KeyCode::Enter) {
            picked = Some(p.hover);
        }
        for (i, opt) in p.options.iter().enumerate() {
            let rr = Rect::new(r.x + 4.0, r.y + 4.0 + i as f32 * row_h, r.w - 8.0, row_h);
            let resp = self.interact(p.owner.with(("opt", i)), rr);
            if resp.hovered && self.mouse_delta().length_squared() > 0.0 {
                p.hover = i;
            }
            if resp.clicked {
                picked = Some(i);
            }
            if i == p.hover {
                fill_rrect(rr, radius::S, Color::new(1.0, 1.0, 1.0, 0.07));
            }
            let on = i == p.selected;
            let st = t.body().color(if on { t.indigo_300 } else { t.text });
            self.fonts
                .draw_in(opt, rr.inset_ltrb(10.0, 0.0, 28.0, 0.0), &st, Align::Left);
            if on {
                draw_icon(
                    Icon::Check,
                    vec2(rr.right() - 14.0, rr.center_y()),
                    14.0,
                    t.indigo_300,
                );
            }
        }
        draw::pop_alpha();
        if let Some(i) = picked {
            self.popup_pick = Some((p.owner, i));
            self.sound(UiSound::Click);
            return;
        }
        let inside = r.contains(self.mouse()) || p.anchor.contains(self.mouse());
        if self.mouse_pressed() && !inside {
            return; // closed by an outside click
        }
        self.popup = Some(p);
    }
}

fn byte_at(s: &str, char_idx: usize) -> usize {
    s.char_indices().nth(char_idx).map_or(s.len(), |(b, _)| b)
}

/// Deterministic avatar colour for a name.
pub fn avatar_color(name: &str) -> Color {
    let i = (Id::new(name).0 % Theme::AVATARS.len() as u64) as usize;
    Theme::AVATARS[i]
}

/// Tab description for [`Ui::tabs`].
#[derive(Clone, Copy, Debug)]
pub struct Tab<'a> {
    pub label: &'a str,
    pub icon: Option<Icon>,
    pub badge: Option<u32>,
}

impl<'a> Tab<'a> {
    pub fn new(label: &'a str) -> Self {
        Tab {
            label,
            icon: None,
            badge: None,
        }
    }
    pub fn icon(mut self, i: Icon) -> Self {
        self.icon = Some(i);
        self
    }
    pub fn badge(mut self, n: u32) -> Self {
        self.badge = Some(n);
        self
    }
}

/// Leading element of a list row.
#[derive(Clone, Copy, Debug)]
pub enum Leading<'a> {
    None,
    Icon(Icon, Color),
    Avatar {
        name: &'a str,
        color: Option<Color>,
        /// Small status dot (on shift / break / off).
        status: Option<Color>,
    },
}

/// List row description.
#[derive(Clone, Copy, Debug)]
pub struct ListRow<'a> {
    pub title: &'a str,
    pub subtitle: Option<&'a str>,
    pub leading: Leading<'a>,
    pub trailing: Option<&'a str>,
    pub trailing_tone: Option<Tone>,
    pub selected: bool,
    pub chevron: bool,
    pub dim: bool,
}

impl<'a> ListRow<'a> {
    pub fn new(title: &'a str) -> Self {
        ListRow {
            title,
            subtitle: None,
            leading: Leading::None,
            trailing: None,
            trailing_tone: None,
            selected: false,
            chevron: false,
            dim: false,
        }
    }
    pub fn subtitle(mut self, s: &'a str) -> Self {
        self.subtitle = Some(s);
        self
    }
    pub fn avatar(mut self, name: &'a str, status: Option<Color>) -> Self {
        self.leading = Leading::Avatar {
            name,
            color: None,
            status,
        };
        self
    }
    pub fn icon(mut self, i: Icon, c: Color) -> Self {
        self.leading = Leading::Icon(i, c);
        self
    }
    pub fn trailing(mut self, v: &'a str, tone: Tone) -> Self {
        self.trailing = Some(v);
        self.trailing_tone = Some(tone);
        self
    }
    pub fn selected(mut self, s: bool) -> Self {
        self.selected = s;
        self
    }
    pub fn chevron(mut self) -> Self {
        self.chevron = true;
        self
    }
    pub fn dim(mut self, d: bool) -> Self {
        self.dim = d;
        self
    }
}

/// Need kinds for [`Ui::need_bar`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Need {
    Energy,
    Mood,
    Hunger,
}

/// Handle returned by [`Ui::begin_scroll`].
#[derive(Clone, Copy, Debug)]
pub struct ScrollArea {
    pub id: Id,
    pub view: Rect,
    /// Current scroll offset (content moved up by this much).
    pub offset: f32,
    pub content_h: f32,
}

impl ScrollArea {
    /// Y where content row 0 should be drawn.
    pub fn top(&self) -> f32 {
        self.view.y - self.offset
    }
    /// Is a content band `[y, y+h)` (absolute) visible? Skip drawing it otherwise.
    pub fn visible(&self, y: f32, h: f32) -> bool {
        y + h >= self.view.y && y <= self.view.bottom()
    }
}

/// Result of [`Ui::text_input`].
#[derive(Clone, Copy, Debug, Default)]
pub struct TextResponse {
    pub response: Response,
    pub changed: bool,
    pub submitted: bool,
    pub focused: bool,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct TextEdit {
    pub owner: Option<Id>,
    pub caret: usize,
    pub scroll: f32,
    pub blink: f64,
}

#[derive(Clone, Debug)]
pub(crate) struct Popup {
    pub owner: Id,
    pub anchor: Rect,
    pub options: Vec<String>,
    pub selected: usize,
    pub hover: usize,
    pub opened: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_index_handles_multibyte() {
        let s = "žluťoučký";
        assert_eq!(byte_at(s, 0), 0);
        assert_eq!(byte_at(s, 1), 'ž'.len_utf8());
        assert_eq!(byte_at(s, 100), s.len());
    }

    #[test]
    fn avatar_colour_is_deterministic() {
        assert_eq!(avatar_color("Lída").r, avatar_color("Lída").r);
    }
}
