//! Build mode (B): category tabs, a catalogue with prices, a ghost that follows the mouse
//! (emerald = fits, red = doesn't), R rotate, X demolish, Ctrl+Z undo. One click per action,
//! with refunds and undo instead of confirm popups.

use std::rc::Rc;

use macroquad::prelude::*;

use super::world_view::sim;
use super::{Ctx, GameScreen};
use crate::content::{BuildCategory, Content};
use crate::i18n::format_money;
use crate::map::TILE;
use crate::render::{FurnitureState, RenderQueue, draw_furniture};
use crate::settings::Locale;
use crate::ui::draw::{fill_rrect, stroke_rrect, with_a};
use crate::ui::{Align, Button, Glass, Icon, RectExt, Tab, Tone, Ui};
use sim::Command;

#[derive(Debug, Clone, Default)]
pub struct BuildState {
    /// 0 = all, then `BuildCategory::ALL`.
    pub tab: usize,
    pub selected: Option<String>,
    pub rot: u8,
    pub demolish: bool,
}

impl BuildState {
    pub fn active(&self) -> bool {
        self.selected.is_some() || self.demolish
    }
    pub fn cancel(&mut self) {
        self.selected = None;
        self.demolish = false;
    }
}

/// The piece under the mouse.
#[derive(Debug, Clone)]
pub struct Ghost {
    pub sprite: String,
    pub tile: (i32, i32),
    pub size: (u32, u32),
    pub rot: u8,
    pub ok: bool,
    pub floor: bool,
}

impl Ghost {
    pub fn origin(&self) -> Vec2 {
        vec2(self.tile.0 as f32, self.tile.1 as f32) * TILE
    }

    /// The ghost piece itself, y-sorted with the room.
    pub fn push(&self, q: &mut RenderQueue, t: f32) {
        if self.floor {
            return;
        }
        let sprite = self.sprite.clone();
        let o = self.origin();
        let size = (self.size.0 as u8, self.size.1 as u8);
        let facing = sim::Facing::from_rot(self.rot);
        let st = FurnitureState {
            active: true,
            t,
            facing: super::world_view::facing(facing),
            ..Default::default()
        };
        q.push_custom(o.y + self.size.1 as f32 * TILE + 0.5, move || {
            draw_furniture(&sprite, o, size, &st)
        });
    }
}

pub fn ghost(g: &GameScreen, content: &Content, mouse: Vec2, over_ui: bool) -> Option<Ghost> {
    if g.panel != Some(super::Panel::Build) || over_ui {
        return None;
    }
    let st = &g.ui_state.build;
    let id = st.selected.as_ref()?;
    let def = content.furniture_def(id)?;
    let size = sim::grid::rotated_size(def, st.rot);
    // centre the footprint on the mouse
    let tx = (mouse.x / TILE - size.0 as f32 / 2.0 + 0.5).floor() as i32;
    let ty = (mouse.y / TILE - size.1 as f32 / 2.0 + 0.5).floor() as i32;
    let ok = sim::grid::can_place(g.world.grid(), def, tx, ty, st.rot, None).is_ok()
        && g.world.money() >= def.price as i64;
    Some(Ghost {
        sprite: def.sprite.clone(),
        tile: (tx, ty),
        size,
        rot: st.rot,
        ok,
        floor: def.category == BuildCategory::Floors,
    })
}

/// Footprint tint over the lit world, demolish highlight.
pub fn draw_world_overlay(g: &GameScreen, content: &Content, ghost: Option<&Ghost>) {
    let t = crate::ui::Theme::SHIFTELLA;
    if let Some(gh) = ghost {
        let c = if gh.ok { t.emerald_400 } else { t.red_400 };
        let r = Rect::new(
            gh.tile.0 as f32 * TILE,
            gh.tile.1 as f32 * TILE,
            gh.size.0 as f32 * TILE,
            gh.size.1 as f32 * TILE,
        );
        draw_rectangle(
            r.x,
            r.y,
            r.w,
            r.h,
            with_a(c, if gh.floor { 0.35 } else { 0.18 }),
        );
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 3.0, with_a(c, 0.85));
    }
    if g.ui_state.build.demolish {
        let m = g.cam.screen_to_world(Vec2::from(mouse_position()));
        let (tx, ty) = ((m.x / TILE).floor() as i32, (m.y / TILE).floor() as i32);
        if let Some(uid) = g.world.grid().object_at(tx, ty)
            && let Some(p) = g.world.furniture_by_uid(uid)
            && let Some(def) = content.furniture_def(&p.def)
        {
            let (w, h) = sim::grid::rotated_size(def, p.rot);
            let r = Rect::new(
                p.x as f32 * TILE,
                p.y as f32 * TILE,
                w as f32 * TILE,
                h as f32 * TILE,
            );
            draw_rectangle(r.x, r.y, r.w, r.h, with_a(t.red_400, 0.22));
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 3.0, with_a(t.red_400, 0.9));
        }
    }
}

