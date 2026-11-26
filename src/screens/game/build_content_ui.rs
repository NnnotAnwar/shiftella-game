//! Package C in the game screen: toasts and ticker lines for the build requests, corners,
//! the attic and the back room, and the "Wishes for the café" cards on the Shiftella app's
//! Requests tab.

use std::rc::Rc;

use macroquad::prelude::*;

use super::world_view::{self as wv, sim};
use super::{Ctx, GameScreen, Panel};
use crate::content::Content;
use crate::settings::Locale;
use crate::ui::{Align, Button, Glass, Icon, RectExt, ToastKind, Tone, Ui, draw_icon};
use sim::{BuildRequest, BuildRequestStatus, BuildWho, SimEvent};

/// The display name of a request's person.
pub fn who_name(g: &GameScreen, loc: &Locale, who: &BuildWho) -> String {
    match who {
        BuildWho::Regular(r) => loc.tr_owned(&format!("regular.{r}.name")),
        BuildWho::Guests(a) => loc.tr_owned(&format!("guest.{a}.name")),
        BuildWho::Staff(id) => g
            .world
            .staff_by_id(*id)
            .map(|s| wv::staff_name(loc, s))
            .unwrap_or_default(),
    }
}

fn who_of(g: &GameScreen, content: &Content, id: &str) -> Option<BuildWho> {
    g.world
        .build_requests(content)
        .into_iter()
        .find(|r| r.id == id)
        .map(|r| r.who)
}

impl GameScreen {
    /// Toasts / ticker lines for package C's events (called from `handle_event`).
    pub(super) fn build_content_event(
        &mut self,
        content: &Rc<Content>,
        e: &SimEvent,
        loc: Option<&Locale>,
    ) {
        let Some(loc) = loc else { return };
        match e {
            SimEvent::BuildWishOpened { id, .. } => {
                let Some(who) = who_of(self, content, id) else {
                    return;
                };
                let name = who_name(self, loc, &who);
                let reason = loc.tr_owned(&format!("build_wish.{id}.reason"));
                let t = loc.trf(
                    "ui.requests.opened",
                    &[("name", &name), ("reason", &reason)],
                );
                self.push_ticker(Icon::Build, t);
            }
            SimEvent::BuildWishFulfilled { id, .. } => {
                let Some(who) = who_of(self, content, id) else {
                    return;
                };
                let name = who_name(self, loc, &who);
                let line = loc.tr_owned(&format!("build_wish.{id}.thanks"));
                self.push_ticker(Icon::Heart, format!("{name}: {line}"));
                self.toast(ToastKind::Success, Icon::Heart, name, Some(line));
            }
            SimEvent::CornerFormed { id, .. } => {
                let Some(def) = content.build.corner(id) else {
                    return;
                };
                let name = loc.tr_owned(&def.name_key());
                let title = loc.trf("ui.corner.formed", &[("name", &name)]);
                let bonus = crate::i18n::format_decimal(loc.lang, def.bonus, 1);
                let body = format!(
                    "{} · {}",
                    loc.tr(&def.desc_key()),
                    loc.trf("ui.corner.bonus", &[("n", &bonus)])
                );
                self.push_ticker(Icon::Sparkle, title.clone());
                self.toast(ToastKind::Success, Icon::Sparkle, title, Some(body));
            }
            SimEvent::BackRoomOpened { .. } => {
                // the café is wider now: the world statics rebuild by their key
                self.push_ticker(Icon::Door, loc.tr_owned("ui.backroom.title"));
            }
            _ => {}
        }
    }
}

/// An open request's card (title + their line); a thanked one is a single row.
const CARD_H: f32 = 56.0;
const DONE_H: f32 = 30.0;

fn card_h(r: &BuildRequest) -> f32 {
    // a larger text size: taller rows (the title and two lines of their words)
    let grow = (crate::ui::text::text_scale() - 1.0) / 0.3;
    if matches!(r.status, BuildRequestStatus::Thanked(_)) {
        DONE_H + 6.0 * grow
    } else {
        CARD_H + 20.0 * grow
    }
}

