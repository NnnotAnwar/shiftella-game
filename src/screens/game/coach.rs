//! Karolina's coach marks for her first nights (design/handson.md §8, `tutorial.ron`):
//! glass tips with an arrow, in her own words or as one of Lida's notes. They never block
//! input or hold the clock (the order card and the mini-game already slow it down), each
//! has "Got it", and "Skip tutorial" ends them for good.
//!
//! Texts, anchors and voices come from `tutorial.ron`; when a step appears and when it is
//! done is decided here from the live game. When a step appears the sim hears
//! `Command::TutorialShown` (its `on_show` effects — a scripted visit, a forced moment — run
//! once; steps with an `At(time)` trigger run them by themselves).

use macroquad::prelude::*;

use super::hud::world_to_ui;
use super::world_view::sim;
use super::{Ctx, GameScreen, Panel};
use crate::content::handson::{TutAnchor, Voice};
use crate::map::TILE;
use crate::ui::Ui;
use crate::ui::handson::{CoachAction, CoachMark, coach_mark};
use sim::{AgentRef, Command, PartyState, Phase};

/// When a step can show: (id, story night, in the Day phase).
pub const STEPS: &[(&str, u32, bool)] = &[
    // night 1 (solo, rain)
    ("light_window", 1, false),
    ("machine", 1, false),
    ("greet", 1, false),
    ("take_order", 1, false),
    ("pick_recipe", 1, false),
    ("pour_over", 1, false),
    ("quick_brew", 1, false),
    ("tip_coin", 1, false),
    ("clear_table", 1, false),
    ("moment", 1, false),
    ("speed", 1, false),
    ("closing", 1, false),
    ("journal", 1, false),
    // day 2 and night 2
    ("book", 2, true),
    ("experiment", 2, true),
    ("jukebox", 2, false),
    ("wash", 2, false),
    ("mop", 2, false),
    // day 3 and night 3: the first helper
    ("duties", 3, true),
    ("rozpis", 3, true),
    ("rail", 3, false),
    ("cat", 3, false),
    // night 4
    ("usual", 4, false),
    ("atmosphere", 4, false),
    // later one-offs
    ("fairness", 8, true),
    ("garlands", 21, false),
];

/// The steps the night-1 count shows ("TUTORIAL · 3/13").
pub const NIGHT1: &[&str] = &[
    "light_window",
    "machine",
    "greet",
    "take_order",
    "pick_recipe",
    "pour_over",
    "quick_brew",
    "tip_coin",
    "clear_table",
    "moment",
    "speed",
    "closing",
    "journal",
];

/// The last night with a coach mark (the tutorial ends after it).
pub const LAST_NIGHT: u32 = 21;

/// The coach mark on screen now: (step id, `GameScreen::time` it appeared, the event
/// counters at that moment).
#[derive(Debug, Clone, Default)]
pub struct CoachState {
    pub shown: Option<(String, f32, super::hands::TutTally)>,
}

/// The coach marks run (tutorial on, past the first day's cards, nothing modal in the way).
pub fn running(g: &GameScreen) -> bool {
    let Some(p) = g.extra.tutorial.as_ref() else {
        return false;
    };
    p.active
        && !p.coach_skipped
        && p.step as usize >= super::tutorial::FIRST_NIGHT
        && g.world.night() <= LAST_NIGHT
        && !g.cutscene_playing()
        && g.world.playing_cutscene().is_none()
        && g.pause.is_none()
}

/// Is this step's mark on screen (widgets highlight what it talks about)?
pub fn wants(g: &GameScreen, id: &str) -> bool {
    running(g) && g.coach.shown.as_ref().is_some_and(|s| s.0 == id)
}

fn done(g: &GameScreen, id: &str) -> bool {
    g.extra
        .tutorial
        .as_ref()
        .is_some_and(|p| p.coach_done.iter().any(|d| d == id))
}

fn mark_done(g: &mut GameScreen, id: &str) {
    if let Some(p) = &mut g.extra.tutorial
        && !p.coach_done.iter().any(|d| d == id)
    {
        p.coach_done.push(id.to_string());
    }
    g.coach.shown = None;
}

