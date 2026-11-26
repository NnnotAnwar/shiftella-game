//! Headless smoke run of the hands-on loop: Karolina plays story nights by herself through
//! the game screen's own intents (the same paths as the mouse and the keyboard), mini-games
//! with their scripted input, moments, rituals, experiments and the handover; the days are
//! planned by the sim's player-bot and the morning polaroid is kept through the screen.
//!
//! ```sh
//! cargo run --release --example game_autoplay -- [nights=1] [speed=1] [difficulty=cozy] [skip]
//! ```
//! `skip`: Settings → Relaxed mini-games on and every mini-game skipped (the T10 check that
//! such a player still reaches an ending).
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
    let difficulty = args.next().unwrap_or_else(|| "cozy".into());
    let skip = args.next().is_some_and(|a| a == "skip");
    let root = env!("CARGO_MANIFEST_DIR");
    let content = Rc::new(Content::load(&format!("{root}/assets/data")).expect("content"));
    let i18n = load_i18n(&format!("{root}/assets/i18n")).expect("i18n");
    let loc = Locale::new(&i18n, shiftella_game::i18n::Lang::En);
    let mut g = GameScreen::new_story(&content, &difficulty, true);
    g.relaxed = skip;
    g.hands.autoplay = Some(shiftella_game::screens::game::hands::Autoplay {
        cooldown: 0.0,
        keyboard: false,
        skip_minigames: skip,
    });
    let mut total_real = 0.0;
    for _ in 0..nights {
        if g.world.ending().is_some() || g.world.is_closed() {
            break;
        }
        // the day: the morning polaroid, the bot's plan, then the night starts
        let mut guard = 0;
        while g.world.phase() == Phase::Day && guard < 20 {
            if g.cutscene_playing() {
                g.finish_cutscene_now(&content, &loc);
            }
            g.autoplay_day(&content, 1.0, 0.0);
            guard += 1;
        }
        g.set_speed(speed);
        let mut real = 0.0;
        while g.world.phase() == Phase::Night && real < 7200.0 {
            g.play_headless(&content, &loc, 60.0, &|_| false);
            real += 60.0;
        }
        total_real += real;
        let Some(s) = g
            .summary
            .clone()
            .or_else(|| g.world.last_summary().cloned())
        else {
            println!("no summary (ending?)");
            break;
        };
        let sys = g.world.systems();
        println!(
            "night {:>2}: ~{:>4.0} s ×{speed} (overlays {:>3.0} s) · served {:>3} · K {:>3} (hand {:>2}) · wishes {:>2}/{:>2} · \
             moments {} · idle {:>3} · tips {:>4} · money {:>6} · coz {:>3.0} · cat {} · napkins {} · \
             journal {} · theme {} · garlands {} · rituals {:?}",
            s.night,
            g.hands.night_real,
            g.hands.overlay_real,
            s.guests_served,
            s.karolina_items,
            s.minigames,
            s.wishes_perfect,
            s.wishes_served,
            s.moments_resolved,
            s.karolina_idle_minutes,
            s.tips,
            g.world.money(),
            g.world.coziness().value,
            g.world.cat_level(),
            g.world.napkins().len(),
            g.world.journal().len() + usize::from(g.world.pending_polaroids().is_some()),
            g.world.theme(&content),
            g.world.garlands_up(&content),
            sys.tool.rituals,
        );
    }
    // keep the last morning's polaroid too
    g.autoplay_day(&content, 1.0, 0.0);
    let sys = g.world.systems();
    println!(
        "done: night {} · ending {:?} · real {:.0} min · journal {} · knows {} recipes · \
         themes owned {} · payroll weeks sent {} · moments ever {}",
        g.world.night(),
        g.world.ending(),
        total_real / 60.0,
        g.world.journal().len(),
        g.world.known_recipes().count(),
        sys.atmos.owned.len(),
        sys.tool.payroll_sent.len(),
        sys.moments.resolved_ever.len(),
    );
}
