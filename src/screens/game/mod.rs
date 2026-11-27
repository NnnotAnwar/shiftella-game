//! A running game: the live café, HUD, panels, day planning, the morning summary, the pause
//! menu and story cutscenes.
//!
//! Split in two halves so the flow can be tested without a window:
//! * **logic** ([`GameScreen::advance`], [`GameScreen::handle_event`], commands, saving) —
//!   no `Ui`, no GPU;
//! * **frame** ([`GameScreen::frame`], [`GameScreen::hotkeys`]) — input, drawing, widgets.

mod app_tool;
mod attic_ui;
mod build;
mod build_cards;
mod build_content_ui;
mod build_overlay;
mod build_themes;
pub mod coach;
mod crew_ui;
mod day;
mod duties;
mod extras_ui;
mod guest_label;
pub mod hands;
mod hands_ui;
mod hands_view;
mod hud;
mod journal_ui;
mod moments_ui;
mod panels;
mod pause;
mod phone_app;
mod photo_mode;
mod purchases_ui;
pub mod scene;
mod scene_fx;
pub mod signature_ui;
mod stories_ui;
pub mod tutorial;
pub mod walk_home;
pub mod world_view;

/// Build mode's drag state (screenshot scenes set it up).
pub use build::{Drag as BuildDrag, THEME_TAB as BUILD_THEME_TAB};
pub use build_content_ui::scene_setup as build_content_scene;
pub use signature_ui::scene as signature_photo_scene;
pub use stories_ui::scene_setup as stories_scene;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use macroquad::prelude::*;

use crate::audio::{Audio, Sfx, Track};
use crate::camera::WorldCamera;
use crate::content::Content;
use crate::cutscene::{CutscenePlayer, CutsceneStatus};
use crate::save::{SaveFile, SaveMeta, SaveStore, SessionExtra, Slot};
use crate::settings::{Locale, Settings};
use crate::ui::{
    Align, Button, Glass, Icon, Toast, ToastAnchor, ToastId, ToastKind, Ui, draw_icon,
};
use scene::{Pick, WorldScene};
use sim::{AgentRef, Command, NightSummary, Phase, SimEvent, World};
use world_view::{self as wv, sim};

/// Real seconds per sim tick at ×1 by default (10 ticks = 1 game minute → a night ≈ 12 min).
/// The live value comes from `World::real_seconds_per_tick` (`handson.ron`).
pub const TICK_SECONDS: f32 = sim::REAL_SECONDS_PER_TICK_X1;

/// Everything a game frame needs from the app.
pub struct Ctx<'a> {
    pub content: &'a Rc<Content>,
    pub loc: Locale<'a>,
    pub settings: &'a mut Settings,
    pub audio: &'a mut Audio,
    pub store: &'a SaveStore,
}

/// What the game asks the app to do.
#[derive(Debug, Clone, PartialEq)]
pub enum GameExit {
    MainMenu,
    Quit,
    /// The story ended (or the sandbox café closed): play the credits.
    Credits {
        ending: Option<String>,
        variant: Option<String>,
    },
    /// Load this slot (from the pause menu).
    Load(crate::save::Slot),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Crew,
    /// The duties board (H; P is an alias): who helps with what tonight.
    Duties,
    App,
    Build,
    Log,
    /// "How to play" reference (F1).
    Help,
    /// The polaroid journal (J).
    Journal,
    /// The weekly payroll basis (podklady pro mzdy) as a decision screen.
    Payroll,
    /// Grandma's attic and the back room (package C; the day card's "Attic").
    Attic,
}

/// A line in the top ticker / log.
#[derive(Debug, Clone)]
pub struct Ticker {
    pub icon: Icon,
    pub time: String,
    pub text: String,
}

/// Something for the UI layer to show (produced by the logic half).
#[derive(Debug, Clone)]
pub enum Notice {
    Toast {
        kind: ToastKind,
        icon: Icon,
        title: String,
        body: Option<String>,
    },
    Choice {
        event: String,
        choices: Vec<String>,
    },
    ChoiceResolved {
        event: String,
        choice: String,
    },
    /// A toast from an i18n key (args: `@key` = translate, `money:N` = format money).
    Key {
        kind: ToastKind,
        icon: Icon,
        key: String,
        args: Vec<(String, String)>,
    },
    /// The same with a title key (a moment's name over its result line).
    KeyTitled {
        kind: ToastKind,
        icon: Icon,
        title: String,
        key: String,
        args: Vec<(String, String)>,
    },
}

/// Seconds a toast stays at night: short, so they never pile over the café.
fn night_duration(kind: ToastKind) -> f32 {
    match kind {
        ToastKind::Info => 4.5,
        ToastKind::Success => 4.0,
        ToastKind::Event => 6.0,
        ToastKind::Warning => 7.0,
    }
}

/// Upper-cases the first letter (any script: Latin, Cyrillic…), keeps the rest.
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// A long line becomes a short title + the rest as the body (titles are one line).
fn split_title(ui: &Ui, text: &str) -> (String, Option<String>) {
    let st = ui.theme.strong();
    let room = 250.0;
    if ui.fonts.measure(text, &st) <= room {
        return (text.to_string(), None);
    }
    for sep in [". ", " — ", ": ", "! ", "? "] {
        if let Some(i) = text.find(sep) {
            let head = text[..i + sep.trim_end().len()].trim().to_string();
            // "… waiting — say hello" → the body starts its own sentence: "Say hello"
            let rest = capitalize_first(text[i + sep.len()..].trim());
            if !rest.is_empty() && ui.fonts.measure(&head, &st) <= room {
                return (
                    head.trim_end_matches(['—', ':']).trim().to_string(),
                    Some(rest),
                );
            }
        }
    }
    // no sentence break that fits: the first words as the title, the whole line below
    let mut head = String::new();
    for w in text.split_whitespace() {
        let next = if head.is_empty() {
            w.to_string()
        } else {
            format!("{head} {w}")
        };
        if ui.fonts.measure(&format!("{next}…"), &st) > room {
            break;
        }
        head = next;
    }
    (format!("{head}…"), Some(text.to_string()))
}

/// Bubble texts may be an i18n key (`@key:<key>`) when the logic half had no locale.
pub fn resolve_text(loc: &Locale, text: &str) -> String {
    match text.strip_prefix("@key:") {
        Some("") => String::new(),
        Some(k) => loc.tr_owned(k),
        None => text.to_string(),
    }
}

/// Translates argument values: `@key` → its text, `money:N` → "N Kč".
pub fn resolve_args(loc: &Locale, args: &[(String, String)]) -> Vec<(String, String)> {
    args.iter()
        .map(|(k, v)| {
            let v = if let Some(key) = v.strip_prefix('@') {
                loc.tr_owned(key)
            } else if let Some(n) = v.strip_prefix("money:") {
                crate::i18n::format_money(loc.lang, n.parse().unwrap_or(0))
            } else {
                v.clone()
            };
            (k.clone(), v)
        })
        .collect()
}

/// Speech bubble over a guest.
#[derive(Debug, Clone)]
pub struct Bubble {
    pub agent: AgentRef,
    pub text: String,
    pub talk: Option<String>,
    pub pinned: bool,
    pub until: f32,
}