/// Mirek's party leader, if he is in the café.
fn mirek(g: &GameScreen) -> Option<(u32, sim::Party)> {
    let p = g
        .world
        .parties()
        .iter()
        .find(|p| p.regular.as_deref() == Some("mirek") && p.state != PartyState::Leaving)?;
    Some((p.leader()?, p.clone()))
}

/// The first table with a tip coin / with cups.
fn coin_table(g: &GameScreen) -> Option<u32> {
    g.hands
        .coins
        .iter()
        .find(|c| c.coin.collected.is_none())
        .map(|c| c.table)
}

fn dirty_table(g: &GameScreen, content: &crate::content::Content) -> Option<u32> {
    g.world
        .furniture()
        .iter()
        .find(|p| {
            p.dishes > 0
                && content
                    .furniture_def(&p.def)
                    .is_some_and(|d| d.has_tag("table"))
        })
        .map(|p| p.uid)
}

fn furniture_with(g: &GameScreen, content: &crate::content::Content, tag: &str) -> Option<u32> {
    g.world
        .furniture()
        .iter()
        .find(|p| {
            content
                .furniture_def(&p.def)
                .is_some_and(|d| d.has_tag(tag))
        })
        .map(|p| p.uid)
}

fn checklist_open(g: &GameScreen, id: &str) -> bool {
    g.world
        .app()
        .checklist
        .iter()
        .any(|c| c.id == id && !c.done && c.claim.is_none())
}

fn moment_on(g: &GameScreen, id: &str) -> bool {
    g.world.active_moments().iter().any(|m| m.id == id)
}

fn helpers(g: &GameScreen) -> usize {
    g.world.staff().iter().filter(|s| !s.owner).count()
}

/// Should the step appear now?
fn trigger(g: &GameScreen, content: &crate::content::Content, id: &str) -> bool {
    let after = |prev: &str| done(g, prev);
    let night = g.world.phase() == Phase::Night;
    let busy = g.hands.minigame.is_some();
    match id {
        "light_window" => night && !g.world.window_light_on(content),
        "machine" => after("light_window") && night && checklist_open(g, "coffee_warmup"),
        "greet" => night && mirek(g).is_some_and(|(_, p)| !p.greeted),
        "take_order" => after("greet") && night && mirek(g).is_some(),
        "pick_recipe" => after("take_order") && g.hands.order.is_some(),
        "pour_over" => after("pick_recipe") && busy,
        "quick_brew" => g
            .hands
            .order
            .as_ref()
            .and_then(|o| g.world.party_of_guest(o.guest))
            .is_some_and(|p| !p.wish_open),
        "tip_coin" => !busy && coin_table(g).is_some(),
        "clear_table" => !busy && after("tip_coin") && dirty_table(g, content).is_some(),
        "moment" => !busy && night && moment_on(g, "m_fogged_window"),
        "speed" => g.anchors.iter().any(|a| a.0 == "hud.lull"),
        "closing" => !busy && night && g.world.minute() >= 465,
        "journal" => g.summary.is_some() && g.anchors.iter().any(|a| a.0 == "summary.polaroid"),
        "book" => !night,
        "experiment" => after("book") && g.hands.book.is_some(),
        "jukebox" => !busy && night && g.world.minute() >= 10,
        "wash" => !busy && night && g.world.sink_dishes() >= 2,
        "mop" => !busy && night && !g.world.dirt_spots().is_empty(),
        "duties" => !night && helpers(g) > 0,
        "rozpis" => after("duties") && !night,
        "rail" => {
            !busy
                && night
                && g.world
                    .ticket_rail(content)
                    .iter()
                    .any(|t| t.owner == sim::TicketOwner::Nobody)
                && helpers(g) > 0
        }
        "cat" => !busy && night && moment_on(g, "m_station_cat"),
        "usual" => g.anchors.iter().any(|a| a.0 == "order.usual"),
        "atmosphere" => night && g.hands.tut.get("atmo_level") > 0,
        "fairness" => {
            g.panel == Some(Panel::App)
                && g.ui_state.app_tab == 0
                && g.anchors.iter().any(|a| a.0 == "rozpis.wellbeing")
        }
        "garlands" => night && moment_on(g, "m_hang_garlands"),
        _ => false,
    }
}

