//! Build mode (B): a compact panel (category tabs + a scrollable card row) at the bottom,
//! a ghost that follows the mouse (emerald = fits, red = doesn't), R rotate, X demolish,
//! Ctrl+Z undo. Placed pieces move by dragging (a table brings its chairs; a click without
//! moving picks a piece up until the next click). While something is held the coziness meter
//! shows the change it makes and the hall shows light pools and beauty reach. The camera
//! keeps the hall above the panel. One click per action, refunds and undo, no confirm popups.
//!
//! Cards: `build_cards.rs` · world overlay + held-piece labels: `build_overlay.rs` ·
//! decor themes: `build_themes.rs`.

use std::rc::Rc;

use macroquad::prelude::*;

use super::world_view::sim;
use super::{Ctx, GameScreen};
use crate::camera::WorldCamera;
use crate::content::{BuildCategory, Content};
use crate::i18n::format_money;
use crate::map::TILE;
use crate::render::{FurnitureState, RenderQueue, draw_furniture};
use crate::settings::Locale;
use crate::ui::draw::{fill_circle, fill_rrect, with_a};
use crate::ui::{Align, Button, Glass, Icon, IconButton, RectExt, Ui, UiSound};
use sim::{Command, PieceMove};

#[derive(Debug, Clone, Default)]
pub struct BuildState {
    /// 0 = all, then [`TABS`], then the decor theme.
    pub tab: usize,
    pub selected: Option<String>,
    pub rot: u8,
    pub demolish: bool,
    /// A placed piece (and its chairs) being moved.
    pub drag: Option<Drag>,
    /// The mouse went down on a placed piece; a drag starts once it moves.
    pub press: Option<Press>,
    /// The held thing's coziness change and whether it fits (recomputed when it moves).
    pub preview: Preview,
    /// The placed piece under the mouse while nothing is held.
    pub hover: Option<u32>,
    /// Screenshot runs: a fixed world point instead of the mouse.
    pub fake_mouse: Option<Vec2>,
    /// Zoom before the build camera framed the hall (restored when the panel closes).
    pub zoom_before: Option<f32>,
    /// The zoom the build camera is easing to.
    pub zoom_goal: Option<f32>,
    /// Top of the panel in screen px (the camera keeps the hall above it).
    pub panel_top_px: f32,
    /// A ghost follows the mouse this frame (not over the UI).
    pub ghost_on: bool,
}

/// A placed piece being moved.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drag {
    pub uid: u32,
    /// Mouse tile − the piece's top-left at pickup (so it doesn't jump under the cursor).
    pub grab: (i32, i32),
    /// Quarter turns (R) of the whole group.
    pub turns: u8,
    /// Picked up with a click: it follows the mouse until the next click.
    pub sticky: bool,
}

/// The mouse went down on a placed piece.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Press {
    pub uid: u32,
    /// Screen position (px) of the press.
    pub at: Vec2,
    pub tile: (i32, i32),
}

/// Cached result for the held thing (key = what is held where).
#[derive(Debug, Clone, Default)]
pub struct Preview {
    pub key: String,
    /// Change of the hall's coziness (0–100) if it were put down here.
    pub delta: f32,
    /// Why it can't go here (a `CommandError` debug string), `None` = fits.
    pub err: Option<String>,
    /// The group move the drag would make.
    pub moves: Vec<PieceMove>,
}

/// Catalogue tabs after "All" (structure editing isn't possible, so it has no tab).
pub const TABS: [BuildCategory; 5] = [
    BuildCategory::Furniture,
    BuildCategory::Appliances,
    BuildCategory::Counters,
    BuildCategory::Decor,
    BuildCategory::Floors,
];

/// Index of the decor-theme tab (after "All" and [`TABS`]).
pub const THEME_TAB: usize = TABS.len() + 1;

