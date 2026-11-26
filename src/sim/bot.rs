//! A scripted player for headless runs and balance checks (design/handson.md §7.2, §9):
//! a calm human who acts once per `react_seconds` real seconds at ×1, takes orders, brews
//! wishes by hand (quality from a configurable distribution; the mini-game and the order card
//! slow the sim by the overlay scale), quick-brews plain tickets nobody else holds, carries,
//! collects tips, clears, washes (hold), mops (swipe), talks to regulars, puts records on,
//! hires by plan and delegates sensibly. It only uses the public `World` API, like the UI.

use super::hands::*;
use super::rng::Rng;
use super::types::*;
use super::world::*;
use crate::content::Content;
use crate::content::handson::Duty;

/// Player-bot settings.
#[derive(Debug, Clone, PartialEq)]
pub struct BotConfig {
    /// Mean mini-game quality and its spread (± around the mean, triangular).
    pub quality: f32,
    pub spread: f32,
    /// Real seconds between two decisions at ×1.
    pub react_seconds: f32,
    /// Real seconds of a mini-game (by hand) and of reading the order card.
    pub minigame_seconds: f32,
    pub card_seconds: f32,
    /// The order card of a plain ticket (just Quick).
    pub plain_card_seconds: f32,
    /// Nights on which one more helper is hired (story; sandbox starts with one).
    pub hire_nights: Vec<u32>,
    /// Wishes always by hand. Else by hand for regulars and for wishes in calm moments (≤ 2
    /// pending things, ≥ `by_hand_gap` game minutes since the last mini-game), the order card's
    /// default deciding (Quick after 3 same-kind games); quick when it's busy.
    pub always_by_hand: bool,
    pub by_hand_gap: u32,
    pub seed: u64,
}

impl Default for BotConfig {
    fn default() -> Self {
        BotConfig {
            quality: 0.75,
            spread: 0.2,
            react_seconds: 3.0,
            minigame_seconds: 12.0,
            card_seconds: 2.5,
            plain_card_seconds: 1.0,
            hire_nights: vec![3, 5, 9],
            always_by_hand: false,
            by_hand_gap: 20,
            seed: 1,
        }
    }
}

/// What the bot did in one night (for the balance report).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BotNight {
    pub night: u32,
    pub decisions: u32,
    pub minigames: u32,
    /// Longest run of the same mini-game kind.
    pub same_kind_max: u32,
    /// Sim ticks that ran under an overlay, and the real seconds those overlays took.
    pub overlay_ticks: u32,
    pub overlay_seconds: f32,
    pub ticks: u32,
    pub max_rail: usize,
    /// Real length of the night at ×1 (minutes): normal ticks × tick length + overlays.
    pub real_minutes: f32,
}

pub struct PlayerBot {
    pub cfg: BotConfig,
    rng: Rng,
    busy: u32,
    record_done: bool,
    last_by_hand: Option<u32>,
    kinds: Vec<String>,
    pub nights: Vec<BotNight>,
    cur: BotNight,
}

impl PlayerBot {
    pub fn new(cfg: BotConfig) -> PlayerBot {
        PlayerBot {
            rng: Rng::new(cfg.seed ^ 0xB07),
            cfg,
            busy: 0,
            record_done: false,
            last_by_hand: None,
            kinds: Vec::new(),
            nights: Vec::new(),
            cur: BotNight::default(),
        }
    }

    fn quality(&mut self) -> f32 {
        let u = self.rng.f32() + self.rng.f32() - 1.0;
        (self.cfg.quality + self.cfg.spread * u).clamp(0.0, 1.0)
    }

    fn ticks(w: &World, content: &Content, real_seconds: f32) -> u32 {
        (real_seconds / w.real_seconds_per_tick(content)).round() as u32
    }

