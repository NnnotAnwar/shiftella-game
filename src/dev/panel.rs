//! The dev panel: a compact glass column on the right over any screen, toggled with ` or F1.
//! Its texts are English only — it is a developer tool, never shown to players (the release
//! build has none of this module).

use macroquad::prelude::*;

use super::catalog;
use super::cheats::{self, Cheat};
use crate::content::{Content, CutsceneDef, CutsceneStep, CutsceneTrigger};
use crate::i18n::Lang;
use crate::screens::game::GameScreen;
use crate::sim::{Mode, Phase};
use crate::ui::draw::{fill_rrect, stroke_rrect};
use crate::ui::widgets::paint_surface;
use crate::ui::{Align, ButtonKind, Glass, RectExt, TextStyle, Ui, draw_glass};

/// The panel's sections, in tab order.
pub const SECTIONS: [&str; 6] = ["Jump", "Screens", "Scenes", "Lang", "Cheats", "Overlay"];

pub const JUMP: usize = 0;
pub const SCREENS: usize = 1;
pub const SCENES: usize = 2;
pub const LANG: usize = 3;
pub const CHEATS: usize = 4;
pub const OVERLAY: usize = 5;

/// Difficulty ids of the Jump section (Carefree is `relaxed` in the data).
pub const DIFFICULTIES: [(&str, &str); 3] = [
    ("Cozy", "cozy"),
    ("Carefree", "relaxed"),
    ("Classic", "classic"),
];

/// What the panel asks the app to do.
#[derive(Debug, Clone, PartialEq)]
pub enum DevAction {
    /// A new story played by the bot to night `night`'s day.
    Story {
        difficulty: &'static str,
        seed: u64,
        night: u32,
        hire: bool,
    },
    /// The running story played on to night `night`'s day.
    ContinueTo {
        night: u32,
        hire: bool,
    },
    /// A new sandbox run (Endless / Challenge) played to night `night`.
    Sandbox {
        difficulty: &'static str,
        seed: u64,
        night: u32,
        challenge: bool,
    },
    /// Opens [`catalog::all`]`()[i]`.
    Open(usize),
    /// Plays a cutscene / talk: over the running café, or `fresh` at its own night; `choice`
    /// stops at its choice.
    Cutscene {
        id: String,
        choice: bool,
        fresh: bool,
    },
    Lang(Lang),
    Cheat(Cheat),
    MainMenu,
}

/// Debug drawing switched in the Overlay section.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Overlays {
    pub fps: bool,
    pub layout: bool,
    pub regions: bool,
    pub nav: bool,
    pub seats: bool,
}

/// What the panel shows about the app this frame.
pub struct View<'a> {
    pub content: &'a Content,
    pub game: Option<&'a GameScreen>,
    pub lang: Lang,
    /// A jump in progress: (night now, target, progress 0–1).
    pub jumping: Option<(u32, u32, f32)>,
}

/// The panel's state.
pub struct DevPanel {
    pub open: bool,
    pub section: usize,
    pub overlays: Overlays,
    // jump
    pub difficulty: usize,
    pub night: u32,
    pub seed: String,
    pub hire: bool,
    // screens / scenes / cheats
    pub filter: String,
    pub scene_filter: String,
    pub scene_fresh: bool,
    pub event_filter: String,
    pub regular: usize,
    pub key: String,
    pub value: String,
    /// The last action's result line (text, when).
    pub status: Option<(String, f64)>,
    /// Content height of each section last frame (for its scroll area).
    heights: [f32; SECTIONS.len()],
}

impl Default for DevPanel {
    fn default() -> Self {
        DevPanel {
            open: false,
            section: JUMP,
            overlays: Overlays::default(),
            difficulty: 0,
            night: 10,
            seed: format!("{}", super::jump::STORY_SEED),
            hire: true,
            filter: String::new(),
            scene_filter: String::new(),
            scene_fresh: false,
            event_filter: String::new(),
            regular: 0,
            key: String::new(),
            value: String::new(),
            status: None,
            heights: [0.0; SECTIONS.len()],
        }
    }
}