pub struct GameScreen {
    pub world: World,
    pub scene: WorldScene,
    pub cam: WorldCamera,
    cam_ready: bool,
    /// 0 = paused, 1..=3.
    pub speed: u8,
    speed_before_pause: u8,
    acc: f32,
    pub time: f32,
    pub panel: Option<Panel>,
    pub pause: Option<pause::PauseMenu>,
    pub summary: Option<NightSummary>,
    /// Summary waiting for a night-end cutscene to finish.
    summary_pending: Option<NightSummary>,
    pub selected: Option<AgentRef>,
    pub hovered: Option<AgentRef>,
    pub bubble: Option<Bubble>,
    pub ticker: Vec<Ticker>,
    pub notices: Vec<Notice>,
    choice_toasts: Vec<(ToastId, String, Vec<String>)>,
    pub player: CutscenePlayer,
    /// Shared world snapshot the cutscene condition callback reads.
    snapshot: Rc<RefCell<Option<World>>>,
    /// The cutscene now on screen is the pre-Day-1 opening (not asked for by the sim).
    intro: bool,
    pub extra: SessionExtra,
    ending: Option<String>,
    exit: Option<GameExit>,
    pub ui_state: UiState,
    /// Something changed that the autosave should catch at the next safe point.
    autosave_due: bool,
    /// Screenshot automation: ignore keyboard / mouse world input.
    pub automation: bool,
    /// Story moments waiting to be read: while one is up the night holds (owner: "how am I
    /// supposed to follow the story when it's a clicker?").
    pub reading: std::collections::VecDeque<ReadCard>,
    /// Events with text that arrived without a locale (handled next frame).
    deferred: Vec<SimEvent>,
    /// Real seconds the 3:33 clock stop still holds the sim.
    clock_stop: f32,
    /// Last game minute a footstep / coffee ambience sound was considered.
    last_ambience_minute: u32,
    /// Timings of the last frame (debug overlay).
    pub times: FrameTimes,
    /// Screen rects of HUD pieces this frame, by name (tutorial arrows point at them).
    pub anchors: Vec<(&'static str, Rect)>,
    /// Karolina's hands-on loop: order card, mini-game, rail, coins, … (see `hands`).
    pub hands: hands::Hands,
    /// Settings: relaxed mini-games (copied from the settings each frame).
    pub relaxed: bool,
    /// Latte art drawn in the last mini-game per guest (served cups show it).
    pub latte_for: HashMap<u32, String>,
    /// Karolina's first-night coach marks.
    pub coach: coach::CoachState,
    /// A camera hint from the sim (a moment, a bot button): ease towards this world point.
    pub cam_target: Option<Vec2>,
    /// Polaroid photos (rendered from the journal's scene snapshots) and the journal's page.
    pub photos: journal_ui::Photos,
    /// Plain toasts on screen (newest last): their ids, a signature and when they came.
    recent_toasts: std::collections::VecDeque<(ToastId, String, f32)>,
    /// The walk home / the back step on screen (design/v0.2.md F).
    pub walk: walk_home::WalkUi,
    /// The café's own drink: the compose card (design/v0.2.md I.5).
    pub sig: signature_ui::SigUi,
    /// Photo mode: the viewfinder and the developing print (design/v0.2.md I.6).
    pub photo: photo_mode::PhotoMode,
    /// Little stories of strangers, the storm card and its 03:00 talk (design/v0.2.md H).
    pub stories: stories_ui::StoriesUi,
}

/// CPU time of the last frame's parts (the F3 debug overlay).
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameTimes {
    pub sim_ms: f32,
    pub world_ms: f32,
    pub ui_ms: f32,
    /// Sim ticks run this frame.
    pub ticks: u32,
}

/// Panel-local state that survives frames.
#[derive(Debug, Clone, Default)]
pub struct UiState {
    pub crew_selected: Option<u32>,
    pub crew_sort: usize,
    pub app_tab: usize,
    pub build: build::BuildState,
    pub handover_text: String,
    pub log_filter: usize,
    /// Whom the board's thank-you note is for (index into the crew).
    pub thank_pick: usize,
    /// The rozpis puzzle board (paint, keyboard cursor, flash).
    pub rozpis: crate::ui::handson::RozpisState,
    /// Staff granted a night off on this week's payroll screen.
    pub toil_granted: Vec<u32>,
}

impl GameScreen {
    fn with_world(content: &Rc<Content>, world: World, extra: SessionExtra) -> GameScreen {
        let mut player = CutscenePlayer::new(content);
        let snapshot: Rc<RefCell<Option<World>>> = Rc::new(RefCell::new(None));
        {
            let snap = snapshot.clone();
            let c = content.clone();
            player.set_condition_check(move |cond| {
                snap.borrow()
                    .as_ref()
                    .is_some_and(|w| w.eval_conditions(&c, std::slice::from_ref(cond)))
            });
        }
        GameScreen {
            world,
            scene: WorldScene::new(),
            cam: WorldCamera::at(vec2(15.0, 8.0) * crate::map::TILE, 1.0),
            cam_ready: false,
            speed: 1,
            speed_before_pause: 1,
            acc: 0.0,
            time: 0.0,
            panel: None,
            pause: None,
            summary: None,
            summary_pending: None,
            selected: None,
            hovered: None,
            bubble: None,
            ticker: Vec::new(),
            notices: Vec::new(),
            choice_toasts: Vec::new(),
            player,
            snapshot,
            intro: false,
            extra,
            ending: None,
            exit: None,
            ui_state: UiState::default(),
            autosave_due: false,
            last_ambience_minute: u32::MAX,
            deferred: Vec::new(),
            clock_stop: 0.0,
            times: FrameTimes::default(),
            anchors: Vec::new(),
            automation: std::env::var("SHIFTELLA_SHOT").is_ok(),
            reading: Default::default(),
            hands: hands::Hands::default(),
            relaxed: false,
            latte_for: HashMap::new(),
            coach: coach::CoachState::default(),
            cam_target: None,
            photos: journal_ui::Photos::default(),
            recent_toasts: Default::default(),
            walk: Default::default(),
            sig: Default::default(),
            photo: Default::default(),
            stories: Default::default(),
        }
    }

    /// A new story. Plays the opening before Day 1 unless `skip_opening`.
    pub fn new_story(content: &Rc<Content>, difficulty: &str, skip_opening: bool) -> GameScreen {
        let world = World::new_story(content, wv::difficulty_from_id(difficulty));
        let mut g = GameScreen::with_world(content, world, SessionExtra::default());
        g.deferred = g.world.drain_events();
        if skip_opening {
            g.extra.opening_seen = Some(Vec::new());
        } else if let Some(def) = content.cutscene("opening") {
            g.stage_player();
            g.player.start(def);
            g.intro = true;
        }
        g
    }

    pub fn new_sandbox(content: &Rc<Content>, setup: sim::SandboxSetup) -> GameScreen {
        let world = World::new_sandbox(content, setup);
        let mut g = GameScreen::with_world(content, world, SessionExtra::default());
        g.deferred = g.world.drain_events();
        g
    }

    pub fn from_save(content: &Rc<Content>, file: &SaveFile) -> Result<GameScreen, String> {
        let mut world = World::load_from_str(&file.world)?;
        world.rebuild_caches(content);
        let mut g = GameScreen::with_world(content, world, file.extra.clone());
        if let Some((x, y, z)) = file.extra.camera {
            g.cam.center = vec2(x, y) * crate::map::TILE;
            g.cam.zoom = z;
            g.cam_ready = true;
        }
        // a cutscene that was on screen when saving starts again
        if let Some(id) = g.world.playing_cutscene().map(str::to_string) {
            g.start_cutscene(content, &id);
        }
        if g.world.ending().is_some() {
            g.ending = g.world.ending().map(str::to_string);
        }
        Ok(g)
    }

    // ───────────────────────── logic (no window needed) ─────────────────────────

    pub fn exit(&mut self) -> Option<GameExit> {
        self.exit.take()
    }

    pub fn cutscene_playing(&self) -> bool {
        self.player.is_playing()
    }

    /// Cutscene text is on screen: the app ducks the music under it.
    pub fn dialogue_on_screen(&self) -> bool {
        self.player.text_on_screen()
    }

    /// Clock frozen for the player (overlays, cutscenes, pause).
    pub fn frozen(&self) -> bool {
        self.speed == 0
            || self.pause.is_some()
            || self.cutscene_playing()
            || self.summary.is_some()
            || self.world.phase() == Phase::Day
            || self.walk.is_open()
            || self.photo.is_open()
            || self.stories.talk_open()
            || !self.reading.is_empty()
    }

    /// The story card at the bottom: who says it, the text, "Continue" (click / Enter / Space).
    fn draw_reading(&mut self, ui: &mut Ui, loc: &Locale) {
        let Some(card) = self.reading.front().cloned() else {
            return;
        };
        let t = ui.theme.clone();
        let s = ui.screen();
        let w = (s.w - 48.0).min(560.0);
        let body = t.body().color(t.text);
        let lines = ui.fonts.wrap(&card.text, w - 40.0, &body).len().max(1) as f32;
        let h = 44.0 + lines * body.line_h() + 44.0;
        let r = Rect::new((s.w - w) / 2.0, s.h - 64.0 - h - 8.0, w, h);
        ui.glass(r, &Glass::panel());
        draw_icon(card.icon, vec2(r.x + 22.0, r.y + 22.0), 14.0, t.amber);
        ui.fonts.draw(
            &card.title,
            r.x + 36.0,
            r.y + 27.0,
            &t.strong().color(t.text),
        );
        ui.fonts.draw_wrapped(
            &card.text,
            Rect::new(
                r.x + 20.0,
                r.y + 40.0,
                w - 40.0,
                lines * body.line_h() + 4.0,
            ),
            &body,
            Align::Left,
            None,
        );
        let more = self.reading.len() > 1;
        let label = loc.tr(if more {
            "ui.read.next"
        } else {
            "ui.read.continue"
        });
        let b = Button::new(label).primary().shortcut("Enter");
        let bw = ui.button_width(&b);
        let br = Rect::new(r.right() - bw - 14.0, r.bottom() - 38.0, bw, 28.0);
        let clicked = ui.button("read_continue", br, &b).clicked;
        if clicked || ui.enter() || ui.key(KeyCode::Space) {
            self.reading.pop_front();
        }
    }

    /// Queues a story moment to read (the night holds until it's closed). Off for automation,
    /// the autoplayer and unit tests, which never stop to read.
    fn read_card(&mut self, icon: Icon, title: String, text: String) {
        if cfg!(test) || self.automation || self.hands.autoplay.is_some() || text.is_empty() {
            return;
        }
        if self.reading.len() < 6 {
            self.reading.push_back(ReadCard { icon, title, text });
        }
    }

    pub fn set_speed(&mut self, s: u8) {
        self.speed = s.min(3);
        if s > 0 {
            self.speed_before_pause = s;
        }
    }

    pub fn toggle_pause(&mut self) {
        if self.speed == 0 {
            self.speed = self.speed_before_pause.max(1);
        } else {
            self.speed_before_pause = self.speed;
            self.speed = 0;
        }
    }

    /// Runs the sim for `dt` real seconds at the current speed; handles the events.
    pub fn advance(&mut self, content: &Rc<Content>, dt: f32, loc: Option<&Locale>) {
        if !self.frozen() {
            let (rate, tick) = self.sim_rate(content);
            self.acc += dt.min(0.25) * rate;
            let mut guard = 0;
            while self.acc >= tick && guard < 64 {
                self.acc -= tick;
                self.world.tick(content);
                self.scene.sim_steps += 1;
                guard += 1;
                // stop stepping as soon as something needs the player
                if self.world.playing_cutscene().is_some() {
                    self.acc = 0.0;
                    break;
                }
            }
        }
        self.drain_and_handle(content, loc);
        if self.world.phase() == Phase::Day && !self.cutscene_playing() {
            self.acc = 0.0;
        }
    }

    pub fn drain_and_handle(&mut self, content: &Rc<Content>, loc: Option<&Locale>) {
        let mut events = self.world.drain_events();
        if let Some(l) = loc {
            let mut all = std::mem::take(&mut self.deferred);
            all.append(&mut events);
            for e in all {
                self.handle_event(content, e, Some(l));
            }
        } else {
            // flow events now; text waits for the next frame's locale
            for e in events {
                if is_flow_event(&e) {
                    self.handle_event(content, e, None);
                } else {
                    self.deferred.push(e);
                }
            }
        }
        // a queued summary shows once no cutscene is on screen
        if !self.cutscene_playing()
            && self.world.playing_cutscene().is_none()
            && let Some(s) = self.summary_pending.take()
        {
            self.summary = Some(s);
        }
        if self.ending.is_some()
            && !self.cutscene_playing()
            && self.world.playing_cutscene().is_none()
            && self.summary.is_none()
            && self.exit.is_none()
        {
            self.extra.ending_played = true;
            self.exit = Some(GameExit::Credits {
                ending: self.ending.clone(),
                variant: self.world.ending_variant().map(str::to_string),
            });
        }
    }

