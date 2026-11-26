//! Night moments on screen (design/handson.md §5.1): a glowing marker in the world (drawn by
//! the scene), a click opens the small glass card beside it, an option sends
//! `Command::ResolveMoment` and Karolina walks over (the card closes at once; her queue chip
//! shows the moment's icon). The lost traveller's "Check the Shiftella app" asks which train
//! first. Cards never block: the night runs slowly while one is open.

use macroquad::prelude::*;

use super::hud::world_to_ui;
use super::world_view::sim;
use super::{Ctx, GameScreen, Notice};
use crate::content::Content;
use crate::content::handson::Interaction;
use crate::i18n::format_money;
use crate::render::MomentMarkerKind;
use crate::settings::Locale;
use crate::ui::handson::{MomentAction, MomentOption, MomentView, moment_card, moment_card_rect};
use crate::ui::{Icon, ToastKind, Ui};
use sim::Command;

/// An open moment card.
#[derive(Debug, Clone, PartialEq)]
pub struct MomentOpen {
    pub id: String,
    /// `GameScreen::time` it opened.
    pub opened: f32,
    /// "Check the Shiftella app": the option waiting for its train.
    pub train_for: Option<String>,
}

/// The card's icon per marker kind.
pub fn icon_of(id: &str) -> Icon {
    use MomentMarkerKind as K;
    match MomentMarkerKind::for_moment(id) {
        K::Cat => Icon::Heart,
        K::Candle => Icon::Flame,
        K::Bucket | K::Umbrella => Icon::Rain,
        K::Window => Icon::Home,
        K::Busker | K::Jukebox => Icon::Vinyl,
        K::Drone => Icon::Plane,
        K::Traveller => Icon::Train,
        K::Blanket => Icon::Moon,
        K::Garland => Icon::Sparkle,
        K::Snow => Icon::Snowflake,
        K::Parcel => Icon::Note,
        K::Coffee => Icon::Coffee,
        K::Phone => Icon::Phone,
        K::Bulb => Icon::Bolt,
        K::Star => Icon::Star,
    }
}

impl GameScreen {
    /// Opens the card of a moment on the map (a marker click, **F**, the autoplayer).
    pub fn open_moment(&mut self, id: &str) -> bool {
        let ok = self
            .world
            .active_moments()
            .iter()
            .any(|m| m.id == id && m.resolving.is_none());
        if ok {
            self.hands.moment = Some(MomentOpen {
                id: id.to_string(),
                opened: self.time,
                train_for: None,
            });
            self.hands.order = None;
            self.bubble = None;
            self.hands.tut.bump("moment_opened");
        }
        ok
    }

    /// The nearest moment Karolina could answer (keyboard **F**).
    pub fn nearest_moment(&self) -> Option<String> {
        let here = self
            .world
            .karolina()
            .map(|k| vec2(k.pos.0, k.pos.1))
            .unwrap_or_default();
        self.scene
            .markers
            .iter()
            .filter(|(id, _)| {
                self.world
                    .active_moments()
                    .iter()
                    .any(|m| &m.id == id && m.resolving.is_none())
            })
            .min_by(|a, b| {
                a.1.anchor
                    .distance(here)
                    .total_cmp(&b.1.anchor.distance(here))
            })
            .map(|(id, _)| id.clone())
            .or_else(|| {
                self.world
                    .active_moments()
                    .iter()
                    .find(|m| m.resolving.is_none())
                    .map(|m| m.id.clone())
            })
    }

    /// Picks an option: Karolina walks over (a train picker asks first).
    pub fn moment_option(&mut self, content: &Content, id: &str, option: &str) -> bool {
        let picker = content
            .handson
            .moment(id)
            .and_then(|d| d.option(option))
            .is_some_and(|o| matches!(o.interaction, Some(Interaction::TrainPicker { .. })));
        if picker
            && self
                .hands
                .moment
                .as_ref()
                .is_some_and(|m| m.id == id && m.train_for.is_none())
            && self.hands.autoplay.is_none()
        {
            if let Some(m) = &mut self.hands.moment {
                m.train_for = Some(option.to_string());
            }
            return true;
        }
        self.hands.moment = None;
        match self.world.apply(
            content,
            Command::ResolveMoment {
                id: id.to_string(),
                option: option.to_string(),
            },
        ) {
            Ok(()) => {
                self.hands.tut.bump("moment_picked");
                true
            }
            Err(sim::CommandError::QueueFull) => {
                self.notices.push(Notice::Key {
                    kind: ToastKind::Info,
                    icon: Icon::Info,
                    key: "ui.queue.full".into(),
                    args: Vec::new(),
                });
                false
            }
            Err(_) => false,
        }
    }

