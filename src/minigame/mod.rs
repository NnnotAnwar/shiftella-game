//! # Brewing mini-games (`design/handson.md` §2)
//!
//! Short, tactile, never-failable moments behind the counter. Each game runs in a
//! 520×340 Shiftella glass card at the bottom centre of the screen (the café is dimmed
//! 35 % behind it) and reports a quality 0..1 (skip = 0.7, Relaxed floor 0.5).
//!
//! ```ignore
//! // when the player picks a recipe ("By hand"):
//! let params = MiniGameParams::new(seed)
//!     .variant(Variation::from_variant_id(recipe.minigame, &variant_id))
//!     .relaxed(settings.relaxed_minigames)
//!     .regular(Some("vera"))
//!     .practised(book.is_practised(recipe));
//! let mut mg = MiniGame::start(recipe.minigame, params)
//!     .with_title(i18n.tr(lang, &recipe.name_key))
//!     .with_guest(guest_name, i18n.tr(lang, &wish.text_key))
//!     .with_text(MiniGameText::from_lookup(|k| i18n.tr(lang, k).to_string()));
//! // every frame, after `ui.begin_frame()` and before game hotkeys:
//! if let MiniGameStatus::Done { quality } = mg.update(&mut ui, dt) {
//!     sim.apply(Command::PlayerBrew { guest, recipe, quality });
//! }
//! mg.draw(&mut ui, ui.screen());                 // after the world, in the UI camera
//! for s in mg.drain_sounds() { let c = s.cue(); audio.play_id(c.id, c.volume) }
//! let loops = mg.loops();                        // steam hiss / pour trickle volumes (§14)
//! ```
//!
//! Controls (§2.0): mouse first; **Space** = hold / primary, **arrows / WASD** = direction
//! (LatteArt wobble taps, PourOver beat taps, Plate moving), **Enter** = done, **Tab** = skip
//! (in Plate Tab cycles the pieces), **Esc** = skip. `update` consumes the keys it uses.
//! [`MiniGame::step`] takes an explicit [`MgInput`] (scripted demos, tests).
//!
//! Content mapping: [`MiniGameKind`] has the same variant names as the content enum
//! (`MilkFroth, LatteArt, PourOver, TeaSteep, Plate`; SoupStir / MulledSpice are not built
//! yet — use the recipe's `fallback_minigame`). Tuning rows of `minigames.ron` map 1:1 onto
//! [`FrothTuning`], [`PourTuning`], [`TeaTuning`]; latte pattern ids onto [`LatteShape::from_id`].

mod froth;
mod latte;
pub mod latte_shapes;
mod paint;
mod plate;
mod pour;
pub mod score;
mod tea;

use std::cell::Cell;

use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

use crate::ui::{RectExt, Ui, ease_out};

pub use latte_shapes::LatteShape;
pub use score::{Rating, SKIP_QUALITY};

/// Size of the glass card in UI units.
pub const CARD_SIZE: Vec2 = Vec2::new(520.0, 340.0);
/// Size of the play area inside the card; game coordinates are stage-local.
pub const STAGE_SIZE: Vec2 = Vec2::new(492.0, 248.0);
const PAD: f32 = 14.0;
const HEADER_H: f32 = 48.0;
/// Gap between the card and the bottom of the area it is drawn in.
const BOTTOM_MARGIN: f32 = 28.0;
/// How long the quality toast stays before `Done` is reported.
const OUTRO: f32 = 1.2;

/// Which mini-game a recipe uses (same variant names as the content enum).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MiniGameKind {
    MilkFroth,
    LatteArt,
    PourOver,
    TeaSteep,
    Plate,
}

impl MiniGameKind {
    pub const ALL: [MiniGameKind; 5] = [
        MiniGameKind::MilkFroth,
        MiniGameKind::LatteArt,
        MiniGameKind::PourOver,
        MiniGameKind::TeaSteep,
        MiniGameKind::Plate,
    ];

    /// snake_case id, as in `minigames.ron` and the i18n keys.
    pub fn id(self) -> &'static str {
        match self {
            MiniGameKind::MilkFroth => "milk_froth",
            MiniGameKind::LatteArt => "latte_art",
            MiniGameKind::PourOver => "pour_over",
            MiniGameKind::TeaSteep => "tea_steep",
            MiniGameKind::Plate => "plate",
        }
    }

    /// Accepts the ids above plus short aliases (`froth`, `latte`, `pour`, `tea`).
    pub fn from_id(id: &str) -> Option<MiniGameKind> {
        Some(match id {
            "milk_froth" | "froth" => MiniGameKind::MilkFroth,
            "latte_art" | "latte" => MiniGameKind::LatteArt,
            "pour_over" | "pour" => MiniGameKind::PourOver,
            "tea_steep" | "tea" => MiniGameKind::TeaSteep,
            "plate" => MiniGameKind::Plate,
            _ => return None,
        })
    }

    /// `minigame.<kind>.title`
    pub fn title_key(self) -> String {
        format!("minigame.{}.title", self.id())
    }
}

/// Which dish is plated in [`MiniGameKind::Plate`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlateSet {
    /// Honey cake slice with cream, berries, mint; sugar-dust garnish.
    Medovnik,
    /// Creamy dill soup (kulajda) with a cream swirl and croutons; dill garnish.
    Soup,
    /// Topinka (fried bread) with garlic, cheese and pickles; onion garnish.
    Topinka,
}

