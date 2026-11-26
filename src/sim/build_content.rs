//! Reasons to build (design/v0.2.md package C): the shared state of requests, corners,
//! grandma's attic and the back room, the sim hooks (day start, night start, each minute,
//! 06:00, after a furniture change) and the before / after polaroid.
//!
//! The pieces live in [`super::build_wishes`], [`super::corners`], [`super::attic`] and
//! [`super::backroom`]. Nothing here is required for the story: a player who never builds
//! still reaches «Light in the Window» (tested).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::grid;
use super::systems::{Highlight, SceneSnapData, SnapTileData};
use super::types::*;
use super::world::*;
use crate::content::Content;
use crate::content::handson::{CozyComponent, HighlightKind};

/// A request's progress (`build_wishes.ron`). Missing = not shown yet.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct WishProgress {
    /// The night it first showed.
    pub opened: u32,
    /// They said it out loud (their bubble) at least once.
    pub asked: bool,
    /// They saw it built and reacted (the reward ran) — on that night.
    pub thanked: Option<u32>,
    /// The helper it belongs to (`WishWho::Staff`), fixed when it opened.
    pub staff: Option<u32>,
}

/// The back room once reopened: the night, its interior (inclusive tile bounds) and the
/// doorway tiles cut into the old right wall.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BackRoomState {
    pub night: u32,
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
    pub door: Vec<(i32, i32)>,
}

impl BackRoomState {
    pub fn contains(&self, t: (i32, i32)) -> bool {
        (t.0 >= self.x0 && t.0 <= self.x1 && t.1 >= self.y0 && t.1 <= self.y1)
            || self.door.contains(&t)
    }
    pub fn center(&self) -> (i32, i32) {
        ((self.x0 + self.x1) / 2, (self.y0 + self.y1) / 2)
    }
}

/// The hall as it was at a night start (the "before" of the next night's comparison).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HallMemory {
    pub night: u32,
    /// Light + beauty + comfort + corners (the furniture's part of the coziness, day light).
    pub score: f32,
    pub corners: BTreeSet<String>,
    pub uids: BTreeSet<u32>,
    pub back_room: bool,
    /// The whole interior (cropped to the "after" window later).
    pub snap: SceneSnapData,
}

/// Tonight's before / after: why, where, and the photo once taken.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BeforeAfterPlan {
    /// `@`-prefixed i18n key of what changed (`@corner.reading_nook.name`, …).
    pub what: String,
    pub focus: (i32, i32),
    pub before: SceneSnapData,
    pub taken: Option<Highlight>,
}

/// Package C's part of the world (`World.build`, every field defaulted: old saves load).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BuildState {
    pub wishes: BTreeMap<String, WishProgress>,
    /// Grandma's attic: this week's offer, the pieces bought (placed or back upstairs), the
    /// night of the last restock.
    pub attic_offer: Vec<String>,
    pub attic_owned: BTreeSet<String>,
    pub attic_restocked: u32,
    pub back_room: Option<BackRoomState>,
    /// Corner instances seen (`<id>:<anchor uid>`) — a new one emits `CornerFormed`.
    pub corners_seen: BTreeSet<String>,
    /// Corner kinds ever formed.
    pub corners_ever: BTreeSet<String>,
    pub memory: Option<HallMemory>,
    pub before_after: Option<BeforeAfterPlan>,
}

// ---------------------------------------------------------------------------------------------
// Hooks (called from `world.rs`)
// ---------------------------------------------------------------------------------------------

pub(crate) fn on_day_start(w: &mut World, content: &Content) {
    super::attic::on_day_start(w, content);
    super::build_wishes::refresh(w, content);
    super::corners::on_furniture_changed(w, content);
}

pub(crate) fn on_night_start(w: &mut World, content: &Content) {
    super::build_wishes::refresh(w, content);
    super::corners::on_furniture_changed(w, content);
    plan_before_after(w, content);
}

pub(crate) fn on_minute(w: &mut World, content: &Content) {
    super::build_wishes::on_minute(w, content);
    let at = content
        .build
        .corners
        .before_after
        .after_at
        .minutes_since_open();
    if at >= 0 && w.minute() == at as u32 {
        take_after(w, content);
    }
}

