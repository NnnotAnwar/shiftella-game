//! Debug trace: prints staff tasks and party states every N game minutes of night 1.
//! `cargo run --example sim_trace -- [night] [every_minutes]`
use shiftella_game::content::Content;
use shiftella_game::sim::{Command, Difficulty, Phase, World};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let target: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    let every: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(30);
    let content = Content::load("assets/data").expect("content");
    let diff = if std::env::var("CLASSIC").is_ok() {
        Difficulty::Classic
    } else {
        Difficulty::Cozy
    };
    let mut w = World::new_story(&content, diff);
    loop {
        while let Some(id) = w.playing_cutscene().map(str::to_string) {
            let _ = w.apply(
                &content,
                Command::CutsceneFinished {
                    id,
                    choices: vec![],
                },
            );
        }
        if w.phase() == Phase::Day {
            if w.night() > target {
                break;
            }
            let ids: Vec<u32> = w.candidates().iter().map(|c| c.id).collect();
            for id in ids.into_iter().take(3usize.saturating_sub(w.staff().len())) {
                let _ = w.apply(&content, Command::Hire { candidate: id });
            }
            w.start_night(&content);
        }
        if w.waiting_for_doors() {
            let _ = w.apply(&content, Command::OpenDoors);
        } else if w.waiting_to_close() {
            let _ = w.apply(&content, Command::CloseUp);
            continue;
        }
        let last = w.minute();
        w.tick(&content);
        let _ = w.drain_events();
        if w.night() == target
            && w.phase() == Phase::Night
            && w.minute() != last
            && w.minute().is_multiple_of(every)
        {
            println!(
                "--- {} q={} guests={}",
                w.clock_label(),
                w.queue_len(),
                w.guests_inside()
            );
            for s in w.staff() {
                println!(
                    "  staff {} {:?} task={:?} path={} e={:.0} m={:.0} h={:.0} pos={:?}",
                    s.id,
                    s.status,
                    s.task,
                    s.path.len(),
                    s.energy,
                    s.mood,
                    s.hunger,
                    s.tile()
                );
            }
            for p in w.parties() {
                println!(
                    "  party {} {:?} {:?} pat={:.1} wishes={:?} items={:?}",
                    p.id,
                    p.archetype,
                    p.state,
                    p.patience,
                    p.wishes,
                    p.items
                        .iter()
                        .map(|i| w
                            .order_items()
                            .iter()
                            .find(|x| x.id == *i)
                            .map(|x| format!("{:?}", x.state)))
                        .collect::<Vec<_>>()
                );
            }
        }
    }
}
