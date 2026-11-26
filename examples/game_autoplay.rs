//! Headless smoke run of the hands-on loop: Karolina plays story nights by herself through
//! the game screen's own intents (the same paths as the mouse and the keyboard), mini-games
//! with their scripted input, and the morning summaries are printed.
//!
//! ```sh
//! cargo run --release --example game_autoplay -- [nights=1] [speed=1]
//! ```
//! For a windowed run with screenshots use the game itself:
//! `SHIFTELLA_SCENE=night1_handson SHIFTELLA_AUTOPLAY=1 SHIFTELLA_SHOT=shots/run.png
//! SHIFTELLA_SHOT_EVERY=300 SHIFTELLA_FRAMES=3000`.

use std::rc::Rc;

use shiftella_game::content::Content;
use shiftella_game::screens::game::GameScreen;
use shiftella_game::settings::{Locale, load_i18n};
use shiftella_game::sim::Phase;

fn main() {
    let mut args = std::env::args().skip(1);
    let nights: u32 = args.next().and_then(|a| a.parse().ok()).unwrap_or(1);
    let speed: u8 = args.next().and_then(|a| a.parse().ok()).unwrap_or(1);
    let root = env!("CARGO_MANIFEST_DIR");
    let content = Rc::new(Content::load(&format!("{root}/assets/data")).expect("content"));
    let i18n = load_i18n(&format!("{root}/assets/i18n")).expect("i18n");
    let loc = Locale::new(&i18n, shiftella_game::i18n::Lang::En);
    let mut g = GameScreen::new_story(&content, "cozy", true);
    for _ in 0..nights {
        g.summary = None;
        g.start_night(&content);
        g.set_speed(speed);
        let mut real = 0.0;
        while g.world.phase() == Phase::Night && real < 7200.0 {
            g.play_headless(&content, &loc, 60.0, &|_| false);
            real += 60.0;
        }
        let s = g
            .summary
            .clone()
            .or_else(|| g.world.last_summary().cloned())
            .expect("the morning came");
        println!(
            "night {:>2}: ~{:>4.0} real s at ×{speed} · served {:>3} · Karolina {:>3} (by hand {:>2}) · \
             wishes {:>2} (perfect {:>2}) · idle {:>3} min · forgiven {} · unhappy {} · tips {} · revenue {}",
            s.night,
            real,
            s.guests_served,
            s.karolina_items,
            s.minigames,
            s.wishes_served,
            s.wishes_perfect,
            s.karolina_idle_minutes,
            s.guests_forgiven,
            s.guests_left_unhappy,
            s.tips,
            s.revenue,
        );
        // the next day: hire whoever is offered (the first helper comes on night 3)
        let ids: Vec<u32> = g.world.candidates().iter().map(|c| c.id).collect();
        for id in ids {
            let _ = g.command(
                &content,
                shiftella_game::sim::Command::Hire { candidate: id },
            );
        }
    }
}
