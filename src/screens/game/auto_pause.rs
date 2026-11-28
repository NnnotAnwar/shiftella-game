//! "Pause on important events" (Settings → Gameplay; the sandbox setup's box, kept in the
//! save): the night pauses itself — the same pause as Space / the ⏸ button — when something
//! needs the player, and a small chip under the top bar says why ("Paused: Thunderstorm").
//! Space or the chip's Continue plays on.
//!
//! What pauses (each at most once, and at most one auto-pause per [`COOLDOWN_MINUTES`] of the
//! night):
//! * a bad event starts (`events.ron`: `kind: Bad` with `severity` ≥ 2 — the storm, a power
//!   cut, the fridge, a student party, a walk-out…), or any bad event asks for an answer;
//! * a regular walks in with a story talk waiting for tonight (a stage of their story);
//! * a stranger shows a little story (`StoryNoticed`);
//! * Classic: a waiting guest is about to give up and leave unhappy
//!   (`World::parties_losing_patience`);
//! * a helper falls ill or walks out;
//! * a crew request / bot message that needs an answer (approve / decline, a late or sick
//!   call, low stock);
//! * the café runs out of supplies (`World::out_of_supplies`).
//!
//! Never while something already holds the night (a cutscene, a story card to read, the walk
//! home, the storm's talk, photo mode, the pause menu, the morning summary) or the hands are
//! busy (a mini-game, a ritual gesture): a reason that comes then waits a few game minutes
//! and pauses once that is closed — except after a cutscene or a story card, which already
//! had the player's attention. Never by day, under screenshot automation, the autoplayer or
//! while the bot plays nights (`fast_forward`, `run_to_morning`).

use std::rc::Rc;

use macroquad::prelude::*;

use super::world_view::{self as wv, sim};
use super::{Ctx, GameScreen};
use crate::content::{BotKind, Content, EventKind};
use crate::settings::Locale;
use crate::ui::draw::{fill_rrect, with_a};
use crate::ui::{Align, Button, Glass, Icon, RectExt, Ui, draw_icon};
use sim::{Phase, SimEvent};

/// At most one auto-pause per this many game minutes.
pub const COOLDOWN_MINUTES: i32 = 15;
/// A reason that came while the night was held waits this long (game minutes).
const PENDING_MINUTES: i32 = 5;

/// Why the night paused itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Why {
    /// A bad event (`events.ron` id).
    Event(String),
    /// A regular came in with tonight's story talk (regular id).
    Regular(String),
    /// A stranger carries a little story.
    Story,
    /// Classic: a guest is about to leave unhappy.
    Patience,
    /// A helper called in sick (staff id).
    StaffSick(u32),
    /// A helper walked out (staff id).
    StaffQuit(u32),
    /// A message on the phone that needs an answer.
    Message,
    /// The bot's low-stock warning.
    LowStock,
    /// Nothing on the menu can be made any more.
    SuppliesOut,
}

impl Why {
    pub fn icon(&self) -> Icon {
        match self {
            Why::Event(id) => event_icon(id),
            Why::Regular(_) => Icon::Heart,
            Why::Story => Icon::Feather,
            Why::Patience => Icon::Clock,
            Why::StaffSick(_) | Why::StaffQuit(_) => Icon::Crew,
            Why::Message => Icon::Phone,
            Why::LowStock | Why::SuppliesOut => Icon::Bean,
        }
    }

    /// The reason line after "Paused:".
    pub fn text(&self, g: &GameScreen, loc: &Locale) -> String {
        let staff = |id: u32| {
            g.world
                .staff_by_id(id)
                .map(|s| wv::staff_name(loc, s))
                .unwrap_or_default()
        };
        match self {
            Why::Event(id) => loc.tr_owned(&format!("event.{id}.title")),
            Why::Regular(r) => {
                let name = loc.tr_owned(&format!("regular.{r}.name"));
                loc.trf("ui.autopause.regular", &[("name", &name)])
            }
            Why::Story => loc.tr_owned("ui.autopause.story"),
            Why::Patience => loc.tr_owned("ui.autopause.patience"),
            Why::StaffSick(id) => loc.trf("ui.autopause.staff_sick", &[("name", &staff(*id))]),
            Why::StaffQuit(id) => loc.trf("ui.autopause.staff_quit", &[("name", &staff(*id))]),
            Why::Message => loc.tr_owned("ui.autopause.message"),
            Why::LowStock => loc.tr_owned("ui.autopause.low_stock"),
            Why::SuppliesOut => loc.tr_owned("ui.autopause.supplies_out"),
        }
    }
}

/// The little picture for a bad event on the chip.
fn event_icon(id: &str) -> Icon {
    match id {
        "storm" => Icon::Cloud,
        "roof_leak" | "downpour" => Icon::Rain,
        "power_outage" | "storm_blackout" => Icon::Bolt,
        "fridge_fail" => Icon::Snowflake,
        "coffee_machine" => Icon::Coffee,
        "student_party" => Icon::Megaphone,
        "supplier_late" => Icon::Clock,
        "dine_and_dash" => Icon::Money,
        "broken_dishes" => Icon::Dishes,
        "staff_sick" | "staff_walkout" | "staff_quarrel" => Icon::Crew,
        _ => Icon::Warning,
    }
}

