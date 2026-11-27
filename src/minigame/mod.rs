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
//! (`MilkFroth, LatteArt, PourOver, TeaSteep, Plate, SoupStir, MulledSpice` — all built).
//! Every recipe variant id has a preset in [`Variation::from_variant_id`] matching its
//! `minigames.ron` row ([`FrothTuning`], [`PourTuning`], [`TeaTuning`], [`PlateSet`],
//! [`SoupSet`], [`MulledSet`]; latte pattern ids onto [`LatteShape::from_id`]); a test checks
//! they agree, so the game always shows the drink on the order card.
//!
//! Around every game: a one-screen guide card the first time a kind or a new step plays
//! ([`MiniGame::offer_guide`], remembered in the settings; "?" reopens it), the live 1–2–3 star
//! bar ([`MiniGame::preview_quality`], marks [`STAR_MARKS`]), the "why this many stars" line
//! ([`MiniGame::advice_key`]), and for practised recipes Lida's flourish (+[`FLOURISH_BONUS`],
//! never required).

mod froth;
mod guide;
mod latte;
pub mod latte_shapes;
mod mulled;
mod paint;
mod plate;
mod pour;
pub mod ritual;
pub mod score;
mod soup;
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
    /// Soups: stir in a calm rhythm, add the ingredients when the pot glows.
    SoupStir,
    /// Svařák: pick the recipe card's spices from the tray, keep the pot warm, not boiling.
    MulledSpice,
}

impl MiniGameKind {
    pub const ALL: [MiniGameKind; 7] = [
        MiniGameKind::MilkFroth,
        MiniGameKind::LatteArt,
        MiniGameKind::PourOver,
        MiniGameKind::TeaSteep,
        MiniGameKind::Plate,
        MiniGameKind::SoupStir,
        MiniGameKind::MulledSpice,
    ];

    /// snake_case id, as in `minigames.ron` and the i18n keys.
    pub fn id(self) -> &'static str {
        match self {
            MiniGameKind::MilkFroth => "milk_froth",
            MiniGameKind::LatteArt => "latte_art",
            MiniGameKind::PourOver => "pour_over",
            MiniGameKind::TeaSteep => "tea_steep",
            MiniGameKind::Plate => "plate",
            MiniGameKind::SoupStir => "soup_stir",
            MiniGameKind::MulledSpice => "mulled_spice",
        }
    }

    /// Accepts the ids above plus short aliases (`froth`, `latte`, `pour`, `tea`, `stir`,
    /// `mulled`).
    pub fn from_id(id: &str) -> Option<MiniGameKind> {
        Some(match id {
            "milk_froth" | "froth" => MiniGameKind::MilkFroth,
            "latte_art" | "latte" => MiniGameKind::LatteArt,
            "pour_over" | "pour" => MiniGameKind::PourOver,
            "tea_steep" | "tea" => MiniGameKind::TeaSteep,
            "plate" => MiniGameKind::Plate,
            "soup_stir" | "stir" => MiniGameKind::SoupStir,
            "mulled_spice" | "mulled" => MiniGameKind::MulledSpice,
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
    /// Česnečka: golden garlic soup with croutons, cheese and garlic; parsley flecks.
    GarlicSoup,
    /// Borscht: beetroot soup with smetana, dill and garlic croutons.
    Borscht,
    /// Chlebíček: a baguette slice with potato salad, ham, egg and a pickle.
    Chlebicek,
    /// Koláč: a round pastry with tvaroh, plum jam and crumble.
    Kolac,
    /// Vera's koláč: poppy seed and tvaroh, crumble.
    VeraKolac,
    /// Varenyky: three dumplings, smetana and fried onion.
    Varenyky,
    /// Bryndzové pirohy: dumplings with bryndza and crispy bacon bits.
    Pirohy,
    /// Buchty: three buns with plum jam.
    Buchty,
}

impl PlateSet {
    pub const ALL: [PlateSet; 11] = [
        PlateSet::Medovnik,
        PlateSet::Soup,
        PlateSet::Topinka,
        PlateSet::GarlicSoup,
        PlateSet::Borscht,
        PlateSet::Chlebicek,
        PlateSet::Kolac,
        PlateSet::VeraKolac,
        PlateSet::Varenyky,
        PlateSet::Pirohy,
        PlateSet::Buchty,
    ];
    /// The dishes a plate game may pick when nothing is asked for (never a soup — soups stir).
    pub const DISHES: [PlateSet; 8] = [
        PlateSet::Medovnik,
        PlateSet::Topinka,
        PlateSet::Chlebicek,
        PlateSet::Kolac,
        PlateSet::VeraKolac,
        PlateSet::Varenyky,
        PlateSet::Pirohy,
        PlateSet::Buchty,
    ];
    pub fn id(self) -> &'static str {
        match self {
            PlateSet::Medovnik => "medovnik",
            PlateSet::Soup => "soup",
            PlateSet::Topinka => "topinka",
            PlateSet::GarlicSoup => "garlic_soup",
            PlateSet::Borscht => "borscht",
            PlateSet::Chlebicek => "chlebicek",
            PlateSet::Kolac => "kolac",
            PlateSet::VeraKolac => "vera_kolac",
            PlateSet::Varenyky => "varenyky",
            PlateSet::Pirohy => "pirohy",
            PlateSet::Buchty => "buchty",
        }
    }
    /// Plate set by id; the soups' recipe variants (`garlic`, `kulajda`) map too.
    pub fn from_id(id: &str) -> Option<PlateSet> {
        match id {
            "garlic" => Some(PlateSet::GarlicSoup),
            "kulajda" => Some(PlateSet::Soup),
            _ => PlateSet::ALL.into_iter().find(|s| s.id() == id),
        }
    }
    /// The soups are served in a bowl.
    pub fn is_soup(self) -> bool {
        matches!(
            self,
            PlateSet::Soup | PlateSet::GarlicSoup | PlateSet::Borscht
        )
    }
    pub fn key(self) -> String {
        format!("minigame.plate.dish.{}", self.id())
    }
}

/// Which soup is stirred in [`MiniGameKind::SoupStir`] (`minigames.ron` → `soup_stir`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SoupSet {
    /// Česnečka: golden broth; garlic, then croutons.
    Garlic,
    /// Kulajda: creamy dill soup; mushrooms, dill, then a cream swirl.
    Kulajda,
    /// Borscht: beetroot, then smetana.
    Borscht,
}

impl SoupSet {
    pub const ALL: [SoupSet; 3] = [SoupSet::Garlic, SoupSet::Kulajda, SoupSet::Borscht];
    pub fn id(self) -> &'static str {
        match self {
            SoupSet::Garlic => "garlic",
            SoupSet::Kulajda => "kulajda",
            SoupSet::Borscht => "borscht",
        }
    }
    pub fn from_id(id: &str) -> Option<SoupSet> {
        match id {
            "garlic_soup" => Some(SoupSet::Garlic),
            "soup" => Some(SoupSet::Kulajda),
            _ => SoupSet::ALL.into_iter().find(|s| s.id() == id),
        }
    }
    /// The ingredients added while stirring, in order (the same as `minigames.ron`).
    pub fn adds(self) -> &'static [&'static str] {
        match self {
            SoupSet::Garlic => &["garlic", "croutons"],
            SoupSet::Kulajda => &["mushrooms", "dill"],
            SoupSet::Borscht => &["beetroot", "smetana"],
        }
    }
    /// Kulajda ends with a cream swirl.
    pub fn finish_tap(self) -> bool {
        self == SoupSet::Kulajda
    }
    pub fn key(self) -> String {
        format!("minigame.soup_stir.dish.{}", self.id())
    }
}

/// Which pot is spiced in [`MiniGameKind::MulledSpice`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MulledSet {
    Svarak,
}

impl MulledSet {
    pub const ALL: [MulledSet; 1] = [MulledSet::Svarak];
    pub fn id(self) -> &'static str {
        "svarak"
    }
    pub fn from_id(id: &str) -> Option<MulledSet> {
        (id == "svarak").then_some(MulledSet::Svarak)
    }
    /// The recipe card's spices (a subset of the tray), as in `minigames.ron`.
    pub fn spices(self) -> &'static [&'static str] {
        &["cinnamon", "clove", "orange", "honey_drop"]
    }
    /// Everything on the tray.
    pub fn tray(self) -> &'static [&'static str] {
        &[
            "cinnamon",
            "clove",
            "star_anise",
            "orange",
            "honey_drop",
            "ginger",
        ]
    }
    pub fn key(self) -> String {
        format!("minigame.mulled_spice.dish.{}", self.id())
    }
}

