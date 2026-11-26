//! Toast stack: non-blocking notifications with optional actions and auto-dismiss.
//!
//! ```ignore
//! ui.push_toast(Toast::new(ToastKind::Event, "Vlak Rn 333 přijíždí")
//!     .body("~8 hostů za 4 minuty").icon(Icon::Train).action("Ukázat", 1));
//! for ev in ui.draw_toasts(ToastAnchor::BottomRight) { if ev.tag == 1 { /* … */ } }
//! ```

use macroquad::prelude::*;

use super::context::{Ui, UiSound, approach, ease_out};
use super::draw::{self, fill_circle, fill_rrect, with_a};
use super::glass::{Glass, draw_glass};
use super::icons::{Icon, draw_icon};
use super::layout::RectExt;
use super::text::Align;
use super::theme::Tone;
use super::widgets::{Button, IconButton};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToastKind {
    Info,
    Success,
    Warning,
    /// Story / world events (violet).
    Event,
}

impl ToastKind {
    pub fn tone(self) -> Tone {
        match self {
            ToastKind::Info => Tone::Info,
            ToastKind::Success => Tone::Ok,
            ToastKind::Warning => Tone::Warn,
            ToastKind::Event => Tone::Violet,
        }
    }
    pub fn default_icon(self) -> Icon {
        match self {
            ToastKind::Info => Icon::Info,
            ToastKind::Success => Icon::Check,
            ToastKind::Warning => Icon::Warning,
            ToastKind::Event => Icon::Sparkle,
        }
    }
    /// Seconds on screen (hover pauses the timer).
    pub fn default_duration(self) -> f32 {
        match self {
            ToastKind::Info => 6.0,
            ToastKind::Success => 5.0,
            ToastKind::Warning => 10.0,
            ToastKind::Event => 8.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ToastAction {
    pub label: String,
    pub tag: u32,
    pub primary: bool,
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub kind: ToastKind,
    pub title: String,
    pub body: Option<String>,
    pub icon: Option<Icon>,
    pub actions: Vec<ToastAction>,
    /// None = kind default.
    pub duration: Option<f32>,
    /// Never auto-dismiss.
    pub sticky: bool,
}

impl Toast {
    pub fn new(kind: ToastKind, title: impl Into<String>) -> Self {
        Toast {
            kind,
            title: title.into(),
            body: None,
            icon: None,
            actions: Vec::new(),
            duration: None,
            sticky: false,
        }
    }
    pub fn body(mut self, b: impl Into<String>) -> Self {
        self.body = Some(b.into());
        self
    }
    pub fn icon(mut self, i: Icon) -> Self {
        self.icon = Some(i);
        self
    }
    /// Secondary action button; `tag` comes back in [`ToastEvent`].
    pub fn action(mut self, label: impl Into<String>, tag: u32) -> Self {
        self.actions.push(ToastAction {
            label: label.into(),
            tag,
            primary: false,
        });
        self
    }
    pub fn primary_action(mut self, label: impl Into<String>, tag: u32) -> Self {
        self.actions.push(ToastAction {
            label: label.into(),
            tag,
            primary: true,
        });
        self
    }
    pub fn duration(mut self, secs: f32) -> Self {
        self.duration = Some(secs);
        self
    }
    pub fn sticky(mut self) -> Self {
        self.sticky = true;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ToastId(pub u64);

/// An action button was clicked (the toast closes itself).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToastEvent {
    pub toast: ToastId,
    pub tag: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastAnchor {
    TopRight,
    BottomRight,
    BottomCenter,
}

#[derive(Clone, Debug)]
struct Live {
    id: ToastId,
    toast: Toast,
    shown: f32,
    enter: f32,
    leave: Option<f32>,
    y: Option<f32>,
}

#[derive(Default)]
pub(crate) struct ToastState {
    live: Vec<Live>,
    next: u64,
    /// Extra distance from the bottom edge (e.g. above the build panel).
    pub bottom_inset: f32,
    /// Extra distance from the right edge (e.g. left of the day planning panel).
    pub right_inset: f32,
}

const WIDTH: f32 = 320.0;
const MAX_VISIBLE: usize = 4;

impl Ui {
    /// Queue a toast (plays `UiSound::Toast`). Returns its id for `dismiss_toast`.
    pub fn push_toast(&mut self, t: Toast) -> ToastId {
        let id = ToastId(self.toasts.next);
        self.toasts.next += 1;
        self.sound(UiSound::Toast(t.kind));
        self.toasts.live.push(Live {
            id,
            toast: t,
            shown: 0.0,
            enter: 0.0,
            leave: None,
            y: None,
        });
        // older ones beyond the visible limit start leaving
        let alive: Vec<usize> = (0..self.toasts.live.len())
            .filter(|&i| self.toasts.live[i].leave.is_none())
            .collect();
        if alive.len() > MAX_VISIBLE {
            for &i in &alive[..alive.len() - MAX_VISIBLE] {
                self.toasts.live[i].leave = Some(0.0);
            }
        }
        id
    }

    pub fn dismiss_toast(&mut self, id: ToastId) {
        if let Some(l) = self.toasts.live.iter_mut().find(|l| l.id == id) {
            l.leave.get_or_insert(0.0);
        }
    }

    pub fn clear_toasts(&mut self) {
        for l in &mut self.toasts.live {
            l.leave.get_or_insert(0.0);
        }
    }

    /// Keep the stack left of a side panel (`v` = its width from the right screen edge).
    pub fn set_toast_right_inset(&mut self, v: f32) {
        self.toasts.right_inset = v.max(0.0);
    }

    /// Lift the bottom-anchored stack (e.g. while the build panel is open).
    pub fn set_toast_bottom_inset(&mut self, v: f32) {
        self.toasts.bottom_inset = v;
    }

    /// Action buttons laid out in rows that fit the text column (`(index, width)` per row).
    fn action_rows(&self, t: &Toast) -> Vec<Vec<(usize, f32)>> {
        let tw = WIDTH - 52.0 - 14.0;
        let mut rows: Vec<Vec<(usize, f32)>> = Vec::new();
        let mut x = 0.0;
        for (k, a) in t.actions.iter().enumerate() {
            let b = if a.primary {
                Button::new(&a.label).primary()
            } else {
                Button::new(&a.label).secondary()
            };
            let w = self.button_width(&b).min(tw);
            match rows.last_mut() {
                Some(row) if x + w <= tw => {
                    row.push((k, w));
                    x += w + 6.0;
                }
                _ => {
                    rows.push(vec![(k, w)]);
                    x = w + 6.0;
                }
            }
        }
        rows
    }

    fn toast_height(&self, t: &Toast) -> f32 {
        let body_lines = t.body.as_ref().map_or(0, |b| {
            self.fonts
                .wrap(b, WIDTH - 52.0 - 14.0, &self.theme.small())
                .len()
                .min(4)
        });
        let text_h = 18.0 + body_lines as f32 * 16.0;
        let mut h = 12.0 + text_h.max(28.0) + 12.0;
        h += self.action_rows(t).len() as f32 * 30.0;
        h
    }

    /// Draw the stack and return clicked actions. Call once per frame, near the end.
    pub fn draw_toasts(&mut self, anchor: ToastAnchor) -> Vec<ToastEvent> {
        let mut events = Vec::new();
        let dt = self.dt();
        let screen = self.screen();
        let margin = 12.0;
        let gap = 8.0;
        // advance timers, drop finished
        self.toasts.live.retain(|l| l.leave.is_none_or(|v| v < 1.0));
        let heights: Vec<f32> = self
            .toasts
            .live
            .iter()
            .map(|l| self.toast_height(&l.toast))
            .collect();
        let n = self.toasts.live.len();
        // target positions: newest closest to the anchor edge
        let mut targets = vec![0.0; n];
        let mut cursor = match anchor {
            ToastAnchor::TopRight => margin,
            _ => screen.h - margin - self.toasts.bottom_inset,
        };
        for i in (0..n).rev() {
            let h = heights[i];
            let leaving = self.toasts.live[i].leave.is_some();
            match anchor {
                ToastAnchor::TopRight => {
                    targets[i] = cursor;
                    if !leaving {
                        cursor += h + gap;
                    }
                }
                _ => {
                    targets[i] = cursor - h;
                    if !leaving {
                        cursor -= h + gap;
                    }
                }
            }
        }
        let x_base = match anchor {
            ToastAnchor::BottomCenter => (screen.w - WIDTH) / 2.0,
            _ => screen.w - WIDTH - margin - self.toasts.right_inset,
        };
        for i in 0..n {
            let h = heights[i];
            let (id, kind, mut enter, leave, y) = {
                let l = &mut self.toasts.live[i];
                l.enter = (l.enter + dt / 0.18).min(1.0);
                if let Some(v) = l.leave.as_mut() {
                    *v += dt / 0.2;
                }
                let y = match l.y {
                    Some(y) => approach(y, targets[i], dt, 0.07),
                    None => targets[i],
                };
                l.y = Some(y);
                (l.id, l.toast.kind, l.enter, l.leave, y)
            };
            let leave_k = leave.map_or(0.0, |v| ease_out(v.min(1.0)));
            enter = ease_out(enter);
            let alpha = (enter * (1.0 - leave_k)).clamp(0.0, 1.0);
            let slide = (1.0 - enter) * 28.0 + leave_k * 28.0;
            let r = Rect::new(x_base + slide, y, WIDTH, h);
            self.push_id(("toast", id.0));
            self.region(r);
            let hovered = self.hovering(r);
            if !hovered && leave.is_none() {
                let l = &mut self.toasts.live[i];
                l.shown += dt;
                let dur = l.toast.duration.unwrap_or(kind.default_duration());
                if !l.toast.sticky && l.shown >= dur {
                    l.leave = Some(0.0);
                }
            }
            draw::push_alpha(alpha);
            let ev = self.draw_one_toast(i, r, hovered);
            draw::pop_alpha();
            self.pop_id();
            if let Some(tag) = ev {
                events.push(ToastEvent { toast: id, tag });
                self.toasts.live[i].leave.get_or_insert(0.0);
            }
        }
        events
    }

    fn draw_one_toast(&mut self, i: usize, r: Rect, hovered: bool) -> Option<u32> {
        let t = self.theme.clone();
        let toast = self.toasts.live[i].toast.clone();
        let shown = self.toasts.live[i].shown;
        let tone = toast.kind.tone();
        let solid = t.tone_solid(tone);
        let fg = t.tone_fg(tone);
        let mut g = Glass::hud().solid();
        if toast.kind == ToastKind::Event {
            g = g.tint(t.violet);
        }
        draw_glass(&t, r, &g);
        // category stripe
        let stripe = Rect::new(r.x, r.y + 10.0, 3.0, 28.0);
        fill_rrect(stripe, 1.5, solid);
        // icon disc
        let c = vec2(r.x + 28.0, r.y + 12.0 + 14.0);
        fill_circle(c, 14.0, with_a(solid, 0.16));
        draw_icon(toast.icon.unwrap_or(toast.kind.default_icon()), c, 16.0, fg);
        let tx = r.x + 52.0;
        let tw = r.right() - 14.0 - tx;
        let title_w = if hovered { tw - 22.0 } else { tw };
        self.fonts.draw_in(
            &toast.title,
            Rect::new(tx, r.y + 12.0, title_w, 18.0),
            &t.strong(),
            Align::Left,
        );
        let mut y = r.y + 12.0 + 18.0;
        if let Some(b) = &toast.body {
            let used = self.fonts.draw_wrapped(
                b,
                Rect::new(tx, y, tw, 64.0),
                &t.small().line_height(1.33),
                Align::Left,
                Some(4),
            );
            y += used;
        }
        let mut result = None;
        if hovered {
            let cr = Rect::new(r.right() - 30.0, r.y + 8.0, 22.0, 22.0);
            if self
                .icon_button("close", cr, Icon::Close, &IconButton::ghost().size(14.0))
                .clicked
            {
                self.toasts.live[i].leave.get_or_insert(0.0);
            }
        }
        // buttons wrap onto more rows instead of running out of the toast
        let rows = self.action_rows(&toast);
        let mut by = (y + 6.0).max(r.bottom() - 12.0 - rows.len() as f32 * 30.0 + 6.0);
        for row in rows {
            let mut x = tx;
            for (k, w) in row {
                let a = &toast.actions[k];
                let b = if a.primary {
                    Button::new(&a.label).primary()
                } else {
                    Button::new(&a.label).secondary()
                };
                let br = Rect::new(x, by, w, 24.0);
                if self.button(("act", k), br, &b).clicked {
                    result = Some(a.tag);
                }
                x += w + 6.0;
            }
            by += 30.0;
        }
        // auto-dismiss timer
        if !toast.sticky {
            let dur = toast.duration.unwrap_or(toast.kind.default_duration());
            let left = (1.0 - shown / dur).clamp(0.0, 1.0);
            let bar = Rect::new(r.x + 12.0, r.bottom() - 3.0, (r.w - 24.0) * left, 2.0);
            if bar.w > 1.0 {
                fill_rrect(bar, 1.0, with_a(solid, 0.55));
            }
        }
        result
    }
}
