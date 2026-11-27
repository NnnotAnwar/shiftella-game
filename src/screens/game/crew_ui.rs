//! The crew panel (C) and the character card (design/v0.2.md §D).
//!
//! A strip of faces (Karolina first, then the helpers, then "Hire") over the selected person's
//! card. The card answers "who is this, what are they doing, how are they, what can I do":
//!
//! * **Karolina** (the player): what she is doing now and her queue (× drops an act), what
//!   waits only for her, tonight's hands-on numbers, the duties no helper holds. Energy only
//!   on Classic (on Cozy she never tires).
//! * **A helper**: role and tonight's shift, the current and the next task in words, the duties
//!   as one-click toggles (the same command path as the Duties board, H), mood (and energy on
//!   Classic or when tired) with the reason, traits with explanations, their rozpis wishes and
//!   who they get along with. Quick actions: Follow, Break, Send home, Rozpis (by day: Let go).
//! * **Hire**: the candidates, each with what they are good at in one line.
//!
//! The panel is as tall as its card (no empty space); a short screen scrolls the card.
//! Keyboard: arrows step through the crew, Esc closes (`GameScreen::hotkeys`).

use macroquad::prelude::*;

use super::duties::{self as board, to_ui};
use super::hands::Intent;
use super::world_view::{self as wv, sim};
use super::{Ctx, GameScreen, Panel};
use crate::content::handson::Duty;
use crate::content::{Content, TraitGroup};
use crate::i18n::format_money;
use crate::settings::Locale;
use crate::ui::draw::{fill_circle, fill_rrect, stroke_rrect, with_a};
use crate::ui::handson::common::capacity_dots;
use crate::ui::handson::rozpis::{PrefChip, pref_chip, pref_chip_width};
use crate::ui::{
    Align, Button, Glass, Icon, Need, RectExt, TextStyle, ToastKind, Tone, Ui, UiSound, Weight,
    draw_icon,
};
use sim::{Activity, AgentRef, Command, Feeling, Phase, ShiftTonight, Staff, StaffStatus, World};

/// The strip's "Hire" cell (as `UiState::crew_selected`).
pub const HIRE: u32 = u32::MAX;
const PANEL_W: f32 = 340.0;
const PAD: f32 = 12.0;
const GAP: f32 = 5.0;
const LINE: f32 = 16.0;
const STRIP_H: f32 = 46.0;
const HEADER_H: f32 = 36.0;

/// Strip / arrow-key order: Karolina, then the helpers on the floor, expected, leaving, off,
/// sick — each group by name.
pub fn crew_order(w: &World) -> Vec<u32> {
    let rank = |s: &Staff| {
        if s.owner {
            return 0;
        }
        match s.status {
            StaffStatus::OnDuty => 1,
            StaffStatus::Expected { .. } => 2,
            StaffStatus::Leaving => 3,
            StaffStatus::OffDuty => 4,
            StaffStatus::Sick | StaffStatus::WalkedOut => 5,
        }
    };
    let mut v: Vec<&Staff> = w.staff().iter().collect();
    v.sort_by(|a, b| rank(a).cmp(&rank(b)).then(a.name.cmp(&b.name)));
    v.iter().map(|s| s.id).collect()
}

/// An arrow key is held while the crew panel is open: the camera leaves them to the strip.
pub fn arrows_held() -> bool {
    [KeyCode::Left, KeyCode::Right, KeyCode::Up, KeyCode::Down]
        .iter()
        .any(|k| is_key_down(*k))
}

/// The next / previous cell of the strip (wraps).
pub fn step(cells: &[u32], sel: u32, by: i32) -> u32 {
    if cells.is_empty() {
        return sel;
    }
    let i = cells.iter().position(|c| *c == sel).unwrap_or(0) as i32;
    let n = cells.len() as i32;
    cells[(((i + by) % n + n) % n) as usize]
}

// ---------------------------------------------------------------------------------------------
// Words
// ---------------------------------------------------------------------------------------------

fn who_text(loc: &Locale, who: &sim::TaskFor) -> String {
    match who {
        sim::TaskFor::Regular(r) => loc.tr_owned(&format!("regular.{r}.name")),
        sim::TaskFor::Guest(a) => wv::guest_one(loc, a),
    }
}

fn menu_text(loc: &Locale, content: &Content, menu: &str, recipe: Option<&str>) -> String {
    // the café's own drink: its name (filled in by the game's i18n layer)
    if recipe == Some(crate::content::handson::SIGNATURE_RECIPE) {
        return loc.tr_owned("recipe.r_signature.name");
    }
    if let Some(r) = recipe.and_then(|r| content.handson.recipe(r)) {
        return loc.tr_owned(&r.name_key());
    }
    loc.tr_owned(&format!("menu.{menu}.name"))
}

/// "Making: Cappuccino · Mrs Vera", "Washing the dishes (8)", "On a break until 01:20"…
pub fn activity_text(loc: &Locale, content: &Content, a: &Activity) -> String {
    let key = format!("crew.do.{}", a.id());
    let time = |m: u32| wv::clock_of_minute(m);
    let with_who = |base: String, who: &Option<sim::TaskFor>| match who {
        Some(w) => format!("{base} · {}", who_text(loc, w)),
        None => base,
    };
    match a {
        Activity::Arrives { minute }
        | Activity::BreakAt { minute }
        | Activity::HomeAt { minute } => loc.trf(&key, &[("time", &time(*minute))]),
        Activity::Break { until, .. } => loc.trf(&key, &[("time", &time(*until))]),
        Activity::TakeOrder { who, .. } => with_who(loc.tr_owned(&key), who),
        Activity::Make { menu, recipe, who } => with_who(
            loc.trf(
                &key,
                &[("item", &menu_text(loc, content, menu, recipe.as_deref()))],
            ),
            who,
        ),
        Activity::Serve { menu, count, who } => {
            let mut item = menu_text(loc, content, menu, None);
            if *count > 1 {
                item = format!("{item} +{}", count - 1);
            }
            with_who(loc.trf(&key, &[("item", &item)]), who)
        }
        Activity::Wash { dishes } | Activity::ClearTable { dishes } if *dishes > 0 => {
            format!("{} ({dishes})", loc.tr_owned(&key))
        }
        Activity::Checklist { id } => loc.trf(
            &key,
            &[("item", &loc.tr_owned(&format!("app.checklist.{id}")))],
        ),
        Activity::Equipment { action } => loc.tr_owned(&format!("equip.act.{}", action.id())),
        Activity::Record { id: Some(r) } => {
            let title = content
                .handson
                .record(r)
                .map(|d| loc.tr_owned(&d.title_key()))
                .unwrap_or_default();
            loc.trf("crew.do.record_named", &[("item", &title)])
        }
        Activity::Moment { id } => {
            let title = content
                .handson
                .moment(id)
                .map(|d| loc.tr_owned(&d.title_key()))
                .unwrap_or_default();
            loc.trf(&key, &[("item", &title)])
        }
        _ => loc.tr_owned(&key),
    }
}

fn first_name(loc: &Locale, w: &World, id: u32) -> String {
    match w.staff_by_id(id) {
        Some(s) if s.owner => loc.tr_owned("cast.player.name"),
        Some(s) => s.name.split(' ').next().unwrap_or(&s.name).to_string(),
        None => String::new(),
    }
}

fn trait_name(loc: &Locale, tr: &str) -> String {
    loc.tr_owned(&format!("trait.{tr}.name"))
}

/// The reason line next to a bar.
pub fn feeling_text(g: &GameScreen, loc: &Locale, f: &Feeling) -> String {
    let key = format!("crew.why.{}", f.id());
    match f {
        Feeling::LongShift { hours } => loc.trf(&key, &[("n", &hours.to_string())]),
        Feeling::TraitKeepsGoing { tr }
        | Feeling::TraitDrains { tr }
        | Feeling::TraitMood { tr, .. } => loc.trf(&key, &[("trait", &trait_name(loc, tr))]),
        Feeling::WithPartner { staff } | Feeling::Friend { staff } | Feeling::Rival { staff } => {
            loc.trf(&key, &[("name", &first_name(loc, &g.world, *staff))])
        }
        Feeling::PrefBroken { pref, need } => {
            let p = crate::content::handson::Pref {
                kind: pref.clone(),
                strength: if *need {
                    crate::content::handson::Strength::Need
                } else {
                    crate::content::handson::Strength::Wish
                },
                quote: String::new(),
            };
            let chip = super::app_tool::chip_of(g, loc, &p, sim::PrefStatus::Violated);
            loc.trf(&key, &[("pref", &chip.label)])
        }
        _ => loc.tr_owned(&key),
    }
}

fn shift_span(st: &ShiftTonight) -> String {
    format!(
        "{}–{}",
        wv::clock_of_minute(st.from),
        wv::clock_of_minute(st.to)
    )
}

