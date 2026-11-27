//! Karolina's loop on screen (design/handson.md §7.4): the ticket rail, the tip jar and lull
//! chips, the order card, the mini-game, the recipe book and grandma's record picker.
//! Each widget gets a view model filled from the sim + content here and returns an action.

use std::rc::Rc;

use macroquad::prelude::*;

use super::hands;
use super::hud::world_to_ui;
use super::world_view::sim;
use super::{Ctx, GameScreen};
use crate::content::Content;
use crate::content::handson::{BookTab, HallMood, Motif, Taste};
use crate::i18n::format_money;
use crate::minigame::{MiniGame, MiniGameStatus, MiniGameText};
use crate::render::PersonLook;
use crate::settings::Locale;
use crate::ui::draw::{fill_rrect, glow_circle, with_a};
use crate::ui::handson::{
    BookAction, BookLabels, BookRecipe, BookSection, BookView, Dish, DishGlyph, ExperimentKind,
    ExperimentResult, Ingredient, OrderAction, OrderKind, OrderLabels, OrderView, RailAction,
    RecipeChoice, RecordAction, RecordLabels, RecordView, RecordsView, Shelf, SleeveMotif,
    TasteNames, TasteTag, Ticket, TicketItem, TicketOwner, order_card, recipe_book, record_picker,
    ticket_rail, ticket_rail_rect,
};
use crate::ui::{Align, Button, Glass, Icon, RectExt, Ui, draw_icon};
use sim::{AgentRef, Command, Phase, TicketStage};

// ───────────────────────── content → widget mapping ─────────────────────────

pub fn taste_tag(t: Taste) -> TasteTag {
    match t {
        Taste::Warm => TasteTag::Warm,
        Taste::Cold => TasteTag::Cold,
        Taste::Sweet => TasteTag::Sweet,
        Taste::Bitter => TasteTag::Bitter,
        Taste::Strong => TasteTag::Strong,
        Taste::Mild => TasteTag::Mild,
        Taste::Comforting => TasteTag::Comforting,
        Taste::Fancy => TasteTag::Fancy,
        Taste::Quick => TasteTag::Quick,
        Taste::Filling => TasteTag::Filling,
        Taste::Spiced => TasteTag::Spiced,
        Taste::Fresh => TasteTag::Fresh,
    }
}

pub fn taste_names(loc: &Locale) -> TasteNames {
    TasteNames {
        names: Taste::ALL.map(|t| loc.tr_owned(&t.key())),
    }
}

/// The little illustration of a recipe / menu item (order chips, rail, book).
pub fn dish_of(content: &Content, recipe_or_menu: &str) -> Dish {
    let menu = content
        .handson
        .recipe(recipe_or_menu)
        .map(|r| r.base_item.as_str())
        .unwrap_or(recipe_or_menu);
    let hex = |h: u32| Color::from_hex(h);
    let (glyph, tint, steam) = match (recipe_or_menu, menu) {
        ("r_espresso_tonic", _) => (DishGlyph::ColdGlass, hex(0x6b4a2f), false),
        ("r_moon_latte", _) => (DishGlyph::Cup, hex(0xc9b6f2), true),
        ("r_linden_tea", _) => (DishGlyph::TeaGlass, hex(0xd9b75a), true),
        ("r_uzvar", _) => (DishGlyph::TeaGlass, hex(0xa8542f), true),
        ("r_lori_encore", _) => (DishGlyph::Mug, hex(0x7a3b22), true),
        ("r_borscht", _) => (DishGlyph::Bowl, hex(0xa3243a), true),
        ("r_buchty", _) => (DishGlyph::Pastry, hex(0xd9a45c), false),
        (_, "black_coffee" | "night_train") => (DishGlyph::Espresso, hex(0x3b2416), true),
        (_, "cappuccino" | "oat_flat_white") => (DishGlyph::Cup, hex(0xd8b48a), true),
        (_, "lemon_tea") => (DishGlyph::TeaGlass, hex(0xc9862e), true),
        (_, "chai") => (DishGlyph::Mug, hex(0xb07a4a), true),
        (_, "hot_chocolate") => (DishGlyph::Mug, hex(0x5e3420), true),
        (_, "svarak") => (DishGlyph::MulledGlass, hex(0x8a1f2c), true),
        (_, "garlic_soup") => (DishGlyph::Bowl, hex(0xe0c27a), true),
        (_, "grandma_kulajda") => (DishGlyph::Bowl, hex(0xefe0b0), true),
        (_, "varenyky" | "bryndzove_pirohy") => (DishGlyph::Dumplings, hex(0xf0dcb0), true),
        (_, "kolac") => (DishGlyph::Pastry, hex(0x7b3f8c), false),
        (_, "medovnik") => (DishGlyph::Cake, hex(0xc98a3e), false),
        (_, "chlebicek") => (DishGlyph::Plate, hex(0xe9a15c), false),
        (_, "topinka") => (DishGlyph::Plate, hex(0xc78a3c), true),
        _ => (DishGlyph::Cup, hex(0xb08a6a), true),
    };
    Dish::new(glyph, tint).steam(steam)
}

/// Display name of a guest and the key of their face (`Ui::register_face`).
fn guest_name(g: &GameScreen, loc: &Locale, guest: u32) -> (String, String) {
    let p = g.world.party_of_guest(guest);
    match p.and_then(|p| p.regular.clone()) {
        Some(r) => {
            let n = loc.tr_owned(&format!("regular.{r}.name"));
            (n.clone(), n)
        }
        None => (
            // a stranger: the description until Karolina knows them, then the name
            super::guest_label::party_label(&g.world, loc, guest),
            // a bounded set of face slots (each baked face is a small texture)
            format!("#guest{}", guest % 32),
        ),
    }
}