/// 06:00, before the journal hands the candidates to the morning: the before / after joins
/// them (always — it replaces the weakest if the list is full).
pub(crate) fn on_night_end(w: &mut World, content: &Content) {
    if w.build
        .before_after
        .as_ref()
        .is_some_and(|p| p.taken.is_none())
    {
        take_after(w, content);
    }
    let Some(h) = w.build.before_after.take().and_then(|p| p.taken) else {
        return;
    };
    let keep = content.handson.journal.keep_per_night.max(1) as usize;
    let list = &mut w.sys.journal.tonight;
    let pos = list
        .iter()
        .position(|x| x.score < h.score)
        .unwrap_or(list.len());
    if pos < keep {
        list.insert(pos, h);
        list.truncate(keep);
    } else {
        list.truncate(keep - 1);
        list.push(h);
    }
}

/// After a furniture command (place / move / rotate / remove / undo): corners and requests.
pub(crate) fn after_furniture_change(w: &mut World, content: &Content) {
    super::corners::on_furniture_changed(w, content);
    super::build_wishes::refresh(w, content);
}

// ---------------------------------------------------------------------------------------------
// Before / after (design/v0.2.md item 10)
// ---------------------------------------------------------------------------------------------

/// The furniture's part of the coziness (light + beauty + comfort in day light, plus the
/// corners): changes only when the player builds.
pub(crate) fn hall_score(w: &World, content: &Content) -> f32 {
    let mut c = w.clone();
    c.phase = Phase::Day;
    c.dirt = 0.0;
    c.events.clear();
    let coz = super::atmos::compute(&c, content);
    let part = |k: CozyComponent| {
        CozyComponent::ALL
            .iter()
            .position(|x| *x == k)
            .map(|i| coz.parts[i])
            .unwrap_or(0.0)
    };
    part(CozyComponent::Light)
        + part(CozyComponent::Beauty)
        + part(CozyComponent::Comfort)
        + super::corners::coziness_bonus(w, content)
}

fn interior_size(w: &World) -> ((i32, i32), (i32, i32)) {
    let (mut x0, mut x1, mut y1) = (i32::MAX, i32::MIN, i32::MIN);
    for y in 0..w.grid.height {
        for x in 0..w.grid.width {
            if w.grid
                .tile(x, y)
                .is_some_and(|t| t.kind != grid::TileKind::Street)
            {
                x0 = x0.min(x);
                x1 = x1.max(x);
                y1 = y1.max(y);
            }
        }
    }
    if x0 > x1 {
        return ((0, 0), (w.grid.width, w.grid.height));
    }
    ((x0, 0), (x1 - x0 + 1, y1 + 1))
}

fn memory_now(w: &World, content: &Content) -> HallMemory {
    let ((x0, _), (sw, sh)) = interior_size(w);
    let focus = (x0 + sw / 2, sh / 2);
    HallMemory {
        night: w.night,
        score: hall_score(w, content),
        corners: super::corners::detect(w, content)
            .into_iter()
            .map(|c| c.key())
            .collect(),
        uids: w.furniture.iter().map(|p| p.uid).collect(),
        back_room: w.build.back_room.is_some(),
        snap: super::journal::snap_sized(w, content, Some(focus), Some((sw, sh))),
    }
}

/// Night start: compare with the hall at the last night start; plan tonight's after photo.
fn plan_before_after(w: &mut World, content: &Content) {
    let now = memory_now(w, content);
    w.build.before_after = None;
    if let Some(prev) = w.build.memory.take() {
        let ba = &content.build.corners.before_after;
        let corners = super::corners::detect(w, content);
        let new_corner = corners.iter().find(|c| !prev.corners.contains(&c.key()));
        let plan = if now.back_room && !prev.back_room {
            w.build
                .back_room
                .as_ref()
                .map(|b| ("@build.before_after.back_room".to_string(), b.center()))
        } else if let Some(c) = new_corner {
            Some((format!("@corner.{}.what", c.id), c.center))
        } else if now.score - prev.score >= ba.min_gain {
            let fresh: Vec<(i32, i32)> = w
                .furniture
                .iter()
                .filter(|p| !prev.uids.contains(&p.uid))
                .map(|p| (p.x, p.y))
                .collect();
            let focus = if fresh.is_empty() {
                (
                    now.snap.origin.0 + now.snap.size.0 as i32 / 2,
                    now.snap.size.1 as i32 / 2,
                )
            } else {
                let n = fresh.len() as i32;
                (
                    fresh.iter().map(|t| t.0).sum::<i32>() / n,
                    fresh.iter().map(|t| t.1).sum::<i32>() / n,
                )
            };
            Some(("@build.before_after.cozier".to_string(), focus))
        } else {
            None
        };
        if let Some((what, focus)) = plan {
            w.build.before_after = Some(BeforeAfterPlan {
                what,
                focus,
                before: prev.snap,
                taken: None,
            });
        }
    }
    w.build.memory = Some(now);
}

