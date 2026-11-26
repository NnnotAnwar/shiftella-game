//! Shared painting for the mini-games: the glass card (header, footer, Skip /
//! Serve buttons), the stage backdrop, sparkles and the result chip.

use macroquad::prelude::*;

use super::{Game, Hint, MiniGame, MiniGameKind, PAD, STAGE_SIZE, score::Rating};
use crate::render::draw as rd;
use crate::ui::draw::{self, Paint, fill_convex, fill_rrect, glow_circle, stroke_rrect};
use crate::ui::{Align, Button, Glass, Icon, RectExt, Ui, draw_icon, ease_out};

pub(crate) const STAGE_TOP: Color = Color::from_hex(0x2a1f3a);
pub(crate) const STAGE_BOTTOM: Color = Color::from_hex(0x140e20);

pub(crate) fn with_alpha(c: Color, a: f32) -> Color {
    Color { a: c.a * a, ..c }
}

pub(crate) fn hex(h: u32) -> Color {
    Color::from_hex(h)
}

/// Points on an ellipse arc (radians, y down).
pub(crate) fn ellipse_arc(c: Vec2, rx: f32, ry: f32, a0: f32, a1: f32) -> Vec<Vec2> {
    let n = 24;
    (0..=n)
        .map(|i| {
            let a = a0 + (a1 - a0) * i as f32 / n as f32;
            c + vec2(a.cos() * rx, a.sin() * ry)
        })
        .collect()
}

pub(crate) struct CardOut {
    pub skip: bool,
    pub done: bool,
}

fn kind_icon(k: MiniGameKind) -> Icon {
    match k {
        MiniGameKind::MilkFroth => Icon::Coffee,
        MiniGameKind::LatteArt => Icon::Heart,
        MiniGameKind::PourOver => Icon::Coffee,
        MiniGameKind::TeaSteep => Icon::Cup,
        MiniGameKind::Plate => Icon::ChefHat,
    }
}

