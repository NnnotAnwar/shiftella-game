//! Cutscene UI: dialogue box with typewriter reveal, narration, choices,
//! letterbox bars and fades.
//!
//! ```ignore
//! let mut dlg = DialogueBox::new();          // keep it in the cutscene state
//! // every frame:
//! ui.letterbox(bars_t);
//! let line = DialogueLine::new("Lída", "Vlak Rn 333 dnes nepřijede…").portrait(true);
//! let r = dlg.dialogue_box(&mut ui, &line);
//! if let Some(p) = r.portrait { render::draw_portrait(p, Character::Lida) }
//! if r.advance { script.next() }
//! ```

use std::collections::VecDeque;
use std::hash::Hash;

use macroquad::prelude::*;

use super::context::{Ui, UiSound, ease, ease_out};
use super::draw::{
    self, Paint, fill_convex, fill_rrect, hline, shadow_rrect, stroke_rrect, with_a,
};
use super::glass::{Glass, draw_glass};
use super::icons::Icon;
use super::layout::RectExt;
use super::text::{Align, TextStyle, Weight, reveal_len};
use super::widgets::{Button, paint_surface};

/// Localised labels for the dialogue chrome.
#[derive(Clone, Copy, Debug)]
pub struct DialogueLabels<'a> {
    pub skip: &'a str,
    pub log: &'a str,
    /// Title of the history panel.
    pub history: &'a str,
}

impl Default for DialogueLabels<'_> {
    fn default() -> Self {
        DialogueLabels {
            skip: "Skip",
            log: "Log",
            history: "History",
        }
    }
}

/// One line of dialogue.
#[derive(Clone, Copy, Debug)]
pub struct DialogueLine<'a> {
    pub speaker: Option<&'a str>,
    pub text: &'a str,
    /// Leave a square slot on the left for `draw_portrait`.
    pub portrait: bool,
    /// Chip colour of the speaker (default indigo).
    pub speaker_color: Option<Color>,
    /// Choices follow: don't advance on click, don't auto-advance while skipping.
    pub awaiting_choice: bool,
    pub labels: DialogueLabels<'a>,
}

impl<'a> DialogueLine<'a> {
    pub fn new(speaker: &'a str, text: &'a str) -> Self {
        DialogueLine {
            speaker: if speaker.is_empty() {
                None
            } else {
                Some(speaker)
            },
            text,
            portrait: false,
            speaker_color: None,
            awaiting_choice: false,
            labels: DialogueLabels::default(),
        }
    }
    pub fn portrait(mut self, p: bool) -> Self {
        self.portrait = p;
        self
    }
    pub fn color(mut self, c: Color) -> Self {
        self.speaker_color = Some(c);
        self
    }
    pub fn awaiting_choice(mut self, a: bool) -> Self {
        self.awaiting_choice = a;
        self
    }
    pub fn labels(mut self, l: DialogueLabels<'a>) -> Self {
        self.labels = l;
        self
    }
}

/// What happened this frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct DialogueResponse {
    /// The player wants the next line.
    pub advance: bool,
    /// Where to draw the speaker portrait (after this call, so it sits on top).
    pub portrait: Option<Rect>,
    /// The whole line is visible.
    pub revealed: bool,
    /// The dialogue box rect (use as `above` for [`Ui::choice_list`]).
    pub rect: Rect,
    pub skipping: bool,
}

/// A past line in the history log.
#[derive(Clone, Debug, PartialEq)]
pub struct HistoryLine {
    pub speaker: Option<String>,
    pub text: String,
}

/// Stateful dialogue presenter (typewriter, skip, history). Keep one per cutscene.
pub struct DialogueBox {
    /// Typewriter speed (chars/s). Settings: 30 (slow) … 90 (fast); 0 = instant.
    pub chars_per_sec: f32,
    pub history: VecDeque<HistoryLine>,
    pub history_max: usize,
    pub log_open: bool,
    /// Skip toggled by the button (holding Ctrl also skips).
    pub skip_on: bool,
    /// Box height cap as a fraction of the screen.
    pub max_height_frac: f32,
    /// Space reserved at the bottom (e.g. letterbox bar height).
    pub bottom_inset: f32,
    current: Option<HistoryLine>,
    log_seen: Option<usize>,
    shown: f32,
    pause: f32,
    since_full: f32,
}

