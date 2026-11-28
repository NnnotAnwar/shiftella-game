//! "Jump to night N": the balance bot (`sim::bot::PlayerBot`, the one `examples/sim_balance`
//! sweeps with) plays the nights before N headless, a few per frame so the panel can show
//! progress, and the player takes over at N's day screen.
//!
//! The nights are played exactly as `GameScreen::fast_forward(N, None, hire)` plays them (the
//! same bot config, one bot for the whole run), so a jump lands on the same world as the
//! screenshot scenes do — and the same seed always lands on the same world.

use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::content::Content;
use crate::save::{SaveFile, SaveMeta, SessionExtra};
use crate::screens::game::GameScreen;
use crate::sim::{self, Phase, World};

/// Nights the bot hires on (as `GameScreen::fast_forward` with `hire`).
pub const HIRE_NIGHTS: [u32; 4] = [3, 5, 9, 14];

/// The story seed a new story uses (`World::new_story`).
pub const STORY_SEED: u64 = 0x5111_F7E1_1A00_2039;

/// What a jump starts from.
#[derive(Debug, Clone, PartialEq)]
pub enum Start {
    /// A new story (`cozy` / `relaxed` (Carefree) / `classic`), this seed.
    Story { difficulty: String, seed: u64 },
    /// A new sandbox run (Endless or the «Challenge»).
    Sandbox {
        difficulty: String,
        seed: u64,
        challenge: bool,
    },
}

/// A jump in progress.
pub struct Jump {
    world: World,
    extra: SessionExtra,
    bot: sim::bot::PlayerBot,
    pub target: u32,
    pub from: u32,
    guard: u32,
}

impl Jump {
    /// A fresh run (opening skipped) to be played to the day of `target`.
    pub fn new(content: &Rc<Content>, start: &Start, target: u32, hire: bool) -> Jump {
        let world = match start {
            Start::Story { difficulty, seed } => World::new_story_seeded(
                content,
                crate::screens::game::world_view::difficulty_from_id(difficulty),
                *seed,
            ),
            Start::Sandbox {
                difficulty,
                seed,
                challenge,
            } => {
                let mut s = sim::SandboxSetup {
                    seed: *seed,
                    difficulty: crate::screens::game::world_view::difficulty_from_id(difficulty),
                    ..Default::default()
                };
                if *challenge {
                    s.options.insert(sim::challenge::OPTION.into(), true);
                }
                World::new_sandbox(content, s)
            }
        };
        let extra = SessionExtra {
            opening_seen: Some(Vec::new()),
            ..Default::default()
        };
        Self::from_world(world, extra, target, hire)
    }

    /// Goes on from a running game's world (its night must be before `target` to play any).
    pub fn from_world(world: World, extra: SessionExtra, target: u32, hire: bool) -> Jump {
        let bot = sim::bot::PlayerBot::new(sim::bot::BotConfig {
            seed: world.seed(),
            hire_nights: if hire {
                HIRE_NIGHTS.to_vec()
            } else {
                Vec::new()
            },
            ..Default::default()
        });
        let from = world.night();
        Jump {
            world,
            extra,
            bot,
            target: target.max(1),
            from,
            guard: 0,
        }
    }

    /// The night being played now.
    pub fn night(&self) -> u32 {
        self.world.night()
    }

    /// 0–1 of the nights to play.
    pub fn progress(&self) -> f32 {
        let span = self.target.saturating_sub(self.from).max(1) as f32;
        (self.world.night().saturating_sub(self.from) as f32 / span).clamp(0.0, 1.0)
    }

    fn playing(&self) -> bool {
        self.world.night() < self.target && self.guard < 400 && !self.world.is_closed()
    }

    /// Plays nights until `budget` has passed (at least one night). True when done.
    pub fn step(&mut self, content: &Rc<Content>, budget: Duration) -> bool {
        let t0 = Instant::now();
        while self.playing() {
            let _ = self.bot.run_night(&mut self.world, content);
            self.guard += 1;
            if t0.elapsed() >= budget {
                break;
            }
        }
        !self.playing()
    }

    /// Plays every night left (tests, scripts).
    pub fn run(&mut self, content: &Rc<Content>) {
        while !self.step(content, Duration::from_secs(3600)) {}
    }

    /// The day screen of the target night (the bot did the day's routine, as
    /// `fast_forward` does), ready for the player. Call when [`Jump::step`] said done.
    pub fn finish(mut self, content: &Rc<Content>) -> Result<GameScreen, String> {
        if self.world.phase() == Phase::Day {
            self.bot.day(&mut self.world, content);
        }
        let _ = self.world.drain_events();
        let meta = SaveMeta {
            mode: "dev".into(),
            night: self.world.night(),
            clock: "day".into(),
            cafe: self.world.cafe_name().to_string(),
            difficulty: String::new(),
            saved_at: 0,
            finished: false,
            money: self.world.money(),
        };
        let file = SaveFile::new(meta, self.extra, self.world.save_to_string()?);
        let mut g = GameScreen::from_save(content, &file)?;
        g.summary = None;
        Ok(g)
    }
}
