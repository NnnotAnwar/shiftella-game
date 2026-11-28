//! Screen-level chaos (QA): `GameScreen` driven headless over many nights, the autoplayer
//! mixed with random player actions from a seeded RNG — intents, rail clicks, every panel,
//! order cards, mini-games, the book, moments, stories, photo mode, the back step, the walk
//! home, speed and pause, build edits, crew and rozpis changes by day, story answers — and
//! save → load round trips (the loaded world must continue exactly like the original). The
//! sim's invariants (`sim::chaos::check`) run after every few frames.
//!
//! Quick run (in `cargo test`): `chaos_quick_screen`. Heavy runs:
//! `CHAOS_SEEDS=8 CHAOS_NIGHTS=30 cargo test --release --lib chaos_heavy_screen -- --ignored --nocapture`
//! (`CHAOS_SEED0` shifts the seeds, `CHAOS_KIND=0..3` picks one mode: story Cozy, Carefree,
//! Classic, sandbox). Each run ends with `[tally] N × kind` lines — distinct incidents by
//! kind, uncapped (one agent on one tile counts once) — and `CHAOS_MAP=1` prints the floor
//! (`chaos::map`) with a run's first violation.

use std::collections::VecDeque;
use std::rc::Rc;

use super::*;
use crate::content::Content;
use crate::content::handson::AppLink;
use crate::settings::load_i18n;
use crate::sim::chaos;
use crate::sim::rng::Rng;
use crate::ui::handson::BrewMode as UiBrew;

fn content() -> Rc<Content> {
    Rc::new(chaos::content())
}

fn i18n() -> crate::i18n::I18n {
    let root = env!("CARGO_MANIFEST_DIR");
    load_i18n(&format!("{root}/assets/i18n")).expect("i18n")
}

const PANELS: [Panel; 9] = [
    Panel::Crew,
    Panel::Duties,
    Panel::App,
    Panel::Build,
    Panel::Log,
    Panel::Help,
    Panel::Journal,
    Panel::Payroll,
    Panel::Attic,
];
const LINKS: [AppLink; 8] = [
    AppLink::Rozpis,
    AppLink::RecipeBook,
    AppLink::Pantry,
    AppLink::Build,
    AppLink::Duties,
    AppLink::Money,
    AppLink::Journal,
    AppLink::Board,
];
const ROUTES: [&str; 5] = ["embankment", "park", "bakery", "tram", "nowhere"];

#[derive(Debug, Default)]
struct Stats {
    nights: u32,
    frames: u64,
    ticks: u64,
    actions: u64,
    save_loads: u32,
    endings: u32,
    max_frames_night: u64,
    off_floor: u64,
    off_floor_examples: Vec<String>,
    /// Distinct violations by kind, uncapped (`chaos::kind_of`).
    kinds: std::collections::BTreeMap<String, u64>,
    seen: std::collections::HashSet<String>,
}

struct Chaos<'a> {
    c: Rc<Content>,
    loc: &'a Locale<'a>,
    g: GameScreen,
    r: Rng,
    /// The last actions (a failure prints them as the repro).
    log: VecDeque<String>,
    violations: Vec<String>,
    stats: Stats,
    /// Frames the current night has taken / the scene the walk has been open for.
    night_frames: u64,
    open_frames: u32,
    frozen_frames: u32,
    /// Tonight: the autoplayer plays (else only random actions).
    autoplay: bool,
    /// Chance of a random action per frame.
    rate: f32,
    /// Credits reached (the run ends).
    done: bool,
    day_bot: sim::bot::PlayerBot,
}

impl<'a> Chaos<'a> {
    fn new(c: &Rc<Content>, loc: &'a Locale<'a>, kind: usize, seed: u64, rate: f32) -> Self {
        let mut g = match kind % 4 {
            0 => GameScreen::new_story(c, "cozy", !seed.is_multiple_of(3)),
            1 => GameScreen::new_story(c, "relaxed", true),
            2 => GameScreen::new_story(c, "classic", true),
            _ => GameScreen::new_sandbox(
                c,
                sim::SandboxSetup {
                    seed,
                    difficulty: if seed.is_multiple_of(2) {
                        sim::Difficulty::Cozy
                    } else {
                        sim::Difficulty::Classic
                    },
                    ..Default::default()
                },
            ),
        };
        g.drain_and_handle(c, Some(loc));
        Chaos {
            c: c.clone(),
            loc,
            g,
            r: Rng::new(seed ^ 0x005C_2EE7),
            log: VecDeque::new(),
            violations: Vec::new(),
            stats: Stats::default(),
            night_frames: 0,
            open_frames: 0,
            frozen_frames: 0,
            autoplay: true,
            rate,
            done: false,
            day_bot: sim::bot::PlayerBot::new(sim::bot::BotConfig {
                seed,
                ..Default::default()
            }),
        }
    }

