//! Cutscene player demo: plays the opening (or any cutscene) from the real content.
//!
//! * `SHIFTELLA_SCENE=<id>` — cutscene id (default `opening`)
//! * `SHIFTELLA_T=<seconds>` — jump ahead (auto-played at the default reading pace)
//! * `SHIFTELLA_LANG=cs|en|sk|uk`
//! * `SHIFTELLA_SHOT=path.png` — render ~40 frames, save the PNG and exit
//!
//! Keys: click / Space — next, 1–4 — choices, hold Esc — skip, A — auto-advance, Ctrl — fast.

use macroquad::prelude::*;

use shiftella_game::content::Content;
use shiftella_game::cutscene::{CutscenePlayer, CutsceneStatus};
use shiftella_game::i18n::Lang;
use shiftella_game::settings::{Locale, load_i18n};
use shiftella_game::ui::{Fonts, Ui};

fn conf() -> Conf {
    Conf {
        window_title: "Shiftella — cutscene demo".to_owned(),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        sample_count: 4,
        ..Default::default()
    }
}

#[macroquad::main(conf)]
async fn main() {
    let root = env!("CARGO_MANIFEST_DIR");
    let content = Content::load(&format!("{root}/assets/data")).expect("content");
    let i18n = load_i18n(&format!("{root}/assets/i18n")).expect("i18n");
    let fonts = Fonts::load_from(&format!("{root}/assets/fonts"))
        .await
        .expect("fonts");
    let mut ui = Ui::new(fonts);
    let lang = std::env::var("SHIFTELLA_LANG")
        .ok()
        .and_then(|c| Lang::from_code(&c))
        .unwrap_or(Lang::Cs);
    let scene_id = std::env::var("SHIFTELLA_SCENE").unwrap_or_else(|_| "opening".into());
    let shot = std::env::var("SHIFTELLA_SHOT").ok();
    let jump: f32 = std::env::var("SHIFTELLA_T")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);

    let Some(def) = content.cutscene(&scene_id).cloned() else {
        eprintln!("no cutscene `{scene_id}`");
        return;
    };
    let mut player = CutscenePlayer::new(&content);
    let loc = Locale::new(&i18n, lang);
    player.start(&def);
    if jump > 0.0 {
        player.fast_forward(&loc, jump);
    }
    if shot.is_some() {
        player.set_text_speed(0.0);
    }

    let mut frame = 0u32;
    loop {
        ui.begin_frame();
        let dt = if shot.is_some() { 1.0 / 60.0 } else { ui.dt() };
        match player.update(&mut ui, &loc, dt) {
            CutsceneStatus::Finished { id, choices } => {
                println!("finished {id}: choices {choices:?}");
                println!("effects {:?}", player.outcome().effects);
            }
            CutsceneStatus::Idle
                if shot.is_none()
                    && (is_key_pressed(KeyCode::R)
                        || is_mouse_button_pressed(MouseButton::Left)) =>
            {
                player.start(&def);
            }
            _ => {}
        }
        for a in player.drain_audio_requests() {
            println!("audio: {a:?}");
        }
        let _ = player.drain_effects();
        player.draw_world();
        ui.set_camera();
        player.draw_ui(&mut ui, &loc);
        ui.end_frame();
        let _ = ui.drain_sounds();

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
