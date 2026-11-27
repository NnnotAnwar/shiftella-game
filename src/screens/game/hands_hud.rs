//! Karolina's queue in the HUD and the follow camera (the owner's report: "where do I manage
//! the acts and the queue?", "how to stop following isn't obvious").
//!
//! * **The queue strip** — bottom-left over the dock, always there at night while she has
//!   something to do: her current act (with its progress) and every queued act as small icons
//!   and "3/8". A click (or its key, default **Q**, Settings → Controls) opens the list: each
//!   act with its icon and a short label, ↑ / ↓ and a drag to reorder, × to drop it, and
//!   "Clear all". One click each, no questions.
//! * **Follow** — the crew card's Follow keeps the camera on that person; a small chip at the
//!   top says so ("Following Karolina · Esc / click to stop" with ×). It stops on Esc, on
//!   the chip, on any camera move (drag, WASD / arrows, the wheel) and on the card's
//!   "Stop following".

use macroquad::prelude::*;

use super::hands::{Intent, karolina_acts, queue_icon};
use super::world_view::sim;
use super::{Ctx, GameScreen, Panel};
use crate::render::draw_queue_icon;
use crate::ui::draw::{fill_rrect, stroke_rrect};
use crate::ui::{Align, Glass, Icon, RectExt, Ui, Weight, draw_icon};
use sim::{AgentRef, PLAYER_QUEUE_MAX, Phase};

/// Width of the open list (pt).
const LIST_W: f32 = 248.0;
const ROW_H: f32 = 22.0;
const HEAD_H: f32 = 26.0;

/// Dock height + its margin + a gap: the strip sits just above the dock.
fn bottom(ui: &Ui, g: &GameScreen) -> f32 {
    let dock_top = g
        .anchors
        .iter()
        .filter(|a| a.0.starts_with("dock."))
        .map(|a| a.1.y)
        .fold(ui.screen().h - 12.0 - 36.0 + 4.0, f32::min);
    dock_top - 4.0 - 8.0
}

/// Is the strip shown at all now?
pub fn strip_shown(g: &GameScreen) -> bool {
    g.world.phase() == Phase::Night
        && g.summary.is_none()
        && g.world.karolina().is_some_and(|k| k.on_duty())
        && !matches!(
            g.panel,
            Some(Panel::Crew | Panel::Build | Panel::App | Panel::Journal | Panel::Payroll)
        )
        && !g.overlay_open()
}