    fn note(&mut self, what: String) {
        let w = &self.g.world;
        let line = format!("n{} {:?} {} {what}", w.night(), w.phase(), w.clock_label());
        self.log.push_back(line);
        if self.log.len() > 16 {
            self.log.pop_front();
        }
        self.stats.actions += 1;
    }

    fn violation(&mut self, v: String) {
        if !self.stats.seen.insert(chaos::head_of(&v).to_string()) {
            return;
        }
        *self.stats.kinds.entry(chaos::kind_of(&v)).or_default() += 1;
        if self.stats.seen.len() == 1 && std::env::var("CHAOS_MAP").is_ok() {
            eprintln!("{v}\n{}", chaos::map(&self.g.world, &self.c));
        }
        if self.violations.len() >= 30 {
            return;
        }
        let tail: Vec<String> = self.log.iter().rev().take(8).cloned().collect();
        self.violations.push(format!("recent {tail:?} | {v}"));
    }

    fn check(&mut self) {
        for v in chaos::check(&self.g.world, &self.c) {
            self.violation(v);
        }
        let off: Vec<String> = Vec::new();
        self.stats.off_floor += off.len() as u64;
        for o in off {
            if self.stats.off_floor_examples.len() < 10 {
                let at = self.g.world.clock_label();
                self.stats
                    .off_floor_examples
                    .push(format!("n{} {at} {o}", self.g.world.night()));
            }
        }
        // screen-side consistency
        let both = self.g.hands.minigame.is_some() && self.g.hands.order.is_some();
        let by_day = self.g.world.phase() == Phase::Day && self.g.hands.minigame.is_some();
        if both {
            self.violation("a mini-game and an order card at once".into());
        }
        if by_day {
            self.violation("a mini-game open by day".into());
        }
    }

    /// One frame: whatever is on screen, then the phase.
    fn frame(&mut self) {
        self.stats.frames += 1;
        let c = self.c.clone();
        let loc = self.loc;
        if let Some(exit) = self.g.exit() {
            match exit {
                GameExit::Credits { ending, .. } => {
                    self.stats.endings += 1;
                    if self.g.world.mode() == sim::Mode::Story
                        && ending.is_none()
                        && !self.g.world.is_closed()
                    {
                        self.violation("credits without an ending".into());
                    }
                    self.done = true;
                }
                other => self.violation(format!("unexpected exit {other:?}")),
            }
            return;
        }
        if self.g.cutscene_playing() {
            if self.r.chance(0.5) {
                self.g.finish_cutscene_now(&c, loc);
            } else {
                self.finish_scene_random();
            }
            return;
        }
        if self.g.summary.is_some() {
            self.morning();
            return;
        }
        if self.g.walk.is_open() {
            self.open_frames += 1;
            if self.open_frames == 3 && self.g.walk.walking() && self.r.chance(0.7) {
                let route = *self.r.pick(&ROUTES).unwrap();
                self.note(format!("walk route {route}"));
                walk_home::debug_seek(&mut self.g, &c, Some(route), 1.0);
            }
            if self.open_frames > 4 + self.r.index(40) as u32 {
                self.note("walk/backstep closes".into());
                self.g.walk = Default::default();
                self.open_frames = 0;
            }
            return;
        }
        if self.g.photo.is_open() {
            self.open_frames += 1;
            if self.open_frames > 2 + self.r.index(30) as u32 {
                self.open_frames = 0;
                if self.r.chance(0.7) {
                    let k = self
                        .g
                        .world
                        .karolina()
                        .map(|k| k.pos)
                        .unwrap_or((900.0, 500.0));
                    let width = 200.0 + self.r.f32() * 900.0;
                    let r = self.g.command(&c, Command::TakePhoto { focus: k, width });
                    self.note(format!("photo taken {r:?}"));
                } else {
                    self.note("photo cancelled".into());
                }
                self.g.photo = Default::default();
            }
            return;
        }
        if self.g.stories.talk_open() {
            self.open_frames += 1;
            if self.open_frames > 20 + self.r.index(60) as u32 {
                self.open_frames = 0;
                self.note("storm talk closed".into());
                self.g.stories.talk = None;
            }
            return;
        }
        self.open_frames = 0;
        match self.g.world.phase() {
            Phase::Day => self.day(),
            Phase::Night => self.night_frame(),
        }
    }

    fn finish_scene_random(&mut self) {
        let c = self.c.clone();
        let mut guard = 0;
        while self.g.player.is_playing() && guard < 8 {
            let picks: Vec<usize> = (0..4).map(|_| self.r.index(4)).collect();
            self.g.player.fast_forward_with(self.loc, 600.0, &picks);
            let id = self.g.player.current_id().to_string();
            let choices = self
                .g
                .player
                .outcome()
                .choices
                .iter()
                .map(|c| c.index)
                .collect();
            if !self.g.player.is_playing() {
                self.note(format!("scene {id} random {picks:?}"));
                self.g.cutscene_finished(&c, id, choices);
            }
            guard += 1;
        }
    }

