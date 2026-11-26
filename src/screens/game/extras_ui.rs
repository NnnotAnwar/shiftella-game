//! The small warm extras on the HUD (design/handson.md §6, §11.3): the atmosphere meter
//! (hover → the weakest part glows in the world), "Count the jar" when the tip jar reaches a
//! goal, napkins, the polaroid flash, and the toasts of the T5 systems the sim leaves to the UI.

use macroquad::prelude::*;

use super::world_view::sim;
use super::{Ctx, GameScreen, Notice};
use crate::content::Content;
use crate::content::handson::CozyComponent;
use crate::ui::draw::{fill_rrect, glow_circle, with_a};
use crate::ui::handson::{
    ATMO_SIZE, AtmosphereView, MomentAction, MomentOption, MomentView, atmosphere_meter,
    moment_card,
};
use crate::ui::{Button, Icon, RectExt, ToastKind, Ui, draw_icon};
use sim::{Command, Phase, SimEvent};

impl GameScreen {
    /// The T5 events the HUD answers with a toast (the sim toasts seasons, the cat, rituals
    /// and expired moments itself).
    pub fn extras_event(&mut self, content: &Content, e: &SimEvent) {
        let key = |k: &str, args: Vec<(String, String)>, icon: Icon, kind: ToastKind| Notice::Key {
            kind,
            icon,
            key: k.to_string(),
            args,
        };
        let n = match e {
            SimEvent::ThemeUnlocked { id, .. } => content.handson.decor.theme(id).map(|t| {
                key(
                    "ui.theme.unlocked",
                    vec![("name".into(), format!("@{}", t.name_key()))],
                    Icon::Sparkle,
                    ToastKind::Success,
                )
            }),
            SimEvent::ThankYouNote { id } => {
                self.hands.napkin = Some((id.clone(), self.time));
                None
            }
            // (with the book open the note shows on its page)
            SimEvent::MarginNote { ingredient, .. } if self.hands.book.is_none() => {
                content.handson.ingredient(ingredient).map(|i| {
                    key(
                        "book.margin_note",
                        vec![("ingredient".into(), format!("@{}", i.name_key()))],
                        Icon::Feather,
                        ToastKind::Info,
                    )
                })
            }
            SimEvent::IngredientUnlocked { id } => content.handson.ingredient(id).map(|i| {
                key(
                    "book.ingredient_unlocked",
                    vec![("ingredient".into(), format!("@{}", i.name_key()))],
                    Icon::Jar,
                    ToastKind::Success,
                )
            }),
            SimEvent::CozinessLevel { level } if self.world.phase() == Phase::Night => {
                self.hands.tut.bump("atmo_level");
                // only a real rise: not the opening minutes' settling, never back down
                let up = *level > self.hands.atmo_level && self.world.minute() > 20;
                self.hands.atmo_level = self.hands.atmo_level.max(*level);
                up.then(|| {
                    key(
                        "atmo.level_up",
                        vec![("level".into(), format!("@atmo.level.{level}"))],
                        Icon::Flame,
                        ToastKind::Success,
                    )
                })
            }
            SimEvent::TipGoalReached { .. } => Some(key(
                "ui.tipjar.pick",
                Vec::new(),
                Icon::Jar,
                ToastKind::Success,
            )),
            SimEvent::PayrollSent {
                total,
                tips_to_crew,
                ..
            } => Some(key(
                "app.payroll.sent",
                vec![
                    ("total".into(), format!("money:{total}")),
                    ("tips".into(), format!("money:{tips_to_crew}")),
                ],
                Icon::FileExport,
                ToastKind::Success,
            )),
            SimEvent::Reminder { key: k, args } => Some(Notice::Key {
                kind: ToastKind::Info,
                icon: Icon::Clock,
                key: k.clone(),
                args: args.clone(),
            }),
            SimEvent::HighlightCaptured { .. } => {
                self.hands.flash_at = self.time;
                None
            }
            SimEvent::CameraHint { tile: Some(t), .. } => {
                self.cam_target = Some(vec2(t.0 as f32 + 0.5, t.1 as f32) * crate::map::TILE);
                None
            }
            SimEvent::OpenApp { link } => {
                self.follow_link(*link);
                None
            }
            _ => None,
        };
        if let Some(n) = n {
            self.notices.push(n);
        }
    }
}

