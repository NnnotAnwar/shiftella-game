//! Karolina's loop in the world (design/handson.md §7.4): what the mouse points at, and the
//! world-space pieces drawn over the lit café — order marks and wish bubbles over guests,
//! her queue chips, tip coins, floats, the hold ring at the sink and the swipe hint.

use macroquad::prelude::*;

use super::GameScreen;
use super::hands::{self, karolina_acts, queue_icon};
use super::scene::Pick;
use super::world_view::{self as wv, sim};
use crate::content::Content;
use crate::content::handson::Taste;
use crate::map::TILE;
use crate::render::{
    self, OrderMark, TasteIcon, draw_order_mark, draw_queue_chips, draw_tip_coin, draw_tip_float,
    draw_wish_bubble, emote_anchor, queue_chip_rects,
};
use crate::ui::RectExt;
use sim::{AgentRef, PartyState, Phase};

/// The thing under the mouse that Karolina can act on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Target {
    /// A chip over her head (0 = the current act when there is one).
    Chip(usize),
    /// A tip coin on this table.
    Coin(u32),
    Guest(u32),
    Staff(u32),
    /// A dirt blot on this tile.
    Dirt((i32, i32)),
    Sink(u32),
    /// A table with cups or coins.
    Table(u32),
    Jukebox(u32),
    /// A window while the light in the window is still off.
    Window,
    /// A night moment's glowing marker (index into `WorldScene::markers`).
    Moment(usize),
    /// An open opening / closing ritual item on this piece (index into `content.app.checklists`).
    Checklist(usize),
    /// An appliance with something to do now (v0.2): the machine, the stove, the oven…
    Equip(u32),
    /// A lamp: on / off.
    Lamp(u32),
}

/// Checklist items Karolina can do by hand in the world right now: (index into
/// `content.app.checklists`, furniture uid it sits on). Opening items until done, closing
/// items from 05:00; the window light, the record and the handover note have their own ways.
pub fn open_rituals(g: &GameScreen, content: &Content) -> Vec<(usize, u32)> {
    let w = &g.world;
    if w.phase() != Phase::Night {
        return Vec::new();
    }
    let m = w.minute();
    let mut out = Vec::new();
    for (i, it) in content.app.checklists.iter().enumerate() {
        // the window has its own target; the record is picked at the jukebox
        if matches!(
            it.id.as_str(),
            "light_window" | "record_on" | "light_off_at_six"
        ) {
            continue;
        }
        // the handover note waits for closing time
        if it.id == "handover_note" && m < 466 {
            continue;
        }
        let open = w
            .app()
            .checklist
            .iter()
            .any(|c| c.id == it.id && !c.done && c.claim.is_none());
        let in_time = match it.list {
            crate::content::ChecklistKind::Opening => true,
            crate::content::ChecklistKind::Closing => m >= 7 * 60,
        };
        let Some(tag) = it.anchor.as_deref() else {
            continue;
        };
        if !open || !in_time {
            continue;
        }
        if let Some(p) = w
            .furniture()
            .iter()
            .find(|p| def_has(content, w, p.uid, tag))
        {
            out.push((i, p.uid));
        }
    }
    out
}

pub fn taste_icon(t: Taste) -> TasteIcon {
    match t {
        Taste::Warm => TasteIcon::Warm,
        Taste::Cold => TasteIcon::Cold,
        Taste::Sweet => TasteIcon::Sweet,
        Taste::Bitter => TasteIcon::Bitter,
        Taste::Strong => TasteIcon::Strong,
        Taste::Mild => TasteIcon::Mild,
        Taste::Comforting => TasteIcon::Comforting,
        Taste::Fancy => TasteIcon::Fancy,
        Taste::Quick => TasteIcon::Quick,
        Taste::Filling => TasteIcon::Filling,
        Taste::Spiced => TasteIcon::Spiced,
        Taste::Fresh => TasteIcon::Fresh,
    }
}

