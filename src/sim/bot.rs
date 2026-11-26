//! A scripted player for headless runs and balance checks (design/handson.md §7.2, §9):
//! a calm human who acts once per `react_seconds` real seconds at ×1, takes orders, brews
//! wishes by hand (quality from a configurable distribution; the mini-game and the order card
//! slow the sim by the overlay scale), quick-brews plain tickets nobody else holds, carries,
//! collects tips, clears, washes (hold), mops (swipe), talks to regulars, puts records on,
//! hires by plan and delegates sensibly. T5: it answers night moments, experiments in the
//! pantry (following grandma's margin notes, like someone without a wiki), keeps a polaroid
//! each morning, counts the tip jar, buys and switches decor themes, plans the rozpis with
//! the auto-fill, answers requests and bot buttons, ticks the checklists, picks the
//! specials, and books the weekly payroll with a tip split. It only uses the public `World`
//! API, like the UI.

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
    /// Wishes always by hand. Else by hand for regulars and for wishes in calm moments (≤
    /// `calm_todo` pending things, ≥ `by_hand_gap` game minutes since the last mini-game), the
    /// order card's default deciding (Quick after 3 same-kind games or once the night had
    /// `BY_HAND_SOFT_CAP` mini-games); quick when it's busy. T9: 4 things / 10 minutes (was
    /// 2 / 20) — an engaged player who enjoys the mini-games, 6–14 a night.
    pub always_by_hand: bool,
    pub by_hand_gap: u32,
    pub calm_todo: u32,
    pub seed: u64,
    /// Balance runs: no decor purchases, the tip split stays with the café, no star bonus
    /// (to compare money with runs before the T5 systems).
    pub thrifty: bool,
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
            by_hand_gap: 10,
            calm_todo: 4,
            seed: 1,
            thrifty: false,
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
    /// T5: moments resolved, experiments tried, bot buttons pressed.
    pub moments: u32,
    pub experiments: u32,
    pub buttons: u32,
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
        for _ in helpers..want {
            // the candidate whose hire card covers the chore nobody holds yet (dishes, then the
            // kitchen, the bar…), like someone who just spent two nights doing everything alone
            let held: Vec<Duty> = w
                .staff()
                .iter()
                .filter(|s| !s.owner)
                .filter_map(|s| content.staff_archetype(&s.archetype))
                .filter_map(|a| a.default_duty)
                .collect();
            let rank = |c: &Candidate| {
                content
                    .staff_archetype(&c.staff.archetype)
                    .and_then(|a| a.default_duty)
                    .filter(|d| !held.contains(d))
                    .and_then(|d| NEED_ORDER.iter().position(|x| *x == d))
                    .unwrap_or(NEED_ORDER.len())
            };
            let Some(id) = w
                .candidates()
                .iter()
                .enumerate()
                .min_by_key(|(i, c)| (rank(c), *i))
                .map(|(_, c)| c.id)
            else {
                break;
            };
            if w.apply(content, Command::Hire { candidate: id }).is_err() {
                break;
            }
        }
        self.systems_day(w, content);
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
        // the day's events stay queued: `run_night` returns them with the night's
    }

    /// Hands tonight's crew the duties the way a player does: everyone keeps the duty their hire
    /// card proposed (one click on the card), then the gaps are filled by need
    /// ([`NEED_ORDER`]), each to the free helper with the best skill for it.
    pub fn delegate(&mut self, w: &mut World, content: &Content) {
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
        // the hire card's duty first (unless a colleague tonight already holds it)
        for &id in &tonight {
            let Some(d) = w
                .staff_by_id(id)
                .and_then(|s| content.staff_archetype(&s.archetype))
                .and_then(|a| a.default_duty)
            else {
                continue;
            };
            if !tonight.iter().any(|o| w.staff_duties(*o).contains(&d)) {
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
        for d in NEED_ORDER {
            if tonight.iter().any(|o| w.staff_duties(*o).contains(&d)) {
                continue;
            }
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
        if let Some(t) = self.systems_night(w, content) {
            return Some(t);
        }
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
        // a quiet minute: the moments on the map, then the pantry
        if queued <= 1
            && let Some(t) = self.moment(w, content)
        {
            return Some(t);
        }
        if queued == 0
            && w.karolina_todo(content) == 0
            && self.cur.experiments == 0
            && let Some(set) = self.experiment_set(w, content)
            && w.apply(content, Command::Experiment { ingredients: set })
                .is_ok()
        {
            self.cur.experiments += 1;
            return Some(self.overlay(w, content, 6.0));
        }
        None
    }

    // -----------------------------------------------------------------------------------------
    // T5: moments, pantry, the app, the journal
    // -----------------------------------------------------------------------------------------

    /// The Day phase's small decisions (a minute of planning).
    fn systems_day(&mut self, w: &mut World, content: &Content) {
        // the morning polaroid: mostly the best one, sometimes the second
        if let Some((_, list)) = w.pending_polaroids() {
            let i = if list.len() > 1 && self.rng.chance(0.25) {
                1
            } else {
                0
            };
            let _ = w.apply(content, Command::PickPolaroid { index: i });
        }
        // the tip jar
        if let Some((_, choices)) = w.tip_goal_pending(content)
            && let Some(c) = choices.first().cloned()
        {
            let _ = w.apply(content, Command::PickTipGoal { choice: c });
        }
        // requests: approve most, decline some (a person has a life, and so does the café)
        let swaps: Vec<u32> = w
            .app()
            .swaps
            .iter()
            .filter(|s| s.status == RequestStatus::Pending)
            .map(|s| s.id)
            .collect();
        for id in swaps {
            let approve = self.rng.chance(0.8);
            let _ = w.apply(content, Command::AnswerSwap { id, approve });
        }
        let offs: Vec<u32> = w
            .app()
            .time_off
            .iter()
            .filter(|s| s.status == RequestStatus::Pending)
            .map(|s| s.id)
            .collect();
        for id in offs {
            let _ = w.apply(content, Command::AnswerTimeOff { id, approve: true });
        }
        let early: Vec<u32> = w
            .early_leave_requests()
            .iter()
            .filter(|s| s.status == RequestStatus::Pending)
            .map(|s| s.id)
            .collect();
        for id in early {
            let _ = w.apply(content, Command::AnswerEarlyLeave { id, approve: true });
        }
        self.buttons(w, content);
        // payroll: a split per week (half, then all to the crew, then the café…), a star
        let weeks: Vec<u32> = w
            .app()
            .payroll
            .iter()
            .map(|p| p.week)
            .filter(|wk| !w.payroll_sent(*wk))
            .collect();
        for week in weeks {
            use crate::content::handson::TipSplit;
            let split = if self.cfg.thrifty {
                TipSplit::Cafe
            } else {
                [TipSplit::Half, TipSplit::Crew, TipSplit::Cafe][week as usize % 3]
            };
            let _ = w.apply(content, Command::SetTipSplit { split });
            if !self.cfg.thrifty
                && let Some(star) = w
                    .staff()
                    .iter()
                    .filter(|s| !s.owner)
                    .max_by_key(|s| (s.nights_worked, s.id))
                    .map(|s| s.id)
            {
                let _ = w.apply(content, Command::PayBonus { staff: star });
            }
            let _ = w.apply(content, Command::SendPayroll { week });
        }
        // owed náhradní volno becomes a night off next week
        if let Some(s) = w
            .staff()
            .iter()
            .find(|s| !s.owner && s.toil_hours >= 8.0)
            .map(|s| s.id)
        {
            let wd = (w.weekday() as u8 + 3) % 7;
            let _ = w.apply(
                content,
                Command::GrantToil {
                    staff: s,
                    weekday: wd,
                },
            );
        }
        // decor: buy an unlocked theme with money to spare; the season's own when it's there
        let unlocked: Vec<String> = w.themes_unlocked().iter().cloned().collect();
        for id in &unlocked {
            let price = content
                .handson
                .theme(id)
                .map(|t| t.price as i64)
                .unwrap_or(0);
            if !self.cfg.thrifty && !w.theme_owned(content, id) && w.money() > price + 9000 {
                let _ = w.apply(content, Command::BuyTheme { id: id.clone() });
            }
        }
        let want = if w.theme_owned(content, "winter_lights") && w.garlands_up(content) {
            Some("winter_lights".to_string())
        } else if w.weekday() == crate::content::Weekday::Mon {
            unlocked
                .iter()
                .filter(|t| w.theme_owned(content, t))
                .nth(w.night() as usize / 7 % unlocked.len().max(1))
                .cloned()
        } else {
            None
        };
        if let Some(t) = want
            && w.theme(content) != t
        {
            let _ = w.apply(content, Command::SetTheme { id: t });
        }
        // the rozpis: auto-fill on Mondays (the app plans the rest)
        if w.weekday() == crate::content::Weekday::Mon {
            let _ = w.apply(content, Command::AutoFillRozpis);
        }
        // the specials board: the season's suggestions she knows
        let specials: Vec<String> = w
            .season_phase(content)
            .map(|p| {
                p.specials
                    .iter()
                    .filter(|r| w.known_recipes().any(|k| k == r.as_str()))
                    .take(2)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        let _ = w.apply(content, Command::SetSpecials { recipes: specials });
        // the pantry by day (one try, more on the free early nights)
        let tries = if w.experiment_cost(content) == 0 {
            2
        } else {
            1
        };
        for _ in 0..tries {
            if let Some(set) = self.experiment_set(w, content) {
                let _ = w.apply(content, Command::Experiment { ingredients: set });
            }
        }
    }

    /// Press a button on unanswered bot messages that have one (the first that isn't a
    /// decline; requests are answered in their own screens).
    fn buttons(&mut self, w: &mut World, content: &Content) -> bool {
        let msgs: Vec<(u32, String)> = w
            .app()
            .inbox
            .iter()
            .filter(|m| m.answered.is_none() && m.request.is_none() && m.night + 1 >= w.night())
            .map(|m| (m.id, m.template.clone()))
            .collect();
        let mut any = false;
        for (id, template) in msgs {
            let Some(t) = content.app.bot_template(&template) else {
                continue;
            };
            let mut acts: Vec<String> = t
                .actions
                .iter()
                .filter(|a| a.id != "decline")
                .map(|a| a.id.clone())
                .collect();
            // the bot tries the button that acts first, then a plain one
            acts.sort_by_key(|a| {
                t.actions
                    .iter()
                    .find(|x| &x.id == a)
                    .map(|x| x.handler.is_none())
                    .unwrap_or(true)
            });
            for a in acts {
                if w.apply(content, Command::BotAction { msg: id, action: a })
                    .is_ok()
                {
                    self.cur.buttons += 1;
                    any = true;
                    break;
                }
            }
        }
        any
    }

    /// Night: rituals and checklists, bot buttons, the handover.
    fn systems_night(&mut self, w: &mut World, content: &Content) -> Option<u32> {
        let m = w.minute();
        let queued = w.player().queue.len() + usize::from(w.player().current.is_some());
        if self.buttons(w, content) {
            return Some(self.overlay(w, content, 2.0));
        }
        // a moment about to fade comes first (they are short and never come back tonight)
        // (a glowing marker is one click: a calm player takes it when her hands are free-ish)
        let urgent = w
            .active_moments()
            .iter()
            .any(|a| a.resolving.is_none() && m > a.spawned && (a.expires <= m + 8 || queued <= 1));
        if urgent
            && queued <= 3
            && let Some(t) = self.moment(w, content)
        {
            return Some(t);
        }
        if queued >= 3 {
            return None;
        }
        // opening / closing items nobody else will do
        let window = if m < 25 {
            Some(crate::content::world::ChecklistKind::Opening)
        } else if m >= 430 {
            Some(crate::content::world::ChecklistKind::Closing)
        } else {
            None
        };
        if let Some(list) = window {
            let open: Vec<String> = content
                .app
                .checklists
                .iter()
                .filter(|c| c.list == list)
                .filter(|c| {
                    w.app()
                        .checklist
                        .iter()
                        .any(|s| s.id == c.id && !s.done && s.claim.is_none())
                })
                .filter(|c| match c.owner() {
                    crate::content::handson::ChecklistOwner::Karolina => {
                        !matches!(c.id.as_str(), "light_off_at_six" | "handover_note") || m >= 466
                    }
                    crate::content::handson::ChecklistOwner::Either(d) => !helper_on(w, d),
                })
                .map(|c| c.id.clone())
                .collect();
            for id in open {
                if id == "menu_board" {
                    let s = w.specials().to_vec();
                    if w.apply(content, Command::SetSpecials { recipes: s })
                        .is_ok()
                    {
                        return Some(0);
                    }
                    continue;
                }
                if w.apply(content, Command::TickChecklist { id }).is_ok() {
                    return Some(0);
                }
            }
        }
        None
    }

    /// Resolve a moment on the map (its card is a short overlay): the first option, the
    /// warmer one for the cat.
    fn moment(&mut self, w: &mut World, content: &Content) -> Option<u32> {
        let open: Vec<String> = w
            .active_moments()
            .iter()
            .filter(|a| a.resolving.is_none() && w.minute() > a.spawned)
            .map(|a| a.id.clone())
            .collect();
        for id in open {
            let opts = w.moment_options(content, &id);
            let pick = opts
                .iter()
                .find(|o| matches!(o.as_str(), "ham" | "hang" | "blanket" | "candles" | "app"))
                .or(opts.first())
                .cloned();
            if let Some(option) = pick
                && w.apply(
                    content,
                    Command::ResolveMoment {
                        id: id.clone(),
                        option,
                    },
                )
                .is_ok()
            {
                self.cur.moments += 1;
                return Some(self.overlay(w, content, self.cfg.card_seconds));
            }
        }
        None
    }

    /// A set of jars to try, the way a curious player would: what grandma's margin notes
    /// revealed plus a guess, never the same set twice.
    fn experiment_set(&mut self, w: &World, content: &Content) -> Option<Vec<String>> {
        let jars: Vec<String> = w.pantry().iter().cloned().collect();
        if jars.len() < 2 {
            return None;
        }
        let target = w.margin_note_target(content);
        let mut base: Vec<String> = target
            .as_ref()
            .map(|r| w.revealed_ingredients(content, r))
            .unwrap_or_default();
        base.retain(|i| jars.contains(i));
        base.truncate(3);
        let tried = |set: &[String]| {
            let mut key = set.to_vec();
            key.sort();
            w.knowledge().tried.contains(&key.join("+"))
        };
        // grandma's notes say all but one jar: go through the shelf for the last one
        let need = target
            .as_ref()
            .and_then(|r| content.handson.recipe(r))
            .map(|r| r.ingredients.len())
            .unwrap_or(0);
        if need >= 2 && base.len() == need && !tried(&base) {
            return Some(base);
        }
        if need >= 2 && base.len() + 1 == need {
            for j in &jars {
                if base.contains(j) {
                    continue;
                }
                let mut set = base.clone();
                set.push(j.clone());
                if !tried(&set) {
                    return Some(set);
                }
            }
        }
        for _ in 0..12 {
            let mut set = base.clone();
            let n = 2 + self.rng.index(2);
            while set.len() < n {
                let j = jars[self.rng.index(jars.len())].clone();
                if !set.contains(&j) {
                    set.push(j);
                }
            }
            let mut key = set.clone();
            key.sort();
            if !w.knowledge().tried.contains(&key.join("+")) {
                return Some(set);
            }
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
        let calm = w.karolina_todo(content) <= self.cfg.calm_todo
            && self
                .last_by_hand
                .is_none_or(|t| w.minute().saturating_sub(t) >= self.cfg.by_hand_gap);
        let mode = if self.cfg.always_by_hand || usual.is_some() {
            BrewMode::ByHand
        } else if regular {
            // regulars by hand even late on a busy night (only the anti-repetition rule)
            w.by_hand_streak_mode(content, &recipe)
        } else if calm {
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

/// The chores by how much of Karolina's night they take when nobody holds them: dishes (every
/// table, the sink), the kitchen (the longest preps), the bar, then serving, register, floor.
const NEED_ORDER: [Duty; 6] = [
    Duty::Dishes,
    Duty::Kitchen,
    Duty::Bar,
    Duty::Serving,
    Duty::Register,
    Duty::Floor,
];

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
