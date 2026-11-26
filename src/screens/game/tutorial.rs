//! First-time tutorial: gentle, skippable coach marks for Day 1 and Night 1 (offered on the
//! first sandbox day too), and the "How to play" reference page (F1 / `?`).
//!
//! A coach mark is a small glass card with an arrow at the UI it talks about and a soft
//! pulsing ring around that target. Nothing blocks: the card only covers itself, steps that
//! ask for an action finish when the player does it (or on **Next**), **Skip tutorial**
//! ends it for good. At night a waiting card holds the clock so it can be read in peace.
//! Progress lives in the save ([`crate::save::TutorialProgress`]); finishing or skipping
//! turns `Settings::show_tutorial` off, "Replay tutorial" in Settings starts it again.

use macroquad::prelude::*;

use super::world_view::sim;
use super::{Ctx, GameScreen, Panel};
use crate::save::TutorialProgress;
use crate::settings::Locale;
use crate::ui::draw::{self, Paint, fill_convex, fill_rrect, stroke_rrect, with_a};
use crate::ui::{Align, Button, Glass, Icon, RectExt, Ui, draw_icon};
use sim::Phase;

/// One coach mark of the first day (the first night is taught by `coach` — Karolina's
/// hands-on coach marks from `tutorial.ron`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Welcome,
    App,
    Build,
    Start,
}

pub const STEPS: [Step; 4] = [Step::Welcome, Step::App, Step::Build, Step::Start];

/// Index where the night coach marks take over.
pub const FIRST_NIGHT: usize = STEPS.len();

impl Step {
    pub fn id(self) -> &'static str {
        match self {
            Step::Welcome => "welcome",
            Step::App => "app",
            Step::Build => "build",
            Step::Start => "start",
        }
    }

    /// Day steps only (kept for the card layout code).
    pub fn night(self) -> bool {
        false
    }

    /// The panel this step is about (its card moves beside the panel while it is open).
    fn panel(self) -> Option<Panel> {
        match self {
            Step::App => Some(Panel::App),
            Step::Build => Some(Panel::Build),
            _ => None,
        }
    }

    /// UI anchor the arrow points at (see `GameScreen::anchors`).
    fn target(self) -> Option<&'static str> {
        match self {
            Step::Welcome => None,
            Step::App => Some("dock.app"),
            Step::Build => Some("dock.build"),
            Step::Start => Some("day.start"),
        }
    }
}

/// Index of the step to show (or `None` when the tutorial is not running).
fn current(p: &TutorialProgress) -> Option<usize> {
    (p.active && (p.step as usize) < STEPS.len()).then_some(p.step as usize)
}

impl GameScreen {
    /// Starts (or restarts) the coach marks for this game.
    pub fn start_tutorial(&mut self) {
        let step = if self.world.phase() == Phase::Night {
            FIRST_NIGHT as u32
        } else {
            0
        };
        self.extra.tutorial = Some(TutorialProgress {
            active: true,
            step,
            ..Default::default()
        });
    }

    /// The coach mark on screen now (none during cutscenes, the summary or the pause menu).
    pub fn tutorial_step(&self) -> Option<Step> {
        let p = self.extra.tutorial.as_ref()?;
        let i = current(p)?;
        let free = !self.cutscene_playing()
            && self.world.playing_cutscene().is_none()
            && self.summary.is_none()
            && self.pause.is_none();
        (self.world.phase() == Phase::Day && free).then_some(STEPS[i])
    }

    /// Day cards are done: the night coach marks take over (the tutorial stays active).
    fn tutorial_advance(&mut self) {
        if let Some(p) = &mut self.extra.tutorial {
            p.step += 1;
            p.panel_seen = false;
        }
    }

