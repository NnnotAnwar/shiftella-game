//! Checklist items in their three states wherever they show (the owner's report: "I click the
//! tick, it flashes and Karolina goes" — a click must never look like *done*):
//!
//! * **open** — an empty box; a click queues it for Karolina;
//! * **taken** — a small clock (Karolina has it queued / is on her way) or a person (a helper
//!   is on it), with who it is; a click on Karolina's cancels it;
//! * **done** — a filled tick, struck through, who and when.
//!
//! The same states draw in the Shiftella app's list ([`row`]), on the box over the item's
//! furniture (`hands_view::draw_check_box`, with [`world_tip`] for the hover text) and on
//! Karolina's card (her queue reads "Checklist: …" with the same clock).
//!
//! By hand (0.2.6, `sim::rituals`): an item Karolina did with its gesture carries a small ✋
//! (amber; brighter for a perfect one), one of grandma's habits a feather; the row's tooltip
//! says what by hand adds, a habit's perk or how many nights are still to go. Under the lists,
//! grandma's habits learned so far ([`habits`]).

use macroquad::prelude::*;

use super::hands::Intent;
use super::hands_view;
use super::world_view::{self as wv, sim};
use super::{GameScreen, hud::world_to_ui};
use crate::content::Content;
use crate::settings::Locale;
use crate::ui::draw::{fill_rrect, hline, with_a};
use crate::ui::{Align, Glass, Icon, RectExt, Ui, draw_icon};
use sim::ChecklistMark;

/// The player stops seeing the "how it works" line after this many checklist clicks.
pub const TIP_UNTIL: u32 = 4;

/// Who an item belongs to before anyone has it ("Karolina" / "Karolina · Barista").
fn owner_text(loc: &Locale, content: &Content, item: usize) -> String {
    let Some(it) = content.app.checklists.get(item) else {
        return String::new();
    };
    match it.owner() {
        crate::content::handson::ChecklistOwner::Karolina => loc.tr_owned("cast.player.name"),
        crate::content::handson::ChecklistOwner::Either(d) => {
            format!("{} · {}", loc.tr("cast.player.name"), loc.tr(&d.name_key()))
        }
    }
}

/// A person's short name ("Karolina", a helper's name).
fn who(g: &GameScreen, loc: &Locale, id: Option<u32>) -> String {
    match id.and_then(|id| g.world.staff_by_id(id)) {
        Some(s) if s.owner || s.name.is_empty() => loc.tr_owned("cast.player.name"),
        Some(s) => s.name.clone(),
        None => loc.tr_owned("hux.check.someone"),
    }
}

/// Why an open item can't be done yet (greyed with this reason; `None` = it can, or it is
/// already someone's / done).
pub fn lock(g: &GameScreen, content: &Content, item: usize) -> Option<sim::ChecklistLock> {
    let it = content.app.checklists.get(item)?;
    if g.world.checklist_mark(&it.id) != ChecklistMark::Open {
        return None;
    }
    g.world.checklist_lock(content, &it.id)
}

/// (the line at the right of the row, the hover text) for an item in its state.
pub fn texts(g: &GameScreen, loc: &Locale, content: &Content, item: usize) -> (String, String) {
    let Some(it) = content.app.checklists.get(item) else {
        return (String::new(), String::new());
    };
    let night = g.world.phase() == sim::Phase::Night;
    if let Some(l) = lock(g, content, item) {
        return (loc.tr_owned(l.key()), loc.tr_owned(l.key()));
    }
    match g.world.checklist_mark(&it.id) {
        ChecklistMark::Open => (
            owner_text(loc, content, item),
            loc.tr_owned(if night {
                "hux.check.tip.open"
            } else {
                "ui.app.night_only"
            }),
        ),
        ChecklistMark::Taken {
            karolina: true,
            doing,
            ..
        } => (
            loc.trf(
                if doing {
                    "hux.check.doing"
                } else {
                    "hux.check.queued"
                },
                &[("name", loc.tr("cast.player.name"))],
            ),
            loc.tr_owned("hux.check.tip.queued"),
        ),
        ChecklistMark::Taken { by, .. } => {
            let name = who(g, loc, Some(by));
            (
                loc.trf("hux.check.helper", &[("name", &name)]),
                loc.trf("hux.check.tip.helper", &[("name", &name)]),
            )
        }
        ChecklistMark::Done { by, minute } => {
            let name = who(g, loc, by);
            match minute {
                Some(m) => {
                    let time = wv::clock_of_minute(m);
                    (
                        format!("{name} · {time}"),
                        loc.trf("hux.check.tip.done", &[("name", &name), ("time", &time)]),
                    )
                }
                None => (
                    name.clone(),
                    loc.trf("hux.check.tip.done_by", &[("name", &name)]),
                ),
            }
        }
    }
}