/// Panel width (UI units).
pub const WIDTH: f32 = 340.0;
const ROW: f32 = 22.0;
const GAP: f32 = 4.0;

/// A small row layout inside the scroll area.
struct Rows {
    x: f32,
    w: f32,
    y: f32,
    top: f32,
}

impl Rows {
    fn take(&mut self, h: f32) -> Rect {
        let r = Rect::new(self.x, self.y, self.w, h);
        self.y += h + GAP;
        r
    }
    fn space(&mut self, h: f32) {
        self.y += h;
    }
    fn height(&self) -> f32 {
        self.y - self.top
    }
}

/// `n` equal cells of `r` with `gap` between.
fn cells(r: Rect, n: usize, gap: f32) -> Vec<Rect> {
    let n = n.max(1);
    let w = (r.w - gap * (n - 1) as f32) / n as f32;
    (0..n)
        .map(|i| Rect::new(r.x + i as f32 * (w + gap), r.y, w, r.h))
        .collect()
}

/// A compact button (12 pt): true when clicked.
fn btn(
    ui: &mut Ui,
    key: &str,
    r: Rect,
    label: &str,
    kind: ButtonKind,
    on: bool,
    off: bool,
) -> bool {
    let id = ui.id(key);
    let resp = ui.interact(id, r);
    let t = ui.theme.clone();
    if off {
        crate::ui::draw::push_alpha(0.4);
    }
    paint_surface(&t, r, 5.0, kind, on, resp.hover_t, resp.press_t);
    let fg = if kind == ButtonKind::Primary || on {
        t.text_on_accent
    } else {
        t.text
    };
    let st = t.small_medium().color(fg);
    ui.fonts
        .draw_in(label, r.inset_xy(3.0, 0.0), &st, Align::Center);
    if off {
        crate::ui::draw::pop_alpha();
    }
    resp.clicked && !off
}

fn label(ui: &mut Ui, r: Rect, text: &str, st: &TextStyle) {
    ui.fonts.draw_in(text, r, st, Align::Left);
}

/// Does a cutscene stop at a choice somewhere?
pub fn has_choice(def: &CutsceneDef) -> bool {
    let mut any = false;
    crate::content::walk_steps(&def.steps, &mut |s| {
        if matches!(s, CutsceneStep::Choice { .. }) {
            any = true;
        }
    });
    any
}

/// A cutscene's trigger in a few words.
pub fn trigger_text(t: &CutsceneTrigger) -> String {
    match t {
        CutsceneTrigger::NightStart(n) => format!("night {n} start"),
        CutsceneTrigger::NightEnd(n) => format!("night {n} end"),
        CutsceneTrigger::At { night, time } => format!("night {night} {}", clock(time)),
        CutsceneTrigger::FlagSet(f) => format!("flag {f}"),
        CutsceneTrigger::ArcStage { regular, stage } => format!("arc {regular} {stage}"),
        CutsceneTrigger::Ending(e) => format!("ending {e}"),
        CutsceneTrigger::Talk { regular, night } => match night {
            Some(n) => format!("talk {regular} · night {n}"),
            None => format!("talk {regular}"),
        },
        CutsceneTrigger::Manual => "manual".into(),
    }
}

/// `HH:MM` of a clock time.
pub fn clock(t: &crate::content::ClockTime) -> String {
    format!("{:02}:{:02}", t.minute_of_day / 60, t.minute_of_day % 60)
}

impl DevPanel {
    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    pub fn say(&mut self, text: impl Into<String>, now: f64) {
        self.status = Some((text.into(), now));
    }

    /// The panel rect on this screen.
    pub fn rect(ui: &Ui) -> Rect {
        let s = ui.screen();
        let w = WIDTH.min(s.w - 16.0);
        Rect::new(s.w - w - 8.0, 8.0, w, s.h - 16.0)
    }

