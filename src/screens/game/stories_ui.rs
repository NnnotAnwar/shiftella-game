//! Little stories of strangers and the storm night on screen (design/v0.2.md H).
//!
//! * **A story** — a small amber speech bubble with three dots beside a stranger's head
//!   (distinct from the indigo moment orbs; scaled by `world_icon_scale` so it stays readable
//!   zoomed out). A click opens the small glass card (the moment card's look): the guest's
//!   label (name once known, else the description), what Karolina notices, 2–3 one-click
//!   answers. An answer sends `Command::AnswerStory`; she walks over; the guest's reaction
//!   floats over them, a good one also writes the night log and shows a small toast.
//! * **The storm** — the forecast toast, the station's announcement, a compact "Snowed in"
//!   card under the atmosphere meter with two one-click chores (blankets, the big pot), grandma's
//!   plaid blankets drawn over everyone tucked in, and at 03:00 the shared talk: letterbox,
//!   the café held still, the cutscene dialogue box with the strangers' names and faces.
//!
//! Sim side: `sim::stories`, `sim::storm`. Sounds: existing `Sfx` only (marked where new ones
//! go).

use std::collections::HashMap;

use macroquad::prelude::*;

use super::hud::world_to_ui;
use super::world_view::{self as wv, sim};
use super::{Bubble, Ctx, GameScreen, Notice};
use crate::content::Content;
use crate::render::{self, Expression, PersonPose, Pose, draw::*};
use crate::settings::Locale;
use crate::ui::handson::{MomentAction, MomentOption, MomentView, moment_card, moment_card_rect};
use crate::ui::{
    Button, DialogueBox, DialogueLabels, DialogueLine, Glass, Icon, ToastKind, Ui, Weight,
    draw_icon,
};
use sim::{AgentRef, Command, SimEvent, StormAct};

/// The screen's stories state (`GameScreen::stories`).
#[derive(Default)]
pub struct StoriesUi {
    /// The open story card (the guest it's about).
    pub card: Option<u32>,
    /// When each marker appeared (it pops in).
    pub born: HashMap<u32, f32>,
    /// The marker under the mouse.
    pub hover: Option<u32>,
    /// The 03:00 talk on screen.
    pub talk: Option<TalkPlay>,
}

/// The storm talk being played.
pub struct TalkPlay {
    pub id: String,
    pub speakers: Vec<u32>,
    /// 0 = the narration, then the talk's lines.
    pub line: usize,
    pub t: f32,
    pub closing: Option<f32>,
    pub dialogue: DialogueBox,
}

impl StoriesUi {
    pub fn talk_open(&self) -> bool {
        self.talk.is_some()
    }
}

// ───────────────────────── markers ─────────────────────────

/// The world point over a person's head (`None` = not drawn this frame).
fn head(g: &mut GameScreen, agent: AgentRef) -> Option<(Vec2, PersonPose)> {
    let v = g.scene.people.iter().find(|v| v.agent == agent)?.clone();
    let look = g.scene.look(&v.look);
    let pose = wv::person_pose(&v, g.time);
    Some((render::emote_anchor(&look, &pose), pose))
}

/// Where a story bubble floats: left of the head (moment orbs float on the right).
fn marker_at(head: Vec2) -> Vec2 {
    let s = render::world_icon_scale();
    head + vec2(-17.0 * s, -4.0 * s)
}

/// Story markers on screen now: (guest, centre, radius) in world px.
pub fn markers(g: &mut GameScreen) -> Vec<(u32, Vec2, f32)> {
    if g.world.phase() != sim::Phase::Night || g.stories.talk.is_some() {
        return Vec::new();
    }
    let open: Vec<u32> = g
        .world
        .stories_active()
        .iter()
        .filter(|a| a.resolving.is_none())
        .map(|a| a.guest)
        .collect();
    let s = render::world_icon_scale();
    open.into_iter()
        .filter_map(|guest| {
            head(g, AgentRef::Guest(guest)).map(|(h, _)| (guest, marker_at(h), 11.0 * s))
        })
        .collect()
}

/// The story marker under the mouse (world px).
pub fn hit(g: &mut GameScreen, m: Vec2) -> Option<u32> {
    markers(g)
        .into_iter()
        .find(|(_, c, r)| c.distance(m) < r + 4.0)
        .map(|x| x.0)
}