/// The by-hand lines of an item's tooltip: what by hand adds (done or on offer), a perfect
/// gesture, the habit's perk or its progress. Empty when there's nothing to say.
pub fn hand_lines(g: &GameScreen, loc: &Locale, content: &Content, item: usize) -> Vec<String> {
    let Some(it) = content.app.checklists.get(item) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let what_key = format!("ritual.hand.{}", it.id);
    let has_what = !it.hand_done.is_empty() || !it.perfect_done.is_empty();
    let what = loc.tr_owned(&what_key);
    match g.world.ritual_by_hand(&it.id) {
        Some(perfect) => {
            if has_what {
                out.push(loc.trf("rituals.hand.done", &[("what", &what)]));
            }
            if perfect {
                out.push(loc.tr_owned("rituals.hand.perfect"));
            }
        }
        None if has_what
            && g.world.phase() == sim::Phase::Night
            && matches!(
                g.world.checklist_mark(&it.id),
                ChecklistMark::Open | ChecklistMark::Taken { karolina: true, .. }
            ) =>
        {
            out.push(loc.trf("rituals.hand.offer", &[("what", &what)]));
        }
        None => {}
    }
    if !it.habit.is_empty() {
        if g.world.habit_learned(&it.id) {
            let perk = loc.tr_owned(&format!("habit.{}.perk", it.id));
            out.push(loc.trf("rituals.habit.mark", &[("perk", &perk)]));
        } else {
            let n = g.world.habits().nights.get(&it.id).copied().unwrap_or(0);
            if n > 0 {
                let max = content.handson.tuning.rituals.habit_nights;
                out.push(loc.trf(
                    "rituals.habit.progress",
                    &[("n", &n.to_string()), ("max", &max.to_string())],
                ));
            }
        }
    }
    out
}

/// Grandma's habits under the lists: the block's height.
pub fn habits_height(g: &GameScreen) -> f32 {
    30.0 + g.world.habits().learned.len().max(1) as f32 * 22.0 + 8.0
}

/// Grandma's habits under the lists (from `y`): the habits learned with their perk, or how to
/// learn one. `visible` = the scroll area's row check.
pub fn habits(
    g: &GameScreen,
    ui: &mut Ui,
    loc: &Locale,
    content: &Content,
    r: Rect,
    y: f32,
    visible: &dyn Fn(f32, f32) -> bool,
) {
    let t = ui.theme.clone();
    let learned = g.world.habits().learned.clone();
    let total = content
        .app
        .checklists
        .iter()
        .filter(|c| !c.habit.is_empty())
        .count();
    let title = loc.trf(
        "rituals.habits.title",
        &[
            ("n", &learned.len().to_string()),
            ("total", &total.to_string()),
        ],
    );
    if visible(y, 26.0) {
        ui.section(Rect::new(r.x, y, r.w - 12.0, 26.0), &title);
    }
    let mut y = y + 30.0;
    let style = t.small().color(t.text);
    if learned.is_empty() {
        if visible(y, 20.0) {
            let max = content.handson.tuning.rituals.habit_nights.to_string();
            draw_icon(Icon::Feather, vec2(r.x + 16.0, y + 10.0), 12.0, t.text_dim);
            ui.fonts.draw_fit(
                &loc.trf("rituals.habits.none", &[("max", &max)]),
                Rect::new(r.x + 34.0, y, r.w - 50.0, 20.0),
                &t.small().color(t.text_dim),
                Align::Left,
                0.8,
            );
        }
        return;
    }
    for id in learned {
        if visible(y, 20.0) {
            draw_icon(
                Icon::Feather,
                vec2(r.x + 16.0, y + 10.0),
                12.0,
                t.violet_400,
            );
            let label = loc.tr_owned(&format!("app.checklist.{id}"));
            let perk = loc.tr_owned(&format!("habit.{id}.perk"));
            ui.fonts.draw_fit(
                &format!("{label} — {perk}"),
                Rect::new(r.x + 34.0, y, r.w - 50.0, 20.0),
                &style,
                Align::Left,
                0.8,
            );
        }
        y += 22.0;
    }
}

