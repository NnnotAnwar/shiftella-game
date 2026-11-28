//! 0.2.7 scene fixes over the live café: every scene's actors stand on the floor or sit on a
//! seat of the player's own layout (Mrs Vera stood on the corner table), and a regular's
//! hello on walking in is a bubble, not a card to click away.

use std::rc::Rc;

use super::*;
use crate::content::Content;
use crate::settings::load_i18n;

fn content() -> Rc<Content> {
    let root = env!("CARGO_MANIFEST_DIR");
    Rc::new(Content::load(&format!("{root}/assets/data")).expect("content"))
}

#[test]
fn scenes_over_the_live_cafe_never_leave_anybody_on_furniture() {
    let c = content();
    let root = env!("CARGO_MANIFEST_DIR");
    let i = load_i18n(&format!("{root}/assets/i18n")).expect("i18n");
    let loc = Locale::new(&i, crate::i18n::Lang::En);
    let mut g = GameScreen::new_story(&c, "cozy", true);
    g.fast_forward(&c, &loc, 8, Some(150), true);
    let seated_before = g
        .world
        .guest_views()
        .iter()
        .filter(|v| v.seat_tile.is_some())
        .count();
    assert!(seated_before > 0, "guests sit in the café");
    let mut checked = 0;
    for def in &c.cutscenes.cutscenes {
        if def.id == "opening" {
            continue;
        }
        g.stage_cutscene_at(&c, &loc, &def.id, 0.0);
        let mut guard = 0;
        while g.player.is_playing() && guard < 4000 {
            g.player.fast_forward_with(&loc, 0.25, &[0]);
            let grid = g.world.grid();
            for a in g.player.actors.iter().filter(|a| !a.walking() && !a.gone) {
                let t = crate::map::TILE;
                let (tile, sitting) = match a.sit.filter(|_| a.seated()) {
                    Some((s, _)) => (s, true),
                    None => (
                        ((a.pos.x / t).floor() as i32, (a.pos.y / t).floor() as i32),
                        false,
                    ),
                };
                let inside = grid.tile(tile.0, tile.1).is_some();
                let ok = if sitting {
                    grid.is_seat(tile.0, tile.1) && a.id != "player"
                } else {
                    !inside || grid.walkable(tile.0, tile.1) && !grid.is_seat(tile.0, tile.1)
                };
                assert!(ok, "{}: {} at {tile:?} (sitting {sitting})", def.id, a.id);
                checked += 1;
            }
            guard += 1;
        }
    }
    assert!(checked > 100, "actors were checked ({checked})");
}

#[test]
fn a_regulars_hello_is_a_bubble_and_story_lines_are_held() {
    for key in [
        "regular.vera.line.0a",
        "regular.mirek.line.1a",
        "regular.vera.hello.1",
    ] {
        assert!(!speech_held_to_read(key), "{key} doesn't stop the night");
    }
    assert!(speech_held_to_read("story.line.n3_grandma"));
}