    /// Draws the panel (when open); what was clicked.
    pub fn draw(&mut self, ui: &mut Ui, v: &View) -> Option<DevAction> {
        if !self.open {
            return None;
        }
        ui.set_camera();
        ui.push_id("dev_panel");
        let r = Self::rect(ui);
        ui.dev_raise();
        ui.region(r);
        draw_glass(&ui.theme, r, &Glass::panel().solid());
        let t = ui.theme.clone();
        let inner = r.inset(10.0);
        // header: title, what is running
        let title = t.h2().size(13.0);
        label(
            ui,
            Rect::new(inner.x, inner.y, inner.w, 18.0),
            "Developer panel",
            &title,
        );
        let keys = t.small().size(11.0);
        ui.fonts.draw_in(
            "` / F1 to close",
            Rect::new(inner.x, inner.y, inner.w, 18.0),
            &keys,
            Align::Right,
        );
        let state = match (v.game, v.jumping) {
            (_, Some((n, target, _))) => format!("jumping… night {n} → {target}"),
            (Some(g), None) => cheats::summary(g),
            (None, None) => "no game running (menu / credits)".into(),
        };
        label(
            ui,
            Rect::new(inner.x, inner.y + 18.0, inner.w, 16.0),
            &state,
            &t.small().size(11.0),
        );
        let mut tab = self.section;
        let tabs_r = Rect::new(inner.x, inner.y + 38.0, inner.w, 24.0);
        let tab_cells = cells(tabs_r, SECTIONS.len(), 2.0);
        for (i, (c, name)) in tab_cells.iter().zip(SECTIONS).enumerate() {
            if btn(
                ui,
                &format!("tab{i}"),
                *c,
                name,
                ButtonKind::Ghost,
                i == tab,
                false,
            ) {
                tab = i;
            }
        }
        self.section = tab;
        // footer: the last result
        let foot = Rect::new(inner.x, inner.bottom() - 16.0, inner.w, 16.0);
        if let Some((text, at)) = &self.status
            && ui.time() - at < 6.0
        {
            label(ui, foot, text, &t.small().size(11.0).color(t.amber));
        }
        let view = Rect::new(
            inner.x,
            tabs_r.bottom() + 8.0,
            inner.w,
            foot.y - tabs_r.bottom() - 12.0,
        );
        let sec = self.section;
        let scroll = ui.begin_scroll(("dev_scroll", sec), view, self.heights[sec]);
        let mut rows = Rows {
            x: view.x,
            w: view.w - 8.0,
            y: scroll.top(),
            top: scroll.top(),
        };
        let act = match sec {
            JUMP => self.jump(ui, &mut rows, v),
            SCREENS => self.screens(ui, &mut rows, &scroll),
            SCENES => self.scenes(ui, &mut rows, &scroll, v),
            LANG => self.lang(ui, &mut rows, v),
            CHEATS => self.cheats(ui, &mut rows, &scroll, v),
            _ => {
                self.overlay(ui, &mut rows);
                None
            }
        };
        self.heights[sec] = rows.height() + 4.0;
        ui.end_scroll(scroll);
        ui.pop_id();
        act
    }

    fn header(ui: &mut Ui, rows: &mut Rows, text: &str) {
        rows.space(4.0);
        let r = rows.take(14.0);
        ui.section(r, text);
    }

    fn hint(ui: &mut Ui, rows: &mut Rows, text: &str) {
        let st = ui.theme.small().size(11.0);
        let lines = ui.fonts.wrap(text, rows.w, &st);
        for l in lines {
            let r = rows.take(13.0);
            label(ui, r, &l, &st);
        }
    }