impl BuildState {
    pub fn active(&self) -> bool {
        self.selected.is_some() || self.demolish || self.drag.is_some()
    }
    pub fn cancel(&mut self) {
        self.selected = None;
        self.demolish = false;
        self.drag = None;
        self.press = None;
    }
    /// Something follows the mouse (a new piece or a moved one).
    pub fn holding(&self) -> bool {
        self.selected.is_some() || self.drag.is_some()
    }
}

/// One piece of the ghost.
#[derive(Debug, Clone)]
pub struct GhostPiece {
    pub def: String,
    pub sprite: String,
    pub tile: (i32, i32),
    pub size: (u32, u32),
    pub facing: sim::Facing,
    pub floor: bool,
}

/// What follows the mouse: a new piece or a moved group.
#[derive(Debug, Clone)]
pub struct Ghost {
    pub pieces: Vec<GhostPiece>,
    pub ok: bool,
    /// A drag (moving placed pieces), not a new one.
    pub moving: bool,
}

impl Ghost {
    pub fn floor(&self) -> bool {
        self.pieces.first().is_some_and(|p| p.floor)
    }

    /// The ghost pieces themselves, y-sorted with the room.
    pub fn push(&self, q: &mut RenderQueue, t: f32) {
        for p in &self.pieces {
            if p.floor {
                continue;
            }
            let sprite = p.sprite.clone();
            let o = vec2(p.tile.0 as f32, p.tile.1 as f32) * TILE;
            let size = (p.size.0 as u8, p.size.1 as u8);
            let st = FurnitureState {
                active: true,
                t,
                facing: super::world_view::facing(p.facing),
                ..Default::default()
            };
            q.push_custom(o.y + p.size.1 as f32 * TILE + 0.5, move || {
                draw_furniture(&sprite, o, size, &st)
            });
        }
    }
}

pub(super) fn tile_of(p: Vec2) -> (i32, i32) {
    ((p.x / TILE).floor() as i32, (p.y / TILE).floor() as i32)
}

/// The placed piece under a world point: the footprint first, then a tall piece standing
/// just below (its body rises above its tile), then the rug layer (rugs, pendant lamps).
pub(super) fn piece_at(world: &sim::World, content: &Content, p: Vec2) -> Option<u32> {
    let (tx, ty) = tile_of(p);
    let g = world.grid();
    if let Some(uid) = g.object_at(tx, ty) {
        return Some(uid);
    }
    if let Some(uid) = g.object_at(tx, ty + 1)
        && world
            .furniture_by_uid(uid)
            .and_then(|f| content.furniture_def(&f.def))
            .is_some_and(|d| !d.walkable && (d.has_tag("light") || d.has_tag("decor")))
    {
        return Some(uid);
    }
    g.idx(tx, ty).and_then(|i| g.rugs[i])
}

pub(super) fn group_in_use(
    world: &sim::World,
    content: &Content,
    uid: u32,
) -> Option<sim::InUseReason> {
    world
        .move_group(content, uid)
        .into_iter()
        .find_map(|u| world.in_use_reason(u))
}

/// Per frame (before the world is drawn): hover, the held ghost, its cached preview.
pub fn ghost(g: &mut GameScreen, content: &Content, mouse: Vec2, over_ui: bool) -> Option<Ghost> {
    let gh = ghost_inner(g, content, mouse, over_ui);
    g.ui_state.build.ghost_on = gh.is_some();
    gh
}