    /// The Day phase: hire by plan (Classic waits for money), delegate free slots, publish.
    pub fn day(&mut self, w: &mut World, content: &Content) {
        finish_cutscenes(w, content);
        let helpers = w.staff().iter().filter(|s| !s.owner).count();
        let want = self
            .cfg
            .hire_nights
            .iter()
            .filter(|n| w.night() >= **n)
            .count()
            + usize::from(w.mode() == Mode::Sandbox);
        if helpers < want {
            let ids: Vec<u32> = w.candidates().iter().map(|c| c.id).collect();
            for id in ids.into_iter().take(want - helpers) {
                let _ = w.apply(content, Command::Hire { candidate: id });
            }
        }
        let _ = w.apply(content, Command::PublishSchedule);
        self.delegate(w, content);
        // tonight's opening record: a starter, by the night
        let recs: Vec<String> = w.records_unlocked().iter().cloned().collect();
        if !recs.is_empty() {
            let id = recs[w.night() as usize % recs.len()].clone();
            let _ = w.apply(
                content,
                Command::PlayRecord {
                    id: Some(id),
                    repeat: true,
                },
            );
        }
        let _ = w.drain_events();
    }

    /// Hands tonight's crew the duties in a sensible order — dishes first (it keeps the hall and
    /// the bar going), then the plain-order chain (bar, serving, register), kitchen, floor — each
    /// to the free helper with the best skill for it.
    pub fn delegate(&mut self, w: &mut World, content: &Content) {
        const ORDER: [Duty; 6] = [
            Duty::Dishes,
            Duty::Bar,
            Duty::Serving,
            Duty::Register,
            Duty::Kitchen,
            Duty::Floor,
        ];
        let wd = w.weekday() as usize;
        let night = w.night();
        let tonight: Vec<u32> = w
            .staff()
            .iter()
            .filter(|s| {
                !s.owner
                    && w.app()
                        .published
                        .get(&s.id)
                        .is_some_and(|r| r[wd] != Shift::Off)
                    && !s.leave_nights.iter().any(|(n, _)| *n == night)
            })
            .map(|s| s.id)
            .collect();
        for &id in &tonight {
            for d in w.staff_duties(id).to_vec() {
                let _ = w.apply(
                    content,
                    Command::SetDuty {
                        staff: id,
                        duty: d,
                        on: false,
                    },
                );
            }
        }
        for d in ORDER {
            let pick = tonight
                .iter()
                .copied()
                .filter(|id| (w.staff_duties(*id).len() as u32) < w.capacity(content, *id))
                .max_by(|a, b| {
                    let sk = |id: u32| {
                        w.staff_by_id(id)
                            .map(|s| s.skill(d.job(), night))
                            .unwrap_or(0.0)
                    };
                    sk(*a).total_cmp(&sk(*b)).then(b.cmp(a))
                });
            if let Some(id) = pick {
                let _ = w.apply(
                    content,
                    Command::SetDuty {
                        staff: id,
                        duty: d,
                        on: true,
                    },
                );
            }
        }
    }

    /// Plays the current night to 06:00 (starting it if it's day). Returns its events.
    pub fn run_night(&mut self, w: &mut World, content: &Content) -> Vec<SimEvent> {
        let mut out = Vec::new();
        if w.phase() == Phase::Day {
            self.day(w, content);
            finish_cutscenes(w, content);
            out.extend(w.drain_events());
            w.start_night(content);
        }
        self.cur = BotNight {
            night: w.night(),
            ..Default::default()
        };
        self.busy = 0;
        self.record_done = false;
        self.last_by_hand = None;
        self.kinds.clear();
        let mut guard = 0u32;
        while w.phase() == Phase::Night && !w.is_closed() && guard < NIGHT_TICKS * 3 {
            finish_cutscenes(w, content);
            self.step(w, content);
            w.tick(content);
            self.cur.ticks += 1;
            out.extend(w.drain_events());
            guard += 1;
        }
        finish_cutscenes(w, content);
        out.extend(w.drain_events());
        let tick_s = w.real_seconds_per_tick(content);
        self.cur.real_minutes = ((self.cur.ticks.saturating_sub(self.cur.overlay_ticks)) as f32
            * tick_s
            + self.cur.overlay_seconds)
            / 60.0;
        self.cur.same_kind_max = longest_run(&self.kinds);
        self.nights.push(std::mem::take(&mut self.cur));
        out
    }