    /// Avatars show faces: staff, candidates, Karolina and the bot, by their display names.
    fn register_faces(&mut self, ui: &mut Ui, loc: &Locale) {
        let staff_look = |s: &sim::Staff| sim::LookRef {
            seed: s.look_seed,
            staff: !s.owner,
            preset: s.owner.then(|| "player".to_string()),
            source: s.archetype.clone(),
        };
        let people: Vec<(String, sim::LookRef)> = self
            .world
            .staff()
            .iter()
            .chain(self.world.candidates().iter().map(|c| &c.staff))
            .map(|s| (wv::staff_name(loc, s), staff_look(s)))
            .collect();
        for (name, l) in people {
            let look = self.scene.look(&l);
            ui.register_face(&name, look);
        }
        let player = crate::render::PersonLook::preset("player");
        ui.register_face(loc.tr("cast.player.name"), player);
        ui.register_face(
            "@shiftella_bot",
            crate::render::PersonLook::preset("shiftella_bot"),
        );
    }

    /// Hands the live café to the cutscene player: walkability for actor paths (around the
    /// player's own furniture) and where tonight's people already are.
    fn stage_player(&mut self) {
        use crate::map::{Map, Tile};
        let g = self.world.grid();
        let map = Map::from_fn(g.width.max(0) as usize, g.height.max(0) as usize, |x, y| {
            let (x, y) = (x as i32, y as i32);
            match g.tile(x, y).map(|t| t.kind) {
                Some(sim::TileKind::Wall) => Tile::Wall,
                Some(sim::TileKind::Window) => Tile::Window,
                Some(sim::TileKind::Door) => Tile::Door,
                _ if g.walkable(x, y) => Tile::Floor,
                _ => Tile::Table,
            }
        });
        self.player.set_nav_map(Some(map));
        let live: Vec<(String, Vec2)> = self
            .world
            .staff_views()
            .into_iter()
            .chain(self.world.guest_views())
            .map(|v| (v.look.source.clone(), vec2(v.feet.0, v.feet.1)))
            .collect();
        self.player.set_live_cast(live);
    }

    fn start_cutscene(&mut self, content: &Rc<Content>, id: &str) {
        // the opening was shown before Day 1: hand its choices straight to the sim
        if id == "opening"
            && let Some(choices) = self.extra.opening_seen.clone()
        {
            let _ = self.world.apply(
                content,
                Command::CutsceneFinished {
                    id: id.to_string(),
                    choices,
                },
            );
            return;
        }
        let Some(def) = content.cutscene(id) else {
            let _ = self.world.apply(
                content,
                Command::CutsceneFinished {
                    id: id.to_string(),
                    choices: Vec::new(),
                },
            );
            return;
        };
        *self.snapshot.borrow_mut() = Some(self.world.clone());
        self.stage_player();
        self.player.start(def);
        self.intro = false;
        self.bubble = None;
    }

    /// The player finished a scene (or a test fast-forwarded it).
    pub fn cutscene_finished(&mut self, content: &Rc<Content>, id: String, choices: Vec<usize>) {
        if self.intro {
            self.intro = false;
            self.extra.opening_seen = Some(choices);
            self.autosave_due = true;
            return;
        }
        // where the scene left its regulars (still on stage, not gone through a door)
        let handoff: Vec<(String, (i32, i32))> = self
            .player
            .actors
            .iter()
            .filter(|a| !a.gone && !a.exit_on_arrive && content.regular(&a.id).is_some())
            .map(|a| {
                let t = crate::map::TILE;
                (
                    a.id.clone(),
                    ((a.pos.x / t).floor() as i32, (a.pos.y / t).floor() as i32),
                )
            })
            .collect();
        let _ = self
            .world
            .apply(content, Command::CutsceneFinished { id, choices });
        for (regular, tile) in handoff {
            let _ = self
                .world
                .apply(content, Command::SceneHandoff { regular, tile });
        }
        self.drain_and_handle(content, None);
        // the next queued scene may already be waiting
        if !self.player.is_playing()
            && let Some(next) = self.world.playing_cutscene().map(str::to_string)
        {
            self.start_cutscene(content, &next);
        }
    }

    /// Skips whatever scene is on screen (tests, `SHIFTELLA_SCENE`).
    pub fn finish_cutscene_now(&mut self, content: &Rc<Content>, loc: &Locale) {
        let mut guard = 0;
        while self.player.is_playing() && guard < 8 {
            // the autoplayer answers like an attentive player (the balance bot's choices)
            let picks = sim::bot::attentive_choices(&self.world, content, self.player.current_id());
            self.player.fast_forward_with(loc, 600.0, &picks);
            let id = self.player.current_id().to_string();
            let choices = self
                .player
                .outcome()
                .choices
                .iter()
                .map(|c| c.index)
                .collect();
            // fast_forward leaves `playing` false; report it like the UI would
            if !self.player.is_playing() {
                self.cutscene_finished(content, id, choices);
            }
            guard += 1;
        }
    }

    fn push_ticker(&mut self, icon: Icon, text: String) {
        let time = if self.world.phase() == Phase::Night {
            self.world.clock_label()
        } else {
            String::new()
        };
        self.ticker.push(Ticker { icon, time, text });
        if self.ticker.len() > 200 {
            self.ticker.remove(0);
        }
    }

    fn toast(&mut self, kind: ToastKind, icon: Icon, title: String, body: Option<String>) {
        self.notices.push(Notice::Toast {
            kind,
            icon,
            title,
            body,
        });
    }