/// Card, header (guest chip + wish, recipe title) and footer (prompt, Done, Skip).
pub(crate) fn card(ui: &mut Ui, card: Rect, mg: &MiniGame, hint: &Hint) -> CardOut {
    let t = ui.theme.clone();
    draw::shadow_rrect(
        card.grow(18.0),
        28.0,
        draw::Shadow::new(0.0, 0.0, 60.0, 0.0, Color::new(0.02, 0.01, 0.06, 0.35)),
    );
    ui.glass(card, &Glass::panel());
    let text = &mg.text;

    // --- header ---
    let tile = Rect::new(card.x + PAD, card.y + 10.0, 28.0, 28.0);
    glow_circle(tile.center(), 26.0, with_alpha(t.indigo, 0.35));
    fill_rrect(tile, 8.0, Paint::diagonal(tile, t.indigo, t.violet));
    fill_rrect(
        tile.inset(1.0).with_h(10.0),
        7.0,
        Paint::vertical(
            tile,
            Color::new(1.0, 1.0, 1.0, 0.22),
            Color::new(1.0, 1.0, 1.0, 0.0),
        ),
    );
    draw_icon(kind_icon(mg.kind), tile.center(), 16.0, t.text_on_accent);
    let x = tile.right() + 10.0;

    // right side of the header: regular's touch chip, practised star
    let mut right = card.right() - PAD;
    if mg.practised {
        let label = text.get("minigame.practised");
        let w = ui.chip_width(label, true);
        let pos = vec2(right - w, tile.center_y() - 11.0);
        ui.chip(pos, label, crate::ui::Tone::Warn, Some(Icon::Star));
        right = pos.x - 6.0;
    }
    if let Some(touch) = mg.variation.touch {
        let key = touch.key();
        let label = text.get(&key);
        let w = ui.chip_width(label, true);
        let pos = vec2(right - w, tile.center_y() - 11.0);
        ui.chip(pos, label, crate::ui::Tone::Violet, Some(Icon::Heart));
        right = pos.x - 6.0;
    }
    let avail = (right - x).max(40.0);

    let detail_key = match &mg.game {
        Game::Latte(_) => mg.variation.shape.map(|s| s.key()),
        Game::Tea(_) => mg.variation.tea_kind.map(|s| s.key()),
        Game::Plate(_) => mg.variation.plate.map(|s| s.key()),
        _ => None,
    };
    let kind_key = mg.kind.title_key();
    let kind_name = text.get(&kind_key);
    let title = if mg.title.is_empty() {
        kind_name
    } else {
        mg.title.as_str()
    };
    // top line: guest portrait chip + wish, or the mini-game's name
    match &mg.guest {
        Some((name, wish)) => {
            ui.avatar(vec2(x + 7.0, card.y + 17.0), 14.0, name, None, false);
            let line = format!("{name} · {wish}");
            let st = ui.fonts.fit_width(&line, avail - 18.0, &t.small());
            let s = ui.fonts.ellipsize(&line, avail - 18.0, &st).into_owned();
            ui.fonts.draw(&s, x + 18.0, card.y + 21.0, &st);
        }
        None => {
            let micro = t.micro();
            let up = kind_name.to_uppercase();
            let s = ui.fonts.ellipsize(&up, avail, &micro).into_owned();
            ui.fonts.draw(&s, x, card.y + 20.0, &micro);
        }
    }
    let st = ui.fonts.fit_width(title, avail, &t.h2());
    let s = ui.fonts.ellipsize(title, avail, &st).into_owned();
    let w = ui.fonts.draw(&s, x, card.y + 38.0, &st);
    if let Some(k) = detail_key {
        let ds = t.small();
        let rest = avail - w - 8.0;
        if rest > 30.0 {
            let d = format!("· {}", text.get(&k));
            let ds = ui.fonts.fit_width(&d, rest, &ds);
            let d = ui.fonts.ellipsize(&d, rest, &ds).into_owned();
            ui.fonts.draw(&d, x + w + 6.0, card.y + 38.0, &ds);
        }
    }

    // --- footer: prompt on the left, Done and Skip bottom-right ---
    let foot_y = card.y + super::HEADER_H + STAGE_SIZE.y;
    let foot = Rect::new(
        card.x + PAD,
        foot_y,
        card.w - PAD * 2.0,
        card.bottom() - foot_y,
    );
    let cy = foot.center_y();
    let mut skip = false;
    let mut done = false;
    if mg.finish.is_none() {
        let skip_key = if mg.kind == MiniGameKind::Plate {
            "Esc"
        } else {
            "Tab"
        };
        let b = Button::new(text.get("minigame.skip"))
            .ghost()
            .shortcut(skip_key);
        let w = ui.button_width(&b);
        let r = Rect::new(foot.right() - w + 6.0, cy - 14.0, w, 28.0);
        skip = ui.button(("minigame_skip", mg.kind.id()), r, &b).clicked;
        let mut text_right = r.x - 6.0;
        if hint.done {
            let b = Button::new(text.get("minigame.done"))
                .primary()
                .icon(Icon::Check)
                .shortcut("Enter");
            let w = ui.button_width(&b);
            let r = Rect::new(text_right - w, cy - 14.0, w, 28.0);
            done = ui.button(("minigame_done", mg.kind.id()), r, &b).clicked;
            text_right = r.x - 10.0;
        }
        let mut x = foot.x + 2.0;
        for k in hint.keys {
            let label = if *k == "Space" {
                text.get("minigame.key.space")
            } else {
                k
            };
            let r = ui.keycap(vec2(x, cy), label, false);
            x = r.right() + 5.0;
        }
        if !hint.keys.is_empty() {
            x += 3.0;
        }
        let msg = text.get(hint.key);
        let st = if hint.emphasis {
            let pulse = 0.75 + 0.25 * (mg.t * 5.0).sin();
            t.strong().color(with_alpha(t.amber, pulse))
        } else {
            t.body().color(t.text_dim)
        };
        // the instruction matters: it shrinks, then takes two lines, before it is ever cut
        let avail = (text_right - x).max(10.0);
        let w = ui.fonts.measure(msg, &st).min(avail);
        if hint.emphasis {
            glow_circle(
                vec2(x + w * 0.5, cy),
                w * 0.5 + 18.0,
                with_alpha(t.amber, 0.08),
            );
        }
        ui.fonts.draw_fit(
            msg,
            Rect::new(x, foot.y, avail, foot.h),
            &st,
            Align::Left,
            0.8,
        );
    }
    CardOut { skip, done }
}

/// Dark warm counter with an amber lamp pool and a lavender rim from the window.
pub(crate) fn stage_backdrop(stage: Rect, t: f32) {
    fill_rrect(stage, 10.0, Paint::vertical(stage, STAGE_TOP, STAGE_BOTTOM));
    // lamp pool (slow breathing)
    let breathe = 1.0 + 0.03 * (t * 0.9).sin();
    rd::radial(
        stage.x + stage.w * 0.36,
        stage.y + stage.h * 0.38,
        stage.w * 0.5 * breathe,
        stage.h * 0.62 * breathe,
        Color::new(1.0, 0.72, 0.42, 0.13),
        Color::new(1.0, 0.72, 0.42, 0.0),
    );
    rd::radial(
        stage.right() - 10.0,
        stage.y + 20.0,
        170.0,
        150.0,
        Color::new(0.72, 0.68, 1.0, 0.09),
        Color::new(0.72, 0.68, 1.0, 0.0),
    );
    // faint wood grain on the counter
    for i in 0..9 {
        let y = stage.y + 18.0 + i as f32 * 31.0 + rd::hash01(i * 7 + 3) * 6.0;
        let a = 0.025 + rd::hash01(i * 13 + 1) * 0.02;
        draw::hline(
            stage.x + 6.0,
            stage.right() - 6.0,
            y,
            Color::new(0.0, 0.0, 0.0, a * 2.0),
        );
        draw::hline(
            stage.x + 6.0,
            stage.right() - 6.0,
            y + 1.0,
            Color::new(1.0, 0.9, 0.8, a * 0.5),
        );
    }
}

