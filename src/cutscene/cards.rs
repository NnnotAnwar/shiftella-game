//! Cutscene overlays drawn in UI space: grandma's handwritten letter / note cards and the
//! Shiftella phone with the first @shiftella_bot message.

use macroquad::prelude::*;

use crate::ui::draw::{
    self, Paint, Shadow, fill_circle, fill_rrect, glow_circle, hline, mix, shadow_rrect,
    stroke_rrect, with_a,
};
use crate::ui::{Align, Glass, Icon, PhoneFrame, RectExt, TextStyle, Ui, Weight, draw_glass};

const PAPER_TOP: Color = Color::new(0.976, 0.937, 0.871, 1.0);
const PAPER_BOTTOM: Color = Color::new(0.925, 0.871, 0.792, 1.0);
const INK: Color = Color::new(0.17, 0.14, 0.38, 1.0);

/// Draws text glyph by glyph with a gentle hand-written wobble (baseline, slant, size).
/// Returns the width. `visible` limits the number of drawn characters.
pub fn draw_handwritten(
    ui: &Ui,
    text: &str,
    x: f32,
    y: f32,
    style: &TextStyle,
    seed: u32,
    visible: usize,
) -> f32 {
    let font = ui.fonts.face(style.weight);
    let mut cx = x;
    let mut buf = [0u8; 4];
    for (i, c) in text.chars().enumerate() {
        let adv = ui.fonts.advance(c, style);
        if i >= visible {
            break;
        }
        if c != ' ' {
            let h = hash(seed.wrapping_add(i as u32 * 7919));
            let wob = (h & 0xff) as f32 / 255.0 - 0.5;
            let wob2 = ((h >> 8) & 0xff) as f32 / 255.0 - 0.5;
            let size = style.size * (1.0 + wob2 * 0.08);
            let fs = (size / draw::px()).round().max(1.0);
            let k = size / fs;
            draw_text_ex(
                c.encode_utf8(&mut buf),
                cx,
                y + wob * 1.6 + (i as f32 * 0.37).sin() * 0.7,
                TextParams {
                    font: Some(font),
                    font_size: fs as u16,
                    font_scale: k,
                    rotation: -0.07 + wob * 0.05,
                    color: draw::faded(style.color),
                    ..Default::default()
                },
            );
        }
        cx += adv * 0.98 + style.tracking;
    }
    cx - x
}

fn hash(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^ (x >> 16)
}

/// A paper card in a lit glass frame. `appear` 0..1 slides it in.
fn paper_card(ui: &mut Ui, r: Rect, appear: f32, tilt_seed: u32) -> Rect {
    let t = ui.theme.clone();
    let k = crate::ui::ease_out(appear.clamp(0.0, 1.0));
    let r = r.translate(vec2(0.0, (1.0 - k) * 26.0));
    draw::push_alpha(k);
    ui.region(r);
    shadow_rrect(
        r,
        16.0,
        Shadow::new(0.0, 22.0, 50.0, 0.0, Color::new(0.02, 0.01, 0.06, 0.55)),
    );
    glow_circle(r.center(), r.w * 0.7, with_a(t.amber, 0.10));
    draw_glass(&t, r, &Glass::card().radius(16.0));
    let paper = r.inset(10.0);
    fill_rrect(paper, 9.0, Paint::vertical(paper, PAPER_TOP, PAPER_BOTTOM));
    // warm lamp light from the top-left, paper grain-ish fibres
    fill_rrect(
        paper,
        9.0,
        Paint::diagonal(
            paper,
            with_a(t.amber, 0.10),
            Color::new(0.4, 0.3, 0.6, 0.06),
        ),
    );
    for i in 0..18u32 {
        let h = hash(tilt_seed + i * 31);
        let fx = paper.x + (h % 1000) as f32 / 1000.0 * paper.w;
        let fy = paper.y + ((h >> 10) % 1000) as f32 / 1000.0 * paper.h;
        hline(
            fx,
            fx + 6.0 + (h % 9) as f32,
            fy,
            Color::new(0.55, 0.45, 0.35, 0.10),
        );
    }
    stroke_rrect(paper, 9.0, 1.0, Color::new(0.45, 0.35, 0.3, 0.18));
    // washi tape strip
    let tape = Rect::new(paper.center().x - 34.0, paper.y - 9.0, 68.0, 18.0);
    fill_rrect(tape, 2.0, with_a(t.violet_400, 0.55));
    stroke_rrect(tape, 2.0, 1.0, Color::new(1.0, 1.0, 1.0, 0.18));
    draw::pop_alpha();
    paper
}

