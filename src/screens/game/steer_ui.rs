//! Walking as Karolina (`sim::steer`) in the café: at night **WASD / the arrows** steer her
//! and the camera follows her; **E** does the thing beside her, with a small "E · Take the
//! order" prompt over it. The free camera keeps the right / middle drag, the wheel and
//! **Shift + WASD**; moving it lets her go off-screen until she is steered again, Home shows
//! the whole café (and stops the follow the same way), and the follow key (default **Z**)
//! turns the follow off / on. By day, in build mode, cutscenes, the walk home and the menus
//! WASD stays the camera.

use macroquad::prelude::*;

use super::*;
use crate::sim::{NearKind, Nearby};
use crate::ui::{Align, Glass, RectExt, Weight};

/// Per-session state of the walking controls.
#[derive(Debug, Default)]
pub struct SteerUi {
    /// The follow key turned the follow off (Settings-free, this session).
    pub follow_off: bool,
    /// The player moved the camera since she was last steered: it stays where they put it.
    pub cam_free: bool,
    /// E let go of the steering while a key was still down: no steering until all are up.
    pub latch: bool,
    /// Screenshot runs (`SHIFTELLA_SCENE=walk`) and the screen chaos: a held direction and
    /// the real seconds it stays held.
    pub fake: Option<((i8, i8), f32)>,
    /// The direction sent last frame.
    pub last: (i8, i8),
    /// Screenshot runs: zoom the camera in this much once (the follow is visible then).
    pub zoom: Option<f32>,
}

/// WASD / the arrows are hers now (else they are the camera's).
pub fn keys_are_hers(g: &GameScreen) -> bool {
    g.world.phase() == Phase::Night
        && g.world.karolina().is_some_and(|k| k.on_duty())
        && !g.cutscene_playing()
        && g.world.playing_cutscene().is_none()
        && g.panel.is_none()
        && g.pause.is_none()
        && g.summary.is_none()
        && !g.walk.is_open()
        && !g.photo.is_open()
        && !g.stories.talk_open()
        && g.stories.card.is_none()
        && !g.hands.overlay()
        && !g.hands.records_open
        && !g.hands.jar_open
        && !g.sig.open
        && !g.rituals.wants_keys()
        && g.reading.is_empty()
}

fn shift() -> bool {
    is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift)
}

/// The direction the keys hold now (Shift + WASD is the camera's). Needs the window.
pub fn held() -> (i8, i8) {
    if shift() {
        return (0, 0);
    }
    let k = |a: KeyCode, b: KeyCode| is_key_down(a) || is_key_down(b);
    let mut d = (0i8, 0i8);
    if k(KeyCode::W, KeyCode::Up) {
        d.1 -= 1;
    }
    if k(KeyCode::S, KeyCode::Down) {
        d.1 += 1;
    }
    if k(KeyCode::A, KeyCode::Left) {
        d.0 -= 1;
    }
    if k(KeyCode::D, KeyCode::Right) {
        d.0 += 1;
    }
    d
}

/// Does the camera take WASD this frame? (Not while they steer her.)
pub fn camera_keys(g: &GameScreen, ui: &Ui) -> bool {
    !(keys_are_hers(g) && !ui.wants_keyboard() && !g.automation) || shift()
}

/// Sends the held direction to the sim (before the frame's ticks). `keyboard` = nothing else
/// has the keys this frame.
pub fn input(g: &mut GameScreen, content: &Content, keyboard: bool, dt: f32) {
    let mut dir = (0, 0);
    if let Some((d, left)) = &mut g.hands.steer.fake {
        *left -= dt;
        dir = if *left > 0.0 { *d } else { (0, 0) };
    } else if keyboard && !g.automation {
        dir = held();
    }
    if !keys_are_hers(g) {
        dir = (0, 0);
    }
    if g.hands.steer.latch {
        if dir == (0, 0) {
            g.hands.steer.latch = false;
        }
        dir = (0, 0);
    }
    if dir != (0, 0) && g.hands.steer.last == (0, 0) {
        // a new walk: the camera comes back to her
        g.hands.steer.cam_free = false;
    }
    g.hands.steer.last = dir;
    if g.world.steering() != dir {
        let _ = g.world.apply(content, Command::PlayerMove { dir });
    }
}