pub fn click(
    g: &mut GameScreen,
    _ui: &mut Ui,
    content: &Rc<Content>,
    ghost: Option<&Ghost>,
    mouse: Vec2,
    loc: &Locale,
) {
    if let Some(gh) = ghost {
        let id = g.ui_state.build.selected.clone().unwrap_or_default();
        let cmd = if gh.floor {
            Command::SetFloor {
                x: gh.tile.0,
                y: gh.tile.1,
                floor: id,
            }
        } else {
            Command::PlaceFurniture {
                def: id,
                x: gh.tile.0,
                y: gh.tile.1,
                rot: gh.rot,
            }
        };
        if let Err(e) = g.command(content, cmd) {
            super::panels::command_error(g, loc, &e);
        }
        return;
    }
    if g.ui_state.build.demolish {
        let (tx, ty) = (
            (mouse.x / TILE).floor() as i32,
            (mouse.y / TILE).floor() as i32,
        );
        if let Some(uid) = g.world.grid().object_at(tx, ty) {
            let refund = g
                .world
                .furniture_by_uid(uid)
                .and_then(|p| content.furniture_def(&p.def))
                .map(|d| d.price as i64 / 2)
                .unwrap_or(0);
            match g.command(content, Command::RemoveFurniture { uid }) {
                Ok(()) => g.toast(
                    crate::ui::ToastKind::Info,
                    Icon::Undo,
                    loc.trf(
                        "ui.build.refund",
                        &[("money", &format_money(loc.lang, refund))],
                    ),
                    Some(loc.tr_owned("ui.build.undo_hint")),
                ),
                Err(e) => super::panels::command_error(g, loc, &e),
            }
        }
    }
}

pub fn hotkeys(g: &mut GameScreen, ui: &mut Ui, content: &Rc<Content>) {
    if ui.key(KeyCode::R) {
        g.ui_state.build.rot = (g.ui_state.build.rot + 1) % 4;
    }
    if ui.key(KeyCode::X) {
        let st = &mut g.ui_state.build;
        st.demolish = !st.demolish;
        st.selected = None;
    }
    if ui.undo() {
        let r = g.world.undo(content).map_err(|e| format!("{e:?}"));
        g.drain_and_handle(content, None);
        if let Ok(()) = r {
            g.notices.push(super::Notice::Toast {
                kind: crate::ui::ToastKind::Info,
                icon: Icon::Undo,
                title: String::from("↶"),
                body: None,
            })
        }
    }
    if is_mouse_button_pressed(MouseButton::Right) && !ui.wants_mouse() {
        g.ui_state.build.cancel();
    }
}

fn cat_key(c: BuildCategory) -> &'static str {
    match c {
        BuildCategory::Furniture => "ui.build.tab.furniture",
        BuildCategory::Appliances => "ui.build.tab.appliances",
        BuildCategory::Counters => "ui.build.tab.counters",
        BuildCategory::Decor => "ui.build.tab.decor",
        BuildCategory::Floors => "ui.build.tab.floors",
        BuildCategory::Structure => "ui.build.tab.structure",
    }
}