/// The auto-pause's state on the game screen (not saved: a loaded night starts fresh).
#[derive(Debug, Clone, Default)]
pub struct AutoPause {
    /// The global setting (copied from the settings every frame, like `relaxed`).
    pub setting: bool,
    /// The reason on screen while the night is auto-paused.
    pub shown: Option<Why>,
    /// A reason that came while something held the night: (why, night, minute).
    pending: Option<(Why, u32, i32)>,
    /// The last auto-pause: (night, minute since 22:00).
    last: Option<(u32, i32)>,
    /// Parties already warned about (patience).
    warned: Vec<u32>,
    /// The café was out of supplies at the last look (a pause on the change only).
    supplies_out: bool,
    /// The bot is playing nights (`fast_forward` / `run_to_morning`): never pause.
    pub driving: bool,
    /// Auto-pauses so far this session (tests, the debug overlay).
    pub count: u32,
}

impl GameScreen {
    /// The global setting or (sandbox) the setup's box.
    pub fn pause_on_events(&self) -> bool {
        self.auto.setting || self.world.sandbox_pauses_on_events()
    }

    /// Never: by day, under automation, the autoplayer or the bot.
    fn auto_pause_off(&self) -> bool {
        !self.pause_on_events()
            || self.automation
            || self.hands.autoplay.is_some()
            || self.hands.skip_scenes
            || self.auto.driving
            || self.world.phase() != Phase::Night
            || self.world.ending().is_some()
    }

    /// Something already holds the night (or the hands are busy): wait.
    fn auto_pause_held(&self) -> bool {
        self.pause.is_some()
            || self.summary.is_some()
            || self.walk.is_open()
            || self.photo.is_open()
            || self.stories.talk_open()
            || self.hands.minigame.is_some()
            || self.rituals.open.is_some()
            || self.clock_stop > 0.0
            || self.attention_taken()
    }

    /// A cutscene or a story card has the player's attention.
    fn attention_taken(&self) -> bool {
        self.cutscene_playing()
            || self.world.playing_cutscene().is_some()
            || !self.reading.is_empty()
    }

    fn now_minute(&self) -> (u32, i32) {
        (self.world.night(), self.world.minutes_since_open())
    }

    fn cooling_down(&self) -> bool {
        let (night, m) = self.now_minute();
        self.auto
            .last
            .is_some_and(|(n, at)| n == night && m - at < COOLDOWN_MINUTES)
    }

