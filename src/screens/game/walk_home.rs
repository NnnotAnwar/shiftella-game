//! Karolina's own small life (design/v0.2.md F): the walk home after the morning summary and
//! the break on the back step. Full-screen, calm, short; one key (Space / Enter / Esc) skips.
//!
//! * The walk (day, after the summary): she locks the café door while a small chip row asks
//!   "Which way home?" — one click picks (or she takes her usual way after a few seconds); she
//!   walks through dawn Holešovice (~15 s), two lines on the way, "Home." and her window
//!   lights up. Then the day planning.
//! * The back step (night, a HUD chip beside the atmosphere meter, once a night, not in a
//!   rush): she sits on the step with a cup, the cat comes over, a phone message arrives.
//!   The café is held while the scene is on screen (guests simply wait).
//!
//! Sim side: `sim::walk` (`Command::WalkHome`, `Command::StepOutBack`).

use std::rc::Rc;

use macroquad::prelude::*;

use super::world_view::sim;
use super::{Ctx, GameScreen};
use crate::audio::Sfx;
use crate::content::{Content, Weather};
use crate::render::backstep::{BackstepFrame, draw_backstep, karolina_feet};
use crate::render::dawn::{
    self, CAFE_DOOR_X, DawnFrame, DawnRoute, GROUND_Y, PAVE_Y, SKY_EXTRA, TRAM_STOP_X, VIEW_H,
};
use crate::render::{Facing, Pose, SeasonLook};
use crate::ui::draw::{fill_rrect, with_a};
use crate::ui::{Align, Button, Glass, Icon, RectExt, Ui};

/// Before the pick: she takes her usual way after this long.
const AUTO_PICK_S: f32 = 7.0;
/// The walk itself.
const WALK_S: f32 = 15.0;
/// "Home." and the window lighting up.
const ARRIVE_S: f32 = 3.4;
const FADE_IN_S: f32 = 0.9;
const FADE_OUT_S: f32 = 0.9;
/// Picking the way home: the street dims into the blue hour over this long, the chosen way's
/// street replaces it while it's dark (0.2.7: the street swapped in one frame before her eyes)…
const DIP_OUT_S: f32 = 0.45;
/// …and comes up again over this long, while she sets off.
const DIP_IN_S: f32 = 0.6;
/// She sets off this long after the pick (in the dip, once the new street is there).
const SET_OFF_S: f32 = DIP_OUT_S + 0.15;
/// After the walk / the back step the café (the day planning, the night) comes up from the
/// same dusk over this long instead of popping in.
const AFTER_S: f32 = 0.7;
/// The colour every transition of these scenes passes through.
const DUSK: Color = Color::new(0.05, 0.04, 0.1, 1.0);
/// The back step closes by itself after this long.
const BACKSTEP_S: f32 = 32.0;
/// The phone buzzes.
const MESSAGE_AT: f32 = 3.5;

/// The scene on screen, if any (`GameScreen::walk`).
#[derive(Debug, Default)]
pub struct WalkUi {
    scene: Option<Scene>,
    /// Seconds left of the café's fade-in after a scene (see [`AFTER_S`]).
    after: f32,
}

impl WalkUi {
    pub fn is_open(&self) -> bool {
        self.scene.is_some()
    }
    pub fn walking(&self) -> bool {
        matches!(self.scene, Some(Scene::Walk(_)))
    }
    pub fn on_back_step(&self) -> bool {
        matches!(self.scene, Some(Scene::Back(_)))
    }
}

#[derive(Debug)]
enum Scene {
    Walk(Walk),
    Back(Back),
}

#[derive(Debug, Clone)]
struct Mood {
    season: SeasonLook,
    rain: f32,
    fog: f32,
    /// A storm: the wind loop plays (sounds).
    storm: bool,
}

#[derive(Debug)]
struct Walk {
    t: f32,
    route: Option<String>,
    picked_at: f32,
    /// Lines on the way (keys), chosen when the route is picked.
    lines: Vec<String>,
    /// Fade-out started at.
    closing: Option<f32>,
    mood: Mood,
    /// Sounds already played (bit flags).
    sounds: u32,
}

#[derive(Debug)]
struct Back {
    t: f32,
    closing: Option<f32>,
    mood: Mood,
    /// Message text key, sender name key.
    message: Option<(String, String)>,
    clock: String,
    cat: f32,
    sounds: u32,
}

fn mood(g: &GameScreen, content: &Content, weather: Option<Weather>) -> Mood {
    let (mut season, _) = super::scene_fx::season_of(&g.world, content);
    let w = weather.unwrap_or(g.world.weather());
    let (rain, fog) = match w {
        Weather::Rain => (0.8, 0.0),
        Weather::Downpour => (1.6, 0.2),
        Weather::Storm => (1.8, 0.1),
        Weather::Fog => (0.0, 1.0),
        Weather::Clear => (0.0, 0.0),
        Weather::Snow => (0.0, 0.0),
    };
    season.snowfall = if w == Weather::Snow {
        1.0
    } else if season.snowfall > 0.0 {
        0.3
    } else {
        0.0
    };
    if w == Weather::Snow {
        season.snow_cover = season.snow_cover.max(0.8);
        season.breath = 1.0;
    }
    Mood {
        season,
        rain,
        fog,
        storm: w == Weather::Storm,
    }
}

// ───────────────────────── starting ─────────────────────────

/// The morning summary closes: keep the polaroid, then walk home (if there is a day ahead).
pub fn close_summary(g: &mut GameScreen, content: &Rc<Content>) {
    close_summary_with(g, content, None);
}