pub fn panel(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let t = ui.theme.clone();
    let loc = cx.loc;
    let content = cx.content;
    let s = ui.screen();
    let h = 214.0;
    let w = (s.w - 24.0).min(980.0);
    let pr = Rect::new((s.w - w) / 2.0, s.h - 60.0 - h, w, h);
    g.anchors.push(("panel", pr));
    ui.glass(pr, &Glass::panel());
    let hint = loc.tr("ui.build.hint");
    let (body, close) = ui.panel_header(pr, loc.tr("ui.build.title"), Some("B"), Some(hint));
    if close {
        g.panel = None;
        g.ui_state.build.cancel();
        return;
    }
    let inner = body.inset_xy(10.0, 8.0);
    let (tabs_r, rest) = inner.split_top(28.0, 8.0);
    let mut labels = vec![Tab::new(loc.tr("ui.build.tab.all"))];
    for c in BuildCategory::ALL {
        labels.push(Tab::new(loc.tr(cat_key(c))));
    }
    // the hall's decor theme (design §11.2) is the last tab
    labels.push(Tab::new(loc.tr("ui.theme.tab")));
    let theme_tab = labels.len() - 1;
    // demolish / undo on the right of the tab row, sized to their labels
    let demolish = Button::new(loc.tr("ui.build.demolish"))
        .icon(Icon::Close)
        .shortcut("X")
        .selected(g.ui_state.build.demolish);
    let undo = Button::new(loc.tr("ui.common.undo"))
        .ghost()
        .icon(Icon::Undo)
        .disabled(!g.world.can_undo())
        .tooltip("Ctrl+Z");
    let (dw, uw) = (ui.button_width(&demolish), ui.button_width(&undo));
    let (tools, tabs_r) = tabs_r.split_right(dw + uw + 6.0, 10.0);
    ui.tabs("build_tabs", tabs_r, &labels, &mut g.ui_state.build.tab);
    let (a, b) = tools.split_left(dw, 6.0);
    if ui.button("demolish", a, &demolish).clicked {
        let st = &mut g.ui_state.build;
        st.demolish = !st.demolish;
        st.selected = None;
    }
    if ui.button("undo", b, &undo).clicked {
        let _ = g.world.undo(content);
        g.drain_and_handle(content, None);
    }

    if g.ui_state.build.tab == theme_tab {
        g.ui_state.build.selected = None;
        g.ui_state.build.demolish = false;
        themes(g, ui, cx, rest);
        return;
    }
    let buildable = g.world.buildable(content);
    let defs: Vec<_> = content
        .furniture
        .iter()
        .filter(|d| {
            g.ui_state.build.tab == 0
                || BuildCategory::ALL.get(g.ui_state.build.tab - 1) == Some(&d.category)
        })
        .filter(|d| d.unlock_flag.is_none() || buildable.contains(&d.id))
        .collect();
    let card_w = 112.0;
    let card_h = rest.h - 8.0;
    let total_w = defs.len() as f32 * (card_w + 8.0);
    // horizontal scroll by wheel
    let id = ui.id("build_scroll_x");
    let wy = if ui.hovering(rest) {
        ui.wheel().y + ui.wheel().x
    } else {
        0.0
    };
    let off = {
        let a = ui.anim(id);
        a.v = (a.v - wy * 40.0).clamp(0.0, (total_w - rest.w).max(0.0));
        a.v
    };
    ui.push_clip(rest);
    for (i, d) in defs.iter().enumerate() {
        let r = Rect::new(
            rest.x + i as f32 * (card_w + 8.0) - off,
            rest.y,
            card_w,
            card_h,
        );
        if r.right() < rest.x || r.x > rest.right() {
            continue;
        }
        let unlocked = buildable.contains(&d.id);
        let afford = g.world.money() >= d.price as i64;
        let sel = g.ui_state.build.selected.as_deref() == Some(d.id.as_str());
        let resp = ui.interact(ui.id(("bcard", d.id.as_str())), r);
        fill_rrect(
            r,
            10.0,
            if sel {
                with_a(t.indigo, 0.35)
            } else if resp.hovered {
                Color::new(1.0, 1.0, 1.0, 0.07)
            } else {
                Color::new(1.0, 1.0, 1.0, 0.035)
            },
        );
        if sel {
            stroke_rrect(r, 10.0, 1.0, t.indigo_300);
        }
        // preview
        let prev = Rect::new(r.x + 8.0, r.y + 6.0, r.w - 16.0, r.h - 52.0);
        draw_preview(&d.sprite, d.size, prev, ui.time() as f32, unlocked);
        let name = loc.tr_owned(&d.name_key());
        ui.fonts.draw_in(
            &name,
            Rect::new(r.x + 6.0, r.bottom() - 42.0, r.w - 12.0, 16.0),
            &t.small_medium()
                .color(if unlocked { t.text } else { t.text_disabled }),
            Align::Center,
        );
        let price = if d.price == 0 {
            loc.tr_owned("ui.build.free")
        } else {
            format_money(loc.lang, d.price as i64)
        };
        ui.fonts.draw_in(
            &price,
            Rect::new(r.x + 6.0, r.bottom() - 24.0, r.w - 12.0, 16.0),
            &t.key().color(if !afford { t.red_400 } else { t.amber }),
            Align::Center,
        );
        let mut tip = loc.tr_owned(&d.desc_key());
        if !unlocked {
            tip = loc.trf("ui.build.locked", &[("night", &d.unlock_night.to_string())]);
        } else {
            tip.push_str(&format!(
                "\n{}",
                loc.trf(
                    "ui.build.size",
                    &[("w", &d.size.0.to_string()), ("h", &d.size.1.to_string())]
                )
            ));
            if d.unique {
                tip.push_str(&format!(" · {}", loc.tr("ui.build.unique")));
            }
        }
        ui.tooltip(&resp, &tip);
        if resp.clicked && unlocked {
            let st = &mut g.ui_state.build;
            st.demolish = false;
            st.selected = if sel { None } else { Some(d.id.clone()) };
        }
        if !unlocked {
            crate::ui::draw_icon(
                Icon::Lock,
                vec2(r.right() - 14.0, r.y + 14.0),
                14.0,
                t.text_dim,
            );
        }
    }
    ui.pop_clip();
    let _ = Tone::Neutral;
}

