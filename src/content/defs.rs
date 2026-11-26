//! Core content definitions: jobs, time, weather, looks, traits, staff, menu, guests, furniture.
//!
//! All ids are plain strings (`"night_owl"`, `"oat_flat_white"`). Player-visible text is never stored
//! here: every definition derives its i18n keys from its id (see the `*_key` methods and
//! `docs/CONTENT.md`).

use std::cmp::Ordering;
use std::fmt;

use serde::{Deserialize, Serialize};

use super::logic::{Condition, Effect};

// ---------------------------------------------------------------------------------------------
// Jobs, needs, time
// ---------------------------------------------------------------------------------------------

/// A kind of work. Skills use the same five values (see [`Skill`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize)]
pub enum Job {
    Bar,
    Kitchen,
    Register,
    Cleaning,
    Serving,
}

/// Skills are the same five as jobs: `skill(Job::Bar)` is barista skill, etc.
pub type Skill = Job;

impl Job {
    pub const ALL: [Job; 5] = [
        Job::Bar,
        Job::Kitchen,
        Job::Register,
        Job::Cleaning,
        Job::Serving,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Job::Bar => "bar",
            Job::Kitchen => "kitchen",
            Job::Register => "register",
            Job::Cleaning => "cleaning",
            Job::Serving => "serving",
        }
    }

    pub fn index(self) -> usize {
        self as usize
    }

    /// Column / job label (`ui.job.bar` → "Bar").
    pub fn name_key(self) -> &'static str {
        match self {
            Job::Bar => "ui.job.bar",
            Job::Kitchen => "ui.job.kitchen",
            Job::Register => "ui.job.register",
            Job::Cleaning => "ui.job.cleaning",
            Job::Serving => "ui.job.serving",
        }
    }

    /// Skill label (`ui.skill.bar` → "Barista").
    pub fn skill_key(self) -> &'static str {
        match self {
            Job::Bar => "ui.skill.bar",
            Job::Kitchen => "ui.skill.kitchen",
            Job::Register => "ui.skill.register",
            Job::Cleaning => "ui.skill.cleaning",
            Job::Serving => "ui.skill.serving",
        }
    }
}

/// Staff needs, 0–100 each (hunger: 100 = full).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum Need {
    Energy,
    Mood,
    Hunger,
}

impl Need {
    pub const ALL: [Need; 3] = [Need::Energy, Need::Mood, Need::Hunger];

    pub fn name_key(self) -> &'static str {
        match self {
            Need::Energy => "ui.need.energy",
            Need::Mood => "ui.need.mood",
            Need::Hunger => "ui.need.hunger",
        }
    }
}

/// Time of day written as `"HH:MM"` in data.
///
/// Ordering follows the *night*: the "game day" starts at 12:00, so
/// `21:50 < 22:00 < 23:59 < 00:00 < 03:33 < 06:00`. Times before 22:00 belong to the
/// day-planning phase that precedes the night.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct ClockTime {
    /// Minute of the day, 0..1440.
    pub minute_of_day: u16,
}

impl ClockTime {
    pub const OPEN: ClockTime = ClockTime::new(22, 0);
    pub const CLOSE: ClockTime = ClockTime::new(6, 0);

    pub const fn new(hour: u16, minute: u16) -> ClockTime {
        ClockTime {
            minute_of_day: (hour % 24) * 60 + minute % 60,
        }
    }

    pub fn parse(text: &str) -> Result<ClockTime, String> {
        let (h, m) = text
            .split_once(':')
            .ok_or_else(|| format!("time `{text}` must be HH:MM"))?;
        let h: u16 = h.parse().map_err(|_| format!("bad hour in `{text}`"))?;
        let m: u16 = m.parse().map_err(|_| format!("bad minute in `{text}`"))?;
        if h > 23 || m > 59 {
            return Err(format!("time `{text}` out of range"));
        }
        Ok(ClockTime::new(h, m))
    }

    pub fn hour(self) -> u16 {
        self.minute_of_day / 60
    }

    pub fn minute(self) -> u16 {
        self.minute_of_day % 60
    }