/// The same with a weather for the scene (screenshots).
pub fn close_summary_with(g: &mut GameScreen, content: &Rc<Content>, weather: Option<Weather>) {
    super::journal_ui::keep_morning(g, content);
    g.summary = None;
    begin(g, content, weather);
}

/// Starts the walk home (no-op in the night, after an ending, or if she already walked).
pub fn begin(g: &mut GameScreen, content: &Content, weather: Option<Weather>) {
    if g.world.phase() != sim::Phase::Day
        || g.world.is_closed()
        || g.world.ending().is_some()
        || g.world.walked_home_today()
        || g.hands.autoplay.is_some()
        || g.hands.skip_scenes
    {
        return;
    }
    let m = mood(g, content, weather);
    g.walk.scene = Some(Scene::Walk(Walk {
        t: 0.0,
        route: None,
        picked_at: 0.0,
        lines: Vec::new(),
        closing: None,
        mood: m,
        sounds: 0,
    }));
}

/// Screenshots: pick a route at once and jump `t` seconds into the scene.
pub fn debug_seek(g: &mut GameScreen, content: &Rc<Content>, route: Option<&str>, t: f32) {
    debug_seek_picked(g, content, route, t, 0.0);
}

/// Screenshots of the pick's transition: the route counts as picked at `picked_at` s
/// (`SHIFTELLA_PICK_AT`), the scene shows `t` s.
pub fn debug_seek_picked(
    g: &mut GameScreen,
    content: &Rc<Content>,
    route: Option<&str>,
    t: f32,
    picked_at: f32,
) {
    if let Some(r) = route {
        pick(g, content, r);
    }
    match &mut g.walk.scene {
        Some(Scene::Walk(w)) => {
            if route.is_some() {
                w.picked_at = picked_at.min(t);
            }
            w.t = t;
        }
        Some(Scene::Back(b)) => b.t = t,
        None => {}
    }
}

/// The HUD chip opens the back step (also callable from tests / scenes).
pub fn step_out(g: &mut GameScreen, content: &Rc<Content>, weather: Option<Weather>) -> bool {
    if g.command(content, sim::Command::StepOutBack).is_err() {
        return false;
    }
    let message = g
        .world
        .backstep_message()
        .map(|m| (m.text_key(), m.from.name_key()));
    let clock = g.world.clock_label();
    let cat = (g.world.cat_level() as f32 / 10.0).clamp(0.0, 1.0);
    let m = mood(g, content, weather);
    g.walk.scene = Some(Scene::Back(Back {
        t: 0.0,
        closing: None,
        mood: m,
        message,
        clock,
        cat,
        sounds: 0,
    }));
    true
}

fn pick(g: &mut GameScreen, content: &Rc<Content>, route: &str) {
    let lines: Vec<String> = g
        .world
        .walk_lines(content, route)
        .into_iter()
        .map(str::to_string)
        .collect();
    let _ = g.command(
        content,
        sim::Command::WalkHome {
            route: route.to_string(),
        },
    );
    if let Some(Scene::Walk(w)) = &mut g.walk.scene
        && w.route.is_none()
    {
        w.route = Some(route.to_string());
        w.picked_at = w.t;
        w.lines = lines;
    }
}

// ───────────────────────── the HUD chip ─────────────────────────

/// At night, beside the atmosphere meter: "Step out back" while the back step is open.
pub fn backstep_chip(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    if g.world.phase() != sim::Phase::Night
        || g.world.backstep_block(cx.content).is_some()
        || g.overlay_open()
        || g.summary.is_some()
    {
        return;
    }
    let loc = cx.loc;
    let b = Button::new(loc.tr("walk.backstep.chip"))
        .ghost()
        .icon(Icon::Door)
        .tooltip(loc.tr("walk.backstep.chip_tip"));
    // under the atmosphere meter, beside the camera button — clear of the ticket rail at the
    // top and of the "open now" chip that sits right of the meter during the prep
    let size = crate::ui::handson::ATMO_SIZE;
    let top = g
        .anchors
        .iter()
        .find(|a| a.0 == "hud.atmosphere")
        .map(|a| a.1.bottom() + 6.0)
        .unwrap_or(12.0 + 64.0 + 6.0 + size.y + 6.0);
    let w = ui.button_width(&b) + 8.0;
    let r = Rect::new(12.0 + 30.0 + 6.0, top, w, 30.0);
    g.anchors.push(("hud.backstep", r));
    ui.glass(r, &Glass::hud().radius(8.0));
    let br = r.inset(4.0);
    if ui.button("walk_backstep", br, &b).clicked {
        cx.audio.play_sfx_vol(Sfx::DoorCreak, 0.9);
        step_out(g, cx.content, None);
    }
}

// ───────────────────────── the frame ─────────────────────────