/// An amber speech bubble with three dots (and a soft halo), `k` = appear 0..1.
fn draw_marker(c: Vec2, t: f32, born: f32, hover: bool) {
    let s = render::world_icon_scale();
    let age = (t - born).max(0.0);
    let pop = {
        let a = (age / 0.35).clamp(0.0, 1.0);
        let c1 = 1.70158;
        (1.0 + (c1 + 1.0) * (a - 1.0).powi(3) + c1 * (a - 1.0).powi(2)).max(0.0)
    };
    if pop <= 0.01 {
        return;
    }
    let k = s * pop * if hover { 1.12 } else { 1.0 };
    let bob = (age * 1.4).sin() * 1.5 * s;
    let c = c + vec2(0.0, bob);
    let (w, h) = (20.0 * k, 14.0 * k);
    let pulse = 0.5 + 0.5 * (age * 2.2).sin();
    glow(
        c.x,
        c.y,
        16.0 * k,
        Color::new(
            1.0,
            0.77,
            0.42,
            0.22 + 0.12 * pulse + if hover { 0.1 } else { 0.0 },
        ),
    );
    let amber = Color::from_hex(0xFFC46B);
    let deep = Color::from_hex(0xF99C4E);
    // the tail towards the head (down-right)
    draw_triangle(
        c + vec2(2.0 * k, h * 0.35),
        c + vec2(8.0 * k, h * 0.35),
        c + vec2(9.5 * k, h * 0.5 + 5.0 * k),
        deep,
    );
    rrect_v(c.x - w / 2.0, c.y - h / 2.0, w, h, 6.0 * k, amber, deep);
    // a lit rim and a highlight
    draw_rectangle_lines(
        c.x - w / 2.0,
        c.y - h / 2.0,
        w,
        h,
        if hover { 1.6 } else { 1.0 },
        Color::new(1.0, 0.95, 0.82, 0.55),
    );
    ellipse(
        c.x - w * 0.22,
        c.y - h * 0.26,
        w * 0.2,
        h * 0.12,
        Color::new(1.0, 1.0, 1.0, 0.35),
    );
    // three dots (a little wave)
    let ink = Color::from_hex(0x4A2A16);
    for i in 0..3 {
        let x = c.x + (i as f32 - 1.0) * 4.6 * k;
        let lift = ((age * 3.0 - i as f32 * 0.6).sin().max(0.0)) * 1.3 * k;
        ellipse(x, c.y - lift, 1.5 * k, 1.5 * k, ink);
    }
}

/// Grandma's plaid over a seated guest's shoulders (drawn in the y-sorted queue).
fn draw_blanket(feet: Vec2, height: f32, facing: render::Facing, t: f32, seed: u32) {
    let colors = [0xB4475E_u32, 0x6D5BD0, 0x3F8F7A, 0xC9873B];
    let base = Color::from_hex(colors[(seed % 4) as usize]);
    let dark = Color::new(base.r * 0.72, base.g * 0.72, base.b * 0.72, 1.0);
    let up = match facing {
        render::Facing::Down => 46.0,
        _ => 40.0,
    } * height;
    let breathe = (t * 0.9 + seed as f32).sin() * 0.6;
    let c = feet - vec2(0.0, up - breathe);
    let (w, h) = (25.0, 19.0);
    rrect_v(c.x - w / 2.0, c.y - h * 0.35, w, h, 6.0, base, dark);
    // the plaid: two light stripes each way
    let line = Color::new(1.0, 0.95, 0.85, 0.28);
    for dx in [-5.0_f32, 5.0] {
        draw_line(c.x + dx, c.y - h * 0.3, c.x + dx, c.y + h * 0.6, 1.4, line);
    }
    for dy in [0.0_f32, 7.0] {
        draw_line(
            c.x - w / 2.0 + 2.0,
            c.y + dy,
            c.x + w / 2.0 - 2.0,
            c.y + dy,
            1.2,
            line,
        );
    }
    // a fringe at the bottom edge
    let fringe = Color::new(1.0, 0.92, 0.8, 0.45);
    for i in 0..6 {
        let x = c.x - w / 2.0 + 3.0 + i as f32 * 3.8;
        draw_line(x, c.y + h * 0.65, x, c.y + h * 0.65 + 2.5, 1.0, fringe);
    }
}

/// The blankets to draw this frame: (sort key, feet, height, facing, seed).
pub type BlanketDraw = (f32, Vec2, f32, render::Facing, u32);

