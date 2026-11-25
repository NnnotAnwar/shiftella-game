//! Пол (вид сверху), стены (вид 3/4 с высотой) и кэш статичного слоя.
//!
//! Стена — «разрез кукольного домика»: верх (cap) рисуется над тайлом,
//! сдвинутый вверх на высоту стены, под ним — передняя грань.
//! Стена, за которой (к югу) комната, — высокая, с окнами и панелями.
//! Стена, перед которой (к северу) комната, — низкий срез, чтобы не закрывать зал.

use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation, PipelineParams};
use macroquad::prelude::*;

use super::colors as c;
use super::draw::*;
use crate::map::TILE;

/// Высота обычной стены (мировые пиксели).
pub const WALL_H: f32 = 1.5 * TILE;
/// Высота низкого «среза» фасадной стены.
pub const LOW_WALL_H: f32 = 0.3 * TILE;
/// Высота панелей-вагонки внизу стены.
const WAINSCOT_H: f32 = 0.42 * TILE;

/// Вид стены.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WallKind {
    #[default]
    Plain,
    Window,
    Door,
}

/// Соседи стены: какие из 4 соседей — тоже стены (стена/окно/дверь)
/// и где пол комнаты (с юга — видим её внутреннюю грань, с севера — она фасадная).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct WallMask(pub u8);

impl WallMask {
    pub const N: u8 = 1;
    pub const E: u8 = 2;
    pub const S: u8 = 4;
    pub const W: u8 = 8;
    /// К северу — пол комнаты.
    pub const FLOOR_N: u8 = 16;
    /// К югу — пол комнаты.
    pub const FLOOR_S: u8 = 32;

    /// Посчитать маску для тайла (x, y). `is_wall` — стена/окно/дверь,
    /// `is_floor` — проходимый пол комнаты (включая мебель на полу).
    pub fn compute(
        x: i32,
        y: i32,
        is_wall: impl Fn(i32, i32) -> bool,
        is_floor: impl Fn(i32, i32) -> bool,
    ) -> WallMask {
        let mut m = 0;
        if is_wall(x, y - 1) {
            m |= Self::N;
        }
        if is_wall(x + 1, y) {
            m |= Self::E;
        }
        if is_wall(x, y + 1) {
            m |= Self::S;
        }
        if is_wall(x - 1, y) {
            m |= Self::W;
        }
        if is_floor(x, y - 1) {
            m |= Self::FLOOR_N;
        }
        if is_floor(x, y + 1) {
            m |= Self::FLOOR_S;
        }
        WallMask(m)
    }

    pub fn has(self, bit: u8) -> bool {
        self.0 & bit != 0
    }

    /// Низкий срез: фасад перед залом или нижний конец боковой стены.
    pub fn is_low(self) -> bool {
        if self.has(Self::FLOOR_S) {
            return false;
        }
        self.has(Self::FLOOR_N) || (self.has(Self::N) && !self.has(Self::S))
    }

    /// Высота стены в мировых пикселях.
    pub fn height(self) -> f32 {
        if self.is_low() { LOW_WALL_H } else { WALL_H }
    }
}

/// Мировые пиксели левого верхнего угла тайла.
pub fn tile_px(tile: Vec2) -> Vec2 {
    tile * TILE
}

/// Прямоугольник стекла окна на стене `tile` (для дождя на стекле и света).
pub fn window_glass_rect(tile: Vec2) -> Rect {
    let p = tile_px(tile);
    let face_top = p.y + TILE - WALL_H;
    let top = face_top + 12.0;
    let bottom = p.y + TILE - WAINSCOT_H - 9.0;
    Rect::new(p.x + 5.0, top, TILE - 10.0, bottom - top)
}

/// Ключ y-сортировки стены (для низких стен в общей очереди).
pub fn wall_sort_key(tile: Vec2) -> f32 {
    (tile.y + 1.0) * TILE
}

// ───────────────────────── пол ─────────────────────────

