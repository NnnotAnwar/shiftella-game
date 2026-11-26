//! The order card (a guest's wish → pick a recipe → Quick / By hand) and the ticket rail
//! (pending orders across the top of the screen). Design §1.2, §6.3, §7.4.

use macroquad::prelude::*;

use super::common::{
    Dish, TasteMark, TasteNames, TasteTag, corner_key, draw_dish, place_near, pointer, ring,
    taste_dot, taste_row, wait_color,
};
use crate::ui::draw::{
    self, Paint, fill_circle, fill_rrect, mix, shadow_rrect, stroke_circle, stroke_rrect, with_a,
};
use crate::ui::layout::RectExt;
use crate::ui::{
    Align, Glass, Icon, IconButton, TextStyle, Ui, UiSound, Weight, draw_icon, ease_out,
};

// ---------------------------------------------------------------------------
// View models
// ---------------------------------------------------------------------------

/// Is the guest asking for something by mood, or for a plain menu item?
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderKind {
    Wish,
    Plain,
}

/// How Karolina makes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BrewMode {
    /// The mini-game (default for wishes and regulars).
    #[default]
    ByHand,
    /// No mini-game, lower quality (default for plain orders).
    Quick,
}

/// One suggested recipe on the order card.
#[derive(Clone, Debug)]
pub struct RecipeChoice {
    pub name: String,
    pub dish: Dish,
    pub tags: Vec<TasteTag>,
    /// Karolina has practised it (small star).
    pub practised: bool,
    /// Tutorial highlight ring.
    pub highlight: bool,
}

/// Everything the order card shows. Strings are already localised by the caller.
#[derive(Clone, Debug)]
pub struct OrderView {
    /// Display name; also the avatar's face key (see `Ui::register_face`).
    pub guest: String,
    /// "Regular · Mirek's corner seat", "Traveller"…
    pub subtitle: String,
    pub regular: bool,
    /// Regulars' trust 0..1 (a small heart meter).
    pub trust: Option<f32>,
    /// Wait clock 0..1 (ring around the portrait).
    pub wait: f32,
    pub kind: OrderKind,
    /// The wish text, or the plain item's name.
    pub text: String,
    pub want: Vec<TasteTag>,
    pub avoid: Vec<TasteTag>,
    /// Setting `show_taste_hints`.
    pub show_hints: bool,
    /// Up to 4 suggestions.
    pub recipes: Vec<RecipeChoice>,
    /// "The usual?" (regulars at trust ≥ 50): the favourite recipe's name.
    pub usual: Option<String>,
    /// A helper with register + bar/kitchen is on shift.
    pub can_hand_off: bool,
    /// A handover note mentioned this guest ("Mirek seemed tired"): the tooltip text.
    pub remember: Option<String>,
}

/// Player-visible labels (the caller translates them).
#[derive(Clone, Debug, Default)]
pub struct OrderLabels {
    pub by_hand: String,
    pub quick: String,
    pub usual: String,
    pub hand_off: String,
    pub book: String,
    pub close: String,
    /// "Plain order" caption above a plain item.
    pub plain: String,
    pub practised: String,
    pub tastes: TasteNames,
}

/// What the player did on the card this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderAction {
    /// Brew recipe `i` with the current [`BrewMode`].
    Pick(usize),
    /// The usual, by hand.
    Usual,
    HandOff,
    /// Open the recipe book in pick mode (K).
    OpenBook,
    Close,
}

pub struct OrderOut {
    pub action: Option<OrderAction>,
    pub rect: Rect,
    /// The mode toggle changed (remember it per order kind).
    pub mode_changed: bool,
}

const W: f32 = 336.0;
const PAD: f32 = 12.0;
const ROW_H: f32 = 38.0;
const ROW_GAP: f32 = 4.0;

fn wish_style(ui: &Ui) -> TextStyle {
    ui.theme.body().line_height(1.38)
}

