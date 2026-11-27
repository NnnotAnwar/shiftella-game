//! The character card (design/v0.2.md §D): small read-only queries that answer "what is this
//! person doing, what comes next, when do they work tonight, why do they feel like this" in a
//! form the UI turns into words (`crew.do.<id>`, `crew.why.<id>`), plus "Send home".
//!
//! Nothing here mutates the world except [`send_home`] (`Command::SendHome`).

use super::duties;
use super::hands::PlayerAct;
use super::staff;
use super::systems::PrefStatus;
use super::types::*;
use super::world::*;
use crate::content::handson::{Duty, PrefKind, Strength};
use crate::content::{Content, Job, Need, TraitEffect, TraitGroup};

/// Whom a task is for: a regular (by id, `regular.<id>.name`) or a guest archetype
/// (`guest.<archetype>.one`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskFor {
    Regular(String),
    Guest(String),
}

/// Why someone stands around.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleWhy {
    /// A helper with no duties: give them one.
    NoDuties,
    /// Their duties have nothing to do right now.
    NothingToDo,
    /// The preparation half hour: the doors are still closed.
    BeforeDoors,
    /// Karolina: nothing queued.
    NothingQueued,
}

/// What a person is doing (or will do), for the card. `id()` → `crew.do.<id>`.
#[derive(Debug, Clone, PartialEq)]
pub enum Activity {
    /// Not on tonight's rozpis.
    OffTonight,
    /// Approved leave tonight (dovolená / náhradní volno).
    OnLeave,
    /// Comes in at this minute (since 22:00).
    Arrives {
        minute: u32,
    },
    Sick,
    WalkedOut,
    GoingHome,
    Idle {
        why: IdleWhy,
    },
    /// A small pastime while the duties have nothing to do (`life.do.<kind>`); `with` = the
    /// first name of the colleague of a chat (a proper noun).
    Pastime {
        kind: super::life::Pastime,
        with: Option<String>,
    },
    /// Resting until this minute.
    Break {
        until: u32,
        eat: bool,
    },
    TakeOrder {
        who: Option<TaskFor>,
        at_register: bool,
    },
    /// Making an item (a helper's ticket, or Karolina's brew with her recipe).
    Make {
        menu: String,
        recipe: Option<String>,
        who: Option<TaskFor>,
    },
    /// Carrying `count` items to a party.
    Serve {
        menu: String,
        count: u32,
        who: Option<TaskFor>,
    },
    ClearTable {
        dishes: u32,
    },
    Wash {
        dishes: u32,
    },
    Mop,
    /// Emptying the bucket under the roof leak.
    Bucket,
    /// A checklist item (`app.checklist.<id>`).
    Checklist {
        id: String,
    },
    /// At an appliance (warm up the machine, stir the soup…): `equip.act.<action>`.
    Equipment {
        action: super::equipment::EquipAction,
    },
    CollectTip,
    /// Putting a record on (`None` = the house playlist).
    Record {
        id: Option<String>,
    },
    /// Walking to a night moment (`moment.<id>.title`).
    Moment {
        id: String,
    },
    /// A pantry experiment at the counter.
    Experiment,
    Walk,
    /// Predicted: the regular break around this minute.
    BreakAt {
        minute: u32,
    },
    /// Predicted: tired or hungry, a break comes soon.
    BreakSoon,
    /// Predicted: the shift ends at this minute.
    HomeAt {
        minute: u32,
    },
    /// Karolina in the day phase.
    Planning,
}

impl Activity {
    pub fn id(&self) -> &'static str {
        match self {
            Activity::OffTonight => "off",
            Activity::OnLeave => "leave",
            Activity::Arrives { .. } => "arrives",
            Activity::Sick => "sick",
            Activity::WalkedOut => "walked_out",
            Activity::GoingHome => "going_home",
            Activity::Idle { why } => match why {
                IdleWhy::NoDuties => "idle_no_duties",
                IdleWhy::NothingToDo => "idle",
                IdleWhy::BeforeDoors => "idle_prep",
                IdleWhy::NothingQueued => "idle_queue",
            },
            Activity::Pastime { .. } => "pastime",
            Activity::Break { eat: true, .. } => "break_eat",
            Activity::Break { .. } => "break",
            Activity::TakeOrder {
                at_register: true, ..
            } => "take_order_register",
            Activity::TakeOrder { .. } => "take_order",
            Activity::Make { .. } => "make",
            Activity::Serve { .. } => "serve",
            Activity::ClearTable { .. } => "clear_table",
            Activity::Wash { .. } => "wash",
            Activity::Mop => "mop",
            Activity::Bucket => "bucket",
            Activity::Checklist { .. } => "checklist",
            Activity::Equipment { .. } => "equipment",
            Activity::CollectTip => "collect_tip",
            Activity::Record { .. } => "record",
            Activity::Moment { .. } => "moment",
            Activity::Experiment => "experiment",
            Activity::Walk => "walk",
            Activity::BreakAt { .. } => "break_at",
            Activity::BreakSoon => "break_soon",
            Activity::HomeAt { .. } => "home_at",
            Activity::Planning => "planning",
        }
    }
    /// The person is working on something (not idle, resting or away).
    pub fn busy(&self) -> bool {
        !matches!(
            self,
            Activity::OffTonight
                | Activity::OnLeave
                | Activity::Arrives { .. }
                | Activity::Sick
                | Activity::WalkedOut
                | Activity::GoingHome
                | Activity::Idle { .. }
                | Activity::Pastime { .. }
                | Activity::Break { .. }
                | Activity::Planning
        )
    }
}