/// The "after": tonight's scene around the change, and the remembered hall cropped to the
/// same window.
fn take_after(w: &mut World, content: &Content) {
    let Some(plan) = w.build.before_after.as_ref() else {
        return;
    };
    if plan.taken.is_some() || w.phase != Phase::Night {
        return;
    }
    let focus = plan.focus;
    let what = plan.what.clone();
    let after = super::journal::snap(w, content, Some(focus));
    let mut before = crop(&plan.before, after.origin, after.size);
    before.focus = after.focus;
    before.zoom = after.zoom;
    let score = content
        .handson
        .journal
        .rule(HighlightKind::BeforeAfter)
        .map(|r| r.score)
        .unwrap_or(50);
    let h = Highlight {
        night: w.night,
        minute: w.minute(),
        kind: HighlightKind::BeforeAfter,
        score,
        caption_key: HighlightKind::BeforeAfter.caption_key(None),
        args: vec![("what".into(), what)],
        snap: after,
        before: Some(before),
    };
    if let Some(p) = w.build.before_after.as_mut() {
        p.taken = Some(h);
    }
    w.emit(SimEvent::HighlightCaptured {
        kind: HighlightKind::BeforeAfter,
        score,
    });
}

/// A window of a bigger snapshot (tiles outside it are void).
pub(crate) fn crop(full: &SceneSnapData, origin: (i32, i32), size: (u8, u8)) -> SceneSnapData {
    let (fx, fy) = full.origin;
    let (fw, fh) = (full.size.0 as i32, full.size.1 as i32);
    let (sw, sh) = (size.0 as i32, size.1 as i32);
    let inside = |t: (i32, i32)| {
        t.0 >= origin.0 && t.0 < origin.0 + sw && t.1 >= origin.1 && t.1 < origin.1 + sh
    };
    let mut tiles = Vec::with_capacity((sw * sh) as usize);
    for y in origin.1..origin.1 + sh {
        for x in origin.0..origin.0 + sw {
            let (lx, ly) = (x - fx, y - fy);
            let t = if lx >= 0 && ly >= 0 && lx < fw && ly < fh {
                full.tiles
                    .get((ly * fw + lx) as usize)
                    .cloned()
                    .unwrap_or(SnapTileData::Void)
            } else {
                SnapTileData::Void
            };
            tiles.push(t);
        }
    }
    let props = full
        .props
        .iter()
        .filter(|p| {
            (0..p.size.0 as i32)
                .any(|dx| (0..p.size.1 as i32).any(|dy| inside((p.tile.0 + dx, p.tile.1 + dy))))
        })
        .cloned()
        .collect();
    let actors = full
        .actors
        .iter()
        .filter(|a| inside(grid::px_to_tile(a.feet)))
        .cloned()
        .collect();
    let cups = full
        .cups
        .iter()
        .filter(|c| inside(grid::px_to_tile(c.pos)))
        .cloned()
        .collect();
    SceneSnapData {
        origin,
        size,
        tiles,
        props,
        actors,
        cups,
        cat: full.cat.filter(|c| inside(grid::px_to_tile(*c))),
        ..full.clone()
    }
}

impl World {
    /// Package C's raw state (requests' progress, the attic, the back room, corners seen).
    pub fn build_state(&self) -> &BuildState {
        &self.build
    }
}