/// Draws the open scene and handles its input. `true` = the scene owns the frame (the café is
/// neither drawn nor ticked).
pub fn frame(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx, dt: f32) -> bool {
    if g.walk.scene.is_none() {
        return false;
    }
    let content = cx.content.clone();
    let dt = dt.min(0.1);
    let mut done = false;
    let mut pick_now: Option<String> = None;
    let s = ui.screen();
    match g.walk.scene.as_mut() {
        Some(Scene::Walk(w)) => {
            w.t += dt;
            let usual = g.world.usual_route().to_string();
            if w.route.is_none() && w.t >= AUTO_PICK_S {
                pick_now = Some(usual.clone());
            }
            let skip = w.closing.is_none() && (ui.enter() || ui.esc() || ui.key(KeyCode::Space));
            if skip {
                if w.route.is_none() {
                    pick_now = Some(usual.clone());
                }
                w.closing = Some(w.t);
            }
            if let Some(c) = w.closing
                && w.t - c >= FADE_OUT_S * 0.6
            {
                done = true;
            }
            if w.route.is_some() && w.t - w.picked_at > SET_OFF_S + WALK_S + ARRIVE_S + FADE_OUT_S {
                done = true;
            }
        }
        Some(Scene::Back(b)) => {
            b.t += dt;
            let skip = b.closing.is_none() && (ui.enter() || ui.esc() || ui.key(KeyCode::Space));
            if skip || (b.closing.is_none() && b.t >= BACKSTEP_S) {
                b.closing = Some(b.t);
            }
            if let Some(c) = b.closing
                && b.t - c >= FADE_OUT_S * 0.6
            {
                done = true;
            }
        }
        None => {}
    }
    if let Some(r) = pick_now {
        pick(g, &content, &r);
    }
    // draw
    clear_background(Color::from_hex(0x0d0c17));
    match g.walk.scene.as_ref() {
        Some(Scene::Walk(w)) => {
            let f = walk_frame(w);
            let vh = VIEW_H + SKY_EXTRA;
            let vw = vh * s.w / s.h.max(1.0);
            let focus = walk_focus(&f);
            let left = dawn::camera_left(f.route, focus, vw);
            let view = Rect::new(left, -SKY_EXTRA, vw, vh);
            set_view_camera(view);
            dawn::draw_dawn(view, &f);
            ui.set_camera();
            walk_ui(g, ui, cx);
        }
        Some(Scene::Back(b)) => {
            let vw = VIEW_H * s.w / s.h.max(1.0);
            let view = Rect::new(0.0, 0.0, vw, VIEW_H);
            set_view_camera(view);
            let f = BackstepFrame {
                t: b.t,
                season: b.mood.season,
                rain: b.mood.rain,
                fog: b.mood.fog,
                cat: b.cat,
                phone: ((b.t - MESSAGE_AT + 0.4) / 0.5).clamp(0.0, 1.0),
            };
            draw_backstep(view, &f);
            let feet = karolina_feet(view);
            ui.set_camera();
            let k = s.h / VIEW_H;
            back_ui(g, ui, cx, vec2(feet.x * k, feet.y * k));
        }
        None => {}
    }
    sounds(g, cx);
    g.update_audio(&content, cx.audio);
    if g.walk.on_back_step() {
        // outside under the awning: its own rain loop instead of the rain on the café window
        cx.audio.set_rain(false);
    }
    if done {
        g.walk.scene = None;
        g.walk.after = AFTER_S;
    }
    true
}

/// The café coming up from the dusk the walk / the back step ended in (over everything,
/// the last thing of a game frame).
pub fn after_veil(g: &mut GameScreen, ui: &Ui, dt: f32) {
    if g.walk.after <= 0.0 || g.walk.scene.is_some() {
        return;
    }
    let k = after_alpha(g.walk.after);
    g.walk.after = (g.walk.after - dt.min(0.1)).max(0.0);
    if k > 0.0 {
        let s = ui.screen();
        draw_rectangle(0.0, 0.0, s.w, s.h, Color { a: k, ..DUSK });
    }
}

/// The veil over the café `left` seconds before its fade-in ends.
fn after_alpha(left: f32) -> f32 {
    ease(left / AFTER_S)
}

fn set_view_camera(view: Rect) {
    let mut cam = Camera2D::from_display_rect(view);
    // from_display_rect is meant for render targets; for the screen flip Y (brief §3)
    cam.zoom.y = -cam.zoom.y;
    set_camera(&cam);
}

/// The scenes' sounds (CC0, package K). Bits in `sounds` mark the one-shots already played;
/// loops are asked for every frame and fade out by themselves when the scene closes.
fn sounds(g: &mut GameScreen, cx: &mut Ctx) {
    let mut cues = Cues::default();
    match g.walk.scene.as_mut() {
        Some(Scene::Walk(w)) => walk_sounds(w, &mut cues),
        Some(Scene::Back(b)) => back_sounds(b, &mut cues),
        None => {}
    }
    for (s, v) in cues.sfx {
        cx.audio.play_sfx_vol(s, v);
    }
    for (a, v) in cues.loops {
        cx.audio.want_amb(a, v);
    }
}

/// What a scene wants to hear this frame (kept apart from the mixer so tests stay silent).
#[derive(Debug, Default)]
struct Cues {
    sfx: Vec<(Sfx, f32)>,
    loops: Vec<(crate::audio::AmbLoop, f32)>,
}

/// `true` the first time `bit` is set.
fn once(bits: &mut u32, bit: u32) -> bool {
    let first = *bits & bit == 0;
    *bits |= bit;
    first
}

fn walk_sounds(w: &mut Walk, c: &mut Cues) {
    use crate::audio::AmbLoop;
    let f = walk_frame(w);
    // she locks the café door
    if w.t > 0.6 && once(&mut w.sounds, 1) {
        c.sfx.push((Sfx::KeysLock, 0.8));
    }
    // her steps on the wet pavement while she walks (louder after rain)
    if f.visible && f.pose == Pose::Walk && w.closing.is_none() {
        let wet = f.season.wet.max(f.rain.min(1.0));
        c.loops.push((AmbLoop::WetSteps, 0.55 + 0.35 * wet));
    }
    // the pigeons take off as she comes near (`dawn::draw_pigeons`)
    if f.visible {
        for (i, gx) in dawn::pigeon_spots(f.route).iter().enumerate().take(4) {
            if f.karolina.x > gx - 80.0 && once(&mut w.sounds, 16 << i) {
                c.sfx.push((Sfx::Pigeons, 0.7));
            }
        }
    }
    if w.route.is_none() {
        return;
    }
    let u = (w.t - w.picked_at - SET_OFF_S) / WALK_S;
    if f.route == Some(DawnRoute::Tram) {
        // the tram comes in, rings as it leaves the stop, and goes on without her
        if u > 0.2 && once(&mut w.sounds, 4) {
            c.sfx.push((Sfx::TramPass, 0.8));
        }
        if u > 0.47 && once(&mut w.sounds, 8) {
            c.sfx.push((Sfx::TramBell, 0.8));
        }
        if u > 0.9 && once(&mut w.sounds, 256) {
            c.sfx.push((Sfx::TramPass, 0.5));
        }
    }
    // home: her front door
    if u > 1.0 + 0.7 / WALK_S && once(&mut w.sounds, 2) {
        c.sfx.push((Sfx::DoorOutside, 0.6));
    }
}