/// A reason behind someone's energy or mood. `id()` → `crew.why.<id>`.
#[derive(Debug, Clone, PartialEq)]
pub enum Feeling {
    // energy
    OnBreak,
    Exhausted,
    Tired,
    Hungry,
    /// They worked last night too (or the rozpis has them back to back).
    AfterYesterday,
    /// A heavy week on the published rozpis (shift-start energy is capped).
    HeavyWeek,
    /// This many hours into the shift.
    LongShift {
        hours: u32,
    },
    /// A trait helps them keep going right now (night owl after midnight…).
    TraitKeepsGoing {
        tr: String,
    },
    /// A trait drains them right now (early bird after 2…).
    TraitDrains {
        tr: String,
    },
    Rested,
    Fresh,
    // mood
    /// A broken preference on this week's rozpis.
    PrefBroken {
        pref: PrefKind,
        need: bool,
    },
    HappyWeek,
    /// Requests of theirs were declined recently.
    Declined,
    /// Their sweetheart is on shift (staff id).
    WithPartner {
        staff: u32,
    },
    /// A friend is on shift (staff id).
    Friend {
        staff: u32,
    },
    /// Someone on shift gets on their nerves (staff id).
    Rival {
        staff: u32,
    },
    /// A trait colours the mood right now (positive or not).
    TraitMood {
        tr: String,
        good: bool,
    },
    Good,
    Fine,
    Down,
}

impl Feeling {
    pub fn id(&self) -> &'static str {
        match self {
            Feeling::OnBreak => "on_break",
            Feeling::Exhausted => "exhausted",
            Feeling::Tired => "tired",
            Feeling::Hungry => "hungry",
            Feeling::AfterYesterday => "after_yesterday",
            Feeling::HeavyWeek => "heavy_week",
            Feeling::LongShift { .. } => "long_shift",
            Feeling::TraitKeepsGoing { .. } => "trait_keeps_going",
            Feeling::TraitDrains { .. } => "trait_drains",
            Feeling::Rested => "rested",
            Feeling::Fresh => "fresh",
            Feeling::PrefBroken { .. } => "pref_broken",
            Feeling::HappyWeek => "happy_week",
            Feeling::Declined => "declined",
            Feeling::WithPartner { .. } => "with_partner",
            Feeling::Friend { .. } => "friend",
            Feeling::Rival { .. } => "rival",
            Feeling::TraitMood { good: true, .. } => "trait_mood_good",
            Feeling::TraitMood { .. } => "trait_mood_bad",
            Feeling::Good => "good",
            Feeling::Fine => "fine",
            Feeling::Down => "down",
        }
    }
    /// A reason that pulls the value down (the card tints it amber).
    pub fn negative(&self) -> bool {
        matches!(
            self,
            Feeling::Exhausted
                | Feeling::Tired
                | Feeling::Hungry
                | Feeling::AfterYesterday
                | Feeling::HeavyWeek
                | Feeling::LongShift { .. }
                | Feeling::TraitDrains { .. }
                | Feeling::PrefBroken { .. }
                | Feeling::Declined
                | Feeling::Rival { .. }
                | Feeling::TraitMood { good: false, .. }
                | Feeling::Down
        )
    }
}

/// Reasons behind the two bars, most relevant first (never empty for a helper).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Feelings {
    pub energy: Vec<Feeling>,
    pub mood: Vec<Feeling>,
}

/// Someone's shift tonight (minutes since 22:00, a late arrival and an early leave included).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShiftTonight {
    pub shift: Shift,
    pub from: u32,
    pub to: u32,
    /// Approved leave tonight (the shift is then `Off`).
    pub leave: Option<LeaveKind>,
}

/// Karolina's to-do broken down (the sum is `World::karolina_todo`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TodoParts {
    /// Orders only she will take (wishes, regulars, no register helper).
    pub orders: u32,
    /// Open wishes waiting for her recipe.
    pub wishes: u32,
    /// Plain rounds nobody at the bar / kitchen will make.
    pub brews: u32,
    /// Ready trays nobody serving will carry.
    pub trays: u32,
    /// Tables with dishes and no dishes helper.
    pub tables: u32,
    /// Dishes waiting at the sink (no dishes helper; counted as one thing).
    pub sink: u32,
    /// The floor needs a mop (no floor helper).
    pub floor: bool,
}

impl TodoParts {
    pub fn total(&self) -> u32 {
        self.orders
            + self.wishes
            + self.brews
            + self.trays
            + self.tables
            + u32::from(self.sink > 0)
            + u32::from(self.floor)
    }
}

fn idx_of(w: &World, id: u32) -> Option<usize> {
    w.staff.iter().position(|s| s.id == id)
}

fn party_for(w: &World, party: u32) -> Option<TaskFor> {
    let p = w.parties.iter().find(|p| p.id == party)?;
    Some(match &p.regular {
        Some(r) => TaskFor::Regular(r.clone()),
        None => TaskFor::Guest(p.archetype.clone()),
    })
}

