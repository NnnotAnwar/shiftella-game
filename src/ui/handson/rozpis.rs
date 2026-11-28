//! The rozpis as a light puzzle (design §12.6): preference chips with their quotes,
//! a wellbeing ring per person, the week's fairness "weather" and a compact board that
//! puts them together (cells cycle Off → Full → Early → Late; right-click goes back; drag
//! paints). Nothing is ever red: under-staffing shows in indigo, broken wishes in amber.

use macroquad::prelude::*;

use super::common::ring;
use crate::ui::draw::{
    self, Paint, fill_circle, fill_rrect, mix, shadow_rrect, stroke_circle, stroke_rrect, with_a,
};
use crate::ui::layout::RectExt;
use crate::ui::{Align, Button, Icon, TextStyle, Theme, Ui, UiSound, Weight, draw_icon};

// ---------------------------------------------------------------------------
// Preference chips
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrefStatus {
    Honoured,
    Violated,
    /// Not decided by the draft yet (e.g. MinNights mid-fill).
    Pending,
}

/// One preference ("Tue off", "late", "≤ 3", "♥ Bára").
#[derive(Clone, Debug)]
pub struct PrefChip {
    /// Moon (late), Sunrise (early), Clock (full), Calendar (day off), Home (weekend off),
    /// LinkBroken (no back-to-back), Minus (max nights), Money (min nights), Heart (works
    /// with), Cloud (avoid).
    pub icon: Icon,
    /// Short text on the chip.
    pub label: String,
    /// Their own words (hover).
    pub quote: String,
    pub status: PrefStatus,
    /// A Need (firmer border) rather than a Wish.
    pub need: bool,
    /// WorksWith / AvoidWith: the colleague's face key.
    pub portrait: Option<String>,
}

pub fn pref_chip_width(ui: &Ui, c: &PrefChip) -> f32 {
    let st = TextStyle::new(11.0, Weight::Medium, WHITE);
    (ui.fonts.measure(&c.label, &st) + 24.0 + if c.portrait.is_some() { 14.0 } else { 0.0 }).ceil()
}

/// Draw a preference chip at `pos` (h 20). Hover shows the quote. Returns its rect.
pub fn pref_chip(
    ui: &mut Ui,
    key: impl std::hash::Hash,
    pos: Vec2,
    c: &PrefChip,
    flash: f32,
) -> Rect {
    let t = ui.theme.clone();
    let w = pref_chip_width(ui, c);
    let r = Rect::new(pos.x, pos.y, w, 20.0);
    let id = ui.id(key);
    let resp = ui.interact(id, r);
    let (fg, bg, edge) = match c.status {
        PrefStatus::Honoured => (
            t.emerald_400,
            with_a(t.emerald, 0.14),
            with_a(t.emerald_400, 0.35),
        ),
        PrefStatus::Violated => (t.amber, with_a(t.amber, 0.14), with_a(t.amber, 0.45)),
        PrefStatus::Pending => (
            t.text_dim,
            Color::new(1.0, 1.0, 1.0, 0.07),
            Color::new(1.0, 1.0, 1.0, 0.12),
        ),
    };
    if c.status == PrefStatus::Honoured {
        shadow_rrect(
            r,
            10.0,
            draw::Shadow::new(0.0, 0.0, 8.0, 0.0, with_a(t.emerald, 0.18)),
        );
    }
    if flash > 0.01 {
        shadow_rrect(
            r,
            10.0,
            draw::Shadow::new(0.0, 0.0, 12.0, 1.0, with_a(t.amber, 0.6 * flash)),
        );
    }
    fill_rrect(r, 10.0, mix(bg, with_a(fg, 0.24), resp.hover_t));
    stroke_rrect(
        r,
        10.0,
        if c.need { 1.4 } else { 1.0 },
        if c.need { with_a(fg, 0.85) } else { edge },
    );
    let mut x = r.x + 6.0;
    draw_icon(c.icon, vec2(x + 5.0, r.center_y()), 11.0, fg);
    x += 13.0;
    if let Some(p) = &c.portrait {
        ui.avatar(vec2(x + 6.0, r.center_y()), 13.0, p, None, false);
        x += 14.0;
    }
    let st = TextStyle::new(
        11.0,
        Weight::Medium,
        if c.status == PrefStatus::Pending {
            t.text
        } else {
            mix(fg, WHITE, 0.35)
        },
    );
    ui.fonts
        .draw(&c.label, x, ui.fonts.baseline_in(r, &st), &st);
    if c.status == PrefStatus::Violated {
        fill_circle(vec2(r.right() - 2.0, r.y + 2.0), 3.0, t.amber);
    }
    ui.tooltip(&resp, &c.quote);
    r
}

