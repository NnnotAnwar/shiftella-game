//! Grandma's jukebox: the record picker (design §10.3). Sleeves with procedural cover art,
//! title, mood chip and the faces of regulars known to like it; locked records are
//! silhouettes with a hint; the record now playing slides out and spins.
//! Keyboard: **←/→** choose, **Enter** play, **R** Repeat / Play once, **Esc** close.

use macroquad::prelude::*;

use super::common::mini_chip;
use crate::ui::draw::{
    self, Paint, arc, fill_circle, fill_convex, fill_rrect, line, mix, shadow_rrect, stroke_circle,
    stroke_rrect, with_a,
};
use crate::ui::layout::RectExt;
use crate::ui::{
    Align, Glass, Icon, IconButton, TextStyle, Ui, UiSound, Weight, draw_icon, ease_out,
};

/// Cover-art motif of a sleeve (`records.ron` → `sleeve.motif`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SleeveMotif {
    /// The house playlist (lamp light and notes).
    House,
    Tram,
    Raindrops,
    PianoKeys,
    Sunflower,
    SynthGrid,
    Rose,
}

#[derive(Clone, Debug)]
pub struct RecordView {
    pub title: String,
    pub artist: String,
    /// Mood chip text ("Lively").
    pub mood: String,
    pub mood_color: Color,
    pub motif: SleeveMotif,
    /// Sleeve base colour.
    pub color: Color,
    pub locked: bool,
    /// Locked: how to get it ("Ask Oleh about Lviv").
    pub hint: Option<String>,
    /// Regulars whose taste Karolina has seen (face keys).
    pub fans: Vec<String>,
    pub playing: bool,
}

#[derive(Clone, Debug)]
pub struct RecordsView {
    /// The house playlist first, then the records.
    pub records: Vec<RecordView>,
    pub repeat: bool,
}

