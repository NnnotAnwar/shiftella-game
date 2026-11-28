//! The polaroid card and the journal scrapbook (J), design §11.4.
//!
//! A polaroid is a white frame around a photo texture (the render agent's
//! `draw_scene_snap` target), tilted a little, with a handwritten caption and the date.
//! The journal is a kraft-paper panel with six polaroids per page; click one to enlarge
//! it; **←/→** turn pages, **Esc** closes the enlarged photo, then the journal.

use macroquad::prelude::*;

use super::common::{Affine, dashed_rrect, draw_handwritten_block, with_transform};
use crate::ui::draw::{
    self, Paint, fill_rrect, fill_rrect_grain, line, shadow_rrect, stroke_rrect, with_a,
};
use crate::ui::layout::RectExt;
use crate::ui::{
    Align, Glass, Icon, IconButton, Response, TextStyle, Ui, UiSound, Weight, draw_icon, ease_out,
};

/// One photo with its caption.
#[derive(Clone, Debug)]
pub struct PolaroidView {
    /// The scene snapshot (any size; drawn to the 4:3 window, cropped to fill).
    pub photo: Option<Texture2D>,
    /// Render targets come out upside down: set this for them.
    pub flip_y: bool,
    pub caption: String,
    /// "7. 11. 2039" (per language).
    pub date: String,
    /// Degrees, −3…+3 (from the entry's seed).
    pub tilt: f32,
    /// 0 = still white, 1 = fully developed (the morning reveal).
    pub develop: f32,
}

/// Size of a polaroid frame of width `w` (photo 4:3, white border, deep bottom band).
pub fn polaroid_size(w: f32) -> Vec2 {
    let w = w.max(20.0);
    let b = (w * 0.05).max(4.0);
    let ph = (w - 2.0 * b) * 0.75;
    vec2(w, b + ph + w * 0.21)
}

/// Height ÷ width of a typical polaroid (for fitting one into a box).
pub fn polaroid_aspect() -> f32 {
    polaroid_size(200.0).y / 200.0
}

/// Draw a polaroid centred at `c`, `w` wide. Returns the (untilted) hit response.
pub fn polaroid(
    ui: &mut Ui,
    key: impl std::hash::Hash,
    c: Vec2,
    w: f32,
    v: &PolaroidView,
) -> Response {
    let id = ui.id(key);
    let size = polaroid_size(w);
    let r = Rect::new(c.x - size.x / 2.0, c.y - size.y / 2.0, size.x, size.y);
    let resp = ui.interact(id, r);
    let lift = resp.hover_t * 3.0;
    let tilt = v.tilt * (1.0 - 0.6 * resp.hover_t);
    let rr = r.translate(vec2(0.0, -lift));
    let xf = Affine::rotate(rr.center(), tilt);
    with_transform(ui, xf, |ui| draw_polaroid(ui, rr, v, 1.0 + lift * 0.3));
    resp
}

