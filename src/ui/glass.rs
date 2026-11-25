//! Frosted-glass panels. macroquad has no backdrop blur, so the look is faked:
//! soft multi-layer drop shadow → translucent vertical gradient with fine grain →
//! diagonal sheen in the top-left → 1 pt lit edge (bright top, faint bottom) →
//! a faint inner top highlight.

use macroquad::prelude::*;

use super::context::Ui;
use super::draw::{
    Paint, clip_convex, fill_convex, fill_rrect, fill_rrect_grain, mix, rounded_rect_path,
    shadow_rrect, stroke_rrect, with_a,
};
use super::icons::Icon;
use super::layout::RectExt;
use super::text::Align;
use super::theme::{Theme, radius};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Elevation {
    /// No drop shadow (inline cards).
    Flat,
    /// HUD blocks, toasts.
    Low,
    /// Big panels, phone, dialogs.
    High,
}

/// Glass panel style.
#[derive(Clone, Copy, Debug)]
pub struct Glass {
    pub radius: f32,
    pub elevation: Elevation,
    /// More opaque fill (menus, tooltips, text-heavy panels).
    pub solid: bool,
    /// Optional colour wash mixed into the fill (event toasts, danger banners).
    pub tint: Option<Color>,
    pub sheen: bool,
    pub grain: bool,
}

impl Default for Glass {
    fn default() -> Self {
        Glass {
            radius: radius::XL,
            elevation: Elevation::High,
            solid: false,
            tint: None,
            sheen: true,
            grain: true,
        }
    }
}

impl Glass {
    /// Large panel (radius 12, high shadow).
    pub fn panel() -> Self {
        Glass::default()
    }
    /// HUD card (radius 10, low shadow).
    pub fn hud() -> Self {
        Glass {
            radius: radius::L,
            elevation: Elevation::Low,
            ..Glass::default()
        }
    }
    /// Inline card inside a panel.
    pub fn card() -> Self {
        Glass {
            radius: radius::M,
            elevation: Elevation::Flat,
            sheen: false,
            ..Glass::default()
        }
    }
    pub fn tooltip() -> Self {
        Glass {
            radius: radius::M,
            elevation: Elevation::Low,
            solid: true,
            sheen: false,
            ..Glass::default()
        }
    }
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r;
        self
    }
    pub fn solid(mut self) -> Self {
        self.solid = true;
        self
    }
    pub fn tint(mut self, c: Color) -> Self {
        self.tint = Some(c);
        self
    }
    pub fn elevation(mut self, e: Elevation) -> Self {
        self.elevation = e;
        self
    }
}

/// Paint a glass panel (no interaction). Use [`Ui::glass`] to also block the mouse.
pub fn draw_glass(t: &Theme, r: Rect, g: &Glass) {
    let rad = g.radius;
    match g.elevation {
        Elevation::Flat => {}
        Elevation::Low => {
            shadow_rrect(r, rad, t.shadow_ambient);
            shadow_rrect(r, rad, t.shadow_md);
        }
        Elevation::High => {
            shadow_rrect(r, rad, t.shadow_ambient);
            shadow_rrect(r, rad, t.shadow_md);
            shadow_rrect(r, rad, t.shadow_lg);
        }
    }
    let (mut top, mut bottom) = if g.solid {
        (t.glass_solid_top, t.glass_solid_bottom)
    } else {
        (t.glass_top, t.glass_bottom)
    };
    if g.elevation == Elevation::Flat {
        top = with_a(top, top.a * 0.55);
        bottom = with_a(bottom, bottom.a * 0.55);
    }
    if let Some(tint) = g.tint {
        top = mix(top, with_a(tint, top.a), 0.22);
        bottom = mix(bottom, with_a(tint, bottom.a), 0.12);
    }
    let fill = Paint::vertical(r, top, bottom);
    if g.grain {
        fill_rrect_grain(r, rad, fill);
    } else {
        fill_rrect(r, rad, fill);
    }
    if g.sheen {
        sheen(t, r, rad);
    }
    // inner top glow (light catching the upper bevel)
    stroke_rrect(
        r.inset(1.0),
        (rad - 1.0).max(0.0),
        1.0,
        Paint::Linear {
            from: vec2(r.x, r.y),
            to: vec2(r.x, r.y + r.h.min(36.0)),
            c0: Color::new(1.0, 1.0, 1.0, 0.06),
            c1: Color::new(1.0, 1.0, 1.0, 0.0),
        },
    );
    // lit 1 pt edge
    let (et, eb) = match g.tint {
        Some(c) => (
            mix(t.edge_top, with_a(c, t.edge_top.a), 0.35),
            t.edge_bottom,
        ),
        None => (t.edge_top, t.edge_bottom),
    };
    stroke_rrect(r, rad, 1.0, Paint::vertical(r, et, eb));
}

/// Diagonal light streak in the top-left corner, fading to nothing.
fn sheen(t: &Theme, r: Rect, rad: f32) {
    let wc = (r.w * 0.85).min(420.0);
    let hc = (r.h * 0.9).min(wc * 0.55);
    let a = vec2(r.x, r.y + hc);
    let b = vec2(r.x + wc, r.y);
    let n = vec2(b.y - a.y, a.x - b.x) * -1.0;
    let n = n.normalize_or_zero();
    let n = if n.x < 0.0 { -n } else { n };
    let region = clip_convex(&rounded_rect_path(r, rad), a, n);
    if region.len() < 3 {
        return;
    }
    let tl = vec2(r.x, r.y);
    let dist = (a - tl).dot(n);
    fill_convex(
        &region,
        Paint::Linear {
            from: tl,
            to: tl + n * dist,
            c0: t.sheen,
            c1: with_a(t.sheen, 0.0),
        },
    );
}

impl Ui {
    /// Glass panel that blocks the mouse for everything under it.
    pub fn glass(&mut self, r: Rect, g: &Glass) {
        self.region(r);
        draw_glass(&self.theme, r, g);
    }

    /// Standard panel header: title (+ keycap, + subtitle) and a close button.
    /// Returns (content rect below the divider, close clicked). Esc is not handled here.
    pub fn panel_header(
        &mut self,
        panel: Rect,
        title: &str,
        shortcut: Option<&str>,
        subtitle: Option<&str>,
    ) -> (Rect, bool) {
        let t = self.theme.clone();
        let head = Rect::new(panel.x, panel.y, panel.w, 44.0);
        let style = t.h1();
        let base = self.fonts.baseline_in(head, &style);
        let mut x = panel.x + 16.0;
        x += self.fonts.draw(title, x, base, &style) + 8.0;
        if let Some(k) = shortcut {
            let kr = self.keycap(vec2(x, head.center_y()), k, false);
            x = kr.right() + 8.0;
        }
        if let Some(s) = subtitle {
            let st = t.small();
            let avail = panel.right() - 48.0 - x;
            self.fonts.draw_in(
                s,
                Rect::new(x, head.y, avail.max(0.0), head.h),
                &st,
                Align::Left,
            );
        }
        let close = Rect::new(panel.right() - 12.0 - 24.0, panel.y + 10.0, 24.0, 24.0);
        let closed = self
            .icon_button(
                ("panel_close", title),
                close,
                Icon::Close,
                &super::widgets::IconButton::ghost().size(16.0),
            )
            .clicked;
        super::draw::hline(
            panel.x + 1.0,
            panel.right() - 1.0,
            panel.y + 44.0,
            t.divider,
        );
        (
            Rect::new(panel.x, panel.y + 45.0, panel.w, panel.h - 45.0),
            closed,
        )
    }
}