/// Пол тайла `tile` (координаты тайла: столбец, строка). `kind`:
/// `floor_wood` | `floor_tile` | `floor_carpet` | `pavement` (другое → дерево).
pub fn draw_floor(kind: &str, tile: Vec2) {
    let p = tile_px(tile);
    let (tx, ty) = (tile.x as i32, tile.y as i32);
    match kind {
        "floor_tile" => {
            let s = TILE / 2.0;
            for j in 0..2 {
                for i in 0..2 {
                    let dark = (tx * 2 + i + ty * 2 + j) % 2 == 0;
                    let base = if dark {
                        Color::from_hex(0x7a6fa0)
                    } else {
                        Color::from_hex(0xbeb4d6)
                    };
                    let v = hash2(tx * 2 + i, ty * 2 + j) * 0.06;
                    let col = shade(base, 0.97 + v);
                    let (x, y) = (p.x + i as f32 * s, p.y + j as f32 * s);
                    draw_rectangle(x, y, s, s, shade(base, 0.72));
                    rrect_v(
                        x + 0.8,
                        y + 0.8,
                        s - 1.6,
                        s - 1.6,
                        2.0,
                        tint_light(col, 0.06),
                        shade(col, 0.94),
                    );
                    draw_line(
                        x + 3.0,
                        y + 3.0,
                        x + s * 0.45,
                        y + 3.0,
                        1.0,
                        alpha(WHITE, 0.18),
                    );
                }
            }
        }
        "pavement" => draw_pavement(p, tx, ty),
        "floor_carpet" => {
            let base = Color::from_hex(0x3e3678);
            draw_rectangle(p.x, p.y, TILE, TILE, base);
            for k in 0..10 {
                let hx = hash01((tx * 131 + ty * 71 + k) as u32);
                let hy = hash01((tx * 37 + ty * 113 + k * 7) as u32);
                ellipse(
                    p.x + hx * TILE,
                    p.y + hy * TILE,
                    3.0,
                    2.0,
                    alpha(Color::from_hex(0x4b428c), 0.7),
                );
            }
            // узор-ромбик в центре каждого второго тайла
            if (tx + ty) % 2 == 0 {
                let cpt = p + vec2(TILE / 2.0, TILE / 2.0);
                let s = 7.0;
                poly(
                    &[
                        cpt + vec2(0.0, -s),
                        cpt + vec2(s, 0.0),
                        cpt + vec2(0.0, s),
                        cpt + vec2(-s, 0.0),
                    ],
                    alpha(c::AMBER_DEEP, 0.35),
                );
            }
        }
        _ => {
            // дерево: 4 доски на тайл, швы со сдвигом
            let ph = TILE / 4.0;
            let tones = [0x83594a, 0x7a5243, 0x8c6150, 0x734c3e, 0x7f5647];
            for row in 0..4 {
                let gy = ty * 4 + row;
                let y = p.y + row as f32 * ph;
                // длина доски — 3 тайла, сдвиг по строкам
                let offset = (hash01(gy as u32 * 7 + 3) * 3.0 * TILE).floor();
                let board_len = 3.0 * TILE;
                let local = (p.x + offset).rem_euclid(board_len);
                let seam_x = p.x + (board_len - local);
                let segs: [(f32, f32, i32); 2] = if seam_x < p.x + TILE {
                    [(p.x, seam_x, 0), (seam_x, p.x + TILE, 1)]
                } else {
                    [(p.x, p.x + TILE, 0), (0.0, 0.0, 9)]
                };
                for (x0, x1, k) in segs {
                    if k == 9 || x1 - x0 < 0.5 {
                        continue;
                    }
                    let board_id = ((p.x + offset) / board_len).floor() as i32 + k;
                    let tone = tones[(hash((gy * 31 + board_id * 17) as u32) % 5) as usize];
                    let base = Color::from_hex(tone);
                    rect_v(x0, y, x1 - x0, ph, tint_light(base, 0.04), shade(base, 0.9));
                    // волокна
                    for g in 0..2 {
                        let gy2 = y + 4.0 + g as f32 * 6.0 + hash2(gy, board_id + g) * 3.0;
                        draw_line(x0, gy2, x1, gy2, 0.7, alpha(shade(base, 0.75), 0.35));
                    }
                }
                if seam_x < p.x + TILE {
                    draw_line(
                        seam_x,
                        y + 0.5,
                        seam_x,
                        y + ph - 0.5,
                        1.0,
                        alpha(Color::from_hex(0x2c1c1a), 0.6),
                    );
                }
                draw_line(
                    p.x,
                    y + ph - 0.5,
                    p.x + TILE,
                    y + ph - 0.5,
                    1.0,
                    alpha(Color::from_hex(0x2c1c1a), 0.55),
                );
                draw_line(
                    p.x,
                    y + 0.6,
                    p.x + TILE,
                    y + 0.6,
                    0.8,
                    alpha(tint_light(Color::from_hex(tones[0]), 0.3), 0.25),
                );
            }
        }
    }
}