/// A furniture sprite scaled into a UI rect (UI camera is active).
fn draw_preview(sprite: &str, size: (u32, u32), r: Rect, t: f32, unlocked: bool) {
    let (w, h) = (size.0.max(1) as f32, size.1.max(1) as f32);
    // sprites are drawn in world px with height above the footprint; fit ~ (w × (h+1.4)) tiles
    let k = (r.w / (w * TILE)).min(r.h / ((h + 1.4) * TILE)).min(0.7);
    let origin = vec2(r.center().x / k - w * TILE / 2.0, r.bottom() / k - h * TILE);
    let st = FurnitureState {
        active: true,
        t,
        ..Default::default()
    };
    push_scale(k);
    draw_furniture(sprite, origin, (w as u8, h as u8), &st);
    pop_scale();
    if !unlocked {
        draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.05, 0.04, 0.1, 0.55));
    }
}

fn push_scale(k: f32) {
    let gl = unsafe { get_internal_gl() };
    gl.quad_gl
        .push_model_matrix(Mat4::from_scale(vec3(k, k, 1.0)));
}

fn pop_scale() {
    let gl = unsafe { get_internal_gl() };
    gl.quad_gl.pop_model_matrix();
}

/// Build → Theme: a card per decor theme with a little preview of its wall, floor and a
/// table; buy once, switch for free in the Day phase.
fn themes(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx, rest: Rect) {
    use crate::render::{DecorTheme, ThemeStyle, with_decor};
    let t = ui.theme.clone();
    let loc = cx.loc;
    let content = cx.content;
    let day = g.world.phase() == sim::Phase::Day;
    let current = g.world.theme(content).to_string();
    let season = super::scene_fx::season_of(&g.world, content).1;
    let defs: Vec<_> = content.handson.decor.themes.clone();
    let card_w = ((rest.w - 8.0 * (defs.len() as f32 - 1.0)) / defs.len().max(1) as f32).min(190.0);
    let card_h = rest.h - 6.0;
    let mut x = rest.x;
    for d in &defs {
        let r = Rect::new(x, rest.y, card_w, card_h);
        x += card_w + 8.0;
        let owned = g.world.theme_owned(content, &d.id);
        let unlocked = owned || g.world.themes_unlocked().contains(&d.id);
        let on = current == d.id;
        let resp = ui.interact(ui.id(("theme_card", d.id.as_str())), r);
        fill_rrect(
            r,
            10.0,
            if on {
                with_a(t.indigo, 0.3)
            } else if resp.hovered {
                Color::new(1.0, 1.0, 1.0, 0.07)
            } else {
                Color::new(1.0, 1.0, 1.0, 0.035)
            },
        );
        if on {
            stroke_rrect(r, 10.0, 1.0, t.indigo_300);
        }
        // the preview: two tiles of wall, a floor, a table and a lamp in the theme's look
        let prev = Rect::new(r.x + 8.0, r.y + 8.0, r.w - 16.0, card_h - 70.0);
        let style = DecorTheme::from_id(&d.id)
            .map(|th| th.style(season))
            .unwrap_or(ThemeStyle::GRANDMA);
        fill_rrect(prev, 8.0, Color::new(0.05, 0.04, 0.1, 0.6));
        ui.push_clip(prev);
        with_decor(style, || {
            draw_theme_preview(content, prev, ui.time() as f32)
        });
        ui.pop_clip();
        if !unlocked {
            draw_rectangle(
                prev.x,
                prev.y,
                prev.w,
                prev.h,
                Color::new(0.05, 0.04, 0.1, 0.6),
            );
            crate::ui::draw_icon(Icon::Lock, prev.center(), 18.0, t.text_dim);
        }
        let name = loc.tr_owned(&d.name_key());
        ui.fonts.draw_in(
            &name,
            Rect::new(r.x + 8.0, prev.bottom() + 4.0, r.w - 16.0, 18.0),
            &t.small_medium()
                .color(if unlocked { t.text } else { t.text_disabled }),
            Align::Center,
        );
        ui.tooltip(&resp, &loc.tr_owned(&d.desc_key()));
        let br = Rect::new(r.x + 8.0, r.bottom() - 34.0, r.w - 16.0, 26.0);
        if on {
            ui.fonts.draw_in(
                &format!("✓ {}", loc.tr("ui.theme.owned")),
                br,
                &t.key().color(t.emerald_400),
                Align::Center,
            );
        } else if owned {
            let b = Button::new(loc.tr("ui.theme.apply"))
                .secondary()
                .disabled(!day)
                .tooltip(if day { "" } else { loc.tr("ui.theme.day_only") });
            if ui.button(("theme_use", d.id.as_str()), br, &b).clicked {
                match g.command(content, Command::SetTheme { id: d.id.clone() }) {
                    Ok(()) => g.hands.tut.bump("theme_set"),
                    Err(e) => super::panels::command_error(g, &loc, &e),
                }
            }
        } else if unlocked {
            let price = format_money(loc.lang, d.price as i64);
            let label = loc.trf("ui.theme.buy", &[("price", &price)]);
            let b = Button::new(&label)
                .primary()
                .disabled(g.world.money() < d.price as i64);
            if ui.button(("theme_buy", d.id.as_str()), br, &b).clicked {
                match g.command(content, Command::BuyTheme { id: d.id.clone() }) {
                    Ok(()) => {
                        g.hands.tut.bump("theme_bought");
                        if day {
                            let _ = g.command(content, Command::SetTheme { id: d.id.clone() });
                        }
                    }
                    Err(e) => super::panels::command_error(g, &loc, &e),
                }
            }
        } else {
            ui.fonts.draw_in(
                loc.tr("ui.theme.locked"),
                br,
                &t.small().color(t.text_disabled),
                Align::Center,
            );
        }
    }
}