impl PlateSet {
    pub const ALL: [PlateSet; 3] = [PlateSet::Medovnik, PlateSet::Soup, PlateSet::Topinka];
    pub fn id(self) -> &'static str {
        match self {
            PlateSet::Medovnik => "medovnik",
            PlateSet::Soup => "soup",
            PlateSet::Topinka => "topinka",
        }
    }
    pub fn from_id(id: &str) -> Option<PlateSet> {
        PlateSet::ALL.into_iter().find(|s| s.id() == id)
    }
    pub fn key(self) -> String {
        format!("minigame.plate.dish.{}", self.id())
    }
}

/// Tea colour family for [`MiniGameKind::TeaSteep`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TeaKind {
    Black,
    Green,
    Herbal,
    Fruit,
}

impl TeaKind {
    pub const ALL: [TeaKind; 4] = [
        TeaKind::Black,
        TeaKind::Green,
        TeaKind::Herbal,
        TeaKind::Fruit,
    ];
    pub fn id(self) -> &'static str {
        match self {
            TeaKind::Black => "black",
            TeaKind::Green => "green",
            TeaKind::Herbal => "herbal",
            TeaKind::Fruit => "fruit",
        }
    }
    pub fn from_id(id: &str) -> Option<TeaKind> {
        TeaKind::ALL.into_iter().find(|s| s.id() == id)
    }
    pub fn key(self) -> String {
        format!("minigame.tea_steep.kind.{}", self.id())
    }
}

/// A regular's personal wish that nudges the target (chip in the header + small tweak).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RegularTouch {
    /// Froth: more foam.
    ExtraFoam,
    /// Froth: less foam.
    LightFoam,
    /// Tea: stronger; pour-over: more water.
    Strong,
    /// Tea: milder.
    Mild,
    /// Latte art: a dusting of cinnamon over the finished pattern.
    Cinnamon,
    /// Plate: an extra honey drizzle piece.
    Honey,
}

impl RegularTouch {
    pub const ALL: [RegularTouch; 6] = [
        RegularTouch::ExtraFoam,
        RegularTouch::LightFoam,
        RegularTouch::Strong,
        RegularTouch::Mild,
        RegularTouch::Cinnamon,
        RegularTouch::Honey,
    ];
    pub fn id(self) -> &'static str {
        match self {
            RegularTouch::ExtraFoam => "extra_foam",
            RegularTouch::LightFoam => "light_foam",
            RegularTouch::Strong => "strong",
            RegularTouch::Mild => "mild",
            RegularTouch::Cinnamon => "cinnamon",
            RegularTouch::Honey => "honey",
        }
    }
    pub fn from_id(id: &str) -> Option<RegularTouch> {
        RegularTouch::ALL.into_iter().find(|t| t.id() == id)
    }
    pub fn key(self) -> String {
        format!("minigame.touch.{}", self.id())
    }
}

/// `minigames.ron` → `milk_froth` row.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrothTuning {
    /// Target temperature, °C.
    pub t_target: f32,
    /// ± tolerance, °C.
    pub tol: f32,
    /// Target foam 0..1.
    pub foam: f32,
}

impl FrothTuning {
    pub const CAPPUCCINO: FrothTuning = FrothTuning {
        t_target: 62.0,
        tol: 4.0,
        foam: 0.6,
    };
    pub const CHOCOLATE: FrothTuning = FrothTuning {
        t_target: 58.0,
        tol: 6.0,
        foam: 0.3,
    };
    pub const RAF: FrothTuning = FrothTuning {
        t_target: 60.0,
        tol: 4.0,
        foam: 0.5,
    };
}

/// `minigames.ron` → `pour_over` row.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PourTuning {
    /// Metronome, beats per minute (48 = 0.8 Hz).
    pub bpm: f32,
    /// "Let it bloom" wait, seconds.
    pub bloom_s: f32,
}

impl PourTuning {
    pub const CLASSIC: PourTuning = PourTuning {
        bpm: 48.0,
        bloom_s: 2.0,
    };
}

/// `minigames.ron` → `tea_steep` row.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TeaTuning {
    /// Target strength on the 1–5 scale.
    pub target: f32,
    /// ± tolerance on that scale (1.5; Vera's "properly made" divides it by 1.5).
    pub tol: f32,
    pub lemon: bool,
    /// Sugar cubes wanted (0 = no sugar step).
    pub sugar: u8,
}

impl TeaTuning {
    pub const LEMON: TeaTuning = TeaTuning {
        target: 3.0,
        tol: 1.5,
        lemon: true,
        sugar: 0,
    };
    pub const TWO_SUGARS: TeaTuning = TeaTuning {
        target: 3.0,
        tol: 1.5,
        lemon: true,
        sugar: 2,
    };
}

/// What the caller asks for. `None` fields are defaults or picked from the seed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Variation {
    pub shape: Option<LatteShape>,
    pub plate: Option<PlateSet>,
    pub tea_kind: Option<TeaKind>,
    pub froth: Option<FrothTuning>,
    pub pour: Option<PourTuning>,
    pub tea: Option<TeaTuning>,
    pub touch: Option<RegularTouch>,
}