/// The queue strip (UI pass, after the HUD).
pub fn queue_strip(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    if !strip_shown(g) {
        g.hands.queue_drag = None;
        return;
    }
    let loc = cx.loc;
    let content = cx.content;
    let t = ui.theme.clone();
    let acts = karolina_acts(&g.world);
    let has_current = g.world.player().current.is_some();
    let queued = g.world.player().queue.len();
    let progress = g
        .world
        .player()
        .current
        .as_ref()
        .map(|c| c.progress())
        .unwrap_or(0.0);
    let plan = g.world.karolina_plan();
    let labels: Vec<String> = plan
        .iter()
        .map(|a| super::crew_ui::activity_text(&loc, content, a))
        .collect();
    let icons: Vec<_> = acts
        .iter()
        .map(|a| queue_icon(content, &g.world, a))
        .collect();
    let key = crate::settings::keys::label(crate::settings::keys::Action::Queue);
    let x0 = 12.0;
    let by = bottom(ui, g);
    let count = format!("{queued}/{PLAYER_QUEUE_MAX}");
    if !g.hands.queue_open {
        // the closed strip: icons in a row (current first, with its ring), "3/8", the key
        let n = icons.len();
        let w = 10.0
            + n.max(1) as f32 * 20.0
            + 8.0
            + ui.fonts.measure(&count, &t.small())
            + 8.0
            + ui.keycap_width(&key)
            + 24.0;
        let r = Rect::new(x0, by - 30.0, w, 30.0);
        g.anchors.push(("hud.queue", r));
        let resp = ui.interact(ui.id("queue_strip"), r);
        ui.glass(r, &Glass::hud());
        if resp.hover_t > 0.01 {
            fill_rrect(r, 10.0, Color::new(1.0, 1.0, 1.0, 0.05 * resp.hover_t));
        }
        let mut x = r.x + 10.0;
        if n == 0 {
            draw_icon(Icon::User, vec2(x + 8.0, r.center_y()), 13.0, t.text_dim);
            x += 20.0;
        }
        for (i, ic) in icons.iter().enumerate() {
            let c = vec2(x + 8.0, r.center_y());
            if i == 0 && has_current {
                fill_rrect(
                    Rect::new(c.x - 9.0, c.y - 9.0, 18.0, 18.0),
                    5.0,
                    Color::new(0.39, 0.4, 0.95, 0.35),
                );
                ring(c, 10.5, progress, t.indigo_300);
            }
            draw_queue_icon(c, 10.0, *ic, ui.time() as f32);
            x += 20.0;
        }
        x += 8.0;
        let cw = ui.fonts.measure(&count, &t.small());
        ui.fonts.draw_in(
            &count,
            Rect::new(x, r.y, cw + 2.0, r.h),
            &t.small(),
            Align::Left,
        );
        x += cw + 8.0;
        ui.keycap(vec2(x, r.center_y()), &key, false);
        draw_icon(
            Icon::ChevronUp,
            vec2(r.right() - 12.0, r.center_y()),
            12.0,
            t.text_dim,
        );
        let tip = loc.trf("hux.queue.open_tip", &[("k", &key)]);
        ui.tooltip(&resp, &tip);
        if resp.clicked {
            g.hands.queue_open = true;
        }
        return;
    }
    // the open list
    let rows = acts.len();
    let empty_h = if rows == 0 { ROW_H } else { 0.0 };
    let h = HEAD_H + rows as f32 * ROW_H + empty_h + 8.0;
    let r = Rect::new(x0, by - h, LIST_W, h);
    g.anchors.push(("hud.queue", r));
    ui.glass(r, &Glass::hud());
    // header: title, n/8, key, Clear all, close
    let hy = r.y + 4.0;
    let title = loc.tr_owned("hux.queue.title");
    let st = t.small().weight(Weight::SemiBold).color(t.text);
    let tw = ui.fonts.measure(&title, &st);
    ui.fonts.draw_in(
        &title,
        Rect::new(r.x + 10.0, hy, tw + 2.0, 20.0),
        &st,
        Align::Left,
    );
    let cw = ui.fonts.measure(&count, &t.small());
    ui.fonts.draw_in(
        &count,
        Rect::new(r.x + 16.0 + tw, hy, cw + 2.0, 20.0),
        &t.small(),
        Align::Left,
    );
    ui.keycap(vec2(r.x + 22.0 + tw + cw, hy + 10.0), &key, false);
    let close = Rect::new(r.right() - 24.0, hy, 20.0, 20.0);
    if small_icon(
        ui,
        "queue_close",
        close,
        Icon::ChevronDown,
        loc.tr("hux.queue.close"),
    ) {
        g.hands.queue_open = false;
    }
    if queued > 0 {
        let label = loc.tr_owned("hux.queue.clear");
        let ls = t.small().weight(Weight::SemiBold);
        let lw = ui.fonts.measure(&label, &ls) + 12.0;
        let cr = Rect::new(close.x - lw - 4.0, hy, lw, 20.0);
        let resp = ui.interact(ui.id("queue_clear"), cr);
        if resp.hover_t > 0.01 {
            fill_rrect(cr, 5.0, Color::new(1.0, 1.0, 1.0, 0.07 * resp.hover_t));
        }
        ui.fonts.draw_in(
            &label,
            cr,
            &ls.color(if resp.hovered { t.text } else { t.indigo_300 }),
            Align::Center,
        );
        ui.tooltip(&resp, loc.tr("hux.queue.clear_tip"));
        if resp.clicked {
            ui.sound(crate::ui::UiSound::Click);
            g.intent(content, Intent::QueueClear);
        }
    }
    let mut y = r.y + HEAD_H;
    if rows == 0 {
        ui.fonts.draw_in(
            loc.tr("hux.queue.empty"),
            Rect::new(r.x + 10.0, y, r.w - 20.0, ROW_H),
            &t.small(),
            Align::Left,
        );
        return;
    }
    let first_queued = usize::from(has_current);
    let mut clicked: Option<Intent> = None;
    // a drag in progress: where it would land
    let drag = g.hands.queue_drag;
    let mouse = ui.mouse();
    let drop_at = drag.map(|_| {
        let k = ((mouse.y - (r.y + HEAD_H)) / ROW_H).floor() as i32 - first_queued as i32;
        k.clamp(0, queued.max(1) as i32 - 1) as usize
    });
    for (i, icon) in icons.iter().enumerate().take(rows) {
        let rr = Rect::new(r.x + 4.0, y, r.w - 8.0, ROW_H);
        y += ROW_H;
        let current = has_current && i == 0;
        let qi = i.saturating_sub(first_queued);
        let dragged = !current && drag.is_some_and(|d| d.0 == qi);
        let btn_w = if current { 20.0 } else { 58.0 };
        // the row's own hit area stops short of its buttons (the first press wins)
        let resp = ui.interact(
            ui.id(("queue_row", i)),
            Rect::new(rr.x, rr.y, rr.w - btn_w - 2.0, rr.h),
        );
        if ui.hovering(rr) || dragged {
            fill_rrect(
                rr,
                5.0,
                Color::new(1.0, 1.0, 1.0, if dragged { 0.1 } else { 0.05 }),
            );
        }
        if let Some(to) = drop_at
            && !current
            && to == qi
            && !dragged
        {
            stroke_rrect(rr, 5.0, 1.0, t.indigo_300);
        }
        // index / ring, the icon, the label
        let ic = vec2(rr.x + 12.0, rr.center_y());
        if current {
            fill_rrect(
                Rect::new(ic.x - 9.0, ic.y - 9.0, 18.0, 18.0),
                5.0,
                Color::new(0.39, 0.4, 0.95, 0.35),
            );
            ring(ic, 10.0, progress, t.indigo_300);
        }
        draw_queue_icon(ic, 10.0, *icon, ui.time() as f32);
        let lr = Rect::new(rr.x + 26.0, rr.y, rr.w - 28.0 - btn_w, rr.h);
        let ls = if current {
            t.small().weight(Weight::SemiBold).color(t.indigo_300)
        } else {
            t.small().color(t.text)
        };
        let text = labels.get(i).cloned().unwrap_or_default();
        ui.fonts.draw_in(&text, lr, &ls, Align::Left);
        // buttons: ↑ ↓ × (the current act: × only)
        let bx = rr.right() - 20.0;
        let xr = Rect::new(bx, rr.y + 1.0, 20.0, 20.0);
        let x_tip = if current {
            loc.tr("hux.queue.cancel_tip")
        } else {
            loc.tr("hux.queue.drop_tip")
        };
        if small_icon(ui, ("queue_x", i), xr, Icon::Close, x_tip) {
            clicked = Some(Intent::Unqueue(i));
        }
        if !current {
            let up = Rect::new(bx - 38.0, rr.y + 1.0, 19.0, 20.0);
            let down = Rect::new(bx - 19.0, rr.y + 1.0, 19.0, 20.0);
            if qi > 0
                && small_icon(
                    ui,
                    ("queue_up", i),
                    up,
                    Icon::ChevronUp,
                    loc.tr("hux.queue.up"),
                )
            {
                clicked = Some(Intent::QueueMove {
                    from: qi,
                    to: qi - 1,
                });
            }
            if qi + 1 < queued
                && small_icon(
                    ui,
                    ("queue_down", i),
                    down,
                    Icon::ChevronDown,
                    loc.tr("hux.queue.down"),
                )
            {
                clicked = Some(Intent::QueueMove {
                    from: qi,
                    to: qi + 1,
                });
            }
            // press on the label starts a drag
            if resp.pressed && drag.is_none() && lr.contains(mouse) && queued > 1 {
                g.hands.queue_drag = Some((qi, mouse.y - rr.y));
            }
            ui.tooltip(&resp, &text);
        } else {
            ui.tooltip(&resp, &text);
        }
    }
    // a drag ends where the mouse lets go
    if let Some((from, _)) = drag
        && !ui.mouse_down()
    {
        g.hands.queue_drag = None;
        if let Some(to) = drop_at
            && to != from
            && r.contains(mouse)
        {
            clicked = Some(Intent::QueueMove { from, to });
        }
    }
    // last: the rest of the card keeps clicks from falling through to the café
    let _ = ui.interact(ui.id("queue_card"), r);
    if let Some(i) = clicked {
        ui.sound(crate::ui::UiSound::Click);
        g.intent(content, i);
    }
}

