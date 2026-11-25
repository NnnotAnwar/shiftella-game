//! Screens. Owner of `menu`: cutscene agent; the rest: integration.
//!
//! * [`boot`] — the loading screen.
//! * [`game`] — everything in a running game (world, HUD, panels, day planning, summary,
//!   pause menu, cutscenes).
//! * [`credits`] — the calm credits scroll after an ending.

pub mod boot;
pub mod credits;
pub mod game;
pub mod menu;
