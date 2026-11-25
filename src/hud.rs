//! Интерфейс поверх мира: верхняя полоса, панель персонала, подсказки.
//!
//! Интерфейс рисуется в «логических точках» (как в вебе: на Retina 1 точка = 2 пикселя),
//! поэтому на любом экране он одного физического размера.
//! Пока стандартный шрифт macroquad (только латиница). TTF-шрифт
//! с кириллицей и чешскими буквами подключим позже.

use macroquad::prelude::*;

use crate::game::{Game, Speed};
use crate::map::Tile;
use crate::palette as pal;
use crate::shapes::rounded_rect;

const TOP_BAR_H: f32 = 44.0;
const SIDE_W: f32 = 232.0;
const MARGIN: f32 = 12.0;
const SPEEDS: [Speed; 4] = [Speed::Paused, Speed::Normal, Speed::Fast, Speed::Faster];

/// Размер экрана в логических точках.
fn ui_size() -> Vec2 {
    vec2(screen_width(), screen_height()) / screen_dpi_scale()
}

/// Позиция мыши в логических точках — в тех же координатах, что и интерфейс.
pub fn ui_mouse() -> Vec2 {
    Vec2::from(mouse_position()) / screen_dpi_scale()
}

/// Свободная от интерфейса область экрана (в пикселях экрана) — туда камера
/// вписывает карту при старте.
pub fn map_area() -> Rect {
    let s = screen_dpi_scale();
    let ui = ui_size();
    Rect::new(
        MARGIN * s,
        (TOP_BAR_H + MARGIN) * s,
        (ui.x - SIDE_W - MARGIN * 3.0) * s,
        (ui.y - TOP_BAR_H - MARGIN * 2.0 - 40.0) * s,
    )
}

/// Где находятся кнопки скорости. Пригодится для кликов мышью.
pub fn speed_buttons() -> [(Speed, Rect); 4] {
    let (w, h, gap) = (34.0, 26.0, 4.0);
    let start_x = ui_size().x - MARGIN - 4.0 * w - 3.0 * gap;
    let y = (TOP_BAR_H - h) / 2.0;
    std::array::from_fn(|i| {
        let x = start_x + i as f32 * (w + gap);
        (SPEEDS[i], Rect::new(x, y, w, h))
    })
}

/// Курсор над интерфейсом? Тогда подсветку тайлов показывать не нужно.
pub fn is_mouse_over_ui() -> bool {
    let m = ui_mouse();
    let ui = ui_size();
    m.y < TOP_BAR_H || m.x > ui.x - SIDE_W - MARGIN
}

pub fn draw(game: &Game, hovered: Option<(usize, usize)>) {
    let ui = ui_size();
    // from_display_rect рассчитан на render target; для экрана переворачиваем Y.
    let mut ui_camera = Camera2D::from_display_rect(Rect::new(0.0, 0.0, ui.x, ui.y));
    ui_camera.zoom.y = -ui_camera.zoom.y;
    set_camera(&ui_camera);

    draw_top_bar(game, ui);
    draw_side_panel(ui);
    draw_bottom(game, hovered, ui);
    if game.night_over {
        draw_night_over(game, ui);
    }
}

/// Текст, чёткий на Retina: растеризуем в физических пикселях,
/// а рисуем в логических точках.
fn text(s: &str, x: f32, y: f32, size: f32, color: Color) {
    let dpi = screen_dpi_scale();
    draw_text_ex(
        s,
        x,
        y,
        TextParams {
            font_size: (size * dpi) as u16,
            font_scale: 1.0 / dpi,
            color,
            ..Default::default()
        },
    );
}

fn text_width(s: &str, size: f32) -> f32 {
    let dpi = screen_dpi_scale();
    measure_text(s, None, (size * dpi) as u16, 1.0 / dpi).width
}