    /// Before each tick: maybe one decision.
    pub fn step(&mut self, w: &mut World, content: &Content) {
        if w.phase() != Phase::Night || w.playing_cutscene().is_some() {
            return;
        }
        self.cur.max_rail = self.cur.max_rail.max(w.ticket_rail(content).len());
        if self.busy > 0 {
            self.busy -= 1;
            return;
        }
        let react = Self::ticks(w, content, self.cfg.react_seconds);
        if let Some(cost) = self.decide(w, content) {
            self.cur.decisions += 1;
            self.busy = react.max(cost);
        } else {
            // look again in a moment
            self.busy = react / 4;
        }
    }

    /// One decision; `Some(ticks busy)` when the bot did something.
    fn decide(&mut self, w: &mut World, content: &Content) -> Option<u32> {
        let m = w.minute();
        w.karolina()?;
        // rituals
        if (1..30).contains(&m)
            && w.app()
                .checklist
                .iter()
                .any(|c| c.id == "light_window" && !c.done && c.claim.is_none())
            && w.apply(
                content,
                Command::TickChecklist {
                    id: "light_window".into(),
                },
            )
            .is_ok()
        {
            return Some(0);
        }
        if m >= 466 {
            for id in ["handover_note", "light_off_at_six"] {
                if w.app()
                    .checklist
                    .iter()
                    .any(|c| c.id == id && !c.done && c.claim.is_none())
                    && w.apply(content, Command::TickChecklist { id: id.into() })
                        .is_ok()
                {
                    return Some(0);
                }
            }
        }
        // holding at the sink
        if let Some(c) = &w.player().current
            && let PlayerAct::Wash { sink } = c.act
            && c.stage == 1
        {
            let secs = (w.sink_dishes() as f32 / super::player::WASH_PER_SECOND).max(0.5);
            if w.apply(
                content,
                Command::PlayerWashProgress {
                    sink,
                    seconds: secs,
                },
            )
            .is_ok()
            {
                return Some(Self::ticks(w, content, secs));
            }
        }
        // an open wish whose order she took: the order card (+ the mini-game)
        // regulars first (their visit windows are short), soonest to leave first
        let mut open: Vec<(u32, bool, u32)> = w
            .parties()
            .iter()
            .filter(|p| p.state == PartyState::WaitFood && p.wish_open && !p.handed_off)
            .filter_map(|p| {
                p.leader()
                    .map(|l| (l, p.regular.is_none(), p.leave_by.unwrap_or(u32::MAX)))
            })
            .collect();
        open.sort_by_key(|x| (x.1, x.2));
        if let Some(&(guest, _, _)) = open.first()
            && w.player().queue.len() < PLAYER_QUEUE_MAX
        {
            return self.order_card(w, content, guest);
        }
        let queued = w.player().queue.len() + usize::from(w.player().current.is_some());
        if queued >= 4 {
            return None;
        }
        // greet a regular who just came in
        if let Some(g) = w
            .parties()
            .iter()
            .filter(|p| p.regular.is_some() && !p.greeted && m.saturating_sub(p.arrived) <= 3)
            .find_map(|p| p.leader())
            && w.apply(content, Command::PlayerGreet { guest: g }).is_ok()
        {
            return Some(0);
        }
        // the record
        if !self.record_done && m >= 2 {
            self.record_done = true;
            if w.music().source == MusicSource::House {
                let recs: Vec<String> = w.records_unlocked().iter().cloned().collect();
                if let Some(id) = recs.get(w.night() as usize % recs.len().max(1)).cloned()
                    && w.apply(
                        content,
                        Command::PlayRecord {
                            id: Some(id),
                            repeat: true,
                        },
                    )
                    .is_ok()
                {
                    return Some(0);
                }
            }
        }
        let rail = w.ticket_rail(content);
        // orders that are hers: wishes and regulars first (longest wait first)
        let mut orders: Vec<&Ticket> = rail
            .iter()
            .filter(|t| t.stage == TicketStage::Order && t.owner == TicketOwner::Nobody)
            .collect();
        orders.sort_by(|a, b| b.wait_frac.total_cmp(&a.wait_frac));
        // a regular's order is done next (shift+click)
        let regular = orders.iter().find(|t| t.regular.is_some()).map(|t| t.party);
        if let Some(party) = regular
            && w.apply(
                content,
                Command::PlayerQueue {
                    act: PlayerAct::TakeOrder { party },
                    front: true,
                },
            )
            .is_ok()
        {
            return Some(0);
        }
        let wishy = |t: &&&Ticket| {
            w.party_of_guest(t.guest)
                .is_some_and(|p| p.order_kind.is_wish() || p.regular.is_some())
        };
        if let Some(t) = orders.iter().find(wishy).or(orders.first())
            && w.apply(content, Command::PlayerTakeOrder { guest: t.guest })
                .is_ok()
        {
            return Some(0);
        }
        // carry what helpers made, when nobody serves
        for t in &rail {
            if let TicketStage::Serve { item, .. } = &t.stage
                && t.owner == TicketOwner::Nobody
                && w.order_items()
                    .iter()
                    .any(|i| i.id == *item && i.state == ItemState::Ready && i.by.is_none())
                && w.apply(
                    content,
                    Command::PlayerQueue {
                        act: PlayerAct::Serve {
                            item: *item,
                            extra: Vec::new(),
                        },
                        front: false,
                    },
                )
                .is_ok()
            {
                return Some(0);
            }
        }
        // plain tickets nobody else will make (or that have waited a while)
        for t in &rail {
            if let TicketStage::Make { item, menu } = &t.stage
                && (t.owner == TicketOwner::Nobody
                    || (matches!(t.owner, TicketOwner::Helper(_))
                        && queued == 0
                        && w.order_items().iter().any(|i| {
                            i.id == *item && i.by.is_none() && m.saturating_sub(i.created) >= 6
                        })))
            {
                let recipe = content
                    .handson
                    .recipes_for_item(menu)
                    .find(|r| w.recipe_available(content, &r.id))
                    .map(|r| r.id.clone());
                if let Some(recipe) = recipe {
                    let mode = w.default_brew_mode(content, false, &recipe);
                    let q = w.quick_quality(content, &recipe);
                    if w.apply(
                        content,
                        Command::PlayerBrew {
                            guest: t.guest,
                            recipe,
                            quality: q,
                            mode,
                        },
                    )
                    .is_ok()
                    {
                        // the order card (plain: the item, Quick) is an overlay too
                        return Some(self.overlay(w, content, self.cfg.plain_card_seconds));
                    }
                }
            }
        }
        // tip coins
        if let Some(table) = w.tips_on_tables().first().map(|t| t.table)
            && !queued_on(
                w,
                |a| matches!(a, PlayerAct::CollectTip { table: x } | PlayerAct::ClearTable { table: x } if *x == table),
            )
            && w.apply(
                content,
                Command::PlayerQueue {
                    act: PlayerAct::CollectTip { table },
                    front: false,
                },
            )
            .is_ok()
        {
            return Some(0);
        }
        // dishes
        if !helper_on(w, Duty::Dishes) {
            let table = w
                .furniture()
                .iter()
                .find(|p| {
                    p.dishes > 0 && p.claim.is_none() && p.seat_by.iter().all(|s| s.is_none())
                })
                .or_else(|| {
                    w.furniture()
                        .iter()
                        .find(|p| p.dishes >= 3 && p.claim.is_none())
                })
                .map(|p| p.uid);
            if let Some(table) = table
                && w.apply(
                    content,
                    Command::PlayerQueue {
                        act: PlayerAct::ClearTable { table },
                        front: false,
                    },
                )
                .is_ok()
            {
                return Some(0);
            }
            if w.sink_dishes() > 0
                && (w.sink_dishes() >= 6 || queued == 0)
                && let Some(sink) = w
                    .furniture()
                    .iter()
                    .find(|p| {
                        content
                            .furniture_def(&p.def)
                            .is_some_and(|d| d.has_tag("sink"))
                    })
                    .map(|p| p.uid)
                && w.apply(
                    content,
                    Command::PlayerQueue {
                        act: PlayerAct::Wash { sink },
                        front: false,
                    },
                )
                .is_ok()
            {
                return Some(0);
            }
        }
        // the floor
        if !helper_on(w, Duty::Floor)
            && w.dirt() > 0.15
            && !queued_on(w, |a| matches!(a, PlayerAct::Mop { .. }))
            && let Some(tile) = w.dirt_spots().first().copied()
            && w.apply(
                content,
                Command::PlayerQueue {
                    act: PlayerAct::Mop { tile },
                    front: false,
                },
            )
            .is_ok()
        {
            return Some(Self::ticks(w, content, 1.0));
        }
        // a regular has something to say
        if queued == 0
            && let Some(g) = w
                .guests()
                .iter()
                .filter(|g| g.leader && !g.gone)
                .map(|g| g.id)
                .find(|g| w.talk_available(content, *g).is_some())
            && w.apply(content, Command::Talk { guest: g }).is_ok()
        {
            return Some(0);
        }
        None
    }