fn ghost_inner(g: &mut GameScreen, content: &Content, mouse: Vec2, over_ui: bool) -> Option<Ghost> {
    g.scene.hidden_furniture.clear();
    if g.panel != Some(super::Panel::Build) {
        let st = &mut g.ui_state.build;
        st.hover = None;
        st.drag = None;
        st.press = None;
        return None;
    }
    let (mouse, over_ui) = match g.ui_state.build.fake_mouse {
        Some(m) if g.automation => (m, false),
        _ => (mouse, over_ui),
    };
    let mt = tile_of(mouse);
    let hover = {
        let st = &g.ui_state.build;
        (!over_ui && !st.holding() && !st.demolish && st.press.is_none() && st.tab != THEME_TAB)
            .then(|| piece_at(&g.world, content, mouse))
            .flatten()
    };
    g.ui_state.build.hover = hover;
    if let Some(d) = g.ui_state.build.drag {
        // the moved pieces leave their old spot while held
        g.scene.hidden_furniture = g.world.move_group(content, d.uid);
    }
    if over_ui {
        return None;
    }
    if let Some(d) = g.ui_state.build.drag {
        return drag_ghost(g, content, d, mt);
    }
    let st = &g.ui_state.build;
    let id = st.selected.clone()?;
    let def = content.furniture_def(&id)?;
    let size = sim::grid::rotated_size(def, st.rot);
    let rot = st.rot;
    // centre the footprint on the mouse
    let tx = (mouse.x / TILE - size.0 as f32 / 2.0 + 0.5).floor() as i32;
    let ty = (mouse.y / TILE - size.1 as f32 / 2.0 + 0.5).floor() as i32;
    let floor = def.category == BuildCategory::Floors;
    let key = format!("p{id}@{tx},{ty},{rot}");
    if g.ui_state.build.preview.key != key {
        let fit = sim::grid::can_place(g.world.grid(), def, tx, ty, rot, None)
            .err()
            .map(|e| format!("{e:?}"));
        let money = (g.world.money() < def.price as i64).then(|| "NotEnoughMoney".to_string());
        g.ui_state.build.preview = Preview {
            key,
            delta: g.world.coziness_preview(content, &id, tx, ty, rot),
            err: fit.or(money),
            moves: Vec::new(),
        };
    }
    Some(Ghost {
        pieces: vec![GhostPiece {
            def: id.clone(),
            sprite: def.sprite.clone(),
            tile: (tx, ty),
            size,
            facing: sim::Facing::from_rot(rot),
            floor,
        }],
        ok: g.ui_state.build.preview.err.is_none(),
        moving: false,
    })
}

fn drag_ghost(g: &mut GameScreen, content: &Content, d: Drag, mt: (i32, i32)) -> Option<Ghost> {
    let to = (mt.0 - d.grab.0, mt.1 - d.grab.1);
    let moves = g.world.group_moves(content, d.uid, to, d.turns);
    let key = format!("m{moves:?}");
    if g.ui_state.build.preview.key != key {
        g.ui_state.build.preview = Preview {
            key,
            delta: g.world.coziness_preview_moves(content, &moves),
            err: g
                .world
                .can_move_pieces(content, &moves)
                .err()
                .map(|e| format!("{e:?}")),
            moves: moves.clone(),
        };
    }
    let table = moves.first().and_then(|m| {
        let p = g.world.furniture_by_uid(m.uid)?;
        let def = content.furniture_def(&p.def)?;
        let s = sim::grid::rotated_size(def, m.rot);
        def.has_tag("table")
            .then_some((m.x as f32 + s.0 as f32 / 2.0, m.y as f32 + s.1 as f32 / 2.0))
    });
    let pieces = moves
        .iter()
        .enumerate()
        .filter_map(|(i, m)| {
            let p = g.world.furniture_by_uid(m.uid)?;
            let def = content.furniture_def(&p.def)?;
            let size = sim::grid::rotated_size(def, m.rot);
            // chairs keep facing their table
            let facing = match table {
                Some((tx, ty)) if i > 0 => sim::Facing::from_delta(
                    tx - (m.x as f32 + size.0 as f32 / 2.0),
                    ty - (m.y as f32 + size.1 as f32 / 2.0),
                ),
                _ => sim::Facing::from_rot(m.rot),
            };
            Some(GhostPiece {
                def: def.id.clone(),
                sprite: def.sprite.clone(),
                tile: (m.x, m.y),
                size,
                facing,
                floor: false,
            })
        })
        .collect::<Vec<_>>();
    if pieces.is_empty() {
        return None;
    }
    Some(Ghost {
        pieces,
        ok: g.ui_state.build.preview.err.is_none(),
        moving: true,
    })
}

