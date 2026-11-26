//! Reasons to build (design/v0.2.md package C): requests from guests and staff
//! (`build_wishes.ron`), corners — pieces placed together (`corners.ron`), grandma's attic
//! and the back room (`attic.ron`).
//!
//! Loaded into `content.build` and validated with the rest. Semantics: `docs/CONTENT.md` §11.

use serde::{Deserialize, Serialize};

use super::defs::{ClockTime, Weekday};
use super::handson::{Cond, Fx};

/// Who asks for a piece.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum WishWho {
    /// A story regular (`regulars.ron` id); the wish shows once they have visited.
    Regular(String),
    /// A helper: of that staff archetype, or (`None`) the first helper hired. The wish shows
    /// once such a helper is employed.
    Staff(Option<String>),
    /// Guests of an archetype (`guests.ron` id), e.g. the students.
    Guests(String),
}

/// One request: "Hana: a quiet corner". Keys `build_wish.<id>.reason` (the short card title,
/// "a quiet corner"), `.ask` (what they say, the card's quote and their bubble) and `.thanks`
/// (their reaction once it stands).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BuildWishDef {
    pub id: String,
    pub who: WishWho,
    /// The piece that answers it (the build card's "wanted" badge).
    #[serde(default)]
    pub def: Option<String>,
    /// …or any piece with this tag.
    #[serde(default)]
    pub tag: Option<String>,
    /// The request shows once these hold (and the person is known).
    #[serde(default)]
    pub appear: Vec<Cond>,
    /// Fulfilled while these hold.
    pub done: Vec<Cond>,
    /// When the person reacts (target = them: their party / the helper / the regular).
    #[serde(default)]
    pub reward: Vec<Fx>,
    /// The request mirrors a story arc's own condition (Filip's socket, Lori's stage, Hana's
    /// corner): the arc keeps its condition, the card only tells the player about it.
    #[serde(default)]
    pub arc: bool,
}

impl BuildWishDef {
    pub fn reason_key(&self) -> String {
        format!("build_wish.{}.reason", self.id)
    }
    pub fn ask_key(&self) -> String {
        format!("build_wish.{}.ask", self.id)
    }
    pub fn thanks_key(&self) -> String {
        format!("build_wish.{}.thanks", self.id)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BuildWishesFile {
    pub wishes: Vec<BuildWishDef>,
}

fn one() -> u32 {
    1
}

fn attract_default() -> f32 {
    1.25
}

/// One ingredient of a corner: a piece whose id is in `defs` or that has one of `tags`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CornerNeed {
    #[serde(default)]
    pub defs: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "one")]
    pub count: u32,
}

impl CornerNeed {
    pub fn matches(&self, def_id: &str, tags: &[String]) -> bool {
        self.defs.iter().any(|d| d == def_id) || self.tags.iter().any(|t| tags.contains(t))
    }
}

/// A named set of pieces placed together ("reading nook"). The first need is the anchor
/// (one piece); every other piece must stand within `radius` tiles (Chebyshev, footprint to
/// footprint) of it, in the hall. Keys `corner.<id>.name` / `.desc`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CornerDef {
    pub id: String,
    pub needs: Vec<CornerNeed>,
    #[serde(default = "one")]
    pub radius: u32,
    /// Coziness points (0–100 scale) per corner of this kind.
    pub bonus: f32,
    /// At most this many of the kind count.
    #[serde(default = "one")]
    pub max: u32,
    /// Guest archetypes that come a little more often while it stands.
    #[serde(default)]
    pub attracts: Vec<String>,
    /// Their arrival weight × this (per corner).
    #[serde(default = "attract_default")]
    pub attract_mul: f32,
}

impl CornerDef {
    pub fn name_key(&self) -> String {
        format!("corner.{}.name", self.id)
    }
    pub fn desc_key(&self) -> String {
        format!("corner.{}.desc", self.id)
    }
}

/// When the morning offers a before / after polaroid.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BeforeAfterDef {
    /// The hall's furniture coziness rose by at least this since the night before.
    pub min_gain: f32,
    /// The "after" photo is taken at this time (people in it).
    pub after_at: ClockTime,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CornersFile {
    pub corners: Vec<CornerDef>,
    /// The corners' coziness together is capped here.
    pub max_bonus: f32,
    pub before_after: BeforeAfterDef,
}