    fn jump(&mut self, ui: &mut Ui, rows: &mut Rows, v: &View) -> Option<DevAction> {
        let mut act = None;
        Self::header(ui, rows, "Difficulty");
        let r = rows.take(ROW);
        for (i, c) in cells(r, 3, GAP).into_iter().enumerate() {
            let (name, _) = DIFFICULTIES[i];
            if btn(
                ui,
                &format!("diff{i}"),
                c,
                name,
                ButtonKind::Secondary,
                self.difficulty == i,
                false,
            ) {
                self.difficulty = i;
            }
        }
        Self::header(ui, rows, "Night");
        let r = rows.take(ROW);
        let c = cells(r, 4, GAP);
        if btn(
            ui,
            "n-",
            c[0],
            "−",
            ButtonKind::Secondary,
            false,
            self.night <= 1,
        ) {
            self.night -= 1;
        }
        let st = ui.theme.strong();
        ui.fonts.draw_in(
            &format!("Night {}", self.night),
            Rect::new(c[1].x, c[1].y, c[2].right() - c[1].x, c[1].h),
            &st,
            Align::Center,
        );
        if btn(
            ui,
            "n+",
            c[3],
            "+",
            ButtonKind::Secondary,
            false,
            self.night >= 30,
        ) {
            self.night += 1;
        }
        let r = rows.take(ROW);
        let quick = [1, 2, 5, 9, 12, 15, 20, 25, 29, 30];
        for (i, c) in cells(r, quick.len(), 2.0).into_iter().enumerate() {
            let n = quick[i];
            if btn(
                ui,
                &format!("q{n}"),
                c,
                &n.to_string(),
                ButtonKind::Ghost,
                self.night == n,
                false,
            ) {
                self.night = n;
            }
        }
        Self::header(ui, rows, "Seed & helpers");
        let r = rows.take(28.0);
        let _ = ui.text_input("seed", r, &mut self.seed, "seed (number)", 24);
        let r = rows.take(ROW);
        let _ = ui.toggle_row(
            "hire",
            r,
            "Bot hires helpers on nights 3, 5, 9, 14",
            &mut self.hire,
        );
        let seed = self.seed.trim().parse::<u64>().ok();
        let busy = v.jumping.is_some();
        rows.space(4.0);
        let r = rows.take(26.0);
        let diff = DIFFICULTIES[self.difficulty].1;
        if btn(
            ui,
            "start",
            r,
            &format!(
                "New story · {} · to night {}",
                DIFFICULTIES[self.difficulty].0, self.night
            ),
            ButtonKind::Primary,
            false,
            busy || seed.is_none(),
        ) {
            act = Some(DevAction::Story {
                difficulty: diff,
                seed: seed.unwrap_or(super::jump::STORY_SEED),
                night: self.night,
                hire: self.hire,
            });
        }
        let story_on = v
            .game
            .filter(|g| g.world.mode() == Mode::Story && !g.world.is_closed());
        let can_continue = story_on.is_some_and(|g| g.world.night() < self.night);
        let r = rows.take(ROW);
        let cont = match story_on {
            Some(g) => format!(
                "Continue this run · night {} → {}",
                g.world.night(),
                self.night
            ),
            None => "Continue this run (no story running)".into(),
        };
        if btn(
            ui,
            "cont",
            r,
            &cont,
            ButtonKind::Secondary,
            false,
            busy || !can_continue,
        ) {
            act = Some(DevAction::ContinueTo {
                night: self.night,
                hire: self.hire,
            });
        }
        if let Some((n, target, p)) = v.jumping {
            let r = rows.take(8.0);
            let c = ui.theme.indigo_400;
            ui.progress(r, p, c);
            Self::hint(
                ui,
                rows,
                &format!("The bot is playing night {n} of {target}…"),
            );
        }
        Self::header(ui, rows, "Sandbox");
        let r = rows.take(ROW);
        let c = cells(r, 2, GAP);
        let seed = seed.unwrap_or(2039);
        if btn(
            ui,
            "sbx",
            c[0],
            &format!("Endless · night {}", self.night),
            ButtonKind::Secondary,
            false,
            busy,
        ) {
            act = Some(DevAction::Sandbox {
                difficulty: diff,
                seed,
                night: self.night,
                challenge: false,
            });
        }
        if btn(
            ui,
            "chal",
            c[1],
            &format!("Challenge · night {}", self.night),
            ButtonKind::Secondary,
            false,
            busy,
        ) {
            act = Some(DevAction::Sandbox {
                difficulty: diff,
                seed,
                night: self.night,
                challenge: true,
            });
        }
        Self::header(ui, rows, "App");
        let r = rows.take(ROW);
        if btn(
            ui,
            "menu",
            r,
            "Back to the main menu",
            ButtonKind::Secondary,
            false,
            busy,
        ) {
            act = Some(DevAction::MainMenu);
        }
        Self::hint(
            ui,
            rows,
            "The bot (sim_balance's player) plays every night before N headless, then you \
             take over at N's day. Same seed → same café. Saves go to the dev folder.",
        );
        act
    }