    pub fn handle_event(&mut self, content: &Rc<Content>, e: SimEvent, loc: Option<&Locale>) {
        // text needs a locale; tests may run without one
        let tr = |key: &str, args: &[(String, String)]| -> String {
            loc.map(|l| wv::tr_args(l, key, args))
                .unwrap_or_else(|| key.to_string())
        };
        let staff_name = |w: &World, id: u32| -> String {
            match (w.staff_by_id(id), loc) {
                (Some(s), Some(l)) => wv::staff_name(l, s),
                (Some(s), None) => s.name.clone(),
                _ => String::from("?"),
            }
        };
        self.hands_event(content, &e);
        self.extras_event(content, &e);
        self.build_content_event(content, &e, loc);
        self.names_event(&e, loc);
        self.signature_event(&e, loc);
        self.stories_event(&e, loc);
        match e {
            SimEvent::DayStarted { .. } => {
                self.autosave_due = true;
            }
            SimEvent::ClockStop { seconds } => {
                self.clock_stop = self.clock_stop.max(seconds);
            }
            SimEvent::NightStarted { night } => {
                let t = tr(
                    "ui.ticker.night_started",
                    &[("night".into(), night.to_string())],
                );
                self.push_ticker(Icon::Moon, t);
                // 21:30: the doors stay closed for a quiet half hour
                if self.world.in_prep() {
                    let title = tr("prep.toast.start.title", &[]);
                    let body = tr("prep.toast.start.body", &[]);
                    self.toast(ToastKind::Info, Icon::Door, title, Some(body));
                }
            }
            SimEvent::DoorsOpened { .. } => {
                self.scene.doors_opened_at = Some(self.time);
                let title = tr("prep.toast.doors_open.title", &[]);
                let body = tr("prep.toast.doors_open.body", &[]);
                self.push_ticker(Icon::Door, title.clone());
                self.toast(ToastKind::Success, Icon::Door, title, Some(body));
            }
            SimEvent::NightEnded { summary } => {
                self.summary_pending = Some(*summary);
                self.panel = None;
                self.selected = None;
                self.bubble = None;
            }
            SimEvent::PlayCutscene(id) => {
                if !self.player.is_playing() {
                    self.start_cutscene(content, &id);
                }
            }
            SimEvent::EndingReached(id) => {
                self.ending = Some(id);
            }
            SimEvent::CafeClosed => {
                if self.world.mode() == sim::Mode::Sandbox {
                    self.ending = Some("closed".into());
                }
            }
            SimEvent::GuestArrived {
                regular: Some(r), ..
            } => {
                let name = tr(&format!("regular.{r}.name"), &[]);
                let t = tr("ui.ticker.regular_arrived", &[("name".into(), name)]);
                self.push_ticker(Icon::Heart, t);
            }
            SimEvent::TrainPassing { train, station } => {
                self.scene.start_train();
                let name = tr(
                    &format!("train.{}.name", train.to_lowercase().replace(' ', "_")),
                    &[],
                );
                let st = content
                    .trains
                    .stations
                    .iter()
                    .find(|s| s.id == station)
                    .map(|s| s.name.clone())
                    .unwrap_or(station);
                let t = tr(
                    "ui.ticker.train",
                    &[("train".into(), name), ("station".into(), st)],
                );
                self.push_ticker(Icon::Train, t);
            }
            SimEvent::Blackout { on } => {
                let key = if on {
                    "ui.ticker.blackout"
                } else {
                    "ui.ticker.power_back"
                };
                let t = tr(key, &[]);
                self.push_ticker(Icon::Warning, t.clone());
                if on {
                    self.toast(ToastKind::Warning, Icon::Warning, t, None);
                }
            }
            SimEvent::EventStarted { id } => {
                let title = tr(&format!("event.{id}.title"), &[]);
                let body = tr(&format!("event.{id}.text"), &[]);
                self.push_ticker(Icon::Sparkle, title.clone());
                let has_choice = content.event(&id).is_some_and(|d| !d.choices.is_empty());
                if !has_choice {
                    let kind = match content.event(&id).map(|d| d.kind) {
                        Some(crate::content::EventKind::Bad) => ToastKind::Warning,
                        Some(crate::content::EventKind::Good) => ToastKind::Success,
                        _ => ToastKind::Event,
                    };
                    self.toast(kind, Icon::Sparkle, title, Some(body));
                }
            }
            SimEvent::ChoiceNeeded { id, choices, .. } => {
                self.notices.push(Notice::Choice { event: id, choices });
                if loc.is_some() && self.speed == 0 {
                    // choices work while paused, nothing else to do
                }
            }
            SimEvent::ChoiceResolved { id, choice } => {
                self.notices
                    .push(Notice::ChoiceResolved { event: id, choice });
            }
            SimEvent::Toast { key } => {
                let t = tr(&key, &[]);
                self.push_ticker(Icon::Info, t.clone());
                self.toast(ToastKind::Info, Icon::Info, t, None);
            }
            SimEvent::Hint { key } => {
                let t = tr(&key, &[]);
                if key.starts_with("story.") {
                    // a story hint: read it with the night held
                    self.push_ticker(Icon::Note, t.clone());
                    let title = tr("ui.read.story", &[]);
                    self.read_card(Icon::Note, title, t);
                } else {
                    self.toast(ToastKind::Info, Icon::Info, t, None);
                }
            }
            SimEvent::BotMessage { key, args, .. } => {
                let t = tr(&key, &args);
                self.push_ticker(Icon::Phone, t.clone());
                let title = tr("cast.bot.name", &[]);
                self.toast(ToastKind::Info, Icon::Phone, title, Some(t));
            }
            SimEvent::NoteFound(id) => {
                let t = tr(
                    "ui.ticker.note_found",
                    &[("n".into(), self.world.notes_found().len().to_string())],
                );
                self.push_ticker(Icon::Note, t.clone());
                // grandma's note itself, to read now (it stays in the book)
                let text = tr(&format!("note.{id}.text"), &[]);
                if text.starts_with("note.") {
                    self.toast(ToastKind::Event, Icon::Note, t, None);
                } else {
                    self.read_card(Icon::Note, t, text);
                }
            }
            SimEvent::PaymentMade { amount, .. } => {
                let money = loc
                    .map(|l| crate::i18n::format_money(l.lang, amount))
                    .unwrap_or_default();
                let t = tr("ui.ticker.payment_made", &[("money".into(), money)]);
                self.push_ticker(Icon::Money, t.clone());
                self.toast(ToastKind::Success, Icon::Money, t, None);
            }
            SimEvent::PaymentMissed { amount } => {
                let money = loc
                    .map(|l| crate::i18n::format_money(l.lang, amount))
                    .unwrap_or_default();
                let t = tr("ui.ticker.payment_missed", &[("money".into(), money)]);
                self.push_ticker(Icon::Warning, t.clone());
                self.toast(ToastKind::Warning, Icon::Money, t, None);
            }
            SimEvent::StaffLate { staff, minutes } => {
                let n = staff_name(&self.world, staff);
                let t = tr(
                    "ui.ticker.staff_late",
                    &[("name".into(), n), ("min".into(), minutes.to_string())],
                );
                self.push_ticker(Icon::Clock, t);
            }
            SimEvent::StaffSick { staff } => {
                let n = staff_name(&self.world, staff);
                let t = tr("ui.ticker.staff_sick", &[("name".into(), n)]);
                self.push_ticker(Icon::Warning, t.clone());
                self.toast(ToastKind::Warning, Icon::Crew, t, None);
            }
            SimEvent::StaffBreakEnded { staff } => {
                let n = staff_name(&self.world, staff);
                let t = tr("ui.ticker.break_ended", &[("name".into(), n)]);
                self.push_ticker(Icon::Cup, t);
            }
            SimEvent::StaffWalkedOut { staff } => {
                let n = staff_name(&self.world, staff);
                let t = tr("ui.ticker.staff_walked_out", &[("name".into(), n)]);
                self.push_ticker(Icon::Warning, t.clone());
                self.toast(ToastKind::Warning, Icon::Crew, t, None);
            }
            SimEvent::WeatherChanged { weather } => {
                let w = tr(wv::weather_key(weather), &[]);
                let t = tr("ui.ticker.weather", &[("weather".into(), w)]);
                self.push_ticker(Icon::Rain, t);
            }
            SimEvent::MenuUnlocked(id) => {
                let name = tr(&format!("menu.{id}.name"), &[]);
                let t = tr("ui.ticker.menu_unlocked", &[("item".into(), name)]);
                self.push_ticker(Icon::Coffee, t.clone());
                self.toast(ToastKind::Success, Icon::Coffee, t, None);
            }
            SimEvent::FurnitureUnlocked(id) => {
                let name = tr(&format!("furniture.{id}.name"), &[]);
                let t = tr("ui.ticker.furniture_unlocked", &[("item".into(), name)]);
                self.push_ticker(Icon::Build, t.clone());
                self.toast(ToastKind::Success, Icon::Build, t, None);
            }
            SimEvent::Speech {
                agent: Some(AgentRef::Guest(g)),
                key,
                speaker,
            } => {
                let text = tr(&key, &[]);
                // a regular's story line (not small talk): held to be read
                if key.starts_with("regular.") || key.starts_with("story.line.") {
                    let who = tr(&format!("regular.{speaker}.name"), &[]);
                    if !who.starts_with("regular.") {
                        self.read_card(Icon::Heart, who, text.clone());
                    }
                }
                if self.bubble.as_ref().is_none_or(|b| !b.pinned) {
                    self.bubble = Some(Bubble {
                        agent: AgentRef::Guest(g),
                        text,
                        talk: None,
                        pinned: false,
                        until: self.time + 6.0,
                    });
                }
            }
            _ => {}
        }
    }

    /// Audio reactions for an event (kept apart from the logic so tests stay silent).
    fn event_audio(audio: &mut Audio, e: &SimEvent) {
        match e {
            SimEvent::GuestArrived { .. } => audio.play_sfx_vol(Sfx::DoorBell, 0.7),
            SimEvent::Payment { .. } => audio.play_sfx_vol(Sfx::CashRegister, 0.7),
            SimEvent::OrderReady { .. } => audio.play_sfx_vol(Sfx::CoffeeMachine, 0.6),
            SimEvent::OrderServed { .. } => audio.play_sfx_vol(Sfx::CupSaucer, 0.7),
            SimEvent::DishBroken { .. } => audio.play_sfx_vol(Sfx::CupSaucer, 1.0),
            SimEvent::TrainPassing { .. } => audio.play_sfx(Sfx::TrainPass),
            SimEvent::DoorsOpened { .. } => audio.play_sfx_vol(Sfx::DoorBell, 0.9),
            SimEvent::Blackout { on: true } => audio.play_sfx(Sfx::PowerOutage),
            SimEvent::ChoiceNeeded { .. } | SimEvent::EventStarted { .. } => {
                audio.play_sfx_vol(Sfx::EventAlert, 0.8)
            }
            // hands-on (design §14): existing sounds only
            SimEvent::PlayerServed { .. } => audio.play_sfx_vol(Sfx::CupSaucer, 0.7),
            SimEvent::PlayerGreeted { .. } => audio.play_sfx_vol(Sfx::UiClick, 0.3),
            SimEvent::RecordChanged { .. } => audio.play_sfx_vol(Sfx::UiClick, 0.3),
            SimEvent::RecordUnlocked { .. } | SimEvent::PractisedRecipe { .. } => {
                audio.play_sfx_vol(Sfx::UiClick, 0.5)
            }
            SimEvent::PlayerOrderTaken { .. } => audio.play_sfx_vol(Sfx::UiClick, 0.35),
            // package C: a corner forms, someone thanks you for what you built
            SimEvent::CornerFormed { .. } => audio.play_sfx_vol(Sfx::UiClick, 0.5),
            SimEvent::BuildWishFulfilled { .. } => audio.play_sfx_vol(Sfx::CupSaucer, 0.6),
            // package K: a blanket over a sleeping guest's shoulders
            SimEvent::MomentResolved { option, .. } if option == "blanket" => {
                audio.play_sfx_vol(Sfx::Blanket, 0.8)
            }
            // 0.2 H: stories and the storm
            SimEvent::StoryNoticed { .. } => audio.play_sfx_vol(Sfx::UiClick, 0.25),
            SimEvent::StoryAnswered { .. } => audio.play_sfx_vol(Sfx::CupSaucer, 0.55),
            SimEvent::StormCancelled { .. } => audio.play_sfx_vol(Sfx::EventAlert, 0.6),
            SimEvent::StormCared { act, .. } => match act {
                sim::StormAct::Blankets => audio.play_sfx_vol(Sfx::Blanket, 0.8),
                sim::StormAct::Soup => audio.play_sfx_vol(Sfx::PotLid, 0.7),
            },
            _ => {}
        }
    }

    /// Sends a player command; returns the error text for a toast.
    pub fn command(&mut self, content: &Rc<Content>, cmd: Command) -> Result<(), String> {
        let r = self.world.apply(content, cmd).map_err(|e| format!("{e:?}"));
        self.drain_and_handle(content, None);
        r
    }

    /// Day → Night.
    pub fn start_night(&mut self, content: &Rc<Content>) {
        if self.world.phase() != Phase::Day || self.cutscene_playing() {
            return;
        }
        self.summary = None;
        self.panel = None;
        let _ = self.world.apply(content, Command::StartNight);
        if self.speed == 0 {
            self.speed = self.speed_before_pause.max(1);
        }
        self.drain_and_handle(content, None);
    }

    pub fn save_file(&self, content: &Content) -> Result<SaveFile, String> {
        let _ = content;
        let w = &self.world;
        let clock = if w.phase() == Phase::Night {
            w.clock_label()
        } else {
            "day".into()
        };
        let meta = SaveMeta {
            mode: wv::mode_id(w).into(),
            night: w.night(),
            clock,
            cafe: w.cafe_name().to_string(),
            difficulty: w.difficulty().id().into(),
            saved_at: crate::save::now(),
            finished: w.ending().is_some(),
            money: w.money(),
        };
        let mut extra = self.extra.clone();
        extra.camera = Some((
            self.cam.center.x / crate::map::TILE,
            self.cam.center.y / crate::map::TILE,
            self.cam.zoom,
        ));
        Ok(SaveFile::new(meta, extra, w.save_to_string()?))
    }

