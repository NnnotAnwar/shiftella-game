//! The café's own drink (design/v0.2.md I.5): the compose card opened from the recipe book
//! (a base, 2–3 jars, a style, a name with one-click ideas), the chalkboard by the counter,
//! the toasts, and the i18n layer that puts the player's name everywhere a recipe name shows.

use std::rc::Rc;

use macroquad::prelude::*;

use super::world_view::sim;
use super::{Ctx, GameScreen};
use crate::content::Content;
use crate::content::handson::{MiniGameKind, SIGNATURE_RECIPE};
use crate::i18n::{I18n, Lang, format_money};
use crate::map::TILE;
use crate::settings::Locale;
use crate::ui::draw::{fill_rrect, line, stroke_rrect, with_a};
use crate::ui::handson::{TasteMark, taste_row};
use crate::ui::{
    Align, Button, Glass, Icon, IconButton, RectExt, TextStyle, ToastKind, Ui, Weight,
};
use sim::{SimEvent, World, signature};

/// The compose card's width.
const W: f32 = 476.0;
/// Chip height.
const CHIP_H: f32 = 24.0;
/// Width of the row labels on the left.
const LABEL_W: f32 = 88.0;
/// Name ideas shown at once.
const IDEAS: u32 = 3;

/// The compose card's state (what's picked so far).
#[derive(Debug, Clone, Default)]
pub struct SigUi {
    pub open: bool,
    pub base: Option<String>,
    pub jars: Vec<String>,
    pub style: Option<MiniGameKind>,
    pub name: String,
    /// Which page of name ideas.
    pub ideas: u32,
    /// Reopen the recipe book when the card closes.
    pub back_to_book: bool,
    /// Real time the board was last chalked (a soft glow).
    pub chalked_at: Option<f32>,
}

impl GameScreen {
    /// Opens the compose card (prefilled with the café's drink when there is one).
    pub fn open_signature(&mut self, content: &Content) {
        let from_book = self.hands.book.is_some();
        self.hands.book = None;
        let mut st = SigUi {
            open: true,
            back_to_book: from_book,
            chalked_at: self.sig.chalked_at,
            ..Default::default()
        };
        match self.world.signature() {
            Some(d) => {
                st.base = Some(d.base.clone());
                st.jars = d.jars.clone();
                st.style = Some(d.style);
                st.name = d.name.clone();
            }
            None => {
                let first = self
                    .world
                    .signature_bases(content)
                    .into_iter()
                    .find(|(_, ready)| *ready)
                    .map(|(b, _)| b);
                st.base = first.map(|b| b.id.clone());
                st.style = first.and_then(|b| b.styles.first().copied());
            }
        }
        self.sig = st;
    }

    /// Closes the card (back to the book if it came from there).
    pub fn close_signature(&mut self) {
        let back = self.sig.back_to_book;
        self.sig.open = false;
        if back {
            self.open_book(None);
        }
    }

    /// Toasts for the drink (needs a locale; the board glows after a change).
    pub(super) fn signature_event(&mut self, e: &SimEvent, loc: Option<&Locale>) {
        let Some(loc) = loc else {
            return;
        };
        match e {
            SimEvent::SignatureComposed { name, first } => {
                self.sig.chalked_at = Some(self.time);
                let key = if *first {
                    "signature.composed"
                } else {
                    "signature.reworked"
                };
                let title = loc.trf(key, &[("drink", name)]);
                let body = first.then(|| loc.tr_owned("signature.composed_body"));
                self.push_ticker(Icon::Sparkle, title.clone());
                self.toast(ToastKind::Success, Icon::Sparkle, title, body);
            }
            SimEvent::SignatureInvite => {
                self.toast(
                    ToastKind::Event,
                    Icon::Note,
                    loc.tr_owned("signature.prompt"),
                    Some(loc.tr_owned("signature.prompt_body")),
                );
            }
            _ => {}
        }
    }
}

// ───────────────────────── i18n: the player's name everywhere ─────────────────────────

fn stamp_of(d: &sim::SignatureDrink) -> u64 {
    let s = format!(
        "{}|{}|{}|{:?}|{}",
        d.name,
        d.base,
        d.jars.join("+"),
        d.style,
        d.made_night
    );
    sim::rng::hash_str(&s) | 1
}