    /// The order card for a wish: the usual if offered, else the best chip; by hand (the
    /// mini-game) unless the anti-repetition default says Quick.
    fn order_card(&mut self, w: &mut World, content: &Content, guest: u32) -> Option<u32> {
        let card = Self::ticks(w, content, self.cfg.card_seconds);
        let scale = w.overlay_scale(content);
        let usual = w.usual_available(content, guest);
        let best = w.best_matches(content, guest).first().cloned();
        let recipe = usual.clone().or(best.map(|b| b.0))?;
        let regular = w.party_of_guest(guest).is_some_and(|p| p.regular.is_some());
        // a calm human: by hand for regulars and in calm moments, quick when it's busy
        let calm = w.karolina_todo(content) <= 2
            && self
                .last_by_hand
                .is_none_or(|t| w.minute().saturating_sub(t) >= self.cfg.by_hand_gap);
        let mode = if self.cfg.always_by_hand || usual.is_some() {
            BrewMode::ByHand
        } else if regular || calm {
            w.default_brew_mode(content, true, &recipe)
        } else {
            BrewMode::Quick
        };
        let q = if mode == BrewMode::ByHand {
            self.quality()
        } else {
            w.quick_quality(content, &recipe)
        };
        let cmd = match (&usual, mode) {
            (Some(_), BrewMode::ByHand) => Command::PlayerUsual { guest, quality: q },
            _ => Command::PlayerBrew {
                guest,
                recipe: recipe.clone(),
                quality: q,
                mode,
            },
        };
        w.apply(content, cmd).ok()?;
        let mut real = self.cfg.card_seconds;
        if mode == BrewMode::ByHand {
            real += self.cfg.minigame_seconds;
            self.cur.minigames += 1;
            self.last_by_hand = Some(w.minute());
            let kind = content
                .handson
                .recipe(&recipe)
                .map(|r| r.playable_minigame(false).0.id().to_string())
                .unwrap_or_default();
            self.kinds.push(kind);
        }
        let _ = (card, scale);
        Some(self.overlay(w, content, real))
    }

