//! The calm credits scroll after an ending, over the live street backdrop.

use macroquad::prelude::*;

use crate::i18n::{format_int, format_money};
use crate::screens::menu::{Backdrop, draw_wordmark, wordmark_width};
use crate::settings::Locale;
use crate::sim::ChallengeReport;
use crate::ui::{Align, Button, Glass, Icon, RectExt, TextStyle, Ui, Weight, draw_icon};

/// Seconds the scroll takes before it settles on the last card.
const SCROLL_SECONDS: f32 = 46.0;

#[derive(Debug, Clone)]
enum Line {
    Wordmark,
    Title(String),
    Heading(String),
    Text(String),
    Small(String),
    Gap(f32),
}

pub struct Credits {
    pub backdrop: Backdrop,
    pub t: f32,
    pub ending: Option<String>,
    pub variant: Option<String>,
    /// The optional "Answer" epilogue was reached.
    pub epilogue: bool,
    /// A sandbox «Challenge» run ended: its final page (the report, the seed's best seasons
    /// before this run) instead of the scroll.
    pub challenge: Option<(ChallengeReport, Option<u32>)>,
}

impl Credits {
    pub fn new(
        ui: &Ui,
        ending: Option<String>,
        variant: Option<String>,
        epilogue: bool,
    ) -> Credits {
        Credits {
            backdrop: Backdrop::new(Some(ui.fonts.face(Weight::Bold).clone())),
            t: 0.0,
            ending,
            variant,
            epilogue,
            challenge: None,
        }
    }

    /// The final page of a Challenge run: `best` = the seed's record before it.
    pub fn with_challenge(mut self, report: ChallengeReport, best: Option<u32>) -> Credits {
        self.challenge = Some((report, best));
        self
    }

    /// The Challenge's final page: why it closed, the run in numbers, the seed's record.
    fn challenge_frame(&mut self, ui: &mut Ui, loc: &Locale) -> bool {
        let Some((r, best)) = self.challenge.clone() else {
            return true;
        };
        let s = ui.screen();
        let t = ui.theme.clone();
        let w = (s.w - 32.0).min(460.0);
        let h = 404.0;
        let p = Rect::new((s.w - w) / 2.0, ((s.h - h) / 2.0).max(12.0), w, h);
        ui.glass(p, &Glass::panel().solid().radius(14.0));
        let c = p.inset(22.0);
        let cx = c.x + c.w / 2.0;
        let mut y = c.y;
        ui.fonts.draw_centered(
            &loc.tr("challenge.final.kicker").to_uppercase(),
            cx,
            y + 12.0,
            &t.micro().color(t.indigo_300),
        );
        y += 22.0;
        ui.fonts.draw_centered(
            loc.tr("ui.credits.closed_title"),
            cx,
            y + 26.0,
            &t.display().size(26.0),
        );
        y += 40.0;
        let reason = r
            .reason
            .map(|x| format!("challenge.final.reason.{}", x.id()))
            .unwrap_or_else(|| "ui.credits.closed_text".into());
        y += ui.fonts.draw_wrapped(
            loc.tr(&reason),
            Rect::new(c.x, y, c.w, 40.0),
            &t.small().color(t.text),
            Align::Center,
            Some(2),
        );
        y += 12.0;
        crate::ui::draw::hline(c.x, c.right(), y, t.divider);
        y += 10.0;
        let rows: [(Icon, String); 5] = [
            (
                Icon::Calendar,
                loc.trf(
                    "challenge.final.seasons",
                    &[("n", &r.seasons_survived.to_string())],
                ),
            ),
            (
                Icon::Moon,
                loc.trf("challenge.final.nights", &[("n", &r.nights.to_string())]),
            ),
            (
                Icon::Star,
                loc.trf(
                    "challenge.final.best_night",
                    &[("money", &format_money(loc.lang, r.best_night))],
                ),
            ),
            (
                Icon::Heart,
                loc.trf(
                    "challenge.final.guests",
                    &[("n", &format_int(loc.lang, r.guests as i64))],
                ),
            ),
            (
                Icon::Sparkle,
                loc.trf(
                    "challenge.final.earned",
                    &[("money", &format_money(loc.lang, r.earned))],
                ),
            ),
        ];
        for (icon, txt) in rows {
            draw_icon(icon, vec2(c.x + 12.0, y + 11.0), 14.0, t.amber);
            ui.fonts.draw_in(
                &txt,
                Rect::new(c.x + 30.0, y, c.w - 30.0, 22.0),
                &TextStyle::new(15.0, Weight::Medium, t.text),
                Align::Left,
            );
            y += 26.0;
        }
        y += 6.0;
        crate::ui::draw::hline(c.x, c.right(), y, t.divider);
        y += 10.0;
        let record = best.map_or(r.seasons_survived, |b| b.max(r.seasons_survived));
        if best.is_none_or(|b| r.seasons_survived > b) {
            ui.fonts.draw_centered(
                loc.tr("challenge.final.new_record"),
                cx,
                y + 14.0,
                &t.strong().color(t.amber),
            );
            y += 22.0;
        }
        ui.fonts.draw_centered(
            &loc.trf("challenge.final.record", &[("n", &record.to_string())]),
            cx,
            y + 14.0,
            &t.small(),
        );
        let b = Button::new(loc.tr("challenge.final.back"))
            .primary()
            .icon(Icon::Home)
            .shortcut("Enter");
        let bw = ui.button_width(&b).max(180.0);
        let clicked = ui
            .button(
                "challenge_done",
                Rect::new(cx - bw / 2.0, c.bottom() - 34.0, bw, 34.0),
                &b,
            )
            .clicked;
        clicked || ui.esc() || ui.enter()
    }