/// Puts the drink's name (and its description, and the by-name wish lines with the name
/// filled in) into the i18n layer, so every place that shows `recipe.r_signature.name` or a
/// `house_special` wish shows the player's words. Cheap when nothing changed.
pub fn sync_i18n(i18n: &mut I18n, world: &World, content: &Content) {
    let stamp = world.signature().map_or(0, stamp_of);
    if i18n.override_stamp() == stamp {
        return;
    }
    i18n.clear_overrides();
    let Some(d) = world.signature() else {
        return;
    };
    let base = signature::data().base(&d.base);
    let wishes: Vec<&crate::content::handson::WishDef> = content
        .handson
        .wishes
        .wishes
        .iter()
        .filter(|w| w.favourite.iter().any(|f| f == SIGNATURE_RECIPE))
        .collect();
    for lang in Lang::ALL {
        let base_name = base
            .map(|b| i18n.tr(lang, &b.name_key()).to_string())
            .unwrap_or_default();
        let jars: Vec<String> = d
            .jars
            .iter()
            .map(|j| {
                let k = format!("ingredient.{j}.name");
                i18n.tr(lang, &k).to_lowercase()
            })
            .collect();
        let desc = i18n.trf(
            lang,
            "signature.desc",
            &[("base", &base_name), ("jars", &jars.join(", "))],
        );
        let lore = i18n.trf(
            lang,
            "signature.lore",
            &[("night", &d.made_night.to_string())],
        );
        let lines: Vec<(String, String)> = wishes
            .iter()
            .map(|w| {
                let k = signature::wish_line_key(&w.id);
                (w.key(), i18n.trf(lang, &k, &[("drink", &d.name)]))
            })
            .collect();
        i18n.set_override(lang, "recipe.r_signature.name", Some(d.name.clone()));
        i18n.set_override(lang, "recipe.r_signature.desc", Some(desc));
        i18n.set_override(lang, "recipe.r_signature.lore", Some(lore));
        for (k, v) in lines {
            i18n.set_override(lang, &k, Some(v));
        }
    }
    i18n.set_override_stamp(stamp);
}

// ───────────────────────── overlays: the book's button and the card ─────────────────────────

/// The book's "Our drink" button (under the open book) and the compose card.
pub fn overlays(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    if g.sig.open {
        card(g, ui, cx);
        return;
    }
    let pick_mode = g.hands.book.as_ref().is_some_and(|b| b.pick_for.is_some());
    if g.hands.book.is_none() || pick_mode || !g.world.signature_unlocked() {
        return;
    }
    let loc = cx.loc;
    let s = ui.screen();
    let bs = crate::ui::handson::BOOK_SIZE;
    let book = Rect::new(
        (s.w - bs.x.min(s.w - 20.0)) / 2.0,
        (s.h - bs.y.min(s.h - 20.0)) / 2.0,
        bs.x.min(s.w - 20.0),
        bs.y.min(s.h - 20.0),
    );
    let label = match g.world.signature() {
        Some(d) => format!("{} · «{}»", loc.tr("signature.book_button"), d.name),
        None => loc.tr_owned("signature.book_button"),
    };
    let tip = loc.tr_owned("signature.book_button_hint");
    let b = Button::new(&label)
        .primary()
        .icon(Icon::Sparkle)
        .tooltip(&tip);
    let bw = ui.button_width(&b).min(book.w - 40.0);
    let y = (book.bottom() + 6.0).min(s.h - 70.0);
    let r = Rect::new(book.center().x - bw / 2.0, y, bw, 26.0);
    ui.region(r);
    if ui.button("sig_book_btn", r, &b).clicked {
        g.open_signature(cx.content);
    }
    // a little amber dot until there is one
    if g.world.signature().is_none() {
        let p = vec2(r.right() - 4.0, r.y + 4.0);
        let pulse = 0.6 + 0.4 * (g.time * 3.0).sin();
        crate::ui::draw::fill_circle(p, 4.0, with_a(ui.theme.amber, pulse));
    }
}

fn row_label(ui: &mut Ui, r: Rect, text: &str) {
    let st = ui.theme.micro();
    ui.fonts.draw_in(&text.to_uppercase(), r, &st, Align::Left);
}

/// Chips flowing from `x` to `right`; returns the bottom.
#[allow(clippy::too_many_arguments)]
fn chip_flow(
    ui: &mut Ui,
    key: &str,
    x0: f32,
    right: f32,
    y0: f32,
    items: &[(String, bool, bool, Option<&str>)],
    mut on_click: impl FnMut(usize),
) -> f32 {
    let (mut x, mut y) = (x0, y0);
    for (i, (label, selected, enabled, tip)) in items.iter().enumerate() {
        let mut b = Button::new(label)
            .secondary()
            .selected(*selected)
            .disabled(!*enabled);
        if let Some(t) = tip {
            b = b.tooltip(t);
        }
        let w = (ui.button_width(&b) - 4.0).min(right - x0);
        if x + w > right && x > x0 {
            x = x0;
            y += CHIP_H + 5.0;
        }
        if ui.button((key, i), Rect::new(x, y, w, CHIP_H), &b).clicked {
            on_click(i);
        }
        x += w + 5.0;
    }
    y + CHIP_H
}