    fn morning(&mut self) {
        let c = self.c.clone();
        let n = self
            .g
            .world
            .pending_polaroids()
            .map(|(_, v)| v.len())
            .unwrap_or(0);
        self.g.photos.pick = self.r.index(n.max(1) + 1);
        let walk = self.r.chance(0.5);
        self.note(format!(
            "morning: keep polaroid {} (of {n}), walk {walk}",
            self.g.photos.pick
        ));
        if walk {
            walk_home::close_summary(&mut self.g, &c);
        } else {
            journal_ui::keep_morning(&mut self.g, &c);
            self.g.summary = None;
        }
    }

    // ───────────────────────── the day ─────────────────────────

    fn day(&mut self) {
        let c = self.c.clone();
        if self.g.world.is_closed() || self.g.world.ending().is_some() {
            // the credits come through `exit()`; a closed sandbox just ends
            self.g.advance(&c, 0.1, Some(self.loc));
            if self.g.world.is_closed() && self.g.world.mode() == sim::Mode::Sandbox {
                self.done = self.g.exit().is_some() || self.night_frames > 50;
                self.night_frames += 1;
            }
            return;
        }
        for _ in 0..self.r.range_u32(0, 14) {
            self.day_action();
            self.check();
        }
        if self.r.chance(0.6) {
            self.note("day bot plans".into());
            self.day_bot.day(&mut self.g.world, &c);
            self.g.drain_and_handle(&c, Some(self.loc));
        }
        for _ in 0..self.r.range_u32(0, 5) {
            self.day_action();
        }
        if self.r.chance(0.15) {
            self.save_load("day");
        }
        if self.g.cutscene_playing() {
            return;
        }
        self.autoplay = self.r.chance(0.8);
        self.g.hands.autoplay = self.autoplay.then_some(hands::Autoplay {
            cooldown: 0.0,
            keyboard: false,
            skip_minigames: self.r.chance(0.3),
        });
        self.night_frames = 0;
        self.note(format!("start night (autoplay {})", self.autoplay));
        self.g.start_night(&c);
        let s = *self.r.pick(&[1u8, 2, 3, 3, 3]).unwrap();
        self.g.set_speed(s);
        self.check();
    }