fn item_info(w: &World, item: u32) -> Option<(&OrderItem, Option<TaskFor>)> {
    let i = w.items.iter().find(|i| i.id == item)?;
    Some((i, party_for(w, i.party)))
}

fn item_recipe(i: &OrderItem) -> Option<String> {
    match &i.origin {
        super::hands::ItemOrigin::Player { recipe, .. } => Some(recipe.clone()),
        _ => None,
    }
}

/// A helper's task in words.
fn task_activity(w: &World, t: &Task) -> Activity {
    match t {
        Task::TakeOrder { party, register } => Activity::TakeOrder {
            who: party_for(w, *party),
            at_register: register.is_some(),
        },
        Task::Prepare { item, .. } => match item_info(w, *item) {
            Some((i, who)) => Activity::Make {
                menu: i.menu.clone(),
                recipe: item_recipe(i),
                who,
            },
            None => Activity::Idle {
                why: IdleWhy::NothingToDo,
            },
        },
        Task::Serve { item, extra, .. } => match item_info(w, *item) {
            Some((i, who)) => Activity::Serve {
                menu: i.menu.clone(),
                count: 1 + extra.len() as u32,
                who,
            },
            None => Activity::Idle {
                why: IdleWhy::NothingToDo,
            },
        },
        Task::ClearTable { table, dishes, .. } => Activity::ClearTable {
            dishes: (*dishes).max(
                w.furniture_by_uid(*table)
                    .map(|p| p.dishes)
                    .unwrap_or_default(),
            ),
        },
        Task::Wash { .. } => Activity::Wash {
            dishes: w.sink_dishes,
        },
        Task::Mop { .. } => Activity::Mop,
        Task::Bucket { .. } => Activity::Bucket,
        Task::Checklist { id, .. } => Activity::Checklist { id: id.clone() },
        Task::Break { until, eat } => Activity::Break {
            until: *until,
            eat: *eat,
        },
        Task::Idle { .. } => Activity::Idle {
            why: IdleWhy::NothingToDo,
        },
        Task::Leave => Activity::GoingHome,
    }
}

/// One of Karolina's acts in words.
fn act_activity(w: &World, a: &PlayerAct) -> Activity {
    match a {
        PlayerAct::TakeOrder { party } => {
            let at_register = w
                .parties
                .iter()
                .find(|p| p.id == *party)
                .is_some_and(|p| p.state == PartyState::Queue);
            Activity::TakeOrder {
                who: party_for(w, *party),
                at_register,
            }
        }
        PlayerAct::Brew { item, .. } => match item_info(w, *item) {
            Some((i, who)) => Activity::Make {
                menu: i.menu.clone(),
                recipe: item_recipe(i),
                who,
            },
            None => Activity::Walk,
        },
        PlayerAct::Serve { item, extra } => match item_info(w, *item) {
            Some((i, who)) => Activity::Serve {
                menu: i.menu.clone(),
                count: 1 + extra.len() as u32,
                who,
            },
            None => Activity::Walk,
        },
        PlayerAct::ClearTable { table } => Activity::ClearTable {
            dishes: w
                .furniture_by_uid(*table)
                .map(|p| p.dishes)
                .unwrap_or_default(),
        },
        PlayerAct::Wash { .. } => Activity::Wash {
            dishes: w.sink_dishes,
        },
        PlayerAct::Mop { .. } => Activity::Mop,
        PlayerAct::CollectTip { .. } => Activity::CollectTip,
        PlayerAct::Checklist { id } => Activity::Checklist { id: id.clone() },
        PlayerAct::Use { action, .. } => Activity::Equipment { action: *action },
        PlayerAct::PlayRecord { record, .. } => Activity::Record { id: record.clone() },
        PlayerAct::GoTo { tile } => match w.sys.moments.walks.iter().find(|j| j.tile == *tile) {
            Some(j) => match &j.kind {
                super::systems::WalkKind::Moment { id, .. } => Activity::Moment { id: id.clone() },
                super::systems::WalkKind::Experiment { .. } => Activity::Experiment,
                super::systems::WalkKind::Story { .. } | super::systems::WalkKind::Storm { .. } => {
                    Activity::Walk
                }
            },
            None => Activity::Walk,
        },
    }
}

/// The shift a person works tonight. By day: the published rozpis for the coming night.
pub(crate) fn shift_tonight(w: &World, idx: usize) -> ShiftTonight {
    let s = &w.staff[idx];
    let leave = s
        .leave_nights
        .iter()
        .find(|(n, _)| *n == w.night)
        .map(|(_, k)| *k);
    let shift = if w.phase == Phase::Night {
        s.shift
    } else if leave.is_some() {
        Shift::Off
    } else {
        w.app
            .published
            .get(&s.id)
            .map(|r| r[w.weekday as usize % 7])
            .unwrap_or(Shift::Full)
    };
    let (from, to) = shift.span().unwrap_or((0, 0));
    let (from, to) = if w.phase == Phase::Night && shift != Shift::Off {
        (from + s.late_minutes, to.min(s.leave_at.max(from)))
    } else {
        (from, to)
    };
    ShiftTonight {
        shift,
        from,
        to,
        leave,
    }
}