fn back_sounds(b: &mut Back, c: &mut Cues) {
    use crate::audio::AmbLoop;
    if b.mood.rain > 0.0 {
        c.loops.push((AmbLoop::RainAwning, b.mood.rain.min(1.0)));
    }
    if b.mood.storm {
        c.loops.push((AmbLoop::Storm, 0.8));
    }
    if b.t > MESSAGE_AT && b.message.is_some() && once(&mut b.sounds, 1) {
        c.sfx.push((Sfx::PhoneBuzz, 0.8));
    }
    // the cat sits down beside her (`render::backstep`, at 6 s), says hello and, if it
    // trusts her, purrs
    if b.t > 6.3 && once(&mut b.sounds, 2) {
        c.sfx.push((Sfx::CatMeow, 0.45 + 0.3 * b.cat));
    }
    if b.t > 6.8 && b.cat > 0.3 && b.closing.is_none() {
        c.loops
            .push((AmbLoop::CatPurr, ((b.cat - 0.3) / 0.7).clamp(0.2, 1.0)));
    }
}

// ───────────────────────── the walk's timeline ─────────────────────────

fn ease(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// A walk that starts and stops gently but keeps a steady pace in the middle.
fn stroll(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    let a = 0.08;
    // accelerate over `a`, steady, decelerate over the last `a`
    let v = 1.0 / (1.0 - a);
    if u < a {
        v * u * u / (2.0 * a)
    } else if u > 1.0 - a {
        let r = 1.0 - u;
        1.0 - v * r * r / (2.0 * a)
    } else {
        v * (u - a / 2.0)
    }
}

/// The route the street shows: the chosen one only once the dip is dark.
fn shown_route(w: &Walk) -> Option<&str> {
    w.route
        .as_deref()
        .filter(|_| w.t - w.picked_at >= DIP_OUT_S)
}

/// The dusk over the walk (0 clear … 1 dark): it opens from dark, dips around the pick (dark
/// when the street changes), closes at home or on a skip. Continuous — no scene change shows.
fn walk_veil(w: &Walk) -> f32 {
    let fade_in = 1.0 - ease(w.t / FADE_IN_S);
    let dip = if w.route.is_some() {
        let a = w.t - w.picked_at;
        if a < DIP_OUT_S {
            ease(a / DIP_OUT_S)
        } else {
            1.0 - ease((a - DIP_OUT_S) / DIP_IN_S)
        }
    } else {
        0.0
    };
    let fade_out = w
        .closing
        .map_or(0.0, |c| ease((w.t - c) / (FADE_OUT_S * 0.6)));
    let end = if w.route.is_some() {
        ease((w.t - w.picked_at - SET_OFF_S - WALK_S - ARRIVE_S) / FADE_OUT_S)
    } else {
        0.0
    };
    fade_in.max(dip).max(fade_out).max(end)
}

fn walk_frame(w: &Walk) -> DawnFrame {
    let route = shown_route(w).and_then(DawnRoute::from_id);
    let home = dawn::home_x(route);
    // the blue hour while she locks up, then the sun comes up on the way
    let pre = |t: f32| 0.1 + 0.12 * ease(t / AUTO_PICK_S);
    let light = if w.route.is_some() {
        let l0 = pre(w.picked_at);
        l0 + (1.0 - l0) * ease((w.t - w.picked_at - SET_OFF_S) / (WALK_S + 1.0))
    } else {
        pre(w.t)
    };
    let mut f = DawnFrame {
        route,
        karolina: vec2(CAFE_DOOR_X, GROUND_Y - 6.0),
        pose: Pose::Work,
        facing: Facing::Up,
        visible: true,
        tram_x: None,
        light,
        t: w.t,
        season: w.mood.season,
        rain: w.mood.rain,
        fog: w.mood.fog,
        window_lit: 0.0,
    };
    if route.is_none() {
        // locking the door, then a look at the sky
        if w.t > 2.2 {
            f.pose = Pose::Idle;
            f.facing = Facing::Down;
        }
        return f;
    }
    let tw = w.t - w.picked_at - SET_OFF_S;
    if tw < 0.0 {
        f.pose = Pose::Idle;
        f.facing = Facing::Right;
        return f;
    }
    let u = tw / WALK_S;
    f.pose = Pose::Walk;
    f.facing = Facing::Right;
    if route == Some(DawnRoute::Tram) {
        tram_timeline(&mut f, u, home);
    } else {
        f.karolina.x = CAFE_DOOR_X + (home - CAFE_DOOR_X) * stroll(u);
        f.karolina.y = GROUND_Y - 6.0 * (1.0 - ease(u * 12.0));
    }
    if u >= 1.0 {
        let a = tw - WALK_S;
        f.karolina = vec2(home, GROUND_Y - 6.0 * ease(a / 0.8));
        f.pose = if a < 0.8 { Pose::Walk } else { Pose::Idle };
        f.facing = Facing::Up;
        f.window_lit = ease((a - 1.0) / 0.9);
    }
    f
}

fn tram_timeline(f: &mut DawnFrame, u: f32, home: f32) {
    let stop = TRAM_STOP_X - 20.0;
    let off_stop = dawn::tram_home_stop(f.route);
    // the tram's front stops so that its rear door faces the stop
    let at_stop = stop + 114.0;
    let at_home = off_stop + 114.0;
    let seg = |a: f32, b: f32| ((u - a) / (b - a)).clamp(0.0, 1.0);
    if u < 0.22 {
        f.karolina.x = CAFE_DOOR_X + (stop - CAFE_DOOR_X) * stroll(seg(0.0, 0.22));
        f.karolina.y = GROUND_Y - 6.0 * (1.0 - ease(u * 40.0));
        f.tram_x = None;
    } else if u < 0.4 {
        f.karolina.x = stop;
        f.pose = Pose::Idle;
        f.facing = Facing::Left;
        let k = seg(0.22, 0.4);
        let e = 1.0 - (1.0 - k) * (1.0 - k);
        f.tram_x = Some(at_stop - 900.0 * (1.0 - e));
    } else if u < 0.47 {
        // she steps in
        let k = seg(0.4, 0.47);
        f.tram_x = Some(at_stop);
        f.karolina = vec2(stop, GROUND_Y - (GROUND_Y - PAVE_Y - 2.0) * ease(k));
        f.facing = Facing::Up;
        f.pose = Pose::Walk;
        f.visible = k < 0.85;
    } else if u < 0.86 {
        let k = seg(0.47, 0.86);
        f.tram_x = Some(at_stop + (at_home - at_stop) * ease(k));
        f.visible = false;
        f.karolina.x = f.tram_x.unwrap_or(at_stop) - 114.0;
    } else if u < 0.9 {
        let k = seg(0.86, 0.9);
        f.tram_x = Some(at_home);
        f.karolina = vec2(off_stop, PAVE_Y + 2.0 + (GROUND_Y - PAVE_Y - 2.0) * ease(k));
        f.facing = Facing::Down;
        f.visible = k > 0.1;
    } else {
        let k = seg(0.9, 1.0);
        f.tram_x = Some(at_home + 1400.0 * k * k);
        f.karolina.x = off_stop + (home - off_stop) * ease(k);
        f.karolina.y = GROUND_Y;
    }
}

/// Where the camera looks: Karolina, or the tram she rides.
fn walk_focus(f: &DawnFrame) -> f32 {
    if f.visible {
        f.karolina.x
    } else {
        f.tram_x.map_or(f.karolina.x, |x| x - 114.0)
    }
}

// ───────────────────────── the walk's UI ─────────────────────────

fn walk_ui(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let Some(Scene::Walk(w)) = g.walk.scene.as_ref() else {
        return;
    };
    let t = ui.theme.clone();
    let loc = cx.loc;
    let s = ui.screen();
    let (wt, route, picked_at, closing) = (w.t, w.route.clone(), w.picked_at, w.closing);
    let lines = w.lines.clone();
    // the place and the time, top left
    let tw = wt - picked_at - SET_OFF_S;
    let minute = 2 + (wt / 2.5) as u32;
    let label = format!("06:{:02} · {}", minute.min(40), loc.tr("walk.place"));
    let a_top = ease(wt / 1.2) * (1.0 - ease((tw - 3.0) / 1.5) * route.is_some() as i32 as f32);
    if a_top > 0.01 {
        crate::ui::draw::push_alpha(a_top);
        ui.fonts.draw(
            &label,
            20.0,
            30.0,
            &t.small_medium().color(with_a(t.text, 0.85)),
        );
        crate::ui::draw::pop_alpha();
    }
    // the route chips while she locks the door
    let usual = g.world.usual_route().to_string();
    // the chips fade out with the street once a way is picked (not gone in one frame)
    let chips_out = if route.is_some() {
        1.0 - ease((wt - picked_at) / DIP_OUT_S)
    } else {
        1.0
    };
    if chips_out > 0.0 && closing.is_none() {
        let a = ease((wt - 0.5) / 0.6) * chips_out;
        crate::ui::draw::push_alpha(a);
        let routes = &sim::walk::data().routes;
        let buttons: Vec<(String, String, String)> = routes
            .iter()
            .map(|r| {
                (
                    r.id.clone(),
                    loc.tr_owned(&r.name_key()),
                    if r.id == usual {
                        format!("{} — {}", loc.tr("walk.usual"), loc.tr(&r.hint_key()))
                    } else {
                        loc.tr_owned(&r.hint_key())
                    },
                )
            })
            .collect();
        let keys = ["1", "2", "3", "4"];
        let widths: Vec<f32> = buttons
            .iter()
            .enumerate()
            .map(|(i, b)| {
                ui.button_width(
                    &Button::new(&b.1)
                        .icon(route_icon(&b.0))
                        .shortcut(keys[i.min(3)]),
                )
            })
            .collect();
        let gap = 6.0;
        let row_w = widths.iter().sum::<f32>() + gap * (widths.len().saturating_sub(1)) as f32;
        let title = loc.tr("walk.title");
        let pw = row_w + 20.0;
        let pr = Rect::new((s.w - pw) / 2.0, s.h - 96.0, pw, 74.0);
        g.anchors.push(("walk.routes", pr));
        ui.glass(pr, &Glass::hud().radius(10.0));
        ui.fonts.draw_in(
            title,
            Rect::new(pr.x + 10.0, pr.y + 6.0, pr.w - 20.0, 20.0),
            &t.strong(),
            Align::Center,
        );
        let mut x = pr.x + 10.0;
        let y = pr.y + 32.0;
        let mut clicked = None;
        for (i, (id, name, hint)) in buttons.iter().enumerate() {
            let r = Rect::new(x, y, widths[i], 32.0);
            let b = Button::new(name)
                .secondary()
                .icon(route_icon(id))
                .shortcut(keys[i.min(3)])
                .selected(*id == usual)
                .tooltip(hint);
            let resp = ui.button(("walk_route", i), r, &b);
            if resp.clicked && route.is_none() {
                clicked = Some(id.clone());
            }
            // the usual way fills a thin line until she just goes
            if *id == usual {
                let k = ((wt - 0.5) / (AUTO_PICK_S - 0.5)).clamp(0.0, 1.0);
                fill_rrect(
                    Rect::new(r.x + 8.0, r.bottom() + 3.0, (r.w - 16.0) * k, 2.0),
                    1.0,
                    with_a(t.indigo_300, 0.7),
                );
            }
            x += widths[i] + gap;
        }
        crate::ui::draw::pop_alpha();
        let digits = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4];
        for (i, k) in digits.iter().enumerate() {
            if route.is_none()
                && ui.key(*k)
                && let Some(b) = buttons.get(i)
            {
                clicked = Some(b.0.clone());
            }
        }
        if let Some(id) = clicked {
            cx.audio.play_sfx(Sfx::UiClick);
            pick(g, cx.content, &id);
        }
    }
    // the lines on the way
    if route.is_some() && closing.is_none() {
        let u = tw / WALK_S;
        let shown = [(0.08, 0.46), (0.52, 0.94)];
        for (i, key) in lines.iter().enumerate().take(2) {
            let (a0, a1) = shown[i];
            let a = ease((u - a0) * WALK_S / 0.7) * (1.0 - ease((u - a1) * WALK_S / 0.7));
            if a > 0.01 {
                caption(ui, loc.tr(key), a, s.h - 58.0, false);
            }
        }
        // home
        if u >= 1.0 {
            let a = ease((tw - WALK_S - 0.6) / 0.6);
            let hint = route
                .as_deref()
                .and_then(sim::walk::route)
                .map(|r| loc.tr_owned(&r.hint_key()))
                .unwrap_or_default();
            crate::ui::draw::push_alpha(a);
            ui.fonts.draw_in(
                loc.tr("walk.home"),
                Rect::new(0.0, s.h - 98.0, s.w, 24.0),
                &t.title().color(t.text),
                Align::Center,
            );
            crate::ui::draw::pop_alpha();
            caption(ui, &hint, a, s.h - 66.0, true);
        }
    }
    // skip, bottom right
    if closing.is_none() {
        skip_button(g, ui, cx);
    }
    // fades (one veil: the opening, the dip around the pick, home / skip)
    let k = g.walk.scene.as_ref().map_or(0.0, |sc| match sc {
        Scene::Walk(w) => walk_veil(w),
        Scene::Back(_) => 0.0,
    });
    if k > 0.0 {
        draw_rectangle(0.0, 0.0, s.w, s.h, Color { a: k, ..DUSK });
    }
}

