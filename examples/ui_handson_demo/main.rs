//! Hands-on widgets gallery (design `handson.md` T6) on mock data.
//! `cargo run --example ui_handson_demo`
//!
//! Tab / Shift+Tab: next / previous page · F2: language · F3/F4: UI scale −/+.
//! Screenshot: `SHIFTELLA_SHOT=shots/x.png SHIFTELLA_PAGE=order cargo run --example ui_handson_demo`
//! Optional: `SHIFTELLA_UI_SCALE=1.2`, `SHIFTELLA_LANG=cs|en|uk|ru`, `SHIFTELLA_DPI=1` (no
//! Retina), `SHIFTELLA_FRAMES=40`, `SHIFTELLA_LAYOUT_DEBUG=1`, `SHIFTELLA_W` / `SHIFTELLA_H`.
//! Pages: hud order rail moment duties records book pantry atmosphere rozpis payroll journal
//! polaroid coach.

mod lang;
mod mock;
mod pages;

use macroquad::prelude::*;

use shiftella_game::render::PersonLook;
use shiftella_game::ui::*;

use lang::Lang;

fn conf() -> Conf {
    let env = |k: &str, d: i32| {
        std::env::var(k)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(d)
    };
    Conf {
        window_title: "Shiftella — hands-on widgets".to_owned(),
        window_width: env("SHIFTELLA_W", 1280),
        window_height: env("SHIFTELLA_H", 760),
        high_dpi: std::env::var("SHIFTELLA_DPI").map_or(true, |v| v != "1"),
        sample_count: 4,
        ..Default::default()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Hud,
    Order,
    Rail,
    Moment,
    Duties,
    Records,
    Book,
    Pantry,
    Atmosphere,
    Rozpis,
    Payroll,
    Journal,
    Polaroid,
    Coach,
}

pub const PAGES: [(Page, &str); 14] = [
    (Page::Hud, "hud"),
    (Page::Order, "order"),
    (Page::Rail, "rail"),
    (Page::Moment, "moment"),
    (Page::Duties, "duties"),
    (Page::Records, "records"),
    (Page::Book, "book"),
    (Page::Pantry, "pantry"),
    (Page::Atmosphere, "atmosphere"),
    (Page::Rozpis, "rozpis"),
    (Page::Payroll, "payroll"),
    (Page::Journal, "journal"),
    (Page::Polaroid, "polaroid"),
    (Page::Coach, "coach"),
];

/// People with faces in the demo (display name → look).
const FACES: [(&str, &str); 20] = [
    ("Mirek", "mirek"),
    ("Vera", "vera"),
    ("Radek", "radek"),
    ("Hana", "hana"),
    ("Filip", "filip"),
    ("Lori", "lori"),
    ("Karolina", "player"),
    ("Karolina (you)", "player"),
    ("Karolína (ty)", "player"),
    ("Кароліна (ти)", "player"),
    ("Каролина (ты)", "player"),
    ("Tereza Nováková", "guest_tereza"),
    ("Jonas Weber", "guest_jonas"),
    ("Olena Bondar", "guest_olena"),
    ("Lukas", "guest_lukas"),
    ("Maria", "guest_maria"),
    ("Pavel", "guest_pavel"),
    ("Sofia", "guest_sofia"),
    ("Andrej", "guest_andrej"),
    ("Ewa", "guest_ewa"),
];

const STAFF: [(&str, u64); 6] = [
    ("Kuba", 11),
    ("Oleh", 23),
    ("Bára", 37),
    ("Ema", 41),
    ("Zdeněk", 53),
    ("Ivana", 67),
];

#[macroquad::main(conf)]
async fn main() {
    let fonts = Fonts::load().await.expect("fonts in assets/fonts");
    let mut ui = Ui::new(fonts);
    if let Some(sc) = std::env::var("SHIFTELLA_UI_SCALE")
        .ok()
        .and_then(|s| s.parse().ok())
    {
        ui.set_scale(sc);
    }
    let shot = std::env::var("SHIFTELLA_SHOT").ok();
    let frames: u32 = std::env::var("SHIFTELLA_FRAMES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(40);
    let page = std::env::var("SHIFTELLA_PAGE")
        .ok()
        .and_then(|p| PAGES.iter().find(|(_, n)| *n == p).map(|(pg, _)| *pg))
        .unwrap_or(Page::Hud);
    let mut d = pages::Demo::new(page, Lang::from_env());
    let mut frame = 0;
    loop {
        ui.begin_frame();
        for (name, preset) in FACES {
            ui.register_face(name, PersonLook::preset(preset));
        }
        for (name, seed) in STAFF {
            let mut look = PersonLook::random(seed, true);
            look.apron = true;
            ui.register_face(name, look);
        }
        d.prepare_textures();
        clear_background(ui.theme.canvas[0]);
        ui.set_camera();
        pages::draw(&mut ui, &mut d);
        for _ev in ui.draw_toasts(ToastAnchor::BottomRight) {}
        ui.end_frame();

        if ui.key(KeyCode::Tab) {
            let i = PAGES.iter().position(|(p, _)| *p == d.page).unwrap_or(0);
            let n = PAGES.len();
            let j = if ui.shift() {
                (i + n - 1) % n
            } else {
                (i + 1) % n
            };
            d.page = PAGES[j].0;
        }
        if ui.key(KeyCode::F2) {
            d.lang = d.lang.next();
        }
        if ui.key(KeyCode::F3) {
            let sc = (ui.scale() - 0.1).max(0.9);
            ui.set_scale(sc);
        }
        if ui.key(KeyCode::F4) {
            let sc = (ui.scale() + 0.1).min(1.3);
            ui.set_scale(sc);
        }
        let _ = ui.drain_sounds();

        if let Some(path) = &shot {
            frame += 1;
            if frame >= frames {
                let mut img = get_screen_data();
                for px in img.bytes.as_chunks_mut::<4>().0 {
                    px[3] = 255;
                }
                img.export_png(path);
                break;
            }
        }
        next_frame().await;
    }
}
