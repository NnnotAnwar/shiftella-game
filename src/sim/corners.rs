//! Corners (design/v0.2.md item 6): pieces placed together in the hall form a named corner —
//! a reading nook (armchair + lamp + bookshelf), a window spot, a little stage, a coat
//! corner… Each gives a coziness bonus (`corners.ron`) and draws its guest archetypes a
//! little more often. Detected live from the furniture (pure); a new one emits
//! `SimEvent::CornerFormed`.

use super::grid::{self, Room};
use super::types::*;
use super::world::*;
use crate::content::Content;

/// A corner standing in the café (for build-mode labels and the coziness meter).
#[derive(Debug, Clone, PartialEq)]
pub struct CornerView {
    pub id: String,
    /// `corner.<id>.name`.
    pub name_key: String,
    /// Every tile its pieces cover (draw a soft label over them).
    pub tiles: Vec<(i32, i32)>,
    /// Coziness points it adds (after the `max` per kind; the total is capped by
    /// `corners.ron` → `max_bonus`).
    pub bonus: f32,
    /// The anchor piece's tile (label position, camera focus).
    pub center: (i32, i32),
    /// Furniture uids that make it up (anchor first).
    pub members: Vec<u32>,
}

impl CornerView {
    /// `<id>:<anchor uid>` — identifies one corner instance.
    pub fn key(&self) -> String {
        format!("{}:{}", self.id, self.members.first().copied().unwrap_or(0))
    }
}

struct Piece<'a> {
    uid: u32,
    def: &'a str,
    tags: &'a [String],
    fp: Vec<(i32, i32)>,
}

fn dist(a: &[(i32, i32)], b: &[(i32, i32)]) -> i32 {
    a.iter()
        .flat_map(|x| b.iter().map(move |y| grid::chebyshev(*x, *y)))
        .min()
        .unwrap_or(i32::MAX)
}

/// Every corner standing now (pure; data order, anchors by uid).
pub(crate) fn detect(w: &World, content: &Content) -> Vec<CornerView> {
    let pieces: Vec<Piece> = w
        .furniture
        .iter()
        .filter_map(|p| {
            let d = content.furniture_def(&p.def)?;
            let fp = grid::footprint(d, p.x, p.y, p.rot);
            fp.iter()
                .any(|t| w.grid.room(t.0, t.1) == Room::Hall)
                .then_some(Piece {
                    uid: p.uid,
                    def: p.def.as_str(),
                    tags: &d.tags,
                    fp,
                })
        })
        .collect();
    let mut out = Vec::new();
    for def in &content.build.corners.corners {
        let Some(anchor_need) = def.needs.first() else {
            continue;
        };
        let mut used: Vec<u32> = Vec::new();
        let mut made = 0u32;
        let mut anchors: Vec<&Piece> = pieces
            .iter()
            .filter(|p| anchor_need.matches(p.def, p.tags))
            .collect();
        anchors.sort_by_key(|p| p.uid);
        for a in anchors {
            if made >= def.max.max(1) {
                break;
            }
            if used.contains(&a.uid) {
                continue;
            }
            let mut members = vec![a.uid];
            let mut ok = true;
            for need in def.needs.iter().skip(1) {
                let mut cands: Vec<(&Piece, i32)> = pieces
                    .iter()
                    .filter(|p| {
                        !members.contains(&p.uid)
                            && !used.contains(&p.uid)
                            && need.matches(p.def, p.tags)
                    })
                    .map(|p| (p, dist(&a.fp, &p.fp)))
                    .filter(|(_, d)| *d <= def.radius as i32)
                    .collect();
                cands.sort_by_key(|(p, d)| (*d, p.uid));
                if cands.len() < need.count as usize {
                    ok = false;
                    break;
                }
                members.extend(cands.iter().take(need.count as usize).map(|(p, _)| p.uid));
            }
            if !ok {
                continue;
            }
            used.extend(members.iter().copied());
            made += 1;
            let mut tiles: Vec<(i32, i32)> = Vec::new();
            for m in &members {
                if let Some(p) = pieces.iter().find(|p| p.uid == *m) {
                    for t in &p.fp {
                        if !tiles.contains(t) {
                            tiles.push(*t);
                        }
                    }
                }
            }
            out.push(CornerView {
                id: def.id.clone(),
                name_key: def.name_key(),
                tiles,
                bonus: def.bonus,
                center: a.fp[0],
                members,
            });
        }
    }
    out
}

/// The corners' coziness points (capped).
pub(crate) fn coziness_bonus(w: &World, content: &Content) -> f32 {
    let sum: f32 = detect(w, content).iter().map(|c| c.bonus).sum();
    sum.min(content.build.corners.max_bonus)
}

/// Arrival weight multipliers per guest archetype from the corners standing now.
pub(crate) fn attraction(w: &World, content: &Content) -> Vec<(String, f32)> {
    let mut out: Vec<(String, f32)> = Vec::new();
    for c in detect(w, content) {
        let Some(def) = content.build.corner(&c.id) else {
            continue;
        };
        for a in &def.attracts {
            match out.iter_mut().find(|x| &x.0 == a) {
                Some(x) => x.1 *= def.attract_mul,
                None => out.push((a.clone(), def.attract_mul)),
            }
        }
    }
    out
}

/// After the furniture changed: new corners → `CornerFormed` (+ the meter).
pub(crate) fn on_furniture_changed(w: &mut World, content: &Content) {
    let now = detect(w, content);
    let keys: std::collections::BTreeSet<String> = now.iter().map(|c| c.key()).collect();
    for c in &now {
        if !w.build.corners_seen.contains(&c.key()) {
            let first = w.build.corners_ever.insert(c.id.clone());
            w.emit(SimEvent::CornerFormed {
                id: c.id.clone(),
                tile: c.center,
                first,
            });
        }
    }
    if keys != w.build.corners_seen {
        w.build.corners_seen = keys;
        super::atmos::recompute(w, content);
    }
}

impl World {
    /// Corners standing now: `CornerView { id, name_key, tiles, bonus, center, members }`
    /// (build-mode labels; `docs/SIM.md` §11).
    pub fn corners(&self, content: &Content) -> Vec<CornerView> {
        detect(self, content)
    }

    /// Their coziness points together (already part of `coziness().value`).
    pub fn corners_bonus(&self, content: &Content) -> f32 {
        coziness_bonus(self, content)
    }
}