impl Default for DialogueBox {
    fn default() -> Self {
        Self::new()
    }
}

impl DialogueBox {
    pub fn new() -> Self {
        DialogueBox {
            chars_per_sec: 55.0,
            history: VecDeque::new(),
            history_max: 60,
            log_open: false,
            skip_on: false,
            max_height_frac: 0.3,
            bottom_inset: 0.0,
            current: None,
            log_seen: None,
            shown: 0.0,
            pause: 0.0,
            since_full: 0.0,
        }
    }

    /// Show the current line completely (e.g. for screenshots or "instant text").
    pub fn reveal_all(&mut self) {
        self.shown = f32::MAX / 4.0;
    }

    fn track(&mut self, speaker: Option<&str>, text: &str) {
        let same = self
            .current
            .as_ref()
            .is_some_and(|c| c.text == text && c.speaker.as_deref() == speaker);
        if same {
            return;
        }
        if let Some(prev) = self.current.take() {
            self.history.push_back(prev);
            while self.history.len() > self.history_max {
                self.history.pop_front();
            }
        }
        self.current = Some(HistoryLine {
            speaker: speaker.map(str::to_string),
            text: text.to_string(),
        });
        self.shown = 0.0;
        self.pause = 0.0;
        self.since_full = 0.0;
    }

    /// Advance the typewriter; returns visible char count.
    fn tick(&mut self, ui: &Ui, total: usize, skipping: bool) -> usize {
        let dt = ui.dt();
        if self.chars_per_sec <= 0.0 || skipping {
            self.shown = total as f32;
        } else if self.pause > 0.0 {
            self.pause -= dt;
        } else {
            let before = self.shown as usize;
            self.shown += self.chars_per_sec * dt;
            let after = (self.shown as usize).min(total);
            // brief pauses after punctuation for a natural rhythm
            if let Some(cur) = &self.current
                && after > before
                && let Some(c) = cur.text.chars().nth(after.saturating_sub(1))
            {
                self.pause = match c {
                    '.' | '!' | '?' | '…' => 0.22,
                    ',' | ';' | ':' | '—' => 0.08,
                    _ => 0.0,
                };
            }
        }
        let n = (self.shown as usize).min(total);
        if n >= total {
            self.since_full += dt;
        }
        n
    }

    fn wants_skip(&self, ui: &Ui) -> bool {
        self.skip_on || ui.ctrl()
    }