/// What the person is doing now.
pub(crate) fn activity(w: &World, idx: usize) -> Activity {
    let s = &w.staff[idx];
    if s.owner {
        if w.phase != Phase::Night {
            return Activity::Planning;
        }
        return match &w.hands.player.current {
            Some(c) => act_activity(w, &c.act),
            None => Activity::Idle {
                why: IdleWhy::NothingQueued,
            },
        };
    }
    match s.status {
        StaffStatus::OffDuty => {
            if s.leave_nights.iter().any(|(n, _)| *n == w.night) {
                Activity::OnLeave
            } else {
                Activity::OffTonight
            }
        }
        StaffStatus::Expected { minute } => Activity::Arrives { minute },
        StaffStatus::Sick => Activity::Sick,
        StaffStatus::WalkedOut => Activity::WalkedOut,
        StaffStatus::Leaving => Activity::GoingHome,
        StaffStatus::OnDuty => match &s.task {
            // a helper with duties fills a free moment (one with none still says "give me one")
            Some(Task::Idle { .. }) if !s.duties.is_empty() && s.life.is_some() => {
                let l = s.life.as_ref().expect("checked");
                let with = l
                    .with
                    .and_then(|id| w.staff.iter().find(|o| o.id == id))
                    .map(|o| o.name.split(' ').next().unwrap_or(&o.name).to_string());
                Activity::Pastime { kind: l.kind, with }
            }
            None | Some(Task::Idle { .. }) => Activity::Idle {
                why: if s.duties.is_empty() {
                    IdleWhy::NoDuties
                } else if w.in_prep() {
                    IdleWhy::BeforeDoors
                } else {
                    IdleWhy::NothingToDo
                },
            },
            Some(t) => task_activity(w, t),
        },
    }
}

/// What a helper will most likely do next (a break that is due, the task their duties offer
/// now, the end of the shift). `None` for people who are away or have nothing coming.
pub(crate) fn next_activity(w: &World, content: &Content, idx: usize) -> Option<Activity> {
    let s = &w.staff[idx];
    if s.owner {
        return w.hands.player.queue.front().map(|a| act_activity(w, a));
    }
    if w.phase != Phase::Night || s.status != StaffStatus::OnDuty {
        return None;
    }
    if matches!(s.task, Some(Task::Leave)) {
        return None;
    }
    let m = w.minute();
    let resting = matches!(s.task, Some(Task::Break { .. }));
    if !resting {
        if s.energy < 32.0 || s.hunger < 28.0 {
            return Some(Activity::BreakSoon);
        }
        let st = shift_tonight(w, idx);
        let full = st.to.saturating_sub(st.from) >= 360;
        let due = st.from + staff::BREAK_AFTER_MINUTES;
        if full && s.breaks_tonight == 0 && due > m && due - m <= 45 {
            return Some(Activity::BreakAt { minute: due });
        }
    }
    if let Some(t) = staff::peek_work(w, content, idx) {
        return Some(task_activity(w, &t));
    }
    if s.leave_at < NIGHT_MINUTES && s.leave_at > m && s.leave_at - m <= 90 {
        return Some(Activity::HomeAt { minute: s.leave_at });
    }
    None
}

fn trait_situation_ok(w: &World, idx: usize, when: &Option<crate::content::Situation>) -> bool {
    when.as_ref()
        .is_none_or(|sit| staff::situation(w, idx, sit))
}

