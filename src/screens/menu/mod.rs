//! # Main menu, first-launch language picker, new story / sandbox setup, settings
//! (owner: cutscene agent)
//!
//! ```ignore
//! let mut menu = MainMenu::new(&content, Settings::load(), &ui);
//! loop {
//!     ui.begin_frame();
//!     menu.update(ui.dt());
//!     match menu.draw(&mut ui, &i18n, &my_host) {             // draws backdrop + UI
//!         Some(MenuAction::StartStory { difficulty }) => { /* World::new_story, play "opening" */ }
//!         Some(MenuAction::StartSandbox(setup)) => { /* World::new_sandbox(setup) */ }
//!         Some(MenuAction::Continue) => { /* load the save */ }
//!         Some(MenuAction::SettingsChanged) => { /* apply menu.settings volumes */ }
//!         Some(MenuAction::Quit) => break,
//!         None => {}
//!     }
//!     for a in menu.drain_audio_requests() { /* mus_menu_theme, rain, ui clicks */ }
//!     ui.end_frame();
//!     next_frame().await;
//! }
//! ```
//! The menu saves settings itself (language, volumes …) and applies the UI scale and
//! fullscreen. First launch (no language yet) opens the language picker.

pub mod backdrop;
mod title;

pub use backdrop::Backdrop;
pub use title::{draw_wordmark, wordmark_width};

use macroquad::prelude::*;

use crate::content::Content;
use crate::cutscene::AudioRequest;
use crate::i18n::{I18n, Lang, format_money};
use crate::render::draw_brand_mark;
use crate::settings::{Locale, PANEL_SIZE, Settings, settings_panel};
use crate::ui::draw::{Paint, fill_rrect, stroke_rrect, with_a};
use crate::ui::{
    Align, Button, Glass, Icon, IconButton, RectExt, TextStyle, Tone, Ui, Weight, draw_icon,
};

/// What the game should do now.
#[derive(Debug, Clone, PartialEq)]
pub enum MenuAction {
    /// Load the latest save.
    Continue,
    /// New story with this difficulty id (`cozy` / `classic`); then play the `opening` cutscene.
    StartStory {
        difficulty: String,
    },
    StartSandbox(SandboxSetup),
    /// Settings were changed and saved (re-apply volumes; language/scale are already live).
    SettingsChanged,
    Quit,
}

/// Sandbox setup chosen in the menu. (Plain data until the sim defines its own type —
/// `World::new_sandbox` can take this directly.)
#[derive(Debug, Clone, PartialEq)]
pub struct SandboxSetup {
    pub cafe_name: String,
    /// The seed as typed (shown / shared).
    pub seed_text: String,
    /// Numeric seed derived from `seed_text` (digits are used as-is, text is hashed).
    pub seed: u64,
    /// Difficulty id (`difficulty.ron`).
    pub difficulty: String,
    /// Night character id (`sandbox.ron` → characters).
    pub character: String,
    pub start_money: i64,
    pub rain_every_night: bool,
    pub pause_on_events: bool,
    pub allow_debt: bool,
}

/// Turns a typed seed into a number: plain digits parse directly, anything else is hashed
/// (FNV-1a, case-insensitive), so "PRAHA-2039" is the same town for everyone.
pub fn seed_from_text(text: &str) -> u64 {
    let t = text.trim();
    if let Ok(n) = t.parse::<u64>() {
        return n;
    }
    t.to_uppercase()
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
            (h ^ b as u64).wrapping_mul(0x0100_0000_01b3)
        })
}

/// What the menu needs to know about saves (implemented by the integration).
pub trait MenuHost {
    fn has_save(&self) -> bool;
    /// "night 3 · 01:40" under Continue (use `ui.menu.continue_hint`).
    fn continue_hint(&self, _loc: &Locale) -> Option<String> {
        None
    }
}

