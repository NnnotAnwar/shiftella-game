//! The duties board (H): who helps with what tonight (design §1.3). Rows are helpers,
//! columns the six duties. Click a duty to give / take it, drag a given duty onto another
//! helper to move it. Capacity dots show how many duties each helper can carry (1–3).
//! Duties nobody holds stay with Karolina (the strip at the bottom).
//!
//! Keyboard: arrows move the focus, **Space**/**Enter** toggle, **S** suggest, **Esc** close.

use macroquad::prelude::*;

use super::common::{Duty, capacity_dots, mini_chip};
use crate::ui::draw::{self, Paint, fill_rrect, mix, shadow_rrect, stroke_rrect, with_a};
use crate::ui::layout::RectExt;
use crate::ui::{Align, Button, Glass, Icon, Ui, UiSound, draw_icon};

#[derive(Clone, Debug)]
pub struct HelperRow {
    /// Display name (also the avatar's face key).
    pub name: String,
    /// "Porter", "Barista"…
    pub role: String,
    /// 1–3 duties.
    pub capacity: u8,
    pub duties: Vec<Duty>,
    pub on_shift: bool,
    /// The archetype's favourite duty (a small heart).
    pub likes: Option<Duty>,
    /// Skill per duty (0–10) in [`Duty::ALL`] order: a faint bar in each cell.
    pub skills: [u8; 6],
    /// "Can take one more duty after 3 nights" etc.
    pub note: Option<String>,
}

#[derive(Clone, Debug)]
pub struct DutiesView {
    pub helpers: Vec<HelperRow>,
    /// Karolina's display name (face key), e.g. "Karolina (you)".
    pub karolina: String,
}