pub fn blankets(g: &mut GameScreen) -> Vec<BlanketDraw> {
    let list: Vec<u32> = g.world.storm().blanketed.clone();
    if list.is_empty() || g.world.phase() != sim::Phase::Night {
        return Vec::new();
    }
    let mut out = Vec::new();
    for gid in list {
        let Some(v) = g
            .scene
            .people
            .iter()
            .find(|v| v.agent == AgentRef::Guest(gid))
            .cloned()
        else {
            continue;
        };
        let pose = wv::person_pose(&v, g.time);
        if pose.pose != Pose::Sit {
            continue;
        }
        let look = g.scene.look(&v.look);
        let key = render::person_sort_key(&pose) + 0.002;
        out.push((key, pose.feet, look.height, pose.facing, gid));
    }
    out
}

/// Pushes the blankets into the y-sorted queue (right after their sitters).
pub fn push_blankets(q: &mut render::RenderQueue, list: &[BlanketDraw], t: f32) {
    for &(key, feet, h, facing, seed) in list {
        q.push_custom(key, move || draw_blanket(feet, h, facing, t, seed));
    }
}

/// World-space pieces over the lit café: story markers, the speaker's ring in the talk.
pub fn draw_world(g: &mut GameScreen) {
    let t = g.time;
    if let Some(tp) = &g.stories.talk {
        // a soft ring under whoever speaks
        let who = current_speaker(g, tp);
        if let Some(agent) = who
            && let Some((_, pose)) = head(g, agent)
        {
            let p = 0.5 + 0.5 * (t * 2.0).sin();
            draw_ellipse_lines(
                pose.feet.x,
                pose.feet.y,
                22.0,
                9.0,
                0.0,
                1.6,
                Color::new(1.0, 0.82, 0.5, 0.45 + 0.25 * p),
            );
        }
        return;
    }
    let hover = g.stories.hover;
    for (guest, c, _) in markers(g) {
        let born = *g.stories.born.entry(guest).or_insert(t);
        draw_marker(c, t, born, hover == Some(guest));
    }
}

fn current_speaker(g: &GameScreen, tp: &TalkPlay) -> Option<AgentRef> {
    if tp.line == 0 {
        return None;
    }
    let def = g.world.storm_talk_def(&tp.id)?;
    let l = def.lines.get(tp.line - 1)?;
    match l.who {
        Some(i) => tp.speakers.get(i as usize).map(|g| AgentRef::Guest(*g)),
        None => g.world.karolina().map(|k| AgentRef::Staff(k.id)),
    }
}

// ───────────────────────── the card ─────────────────────────

impl GameScreen {
    /// Opens the story card of a stranger (a marker click).
    pub fn open_story(&mut self, guest: u32) -> bool {
        let ok = self
            .world
            .story_of(guest)
            .is_some_and(|a| a.resolving.is_none());
        if ok {
            self.stories.card = Some(guest);
            self.hands.order = None;
            self.hands.moment = None;
            self.bubble = None;
        }
        ok
    }

    /// Picks an answer: Karolina walks over (the card closes at once).
    pub fn answer_story(&mut self, content: &Content, guest: u32, answer: &str) -> bool {
        self.stories.card = None;
        match self.world.apply(
            content,
            Command::AnswerStory {
                guest,
                answer: answer.to_string(),
            },
        ) {
            Ok(()) => true,
            Err(sim::CommandError::QueueFull) => {
                self.notices.push(Notice::Key {
                    kind: ToastKind::Info,
                    icon: Icon::Info,
                    key: "ui.queue.full".into(),
                    args: Vec::new(),
                });
                false
            }
            // a click raced the guest out of the door
            Err(sim::CommandError::Invalid(why)) if why == sim::GUEST_LEFT => {
                self.notices.push(Notice::Key {
                    kind: ToastKind::Info,
                    icon: Icon::Door,
                    key: "hux.story.left".into(),
                    args: Vec::new(),
                });
                false
            }
            Err(_) => false,
        }
    }

