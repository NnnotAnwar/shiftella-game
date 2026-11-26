//! Grandma's attic (design/v0.2.md items 7 and 9): this week's second-hand pieces, each with
//! its one-line story, one click brings a piece down into the hall; what waits upstairs; and
//! grandma's back room (reopen once). A compact panel opened from the day card.

use macroquad::prelude::*;

use super::world_view::sim;
use super::{Ctx, GameScreen, Panel};
use crate::i18n::format_money;
use crate::map::TILE;
use crate::render::{FurnitureState, draw_furniture};
use crate::ui::draw::{fill_rrect, hline, with_a};
use crate::ui::{Align, Button, Glass, Icon, RectExt, ToastKind, Ui, draw_icon};

/// The panel width (the café stays visible around it).
const W: f32 = 600.0;
/// An attic card's height.
const CARD_H: f32 = 196.0;

pub fn panel(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let t = ui.theme.clone();
    let loc = cx.loc;
    let content = cx.content;
    let s = ui.screen();
    let w = (s.w - 24.0).min(W);
    let view = g.world.attic(content);
    // header + cards (+ the upstairs row) + the back room row, nothing more
    let h = 44.0 + 10.0 + CARD_H + 8.0 + if view.upstairs.is_empty() { 0.0 } else { 30.0 } + 70.0;
    let pr = Rect::new((s.w - w) / 2.0, 88.0, w, h.min(s.h - 160.0));
    g.anchors.push(("panel", pr));
    ui.region(pr);
    ui.glass(pr, &Glass::panel());
    let sub = if view.open {
        loc.trf("ui.attic.sub", &[("night", &view.next_night.to_string())])
    } else {
        loc.trf(
            "ui.attic.locked",
            &[("night", &view.next_night.to_string())],
        )
    };
    let (body, close) = ui.panel_header(pr, loc.tr("ui.attic.title"), None, Some(&sub));
    if close {
        g.panel = None;
        return;
    }
    let day = g.world.phase() == sim::Phase::Day;
    let c = body.inset_xy(12.0, 10.0);
    // --- this week's pieces --------------------------------------------------------------------
    let cards_h = CARD_H;
    let row = Rect::new(c.x, c.y, c.w, cards_h);
    if view.offers.is_empty() {
        let txt = if view.open {
            loc.tr("ui.attic.empty")
        } else {
            sub.as_str()
        };
        ui.fonts
            .draw_in(txt, row.with_h(40.0), &t.small(), Align::Center);
    }
    let n = content.build.attic.per_week.max(1) as f32;
    let gap = 8.0;
    let cw = ((row.w - gap * (n - 1.0)) / n).min(200.0);
    let used = view.offers.len() as f32 * (cw + gap) - gap;
    let x0 = row.x + ((row.w - used) / 2.0).max(0.0);
    for (i, o) in view.offers.iter().enumerate() {
        let r = Rect::new(x0 + i as f32 * (cw + gap), row.y, cw, cards_h);
        offer_card(g, ui, cx, r, o, day);
    }
    let mut y = row.bottom() + 8.0;
    // --- waiting upstairs (bought, not in the café now) ---------------------------------------
    if !view.upstairs.is_empty() {
        ui.fonts.draw_in(
            loc.tr("ui.attic.upstairs"),
            Rect::new(c.x, y, 120.0, 24.0),
            &t.micro(),
            Align::Left,
        );
        let mut x = c.x + 124.0;
        for o in &view.upstairs {
            let name = loc.tr_owned(&format!("furniture.{}.name", o.id));
            let b = Button::new(&name)
                .ghost()
                .icon(Icon::ChevronDown)
                .disabled(!day);
            let bw = ui.button_width(&b).min(c.right() - x);
            if bw < 60.0 {
                break;
            }
            if ui
                .button(("attic_up", o.id.as_str()), Rect::new(x, y, bw, 24.0), &b)
                .clicked
            {
                bring(g, cx, &o.id, &name);
            }
            x += bw + 6.0;
        }
        y += 30.0;
    }
    // --- the back room ------------------------------------------------------------------------
    hline(c.x, c.right(), y, t.divider);
    y += 8.0;
    let br = g.world.back_room(content);
    let rr = Rect::new(c.x, y, c.w, (pr.bottom() - 10.0 - y).max(40.0));
    draw_icon(Icon::Door, vec2(rr.x + 9.0, rr.y + 10.0), 16.0, t.amber);
    ui.fonts.draw_in(
        loc.tr("ui.backroom.title"),
        Rect::new(rr.x + 24.0, rr.y, rr.w - 200.0, 20.0),
        &t.strong(),
        Align::Left,
    );
    let state = if br.open {
        let night = g
            .world
            .build_state()
            .back_room
            .as_ref()
            .map(|b| b.night)
            .unwrap_or(br.from_night);
        loc.trf("ui.backroom.is_open", &[("night", &night.to_string())])
    } else if br.available {
        loc.tr_owned("ui.backroom.ready")
    } else {
        loc.trf(
            "ui.backroom.locked",
            &[("night", &br.from_night.to_string())],
        )
    };
    ui.fonts.draw_wrapped(
        &state,
        Rect::new(rr.x + 24.0, rr.y + 20.0, rr.w - 200.0, 34.0),
        &t.small().color(t.text_dim),
        Align::Left,
        Some(2),
    );
    if !br.open {
        let price = format_money(loc.lang, br.price as i64);
        let label = format!("{} · {price}", loc.tr("ui.backroom.open"));
        let afford = g.world.money() >= br.price as i64;
        let b = Button::new(&label)
            .primary()
            .icon(Icon::Door)
            .disabled(!br.available || !day || !afford);
        let bw = ui.button_width(&b).min(180.0);
        let bt = Rect::new(rr.right() - bw, rr.y + 6.0, bw, 30.0);
        let resp = ui.button("backroom_open", bt, &b);
        if !day {
            ui.tooltip(&resp, loc.tr("ui.attic.day_only"));
        }
        if resp.clicked {
            match g.command(content, sim::Command::OpenBackRoom) {
                Ok(()) => {
                    cx.audio.play_sfx(crate::audio::Sfx::DoorBell);
                    // show the room: the camera eases over to it
                    if let Some(b) = g.world.build_state().back_room.as_ref() {
                        let (x, y) = b.center();
                        g.cam.center = vec2((x as f32 + 0.5) * TILE, (y as f32 + 0.5) * TILE);
                    }
                    g.panel = None;
                }
                Err(e) => super::panels::command_error(g, &loc, &e),
            }
        }
    }
}

