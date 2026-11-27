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

use super::grid;
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
    /// T10: plays the story like an attentive player — answers story choices kindly
    /// ([`attentive_choices`]) and builds what a regular asks for (Filip's socket table, Lori's
    /// stage). `false` = every scene ends on its defaults (no choice picked), as before.
    pub attentive: bool,
    /// 0.2: an attentive bot also builds — what the regulars ask for, a piece from grandma's
    /// attic now and then, the back room once affordable. `false` = a player who never builds
    /// (the story still ends in the light; tested).
    pub build: bool,
    /// v0.2: uses the appliances now and then (`false` = never touches them, the
    /// "a no-equipment player still finishes the story" runs).
    pub equipment: bool,
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
            attentive: true,
            build: true,
            equipment: true,
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
    /// Ticks of the preparation half hour (they run at `prep_tick_scale`).
    pub prep_ticks: u32,
    pub overlay_seconds: f32,
    pub ticks: u32,
    pub max_rail: usize,
    /// Real length of the night at ×1 (minutes): normal ticks × tick length + overlays.
    pub real_minutes: f32,
    /// T5: moments resolved, experiments tried, bot buttons pressed.
    pub moments: u32,
    pub experiments: u32,
    pub buttons: u32,
    /// v0.2: appliance chores done by hand.
    pub equipment: u32,
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
    /// Tonight's optional appliance chores the bot feels like doing (v0.2 equipment): a
    /// player sometimes puts the soup on, sometimes doesn't.
    equip_plan: Vec<super::equipment::EquipAction>,
    /// The night `equip_plan` was rolled for (`step` alone plans too).
    equip_night: u32,
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
            equip_plan: Vec::new(),
            equip_night: 0,
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
        self.finish_scenes(w, content);
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
        if self.cfg.attentive {
            signature_day(w, content, self.cfg.seed);
        }
        if self.cfg.attentive && self.cfg.build {
            build_for_regulars(w, content);
            if !self.cfg.thrifty {
                build_extras(w, content);
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
            self.finish_scenes(w, content);
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
        self.plan_equipment(w.night());
        let mut guard = 0u32;
        while w.phase() == Phase::Night && !w.is_closed() && guard < NIGHT_TICKS * 3 {
            self.finish_scenes(w, content);
            self.step(w, content);
            if w.in_prep() {
                self.cur.prep_ticks += 1;
            }
            w.tick(content);
            self.cur.ticks += 1;
            out.extend(w.drain_events());
            guard += 1;
        }
        self.finish_scenes(w, content);
        out.extend(w.drain_events());
        let tick_s = w.real_seconds_per_tick(content);
        // the preparation half hour runs at `prep_tick_scale` of a tick
        let prep_s = tick_s * content.handson.tuning.prep_tick_scale.clamp(0.3, 1.0);
        let plain = self
            .cur
            .ticks
            .saturating_sub(self.cur.overlay_ticks + self.cur.prep_ticks);
        self.cur.real_minutes = (plain as f32 * tick_s
            + self.cur.prep_ticks as f32 * prep_s
            + self.cur.overlay_seconds)
            / 60.0;
        self.cur.same_kind_max = longest_run(&self.kinds);
        self.nights.push(std::mem::take(&mut self.cur));
        out
    }

    /// Ends playing story scenes: attentive choices, or the defaults.
    pub fn finish_scenes(&mut self, w: &mut World, content: &Content) {
        if self.cfg.attentive {
            finish_cutscenes_attentive(w, content);
        } else {
            finish_cutscenes(w, content);
        }
    }

    /// Before each tick: maybe one decision.
    pub fn step(&mut self, w: &mut World, content: &Content) {
        if w.phase() != Phase::Night || w.playing_cutscene().is_some() {
            return;
        }
        self.cur.max_rail = self.cur.max_rail.max(w.ticket_rail(content).len());
        if self.equip_night != w.night() {
            self.plan_equipment(w.night());
        }
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
        // (the preparation half hour first: the window before the doors open)
        if (w.in_prep() || (1..30).contains(&m))
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
        // holding a hold chore at an appliance (warm up, purge, clean the group head)
        if let Some(c) = &w.player().current
            && let PlayerAct::Use { action, .. } = c.act
            && action.hold()
            && c.stage == 1
            && w.apply(content, Command::EquipmentHold { seconds: 1.0 })
                .is_ok()
        {
            return Some(Self::ticks(w, content, 1.0));
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
            if !self.cfg.thrifty && !w.theme_owned(content, id) && spare(w, content) > price + 9000
            {
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
        // the appliances (v0.2): the machine, the soup pot, the oven, the milk, the till, the bin
        if let Some(t) = self.equipment(w, content, queued) {
            return Some(t);
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
                    // (the last ten minutes before the doors open: whatever the crew hasn't
                    // started, she does herself)
                    crate::content::handson::ChecklistOwner::Either(d) => {
                        !helper_on(w, d) || w.in_prep() && w.prep_minute() >= 20
                    }
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

    /// Rolls tonight's optional appliance chores (separate RNG: the rest of the bot's choices
    /// stay as they were).
    fn plan_equipment(&mut self, night: u32) {
        use super::equipment::EquipAction as A;
        self.equip_night = night;
        if !self.cfg.equipment {
            self.equip_plan.clear();
            return;
        }
        let mut rng = Rng::new(self.cfg.seed ^ 0xE9E9 ^ (night as u64).wrapping_mul(0x9E37));
        self.equip_plan = [
            (A::WarmUp, 0.6),
            (A::SoupOn, 0.7),
            (A::Milk, 0.6),
            (A::Bake, 0.7),
            (A::Stir, 0.8),
            (A::TakeOut, 1.0),
            (A::PullTray, 0.5),
            (A::Service, 1.0),
            (A::CleanUp, 0.5),
            (A::CountTill, 0.7),
            (A::Trash, 0.6),
        ]
        .into_iter()
        .filter(|(_, p)| rng.chance(*p))
        .map(|(a, _)| a)
        .collect();
    }

    /// One appliance chore when it suits (calm hands, the right moment).
    fn equipment(&mut self, w: &mut World, content: &Content, queued: usize) -> Option<u32> {
        use super::equipment::EquipAction as A;
        if queued >= 2 || self.equip_plan.is_empty() {
            return None;
        }
        let m = w.minute();
        let soon_train = content.trains.trains.iter().any(|t| {
            let at = t.arrives.minutes_since_open();
            (0..480).contains(&at) && (m as i32..m as i32 + 10).contains(&at)
        });
        let offers: Vec<(u32, A)> = w
            .furniture()
            .iter()
            .filter_map(|p| w.equipment_action(content, p.uid).map(|a| (p.uid, a)))
            .collect();
        for (uid, a) in offers {
            if !self.equip_plan.contains(&a) {
                continue;
            }
            let now = match a {
                A::WarmUp | A::SoupOn | A::Milk | A::Bake => w.in_prep() || m < 20,
                A::Stir | A::TakeOut => queued == 0,
                A::PullTray => queued == 0 && soon_train,
                A::Service => queued <= 1,
                A::CleanUp | A::CountTill | A::Trash => m >= 430 && queued == 0,
            };
            if now
                && w.apply(content, Command::UseEquipment { uid, action: a })
                    .is_ok()
            {
                self.cur.equipment += 1;
                return Some(0);
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

/// Ends every playing cutscene answering its choices like an attentive, kind player
/// ([`attentive_choices`]).
pub fn finish_cutscenes_attentive(w: &mut World, content: &Content) {
    for _ in 0..64 {
        let Some(id) = w.playing_cutscene().map(str::to_string) else {
            return;
        };
        let choices = content
            .cutscene(&id)
            .map(|cs| {
                let mut out = Vec::new();
                attentive_walk(w, content, &cs.steps, &mut out, 0);
                out
            })
            .unwrap_or_default();
        let _ = w.apply(content, Command::CutsceneFinished { id, choices });
    }
}

/// The choices an attentive player makes in cutscene `id` (indices into each `Choice`'s full
/// option list, in the order shown): among the options offered right now, the one that does
/// the most good for the people in it (trust, reputation), never selling the café or sending
/// someone away; ties go to the first option.
pub fn attentive_choices(w: &World, content: &Content, id: &str) -> Vec<usize> {
    let mut out = Vec::new();
    if let Some(cs) = content.cutscene(id) {
        attentive_walk(w, content, &cs.steps, &mut out, 0);
    }
    out
}

fn attentive_walk(
    w: &World,
    content: &Content,
    steps: &[crate::content::CutsceneStep],
    out: &mut Vec<usize>,
    depth: u32,
) {
    use crate::content::CutsceneStep;
    if depth > 6 {
        return;
    }
    for step in steps {
        match step {
            CutsceneStep::Choice { options } => {
                let best = options
                    .iter()
                    .enumerate()
                    .filter(|(_, o)| w.eval_conditions(content, &o.conditions))
                    .map(|(i, o)| (i, kindness(o)))
                    .fold(None::<(usize, f32)>, |acc, (i, k)| match acc {
                        Some((_, b)) if b >= k => acc,
                        _ => Some((i, k)),
                    });
                if let Some((i, _)) = best {
                    out.push(i);
                }
            }
            CutsceneStep::If {
                conditions,
                then,
                otherwise,
            } => {
                let branch = if w.eval_conditions(content, conditions) {
                    then
                } else {
                    otherwise
                };
                attentive_walk(w, content, branch, out, depth + 1);
            }
            CutsceneStep::Call { cutscene } => {
                if let Some(cs) = content.cutscene(cutscene) {
                    attentive_walk(w, content, &cs.steps, out, depth + 1);
                }
            }
            _ => {}
        }
    }
}

/// How kind a story option is (see [`attentive_choices`]).
fn kindness(o: &crate::content::ChoiceOption) -> f32 {
    use crate::content::Effect;
    let mut k = 0.0;
    for e in &o.effects {
        match e {
            Effect::Trust(_, t) => k += t,
            Effect::Reputation(r) => k += r * 2.0,
            Effect::Money(m) => k += (*m as f32 / 200.0).clamp(-5.0, 0.0),
            Effect::SetVar(_, v) if v == "accepted" || v == "burned" => k -= 100.0,
            _ => {}
        }
    }
    if let Some(f) = &o.set_flag
        && ["_sent_away", "_alone", "_silence", "_woken"]
            .iter()
            .any(|s| f.ends_with(s))
    {
        k -= 50.0;
    }
    k
}

/// An attentive player builds what a regular asks for: a table with a socket once Filip has
/// asked for one (from night 7), a little stage once Lori is invited — with money to spare.
pub fn build_for_regulars(w: &mut World, content: &Content) {
    let has_tag = |w: &World, tag: &str| {
        w.furniture().iter().any(|p| {
            content
                .furniture_def(&p.def)
                .is_some_and(|d| d.has_tag(tag))
        })
    };
    let filip_met = w.night() > 7 && w.story().regulars.contains_key("filip");
    if filip_met && !has_tag(w, "power_socket") {
        place_group(w, content, "table_socket", true);
    }
    if w.flag("lori_invited") && !has_tag(w, "stage") {
        place_group(w, content, "stage_corner", false);
    }
}

/// 0.2 package C: the back room once there's money to spare (10 000 after the price), and the
/// cheapest piece of grandma's attic every fourth day (20 000 to spare) — never the money for
/// the debt payments of the next ten nights.
pub fn build_extras(w: &mut World, content: &Content) {
    // nice-to-haves never touch the money for the debt payments of the next ten nights
    let due: i64 = if w.mode() == Mode::Story {
        (w.night()..w.night() + 10)
            .filter_map(|n| content.story.payment_on(n))
            .map(|p| p.amount)
            .sum::<i64>()
            .min(w.debt().max(0))
    } else {
        0
    };
    let free = w.money() - due;
    let br = w.back_room(content);
    if br.available && free >= br.price as i64 + 10_000 {
        let _ = w.apply(content, Command::OpenBackRoom);
    }
    let attic = w.attic(content);
    let free = w.money() - due;
    // a piece now and then (every fourth day), not one each time there's money to spare
    if w.night() >= 8
        && w.night().is_multiple_of(4)
        && let Some(o) = attic
            .offers
            .iter()
            .filter(|o| !o.owned)
            .min_by_key(|o| (o.price, o.id.clone()))
        && free >= o.price as i64 + 20_000
    {
        let _ = w.apply(content, Command::BuyAtticPiece { id: o.id.clone() });
    }
}

/// Places `def` on the first free hall spot (with chairs on both sides for a table), keeping a
/// free row around it so nothing gets walled in. Needs the price + a 6000 cushion.
fn place_group(w: &mut World, content: &Content, def: &str, chairs: bool) -> bool {
    let Some(d) = content.furniture_def(def) else {
        return false;
    };
    let cost = d.price as i64 + if chairs { 900 } else { 0 };
    if spare(w, content) < cost + 6000 {
        return false;
    }
    let (gw, gh) = (w.grid().width, w.grid().height);
    let (sw, sh) = (d.size.0 as i32, d.size.1 as i32);
    let free = |w: &World, x: i32, y: i32| {
        w.grid().is_indoor_floor(x, y)
            && !w.grid().room(x, y).staff_only()
            && w.grid().object_at(x, y).is_none()
            && w.grid()
                .tile(x, y)
                .is_some_and(|t| t.kind == grid::TileKind::Floor)
    };
    for y in 2..gh - 2 {
        for x in 2..gw - 2 {
            let (x0, x1) = if chairs {
                (x - 1, x + sw)
            } else {
                (x, x + sw - 1)
            };
            let ok = (y - 1..=y + sh).all(|ty| (x0 - 1..=x1 + 1).all(|tx| free(w, tx, ty)));
            if !ok {
                continue;
            }
            if w.apply(
                content,
                Command::PlaceFurniture {
                    def: def.to_string(),
                    x,
                    y,
                    rot: 0,
                },
            )
            .is_err()
            {
                continue;
            }
            if chairs {
                for (cx, rot) in [(x - 1, 3), (x + sw, 1)] {
                    let _ = w.apply(
                        content,
                        Command::PlaceFurniture {
                            def: "chair".into(),
                            x: cx,
                            y,
                            rot,
                        },
                    );
                }
            }
            return true;
        }
    }
    false
}

/// Money the bot may spend on nice-to-haves: on Classic the next debt payment (within a week)
/// is kept aside, like a careful player does; on Cozy a missed payment is only a mild story
/// consequence.
fn spare(w: &World, content: &Content) -> i64 {
    if w.difficulty() != Difficulty::Classic || w.mode() != Mode::Story {
        return w.money();
    }
    let due = (w.night()..w.night() + 7)
        .find_map(|n| content.story.payment_on(n))
        .map(|p| p.amount.min(w.debt().max(0)))
        .unwrap_or(0);
    w.money() - due
}

/// The café's own drink (v0.2 package I): some players make one, a night or three after
/// grandma's note, from what is on the shelf; others never bother. Decided by the seed (the
/// bot's RNG stays untouched, so the rest of the night plays the same).
fn signature_day(w: &mut World, content: &Content, seed: u64) {
    if w.signature().is_some() || !w.signature_unlocked() {
        return;
    }
    let h = super::rng::hash_str(&format!("signature:{seed}"));
    if h.is_multiple_of(3) {
        return;
    }
    let from = if w.mode() == Mode::Sandbox {
        super::signature::data().sandbox_from_night
    } else {
        super::signature::data().from_night
    };
    if w.night() < from + (h / 3 % 3) as u32 {
        return;
    }
    let jars: Vec<String> = w.signature_jars().iter().map(|s| s.to_string()).collect();
    let bases: Vec<(String, Vec<crate::content::handson::MiniGameKind>)> = w
        .signature_bases(content)
        .into_iter()
        .filter(|(_, ready)| *ready)
        .map(|(b, _)| (b.id.clone(), b.styles.clone()))
        .collect();
    for (base, styles) in bases.iter().rev() {
        for (i, a) in jars.iter().enumerate() {
            for b in jars.iter().skip(i + 1) {
                let pick = vec![a.clone(), b.clone()];
                let style = styles[(h as usize / 9) % styles.len()];
                if w.signature_preview(content, base, &pick, style).is_ok()
                    && w.apply(
                        content,
                        Command::ComposeSignature {
                            base: base.clone(),
                            jars: pick,
                            style,
                            name: "Holešovická noc".into(),
                        },
                    )
                    .is_ok()
                {
                    return;
                }
            }
        }
    }
}
