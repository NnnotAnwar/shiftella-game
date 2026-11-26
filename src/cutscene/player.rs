//! The cutscene player: runs a [`CutsceneDef`] step by step, animates camera / actors / train /
//! fades, and draws the dialogue chrome with the UI kit.

use std::collections::{HashMap, HashSet, VecDeque};

use macroquad::prelude::*;

use crate::content::{
    ChoiceOption, Condition, Content, CutsceneDef, CutsceneLib, CutsceneStep, Effect, Focus, Place,
};
use crate::map::{Map, TILE};
use crate::render::{Expression, draw_portrait};
use crate::settings::Locale;
use crate::ui::{DialogueBox, DialogueLabels, DialogueLine, RectExt, TextStyle, Ui, Weight};

use super::actors::{Actor, find_path, tile_feet};
use super::cards::{self, PhoneLabels};
use super::cast::{Cast, expression};
use super::scene::{CafeScene, OVERVIEW_CENTER};
use super::staging::{self, Op, Stage, TRAIN_STOP};

/// A sound the integration should play (the player never touches `audio`).
#[derive(Debug, Clone, PartialEq)]
pub enum AudioRequest {
    /// Switch music (`None` = fade out). Ids from `content::MUSIC_TRACKS`.
    Music { track: Option<String> },
    /// One-shot or ambience (`amb_*` ids loop). Ids from `content::SFX_IDS`.
    Sfx { id: String, volume: f32 },
}

/// One answered choice.
#[derive(Debug, Clone, PartialEq)]
pub struct ChoiceMade {
    /// Index in the step's `options` (as written in data, before hiding failed conditions).
    pub index: usize,
    pub label_key: String,
    pub set_flag: Option<String>,
}

/// Everything a finished cutscene did.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CutsceneOutcome {
    pub id: String,
    pub choices: Vec<ChoiceMade>,
    /// Flags set by choices or `SetFlag` effects.
    pub flags: Vec<String>,
    /// Every effect in order (`Apply` steps, choice effects, `FindNote` for shown notes).
    pub effects: Vec<Effect>,
    /// Notes shown (and therefore found).
    pub notes: Vec<String>,
    /// The player held Esc to skip.
    pub skipped: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CutsceneStatus {
    /// Nothing is playing.
    Idle,
    Playing,
    /// Returned once, on the frame the scene ends. Send `Command::CutsceneFinished`.
    Finished {
        id: String,
        choices: Vec<ChoiceMade>,
    },
}

/// Player input gathered by `draw_ui` (or synthesised by tests / fast-forward).
#[derive(Debug, Clone, Copy, Default)]
pub struct StepInput {
    pub advance: bool,
    pub choice: Option<usize>,
    /// Hold-to-skip completed.
    pub skip: bool,
    /// Ctrl / the Skip button: speed timers up.
    pub fast: bool,
}

