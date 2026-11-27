//! The app state machine: Boot → (language picker) → Main menu → New story / Sandbox /
//! Continue → Game (opening, days, nights, cutscenes) → Ending → Credits → Main menu.
//!
//! [`route`] is the pure transition table (tested headless); [`run`] drives it with a window.
//!
//! Test automation (env):
//! * `SHIFTELLA_SHOT=path.png` — render `SHIFTELLA_FRAMES` (default 45) frames, save, exit
//! * `SHIFTELLA_SCENE=menu|language|opening|story_day1|night|night9_anwar|build|crew|app|
//!   crew_karolina|crew_helper|crew_classic|crew_day|crew_hire|
//!   duties|log|summary|pause|ending_light|ending_last_train|credits|sandbox|
//!   night1_handson|order_card|minigame|jukebox|book|moments|garlands|cat|pantry|napkins|
//!   journal|polaroid|themes|rozpis|payroll|season_snow|night27_snow|night22|night14|day|
//!   build_drag|attic|backroom|requests|corner|before_after|to_go` (`SHIFTELLA_BUILD_DEF` / `_AT=x,y`
//!   / `_TAB` / `_TURNS`: a held or dragged piece)
//!   equipment|guest_names|walk_home|walk_home_snow|walk_home_rain|backstep|backstep_snow|
//!   story_marker|story_card|storm|storm_talk|hands_ux` (`SHIFTELLA_HUX=checklist|rituals|
//!   queue|follow|gesture`: the checklist states, the queue strip with 8 acts, the follow chip,
//!   a ritual gesture over its piece — `SHIFTELLA_GESTURE=<checklist id | equip.<chore>>`,
//!   `SHIFTELLA_GESTURE_T=<s>` played that long first, default 21:40 with the list untouched;
//!   `SHIFTELLA_STORY=<id>`: the vignette put on a
//!   stranger when none came by itself; `SHIFTELLA_LINE=n`: the storm talk's line)
//!   (`SHIFTELLA_EQUIP=prep|night|worn|closing`: the appliances' states)
//!   (hands-on scenes are played to their moment by the autoplayer;
//!   `SHIFTELLA_MG=froth|chocolate|raf|encore|latte|flat_white|pour|turek|night_train|tonic|
//!   tea|chai|linden|tatra|uzvar|rn333|plate|chlebicek|kolac|vera_kolac|medovnik|varenyky|
//!   pirohy|buchty|soup|kulajda|borscht|svarak` (or any recipe id `r_…`) picks the game,
//!   `SHIFTELLA_MG_T=<seconds>` plays it by autoplay that long first (the guide card shows on a
//!   fresh settings file; list the read ones in `minigame_guides` to skip it),
//!   `SHIFTELLA_MOMENT=<id>` the moment, `SHIFTELLA_PAGE` / `SHIFTELLA_OPEN` the journal page /
//!   the enlarged polaroid, `SHIFTELLA_AUTOFILL=1` auto-fills the rozpis first)
//! * `SHIFTELLA_AUTOPLAY=1|keys|skip` — Karolina plays the night by herself through the same
//!   intents as the mouse and keyboard (`keys`: mini-games by keyboard, `skip`: every
//!   mini-game skipped);
//!   `SHIFTELLA_SPEED=1|2|3` — the starting speed;
//!   `SHIFTELLA_SHOT_EVERY=n` — also save `<shot>_<frame>.png` every n frames;
//!   `SHIFTELLA_SKIP_SCENES=1` — story scenes finish at once
//! * `SHIFTELLA_LANG=cs|en|uk|ru`, `SHIFTELLA_NIGHT=n`, `SHIFTELLA_TIME=HH:MM`,
//!   `SHIFTELLA_T=<seconds>` (cutscene fast-forward), `SHIFTELLA_SETTINGS`, `SHIFTELLA_SAVES`.
//! * `SHIFTELLA_DIRT=0.4` — dirt blots on the floor; `SHIFTELLA_SETTINGS_TAB=0|1|2` — the
//!   settings scene opens on General / Accessibility / Controls (package J, access).
//! * `SHIFTELLA_COMBO=story_card,crew,photo,…` — opens several layers at once over the scene
//!   (QA overlap shots; see `GameScreen::combo_layers`).
//! * `SHIFTELLA_TEST_PANIC=1` — panic on purpose right after boot (checks the crash log,
//!   [`crate::crashlog`]; nothing is saved).

use std::rc::Rc;

use macroquad::prelude::*;

use crate::audio::{Audio, Track};
use crate::content::Content;
use crate::i18n::{I18n, Lang};
use crate::save::{SaveMeta, SaveStore, Slot};
use crate::screens::credits::Credits;
use crate::screens::game::world_view::{self as wv, sim};
use crate::screens::game::{Ctx, GameExit, GameScreen};
use crate::screens::menu::{MainMenu, MenuAction, MenuHost, MenuPage};
use crate::screens::{boot, game};
use crate::settings::{Locale, Settings, load_i18n};
use crate::ui::{Fonts, Ui};

/// Where the app is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Menu,
    Game,
    Credits,
    Quit,
}

/// What happened on the current screen.
#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    NewStory,
    NewSandbox,
    /// `true` if a save could be loaded.
    Continue(bool),
    Game(GameExit),
    CreditsDone,
    MenuQuit,
}

/// The transition table of the whole app.
pub fn route(from: Route, ev: &AppEvent) -> Route {
    match (from, ev) {
        (Route::Menu, AppEvent::NewStory | AppEvent::NewSandbox) => Route::Game,
        (Route::Menu, AppEvent::Continue(true)) => Route::Game,
        (Route::Menu, AppEvent::Continue(false)) => Route::Menu,
        (Route::Menu, AppEvent::MenuQuit) => Route::Quit,
        (Route::Game, AppEvent::Game(GameExit::MainMenu)) => Route::Menu,
        (Route::Game, AppEvent::Game(GameExit::Quit)) => Route::Quit,
        (Route::Game, AppEvent::Game(GameExit::Credits { .. })) => Route::Credits,
        (Route::Game, AppEvent::Game(GameExit::Load(_))) => Route::Game,
        (Route::Credits, AppEvent::CreditsDone) => Route::Menu,
        (r, _) => r,
    }
}

enum Screen {
    Menu(Box<MainMenu>),
    Game(Box<GameScreen>),
    Credits(Box<Credits>),
}

struct SaveHost {
    latest: Option<SaveMeta>,
}