    fn day_action(&mut self) {
        let c = self.c.clone();
        let w = &self.g.world;
        let helpers: Vec<u32> = w
            .staff()
            .iter()
            .filter(|s| !s.owner)
            .map(|s| s.id)
            .collect();
        let cmd = match self.r.index(24) {
            0 | 1 => w
                .candidates()
                .first()
                .map(|x| Command::Hire { candidate: x.id }),
            2 => self.r.pick(&helpers).map(|&s| Command::Fire { staff: s }),
            3 | 4 => self.r.pick(&helpers).map(|&s| Command::SetDuty {
                staff: s,
                duty: *self.r.pick(&crate::content::handson::Duty::ALL).unwrap(),
                on: self.r.chance(0.6),
            }),
            5 | 6 => self.r.pick(&helpers).map(|&s| Command::SetShift {
                staff: s,
                weekday: self.r.range_u32(0, 7) as u8,
                shift: *self
                    .r
                    .pick(&[
                        sim::Shift::Off,
                        sim::Shift::Full,
                        sim::Shift::Early,
                        sim::Shift::Late,
                    ])
                    .unwrap(),
            }),
            7 => Some(Command::PublishSchedule),
            8 => Some(Command::AutoFillRozpis),
            9 => Some(Command::AutoSchedule),
            10..=13 => {
                // build: a new piece somewhere / a floor / remove / move a group
                let defs = w.buildable(&c);
                let x = self.r.range_u32(0, w.grid().width as u32) as i32;
                let y = self.r.range_u32(0, w.grid().height as u32) as i32;
                let uids: Vec<u32> = w.furniture().iter().map(|f| f.uid).collect();
                match self.r.index(5) {
                    0 | 1 => self.r.pick(&defs).map(|d| {
                        if d.starts_with("floor_") {
                            Command::SetFloor {
                                x,
                                y,
                                floor: d.clone(),
                            }
                        } else {
                            Command::PlaceFurniture {
                                def: d.clone(),
                                x,
                                y,
                                rot: self.r.range_u32(0, 4) as u8,
                            }
                        }
                    }),
                    2 => self
                        .r
                        .pick(&uids)
                        .map(|&uid| Command::RemoveFurniture { uid }),
                    3 => self
                        .r
                        .pick(&uids)
                        .map(|&uid| Command::RotateFurniture { uid }),
                    _ => self.r.pick(&uids).map(|&uid| Command::MoveGroup {
                        moves: w.group_moves(&c, uid, (x, y), self.r.range_u32(0, 4) as u8),
                    }),
                }
            }
            14 => {
                self.note("undo".into());
                let r = self.g.world.undo(&c);
                self.note(format!("undo → {r:?}"));
                None
            }
            15 => {
                let th: Vec<String> = c
                    .handson
                    .decor
                    .themes
                    .iter()
                    .map(|t| t.id.clone())
                    .collect();
                let id = self.r.pick(&th).cloned().unwrap_or_default();
                Some(if self.r.chance(0.5) {
                    Command::BuyTheme { id }
                } else {
                    Command::SetTheme { id }
                })
            }
            16 => {
                let a = w.attic(&c);
                match self.r.pick(&a.offers) {
                    Some(o) => Some(Command::BuyAtticPiece { id: o.id.clone() }),
                    None => Some(Command::OpenBackRoom),
                }
            }
            17 => {
                let recs: Vec<String> = w.records_unlocked().iter().cloned().collect();
                Some(Command::PlayRecord {
                    id: self.r.pick(&recs).cloned(),
                    repeat: self.r.chance(0.5),
                })
            }
            18 => {
                let pantry: Vec<String> = w.pantry().iter().cloned().collect();
                let n = 2 + self.r.index(2);
                Some(Command::TryExperiment {
                    ingredients: (0..n)
                        .filter_map(|_| self.r.pick(&pantry).cloned())
                        .collect(),
                })
            }
            19 => {
                // the panels by day
                let p = *self.r.pick(&PANELS).unwrap();
                self.g.panel = if self.r.chance(0.3) { None } else { Some(p) };
                self.g.ui_state.app_tab = self.r.index(6);
                self.note(format!("panel {:?}", self.g.panel));
                if self.r.chance(0.3) {
                    let l = *self.r.pick(&LINKS).unwrap();
                    self.g.follow_link(l);
                    self.note(format!("link {l:?}"));
                }
                if self.r.chance(0.2) {
                    attic_ui::toggle(&mut self.g);
                }
                None
            }
            20 => {
                // the café's drink card
                if self.g.sig.open {
                    self.g.close_signature();
                    self.note("signature card closed".into());
                    None
                } else if self.r.chance(0.5) {
                    self.g.open_signature(&c);
                    self.note("signature card opened".into());
                    None
                } else {
                    let base = w
                        .signature_bases(&c)
                        .into_iter()
                        .find(|(_, ok)| *ok)
                        .map(|(b, _)| (b.id.clone(), b.styles.first().copied()));
                    let jars: Vec<String> = w
                        .signature_jars()
                        .iter()
                        .take(2)
                        .map(|s| s.to_string())
                        .collect();
                    base.map(|(b, st)| Command::ComposeSignature {
                        base: b,
                        jars,
                        style: st.unwrap_or(crate::content::handson::MiniGameKind::MilkFroth),
                        name: "Noční vlak".into(),
                    })
                }
            }
            21 => {
                let req: Vec<u32> = w.app().time_off.iter().map(|t| t.id).collect();
                let sw: Vec<u32> = w.app().swaps.iter().map(|t| t.id).collect();
                if self.r.chance(0.5) {
                    self.r.pick(&req).map(|&id| Command::AnswerTimeOff {
                        id,
                        approve: self.r.chance(0.5),
                    })
                } else {
                    self.r.pick(&sw).map(|&id| Command::AnswerSwap {
                        id,
                        approve: self.r.chance(0.5),
                    })
                }
            }
            22 => {
                let m = self.r.pick(&w.app().inbox).cloned();
                m.and_then(|m| {
                    let acts: Vec<String> = c
                        .app
                        .bot
                        .iter()
                        .find(|b| b.id == m.template)
                        .map(|b| b.actions.iter().map(|a| a.id.clone()).collect())
                        .unwrap_or_default();
                    self.r.pick(&acts).map(|a| Command::BotAction {
                        msg: m.id,
                        action: a.clone(),
                    })
                })
            }
            _ => Some(chaos::random_command(&self.g.world, &c, &mut self.r)),
        };
        if let Some(cmd) = cmd {
            let label = format!("{cmd:?}").chars().take(140).collect::<String>();
            let r = self.g.command(&c, cmd);
            self.note(format!("day {label} → {r:?}"));
        }
    }

    // ───────────────────────── the night ─────────────────────────

