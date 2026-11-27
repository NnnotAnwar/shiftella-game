//! Build cards: a small preview, the name, the price and chips for what a piece gives
//! (coziness, comfort, seats, light, music, a work station, a socket). Hover = the description
//! and the exact numbers. Locked cards say how to unlock; a face + "Filip is waiting" when a
//! regular or a helper wants that piece (`World::build_wishes`). The row scrolls sideways
//! (wheel, or the arrows at its ends) and shows that there is more.

use std::rc::Rc;

use macroquad::prelude::*;

use super::world_view::sim;
use super::{Ctx, GameScreen};
use crate::content::{BuildCategory, Content, FurnitureDef};
use crate::i18n::{format_decimal, format_money};
use crate::render::PersonLook;
use crate::settings::Locale;
use crate::ui::draw::{fill_rrect, stroke_rrect, with_a};
use crate::ui::{Align, Icon, Paint, RectExt, Theme, Ui, draw_icon};
use sim::{BuildWish, World};

const CARD_W: f32 = 106.0;
const GAP: f32 = 6.0;

/// The open wish that this piece answers (by id or by tag).
pub fn wish_for<'a>(wishes: &'a [BuildWish], d: &FurnitureDef) -> Option<&'a BuildWish> {
    wishes.iter().find(|w| {
        w.def.as_deref() == Some(d.id.as_str()) || w.tag.as_deref().is_some_and(|t| d.has_tag(t))
    })
}

/// Display name and look of whoever wishes (a regular by id, else a helper by id or name).
pub fn wisher(g: &GameScreen, loc: &Locale, who: &str) -> (String, Option<PersonLook>) {
    let key = format!("regular.{who}.name");
    if loc.has(&key) {
        return (loc.tr_owned(&key), Some(PersonLook::preset(who)));
    }
    let staff = g.world.staff().iter().find(|s| {
        s.id.to_string() == who || s.archetype == who || s.name.eq_ignore_ascii_case(who)
    });
    match staff {
        Some(s) => (super::world_view::staff_name(loc, s), None),
        // a guest kind's wish ("students would like a second socket")
        None if loc.has(&format!("guest.{who}.name")) => {
            (loc.tr_owned(&format!("guest.{who}.name")), None)
        }
        None => (who.to_string(), None),
    }
}

/// How a locked piece unlocks, in plain words — `None` for pieces that only come as a
/// surprise (tip-jar rewards, theme accents) and stay hidden until then.
pub fn lock_hint(loc: &Locale, world: &World, d: &FurnitureDef) -> Option<String> {
    if d.category == BuildCategory::Structure {
        return None;
    }
    if let Some(flag) = &d.unlock_flag
        && !world.story().flags.contains(flag)
    {
        let key = format!("bux.lock.flag.{flag}");
        return loc.has(&key).then(|| loc.tr_owned(&key));
    }
    match world.mode() {
        sim::Mode::Story if d.unlock_night > world.night() => {
            Some(loc.trf("bux.lock.night", &[("night", &d.unlock_night.to_string())]))
        }
        sim::Mode::Sandbox => Some(loc.tr_owned("bux.lock.sandbox")),
        _ => None,
    }
}

fn num(loc: &Locale, v: f32) -> String {
    if (v - v.round()).abs() < 0.05 {
        format!("{}", v.round() as i32)
    } else {
        format_decimal(loc.lang, v, 1)
    }
}

/// The station kind of a piece (`station:coffee` → `coffee`, a sink, the register…).
fn station(d: &FurnitureDef) -> Option<&str> {
    d.tags
        .iter()
        .find_map(|t| t.strip_prefix("station:"))
        .or_else(|| d.has_tag("sink").then_some("sink"))
        .or_else(|| d.has_tag("register").then_some("register"))
        .or_else(|| d.has_tag("drone").then_some("drone"))
        .or_else(|| d.has_tag("trash").then_some("trash"))
}

/// A station helper (`helps:*`): the station it speeds up (as a station kind) and how much.
fn helper(d: &FurnitureDef) -> Option<(&'static str, f32)> {
    sim::companions::helps(d).map(|(tag, mul)| (tag.strip_prefix("station:").unwrap_or(tag), mul))
}