/// Skill 0–10 → a word (`crew.skill.<0–4>`).
pub fn skill_word(loc: &Locale, v: f32) -> String {
    let band = ((v.clamp(0.0, 10.0) + 0.5) as u32 / 2).min(4);
    loc.tr_owned(&format!("crew.skill.{}", band.min(4)))
}

// ---------------------------------------------------------------------------------------------
// The card's data (owned, so the drawing can send commands)
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
enum Act {
    Follow,
    Break { resting: bool },
    Home,
    Rozpis,
    LetGo,
    Duties,
    Book,
}

#[derive(Clone, Debug)]
struct Bar {
    need: Need,
    value: f32,
    reason: String,
    negative: bool,
}

#[derive(Clone, Debug)]
struct Duties {
    held: Vec<Duty>,
    cap: u32,
    /// Skill per duty (`Duty::ALL` order).
    skills: [f32; 6],
    likes: Option<Duty>,
}

#[derive(Clone, Debug, Default)]
struct Card {
    id: u32,
    owner: bool,
    name: String,
    role: String,
    status: String,
    status_color: Option<Color>,
    now: Option<String>,
    /// Karolina's progress ring of the current act, and whether × cancels it.
    now_progress: Option<f32>,
    now_cancel: bool,
    /// (text, unqueue index) — Karolina's queue; a helper's single "next".
    next: Vec<(String, Option<usize>)>,
    next_more: usize,
    /// Karolina: (icon, count, tooltip); `None` count = no number.
    waiting: Option<Vec<(Icon, Option<u32>, String)>>,
    /// Karolina: (value, label, tooltip) tiles.
    stats: Vec<(String, String, String)>,
    stats_title: String,
    yours: Option<Vec<Duty>>,
    duties: Option<Duties>,
    bars: Vec<Bar>,
    traits: Vec<(String, String, Tone)>,
    prefs: Option<Vec<PrefChip>>,
    rel: Option<String>,
    actions: Vec<Act>,
}

fn status_of(
    g: &GameScreen,
    loc: &Locale,
    s: &Staff,
    st: &ShiftTonight,
) -> (String, Option<Color>) {
    let t = crate::ui::Theme::SHIFTELLA;
    let night = g.world.phase() == Phase::Night;
    if s.owner {
        return if night {
            (loc.tr_owned("crew.status.counter"), Some(t.emerald_400))
        } else {
            (loc.tr_owned("crew.status.planning"), None)
        };
    }
    if !night {
        return match (st.shift, st.leave) {
            (_, Some(_)) => (loc.tr_owned("crew.status.leave"), None),
            (sim::Shift::Off, _) => (loc.tr_owned("crew.status.off"), None),
            _ => (
                loc.trf("crew.status.tonight", &[("time", &shift_span(st))]),
                Some(t.indigo_300),
            ),
        };
    }
    match s.status {
        StaffStatus::OnDuty => (shift_span(st), Some(t.emerald_400)),
        StaffStatus::Expected { minute } => (
            loc.trf(
                "crew.status.from",
                &[("time", &wv::clock_of_minute(minute))],
            ),
            Some(t.amber),
        ),
        StaffStatus::Leaving => (loc.tr_owned("crew.status.leaving"), None),
        StaffStatus::Sick => (loc.tr_owned("crew.status.sick"), Some(t.red_400)),
        StaffStatus::WalkedOut => (loc.tr_owned("crew.status.walked_out"), Some(t.red_400)),
        StaffStatus::OffDuty => match st.leave {
            Some(_) => (loc.tr_owned("crew.status.leave"), None),
            None => (loc.tr_owned("crew.status.off"), None),
        },
    }
}

fn relations_line(g: &GameScreen, loc: &Locale, s: &Staff) -> Option<String> {
    let w = &g.world;
    let others: Vec<&Staff> = w
        .staff()
        .iter()
        .filter(|o| !o.owner && o.id != s.id)
        .collect();
    if others.is_empty() {
        return None;
    }
    let mut parts = Vec::new();
    if let Some(p) = s.romance_with.filter(|p| others.iter().any(|o| o.id == *p)) {
        parts.push(loc.trf("crew.rel.partner", &[("name", &first_name(loc, w, p))]));
    }
    let mut friends: Vec<(f32, u32)> = others
        .iter()
        .filter(|o| Some(o.id) != s.romance_with)
        .map(|o| (w.relationship(s.id, o.id), o.id))
        .filter(|(r, _)| *r >= 40.0)
        .collect();
    friends.sort_by(|a, b| b.0.total_cmp(&a.0));
    if !friends.is_empty() {
        let names: Vec<String> = friends
            .iter()
            .take(2)
            .map(|(_, id)| first_name(loc, w, *id))
            .collect();
        parts.push(loc.trf("crew.rel.friends", &[("names", &names.join(", "))]));
    }
    let rival = others
        .iter()
        .map(|o| (w.relationship(s.id, o.id), o.id))
        .filter(|(r, _)| *r <= -25.0)
        .min_by(|a, b| a.0.total_cmp(&b.0));
    if let Some((_, id)) = rival {
        parts.push(loc.trf("crew.rel.rival", &[("name", &first_name(loc, w, id))]));
    }
    if parts.is_empty() {
        parts.push(loc.tr_owned(if w.night().saturating_sub(s.hired_night) < 2 {
            "crew.rel.new"
        } else {
            "crew.rel.everyone"
        }));
    }
    Some(parts.join(" · "))
}

fn karolina_card(g: &GameScreen, loc: &Locale, content: &Content, s: &Staff) -> Card {
    let w = &g.world;
    let night = w.phase() == Phase::Night;
    let classic = w.difficulty() == sim::Difficulty::Classic;
    let st = w.shift_tonight(s.id).unwrap_or(ShiftTonight {
        shift: sim::Shift::Full,
        from: 0,
        to: 480,
        leave: None,
    });
    let (status, status_color) = status_of(g, loc, s, &st);
    let mut c = Card {
        id: s.id,
        owner: true,
        name: wv::staff_name(loc, s),
        role: loc.tr_owned("crew.role.karolina"),
        status,
        status_color,
        ..Default::default()
    };
    let plan = w.karolina_plan();
    if night {
        match plan.first() {
            Some(a) => {
                c.now = Some(activity_text(loc, content, a));
                c.now_progress = w.player().current.as_ref().map(|a| a.progress());
                c.now_cancel = true;
            }
            None => c.now = w.activity(s.id).map(|a| activity_text(loc, content, &a)),
        }
        for (i, a) in plan.iter().enumerate().skip(1).take(3) {
            c.next.push((activity_text(loc, content, a), Some(i)));
        }
        c.next_more = plan.len().saturating_sub(4);
        let p = w.karolina_todo_parts(content);
        let mut chips = Vec::new();
        let mut add = |icon: Icon, n: u32, key: &str, number: bool| {
            if n > 0 {
                chips.push((icon, number.then_some(n), loc.tr_owned(key)));
            }
        };
        add(Icon::Note, p.orders, "crew.wait.orders", true);
        add(Icon::Heart, p.wishes, "crew.wait.wishes", true);
        add(Icon::Coffee, p.brews, "crew.wait.brews", true);
        add(Icon::Tray, p.trays, "crew.wait.trays", true);
        add(Icon::Dishes, p.tables, "crew.wait.tables", true);
        add(Icon::Bowl, p.sink, "crew.wait.sink", true);
        add(Icon::Broom, u32::from(p.floor), "crew.wait.floor", false);
        let coins = w.tips_on_tables().len() as u32;
        add(Icon::Money, coins, "crew.wait.tips", true);
        let moments = w
            .active_moments()
            .iter()
            .filter(|m| m.resolving.is_none())
            .count() as u32;
        add(Icon::Sparkle, moments, "crew.wait.moments", true);
        c.waiting = Some(chips);
    } else {
        c.now = Some(loc.tr_owned("crew.do.planning"));
    }
    let tally = w.hands_tally();
    c.stats_title = loc.tr_owned(if night {
        "crew.tonight"
    } else {
        "crew.last_night"
    });
    let stat = |v: String, key: &str| (v, loc.tr_owned(key), loc.tr_owned(&format!("{key}_tip")));
    c.stats = vec![
        stat(tally.karolina_items.to_string(), "crew.stat.cups"),
        stat(tally.tiers[0].to_string(), "crew.stat.perfect"),
        stat(
            format_money(loc.lang, tally.tips_collected),
            "crew.stat.tips",
        ),
        if night {
            stat(
                format!("{} / {}", w.rhythm(), tally.best_rhythm.max(w.rhythm())),
                "crew.stat.rhythm",
            )
        } else {
            (
                tally.best_rhythm.to_string(),
                loc.tr_owned("crew.stat.rhythm_best"),
                loc.tr_owned("crew.stat.rhythm_tip"),
            )
        },
    ];
    c.yours = Some(w.duties_left_for_karolina());
    if classic {
        let f = w.feelings(content, s.id);
        if let Some(first) = f.energy.first() {
            c.bars.push(Bar {
                need: Need::Energy,
                value: s.energy / 100.0,
                reason: feeling_text(g, loc, first),
                negative: first.negative(),
            });
        }
    }
    if night {
        c.actions.push(Act::Follow);
    }
    c.actions.push(Act::Duties);
    c.actions.push(Act::Book);
    c
}

