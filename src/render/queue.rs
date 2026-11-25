//! Очередь отрисовки с y-сортировкой: мебель, стены и люди перемешиваются
//! правильно — кто ниже на экране (ближе к камере), тот рисуется позже.

use macroquad::prelude::*;

use super::furniture::{FurnitureState, draw_furniture, furniture_sort_key};
use super::look::{Expression, PersonLook, PersonPose};
use super::person::{draw_person_ex, draw_selection, person_sort_key};
use super::tiles::{WallKind, WallMask, draw_wall, wall_sort_key};

enum Cmd<'a> {
    Person(&'a PersonLook, PersonPose, Expression),
    Furniture(&'a str, Vec2, (u8, u8), FurnitureState),
    Wall(Vec2, WallMask, WallKind),
    Selection(Vec2),
    Custom(Box<dyn FnOnce() + 'a>),
}

/// Накопить команды за кадр, затем `flush()` — отсортировать по ключу и нарисовать.
/// При равных ключах сохраняется порядок добавления.
#[derive(Default)]
pub struct RenderQueue<'a> {
    items: Vec<(f32, u32, Cmd<'a>)>,
    seq: u32,
}

impl<'a> RenderQueue<'a> {
    pub fn new() -> Self {
        Self {
            items: Vec::with_capacity(256),
            seq: 0,
        }
    }

    fn push(&mut self, key: f32, cmd: Cmd<'a>) {
        self.items.push((key, self.seq, cmd));
        self.seq += 1;
    }

    /// Персонаж (ключ — `person_sort_key`).
    pub fn push_person(&mut self, look: &'a PersonLook, pose: PersonPose) {
        self.push_person_ex(look, pose, Expression::Neutral);
    }

    pub fn push_person_ex(&mut self, look: &'a PersonLook, pose: PersonPose, expr: Expression) {
        self.push(person_sort_key(&pose), Cmd::Person(look, pose, expr));
    }

    /// Кольцо выделения под персонажем.
    pub fn push_selection(&mut self, pose: &PersonPose) {
        self.push(person_sort_key(pose) - 0.01, Cmd::Selection(pose.feet));
    }

    /// Предмет мебели (ключ — `furniture_sort_key`).
    pub fn push_furniture(
        &mut self,
        sprite: &'a str,
        tile_origin: Vec2,
        size: (u8, u8),
        state: FurnitureState,
    ) {
        let key = furniture_sort_key(sprite, tile_origin, size);
        self.push(key, Cmd::Furniture(sprite, tile_origin, size, state));
    }

    /// Стена (обычно только низкие фасадные — высокие лучше держать в `StaticLayer`).
    pub fn push_wall(&mut self, tile: Vec2, mask: WallMask, kind: WallKind) {
        self.push(wall_sort_key(tile), Cmd::Wall(tile, mask, kind));
    }

    /// Любая отрисовка со своим ключом (мировой y «ног» объекта).
    pub fn push_custom(&mut self, key: f32, f: impl FnOnce() + 'a) {
        self.push(key, Cmd::Custom(Box::new(f)));
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Порядок индексов (в порядке добавления), в котором будут нарисованы команды.
    pub fn order(&self) -> Vec<u32> {
        let mut keys: Vec<(f32, u32)> = self.items.iter().map(|(k, s, _)| (*k, *s)).collect();
        keys.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        keys.into_iter().map(|(_, s)| s).collect()
    }

    /// Отсортировать и нарисовать всё, очередь опустеет.
    pub fn flush(&mut self) {
        self.items
            .sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        for (_, _, cmd) in self.items.drain(..) {
            match cmd {
                Cmd::Person(look, pose, expr) => draw_person_ex(look, &pose, expr),
                Cmd::Furniture(sprite, o, size, st) => draw_furniture(sprite, o, size, &st),
                Cmd::Wall(tile, mask, kind) => draw_wall(tile, mask, kind),
                Cmd::Selection(feet) => draw_selection(feet),
                Cmd::Custom(f) => f(),
            }
        }
        self.seq = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::look::{Facing, Pose};
    use std::cell::RefCell;

    #[test]
    fn custom_commands_draw_in_y_order() {
        let log = RefCell::new(Vec::new());
        let mut q = RenderQueue::new();
        for (name, y) in [("c", 300.0), ("a", 100.0), ("b", 200.0), ("a2", 100.0)] {
            let log = &log;
            q.push_custom(y, move || log.borrow_mut().push(name));
        }
        q.flush();
        assert_eq!(
            *log.borrow(),
            vec!["a", "a2", "b", "c"],
            "stable for equal keys"
        );
        assert!(q.is_empty());
    }

    #[test]
    fn person_behind_table_draws_first() {
        let look = PersonLook::random(1, false);
        let mut q = RenderQueue::new();
        // стол в строке 3 (низ следа = 4 * 64), человек стоит выше (в строке 2)
        q.push_furniture(
            "table_round_2",
            vec2(0.0, 192.0),
            (1, 1),
            FurnitureState::default(),
        );
        q.push_person(
            &look,
            PersonPose {
                feet: vec2(32.0, 170.0),
                facing: Facing::Down,
                pose: Pose::Idle,
                t: 0.0,
            },
        );
        // ковёр под всем, добавлен последним
        q.push_furniture("rug", vec2(0.0, 0.0), (3, 2), FurnitureState::default());
        assert_eq!(q.order(), vec![2, 1, 0]);
    }
}
