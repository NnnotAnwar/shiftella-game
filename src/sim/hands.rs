//! Hands-on state (design/handson.md §1, §3, §10): Karolina's action queue, order kinds and
//! item origins, tips on tables, recipe knowledge, rush & rhythm, the lull director, the
//! jukebox, and the night's hands-on tallies. Everything here is `#[serde(default)]` so saves
//! of version 1 load (see `World::load_from_str`).

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::content::handson::{HallMood, MatchTier, MiniGameKind};

/// Real seconds per sim tick at speed ×1 (handson.md §1.4): a night (4 800 ticks) lasts
/// 12 real minutes at ×1, 6 at ×2 and 4 at ×3. The tuning value in `handson.ron`
/// (`real_seconds_per_tick_x1`) is the source of truth — use
/// [`World::real_seconds_per_tick`](super::World::real_seconds_per_tick); this constant is the
/// shipped default.
pub const REAL_SECONDS_PER_TICK_X1: f32 = 0.15;
/// Karolina's action queue holds at most this many acts (current act not included).
pub const PLAYER_QUEUE_MAX: usize = 5;

/// How Karolina makes a drink or a dish.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BrewMode {
    /// No mini-game: she works the full prep time; quality = `quick_quality` (0.8 once
    /// practised).
    #[default]
    Quick,
    /// The mini-game *is* the work: she then works only `prep × by_hand_work_share`; the
    /// quality is what the mini-game returned.
    ByHand,
}

/// What a party's leader ordered (handson.md §1.2). Other members always order plain items.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum OrderKind {
    /// A menu item (the party's `wishes` list holds the menu ids).
    #[default]
    Plain,
    /// A wish (`wish.<id>`): Karolina picks a recipe for it.
    Wish { wish: String },
    /// A trusted regular: the wish, plus "The usual?" = `recipe` by hand with one click.
    Usual { wish: String, recipe: String },
}

impl OrderKind {
    /// The wish id (`Wish` / `Usual`).
    pub fn wish(&self) -> Option<&str> {
        match self {
            OrderKind::Plain => None,
            OrderKind::Wish { wish } | OrderKind::Usual { wish, .. } => Some(wish),
        }
    }
    pub fn is_wish(&self) -> bool {
        !matches!(self, OrderKind::Plain)
    }
    /// "The usual" recipe of a trusted regular.
    pub fn usual(&self) -> Option<&str> {
        match self {
            OrderKind::Usual { recipe, .. } => Some(recipe),
            _ => None,
        }
    }
}

/// Who made an order item, and how well.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ItemOrigin {
    /// Nobody has made it yet (a pending plain ticket).
    #[default]
    Unassigned,
    /// Karolina: the recipe she chose, the quality (0–1) and the match with the guest's wish
    /// (1.0 for a plain order — they got exactly what they asked for).
    Player {
        recipe: String,
        quality: f32,
        match_score: f32,
        mode: BrewMode,
    },
    /// A helper: quality `helper_quality.at(skill)` (≤ 0.75), never latte art.
    Helper { quality: f32 },
}

impl ItemOrigin {
    pub fn by_player(&self) -> bool {
        matches!(self, ItemOrigin::Player { .. })
    }
}

/// One thing Karolina can do. Queued with `Command::PlayerQueue` (or the sugar commands
/// `PlayerTakeOrder` / `PlayerBrew` / `PlayRecord` / `TickChecklist` at night).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PlayerAct {
    /// Walk to the party (table or register) and take their order.
    TakeOrder { party: u32 },
    /// Make an order item at its station (plus `extra` items of the same party at the same
    /// station — a quick plain brew makes the whole round), then carry them over on a tray.
    /// Created by `Command::PlayerBrew`; not queueable directly.
    Brew {
        item: u32,
        #[serde(default)]
        extra: Vec<u32>,
    },
    /// Carry a ready item a helper made to its guest (the sim adds the party's other ready
    /// items to the tray as `extra`; pass an empty list).
    Serve {
        item: u32,
        #[serde(default)]
        extra: Vec<u32>,
    },
    /// Pick up the dishes (and any tip coins) and bring them to the sink.
    ClearTable { table: u32 },
    /// Stand at the sink; `Command::PlayerWashProgress` washes (hold: 1 s per 2 dishes).
    Wash { sink: u32 },
    /// Mop a dirt spot (`World::dirt_spots`).
    Mop { tile: (i32, i32) },
    /// Collect the tip coins on a table.
    CollectTip { table: u32 },
    /// A checklist item (`app.checklist.<id>`): walk to its anchor and tick it.
    Checklist { id: String },
    /// Put a record on the jukebox (`None` = the house playlist).
    PlayRecord {
        record: Option<String>,
        repeat: bool,
    },
    /// Just walk somewhere.
    GoTo { tile: (i32, i32) },
}