/// Footprints, light pools, beauty reach and hover outlines over the lit world.
pub fn draw_world_overlay(g: &GameScreen, content: &Content, ghost: Option<&Ghost>) {
    super::build_overlay::world(g, content, ghost);
}

/// A left press in the world (not over the UI).
pub fn click(
    g: &mut GameScreen,
    ui: &mut Ui,
    content: &Rc<Content>,
    ghost: Option<&Ghost>,
    mouse: Vec2,
    loc: &Locale,
) {
    let mouse = match g.ui_state.build.fake_mouse {
        Some(m) if g.automation => m,
        _ => mouse,
    };
    if let Some(d) = g.ui_state.build.drag {
        // a piece picked up with a click goes down with the next one
        if d.sticky {
            drop_drag(g, ui, content, loc, true);
        }
        return;
    }
    if let Some(gh) = ghost {
        let id = g.ui_state.build.selected.clone().unwrap_or_default();
        let (x, y) = gh.pieces[0].tile;
        let cmd = if gh.floor() {
            Command::SetFloor { x, y, floor: id }
        } else {
            Command::PlaceFurniture {
                def: id,
                x,
                y,
                rot: g.ui_state.build.rot,
            }
        };
        match g.command(content, cmd) {
            Ok(()) => ui.sound(UiSound::Toggle),
            Err(e) => super::panels::command_error(g, loc, &e),
        }
        return;
    }
    if g.ui_state.build.demolish {
        let Some(uid) = piece_at(&g.world, content, mouse) else {
            return;
        };
        let refund = g
            .world
            .furniture_by_uid(uid)
            .map(|p| p.paid as i64 / 2)
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
            Err(e) => in_use_toast(g, content, loc, uid, &e),
        }
        return;
    }
    // nothing held: a press on a placed piece can become a drag
    if g.ui_state.build.tab != THEME_TAB
        && let Some(uid) = piece_at(&g.world, content, mouse)
    {
        if group_in_use(&g.world, content, uid).is_some() {
            in_use_toast(g, content, loc, uid, "InUse");
            return;
        }
        g.ui_state.build.press = Some(Press {
            uid,
            at: Vec2::from(crate::platform::pointer()),
            tile: tile_of(mouse),
        });
    }
}

/// "Why can't I move / remove it": the piece's own reason in words.
fn in_use_toast(g: &mut GameScreen, content: &Content, loc: &Locale, uid: u32, e: &str) {
    if !e.starts_with("InUse") {
        super::panels::command_error(g, loc, e);
        return;
    }
    let key = match group_in_use(&g.world, content, uid) {
        Some(sim::InUseReason::Seated) => "bux.in_use.seated",
        _ => "bux.in_use.busy",
    };
    g.toast(
        crate::ui::ToastKind::Warning,
        Icon::Warning,
        loc.tr_owned(key),
        None,
    );
}

fn start_drag(g: &mut GameScreen, content: &Content, ui: &mut Ui, p: Press, sticky: bool) {
    let Some(f) = g.world.furniture_by_uid(p.uid) else {
        return;
    };
    let size = content
        .furniture_def(&f.def)
        .map(|d| sim::grid::rotated_size(d, f.rot))
        .unwrap_or((1, 1));
    let grab = (
        (p.tile.0 - f.x).clamp(0, size.0 as i32 - 1),
        (p.tile.1 - f.y).clamp(0, size.1 as i32 - 1),
    );
    let st = &mut g.ui_state.build;
    st.drag = Some(Drag {
        uid: p.uid,
        grab,
        turns: 0,
        sticky,
    });
    st.preview = Preview::default();
    ui.sound(UiSound::Click);
}