/// Height of the card for `v` (so the caller can place it before drawing).
pub fn order_card_size(ui: &Ui, v: &OrderView) -> Vec2 {
    let st = wish_style(ui);
    let lines = ui.fonts.wrap(&v.text, text_w(v), &st).len().clamp(1, 3);
    let mut h = PAD + 36.0 + 8.0;
    h += bubble_h(lines, v) + 8.0;
    if v.usual.is_some() {
        h += 30.0 + 6.0;
    }
    let rows = v.recipes.len().clamp(1, 4) as f32;
    h += rows * (ROW_H + ROW_GAP) - ROW_GAP + 10.0;
    h += 28.0 + PAD;
    vec2(W, h.ceil())
}

fn bubble_h(lines: usize, v: &OrderView) -> f32 {
    let taste =
        v.kind == OrderKind::Wish && v.show_hints && !(v.want.is_empty() && v.avoid.is_empty());
    let lead = if v.kind == OrderKind::Plain {
        14.0
    } else {
        0.0
    };
    10.0 + lead + lines as f32 * 18.0 + if taste { 26.0 } else { 0.0 } + 8.0
}

/// Width of the wish text inside the bubble (room for the "remember" note mark).
fn text_w(v: &OrderView) -> f32 {
    W - PAD * 2.0 - 20.0 - if v.remember.is_some() { 20.0 } else { 0.0 }
}