/// Karolina's head anchor (queue chips float over it).
fn karolina_anchor(g: &mut GameScreen) -> Option<Vec2> {
    let k = g.world.karolina()?.id;
    let v = g
        .scene
        .people
        .iter()
        .find(|v| v.agent == AgentRef::Staff(k))?
        .clone();
    let look = g.scene.look(&v.look);
    Some(emote_anchor(&look, &wv::person_pose(&v, g.time)))
}

fn def_has(content: &Content, w: &sim::World, uid: u32, tag: &str) -> bool {
    w.furniture_by_uid(uid)
        .and_then(|p| content.furniture_def(&p.def))
        .is_some_and(|d| d.has_tag(tag))
}

/// What the mouse points at (`pick` = the scene's hit test). Night only, except the jukebox.
pub fn target(
    g: &mut GameScreen,
    content: &Content,
    pick: Option<Pick>,
    m: Vec2,
) -> Option<Target> {
    let night = g.world.phase() == Phase::Night;
    if night {
        // chips over her head first (they float above everything)
        if let Some(anchor) = karolina_anchor(g) {
            let n = karolina_acts(&g.world).len();
            for (i, r) in queue_chip_rects(anchor, n).iter().enumerate() {
                if r.grow(2.0).contains(m) {
                    return Some(Target::Chip(i));
                }
            }
        }
        for c in &g.hands.coins {
            if c.coin.collected.is_none() && c.coin.position(g.time).distance(m) < 11.0 {
                return Some(Target::Coin(c.table));
            }
        }
        // moment markers float over everything else in the room
        for (i, (_, mk)) in g.scene.markers.iter().enumerate() {
            if mk.orb(g.time).distance(m) < crate::render::marker_radius(mk) + 4.0 {
                return Some(Target::Moment(i));
            }
        }
    }
    match pick {
        Some(Pick::Agent(AgentRef::Guest(id))) => return Some(Target::Guest(id)),
        Some(Pick::Agent(AgentRef::Staff(id))) => return Some(Target::Staff(id)),
        _ => {}
    }
    // pendant lamps hang high over the tables: their shades are a target of their own
    if let Some(uid) = lamp_shade_at(g, content, m) {
        return Some(Target::Lamp(uid));
    }
    let tile = ((m.x / TILE).floor() as i32, (m.y / TILE).floor() as i32);
    if night && g.world.dirt_spots().contains(&tile) {
        return Some(Target::Dirt(tile));
    }
    // furniture: the pick, or the piece drawn up into this tile (3/4 view: tall pieces
    // reach one tile up)
    let below = (tile.0, tile.1 + 1);
    let uid = match pick {
        Some(Pick::Furniture(u)) => Some(u),
        _ => None,
    }
    .or_else(|| g.world.grid().object_at(below.0, below.1));
    if let Some(uid) = uid {
        let w = &g.world;
        if def_has(content, w, uid, "music") {
            return Some(Target::Jukebox(uid));
        }
        if night && def_has(content, w, uid, "dishes") && w.sink_dishes() > 0 {
            return Some(Target::Sink(uid));
        }
        if night && def_has(content, w, uid, "table") {
            let busy = w.furniture_by_uid(uid).is_some_and(|p| p.dishes > 0)
                || w.tips_on_tables().iter().any(|t| t.table == uid);
            if busy {
                return Some(Target::Table(uid));
            }
        }
        // an appliance with something to do (warm up, the soup pot, the cake, the till…)
        if night && g.world.equipment_action(content, uid).is_some() {
            return Some(Target::Equip(uid));
        }
        // a ritual item on this piece (count the float, the fridge check…)
        if let Some((i, _)) = open_rituals(g, content).into_iter().find(|r| r.1 == uid) {
            return Some(Target::Checklist(i));
        }
        // the fridge opens the pantry; a floor lamp toggles
        match g.world.equipment_kind(content, uid) {
            Some(sim::EquipKind::Fridge) if night => return Some(Target::Equip(uid)),
            Some(sim::EquipKind::Lamp)
                if !(night
                    && super::hands::is_window_lamp(&g.world, content, uid)
                    && (!g.world.window_light_on(content) || super::hands::lights_off_due(g))) =>
            {
                return Some(Target::Lamp(uid));
            }
            _ => {}
        }
    }
    let closing = night && super::hands::lights_off_due(g);
    if night
        && (!g.world.window_light_on(content) || closing)
        && [tile, below].iter().any(|t| {
            g.world
                .grid()
                .tile(t.0, t.1)
                .is_some_and(|i| i.kind == sim::TileKind::Window)
        })
    {
        return Some(Target::Window);
    }
    None
}

