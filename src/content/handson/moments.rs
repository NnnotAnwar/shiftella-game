//! Small night moments (`assets/data/moments.ron`, handson.md §5, §11.1).

use serde::{Deserialize, Serialize};

use super::Tier;
use super::logic::{Cond, Fx};

/// Drives the marker icon and the quiet-hour filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum MomentKind {
    Music,
    Light,
    Leak,
    Cat,
    Traveller,
    Window,
    Guest,
    Staff,
    Drone,
    Street,
    Season,
    Delivery,
}

/// Which agent a moment is anchored to. The moment is only eligible while one matches.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum AgentSelector {
    /// A present regular whose liked record is unlocked and not playing.
    RegularWantsRecord,
    /// An on-shift helper with energy below the value.
    StaffLowEnergy(f32),
    /// A guest matching the filter.
    Guest(GuestFilter),
}

/// Filter for [`AgentSelector::Guest`]; every set field must match.
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct GuestFilter {
    /// Guest archetype ids (empty = any).
    pub archetypes: Vec<String>,
    /// Regular ids that also match (in addition to `archetypes`).
    pub regulars: Vec<String>,
    /// Regulars that never match (e.g. Hana on her cutscene night).
    pub not_regulars: Vec<String>,
    /// The party is at the door (just arrived, not seated yet).
    pub at_door: bool,
    /// Seated for at least N game minutes.
    pub seated_minutes: Option<u32>,
    /// Has been served and finished eating.
    pub fed: bool,
    /// Party size ≥ n.
    pub party_min: Option<u32>,
    /// Look accessory one member carries (`child`, `umbrella`).
    pub accessory: Option<String>,
}

/// Where the glowing marker sits.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum MomentAnchor {
    Tile(i32, i32),
    /// The nearest placed furniture with this tag (`window`).
    Tag(String),
    /// A placed furniture piece by id (`radio`, `jukebox`).
    Furniture(String),
    Door,
    Street,
    Counter,
    /// The tile blocked by the roof leak.
    Leak,
    /// The bucket placed by `m_leak_bucket`.
    Bucket,
    Agent(AgentSelector),
}

/// What resolving an option asks of the player's hands (beyond one click).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum Interaction {
    /// One swipe across the anchor (wipe the glass).
    Swipe,
    /// Hold for N seconds.
    Hold(f32),
    /// The Shiftella app's mini card with N trains; picking the right one applies the effects.
    TrainPicker { trains: u32 },
    /// A quick click per table (or one click for all).
    LightEachTable,
    /// N glowing hang spots, one click + 1.5 s on a stool each.
    HangSpots(u32),
    /// Karolina quick-brews `count` × `recipe` before the effects apply.
    QuickBrews { recipe: String, count: u32 },
    /// A by-hand mini-game first: the target helper's favourite (or a known recipe).
    ByHandFavourite,
}

/// One button on the moment card (`moment.<id>.opt.<opt>`, `moment.<id>.result.<opt>`).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MomentOption {
    pub id: String,
    /// Money shown on the button (informational; the cost is in `effects`).
    #[serde(default)]
    pub cost: i64,
    #[serde(default)]
    pub effects: Vec<Fx>,
    #[serde(default)]
    pub interaction: Option<Interaction>,
    /// Option hidden unless these hold.
    #[serde(default)]
    pub show_if: Vec<Cond>,
    /// Only offered within N game minutes of the moment appearing ("run after them").
    #[serde(default)]
    pub within_minutes: Option<u32>,
}

fn weight() -> f32 {
    10.0
}

fn one() -> u32 {
    1
}

fn expires() -> u32 {
    25
}

/// A night moment (`moment.<id>.title/.text`; `.expired` when `on_expire` is non-empty).
///
/// Lifecycle: eligible (conditions + anchor) → `MomentAvailable { id, anchor }` (marker) →
/// `ResolveMoment { id, option }` (Karolina walks there; effects on arrival) or expiry
/// (`on_expire`). Only Karolina resolves moments.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MomentDef {
    pub id: String,
    pub kind: MomentKind,
    #[serde(default)]
    pub tier: Tier,
    pub anchor: MomentAnchor,
    #[serde(default)]
    pub conditions: Vec<Cond>,
    /// Replaces that event's choice toast: the event fires its base effects and spawns this
    /// moment instead of showing `choices` (its default choice becomes `on_expire`).
    #[serde(default)]
    pub from_event: Option<String>,
    #[serde(default = "weight")]
    pub weight: f32,
    #[serde(default = "one")]
    pub per_night_max: u32,
    /// `None` = unlimited.
    #[serde(default)]
    pub max_per_game: Option<u32>,
    #[serde(default)]
    pub cooldown_nights: u32,
    #[serde(default = "expires")]
    pub expires_minutes: u32,
    /// Preferred during the quiet hour.
    #[serde(default)]
    pub quiet: bool,
    /// Spawned only by `SpawnMoment` (a follow-up); not counted in the night budget.
    #[serde(default)]
    pub follow_up: bool,
    pub options: Vec<MomentOption>,
    #[serde(default)]
    pub on_expire: Vec<Fx>,
}

impl MomentDef {
    pub fn title_key(&self) -> String {
        format!("moment.{}.title", self.id)
    }
    pub fn text_key(&self) -> String {
        format!("moment.{}.text", self.id)
    }
    pub fn option_key(&self, opt: &str) -> String {
        format!("moment.{}.opt.{opt}", self.id)
    }
    pub fn result_key(&self, opt: &str) -> String {
        format!("moment.{}.result.{opt}", self.id)
    }
    /// Toast when it expires with `on_expire` effects ("Kuba put a bucket under it").
    pub fn expired_key(&self) -> String {
        format!("moment.{}.expired", self.id)
    }
    pub fn option(&self, id: &str) -> Option<&MomentOption> {
        self.options.iter().find(|o| o.id == id)
    }
}