impl MenuHost for SaveHost {
    fn has_save(&self) -> bool {
        self.latest.as_ref().is_some_and(|m| !m.finished)
    }
    fn continue_hint(&self, loc: &Locale) -> Option<String> {
        let m = self.latest.as_ref()?;
        let time = if m.clock == "day" {
            loc.tr_owned("ui.save.day")
        } else {
            m.clock.clone()
        };
        Some(loc.trf(
            "ui.menu.continue_hint",
            &[("night", &m.night.to_string()), ("time", &time)],
        ))
    }
}

pub struct App {
    pub content: Rc<Content>,
    pub i18n: I18n,
    pub settings: Settings,
    pub audio: Audio,
    pub store: SaveStore,
    pub ui: Ui,
    screen: Screen,
    host: SaveHost,
    /// The key bindings the i18n key tokens were last filled with (`sync_access`).
    keys_applied: crate::settings::keys::KeyBindings,
}

/// Text size, colour-blind markers and key bindings from the settings in force (the main
/// menu edits its own copy): applied every frame, the i18n key tokens only on a change.
fn sync_access(app: &mut App) {
    let s = match &app.screen {
        Screen::Menu(m) => &m.settings,
        _ => &app.settings,
    };
    s.apply_ui(&mut app.ui);
    if app.keys_applied != s.keys {
        app.i18n.apply_keys(&s.keys.badges());
        app.keys_applied = s.keys.clone();
    }
}

/// F3 / `SHIFTELLA_FPS=1`: frame rate, frame time (avg / max over the last second),
/// sim ticks and the CPU split of a game frame.
#[derive(Default)]
struct DebugOverlay {
    on: bool,
    /// (seconds since start, frame ms) of the last second.
    frames: std::collections::VecDeque<(f64, f32)>,
}

impl DebugOverlay {
    fn record(&mut self, now: f64, frame_ms: f32) {
        self.frames.push_back((now, frame_ms));
        while self.frames.front().is_some_and(|(t, _)| now - t > 1.0) {
            self.frames.pop_front();
        }
    }

    fn stats(&self) -> (f32, f32, f32) {
        let n = self.frames.len().max(1) as f32;
        let sum: f32 = self.frames.iter().map(|f| f.1).sum();
        let max = self.frames.iter().map(|f| f.1).fold(0.0, f32::max);
        let span = match (self.frames.front(), self.frames.back()) {
            (Some(a), Some(b)) if b.0 > a.0 => (b.0 - a.0) as f32,
            _ => 1.0,
        };
        let fps = if self.frames.len() > 1 {
            (self.frames.len() - 1) as f32 / span
        } else {
            0.0
        };
        (fps, sum / n, max)
    }

    fn draw(&self, ui: &mut Ui, times: Option<game::FrameTimes>) {
        if !self.on {
            return;
        }
        let (fps, avg, max) = self.stats();
        let mut lines = vec![
            format!("{fps:.0} fps"),
            format!("frame {avg:.1} ms avg · {max:.1} max"),
        ];
        if let Some(t) = times {
            lines.push(format!(
                "sim {:.1} · world {:.1} · ui {:.1} ms",
                t.sim_ms, t.world_ms, t.ui_ms
            ));
            lines.push(format!("ticks/frame {}", t.ticks));
        }
        let (sw, sh) = (screen_width(), screen_height());
        lines.push(format!(
            "{:.0}×{:.0} @{:.1}x",
            sw / screen_dpi_scale(),
            sh / screen_dpi_scale(),
            screen_dpi_scale()
        ));
        ui.set_camera();
        let st = ui.theme.small().size(11.0).color(ui.theme.text);
        let w = lines
            .iter()
            .map(|l| ui.fonts.measure(l, &st))
            .fold(0.0, f32::max)
            + 20.0;
        let r = Rect::new(8.0, 64.0, w, lines.len() as f32 * 15.0 + 12.0);
        crate::ui::draw_glass(&ui.theme, r, &crate::ui::Glass::tooltip());
        for (i, l) in lines.iter().enumerate() {
            let c = if i == 0 && fps < 55.0 {
                ui.theme.amber
            } else {
                ui.theme.text
            };
            ui.fonts
                .draw(l, r.x + 10.0, r.y + 18.0 + i as f32 * 15.0, &st.color(c));
        }
    }
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|s| !s.trim().is_empty())
}

/// `SHIFTELLA_TIME=HH:MM` as minutes since 22:00; 21:30–21:59 (the preparation half hour
/// before the doors open) are negative.
fn env_minute() -> Option<i32> {
    let t = env("SHIFTELLA_TIME")?;
    let (h, m) = t.split_once(':')?;
    let (h, m): (i32, i32) = (h.parse().ok()?, m.parse().ok()?);
    let since = ((h + 24 - 22) % 24) * 60 + m;
    Some(if since >= 24 * 60 - 30 {
        since - 24 * 60
    } else {
        since
    })
}