fn draw_polaroid(ui: &mut Ui, r: Rect, v: &PolaroidView, shadow_k: f32) {
    let w = r.w;
    let b = (w * 0.05).max(4.0);
    shadow_rrect(
        r,
        3.0,
        draw::Shadow::new(
            0.0,
            3.0 * shadow_k,
            10.0 * shadow_k,
            0.0,
            Color::new(0.0, 0.0, 0.0, 0.45),
        ),
    );
    fill_rrect_grain(
        r,
        2.5,
        Paint::vertical(r, Color::from_hex(0xFBF8F1), Color::from_hex(0xEDE6D8)),
    );
    let photo = Rect::new(r.x + b, r.y + b, w - 2.0 * b, (w - 2.0 * b) * 0.75);
    fill_rrect(photo, 1.0, Color::from_hex(0x1A1626));
    if let Some(tex) = &v.photo {
        // crop to fill the 4:3 window
        let (tw, th) = (tex.width(), tex.height());
        let want = photo.w / photo.h;
        let src = if tw / th > want {
            let cw = th * want;
            Rect::new((tw - cw) / 2.0, 0.0, cw, th)
        } else {
            let ch = tw / want;
            Rect::new(0.0, (th - ch) / 2.0, tw, ch)
        };
        draw_texture_ex(
            tex,
            photo.x,
            photo.y,
            draw::faded(WHITE),
            DrawTextureParams {
                dest_size: Some(vec2(photo.w, photo.h)),
                source: Some(src),
                flip_y: v.flip_y,
                ..Default::default()
            },
        );
        // photo grade: warm vignette + a soft flash highlight
        let vg = Color::from_hex(0x1A0F08);
        for (k, a) in [(0.0, 0.3), (0.06, 0.16), (0.12, 0.07)] {
            stroke_rrect(photo.inset(photo.w * k), 1.0, photo.w * 0.07, with_a(vg, a));
        }
        draw::glow_ellipse(
            vec2(photo.x + photo.w * 0.3, photo.y + photo.h * 0.25),
            photo.w * 0.35,
            photo.h * 0.3,
            Color::new(1.0, 0.95, 0.85, 0.08),
        );
    }
    // developing: white → photo
    let dev = v.develop.clamp(0.0, 1.0);
    if dev < 0.999 {
        fill_rrect(
            photo,
            1.0,
            with_a(Color::from_hex(0xF1EEE6), 1.0 - ease_out(dev)),
        );
    }
    stroke_rrect(photo, 1.0, 1.0, Color::new(0.0, 0.0, 0.0, 0.18));
    // caption in the bottom band
    let band = Rect::new(
        r.x + b,
        photo.bottom(),
        photo.w,
        r.bottom() - photo.bottom(),
    );
    // the caption gets the whole band; the date sits small in its bottom-right corner, and
    // the handwriting shrinks (down to a still-legible size) until it fits in two lines
    let ink = Color::from_hex(0x2E2840);
    let date_s = TextStyle::new(
        (w * 0.034).clamp(7.0, 11.0),
        Weight::Medium,
        Color::from_hex(0x8A8396),
    )
    .tabular();
    let dw = ui.fonts.measure(&v.date, &date_s);
    let date_line = date_s.line_h() * 0.9;
    // the handwriting shrinks (to a still-legible size) until it fits in two lines; on a
    // small grid polaroid that still can't, the date gives up its corner (the night is
    // written under the photo anyway) — the caption is the memory
    let fit = |ui: &Ui, date_h: f32| {
        let avail = Rect::new(
            band.x + 2.0,
            band.y + 2.0,
            band.w - 4.0,
            (band.h - date_h - 3.0).max(8.0),
        );
        let mut size = (w * 0.05).clamp(10.0, 17.0);
        let min = (w * 0.04).clamp(6.5, 12.0);
        loop {
            let cs = TextStyle::new(size, Weight::Medium, ink).line_height(1.12);
            let n = ui
                .fonts
                .wrap(&v.caption, (avail.w - size * 0.16).max(10.0), &cs)
                .len();
            let max_lines = ((avail.h / cs.line_h()).floor() as usize).clamp(1, 2);
            if n <= max_lines || size <= min {
                return (avail, size, cs, n <= max_lines);
            }
            size = (size - 0.5).max(min);
        }
    };
    let with_date = if v.date.is_empty() { 0.0 } else { date_line };
    let (mut avail, mut size, mut cs, fits) = fit(ui, with_date);
    let mut show_date = !v.date.is_empty();
    if !fits && show_date {
        let (a, sz, c, fits2) = fit(ui, 0.0);
        if fits2 {
            (avail, size, cs) = (a, sz, c);
            show_date = false;
        }
    }
    let max_lines = ((avail.h / cs.line_h()).floor() as usize).clamp(1, 2);
    let lean = size * 0.16;
    let n = ui
        .fonts
        .wrap(&v.caption, (avail.w - lean).max(10.0), &cs)
        .len()
        .clamp(1, max_lines);
    let lines_h = n as f32 * cs.line_h();
    let cap_r = Rect::new(
        avail.x,
        avail.y + (avail.h - lines_h) / 2.0,
        avail.w,
        lines_h,
    );
    draw_handwritten_block(ui, &v.caption, cap_r, &cs, Align::Left, max_lines);
    if show_date {
        ui.fonts.draw(
            &v.date,
            band.right() - dw - 1.0,
            band.bottom() - date_s.line_h() * 0.28,
            &date_s,
        );
    }
}