fn guest_look(g: &mut GameScreen, id: u32) -> Option<PersonLook> {
    let l = g
        .scene
        .people
        .iter()
        .find(|v| v.agent == AgentRef::Guest(id))
        .map(|v| v.look.clone())?;
    Some(g.scene.look(&l))
}

/// Faces for the rail, the card, the mini-game and the record sleeves. Call at the very
/// start of a frame (baking a face must happen before anything is drawn).
pub fn faces(g: &mut GameScreen, ui: &mut Ui, content: &Content, loc: &Locale) {
    if g.world.phase() == Phase::Night {
        let guests: Vec<u32> = g
            .world
            .ticket_rail(content)
            .iter()
            .map(|t| t.guest)
            .collect();
        for id in guests {
            let (_, key) = guest_name(g, loc, id);
            if let Some(look) = guest_look(g, id) {
                ui.register_face(&key, look);
            }
        }
        for (r, _, _) in g.hands.incoming.clone() {
            ui.register_face(
                &loc.tr_owned(&format!("regular.{r}.name")),
                PersonLook::preset(&r),
            );
        }
        // the card and the mini-game show the guest's name: their face goes under it
        let on_card = g
            .hands
            .order
            .as_ref()
            .map(|o| o.guest)
            .or_else(|| g.hands.minigame.as_ref().map(|m| m.guest));
        if let Some(id) = on_card
            && let Some(look) = guest_look(g, id)
        {
            let (name, _) = guest_name(g, loc, id);
            ui.register_face(&name, look);
        }
    }
    if g.hands.records_open {
        let fans: Vec<String> = g
            .world
            .music()
            .known_tastes
            .iter()
            .map(|(_, r)| r.clone())
            .collect();
        for r in fans {
            ui.register_face(
                &loc.tr_owned(&format!("regular.{r}.name")),
                PersonLook::preset(&r),
            );
        }
    }
}

// ───────────────────────── HUD: rail, tip jar, lull chip ─────────────────────────

/// Rail tickets as `(widget id, party, guest)`, in rail order.
pub fn rail_tickets(g: &GameScreen, content: &Rc<Content>) -> Vec<(u64, u32, u32)> {
    let mut seen: std::collections::HashMap<u32, u64> = Default::default();
    g.world
        .ticket_rail(content)
        .iter()
        .map(|t| {
            let k = seen.entry(t.party).or_insert(0);
            *k += 1;
            (t.party as u64 * 64 + *k, t.party, t.guest)
        })
        .collect()
}

pub fn hud(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    if g.world.phase() != Phase::Night || g.summary.is_some() {
        return;
    }
    // the big panels (the app, the journal, payroll) have the screen; the rail waits
    let big = matches!(
        g.panel,
        Some(super::Panel::App | super::Panel::Journal | super::Panel::Payroll)
    );
    if !big {
        rail(g, ui, cx);
    }
    tip_jar(g, ui, cx);
}