impl App {
    fn loc(&self) -> Locale<'_> {
        Locale::new(&self.i18n, self.settings.lang())
    }

    fn menu(&mut self, page: Option<MenuPage>) -> Screen {
        self.host.latest = self.store.latest().map(|(_, m)| m);
        let mut m = MainMenu::new(&self.content, self.settings.clone(), &self.ui);
        if let Some(p) = page {
            m.show(p);
        }
        self.audio.set_music(Some(Track::Menu));
        self.audio.set_rain(true);
        Screen::Menu(Box::new(m))
    }

    fn game(&mut self, mut g: GameScreen) -> Screen {
        g.player.apply_settings(&self.settings);
        // first day of a new game: offer the coach marks (screenshot runs only on request)
        if g.extra.tutorial.is_none() {
            let shot = env("SHIFTELLA_SHOT").is_some();
            let fresh = g.world.night() == 1 && g.world.phase() == sim::Phase::Day;
            if fresh
                && self.settings.show_tutorial
                && (!shot || env("SHIFTELLA_TUTORIAL").is_some())
            {
                g.start_tutorial();
            } else {
                g.extra.tutorial = Some(Default::default());
            }
        }
        if let Some(step) = env("SHIFTELLA_TUTORIAL").and_then(|s| s.parse::<u32>().ok())
            && let Some(p) = &mut g.extra.tutorial
        {
            p.active = true;
            p.step = step;
        }
        if let Some(a) = env("SHIFTELLA_AUTOPLAY") {
            g.hands.autoplay = Some(game::hands::Autoplay {
                cooldown: 0.5,
                keyboard: a == "keys",
                skip_minigames: a == "skip",
            });
        }
        g.hands.skip_scenes = env("SHIFTELLA_SKIP_SCENES").is_some();
        // SHIFTELLA_DIRT=0.4: dirt blots on the hall floor (colour-blind marker shots)
        if let Some(d) = env("SHIFTELLA_DIRT").and_then(|d| d.parse::<f32>().ok()) {
            g.world.dirt = d.clamp(0.0, 1.0);
        }
        if let Some(s) = env("SHIFTELLA_SPEED").and_then(|s| s.parse::<u8>().ok()) {
            g.set_speed(s);
        }
        // SHIFTELLA_COMBO=story_card,crew,…: several layers over the scene at once
        if let Some(list) = env("SHIFTELLA_COMBO") {
            g.combo_layers(&list);
        }
        Screen::Game(Box::new(g))
    }

    /// A game screen whose cutscene text appears at once in screenshot runs
    /// (set after [`App::game`], which applies the player's text speed).
    fn game_instant_text(&mut self, g: GameScreen, shot: bool) -> Screen {
        let mut s = self.game(g);
        if shot && let Screen::Game(g) = &mut s {
            g.player.set_text_speed(0.0);
        }
        s
    }

    fn load(&mut self, slot: Option<Slot>) -> Option<Screen> {
        let slot = slot.or_else(|| self.store.latest().map(|(s, _)| s))?;
        match self
            .store
            .read(slot)
            .and_then(|f| GameScreen::from_save(&self.content, &f))
        {
            Ok(g) => Some(self.game(g)),
            Err(e) => {
                eprintln!("warning: can't load {slot:?}: {e}");
                crate::crashlog::warn(&format!("can't load {slot:?}: {e}"));
                None
            }
        }
    }

    /// Builds the screen asked for by `SHIFTELLA_SCENE`.
    fn scene(&mut self, name: &str) -> Screen {
        let content = self.content.clone();
        let night = env("SHIFTELLA_NIGHT").and_then(|n| n.parse().ok());
        let minute = env_minute();
        let shot = env("SHIFTELLA_SHOT").is_some();
        let story_at = |night: u32, minute: Option<i32>, i18n: &I18n, lang: Lang| {
            let loc = Locale::new(i18n, lang);
            let mut g = GameScreen::new_story(&content, "cozy", true);
            g.fast_forward(&content, &loc, night, minute, night > 1);
            g
        };
        let lang = self.settings.lang();
        match name {
            "language" => self.menu(Some(MenuPage::Language)),
            "newgame" => self.menu(Some(MenuPage::NewGame)),
            "sandbox_setup" => self.menu(Some(MenuPage::Sandbox)),
            "settings" => {
                // SHIFTELLA_SETTINGS_TAB=0|1|2: General / Accessibility / Controls
                if let Some(t) = env("SHIFTELLA_SETTINGS_TAB").and_then(|t| t.parse().ok()) {
                    crate::settings::set_tab(t);
                }
                self.menu(Some(MenuPage::Settings))
            }
            "opening" => {
                let mut g = GameScreen::new_story(&content, "cozy", false);
                if let Some(t) = env("SHIFTELLA_T").and_then(|t| t.parse::<f32>().ok()) {
                    let loc = Locale::new(&self.i18n, lang);
                    g.player.fast_forward(&loc, t);
                }
                self.game_instant_text(g, shot)
            }
            "story_day1" => {
                let mut g = GameScreen::new_story(&content, "cozy", true);
                // SHIFTELLA_PANEL opens a panel over the first day (tutorial shots)
                g.panel = match env("SHIFTELLA_PANEL").as_deref() {
                    Some("crew") => Some(game::Panel::Crew),
                    Some("app") => Some(game::Panel::App),
                    Some("priorities" | "duties") => Some(game::Panel::Duties),
                    Some("build") => Some(game::Panel::Build),
                    Some("help") => Some(game::Panel::Help),
                    _ => None,
                };
                self.game(g)
            }
            "night" | "pause" | "crew" | "app" | "priorities" | "duties" | "log" | "build"
            | "build_drag" => {
                let drag = name == "build_drag";
                let name = if drag { "build" } else { name };
                let n = night.unwrap_or(if name == "night" || name == "pause" {
                    1
                } else {
                    3
                });
                let day = name == "build" && minute.is_none();
                let m = if day {
                    None
                } else {
                    Some(minute.unwrap_or(150))
                };
                let mut g = story_at(n, m, &self.i18n, lang);
                g.panel = match name {
                    "crew" => Some(game::Panel::Crew),
                    "app" => Some(game::Panel::App),
                    "priorities" | "duties" => Some(game::Panel::Duties),
                    "log" => Some(game::Panel::Log),
                    "build" => Some(game::Panel::Build),
                    _ => None,
                };
                if name == "build" {
                    // SHIFTELLA_BUILD_DEF / _AT=x,y / _TAB / _TURNS: a held piece over a tile;
                    // build_drag: the first round table (with its chairs) is being moved
                    let def = env("SHIFTELLA_BUILD_DEF").unwrap_or_else(|| "table_round".into());
                    let at = env("SHIFTELLA_BUILD_AT").and_then(|s| {
                        let (x, y) = s.split_once(',')?;
                        Some((x.parse::<f32>().ok()?, y.parse::<f32>().ok()?))
                    });
                    let st = &mut g.ui_state.build;
                    if let Some(tab) = env("SHIFTELLA_BUILD_TAB").and_then(|t| t.parse().ok()) {
                        st.tab = tab;
                    }
                    st.fake_mouse = at.map(|(x, y)| vec2(x + 0.5, y + 0.5) * crate::map::TILE);
                    if drag {
                        let table = g
                            .world
                            .furniture()
                            .iter()
                            .find(|p| p.def == "table_round")
                            .map(|p| p.uid);
                        if let Some(uid) = table {
                            g.ui_state.build.drag = Some(game::BuildDrag {
                                uid,
                                grab: (0, 0),
                                turns: env("SHIFTELLA_BUILD_TURNS")
                                    .and_then(|t| t.parse().ok())
                                    .unwrap_or(0),
                                sticky: true,
                            });
                        }
                    } else if def != "none" {
                        g.ui_state.build.selected = Some(def);
                    }
                }
                if let Some(tab) = env("SHIFTELLA_APP_TAB").and_then(|t| t.parse().ok()) {
                    g.ui_state.app_tab = tab;
                }
                if name == "pause" {
                    g.pause = Some(Default::default());
                }
                self.game(g)
            }
            // 0.2.2: a full house — every seat taken (the bar stools too), the overflow taking
            // it to go: paper cups over the heads by the register, out of the door in hand
            "to_go" => {
                let mut g = story_at(
                    night.unwrap_or(10),
                    Some(minute.unwrap_or(150)),
                    &self.i18n,
                    lang,
                );
                g.world.debug_spawn_guests(&content, "students", 44, 5);
                for _ in 0..(sim::TICKS_PER_MINUTE * 12) {
                    g.world.tick(&content);
                    sim::bot::finish_cutscenes(&mut g.world, &content);
                }
                self.game(g)
            }
            // v0.2 appliances: SHIFTELLA_EQUIP=prep (21:40, the default) | night | worn | closing
            "equipment" => {
                let loc = Locale::new(&self.i18n, lang);
                let variant = env("SHIFTELLA_EQUIP").unwrap_or_else(|| "prep".into());
                let n = night.unwrap_or(3);
                let m = minute.unwrap_or(match variant.as_str() {
                    "night" => 40,
                    "worn" => 60,
                    "closing" => 485,
                    _ => -20,
                });
                let mut g = GameScreen::new_story(&content, "cozy", true);
                g.fast_forward(&content, &loc, n, None, n > 1);
                // an oven for the medovník (bought in the day)
                if !g.world.furniture().iter().any(|p| p.def == "oven") {
                    'place: for y in [13, 14] {
                        for x in (22..28).rev() {
                            let cmd = sim::Command::PlaceFurniture {
                                def: "oven".into(),
                                x,
                                y,
                                rot: 2,
                            };
                            if g.world.apply(&content, cmd).is_ok() {
                                break 'place;
                            }
                        }
                    }
                }
                g.fast_forward(&content, &loc, n, Some(m), n > 1);
                g.stage_equipment(&content, &loc, &variant);
                self.game(g)
            }
            // the character card (C): Karolina's, a helper's (also Classic and by day), hiring
            // hands-ux shots: SHIFTELLA_HUX=checklist|rituals|queue|follow
            "hands_ux" => {
                let loc = Locale::new(&self.i18n, lang);
                let which = env("SHIFTELLA_HUX").unwrap_or_else(|| "queue".into());
                let early = matches!(which.as_str(), "checklist" | "rituals" | "gesture");
                let n = night.unwrap_or(if early { 1 } else { 8 });
                let m = minute.unwrap_or(if which == "gesture" {
                    -20
                } else if early {
                    2
                } else {
                    150
                });
                let mut g = GameScreen::new_story(&content, "cozy", true);
                g.fast_forward(&content, &loc, n, Some(m), n > 1);
                if which == "gesture" {
                    // SHIFTELLA_GESTURE=<checklist id | equip.<chore>>, SHIFTELLA_GESTURE_T=<s>
                    let key = env("SHIFTELLA_GESTURE").unwrap_or_else(|| "light_window".into());
                    let secs = env("SHIFTELLA_GESTURE_T")
                        .and_then(|t| t.parse::<f32>().ok())
                        .unwrap_or(0.6);
                    g.stage_ritual(&content, &loc, &key, secs);
                } else {
                    g.stage_hands_ux(&content, &which);
                }
                self.game(g)
            }
            "crew_karolina" | "crew_helper" | "crew_classic" | "crew_day" | "crew_hire" => {
                let loc = Locale::new(&self.i18n, lang);
                let diff = if name == "crew_classic" {
                    "classic"
                } else {
                    "cozy"
                };
                let n = night.unwrap_or(8);
                let m = (name != "crew_day").then(|| minute.unwrap_or(150));
                let mut g = GameScreen::new_story(&content, diff, true);
                g.fast_forward(&content, &loc, n, m, n > 1);
                g.open_crew_card(match name {
                    "crew_karolina" => "karolina",
                    "crew_hire" => "hire",
                    _ => "helper",
                });
                self.game(g)
            }
            "night1_handson" | "order_card" | "minigame" | "jukebox" | "book" => {
                let loc = Locale::new(&self.i18n, lang);
                let n = night.unwrap_or(if name == "jukebox" { 2 } else { 1 });
                let mut g = story_at(n, Some(0), &self.i18n, lang);
                let until = minute.unwrap_or(match name {
                    "night1_handson" => 95,
                    "jukebox" => 40,
                    _ => 75,
                });
                let mg = env("SHIFTELLA_MG");
                match name {
                    "order_card" => g.play_headless(&content, &loc, 900.0, &|g| {
                        g.world.minutes_since_open() >= until && g.hands.order.is_some()
                    }),
                    "minigame" => {
                        g.play_headless(&content, &loc, 900.0, &|g| {
                            g.world.minutes_since_open() >= until
                                && (g.hands.minigame.is_some() || g.hands.order.is_some())
                        });
                        // a chosen game for the guest on the card (or a seated guest)
                        let guest = g
                            .hands
                            .order
                            .as_ref()
                            .map(|o| o.guest)
                            .or_else(|| g.hands.minigame.as_ref().map(|m| m.guest));
                        let recipe = match mg.as_deref() {
                            Some("froth") => Some("r_cappuccino"),
                            Some("chocolate") => Some("r_hot_chocolate"),
                            Some("raf") => Some("r_raf"),
                            Some("encore") => Some("r_lori_encore"),
                            Some("latte") => Some("r_moon_latte"),
                            Some("flat_white") => Some("r_oat_flat_white"),
                            Some("pour") => Some("r_black"),
                            Some("turek") => Some("r_turek"),
                            Some("night_train") => Some("r_night_train"),
                            Some("tonic") => Some("r_espresso_tonic"),
                            Some("tea") => Some("r_lemon_tea"),
                            Some("chai") => Some("r_chai"),
                            Some("linden") => Some("r_linden_tea"),
                            Some("tatra") => Some("r_tatra_tea"),
                            Some("uzvar") => Some("r_uzvar"),
                            Some("rn333") => Some("r_rn333_tea"),
                            Some("plate") => Some("r_topinka"),
                            Some("chlebicek") => Some("r_chlebicek"),
                            Some("kolac") => Some("r_kolac"),
                            Some("vera_kolac") => Some("r_vera_kolac"),
                            Some("medovnik") => Some("r_medovnik"),
                            Some("varenyky") => Some("r_varenyky"),
                            Some("pirohy") => Some("r_pirohy"),
                            Some("buchty") => Some("r_buchty"),
                            Some("soup") => Some("r_garlic_soup"),
                            Some("kulajda") => Some("r_kulajda"),
                            Some("borscht") => Some("r_borscht"),
                            Some("svarak" | "mulled") => Some("r_svarak"),
                            Some(r) if r.starts_with("r_") => Some(r),
                            _ => None,
                        };
                        if let (Some(guest), Some(r)) = (guest, recipe) {
                            g.hands.order = None;
                            g.start_minigame(&content, guest, r, false);
                            // played a little by itself, for a mid-game picture
                            let secs: f32 = env("SHIFTELLA_MG_T")
                                .and_then(|v| v.parse().ok())
                                .unwrap_or(0.0);
                            if let Some(open) = g.hands.minigame.as_mut() {
                                let dt = 1.0 / 60.0;
                                for _ in 0..(secs / dt) as u32 {
                                    let i = open.game.autoplay_input();
                                    open.game.step(&i, dt);
                                    // stop on the result card (it shows the stars and why)
                                    if open.game.is_finished() {
                                        break;
                                    }
                                }
                            }
                        } else if g.hands.minigame.is_none()
                            && let Some(o) = g.hands.order.clone()
                            && let Some((r, _)) = g.world.best_matches(&content, o.guest).first()
                        {
                            let r = r.clone();
                            g.brew_chosen(
                                &content,
                                o.guest,
                                &r,
                                crate::ui::handson::BrewMode::ByHand,
                            );
                        }
                    }
                    _ => g.play_headless(&content, &loc, 900.0, &|g| {
                        g.world.minutes_since_open() >= until
                    }),
                }
                if name == "jukebox" {
                    g.hands.records_open = true;
                }
                if name == "book" {
                    g.open_book(None);
                }
                self.game(g)
            }
            // 0.2 E: strangers by description, then by name (an unknown one's card + bubble)
            "guest_names" => {
                let loc = Locale::new(&self.i18n, lang);
                let mut g = story_at(night.unwrap_or(8), Some(0), &self.i18n, lang);
                let until = minute.unwrap_or(150);
                // stop at the first moment past `until` with someone she doesn't know yet
                g.play_headless(&content, &loc, 1500.0, &move |g| {
                    g.world.minutes_since_open() >= until && !g.unknown_strangers().is_empty()
                });
                g.show_strangers(&content, &loc);
                self.game(g)
            }
            // 0.2 H: a stranger's little story (the marker, the card) and the storm night
            "story_marker" | "story_card" | "storm" | "storm_talk" => {
                let loc = Locale::new(&self.i18n, lang);
                let n = night.unwrap_or(if name.starts_with("storm") { 29 } else { 8 });
                let mut g = story_at(n, Some(0), &self.i18n, lang);
                game::stories_scene(&mut g, &content, &loc, name);
                self.game(g)
            }
            // T8: the world's systems, each played to its moment
            "moments" | "garlands" | "cat" => {
                let loc = Locale::new(&self.i18n, lang);
                let n = night.unwrap_or(match name {
                    "garlands" => 21,
                    "cat" => 12,
                    _ => 3,
                });
                let mut g = story_at(n, Some(minute.unwrap_or(5)), &self.i18n, lang);
                let want = env("SHIFTELLA_MOMENT");
                let want_c = want.clone();
                g.play_headless(&content, &loc, 1500.0, &move |g| {
                    g.world.active_moments().iter().any(|m| {
                        m.resolving.is_none()
                            && want_c.as_deref().is_none_or(|w| w == m.id)
                            && (name != "garlands" || m.id == "m_hang_garlands")
                            && (name != "cat" || m.id == "m_station_cat")
                    })
                });
                // no autoplayer on screen: the card stays open for the shot
                let id = g
                    .world
                    .active_moments()
                    .iter()
                    .find(|m| m.resolving.is_none())
                    .map(|m| m.id.clone());
                if let Some(id) = id
                    && env("SHIFTELLA_NO_CARD").is_none()
                {
                    g.open_moment(&id);
                }
                self.game(g)
            }
            "pantry" | "napkins" => {
                let n = night.unwrap_or(if name == "napkins" { 10 } else { 4 });
                let mut g = story_at(n, None, &self.i18n, lang);
                g.open_book(None);
                if let Some(b) = &mut g.hands.book {
                    b.state.tab = 3;
                    if name == "napkins" {
                        b.napkins = true;
                        b.state.tab = 0;
                    }
                }
                // one experiment on the counter, so the page shows a result
                if name == "pantry" {
                    let pantry: Vec<String> = g.world.pantry().iter().take(2).cloned().collect();
                    let _ = g.command(
                        &content,
                        sim::Command::TryExperiment {
                            ingredients: pantry,
                        },
                    );
                    g.drain_and_handle(&content, Some(&Locale::new(&self.i18n, lang)));
                }
                self.game(g)
            }
            "journal" => {
                let n = night.unwrap_or(12);
                let mut g = story_at(n, None, &self.i18n, lang);
                g.summary = None;
                g.panel = Some(game::Panel::Journal);
                if let Some(p) = env("SHIFTELLA_PAGE").and_then(|p| p.parse::<usize>().ok()) {
                    g.photos.journal.page = p;
                }
                if let Some(o) = env("SHIFTELLA_OPEN").and_then(|p| p.parse::<usize>().ok()) {
                    g.photos.journal.open = Some(o);
                }
                self.game(g)
            }
            "polaroid" => {
                let n = night.unwrap_or(5);
                let mut g = story_at(n, Some(479), &self.i18n, lang);
                g.run_to_morning(&content, &Locale::new(&self.i18n, lang));
                self.game(g)
            }
            "themes" => {
                let n = night.unwrap_or(22);
                let mut g = story_at(n, None, &self.i18n, lang);
                g.panel = Some(game::Panel::Build);
                g.ui_state.build.tab = game::BUILD_THEME_TAB;
                self.game(g)
            }
            "rozpis" | "payroll" => {
                let n = night.unwrap_or(8);
                let mut g = story_at(n, minute, &self.i18n, lang);
                g.summary = None;
                if name == "payroll" {
                    g.panel = Some(game::Panel::Payroll);
                } else {
                    g.panel = Some(game::Panel::App);
                    g.ui_state.app_tab = 0;
                    if env("SHIFTELLA_AUTOFILL").is_some() {
                        let _ = g.command(&content, sim::Command::AutoFillRozpis);
                    }
                }
                self.game(g)
            }
            // a plain day of night N (coach marks: `SHIFTELLA_TUTORIAL=4`)
            "day" => {
                let n = night.unwrap_or(2);
                let mut g = story_at(n, None, &self.i18n, lang);
                g.summary = None;
                self.game(g)
            }
            // package C: grandma's attic, the back room, requests, a corner, before / after
            "attic" | "backroom" | "requests" | "corner" | "before_after" => {
                let n = night.unwrap_or(match name {
                    "attic" => 13,
                    "backroom" => 16,
                    "requests" => 12,
                    _ => 9,
                });
                let loc = Locale::new(&self.i18n, lang);
                let mut g = story_at(n, None, &self.i18n, lang);
                game::build_content_scene(&mut g, &content, &loc, name, n);
                self.game(g)
            }
            // package I: the house drink and photo mode
            "signature_compose" | "signature_board" | "signature_order" | "photo_mode"
            | "photo_print" | "journal_photos" => {
                let loc = Locale::new(&self.i18n, lang);
                let mut g = game::signature_photo_scene(&content, &loc, name, night, minute);
                g.summary = None;
                self.game(g)
            }
            "season_snow" | "night27_snow" | "night22" | "night14" => {
                let n = night.unwrap_or(match name {
                    "night22" => 22,
                    "night14" => 14,
                    "night27_snow" => 27,
                    _ => 29,
                });
                let g = story_at(n, Some(minute.unwrap_or(150)), &self.i18n, lang);
                self.game(g)
            }
            "night9_anwar" | "anwar" => {
                let n = night.unwrap_or(9);
                let mut g = story_at(n, Some(minute.unwrap_or(85)), &self.i18n, lang);
                g.focus_regular(&content, &Locale::new(&self.i18n, lang), "guest_anwar");
                self.game(g)
            }
            // the walk home after the morning summary (design/v0.2.md F): SHIFTELLA_ROUTE picks
            // the way, SHIFTELLA_T jumps into the walk, SHIFTELLA_WEATHER=rain|fog|snow|clear
            "walk_home" | "walk_home_snow" | "walk_home_rain" | "backstep" | "backstep_snow" => {
                let loc = Locale::new(&self.i18n, lang);
                let weather = env("SHIFTELLA_WEATHER").and_then(|w| match w.as_str() {
                    "rain" => Some(crate::content::Weather::Rain),
                    "downpour" => Some(crate::content::Weather::Downpour),
                    "fog" => Some(crate::content::Weather::Fog),
                    "snow" => Some(crate::content::Weather::Snow),
                    "clear" => Some(crate::content::Weather::Clear),
                    _ => None,
                });
                let weather = weather.or(match name {
                    "walk_home_rain" => Some(crate::content::Weather::Rain),
                    "walk_home_snow" | "backstep_snow" => Some(crate::content::Weather::Snow),
                    _ => None,
                });
                let snowy = name.ends_with("_snow");
                let route = env("SHIFTELLA_ROUTE");
                let t = env("SHIFTELLA_T").and_then(|t| t.parse::<f32>().ok());
                let mut g = if name.starts_with("backstep") {
                    let n = night.unwrap_or(if snowy { 28 } else { 8 });
                    let mut g = story_at(n, Some(minute.unwrap_or(95)), &self.i18n, lang);
                    g.world.debug_end_rush();
                    game::walk_home::step_out(&mut g, &content, weather);
                    g
                } else {
                    let n = night.unwrap_or(if snowy { 27 } else { 4 });
                    let mut g = story_at(n, Some(479), &self.i18n, lang);
                    g.run_to_morning(&content, &loc);
                    game::walk_home::close_summary_with(&mut g, &content, weather);
                    g
                };
                if route.is_some() || t.is_some() {
                    game::walk_home::debug_seek(
                        &mut g,
                        &content,
                        route.as_deref(),
                        t.unwrap_or(0.0),
                    );
                }
                self.game(g)
            }
            "summary" => {
                let n = night.unwrap_or(1);
                let mut g = story_at(n, Some(479), &self.i18n, lang);
                g.run_to_morning(&content, &Locale::new(&self.i18n, lang));
                self.game(g)
            }
            "ending_light" | "ending_last_train" => {
                let id = name.trim_start_matches("ending_");
                let mut g = GameScreen::new_story(&content, "cozy", true);
                g.play_scene(&content, &format!("ending_{id}"));
                if let Some(t) = env("SHIFTELLA_T").and_then(|t| t.parse::<f32>().ok()) {
                    g.player.fast_forward(&Locale::new(&self.i18n, lang), t);
                }
                self.game_instant_text(g, shot)
            }
            "credits" => Screen::Credits(Box::new(Credits::new(
                &self.ui,
                Some("light".into()),
                Some("full".into()),
                true,
            ))),
            "sandbox" => {
                let mut g = GameScreen::new_sandbox(
                    &content,
                    sim::SandboxSetup {
                        seed: 2039,
                        ..Default::default()
                    },
                );
                g.fast_forward(
                    &content,
                    &Locale::new(&self.i18n, lang),
                    night.unwrap_or(1),
                    Some(minute.unwrap_or(120)),
                    false,
                );
                self.game(g)
            }
            _ => self.menu(Some(MenuPage::Main)),
        }
    }

    fn handle_menu(&mut self, a: MenuAction) -> Option<Route> {
        if let Screen::Menu(m) = &self.screen {
            self.settings = m.settings.clone();
        }
        self.audio
            .set_volumes(self.settings.music_volume, self.settings.sfx_volume);
        match a {
            MenuAction::StartStory { difficulty } => {
                let g = GameScreen::new_story(&self.content, &difficulty, false);
                self.screen = self.game(g);
                Some(route(Route::Menu, &AppEvent::NewStory))
            }
            MenuAction::StartSandbox(setup) => {
                let mut g = GameScreen::new_sandbox(&self.content, wv::sandbox_setup(&setup));
                g.extra.opening_seen = Some(Vec::new());
                self.screen = self.game(g);
                Some(route(Route::Menu, &AppEvent::NewSandbox))
            }
            MenuAction::Continue => {
                let ok = match self.load(None) {
                    Some(s) => {
                        self.screen = s;
                        true
                    }
                    None => false,
                };
                Some(route(Route::Menu, &AppEvent::Continue(ok)))
            }
            MenuAction::SettingsChanged => None,
            MenuAction::Quit => Some(Route::Quit),
        }
    }

    fn handle_game_exit(&mut self, e: GameExit) -> Route {
        let r = route(Route::Game, &AppEvent::Game(e.clone()));
        match e {
            GameExit::MainMenu => self.screen = self.menu(Some(MenuPage::Main)),
            GameExit::Quit => {}
            GameExit::Load(slot) => {
                if let Some(s) = self.load(Some(slot)) {
                    self.screen = s;
                }
            }
            GameExit::Credits { ending, variant } => {
                let epilogue = match &self.screen {
                    Screen::Game(g) => g.world.story().played.contains("answer"),
                    _ => false,
                };
                // the finished story stays loadable but no longer offers Continue
                if let Screen::Game(g) = &self.screen
                    && std::env::var("SHIFTELLA_NO_AUTOSAVE").is_err()
                {
                    let _ = g.save(&self.content, &self.store, Slot::Auto);
                }
                self.audio.set_music(Some(Track::Calm));
                self.screen =
                    Screen::Credits(Box::new(Credits::new(&self.ui, ending, variant, epilogue)));
            }
        }
        r
    }
}

