//! The one-screen guide card (first time a game type or a new step plays; "?" reopens it),
//! the latte pattern chooser and the practised flourish ring.

use macroquad::prelude::*;

use super::paint::{hex, with_alpha};
use super::{
    Chooser, FLOURISH_RING, Flourish, GuideCard, LatteMilk, MiniGameText, STAR_MARKS, chooser_rect,
    flourish_ok, flourish_radius,
};
use crate::ui::draw::{self, Paint, fill_circle, fill_rrect, glow_circle, stroke_circle};
use crate::ui::{Align, Button, Glass, Icon, Ui, draw_glass, draw_icon, ease_out};

/// `minigame.guide.<kind>.<part>`
fn kind_key(kind: &str, part: &str) -> String {
    format!("minigame.guide.{kind}.{part}")
}

/// `minigame.guide.step.<topic>`
pub(crate) fn step_key(topic: &str) -> String {
    format!("minigame.guide.step.{topic}")
}

/// Draws the guide over the stage; returns true when "Got it" was clicked.
pub(crate) fn draw_guide(ui: &mut Ui, stage: Rect, g: &GuideCard, text: &MiniGameText) -> bool {
    let th = ui.theme.clone();
    let a = ease_out(g.age / 0.25);
    fill_rrect(stage, 10.0, Color::new(0.04, 0.02, 0.1, 0.55 * a));
    let r = Rect::new(
        stage.x + 10.0,
        stage.y + 8.0 + (1.0 - a) * 8.0,
        stage.w - 20.0,
        stage.h - 16.0,
    );
    draw::push_alpha(a);
    draw_glass(&th, r, &Glass::hud().solid());
    let x = r.x + 14.0;
    let w = r.w - 28.0;
    let mut y = r.y + 12.0;
    let micro = th.micro();
    let head = if g.kind.is_some() {
        text.get("minigame.guide.how")
    } else {
        text.get("minigame.guide.new")
    };
    ui.fonts.draw(&head.to_uppercase(), x, y + 9.0, &micro);
    let kind = g.kind.map(|k| k.id());
    if let Some(k) = kind {
        let tk = format!("minigame.{k}.title");
        ui.fonts
            .draw(text.get(&tk), x, y + 30.0, &th.h2().color(th.text));
        y += 40.0;
    } else {
        y += 18.0;
    }
    let label = th.small_medium().color(th.indigo_300);
    let mut rows: Vec<(Icon, Option<&str>, String)> = Vec::new();
    if let Some(k) = kind {
        rows.push((Icon::Info, None, text.get(&kind_key(k, "what")).to_string()));
        rows.push((
            Icon::User,
            Some(text.get("minigame.guide.mouse")),
            text.get(&kind_key(k, "mouse")).to_string(),
        ));
        rows.push((
            Icon::Settings,
            Some(text.get("minigame.guide.keys")),
            text.get(&kind_key(k, "keys")).to_string(),
        ));
    }
    let step_label = if g.kind.is_some() {
        text.get("minigame.guide.step")
    } else {
        text.get("minigame.guide.new_step")
    };
    for st in &g.steps {
        rows.push((
            Icon::Sparkle,
            Some(step_label),
            text.get(&step_key(st)).to_string(),
        ));
    }
    // the card is one screen: the text shrinks a little before anything is cut
    let avail = r.bottom() - 48.0 - y;
    let mut laid: Vec<(Icon, Option<&str>, f32, Vec<String>)> = Vec::new();
    let mut body = th.small().size(12.5).line_height(1.35).color(th.text_dim);
    for size in [12.5, 11.5, 10.5] {
        body = th.small().size(size).line_height(1.3).color(th.text_dim);
        laid.clear();
        let mut h = 0.0;
        for (icon, lab, msg) in &rows {
            let lw = lab.map_or(0.0, |l| ui.fonts.measure(l, &label) + 6.0);
            let first_w = w - 20.0 - lw;
            let mut lines = ui.fonts.wrap(msg, first_w, &body);
            if lines.len() > 1 {
                // the rest wraps under the label too
                let rest = lines[1..].join(" ");
                let mut more = ui.fonts.wrap(&rest, w - 20.0, &body);
                lines.truncate(1);
                lines.append(&mut more);
            }
            h += lines.len() as f32 * body.line_h() + 3.0;
            laid.push((*icon, *lab, lw, lines));
        }
        if h <= avail {
            break;
        }
    }
    let lh = body.line_h();
    for (icon, lab, lw, lines) in &laid {
        draw_icon(*icon, vec2(x + 7.0, y + lh * 0.5), 12.0, th.indigo_300);
        if let Some(l) = lab {
            ui.fonts.draw(l, x + 20.0, y + lh * 0.78, &label);
        }
        for (i, line) in lines.iter().enumerate() {
            let lx = if i == 0 { x + 20.0 + lw } else { x + 20.0 };
            if y + lh <= r.bottom() - 44.0 {
                ui.fonts.draw(line, lx, y + lh * 0.78, &body);
            }
            y += lh;
        }
        y += 3.0;
    }
    // how stars are earned: ★ always · ★★ from 70 % · ★★★ from 90 %
    let stars_msg = match kind {
        Some(k) => text.get(&kind_key(k, "stars")).to_string(),
        None => text.get("minigame.guide.stars_any").to_string(),
    };
    let sy = r.bottom() - 40.0;
    let mut sx = x;
    for n in 1..=3u32 {
        for i in 0..n {
            draw_icon(
                Icon::Star,
                vec2(sx + 5.0 + i as f32 * 10.0, sy + 6.0),
                10.0,
                th.amber,
            );
        }
        sx += n as f32 * 10.0 + 4.0;
        let pct = match n {
            1 => "—".to_string(),
            2 => format!("{}%", (STAR_MARKS[0] * 100.0).round()),
            _ => format!("{}%", (STAR_MARKS[1] * 100.0).round()),
        };
        let pw = ui.fonts.draw(&pct, sx, sy + 10.0, &th.small().tabular());
        sx += pw + 12.0;
    }
    let st = th.small().size(11.5).color(th.text_dim);
    let b = Button::new(text.get("minigame.guide.got_it"))
        .primary()
        .icon(Icon::Check)
        .shortcut("Enter");
    let bw = ui.button_width(&b);
    let rest = (r.right() - 14.0 - bw - 10.0) - sx;
    let lines = ui.fonts.wrap(&stars_msg, rest.max(60.0), &st);
    ui.fonts.draw_lines(
        &lines,
        Rect::new(sx, sy - 4.0, rest.max(60.0), 30.0),
        &st,
        Align::Left,
        Some(2),
        usize::MAX,
    );
    let br = Rect::new(r.right() - 14.0 - bw, r.bottom() - 40.0, bw, 28.0);
    let clicked = ui.button("minigame_guide_ok", br, &b).clicked;
    draw::pop_alpha();
    clicked
}