    /// Story & storm events: reactions, the night log, toasts, the talk.
    pub(super) fn stories_event(&mut self, e: &SimEvent, loc: Option<&Locale>) {
        let Some(loc) = loc else {
            if let SimEvent::StormTalk { talk, speakers } = e {
                self.start_talk(talk, speakers);
            }
            return;
        };
        match e {
            SimEvent::StoryNoticed { guest, .. } => {
                self.stories.born.insert(*guest, self.time);
            }
            SimEvent::StoryFaded { guest, .. } => {
                self.stories.born.remove(guest);
                if self.stories.card == Some(*guest) {
                    self.stories.card = None;
                }
            }
            SimEvent::StoryAnswered {
                guest,
                story,
                answer,
                good,
            } => {
                self.stories.born.remove(guest);
                let Some(def) = self.world.story_def(story) else {
                    return;
                };
                if let Some(a) = def.answer(answer) {
                    let text = loc.tr_owned(&a.react_key(story));
                    if self.bubble.as_ref().is_none_or(|b| !b.pinned) {
                        self.bubble = Some(Bubble {
                            agent: AgentRef::Guest(*guest),
                            text,
                            talk: None,
                            pinned: false,
                            until: self.time + 7.0,
                        });
                    }
                }
                if *good {
                    let name = name_of(&self.world, loc, *guest);
                    let line = loc.trf(&def.log_key(), &[("name", &name)]);
                    self.push_ticker(Icon::Heart, line.clone());
                    self.toast(
                        ToastKind::Success,
                        Icon::Heart,
                        loc.tr_owned("story.toast"),
                        Some(line),
                    );
                }
            }
            SimEvent::StormForecast => {
                let title = loc.tr_owned("storm.forecast.title");
                self.push_ticker(Icon::Snowflake, title.clone());
                self.toast(
                    ToastKind::Info,
                    Icon::Snowflake,
                    title,
                    Some(loc.tr_owned("storm.forecast.text")),
                );
            }
            SimEvent::StormCancelled { .. } => {
                let text = loc.tr_owned("storm.cancel.text");
                self.push_ticker(Icon::Train, text.clone());
                self.toast(
                    ToastKind::Info,
                    Icon::Snowflake,
                    loc.tr_owned("storm.cancel.title"),
                    Some(format!("{text} {}", loc.tr("storm.cancel.stay"))),
                );
            }
            SimEvent::StormCared { act, .. } => {
                let key = format!("storm.cared.{}", act.id());
                let icon = match act {
                    StormAct::Blankets => Icon::Heart,
                    StormAct::Soup => Icon::Bowl,
                };
                self.push_ticker(icon, loc.tr_owned(&key));
            }
            SimEvent::StormTalk { talk, speakers } => self.start_talk(talk, speakers),
            SimEvent::StormMorning { cared } => {
                let body = if *cared {
                    "storm.morning.cared"
                } else {
                    "storm.morning.plain"
                };
                self.push_ticker(Icon::Sun, loc.tr_owned(body));
                self.toast(
                    ToastKind::Success,
                    Icon::Sun,
                    loc.tr_owned("storm.morning.title"),
                    Some(loc.tr_owned(body)),
                );
            }
            _ => {}
        }
    }

    fn start_talk(&mut self, talk: &str, speakers: &[u32]) {
        if talk.is_empty() || !talk_still_on(self, speakers) {
            return;
        }
        self.stories.card = None;
        self.hands.order = None;
        self.hands.moment = None;
        self.bubble = None;
        let mut dialogue = DialogueBox::new();
        if self.automation {
            dialogue.reveal_all();
        }
        self.stories.talk = Some(TalkPlay {
            id: talk.to_string(),
            speakers: speakers.to_vec(),
            line: 0,
            t: 0.0,
            closing: None,
            dialogue,
        });
        // the camera settles on the table(s) where they sit
        let pts: Vec<Vec2> = speakers
            .iter()
            .filter_map(|g| self.world.guests().iter().find(|x| x.id == *g))
            .map(|x| vec2(x.pos.0, x.pos.1))
            .collect();
        if !pts.is_empty() {
            let c = pts.iter().fold(Vec2::ZERO, |a, b| a + *b) / pts.len() as f32;
            self.cam_target = Some(c - vec2(0.0, 30.0));
            if self.automation {
                self.cam.center = c - vec2(0.0, 30.0);
            }
        }
    }
}

/// The talk can still play: the night is on and its people are in the café.
fn talk_still_on(g: &GameScreen, speakers: &[u32]) -> bool {
    g.world.phase() == sim::Phase::Night
        && !speakers.is_empty()
        && speakers
            .iter()
            .all(|s| g.world.guests().iter().any(|x| x.id == *s && !x.gone))
}

/// How a guest is called on screen (a regular's name, else who they are).
fn name_of(w: &sim::World, loc: &Locale, guest: u32) -> String {
    super::guest_label::label(w, loc, guest)
}