fn rail(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let content = cx.content;
    let loc = cx.loc;
    let tickets = g.world.ticket_rail(content);
    let ids = rail_tickets(g, content);
    let mut out: Vec<Ticket> = Vec::new();
    for (t, (id, _, _)) in tickets.iter().zip(&ids) {
        let (name, face) = guest_name(g, &loc, t.guest);
        let (item, stage) = match &t.stage {
            TicketStage::Order => {
                let wish = g
                    .world
                    .party_of_guest(t.guest)
                    .is_some_and(|p| p.order_kind.is_wish());
                let first = g
                    .world
                    .party_of_guest(t.guest)
                    .and_then(|p| p.wishes.first().cloned())
                    .unwrap_or_default();
                (
                    if wish {
                        TicketItem::Wish
                    } else {
                        TicketItem::Plain(dish_of(content, &first))
                    },
                    loc.tr_owned("ui.rail.stage.order"),
                )
            }
            TicketStage::Wish { wish } => (TicketItem::Wish, loc.tr_owned(&format!("wish.{wish}"))),
            TicketStage::Make { menu, .. } => (
                TicketItem::Plain(dish_of(content, menu)),
                loc.trf(
                    "ui.rail.stage.make",
                    &[("item", loc.tr(&format!("menu.{menu}.name")))],
                ),
            ),
            TicketStage::Serve { menu, .. } => (
                TicketItem::Plain(dish_of(content, menu)),
                loc.trf(
                    "ui.rail.stage.serve",
                    &[("item", loc.tr(&format!("menu.{menu}.name")))],
                ),
            ),
        };
        let owner = match t.owner {
            sim::TicketOwner::Karolina => TicketOwner::Karolina,
            sim::TicketOwner::Helper(h) => TicketOwner::Helper(
                g.world
                    .staff_by_id(h)
                    .map(|s| s.name.clone())
                    .unwrap_or_default(),
            ),
            sim::TicketOwner::Nobody => TicketOwner::Nobody,
        };
        let who = match &owner {
            TicketOwner::Karolina => loc.tr_owned("ui.rail.karolina"),
            TicketOwner::Helper(n) => n.clone(),
            TicketOwner::Nobody => loc.tr_owned("ui.rail.unowned"),
        };
        out.push(Ticket {
            id: *id,
            guest: face,
            label: super::guest_label::rail_label(ui, &g.world, &loc, t.guest),
            tooltip: format!("{name} · {stage} · {who}"),
            item,
            owner,
            wait: t.wait_frac,
            regular: t.regular.is_some(),
            incoming: false,
        });
    }
    // regulars announced ahead: dashed chips
    for (r, party, _) in g.hands.incoming.clone() {
        let name = loc.tr_owned(&format!("regular.{r}.name"));
        out.push(Ticket {
            id: party as u64 * 64 + 63,
            guest: name.clone(),
            label: name.clone(),
            tooltip: loc.trf("ui.rail.incoming", &[("name", &name)]),
            item: TicketItem::Wish,
            owner: TicketOwner::Karolina,
            wait: 0.0,
            regular: true,
            incoming: true,
        });
    }
    if out.is_empty() {
        return;
    }
    // between the clock and the money cards when it fits; a long rail drops below them
    let mut top = vec2(ui.screen().w / 2.0, 52.0);
    let rr = ticket_rail_rect(top, out.len());
    let clash = g
        .anchors
        .iter()
        .filter(|a| matches!(a.0, "hud.clock" | "hud.money"))
        .any(|a| a.1.overlaps(&rr));
    if clash {
        top.y = 12.0 + 64.0 + 6.0;
        // below the cards: between the atmosphere meter (left) and the tip jar (right,
        // up to 260 pt); when even that is too narrow (a large UI scale / text size) the
        // rail goes one row lower
        let mut left = 12.0 + crate::ui::handson::ATMO_SIZE.x + 8.0;
        let right = ui.screen().w - 12.0 - 200.0 - 8.0;
        let rr = ticket_rail_rect(top, out.len());
        let w = rr.w;
        // the column under the meter (camera, "step out", the storm card) when it reaches
        // the rail's height
        for a in g.anchors.iter().filter(|a| {
            matches!(a.0, "hud.camera" | "hud.backstep" | "hud.storm")
                && a.1.y < rr.bottom()
                && a.1.bottom() > rr.y
        }) {
            left = left.max(a.1.right() + 8.0);
        }
        if w <= right - left {
            top.x = top.x.clamp(left + w / 2.0, right - w / 2.0);
        } else {
            top.y += 28.0 + 8.0;
            // one row lower it may still reach the column under the meter: slide right
            let far = ui.screen().w - 12.0 - w / 2.0;
            top.x = top.x.max(left + w / 2.0).min(far.max(left + w / 2.0));
        }
    }
    g.anchors.push(("rail", ticket_rail_rect(top, out.len())));
    let more = loc.tr_owned("ui.rail.title");
    if let Some(RailAction::Focus(id)) =
        ticket_rail(ui, "rail", top, &out, &mut g.hands.rail, &more)
    {
        g.hands.rail.focused = Some(id);
        let Some((_, party, guest)) = ids.iter().find(|t| t.0 == id).copied() else {
            return;
        };
        // [ / ] only move the focus (Enter acts on it); a click claims at once
        let keys = is_key_pressed(KeyCode::LeftBracket) || is_key_pressed(KeyCode::RightBracket);
        if keys {
            g.selected = Some(AgentRef::Guest(guest));
        } else {
            g.intent(content, hands::Intent::Ticket { party, guest });
        }
    }
}

fn tip_jar(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let t = ui.theme.clone();
    let loc = cx.loc;
    let s = ui.screen();
    let tips = g.world.tonight().tips;
    let label = loc.trf(
        "ui.tipjar.tonight",
        &[("amount", &format_money(loc.lang, tips))],
    );
    let st = t.small_medium().color(t.text);
    let rhythm = g.world.rhythm() as u32;
    let rw = if rhythm > 0 { 34.0 } else { 0.0 };
    let w = (ui.fonts.measure(&label, &st) + 44.0 + rw).clamp(120.0, 260.0);
    let r = Rect::new(s.w - 12.0 - w, 12.0 + 64.0 + 6.0, w, 28.0);
    g.anchors.push(("hud.tipjar", r));
    ui.glass(r, &Glass::hud().radius(9.0));
    let pulse = (1.0 - (g.time - g.hands.jar_pulse) / 0.8).clamp(0.0, 1.0);
    if pulse > 0.0 {
        fill_rrect(r, 9.0, with_a(t.amber, 0.16 * pulse));
    }
    let ic = vec2(r.x + 16.0, r.center_y());
    if pulse > 0.0 || rhythm > 0 {
        glow_circle(ic, 12.0 + 4.0 * pulse, with_a(t.amber, 0.25 + 0.3 * pulse));
    }
    draw_icon(Icon::Jar, ic, 15.0, t.amber);
    ui.fonts.draw_in(
        &label,
        Rect::new(r.x + 30.0, r.y, r.w - 38.0 - rw, r.h),
        &st,
        Align::Left,
    );
    if rhythm > 0 {
        let fx = r.right() - rw;
        draw_icon(Icon::Flame, vec2(fx + 7.0, r.center_y()), 13.0, t.amber);
        ui.fonts.draw(
            &format!("×{rhythm}"),
            fx + 15.0,
            ui.fonts.baseline_in(r, &t.key()),
            &t.key().color(t.amber),
        );
    }
    let resp = ui.interact(ui.id("tipjar"), r);
    let tip = if rhythm > 0 {
        loc.trf(
            "ui.tipjar.rhythm",
            &[("percent", &(rhythm * 4).to_string())],
        )
    } else {
        let life = g.world.tips_lifetime();
        let mut t = loc.trf(
            "ui.tipjar.lifetime",
            &[("amount", &format_money(loc.lang, life))],
        );
        // the next lifetime goal (it brings a piece of decor for the hall)
        if let Some(goal) = cx
            .content
            .handson
            .extras
            .tip_goals
            .iter()
            .map(|g| g.lifetime as i64)
            .find(|g| *g > life)
        {
            t.push('\n');
            t.push_str(&loc.trf(
                "ui.tipjar.goal",
                &[("amount", &format_money(loc.lang, goal))],
            ));
        }
        t
    };
    ui.tooltip(&resp, &tip);
    // "It's quiet — ×2?" (the lull director's last suggestion; never speeds up by itself)
    if g.time < g.hands.lull_until && g.speed < 2 && !g.overlay_open() {
        let s2 = crate::settings::keys::label(crate::settings::keys::Action::Speed2);
        let b = Button::new(loc.tr("ui.lull.speed"))
            .secondary()
            .icon(Icon::Fast)
            .shortcut(&s2);
        let bw = ui.button_width(&b);
        let br = Rect::new(s.w - 12.0 - bw, r.bottom() + 6.0, bw, 28.0);
        g.anchors.push(("hud.lull", br));
        if ui.button("lull_speed", br, &b).clicked {
            g.set_speed(2);
            g.hands.lull_until = 0.0;
        }
    }
}

