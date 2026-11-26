//! Grandma's back room (design/v0.2.md item 9): a late-game goal — pay once to reopen the
//! small second hall behind the right wall (more seats, more floor to decorate). Optional and
//! story-safe: available from `attic.ron` → `back_room.from_night` (15), priced so a Cozy
//! player affords it by night 20.
//!
//! Reopening widens the grid by [`BACK_COLS`] columns to the right, so every existing tile
//! keeps its coordinates (cutscene spots, saves, the jukebox tile); a doorway is cut into the
//! old right wall in the hall part, and grandma's old tables wait inside.

use super::build_content::BackRoomState;
use super::grid::{Floor, Grid, Placed, Room, TileInfo, TileKind};
use super::types::*;
use super::world::*;
use crate::content::Content;

/// Columns the back room adds: 5 of floor + the new outer wall.
pub const BACK_COLS: i32 = 6;

/// What the build / attic panel shows about the back room.
#[derive(Debug, Clone, PartialEq)]
pub struct BackRoomView {
    pub open: bool,
    /// It can be reopened now (night reached; Day phase is checked by the command).
    pub available: bool,
    pub from_night: u32,
    pub price: u32,
    /// Floor tiles inside (and the doorway) once open.
    pub tiles: Vec<(i32, i32)>,
}

fn from_night(w: &World, content: &Content) -> u32 {
    let b = &content.build.attic.back_room;
    match w.mode {
        Mode::Story => b.from_night,
        Mode::Sandbox => b.sandbox_night,
    }
}

/// Builds the widened grid: the old tiles at the same coordinates, the back room to the
/// right of the old right wall, the street continued below, a doorway in the hall part.
fn widen(w: &mut World, content: &Content) -> Option<BackRoomState> {
    let old = &w.grid;
    let (ow, h) = (old.width, old.height);
    let nw = ow + BACK_COLS;
    // the café's last indoor row (the front wall) = the row above the first street row
    let street_from = (0..h)
        .find(|&y| {
            old.tile(ow - 1, y)
                .is_some_and(|t| t.kind == TileKind::Street)
        })
        .unwrap_or(h);
    let front = street_from - 1;
    if front < 4 {
        return None;
    }
    // the doorway: two rows of the right wall next to free hall floor, near the top
    let free_hall = |y: i32| {
        old.tile(ow - 1, y)
            .is_some_and(|t| t.kind == TileKind::Wall)
            && old
                .tile(ow - 2, y)
                .is_some_and(|t| t.kind == TileKind::Floor && t.room == Room::Hall)
            && old.walkable(ow - 2, y)
    };
    let mut door: Vec<(i32, i32)> = Vec::new();
    for y in [3, 2, 4, 5, 6, 1, 7, 8] {
        if y + 1 < front && free_hall(y) && free_hall(y + 1) {
            door = vec![(ow - 1, y), (ow - 1, y + 1)];
            break;
        }
    }
    if door.is_empty()
        && let Some(y) = (1..front).find(|&y| free_hall(y))
    {
        door = vec![(ow - 1, y)];
    }
    if door.is_empty() {
        return None;
    }
    let n = (nw * h) as usize;
    let mut g: Grid = old.clone();
    g.width = nw;
    g.height = h;
    g.tiles = Vec::with_capacity(n);
    g.objects = vec![None; n];
    g.rugs = vec![None; n];
    g.blocked = vec![false; n];
    let wall = TileInfo {
        kind: TileKind::Wall,
        floor: Floor::Wood,
        room: Room::None,
    };
    let floor = TileInfo {
        kind: TileKind::Floor,
        floor: Floor::Wood,
        room: Room::Hall,
    };
    for y in 0..h {
        for x in 0..nw {
            let t = if x < ow {
                let i = (y * ow + x) as usize;
                let ni = (y * nw + x) as usize;
                g.objects[ni] = old.objects[i];
                g.rugs[ni] = old.rugs[i];
                g.blocked[ni] = old.blocked[i];
                if door.contains(&(x, y)) {
                    floor
                } else {
                    old.tiles[i]
                }
            } else {
                let dx = x - ow;
                if y >= street_from {
                    old.tile(ow - 1, y).unwrap_or(wall)
                } else if y == 0 {
                    if dx == 2 || dx == 3 {
                        TileInfo {
                            kind: TileKind::Window,
                            ..wall
                        }
                    } else {
                        wall
                    }
                } else if y == front || dx == BACK_COLS - 1 {
                    wall
                } else {
                    floor
                }
            };
            g.tiles.push(t);
        }
    }
    g.version = old.version.wrapping_add(1);
    w.grid = g;
    w.grid.rebuild(&w.furniture, content);
    Some(BackRoomState {
        night: w.night,
        x0: ow,
        y0: 1,
        x1: ow + BACK_COLS - 2,
        y1: front - 1,
        door,
    })
}

