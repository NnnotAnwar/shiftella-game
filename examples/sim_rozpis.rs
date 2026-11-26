//! Prints the default rozpis for 1–6 hires (sandbox day 1, or story day 8) — a quick look at the generator.
//! `cargo run --example sim_rozpis [story]`
use shiftella_game::content::Content;
use shiftella_game::sim::{Command, Difficulty, Shift, World};

fn main() {
    let content = Content::load("assets/data").expect("content");
    let mut w = if std::env::args().any(|a| a == "story") {
        let mut w = World::new_story(&content, Difficulty::Cozy);
        while w.night() < 8 {
            w.run_night_headless(&content);
        }
        w
    } else {
        World::new_sandbox(&content, Default::default())
    };
    for round in 0..6 {
        if round > 0 {
            let id = w.candidates().first().map(|c| c.id);
            match id {
                Some(id) => w.apply(&content, Command::Hire { candidate: id }).unwrap(),
                None => break,
            }
        }
        println!("--- {} people (incl. owner)", w.staff().len());
        let mut per_night = [0f32; 7];
        for s in w.staff() {
            let row = w.app().published[&s.id];
            let cells: Vec<&str> = row
                .iter()
                .map(|x| match x {
                    Shift::Off => " -- ",
                    Shift::Full => "22-6",
                    Shift::Early => "22-2",
                    Shift::Late => "2-6 ",
                })
                .collect();
            for (d, x) in row.iter().enumerate() {
                per_night[d] += x.hours() / 8.0;
            }
            let h: f32 = row.iter().map(|x| x.hours()).sum();
            println!(
                "{:>12} {} {h:>4.0} h",
                if s.owner { "owner" } else { &s.name },
                cells.join(" ")
            );
        }
        println!("{:>12} {:?}", "people", per_night);
    }
}
