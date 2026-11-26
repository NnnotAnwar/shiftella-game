//! Main menu demo.
//!
//! * `SHIFTELLA_PAGE=menu|language|newgame|sandbox|settings` (default: language on first
//!   launch of the demo's own settings file, else menu)
//! * `SHIFTELLA_LANG=cs|en|uk|ru`, `SHIFTELLA_T=<seconds>` (background time; a train passes
//!   at 5–14 s, then every 40 s), `SHIFTELLA_SAVE=1` (pretend a save exists)
//! * `SHIFTELLA_SHOT=path.png` — render ~40 frames, save the PNG and exit
//!
//! Settings go to `SHIFTELLA_SETTINGS` (default: a temp file, so the demo never touches your
//! real settings).

use macroquad::prelude::*;

use shiftella_game::content::Content;
use shiftella_game::i18n::Lang;
use shiftella_game::screens::menu::{MainMenu, MenuAction, MenuHost, MenuPage};
use shiftella_game::settings::{Locale, Settings, load_i18n};
use shiftella_game::ui::{Fonts, Ui};

fn conf() -> Conf {
    shiftella_game::platform::hide_for_automation();
    Conf {
        window_title: "Shiftella — menu demo".to_owned(),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        sample_count: 4,
        ..Default::default()
    }
}

struct DemoHost {
    save: bool,
}

impl MenuHost for DemoHost {
    fn has_save(&self) -> bool {
        self.save
    }
    fn continue_hint(&self, loc: &Locale) -> Option<String> {
        Some(loc.trf(
            "ui.menu.continue_hint",
            &[("night", "3"), ("time", "01:40")],
        ))
    }
}

#[macroquad::main(conf)]
async fn main() {
    if std::env::var("SHIFTELLA_SETTINGS").is_err() {
        let p = std::env::temp_dir().join("shiftella-menu-demo-settings.ron");
        // SAFETY: single-threaded at this point, before anything reads the environment.
        unsafe { std::env::set_var("SHIFTELLA_SETTINGS", p) };
    }
    let root = env!("CARGO_MANIFEST_DIR");
    let content = Content::load(&format!("{root}/assets/data")).expect("content");
    let i18n = load_i18n(&format!("{root}/assets/i18n")).expect("i18n");
    let fonts = Fonts::load_from(&format!("{root}/assets/fonts"))
        .await
        .expect("fonts");
    let mut ui = Ui::new(fonts);
    let shot = std::env::var("SHIFTELLA_SHOT").ok();
    let page = std::env::var("SHIFTELLA_PAGE").ok();
    let mut settings = Settings::load();
    if let Some(l) = std::env::var("SHIFTELLA_LANG")
        .ok()
        .and_then(|c| Lang::from_code(&c))
    {
        settings.language = Some(l);
    }
    if page.as_deref() == Some("language") {
        settings.language = None;
    } else if page.is_some() && settings.language.is_none() {
        settings.language = Some(Lang::Cs);
    }
    ui.set_scale(settings.ui_scale);
    let mut menu = MainMenu::new(&content, settings, &ui);
    match page.as_deref() {
        Some("menu") => menu.show(MenuPage::Main),
        Some("newgame") => menu.show(MenuPage::NewGame),
        Some("sandbox") => menu.show(MenuPage::Sandbox),
        Some("settings") => menu.show(MenuPage::Settings),
        Some("language") => menu.show(MenuPage::Language),
        _ => {}
    }
    if let Some(t) = std::env::var("SHIFTELLA_T")
        .ok()
        .and_then(|s| s.parse::<f32>().ok())
    {
        menu.backdrop_mut().t = t;
    }
    let host = DemoHost {
        save: std::env::var("SHIFTELLA_SAVE").is_ok(),
    };

    let mut frame = 0u32;
    loop {
        ui.begin_frame();
        let dt = if shot.is_some() { 1.0 / 60.0 } else { ui.dt() };
        menu.update(dt);
        if let Some(action) = menu.draw(&mut ui, &i18n, &host) {
            println!("action: {action:?}");
            if action == MenuAction::Quit {
                break;
            }
        }
        for a in menu.drain_audio_requests() {
            if shot.is_none() {
                println!("audio: {a:?}");
            }
        }
        ui.end_frame();

        frame += 1;
        if let Some(path) = &shot
            && frame >= 40
        {
            let mut img = get_screen_data();
            for px in img.bytes.as_chunks_mut::<4>().0 {
                px[3] = 255;
            }
            img.export_png(path);
            println!("saved {path}");
            break;
        }
        next_frame().await;
    }
}
