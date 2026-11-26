//! HUD: clock card (top-left), ticker (top-centre), resources (top-right), the dock
//! (bottom-left) and the speech bubble over a guest.

use macroquad::prelude::*;

use super::world_view::{self as wv, sim};
use super::{Ctx, GameScreen, Panel};
use crate::content::Content;
use crate::i18n::{format_decimal, format_money, format_money_delta};
use crate::settings::Locale;
use crate::ui::draw::{self, Paint, fill_rrect, glow_circle, hline, with_a};
use crate::ui::{
    Align, Anchor, Button, Flow, Glass, Icon, IconButton, RectExt, Ui, anchored, draw_icon,
};
use sim::{AgentRef, Phase};

/// World point → UI units.
pub fn world_to_ui(g: &GameScreen, ui: &Ui, p: Vec2) -> Vec2 {
    let s = (p - g.cam.center) * g.cam.zoom + vec2(screen_width(), screen_height()) / 2.0;
    s / ui.scale()
}

pub fn draw(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    clock_card(g, ui, cx);
    ticker(g, ui, cx);
    resources(g, ui, cx);
    dock(g, ui, cx);
}

fn weekday_short(w: crate::content::Weekday) -> String {
    format!("ui.weekday.{}", w.id())
}

fn clock_card(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let t = ui.theme.clone();
    let loc = cx.loc;
    let r = anchored(ui.screen(), Anchor::TopLeft, vec2(340.0, 64.0), 12.0);
    g.anchors.push(("hud.clock", r));
    ui.glass(r, &Glass::hud());
    let speed = g.speed;
    let w = &g.world;
    let day = w.phase() == Phase::Day;
    draw_icon(
        if day { Icon::Sun } else { Icon::Moon },
        vec2(r.x + 24.0, r.y + 26.0),
        18.0,
        if day { t.amber } else { t.indigo_300 },
    );
    let clock = if day {
        "21:30".to_string()
    } else {
        wv::clock_of_minute(w.minute())
    };
    let cw = ui.fonts.draw(&clock, r.x + 42.0, r.y + 34.0, &t.clock());
    let wd = loc.tr_owned(&weekday_short(w.weekday()));
    let sub = loc.trf(
        "ui.hud.day_night",
        &[("weekday", &wd), ("night", &w.night().to_string())],
    );
    ui.fonts.draw_in(
        &sub,
        Rect::new(
            r.x + 50.0 + cw,
            r.y + 20.0,
            104.0 - (cw - 76.0).max(0.0),
            18.0,
        ),
        &t.small(),
        Align::Left,
    );
    // speed buttons
    let mut speed_click = None;
    let mut fr = Flow::row_rev(Rect::new(r.x, r.y + 12.0, r.w - 12.0, 28.0), 2.0);
    let tips = [
        loc.tr("ui.speed.pause"),
        loc.tr("ui.speed.normal"),
        loc.tr("ui.speed.fast"),
        loc.tr("ui.speed.faster"),
    ];
    for (i, ic) in [Icon::Faster, Icon::Fast, Icon::Play, Icon::Pause]
        .iter()
        .enumerate()
    {
        let idx = 3 - i;
        let b = IconButton::ghost()
            .size(16.0)
            .selected(speed as usize == idx)
            .disabled(day)
            .tooltip(tips[idx]);
        if ui
            .icon_button(("speed", idx), fr.next(28.0), *ic, &b)
            .clicked
        {
            speed_click = Some(idx as u8);
        }
    }
    // night progress with train markers
    let bar = Rect::new(r.x + 14.0, r.bottom() - 14.0, r.w - 28.0, 5.0);
    let prog = if day { 0.0 } else { w.night_progress() };
    fill_rrect(bar, 2.5, Color::new(1.0, 1.0, 1.0, 0.08));
    let fb = Rect::new(bar.x, bar.y, bar.w * prog, bar.h);
    if prog > 0.0 {
        fill_rrect(fb, 2.5, Paint::horizontal(bar, t.violet, t.indigo_300));
    }
    let dawn = Rect::new(bar.x + bar.w * 0.875, bar.y, bar.w * 0.125, bar.h);
    fill_rrect(
        dawn,
        2.5,
        Paint::horizontal(dawn, with_a(t.amber, 0.0), with_a(t.amber, 0.45)),
    );
    for tr in &cx.content.trains.trains {
        let m = tr.arrives.minutes_since_open();
        if !(0..480).contains(&m) {
            continue;
        }
        let x = bar.x + bar.w * m as f32 / 480.0;
        draw::fill_convex(
            &[
                vec2(x - 3.0, bar.y - 6.0),
                vec2(x + 3.0, bar.y - 6.0),
                vec2(x, bar.y - 2.0),
            ],
            Paint::Solid(with_a(t.amber, 0.8)),
        );
    }
    if !day {
        let knob = vec2(fb.right(), bar.center_y());
        glow_circle(knob, 10.0, with_a(t.indigo_300, 0.5));
        draw::fill_circle(knob, 4.5, WHITE);
    }
    if let Some(s) = speed_click {
        g.set_speed(s);
    }
}

