//! `SHIFTELLA_SCENE=seats` (verification shots): guests sitting on every kind of seat piece in
//! every facing — sofas facing down / up / left / right (with a table in front), a sofa facing
//! away from its table, armchairs, a window seat — on the free floor of grandma's café.

use std::rc::Rc;

use macroquad::prelude::*;

use super::GameScreen;
use super::world_view::sim;
use crate::content::Content;

/// (def, x, y, rot): the seat pieces and the tables in front of them.
const PIECES: &[(&str, i32, i32, u8)] = &[
    // a sofa facing down at its table
    ("sofa", 23, 1, 0),
    ("table_round", 24, 2, 0),
    // a sofa facing up at its table (seen from behind)
    ("table_round", 23, 4, 0),
    ("sofa", 23, 5, 2),
    // sofas along a table, facing left and right
    ("table_round", 15, 3, 0),
    ("sofa", 16, 3, 1),
    ("table_round", 19, 4, 0),
    ("sofa", 18, 4, 3),
    // a sofa facing down with the table behind it (the guests face their table)
    ("table_round", 15, 6, 0),
    ("sofa", 15, 7, 0),
    // armchairs and a window seat on their own
    ("armchair", 26, 2, 0),
    ("armchair", 26, 6, 2),
    ("armchair", 21, 7, 1),
    ("window_seat", 7, 1, 0),
];

/// Places the pieces, seats a guest on every seat of them and frames the camera.
pub fn setup(g: &mut GameScreen, content: &Rc<Content>) {
    g.summary = None;
    g.panel = None;
    g.world.debug_set_money(1_000_000);
    let before: Vec<u32> = g.world.furniture().iter().map(|p| p.uid).collect();
    for &(def, x, y, rot) in PIECES {
        if let Err(e) = g.command(
            content,
            sim::Command::PlaceFurniture {
                def: def.into(),
                x,
                y,
                rot,
            },
        ) {
            eprintln!("seats: can't place {def} at ({x}, {y}) r{rot}: {e:?}");
        }
    }
    let seats: Vec<u32> = g
        .world
        .furniture()
        .iter()
        .filter(|p| !before.contains(&p.uid))
        .filter(|p| content.furniture_def(&p.def).is_some_and(|d| d.seats > 0))
        .map(|p| p.uid)
        .collect();
    let sat = g.world.debug_fill_seats(content, &seats);
    eprintln!("seats: {sat} guests sitting on {} pieces", seats.len());
    let _ = g.world.drain_events();
    g.cam = g
        .scene
        .overview_camera(Rect::new(0.0, 0.0, screen_width(), screen_height()));
    g.cam.center = vec2(20.0, 4.4) * crate::map::TILE;
    g.cam.zoom *= 2.3;
    g.speed = 0;
    g.cam_ready = true;
}