/// `persona:<Latin>|<uk>|<ru>` (a sim arg, e.g. a polaroid caption) → the name in `loc`'s
/// language.
pub fn persona_arg_text(loc: &Locale, v: &str) -> Option<String> {
    let rest = v.strip_prefix("persona:")?;
    let mut it = rest.split('|');
    let lat = it.next().unwrap_or_default();
    let uk = it.next().filter(|s| !s.is_empty()).unwrap_or(lat);
    let ru = it.next().filter(|s| !s.is_empty()).unwrap_or(uk);
    Some(
        match loc.lang.code() {
            "uk" => uk,
            "ru" => ru,
            _ => lat,
        }
        .to_string(),
    )
}

/// The storm card (night UI, the left column after the HUD).
pub fn draw(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    // (under the crew card, which covers the left column, the storm card waits)
    if g.panel != Some(super::Panel::Crew) {
        storm_card(g, ui, cx);
    }
}

/// The open story card — with the other cards, above the panels (like a moment's card; drawn
/// with the HUD it sat under an open crew card and showed through its glass).
pub fn card(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    if g.world.phase() != sim::Phase::Night || g.summary.is_some() {
        return;
    }
    let Some(guest) = g.stories.card else {
        return;
    };
    let content = cx.content;
    let loc = cx.loc;
    let Some(a) = g.world.story_of(guest).cloned() else {
        g.stories.card = None;
        return;
    };
    let Some(def) = g.world.story_def(&a.story) else {
        g.stories.card = None;
        return;
    };
    if a.resolving.is_some() || g.world.phase() != sim::Phase::Night {
        g.stories.card = None;
        return;
    }
    let anchor = markers(g)
        .into_iter()
        .find(|m| m.0 == guest)
        .map(|m| world_to_ui(g, ui, m.1))
        .unwrap_or(vec2(ui.screen().w / 2.0, ui.screen().h / 2.0));
    let now = g.world.minute();
    let span = a.expires.saturating_sub(a.spawned).max(1) as f32;
    let left = a.expires.saturating_sub(now) as f32 / span;
    let v = MomentView {
        icon: Icon::Heart,
        title: super::guest_label::label(&g.world, &loc, guest),
        text: Some(loc.tr_owned(&def.text_key())),
        options: def
            .answers
            .iter()
            .map(|o| MomentOption::new(loc.tr_owned(&o.label_key(&def.id))))
            .collect(),
        expires: Some(left.clamp(0.0, 1.0)),
    };
    let mut avoid = super::hands_ui::hud_avoid(g);
    avoid.extend(super::moments_ui::toast_zone(g, ui));
    let r = moment_card_rect(ui, anchor, &v, &avoid);
    g.anchors.push(("story.card", r));
    match moment_card(
        ui,
        ("story", guest),
        anchor,
        &v,
        loc.tr("ui.common.close"),
        &avoid,
    ) {
        Some(MomentAction::Pick(i)) => {
            if let Some(o) = def.answers.get(i) {
                let id = o.id.clone();
                // a new sound would go here: a soft "on my way" chime (sounds agent)
                cx.audio.play_sfx_vol(crate::audio::Sfx::UiClick, 0.4);
                g.answer_story(content, guest, &id);
            }
        }
        Some(MomentAction::Dismiss) => g.stories.card = None,
        None => {}
    }
}

