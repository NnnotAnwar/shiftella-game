//! # Brewing mini-games
//!
//! Short, tactile, never-failable moments behind the counter (brief §6). Each game
//! runs 8–20 s inside a centred Shiftella glass card and reports a quality in
//! 0.55..1.0 (skip = 0.7).
//!
//! ```ignore
//! // when the player picks a recipe:
//! let mut mg = MiniGame::start(recipe.minigame, seed, Variation::latte(LatteShape::Heart))
//!     .with_title(i18n.tr(lang, &recipe.name_key))
//!     .with_text(MiniGameText::from_lookup(|k| i18n.tr(lang, k).to_string()))
//!     .relaxed(settings.relaxed_minigames);
//! // every frame, after `ui.begin_frame()` and before game hotkeys:
//! if let MiniGameStatus::Done { quality } = mg.update(&mut ui, dt) {
//!     sim.apply(Command::PlayerBrew { guest, recipe, quality });
//! }
//! mg.draw(&mut ui, ui.screen());           // after the world, in the UI camera
//! for s in mg.drain_sounds() { audio.play_sfx(s.suggested_sfx()) }
//! audio_hiss_volume = mg.ambience();       // gentle loop level (froth hiss, pouring)
//! ```
//!
//! Input: mouse (presses count inside the stage), **Space** = hold / act, **Enter** =
//! act / serve, arrows or WASD move the pouring point in LatteArt, **Esc** = skip.
//! `update` consumes Space / Enter / Esc so game hotkeys do not fire underneath.
//! [`MiniGame::step`] takes an explicit [`MgInput`] (scripted demos, tests).

mod froth;
mod latte;
mod paint;
mod plate;
mod pour;
pub mod score;
pub mod shapes;
mod tea;

use std::cell::Cell;

use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

use crate::ui::{RectExt, Ui, ease_out};

pub use score::{Rating, SKIP_QUALITY, quality_from_perf};
pub use shapes::LatteShape;

/// Size of the glass card in UI units.
pub const CARD_SIZE: Vec2 = Vec2::new(520.0, 380.0);
/// Size of the play area inside the card; game coordinates are stage-local.
pub const STAGE_SIZE: Vec2 = Vec2::new(492.0, 282.0);
const PAD: f32 = 14.0;
const HEADER_H: f32 = 50.0;
/// How long the result chip stays before `Done` is reported.
const OUTRO: f32 = 1.35;

/// Which mini-game a recipe uses.
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

    pub fn name_key(self) -> String {
        format!("minigame.kind.{}", self.id())
    }
}

/// Which pastry / soup is garnished in [`MiniGameKind::Plate`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlateSet {
    /// Honey cake slice with cream, berries and mint.
    Medovnik,
    /// Creamy dill soup (kulajda) with cream, dill and croutons.
    Soup,
}

impl PlateSet {
    pub const ALL: [PlateSet; 2] = [PlateSet::Medovnik, PlateSet::Soup];
    pub fn key(self) -> &'static str {
        match self {
            PlateSet::Medovnik => "minigame.plate.medovnik",
            PlateSet::Soup => "minigame.plate.soup",
        }
    }
}

/// Tea for [`MiniGameKind::TeaSteep`] (sets the target colour and time).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TeaKind {
    Black,
    Green,
    Herbal,
    Fruit,
}

impl TeaKind {
    pub const ALL: [TeaKind; 4] = [TeaKind::Black, TeaKind::Green, TeaKind::Herbal, TeaKind::Fruit];
    pub fn key(self) -> &'static str {
        match self {
            TeaKind::Black => "minigame.tea.black",
            TeaKind::Green => "minigame.tea.green",
            TeaKind::Herbal => "minigame.tea.herbal",
            TeaKind::Fruit => "minigame.tea.fruit",
        }
    }
}

/// A regular's personal wish that nudges the target ("extra foam, please").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RegularTouch {
    /// Froth: higher sweet spot.
    ExtraFoam,
    /// Froth: lower sweet spot.
    LightFoam,
    /// Tea: steep longer / darker; pour-over: more water.
    Strong,
    /// Tea: shorter / lighter.
    Mild,
    /// Latte art: a dusting of cinnamon over the finished pattern.
    Cinnamon,
    /// Plate: an extra honey drizzle piece.
    Honey,
}