    /// Logic: steps that finish when the player does the thing. Call once per frame.
    pub fn tutorial_update(&mut self, cx: &mut Ctx) {
        let Some(p) = self.extra.tutorial.clone() else {
            return;
        };
        if !p.active {
            return;
        }
        // past the coach marks' last night (a loaded save): done
        if self.world.night() > super::coach::LAST_NIGHT {
            self.finish_tutorial(cx);
            return;
        }
        let Some(i) = current(&p) else { return };
        let step = STEPS[i];
        // the night started before the day cards were through: the night marks take over
        if self.world.phase() == Phase::Night {
            if let Some(p) = &mut self.extra.tutorial {
                p.step = FIRST_NIGHT as u32;
                p.panel_seen = false;
            }
            return;
        }
        let open = step.panel().is_some() && self.panel == step.panel();
        if open
            && !p.panel_seen
            && let Some(p) = &mut self.extra.tutorial
        {
            p.panel_seen = true;
        }
        let done = match step {
            Step::App | Step::Build => p.panel_seen && !open,
            _ => false,
        };
        if done {
            self.tutorial_advance();
        }
    }

    /// Ends the tutorial for good (finished or skipped): new games won't show it again.
    pub fn finish_tutorial(&mut self, cx: &mut Ctx) {
        if let Some(p) = &mut self.extra.tutorial {
            p.active = false;
        }
        if cx.settings.show_tutorial {
            cx.settings.show_tutorial = false;
            if let Err(e) = cx.settings.save() {
                eprintln!("warning: settings not saved: {e}");
            }
        }
    }

    fn anchor(&self, name: &str) -> Option<Rect> {
        self.anchors.iter().find(|a| a.0 == name).map(|a| a.1)
    }
}

/// Where a `w × h` card goes next to `target`: beside, above or below it, whichever
/// fits on screen without covering `avoid`.
fn place(screen: Rect, target: Rect, w: f32, h: f32, avoid: &[Rect]) -> (Rect, Vec2) {
    let gap = 18.0;
    let c = target.center();
    let clamp_x = |x: f32| x.clamp(12.0, screen.w - 12.0 - w);
    let clamp_y = |y: f32| y.clamp(12.0, screen.h - 12.0 - h);
    let candidates = [
        // above, centred on the target
        (
            Rect::new(clamp_x(c.x - w / 2.0), target.y - gap - h, w, h),
            vec2(c.x, target.y - 4.0),
        ),
        // right
        (
            Rect::new(target.right() + gap, clamp_y(c.y - h / 2.0), w, h),
            vec2(target.right() + 4.0, c.y),
        ),
        // below
        (
            Rect::new(clamp_x(c.x - w / 2.0), target.bottom() + gap, w, h),
            vec2(c.x, target.bottom() + 4.0),
        ),
        // left
        (
            Rect::new(target.x - gap - w, clamp_y(c.y - h / 2.0), w, h),
            vec2(target.x - 4.0, c.y),
        ),
    ];
    let fits = |r: &Rect| {
        r.x >= 8.0
            && r.y >= 8.0
            && r.right() <= screen.w - 8.0
            && r.bottom() <= screen.h - 8.0
            && !avoid.iter().any(|a| a.overlaps(r))
    };
    candidates
        .iter()
        .find(|(r, _)| fits(r))
        .or_else(|| {
            candidates.iter().find(|(r, _)| {
                r.x >= 8.0
                    && r.y >= 8.0
                    && r.right() <= screen.w - 8.0
                    && r.bottom() <= screen.h - 8.0
            })
        })
        .copied()
        .unwrap_or((
            // no room around a big panel: tuck the card into its lower-right corner,
            // above a tab bar, where panels keep the least important things
            Rect::new(
                clamp_x(target.right() - w - 16.0),
                clamp_y(target.bottom() - h - 100.0),
                w,
                h,
            ),
            target.center(),
        ))
}