/// The atmosphere meter (below the clock card). Hover lights the weakest part in the world.
pub fn atmosphere(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let loc = cx.loc;
    let content = cx.content;
    let c = g.world.coziness().clone();
    let weights = &content.handson.decor.coziness.weights;
    let components = CozyComponent::ALL
        .iter()
        .enumerate()
        .map(|(i, comp)| {
            let w = weights
                .iter()
                .find(|(k, _)| k == comp)
                .map_or(1.0, |(_, w)| *w)
                .max(0.01);
            (
                loc.tr_owned(&comp.name_key()),
                (c.parts.get(i).copied().unwrap_or(0.0) / w).clamp(0.0, 1.0),
            )
        })
        .collect();
    let level = c.level.clamp(1, 5);
    let weakest = CozyComponent::ALL.get(c.weakest).copied();
    let view = AtmosphereView {
        value: c.value,
        level: (level - 1) as usize,
        level_name: loc.tr_owned(&format!("atmo.level.{level}")),
        title: loc.tr_owned("atmo.title"),
        components,
        tip: weakest.map(|w| loc.tr_owned(&w.tip_key())),
    };
    let r = Rect::new(12.0, 12.0 + 64.0 + 6.0, ATMO_SIZE.x, ATMO_SIZE.y);
    g.anchors.push(("hud.atmosphere", r));
    let resp = atmosphere_meter(ui, "atmo", r, &view);
    g.scene.cozy_hint = resp.hovered.then_some(c.weakest);
    // the polaroid flash: a little white blink beside the meter
    let f = 1.0 - (g.time - g.hands.flash_at) / 0.6;
    if (0.0..=1.0).contains(&f) && g.hands.flash_at > 0.0 {
        let p = vec2(r.right() + 18.0, r.center_y());
        glow_circle(
            p,
            16.0 * (0.6 + 0.4 * f),
            Color::new(1.0, 1.0, 1.0, 0.55 * f),
        );
        draw_icon(Icon::Camera, p, 14.0, Color::new(1.0, 1.0, 1.0, f));
    }
}