fn ticker(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let t = ui.theme.clone();
    let loc = cx.loc;
    let s = ui.screen();
    let w = (s.w - 2.0 * 364.0).clamp(220.0, 460.0);
    let er = anchored(s, Anchor::Top, vec2(w, 32.0), 12.0);
    g.anchors.push(("hud.ticker", er));
    // glass before interact: a region declared after `interact` blocks it next frame
    ui.glass(er, &Glass::hud().radius(8.0));
    let resp = ui.interact(ui.id("ticker"), er);
    if resp.hovered {
        fill_rrect(er, 8.0, with_a(t.indigo, 0.08));
    }
    let (icon, time, text) = match g.ticker.last() {
        Some(tk) => (tk.icon, tk.time.clone(), tk.text.clone()),
        None => (
            Icon::Moon,
            String::new(),
            if g.world.phase() == Phase::Day {
                loc.tr_owned("ui.ticker.day")
            } else {
                loc.tr_owned("ui.log.empty")
            },
        ),
    };
    draw_icon(icon, vec2(er.x + 18.0, er.center_y()), 16.0, t.violet_400);
    let mut x = er.x + 34.0;
    if !time.is_empty() {
        let st = t.number().size(12.0).color(t.text_disabled);
        x += ui.fonts.draw(&time, x, ui.fonts.baseline_in(er, &st), &st) + 8.0;
    }
    ui.label(
        Rect::new(x, er.y, er.right() - 44.0 - x, er.h),
        &text,
        &t.body_medium(),
        Align::Left,
    );
    draw::vline(er.right() - 40.0, er.y + 8.0, er.bottom() - 8.0, t.divider);
    ui.keycap(vec2(er.right() - 30.0, er.center_y()), "L", false);
    if resp.clicked {
        g.panel = if g.panel == Some(Panel::Log) {
            None
        } else {
            Some(Panel::Log)
        };
    }
    ui.tooltip(&resp, loc.tr("ui.ticker.open_log"));
}

/// Seats in the café (guest capacity).
pub fn capacity(content: &Content, w: &sim::World) -> u32 {
    w.furniture()
        .iter()
        .filter_map(|p| content.furniture_def(&p.def))
        .map(|d| d.seats)
        .sum()
}