/// A second-hand piece in grandma's attic: a furniture id (hidden from the build catalogue by
/// the flag `attic_piece`), its flea-market price and the night it can first turn up.
/// Key `attic.<id>.story` (a one-line story about grandma Lida).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AtticPiece {
    pub id: String,
    pub price: u32,
    #[serde(default = "one")]
    pub from_night: u32,
}

impl AtticPiece {
    pub fn story_key(&self) -> String {
        format!("attic.{}.story", self.id)
    }
}

/// Grandma's back room: a second small hall behind the right wall, reopened once.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BackRoomDef {
    pub from_night: u32,
    /// Sandbox: from this night.
    pub sandbox_night: u32,
    pub price: u32,
    /// Grandma's old pieces waiting inside: (furniture, x, y, rot), tiles of the widened grid.
    pub pieces: Vec<(String, i32, i32, u8)>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AtticFile {
    /// The attic gets new things on this weekday's morning.
    pub restock_weekday: Weekday,
    /// …from this night on.
    pub first_night: u32,
    /// Pieces on offer per week.
    pub per_week: u32,
    pub pieces: Vec<AtticPiece>,
    pub back_room: BackRoomDef,
}

/// Everything of package C, loaded together.
#[derive(Debug, Clone, PartialEq)]
pub struct BuildContent {
    pub wishes: BuildWishesFile,
    pub corners: CornersFile,
    pub attic: AtticFile,
}

/// The data files of [`BuildContent`].
pub const BUILD_FILES: &[&str] = &["build_wishes.ron", "corners.ron", "attic.ron"];

/// The flag that hides attic pieces from the build catalogue (never set).
pub const ATTIC_FLAG: &str = "attic_piece";

impl BuildContent {
    pub fn load(dir: &str) -> Result<BuildContent, String> {
        Ok(BuildContent {
            wishes: super::read(dir, "build_wishes.ron")?,
            corners: super::read(dir, "corners.ron")?,
            attic: super::read(dir, "attic.ron")?,
        })
    }

    pub fn wish(&self, id: &str) -> Option<&BuildWishDef> {
        self.wishes.wishes.iter().find(|w| w.id == id)
    }
    pub fn corner(&self, id: &str) -> Option<&CornerDef> {
        self.corners.corners.iter().find(|c| c.id == id)
    }
    pub fn attic_piece(&self, id: &str) -> Option<&AtticPiece> {
        self.attic.pieces.iter().find(|p| p.id == id)
    }
}

/// Fixed i18n keys of package C (sim toasts, the before / after caption, the attic & requests
/// UI) with their placeholders.
pub const UI_KEYS: &[(&str, &[&str])] = &[
    ("attic.toast.found", &[]),
    ("attic.toast.restocked", &[]),
    ("backroom.toast.opened", &[]),
    ("build.before_after.back_room", &[]),
    ("build.before_after.cozier", &[]),
    ("journal.caption.before_after", &["what"]),
    ("ui.attic.title", &[]),
    ("ui.attic.button", &[]),
    ("ui.attic.sub", &["night"]),
    ("ui.attic.locked", &["night"]),
    ("ui.attic.empty", &[]),
    ("ui.attic.bring", &[]),
    ("ui.attic.new_price", &["price"]),
    ("ui.attic.upstairs", &[]),
    ("ui.attic.free", &[]),
    ("ui.attic.placed", &["item"]),
    ("ui.attic.day_only", &[]),
    ("ui.attic.no_room", &[]),
    ("ui.backroom.title", &[]),
    ("ui.backroom.locked", &["night"]),
    ("ui.backroom.ready", &[]),
    ("ui.backroom.open", &[]),
    ("ui.backroom.is_open", &["night"]),
    ("ui.requests.build_title", &[]),
    ("ui.requests.waiting", &[]),
    ("ui.requests.built", &[]),
    ("ui.requests.thanked", &[]),
    ("ui.requests.story", &[]),
    ("ui.requests.build", &[]),
    ("ui.requests.opened", &["name", "reason"]),
    ("ui.corner.formed", &["name"]),
    ("ui.corner.bonus", &["n"]),
];