/// "Count the jar": a chip under the tip jar when a lifetime goal is reached, then a small
/// card with the two decor pieces to choose from.
pub fn tip_goal(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let content = cx.content;
    let loc = cx.loc;
    let Some((_, choices)) = g.world.tip_goal_pending(content) else {
        g.hands.jar_open = false;
        return;
    };
    if g.hands.autoplay.is_some() {
        if let Some(c) = choices.first().cloned() {
            let _ = g.command(content, Command::PickTipGoal { choice: c });
        }
        return;
    }
    let Some(jar) = g.anchors.iter().find(|a| a.0 == "hud.tipjar").map(|a| a.1) else {
        return;
    };
    let b = Button::new(loc.tr("ui.tipjar.count"))
        .primary()
        .icon(Icon::Jar);
    let bw = ui.button_width(&b);
    let below = g
        .anchors
        .iter()
        .find(|a| a.0 == "hud.lull")
        .map_or(jar.bottom(), |a| a.1.bottom());
    let br = Rect::new(jar.right() - bw, below + 6.0, bw, 28.0);
    g.anchors.push(("hud.jar_goal", br));
    let pulse = 0.5 + 0.5 * (g.time * 2.4).sin();
    fill_rrect(br.grow(3.0), 11.0, with_a(ui.theme.amber, 0.12 * pulse));
    if ui.button("jar_goal", br, &b).clicked {
        g.hands.jar_open = !g.hands.jar_open;
    }
    if !g.hands.jar_open {
        return;
    }
    let view = MomentView {
        icon: Icon::Jar,
        title: loc.tr_owned("ui.tipjar.count"),
        text: Some(loc.tr_owned("ui.tipjar.pick")),
        options: choices
            .iter()
            .map(|c| {
                MomentOption::new(loc.tr_owned(&format!("furniture.{c}.name"))).icon(Icon::Gem)
            })
            .collect(),
        expires: None,
    };
    let anchor = vec2(br.center().x, br.bottom());
    let avoid = super::hands_ui::hud_avoid(g);
    match moment_card(
        ui,
        "jar_card",
        anchor,
        &view,
        loc.tr("ui.common.close"),
        &avoid,
    ) {
        Some(MomentAction::Pick(i)) => {
            if let Some(c) = choices.get(i).cloned() {
                g.hands.jar_open = false;
                match g.command(content, Command::PickTipGoal { choice: c.clone() }) {
                    Ok(()) => g.notices.push(Notice::Key {
                        kind: ToastKind::Success,
                        icon: Icon::Build,
                        key: "ui.ticker.furniture_unlocked".into(),
                        args: vec![("item".into(), format!("@furniture.{c}.name"))],
                    }),
                    Err(e) => super::panels::command_error(g, &loc, &e),
                }
            }
        }
        Some(MomentAction::Dismiss) => g.hands.jar_open = false,
        None => {}
    }
}

/// A thank-you napkin that was just collected: a folded paper card under the tip jar.
pub fn napkin(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let Some((id, at)) = g.hands.napkin.clone() else {
        return;
    };
    let age = g.time - at;
    if age > 7.0 {
        g.hands.napkin = None;
        return;
    }
    let loc = cx.loc;
    let t = ui.theme.clone();
    let text = loc.tr_owned(&format!("thanks.{id}"));
    let s = ui.screen();
    let w = 260.0_f32.min(s.w - 24.0);
    let st = t.small().size(13.0).color(Color::from_hex(0x3a3450));
    let lines = ui.fonts.wrap(&text, w - 28.0, &st);
    let h = 34.0 + lines.len().min(4) as f32 * st.line_h() + 12.0;
    let appear = (age / 0.35).clamp(0.0, 1.0) * (1.0 - ((age - 6.4) / 0.6).clamp(0.0, 1.0));
    let top = g
        .anchors
        .iter()
        .filter(|a| matches!(a.0, "hud.tipjar" | "hud.lull" | "hud.jar_goal"))
        .map(|a| a.1.bottom())
        .fold(90.0, f32::max);
    let r = Rect::new(s.w - 12.0 - w, top + 8.0 - (1.0 - appear) * 8.0, w, h);
    g.anchors.push(("hud.napkin", r));
    crate::ui::draw::push_alpha(appear);
    crate::ui::draw::shadow_rrect(
        r,
        4.0,
        crate::ui::draw::Shadow::new(0.0, 4.0, 14.0, 0.0, Color::new(0.0, 0.0, 0.0, 0.4)),
    );
    fill_rrect(r, 4.0, Color::from_hex(0xF7F1E6));
    // the fold
    crate::ui::draw::hline(
        r.x + 10.0,
        r.right() - 10.0,
        r.y + r.h * 0.5,
        Color::new(0.0, 0.0, 0.0, 0.05),
    );
    draw_icon(
        Icon::Heart,
        vec2(r.x + 16.0, r.y + 16.0),
        12.0,
        Color::from_hex(0xC2566F),
    );
    ui.fonts.draw(
        loc.tr("ui.napkin.title"),
        r.x + 28.0,
        r.y + 20.0,
        &t.micro().color(Color::from_hex(0x8A8098)),
    );
    let mut y = r.y + 34.0 + st.line_h() * 0.75;
    for l in lines.iter().take(4) {
        crate::ui::handson::draw_handwritten(ui, l, r.x + 14.0, y, &st);
        y += st.line_h();
    }
    crate::ui::draw::pop_alpha();
}