    /// Bottom dialogue panel. Advance with click / Space / Enter; Ctrl or Skip fast-forwards.
    pub fn dialogue_box(&mut self, ui: &mut Ui, line: &DialogueLine) -> DialogueResponse {
        self.track(line.speaker, line.text);
        let t = ui.theme.clone();
        let screen = ui.screen();
        let id = ui.id("dialogue_box");
        // click-catcher over the whole screen: cutscenes own the mouse
        ui.region(screen);
        let catcher = ui.interact(id.with("catch"), screen);

        let w = (screen.w - 48.0).min(900.0);
        let h = (screen.h * self.max_height_frac).clamp(118.0, 160.0);
        let r = Rect::new(
            (screen.w - w) / 2.0,
            screen.h - 20.0 - self.bottom_inset - h,
            w,
            h,
        );
        let mut out = DialogueResponse {
            rect: r,
            ..Default::default()
        };

        // history panel above the box
        if self.log_open {
            self.draw_log(ui, r, line.labels.history);
        }

        ui.glass(r, &Glass::panel().radius(14.0));
        // portrait slot
        let mut text_x = r.x + 22.0;
        if line.portrait {
            let s = h - 24.0;
            let pr = Rect::new(r.x + 12.0, r.y + 12.0, s, s);
            fill_rrect(
                pr,
                10.0,
                Paint::vertical(
                    pr,
                    with_a(t.indigo_600, 0.28),
                    Color::new(0.03, 0.02, 0.09, 0.6),
                ),
            );
            stroke_rrect(pr, 10.0, 1.0, Color::new(1.0, 1.0, 1.0, 0.10));
            out.portrait = Some(pr);
            text_x = pr.right() + 18.0;
        }
        // speaker chip on the top edge
        if let Some(name) = line.speaker {
            let st = TextStyle::new(13.0, Weight::Bold, WHITE).tracking(0.2);
            let cw = ui.fonts.measure(name, &st) + 24.0;
            let chip = Rect::new(text_x - 4.0, r.y - 13.0, cw, 26.0);
            let c = line.speaker_color.unwrap_or(t.indigo);
            shadow_rrect(
                chip,
                13.0,
                draw::Shadow::new(0.0, 4.0, 14.0, 0.0, with_a(c, 0.5)),
            );
            fill_rrect(
                chip,
                13.0,
                Paint::diagonal(
                    chip,
                    draw::mix(c, WHITE, 0.08),
                    draw::mix(c, t.violet, 0.55),
                ),
            );
            stroke_rrect(
                chip,
                13.0,
                1.0,
                Paint::vertical(
                    chip,
                    Color::new(1.0, 1.0, 1.0, 0.4),
                    Color::new(1.0, 1.0, 1.0, 0.05),
                ),
            );
            ui.fonts.draw_in(name, chip, &st, Align::Center);
        }
        // chrome buttons (top-right)
        let skipping_now = self.wants_skip(ui);
        let by = r.bottom() - 10.0 - 24.0;
        let mut fx = r.right() - 40.0;
        let skip_b = Button::new(line.labels.skip)
            .ghost()
            .trailing(Icon::Skip)
            .selected(self.skip_on);
        let sw = ui.button_width(&skip_b);
        let skip_r = Rect::new(fx - sw, by, sw, 24.0);
        if ui.button(id.with("skip"), skip_r, &skip_b).clicked {
            self.skip_on = !self.skip_on;
        }
        fx = skip_r.x - 4.0;
        let log_b = Button::new(line.labels.log)
            .ghost()
            .icon(Icon::Log)
            .selected(self.log_open);
        let lw = ui.button_width(&log_b);
        let log_r = Rect::new(fx - lw, by, lw, 24.0);
        if ui.button(id.with("log"), log_r, &log_b).clicked {
            self.log_open = !self.log_open;
            self.log_seen = None;
        }

        let box_click = ui.interact(id.with("box"), r);

        // text
        let st = TextStyle::new(15.0, Weight::Regular, t.text).line_height(1.5);
        let text_r = Rect::new(
            text_x,
            r.y + 24.0,
            r.right() - 28.0 - text_x,
            h - 24.0 - 40.0,
        );
        let lines = ui.fonts.wrap(line.text, text_r.w, &st);
        let total = reveal_len(&lines);
        let visible = self.tick(ui, total, skipping_now);
        let max_lines = ((text_r.h / st.line_h()).floor() as usize).max(1);
        ui.fonts
            .draw_lines(&lines, text_r, &st, Align::Left, Some(max_lines), visible);
        out.revealed = visible >= total;
        out.skipping = skipping_now;

        // continue indicator
        if out.revealed && !line.awaiting_choice {
            let blink = 0.55 + 0.45 * ((ui.time() * 3.2).sin() as f32);
            let bob = ((ui.time() * 3.2).sin() as f32) * 1.5;
            let c = vec2(r.right() - 24.0 + bob, by + 12.0);
            draw::push_alpha(blink);
            fill_convex(
                &[
                    c + vec2(-4.0, -5.5),
                    c + vec2(5.0, 0.0),
                    c + vec2(-4.0, 5.5),
                ],
                Paint::Solid(t.indigo_300),
            );
            draw::pop_alpha();
        }

        // input
        let key_adv = ui.take_key(KeyCode::Space) || ui.enter();
        let click_adv = catcher.clicked || box_click.clicked;
        if key_adv || click_adv {
            if !out.revealed {
                self.reveal_all();
            } else if !line.awaiting_choice {
                out.advance = true;
            }
        }
        if skipping_now && out.revealed && !line.awaiting_choice && self.since_full > 0.09 {
            out.advance = true;
        }
        if out.advance {
            ui.sound(UiSound::Advance);
        }
        out
    }