impl RegularTouch {
    pub fn key(self) -> &'static str {
        match self {
            RegularTouch::ExtraFoam => "minigame.touch.extra_foam",
            RegularTouch::LightFoam => "minigame.touch.light_foam",
            RegularTouch::Strong => "minigame.touch.strong",
            RegularTouch::Mild => "minigame.touch.mild",
            RegularTouch::Cinnamon => "minigame.touch.cinnamon",
            RegularTouch::Honey => "minigame.touch.honey",
        }
    }
}

/// What the caller asks for. `None` fields are picked from the seed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Variation {
    pub shape: Option<LatteShape>,
    pub plate: Option<PlateSet>,
    pub tea: Option<TeaKind>,
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
    pub fn tea(kind: TeaKind) -> Self {
        Variation {
            tea: Some(kind),
            ..Default::default()
        }
    }
    pub fn touch(mut self, t: RegularTouch) -> Self {
        self.touch = Some(t);
        self
    }
}

/// Result of [`MiniGame::update`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MiniGameStatus {
    Running,
    /// Reported once the result chip has been shown (or right away on skip); stays
    /// `Done` on later calls.
    Done { quality: f32 },
}

/// Sound requests; map them onto the game's SFX ([`MiniGameSound::suggested_sfx`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MiniGameSound {
    /// Steam wand starts (soft hiss).
    Froth,
    /// Milk / water starts pouring, tea bag lifted.
    Pour,
    /// Ceramic / glass touch: garnish placed, cup set down.
    Clink,
    /// Finished with a nice result.
    Success,
}

impl MiniGameSound {
    /// Closest existing SFX.
    pub fn suggested_sfx(self) -> crate::audio::Sfx {
        use crate::audio::Sfx;
        match self {
            MiniGameSound::Froth | MiniGameSound::Pour => Sfx::CoffeeMachine,
            MiniGameSound::Clink | MiniGameSound::Success => Sfx::CupSaucer,
        }
    }
}

/// Player-visible strings (from `assets/i18n/src/minigame.tr`).
#[derive(Clone, Debug)]
pub struct MiniGameText {
    lookup: std::collections::HashMap<&'static str, String>,
}

/// Every key the mini-games use (all present in `minigame.tr`).
pub const TEXT_KEYS: &[&str] = &[
    "minigame.kind.milk_froth",
    "minigame.kind.latte_art",
    "minigame.kind.pour_over",
    "minigame.kind.tea_steep",
    "minigame.kind.plate",
    "minigame.hint.froth",
    "minigame.hint.froth_release",
    "minigame.hint.latte",
    "minigame.hint.latte_serve",
    "minigame.hint.pour",
    "minigame.hint.pour_keep",
    "minigame.hint.tea",
    "minigame.hint.tea_ready",
    "minigame.hint.plate",
    "minigame.hint.plate_serve",
    "minigame.key.space",
    "minigame.skip",
    "minigame.serve",
    "minigame.target",
    "minigame.milk",
    "minigame.water",
    "minigame.result.cosy",
    "minigame.result.nice",
    "minigame.result.lovely",
    "minigame.result.perfect",
    "minigame.touch.title",
    "minigame.touch.extra_foam",
    "minigame.touch.light_foam",
    "minigame.touch.strong",
    "minigame.touch.mild",
    "minigame.touch.cinnamon",
    "minigame.touch.honey",
    "minigame.shape.heart",
    "minigame.shape.rosetta",
    "minigame.shape.tulip",
    "minigame.shape.moon",
    "minigame.shape.train",
    "minigame.tea.black",
    "minigame.tea.green",
    "minigame.tea.herbal",
    "minigame.tea.fruit",
    "minigame.plate.medovnik",
    "minigame.plate.soup",
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
    /// Space (hold / act).
    pub hold: bool,
    pub hold_pressed: bool,
    pub hold_released: bool,
    /// Enter: act / serve.
    pub confirm: bool,
    /// Esc or the Skip button.
    pub skip: bool,
    /// Arrows / WASD, each axis −1..1.
    pub axis: Vec2,
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
    pub touch: Option<RegularTouch>,
    pub sounds: &'a mut Vec<MiniGameSound>,
}