/// "Snowed in": how many stay the night, and Karolina's two chores as one-click buttons.
fn storm_card(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    if !g.world.storm_line_closed() || g.overlay_open() || g.summary.is_some() {
        return;
    }
    let stranded = g.world.storm_stranded().len() as u32;
    let storm = g.world.storm().clone();
    if stranded == 0 && !(storm.blankets_done || storm.soup_done) {
        return;
    }
    let content = cx.content;
    let loc = cx.loc;
    let t = ui.theme.clone();
    let size = crate::ui::handson::ATMO_SIZE;
    // under the atmosphere meter (and the back-step chip beside it)
    let top = g
        .anchors
        .iter()
        .filter(|a| matches!(a.0, "hud.backstep"))
        .map(|a| a.1.bottom())
        .fold(12.0 + 64.0 + 6.0 + size.y, f32::max)
        + 6.0;
    let title = loc.tr_owned("storm.chip.title");
    let count = loc
        .i18n
        .trn(loc.lang, "storm.chip.stranded", stranded as i64, &[]);
    let st = t.small().size(12.0).weight(Weight::SemiBold);
    let sub = t.small().size(11.0).color(t.text_dim);
    let acts = [
        (StormAct::Blankets, storm.blankets_done, Icon::Heart),
        (StormAct::Soup, storm.soup_done, Icon::Bowl),
    ];
    let labels: Vec<String> = acts
        .iter()
        .map(|(a, done, _)| {
            if *done {
                loc.tr_owned(&format!("storm.done.{}", a.id()))
            } else if storm.busy.contains(a) {
                loc.tr_owned("storm.act.on_way")
            } else {
                loc.tr_owned(&format!("storm.act.{}", a.id()))
            }
        })
        .collect();
    let tips: Vec<String> = acts
        .iter()
        .map(|(a, _, _)| loc.tr_owned(&format!("storm.act.{}.tip", a.id())))
        .collect();
    let buttons: Vec<Button> = acts
        .iter()
        .enumerate()
        .map(|(i, (a, done, icon))| {
            let busy = storm.busy.contains(a);
            Button::new(&labels[i])
                .ghost()
                .align(crate::ui::Align::Left)
                .icon(if *done { Icon::Check } else { *icon })
                .selected(*done)
                .disabled(busy || (!*done && !g.world.storm_can(content, *a)))
                .tooltip(&tips[i])
        })
        .collect();
    // a narrow column under the atmosphere meter: it stays clear of the hall
    // as wide as the meter, wider when the count line needs it (large text, long languages)
    let w = size
        .x
        .max(ui.fonts.measure(&count, &sub) + 20.0)
        .max(ui.fonts.measure(&title, &st) + 36.0);
    let r = Rect::new(12.0, top, w, 40.0 + 2.0 * 26.0 + 3.0 + 6.0);
    g.anchors.push(("hud.storm", r));
    ui.glass(r, &Glass::hud().radius(8.0));
    draw_icon(
        Icon::Snowflake,
        vec2(r.x + 14.0, r.y + 13.0),
        12.0,
        Color::from_hex(0xA5B4FC),
    );
    ui.fonts
        .draw(&title, r.x + 26.0, r.y + 17.0, &st.color(t.text));
    ui.fonts.draw(&count, r.x + 10.0, r.y + 33.0, &sub);
    let mut y = r.y + 40.0;
    for (i, b) in buttons.iter().enumerate() {
        let br = Rect::new(r.x + 4.0, y, w - 8.0, 26.0);
        if ui.button(("storm_act", i), br, b).clicked && !acts[i].1 {
            let cmd = match acts[i].0 {
                StormAct::Blankets => Command::StormBlankets,
                StormAct::Soup => Command::StormSoup,
            };
            // a new sound would go here: blankets rustling / a ladle on a pot (sounds agent)
            cx.audio.play_sfx_vol(crate::audio::Sfx::UiClick, 0.4);
            let _ = g.command(content, cmd);
        }
        y += 26.0 + 3.0;
    }
}

// ───────────────────────── the talk ─────────────────────────