/// The order card, placed next to the guest at `anchor` (UI units), away from `avoid`
/// (the rail, other open cards). Keyboard while open:
/// **1–4** pick, **Q** toggle Quick/By hand, **U** the usual, **H** hand off, **K** book,
/// **Esc** close.
pub fn order_card(
    ui: &mut Ui,
    key: impl std::hash::Hash,
    anchor: Vec2,
    v: &OrderView,
    labels: &OrderLabels,
    mode: &mut BrewMode,
    avoid: &[Rect],
) -> OrderOut {
    let t = ui.theme.clone();
    let id = ui.id(key);
    let size = order_card_size(ui, v);
    let base = place_near(ui.screen(), anchor, size, 22.0, avoid);
    let k = ui.ease_value(id.with("appear"), 1.0, 0.07);
    let r = base.translate(vec2(0.0, (1.0 - ease_out(k)) * 8.0));
    draw::push_alpha(k);
    pointer(r, anchor, t.glass_solid_top);
    ui.glass(r, &Glass::panel().solid().radius(12.0));
    let mut out = OrderOut {
        action: None,
        rect: r,
        mode_changed: false,
    };
    let inner = r.inset(PAD);

    // --- header: portrait, name, trust, close --------------------------------
    let head = inner.with_h(36.0);
    let pc = vec2(head.x + 18.0, head.center_y());
    ring(
        pc,
        19.0,
        2.0,
        v.wait,
        Color::new(1.0, 1.0, 1.0, 0.08),
        wait_color(&t, v.wait),
    );
    if v.regular {
        stroke_circle(pc, 22.0, 1.2, with_a(t.amber, 0.65));
    }
    ui.avatar(pc, 32.0, &v.guest, None, false);
    let close = Rect::new(head.right() - 22.0, head.y + 2.0, 22.0, 22.0);
    let tx = head.x + 46.0;
    let tw = close.x - tx - 6.0;
    ui.fonts.draw_in(
        &v.guest,
        Rect::new(tx, head.y, tw, 20.0),
        &t.h2(),
        Align::Left,
    );
    let mut sx = tx;
    if let Some(tr) = v.trust {
        draw_icon(Icon::Heart, vec2(sx + 5.0, head.y + 28.0), 11.0, t.amber);
        let bar = Rect::new(sx + 13.0, head.y + 26.5, 34.0, 3.0);
        ui.progress(bar, tr, t.amber);
        sx += 54.0;
    }
    ui.fonts.draw_in(
        &v.subtitle,
        Rect::new(sx, head.y + 20.0, (close.x - sx - 4.0).max(0.0), 16.0),
        &t.small(),
        Align::Left,
    );
    if ui
        .icon_button(
            id.with("close"),
            close,
            Icon::Close,
            &IconButton::ghost().size(14.0).tooltip(&labels.close),
        )
        .clicked
    {
        out.action = Some(OrderAction::Close);
    }

    // --- the wish bubble ---------------------------------------------------
    let st = wish_style(ui);
    let lines = ui.fonts.wrap(&v.text, text_w(v), &st);
    let n = lines.len().clamp(1, 3);
    let bub = Rect::new(inner.x, head.bottom() + 8.0, inner.w, bubble_h(n, v));
    fill_rrect(
        bub,
        10.0,
        Paint::vertical(
            bub,
            Color::new(1.0, 1.0, 1.0, 0.075),
            Color::new(1.0, 1.0, 1.0, 0.035),
        ),
    );
    stroke_rrect(bub, 10.0, 1.0, Color::new(1.0, 1.0, 1.0, 0.07));
    // little tail towards the portrait
    draw::fill_convex(
        &[
            vec2(bub.x + 14.0, bub.y + 0.5),
            vec2(bub.x + 20.0, bub.y - 6.0),
            vec2(bub.x + 26.0, bub.y + 0.5),
        ],
        Paint::Solid(Color::new(1.0, 1.0, 1.0, 0.075)),
    );
    let mut ty = bub.y + 10.0;
    if v.kind == OrderKind::Plain {
        ui.fonts.draw_in(
            &labels.plain.to_uppercase(),
            Rect::new(bub.x + 10.0, ty - 2.0, bub.w - 20.0, 14.0),
            &t.micro(),
            Align::Left,
        );
        ty += 14.0;
    }
    let text_st = if v.kind == OrderKind::Plain {
        t.strong()
    } else {
        st
    };
    ui.fonts.draw_lines(
        &lines,
        Rect::new(bub.x + 10.0, ty, text_w(v), n as f32 * 18.0),
        &text_st.line_height(1.38),
        Align::Left,
        Some(n),
        usize::MAX,
    );
    ty += n as f32 * 18.0;
    if v.kind == OrderKind::Wish && v.show_hints && !(v.want.is_empty() && v.avoid.is_empty()) {
        let mut tags: Vec<(TasteTag, TasteMark)> =
            v.want.iter().map(|t| (*t, TasteMark::Want)).collect();
        tags.extend(v.avoid.iter().map(|t| (*t, TasteMark::Avoid)));
        taste_row(
            ui,
            id.with("wish_tastes"),
            bub.x + 10.0,
            ty + 13.0,
            18.0,
            &tags,
            Some(&labels.tastes),
        );
    }
    if let Some(note) = &v.remember {
        let nr = Rect::new(bub.right() - 24.0, bub.y + 4.0, 20.0, 20.0);
        let resp = ui.interact(id.with("remember"), nr);
        draw_icon(Icon::Note, nr.center(), 13.0, t.amber);
        ui.tooltip(&resp, note);
    }
    let mut y = bub.bottom() + 8.0;

    // --- the usual ---------------------------------------------------------
    if let Some(fav) = &v.usual {
        let ur = Rect::new(inner.x, y, inner.w, 30.0);
        let resp = ui.interact(id.with("usual"), ur);
        let h = resp.hover_t;
        shadow_rrect(
            ur,
            8.0,
            draw::Shadow::new(0.0, 2.0, 10.0, -1.0, with_a(t.amber, 0.18 + 0.15 * h)),
        );
        fill_rrect(
            ur,
            8.0,
            Paint::horizontal(
                ur,
                with_a(t.amber, 0.2 + 0.08 * h),
                with_a(t.amber_deep, 0.1 + 0.06 * h),
            ),
        );
        stroke_rrect(ur, 8.0, 1.0, with_a(t.amber, 0.38 + 0.2 * h));
        draw_icon(Icon::Star, vec2(ur.x + 15.0, ur.center_y()), 14.0, t.amber);
        let ls = t.strong().color(mix(t.amber, WHITE, 0.35));
        let lw = ui.fonts.draw_in(
            &labels.usual,
            Rect::new(ur.x + 28.0, ur.y, ur.w * 0.5, ur.h),
            &ls,
            Align::Left,
        );
        let kw = ui.keycap_width("U");
        let fr = Rect::new(
            ur.x + 36.0 + lw,
            ur.y,
            ur.right() - kw - 14.0 - (ur.x + 36.0 + lw),
            ur.h,
        );
        ui.fonts
            .draw_in(fav, fr, &t.small().color(t.text), Align::Left);
        ui.keycap(vec2(ur.right() - kw - 7.0, ur.center_y()), "U", false);
        if resp.clicked {
            ui.sound(UiSound::Click);
            out.action = Some(OrderAction::Usual);
        }
        y += 36.0;
    }

    // --- recipe rows (1–4) ------------------------------------------------
    let count = v.recipes.len().min(4);
    for (i, rc) in v.recipes.iter().take(4).enumerate() {
        let cell = Rect::new(inner.x, y + i as f32 * (ROW_H + ROW_GAP), inner.w, ROW_H);
        let resp = ui.interact(id.with(("chip", i)), cell);
        let h = resp.hover_t;
        let cr = cell.translate(vec2(0.0, -h + resp.press_t * 0.5));
        if h > 0.01 {
            shadow_rrect(
                cr,
                9.0,
                draw::Shadow::new(0.0, 3.0, 12.0, -1.0, with_a(t.indigo, 0.25 * h)),
            );
        }
        fill_rrect(
            cr,
            9.0,
            Paint::vertical(
                cr,
                Color::new(1.0, 1.0, 1.0, 0.065 + 0.05 * h),
                Color::new(1.0, 1.0, 1.0, 0.03 + 0.03 * h),
            ),
        );
        stroke_rrect(
            cr,
            9.0,
            1.0,
            mix(
                Color::new(1.0, 1.0, 1.0, 0.08),
                with_a(t.indigo_300, 0.7),
                h,
            ),
        );
        if rc.highlight {
            let pulse = 0.5 + 0.5 * ((ui.time() * 2.6).sin() as f32);
            stroke_rrect(
                cr.grow(3.0),
                11.0,
                1.6,
                with_a(t.indigo_300, 0.45 + 0.4 * pulse),
            );
        }
        corner_key(
            ui,
            vec2(cr.x + 8.0, cr.center_y() - 7.0),
            &(i + 1).to_string(),
        );
        let gc = vec2(cr.x + 44.0, cr.center_y());
        draw::glow_circle(gc, 20.0, with_a(rc.dish.tint, 0.10 + 0.10 * h));
        draw_dish(gc, 30.0, rc.dish, None, ui.time());
        // tags right-aligned, then the practised star
        let star_x = cr.right() - 16.0;
        let tags: Vec<TasteTag> = rc.tags.iter().take(4).copied().collect();
        let tags_w = tags.len() as f32 * 16.0;
        let mut tx = star_x - 12.0 - tags_w;
        let name_r = Rect::new(cr.x + 66.0, cr.y, (tx - 8.0 - (cr.x + 66.0)).max(0.0), cr.h);
        ui.fonts
            .draw_in(&rc.name, name_r, &t.strong().size(12.5), Align::Left);
        for tag in &tags {
            let lit = v.want.contains(tag);
            let m = if lit && h > 0.01 {
                TasteMark::Match
            } else if lit && v.show_hints {
                TasteMark::Want
            } else {
                TasteMark::Plain
            };
            taste_dot(vec2(tx + 7.0, cr.center_y()), 14.0, *tag, m);
            tx += 16.0;
        }
        if rc.practised {
            draw_icon(
                Icon::Star,
                vec2(star_x, cr.center_y()),
                11.0,
                with_a(t.amber, 0.9),
            );
        }
        if resp.clicked {
            ui.sound(UiSound::Click);
            out.action = Some(OrderAction::Pick(i));
        }
        if rc.practised {
            ui.tooltip(&resp, &labels.practised);
        }
    }
    y += count.max(1) as f32 * (ROW_H + ROW_GAP) - ROW_GAP + 10.0;

    // --- footer: mode toggle, hand off, book --------------------------------
    let foot = Rect::new(inner.x, y, inner.w, 28.0);
    let mut sel = if *mode == BrewMode::ByHand { 0 } else { 1 };
    let seg_w = (foot.w - 28.0 * 2.0 - 12.0).min(196.0);
    let seg = Rect::new(foot.x, foot.y, seg_w, 28.0);
    if ui
        .segmented(
            id.with("mode"),
            seg,
            &[labels.by_hand.as_str(), labels.quick.as_str()],
            &mut sel,
        )
        .changed
    {
        *mode = if sel == 0 {
            BrewMode::ByHand
        } else {
            BrewMode::Quick
        };
        out.mode_changed = true;
    }
    let book_r = Rect::new(foot.right() - 28.0, foot.y, 28.0, 28.0);
    let book_tip = format!("{} · K", labels.book);
    if ui
        .icon_button(
            id.with("book"),
            book_r,
            Icon::Book,
            &IconButton::secondary().size(16.0).tooltip(&book_tip),
        )
        .clicked
    {
        out.action = Some(OrderAction::OpenBook);
    }
    if v.can_hand_off {
        let hr = Rect::new(book_r.x - 34.0, foot.y, 28.0, 28.0);
        let tip = format!("{} · H", labels.hand_off);
        if ui
            .icon_button(
                id.with("handoff"),
                hr,
                Icon::Forward,
                &IconButton::secondary().size(16.0).tooltip(&tip),
            )
            .clicked
        {
            out.action = Some(OrderAction::HandOff);
        }
    }
    draw::pop_alpha();

    // --- keyboard --------------------------------------------------------
    let num = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4];
    for (i, kc) in num.iter().enumerate().take(count) {
        if ui.take_key(*kc) {
            out.action = Some(OrderAction::Pick(i));
        }
    }
    if ui.take_key(KeyCode::Q) {
        *mode = if *mode == BrewMode::ByHand {
            BrewMode::Quick
        } else {
            BrewMode::ByHand
        };
        out.mode_changed = true;
        ui.sound(UiSound::Toggle);
    }
    if v.usual.is_some() && ui.take_key(KeyCode::U) {
        out.action = Some(OrderAction::Usual);
    }
    if v.can_hand_off && ui.take_key(KeyCode::H) {
        out.action = Some(OrderAction::HandOff);
    }
    if ui.take_key(KeyCode::K) {
        out.action = Some(OrderAction::OpenBook);
    }
    if ui.esc() {
        out.action = Some(OrderAction::Close);
    }
    out
}