/// Is the step on screen done? (`since` = counters when it appeared, `age` = seconds shown)
fn finished(
    g: &GameScreen,
    content: &crate::content::Content,
    id: &str,
    since: &super::hands::TutTally,
    age: f32,
) -> bool {
    let more = |what: &str| g.hands.tut.get(what) > since.get(what);
    match id {
        "light_window" => {
            g.world.window_light_on(content)
                || g.world
                    .app()
                    .checklist
                    .iter()
                    .any(|c| c.id == "light_window" && c.claim.is_some())
        }
        "machine" => !checklist_open(g, "coffee_warmup"),
        // optional: a few seconds unanswered is fine too
        "greet" => more("greeted") || mirek(g).is_none_or(|(_, p)| p.greeted) || age > 9.0,
        "take_order" => {
            more("order_taken")
                || mirek(g).is_none_or(|(_, p)| p.order_claim.is_some() || p.ordered_at.is_some())
        }
        "pick_recipe" => more("brew_chosen") || g.hands.order.is_none() && age > 1.0,
        "pour_over" => more("minigame_done") || g.hands.minigame.is_none(),
        "quick_brew" => more("quick_chosen") || g.hands.order.is_none(),
        "tip_coin" => more("tip_collected") || coin_table(g).is_none(),
        "clear_table" => more("table_cleared") || dirty_table(g, content).is_none(),
        "moment" => more("moment_picked") || !moment_on(g, "m_fogged_window"),
        "speed" => age > 8.0 || g.speed >= 2,
        "closing" => {
            more("handover_written")
                || g.world.handover_picks().len() >= 2
                || g.world.phase() != Phase::Night
                || age > 20.0
        }
        "journal" => age > 10.0 || more("polaroid_kept") || g.summary.is_none(),
        "book" => more("book_opened") || g.hands.book.is_some(),
        "experiment" => more("experiment_done") || g.hands.book.is_none(),
        "jukebox" => more("record_changed") || age > 14.0,
        "wash" => more("washed") || g.world.sink_dishes() == 0,
        "mop" => more("mopped") || g.world.dirt_spots().is_empty(),
        "duties" => {
            g.world
                .staff()
                .iter()
                .any(|s| !s.owner && !g.world.staff_duties(s.id).is_empty())
                && (g.panel == Some(Panel::Duties) || age > 3.0)
                || age > 25.0
        }
        "rozpis" => more("published") || g.world.phase() == Phase::Night || age > 30.0,
        "rail" => age > 8.0,
        "cat" => more("moment_picked") || !moment_on(g, "m_station_cat"),
        "usual" => more("usual_chosen") || !g.anchors.iter().any(|a| a.0 == "order.usual"),
        "atmosphere" => age > 8.0,
        "fairness" => age > 10.0 || g.panel != Some(Panel::App),
        "garlands" => more("moment_picked") || !moment_on(g, "m_hang_garlands"),
        _ => true,
    }
}

fn marker_rect(g: &GameScreen, ui: &Ui, id: &str) -> Option<Rect> {
    let m = g.scene.markers.iter().find(|m| m.0 == id)?.1;
    let c = world_to_ui(g, ui, m.orb(g.time));
    Some(Rect::new(c.x - 20.0, c.y - 20.0, 40.0, 40.0))
}

fn furniture_rect(g: &GameScreen, ui: &Ui, uid: u32, lift: f32) -> Option<Rect> {
    let p = g.world.furniture_by_uid(uid)?;
    let c = world_to_ui(
        g,
        ui,
        vec2(p.x as f32 + 0.5, p.y as f32 + 0.5 - lift) * TILE,
    );
    Some(Rect::new(c.x - 28.0, c.y - 30.0, 56.0, 60.0))
}