// ───────────────────────── overlays ─────────────────────────

/// The order card, the record picker, the book and the mini-game (in that order, so the
/// mini-game sits on top).
pub fn overlays(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx, dt: f32) {
    if g.summary.is_some() {
        return;
    }
    if g.world.phase() == Phase::Night {
        order(g, ui, cx);
    }
    if g.world.phase() == Phase::Night {
        super::moments_ui::draw(g, ui, cx);
    }
    if g.hands.records_open {
        records(g, ui, cx);
    }
    if g.hands.book.is_some() {
        book(g, ui, cx);
        if g.hands.book.as_ref().is_some_and(|b| b.pick_for.is_none()) {
            super::extras_ui::napkins_page(g, ui, cx);
        }
    }
    if g.hands.minigame.is_some() {
        minigame(g, ui, cx, dt);
    }
}

pub fn hud_avoid(g: &GameScreen) -> Vec<Rect> {
    [
        "hud.clock",
        "hud.money",
        "hud.ticker",
        "hud.tipjar",
        "hud.lull",
        "rail",
    ]
    .iter()
    .filter_map(|n| g.anchors.iter().find(|a| a.0 == *n).map(|a| a.1))
    .chain(
        g.anchors
            .iter()
            .filter(|a| a.0.starts_with("dock."))
            .map(|a| a.1),
    )
    .collect()
}

/// The order card's view of a guest (and the recipe ids behind its chips).
pub fn order_view(
    g: &GameScreen,
    content: &Content,
    loc: &Locale,
    guest: u32,
    highlight: bool,
) -> Option<(OrderView, Vec<String>)> {
    let w = &g.world;
    let p = w.party_of_guest(guest)?;
    let (name, _) = guest_name(g, loc, guest);
    let wish = p.wish_open && p.leader() == Some(guest);
    let wish_def = p
        .order_kind
        .wish()
        .filter(|_| wish)
        .and_then(|id| content.handson.wish(id));
    let text = match wish_def {
        Some(d) => loc.tr_owned(&d.key()),
        None => {
            let item = w
                .order_items()
                .iter()
                .filter(|i| i.party == p.id && i.state == sim::ItemState::Pending && i.by.is_none())
                .min_by_key(|i| (i.guest != guest, i.id))
                .map(|i| i.menu.clone())
                .unwrap_or_default();
            loc.tr_owned(&format!("menu.{item}.name"))
        }
    };
    let matches = w.best_matches(content, guest);
    let ids: Vec<String> = matches.iter().map(|m| m.0.clone()).collect();
    let recipes = matches
        .iter()
        .enumerate()
        .filter_map(|(i, (id, _))| {
            // (the café's own drink lives in the world)
            let r = w.recipe_def(content, id)?;
            Some(RecipeChoice {
                name: loc.tr_owned(&r.name_key()),
                dish: dish_of(content, id),
                tags: r.tags.iter().map(|t| taste_tag(*t)).collect(),
                practised: w.knowledge().recipes.get(id).is_some_and(|s| s.practised),
                highlight: highlight && i == 0,
            })
        })
        .collect();
    let regular = p.regular.clone();
    let wait = w
        .ticket_rail(content)
        .iter()
        .find(|t| t.party == p.id)
        .map_or(0.0, |t| t.wait_frac);
    let helper_brews = w.staff().iter().any(|s| {
        !s.owner
            && s.on_duty()
            && s.duties.iter().any(|d| {
                matches!(
                    d,
                    crate::content::handson::Duty::Bar | crate::content::handson::Duty::Kitchen
                )
            })
    });
    let view = OrderView {
        guest: name,
        subtitle: if regular.is_some() {
            loc.tr_owned("ui.guest.regular")
        } else if wish {
            loc.tr_owned("ui.order.matches")
        } else {
            String::new()
        },
        regular: regular.is_some(),
        trust: regular
            .as_ref()
            .map(|r| w.story().trust.get(r).copied().unwrap_or(0.0) / 100.0),
        wait,
        kind: if wish {
            OrderKind::Wish
        } else {
            OrderKind::Plain
        },
        text,
        want: wish_def
            .map(|d| d.want.iter().map(|t| taste_tag(*t)).collect())
            .unwrap_or_default(),
        avoid: wish_def
            .map(|d| d.avoid.iter().map(|t| taste_tag(*t)).collect())
            .unwrap_or_default(),
        show_hints: true,
        recipes,
        usual: w.usual_available(content, guest).and_then(|r| {
            content
                .handson
                .recipe(&r)
                .map(|d| loc.tr_owned(&d.name_key()))
        }),
        can_hand_off: wish && helper_brews,
        remember: None,
    };
    Some((view, ids))
}