fn helper_card(g: &GameScreen, loc: &Locale, content: &Content, s: &Staff) -> Card {
    let w = &g.world;
    let night = w.phase() == Phase::Night;
    let classic = w.difficulty() == sim::Difficulty::Classic;
    let st = w.shift_tonight(s.id).unwrap_or(ShiftTonight {
        shift: sim::Shift::Off,
        from: 0,
        to: 0,
        leave: None,
    });
    let (status, status_color) = status_of(g, loc, s, &st);
    let role = {
        let r = wv::staff_role(loc, content, s);
        loc.trf(
            "crew.role.helper",
            &[("role", &r), ("age", &s.age.to_string())],
        )
    };
    let mut c = Card {
        id: s.id,
        owner: false,
        name: wv::staff_name(loc, s),
        role,
        status,
        status_color,
        ..Default::default()
    };
    let on = night && s.on_duty();
    if night {
        c.now = w.activity(s.id).map(|a| activity_text(loc, content, &a));
        if let Some(n) = w.next_activity(content, s.id) {
            c.next.push((activity_text(loc, content, &n), None));
        }
    }
    let likes = content
        .staff_archetype(&s.archetype)
        .and_then(|a| a.default_duty);
    c.duties = Some(Duties {
        held: s.duties.clone(),
        cap: w.capacity(content, s.id).max(1),
        skills: Duty::ALL.map(|d| s.skill(d.job(), w.night())),
        likes,
    });
    let f = w.feelings(content, s.id);
    if let Some(m) = f.mood.first() {
        c.bars.push(Bar {
            need: Need::Mood,
            value: s.mood / 100.0,
            reason: feeling_text(g, loc, m),
            negative: m.negative(),
        });
    }
    // energy: always on Classic; on Cozy only when it tells something (they tire, then rest)
    let tired = s.energy < 40.0 || s.hunger < 30.0;
    if ((classic && (on || !night)) || (on && tired))
        && let Some(e) = f.energy.first()
    {
        c.bars.push(Bar {
            need: Need::Energy,
            value: s.energy / 100.0,
            reason: feeling_text(g, loc, e),
            negative: e.negative(),
        });
    }
    for tr in &s.traits {
        let tone = match content.trait_def(tr).map(|d| d.group) {
            Some(TraitGroup::Positive) => Tone::Ok,
            Some(TraitGroup::Negative) => Tone::Warn,
            _ => Tone::Neutral,
        };
        c.traits.push((
            trait_name(loc, tr),
            loc.tr_owned(&format!("trait.{tr}.desc")),
            tone,
        ));
    }
    let prefs = w.pref_status(content, s.id);
    c.prefs = Some(
        prefs
            .iter()
            .map(|(p, st)| super::app_tool::chip_of(g, loc, p, *st))
            .collect(),
    );
    c.rel = relations_line(g, loc, s);
    if night && s.present() {
        c.actions.push(Act::Follow);
    }
    if on {
        let resting = matches!(s.task, Some(sim::Task::Break { .. }));
        if classic || tired || resting {
            c.actions.push(Act::Break { resting });
        }
        c.actions.push(Act::Home);
    }
    c.actions.push(Act::Rozpis);
    if !night {
        c.actions.push(Act::LetGo);
    }
    c
}

// ---------------------------------------------------------------------------------------------
// Layout + drawing (one walk: `draw = false` measures)
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
enum Clicked {
    Act(Act),
    Duty(Duty, bool),
    Unqueue(usize),
}

fn act_label(loc: &Locale, a: Act) -> (Icon, String, String) {
    let (icon, key) = match a {
        Act::Follow => (Icon::Search, "crew.act.follow"),
        Act::Break { .. } => (Icon::Cup, "crew.act.break"),
        Act::Home => (Icon::Home, "crew.act.home"),
        Act::Rozpis => (Icon::Calendar, "crew.act.rozpis"),
        Act::LetGo => (Icon::Close, "crew.act.let_go"),
        Act::Duties => (Icon::Priorities, "crew.act.duties"),
        Act::Book => (Icon::Book, "crew.act.book"),
    };
    (icon, loc.tr_owned(key), loc.tr_owned(&format!("{key}_tip")))
}

const ACT_H: f32 = 26.0;

fn act_style(t: &crate::ui::Theme) -> TextStyle {
    TextStyle::new(12.0, Weight::SemiBold, t.text)
}

/// Natural width of a compact action button (icon + 12 pt label).
fn act_width(ui: &Ui, label: &str) -> f32 {
    (ui.fonts.measure(label, &act_style(&ui.theme)) + 35.0).ceil()
}

/// The action buttons in rows: one row when they fit, else as few balanced rows as fit.
fn action_rows(ui: &Ui, loc: &Locale, acts: &[Act], w: f32) -> Vec<Vec<Act>> {
    let widths: Vec<f32> = acts
        .iter()
        .map(|a| act_width(ui, &act_label(loc, *a).1))
        .collect();
    for rows in 1..=acts.len().max(1) {
        let per = acts.len().div_ceil(rows);
        let chunks: Vec<&[f32]> = widths.chunks(per.max(1)).collect();
        let fits = chunks.iter().all(|c| {
            let max = c.iter().copied().fold(0.0, f32::max);
            // even split: every button gets the row's width / n
            max * c.len() as f32 + 6.0 * (c.len() as f32 - 1.0) <= w
                || c.iter().sum::<f32>() + 6.0 * (c.len() as f32 - 1.0) <= w
        });
        if fits {
            return acts.chunks(per.max(1)).map(|c| c.to_vec()).collect();
        }
    }
    acts.iter().map(|a| vec![*a]).collect()
}

/// A compact one-click action (icon + label); returns clicked.
#[allow(clippy::too_many_arguments)]
fn act_button(
    ui: &mut Ui,
    key: impl std::hash::Hash,
    r: Rect,
    icon: Icon,
    label: &str,
    tip: &str,
    kind: crate::ui::ButtonKind,
    selected: bool,
    disabled: bool,
) -> bool {
    let t = ui.theme.clone();
    let id = ui.id(key);
    let resp = ui.interact(id, r);
    if disabled {
        crate::ui::draw::push_alpha(0.5);
    }
    crate::ui::widgets::paint_surface(&t, r, 6.0, kind, selected, resp.hover_t, resp.press_t);
    let fg = if selected {
        t.text_on_accent
    } else if kind == crate::ui::ButtonKind::Danger {
        t.red_400
    } else {
        t.text
    };
    let st = act_style(&t).color(fg);
    let lw = ui.fonts.measure(label, &st).min(r.w - 35.0);
    let x0 = r.x + ((r.w - lw - 19.0) / 2.0).max(8.0);
    draw_icon(icon, vec2(x0 + 7.0, r.center_y()), 14.0, fg);
    ui.fonts.draw_in(
        label,
        Rect::new(x0 + 19.0, r.y, (r.right() - 8.0 - x0 - 19.0).max(0.0), r.h),
        &st,
        Align::Left,
    );
    if disabled {
        crate::ui::draw::pop_alpha();
    }
    ui.tooltip(&resp, tip);
    let clicked = resp.clicked && !disabled;
    if clicked {
        ui.sound(UiSound::Click);
    }
    clicked
}

fn chip_rows(ui: &Ui, texts: &[String], w: f32) -> usize {
    let (mut rows, mut x) = (1, 0.0);
    for t in texts {
        let cw = ui.chip_width(t, false) + 6.0;
        if x > 0.0 && x + cw > w {
            rows += 1;
            x = 0.0;
        }
        x += cw;
    }
    if texts.is_empty() { 0 } else { rows }
}

/// The left label column: as wide as the longest label in this language (44–92 pt).
fn label_w(ui: &Ui, loc: &Locale) -> f32 {
    let st = ui.theme.micro();
    [
        "crew.now",
        "crew.next",
        "crew.waiting",
        "crew.yours",
        "crew.mood",
        "crew.energy",
        "crew.rozpis",
    ]
    .iter()
    .map(|k| ui.fonts.measure(&loc.tr(k).to_uppercase(), &st))
    .fold(0.0, f32::max)
    .clamp(36.0, 84.0)
        + 8.0
}