/// One row of the app's checklist (28 pt): the box in its state, the label (struck through
/// when done), who / when on the right. Returns whether it was clicked.
pub fn row(
    g: &mut GameScreen,
    ui: &mut Ui,
    loc: &Locale,
    content: &Content,
    r: Rect,
    item: usize,
) -> bool {
    let t = ui.theme.clone();
    let Some(it) = content.app.checklists.get(item) else {
        return false;
    };
    let mark = g.world.checklist_mark(&it.id);
    let locked = lock(g, content, item).is_some();
    let (meta, tip) = texts(g, loc, content, item);
    let id = ui.id(("chk_row", it.id.as_str()));
    let resp = ui.interact(id, r);
    // (a locked row isn't a button: no hover wash, the reason on the right and in the tip)
    if resp.hover_t > 0.01 && !locked {
        fill_rrect(r, 6.0, Color::new(1.0, 1.0, 1.0, 0.04 * resp.hover_t));
    }
    let done = matches!(mark, ChecklistMark::Done { .. });
    let bc = vec2(r.x + 8.0 + 8.0, r.center_y());
    if locked {
        hands_view::draw_locked_box(bc, 16.0, 1.0);
    } else {
        hands_view::draw_check_box(bc, 16.0, mark, resp.hovered, 1.0);
    }
    let ms = t.small().color(match mark {
        _ if locked => with_a(t.text_dim, 0.8),
        ChecklistMark::Taken { karolina: true, .. } => t.indigo_300,
        _ => t.text_dim,
    });
    let mw = ui.fonts.measure(&meta, &ms).min(r.w * 0.45);
    ui.fonts.draw_in(
        &meta,
        Rect::new(r.right() - 8.0 - mw, r.y, mw, r.h),
        &ms,
        Align::Left,
    );
    let label = loc.tr_owned(&format!("app.checklist.{}", it.id));
    // by hand ✋ / grandma's habit (a feather) after the label
    let hand = g.world.ritual_by_hand(&it.id);
    let habit = g.world.habit_learned(&it.id);
    let marks_w = 16.0 * (hand.is_some() as u8 + habit as u8) as f32;
    let lr = Rect::new(
        r.x + 34.0,
        r.y,
        (r.w - 34.0 - mw - 18.0 - marks_w).max(0.0),
        r.h,
    );
    let ls = t
        .body()
        .color(if done || locked { t.text_dim } else { t.text });
    let lw = ui.fonts.draw_in(&label, lr, &ls, Align::Left);
    if done {
        hline(lr.x - 1.0, lr.x + lw + 1.0, lr.center_y() - 0.5, t.text_dim);
    }
    let mut mx = lr.x + lw + 12.0;
    if let Some(perfect) = hand {
        let c = if perfect {
            t.amber
        } else {
            with_a(t.amber, 0.75)
        };
        draw_icon(Icon::Hand, vec2(mx, r.center_y()), 13.0, c);
        if perfect {
            draw_icon(
                Icon::Sparkle,
                vec2(mx + 6.0, r.center_y() - 6.0),
                7.0,
                t.amber,
            );
        }
        mx += 16.0;
    }
    if habit {
        draw_icon(Icon::Feather, vec2(mx, r.center_y()), 12.0, t.violet_400);
    }
    let extra = hand_lines(g, loc, content, item);
    if extra.is_empty() {
        ui.tooltip(&resp, &tip);
    } else {
        ui.tooltip(&resp, &format!("{tip}\n{}", extra.join("\n")));
    }
    resp.clicked
}

/// The app's checklist: a click on a row (night only; by day a gentle toast).
pub fn row_clicked(g: &mut GameScreen, loc: &Locale, content: &Content, item: usize) {
    if g.world.phase() != sim::Phase::Night {
        g.toast(
            crate::ui::ToastKind::Info,
            Icon::ClipboardCheck,
            loc.tr_owned("ui.app.night_only"),
            None,
        );
        return;
    }
    let mark = content
        .app
        .checklists
        .get(item)
        .map(|it| g.world.checklist_mark(&it.id));
    if let Some(ChecklistMark::Taken {
        karolina: false, ..
    }) = mark
    {
        return;
    }
    // (a locked item says why; `intent` does the same for the box over its piece)
    if g.intent(content, Intent::Checklist(item)) {
        g.extra.checklist_tip = g.extra.checklist_tip.saturating_add(1);
    }
}