    pub fn save(&self, content: &Content, store: &SaveStore, slot: Slot) -> Result<(), String> {
        let r = self
            .save_file(content)
            .and_then(|f| store.write(slot, &f).map(|_| ()));
        if let Err(e) = &r {
            crate::crashlog::warn(&format!("can't save {slot:?}: {e}"));
        }
        r
    }

    /// Autosave at a safe point (start of a day, not inside a cutscene).
    pub fn autosave_if_due(&mut self, content: &Content, store: &SaveStore) {
        if self.autosave_due
            && !self.cutscene_playing()
            && self.world.playing_cutscene().is_none()
            && std::env::var("SHIFTELLA_NO_AUTOSAVE").is_err()
        {
            self.autosave_due = false;
            if let Err(e) = self.save(content, store, Slot::Auto) {
                eprintln!("warning: autosave failed: {e}");
            }
        }
    }

    /// Music from the sim (house calm / rush / quiet hour, grandma's records, the day
    /// theme; silence in a blackout), rain by weather.
    fn update_audio(&mut self, content: &Content, audio: &mut Audio) {
        if self.cutscene_playing() {
            return;
        }
        let night = self.world.phase() == Phase::Night;
        // the old radio (a moment) plays over the house music for a while
        let radio = self
            .world
            .radio()
            .filter(|_| !self.world.blackout())
            .and_then(|(id, vol)| Track::from_id(id).map(|t| (t, vol)));
        match (radio, self.world.music_track(content)) {
            (Some((t, vol)), _) => audio.set_music_vol(Some(t), vol),
            (None, Some((id, vol))) => audio.set_music_vol(Track::from_id(&id), vol),
            (None, None) => audio.set_music(None),
        }
        audio.set_rain(self.world.weather().is_wet());
        self.ambience_loops(audio, night);
        // café life: an occasional footstep / coffee machine while it is busy
        if night && self.speed > 0 {
            let m = self.world.minute();
            if m != self.last_ambience_minute {
                self.last_ambience_minute = m;
                let n = self.world.guests_inside() as u32;
                if n > 3 && m.is_multiple_of(7) {
                    audio.play_sfx_vol(Sfx::Footsteps, 0.6);
                }
            }
        }
    }

    /// The CC0 loops inside the café (package K): the soup pot simmering on the stove (one
    /// loop however many stoves, louder while it bubbles and asks for a stir) and the storm's
    /// wind at the window. Not while Karolina is out on the walk home or the back step.
    fn ambience_loops(&self, audio: &mut Audio, night: bool) {
        use crate::audio::AmbLoop;
        if !night || self.walk.is_open() {
            return;
        }
        if let Some(p) = &self.world.equipment().pot {
            let t = self.world.minutes_since_open();
            let level = if p.bubbling_until.is_some_and(|u| t < u) {
                1.0
            } else if p.stirred_until.is_some_and(|u| t < u) {
                0.7
            } else {
                0.45
            };
            audio.want_amb(AmbLoop::Soup, level);
        }
        if self.world.weather() == crate::content::Weather::Storm {
            audio.want_amb(AmbLoop::Storm, 0.55);
        }
        // the storm night: the wind round the café, a little louder once the line is closed
        let storm = self.world.storm();
        if storm.tonight {
            let level = if storm.cancelled_at.is_some() {
                0.6
            } else {
                0.4
            };
            audio.want_amb(AmbLoop::Storm, level);
        }
    }

    // ───────────────────────── test / automation helpers ─────────────────────────

    /// Jumps to `night` (from Day 1) and, if `minute` is set, runs that night up to that
    /// minute. Cutscenes on the way are finished at once with their default path.
    pub fn fast_forward(
        &mut self,
        content: &Rc<Content>,
        loc: &Locale,
        night: u32,
        minute: Option<i32>,
        hire: bool,
    ) {
        let mut guard = 0;
        // past nights are played by the sim's player-bot (a calm player: by hand for the
        // regulars, moments, the jar, a polaroid every morning), so a late night looks like
        // one a person would have reached
        let mut bot = sim::bot::PlayerBot::new(sim::bot::BotConfig {
            seed: self.world.seed(),
            hire_nights: if hire { vec![3, 5, 9, 14] } else { Vec::new() },
            ..Default::default()
        });
        while self.world.night() < night && guard < 400 && !self.world.is_closed() {
            let _ = bot.run_night(&mut self.world, content);
            guard += 1;
        }
        if self.world.phase() == Phase::Day {
            bot.day(&mut self.world, content);
        }

        let _ = self.world.drain_events();
        self.deferred.clear();
        self.summary = None;
        self.summary_pending = None;
        if let Some(m) = minute {
            self.world.start_night(content);
            let mut g = 0;
            // (minutes since 22:00: negative = the preparation half hour)
            while self.world.phase() == Phase::Night
                && self.world.minutes_since_open() < m
                && g < 20_000
            {
                if let Some(id) = self.world.playing_cutscene().map(str::to_string) {
                    let _ = self.world.apply(
                        content,
                        Command::CutsceneFinished {
                            id,
                            choices: Vec::new(),
                        },
                    );
                }
                bot.step(&mut self.world, content);
                self.world.tick(content);
                self.scene.sim_steps += 1;
                let evs = self.world.drain_events();
                for e in evs {
                    if !matches!(e, SimEvent::PlayCutscene(_)) {
                        self.handle_event(content, e, Some(loc));
                    }
                }
                g += 1;
            }
            while let Some(id) = self.world.playing_cutscene().map(str::to_string) {
                let _ = self.world.apply(
                    content,
                    Command::CutsceneFinished {
                        id,
                        choices: Vec::new(),
                    },
                );
            }
            let _ = self.world.drain_events();
            self.notices.clear();
        }
        self.player = {
            let mut p = CutscenePlayer::new(content);
            let snap = self.snapshot.clone();
            let c = content.clone();
            p.set_condition_check(move |cond| {
                snap.borrow()
                    .as_ref()
                    .is_some_and(|w| w.eval_conditions(&c, std::slice::from_ref(cond)))
            });
            p
        };
        self.intro = false;
    }

    /// Runs the current night to 06:00 and shows the morning summary.
    pub fn run_to_morning(&mut self, content: &Rc<Content>, loc: &Locale) {
        let mut g = 0;
        while self.world.phase() == Phase::Night && g < 60_000 {
            if let Some(id) = self.world.playing_cutscene().map(str::to_string) {
                let _ = self.world.apply(
                    content,
                    Command::CutsceneFinished {
                        id,
                        choices: Vec::new(),
                    },
                );
            }
            self.world.tick(content);
            self.scene.sim_steps += 1;
            for e in self.world.drain_events() {
                if !matches!(e, SimEvent::PlayCutscene(_)) {
                    self.handle_event(content, e, Some(loc));
                }
            }
            g += 1;
        }
        while let Some(id) = self.world.playing_cutscene().map(str::to_string) {
            let _ = self.world.apply(
                content,
                Command::CutsceneFinished {
                    id,
                    choices: Vec::new(),
                },
            );
        }
        let _ = self.world.drain_events();
        self.summary = self.summary_pending.take();
        self.notices.clear();
    }

    /// Plays any cutscene by id (endings preview, tests) without the sim asking.
    pub fn play_scene(&mut self, content: &Rc<Content>, id: &str) {
        if let Some(def) = content.cutscene(id) {
            *self.snapshot.borrow_mut() = Some(self.world.clone());
            self.stage_player();
            self.player.start(def);
            self.intro = true;
        }
    }

    /// Selects a regular by id and opens their bubble (with Talk if available).
    pub fn focus_regular(&mut self, content: &Rc<Content>, loc: &Locale, regular: &str) {
        let party = self
            .world
            .parties()
            .iter()
            .find(|p| p.regular.as_deref() == Some(regular))
            .map(|p| p.id);
        let Some(guest) = party.and_then(|pid| {
            self.world
                .guests()
                .iter()
                .find(|g| g.party == pid && g.leader)
                .map(|g| g.id)
        }) else {
            return;
        };
        let a = AgentRef::Guest(guest);
        self.selected = Some(a);
        let text = self
            .world
            .ambient_line_for(content, guest)
            .map(|k| loc.tr_owned(&k))
            .unwrap_or_default();
        self.bubble = Some(Bubble {
            agent: a,
            text,
            talk: self.world.talk_available(content, guest),
            pinned: true,
            until: f32::MAX,
        });
        self.speed = 0;
        if let Some(g) = self.world.guests().iter().find(|g| g.id == guest) {
            self.cam =
                self.scene
                    .overview_camera(Rect::new(0.0, 0.0, screen_width(), screen_height()));
            self.cam.center = vec2(g.pos.0, g.pos.1 - 20.0);
            self.cam.zoom *= 2.2;
            self.cam_ready = true;
        }
    }

    // ───────────────────────── frame ─────────────────────────