fn label(ui: &mut Ui, r: Rect, text: &str, lw: f32) {
    let t = ui.theme.clone();
    ui.fonts.draw_in(
        &text.to_uppercase(),
        Rect::new(r.x, r.y, lw - 6.0, r.h),
        &t.micro(),
        Align::Left,
    );
}

/// Walks the card: draws it when `draw` (else only measures). Returns the height used and
/// what was clicked.
fn walk(
    ui: &mut Ui,
    loc: &Locale,
    c: &Card,
    area: Rect,
    draw: bool,
    time: f64,
) -> (f32, Option<Clicked>) {
    let t = ui.theme.clone();
    let x = area.x;
    let w = area.w;
    let mut y = area.y;
    let mut clicked = None;
    let lw = label_w(ui, loc);
    let tip_w = w - lw;
    // --- header: face, name, role, status pill
    if draw {
        ui.avatar(vec2(x + 17.0, y + 18.0), 34.0, &c.name, None, false);
        let pill_st = TextStyle::new(11.0, Weight::SemiBold, t.text);
        let pw = (ui.fonts.measure(&c.status, &pill_st)
            + 18.0
            + if c.status_color.is_some() { 10.0 } else { 0.0 })
        .min(w * 0.5)
        .ceil();
        let pr = Rect::new(x + w - pw, y + 1.0, pw, 18.0);
        fill_rrect(pr, 9.0, Color::new(1.0, 1.0, 1.0, 0.07));
        stroke_rrect(pr, 9.0, 1.0, Color::new(1.0, 1.0, 1.0, 0.08));
        let mut tx = pr.x + 9.0;
        if let Some(col) = c.status_color {
            fill_circle(vec2(tx + 3.0, pr.center_y()), 3.0, col);
            tx += 10.0;
        }
        ui.fonts.draw_in(
            &c.status,
            Rect::new(tx, pr.y, pr.right() - tx - 8.0, pr.h),
            &pill_st,
            Align::Left,
        );
        ui.fonts.draw_in(
            &c.name,
            Rect::new(x + 44.0, y, pr.x - x - 50.0, 19.0),
            &t.h2(),
            Align::Left,
        );
        ui.fonts.draw_in(
            &c.role,
            Rect::new(x + 44.0, y + 19.0, w - 44.0, 16.0),
            &t.small(),
            Align::Left,
        );
    }
    y += HEADER_H + GAP;
    // --- now / next
    if let Some(now) = &c.now {
        if draw {
            let r = Rect::new(x, y, w, LINE);
            label(ui, r, loc.tr("crew.now"), lw);
            let cancel_w = if c.now_cancel { 20.0 } else { 0.0 };
            let tr = Rect::new(x + lw, y, tip_w - cancel_w, LINE);
            ui.fonts
                .draw_in(now, tr, &t.small_medium().color(t.indigo_300), Align::Left);
            let resp = ui.interact(ui.id(("crew_now", c.id)), tr);
            ui.tooltip(&resp, now);
            if c.now_cancel {
                let xr = Rect::new(x + w - 18.0, y, 18.0, LINE);
                if x_button(ui, ("crew_cancel", c.id), xr, loc.tr("crew.drop_tip")) {
                    clicked = Some(Clicked::Unqueue(0));
                }
            }
            if let Some(p) = c.now_progress.filter(|p| *p > 0.0) {
                ui.progress(
                    Rect::new(x + lw, y + LINE, (tip_w - cancel_w).min(120.0), 3.0),
                    p,
                    t.indigo_300,
                );
            }
        }
        y += LINE
            + if c.now_progress.is_some_and(|p| p > 0.0) {
                4.0
            } else {
                0.0
            };
        let n = c.next.len() + usize::from(c.next_more > 0);
        if c.owner && c.next.is_empty() && c.now_cancel {
            // her queue is empty: say how to fill it
            if draw {
                let r = Rect::new(x, y, w, LINE);
                label(ui, r, loc.tr("crew.next"), lw);
                ui.fonts.draw_in(
                    loc.tr("crew.queue_empty"),
                    Rect::new(x + lw, y, tip_w, LINE),
                    &t.small(),
                    Align::Left,
                );
            }
            y += LINE;
        }
        for (i, (text, unq)) in c.next.iter().enumerate() {
            if draw {
                let r = Rect::new(x, y, w, LINE);
                if i == 0 {
                    label(ui, r, loc.tr("crew.next"), lw);
                }
                let xw = if unq.is_some() { 20.0 } else { 0.0 };
                let tr = Rect::new(x + lw, y, tip_w - xw, LINE);
                ui.fonts
                    .draw_in(text, tr, &t.small().color(t.text), Align::Left);
                let resp = ui.interact(ui.id(("crew_next", c.id, i)), tr);
                ui.tooltip(&resp, text);
                if let Some(k) = unq {
                    let xr = Rect::new(x + w - 18.0, y, 18.0, LINE);
                    if x_button(ui, ("crew_unq", c.id, *k), xr, loc.tr("crew.drop_tip")) {
                        clicked = Some(Clicked::Unqueue(*k));
                    }
                }
            }
            y += LINE;
        }
        if c.next_more > 0 {
            if draw {
                ui.fonts.draw_in(
                    &loc.trf("crew.more", &[("n", &c.next_more.to_string())]),
                    Rect::new(x + lw, y, tip_w, LINE - 2.0),
                    &t.small(),
                    Align::Left,
                );
            }
            y += LINE - 2.0;
        }
        let _ = n;
        y += GAP;
    }
    // --- Karolina: what waits only for her
    if let Some(chips) = &c.waiting {
        if draw {
            let r = Rect::new(x, y, w, 20.0);
            label(ui, r, loc.tr("crew.waiting"), lw);
            let mut cx = x + lw;
            if chips.is_empty() {
                ui.fonts.draw_in(
                    loc.tr("crew.wait.none"),
                    Rect::new(cx, y, tip_w, 20.0),
                    &t.small(),
                    Align::Left,
                );
            }
            for (i, (icon, n, tip)) in chips.iter().enumerate() {
                let txt = n.map(|n| n.to_string()).unwrap_or_default();
                let st = TextStyle::new(11.0, Weight::SemiBold, t.text).tabular();
                let cw =
                    (ui.fonts.measure(&txt, &st) + if txt.is_empty() { 22.0 } else { 28.0 }).ceil();
                if cx + cw > x + w {
                    break;
                }
                let cr = Rect::new(cx, y + 1.0, cw, 18.0);
                fill_rrect(cr, 9.0, with_a(t.amber, 0.14));
                draw_icon(*icon, vec2(cr.x + 11.0, cr.center_y()), 11.0, t.amber);
                if !txt.is_empty() {
                    ui.fonts
                        .draw(&txt, cr.x + 20.0, ui.fonts.baseline_in(cr, &st), &st);
                }
                let resp = ui.interact(ui.id(("crew_wait", i)), cr);
                ui.tooltip(&resp, tip);
                cx += cw + 4.0;
            }
        }
        y += 20.0 + GAP;
    }
    // --- Karolina: tonight's numbers
    if !c.stats.is_empty() {
        if draw {
            ui.fonts.draw_in(
                &c.stats_title.to_uppercase(),
                Rect::new(x, y, w, 14.0),
                &t.micro(),
                Align::Left,
            );
        }
        y += 15.0;
        if draw {
            let n = c.stats.len() as f32;
            let tw = (w - 6.0 * (n - 1.0)) / n;
            for (i, (v, l, tip)) in c.stats.iter().enumerate() {
                let r = Rect::new(x + i as f32 * (tw + 6.0), y, tw, 32.0);
                let resp = ui.interact(ui.id(("crew_stat", i)), r);
                ui.tooltip(&resp, tip);
                fill_rrect(r, 8.0, Color::new(1.0, 1.0, 1.0, 0.045));
                ui.fonts.draw_in(
                    v,
                    Rect::new(r.x + 6.0, r.y + 1.0, r.w - 12.0, 17.0),
                    &t.number().size(14.0),
                    Align::Center,
                );
                ui.fonts.draw_in(
                    l,
                    Rect::new(r.x + 4.0, r.y + 17.0, r.w - 8.0, 14.0),
                    &t.small().size(11.0),
                    Align::Center,
                );
            }
        }
        y += 32.0 + GAP;
    }
    // --- Karolina: duties nobody holds
    if let Some(yours) = &c.yours {
        if draw {
            label(ui, Rect::new(x, y, w, 20.0), loc.tr("crew.yours"), lw);
            let mut cx = x + lw;
            if yours.is_empty() {
                ui.fonts.draw_in(
                    loc.tr("crew.yours_none"),
                    Rect::new(cx, y, tip_w, 20.0),
                    &t.small(),
                    Align::Left,
                );
            }
            let names: Vec<String> = yours.iter().map(|d| loc.tr_owned(&d.name_key())).collect();
            let st = TextStyle::new(11.0, Weight::SemiBold, t.text);
            let full: f32 = names.iter().map(|n| ui.fonts.measure(n, &st) + 30.0).sum();
            let icons_only = full > tip_w;
            for (d, n) in yours.iter().zip(&names) {
                let cw = if icons_only {
                    24.0
                } else {
                    (ui.fonts.measure(n, &st) + 26.0).ceil()
                };
                let cr = Rect::new(cx, y + 1.0, cw, 18.0);
                fill_rrect(cr, 9.0, with_a(t.indigo, 0.18));
                draw_icon(
                    to_ui(*d).icon(),
                    vec2(cr.x + 11.0, cr.center_y()),
                    11.0,
                    t.indigo_300,
                );
                if !icons_only {
                    ui.fonts
                        .draw(n, cr.x + 20.0, ui.fonts.baseline_in(cr, &st), &st);
                }
                let resp = ui.interact(ui.id(("crew_yours", d.id())), cr);
                ui.tooltip(&resp, &loc.trf("crew.yours_tip", &[("duty", n)]));
                cx += cw + 4.0;
            }
        }
        y += 20.0 + GAP;
    }
    // --- a helper's duties: one-click toggles (the Duties board's command path)
    if let Some(d) = &c.duties {
        if draw {
            let title = loc.tr("crew.duties").to_uppercase();
            let tw = ui.fonts.draw_in(
                &title,
                Rect::new(x, y, w * 0.5, 14.0),
                &t.micro(),
                Align::Left,
            );
            let used = d.held.len();
            let end = capacity_dots(&t, x + tw + 8.0, y + 7.0, used, d.cap as usize);
            let cap_txt = format!("{used}/{}", d.cap);
            let cw = ui.fonts.measure(&cap_txt, &t.small().size(11.0)) + 2.0;
            let cr = Rect::new(end + 6.0, y, cw, 14.0);
            ui.fonts
                .draw_in(&cap_txt, cr, &t.small().size(11.0), Align::Left);
            let mut tip = loc.trf(
                "ui.duties.capacity",
                &[("n", &used.to_string()), ("max", &d.cap.to_string())],
            );
            if d.cap < 3 {
                tip = format!("{tip}\n{}", loc.tr("ui.duties.grows"));
            }
            let resp = ui.interact(
                ui.id(("crew_cap", c.id)),
                Rect::new(x, y, end + cw + 6.0 - x, 14.0),
            );
            ui.tooltip(&resp, &tip);
        }
        y += 16.0;
        if draw {
            let n = Duty::ALL.len() as f32;
            let cw = (w - 4.0 * (n - 1.0)) / n;
            for (i, duty) in Duty::ALL.iter().enumerate() {
                let r = Rect::new(x + i as f32 * (cw + 4.0), y, cw, 38.0);
                let on = d.held.contains(duty);
                let full = !on && d.held.len() as u32 >= d.cap;
                let id = ui.id(("crew_duty", c.id, i));
                let resp = ui.interact(id, r);
                let bg = if on {
                    with_a(t.indigo, 0.42 + 0.12 * resp.hover_t)
                } else {
                    Color::new(1.0, 1.0, 1.0, 0.05 + 0.05 * resp.hover_t)
                };
                fill_rrect(r, 8.0, bg);
                stroke_rrect(
                    r,
                    8.0,
                    1.0,
                    if on {
                        with_a(t.indigo_300, 0.55)
                    } else {
                        Color::new(1.0, 1.0, 1.0, 0.08)
                    },
                );
                let fg = if on {
                    t.text
                } else if full {
                    t.text_disabled
                } else {
                    t.text_dim
                };
                draw_icon(
                    to_ui(*duty).icon(),
                    vec2(r.center().x, r.y + 11.0),
                    13.0,
                    fg,
                );
                if on {
                    draw_icon(
                        Icon::Check,
                        vec2(r.right() - 7.0, r.y + 7.0),
                        9.0,
                        t.emerald_400,
                    );
                } else if d.likes == Some(*duty) {
                    draw_icon(
                        Icon::Heart,
                        vec2(r.right() - 7.0, r.y + 7.0),
                        8.0,
                        t.violet_400,
                    );
                }
                let name = loc.tr_owned(&duty.name_key());
                ui.fonts.draw_in(
                    &name,
                    Rect::new(r.x + 2.0, r.y + 19.0, r.w - 4.0, 11.0),
                    &TextStyle::new(10.0, Weight::Medium, fg),
                    Align::Center,
                );
                // skill pips (0–5): how good they are at it
                let pips = ((d.skills[i] + 1.0) / 2.0).floor().clamp(0.0, 5.0) as usize;
                let pw = 4.0;
                let px0 = r.center().x - (5.0 * pw + 4.0 * 1.5) / 2.0;
                for k in 0..5 {
                    let pr = Rect::new(px0 + k as f32 * (pw + 1.5), r.bottom() - 6.0, pw, 2.0);
                    fill_rrect(
                        pr,
                        1.0,
                        if k < pips {
                            if on {
                                t.indigo_300
                            } else {
                                with_a(t.indigo_300, 0.7)
                            }
                        } else {
                            Color::new(1.0, 1.0, 1.0, 0.1)
                        },
                    );
                }
                let mut tip = format!(
                    "{} — {}\n{}",
                    name,
                    loc.tr_owned(&duty.desc_key()),
                    loc.trf(
                        "crew.skill_line",
                        &[
                            ("word", &skill_word(loc, d.skills[i])),
                            ("n", &format!("{:.0}", d.skills[i])),
                        ],
                    )
                );
                if d.likes == Some(*duty) {
                    tip.push('\n');
                    tip.push_str(loc.tr("ui.duties.likes"));
                }
                if full {
                    tip.push('\n');
                    tip.push_str(loc.tr("ui.duties.full_short"));
                }
                ui.tooltip(&resp, &tip);
                if resp.clicked {
                    ui.sound(UiSound::Toggle);
                    clicked = Some(Clicked::Duty(*duty, !on));
                }
            }
        }
        y += 38.0 + GAP;
    }
    // --- bars with the reason
    for (i, b) in c.bars.iter().enumerate() {
        if draw {
            let r = Rect::new(x, y, w, LINE);
            label(
                ui,
                r,
                loc.tr(match b.need {
                    Need::Mood => "crew.mood",
                    _ => "crew.energy",
                }),
                lw,
            );
            ui.need_bar(Rect::new(x + lw, y + 6.0, 64.0, 5.0), b.need, b.value);
            let rr = Rect::new(x + lw + 72.0, y, w - lw - 72.0, LINE);
            ui.fonts.draw_in(
                &b.reason,
                rr,
                &t.small()
                    .color(if b.negative { t.amber } else { t.text_dim }),
                Align::Left,
            );
            let resp = ui.interact(ui.id(("crew_bar", c.id, i)), rr);
            ui.tooltip(
                &resp,
                &format!("{} {:.0} / 100", b.reason, (b.value * 100.0).round()),
            );
        }
        y += LINE + 1.0;
    }
    if !c.bars.is_empty() {
        y += GAP - 1.0;
    }
    // --- traits
    if !c.traits.is_empty() {
        let names: Vec<String> = c.traits.iter().map(|t| t.0.clone()).collect();
        let rows = chip_rows(ui, &names, w);
        if draw {
            let (mut cx, mut cy) = (x, y);
            for (name, desc, tone) in &c.traits {
                let cw = ui.chip_width(name, false);
                if cx > x && cx + cw > x + w {
                    cx = x;
                    cy += 26.0;
                }
                let r = ui.chip(vec2(cx, cy), name, *tone, None);
                let resp = ui.interact(ui.id(("crew_trait", c.id, name.as_str())), r);
                ui.tooltip(&resp, desc);
                cx = r.right() + 6.0;
            }
        }
        y += rows as f32 * 26.0 - 4.0 + GAP;
    }
    // --- rozpis wishes, and who they get along with (on the same row when it fits)
    let rel_w = c
        .rel
        .as_ref()
        .map(|r| ui.fonts.measure(r, &t.small()) + 22.0)
        .unwrap_or(0.0);
    let prefs_w: f32 = match &c.prefs {
        Some(p) if p.is_empty() => ui.fonts.measure(loc.tr("crew.no_prefs"), &t.small()) + 4.0,
        Some(p) => p.iter().map(|p| pref_chip_width(ui, p) + 4.0).sum(),
        None => 0.0,
    };
    let rel_inline = c.prefs.is_some() && c.rel.is_some() && lw + prefs_w + 8.0 + rel_w <= w;
    let mut rel_x = x;
    if let Some(prefs) = &c.prefs {
        if draw {
            label(ui, Rect::new(x, y, w, 20.0), loc.tr("crew.rozpis"), lw);
            let mut cx = x + lw;
            if prefs.is_empty() {
                let nw = ui.fonts.draw_in(
                    loc.tr("crew.no_prefs"),
                    Rect::new(cx, y, tip_w, 20.0),
                    &t.small(),
                    Align::Left,
                );
                cx += nw + 4.0;
            }
            for (i, p) in prefs.iter().enumerate() {
                let pw = pref_chip_width(ui, p);
                if cx + pw > x + w {
                    break;
                }
                let r = pref_chip(ui, ("crew_pref", c.id, i), vec2(cx, y), p, 0.0);
                cx = r.right() + 4.0;
            }
            rel_x = cx + 4.0;
        }
        if !rel_inline {
            y += 20.0 + 4.0;
        }
    }
    if let Some(rel) = &c.rel {
        if draw {
            let rx = if rel_inline { rel_x } else { x };
            let r = Rect::new(rx, y + if rel_inline { 1.5 } else { 0.0 }, x + w - rx, LINE);
            draw_icon(
                Icon::Heart,
                vec2(rx + 6.0, r.center_y()),
                11.0,
                t.violet_400,
            );
            let tr = Rect::new(rx + 16.0, r.y, r.w - 16.0, LINE);
            ui.fonts.draw_in(rel, tr, &t.small(), Align::Left);
            let resp = ui.interact(ui.id(("crew_rel", c.id)), tr);
            ui.tooltip(&resp, rel);
        }
        y += if rel_inline { 20.0 } else { LINE } + 4.0;
    }
    if c.prefs.is_some() || c.rel.is_some() {
        y += GAP - 4.0;
    }
    // --- quick actions
    let rows = action_rows(ui, loc, &c.actions, w);
    for (ri, row) in rows.iter().enumerate() {
        if draw {
            let n = row.len() as f32;
            let natural: Vec<f32> = row
                .iter()
                .map(|a| act_width(ui, &act_label(loc, *a).1))
                .collect();
            let sum: f32 = natural.iter().sum();
            let spare = (w - 6.0 * (n - 1.0) - sum).max(0.0) / n;
            let even = (w - 6.0 * (n - 1.0)) / n;
            let max = natural.iter().copied().fold(0.0, f32::max);
            let mut bx = x;
            for (i, a) in row.iter().enumerate() {
                // even widths when every label fits, else each its own width plus a share
                let bw = if max <= even {
                    even
                } else {
                    natural[i] + spare
                };
                let r = Rect::new(bx, y, bw, ACT_H);
                bx += bw + 6.0;
                let (icon, label, tip) = act_label(loc, *a);
                let (kind, sel, dis) = match a {
                    Act::Break { resting } => {
                        (crate::ui::ButtonKind::Secondary, *resting, *resting)
                    }
                    Act::LetGo => (crate::ui::ButtonKind::Danger, false, false),
                    _ => (crate::ui::ButtonKind::Secondary, false, false),
                };
                if act_button(
                    ui,
                    ("crew_act", c.id, ri, i),
                    r,
                    icon,
                    &label,
                    &tip,
                    kind,
                    sel,
                    dis,
                ) {
                    clicked = Some(Clicked::Act(*a));
                }
            }
        }
        y += ACT_H + if ri + 1 < rows.len() { 6.0 } else { 0.0 };
    }
    let _ = time;
    (y - area.y, clicked)
}