/// Height the build-request cards take in the Requests tab (0 = none).
pub fn requests_height(g: &GameScreen, content: &Content) -> f32 {
    let list = g.world.build_requests(content);
    if list.is_empty() {
        0.0
    } else {
        22.0 + list.iter().map(|r| card_h(r) + 6.0).sum::<f32>()
    }
}

/// "Wishes for the café": what the regulars, the crew and the guests would like built, with
/// their line, the state and a one-click jump into build mode. Scrolls inside its rect.
pub fn requests_section(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx, r: Rect) {
    let t = ui.theme.clone();
    let loc = cx.loc;
    let content = cx.content;
    let list: Vec<BuildRequest> = g.world.build_requests(content);
    if list.is_empty() {
        return;
    }
    ui.fonts.draw_in(
        loc.tr("ui.requests.build_title"),
        Rect::new(r.x + 2.0, r.y, r.w, 18.0),
        &t.micro(),
        Align::Left,
    );
    let body = Rect::new(r.x, r.y + 22.0, r.w, r.h - 22.0);
    let total = list.iter().map(|r| card_h(r) + 6.0).sum::<f32>();
    let sa = ui.begin_scroll("build_req_scroll", body, total);
    let mut y = sa.top();
    let w = body.w - 10.0;
    for req in &list {
        let ch = card_h(req);
        let cr = Rect::new(body.x, y, w, ch);
        y += ch + 6.0;
        if !sa.visible(cr.y, cr.h) {
            continue;
        }
        let done = matches!(req.status, BuildRequestStatus::Thanked(_));
        let tint = match req.status {
            BuildRequestStatus::Waiting if req.arc => Some(t.amber),
            BuildRequestStatus::Built => Some(t.emerald),
            _ => None,
        };
        let glass = match tint {
            Some(c) => Glass::card().tint(c),
            None => Glass::card(),
        };
        ui.glass(cr, &glass);
        let c = cr.inset_xy(10.0, 7.0);
        let icon = match req.who {
            BuildWho::Regular(_) => Icon::Heart,
            BuildWho::Staff(_) => Icon::Crew,
            BuildWho::Guests(_) => Icon::User,
        };
        let dim = if done { t.text_dim } else { t.violet_400 };
        draw_icon(icon, vec2(c.x + 8.0, c.y + 8.0), 14.0, dim);
        let name = who_name(g, &loc, &req.who);
        let reason = loc.tr_owned(&req.reason_key);
        let (chip, tone) = match req.status {
            BuildRequestStatus::Waiting => (loc.tr("ui.requests.waiting"), Tone::Warn),
            BuildRequestStatus::Built => (loc.tr("ui.requests.built"), Tone::Ok),
            BuildRequestStatus::Thanked(_) => (loc.tr("ui.requests.thanked"), Tone::Neutral),
        };
        let chip_w = ui.chip_width(chip, false);
        let btn = Button::new(loc.tr("ui.requests.build"))
            .ghost()
            .icon(Icon::Build)
            .tooltip(loc.tr("ui.toolbar.build"));
        let bw = if req.status == BuildRequestStatus::Waiting {
            ui.button_width(&btn).min(110.0)
        } else {
            0.0
        };
        let title_w = (c.w - 22.0 - chip_w - bw - 16.0).max(40.0);
        ui.fonts.draw_in(
            &format!("{name} · {reason}"),
            Rect::new(c.x + 20.0, c.y, title_w, 16.0),
            &t.strong().color(if done { t.text_dim } else { t.text }),
            Align::Left,
        );
        ui.chip(
            vec2(
                c.right() - chip_w - bw - if bw > 0.0 { 6.0 } else { 0.0 },
                c.y - 3.0,
            ),
            chip,
            tone,
            None,
        );
        if bw > 0.0 {
            let br = Rect::new(c.right() - bw, c.y - 4.0, bw, 24.0);
            if ui.button(("build_req", req.id.as_str()), br, &btn).clicked {
                g.panel = Some(Panel::Build);
            }
        }
        if done {
            // a thanked one stays as a short row; their thank-you on hover
            let resp = ui.interact(ui.id(("build_req_done", req.id.as_str())), cr);
            let thanks = format!("“{}”", loc.tr(&req.thanks_key));
            ui.tooltip(&resp, &thanks);
            continue;
        }
        // their own words; a story request says so
        let mut line = format!("“{}”", loc.tr(&req.ask_key));
        if req.arc && !done {
            line = format!("{line} · {}", loc.tr("ui.requests.story"));
        }
        let tl = t.strong().line_h().max(18.0);
        ui.fonts.draw_wrapped(
            &line,
            Rect::new(
                c.x + 20.0,
                c.y + tl,
                c.w - 20.0,
                cr.bottom() - 4.0 - c.y - tl,
            ),
            &t.small().color(t.text_dim),
            Align::Left,
            Some(2),
        );
    }
    ui.end_scroll(sa);
}