fn card(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let content = cx.content;
    let loc = cx.loc;
    let t = ui.theme.clone();
    let s = ui.screen();
    let w = (s.w - 24.0).min(W);
    let d = signature::data();
    let bases = g.world.signature_bases(content);
    let jars_all = g.world.signature_jars();
    let base = g.sig.base.clone().and_then(|b| d.base(&b).cloned());
    // the height follows the jar rows (measured like the flow below)
    let jar_rows = {
        let mut rows = 1;
        let mut x = 0.0;
        let avail = w - 24.0 - LABEL_W;
        for j in &jars_all {
            let name = loc.tr_owned(&format!("ingredient.{j}.name"));
            let bw = ui.button_width(&Button::new(&name)) - 4.0;
            if x + bw > avail && x > 0.0 {
                rows += 1;
                x = 0.0;
            }
            x += bw + 5.0;
        }
        rows
    };
    // the bases wrap too (ru / uk at a large text size)
    let base_rows = {
        let mut rows = 1;
        let mut x = 0.0;
        let avail = w - 24.0 - LABEL_W;
        for (b, _) in &bases {
            let bw = ui.button_width(&Button::new(&loc.tr_owned(&b.name_key()))) - 4.0;
            if x + bw > avail && x > 0.0 {
                rows += 1;
                x = 0.0;
            }
            x += bw + 5.0;
        }
        rows
    };
    // header, the line from grandma, base, jars, style, tastes, name, ideas, footer
    let h = 44.0
        + 8.0
        + 20.0
        + (CHIP_H + 6.0)
        + (base_rows - 1) as f32 * (CHIP_H + 5.0)
        + jar_rows as f32 * (CHIP_H + 5.0)
        - 5.0
        + 6.0
        + (CHIP_H + 8.0)
        + (26.0 + 8.0)
        + 36.0
        + (CHIP_H + 10.0)
        + 40.0;
    let pr = Rect::new((s.w - w) / 2.0, ((s.h - h) / 2.0).max(84.0), w, h);
    g.anchors.push(("signature", pr));
    ui.region(pr);
    ui.glass(pr, &Glass::panel());
    let (body, close) = ui.panel_header(pr, loc.tr("signature.title"), None, None);
    if close {
        g.close_signature();
        return;
    }
    let c = body.inset_xy(12.0, 8.0);
    let mut y = c.y;
    ui.fonts.draw_in(
        loc.tr("signature.sub"),
        Rect::new(c.x, y - 2.0, c.w, 18.0),
        &t.small().color(t.text_dim),
        Align::Left,
    );
    y += 20.0;
    let x0 = c.x + LABEL_W;
    // --- the base ----------------------------------------------------------------------------
    row_label(
        ui,
        Rect::new(c.x, y, LABEL_W, CHIP_H),
        loc.tr("signature.step.base"),
    );
    let not_ready = loc.tr_owned("signature.not_ready");
    let items: Vec<(String, bool, bool, Option<&str>)> = bases
        .iter()
        .map(|(b, ready)| {
            (
                loc.tr_owned(&b.name_key()),
                g.sig.base.as_deref() == Some(b.id.as_str()),
                *ready,
                (!*ready).then_some(not_ready.as_str()),
            )
        })
        .collect();
    let mut pick_base = None;
    y = chip_flow(ui, "sig_base", x0, c.right(), y, &items, |i| {
        pick_base = Some(i)
    });
    if let Some(i) = pick_base {
        let b = bases[i].0;
        g.sig.base = Some(b.id.clone());
        // its own jar can't be an extra; keep a style that suits it
        g.sig.jars.retain(|j| *j != b.ingredient);
        if !g.sig.style.is_some_and(|s| b.styles.contains(&s)) {
            g.sig.style = b.styles.first().copied();
        }
    }
    y += 6.0;
    // --- the jars ----------------------------------------------------------------------------
    row_label(
        ui,
        Rect::new(c.x, y, LABEL_W, CHIP_H),
        loc.tr("signature.step.jars"),
    );
    let full = g.sig.jars.len() >= d.jars.1;
    let items: Vec<(String, bool, bool, Option<&str>)> = jars_all
        .iter()
        .map(|j| {
            let on = g.sig.jars.iter().any(|x| x == j);
            let own = base.as_ref().is_some_and(|b| b.ingredient == *j);
            (
                loc.tr_owned(&format!("ingredient.{j}.name")),
                on,
                !own && (on || !full),
                None,
            )
        })
        .collect();
    let mut toggle = None;
    y = chip_flow(ui, "sig_jar", x0, c.right(), y, &items, |i| {
        toggle = Some(i)
    });
    if let Some(i) = toggle {
        let j = jars_all[i].to_string();
        if let Some(p) = g.sig.jars.iter().position(|x| *x == j) {
            g.sig.jars.remove(p);
        } else if g.sig.jars.len() < d.jars.1 {
            g.sig.jars.push(j);
        }
    }
    y += 6.0;
    // --- the style ---------------------------------------------------------------------------
    row_label(
        ui,
        Rect::new(c.x, y, LABEL_W, CHIP_H),
        loc.tr("signature.step.style"),
    );
    let styles: Vec<MiniGameKind> = base.as_ref().map(|b| b.styles.clone()).unwrap_or_default();
    let items: Vec<(String, bool, bool, Option<&str>)> = styles
        .iter()
        .map(|k| {
            (
                loc.tr_owned(&format!("signature.style.{}", k.id())),
                g.sig.style == Some(*k),
                true,
                None,
            )
        })
        .collect();
    let mut pick_style = None;
    y = chip_flow(ui, "sig_style", x0, c.right(), y, &items, |i| {
        pick_style = Some(i)
    });
    if let Some(i) = pick_style {
        g.sig.style = Some(styles[i]);
    }
    y += 8.0;
    // --- what it would be --------------------------------------------------------------------
    let preview = match (&g.sig.base, g.sig.style) {
        (Some(b), Some(st)) => Some(g.world.signature_preview(content, b, &g.sig.jars, st)),
        _ => None,
    };
    let row = Rect::new(c.x, y, c.w, 26.0);
    fill_rrect(row, 6.0, Color::new(0.03, 0.02, 0.09, 0.35));
    row_label(
        ui,
        Rect::new(row.x + 8.0, row.y, LABEL_W, row.h),
        loc.tr("signature.tastes"),
    );
    let mut problem: Option<String> = None;
    match &preview {
        Some(Ok(def)) => {
            let tags: Vec<_> = def
                .tags
                .iter()
                .map(|t| (super::hands_ui::taste_tag(*t), TasteMark::Want))
                .collect();
            let names = super::hands_ui::taste_names(&loc);
            taste_row(
                ui,
                "sig_tags",
                x0,
                row.center_y(),
                18.0,
                &tags,
                Some(&names),
            );
            let price = g.world.price_of(content, &def.base_item) + def.price_add;
            let txt = loc.trf(
                "signature.price",
                &[("price", &format_money(loc.lang, price as i64))],
            );
            ui.fonts.draw_in(
                &txt,
                row.inset_xy(8.0, 0.0),
                &t.small_medium().color(t.amber),
                Align::Right,
            );
        }
        Some(Err(e)) => {
            problem = Some(problem_text(&loc, e, &g.sig.jars));
        }
        None => problem = Some(loc.tr_owned("signature.need_base")),
    }
    if let Some(p) = &problem {
        ui.fonts.draw_in(
            p,
            Rect::new(x0, row.y, row.right() - x0 - 8.0, row.h),
            &t.small().color(t.text_dim),
            Align::Left,
        );
    }
    y = row.bottom() + 8.0;
    // --- the name ----------------------------------------------------------------------------
    row_label(
        ui,
        Rect::new(c.x, y, LABEL_W, 30.0),
        loc.tr("signature.step.name"),
    );
    let ir = Rect::new(x0, y, c.right() - x0 - 44.0, 30.0);
    let resp = ui.text_input(
        "sig_name",
        ir,
        &mut g.sig.name,
        loc.tr("signature.name_hint"),
        d.name_max,
    );
    let n = g.sig.name.chars().count();
    ui.fonts.draw_in(
        &format!("{n}/{}", d.name_max),
        Rect::new(ir.right() + 4.0, y, 40.0, 30.0),
        &t.small()
            .color(if n >= d.name_max { t.amber } else { t.text_dim }),
        Align::Right,
    );
    y += 36.0;
    // --- name ideas: one click takes one -------------------------------------------------------
    row_label(
        ui,
        Rect::new(c.x, y, LABEL_W, CHIP_H),
        loc.tr("signature.ideas"),
    );
    let pages = (d.suggestions / IDEAS).max(1);
    let page = g.sig.ideas % pages;
    let seed = g.world.night() + base.as_ref().map_or(0, |b| b.id.len() as u32);
    let ideas: Vec<String> = (0..IDEAS.min(d.suggestions))
        .map(|i| {
            let n = 1 + (page * IDEAS + i + seed * IDEAS) % d.suggestions.max(1);
            loc.tr_owned(&format!("signature.suggest.{n}"))
        })
        .collect();
    let mut x = x0;
    let more = Rect::new(c.right() - CHIP_H, y, CHIP_H, CHIP_H);
    for (i, idea) in ideas.iter().enumerate() {
        let b = Button::new(idea)
            .ghost()
            .selected(g.sig.name.trim() == idea.as_str());
        let bw = ui.button_width(&b) - 4.0;
        if x + bw > more.x - 6.0 {
            continue;
        }
        if ui
            .button(("sig_idea", i), Rect::new(x, y, bw, CHIP_H), &b)
            .clicked
        {
            g.sig.name = idea.clone();
        }
        x += bw + 5.0;
    }
    let more_tip = loc.tr_owned("signature.more_ideas");
    if ui
        .icon_button(
            "sig_more",
            more,
            Icon::Repeat,
            &IconButton::ghost().size(13.0).tooltip(&more_tip),
        )
        .clicked
    {
        g.sig.ideas = g.sig.ideas.wrapping_add(1);
    }
    y += CHIP_H + 10.0;
    // --- the footer: when it can change, and the button ---------------------------------------
    crate::ui::draw::hline(c.x, c.right(), y - 4.0, t.divider);
    let next = g.world.signature_next_change();
    let has = g.world.signature().is_some();
    let status = match next {
        Some(n) => loc.trf("signature.locked", &[("night", &n.to_string())]),
        None if has
            && g.world
                .signature()
                .is_some_and(|d| d.changed_night == g.world.night()) =>
        {
            loc.tr_owned("signature.tweak")
        }
        None if has => loc.tr_owned("signature.once_a_week"),
        None => String::new(),
    };
    let asked = g.world.signature_asked_served();
    let status = if asked > 0 {
        let a = loc.trf("signature.asked", &[("n", &asked.to_string())]);
        if status.is_empty() {
            a
        } else {
            format!("{status} · {a}")
        }
    } else {
        status
    };
    let name_ok = signature::clean_name(&g.sig.name).is_some();
    let ok = matches!(preview, Some(Ok(_))) && name_ok && next.is_none();
    let label = loc.tr_owned(if has {
        "signature.save"
    } else {
        "signature.chalk"
    });
    let b = Button::new(&label)
        .primary()
        .icon(Icon::Check)
        .disabled(!ok);
    let bw = ui.button_width(&b);
    let br = Rect::new(c.right() - bw, y + 2.0, bw, 28.0);
    let hint = if !name_ok && problem.is_none() && next.is_none() {
        loc.tr_owned("signature.need_name")
    } else {
        status
    };
    ui.fonts.draw_wrapped(
        &hint,
        Rect::new(c.x, y, (br.x - c.x - 10.0).max(40.0), 32.0),
        &t.small().color(t.text_dim),
        Align::Left,
        Some(2),
    );
    let submit = resp.submitted && ok;
    if ui.button("sig_chalk", br, &b).clicked || submit {
        let cmd = sim::Command::ComposeSignature {
            base: g.sig.base.clone().unwrap_or_default(),
            jars: g.sig.jars.clone(),
            style: g.sig.style.unwrap_or(MiniGameKind::MilkFroth),
            name: g.sig.name.clone(),
        };
        match g.command(content, cmd) {
            Ok(()) => {
                // the name goes up on the board in chalk
                cx.audio.play_sfx_vol(crate::audio::Sfx::Chalk, 0.7);
                g.sig.back_to_book = false;
                g.close_signature();
            }
            Err(e) => super::panels::command_error(g, &loc, &e),
        }
    }
}