/// A small × for dropping a queued act.
fn x_button(ui: &mut Ui, key: impl std::hash::Hash, r: Rect, tip: &str) -> bool {
    let t = ui.theme.clone();
    let id = ui.id(key);
    let resp = ui.interact(id, r);
    let c = if resp.hovered {
        t.text
    } else {
        t.text_disabled
    };
    draw_icon(Icon::Close, vec2(r.center().x, r.center_y()), 10.0, c);
    ui.tooltip(&resp, tip);
    if resp.clicked {
        ui.sound(UiSound::Click);
    }
    resp.clicked
}

// ---------------------------------------------------------------------------------------------
// The candidates (the strip's "Hire" cell)
// ---------------------------------------------------------------------------------------------

const CAND_ROW: f32 = 46.0;

/// A candidate's row: 46 pt, taller at a larger text size (three lines of text).
fn cand_row() -> f32 {
    CAND_ROW + 16.0 * ((crate::ui::text::text_scale() - 1.0) / 0.3).clamp(0.0, 1.0)
}

fn good_at(loc: &Locale, content: &Content, s: &Staff) -> String {
    let mut jobs: Vec<(f32, Duty)> = [
        Duty::Bar,
        Duty::Kitchen,
        Duty::Register,
        Duty::Dishes,
        Duty::Serving,
    ]
    .iter()
    .map(|d| (s.skills[d.job().index()], *d))
    .collect();
    jobs.sort_by(|a, b| b.0.total_cmp(&a.0));
    let top: Vec<String> = jobs
        .iter()
        .filter(|(v, _)| *v >= 5.0)
        .take(2)
        .map(|(_, d)| loc.tr_owned(&d.name_key()))
        .collect();
    let mut parts = Vec::new();
    if !top.is_empty() {
        parts.push(loc.trf("crew.cand.good_at", &[("jobs", &top.join(", "))]));
    }
    if let Some(tr) = s.traits.iter().find(|t| {
        content
            .trait_def(t)
            .is_some_and(|d| d.group == TraitGroup::Positive)
    }) {
        parts.push(trait_name(loc, tr));
    }
    if parts.is_empty() {
        parts.push(loc.tr_owned("crew.cand.all_round"));
    }
    parts.join(" · ")
}