pub fn order_labels(loc: &Locale) -> OrderLabels {
    OrderLabels {
        by_hand: loc.tr_owned("ui.order.by_hand"),
        quick: loc.tr_owned("ui.order.quick"),
        usual: loc.tr_owned("ui.order.usual"),
        hand_off: loc.tr_owned("ui.order.hand_off"),
        book: loc.tr_owned("book.title"),
        close: loc.tr_owned("ui.common.close"),
        plain: loc.tr_owned("ui.order.plain_caption"),
        practised: loc.tr_owned("book.practised"),
        tastes: taste_names(loc),
    }
}

fn order(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let Some(o) = g.hands.order.clone() else {
        return;
    };
    let content = cx.content;
    let loc = cx.loc;
    let highlight = super::coach::wants(g, "pick_recipe");
    let Some((view, ids)) = order_view(g, content, &loc, o.guest, highlight) else {
        g.hands.order = None;
        return;
    };
    let head = g
        .scene
        .head_of(AgentRef::Guest(o.guest), g.time)
        .map(|h| world_to_ui(g, ui, h))
        .unwrap_or(vec2(ui.screen().w / 2.0, ui.screen().h / 2.0));
    let avoid = hud_avoid(g);
    let mut mode = o.mode;
    let out = order_card(
        ui,
        ("order", o.guest),
        head,
        &view,
        &order_labels(&loc),
        &mut mode,
        &avoid,
    );
    // anchors for the coach marks (same layout as the card: footer, then the rows above it)
    let r = out.rect;
    g.anchors.push(("order.card", r));
    let foot_y = r.bottom() - 12.0 - 28.0;
    let n = view.recipes.len().clamp(1, 4) as f32;
    let chip0 = foot_y - 10.0 - n * 42.0 + 4.0;
    g.anchors.push((
        "order.chip.0",
        Rect::new(r.x + 12.0, chip0, r.w - 24.0, 38.0),
    ));
    g.anchors
        .push(("order.mode", Rect::new(r.x + 12.0, foot_y, 196.0, 28.0)));
    if view.usual.is_some() {
        g.anchors.push((
            "order.usual",
            Rect::new(r.x + 12.0, chip0 - 36.0, r.w - 24.0, 30.0),
        ));
        g.hands.tut.bump("usual_offered");
    }
    if let Some(open) = &mut g.hands.order {
        open.mode = mode;
    }
    match out.action {
        Some(OrderAction::Pick(i)) => {
            if let Some(id) = ids.get(i).cloned() {
                g.brew_chosen(content, o.guest, &id, mode);
            }
        }
        Some(OrderAction::Usual) => g.usual_chosen(content, o.guest),
        Some(OrderAction::HandOff) => g.hand_off(content, o.guest),
        Some(OrderAction::OpenBook) => {
            g.hands.order = None;
            g.open_book(Some((o.guest, mode)));
        }
        Some(OrderAction::Close) => {
            g.hands.order = None;
            g.next_card(content);
        }
        None => {}
    }
}

fn minigame(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx, dt: f32) {
    let content = cx.content;
    let loc = cx.loc;
    // the first frame: titles and prompts in the player's language
    if let Some(mut mg) = g.hands.minigame.take() {
        if !mg.named {
            let title = g
                .world
                .recipe_def(content, &mg.recipe)
                .map(|r| loc.tr_owned(&r.name_key()))
                .unwrap_or_default();
            let (face, _) = guest_name(g, &loc, mg.guest);
            let wish = g
                .world
                .party_of_guest(mg.guest)
                .filter(|p| p.leader() == Some(mg.guest))
                .and_then(|p| p.order_kind.wish().map(str::to_string))
                .map(|w| loc.tr_owned(&format!("wish.{w}")))
                .unwrap_or_else(|| {
                    content
                        .handson
                        .recipe(&mg.recipe)
                        .map(|r| loc.tr_owned(&format!("menu.{}.name", r.base_item)))
                        .unwrap_or_default()
                });
            let game = std::mem::replace(&mut mg.game, placeholder());
            mg.game = game
                .with_title(title)
                .with_guest(face, wish)
                .with_text(MiniGameText::from_lookup(|k| loc.tr_owned(k)));
            mg.named = true;
            // the first time a game type (or a new step) plays: its guide card
            if g.hands.autoplay.is_none() {
                mg.game.offer_guide(&cx.settings.minigame_guides);
            }
        }
        g.hands.minigame = Some(mg);
    }
    let paused = g.pause.is_some();
    let auto = g.hands.autoplay;
    let Some(mg) = g.hands.minigame.as_mut() else {
        return;
    };
    let status = if paused {
        MiniGameStatus::Running
    } else if let Some(ap) = auto {
        let input = if ap.skip_minigames {
            crate::minigame::MgInput {
                skip: true,
                ..Default::default()
            }
        } else if ap.keyboard {
            mg.game.autoplay_keyboard_input()
        } else {
            mg.game.autoplay_input()
        };
        mg.game.step(&input, dt)
    } else {
        mg.game.update(ui, dt)
    };
    let area = ui.screen();
    mg.game.draw(ui, area);
    let seen = mg.game.take_seen_guides();
    if !seen.is_empty() {
        cx.settings.minigame_guides.extend(seen);
        let _ = cx.settings.save();
    }
    let card = MiniGame::card_rect(area);
    for s in mg.game.drain_sounds() {
        let c = s.cue();
        if let Some(sfx) = crate::audio::Sfx::from_id(c.id) {
            cx.audio.play_sfx_vol(sfx, c.volume);
        }
    }
    g.anchors.push(("minigame", card));
    g.anchors.push((
        "minigame.skip",
        Rect::new(card.right() - 110.0, card.bottom() - 40.0, 100.0, 30.0),
    ));
    if let MiniGameStatus::Done { quality } = status {
        g.minigame_done(content, quality);
    }
}