/// Where the arrow points (UI units).
fn target(
    g: &mut GameScreen,
    ui: &Ui,
    content: &crate::content::Content,
    id: &str,
) -> Option<Rect> {
    let anchor = |g: &GameScreen, name: &str| g.anchors.iter().find(|a| a.0 == name).map(|a| a.1);
    let around = |p: Vec2, w: f32, h: f32| Rect::new(p.x - w / 2.0, p.y - h / 2.0, w, h);
    match id {
        "light_window" => {
            // the window nearest the middle of the front wall
            let grid = g.world.grid();
            let mid = grid.width as f32 / 2.0;
            let mut best: Option<(f32, (i32, i32))> = None;
            for y in 0..grid.height {
                for x in 0..grid.width {
                    if grid
                        .tile(x, y)
                        .is_some_and(|t| t.kind == sim::TileKind::Window)
                    {
                        let d = (x as f32 - mid).abs() + y as f32 * 0.1;
                        if best.is_none_or(|b| d < b.0) {
                            best = Some((d, (x, y)));
                        }
                    }
                }
            }
            let (x, y) = best?.1;
            let c = world_to_ui(g, ui, vec2(x as f32 + 0.5, y as f32 - 0.3) * TILE);
            Some(around(c, 46.0, 50.0))
        }
        "machine" => {
            furniture_with(g, content, "station:coffee").and_then(|u| furniture_rect(g, ui, u, 0.6))
        }
        "greet" | "take_order" => {
            let (leader, _) = mirek(g)?;
            let head = g.scene.head_of(AgentRef::Guest(leader), g.time)?;
            let c = world_to_ui(g, ui, head + vec2(0.0, 30.0));
            Some(around(c, 40.0, 70.0))
        }
        "pick_recipe" => anchor(g, "order.chip.0").or_else(|| anchor(g, "rail")),
        "pour_over" => anchor(g, "minigame"),
        "quick_brew" => anchor(g, "order.mode"),
        "tip_coin" => {
            let table = coin_table(g)?;
            let p = super::hands::table_top(&g.world, content, table)?;
            Some(around(world_to_ui(g, ui, p), 40.0, 34.0))
        }
        "clear_table" => {
            let table = dirty_table(g, content)?;
            let p = super::hands::table_top(&g.world, content, table)?;
            Some(around(world_to_ui(g, ui, p), 56.0, 40.0))
        }
        "moment" => marker_rect(g, ui, "m_fogged_window"),
        "cat" => marker_rect(g, ui, "m_station_cat"),
        "garlands" => marker_rect(g, ui, "m_hang_garlands"),
        "speed" => anchor(g, "hud.lull"),
        "closing" => anchor(g, "app.handover").or_else(|| anchor(g, "dock.app")),
        "journal" => anchor(g, "summary.polaroid"),
        "book" => anchor(g, "dock.book"),
        "experiment" => anchor(g, "book"),
        "jukebox" => {
            furniture_with(g, content, "music").and_then(|u| furniture_rect(g, ui, u, 0.9))
        }
        "wash" => furniture_with(g, content, "dishes").and_then(|u| furniture_rect(g, ui, u, 0.4)),
        "mop" => {
            let t = *g.world.dirt_spots().first()?;
            let c = world_to_ui(g, ui, vec2(t.0 as f32 + 0.5, t.1 as f32 + 0.5) * TILE);
            Some(around(c, 52.0, 34.0))
        }
        "duties" => anchor(g, "panel")
            .filter(|_| g.panel == Some(Panel::Duties))
            .or_else(|| anchor(g, "dock.duties")),
        "rozpis" => anchor(g, "rozpis.prefs").or_else(|| anchor(g, "dock.app")),
        "rail" => anchor(g, "rail"),
        "usual" => anchor(g, "order.usual"),
        "atmosphere" => anchor(g, "hud.atmosphere"),
        "fairness" => anchor(g, "rozpis.wellbeing"),
        _ => None,
    }
}