/// A tiny room in the current decor: the lower wall with a window, the floor, a table with a
/// chair, a lamp and a plant (cropped to `r`; the caller clips).
fn draw_theme_preview(content: &Content, r: Rect, t: f32) {
    use crate::render::{WallKind, WallMask, draw_floor, draw_wall};
    let sprite = |id: &str| {
        content
            .furniture_def(id)
            .map(|d| d.sprite.clone())
            .unwrap_or_else(|| id.to_string())
    };
    // 5 tiles wide; the view starts a little above the wall's foot (the wainscot and the
    // pattern show), the furniture stands right in front of it
    let k = r.w / (5.0 * TILE);
    let origin = vec2(r.x, r.y + 0.62 * TILE * k);
    let gl = unsafe { get_internal_gl() };
    gl.quad_gl.push_model_matrix(
        Mat4::from_translation(vec3(origin.x, origin.y, 0.0)) * Mat4::from_scale(vec3(k, k, 1.0)),
    );
    let is_wall = |x: i32, y: i32| y == 0 && (-1..6).contains(&x);
    let is_floor = |x: i32, y: i32| (1..4).contains(&y) && (-1..6).contains(&x);
    for y in 1..4 {
        for x in 0..5 {
            draw_floor("floor_wood", vec2(x as f32, y as f32 - 1.0));
        }
    }
    for x in 0..5 {
        let m = WallMask::compute(x, 0, is_wall, is_floor);
        let kind = if x == 2 {
            WallKind::Window
        } else {
            WallKind::Plain
        };
        draw_wall(vec2(x as f32, -1.0), m, kind);
    }
    let st = FurnitureState {
        active: true,
        t,
        ..Default::default()
    };
    let chair = FurnitureState {
        facing: crate::render::Facing::Right,
        ..st
    };
    draw_furniture(&sprite("plant_tall"), vec2(0.1, -0.35) * TILE, (1, 1), &st);
    draw_furniture(&sprite("chair"), vec2(1.1, -0.05) * TILE, (1, 1), &chair);
    draw_furniture(&sprite("table_round"), vec2(2.0, -0.05) * TILE, (1, 1), &st);
    draw_furniture(&sprite("floor_lamp"), vec2(3.2, -0.4) * TILE, (1, 1), &st);
    draw_furniture(&sprite("armchair"), vec2(4.0, -0.05) * TILE, (1, 1), &st);
    gl.quad_gl.pop_model_matrix();
}
