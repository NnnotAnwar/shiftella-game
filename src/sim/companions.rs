//! Station helpers: small pieces that do no work themselves but make the station next to them
//! faster (tag `helps:<what>`). Each one has its own job, so no two pieces repeat each other:
//!
//! * `dish_rack` (`helps:wash`) — the drying rack beside the sink: the cups dry on it by
//!   themselves instead of being towelled one by one, so washing at that sink goes
//!   [`RACK_WASH_MUL`]× faster. Dirty dishes still go to the sink (the only `sink` piece).
//! * `espresso_grinder` (`helps:coffee`) — fresh grounds beside the coffee machine: coffee at
//!   that machine brews [`GRINDER_BREW_MUL`]× faster.
//!
//! "Beside" = the footprints touch on a side or a corner. One helper is enough; a second one
//! next to the same station adds nothing.

use super::grid::{self, Placed};
use super::world::World;
use crate::content::Content;

/// Washing speed at a sink with a drying rack beside it.
pub const RACK_WASH_MUL: f32 = 1.5;
/// Brewing speed at a coffee machine with a grinder beside it.
pub const GRINDER_BREW_MUL: f32 = 1.2;

/// What a helper tag speeds up: (`helps:` tag, the station's own tag, multiplier).
pub const HELPERS: &[(&str, &str, f32)] = &[
    ("helps:wash", "sink", RACK_WASH_MUL),
    ("helps:coffee", "station:coffee", GRINDER_BREW_MUL),
];

fn touches(content: &Content, a: &Placed, b: &Placed) -> bool {
    let (Some(da), Some(db)) = (content.furniture_def(&a.def), content.furniture_def(&b.def))
    else {
        return false;
    };
    let fa = grid::footprint(da, a.x, a.y, a.rot);
    let fb = grid::footprint(db, b.x, b.y, b.rot);
    fa.iter()
        .any(|p| fb.iter().any(|q| p != q && grid::chebyshev(*p, *q) <= 1))
}

/// The speed factor a station gets from its helpers (1.0 = none beside it).
pub fn station_mul(w: &World, content: &Content, station: u32) -> f32 {
    let Some(st) = w.furniture.iter().find(|p| p.uid == station) else {
        return 1.0;
    };
    let Some(sd) = content.furniture_def(&st.def) else {
        return 1.0;
    };
    let mut mul = 1.0;
    for (helps, tag, m) in HELPERS {
        if !sd.has_tag(tag) {
            continue;
        }
        let helped = w.furniture.iter().any(|p| {
            p.uid != station
                && content
                    .furniture_def(&p.def)
                    .is_some_and(|d| d.has_tag(helps))
                && touches(content, st, p)
        });
        if helped {
            mul *= m;
        }
    }
    mul
}

/// The station tag a helper piece speeds up and by how much (`dish_rack` → `sink`, 1.5).
pub fn helps(def: &crate::content::FurnitureDef) -> Option<(&'static str, f32)> {
    HELPERS
        .iter()
        .find(|(h, _, _)| def.has_tag(h))
        .map(|(_, tag, m)| (*tag, *m))
}

/// Is this helper piece standing beside a station it helps?
pub fn helper_working(w: &World, content: &Content, uid: u32) -> bool {
    let Some(p) = w.furniture.iter().find(|p| p.uid == uid) else {
        return false;
    };
    let Some((tag, _)) = content.furniture_def(&p.def).and_then(helps) else {
        return false;
    };
    w.furniture.iter().any(|s| {
        s.uid != uid
            && content
                .furniture_def(&s.def)
                .is_some_and(|d| d.has_tag(tag))
            && touches(content, p, s)
    })
}