    fn draw_log(&mut self, ui: &mut Ui, above: Rect, title: &str) {
        let t = ui.theme.clone();
        let h = 230.0_f32.min(above.y - 40.0).max(80.0);
        let r = Rect::new(above.x + above.w - 420.0, above.y - 18.0 - h, 420.0, h);
        ui.glass(r, &Glass::panel().solid());
        let st_title = t.micro();
        ui.fonts
            .draw(&title.to_uppercase(), r.x + 16.0, r.y + 24.0, &st_title);
        hline(r.x + 1.0, r.right() - 1.0, r.y + 36.0, t.divider);
        let view = Rect::new(r.x + 8.0, r.y + 40.0, r.w - 16.0, r.h - 46.0);
        let body = t.small().color(t.text_dim);
        let name = t.strong().size(12.0).color(t.indigo_300);
        let tw = view.w - 16.0;
        let items: Vec<(Option<String>, Vec<String>)> = self
            .history
            .iter()
            .map(|l| (l.speaker.clone(), ui.fonts.wrap(&l.text, tw, &body)))
            .collect();
        let item_h = |it: &(Option<String>, Vec<String>)| {
            (if it.0.is_some() { 17.0 } else { 0.0 }) + it.1.len() as f32 * body.line_h() + 8.0
        };
        let content_h: f32 = items.iter().map(item_h).sum::<f32>() + 4.0;
        let key = "dialogue_log";
        // jump to the newest line when the log opens or a line is added; otherwise free scroll
        if self.log_seen != Some(self.history.len()) {
            self.log_seen = Some(self.history.len());
            ui.scroll_to_end(key, content_h, view.h);
        }
        let s = ui.begin_scroll(key, view, content_h);
        let mut y = s.top() + 4.0;
        for it in &items {
            let ih = item_h(it);
            if s.visible(y, ih) {
                let mut yy = y;
                if let Some(sp) = &it.0 {
                    ui.fonts.draw(sp, view.x + 8.0, yy + 12.0, &name);
                    yy += 17.0;
                }
                ui.fonts.draw_lines(
                    &it.1,
                    Rect::new(view.x + 8.0, yy, tw, 0.0),
                    &body,
                    Align::Left,
                    None,
                    usize::MAX,
                );
            }
            y += ih;
        }
        ui.end_scroll(s);
    }

    /// Narration: a solid glass card low on the screen (where the dialogue box sits), so the
    /// scene stays visible and the text reads on any background. Same advance / skip rules
    /// as the box.
    pub fn narration_box(&mut self, ui: &mut Ui, text: &str) -> DialogueResponse {
        self.track(None, text);
        let t = ui.theme.clone();
        let screen = ui.screen();
        let id = ui.id("narration_box");
        ui.region(screen);
        let catcher = ui.interact(id, screen);
        let st = TextStyle::new(16.0, Weight::Medium, t.text)
            .line_height(1.55)
            .tracking(0.1);
        let pad_x = 30.0;
        let pad_y = 20.0;
        let max_w = (screen.w - 48.0).min(760.0);
        let lines = ui.fonts.wrap(text, max_w - 2.0 * pad_x, &st);
        // shrink the card to the longest line (short lines don't float in a wide box)
        let widest = lines
            .iter()
            .map(|l| ui.fonts.measure(l, &st))
            .fold(0.0, f32::max);
        let w = (widest + 2.0 * pad_x).clamp(360.0_f32.min(max_w), max_w);
        let total = reveal_len(&lines);
        let skipping = self.wants_skip(ui);
        let visible = self.tick(ui, total, skipping);
        let text_h = lines.len().max(1) as f32 * st.line_h();
        let h = text_h + 2.0 * pad_y;
        let r = Rect::new(
            ((screen.w - w) / 2.0).round(),
            (screen.h - 20.0 - self.bottom_inset - h).round(),
            w.round(),
            h.round(),
        );
        // a soft dark wash behind the card keeps it legible over bright lamps
        let wash = Rect::new(0.0, r.y - 80.0, screen.w, screen.h - r.y + 80.0);
        fill_rrect(
            wash,
            0.0,
            Paint::vertical(
                wash,
                Color::new(0.027, 0.024, 0.07, 0.0),
                Color::new(0.027, 0.024, 0.07, 0.55),
            ),
        );
        ui.glass(r, &Glass::panel().solid().radius(14.0));
        // a thin violet accent marks narration (vs. a spoken line)
        let accent = Rect::new(r.x + 12.0, r.y + 16.0, 3.0, r.h - 32.0);
        fill_rrect(
            accent,
            1.5,
            Paint::vertical(accent, with_a(t.indigo_300, 0.9), with_a(t.violet, 0.5)),
        );
        let text_r = Rect::new(r.x + pad_x, r.y + pad_y, r.w - 2.0 * pad_x, text_h);
        ui.fonts
            .draw_lines(&lines, text_r, &st, Align::Left, None, visible);
        let revealed = visible >= total;
        if revealed {
            let blink = 0.55 + 0.45 * ((ui.time() * 3.2).sin() as f32);
            let bob = ((ui.time() * 3.2).sin() as f32) * 1.5;
            draw::push_alpha(blink);
            let c = vec2(r.right() - 16.0 + bob, r.bottom() - 16.0);
            fill_convex(
                &[
                    c + vec2(-4.0, -5.5),
                    c + vec2(5.0, 0.0),
                    c + vec2(-4.0, 5.5),
                ],
                Paint::Solid(t.indigo_300),
            );
            draw::pop_alpha();
        }
        let mut out = DialogueResponse {
            rect: r,
            revealed,
            skipping,
            ..Default::default()
        };
        if ui.take_key(KeyCode::Space) || ui.enter() || catcher.clicked {
            if !revealed {
                self.reveal_all();
            } else {
                out.advance = true;
            }
        }
        if skipping && revealed && self.since_full > 0.09 {
            out.advance = true;
        }
        if out.advance {
            ui.sound(UiSound::Advance);
        }
        out
    }
}