fn hire_list(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx, area: Rect) {
    let t = ui.theme.clone();
    let loc = cx.loc;
    let content = cx.content;
    let cands = g.world.candidates().to_vec();
    let mut y = area.y;
    for c in &cands {
        let r = Rect::new(area.x, y, area.w, cand_row());
        y += cand_row() + 2.0;
        // line boxes from the styles (they grow with the text size)
        let ts = t.text_scale;
        let (l1, l2) = (15.0 * ts, 14.0 * ts);
        let top = r.y + ((r.h - l1 - 2.0 * l2) / 2.0).max(0.0);
        let name = c.staff.name.clone();
        ui.avatar(vec2(r.x + 15.0, r.center_y()), 30.0, &name, None, false);
        let b = Button::new(loc.tr("ui.crew.hire"))
            .primary()
            .icon(Icon::Plus);
        let bw = ui.button_width(&b);
        let tx = r.x + 38.0;
        let tw = r.w - 38.0 - bw - 8.0;
        ui.fonts.draw_in(
            &name,
            Rect::new(tx, top, tw, l1),
            &t.strong(),
            Align::Left,
        );
        let role = wv::staff_role(&loc, content, &c.staff);
        let sub = format!(
            "{role} · {}",
            loc.trf(
                "ui.crew.wage",
                &[("money", &format_money(loc.lang, c.staff.wage as i64))]
            )
        );
        ui.fonts.draw_fit(
            &sub,
            Rect::new(tx, top + l1, tw, l2),
            &t.small(),
            Align::Left,
            0.8,
        );
        let good = good_at(&loc, content, &c.staff);
        ui.fonts.draw_fit(
            &good,
            Rect::new(tx, top + l1 + l2, tw, l2),
            &t.small().color(t.indigo_300),
            Align::Left,
            0.8,
        );
        let br = Rect::new(r.right() - bw, r.center_y() - 12.0, bw, 24.0);
        if ui.button(("hire", c.id), br, &b).clicked {
            match g.command(content, Command::Hire { candidate: c.id }) {
                Ok(()) => {
                    g.toast(
                        ToastKind::Success,
                        Icon::Crew,
                        loc.trf("ui.crew.hired_toast", &[("name", &name)]),
                        None,
                    );
                    g.ui_state.crew_selected = g.world.staff().last().map(|s| s.id);
                }
                Err(e) => super::panels::command_error(g, &loc, &e),
            }
        }
    }
}

fn hire_height(g: &GameScreen) -> f32 {
    let n = g.world.candidates().len().max(1) as f32;
    n * (cand_row() + 2.0) - 2.0
}

// ---------------------------------------------------------------------------------------------
// The panel
// ---------------------------------------------------------------------------------------------

fn card_for(g: &GameScreen, loc: &Locale, content: &Content, id: u32) -> Option<Card> {
    let s = g.world.staff_by_id(id)?;
    Some(if s.owner {
        karolina_card(g, loc, content, s)
    } else {
        helper_card(g, loc, content, s)
    })
}

