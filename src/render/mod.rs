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
//!   `train_sweep(progress, street)`, `apply()`; [`ambient_for`]`(night_progress)`, [`ambient_blackout`].
//!   `lighting.coziness` (0..1, 0.5 neutral) grades warmth / shadow softness / bloom
//!   ([`CozyGrade`]); `lighting.draw_dust_motes(t)` after `apply`.
//! - Seasons (`handson.md` §11.1): [`Season`] + a blendable [`SeasonLook`]
//!   ([`Season::blend`]); [`draw_outside_season`], [`draw_sign_season`], [`Snowfall`],
//!   [`draw_falling_leaves`], [`draw_window_season`], [`draw_floor_season`] /
//!   [`draw_doorstep_season`] (static layer), [`draw_breath`] (people outside).
//! - Festive: [`Garlands`] (`draw(progress)` / `lights` / `draw_glow` / `hang_spots`),
//!   [`draw_advent_wreath`], furniture `spruce_tree`, `garland_box`, `advent_wreath`, `tip_jar`.
//! - Decor themes (§11.2): [`DecorTheme`] → [`ThemeStyle`]; [`set_decor`] / [`with_decor`]
//!   before building the static layer and drawing furniture; cache key [`static_key`] +
//!   [`StaticLayer::build_keyed`].
//! - Hands-on (§1–3): [`TipCoin`] + [`draw_tip_coin`] / [`draw_tip_float`] / [`draw_tip_jar`],
//!   [`draw_wish_bubble`] ([`TasteIcon`]), [`draw_order_mark`] ([`OrderMark`]),
//!   [`draw_queue_chips`] ([`QueueIcon`], [`queue_chip_rects`]), [`draw_cup_glyph`]
//!   ([`CupGlyph`], [`LatteArt`]), [`draw_served_cup`]; emotes `Sparkle`, `Smile`,
//!   `WarmHeart`, `Sleepy`, `LikedRecord`.
//! - Moments (§5): [`MomentMarker`] ([`MomentMarkerKind`]) + [`draw_moment_marker`] /
//!   [`moment_marker_light`]; blackout candles [`draw_candle_lighting`] at
//!   [`table_candle_anchor`].
//! - Polaroid (§11.4): [`SceneSnap`] → [`SnapRenderer::render`] (deterministic) →
//!   [`draw_polaroid`] ([`PolaroidStyle`], [`polaroid_tilt`]).
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
//! draw_outside(street_rect, t);  draw_outside_season(street_rect, &look, t);
//! draw_train(street_rect, p, t);
//! draw_holo_sign(pos, name, t);  draw_sign_season(pos, name, &look, t);
//! weather.draw_rain(street_rect, t);  Snowfall::from_look(&look).draw(street_rect, t);
//! draw_falling_leaves(street_rect, look.leaves, wind, t);
//! statics.draw();
//! for g in glass_rects { weather.draw_window_rain(g, t); draw_window_season(g, &look, t) }
//! window_garlands.draw(progress, t);
//! let mut q = RenderQueue::new();  // furniture, low walls, people, selection rings,
//! q.flush();                       // served cups, counter garlands, candles being lit
//! lighting.coziness = coziness;
//! lighting.begin(&cam, ambient_for(night_progress));
//! lighting.add_window_light(1.0); lighting.train_sweep(p, street);
//! for l in furniture_lights(..) / outside_lights(..) / garlands.lights(..) { lighting.add_light(l) }
//! lighting.add_light(moment_marker_light(&m, now));
//! lighting.apply();              // `cam` is active again afterwards
//! lighting.draw_dust_motes(t);  garlands.draw_glow(progress, t);
//! draw_breath(..);  draw_tip_coin(..);  draw_moment_marker(..);  draw_wish_bubble(..);
//! draw_order_mark(..);  draw_queue_chips(..);
//! draw_emote_ex(..);             // emotes on top, not darkened
//! ```

#![allow(clippy::too_many_arguments)]

mod brand;
pub mod cat;
pub mod colors;
pub mod draw;
mod equipment;
pub mod faces;
mod festive;
mod furniture;
mod handson;
mod light;
mod look;
mod moments;
mod outside;
mod person;
mod polaroid;
mod queue;
mod season;
mod theme;
mod tiles;

pub use brand::{draw_brand_mark, init as init_brand};
pub use equipment::{
    ApplianceKind, ApplianceLook, CakeLook, EquipGlyph, PotLook, appliance_sort_key,
    draw_appliance_state, draw_equip_cue, draw_work_ring,
};
pub use festive::{
    GarlandSpan, GarlandStyle, Garlands, advent_wreath_lights, bow, draw_advent_wreath,
    draw_tip_jar,
};
pub use furniture::{
    COUNTER_H, FURNITURE_IDS, FurnitureState, draw_furniture, furniture_lights, furniture_sort_key,
    holo_flicker,
};
pub use handson::{
    CupGlyph, LatteArt, OrderMark, QUEUE_CHIP, QueueIcon, TIP_COLLECT_SECS, TIP_SPAWN_SECS,
    TasteIcon, TipCoin, draw_cup_glyph, draw_latte_art, draw_order_mark, draw_queue_chips,
    draw_queue_icon, draw_rhythm_glow, draw_served_cup, draw_taste_icon, draw_tip_coin,
    draw_tip_float, draw_wish_bubble, heart, icon_scale_for_zoom, queue_chip_rects,
    set_world_icon_scale, sparkle, tip_coin_light, world_icon_scale,
};
pub use light::{CozyGrade, Light, Lighting, ambient_blackout, ambient_for};
pub use look::{
    Accessory, Emote, Expression, FacialHair, Facing, HairStyle, PersonLook, PersonPose, Pose,
    Tone, TopStyle,
};
pub use moments::{
    CANDLE_LIGHT_SECS, MARKER_LIFT, MARKER_PULSE, MomentMarker, MomentMarkerKind,
    candle_lighting_light, draw_candle_lighting, draw_moment_icon, draw_moment_marker,
    marker_radius, moment_marker_light, table_candle_anchor,
};
pub use outside::{
    Weather, draw_holo_sign, draw_outside, draw_train, outside_lights, world_text, world_text_rot,
    world_text_width,
};
pub use person::{
    draw_actor_highlight, draw_cup, draw_emote, draw_emote_ex, draw_laptop, draw_person,
    draw_person_ex, draw_portrait, draw_selection, draw_shiftella_logo, draw_steam, emote_anchor,
    person_sort_key, sit_anchor,
};
pub use polaroid::{
    PolaroidLayout, PolaroidStyle, SceneSnap, SnapActor, SnapCup, SnapFloor, SnapProp,
    SnapRenderer, SnapTile, draw_polaroid, polaroid_layout, polaroid_tilt,
};
pub use queue::RenderQueue;
pub use season::{
    Season, SeasonLook, Snowfall, breath_anchor, draw_breath, draw_doorstep_season,
    draw_falling_leaves, draw_floor_season, draw_outside_season, draw_sign_season,
    draw_window_season, star,
};
pub use theme::{
    CurtainStyle, DecorTheme, FloorVariant, LampStyle, ThemeStyle, WallPattern, decor, parse_hex,
    set_decor, with_decor,
};
pub use tiles::{
    LOW_WALL_H, StaticLayer, WALL_H, WallKind, WallMask, draw_floor, draw_wall, static_key,
    tile_px, wall_sort_key, window_glass_rect,
};
