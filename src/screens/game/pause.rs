//! Esc pause menu: resume, settings, save (3 slots), load, main menu, quit.
//! One click per action — saving over a slot shows a toast, no confirm popups.

use macroquad::prelude::*;

use super::{Ctx, GameExit, GameScreen};
use crate::save::{SaveMeta, Slot};
use crate::settings::{Locale, settings_panel};
use crate::ui::{Align, Button, Glass, Icon, RectExt, Ui, draw_icon};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Main,
    Settings,
    Save,
    Load,
}

#[derive(Debug, Clone, Default)]
pub struct PauseMenu {
    pub page: Page,
    /// Slot list read when the page opened.
    slots: Option<Vec<(Slot, Option<SaveMeta>)>>,
}

fn slot_line(loc: &Locale, m: &SaveMeta) -> String {
    let mode = loc.tr_owned(&format!("ui.save.mode.{}", m.mode));
    let time = if m.clock == "day" {
        loc.tr_owned("ui.save.day")
    } else {
        m.clock.clone()
    };
    loc.trf(
        "ui.save.slot",
        &[
            ("mode", &mode),
            ("night", &m.night.to_string()),
            ("time", &time),
        ],
    )
}

fn slot_name(loc: &Locale, s: Slot) -> String {
    match s {
        Slot::Auto => loc.tr_owned("ui.save.autosave"),
        Slot::Manual(n) => loc.trf("ui.save.slot_n", &[("n", &n.to_string())]),
    }
}

pub fn draw(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let t = ui.theme.clone();
    let loc = cx.loc;
    let s = ui.screen();
    draw_rectangle(0.0, 0.0, s.w, s.h, Color::new(0.03, 0.02, 0.08, 0.55));
    // block clicks to the world
    ui.region(s);
    let page = g.pause.as_ref().map(|p| p.page).unwrap_or_default();
    match page {
        Page::Main => {
            let (w, h) = (300.0, 366.0);
            let pr = Rect::new((s.w - w) / 2.0, (s.h - h) / 2.0, w, h);
            ui.glass(pr, &Glass::panel().radius(14.0));
            let c = pr.inset(20.0);
            ui.fonts.draw_centered(
                loc.tr("ui.pause.title"),
                pr.center().x,
                c.y + 18.0,
                &t.title(),
            );
            let items: [(&str, Icon, u8); 7] = [
                ("ui.pause.resume", Icon::Play, 0),
                ("ui.help.title", Icon::Info, 6),
                ("ui.pause.settings", Icon::Settings, 1),
                ("ui.pause.save", Icon::Check, 2),
                ("ui.pause.load", Icon::Undo, 3),
                ("ui.pause.main_menu", Icon::Home, 4),
                ("ui.pause.quit", Icon::Door, 5),
            ];
            let mut y = c.y + 40.0;
            for (key, icon, i) in items {
                let r = Rect::new(c.x, y, c.w, 36.0);
                y += 42.0;
                let mut b = Button::new(loc.tr(key)).icon(icon);
                if i == 0 {
                    b = b.primary().shortcut("Esc");
                }
                if ui.button(("pause", i), r, &b).clicked {
                    match i {
                        0 => g.pause = None,
                        1 => set_page(g, Page::Settings, cx),
                        2 => set_page(g, Page::Save, cx),
                        3 => set_page(g, Page::Load, cx),
                        4 => {
                            autosave(g, cx);
                            g.exit = Some(GameExit::MainMenu);
                        }
                        6 => {
                            g.pause = None;
                            g.panel = Some(super::Panel::Help);
                        }
                        _ => {
                            autosave(g, cx);
                            g.exit = Some(GameExit::Quit);
                        }
                    }
                }
            }
        }
        Page::Settings => {
            let r = crate::settings::panel_rect(vec2(s.w, s.h));
            let resp = settings_panel(ui, &loc, cx.settings, r);
            if resp.changed {
                if let Err(e) = cx.settings.save() {
                    eprintln!("warning: settings not saved: {e}");
                }
                cx.audio
                    .set_volumes(cx.settings.music_volume, cx.settings.sfx_volume);
                ui.set_scale(cx.settings.ui_scale);
                g.player.apply_settings(cx.settings);
                cx.settings.apply_window();
            }
            if resp.replay_tutorial {
                g.start_tutorial();
                g.pause = None;
            } else if resp.close {
                set_page(g, Page::Main, cx);
            }
        }
        Page::Save | Page::Load => slots_page(g, ui, cx, page),
    }
}

