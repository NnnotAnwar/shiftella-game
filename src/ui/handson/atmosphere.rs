//! The atmosphere (coziness) meter, design §11.3: a small HUD gauge — a candle whose flame
//! grows with the level, the level name and five steps. Hover shows the component bars
//! and one tip for the weakest part. A level-up makes the flame flare once.

use macroquad::prelude::*;

use crate::ui::draw::{
    self, Paint, fill_circle, fill_convex, fill_rrect, mix, stroke_rrect, with_a,
};
use crate::ui::glass::draw_glass;
use crate::ui::layout::RectExt;
use crate::ui::{Align, Glass, Icon, Response, Ui, draw_icon};

#[derive(Clone, Debug)]
pub struct AtmosphereView {
    /// 0–100.
    pub value: f32,
    /// 0–4: Bare, Plain, Pleasant, Cosy, Like home.
    pub level: usize,
    pub level_name: String,
    /// Popover title ("Atmosphere").
    pub title: String,
    /// (label, 0..1) per component: light, beauty, comfort, clean, music, theme, touches.
    pub components: Vec<(String, f32)>,
    /// One tip for the weakest component.
    pub tip: Option<String>,
}

/// Natural size of the HUD pill.
pub const ATMO_SIZE: Vec2 = Vec2::new(176.0, 36.0);

/// The meter in `r` (use [`ATMO_SIZE`]). Returns the hover response (the integration
/// may highlight dark seats / dirty tables in the world while it is hovered).
pub fn atmosphere_meter(
    ui: &mut Ui,
    key: impl std::hash::Hash,
    r: Rect,
    v: &AtmosphereView,
) -> Response {
    let t = ui.theme.clone();
    let id = ui.id(key);
    // a full capsule (radius = h/2) leaves zero-length edges that spike the shadow mesh
    ui.glass(r, &Glass::hud().radius(r.h / 2.0 - 2.0));
    let resp = ui.interact(id, r);
    // level-up flare
    let last = ui.anim(id.with("lvl")).n as usize;
    let flare_id = id.with("flare");
    if last != v.level + 1 {
        if last != 0 && v.level + 1 > last {
            ui.anim(flare_id).v = 1.0;
        }
        ui.anim(id.with("lvl")).n = (v.level + 1) as f32;
    }
    let flare = ui.ease_value(flare_id, 0.0, 0.6);
    let lv = ui.ease_value(id.with("grow"), v.level as f32, 0.25);

    let fc = vec2(r.x + 19.0, r.center_y() + 1.0);
    candle(fc, lv, flare, ui.time());
    let tx = r.x + 36.0;
    ui.fonts.draw_in(
        &v.level_name,
        Rect::new(tx, r.y + 3.0, r.right() - tx - 10.0, 17.0),
        &t.strong().size(12.5),
        Align::Left,
    );
    // five steps
    let pip_w = (r.right() - tx - 14.0 - 4.0 * 3.0) / 5.0;
    for i in 0..5 {
        let pr = Rect::new(tx + i as f32 * (pip_w + 3.0), r.bottom() - 11.0, pip_w, 4.0);
        let fill = ((v.value / 20.0) - i as f32).clamp(0.0, 1.0);
        fill_rrect(pr, 2.0, Color::new(1.0, 1.0, 1.0, 0.09));
        if fill > 0.0 {
            let f = pr.with_w((pr.w * fill).max(pr.h));
            let c = mix(t.amber_deep, t.amber, i as f32 / 4.0);
            fill_rrect(
                f,
                2.0,
                Paint::horizontal(f, mix(c, BLACK, 0.1), mix(c, WHITE, 0.15)),
            );
        }
    }

    // hover popover
    let k = ui.ease_value(id.with("pop"), if resp.hovered { 1.0 } else { 0.0 }, 0.06);
    if k > 0.02 {
        let s = ui.screen();
        let w = 236.0;
        let tip_h = v.tip.as_ref().map_or(0.0, |tip| {
            ui.fonts.wrapped_height(tip, w - 46.0, &t.small(), Some(3)) + 14.0
        });
        let h = 38.0 + v.components.len() as f32 * 20.0 + tip_h + 8.0;
        let above = r.y - h - 8.0 >= 8.0;
        let py = if above {
            r.y - h - 8.0
        } else {
            r.bottom() + 8.0
        };
        let px = r.x.clamp(8.0, (s.w - w - 8.0).max(8.0));
        let pr = Rect::new(px, py + (1.0 - k) * if above { 6.0 } else { -6.0 }, w, h);
        draw::push_alpha(k);
        draw_glass(&t, pr, &Glass::tooltip().radius(12.0));
        let inner = pr.inset(12.0);
        ui.fonts.draw_in(
            &v.title.to_uppercase(),
            inner.with_h(14.0),
            &t.micro(),
            Align::Left,
        );
        let val = format!("{:.0}", v.value.clamp(0.0, 100.0));
        ui.fonts.draw_in(
            &val,
            inner.with_h(14.0),
            &t.number().color(t.amber),
            Align::Right,
        );
        let mut y = inner.y + 24.0;
        let weakest = v
            .components
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.1.total_cmp(&b.1.1))
            .map(|(i, _)| i);
        for (i, (label, f)) in v.components.iter().enumerate() {
            let weak = Some(i) == weakest;
            let ls = t.small().color(if weak { t.amber } else { t.text_dim });
            ui.fonts
                .draw_in(label, Rect::new(inner.x, y, 84.0, 16.0), &ls, Align::Left);
            let bar = Rect::new(inner.x + 90.0, y + 6.0, inner.w - 90.0, 5.0);
            ui.progress(bar, *f, if weak { t.amber_deep } else { t.indigo_400 });
            y += 20.0;
        }
        if let Some(tip) = &v.tip {
            let tr = Rect::new(inner.x, y + 4.0, inner.w, tip_h - 6.0);
            fill_rrect(tr, 8.0, with_a(t.amber, 0.09));
            stroke_rrect(tr, 8.0, 1.0, with_a(t.amber, 0.18));
            draw_icon(Icon::Info, vec2(tr.x + 13.0, tr.y + 13.0), 13.0, t.amber);
            ui.fonts.draw_wrapped(
                tip,
                Rect::new(tr.x + 26.0, tr.y + 4.0, tr.w - 32.0, tr.h - 6.0),
                &t.small().color(t.text),
                Align::Left,
                Some(3),
            );
        }
        draw::pop_alpha();
    }
    resp
}