fn route_icon(id: &str) -> Icon {
    match id {
        "embankment" => Icon::Sunrise,
        "park" => Icon::Leaf,
        "bakery" => Icon::ChefHat,
        "tram" => Icon::Train,
        _ => Icon::Home,
    }
}

/// A one-line caption in a soft glass pill, centred.
fn caption(ui: &mut Ui, text: &str, a: f32, y: f32, dim: bool) {
    let t = ui.theme.clone();
    let s = ui.screen();
    let st = if dim {
        t.small().color(t.indigo_300)
    } else {
        t.body_medium().color(t.text)
    };
    let maxw = (s.w - 80.0).min(620.0);
    let w = (ui.fonts.measure(text, &st) + 28.0).min(maxw);
    let lines = if ui.fonts.measure(text, &st) + 28.0 > maxw {
        2
    } else {
        1
    };
    let h = 12.0 + st.line_h() * lines as f32;
    let r = Rect::new((s.w - w) / 2.0, y - h / 2.0, w, h);
    crate::ui::draw::push_alpha(a);
    if !dim {
        ui.glass(r, &Glass::hud().radius(h / 2.0));
    }
    ui.fonts
        .draw_wrapped(text, r.inset_xy(14.0, 6.0), &st, Align::Center, Some(2));
    crate::ui::draw::pop_alpha();
}