/// The one-line "how it works" hint over the list, until it's familiar (or dismissed).
/// Returns the height it took (0 when hidden).
pub fn hint(g: &mut GameScreen, ui: &mut Ui, loc: &Locale, r: Rect) -> f32 {
    if g.extra.checklist_tip >= TIP_UNTIL || g.world.phase() != sim::Phase::Night {
        return 0.0;
    }
    let t = ui.theme.clone();
    let h = 24.0;
    let hr = Rect::new(r.x, r.y, r.w - 12.0, h);
    fill_rrect(hr, 6.0, Color::new(0.39, 0.4, 0.95, 0.12));
    draw_icon(
        Icon::Info,
        vec2(hr.x + 12.0, hr.center_y()),
        13.0,
        t.indigo_300,
    );
    ui.fonts.draw_in(
        loc.tr("hux.check.hint"),
        Rect::new(hr.x + 24.0, hr.y, hr.w - 50.0, h),
        &t.small().color(t.text),
        Align::Left,
    );
    let xr = Rect::new(hr.right() - 22.0, hr.y + 2.0, 20.0, 20.0);
    let resp = ui.interact(ui.id("chk_hint_x"), xr);
    draw_icon(
        Icon::Close,
        xr.center(),
        10.0,
        if resp.hovered { t.text } else { t.text_dim },
    );
    ui.tooltip(&resp, loc.tr("hux.check.hint_close"));
    if resp.clicked {
        g.extra.checklist_tip = 99;
    }
    h + 6.0
}