    fn screens(
        &mut self,
        ui: &mut Ui,
        rows: &mut Rows,
        scroll: &crate::ui::ScrollArea,
    ) -> Option<DevAction> {
        let mut act = None;
        let r = rows.take(28.0);
        let _ = ui.text_input("filter", r, &mut self.filter, "filter screens…", 40);
        let f = self.filter.trim().to_lowercase();
        Self::hint(
            ui,
            rows,
            "Each opens a fresh café set up like its screenshot scene (late nights take a moment).",
        );
        let all = catalog::all();
        for group in catalog::groups() {
            let items: Vec<(usize, &catalog::Opener)> = all
                .iter()
                .enumerate()
                .filter(|(_, o)| o.group == group)
                .filter(|(_, o)| {
                    f.is_empty()
                        || o.label.to_lowercase().contains(&f)
                        || o.group.to_lowercase().contains(&f)
                        || o.scene.contains(&f)
                })
                .collect();
            if items.is_empty() {
                continue;
            }
            Self::header(ui, rows, group);
            let cols = if group == "Mini-games" { 3 } else { 2 };
            for chunk in items.chunks(cols) {
                let r = rows.take(ROW);
                if !scroll.visible(r.y, r.h) {
                    continue;
                }
                for ((i, o), c) in chunk.iter().zip(cells(r, cols, GAP)) {
                    if btn(
                        ui,
                        &format!("open{i}"),
                        c,
                        o.label,
                        ButtonKind::Secondary,
                        false,
                        false,
                    ) {
                        act = Some(DevAction::Open(*i));
                    }
                }
            }
        }
        act
    }

    fn scenes(
        &mut self,
        ui: &mut Ui,
        rows: &mut Rows,
        scroll: &crate::ui::ScrollArea,
        v: &View,
    ) -> Option<DevAction> {
        let mut act = None;
        let r = rows.take(28.0);
        let _ = ui.text_input(
            "scene_filter",
            r,
            &mut self.scene_filter,
            "search cutscenes & talks…",
            40,
        );
        let r = rows.take(ROW);
        let _ = ui.toggle_row(
            "fresh",
            r,
            "At its own night (a fresh café)",
            &mut self.scene_fresh,
        );
        let fresh = self.scene_fresh || v.game.is_none();
        Self::hint(
            ui,
            rows,
            if fresh {
                "Plays over a fresh café at the scene's night (talks: the regular's visit)."
            } else {
                "Plays over the café that is running now."
            },
        );
        let f = self.scene_filter.trim().to_lowercase();
        let st = ui.theme.small_medium();
        let dim = ui.theme.small().size(11.0);
        let list = &v.content.cutscenes.cutscenes;
        let mut shown = 0;
        for def in list {
            let trig = trigger_text(&def.trigger);
            if !f.is_empty() && !def.id.contains(&f) && !trig.to_lowercase().contains(&f) {
                continue;
            }
            shown += 1;
            let r = rows.take(ROW);
            if !scroll.visible(r.y, r.h) {
                continue;
            }
            let choice = has_choice(def);
            let bw = 50.0;
            let name_r = Rect::new(r.x, r.y, r.w - 2.0 * bw - 2.0 * GAP, r.h);
            let half = name_r.w * 0.56;
            label(ui, Rect::new(r.x, r.y, half, r.h), &def.id, &st);
            label(
                ui,
                Rect::new(r.x + half + 4.0, r.y, name_r.w - half - 4.0, r.h),
                &trig,
                &dim,
            );
            let play = Rect::new(r.right() - 2.0 * bw - GAP, r.y, bw, r.h);
            let ch = Rect::new(r.right() - bw, r.y, bw, r.h);
            if btn(
                ui,
                &format!("play_{}", def.id),
                play,
                "Play",
                ButtonKind::Secondary,
                false,
                false,
            ) {
                act = Some(DevAction::Cutscene {
                    id: def.id.clone(),
                    choice: false,
                    fresh,
                });
            }
            if btn(
                ui,
                &format!("choice_{}", def.id),
                ch,
                "Choice",
                ButtonKind::Ghost,
                false,
                !choice,
            ) {
                act = Some(DevAction::Cutscene {
                    id: def.id.clone(),
                    choice: true,
                    fresh,
                });
            }
        }
        Self::hint(ui, rows, &format!("{shown} of {} scenes", list.len()));
        act
    }