    /// Moment events: a result line when Karolina got there, a gentle note on expiry.
    pub fn moment_event(&mut self, content: &Content, e: &sim::SimEvent) {
        match e {
            sim::SimEvent::MomentAvailable { .. } => self.hands.tut.bump("moment_shown"),
            sim::SimEvent::MomentResolved { id, option } => {
                self.hands.tut.bump("moment_resolved");
                if let Some(d) = content.handson.moment(id) {
                    self.notices.push(Notice::KeyTitled {
                        kind: ToastKind::Success,
                        icon: icon_of(id),
                        title: d.title_key(),
                        key: d.result_key(option),
                        args: Vec::new(),
                    });
                }
            }
            // (the sim toasts `moment.<id>.expired` itself when the expiry did something)
            sim::SimEvent::MomentExpired { id }
                if self.hands.moment.as_ref().is_some_and(|m| &m.id == id) =>
            {
                self.hands.moment = None;
            }
            _ => {}
        }
    }

    /// The autoplayer answers a moment the way a calm player would (the first option).
    pub fn autoplay_moment(&mut self, content: &Content) -> bool {
        let Some(m) = self
            .world
            .active_moments()
            .iter()
            .find(|m| m.resolving.is_none())
            .cloned()
        else {
            return false;
        };
        let opts = self.world.moment_options(content, &m.id);
        // the card's first option (the warm one), unless it costs real money and a free one
        // is offered too
        let costly = |o: &String| {
            content
                .handson
                .moment(&m.id)
                .and_then(|d| d.option(o))
                .is_some_and(|o| o.cost > 200)
        };
        let pick = opts
            .iter()
            .find(|o| !costly(o))
            .or_else(|| opts.first())
            .cloned();
        match pick {
            Some(o) => self.moment_option(content, &m.id, &o),
            None => false,
        }
    }
}

/// The view of a moment's card.
fn view(
    g: &GameScreen,
    content: &Content,
    loc: &Locale,
    id: &str,
) -> Option<(MomentView, Vec<String>)> {
    let def = content.handson.moment(id)?;
    let m = g.world.active_moments().iter().find(|m| m.id == id)?;
    let now = g.world.minute();
    let span = m.expires.saturating_sub(m.spawned).max(1) as f32;
    let left = m.expires.saturating_sub(now) as f32 / span;
    let ids = g.world.moment_options(content, id);
    let options = ids
        .iter()
        .filter_map(|o| def.option(o))
        .map(|o| {
            let mut b = MomentOption::new(loc.tr_owned(&def.option_key(&o.id)));
            if o.cost > 0 {
                b = b.cost(format!("−{}", format_money(loc.lang, o.cost)));
            }
            if matches!(o.interaction, Some(Interaction::TrainPicker { .. })) {
                b = b.icon(Icon::Phone);
            }
            b
        })
        .collect();
    let text_key = def.text_key();
    Some((
        MomentView {
            icon: icon_of(id),
            title: loc.tr_owned(&def.title_key()),
            text: loc.has(&text_key).then(|| loc.tr_owned(&text_key)),
            options,
            expires: Some(left.clamp(0.0, 1.0)),
        },
        ids,
    ))
}