/// The hover text of a checklist box over its furniture (UI pass).
pub fn world_tip(g: &mut GameScreen, ui: &mut Ui, loc: &Locale, content: &Content) {
    let Some(item) = g.hands.ritual_hover else {
        return;
    };
    let Some(m) = hands_view::ritual_marks(g, content)
        .into_iter()
        .find(|m| m.item == item)
    else {
        return;
    };
    let t = ui.theme.clone();
    let label = content
        .app
        .checklists
        .get(item)
        .map(|it| loc.tr_owned(&format!("app.checklist.{}", it.id)))
        .unwrap_or_default();
    let (_, tip) = texts(g, loc, content, item);
    let top = hands_view::ritual_rect(&m, g.time);
    let anchor = world_to_ui(g, ui, vec2(top.center().x, top.y));
    let st = t.small().weight(crate::ui::Weight::SemiBold);
    let sub = t.small().color(t.text_dim);
    let w = ui
        .fonts
        .measure(&label, &st)
        .max(ui.fonts.measure(&tip, &sub))
        + 20.0;
    let h = 40.0;
    let sw = ui.screen().w;
    let r = Rect::new(
        (anchor.x - w / 2.0).clamp(8.0, (sw - w - 8.0).max(8.0)),
        (anchor.y - h - 8.0).max(8.0),
        w,
        h,
    );
    ui.glass(r, &Glass::tooltip().radius(8.0));
    ui.fonts.draw_in(
        &label,
        Rect::new(r.x + 10.0, r.y + 3.0, w - 20.0, 18.0),
        &st,
        Align::Left,
    );
    ui.fonts.draw_in(
        &tip,
        Rect::new(r.x + 10.0, r.y + 19.0, w - 20.0, 18.0),
        &sub,
        Align::Left,
    );
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;

    fn content() -> Rc<Content> {
        let root = env!("CARGO_MANIFEST_DIR");
        Rc::new(Content::load(&format!("{root}/assets/data")).expect("content"))
    }

    #[test]
    fn every_checklist_item_with_a_place_has_a_marker_there() {
        let c = content();
        let mut g = GameScreen::new_story(&c, "cozy", true);
        g.start_night(&c);
        g.world.skip_prep(&c);
        let ids = |g: &GameScreen| -> Vec<String> {
            hands_view::ritual_marks(g, &c)
                .iter()
                .map(|m| c.app.checklists[m.item].id.clone())
                .collect()
        };
        // the opening rituals right after the doors open — the window lamp included
        let now = ids(&g);
        for it in c
            .app
            .checklists
            .iter()
            .filter(|i| i.list == crate::content::ChecklistKind::Opening)
        {
            assert!(it.anchor.is_some(), "{} has a place", it.id);
            assert!(
                now.contains(&it.id) || g.world.checklist_mark(&it.id) != ChecklistMark::Open,
                "opening item {} has no marker",
                it.id
            );
        }
        assert!(
            now.iter().any(|i| i == "light_window"),
            "the light in the window"
        );
        // the closing ones at the end of the night
        g.world.tick = 470 * sim::TICKS_PER_MINUTE;
        let later = ids(&g);
        for it in c
            .app
            .checklists
            .iter()
            .filter(|i| i.list == crate::content::ChecklistKind::Closing)
        {
            assert!(
                later.contains(&it.id),
                "closing item {} has no marker",
                it.id
            );
        }
    }

    #[test]
    fn a_click_queues_an_item_a_second_click_cancels_it_and_it_never_looks_done() {
        let c = content();
        let mut g = GameScreen::new_story(&c, "cozy", true);
        g.start_night(&c);
        g.world.skip_prep(&c);
        let i = c
            .app
            .checklists
            .iter()
            .position(|x| x.id == "count_float")
            .unwrap();
        assert_eq!(g.world.checklist_mark("count_float"), ChecklistMark::Open);
        assert!(g.intent(&c, Intent::Checklist(i)));
        let m = g.world.checklist_mark("count_float");
        assert!(
            matches!(m, ChecklistMark::Taken { karolina: true, .. }),
            "a click puts it in her hands, not done: {m:?}"
        );
        // the marker over the till shows it (the clock), still clickable
        assert!(
            hands_view::ritual_marks(&g, &c)
                .iter()
                .any(|r| r.item == i && r.mark == m)
        );
        // the second click cancels
        assert!(g.intent(&c, Intent::Checklist(i)));
        assert_eq!(g.world.checklist_mark("count_float"), ChecklistMark::Open);
        assert!(g.world.player().current.is_none() && g.world.player().queue.is_empty());
    }

    /// fix/closing: an item that can't be done yet is never an empty box that does nothing —
    /// greyed with its reason in the row, a lock over its piece, a click says why.
    #[test]
    fn a_locked_item_says_why_and_never_looks_clickable() {
        let c = content();
        let i18n = {
            let root = env!("CARGO_MANIFEST_DIR");
            crate::settings::load_i18n(&format!("{root}/assets/i18n")).expect("i18n")
        };
        let loc = Locale::new(&i18n, crate::i18n::Lang::En);
        let mut g = GameScreen::new_story(&c, "cozy", true);
        g.start_night(&c);
        g.world.skip_prep(&c);
        let at = |id: &str| c.app.checklists.iter().position(|x| x.id == id).unwrap();
        // 22:00: the closing list is for later
        let dishes = at("wash_dishes");
        assert_eq!(lock(&g, &c, dishes), Some(sim::ChecklistLock::FromFive));
        let (meta, tip) = texts(&g, &loc, &c, dishes);
        assert_eq!(meta, "From 5:00");
        assert_eq!(tip, meta);
        let before = g.notices.len();
        assert!(!g.intent(&c, Intent::Checklist(dishes)));
        assert_eq!(g.notices.len(), before + 1, "a click says why");
        assert_eq!(g.world.checklist_mark("wash_dishes"), ChecklistMark::Open);
        assert!(g.world.player().current.is_none() && g.world.player().queue.is_empty());
        // the opening list is doable: no lock
        assert_eq!(lock(&g, &c, at("count_float")), None);
        // near closing: the till's marker over the register is a lock (the last step, after
        // the doors close), the dishes' one an open box
        g.world.tick = 470 * sim::TICKS_PER_MINUTE;
        let marks = hands_view::ritual_marks(&g, &c);
        let till = marks
            .iter()
            .find(|m| m.item == at(sim::TILL_ITEM))
            .expect("the till's marker");
        assert!(till.locked);
        assert_eq!(
            texts(&g, &loc, &c, at(sim::TILL_ITEM)).0,
            "The last step, after 6:00"
        );
        let d = marks.iter().find(|m| m.item == dishes).expect("the sink's");
        assert!(!d.locked);
        // the opening markers are gone once the doors close (missed tonight, not dead boxes)
        assert!(marks.iter().all(|m| c.app.checklists[m.item].list
            == crate::content::ChecklistKind::Closing
            || !g.world.in_closing()));
    }
}