/// The coach mark. Call after the HUD and panels (they register `anchors`).
pub fn draw(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let Some(step) = g.tutorial_step() else {
        return;
    };
    let Some(i) = g.extra.tutorial.as_ref().map(|p| p.step as usize) else {
        return;
    };
    let loc = cx.loc;
    let t = ui.theme.clone();
    let s = ui.screen();
    let sandbox = g.world.mode() == sim::Mode::Sandbox;
    let key = |part: &str| {
        let k = format!("tutorial.{}.{part}", step.id());
        if sandbox && step == Step::Welcome {
            format!("tutorial.welcome_sandbox.{part}")
        } else {
            k
        }
    };
    let title = loc.tr_owned(&key("title"));
    let body = loc.tr_owned(&key("body"));

    // layout
    let w = 312.0_f32.min(s.w - 24.0);
    let body_st = t.small().size(13.0).line_height(1.45).color(t.text_dim);
    let title_st = t.strong().size(15.0);
    let lines = ui.fonts.wrap(&body, w - 32.0, &body_st);
    let h = 16.0 + 16.0 + 24.0 + lines.len() as f32 * body_st.line_h() + 14.0 + 28.0 + 14.0;
    let (n, of) = (i + 1, STEPS.len());
    let panel_open = step.panel().is_some() && g.panel == step.panel();
    let target = if panel_open {
        g.anchor("panel")
    } else {
        step.target().and_then(|a| g.anchor(a)).or_else(|| {
            (step.target() == Some("toasts"))
                .then(|| Rect::new(s.w - 12.0 - 360.0, s.h - 56.0 - 110.0, 360.0, 100.0))
        })
    };
    let avoid: Vec<Rect> = ["day.panel", "hud.ticker", "hud.money", "hud.clock", "panel"]
        .iter()
        .filter(|n| step.target() != Some(**n) && !(panel_open && **n == "panel"))
        .filter_map(|n| g.anchor(n))
        .collect();
    let (r, tip) = match target {
        Some(tr) => {
            let (r, tip) = place(s, tr, w, h, &avoid);
            (r, Some((tr, tip)))
        }
        None => (
            Rect::new(
                ((s.w - w) / 2.0).round(),
                (s.h * 0.34 - h / 2.0).round(),
                w,
                h,
            ),
            None,
        ),
    };

    // appear: fade + 6 pt rise per step
    let id = ui.id(("tutorial", i));
    let k = ui.ease_value(id.with("appear"), 1.0, 0.08);
    let r = r.translate(vec2(0.0, (1.0 - k) * 6.0));
    draw::push_alpha(k);

    // pulsing ring around the target (not around a whole open panel)
    if let Some((tr, tip)) = tip
        && !panel_open
    {
        let pulse = 0.5 + 0.5 * ((ui.time() * 2.4).sin() as f32);
        let ring = tr.grow(4.0 + 3.0 * pulse);
        stroke_rrect(ring, 12.0, 2.0, with_a(t.indigo_300, 0.55 + 0.35 * pulse));
        stroke_rrect(
            ring.grow(4.0),
            14.0,
            1.0,
            with_a(t.violet_400, 0.25 * pulse),
        );
        // arrow from the card towards the target
        let from = vec2(
            tip.x.clamp(r.x + 18.0, r.right() - 18.0),
            tip.y.clamp(r.y + 18.0, r.bottom() - 18.0),
        );
        let dir = (tip - from).normalize_or_zero();
        if dir.length() > 0.5 {
            let n = vec2(-dir.y, dir.x);
            let base = edge_point(r, from, dir);
            let point = base + dir * 11.0;
            fill_convex(
                &[base + n * 8.0, point, base - n * 8.0],
                Paint::Solid(t.glass_solid_top),
            );
        }
    }

    ui.glass(r, &Glass::tooltip().radius(12.0));
    let accent = Rect::new(r.x, r.y + 14.0, 3.0, 22.0);
    fill_rrect(accent, 1.5, Paint::vertical(accent, t.indigo_300, t.violet));
    let mut y = r.y + 16.0;
    let label = format!("{} · {n}/{of}", loc.tr("ui.tutorial.label").to_uppercase());
    ui.fonts.draw_in(
        &label,
        Rect::new(r.x + 16.0, y, r.w - 32.0, 14.0),
        &t.micro().color(t.indigo_300),
        Align::Left,
    );
    draw_icon(
        Icon::Sparkle,
        vec2(r.right() - 22.0, y + 7.0),
        13.0,
        with_a(t.amber, 0.9),
    );
    y += 18.0;
    ui.fonts.draw_in(
        &title,
        Rect::new(r.x + 16.0, y, r.w - 32.0, 22.0),
        &title_st,
        Align::Left,
    );
    y += 26.0;
    ui.fonts.draw_lines(
        &lines,
        Rect::new(
            r.x + 16.0,
            y,
            r.w - 32.0,
            lines.len() as f32 * body_st.line_h(),
        ),
        &body_st,
        Align::Left,
        None,
        usize::MAX,
    );
    // buttons
    let by = r.bottom() - 14.0 - 28.0;
    // the night coach marks follow the day cards, so there is always a "next"
    let next = Button::new(loc.tr("ui.tutorial.next"))
        .primary()
        .trailing(Icon::ChevronRight);
    let nw = ui.button_width(&next);
    let nr = Rect::new(r.right() - 16.0 - nw, by, nw, 28.0);
    let advance = ui.button(id.with("next"), nr, &next).clicked;
    let skip = Button::new(loc.tr("ui.tutorial.skip")).ghost();
    let sw = ui.button_width(&skip).min(nr.x - r.x - 24.0);
    let sr = Rect::new(r.x + 10.0, by, sw, 28.0);
    if ui.button(id.with("skip"), sr, &skip).clicked {
        draw::pop_alpha();
        g.finish_tutorial(cx);
        return;
    }
    draw::pop_alpha();
    if advance {
        g.tutorial_advance();
    }
}