/// A progress ring (UI px) round an icon.
fn ring(c: Vec2, r: f32, p: f32, col: Color) {
    let segs = 28;
    let k = (segs as f32 * p.clamp(0.0, 1.0)) as usize;
    for i in 0..k {
        let a0 = -std::f32::consts::FRAC_PI_2 + i as f32 / segs as f32 * std::f32::consts::TAU;
        let a1 = a0 + std::f32::consts::TAU / segs as f32;
        draw_line(
            c.x + a0.cos() * r,
            c.y + a0.sin() * r,
            c.x + a1.cos() * r,
            c.y + a1.sin() * r,
            1.8,
            col,
        );
    }
}

/// A small icon button (20 pt): returns clicked.
fn small_icon(ui: &mut Ui, key: impl std::hash::Hash, r: Rect, icon: Icon, tip: &str) -> bool {
    let t = ui.theme.clone();
    let resp = ui.interact(ui.id(key), r);
    if resp.hover_t > 0.01 {
        fill_rrect(r, 5.0, Color::new(1.0, 1.0, 1.0, 0.08 * resp.hover_t));
    }
    draw_icon(
        icon,
        r.center(),
        11.0,
        if resp.hovered { t.text } else { t.text_dim },
    );
    ui.tooltip(&resp, tip);
    resp.clicked
}