/// Why the bars are where they are (helpers; Karolina gets the energy half on Classic).
pub(crate) fn feelings(w: &World, content: &Content, idx: usize) -> Feelings {
    let s = &w.staff[idx];
    let night = w.phase == Phase::Night;
    let on = night && s.on_duty();
    let mut energy = Vec::new();
    let mut mood = Vec::new();
    // ---- energy
    if on && matches!(s.task, Some(Task::Break { .. })) {
        energy.push(Feeling::OnBreak);
    }
    if s.energy < 25.0 {
        energy.push(Feeling::Exhausted);
    } else if s.energy < 45.0 {
        energy.push(Feeling::Tired);
    }
    if s.hunger < 30.0 && on {
        energy.push(Feeling::Hungry);
    }
    if !s.owner {
        let wd = w.weekday as usize % 7;
        let worked_yesterday = w
            .app
            .published
            .get(&s.id)
            .is_some_and(|r| r[(wd + 6) % 7] != Shift::Off);
        if on && worked_yesterday && s.energy < 75.0 {
            energy.push(Feeling::AfterYesterday);
        }
        let wb = w.sys.rozpis.published_wb.get(&s.id).copied();
        if on && wb.is_some_and(|wb| wb < 60.0) {
            energy.push(Feeling::HeavyWeek);
        }
    }
    if on {
        for tr in &s.traits {
            let Some(d) = content.trait_def(tr) else {
                continue;
            };
            for e in &d.effects {
                if let TraitEffect::NeedRate {
                    need: Need::Energy,
                    mul,
                    when,
                } = e
                    && when.is_some()
                    && trait_situation_ok(w, idx, when)
                {
                    let f = if *mul < 1.0 {
                        Feeling::TraitKeepsGoing { tr: tr.clone() }
                    } else {
                        Feeling::TraitDrains { tr: tr.clone() }
                    };
                    if !energy.contains(&f) {
                        energy.push(f);
                    }
                }
            }
        }
        let hours = s.minutes_worked / 60;
        if hours >= 4 {
            energy.push(Feeling::LongShift { hours });
        }
        if s.breaks_tonight > 0 && s.energy >= 60.0 {
            energy.push(Feeling::Rested);
        }
    }
    if energy.is_empty() {
        energy.push(Feeling::Fresh);
    }
    // a break first, then the negative reasons (they explain a low bar), then the rest
    energy.sort_by_key(|f| (!matches!(f, Feeling::OnBreak), !f.negative()));
    // ---- mood
    if !s.owner {
        let published = super::rozpis::effective(w, content, &w.app.published);
        if let Some(row) = published.get(&s.id) {
            let mut broken: Vec<(PrefKind, bool)> = super::rozpis::prefs_of(w, s.id)
                .into_iter()
                .filter(|p| {
                    super::rozpis::pref_status(w, &published, s.id, row, p) == PrefStatus::Violated
                })
                .map(|p| (p.kind, p.strength == Strength::Need))
                .collect();
            broken.sort_by_key(|(_, need)| !*need);
            for (pref, need) in broken.into_iter().take(1) {
                mood.push(Feeling::PrefBroken { pref, need });
            }
        }
        let week = super::app::week_of(w.night);
        if w.sys.rozpis.happy_week == Some(week) {
            mood.push(Feeling::HappyWeek);
        }
        let recent = w
            .sys
            .rozpis
            .declines
            .get(&s.id)
            .is_some_and(|v| v.iter().any(|n| *n + 7 > w.night));
        if recent {
            mood.push(Feeling::Declined);
        }
        if on {
            if let Some(p) = s.romance_with
                && w.staff.iter().any(|o| o.id == p && o.on_duty())
            {
                mood.push(Feeling::WithPartner { staff: p });
            }
            let mut friend: Option<(f32, u32)> = None;
            let mut rival: Option<(f32, u32)> = None;
            for o in w
                .staff
                .iter()
                .filter(|o| o.id != s.id && !o.owner && o.on_duty())
            {
                if Some(o.id) == s.romance_with {
                    continue;
                }
                let r = w.relationship(s.id, o.id);
                let irritated = s.traits.iter().any(|t| {
                    content
                        .trait_def(t)
                        .is_some_and(|d| d.irritated_by.iter().any(|x| o.has_trait(x)))
                });
                if (r <= -25.0 || (irritated && r < 40.0)) && rival.is_none_or(|(v, _)| r < v) {
                    rival = Some((r, o.id));
                } else if r >= 40.0 && friend.is_none_or(|(v, _)| r > v) {
                    friend = Some((r, o.id));
                }
            }
            if let Some((_, id)) = rival {
                mood.push(Feeling::Rival { staff: id });
            }
            if let Some((_, id)) = friend {
                mood.push(Feeling::Friend { staff: id });
            }
            for tr in &s.traits {
                let Some(d) = content.trait_def(tr) else {
                    continue;
                };
                for e in &d.effects {
                    if let TraitEffect::Mood { add, when } = e
                        && !matches!(
                            when,
                            crate::content::Situation::ShiftStart
                                | crate::content::Situation::BadEvent
                                | crate::content::Situation::MetGuest333
                                | crate::content::Situation::RomanceEnded
                        )
                        && staff::situation(w, idx, when)
                    {
                        let f = Feeling::TraitMood {
                            tr: tr.clone(),
                            good: *add > 0.0,
                        };
                        if !mood.contains(&f) {
                            mood.push(f);
                        }
                    }
                }
                // a grumpy or gloomy nature, even when no situation is active
                if d.group == TraitGroup::Negative
                    && s.mood < 50.0
                    && d.effects
                        .iter()
                        .any(|e| matches!(e, TraitEffect::MoodFloor(_)))
                {
                    let f = Feeling::TraitMood {
                        tr: tr.clone(),
                        good: false,
                    };
                    if !mood.contains(&f) {
                        mood.push(f);
                    }
                }
            }
            if s.energy < 25.0 {
                mood.push(Feeling::Exhausted);
            } else if s.hunger < 25.0 {
                mood.push(Feeling::Hungry);
            }
        }
    }
    mood.push(if s.mood >= 70.0 {
        Feeling::Good
    } else if s.mood < 40.0 {
        Feeling::Down
    } else {
        Feeling::Fine
    });
    // a low mood explains itself with the negative reasons first, a high one with the good
    let low = s.mood < 55.0;
    mood.sort_by_key(|f| f.negative() != low);
    Feelings { energy, mood }
}