/// Inner shadow and lit edge on top of the stage content.
pub(crate) fn stage_vignette(stage: Rect) {
    let top = stage.with_h(22.0);
    rd::rect_v(
        top.x,
        top.y,
        top.w,
        top.h,
        Color::new(0.02, 0.0, 0.06, 0.32),
        Color::new(0.02, 0.0, 0.06, 0.0),
    );
    let bot = Rect::new(stage.x, stage.bottom() - 26.0, stage.w, 26.0);
    rd::rect_v(
        bot.x,
        bot.y,
        bot.w,
        bot.h,
        Color::new(0.02, 0.0, 0.06, 0.0),
        Color::new(0.02, 0.0, 0.06, 0.3),
    );
    stroke_rrect(
        stage,
        10.0,
        1.0,
        Paint::vertical(
            stage,
            Color::new(0.0, 0.0, 0.0, 0.35),
            Color::new(1.0, 1.0, 1.0, 0.07),
        ),
    );
}

/// Four-point twinkle. `age` 0..1.
pub(crate) fn sparkle(p: Vec2, size: f32, age: f32, warm: bool) {
    let a = (1.0 - age).clamp(0.0, 1.0);
    let k = size * (0.6 + 0.6 * ease_out(age * 3.0)) * (1.0 - age * 0.4);
    let c = if warm {
        Color::new(1.0, 0.86, 0.55, a)
    } else {
        Color::new(0.86, 0.85, 1.0, a)
    };
    rd::glow(p.x, p.y, k * 2.2, with_alpha(c, 0.35));
    fill_convex(
        &[
            p + vec2(0.0, -k * 1.6),
            p + vec2(k * 0.28, 0.0),
            p + vec2(0.0, k * 1.6),
            p + vec2(-k * 0.28, 0.0),
        ],
        Paint::Solid(c),
    );
    fill_convex(
        &[
            p + vec2(-k * 1.6, 0.0),
            p + vec2(0.0, -k * 0.28),
            p + vec2(k * 1.6, 0.0),
            p + vec2(0.0, k * 0.28),
        ],
        Paint::Solid(c),
    );
}

/// The little "Lovely!" chip with stars that pops in at the end.
pub(crate) fn result_chip(
    ui: &mut Ui,
    stage: Rect,
    quality: f32,
    age: f32,
    text: &super::MiniGameText,
) {
    let t = ui.theme.clone();
    let veil = ease_out(age / 0.4);
    fill_rrect(stage, 10.0, Color::new(0.04, 0.02, 0.1, 0.28 * veil));
    let a = (age / 0.32).clamp(0.0, 1.0);
    // easeOutBack
    let c1 = 1.70158;
    let pop = 1.0 + (c1 + 1.0) * (a - 1.0).powi(3) + c1 * (a - 1.0).powi(2);
    if pop <= 0.01 {
        return;
    }
    let rating = Rating::from_quality(quality);
    let word = text.get(rating.key());
    let st = t.h1();
    let w = (ui.fonts.measure(word, &st) + 44.0).max(150.0);
    let h = 70.0;
    // in the right-hand column, so the finished drink stays visible
    let cx = stage.right() - 12.0 - w / 2.0;
    let cy = stage.center_y();
    let size = vec2(w, h) * (0.85 + 0.15 * pop);
    let r = Rect::new(cx - size.x / 2.0, cy - size.y / 2.0, size.x, size.y);
    draw::push_alpha(a.min(1.0));
    glow_circle(r.center(), w * 0.75, with_alpha(t.amber, 0.12));
    crate::ui::draw_glass(&t, r, &Glass::hud().solid());
    let stars = rating.stars();
    for i in 0..3 {
        let sc = vec2(r.center().x + (i as f32 - 1.0) * 22.0, r.y + 22.0);
        let lit = (i as u32) < stars;
        let delay = (age - 0.18 - i as f32 * 0.09).max(0.0);
        let s = if lit {
            14.0 + 3.0 * (1.0 - ease_out(delay / 0.25))
        } else {
            13.0
        };
        if lit {
            glow_circle(sc, 16.0, with_alpha(t.amber, 0.3));
        }
        draw_icon(
            Icon::Star,
            sc,
            s,
            if lit {
                t.amber
            } else {
                with_alpha(t.text_disabled, 0.6)
            },
        );
    }
    ui.fonts.draw_in(
        word,
        Rect::new(r.x, r.y + 34.0, r.w, 28.0),
        &st,
        Align::Center,
    );
    draw::pop_alpha();
}

// --- shared props -------------------------------------------------------------

/// Soft contact shadow under an object standing on the counter.
pub(crate) fn contact_shadow(c: Vec2, rx: f32, ry: f32, strength: f32) {
    rd::soft_shadow(c.x, c.y, rx, ry, strength);
}