fn draw_top_bar(game: &Game, ui: Vec2) {
    draw_rectangle(0.0, 0.0, ui.x, TOP_BAR_H, pal::PANEL);

    text(
        &format!("NIGHT {}", game.night),
        16.0,
        28.0,
        18.0,
        pal::TEXT,
    );

    // Часы по центру + полоска прогресса ночи
    let label = game.clock.label();
    let w = text_width(&label, 22.0);
    text(&label, (ui.x - w) / 2.0, 26.0, 22.0, pal::WARM);

    let bar_w = 180.0;
    let bar_x = (ui.x - bar_w) / 2.0;
    rounded_rect(bar_x, 33.0, bar_w, 4.0, 2.0, pal::PANEL_BORDER);
    if game.clock.progress() > 0.0 {
        rounded_rect(
            bar_x,
            33.0,
            bar_w * game.clock.progress(),
            4.0,
            2.0,
            pal::WARM,
        );
    }

    // Кнопки скорости (клик мышью — твоё первое задание)
    for (speed, r) in speed_buttons() {
        let active = game.speed == speed;
        let bg = if active { pal::NEON } else { pal::BUTTON };
        let fg = if active {
            pal::BACKGROUND
        } else {
            pal::TEXT_DIM
        };
        rounded_rect(r.x, r.y, r.w, r.h, 6.0, bg);
        draw_speed_icon(speed, r.center(), fg);
    }
}

/// Иконка скорости с центром в `c`: «||» или 1–3 треугольника.
fn draw_speed_icon(speed: Speed, c: Vec2, color: Color) {
    let count = match speed {
        Speed::Paused => {
            rounded_rect(c.x - 5.0, c.y - 6.0, 3.5, 12.0, 1.5, color);
            rounded_rect(c.x + 1.5, c.y - 6.0, 3.5, 12.0, 1.5, color);
            return;
        }
        Speed::Normal => 1,
        Speed::Fast => 2,
        Speed::Faster => 3,
    };
    let w = 7.0;
    let start = c.x - (count as f32 * w) / 2.0;
    for i in 0..count {
        let x = start + i as f32 * w;
        draw_triangle(
            vec2(x, c.y - 6.0),
            vec2(x, c.y + 6.0),
            vec2(x + w, c.y),
            color,
        );
    }
}

fn draw_side_panel(ui: Vec2) {
    let x = ui.x - SIDE_W - MARGIN;
    let y = TOP_BAR_H + MARGIN;
    let h = ui.y - y - MARGIN;
    rounded_rect(x, y, SIDE_W, h, 10.0, pal::PANEL);
    text("STAFF", x + 14.0, y + 26.0, 16.0, pal::TEXT);
    text("no staff yet", x + 14.0, y + 48.0, 14.0, pal::TEXT_DIM);
}

fn draw_bottom(game: &Game, hovered: Option<(usize, usize)>, ui: Vec2) {
    let y = ui.y - MARGIN;
    if let Some((x, ty)) = hovered {
        let tile: Tile = game.map.get(x, ty).unwrap();
        let walk = if tile.walkable() {
            "walkable"
        } else {
            "blocked"
        };
        let info = format!("({x}, {ty})  {}  -  {walk}", tile.name());
        let w = text_width(&info, 14.0) + 20.0;
        rounded_rect(MARGIN, y - 52.0, w, 24.0, 6.0, pal::PANEL);
        text(&info, MARGIN + 10.0, y - 35.0, 14.0, pal::TEXT);
    }
    text(
        "Space pause   1/2/3 speed   WASD / right-drag move   wheel zoom   M sound",
        MARGIN,
        y - 6.0,
        13.0,
        pal::TEXT_DIM,
    );
}

fn draw_night_over(game: &Game, ui: Vec2) {
    let (w, h) = (300.0, 92.0);
    let x = (ui.x - w) / 2.0;
    let y = (ui.y - h) / 2.0;
    rounded_rect(x, y, w, h, 12.0, pal::PANEL);

    let title = format!("NIGHT {} IS OVER", game.night);
    let tw = text_width(&title, 20.0);
    text(&title, x + (w - tw) / 2.0, y + 38.0, 20.0, pal::TEXT);

    let hint = "Press N for the next night";
    let hw = text_width(hint, 14.0);
    text(hint, x + (w - hw) / 2.0, y + 66.0, 14.0, pal::TEXT_DIM);
}