fn station_icon(kind: &str) -> Icon {
    match kind {
        "coffee" => Icon::Coffee,
        "stove" => Icon::Flame,
        "oven" => Icon::ChefHat,
        "fridge" => Icon::Snowflake,
        "sink" => Icon::Dishes,
        "register" => Icon::CashRegister,
        "drone" => Icon::Plane,
        "trash" => Icon::Broom,
        _ => Icon::Build,
    }
}

/// A chip: an icon (+ a short number) in a colour.
struct Chip {
    icon: Icon,
    text: String,
    color: Color,
}

fn chips(loc: &Locale, t: &Theme, d: &FurnitureDef) -> Vec<Chip> {
    let mut v = Vec::new();
    if d.beauty > 0.0 {
        v.push(Chip {
            icon: Icon::Heart,
            text: format!("+{}", num(loc, d.beauty)),
            color: t.violet_400,
        });
    }
    if d.seats > 0 {
        v.push(Chip {
            icon: Icon::User,
            text: d.seats.to_string(),
            color: t.text,
        });
    }
    if d.comfort > 0.0 {
        v.push(Chip {
            icon: Icon::Feather,
            text: num(loc, d.comfort),
            color: t.indigo_300,
        });
    }
    if d.light_radius > 0.0 && d.has_tag("light") {
        v.push(Chip {
            icon: Icon::Sun,
            text: num(loc, d.light_radius),
            color: t.amber,
        });
    }
    if d.has_tag("music") {
        v.push(Chip {
            icon: Icon::Vinyl,
            text: String::new(),
            color: t.violet_400,
        });
    }
    if let Some(s) = station(d) {
        v.push(Chip {
            icon: station_icon(s),
            text: String::new(),
            color: t.emerald_400,
        });
    }
    if let Some((kind, mul)) = helper(d) {
        v.push(Chip {
            icon: station_icon(kind),
            text: format!("×{}", num(loc, mul)),
            color: t.emerald_400,
        });
    }
    if d.has_tag("power_socket") || d.needs_power && !d.has_tag("music") {
        v.push(Chip {
            icon: Icon::Bolt,
            text: String::new(),
            color: t.amber,
        });
    }
    v
}

/// Hover text: the description, then every number in words.
fn tooltip(
    loc: &Locale,
    g: &GameScreen,
    d: &FurnitureDef,
    lock: Option<&str>,
    wish: Option<&BuildWish>,
) -> String {
    let mut lines = vec![loc.tr_owned(&d.desc_key())];
    if let Some(w) = wish {
        let (name, _) = wisher(g, loc, &w.who);
        lines.push(format!(
            "{} — {}",
            loc.trf("bux.wanted", &[("name", &name)]),
            loc.tr(&w.reason_key)
        ));
    }
    let n = |v: f32| num(loc, v);
    if d.beauty > 0.0 {
        lines.push(loc.trf("bux.tip.beauty", &[("n", &n(d.beauty))]));
    }
    if d.seats > 0 {
        let mut s = loc.trf("bux.tip.seats", &[("n", &d.seats.to_string())]);
        if d.comfort > 0.0 {
            s.push_str(" · ");
            s.push_str(&loc.trf("bux.tip.comfort", &[("n", &n(d.comfort))]));
        }
        lines.push(s);
    } else if d.comfort > 0.0 {
        lines.push(loc.trf("bux.tip.comfort_hall", &[("n", &n(d.comfort))]));
    }
    if d.light_radius > 0.0 && d.has_tag("light") {
        lines.push(loc.trf("bux.tip.light", &[("n", &n(d.light_radius))]));
    }
    if d.has_tag("music") {
        lines.push(loc.tr_owned("bux.tip.music"));
    }
    if let Some(s) = station(d) {
        lines.push(loc.tr_owned(&format!("bux.tip.station.{s}")));
    }
    if let Some((kind, mul)) = helper(d) {
        lines.push(loc.trf(&format!("bar.tip.helps.{kind}"), &[("n", &n(mul))]));
    }
    if d.has_tag("counter") && d.category == BuildCategory::Counters {
        lines.push(loc.tr_owned("bar.tip.counter"));
    }
    if d.has_tag("power_socket") {
        lines.push(loc.tr_owned("bux.tip.socket"));
    }
    let mut last = loc.trf(
        "ui.build.size",
        &[("w", &d.size.0.to_string()), ("h", &d.size.1.to_string())],
    );
    if d.unique {
        last.push_str(" · ");
        last.push_str(loc.tr("ui.build.unique"));
    }
    if d.category == BuildCategory::Floors {
        last.push_str(" · ");
        last.push_str(loc.tr("bux.tip.per_tile"));
    }
    lines.push(last);
    if let Some(l) = lock {
        lines.push(format!("{} {l}", loc.tr("bux.tip.locked")));
    }
    lines.join("\n")
}