/// Plays the 03:00 talk over the held café: `true` = the talk owns the UI this frame.
pub fn talk_ui(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx, dt: f32) -> bool {
    let Some(mut tp) = g.stories.talk.take() else {
        return false;
    };
    let loc = cx.loc;
    tp.t += dt;
    let Some(def) = g.world.storm_talk_def(&tp.id) else {
        return false;
    };
    // a talk that arrived late (the night ran on without a window: fast-forward, a skipped
    // night) is dropped once its people have gone home
    if !talk_still_on(g, &tp.speakers) {
        return false;
    }
    // the autoplayer listens a moment, then goes on (like a story scene)
    if (g.hands.autoplay.is_some() || g.hands.skip_scenes) && tp.closing.is_none() {
        let wait = if g.hands.skip_scenes { 0.3 } else { 2.5 };
        if tp.t > wait * (tp.line + 1) as f32 {
            tp.line += 1;
            if tp.line > def.lines.len() {
                tp.closing = Some(tp.t);
            }
        }
    }
    let fade_in = (tp.t / 0.6).clamp(0.0, 1.0);
    let fade_out = tp
        .closing
        .map_or(1.0, |c| 1.0 - ((tp.t - c) / 0.5).clamp(0.0, 1.0));
    let bars = ui.letterbox(fade_in * fade_out);
    tp.dialogue.bottom_inset = (bars.y - 12.0).max(0.0);
    if let Some(c) = tp.closing {
        if tp.t - c >= 0.5 {
            // a new sound would go here: the room's murmur coming back (sounds agent)
            return true;
        }
        g.stories.talk = Some(tp);
        return true;
    }
    let labels = DialogueLabels {
        skip: loc.tr("ui.cutscene.skip"),
        log: loc.tr("ui.log.title"),
        history: loc.tr("ui.log.title"),
    };
    let names: Vec<String> = tp
        .speakers
        .iter()
        .map(|s| name_of(&g.world, &loc, *s))
        .collect();
    let arg = |i: usize| names.get(i).cloned().unwrap_or_default();
    let (a, b, c) = (arg(0), arg(1), arg(2));
    let args = [("a", a.as_str()), ("b", b.as_str()), ("c", c.as_str())];
    if g.automation {
        // screenshots: the whole line at once
        tp.dialogue.reveal_all();
    }
    let advance = if tp.line == 0 {
        let text = loc.tr_owned("storm.talk.intro");
        tp.dialogue.narration_box(ui, &text).advance
    } else if let Some(l) = def.lines.get(tp.line - 1) {
        let text = loc.trf(&l.key, &args);
        let (name, agent, color) = match l.who {
            Some(i) => (
                arg(i as usize),
                tp.speakers.get(i as usize).map(|x| AgentRef::Guest(*x)),
                Color::from_hex(0xC9873B),
            ),
            None => (
                loc.tr_owned("cast.player.name"),
                g.world.karolina().map(|k| AgentRef::Staff(k.id)),
                Color::from_hex(0x6366F1),
            ),
        };
        let line = DialogueLine::new(&name, &text)
            .portrait(agent.is_some())
            .color(color)
            .labels(labels);
        let r = tp.dialogue.dialogue_box(ui, &line);
        if let (Some(pr), Some(agent)) = (r.portrait, agent)
            && let Some(v) = g.scene.people.iter().find(|v| v.agent == agent).cloned()
        {
            let look = g.scene.look(&v.look);
            render::draw_portrait(&look, Expression::Smile, pr, g.time);
        }
        r.advance
    } else {
        true
    };
    if advance {
        tp.line += 1;
        if tp.line > def.lines.len() {
            tp.closing = Some(tp.t);
        }
    }
    if ui.esc() {
        tp.closing = Some(tp.t);
    }
    g.stories.talk = Some(tp);
    true
}

/// The talk keeps the keyboard (Esc skips it).
pub fn talk_hotkeys(g: &mut GameScreen, ui: &mut Ui) -> bool {
    let Some(tp) = g.stories.talk.as_mut() else {
        return false;
    };
    if ui.esc() && tp.closing.is_none() {
        tp.closing = Some(tp.t);
    }
    true
}

// ───────────────────────── screenshot scenes ─────────────────────────