fn skip_button(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let loc = cx.loc;
    let s = ui.screen();
    let b = Button::new(loc.tr("walk.skip"))
        .ghost()
        .trailing(Icon::Forward)
        .shortcut("Space");
    let w = ui.button_width(&b);
    let mut r = Rect::new(s.w - 16.0 - w, s.h - 16.0 - 30.0, w, 30.0);
    // a wide route picker (ru / uk at a large UI scale) reaches the corner: skip sits above it
    if let Some(pr) = g.anchors.iter().find(|a| a.0 == "walk.routes").map(|a| a.1)
        && pr.right() + 8.0 > r.x
    {
        r.y = pr.y - 8.0 - r.h;
    }
    if ui.button("walk_skip", r, &b).clicked {
        let usual = g.world.usual_route().to_string();
        if let Some(Scene::Walk(w)) = g.walk.scene.as_ref()
            && w.route.is_none()
        {
            pick(g, cx.content, &usual);
        }
        if let Some(Scene::Walk(w)) = g.walk.scene.as_mut() {
            w.closing = Some(w.t);
        }
    }
}

// ───────────────────────── the back step's UI ─────────────────────────

fn back_ui(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx, feet: Vec2) {
    let Some(Scene::Back(b)) = g.walk.scene.as_ref() else {
        return;
    };
    let t = ui.theme.clone();
    let loc = cx.loc;
    let s = ui.screen();
    let (bt, closing) = (b.t, b.closing);
    // the message, typed in beside her
    if let Some((text_key, from_key)) = &b.message {
        let k = ease((bt - MESSAGE_AT) / 0.5);
        if k > 0.0 {
            let text = loc.tr(text_key);
            let from = loc.tr(from_key);
            let w = 300.0_f32.min(s.w - feet.x - 60.0).max(240.0);
            let body = t.body().color(t.text);
            let tw = w - 24.0;
            let lines = ui.fonts.wrap(text, tw, &body);
            let h = 44.0 + lines.len() as f32 * body.line_h() + 10.0;
            let x = (feet.x + 70.0).min(s.w - w - 16.0);
            let y = (feet.y - 210.0 - 8.0 * (1.0 - k)).max(16.0);
            let r = Rect::new(x, y, w, h);
            crate::ui::draw::push_alpha(k);
            ui.glass(r, &Glass::card().radius(12.0));
            ui.avatar(vec2(r.x + 22.0, r.y + 22.0), 24.0, from, None, false);
            ui.fonts.draw_in(
                from,
                Rect::new(r.x + 40.0, r.y + 10.0, r.w - 100.0, 18.0),
                &t.strong(),
                Align::Left,
            );
            ui.fonts.draw_in(
                &b.clock,
                Rect::new(r.right() - 72.0, r.y + 10.0, 60.0, 18.0),
                &t.small().tabular(),
                Align::Right,
            );
            let chars = ((bt - MESSAGE_AT - 0.4).max(0.0) * 42.0) as usize;
            ui.fonts.draw_lines(
                &lines,
                Rect::new(r.x + 12.0, r.y + 38.0, tw, h - 44.0),
                &body,
                Align::Left,
                None,
                chars,
            );
            crate::ui::draw::pop_alpha();
        }
    }
    // back inside
    if closing.is_none() {
        let a = ease((bt - 1.0) / 0.8);
        crate::ui::draw::push_alpha(a);
        let btn = Button::new(loc.tr("walk.backstep.back"))
            .secondary()
            .icon(Icon::Door)
            .shortcut("Esc");
        let w = ui.button_width(&btn);
        let r = Rect::new(s.w - 16.0 - w, s.h - 16.0 - 32.0, w, 32.0);
        let clicked = ui.button("walk_back_inside", r, &btn).clicked;
        crate::ui::draw::pop_alpha();
        if clicked && let Some(Scene::Back(b)) = g.walk.scene.as_mut() {
            b.closing = Some(b.t);
        }
    }
    let fade_in = 1.0 - ease(bt / FADE_IN_S);
    let fade_out = closing.map_or(0.0, |c| ease((bt - c) / (FADE_OUT_S * 0.6)));
    let k = fade_in.max(fade_out);
    if k > 0.0 {
        draw_rectangle(0.0, 0.0, s.w, s.h, Color::new(0.05, 0.04, 0.1, k));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn walk_with(route: &str) -> Walk {
        Walk {
            t: 0.0,
            route: Some(route.into()),
            picked_at: 0.0,
            lines: Vec::new(),
            closing: None,
            mood: Mood {
                season: SeasonLook::NONE,
                rain: 0.0,
                fog: 0.0,
                storm: false,
            },
            sounds: 0,
        }
    }

    /// Runs a walk to the end and collects what it played (one-shots) and the loops it asked for.
    fn walk_cues(route: &str) -> (Vec<Sfx>, usize) {
        let mut w = walk_with(route);
        let (mut sfx, mut steps) = (Vec::new(), 0);
        while w.t < SET_OFF_S + WALK_S + ARRIVE_S {
            let mut c = Cues::default();
            walk_sounds(&mut w, &mut c);
            sfx.extend(c.sfx.iter().map(|s| s.0));
            steps += c.loops.len();
            w.t += 1.0 / 30.0;
        }
        (sfx, steps)
    }

    #[test]
    fn the_walk_home_sounds_like_the_way_she_takes() {
        for id in ["embankment", "park", "bakery", "tram"] {
            let (sfx, steps) = walk_cues(id);
            let n = |s: Sfx| sfx.iter().filter(|x| **x == s).count();
            assert_eq!(n(Sfx::KeysLock), 1, "{id}: locks up once");
            assert_eq!(n(Sfx::DoorOutside), 1, "{id}: her front door once");
            assert!(steps > 100, "{id}: steps while she walks ({steps})");
            let spots = dawn::pigeon_spots(DawnRoute::from_id(id)).len();
            assert!(
                n(Sfx::Pigeons) >= 1 && n(Sfx::Pigeons) <= spots,
                "{id}: pigeons"
            );
            if id == "tram" {
                assert_eq!(n(Sfx::TramBell), 1);
                assert_eq!(n(Sfx::TramPass), 2, "it comes, and goes on");
            } else {
                assert_eq!(n(Sfx::TramBell) + n(Sfx::TramPass), 0, "{id}: no tram");
            }
            assert!(!sfx.contains(&Sfx::DoorBell), "{id}: no café bell outside");
        }
    }

    #[test]
    fn the_back_step_has_rain_on_the_awning_the_phone_and_the_cat() {
        let mut b = Back {
            t: 0.0,
            closing: None,
            mood: Mood {
                season: SeasonLook::NONE,
                rain: 0.8,
                fog: 0.0,
                storm: false,
            },
            message: Some(("m".into(), "n".into())),
            clock: String::new(),
            cat: 0.9,
            sounds: 0,
        };
        let (mut sfx, mut purr, mut rain) = (Vec::new(), 0, 0);
        while b.t < 12.0 {
            let mut c = Cues::default();
            back_sounds(&mut b, &mut c);
            sfx.extend(c.sfx.iter().map(|s| s.0));
            for (a, _) in &c.loops {
                match a {
                    crate::audio::AmbLoop::CatPurr => purr += 1,
                    crate::audio::AmbLoop::RainAwning => rain += 1,
                    _ => {}
                }
            }
            b.t += 1.0 / 30.0;
        }
        assert_eq!(sfx, vec![Sfx::PhoneBuzz, Sfx::CatMeow]);
        assert!(purr > 60 && rain > 300, "purr {purr}, rain {rain}");
        // a shy cat (low affection) says hello but doesn't purr; no rain, no awning loop
        b.t = 0.0;
        b.sounds = 0;
        b.cat = 0.1;
        b.mood.rain = 0.0;
        b.message = None;
        let mut c = Cues::default();
        b.t = 8.0;
        back_sounds(&mut b, &mut c);
        assert_eq!(c.sfx.len(), 1);
        assert!(c.loops.is_empty());
    }

    #[test]
    fn every_route_ends_at_her_door_and_the_light_rises() {
        for id in ["embankment", "park", "bakery", "tram"] {
            let mut w = walk_with(id);
            let mut last_light = 0.0;
            let mut prev_x = 0.0_f32;
            let mut steps = 0;
            while w.t < SET_OFF_S + WALK_S + ARRIVE_S {
                let f = walk_frame(&w);
                assert!(f.light + 1e-4 >= last_light, "{id}: light never dims");
                last_light = f.light;
                if f.visible {
                    assert!(
                        f.karolina.x + 0.5 >= prev_x,
                        "{id}: she never walks back ({} < {prev_x})",
                        f.karolina.x
                    );
                    prev_x = f.karolina.x;
                }
                w.t += 0.05;
                steps += 1;
            }
            let f = walk_frame(&w);
            let home = dawn::home_x(DawnRoute::from_id(id));
            assert!(
                (f.karolina.x - home).abs() < 1.0,
                "{id} at {}",
                f.karolina.x
            );
            assert!(f.window_lit > 0.9, "{id}: her window is lit");
            assert!(f.light > 0.9);
            assert!(steps > 100);
        }
    }

    /// 0.2.7: "the world changes abruptly depending on the chosen way home". Locking up, the
    /// pick, the walk, home: one veil that never jumps, dark whenever the street changes, and
    /// the café comes up from the same dusk afterwards.
    #[test]
    fn the_way_home_changes_only_under_a_soft_dusk() {
        for id in ["embankment", "park", "bakery", "tram"] {
            let mut w = walk_with(id);
            w.route = None;
            let dt = 1.0 / 60.0;
            let picked = 3.0;
            let (mut prev_veil, mut prev_route, mut prev_frame) =
                (walk_veil(&w), None::<DawnRoute>, walk_frame(&w).route);
            let mut changes = 0;
            let mut darkest = 0.0_f32;
            while w.t < picked + SET_OFF_S + WALK_S + ARRIVE_S + FADE_OUT_S {
                w.t += dt;
                if w.route.is_none() && w.t >= picked {
                    w.route = Some(id.into());
                    w.picked_at = w.t;
                }
                let veil = walk_veil(&w);
                assert!(
                    (veil - prev_veil).abs() < 0.1,
                    "{id}: the veil jumps at {:.2}s ({prev_veil} → {veil})",
                    w.t
                );
                let f = walk_frame(&w);
                if f.route != prev_frame {
                    changes += 1;
                    assert!(veil > 0.95, "{id}: the street changed in the open ({veil})");
                }
                if w.t > picked && w.t < picked + 1.5 {
                    darkest = darkest.max(veil);
                }
                prev_veil = veil;
                prev_frame = f.route;
                prev_route = f.route.or(prev_route);
            }
            assert_eq!(changes, 1, "{id}: the street changes once, at the pick");
            assert_eq!(prev_route, DawnRoute::from_id(id));
            assert!(darkest > 0.95, "{id}: the pick dips into the dusk");
            assert!(prev_veil > 0.99, "{id}: home fades out to the dusk");
            // she was standing at the door until the new street was there
            w.t = picked + DIP_OUT_S * 0.5;
            w.picked_at = picked;
            assert_eq!(walk_frame(&w).karolina.x, CAFE_DOOR_X);
        }
        // the café after it: from the dusk to clear, smoothly
        let mut left = AFTER_S;
        let mut prev = after_alpha(left);
        assert!(prev > 0.99);
        while left > 0.0 {
            left = (left - 1.0 / 60.0).max(0.0);
            let k = after_alpha(left);
            assert!(k <= prev && prev - k < 0.1);
            prev = k;
        }
        assert_eq!(prev, 0.0);
    }

    #[test]
    fn the_tram_carries_her_out_of_sight_and_lets_her_off_near_home() {
        let mut w = walk_with("tram");
        let mut hidden = false;
        while w.t < SET_OFF_S + WALK_S {
            let f = walk_frame(&w);
            if !f.visible {
                hidden = true;
                assert!(f.tram_x.is_some());
            }
            w.t += 0.05;
        }
        assert!(hidden);
    }
}