impl Ui {
    /// 2–4 stacked choices above `above` (the dialogue box). Keys 1–4 pick directly,
    /// ↑/↓ move the highlight, Enter picks. Returns the picked index.
    pub fn choice_list(&mut self, key: impl Hash, above: Rect, choices: &[&str]) -> Option<usize> {
        let id = self.id(key);
        let t = self.theme.clone();
        let n = choices.len().min(9);
        if n == 0 {
            return None;
        }
        let st = t.body_medium().size(14.0);
        let widest = choices
            .iter()
            .map(|c| self.fonts.measure(c, &st))
            .fold(0.0, f32::max);
        let w = (widest + 82.0)
            .clamp(300.0, 560.0)
            .min(self.screen().w - 32.0);
        let row_h = 36.0;
        let gap = 6.0;
        let total_h = n as f32 * row_h + (n - 1) as f32 * gap;
        let x = above.x + (above.w - w) / 2.0;
        let y0 = above.y - 30.0 - total_h;
        // appear animation
        let appear = {
            let dt = self.dt();
            let a = self.anim(id.with("appear"));
            a.v = (a.v + dt / 0.22).min(1.0);
            ease_out(a.v)
        };
        let mut hl = self.anim(id).t as usize;
        if hl >= n {
            hl = 0;
        }
        let mut picked = None;
        if self.key_repeat(KeyCode::Down) {
            hl = (hl + 1) % n;
            self.consume_key(KeyCode::Down);
        }
        if self.key_repeat(KeyCode::Up) {
            hl = (hl + n - 1) % n;
            self.consume_key(KeyCode::Up);
        }
        const NUMS: [KeyCode; 9] = [
            KeyCode::Key1,
            KeyCode::Key2,
            KeyCode::Key3,
            KeyCode::Key4,
            KeyCode::Key5,
            KeyCode::Key6,
            KeyCode::Key7,
            KeyCode::Key8,
            KeyCode::Key9,
        ];
        for (i, k) in NUMS.iter().enumerate().take(n) {
            if self.take_key(*k) {
                picked = Some(i);
            }
        }
        if self.enter() {
            picked = Some(hl);
        }
        for (i, choice) in choices.iter().enumerate().take(n) {
            let delay = i as f32 * 0.12;
            let k = ((appear - delay) / (1.0 - delay).max(0.01)).clamp(0.0, 1.0);
            let r = Rect::new(
                x,
                y0 + i as f32 * (row_h + gap) + (1.0 - k) * 10.0,
                w,
                row_h,
            );
            self.region(r);
            let resp = self.interact(id.with(i), r);
            if resp.hovered && self.mouse_delta().length_squared() > 0.0 {
                hl = i;
            }
            if resp.clicked {
                picked = Some(i);
            }
            let on = i == hl;
            let hk = self.ease_value(id.with(("hl", i)), if on { 1.0 } else { 0.0 }, 0.05);
            draw::push_alpha(k);
            draw_glass(&t, r, &Glass::hud().radius(10.0).solid());
            if hk > 0.01 {
                draw::push_alpha(hk);
                paint_surface(
                    &t,
                    r,
                    10.0,
                    super::widgets::ButtonKind::Secondary,
                    false,
                    1.0,
                    resp.press_t,
                );
                fill_rrect(
                    r,
                    10.0,
                    Paint::horizontal(r, with_a(t.indigo, 0.30), with_a(t.violet, 0.10)),
                );
                stroke_rrect(
                    r,
                    10.0,
                    1.0,
                    Paint::vertical(r, with_a(t.indigo_300, 0.7), with_a(t.indigo_400, 0.25)),
                );
                draw::pop_alpha();
            }
            let label = (i + 1).to_string();
            let kc = self.keycap(vec2(r.x + 12.0, r.center_y()), &label, on);
            let tr = Rect::new(kc.right() + 12.0, r.y, r.right() - kc.right() - 40.0, r.h);
            self.fonts.draw_in(
                choice,
                tr,
                &st.color(if on { WHITE } else { t.text }),
                Align::Left,
            );
            if hk > 0.01 {
                draw::push_alpha(hk);
                super::icons::draw_icon(
                    Icon::ChevronRight,
                    vec2(r.right() - 18.0 + (1.0 - hk) * -4.0, r.center_y()),
                    14.0,
                    t.indigo_300,
                );
                draw::pop_alpha();
            }
            draw::pop_alpha();
        }
        self.anim(id).t = hl as f32;
        if picked.is_some() {
            self.anim(id.with("appear")).v = 0.0;
            self.sound(UiSound::Choice);
        }
        picked
    }