/// `SHIFTELLA_SCENE=story_marker|story_card|storm|storm_talk` (verification shots): plays the
/// night to the moment with the autoplayer-free headless loop, then sets the shot up.
pub fn scene_setup(g: &mut GameScreen, content: &std::rc::Rc<Content>, loc: &Locale, name: &str) {
    match name {
        "story_marker" | "story_card" => {
            let until = crate::app::scene_env("SHIFTELLA_TIME")
                .and_then(|t| crate::content::ClockTime::parse(&t).ok())
                .map(|t| t.minutes_since_open().max(0))
                .unwrap_or(60);
            g.play_headless(content, loc, 1500.0, &move |g| {
                let m = g.world.minutes_since_open();
                (m >= until && !g.world.stories_active().is_empty()) || m >= until + 30
            });
            // none came on its own by then: put the wanted one on a seated stranger
            if g.world.stories_active().is_empty() {
                let want =
                    crate::app::scene_env("SHIFTELLA_STORY").unwrap_or_else(|| "call".into());
                let pick = g
                    .world
                    .parties()
                    .iter()
                    .filter(|p| p.regular.is_none())
                    .filter(|p| {
                        matches!(p.state, sim::PartyState::WaitFood | sim::PartyState::Eating)
                    })
                    .filter_map(|p| p.leader())
                    .find(|l| {
                        g.world.persona(*l).is_some()
                            && g.world
                                .guests()
                                .iter()
                                .any(|x| x.id == *l && x.seat.is_some())
                    });
                if let Some(l) = pick {
                    g.world.debug_story(&want, l);
                    g.drain_and_handle(content, Some(loc));
                }
            }
            if name == "story_card"
                && let Some(gid) = g.world.stories_active().first().map(|a| a.guest)
            {
                g.open_story(gid);
            }
        }
        "storm" | "storm_talk" => {
            let until = crate::app::scene_env("SHIFTELLA_TIME")
                .and_then(|t| crate::content::ClockTime::parse(&t).ok())
                .map(|t| t.minutes_since_open().max(0))
                .unwrap_or(if name == "storm" { 200 } else { 302 });
            let blankets_at = 175;
            g.play_headless(content, loc, 1500.0, &move |g| {
                g.world.minutes_since_open() >= blankets_at.min(until)
            });
            let _ = g.command(content, Command::StormBlankets);
            g.play_headless(content, loc, 1500.0, &move |g| {
                g.world.storm().blankets_done || g.world.minutes_since_open() >= until
            });
            let _ = g.command(content, Command::StormSoup);
            g.play_headless(content, loc, 1500.0, &move |g| {
                g.world.minutes_since_open() >= until || g.stories.talk.is_some()
            });
            g.drain_and_handle(content, Some(loc));
            // shots: the talk on screen at its N-th line (SHIFTELLA_LINE)
            if let Some(tp) = g.stories.talk.as_mut() {
                tp.line = crate::app::scene_env("SHIFTELLA_LINE")
                    .and_then(|l| l.parse().ok())
                    .unwrap_or(1);
                tp.t = 2.0;
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::super::{Phase, TICK_SECONDS};
    use super::*;
    use crate::i18n::Lang;
    use crate::settings::load_i18n;

    fn content() -> Rc<Content> {
        let root = env!("CARGO_MANIFEST_DIR");
        Rc::new(Content::load(&format!("{root}/assets/data")).expect("content"))
    }

    fn i18n() -> crate::i18n::I18n {
        let root = env!("CARGO_MANIFEST_DIR");
        load_i18n(&format!("{root}/assets/i18n")).expect("i18n")
    }

    #[test]
    fn a_persona_arg_reads_in_the_players_language() {
        let i = i18n();
        let p = sim::Persona {
            name: "Hanna".into(),
            name_uk: "Ганна".into(),
            name_ru: String::new(),
            ..Default::default()
        };
        let arg = sim::persona_arg(&p);
        let name = |lang| persona_arg_text(&Locale::new(&i, lang), &arg).unwrap();
        assert_eq!(name(Lang::Cs), "Hanna");
        assert_eq!(name(Lang::En), "Hanna");
        assert_eq!(name(Lang::Uk), "Ганна");
        assert_eq!(
            name(Lang::Ru),
            "Ганна",
            "no Russian spelling: the Ukrainian one"
        );
        assert!(persona_arg_text(&Locale::new(&i, Lang::En), "money:5").is_none());
    }

    #[test]
    fn the_storm_night_on_screen_plays_the_talk_and_drops_it_at_dawn() {
        let c = content();
        let i = i18n();
        let loc = Locale::new(&i, Lang::Ru);
        let mut g = GameScreen::new_story(&c, "cozy", true);
        g.fast_forward(&c, &loc, 29, Some(0), true);
        assert!(g.world.storm().tonight);
        let mut guard = 0;
        while g.stories.talk.is_none() && g.world.phase() == Phase::Night && guard < 40_000 {
            if g.cutscene_playing() {
                g.finish_cutscene_now(&c, &loc);
            }
            g.advance(&c, TICK_SECONDS * 3.0, Some(&loc));
            guard += 1;
        }
        let tp = g.stories.talk.as_ref().expect("the 03:00 talk opens");
        assert!(tp.speakers.len() >= 2);
        assert!(g.frozen(), "the café holds still while they talk");
        assert!(
            g.ticker
                .iter()
                .any(|t| t.text == loc.tr_owned("storm.cancel.text")),
            "the station's announcement is in the night log"
        );
        // the talk is never left hanging over an empty morning
        let speakers = tp.speakers.clone();
        g.stories.talk = None;
        g.start_talk("snow", &speakers);
        assert!(g.stories.talk.is_some());
        g.stories.talk = None;
        let mut guard = 0;
        while g.world.phase() == Phase::Night && guard < 40_000 {
            if g.cutscene_playing() {
                g.finish_cutscene_now(&c, &loc);
            }
            g.stories.talk = None;
            g.advance(&c, TICK_SECONDS * 3.0, Some(&loc));
            guard += 1;
        }
        g.start_talk("snow", &speakers);
        assert!(g.stories.talk.is_none(), "no talk once the night is over");
    }
}