/// The strip of faces: Karolina, the helpers, "Hire". Returns the clicked cell.
fn strip(
    g: &GameScreen,
    ui: &mut Ui,
    loc: &Locale,
    content: &Content,
    r: Rect,
    cells: &[u32],
    sel: u32,
) -> Option<u32> {
    let t = ui.theme.clone();
    let n = cells.len().max(1) as f32;
    let cw = (r.w / n).min(58.0);
    let names = cw >= 40.0;
    let mut out = None;
    for (i, id) in cells.iter().enumerate() {
        let cr = Rect::new(r.x + i as f32 * cw, r.y, cw - 2.0, r.h);
        let selected = *id == sel;
        let resp = ui.interact(ui.id(("crew_cell", *id)), cr);
        if selected {
            fill_rrect(cr, 8.0, with_a(t.indigo, 0.28));
            stroke_rrect(cr, 8.0, 1.0, with_a(t.indigo_300, 0.45));
        } else if resp.hover_t > 0.01 {
            fill_rrect(cr, 8.0, Color::new(1.0, 1.0, 1.0, 0.05 * resp.hover_t));
        }
        let c = vec2(cr.center().x, cr.y + if names { 17.0 } else { cr.h / 2.0 });
        let (label, tip) = if *id == HIRE {
            fill_circle(c, 13.0, with_a(t.indigo, 0.35));
            draw_icon(Icon::Plus, c, 14.0, t.indigo_300);
            let k = g.world.candidates().len();
            ui.badge(c + vec2(11.0, -10.0), k as u32);
            (
                loc.tr_owned("crew.hire_cell"),
                loc.trf("ui.crew.candidates", &[("n", &k.to_string())]),
            )
        } else {
            let Some(s) = g.world.staff_by_id(*id) else {
                continue;
            };
            let name = wv::staff_name(loc, s);
            ui.avatar(c, 26.0, &name, None, selected);
            let dot = match s.status {
                StaffStatus::OnDuty => Some(t.emerald_400),
                StaffStatus::Expected { .. } => Some(t.amber),
                StaffStatus::Sick | StaffStatus::WalkedOut => Some(t.red_400),
                _ => None,
            };
            if let Some(col) = dot.filter(|_| g.world.phase() == Phase::Night) {
                let p = c + vec2(9.0, 9.0);
                fill_circle(p, 4.5, t.ink);
                fill_circle(p, 3.2, col);
            }
            let doing = g
                .world
                .activity(*id)
                .map(|a| activity_text(loc, content, &a))
                .unwrap_or_default();
            (first_name(loc, &g.world, *id), format!("{name}\n{doing}"))
        };
        if names {
            ui.fonts.draw_in(
                &label,
                Rect::new(cr.x + 2.0, cr.y + 32.0, cr.w - 4.0, 13.0),
                &TextStyle::new(
                    10.5,
                    Weight::Medium,
                    if selected { t.text } else { t.text_dim },
                ),
                Align::Center,
            );
        }
        ui.tooltip(&resp, &tip);
        if resp.clicked {
            ui.sound(UiSound::Click);
            out = Some(*id);
        }
    }
    out
}

pub fn panel(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let loc = cx.loc;
    let content = cx.content.clone();
    let mut cells = crew_order(&g.world);
    if !g.world.candidates().is_empty() {
        cells.push(HIRE);
    }
    let Some(&first) = cells.first() else {
        return;
    };
    let mut sel = match g.ui_state.crew_selected {
        Some(id) if cells.contains(&id) => id,
        _ => first,
    };
    // arrow keys step through the crew
    let by = if ui.key_repeat(KeyCode::Right) || ui.key_repeat(KeyCode::Down) {
        1
    } else if ui.key_repeat(KeyCode::Left) || ui.key_repeat(KeyCode::Up) {
        -1
    } else {
        0
    };
    if by != 0 {
        for k in [KeyCode::Left, KeyCode::Right, KeyCode::Up, KeyCode::Down] {
            ui.consume_key(k);
        }
        sel = step(&cells, sel, by);
        select(g, sel);
    }
    g.ui_state.crew_selected = Some(sel);

    let card = (sel != HIRE)
        .then(|| card_for(g, &loc, &content, sel))
        .flatten();
    let s = ui.screen();
    let pw = PANEL_W.min(s.w - 24.0);
    let inner_w = pw - 2.0 * PAD;
    let body_h = match &card {
        Some(c) => walk(ui, &loc, c, Rect::new(0.0, 0.0, inner_w, 0.0), false, 0.0).0,
        None => hire_height(g),
    };
    let avail = (s.h - 88.0 - 60.0).max(200.0);
    let want = 45.0 + 8.0 + STRIP_H + 8.0 + body_h + PAD;
    let pr = Rect::new(12.0, 88.0, pw, want.min(avail));
    g.anchors.push(("panel", pr));
    ui.glass(pr, &Glass::panel());
    let on = g.world.staff().iter().filter(|s| s.on_duty()).count();
    let sub = if g.world.phase() == Phase::Night {
        loc.trf(
            "ui.crew.on_shift",
            &[
                ("on", &on.to_string()),
                ("total", &g.world.staff().len().to_string()),
            ],
        )
    } else {
        String::new()
    };
    let (content_r, close) = ui.panel_header(
        pr,
        loc.tr("ui.crew.title"),
        Some(&crate::settings::keys::label(
            crate::settings::keys::Action::Crew,
        )),
        (!sub.is_empty()).then_some(sub.as_str()),
    );
    if close {
        g.panel = None;
        return;
    }
    let strip_r = Rect::new(pr.x + 8.0, content_r.y + 6.0, pr.w - 16.0, STRIP_H);
    if let Some(id) = strip(g, ui, &loc, &content, strip_r, &cells, sel) {
        select(g, id);
        g.ui_state.crew_selected = Some(id);
    }
    let top = strip_r.bottom() + 8.0;
    crate::ui::draw::hline(pr.x + PAD, pr.right() - PAD, top - 4.0, ui.theme.divider);
    let view = Rect::new(
        pr.x + PAD,
        top,
        inner_w,
        (pr.bottom() - PAD - top).max(10.0),
    );
    let scrolls = body_h > view.h + 0.5;
    // the scroll bar takes a few points: measure again at the narrower width
    let body_h = match (&card, scrolls) {
        (Some(c), true) => {
            walk(
                ui,
                &loc,
                c,
                Rect::new(0.0, 0.0, inner_w - 6.0, 0.0),
                false,
                0.0,
            )
            .0
        }
        _ => body_h,
    };
    let scroll = scrolls.then(|| ui.begin_scroll(("crew_card", sel), view, body_h));
    let area = match &scroll {
        Some(sa) => Rect::new(view.x, sa.top(), view.w - 6.0, body_h),
        None => Rect::new(view.x, view.y, view.w, body_h),
    };
    let clicked = match &card {
        Some(c) => walk(ui, &loc, c, area, true, ui.time()).1,
        None => {
            hire_list(g, ui, cx, area);
            None
        }
    };
    if let Some(sa) = scroll {
        ui.end_scroll(sa);
    }
    if let (Some(c), Some(k)) = (card, clicked) {
        apply(g, ui, cx, &c, k);
    }
}

/// Selects a cell (the café's selection ring follows a person who is inside).
fn select(g: &mut GameScreen, id: u32) {
    g.ui_state.crew_selected = Some(id);
    let present = g
        .world
        .staff_by_id(id)
        .is_some_and(|s| s.present() && g.world.phase() == Phase::Night);
    g.selected = present.then_some(AgentRef::Staff(id));
}

/// Index of a helper's row on the rozpis board (Karolina first, then `staff()` order).
fn rozpis_row(w: &World, id: u32) -> usize {
    let mut row = 1;
    for s in w.staff() {
        if s.owner {
            continue;
        }
        if s.id == id {
            return row;
        }
        row += 1;
    }
    0
}

fn apply(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx, c: &Card, k: Clicked) {
    let loc = cx.loc;
    let content = cx.content.clone();
    let name = c.name.clone();
    match k {
        Clicked::Duty(d, on) => {
            board::set_duty(g, &content, c.id, d, on);
        }
        Clicked::Unqueue(i) => {
            g.intent(&content, Intent::Unqueue(i));
        }
        Clicked::Act(Act::Follow) => {
            if let Some(head) = g.scene.head_of(AgentRef::Staff(c.id), g.time) {
                g.cam_target = Some(head);
                g.selected = Some(AgentRef::Staff(c.id));
            }
        }
        Clicked::Act(Act::Break { .. }) => {
            match g.command(&content, Command::SendOnBreak { staff: c.id }) {
                Ok(()) => g.toast(
                    ToastKind::Info,
                    Icon::Cup,
                    loc.trf("ui.crew.break_toast", &[("name", &name)]),
                    None,
                ),
                Err(e) => super::panels::command_error(g, &loc, &e),
            }
        }
        Clicked::Act(Act::Home) => match g.command(&content, Command::SendHome { staff: c.id }) {
            Ok(()) => g.toast(
                ToastKind::Info,
                Icon::Home,
                loc.trf("crew.home_toast", &[("name", &name)]),
                None,
            ),
            Err(e) => super::panels::command_error(g, &loc, &e),
        },
        Clicked::Act(Act::Rozpis) => {
            g.ui_state.rozpis.cursor = (rozpis_row(&g.world, c.id), g.world.weekday() as usize);
            g.ui_state.rozpis.flash_until = ui.time() + 1.6;
            g.ui_state.app_tab = 0;
            g.panel = Some(Panel::App);
        }
        Clicked::Act(Act::LetGo) => match g.command(&content, Command::Fire { staff: c.id }) {
            Ok(()) => g.toast(
                ToastKind::Info,
                Icon::Crew,
                loc.trf("ui.crew.fired_toast", &[("name", &name)]),
                None,
            ),
            Err(e) => super::panels::command_error(g, &loc, &e),
        },
        Clicked::Act(Act::Duties) => g.panel = Some(Panel::Duties),
        Clicked::Act(Act::Book) => g.open_book(None),
    }
}