/// The camera follows her while she is steered (and a moment after). `moved` = the player
/// panned the camera this frame (a zoom keeps the follow).
pub fn camera(g: &mut GameScreen, moved: bool, dt: f32) {
    if let Some(z) = g.hands.steer.zoom.take() {
        g.cam.zoom *= z;
    }
    if moved && g.world.steered_lately() {
        g.hands.steer.cam_free = true;
    }
    if g.hands.steer.follow_off
        || g.hands.steer.cam_free
        || g.follow.is_some()
        || !g.world.steered_lately()
        || g.world.phase() != Phase::Night
    {
        return;
    }
    let Some(k) = g.world.karolina().map(|k| k.id) else {
        return;
    };
    if let Some(head) = g.scene.head_of(AgentRef::Staff(k), g.time) {
        let want = head + vec2(0.0, 40.0);
        g.cam_target = None;
        g.cam.center = g.cam.center.lerp(want, (dt * 5.0).min(1.0));
    }
}

/// Home: the whole café, and the camera stays there until she is steered again.
pub fn home(g: &mut GameScreen) {
    g.hands.steer.cam_free = true;
}

/// The follow key: the camera follows her steps / stays put.
pub fn toggle_follow(g: &mut GameScreen) {
    g.hands.steer.follow_off = !g.hands.steer.follow_off;
    g.hands.steer.cam_free = false;
    let key = if g.hands.steer.follow_off {
        "walk.follow.off"
    } else {
        "walk.follow.on"
    };
    g.toast_key(ToastKind::Info, Icon::Info, key, &[]);
}

/// What the prompt offers now: she is free (no act) and something is in reach.
pub fn offer(g: &GameScreen, content: &Content) -> Option<Nearby> {
    if g.world.player().current.is_some() || !keys_are_hers(g) {
        return None;
    }
    g.world.nearby(content)
}

/// E: the thing beside her. False = nothing in reach (the old "nearest chore" goes then).
/// `keys_down` = a WASD / arrow key is still held (it waits for a fresh press).
pub fn interact(g: &mut GameScreen, content: &Content, keys_down: bool) -> bool {
    let Some(n) = g.world.nearby(content) else {
        return false;
    };
    // a wave first when they just came in (as a click on them)
    let party = match &n.kind {
        NearKind::Act(sim::PlayerAct::TakeOrder { party }) => Some(*party),
        NearKind::Card { guest } => g.world.party_of_guest(*guest).map(|p| p.id),
        _ => None,
    };
    if let Some(p) = party.and_then(|id| g.world.parties().iter().find(|p| p.id == id).cloned())
        && hands::greetable(content, &g.world, &p)
        && let Some(l) = p.leader()
    {
        let _ = g.world.apply(content, Command::PlayerGreet { guest: l });
    }
    match n.kind {
        NearKind::Act(_) | NearKind::OpenDoors => {
            if let Some(cmd) = n.command() {
                g.hand(content, cmd);
            }
        }
        NearKind::Card { guest } => g.open_card(content, guest),
        NearKind::Records { .. } => g.hands.records_open = true,
    }
    // keys still down: they wait for a fresh press (E let go of the steering)
    if keys_down {
        g.hands.steer.latch = true;
    }
    g.hands.tut.bump("walk_interact");
    true
}

/// The prompt over the thing beside her: a keycap and a verb, small (UI pass).
pub fn prompt(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let content = cx.content;
    let Some(n) = offer(g, content) else {
        return;
    };
    let loc = cx.loc;
    let t = ui.theme.clone();
    let key = crate::settings::keys::label(crate::settings::keys::Action::Chore);
    let text = loc.tr_owned(&n.label);
    let st = t.small().weight(Weight::SemiBold).color(t.text);
    let kw = ui.keycap_width(&key);
    let w = ui.fonts.measure(&text, &st) + kw + 22.0;
    let h = 24.0;
    // over the piece; over a person: above their head and its "!" (a tile and a bit up)
    let person = g.world.guests().iter().any(|x| x.tile() == n.tile);
    let lift = if person { 1.25 } else { 0.55 };
    let top = vec2(
        (n.tile.0 as f32 + 0.5) * crate::map::TILE,
        (n.tile.1 as f32 - lift) * crate::map::TILE,
    );
    let a = hud::world_to_ui(g, ui, top);
    let sw = ui.screen().w;
    let sh = ui.screen().h;
    let r = Rect::new(
        (a.x - w / 2.0).clamp(8.0, (sw - w - 8.0).max(8.0)),
        (a.y - h).clamp(8.0, (sh - h - 8.0).max(8.0)),
        w,
        h,
    );
    g.anchors.push(("walk.prompt", r));
    ui.glass(r, &Glass::tooltip().radius(8.0));
    let kr = ui.keycap(vec2(r.x + 6.0, r.center_y()), &key, false);
    ui.fonts.draw_in(
        &text,
        Rect::new(kr.right() + 6.0, r.y, w - kw - 16.0, h),
        &st,
        Align::Left,
    );
}