/// Mirrors `pacing::todo` thing by thing (a test keeps them equal).
pub(crate) fn todo_parts(w: &World, content: &Content) -> TodoParts {
    let reg = duties::duty_on_floor(w, Duty::Register);
    let serving = duties::duty_on_floor(w, Duty::Serving);
    let bar = duties::duty_on_floor(w, Duty::Bar);
    let kitchen = duties::duty_on_floor(w, Duty::Kitchen);
    let dishes = duties::duty_on_floor(w, Duty::Dishes);
    let floor = duties::duty_on_floor(w, Duty::Floor);
    let mut out = TodoParts::default();
    for p in &w.parties {
        match p.state {
            PartyState::Queue | PartyState::WaitOrder if p.order_claim.is_none() => {
                let helper_can = (!p.order_kind.is_wish() || p.handed_off) && p.regular.is_none();
                if !(helper_can && reg) {
                    out.orders += 1;
                }
            }
            PartyState::WaitFood if p.wish_open && !p.handed_off => out.wishes += 1,
            _ => {}
        }
    }
    let mut rounds: Vec<(u32, &str)> = Vec::new();
    for i in &w.items {
        if i.by.is_some() {
            continue;
        }
        match i.state {
            ItemState::Pending => {
                let Some(mi) = content.menu_item(&i.menu) else {
                    continue;
                };
                let helper = match mi.job {
                    Job::Kitchen => kitchen,
                    _ => bar,
                };
                if !helper && !rounds.contains(&(i.party, mi.station.as_str())) {
                    rounds.push((i.party, mi.station.as_str()));
                    out.brews += 1;
                }
            }
            ItemState::Ready if !serving && !rounds.contains(&(i.party, "tray")) => {
                rounds.push((i.party, "tray"));
                out.trays += 1;
            }
            _ => {}
        }
    }
    if !dishes {
        out.tables = w
            .furniture
            .iter()
            .filter(|p| p.dishes > 0 && p.claim.is_none())
            .count() as u32;
        if w.sink_dishes > 0 && w.sink_claim.is_none() {
            out.sink = w.sink_dishes;
        }
    }
    out.floor = !floor && w.dirt > super::guests::MOP_DIRT;
    out
}

/// Duties no helper holds tonight: Karolina's by default. By day: the helpers on the coming
/// night's rozpis count.
pub(crate) fn duties_left_for_karolina(w: &World) -> Vec<Duty> {
    Duty::ALL
        .iter()
        .copied()
        .filter(|d| {
            if w.phase == Phase::Night {
                !duties::duty_tonight(w, *d)
            } else {
                !w.staff.iter().enumerate().any(|(i, s)| {
                    !s.owner && s.duties.contains(d) && shift_tonight(w, i).shift != Shift::Off
                })
            }
        })
        .collect()
}

/// `Command::SendHome`: a helper on the floor ends their shift now (they walk out on the next
/// tick; wages follow the minutes worked). Tired people are glad of it (mood +4).
pub(crate) fn send_home(w: &mut World, content: &Content, staff: u32) -> Result<(), CommandError> {
    if w.phase != Phase::Night {
        return Err(CommandError::WrongPhase);
    }
    let i = idx_of(w, staff).ok_or_else(|| CommandError::UnknownId(staff.to_string()))?;
    let s = &w.staff[i];
    if s.owner {
        return Err(CommandError::Invalid("Karolina stays till closing".into()));
    }
    if !s.on_duty() {
        return Err(CommandError::Invalid("not on the floor".into()));
    }
    if w.minute() >= NIGHT_MINUTES - 5 {
        return Err(CommandError::Invalid("the night is almost over".into()));
    }
    let tired = s.energy < 35.0;
    w.staff[i].leave_at = w.minute();
    if tired {
        super::logic::change_mood(w, content, i, 4.0, false);
    }
    Ok(())
}