impl PlayerAct {
    /// Short id for queue icons (`ui.act.<id>` suggested).
    pub fn id(&self) -> &'static str {
        match self {
            PlayerAct::TakeOrder { .. } => "take_order",
            PlayerAct::Brew { .. } => "brew",
            PlayerAct::Serve { .. } => "serve",
            PlayerAct::ClearTable { .. } => "clear_table",
            PlayerAct::Wash { .. } => "wash",
            PlayerAct::Mop { .. } => "mop",
            PlayerAct::CollectTip { .. } => "collect_tip",
            PlayerAct::Checklist { .. } => "checklist",
            PlayerAct::PlayRecord { .. } => "record",
            PlayerAct::GoTo { .. } => "go_to",
        }
    }
}

/// The act Karolina is doing right now.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveAct {
    pub act: PlayerAct,
    /// 0 = walking to the first spot, 1 = working there, 2 = carrying to the guest / sink,
    /// 3 = handing it over.
    pub stage: u8,
    /// Work ticks left in the current stage (scaled by her speed).
    pub work_left: f32,
    /// Work ticks of the current stage at the start (for the progress ring).
    pub work_total: f32,
    /// Ticks spent waiting (a busy station, the sink without a hold).
    pub waited: u32,
    /// The station she holds while brewing.
    #[serde(default)]
    pub target: Option<u32>,
    /// Dishes she carries to the sink.
    #[serde(default)]
    pub count: u32,
}

impl ActiveAct {
    /// 0–1 progress of the current work stage (UI ring).
    pub fn progress(&self) -> f32 {
        if self.work_total <= 0.0 {
            0.0
        } else {
            (1.0 - self.work_left / self.work_total).clamp(0.0, 1.0)
        }
    }
}

/// Karolina, the player actor (handson.md §1.1). She has no job priorities: she only does
/// what the player queues.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PlayerState {
    /// FIFO, at most [`PLAYER_QUEUE_MAX`].
    pub queue: VecDeque<PlayerAct>,
    pub current: Option<ActiveAct>,
    /// Game minutes in a row with nothing meaningful to do (the lull director's clock).
    pub idle_minutes: u32,
    /// Rhythm streak (0 = none): serves within `rhythm.window_minutes` of the previous one.
    pub rhythm: u8,
    pub last_serve_minute: Option<u32>,
    /// Fractional dishes from hold-to-wash.
    pub wash_bank: f32,
    /// The last by-hand mini-game kind and how many in a row (anti-repetition, §2.0).
    pub minigame_streak: Option<(MiniGameKind, u32)>,
    /// Last mode chosen for plain / wish orders (the order card remembers it).
    pub last_mode_plain: BrewMode,
    pub last_mode_wish: BrewMode,
}

/// A tip coin a guest left on their table (handson.md §1.2): click it to collect.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TipCoin {
    pub id: u32,
    /// Table furniture uid.
    pub table: u32,
    pub amount: i64,
    /// Who served the party (for `TipReceived`).
    pub staff: Option<u32>,
    pub minute: u32,
}

/// What Karolina knows about one recipe.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RecipeStats {
    pub learned_night: u32,
    /// Karolina-made serves.
    pub served: u32,
    /// By-hand plays at ≥ `practised_min_quality`.
    pub good_by_hand: u32,
    pub best_quality: f32,
    /// After `practised_after` good by-hand plays: Quick brew quality rises.
    pub practised: bool,
}

/// Recipe book & pantry knowledge (T4: recipes and ingredient unlocks; experiments are T5).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Knowledge {
    pub recipes: BTreeMap<String, RecipeStats>,
    pub ingredients: BTreeSet<String>,
}

/// What the jukebox plays.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum MusicSource {
    /// The house playlist (`records.ron` → `house`): calm / rush / quiet hour. No hall mood.
    #[default]
    House,
    /// A record of grandma's; `repeat: false` = one pass, then House.
    Record { id: String, repeat: bool },
}

/// The state of the house playlist (which house track plays).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HouseState {
    Calm,
    Rush,
    Quiet,
}

