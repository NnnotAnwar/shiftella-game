//! Polaroid journal capture rules (`assets/data/journal.ron`, handson.md §11.4).

use serde::{Deserialize, Serialize};

use crate::content::defs::ClockTime;

/// What a highlight shows. Caption keys (see [`HighlightKind::caption_key`]):
/// most kinds `journal.caption.<id>`; `ArcBeat` → `journal.caption.arc.<regular>.<stage>`;
/// `StoryScene` → `journal.caption.scene.<cutscene>`; `MomentWarm` → `journal.caption.moment.<id>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum HighlightKind {
    Guest333,
    FirstSnow,
    ArcBeat,
    Mikulas,
    StoryScene,
    GarlandsUp,
    CozyLikeHome,
    PerfectServe,
    CatLevel,
    FullHouse,
    BeautifulLatte,
    MomentWarm,
    RecordLoved,
    RecipeDiscovered,
    RozpisHappy,
    AnwarCameo,
    QuietNight,
}

impl HighlightKind {
    pub const ALL: [HighlightKind; 17] = [
        HighlightKind::Guest333,
        HighlightKind::FirstSnow,
        HighlightKind::ArcBeat,
        HighlightKind::Mikulas,
        HighlightKind::StoryScene,
        HighlightKind::GarlandsUp,
        HighlightKind::CozyLikeHome,
        HighlightKind::PerfectServe,
        HighlightKind::CatLevel,
        HighlightKind::FullHouse,
        HighlightKind::BeautifulLatte,
        HighlightKind::MomentWarm,
        HighlightKind::RecordLoved,
        HighlightKind::RecipeDiscovered,
        HighlightKind::RozpisHappy,
        HighlightKind::AnwarCameo,
        HighlightKind::QuietNight,
    ];

    pub fn id(self) -> &'static str {
        match self {
            HighlightKind::Guest333 => "guest_333",
            HighlightKind::FirstSnow => "first_snow",
            HighlightKind::ArcBeat => "arc",
            HighlightKind::Mikulas => "mikulas",
            HighlightKind::StoryScene => "scene",
            HighlightKind::GarlandsUp => "garlands_up",
            HighlightKind::CozyLikeHome => "like_home",
            HighlightKind::PerfectServe => "perfect_serve",
            HighlightKind::CatLevel => "cat_level",
            HighlightKind::FullHouse => "full_house",
            HighlightKind::BeautifulLatte => "beautiful_latte",
            HighlightKind::MomentWarm => "moment",
            HighlightKind::RecordLoved => "record_loved",
            HighlightKind::RecipeDiscovered => "recipe_discovered",
            HighlightKind::RozpisHappy => "rozpis_happy",
            HighlightKind::AnwarCameo => "anwar",
            HighlightKind::QuietNight => "quiet_night",
        }
    }

    /// Kinds whose caption key needs a sub-id (regular + stage, cutscene, moment).
    pub fn has_sub_captions(self) -> bool {
        matches!(
            self,
            HighlightKind::ArcBeat | HighlightKind::StoryScene | HighlightKind::MomentWarm
        )
    }

    /// `journal.caption.<id>` or, with a sub-id, `journal.caption.<id>.<sub>`
    /// (`sub` = `mirek.2`, a cutscene id or a moment id).
    pub fn caption_key(self, sub: Option<&str>) -> String {
        match sub {
            Some(s) => format!("journal.caption.{}.{s}", self.id()),
            None => format!("journal.caption.{}", self.id()),
        }
    }
}

/// Score and caption placeholders of one kind.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HighlightRule {
    pub kind: HighlightKind,
    pub score: u32,
    /// Score for repeats (the 3:33 guest after his first visit).
    #[serde(default)]
    pub repeat_score: Option<u32>,
    /// Placeholders of the caption (`{name}`, `{pattern}`, `{record}`, `{recipe}`).
    #[serde(default)]
    pub placeholders: Vec<String>,
}

/// The night's guaranteed fallback snapshot.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FallbackShot {
    pub at: ClockTime,
    /// Focus: the front window.
    pub focus_tag: String,
}

/// `assets/data/journal.ron`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JournalFile {
    /// Candidates kept per night (the morning shows the best + the others).
    pub keep_per_night: u32,
    /// A kind kept the previous night scores this much less.
    pub repeat_kind_penalty: u32,
    /// ArcBeat bonus when the arc ended well.
    pub arc_good_bonus: u32,
    /// Snapshot window in tiles around the focus.
    pub snap_tiles: (u32, u32),
    pub fallback: FallbackShot,
    /// FullHouse: share of seats taken.
    pub full_house_share: f32,
    /// BeautifulLatte: mini-game quality ≥ this.
    pub beautiful_latte_quality: f32,
    /// CatLevel: from this cat level.
    pub cat_level_min: u32,
    pub rules: Vec<HighlightRule>,
    /// Key cutscenes with their own caption (`journal.caption.scene.<id>`).
    pub scenes: Vec<String>,
    /// Moments with a warm caption (`journal.caption.moment.<id>`).
    pub moments: Vec<String>,
}

impl JournalFile {
    pub fn rule(&self, kind: HighlightKind) -> Option<&HighlightRule> {
        self.rules.iter().find(|r| r.kind == kind)
    }
}