impl GameScreen {
    /// Opens the crew panel on someone (screenshot scenes): `"karolina"`, `"helper"` (the
    /// first helper on the floor, else any helper) or `"hire"`.
    pub fn open_crew_card(&mut self, which: &str) {
        let w = &self.world;
        let id = match which {
            "karolina" => w.karolina().map(|s| s.id),
            "hire" => (!w.candidates().is_empty()).then_some(HIRE),
            _ => w
                .staff()
                .iter()
                .filter(|s| !s.owner)
                .max_by_key(|s| {
                    (
                        s.on_duty(),
                        s.task
                            .as_ref()
                            .is_some_and(|t| !matches!(t, sim::Task::Idle { .. })),
                    )
                })
                .map(|s| s.id),
        };
        self.panel = Some(Panel::Crew);
        if let Some(id) = id {
            select(self, id);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::content::Content;
    use crate::settings::load_i18n;

    fn setup() -> (Rc<Content>, crate::i18n::I18n) {
        let root = env!("CARGO_MANIFEST_DIR");
        let c = Rc::new(Content::load(&format!("{root}/assets/data")).expect("content"));
        let i = load_i18n(&format!("{root}/assets/i18n")).expect("i18n");
        (c, i)
    }

    /// Story night 4 at 00:30 with the first hires on the floor.
    fn night(c: &Rc<Content>, loc: &Locale) -> GameScreen {
        let mut g = GameScreen::new_story(c, "cozy", true);
        g.fast_forward(c, loc, 4, Some(150), true);
        g
    }

    #[test]
    fn the_card_toggle_and_the_duties_board_agree() {
        let (c, i) = setup();
        let loc = Locale::new(&i, crate::i18n::Lang::En);
        let mut g = night(&c, &loc);
        let h = g
            .world
            .staff()
            .iter()
            .find(|s| !s.owner)
            .expect("a helper by night 4")
            .clone();
        // a free duty this helper doesn't hold (make room first)
        for d in h.duties.clone() {
            assert!(board::set_duty(&mut g, &c, h.id, d, false));
        }
        let duty = Duty::Floor;
        // the card's toggle is `Clicked::Duty(duty, on)` → `board::set_duty`
        let card = helper_card(&g, &loc, &c, g.world.staff_by_id(h.id).unwrap());
        assert!(!card.duties.as_ref().unwrap().held.contains(&duty));
        assert!(board::set_duty(&mut g, &c, h.id, duty, true));
        let card = helper_card(&g, &loc, &c, g.world.staff_by_id(h.id).unwrap());
        let crew = board::helpers(&g);
        let rows = board::helper_rows(&g, &c, &loc, &crew);
        let row = crew.iter().position(|s| s.id == h.id).unwrap();
        let on_board: Vec<Duty> = rows[row].duties.iter().map(|d| board::to_sim(*d)).collect();
        assert_eq!(card.duties.as_ref().unwrap().held, on_board);
        assert!(on_board.contains(&duty));
        // …and back off, from the board's side this time: the card follows
        assert!(board::set_duty(&mut g, &c, h.id, duty, false));
        let card = helper_card(&g, &loc, &c, g.world.staff_by_id(h.id).unwrap());
        assert!(!card.duties.unwrap().held.contains(&duty));
        // a full helper: the toggle is refused like on the board (a toast, no change)
        let cap = g.world.capacity(&c, h.id);
        let mut given = 0;
        for d in Duty::ALL {
            if given < cap && board::set_duty(&mut g, &c, h.id, d, true) {
                given += 1;
            }
        }
        let before = g.world.staff_duties(h.id).to_vec();
        // (a duty they're skilled at would raise the capacity itself: take another one)
        let night = g.world.night();
        let me = g.world.staff_by_id(h.id).unwrap().clone();
        let bonus = c.handson.tuning.capacity.skill_bonus_at;
        let spare = Duty::ALL
            .iter()
            .find(|d| !before.contains(d) && me.skill(d.job(), night) < bonus)
            .copied();
        if let Some(d) = spare
            && before.len() as u32 >= g.world.capacity(&c, h.id)
        {
            assert!(!board::set_duty(&mut g, &c, h.id, d, true));
            assert_eq!(g.world.staff_duties(h.id), before.as_slice());
        }
    }

    #[test]
    fn karolina_first_and_the_arrows_wrap() {
        let (c, i) = setup();
        let loc = Locale::new(&i, crate::i18n::Lang::En);
        let mut g = night(&c, &loc);
        let order = crew_order(&g.world);
        assert_eq!(Some(order[0]), g.world.karolina().map(|s| s.id));
        assert!(order.len() >= 2);
        let mut cells = order.clone();
        cells.push(HIRE);
        assert_eq!(step(&cells, cells[0], -1), HIRE);
        assert_eq!(step(&cells, HIRE, 1), cells[0]);
        assert_eq!(step(&cells, cells[0], 1), cells[1]);
        // the scene hook opens Karolina's card, a helper's, or hiring
        g.open_crew_card("karolina");
        assert_eq!(g.panel, Some(Panel::Crew));
        assert_eq!(g.ui_state.crew_selected, Some(order[0]));
        g.open_crew_card("helper");
        assert_ne!(g.ui_state.crew_selected, Some(order[0]));
    }

    #[test]
    fn karolinas_card_has_no_break_and_no_energy_on_cozy() {
        let (c, i) = setup();
        let loc = Locale::new(&i, crate::i18n::Lang::En);
        let g = night(&c, &loc);
        let k = g.world.karolina().unwrap();
        let card = karolina_card(&g, &loc, &c, k);
        assert!(card.bars.is_empty(), "no energy on Cozy");
        assert!(
            !card
                .actions
                .iter()
                .any(|a| matches!(a, Act::Break { .. } | Act::Home))
        );
        assert!(card.duties.is_none() && card.waiting.is_some() && card.yours.is_some());
        assert_eq!(card.stats.len(), 4);
        // a helper's card says what they do and has words, not raw keys
        for s in g.world.staff().iter().filter(|s| !s.owner) {
            let card = helper_card(&g, &loc, &c, s);
            for text in card.now.iter().chain(card.next.iter().map(|n| &n.0)) {
                assert!(!text.contains("crew."), "untranslated: {text}");
            }
            for b in &card.bars {
                assert!(!b.reason.contains("crew."), "untranslated: {}", b.reason);
            }
            assert!(!card.status.contains("crew."));
        }
    }

    #[test]
    fn every_activity_and_feeling_has_words_in_all_languages() {
        let (_c, i) = setup();
        let acts = [
            "off",
            "leave",
            "arrives",
            "sick",
            "walked_out",
            "going_home",
            "idle_no_duties",
            "idle",
            "idle_prep",
            "idle_queue",
            "break_eat",
            "break",
            "take_order_register",
            "take_order",
            "make",
            "serve",
            "clear_table",
            "wash",
            "mop",
            "bucket",
            "checklist",
            "collect_tip",
            "record",
            "record_named",
            "moment",
            "experiment",
            "walk",
            "break_at",
            "break_soon",
            "home_at",
            "planning",
        ];
        let why = [
            "on_break",
            "exhausted",
            "tired",
            "hungry",
            "after_yesterday",
            "heavy_week",
            "long_shift",
            "trait_keeps_going",
            "trait_drains",
            "rested",
            "fresh",
            "pref_broken",
            "happy_week",
            "declined",
            "with_partner",
            "friend",
            "rival",
            "trait_mood_good",
            "trait_mood_bad",
            "good",
            "fine",
            "down",
        ];
        for lang in crate::i18n::Lang::ALL {
            for a in acts {
                assert!(i.has(lang, &format!("crew.do.{a}")), "{lang:?} crew.do.{a}");
            }
            for f in why {
                assert!(
                    i.has(lang, &format!("crew.why.{f}")),
                    "{lang:?} crew.why.{f}"
                );
            }
        }
    }
}