    fn lang(&mut self, ui: &mut Ui, rows: &mut Rows, v: &View) -> Option<DevAction> {
        let mut act = None;
        Self::header(ui, rows, "Language (live)");
        let names = [
            (Lang::Cs, "Čeština"),
            (Lang::En, "English"),
            (Lang::Uk, "Українська"),
            (Lang::Ru, "Русский"),
        ];
        let r = rows.take(26.0);
        for ((l, name), c) in names.iter().zip(cells(r, 4, GAP)) {
            if btn(
                ui,
                &format!("lang_{}", l.code()),
                c,
                name,
                ButtonKind::Secondary,
                v.lang == *l,
                false,
            ) {
                act = Some(DevAction::Lang(*l));
            }
        }
        Self::hint(
            ui,
            rows,
            "Switches every screen at once (not saved). The dev panel itself stays in English.",
        );
        act
    }

    fn cheats(
        &mut self,
        ui: &mut Ui,
        rows: &mut Rows,
        scroll: &crate::ui::ScrollArea,
        v: &View,
    ) -> Option<DevAction> {
        let Some(g) = v.game else {
            Self::hint(ui, rows, "Start or open a game first (Jump or Screens).");
            return None;
        };
        let mut act: Option<Cheat> = None;
        let night = g.world.phase() == Phase::Night;
        let mut grid =
            |ui: &mut Ui, rows: &mut Rows, key: &str, items: &[(&str, Cheat)], off: bool| {
                let r = rows.take(ROW);
                for (i, ((name, c), cell)) in
                    items.iter().zip(cells(r, items.len(), GAP)).enumerate()
                {
                    if btn(
                        ui,
                        &format!("{key}{i}"),
                        cell,
                        name,
                        ButtonKind::Secondary,
                        false,
                        off,
                    ) {
                        act = Some(c.clone());
                    }
                }
            };
        Self::header(ui, rows, "Money & reputation");
        grid(
            ui,
            rows,
            "money",
            &[
                ("−1 000", Cheat::Money(-1000)),
                ("+1 000", Cheat::Money(1000)),
                ("+10 000", Cheat::Money(10_000)),
                ("rep −10", Cheat::Reputation(-10.0)),
                ("rep +10", Cheat::Reputation(10.0)),
            ],
            false,
        );
        Self::header(ui, rows, "Clock & speed");
        let times: Vec<(&str, Cheat)> = cheats::TIMES
            .iter()
            .map(|(l, m)| (*l, Cheat::Time(*m)))
            .collect();
        grid(ui, rows, "time", &times, false);
        grid(
            ui,
            rows,
            "speed",
            &[
                ("Pause", Cheat::Speed(0)),
                ("×1", Cheat::Speed(1)),
                ("×2", Cheat::Speed(2)),
                ("×3", Cheat::Speed(3)),
                ("Morning", Cheat::Morning),
            ],
            false,
        );
        Self::header(ui, rows, "Weather, dirt, fridge");
        let weathers: Vec<(String, Cheat)> = cheats::WEATHERS
            .iter()
            .map(|w| (format!("{w:?}"), Cheat::Weather(*w)))
            .collect();
        let wref: Vec<(&str, Cheat)> = weathers
            .iter()
            .map(|(n, c)| (n.as_str(), c.clone()))
            .collect();
        grid(ui, rows, "weather", &wref, false);
        grid(
            ui,
            rows,
            "dirt",
            &[
                ("Clean", Cheat::Dirt(0.0)),
                ("Dirt ½", Cheat::Dirt(0.5)),
                ("Dirty", Cheat::Dirt(1.0)),
                ("Fill fridge", Cheat::Fridge(true)),
                ("Empty", Cheat::Fridge(false)),
            ],
            false,
        );
        Self::header(ui, rows, "Crew & unlocks");
        grid(
            ui,
            rows,
            "crew",
            &[
                ("Hire first candidate", Cheat::Hire),
                ("Fire last helper", Cheat::Fire),
            ],
            false,
        );
        grid(
            ui,
            rows,
            "unlock",
            &[
                ("Notes", Cheat::Notes),
                ("Recipes", Cheat::Recipes),
                ("Furniture", Cheat::Furniture),
                ("Themes", Cheat::Themes),
            ],
            false,
        );
        // regulars and cameos
        Self::header(ui, rows, "Regulars & cameos");
        let regs = &v.content.regulars;
        if !regs.is_empty() {
            self.regular = self.regular.min(regs.len() - 1);
            for chunk in regs.iter().enumerate().collect::<Vec<_>>().chunks(4) {
                let r = rows.take(ROW);
                for ((i, reg), c) in chunk.iter().zip(cells(r, 4, GAP)) {
                    let name = if reg.story_nights.is_empty() {
                        reg.id.clone()
                    } else {
                        format!("★{}", reg.id.trim_start_matches("guest_"))
                    };
                    if btn(
                        ui,
                        &format!("reg{i}"),
                        c,
                        &name,
                        ButtonKind::Ghost,
                        self.regular == *i,
                        false,
                    ) {
                        self.regular = *i;
                    }
                }
            }
            let id = regs[self.regular].id.clone();
            let arc = g
                .world
                .story()
                .counters
                .get(&format!("arc.{id}"))
                .copied()
                .unwrap_or(0);
            let trust = g.world.story().trust.get(&id).copied().unwrap_or(0.0);
            Self::hint(
                ui,
                rows,
                &format!("{id}: arc stage {arc} · trust {trust:.2} (★ = cameo)"),
            );
            grid(
                ui,
                rows,
                "regact",
                &[
                    ("Walk in", Cheat::Spawn(id.clone())),
                    ("arc −", Cheat::Arc(id.clone(), -1)),
                    ("arc +", Cheat::Arc(id.clone(), 1)),
                    ("trust −", Cheat::Trust(id.clone(), -5.0)),
                    ("trust +", Cheat::Trust(id.clone(), 5.0)),
                ],
                false,
            );
            if !night {
                Self::hint(ui, rows, "(walk-ins only at night)");
            }
        }
        Self::header(ui, rows, "Story state");
        let r = rows.take(28.0);
        let c = cells(r, 2, GAP);
        let _ = ui.text_input("key", c[0], &mut self.key, "flag / var / counter", 48);
        let _ = ui.text_input("value", c[1], &mut self.value, "value", 48);
        let k = self.key.clone();
        let val = self.value.clone();
        let num = val.trim().parse::<i32>().unwrap_or(0);
        grid(
            ui,
            rows,
            "story",
            &[
                ("Set flag", Cheat::Flag(k.clone(), true)),
                ("Clear flag", Cheat::Flag(k.clone(), false)),
                ("Set var", Cheat::Var(k.clone(), val.clone())),
                ("Set counter", Cheat::Counter(k.clone(), num)),
            ],
            false,
        );
        // events
        Self::header(ui, rows, "Events (events.ron)");
        let r = rows.take(28.0);
        let _ = ui.text_input(
            "event_filter",
            r,
            &mut self.event_filter,
            "filter events…",
            40,
        );
        let f = self.event_filter.trim().to_lowercase();
        let ids: Vec<&str> = v
            .content
            .events
            .iter()
            .map(|e| e.id.as_str())
            .filter(|id| f.is_empty() || id.contains(&f))
            .collect();
        for (n, chunk) in ids.chunks(2).enumerate() {
            let r = rows.take(ROW);
            if !scroll.visible(r.y, r.h) {
                continue;
            }
            for (j, (id, c)) in chunk.iter().zip(cells(r, 2, GAP)).enumerate() {
                if btn(
                    ui,
                    &format!("ev{n}_{j}"),
                    c,
                    id,
                    ButtonKind::Secondary,
                    false,
                    false,
                ) {
                    act = Some(Cheat::Event(id.to_string()));
                }
            }
        }
        act.map(DevAction::Cheat)
    }