    /// One frame: logic, world, HUD and panels. Call between `ui.begin_frame()` and
    /// `ui.end_frame()`; then call [`GameScreen::hotkeys`].
    pub fn frame(&mut self, ui: &mut Ui, cx: &mut Ctx, dt: f32) {
        let t_start = std::time::Instant::now();
        let mut ticks_run = 0u32;
        let content = cx.content;
        // 3:33 — everything holds its breath
        if self.clock_stop <= 0.0 {
            self.time += dt;
        }
        let loc = cx.loc;
        // faces for avatars: baked before anything of this frame is drawn
        self.register_faces(ui, &loc);
        hands_ui::faces(self, ui, content, &loc);
        build::faces(self, ui, content, &loc);
        for e in std::mem::take(&mut self.deferred) {
            self.handle_event(content, e, Some(&loc));
        }
        // the walk home / the back step own the screen while they play
        if walk_home::frame(self, ui, cx, dt) {
            return;
        }

        // camera fits the café on the first frame
        if !self.cam_ready {
            self.cam =
                self.scene
                    .overview_camera(Rect::new(0.0, 0.0, screen_width(), screen_height()));
            self.cam_ready = true;
        }

        // 1. cutscene or sim
        self.relaxed = cx.settings.relaxed_minigames;
        let mut finished = None;
        if self.player.is_playing() {
            if let CutsceneStatus::Finished { id, choices } = self.player.update(ui, &loc, dt) {
                finished = Some((id, choices.iter().map(|c| c.index).collect::<Vec<_>>()));
            }
            for a in self.player.drain_audio_requests() {
                cx.audio.request(&a);
            }
            let _ = self.player.drain_effects();
        }
        if let Some((id, choices)) = finished {
            self.cutscene_finished(content, id, choices);
        }
        // the autoplayer reads a scene for a moment, then moves on (default choices)
        if (self.hands.autoplay.is_some() || self.hands.skip_scenes) && self.player.is_playing() {
            self.hands.scene_wait += dt;
            if self.hands.scene_wait > if self.hands.skip_scenes { 0.5 } else { 4.0 } {
                self.hands.scene_wait = 0.0;
                self.finish_cutscene_now(content, &loc);
            }
        }
        let events_before = self.world.drain_events();
        for e in &events_before {
            Self::event_audio(cx.audio, e);
        }
        for e in events_before {
            self.handle_event(content, e, Some(&loc));
        }
        let (rate, tick) = self.sim_rate(content);
        if self.clock_stop > 0.0 {
            self.clock_stop = (self.clock_stop - dt).max(0.0);
        } else if !self.frozen() {
            self.acc += dt.min(0.25) * rate;
            let mut guard = 0;
            while self.acc >= tick && guard < 64 {
                self.acc -= tick;
                self.world.tick(content);
                self.scene.sim_steps += 1;
                guard += 1;
                ticks_run += 1;
                let evs = self.world.drain_events();
                for e in &evs {
                    Self::event_audio(cx.audio, e);
                }
                for e in evs {
                    self.handle_event(content, e, Some(&loc));
                }
                if self.cutscene_playing() || self.world.playing_cutscene().is_some() {
                    self.acc = 0.0;
                    break;
                }
            }
        }
        self.drain_and_handle(content, Some(&loc));
        if !self.player.is_playing() {
            for (sfx, vol) in self.hands_frame(content, dt) {
                cx.audio.play_sfx_vol(sfx, vol);
            }
            if self.hands.autoplay.is_some() {
                self.autoplay_step(content, dt);
                self.autoplay_day(content, dt, 1.5);
            }
        }
        self.autosave_if_due(content, cx.store);
        self.update_audio(content, cx.audio);
        let anim_dt = if self.frozen() && !self.cutscene_playing() {
            dt * 0.35
        } else if self.overlay_open() {
            dt * 0.6
        } else {
            dt * self.speed.max(1) as f32
        };
        self.scene.update(anim_dt);
        let alpha = if self.frozen() || self.clock_stop > 0.0 {
            1.0
        } else {
            self.acc / tick
        };
        // little icons over heads and on furniture stay readable when zoomed out
        crate::render::set_world_icon_scale(crate::render::icon_scale_for_zoom(self.cam.zoom));
        self.scene.refresh(&self.world, self.time, alpha);

        let t_sim = std::time::Instant::now();
        // 2. world
        if self.player.is_playing() {
            // story scenes play on the live café (its furniture, lamps, weather, people)
            self.scene
                .draw_cutscene(&self.world, content, self.time, &self.player);
        } else {
            let wants_mouse = ui.wants_mouse();
            // the mini-game owns WASD / arrows / the wheel while it is open
            if !wants_mouse
                && self.pause.is_none()
                && !self.automation
                && self.hands.minigame.is_none()
                && !ui.wants_keyboard()
                && !(self.panel == Some(Panel::Crew) && crew_ui::arrows_held())
            {
                self.cam.update();
            }
            if let Some(tgt) = self.cam_target {
                self.cam.center = self.cam.center.lerp(tgt, (dt * 2.5).min(1.0));
                if self.cam.center.distance(tgt) < 6.0 || self.automation {
                    self.cam_target = None;
                }
            }
            // screenshot runs: SHIFTELLA_ZOOM=k follows Karolina k× closer than the overview
            if self.automation
                && let Some(z) = std::env::var("SHIFTELLA_ZOOM")
                    .ok()
                    .and_then(|z| z.parse::<f32>().ok())
                && let Some(k) = self.world.karolina()
            {
                let base = self.scene.overview_camera(Rect::new(
                    0.0,
                    0.0,
                    screen_width(),
                    screen_height(),
                ));
                self.cam.zoom = base.zoom * z;
                let want = vec2(k.pos.0, k.pos.1 - 40.0);
                self.cam.center = self.cam.center.lerp(want, (dt * 3.0).min(1.0));
            }
            // build mode keeps the hall above its panel (else the scene's own clamp)
            build::camera(self, dt);
            photo_mode::clamp_camera(self, Rect::new(0.0, 0.0, screen_width(), screen_height()));
            let cam = self.cam.to_camera2d();
            let mouse_world = self.cam.screen_to_world(Vec2::from(mouse_position()));
            self.hovered = None;
            let mut pick = None;
            if !wants_mouse && self.pause.is_none() {
                pick = self
                    .scene
                    .pick(&self.world, content, mouse_world, self.time);
                if let Some(Pick::Agent(a)) = pick {
                    self.hovered = Some(a);
                }
            }
            let ghost = build::ghost(self, content, mouse_world, wants_mouse);
            let t = self.time;
            let (sel, hov) = (self.selected, self.hovered);
            let blankets = stories_ui::blankets(self);
            let mut extra = |q: &mut crate::render::RenderQueue| {
                if let Some(g) = &ghost {
                    g.push(q, t);
                }
                stories_ui::push_blankets(q, &blankets, t);
            };
            self.scene
                .draw(&self.world, content, &cam, t, sel, hov, &mut extra);
            let target = if wants_mouse || self.pause.is_some() || self.panel == Some(Panel::Build)
            {
                None
            } else {
                hands_view::target(self, content, pick, mouse_world)
            };
            self.scene.hover_moment = match target {
                Some(hands_view::Target::Moment(i)) => {
                    self.scene.markers.get(i).map(|m| m.0.clone())
                }
                _ => None,
            };
            self.stories.hover = match target {
                Some(hands_view::Target::Story(g)) => Some(g),
                _ => None,
            };
            self.scene.sec_per_minute = self.world.real_seconds_per_tick(content) * 10.0;
            if !self.photo.is_open() {
                hands_view::draw_world(self, content, &loc, target, mouse_world);
            }
            hands_view::draw_world(self, content, &loc, target, mouse_world);
            stories_ui::draw_world(self);
            if self.panel == Some(Panel::Build) {
                build::draw_world_overlay(self, content, ghost.as_ref());
            }
            if !self.photo.is_open() {
                self.world_click(
                    ui,
                    content,
                    pick,
                    target,
                    mouse_world,
                    ghost.as_ref(),
                    &loc,
                    dt,
                );
            }
        }

        let t_world = std::time::Instant::now();
        // 3. UI (photos for polaroids render first: they use their own camera)
        journal_ui::prepare(self);
        ui.set_camera();
        self.anchors.clear();
        if self.player.is_playing() {
            self.player.draw_ui(ui, &loc);
        } else if self.photo.is_open() {
            // the viewfinder: only the café and the camera, no HUD
            photo_mode::frame_ui(self, ui, cx, dt);
        } else if stories_ui::talk_ui(self, ui, cx, dt) {
            // the storm's 03:00 talk holds the café (letterbox, dialogue)
        } else {
            signature_ui::board(self, ui, cx);
            hud::draw(self, ui, cx);
            if self.summary.is_none() {
                extras_ui::atmosphere(self, ui, cx);
                // screenshot runs: SHIFTELLA_COZY_HINT=1 shows the meter's world hint
                if self.automation && std::env::var("SHIFTELLA_COZY_HINT").is_ok() {
                    self.scene.cozy_hint = Some(self.world.coziness().weakest);
                }
            }
            // the left column under the meter first: the rail keeps clear of its anchors
            if self.world.phase() == Phase::Night && self.summary.is_none() {
                walk_home::backstep_chip(self, ui, cx);
                photo_mode::button(self, ui, cx);
                stories_ui::draw(self, ui, cx);
            }
            hands_ui::hud(self, ui, cx);
            if self.world.phase() == Phase::Night && self.summary.is_none() {
                extras_ui::tip_goal(self, ui, cx);
                extras_ui::napkin(self, ui, cx);
            }
            // after the HUD and the rail: the bubble keeps clear of them
            if !matches!(self.panel, Some(Panel::App | Panel::Duties)) {
                hud::bubble(self, ui, cx);
            }
            match self.panel {
                Some(Panel::Crew) => crew_ui::panel(self, ui, cx),
                Some(Panel::Duties) => duties::board(self, ui, cx),
                Some(Panel::Log) => panels::log(self, ui, cx),
                Some(Panel::App) => phone_app::draw(self, ui, cx),
                Some(Panel::Build) => build::panel(self, ui, cx),
                Some(Panel::Help) => tutorial::help(self, ui, cx),
                Some(Panel::Journal) => journal_ui::journal_panel(self, ui, cx),
                Some(Panel::Payroll) => phone_app::payroll_panel(self, ui, cx),
                Some(Panel::Attic) => attic_ui::panel(self, ui, cx),
                None => {}
            }
            if self.world.phase() == Phase::Day && self.summary.is_none() {
                day::planning(self, ui, cx);
            }
            hands_ui::overlays(self, ui, cx, dt);
            signature_ui::overlays(self, ui, cx);
            photo_mode::frame_ui(self, ui, cx, dt);
            self.tutorial_update(cx);
            tutorial::draw(self, ui, cx);
            if self.summary.is_some() {
                day::summary(self, ui, cx);
            }
            coach::draw(self, ui, cx);
            if self.pause.is_some() {
                pause::draw(self, ui, cx);
            }
        }
        // the mini-game loops (steam hiss, pour trickle) follow the game; silence otherwise
        let loops = self
            .hands
            .minigame
            .as_ref()
            .filter(|_| self.pause.is_none())
            .map(|m| m.game.loops())
            .unwrap_or_default();
        cx.audio.set_loops(loops.steam, loops.trickle);
        // a story moment to read: the night holds under it
        if !self.player.is_playing() && self.hands.minigame.is_none() {
            self.draw_reading(ui, &loc);
        }
        // toasts wait while a scene or a mini-game is on screen (the cup has her attention)
        if !self.player.is_playing() && self.hands.minigame.is_none() && !self.stories.talk_open() {
            self.flush_notices(ui, &loc);
            // the build panel sits over the dock: toasts rise above it
            let inset = if self.panel == Some(Panel::Build) {
                60.0 + build::PANEL_H + 8.0
            } else {
                56.0
            };
            ui.set_toast_bottom_inset(inset);
            // by day the planning panel owns the right column: toasts stay beside it
            let right = self
                .anchors
                .iter()
                .find(|a| a.0 == "day.panel")
                .map_or(0.0, |a| ui.screen().w - a.1.x);
            ui.set_toast_right_inset(right);
            for ev in ui.draw_toasts(ToastAnchor::BottomRight) {
                self.toast_clicked(content, ev.toast, ev.tag);
            }
        }
        let ms = |a: std::time::Instant, b: std::time::Instant| (b - a).as_secs_f32() * 1000.0;
        let t_end = std::time::Instant::now();
        self.times = FrameTimes {
            sim_ms: ms(t_start, t_sim),
            world_ms: ms(t_sim, t_world),
            ui_ms: ms(t_world, t_end),
            ticks: ticks_run,
        };
    }

