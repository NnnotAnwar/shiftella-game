//! Build-mode helpers (design/v0.2.md §A): a table moves together with its chairs as one undo
//! step, the coziness a held piece would make, and why a piece can't be moved right now.
//! Pure queries on [`World`] plus [`move_pieces`] behind `Command::MoveGroup`.

use serde::{Deserialize, Serialize};

use super::grid::{self, Placed};
use super::types::CommandError;
use super::world::World;
use crate::content::{BuildCategory, Content};

/// One piece of a group move: its new top-left tile and rotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PieceMove {
    pub uid: u32,
    pub x: i32,
    pub y: i32,
    pub rot: u8,
}

/// Why a piece can't be moved or removed right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InUseReason {
    /// A guest sits on it (or at a chair of the table).
    Seated,
    /// A helper or Karolina is working at it (station, register, sink) or clearing it.
    Busy,
}

/// Moves every piece of `moves` at once: all are lifted, then put down at their new spots, so
/// a table and its chairs may swap tiles. Nothing changes on an error. Returns where each piece
/// stood before (the undo data).
pub(crate) fn move_pieces(
    w: &mut World,
    content: &Content,
    moves: &[PieceMove],
) -> Result<Vec<PieceMove>, CommandError> {
    if moves.is_empty() {
        return Err(CommandError::Invalid("nothing to move".into()));
    }
    let mut old = Vec::with_capacity(moves.len());
    for m in moves {
        let p = w
            .furniture_by_uid(m.uid)
            .ok_or_else(|| CommandError::UnknownId(m.uid.to_string()))?;
        if super::commands::in_use(w, p) {
            return Err(CommandError::InUse);
        }
        if old.iter().any(|o: &PieceMove| o.uid == m.uid) {
            return Err(CommandError::Invalid("a piece twice in one move".into()));
        }
        old.push(PieceMove {
            uid: p.uid,
            x: p.x,
            y: p.y,
            rot: p.rot,
        });
    }
    let saved_furniture = w.furniture.clone();
    let saved_objects = w.grid.objects.clone();
    let saved_rugs = w.grid.rugs.clone();
    let restore = |w: &mut World| {
        w.furniture = saved_furniture.clone();
        w.grid.objects = saved_objects.clone();
        w.grid.rugs = saved_rugs.clone();
        w.grid.rebuild(&w.furniture, content);
    };
    // lift the whole group, re-mark what stays (rug-layer pieces may share tiles)
    let ids: Vec<u32> = moves.iter().map(|m| m.uid).collect();
    for p in w.furniture.clone() {
        if ids.contains(&p.uid)
            && let Some(d) = content.furniture_def(&p.def)
        {
            super::commands::occupy(w, d, &p, false);
        }
    }
    for p in w.furniture.clone() {
        if !ids.contains(&p.uid)
            && let Some(d) = content.furniture_def(&p.def)
        {
            super::commands::occupy(w, d, &p, true);
        }
    }
    // put each one down; members already placed block the later ones
    for m in moves {
        let Some(i) = w.furniture.iter().position(|p| p.uid == m.uid) else {
            restore(w);
            return Err(CommandError::UnknownId(m.uid.to_string()));
        };
        let d = match super::commands::def(content, &w.furniture[i].def) {
            Ok(d) => d,
            Err(e) => {
                restore(w);
                return Err(e);
            }
        };
        let rot = m.rot % 4;
        if let Err(e) = grid::can_place(&w.grid, d, m.x, m.y, rot, None) {
            restore(w);
            return Err(e);
        }
        let mut p = w.furniture[i].clone();
        p.x = m.x;
        p.y = m.y;
        p.rot = rot;
        super::commands::occupy(w, d, &p, true);
        w.furniture[i] = p;
    }
    w.grid.rebuild(&w.furniture, content);
    if !super::commands::connectivity_ok(w, content) {
        restore(w);
        return Err(CommandError::BlocksPath);
    }
    Ok(old)
}

/// Doubled footprint centre (integer maths for half tiles).
fn center2(x: i32, y: i32, size: (u32, u32)) -> (i32, i32) {
    (2 * x + size.0 as i32, 2 * y + size.1 as i32)
}

/// A quarter turn in the direction of `rot + 1` (Down → Left → Up → Right) on screen axes.
fn turn((dx, dy): (i32, i32), turns: u8) -> (i32, i32) {
    let mut v = (dx, dy);
    for _ in 0..turns % 4 {
        v = (-v.1, v.0);
    }
    v
}

