//! Karolina's coach marks for her first night (design/handson.md §8, `tutorial.ron`):
//! glass tips with an arrow, in her own words or as one of Lida's notes. They never block
//! input or hold the clock (the order card and the mini-game already slow it down), each
//! has "Got it", and "Skip tutorial" ends them for good.
//!
//! Texts, anchors and voices come from `tutorial.ron`; when a step appears and when it is
//! done is decided here from the live game (the sim has no tutorial hooks yet — see the
//! T7 report: `SpawnRegular` / `SpawnMoment` `on_show` effects need a sim command). Steps
//! that need T8 systems (the machine ritual, moments, the closing note, the journal) wait.

use macroquad::prelude::*;

use super::hud::world_to_ui;
use super::world_view::sim;
use super::{Ctx, GameScreen};
use crate::content::handson::{TutAnchor, Voice};
use crate::map::TILE;
use crate::ui::Ui;
use crate::ui::handson::{CoachAction, CoachMark, coach_mark};
use sim::{AgentRef, PartyState, Phase};

/// Night-1 steps this build can teach, in `tutorial.ron` order.
pub const NIGHT1: &[&str] = &[
    "light_window",
    "greet",
    "take_order",
    "pick_recipe",
    "pour_over",
    "quick_brew",
    "tip_coin",
    "clear_table",
    "speed",
];

/// The coach mark on screen now: (step id, `GameScreen::time` it appeared, the event
/// counters at that moment).
#[derive(Debug, Clone, Default)]
pub struct CoachState {
    pub shown: Option<(String, f32, super::hands::TutTally)>,
}

/// The night coach marks run (first night, tutorial on, nothing modal in the way).
pub fn running(g: &GameScreen) -> bool {
    let Some(p) = g.extra.tutorial.as_ref() else {
        return false;
    };
    p.active
        && !p.coach_skipped
        && p.step as usize >= super::tutorial::FIRST_NIGHT
        && g.world.night() == 1
        && g.world.phase() == Phase::Night
        && !g.cutscene_playing()
        && g.world.playing_cutscene().is_none()
        && g.summary.is_none()
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

/// Should the step appear now?
fn trigger(g: &GameScreen, content: &crate::content::Content, id: &str) -> bool {
    let after = |prev: &str| done(g, prev);
    match id {
        "light_window" => !g.world.window_light_on(content),
        "greet" => mirek(g).is_some_and(|(_, p)| !p.greeted),
        "take_order" => after("greet") && mirek(g).is_some(),
        "pick_recipe" => after("take_order") && g.hands.order.is_some(),
        "pour_over" => after("pick_recipe") && g.hands.minigame.is_some(),
        "quick_brew" => g
            .hands
            .order
            .as_ref()
            .and_then(|o| g.world.party_of_guest(o.guest))
            .is_some_and(|p| !p.wish_open),
        "tip_coin" => coin_table(g).is_some(),
        "clear_table" => after("tip_coin") && dirty_table(g, content).is_some(),
        "speed" => g.anchors.iter().any(|a| a.0 == "hud.lull"),
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
        "speed" => age > 8.0 || g.speed >= 2,
        _ => true,
    }
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
        "speed" => anchor(g, "hud.lull"),
        _ => None,
    }
}

/// Logic + drawing of the night coach marks (call after the HUD and the overlays).
pub fn draw(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    if !running(g) {
        g.coach.shown = None;
        return;
    }
    let content = cx.content;
    let loc = cx.loc;
    // the mark on screen: done?
    if let Some((id, at, since)) = g.coach.shown.clone()
        && finished(g, content, &id, &since, g.time - at)
    {
        mark_done(g, &id);
    }
    // the next one
    if g.coach.shown.is_none() {
        let next = NIGHT1
            .iter()
            .find(|id| !done(g, id) && trigger(g, content, id))
            .copied();
        if let Some(id) = next {
            // a step already satisfied just counts as learned
            let snap = g.hands.tut.clone();
            if finished(g, content, id, &snap, 0.0) && id != "speed" && id != "greet" {
                mark_done(g, id);
                return;
            }
            g.coach.shown = Some((id.to_string(), g.time, snap));
        }
    }
    let Some((id, _, _)) = g.coach.shown.clone() else {
        // every step is learned: the tutorial is over
        if NIGHT1.iter().all(|s| done(g, s)) {
            g.finish_tutorial(cx);
        }
        return;
    };
    let Some(step) = content.handson.tutorial_step(&id) else {
        mark_done(g, &id);
        return;
    };
    let n = NIGHT1.iter().position(|s| *s == id).unwrap_or(0) + 1;
    let tgt = target(g, ui, content, &id);
    let ring = !matches!(step.anchor, TutAnchor::Free) && tgt.is_some();
    let mark = CoachMark {
        label: format!(
            "{} · {n}/{}",
            loc.tr("ui.tutorial.label").to_uppercase(),
            NIGHT1.len()
        ),
        title: None,
        body: loc.tr_owned(&step.key()),
        lida: step.voice == Voice::Lida,
        target: tgt,
        next: loc.tr_owned("ui.tutorial.got_it"),
        skip: Some(loc.tr_owned("ui.tutorial.skip")),
        ring: ring && id != "pour_over",
    };
    let avoid: Vec<Rect> = [
        "hud.clock",
        "hud.money",
        "hud.ticker",
        "hud.tipjar",
        "rail",
        "order.card",
        "minigame",
        "book",
    ]
    .iter()
    .filter_map(|a| g.anchors.iter().find(|x| x.0 == *a).map(|x| x.1))
    .collect();
    match coach_mark(ui, ("coach", id.as_str()), &mark, &avoid, false) {
        Some(CoachAction::Next) => mark_done(g, &id),
        Some(CoachAction::Skip) => {
            if let Some(p) = &mut g.extra.tutorial {
                p.coach_skipped = true;
            }
            g.coach.shown = None;
            g.finish_tutorial(cx);
        }
        None => {}
    }
}