fn set_page(g: &mut GameScreen, page: Page, cx: &Ctx) {
    if let Some(p) = &mut g.pause {
        p.page = page;
        p.slots = Some(
            Slot::all()
                .into_iter()
                .map(|s| (s, cx.store.read(s).ok().map(|f| f.meta)))
                .collect(),
        );
    }
}

fn autosave(g: &mut GameScreen, cx: &Ctx) {
    if std::env::var("SHIFTELLA_NO_AUTOSAVE").is_ok() || g.world.ending().is_some() {
        return;
    }
    if let Err(e) = g.save(cx.content, cx.store, Slot::Auto) {
        eprintln!("warning: autosave failed: {e}");
    }
}

fn slots_page(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx, page: Page) {
    let t = ui.theme.clone();
    let loc = cx.loc;
    let s = ui.screen();
    let (w, h) = (420.0, 330.0);
    let pr = Rect::new((s.w - w) / 2.0, (s.h - h) / 2.0, w, h);
    ui.glass(pr, &Glass::panel().radius(14.0));
    let title = if page == Page::Save {
        loc.tr("ui.pause.save")
    } else {
        loc.tr("ui.pause.load")
    };
    let (body, close) = ui.panel_header(pr, title, None, None);
    if close {
        set_page(g, Page::Main, cx);
        return;
    }
    let c = body.inset(14.0);
    let slots = g
        .pause
        .as_ref()
        .and_then(|p| p.slots.clone())
        .unwrap_or_default();
    let mut y = c.y;
    for (slot, meta) in slots {
        if page == Page::Save && slot == Slot::Auto {
            continue;
        }
        let r = Rect::new(c.x, y, c.w, 54.0);
        y += 60.0;
        let resp = ui.interact(ui.id(("slot", format!("{slot:?}"))), r);
        let enabled = page == Page::Save || meta.is_some();
        crate::ui::draw::fill_rrect(
            r,
            10.0,
            if resp.hovered && enabled {
                crate::ui::draw::with_a(t.indigo, 0.22)
            } else {
                Color::new(1.0, 1.0, 1.0, 0.04)
            },
        );
        draw_icon(
            if slot == Slot::Auto {
                Icon::Clock
            } else {
                Icon::Note
            },
            vec2(r.x + 22.0, r.center_y()),
            16.0,
            t.indigo_300,
        );
        ui.fonts.draw_in(
            &slot_name(&loc, slot),
            Rect::new(r.x + 42.0, r.y + 8.0, r.w - 50.0, 18.0),
            &t.strong(),
            Align::Left,
        );
        let sub = match &meta {
            Some(m) => format!("{} · {}", slot_line(&loc, m), m.cafe),
            None => loc.tr_owned("ui.save.empty_slot"),
        };
        ui.fonts.draw_in(
            &sub,
            Rect::new(r.x + 42.0, r.y + 28.0, r.w - 50.0, 16.0),
            &t.small(),
            Align::Left,
        );
        if resp.clicked && enabled {
            if page == Page::Save {
                match g.save(cx.content, cx.store, slot) {
                    Ok(()) => {
                        g.toast(
                            crate::ui::ToastKind::Success,
                            Icon::Check,
                            loc.tr_owned("ui.save.saved"),
                            Some(slot_name(&loc, slot)),
                        );
                        g.pause = None;
                    }
                    Err(e) => g.toast(
                        crate::ui::ToastKind::Warning,
                        Icon::Warning,
                        loc.trf("ui.save.failed", &[("reason", &e)]),
                        None,
                    ),
                }
            } else {
                g.exit = Some(GameExit::Load(slot));
            }
            return;
        }
    }
}