enum Active {
    Text {
        speaker: Option<String>,
        expr: Expression,
        key: String,
        t: f32,
    },
    Choice {
        options: Vec<(usize, ChoiceOption)>,
        t: f32,
    },
    Timer(f32),
    Fade {
        from: f32,
        to: f32,
        dur: f32,
        t: f32,
    },
    Walk(String),
    Train,
    Note {
        id: String,
        t: f32,
    },
    Lights,
    Phone {
        key: String,
        t: f32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum TrainPhase {
    Pass,
    Arriving,
    Stopped,
    Departing,
}

struct TrainAnim {
    phase: TrainPhase,
    t: f32,
    p: f32,
}

struct Cam {
    center: Vec2,
    zoom: f32,
    from: (Vec2, f32),
    to_center: Vec2,
    to_zoom: f32,
    t: f32,
    dur: f32,
    follow: Option<String>,
}

/// Plays cutscenes. Keep one; call `start` for each scene.
///
/// ```ignore
/// let mut player = CutscenePlayer::new(&content);
/// player.start(content.cutscene("opening").unwrap());
/// loop {
///     ui.begin_frame();
///     let loc = Locale::new(&i18n, settings.lang());
///     match player.update(&mut ui, &loc, ui.dt()) { CutsceneStatus::Finished { id, choices } => …, _ => {} }
///     player.draw_world();            // or draw the live world with player.world_camera()
///     ui.set_camera();
///     player.draw_ui(&mut ui, &loc);
///     for a in player.drain_audio_requests() { audio.request(a) }
///     ui.end_frame();
/// }
/// ```
pub struct CutscenePlayer {
    lib: CutsceneLib,
    cast: Cast,
    note_numbers: HashMap<String, u32>,
    pub scene: CafeScene,
    id: String,
    skippable: bool,
    queue: VecDeque<Op>,
    active: Option<Active>,
    playing: bool,
    finished_pending: bool,
    outcome: CutsceneOutcome,
    local_flags: HashSet<String>,
    local_items: HashSet<String>,
    host_check: Option<ConditionCheck>,
    audio: Vec<AudioRequest>,
    effects: Vec<Effect>,
    pub actors: Vec<Actor>,
    hand_light: HashSet<String>,
    cam: Cam,
    train: Option<TrainAnim>,
    fade: f32,
    bars: f32,
    bars_target: f32,
    letter: Option<Vec<String>>,
    letter_appear: f32,
    last_line: Option<(Option<String>, Expression, String)>,
    pending: StepInput,
    skip_hold: f32,
    typed: f32,
    card_appear: f32,
    time: f32,
    calls: u32,
    pub dialogue: DialogueBox,
    /// Lines advance on their own (settings → auto-advance; `A` toggles).
    pub auto_advance: bool,
    /// Walkability of the live café (paths go around the player's own furniture).
    nav: Option<Map>,
    /// Feet of people already in the live café, by cast id (regulars, `guest_anwar` …).
    live: HashMap<String, Vec2>,
    /// Cast ids that left the stage during this scene (the live copy stays hidden).
    exited: HashSet<String>,
}

/// Story-condition callback supplied by the integration.
type ConditionCheck = Box<dyn FnMut(&Condition) -> bool>;

const SKIP_HOLD_SECONDS: f32 = 0.8;

impl CutscenePlayer {
    pub fn new(content: &Content) -> CutscenePlayer {
        CutscenePlayer {
            lib: content.cutscenes.clone(),
            cast: Cast::from_content(content),
            note_numbers: content
                .story
                .notes
                .iter()
                .map(|n| (n.id.clone(), n.number))
                .collect(),
            scene: CafeScene::new(),
            id: String::new(),
            skippable: true,
            queue: VecDeque::new(),
            active: None,
            playing: false,
            finished_pending: false,
            outcome: CutsceneOutcome::default(),
            local_flags: HashSet::new(),
            local_items: HashSet::new(),
            host_check: None,
            audio: Vec::new(),
            effects: Vec::new(),
            actors: Vec::new(),
            hand_light: HashSet::new(),
            cam: Cam {
                center: OVERVIEW_CENTER,
                zoom: 1.0,
                from: (OVERVIEW_CENTER, 1.0),
                to_center: OVERVIEW_CENTER,
                to_zoom: 1.0,
                t: 1.0,
                dur: 1.0,
                follow: None,
            },
            train: None,
            fade: 0.0,
            bars: 0.0,
            bars_target: 0.0,
            letter: None,
            letter_appear: 0.0,
            last_line: None,
            pending: StepInput::default(),
            skip_hold: 0.0,
            typed: 0.0,
            card_appear: 0.0,
            time: 0.0,
            calls: 0,
            dialogue: DialogueBox::new(),
            auto_advance: false,
            nav: None,
            live: HashMap::new(),
            exited: HashSet::new(),
        }
    }

    /// Story conditions for `If` steps and choice options, answered by the sim.
    /// Flags/items set inside the running scene are always known to the player itself.
    pub fn set_condition_check(&mut self, f: impl FnMut(&Condition) -> bool + 'static) {
        self.host_check = Some(Box::new(f));
    }

    /// Typewriter speed (`Settings::text_speed.chars_per_sec()`).
    pub fn set_text_speed(&mut self, chars_per_sec: f32) {
        self.dialogue.chars_per_sec = chars_per_sec;
    }

    /// Applies the player's settings (typewriter speed, auto-advance).
    pub fn apply_settings(&mut self, s: &crate::settings::Settings) {
        self.dialogue.chars_per_sec = s.text_speed.chars_per_sec();
        self.auto_advance = s.auto_advance;
    }

    /// Walkability of the live café for actor paths (`None` = the stage's own layout).
    pub fn set_nav_map(&mut self, map: Option<Map>) {
        self.nav = map;
    }

    /// People already in the live café, by cast id. A scene that moves, exits or looks at one
    /// of them starts from where they really are. Call before [`CutscenePlayer::start`].
    pub fn set_live_cast(&mut self, cast: impl IntoIterator<Item = (String, Vec2)>) {
        self.live = cast.into_iter().collect();
    }

    /// Cast ids the scene plays itself right now (on stage or gone through a door): the host
    /// hides its live copies of them.
    pub fn stage_cast(&self) -> impl Iterator<Item = &str> {
        self.actors
            .iter()
            .map(|a| a.id.as_str())
            .chain(self.exited.iter().map(String::as_str))
    }

    /// Progress 0..1 of the scene's own train (`None` = no train).
    pub fn train_progress(&self) -> Option<f32> {
        self.train.as_ref().map(|t| t.p)
    }

    /// Starts (or restarts) a scene. The stage is reset: lights on, no actors.
    pub fn start(&mut self, def: &CutsceneDef) {
        self.id = def.id.clone();
        self.skippable = def.skippable;
        self.queue = staging::expand(&def.id, &def.steps, true).into();
        self.active = None;
        self.playing = true;
        self.finished_pending = false;
        self.outcome = CutsceneOutcome {
            id: def.id.clone(),
            ..Default::default()
        };
        self.local_flags.clear();
        self.local_items.clear();
        self.actors.clear();
        self.exited.clear();
        self.hand_light.clear();
        self.train = None;
        self.scene.train = None;
        self.scene.set_lights(true);
        // a scene that starts in the dark café must not flash lit for a frame
        while let Some(Op::Stage(Stage::CafeDark)) = self.queue.front() {
            self.queue.pop_front();
            self.scene.set_lights(false);
        }
        self.fade = 0.0;
        self.letter = None;
        self.letter_appear = 0.0;
        self.last_line = None;
        self.pending = StepInput::default();
        self.skip_hold = 0.0;
        self.calls = 0;
        self.cam.follow = None;
        self.cam.t = self.cam.dur;
        let hist = std::mem::take(&mut self.dialogue.history);
        let (cps, skip) = (self.dialogue.chars_per_sec, false);
        self.dialogue = DialogueBox::new();
        self.dialogue.chars_per_sec = cps;
        self.dialogue.skip_on = skip;
        drop(hist);
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }
    /// Dialogue, narration, a choice, a note, the letter or the phone message is on screen
    /// (the music ducks under it).
    pub fn text_on_screen(&self) -> bool {
        self.playing
            && matches!(
                self.active,
                Some(
                    Active::Text { .. }
                        | Active::Choice { .. }
                        | Active::Note { .. }
                        | Active::Phone { .. }
                )
            )
    }
    pub fn current_id(&self) -> &str {
        &self.id
    }
    pub fn outcome(&self) -> &CutsceneOutcome {
        &self.outcome
    }
    pub fn drain_audio_requests(&mut self) -> Vec<AudioRequest> {
        std::mem::take(&mut self.audio)
    }
    /// Effects applied since the last call (apply them live so later `If` steps see them).
    pub fn drain_effects(&mut self) -> Vec<Effect> {
        std::mem::take(&mut self.effects)
    }
    /// Current camera (world centre, zoom multiplier — 1.0 = overview).
    pub fn camera_view(&self) -> (Vec2, f32) {
        (self.cam.center, self.cam.zoom)
    }
    /// The macroquad camera for drawing the world of this cutscene.
    pub fn world_camera(&self) -> Camera2D {
        self.scene.camera(self.cam.center, self.cam.zoom)
    }
    /// Seconds since the player was created (world animation time).
    pub fn time(&self) -> f32 {
        self.time
    }

    // ───────────────────────── frame ─────────────────────────

    /// Advances the scene. Reads skip / auto keys; dialogue clicks come from the last `draw_ui`.
    pub fn update(&mut self, ui: &mut Ui, loc: &Locale, dt: f32) -> CutsceneStatus {
        if !self.playing {
            if self.finished_pending {
                self.finished_pending = false;
                return CutsceneStatus::Finished {
                    id: self.outcome.id.clone(),
                    choices: self.outcome.choices.clone(),
                };
            }
            self.tick_world(dt);
            return CutsceneStatus::Idle;
        }
        let mut input = std::mem::take(&mut self.pending);
        if !ui.wants_keyboard() {
            if self.skippable && is_key_down(KeyCode::Escape) {
                self.skip_hold += dt;
                if self.skip_hold >= SKIP_HOLD_SECONDS {
                    input.skip = true;
                    self.skip_hold = 0.0;
                }
            } else {
                self.skip_hold = (self.skip_hold - dt * 3.0).max(0.0);
            }
            if ui.key(KeyCode::A) {
                self.auto_advance = !self.auto_advance;
            }
        }
        input.fast |= self.dialogue.skip_on || ui.ctrl();
        self.step(loc, dt, input)
    }

    /// Pure logic step (tests, fast-forward). `update` = input gathering + this.
    pub fn step(&mut self, loc: &Locale, dt: f32, input: StepInput) -> CutsceneStatus {
        if !self.playing {
            return CutsceneStatus::Idle;
        }
        if input.skip && self.skippable {
            self.skip_to_end(loc);
        } else {
            let k = if input.fast { 4.0 } else { 1.0 };
            self.tick_world(dt * k);
            self.run(loc, dt * k, input);
        }
        if !self.playing {
            self.finished_pending = false;
            return CutsceneStatus::Finished {
                id: self.outcome.id.clone(),
                choices: self.outcome.choices.clone(),
            };
        }
        CutsceneStatus::Playing
    }

    /// Auto-plays `seconds` of the scene at the default pace (demos jump with `SHIFTELLA_T`).
    /// Choices pick their first option.
    pub fn fast_forward(&mut self, loc: &Locale, seconds: f32) {
        let auto = self.auto_advance;
        self.auto_advance = true;
        let dt = 1.0 / 30.0;
        let mut t = 0.0;
        while t < seconds && self.playing {
            let choice =
                matches!(self.active, Some(Active::Choice { t, .. }) if t > 1.2).then_some(0);
            self.step(
                loc,
                dt,
                StepInput {
                    choice,
                    ..Default::default()
                },
            );
            t += dt;
        }
        self.auto_advance = auto;
    }

    fn tick_world(&mut self, dt: f32) {
        self.time += dt;
        self.scene.update(dt);
        for a in &mut self.actors {
            a.update(dt);
        }
        self.actors.retain(|a| !a.gone);
        let ease = |x: f32| x * x * (3.0 - 2.0 * x);
        // train
        if let Some(tr) = &mut self.train {
            tr.t += dt;
            match tr.phase {
                TrainPhase::Pass => tr.p = tr.t / 7.5,
                TrainPhase::Arriving => {
                    let k = (tr.t / 4.5).min(1.0);
                    tr.p = TRAIN_STOP * (1.0 - (1.0 - k).powi(3));
                    if k >= 1.0 {
                        tr.phase = TrainPhase::Stopped;
                        tr.t = 0.0;
                    }
                }
                TrainPhase::Stopped => tr.p = TRAIN_STOP,
                TrainPhase::Departing => {
                    let k = tr.t / 5.0;
                    tr.p = TRAIN_STOP + (1.0 - TRAIN_STOP) * k * k;
                }
            }
            if tr.p > 1.0 {
                self.train = None;
            }
        }
        self.scene.train = self.train.as_ref().map(|t| t.p);
        // camera
        let follow_pos = self
            .cam
            .follow
            .as_ref()
            .and_then(|id| self.actors.iter().find(|a| &a.id == id))
            .map(|a| a.pos - vec2(0.0, 0.4 * TILE) + vec2(0.0, 2.4 * TILE / self.cam.to_zoom));
        if let Some(p) = follow_pos {
            self.cam.to_center = p;
        }
        if self.cam.t < self.cam.dur {
            self.cam.t += dt;
            let k = ease((self.cam.t / self.cam.dur).min(1.0));
            self.cam.center = self.cam.from.0.lerp(self.cam.to_center, k);
            self.cam.zoom = (self.cam.from.1.ln() * (1.0 - k) + self.cam.to_zoom.ln() * k).exp();
        } else if follow_pos.is_some() {
            let k = 1.0 - (-dt / 0.35).exp();
            self.cam.center = self.cam.center.lerp(self.cam.to_center, k);
        }
        // letterbox, cards
        let bk = dt / 0.6;
        self.bars = if self.bars < self.bars_target {
            (self.bars + bk).min(self.bars_target)
        } else {
            (self.bars - bk).max(self.bars_target)
        };
        let la = if self.letter.is_some() { 1.0 } else { 0.0 };
        // move towards the target (`signum(0.0)` is 1.0, which made the card flicker)
        let step = dt / 0.45;
        self.letter_appear = if self.letter_appear < la {
            (self.letter_appear + step).min(la)
        } else {
            (self.letter_appear - step).max(la)
        };
        self.card_appear = (self.card_appear + dt / 0.45).min(1.0);
        let cps = self.dialogue.chars_per_sec;
        self.typed += if cps <= 0.0 { 1.0e6 } else { cps * dt };
    }

    fn text_len(loc: &Locale, key: &str) -> usize {
        loc.tr_owned(key).chars().count()
    }

    fn auto_time(&self, chars: usize) -> f32 {
        let cps = self.dialogue.chars_per_sec;
        let reveal = if cps <= 0.0 { 0.0 } else { chars as f32 / cps };
        reveal + (1.1 + chars as f32 * 0.03).min(5.0)
    }

    fn run(&mut self, loc: &Locale, dt: f32, input: StepInput) {
        let mut budget = 400;
        let mut dt = dt;
        let mut input = input;
        loop {
            budget -= 1;
            if budget == 0 {
                break;
            }
            if self.active.is_none() {
                let Some(op) = self.queue.pop_front() else {
                    self.finish();
                    return;
                };
                self.begin(loc, op);
                continue;
            }
            if self.update_active(loc, dt, &input) {
                self.active = None;
                dt = 0.0;
                input = StepInput::default();
                continue;
            }
            break;
        }
    }

    fn finish(&mut self) {
        self.playing = false;
        self.bars_target = 0.0;
        self.letter = None;
        self.active = None;
        for a in &mut self.actors {
            a.hand_light = false;
        }
    }

    fn update_active(&mut self, loc: &Locale, dt: f32, input: &StepInput) -> bool {
        let auto = self.auto_advance;
        if let Some(Active::Text { key, t, .. }) = &mut self.active {
            *t += dt;
            let (key, elapsed) = (key.clone(), *t);
            let need = self.auto_time(Self::text_len(loc, &key));
            let done = input.advance || (auto && elapsed >= need);
            if done {
                self.end_text(loc);
            }
            return done;
        }
        match &mut self.active {
            None | Some(Active::Text { .. }) => true,
            Some(Active::Choice { options, t }) => {
                *t += dt;
                if let Some(i) = input.choice
                    && i < options.len()
                {
                    let (index, opt) = options[i].clone();
                    self.pick(index, &opt);
                    return true;
                }
                false
            }
            Some(Active::Timer(left)) => {
                *left -= dt;
                *left <= 0.0
            }
            Some(Active::Fade { from, to, dur, t }) => {
                *t += dt;
                let k = (*t / dur.max(1e-3)).min(1.0);
                self.fade = *from + (*to - *from) * k;
                k >= 1.0
            }
            Some(Active::Walk(id)) => {
                let id = id.clone();
                self.actors
                    .iter()
                    .find(|a| a.id == id)
                    .is_none_or(|a| a.arrived())
            }
            Some(Active::Train) => match &self.train {
                None => true,
                Some(tr) => match tr.phase {
                    TrainPhase::Pass => tr.p >= 0.3,
                    TrainPhase::Arriving => false,
                    _ => true,
                },
            },
            Some(Active::Note { t, .. }) => {
                *t += dt;
                input.advance || (auto && *t > 6.0)
            }
            Some(Active::Lights) => self.scene.cascade_done(),
            Some(Active::Phone { t, .. }) => {
                *t += dt;
                input.advance || (auto && *t > 6.0)
            }
        }
    }

    fn end_text(&mut self, loc: &Locale) {
        if let Some(Active::Text { speaker, key, .. }) = &self.active
            && let Some(letter) = &mut self.letter
            && speaker.as_deref() == Some("grandma")
        {
            letter.push(loc.tr_owned(key));
        }
    }

    fn pick(&mut self, index: usize, opt: &ChoiceOption) {
        self.outcome.choices.push(ChoiceMade {
            index,
            label_key: opt.label_key.clone(),
            set_flag: opt.set_flag.clone(),
        });
        for e in &opt.effects {
            self.apply_effect(e.clone());
        }
        if let Some(f) = &opt.set_flag {
            self.apply_effect(Effect::SetFlag(f.clone()));
        }
    }

    fn apply_effect(&mut self, e: Effect) {
        match &e {
            Effect::SetFlag(f) => {
                self.local_flags.insert(f.clone());
                self.outcome.flags.push(f.clone());
            }
            Effect::ClearFlag(f) => {
                self.local_flags.remove(f);
            }
            Effect::GiveItem(i) => {
                self.local_items.insert(i.clone());
            }
            _ => {}
        }
        self.outcome.effects.push(e.clone());
        self.effects.push(e);
    }

    fn check(&mut self, conds: &[Condition]) -> bool {
        conds.iter().all(|c| {
            let local = match c {
                Condition::Flag(f) if self.local_flags.contains(f) => Some(true),
                Condition::NotFlag(f) if self.local_flags.contains(f) => Some(false),
                Condition::HasItem(i) if self.local_items.contains(i) => Some(true),
                _ => None,
            };
            local.unwrap_or_else(|| match &mut self.host_check {
                Some(f) => f(c),
                None => matches!(c, Condition::NotFlag(_)),
            })
        })
    }

    fn spot(&self, name: &str) -> Option<(i32, i32)> {
        self.lib.spots.get(name).copied()
    }

    fn place_px(&self, p: &Place) -> Vec2 {
        match p {
            Place::Tile(x, y) => tile_feet(*x, *y),
            Place::Spot(s) => self
                .spot(s)
                .map(|(x, y)| tile_feet(x, y))
                .unwrap_or(OVERVIEW_CENTER),
        }
    }

    fn door_px(&self) -> Vec2 {
        let (x, y) = self.spot("door").unwrap_or((1, 14));
        tile_feet(x, y)
    }

    fn camera_to(&mut self, focus: &Focus, zoom: f32) {
        let (center, follow) = match focus {
            Focus::Tile(x, y) => (vec2(*x as f32 + 0.5, *y as f32 + 0.5) * TILE, None),
            Focus::Spot(s) => (
                self.spot(s)
                    .map(|(x, y)| vec2(x as f32 + 0.5, y as f32 + 0.5) * TILE)
                    .unwrap_or(OVERVIEW_CENTER),
                None,
            ),
            Focus::Overview => (OVERVIEW_CENTER, None),
            Focus::Actor(a) => match self.actors.iter().find(|x| &x.id == a) {
                Some(x) => (x.pos - vec2(0.0, 0.4 * TILE), Some(a.clone())),
                None => match self.live.get(a) {
                    Some(p) => (*p - vec2(0.0, 0.4 * TILE), None),
                    None => (self.cam.to_center, None),
                },
            },
        };
        let zoom = zoom.clamp(0.4, 4.0);
        // frame people/spots in the upper part of the view: the dialogue box covers the bottom
        let center = match focus {
            Focus::Overview => center,
            _ => center + vec2(0.0, 2.4 * TILE / zoom),
        };
        let dist = self.cam.center.distance(center) / TILE;
        let dz = (zoom / self.cam.zoom).ln().abs();
        self.cam.from = (self.cam.center, self.cam.zoom);
        self.cam.to_center = center;
        self.cam.to_zoom = zoom;
        self.cam.dur = (1.1 + dist * 0.06 + dz * 0.8).min(2.6);
        self.cam.t = 0.0;
        self.cam.follow = follow;
    }

    /// Snap the camera (start of a scene, skipping).
    fn camera_snap(&mut self) {
        self.cam.t = self.cam.dur;
        self.cam.center = self.cam.to_center;
        self.cam.zoom = self.cam.to_zoom;
    }

    fn actor_mut(&mut self, id: &str) -> Option<&mut Actor> {
        self.actors.iter_mut().find(|a| a.id == id && !a.gone)
    }

    fn spawn(&mut self, id: &str, at: Vec2) -> &mut Actor {
        self.actors.retain(|a| a.id != id);
        let mut a = Actor::new(id, self.cast.look(id), at);
        a.hand_light = self.hand_light.contains(id);
        self.actors.push(a);
        self.actors.last_mut().expect("just pushed")
    }

    fn walk(&mut self, id: &str, to: Vec2, exit: bool, enter: bool) {
        let exists = self.actor_mut(id).is_some();
        if !exists && let Some(at) = self.live.get(id).copied() {
            // already in the café: start from where they sit / stand, fully visible
            self.spawn(id, at).alpha = 1.0;
        } else if !exists || enter && !exists {
            let start = if to.y < 0.0 {
                vec2(self.scene.street.x + 2.0 * TILE, to.y)
            } else {
                self.door_px()
            };
            self.spawn(id, start);
        }
        let from = self.actor_mut(id).map(|a| a.pos).unwrap_or(to);
        let path = find_path(self.nav.as_ref().unwrap_or(&self.scene.map), from, to);
        if let Some(a) = self.actor_mut(id) {
            a.exit_on_arrive = exit;
            a.walk(path);
        }
        // camera steps right after a walk start together with it
        while let Some(Op::Step(CutsceneStep::Camera { .. })) = self.queue.front() {
            if let Some(Op::Step(CutsceneStep::Camera { focus, zoom })) = self.queue.pop_front() {
                self.camera_to(&focus, zoom);
            }
        }
        self.active = Some(Active::Walk(id.to_string()));
    }

    fn begin(&mut self, loc: &Locale, op: Op) {
        self.typed = 0.0;
        match op {
            Op::Step(step) => self.begin_step(loc, step),
            Op::Stage(stage) => self.begin_stage(stage),
        }
    }

    fn begin_step(&mut self, _loc: &Locale, step: CutsceneStep) {
        match step {
            CutsceneStep::Narration { text_key } => {
                self.last_line = Some((None, Expression::Neutral, text_key.clone()));
                self.active = Some(Active::Text {
                    speaker: None,
                    expr: Expression::Neutral,
                    key: text_key,
                    t: 0.0,
                });
            }
            CutsceneStep::Line {
                speaker,
                expression: e,
                text_key,
            } => {
                let expr = expression(e);
                self.last_line = Some((Some(speaker.clone()), expr, text_key.clone()));
                self.active = Some(Active::Text {
                    speaker: Some(speaker),
                    expr,
                    key: text_key,
                    t: 0.0,
                });
            }
            CutsceneStep::Choice { options } => {
                let shown: Vec<(usize, ChoiceOption)> = options
                    .into_iter()
                    .enumerate()
                    .filter(|(_, o)| {
                        let conds = o.conditions.clone();
                        self.check(&conds)
                    })
                    .collect();
                if !shown.is_empty() {
                    self.active = Some(Active::Choice {
                        options: shown,
                        t: 0.0,
                    });
                }
            }
            CutsceneStep::Camera { focus, zoom } => {
                self.camera_to(&focus, zoom);
                // the first camera of a scene that starts in black snaps
                if self.fade >= 0.99 {
                    self.camera_snap();
                }
            }
            CutsceneStep::ActorEnter { actor, to_tile } => {
                let to = self.place_px(&to_tile);
                self.walk(&actor, to, false, true);
            }
            CutsceneStep::ActorMove { actor, to_tile } => {
                let to = self.place_px(&to_tile);
                self.walk(&actor, to, false, false);
            }
            CutsceneStep::ActorExit { actor, to_tile } => {
                let to = self.place_px(&to_tile);
                if self.actor_mut(&actor).is_some() || self.live.contains_key(&actor) {
                    self.walk(&actor, to, true, false);
                    self.exited.insert(actor);
                }
            }
            CutsceneStep::Wait { seconds } => self.active = Some(Active::Timer(seconds)),
            CutsceneStep::Fade { to_black, seconds } => {
                let to = if to_black { 1.0 } else { 0.0 };
                if seconds <= 0.0 {
                    self.fade = to;
                } else {
                    self.active = Some(Active::Fade {
                        from: self.fade,
                        to,
                        dur: seconds,
                        t: 0.0,
                    });
                }
            }
            CutsceneStep::Music { track } => self.audio.push(AudioRequest::Music { track }),
            CutsceneStep::Sfx { id } => self.audio.push(AudioRequest::Sfx { id, volume: 1.0 }),
            CutsceneStep::TrainPass => {
                self.train = Some(TrainAnim {
                    phase: TrainPhase::Pass,
                    t: 0.0,
                    p: 0.0,
                });
                self.audio.push(AudioRequest::Sfx {
                    id: "amb_train_pass_far".into(),
                    volume: 1.0,
                });
                self.active = Some(Active::Train);
            }
            CutsceneStep::Letterbox { on } => self.bars_target = if on { 1.0 } else { 0.0 },
            CutsceneStep::ShowNote { note_id } => {
                self.outcome.notes.push(note_id.clone());
                self.apply_effect(Effect::FindNote(note_id.clone()));
                self.card_appear = 0.0;
                self.active = Some(Active::Note {
                    id: note_id,
                    t: 0.0,
                });
            }
            CutsceneStep::If {
                conditions,
                then,
                otherwise,
            } => {
                let branch = if self.check(&conditions) {
                    then
                } else {
                    otherwise
                };
                for op in staging::expand(&self.id, &branch, false).into_iter().rev() {
                    self.queue.push_front(op);
                }
            }
            CutsceneStep::Call { cutscene } => {
                self.calls += 1;
                if self.calls < 16
                    && let Some(def) = self.lib.get(&cutscene)
                {
                    let steps = def.steps.clone();
                    for op in staging::expand(&cutscene, &steps, false).into_iter().rev() {
                        self.queue.push_front(op);
                    }
                }
            }
            CutsceneStep::Apply { effects } => {
                for e in effects {
                    self.apply_effect(e);
                }
            }
        }
    }

    fn begin_stage(&mut self, stage: Stage) {
        match stage {
            Stage::CafeDark => self.scene.set_lights(false),
            Stage::TrainArrive => {
                self.train = Some(TrainAnim {
                    phase: TrainPhase::Arriving,
                    t: 0.0,
                    p: 0.0,
                });
                self.audio.push(AudioRequest::Sfx {
                    id: "amb_train_pass_far".into(),
                    volume: 0.8,
                });
                self.active = Some(Active::Train);
            }
            Stage::TrainDepart => {
                if let Some(tr) = &mut self.train {
                    tr.phase = TrainPhase::Departing;
                    tr.t = 0.0;
                }
            }
            Stage::Appear {
                actor,
                at,
                suitcase,
            } => {
                let a = self.spawn(&actor, at);
                a.suitcase = suitcase;
                a.facing = crate::render::Facing::Down;
            }
            Stage::Walk { actor, to } => {
                let from = self.actor_mut(&actor).map(|a| a.pos);
                if let Some(from) = from {
                    let path = find_path(self.nav.as_ref().unwrap_or(&self.scene.map), from, to);
                    if let Some(a) = self.actor_mut(&actor) {
                        a.walk(path);
                    }
                }
            }
            Stage::Hide { actor } => {
                if let Some(a) = self.actor_mut(&actor) {
                    a.gone = true;
                }
                self.exited.insert(actor.clone());
                self.actors.retain(|a| !a.gone);
            }
            Stage::HandLight { actor, on } => {
                if on {
                    self.hand_light.insert(actor.clone());
                } else {
                    self.hand_light.remove(&actor);
                }
                if let Some(a) = self.actor_mut(&actor) {
                    a.hand_light = on;
                }
            }
            Stage::LetterOn => {
                self.letter = Some(Vec::new());
            }
            Stage::LetterOff => {
                self.letter = None;
                self.active = Some(Active::Timer(0.35));
            }
            Stage::LightsCascade { seconds } => {
                self.scene.lights_cascade(seconds);
                self.active = Some(Active::Lights);
            }
            Stage::LightsOut { seconds } => {
                self.scene.lights_out(seconds);
                self.active = Some(Active::Lights);
            }
            Stage::Phone { message_key } => {
                self.card_appear = 0.0;
                self.audio.push(AudioRequest::Sfx {
                    id: "ui_alert_event".into(),
                    volume: 0.7,
                });
                self.active = Some(Active::Phone {
                    key: message_key,
                    t: 0.0,
                });
            }
            Stage::Pause(s) => self.active = Some(Active::Timer(s)),
        }
    }

    /// Hold-to-skip: runs the rest instantly (effects, notes, music state), choices unanswered.
    fn skip_to_end(&mut self, loc: &Locale) {
        self.outcome.skipped = true;
        let mut last_music = None;
        self.active = None;
        let mut guard = 0;
        while let Some(op) = self.queue.pop_front() {
            guard += 1;
            if guard > 2000 {
                break;
            }
            match op {
                Op::Step(CutsceneStep::Music { track }) => last_music = Some(track),
                Op::Step(CutsceneStep::Fade { to_black, .. }) => {
                    self.fade = if to_black { 1.0 } else { 0.0 }
                }
                Op::Step(CutsceneStep::Choice { .. })
                | Op::Step(CutsceneStep::Narration { .. })
                | Op::Step(CutsceneStep::Line { .. })
                | Op::Step(CutsceneStep::Wait { .. })
                | Op::Step(CutsceneStep::Sfx { .. })
                | Op::Step(CutsceneStep::TrainPass)
                | Op::Step(CutsceneStep::Letterbox { .. }) => {}
                Op::Step(CutsceneStep::ShowNote { note_id }) => {
                    self.outcome.notes.push(note_id.clone());
                    self.apply_effect(Effect::FindNote(note_id));
                }
                Op::Step(
                    step @ (CutsceneStep::If { .. }
                    | CutsceneStep::Call { .. }
                    | CutsceneStep::Apply { .. }
                    | CutsceneStep::Camera { .. }),
                ) => self.begin_step(loc, step),
                Op::Step(
                    step @ (CutsceneStep::ActorEnter { .. }
                    | CutsceneStep::ActorMove { .. }
                    | CutsceneStep::ActorExit { .. }),
                ) => {
                    self.begin_step(loc, step);
                    self.active = None;
                }
                Op::Stage(Stage::LightsCascade { .. }) => self.scene.set_lights(true),
                Op::Stage(Stage::LightsOut { .. }) => self.scene.set_lights(false),
                Op::Stage(Stage::Phone { .. } | Stage::Pause(_) | Stage::TrainArrive) => {}
                Op::Stage(s) => {
                    self.begin_stage(s);
                    self.active = None;
                }
            }
        }
        for a in &mut self.actors {
            a.finish();
        }
        self.actors.retain(|a| !a.gone);
        self.train = None;
        self.camera_snap();
        if let Some(track) = last_music {
            self.audio.push(AudioRequest::Music { track });
        }
        self.finish();
    }

    // ───────────────────────── drawing ─────────────────────────

    /// Draws the café/street stage with the cutscene camera. Leaves the world camera set.
    pub fn draw_world(&mut self) {
        let cam = self.world_camera();
        let t = self.time;
        self.scene.draw(&cam, t, &self.actors);
    }

    /// Dialogue, narration, choices, cards, letterbox and fade. Call after `ui.set_camera()`.
    pub fn draw_ui(&mut self, ui: &mut Ui, loc: &Locale) {
        let screen = ui.screen();
        let bars = ui.letterbox(self.bars);
        let bar_h = bars.y;
        self.dialogue.bottom_inset = (bar_h - 12.0).max(0.0);
        if !self.playing {
            ui.fade(self.fade);
            return;
        }
        let labels = DialogueLabels {
            skip: loc.tr("ui.cutscene.skip"),
            log: loc.tr("ui.log.title"),
            history: loc.tr("ui.log.title"),
        };
        let mut input = StepInput::default();
        let t = self.time;
        let typed = self.typed as usize;

        // grandma's letter stays on screen while it is open
        if self.letter_appear > 0.0 {
            let letter = self.letter.clone().unwrap_or_default();
            let mut lines = letter.clone();
            let mut visible = usize::MAX;
            if let Some(Active::Text {
                speaker: Some(s),
                key,
                ..
            }) = &self.active
                && s == "grandma"
            {
                lines.push(loc.tr_owned(key));
                visible = typed;
            }
            cards::letter_card(
                ui,
                loc.tr("cutsceneui.letter.envelope"),
                &lines,
                visible,
                self.letter_appear,
                t,
            );
        }

        match &self.active {
            Some(Active::Text {
                speaker, expr, key, ..
            }) => {
                let text = loc.tr_owned(key);
                let in_letter = self.letter.is_some() && speaker.as_deref() == Some("grandma");
                if in_letter {
                    if Self::card_advance(ui) {
                        if typed < text.chars().count() {
                            self.typed = 1.0e6;
                        } else {
                            input.advance = true;
                        }
                    }
                    self.hint(ui, loc, bar_h);
                } else if let Some(sp) = speaker {
                    let name = self.cast.name(loc, sp);
                    let portrait = self.cast.has_portrait(sp);
                    let line = DialogueLine::new(&name, &text)
                        .portrait(portrait)
                        .color(self.cast.color(sp))
                        .labels(labels);
                    let r = self.dialogue.dialogue_box(ui, &line);
                    if let Some(pr) = r.portrait {
                        draw_portrait(&self.cast.look(sp), *expr, pr, t);
                    }
                    input.advance = r.advance;
                } else {
                    let r = self.dialogue.narration_box(ui, &text);
                    input.advance = r.advance;
                }
            }
            Some(Active::Choice { options, .. }) => {
                let labels_txt: Vec<String> = options
                    .iter()
                    .map(|(_, o)| loc.tr_owned(&o.label_key))
                    .collect();
                let above = match &self.last_line {
                    Some((Some(sp), expr, key)) => {
                        let name = self.cast.name(loc, sp);
                        let text = loc.tr_owned(key);
                        let line = DialogueLine::new(&name, &text)
                            .portrait(self.cast.has_portrait(sp))
                            .color(self.cast.color(sp))
                            .awaiting_choice(true)
                            .labels(labels);
                        let r = self.dialogue.dialogue_box(ui, &line);
                        if let Some(pr) = r.portrait {
                            draw_portrait(&self.cast.look(sp), *expr, pr, t);
                        }
                        r.rect
                    }
                    _ => Rect::new(screen.w / 2.0 - 300.0, screen.h - bar_h - 60.0, 600.0, 40.0),
                };
                let refs: Vec<&str> = labels_txt.iter().map(String::as_str).collect();
                input.choice = ui.choice_list(
                    ("cutscene_choice", self.outcome.choices.len()),
                    above,
                    &refs,
                );
            }
            Some(Active::Note { id, .. }) => {
                let text = loc.tr_owned(&format!("note.{id}.text"));
                let wh = loc.tr_owned(&format!("note.{id}.where"));
                let num = self
                    .note_numbers
                    .get(id)
                    .map(|n| format!("{n} / 10"))
                    .unwrap_or_default();
                cards::note_card(ui, &wh, &num, &text, typed, self.card_appear);
                if Self::card_advance(ui) {
                    if typed < text.chars().count() {
                        self.typed = 1.0e6;
                    } else {
                        input.advance = true;
                    }
                }
                self.hint(ui, loc, bar_h);
            }
            Some(Active::Phone { key, .. }) => {
                let msg = loc.tr_owned(key);
                let today = loc.tr("cutsceneui.phone.today");
                let pl = PhoneLabels {
                    time: "22:04",
                    title: loc.tr("ui.app.title"),
                    subtitle: "Café Nocturne · Holešovice",
                    today,
                    bot_handle: "@shiftella_bot",
                    tab_schedule: loc.tr("ui.app.tab.schedule"),
                    tab_bot: loc.tr("ui.app.tab.bot"),
                    tab_board: loc.tr("ui.app.tab.announcements"),
                    tab_checklists: loc.tr("ui.app.tab.checklists"),
                };
                let delay = 0.9;
                let shown =
                    ((self.typed - delay * self.dialogue.chars_per_sec.max(1.0)).max(0.0)) as usize;
                cards::phone_bot(ui, &pl, &msg, shown, self.card_appear, t);
                // a caption on the left: the night begins
                let cap = TextStyle::new(22.0, Weight::Bold, ui.theme.text).tracking(-0.3);
                let sub = ui.theme.body().color(ui.theme.text_dim);
                let x = 64.0;
                let y = screen.h * 0.42;
                let shade = Rect::new(0.0, 0.0, 560.0, screen.h);
                let ink = ui.theme.canvas[0];
                crate::ui::draw::fill_rrect(
                    shade,
                    0.0,
                    crate::ui::Paint::horizontal(
                        shade,
                        crate::ui::draw::with_a(ink, 0.85 * self.card_appear),
                        crate::ui::draw::with_a(ink, 0.0),
                    ),
                );
                ui.fonts
                    .draw(loc.tr("cutsceneui.phone.caption"), x, y, &cap);
                ui.fonts
                    .draw(loc.tr("cutsceneui.phone.caption_sub"), x, y + 26.0, &sub);
                if Self::card_advance(ui) {
                    if shown < msg.chars().count() {
                        self.typed = 1.0e6;
                    } else {
                        input.advance = true;
                    }
                }
                self.hint(ui, loc, bar_h);
            }
            _ => {}
        }

        // corner chrome: hold-to-skip + auto
        let micro = ui.theme.small().size(11.0);
        let right = screen.w - 20.0;
        let y = (bar_h * 0.5 + 4.0).max(22.0);
        if self.skippable {
            let label = loc.tr("ui.cutscene.hold_to_skip");
            let w = ui.fonts.measure(label, &micro);
            ui.fonts.draw(
                label,
                right - w,
                y,
                &micro.color(crate::ui::draw::with_a(ui.theme.text_dim, 0.75)),
            );
            if self.skip_hold > 0.0 {
                let k = (self.skip_hold / SKIP_HOLD_SECONDS).min(1.0);
                let c = vec2(right - w - 14.0, y - 4.0);
                crate::ui::draw::arc(
                    c,
                    6.0,
                    -std::f32::consts::FRAC_PI_2,
                    -std::f32::consts::FRAC_PI_2 + k * std::f32::consts::TAU,
                    2.0,
                    ui.theme.indigo_300,
                );
            }
        }
        if self.auto_advance {
            let label = loc.tr("cutsceneui.auto");
            ui.chip(
                vec2(20.0, y - 15.0),
                label,
                crate::ui::Tone::Accent,
                Some(crate::ui::Icon::Play),
            );
        }
        ui.fade(self.fade);
        self.pending.advance |= input.advance;
        if input.choice.is_some() {
            self.pending.choice = input.choice;
        }
    }

    fn card_advance(ui: &mut Ui) -> bool {
        let s = ui.screen();
        ui.region(s);
        let id = ui.id("cutscene_card_catch");
        let c = ui.interact(id, s);
        let k = ui.take_key(KeyCode::Space) || ui.enter();
        c.clicked || k
    }

    fn hint(&self, ui: &mut Ui, loc: &Locale, bar_h: f32) {
        let s = ui.screen();
        let st = ui
            .theme
            .small()
            .color(crate::ui::draw::with_a(ui.theme.text_dim, 0.8));
        let label = loc.tr("ui.cutscene.next");
        let w = ui.fonts.measure(label, &st);
        let blink = 0.55 + 0.45 * (self.time * 2.6).sin();
        crate::ui::draw::push_alpha(blink);
        ui.fonts.draw(
            label,
            (s.w - w) / 2.0,
            s.h - bar_h.max(24.0) * 0.5 - 0.0 + 4.0 - (if bar_h > 0.0 { 0.0 } else { 12.0 }),
            &st,
        );
        crate::ui::draw::pop_alpha();
        let _ = s.inset(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{ChoiceOption, CutsceneTrigger, Focus};
    use crate::i18n::{I18n, Lang};

    const ROOT: &str = env!("CARGO_MANIFEST_DIR");

    fn setup() -> (CutscenePlayer, I18n, Content) {
        let content = Content::load(&format!("{ROOT}/assets/data")).expect("content");
        let i18n = crate::settings::load_i18n(&format!("{ROOT}/assets/i18n")).expect("i18n");
        (CutscenePlayer::new(&content), i18n, content)
    }

    fn def(id: &str, steps: Vec<CutsceneStep>) -> CutsceneDef {
        CutsceneDef {
            id: id.into(),
            trigger: CutsceneTrigger::Manual,
            conditions: vec![],
            skippable: true,
            repeat: false,
            steps,
        }
    }

    fn narration(k: &str) -> CutsceneStep {
        CutsceneStep::Narration { text_key: k.into() }
    }

    fn opt(label: &str, flag: Option<&str>, conds: Vec<Condition>) -> ChoiceOption {
        ChoiceOption {
            label_key: label.into(),
            effects: vec![Effect::Money(-10)],
            set_flag: flag.map(str::to_string),
            conditions: conds,
        }
    }

    fn advance() -> StepInput {
        StepInput {
            advance: true,
            ..Default::default()
        }
    }

    #[test]
    fn steps_run_in_order_and_wait_for_clicks() {
        let (mut p, i18n, _) = setup();
        let loc = Locale::new(&i18n, Lang::En);
        p.start(&def(
            "t",
            vec![
                CutsceneStep::Music {
                    track: Some("mus_night_calm_loop".into()),
                },
                narration("scene.opening.01"),
                CutsceneStep::Wait { seconds: 1.0 },
                CutsceneStep::Sfx {
                    id: "sfx_door_bell".into(),
                },
                CutsceneStep::Camera {
                    focus: Focus::Overview,
                    zoom: 1.2,
                },
                narration("scene.opening.03"),
            ],
        ));
        // music starts at once, then the first narration waits
        for _ in 0..30 {
            assert_eq!(
                p.step(&loc, 0.1, StepInput::default()),
                CutsceneStatus::Playing
            );
        }
        assert_eq!(
            p.drain_audio_requests(),
            vec![AudioRequest::Music {
                track: Some("mus_night_calm_loop".into())
            }]
        );
        p.step(&loc, 0.016, advance());
        // the wait blocks the sfx for a second
        p.step(&loc, 0.5, StepInput::default());
        assert!(p.drain_audio_requests().is_empty());
        p.step(&loc, 0.6, StepInput::default());
        assert_eq!(p.drain_audio_requests().len(), 1);
        // the camera is non-blocking: we are on the last narration now
        assert_eq!(
            p.step(&loc, 0.1, advance()),
            CutsceneStatus::Finished {
                id: "t".into(),
                choices: vec![]
            }
        );
        assert!(!p.is_playing());
    }

    #[test]
    fn choices_are_collected_with_their_flags_and_branches() {
        let (mut p, i18n, _) = setup();
        let loc = Locale::new(&i18n, Lang::Cs);
        p.set_condition_check(|c| !matches!(c, Condition::Flag(f) if f == "never"));
        p.start(&def(
            "c",
            vec![
                CutsceneStep::Choice {
                    options: vec![
                        opt("a", None, vec![]),
                        opt("hidden", None, vec![Condition::Flag("never".into())]),
                        opt("b", Some("picked_b"), vec![]),
                    ],
                },
                CutsceneStep::If {
                    conditions: vec![Condition::Flag("picked_b".into())],
                    then: vec![CutsceneStep::Apply {
                        effects: vec![Effect::GiveItem("old_coin".into())],
                    }],
                    otherwise: vec![],
                },
            ],
        ));
        p.step(&loc, 0.1, StepInput::default());
        // the hidden option is not offered: shown index 1 is data index 2
        let status = p.step(
            &loc,
            0.1,
            StepInput {
                choice: Some(1),
                ..Default::default()
            },
        );
        let CutsceneStatus::Finished { choices, .. } = status else {
            panic!("should finish, got {status:?}");
        };
        assert_eq!(choices.len(), 1);
        assert_eq!(choices[0].index, 2);
        assert_eq!(choices[0].set_flag.as_deref(), Some("picked_b"));
        let o = p.outcome().clone();
        assert_eq!(o.flags, vec!["picked_b".to_string()]);
        assert!(o.effects.contains(&Effect::GiveItem("old_coin".into())));
        assert!(o.effects.contains(&Effect::Money(-10)));
        assert_eq!(p.drain_effects().len(), o.effects.len());
    }

    #[test]
    fn hold_skip_applies_effects_and_notes_but_answers_no_choice() {
        let (mut p, i18n, _) = setup();
        let loc = Locale::new(&i18n, Lang::En);
        p.start(&def(
            "s",
            vec![
                narration("scene.opening.01"),
                CutsceneStep::Choice {
                    options: vec![opt("a", Some("x"), vec![])],
                },
                CutsceneStep::ShowNote {
                    note_id: "note_01".into(),
                },
                CutsceneStep::Apply {
                    effects: vec![Effect::GiveItem("cafe_keys".into())],
                },
                CutsceneStep::Music { track: None },
            ],
        ));
        p.step(&loc, 0.1, StepInput::default());
        let status = p.step(
            &loc,
            0.1,
            StepInput {
                skip: true,
                ..Default::default()
            },
        );
        assert!(
            matches!(status, CutsceneStatus::Finished { ref choices, .. } if choices.is_empty())
        );
        let o = p.outcome().clone();
        assert!(o.skipped);
        assert_eq!(o.notes, vec!["note_01".to_string()]);
        assert!(o.effects.contains(&Effect::GiveItem("cafe_keys".into())));
        assert!(o.flags.is_empty());
        assert!(
            p.drain_audio_requests()
                .contains(&AudioRequest::Music { track: None })
        );
    }

    #[test]
    fn overlays_never_blink_between_frames() {
        // the letter card used to oscillate 0 → 0.04 → 0 every frame (signum(0.0) == 1.0)
        let (mut p, i18n, content) = setup();
        let loc = Locale::new(&i18n, Lang::En);
        p.start(content.cutscene("opening").expect("opening exists"));
        let mut prev = p.letter_appear;
        let mut was_open = p.letter.is_some();
        let dt = 1.0 / 60.0;
        for frame in 0..60 * 120 {
            p.auto_advance = true;
            p.step(&loc, dt, StepInput::default());
            let open = p.letter.is_some();
            if open == was_open {
                // while the letter state holds, the card only moves one way
                if open {
                    assert!(
                        p.letter_appear >= prev,
                        "frame {frame}: card faded while open"
                    );
                } else {
                    assert!(
                        p.letter_appear <= prev,
                        "frame {frame}: card flashed while closed"
                    );
                }
            }
            if !open && !was_open {
                assert!(p.letter_appear == 0.0 || p.letter_appear < prev);
            }
            prev = p.letter_appear;
            was_open = open;
            if !p.is_playing() {
                break;
            }
        }
    }

    #[test]
    fn the_opening_plays_through_with_its_stage_directions() {
        let (mut p, i18n, content) = setup();
        let loc = Locale::new(&i18n, Lang::Uk);
        p.start(content.cutscene("opening").expect("opening exists"));
        assert!(p.scene.light_level() < 0.01, "the café starts dark");
        p.fast_forward(&loc, 400.0);
        assert!(!p.is_playing(), "auto-play reaches the end");
        assert!(p.scene.light_level() > 0.99, "the lamps are on at the end");
        assert!(
            p.outcome()
                .effects
                .contains(&Effect::GiveItem("cafe_keys".into()))
        );
        assert!(p.actors.iter().any(|a| a.id == "player"));
    }

    #[test]
    fn every_real_cutscene_finishes_on_auto_play() {
        let (mut p, i18n, content) = setup();
        let loc = Locale::new(&i18n, Lang::En);
        for def in &content.cutscenes.cutscenes {
            p.start(def);
            p.fast_forward(&loc, 900.0);
            assert!(!p.is_playing(), "cutscene {} did not finish", def.id);
        }
    }
}