    /// Cinematic bars. `progress` 0..1 (eased inside). Returns the free area between bars.
    pub fn letterbox(&mut self, progress: f32) -> Rect {
        let s = self.screen();
        let k = ease(progress);
        let bh = (s.h * 0.1).round() * k;
        if bh > 0.1 {
            let c = Color::from_hex(0x040309);
            fill_rrect(Rect::new(0.0, 0.0, s.w, bh), 0.0, c);
            fill_rrect(Rect::new(0.0, s.h - bh, s.w, bh), 0.0, c);
            let glow = Color::new(0.39, 0.4, 0.95, 0.10 * k);
            hline(0.0, s.w, bh, glow);
            hline(0.0, s.w, s.h - bh - draw::px(), glow);
        }
        Rect::new(0.0, bh, s.w, s.h - 2.0 * bh)
    }

    /// Full-screen fade to night ink (`alpha` 0..1).
    pub fn fade(&mut self, alpha: f32) {
        self.fade_to(alpha, Color::from_hex(0x07061A));
    }

    pub fn fade_to(&mut self, alpha: f32, color: Color) {
        if alpha > 0.001 {
            fill_rrect(
                self.screen().grow(2.0),
                0.0,
                with_a(color, alpha.clamp(0.0, 1.0)),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_tracks_previous_lines() {
        let mut d = DialogueBox::new();
        d.track(Some("Lída"), "Dobrý večer.");
        d.track(Some("Lída"), "Dobrý večer.");
        assert!(d.history.is_empty());
        d.track(None, "Venku prší.");
        assert_eq!(d.history.len(), 1);
        assert_eq!(d.history[0].speaker.as_deref(), Some("Lída"));
        d.history_max = 1;
        d.track(Some("Oksana"), "Їжак ґречно їсть");
        assert_eq!(d.history.len(), 1);
        assert_eq!(d.history[0].text, "Venku prší.");
    }
}