/// What a game shows in the footer.
pub(crate) struct Hint<'a> {
    pub key: &'a str,
    /// Highlighted call to action ("Release!").
    pub emphasis: bool,
    /// Keycaps shown before the text (already translated where needed).
    pub keys: &'a [&'a str],
    /// Show the Serve button (LatteArt / Plate).
    pub serve: bool,
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
    title: String,
    text: MiniGameText,
    t: f32,
    game: Game,
    finish: Option<Finish>,
    sounds: Vec<MiniGameSound>,
    rng: Rng,
    sparkles: Vec<Sparkle>,
    skip_req: Cell<bool>,
    serve_req: Cell<bool>,
    stage_origin: Cell<Option<Vec2>>,
}

impl MiniGame {
    /// Starts a game. `seed` picks the unspecified parts of `variation` and the
    /// small random details (sweet-spot height, tea timing…), so it is reproducible.
    pub fn start(kind: MiniGameKind, seed: u64, variation: Variation) -> MiniGame {
        let mut rng = Rng::new(seed);
        let mut v = variation;
        let game = match kind {
            MiniGameKind::MilkFroth => Game::Froth(froth::Froth::new(&mut rng, v.touch)),
            MiniGameKind::LatteArt => {
                let shape = *v.shape.get_or_insert_with(|| rng.pick(&LatteShape::ALL));
                Game::Latte(latte::Latte::new(shape))
            }
            MiniGameKind::PourOver => Game::Pour(pour::Pour::new(&mut rng, v.touch)),
            MiniGameKind::TeaSteep => {
                let tea = *v.tea.get_or_insert_with(|| rng.pick(&TeaKind::ALL));
                Game::Tea(tea::Tea::new(&mut rng, tea, v.touch))
            }
            MiniGameKind::Plate => {
                let set = *v.plate.get_or_insert_with(|| rng.pick(&PlateSet::ALL));
                Game::Plate(plate::Plate::new(&mut rng, set, v.touch))
            }
        };
        MiniGame {
            kind,
            variation: v,
            relaxed: false,
            title: String::new(),
            text: MiniGameText::default(),
            t: 0.0,
            game,
            finish: None,
            sounds: Vec::new(),
            rng,
            sparkles: Vec::new(),
            skip_req: Cell::new(false),
            serve_req: Cell::new(false),
            stage_origin: Cell::new(None),
        }
    }