/// A throwaway game used while swapping the real one out of its slot.
fn placeholder() -> MiniGame {
    MiniGame::start(
        crate::minigame::MiniGameKind::TeaSteep,
        crate::minigame::MiniGameParams::new(0),
    )
}

// ───────────────────────── grandma's jukebox ─────────────────────────

fn motif(m: Motif) -> SleeveMotif {
    match m {
        Motif::Tram => SleeveMotif::Tram,
        Motif::Raindrops => SleeveMotif::Raindrops,
        Motif::PianoKeys => SleeveMotif::PianoKeys,
        Motif::Sunflower => SleeveMotif::Sunflower,
        Motif::SynthGrid => SleeveMotif::SynthGrid,
        Motif::Rose => SleeveMotif::Rose,
    }
}

fn mood_color(ui: &Ui, m: HallMood) -> Color {
    let t = &ui.theme;
    match m {
        HallMood::Lively => t.amber,
        HallMood::Calm => t.indigo_300,
        HallMood::Dreamy => t.violet_400,
        HallMood::Tender => Color::from_hex(0xf0a6c0),
        HallMood::Nostalgic => Color::from_hex(0xd9b27a),
    }
}

fn records(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let content = cx.content;
    let loc = cx.loc;
    let music = g.world.music().clone();
    let playing_id = match &music.source {
        sim::MusicSource::Record { id, .. } => Some(id.clone()),
        sim::MusicSource::House => None,
    };
    let mut list = vec![RecordView {
        title: loc.tr_owned("record.house.title"),
        artist: loc.tr_owned("record.house.artist"),
        mood: String::new(),
        mood_color: ui.theme.text_dim,
        motif: SleeveMotif::House,
        color: Color::from_hex(0x3b3566),
        locked: false,
        hint: None,
        fans: Vec::new(),
        playing: playing_id.is_none() && g.world.phase() == Phase::Night,
    }];
    let defs = &content.handson.records.records;
    for d in defs {
        let locked = !g.world.records_unlocked().contains(&d.id);
        let fans: Vec<String> = music
            .known_tastes
            .iter()
            .filter(|(rec, _)| rec == &d.id)
            .map(|(_, r)| loc.tr_owned(&format!("regular.{r}.name")))
            .collect();
        list.push(RecordView {
            title: loc.tr_owned(&d.title_key()),
            artist: loc.tr_owned(&d.artist_key()),
            mood: loc.tr_owned(&d.mood.key()),
            mood_color: mood_color(ui, d.mood),
            motif: motif(d.sleeve.motif),
            color: crate::render::parse_hex(&d.sleeve.color).unwrap_or(GRAY),
            locked,
            hint: locked.then(|| loc.tr_owned(&d.hint_key())),
            fans,
            playing: playing_id.as_deref() == Some(d.id.as_str()),
        });
    }
    let repeat = match &music.source {
        sim::MusicSource::Record { repeat, .. } => *repeat,
        _ => !g.hands.play_once,
    };
    let view = RecordsView {
        records: list,
        repeat,
    };
    let labels = RecordLabels {
        title: loc.tr_owned("ui.jukebox.title"),
        repeat: loc.tr_owned("ui.jukebox.repeat"),
        play_once: loc.tr_owned("ui.jukebox.once"),
        now_playing: loc.tr_owned("ui.jukebox.now_playing_short"),
        close: loc.tr_owned("ui.common.close"),
        liked_by: loc.tr_owned("ui.jukebox.liked_by"),
    };
    let jukebox = g.world.furniture().iter().find(|p| {
        content
            .furniture_def(&p.def)
            .is_some_and(|d| d.has_tag("music"))
    });
    let mut anchor = jukebox
        .map(|p| {
            world_to_ui(
                g,
                ui,
                vec2(p.x as f32 + 0.5, p.y as f32 - 0.6) * crate::map::TILE,
            )
        })
        .unwrap_or(vec2(ui.screen().w / 2.0, ui.screen().h / 2.0));
    // keep the picker under the top HUD and the rail (it hangs above its anchor)
    let size = crate::ui::handson::record_picker_size(view.records.len());
    let hud_bottom = g
        .anchors
        .iter()
        .filter(|a| matches!(a.0, "hud.clock" | "hud.money" | "hud.tipjar" | "rail"))
        .map(|a| a.1.bottom())
        .fold(0.0, f32::max);
    anchor.y = anchor.y.max(hud_bottom + 8.0 + size.y + 18.0);
    let action = record_picker(ui, "records", anchor, &view, &labels, &mut g.hands.records);
    match action {
        Some(RecordAction::Play(i)) => {
            let id = if i == 0 {
                None
            } else {
                defs.get(i - 1).map(|d| d.id.clone())
            };
            match g.command(
                content,
                sim::Command::PlayRecord {
                    id,
                    repeat: !g.hands.play_once,
                },
            ) {
                Ok(()) => {
                    g.hands.records_open = false;
                    // putting a record on is the opening ritual's item too
                    let open = g
                        .world
                        .app()
                        .checklist
                        .iter()
                        .any(|c| c.id == "record_on" && !c.done && c.claim.is_none());
                    if open && g.world.phase() == Phase::Night {
                        let _ = g.command(
                            content,
                            sim::Command::TickChecklist {
                                id: "record_on".into(),
                            },
                        );
                    }
                }
                Err(e) if e.contains("NoPower") => g.notices.push(super::Notice::Key {
                    kind: crate::ui::ToastKind::Info,
                    icon: Icon::Vinyl,
                    key: "ui.jukebox.no_power".into(),
                    args: Vec::new(),
                }),
                Err(e) if e.contains("QueueFull") => g.notices.push(super::Notice::Key {
                    kind: crate::ui::ToastKind::Info,
                    icon: Icon::Info,
                    key: "ui.queue.full".into(),
                    args: Vec::new(),
                }),
                Err(_) => {}
            }
        }
        Some(RecordAction::SetRepeat(r)) => g.hands.play_once = !r,
        Some(RecordAction::Close) => g.hands.records_open = false,
        None => {}
    }
}