/// Tells the crash log what is on screen (it writes a line only when that changes).
fn log_scene(app: &App) {
    crate::crashlog::set_lang(app.settings.lang());
    let (scene, clock) = match &app.screen {
        Screen::Menu(_) => ("menu".to_string(), None),
        Screen::Credits(_) => ("credits".to_string(), None),
        Screen::Game(g) => {
            let w = &g.world;
            let n = w.night();
            let place = if g.cutscene_playing() {
                format!("cutscene {} · night {n}", g.player.current_id())
            } else if w.ending().is_some() {
                format!("ending · night {n}")
            } else if w.phase() == sim::Phase::Day {
                format!("day {n}")
            } else if w.in_prep() {
                format!("preparation · night {n}")
            } else {
                format!("night {n}")
            };
            let clock = (w.phase() == sim::Phase::Night).then(|| w.minutes_since_open());
            (place, clock)
        }
    };
    crate::crashlog::set_scene(&scene);
    crate::crashlog::set_clock(clock);
}

/// Opens the window's game: boot, then the state machine until Quit.
pub async fn run() {
    let shot = env("SHIFTELLA_SHOT");
    let frames: u32 = env("SHIFTELLA_FRAMES")
        .and_then(|f| f.parse().ok())
        .unwrap_or(45);
    if shot.is_some() && std::env::var("SHIFTELLA_SAVES").is_err() {
        let dir = std::env::temp_dir().join("shiftella-shot-saves");
        // SAFETY: single-threaded, before anything reads the environment.
        unsafe { std::env::set_var("SHIFTELLA_SAVES", dir) };
    }
    if shot.is_some() {
        // SAFETY: as above.
        unsafe { std::env::set_var("SHIFTELLA_NO_AUTOSAVE", "1") };
    }
    // the crash log next to the saves (after SHIFTELLA_SAVES is final)
    crate::crashlog::init();
    crate::crashlog::info("startup");
    crate::profile::init();
    let boot_clock = std::time::Instant::now();

    // ── boot ──
    clear_background(crate::render::colors::NIGHT_0);
    next_frame().await;
    let fonts = match Fonts::load().await {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: fonts: {e}");
            crate::crashlog::error(&format!("fonts: {e}"));
            return;
        }
    };
    crate::render::init_brand(fonts.face(crate::ui::Weight::Bold));
    let mut ui = Ui::new(fonts);
    if shot.is_some() {
        // the game already steps 1/60 s a frame in screenshot runs; the UI's clock too
        ui.set_fixed_dt(Some(1.0 / 60.0));
        // and the desktop cursor stays out of the invisible window (no stray hovers)
        crate::platform::park_pointer();
    }
    let mut settings = Settings::load();
    if let Some(l) = env("SHIFTELLA_LANG").and_then(|c| Lang::from_code(&c)) {
        settings.language = Some(l);
    }
    ui.set_scale(settings.ui_scale);
    settings.apply_window();
    let i18n = match load_i18n("assets/i18n") {
        Ok(i) => i,
        Err(e) => {
            eprintln!("error: i18n: {e}");
            crate::crashlog::error(&format!("i18n: {e}"));
            return;
        }
    };
    let loading = |i18n: &I18n, s: &Settings| {
        i18n.tr(s.language.unwrap_or(Lang::En), "ui.boot.loading")
            .to_string()
    };
    let label = loading(&i18n, &settings);
    let mut t = 0.0;
    let step = |ui: &mut Ui, p: f32, t: &mut f32| {
        ui.begin_frame();
        *t += ui.dt();
        boot::draw(ui, p, &label, *t);
        ui.end_frame();
    };
    step(&mut ui, 0.05, &mut t);
    next_frame().await;
    let content = match Content::load("assets/data") {
        Ok(c) => Rc::new(c),
        Err(e) => {
            eprintln!("error: content: {e}");
            crate::crashlog::error(&format!("content: {e}"));
            return;
        }
    };
    step(&mut ui, 0.2, &mut t);
    next_frame().await;
    let mut audio = Audio::silent();
    let mut silent = shot.is_some() || env("SHIFTELLA_MUTE").is_some();
    if !silent && !crate::audio::audio_available() {
        eprintln!("info: assets/audio not found — playing without sound");
        silent = true;
    } else if !silent && !crate::audio::generated_pack_available() {
        eprintln!("info: no generated music / sound pack — playing the CC0 sounds only");
    }
    if !silent {
        for i in 0..Audio::FILES {
            audio.load_step(i).await;
            step(
                &mut ui,
                0.2 + 0.8 * (i + 1) as f32 / Audio::FILES as f32,
                &mut t,
            );
            next_frame().await;
        }
    }
    audio.set_volumes(settings.music_volume, settings.sfx_volume);
    // an invisible automation run is silent too (the audio is still loaded and mixed)
    if crate::platform::automation_hidden() && !audio.mix.muted {
        audio.toggle_mute();
    }

    let placeholder = MainMenu::new(&content, settings.clone(), &ui);
    let mut app = App {
        content,
        i18n,
        settings,
        audio,
        store: SaveStore::default(),
        ui,
        screen: Screen::Menu(Box::new(placeholder)),
        host: SaveHost { latest: None },
        keys_applied: Default::default(),
    };
    app.screen = match env("SHIFTELLA_SCENE") {
        Some(s) => {
            if s != "language" && app.settings.language.is_none() {
                app.settings.language = Some(Lang::Cs);
            }
            app.scene(&s)
        }
        None => app.menu(None),
    };
    crate::crashlog::info("boot done");
    if crate::profile::on() {
        eprintln!(
            "profile: boot {:.0} ms (fonts, i18n, content, audio, first scene)",
            boot_clock.elapsed().as_secs_f64() * 1000.0
        );
    }
    log_scene(&app);
    crate::crashlog::test_panic_if_asked();

    let mut frame = 0u32;
    let mut route_now = Route::Menu;
    let mut debug = DebugOverlay {
        on: env("SHIFTELLA_FPS").is_some_and(|v| v != "0"),
        ..Default::default()
    };
    let clock = std::time::Instant::now();
    let mut last_frame = clock;
    loop {
        let now = std::time::Instant::now();
        debug.record(
            (now - clock).as_secs_f64(),
            (now - last_frame).as_secs_f32() * 1000.0,
        );
        last_frame = now;
        let p_frame = crate::profile::span(crate::profile::Sec::Frame);
        if is_key_pressed(KeyCode::F3) {
            debug.on = !debug.on;
        }
        log_scene(&app);
        sync_access(&mut app);
        app.ui.begin_frame();
        let dt = if shot.is_some() {
            1.0 / 60.0
        } else {
            app.ui.dt()
        };
        let mut ducked = false;
        let mut next: Option<Route> = None;
        match &mut app.screen {
            Screen::Menu(m) => {
                m.update(dt);
                let action = m.draw(&mut app.ui, &app.i18n, &app.host);
                let reqs = m.drain_audio_requests();
                for r in reqs {
                    app.audio.request(&r);
                }
                app.ui.end_frame();
                if let Some(a) = action {
                    next = app.handle_menu(a);
                }
            }
            Screen::Game(g) => {
                // the café's own drink: its name wherever a recipe name shows
                game::signature_ui::sync_i18n(&mut app.i18n, &g.world, &app.content);
                let lang = app.settings.lang();
                let loc = Locale::new(&app.i18n, lang);
                let mut cx = Ctx {
                    content: &app.content,
                    loc,
                    settings: &mut app.settings,
                    audio: &mut app.audio,
                    store: &app.store,
                };
                g.frame(&mut app.ui, &mut cx, dt);
                ducked = g.dialogue_on_screen();
                app.ui.end_frame();
                g.hotkeys(&mut app.ui, &mut cx);
                if let Some(e) = g.exit() {
                    next = Some(app.handle_game_exit(e));
                }
            }
            Screen::Credits(c) => {
                let loc = Locale::new(&app.i18n, app.settings.lang());
                let done = c.frame(&mut app.ui, &loc, dt);
                app.ui.end_frame();
                if done {
                    app.screen = app.menu(Some(MenuPage::Main));
                    next = Some(route(Route::Credits, &AppEvent::CreditsDone));
                }
            }
        }
        let times = match &app.screen {
            Screen::Game(g) => Some(g.times),
            _ => None,
        };
        debug.draw(&mut app.ui, times);
        for s in app.ui.drain_sounds() {
            app.audio.ui_sound(s);
        }
        app.audio.update(dt, ducked);
        if let Some(r) = next {
            route_now = r;
        }
        if route_now == Route::Quit {
            if let Screen::Game(g) = &app.screen
                && g.world.ending().is_none()
                && std::env::var("SHIFTELLA_NO_AUTOSAVE").is_err()
            {
                let _ = g.save(&app.content, &app.store, Slot::Auto);
            }
            crate::crashlog::info("quit");
            break;
        }
        drop(p_frame);
        frame += 1;
        // SHIFTELLA_SHOT_EVERY=n: a numbered screenshot every n frames on the way (smoke runs)
        if let Some(path) = &shot
            && let Some(every) = env("SHIFTELLA_SHOT_EVERY").and_then(|n| n.parse::<u32>().ok())
            && every > 0
            && frame.is_multiple_of(every)
            && frame < frames
        {
            let mut img = get_screen_data();
            for px in img.bytes.as_chunks_mut::<4>().0 {
                px[3] = 255;
            }
            let stem = path.trim_end_matches(".png");
            img.export_png(&format!("{stem}_{frame:05}.png"));
        }
        if let Some(path) = &shot
            && frame >= frames
        {
            let mut img = get_screen_data();
            for px in img.bytes.as_chunks_mut::<4>().0 {
                px[3] = 255;
            }
            img.export_png(path);
            println!("saved {path}");
            crate::profile::frame_end(&profile_label(&app));
            if debug.on {
                let (fps, avg, max) = debug.stats();
                let t = times.unwrap_or_default();
                println!(
                    "perf: {fps:.1} fps, frame {avg:.2} ms avg / {max:.2} max, sim {:.2} world {:.2} ui {:.2} ms, {} ticks",
                    t.sim_ms, t.world_ms, t.ui_ms, t.ticks
                );
            }
            break;
        }
        {
            let _p = crate::profile::span(crate::profile::Sec::Present);
            next_frame().await;
        }
        if crate::profile::on() {
            crate::profile::frame_end(&profile_label(&app));
        }
    }
    crate::profile::finish();
    let _ = app.loc();
}