/// The scrollable card row.
pub fn row(
    g: &mut GameScreen,
    ui: &mut Ui,
    cx: &mut Ctx,
    rest: Rect,
    defs: &[&FurnitureDef],
    wishes: &[BuildWish],
    buildable: &[String],
) {
    let t = ui.theme.clone();
    let loc = cx.loc;
    let content: &Rc<Content> = cx.content;
    let card_h = rest.h - 4.0;
    let total_w = defs.len() as f32 * (CARD_W + GAP) - GAP;
    let max_off = (total_w - rest.w).max(0.0);
    let id = ui.id("build_scroll_x");
    let wheel = if ui.hovering(rest) {
        ui.wheel().y + ui.wheel().x
    } else {
        0.0
    };
    let page = (rest.w - 2.0 * CARD_W).max(CARD_W);
    // the arrows at both ends scroll a page (drawn after the cards, hit-tested first)
    let left_r = Rect::new(rest.x - 2.0, rest.y + card_h / 2.0 - 13.0, 22.0, 26.0);
    let right_r = Rect::new(
        rest.right() - 20.0,
        rest.y + card_h / 2.0 - 13.0,
        22.0,
        26.0,
    );
    let off_now = ui.anim(id).v;
    let mut nudge = 0.0;
    let show_l = max_off > 0.0 && off_now > 1.0;
    let show_r = max_off > 0.0 && off_now < max_off - 1.0;
    let left = show_l.then(|| ui.interact(ui.id("bscroll_l"), left_r));
    let right = show_r.then(|| ui.interact(ui.id("bscroll_r"), right_r));
    if left.is_some_and(|r| r.clicked) {
        nudge -= page;
    }
    if right.is_some_and(|r| r.clicked) {
        nudge += page;
    }
    let off = {
        let a = ui.anim(id);
        a.v = (a.v - wheel * 40.0 + nudge).clamp(0.0, max_off);
        a.v
    };
    ui.push_clip(rest);
    for (i, d) in defs.iter().enumerate() {
        let r = Rect::new(
            rest.x + i as f32 * (CARD_W + GAP) - off,
            rest.y,
            CARD_W,
            card_h,
        );
        if r.right() < rest.x || r.x > rest.right() {
            continue;
        }
        card(g, ui, &t, &loc, content, r, d, wishes, buildable);
    }
    ui.pop_clip();
    // more to the side: a soft fade + the arrow; a thin track under the row
    if max_off > 0.0 {
        let fade = |r: Rect, left: bool| {
            let (a, b) = (
                Color::new(0.07, 0.06, 0.13, 0.9),
                Color::new(0.07, 0.06, 0.13, 0.0),
            );
            let p = if left {
                Paint::horizontal(r, a, b)
            } else {
                Paint::horizontal(r, b, a)
            };
            fill_rrect(r, 0.0, p);
        };
        if let Some(resp) = left {
            fade(Rect::new(rest.x, rest.y, 34.0, card_h), true);
            arrow(&t, left_r, Icon::ChevronLeft, resp.hovered);
        }
        if let Some(resp) = right {
            fade(Rect::new(rest.right() - 34.0, rest.y, 34.0, card_h), false);
            arrow(&t, right_r, Icon::ChevronRight, resp.hovered);
        }
        let track = Rect::new(rest.x + 2.0, rest.bottom() - 2.0, rest.w - 4.0, 2.0);
        fill_rrect(track, 1.0, Color::new(1.0, 1.0, 1.0, 0.06));
        let k = rest.w / total_w;
        let thumb = Rect::new(
            track.x + track.w * (off / total_w),
            track.y,
            (track.w * k).max(24.0),
            2.0,
        );
        fill_rrect(thumb, 1.0, with_a(t.indigo_300, 0.7));
    }
}