fn problem_text(loc: &Locale, e: &sim::CommandError, jars: &[String]) -> String {
    let d = signature::data();
    match e {
        sim::CommandError::Locked => loc.tr_owned("signature.not_ready"),
        sim::CommandError::Invalid(m) if m.starts_with("one of") => {
            loc.tr_owned("signature.one_of")
        }
        sim::CommandError::Invalid(m) if m.contains("style") => {
            loc.tr_owned("signature.need_style")
        }
        _ if jars.len() < d.jars.0 || jars.len() > d.jars.1 => loc.tr_owned("signature.need_jars"),
        _ => loc.tr_owned("signature.need_jars"),
    }
}

/// Esc closes the card (consumed). True when the card had the keyboard.
pub fn hotkeys(g: &mut GameScreen, ui: &mut Ui) -> bool {
    if !g.sig.open {
        return false;
    }
    if ui.esc() || crate::settings::keys::pressed(ui, crate::settings::keys::Action::Book) {
        g.close_signature();
    }
    true
}

// ───────────────────────── the chalkboard by the counter ─────────────────────────

/// The house drink chalked on a little A-frame board at the left end of the counter. Drawn
/// in the UI pass (under the HUD) at the board's place in the café, never smaller than
/// readable; hover shows the tastes, a click opens the card.
pub fn board(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let content = cx.content;
    let loc = cx.loc;
    let Some(d) = g.world.signature().cloned() else {
        return;
    };
    if g.player.is_playing() || g.walk.is_open() || g.photo.is_open() || g.summary.is_some() {
        return;
    }
    let Some((x0, _x1, front)) = super::scene_fx::counter_span(&g.world, content) else {
        return;
    };
    let feet = super::hud::world_to_ui(g, ui, vec2(x0 - 1.8 * TILE, front + 6.0));
    // logical px per world px; the board is ~1.6 tiles wide when zoomed in, never tiny
    let zl = g.cam.zoom / ui.scale();
    let k = (zl * 1.6).clamp(1.0, 2.0);
    let bw = 108.0 * k;
    let bh = 62.0 * k;
    let legs = 12.0 * k;
    let r = Rect::new(feet.x - bw / 2.0, feet.y - legs - bh, bw, bh);
    let s = ui.screen();
    if r.right() < 0.0 || r.x > s.w || r.bottom() < 0.0 || r.y > s.h {
        return;
    }
    // the A-frame legs and the frame
    let wood = Color::from_hex(0x8a5a35);
    let wood_d = Color::from_hex(0x5e3b22);
    line(
        vec2(r.x + 10.0 * k, r.bottom() - 2.0),
        vec2(r.x + 4.0 * k, r.bottom() + legs),
        3.0 * k,
        wood_d,
    );
    line(
        vec2(r.right() - 10.0 * k, r.bottom() - 2.0),
        vec2(r.right() - 4.0 * k, r.bottom() + legs),
        3.0 * k,
        wood_d,
    );
    draw_ellipse(
        feet.x,
        feet.y + 1.0,
        bw * 0.42,
        3.0 * k,
        0.0,
        Color::new(0.0, 0.0, 0.0, 0.25),
    );
    fill_rrect(r, 4.0 * k, wood);
    let slate = r.inset(4.0 * k);
    fill_rrect(
        slate,
        2.5 * k,
        crate::ui::draw::Paint::vertical(
            slate,
            Color::from_hex(0x2b3532),
            Color::from_hex(0x1d2523),
        ),
    );
    // chalk dust
    for i in 0..7 {
        let fx = ((i * 7919 + d.made_night as usize * 31) % 1000) as f32 / 1000.0;
        let fy = ((i * 104_729) % 1000) as f32 / 1000.0;
        let p = vec2(slate.x + fx * slate.w, slate.y + fy * slate.h);
        line(
            p,
            p + vec2(6.0 * k, -1.5 * k),
            1.0 * k,
            Color::new(1.0, 1.0, 1.0, 0.05),
        );
    }
    // a soft glow just after it was chalked
    if let Some(at) = g.sig.chalked_at {
        let a = (1.0 - (g.time - at) / 2.5).clamp(0.0, 1.0);
        if a > 0.0 {
            crate::ui::draw::glow_circle(
                r.center(),
                bw * 0.7,
                Color::new(1.0, 0.85, 0.55, 0.35 * a),
            );
        }
    }
    let chalk = Color::new(0.96, 0.95, 0.9, 0.93);
    let inner = slate.inset_xy(6.0 * k, 4.0 * k);
    // the head line
    let hs = TextStyle::new(8.5 * k, Weight::SemiBold, with_a(chalk, 0.62)).tracking(0.6 * k);
    ui.fonts.draw_in(
        &loc.tr("signature.board.head").to_uppercase(),
        Rect::new(inner.x, inner.y, inner.w, 11.0 * k),
        &hs,
        Align::Center,
    );
    // the name, handwritten, up to two lines
    let price = g.world.price_of(content, &d.def.base_item) + d.def.price_add;
    let ps = TextStyle::new(10.5 * k, Weight::SemiBold, Color::new(1.0, 0.8, 0.45, 0.95));
    let name_box = Rect::new(
        inner.x,
        inner.y + 12.0 * k,
        inner.w,
        inner.h - 12.0 * k - 13.0 * k,
    );
    let mut size = 13.0 * k;
    let ns = loop {
        let st = TextStyle::new(size, Weight::SemiBold, chalk).line_height(1.05);
        let lines = ui.fonts.wrap(&d.name, name_box.w - size * 0.2, &st).len();
        if (lines as f32) * st.line_h() <= name_box.h + 1.0 || size <= 9.5 * k {
            break st;
        }
        size -= 0.5;
    };
    let lines = ui
        .fonts
        .wrap(&d.name, name_box.w - ns.size * 0.2, &ns)
        .len()
        .clamp(1, 2);
    let lh = lines as f32 * ns.line_h();
    let nb = Rect::new(
        name_box.x,
        name_box.y + (name_box.h - lh) / 2.0,
        name_box.w,
        lh,
    );
    crate::ui::handson::draw_handwritten_block(ui, &d.name, nb, &ns, Align::Center, 2);
    let pr = Rect::new(inner.x, inner.bottom() - 12.0 * k, inner.w, 12.0 * k);
    ui.fonts.draw_in(
        &format_money(loc.lang, price as i64),
        pr,
        &ps,
        Align::Center,
    );
    // a tiny chalk cup on the left of the price
    crate::ui::draw_icon(
        Icon::Cup,
        vec2(pr.x + 8.0 * k, pr.center_y()),
        9.0 * k,
        with_a(chalk, 0.7),
    );
    stroke_rrect(slate, 2.5 * k, 1.0, Color::new(0.0, 0.0, 0.0, 0.35));
    // hover: what it is; a click opens the card (no world click under it)
    let hit = Rect::new(r.x, r.y, r.w, r.h + legs);
    if g.panel.is_none() && !g.hands.overlay() && !g.sig.open {
        ui.region(hit);
        let id = ui.id("sig_board");
        let resp = ui.interact(id, hit);
        let tags: Vec<String> = d.def.tags.iter().map(|t| loc.tr_owned(&t.key())).collect();
        let tip = format!(
            "«{}» · {} · {}",
            d.name,
            tags.join(", "),
            loc.tr("recipe.r_signature.desc")
        );
        ui.tooltip(&resp, &tip);
        if resp.clicked {
            g.open_signature(content);
            g.sig.back_to_book = false;
        }
    }
}