impl Variation {
    pub fn latte(shape: LatteShape) -> Self {
        Variation {
            shape: Some(shape),
            ..Default::default()
        }
    }
    pub fn plate(set: PlateSet) -> Self {
        Variation {
            plate: Some(set),
            ..Default::default()
        }
    }
    pub fn froth(t: FrothTuning) -> Self {
        Variation {
            froth: Some(t),
            ..Default::default()
        }
    }
    pub fn pour(t: PourTuning) -> Self {
        Variation {
            pour: Some(t),
            ..Default::default()
        }
    }
    pub fn tea(t: TeaTuning) -> Self {
        Variation {
            tea: Some(t),
            ..Default::default()
        }
    }
    pub fn tea_kind(mut self, k: TeaKind) -> Self {
        self.tea_kind = Some(k);
        self
    }
    pub fn touch(mut self, t: RegularTouch) -> Self {
        self.touch = Some(t);
        self
    }
    /// Presets for the variant ids of `design/handson.md` §7.1 (`cappuccino`, `chocolate`, `raf`,
    /// `classic`, `lemon`, `two_sugars`, latte pattern ids, plate dish ids). Unknown → default.
    pub fn from_variant_id(kind: MiniGameKind, id: &str) -> Self {
        match kind {
            MiniGameKind::MilkFroth => Variation::froth(match id {
                "chocolate" => FrothTuning::CHOCOLATE,
                "raf" => FrothTuning::RAF,
                _ => FrothTuning::CAPPUCCINO,
            }),
            MiniGameKind::PourOver => Variation::pour(PourTuning::CLASSIC),
            MiniGameKind::TeaSteep => Variation::tea(match id {
                "two_sugars" => TeaTuning::TWO_SUGARS,
                _ => TeaTuning::LEMON,
            }),
            MiniGameKind::LatteArt => Variation {
                shape: LatteShape::from_id(id),
                ..Default::default()
            },
            MiniGameKind::Plate => Variation {
                plate: PlateSet::from_id(id),
                ..Default::default()
            },
        }
    }
}

/// `start(kind, MiniGameParams { seed, variant, relaxed, regular })` (§2.0).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MiniGameParams {
    pub seed: u64,
    pub variant: Variation,
    /// `settings.relaxed_minigames`: tolerance ×1.6, quality floor 0.5, no auto-end (Done
    /// button), guides stay visible.
    pub relaxed: bool,
    /// Regular's id (e.g. `"vera"` → stricter tea).
    pub regular: Option<String>,
    /// Practised recipe: a thin sparkle on the sweet spot.
    pub practised: bool,
}

impl MiniGameParams {
    pub fn new(seed: u64) -> Self {
        MiniGameParams {
            seed,
            ..Default::default()
        }
    }
    pub fn variant(mut self, v: Variation) -> Self {
        self.variant = v;
        self
    }
    pub fn relaxed(mut self, on: bool) -> Self {
        self.relaxed = on;
        self
    }
    pub fn regular(mut self, id: Option<&str>) -> Self {
        self.regular = id.map(str::to_string);
        self
    }
    pub fn practised(mut self, on: bool) -> Self {
        self.practised = on;
        self
    }
}

/// Result of [`MiniGame::update`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MiniGameStatus {
    Running,
    /// Reported after the quality toast (or right away on skip); stays `Done`.
    Done {
        quality: f32,
    },
}

/// One-shot sound requests. [`MiniGameSound::cue`] maps them onto existing files (§14).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MiniGameSound {
    /// The steam wand opens.
    Froth,
    /// Tea bag lifted, lemon dropped, sugar cube.
    Pour,
    /// Garnish placed, cup set down.
    Clink,
    /// Served (end of a played game).
    Success,
    /// Entering a sweet spot (soft chime).
    Cue,
    /// Metronome tick.
    Tick,
}

/// An existing sound file and its volume.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SoundCue {
    /// Content id (`content::SFX_IDS`).
    pub id: &'static str,
    pub volume: f32,
}

impl MiniGameSound {
    pub const ALL: [MiniGameSound; 6] = [
        MiniGameSound::Froth,
        MiniGameSound::Pour,
        MiniGameSound::Clink,
        MiniGameSound::Success,
        MiniGameSound::Cue,
        MiniGameSound::Tick,
    ];
    /// §14: cues/ticks → `ui_click`, ceramic → `sfx_cup_saucer`.
    pub fn cue(self) -> SoundCue {
        let (id, volume) = match self {
            MiniGameSound::Froth => ("sfx_coffee_machine", 0.3),
            MiniGameSound::Pour => ("sfx_cup_saucer", 0.4),
            MiniGameSound::Clink => ("sfx_cup_saucer", 0.5),
            MiniGameSound::Success => ("sfx_cup_saucer", 0.7),
            MiniGameSound::Cue => ("ui_click", 0.5),
            MiniGameSound::Tick => ("ui_click", 0.15),
        };
        SoundCue { id, volume }
    }
    /// Closest [`crate::audio::Sfx`].
    pub fn suggested_sfx(self) -> crate::audio::Sfx {
        use crate::audio::Sfx;
        match self {
            MiniGameSound::Froth => Sfx::CoffeeMachine,
            MiniGameSound::Pour | MiniGameSound::Clink | MiniGameSound::Success => Sfx::CupSaucer,
            MiniGameSound::Cue | MiniGameSound::Tick => Sfx::UiClick,
        }
    }
}

/// Loop volumes for this frame (§14): `steam` → `sfx_coffee_machine` looped (0.3 → 0.8 with
/// temperature), `trickle` → a second `amb_rain_window_loop` instance (0.25 while pouring).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MiniGameLoops {
    pub steam: f32,
    pub trickle: f32,
}

/// Player-visible strings (from `assets/i18n/src/minigame.tr`).
#[derive(Clone, Debug)]
pub struct MiniGameText {
    lookup: std::collections::HashMap<&'static str, String>,
}