/// Мокрая уличная брусчатка (как в `draw_outside`): камни со сдвигом рядов,
/// блики, иногда лужица с отражением фонаря. Статична — годится для кэша.
fn draw_pavement(p: Vec2, tx: i32, ty: i32) {
    draw_rectangle(p.x, p.y, TILE, TILE, Color::from_hex(0x2e2842));
    let (sw, sh) = (16.0, 11.2);
    let rows = (TILE / sh).ceil() as i32;
    for r in 0..rows {
        let gy = ty * 6 + r;
        let y = p.y + r as f32 * sh;
        let off = if gy % 2 == 0 { 0.0 } else { sw / 2.0 };
        let mut x = p.x - off;
        let mut k = 0;
        while x < p.x + TILE {
            let x0 = x.max(p.x);
            let x1 = (x + sw).min(p.x + TILE);
            let h = (sh - 2.0).min(p.y + TILE - y - 1.0);
            if x1 - x0 > 2.0 && h > 1.0 {
                let v = hash2(tx * 8 + k, gy);
                let col = mix(Color::from_hex(0x4a4266), Color::from_hex(0x5a5078), v);
                rrect_v(
                    x0 + 1.0,
                    y + 1.0,
                    x1 - x0 - 2.0,
                    h,
                    3.0,
                    col,
                    shade(col, 0.8),
                );
                if v > 0.6 {
                    draw_line(
                        x0 + 3.0,
                        y + 2.5,
                        x1 - 4.0,
                        y + 2.5,
                        1.0,
                        alpha(c::LAVENDER_RIM, 0.35),
                    );
                }
            }
            x += sw;
            k += 1;
        }
    }
    // лужа примерно на каждом четвёртом тайле
    if hash2(tx * 3 + 1, ty * 7 + 5) > 0.74 {
        let cx = p.x + TILE * (0.3 + hash2(tx, ty + 9) * 0.4);
        let cy = p.y + TILE * (0.35 + hash2(tx + 4, ty) * 0.3);
        let w = 16.0 + hash2(tx + 2, ty + 2) * 10.0;
        ellipse_v(
            cx,
            cy,
            w,
            7.0,
            Color::from_hex(0x1f1a38),
            Color::from_hex(0x2a2250),
        );
        glow(cx + w * 0.25, cy, 6.0, alpha(c::AMBER, 0.28));
        draw_line(
            cx - w * 0.5,
            cy - 2.5,
            cx + w * 0.05,
            cy - 3.2,
            1.0,
            alpha(c::LAVENDER_RIM, 0.4),
        );
    }
}

// ───────────────────────── стены ─────────────────────────