/// Puts the held group down where the ghost is (a sticky one that doesn't fit stays in the
/// hand). A drop on its own spot just ends the drag.
fn drop_drag(g: &mut GameScreen, ui: &mut Ui, content: &Rc<Content>, loc: &Locale, sticky: bool) {
    let Some(d) = g.ui_state.build.drag else {
        return;
    };
    let pv = g.ui_state.build.preview.clone();
    let unchanged = pv.moves.iter().all(|m| {
        g.world
            .furniture_by_uid(m.uid)
            .is_some_and(|p| (p.x, p.y, p.rot) == (m.x, m.y, m.rot % 4))
    });
    if unchanged || pv.moves.first().map(|m| m.uid) != Some(d.uid) {
        g.ui_state.build.drag = None;
        return;
    }
    if let Some(e) = pv.err {
        super::panels::command_error(g, loc, &e);
        if !sticky {
            g.ui_state.build.drag = None;
        }
        return;
    }
    match g.command(content, Command::MoveGroup { moves: pv.moves }) {
        Ok(()) => {
            ui.sound(UiSound::Toggle);
            g.ui_state.build.preview = Preview::default();
        }
        Err(e) => in_use_toast(g, content, loc, d.uid, &e),
    }
    g.ui_state.build.drag = None;
}

/// Per frame in build mode: a press turns into a drag (or a pick-up), a released drag drops.
pub fn frame(g: &mut GameScreen, ui: &mut Ui, content: &Rc<Content>, loc: &Locale, blocked: bool) {
    if g.panel != Some(super::Panel::Build) {
        return;
    }
    let down = is_mouse_button_down(MouseButton::Left);
    if let Some(p) = g.ui_state.build.press {
        let now = Vec2::from(crate::platform::pointer());
        if !down {
            g.ui_state.build.press = None;
            start_drag(g, content, ui, p, true);
        } else if now.distance(p.at) > 5.0 {
            g.ui_state.build.press = None;
            start_drag(g, content, ui, p, false);
        }
        return;
    }
    if let Some(d) = g.ui_state.build.drag
        && !d.sticky
        && !down
    {
        if blocked {
            // let go over the panel: it stays where it was
            g.ui_state.build.drag = None;
        } else {
            drop_drag(g, ui, content, loc, false);
        }
    }
}