    /// The reason an event gives to pause, if any.
    fn auto_pause_reason(&self, content: &Content, e: &SimEvent) -> Option<Why> {
        let bad = |id: &str| {
            content
                .event(id)
                .filter(|d| d.kind == EventKind::Bad)
                .map(|d| d.severity)
        };
        match e {
            SimEvent::EventStarted { id } if bad(id).is_some_and(|s| s >= 2) => {
                Some(Why::Event(id.clone()))
            }
            SimEvent::ChoiceNeeded { id, .. } if bad(id).is_some() => Some(Why::Event(id.clone())),
            SimEvent::RegularArrived { regular, party } => {
                let leader = self
                    .world
                    .parties()
                    .iter()
                    .find(|p| p.id == *party)
                    .and_then(|p| p.members.first().copied())?;
                let talk = self.world.talk_available(content, leader)?;
                // a talk for tonight only (Anwar's visits), not a talk they have every night nor
                // the act's chat of a regular (talks.ron: open all act, no reason to stop the night)
                let once = content.cutscene(&talk).is_some_and(|c| {
                    !c.repeat
                        && matches!(
                            c.trigger,
                            crate::content::CutsceneTrigger::Talk { night: Some(_), .. }
                        )
                });
                once.then(|| Why::Regular(regular.clone()))
            }
            SimEvent::StoryNoticed { .. } => Some(Why::Story),
            SimEvent::StaffSick { staff } => Some(Why::StaffSick(*staff)),
            SimEvent::StaffWalkedOut { staff } => Some(Why::StaffQuit(*staff)),
            SimEvent::BotMessage { template, .. } => {
                let t = content.app.bot.iter().find(|b| b.id == *template)?;
                if t.id == "low_stock" {
                    Some(Why::LowStock)
                } else if matches!(
                    t.kind,
                    BotKind::LateAlert
                        | BotKind::SickCall
                        | BotKind::SwapRequest
                        | BotKind::TimeOffRequest
                ) || t.actions.iter().any(|a| a.id == "approve")
                {
                    Some(Why::Message)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Every sim event passes here (from `handle_event`).
    pub(super) fn auto_pause_event(&mut self, content: &Content, e: &SimEvent) {
        if self.auto_pause_off() {
            return;
        }
        if let Some(why) = self.auto_pause_reason(content, e) {
            self.want_auto_pause(why);
        }
    }

    fn want_auto_pause(&mut self, why: Why) {
        // already paused by the player, or paused a moment ago: nothing to add
        if self.speed == 0 || self.cooling_down() || self.auto.pending.is_some() {
            return;
        }
        let (n, m) = self.now_minute();
        // decided after the whole batch (`auto_pause_watch`): a cutscene from the same tick
        // takes the moment instead
        self.auto.pending = Some((why, n, m));
    }

    /// Pauses for the waiting reason once nothing holds the night.
    fn try_auto_pause(&mut self) {
        let Some((_, n, at)) = &self.auto.pending else {
            return;
        };
        let (night, m) = self.now_minute();
        let stale = *n != night || m - at > PENDING_MINUTES;
        if stale || self.auto_pause_off() || self.cooling_down() || self.attention_taken() {
            self.auto.pending = None;
            return;
        }
        if self.auto_pause_held() {
            return;
        }
        let Some((why, _, _)) = self.auto.pending.take() else {
            return;
        };
        if self.speed == 0 {
            // the player paused on their own meanwhile
            return;
        }
        self.toggle_pause();
        self.auto.shown = Some(why);
        self.auto.last = Some((night, m));
        self.auto.count += 1;
    }

    /// After every tick / event batch: the polled reasons (patience, supplies), a waiting
    /// reason, the chip goes once the night runs again.
    pub(super) fn auto_pause_watch(&mut self, content: &Content) {
        if self.auto.shown.is_some() && (self.speed > 0 || self.world.phase() != Phase::Night) {
            self.auto.shown = None;
        }
        if self.world.phase() != Phase::Night {
            self.auto.warned.clear();
            self.auto.supplies_out = false;
            self.auto.pending = None;
            return;
        }
        if self.auto_pause_off() {
            return;
        }
        let losing = self.world.parties_losing_patience();
        self.auto.warned.retain(|p| losing.contains(p));
        if let Some(p) = losing
            .iter()
            .find(|p| !self.auto.warned.contains(p))
            .copied()
        {
            self.auto.warned.push(p);
            self.want_auto_pause(Why::Patience);
        }
        let out = self.world.out_of_supplies(content);
        if out && !self.auto.supplies_out {
            self.want_auto_pause(Why::SuppliesOut);
        }
        self.auto.supplies_out = out;
        self.try_auto_pause();
    }

    /// Screenshot scenes (`SHIFTELLA_SCENE=auto_pause`): `event` starts now and the night is
    /// shown auto-paused for it (automation never pauses by itself).
    pub fn stage_auto_pause(&mut self, content: &Rc<Content>, loc: &Locale, event: &str) {
        self.world.debug_fire_event(content, event);
        self.drain_and_handle(content, Some(loc));
        if self.speed > 0 {
            self.toggle_pause();
        }
        self.auto.shown = Some(Why::Event(event.to_string()));
    }
}

/// "⏸ Paused: Thunderstorm   [▶ Continue · Space]" under the top bar while auto-paused.
pub fn chip(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let Some(why) = g.auto.shown.clone() else {
        return;
    };
    if g.speed > 0 {
        return;
    }
    let loc = cx.loc;
    let t = ui.theme.clone();
    let reason = why.text(g, &loc);
    let text = loc.trf("ui.autopause.chip", &[("reason", &reason)]);
    let st = t.small().color(t.text);
    let key = crate::settings::keys::label(crate::settings::keys::Action::Pause);
    let b = Button::new(loc.tr("ui.autopause.continue"))
        .primary()
        .icon(Icon::Play)
        .shortcut(&key);
    let bw = ui.button_width(&b);
    let h = 32.0;
    let s = ui.screen();
    let max_text = (s.w - 32.0 - 40.0 - bw - 12.0).max(80.0);
    let tw = ui.fonts.measure(&text, &st).min(max_text);
    let w = 34.0 + tw + 12.0 + bw + 4.0;
    // under the top row (the ticker, the clock, the follow chip) and the ticket rail, centred
    let top = g
        .anchors
        .iter()
        .filter(|a| {
            matches!(
                a.0,
                "hud.ticker" | "hud.clock" | "hud.money" | "hud.follow" | "rail"
            )
        })
        .filter(|a| a.1.x < s.w / 2.0 + w / 2.0 && a.1.right() > s.w / 2.0 - w / 2.0)
        .map(|a| a.1.bottom())
        .fold(12.0, f32::max);
    let r = Rect::new((s.w - w) / 2.0, top + 6.0, w, h);
    g.anchors.push(("hud.autopause", r));
    ui.glass(r, &Glass::hud().tint(t.amber));
    let a = 0.08 + 0.05 * (ui.time() as f32 * 2.5).sin().abs();
    fill_rrect(r, 10.0, with_a(t.amber, a));
    draw_icon(why.icon(), vec2(r.x + 17.0, r.center_y()), 14.0, t.amber);
    ui.fonts.draw_in(
        &text,
        Rect::new(r.x + 32.0, r.y, tw + 2.0, h),
        &st,
        Align::Left,
    );
    let br = Rect::new(r.right() - bw - 4.0, r.y + 3.0, bw, h - 6.0);
    if ui.button("autopause_continue", br, &b).clicked {
        g.toggle_pause();
    }
}