fn resources(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let t = ui.theme.clone();
    let loc = cx.loc;
    let w = &g.world;
    let rr = anchored(ui.screen(), Anchor::TopRight, vec2(340.0, 64.0), 12.0);
    g.anchors.push(("hud.money", rr));
    ui.glass(rr, &Glass::hud());
    let base = rr.y + 30.0;
    draw_icon(Icon::Money, vec2(rr.x + 22.0, base - 5.0), 18.0, t.amber);
    let money = format_money(loc.lang, w.money());
    let mw = ui.fonts.draw(
        &money,
        rr.x + 38.0,
        base,
        &t.h1()
            .color(if w.money() < 0 { t.red_400 } else { t.amber })
            .tabular(),
    );
    let money_r = Rect::new(rr.x + 8.0, rr.y + 8.0, mw + 40.0, 30.0);
    let mresp = ui.interact(ui.id("money_tip"), money_r);
    let debt_tip = if w.debt() > 0 {
        loc.trf(
            "ui.hud.debt",
            &[("money", &format_money(loc.lang, w.debt()))],
        )
    } else {
        loc.tr_owned("ui.hud.money")
    };
    ui.tooltip(&mresp, &debt_tip);
    let mut x = rr.x + 46.0 + mw + 12.0;
    draw_icon(Icon::Star, vec2(x, base - 5.0), 16.0, t.amber);
    x += 12.0;
    let stars = w.reputation() / 20.0;
    let rep = format_decimal(loc.lang, stars, 1);
    let rw = ui.fonts.draw(&rep, x, base, &t.h2().tabular());
    let rresp = ui.interact(
        ui.id("rep_tip"),
        Rect::new(x - 20.0, rr.y + 8.0, rw + 24.0, 30.0),
    );
    ui.tooltip(&rresp, loc.tr("ui.hud.reputation"));
    let delta = (w.reputation() - w.tonight().reputation_start) / 20.0;
    if w.phase() == Phase::Night && delta.abs() >= 0.05 {
        let txt = format!(
            "{}{}",
            if delta > 0.0 { "+" } else { "−" },
            format_decimal(loc.lang, delta.abs(), 1)
        );
        let c = if delta > 0.0 {
            t.emerald_400
        } else {
            t.red_400
        };
        ui.fonts.draw(&txt, x + rw + 4.0, base, &t.key().color(c));
    }
    x = rr.right() - 86.0;
    draw_icon(Icon::User, vec2(x, base - 5.0), 16.0, t.text_dim);
    let cap = capacity(cx.content, w);
    let gv = loc.trf(
        "ui.hud.guests_value",
        &[
            ("count", &w.guests_inside().to_string()),
            ("cap", &cap.to_string()),
        ],
    );
    ui.fonts.draw(&gv, x + 12.0, base, &t.h2().tabular());
    let gresp = ui.interact(
        ui.id("guests_tip"),
        Rect::new(x - 10.0, rr.y + 8.0, 84.0, 30.0),
    );
    ui.tooltip(&gresp, loc.tr("ui.hud.guests"));
    hline(rr.x + 12.0, rr.right() - 12.0, rr.y + 40.0, t.divider);
    let small = t.small();
    let net = w.tonight().revenue + w.tonight().tips;
    let line = if w.phase() == Phase::Night {
        loc.trf(
            "ui.hud.tonight",
            &[("money", &format_money_delta(loc.lang, net))],
        )
    } else {
        loc.trf(
            "ui.hud.debt",
            &[("money", &format_money(loc.lang, w.debt()))],
        )
    };
    ui.fonts.draw_in(
        &line,
        Rect::new(rr.x + 14.0, rr.y + 44.0, 124.0, 16.0),
        &small.color(if w.phase() == Phase::Night {
            t.emerald_400
        } else {
            t.text_dim
        }),
        Align::Left,
    );
    let q = loc.trf("ui.hud.queue", &[("n", &w.queue_len().to_string())]);
    ui.fonts.draw_in(
        &q,
        Rect::new(rr.x + 140.0, rr.y + 44.0, 64.0, 16.0),
        &small,
        Align::Left,
    );
    let wet = w.weather().is_wet();
    draw_icon(
        if wet { Icon::Rain } else { Icon::Moon },
        vec2(rr.x + 214.0, rr.y + 52.0),
        14.0,
        t.indigo_300,
    );
    ui.fonts.draw_in(
        loc.tr(wv::weather_key(w.weather())),
        Rect::new(rr.x + 226.0, rr.y + 44.0, rr.w - 236.0, 16.0),
        &small,
        Align::Left,
    );
}

