//! The app side of the developer build (`--features dev`, `crate::dev`): the panel's keys,
//! drawing it over every screen, and carrying out what it asks for.
//!
//! Screenshot hooks (dev build only): `SHIFTELLA_DEV=jump|screens|scenes|lang|cheats|overlay`
//! opens the panel on that section, `SHIFTELLA_DEV_FILTER=<text>` types into its search field,
//! `SHIFTELLA_DEV_JUMP=<night>` starts a Cozy jump at boot (its progress card),
//! `SHIFTELLA_DEV_OVERLAYS=nav,seats,regions,layout,fps` switches debug drawing on,
//! `SHIFTELLA_DEV_OPEN=<Screens label>` opens that screen as its button does,
//! `SHIFTELLA_DEV_CUTSCENE=<id>` plays that scene as the Scenes section's Play does.

use std::time::Duration;

use super::*;
use crate::dev::jump::{Jump, Start};
use crate::dev::panel::{self, View};
use crate::dev::{DevAction, DevPanel, catalog, cheats};

/// The panel, a running jump and an action waiting one frame (its "Opening…" card shows).
pub(super) struct DevLayer {
    pub panel: DevPanel,
    jump: Option<Jump>,
    pending: Option<DevAction>,
    fps_prev: bool,
    layout_prev: Option<bool>,
}

impl DevLayer {
    pub fn new() -> DevLayer {
        let mut panel = DevPanel::default();
        if let Some(s) = env("SHIFTELLA_DEV") {
            panel.open = true;
            panel.section = panel::SECTIONS
                .iter()
                .position(|n| n.eq_ignore_ascii_case(&s) || (s == "cutscenes" && *n == "Scenes"))
                .unwrap_or(0);
        }
        if let Some(f) = env("SHIFTELLA_DEV_FILTER") {
            match panel.section {
                panel::SCENES => panel.scene_filter = f,
                panel::CHEATS => panel.event_filter = f,
                _ => panel.filter = f,
            }
        }
        for o in env("SHIFTELLA_DEV_OVERLAYS").unwrap_or_default().split(',') {
            let ov = &mut panel.overlays;
            match o.trim() {
                "nav" => ov.nav = true,
                "seats" => ov.seats = true,
                "regions" => ov.regions = true,
                "layout" => ov.layout = true,
                "fps" => ov.fps = true,
                _ => {}
            }
        }
        let pending = if let Some(label) = env("SHIFTELLA_DEV_OPEN") {
            catalog::all()
                .iter()
                .position(|o| o.label.eq_ignore_ascii_case(&label))
                .map(DevAction::Open)
        } else {
            env("SHIFTELLA_DEV_CUTSCENE").map(|id| DevAction::Cutscene {
                id,
                choice: false,
                fresh: true,
            })
        };
        DevLayer {
            panel,
            jump: None,
            pending,
            fps_prev: false,
            layout_prev: None,
        }
    }

    /// Before the screen's frame: ` / F1 toggle the panel (F1 is taken from the game's Help
    /// in the dev build; ` only while no text field has focus).
    pub fn begin(&mut self, ui: &mut Ui, fps_on: &mut bool) {
        let f1 = is_key_pressed(KeyCode::F1);
        if f1 {
            ui.consume_key(KeyCode::F1);
        }
        if f1 || ui.take_key(KeyCode::GraveAccent) {
            self.panel.toggle();
        }
        // F3 and the Overlay toggle drive the same FPS overlay
        if self.panel.overlays.fps != self.fps_prev {
            *fps_on = self.panel.overlays.fps;
        } else {
            self.panel.overlays.fps = *fps_on;
        }
        self.fps_prev = self.panel.overlays.fps;
        match self.layout_prev {
            None => {
                self.panel.overlays.layout |= ui.dev_layout_debug();
                ui.dev_set_layout_debug(self.panel.overlays.layout);
            }
            Some(p) if p != self.panel.overlays.layout => {
                ui.dev_set_layout_debug(self.panel.overlays.layout);
            }
            _ => {}
        }
        self.layout_prev = Some(self.panel.overlays.layout);
    }

    pub fn jumping(&self) -> Option<(u32, u32, f32)> {
        self.jump
            .as_ref()
            .map(|j| (j.night(), j.target, j.progress()))
    }
}

/// Everything the dev build draws over a screen, inside its frame (before `end_frame`, so the
/// panel's widgets get their clicks): overlays, the menu's Developer button, the panel, the
/// jump's progress, the badge.
pub(super) fn draw(
    dev: &mut DevLayer,
    ui: &mut Ui,
    content: &Content,
    game: Option<&GameScreen>,
    main_menu: bool,
    lang: Lang,
) -> Option<DevAction> {
    if let Some(g) = game {
        crate::dev::overlays::draw_world(g, content, &dev.panel.overlays);
    }
    if dev.panel.overlays.regions {
        crate::dev::overlays::draw_regions(ui);
    }
    if main_menu && !dev.panel.open && panel::menu_button(ui) {
        dev.panel.open = true;
    }
    let jumping = dev.jumping();
    let act = dev.panel.draw(
        ui,
        &View {
            content,
            game,
            lang,
            jumping,
        },
    );
    if let Some((n, target, p)) = jumping {
        panel::draw_progress(ui, n, target, p);
    }
    if let Some(a) = &dev.pending {
        let what = match a {
            DevAction::Open(i) => format!("Opening {}…", catalog::all()[*i].label),
            DevAction::Cutscene { id, .. } => format!("Setting up {id}…"),
            _ => "Working…".into(),
        };
        panel::draw_busy(ui, &what);
    }
    panel::draw_badge(ui);
    act
}