// ───────────────────────── follow ─────────────────────────

impl GameScreen {
    /// Follow someone with the camera (`None` stops). The crew card's Follow toggles it.
    pub fn set_follow(&mut self, who: Option<AgentRef>) {
        self.follow = who;
        if who.is_none() {
            self.cam_target = None;
        }
    }

    /// Esc while following: stops it (and nothing else this press). Returns whether it did.
    pub fn esc_stops_follow(&mut self) -> bool {
        if self.follow.is_some() {
            self.set_follow(None);
            true
        } else {
            false
        }
    }

    /// Per frame, after the camera's own input (`moved` = the player panned / zoomed it):
    /// a camera move stops following; otherwise the camera glides after them.
    pub fn follow_frame(&mut self, moved: bool, dt: f32) {
        let Some(a) = self.follow else {
            return;
        };
        if moved || self.world.phase() != Phase::Night {
            self.follow = None;
            return;
        }
        match self.scene.head_of(a, self.time) {
            Some(head) => {
                // the body, a little under the head, in the middle of the screen
                let want = head + vec2(0.0, 40.0);
                self.cam_target = None;
                self.cam.center = self.cam.center.lerp(want, (dt * 4.0).min(1.0));
            }
            None => self.follow = None,
        }
    }

    /// The name shown on the follow chip.
    fn follow_name(&self, loc: &crate::settings::Locale) -> Option<String> {
        match self.follow? {
            AgentRef::Staff(id) => {
                let s = self.world.staff_by_id(id)?;
                Some(if s.owner || s.name.is_empty() {
                    loc.tr_owned("cast.player.name")
                } else {
                    s.name.clone()
                })
            }
            AgentRef::Guest(g) => Some(super::guest_label::label(&self.world, loc, g)),
        }
    }
}