fn dock(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let loc = cx.loc;
    let pending = pending_requests(&g.world);
    let items = [
        (Icon::Crew, "ui.toolbar.crew", "C", Panel::Crew),
        (Icon::Priorities, "ui.duties.title", "H", Panel::Duties),
        (Icon::Phone, "ui.toolbar.app", "T", Panel::App),
        (Icon::Build, "ui.toolbar.build", "B", Panel::Build),
        (Icon::Log, "ui.toolbar.log", "L", Panel::Log),
    ];
    let anchor_names = [
        "dock.crew",
        "dock.duties",
        "dock.app",
        "dock.build",
        "dock.log",
    ];
    let widths: Vec<f32> = items
        .iter()
        .map(|(ic, l, k, _)| ui.button_width(&Button::new(loc.tr(l)).ghost().icon(*ic).shortcut(k)))
        .collect();
    let book_w = ui.button_width(
        &Button::new(loc.tr("ui.toolbar.book"))
            .ghost()
            .icon(Icon::Book)
            .shortcut("K"),
    );
    let total: f32 =
        widths.iter().sum::<f32>() + book_w + 4.0 * (items.len() as f32 + 1.0) + 36.0 + 32.0;
    let tb = anchored(ui.screen(), Anchor::BottomLeft, vec2(total, 36.0), 12.0);
    ui.glass(tb, &Glass::hud());
    let mut fr = Flow::row(tb.inset(4.0), 4.0);
    for (i, ((ic, l, k, p), w)) in items.iter().zip(widths).enumerate() {
        let r = fr.next(w);
        g.anchors.push((anchor_names[i], r));
        if ui
            .button(
                ("dock", i),
                r,
                &Button::new(loc.tr(l))
                    .ghost()
                    .icon(*ic)
                    .shortcut(k)
                    .selected(g.panel == Some(*p)),
            )
            .clicked
        {
            g.panel = if g.panel == Some(*p) { None } else { Some(*p) };
        }
        if *p == Panel::App && pending > 0 {
            ui.badge(vec2(r.x + 22.0, r.y + 4.0), pending);
        }
    }
    // the recipe book is an overlay, not a panel
    let book = Button::new(loc.tr("ui.toolbar.book"))
        .ghost()
        .icon(Icon::Book)
        .shortcut("K")
        .selected(g.hands.book.is_some());
    let bw = ui.button_width(&book);
    let br = fr.next(bw);
    g.anchors.push(("dock.book", br));
    if ui.button(("dock", "book"), br, &book).clicked {
        g.open_book(None);
    }
    let help_tip = loc.tr("ui.help.tip");
    if ui
        .icon_button(
            "help",
            fr.next(28.0),
            Icon::Info,
            &IconButton::ghost()
                .tooltip(help_tip)
                .selected(g.panel == Some(Panel::Help)),
        )
        .clicked
    {
        g.panel = if g.panel == Some(Panel::Help) {
            None
        } else {
            Some(Panel::Help)
        };
    }
    let menu_tip = loc.tr("ui.hud.menu_tip");
    if ui
        .icon_button(
            "menu",
            fr.next(28.0),
            Icon::Settings,
            &IconButton::ghost().tooltip(menu_tip),
        )
        .clicked
    {
        g.pause = Some(super::pause::PauseMenu::default());
    }
}

/// Requests waiting for the manager in the Shiftella app.
pub fn pending_requests(w: &sim::World) -> u32 {
    let a = w.app();
    let p = |s: &sim::RequestStatus| *s == sim::RequestStatus::Pending;
    (a.swaps.iter().filter(|s| p(&s.status)).count()
        + a.time_off.iter().filter(|s| p(&s.status)).count()
        + w.early_leave_requests()
            .iter()
            .filter(|s| p(&s.status))
            .count()) as u32
}

/// Fallback bubble text for a guest who is not a regular.
pub fn guest_line(g: &GameScreen, content: &Content, guest: u32, loc: &Locale) -> String {
    let w = &g.world;
    let Some(gs) = w.guests().iter().find(|x| x.id == guest) else {
        return String::new();
    };
    let Some(p) = w.parties().iter().find(|p| p.id == gs.party) else {
        return String::new();
    };
    let _ = content;
    let name = wv::guest_one(loc, &p.archetype);
    use sim::PartyState as S;
    let state = match p.state {
        S::Queue | S::WaitOrder => loc.tr_owned("ui.guest.waiting"),
        S::WaitFood => {
            let wish = p
                .wishes
                .first()
                .map(|m| loc.tr_owned(&format!("menu.{m}.name")))
                .unwrap_or_default();
            loc.trf("ui.guest.wants", &[("item", &wish)])
        }
        S::Eating => {
            if p.satisfaction >= 70.0 {
                loc.tr_owned("ui.guest.happy")
            } else if p.satisfaction >= 40.0 {
                loc.tr_owned("ui.guest.content")
            } else {
                loc.tr_owned("ui.guest.unhappy")
            }
        }
        S::Arriving | S::Leaving => String::new(),
    };
    if state.is_empty() {
        name
    } else {
        format!("{name} · {state}")
    }
}

