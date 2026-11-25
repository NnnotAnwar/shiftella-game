//! Карта кафе: сетка тайлов, вид сверху.
//!
//! Карта задаётся текстом — каждый символ это тайл (см. `Tile::from_char`).
//! Так её легко менять руками, а позже — грузить из файла.
//! Координаты мира — в «мировых пикселях», один тайл = `TILE`.

use macroquad::prelude::*;

use crate::palette as pal;
use crate::shapes::{object, rounded_rect, soft_light};

/// Размер тайла в мировых пикселях (под текстуры 64×64, на Retina — 128×128).
pub const TILE: f32 = 64.0;

pub const CAFE_LAYOUT: &str = "\
####WWWW######WWWW######WWWW##
#............................#
#..hTh.....hTh.....hTh.......#
#............................#
#............................#
#..hTh.....hTh.....hTh.......#
#............................#
#............................#
#..hTh.....hTh...............#
#............................#
#...............CCCCCCCCCCC..#
#............................#
#............................#
#...............MMSSKK.......#
#............................#
#DD###########################";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    Floor,
    Wall,
    Window,
    Door,
    Counter,
    CoffeeMachine,
    Stove,
    Sink,
    Table,
    Chair,
}

impl Tile {
    pub fn from_char(c: char) -> Option<Tile> {
        match c {
            '.' => Some(Tile::Floor),
            '#' => Some(Tile::Wall),
            'W' => Some(Tile::Window),
            'D' => Some(Tile::Door),
            'C' => Some(Tile::Counter),
            'M' => Some(Tile::CoffeeMachine),
            'S' => Some(Tile::Stove),
            'K' => Some(Tile::Sink),
            'T' => Some(Tile::Table),
            'h' => Some(Tile::Chair),
            _ => None,
        }
    }

    /// Можно ли по этому тайлу ходить. Пригодится для поиска пути.
    pub fn walkable(self) -> bool {
        matches!(self, Tile::Floor | Tile::Door | Tile::Chair)
    }

    pub fn name(self) -> &'static str {
        match self {
            Tile::Floor => "Floor",
            Tile::Wall => "Wall",
            Tile::Window => "Window",
            Tile::Door => "Door",
            Tile::Counter => "Counter",
            Tile::CoffeeMachine => "Coffee machine",
            Tile::Stove => "Stove",
            Tile::Sink => "Sink",
            Tile::Table => "Table",
            Tile::Chair => "Chair",
        }
    }
}

pub struct Map {
    pub width: usize,
    pub height: usize,
    tiles: Vec<Tile>,
}

impl Map {
    /// Строит карту из текста. Падает, если строки разной длины
    /// или встретился неизвестный символ — лучше узнать сразу.
    pub fn from_layout(layout: &str) -> Map {
        let rows: Vec<&str> = layout.lines().collect();
        let height = rows.len();
        let width = rows[0].chars().count();
        let mut tiles = Vec::with_capacity(width * height);

        for (y, row) in rows.iter().enumerate() {
            assert_eq!(row.chars().count(), width, "row {y} has a different width");
            for c in row.chars() {
                let tile = Tile::from_char(c).unwrap_or_else(|| panic!("unknown tile '{c}'"));
                tiles.push(tile);
            }
        }

        Map {
            width,
            height,
            tiles,
        }
    }

    /// A map filled by `tile(x, y)` (e.g. a walkability copy of the live sim grid).
    pub fn from_fn(width: usize, height: usize, mut tile: impl FnMut(usize, usize) -> Tile) -> Map {
        let mut tiles = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                tiles.push(tile(x, y));
            }
        }
        Map {
            width,
            height,
            tiles,
        }
    }

    pub fn get(&self, x: usize, y: usize) -> Option<Tile> {
        if x < self.width && y < self.height {
            Some(self.tiles[y * self.width + x])
        } else {
            None
        }
    }

    /// Размер карты в мировых пикселях.
    pub fn size_px(&self) -> Vec2 {
        vec2(self.width as f32 * TILE, self.height as f32 * TILE)
    }

    /// Какой тайл под точкой мира `point`.
    pub fn tile_at(&self, point: Vec2) -> Option<(usize, usize)> {
        if point.x < 0.0 || point.y < 0.0 {
            return None;
        }
        let x = (point.x / TILE) as usize;
        let y = (point.y / TILE) as usize;
        self.get(x, y).map(|_| (x, y))
    }

    /// Рисуем слоями: пол → предметы → свет. Так тени и свет
    /// ложатся поверх соседних тайлов, а не обрезаются.
    pub fn draw(&self) {
        self.for_each(|x, y, tile| draw_ground(tile, x, y));
        self.for_each(|x, y, tile| draw_object(tile, x, y));
        self.for_each(|x, y, tile| {
            if tile == Tile::Table {
                soft_light(vec2(x + TILE / 2.0, y + TILE / 2.0), TILE * 1.6, pal::WARM);
            }
        });
    }

    fn for_each(&self, mut f: impl FnMut(f32, f32, Tile)) {
        for ty in 0..self.height {
            for tx in 0..self.width {
                let tile = self.tiles[ty * self.width + tx];
                f(tx as f32 * TILE, ty as f32 * TILE, tile);
            }
        }
    }
}