#[allow(clippy::too_many_arguments)]
fn card(
    g: &mut GameScreen,
    ui: &mut Ui,
    t: &Theme,
    loc: &Locale,
    content: &Rc<Content>,
    r: Rect,
    d: &FurnitureDef,
    wishes: &[BuildWish],
    buildable: &[String],
) {
    let unlocked = buildable.iter().any(|b| b == &d.id);
    let afford = g.world.money() >= d.price as i64;
    let placed_unique = d.unique && g.world.furniture().iter().any(|p| p.def == d.id);
    let sel = g.ui_state.build.selected.as_deref() == Some(d.id.as_str());
    let lock = (!unlocked).then(|| lock_hint(loc, &g.world, d)).flatten();
    let wish = unlocked.then(|| wish_for(wishes, d)).flatten();
    let resp = ui.interact(ui.id(("bcard", d.id.as_str())), r);
    fill_rrect(
        r,
        9.0,
        if sel {
            with_a(t.indigo, 0.35)
        } else if resp.hovered {
            Color::new(1.0, 1.0, 1.0, 0.075)
        } else {
            Color::new(1.0, 1.0, 1.0, 0.035)
        },
    );
    if sel {
        stroke_rrect(r, 9.0, 1.0, t.indigo_300);
    } else if wish.is_some() {
        stroke_rrect(r, 9.0, 1.0, with_a(t.amber, 0.55));
    }
    let time = ui.time() as f32;
    let prev = Rect::new(r.x + 8.0, r.y + 4.0, r.w - 16.0, 38.0);
    super::build::draw_preview(&d.sprite, d.size, prev, time, unlocked && !placed_unique);
    let dim = !unlocked || placed_unique;
    let name = loc.tr_owned(&d.name_key());
    ui.fonts.draw_fit(
        &name,
        Rect::new(r.x + 3.0, r.y + 43.0, r.w - 6.0, 15.0),
        &t.small_medium()
            .color(if dim { t.text_disabled } else { t.text }),
        Align::Center,
        0.78,
    );
    if let Some(l) = &lock {
        // how to unlock, on the card itself (two short lines)
        draw_icon(
            Icon::Lock,
            vec2(r.right() - 12.0, r.y + 11.0),
            11.0,
            t.text_dim,
        );
        let lr = Rect::new(r.x + 5.0, r.y + 60.0, r.w - 10.0, 30.0);
        let st = ui
            .fonts
            .fit_lines(l, lr, &t.small().size(11.0).color(t.text_dim), 2);
        ui.fonts.draw_wrapped(l, lr, &st, Align::Center, Some(2));
    } else {
        let price = if placed_unique {
            loc.tr_owned("bux.placed")
        } else if d.price == 0 {
            loc.tr_owned("ui.build.free")
        } else {
            format_money(loc.lang, d.price as i64)
        };
        ui.fonts.draw_in(
            &price,
            Rect::new(r.x + 4.0, r.y + 59.0, r.w - 8.0, 13.0),
            &t.key().color(if placed_unique {
                t.text_disabled
            } else if !afford {
                t.red_400
            } else {
                t.amber
            }),
            Align::Center,
        );
        draw_chips(
            ui,
            t,
            &chips(loc, t, d),
            Rect::new(r.x + 4.0, r.y + 75.0, r.w - 8.0, 15.0),
        );
    }
    if let Some(w) = wish {
        badge(g, ui, t, loc, r, &w.who);
    }
    let tip = tooltip(loc, g, d, lock.as_deref(), wish);
    ui.tooltip(&resp, &tip);
    if resp.clicked && unlocked && !placed_unique {
        let st = &mut g.ui_state.build;
        st.demolish = false;
        st.drag = None;
        st.selected = if sel { None } else { Some(d.id.clone()) };
    }
    let _ = content;
}