impl GameScreen {
    /// Karolina beside the piece `uid`, facing it (tests).
    #[cfg(test)]
    fn stage_walk_to_piece(&mut self, content: &Content, uid: u32) {
        if let Some(p) = self.world.furniture.iter_mut().find(|p| p.uid == uid) {
            p.dishes = p.dishes.max(2);
        }
        let p = self.world.furniture_by_uid(uid).cloned().unwrap();
        let fp = content
            .furniture_def(&p.def)
            .map(|d| sim::grid::footprint(d, p.x, p.y, p.rot))
            .unwrap_or_default();
        for t in &fp {
            for d in [(0i32, 1i32), (0, -1), (1, 0), (-1, 0)] {
                let n = (t.0 - d.0, t.1 - d.1);
                if !fp.contains(&n) && self.world.can_step_to(n) {
                    let i = self.world.staff.iter().position(|s| s.owner).unwrap();
                    self.world.staff[i].pos = sim::grid::tile_center(n);
                    self.world.staff[i].path.clear();
                    self.world.staff[i].facing = sim::Facing::from_delta(d.0 as f32, d.1 as f32);
                    return;
                }
            }
        }
    }

    /// `SHIFTELLA_SCENE=walk` (screenshots): Karolina a few tiles from `which` (`guest` — a
    /// party waiting for their order, `table` — a dirty table, `sink` — dishes in the sink),
    /// steered towards it for a moment by the fake keys; the camera zoomed in follows her, she
    /// stops beside it and the E prompt shows over it.
    pub fn stage_walk(&mut self, content: &Content, which: &str) {
        let _ = self.world.apply(content, Command::PlayerClearQueue);
        let _ = self.world.apply(content, Command::PlayerCancel);
        // nothing open over the café (the fast-forward may leave an order card up)
        self.hands.order = None;
        self.hands.pending_cards.clear();
        self.hands.moment = None;
        self.hands.book = None;
        // the crew on a break: nobody takes the order / the cups before she gets there
        let helpers: Vec<u32> = self
            .world
            .staff()
            .iter()
            .filter(|s| !s.owner && s.on_duty())
            .map(|s| s.id)
            .collect();
        for id in helpers {
            let _ = self
                .world
                .apply(content, Command::SendOnBreak { staff: id });
        }
        let tag_tiles = |g: &GameScreen, tag: &str| -> Vec<(i32, i32)> {
            g.world
                .furniture()
                .iter()
                .filter(|p| {
                    content
                        .furniture_def(&p.def)
                        .is_some_and(|d| d.has_tag(tag))
                })
                .flat_map(|p| {
                    content
                        .furniture_def(&p.def)
                        .map(|d| sim::grid::footprint(d, p.x, p.y, p.rot))
                        .unwrap_or_default()
                })
                .collect()
        };
        let targets: Vec<(i32, i32)> = match which {
            "table" => {
                let uid = self
                    .world
                    .furniture()
                    .iter()
                    .find(|p| {
                        content
                            .furniture_def(&p.def)
                            .is_some_and(|d| d.has_tag("table"))
                    })
                    .map(|p| p.uid);
                if let Some(uid) = uid
                    && let Some(p) = self.world.furniture.iter_mut().find(|p| p.uid == uid)
                {
                    p.dishes = p.dishes.max(3);
                }
                uid.and_then(|u| self.world.furniture_by_uid(u))
                    .and_then(|p| {
                        content
                            .furniture_def(&p.def)
                            .map(|d| sim::grid::footprint(d, p.x, p.y, p.rot))
                    })
                    .unwrap_or_default()
            }
            "sink" => {
                self.world.sink_dishes = self.world.sink_dishes.max(6);
                tag_tiles(self, "sink")
            }
            _ => {
                let w = &self.world;
                let open = |p: &&sim::Party| {
                    matches!(p.state, sim::PartyState::WaitOrder | sim::PartyState::Queue)
                        && p.order_claim.is_none()
                };
                w.parties()
                    .iter()
                    .filter(open)
                    .chain(w.parties().iter())
                    .filter_map(|p| p.leader())
                    .filter_map(|l| w.guests().iter().find(|g| g.id == l))
                    .map(|g| g.tile())
                    .take(6)
                    .collect()
            }
        };
        // a spot beside the target with a straight walk of a few tiles up to it
        let dirs: [(i8, i8); 4] = [(0, 1), (0, -1), (1, 0), (-1, 0)];
        // (the spot beside it, the way in, how many tiles she walks)
        type Approach = ((i32, i32), (i8, i8), i32);
        let mut best: Option<Approach> = None;
        for t in &targets {
            // the first target with a way up to it (the open party before anyone else)
            if best.is_some() {
                break;
            }
            for d in dirs {
                let spot = (t.0 - d.0 as i32, t.1 - d.1 as i32);
                if targets.contains(&spot) || !self.world.can_step_to(spot) {
                    continue;
                }
                let mut k = 0;
                while k < 4 {
                    let n = (spot.0 - d.0 as i32 * (k + 1), spot.1 - d.1 as i32 * (k + 1));
                    if !self.world.can_step_to(n) {
                        break;
                    }
                    k += 1;
                }
                if k > 0 && best.is_none_or(|b| k > b.2) {
                    best = Some((spot, d, k));
                }
            }
        }
        let Some((spot, d, k)) = best else {
            return;
        };
        let start = (spot.0 - d.0 as i32 * k, spot.1 - d.1 as i32 * k);
        if let Some(i) = self.world.staff.iter().position(|s| s.owner) {
            self.world.staff[i].pos = sim::grid::tile_center(start);
            self.world.staff[i].path.clear();
        }
        let secs = std::env::var("SHIFTELLA_WALK_T")
            .ok()
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(1.2);
        self.hands.steer.fake = Some((d, secs));
        self.hands.steer.zoom = Some(
            std::env::var("SHIFTELLA_WALK_ZOOM")
                .ok()
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(2.2),
        );
        self.speed = 1;
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;

    fn content() -> Rc<Content> {
        let root = env!("CARGO_MANIFEST_DIR");
        Rc::new(Content::load(&format!("{root}/assets/data")).expect("content"))
    }

    fn night(c: &Rc<Content>) -> GameScreen {
        let mut g = GameScreen::new_story(c, "cozy", true);
        g.start_night(c);
        g.world.skip_prep(c);
        let _ = g.world.drain_events();
        g
    }

    #[test]
    fn e_beside_a_dirty_table_gives_her_that_table() {
        let c = content();
        let mut g = night(&c);
        let _ = g.world.apply(&c, Command::PlayerClearQueue);
        let _ = g.world.apply(&c, Command::PlayerCancel);
        let uid = g
            .world
            .furniture()
            .iter()
            .find(|p| c.furniture_def(&p.def).is_some_and(|d| d.has_tag("table")))
            .map(|p| p.uid)
            .unwrap();
        g.stage_walk_to_piece(&c, uid);
        let n = g.world.nearby(&c).expect("the table is beside her");
        assert_eq!(
            n.kind,
            NearKind::Act(sim::PlayerAct::ClearTable { table: uid })
        );
        assert!(interact(&mut g, &c, true));
        assert!(g.hands.steer.latch, "held keys wait for a fresh press");
        let acts = hands::karolina_acts(&g.world);
        assert!(
            acts.contains(&sim::PlayerAct::ClearTable { table: uid }),
            "{acts:?}"
        );
        // the latch holds the keys back until they are let go
        g.hands.steer.fake = Some(((1, 0), 1.0));
        input(&mut g, &c, false, 0.1);
        assert_eq!(g.world.steering(), (0, 0));
        g.hands.steer.fake = None;
        input(&mut g, &c, false, 0.1);
        assert!(!g.hands.steer.latch);
    }

    #[test]
    fn the_keys_are_hers_only_on_the_floor_at_night() {
        let c = content();
        let mut g = GameScreen::new_story(&c, "cozy", true);
        assert!(!keys_are_hers(&g), "by day WASD is the camera");
        g.start_night(&c);
        g.world.skip_prep(&c);
        assert!(keys_are_hers(&g));
        g.panel = Some(Panel::Build);
        assert!(!keys_are_hers(&g), "build mode keeps WASD for the camera");
        g.hands.steer.fake = Some(((1, 0), 1.0));
        input(&mut g, &c, false, 0.1);
        assert_eq!(g.world.steering(), (0, 0), "nothing is sent in build mode");
        g.panel = None;
        input(&mut g, &c, false, 0.1);
        assert_eq!(g.world.steering(), (1, 0));
        // the fake keys run out: let go
        input(&mut g, &c, false, 2.0);
        assert_eq!(g.world.steering(), (0, 0));
    }

    #[test]
    fn moving_the_camera_while_she_walks_frees_it_until_the_next_walk() {
        let c = content();
        let mut g = night(&c);
        g.hands.steer.fake = Some(((1, 0), 5.0));
        input(&mut g, &c, false, 0.1);
        assert!(g.world.steered_lately());
        camera(&mut g, true, 0.1);
        assert!(g.hands.steer.cam_free);
        // let go, then a new walk brings the follow back
        g.hands.steer.fake = None;
        input(&mut g, &c, false, 0.1);
        g.hands.steer.fake = Some(((0, 1), 5.0));
        input(&mut g, &c, false, 0.1);
        assert!(!g.hands.steer.cam_free);
    }
}