fn offer_card(
    g: &mut GameScreen,
    ui: &mut Ui,
    cx: &mut Ctx,
    r: Rect,
    o: &sim::AtticOffer,
    day: bool,
) {
    let t = ui.theme.clone();
    let loc = cx.loc;
    let content = cx.content;
    ui.glass(r, &Glass::card());
    let resp = ui.interact(ui.id(("attic_card", o.id.as_str())), r);
    if resp.hovered {
        fill_rrect(r, 10.0, Color::new(1.0, 1.0, 1.0, 0.04));
    }
    let Some(d) = content.furniture_def(&o.id) else {
        return;
    };
    // the piece, warm in the attic's lamp light
    let prev = Rect::new(r.x + 10.0, r.y + 6.0, r.w - 20.0, 58.0);
    fill_rrect(
        prev.inset_xy(10.0, 4.0),
        8.0,
        with_a(t.amber, if resp.hovered { 0.10 } else { 0.06 }),
    );
    draw_preview(&d.sprite, d.size, prev, ui.time() as f32);
    let name = loc.tr_owned(&d.name_key());
    ui.fonts.draw_in(
        &name,
        Rect::new(r.x + 8.0, prev.bottom() + 2.0, r.w - 16.0, 18.0),
        &t.small_medium(),
        Align::Center,
    );
    let story = loc.tr_owned(&format!("attic.{}.story", o.id));
    let story_style = t.micro().color(t.text_dim);
    ui.fonts.draw_wrapped(
        &story,
        Rect::new(
            r.x + 10.0,
            prev.bottom() + 22.0,
            r.w - 20.0,
            story_style.line_h() * 3.0,
        ),
        &story_style,
        Align::Center,
        Some(3),
    );
    let mut tip = loc.tr_owned(&d.desc_key());
    tip.push('\n');
    tip.push_str(&story);
    ui.tooltip(&resp, &tip);
    // price (and the price of a new one, crossed out) + one click
    let py = r.bottom() - 56.0;
    let price = format_money(loc.lang, o.price as i64);
    let newp = loc.trf(
        "ui.attic.new_price",
        &[("price", &format_money(loc.lang, o.new_price as i64))],
    );
    let afford = g.world.money() >= o.price as i64;
    let pw = ui.fonts.measure(&price, &t.key());
    let nw = ui.fonts.measure(&newp, &t.micro());
    let x0 = r.center().x - (pw + 8.0 + nw) / 2.0;
    ui.fonts.draw_in(
        &price,
        Rect::new(x0, py, pw + 2.0, 18.0),
        &t.key().color(if afford { t.amber } else { t.red_400 }),
        Align::Left,
    );
    let nr = Rect::new(x0 + pw + 8.0, py, nw + 2.0, 18.0);
    ui.fonts
        .draw_in(&newp, nr, &t.micro().color(t.text_disabled), Align::Left);
    hline(
        nr.x,
        nr.x + nw,
        nr.center_y() + 1.0,
        with_a(t.text_disabled, 0.8),
    );
    let b = Button::new(loc.tr("ui.attic.bring"))
        .primary()
        .icon(Icon::ChevronDown)
        .disabled(!day || !afford);
    let br = Rect::new(r.x + 10.0, r.bottom() - 34.0, r.w - 20.0, 26.0);
    let bresp = ui.button(("attic_buy", o.id.as_str()), br, &b);
    if !day {
        ui.tooltip(&bresp, loc.tr("ui.attic.day_only"));
    }
    if bresp.clicked {
        bring(g, cx, &o.id, &name);
    }
}