/// The speech bubble over a guest (hover = name/mood; click = mood line + Talk).
pub fn bubble(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let content = cx.content;
    let loc = cx.loc;
    // hover bubble for guests when nothing is pinned
    if g.bubble.as_ref().is_none_or(|b| !b.pinned)
        && let Some(AgentRef::Guest(id)) = g.hovered
        && g.bubble
            .as_ref()
            .is_none_or(|b| b.agent != AgentRef::Guest(id))
    {
        let regular = is_regular(&g.world, id);
        let text = if regular {
            g.world
                .ambient_line_for(content, id)
                .map(|k| loc.tr_owned(&k))
                .unwrap_or_else(|| guest_line(g, content, id, &loc))
        } else {
            guest_line(g, content, id, &loc)
        };
        if !text.is_empty() {
            g.bubble = Some(super::Bubble {
                agent: AgentRef::Guest(id),
                text,
                talk: None,
                pinned: false,
                until: g.time + 0.6,
            });
        }
    }
    let Some(mut b) = g.bubble.clone() else {
        return;
    };
    b.text = super::resolve_text(&loc, &b.text);
    if b.text.is_empty() && b.talk.is_none() {
        return;
    }
    if g.time > b.until && !(g.hovered == Some(b.agent) && !b.pinned) {
        g.bubble = None;
        return;
    }
    let Some(head) = g.scene.head_of(b.agent, g.time) else {
        g.bubble = None;
        return;
    };
    let t = ui.theme.clone();
    let anchor = world_to_ui(g, ui, head);
    let style = t.body();
    let max_w = 260.0;
    let lines = ui.fonts.wrap(&b.text, max_w, &style);
    let text_w = lines
        .iter()
        .map(|l| ui.fonts.measure(l, &style))
        .fold(0.0, f32::max);
    let talk_label = loc.tr("ui.guest.talk");
    let talk_w = if b.talk.is_some() {
        ui.button_width(&Button::new(talk_label).primary().icon(Icon::Note))
    } else {
        0.0
    };
    let w = text_w.max(talk_w) + 24.0;
    let lh = style.line_h();
    let h = lines.len() as f32 * lh + 18.0 + if b.talk.is_some() { 34.0 } else { 0.0 };
    let mut r = Rect::new(anchor.x - w / 2.0, anchor.y - h - 12.0, w, h);
    // never under the top HUD / the ticket rail: then the bubble hangs below the head
    let top = g
        .anchors
        .iter()
        .filter(|a| matches!(a.0, "hud.clock" | "hud.ticker" | "hud.money" | "rail"))
        .filter(|a| a.1.x < r.right() && a.1.right() > r.x)
        .map(|a| a.1.bottom())
        .fold(0.0, f32::max);
    // keep it on screen (a guest at the edge of the view)
    let sw = ui.screen().w;
    r.x = r.x.clamp(8.0, (sw - r.w - 8.0).max(8.0));
    let below = r.y < top + 6.0;
    if below {
        r.y = anchor.y + 70.0;
    }
    ui.glass(r, &Glass::tooltip().radius(10.0));
    // tail (towards the head, above or below)
    if below {
        draw::fill_convex(
            &[
                vec2(anchor.x - 7.0, r.y + 1.0),
                vec2(anchor.x, r.y - 9.0),
                vec2(anchor.x + 7.0, r.y + 1.0),
            ],
            Paint::Solid(t.glass_top),
        );
    } else {
        draw::fill_convex(
            &[
                vec2(anchor.x - 7.0, r.bottom() - 1.0),
                vec2(anchor.x + 7.0, r.bottom() - 1.0),
                vec2(anchor.x, r.bottom() + 9.0),
            ],
            Paint::Solid(t.glass_bottom),
        );
    }
    let mut y = r.y + 9.0 + lh * 0.78;
    for l in &lines {
        ui.fonts.draw(l, r.x + 12.0, y, &style.color(t.text));
        y += lh;
    }
    if let (Some(id), AgentRef::Guest(guest)) = (&b.talk, b.agent) {
        let br = Rect::new(r.x + 12.0, r.bottom() - 34.0, talk_w, 26.0);
        if ui
            .button(
                ("talk", guest),
                br,
                &Button::new(talk_label).primary().icon(Icon::Note),
            )
            .clicked
        {
            let _ = id;
            let _ = g.command(content, sim::Command::Talk { guest });
            g.bubble = None;
        }
    }
}

fn is_regular(w: &sim::World, guest: u32) -> bool {
    w.guests()
        .iter()
        .find(|x| x.id == guest)
        .and_then(|gs| w.parties().iter().find(|p| p.id == gs.party))
        .is_some_and(|p| p.regular.is_some())
}