/// A short label of what is on screen for the profiler's CSV (`SHIFTELLA_PROFILE`).
fn profile_label(app: &App) -> String {
    match &app.screen {
        Screen::Menu(_) => "menu".into(),
        Screen::Credits(_) => "credits".into(),
        Screen::Game(g) => {
            let w = &g.world;
            let what = if g.cutscene_playing() {
                "cutscene".to_string()
            } else if g.walk.is_open() {
                "walk".to_string()
            } else if g.summary.is_some() {
                "summary".to_string()
            } else if let Some(p) = g.panel {
                format!("{p:?}").to_lowercase()
            } else if w.phase() == sim::Phase::Day {
                "day".to_string()
            } else {
                "night".to_string()
            };
            format!("n{} {what} x{}", w.night(), g.speed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes() {
        assert_eq!(route(Route::Menu, &AppEvent::NewStory), Route::Game);
        assert_eq!(route(Route::Menu, &AppEvent::Continue(false)), Route::Menu);
        assert_eq!(route(Route::Menu, &AppEvent::MenuQuit), Route::Quit);
        assert_eq!(
            route(
                Route::Game,
                &AppEvent::Game(GameExit::Credits {
                    ending: Some("light".into()),
                    variant: None
                })
            ),
            Route::Credits
        );
        assert_eq!(route(Route::Credits, &AppEvent::CreditsDone), Route::Menu);
        assert_eq!(
            route(Route::Game, &AppEvent::Game(GameExit::MainMenu)),
            Route::Menu
        );
        assert_eq!(
            route(Route::Game, &AppEvent::Game(GameExit::Quit)),
            Route::Quit
        );
    }

    #[test]
    fn env_time_parses_night_minutes() {
        // SAFETY: test-local variable, read right after.
        unsafe { std::env::set_var("SHIFTELLA_TIME", "03:33") };
        assert_eq!(env_minute(), Some(5 * 60 + 33));
        // the preparation half hour before the doors open
        unsafe { std::env::set_var("SHIFTELLA_TIME", "21:40") };
        assert_eq!(env_minute(), Some(-20));
        unsafe { std::env::remove_var("SHIFTELLA_TIME") };
    }
}
