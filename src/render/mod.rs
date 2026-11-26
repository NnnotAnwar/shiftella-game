//! Drawing the "violet dollhouse" world: people, furniture, floors/walls, the street,
//! night light and atmosphere. Module owner: the render agent.
//!
//! Floors and rugs are seen from above; furniture, walls and people in a gentle 3/4 view
//! (front face and top visible, with height, y-sorted). No black outlines:
//! volume comes from gradients, lavender rim light from the windows and warm lamp pools.
//!
//! # Coordinates
//! World pixels, tile = [`crate::map::TILE`] (64). Oblique projection:
//! a floor point (x, y) at height z is drawn at (x, y − z). An object's sort key
//! is the y of its "feet" / the bottom edge of its footprint on the floor.
//! - `draw_floor` / `draw_wall` take **tile coordinates** (column, row) as a `Vec2`.
//! - `draw_furniture` takes the **world pixels** of the footprint's top-left corner.
//! - `PersonPose::feet` — world pixels of the point between the feet.
//!
//! # Public API
//! - People: [`PersonLook`] (+ `random(seed, staff)`, `preset(id)`), [`Facing`], [`Pose`],
//!   [`PersonPose`], [`draw_person`] / [`draw_person_ex`] (with an [`Expression`]),
//!   [`draw_emote`] / [`draw_emote_ex`] ([`Emote`]), [`emote_anchor`], [`draw_selection`],
//!   [`draw_actor_highlight`], [`draw_laptop`], [`draw_shiftella_logo`], [`sit_anchor`], [`person_sort_key`], [`draw_cup`], [`draw_steam`].
//! - Dialogue portraits: [`draw_portrait`]`(look, expression, rect, t)` — in the current
//!   (UI) camera, transparent background. Presets: [`PersonLook::PRESETS`].
//! - Furniture: [`draw_furniture`]`(id, tile_origin_px, (w,h), &FurnitureState)` for every
//!   [`FURNITURE_IDS`]; an unknown id → a placeholder. [`furniture_sort_key`],
//!   [`furniture_lights`] (lamp / screen lights to add to `Lighting`).
//!   `FurnitureState { active, dirty, t, facing }` (`..Default::default()`); for chairs
//!   `facing` is where the sitter looks, for tables `active` = a candle is lit.
//! - Floors / walls: [`draw_floor`]`("floor_wood"|"floor_tile"|"floor_carpet"|"pavement", tile)`,
//!   [`draw_wall`]`(tile, WallMask::compute(..), WallKind)`, [`window_glass_rect`],
//!   [`StaticLayer`] — a render-target cache of the static parts.
//! - Street: [`draw_outside`]`(rect, t)`, [`draw_train`], [`outside_lights`],
//!   [`draw_holo_sign`], [`Weather`] (`draw_rain`, `draw_window_rain`).
//! - Depth: [`RenderQueue`] — `push_*` with a key, then `flush()`.
//! - Light: [`Lighting`] — `begin(&cam, ambient)`, `add_light(Light)`, `add_window_light`,
//!   `train_sweep(progress)`, `apply()`; [`ambient_for`]`(night_progress)`, [`ambient_blackout`].
//!
//! # Frame order
//! ```ignore
//! // once (and when the map changes): cache of the floor and the tall walls
//! statics.build(world_rect, 2.0, || { draw_floor(..); draw_wall(..); });
//! lighting.set_windows(glass_rects);
//! // every frame
//! let cam = world_camera.to_camera2d();
//! set_camera(&cam);
//! clear_background(colors::NIGHT_0);
//! draw_outside(street_rect, t);  draw_train(street_rect, p, t);  weather.draw_rain(street_rect, t);
//! statics.draw();
//! for g in glass_rects { weather.draw_window_rain(g, t) }
//! let mut q = RenderQueue::new();  // furniture, low walls, people, selection rings
//! q.flush();
//! lighting.begin(&cam, ambient_for(night_progress));
//! lighting.add_window_light(1.0); lighting.train_sweep(p);
//! for l in furniture_lights(..) / outside_lights(..) { lighting.add_light(l) }
//! lighting.apply();              // `cam` is active again afterwards
//! draw_emote_ex(..);             // emotes on top, not darkened
//! ```

#![allow(clippy::too_many_arguments)]

mod brand;
pub mod colors;
pub mod draw;
pub mod faces;
mod furniture;
mod light;
mod look;
mod outside;
mod person;
mod queue;
mod tiles;

pub use brand::{draw_brand_mark, init as init_brand};
pub use furniture::{
    COUNTER_H, FURNITURE_IDS, FurnitureState, draw_furniture, furniture_lights, furniture_sort_key,
    holo_flicker,
};
pub use light::{Light, Lighting, ambient_blackout, ambient_for};
pub use look::{
    Accessory, Emote, Expression, FacialHair, Facing, HairStyle, PersonLook, PersonPose, Pose,
    Tone, TopStyle,
};
pub use outside::{Weather, draw_holo_sign, draw_outside, draw_train, outside_lights, world_text};
pub use person::{
    draw_actor_highlight, draw_cup, draw_emote, draw_emote_ex, draw_laptop, draw_person,
    draw_person_ex, draw_portrait, draw_selection, draw_shiftella_logo, draw_steam, emote_anchor,
    person_sort_key, sit_anchor,
};
pub use queue::RenderQueue;
pub use tiles::{
    LOW_WALL_H, StaticLayer, WALL_H, WallKind, WallMask, draw_floor, draw_wall, tile_px,
    wall_sort_key, window_glass_rect,
};
