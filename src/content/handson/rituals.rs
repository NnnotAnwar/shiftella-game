//! Hands-on rituals (the owner: "more interesting than just clicking a tick"): when Karolina
//! herself does an opening / closing checklist item (or an appliance chore) and the player is
//! watching at ×1, a tiny in-world gesture plays over the piece — pull the window lamp's cord,
//! hold the machine's needle in the green, count the float into the tray… `handson.ron` →
//! `rituals`. Never needed: ignored, at ×2/×3, by the autoplayer or a helper the item simply
//! completes on its own (`sim::rituals`, `screens::game::rituals_ui`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// What a cloth / chalk / sponge / brush stroke gesture looks like.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum StrokeLook {
    /// A cloth over a table top (crumbs and a ring fade).
    Cloth,
    /// Chalk on the specials board (lines of writing appear).
    Chalk,
    /// A sponge over a plate (suds).
    Sponge,
    /// A brush on the machine's group head (coffee grounds go).
    Brush,
}

/// What sits on the shelf of a tick-off gesture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum ShelfLook {
    /// Jars and bags in the pantry (the stock check).
    Pantry,
    /// Bottles and boxes in the fridge (the closing fridge check).
    Fridge,
}

/// What is slid along the track of a slide gesture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum SlideLook {
    /// The milk bottle out of the fridge.
    Bottle,
    /// The cake tray into the oven.
    Tray,
    /// The tied trash bag to the door.
    Bag,
}

/// One small gesture (2–5 seconds). Every kind works with the mouse and with Space / arrows.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum RitualGesture {
    /// Pull the lamp's cord down (a short drag; or hold ↓ / Space).
    Cord,
    /// Hold to build the pressure; let go with the needle in the green band (0–1).
    Needle { green: (f32, f32) },
    /// Tap coins and notes into the tray until the total matches (Kč; the target is picked
    /// from `total`, a range, per night).
    Count { coins: Vec<u32>, total: (u32, u32) },
    /// `count` strokes across the surface (drag; or ← → in turn).
    Strokes { count: u32, look: StrokeLook },
    /// Tick off `items` things on a shelf, the glowing one first (click; or Space).
    Shelf { items: u32, look: ShelfLook },
    /// Drop the needle as the swinging arm passes over the lead-in groove (a timed click; the
    /// groove is `band` wide, 0–1).
    Drop { band: f32 },
    /// Sort `coins` coins into their stacks (click the stack; or ← ↓ →).
    Sort { coins: u32 },
    /// Pick one of `count` short lines to write (the handover note's suggestions).
    Lines { count: u32 },
    /// Drag the thing along its track (or hold → / Space).
    Slide { look: SlideLook },
    /// Stir once around the pot (drag around; or hold Space).
    Stir,
}

impl RitualGesture {
    /// i18n id (`ritual.hint.<id>`) and the remembered-hint key (`Settings::ritual_hints`).
    pub fn id(&self) -> &'static str {
        match self {
            RitualGesture::Cord => "cord",
            RitualGesture::Needle { .. } => "needle",
            RitualGesture::Count { .. } => "count",
            RitualGesture::Strokes { .. } => "strokes",
            RitualGesture::Shelf { .. } => "shelf",
            RitualGesture::Drop { .. } => "drop",
            RitualGesture::Sort { .. } => "sort",
            RitualGesture::Lines { .. } => "lines",
            RitualGesture::Slide { .. } => "slide",
            RitualGesture::Stir => "stir",
        }
    }

    pub const IDS: [&'static str; 10] = [
        "cord", "needle", "count", "strokes", "shelf", "drop", "sort", "lines", "slide", "stir",
    ];
}

/// `handson.ron` → `rituals`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct RitualTuning {
    /// Ticks Karolina waits at the piece for a gesture the player has started (then she
    /// finishes on her own: a forgotten gesture never holds her).
    pub patience_ticks: u32,
    /// Perfect gestures in one list for its ritual's `perfect` reward (`shiftella.ron`).
    pub perfect_min: u32,
    /// Gesture per checklist id, or per appliance chore as `equip.<action>` (an appliance
    /// chore without its own entry uses the gesture of the checklist item it ticks).
    pub gestures: BTreeMap<String, RitualGesture>,
}

impl Default for RitualTuning {
    fn default() -> Self {
        RitualTuning {
            patience_ticks: 120,
            perfect_min: 3,
            gestures: BTreeMap::new(),
        }
    }
}

impl RitualTuning {
    pub fn gesture(&self, key: &str) -> Option<&RitualGesture> {
        self.gestures.get(key)
    }
}