/// Стена на тайле `tile` (столбец, строка). Высота — по маске (см. `WallMask::is_low`).
/// Рисуйте стены по строкам сверху вниз (или через очередь с `wall_sort_key`).
pub fn draw_wall(tile: Vec2, neighbours: WallMask, kind: WallKind) {
    let p = tile_px(tile);
    let m = neighbours;
    let h = m.height();
    let low = m.is_low();
    let (x, y) = (p.x, p.y);
    let face_top = y + TILE - h;
    let face_bot = y + TILE;

    // ── передняя грань
    if m.has(WallMask::FLOOR_S) {
        // внутренняя стена зала: штукатурка + панели
        rect_v(
            x,
            face_top,
            TILE,
            h - WAINSCOT_H,
            c::WALL_PLASTER_TOP,
            c::WALL_PLASTER_BOTTOM,
        );
        // лёгкая фактура штукатурки
        for k in 0..3 {
            let hx = hash2(tile.x as i32 * 3 + k, tile.y as i32);
            ellipse(
                x + hx * TILE,
                face_top + 14.0 + k as f32 * 17.0,
                9.0,
                4.0,
                alpha(WHITE, 0.03),
            );
        }
        let wy = face_bot - WAINSCOT_H;
        rect_v(x, wy, TILE, WAINSCOT_H, c::WAINSCOT_TOP, c::WAINSCOT_BOTTOM);
        for i in 0..2 {
            let px = x + 4.0 + i as f32 * TILE / 2.0;
            rrect_v(
                px,
                wy + 5.0,
                TILE / 2.0 - 8.0,
                WAINSCOT_H - 12.0,
                2.0,
                shade(c::WAINSCOT_TOP, 1.12),
                shade(c::WAINSCOT_BOTTOM, 1.05),
            );
        }
        // молдинг и плинтус
        rect_v(
            x,
            wy - 2.0,
            TILE,
            4.0,
            c::WALL_TRIM,
            shade(c::WALL_TRIM, 0.7),
        );
        rect_v(
            x,
            face_bot - 5.0,
            TILE,
            5.0,
            shade(c::WAINSCOT_BOTTOM, 0.8),
            shade(c::WAINSCOT_BOTTOM, 0.6),
        );
        // AO у потолка (под верхом стены)
        rect_v(
            x,
            face_top,
            TILE,
            8.0,
            alpha(Color::from_hex(0x1a1430), 0.45),
            alpha(Color::from_hex(0x1a1430), 0.0),
        );

        match kind {
            WallKind::Window => draw_window(tile),
            WallKind::Door => {
                let (dx, dw) = (x + 8.0, TILE - 16.0);
                rect_v(
                    dx - 3.0,
                    face_top + 10.0,
                    dw + 6.0,
                    h - 10.0,
                    c::WOOD_LIGHT,
                    c::WOOD,
                );
                rect_v(
                    dx,
                    face_top + 14.0,
                    dw,
                    h - 14.0,
                    Color::from_hex(0x2a1f3c),
                    Color::from_hex(0x3b2a40),
                );
                glow(x + TILE / 2.0, face_bot - 10.0, 22.0, alpha(c::AMBER, 0.25));
            }
            WallKind::Plain => {}
        }
    } else if !low || !m.has(WallMask::FLOOR_N) || kind != WallKind::Door {
        // наружная/торцевая грань — темнее, кирпичная кладка
        let top = Color::from_hex(0x4a3c6e);
        let bot = Color::from_hex(0x2e2546);
        rect_v(x, face_top, TILE, h, top, bot);
        let rows = (h / 9.0).ceil() as i32;
        for r in 0..rows {
            let ry = face_top + r as f32 * 9.0;
            draw_line(
                x,
                ry,
                x + TILE,
                ry,
                0.8,
                alpha(Color::from_hex(0x221a36), 0.55),
            );
            let off = if r % 2 == 0 { 0.0 } else { 10.0 };
            let mut bx = x + off;
            while bx < x + TILE {
                draw_line(
                    bx,
                    ry,
                    bx,
                    (ry + 9.0).min(face_bot),
                    0.8,
                    alpha(Color::from_hex(0x221a36), 0.45),
                );
                bx += 20.0;
            }
        }
        rect_v(
            x,
            face_bot - 3.0,
            TILE,
            3.0,
            alpha(BLACK, 0.0),
            alpha(BLACK, 0.3),
        );
    }

    // ── верх (срез стены)
    let cap_y = y - h;
    if low && kind == WallKind::Door {
        // проём двери в низком фасаде: латунный порог и тёплый свет с крыльца
        rect_v(
            x,
            y + TILE - 12.0,
            TILE,
            12.0,
            Color::from_hex(0x4a3c5e),
            Color::from_hex(0x2e2546),
        );
        rect_v(
            x,
            y + TILE - 14.0,
            TILE,
            3.0,
            Color::from_hex(0xe0b26a),
            Color::from_hex(0x9a7040),
        );
        rect_v(
            x,
            y + 8.0,
            TILE,
            TILE - 22.0,
            alpha(c::AMBER, 0.0),
            alpha(c::AMBER, 0.12),
        );
        return;
    }
    rect_v(x, cap_y, TILE, TILE, c::WALL_CAP, shade(c::WALL_CAP, 0.85));
    let edge = alpha(c::WALL_CAP_EDGE, 0.95);
    let lit = alpha(c::LAVENDER_RIM, 0.55);
    if !m.has(WallMask::N) {
        draw_line(x, cap_y + 1.0, x + TILE, cap_y + 1.0, 2.0, lit);
    }
    if !m.has(WallMask::S) {
        draw_line(
            x,
            cap_y + TILE - 1.0,
            x + TILE,
            cap_y + TILE - 1.0,
            2.0,
            edge,
        );
    }
    if !m.has(WallMask::W) {
        draw_line(x + 1.0, cap_y, x + 1.0, cap_y + TILE, 2.0, edge);
    }
    if !m.has(WallMask::E) {
        draw_line(
            x + TILE - 1.0,
            cap_y,
            x + TILE - 1.0,
            cap_y + TILE,
            2.0,
            edge,
        );
    }
}

