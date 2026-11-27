//! What build mode draws over the hall: the held piece's footprint (emerald fits / red
//! doesn't), warm light pools of the lamps with the seats they light, the beauty reach of a
//! held decor piece (the 3-tile radius guests notice), the outline of the piece under the
//! mouse (with its chairs) — and, in the UI pass, the held piece's coziness change by the
//! piece and by the atmosphere meter, and why a piece in use can't move.

use macroquad::prelude::*;

use super::world_view::sim;
use super::{Ctx, GameScreen};
use crate::content::{Content, FurnitureDef};
use crate::i18n::format_decimal;
use crate::map::TILE;
use crate::settings::Locale;
use crate::ui::draw::{fill_circle, fill_rrect, glow_circle, stroke_circle, stroke_rrect, with_a};
use crate::ui::{Align, Icon, RectExt, Theme, Ui, draw_icon};
use sim::Room;

use super::build::Ghost;

/// Beauty reach used by guests around their seat (chebyshev tiles).
const BEAUTY_REACH: i32 = 3;

fn tiles_rect(x: i32, y: i32, w: u32, h: u32) -> Rect {
    Rect::new(
        x as f32 * TILE,
        y as f32 * TILE,
        w as f32 * TILE,
        h as f32 * TILE,
    )
}

fn is_light(d: &FurnitureDef) -> bool {
    d.light_radius > 0.0 && d.has_tag("light")
}

/// A light: footprint-centre in tile coords (as the coziness rule measures) and its reach.
struct Light {
    c: (f32, f32),
    r: f32,
    held: bool,
}

fn center_of(tiles: &[(i32, i32)]) -> (f32, f32) {
    let n = tiles.len().max(1) as f32;
    (
        tiles.iter().map(|t| t.0 as f32).sum::<f32>() / n,
        tiles.iter().map(|t| t.1 as f32).sum::<f32>() / n,
    )
}

/// Lights and hall seats as they would be with the ghost put down.
fn lights_and_seats(
    g: &GameScreen,
    content: &Content,
    ghost: &Ghost,
) -> (Vec<Light>, Vec<(i32, i32)>) {
    let night = g.world.phase() == sim::Phase::Night;
    let hidden = &g.scene.hidden_furniture;
    let mut lights = Vec::new();
    let mut seats = Vec::new();
    let hall = |t: &(i32, i32)| g.world.grid().room(t.0, t.1) == Room::Hall;
    for p in g.world.furniture() {
        if hidden.contains(&p.uid) {
            continue;
        }
        let Some(d) = content.furniture_def(&p.def) else {
            continue;
        };
        let fp = sim::grid::footprint(d, p.x, p.y, p.rot);
        if is_light(d) && (p.on || !night) {
            lights.push(Light {
                c: center_of(&fp),
                r: d.light_radius,
                held: false,
            });
        }
        seats.extend(fp.iter().take(d.seats as usize).filter(|t| hall(t)));
    }
    for gp in &ghost.pieces {
        let Some(d) = content.furniture_def(&gp.def) else {
            continue;
        };
        let rot = (0..4u8)
            .find(|r| sim::grid::rotated_size(d, *r) == gp.size)
            .unwrap_or(0);
        let fp = sim::grid::footprint(d, gp.tile.0, gp.tile.1, rot);
        if is_light(d) {
            lights.push(Light {
                c: center_of(&fp),
                r: d.light_radius,
                held: true,
            });
        }
        seats.extend(fp.iter().take(d.seats as usize).filter(|t| hall(t)));
    }
    (lights, seats)
}

