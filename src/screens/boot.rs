//! The loading screen: the Shiftella wordmark, a soft glowing progress line and rain.

use macroquad::prelude::*;

use crate::render::colors as pal;
use crate::screens::menu::{draw_wordmark, wordmark_width};
use crate::ui::draw::{Paint, fill_rrect, glow_circle, with_a};
use crate::ui::{RectExt, Ui};

/// One loading frame. `progress` 0..1, `label` already translated.
pub fn draw(ui: &mut Ui, progress: f32, label: &str, t: f32) {
    clear_background(pal::NIGHT_0);
    ui.set_camera();
    let s = ui.screen();
    let th = ui.theme.clone();
    // night gradient + a warm window glow in the middle
    fill_rrect(
        s,
        0.0,
        Paint::vertical(s, Color::from_hex(0x0d0c17), Color::from_hex(0x1a1730)),
    );
    glow_circle(
        vec2(s.w / 2.0, s.h * 0.46),
        s.w.min(s.h) * 0.42,
        with_a(th.indigo, 0.10),
    );
    glow_circle(vec2(s.w / 2.0, s.h * 0.46), 70.0, with_a(th.amber, 0.06));
    // light rain streaks
    for i in 0..90 {
        let fi = i as f32;
        let x = (fi * 97.31).rem_euclid(s.w + 40.0) - 20.0;
        let speed = 260.0 + (fi * 13.7) % 140.0;
        let y = (fi * 53.9 + t * speed).rem_euclid(s.h + 60.0) - 30.0;
        draw_line(
            x,
            y,
            x - 3.0,
            y + 14.0,
            1.0,
            Color::new(0.65, 0.68, 1.0, 0.10 + (fi % 5.0) * 0.02),
        );
    }
    let size = 46.0;
    let w = wordmark_width(ui, size);
    let mark = 48.0;
    let total = mark + 12.0 + w;
    let x0 = (s.w - total) / 2.0;
    crate::render::draw_brand_mark(vec2(x0 + mark / 2.0, s.h * 0.46 - 16.0), mark, 0.8);
    draw_wordmark(ui, x0 + mark + 12.0, s.h * 0.46, size, t);
    // progress line
    let bw = 220.0;
    let bar = Rect::new((s.w - bw) / 2.0, s.h * 0.46 + 40.0, bw, 3.0);
    fill_rrect(bar, 1.5, Color::new(1.0, 1.0, 1.0, 0.08));
    let p = progress.clamp(0.0, 1.0);
    if p > 0.0 {
        let fb = Rect::new(bar.x, bar.y, bar.w * p, bar.h);
        fill_rrect(fb, 1.5, Paint::horizontal(bar, th.violet, th.indigo_300));
        glow_circle(
            vec2(fb.right(), bar.center_y()),
            8.0,
            with_a(th.indigo_300, 0.5),
        );
    }
    ui.fonts
        .draw_centered(label, s.w / 2.0, bar.y + 26.0, &th.small());
}