// ---------------------------------------------------------------------------
// Ticket rail
// ---------------------------------------------------------------------------

/// Who is on a ticket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TicketOwner {
    Karolina,
    /// A helper (their name; the badge shows the initial).
    Helper(String),
    /// Nobody holds the duty: hollow ring, Karolina should take it.
    Nobody,
}

/// What the ticket is for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TicketItem {
    /// A wish (sparkle badge).
    Wish,
    /// A plain item (its dish glyph).
    Plain(Dish),
}

/// One chip on the rail.
#[derive(Clone, Debug)]
pub struct Ticket {
    /// The caller's id (party / guest), returned in [`RailAction`].
    pub id: u64,
    pub guest: String,
    /// Short line under the portrait ("Mirek", "to go").
    pub label: String,
    /// Full hover text ("Mirek · wish · waiting 4 min").
    pub tooltip: String,
    pub item: TicketItem,
    pub owner: TicketOwner,
    /// Wait clock 0..1.
    pub wait: f32,
    pub regular: bool,
    /// A regular announced ahead ("Radek — to go"): dashed, translucent.
    pub incoming: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RailAction {
    /// Clicked (or chosen with `[` / `]`): claim the order / focus the camera.
    Focus(u64),
}

/// Rail memory (keyboard cursor).
#[derive(Clone, Debug, Default)]
pub struct RailState {
    pub focused: Option<u64>,
}

const CHIP_W: f32 = 50.0;
const CHIP_H: f32 = 58.0;
const MAX_CHIPS: usize = 8;

/// Natural size of the rail for `n` tickets.
pub fn ticket_rail_size(n: usize) -> Vec2 {
    let shown = n.min(MAX_CHIPS) + usize::from(n > MAX_CHIPS);
    let w = shown.max(1) as f32 * (CHIP_W + 4.0) - 4.0 + 12.0;
    vec2(w, CHIP_H + 12.0)
}

/// The rail's rect for `n` tickets under `top` (see [`ticket_rail`]).
pub fn ticket_rail_rect(top: Vec2, n: usize) -> Rect {
    let size = ticket_rail_size(n);
    Rect::new((top.x - size.x / 2.0).round(), top.y, size.x, size.y)
}

/// The ticket rail, centred horizontally under `top` (its top edge at `top.y`, centred on
/// `top.x`). Empty → draws nothing. Keys: **[** / **]** move the focus.
pub fn ticket_rail(
    ui: &mut Ui,
    key: impl std::hash::Hash,
    top: Vec2,
    tickets: &[Ticket],
    state: &mut RailState,
    more_tooltip: &str,
) -> Option<RailAction> {
    if tickets.is_empty() {
        return None;
    }
    let t = ui.theme.clone();
    let id = ui.id(key);
    let r = ticket_rail_rect(top, tickets.len());
    ui.glass(r, &Glass::hud().radius(14.0));
    let mut action = None;
    let shown = tickets.len().min(MAX_CHIPS);
    for (i, tk) in tickets.iter().take(shown).enumerate() {
        let cell = Rect::new(
            r.x + 6.0 + i as f32 * (CHIP_W + 4.0),
            r.y + 6.0,
            CHIP_W,
            CHIP_H,
        );
        let resp = ui.interact(id.with(tk.id), cell);
        let focused = state.focused == Some(tk.id);
        let h = resp.hover_t.max(if focused { 0.6 } else { 0.0 });
        if tk.incoming {
            draw::push_alpha(0.72);
        }
        if h > 0.01 {
            fill_rrect(cell, 10.0, Color::new(1.0, 1.0, 1.0, 0.06 * h));
        }
        if focused {
            stroke_rrect(cell, 10.0, 1.2, with_a(t.indigo_300, 0.8));
        }
        let pc = vec2(cell.x + cell.w / 2.0, cell.y + 22.0);
        if tk.incoming {
            super::common::dashed_rrect(
                Rect::new(pc.x - 19.0, pc.y - 19.0, 38.0, 38.0),
                19.0,
                1.2,
                3.0,
                with_a(t.amber, 0.8),
            );
        } else {
            ring(
                pc,
                18.0,
                2.4,
                tk.wait,
                Color::new(1.0, 1.0, 1.0, 0.09),
                wait_color(&t, tk.wait),
            );
        }
        if tk.regular {
            stroke_circle(pc, 15.2, 1.4, t.amber);
        }
        ui.avatar(pc, 28.0, &tk.guest, None, false);
        // item badge (bottom-right of the portrait)
        let ib = pc + vec2(12.0, 11.0);
        fill_circle(ib, 8.0, t.glass_solid_top);
        stroke_circle(ib, 8.0, 1.0, Color::new(1.0, 1.0, 1.0, 0.16));
        match tk.item {
            TicketItem::Wish => draw_icon(Icon::Sparkle, ib, 10.0, t.amber),
            TicketItem::Plain(d) => draw_dish(ib, 15.0, d, None, 0.0),
        }
        // owner badge (top-right)
        let ob = pc + vec2(13.0, -12.0);
        match &tk.owner {
            TicketOwner::Karolina => {
                fill_circle(
                    ob,
                    7.0,
                    Paint::vertical(
                        Rect::new(ob.x - 7.0, ob.y - 7.0, 14.0, 14.0),
                        t.indigo_400,
                        t.violet,
                    ),
                );
                draw_icon(Icon::User, ob, 9.0, WHITE);
            }
            TicketOwner::Helper(name) => {
                let ini: String = name
                    .chars()
                    .next()
                    .map(|c| c.to_uppercase().collect())
                    .unwrap_or_default();
                fill_circle(ob, 7.0, t.glass_solid_top);
                stroke_circle(ob, 7.0, 1.0, with_a(t.violet_400, 0.9));
                ui.fonts.draw_in(
                    &ini,
                    Rect::new(ob.x - 7.0, ob.y - 7.0, 14.0, 14.0),
                    &TextStyle::new(9.0, Weight::Bold, t.violet_400),
                    Align::Center,
                );
            }
            TicketOwner::Nobody => {
                stroke_circle(ob, 6.0, 1.4, with_a(t.text_dim, 0.9));
            }
        }
        let ls = TextStyle::new(
            10.5,
            Weight::Medium,
            if tk.incoming { t.amber } else { t.text_dim },
        );
        ui.fonts.draw_in(
            &tk.label,
            Rect::new(cell.x + 2.0, cell.bottom() - 15.0, cell.w - 4.0, 13.0),
            &ls,
            Align::Center,
        );
        if tk.incoming {
            draw::pop_alpha();
        }
        ui.tooltip(&resp, &tk.tooltip);
        if resp.clicked {
            ui.sound(UiSound::Click);
            state.focused = Some(tk.id);
            action = Some(RailAction::Focus(tk.id));
        }
    }
    if tickets.len() > MAX_CHIPS {
        let cell = Rect::new(
            r.x + 6.0 + shown as f32 * (CHIP_W + 4.0),
            r.y + 6.0,
            CHIP_W,
            CHIP_H,
        );
        let resp = ui.interact(id.with("more"), cell);
        let label = format!("+{}", tickets.len() - MAX_CHIPS);
        let c = vec2(cell.x + cell.w / 2.0, cell.y + 22.0);
        fill_circle(
            c,
            15.0,
            Color::new(1.0, 1.0, 1.0, 0.07 + 0.05 * resp.hover_t),
        );
        ui.fonts.draw_in(
            &label,
            Rect::new(c.x - 15.0, c.y - 15.0, 30.0, 30.0),
            &t.strong(),
            Align::Center,
        );
        ui.tooltip(&resp, more_tooltip);
    }
    // keyboard: [ / ] cycle through the tickets (claims the focused one)
    let step = if ui.take_key(KeyCode::RightBracket) {
        1
    } else if ui.take_key(KeyCode::LeftBracket) {
        -1
    } else {
        0
    };
    if step != 0 {
        let n = tickets.len() as i64;
        let cur = state
            .focused
            .and_then(|f| tickets.iter().position(|t| t.id == f))
            .map(|i| i as i64);
        let next = match cur {
            Some(i) => (i + step).rem_euclid(n),
            None if step > 0 => 0,
            None => n - 1,
        } as usize;
        state.focused = Some(tickets[next].id);
        action = Some(RailAction::Focus(tickets[next].id));
    }
    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rail_size_caps_at_eight_plus_more() {
        let a = ticket_rail_size(3);
        let b = ticket_rail_size(8);
        let c = ticket_rail_size(20);
        assert!(a.x < b.x);
        assert!((c.x - b.x - (CHIP_W + 4.0)).abs() < 1e-3);
        assert_eq!(ticket_rail_size(0).y, CHIP_H + 12.0);
    }
}