#[derive(Clone, Debug, Default)]
pub struct RecordLabels {
    pub title: String,
    pub repeat: String,
    pub play_once: String,
    pub now_playing: String,
    pub close: String,
    /// Tooltip prefix on fans ("Likes it:").
    pub liked_by: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordAction {
    Play(usize),
    SetRepeat(bool),
    Close,
}

#[derive(Clone, Debug, Default)]
pub struct RecordState {
    pub cursor: usize,
    keyboard: bool,
}

const SLEEVE: f32 = 82.0;
const CELL_W: f32 = 92.0;

pub fn record_picker_size(n: usize) -> Vec2 {
    vec2(n.max(1) as f32 * CELL_W + 20.0, 40.0 + SLEEVE + 64.0 + 12.0)
}

/// The picker centred above `anchor` (the jukebox, UI units), kept on screen.
pub fn record_picker(
    ui: &mut Ui,
    key: impl std::hash::Hash,
    anchor: Vec2,
    v: &RecordsView,
    l: &RecordLabels,
    st: &mut RecordState,
) -> Option<RecordAction> {
    let t = ui.theme.clone();
    let id = ui.id(key);
    let size = record_picker_size(v.records.len());
    let s = ui.screen();
    let x = (anchor.x - size.x / 2.0).clamp(10.0, (s.w - size.x - 10.0).max(10.0));
    let y = (anchor.y - size.y - 18.0).clamp(10.0, (s.h - size.y - 10.0).max(10.0));
    let k = ui.ease_value(id.with("appear"), 1.0, 0.07);
    let r = Rect::new(x, y + (1.0 - ease_out(k)) * 8.0, size.x, size.y);
    draw::push_alpha(k);
    ui.glass(r, &Glass::panel().solid().radius(14.0));
    let mut action = None;
    let inner = r.inset_xy(10.0, 10.0);

    // header: title · repeat toggle · close
    let head = inner.with_h(28.0);
    draw_icon(
        Icon::Vinyl,
        vec2(head.x + 12.0, head.center_y()),
        16.0,
        t.amber,
    );
    let close = Rect::new(head.right() - 24.0, head.y + 2.0, 24.0, 24.0);
    let seg_w = 176.0;
    let seg = Rect::new(close.x - 8.0 - seg_w, head.y + 1.0, seg_w, 26.0);
    ui.fonts.draw_in(
        &l.title,
        Rect::new(head.x + 26.0, head.y, seg.x - head.x - 34.0, head.h),
        &t.h2(),
        Align::Left,
    );
    let mut sel = if v.repeat { 0 } else { 1 };
    if ui
        .segmented(
            id.with("repeat"),
            seg,
            &[l.repeat.as_str(), l.play_once.as_str()],
            &mut sel,
        )
        .changed
    {
        action = Some(RecordAction::SetRepeat(sel == 0));
    }
    if ui
        .icon_button(
            id.with("close"),
            close,
            Icon::Close,
            &IconButton::ghost().size(14.0).tooltip(&l.close),
        )
        .clicked
    {
        action = Some(RecordAction::Close);
    }

    // sleeves
    let top = head.bottom() + 12.0;
    for (i, rec) in v.records.iter().enumerate() {
        let cell = Rect::new(inner.x + i as f32 * CELL_W, top, CELL_W, SLEEVE + 64.0);
        let resp = ui.interact(id.with(("rec", i)), cell);
        let focus = st.keyboard && st.cursor == i;
        let h = if rec.locked {
            resp.hover_t * 0.3
        } else {
            resp.hover_t.max(if focus { 1.0 } else { 0.0 })
        };
        let out = ui.ease_value(
            id.with(("out", i)),
            if rec.playing { 1.0 } else { 0.0 },
            0.12,
        );
        let sx = cell.x + (CELL_W - SLEEVE) / 2.0 - out * 4.0;
        let sleeve = Rect::new(sx, top - h * 3.0, SLEEVE, SLEEVE);
        // the disc slides out to the right while it plays
        if out > 0.01 {
            let dc = vec2(sleeve.center().x + out * 16.0, sleeve.center_y());
            draw_disc(dc, SLEEVE * 0.46, ui.time() as f32 * 2.2, rec.color);
        }
        shadow_rrect(
            sleeve,
            5.0,
            draw::Shadow::new(
                0.0,
                4.0 + 3.0 * h,
                12.0,
                0.0,
                Color::new(0.0, 0.0, 0.0, 0.45),
            ),
        );
        if rec.locked {
            fill_rrect(
                sleeve,
                5.0,
                Paint::vertical(sleeve, Color::from_hex(0x2B2752), Color::from_hex(0x1B1938)),
            );
            super::common::dashed_rrect(
                sleeve.inset(5.0),
                3.0,
                1.0,
                4.0,
                with_a(t.indigo_300, 0.25),
            );
            draw_icon(
                Icon::Lock,
                sleeve.center(),
                20.0,
                with_a(t.indigo_300, 0.55),
            );
        } else {
            draw_sleeve(sleeve, rec.motif, rec.color, ui.time());
        }
        stroke_rrect(
            sleeve,
            5.0,
            1.0,
            mix(
                Color::new(1.0, 1.0, 1.0, 0.16),
                with_a(t.indigo_300, 0.9),
                h,
            ),
        );
        if focus {
            stroke_rrect(sleeve.grow(3.0), 7.0, 1.5, t.indigo_300);
        }
        // fans' faces on the sleeve's bottom edge
        let mut fx = sleeve.x + 10.0;
        for f in rec.fans.iter().take(3) {
            let c = vec2(fx, sleeve.bottom() - 2.0);
            fill_circle(c, 9.0, t.glass_solid_top);
            ui.avatar(c, 16.0, f, None, false);
            fx += 13.0;
        }
        if !rec.fans.is_empty() {
            let fr = Rect::new(sleeve.x, sleeve.bottom() - 11.0, fx - sleeve.x, 18.0);
            let fresp = ui.interact(id.with(("fans", i)), fr);
            ui.tooltip(&fresp, &format!("{} {}", l.liked_by, rec.fans.join(", ")));
        }
        // text under the sleeve
        let ty = top + SLEEVE + 10.0;
        let tr = Rect::new(cell.x + 3.0, ty, cell.w - 6.0, 30.0);
        if rec.locked {
            let hint = rec.hint.as_deref().unwrap_or("?");
            ui.fonts.draw_wrapped(
                hint,
                tr.with_h(44.0),
                &t.small().size(11.0).line_height(1.3),
                Align::Center,
                Some(3),
            );
        } else {
            let ts = TextStyle::new(
                11.5,
                Weight::SemiBold,
                if rec.playing { t.amber } else { t.text },
            )
            .line_height(1.3);
            ui.fonts
                .draw_wrapped(&rec.title, tr, &ts, Align::Center, Some(2));
            let ms = TextStyle::new(11.0, Weight::SemiBold, WHITE).tabular();
            let mood = ui
                .fonts
                .ellipsize(&rec.mood, cell.w - 18.0, &ms)
                .into_owned();
            let mw = super::common::mini_chip_width(ui, &mood, false);
            let mc = vec2(cell.x + (cell.w - mw) / 2.0, ty + 32.0);
            mini_chip(
                ui,
                mc,
                &mood,
                mix(rec.mood_color, WHITE, 0.2),
                with_a(rec.mood_color, 0.18),
                None,
            );
            if rec.playing {
                equalizer(
                    vec2(cell.x + (cell.w - mw) / 2.0 - 14.0, ty + 41.0),
                    ui.time(),
                    t.amber,
                );
            }
        }
        let tip = if rec.locked {
            rec.hint.clone().unwrap_or_default()
        } else if rec.playing {
            format!("{} · {} · {}", l.now_playing, rec.title, rec.artist)
        } else {
            format!("{} · {}", rec.title, rec.artist)
        };
        ui.tooltip(&resp, &tip);
        if resp.clicked && !rec.locked {
            ui.sound(UiSound::Click);
            st.cursor = i;
            action = Some(RecordAction::Play(i));
        }
    }
    draw::pop_alpha();

    // keyboard
    let n = v.records.len();
    if n > 0 {
        if ui.take_key(KeyCode::Left) {
            st.keyboard = true;
            st.cursor = (st.cursor + n - 1) % n;
        }
        if ui.take_key(KeyCode::Right) {
            st.keyboard = true;
            st.cursor = (st.cursor + 1) % n;
        }
        if ui.enter() && !v.records[st.cursor.min(n - 1)].locked {
            action = Some(RecordAction::Play(st.cursor.min(n - 1)));
        }
    }
    if ui.take_key(KeyCode::R) {
        action = Some(RecordAction::SetRepeat(!v.repeat));
    }
    if ui.esc() {
        action = Some(RecordAction::Close);
    }
    action
}

/// Three tiny animated bars ("now playing").
fn equalizer(left_mid: Vec2, t: f64, c: Color) {
    for i in 0..3 {
        let h = 3.0 + 5.0 * (0.5 + 0.5 * ((t * 7.0 + i as f64 * 1.9).sin() as f32));
        fill_rrect(
            Rect::new(left_mid.x + i as f32 * 3.5, left_mid.y + 4.0 - h, 2.2, h),
            1.0,
            c,
        );
    }
}

/// A spinning vinyl disc with a label in `label` colour.
fn draw_disc(c: Vec2, r: f32, rot: f32, label: Color) {
    fill_circle(
        c,
        r,
        Paint::vertical(
            Rect::new(c.x - r, c.y - r, r * 2.0, r * 2.0),
            Color::from_hex(0x24212C),
            Color::from_hex(0x0E0C12),
        ),
    );
    for k in 1..5 {
        stroke_circle(
            c,
            r * (0.45 + k as f32 * 0.12),
            0.6,
            Color::new(1.0, 1.0, 1.0, 0.06),
        );
    }
    // a rotating glint
    arc(
        c,
        r * 0.8,
        rot,
        rot + 0.7,
        1.4,
        Color::new(1.0, 1.0, 1.0, 0.18),
    );
    arc(
        c,
        r * 0.8,
        rot + std::f32::consts::PI,
        rot + std::f32::consts::PI + 0.7,
        1.4,
        Color::new(1.0, 1.0, 1.0, 0.1),
    );
    fill_circle(c, r * 0.32, label);
    fill_circle(c, r * 0.05, Color::from_hex(0x0E0C12));
}

/// Procedural sleeve art inside `r`.
pub fn draw_sleeve(r: Rect, motif: SleeveMotif, base: Color, time: f64) {
    let dark = mix(base, Color::from_hex(0x120C1F), 0.45);
    fill_rrect(r, 5.0, Paint::diagonal(r, mix(base, WHITE, 0.12), dark));
    let u = r.w / 80.0;
    let p = |x: f32, y: f32| vec2(r.x + x * u, r.y + y * u);
    let cream = Color::from_hex(0xF6EEDC);
    match motif {
        SleeveMotif::House => {
            draw::glow_circle(
                p(40.0, 30.0),
                34.0 * u,
                with_a(Color::from_hex(0xFFC46B), 0.55),
            );
            // a lamp shade
            fill_convex(
                &[p(30.0, 30.0), p(50.0, 30.0), p(45.0, 20.0), p(35.0, 20.0)],
                Paint::Solid(Color::from_hex(0xFFDCA0)),
            );
            line(p(40.0, 8.0), p(40.0, 20.0), 1.2 * u, cream);
            for (i, x) in [24.0, 40.0, 56.0].iter().enumerate() {
                let y = 54.0 + ((time * 1.5 + i as f64) as f32).sin() * 2.0;
                fill_circle(p(*x, y + 6.0), 3.2 * u, cream);
                line(p(x + 3.0, y + 6.0), p(x + 3.0, y - 6.0), 1.4 * u, cream);
            }
        }
        SleeveMotif::Tram => {
            fill_rrect(
                Rect::new(r.x, r.y + 56.0 * u, r.w, 24.0 * u),
                0.0,
                with_a(BLACK, 0.18),
            );
            line(p(4.0, 64.0), p(76.0, 64.0), 1.2 * u, with_a(cream, 0.6));
            let body = Rect::new(p(14.0, 34.0).x, p(0.0, 34.0).y, 52.0 * u, 26.0 * u);
            fill_rrect(
                body,
                5.0 * u,
                Paint::vertical(body, Color::from_hex(0xE8463C), Color::from_hex(0xB42A2A)),
            );
            fill_rrect(
                Rect::new(body.x, body.y + 16.0 * u, body.w, 4.0 * u),
                0.0,
                cream,
            );
            for i in 0..4 {
                fill_rrect(
                    Rect::new(
                        body.x + (4.0 + i as f32 * 12.0) * u,
                        body.y + 4.0 * u,
                        9.0 * u,
                        9.0 * u,
                    ),
                    1.5 * u,
                    Color::from_hex(0xFFE7A8),
                );
            }
            line(p(40.0, 34.0), p(46.0, 22.0), 1.2 * u, cream);
            line(p(30.0, 18.0), p(62.0, 26.0), 1.0 * u, with_a(cream, 0.7));
            fill_circle(p(24.0, 61.0), 3.0 * u, Color::from_hex(0x2A2030));
            fill_circle(p(56.0, 61.0), 3.0 * u, Color::from_hex(0x2A2030));
            for (x, y) in [(12.0, 12.0), (64.0, 10.0), (70.0, 22.0)] {
                fill_circle(p(x, y), 0.9 * u, cream);
            }
        }
        SleeveMotif::Raindrops => {
            for i in 0..9 {
                let x = 8.0 + i as f32 * 8.5;
                let y = 6.0 + ((i * 37) % 23) as f32;
                line(p(x, y), p(x - 3.0, y + 12.0), 1.0 * u, with_a(cream, 0.45));
            }
            for (x, y, s) in [(26.0, 40.0, 1.0_f32), (52.0, 30.0, 0.8), (44.0, 52.0, 0.7)] {
                let d = p(x, y);
                fill_convex(
                    &[
                        d + vec2(0.0, -7.0 * s * u),
                        d + vec2(4.6 * s * u, 1.5 * s * u),
                        d + vec2(0.0, 5.4 * s * u),
                        d + vec2(-4.6 * s * u, 1.5 * s * u),
                    ],
                    Paint::Solid(with_a(Color::from_hex(0xBFE4FF), 0.9)),
                );
                fill_circle(
                    d + vec2(0.0, 1.5 * s * u),
                    4.6 * s * u,
                    with_a(Color::from_hex(0xBFE4FF), 0.9),
                );
            }
            for k in 0..3 {
                let rr = (8.0 + k as f32 * 7.0) * u;
                draw::stroke_path(
                    &(0..=24)
                        .map(|i| {
                            let a = std::f32::consts::PI * i as f32 / 24.0;
                            p(40.0, 70.0) + vec2(-a.cos() * rr, -a.sin() * rr * 0.28)
                        })
                        .collect::<Vec<_>>(),
                    false,
                    1.0 * u,
                    Paint::Solid(with_a(cream, 0.35 - k as f32 * 0.08)),
                );
            }
        }
        SleeveMotif::PianoKeys => {
            fill_circle(p(58.0, 20.0), 9.0 * u, Color::from_hex(0xF8EBC4));
            fill_circle(p(62.0, 17.0), 8.0 * u, mix(base, WHITE, 0.08));
            let keys = Rect::new(r.x + 6.0 * u, r.y + 42.0 * u, r.w - 12.0 * u, 30.0 * u);
            fill_rrect(keys, 2.0 * u, cream);
            let kw = keys.w / 9.0;
            for i in 1..9 {
                draw::vline(
                    keys.x + i as f32 * kw,
                    keys.y,
                    keys.bottom(),
                    with_a(BLACK, 0.3),
                );
            }
            for i in [0, 1, 3, 4, 5, 7] {
                fill_rrect(
                    Rect::new(
                        keys.x + (i as f32 + 0.68) * kw,
                        keys.y,
                        kw * 0.62,
                        keys.h * 0.6,
                    ),
                    1.0,
                    Color::from_hex(0x1A1622),
                );
            }
        }
        SleeveMotif::Sunflower => {
            // cross-stitch border
            for i in 0..14 {
                let f = i as f32 / 13.0;
                for a in [p(6.0 + f * 68.0, 6.0), p(6.0 + f * 68.0, 74.0)] {
                    line(
                        a + vec2(-1.4, -1.4) * u,
                        a + vec2(1.4, 1.4) * u,
                        0.9 * u,
                        Color::from_hex(0xC0392B),
                    );
                    line(
                        a + vec2(1.4, -1.4) * u,
                        a + vec2(-1.4, 1.4) * u,
                        0.9 * u,
                        Color::from_hex(0xC0392B),
                    );
                }
            }
            let cc = p(40.0, 40.0);
            for k in 0..12 {
                let a = k as f32 / 12.0 * std::f32::consts::TAU;
                let d = vec2(a.cos(), a.sin());
                let n = vec2(-d.y, d.x);
                fill_convex(
                    &[
                        cc + d * 8.0 * u + n * 4.0 * u,
                        cc + d * 22.0 * u,
                        cc + d * 8.0 * u - n * 4.0 * u,
                    ],
                    Paint::Solid(Color::from_hex(0xF6C343)),
                );
            }
            fill_circle(cc, 9.0 * u, Color::from_hex(0x6B3F1F));
            for k in 0..6 {
                let a = k as f32 / 6.0 * std::f32::consts::TAU;
                fill_circle(
                    cc + vec2(a.cos(), a.sin()) * 4.5 * u,
                    1.0 * u,
                    Color::from_hex(0x3E2411),
                );
            }
        }
        SleeveMotif::SynthGrid => {
            let sun = p(40.0, 40.0);
            fill_circle(
                sun,
                18.0 * u,
                Paint::vertical(
                    Rect::new(sun.x, sun.y - 18.0 * u, 1.0, 36.0 * u),
                    Color::from_hex(0xFFD27A),
                    Color::from_hex(0xFF5FA2),
                ),
            );
            for i in 0..4 {
                fill_rrect(
                    Rect::new(
                        sun.x - 19.0 * u,
                        sun.y + (2.0 + i as f32 * 4.0) * u,
                        38.0 * u,
                        1.4 * u,
                    ),
                    0.0,
                    mix(base, BLACK, 0.2),
                );
            }
            let hz = 56.0;
            fill_rrect(
                Rect::new(r.x, r.y + hz * u, r.w, r.h - hz * u),
                0.0,
                mix(base, BLACK, 0.35),
            );
            for i in 0..9 {
                let x = i as f32 * 10.0;
                line(
                    p(40.0 + (x - 40.0) * 0.25, hz),
                    p(x, 80.0),
                    0.8 * u,
                    with_a(Color::from_hex(0xA78BFA), 0.8),
                );
            }
            for y in [58.0, 62.0, 68.0, 76.0] {
                line(
                    p(0.0, y),
                    p(80.0, y),
                    0.8 * u,
                    with_a(Color::from_hex(0xA78BFA), 0.7),
                );
            }
        }
        SleeveMotif::Rose => {
            let rc = p(40.0, 32.0);
            line(
                rc + vec2(0.0, 8.0 * u),
                p(42.0, 70.0),
                1.6 * u,
                Color::from_hex(0x4E8A55),
            );
            fill_convex(
                &[p(42.0, 52.0), p(54.0, 46.0), p(50.0, 56.0)],
                Paint::Solid(Color::from_hex(0x5E9E63)),
            );
            fill_circle(rc, 11.0 * u, Color::from_hex(0xC2334D));
            for k in 0..3 {
                arc(
                    rc,
                    (3.0 + k as f32 * 3.0) * u,
                    0.3 + k as f32,
                    4.4 + k as f32,
                    1.2 * u,
                    Color::from_hex(0x8E1F35),
                );
            }
            fill_rrect(
                Rect::new(r.x + 18.0 * u, r.y + 72.0 * u, 44.0 * u, 0.8 * u),
                0.0,
                with_a(cream, 0.5),
            );
        }
    }
    // paper wear + a centre hole hint
    fill_rrect(
        Rect::new(r.x, r.y, r.w, r.h * 0.5),
        5.0,
        Paint::vertical(
            r,
            Color::new(1.0, 1.0, 1.0, 0.07),
            Color::new(1.0, 1.0, 1.0, 0.0),
        ),
    );
}