#[derive(Clone, Debug, Default)]
pub struct DutiesLabels {
    pub title: String,
    pub subtitle: String,
    /// Duty names in [`Duty::ALL`] order.
    pub duty: [String; 6],
    /// One-line descriptions (tooltips) in [`Duty::ALL`] order.
    pub duty_desc: [String; 6],
    pub suggest: String,
    /// "Left for you:" before the duties nobody holds.
    pub karolina_takes: String,
    /// Shown when every duty has a helper.
    pub all_covered: String,
    pub off_shift: String,
    /// Tooltip on a full helper's free cells.
    pub full: String,
    /// "{used} of {cap} duties" (placeholders replaced here).
    pub capacity: String,
    /// "skill {n}" in cell tooltips.
    pub skill: String,
    pub likes: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DutiesAction {
    /// Give the duty if the helper doesn't have it, take it away if they do.
    Toggle {
        helper: usize,
        duty: Duty,
    },
    /// Dragged from one helper to another.
    Move {
        duty: Duty,
        from: usize,
        to: usize,
    },
    Suggest,
    Close,
}

/// Board memory: keyboard focus and the duty being dragged.
#[derive(Clone, Debug, Default)]
pub struct DutiesState {
    pub focus: (usize, usize),
    keyboard: bool,
    press: Option<(usize, Duty, Vec2)>,
    dragging: bool,
}

const NAME_W: f32 = 222.0;
const ROW_H: f32 = 50.0;

/// Natural size for `n` helpers (the caller may give less height: rows scroll).
pub fn duties_board_size(n: usize) -> Vec2 {
    vec2(
        760.0,
        45.0 + 10.0 + 26.0 + 4.0 + n.max(1) as f32 * (ROW_H + 6.0) - 6.0 + 8.0 + 52.0 + 12.0,
    )
}

pub fn duties_board(
    ui: &mut Ui,
    key: impl std::hash::Hash,
    r: Rect,
    v: &DutiesView,
    l: &DutiesLabels,
    st: &mut DutiesState,
) -> Option<DutiesAction> {
    let t = ui.theme.clone();
    let id = ui.id(key);
    ui.glass(r, &Glass::panel().solid());
    let (body, closed) = ui.panel_header(
        r,
        &l.title,
        Some(&crate::settings::keys::label(
            crate::settings::keys::Action::Duties,
        )),
        Some(&l.subtitle),
    );
    let mut action = closed.then_some(DutiesAction::Close);
    let inner = body.inset_ltrb(14.0, 10.0, 14.0, 12.0);
    // rows leave 6 pt on the right for a scrollbar; columns follow the rows
    let col_w = (inner.w - 6.0 - NAME_W) / 6.0;

    // column headers
    let hr = inner.with_h(26.0);
    for (ci, d) in Duty::ALL.iter().enumerate() {
        let cr = Rect::new(inner.x + NAME_W + ci as f32 * col_w, hr.y, col_w, hr.h);
        let resp = ui.interact(id.with(("head", ci)), cr);
        let mut hs = t.small_medium().color(t.text_dim);
        // a larger text size gives way before the word is cut (back to 100 %)
        while ui.fonts.measure(&l.duty[ci], &hs) > cr.w - 26.0 && hs.scale > 1.0 {
            hs = hs.scaled((hs.scale - 0.05).max(1.0));
        }
        let label = ui
            .fonts
            .ellipsize(&l.duty[ci], cr.w - 26.0, &hs)
            .into_owned();
        let lw = ui.fonts.measure(&label, &hs);
        let x0 = cr.x + (cr.w - lw - 19.0) / 2.0;
        draw_icon(d.icon(), vec2(x0 + 7.0, cr.center_y()), 14.0, t.indigo_300);
        ui.fonts
            .draw(&label, x0 + 19.0, ui.fonts.baseline_in(cr, &hs), &hs);
        ui.tooltip(&resp, &l.duty_desc[ci]);
    }

    // helper rows (scroll when short of room)
    let strip_h = 52.0;
    let rows_view = Rect::new(
        inner.x,
        hr.bottom() + 4.0,
        inner.w,
        inner.bottom() - hr.bottom() - 4.0 - strip_h - 8.0,
    );
    let content_h = (v.helpers.len() as f32 * (ROW_H + 6.0) - 6.0).max(0.0);
    let sa = ui.begin_scroll(id.with("rows"), rows_view, content_h);
    let mut y = sa.top();
    let mouse = ui.mouse();
    let mut drop_row: Option<usize> = None;
    for (ri, h) in v.helpers.iter().enumerate() {
        let row = Rect::new(inner.x, y, inner.w - 6.0, ROW_H);
        y += ROW_H + 6.0;
        if !sa.visible(row.y, row.h) {
            continue;
        }
        let used = h.duties.len();
        let cap = h.capacity.clamp(1, 3) as usize;
        let full = used >= cap;
        let drag_target =
            st.dragging && row.contains(mouse) && st.press.is_some_and(|(from, _, _)| from != ri);
        if drag_target {
            drop_row = Some(ri);
        }
        fill_rrect(
            row,
            10.0,
            Paint::vertical(
                row,
                Color::new(1.0, 1.0, 1.0, 0.055),
                Color::new(1.0, 1.0, 1.0, 0.03),
            ),
        );
        stroke_rrect(
            row,
            10.0,
            1.0,
            if drag_target {
                with_a(t.indigo_300, 0.9)
            } else {
                Color::new(1.0, 1.0, 1.0, 0.06)
            },
        );
        if !h.on_shift {
            draw::push_alpha(0.55);
        }
        // name block
        ui.avatar(
            vec2(row.x + 24.0, row.center_y()),
            32.0,
            &h.name,
            None,
            false,
        );
        let nx = row.x + 46.0;
        let nw = NAME_W - 52.0;
        ui.fonts.draw_in(
            &h.name,
            Rect::new(nx, row.y + 7.0, nw, 18.0),
            &t.strong(),
            Align::Left,
        );
        let cap_text = l
            .capacity
            .replace("{used}", &used.to_string())
            .replace("{cap}", &cap.to_string());
        let dots_end = capacity_dots(&t, nx, row.y + 34.0, used, cap);
        let sub_r = Rect::new(
            dots_end + 6.0,
            row.y + 26.0,
            (row.x + NAME_W - 6.0 - dots_end - 6.0).max(0.0),
            16.0,
        );
        // the count first; when "Duties: 3/3 · role" doesn't fit, the dots already say
        // "duties", so the count goes numeric ("3/3 · role") before the role would be cut
        let sub = if h.on_shift {
            let full = format!("{cap_text} · {}", h.role);
            if ui.fonts.measure(&full, &t.small()) <= sub_r.w {
                full
            } else {
                format!("{used}/{cap} · {}", h.role)
            }
        } else {
            l.off_shift.clone()
        };
        let resp = ui.interact(id.with(("who", ri)), Rect::new(row.x, row.y, NAME_W, row.h));
        ui.fonts
            .draw_fit(&sub, sub_r, &t.small(), Align::Left, 0.78);
        if let Some(n) = &h.note {
            ui.tooltip(&resp, n);
        }
        // duty cells
        for (ci, d) in Duty::ALL.iter().enumerate() {
            let cell = Rect::new(
                row.x + NAME_W + ci as f32 * col_w + 3.0,
                row.y + 8.0,
                col_w - 6.0,
                row.h - 16.0,
            );
            let on = h.duties.contains(d);
            let blocked = full && !on;
            let cid = id.with(("cell", ri, ci));
            let resp = ui.interact(cid, cell);
            let hover = if blocked { 0.0 } else { resp.hover_t };
            let focus = st.keyboard && st.focus == (ri, ci);
            let k = ui.ease_value(cid.with("on"), if on { 1.0 } else { 0.0 }, 0.07);
            if k > 0.01 {
                shadow_rrect(
                    cell,
                    8.0,
                    draw::Shadow::new(0.0, 2.0, 10.0, -1.0, with_a(t.indigo, 0.35 * k)),
                );
                fill_rrect(
                    cell,
                    8.0,
                    Paint::diagonal(
                        cell,
                        with_a(mix(t.indigo, WHITE, 0.08 * hover), k),
                        with_a(t.violet, k),
                    ),
                );
                stroke_rrect(
                    cell,
                    8.0,
                    1.0,
                    Paint::vertical(
                        cell,
                        Color::new(1.0, 1.0, 1.0, 0.32 * k),
                        Color::new(1.0, 1.0, 1.0, 0.05 * k),
                    ),
                );
            }
            if k < 0.99 {
                let a = 1.0 - k;
                if blocked {
                    super::common::dashed_rrect(
                        cell,
                        8.0,
                        1.0,
                        3.0,
                        Color::new(1.0, 1.0, 1.0, 0.08 * a),
                    );
                } else {
                    fill_rrect(
                        cell,
                        8.0,
                        Color::new(1.0, 1.0, 1.0, (0.035 + 0.06 * hover) * a),
                    );
                    stroke_rrect(
                        cell,
                        8.0,
                        1.0,
                        mix(
                            Color::new(1.0, 1.0, 1.0, 0.07),
                            with_a(t.indigo_300, 0.6),
                            hover,
                        ),
                    );
                }
            }
            if focus {
                stroke_rrect(cell.grow(2.0), 9.0, 1.5, t.indigo_300);
            }
            let fg = if on {
                WHITE
            } else if blocked {
                with_a(t.text_disabled, 0.7)
            } else {
                mix(t.text_dim, t.text, hover)
            };
            let ic = vec2(cell.x + cell.w / 2.0, cell.center_y() - 2.0);
            draw_icon(d.icon(), ic, 16.0, fg);
            // skill bar
            let sk = (h.skills[ci] as f32 / 10.0).clamp(0.0, 1.0);
            let bar = Rect::new(cell.x + cell.w / 2.0 - 12.0, cell.bottom() - 6.0, 24.0, 2.0);
            fill_rrect(
                bar,
                1.0,
                Color::new(1.0, 1.0, 1.0, if on { 0.22 } else { 0.08 }),
            );
            if sk > 0.0 {
                fill_rrect(
                    bar.with_w(24.0 * sk),
                    1.0,
                    if on {
                        with_a(WHITE, 0.85)
                    } else {
                        with_a(t.indigo_300, 0.7)
                    },
                );
            }
            if h.likes == Some(*d) {
                draw_icon(
                    Icon::Heart,
                    vec2(cell.right() - 8.0, cell.y + 8.0),
                    9.0,
                    if on { WHITE } else { t.amber },
                );
            }
            let tip = if blocked {
                l.full.clone()
            } else {
                let mut s = format!("{} · {} {}", l.duty[ci], l.skill, h.skills[ci]);
                if h.likes == Some(*d) {
                    s.push_str(&format!(" · {}", l.likes));
                }
                s
            };
            ui.tooltip(&resp, &tip);
            // press → maybe a drag (only duties she already holds can be dragged)
            if resp.pressed && st.press.is_none() && on {
                st.press = Some((ri, *d, mouse));
            }
            if resp.clicked && !st.dragging && !blocked {
                ui.sound(UiSound::Toggle);
                action = Some(DutiesAction::Toggle {
                    helper: ri,
                    duty: *d,
                });
            }
        }
        if !h.on_shift {
            draw::pop_alpha();
        }
    }
    ui.end_scroll(sa);

    // drag bookkeeping
    if let Some((from, duty, at)) = st.press {
        if ui.mouse_down() {
            if !st.dragging && (mouse - at).length() > 5.0 {
                st.dragging = true;
            }
            if st.dragging {
                let pr = Rect::new(mouse.x - 34.0, mouse.y - 15.0, 68.0, 30.0);
                shadow_rrect(pr, 8.0, t.shadow_md);
                fill_rrect(pr, 8.0, Paint::diagonal(pr, t.indigo, t.violet));
                draw_icon(duty.icon(), pr.center(), 16.0, WHITE);
            }
        } else {
            if st.dragging
                && let Some(to) = drop_row
            {
                ui.sound(UiSound::Toggle);
                action = Some(DutiesAction::Move { duty, from, to });
            }
            st.press = None;
            st.dragging = false;
        }
    }

    // Karolina's strip: what nobody holds goes to her
    let strip = Rect::new(inner.x, inner.bottom() - strip_h, inner.w, strip_h);
    fill_rrect(
        strip,
        10.0,
        Paint::horizontal(strip, with_a(t.indigo, 0.16), with_a(t.violet, 0.06)),
    );
    stroke_rrect(strip, 10.0, 1.0, with_a(t.indigo_300, 0.22));
    ui.avatar(
        vec2(strip.x + 24.0, strip.center_y()),
        30.0,
        &v.karolina,
        None,
        true,
    );
    let unowned: Vec<Duty> = Duty::ALL
        .iter()
        .copied()
        .filter(|d| !v.helpers.iter().any(|h| h.on_shift && h.duties.contains(d)))
        .collect();
    let sug = Button::new(&l.suggest).icon(Icon::Sparkle).shortcut("S");
    let sw = ui.button_width(&sug);
    let sr = Rect::new(strip.right() - sw - 10.0, strip.center_y() - 14.0, sw, 28.0);
    let lx = strip.x + 46.0;
    let ls = t.small_medium().color(t.indigo_300);
    let lw = ui.fonts.measure(&l.karolina_takes, &ls).min(170.0);
    ui.fonts.draw_in(
        &l.karolina_takes,
        Rect::new(lx, strip.y, lw + 1.0, strip.h),
        &ls,
        Align::Left,
    );
    let mut cx = lx + lw + 10.0;
    if unowned.is_empty() {
        ui.fonts.draw_in(
            &l.all_covered,
            Rect::new(cx, strip.y, (sr.x - cx - 8.0).max(0.0), strip.h),
            &t.small(),
            Align::Left,
        );
    }
    for d in &unowned {
        let name = &l.duty[d.index()];
        let w = super::common::mini_chip_width(ui, name, true);
        if cx + w > sr.x - 8.0 {
            break;
        }
        mini_chip(
            ui,
            vec2(cx, strip.center_y() - 9.0),
            name,
            t.text,
            Color::new(1.0, 1.0, 1.0, 0.09),
            Some(d.icon()),
        );
        cx += w + 6.0;
    }
    if ui.button(id.with("suggest"), sr, &sug).clicked {
        action = Some(DutiesAction::Suggest);
    }

    // keyboard
    let n = v.helpers.len();
    if n > 0 {
        let (mut fr, mut fc) = st.focus;
        let mut moved = false;
        if ui.take_key(KeyCode::Up) {
            fr = (fr + n - 1) % n;
            moved = true;
        }
        if ui.take_key(KeyCode::Down) {
            fr = (fr + 1) % n;
            moved = true;
        }
        if ui.take_key(KeyCode::Left) {
            fc = (fc + 5) % 6;
            moved = true;
        }
        if ui.take_key(KeyCode::Right) {
            fc = (fc + 1) % 6;
            moved = true;
        }
        if moved {
            st.keyboard = true;
            st.focus = (fr.min(n - 1), fc);
        }
        if st.keyboard && (ui.take_key(KeyCode::Space) || ui.enter()) {
            let (r0, c0) = st.focus;
            let h = &v.helpers[r0];
            let d = Duty::ALL[c0];
            if h.duties.contains(&d) || h.duties.len() < h.capacity as usize {
                ui.sound(UiSound::Toggle);
                action = Some(DutiesAction::Toggle {
                    helper: r0,
                    duty: d,
                });
            }
        }
    }
    if ui.take_key(KeyCode::S) {
        action = Some(DutiesAction::Suggest);
    }
    if ui.esc() {
        action = Some(DutiesAction::Close);
    }
    if ui.mouse_pressed() && !ui.hovering(r) {
        st.keyboard = false;
    }
    action
}
