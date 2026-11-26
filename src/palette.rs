//! All game colours in one place. When the designer's style guide is ready,
//! change the values here and the whole game is recoloured.

use macroquad::prelude::Color;

pub const BACKGROUND: Color = Color::from_hex(0x0c0b16);
pub const PANEL: Color = Color::new(0.09, 0.085, 0.16, 0.9);
pub const PANEL_BORDER: Color = Color::from_hex(0x2e2a4f);
pub const BUTTON: Color = Color::from_hex(0x2a2648);
pub const TEXT: Color = Color::from_hex(0xece8f7);
pub const TEXT_DIM: Color = Color::from_hex(0x9690b5);
pub const NEON: Color = Color::from_hex(0xff4fa3);
pub const WARM: Color = Color::from_hex(0xffc46b);

// World
pub const FLOOR_A: Color = Color::from_hex(0x4a3a33);
pub const FLOOR_B: Color = Color::from_hex(0x45362f);
pub const FLOOR_LINE: Color = Color::new(0.0, 0.0, 0.0, 0.12);
pub const WALL: Color = Color::from_hex(0x1c1a2b);
pub const WALL_TOP: Color = Color::from_hex(0x2c2942);
pub const WINDOW: Color = Color::from_hex(0x3b6f9e);
pub const DOOR: Color = Color::from_hex(0x7a4e33);
pub const COUNTER: Color = Color::from_hex(0x5e4032);
pub const COUNTER_TOP: Color = Color::from_hex(0x7b5845);
pub const COFFEE_MACHINE: Color = Color::from_hex(0xb9b2a6);
pub const STOVE: Color = Color::from_hex(0x6d6c78);
pub const SINK: Color = Color::from_hex(0x9fb4bf);
pub const SINK_DARK: Color = Color::from_hex(0x5f7a88);
pub const TABLE: Color = Color::from_hex(0x8a6243);
pub const TABLE_EDGE: Color = Color::from_hex(0x6e4c33);
pub const CHAIR: Color = Color::from_hex(0x3d2b22);
