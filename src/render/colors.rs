//! Палитра мира «violet dollhouse» (см. `docs/GAME-BRIEF.md` §2).
//! Базовые цвета материалов — ДО освещения: ночная темнота и лампы
//! умножаются поверх, поэтому здесь цвета светлее и сочнее, чем на экране.

use macroquad::prelude::Color;

// Акценты Shiftella
pub const INDIGO: Color = Color::from_hex(0x6366f1);
pub const INDIGO_600: Color = Color::from_hex(0x4f46e5);
pub const INDIGO_GLOW: Color = Color::from_hex(0x818cf8);
pub const INDIGO_SOFT: Color = Color::from_hex(0xa5b4fc);
pub const VIOLET: Color = Color::from_hex(0x7c3aed);
pub const VIOLET_SOFT: Color = Color::from_hex(0xa78bfa);
pub const INK: Color = Color::from_hex(0x1b1a2e);
pub const AMBER: Color = Color::from_hex(0xffc46b);
pub const AMBER_DEEP: Color = Color::from_hex(0xf99c4e);
pub const EMERALD: Color = Color::from_hex(0x10b981);
pub const ROSE: Color = Color::from_hex(0xf0607a);
pub const LAVENDER_RIM: Color = Color::from_hex(0xc9c2ff);

// Ночное небо и улица
pub const NIGHT_0: Color = Color::from_hex(0x0d0c17);
pub const NIGHT_1: Color = Color::from_hex(0x121022);
pub const NIGHT_2: Color = Color::from_hex(0x1a1730);
pub const VIOLET_NIGHT_0: Color = Color::from_hex(0x120c1f);
pub const VIOLET_NIGHT_1: Color = Color::from_hex(0x201634);

// Стены (вид 3/4)
pub const WALL_PLASTER_TOP: Color = Color::from_hex(0x8a78b8);
pub const WALL_PLASTER_BOTTOM: Color = Color::from_hex(0x6a5a99);
pub const WAINSCOT_TOP: Color = Color::from_hex(0x5a3f6e);
pub const WAINSCOT_BOTTOM: Color = Color::from_hex(0x3f2b52);
pub const WALL_TRIM: Color = Color::from_hex(0xb7a6e6);
pub const WALL_CAP: Color = Color::from_hex(0x2a2140);
pub const WALL_CAP_EDGE: Color = Color::from_hex(0x4c3f72);

// Дерево мебели
pub const WOOD_LIGHT: Color = Color::from_hex(0xc08a5e);
pub const WOOD: Color = Color::from_hex(0x9c6a47);
pub const WOOD_DARK: Color = Color::from_hex(0x6e4532);
pub const WOOD_PLUM: Color = Color::from_hex(0x6b4560);

// Металл техники
pub const STEEL_LIGHT: Color = Color::from_hex(0xc9cfe2);
pub const STEEL: Color = Color::from_hex(0x8e96ad);
pub const STEEL_DARK: Color = Color::from_hex(0x4a5068);
pub const GRAPHITE: Color = Color::from_hex(0x2c2f3e);

// Ткань
pub const FABRIC_PLUM: Color = Color::from_hex(0x7b4f8e);
pub const FABRIC_TEAL: Color = Color::from_hex(0x3f8a8c);
pub const FABRIC_ROSE: Color = Color::from_hex(0xb86b7f);
pub const CREAM: Color = Color::from_hex(0xf3e6d0);

// Растения
pub const LEAF: Color = Color::from_hex(0x4f9a6b);
pub const LEAF_DARK: Color = Color::from_hex(0x2f6a4f);
pub const POT: Color = Color::from_hex(0xc97b5a);