// ---------------------------------------------------------------------------
// Wellbeing ring & fairness
// ---------------------------------------------------------------------------

/// Ring colour for a wellbeing value (Happy emerald, Fine indigo, Tired amber, Stretched
/// soft violet). Never red.
pub fn wellbeing_color(t: &Theme, wb: f32) -> Color {
    if wb >= 85.0 {
        t.emerald_400
    } else if wb >= 65.0 {
        t.indigo_400
    } else if wb >= 45.0 {
        t.amber
    } else {
        t.violet_400
    }
}

/// A wellbeing ring of diameter `d` at `c` with the number inside.
pub fn wellbeing_ring(ui: &mut Ui, c: Vec2, d: f32, wb: f32) {
    let t = ui.theme.clone();
    let col = wellbeing_color(&t, wb);
    let r = d / 2.0 - 2.0;
    fill_circle(c, r, with_a(col, 0.08));
    ring(c, r, 3.0, wb / 100.0, Color::new(1.0, 1.0, 1.0, 0.08), col);
    let st = TextStyle::new((d * 0.34).clamp(9.0, 14.0), Weight::Bold, t.text).tabular();
    ui.fonts.draw_in(
        &format!("{:.0}", wb.clamp(0.0, 100.0)),
        Rect::new(c.x - r, c.y - r, r * 2.0, r * 2.0),
        &st,
        Align::Center,
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeekWeather {
    Sunny,
    MostlySunny,
    Cloudy,
    /// "Rain, but cosy" — the gentlest way to say the week is hard.
    CosyRain,
}

impl WeekWeather {
    /// From the fairness score (`mean − 0.5 × stdev`).
    pub fn from_score(f: f32) -> Self {
        if f >= 82.0 {
            WeekWeather::Sunny
        } else if f >= 68.0 {
            WeekWeather::MostlySunny
        } else if f >= 52.0 {
            WeekWeather::Cloudy
        } else {
            WeekWeather::CosyRain
        }
    }
    pub fn icon(self) -> Icon {
        match self {
            WeekWeather::Sunny => Icon::Sun,
            WeekWeather::MostlySunny => Icon::CloudSun,
            WeekWeather::Cloudy => Icon::Cloud,
            WeekWeather::CosyRain => Icon::Rain,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FairnessView {
    pub score: f32,
    pub weather: WeekWeather,
    /// "Mostly sunny week".
    pub label: String,
    /// Hover detail ("9 of 11 wishes kept").
    pub detail: String,
}

/// The week's fairness badge in `r` (h ≈ 30).
pub fn fairness_badge(ui: &mut Ui, key: impl std::hash::Hash, r: Rect, f: &FairnessView) {
    let t = ui.theme.clone();
    let id = ui.id(key);
    let resp = ui.interact(id, r);
    let col = match f.weather {
        WeekWeather::Sunny => t.amber,
        WeekWeather::MostlySunny => mix(t.amber, t.indigo_300, 0.4),
        WeekWeather::Cloudy => t.indigo_300,
        WeekWeather::CosyRain => t.violet_400,
    };
    fill_rrect(
        r,
        r.h / 2.0,
        Paint::horizontal(
            r,
            with_a(col, 0.16 + 0.06 * resp.hover_t),
            with_a(col, 0.05),
        ),
    );
    stroke_rrect(r, r.h / 2.0, 1.0, with_a(col, 0.3));
    draw::glow_circle(vec2(r.x + 16.0, r.center_y()), 16.0, with_a(col, 0.25));
    draw_icon(f.weather.icon(), vec2(r.x + 16.0, r.center_y()), 16.0, col);
    let num = format!("{:.0}", f.score);
    let ns = t.number().color(col);
    let nw = ui.fonts.measure(&num, &ns);
    ui.fonts.draw_in(
        &f.label,
        Rect::new(r.x + 30.0, r.y, r.w - 44.0 - nw, r.h),
        &t.small_medium(),
        Align::Left,
    );
    ui.fonts.draw(
        &num,
        r.right() - 12.0 - nw,
        ui.fonts.baseline_in(r, &ns),
        &ns,
    );
    ui.tooltip(&resp, &f.detail);
}

/// Width a fairness badge wants for its label.
pub fn fairness_badge_width(ui: &Ui, f: &FairnessView) -> f32 {
    ui.fonts.measure(&f.label, &ui.theme.small_medium())
        + 44.0
        + ui.fonts.measure("100", &ui.theme.number())
}

// ---------------------------------------------------------------------------
// The board
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ShiftCell {
    #[default]
    Off,
    Full,
    Early,
    Late,
    /// Approved time off (not clickable).
    Leave,
}

impl ShiftCell {
    /// Off → Full → Early → Late → Off (`back` reverses).
    pub fn next(self, back: bool) -> ShiftCell {
        use ShiftCell::*;
        match (self, back) {
            (Off, false) => Full,
            (Full, false) => Early,
            (Early, false) => Late,
            (Late, false) => Off,
            (Off, true) => Late,
            (Late, true) => Early,
            (Early, true) => Full,
            (Full, true) => Off,
            (Leave, _) => Leave,
        }
    }
    fn index(self) -> usize {
        match self {
            ShiftCell::Off => 0,
            ShiftCell::Full => 1,
            ShiftCell::Early => 2,
            ShiftCell::Late => 3,
            ShiftCell::Leave => 4,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RozpisRow {
    pub name: String,
    pub prefs: Vec<PrefChip>,
    pub cells: [ShiftCell; 7],
    /// A cell breaks one of their preferences (amber corner dot).
    pub conflicts: [bool; 7],
    pub wellbeing: f32,
    /// Karolina's own row: always Full, not clickable.
    pub locked: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CoverageNight {
    /// Helpers on this night (Karolina not counted).
    pub on: u8,
    /// Suggested by the weekday demand.
    pub suggested: u8,
    /// Busy-ness 1–3 (cups).
    pub cups: u8,
}

#[derive(Clone, Debug)]
pub struct RozpisView {
    /// Short weekday names, Monday first.
    pub days: [String; 7],
    pub today: Option<usize>,
    pub rows: Vec<RozpisRow>,
    pub coverage: [CoverageNight; 7],
    pub fairness: FairnessView,
    /// "Auto-fill kept 9 of 11 wishes. Ema's 'weekend off' didn't fit Saturday."
    pub autofill_line: Option<String>,
    /// The draft differs from the published week.
    pub dirty: bool,
    /// Published by hand this week already: the button shows a lock and says when (it still
    /// answers a click with the same line).
    pub publish_locked: bool,
}

#[derive(Clone, Debug, Default)]
pub struct RozpisLabels {
    /// Cell text per shift: Off, Full, Early, Late, Leave.
    pub shift: [String; 5],
    pub staff: String,
    pub wellbeing: String,
    pub coverage: String,
    pub autofill: String,
    pub clear: String,
    pub publish: String,
    /// Cell tooltip ("Click: next shift · right-click: back · drag to paint").
    pub cell_tip: String,
    /// Karolina's row tooltip.
    pub locked_tip: String,
    /// "{on} of {suggested} helpers" (placeholders replaced here).
    pub coverage_tip: String,
    /// "The rozpis changes once a week — next changes from night N…" (the locked Publish).
    pub publish_locked_tip: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RozpisAction {
    SetCell {
        row: usize,
        day: usize,
        shift: ShiftCell,
    },
    AutoFill,
    Clear,
    Publish,
    /// The auto-fill result line was clicked: flash the affected chips.
    FlashAutofill,
}

#[derive(Clone, Debug, Default)]
pub struct RozpisState {
    paint: Option<ShiftCell>,
    last: Option<(usize, usize)>,
    /// Keyboard cursor (row, day), shown once an arrow key was used.
    pub cursor: (usize, usize),
    keyboard: bool,
    /// Set by the caller to flash violated chips for a second.
    pub flash_until: f64,
}

const NAME_W: f32 = 232.0;
const RING_W: f32 = 50.0;
const ROW_H: f32 = 52.0;

/// Natural size of the board for `n` people.
pub fn rozpis_board_size(n: usize) -> Vec2 {
    vec2(
        640.0,
        24.0 + n.max(1) as f32 * (ROW_H + 4.0) + 8.0 + 36.0 + 10.0 + 34.0,
    )
}

pub fn rozpis_board(
    ui: &mut Ui,
    key: impl std::hash::Hash,
    r: Rect,
    v: &RozpisView,
    l: &RozpisLabels,
    st: &mut RozpisState,
) -> Option<RozpisAction> {
    let t = ui.theme.clone();
    let id = ui.id(key);
    let mut action = None;
    let col_w = (r.w - 6.0 - NAME_W - RING_W) / 7.0;
    let cx0 = r.x + NAME_W;
    let flash = ((st.flash_until - ui.time()).clamp(0.0, 1.0)) as f32;

    // header row: day names
    ui.fonts.draw_in(
        &l.staff.to_uppercase(),
        Rect::new(r.x + 4.0, r.y, NAME_W, 20.0),
        &t.micro(),
        Align::Left,
    );
    for (i, d) in v.days.iter().enumerate() {
        let cr = Rect::new(cx0 + i as f32 * col_w, r.y, col_w, 20.0);
        let today = v.today == Some(i);
        let ks = t.key().color(if today {
            t.indigo_300
        } else if i >= 5 {
            t.violet_400
        } else {
            t.text_dim
        });
        ui.fonts.draw_in(d, cr, &ks, Align::Center);
        if today {
            fill_rrect(
                Rect::new(cr.center().x - 9.0, cr.bottom() - 1.0, 18.0, 2.0),
                1.0,
                t.indigo_400,
            );
        }
    }
    ui.fonts.draw_in(
        &l.wellbeing.to_uppercase(),
        Rect::new(cx0 + 7.0 * col_w, r.y, RING_W, 20.0),
        &t.micro(),
        Align::Center,
    );

    // rows
    let footer_h = 36.0 + 10.0 + 34.0;
    let view = Rect::new(r.x, r.y + 24.0, r.w, r.h - 24.0 - footer_h - 8.0);
    let content_h = (v.rows.len() as f32 * (ROW_H + 4.0) - 4.0).max(0.0);
    let sa = ui.begin_scroll(id.with("rows"), view, content_h);
    let mouse = ui.mouse();
    if !ui.mouse_down() {
        st.paint = None;
        st.last = None;
    }
    for (ri, row) in v.rows.iter().enumerate() {
        let rr = Rect::new(r.x, sa.top() + ri as f32 * (ROW_H + 4.0), r.w - 6.0, ROW_H);
        if !sa.visible(rr.y, rr.h) {
            continue;
        }
        fill_rrect(
            rr,
            10.0,
            if row.locked {
                with_a(t.indigo, 0.1)
            } else {
                Color::new(1.0, 1.0, 1.0, 0.035)
            },
        );
        if row.locked {
            stroke_rrect(rr, 10.0, 1.0, with_a(t.indigo_300, 0.2));
        }
        ui.avatar(
            vec2(rr.x + 20.0, rr.center_y()),
            28.0,
            &row.name,
            None,
            row.locked,
        );
        let nx = rr.x + 40.0;
        let name_y = if row.prefs.is_empty() {
            rr.center_y() - 9.0
        } else {
            rr.y + 5.0
        };
        ui.fonts.draw_in(
            &row.name,
            Rect::new(nx, name_y, NAME_W - 46.0, 18.0),
            &t.strong().size(12.5),
            Align::Left,
        );
        // chips (as many as fit; the rest go into a "+n" with all quotes)
        let mut x = nx;
        let max_x = rr.x + NAME_W - 8.0;
        let mut hidden: Vec<&PrefChip> = Vec::new();
        for (pi, p) in row.prefs.iter().enumerate() {
            let w = pref_chip_width(ui, p);
            if x + w > max_x || !hidden.is_empty() {
                hidden.push(p);
                continue;
            }
            let fl = if p.status == PrefStatus::Violated {
                flash
            } else {
                0.0
            };
            pref_chip(ui, id.with(("pref", ri, pi)), vec2(x, rr.y + 26.0), p, fl);
            x += w + 4.0;
        }
        if !hidden.is_empty() {
            let label = format!("+{}", hidden.len());
            let mr = Rect::new(x, rr.y + 26.0, 26.0, 20.0);
            if mr.right() <= max_x + 6.0 {
                let resp = ui.interact(id.with(("more", ri)), mr);
                fill_rrect(
                    mr,
                    10.0,
                    Color::new(1.0, 1.0, 1.0, 0.08 + 0.05 * resp.hover_t),
                );
                ui.fonts.draw_in(&label, mr, &t.key(), Align::Center);
                let quotes: Vec<String> = hidden
                    .iter()
                    .map(|p| format!("{} — {}", p.label, p.quote))
                    .collect();
                ui.tooltip(&resp, &quotes.join("\n"));
            }
        }
        // cells
        for (di, cell_v) in row.cells.iter().enumerate() {
            let cell = Rect::new(
                cx0 + di as f32 * col_w + 2.5,
                rr.y + 10.0,
                col_w - 5.0,
                rr.h - 20.0,
            );
            let cid = id.with(("cell", ri, di));
            let resp = ui.interact(cid, cell);
            let clickable = !row.locked && *cell_v != ShiftCell::Leave;
            draw_cell(
                ui,
                cell,
                *cell_v,
                &l.shift[cell_v.index()],
                if clickable { resp.hover_t } else { 0.0 },
                row.locked,
            );
            if row.conflicts[di] {
                fill_circle(vec2(cell.right() - 3.0, cell.y + 3.0), 3.2, t.amber);
            }
            if st.keyboard && st.cursor == (ri, di) {
                stroke_rrect(cell.grow(2.0), 8.0, 1.5, t.indigo_300);
            }
            ui.tooltip(
                &resp,
                if row.locked {
                    &l.locked_tip
                } else {
                    &l.cell_tip
                },
            );
            if !clickable {
                continue;
            }
            if resp.right_clicked {
                ui.sound(UiSound::Toggle);
                action = Some(RozpisAction::SetCell {
                    row: ri,
                    day: di,
                    shift: cell_v.next(true),
                });
            }
            // press starts painting with the next shift; dragging paints the same shift
            if ui.mouse_pressed() && resp.hovered && st.paint.is_none() {
                let s = cell_v.next(false);
                st.paint = Some(s);
                st.last = Some((ri, di));
                ui.sound(UiSound::Toggle);
                action = Some(RozpisAction::SetCell {
                    row: ri,
                    day: di,
                    shift: s,
                });
            } else if let Some(p) = st.paint
                && ui.mouse_down()
                && cell.contains(mouse)
                && st.last != Some((ri, di))
            {
                st.last = Some((ri, di));
                if *cell_v != p {
                    action = Some(RozpisAction::SetCell {
                        row: ri,
                        day: di,
                        shift: p,
                    });
                }
            }
        }
        // wellbeing ring
        let rc = vec2(cx0 + 7.0 * col_w + RING_W / 2.0, rr.center_y());
        wellbeing_ring(ui, rc, 34.0, row.wellbeing);
    }
    ui.end_scroll(sa);

    // keyboard: arrows move, Space / Enter next shift, Shift+Space back, Backspace off
    let n = v.rows.len();
    if n > 0 {
        let (mut cr, mut cd) = st.cursor;
        let mut moved = false;
        if ui.key_repeat(KeyCode::Up) {
            cr = (cr + n - 1) % n;
            moved = true;
        }
        if ui.key_repeat(KeyCode::Down) {
            cr = (cr + 1) % n;
            moved = true;
        }
        if ui.key_repeat(KeyCode::Left) {
            cd = (cd + 6) % 7;
            moved = true;
        }
        if ui.key_repeat(KeyCode::Right) {
            cd = (cd + 1) % 7;
            moved = true;
        }
        if moved {
            st.keyboard = true;
            st.cursor = (cr.min(n - 1), cd);
            let (r0, _) = st.cursor;
            ui.scroll_to(&sa, r0 as f32 * (ROW_H + 4.0), ROW_H);
        }
        let (r0, d0) = (st.cursor.0.min(n - 1), st.cursor.1);
        let row = &v.rows[r0];
        let cell = row.cells[d0];
        if st.keyboard && !row.locked && cell != ShiftCell::Leave {
            let back = ui.shift();
            if ui.take_key(KeyCode::Space) || ui.enter() {
                ui.sound(UiSound::Toggle);
                action = Some(RozpisAction::SetCell {
                    row: r0,
                    day: d0,
                    shift: cell.next(back),
                });
            }
            if ui.take_key(KeyCode::Backspace) {
                action = Some(RozpisAction::SetCell {
                    row: r0,
                    day: d0,
                    shift: ShiftCell::Off,
                });
            }
        }
        if ui.mouse_pressed() {
            st.keyboard = false;
        }
    }

    // coverage footer: dots (suggested hollow, on filled) + cups
    let cov = Rect::new(r.x, view.bottom() + 8.0, r.w, 36.0);
    fill_rrect(cov, 10.0, Color::new(0.03, 0.02, 0.09, 0.3));
    ui.fonts.draw_in(
        &l.coverage.to_uppercase(),
        Rect::new(cov.x + 12.0, cov.y, NAME_W - 16.0, cov.h),
        &t.micro(),
        Align::Left,
    );
    for (i, c) in v.coverage.iter().enumerate() {
        let cr = Rect::new(cx0 + i as f32 * col_w, cov.y, col_w, cov.h);
        let resp = ui.interact(id.with(("cov", i)), cr);
        let n = c.on.max(c.suggested).min(5) as usize;
        let dx0 = cr.center().x - (n as f32 * 7.0 - 2.0) / 2.0;
        for k in 0..n {
            let p = vec2(dx0 + k as f32 * 7.0 + 2.5, cr.y + 12.0);
            if (k as u8) < c.on.min(c.suggested) {
                fill_circle(p, 2.6, t.emerald_400);
            } else if (k as u8) < c.on {
                fill_circle(p, 2.6, t.indigo_300);
            } else {
                stroke_circle(p, 2.3, 1.0, t.indigo_300);
            }
        }
        let cups = c.cups.clamp(1, 3) as usize;
        let cx = cr.center().x - (cups as f32 * 11.0) / 2.0;
        for k in 0..cups {
            draw_icon(
                Icon::Coffee,
                vec2(cx + k as f32 * 11.0 + 5.5, cr.y + 26.0),
                10.0,
                with_a(t.amber, 0.8),
            );
        }
        let tip = l
            .coverage_tip
            .replace("{on}", &c.on.to_string())
            .replace("{suggested}", &c.suggested.to_string());
        ui.tooltip(&resp, &tip);
    }

    // action row: auto-fill line · fairness · Auto-fill · Clear · Publish
    let ar = Rect::new(r.x, cov.bottom() + 10.0, r.w, 30.0);
    let publish = if v.publish_locked && v.dirty {
        Button::new(&l.publish)
            .secondary()
            .icon(Icon::Lock)
            .tooltip(&l.publish_locked_tip)
    } else {
        Button::new(&l.publish)
            .primary()
            .icon(Icon::Check)
            .disabled(!v.dirty)
    };
    let auto = Button::new(&l.autofill).icon(Icon::Sparkle);
    let clear = Button::new(&l.clear).ghost();
    let pw = ui.button_width(&publish);
    let mut aw = ui.button_width(&auto);
    let mut cw = ui.button_width(&clear);
    // a narrow tablet (large UI scale, a long language): Clear, then Auto-fill become icon
    // buttons (with their names as tooltips) so the week's badge keeps its room
    let badge_min = fairness_badge_width(ui, &v.fairness).min(150.0) + 12.0;
    let icon_clear = ar.w < pw + cw + aw + 10.0 + badge_min;
    if icon_clear {
        cw = 30.0;
    }
    let icon_auto = ar.w < pw + cw + aw + 10.0 + badge_min;
    if icon_auto {
        aw = 30.0;
    }
    let mut x = ar.right() - pw;
    if ui
        .button(id.with("publish"), Rect::new(x, ar.y, pw, 30.0), &publish)
        .clicked
    {
        action = Some(RozpisAction::Publish);
    }
    x -= cw + 4.0;
    let clear_r = Rect::new(x, ar.y, cw, 30.0);
    let clicked = if icon_clear {
        ui.icon_button(
            id.with("clear"),
            clear_r,
            Icon::Undo,
            &crate::ui::IconButton::ghost().tooltip(&l.clear),
        )
        .clicked
    } else {
        ui.button(id.with("clear"), clear_r, &clear).clicked
    };
    if clicked {
        action = Some(RozpisAction::Clear);
    }
    x -= aw + 6.0;
    let auto_r = Rect::new(x, ar.y, aw, 30.0);
    let clicked = if icon_auto {
        ui.icon_button(
            id.with("auto"),
            auto_r,
            Icon::Sparkle,
            &crate::ui::IconButton::secondary().tooltip(&l.autofill),
        )
        .clicked
    } else {
        ui.button(id.with("auto"), auto_r, &auto).clicked
    };
    if clicked {
        action = Some(RozpisAction::AutoFill);
    }
    let fw = fairness_badge_width(ui, &v.fairness).min(x - ar.x - 12.0);
    let fr = Rect::new(ar.x, ar.y, fw.max(60.0), 30.0);
    fairness_badge(ui, id.with("fair"), fr, &v.fairness);
    if let Some(line) = &v.autofill_line {
        let lr = Rect::new(
            fr.right() + 10.0,
            ar.y,
            (x - fr.right() - 18.0).max(0.0),
            30.0,
        );
        // the whole line, or just its first sentence, in two lines without breaking a word;
        // when not even that fits (a large UI scale / text size) the fairness badge's
        // tooltip carries it
        let ls0 = t.small().size(11.0).line_height(1.3);
        let fits = |ui: &Ui, s: &str| {
            let words_fit = s
                .split_whitespace()
                .all(|w| ui.fonts.measure(w, &ls0) <= lr.w);
            words_fit && ui.fonts.wrap(s, lr.w, &ls0).len() <= 2
        };
        let first = line
            .split_inclusive(". ")
            .next()
            .unwrap_or(line)
            .trim_end()
            .to_string();
        let shown = [line.clone(), first].into_iter().find(|s| fits(ui, s));
        if lr.w > 40.0
            && let Some(shown) = shown
        {
            let resp = ui.interact(id.with("autoline"), lr);
            let ls = ls0.color(mix(t.text_dim, t.text, resp.hover_t));
            ui.fonts.draw_wrapped(&shown, lr, &ls, Align::Left, Some(2));
            ui.tooltip(&resp, line);
            if resp.clicked {
                action = Some(RozpisAction::FlashAutofill);
            }
        }
    }
    action
}

fn draw_cell(ui: &mut Ui, cell: Rect, s: ShiftCell, label: &str, hover: f32, locked: bool) {
    let t = ui.theme.clone();
    let st = TextStyle::new(10.5, Weight::SemiBold, WHITE).tabular();
    let fit = |ui: &Ui, st: TextStyle| {
        let w = ui.fonts.measure(label, &st);
        if w > cell.w - 4.0 {
            st.size((10.5 * (cell.w - 4.0) / w).max(7.5))
        } else {
            st
        }
    };
    match s {
        ShiftCell::Off => {
            fill_rrect(cell, 7.0, Color::new(1.0, 1.0, 1.0, 0.03 + 0.05 * hover));
            ui.fonts.draw_in(
                label,
                cell,
                &t.small().color(t.text_disabled),
                Align::Center,
            );
        }
        ShiftCell::Leave => {
            fill_rrect(cell, 7.0, with_a(t.amber, 0.12));
            let st = fit(ui, st.color(t.amber));
            ui.fonts.draw_in(label, cell, &st, Align::Center);
        }
        _ => {
            let (c0, c1) = match s {
                ShiftCell::Full => (t.indigo, t.violet),
                ShiftCell::Early => (mix(t.indigo, t.amber_deep, 0.35), t.indigo_600),
                _ => (t.indigo_600, mix(t.violet, BLACK, 0.25)),
            };
            let (c0, c1) = if locked {
                (with_a(c0, 0.55), with_a(c1, 0.55))
            } else {
                (c0, c1)
            };
            shadow_rrect(
                cell,
                7.0,
                draw::Shadow::new(0.0, 2.0, 8.0, -1.0, with_a(c0, 0.3)),
            );
            fill_rrect(
                cell,
                7.0,
                Paint::diagonal(cell, mix(c0, WHITE, 0.1 * hover), c1),
            );
            stroke_rrect(
                cell,
                7.0,
                1.0,
                Paint::vertical(
                    cell,
                    Color::new(1.0, 1.0, 1.0, 0.3),
                    Color::new(1.0, 1.0, 1.0, 0.04),
                ),
            );
            let st = fit(ui, st);
            if ui.fonts.measure(label, &st) > cell.w - 3.0 {
                // a narrow cell (large UI scale): the shift's icon instead of a cut word
                let icon = match s {
                    ShiftCell::Full => Icon::Clock,
                    ShiftCell::Early => Icon::Sunrise,
                    _ => Icon::Moon,
                };
                draw_icon(icon, cell.center(), 12.0, WHITE);
            } else {
                ui.fonts.draw_in(label, cell, &st, Align::Center);
            }
        }
    }
    if hover > 0.01 {
        stroke_rrect(cell, 7.0, 1.0, with_a(t.indigo_300, hover));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shifts_cycle_both_ways() {
        let mut s = ShiftCell::Off;
        for _ in 0..4 {
            let n = s.next(false);
            assert_eq!(n.next(true), s);
            s = n;
        }
        assert_eq!(s, ShiftCell::Off);
        assert_eq!(ShiftCell::Leave.next(false), ShiftCell::Leave);
    }

    #[test]
    fn weather_from_score_and_never_red() {
        assert_eq!(WeekWeather::from_score(90.0), WeekWeather::Sunny);
        assert_eq!(WeekWeather::from_score(70.0), WeekWeather::MostlySunny);
        assert_eq!(WeekWeather::from_score(55.0), WeekWeather::Cloudy);
        assert_eq!(WeekWeather::from_score(10.0), WeekWeather::CosyRain);
        let t = Theme::SHIFTELLA;
        for wb in [0.0, 30.0, 50.0, 70.0, 90.0] {
            assert_ne!(wellbeing_color(&t, wb), t.red);
        }
    }
}