/// The render looks of every appliance (world px origin, size, look).
pub fn appliance_looks(
    w: &sim::World,
    content: &Content,
) -> Vec<(Vec2, (u8, u8), render::ApplianceLook)> {
    use render::{ApplianceKind as K, CakeLook, PotLook};
    let slices = content.handson.tuning.equipment.cake.slices;
    let mut out = Vec::new();
    for v in w.equipment_views(content) {
        let Some(p) = w.furniture_by_uid(v.uid) else {
            continue;
        };
        let Some(def) = content.furniture_def(&p.def) else {
            continue;
        };
        let kind = match v.kind {
            sim::EquipKind::Machine => K::Machine,
            sim::EquipKind::Stove => K::Stove,
            sim::EquipKind::Oven => K::Oven,
            sim::EquipKind::Fridge => K::Fridge,
            sim::EquipKind::Register => K::Register,
            sim::EquipKind::Trash => K::Trash,
            sim::EquipKind::Lamp => continue,
        };
        let (sw, sh) = sim::grid::rotated_size(def, p.rot);
        let mut look = render::ApplianceLook::new(kind);
        look.warm = v.warm;
        look.working = v.working.map(|(_, k)| k);
        look.worn = v.worn;
        look.tray = v.tray;
        look.pot = match v.pot {
            sim::PotView::None => PotLook::None,
            sim::PotView::Warm => PotLook::Warm,
            sim::PotView::Bubbling => PotLook::Bubbling,
            sim::PotView::Stirred => PotLook::Stirred,
        };
        look.cake = match v.cake {
            sim::CakeView::None => CakeLook::None,
            sim::CakeView::Baking(k) => CakeLook::Baking(k),
            sim::CakeView::Ready => CakeLook::Ready,
            sim::CakeView::Out(n) => CakeLook::Out {
                left: n,
                of: slices.max(n),
            },
        };
        look.milk_out = v.milk_out;
        look.counted = v.counted;
        look.bag = v.bag;
        out.push((
            vec2(p.x as f32, p.y as f32) * TILE,
            (sw as u8, sh as u8),
            look,
        ));
    }
    out
}

/// The cue chip of a chore (`None` = the ritual chip or nothing marks it).
fn cue_glyph(a: sim::EquipAction) -> Option<render::EquipGlyph> {
    use render::EquipGlyph as G;
    use sim::EquipAction as A;
    Some(match a {
        A::Service => G::Puff,
        A::SoupOn | A::Stir => G::Pot,
        A::Bake | A::TakeOut => G::Cake,
        A::Milk => G::Milk,
        A::CountTill => G::Coins,
        A::Trash => G::Bag,
        // a tray ahead: only while a train is due (see `draw_world`)
        A::PullTray => G::Cup,
        // the warm-up / clean-up have the ritual chip
        A::WarmUp | A::CleanUp => return None,
    })
}

/// Where a lamp's shade glows (world px): pendants hang ~1.5 tiles over their tile.
pub fn lamp_shade(g: &GameScreen, content: &Content, uid: u32) -> Option<Vec2> {
    let p = g.world.furniture_by_uid(uid)?;
    let d = content.furniture_def(&p.def)?;
    if !d.has_tag("light") || d.has_tag("music") {
        return None;
    }
    let c = vec2(p.x as f32 + 0.5, p.y as f32 + 0.5) * TILE;
    Some(match d.sprite.as_str() {
        "pendant_lamp" => c - vec2(0.0, 90.0),
        "floor_lamp" => c - vec2(0.0, 84.0),
        _ => c - vec2(0.0, 30.0),
    })
}