// ───────────────────────── the recipe book (K) ─────────────────────────

fn book(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let content = cx.content;
    let loc = cx.loc;
    let know = g.world.knowledge().clone();
    let tiers = content.handson.minigames.quality_tiers;
    let mut ids = Vec::new();
    let mut recipes = Vec::new();
    // (+ the café's own drink, on the Specials tab)
    for r in g.world.all_recipes(content) {
        let Some(base) = content.menu_item(&r.base_item) else {
            continue;
        };
        let stats = know.recipes.get(&r.id);
        let known = stats.is_some();
        let section = match r.book_tab(base) {
            BookTab::Drinks => BookSection::Drinks,
            BookTab::Food => BookSection::Food,
            BookTab::Specials => BookSection::Specials,
        };
        let ing = |id: &String| {
            content
                .handson
                .ingredient(id)
                .map(|i| loc.tr_owned(&i.name_key()))
                .unwrap_or_else(|| id.clone())
        };
        let ingredients = if known {
            r.ingredients.iter().map(ing).collect()
        } else {
            // the data's hints plus grandma's margin notes
            g.world
                .revealed_ingredients(content, &r.id)
                .iter()
                .map(ing)
                .collect()
        };
        let patterns = if known && r.minigame == crate::content::handson::MiniGameKind::LatteArt {
            g.world
                .lattes()
                .iter()
                .map(|p| {
                    let k = format!("latte.{p}");
                    if loc.has(&k) {
                        loc.tr_owned(&k)
                    } else {
                        p.clone()
                    }
                })
                .collect()
        } else {
            Vec::new()
        };
        let price = g.world.price_of(content, &r.base_item) + r.price_add;
        recipes.push(BookRecipe {
            section,
            name: loc.tr_owned(&r.name_key()),
            known,
            practised: stats.is_some_and(|s| s.practised),
            new: g.hands.new_recipes.contains(&r.id),
            dish: dish_of(content, &r.id),
            tags: r.tags.iter().map(|t| taste_tag(*t)).collect(),
            ingredient_count: r.ingredients.len() as u8,
            ingredients,
            desc: known.then(|| loc.tr_owned(&r.desc_key())),
            lore: known.then(|| loc.tr_owned(&r.lore_key())),
            minigame: Some(loc.tr_owned(&r.minigame.title_key())),
            base_line: Some(format!(
                "{} · {}",
                loc.tr(&format!("menu.{}.name", r.base_item)),
                format_money(loc.lang, price as i64)
            )),
            stats_line: stats.filter(|s| s.served > 0).map(|s| {
                format!(
                    "{} · {}",
                    loc.trf("book.served", &[("n", &s.served.to_string())]),
                    loc.trf(
                        "book.best",
                        &[(
                            "quality",
                            loc.tr(&format!("minigame.quality.{}", tiers.label(s.best_quality)))
                        )]
                    )
                )
            }),
            source_hint: (!known)
                .then(|| {
                    r.hint
                        .as_ref()
                        .map(|h| loc.tr_owned(&format!("book.hint.{h}")))
                })
                .flatten(),
            patterns,
        });
        ids.push(r.id.clone());
    }
    let pantry: Vec<Ingredient> = content
        .handson
        .recipes
        .ingredients
        .iter()
        .map(|i| Ingredient {
            name: loc.tr_owned(&i.name_key()),
            family: loc.tr_owned(&i.family.key()),
            shelf: match i.shelf {
                crate::content::handson::Shelf::Bar => Shelf::Bar,
                crate::content::handson::Shelf::Kitchen => Shelf::Kitchen,
            },
            unlocked: g.world.pantry().contains(&i.id),
            color: family_color(i.family),
        })
        .collect();
    let jar_ids: Vec<String> = content
        .handson
        .recipes
        .ingredients
        .iter()
        .map(|i| i.id.clone())
        .collect();
    let cost = g.world.experiment_cost(content);
    let try_cost = Some(if cost <= 0 {
        loc.tr_owned("book.free")
    } else {
        loc.trf("book.cost", &[("amount", &format_money(loc.lang, cost))])
    });
    let Some(open) = g.hands.book.as_mut() else {
        return;
    };
    let result = open.note.clone().map(|n| {
        let args = super::resolve_args(&loc, &n.args);
        let mut text = super::world_view::tr_args(&loc, &n.key, &args);
        if let Some(cat) = args.iter().find(|a| a.0 == "cat") {
            text.push(' ');
            text.push_str(&cat.1);
        }
        ExperimentResult {
            kind: n.kind,
            text,
            recipe: n
                .recipe
                .as_ref()
                .and_then(|r| ids.iter().position(|x| x == r)),
        }
    });
    let margin_note = open.margin.as_ref().and_then(|rid| {
        let ing = g.world.revealed_ingredients(content, rid);
        let last = ing.last()?;
        let name = content.handson.ingredient(last)?.name_key();
        Some(loc.trf("book.margin_note", &[("ingredient", loc.tr(&name))]))
    });
    let view = BookView {
        recipes,
        pantry,
        result,
        pick_mode: open.pick_for.is_some(),
        try_cost,
        margin_note,
    };
    let labels = BookLabels {
        title: loc.tr_owned("book.title"),
        tabs: [
            loc.tr_owned("book.tab.drinks"),
            loc.tr_owned("book.tab.food"),
            loc.tr_owned("book.tab.specials"),
            loc.tr_owned("book.tab.pantry"),
        ],
        pick: loc.tr_owned("book.pick"),
        ingredients: loc.tr_owned("book.ingredients"),
        tastes_head: loc.tr_owned("ui.book.tastes"),
        minigame: loc.tr_owned("ui.book.minigame"),
        patterns: loc.tr_owned("book.patterns"),
        unknown: loc.tr_owned("book.unknown"),
        where_to_learn: loc.tr_owned("ui.book.where"),
        practised: loc.tr_owned("book.practised"),
        try_label: loc.tr_owned("book.try"),
        clear: loc.tr_owned("ui.book.clear"),
        slots_hint: loc.tr_owned("ui.book.slots"),
        bar_shelf: loc.tr_owned("ui.book.bar_shelf"),
        // the kitchen shelf's heading carries the fridge: fresh stock / the room for it
        kitchen_shelf: {
            let fr = g.world.fridge(content);
            let shelf = loc.tr_owned("ui.book.kitchen_shelf");
            if fr.pieces > 0 {
                let f = loc.trf(
                    "fridge.pantry",
                    &[
                        ("fresh", &crate::i18n::format_int(loc.lang, fr.fresh)),
                        ("cap", &format_money(loc.lang, fr.capacity)),
                    ],
                );
                format!("{shelf} · {f}")
            } else {
                shelf
            }
        },
        locked_jar: loc.tr_owned("book.locked_jar"),
        empty_section: loc.tr_owned("ui.book.empty"),
        close: loc.tr_owned("ui.common.close"),
        tastes: taste_names(&loc),
    };
    let area = ui.screen();
    let tab_before = open.state.tab;
    let action = recipe_book(ui, "book", area, &view, &labels, &mut open.state);
    if open.state.tab != tab_before {
        cx.audio.play_sfx_vol(crate::audio::Sfx::PageFlip, 0.7);
    }
    let bs = crate::ui::handson::BOOK_SIZE;
    g.anchors.push((
        "book",
        Rect::new((area.w - bs.x) / 2.0, (area.h - bs.y) / 2.0, bs.x, bs.y),
    ));
    match action {
        Some(BookAction::Pick(i)) => {
            let pick_for = open.pick_for;
            let mode = open.mode;
            if let (Some(guest), Some(id)) = (pick_for, ids.get(i).cloned()) {
                if g.can_brew(content, guest, &id) {
                    g.brew_chosen(content, guest, &id, mode);
                } else if let Some(open) = g.hands.book.as_mut() {
                    open.note = Some(super::hands::BookNote {
                        kind: ExperimentKind::Nothing,
                        key: "ui.book.not_for_this".into(),
                        args: Vec::new(),
                        recipe: None,
                    });
                }
            }
        }
        Some(BookAction::Try(slots)) => {
            let ingredients: Vec<String> = slots
                .iter()
                .flatten()
                .filter_map(|i| jar_ids.get(*i).cloned())
                .collect();
            g.hands.tut.bump("experiment_tried");
            match g.command(content, Command::TryExperiment { ingredients }) {
                Ok(()) => {
                    if g.world.phase() == Phase::Night
                        && let Some(open) = g.hands.book.as_mut()
                    {
                        // at night she tries it at the counter: the result follows
                        open.note = Some(super::hands::BookNote {
                            kind: ExperimentKind::Nothing,
                            key: "ui.book.trying".into(),
                            args: Vec::new(),
                            recipe: None,
                        });
                        open.state.slots.clear();
                    } else if let Some(open) = g.hands.book.as_mut() {
                        open.state.slots.clear();
                    }
                }
                Err(e) => super::panels::command_error(g, &loc, &e),
            }
        }
        Some(BookAction::Opened(i)) => {
            if let Some(id) = ids.get(i) {
                g.hands.new_recipes.remove(id);
            }
        }
        Some(BookAction::Close) => {
            let back = open.pick_for;
            g.hands.book = None;
            if let Some(guest) = back {
                g.open_card(content, guest);
            }
        }
        None => {}
    }
}

fn family_color(f: crate::content::handson::Family) -> Color {
    use crate::content::handson::Family as F;
    Color::from_hex(match f {
        F::Coffee => 0x5a3825,
        F::Milk => 0xf2ead8,
        F::Tea => 0xb9863a,
        F::Sweet => 0xe8b04a,
        F::Spice => 0xb5552c,
        F::Fruit => 0xe0703a,
        F::Cold => 0x9fd3e6,
        F::Savoury => 0x8aa05a,
        F::Dough => 0xe2c48e,
    })
}