/// The train picker of "Check the Shiftella app": the next trains, the right one first.
fn train_view(
    g: &GameScreen,
    content: &Content,
    loc: &Locale,
    id: &str,
    option: &str,
) -> MomentView {
    let def = content.handson.moment(id);
    let n = def
        .and_then(|d| d.option(option))
        .and_then(|o| match o.interaction {
            Some(Interaction::TrainPicker { trains }) => Some(trains),
            _ => None,
        })
        .unwrap_or(3)
        .clamp(2, 4) as usize;
    let now = g.world.minute();
    let mut trains: Vec<_> = content
        .trains
        .trains
        .iter()
        .filter(|t| t.announce_if.is_empty())
        .map(|t| {
            let m = t.arrives.minutes_since_open();
            ((m - now as i32).rem_euclid(480), t)
        })
        .collect();
    trains.sort_by_key(|t| t.0);
    let options = trains
        .iter()
        .take(n)
        .enumerate()
        .map(|(i, (_, t))| {
            let key = format!("train.{}.name", t.id.to_lowercase().replace(' ', "_"));
            let name = if loc.has(&key) {
                loc.tr_owned(&key)
            } else {
                format!("{} → {}", t.id, t.to)
            };
            let b = MomentOption::new(format!("{} · {name}", t.arrives)).icon(Icon::Train);
            if i == 0 { b.gain("✓") } else { b }
        })
        .collect();
    MomentView {
        icon: Icon::Phone,
        title: def
            .map(|d| loc.tr_owned(&d.option_key(option)))
            .unwrap_or_default(),
        text: def.map(|d| loc.tr_owned(&d.title_key())),
        options,
        expires: None,
    }
}

/// The open moment card (call with the other overlays).
pub fn draw(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let Some(open) = g.hands.moment.clone() else {
        return;
    };
    let content = cx.content;
    let loc = cx.loc;
    let still = g
        .world
        .active_moments()
        .iter()
        .any(|m| m.id == open.id && m.resolving.is_none());
    if !still || g.world.phase() != sim::Phase::Night {
        g.hands.moment = None;
        return;
    }
    let anchor = g
        .scene
        .markers
        .iter()
        .find(|(id, _)| *id == open.id)
        .map(|(_, m)| world_to_ui(g, ui, m.orb(g.time)))
        .unwrap_or(vec2(ui.screen().w / 2.0, ui.screen().h / 2.0));
    let mut avoid = super::hands_ui::hud_avoid(g);
    if let Some(r) = g.anchors.iter().find(|a| a.0 == "order.card") {
        avoid.push(r.1);
    }
    avoid.extend(toast_zone(g, ui));
    let (v, ids) = match &open.train_for {
        Some(opt) => {
            let v = train_view(g, content, &loc, &open.id, opt);
            let n = v.options.len();
            (v, vec![opt.clone(); n])
        }
        None => match view(g, content, &loc, &open.id) {
            Some(x) => x,
            None => {
                g.hands.moment = None;
                return;
            }
        },
    };
    let r = moment_card_rect(ui, anchor, &v, &avoid);
    g.anchors.push(("moment.card", r));
    match moment_card(
        ui,
        ("moment", open.id.as_str(), open.train_for.is_some()),
        anchor,
        &v,
        loc.tr("ui.common.close"),
        &avoid,
    ) {
        Some(MomentAction::Pick(i)) => {
            if let Some(opt) = ids.get(i).cloned() {
                if open.train_for.is_some() {
                    // the picker answered: now she walks over
                    g.hands.moment = None;
                    let _ = g.command(
                        content,
                        Command::ResolveMoment {
                            id: open.id.clone(),
                            option: opt,
                        },
                    );
                    g.hands.tut.bump("moment_picked");
                } else {
                    g.moment_option(content, &open.id, &opt);
                }
            }
        }
        Some(MomentAction::Dismiss) => g.hands.moment = None,
        None => {}
    }
}

/// The bottom-right column where tonight's toasts are (cards keep out of it).
pub fn toast_zone(g: &GameScreen, ui: &Ui) -> Option<Rect> {
    let fresh = g
        .recent_toasts
        .iter()
        .filter(|(_, _, at)| g.time - at < 7.0)
        .count();
    (fresh > 0).then(|| {
        let s = ui.screen();
        let h = 96.0 * fresh.min(2) as f32 + 16.0;
        Rect::new(s.w - 344.0, s.h - 56.0 - h, 344.0, h)
    })
}
