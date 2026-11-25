//! The "Shiftella" wordmark: Inter Bold with an indigo → violet gradient and a soft glow.

use macroquad::prelude::*;

use crate::ui::draw::{self, glow_circle, mix, with_a};
use crate::ui::{TextStyle, Ui, Weight};

const WORD: &str = "Shiftella";

fn style(size: f32) -> TextStyle {
    TextStyle::new(size, Weight::Bold, WHITE).tracking(-size * 0.02)
}

pub fn wordmark_width(ui: &Ui, size: f32) -> f32 {
    ui.fonts.measure(WORD, &style(size))
}

/// Draws the wordmark with its baseline at `y`; returns the width.
/// The gradient runs left → right (indigo → violet) and top → bottom (light → deep),
/// painted in horizontal bands through the scissor.
pub fn draw_wordmark(ui: &mut Ui, x: f32, y: f32, size: f32, t: f32) -> f32 {
    let st = style(size);
    let w = ui.fonts.measure(WORD, &st);
    let cap = ui.fonts.cap_height(&st);
    let th = ui.theme.clone();
    // glow: a breathing violet haze behind the letters
    let breathe = 0.85 + 0.15 * (t * 0.8).sin();
    glow_circle(
        vec2(x + w * 0.5, y - cap * 0.5),
        w * 0.62,
        with_a(th.violet, 0.16 * breathe),
    );
    glow_circle(
        vec2(x + w * 0.3, y - cap * 0.55),
        w * 0.35,
        with_a(th.indigo, 0.18 * breathe),
    );
    // soft shadow + glow copies
    let mut sh = st;
    sh.color = Color::new(0.02, 0.01, 0.08, 0.55);
    ui.fonts.draw(WORD, x, y + size * 0.05, &sh);
    for (dx, dy) in [(-1.5, 0.0), (1.5, 0.0), (0.0, -1.5), (0.0, 1.5)] {
        let mut g = st;
        g.color = with_a(th.indigo_400, 0.10 * breathe);
        ui.fonts.draw(WORD, x + dx, y + dy, &g);
    }
    // gradient bands
    let top = y - cap - size * 0.08;
    let bottom = y + size * 0.24;
    let bands = 10;
    let bh = (bottom - top) / bands as f32;
    let left_top = Color::from_hex(0xC7D2FE);
    let left_bot = th.indigo;
    let right_top = Color::from_hex(0xDDD6FE);
    let right_bot = th.violet;
    let prev = ui.clip_rect();
    for b in 0..bands {
        let fy = (b as f32 + 0.5) / bands as f32;
        let band = Rect::new(x - 8.0, top + b as f32 * bh, w + 16.0, bh + 0.5);
        ui.push_clip(band);
        // per letter left → right
        let mut cx = x;
        let mut buf = [0u8; 4];
        for c in WORD.chars() {
            let adv = ui.fonts.advance(c, &st);
            let fx = ((cx + adv * 0.5 - x) / w).clamp(0.0, 1.0);
            let col = mix(
                mix(left_top, right_top, fx),
                mix(left_bot, right_bot, fx),
                fy,
            );
            let mut s = st;
            s.color = col;
            s.tracking = 0.0;
            ui.fonts.draw(c.encode_utf8(&mut buf), cx, y, &s);
            cx += adv + st.tracking;
        }
        ui.pop_clip();
    }
    if prev.is_none() {
        draw::set_scissor(None);
    }
    // a thin highlight sweeping across now and then
    let sweep = ((t * 0.12) % 1.0) * 3.0 - 1.0;
    if (0.0..=1.0).contains(&sweep) {
        let sx = x + w * sweep;
        ui.push_clip(Rect::new(sx - 10.0, top, 20.0, bottom - top));
        let mut s = st;
        s.color = with_a(WHITE, 0.35 * (1.0 - (sweep * 2.0 - 1.0).abs()));
        ui.fonts.draw(WORD, x, y, &s);
        ui.pop_clip();
    }
    w
}
