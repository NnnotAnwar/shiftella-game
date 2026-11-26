//! Build → Theme: a card per decor theme with a little preview of its wall, floor and a
//! table; buy once, switch for free in the Day phase. A locked theme says in plain words how
//! it unlocks (reputation 50 *or* 3 000 Kč in tips, the 1979 timetable *or* night 18, …);
//! the hover shows how far along that is.

use macroquad::prelude::*;

use super::world_view::sim;
use super::{Ctx, GameScreen};
use crate::content::Content;
use crate::content::handson::{Cond, Unlock, XCondition};
use crate::content::logic::Condition;
use crate::i18n::format_money;
use crate::map::TILE;
use crate::render::{FurnitureState, draw_furniture};
use crate::settings::Locale;
use crate::ui::draw::{fill_rrect, stroke_rrect, with_a};
use crate::ui::{Align, Button, Icon, Ui};
use sim::{Command, World};

/// One condition in words (`None` when it has no player-facing wording).
fn cond_text(loc: &Locale, w: &World, c: &Cond) -> Option<String> {
    let join = |v: &[Cond], key: &str| -> Option<String> {
        let parts: Vec<String> = v.iter().filter_map(|c| cond_text(loc, w, c)).collect();
        (!parts.is_empty()).then(|| parts.join(&format!(" {} ", loc.tr(key))))
    };
    match c {
        Cond::X(XCondition::Any(v)) => join(v, "bux.cond.or"),
        Cond::X(XCondition::All(v)) => join(v, "bux.cond.and"),
        Cond::X(XCondition::TipsLifetimeAtLeast(n)) => Some(loc.trf(
            "bux.cond.tips",
            &[("money", &format_money(loc.lang, *n as i64))],
        )),
        Cond::X(XCondition::CozinessAtLeast(n)) => Some(loc.trf(
            "bux.cond.coziness",
            &[("n", &format!("{}", n.round() as i32))],
        )),
        Cond::Core(Condition::NightAtLeast(n)) => {
            Some(loc.trf("bux.cond.night", &[("n", &n.to_string())]))
        }
        Cond::Core(Condition::ReputationAtLeast(r)) => Some(loc.trf(
            "bux.cond.reputation",
            &[("n", &format!("{}", r.round() as i32))],
        )),
        Cond::Core(Condition::HasItem(id)) => {
            let key = format!("item.{id}.name");
            loc.has(&key)
                .then(|| loc.trf("bux.cond.item", &[("item", loc.tr(&key))]))
        }
        Cond::Core(Condition::ArcAtLeast(r, _)) => {
            let key = format!("regular.{r}.name");
            loc.has(&key)
                .then(|| loc.trf("bux.cond.arc", &[("name", loc.tr(&key))]))
        }
        Cond::Core(Condition::Flag(f)) => {
            let key = format!("bux.cond.flag.{f}");
            loc.has(&key).then(|| loc.tr_owned(&key))
        }
        _ => {
            let _ = w;
            None
        }
    }
}

/// How the theme unlocks, in plain words.
pub fn unlock_text(
    loc: &Locale,
    w: &World,
    id: &str,
    unlock: &Unlock,
    sandbox_night: Option<u32>,
) -> String {
    let special = format!("bux.theme_unlock.{id}");
    if loc.has(&special) {
        return loc.tr_owned(&special);
    }
    let mut parts: Vec<String> = match unlock {
        Unlock::When(c) => c.iter().filter_map(|c| cond_text(loc, w, c)).collect(),
        _ => Vec::new(),
    };
    if w.mode() == sim::Mode::Sandbox
        && let Some(n) = sandbox_night
    {
        parts = vec![loc.trf("bux.cond.night", &[("n", &n.to_string())])];
    }
    if parts.is_empty() {
        loc.tr_owned("ui.theme.locked")
    } else {
        let s = parts.join(&format!(" {} ", loc.tr("bux.cond.and")));
        let mut c = s.chars();
        c.next()
            .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
            .unwrap_or_default()
    }
}