/// A host without saves (demos, first launch).
pub struct NoSaves;
impl MenuHost for NoSaves {
    fn has_save(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuPage {
    Language,
    Main,
    NewGame,
    Sandbox,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Item {
    Continue,
    Story,
    Sandbox,
    Settings,
    Quit,
}

struct SandboxForm {
    cafe_name: String,
    seed: String,
    difficulty: usize,
    character: f32,
    money: usize,
    rain: bool,
    pause: bool,
    debt: bool,
}

pub struct MainMenu {
    pub settings: Settings,
    page: MenuPage,
    page_t: f32,
    sel: usize,
    hovered: Option<usize>,
    lang_sel: usize,
    diff_sel: usize,
    backdrop: Backdrop,
    difficulties: Vec<String>,
    default_difficulty: usize,
    characters: Vec<String>,
    money_options: Vec<i64>,
    cafe_names: Vec<String>,
    seed_roots: Vec<String>,
    form: SandboxForm,
    audio: Vec<AudioRequest>,
    rng: u64,
    /// Shown in the corner (`ui.menu.version`).
    pub version: String,
}

impl MainMenu {
    pub fn new(content: &Content, settings: Settings, ui: &Ui) -> MainMenu {
        let difficulties: Vec<String> = content.difficulties.iter().map(|d| d.id.clone()).collect();
        let default_difficulty = content
            .difficulties
            .iter()
            .position(|d| d.default)
            .unwrap_or(0);
        let sb = &content.sandbox;
        let characters: Vec<String> = sb.characters.iter().map(|c| c.id.clone()).collect();
        let def_char = characters
            .iter()
            .position(|c| *c == sb.default_character)
            .unwrap_or(0);
        let money = sb
            .start_money_options
            .iter()
            .position(|m| *m == sb.default_start_money)
            .unwrap_or(0);
        let opt = |k: &str, d: bool| sb.options.get(k).copied().unwrap_or(d);
        let page = if settings.needs_language() {
            MenuPage::Language
        } else {
            MenuPage::Main
        };
        let lang_sel = settings.lang().index();
        let font = ui.fonts.face(Weight::Bold).clone();
        let rng = (get_time() * 1.0e6) as u64 ^ 0x5eed_2039;
        let mut m = MainMenu {
            page,
            page_t: 0.0,
            sel: 0,
            hovered: None,
            lang_sel,
            diff_sel: default_difficulty,
            backdrop: Backdrop::new(Some(font)),
            difficulties,
            default_difficulty,
            characters,
            money_options: sb.start_money_options.clone(),
            cafe_names: sb.cafe_names.clone(),
            seed_roots: sb.town_names.roots.clone(),
            form: SandboxForm {
                cafe_name: String::new(),
                seed: String::new(),
                difficulty: default_difficulty,
                character: def_char as f32,
                money,
                rain: opt("rain_every_night", true),
                pause: settings.pause_on_events || opt("pause_on_events", false),
                debt: opt("allow_debt", true),
            },
            audio: vec![
                AudioRequest::Music {
                    track: Some("mus_menu_theme".into()),
                },
                AudioRequest::Sfx {
                    id: "amb_rain_window_loop".into(),
                    volume: 0.6,
                },
            ],
            rng,
            version: env!("CARGO_PKG_VERSION").to_string(),
            settings,
        };
        m.form.seed = m.random_seed();
        m
    }

    pub fn page(&self) -> MenuPage {
        self.page
    }

    /// Jump to a page (e.g. `Settings` from the in-game pause menu, or demos).
    pub fn show(&mut self, page: MenuPage) {
        self.page = page;
        self.page_t = 0.0;
        self.sel = 0;
    }

    pub fn backdrop_mut(&mut self) -> &mut Backdrop {
        &mut self.backdrop
    }

    pub fn drain_audio_requests(&mut self) -> Vec<AudioRequest> {
        std::mem::take(&mut self.audio)
    }

    pub fn update(&mut self, dt: f32) {
        self.backdrop.update(dt);
        self.page_t = (self.page_t + dt / 0.35).min(1.0);
    }

    fn next_rand(&mut self) -> u64 {
        self.rng = self.rng.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.rng;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn random_seed(&mut self) -> String {
        let r = self.next_rand();
        let root = self
            .seed_roots
            .get((r % self.seed_roots.len().max(1) as u64) as usize)
            .cloned()
            .unwrap_or_else(|| "PRAHA".into());
        format!("{}-{:04}", root.to_uppercase(), (r >> 20) % 10_000)
    }

    fn click(&mut self, volume: f32) {
        self.audio.push(AudioRequest::Sfx {
            id: "ui_click".into(),
            volume,
        });
    }

    fn go(&mut self, page: MenuPage) {
        self.click(0.6);
        self.show(page);
    }

    /// Draws the live background and the current page. Returns an action when one is chosen.
    pub fn draw(&mut self, ui: &mut Ui, i18n: &I18n, host: &dyn MenuHost) -> Option<MenuAction> {
        self.backdrop.draw();
        ui.set_camera();
        let loc = Locale::new(i18n, self.settings.lang());
        let s = ui.screen();
        let ink = ui.theme.canvas[0];
        match self.page {
            MenuPage::Main => {
                let r = Rect::new(0.0, 0.0, (s.w * 0.55).max(520.0), s.h);
                fill_rrect(
                    r,
                    0.0,
                    Paint::horizontal(r, with_a(ink, 0.82), with_a(ink, 0.0)),
                );
            }
            _ => fill_rrect(s, 0.0, with_a(ink, 0.62)),
        }
        let k = crate::ui::ease_out(self.page_t);
        crate::ui::draw::push_alpha(k);
        let out = match self.page {
            MenuPage::Language => self.language_page(ui, i18n),
            MenuPage::Main => self.main_page(ui, &loc, host),
            MenuPage::NewGame => self.new_game_page(ui, &loc),
            MenuPage::Sandbox => self.sandbox_page(ui, &loc),
            MenuPage::Settings => self.settings_page(ui, &loc),
        };
        crate::ui::draw::pop_alpha();
        for _ in ui.drain_sounds() {
            self.click(0.5);
        }
        out
    }

    fn title(&self, ui: &mut Ui, loc: &Locale, x: f32, y: f32, centered: bool) {
        let size = 54.0;
        let ww = wordmark_width(ui, size);
        let logo = 54.0;
        let total = logo + 14.0 + ww;
        let x0 = if centered { x - total / 2.0 } else { x };
        draw_brand_mark(vec2(x0 + logo / 2.0, y - 19.0), logo, 1.0);
        draw_wordmark(ui, x0 + logo + 14.0, y, size, self.backdrop.t);
        let th = ui.theme.clone();
        let sub = TextStyle::new(16.0, Weight::Medium, th.text);
        let micro = th.micro().color(with_a(th.indigo_300, 0.8)).tracking(1.4);
        let place = "2039 · PRAHA-HOLEŠOVICE";
        if centered {
            ui.fonts
                .draw_centered(loc.tr("mainmenu.subtitle"), x, y + 34.0, &sub);
            ui.fonts.draw_centered(place, x, y + 56.0, &micro);
        } else {
            ui.fonts
                .draw(loc.tr("mainmenu.subtitle"), x + 2.0, y + 34.0, &sub);
            ui.fonts.draw(place, x + 2.0, y + 56.0, &micro);
        }
    }

    // ───────────────────────── main ─────────────────────────

    fn main_page(&mut self, ui: &mut Ui, loc: &Locale, host: &dyn MenuHost) -> Option<MenuAction> {
        let s = ui.screen();
        let th = ui.theme.clone();
        let top = (s.h * 0.2).clamp(90.0, 150.0);
        self.title(ui, loc, 64.0, top, false);

        let mut items = Vec::new();
        if host.has_save() {
            items.push(Item::Continue);
        }
        items.extend([Item::Story, Item::Sandbox, Item::Settings, Item::Quit]);
        if self.sel >= items.len() {
            self.sel = 0;
        }
        let row_h = 46.0;
        let gap = 4.0;
        let panel = Rect::new(
            56.0,
            top + 92.0,
            318.0,
            items.len() as f32 * (row_h + gap) - gap + 16.0,
        );
        ui.glass(panel, &Glass::panel().radius(14.0));

        let n = items.len();
        if ui.key_repeat(KeyCode::Down) {
            self.sel = (self.sel + 1) % n;
            ui.consume_key(KeyCode::Down);
            self.click(0.25);
        }
        if ui.key_repeat(KeyCode::Up) {
            self.sel = (self.sel + n - 1) % n;
            ui.consume_key(KeyCode::Up);
            self.click(0.25);
        }
        let mut activate = None;
        if ui.enter() {
            activate = Some(items[self.sel]);
        }
        let mut hovered = None;
        for (i, it) in items.iter().enumerate() {
            let r = Rect::new(
                panel.x + 8.0,
                panel.y + 8.0 + i as f32 * (row_h + gap),
                panel.w - 16.0,
                row_h,
            );
            let id = ui.id(("menu_item", i));
            let resp = ui.interact(id, r);
            if resp.hovered {
                hovered = Some(i);
                if ui.mouse_delta().length_squared() > 0.0 {
                    self.sel = i;
                }
            }
            if resp.clicked {
                activate = Some(*it);
            }
            let on = self.sel == i;
            let hk = ui.ease_value(id.with("sel"), if on { 1.0 } else { 0.0 }, 0.06);
            if hk > 0.01 {
                crate::ui::draw::push_alpha(hk);
                fill_rrect(
                    r,
                    10.0,
                    Paint::horizontal(r, with_a(th.indigo, 0.26), with_a(th.violet, 0.06)),
                );
                stroke_rrect(r, 10.0, 1.0, with_a(th.indigo_300, 0.22));
                fill_rrect(
                    Rect::new(r.x - 1.0, r.center_y() - 10.0, 3.0, 20.0),
                    1.5,
                    th.indigo_400,
                );
                draw_icon(
                    Icon::ChevronRight,
                    vec2(r.right() - 16.0 + hk * 2.0, r.center_y()),
                    14.0,
                    th.indigo_300,
                );
                crate::ui::draw::pop_alpha();
            }
            let (icon, label, hint): (Icon, &str, Option<String>) = match it {
                Item::Continue => (
                    Icon::Play,
                    loc.tr("ui.menu.continue"),
                    host.continue_hint(loc),
                ),
                Item::Story => (
                    Icon::Train,
                    loc.tr("ui.menu.new_story"),
                    Some(loc.tr("ui.menu.new_story_hint").to_string()),
                ),
                Item::Sandbox => (
                    Icon::Build,
                    loc.tr("ui.menu.sandbox"),
                    Some(loc.tr("ui.menu.sandbox_hint").to_string()),
                ),
                Item::Settings => (Icon::Settings, loc.tr("ui.menu.settings"), None),
                Item::Quit => (Icon::Door, loc.tr("ui.menu.quit"), None),
            };
            let ic = if on { th.indigo_300 } else { th.text_dim };
            draw_icon(icon, vec2(r.x + 22.0, r.center_y()), 18.0, ic);
            let lst = TextStyle::new(15.0, Weight::SemiBold, if on { WHITE } else { th.text });
            let tx = r.x + 44.0;
            match &hint {
                Some(h) => {
                    ui.fonts.draw(label, tx, r.y + 20.0, &lst);
                    let hs = th.small().size(11.5);
                    let h = ui.fonts.ellipsize(h, r.w - 72.0, &hs).to_string();
                    ui.fonts.draw(&h, tx, r.y + 37.0, &hs);
                }
                None => {
                    ui.fonts.draw_in(
                        label,
                        Rect::new(tx, r.y, r.w - 60.0, r.h),
                        &lst,
                        Align::Left,
                    );
                }
            }
        }
        if hovered != self.hovered {
            if hovered.is_some() {
                self.click(0.18);
            }
            self.hovered = hovered;
        }

        // footer + version
        let fs = th.small().color(with_a(th.text_dim, 0.8));
        ui.fonts
            .draw(loc.tr("ui.menu.footer"), 64.0, s.h - 26.0, &fs);
        let v = loc.trf("ui.menu.version", &[("version", &self.version)]);
        let vs = th.small().color(th.text_disabled);
        let vw = ui.fonts.measure(&v, &vs);
        ui.fonts.draw(&v, s.w - 24.0 - vw, s.h - 26.0, &vs);

        match activate? {
            Item::Continue => {
                self.click(0.6);
                Some(MenuAction::Continue)
            }
            Item::Story => {
                self.diff_sel = self.default_difficulty;
                self.go(MenuPage::NewGame);
                None
            }
            Item::Sandbox => {
                self.go(MenuPage::Sandbox);
                None
            }
            Item::Settings => {
                self.go(MenuPage::Settings);
                None
            }
            Item::Quit => Some(MenuAction::Quit),
        }
    }

    // ───────────────────────── language ─────────────────────────

    fn language_page(&mut self, ui: &mut Ui, i18n: &I18n) -> Option<MenuAction> {
        let s = ui.screen();
        let th = ui.theme.clone();
        let hl = Lang::ALL[self.lang_sel.min(3)];
        let loc = Locale::new(i18n, hl);
        let gap = 14.0;
        let two_rows = s.w < 860.0;
        let (cw, ch) = (184.0, 128.0);
        let per_row = if two_rows { 2 } else { 4 };
        let rows = 4 / per_row;
        let total_w = per_row as f32 * cw + (per_row - 1) as f32 * gap;
        let total_h = rows as f32 * ch + (rows - 1) as f32 * gap;
        let y0 = (s.h - total_h) / 2.0 + 40.0;
        self.title(ui, &loc, s.w / 2.0, y0 - 150.0, true);
        let ts = TextStyle::new(18.0, Weight::Bold, th.text);
        ui.fonts
            .draw_centered(loc.tr("ui.lang.title"), s.w / 2.0, y0 - 26.0, &ts);

        if ui.key_repeat(KeyCode::Right) || ui.key_repeat(KeyCode::Down) {
            self.lang_sel = (self.lang_sel + 1) % 4;
            self.click(0.25);
        }
        if ui.key_repeat(KeyCode::Left) || ui.key_repeat(KeyCode::Up) {
            self.lang_sel = (self.lang_sel + 3) % 4;
            self.click(0.25);
        }
        let mut pick = None;
        for (i, key) in [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4]
            .iter()
            .enumerate()
        {
            if ui.take_key(*key) {
                pick = Some(i);
            }
        }
        if ui.enter() {
            pick = Some(self.lang_sel);
        }
        let mut hovered = None;
        for (i, lang) in Lang::ALL.iter().enumerate() {
            let (cx, cy) = (i % per_row, i / per_row);
            let r = Rect::new(
                (s.w - total_w) / 2.0 + cx as f32 * (cw + gap),
                y0 + cy as f32 * (ch + gap),
                cw,
                ch,
            );
            let id = ui.id(("lang_card", i));
            let resp = ui.interact(id, r);
            if resp.hovered {
                hovered = Some(i);
                if ui.mouse_delta().length_squared() > 0.0 {
                    self.lang_sel = i;
                }
            }
            if resp.clicked {
                pick = Some(i);
            }
            let on = self.lang_sel == i;
            let hk = ui.ease_value(id.with("on"), if on { 1.0 } else { 0.0 }, 0.07);
            let r = r.translate(vec2(0.0, -hk * 4.0));
            // raised, opaque glass: the lit sign behind must not show through the text
            ui.glass(
                r,
                &Glass::card()
                    .solid()
                    .radius(14.0)
                    .elevation(crate::ui::Elevation::Low),
            );
            if hk > 0.01 {
                crate::ui::draw::push_alpha(hk);
                crate::ui::draw::glow_circle(r.center(), cw * 0.75, with_a(th.indigo, 0.16));
                fill_rrect(
                    r,
                    14.0,
                    Paint::vertical(r, with_a(th.indigo, 0.22), with_a(th.violet, 0.08)),
                );
                stroke_rrect(
                    r,
                    14.0,
                    1.5,
                    Paint::vertical(r, with_a(th.indigo_300, 0.9), with_a(th.violet_400, 0.4)),
                );
                crate::ui::draw::pop_alpha();
            }
            let own = Locale::new(i18n, *lang);
            let name = TextStyle::new(20.0, Weight::Bold, if on { WHITE } else { th.text });
            ui.fonts
                .draw_centered(lang.native_name(), r.center().x, r.y + 52.0, &name);
            let gs = th
                .small()
                .color(if on { th.indigo_300 } else { th.text_dim });
            ui.fonts.draw_centered(
                own.tr("mainmenu.lang.greeting"),
                r.center().x,
                r.y + 76.0,
                &gs,
            );
            let code = lang.short_label();
            let chip_w = ui.chip_width(code, false);
            ui.chip(
                vec2(r.center().x - chip_w / 2.0, r.bottom() - 36.0),
                code,
                if on { Tone::Accent } else { Tone::Neutral },
                None,
            );
        }
        if hovered != self.hovered {
            if hovered.is_some() {
                self.click(0.18);
            }
            self.hovered = hovered;
        }
        let hs = th.small().color(th.text_dim);
        ui.fonts
            .draw_centered(loc.tr("ui.lang.hint"), s.w / 2.0, y0 + total_h + 34.0, &hs);
        let i = pick?;
        self.lang_sel = i;
        self.settings.language = Some(Lang::ALL[i]);
        let _ = self.settings.save();
        self.go(MenuPage::Main);
        Some(MenuAction::SettingsChanged)
    }

    // ───────────────────────── new story ─────────────────────────

    fn new_game_page(&mut self, ui: &mut Ui, loc: &Locale) -> Option<MenuAction> {
        let s = ui.screen();
        let th = ui.theme.clone();
        let w = (s.w - 48.0).min(620.0);
        let h = 316.0;
        let p = Rect::new((s.w - w) / 2.0, (s.h - h) / 2.0 + 40.0, w, h);
        self.title(ui, loc, s.w / 2.0, p.y - 96.0, true);
        ui.glass(p, &Glass::panel().solid().radius(14.0));
        let (body, closed) = ui.panel_header(p, loc.tr("mainmenu.newgame.title"), None, None);
        let hint = th.small().color(th.text_dim);
        ui.fonts.draw(
            loc.tr("mainmenu.newgame.hint"),
            body.x + 18.0,
            body.y + 24.0,
            &hint,
        );

        let n = self.difficulties.len().max(1);
        if ui.key_repeat(KeyCode::Right) {
            self.diff_sel = (self.diff_sel + 1) % n;
            self.click(0.25);
        }
        if ui.key_repeat(KeyCode::Left) {
            self.diff_sel = (self.diff_sel + n - 1) % n;
            self.click(0.25);
        }
        let cards = crate::ui::row(
            Rect::new(body.x + 18.0, body.y + 40.0, body.w - 36.0, 162.0),
            n,
            12.0,
        );
        let mut start = false;
        let ids = self.difficulties.clone();
        for (i, (r, id)) in cards.iter().zip(ids.iter()).enumerate() {
            let uid = ui.id(("diff", i));
            // Draw the card's glass first: `glass` registers an input region, and a
            // region declared after `interact` would block this very card next frame.
            ui.glass(*r, &Glass::card().solid().radius(12.0));
            let resp = ui.interact(uid, *r);
            if resp.clicked {
                // a second click on the selected card starts
                start = self.diff_sel == i;
                self.diff_sel = i;
                self.click(0.4);
            }
            let on = self.diff_sel == i;
            let hk = ui.ease_value(uid.with("on"), if on { 1.0 } else { 0.0 }, 0.07);
            if hk > 0.01 {
                crate::ui::draw::push_alpha(hk);
                fill_rrect(
                    *r,
                    12.0,
                    Paint::vertical(*r, with_a(th.indigo, 0.22), with_a(th.violet, 0.06)),
                );
                stroke_rrect(
                    *r,
                    12.0,
                    1.5,
                    Paint::vertical(*r, with_a(th.indigo_300, 0.9), with_a(th.violet_400, 0.35)),
                );
                crate::ui::draw::pop_alpha();
            } else if resp.hovered {
                stroke_rrect(*r, 12.0, 1.0, with_a(th.indigo_300, 0.3));
            }
            let icon = if i == self.default_difficulty {
                Icon::Moon
            } else {
                Icon::Chart
            };
            draw_icon(
                icon,
                vec2(r.x + 26.0, r.y + 28.0),
                18.0,
                if on { th.indigo_300 } else { th.text_dim },
            );
            let name = TextStyle::new(16.0, Weight::Bold, if on { WHITE } else { th.text });
            ui.fonts.draw(
                &loc.tr_owned(&format!("difficulty.{id}.name")),
                r.x + 44.0,
                r.y + 34.0,
                &name,
            );
            if i == self.default_difficulty {
                let label = loc.tr("mainmenu.newgame.recommended");
                let cw = ui.chip_width(label, true);
                ui.chip(
                    vec2(r.right() - 12.0 - cw, r.y + 17.0),
                    label,
                    Tone::Ok,
                    Some(Icon::Star),
                );
            }
            let desc = loc.tr_owned(&format!("difficulty.{id}.desc"));
            let ds = th.small().color(th.text_dim).line_height(1.5);
            ui.fonts.draw_wrapped(
                &desc,
                Rect::new(r.x + 18.0, r.y + 52.0, r.w - 36.0, r.h - 60.0),
                &ds,
                Align::Left,
                Some(4),
            );
        }

        let (back, go) = self.footer(ui, loc, p, loc.tr("mainmenu.newgame.start"), Icon::Train);
        if back || closed || ui.esc() {
            self.go(MenuPage::Main);
            return None;
        }
        if go || start || ui.enter() {
            self.click(0.7);
            let difficulty = self
                .difficulties
                .get(self.diff_sel)
                .cloned()
                .unwrap_or_else(|| "cozy".into());
            return Some(MenuAction::StartStory { difficulty });
        }
        None
    }

    /// Back (left) + primary (right) buttons at the bottom of a panel.
    fn footer(
        &mut self,
        ui: &mut Ui,
        loc: &Locale,
        p: Rect,
        primary: &str,
        icon: Icon,
    ) -> (bool, bool) {
        let back = Button::new(loc.tr("ui.common.back"))
            .ghost()
            .icon(Icon::Back)
            .shortcut("Esc");
        let bw = ui.button_width(&back);
        let by = p.bottom() - 18.0 - 30.0;
        let b = ui
            .button("menu_back", Rect::new(p.x + 18.0, by, bw, 30.0), &back)
            .clicked;
        let start = Button::new(primary).primary().icon(icon).shortcut("Enter");
        let sw = ui.button_width(&start);
        let g = ui
            .button(
                "menu_go",
                Rect::new(p.right() - 18.0 - sw, by, sw, 30.0),
                &start,
            )
            .clicked;
        (b, g)
    }

    // ───────────────────────── sandbox ─────────────────────────

    fn sandbox_page(&mut self, ui: &mut Ui, loc: &Locale) -> Option<MenuAction> {
        let s = ui.screen();
        let th = ui.theme.clone();
        let w = (s.w - 48.0).min(640.0);
        let h = 486.0;
        let p = Rect::new((s.w - w) / 2.0, ((s.h - h) / 2.0).max(16.0), w, h);
        ui.glass(p, &Glass::panel().solid().radius(14.0));
        let (body, closed) = ui.panel_header(p, loc.tr("ui.sandbox.title"), None, None);
        let label_w = 178.0;
        let x0 = body.x + 20.0;
        let cx = x0 + label_w;
        let cw = body.right() - 20.0 - cx;
        let mut y = body.y + 14.0;
        let ls = th.body_medium();
        let label = |ui: &mut Ui, y: f32, text: &str| {
            ui.fonts.draw_in(
                text,
                Rect::new(x0, y, label_w - 10.0, 32.0),
                &ls,
                Align::Left,
            );
        };
        let hs = th.small().color(th.text_dim);

        // café name
        label(ui, y, loc.tr("ui.sandbox.cafe_name"));
        let ph = self.cafe_names.first().cloned().unwrap_or_default();
        ui.text_input(
            "sb_name",
            Rect::new(cx, y, cw.min(300.0), 32.0),
            &mut self.form.cafe_name,
            &ph,
            32,
        );
        y += 44.0;

        // seed
        label(ui, y, loc.tr("ui.sandbox.seed"));
        ui.text_input(
            "sb_seed",
            Rect::new(cx, y, 190.0, 32.0),
            &mut self.form.seed,
            "PRAHA-2039",
            24,
        );
        let dice = ui.icon_button(
            "sb_dice",
            Rect::new(cx + 198.0, y, 32.0, 32.0),
            Icon::Sparkle,
            &IconButton::secondary().tooltip(loc.tr("mainmenu.sandbox.dice")),
        );
        if dice.clicked || ui.key(KeyCode::R) {
            self.form.seed = self.random_seed();
            self.click(0.4);
        }
        ui.fonts.draw(
            loc.tr("ui.sandbox.random"),
            cx + 242.0,
            y + 20.0,
            &hs.color(th.text_disabled),
        );
        ui.fonts
            .draw(loc.tr("ui.sandbox.seed_hint"), cx, y + 48.0, &hs);
        y += 64.0;

        // difficulty
        label(ui, y, loc.tr("ui.sandbox.difficulty"));
        let names: Vec<String> = self
            .difficulties
            .iter()
            .map(|d| loc.tr_owned(&format!("difficulty.{d}.name")))
            .collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        ui.segmented(
            "sb_diff",
            Rect::new(cx, y + 2.0, cw.min(300.0), 28.0),
            &refs,
            &mut self.form.difficulty,
        );
        y += 44.0;

        // night character: stepped slider + chip
        label(ui, y, loc.tr("ui.sandbox.night_character"));
        let nc = self.characters.len().max(1);
        ui.slider(
            "sb_char",
            Rect::new(cx, y + 4.0, cw - 140.0, 24.0),
            &mut self.form.character,
            0.0,
            (nc - 1) as f32,
            Some(1.0),
        );
        let ci = (self.form.character.round() as usize).min(nc - 1);
        let cid = self.characters.get(ci).cloned().unwrap_or_default();
        let cname = loc.tr_owned(&format!("sandbox.character.{cid}.name"));
        ui.chip(
            vec2(cx + cw - 128.0, y + 5.0),
            &cname,
            Tone::Accent,
            Some(Icon::Moon),
        );
        let cdesc = loc.tr_owned(&format!("sandbox.character.{cid}.desc"));
        // two lines at most (long in cs / uk), never cut mid-sentence on one line
        let used = ui.fonts.draw_wrapped(
            &cdesc,
            Rect::new(cx, y + 34.0, cw, 32.0),
            &hs,
            Align::Left,
            Some(2),
        );
        y += 48.0 + used.max(hs.line_h());

        // start money
        label(ui, y, loc.tr("ui.sandbox.start_money"));
        let monies: Vec<String> = self
            .money_options
            .iter()
            .map(|m| format_money(loc.lang, *m))
            .collect();
        let refs: Vec<&str> = monies.iter().map(String::as_str).collect();
        ui.segmented(
            "sb_money",
            Rect::new(cx, y + 2.0, cw.min(360.0), 28.0),
            &refs,
            &mut self.form.money,
        );
        y += 46.0;

        // options
        for (key, text, v) in [
            (
                "sb_rain",
                "ui.sandbox.rain_every_night",
                &mut self.form.rain,
            ),
            (
                "sb_pause",
                "ui.sandbox.pause_on_events",
                &mut self.form.pause,
            ),
            ("sb_debt", "ui.sandbox.allow_debt", &mut self.form.debt),
        ] {
            ui.checkbox(key, Rect::new(cx, y, cw, 24.0), loc.tr(text), v, false);
            y += 28.0;
        }

        let (back, go) = self.footer(ui, loc, p, loc.tr("ui.sandbox.start"), Icon::Play);
        if back || closed || ui.esc() {
            self.go(MenuPage::Main);
            return None;
        }
        if go || ui.enter() {
            self.click(0.7);
            let seed_text = if self.form.seed.trim().is_empty() {
                self.random_seed()
            } else {
                self.form.seed.trim().to_string()
            };
            let cafe_name = if self.form.cafe_name.trim().is_empty() {
                self.cafe_names
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "Café Nocturne".into())
            } else {
                self.form.cafe_name.trim().to_string()
            };
            return Some(MenuAction::StartSandbox(SandboxSetup {
                cafe_name,
                seed: seed_from_text(&seed_text),
                seed_text,
                difficulty: self
                    .difficulties
                    .get(self.form.difficulty)
                    .cloned()
                    .unwrap_or_else(|| "cozy".into()),
                character: cid,
                start_money: self
                    .money_options
                    .get(self.form.money)
                    .copied()
                    .unwrap_or(25_000),
                rain_every_night: self.form.rain,
                pause_on_events: self.form.pause,
                allow_debt: self.form.debt,
            }));
        }
        None
    }

    // ───────────────────────── settings ─────────────────────────

    fn settings_page(&mut self, ui: &mut Ui, loc: &Locale) -> Option<MenuAction> {
        let s = ui.screen();
        let r = Rect::new(
            (s.w - PANEL_SIZE.x) / 2.0,
            ((s.h - PANEL_SIZE.y) / 2.0).max(12.0),
            PANEL_SIZE.x,
            PANEL_SIZE.y,
        );
        let before_full = self.settings.fullscreen;
        let resp = settings_panel(ui, loc, &mut self.settings, r);
        let mut out = None;
        if resp.changed {
            let _ = self.settings.save();
            ui.set_scale(self.settings.ui_scale);
            if before_full != self.settings.fullscreen {
                self.settings.apply_window();
            }
            if resp.language_changed {
                self.lang_sel = self.settings.lang().index();
            }
            out = Some(MenuAction::SettingsChanged);
        }
        if resp.close {
            self.go(MenuPage::Main);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeds_are_stable_and_case_insensitive() {
        assert_eq!(seed_from_text("2039"), 2039);
        assert_eq!(seed_from_text("praha-2039"), seed_from_text("PRAHA-2039"));
        assert_ne!(seed_from_text("PRAHA-2039"), seed_from_text("PRAHA-2040"));
    }
}