    fn lines(&self, loc: &Locale) -> Vec<Line> {
        let mut v = vec![Line::Gap(40.0)];
        match self.ending.as_deref() {
            Some("closed") => {
                v.push(Line::Title(loc.tr_owned("ui.credits.closed_title")));
                v.push(Line::Text(loc.tr_owned("ui.credits.closed_text")));
            }
            Some(id) => {
                v.push(Line::Small(loc.tr_owned("ui.ending.title")));
                v.push(Line::Title(loc.tr_owned(&format!("ending.{id}.title"))));
                v.push(Line::Text(loc.tr_owned(&format!("ending.{id}.summary"))));
                if let Some(var) = &self.variant {
                    let k = format!("ending.{id}.variant.{var}");
                    if loc.has(&k) {
                        v.push(Line::Small(loc.tr_owned(&k)));
                    }
                }
                if self.epilogue {
                    let title = loc.tr_owned("ending.extra.answer.title");
                    v.push(Line::Small(
                        loc.trf("ui.ending.extra_unlocked", &[("title", &title)]),
                    ));
                }
            }
            None => {}
        }
        v.push(Line::Gap(120.0));
        v.push(Line::Wordmark);
        v.push(Line::Gap(30.0));
        v.push(Line::Heading(loc.tr_owned("ui.credits.made_by_title")));
        v.push(Line::Text(loc.tr_owned("ui.credits.made_by")));
        v.push(Line::Gap(36.0));
        v.push(Line::Heading(loc.tr_owned("ui.credits.music_title")));
        v.push(Line::Text(loc.tr_owned("ui.credits.music")));
        v.push(Line::Gap(36.0));
        v.push(Line::Heading(loc.tr_owned("ui.credits.font_title")));
        v.push(Line::Text(loc.tr_owned("ui.credits.font")));
        v.push(Line::Gap(36.0));
        v.push(Line::Heading(loc.tr_owned("ui.credits.places_title")));
        v.push(Line::Text(
            "Adamov · Beroun · Cheb · Děčín · Ejpovice · Frýdlant · Praha-Holešovice".into(),
        ));
        v.push(Line::Gap(60.0));
        v.push(Line::Text(loc.tr_owned("ui.ending.credits_thanks")));
        v.push(Line::Gap(60.0));
        v.push(Line::Title(loc.tr_owned("ui.credits.thanks_for_playing")));
        v
    }

