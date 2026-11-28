//! The Overlay section's debug drawing over the café (world camera) and over the UI.

use macroquad::prelude::*;

use super::panel::Overlays;
use crate::content::Content;
use crate::screens::game::GameScreen;
use crate::sim::grid::{TILE, rotated_size, tile_center};
use crate::ui::Ui;
use crate::ui::draw::stroke_rrect;

/// Nav grid and seat claims over the floor (call after the game drew its frame).
pub fn draw_world(g: &GameScreen, content: &Content, o: &Overlays) {
    if !o.nav && !o.seats {
        return;
    }
    set_camera(&g.cam.to_camera2d());
    let w = &g.world;
    let grid = w.grid();
    if o.nav {
        for y in 0..grid.height {
            for x in 0..grid.width {
                let c = if grid.is_seat(x, y) {
                    Color::new(1.0, 0.75, 0.2, 0.28)
                } else if grid.guest_ok(x, y) {
                    Color::new(0.3, 0.55, 1.0, 0.22)
                } else if grid.walkable(x, y) {
                    Color::new(0.25, 0.9, 0.45, 0.22)
                } else {
                    Color::new(1.0, 0.25, 0.3, 0.16)
                };
                let r = Rect::new(
                    x as f32 * TILE + 2.0,
                    y as f32 * TILE + 2.0,
                    TILE - 4.0,
                    TILE - 4.0,
                );
                draw_rectangle(r.x, r.y, r.w, r.h, c);
            }
        }
    }
    if o.seats {
        for p in w.furniture() {
            let Some(def) = content.furniture_def(&p.def) else {
                continue;
            };
            let busy = p.seat_by.iter().any(Option::is_some);
            if p.seat_by.is_empty() && p.user.is_none() && p.claim.is_none() {
                continue;
            }
            let (fw, fh) = rotated_size(def, p.rot);
            let r = Rect::new(
                p.x as f32 * TILE,
                p.y as f32 * TILE,
                fw as f32 * TILE,
                fh as f32 * TILE,
            );
            let col = if busy {
                Color::new(1.0, 0.45, 0.35, 0.9)
            } else if p.user.is_some() || p.claim.is_some() {
                Color::new(0.3, 0.85, 1.0, 0.9)
            } else {
                Color::new(0.4, 1.0, 0.55, 0.7)
            };
            stroke_rrect(r.inset_px(3.0), 6.0, 2.0, col);
            let centre = r.center();
            for gid in p.seat_by.iter().flatten() {
                if let Some(guest) = w.guests().iter().find(|x| x.id == *gid) {
                    draw_line(guest.pos.0, guest.pos.1, centre.x, centre.y, 2.0, col);
                    draw_circle(guest.pos.0, guest.pos.1, 5.0, col);
                }
            }
            for sid in [p.user, p.claim].into_iter().flatten() {
                if let Some(s) = w.staff().iter().find(|s| s.id == sid) {
                    let (sx, sy) = tile_center(s.tile());
                    draw_line(
                        sx,
                        sy,
                        centre.x,
                        centre.y,
                        2.0,
                        Color::new(0.3, 0.85, 1.0, 0.9),
                    );
                }
            }
        }
    }
}

/// Last frame's opaque UI regions, outlined and numbered by layer.
pub fn draw_regions(ui: &mut Ui) {
    ui.set_camera();
    let regions: Vec<(Rect, u32)> = ui.dev_regions().to_vec();
    let st = ui
        .theme
        .small()
        .size(10.0)
        .color(Color::new(1.0, 0.85, 0.3, 1.0));
    for (r, z) in regions {
        if r.w <= 0.0 || r.h <= 0.0 {
            continue;
        }
        stroke_rrect(r, 0.0, 1.0, Color::new(1.0, 0.85, 0.3, 0.8));
        ui.fonts.draw(&format!("{z}"), r.x + 3.0, r.y + 11.0, &st);
    }
}

trait InsetPx {
    fn inset_px(self, d: f32) -> Rect;
}

impl InsetPx for Rect {
    fn inset_px(self, d: f32) -> Rect {
        Rect::new(
            self.x + d,
            self.y + d,
            (self.w - 2.0 * d).max(0.0),
            (self.h - 2.0 * d).max(0.0),
        )
    }
}