pub fn hotkeys(g: &mut GameScreen, ui: &mut Ui, content: &Rc<Content>) {
    if ui.key(KeyCode::R) {
        let st = &mut g.ui_state.build;
        if let Some(d) = &mut st.drag {
            d.turns = (d.turns + 1) % 4;
            // the turned piece stays under the cursor
            d.grab = (0, 0);
        } else {
            st.rot = (st.rot + 1) % 4;
        }
    }
    if ui.key(KeyCode::X) {
        let st = &mut g.ui_state.build;
        st.demolish = !st.demolish;
        st.selected = None;
        st.drag = None;
    }
    if !g.ui_state.build.holding() && ui.key(KeyCode::Tab) {
        let n = THEME_TAB + 1;
        let back = ui.shift();
        let st = &mut g.ui_state.build;
        st.tab = (st.tab + if back { n - 1 } else { 1 }) % n;
    }
    if ui.undo() {
        g.ui_state.build.drag = None;
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

/// Faces for the "wanted" badges (call at the start of a frame, before drawing).
pub fn faces(g: &GameScreen, ui: &mut Ui, content: &Content, loc: &Locale) {
    if g.panel != Some(super::Panel::Build) {
        return;
    }
    for w in g.world.build_wishes(content) {
        let (name, look) = super::build_cards::wisher(g, loc, &w.who);
        if let Some(look) = look {
            ui.register_face(&name, look);
        }
    }
}

// ───────────────────────────── camera ─────────────────────────────

/// The world view that matters for building: the indoor rooms with the back wall.
fn hall_view(g: &GameScreen) -> Rect {
    let rows = g
        .world
        .grid()
        .height
        .min(crate::map::CAFE_LAYOUT.len() as i32) as f32;
    let w = g.world.grid().width as f32;
    Rect::new(-0.3 * TILE, -TILE, (w + 0.6) * TILE, (rows + 1.1) * TILE)
}

/// Replaces the scene's camera clamp: in build mode the hall stays above the panel (on
/// opening it zooms out a little if needed), holding a piece near an edge pans the view.
pub fn camera(g: &mut GameScreen, dt: f32) {
    let (sw, sh) = (screen_width(), screen_height());
    if g.panel != Some(super::Panel::Build) {
        let st = &mut g.ui_state.build;
        st.zoom_goal = None;
        if let Some(z) = st.zoom_before.take() {
            g.cam.zoom = z;
        }
        // under the photo viewfinder its own clamp rules (the frame may reach the café's ends,
        // which the overview clamp would pull back to the middle)
        if !g.photo.is_open() {
            g.scene.clamp_camera(&mut g.cam);
        }
        return;
    }
    // the top HUD floats over the back wall: frame from just under the clock card
    let top = sh * 0.07;
    let panel_top = if g.ui_state.build.panel_top_px > 0.0 {
        g.ui_state.build.panel_top_px
    } else {
        sh * 0.72
    };
    let free = Rect::new(0.0, top, sw, (panel_top - 6.0 - top).max(sh * 0.3));
    let fit = WorldCamera::fit(hall_view(g).size(), free).zoom;
    // frame once the panel has been laid out (its height depends on the UI scale)
    if g.ui_state.build.zoom_before.is_none() && g.ui_state.build.panel_top_px > 0.0 {
        g.ui_state.build.zoom_before = Some(g.cam.zoom);
        // only ever zoom out (a little) to frame the hall; a closer view stays as it is
        if g.cam.zoom > fit {
            g.ui_state.build.zoom_goal = Some(fit.max(g.cam.zoom * 0.65));
        }
    }
    if let Some(goal) = g.ui_state.build.zoom_goal {
        g.cam.zoom += (goal - g.cam.zoom) * (dt * 10.0).min(1.0);
        if (g.cam.zoom - goal).abs() < 0.002 || g.automation {
            g.cam.zoom = goal;
            g.ui_state.build.zoom_goal = None;
        }
    }
    // holding a piece at the edge of the view: pan towards it
    if g.ui_state.build.holding() && !g.automation {
        let m = Vec2::from(crate::platform::pointer());
        let edge = 36.0;
        let mut dir = Vec2::ZERO;
        if m.y > free.bottom() - edge && m.y < panel_top {
            dir.y += 1.0;
        }
        if m.y < top + edge * 0.5 && m.y > 0.0 {
            dir.y -= 1.0;
        }
        if m.x < edge && m.x > 0.0 {
            dir.x -= 1.0;
        }
        if m.x > sw - edge && m.x < sw {
            dir.x += 1.0;
        }
        g.cam.center += dir * 520.0 * dt / g.cam.zoom;
    }
    // clamp: the stage may rise above the panel (the void under it is hidden by the panel)
    let b = g.scene.bounds();
    let z = g.cam.zoom.max(fit * 0.7);
    g.cam.zoom = z;
    let sc = vec2(sw, sh) / 2.0;
    let mid_y = free.center().y;
    // world [lo_w, hi_w] covers screen [lo_s, hi_s]; when it is smaller it sits at mid_s
    let axis = |c: f32, lo_w: f32, hi_w: f32, lo_s: f32, hi_s: f32, mid_s: f32, half: f32| {
        let lo = lo_w + (half - lo_s) / z;
        let hi = hi_w - (hi_s - half) / z;
        if lo > hi {
            (lo_w + hi_w) / 2.0 - (mid_s - half) / z
        } else {
            c.clamp(lo, hi)
        }
    };
    g.cam.center = vec2(
        axis(g.cam.center.x, b.x, b.right(), 0.0, sw, sc.x, sc.x),
        axis(
            g.cam.center.y,
            b.y,
            b.bottom(),
            0.0,
            free.bottom(),
            mid_y,
            sc.y,
        ),
    );
}

// ───────────────────────────── panel ─────────────────────────────

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

/// Panel height in UI units (header row + cards + hint line).
pub const PANEL_H: f32 = 164.0;

pub fn panel(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let t = ui.theme.clone();
    let loc = cx.loc;
    let content = cx.content;
    let s = ui.screen();
    let w = (s.w - 24.0).min(900.0);
    let pr = Rect::new((s.w - w) / 2.0, s.h - 60.0 - PANEL_H, w, PANEL_H);
    g.anchors.push(("panel", pr));
    g.ui_state.build.panel_top_px = pr.y * ui.scale();
    // the held piece's label in the hall and the meter's delta
    super::build_overlay::labels(g, ui, cx);
    ui.glass(pr, &Glass::panel());
    let inner = pr.inset_xy(10.0, 8.0);
    let (head, rest) = inner.split_top(26.0, 6.0);
    let (foot, rest) = rest.split_bottom(14.0, 3.0);

    // header: title + B │ tabs │ demolish · undo · close
    let title = loc.tr("ui.build.title");
    let hs = t.h2();
    let tw = ui.fonts.measure(title, &hs);
    ui.fonts.draw_in(
        title,
        Rect::new(head.x + 2.0, head.y, tw, head.h),
        &hs,
        Align::Left,
    );
    let kr = ui.keycap(
        vec2(head.x + tw + 8.0, head.center_y()),
        &crate::settings::keys::label(crate::settings::keys::Action::Build),
        false,
    );
    let close_r = Rect::new(head.right() - 24.0, head.y + 1.0, 24.0, 24.0);
    if ui
        .icon_button(
            "build_close",
            close_r,
            Icon::Close,
            &IconButton::ghost().size(16.0),
        )
        .clicked
    {
        g.panel = None;
        g.ui_state.build.cancel();
        return;
    }
    let undo_r = Rect::new(close_r.x - 30.0, head.y + 1.0, 26.0, 24.0);
    let undo_tip = format!("{} · Ctrl+Z", loc.tr("ui.common.undo"));
    if ui
        .icon_button(
            "build_undo",
            undo_r,
            Icon::Undo,
            &IconButton::ghost()
                .size(15.0)
                .disabled(!g.world.can_undo())
                .tooltip(&undo_tip),
        )
        .clicked
    {
        g.ui_state.build.drag = None;
        let _ = g.world.undo(content);
        g.drain_and_handle(content, None);
    }
    let demolish = Button::new(loc.tr("ui.build.demolish"))
        .ghost()
        .icon(Icon::Close)
        .shortcut("X")
        .selected(g.ui_state.build.demolish);
    let dw = ui.button_width(&demolish);
    let dr = Rect::new(undo_r.x - 4.0 - dw, head.y, dw, 26.0);
    if ui.button("build_demolish", dr, &demolish).clicked {
        let st = &mut g.ui_state.build;
        st.demolish = !st.demolish;
        st.selected = None;
        st.drag = None;
    }
    let tabs_x = kr.right() + 12.0;
    let tabs_r = Rect::new(tabs_x, head.y, (dr.x - 8.0 - tabs_x).max(40.0), 26.0);
    let wishes = g.world.build_wishes(content);
    let buildable = g.world.buildable(content);
    let wanted = |d: &crate::content::FurnitureDef| {
        buildable.contains(&d.id) && super::build_cards::wish_for(&wishes, d).is_some()
    };
    let count = |c: Option<BuildCategory>| {
        content
            .furniture
            .iter()
            .filter(|d| c.is_none_or(|c| d.category == c) && wanted(d))
            .count() as u32
    };
    let mut labels: Vec<(String, u32)> = vec![(loc.tr_owned("ui.build.tab.all"), count(None))];
    for c in TABS {
        labels.push((loc.tr_owned(cat_key(c)), count(Some(c))));
    }
    labels.push((loc.tr_owned("ui.theme.tab"), 0));
    let theme_tab = labels.len() - 1;
    g.ui_state.build.tab = g.ui_state.build.tab.min(theme_tab);
    mini_tabs(ui, tabs_r, &labels, &mut g.ui_state.build.tab);

    if g.ui_state.build.tab == theme_tab {
        let st = &mut g.ui_state.build;
        st.selected = None;
        st.demolish = false;
        st.drag = None;
        super::build_themes::themes(g, ui, cx, rest);
        hint_line(ui, foot, loc.tr("bux.hint.themes"));
        return;
    }
    let tab = g.ui_state.build.tab;
    let defs: Vec<_> = content
        .furniture
        .iter()
        .filter(|d| d.category != BuildCategory::Structure)
        .filter(|d| tab == 0 || TABS.get(tab - 1) == Some(&d.category))
        .filter(|d| {
            buildable.contains(&d.id) || super::build_cards::lock_hint(&loc, &g.world, d).is_some()
        })
        .collect();
    super::build_cards::row(g, ui, cx, rest, &defs, &wishes, &buildable);
    let st = &g.ui_state.build;
    let hint = if st.drag.is_some() {
        "bux.hint.moving"
    } else if st.selected.is_some() {
        "bux.hint.placing"
    } else if st.demolish {
        "bux.hint.demolish"
    } else {
        "bux.hint.idle"
    };
    hint_line(ui, foot, loc.tr(hint));
}

fn hint_line(ui: &mut Ui, r: Rect, text: &str) {
    let st = ui.theme.small().size(11.0);
    ui.fonts
        .draw_in(text, r.inset_xy(2.0, 0.0), &st, Align::Left);
}

/// Compact tabs (12 pt): a pill for the open one, an amber count when someone waits for a
/// piece in that tab. They shrink a little to fit the row in the longer languages.
fn mini_tabs(ui: &mut Ui, r: Rect, labels: &[(String, u32)], selected: &mut usize) {
    let t = ui.theme.clone();
    let badge_w = |n: u32| if n > 0 { 16.0 } else { 0.0 };
    let natural = |ui: &Ui, st: &crate::ui::TextStyle, pad: f32| -> f32 {
        labels
            .iter()
            .map(|(l, n)| ui.fonts.measure(l, st) + 2.0 * pad + badge_w(*n) + 2.0)
            .sum::<f32>()
    };
    let mut st = t.small_medium();
    let mut pad = 9.0;
    while natural(ui, &st, pad) > r.w && pad > 5.0 {
        pad -= 1.0;
    }
    while natural(ui, &st, pad) > r.w && st.size > 10.5 {
        st = st.size(st.size - 0.5);
    }
    let mut x = r.x;
    for (i, (label, n)) in labels.iter().enumerate() {
        let w = ui.fonts.measure(label, &st) + 2.0 * pad + badge_w(*n);
        let cell = Rect::new(x, r.y + 1.0, w, r.h - 2.0);
        x += w + 2.0;
        let resp = ui.interact(ui.id(("btab", i)), cell);
        let on = *selected == i;
        if resp.clicked && !on {
            *selected = i;
            ui.sound(UiSound::Click);
        }
        if on {
            fill_rrect(cell, cell.h / 2.0, with_a(t.indigo, 0.34));
        } else if resp.hovered {
            fill_rrect(cell, cell.h / 2.0, Color::new(1.0, 1.0, 1.0, 0.06));
        }
        let fg = if on { t.text } else { t.text_dim };
        let text_r = Rect::new(cell.x + pad, cell.y, w - 2.0 * pad - badge_w(*n), cell.h);
        ui.fonts
            .draw_in(label, text_r, &st.color(fg), Align::Center);
        if *n > 0 {
            let c = vec2(text_r.right() + 9.0, cell.center_y());
            fill_circle(c, 6.5, t.amber);
            ui.fonts.draw_in(
                &n.to_string(),
                Rect::new(c.x - 6.0, c.y - 7.0, 12.0, 14.0),
                &t.key().size(10.0).color(t.ink),
                Align::Center,
            );
        }
    }
}

/// A furniture sprite scaled into a UI rect (UI camera is active).
pub(super) fn draw_preview(sprite: &str, size: (u32, u32), r: Rect, t: f32, unlocked: bool) {
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
