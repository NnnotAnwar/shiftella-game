//! Night Shift — точка входа и главный цикл.
//!
//! Каждый кадр:
//!   1. читаем ввод (игра + камера)
//!   2. двигаем симуляцию фиксированными шагами (тиками)
//!   3. рисуем мир через камеру, потом интерфейс поверх

mod audio;
mod camera;
mod clock;
mod content;
mod game;
mod hud;
mod i18n;
mod map;
mod palette;
mod render;
mod shapes;
mod sim;
mod ui;

use macroquad::prelude::*;

use audio::Audio;
use camera::WorldCamera;
use game::{Game, Speed};
use map::TILE;

/// Длина одного тика симуляции в реальных секундах (при скорости ×1).
const TICK_SECONDS: f32 = 0.1;

fn window_conf() -> Conf {
    Conf {
        window_title: "Shiftella".to_owned(),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,  // чёткая картинка на Retina
        sample_count: 4, // сглаживание краёв (MSAA)
        ..Default::default()
    }
}

fn handle_input(game: &mut Game) {
    if is_key_pressed(KeyCode::Space) {
        game.toggle_pause();
    }
    if is_key_pressed(KeyCode::Key1) {
        game.set_speed(Speed::Normal);
    }
    if is_key_pressed(KeyCode::Key2) {
        game.set_speed(Speed::Fast);
    }
    if is_key_pressed(KeyCode::Key3) {
        game.set_speed(Speed::Faster);
    }
    if game.night_over && is_key_pressed(KeyCode::N) {
        game.start_next_night();
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    // Пока грузится звук — показываем «Loading…», чтобы окно не было пустым.
    clear_background(palette::BACKGROUND);
    draw_text("Loading...", 40.0, 80.0, 48.0, palette::TEXT_DIM);
    next_frame().await;
    let mut audio = Audio::load().await;

    let mut game = Game::new();
    let mut camera = WorldCamera::fit(game.map.size_px(), hud::map_area());
    let mut accumulator = 0.0;

    // Помощник для разработки: NIGHT_SHIFT_SHOT=shot.png cargo run
    // сохранит скриншот через полсекунды и закроет игру.
    let shot_path = std::env::var("NIGHT_SHIFT_SHOT").ok();
    let mut frame = 0;

    loop {
        handle_input(&mut game);
        if is_key_pressed(KeyCode::M) {
            audio.toggle_mute();
        }
        camera.update();

        // Фиксированный шаг: сколько бы ни было FPS, мир идёт с одной скоростью.
        // .min(0.25) — защита от «рывка», если окно подвисло.
        accumulator += get_frame_time().min(0.25) * game.speed.multiplier();
        while accumulator >= TICK_SECONDS {
            accumulator -= TICK_SECONDS;
            game.tick();
        }

        // Звук реагирует на то, что случилось в мире за этот кадр.
        for event in game.drain_events() {
            audio.handle(event);
        }

        let hovered = if hud::is_mouse_over_ui() {
            None
        } else {
            let world_mouse = camera.screen_to_world(Vec2::from(mouse_position()));
            game.map.tile_at(world_mouse)
        };

        // Мир
        clear_background(palette::BACKGROUND);
        set_camera(&camera.to_camera2d());
        game.map.draw();
        if let Some((x, y)) = hovered {
            let (px, py) = (x as f32 * TILE, y as f32 * TILE);
            draw_rectangle_lines(px, py, TILE, TILE, 3.0, palette::NEON);
        }

        // Интерфейс поверх
        hud::draw(&game, hovered);

        if let Some(path) = &shot_path {
            frame += 1;
            if frame == 30 {
                get_screen_data().export_png(path);
                break;
            }
        }

        next_frame().await;
    }
}