/// "Following Karolina · Esc / click to stop  ×" — a small chip at the top while following.
pub fn follow_chip(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let loc = cx.loc;
    let Some(name) = g.follow_name(&loc) else {
        return;
    };
    let t = ui.theme.clone();
    let text = loc.trf("hux.follow.chip", &[("name", &name)]);
    let st = t.small().weight(Weight::SemiBold).color(t.text);
    let w = ui.fonts.measure(&text, &st) + 30.0 + 24.0;
    let h = 26.0;
    // under the top HUD row (the ticker / the clock), centred
    let top = g
        .anchors
        .iter()
        .filter(|a| matches!(a.0, "hud.ticker" | "hud.clock" | "hud.money"))
        .filter(|a| {
            a.1.x < ui.screen().w / 2.0 + w / 2.0 && a.1.right() > ui.screen().w / 2.0 - w / 2.0
        })
        .map(|a| a.1.bottom())
        .fold(12.0, f32::max);
    let r = Rect::new((ui.screen().w - w) / 2.0, top + 6.0, w, h);
    g.anchors.push(("hud.follow", r));
    let resp = ui.interact(ui.id("follow_chip"), r);
    ui.glass(r, &Glass::hud().tint(t.indigo));
    if resp.hover_t > 0.01 {
        fill_rrect(r, 10.0, Color::new(1.0, 1.0, 1.0, 0.06 * resp.hover_t));
    }
    draw_icon(
        Icon::Camera,
        vec2(r.x + 14.0, r.center_y()),
        13.0,
        t.indigo_300,
    );
    ui.fonts.draw_in(
        &text,
        Rect::new(r.x + 26.0, r.y, w - 26.0 - 24.0, h),
        &st,
        Align::Left,
    );
    draw_icon(
        Icon::Close,
        vec2(r.right() - 13.0, r.center_y()),
        10.0,
        if resp.hovered { t.text } else { t.text_dim },
    );
    ui.tooltip(&resp, loc.tr("hux.follow.stop_tip"));
    if resp.clicked {
        ui.sound(crate::ui::UiSound::Click);
        g.set_follow(None);
    }
}