/// Every key the mini-games use (all present in `minigame.tr`).
pub const TEXT_KEYS: &[&str] = &[
    "minigame.milk_froth.title",
    "minigame.latte_art.title",
    "minigame.pour_over.title",
    "minigame.tea_steep.title",
    "minigame.plate.title",
    "minigame.milk_froth.prompt.hold",
    "minigame.milk_froth.prompt.release",
    "minigame.milk_froth.prompt.again",
    "minigame.latte_art.prompt.trace",
    "minigame.latte_art.prompt.keys",
    "minigame.latte_art.prompt.done",
    "minigame.pour_over.prompt.bloom_pour",
    "minigame.pour_over.prompt.bloom",
    "minigame.pour_over.prompt.circles",
    "minigame.pour_over.prompt.keys",
    "minigame.tea_steep.prompt.lift",
    "minigame.tea_steep.prompt.ready",
    "minigame.tea_steep.prompt.lemon",
    "minigame.tea_steep.prompt.sugar",
    "minigame.plate.prompt.ghost",
    "minigame.plate.prompt.drag",
    "minigame.plate.prompt.garnish",
    "minigame.plate.prompt.done",
    "minigame.quality.beautiful",
    "minigame.quality.lovely",
    "minigame.quality.good",
    "minigame.quality.homely",
    "minigame.skip",
    "minigame.done",
    "minigame.key.space",
    "minigame.target",
    "minigame.strength",
    "minigame.water",
    "minigame.sugar",
    "minigame.practised",
    "minigame.touch.extra_foam",
    "minigame.touch.light_foam",
    "minigame.touch.strong",
    "minigame.touch.mild",
    "minigame.touch.cinnamon",
    "minigame.touch.honey",
    "latte.heart",
    "latte.tulip",
    "latte.rosetta",
    "latte.leaf",
    "latte.swan",
    "latte.snowflake",
    "latte.moon",
    "latte.train",
    "latte.cat",
    "latte.note",
    "latte.heart_l",
    "minigame.tea_steep.kind.black",
    "minigame.tea_steep.kind.green",
    "minigame.tea_steep.kind.herbal",
    "minigame.tea_steep.kind.fruit",
    "minigame.plate.dish.medovnik",
    "minigame.plate.dish.soup",
    "minigame.plate.dish.topinka",
];

impl MiniGameText {
    /// Fills every key from `f` (e.g. `|k| i18n.tr(lang, k).to_string()`).
    pub fn from_lookup(mut f: impl FnMut(&str) -> String) -> Self {
        MiniGameText {
            lookup: TEXT_KEYS.iter().map(|k| (*k, f(k))).collect(),
        }
    }
    /// The text for `key`, or the key itself (same fallback as `I18n`).
    pub fn get<'a>(&'a self, key: &'a str) -> &'a str {
        self.lookup.get(key).map(String::as_str).unwrap_or(key)
    }
}

impl Default for MiniGameText {
    fn default() -> Self {
        MiniGameText::from_lookup(|k| k.to_string())
    }
}

/// Input for one step, in stage-local coordinates (see [`STAGE_SIZE`]).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MgInput {
    /// Mouse position relative to the stage's top-left corner.
    pub pointer: Option<Vec2>,
    /// Left button went down inside the stage this frame.
    pub pressed: bool,
    pub down: bool,
    pub released: bool,
    /// Space (hold / primary).
    pub hold: bool,
    pub hold_pressed: bool,
    /// Left / A and Right / D presses (wobble and beat taps).
    pub left: bool,
    pub right: bool,
    /// Arrows / WASD held, each axis −1..1.
    pub axis: Vec2,
    /// Tab in Plate: next piece (elsewhere Tab is skip).
    pub cycle: bool,
    /// Enter or the Done button.
    pub confirm: bool,
    /// Esc / Tab / the Skip button.
    pub skip: bool,
}

/// Tiny deterministic RNG (splitmix64).
#[derive(Clone, Debug)]
pub(crate) struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// 0..1
    pub fn f(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f()
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }
}

/// Shared per-step context handed to the individual games.
pub(crate) struct Ctx<'a> {
    pub t: f32,
    pub relaxed: bool,
    pub sounds: &'a mut Vec<MiniGameSound>,
}

/// What a game shows in the footer.
pub(crate) struct Hint<'a> {
    pub key: &'a str,
    /// Highlighted call to action ("Release!").
    pub emphasis: bool,
    /// Keycaps shown before the text ("Space" is translated).
    pub keys: &'a [&'a str],
    /// Show the Done button.
    pub done: bool,
}

/// Settings shared by all games.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Common {
    pub relaxed: bool,
    pub practised: bool,
}

enum Game {
    Froth(froth::Froth),
    Latte(latte::Latte),
    Pour(pour::Pour),
    Tea(tea::Tea),
    Plate(plate::Plate),
}

struct Finish {
    quality: f32,
    age: f32,
    skipped: bool,
}

struct Sparkle {
    p: Vec2,
    v: Vec2,
    age: f32,
    life: f32,
    size: f32,
    warm: bool,
}

/// One running mini-game. Create with [`MiniGame::start`].
pub struct MiniGame {
    kind: MiniGameKind,
    variation: Variation,
    relaxed: bool,
    practised: bool,
    title: String,
    guest: Option<(String, String)>,
    text: MiniGameText,
    t: f32,
    game: Game,
    finish: Option<Finish>,
    sounds: Vec<MiniGameSound>,
    rng: Rng,
    sparkles: Vec<Sparkle>,
    skip_req: Cell<bool>,
    done_req: Cell<bool>,
    stage_origin: Cell<Option<Vec2>>,
}