/// Grandma's jukebox (handson.md §10).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MusicState {
    pub source: MusicSource,
    /// Picked in the Day phase: starts at 22:00.
    pub opening: Option<(String, bool)>,
    /// Game minutes the current record has played tonight.
    pub record_minutes: u32,
    /// The regular whose `m_record_request` put this record on (request trust bonus, T5).
    pub requested_by: Option<String>,
    pub unlocked: BTreeSet<String>,
    /// (record, regular) pairs whose taste Karolina has seen (sleeve faces).
    pub known_tastes: BTreeSet<(String, String)>,
}

/// A rush after a train or a crowd (handson.md §1.5).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RushState {
    pub until: Option<u32>,
    /// Arrival minutes of recent parties.
    pub arrivals: Vec<u32>,
}

/// A hall mood set by an effect (the playing record's mood applies otherwise).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HallMoodState {
    pub mood: Option<HallMood>,
    pub until: Option<u32>,
}

/// What the lull director suggests when Karolina has been idle (handson.md §1.6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LullHint {
    /// A present regular has something to talk about: pulse their Talk chip.
    Talk { guest: u32 },
    /// A guest walks in (and the guest rate rises for a while from story night 3).
    WalkIn,
    /// Rain brought in some mud by the door (a mop spot).
    Mud,
    /// "It's quiet — ×2?" chip.
    Speed,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LullState {
    pub last_fire: Option<u32>,
    pub walk_ins: u32,
}

/// Hands-on tallies of the current night (feed `NightSummary` and the balance harness).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HandsTally {
    pub karolina_items: u32,
    pub helper_items: u32,
    pub by_hand: u32,
    pub wishes_served: u32,
    /// Served wishes by tier: perfect, good, okay, not quite.
    pub tiers: [u32; 4],
    pub regular_items: u32,
    pub regular_items_karolina: u32,
    pub forgiven_parties: u32,
    pub parties: u32,
    /// Night minutes with nothing meaningful for Karolina (quiet hour counted apart).
    pub idle_minutes: u32,
    pub idle_quiet_minutes: u32,
    pub quiet_minutes: u32,
    /// Histogram of Karolina's pending things per minute (index = count, last = 15+).
    pub todo_hist: Vec<u32>,
    pub todo_peak: u32,
    pub rushes: u32,
    pub best_rhythm: u8,
    pub tips_collected: i64,
    pub lull_fired: u32,
}

/// Every hands-on piece of world state.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HandsState {
    /// Knowledge, records and duties were initialised from content (new game or migrated save).
    pub ready: bool,
    pub player: PlayerState,
    pub knowledge: Knowledge,
    pub music: MusicState,
    pub tips: Vec<TipCoin>,
    pub tips_lifetime: i64,
    pub rush: RushState,
    pub hall_mood: HallMoodState,
    /// Wish id → uses tonight.
    pub wish_uses: BTreeMap<String, u32>,
    pub lull: LullState,
    pub tally: HandsTally,
    /// Accessibility `assist_wishes`: helpers may hand off wishes after a while.
    pub assist_wishes: bool,
    /// A version 1 save: grandma's jukebox still has to be put in its place (next Day).
    pub place_jukebox: bool,
}

/// Who holds a ticket on the rail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TicketOwner {
    Karolina,
    Helper(u32),
    /// No helper holds that duty (hollow ring): Karolina's to take.
    Nobody,
}

/// What a ticket is waiting for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TicketStage {
    /// The order hasn't been taken yet.
    Order,
    /// A wish waits for Karolina to pick a recipe.
    Wish { wish: String },
    /// An item is being / waits to be made.
    Make { item: u32, menu: String },
    /// A made item waits to be carried over.
    Serve { item: u32, menu: String },
}

/// One chip on the ticket rail (handson.md §7.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ticket {
    pub party: u32,
    /// The guest the chip is about (leader / the item's guest).
    pub guest: u32,
    pub stage: TicketStage,
    pub owner: TicketOwner,
    /// 0–1 of the party's patience used (never red on Cozy).
    pub wait_frac: f32,
    pub regular: Option<String>,
}

/// A served item's reaction tier (re-exported helper for UI).
pub fn tier_index(t: MatchTier) -> usize {
    match t {
        MatchTier::Perfect => 0,
        MatchTier::Good => 1,
        MatchTier::Okay => 2,
        MatchTier::NotQuite => 3,
    }
}