/// Нижний слой: пол, стены, окна, дверь.
fn draw_ground(tile: Tile, x: f32, y: f32) {
    match tile {
        Tile::Wall => {
            draw_rectangle(x, y, TILE, TILE, pal::WALL);
            draw_rectangle(x, y, TILE, 6.0, pal::WALL_TOP);
        }
        Tile::Window => {
            draw_rectangle(x, y, TILE, TILE, pal::WALL);
            rounded_rect(
                x + 6.0,
                y + 14.0,
                TILE - 12.0,
                TILE - 28.0,
                4.0,
                pal::WINDOW,
            );
            draw_line(
                x + TILE / 2.0,
                y + 14.0,
                x + TILE / 2.0,
                y + TILE - 14.0,
                3.0,
                pal::WALL_TOP,
            );
        }
        Tile::Door => {
            draw_floor(x, y);
            rounded_rect(x + 4.0, y + 8.0, TILE - 8.0, TILE - 16.0, 6.0, pal::DOOR);
        }
        _ => draw_floor(x, y),
    }
}

/// Деревянный пол: доски по 32 мировых пикселя.
fn draw_floor(x: f32, y: f32) {
    let row = (y / TILE) as usize;
    let color = if row.is_multiple_of(2) {
        pal::FLOOR_A
    } else {
        pal::FLOOR_B
    };
    draw_rectangle(x, y, TILE, TILE, color);
    for i in 1..2 {
        let ly = y + i as f32 * 32.0;
        draw_line(x, ly, x + TILE, ly, 1.0, pal::FLOOR_LINE);
    }
}

/// Верхний слой: мебель и техника с тенями.
fn draw_object(tile: Tile, x: f32, y: f32) {
    match tile {
        Tile::Counter => {
            object(x, y + 4.0, TILE, TILE - 8.0, 2.0, pal::COUNTER);
            draw_rectangle(x, y + 8.0, TILE, 6.0, pal::COUNTER_TOP);
        }
        Tile::CoffeeMachine => {
            object(
                x + 10.0,
                y + 10.0,
                TILE - 20.0,
                TILE - 20.0,
                8.0,
                pal::COFFEE_MACHINE,
            );
            draw_circle(x + TILE / 2.0, y + TILE / 2.0, 8.0, pal::WALL);
        }
        Tile::Stove => {
            object(x + 6.0, y + 6.0, TILE - 12.0, TILE - 12.0, 6.0, pal::STOVE);
            for (dx, dy) in [(20.0, 20.0), (44.0, 20.0), (20.0, 44.0), (44.0, 44.0)] {
                draw_circle_lines(x + dx, y + dy, 7.0, 2.0, pal::WALL);
            }
        }
        Tile::Sink => {
            object(x + 8.0, y + 8.0, TILE - 16.0, TILE - 16.0, 8.0, pal::SINK);
            rounded_rect(
                x + 16.0,
                y + 16.0,
                TILE - 32.0,
                TILE - 32.0,
                6.0,
                pal::SINK_DARK,
            );
        }
        Tile::Table => {
            let c = vec2(x + TILE / 2.0, y + TILE / 2.0);
            draw_circle(c.x + 2.0, c.y + 5.0, 26.0, Color::new(0.0, 0.0, 0.0, 0.28));
            draw_circle(c.x, c.y, 26.0, pal::TABLE);
            draw_circle_lines(c.x, c.y, 26.0, 2.0, pal::TABLE_EDGE);
        }
        Tile::Chair => {
            object(
                x + 18.0,
                y + 18.0,
                TILE - 36.0,
                TILE - 36.0,
                6.0,
                pal::CHAIR,
            );
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cafe_layout_parses() {
        let map = Map::from_layout(CAFE_LAYOUT);
        assert_eq!(map.width, 30);
        assert_eq!(map.height, 16);
        assert_eq!(map.get(0, 0), Some(Tile::Wall));
        assert_eq!(map.get(1, 15), Some(Tile::Door));
        assert_eq!(map.get(99, 0), None);
    }

    #[test]
    fn walls_block_floors_dont() {
        assert!(!Tile::Wall.walkable());
        assert!(!Tile::Counter.walkable());
        assert!(Tile::Floor.walkable());
    }

    #[test]
    fn tile_at_maps_world_to_tiles() {
        let map = Map::from_layout(CAFE_LAYOUT);
        assert_eq!(map.tile_at(vec2(0.0, 0.0)), Some((0, 0)));
        assert_eq!(map.tile_at(vec2(TILE * 2.5, TILE * 1.2)), Some((2, 1)));
        assert_eq!(map.tile_at(vec2(-1.0, 0.0)), None);
        assert_eq!(map.tile_at(vec2(TILE * 100.0, 0.0)), None);
    }
}