    fn line_h(l: &Line) -> f32 {
        match l {
            Line::Wordmark => 70.0,
            Line::Title(_) => 52.0,
            Line::Heading(_) => 26.0,
            Line::Text(_) => 48.0,
            Line::Small(_) => 28.0,
            Line::Gap(h) => *h,
        }
    }

    /// Draws the credits; returns true when the player wants to leave (or it ended).
    pub fn frame(&mut self, ui: &mut Ui, loc: &Locale, dt: f32) -> bool {
        self.t += dt;
        self.backdrop.update(dt);
        self.backdrop.set_lang(loc.lang);
        self.backdrop.draw();
        ui.set_camera();
        let s = ui.screen();
        let theme = ui.theme.clone();
        draw_rectangle(0.0, 0.0, s.w, s.h, Color::new(0.04, 0.03, 0.09, 0.5));
        if self.challenge.is_some() {
            return self.challenge_frame(ui, loc);
        }
        let lines = self.lines(loc);
        let total: f32 = lines.iter().map(Self::line_h).sum();
        let p = (self.t / SCROLL_SECONDS).min(1.0);
        let start = s.h * 0.9;
        let end = s.h * 0.62 - total;
        let mut y = start + p * (end - start);
        let cx = s.w / 2.0;
        let maxw = (s.w * 0.6).min(620.0);
        for l in &lines {
            let h = Self::line_h(l);
            if y > -80.0 && y < s.h + 80.0 {
                match l {
                    Line::Wordmark => {
                        let size = 40.0;
                        let w = wordmark_width(ui, size);
                        let mark = 44.0;
                        let total = mark + 12.0 + w;
                        let x0 = cx - total / 2.0;
                        crate::render::draw_brand_mark(vec2(x0 + mark / 2.0, y + 26.0), mark, 0.8);
                        draw_wordmark(ui, x0 + mark + 12.0, y + 40.0, size, self.t);
                    }
                    Line::Title(txt) => {
                        ui.fonts
                            .draw_centered(txt, cx, y + 36.0, &theme.display().size(30.0));
                    }
                    Line::Heading(txt) => {
                        ui.fonts.draw_centered(
                            txt,
                            cx,
                            y + 16.0,
                            &theme.micro().color(theme.indigo_300),
                        );
                    }
                    Line::Text(txt) => {
                        ui.fonts.draw_wrapped(
                            txt,
                            Rect::new(cx - maxw / 2.0, y, maxw, h),
                            &TextStyle::new(15.0, Weight::Medium, theme.text),
                            Align::Center,
                            Some(2),
                        );
                    }
                    Line::Small(txt) => {
                        ui.fonts.draw_centered(txt, cx, y + 18.0, &theme.small());
                    }
                    Line::Gap(_) => {}
                }
            }
            y += h;
        }
        // hint
        let hint = loc.tr("ui.credits.skip");
        let hr = Rect::new(s.w - 230.0, s.h - 44.0, 218.0, 32.0);
        ui.glass(hr, &Glass::hud().radius(8.0));
        draw_icon(
            Icon::Home,
            vec2(hr.x + 18.0, hr.center_y()),
            14.0,
            theme.indigo_300,
        );
        ui.fonts.draw_in(
            hint,
            Rect::new(hr.x + 32.0, hr.y, hr.w - 40.0, hr.h),
            &theme.small(),
            Align::Left,
        );
        let clicked = ui.interact(ui.id("credits_done"), hr).clicked;
        clicked
            || ui.esc()
            || ui.enter()
            || self.t > SCROLL_SECONDS + 12.0
            || (self.t > 1.0 && ui.mouse_pressed() && !ui.wants_mouse())
    }
}