/// Grandma's letter: lines so far + the current one typing (`visible` chars of the last).
pub fn letter_card(
    ui: &mut Ui,
    header: &str,
    lines: &[String],
    visible_last: usize,
    appear: f32,
    t: f32,
) -> Rect {
    let s = ui.screen();
    let w = (s.w - 48.0).min(560.0);
    let body = TextStyle::new(17.0, Weight::Medium, INK).line_height(1.62);
    let inner_w = w - 20.0 - 64.0;
    let mut wrapped: Vec<Vec<String>> = lines
        .iter()
        .map(|l| ui.fonts.wrap(l, inner_w, &body))
        .collect();
    // a long letter scrolls: the oldest paragraphs leave the card
    let max_rows = ((s.h - 150.0 - 96.0) / body.line_h()).floor().max(3.0) as usize;
    let rows = |w: &[Vec<String>]| w.iter().map(|p| p.len()).sum::<usize>() + w.len();
    while wrapped.len() > 1 && rows(&wrapped) > max_rows {
        wrapped.remove(0);
    }
    let n_rows = rows(&wrapped);
    let h = (96.0 + n_rows.max(4) as f32 * body.line_h()).min(s.h - 150.0);
    let r = Rect::new((s.w - w) / 2.0, (s.h - h) / 2.0 - 34.0, w, h);
    let paper = paper_card(ui, r, appear, 333);
    let k = crate::ui::ease_out(appear.clamp(0.0, 1.0));
    draw::push_alpha(k);
    // header + ruled lines
    let hs = TextStyle::new(12.0, Weight::SemiBold, with_a(INK, 0.6)).tracking(0.4);
    ui.fonts.draw_in(
        header,
        Rect::new(paper.x + 32.0, paper.y + 14.0, paper.w - 64.0, 20.0),
        &hs,
        Align::Left,
    );
    let x0 = paper.x + 32.0;
    let mut y = paper.y + 44.0;
    let rule = Color::new(0.39, 0.4, 0.95, 0.13);
    let mut rows_y = Vec::new();
    while y + body.line_h() < paper.bottom() - 14.0 {
        rows_y.push(y + body.size * 1.1);
        hline(
            x0 - 6.0,
            paper.right() - 26.0,
            y + body.size * 1.1 + 7.0,
            rule,
        );
        y += body.line_h();
    }
    // writing
    let mut row = 0usize;
    for (li, wl) in wrapped.iter().enumerate() {
        let last = li + 1 == wrapped.len();
        let mut left = if last { visible_last } else { usize::MAX };
        for (wi, text) in wl.iter().enumerate() {
            let Some(by) = rows_y.get(row).copied() else {
                break;
            };
            let n = text.chars().count();
            let shown = left.min(n);
            draw_handwritten(ui, text, x0, by, &body, (li * 97 + wi * 13) as u32, shown);
            left = left.saturating_sub(n + 1);
            row += 1;
            if left == 0 && last {
                break;
            }
        }
        row += 1; // paragraph gap
    }
    // grandma's brass key, tucked under the letter once she mentions it
    if lines.len() >= 2 {
        draw_key(vec2(paper.x + 44.0, paper.bottom() - 26.0), t);
    }
    // a pen nib blink at the end while writing
    let pulse = 0.5 + 0.5 * (t * 4.0).sin();
    fill_circle(
        vec2(paper.right() - 30.0, paper.bottom() - 22.0),
        2.0 + pulse,
        with_a(ui.theme.violet, 0.35 + 0.3 * pulse),
    );
    draw::pop_alpha();
    r
}

/// A small brass key (the café's) lying at an angle.
fn draw_key(p: Vec2, t: f32) {
    let brass = Color::new(0.85, 0.64, 0.3, 1.0);
    let dark = Color::new(0.55, 0.38, 0.16, 1.0);
    let dir = vec2(0.94, -0.34);
    let n = vec2(-dir.y, dir.x);
    crate::ui::draw::fill_circle(p + vec2(2.0, 3.0), 9.0, Color::new(0.2, 0.12, 0.05, 0.18));
    crate::ui::draw::fill_circle(p, 9.0, brass);
    crate::ui::draw::fill_circle(p, 4.0, PAPER_TOP);
    let a = p + dir * 8.0;
    let b = p + dir * 38.0;
    crate::ui::draw::line(a, b, 3.4, brass);
    crate::ui::draw::line(b - dir * 3.0, b - dir * 3.0 + n * 7.0, 3.0, brass);
    crate::ui::draw::line(b - dir * 10.0, b - dir * 10.0 + n * 5.0, 3.0, brass);
    crate::ui::draw::line(a, b, 1.0, dark);
    let glint = 0.5 + 0.5 * (t * 2.0).sin();
    crate::ui::draw::fill_circle(p + vec2(-4.0, -4.0), 1.6, with_a(WHITE, 0.5 * glint));
}