/// Tea colour family for [`MiniGameKind::TeaSteep`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TeaKind {
    Black,
    Green,
    Herbal,
    Fruit,
    /// Grandma's linden blossom: pale gold, steeps gently.
    Linden,
    /// Tatra mountain herbs: green-amber.
    Mountain,
    /// Uzvar: dried pears, apples and prunes — deep amber.
    Uzvar,
}

impl TeaKind {
    pub const ALL: [TeaKind; 7] = [
        TeaKind::Black,
        TeaKind::Green,
        TeaKind::Herbal,
        TeaKind::Fruit,
        TeaKind::Linden,
        TeaKind::Mountain,
        TeaKind::Uzvar,
    ];
    pub fn id(self) -> &'static str {
        match self {
            TeaKind::Black => "black",
            TeaKind::Green => "green",
            TeaKind::Herbal => "herbal",
            TeaKind::Fruit => "fruit",
            TeaKind::Linden => "linden",
            TeaKind::Mountain => "mountain",
            TeaKind::Uzvar => "uzvar",
        }
    }
    pub fn from_id(id: &str) -> Option<TeaKind> {
        TeaKind::ALL.into_iter().find(|s| s.id() == id)
    }
    pub fn key(self) -> String {
        format!("minigame.tea_steep.kind.{}", self.id())
    }
    /// Steeping time constant in seconds: herbs and fruit open slower than black tea.
    pub fn tau(self) -> f32 {
        match self {
            TeaKind::Black => 7.0,
            TeaKind::Green => 6.5,
            TeaKind::Herbal | TeaKind::Mountain => 8.5,
            TeaKind::Linden => 9.0,
            TeaKind::Fruit | TeaKind::Uzvar => 8.0,
        }
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

/// What is in the cup under the froth (drawn, and named in the header).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FrothDrink {
    /// Espresso; thick foam.
    Cappuccino,
    /// Cocoa in the cup; light foam.
    Chocolate,
    /// Espresso steamed together with cream and vanilla; silky foam.
    Raf,
    /// Lori's encore: cocoa with a pinch of chili on top.
    Encore,
}

/// A swinging drop in the froth game: vanilla into the cream first, chili over the cup last.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FrothDrop {
    Vanilla,
    Chili,
}

impl FrothDrop {
    pub fn id(self) -> &'static str {
        match self {
            FrothDrop::Vanilla => "vanilla",
            FrothDrop::Chili => "chili",
        }
    }
}

/// `minigames.ron` → `milk_froth` row.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrothTuning {
    /// Target temperature, °C.
    pub t_target: f32,
    /// ± tolerance, °C.
    pub tol: f32,
    /// Target foam 0..1 (the texture: ≥ 0.55 thick, ≤ 0.35 light, else silky).
    pub foam: f32,
    pub foam_tol: f32,
    /// Share of the quality from the temperature (the rest is the foam).
    pub t_weight: f32,
    pub drink: FrothDrink,
    /// A timed drop and its share of the quality.
    pub drop: Option<(FrothDrop, f32)>,
}

impl FrothTuning {
    pub const CAPPUCCINO: FrothTuning = FrothTuning {
        t_target: 62.0,
        tol: 4.0,
        foam: 0.6,
        foam_tol: 0.3,
        t_weight: 0.75,
        drink: FrothDrink::Cappuccino,
        drop: None,
    };
    pub const CHOCOLATE: FrothTuning = FrothTuning {
        t_target: 58.0,
        tol: 6.0,
        foam: 0.3,
        foam_tol: 0.3,
        t_weight: 0.75,
        drink: FrothDrink::Chocolate,
        drop: None,
    };
    pub const RAF: FrothTuning = FrothTuning {
        t_target: 60.0,
        tol: 4.0,
        foam: 0.5,
        foam_tol: 0.3,
        t_weight: 0.75,
        drink: FrothDrink::Raf,
        drop: Some((FrothDrop::Vanilla, 0.15)),
    };
    pub const ENCORE: FrothTuning = FrothTuning {
        t_target: 58.0,
        tol: 6.0,
        foam: 0.3,
        foam_tol: 0.3,
        t_weight: 0.75,
        drink: FrothDrink::Encore,
        drop: Some((FrothDrop::Chili, 0.15)),
    };
    /// `minigame.milk_froth.texture.<thick|silky|light>`
    pub fn texture_key(&self) -> &'static str {
        if self.foam >= 0.55 {
            "minigame.milk_froth.texture.thick"
        } else if self.foam <= 0.35 {
            "minigame.milk_froth.texture.light"
        } else {
            "minigame.milk_froth.texture.silky"
        }
    }
}

/// How a pour-over variant is made.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PourStyle {
    /// A paper dripper: bloom, then slow circles.
    Dripper,
    /// Turek: water straight onto the grounds in a glass, three stirs, let it settle.
    Turek,
    /// Espresso tonic: a slow pour over ice into a tall glass.
    Tonic,
}

/// `minigames.ron` → `pour_over` row.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PourTuning {
    /// Metronome, beats per minute (48 = 0.8 Hz).
    pub bpm: f32,
    /// "Let it bloom" wait, seconds (0 = no bloom step).
    pub bloom_s: f32,
    pub style: PourStyle,
    /// Calm circling band, revolutions per second.
    pub rev: (f32, f32),
    /// Turek: stirs, then a stillness wait.
    pub stirs: u8,
    pub settle_s: f32,
    /// Night train: a final cocoa-and-spice dust tap.
    pub dust: bool,
}