    fn night_frame(&mut self) {
        let c = self.c.clone();
        let loc = self.loc;
        self.night_frames += 1;
        self.stats.max_frames_night = self.stats.max_frames_night.max(self.night_frames);
        if self.night_frames > 60_000 {
            let why = format!(
                "night {} never ends: {} frames, clock {}, speed {}, frozen {}, panel {:?}, overlay {}",
                self.g.world.night(),
                self.night_frames,
                self.g.world.clock_label(),
                self.g.speed,
                self.g.frozen(),
                self.g.panel,
                self.g.overlay_open()
            );
            self.violation(why);
            self.done = true;
            return;
        }
        let dt = *self.r.pick(&[1.0 / 30.0, 1.0 / 30.0, 0.1, 0.25]).unwrap();
        let before = self.g.world.tick_count() + self.g.world.prep_left_ticks();
        self.g.time += dt;
        // WASD held (or not) as the frame does it (steer_ui)
        steer_ui::input(&mut self.g, &c, false, dt);
        self.g.advance(&c, dt, Some(loc));
        let _ = self.g.hands_frame(&c, dt);
        if self.autoplay {
            self.g.autoplay_step(&c, dt);
        }
        if let Some(mg) = self.g.hands.minigame.as_mut() {
            let input = if self.autoplay {
                mg.game.autoplay_input()
            } else {
                crate::minigame::MgInput {
                    skip: true,
                    ..Default::default()
                }
            };
            if let crate::minigame::MiniGameStatus::Done { quality } = mg.game.step(&input, dt) {
                self.g.minigame_done(&c, quality);
            }
        }
        self.g.wash_input(&c, self.r.chance(0.8), dt);
        if self.r.chance(0.1) {
            self.g.equip_input(&c, self.r.chance(0.5), dt);
        }
        if self.r.chance(self.rate) {
            self.night_action();
        }
        // a frozen night is only allowed while something the harness can close holds it
        let after = self.g.world.tick_count() + self.g.world.prep_left_ticks();
        if after == before && self.g.world.phase() == Phase::Night {
            self.frozen_frames += 1;
            if self.frozen_frames > 900 {
                // like a player: unpause, close whatever is open
                self.note(format!(
                    "unfreeze (speed {}, frozen {}, overlay {}, cutscene {:?})",
                    self.g.speed,
                    self.g.frozen(),
                    self.g.overlay_open(),
                    self.g.world.playing_cutscene()
                ));
                if self.frozen_frames > 4000 {
                    let why = format!(
                        "the night holds still for {} frames at {} (speed {}, frozen {}, \
                         cutscene {:?}/{}, overlay {}, summary {}, walk {}, photo {}, talk {})",
                        self.frozen_frames,
                        self.g.world.clock_label(),
                        self.g.speed,
                        self.g.frozen(),
                        self.g.world.playing_cutscene(),
                        self.g.cutscene_playing(),
                        self.g.overlay_open(),
                        self.g.summary.is_some(),
                        self.g.walk.is_open(),
                        self.g.photo.is_open(),
                        self.g.stories.talk_open()
                    );
                    self.violation(why);
                    self.frozen_frames = 0;
                }
                self.g.set_speed(3);
                self.g.pause = None;
                self.g.panel = None;
                self.g.hands.book = None;
                self.g.hands.order = None;
                self.g.hands.moment = None;
                if let Some(id) = self.g.world.playing_cutscene().map(str::to_string)
                    && !self.g.cutscene_playing()
                {
                    let _ = self.g.command(
                        &c,
                        Command::CutsceneFinished {
                            id,
                            choices: Vec::new(),
                        },
                    );
                }
            }
        } else {
            self.frozen_frames = 0;
        }
        self.stats.ticks += after.saturating_sub(before) as u64;
        if self.stats.frames.is_multiple_of(4) || self.g.world.phase() != Phase::Night {
            self.check();
        }
        if self.g.world.phase() == Phase::Day {
            self.stats.nights += 1;
        }
    }