    /// An overlay open for `real` seconds slows the sim: the bot is inside it this many ticks.
    fn overlay(&mut self, w: &World, content: &Content, real: f32) -> u32 {
        let sim = Self::ticks(w, content, real * w.overlay_scale(content)).max(1);
        self.cur.overlay_ticks += sim;
        self.cur.overlay_seconds += real;
        sim
    }
}

fn helper_on(w: &World, d: Duty) -> bool {
    w.staff()
        .iter()
        .any(|s| !s.owner && s.on_duty() && s.duties.contains(&d))
}

fn queued_on(w: &World, f: impl Fn(&PlayerAct) -> bool) -> bool {
    w.player().queue.iter().any(&f) || w.player().current.as_ref().is_some_and(|c| f(&c.act))
}

fn longest_run(v: &[String]) -> u32 {
    let mut best = 0;
    let mut run = 0;
    for (i, k) in v.iter().enumerate() {
        if i > 0 && v[i - 1] == *k {
            run += 1;
        } else {
            run = 1;
        }
        best = best.max(run);
    }
    best
}

/// Ends every playing cutscene with no choices (their defaults).
pub fn finish_cutscenes(w: &mut World, content: &Content) {
    for _ in 0..64 {
        let Some(id) = w.playing_cutscene().map(str::to_string) else {
            return;
        };
        let _ = w.apply(
            content,
            Command::CutsceneFinished {
                id,
                choices: Vec::new(),
            },
        );
    }
}