impl PourTuning {
    pub const CLASSIC: PourTuning = PourTuning {
        bpm: 48.0,
        bloom_s: 2.0,
        style: PourStyle::Dripper,
        rev: (0.6, 1.2),
        stirs: 0,
        settle_s: 0.0,
        dust: false,
    };
    pub const TUREK: PourTuning = PourTuning {
        bpm: 48.0,
        bloom_s: 0.0,
        style: PourStyle::Turek,
        rev: (0.6, 1.2),
        stirs: 3,
        settle_s: 3.0,
        dust: false,
    };
    pub const NIGHT_TRAIN: PourTuning = PourTuning {
        dust: true,
        ..PourTuning::CLASSIC
    };
    pub const TONIC: PourTuning = PourTuning {
        bpm: 40.0,
        bloom_s: 0.0,
        style: PourStyle::Tonic,
        rev: (0.4, 0.9),
        stirs: 0,
        settle_s: 0.0,
        dust: false,
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
    /// A spoonful of honey dropped like the lemon.
    pub honey: bool,
    /// Chai: pour the warm milk to the caramel mark.
    pub milk: bool,
    /// Chai: spices added in the recipe card's order before steeping (0 = none).
    pub spices: u8,
    /// Chai simmers in a little pan instead of a glass mug.
    pub pan: bool,
}

impl TeaTuning {
    pub const LEMON: TeaTuning = TeaTuning {
        target: 3.0,
        tol: 1.5,
        lemon: true,
        sugar: 0,
        honey: false,
        milk: false,
        spices: 0,
        pan: false,
    };
    pub const TWO_SUGARS: TeaTuning = TeaTuning {
        sugar: 2,
        ..TeaTuning::LEMON
    };
    pub const CHAI: TeaTuning = TeaTuning {
        target: 4.0,
        tol: 1.5,
        lemon: false,
        sugar: 0,
        honey: false,
        milk: true,
        spices: 3,
        pan: true,
    };
    pub const LINDEN: TeaTuning = TeaTuning {
        target: 2.5,
        lemon: false,
        honey: true,
        ..TeaTuning::LEMON
    };
    pub const TATRA: TeaTuning = TeaTuning {
        target: 3.5,
        honey: true,
        ..TeaTuning::LEMON
    };
    pub const UZVAR: TeaTuning = TeaTuning {
        target: 3.0,
        lemon: false,
        honey: true,
        ..TeaTuning::LEMON
    };
}

/// Milk in the latte pitcher (colour of the pattern).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LatteMilk {
    Dairy,
    Oat,
    /// The moonlight latte: oat milk tinted with butterfly-pea flower.
    Butterfly,
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
    pub soup: Option<SoupSet>,
    pub mulled: Option<MulledSet>,
    pub latte_milk: Option<LatteMilk>,
    /// LatteArt without a fixed `shape`: the patterns offered to choose from (the player's
    /// unlocked ones; empty = the basic ones that fit the seed).
    pub choices: [Option<LatteShape>; 3],
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
    pub fn soup(s: SoupSet) -> Self {
        Variation {
            soup: Some(s),
            ..Default::default()
        }
    }
    pub fn mulled(m: MulledSet) -> Self {
        Variation {
            mulled: Some(m),
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
    /// The latte patterns to choose from (at most three are used).
    pub fn choices(mut self, shapes: &[LatteShape]) -> Self {
        self.choices = [None; 3];
        for (slot, s) in self.choices.iter_mut().zip(shapes) {
            *slot = Some(*s);
        }
        self
    }
    /// Presets for every recipe variant id of `recipes.ron` (`cappuccino`, `chocolate`, `raf`,
    /// `encore`; `classic`, `turek`, `night_train`, `tonic`; `lemon`, `chai`, `linden`,
    /// `two_sugars`, `tatra`, `uzvar`; `flat_white`, `moon` and latte pattern ids; plate dish
    /// ids; `garlic`, `kulajda`, `borscht`; `svarak`). The tea kind follows the variant, so the
    /// cup always shows the drink on the order card. Unknown → the kind's default.
    pub fn from_variant_id(kind: MiniGameKind, id: &str) -> Self {
        match kind {
            MiniGameKind::MilkFroth => Variation::froth(match id {
                "chocolate" => FrothTuning::CHOCOLATE,
                "raf" => FrothTuning::RAF,
                "encore" => FrothTuning::ENCORE,
                _ => FrothTuning::CAPPUCCINO,
            }),
            MiniGameKind::PourOver => Variation::pour(match id {
                "turek" => PourTuning::TUREK,
                "night_train" => PourTuning::NIGHT_TRAIN,
                "tonic" => PourTuning::TONIC,
                _ => PourTuning::CLASSIC,
            }),
            MiniGameKind::TeaSteep => {
                let (tu, k) = match id {
                    "two_sugars" => (TeaTuning::TWO_SUGARS, TeaKind::Black),
                    "chai" => (TeaTuning::CHAI, TeaKind::Black),
                    "linden" => (TeaTuning::LINDEN, TeaKind::Linden),
                    "tatra" => (TeaTuning::TATRA, TeaKind::Mountain),
                    "uzvar" => (TeaTuning::UZVAR, TeaKind::Uzvar),
                    _ => (TeaTuning::LEMON, TeaKind::Black),
                };
                Variation::tea(tu).tea_kind(k)
            }
            MiniGameKind::LatteArt => match id {
                "moon" => Variation {
                    shape: Some(LatteShape::Moon),
                    latte_milk: Some(LatteMilk::Butterfly),
                    ..Default::default()
                },
                "flat_white" => Variation {
                    latte_milk: Some(LatteMilk::Oat),
                    ..Default::default()
                },
                _ => Variation {
                    shape: LatteShape::from_id(id),
                    ..Default::default()
                },
            },
            MiniGameKind::Plate => Variation {
                plate: PlateSet::from_id(id),
                ..Default::default()
            },
            MiniGameKind::SoupStir => Variation {
                soup: SoupSet::from_id(id),
                ..Default::default()
            },
            MiniGameKind::MulledSpice => Variation {
                mulled: MulledSet::from_id(id),
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
    "minigame.plate.dish.garlic_soup",
    "minigame.plate.dish.borscht",
];

/// Every key of `minigames2.tr` the mini-games read (the new games, steps, guides, stars).
pub const TEXT_KEYS2: &[&str] = &[
    "minigame.soup_stir.title",
    "minigame.mulled_spice.title",
    "minigame.guide.how",
    "minigame.guide.new",
    "minigame.guide.mouse",
    "minigame.guide.keys",
    "minigame.guide.step",
    "minigame.guide.new_step",
    "minigame.guide.got_it",
    "minigame.guide.read",
    "minigame.guide.stars_any",
    "minigame.stars.now",
    "minigame.guide.milk_froth.what",
    "minigame.guide.milk_froth.mouse",
    "minigame.guide.milk_froth.keys",
    "minigame.guide.milk_froth.stars",
    "minigame.guide.latte_art.what",
    "minigame.guide.latte_art.mouse",
    "minigame.guide.latte_art.keys",
    "minigame.guide.latte_art.stars",
    "minigame.guide.pour_over.what",
    "minigame.guide.pour_over.mouse",
    "minigame.guide.pour_over.keys",
    "minigame.guide.pour_over.stars",
    "minigame.guide.tea_steep.what",
    "minigame.guide.tea_steep.mouse",
    "minigame.guide.tea_steep.keys",
    "minigame.guide.tea_steep.stars",
    "minigame.guide.plate.what",
    "minigame.guide.plate.mouse",
    "minigame.guide.plate.keys",
    "minigame.guide.plate.stars",
    "minigame.guide.soup_stir.what",
    "minigame.guide.soup_stir.mouse",
    "minigame.guide.soup_stir.keys",
    "minigame.guide.soup_stir.stars",
    "minigame.guide.mulled_spice.what",
    "minigame.guide.mulled_spice.mouse",
    "minigame.guide.mulled_spice.keys",
    "minigame.guide.mulled_spice.stars",
    "minigame.guide.step.latte_art.choose",
    "minigame.guide.step.flourish",
    "minigame.guide.step.milk_froth.vanilla",
    "minigame.guide.step.milk_froth.chili",
    "minigame.guide.step.pour_over.stir",
    "minigame.guide.step.pour_over.slow",
    "minigame.guide.step.pour_over.dust",
    "minigame.guide.step.tea_steep.spices",
    "minigame.guide.step.tea_steep.honey",
    "minigame.guide.step.tea_steep.milk",
    "minigame.guide.step.plate.choice",
    "minigame.guide.step.soup_stir.swirl",
    "minigame.milk_froth.texture.thick",
    "minigame.milk_froth.texture.silky",
    "minigame.milk_froth.texture.light",
    "minigame.milk_froth.prompt.vanilla",
    "minigame.milk_froth.prompt.chili",
    "minigame.latte_art.choose",
    "minigame.latte_art.prompt.choose",
    "minigame.pour_over.style.dripper",
    "minigame.pour_over.style.night_train",
    "minigame.pour_over.style.turek",
    "minigame.pour_over.style.tonic",
    "minigame.pour_over.prompt.slow",
    "minigame.pour_over.prompt.stir",
    "minigame.pour_over.prompt.settle",
    "minigame.pour_over.prompt.dust",
    "minigame.tea_steep.kind.linden",
    "minigame.tea_steep.kind.mountain",
    "minigame.tea_steep.kind.uzvar",
    "minigame.tea_steep.card",
    "minigame.tea_steep.card_hint",
    "minigame.tea_steep.prompt.spices",
    "minigame.tea_steep.prompt.lift_pan",
    "minigame.tea_steep.prompt.honey",
    "minigame.tea_steep.prompt.milk",
    "minigame.milk",
    "minigame.spice.cinnamon",
    "minigame.spice.cardamom",
    "minigame.spice.ginger",
    "minigame.spice.clove",
    "minigame.spice.star_anise",
    "minigame.spice.orange",
    "minigame.spice.honey_drop",
    "minigame.plate.dish.chlebicek",
    "minigame.plate.dish.kolac",
    "minigame.plate.dish.vera_kolac",
    "minigame.plate.dish.varenyky",
    "minigame.plate.dish.pirohy",
    "minigame.plate.dish.buchty",
    "minigame.plate.garnish",
    "minigame.plate.prompt.choose",
    "minigame.garnish.sugar",
    "minigame.garnish.dill",
    "minigame.garnish.onion",
    "minigame.garnish.parsley",
    "minigame.garnish.paprika",
    "minigame.garnish.chives",
    "minigame.garnish.cinnamon",
    "minigame.garnish.cocoa",
    "minigame.garnish.poppy",
    "minigame.soup_stir.dish.garlic",
    "minigame.soup_stir.dish.kulajda",
    "minigame.soup_stir.dish.borscht",
    "minigame.mulled_spice.dish.svarak",
    "minigame.soup_stir.add.garlic",
    "minigame.soup_stir.add.croutons",
    "minigame.soup_stir.add.mushrooms",
    "minigame.soup_stir.add.dill",
    "minigame.soup_stir.add.beetroot",
    "minigame.soup_stir.add.smetana",
    "minigame.soup_stir.add.cream",
    "minigame.soup_stir.adds",
    "minigame.soup_stir.prompt.stir",
    "minigame.soup_stir.prompt.add",
    "minigame.soup_stir.prompt.swirl",
    "minigame.soup_stir.prompt.done",
    "minigame.mulled_spice.card",
    "minigame.mulled_spice.prompt.spices",
    "minigame.mulled_spice.prompt.warm",
    "minigame.mulled_spice.prompt.hot",
    "minigame.mulled_spice.prompt.hold",
    "minigame.flourish.prompt",
    "minigame.flourish.hit",
    "minigame.flourish.miss",
    "minigame.why.perfect",
    "minigame.why.skipped",
    "minigame.why.drop",
    "minigame.why.froth.cold",
    "minigame.why.froth.hot",
    "minigame.why.froth.foam_low",
    "minigame.why.froth.foam_high",
    "minigame.why.latte.coverage",
    "minigame.why.latte.stray",
    "minigame.why.latte.taps",
    "minigame.why.pour.bloom",
    "minigame.why.pour.rhythm",
    "minigame.why.pour.centre",
    "minigame.why.pour.beat",
    "minigame.why.pour.stir",
    "minigame.why.pour.dust",
    "minigame.why.tea.weak",
    "minigame.why.tea.strong",
    "minigame.why.tea.sugar",
    "minigame.why.tea.spices",
    "minigame.why.tea.milk",
    "minigame.why.plate.slots",
    "minigame.why.plate.garnish",
    "minigame.why.plate.unplaced",
    "minigame.why.soup.rhythm",
    "minigame.why.soup.adds",
    "minigame.why.mulled.spices",
    "minigame.why.mulled.hot",
    "minigame.why.mulled.cool",
];

impl MiniGameText {
    /// Fills every key from `f` (e.g. `|k| i18n.tr(lang, k).to_string()`).
    pub fn from_lookup(mut f: impl FnMut(&str) -> String) -> Self {
        MiniGameText {
            lookup: TEXT_KEYS
                .iter()
                .chain(TEXT_KEYS2)
                .map(|k| (*k, f(k)))
                .collect(),
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
    /// Up / W and Down / S presses (spice trays, the soup's add).
    pub up: bool,
    pub down_tap: bool,
    /// Arrows / WASD held, each axis −1..1.
    pub axis: Vec2,
    /// Tab in Plate: next piece (elsewhere Tab is skip).
    pub cycle: bool,
    /// Enter or the Done button.
    pub confirm: bool,
    /// Esc / Tab / the Skip button.
    pub skip: bool,
    /// Esc while the guide card is open (closes the card, not the game).
    pub back: bool,
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
    Soup(soup::Soup),
    Mulled(mulled::Mulled),
}

/// Calls the same method on whichever game runs.
macro_rules! each {
    ($game:expr, $g:ident => $e:expr) => {
        match $game {
            Game::Froth($g) => $e,
            Game::Latte($g) => $e,
            Game::Pour($g) => $e,
            Game::Tea($g) => $e,
            Game::Plate($g) => $e,
            Game::Soup($g) => $e,
            Game::Mulled($g) => $e,
        }
    };
}

struct Finish {
    quality: f32,
    age: f32,
    skipped: bool,
    /// `minigame.why.*`: the one-line "why this many stars".
    advice: &'static str,
    flourish: bool,
}

struct Sparkle {
    p: Vec2,
    v: Vec2,
    age: f32,
    life: f32,
    size: f32,
    warm: bool,
}

/// The one-screen guide card shown before a game type (or a new step) plays the first time.
pub(crate) struct GuideCard {
    /// `None` = the kind is already known: only the new steps are listed.
    pub kind: Option<MiniGameKind>,
    /// Step topics (`tea_steep.spices`, `flourish`, …).
    pub steps: Vec<&'static str>,
    /// Opened with "?" (nothing new is recorded on close).
    pub reopened: bool,
    pub age: f32,
}

/// LatteArt: the pattern is chosen before the pour.
pub(crate) struct Chooser {
    pub options: Vec<LatteShape>,
    pub sel: usize,
    pub t: f32,
    /// The seed's pick (autoplay takes it, so the bot's lattes vary too).
    pub auto: usize,
}

/// Practised recipes: Lida's flourish — a shrinking ring, tapped when it meets the cup.
pub(crate) struct Flourish {
    pub raw: f32,
    pub t: f32,
    pub hit: Option<bool>,
    pub at: f32,
}

/// The practised flourish adds this to the quality when it lands (never required).
pub const FLOURISH_BONUS: f32 = 0.05;
const FLOURISH_T: f32 = 1.8;
const FLOURISH_FROM: f32 = 86.0;
const FLOURISH_TO: f32 = 18.0;
const FLOURISH_RING: f32 = 40.0;
const CHOOSE_AUTO_T: f32 = 6.0;
/// Quality marks of the 2nd and 3rd star (the 1st is always lit): [`Rating::stars`].
pub const STAR_MARKS: [f32; 2] = [0.7, 0.9];

/// Stars (1..=3) a quality earns — the same as [`Rating::stars`].
pub fn stars_for(q: f32) -> u32 {
    1 + STAR_MARKS.iter().filter(|m| q >= **m).count() as u32
}

fn flourish_radius(t: f32) -> f32 {
    FLOURISH_FROM + (FLOURISH_TO - FLOURISH_FROM) * (t / FLOURISH_T).clamp(0.0, 1.0)
}

fn flourish_ok(t: f32, relaxed: bool) -> bool {
    (flourish_radius(t) - FLOURISH_RING).abs() <= 7.0 * score::widen(relaxed)
}

/// Chooser chip rects (stage-local).
pub(crate) fn chooser_rect(i: usize, n: usize) -> Rect {
    let w = 118.0;
    let gap = 16.0;
    let total = n as f32 * w + (n as f32 - 1.0) * gap;
    let x0 = (STAGE_SIZE.x - total) / 2.0;
    Rect::new(x0 + i as f32 * (w + gap), 52.0, w, 150.0)
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
    /// Real time since start (animations, the card's appear).
    t: f32,
    /// The game's own clock: runs only while it is played (not under the guide / chooser).
    game_t: f32,
    game: Game,
    finish: Option<Finish>,
    guide: Option<GuideCard>,
    chooser: Option<Chooser>,
    flourish: Option<Flourish>,
    new_seen: Vec<String>,
    sounds: Vec<MiniGameSound>,
    rng: Rng,
    sparkles: Vec<Sparkle>,
    skip_req: Cell<bool>,
    done_req: Cell<bool>,
    help_req: Cell<bool>,
    got_it_req: Cell<bool>,
    stage_origin: Cell<Option<Vec2>>,
}

impl MiniGame {
    /// Starts a game. `params.seed` picks unspecified parts of the variation and small
    /// details (sweet-spot drift, plate layout, spice order…), so a run is reproducible and
    /// no two plays look exactly alike.
    pub fn start(kind: MiniGameKind, params: MiniGameParams) -> MiniGame {
        let mut rng = Rng::new(params.seed);
        let mut v = params.variant;
        let common = Common {
            relaxed: params.relaxed,
            practised: params.practised,
        };
        let mut chooser = None;
        let game = match kind {
            MiniGameKind::MilkFroth => {
                let mut tu = *v.froth.get_or_insert(FrothTuning::CAPPUCCINO);
                match v.touch {
                    Some(RegularTouch::ExtraFoam) => tu.foam = (tu.foam + 0.15).min(0.9),
                    Some(RegularTouch::LightFoam) => tu.foam = (tu.foam - 0.15).max(0.15),
                    _ => {}
                }
                Game::Froth(froth::Froth::new(&mut rng, tu, common))
            }
            MiniGameKind::LatteArt => {
                let milk = *v.latte_milk.get_or_insert(LatteMilk::Dairy);
                let shape = match v.shape {
                    Some(s) => s,
                    None => {
                        let mut options: Vec<LatteShape> =
                            v.choices.iter().flatten().copied().collect();
                        if options.is_empty() {
                            options = LatteShape::BASIC[..3].to_vec();
                        }
                        // a different order each time: the first chip is not always the same
                        let rot = (rng.next_u64() % options.len() as u64) as usize;
                        options.rotate_left(rot);
                        let auto = (rng.next_u64() % options.len() as u64) as usize;
                        let first = options[0];
                        if options.len() > 1 {
                            chooser = Some(Chooser {
                                options,
                                sel: 0,
                                t: 0.0,
                                auto,
                            });
                        }
                        first
                    }
                };
                v.shape = Some(shape);
                Game::Latte(latte::Latte::new(shape, milk, v.touch, common))
            }
            MiniGameKind::PourOver => {
                let tu = *v.pour.get_or_insert(PourTuning::CLASSIC);
                Game::Pour(pour::Pour::new(&mut rng, tu, v.touch, common))
            }
            MiniGameKind::TeaSteep => {
                let kind = *v.tea_kind.get_or_insert(TeaKind::Black);
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
                let set = *v.plate.get_or_insert_with(|| rng.pick(&PlateSet::DISHES));
                Game::Plate(plate::Plate::new(&mut rng, set, v.touch, common))
            }
            MiniGameKind::SoupStir => {
                let set = *v.soup.get_or_insert(SoupSet::Garlic);
                Game::Soup(soup::Soup::new(&mut rng, set, common))
            }
            MiniGameKind::MulledSpice => {
                let set = *v.mulled.get_or_insert(MulledSet::Svarak);
                Game::Mulled(mulled::Mulled::new(&mut rng, set, common))
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
            game_t: 0.0,
            game,
            finish: None,
            guide: None,
            chooser,
            flourish: None,
            new_seen: Vec::new(),
            sounds: Vec::new(),
            rng,
            sparkles: Vec::new(),
            skip_req: Cell::new(false),
            done_req: Cell::new(false),
            help_req: Cell::new(false),
            got_it_req: Cell::new(false),
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
    /// The variation with defaults / seed-picked fields filled in (the latte shape is the
    /// chosen one once picked).
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
    /// The "why this many stars" line (`minigame.why.*`) once finished.
    pub fn advice_key(&self) -> Option<&'static str> {
        self.finish
            .as_ref()
            .filter(|f| !f.skipped)
            .map(|f| f.advice)
    }
    /// Did the practised flourish land?
    pub fn flourished(&self) -> bool {
        self.finish.as_ref().is_some_and(|f| f.flourish)
    }

    /// Guide topics of this game: its kind id plus the steps it uses (`tea_steep.spices`, …).
    pub fn guide_topics(&self) -> Vec<String> {
        let mut v = vec![self.kind.id().to_string()];
        v.extend(self.steps().into_iter().map(str::to_string));
        v
    }

    fn steps(&self) -> Vec<&'static str> {
        let mut v = each!(&self.game, g => g.steps());
        if self.chooser.is_some() {
            v.insert(0, "latte_art.choose");
        }
        if self.practised {
            v.push("flourish");
        }
        v
    }

    /// Shows the guide card for the topics not in `seen` (call before the first step; a
    /// game already under way is left alone). Returns whether a card opened.
    pub fn offer_guide(&mut self, seen: &std::collections::BTreeSet<String>) -> bool {
        if self.game_t > 0.0 || self.finish.is_some() || self.guide.is_some() {
            return false;
        }
        let kind_new = !seen.contains(self.kind.id());
        let steps: Vec<&'static str> = self
            .steps()
            .into_iter()
            .filter(|s| !seen.contains(*s))
            .collect();
        if !kind_new && steps.is_empty() {
            return false;
        }
        self.guide = Some(GuideCard {
            kind: kind_new.then_some(self.kind),
            steps,
            reopened: false,
            age: 0.0,
        });
        true
    }

    /// Reopens the guide with everything about this game (the "?" in the header).
    pub fn open_guide(&mut self) {
        if self.finish.is_some() {
            return;
        }
        self.guide = Some(GuideCard {
            kind: Some(self.kind),
            steps: self.steps(),
            reopened: true,
            age: 0.0,
        });
    }

    pub fn guide_open(&self) -> bool {
        self.guide.is_some()
    }

    /// Topics the player has just read ("Got it"), to remember in the settings.
    pub fn take_seen_guides(&mut self) -> Vec<String> {
        std::mem::take(&mut self.new_seen)
    }

    /// The quality if the game ended now (fills the live star bar).
    pub fn preview_quality(&self) -> f32 {
        if let Some(f) = &self.finish {
            return f.quality;
        }
        if let Some(f) = &self.flourish {
            return score::finalize(f.raw, self.relaxed);
        }
        score::finalize(
            each!(&self.game, g => g.preview(self.relaxed)),
            self.relaxed,
        )
    }

    /// Requested one-shot sounds since the last call.
    pub fn drain_sounds(&mut self) -> Vec<MiniGameSound> {
        std::mem::take(&mut self.sounds)
    }

    /// Loop volumes for this frame.
    pub fn loops(&self) -> MiniGameLoops {
        if self.finish.is_some() || self.guide.is_some() {
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
            Game::Tea(g) => MiniGameLoops {
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
        let up = ui.take_key(KeyCode::Up) | ui.take_key(KeyCode::W);
        let down_tap = ui.take_key(KeyCode::Down) | ui.take_key(KeyCode::S);
        let esc = ui.esc();
        let guide = self.guide.is_some();
        let input = MgInput {
            pointer,
            pressed: ui.mouse_pressed() && in_stage,
            down: ui.mouse_down(),
            released: ui.mouse_released(),
            hold: key_down(KeyCode::Space),
            hold_pressed: ui.take_key(KeyCode::Space),
            left,
            right,
            up,
            down_tap,
            axis,
            cycle: tab && plate,
            confirm: ui.enter() || self.done_req.replace(false),
            // under the guide Esc only closes the card
            skip: (!guide && esc) || (tab && !plate && !guide) || self.skip_req.replace(false),
            back: guide && esc,
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
        if self.help_req.replace(false) && self.guide.is_none() {
            self.open_guide();
        }
        // the guide card: the game waits until "Got it"
        if let Some(g) = &mut self.guide {
            g.age += dt;
            let close = g.age > 0.15
                && (input.confirm
                    || input.back
                    || input.hold_pressed
                    || self.got_it_req.replace(false));
            if close {
                let g = self.guide.take().expect("open");
                if !g.reopened {
                    if let Some(k) = g.kind {
                        self.new_seen.push(k.id().to_string());
                    }
                    self.new_seen.extend(g.steps.iter().map(|s| s.to_string()));
                }
                self.sounds.push(MiniGameSound::Cue);
            }
            return MiniGameStatus::Running;
        }
        if input.skip {
            self.finish = Some(Finish {
                quality: SKIP_QUALITY,
                age: 0.0,
                skipped: true,
                advice: "minigame.why.skipped",
                flourish: false,
            });
            return MiniGameStatus::Done {
                quality: SKIP_QUALITY,
            };
        }
        if self.chooser.is_some() {
            self.step_chooser(input, dt);
            return MiniGameStatus::Running;
        }
        if self.flourish.is_some() {
            self.step_flourish(input, dt);
            return MiniGameStatus::Running;
        }
        self.game_t += dt;
        let mut ctx = Ctx {
            t: self.game_t,
            relaxed: self.relaxed,
            sounds: &mut self.sounds,
        };
        let raw = each!(&mut self.game, g => g.step(&mut ctx, input, dt));
        if let Some(q) = raw {
            if self.practised && score::finalize(q, self.relaxed) < 1.0 {
                self.flourish = Some(Flourish {
                    raw: q,
                    t: 0.0,
                    hit: None,
                    at: 0.0,
                });
            } else {
                self.end(q, false);
            }
        }
        MiniGameStatus::Running
    }

    fn end(&mut self, raw: f32, flourish: bool) {
        let bonus = if flourish { FLOURISH_BONUS } else { 0.0 };
        let quality = score::finalize(raw + bonus, self.relaxed);
        let advice = if stars_for(quality) >= 3 {
            "minigame.why.perfect"
        } else {
            each!(&self.game, g => g.advice(self.relaxed))
        };
        self.finish = Some(Finish {
            quality,
            age: 0.0,
            skipped: false,
            advice,
            flourish,
        });
        self.sounds.push(MiniGameSound::Success);
        let focus = self.focus();
        let n = if flourish { 34 } else { 22 };
        for i in 0..n {
            let a = i as f32 / n as f32 * std::f32::consts::TAU + self.rng.range(-0.2, 0.2);
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

    fn step_chooser(&mut self, input: &MgInput, dt: f32) {
        let Some(c) = &mut self.chooser else {
            return;
        };
        c.t += dt;
        let n = c.options.len();
        if input.left {
            c.sel = (c.sel + n - 1) % n;
        }
        if input.right {
            c.sel = (c.sel + 1) % n;
        }
        let mut pick = None;
        if input.pressed
            && let Some(p) = input.pointer
        {
            pick = (0..n).find(|i| chooser_rect(*i, n).contains(p));
        }
        if c.t > 0.2 && (input.confirm || input.hold_pressed) {
            pick = Some(c.sel);
        }
        if pick.is_none() && !self.relaxed && c.t > CHOOSE_AUTO_T {
            pick = Some(c.sel);
        }
        if let Some(i) = pick {
            let shape = c.options[i];
            self.chooser = None;
            self.variation.shape = Some(shape);
            let common = Common {
                relaxed: self.relaxed,
                practised: self.practised,
            };
            let milk = self.variation.latte_milk.unwrap_or(LatteMilk::Dairy);
            self.game = Game::Latte(latte::Latte::new(shape, milk, self.variation.touch, common));
            self.sounds.push(MiniGameSound::Cue);
        }
    }

    fn step_flourish(&mut self, input: &MgInput, dt: f32) {
        let relaxed = self.relaxed;
        let Some(f) = &mut self.flourish else {
            return;
        };
        f.t += dt;
        if f.hit.is_none() && f.t > 0.1 && (input.pressed || input.hold_pressed) {
            let ok = flourish_ok(f.t, relaxed);
            f.hit = Some(ok);
            f.at = f.t;
            self.sounds.push(if ok {
                MiniGameSound::Cue
            } else {
                MiniGameSound::Clink
            });
        }
        let over = match f.hit {
            Some(_) => f.t - f.at > 0.35,
            None => f.t > FLOURISH_T + 0.25,
        };
        if over {
            let (raw, hit) = (f.raw, f.hit == Some(true));
            self.flourish = None;
            self.end(raw, hit);
        }
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
        if let Some(g) = &self.guide {
            return MgInput {
                confirm: g.age > 0.3,
                ..Default::default()
            };
        }
        if let Some(c) = &self.chooser {
            if c.t < 0.6 {
                return MgInput::default();
            }
            if keyboard {
                return MgInput {
                    right: c.sel != c.auto && ((c.t * 60.0).round() as u32).is_multiple_of(20),
                    confirm: c.sel == c.auto,
                    ..Default::default()
                };
            }
            let r = chooser_rect(c.auto, c.options.len());
            return MgInput {
                pointer: Some(r.center()),
                pressed: true,
                ..Default::default()
            };
        }
        if let Some(f) = &self.flourish {
            let now =
                f.hit.is_none() && (flourish_radius(f.t) - FLOURISH_RING).abs() < 2.5 && f.t > 0.1;
            return if keyboard {
                MgInput {
                    hold_pressed: now,
                    ..Default::default()
                }
            } else {
                MgInput {
                    pointer: Some(self.focus()),
                    pressed: now,
                    ..Default::default()
                }
            };
        }
        let mut i = each!(&self.game, g => g.autoplay(self.game_t, keyboard));
        // Relaxed games wait for Done: press it once the game offers it.
        let hint_done = each!(&self.game, g => g.hint().done);
        if self.relaxed && hint_done && self.ready_for_done() {
            i.confirm = true;
        }
        i
    }

    fn ready_for_done(&self) -> bool {
        match &self.game {
            Game::Latte(g) => g.finished_gesture() && g.autoplay_finished(self.game_t),
            _ => each!(&self.game, g => g.finished_gesture()),
        }
    }

    fn focus(&self) -> Vec2 {
        each!(&self.game, g => g.focus())
    }

    /// The header's detail after the title: what makes this cup itself.
    fn detail_key(&self) -> Option<String> {
        let v = &self.variation;
        match &self.game {
            Game::Latte(_) => v.shape.map(|s| s.key()),
            Game::Tea(_) => v.tea_kind.map(|s| s.key()),
            Game::Plate(_) => v.plate.map(|s| s.key()),
            Game::Froth(_) => v.froth.map(|f| f.texture_key().to_string()),
            Game::Soup(_) => v.soup.map(|s| s.key()),
            Game::Mulled(_) => v.mulled.map(|s| s.key()),
            Game::Pour(_) => v.pour.map(|p| {
                match p.style {
                    PourStyle::Dripper if p.dust => "minigame.pour_over.style.night_train",
                    PourStyle::Dripper => "minigame.pour_over.style.dripper",
                    PourStyle::Turek => "minigame.pour_over.style.turek",
                    PourStyle::Tonic => "minigame.pour_over.style.tonic",
                }
                .to_string()
            }),
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

        let hint = if self.guide.is_some() {
            Hint {
                key: "minigame.guide.read",
                emphasis: false,
                keys: &[],
                done: false,
            }
        } else if self.chooser.is_some() {
            Hint {
                key: "minigame.latte_art.prompt.choose",
                emphasis: false,
                keys: &["←", "→", "Space"],
                done: false,
            }
        } else if self.flourish.is_some() {
            Hint {
                key: "minigame.flourish.prompt",
                emphasis: true,
                keys: &["Space"],
                done: false,
            }
        } else {
            each!(&self.game, g => g.hint())
        };
        let detail = self.detail_key();
        let out = paint::card(ui, card, self, &hint, detail.as_deref());
        if out.skip {
            self.skip_req.set(true);
        }
        if out.done {
            self.done_req.set(true);
        }
        if out.help {
            self.help_req.set(true);
        }

        ui.push_clip(stage);
        paint::stage_backdrop(stage, self.t);
        let o = stage.point();
        if let Some(c) = &self.chooser {
            guide::draw_chooser(ui, stage, c, self.variation.latte_milk, &self.text, self.t);
        } else {
            each!(&self.game, g => g.draw(ui, o, self.game_t, &self.text));
        }
        paint::stage_vignette(stage);
        if let Some(f) = &self.flourish {
            guide::draw_flourish(ui, o + self.focus(), f, self.relaxed, &self.text);
        }
        for s in &self.sparkles {
            paint::sparkle(o + s.p, s.size, s.age / s.life, s.warm);
        }
        if let Some(f) = &self.finish
            && !f.skipped
        {
            paint::result_chip(
                ui, stage, f.quality, f.age, &self.text, f.advice, f.flourish,
            );
        }
        if let Some(g) = &self.guide
            && guide::draw_guide(ui, stage, g, &self.text)
        {
            self.got_it_req.set(true);
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
            MiniGameKind::SoupStir => (9.0, 20.5),
            MiniGameKind::MulledSpice => (6.0, 16.5),
        }
    }

    fn content() -> crate::content::Content {
        let root = env!("CARGO_MANIFEST_DIR");
        crate::content::Content::load(&format!("{root}/assets/data")).expect("content")
    }

    fn content_kind(c: crate::content::handson::MiniGameKind) -> MiniGameKind {
        MiniGameKind::from_id(c.id()).expect("same ids")
    }

    /// Every recipe of `recipes.ron` plays its own game, and every one of its variants is a
    /// real preset that matches its `minigames.ron` row (no seed-picked "random tea").
    #[test]
    fn every_recipe_plays_its_own_drink() {
        let c = content();
        let mg = &c.handson.minigames;
        for r in &c.handson.recipes.recipes {
            let kind = content_kind(r.minigame);
            assert!(!r.variants.is_empty(), "{}", r.id);
            for v in &r.variants {
                assert!(mg.has_variant(r.minigame, v), "{}: {v} has no row", r.id);
                let var = Variation::from_variant_id(kind, v);
                let game = MiniGame::start(kind, MiniGameParams::new(7).variant(var));
                let got = game.variation();
                match kind {
                    MiniGameKind::TeaSteep => {
                        let row = mg.tea_steep(v).unwrap();
                        let tu = var.tea.expect("tea preset");
                        assert_eq!(
                            var.tea_kind.map(|k| k.id()),
                            Some(row.tea_kind.as_str()),
                            "{}",
                            r.id
                        );
                        assert_eq!(
                            got.tea_kind, var.tea_kind,
                            "{}: the kind must not be random",
                            r.id
                        );
                        assert_eq!((tu.target, tu.tol), (row.target, row.tol), "{}", r.id);
                        assert_eq!(tu.lemon, row.lemon, "{}", r.id);
                        assert_eq!(tu.sugar as u32, row.sugar, "{}", r.id);
                        assert_eq!(tu.milk, row.milk_hold, "{}", r.id);
                        assert_eq!(tu.honey, row.honey, "{}", r.id);
                        assert_eq!(tu.spices as u32, row.spices, "{}", r.id);
                        assert_eq!(tu.pan, row.pan, "{}", r.id);
                    }
                    MiniGameKind::MilkFroth => {
                        let row = mg.milk_froth(v).unwrap();
                        let tu = var.froth.expect("froth preset");
                        assert_eq!(
                            (tu.t_target, tu.tol, tu.foam),
                            (row.t_target, row.tol, row.foam),
                            "{}",
                            r.id
                        );
                        assert_eq!(
                            (tu.foam_tol, tu.t_weight),
                            (row.foam_tol, row.t_weight),
                            "{}",
                            r.id
                        );
                        assert_eq!(
                            tu.drop.map(|d| d.0.id()),
                            row.tap.as_ref().map(|t| t.id.as_str()),
                            "{}",
                            r.id
                        );
                    }
                    MiniGameKind::PourOver => {
                        let row = mg.pour_over(v).unwrap();
                        let tu = var.pour.expect("pour preset");
                        assert_eq!(
                            (tu.bpm, tu.bloom_s, tu.rev),
                            (row.bpm, row.bloom_s, row.rev_per_s),
                            "{}",
                            r.id
                        );
                        assert_eq!(
                            (tu.stirs as u32, tu.settle_s),
                            (row.stirs, row.settle_s),
                            "{}",
                            r.id
                        );
                        assert_eq!(tu.dust, row.dust_tap, "{}", r.id);
                        assert_eq!(tu.style == PourStyle::Tonic, row.slow_pour, "{}", r.id);
                    }
                    MiniGameKind::LatteArt => {
                        let row = mg
                            .latte_art
                            .variants
                            .iter()
                            .find(|x| &x.variant == v)
                            .unwrap();
                        assert_eq!(
                            var.shape.map(|s| s.id()),
                            row.pattern.as_deref(),
                            "{}",
                            r.id
                        );
                    }
                    MiniGameKind::Plate => {
                        let row = mg.plate(v).unwrap();
                        assert_eq!(var.plate.map(|p| p.id()), row.dish.as_deref(), "{}", r.id);
                        assert_eq!(
                            got.plate, var.plate,
                            "{}: the dish must not be random",
                            r.id
                        );
                    }
                    MiniGameKind::SoupStir => {
                        let row = mg.soup_stir(v).unwrap();
                        let set = var.soup.expect("soup preset");
                        assert_eq!(
                            set.adds(),
                            row.adds.iter().map(String::as_str).collect::<Vec<_>>(),
                            "{}",
                            r.id
                        );
                        assert_eq!(set.finish_tap(), row.finish_tap, "{}", r.id);
                    }
                    MiniGameKind::MulledSpice => {
                        let row = mg.mulled_spice(v).unwrap();
                        let set = var.mulled.expect("mulled preset");
                        assert_eq!(
                            set.spices(),
                            row.spices.iter().map(String::as_str).collect::<Vec<_>>(),
                            "{}",
                            r.id
                        );
                        assert_eq!(
                            set.tray(),
                            row.tray.iter().map(String::as_str).collect::<Vec<_>>(),
                            "{}",
                            r.id
                        );
                    }
                }
            }
        }
    }

    /// `cargo test --lib autoplay_quality_table -- --ignored --nocapture`: the bot's quality
    /// per recipe variant (mean of 8 seeds, mouse / keyboard).
    #[test]
    #[ignore]
    fn autoplay_quality_table() {
        let c = content();
        for r in &c.handson.recipes.recipes {
            let kind = content_kind(r.minigame);
            for v in &r.variants {
                let mut line = format!("{:<18} {:<12} {:<12}", r.id, kind.id(), v);
                for keyboard in [false, true] {
                    let (mut qs, mut ts) = (0.0, 0.0);
                    for seed in 0..8 {
                        let p = MiniGameParams::new(seed * 7 + 1)
                            .variant(Variation::from_variant_id(kind, v));
                        let (q, t) = run(kind, p, keyboard);
                        qs += q;
                        ts += t;
                    }
                    line += &format!("  q={:.3} t={:>4.1}s", qs / 8.0, ts / 8.0);
                }
                eprintln!("{line}");
            }
        }
    }

    /// The masala chai is black tea with spices and milk, in a pan — never a random tea.
    #[test]
    fn masala_chai_is_chai() {
        for seed in 0..20 {
            let v = Variation::from_variant_id(MiniGameKind::TeaSteep, "chai");
            let g = MiniGame::start(MiniGameKind::TeaSteep, MiniGameParams::new(seed).variant(v));
            assert_eq!(g.variation().tea_kind, Some(TeaKind::Black));
            let tu = g.variation().tea.unwrap();
            assert!(tu.milk && tu.pan && tu.spices == 3);
            assert!(g.guide_topics().contains(&"tea_steep.spices".to_string()));
        }
    }

    /// Every game type and every recipe variant: autoplay reaches at least two stars, by mouse
    /// and by keyboard, also practised (the flourish) and relaxed.
    #[test]
    fn every_variant_autoplays_to_two_stars() {
        let c = content();
        for r in &c.handson.recipes.recipes {
            let kind = content_kind(r.minigame);
            for v in &r.variants {
                for keyboard in [false, true] {
                    for (seed, practised) in [(3, false), (11, true), (29, false)] {
                        let p = MiniGameParams::new(seed)
                            .variant(Variation::from_variant_id(kind, v))
                            .practised(practised);
                        let (q, t) = run(kind, p, keyboard);
                        assert!(
                            stars_for(q) >= 2,
                            "{} {v} kb={keyboard} seed {seed}: q={q:.3}",
                            r.id
                        );
                        assert!(t < 26.0, "{} {v} kb={keyboard}: {t}s", r.id);
                    }
                }
            }
        }
    }

    /// The live bar uses the same marks as the result stars, the content's tiers agree, and the
    /// bar's last value is the quality the game reports (when no flourish is added).
    #[test]
    fn star_thresholds_match_the_quality_formula() {
        for i in 0..=100 {
            let q = i as f32 / 100.0;
            assert_eq!(stars_for(q), Rating::from_quality(q).stars(), "q={q}");
        }
        let c = content();
        let tiers = c.handson.minigames.quality_tiers;
        assert_eq!(STAR_MARKS, [tiers.lovely, tiers.beautiful]);
        for kind in MiniGameKind::ALL {
            for keyboard in [false, true] {
                let mut mg = MiniGame::start(kind, MiniGameParams::new(5));
                let mut last = None;
                for _ in 0..(60 * 60) {
                    let before = mg.preview_quality();
                    let i = if keyboard {
                        mg.autoplay_keyboard_input()
                    } else {
                        mg.autoplay_input()
                    };
                    if let MiniGameStatus::Done { quality } = mg.step(&i, 1.0 / 60.0) {
                        last = Some((before, quality));
                        break;
                    }
                }
                let (before, q) = last.expect("ends");
                assert!((0.0..=1.0).contains(&before));
                assert!((mg.preview_quality() - q).abs() < 1e-5, "{kind:?}");
                let _ = before;
            }
        }
    }

    /// The guide card opens once per type (and once per new step), waits, and is remembered.
    #[test]
    fn guide_shows_once_per_type_and_step() {
        let mut seen = std::collections::BTreeSet::new();
        for kind in MiniGameKind::ALL {
            let mut mg = MiniGame::start(kind, MiniGameParams::new(1));
            assert!(mg.offer_guide(&seen), "{kind:?}: first time");
            // the game waits under the card
            for _ in 0..120 {
                mg.step(&MgInput::default(), 1.0 / 60.0);
            }
            assert!(mg.guide_open());
            assert_eq!(mg.game_t, 0.0);
            mg.step(
                &MgInput {
                    confirm: true,
                    ..Default::default()
                },
                1.0 / 60.0,
            );
            assert!(!mg.guide_open());
            seen.extend(mg.take_seen_guides());
            assert!(seen.contains(kind.id()));
            let mut again = MiniGame::start(kind, MiniGameParams::new(2));
            assert!(!again.offer_guide(&seen), "{kind:?}: never shown again");
            // "?" reopens it, without recording anything new
            again.open_guide();
            assert!(again.guide_open());
            again.step(
                &MgInput {
                    confirm: true,
                    ..Default::default()
                },
                0.5,
            );
            again.step(
                &MgInput {
                    confirm: true,
                    ..Default::default()
                },
                0.016,
            );
            assert!(again.take_seen_guides().is_empty());
        }
        // a new step of a known kind: only the step
        let chai = Variation::from_variant_id(MiniGameKind::TeaSteep, "chai");
        let mut mg = MiniGame::start(MiniGameKind::TeaSteep, MiniGameParams::new(3).variant(chai));
        assert!(mg.offer_guide(&seen));
        assert!(
            mg.guide
                .as_ref()
                .is_some_and(|g| g.kind.is_none() && g.steps.contains(&"tea_steep.spices"))
        );
        for _ in 0..3 {
            mg.step(&MgInput::default(), 0.1);
        }
        mg.step(
            &MgInput {
                confirm: true,
                ..Default::default()
            },
            0.016,
        );
        seen.extend(mg.take_seen_guides());
        let mut mg = MiniGame::start(MiniGameKind::TeaSteep, MiniGameParams::new(4).variant(chai));
        assert!(!mg.offer_guide(&seen));
        // the practised flourish is a step too
        let mut mg = MiniGame::start(
            MiniGameKind::MilkFroth,
            MiniGameParams::new(4).practised(true),
        );
        assert!(mg.offer_guide(&seen));
    }

    /// The practised flourish only ever adds; missing it changes nothing.
    #[test]
    fn flourish_is_a_bonus_never_a_cost() {
        for kind in MiniGameKind::ALL {
            let (plain, _) = run(kind, MiniGameParams::new(8), false);
            let (hit, _) = run(kind, MiniGameParams::new(8).practised(true), false);
            assert!(hit + 1e-5 >= plain, "{kind:?}: {hit} < {plain}");
            // ignore the ring: same as not practised
            let mut mg = MiniGame::start(kind, MiniGameParams::new(8).practised(true));
            let q = loop {
                let i = if mg.flourish.is_some() {
                    MgInput::default()
                } else {
                    mg.autoplay_input()
                };
                if let MiniGameStatus::Done { quality } = mg.step(&i, 1.0 / 60.0) {
                    break quality;
                }
            };
            assert!(
                (q - plain).abs() < 1e-5,
                "{kind:?}: missed flourish {q} vs {plain}"
            );
        }
    }

    /// No two plays look exactly alike: the seed moves targets, orders and layouts.
    #[test]
    fn small_variation_per_play() {
        let chai = Variation::from_variant_id(MiniGameKind::TeaSteep, "chai");
        let cards: std::collections::BTreeSet<String> = (0..12)
            .map(|s| {
                let g =
                    MiniGame::start(MiniGameKind::TeaSteep, MiniGameParams::new(s).variant(chai));
                match &g.game {
                    Game::Tea(t) => t.card_ids(),
                    _ => unreachable!(),
                }
            })
            .collect();
        assert!(cards.len() > 3, "the chai card's order varies: {cards:?}");
        // slots never sit on top of each other (pieces must find a free spot)
        for set in PlateSet::ALL {
            for seed in 0..10 {
                let g = MiniGame::start(
                    MiniGameKind::Plate,
                    MiniGameParams::new(seed).variant(Variation::plate(set)),
                );
                let Game::Plate(p) = &g.game else {
                    unreachable!()
                };
                let l = p.layout();
                for i in 0..l.len() {
                    for j in i + 1..l.len() {
                        let d =
                            (((l[i].0 - l[j].0).pow(2) + (l[i].1 - l[j].1).pow(2)) as f32).sqrt();
                        assert!(d > 6.0, "{set:?} seed {seed}: slots {i} and {j} overlap");
                    }
                }
            }
        }
        let a = MiniGame::start(
            MiniGameKind::Plate,
            MiniGameParams::new(1).variant(Variation::plate(PlateSet::Kolac)),
        );
        let b = MiniGame::start(
            MiniGameKind::Plate,
            MiniGameParams::new(2).variant(Variation::plate(PlateSet::Kolac)),
        );
        let (Game::Plate(a), Game::Plate(b)) = (&a.game, &b.game) else {
            unreachable!()
        };
        assert_ne!(a.layout(), b.layout());
    }

    /// The new games and steps can't be failed: an idle player still gets a cup.
    #[test]
    fn advice_line_after_every_game() {
        for kind in MiniGameKind::ALL {
            let mut mg = MiniGame::start(kind, MiniGameParams::new(3));
            loop {
                if let MiniGameStatus::Done { .. } = mg.step(&MgInput::default(), 1.0 / 60.0) {
                    break;
                }
            }
            let k = mg.advice_key().expect("a why line");
            assert!(TEXT_KEYS2.contains(&k), "{kind:?}: {k}");
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
        assert!(mg.variation().plate.is_some_and(|p| !p.is_soup()));
        let v = Variation::from_variant_id(MiniGameKind::TeaSteep, "linden");
        assert_eq!(v.tea_kind, Some(TeaKind::Linden));
        let v = Variation::from_variant_id(MiniGameKind::PourOver, "turek");
        assert_eq!(v.pour.map(|p| p.style), Some(PourStyle::Turek));
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
        for s in SoupSet::ALL {
            assert_eq!(SoupSet::from_id(s.id()), Some(s));
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
        let all = |k: &str| TEXT_KEYS.contains(&k) || TEXT_KEYS2.contains(&k);
        for k in MiniGameKind::ALL {
            assert!(all(&k.title_key()), "{k:?}");
        }
        for s in PlateSet::ALL {
            assert!(all(&s.key()), "{s:?}");
        }
        for s in TeaKind::ALL {
            assert!(all(&s.key()), "{s:?}");
        }
        for s in SoupSet::ALL {
            assert!(all(&s.key()), "{s:?}");
        }
        for s in MulledSet::ALL {
            assert!(all(&s.key()), "{s:?}");
        }
        for s in RegularTouch::ALL {
            assert!(TEXT_KEYS.contains(&s.key().as_str()));
        }
        // minigames2.tr: every key, in every language
        let src = include_str!("../../assets/i18n/src/minigames2.tr");
        let mut tables: crate::i18n::source::Tables = Default::default();
        crate::i18n::source::parse_into(&mut tables, src, "minigames2.tr").expect("valid .tr");
        for lang in crate::i18n::Lang::ALL {
            for k in TEXT_KEYS2 {
                assert!(
                    tables[lang.index()].contains_key(*k),
                    "{k} missing in {lang:?}"
                );
            }
        }
        // every key the games ask for is listed
        let used = [
            include_str!("mod.rs"),
            include_str!("froth.rs"),
            include_str!("latte.rs"),
            include_str!("pour.rs"),
            include_str!("tea.rs"),
            include_str!("plate.rs"),
            include_str!("soup.rs"),
            include_str!("mulled.rs"),
            include_str!("guide.rs"),
            include_str!("paint.rs"),
        ];
        for src in used {
            for part in src.split('"').skip(1).step_by(2) {
                if part.starts_with("minigame.")
                    && !part.ends_with('.')
                    && !part.contains(' ')
                    && !part.contains('{')
                    && part != "minigame.tr"
                    && part != "minigames2.tr"
                {
                    assert!(all(part), "{part} is not in TEXT_KEYS / TEXT_KEYS2");
                }
            }
        }
        for kind in MiniGameKind::ALL {
            for part in ["what", "mouse", "keys", "stars"] {
                let k = format!("minigame.guide.{}.{part}", kind.id());
                assert!(all(&k), "{k}");
            }
            let mg = MiniGame::start(kind, MiniGameParams::new(1).practised(true));
            for t in mg.guide_topics().iter().skip(1) {
                assert!(all(&guide::step_key(t)), "{t}");
            }
        }
        for v in [
            ("chai", MiniGameKind::TeaSteep),
            ("linden", MiniGameKind::TeaSteep),
            ("turek", MiniGameKind::PourOver),
            ("night_train", MiniGameKind::PourOver),
            ("tonic", MiniGameKind::PourOver),
            ("raf", MiniGameKind::MilkFroth),
            ("encore", MiniGameKind::MilkFroth),
            ("kulajda", MiniGameKind::SoupStir),
            ("flat_white", MiniGameKind::LatteArt),
            ("kolac", MiniGameKind::Plate),
        ] {
            let mg = MiniGame::start(
                v.1,
                MiniGameParams::new(1).variant(Variation::from_variant_id(v.1, v.0)),
            );
            for t in mg.guide_topics().iter().skip(1) {
                assert!(all(&guide::step_key(t)), "{t}");
            }
        }
    }
}