impl World {
    /// The pieces that move together with `uid`: a table brings its chairs (seat pieces whose
    /// every seat belongs to this table — within one tile, facing it). `uid` comes first;
    /// anything else is just itself.
    pub fn move_group(&self, content: &Content, uid: u32) -> Vec<u32> {
        let mut out = vec![uid];
        let is_table = self
            .furniture_by_uid(uid)
            .and_then(|p| content.furniture_def(&p.def))
            .is_some_and(|d| d.has_tag("table"));
        if !is_table {
            return out;
        }
        let seats = super::guests::seats(self, content);
        for p in &self.furniture {
            if p.uid == uid
                || content
                    .furniture_def(&p.def)
                    .is_none_or(|d| d.has_tag("table") || d.seats == 0)
            {
                continue;
            }
            let mine: Vec<_> = seats.iter().filter(|s| s.uid == p.uid).collect();
            if !mine.is_empty() && mine.iter().all(|s| s.table == Some(uid)) {
                out.push(p.uid);
            }
        }
        out
    }

    /// Where the group of `uid` lands when its first piece's top-left goes to `to` and the
    /// whole group turns `turns` quarter turns around that piece (R while dragging).
    pub fn group_moves(
        &self,
        content: &Content,
        uid: u32,
        to: (i32, i32),
        turns: u8,
    ) -> Vec<PieceMove> {
        let Some(anchor) = self.furniture_by_uid(uid) else {
            return Vec::new();
        };
        let Some(ad) = content.furniture_def(&anchor.def) else {
            return Vec::new();
        };
        let a_rot = (anchor.rot + turns) % 4;
        let a_c0 = center2(anchor.x, anchor.y, grid::rotated_size(ad, anchor.rot));
        let a_c1 = center2(to.0, to.1, grid::rotated_size(ad, a_rot));
        let mut out = vec![PieceMove {
            uid,
            x: to.0,
            y: to.1,
            rot: a_rot,
        }];
        for m in self.move_group(content, uid).into_iter().skip(1) {
            let Some(p) = self.furniture_by_uid(m) else {
                continue;
            };
            let Some(d) = content.furniture_def(&p.def) else {
                continue;
            };
            let c = center2(p.x, p.y, grid::rotated_size(d, p.rot));
            let off = turn((c.0 - a_c0.0, c.1 - a_c0.1), turns);
            let rot = (p.rot + turns) % 4;
            let size = grid::rotated_size(d, rot);
            let c1 = (a_c1.0 + off.0, a_c1.1 + off.1);
            out.push(PieceMove {
                uid: m,
                x: (c1.0 - size.0 as i32).div_euclid(2),
                y: (c1.1 - size.1 as i32).div_euclid(2),
                rot,
            });
        }
        out
    }

    /// Would `Command::MoveGroup { moves }` succeed? (A dry run on a copy; no events.)
    pub fn can_move_pieces(
        &self,
        content: &Content,
        moves: &[PieceMove],
    ) -> Result<(), CommandError> {
        let mut w = self.clone();
        move_pieces(&mut w, content, moves).map(|_| ())
    }

    /// Why `uid` can't be moved / removed right now (`None` = free to move).
    pub fn in_use_reason(&self, uid: u32) -> Option<InUseReason> {
        let p = self.furniture_by_uid(uid)?;
        if p.seat_by.iter().any(|s| s.is_some()) {
            Some(InUseReason::Seated)
        } else if p.user.is_some() || p.claim.is_some() {
            Some(InUseReason::Busy)
        } else {
            None
        }
    }

    /// The change of the hall's coziness (0–100 scale) if a new `def` were placed at `x, y`
    /// with `rot` — e.g. `+0.6` for a lamp over the dark corner table, negative for a chair
    /// out of the light. Pure (works on a copy); placement validity is not checked. Floors
    /// don't change coziness → 0.
    pub fn coziness_preview(&self, content: &Content, def: &str, x: i32, y: i32, rot: u8) -> f32 {
        let Some(d) = content.furniture_def(def) else {
            return 0.0;
        };
        if matches!(d.category, BuildCategory::Floors | BuildCategory::Structure) {
            return 0.0;
        }
        let before = super::atmos::compute(self, content).value;
        let mut w = self.clone();
        w.furniture.push(Placed {
            uid: u32::MAX,
            def: def.to_string(),
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
        });
        super::atmos::compute(&w, content).value - before
    }

    /// The change of coziness if the pieces of `moves` stood at their new spots (a drag).
    pub fn coziness_preview_moves(&self, content: &Content, moves: &[PieceMove]) -> f32 {
        let before = super::atmos::compute(self, content).value;
        let mut w = self.clone();
        for m in moves {
            if let Some(p) = w.furniture.iter_mut().find(|p| p.uid == m.uid) {
                p.x = m.x;
                p.y = m.y;
                p.rot = m.rot % 4;
            }
        }
        super::atmos::compute(&w, content).value - before
    }
}
