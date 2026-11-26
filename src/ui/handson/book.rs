//! The recipe book (K), design §4.4–4.5: a two-page glass book. Left page: tabs
//! (Drinks · Food · Specials · Pantry) and a grid of recipe cards — known ones illustrated,
//! undiscovered ones as indigo silhouettes with taste icons and ingredient-count dots.
//! Right page: the selected recipe (Lida's handwritten lore, ingredients, tastes,
//! mini-game, stats) or, on the Pantry tab, the experiments workbench (2–3 slots, Try,
//! a result card and grandma's margin note).
//!
//! Keyboard: **1–4** tabs, arrows move, **Enter** pick (pick mode) / add an ingredient,
//! **Backspace** remove, **T** try, **Esc** close.

use macroquad::prelude::*;

use super::common::{
    Dish, TasteMark, TasteNames, TasteTag, dashed_rrect, draw_dish, draw_handwritten_block,
    handwritten_height, mini_chip, mini_chip_width, taste_dot,
};
use crate::ui::draw::{
    self, Paint, fill_circle, fill_rrect, mix, shadow_rrect, stroke_circle, stroke_rrect, with_a,
};
use crate::ui::layout::RectExt;
use crate::ui::{
    Align, Button, Glass, Icon, IconButton, Tab, TextStyle, Ui, UiSound, Weight, draw_icon,
    ease_out,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BookSection {
    Drinks,
    Food,
    Specials,
}

#[derive(Clone, Debug)]
pub struct BookRecipe {
    pub section: BookSection,
    pub name: String,
    pub known: bool,
    pub practised: bool,
    /// Discovered recently (a small amber dot until opened).
    pub new: bool,
    pub dish: Dish,
    pub tags: Vec<TasteTag>,
    pub ingredient_count: u8,
    /// Known: all ingredient names. Unknown: the ones Lida's margin notes revealed.
    pub ingredients: Vec<String>,
    pub desc: Option<String>,
    /// Lida's handwritten line (amber, slanted).
    pub lore: Option<String>,
    /// Mini-game name ("Pour-over").
    pub minigame: Option<String>,
    /// "Black coffee · 45 Kč".
    pub base_line: Option<String>,
    /// "Served 12× · best: Beautiful".
    pub stats_line: Option<String>,
    /// Unknown: where to learn it ("Someone at the taxi rank might know").
    pub source_hint: Option<String>,
    /// Unlocked latte patterns.
    pub patterns: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shelf {
    Bar,
    Kitchen,
}

#[derive(Clone, Debug)]
pub struct Ingredient {
    pub name: String,
    /// "Tea", "Sweet"… (tooltip).
    pub family: String,
    pub shelf: Shelf,
    pub unlocked: bool,
    /// Contents colour of the jar.
    pub color: Color,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExperimentKind {
    Discovered,
    Known,
    /// "So close — swap the …", "Almost — …".
    Close,
    Nothing,
    /// A story-gated recipe ("Vera would know the rest of this one").
    Gated,
}

#[derive(Clone, Debug)]
pub struct ExperimentResult {
    pub kind: ExperimentKind,
    pub text: String,
    /// The recipe it concerns (index into `BookView::recipes`).
    pub recipe: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct BookView {
    pub recipes: Vec<BookRecipe>,
    pub pantry: Vec<Ingredient>,
    pub result: Option<ExperimentResult>,
    /// Opened from the order card: clicking a known recipe picks it for the guest.
    pub pick_mode: bool,
    /// Cost line next to Try ("10 Kč", "free tonight").
    pub try_cost: Option<String>,
    /// Lida's margin note (after failed tries).
    pub margin_note: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct BookLabels {
    pub title: String,
    /// Drinks, Food, Specials, Pantry.
    pub tabs: [String; 4],
    pub pick: String,
    pub ingredients: String,
    pub tastes_head: String,
    pub minigame: String,
    pub patterns: String,
    pub unknown: String,
    pub where_to_learn: String,
    pub practised: String,
    pub try_label: String,
    pub clear: String,
    /// "Pick 2–3 ingredients".
    pub slots_hint: String,
    pub bar_shelf: String,
    pub kitchen_shelf: String,
    pub locked_jar: String,
    pub empty_section: String,
    pub close: String,
    pub tastes: TasteNames,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BookAction {
    /// Pick mode: make this recipe for the guest.
    Pick(usize),
    /// Try the ingredients (indices into `BookView::pantry`, 2–3 of them).
    Try([Option<usize>; 3]),
    /// A recipe card was opened (clear its `new` dot).
    Opened(usize),
    Close,
}

#[derive(Clone, Debug, Default)]
pub struct BookState {
    pub tab: usize,
    /// Selected recipe (index into `BookView::recipes`).
    pub selected: Option<usize>,
    /// Chosen pantry slots (indices into `BookView::pantry`).
    pub slots: Vec<usize>,
    /// Pantry keyboard cursor.
    pub jar_cursor: usize,
}

pub const BOOK_SIZE: Vec2 = Vec2::new(780.0, 500.0);
const CARD: Vec2 = Vec2::new(84.0, 82.0);

/// The book, centred in `area` (usually the screen; it opens from the right).
pub fn recipe_book(
    ui: &mut Ui,
    key: impl std::hash::Hash,
    area: Rect,
    v: &BookView,
    l: &BookLabels,
    st: &mut BookState,
) -> Option<BookAction> {
    let t = ui.theme.clone();
    let id = ui.id(key);
    let size = vec2(
        BOOK_SIZE.x.min(area.w - 20.0),
        BOOK_SIZE.y.min(area.h - 20.0),
    );
    let k = ui.ease_value(id.with("open"), 1.0, 0.09);
    let base = area.centered(size);
    let r = base.translate(vec2((1.0 - ease_out(k)) * 40.0, 0.0));
    draw::push_alpha(k);
    ui.glass(r, &Glass::panel().solid().radius(14.0));
    let mut action = None;

    // spine and pages
    let mid = r.x + r.w / 2.0;
    for (x0, x1, a0, a1) in [(mid - 14.0, mid, 0.0, 0.32), (mid, mid + 14.0, 0.32, 0.0)] {
        let sr = Rect::new(x0, r.y + 1.0, x1 - x0, r.h - 2.0);
        fill_rrect(
            sr,
            0.0,
            Paint::horizontal(
                sr,
                Color::new(0.0, 0.0, 0.0, a0),
                Color::new(0.0, 0.0, 0.0, a1),
            ),
        );
    }
    draw::vline(
        mid,
        r.y + 1.0,
        r.bottom() - 1.0,
        Color::new(1.0, 1.0, 1.0, 0.07),
    );
    let left = Rect::new(r.x + 16.0, r.y + 14.0, mid - r.x - 30.0, r.h - 28.0);
    let right = Rect::new(mid + 16.0, r.y + 14.0, r.right() - mid - 30.0, r.h - 28.0);
    // ribbon bookmark
    let rb = Rect::new(right.right() - 34.0, r.y, 12.0, 34.0);
    draw::fill_convex(
        &[
            vec2(rb.x, rb.y),
            vec2(rb.right(), rb.y),
            vec2(rb.right(), rb.bottom()),
            vec2(rb.center().x, rb.bottom() - 5.0),
            vec2(rb.x, rb.bottom()),
        ],
        Paint::vertical(rb, t.amber_deep, t.amber),
    );

    // left page header + tabs
    let head = left.with_h(26.0);
    draw_icon(
        Icon::Book,
        vec2(head.x + 9.0, head.center_y()),
        16.0,
        t.amber,
    );
    let tw = ui.fonts.draw_in(
        &l.title,
        Rect::new(head.x + 24.0, head.y, head.w - 60.0, head.h),
        &t.title(),
        Align::Left,
    );
    ui.keycap(vec2(head.x + 32.0 + tw, head.center_y()), "K", false);
    let close = Rect::new(r.right() - 38.0, r.y + 12.0, 26.0, 26.0);
    if ui
        .icon_button(
            id.with("close"),
            close,
            Icon::Close,
            &IconButton::ghost().size(15.0).tooltip(&l.close),
        )
        .clicked
    {
        action = Some(BookAction::Close);
    }
    let counts: Vec<u32> = [
        BookSection::Drinks,
        BookSection::Food,
        BookSection::Specials,
    ]
    .iter()
    .map(|s| {
        v.recipes
            .iter()
            .filter(|r| r.section == *s && r.new)
            .count() as u32
    })
    .collect();
    let tabs: Vec<Tab> = (0..4)
        .map(|i| {
            let tab = Tab::new(&l.tabs[i]);
            if i < 3 && counts[i] > 0 {
                tab.badge(counts[i])
            } else {
                tab
            }
        })
        .collect();
    let tabs_r = Rect::new(left.x, head.bottom() + 6.0, left.w, 32.0);
    let mut tab = st.tab.min(3);
    if ui.tabs(id.with("tabs"), tabs_r, &tabs, &mut tab).changed {
        st.tab = tab;
    }
    let page = Rect::new(
        left.x,
        tabs_r.bottom() + 10.0,
        left.w,
        left.bottom() - tabs_r.bottom() - 10.0,
    );

    if st.tab < 3 {
        let section = [
            BookSection::Drinks,
            BookSection::Food,
            BookSection::Specials,
        ][st.tab];
        let list: Vec<usize> = (0..v.recipes.len())
            .filter(|i| v.recipes[*i].section == section)
            .collect();
        if st.selected.is_none_or(|s| !list.contains(&s)) {
            st.selected = list.first().copied();
        }
        if let Some(a) = recipe_grid(ui, id.with(("grid", st.tab)), page, v, &list, l, st) {
            action = Some(a);
        }
        match st.selected {
            Some(i) => {
                if let Some(a) = recipe_page(ui, id.with("detail"), right, v, i, l) {
                    action = Some(a);
                }
            }
            None => {
                ui.fonts.draw_wrapped(
                    &l.empty_section,
                    right.inset(20.0),
                    &t.small(),
                    Align::Center,
                    Some(3),
                );
            }
        }
    } else if let Some(a) = pantry(ui, id.with("pantry"), page, right, v, l, st) {
        action = Some(a);
    }
    draw::pop_alpha();

    // tabs by number
    for (i, kc) in [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4]
        .iter()
        .enumerate()
    {
        if ui.take_key(*kc) {
            st.tab = i;
        }
    }
    if ui.esc() {
        action = Some(BookAction::Close);
    }
    action
}

fn recipe_grid(
    ui: &mut Ui,
    id: crate::ui::Id,
    page: Rect,
    v: &BookView,
    list: &[usize],
    l: &BookLabels,
    st: &mut BookState,
) -> Option<BookAction> {
    let t = ui.theme.clone();
    let cols = ((page.w + 6.0) / (CARD.x + 6.0)).floor().max(1.0) as usize;
    let gap = ((page.w - cols as f32 * CARD.x) / (cols as f32 - 1.0).max(1.0)).max(4.0);
    let rows = list.len().div_ceil(cols);
    let content_h = rows as f32 * (CARD.y + gap) - gap;
    let sa = ui.begin_scroll(id.with("scroll"), page, content_h);
    let mut action = None;
    for (n, &ri) in list.iter().enumerate() {
        let rec = &v.recipes[ri];
        let (c, rr) = (n % cols, n / cols);
        let cr = Rect::new(
            page.x + c as f32 * (CARD.x + gap),
            sa.top() + rr as f32 * (CARD.y + gap),
            CARD.x,
            CARD.y,
        );
        if !sa.visible(cr.y, cr.h) {
            continue;
        }
        let resp = ui.interact(id.with(ri), cr);
        let sel = st.selected == Some(ri);
        let h = resp.hover_t;
        if sel {
            shadow_rrect(
                cr,
                10.0,
                draw::Shadow::new(0.0, 3.0, 12.0, -1.0, with_a(t.indigo, 0.4)),
            );
        }
        let (c0, c1) = if rec.known {
            (
                Color::new(1.0, 1.0, 1.0, 0.075 + 0.05 * h),
                Color::new(1.0, 1.0, 1.0, 0.03 + 0.03 * h),
            )
        } else {
            (
                with_a(t.indigo_600, 0.14 + 0.06 * h),
                with_a(t.violet, 0.06 + 0.04 * h),
            )
        };
        fill_rrect(cr, 10.0, Paint::vertical(cr, c0, c1));
        if rec.known {
            stroke_rrect(
                cr,
                10.0,
                1.0,
                if sel {
                    t.indigo_300
                } else {
                    mix(
                        Color::new(1.0, 1.0, 1.0, 0.08),
                        with_a(t.indigo_300, 0.6),
                        h,
                    )
                },
            );
        } else {
            dashed_rrect(
                cr,
                10.0,
                1.0,
                4.0,
                if sel {
                    t.indigo_300
                } else {
                    with_a(t.indigo_300, 0.3 + 0.3 * h)
                },
            );
        }
        let gc = vec2(cr.x + cr.w / 2.0, cr.y + 26.0);
        if rec.known {
            draw::glow_circle(gc, 26.0, with_a(rec.dish.tint, 0.12 + 0.1 * h));
            draw_dish(gc, 38.0, rec.dish, None, ui.time());
            ui.fonts.draw_wrapped(
                &rec.name,
                Rect::new(cr.x + 4.0, cr.y + 48.0, cr.w - 8.0, 30.0),
                &TextStyle::new(11.0, Weight::SemiBold, t.text).line_height(1.25),
                Align::Center,
                Some(2),
            );
            if rec.practised {
                draw_icon(
                    Icon::Star,
                    vec2(cr.right() - 11.0, cr.y + 11.0),
                    10.0,
                    t.amber,
                );
            }
            if rec.new {
                draw::glow_circle(vec2(cr.x + 11.0, cr.y + 11.0), 8.0, with_a(t.amber, 0.6));
                fill_circle(vec2(cr.x + 11.0, cr.y + 11.0), 3.0, t.amber);
            }
        } else {
            draw_dish(gc, 36.0, rec.dish, Some(with_a(t.indigo_400, 0.38)), 0.0);
            // tag icons + count dots
            let n_tags = rec.tags.len().min(4) as f32;
            let mut x = cr.x + (cr.w - n_tags * 14.0) / 2.0;
            for tag in rec.tags.iter().take(4) {
                taste_dot(vec2(x + 6.0, cr.y + 55.0), 12.0, *tag, TasteMark::Plain);
                x += 14.0;
            }
            let nd = rec.ingredient_count.max(1) as f32;
            let mut dx = cr.x + (cr.w - nd * 8.0) / 2.0 + 4.0;
            for k in 0..rec.ingredient_count {
                let revealed = (k as usize) < rec.ingredients.len();
                if revealed {
                    fill_circle(vec2(dx, cr.y + 70.0), 2.6, t.amber);
                } else {
                    stroke_circle(vec2(dx, cr.y + 70.0), 2.3, 1.0, with_a(t.indigo_300, 0.6));
                }
                dx += 8.0;
            }
        }
        let tip = if rec.known {
            rec.name.clone()
        } else {
            l.unknown.clone()
        };
        ui.tooltip(&resp, &tip);
        if resp.clicked {
            ui.sound(UiSound::Click);
            if v.pick_mode && rec.known && st.selected == Some(ri) {
                action = Some(BookAction::Pick(ri));
            } else {
                st.selected = Some(ri);
                if rec.new {
                    action = Some(BookAction::Opened(ri));
                }
            }
        }
    }
    ui.end_scroll(sa);

    // keyboard: arrows across the grid, Enter picks
    if let Some(cur) = st.selected.and_then(|s| list.iter().position(|x| *x == s)) {
        let n = list.len();
        let mut next = cur;
        if ui.key_repeat(KeyCode::Right) {
            next = (cur + 1).min(n - 1);
        }
        if ui.key_repeat(KeyCode::Left) {
            next = cur.saturating_sub(1);
        }
        if ui.key_repeat(KeyCode::Down) && cur + cols < n {
            next = cur + cols;
        }
        if ui.key_repeat(KeyCode::Up) && cur >= cols {
            next = cur - cols;
        }
        if next != cur {
            st.selected = Some(list[next]);
            ui.scroll_to(&sa, (next / cols) as f32 * (CARD.y + gap), CARD.y);
        }
        if v.pick_mode && ui.enter() && v.recipes[list[next]].known {
            action = Some(BookAction::Pick(list[next]));
        }
    }
    action
}

/// Right page: one recipe.
fn recipe_page(
    ui: &mut Ui,
    id: crate::ui::Id,
    page: Rect,
    v: &BookView,
    ri: usize,
    l: &BookLabels,
) -> Option<BookAction> {
    let t = ui.theme.clone();
    let rec = &v.recipes[ri];
    let mut action = None;
    let top = page.inset_ltrb(4.0, 30.0, 4.0, 0.0);
    // illustration
    let gc = vec2(top.x + 44.0, top.y + 40.0);
    if rec.known {
        draw::glow_circle(gc, 56.0, with_a(rec.dish.tint, 0.22));
        fill_circle(gc, 38.0, Color::new(1.0, 1.0, 1.0, 0.05));
        draw_dish(gc, 68.0, rec.dish, None, ui.time());
    } else {
        fill_circle(gc, 38.0, with_a(t.indigo_600, 0.14));
        dashed_rrect(
            Rect::new(gc.x - 38.0, gc.y - 38.0, 76.0, 76.0),
            38.0,
            1.0,
            4.0,
            with_a(t.indigo_300, 0.4),
        );
        draw_dish(gc, 62.0, rec.dish, Some(with_a(t.indigo_400, 0.45)), 0.0);
    }
    let tx = top.x + 98.0;
    let tw = top.right() - tx;
    let title = if rec.known {
        rec.name.as_str()
    } else {
        l.unknown.as_str()
    };
    let ts = t.title().line_height(1.25);
    let th = ui.fonts.draw_wrapped(
        title,
        Rect::new(tx, top.y + 2.0, tw, 46.0),
        &ts,
        Align::Left,
        Some(2),
    );
    let mut y = top.y + 2.0 + th + 4.0;
    // tastes as small labelled chips
    let mut x = tx;
    for tag in rec.tags.iter().take(5) {
        let name = l.tastes.get(*tag);
        let w = mini_chip_width(ui, name, true);
        if x + w > top.right() {
            break;
        }
        let c = tag.color();
        mini_chip(ui, vec2(x, y), name, c, with_a(c, 0.14), Some(tag.icon()));
        x += w + 4.0;
    }
    y = y.max(top.y + 44.0) + 26.0;
    y = y.max(gc.y + 48.0);

    let body = Rect::new(top.x, y, top.w, page.bottom() - y);
    let mut y = body.y;
    if rec.known {
        if let Some(lore) = &rec.lore {
            let ls = TextStyle::new(13.5, Weight::Medium, t.amber).line_height(1.38);
            let lh = handwritten_height(ui, lore, body.w - 14.0, &ls, 3);
            let bar = Rect::new(body.x, y + 2.0, 2.0, lh - 4.0);
            fill_rrect(bar, 1.0, with_a(t.amber, 0.5));
            draw_handwritten_block(
                ui,
                lore,
                Rect::new(body.x + 12.0, y, body.w - 14.0, lh),
                &ls,
                Align::Left,
                3,
            );
            y += lh + 10.0;
        }
        if let Some(d) = &rec.desc {
            y += ui.fonts.draw_wrapped(
                d,
                Rect::new(body.x, y, body.w, 40.0),
                &t.small(),
                Align::Left,
                Some(2),
            ) + 10.0;
        }
        y = section_chips(
            ui,
            body,
            y,
            &l.ingredients,
            &rec.ingredients,
            Icon::Jar,
            t.text,
        );
        if let Some(mg) = &rec.minigame {
            ui.section(Rect::new(body.x, y, body.w, 14.0), &l.minigame);
            y += 20.0;
            mini_chip(
                ui,
                vec2(body.x, y),
                mg,
                t.indigo_300,
                with_a(t.indigo, 0.18),
                Some(Icon::Sparkle),
            );
            let mut px = body.x + mini_chip_width(ui, mg, true) + 6.0;
            for p in &rec.patterns {
                let w = mini_chip_width(ui, p, true);
                if px + w > body.right() {
                    break;
                }
                mini_chip(
                    ui,
                    vec2(px, y),
                    p,
                    t.amber,
                    with_a(t.amber, 0.12),
                    Some(Icon::Heart),
                );
                px += w + 4.0;
            }
            y += 28.0;
        }
        for line in [&rec.base_line, &rec.stats_line].into_iter().flatten() {
            ui.fonts.draw_in(
                line,
                Rect::new(body.x, y, body.w, 18.0),
                &t.small(),
                Align::Left,
            );
            y += 18.0;
        }
        if rec.practised {
            let pr = Rect::new(body.x, y + 2.0, body.w, 18.0);
            draw_icon(Icon::Star, vec2(pr.x + 6.0, pr.center_y()), 11.0, t.amber);
            ui.fonts.draw_in(
                &l.practised,
                pr.inset_ltrb(16.0, 0.0, 0.0, 0.0),
                &t.small().color(t.amber),
                Align::Left,
            );
        }
        if v.pick_mode {
            let b = Button::new(&l.pick)
                .primary()
                .icon(Icon::Coffee)
                .shortcut("Enter");
            let bw = ui.button_width(&b);
            let br = Rect::new(page.right() - bw - 4.0, page.bottom() - 30.0, bw, 30.0);
            if ui.button(id.with("pick"), br, &b).clicked {
                action = Some(BookAction::Pick(ri));
            }
        }
    } else {
        // undiscovered: count dots with revealed names, where to learn it
        let names: Vec<String> = (0..rec.ingredient_count as usize)
            .map(|i| {
                rec.ingredients
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| "?".to_string())
            })
            .collect();
        y = section_chips(ui, body, y, &l.ingredients, &names, Icon::Jar, t.text_dim);
        if let Some(h) = &rec.source_hint {
            ui.section(Rect::new(body.x, y, body.w, 14.0), &l.where_to_learn);
            y += 22.0;
            let hs = TextStyle::new(13.5, Weight::Medium, t.amber).line_height(1.38);
            let hh = handwritten_height(ui, h, body.w - 40.0, &hs, 3) + 16.0;
            let card = Rect::new(body.x, y, body.w, hh);
            fill_rrect(card, 10.0, with_a(t.amber, 0.08));
            stroke_rrect(card, 10.0, 1.0, with_a(t.amber, 0.2));
            draw_icon(
                Icon::Note,
                vec2(card.x + 16.0, card.y + 18.0),
                14.0,
                t.amber,
            );
            draw_handwritten_block(
                ui,
                h,
                card.inset_ltrb(32.0, 8.0, 8.0, 8.0),
                &hs,
                Align::Left,
                3,
            );
        }
    }
    action
}

/// "SECTION" header + a wrapped flow of chips. Returns the new y.
fn section_chips(
    ui: &mut Ui,
    body: Rect,
    y: f32,
    title: &str,
    items: &[String],
    icon: Icon,
    fg: Color,
) -> f32 {
    if items.is_empty() {
        return y;
    }
    let mut y = y;
    ui.section(Rect::new(body.x, y, body.w, 14.0), title);
    y += 20.0;
    let mut x = body.x;
    for it in items {
        let w = mini_chip_width(ui, it, true);
        if x + w > body.right() && x > body.x {
            x = body.x;
            y += 22.0;
        }
        mini_chip(
            ui,
            vec2(x, y),
            it,
            fg,
            Color::new(1.0, 1.0, 1.0, 0.08),
            Some(icon),
        );
        x += w + 4.0;
    }
    y + 30.0
}

// ---------------------------------------------------------------------------
// Pantry & experiments
// ---------------------------------------------------------------------------

const JAR: Vec2 = Vec2::new(52.0, 56.0);

/// A glass jar with contents in `color`.
pub fn draw_jar(r: Rect, color: Color, unlocked: bool, chosen: bool, hover: f32) {
    let body = Rect::new(r.x + r.w * 0.16, r.y + r.h * 0.22, r.w * 0.68, r.h * 0.74);
    shadow_rrect(
        body,
        7.0,
        draw::Shadow::new(0.0, 3.0, 8.0, 0.0, Color::new(0.0, 0.0, 0.0, 0.35)),
    );
    if unlocked {
        let fill = Rect::new(
            body.x + 3.0,
            body.y + body.h * 0.3,
            body.w - 6.0,
            body.h * 0.7 - 3.0,
        );
        fill_rrect(body, 7.0, Color::new(0.85, 0.9, 1.0, 0.12 + 0.06 * hover));
        fill_rrect(
            fill,
            5.0,
            Paint::vertical(fill, mix(color, WHITE, 0.15), mix(color, BLACK, 0.2)),
        );
        // highlight streak
        fill_rrect(
            Rect::new(body.x + 4.0, body.y + 5.0, 3.0, body.h - 12.0),
            1.5,
            Color::new(1.0, 1.0, 1.0, 0.25),
        );
        stroke_rrect(body, 7.0, 1.0, Color::new(1.0, 1.0, 1.0, 0.3 + 0.2 * hover));
        // label
        let lab = Rect::new(
            body.x + body.w * 0.24,
            body.y + body.h * 0.58,
            body.w * 0.52,
            body.h * 0.16,
        );
        fill_rrect(lab, 1.5, with_a(Color::from_hex(0xF3EAD6), 0.85));
    } else {
        fill_rrect(body, 7.0, Color::new(1.0, 1.0, 1.0, 0.04));
        dashed_rrect(body, 7.0, 1.0, 3.0, Color::new(1.0, 1.0, 1.0, 0.16));
    }
    let lid = Rect::new(r.x + r.w * 0.22, r.y + r.h * 0.1, r.w * 0.56, r.h * 0.14);
    fill_rrect(
        lid,
        3.0,
        if unlocked {
            Paint::vertical(lid, Color::from_hex(0xD8A65E), Color::from_hex(0x8F6430))
        } else {
            Paint::Solid(Color::new(1.0, 1.0, 1.0, 0.08))
        },
    );
    if chosen {
        stroke_rrect(r, 10.0, 1.6, Color::from_hex(0xA5B4FC));
    }
}

fn pantry(
    ui: &mut Ui,
    id: crate::ui::Id,
    page: Rect,
    right: Rect,
    v: &BookView,
    l: &BookLabels,
    st: &mut BookState,
) -> Option<BookAction> {
    let t = ui.theme.clone();
    let mut action = None;
    st.slots.retain(|i| *i < v.pantry.len());
    // left page: two shelves of jars
    let cols = ((page.w + 4.0) / (JAR.x + 4.0)).floor().max(1.0) as usize;
    let gap = ((page.w - cols as f32 * JAR.x) / (cols as f32 - 1.0).max(1.0)).max(2.0);
    let shelves = [
        (Shelf::Bar, &l.bar_shelf),
        (Shelf::Kitchen, &l.kitchen_shelf),
    ];
    let item_h = JAR.y + 15.0;
    let mut content_h = 0.0;
    for (sh, _) in shelves {
        let n = v.pantry.iter().filter(|i| i.shelf == sh).count();
        content_h += 22.0 + n.div_ceil(cols) as f32 * (item_h + 4.0) + 8.0;
    }
    let sa = ui.begin_scroll(id.with("shelves"), page, content_h);
    let mut y = sa.top();
    let mut order: Vec<usize> = Vec::new();
    for (sh, title) in shelves {
        ui.section(Rect::new(page.x, y, page.w - 8.0, 14.0), title);
        y += 22.0;
        let items: Vec<usize> = (0..v.pantry.len())
            .filter(|i| v.pantry[*i].shelf == sh)
            .collect();
        for (n, &ii) in items.iter().enumerate() {
            order.push(ii);
            let ing = &v.pantry[ii];
            let cell = Rect::new(
                page.x + (n % cols) as f32 * (JAR.x + gap),
                y + (n / cols) as f32 * (item_h + 4.0),
                JAR.x,
                item_h,
            );
            if !sa.visible(cell.y, cell.h) {
                continue;
            }
            let resp = ui.interact(id.with(("jar", ii)), cell);
            let chosen = st.slots.contains(&ii);
            let focus = st.jar_cursor == order.len() - 1 && st.tab == 3;
            let h = if ing.unlocked { resp.hover_t } else { 0.0 };
            if h > 0.01 || (focus && ing.unlocked) {
                fill_rrect(cell, 10.0, Color::new(1.0, 1.0, 1.0, 0.05 * h.max(0.6)));
            }
            draw_jar(
                Rect::new(cell.x, cell.y - h * 2.0, JAR.x, JAR.y),
                ing.color,
                ing.unlocked,
                chosen,
                h,
            );
            if !ing.unlocked {
                ui.fonts.draw_in(
                    "?",
                    Rect::new(cell.x, cell.y + 22.0, JAR.x, 30.0),
                    &t.h1().color(with_a(t.indigo_300, 0.6)),
                    Align::Center,
                );
            }
            let name = if ing.unlocked { ing.name.as_str() } else { "" };
            ui.fonts.draw_in(
                name,
                Rect::new(cell.x - 2.0, cell.y + JAR.y, JAR.x + 4.0, 15.0),
                &TextStyle::new(
                    10.5,
                    Weight::Medium,
                    if chosen { t.indigo_300 } else { t.text_dim },
                ),
                Align::Center,
            );
            let tip = if ing.unlocked {
                format!("{} · {}", ing.name, ing.family)
            } else {
                l.locked_jar.clone()
            };
            ui.tooltip(&resp, &tip);
            if resp.clicked && ing.unlocked {
                ui.sound(UiSound::Toggle);
                st.jar_cursor = order.len() - 1;
                toggle_slot(st, ii);
            }
        }
        y += items.len().div_ceil(cols) as f32 * (item_h + 4.0) + 8.0;
    }
    ui.end_scroll(sa);

    // right page: the workbench
    let wb = right.inset_ltrb(4.0, 30.0, 4.0, 0.0);
    ui.fonts.draw_in(
        &l.slots_hint,
        Rect::new(wb.x, wb.y, wb.w, 18.0),
        &t.small(),
        Align::Left,
    );
    let slot_d = 74.0;
    let sy = wb.y + 28.0;
    let total = 3.0 * slot_d + 2.0 * 26.0;
    let sx0 = wb.x + (wb.w - total) / 2.0;
    for i in 0..3 {
        let c = vec2(
            sx0 + slot_d / 2.0 + i as f32 * (slot_d + 26.0),
            sy + slot_d / 2.0,
        );
        let sr = Rect::new(c.x - slot_d / 2.0, c.y - slot_d / 2.0, slot_d, slot_d);
        let resp = ui.interact(id.with(("slot", i)), sr);
        match st.slots.get(i).copied() {
            Some(ii) => {
                let ing = &v.pantry[ii];
                draw::glow_circle(c, 46.0, with_a(ing.color, 0.22));
                fill_circle(
                    c,
                    slot_d / 2.0,
                    Color::new(1.0, 1.0, 1.0, 0.06 + 0.04 * resp.hover_t),
                );
                stroke_circle(c, slot_d / 2.0 - 0.5, 1.0, with_a(t.indigo_300, 0.5));
                draw_jar(
                    Rect::new(c.x - 24.0, c.y - 32.0, 48.0, 56.0),
                    ing.color,
                    true,
                    false,
                    0.0,
                );
                ui.fonts.draw_in(
                    &ing.name,
                    Rect::new(sr.x - 10.0, sr.bottom() + 4.0, sr.w + 20.0, 16.0),
                    &t.small_medium(),
                    Align::Center,
                );
                if resp.hover_t > 0.01 {
                    draw_icon(
                        Icon::Close,
                        vec2(sr.right() - 8.0, sr.y + 8.0),
                        11.0,
                        with_a(t.text, resp.hover_t),
                    );
                }
                ui.tooltip(&resp, &l.clear);
                if resp.clicked {
                    st.slots.remove(i);
                }
            }
            None => {
                fill_circle(c, slot_d / 2.0, Color::new(0.03, 0.02, 0.09, 0.35));
                dashed_rrect(sr, slot_d / 2.0, 1.2, 5.0, with_a(t.indigo_300, 0.35));
                draw_icon(Icon::Plus, c, 16.0, with_a(t.indigo_300, 0.5));
            }
        }
        if i < 2 {
            draw_icon(
                Icon::Plus,
                vec2(c.x + slot_d / 2.0 + 13.0, c.y),
                12.0,
                t.text_disabled,
            );
        }
    }
    let by = sy + slot_d + 30.0;
    let can_try = st.slots.len() >= 2;
    let tb = Button::new(&l.try_label)
        .primary()
        .icon(Icon::Flask)
        .shortcut("T")
        .disabled(!can_try);
    let tbw = ui.button_width(&tb);
    let cost_w = v
        .try_cost
        .as_ref()
        .map_or(0.0, |c| mini_chip_width(ui, c, false) + 8.0);
    let clr = Button::new(&l.clear).ghost().disabled(st.slots.is_empty());
    // the Clear label gives way to an icon when the row is too wide (long languages)
    let full_cw = ui.button_width(&clr);
    let compact = tbw + cost_w + 8.0 + full_cw > wb.w;
    let cw = if compact { 30.0 } else { full_cw };
    let row_w = tbw + cost_w + 8.0 + cw;
    let bx = wb.x + ((wb.w - row_w) / 2.0).max(0.0);
    if ui
        .button(id.with("try"), Rect::new(bx, by, tbw, 30.0), &tb)
        .clicked
        && can_try
    {
        action = Some(BookAction::Try(slots_arr(st)));
    }
    if let Some(c) = &v.try_cost {
        mini_chip(
            ui,
            vec2(bx + tbw + 8.0, by + 6.0),
            c,
            t.amber,
            Color::new(0.03, 0.02, 0.09, 0.4),
            None,
        );
    }
    let cr = Rect::new(bx + tbw + cost_w + 8.0, by, cw, 30.0);
    let cleared = if compact {
        ui.icon_button(
            id.with("clear"),
            cr,
            Icon::Undo,
            &IconButton::ghost()
                .size(15.0)
                .disabled(st.slots.is_empty())
                .tooltip(&l.clear),
        )
        .clicked
    } else {
        ui.button(id.with("clear"), cr, &clr).clicked
    };
    if cleared {
        st.slots.clear();
    }
    let mut y = by + 46.0;
    if let Some(res) = &v.result {
        let (tone, icon) = match res.kind {
            ExperimentKind::Discovered => (t.amber, Icon::Sparkle),
            ExperimentKind::Known => (t.emerald_400, Icon::Check),
            ExperimentKind::Close => (t.indigo_300, Icon::Flask),
            ExperimentKind::Gated => (t.violet_400, Icon::Lock),
            ExperimentKind::Nothing => (t.text_dim, Icon::Cup),
        };
        let dish = res.recipe.and_then(|i| v.recipes.get(i)).map(|r| r.dish);
        let text_x = if dish.is_some() { 66.0 } else { 38.0 };
        let th = ui
            .fonts
            .wrapped_height(&res.text, wb.w - text_x - 12.0, &t.body(), Some(3));
        let card = Rect::new(
            wb.x,
            y,
            wb.w,
            (th + 20.0).max(if dish.is_some() { 64.0 } else { 40.0 }),
        );
        let appear = ui.ease_value(id.with(("result", res.text.len(), res.recipe)), 1.0, 0.1);
        if res.kind == ExperimentKind::Discovered {
            shadow_rrect(
                card,
                12.0,
                draw::Shadow::new(0.0, 0.0, 22.0, 0.0, with_a(t.amber, 0.25 * appear)),
            );
        }
        fill_rrect(
            card,
            12.0,
            Paint::horizontal(card, with_a(tone, 0.16), with_a(tone, 0.05)),
        );
        stroke_rrect(card, 12.0, 1.0, with_a(tone, 0.35));
        if let Some(d) = dish {
            let gc = vec2(card.x + 34.0, card.center_y());
            draw::glow_circle(gc, 30.0, with_a(d.tint, 0.3));
            draw_dish(gc, 44.0 * (0.8 + 0.2 * appear), d, None, ui.time());
        } else {
            draw_icon(icon, vec2(card.x + 20.0, card.center_y()), 16.0, tone);
        }
        ui.fonts.draw_wrapped(
            &res.text,
            Rect::new(card.x + text_x, card.y + 10.0, card.w - text_x - 12.0, th),
            &t.body(),
            Align::Left,
            Some(3),
        );
        y = card.bottom() + 12.0;
    }
    if let Some(note) = &v.margin_note {
        let ns = TextStyle::new(13.5, Weight::Medium, t.amber).line_height(1.38);
        let nh = handwritten_height(ui, note, wb.w - 30.0, &ns, 3);
        if y + nh <= wb.bottom() {
            let ny = wb.bottom() - nh;
            draw_icon(
                Icon::Note,
                vec2(wb.x + 8.0, ny + 9.0),
                13.0,
                with_a(t.amber, 0.8),
            );
            draw_handwritten_block(
                ui,
                note,
                Rect::new(wb.x + 24.0, ny, wb.w - 26.0, nh),
                &ns,
                Align::Left,
                3,
            );
        }
    }

    // keyboard on the pantry
    let n = order.len();
    if n > 0 {
        let mut cur = st.jar_cursor.min(n - 1);
        if ui.key_repeat(KeyCode::Right) {
            cur = (cur + 1).min(n - 1);
        }
        if ui.key_repeat(KeyCode::Left) {
            cur = cur.saturating_sub(1);
        }
        if ui.key_repeat(KeyCode::Down) {
            cur = (cur + cols).min(n - 1);
        }
        if ui.key_repeat(KeyCode::Up) {
            cur = cur.saturating_sub(cols);
        }
        st.jar_cursor = cur;
        if ui.enter() && v.pantry[order[cur]].unlocked {
            toggle_slot(st, order[cur]);
        }
    }
    if ui.key_repeat(KeyCode::Backspace) {
        st.slots.pop();
    }
    if ui.take_key(KeyCode::T) && st.slots.len() >= 2 {
        action = Some(BookAction::Try(slots_arr(st)));
    }
    action
}

fn toggle_slot(st: &mut BookState, ii: usize) {
    if let Some(p) = st.slots.iter().position(|x| *x == ii) {
        st.slots.remove(p);
    } else if st.slots.len() < 3 {
        st.slots.push(ii);
    }
}

fn slots_arr(st: &BookState) -> [Option<usize>; 3] {
    [
        st.slots.first().copied(),
        st.slots.get(1).copied(),
        st.slots.get(2).copied(),
    ]
}