impl MiniGame {
    /// Starts a game. `params.seed` picks unspecified parts of the variation and small
    /// details (sweet-spot drift, plate jitter…), so a run is reproducible.
    pub fn start(kind: MiniGameKind, params: MiniGameParams) -> MiniGame {
        let mut rng = Rng::new(params.seed);
        let mut v = params.variant;
        let common = Common {
            relaxed: params.relaxed,
            practised: params.practised,
        };
        let game = match kind {
            MiniGameKind::MilkFroth => {
                let mut tu = *v.froth.get_or_insert(FrothTuning::CAPPUCCINO);
                match v.touch {
                    Some(RegularTouch::ExtraFoam) => tu.foam = (tu.foam + 0.15).min(0.9),
                    Some(RegularTouch::LightFoam) => tu.foam = (tu.foam - 0.15).max(0.15),
                    _ => {}
                }
                Game::Froth(froth::Froth::new(tu, common))
            }
            MiniGameKind::LatteArt => {
                let shape = *v.shape.get_or_insert_with(|| rng.pick(&LatteShape::BASIC));
                Game::Latte(latte::Latte::new(shape, v.touch, common))
            }
            MiniGameKind::PourOver => {
                let tu = *v.pour.get_or_insert(PourTuning::CLASSIC);
                Game::Pour(pour::Pour::new(&mut rng, tu, v.touch, common))
            }
            MiniGameKind::TeaSteep => {
                let kind = *v.tea_kind.get_or_insert_with(|| rng.pick(&TeaKind::ALL));
                let mut tu = *v.tea.get_or_insert(TeaTuning::LEMON);
                match v.touch {
                    Some(RegularTouch::Strong) => tu.target = (tu.target + 1.0).min(4.6),
                    Some(RegularTouch::Mild) => tu.target = (tu.target - 1.0).max(1.4),
                    _ => {}
                }
                if params.regular.as_deref() == Some("vera") {
                    tu.tol /= 1.5;
                }
                Game::Tea(tea::Tea::new(&mut rng, kind, tu, common))
            }
            MiniGameKind::Plate => {
                let set = *v.plate.get_or_insert_with(|| rng.pick(&PlateSet::ALL));
                Game::Plate(plate::Plate::new(&mut rng, set, v.touch, common))
            }
        };
        MiniGame {
            kind,
            variation: v,
            relaxed: params.relaxed,
            practised: params.practised,
            title: String::new(),
            guest: None,
            text: MiniGameText::default(),
            t: 0.0,
            game,
            finish: None,
            sounds: Vec::new(),
            rng,
            sparkles: Vec::new(),
            skip_req: Cell::new(false),
            done_req: Cell::new(false),
            stage_origin: Cell::new(None),
        }
    }

