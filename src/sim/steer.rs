//! Walking as Karolina (the owner's "walk around the café, not stand like a bot"): WASD /
//! the arrows steer her tile by tile (`Command::PlayerMove`), **E** does the thing beside her
//! ([`World::nearby`] → `Command::PlayerInteract`). Clicking still sends her by herself.
//!
//! * **Steering** keeps to the café's floor: walls, furniture, seats (someone may sit down
//!   there) and people block her, a diagonal never cuts a corner, and she always stops on a
//!   tile's centre — never on a chair, never inside anyone. Pushed against something she
//!   just turns that way (what she faces is what **E** picks first). Her speed is her
//!   auto-walk speed.
//! * **Her queue while steered.** A walk to her current act's first spot is *cancelled*, the
//!   same way as `PlayerCancel` (claims released, nothing half done); a sink she waits at is
//!   left too. While she works or carries a tray she finishes first (the keys wait). Acts
//!   still queued wait while she is steered and [`HOLD_TICKS`] after; a click (any queued
//!   act) or **E** ends that wait at once.
//! * **Interact** queues the act in front, from where she stands (a step at most: the act's
//!   own spot), and lets go of the steering.
//!
//! Everything here is a pure function of the world: the same commands give the same world.

use super::grid;
use super::hands::PlayerAct;
use super::staff::STAFF_SPEED;
use super::types::*;
use super::world::*;
use crate::content::Content;

/// Ticks her queue waits after the last manual step (≈ 1.3 s at ×1).
pub const HOLD_TICKS: u32 = 10;

/// `Command::PlayerMove`.
pub(crate) fn set_dir(w: &mut World, dir: (i8, i8)) -> Result<(), CommandError> {
    if !(-1..=1).contains(&dir.0) || !(-1..=1).contains(&dir.1) {
        return Err(CommandError::Invalid("dir: each part −1..=1".into()));
    }
    if dir != (0, 0) {
        if w.phase != Phase::Night {
            return Err(CommandError::WrongPhase);
        }
        if super::player::karolina_idx(w).is_none_or(|i| !w.staff[i].on_duty()) {
            return Err(CommandError::Invalid(
                "Karolina isn't behind the counter".into(),
            ));
        }
    }
    w.hands.player.steer = dir;
    Ok(())
}

/// Night start / end: not steered.
pub(crate) fn reset(w: &mut World) {
    w.hands.player.steer = (0, 0);
    w.hands.player.steer_hold = 0;
    w.hands.player.interact_at = None;
}

