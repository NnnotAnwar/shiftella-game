//! # Shiftella UI kit (immediate mode)
//!
//! Purple/indigo frosted-glass UI in **UI units** = logical points ÷ `ui_scale`.
//! Text is rasterised at physical pixels, all shapes have a 1-px AA fringe, so it is
//! crisp on Retina. Owner: ui-kit agent. See `examples/ui_demo.rs` for every widget.
//!
//! ## Frame order (with the world camera)
//! ```ignore
//! let mut ui = Ui::new(Fonts::load().await?);
//! ui.set_scale(settings.ui_scale);                 // 0.9–1.3
//! loop {
//!     ui.begin_frame();                            // 1. read input (UI units)
//!     if !ui.wants_mouse() { camera.update(); }    // 2. world input (uses last frame's UI rects)
//!     set_camera(&world_cam); draw_world();        // 3. world
//!     ui.set_camera();                             // 4. UI camera (y-down, positive zoom.y)
//!     hud(&mut ui); screens(&mut ui);              // 5. widgets
//!     for ev in ui.draw_toasts(ToastAnchor::BottomRight) { /* toast actions */ }
//!     ui.end_frame();                              // 6. popups, tooltips, cursor
//!     if ui.key(KeyCode::Space) { game.toggle_pause() }  // 7. hotkeys AFTER widgets
//!     if ui.undo() { undo_stack.pop() }
//!     for s in ui.drain_sounds() { audio.play_ui(s) }    // UiSound → Sfx::UiClick…
//!     next_frame().await;
//! }
//! ```
//! `ui.key(k)` ignores keys consumed by widgets this frame (dialogue Space, Enter, Esc
//! closing a popup, 1–4 in choices) and everything while a text field has focus.
//!
//! ## API summary
//! * **Text** — [`Fonts`] (`load`, `draw`, `draw_in`, `measure`, `wrap`, `ellipsize`,
//!   `draw_wrapped`, `draw_lines` for typewriter), [`TextStyle`] (size pt, [`Weight`],
//!   colour, `.tabular()` fixed-advance digits, `.tracking()`), [`Align`]. Presets on
//!   [`Theme`]: `display clock title h1 h2 body body_medium strong small small_medium micro key number`.
//! * **Tokens** — [`Theme::SHIFTELLA`] colours, [`theme::space`], [`theme::radius`],
//!   [`theme::size`], shadows, [`Tone`].
//! * **Painting** — [`draw`]: `fill_rrect`, `stroke_rrect`, `shadow_rrect`, `glow_circle`,
//!   `line`, `arc`, `hline`, [`Paint`] gradients, `push_alpha`/`pop_alpha`.
//!   Glass: [`draw_glass`] / [`Ui::glass`] with [`Glass`] presets (`panel hud card tooltip`).
//! * **Context** — [`Ui`]: `begin_frame end_frame set_camera screen mouse wants_mouse
//!   wants_keyboard id push_id pop_id region push_clip pop_clip interact hovering tooltip
//!   key take_key esc enter undo redo consume_key drain_sounds set_scale`.
//! * **Widgets** — `button(Button)`, `icon_button(IconButton)`, `keycap`, `badge`, `toggle`,
//!   `toggle_row`, `checkbox`, `check_item`, `slider`, `segmented`, `tabs(Tab)`,
//!   `list_row(ListRow)`, `begin_scroll/end_scroll`, `progress`, `need_bar(Need)`, `chip`,
//!   `avatar`, `section`, `divider`, `label`, `text_input`, `dropdown`, `panel_header`.
//! * **Toasts** — `push_toast(Toast)`, `draw_toasts(ToastAnchor) -> Vec<ToastEvent>`,
//!   `dismiss_toast`, `set_toast_bottom_inset`.
//! * **Shiftella device** — `phone_frame(key, rect, &PhoneFrame) -> PhoneOut { content, … }`.
//! * **Cutscenes** — [`DialogueBox`] (`dialogue_box`, `narration_box`), `choice_list`,
//!   `letterbox`, `fade`.
//! * **Layout** — [`layout`]: `anchored(Anchor)`, [`RectExt`] (`inset split_left split_top
//!   centered …`), `row column weighted_row grid grid_cell`, [`Flow`] cursor.
//! * **Icons** — [`Icon`] (78 glyphs, `Icon::ALL`), [`draw_icon`].
//! * **Hands-on widgets** — [`handson`]: order card, ticket rail, moment card, duties board,
//!   record picker, recipe book, atmosphere meter, rozpis puzzle, payroll, journal, coach marks.
//!
//! ## Example
//! ```ignore
//! let bar = anchored(ui.screen(), Anchor::TopLeft, vec2(340.0, 64.0), 12.0);
//! ui.glass(bar, &Glass::hud());
//! ui.fonts.draw("03:33", bar.x + 16.0, bar.y + 40.0, &ui.theme.clock());
//! let mut f = Flow::row_rev(bar.inset(18.0).with_h(28.0), 2.0);
//! if ui.icon_button("pause", f.next(28.0), Icon::Pause, &IconButton::ghost().tooltip("Pauza · Space")).clicked {
//!     game.toggle_pause();
//! }
//! if ui.button("start", r, &Button::new("Začít noc").primary().icon(Icon::Play).shortcut("Enter")).clicked { /* … */ }
//! ```

pub mod context;
pub mod dialogue;
pub mod draw;
pub mod glass;
pub mod handson;
pub mod icons;
pub mod id;
pub mod layout;
pub mod phone;
pub mod text;
pub mod theme;
pub mod toast;
pub mod widgets;

pub use context::{Response, Ui, UiSound, approach, ease, ease_out};
pub use dialogue::{DialogueBox, DialogueLabels, DialogueLine, DialogueResponse, HistoryLine};
pub use draw::{Paint, Shadow};
pub use glass::{Elevation, Glass, draw_glass};
pub use icons::{Icon, draw_icon};
pub use id::Id;
pub use layout::{Anchor, Flow, RectExt, anchored, column, grid, grid_cell, row, weighted_row};
pub use phone::{Device, PhoneFrame, PhoneOut};
pub use text::{Align, Fonts, TextStyle, Weight};
pub use theme::{Theme, Tone};
pub use toast::{Toast, ToastAnchor, ToastEvent, ToastId, ToastKind};
pub use widgets::{
    Button, ButtonKind, IconButton, Leading, ListRow, Need, ScrollArea, Tab, TextResponse,
    avatar_color,
};