/// Where the ray from `p` (inside `r`) along `dir` leaves `r`.
fn edge_point(r: Rect, p: Vec2, dir: Vec2) -> Vec2 {
    let tx = if dir.x > 1e-4 {
        (r.right() - p.x) / dir.x
    } else if dir.x < -1e-4 {
        (r.x - p.x) / dir.x
    } else {
        f32::MAX
    };
    let ty = if dir.y > 1e-4 {
        (r.bottom() - p.y) / dir.y
    } else if dir.y < -1e-4 {
        (r.y - p.y) / dir.y
    } else {
        f32::MAX
    };
    p + dir * tx.min(ty)
}

/// Sections of the "How to play" page: (icon, i18n id).
const HELP: [(Icon, &str); 10] = [
    (Icon::Sun, "rhythm"),
    (Icon::Coffee, "hands"),
    (Icon::Cup, "brewing"),
    (Icon::Crew, "crew"),
    (Icon::Phone, "app"),
    (Icon::Priorities, "duties"),
    (Icon::Build, "build"),
    (Icon::Moon, "night"),
    (Icon::Sparkle, "events"),
    (Icon::Money, "money"),
];

/// "How to play" — a compact reference (F1, the `?` in the dock, the pause menu).
pub fn help(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let loc: Locale = cx.loc;
    let t = ui.theme.clone();
    let s = ui.screen();
    // by day the planning panel stays visible on the right
    let day = g.world.phase() == Phase::Day && g.summary.is_none();
    let room = if day { s.w - 24.0 - 344.0 } else { s.w - 24.0 };
    let w = room.clamp(320.0, 760.0);
    let h = (s.h - 88.0 - 60.0).clamp(300.0, 560.0);
    let x = if day { 12.0 } else { ((s.w - w) / 2.0).round() };
    let pr = Rect::new(x, 88.0, w, h);
    g.anchors.push(("panel", pr));
    ui.glass(pr, &Glass::panel().solid());
    let (body, close) = ui.panel_header(pr, loc.tr("ui.help.title"), Some("F1"), None);
    if close {
        g.panel = None;
    }
    let cols = if body.w > 560.0 { 2 } else { 1 };
    let gap = 14.0;
    let cw = (body.w - 32.0 - gap * (cols as f32 - 1.0)) / cols as f32;
    let text_st = t.small().size(12.5).line_height(1.45).color(t.text_dim);
    let head_st = t.strong().size(13.5);
    let items: Vec<(Icon, String, Vec<String>)> = HELP
        .iter()
        .map(|(ic, id)| {
            let text = loc.tr_owned(&format!("ui.help.{id}.body"));
            (
                *ic,
                loc.tr_owned(&format!("ui.help.{id}.title")),
                ui.fonts.wrap(&text, cw - 28.0, &text_st),
            )
        })
        .collect();
    let item_h =
        |it: &(Icon, String, Vec<String>)| 22.0 + it.2.len() as f32 * text_st.line_h() + 12.0;
    let rows = items.len().div_ceil(cols);
    let content_h: f32 = (0..rows)
        .map(|row| {
            (0..cols)
                .filter_map(|c| items.get(row * cols + c))
                .map(item_h)
                .fold(0.0, f32::max)
        })
        .sum::<f32>()
        + 8.0;
    let view = Rect::new(body.x + 16.0, body.y + 8.0, body.w - 32.0, body.h - 16.0);
    let sc = ui.begin_scroll("help_scroll", view, content_h);
    let mut y = sc.top();
    for row in 0..rows {
        let mut row_h: f32 = 0.0;
        for c in 0..cols {
            let Some(it) = items.get(row * cols + c) else {
                continue;
            };
            let x = view.x + c as f32 * (cw + gap);
            let ih = item_h(it);
            row_h = row_h.max(ih);
            if sc.visible(y, ih) {
                draw_icon(it.0, vec2(x + 9.0, y + 9.0), 14.0, t.indigo_300);
                ui.fonts.draw_in(
                    &it.1,
                    Rect::new(x + 26.0, y, cw - 26.0, 18.0),
                    &head_st,
                    Align::Left,
                );
                ui.fonts.draw_lines(
                    &it.2,
                    Rect::new(x + 26.0, y + 22.0, cw - 28.0, 0.0),
                    &text_st,
                    Align::Left,
                    None,
                    usize::MAX,
                );
            }
        }
        y += row_h;
    }
    ui.end_scroll(sc);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cards_stay_on_screen_and_off_the_avoided_panels() {
        let screen = Rect::new(0.0, 0.0, 1280.0, 720.0);
        let planning = Rect::new(948.0, 88.0, 320.0, 470.0);
        for target in [
            Rect::new(12.0, 672.0, 110.0, 36.0),  // dock button
            Rect::new(12.0, 12.0, 340.0, 64.0),   // clock card
            Rect::new(964.0, 506.0, 288.0, 36.0), // start the night
            Rect::new(262.0, 92.0, 756.0, 560.0), // the tablet
        ] {
            let (r, _) = place(screen, target, 312.0, 170.0, &[planning]);
            assert!(r.x >= 8.0 && r.y >= 8.0, "{r:?}");
            assert!(r.right() <= 1272.0 && r.bottom() <= 712.0, "{r:?}");
        }
        let (r, _) = place(
            screen,
            Rect::new(12.0, 672.0, 110.0, 36.0),
            312.0,
            170.0,
            &[planning],
        );
        assert!(!r.overlaps(&planning));
    }

    #[test]
    fn the_night_coach_takes_over_after_the_day_cards() {
        assert!(STEPS.iter().all(|s| !s.night()));
        let p = TutorialProgress {
            active: true,
            step: FIRST_NIGHT as u32,
            ..Default::default()
        };
        assert_eq!(current(&p), None);
    }
}