/// Over the lit world (world camera active).
pub fn world(g: &GameScreen, content: &Content, ghost: Option<&Ghost>) {
    let t = Theme::SHIFTELLA;
    let st = &g.ui_state.build;
    if let Some(gh) = ghost {
        let defs: Vec<&FurnitureDef> = gh
            .pieces
            .iter()
            .filter_map(|p| content.furniture_def(&p.def))
            .collect();
        let shows_light = defs.iter().any(|d| is_light(d) || d.seats > 0);
        if shows_light && !gh.floor() {
            let (lights, seats) = lights_and_seats(g, content, gh);
            for l in &lights {
                let c = vec2(l.c.0 + 0.5, l.c.1 + 0.5) * TILE;
                let rad = (l.r + 0.5) * TILE;
                if l.held {
                    glow_circle(c, rad, with_a(t.amber, 0.26));
                    stroke_circle(c, rad, 2.0, with_a(t.amber, 0.55));
                } else {
                    glow_circle(c, rad, with_a(t.amber, 0.14));
                    stroke_circle(c, rad, 1.5, with_a(t.amber, 0.22));
                }
            }
            // seats: warm dot = lit, a faint violet ring = in the dark
            for s in &seats {
                let lit = lights.iter().any(|l| {
                    let (dx, dy) = (s.0 as f32 - l.c.0, s.1 as f32 - l.c.1);
                    (dx * dx + dy * dy).sqrt() <= l.r + 0.5
                });
                let c = vec2(s.0 as f32 + 0.5, s.1 as f32 + 0.78) * TILE;
                if lit {
                    fill_circle(c, 5.0, with_a(t.amber, 0.9));
                } else {
                    stroke_circle(c, 6.0, 2.0, with_a(t.violet_400, 0.8));
                }
            }
        }
        // beauty reach of a decor piece
        if let (Some(d), Some(p)) = (defs.first(), gh.pieces.first())
            && d.beauty > 0.0
            && !gh.floor()
            && !gh.moving
        {
            let r = tiles_rect(
                p.tile.0 - BEAUTY_REACH,
                p.tile.1 - BEAUTY_REACH,
                p.size.0 + 2 * BEAUTY_REACH as u32,
                p.size.1 + 2 * BEAUTY_REACH as u32,
            );
            fill_rrect(r, TILE * 0.6, with_a(t.violet_400, 0.07));
            stroke_rrect(r, TILE * 0.6, 2.0, with_a(t.violet_400, 0.35));
        }
        let c = if gh.ok { t.emerald_400 } else { t.red_400 };
        for p in &gh.pieces {
            let r = tiles_rect(p.tile.0, p.tile.1, p.size.0, p.size.1);
            draw_rectangle(
                r.x,
                r.y,
                r.w,
                r.h,
                with_a(c, if p.floor { 0.35 } else { 0.18 }),
            );
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 3.0, with_a(c, 0.85));
        }
    }
    // the piece under the mouse (and the chairs that come along)
    if let Some(uid) = st.hover {
        let busy = super::build::group_in_use(&g.world, content, uid).is_some();
        let col = if busy { t.red_400 } else { t.indigo_300 };
        for (i, u) in g.world.move_group(content, uid).into_iter().enumerate() {
            if let Some(p) = g.world.furniture_by_uid(u)
                && let Some(d) = content.furniture_def(&p.def)
            {
                let (w, h) = sim::grid::rotated_size(d, p.rot);
                let r = tiles_rect(p.x, p.y, w, h);
                draw_rectangle(
                    r.x,
                    r.y,
                    r.w,
                    r.h,
                    with_a(col, if i == 0 { 0.14 } else { 0.08 }),
                );
                draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, with_a(col, 0.7));
            }
        }
    }
    if st.demolish {
        let m = g
            .cam
            .screen_to_world(Vec2::from(crate::platform::pointer()));
        if let Some(uid) = super::build::piece_at(&g.world, content, m)
            && let Some(p) = g.world.furniture_by_uid(uid)
            && let Some(def) = content.furniture_def(&p.def)
        {
            let (w, h) = sim::grid::rotated_size(def, p.rot);
            let r = tiles_rect(p.x, p.y, w, h);
            draw_rectangle(r.x, r.y, r.w, r.h, with_a(t.red_400, 0.22));
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 3.0, with_a(t.red_400, 0.9));
        }
    }
}

/// World px → UI units.
fn to_ui(g: &GameScreen, ui: &Ui, p: Vec2) -> Vec2 {
    let sc = vec2(screen_width(), screen_height()) / 2.0;
    ((p - g.cam.center) * g.cam.zoom + sc) / ui.scale()
}

/// `+0,6` / `−1,2` / `±0` (coziness points).
pub fn delta_text(loc: &Locale, d: f32) -> String {
    if d.abs() < 0.05 {
        "±0".to_string()
    } else if d > 0.0 {
        format!("+{}", format_decimal(loc.lang, d, 1))
    } else {
        format_decimal(loc.lang, d, 1)
    }
}

fn delta_color(t: &Theme, d: f32) -> Color {
    if d >= 0.05 {
        t.emerald_400
    } else if d <= -0.05 {
        t.red_400
    } else {
        t.text_dim
    }
}

/// A small pill centred above `at` (UI units): an icon and a text. Near the top HUD it goes
/// under the piece instead (`below` = the piece's foot).
fn pill(ui: &mut Ui, at: Vec2, below: f32, icon: Icon, text: &str, fg: Color, bg: Color) -> Rect {
    let st = ui.theme.key().size(11.5).color(fg);
    let w = ui.fonts.measure(text, &st) + 30.0;
    let s = ui.screen();
    let y = if at.y - 22.0 < 96.0 {
        below + 4.0
    } else {
        at.y - 22.0
    };
    let x = (at.x - w / 2.0).clamp(8.0, (s.w - w - 8.0).max(8.0));
    let r = Rect::new(x, y, w, 20.0);
    fill_rrect(r, 10.0, bg);
    stroke_rrect(r, 10.0, 1.0, Color::new(1.0, 1.0, 1.0, 0.12));
    draw_icon(icon, vec2(r.x + 13.0, r.center_y()), 12.0, fg);
    ui.fonts.draw_in(
        text,
        Rect::new(r.x + 22.0, r.y, w - 26.0, r.h),
        &st,
        Align::Left,
    );
    r
}

fn error_key(e: &str) -> &'static str {
    if e.starts_with("NotEnoughMoney") {
        "ui.error.money"
    } else if e.starts_with("BlocksPath") {
        "ui.error.blocks_path"
    } else if e.starts_with("InUse") {
        "ui.error.in_use"
    } else {
        "ui.error.occupied"
    }
}