/// The tiles an act is about (the party, the piece), for doing it from beside them.
fn act_tiles(w: &World, content: &Content, act: &PlayerAct) -> Vec<(i32, i32)> {
    match act {
        PlayerAct::TakeOrder { party } => {
            let Some(p) = w.parties.iter().find(|p| p.id == *party) else {
                return Vec::new();
            };
            let mut v: Vec<(i32, i32)> = p
                .members
                .iter()
                .filter_map(|m| w.guests.iter().find(|g| g.id == *m && !g.gone))
                .map(|g| g.tile())
                .collect();
            if p.state == PartyState::Queue
                && let Some(r) = p.register
            {
                v.extend(w.piece_tiles(content, r));
            }
            v
        }
        PlayerAct::ClearTable { table: u }
        | PlayerAct::CollectTip { table: u }
        | PlayerAct::Wash { sink: u }
        | PlayerAct::Use { uid: u, .. } => w.piece_tiles(content, *u),
        PlayerAct::Checklist { id } => {
            let uid = if matches!(id.as_str(), "light_window" | "light_off_at_six") {
                w.window_light(content)
            } else {
                content
                    .app
                    .checklists
                    .iter()
                    .find(|c| &c.id == id)
                    .and_then(|c| c.anchor.as_deref())
                    .and_then(|tag| w.first_with_tag(content, tag))
            };
            uid.map(|u| w.piece_tiles(content, u)).unwrap_or_default()
        }
        PlayerAct::PlayRecord { .. } => super::music::jukebox(w)
            .map(|u| w.piece_tiles(content, u))
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// An act **E** gave her starts where she stands while she still stands there, beside what
/// it's about, on floor (never a seat) — not at the piece's usual spot round the other side.
pub(crate) fn here_goal(
    w: &World,
    content: &Content,
    idx: usize,
    act: &PlayerAct,
) -> Option<(i32, i32)> {
    let (a, spot) = w.hands.player.interact_at.as_ref()?;
    let here = w.staff[idx].tile();
    if !same_act(a, act) || here != *spot {
        return None;
    }
    if !w.grid.walkable(here.0, here.1) || w.grid.is_seat(here.0, here.1) {
        return None;
    }
    act_tiles(w, content, act)
        .iter()
        .any(|t| grid::chebyshev(*t, here) <= 1 && *t != here)
        .then_some(here)
}

/// Her act is only a walk so far (its first spot), or she just stands at the sink: steering
/// cancels it. Working, carrying, handing over: she finishes first.
fn walk_only(w: &World) -> bool {
    w.hands
        .player
        .current
        .as_ref()
        .is_none_or(|c| c.stage == 0 || (matches!(c.act, PlayerAct::Wash { .. }) && c.stage == 1))
}

/// One tick of steering, before the rest of her tick. True = handled (skip the rest).
pub(crate) fn tick(w: &mut World, content: &Content, idx: usize) -> bool {
    let dir = w.hands.player.steer;
    if dir == (0, 0) {
        if w.hands.player.steer_hold == 0 {
            return false;
        }
        w.hands.player.steer_hold -= 1;
        // the step she is on finishes as a walk; standing, the queue waits a moment
        if w.staff[idx].path.is_empty() && w.hands.player.current.is_none() {
            w.staff[idx].pose = Pose::Idle;
            return true;
        }
        return false;
    }
    if !walk_only(w) {
        return false;
    }
    if w.hands.player.current.is_some() {
        super::player::cancel_walk(w, content);
    }
    w.hands.player.steer_hold = HOLD_TICKS;
    step(w, content, idx, dir);
    true
}

/// May she step onto `t` by hand? Floor of the café that the door reaches, no furniture,
/// no seat, nobody standing or sitting there.
pub(crate) fn passable(w: &World, idx: usize, t: (i32, i32)) -> bool {
    let g = &w.grid;
    if !(g.walkable(t.0, t.1)
        && g.reached(t.0, t.1)
        && g.is_indoor_floor(t.0, t.1)
        && !g.is_seat(t.0, t.1)
        && !g.is_taken(t.0, t.1))
    {
        return false;
    }
    let guest = w.guests.iter().any(|x| !x.gone && x.tile() == t);
    let staff = w
        .staff
        .iter()
        .enumerate()
        .any(|(i, s)| i != idx && s.present() && s.tile() == t);
    !guest && !staff
}

/// The next tile from `here` going `dir`: a diagonal only with both sides free (no corner
/// cutting), else sliding along the free side (horizontal first).
pub(crate) fn next_tile(
    w: &World,
    idx: usize,
    here: (i32, i32),
    dir: (i8, i8),
) -> Option<(i32, i32)> {
    let (dx, dy) = (dir.0 as i32, dir.1 as i32);
    let ok = |t: (i32, i32)| passable(w, idx, t);
    if dx != 0 && dy != 0 {
        let (d, h, v) = (
            (here.0 + dx, here.1 + dy),
            (here.0 + dx, here.1),
            (here.0, here.1 + dy),
        );
        if ok(h) && ok(v) && ok(d) {
            return Some(d);
        }
        if ok(h) {
            return Some(h);
        }
        return ok(v).then_some(v);
    }
    let n = (here.0 + dx, here.1 + dy);
    ok(n).then_some(n)
}

/// Moves her one tick's worth along `dir`, tile centre to tile centre.
fn step(w: &mut World, content: &Content, idx: usize, dir: (i8, i8)) {
    let speed = STAFF_SPEED * content.handson.tuning.player_walk_mul;
    let mut budget = speed * grid::TILE;
    // she turns the way she's pushed, also against a wall (E picks what she faces first)
    w.staff[idx].facing = Facing::from_delta(dir.0 as f32, dir.1 as f32);
    // (left of an auto-walk: only the tile she is stepping into)
    w.staff[idx].path.truncate(1);
    let start = w.staff[idx].pos;
    for _ in 0..8 {
        if budget <= 1e-3 {
            break;
        }
        if w.staff[idx].path.is_empty() {
            let here = w.staff[idx].tile();
            let c = grid::tile_center(here);
            let p = w.staff[idx].pos;
            let next = if (c.0 - p.0).abs() + (c.1 - p.1).abs() > 0.01 {
                // off a centre (a cancelled walk stopped her between tiles): onto it first
                Some(here)
            } else {
                next_tile(w, idx, here, dir)
            };
            match next {
                Some(n) => w.staff[idx].path.push(n),
                None => break,
            }
        }
        let next = w.staff[idx].path[0];
        // a piece put there meanwhile: back to her own tile's centre
        if next != w.staff[idx].tile() && !w.grid.walkable(next.0, next.1) {
            let here = w.staff[idx].tile();
            w.staff[idx].path = vec![here];
            continue;
        }
        let s = &mut w.staff[idx];
        let c = grid::tile_center(next);
        let (ddx, ddy) = (c.0 - s.pos.0, c.1 - s.pos.1);
        let d = (ddx * ddx + ddy * ddy).sqrt();
        if d <= budget {
            s.pos = c;
            budget -= d;
            s.path.remove(0);
        } else {
            s.pos.0 += ddx / d * budget;
            s.pos.1 += ddy / d * budget;
            budget = 0.0;
        }
    }
    let s = &mut w.staff[idx];
    let moved = (s.pos.0 - start.0).abs() + (s.pos.1 - start.1).abs() > 0.01;
    s.pose = if moved { Pose::Walk } else { Pose::Idle };
    s.stuck_ticks = 0;
}

// ---------------------------------------------------------------------------------------------
// Interact: what's beside her
// ---------------------------------------------------------------------------------------------

/// What **E** does with the thing beside her ([`World::nearby`]).
#[derive(Debug, Clone, PartialEq)]
pub enum NearKind {
    /// `Command::PlayerInteract { act }`.
    Act(PlayerAct),
    /// The guest's order card (a wish to answer, a plain item to make — her brew follows
    /// from the card as from a click).
    Card { guest: u32 },
    /// Grandma's jukebox: the record picker.
    Records { jukebox: u32 },
    /// The preparation half hour at the door: open the doors now (`Command::OpenDoors`).
    OpenDoors,
}

/// The thing beside Karolina that **E** would do.
#[derive(Debug, Clone, PartialEq)]
pub struct Nearby {
    pub kind: NearKind,
    /// Where the prompt floats (the guest's tile, the piece's top-left tile, the blot).
    pub tile: (i32, i32),
    /// i18n key of the prompt's verb (`walk.do.<…>`, `equip.act.<…>`, `app.checklist.<…>`).
    pub label: String,
    /// She faces it (what she faces wins over everything else in reach).
    pub faced: bool,
}

impl Nearby {
    /// The command **E** sends (`None`: a card / the record picker the app opens).
    pub fn command(&self) -> Option<Command> {
        match &self.kind {
            NearKind::Act(a) => Some(Command::PlayerInteract { act: a.clone() }),
            NearKind::OpenDoors => Some(Command::OpenDoors),
            NearKind::Card { .. } | NearKind::Records { .. } => None,
        }
    }
}

/// Same act and target (a serve's tray aside).
pub(crate) fn same_act(a: &PlayerAct, b: &PlayerAct) -> bool {
    match (a, b) {
        (PlayerAct::Serve { item: x, .. }, PlayerAct::Serve { item: y, .. })
        | (PlayerAct::Brew { item: x, .. }, PlayerAct::Brew { item: y, .. }) => x == y,
        _ => a == b,
    }
}

/// The act is already hers (current or queued).
pub(crate) fn in_hands(w: &World, act: &PlayerAct) -> bool {
    let p = &w.hands.player;
    p.current.iter().any(|c| same_act(&c.act, act)) || p.queue.iter().any(|q| same_act(q, act))
}

fn facing_delta(f: Facing) -> (i32, i32) {
    match f {
        Facing::Down => (0, 1),
        Facing::Up => (0, -1),
        Facing::Left => (-1, 0),
        Facing::Right => (1, 0),
    }
}

impl World {
    /// The tiles a placed piece covers.
    fn piece_tiles(&self, content: &Content, uid: u32) -> Vec<(i32, i32)> {
        let Some(p) = self.furniture_by_uid(uid) else {
            return Vec::new();
        };
        match content.furniture_def(&p.def) {
            Some(d) => grid::footprint(d, p.x, p.y, p.rot),
            None => vec![(p.x, p.y)],
        }
    }

    fn has_tag(&self, content: &Content, uid: u32, tag: &str) -> bool {
        self.furniture_by_uid(uid)
            .and_then(|p| content.furniture_def(&p.def))
            .is_some_and(|d| d.has_tag(tag))
    }

    /// Everything within Karolina's reach that **E** could do, best first: what she faces,
    /// then guests (take the order / their card / serve what a helper made), a station with
    /// a plain order waiting for it, tables (cups, coins), the sink, an appliance's chore, a
    /// ritual item, a dirt blot, the doors (the preparation), the jukebox — nearer first.
    /// In reach = on her tile or one of the eight around it. Only at night, Karolina on the
    /// floor; acts already hers are left out.
    pub fn nearby_all(&self, content: &Content) -> Vec<Nearby> {
        let Some(k) = self.karolina() else {
            return Vec::new();
        };
        if self.phase != Phase::Night || !k.on_duty() {
            return Vec::new();
        }
        let me = k.id;
        let here = k.tile();
        let fd = facing_delta(k.facing);
        let front = (here.0 + fd.0, here.1 + fd.1);
        // (priority, distance, faced, nearby)
        let mut out: Vec<(u8, i32, Nearby)> = Vec::new();
        let mut add = |prio: u8, tiles: &[(i32, i32)], kind: NearKind, label: String| {
            let near: Vec<&(i32, i32)> = tiles
                .iter()
                .filter(|t| grid::chebyshev(**t, here) <= 1)
                .collect();
            let Some(&&first) = near.first() else {
                return;
            };
            if let NearKind::Act(a) = &kind
                && in_hands(self, a)
            {
                return;
            }
            let faced = tiles.contains(&front);
            let tile = if faced { front } else { first };
            let dist = near
                .iter()
                .map(|t| grid::manhattan(**t, here))
                .min()
                .unwrap_or(0);
            out.push((
                prio,
                dist,
                Nearby {
                    kind,
                    tile,
                    label,
                    faced,
                },
            ));
        };
        let label = |s: &str| format!("walk.do.{s}");

        // ── guests: one thing per party, as a click on them does ──
        for p in &self.parties {
            let mut tiles: Vec<(i32, i32)> = p
                .members
                .iter()
                .filter_map(|m| self.guests.iter().find(|g| g.id == *m && !g.gone))
                .map(|g| g.tile())
                .collect();
            let leader = p.leader().unwrap_or(p.id);
            match p.state {
                PartyState::Queue | PartyState::WaitOrder => {
                    if p.order_claim.is_some() || !super::guests::settled(self, p) {
                        continue;
                    }
                    if p.state == PartyState::Queue
                        && let Some(r) = p.register
                    {
                        tiles.extend(self.piece_tiles(content, r));
                    }
                    add(
                        0,
                        &tiles,
                        NearKind::Act(PlayerAct::TakeOrder { party: p.id }),
                        label("take_order"),
                    );
                }
                PartyState::WaitFood | PartyState::Eating => {
                    let open = self
                        .items
                        .iter()
                        .filter(|i| {
                            i.party == p.id
                                && i.state == ItemState::Pending
                                && i.by.is_none()
                                && matches!(i.origin, super::hands::ItemOrigin::Unassigned)
                        })
                        .min_by_key(|i| i.id);
                    let ready = self
                        .items
                        .iter()
                        .find(|i| i.party == p.id && i.state == ItemState::Ready && i.by.is_none());
                    if p.wish_open {
                        add(0, &tiles, NearKind::Card { guest: leader }, label("order"));
                    } else if let Some(i) = open {
                        add(0, &tiles, NearKind::Card { guest: i.guest }, label("order"));
                    } else if let Some(i) = ready {
                        // beside them or beside the cups on the counter
                        tiles.extend(i.ready_tile);
                        add(
                            0,
                            &tiles,
                            NearKind::Act(PlayerAct::Serve {
                                item: i.id,
                                extra: Vec::new(),
                            }),
                            label("serve"),
                        );
                    }
                }
                _ => {}
            }
        }

        // ── the pieces ──
        let ready_stations: Vec<(u32, u32, String)> = self
            .items
            .iter()
            .filter(|i| {
                i.state == ItemState::Pending
                    && i.by.is_none()
                    && matches!(i.origin, super::hands::ItemOrigin::Unassigned)
                    && self.parties.iter().any(|p| {
                        p.id == i.party
                            && matches!(p.state, PartyState::WaitFood | PartyState::Eating)
                    })
            })
            .filter_map(|i| {
                content
                    .menu_item(&i.menu)
                    .map(|m| (i.id, i.guest, m.station.clone()))
            })
            .collect();
        let window = self.window_light(content);
        let closing = self.closing_time();
        for p in &self.furniture {
            let uid = p.uid;
            let tiles = self.piece_tiles(content, uid);
            if !tiles.iter().any(|t| grid::chebyshev(*t, here) <= 1) {
                continue;
            }
            // a station with a plain order for it: that guest's card (the oldest order)
            if let Some((_, guest, _)) = ready_stations
                .iter()
                .filter(|(_, _, st)| self.has_tag(content, uid, st))
                .min_by_key(|(id, _, _)| *id)
                && p.user.is_none_or(|u| u == me)
            {
                add(1, &tiles, NearKind::Card { guest: *guest }, label("brew"));
            }
            if self.has_tag(content, uid, "table") && p.claim.is_none_or(|c| c == me) {
                let tip = self.hands.tips.iter().any(|t| t.table == uid);
                if p.dishes > 0 {
                    add(
                        2,
                        &tiles,
                        NearKind::Act(PlayerAct::ClearTable { table: uid }),
                        label("clear"),
                    );
                } else if tip {
                    add(
                        2,
                        &tiles,
                        NearKind::Act(PlayerAct::CollectTip { table: uid }),
                        label("tip"),
                    );
                }
            }
            if self.has_tag(content, uid, "sink")
                && self.sink_dishes > 0
                && self.sink_claim.is_none_or(|c| c == me)
                && !in_hands_kind(self, |a| matches!(a, PlayerAct::Wash { .. }))
            {
                add(
                    3,
                    &tiles,
                    NearKind::Act(PlayerAct::Wash { sink: uid }),
                    label("wash"),
                );
            }
            if let Some(action) = self.equipment_action(content, uid) {
                add(
                    4,
                    &tiles,
                    NearKind::Act(PlayerAct::Use { uid, action }),
                    format!("equip.act.{}", action.id()),
                );
            }
            // ritual items on this piece (the window lamp's light; the record is the
            // jukebox's)
            for it in &content.app.checklists {
                let on_piece = match it.id.as_str() {
                    "record_on" => false,
                    "light_window" => window == Some(uid) && !self.window_light_on(content),
                    "light_off_at_six" => window == Some(uid) && closing,
                    // the handover note waits for closing time
                    "handover_note" if self.minute() < 466 && !self.in_closing() => false,
                    _ => it
                        .anchor
                        .as_deref()
                        .is_some_and(|tag| self.first_with_tag(content, tag) == Some(uid)),
                };
                let open = self
                    .app
                    .checklist
                    .iter()
                    .any(|c| c.id == it.id && !c.done && c.claim.is_none());
                if on_piece && open && self.checklist_item_open(content, &it.id) {
                    add(
                        5,
                        &tiles,
                        NearKind::Act(PlayerAct::Checklist { id: it.id.clone() }),
                        format!("app.checklist.{}", it.id),
                    );
                }
            }
            if self.has_tag(content, uid, "music") {
                add(
                    8,
                    &tiles,
                    NearKind::Records { jukebox: uid },
                    label("records"),
                );
            }
        }
        // ── dirt blots ──
        if self.dirt > 0.0 {
            for t in self.dirt_spots() {
                if self.grid.walkable(t.0, t.1) && !self.grid.room(t.0, t.1).staff_only() {
                    add(
                        6,
                        &[t],
                        NearKind::Act(PlayerAct::Mop { tile: t }),
                        label("mop"),
                    );
                }
            }
        }
        // ── the doors in the preparation half hour ──
        if self.in_prep() {
            let doors: Vec<(i32, i32)> = (here.1 - 1..=here.1 + 1)
                .flat_map(|y| (here.0 - 1..=here.0 + 1).map(move |x| (x, y)))
                .filter(|t| {
                    self.grid
                        .tile(t.0, t.1)
                        .is_some_and(|i| i.kind == grid::TileKind::Door)
                })
                .collect();
            add(7, &doors, NearKind::OpenDoors, label("open"));
        }
        out.sort_by_key(|(prio, dist, n)| (!n.faced, *prio, *dist));
        out.into_iter().map(|(_, _, n)| n).collect()
    }

    /// The best thing within Karolina's reach for **E** (see [`World::nearby_all`]).
    pub fn nearby(&self, content: &Content) -> Option<Nearby> {
        self.nearby_all(content).into_iter().next()
    }

    /// The first placed piece with `tag` (the piece a ritual item sits on).
    fn first_with_tag(&self, content: &Content, tag: &str) -> Option<u32> {
        self.furniture
            .iter()
            .find(|p| {
                content
                    .furniture_def(&p.def)
                    .is_some_and(|d| d.has_tag(tag))
            })
            .map(|p| p.uid)
    }

    /// Could Karolina step onto `t` by hand now ([`passable`])?
    pub fn can_step_to(&self, t: (i32, i32)) -> bool {
        super::player::karolina_idx(self).is_some_and(|i| passable(self, i, t))
    }

    /// The direction she is steered (`Command::PlayerMove`), `(0, 0)` = not.
    pub fn steering(&self) -> (i8, i8) {
        self.hands.player.steer
    }

    /// She was steered a moment ago (her queue still waits; the camera keeps following).
    pub fn steered_lately(&self) -> bool {
        self.hands.player.steer != (0, 0) || self.hands.player.steer_hold > 0
    }
}

fn in_hands_kind(w: &World, f: impl Fn(&PlayerAct) -> bool) -> bool {
    let p = &w.hands.player;
    p.current.iter().any(|c| f(&c.act)) || p.queue.iter().any(&f)
}