/// LatteArt: three pattern chips; the selected one glows.
pub(crate) fn draw_chooser(
    ui: &mut Ui,
    stage: Rect,
    c: &Chooser,
    milk: Option<LatteMilk>,
    text: &MiniGameText,
    t: f32,
) {
    let th = ui.theme.clone();
    let o = stage.point();
    ui.fonts.draw_in(
        text.get("minigame.latte_art.choose"),
        Rect::new(stage.x, stage.y + 14.0, stage.w, 22.0),
        &th.strong(),
        Align::Center,
    );
    let milk_c = super::latte::milk_color(milk.unwrap_or(LatteMilk::Dairy));
    let n = c.options.len();
    for (i, shape) in c.options.iter().enumerate() {
        let r = chooser_rect(i, n).offset(o);
        let sel = i == c.sel;
        let hover = ui.mouse().distance(r.center()) < 60.0;
        if sel {
            glow_circle(r.center(), 76.0, with_alpha(th.amber, 0.12));
        }
        draw_glass(&th, r, &Glass::card());
        if sel || hover {
            draw::stroke_rrect(
                r,
                10.0,
                1.5,
                with_alpha(th.amber, if sel { 0.9 } else { 0.4 }),
            );
        }
        let cc = vec2(r.center().x, r.y + 62.0);
        let cr = 42.0;
        fill_circle(
            cc,
            cr + 6.0,
            Paint::diagonal(
                Rect::new(cc.x - cr, cc.y - cr, cr * 2.0, cr * 2.0),
                hex(0xfcf9fd),
                hex(0xc4b9d6),
            ),
        );
        fill_circle(
            cc,
            cr,
            Paint::diagonal(
                Rect::new(cc.x - cr, cc.y - cr, cr * 2.0, cr * 2.0),
                hex(0xb27a45),
                hex(0x5a321e),
            ),
        );
        let pulse = if sel { 0.15 * (t * 4.0).sin() } else { 0.0 };
        for s in shape.strokes() {
            let pts: Vec<Vec2> = s.iter().map(|p| cc + *p * (cr - 6.0)).collect();
            draw::stroke_path(&pts, false, 3.0 + pulse, Paint::Solid(milk_c));
        }
        ui.fonts.draw_in(
            text.get(&shape.key()),
            Rect::new(r.x + 4.0, r.bottom() - 34.0, r.w - 8.0, 20.0),
            &th.small_medium(),
            Align::Center,
        );
        let n_key = format!("{}", i + 1);
        ui.fonts.draw_in(
            &n_key,
            Rect::new(r.x + 4.0, r.bottom() - 18.0, r.w - 8.0, 14.0),
            &th.micro(),
            Align::Center,
        );
    }
}

/// The practised flourish: a golden ring shrinks towards the cup; tap when it meets the rim.
pub(crate) fn draw_flourish(
    ui: &mut Ui,
    c: Vec2,
    f: &Flourish,
    relaxed: bool,
    text: &MiniGameText,
) {
    let th = ui.theme.clone();
    let rim = FLOURISH_RING;
    stroke_circle(c, rim, 2.0, with_alpha(th.text, 0.5));
    let t_draw = if f.hit.is_some() { f.at } else { f.t };
    let rr = flourish_radius(t_draw);
    let ok = flourish_ok(t_draw, relaxed);
    let col = if ok { th.amber } else { th.indigo_300 };
    glow_circle(c, rr + 12.0, with_alpha(col, 0.12));
    stroke_circle(c, rr, 2.6, with_alpha(col, 0.9));
    if let Some(hit) = f.hit {
        let k = ((f.t - f.at) / 0.35).clamp(0.0, 1.0);
        let key = if hit {
            "minigame.flourish.hit"
        } else {
            "minigame.flourish.miss"
        };
        ui.fonts.draw_in(
            text.get(key),
            Rect::new(c.x - 90.0, c.y - rim - 34.0 - k * 6.0, 180.0, 20.0),
            &th.strong().color(with_alpha(
                if hit { th.amber } else { th.text_dim },
                1.0 - k * 0.5,
            )),
            Align::Center,
        );
    }
}