    /// A plain toast: short at night, at most two on screen, no repeats in a row.
    fn push_plain_toast(
        &mut self,
        ui: &mut Ui,
        kind: ToastKind,
        icon: Icon,
        title: String,
        body: Option<String>,
    ) {
        let (title, body) = match body {
            Some(b) => (title, Some(b)),
            None => split_title(ui, &title),
        };
        let sig = format!("{title}|{}", body.as_deref().unwrap_or(""));
        if self
            .recent_toasts
            .iter()
            .any(|(_, s, at)| *s == sig && self.time - at < 8.0)
        {
            return;
        }
        let mut t = Toast::new(kind, title).icon(icon);
        if let Some(b) = body {
            t = t.body(b);
        }
        let night = self.world.phase() == Phase::Night;
        if night {
            t = t.duration(night_duration(kind));
        }
        let id = ui.push_toast(t);
        self.recent_toasts.push_back((id, sig, self.time));
        // at night the stack collapses to the two newest (the log keeps everything)
        let keep = if night { 2 } else { 3 };
        while self.recent_toasts.len() > keep {
            if let Some((old, _, _)) = self.recent_toasts.pop_front() {
                ui.dismiss_toast(old);
            }
        }
    }

    fn flush_notices(&mut self, ui: &mut Ui, loc: &Locale) {
        for n in std::mem::take(&mut self.notices) {
            match n {
                Notice::Toast {
                    kind,
                    icon,
                    title,
                    body,
                } => self.push_plain_toast(ui, kind, icon, title, body),
                Notice::Choice { event, choices } => {
                    let title = loc.tr_owned(&format!("event.{event}.title"));
                    let body = loc.tr_owned(&format!("event.{event}.text"));
                    let mut t = Toast::new(ToastKind::Event, title)
                        .body(body)
                        .icon(Icon::Sparkle)
                        .sticky();
                    for (i, c) in choices.iter().enumerate() {
                        let label = loc.tr_owned(&format!("event.{event}.choice.{c}"));
                        t = if i == 0 {
                            t.primary_action(label, i as u32)
                        } else {
                            t.action(label, i as u32)
                        };
                    }
                    let id = ui.push_toast(t);
                    self.choice_toasts.push((id, event, choices));
                }
                Notice::Key {
                    kind,
                    icon,
                    key,
                    args,
                } => {
                    let a = resolve_args(loc, &args);
                    let text = wv::tr_args(loc, &key, &a);
                    self.push_plain_toast(ui, kind, icon, text, None);
                }
                Notice::KeyTitled {
                    kind,
                    icon,
                    title,
                    key,
                    args,
                } => {
                    let a = resolve_args(loc, &args);
                    let text = wv::tr_args(loc, &key, &a);
                    self.push_plain_toast(ui, kind, icon, loc.tr_owned(&title), Some(text));
                }
                Notice::ChoiceResolved { event, choice } => {
                    if let Some(i) = self.choice_toasts.iter().position(|c| c.1 == event) {
                        let (id, _, _) = self.choice_toasts.remove(i);
                        ui.dismiss_toast(id);
                    }
                    let key = format!("event.{event}.result.{choice}");
                    if loc.has(&key) {
                        let title = loc.tr_owned(&format!("event.{event}.title"));
                        ui.push_toast(
                            Toast::new(ToastKind::Event, title)
                                .body(loc.tr_owned(&key))
                                .icon(Icon::Sparkle),
                        );
                    }
                }
            }
        }
    }