    fn night_action(&mut self) {
        let c = self.c.clone();
        let w = &self.g.world;
        let guests: Vec<u32> = w.guests().iter().map(|g| g.id).collect();
        let tables: Vec<u32> = w
            .furniture()
            .iter()
            .filter(|p| p.dishes > 0 || w.tips_on_tables().iter().any(|t| t.table == p.uid))
            .map(|p| p.uid)
            .collect();
        let uids: Vec<u32> = w.furniture().iter().map(|f| f.uid).collect();
        let sinks: Vec<u32> = w
            .furniture()
            .iter()
            .filter(|p| p.def.contains("sink"))
            .map(|p| p.uid)
            .collect();
        let dirt = w.dirt_spots();
        let front = self.r.chance(0.25);
        let intent = match self.r.index(48) {
            0..=4 => self
                .r
                .pick(&guests)
                .map(|&guest| hands::Intent::Guest { guest, front }),
            5 | 6 => self
                .r
                .pick(&tables)
                .map(|&table| hands::Intent::Table { table, front }),
            7 => self
                .r
                .pick(&tables)
                .map(|&table| hands::Intent::Tip { table, front }),
            8 => self
                .r
                .pick(&sinks)
                .map(|&sink| hands::Intent::Sink { sink, front }),
            9 => self
                .r
                .pick(&dirt)
                .map(|&tile| hands::Intent::Mop { tile, front }),
            10 => Some(hands::Intent::Window),
            11 => Some(hands::Intent::Jukebox),
            12 | 13 => Some(hands::Intent::Chore { front }),
            14 => Some(hands::Intent::Greet),
            15 => Some(hands::Intent::Cancel),
            16 => Some(hands::Intent::Unqueue(self.r.index(6))),
            17 => self
                .r
                .pick(&uids)
                .map(|&uid| hands::Intent::Equip { uid, front }),
            18 => self.r.pick(&uids).map(|&uid| hands::Intent::Lamp { uid }),
            19 => Some(hands::Intent::EquipHold {
                seconds: self.r.f32() * 2.0,
            }),
            20 | 21 => {
                self.note("rail click".into());
                self.g.act_on_rail(&c);
                None
            }
            22 => {
                // a panel opens or closes (the phone slows the night)
                let p = *self.r.pick(&PANELS).unwrap();
                self.g.panel = if self.g.panel.is_some() || self.r.chance(0.3) {
                    None
                } else {
                    Some(p)
                };
                self.g.ui_state.app_tab = self.r.index(6);
                self.note(format!("panel {:?}", self.g.panel));
                None
            }
            23 => {
                let l = *self.r.pick(&LINKS).unwrap();
                self.note(format!("link {l:?}"));
                self.g.follow_link(l);
                None
            }
            24 => {
                self.note("book".into());
                self.g.open_book(None);
                None
            }
            25..=27 => {
                // the order card: open one, pick something, or close it
                self.order_card();
                None
            }
            28 => {
                if self.g.hands.minigame.is_some() {
                    let q = self.r.f32();
                    self.note(format!("mini-game done q {q:.2}"));
                    self.g.minigame_done(&c, q);
                }
                None
            }
            29 | 30 => {
                let ms: Vec<String> = w
                    .active_moments()
                    .iter()
                    .filter(|m| m.resolving.is_none())
                    .map(|m| m.id.clone())
                    .collect();
                if let Some(id) = self.r.pick(&ms).cloned() {
                    let opened = self.g.open_moment(&id);
                    let opts = self.g.world.moment_options(&c, &id);
                    if let Some(o) = self.r.pick(&opts).cloned()
                        && self.r.chance(0.7)
                    {
                        let ok = self.g.moment_option(&c, &id, &o);
                        self.note(format!("moment {id} open {opened} → {o} {ok}"));
                    } else {
                        self.note(format!("moment {id} open {opened}, left open"));
                    }
                } else if self.g.hands.moment.is_some() {
                    self.g.hands.moment = None;
                    self.note("moment card closed".into());
                }
                None
            }
            31 => {
                let st = w
                    .stories_active()
                    .iter()
                    .find(|s| s.resolving.is_none())
                    .cloned();
                if let Some(s) = st {
                    let opened = self.g.open_story(s.guest);
                    let answers: Vec<String> = self
                        .g
                        .world
                        .story_def(&s.story)
                        .map(|d| d.answers.iter().map(|a| a.id.clone()).collect())
                        .unwrap_or_default();
                    if let Some(a) = self.r.pick(&answers).cloned() {
                        let ok = self.g.answer_story(&c, s.guest, &a);
                        self.note(format!("story {} open {opened} → {a} {ok}", s.story));
                    }
                }
                None
            }
            32 => {
                let cmd = if self.r.chance(0.5) {
                    Command::StormBlankets
                } else {
                    Command::StormSoup
                };
                let r = self.g.command(&c, cmd.clone());
                self.note(format!("{cmd:?} → {r:?}"));
                None
            }
            33 => {
                self.note("photo mode".into());
                photo_mode::open(&mut self.g, self.loc);
                None
            }
            34 => {
                let ok = walk_home::step_out(&mut self.g, &c, None);
                self.note(format!("back step {ok}"));
                None
            }
            35 | 36 => {
                let s = self.r.index(4) as u8;
                if self.r.chance(0.3) {
                    self.g.toggle_pause();
                    self.note(format!("pause → speed {}", self.g.speed));
                } else {
                    self.g.set_speed(s.max(1));
                    self.note(format!("speed {}", s.max(1)));
                }
                None
            }
            37 => {
                if self.g.sig.open {
                    self.g.close_signature();
                } else {
                    self.g.open_signature(&c);
                }
                self.note(format!("signature card {}", self.g.sig.open));
                None
            }
            38 => {
                let hs: Vec<sim::Staff> = duties::helpers(&self.g);
                if let Some(s) = self.r.pick(&hs) {
                    let d = *self.r.pick(&crate::content::handson::Duty::ALL).unwrap();
                    let on = self.r.chance(0.6);
                    let ok = duties::set_duty(&mut self.g, &c, s.id, d, on);
                    self.note(format!("duty {} {d:?} {on} → {ok}", s.id));
                }
                None
            }
            39 => {
                self.save_load("night");
                None
            }
            40 => {
                let r = self.g.world.undo(&c);
                self.note(format!("undo → {r:?}"));
                None
            }
            41 => {
                // build at night: a lamp toggle, a move, a piece
                let uid = self.r.pick(&uids).copied().unwrap_or(0);
                let x = self.r.range_u32(0, 30) as i32;
                let y = self.r.range_u32(0, 16) as i32;
                let moves = self
                    .g
                    .world
                    .group_moves(&c, uid, (x, y), self.r.range_u32(0, 4) as u8);
                let r = self.g.command(&c, Command::MoveGroup { moves });
                self.note(format!("night move {uid} → {x},{y}: {r:?}"));
                None
            }
            42 => {
                let pend = self.g.world.pending_choices().first().cloned();
                if let Some(p) = pend {
                    let ch = self.r.pick(&p.choices).cloned().unwrap_or_default();
                    let r = self.g.command(
                        &c,
                        Command::AnswerEvent {
                            event: p.event.clone(),
                            choice: ch.clone(),
                        },
                    );
                    self.note(format!("event {} → {ch} {r:?}", p.event));
                }
                None
            }
            43 => {
                let hs: Vec<u32> = self
                    .g
                    .world
                    .staff()
                    .iter()
                    .filter(|s| !s.owner)
                    .map(|s| s.id)
                    .collect();
                if let Some(&s) = self.r.pick(&hs) {
                    let cmd = if self.r.chance(0.5) {
                        Command::SendHome { staff: s }
                    } else {
                        Command::SendOnBreak { staff: s }
                    };
                    let r = self.g.command(&c, cmd.clone());
                    self.note(format!("{cmd:?} → {r:?}"));
                }
                None
            }
            46 => {
                // WASD held a while (walking as Karolina), sometimes let go at once
                let d = (self.r.range_i64(-1, 2) as i8, self.r.range_i64(-1, 2) as i8);
                let secs = self.r.f32() * 2.5;
                self.g.hands.steer.fake = Some((d, secs));
                self.note(format!("steer {d:?} {secs:.1}s"));
                None
            }
            47 => {
                let n = self.g.world.nearby(&c);
                let ok = steer_ui::interact(&mut self.g, &c, self.r.chance(0.3));
                self.note(format!("E {:?} → {ok}", n.map(|n| n.kind)));
                None
            }
            _ => {
                let cmd = chaos::random_command(&self.g.world, &c, &mut self.r);
                let label = format!("{cmd:?}").chars().take(140).collect::<String>();
                let r = self.g.command(&c, cmd);
                self.note(format!("night {label} → {r:?}"));
                None
            }
        };
        if let Some(i) = intent {
            let ok = self.g.intent(&c, i);
            self.note(format!("intent {i:?} → {ok}"));
        }
    }