/// Logic + drawing of the coach marks (call after the HUD, the overlays and the summary).
pub fn draw(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    if !running(g) {
        g.coach.shown = None;
        return;
    }
    let content = cx.content;
    let loc = cx.loc;
    let night = g.world.night();
    let day = g.world.phase() == Phase::Day;
    // the mark on screen: done?
    if let Some((id, at, since)) = g.coach.shown.clone()
        && finished(g, content, &id, &since, g.time - at)
    {
        mark_done(g, &id);
    }
    // only the journal step may sit over the morning summary
    if g.summary.is_some() && g.coach.shown.as_ref().is_some_and(|s| s.0 != "journal") {
        g.coach.shown = None;
    }
    // the next one (tonight's or today's steps, in order; a later night skips the rest)
    if g.coach.shown.is_none() {
        let night_of_summary = if g.summary.is_some() {
            g.summary.as_ref().map_or(night, |s| s.night)
        } else {
            night
        };
        let next = STEPS
            .iter()
            .filter(|(id, n, d)| {
                !done(g, id)
                    && if *id == "journal" {
                        *n == night_of_summary && g.summary.is_some()
                    } else {
                        // one-offs (fairness, garlands) wait for their moment once reached
                        (if *n >= 8 { night >= *n } else { night == *n })
                            && *d == day
                            && g.summary.is_none()
                    }
            })
            .find(|(id, _, _)| trigger(g, content, id))
            .map(|s| s.0);
        if let Some(id) = next {
            // a step already satisfied just counts as learned
            let snap = g.hands.tut.clone();
            let optional = matches!(
                id,
                "speed" | "greet" | "rail" | "atmosphere" | "fairness" | "journal"
            );
            if !optional && finished(g, content, id, &snap, 0.0) {
                mark_done(g, id);
                return;
            }
            g.coach.shown = Some((id.to_string(), g.time, snap));
            // the sim runs the step's `on_show` (a scripted visit, a forced moment) once
            let _ = g
                .world
                .apply(content, Command::TutorialShown { id: id.to_string() });
        }
    }
    let Some((id, _, _)) = g.coach.shown.clone() else {
        // every step is learned (or we are past them): the tutorial is over
        if night > LAST_NIGHT || STEPS.iter().all(|s| done(g, s.0)) {
            g.finish_tutorial(cx);
        }
        return;
    };
    let Some(step) = content.handson.tutorial_step(&id) else {
        mark_done(g, &id);
        return;
    };
    // "TUTORIAL · 3/13" on the first night; later nights count their own steps
    let here: Vec<&str> = STEPS
        .iter()
        .filter(|(_, n, _)| *n == step.night)
        .map(|s| s.0)
        .collect();
    let n = here.iter().position(|s| *s == id).unwrap_or(0) + 1;
    let tgt = target(g, ui, content, &id);
    let ring = !matches!(step.anchor, TutAnchor::Free) && tgt.is_some();
    let mark = CoachMark {
        label: format!(
            "{} · {n}/{}",
            loc.tr("ui.tutorial.label").to_uppercase(),
            here.len()
        ),
        title: None,
        body: loc.tr_owned(&step.key()),
        lida: step.voice == Voice::Lida,
        target: tgt,
        next: loc.tr_owned("ui.tutorial.got_it"),
        skip: Some(loc.tr_owned("ui.tutorial.skip")),
        ring: ring && id != "pour_over" && id != "experiment",
    };
    let avoid: Vec<Rect> = [
        "hud.clock",
        "hud.money",
        "hud.ticker",
        "hud.tipjar",
        "hud.atmosphere",
        "rail",
        "order.card",
        "moment.card",
        "minigame",
        "book",
    ]
    .iter()
    .filter_map(|a| g.anchors.iter().find(|x| x.0 == *a).map(|x| x.1))
    // the dock's buttons stay usable (the one it points at has the ring)
    .chain(
        g.anchors
            .iter()
            .filter(|a| a.0.starts_with("dock."))
            .filter(|a| Some(a.1) != tgt)
            .map(|a| a.1),
    )
    .collect();
    match coach_mark(ui, ("coach", id.as_str()), &mark, &avoid, false) {
        Some(CoachAction::Next) => mark_done(g, &id),
        Some(CoachAction::Skip) => {
            if let Some(p) = &mut g.extra.tutorial {
                p.coach_skipped = true;
            }
            g.coach.shown = None;
            // no more scripted tutorial visits either
            let _ = g.world.apply(content, Command::SetTutorial { on: false });
            g.finish_tutorial(cx);
        }
        None => {}
    }
}
