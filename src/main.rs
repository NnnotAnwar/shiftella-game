//! Shiftella — entry point. The game itself lives in the library (`src/lib.rs`):
//! [`shiftella_game::app::run`] boots, shows the menu and runs the state machine.

use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "Shiftella".to_owned(),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,  // crisp on Retina
        sample_count: 4, // MSAA
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    shiftella_game::app::run().await;
}