    fn order_card(&mut self) {
        let c = self.c.clone();
        if let Some(o) = self.g.hands.order.clone() {
            match self.r.index(5) {
                0 => {
                    self.g.hands.order = None;
                    self.note("card closed".into());
                }
                1 => {
                    self.g.usual_chosen(&c, o.guest);
                    self.note(format!("usual for {}", o.guest));
                }
                2 => {
                    self.g.hand_off(&c, o.guest);
                    self.note(format!("hand off {}", o.guest));
                }
                _ => {
                    let mut rs: Vec<String> = self
                        .g
                        .world
                        .best_matches(&c, o.guest)
                        .into_iter()
                        .map(|(r, _)| r)
                        .collect();
                    if self.r.chance(0.3) {
                        rs = self.g.world.known_recipes().map(str::to_string).collect();
                    }
                    if let Some(rec) = self.r.pick(&rs).cloned() {
                        let mode = if self.r.chance(0.5) {
                            UiBrew::ByHand
                        } else {
                            UiBrew::Quick
                        };
                        self.g.brew_chosen(&c, o.guest, &rec, mode);
                        self.note(format!("brew {rec} {mode:?} for {}", o.guest));
                    }
                }
            }
        } else {
            let gs: Vec<u32> = self
                .g
                .world
                .parties()
                .iter()
                .filter(|p| p.wish_open)
                .filter_map(|p| p.leader())
                .collect();
            if let Some(&g) = self.r.pick(&gs) {
                self.g.open_card(&c, g);
                self.note(format!("card opened for {g}"));
            }
        }
    }