fn draw_window(tile: Vec2) {
    let g = window_glass_rect(tile);
    let frame = Color::from_hex(0xd9cdf2);
    // рама
    rrect_v(
        g.x - 4.0,
        g.y - 4.0,
        g.w + 8.0,
        g.h + 8.0,
        3.0,
        frame,
        shade(frame, 0.75),
    );
    // стекло: ночь снаружи, огоньки города в боке
    rect_v(
        g.x,
        g.y,
        g.w,
        g.h,
        Color::from_hex(0x1b1a48),
        Color::from_hex(0x3a2d6a),
    );
    let (tx, ty) = (tile.x as i32, tile.y as i32);
    for k in 0..4 {
        let hx = hash2(tx * 5 + k, ty * 3);
        let hy = hash2(tx * 7, ty + k * 11);
        let col = if k % 2 == 0 { c::AMBER } else { c::INDIGO_SOFT };
        glow(
            g.x + 4.0 + hx * (g.w - 8.0),
            g.y + g.h * (0.45 + hy * 0.45),
            5.0 + hx * 4.0,
            alpha(col, 0.35),
        );
    }
    // отражение-блик
    quad(
        [
            vec2(g.x + 6.0, g.y),
            vec2(g.x + 16.0, g.y),
            vec2(g.x + 4.0, g.y + g.h),
            vec2(g.x - 6.0 + 6.0, g.y + g.h),
        ],
        [
            alpha(WHITE, 0.1),
            alpha(WHITE, 0.1),
            alpha(WHITE, 0.0),
            alpha(WHITE, 0.0),
        ],
    );
    // переплёт
    let mx = g.x + g.w / 2.0;
    draw_line(mx, g.y, mx, g.y + g.h, 2.5, frame);
    draw_line(
        g.x,
        g.y + g.h * 0.38,
        g.x + g.w,
        g.y + g.h * 0.38,
        2.0,
        frame,
    );
    // подоконник
    rrect_v(
        g.x - 7.0,
        g.y + g.h + 3.0,
        g.w + 14.0,
        5.0,
        2.0,
        tint_light(frame, 0.2),
        shade(frame, 0.7),
    );
}

// ───────────────────────── кэш ─────────────────────────

const VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying lowp vec2 uv;
varying lowp vec4 color;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    uv = texcoord;
    color = color0 / 255.0;
}"#;
const FRAGMENT: &str = r#"#version 100
precision mediump float;
varying lowp vec2 uv;
varying lowp vec4 color;
uniform sampler2D Texture;
void main() {
    gl_FragColor = color * texture2D(Texture, uv);
}"#;

/// Кэш статичных слоёв (пол, высокие стены) в render target. Перестраивайте
/// при изменении карты (`build`), рисуйте каждый кадр (`draw`) камерой мира.
pub struct StaticLayer {
    rt: Option<RenderTarget>,
    rect: Rect,
    mat: Material,
}

