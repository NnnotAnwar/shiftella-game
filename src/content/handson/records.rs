//! Grandma's jukebox: six real records, hall moods and the music rules
//! (`assets/data/records.ron`, handson.md §10).

use serde::{Deserialize, Serialize};

use super::Unlock;
use super::recipes::Taste;

/// The hall's mood while a record plays (`hall_mood.<id>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum HallMood {
    Lively,
    Calm,
    Dreamy,
    Tender,
    Nostalgic,
}

impl HallMood {
    pub const ALL: [HallMood; 5] = [
        HallMood::Lively,
        HallMood::Calm,
        HallMood::Dreamy,
        HallMood::Tender,
        HallMood::Nostalgic,
    ];

    pub fn id(self) -> &'static str {
        match self {
            HallMood::Lively => "lively",
            HallMood::Calm => "calm",
            HallMood::Dreamy => "dreamy",
            HallMood::Tender => "tender",
            HallMood::Nostalgic => "nostalgic",
        }
    }

    pub fn key(self) -> String {
        format!("hall_mood.{}", self.id())
    }
}

/// Procedural sleeve motif (render).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum Motif {
    Tram,
    Raindrops,
    PianoKeys,
    Sunflower,
    SynthGrid,
    Rose,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Sleeve {
    /// `#RRGGBB`.
    pub color: String,
    pub motif: Motif,
}

/// A record (`record.<id>.title/.artist`; non-starters also `.hint` (locked sleeve) and
/// `.source` (unlock toast); `record.<id>.liked.<regular>` / `.disliked.<regular>` bubbles).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecordDef {
    pub id: String,
    pub mood: HallMood,
    /// Music track id (`content::MUSIC_TRACKS`, file `assets/audio/music/<track>.ogg`).
    pub track: String,
    /// Length of one pass in real seconds (measured with ffprobe; ends a Play-once pass).
    pub seconds: f32,
    #[serde(default)]
    pub unlock: Unlock,
    /// Sandbox: unlocked from this night at the latest.
    #[serde(default)]
    pub sandbox_night: Option<u32>,
    #[serde(default)]
    pub liked_by: Vec<String>,
    #[serde(default)]
    pub disliked_by: Vec<String>,
    pub sleeve: Sleeve,
}

impl RecordDef {
    pub fn title_key(&self) -> String {
        format!("record.{}.title", self.id)
    }
    pub fn artist_key(&self) -> String {
        format!("record.{}.artist", self.id)
    }
    pub fn hint_key(&self) -> String {
        format!("record.{}.hint", self.id)
    }
    pub fn source_key(&self) -> String {
        format!("record.{}.source", self.id)
    }
    pub fn liked_key(&self, regular: &str) -> String {
        format!("record.{}.liked.{regular}", self.id)
    }
    pub fn disliked_key(&self, regular: &str) -> String {
        format!("record.{}.disliked.{regular}", self.id)
    }
    pub fn is_starter(&self) -> bool {
        self.unlock == Unlock::Starter
    }
}

/// Satisfaction / patience modifiers for some guest archetypes.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArchetypeMod {
    pub archetypes: Vec<String>,
    pub value: f32,
}

/// Gentle effects of a hall mood while it lasts (§10.4).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct MoodEffect {
    pub mood: HallMood,
    pub tips_mul: f32,
    pub helper_speed_mul: f32,
    pub stay_mul: f32,
    /// Added to coziness (0–100).
    pub coziness: f32,
    /// Wish pick weight × mul for wishes wanting the tag.
    pub wish_mul: Vec<(Taste, f32)>,
    /// Satisfaction + value for these archetypes (once, when they sit down).
    pub satisfaction: Vec<ArchetypeMod>,
    /// Patience × value for these archetypes.
    pub patience: Vec<ArchetypeMod>,
    /// Moment weight × mul.
    pub moment_weight: Vec<(String, f32)>,
    /// Regulars aged ≥ `.0` get `.1` trust per visit.
    pub elder_trust: Option<(u32, f32)>,
}

impl Default for MoodEffect {
    fn default() -> Self {
        MoodEffect {
            mood: HallMood::Calm,
            tips_mul: 1.0,
            helper_speed_mul: 1.0,
            stay_mul: 1.0,
            coziness: 0.0,
            wish_mul: Vec::new(),
            satisfaction: Vec::new(),
            patience: Vec::new(),
            moment_weight: Vec::new(),
            elder_trust: None,
        }
    }
}

/// The *House playlist* (no record chosen): track per state.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HouseMusic {
    pub calm: String,
    pub rush: String,
    /// Quiet hour track and volume.
    pub quiet: String,
    pub quiet_volume: f32,
    /// Day phase.
    pub day: String,
}

/// Likes, dislikes and presentation rules.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MusicRules {
    /// A liked record must have played this many game minutes while the regular is present.
    pub like_minutes: u32,
    pub like_trust: f32,
    /// Trust instead of `like_trust` when the record came from `m_record_request`.
    pub request_trust: f32,
    /// Satisfaction for a regular's disliked record (never a trust loss).
    pub dislike_satisfaction: f32,
    /// Archetype `music_likes` / `music_dislikes` satisfaction.
    pub archetype_like: f32,
    pub archetype_dislike: f32,
    /// Crossfade, real seconds.
    pub crossfade_s: f32,
    /// Needle drop: `ui_click` at this volume.
    pub needle_volume: f32,
    /// Sandbox fallback unlock nights for the non-starters (whichever comes first).
    pub sandbox_nights: Vec<u32>,
}

/// `assets/data/records.ron`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecordsFile {
    pub house: HouseMusic,
    pub rules: MusicRules,
    pub moods: Vec<MoodEffect>,
    pub records: Vec<RecordDef>,
}

impl RecordsFile {
    pub fn record(&self, id: &str) -> Option<&RecordDef> {
        self.records.iter().find(|r| r.id == id)
    }
    pub fn mood(&self, mood: HallMood) -> Option<&MoodEffect> {
        self.moods.iter().find(|m| m.mood == mood)
    }
}