    /// Save → load: the loaded world must continue exactly like a copy of the original; the
    /// game then goes on with the loaded screen (like Continue).
    fn save_load(&mut self, when: &str) {
        let c = self.c.clone();
        if self.g.cutscene_playing() {
            return;
        }
        self.stats.save_loads += 1;
        let file = match self.g.save_file(&c) {
            Ok(f) => f,
            Err(e) => {
                self.violation(format!("save failed: {e}"));
                return;
            }
        };
        // through the text, like the disk
        let text = match ron::to_string(&file) {
            Ok(t) => t,
            Err(e) => {
                self.violation(format!("save file text: {e}"));
                return;
            }
        };
        let back: crate::save::SaveFile = match ron::from_str(&text) {
            Ok(f) => f,
            Err(e) => {
                self.violation(format!("save file parse: {e}"));
                return;
            }
        };
        let loaded = match GameScreen::from_save(&c, &back) {
            Ok(g) => g,
            Err(e) => {
                self.violation(format!("load failed: {e}"));
                return;
            }
        };
        // determinism: a plain copy and the loaded world, N ticks with the same scene answers
        let mut a = loaded.world.clone();
        let mut b = self.g.world.clone();
        let n = 100 + self.r.index(500);
        for _ in 0..n {
            sim::bot::finish_cutscenes(&mut a, &c);
            sim::bot::finish_cutscenes(&mut b, &c);
            a.tick(&c);
            b.tick(&c);
        }
        let (sa, sb) = (a.save_to_string().unwrap(), b.save_to_string().unwrap());
        if sa != sb {
            let i = sa
                .bytes()
                .zip(sb.bytes())
                .position(|(x, y)| x != y)
                .unwrap_or(0);
            let lo = i.saturating_sub(100);
            let cut = |s: &str| {
                let mut a = lo;
                while !s.is_char_boundary(a) {
                    a -= 1;
                }
                let mut z = (i + 80).min(s.len());
                while !s.is_char_boundary(z) {
                    z -= 1;
                }
                s[a..z].to_string()
            };
            self.violation(format!(
                "{when} save: the loaded world diverged after {n} ticks: …{}… vs …{}…",
                cut(&sa),
                cut(&sb)
            ));
        }
        self.note(format!("save/load ({when})"));
        let autoplay = self.g.hands.autoplay;
        let speed = self.g.speed;
        self.g = loaded;
        self.g.hands.autoplay = autoplay;
        self.g.set_speed(speed.max(1));
        self.g.drain_and_handle(&c, Some(self.loc));
    }

    fn run(&mut self, nights: u32) {
        let start = self.g.world.night();
        let mut guard = 0u64;
        while !self.done && self.g.world.night() < start + nights {
            let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.frame()));
            if let Err(p) = panicked {
                let msg = p
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_default();
                self.violation(format!("PANIC: {msg}"));
                return;
            }
            guard += 1;
            if guard > 3_000_000 {
                self.violation("the run never finishes".into());
                return;
            }
        }
    }
}

fn run_one(
    c: &Rc<Content>,
    loc: &Locale,
    kind: usize,
    seed: u64,
    nights: u32,
    rate: f32,
) -> Vec<String> {
    let mut ch = Chaos::new(c, loc, kind, seed, rate);
    let t0 = std::time::Instant::now();
    ch.run(nights);
    let s = &ch.stats;
    eprintln!(
        "[chaos-screen] kind {kind} seed {seed}: {} nights (now {}), {} frames, {} ticks, {} actions, \
         {} save/loads, endings {}, max frames/night {}, off-floor {}, {:.1}s, {} violations",
        s.nights,
        ch.g.world.night(),
        s.frames,
        s.ticks,
        s.actions,
        s.save_loads,
        s.endings,
        s.max_frames_night,
        s.off_floor,
        t0.elapsed().as_secs_f32(),
        ch.violations.len()
    );
    for o in &s.off_floor_examples {
        eprintln!("    off floor: {o}");
    }
    for v in &ch.violations {
        eprintln!("  !! {v}");
    }
    for (k, n) in &s.kinds {
        eprintln!("[tally] {n} × {k}");
    }
    // the story's 30th night ends the run with exactly one ending
    if ch.g.world.mode() == sim::Mode::Story && ch.done && s.endings != 1 {
        ch.violations.push(format!("{} endings", s.endings));
    }
    ch.violations
        .into_iter()
        .map(|v| format!("kind {kind} seed {seed}: {v}"))
        .collect()
}

fn env_u32(k: &str, d: u32) -> u32 {
    std::env::var(k)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(d)
}

/// The quick run (CI): two nights per mode with a high action rate.
#[test]
fn chaos_quick_screen() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, crate::i18n::Lang::En);
    let mut all = Vec::new();
    for kind in 0..4 {
        all.extend(run_one(&c, &loc, kind, 3 + kind as u64, 2, 0.2));
    }
    assert!(all.is_empty(), "{all:#?}");
}

/// Heavy: `CHAOS_SEEDS` seeds × the modes × `CHAOS_NIGHTS` nights.
#[test]
#[ignore]
fn chaos_heavy_screen() {
    let c = content();
    let i = i18n();
    let loc = Locale::new(&i, crate::i18n::Lang::En);
    let seeds = env_u32("CHAOS_SEEDS", 4);
    let nights = env_u32("CHAOS_NIGHTS", 30);
    let base = env_u32("CHAOS_SEED0", 1000) as u64;
    let only = std::env::var("CHAOS_KIND")
        .ok()
        .and_then(|v| v.parse::<usize>().ok());
    let rate = std::env::var("CHAOS_RATE")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(0.08);
    let mut all = Vec::new();
    for s in 0..seeds as u64 {
        for kind in 0..4 {
            if only.is_some_and(|k| k != kind) {
                continue;
            }
            all.extend(run_one(
                &c,
                &loc,
                kind,
                base + s * 31 + kind as u64,
                nights,
                rate,
            ));
        }
    }
    assert!(
        all.is_empty(),
        "{} violations:\n{}",
        all.len(),
        all.join("\n")
    );
}