/// A small candle at `c`: the flame height follows `level` (0–4, fractional while it eases),
/// `flare` (1 → 0) brightens it after a level-up.
pub fn candle(c: Vec2, level: f32, flare: f32, time: f64) {
    let body = Rect::new(c.x - 4.0, c.y + 1.0, 8.0, 11.0);
    let s = 0.55 + level * 0.14 + flare * 0.25;
    let flick = ((time * 9.0).sin() * 0.5 + (time * 13.7).sin() * 0.3) as f32;
    let tip = vec2(c.x + flick * 0.6, c.y - 3.0 - 11.0 * s);
    let base = vec2(c.x, c.y - 1.0);
    draw::glow_circle(
        base + vec2(0.0, -5.0 * s),
        12.0 + 8.0 * level + 10.0 * flare,
        Color::new(1.0, 0.72, 0.35, 0.22 + 0.05 * level + 0.25 * flare),
    );
    fill_rrect(
        body,
        2.0,
        Paint::vertical(body, Color::from_hex(0xF6EEDC), Color::from_hex(0xCDBFA4)),
    );
    // flame: a teardrop from the wick up to the tip
    let w = 3.6 * s + 1.2;
    let mut pts = Vec::new();
    for i in 0..=16 {
        let a = std::f32::consts::PI * i as f32 / 16.0;
        pts.push(base + vec2(a.cos() * w, a.sin() * w * 0.9 - w * 0.3));
    }
    pts.push(tip);
    pts.rotate_left(0);
    fill_convex(
        &pts,
        Paint::Linear {
            from: tip,
            to: base,
            c0: Color::from_hex(0xFFE7A6),
            c1: Color::from_hex(0xF99C4E),
        },
    );
    fill_circle(
        base + vec2(0.0, -w * 0.5),
        w * 0.45,
        Color::new(1.0, 0.98, 0.9, 0.85),
    );
}