/// The napkins beside the recipe book: a ribbon under the book, and a little board of the
/// thank-you napkins collected so far (newest first).
pub fn napkins_page(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let loc = cx.loc;
    let t = ui.theme.clone();
    let Some(book) = g.anchors.iter().find(|a| a.0 == "book").map(|a| a.1) else {
        return;
    };
    let napkins: Vec<String> = g.world.napkins().to_vec();
    let open = g.hands.book.as_ref().is_some_and(|b| b.napkins);
    let label = format!("{} · {}", loc.tr("book.tab.napkins"), napkins.len());
    let b = Button::new(&label)
        .secondary()
        .icon(Icon::Heart)
        .selected(open);
    let bw = ui.button_width(&b);
    let s = ui.screen();
    let br = Rect::new(
        book.right() - bw - 16.0,
        (book.bottom() + 6.0).min(s.h - 34.0),
        bw,
        28.0,
    );
    g.anchors.push(("book.napkins", br));
    if ui.button("book_napkins", br, &b).clicked
        && let Some(bk) = g.hands.book.as_mut()
    {
        bk.napkins = !bk.napkins;
    }
    if !open {
        return;
    }
    // over the right page of the book
    let r = Rect::new(
        book.center().x + 8.0,
        book.y + 44.0,
        book.w / 2.0 - 24.0,
        book.h - 58.0,
    );
    ui.region(r);
    ui.glass(r, &crate::ui::Glass::panel().solid().radius(10.0));
    let head = Rect::new(r.x + 14.0, r.y + 8.0, r.w - 28.0, 22.0);
    ui.fonts.draw_in(
        &loc.trf("ui.napkin.collected", &[("n", &napkins.len().to_string())]),
        head,
        &t.h2(),
        crate::ui::Align::Left,
    );
    let body = Rect::new(
        r.x + 10.0,
        head.bottom() + 6.0,
        r.w - 20.0,
        r.bottom() - head.bottom() - 16.0,
    );
    if napkins.is_empty() {
        ui.fonts.draw_wrapped(
            loc.tr("ui.napkin.empty"),
            body,
            &t.small().color(t.text_dim),
            crate::ui::Align::Left,
            Some(3),
        );
        return;
    }
    let st = t.small().size(12.0).color(Color::from_hex(0x3a3450));
    let cols = 2;
    let cw = (body.w - 8.0) / cols as f32;
    let ch = 70.0;
    let rows = napkins.len().div_ceil(cols);
    let sa = ui.begin_scroll("napkins", body, rows as f32 * (ch + 8.0));
    for (i, id) in napkins.iter().rev().enumerate() {
        let (cx0, cy0) = ((i % cols) as f32, (i / cols) as f32);
        let nr = Rect::new(
            body.x + cx0 * (cw + 8.0),
            sa.top() + cy0 * (ch + 8.0),
            cw,
            ch,
        );
        if !sa.visible(nr.y, nr.h) {
            continue;
        }
        fill_rrect(nr, 3.0, Color::from_hex(0xF7F1E6));
        crate::ui::draw::hline(
            nr.x + 6.0,
            nr.right() - 6.0,
            nr.center_y(),
            Color::new(0.0, 0.0, 0.0, 0.05),
        );
        let text = loc.tr_owned(&format!("thanks.{id}"));
        let lines = ui.fonts.wrap(&text, nr.w - 16.0, &st);
        let mut y = nr.y + 8.0 + st.line_h() * 0.8;
        for l in lines.iter().take(3) {
            crate::ui::handson::draw_handwritten(ui, l, nr.x + 8.0, y, &st);
            y += st.line_h();
        }
    }
    ui.end_scroll(sa);
}