/// Grandma's old pieces waiting inside (free; skipped where they don't fit).
fn furnish(w: &mut World, content: &Content, room: &BackRoomState) {
    for (id, x, y, rot) in content.build.attic.back_room.pieces.clone() {
        let Some(d) = content.furniture_def(&id) else {
            continue;
        };
        if !room.contains((x, y)) {
            continue;
        }
        let uid = w.alloc_id();
        let piece = Placed {
            uid,
            def: id.clone(),
            x,
            y,
            rot: rot % 4,
            on: d.has_tag("light") && !d.has_tag("music"),
            dirt: 0.0,
            dishes: 0,
            wear: 0.0,
            seat_by: vec![None; d.seats as usize],
            user: None,
            claim: None,
            paid: 0,
        };
        let _ = super::commands::place_piece(w, content, piece);
    }
}

/// `Command::OpenBackRoom` (Day only, once).
pub(crate) fn open(w: &mut World, content: &Content) -> Result<(), CommandError> {
    if w.build.back_room.is_some() {
        return Err(CommandError::AlreadyPlaced);
    }
    if w.phase != Phase::Day {
        return Err(CommandError::WrongPhase);
    }
    if w.night < from_night(w, content) {
        return Err(CommandError::NotUnlocked);
    }
    let price = content.build.attic.back_room.price as i64;
    if w.econ.money < price {
        return Err(CommandError::NotEnoughMoney {
            need: price,
            have: w.econ.money,
        });
    }
    let room = widen(w, content)
        .ok_or_else(|| CommandError::Invalid("no wall to open the back room".into()))?;
    w.econ.money -= price;
    furnish(w, content, &room);
    // the undo stack's tiles are still valid, but a widened café is not undoable
    w.undo.clear();
    let tiles = tiles_of(w, &room);
    w.build.back_room = Some(room);
    w.emit(SimEvent::BackRoomOpened { price, tiles });
    w.emit(SimEvent::Toast {
        key: "backroom.toast.opened".into(),
    });
    super::build_content::after_furniture_change(w, content);
    super::atmos::recompute(w, content);
    Ok(())
}

fn tiles_of(w: &World, room: &BackRoomState) -> Vec<(i32, i32)> {
    let mut v = Vec::new();
    for y in room.y0..=room.y1 {
        for x in room.x0..=room.x1 {
            if w.grid.is_indoor_floor(x, y) {
                v.push((x, y));
            }
        }
    }
    v.extend(room.door.iter().copied());
    v
}

/// Floor tiles of the back room (they don't dilute the hall's beauty: it's extra floor).
pub(crate) fn floor_tiles(w: &World) -> usize {
    w.build
        .back_room
        .as_ref()
        .map(|r| tiles_of(w, r).len())
        .unwrap_or(0)
}

impl World {
    /// Grandma's back room: open?, can it be reopened now, from which night, the price, its
    /// tiles once open.
    pub fn back_room(&self, content: &Content) -> BackRoomView {
        let b = &content.build.attic.back_room;
        let from = from_night(self, content);
        BackRoomView {
            open: self.build.back_room.is_some(),
            available: self.build.back_room.is_none() && self.night >= from,
            from_night: from,
            price: b.price,
            tiles: self
                .build
                .back_room
                .as_ref()
                .map(|r| tiles_of(self, r))
                .unwrap_or_default(),
        }
    }
}