/// The route a screen belongs to.
fn route_of(s: &Screen) -> Route {
    match s {
        Screen::Menu(_) => Route::Menu,
        Screen::Game(_) => Route::Game,
        Screen::Credits(_) => Route::Credits,
    }
}

impl App {
    fn builder(&self) -> SceneBuilder<'_> {
        SceneBuilder {
            content: self.content.clone(),
            i18n: &self.i18n,
            settings: &self.settings,
        }
    }

    /// Runs a panel action (heavy ones wait a frame: [`App::dev_tick`]).
    pub(super) fn dev_apply(&mut self, dev: &mut DevLayer, a: DevAction) -> Option<Route> {
        let now = self.ui.time();
        match a {
            DevAction::Story {
                difficulty,
                seed,
                night,
                hire,
            } => {
                let start = Start::Story {
                    difficulty: difficulty.into(),
                    seed,
                };
                dev.jump = Some(Jump::new(&self.content, &start, night, hire));
                None
            }
            DevAction::Sandbox {
                difficulty,
                seed,
                night,
                challenge,
            } => {
                let start = Start::Sandbox {
                    difficulty: difficulty.into(),
                    seed,
                    challenge,
                };
                dev.jump = Some(Jump::new(&self.content, &start, night, true));
                None
            }
            DevAction::ContinueTo { night, hire } => {
                if let Screen::Game(g) = &self.screen {
                    dev.jump = Some(Jump::from_world(
                        g.world.clone(),
                        g.extra.clone(),
                        night,
                        hire,
                    ));
                }
                None
            }
            DevAction::Open(_) | DevAction::Cutscene { fresh: true, .. } => {
                dev.pending = Some(a);
                None
            }
            DevAction::Cutscene {
                id,
                choice,
                fresh: false,
            } if id != "opening" => {
                let loc = Locale::new(&self.i18n, self.settings.lang());
                let content = self.content.clone();
                if let Screen::Game(g) = &mut self.screen {
                    if choice {
                        if !g.stage_cutscene_choice(&content, &loc, &id) {
                            dev.panel.say(format!("{id}: no choice reached"), now);
                        }
                    } else {
                        g.stage_cutscene_at(&content, &loc, &id, 0.0);
                    }
                    dev.panel.open = false;
                } else {
                    dev.pending = Some(DevAction::Cutscene {
                        id,
                        choice,
                        fresh: true,
                    });
                }
                None
            }
            // (the opening only plays as the opening scene)
            DevAction::Cutscene { .. } => {
                dev.pending = Some(a);
                None
            }
            DevAction::Lang(l) => {
                self.settings.language = Some(l);
                if let Screen::Menu(m) = &mut self.screen {
                    m.settings.language = Some(l);
                }
                dev.panel.say(format!("language {}", l.code()), now);
                None
            }
            DevAction::Cheat(c) => {
                let loc = Locale::new(&self.i18n, self.settings.lang());
                let content = self.content.clone();
                if let Screen::Game(g) = &mut self.screen {
                    let msg = cheats::apply(g, &content, &loc, &c);
                    dev.panel.say(msg, now);
                }
                None
            }
            DevAction::MainMenu => {
                self.screen = self.menu(Some(MenuPage::Main));
                Some(Route::Menu)
            }
        }
    }

    /// Once a frame after the screen: plays the jump on, runs a waiting action.
    pub(super) fn dev_tick(&mut self, dev: &mut DevLayer) -> Option<Route> {
        let now = self.ui.time();
        if let Some(j) = &mut dev.jump
            && j.step(&self.content, Duration::from_millis(12))
        {
            let j = dev.jump.take()?;
            let target = j.target;
            return match j.finish(&self.content) {
                Ok(g) => {
                    let n = g.world.night();
                    self.screen = self.game(g);
                    dev.panel.say(format!("night {n} (asked {target})"), now);
                    dev.panel.open = false;
                    Some(Route::Game)
                }
                Err(e) => {
                    dev.panel.say(format!("jump failed: {e}"), now);
                    None
                }
            };
        }
        let a = dev.pending.take()?;
        let (scene, opts): (String, Vec<(String, String)>) = match &a {
            DevAction::Open(i) => {
                let o = catalog::all()[*i];
                (
                    o.scene.to_string(),
                    o.opts
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect(),
                )
            }
            DevAction::Cutscene { id, choice, .. } => {
                crate::dev::cutscene_scene(&self.content, id, *choice)
            }
            _ => return None,
        };
        let o: Vec<(&str, &str)> = opts.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let built = crate::dev::opts::with(&o, || self.builder().scene(&scene));
        self.screen = crate::dev::opts::with(&o, || self.realize(built));
        let label = match &a {
            DevAction::Open(i) => catalog::all()[*i].label.to_string(),
            DevAction::Cutscene { id, .. } => id.clone(),
            _ => String::new(),
        };
        dev.panel.say(format!("opened {label}"), now);
        dev.panel.open = false;
        Some(route_of(&self.screen))
    }
}

/// `SHIFTELLA_DEV_JUMP=<night>`: a Cozy story jump started at boot (screenshots of the
/// progress card).
pub(super) fn boot_jump(dev: &mut DevLayer, content: &Rc<Content>) {
    if let Some(n) = env("SHIFTELLA_DEV_JUMP").and_then(|n| n.parse().ok()) {
        let start = Start::Story {
            difficulty: "cozy".into(),
            seed: crate::dev::jump::STORY_SEED,
        };
        dev.jump = Some(Jump::new(content, &start, n, true));
    }
}