impl World {
    /// What a person is doing now, in a form the card turns into words (`crew.do.<id>`).
    pub fn activity(&self, staff: u32) -> Option<Activity> {
        idx_of(self, staff).map(|i| activity(self, i))
    }
    /// What comes next: Karolina's next queued act; for a helper the break that is due, the
    /// task their duties offer now, or the end of the shift.
    pub fn next_activity(&self, content: &Content, staff: u32) -> Option<Activity> {
        idx_of(self, staff).and_then(|i| next_activity(self, content, i))
    }
    /// Karolina's current act and her queue, in words (current first).
    pub fn karolina_plan(&self) -> Vec<Activity> {
        let p = &self.hands.player;
        p.current
            .iter()
            .map(|c| act_activity(self, &c.act))
            .chain(p.queue.iter().map(|a| act_activity(self, a)))
            .collect()
    }
    /// Tonight's shift (by day: the coming night's published rozpis).
    pub fn shift_tonight(&self, staff: u32) -> Option<ShiftTonight> {
        idx_of(self, staff).map(|i| shift_tonight(self, i))
    }
    /// Reasons behind someone's energy and mood bars, most relevant first.
    pub fn feelings(&self, content: &Content, staff: u32) -> Feelings {
        idx_of(self, staff)
            .map(|i| feelings(self, content, i))
            .unwrap_or_default()
    }
    /// What only Karolina will do right now, thing by thing (sums to `karolina_todo`).
    pub fn karolina_todo_parts(&self, content: &Content) -> TodoParts {
        todo_parts(self, content)
    }
    /// Duties no helper holds tonight (they stay with Karolina).
    pub fn duties_left_for_karolina(&self) -> Vec<Duty> {
        duties_left_for_karolina(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::bot;

    const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data");

    fn content() -> Content {
        Content::load(DATA).unwrap_or_else(|e| panic!("{e}"))
    }

    /// A sandbox café with two helpers on tonight's rozpis, played by the player-bot until
    /// `minutes` after opening. The first seed where both helpers came in.
    fn night_with_helpers(c: &Content, difficulty: Difficulty, minutes: u32) -> World {
        for seed in 1..40 {
            let mut w = World::new_sandbox(
                c,
                SandboxSetup {
                    seed,
                    difficulty,
                    ..Default::default()
                },
            );
            let ids: Vec<u32> = w.candidates().iter().map(|x| x.id).take(2).collect();
            if ids.len() < 2 {
                continue;
            }
            for id in ids {
                w.apply(c, Command::Hire { candidate: id }).unwrap();
            }
            w.start_night(c);
            w.skip_prep(c);
            let mut b = bot::PlayerBot::new(bot::BotConfig {
                hire_nights: Vec::new(),
                seed,
                ..Default::default()
            });
            while w.minute() < minutes && w.phase() == Phase::Night {
                b.step(&mut w, c);
                w.tick(c);
            }
            let _ = w.drain_events();
            if w.staff().iter().filter(|s| !s.owner && s.on_duty()).count() == 2 {
                return w;
            }
        }
        panic!("no seed with both helpers on the floor");
    }

    #[test]
    fn activity_follows_the_task_and_the_status() {
        let c = content();
        let mut w = night_with_helpers(&c, Difficulty::Cozy, 90);
        let k = w.karolina().unwrap().id;
        // Karolina: her current act (or "hands free"), never a helper's status words
        match (w.player().current.is_some(), w.activity(k).unwrap()) {
            (true, a) => assert!(a.busy() || matches!(a, Activity::Walk), "{a:?}"),
            (false, a) => assert_eq!(
                a,
                Activity::Idle {
                    why: IdleWhy::NothingQueued
                }
            ),
        }
        assert_eq!(
            w.karolina_plan().len(),
            w.player().current.iter().count() + w.player().queue.len()
        );
        // every minute for a while: each helper's words match their task
        for _ in 0..60 {
            for _ in 0..TICKS_PER_MINUTE {
                w.tick(&c);
            }
            for s in w.staff().iter().filter(|s| !s.owner) {
                let a = w.activity(s.id).unwrap();
                match (&s.status, &s.task) {
                    (StaffStatus::OnDuty, Some(Task::Wash { .. })) => {
                        assert!(matches!(a, Activity::Wash { .. }))
                    }
                    (StaffStatus::OnDuty, Some(Task::Prepare { .. })) => {
                        assert!(matches!(a, Activity::Make { .. }), "{a:?}")
                    }
                    (StaffStatus::OnDuty, Some(Task::Break { .. })) => {
                        assert!(matches!(a, Activity::Break { .. }))
                    }
                    (StaffStatus::OnDuty, None | Some(Task::Idle { .. })) => {
                        assert!(matches!(
                            a,
                            Activity::Idle { .. } | Activity::Pastime { .. }
                        ))
                    }
                    (StaffStatus::OffDuty, _) => {
                        assert!(matches!(a, Activity::OffTonight | Activity::OnLeave))
                    }
                    _ => {}
                }
                // "next" never repeats the very task they hold
                if let (Some(n), Some(Task::Wash { .. })) = (w.next_activity(&c, s.id), &s.task) {
                    assert!(!matches!(n, Activity::Wash { .. }));
                }
            }
        }
        // a helper with no duties says so
        let Some(h) = w
            .staff()
            .iter()
            .find(|s| !s.owner && s.on_duty())
            .map(|s| s.id)
        else {
            return;
        };
        for d in w.staff_duties(h).to_vec() {
            w.apply(
                &c,
                Command::SetDuty {
                    staff: h,
                    duty: d,
                    on: false,
                },
            )
            .unwrap();
        }
        w.tick(&c);
        let a = w.activity(h).unwrap();
        assert!(
            matches!(
                a,
                Activity::Idle {
                    why: IdleWhy::NoDuties
                } | Activity::Break { .. }
                    | Activity::GoingHome
            ),
            "{a:?}"
        );
        // by day: Karolina plans, helpers are off until tonight
        let mut guard = 0;
        while w.phase() == Phase::Night && guard < 10_000 {
            w.tick(&c);
            guard += 1;
            if let Some(id) = w.playing_cutscene().map(str::to_string) {
                let _ = w.apply(
                    &c,
                    Command::CutsceneFinished {
                        id,
                        choices: Vec::new(),
                    },
                );
            }
        }
        assert_eq!(w.activity(k), Some(Activity::Planning));
        assert_eq!(w.activity(h), Some(Activity::OffTonight));
        assert_eq!(w.next_activity(&c, h), None);
    }

    #[test]
    fn todo_parts_add_up_to_karolinas_todo() {
        let c = content();
        let mut w = night_with_helpers(&c, Difficulty::Cozy, 30);
        // take a helper's duties away so more lands on Karolina
        let h = w.staff().iter().find(|s| !s.owner).unwrap().id;
        for d in w.staff_duties(h).to_vec() {
            let _ = w.apply(
                &c,
                Command::SetDuty {
                    staff: h,
                    duty: d,
                    on: false,
                },
            );
        }
        let mut seen = 0;
        for _ in 0..300 {
            for _ in 0..TICKS_PER_MINUTE {
                w.tick(&c);
            }
            if w.phase() != Phase::Night {
                break;
            }
            let parts = w.karolina_todo_parts(&c);
            assert_eq!(parts.total(), w.karolina_todo(&c), "{parts:?}");
            seen += parts.total();
        }
        assert!(seen > 0, "something waited for Karolina at some point");
    }

    #[test]
    fn shift_tonight_by_day_and_at_night() {
        let c = content();
        let mut w = World::new_sandbox(&c, SandboxSetup::default());
        let id = w.candidates()[0].id;
        w.apply(&c, Command::Hire { candidate: id }).unwrap();
        let h = w.staff().last().unwrap().id;
        let wd = w.weekday() as u8;
        w.apply(
            &c,
            Command::SetShift {
                staff: h,
                weekday: wd,
                shift: Shift::Early,
            },
        )
        .unwrap();
        w.apply(&c, Command::PublishSchedule).unwrap();
        let st = w.shift_tonight(h).unwrap();
        assert_eq!((st.shift, st.from, st.to), (Shift::Early, 0, 240));
        w.start_night(&c);
        let st = w.shift_tonight(h).unwrap();
        assert_eq!(st.shift, Shift::Early);
        let late = w.staff_by_id(h).unwrap().late_minutes;
        assert_eq!((st.from, st.to), (late, 240));
    }

    #[test]
    fn send_home_ends_the_shift_now() {
        let c = content();
        let mut w = World::new_sandbox(&c, SandboxSetup::default());
        let k = w.karolina().unwrap().id;
        assert_eq!(
            w.apply(&c, Command::SendHome { staff: k }),
            Err(CommandError::WrongPhase)
        );
        let mut w = night_with_helpers(&c, Difficulty::Classic, 60);
        assert!(matches!(
            w.apply(&c, Command::SendHome { staff: k }),
            Err(CommandError::Invalid(_))
        ));
        let h = w
            .staff()
            .iter()
            .find(|s| !s.owner && s.on_duty())
            .unwrap()
            .id;
        w.staff.iter_mut().find(|s| s.id == h).unwrap().energy = 20.0;
        let mood = w.staff_by_id(h).unwrap().mood;
        w.apply(&c, Command::SendHome { staff: h }).unwrap();
        assert!(w.staff_by_id(h).unwrap().mood > mood, "tired: glad of it");
        w.tick(&c);
        assert_eq!(w.staff_by_id(h).unwrap().status, StaffStatus::Leaving);
        assert_eq!(w.activity(h), Some(Activity::GoingHome));
        let worked = w.staff_by_id(h).unwrap().minutes_worked;
        for _ in 0..60 * TICKS_PER_MINUTE {
            w.tick(&c);
        }
        let s = w.staff_by_id(h).unwrap();
        assert_eq!(s.status, StaffStatus::OffDuty, "walked home");
        assert_eq!(s.minutes_worked, worked, "no more paid minutes");
        assert!(matches!(
            w.apply(&c, Command::SendHome { staff: h }),
            Err(CommandError::Invalid(_))
        ));
    }

    #[test]
    fn feelings_explain_the_bars() {
        let c = content();
        let mut w = night_with_helpers(&c, Difficulty::Classic, 60);
        let h = w
            .staff()
            .iter()
            .find(|s| !s.owner && s.on_duty() && !matches!(s.task, Some(Task::Break { .. })))
            .unwrap()
            .id;
        let f = w.feelings(&c, h);
        assert!(!f.energy.is_empty() && !f.mood.is_empty());
        {
            let s = w.staff.iter_mut().find(|s| s.id == h).unwrap();
            s.energy = 10.0;
            s.mood = 30.0;
        }
        let f = w.feelings(&c, h);
        assert_eq!(f.energy[0], Feeling::Exhausted);
        assert!(
            f.mood[0].negative(),
            "a low mood leads with a reason why: {:?}",
            f.mood
        );
        // a friend on the same shift shows up in a good mood
        let other = w
            .staff()
            .iter()
            .find(|s| !s.owner && s.id != h && s.on_duty())
            .unwrap()
            .id;
        super::super::staff::add_relationship(&mut w, h, other, 80.0);
        {
            let s = w.staff.iter_mut().find(|s| s.id == h).unwrap();
            s.energy = 90.0;
            s.mood = 85.0;
        }
        let f = w.feelings(&c, h);
        assert!(
            f.mood.contains(&Feeling::Friend { staff: other })
                || f.mood.contains(&Feeling::WithPartner { staff: other }),
            "{:?}",
            f.mood
        );
        assert!(!f.mood[0].negative());
        // Karolina has no colleagues' moods or rozpis reasons (she is the player)
        let k = w.karolina().unwrap().id;
        let f = w.feelings(&c, k);
        assert!(
            f.mood
                .iter()
                .all(|m| matches!(m, Feeling::Good | Feeling::Fine | Feeling::Down)),
            "{:?}",
            f.mood
        );
    }

    #[test]
    fn duties_left_for_karolina_follow_the_helpers() {
        let c = content();
        let mut w = night_with_helpers(&c, Difficulty::Cozy, 30);
        let held: Vec<Duty> = w
            .staff()
            .iter()
            .filter(|s| !s.owner && s.on_duty())
            .flat_map(|s| s.duties.clone())
            .collect();
        let left = w.duties_left_for_karolina();
        for d in Duty::ALL {
            assert_eq!(left.contains(&d), !held.contains(&d), "{d:?}");
        }
        // take every duty back: all six are hers
        for s in w.staff().to_vec().iter().filter(|s| !s.owner) {
            for d in s.duties.clone() {
                w.apply(
                    &c,
                    Command::SetDuty {
                        staff: s.id,
                        duty: d,
                        on: false,
                    },
                )
                .unwrap();
            }
        }
        assert_eq!(w.duties_left_for_karolina(), Duty::ALL.to_vec());
    }
}