// ───────────────────────── screenshot scenes ─────────────────────────

/// `SHIFTELLA_SCENE=signature_compose` (the card on the day of night 7, half filled) and
/// `signature_board` (night 8 at 00:30 with a drink chalked on the board).
pub fn scene_setup(g: &mut GameScreen, content: &Rc<Content>, loc: &Locale, name: &str) {
    let lang_name = loc.tr_owned("signature.suggest.1");
    match name {
        "signature_compose" => {
            g.summary = None;
            g.open_signature(content);
            g.sig.base = Some("cocoa".into());
            g.sig.style = Some(MiniGameKind::MilkFroth);
            g.sig.jars = vec!["milk".into(), "spice".into()];
            g.sig.name = std::env::var("SHIFTELLA_SIG_NAME").unwrap_or(lang_name);
        }
        _ => {
            let name = std::env::var("SHIFTELLA_SIG_NAME").unwrap_or(lang_name);
            let _ = g.command(
                content,
                sim::Command::ComposeSignature {
                    base: "cocoa".into(),
                    jars: vec!["milk".into(), "spice".into()],
                    style: MiniGameKind::MilkFroth,
                    name,
                },
            );
            g.drain_and_handle(content, Some(loc));
        }
    }
}

/// Builds the package-I screenshot scenes (`SHIFTELLA_SCENE`): `signature_compose` (the day
/// of night 7, the card half filled), `signature_board` (night 8, 00:30, the board by the
/// counter), `photo_mode` (the viewfinder up), `photo_print` (a print developing) and
/// `journal_photos` (the day of night 9, extra photos on nights 3, 5 and 7; `SHIFTELLA_OPEN`
/// enlarges a slot, `SHIFTELLA_SHOWN` picks its photo). `SHIFTELLA_SIG_NAME` names the drink.
pub fn scene(
    content: &Rc<Content>,
    loc: &Locale,
    name: &str,
    night: Option<u32>,
    minute: Option<i32>,
) -> GameScreen {
    let n = night.unwrap_or(match name {
        "signature_compose" => 7,
        "journal_photos" => 9,
        _ => 8,
    });
    let m = minute.unwrap_or(150);
    let mut g = GameScreen::new_story(content, "cozy", true);
    match name {
        "signature_compose" => {
            g.fast_forward(content, loc, n, None, n > 1);
            // the scene shows a first drink, whatever the fast-forward's player did
            g.world.sig.drink = None;
            scene_setup(&mut g, content, loc, name);
        }
        "signature_board" => {
            g.fast_forward(content, loc, n, None, n > 1);
            g.world.sig.drink = None;
            scene_setup(&mut g, content, loc, name);
            g.fast_forward(content, loc, n, Some(m), n > 1);
        }
        "signature_order" => {
            // a stranger asks for it by name: the order card with the drink's chip
            g.fast_forward(content, loc, n, None, n > 1);
            g.world.sig.drink = None;
            scene_setup(&mut g, content, loc, "signature_board");
            g.fast_forward(content, loc, n, Some(minute.unwrap_or(20)), n > 1);
            let by_name = |g: &GameScreen| {
                g.hands.order.as_ref().is_some_and(|o| {
                    g.world
                        .order_kind(o.guest)
                        .and_then(|k| k.wish())
                        .is_some_and(|w| w == "house_special")
                })
            };
            for _ in 0..12 {
                for p in g.world.parties.iter_mut() {
                    if p.regular.is_none() && p.wish_open && p.order_kind.wish().is_some() {
                        p.order_kind = sim::OrderKind::Wish {
                            wish: "house_special".into(),
                        };
                    }
                }
                g.play_headless(content, loc, 40.0, &by_name);
                if by_name(&g) {
                    break;
                }
                g.hands.order = None;
            }
            // the autoplayer would brew it at once: hold the card
            g.hands.autoplay = None;
        }
        "photo_mode" | "photo_print" => {
            g.fast_forward(content, loc, n, Some(m), n > 1);
            super::photo_mode::scene_setup(&mut g, content, loc, name);
        }
        _ => {
            for (night, shots) in [(3u32, 2u32), (5, 3), (7, 1)] {
                if night >= n {
                    break;
                }
                g.fast_forward(content, loc, night, Some(m), night > 1);
                let k = g.world.karolina().map(|k| k.pos).unwrap_or((960.0, 500.0));
                for i in 0..shots {
                    let _ = g.command(
                        content,
                        sim::Command::TakePhoto {
                            focus: (k.0 - 200.0 + i as f32 * 180.0, k.1 - 120.0),
                            width: 9.0 * TILE,
                        },
                    );
                }
            }
            g.fast_forward(content, loc, n, None, n > 1);
            g.panel = Some(super::Panel::Journal);
            let env = |k: &str| std::env::var(k).ok().and_then(|v| v.parse::<usize>().ok());
            g.photos.journal.page = env("SHIFTELLA_PAGE").unwrap_or(0);
            if let Some(o) = env("SHIFTELLA_OPEN") {
                g.photos.journal.open = Some(o);
                g.photos.journal.shown = env("SHIFTELLA_SHOWN").unwrap_or(0);
            }
        }
    }
    g
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;

    fn content() -> Rc<Content> {
        let root = env!("CARGO_MANIFEST_DIR");
        Rc::new(Content::load(&format!("{root}/assets/data")).expect("content"))
    }

    fn i18n() -> I18n {
        let root = env!("CARGO_MANIFEST_DIR");
        crate::settings::load_i18n(&format!("{root}/assets/i18n")).expect("i18n")
    }

    /// A story game on the day of night 6 (cutscenes done).
    fn day6(c: &Rc<Content>, i: &I18n) -> GameScreen {
        let loc = Locale::new(i, Lang::En);
        let mut g = GameScreen::new_story(c, "cozy", true);
        g.fast_forward(c, &loc, 6, None, false);
        g.world.sig.drink = None;
        g
    }

    #[test]
    fn the_players_name_shows_wherever_the_recipe_and_the_wishes_do() {
        let c = content();
        let mut i = i18n();
        let mut g = day6(&c, &i);
        sync_i18n(&mut i, &g.world, &c);
        assert_eq!(i.tr(Lang::En, "recipe.r_signature.name"), "The house drink");
        g.command(
            &c,
            sim::Command::ComposeSignature {
                base: "cocoa".into(),
                jars: vec!["milk".into(), "spice".into()],
                style: MiniGameKind::MilkFroth,
                name: "Голешовицкая ночь".into(),
            },
        )
        .unwrap();
        sync_i18n(&mut i, &g.world, &c);
        for lang in Lang::ALL {
            assert_eq!(i.tr(lang, "recipe.r_signature.name"), "Голешовицкая ночь");
            let line = i.tr(lang, "wish.house_special");
            assert!(line.contains("Голешовицкая ночь"), "{lang:?}: {line}");
            assert!(!line.contains('{'), "{lang:?}: {line}");
            let mirek = i.tr(lang, "wish.mirek.house_special");
            assert!(mirek.contains("Голешовицкая ночь"), "{lang:?}: {mirek}");
            let desc = i.tr(lang, "recipe.r_signature.desc");
            assert!(!desc.contains('{') && desc.len() > 5, "{lang:?}: {desc}");
        }
        // a world without the drink clears the layer again
        let other = GameScreen::new_story(&c, "cozy", true);
        sync_i18n(&mut i, &other.world, &c);
        assert_eq!(i.tr(Lang::En, "recipe.r_signature.name"), "The house drink");
        assert_eq!(
            i.tr(Lang::Cs, "wish.house_special"),
            "Tu specialitu z tabule, prosím."
        );
    }

    #[test]
    fn the_card_prefills_from_the_drink_and_toasts_follow_the_events() {
        let c = content();
        let i = i18n();
        let loc = Locale::new(&i, Lang::Cs);
        let mut g = day6(&c, &i);
        g.open_signature(&c);
        assert!(g.sig.open);
        assert!(g.sig.base.is_some() && g.sig.style.is_some());
        assert!(g.overlay_open(), "the card slows the night like the book");
        g.close_signature();
        assert!(!g.sig.open);
        g.notices.clear();
        g.command(
            &c,
            sim::Command::ComposeSignature {
                base: "coffee".into(),
                jars: vec!["milk".into(), "honey".into()],
                style: MiniGameKind::LatteArt,
                name: "Lídin čaj".into(),
            },
        )
        .unwrap();
        g.drain_and_handle(&c, Some(&loc));
        assert!(g.notices.iter().any(|n| matches!(n,
            super::super::Notice::Toast { title, .. } if title.contains("Lídin čaj"))));
        assert!(g.sig.chalked_at.is_some());
        g.open_signature(&c);
        assert_eq!(g.sig.name, "Lídin čaj");
        assert_eq!(g.sig.base.as_deref(), Some("coffee"));
        assert_eq!(g.sig.jars, vec!["milk".to_string(), "honey".to_string()]);
        assert_eq!(g.sig.style, Some(MiniGameKind::LatteArt));
    }
}
