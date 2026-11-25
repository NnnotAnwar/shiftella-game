//! Shiftella — entry point. The game itself lives in the library (`src/lib.rs`):
//! [`shiftella_game::app::run`] boots, shows the menu and runs the state machine.

use macroquad::prelude::*;

/// Window size in logical points; `SHIFTELLA_WINDOW=WxH` overrides it (screenshots, QA).
fn window_size() -> (i32, i32) {
    std::env::var("SHIFTELLA_WINDOW")
        .ok()
        .and_then(|v| {
            let (w, h) = v.split_once('x')?;
            Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
        })
        .filter(|&(w, h): &(i32, i32)| w >= 640 && h >= 400)
        .unwrap_or((1280, 720))
}

fn window_conf() -> Conf {
    let (window_width, window_height) = window_size();
    Conf {
        window_title: "Shiftella".to_owned(),
        window_width,
        window_height,
        high_dpi: true,  // crisp on Retina
        sample_count: 4, // MSAA
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    shiftella_game::app::run().await;
}
