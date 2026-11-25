//! The app state machine: Boot → (language picker) → Main menu → New story / Sandbox /
//! Continue → Game (opening, days, nights, cutscenes) → Ending → Credits → Main menu.
//!
//! [`route`] is the pure transition table (tested headless); [`run`] drives it with a window.
//!
//! Test automation (env):
//! * `SHIFTELLA_SHOT=path.png` — render `SHIFTELLA_FRAMES` (default 45) frames, save, exit
//! * `SHIFTELLA_SCENE=menu|language|opening|story_day1|night|night9_anwar|build|crew|app|
//!   priorities|log|summary|pause|ending_light|ending_last_train|credits|sandbox`
//! * `SHIFTELLA_LANG=cs|en|sk|uk`, `SHIFTELLA_NIGHT=n`, `SHIFTELLA_TIME=HH:MM`,
//!   `SHIFTELLA_T=<seconds>` (cutscene fast-forward), `SHIFTELLA_SETTINGS`, `SHIFTELLA_SAVES`.

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

fn env_minute() -> Option<u32> {
    let t = env("SHIFTELLA_TIME")?;
    let (h, m) = t.split_once(':')?;
    let (h, m): (u32, u32) = (h.parse().ok()?, m.parse().ok()?);
    Some(((h + 24 - 22) % 24) * 60 + m)
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
        let story_at = |night: u32, minute: Option<u32>, i18n: &I18n, lang: Lang| {
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
            "opening" => {
                let mut g = GameScreen::new_story(&content, "cozy", false);
                if let Some(t) = env("SHIFTELLA_T").and_then(|t| t.parse::<f32>().ok()) {
                    let loc = Locale::new(&self.i18n, lang);
                    g.player.fast_forward(&loc, t);
                }
                self.game_instant_text(g, shot)
            }
            "story_day1" => {
                let g = GameScreen::new_story(&content, "cozy", true);
                self.game(g)
            }
            "night" | "pause" | "crew" | "app" | "priorities" | "log" | "build" => {
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
                    "priorities" => Some(game::Panel::Priorities),
                    "log" => Some(game::Panel::Log),
                    "build" => Some(game::Panel::Build),
                    _ => None,
                };
                if name == "build" {
                    g.ui_state.build.selected = Some("table_round".into());
                }
                if name == "pause" {
                    g.pause = Some(Default::default());
                }
                self.game(g)
            }
            "night9_anwar" | "anwar" => {
                let n = night.unwrap_or(9);
                let mut g = story_at(n, Some(minute.unwrap_or(85)), &self.i18n, lang);
                g.focus_regular(&content, &Locale::new(&self.i18n, lang), "guest_anwar");
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

    // ── boot ──
    clear_background(crate::render::colors::NIGHT_0);
    next_frame().await;
    let fonts = match Fonts::load().await {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: fonts: {e}");
            return;
        }
    };
    let mut ui = Ui::new(fonts);
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
        if is_key_pressed(KeyCode::F3) {
            debug.on = !debug.on;
        }
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
                ducked = g.cutscene_playing();
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
            break;
        }
        frame += 1;
        if let Some(path) = &shot
            && frame >= frames
        {
            let mut img = get_screen_data();
            for px in img.bytes.as_chunks_mut::<4>().0 {
                px[3] = 255;
            }
            img.export_png(path);
            println!("saved {path}");
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
        next_frame().await;
    }
    let _ = app.loc();
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
        unsafe { std::env::remove_var("SHIFTELLA_TIME") };
    }
}