    fn overlay(&mut self, ui: &mut Ui, rows: &mut Rows) {
        Self::header(ui, rows, "Debug drawing");
        let o = &mut self.overlays;
        for (key, text, v) in [
            ("fps", "FPS & frame times (F3)", &mut o.fps),
            (
                "layout",
                "Layout debug: text / widgets out of bounds",
                &mut o.layout,
            ),
            (
                "regions",
                "UI regions (opaque areas, by layer)",
                &mut o.regions,
            ),
            ("nav", "Nav grid: walkable / guests / seats", &mut o.nav),
            ("seats", "Seat claims: who sits where", &mut o.seats),
        ] {
            let r = rows.take(ROW);
            let _ = ui.toggle_row(key, r, text, v);
        }
        Self::hint(
            ui,
            rows,
            "Nav grid: green walkable, blue guests too, amber a seat, red blocked. Seats: \
             a line from each seated guest to their chair.",
        );
    }
}

/// The small "DEV" badge in the bottom-left corner, under the HUD's toolbar (every screen of
/// the dev build).
pub fn draw_badge(ui: &mut Ui) {
    ui.set_camera();
    let s = ui.screen();
    let t = ui.theme.clone();
    let st = t.micro().color(t.ink).size(9.0);
    let w = ui.fonts.measure("DEV", &st) + 10.0;
    let r = Rect::new(3.0, s.h - 14.0, w, 12.0);
    fill_rrect(r, 3.0, t.amber);
    stroke_rrect(r, 4.0, 1.0, Color::new(0.0, 0.0, 0.0, 0.25));
    ui.fonts.draw_in("DEV", r, &st, Align::Center);
}