/// One of Lida's notes (`note.<id>.text`), with where it was found and its number.
pub fn note_card(
    ui: &mut Ui,
    where_found: &str,
    number: &str,
    text: &str,
    visible: usize,
    appear: f32,
) -> Rect {
    let s = ui.screen();
    let w = (s.w - 48.0).min(440.0);
    let body = TextStyle::new(17.0, Weight::Medium, INK).line_height(1.6);
    let lines = ui.fonts.wrap(text, w - 84.0, &body);
    let h = (110.0 + lines.len() as f32 * body.line_h()).min(s.h - 150.0);
    let r = Rect::new((s.w - w) / 2.0, (s.h - h) / 2.0 - 30.0, w, h);
    let paper = paper_card(ui, r, appear, 7);
    let k = crate::ui::ease_out(appear.clamp(0.0, 1.0));
    draw::push_alpha(k);
    let hs = TextStyle::new(11.0, Weight::Bold, with_a(INK, 0.55)).tracking(0.6);
    ui.fonts.draw_in(
        &where_found.to_uppercase(),
        Rect::new(paper.x + 30.0, paper.y + 16.0, paper.w - 110.0, 18.0),
        &hs,
        Align::Left,
    );
    ui.fonts.draw_in(
        number,
        Rect::new(paper.right() - 90.0, paper.y + 16.0, 60.0, 18.0),
        &hs,
        Align::Right,
    );
    let mut left = visible;
    let mut y = paper.y + 52.0 + body.size;
    for (i, l) in lines.iter().enumerate() {
        let n = l.chars().count();
        draw_handwritten(
            ui,
            l,
            paper.x + 30.0,
            y,
            &body,
            i as u32 * 19 + 5,
            left.min(n),
        );
        left = left.saturating_sub(n + 1);
        y += body.line_h();
    }
    // signature heart
    let c = vec2(paper.right() - 40.0, paper.bottom() - 26.0);
    crate::ui::draw_icon(Icon::Heart, c, 14.0, with_a(ui.theme.violet, 0.7));
    draw::pop_alpha();
    r
}

/// The Shiftella app sliding up with a bot message. Returns the phone rect.
pub fn phone_bot(
    ui: &mut Ui,
    labels: &PhoneLabels,
    message: &str,
    visible: usize,
    appear: f32,
    t: f32,
) -> Rect {
    let s = ui.screen();
    let k = crate::ui::ease_out(appear.clamp(0.0, 1.0));
    let h = (s.h - 120.0).min(520.0);
    let size = crate::ui::Device::Phone.size_for_height(h);
    let r = Rect::new(
        s.w - size.x - 64.0,
        s.h - h - 40.0 + (1.0 - k) * (h + 60.0),
        size.x,
        size.y,
    );
    let tabs = [
        (Icon::Calendar, labels.tab_schedule),
        (Icon::Bell, labels.tab_bot),
        (Icon::Megaphone, labels.tab_board),
    ];
    let out = ui.phone_frame(
        "cutscene_phone",
        r,
        &PhoneFrame::new(labels.time, &tabs)
            .title(labels.title)
            .subtitle(labels.subtitle)
            .selected(1)
            .badge(1),
    );
    let c = out.content.inset_xy(10.0, 8.0);
    let th = ui.theme.clone();
    // day separator
    let ds = th.micro();
    ui.fonts.draw_in(
        &labels.today.to_uppercase(),
        Rect::new(c.x, c.y, c.w, 18.0),
        &ds,
        Align::Center,
    );
    // bot avatar + bubble
    let av = vec2(c.x + 14.0, c.y + 46.0);
    fill_circle(
        av,
        13.0,
        Paint::diagonal(
            Rect::new(av.x - 13.0, av.y - 13.0, 26.0, 26.0),
            th.indigo,
            th.violet,
        ),
    );
    crate::ui::draw_icon(Icon::Sparkle, av, 14.0, WHITE);
    let handle = th.strong().size(12.0).color(th.indigo_300);
    ui.fonts
        .draw(labels.bot_handle, av.x + 22.0, c.y + 38.0, &handle);
    let body = th.body().line_height(1.45);
    let bw = c.w - 34.0;
    let lines = ui.fonts.wrap(message, bw - 22.0, &body);
    let bh = lines.len() as f32 * body.line_h() + 18.0;
    let br = Rect::new(c.x + 30.0, c.y + 46.0, bw, bh);
    let typing = visible < message.chars().count();
    fill_rrect(
        br,
        12.0,
        Paint::vertical(
            br,
            mix(th.indigo_600, th.glass_top, 0.55),
            with_a(th.indigo_600, 0.25),
        ),
    );
    stroke_rrect(br, 12.0, 1.0, with_a(th.indigo_300, 0.35));
    ui.fonts.draw_lines(
        &lines,
        br.inset_xy(11.0, 9.0),
        &body,
        Align::Left,
        None,
        visible,
    );
    if typing {
        for i in 0..3 {
            let a = 0.35 + 0.65 * ((t * 6.0 - i as f32 * 0.7).sin() * 0.5 + 0.5);
            fill_circle(
                vec2(br.x + 14.0 + i as f32 * 9.0, br.bottom() + 12.0),
                2.6,
                with_a(th.indigo_300, a),
            );
        }
    } else {
        let ts = th.small().size(11.0);
        ui.fonts.draw_in(
            labels.time,
            Rect::new(br.x, br.bottom() + 4.0, br.w - 4.0, 14.0),
            &ts,
            Align::Right,
        );
    }
    r
}

/// Texts of the phone chrome.
pub struct PhoneLabels<'a> {
    pub time: &'a str,
    pub title: &'a str,
    pub subtitle: &'a str,
    pub today: &'a str,
    pub bot_handle: &'a str,
    pub tab_schedule: &'a str,
    pub tab_bot: &'a str,
    pub tab_board: &'a str,
    pub tab_checklists: &'a str,
}