/// A pendant lamp's shade under the mouse (they share their tile with a table).
fn lamp_shade_at(g: &GameScreen, content: &Content, m: Vec2) -> Option<u32> {
    g.world
        .furniture()
        .iter()
        .filter(|p| {
            content
                .furniture_def(&p.def)
                .is_some_and(|d| d.sprite == "pendant_lamp" && d.has_tag("light"))
        })
        .find(|p| lamp_shade(g, content, p.uid).is_some_and(|c| c.distance(m) < 15.0))
        .map(|p| p.uid)
}

/// Draws the hands-on pieces over the lit café (the world camera is active).
pub fn draw_world(
    g: &mut GameScreen,
    content: &Content,
    loc: &crate::settings::Locale,
    hover: Option<Target>,
    mouse: Vec2,
) {
    let t = g.time;
    let night = g.world.phase() == Phase::Night;
    if !night {
        return;
    }
    // hover rings on things she can act on
    if let Some(h) = hover {
        let ring = |c: Vec2, rx: f32| {
            let pulse = 0.5 + 0.5 * (t * 3.0).sin();
            draw_ellipse_lines(
                c.x,
                c.y,
                rx,
                rx * 0.42,
                0.0,
                1.6,
                Color::new(0.65, 0.7, 1.0, 0.55 + 0.3 * pulse),
            );
        };
        let at = |uid: u32| {
            g.world
                .furniture_by_uid(uid)
                .map(|p| vec2(p.x as f32 + 0.5, p.y as f32 + 0.75) * TILE)
        };
        match h {
            Target::Sink(u) | Target::Table(u) | Target::Jukebox(u) | Target::Equip(u) => {
                if let Some(c) = at(u) {
                    ring(c, 30.0);
                }
            }
            Target::Lamp(u) => {
                if let Some(c) = lamp_shade(g, content, u) {
                    draw_circle_lines(c.x, c.y, 13.0, 1.4, Color::new(1.0, 0.85, 0.55, 0.7));
                }
            }
            Target::Dirt(tile) => ring(vec2(tile.0 as f32 + 0.5, tile.1 as f32 + 0.5) * TILE, 26.0),
            _ => {}
        }
    }
    // a little "Reserved" tent card on a seat kept through the app for a regular
    let reserved: Vec<u32> = g.world.reserved_seats().values().copied().collect();
    if !reserved.is_empty() {
        let label = loc.tr_owned("ui.reserved");
        for uid in reserved {
            let Some(p) = g.world.furniture_by_uid(uid) else {
                continue;
            };
            let c = vec2(p.x as f32 + 0.5, p.y as f32 + 0.1) * TILE;
            let w = render::world_text_width(&label, 9.0) + 10.0;
            let r = Rect::new(c.x - w / 2.0, c.y - 13.0, w, 15.0);
            draw_rectangle(
                r.x + 1.5,
                r.y + 2.0,
                r.w,
                r.h,
                Color::new(0.05, 0.03, 0.1, 0.35),
            );
            draw_rectangle(r.x, r.y, r.w, r.h, Color::new(1.0, 0.95, 0.86, 0.95));
            draw_rectangle(r.x, r.y, r.w, 2.0, Color::new(1.0, 0.77, 0.42, 1.0));
            render::world_text(
                &label,
                vec2(c.x, r.y + 11.5),
                9.0,
                Color::new(0.24, 0.17, 0.42, 1.0),
                true,
            );
        }
    }
    // the little sign on the door: "Closed" through the preparation half hour, it flips to
    // "Open" at 22:00
    if let Some(&(dx, dy)) = g.world.grid().door_tiles().first() {
        let open = g.world.doors_open();
        let label = loc.tr_owned(if open {
            "prep.sign.open"
        } else {
            "prep.sign.closed"
        });
        let flip = g
            .scene
            .doors_opened_at
            .map(|at| ((t - at) / 0.7).clamp(0.0, 1.0))
            .unwrap_or(1.0);
        // a quick turn on its string: narrow → full width (the new side)
        let sx = (flip * std::f32::consts::PI).cos().abs().max(0.08);
        let c = vec2(dx as f32 + 0.5, dy as f32 + 0.1) * TILE;
        let w = (render::world_text_width(&label, 11.0) + 12.0) * sx;
        let r = Rect::new(c.x - w / 2.0, c.y - 9.0, w, 18.0);
        let base = if open {
            Color::new(0.06, 0.6, 0.45, 0.95)
        } else {
            Color::new(0.62, 0.22, 0.32, 0.92)
        };
        draw_line(
            c.x - 6.0,
            r.y,
            c.x,
            r.y - 6.0,
            1.0,
            Color::new(0.8, 0.7, 0.5, 0.8),
        );
        draw_line(
            c.x + 6.0,
            r.y,
            c.x,
            r.y - 6.0,
            1.0,
            Color::new(0.8, 0.7, 0.5, 0.8),
        );
        if open && flip < 1.0 {
            draw_circle(
                c.x,
                c.y,
                18.0 * (1.0 - flip),
                Color::new(0.2, 0.9, 0.6, 0.25),
            );
        }
        crate::ui::draw::fill_rrect(r, 3.0, base);
        if sx > 0.6 {
            render::world_text(&label, vec2(c.x, r.y + 13.5), 11.0, WHITE, true);
        }
    }
    // marks over guests: "!" waiting to order, a check when she's on it, the usual, water,
    // and the wish bubble while a wish waits for a recipe
    let me = g.world.karolina().map(|k| k.id);
    let views = g.scene.people.clone();
    for v in &views {
        let AgentRef::Guest(id) = v.agent else {
            continue;
        };
        if v.emote.is_some() {
            continue;
        }
        let Some(p) = g.world.party_of_guest(id).cloned() else {
            continue;
        };
        if p.leader() != Some(id) {
            continue;
        }
        let look = g.scene.look(&v.look);
        let pose = wv::person_pose(v, t);
        let anchor = emote_anchor(&look, &pose);
        let age = 1.0;
        if g.hands.water.contains_key(&p.id) {
            draw_order_mark(anchor, OrderMark::Water, 1.0, t);
            continue;
        }
        match p.state {
            PartyState::WaitOrder | PartyState::Queue => {
                let mark = match p.order_claim {
                    Some(c) if Some(c) == me => OrderMark::Claimed,
                    Some(_) => continue,
                    None if p.order_kind.usual().is_some() => OrderMark::Usual,
                    None => OrderMark::Waiting,
                };
                draw_order_mark(anchor, mark, age, t);
            }
            PartyState::WaitFood | PartyState::Eating if p.wish_open => {
                if let Some(wish) = p.order_kind.wish().and_then(|w| content.handson.wish(w)) {
                    let tastes: Vec<TasteIcon> =
                        wish.want.iter().take(4).map(|t| taste_icon(*t)).collect();
                    draw_wish_bubble(anchor, &tastes, age, t);
                }
            }
            _ => {}
        }
    }
    // ritual items: a little checkbox chip on their furniture
    for (i, uid) in open_rituals(g, content) {
        let Some(p) = g.world.furniture_by_uid(uid) else {
            continue;
        };
        let hov = hover == Some(Target::Checklist(i));
        let s = crate::render::world_icon_scale();
        let c = vec2(p.x as f32 + 0.5, p.y as f32 - 0.45) * TILE + vec2(0.0, (t * 2.0).sin() * 1.5);
        let r = Rect::new(c.x - 9.0 * s, c.y - 9.0 * s, 18.0 * s, 18.0 * s);
        draw_circle(
            c.x,
            c.y,
            15.0 * s,
            Color::new(0.39, 0.4, 0.95, if hov { 0.3 } else { 0.16 }),
        );
        crate::ui::draw::fill_rrect(r, 5.0 * s, Color::new(0.11, 0.1, 0.24, 0.9));
        crate::ui::draw::stroke_rrect(r, 5.0 * s, 1.4 * s, Color::new(0.65, 0.7, 1.0, 0.9));
        let ink = Color::new(0.75, 0.8, 1.0, 0.85);
        draw_line(c.x - 4.0 * s, c.y, c.x - 1.0 * s, c.y + 3.5 * s, 2.0 * s, ink);
        draw_line(c.x - 1.0 * s, c.y + 3.5 * s, c.x + 4.5 * s, c.y - 3.5 * s, 2.0 * s, ink);
    }
    // appliances that offer something: a small amber chip (the ritual chip marks the rest)
    let rituals: Vec<u32> = open_rituals(g, content).iter().map(|r| r.1).collect();
    let working = g.world.player().current.as_ref().and_then(|c| match c.act {
        sim::PlayerAct::Use { uid, action } if c.stage >= 1 => Some((uid, action, c.progress())),
        _ => None,
    });
    let m = g.world.minutes_since_open();
    let train_soon = content.trains.trains.iter().any(|tr| {
        let at = tr.arrives.minutes_since_open();
        (0..480).contains(&at) && (m..m + 10).contains(&at)
    });
    for v in g.world.equipment_views(content) {
        let Some(a) = v.action else {
            continue;
        };
        let Some(glyph) = cue_glyph(a) else {
            continue;
        };
        if a == sim::EquipAction::PullTray && !train_soon {
            continue;
        }
        if rituals.contains(&v.uid) || working.is_some_and(|w| w.0 == v.uid) {
            continue;
        }
        let Some(p) = g.world.furniture_by_uid(v.uid) else {
            continue;
        };
        let (sw, _) = content
            .furniture_def(&p.def)
            .map(|d| sim::grid::rotated_size(d, p.rot))
            .unwrap_or((1, 1));
        let c = vec2(p.x as f32 + sw as f32 * 0.5, p.y as f32 - 0.5) * TILE;
        let urgent = matches!(
            a,
            sim::EquipAction::Stir | sim::EquipAction::TakeOut | sim::EquipAction::Service
        );
        render::draw_equip_cue(c, glyph, t, hover == Some(Target::Equip(v.uid)), urgent);
    }
    // a chore in progress at an appliance: its ring (a hold chore fills faster held)
    if let Some((uid, action, progress)) = working
        && let Some(p) = g.world.furniture_by_uid(uid)
    {
        let c = vec2(p.x as f32 + 0.5, p.y as f32 - 0.55) * TILE;
        let held = g.hands.equip_hold.is_some_and(|h| h.sink == uid);
        render::draw_work_ring(c, progress, action.hold(), held, t);
    }
    // the hover label: what a click does here (compact, one line)
    if let Some(h) = hover {
        let label = match h {
            Target::Equip(u) => Some(match g.world.equipment_action(content, u) {
                Some(a) if a.hold() => format!(
                    "{} · {}",
                    loc.tr_owned(&format!("equip.act.{}", a.id())),
                    loc.tr_owned("equip.hold")
                ),
                Some(a) => loc.tr_owned(&format!("equip.act.{}", a.id())),
                None => loc.tr_owned("equip.act.pantry"),
            })
            .zip(
                g.world
                    .furniture_by_uid(u)
                    .map(|p| vec2(p.x as f32 + 0.5, p.y as f32 - 0.95) * TILE),
            ),
            Target::Lamp(u) => g.world.furniture_by_uid(u).and_then(|p| {
                let key = if p.on {
                    "equip.act.lamp_off"
                } else {
                    "equip.act.lamp_on"
                };
                lamp_shade(g, content, u).map(|c| (loc.tr_owned(key), c - vec2(0.0, 20.0)))
            }),
            _ => None,
        };
        if let Some((text, at)) = label {
            let w = render::world_text_width(&text, 9.0) + 10.0;
            let r = Rect::new(at.x - w / 2.0, at.y - 8.0, w, 14.0);
            crate::ui::draw::fill_rrect(r, 4.0, Color::new(0.08, 0.07, 0.17, 0.88));
            crate::ui::draw::stroke_rrect(r, 4.0, 1.0, Color::new(1.0, 0.8, 0.5, 0.55));
            render::world_text(
                &text,
                vec2(at.x, r.y + 10.5),
                9.0,
                Color::new(0.97, 0.94, 1.0, 1.0),
                true,
            );
        }
    }
    // tip coins and their glints
    for c in &g.hands.coins {
        draw_tip_coin(&c.coin, t);
    }
    // Karolina's queue (current first, with the progress ring)
    if let Some(anchor) = karolina_anchor(g) {
        let acts = karolina_acts(&g.world);
        if !acts.is_empty() {
            let icons: Vec<_> = acts
                .iter()
                .take(5)
                .map(|a| queue_icon(content, &g.world, a))
                .collect();
            let progress = g
                .world
                .player()
                .current
                .as_ref()
                .map(|c| c.progress())
                .unwrap_or(0.0);
            let hovered = match hover {
                Some(Target::Chip(i)) => Some(i),
                _ => None,
            };
            draw_queue_chips(anchor, &icons, progress, hovered, t);
        }
    }
    // the sink: a hold ring while she washes
    let washing = g.world.player().current.as_ref().and_then(|c| match c.act {
        sim::PlayerAct::Wash { sink } if c.stage == 1 => Some(sink),
        _ => None,
    });
    if let Some(sink) = washing
        && let Some(p) = g.world.furniture_by_uid(sink)
    {
        let c = vec2(p.x as f32 + 0.5, p.y as f32) * TILE - vec2(0.0, 30.0);
        let held = g.hands.wash.map_or(0.0, |h| h.held);
        let left = g.world.sink_dishes();
        let pulse = 0.5 + 0.5 * (t * 4.0).sin();
        draw_circle(c.x, c.y, 13.0, Color::new(0.11, 0.1, 0.24, 0.85));
        draw_circle_lines(c.x, c.y, 13.0, 1.4, Color::new(0.65, 0.7, 1.0, 0.5));
        let frac = (held * 2.0 % 1.0).clamp(0.0, 1.0);
        let k = if g.hands.wash.is_some() { frac } else { 0.0 };
        let segs = 24;
        for i in 0..(segs as f32 * k) as usize {
            let a0 = -std::f32::consts::FRAC_PI_2 + i as f32 / segs as f32 * std::f32::consts::TAU;
            let a1 = a0 + std::f32::consts::TAU / segs as f32;
            draw_line(
                c.x + a0.cos() * 13.0,
                c.y + a0.sin() * 13.0,
                c.x + a1.cos() * 13.0,
                c.y + a1.sin() * 13.0,
                2.4,
                Color::new(0.65, 0.7, 1.0, 1.0),
            );
        }
        render::world_text(
            &left.to_string(),
            c + vec2(0.0, 4.0),
            11.0,
            Color::new(0.93, 0.92, 1.0, 0.7 + 0.3 * pulse),
            true,
        );
    }
    // floats: "+38 Kč" over the jar, "Perfect · Lovely" over a served guest
    for f in &g.hands.floats {
        let age = t - f.born;
        if age < 0.0 {
            continue;
        }
        let args = super::resolve_args(loc, &f.args);
        let text = wv::tr_args(loc, &f.text, &args);
        draw_tip_float(f.at, &text, age);
    }
    // the swipe hint over a dirt blot (a click, not a swipe)
    if let Some((tile, since)) = g.hands.swipe_hint {
        let age = t - since;
        if age < 1.6 {
            let c = vec2(tile.0 as f32 + 0.5, tile.1 as f32 + 0.5) * TILE;
            let k = (age / 1.6).clamp(0.0, 1.0);
            let a = (1.0 - k) * 0.9;
            let x0 = c.x - 22.0 + 44.0 * k;
            let col = Color::new(0.75, 0.78, 1.0, a);
            draw_line(c.x - 22.0, c.y, x0, c.y, 2.5, col);
            draw_triangle(
                vec2(x0 + 7.0, c.y),
                vec2(x0 - 2.0, c.y - 5.0),
                vec2(x0 - 2.0, c.y + 5.0),
                col,
            );
        } else {
            g.hands.swipe_hint = None;
        }
    }
    // a swipe in progress: a little trail
    if let Some(sw) = g.hands.swipe {
        let k = (sw.travelled / hands::SWIPE_MIN).clamp(0.0, 1.0);
        draw_circle(
            mouse.x,
            mouse.y,
            4.0 + 4.0 * k,
            Color::new(0.75, 0.8, 1.0, 0.35),
        );
    }
}