    /// Minutes since 12:00 — the sort key of a game day (22:00 = 600, 06:00 = 1080).
    pub fn night_order(self) -> u16 {
        (self.minute_of_day + 1440 - 720) % 1440
    }

    /// Minutes since opening (22:00 = 0, 06:00 = 480). Negative before opening.
    pub fn minutes_since_open(self) -> i32 {
        self.night_order() as i32 - 600
    }

    /// Inverse of [`ClockTime::minutes_since_open`] — matches `clock::Clock::minutes_since_open`.
    pub fn from_minutes_since_open(minutes: u32) -> ClockTime {
        let m = (22 * 60 + minutes) % 1440;
        ClockTime {
            minute_of_day: m as u16,
        }
    }

    /// `from <= self < to` in night order (works across midnight).
    pub fn is_between(self, from: ClockTime, to: ClockTime) -> bool {
        let t = self.night_order();
        from.night_order() <= t && t < to.night_order()
    }

    /// Index of the opening hour this time falls in: 22:xx → 0, 23:xx → 1, 00:xx → 2 … 05:xx → 7.
    /// `None` outside opening hours. Used with [`GuestArchetype::hour_weights`].
    pub fn open_hour_index(self) -> Option<usize> {
        let m = self.minutes_since_open();
        (0..480).contains(&m).then_some((m / 60) as usize)
    }
}

impl TryFrom<String> for ClockTime {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        ClockTime::parse(&value)
    }
}

impl From<ClockTime> for String {
    fn from(t: ClockTime) -> String {
        t.to_string()
    }
}

impl PartialOrd for ClockTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ClockTime {
    fn cmp(&self, other: &Self) -> Ordering {
        self.night_order().cmp(&other.night_order())
    }
}

impl fmt::Display for ClockTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}", self.hour(), self.minute())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum Weather {
    Clear,
    Rain,
    Downpour,
    Fog,
    Storm,
    Snow,
}

impl Weather {
    pub const ALL: [Weather; 6] = [
        Weather::Clear,
        Weather::Rain,
        Weather::Downpour,
        Weather::Fog,
        Weather::Storm,
        Weather::Snow,
    ];

