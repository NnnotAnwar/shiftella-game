//! Grandma's attic (design/v0.2.md item 7): once a week a few second-hand pieces come down
//! from the attic at a flea-market price, each with a one-line story about grandma Lida
//! (`attic.ron`). Unique: each piece can be bought once. Buying brings it down into the hall
//! at once (a free spot near the middle — drag it anywhere in build mode); taken away again
//! it goes back upstairs and can be brought down for free.

use super::grid::{self, Placed, TileKind};
use super::rng::{Rng, hash_str};
use super::types::*;
use super::world::*;
use crate::content::Content;

/// An attic card.
#[derive(Debug, Clone, PartialEq)]
pub struct AtticOffer {
    /// The piece = its furniture id (`furniture.<id>.name`, story `attic.<id>.story`).
    pub id: String,
    /// The flea-market price and the price of a new one.
    pub price: u32,
    pub new_price: u32,
    /// Bought already: `placed` = it stands in the café, else it waits upstairs.
    pub owned: bool,
    pub placed: bool,
}

/// What the attic shows: this week's pieces, the ones bought that wait upstairs, and when
/// new things turn up next.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AtticView {
    /// The attic has been found (from `first_night`).
    pub open: bool,
    pub offers: Vec<AtticOffer>,
    /// Bought, not in the café now (bring down for free).
    pub upstairs: Vec<AtticOffer>,
    /// The next restock night.
    pub next_night: u32,
}

fn restock_due(w: &World, content: &Content) -> bool {
    let a = &content.build.attic;
    if w.night < a.first_night || w.build.attic_restocked == w.night {
        return false;
    }
    w.night == a.first_night || w.build.attic_restocked == 0 || w.weekday() == a.restock_weekday
}

pub(crate) fn on_day_start(w: &mut World, content: &Content) {
    if !restock_due(w, content) {
        return;
    }
    let a = &content.build.attic;
    let mut pool: Vec<&str> = a
        .pieces
        .iter()
        .filter(|p| p.from_night <= w.night && !w.build.attic_owned.contains(&p.id))
        .map(|p| p.id.as_str())
        .collect();
    // its own seeded roll (the world's RNG stream stays untouched)
    let mut rng = Rng::new(w.seed ^ hash_str(&format!("attic:{}", w.night)));
    let mut offer = Vec::new();
    while offer.len() < a.per_week as usize && !pool.is_empty() {
        let i = rng.index(pool.len());
        offer.push(pool.remove(i).to_string());
    }
    let first = w.build.attic_restocked == 0;
    w.build.attic_offer = offer;
    w.build.attic_restocked = w.night;
    if !w.build.attic_offer.is_empty() {
        w.emit(SimEvent::AtticRestocked {
            pieces: w.build.attic_offer.clone(),
            first,
        });
        w.emit(SimEvent::Toast {
            key: if first {
                "attic.toast.found".into()
            } else {
                "attic.toast.restocked".into()
            },
        });
    }
}