/// A taped strip of washi tape across a corner (decoration on the scrapbook).
fn tape(c: Vec2, w: f32, deg: f32, color: Color) {
    let (s, co) = deg.to_radians().sin_cos();
    let d = vec2(co, s) * w / 2.0;
    let n = vec2(-s, co) * 7.0;
    draw::fill_convex(
        &[c - d - n, c + d - n, c + d + n, c - d + n],
        Paint::Solid(color),
    );
}

// ---------------------------------------------------------------------------
// Journal
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct JournalEntry {
    pub night: u32,
    /// The night's title ("Rain and the first regular").
    pub title: String,
    /// None: a dashed empty slot ("night 12").
    pub polaroid: Option<PolaroidView>,
    /// Extra photos of that night (photo mode): small prints tucked beside the polaroid;
    /// the enlarged view steps through them.
    pub extras: Vec<PolaroidView>,
}

/// One of Lída's notes on the journal's last page.
#[derive(Clone, Debug)]
pub struct NoteSlip {
    pub number: u32,
    pub found: bool,
    /// Found: where it was found. Not yet: the same line read as a riddle (where to look).
    pub place: String,
    /// Found: the note itself (shown shortened). Empty while not found.
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct JournalView {
    pub entries: Vec<JournalEntry>,
    /// Lída's notes (story mode): an extra last page. Empty = no notes page.
    pub notes: Vec<NoteSlip>,
}

impl JournalView {
    /// Pages of polaroids (six a page).
    pub fn photo_pages(&self) -> usize {
        self.entries.len().div_ceil(PER_PAGE).max(1)
    }
    /// All pages: the photos, then the notes page (when there are notes).
    pub fn pages(&self) -> usize {
        self.photo_pages() + usize::from(!self.notes.is_empty())
    }
    /// The page index of the notes page.
    pub fn notes_page(&self) -> Option<usize> {
        (!self.notes.is_empty()).then(|| self.photo_pages())
    }
}

#[derive(Clone, Debug, Default)]
pub struct JournalLabels {
    pub title: String,
    /// "Page {n} of {m}".
    pub page: String,
    /// "Night {n}" (placeholder replaced here).
    pub night: String,
    pub close: String,
    pub prev: String,
    pub next: String,
    /// "Lída's notes" (the notes page title and the header chip's tooltip).
    pub notes: String,
    /// The line under an unfound note's riddle ("Not found yet").
    pub not_found: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JournalAction {
    Close,
}

#[derive(Clone, Debug, Default)]
pub struct JournalState {
    pub page: usize,
    pub open: Option<usize>,
    /// In the enlarged view: which photo of the night (0 = the polaroid, then the extras).
    pub shown: usize,
    turn: f32,
}

pub const JOURNAL_SIZE: Vec2 = Vec2::new(780.0, 500.0);
const PER_PAGE: usize = 6;

pub fn journal(
    ui: &mut Ui,
    key: impl std::hash::Hash,
    r: Rect,
    v: &JournalView,
    l: &JournalLabels,
    st: &mut JournalState,
) -> Option<JournalAction> {
    let id = ui.id(key);
    let mut action = None;
    ui.glass(r, &Glass::panel().solid().radius(14.0));
    // kraft paper inside the glass frame
    let paper = r.inset(8.0);
    fill_rrect_grain(
        paper,
        9.0,
        Paint::diagonal(paper, Color::from_hex(0x9C7A55), Color::from_hex(0x6F5238)),
    );
    for i in 0..60 {
        let fx = ((i * 7919) % 1000) as f32 / 1000.0;
        let fy = ((i * 104_729) % 1000) as f32 / 1000.0;
        let p = vec2(paper.x + fx * paper.w, paper.y + fy * paper.h);
        let a = (i % 7) as f32 * 0.9;
        line(
            p,
            p + vec2(a.cos(), a.sin()) * (6.0 + (i % 5) as f32 * 3.0),
            0.6,
            Color::new(0.2, 0.12, 0.05, 0.12),
        );
    }
    fill_rrect(
        paper,
        9.0,
        Paint::vertical(
            paper,
            Color::new(1.0, 0.9, 0.7, 0.06),
            Color::new(0.0, 0.0, 0.0, 0.12),
        ),
    );
    stroke_rrect(paper, 9.0, 1.0, Color::new(0.0, 0.0, 0.0, 0.25));

    // header
    let cream = Color::from_hex(0xF6EEDC);
    let head = Rect::new(paper.x + 16.0, paper.y + 10.0, paper.w - 32.0, 30.0);
    draw_icon(
        Icon::Camera,
        vec2(head.x + 10.0, head.center_y()),
        17.0,
        cream,
    );
    let ts = TextStyle::new(18.0, Weight::Bold, cream);
    let tw = super::common::draw_handwritten(
        ui,
        &l.title,
        head.x + 26.0,
        ui.fonts.baseline_in(head, &ts),
        &ts,
    );
    ui.keycap(
        vec2(head.x + 36.0 + tw, head.center_y()),
        &crate::settings::keys::label(crate::settings::keys::Action::Journal),
        false,
    );
    let close = Rect::new(head.right() - 26.0, head.y + 2.0, 26.0, 26.0);
    if ui
        .icon_button(
            id.with("close"),
            close,
            Icon::Close,
            &IconButton::secondary().size(14.0).tooltip(&l.close),
        )
        .clicked
    {
        action = Some(JournalAction::Close);
    }
    let pages = v.pages();
    st.page = st.page.min(pages - 1);
    // header chip: the notes found so far; a click opens the notes page
    if let Some(np) = v.notes_page() {
        let found = v.notes.iter().filter(|n| n.found).count();
        let label = format!("{found}/{}", v.notes.len());
        let cs = TextStyle::new(12.0, Weight::SemiBold, cream);
        let cw = ui.fonts.measure(&label, &cs) + 30.0;
        let chip = Rect::new(close.x - 8.0 - cw, head.y + 3.0, cw, 24.0);
        let resp = ui.interact(id.with("notes_chip"), chip);
        let on = st.page == np;
        fill_rrect(
            chip,
            12.0,
            Color::new(
                1.0,
                0.95,
                0.85,
                if on { 0.22 } else { 0.10 + 0.08 * resp.hover_t },
            ),
        );
        draw_icon(
            Icon::Note,
            vec2(chip.x + 13.0, chip.center_y()),
            12.0,
            cream,
        );
        ui.fonts.draw_in(
            &label,
            Rect::new(chip.x + 22.0, chip.y, chip.w - 26.0, chip.h),
            &cs,
            Align::Center,
        );
        ui.tooltip(&resp, &l.notes);
        if resp.clicked && !on {
            ui.sound(UiSound::Click);
            st.turn = 1.0;
            st.page = np;
            st.open = None;
        }
    }

    // page turn: slide + fade
    st.turn = crate::ui::approach(st.turn, 0.0, ui.dt(), 0.09);
    let grid = Rect::new(
        paper.x + 18.0,
        head.bottom() + 6.0,
        paper.w - 36.0,
        paper.bottom() - head.bottom() - 6.0 - 44.0,
    );
    let cell_w = grid.w / 3.0;
    let cell_h = grid.h / 2.0;
    let pw = (cell_w - 40.0)
        .min((cell_h - 34.0) / polaroid_aspect())
        .max(40.0);
    draw::push_alpha(1.0 - st.turn.abs().min(1.0) * 0.8);
    let shift = st.turn * 60.0;
    let start = st.page * PER_PAGE;
    let tape_cols = [
        Color::new(0.75, 0.72, 0.95, 0.55),
        Color::new(1.0, 0.85, 0.6, 0.5),
        Color::new(0.7, 0.9, 0.85, 0.5),
    ];
    let mut clicked = None;
    if Some(st.page) == v.notes_page() {
        notes_page(ui, grid.translate(vec2(shift, 0.0)), &v.notes, l, cream);
    }
    for (k, e) in v.entries.iter().enumerate().skip(start).take(PER_PAGE) {
        let slot = k - start;
        let (cx, cy) = ((slot % 3) as f32, (slot / 3) as f32);
        let cell = Rect::new(
            grid.x + cx * cell_w + shift,
            grid.y + cy * cell_h,
            cell_w,
            cell_h,
        );
        let size = polaroid_size(pw);
        let c = vec2(cell.center().x, cell.y + 6.0 + size.y / 2.0);
        let label = l.night.replace("{n}", &e.night.to_string());
        match &e.polaroid {
            Some(p) => {
                let resp = polaroid(ui, id.with(("pol", k)), c, pw, p);
                tape(
                    vec2(c.x - size.x * 0.32, c.y - size.y / 2.0 + 2.0),
                    34.0,
                    -28.0 + p.tilt * 2.0,
                    tape_cols[k % 3],
                );
                if resp.clicked {
                    ui.sound(UiSound::Click);
                    clicked = Some(k);
                    st.shown = 0;
                }
                ui.tooltip(&resp, &p.caption);
            }
            None => {
                let er = Rect::new(c.x - size.x / 2.0, c.y - size.y / 2.0, size.x, size.y);
                fill_rrect(er, 3.0, Color::new(0.0, 0.0, 0.0, 0.1));
                dashed_rrect(er, 3.0, 1.2, 5.0, with_a(cream, 0.4));
                ui.fonts.draw_in(
                    &label,
                    er,
                    &TextStyle::new(12.0, Weight::Medium, with_a(cream, 0.55)),
                    Align::Center,
                );
            }
        }
        // the night's extra photos: little prints tucked along the right edge, the newest on top
        if !e.extras.is_empty() {
            let tw = (pw * 0.26).clamp(24.0, 42.0);
            let ts = polaroid_size(tw);
            let n = e.extras.len().min(3);
            let x = (c.x + size.x / 2.0 + 6.0).min(cell.right() - tw / 2.0 - 2.0);
            for (i, ex) in e.extras.iter().take(n).enumerate() {
                let y = c.y - size.y / 2.0 + ts.y / 2.0 + 4.0 + i as f32 * ts.y * 0.55;
                let mut v = ex.clone();
                v.tilt = 4.0 + i as f32 * 3.0 - 2.0 * v.tilt.signum();
                v.caption = String::new();
                v.date = String::new();
                let resp = polaroid(ui, id.with(("extra", k, i)), vec2(x, y), tw, &v);
                ui.tooltip(&resp, &ex.caption);
                if resp.clicked {
                    ui.sound(UiSound::Click);
                    clicked = Some(k);
                    st.shown = i + usize::from(e.polaroid.is_some());
                }
            }
            if e.extras.len() > n {
                let r = Rect::new(
                    x - 12.0,
                    c.y - size.y / 2.0 + 4.0 + n as f32 * ts.y * 0.55 + ts.y * 0.5,
                    24.0,
                    14.0,
                );
                ui.fonts.draw_in(
                    &format!("+{}", e.extras.len() - n),
                    r,
                    &TextStyle::new(10.0, Weight::Bold, cream),
                    Align::Center,
                );
            }
        }
        let ty = c.y + size.y / 2.0 + 6.0;
        let ls = TextStyle::new(11.0, Weight::SemiBold, with_a(cream, 0.95));
        let txt = if e.polaroid.is_some() {
            format!("{label} · {}", e.title)
        } else {
            String::new()
        };
        ui.fonts.draw_in(
            &txt,
            Rect::new(cell.x + 8.0, ty, cell.w - 16.0, 16.0),
            &ls,
            Align::Center,
        );
    }
    draw::pop_alpha();
    if let Some(k) = clicked {
        st.open = Some(k);
    }

    // footer: page arrows
    let foot = Rect::new(paper.x + 16.0, paper.bottom() - 36.0, paper.w - 32.0, 28.0);
    let page_txt = l
        .page
        .replace("{n}", &(st.page + 1).to_string())
        .replace("{m}", &pages.to_string());
    ui.fonts.draw_in(
        &page_txt,
        foot,
        &TextStyle::new(12.0, Weight::Medium, cream),
        Align::Center,
    );
    let pw_btn = 28.0;
    let prev = Rect::new(foot.center().x - 90.0 - pw_btn, foot.y, pw_btn, 28.0);
    let next = Rect::new(foot.center().x + 90.0, foot.y, pw_btn, 28.0);
    let mut turn: i32 = 0;
    if ui
        .icon_button(
            id.with("prev"),
            prev,
            Icon::ChevronLeft,
            &IconButton::secondary()
                .size(14.0)
                .disabled(st.page == 0)
                .tooltip(&l.prev),
        )
        .clicked
    {
        turn = -1;
    }
    if ui
        .icon_button(
            id.with("next"),
            next,
            Icon::ChevronRight,
            &IconButton::secondary()
                .size(14.0)
                .disabled(st.page + 1 >= pages)
                .tooltip(&l.next),
        )
        .clicked
    {
        turn = 1;
    }

    // enlarged photo
    if let Some(k) = st.open {
        let views: Vec<&PolaroidView> = v
            .entries
            .get(k)
            .map(|e| e.polaroid.iter().chain(e.extras.iter()).collect())
            .unwrap_or_default();
        st.shown = st.shown.min(views.len().saturating_sub(1));
        if let Some(p) = views.get(st.shown).copied() {
            let a = ui.ease_value(id.with(("big", k)), 1.0, 0.08);
            fill_rrect(paper, 9.0, Color::new(0.03, 0.02, 0.07, 0.6 * a));
            ui.region(paper);
            let bw = ((paper.h - 150.0) / polaroid_aspect())
                .clamp(60.0, (paper.w * 0.6).clamp(60.0, 400.0));
            let c = vec2(paper.center().x, paper.center_y() - 22.0 + (1.0 - a) * 12.0);
            let mut big = p.clone();
            big.tilt *= 0.4;
            draw::push_alpha(a);
            let resp = polaroid(ui, id.with(("bigpol", k, st.shown)), c, bw, &big);
            let e = &v.entries[k];
            let mut title = format!(
                "{} · {}",
                l.night.replace("{n}", &e.night.to_string()),
                e.title
            );
            if views.len() > 1 {
                title.push_str(&format!(" · {}/{}", st.shown + 1, views.len()));
            }
            let ty = c.y + polaroid_size(bw).y / 2.0 + 8.0;
            ui.fonts.draw_in(
                &title,
                Rect::new(paper.x, ty, paper.w, 20.0),
                &TextStyle::new(13.0, Weight::SemiBold, cream),
                Align::Center,
            );
            // more than one photo that night: arrows step through them (← → too)
            let mut step: i32 = 0;
            if views.len() > 1 {
                let cy = c.y;
                let half = bw / 2.0 + 30.0;
                let pr = Rect::new(c.x - half - 14.0, cy - 14.0, 28.0, 28.0);
                let nr = Rect::new(c.x + half - 14.0, cy - 14.0, 28.0, 28.0);
                if ui
                    .icon_button(
                        id.with("big_prev"),
                        pr,
                        Icon::ChevronLeft,
                        &IconButton::secondary().size(14.0).disabled(st.shown == 0),
                    )
                    .clicked
                {
                    step = -1;
                }
                if ui
                    .icon_button(
                        id.with("big_next"),
                        nr,
                        Icon::ChevronRight,
                        &IconButton::secondary()
                            .size(14.0)
                            .disabled(st.shown + 1 >= views.len()),
                    )
                    .clicked
                {
                    step = 1;
                }
                if ui.key_repeat(KeyCode::Left) {
                    step = -1;
                }
                if ui.key_repeat(KeyCode::Right) {
                    step = 1;
                }
            }
            draw::pop_alpha();
            let backdrop = ui.interact(id.with("backdrop"), paper);
            if step != 0 {
                st.shown = (st.shown as i32 + step).clamp(0, views.len() as i32 - 1) as usize;
            } else if resp.clicked || backdrop.clicked {
                st.open = None;
            }
        } else {
            st.open = None;
        }
    }

    // keyboard
    if st.open.is_none() {
        if ui.key_repeat(KeyCode::Left) && st.page > 0 {
            turn = -1;
        }
        if ui.key_repeat(KeyCode::Right) && st.page + 1 < pages {
            turn = 1;
        }
    }
    if turn != 0 {
        let np = (st.page as i32 + turn).clamp(0, pages as i32 - 1) as usize;
        if np != st.page {
            st.page = np;
            st.turn = turn as f32;
            ui.sound(UiSound::Click);
        }
    }
    if ui.esc() {
        if st.open.is_some() {
            st.open = None;
        } else {
            action = Some(JournalAction::Close);
        }
    }
    action
}

/// The notes page: ten slips in two columns. A found note shows where it was and the start
/// of its text; an unfound one shows that place as a riddle (where to look, and when).
fn notes_page(ui: &mut Ui, grid: Rect, notes: &[NoteSlip], l: &JournalLabels, cream: Color) {
    let ts = TextStyle::new(14.0, Weight::Bold, cream);
    super::common::draw_handwritten(ui, &l.notes, grid.x + 4.0, grid.y + 16.0, &ts);
    let rows = notes.len().div_ceil(2).max(1);
    let top = grid.y + 26.0;
    let gap = 8.0;
    let slip_h = ((grid.bottom() - top - gap * (rows - 1) as f32) / rows as f32).min(76.0);
    let slip_w = (grid.w - gap) / 2.0;
    let paper = Color::from_hex(0xF3E9D2);
    let ink = Color::from_hex(0x3B2F4A);
    for (i, n) in notes.iter().enumerate() {
        let (col, row) = ((i / rows) as f32, (i % rows) as f32);
        let r = Rect::new(
            grid.x + col * (slip_w + gap),
            top + row * (slip_h + gap),
            slip_w,
            slip_h,
        );
        let badge = vec2(r.x + 16.0, r.y + 16.0);
        let body = Rect::new(r.x + 32.0, r.y + 6.0, r.w - 40.0, r.h - 10.0);
        if n.found {
            shadow_rrect(
                r,
                3.0,
                draw::Shadow::new(0.0, 2.0, 6.0, 0.0, Color::new(0.0, 0.0, 0.0, 0.3)),
            );
            fill_rrect(r, 3.0, paper);
            draw::fill_circle(badge, 9.0, with_a(ink, 0.12));
            ui.fonts.draw_in(
                &n.number.to_string(),
                Rect::new(badge.x - 9.0, badge.y - 9.0, 18.0, 18.0),
                &TextStyle::new(11.0, Weight::Bold, ink),
                Align::Center,
            );
            ui.fonts.draw_in(
                &n.place,
                Rect::new(body.x, body.y, body.w, 14.0),
                &TextStyle::new(10.5, Weight::SemiBold, with_a(ink, 0.6)),
                Align::Left,
            );
            let hs = TextStyle::new(12.0, Weight::Medium, ink);
            let lines = ((body.h - 16.0) / hs.line_h()).floor().max(1.0) as usize;
            super::common::draw_handwritten_block(
                ui,
                &n.text,
                Rect::new(body.x, body.y + 16.0, body.w, body.h - 16.0),
                &hs,
                Align::Left,
                lines,
            );
        } else {
            fill_rrect(r, 3.0, Color::new(0.0, 0.0, 0.0, 0.12));
            dashed_rrect(r, 3.0, 1.2, 5.0, with_a(cream, 0.45));
            draw::fill_circle(badge, 9.0, Color::new(1.0, 1.0, 1.0, 0.08));
            ui.fonts.draw_in(
                "?",
                Rect::new(badge.x - 9.0, badge.y - 9.0, 18.0, 18.0),
                &TextStyle::new(12.0, Weight::Bold, with_a(cream, 0.8)),
                Align::Center,
            );
            ui.fonts.draw_in(
                &format!("{} · {}", n.number, l.not_found),
                Rect::new(body.x, body.y, body.w, 14.0),
                &TextStyle::new(10.5, Weight::SemiBold, with_a(cream, 0.55)),
                Align::Left,
            );
            let hs = TextStyle::new(12.0, Weight::Medium, with_a(cream, 0.92));
            let lines = ((body.h - 16.0) / hs.line_h()).floor().max(1.0) as usize;
            super::common::draw_handwritten_block(
                ui,
                &n.place,
                Rect::new(body.x, body.y + 16.0, body.w, body.h - 16.0),
                &hs,
                Align::Left,
                lines,
            );
        }
    }
}