    pub fn name_key(self) -> &'static str {
        match self {
            Weather::Clear => "ui.weather.clear",
            Weather::Rain => "ui.weather.rain",
            Weather::Downpour => "ui.weather.downpour",
            Weather::Fog => "ui.weather.fog",
            Weather::Storm => "ui.weather.storm",
            Weather::Snow => "ui.weather.snow",
        }
    }

    /// Rain, downpour and storm all count as "a rainy night" (Lori, `Raining` situations).
    pub fn is_wet(self) -> bool {
        matches!(self, Weather::Rain | Weather::Downpour | Weather::Storm)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl Weekday {
    pub const ALL: [Weekday; 7] = [
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
        Weekday::Sat,
        Weekday::Sun,
    ];

    /// `ui.weekday.fri` (short, "Fri") and `ui.weekday.fri.long` ("Friday").
    pub fn short_key(self) -> String {
        format!("ui.weekday.{}", self.id())
    }

    pub fn long_key(self) -> String {
        format!("ui.weekday.{}.long", self.id())
    }

    pub fn id(self) -> &'static str {
        match self {
            Weekday::Mon => "mon",
            Weekday::Tue => "tue",
            Weekday::Wed => "wed",
            Weekday::Thu => "thu",
            Weekday::Fri => "fri",
            Weekday::Sat => "sat",
            Weekday::Sun => "sun",
        }
    }

    pub fn next(self) -> Weekday {
        Weekday::ALL[(self as usize + 1) % 7]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum GameMode {
    Story,
    Sandbox,
}

/// Which modes a definition is used in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
pub enum Modes {
    Story,
    Sandbox,
    #[default]
    Both,
}

impl Modes {
    pub fn allows(self, mode: GameMode) -> bool {
        matches!(
            (self, mode),
            (Modes::Both, _)
                | (Modes::Story, GameMode::Story)
                | (Modes::Sandbox, GameMode::Sandbox)
        )
    }
}

/// Look hints for the render agent. Lists are *options*: generators pick one with the seed,
/// a unique character has exactly one entry. Colours are `#RRGGBB`.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Look {
    /// `slim`, `average`, `broad`, `small`.
    pub body: String,
    /// Relative height, 1.0 ≈ 1.3 tiles.
    pub height: f32,
    pub skin: Vec<String>,
    /// Hair styles: `short`, `bob`, `long`, `bun`, `ponytail`, `curly`, `buzz`, `bald`, `grey_short`, `braid`, `beanie`.
    pub hair: Vec<String>,
    pub hair_colors: Vec<String>,
    /// Outfit style: `apron`, `coat`, `scrubs`, `hoodie`, `uniform`, `jacket`, `cardigan`, `raincoat`, `hi_vis`, `suit`.
    pub outfit: String,
    pub outfit_colors: Vec<String>,
    /// Optional props: `glasses`, `headphones`, `scarf`, `cap`, `laptop`, `guitar`, `suitcase`,
    /// `crossword`, `drone_controller`, `badge_shiftella`, `umbrella`, `old_coat`, `knife_roll`.
    pub accessories: Vec<String>,
}

impl Default for Look {
    fn default() -> Self {
        Look {
            body: "average".into(),
            height: 1.0,
            skin: Vec::new(),
            hair: Vec::new(),
            hair_colors: Vec::new(),
            outfit: "jacket".into(),
            outfit_colors: Vec::new(),
            accessories: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Traits
// ---------------------------------------------------------------------------------------------

/// How the trait reads to the player (chip colour on the crew card).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum TraitGroup {
    Positive,
    Mixed,
    Negative,
}

/// When a trait effect applies. Continuous situations apply while true;
/// one-shot ones (`ShiftStart`, `BadEvent`, `MetGuest333`, `RomanceEnded`) apply once when they happen.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum Situation {
    /// Clock time ≥ the given time (night order, so `"00:00"` means "after midnight").
    TimeFrom(ClockTime),
    /// Clock time < the given time (night order).
    TimeBefore(ClockTime),
    ShiftStart,
    /// The staff member is serving / talking to guests.
    Serving,
    QueueAtLeast(u32),
    QueueAtMost(u32),
    /// No other staff in the same room.
    AloneInRoom,
    /// At least N other staff within 3 tiles.
    StaffNearbyAtLeast(u32),
    /// Dirt of the current room above the fraction (0–1).
    DirtAbove(f32),
    MusicOn,
    MusicOff,
    Raining,
    /// A `Bad` event just fired.
    BadEvent,
    /// Saw the 3:33 guest / any uncanny guest.
    MetGuest333,
    /// Their staff romance has ended (lasts 2 nights).
    RomanceEnded,
}

/// Who a relationship modifier applies to.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum RelTarget {
    All,
    /// Colleagues having this trait.
    Trait(String),
}

/// A trait's effect on its owner. Multipliers stack multiplicatively, adds stack additively.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum TraitEffect {
    /// Flat skill points (0–10 scale) for one job.
    Skill { job: Job, add: f32 },
    /// Work speed multiplier; `job: None` = every job.
    Speed {
        job: Option<Job>,
        mul: f32,
        #[serde(default)]
        when: Option<Situation>,
    },
    /// Multiplies how fast a need drains (energy, hunger) or recovers (mood: drift speed).
    NeedRate {
        need: Need,
        mul: f32,
        #[serde(default)]
        when: Option<Situation>,
    },
    /// Mood change. For continuous situations: per game hour. For one-shot situations: once.
    Mood { add: f32, when: Situation },
    /// Mood never drops below this value.
    MoodFloor(f32),
    /// Multiplies mood *penalties* from bad events (0.5 = half).
    BadEventMoodMul(f32),
    /// Guest satisfaction bonus (0–100 scale) on guests this person served.
    GuestSatisfaction {
        add: f32,
        #[serde(default)]
        when: Option<Situation>,
    },
    /// Bonus satisfaction from guests with high patience ("real character").
    PatientGuestSatisfaction(f32),
    /// Order quality bonus (0–100): raises tips and reviews.
    OrderQuality(f32),
    /// Chance per finished action of a mishap (fires `broken_dishes`).
    Mishap {
        chance: f32,
        #[serde(default)]
        when: Option<Situation>,
    },
    /// Multiplies the chance of mistakes.
    ErrorChance {
        mul: f32,
        #[serde(default)]
        when: Option<Situation>,
    },
    /// Relationship change per hour of working near colleagues of the target.
    Relationship { with: RelTarget, per_hour: f32 },
    /// Nearby colleagues get this much mood per hour.
    AuraMood(f32),
    /// Can comfort a sad guest (+satisfaction).
    ComfortGuest(f32),
    /// Romance threshold override (default 80).
    RomanceThreshold(f32),
    /// Break length multiplier.
    BreakLength(f32),
    /// Skips breaks while the queue is longer than N.
    SkipsBreakIfQueue(u32),
    /// Notices uncanny guests and the 3:33 guest.
    SeesUncanny,
    /// Chance to learn about a coming event `minutes` early (a hint toast).
    Foresight { chance: f32, minutes: u32 },
    /// Cleans between orders without being told.
    AutoClean,
    /// Turns the radio / jukebox on by themselves.
    PlaysMusic,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TraitDef {
    pub id: String,
    pub group: TraitGroup,
    /// Never rolled together on one person (should be symmetric).
    #[serde(default)]
    pub conflicts: Vec<String>,
    /// Allowed on one person, but colleagues with these traits lose relationship (−2/h together).
    #[serde(default)]
    pub irritated_by: Vec<String>,
    pub effects: Vec<TraitEffect>,
}

impl TraitDef {
    pub fn name_key(&self) -> String {
        format!("trait.{}.name", self.id)
    }
    pub fn desc_key(&self) -> String {
        format!("trait.{}.desc", self.id)
    }
}

// ---------------------------------------------------------------------------------------------
// Staff
// ---------------------------------------------------------------------------------------------

/// Skill values 0–10 for the five jobs.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(default)]
pub struct Skills {
    pub bar: f32,
    pub kitchen: f32,
    pub register: f32,
    pub cleaning: f32,
    pub serving: f32,
}

impl Skills {
    pub fn get(&self, job: Job) -> f32 {
        match job {
            Job::Bar => self.bar,
            Job::Kitchen => self.kitchen,
            Job::Register => self.register,
            Job::Cleaning => self.cleaning,
            Job::Serving => self.serving,
        }
    }

    pub fn set(&mut self, job: Job, value: f32) {
        let v = value.clamp(0.0, 10.0);
        match job {
            Job::Bar => self.bar = v,
            Job::Kitchen => self.kitchen = v,
            Job::Register => self.register = v,
            Job::Cleaning => self.cleaning = v,
            Job::Serving => self.serving = v,
        }
    }

    /// Work speed formula from the design: `0.5 + skill × 0.1`.
    pub fn speed(&self, job: Job) -> f32 {
        0.5 + self.get(job) * 0.1
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Gender {
    Female,
    Male,
    /// Pick either with the seed.
    Any,
}

/// A hireable staff template. In Story mode the `story_*` person is offered; in Sandbox
/// the generator takes the archetype, a random name, ±`skill_spread` on skills and 2 traits.
#[derive(Debug, Clone, Deserialize)]
pub struct StaffArchetype {
    pub id: String,
    /// Main job (the "favourite work" heart on the crew card).
    pub role: Job,
    pub skills: Skills,
    #[serde(default = "default_spread")]
    pub skill_spread: f32,
    /// Generator picks 2 traits from here (never two that conflict).
    pub trait_pool: Vec<String>,
    /// Fixed traits of the story person.
    pub story_traits: Vec<String>,
    pub story_name: String,
    pub story_age: u32,
    pub gender: Gender,
    /// Age range for generated people.
    pub age: (u32, u32),
    /// Wage in CZK per night (8 h).
    pub wage: u32,
    /// Story: first night the person is offered for hire.
    pub hire_from_night: u32,
    /// Name pools the generator draws from (ids of [`NamePool`]).
    pub name_pools: Vec<String>,
    pub look: Look,
    /// Hands-on: the duty the hire card proposes (one click accepts it).
    #[serde(default)]
    pub default_duty: Option<super::handson::Duty>,
    /// Hands-on: the story person's rozpis preferences (1–3; quotes are i18n keys).
    #[serde(default)]
    pub prefs: Vec<super::handson::Pref>,
}

fn default_spread() -> f32 {
    2.0
}

impl StaffArchetype {
    /// Short role title ("Night barista").
    pub fn role_key(&self) -> String {
        format!("staff.{}.role", self.id)
    }
    /// Story bio of the story person.
    pub fn bio_key(&self) -> String {
        format!("staff.{}.bio", self.id)
    }
    pub fn likes_key(&self) -> String {
        format!("staff.{}.likes", self.id)
    }
    pub fn dislikes_key(&self) -> String {
        format!("staff.{}.dislikes", self.id)
    }
}

/// Names are not translated. Surnames in `female_last` are the feminine forms
/// (Nováková); when empty, `male_last` is used for everyone (international names).
#[derive(Debug, Clone, Deserialize)]
pub struct NamePool {
    pub id: String,
    pub female_first: Vec<String>,
    pub male_first: Vec<String>,
    pub male_last: Vec<String>,
    #[serde(default)]
    pub female_last: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StaffFile {
    pub archetypes: Vec<StaffArchetype>,
    pub name_pools: Vec<NamePool>,
}

// ---------------------------------------------------------------------------------------------
// Menu
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum MenuCategory {
    Drink,
    Food,
    Dessert,
    Special,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MenuItem {
    pub id: String,
    pub category: MenuCategory,
    /// Sale price, CZK.
    pub price: u32,
    /// Ingredient cost, CZK (paid at the day-phase restock).
    pub cost: u32,
    pub job: Job,
    /// Preparation time in game minutes at skill speed 1.0.
    pub prep_minutes: f32,
    /// Furniture tag of the station needed (`station:coffee`, `station:stove`, `station:oven`, `station:fridge`).
    pub station: String,
    /// Minimum skill of `job` to make it at all.
    #[serde(default)]
    pub min_skill: f32,
    /// Story: first night it can be on the menu. Sandbox: ignored (reputation tiers decide).
    #[serde(default = "one")]
    pub unlock_night: u32,
    /// Additionally needs this story flag (e.g. a recipe from a note).
    #[serde(default)]
    pub unlock_flag: Option<String>,
    /// Extra guest satisfaction when served (0–100 scale).
    #[serde(default)]
    pub mood_bonus: f32,
    /// False = can still be sold during a power outage (cold items).
    #[serde(default = "yes")]
    pub needs_power: bool,
    /// Sold "to go": guest doesn't sit.
    #[serde(default)]
    pub to_go: bool,
}

fn one() -> u32 {
    1
}

fn yes() -> bool {
    true
}

impl MenuItem {
    pub fn name_key(&self) -> String {
        format!("menu.{}.name", self.id)
    }
    pub fn desc_key(&self) -> String {
        format!("menu.{}.desc", self.id)
    }
}

// ---------------------------------------------------------------------------------------------
// Guests
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct GuestArchetype {
    pub id: String,
    /// Relative arrival weight per opening hour: index 0 = 22:00–23:00 … 7 = 05:00–06:00.
    pub hour_weights: Vec<f32>,
    /// Arrival multiplier per weekday (Mon..Sun, 7 values). Empty = 1.0 every day.
    #[serde(default)]
    pub weekday_mult: Vec<f32>,
    /// Menu ids, most likely first.
    pub favourites: Vec<String>,
    /// How long they wait for an order before leaving unhappy (game minutes).
    pub patience_minutes: f32,
    /// Multiplier on menu prices they pay (and how much they order).
    pub spend_mult: f32,
    /// Chance of tipping when satisfied (0–1).
    pub tip_chance: f32,
    /// Group size range (inclusive).
    pub group: (u32, u32),
    /// How long they stay after being served (game minutes, inclusive range).
    pub stay_minutes: (u32, u32),
    /// Sandbox institution that enables this pool (`None` = always present).
    #[serde(default)]
    pub institution: Option<String>,
    /// Use the Shiftella app themselves (flavour lines, `shiftella_fan` reviews).
    #[serde(default)]
    pub uses_shiftella: bool,
    /// Review weight multiplier (medics are grateful: 1.5).
    #[serde(default = "one_f")]
    pub review_mult: f32,
    pub look: Look,
    /// Hands-on: jukebox record ids they like (+satisfaction while playing).
    #[serde(default)]
    pub music_likes: Vec<String>,
    /// Hands-on: record ids they dislike (−satisfaction, gentle).
    #[serde(default)]
    pub music_dislikes: Vec<String>,
    /// Hands-on (SHOULD): arrival multiplier per weather (missing = 1.0).
    #[serde(default)]
    pub weather_mult: Vec<(Weather, f32)>,
}

fn one_f() -> f32 {
    1.0
}

impl GuestArchetype {
    pub fn name_key(&self) -> String {
        format!("guest.{}.name", self.id)
    }
    pub fn desc_key(&self) -> String {
        format!("guest.{}.desc", self.id)
    }
    /// Arrival weight for an opening hour index (see [`ClockTime::open_hour_index`]).
    pub fn weight_at(&self, hour_index: usize) -> f32 {
        self.hour_weights.get(hour_index).copied().unwrap_or(0.0)
    }
    /// Arrival multiplier for a weather (1.0 when not listed).
    pub fn weather_mul(&self, weather: Weather) -> f32 {
        self.weather_mult
            .iter()
            .find(|(w, _)| *w == weather)
            .map(|(_, m)| *m)
            .unwrap_or(1.0)
    }
}

// ---------------------------------------------------------------------------------------------
// Furniture
// ---------------------------------------------------------------------------------------------

/// Build-menu tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum BuildCategory {
    Furniture,
    Appliances,
    Counters,
    Decor,
    Floors,
    Structure,
}

impl BuildCategory {
    pub const ALL: [BuildCategory; 6] = [
        BuildCategory::Furniture,
        BuildCategory::Appliances,
        BuildCategory::Counters,
        BuildCategory::Decor,
        BuildCategory::Floors,
        BuildCategory::Structure,
    ];

    pub fn name_key(self) -> &'static str {
        match self {
            BuildCategory::Furniture => "ui.build.tab.furniture",
            BuildCategory::Appliances => "ui.build.tab.appliances",
            BuildCategory::Counters => "ui.build.tab.counters",
            BuildCategory::Decor => "ui.build.tab.decor",
            BuildCategory::Floors => "ui.build.tab.floors",
            BuildCategory::Structure => "ui.build.tab.structure",
        }
    }
}

/// Sprite ids the render agent draws (contract). Furniture `sprite` must be one of these.
pub const SPRITES: &[&str] = &[
    "table_round_2",
    "table_square_4",
    "chair",
    "stool",
    "bar_counter",
    "counter_corner",
    "coffee_machine",
    "espresso_grinder",
    "stove",
    "oven",
    "sink",
    "fridge",
    "dish_rack",
    "register",
    "menu_holo_board",
    "plant_small",
    "plant_tall",
    "floor_lamp",
    "pendant_lamp",
    "sofa",
    "armchair",
    "bookshelf",
    "jukebox",
    "rug",
    "trash_bin",
    "coat_rack",
    "window_seat",
    "drone_dock",
    "grandma_photo",
    "radio",
    "floor_wood",
    "floor_tile",
    "floor_carpet",
    "wall",
    "window",
    "door",
];

/// Known furniture tags. Tags with a `station:` prefix are cooking stations for menu items.
pub const FURNITURE_TAGS: &[&str] = &[
    "seat",
    "table",
    "counter",
    "station:coffee",
    "station:stove",
    "station:oven",
    "station:fridge",
    "register",
    "sink",
    "dishes",
    "light",
    "decor",
    "music",
    "comfort",
    "quiet_corner",
    "stage",
    "window_spot",
    "drone",
    "trash",
    "coats",
    "floor",
    "wall",
    "window",
    "door",
    "power_socket",
    "grandma",
];

#[derive(Debug, Clone, Deserialize)]
pub struct FurnitureDef {
    pub id: String,
    /// Render sprite id (one of [`SPRITES`]).
    pub sprite: String,
    pub category: BuildCategory,
    /// Footprint in tiles (w, h) before rotation.
    pub size: (u32, u32),
    /// Price in CZK (demolish refunds 50 %). Floors: per tile.
    pub price: u32,
    pub tags: Vec<String>,
    /// Comfort for guests sitting on / next to it (0–10).
    #[serde(default)]
    pub comfort: f32,
    /// Beauty of the room around it (−5..10).
    #[serde(default)]
    pub beauty: f32,
    #[serde(default)]
    pub walkable: bool,
    /// Light radius in tiles (only with the `light` tag).
    #[serde(default)]
    pub light_radius: f32,
    /// Seats provided (chairs 1, sofa 2, window seat 2).
    #[serde(default)]
    pub seats: u32,
    #[serde(default = "one")]
    pub unlock_night: u32,
    /// Needs a story flag to appear in the build menu.
    #[serde(default)]
    pub unlock_flag: Option<String>,
    /// Can be bought only once (grandma's photo, radio).
    #[serde(default)]
    pub unique: bool,
    /// Wear per night of use, 0–1 (appliances; `coffee_machine` event when > 0.5).
    #[serde(default)]
    pub wear_per_night: f32,
    /// Stops working during a power outage (the jukebox).
    #[serde(default)]
    pub needs_power: bool,
    /// Render variant hint for pieces that reuse a contract sprite (`fairy_lights` on a
    /// `pendant_lamp`); the render draws `sprite` when it doesn't know the skin.
    #[serde(default)]
    pub skin: Option<String>,
}

impl FurnitureDef {
    pub fn name_key(&self) -> String {
        format!("furniture.{}.name", self.id)
    }
    pub fn desc_key(&self) -> String {
        format!("furniture.{}.desc", self.id)
    }
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }
}

// ---------------------------------------------------------------------------------------------
// Regulars (guests with arcs)
// ---------------------------------------------------------------------------------------------

/// How a regular pays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
pub enum Pays {
    #[default]
    Normal,
    /// Exact change, never a tip.
    Exact,
    /// Old Czechoslovak coins: 0 CZK, gives the `old_coin` item.
    OldCoins,
    /// Plays a song instead of paying.
    Song,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Visit {
    pub from: ClockTime,
    pub to: ClockTime,
    /// Chance the visit happens on a given night.
    #[serde(default = "one_f")]
    pub chance: f32,
}

/// One stage of a regular's arc.
///
/// The arc counter is `arc.<regular>` (starts at 0). Each night, while the regular visits,
/// the sim checks the **next** stage (`current + 1`): if `night ∈ nights` and every condition
/// holds, the arc advances, `effects` apply, and any cutscene with trigger
/// `ArcStage { regular, stage }` plays. `lines` are ambient speech-bubble keys used while the
/// regular is at this stage.
#[derive(Debug, Clone, Deserialize)]
pub struct ArcStage {
    pub stage: u32,
    /// Inclusive night range in which this stage can be reached.
    pub nights: (u32, u32),
    #[serde(default)]
    pub conditions: Vec<Condition>,
    /// Hands-on conditions (`Cond`: core or extension, e.g. `WishMatchStreak`,
    /// `ServedRecipe`); all of `conditions` and `when` must hold.
    #[serde(default)]
    pub when: Vec<super::handson::Cond>,
    #[serde(default)]
    pub lines: Vec<String>,
    #[serde(default)]
    pub effects: Vec<Effect>,
}

/// A one-liner shown in a small speech bubble when the player hovers / clicks a regular.
///
/// Selection: among lines with a non-empty `when` whose conditions all hold, pick one at random;
/// if none hold, pick among the lines with an empty `when` (the fallbacks).
#[derive(Debug, Clone, Deserialize)]
pub struct AmbientLine {
    pub key: String,
    #[serde(default)]
    pub when: Vec<Condition>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegularDef {
    pub id: String,
    /// Full name (not translated).
    pub full_name: String,
    pub age: u32,
    /// Guest archetype used for default behaviour.
    pub base: String,
    pub first_night: u32,
    /// Leaves the story for good after this night (`None` = stays to the end).
    #[serde(default)]
    pub last_night: Option<u32>,
    pub visits: Vec<Visit>,
    /// Only comes in this weather (empty = any).
    #[serde(default)]
    pub weather: Vec<Weather>,
    pub orders: Vec<String>,
    pub patience_minutes: f32,
    #[serde(default)]
    pub pays: Pays,
    /// Preferred seat: a furniture tag (`window_spot`, `power_socket`, `quiet_corner`) or `None`.
    #[serde(default)]
    pub seat: Option<String>,
    /// Never comes when any of these hold (e.g. light off, franchise accepted).
    #[serde(default)]
    pub absent_if: Vec<Condition>,
    pub look: Look,
    pub stages: Vec<ArcStage>,
    /// Story: comes only on these nights (empty = every night from `first_night`).
    /// Used for cameo guests such as `guest_anwar`.
    #[serde(default)]
    pub story_nights: Vec<u32>,
    /// Sandbox: chance per night to drop in as a rare regular (0 = story only).
    #[serde(default)]
    pub sandbox_chance: f32,
    /// Sandbox: comes only on these nights of a save (empty = governed by `sandbox_chance`).
    #[serde(default)]
    pub sandbox_nights: Vec<u32>,
    /// Context mood lines for the hover/click speech bubble (see [`AmbientLine`]).
    #[serde(default)]
    pub ambient_lines: Vec<AmbientLine>,
    /// Render preset id when the render agent draws this character specially (e.g. `anwar`).
    #[serde(default)]
    pub look_preset: Option<String>,
}

impl RegularDef {
    /// Display name (cast-style, e.g. "Mirek" / "Paní Vera").
    pub fn name_key(&self) -> String {
        format!("regular.{}.name", self.id)
    }
    pub fn bio_key(&self) -> String {
        format!("regular.{}.bio", self.id)
    }
    pub fn arc_counter(&self) -> String {
        format!("arc.{}", self.id)
    }
    /// Does this regular come on sandbox night `night` (before the `sandbox_chance` roll)?
    /// `Some(true)` = always, `Some(false)` = never, `None` = roll `sandbox_chance`.
    pub fn sandbox_visit_rule(&self, night: u32) -> Option<bool> {
        if !self.sandbox_nights.is_empty() {
            Some(self.sandbox_nights.contains(&night))
        } else if self.sandbox_chance <= 0.0 {
            Some(false)
        } else {
            None
        }
    }

    /// Does this regular come on story night `night` (ignores weather / `absent_if`)?
    pub fn visits_story_night(&self, night: u32) -> bool {
        night >= self.first_night
            && self.last_night.is_none_or(|last| night <= last)
            && (self.story_nights.is_empty() || self.story_nights.contains(&night))
    }

    pub fn stage(&self, stage: u32) -> Option<&ArcStage> {
        self.stages.iter().find(|s| s.stage == stage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_time_orders_by_night() {
        let t = |s: &str| ClockTime::parse(s).unwrap();
        assert!(t("21:50") < t("22:00"));
        assert!(t("23:59") < t("00:00"));
        assert!(t("03:33") < t("06:00"));
        assert_eq!(t("22:00").minutes_since_open(), 0);
        assert_eq!(t("06:00").minutes_since_open(), 480);
        assert_eq!(ClockTime::from_minutes_since_open(333), t("03:33"));
        assert!(t("00:30").is_between(t("23:00"), t("01:00")));
        assert_eq!(t("05:40").open_hour_index(), Some(7));
        assert_eq!(t("12:00").open_hour_index(), None);
        assert!(ClockTime::parse("24:00").is_err());
        assert_eq!(t("03:33").to_string(), "03:33");
    }

    #[test]
    fn job_ids_and_keys_line_up() {
        for (i, job) in Job::ALL.iter().enumerate() {
            assert_eq!(job.index(), i);
            assert!(job.name_key().ends_with(job.id()));
        }
        assert!(Modes::Both.allows(GameMode::Sandbox));
        assert!(!Modes::Story.allows(GameMode::Sandbox));
    }
}