    /// Recipe name shown as the title (already translated).
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }
    /// The guest's name (for the portrait chip) and their wish line (already translated).
    pub fn with_guest(mut self, name: impl Into<String>, wish: impl Into<String>) -> Self {
        self.guest = Some((name.into(), wish.into()));
        self
    }
    pub fn with_text(mut self, text: MiniGameText) -> Self {
        self.text = text;
        self
    }

    pub fn kind(&self) -> MiniGameKind {
        self.kind
    }
    /// The variation with defaults / seed-picked fields filled in.
    pub fn variation(&self) -> Variation {
        self.variation
    }
    pub fn elapsed(&self) -> f32 {
        self.t
    }
    pub fn is_finished(&self) -> bool {
        self.finish.is_some()
    }
    /// Final quality once known (also during the quality toast).
    pub fn quality(&self) -> Option<f32> {
        self.finish.as_ref().map(|f| f.quality)
    }

    /// Requested one-shot sounds since the last call.
    pub fn drain_sounds(&mut self) -> Vec<MiniGameSound> {
        std::mem::take(&mut self.sounds)
    }

    /// Loop volumes for this frame.
    pub fn loops(&self) -> MiniGameLoops {
        if self.finish.is_some() {
            return MiniGameLoops::default();
        }
        match &self.game {
            Game::Froth(g) => MiniGameLoops {
                steam: g.steam_volume(),
                trickle: 0.0,
            },
            Game::Latte(g) => MiniGameLoops {
                steam: 0.0,
                trickle: 0.25 * g.flow(),
            },
            Game::Pour(g) => MiniGameLoops {
                steam: 0.0,
                trickle: 0.25 * g.flow(),
            },
            _ => MiniGameLoops::default(),
        }
    }

    /// Reads mouse / keyboard from the UI and advances the game.
    pub fn update(&mut self, ui: &mut Ui, dt: f32) -> MiniGameStatus {
        let origin = self.stage_origin.get();
        let pointer = origin.map(|o| ui.mouse() - o);
        let in_stage =
            pointer.is_some_and(|p| Rect::new(0.0, 0.0, STAGE_SIZE.x, STAGE_SIZE.y).contains(p));
        let typing = ui.wants_keyboard();
        let key_down = |k: KeyCode| !typing && is_key_down(k);
        let mut axis = Vec2::ZERO;
        if key_down(KeyCode::Left) || key_down(KeyCode::A) {
            axis.x -= 1.0;
        }
        if key_down(KeyCode::Right) || key_down(KeyCode::D) {
            axis.x += 1.0;
        }
        if key_down(KeyCode::Up) || key_down(KeyCode::W) {
            axis.y -= 1.0;
        }
        if key_down(KeyCode::Down) || key_down(KeyCode::S) {
            axis.y += 1.0;
        }
        let tab = ui.take_key(KeyCode::Tab);
        let plate = self.kind == MiniGameKind::Plate;
        let left = ui.take_key(KeyCode::Left) | ui.take_key(KeyCode::A);
        let right = ui.take_key(KeyCode::Right) | ui.take_key(KeyCode::D);
        let input = MgInput {
            pointer,
            pressed: ui.mouse_pressed() && in_stage,
            down: ui.mouse_down(),
            released: ui.mouse_released(),
            hold: key_down(KeyCode::Space),
            hold_pressed: ui.take_key(KeyCode::Space),
            left,
            right,
            axis,
            cycle: tab && plate,
            confirm: ui.enter() || self.done_req.replace(false),
            skip: ui.esc() || (tab && !plate) || self.skip_req.replace(false),
        };
        self.step(&input, dt)
    }

    /// Advances the game with explicit input (demos, tests, attract mode).
    pub fn step(&mut self, input: &MgInput, dt: f32) -> MiniGameStatus {
        let dt = dt.clamp(0.0, 0.1);
        self.t += dt;
        self.step_sparkles(dt);
        if let Some(f) = &mut self.finish {
            f.age += dt;
            return if f.skipped || f.age >= OUTRO {
                MiniGameStatus::Done { quality: f.quality }
            } else {
                MiniGameStatus::Running
            };
        }
        if input.skip {
            self.finish = Some(Finish {
                quality: SKIP_QUALITY,
                age: 0.0,
                skipped: true,
            });
            return MiniGameStatus::Done {
                quality: SKIP_QUALITY,
            };
        }
        let mut ctx = Ctx {
            t: self.t,
            relaxed: self.relaxed,
            sounds: &mut self.sounds,
        };
        let raw = match &mut self.game {
            Game::Froth(g) => g.step(&mut ctx, input, dt),
            Game::Latte(g) => g.step(&mut ctx, input, dt),
            Game::Pour(g) => g.step(&mut ctx, input, dt),
            Game::Tea(g) => g.step(&mut ctx, input, dt),
            Game::Plate(g) => g.step(&mut ctx, input, dt),
        };
        if let Some(q) = raw {
            let quality = score::finalize(q, self.relaxed);
            self.finish = Some(Finish {
                quality,
                age: 0.0,
                skipped: false,
            });
            self.sounds.push(MiniGameSound::Success);
            let focus = self.focus();
            for i in 0..22 {
                let a = i as f32 / 22.0 * std::f32::consts::TAU + self.rng.range(-0.2, 0.2);
                let sp = self.rng.range(40.0, 120.0);
                self.sparkles.push(Sparkle {
                    p: focus + vec2(a.cos(), a.sin()) * self.rng.range(10.0, 40.0),
                    v: vec2(a.cos(), a.sin()) * sp + vec2(0.0, -30.0),
                    age: 0.0,
                    life: self.rng.range(0.7, 1.2),
                    size: self.rng.range(3.0, 7.0),
                    warm: i % 3 != 0,
                });
            }
        }
        MiniGameStatus::Running
    }

    /// Scripted mouse input that plays the game nicely — demos, screenshots, tests.
    pub fn autoplay_input(&self) -> MgInput {
        self.autoplay(false)
    }

    /// Scripted keyboard-only input (Space / arrows / Tab / Enter).
    pub fn autoplay_keyboard_input(&self) -> MgInput {
        self.autoplay(true)
    }

    fn autoplay(&self, keyboard: bool) -> MgInput {
        if self.finish.is_some() {
            return MgInput::default();
        }
        let mut i = match &self.game {
            Game::Froth(g) => g.autoplay(self.t, keyboard),
            Game::Latte(g) => g.autoplay(self.t, keyboard),
            Game::Pour(g) => g.autoplay(self.t, keyboard),
            Game::Tea(g) => g.autoplay(self.t, keyboard),
            Game::Plate(g) => g.autoplay(self.t, keyboard),
        };
        // Relaxed games wait for Done: press it once the game offers it.
        let hint_done = match &self.game {
            Game::Froth(g) => g.hint().done,
            Game::Latte(g) => g.hint().done,
            Game::Pour(g) => g.hint().done,
            Game::Tea(g) => g.hint().done,
            Game::Plate(g) => g.hint().done,
        };
        if self.relaxed && hint_done && self.ready_for_done() {
            i.confirm = true;
        }
        i
    }

    fn ready_for_done(&self) -> bool {
        match &self.game {
            Game::Froth(g) => g.finished_gesture(),
            Game::Latte(g) => g.finished_gesture(),
            Game::Pour(g) => g.finished_gesture(),
            Game::Tea(g) => g.finished_gesture(),
            Game::Plate(g) => g.finished_gesture(),
        }
    }

    fn focus(&self) -> Vec2 {
        match &self.game {
            Game::Froth(g) => g.focus(),
            Game::Latte(g) => g.focus(),
            Game::Pour(g) => g.focus(),
            Game::Tea(g) => g.focus(),
            Game::Plate(g) => g.focus(),
        }
    }

    fn step_sparkles(&mut self, dt: f32) {
        for s in &mut self.sparkles {
            s.age += dt;
            s.p += s.v * dt;
            s.v *= (-2.6 * dt).exp();
            s.v.y += 18.0 * dt;
        }
        self.sparkles.retain(|s| s.age < s.life);
    }

    /// The glass card rect for a given screen area (bottom centre).
    pub fn card_rect(area: Rect) -> Rect {
        let y = (area.y + area.h - CARD_SIZE.y - BOTTOM_MARGIN).max(area.y);
        Rect::new(
            (area.x + (area.w - CARD_SIZE.x) / 2.0).round(),
            y.round(),
            CARD_SIZE.x,
            CARD_SIZE.y,
        )
    }

    /// Dims `rect` by 35 % and draws the card at its bottom centre (UI camera).
    pub fn draw(&self, ui: &mut Ui, rect: Rect) {
        let appear = ease_out(self.t / 0.35);
        draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            Color::new(0.03, 0.02, 0.07, 0.35 * appear),
        );
        let card = Self::card_rect(rect).translate(vec2(0.0, (1.0 - appear) * 14.0));
        let stage = Rect::new(card.x + PAD, card.y + HEADER_H, STAGE_SIZE.x, STAGE_SIZE.y);
        self.stage_origin.set(Some(stage.point()));

        let hint = match &self.game {
            Game::Froth(g) => g.hint(),
            Game::Latte(g) => g.hint(),
            Game::Pour(g) => g.hint(),
            Game::Tea(g) => g.hint(),
            Game::Plate(g) => g.hint(),
        };
        let out = paint::card(ui, card, self, &hint);
        if out.skip {
            self.skip_req.set(true);
        }
        if out.done {
            self.done_req.set(true);
        }

        ui.push_clip(stage);
        paint::stage_backdrop(stage, self.t);
        let o = stage.point();
        match &self.game {
            Game::Froth(g) => g.draw(ui, o, self.t),
            Game::Latte(g) => g.draw(ui, o, self.t, &self.text),
            Game::Pour(g) => g.draw(ui, o, self.t, &self.text),
            Game::Tea(g) => g.draw(ui, o, self.t, &self.text),
            Game::Plate(g) => g.draw(ui, o, self.t),
        }
        paint::stage_vignette(stage);
        for s in &self.sparkles {
            paint::sparkle(o + s.p, s.size, s.age / s.life, s.warm);
        }
        if let Some(f) = &self.finish
            && !f.skipped
        {
            paint::result_chip(ui, stage, f.quality, f.age, &self.text);
        }
        if appear < 1.0 {
            crate::ui::draw::fill_rrect(
                stage,
                10.0,
                paint::with_alpha(paint::STAGE_TOP, 1.0 - appear),
            );
        }
        ui.pop_clip();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(kind: MiniGameKind, p: MiniGameParams, keyboard: bool) -> (f32, f32) {
        let mut mg = MiniGame::start(kind, p);
        let dt = 1.0 / 60.0;
        for _ in 0..(60 * 60) {
            let input = if keyboard {
                mg.autoplay_keyboard_input()
            } else {
                mg.autoplay_input()
            };
            if let MiniGameStatus::Done { quality } = mg.step(&input, dt) {
                return (quality, mg.elapsed());
            }
        }
        panic!("{kind:?} did not finish (keyboard: {keyboard})");
    }

    /// Design durations (§2.1–2.5) plus the 1.2 s quality toast and a little slack.
    fn duration_range(kind: MiniGameKind) -> (f32, f32) {
        match kind {
            MiniGameKind::MilkFroth => (6.0, 12.5),
            MiniGameKind::LatteArt => (8.0, 16.5),
            MiniGameKind::PourOver => (12.0, 20.5),
            MiniGameKind::TeaSteep => (8.0, 16.5),
            MiniGameKind::Plate => (8.0, 17.0),
        }
    }

    #[test]
    fn every_game_plays_well_by_mouse_and_keyboard() {
        for kind in MiniGameKind::ALL {
            for keyboard in [false, true] {
                for seed in [1, 7, 42] {
                    let (q, t) = run(kind, MiniGameParams::new(seed), keyboard);
                    eprintln!("{kind:?} kb={keyboard} seed {seed}: q={q:.3} t={t:.1}s");
                    assert!((0.0..=1.0).contains(&q), "{kind:?} q={q}");
                    assert!(
                        q >= 0.7,
                        "autoplay should play well: {kind:?} kb={keyboard} q={q}"
                    );
                    let (lo, hi) = duration_range(kind);
                    assert!((lo..=hi).contains(&t), "{kind:?} kb={keyboard} took {t}s");
                }
            }
        }
    }

    #[test]
    fn every_latte_pattern_is_playable() {
        for shape in LatteShape::ALL {
            for keyboard in [false, true] {
                let p = MiniGameParams::new(3).variant(Variation::latte(shape));
                let (q, t) = run(MiniGameKind::LatteArt, p, keyboard);
                eprintln!("{shape:?} kb={keyboard}: q={q:.3} t={t:.1}s");
                assert!(q >= 0.7, "{shape:?} kb={keyboard} q={q}");
                assert!(t <= 17.0, "{shape:?} took {t}s");
            }
        }
    }

    #[test]
    fn idle_player_times_out_gently_on_normal() {
        for kind in MiniGameKind::ALL {
            let mut mg = MiniGame::start(kind, MiniGameParams::new(3));
            let mut done = None;
            for _ in 0..(60 * 40) {
                if let MiniGameStatus::Done { quality } = mg.step(&MgInput::default(), 1.0 / 60.0) {
                    done = Some(quality);
                    break;
                }
            }
            let q = done.unwrap_or_else(|| panic!("{kind:?} must end on its own"));
            assert!((0.0..=1.0).contains(&q), "{kind:?} q={q}");
        }
    }

    #[test]
    fn relaxed_has_no_auto_end_and_a_floor() {
        for kind in MiniGameKind::ALL {
            let mut mg = MiniGame::start(kind, MiniGameParams::new(3).relaxed(true));
            for _ in 0..(60 * 30) {
                let s = mg.step(&MgInput::default(), 1.0 / 60.0);
                assert_eq!(s, MiniGameStatus::Running, "{kind:?} ended without Done");
            }
            let mut q = None;
            for _ in 0..200 {
                let s = mg.step(
                    &MgInput {
                        confirm: true,
                        ..Default::default()
                    },
                    1.0 / 60.0,
                );
                if let MiniGameStatus::Done { quality } = s {
                    q = Some(quality);
                    break;
                }
            }
            let q = q.unwrap_or_else(|| panic!("{kind:?}: Done must end a relaxed game"));
            assert!(q >= 0.5, "{kind:?} relaxed floor, q={q}");
        }
    }

    #[test]
    fn relaxed_never_scores_lower() {
        for kind in MiniGameKind::ALL {
            let (n, _) = run(kind, MiniGameParams::new(5), false);
            let (r, _) = run(kind, MiniGameParams::new(5).relaxed(true), false);
            assert!(r + 1e-4 >= n, "{kind:?}: relaxed {r} < normal {n}");
        }
    }

    #[test]
    fn skip_returns_seven_tenths() {
        for kind in MiniGameKind::ALL {
            let mut mg = MiniGame::start(kind, MiniGameParams::new(9));
            mg.step(&MgInput::default(), 0.016);
            let s = mg.step(
                &MgInput {
                    skip: true,
                    ..Default::default()
                },
                0.016,
            );
            assert_eq!(s, MiniGameStatus::Done { quality: 0.7 });
            assert_eq!(
                mg.step(&MgInput::default(), 0.016),
                MiniGameStatus::Done { quality: 0.7 }
            );
        }
    }

    #[test]
    fn deterministic_by_seed() {
        for kind in MiniGameKind::ALL {
            let a = run(kind, MiniGameParams::new(11), false);
            let b = run(kind, MiniGameParams::new(11), false);
            assert_eq!(a, b, "{kind:?}");
        }
    }

    #[test]
    fn variation_presets_and_seed_picks() {
        let mg = MiniGame::start(MiniGameKind::LatteArt, MiniGameParams::new(5));
        assert!(mg.variation().shape.is_some());
        let v = Variation::from_variant_id(MiniGameKind::LatteArt, "train");
        assert_eq!(v.shape, Some(LatteShape::Train));
        let v = Variation::from_variant_id(MiniGameKind::MilkFroth, "chocolate");
        assert_eq!(v.froth, Some(FrothTuning::CHOCOLATE));
        let v = Variation::from_variant_id(MiniGameKind::TeaSteep, "two_sugars");
        assert_eq!(v.tea.map(|t| t.sugar), Some(2));
        let mg = MiniGame::start(MiniGameKind::Plate, MiniGameParams::new(5));
        assert!(mg.variation().plate.is_some());
    }

    #[test]
    fn ids_round_trip() {
        for k in MiniGameKind::ALL {
            assert_eq!(MiniGameKind::from_id(k.id()), Some(k));
        }
        for s in PlateSet::ALL {
            assert_eq!(PlateSet::from_id(s.id()), Some(s));
        }
        for s in TeaKind::ALL {
            assert_eq!(TeaKind::from_id(s.id()), Some(s));
        }
        for s in RegularTouch::ALL {
            assert_eq!(RegularTouch::from_id(s.id()), Some(s));
        }
    }

    #[test]
    fn sounds_map_onto_existing_files() {
        for s in MiniGameSound::ALL {
            assert!(
                crate::content::SFX_IDS.contains(&s.cue().id),
                "{s:?} → {}",
                s.cue().id
            );
        }
    }

    #[test]
    fn text_keys_exist_in_tr_source() {
        let src = include_str!("../../assets/i18n/src/minigame.tr");
        let mut tables: crate::i18n::source::Tables = Default::default();
        crate::i18n::source::parse_into(&mut tables, src, "minigame.tr").expect("valid .tr");
        for lang in crate::i18n::Lang::ALL {
            for k in TEXT_KEYS {
                assert!(
                    tables[lang.index()].contains_key(*k),
                    "{k} missing in {lang:?}"
                );
            }
            assert_eq!(
                tables[lang.index()].len(),
                TEXT_KEYS.len(),
                "unused keys in .tr"
            );
        }
        for s in LatteShape::ALL {
            assert!(TEXT_KEYS.contains(&s.key().as_str()), "{s:?}");
        }
        for k in MiniGameKind::ALL {
            assert!(TEXT_KEYS.contains(&k.title_key().as_str()));
        }
        for s in PlateSet::ALL {
            assert!(TEXT_KEYS.contains(&s.key().as_str()));
        }
        for s in TeaKind::ALL {
            assert!(TEXT_KEYS.contains(&s.key().as_str()));
        }
        for s in RegularTouch::ALL {
            assert!(TEXT_KEYS.contains(&s.key().as_str()));
        }
    }
}
