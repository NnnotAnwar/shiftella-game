//! Coach marks (design §8): a small glass tip with an arrow at the thing it talks about
//! and a soft pulsing ring around it, **Next** and **Skip**. Never blocks input. Body text
//! can be one of Lida's notes (amber, handwritten). Same look as the first-night tutorial.

use macroquad::prelude::*;

use super::common::{draw_handwritten_block, handwritten_height};
use crate::ui::draw::{self, Paint, fill_convex, fill_rrect, stroke_rrect, with_a};
use crate::ui::layout::RectExt;
use crate::ui::{Align, Button, Glass, Icon, TextStyle, Ui, Weight, draw_icon};

#[derive(Clone, Debug)]
pub struct CoachMark {
    /// "TUTORIAL · 2/5" (shown uppercase).
    pub label: String,
    pub title: Option<String>,
    pub body: String,
    /// The body is Lida's note: amber, handwritten.
    pub lida: bool,
    /// What it points at (UI units); None = centred card.
    pub target: Option<Rect>,
    /// "Got it" / "Next".
    pub next: String,
    /// "Skip tutorial" (None hides it).
    pub skip: Option<String>,
    /// Pulse a ring around the target (off when the target is a whole panel).
    pub ring: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoachAction {
    Next,
    Skip,
}

const W: f32 = 300.0;

/// Draw the coach mark. `avoid` lists rects the card should not cover (HUD blocks).
/// **Enter** = Next unless `enter_is_next` is false (when the step itself asks for Enter).
pub fn coach_mark(
    ui: &mut Ui,
    key: impl std::hash::Hash,
    m: &CoachMark,
    avoid: &[Rect],
    enter_is_next: bool,
) -> Option<CoachAction> {
    let t = ui.theme.clone();
    let id = ui.id(key);
    let s = ui.screen();
    let w = W.min(s.w - 24.0);
    let body_st = if m.lida {
        TextStyle::new(14.0, Weight::Medium, t.amber).line_height(1.4)
    } else {
        t.small().size(13.0).line_height(1.45).color(t.text_dim)
    };
    let body_h = if m.lida {
        handwritten_height(ui, &m.body, w - 32.0, &body_st, 5)
    } else {
        ui.fonts
            .wrapped_height(&m.body, w - 32.0, &body_st, Some(5))
    };
    let title_h = if m.title.is_some() { 26.0 } else { 4.0 };
    let h = 16.0 + 16.0 + title_h + body_h + 14.0 + 28.0 + 14.0;
    let (r, tip) = match m.target {
        Some(tr) => {
            let (r, tip) = place(s, tr, w, h, avoid);
            (r, Some((tr, tip)))
        }
        None => (
            Rect::new(
                ((s.w - w) / 2.0).round(),
                (s.h * 0.34 - h / 2.0).round(),
                w,
                h,
            ),
            None,
        ),
    };
    let k = ui.ease_value(id.with("appear"), 1.0, 0.08);
    let r = r.translate(vec2(0.0, (1.0 - k) * 6.0));
    draw::push_alpha(k);
    if let Some((tr, tip)) = tip {
        if m.ring {
            let pulse = 0.5 + 0.5 * ((ui.time() * 2.4).sin() as f32);
            let ring = tr.grow(4.0 + 3.0 * pulse);
            stroke_rrect(ring, 12.0, 2.0, with_a(t.indigo_300, 0.55 + 0.35 * pulse));
            stroke_rrect(
                ring.grow(4.0),
                14.0,
                1.0,
                with_a(t.violet_400, 0.25 * pulse),
            );
        }
        let from = vec2(
            tip.x.clamp(r.x + 18.0, r.right() - 18.0),
            tip.y.clamp(r.y + 18.0, r.bottom() - 18.0),
        );
        let dir = (tip - from).normalize_or_zero();
        if dir.length() > 0.5 {
            let n = vec2(-dir.y, dir.x);
            let base = edge_point(r, from, dir);
            fill_convex(
                &[base + n * 8.0, base + dir * 11.0, base - n * 8.0],
                Paint::Solid(t.glass_solid_top),
            );
        }
    }
    ui.glass(r, &Glass::tooltip().radius(12.0));
    let accent = Rect::new(r.x, r.y + 14.0, 3.0, 22.0);
    fill_rrect(
        accent,
        1.5,
        Paint::vertical(
            accent,
            if m.lida { t.amber } else { t.indigo_300 },
            if m.lida { t.amber_deep } else { t.violet },
        ),
    );
    let mut y = r.y + 16.0;
    ui.fonts.draw_in(
        &m.label.to_uppercase(),
        Rect::new(r.x + 16.0, y, r.w - 48.0, 14.0),
        &t.micro().color(if m.lida { t.amber } else { t.indigo_300 }),
        Align::Left,
    );
    draw_icon(
        if m.lida { Icon::Note } else { Icon::Sparkle },
        vec2(r.right() - 22.0, y + 7.0),
        13.0,
        with_a(t.amber, 0.9),
    );
    y += 18.0;
    if let Some(title) = &m.title {
        ui.fonts.draw_in(
            title,
            Rect::new(r.x + 16.0, y, r.w - 32.0, 22.0),
            &t.strong().size(15.0),
            Align::Left,
        );
        y += 26.0;
    } else {
        y += 4.0;
    }
    let br = Rect::new(r.x + 16.0, y, r.w - 32.0, body_h);
    if m.lida {
        draw_handwritten_block(ui, &m.body, br, &body_st, Align::Left, 5);
    } else {
        ui.fonts
            .draw_wrapped(&m.body, br, &body_st, Align::Left, Some(5));
    }
    let by = r.bottom() - 14.0 - 28.0;
    let next = Button::new(&m.next).primary().trailing(Icon::ChevronRight);
    let nw = ui.button_width(&next).min(r.w - 32.0);
    let nr = Rect::new(r.right() - 16.0 - nw, by, nw, 28.0);
    let mut action = ui
        .button(id.with("next"), nr, &next)
        .clicked
        .then_some(CoachAction::Next);
    if let Some(skip) = &m.skip {
        let sb = Button::new(skip).ghost();
        let sw = ui.button_width(&sb).min(nr.x - r.x - 20.0);
        if sw > 30.0
            && ui
                .button(id.with("skip"), Rect::new(r.x + 10.0, by, sw, 28.0), &sb)
                .clicked
        {
            action = Some(CoachAction::Skip);
        }
    }
    draw::pop_alpha();
    if enter_is_next && ui.enter() {
        action = Some(CoachAction::Next);
    }
    action
}

/// Where a `w × h` card goes next to `target`: above, right, below or left — the first
/// that fits on screen without covering `avoid`.
pub fn place(screen: Rect, target: Rect, w: f32, h: f32, avoid: &[Rect]) -> (Rect, Vec2) {
    let gap = 18.0;
    let c = target.center();
    let clamp_x = |x: f32| x.clamp(12.0, (screen.w - 12.0 - w).max(12.0));
    let clamp_y = |y: f32| y.clamp(12.0, (screen.h - 12.0 - h).max(12.0));
    let candidates = [
        (
            Rect::new(clamp_x(c.x - w / 2.0), target.y - gap - h, w, h),
            vec2(c.x, target.y - 4.0),
        ),
        (
            Rect::new(target.right() + gap, clamp_y(c.y - h / 2.0), w, h),
            vec2(target.right() + 4.0, c.y),
        ),
        (
            Rect::new(clamp_x(c.x - w / 2.0), target.bottom() + gap, w, h),
            vec2(c.x, target.bottom() + 4.0),
        ),
        (
            Rect::new(target.x - gap - w, clamp_y(c.y - h / 2.0), w, h),
            vec2(target.x - 4.0, c.y),
        ),
    ];
    let on_screen = |r: &Rect| {
        r.x >= 8.0 && r.y >= 8.0 && r.right() <= screen.w - 8.0 && r.bottom() <= screen.h - 8.0
    };
    candidates
        .iter()
        .find(|(r, _)| on_screen(r) && !avoid.iter().any(|a| a.overlaps(r)))
        .or_else(|| candidates.iter().find(|(r, _)| on_screen(r)))
        .copied()
        .unwrap_or((
            Rect::new(
                clamp_x(target.right() - w - 16.0),
                clamp_y(target.bottom() - h - 100.0),
                w,
                h,
            ),
            target.center(),
        ))
}

fn edge_point(r: Rect, p: Vec2, dir: Vec2) -> Vec2 {
    let tx = if dir.x > 1e-4 {
        (r.right() - p.x) / dir.x
    } else if dir.x < -1e-4 {
        (r.x - p.x) / dir.x
    } else {
        f32::MAX
    };
    let ty = if dir.y > 1e-4 {
        (r.bottom() - p.y) / dir.y
    } else if dir.y < -1e-4 {
        (r.y - p.y) / dir.y
    } else {
        f32::MAX
    };
    p + dir * tx.min(ty)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coach_card_avoids_and_stays_on_screen() {
        let s = Rect::new(0.0, 0.0, 1280.0, 720.0);
        let on_screen =
            |r: &Rect| r.x >= 8.0 && r.y >= 8.0 && r.right() <= 1272.0 && r.bottom() <= 712.0;
        // no room above a target at the top edge: the card goes beside or below, never over it
        let target = Rect::new(600.0, 20.0, 80.0, 40.0);
        let (r, _) = place(s, target, 300.0, 160.0, &[]);
        assert!(on_screen(&r) && !r.overlaps(&target));
        // a HUD block to avoid on the right: the card picks another side
        let avoid = [Rect::new(690.0, 0.0, 590.0, 720.0)];
        let (r2, _) = place(s, target, 300.0, 160.0, &avoid);
        assert!(on_screen(&r2) && !avoid[0].overlaps(&r2) && !r2.overlaps(&target));
    }
}