/// Chips in a centred row; the ones that don't fit are left to the tooltip.
fn draw_chips(ui: &mut Ui, t: &Theme, chips: &[Chip], r: Rect) {
    let st = t.key().size(10.5);
    let widths: Vec<f32> = chips
        .iter()
        .map(|c| {
            let tw = if c.text.is_empty() {
                0.0
            } else {
                ui.fonts.measure(&c.text, &st) + 3.0
            };
            12.0 + tw + 8.0
        })
        .collect();
    let mut n = chips.len();
    while n > 0 && widths[..n].iter().sum::<f32>() + 3.0 * (n as f32 - 1.0) > r.w {
        n -= 1;
    }
    let total = widths[..n].iter().sum::<f32>() + 3.0 * (n.max(1) as f32 - 1.0);
    let mut x = r.center().x - total / 2.0;
    for (c, w) in chips.iter().zip(&widths).take(n) {
        let cr = Rect::new(x, r.y, *w, r.h);
        fill_rrect(cr, r.h / 2.0, with_a(c.color, 0.13));
        draw_icon(c.icon, vec2(cr.x + 4.0 + 6.0, cr.center_y()), 10.0, c.color);
        if !c.text.is_empty() {
            ui.fonts.draw_in(
                &c.text,
                Rect::new(cr.x + 16.0, cr.y, cr.w - 19.0, cr.h),
                &st.color(t.text),
                Align::Left,
            );
        }
        x += w + 3.0;
    }
}

/// "Filip is waiting": a little face and the words over the top of the card.
fn badge(g: &GameScreen, ui: &mut Ui, t: &Theme, loc: &Locale, r: Rect, who: &str) {
    let (name, _) = wisher(g, loc, who);
    // a guest kind ("Medics") waits in the plural
    let many = !loc.has(&format!("regular.{who}.name"))
        && !g.world.staff().iter().any(|s| s.id.to_string() == who)
        && loc.has(&format!("guest.{who}.name"));
    let key = if many { "bux.wanted_many" } else { "bux.wanted" };
    let st = t.key().size(10.5).color(t.ink);
    // "Mrs Vera is waiting" when it fits the card; else the first name, else the name alone
    // (the amber badge with a face already says "wanted")
    // (a first name only for the crew: "Mrs Vera" must not become "Mrs")
    let staff = g.world.staff().iter().any(|s| s.id.to_string() == who);
    let first = if staff {
        name.split_whitespace().next().unwrap_or(&name).to_string()
    } else {
        name.clone()
    };
    let room = r.w - 30.0;
    let text = [
        loc.trf(key, &[("name", &name)]),
        loc.trf(key, &[("name", &first)]),
        name.clone(),
        first,
    ]
    .into_iter()
    .find(|s| ui.fonts.measure(s, &st) <= room)
    .unwrap_or_else(|| name.clone());
    let tw = ui.fonts.measure(&text, &st).min(room);
    let br = Rect::new(r.x + 3.0, r.y + 3.0, (tw + 26.0).min(r.w - 6.0), 17.0);
    fill_rrect(br, br.h / 2.0, t.amber);
    ui.avatar(vec2(br.x + 8.5, br.center_y()), 15.0, &name, None, false);
    ui.fonts.draw_in(
        &text,
        Rect::new(br.x + 19.0, br.y, br.w - 23.0, br.h),
        &st,
        Align::Left,
    );
}

/// The little "more this way" button at an end of the row.
fn arrow(t: &Theme, r: Rect, icon: Icon, hovered: bool) {
    let c = r.center();
    crate::ui::draw::fill_circle(
        c,
        11.0,
        if hovered {
            with_a(t.indigo, 0.85)
        } else {
            Color::new(0.16, 0.14, 0.27, 0.92)
        },
    );
    crate::ui::draw::stroke_circle(c, 11.0, 1.0, Color::new(1.0, 1.0, 1.0, 0.14));
    draw_icon(icon, c, 13.0, t.text);
}
