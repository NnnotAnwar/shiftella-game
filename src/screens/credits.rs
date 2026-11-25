//! The calm credits scroll after an ending, over the live street backdrop.

use macroquad::prelude::*;

use crate::screens::menu::{Backdrop, draw_wordmark, wordmark_width};
use crate::settings::Locale;
use crate::ui::{Align, Glass, Icon, RectExt, TextStyle, Ui, Weight, draw_icon};

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
        }
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
            "Adamov · Beroun · Cheb · Děčín · Ejpovice · Frýdlant · Mlhov nad Šeřavou".into(),
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
        self.backdrop.draw();
        ui.set_camera();
        let s = ui.screen();
        let theme = ui.theme.clone();
        draw_rectangle(0.0, 0.0, s.w, s.h, Color::new(0.04, 0.03, 0.09, 0.5));
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
                        draw_wordmark(ui, cx - w / 2.0, y + 40.0, size, self.t);
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