/// `SHIFTELLA_SCENE=attic|backroom|requests|corner|before_after` (verification shots): the
/// story played by the bot up to night `night`, then package C's view of it.
pub fn scene_setup(
    g: &mut GameScreen,
    content: &Rc<Content>,
    loc: &Locale,
    name: &str,
    night: u32,
) {
    use sim::Command;
    let focus = |g: &mut GameScreen, t: (i32, i32), zoom: f32| {
        g.cam = g
            .scene
            .overview_camera(Rect::new(0.0, 0.0, screen_width(), screen_height()));
        g.cam.center = vec2(
            (t.0 as f32 + 0.5) * crate::map::TILE,
            (t.1 as f32 + 0.5) * crate::map::TILE,
        );
        g.cam.zoom *= zoom;
        g.cam_ready = true;
    };
    // a reading nook by grandma's bookshelf (28, 5)
    let nook = |g: &mut GameScreen| {
        for (def, x, y, rot) in [("armchair", 27, 6, 1u8), ("floor_lamp", 27, 4, 0)] {
            let _ = g.command(
                content,
                Command::PlaceFurniture {
                    def: def.into(),
                    x,
                    y,
                    rot,
                },
            );
        }
    };
    g.summary = None;
    match name {
        "attic" => g.panel = Some(Panel::Attic),
        "requests" => {
            g.fast_forward(content, loc, night, Some(150), true);
            g.panel = Some(Panel::App);
            g.ui_state.app_tab = 1;
        }
        "backroom" => {
            // the first day from `night` with the money for it
            let mut n = night;
            loop {
                if g.world.back_room(content).available {
                    let _ = g.command(content, Command::OpenBackRoom);
                }
                if g.world.back_room(content).open || n >= night + 8 {
                    break;
                }
                n += 1;
                g.fast_forward(content, loc, n, None, true);
            }
            g.fast_forward(content, loc, n, Some(150), true);
            // the whole (wider) café: the overview camera of the first frame
            g.hands.order = None;
            g.selected = None;
            g.cam_ready = false;
        }
        "corner" => {
            nook(g);
            g.panel = Some(Panel::Build);
            focus(g, (26, 5), 1.8);
        }
        "before_after" => {
            nook(g);
            g.fast_forward(content, loc, night, Some(479), true);
            g.run_to_morning(content, loc);
            if let Some((n, list)) = g.world.pending_polaroids() {
                let i = list
                    .iter()
                    .position(|h| h.kind == crate::content::handson::HighlightKind::BeforeAfter);
                g.photos.pick_night = n;
                g.photos.pick = i.unwrap_or(0);
                g.photos.develop = 1.0;
            }
        }
        _ => {}
    }
}
