//! Рисование мира «violet dollhouse»: персонажи, мебель, пол/стены, улица,
//! ночной свет и атмосфера. Владелец модуля: агент render.
//!
//! Пол и ковры — вид сверху; мебель, стены и люди — мягкий вид 3/4
//! (видна передняя грань и верх, есть высота, y-сортировка). Без чёрных контуров:
//! объём — градиентами, лавандовым rim light из окон и тёплыми пятнами ламп.
//!
//! # Координаты
//! Мировые пиксели, тайл = [`crate::map::TILE`] (64). Проекция косая:
//! точка пола (x, y) на высоте z рисуется в (x, y − z). Ключ сортировки
//! объекта — y его «ног» / нижнего края следа на полу.
//! - `draw_floor` / `draw_wall` принимают **координаты тайла** (столбец, строка) как `Vec2`.
//! - `draw_furniture` принимает **мировые пиксели** левого верхнего угла следа.
//! - `PersonPose::feet` — мировые пиксели точки между ступнями.
//!
//! # Публичное API
//! - Люди: [`PersonLook`] (+ `random(seed, staff)`, `preset(id)`), [`Facing`], [`Pose`],
//!   [`PersonPose`], [`draw_person`] / [`draw_person_ex`] (с [`Expression`]),
//!   [`draw_emote`] / [`draw_emote_ex`] ([`Emote`]), [`emote_anchor`], [`draw_selection`],
//!   [`draw_actor_highlight`], [`draw_laptop`], [`draw_shiftella_logo`], [`sit_anchor`], [`person_sort_key`], [`draw_cup`], [`draw_steam`].
//! - Портреты для диалогов: [`draw_portrait`]`(look, expression, rect, t)` — в текущей
//!   (UI) камере, фон прозрачный. Пресеты: [`PersonLook::PRESETS`].
//! - Мебель: [`draw_furniture`]`(id, tile_origin_px, (w,h), &FurnitureState)` для всех
//!   [`FURNITURE_IDS`]; неизвестный id → заглушка. [`furniture_sort_key`],
//!   [`furniture_lights`] (свет ламп/экранов, добавлять в `Lighting`).
//!   `FurnitureState { active, dirty, t, facing }` (`..Default::default()`); у стульев
//!   `facing` — куда смотрит сидящий, у столов `active` — горит свеча.
//! - Пол/стены: [`draw_floor`]`("floor_wood"|"floor_tile"|"floor_carpet"|"pavement", tile)`,
//!   [`draw_wall`]`(tile, WallMask::compute(..), WallKind)`, [`window_glass_rect`],
//!   [`StaticLayer`] — кэш статики в render target.
//! - Улица: [`draw_outside`]`(rect, t)`, [`draw_train`], [`outside_lights`],
//!   [`draw_holo_sign`], [`Weather`] (`draw_rain`, `draw_window_rain`).
//! - Глубина: [`RenderQueue`] — `push_*` с ключом, затем `flush()`.
//! - Свет: [`Lighting`] — `begin(&cam, ambient)`, `add_light(Light)`, `add_window_light`,
//!   `train_sweep(progress)`, `apply()`; [`ambient_for`]`(night_progress)`, [`ambient_blackout`].
//!
//! # Порядок кадра
//! ```ignore
//! // один раз (и при изменении карты): кэш пола и высоких стен
//! statics.build(world_rect, 2.0, || { draw_floor(..); draw_wall(..); });
//! lighting.set_windows(glass_rects);
//! // каждый кадр
//! let cam = world_camera.to_camera2d();
//! set_camera(&cam);
//! clear_background(colors::NIGHT_0);
//! draw_outside(street_rect, t);  draw_train(street_rect, p, t);  weather.draw_rain(street_rect, t);
//! statics.draw();
//! for g in glass_rects { weather.draw_window_rain(g, t) }
//! let mut q = RenderQueue::new();  // мебель, низкие стены, люди, кольца выделения
//! q.flush();
//! lighting.begin(&cam, ambient_for(night_progress));
//! lighting.add_window_light(1.0); lighting.train_sweep(p);
//! for l in furniture_lights(..) / outside_lights(..) { lighting.add_light(l) }
//! lighting.apply();              // после него снова активна `cam`
//! draw_emote_ex(..);             // эмоции поверх, не затемняются
//! ```

#![allow(clippy::too_many_arguments)]

pub mod colors;
pub mod draw;
mod furniture;
mod light;
mod look;
mod outside;
mod person;
mod queue;
mod tiles;

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