/// The main menu's "Developer" button in its top-right corner (dev build): true when clicked.
pub fn menu_button(ui: &mut Ui) -> bool {
    ui.set_camera();
    let s = ui.screen();
    let r = Rect::new(s.w - 128.0, 10.0, 118.0, 24.0);
    ui.region(r);
    btn(
        ui,
        "dev_menu_button",
        r,
        "Developer  `",
        ButtonKind::Secondary,
        false,
        false,
    )
}

/// The jump's progress card in the middle of the screen.
pub fn draw_progress(ui: &mut Ui, night: u32, target: u32, p: f32) {
    ui.set_camera();
    let s = ui.screen();
    let r = Rect::new(s.w / 2.0 - 140.0, s.h / 2.0 - 34.0, 280.0, 68.0);
    ui.dev_raise();
    ui.region(r);
    draw_glass(&ui.theme, r, &Glass::card().solid());
    let t = ui.theme.clone();
    ui.fonts.draw_in(
        &format!("The bot is playing night {night} of {target}…"),
        Rect::new(r.x + 14.0, r.y + 10.0, r.w - 28.0, 20.0),
        &t.strong(),
        Align::Left,
    );
    ui.progress(
        Rect::new(r.x + 14.0, r.y + 40.0, r.w - 28.0, 8.0),
        p,
        t.indigo_400,
    );
}

/// "Opening…" while a screen is being built (the frame before it blocks).
pub fn draw_busy(ui: &mut Ui, what: &str) {
    ui.set_camera();
    let s = ui.screen();
    let t = ui.theme.clone();
    let st = t.strong();
    let w = ui.fonts.measure(what, &st) + 32.0;
    let r = Rect::new(s.w / 2.0 - w / 2.0, s.h / 2.0 - 20.0, w, 40.0);
    draw_glass(&ui.theme, r, &Glass::card().solid());
    ui.fonts.draw_in(what, r, &st, Align::Center);
}