/// Where the player stands on the numbers a theme waits for (for the hover).
fn progress_text(loc: &Locale, w: &World, unlock: &Unlock) -> Option<String> {
    let mut rep = false;
    let mut tips = false;
    let mut night = false;
    fn walk(c: &Cond, f: &mut dyn FnMut(&Cond)) {
        f(c);
        if let Cond::X(XCondition::Any(v) | XCondition::All(v)) = c {
            for c in v {
                walk(c, f);
            }
        }
    }
    for c in unlock.conditions() {
        walk(c, &mut |c| match c {
            Cond::Core(Condition::ReputationAtLeast(_)) => rep = true,
            Cond::X(XCondition::TipsLifetimeAtLeast(_)) => tips = true,
            Cond::Core(Condition::NightAtLeast(_)) => night = true,
            _ => {}
        });
    }
    let mut v = Vec::new();
    if rep {
        v.push(loc.trf(
            "bux.now.reputation",
            &[("n", &format!("{}", w.reputation().round() as i32))],
        ));
    }
    if tips {
        v.push(loc.trf(
            "bux.now.tips",
            &[("money", &format_money(loc.lang, w.tips_lifetime()))],
        ));
    }
    if night {
        v.push(loc.trf("bux.now.night", &[("n", &w.night().to_string())]));
    }
    let s = v.join(" · ");
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
}

pub fn themes(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx, rest: Rect) {
    use crate::render::{DecorTheme, ThemeStyle, with_decor};
    let t = ui.theme.clone();
    let loc = cx.loc;
    let content = cx.content;
    let day = g.world.phase() == sim::Phase::Day;
    let current = g.world.theme(content).to_string();
    let season = super::scene_fx::season_of(&g.world, content).1;
    let defs: Vec<_> = content.handson.decor.themes.clone();
    let gap = 6.0;
    let card_w = ((rest.w - gap * (defs.len() as f32 - 1.0)) / defs.len().max(1) as f32).min(180.0);
    let card_h = rest.h - 4.0;
    let mut x = rest.x;
    for d in &defs {
        let r = Rect::new(x, rest.y, card_w, card_h);
        x += card_w + gap;
        let owned = g.world.theme_owned(content, &d.id);
        let unlocked = owned || g.world.themes_unlocked().contains(&d.id);
        let on = current == d.id;
        let resp = ui.interact(ui.id(("theme_card", d.id.as_str())), r);
        fill_rrect(
            r,
            9.0,
            if on {
                with_a(t.indigo, 0.3)
            } else if resp.hovered {
                Color::new(1.0, 1.0, 1.0, 0.07)
            } else {
                Color::new(1.0, 1.0, 1.0, 0.035)
            },
        );
        if on {
            stroke_rrect(r, 9.0, 1.0, t.indigo_300);
        }
        // the preview: two tiles of wall, a floor, a table and a lamp in the theme's look
        let prev = Rect::new(r.x + 6.0, r.y + 5.0, r.w - 12.0, card_h - 56.0);
        let style = DecorTheme::from_id(&d.id)
            .map(|th| th.style(season))
            .unwrap_or(ThemeStyle::GRANDMA);
        fill_rrect(prev, 7.0, Color::new(0.05, 0.04, 0.1, 0.6));
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
            crate::ui::draw_icon(Icon::Lock, prev.center(), 16.0, t.text_dim);
        }
        let name = loc.tr_owned(&d.name_key());
        ui.fonts.draw_in(
            &name,
            Rect::new(r.x + 6.0, prev.bottom() + 2.0, r.w - 12.0, 16.0),
            &t.small_medium()
                .color(if unlocked { t.text } else { t.text_dim }),
            Align::Center,
        );
        let hint =
            (!unlocked).then(|| unlock_text(&loc, &g.world, &d.id, &d.unlock, d.sandbox_night));
        let mut tip = loc.tr_owned(&d.desc_key());
        if let Some(h) = &hint {
            tip.push_str(&format!("\n{} {h}", loc.tr("bux.tip.locked")));
            if let Some(p) = progress_text(&loc, &g.world, &d.unlock) {
                tip.push_str(&format!("\n{p}"));
            }
        }
        ui.tooltip(&resp, &tip);
        let br = Rect::new(r.x + 6.0, r.bottom() - 30.0, r.w - 12.0, 26.0);
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
        } else if let Some(h) = hint {
            // how to unlock, in plain words (two short lines)
            let hr = Rect::new(br.x, br.y - 2.0, br.w, 28.0);
            let st = ui
                .fonts
                .fit_lines(&h, hr, &t.small().size(11.0).color(t.text_dim), 2);
            ui.fonts.draw_wrapped(&h, hr, &st, Align::Center, Some(2));
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