/// A free spot in the hall for a piece, nearest to the open floor in the hall's lower left
/// (never next to the door), keeping every piece reachable.
fn bring_down(w: &mut World, content: &Content, id: &str) -> Result<u32, CommandError> {
    let d = content
        .furniture_def(id)
        .ok_or_else(|| CommandError::UnknownId(id.to_string()))?;
    let doors = w.grid.door_tiles();
    let target = (8, 12);
    let mut spots: Vec<(i32, i32)> = Vec::new();
    for y in 1..w.grid.height {
        for x in 1..w.grid.width {
            let t = w.grid.tile(x, y);
            if t.is_some_and(|t| t.kind == TileKind::Floor && t.room == grid::Room::Hall)
                && doors.iter().all(|dt| grid::chebyshev(*dt, (x, y)) > 2)
            {
                spots.push((x, y));
            }
        }
    }
    spots.sort_by_key(|t| (grid::manhattan(*t, target), t.1, t.0));
    for (x, y) in spots {
        // room around it: the piece's footprint and a free ring (walls in the hall stay walkable)
        let fp = grid::footprint(d, x, y, 0);
        let ring_free = fp.iter().all(|&(fx, fy)| {
            [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().all(|(dx, dy)| {
                let t = (fx + dx, fy + dy);
                fp.contains(&t) || w.grid.walkable(t.0, t.1) || !w.grid.is_indoor_floor(t.0, t.1)
            })
        });
        if !ring_free || grid::can_place(&w.grid, d, x, y, 0, None).is_err() {
            continue;
        }
        let uid = w.alloc_id();
        let piece = Placed {
            uid,
            def: id.to_string(),
            x,
            y,
            rot: 0,
            on: d.has_tag("light") && !d.has_tag("music"),
            dirt: 0.0,
            dishes: 0,
            wear: 0.0,
            seat_by: vec![None; d.seats as usize],
            user: None,
            claim: None,
            // paid upstairs: taking it away refunds nothing, it goes back to the attic
            paid: 0,
        };
        if super::commands::place_piece(w, content, piece).is_ok() {
            return Ok(uid);
        }
    }
    Err(CommandError::Occupied)
}

/// `Command::BuyAtticPiece`: buys this week's piece (Day only) and brings it down; for a
/// piece bought before that waits upstairs, brings it down for free.
pub(crate) fn buy(w: &mut World, content: &Content, id: &str) -> Result<(), CommandError> {
    let piece = content
        .build
        .attic_piece(id)
        .ok_or_else(|| CommandError::UnknownId(id.to_string()))?;
    if w.phase != Phase::Day {
        return Err(CommandError::WrongPhase);
    }
    if w.furniture.iter().any(|p| p.def == id) {
        return Err(CommandError::AlreadyPlaced);
    }
    let owned = w.build.attic_owned.contains(id);
    if !owned && !w.build.attic_offer.iter().any(|o| o == id) {
        return Err(CommandError::Locked);
    }
    let price = if owned { 0 } else { piece.price as i64 };
    if w.econ.money < price {
        return Err(CommandError::NotEnoughMoney {
            need: price,
            have: w.econ.money,
        });
    }
    let uid = bring_down(w, content, id)?;
    super::economy::spend_on_cafe(w, price);
    if !owned {
        w.build.attic_owned.insert(id.to_string());
        w.build.attic_offer.retain(|o| o != id);
    }
    w.emit(SimEvent::AtticPieceBought {
        id: id.to_string(),
        uid,
        price,
    });
    w.emit(SimEvent::FurniturePlaced { uid });
    super::build_content::after_furniture_change(w, content);
    Ok(())
}

impl World {
    /// Grandma's attic: this week's offer and what waits upstairs (`attic.ron`).
    pub fn attic(&self, content: &Content) -> AtticView {
        let a = &content.build.attic;
        let card = |id: &str| -> Option<AtticOffer> {
            let p = content.build.attic_piece(id)?;
            let d = content.furniture_def(id)?;
            Some(AtticOffer {
                id: id.to_string(),
                price: p.price,
                new_price: d.price,
                owned: self.build.attic_owned.contains(id),
                placed: self.furniture.iter().any(|f| f.def == id),
            })
        };
        let offers = self
            .build
            .attic_offer
            .iter()
            .filter_map(|id| card(id))
            .collect();
        let upstairs = self
            .build
            .attic_owned
            .iter()
            .filter_map(|id| card(id))
            .filter(|o| !o.placed)
            .collect();
        // the next restock: the first night on the restock weekday after tonight
        let mut next = self.night + 1;
        if self.night < a.first_night {
            next = a.first_night;
        } else {
            let wd = |n: u32| -> crate::content::Weekday {
                match self.mode {
                    Mode::Story => content
                        .story
                        .night(n)
                        .map(|x| x.weekday)
                        .unwrap_or(crate::content::Weekday::ALL[((n - 1) % 7) as usize]),
                    Mode::Sandbox => crate::content::Weekday::ALL[((n - 1) % 7) as usize],
                }
            };
            while wd(next) != a.restock_weekday && next < self.night + 8 {
                next += 1;
            }
        }
        AtticView {
            open: self.night >= a.first_night,
            offers,
            upstairs,
            next_night: next,
        }
    }
}