/// UI pass (before the panel): the held piece's label, the meter's delta, why a hovered
/// piece can't move.
pub fn labels(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let t = ui.theme.clone();
    let loc = cx.loc;
    let content = cx.content;
    let bg = Color::new(0.09, 0.08, 0.16, 0.9);
    let st = g.ui_state.build.clone();
    // the corners the hall already has (reading nook, window spot…): a soft name tag over
    // each, with what it adds to the coziness
    if !st.holding() {
        for corner in g.world.corners(content) {
            let (cx_t, cy_t) = corner.center;
            let top = to_ui(
                g,
                ui,
                vec2((cx_t as f32 + 0.5) * TILE, (cy_t as f32 - 0.6) * TILE),
            );
            let foot = to_ui(g, ui, vec2(0.0, (cy_t as f32 + 1.0) * TILE)).y;
            let text = format!(
                "{} · {}",
                loc.tr_owned(&corner.name_key),
                delta_text(&loc, corner.bonus)
            );
            pill(ui, top, foot, Icon::Sparkle, &text, t.amber, bg);
        }
    }
    if st.holding() && st.ghost_on && !st.preview.key.is_empty() {
        let tile = if let Some(m) = st.preview.moves.first() {
            Some((m.x, m.y))
        } else {
            st.preview.key.split_once('@').and_then(|(_, r)| {
                let mut it = r.split(',');
                Some((it.next()?.parse().ok()?, it.next()?.parse().ok()?))
            })
        };
        let floor = st
            .selected
            .as_ref()
            .and_then(|id| content.furniture_def(id))
            .is_some_and(|d| d.category == crate::content::BuildCategory::Floors);
        if let Some((x, y)) = tile {
            let w = st
                .preview
                .moves
                .first()
                .and_then(|m| g.world.furniture_by_uid(m.uid))
                .and_then(|p| content.furniture_def(&p.def))
                .or_else(|| {
                    st.selected
                        .as_ref()
                        .and_then(|id| content.furniture_def(id))
                })
                .map(|d| d.size.0)
                .unwrap_or(1);
            let top = to_ui(
                g,
                ui,
                vec2((x as f32 + w as f32 / 2.0) * TILE, (y as f32 - 0.9) * TILE),
            );
            let foot = to_ui(g, ui, vec2(0.0, (y as f32 + 1.0) * TILE)).y;
            if let Some(e) = &st.preview.err {
                pill(
                    ui,
                    top,
                    foot,
                    Icon::Warning,
                    loc.tr(error_key(e)),
                    t.red_400,
                    bg,
                );
            } else if !floor {
                let d = st.preview.delta;
                pill(
                    ui,
                    top,
                    foot,
                    Icon::Heart,
                    &delta_text(&loc, d),
                    delta_color(&t, d),
                    bg,
                );
            }
        }
        // the atmosphere meter shows the change too
        if !floor
            && let Some(m) = g
                .anchors
                .iter()
                .find(|a| a.0 == "hud.atmosphere")
                .map(|a| a.1)
        {
            let d = st.preview.delta;
            let text = delta_text(&loc, d);
            let s = t.key().size(12.0).color(delta_color(&t, d));
            let w = ui.fonts.measure(&text, &s) + 28.0;
            let r = Rect::new(m.right() + 6.0, m.center_y() - 11.0, w, 22.0);
            fill_rrect(r, 11.0, bg);
            stroke_rrect(r, 11.0, 1.0, with_a(delta_color(&t, d), 0.45));
            draw_icon(
                Icon::Heart,
                vec2(r.x + 12.0, r.center_y()),
                12.0,
                delta_color(&t, d),
            );
            ui.fonts.draw_in(
                &text,
                Rect::new(r.x + 21.0, r.y, w - 24.0, r.h),
                &s,
                Align::Left,
            );
        }
    }
    // hovering a placed piece: in use → why; free → "drag to move"
    if let Some(uid) = st.hover
        && let Some(p) = g.world.furniture_by_uid(uid)
        && let Some(d) = content.furniture_def(&p.def)
    {
        let (w, _) = sim::grid::rotated_size(d, p.rot);
        let top = to_ui(
            g,
            ui,
            vec2(
                (p.x as f32 + w as f32 / 2.0) * TILE,
                (p.y as f32 - 0.9) * TILE,
            ),
        );
        let foot = to_ui(g, ui, vec2(0.0, (p.y as f32 + 1.0) * TILE)).y;
        match super::build::group_in_use(&g.world, content, uid) {
            Some(why) => {
                let key = match why {
                    sim::InUseReason::Seated => "bux.in_use.seated",
                    sim::InUseReason::Busy => "bux.in_use.busy",
                };
                pill(ui, top, foot, Icon::Lock, loc.tr(key), t.red_400, bg);
            }
            None => {
                let chairs = g.world.move_group(content, uid).len() > 1;
                let key = if chairs {
                    "bux.hover.move_chairs"
                } else {
                    "bux.hover.move"
                };
                pill(ui, top, foot, Icon::Swap, loc.tr(key), t.text, bg);
            }
        }
    }
}