/// One click: buy (or bring back down) and say where it went.
fn bring(g: &mut GameScreen, cx: &mut Ctx, id: &str, name: &str) {
    let loc = cx.loc;
    match g.command(
        cx.content,
        sim::Command::BuyAtticPiece { id: id.to_string() },
    ) {
        Ok(()) => {
            cx.audio.play_sfx_vol(crate::audio::Sfx::Footsteps, 0.8);
            g.toast(
                ToastKind::Success,
                Icon::Home,
                loc.trf("ui.attic.placed", &[("item", name)]),
                None,
            );
        }
        Err(e) if e.starts_with("Occupied") => {
            g.toast(
                ToastKind::Warning,
                Icon::Warning,
                loc.tr_owned("ui.attic.no_room"),
                None,
            );
        }
        Err(e) => super::panels::command_error(g, &loc, &e),
    }
}

/// A furniture sprite scaled into a UI rect (the UI camera is active).
fn draw_preview(sprite: &str, size: (u32, u32), r: Rect, t: f32) {
    let (w, h) = (size.0.max(1) as f32, size.1.max(1) as f32);
    let k = (r.w / (w * TILE)).min(r.h / ((h + 1.4) * TILE)).min(0.7);
    let origin = vec2(r.center().x / k - w * TILE / 2.0, r.bottom() / k - h * TILE);
    let st = FurnitureState {
        active: true,
        t,
        ..Default::default()
    };
    let gl = unsafe { get_internal_gl() };
    gl.quad_gl
        .push_model_matrix(Mat4::from_scale(vec3(k, k, 1.0)));
    draw_furniture(sprite, origin, (w as u8, h as u8), &st);
    let gl = unsafe { get_internal_gl() };
    gl.quad_gl.pop_model_matrix();
}

/// Opens / closes the attic (the day card's button).
pub fn toggle(g: &mut GameScreen) {
    g.panel = if g.panel == Some(Panel::Attic) {
        None
    } else {
        Some(Panel::Attic)
    };
}