impl Default for StaticLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl StaticLayer {
    pub fn new() -> StaticLayer {
        // Правильное «over» для альфы: иначе полупрозрачные тени делали бы дыры в кэше.
        let mat = load_material(
            ShaderSource::Glsl {
                vertex: VERTEX,
                fragment: FRAGMENT,
            },
            MaterialParams {
                pipeline_params: PipelineParams {
                    color_blend: Some(BlendState::new(
                        Equation::Add,
                        BlendFactor::Value(BlendValue::SourceAlpha),
                        BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                    )),
                    alpha_blend: Some(BlendState::new(
                        Equation::Add,
                        BlendFactor::One,
                        BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                    )),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .expect("static layer shader");
        StaticLayer {
            rt: None,
            rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            mat,
        }
    }

    /// Перерисовать кэш: всё, что нарисует `draw` в мировых координатах внутри
    /// `world_rect`, попадёт в текстуру с плотностью `scale` пикселей на мировой пиксель.
    /// После вызова активна камера по умолчанию — поставьте свою камеру снова.
    pub fn build(&mut self, world_rect: Rect, scale: f32, draw: impl FnOnce()) {
        let max = 8192.0;
        let scale = scale.min(max / world_rect.w).min(max / world_rect.h);
        let (w, h) = ((world_rect.w * scale) as u32, (world_rect.h * scale) as u32);
        let rt = render_target(w.max(1), h.max(1));
        rt.texture.set_filter(FilterMode::Linear);
        let mut cam = Camera2D::from_display_rect(world_rect);
        cam.render_target = Some(rt.clone());
        set_camera(&cam);
        clear_background(Color::new(0.0, 0.0, 0.0, 0.0));
        gl_use_material(&self.mat);
        draw();
        gl_use_default_material();
        set_default_camera();
        self.rt = Some(rt);
        self.rect = world_rect;
    }

    pub fn is_built(&self) -> bool {
        self.rt.is_some()
    }

    /// Нарисовать кэш (текущая камера — камера мира).
    pub fn draw(&self) {
        if let Some(rt) = &self.rt {
            draw_texture_ex(
                &rt.texture,
                self.rect.x,
                self.rect.y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(self.rect.size()),
                    // from_display_rect пишет в RT «вверх ногами» относительно мира
                    flip_y: true,
                    ..Default::default()
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAYOUT: [&str; 4] = ["#WW#", "#..#", "#..#", "#D##"];

    fn mask(x: i32, y: i32) -> WallMask {
        let at = |x: i32, y: i32| -> char {
            if x < 0 || y < 0 || y >= LAYOUT.len() as i32 || x >= 4 {
                return ' ';
            }
            LAYOUT[y as usize].chars().nth(x as usize).unwrap()
        };
        WallMask::compute(
            x,
            y,
            |x, y| matches!(at(x, y), '#' | 'W' | 'D'),
            |x, y| at(x, y) == '.',
        )
    }

    #[test]
    fn back_wall_is_tall_and_faces_room() {
        let m = mask(1, 0);
        assert!(m.has(WallMask::FLOOR_S));
        assert!(m.has(WallMask::E) && m.has(WallMask::W));
        assert!(!m.is_low());
        assert_eq!(m.height(), WALL_H);
    }

    #[test]
    fn front_wall_is_a_low_cutaway() {
        let m = mask(2, 3);
        assert!(m.has(WallMask::FLOOR_N));
        assert!(m.is_low());
        // нижний угол боковой стены — тоже низкий
        assert!(mask(0, 3).is_low());
        // а боковая стена над ним — высокая
        assert!(!mask(0, 2).is_low());
        assert!(mask(0, 2).has(WallMask::N) && mask(0, 2).has(WallMask::S));
    }

    #[test]
    fn corners_know_their_neighbours() {
        let m = mask(0, 0);
        assert_eq!(m.0 & (WallMask::N | WallMask::W), 0);
        assert!(m.has(WallMask::E) && m.has(WallMask::S));
        assert!(!m.is_low());
    }

    #[test]
    fn window_glass_is_inside_the_wall_face() {
        let g = window_glass_rect(vec2(2.0, 0.0));
        assert!(g.x > 2.0 * TILE && g.right() < 3.0 * TILE);
        assert!(g.y > TILE - WALL_H && g.bottom() < TILE - WAINSCOT_H);
        assert!(g.h > 20.0);
    }
}