    /// Recipe name shown in the header (already translated).
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }
    pub fn with_text(mut self, text: MiniGameText) -> Self {
        self.text = text;
        self
    }
    /// Relaxed mode: wider tolerances everywhere.
    pub fn relaxed(mut self, on: bool) -> Self {
        self.relaxed = on;
        self
    }

    pub fn kind(&self) -> MiniGameKind {
        self.kind
    }
    /// The variation with seed-picked fields filled in.
    pub fn variation(&self) -> Variation {
        self.variation
    }
    pub fn elapsed(&self) -> f32 {
        self.t
    }
    pub fn is_finished(&self) -> bool {
        self.finish.is_some()
    }
    /// Final quality once known (also during the result chip).
    pub fn quality(&self) -> Option<f32> {
        self.finish.as_ref().map(|f| f.quality)
    }

    /// Requested sounds since the last call.
    pub fn drain_sounds(&mut self) -> Vec<MiniGameSound> {
        std::mem::take(&mut self.sounds)
    }

    /// Gentle loop level 0..1 (steam hiss / pouring) for an ambience channel.
    pub fn ambience(&self) -> f32 {
        if self.finish.is_some() {
            return 0.0;
        }
        match &self.game {
            Game::Froth(g) => g.hiss(),
            Game::Latte(g) => g.flow(),
            Game::Pour(g) => g.flow(),
            _ => 0.0,
        }
    }

    /// Reads mouse / keyboard from the UI and advances the game.
    pub fn update(&mut self, ui: &mut Ui, dt: f32) -> MiniGameStatus {
        let origin = self.stage_origin.get();
        let pointer = origin.map(|o| ui.mouse() - o);
        let in_stage = pointer.is_some_and(|p| Rect::new(0.0, 0.0, STAGE_SIZE.x, STAGE_SIZE.y).contains(p));
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
        let input = MgInput {
            pointer,
            pressed: ui.mouse_pressed() && in_stage,
            down: ui.mouse_down(),
            released: ui.mouse_released(),
            hold: key_down(KeyCode::Space),
            hold_pressed: ui.take_key(KeyCode::Space),
            hold_released: !typing && is_key_released(KeyCode::Space),
            confirm: ui.enter() || self.serve_req.replace(false),
            skip: ui.esc() || self.skip_req.replace(false),
            axis,
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
            touch: self.variation.touch,
            sounds: &mut self.sounds,
        };
        let perf = match &mut self.game {
            Game::Froth(g) => g.step(&mut ctx, input, dt),
            Game::Latte(g) => g.step(&mut ctx, input, dt),
            Game::Pour(g) => g.step(&mut ctx, input, dt),
            Game::Tea(g) => g.step(&mut ctx, input, dt),
            Game::Plate(g) => g.step(&mut ctx, input, dt),
        };
        if let Some(p) = perf {
            let quality = quality_from_perf(p);
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
                    life: self.rng.range(0.7, 1.3),
                    size: self.rng.range(3.0, 7.0),
                    warm: i % 3 != 0,
                });
            }
        }
        MiniGameStatus::Running
    }

    /// Scripted input that plays the game nicely — for demos, screenshots and tests.
    pub fn autoplay_input(&self) -> MgInput {
        if self.finish.is_some() {
            return MgInput::default();
        }
        match &self.game {
            Game::Froth(g) => g.autoplay(self.t),
            Game::Latte(g) => g.autoplay(self.t),
            Game::Pour(g) => g.autoplay(self.t),
            Game::Tea(g) => g.autoplay(self.t),
            Game::Plate(g) => g.autoplay(self.t),
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

    /// The glass card rect for a given screen area.
    pub fn card_rect(area: Rect) -> Rect {
        Rect::new(
            (area.x + (area.w - CARD_SIZE.x) / 2.0).round(),
            (area.y + (area.h - CARD_SIZE.y) / 2.0).round(),
            CARD_SIZE.x,
            CARD_SIZE.y,
        )
    }

    /// Draws the card centred in `rect` (UI units, UI camera).
    pub fn draw(&self, ui: &mut Ui, rect: Rect) {
        let appear = ease_out(self.t / 0.35);
        let card = Self::card_rect(rect).translate(vec2(0.0, (1.0 - appear) * 12.0));
        let stage = Rect::new(card.x + PAD, card.y + HEADER_H, STAGE_SIZE.x, STAGE_SIZE.y);
        self.stage_origin.set(Some(stage.point()));

        let hint = match &self.game {
            Game::Froth(g) => g.hint(),
            Game::Latte(g) => g.hint(),
            Game::Pour(g) => g.hint(),
            Game::Tea(g) => g.hint(),
            Game::Plate(g) => g.hint(),
        };
        let skip = paint::card(ui, card, self, &hint);
        if skip.skip {
            self.skip_req.set(true);
        }
        if skip.serve {
            self.serve_req.set(true);
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
            paint::result_chip(ui, stage, f.quality, f.age, &self.text, self.focus() + o);
        }
        // the stage content eases in under a veil
        if appear < 1.0 {
            crate::ui::draw::fill_rrect(stage, 10.0, paint::with_alpha(paint::STAGE_TOP, 1.0 - appear));
        }
        ui.pop_clip();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_auto(kind: MiniGameKind, seed: u64, v: Variation, relaxed: bool) -> (f32, f32) {
        let mut mg = MiniGame::start(kind, seed, v).relaxed(relaxed);
        let dt = 1.0 / 60.0;
        for _ in 0..(60 * 40) {
            let input = mg.autoplay_input();
            if let MiniGameStatus::Done { quality } = mg.step(&input, dt) {
                return (quality, mg.elapsed());
            }
        }
        panic!("{kind:?} did not finish");
    }

    #[test]
    fn every_game_finishes_in_range_with_autoplay() {
        for kind in MiniGameKind::ALL {
            for seed in [1, 7, 42] {
                let (q, t) = run_auto(kind, seed, Variation::default(), false);
                eprintln!("{kind:?} seed {seed}: q={q:.3} t={t:.1}s");
                assert!((0.55..=1.0).contains(&q), "{kind:?} q={q}");
                assert!(q > 0.8, "autoplay should play well: {kind:?} q={q}");
                assert!((6.0..=22.0).contains(&t), "{kind:?} took {t}s");
            }
        }
    }

    #[test]
    fn idle_player_still_gets_a_fine_result() {
        for kind in MiniGameKind::ALL {
            let mut mg = MiniGame::start(kind, 3, Variation::default());
            let mut done = None;
            for _ in 0..(60 * 40) {
                if let MiniGameStatus::Done { quality } = mg.step(&MgInput::default(), 1.0 / 60.0) {
                    done = Some(quality);
                    break;
                }
            }
            let q = done.unwrap_or_else(|| panic!("{kind:?} must time out gently"));
            assert!((0.55..=1.0).contains(&q), "{kind:?} q={q}");
        }
    }

    #[test]
    fn skip_returns_seven_tenths() {
        for kind in MiniGameKind::ALL {
            let mut mg = MiniGame::start(kind, 9, Variation::default());
            mg.step(&MgInput::default(), 0.016);
            let s = mg.step(
                &MgInput {
                    skip: true,
                    ..Default::default()
                },
                0.016,
            );
            assert_eq!(s, MiniGameStatus::Done { quality: 0.7 });
            // stays done
            assert_eq!(
                mg.step(&MgInput::default(), 0.016),
                MiniGameStatus::Done { quality: 0.7 }
            );
        }
    }

    #[test]
    fn deterministic_by_seed() {
        for kind in MiniGameKind::ALL {
            let a = run_auto(kind, 11, Variation::default(), false);
            let b = run_auto(kind, 11, Variation::default(), false);
            assert_eq!(a, b, "{kind:?}");
        }
    }

    #[test]
    fn variation_is_resolved_from_seed() {
        let mg = MiniGame::start(MiniGameKind::LatteArt, 5, Variation::default());
        assert!(mg.variation().shape.is_some());
        let mg = MiniGame::start(MiniGameKind::LatteArt, 5, Variation::latte(LatteShape::Train));
        assert_eq!(mg.variation().shape, Some(LatteShape::Train));
        let mg = MiniGame::start(MiniGameKind::Plate, 5, Variation::default());
        assert!(mg.variation().plate.is_some());
    }

    #[test]
    fn kind_ids_round_trip() {
        for k in MiniGameKind::ALL {
            assert_eq!(MiniGameKind::from_id(k.id()), Some(k));
        }
        assert_eq!(MiniGameKind::from_id("froth"), Some(MiniGameKind::MilkFroth));
    }

    #[test]
    fn text_keys_exist_in_tr_source() {
        let src = include_str!("../../assets/i18n/src/minigame.tr");
        let mut tables: crate::i18n::source::Tables = Default::default();
        crate::i18n::source::parse_into(&mut tables, src, "minigame.tr").expect("valid .tr");
        for lang in crate::i18n::Lang::ALL {
            for k in TEXT_KEYS {
                assert!(tables[lang.index()].contains_key(*k), "{k} missing in {lang:?}");
            }
        }
        for s in LatteShape::ALL {
            assert!(TEXT_KEYS.contains(&s.key()));
        }
        for k in MiniGameKind::ALL {
            assert!(TEXT_KEYS.contains(&k.name_key().as_str()));
        }
    }
}