    fn toast_clicked(&mut self, content: &Rc<Content>, toast: ToastId, tag: u32) {
        if let Some(i) = self.choice_toasts.iter().position(|c| c.0 == toast) {
            let (_, event, choices) = self.choice_toasts[i].clone();
            if let Some(choice) = choices.get(tag as usize) {
                let _ = self.command(
                    content,
                    Command::AnswerEvent {
                        event,
                        choice: choice.clone(),
                    },
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn world_click(
        &mut self,
        ui: &mut Ui,
        content: &Rc<Content>,
        pick: Option<Pick>,
        target: Option<hands_view::Target>,
        mouse_world: Vec2,
        ghost: Option<&build::Ghost>,
        loc: &Locale,
        dt: f32,
    ) {
        use hands::Intent;
        use hands_view::Target;
        let night = self.world.phase() == Phase::Night;
        let blocked = ui.wants_mouse()
            || self.pause.is_some()
            || self.summary.is_some()
            || self.automation
            || self.stories.talk_open();
        let down = is_mouse_button_down(MouseButton::Left);
        // hold-to-wash: the mouse went down on the sink, or E is held (keyboard play)
        if night {
            let mouse_hold = down && self.hands.press_sink.is_some() && !self.automation;
            let key_hold = !ui.wants_keyboard()
                && !self.automation
                && self.hands.minigame.is_none()
                && is_key_down(crate::settings::keys::key(
                    crate::settings::keys::Action::Chore,
                ));
            let auto_hold = self.hands.autoplay.is_some();
            self.wash_input(content, mouse_hold || key_hold || auto_hold, dt);
            // hold chores at an appliance (warm up, purge, clean): the same hold
            let equip_hold = down && self.hands.press_equip.is_some() && !self.automation;
            self.equip_input(content, equip_hold || key_hold || auto_hold, dt);
        }
        if !down {
            self.hands.press_sink = None;
            self.hands.press_equip = None;
        }
        // a mop swipe across a dirt blot
        if let Some(mut sw) = self.hands.swipe {
            if down {
                sw.travelled += sw.last.distance(mouse_world);
                sw.last = mouse_world;
                if sw.travelled >= hands::SWIPE_MIN {
                    self.hands.swipe = None;
                    self.intent(
                        content,
                        Intent::Mop {
                            tile: sw.tile,
                            front: ui.shift(),
                        },
                    );
                } else {
                    self.hands.swipe = Some(sw);
                }
            } else {
                // only a click: show how (an arrow over the blot)
                self.hands.swipe = None;
                self.hands.swipe_hint = Some((sw.tile, self.time));
                self.hands.swipe_hints_shown += 1;
            }
        }
        // right-click (not a camera drag): cancel the current act / drop a queued chip
        if is_mouse_button_pressed(MouseButton::Right) && !blocked {
            self.hands.right_press = Some(Vec2::from(mouse_position()));
        }
        if is_mouse_button_released(MouseButton::Right)
            && let Some(p0) = self.hands.right_press.take()
            && p0.distance(Vec2::from(mouse_position())) < 6.0
            && night
            && !blocked
            && self.panel != Some(Panel::Build)
        {
            match target {
                Some(Target::Chip(i)) => {
                    self.intent(content, Intent::Unqueue(i));
                }
                _ => {
                    self.intent(content, Intent::Cancel);
                }
            }
        }
        // build mode: a press on a placed piece becomes a drag, a released drag drops
        build::frame(self, ui, content, loc, blocked);
        if blocked || !is_mouse_button_pressed(MouseButton::Left) {
            return;
        }
        if self.panel == Some(Panel::Build) {
            build::click(self, ui, content, ghost, mouse_world, loc);
            return;
        }
        let front = ui.shift();
        match target {
            Some(Target::Chip(i)) => {
                self.intent(content, Intent::Unqueue(i));
                return;
            }
            Some(Target::Coin(table)) => {
                self.intent(content, Intent::Tip { table, front });
                return;
            }
            Some(Target::Dirt(tile)) => {
                self.hands.swipe = Some(hands::Swipe {
                    tile,
                    last: mouse_world,
                    travelled: 0.0,
                });
                return;
            }
            Some(Target::Sink(sink)) => {
                self.hands.press_sink = Some(sink);
                self.intent(content, Intent::Sink { sink, front });
                return;
            }
            Some(Target::Table(table)) => {
                self.intent(content, Intent::Table { table, front });
                return;
            }
            Some(Target::Jukebox(_)) => {
                self.intent(content, Intent::Jukebox);
                return;
            }
            Some(Target::Window) => {
                self.intent(content, Intent::Window);
                return;
            }
            Some(Target::Equip(uid)) => {
                self.hands.press_equip = Some(uid);
                self.intent(content, Intent::Equip { uid, front });
                return;
            }
            Some(Target::Lamp(uid)) => {
                self.intent(content, Intent::Lamp { uid });
                return;
            }
            Some(Target::Checklist(i)) => {
                if let Some(it) = content.app.checklists.get(i) {
                    let id = it.id.clone();
                    if self.hand(content, Command::TickChecklist { id }) {
                        self.hands.tut.bump("ritual_ticked");
                    }
                }
                return;
            }
            Some(Target::Moment(i)) => {
                if let Some(id) = self.scene.markers.get(i).map(|m| m.0.clone()) {
                    self.open_moment(&id);
                }
                return;
            }
            Some(Target::Story(guest)) => {
                self.open_story(guest);
                return;
            }
            _ => {}
        }
        match pick {
            Some(Pick::Agent(a)) => {
                self.selected = Some(a);
                match a {
                    AgentRef::Staff(id) => {
                        self.ui_state.crew_selected = Some(id);
                        self.panel = Some(Panel::Crew);
                        self.bubble = None;
                    }
                    AgentRef::Guest(g) => {
                        let acted =
                            night && self.intent(content, Intent::Guest { guest: g, front });
                        let talk = self.world.talk_available(content, g);
                        // a click that did something shows no bubble (unless there's a Talk)
                        if acted && talk.is_none() {
                            return;
                        }
                        let line = self.world.ambient_line_for(content, g);
                        let text = match line {
                            Some(k) => loc.tr_owned(&k),
                            None => hud::guest_line(self, content, g, loc),
                        };
                        self.bubble = Some(Bubble {
                            agent: a,
                            text,
                            talk,
                            pinned: true,
                            until: self.time + 12.0,
                        });
                    }
                }
            }
            _ => {
                self.selected = None;
                if self.bubble.as_ref().is_some_and(|b| b.pinned) {
                    self.bubble = None;
                }
            }
        }
    }

    /// Hotkeys — after `ui.end_frame()` (widgets consumed their keys first).
    pub fn hotkeys(&mut self, ui: &mut Ui, cx: &mut Ctx) {
        use hands::Intent;
        let content = cx.content;
        if self.player.is_playing() || self.automation {
            return;
        }
        use crate::settings::keys::{Action as A, pressed};
        if pressed(ui, A::Mute) {
            cx.audio.toggle_mute();
        }
        if self.walk.is_open() || stories_ui::talk_hotkeys(self, ui) {
            return;
        }
        if self.pause.is_some() {
            if ui.esc() {
                self.pause = None;
            }
            return;
        }
        if self.summary.is_some() {
            if ui.enter() || ui.esc() || ui.key(KeyCode::Space) {
                journal_ui::keep_morning(self, content);
                self.summary = None;
                walk_home::begin(self, content, None);
            }
            return;
        }
        // an open mini-game / card / book keeps the keyboard (it consumed its own keys)
        if self.hands.minigame.is_some() {
            return;
        }
        // the house drink's card and the camera keep their keys (Esc, P)
        if signature_ui::hotkeys(self, ui) || photo_mode::hotkeys(self, ui, &cx.loc) {
            return;
        }
        if ui.esc() {
            if self.hands.book.is_some() {
                self.hands.book = None;
            } else if self.hands.moment.is_some() {
                self.hands.moment = None;
            } else if self.stories.card.is_some() {
                self.stories.card = None;
            } else if self.hands.records_open {
                self.hands.records_open = false;
            } else if self.hands.order.is_some() {
                self.hands.order = None;
            } else if self.panel == Some(Panel::Build) && self.ui_state.build.active() {
                self.ui_state.build.cancel();
            } else if self.panel == Some(Panel::Crew) {
                // the character card closes at once (and the café's selection ring with it)
                self.panel = None;
                self.selected = None;
            } else if self.bubble.as_ref().is_some_and(|b| b.pinned) || self.selected.is_some() {
                self.bubble = None;
                self.selected = None;
            } else if self.panel.is_some() {
                self.panel = None;
            } else {
                self.pause = Some(pause::PauseMenu::default());
            }
            return;
        }
        let card = self.hands.order.is_some() || self.hands.book.is_some();
        let toggle = |p: &mut Option<Panel>, which: Panel| {
            *p = if *p == Some(which) { None } else { Some(which) };
        };
        if !card {
            // the keys come from Settings → Controls (crate::settings::keys)
            if pressed(ui, A::Crew) {
                toggle(&mut self.panel, Panel::Crew);
            }
            // H = the duties board (P stays as the old priorities key while P is free)
            if pressed(ui, A::Duties)
                || (crate::settings::keys::legacy_duties_p() && ui.key(KeyCode::P))
            {
                toggle(&mut self.panel, Panel::Duties);
            }
            if pressed(ui, A::App) {
                toggle(&mut self.panel, Panel::App);
            }
            if pressed(ui, A::Build) {
                toggle(&mut self.panel, Panel::Build);
            }
            if pressed(ui, A::Log) {
                toggle(&mut self.panel, Panel::Log);
            }
            if pressed(ui, A::Book) {
                self.open_book(None);
            }
            if pressed(ui, A::Journal) {
                toggle(&mut self.panel, Panel::Journal);
            }
            if pressed(ui, A::Jukebox) {
                self.intent(content, Intent::Jukebox);
            }
        }
        if pressed(ui, A::Help) {
            toggle(&mut self.panel, Panel::Help);
        }
        if pressed(ui, A::Pause) {
            self.toggle_pause();
        }
        for (a, s) in [(A::Speed1, 1), (A::Speed2, 2), (A::Speed3, 3)] {
            if pressed(ui, a) {
                self.set_speed(s);
            }
        }
        if ui.key(KeyCode::Home) {
            self.cam_ready = false;
        }
        if self.panel == Some(Panel::Build) {
            build::hotkeys(self, ui, content);
        }
        if self.world.phase() == Phase::Day && ui.enter() {
            self.start_night(content);
        }
        // keyboard play at night (design §9: a whole night keyboard-only)
        if self.world.phase() == Phase::Night && !card && self.panel != Some(Panel::Build) {
            let front = ui.shift();
            if ui.enter() {
                // the focused ticket on the rail ([ / ] move), else the first one
                self.act_on_rail(content);
            }
            // at the sink E is the hold-to-wash key, not "the next chore"
            let at_sink = self
                .world
                .player()
                .current
                .as_ref()
                .is_some_and(|c| matches!(c.act, sim::PlayerAct::Wash { .. }) && c.stage == 1);
            // (and at a hold chore at an appliance)
            let at_sink = at_sink || self.at_hold_chore();
            if pressed(ui, A::Chore) && !at_sink {
                self.intent(content, Intent::Chore { front });
            }
            if pressed(ui, A::Greet) {
                self.intent(content, Intent::Greet);
            }
            // F: the nearest night moment's card
            if pressed(ui, A::Moment)
                && let Some(id) = self.nearest_moment()
            {
                self.open_moment(&id);
            }
            if ui.key(KeyCode::Backspace) {
                self.intent(content, Intent::Cancel);
            }
        }
    }

    /// Enter on the rail: claim / open the focused ticket (or the first).
    pub fn act_on_rail(&mut self, content: &Rc<Content>) {
        let tickets = hands_ui::rail_tickets(self, content);
        let pick = self
            .hands
            .rail
            .focused
            .and_then(|f| tickets.iter().find(|t| t.0 == f))
            .or_else(|| tickets.first())
            .cloned();
        if let Some((_, party, guest)) = pick {
            self.intent(content, hands::Intent::Ticket { party, guest });
        }
    }

    /// A deep link from the app (a digest insight, a bot button, the payroll screen).
    pub fn follow_link(&mut self, link: crate::content::handson::AppLink) {
        use crate::content::handson::AppLink as L;
        match link {
            L::Rozpis => {
                self.panel = Some(Panel::App);
                self.ui_state.app_tab = 0;
            }
            L::Board => {
                self.panel = Some(Panel::App);
                self.ui_state.app_tab = 3;
            }
            L::RecipeBook => {
                self.panel = None;
                self.open_book(None);
            }
            L::Pantry => {
                self.panel = None;
                self.open_book(None);
                if let Some(b) = &mut self.hands.book {
                    b.state.tab = 3;
                }
            }
            L::Build => self.panel = Some(Panel::Build),
            L::Duties => self.panel = Some(Panel::Duties),
            L::Money => self.panel = Some(Panel::Payroll),
            L::Journal => self.panel = Some(Panel::Journal),
        }
    }

    /// The recipe book (K); `pick_for` = from an order card (pick mode).
    pub fn open_book(&mut self, pick_for: Option<(u32, crate::ui::handson::BrewMode)>) {
        if self.hands.book.is_some() && pick_for.is_none() {
            self.hands.book = None;
            return;
        }
        self.hands.book = Some(hands::BookOpen {
            pick_for: pick_for.map(|p| p.0),
            mode: pick_for.map(|p| p.1).unwrap_or_default(),
            note: self.hands.last_experiment.take(),
            margin: self.hands.last_margin.clone(),
            ..Default::default()
        });
        self.hands
            .equip_sounds
            .push((crate::audio::Sfx::BookOpen, 0.7));
        self.hands.tut.bump("book_opened");
    }
}

/// Events that drive the flow (handled even without a locale).
fn is_flow_event(e: &SimEvent) -> bool {
    matches!(
        e,
        SimEvent::DayStarted { .. }
            | SimEvent::NightEnded { .. }
            | SimEvent::PlayCutscene(_)
            | SimEvent::EndingReached(_)
            | SimEvent::CafeClosed
            | SimEvent::ClockStop { .. }
    )
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod capitalize_tests {
    use super::capitalize_first;

    #[test]
    fn toast_body_starts_with_a_capital_in_every_language() {
        assert_eq!(
            capitalize_first("say hello, take the order"),
            "Say hello, take the order"
        );
        assert_eq!(
            capitalize_first("pozdrav, vezmi objednávku"),
            "Pozdrav, vezmi objednávku"
        );
        assert_eq!(capitalize_first("привітайся"), "Привітайся");
        assert_eq!(capitalize_first("поздоровайся"), "Поздоровайся");
        assert_eq!(capitalize_first("Already"), "Already");
        assert_eq!(capitalize_first(""), "");
    }
}

/// A story moment held on screen until it's read.
#[derive(Debug, Clone)]
pub struct ReadCard {
    pub icon: Icon,
    pub title: String,
    pub text: String,
}
