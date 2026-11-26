//! QA regression demo for the glyph-atlas growth bug (handwritten Cyrillic captions drawn as
//! black boxes): every frame draws handwritten captions first and then text at a brand-new
//! font size (new glyphs → the font atlas grows mid-frame). Before the fix the captions drawn
//! earlier in that frame sampled the deleted atlas texture (black boxes).
//!
//! `SHIFTELLA_SHOT=shots/atlas.png cargo run --example qa_glyph_atlas`

use macroquad::prelude::*;
use shiftella_game::ui::handson::common::draw_handwritten_block;
use shiftella_game::ui::*;

fn conf() -> Conf {
    shiftella_game::platform::hide_for_automation();
    Conf {
        window_title: "Shiftella — glyph atlas".to_owned(),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        ..Default::default()
    }
}

const CAPTIONS: [&str; 4] = [
    "3:33. Часы остановились, и он снова был здесь.",
    "Мирек: «То, что надо». Первая ночь. Ключи были ещё холодными.",
    "Лори пела на перроне — дождь, гитара и ваш какао.",
    "Ніч 4 · Годинник зупинився, і він знову був тут.",
];

#[macroquad::main(conf)]
async fn main() {
    let fonts = Fonts::load().await.expect("fonts");
    let mut ui = Ui::new(fonts);
    let shot = std::env::var("SHIFTELLA_SHOT").ok();
    let frames: u32 = std::env::var("SHIFTELLA_FRAMES")
        .ok()
        .and_then(|f| f.parse().ok())
        .unwrap_or(8);
    for frame in 0..frames {
        ui.begin_frame();
        clear_background(Color::from_hex(0x3a2c22));
        let ink = Color::from_hex(0x2b2440);
        for (i, c) in CAPTIONS.iter().enumerate() {
            let r = Rect::new(40.0, 40.0 + i as f32 * 90.0, 560.0, 80.0);
            draw::fill_rrect(r, 8.0, draw::Paint::Solid(Color::from_hex(0xf2ece0)));
            let st = TextStyle::new(18.0, Weight::Medium, ink);
            draw_handwritten_block(&mut ui, c, r.inset(8.0), &st, Align::Left, 3);
        }
        // a new size every frame: fresh glyphs for every weight → the atlases keep growing
        let size = 20.0 + frame as f32 * 1.5;
        for (k, w) in [
            Weight::Regular,
            Weight::Medium,
            Weight::SemiBold,
            Weight::Bold,
        ]
        .into_iter()
        .enumerate()
        {
            let st = TextStyle::new(size, w, WHITE);
            ui.fonts.draw(
                "Щёлкни ЖЮЁ ЯЭЪ ЫЧЦ — žluťoučký kůň ĎŤŇ",
                640.0,
                80.0 + k as f32 * (size + 30.0),
                &st,
            );
        }
        ui.end_frame();
        if frame + 1 == frames {
            if let Some(p) = &shot {
                let mut img = get_screen_data();
                for px in img.bytes.as_chunks_mut::<4>().0 {
                    px[3] = 255;
                }
                img.export_png(p);
                println!("saved {p}");
            }
            break;
        }
        next_frame().await;
    }
}