impl GameScreen {
    /// Screenshot scenes (`SHIFTELLA_SCENE=hands_ux`, `SHIFTELLA_HUX=checklist|rituals|
    /// queue|follow`): the checklist in its three states (the app / the boxes in the café),
    /// a full queue of eight with the strip open, following Karolina. Paused, so the shot
    /// shows the staged state.
    pub fn stage_hands_ux(&mut self, content: &crate::content::Content, which: &str) {
        use sim::{Command, PlayerAct};
        let ids: Vec<String> = content
            .app
            .checklists
            .iter()
            .map(|c| c.id.clone())
            .collect();
        match which {
            "checklist" | "rituals" => {
                // the opening list back to open (the fast-forward did it all), then: one
                // done, one on her way, one queued after it, the rest open
                let _ = self.world.apply(content, Command::PlayerClearQueue);
                let _ = self.world.apply(content, Command::PlayerCancel);
                for c in self.world.app.checklist.iter_mut() {
                    if c.id != "light_window" {
                        c.done = false;
                        c.by = None;
                        c.minute = None;
                        c.claim = None;
                    }
                }
                let done = ["coffee_warmup"];
                for id in done {
                    let _ = self
                        .world
                        .apply(content, Command::TickChecklist { id: id.into() });
                    for _ in 0..400 {
                        if matches!(
                            self.world.checklist_mark(id),
                            sim::ChecklistMark::Done { .. }
                        ) {
                            break;
                        }
                        self.world.tick(content);
                    }
                }
                let _ = self.world.drain_events();
                for id in ["count_float", "check_stock"] {
                    if ids.iter().any(|i| i == id) {
                        let _ = self
                            .world
                            .apply(content, Command::TickChecklist { id: id.into() });
                    }
                }
                self.world.tick(content);
                if which == "checklist" {
                    self.ui_state.app_tab = 4;
                    self.panel = Some(Panel::App);
                } else {
                    // the camera over the counter and the till
                    if let Some(uid) = self.world.window_light(content)
                        && let Some(p) = self.world.furniture_by_uid(uid)
                    {
                        let c = vec2(p.x as f32 + 3.0, p.y as f32 + 1.0) * crate::map::TILE;
                        self.cam.center = c;
                    }
                }
            }
            "queue" | "follow" => {
                let tiles: Vec<(i32, i32)> = (3..12)
                    .flat_map(|y| (2..14).map(move |x| (x, y)))
                    .filter(|t| self.world.grid().walkable(t.0, t.1))
                    .step_by(2)
                    .take(PLAYER_QUEUE_MAX + 1)
                    .collect();
                let _ = self.world.apply(content, Command::PlayerClearQueue);
                for t in tiles {
                    let _ = self.world.apply(
                        content,
                        Command::PlayerQueue {
                            act: PlayerAct::GoTo { tile: t },
                            front: false,
                        },
                    );
                    if self.world.player().current.is_none() {
                        self.world.tick(content);
                    }
                }
                // a checklist item and a tip among them, so the labels vary
                if let Some(i) = self.world.player().queue.len().checked_sub(1) {
                    let _ = self
                        .world
                        .apply(content, Command::PlayerUnqueue { index: i });
                    let _ = self.world.apply(
                        content,
                        Command::TickChecklist {
                            id: "count_float".into(),
                        },
                    );
                }
                let _ = self.world.drain_events();
                if which == "queue" {
                    self.hands.queue_open = true;
                } else {
                    self.open_crew_card("karolina");
                    // a closer camera (the overview can't move: the whole café fits)
                    self.cam = self.scene.overview_camera(Rect::new(
                        0.0,
                        0.0,
                        screen_width(),
                        screen_height(),
                    ));
                    self.cam.zoom *= 2.0;
                    self.cam_ready = true;
                    if let Some(k) = self.world.karolina() {
                        self.set_follow(Some(AgentRef::Staff(k.id)));
                    }
                }
            }
            _ => {}
        }
        self.set_speed(0);
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::content::Content;

    fn content() -> Rc<Content> {
        let root = env!("CARGO_MANIFEST_DIR");
        Rc::new(Content::load(&format!("{root}/assets/data")).expect("content"))
    }

    fn night(c: &Rc<Content>) -> GameScreen {
        let mut g = GameScreen::new_story(c, "cozy", true);
        g.start_night(c);
        g.scene.refresh(&g.world, 0.0, 0.0);
        g
    }

    #[test]
    fn following_glides_after_her_and_stops_on_esc_a_camera_move_or_the_toggle() {
        let c = content();
        let mut g = night(&c);
        let k = AgentRef::Staff(g.world.karolina().unwrap().id);
        g.cam.center = vec2(-500.0, -500.0);
        g.set_follow(Some(k));
        let before = g.cam.center;
        g.follow_frame(false, 0.1);
        assert!(g.follow.is_some());
        assert!(
            g.cam.center.distance(before) > 1.0,
            "the camera glides after her"
        );
        // a camera move (drag / WASD / wheel) stops it
        g.follow_frame(true, 0.1);
        assert!(g.follow.is_none());
        // Esc stops it (and only while following)
        g.set_follow(Some(k));
        assert!(g.esc_stops_follow());
        assert!(g.follow.is_none());
        assert!(!g.esc_stops_follow());
        // the card's toggle: Follow, then "Stop following"
        g.set_follow(Some(k));
        g.set_follow(None);
        assert!(g.follow.is_none() && g.cam_target.is_none());
        // a person no longer drawn (gone home) ends it too
        g.set_follow(Some(AgentRef::Staff(9999)));
        g.follow_frame(false, 0.1);
        assert!(g.follow.is_none());
    }

    #[test]
    fn the_strip_reorders_drops_and_clears_through_intents() {
        let c = content();
        let mut g = night(&c);
        g.world.skip_prep(&c);
        let tiles: Vec<(i32, i32)> = (3..12)
            .flat_map(|y| (2..14).map(move |x| (x, y)))
            .filter(|t| g.world.grid().walkable(t.0, t.1))
            .take(9)
            .collect();
        let mut ok = 0;
        for t in &tiles {
            if g.hand(
                &c,
                sim::Command::PlayerQueue {
                    act: sim::PlayerAct::GoTo { tile: *t },
                    front: false,
                },
            ) {
                ok += 1;
            }
        }
        assert_eq!(
            ok, PLAYER_QUEUE_MAX,
            "eight fit, the ninth gets the gentle toast"
        );
        let q = |g: &GameScreen| g.world.player().queue.iter().cloned().collect::<Vec<_>>();
        let first = q(&g)[0].clone();
        assert!(g.intent(&c, Intent::QueueMove { from: 0, to: 3 }));
        assert_eq!(q(&g)[3], first);
        assert!(g.intent(&c, Intent::Unqueue(1)));
        assert_eq!(q(&g).len(), PLAYER_QUEUE_MAX - 1);
        assert!(g.intent(&c, Intent::QueueClear));
        assert!(g.world.player().queue.is_empty());
    }
}
