//! Logos, crisp at any size.
//!
//! * **Game brand mark** (cup under a crescent moon, `assets/icon/mark_256.png`): decoded
//!   once into a small mip chain (box-filtered halves), each level linear-filtered; the level
//!   closest to twice the drawn pixel size is used, so the downscale never aliases.
//! * **Shiftella app logo** (the "S" squircle on staff badges, laptops and the in-game app):
//!   pre-rasterised at boot ([`init`]) with the Inter Bold glyph on a smooth indigo → violet
//!   squircle with a lit top edge, at several sizes rendered 2× (supersampled). Render code
//!   needs no UI fonts: [`super::draw_shiftella_logo`] draws the cached texture, and falls
//!   back to vector strokes before `init` (headless tests).

use std::cell::RefCell;

use macroquad::prelude::*;

use super::colors as c;
use super::draw::{alpha, glow, rrect_v};

struct Brand {
    /// Mip chain of the brand mark, largest first.
    mark: Vec<Texture2D>,
    /// "S" app logo, largest first (texture side in px). The targets stay alive: dropping
    /// one frees its texture.
    app: Vec<RenderTarget>,
}

thread_local! {
    static BRAND: RefCell<Option<Brand>> = const { RefCell::new(None) };
}

const MARK_PNG: &[u8] = include_bytes!("../../assets/icon/mark_256.png");

/// Halves an RGBA image with a 2×2 box filter (premultiplied, so edges don't darken).
fn half(img: &Image) -> Image {
    let (w, h) = ((img.width / 2).max(1), (img.height / 2).max(1));
    let src = &img.bytes;
    let sw = img.width as usize;
    let mut out = vec![0u8; w as usize * h as usize * 4];
    for y in 0..h as usize {
        for x in 0..w as usize {
            let mut acc = [0f32; 4];
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let i = ((y * 2 + dy) * sw + x * 2 + dx) * 4;
                let a = src[i + 3] as f32 / 255.0;
                for k in 0..3 {
                    acc[k] += src[i + k] as f32 * a;
                }
                acc[3] += a;
            }
            let o = (y * w as usize + x) * 4;
            let a = acc[3] / 4.0;
            for k in 0..3 {
                out[o + k] = if acc[3] > 0.0 {
                    (acc[k] / acc[3]).round().clamp(0.0, 255.0) as u8
                } else {
                    0
                };
            }
            out[o + 3] = (a * 255.0).round() as u8;
        }
    }
    Image {
        bytes: out,
        width: w,
        height: h,
    }
}

fn texture(img: &Image) -> Texture2D {
    let t = Texture2D::from_image(img);
    t.set_filter(FilterMode::Linear);
    t
}

/// Renders the "S" squircle into a `side`-px render target.
fn bake_app_logo(side: u32, font: &Font) -> RenderTarget {
    let rt = render_target(side, side);
    rt.texture.set_filter(FilterMode::Linear);
    let s = side as f32;
    let mut cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, s, s));
    cam.render_target = Some(rt.clone());
    set_camera(&cam);
    clear_background(Color::new(0.0, 0.0, 0.0, 0.0));
    let r = s * 0.27;
    // body: indigo → violet, with a deeper bottom
    rrect_v(0.0, 0.0, s, s, r, Color::from_hex(0x7174f6), c::VIOLET);
    // lit top edge: a thin bright rim fading into the body
    rrect_v(
        s * 0.015,
        s * 0.012,
        s * 0.97,
        s * 0.5,
        r * 0.97,
        Color::new(1.0, 1.0, 1.0, 0.30),
        Color::new(1.0, 1.0, 1.0, 0.0),
    );
    rrect_v(
        s * 0.045,
        s * 0.04,
        s * 0.91,
        s * 0.93,
        r * 0.9,
        alpha(c::INDIGO_600, 0.0),
        alpha(c::INDIGO_600, 0.18),
    );
    // the S, optically centred (the bowl sits a hair high in Inter)
    let fs = (s * 0.66).round() as u16;
    let dim = measure_text("S", Some(font), fs, 1.0);
    let x = (s - dim.width) / 2.0 - s * 0.004;
    let base = (s + dim.offset_y) / 2.0 - s * 0.01;
    draw_text_ex(
        "S",
        x,
        base + s * 0.012,
        TextParams {
            font: Some(font),
            font_size: fs,
            color: Color::new(0.16, 0.08, 0.38, 0.35),
            ..Default::default()
        },
    );
    draw_text_ex(
        "S",
        x,
        base,
        TextParams {
            font: Some(font),
            font_size: fs,
            color: WHITE,
            ..Default::default()
        },
    );
    // SAFETY: flushes macroquad's own batcher so the target is filled right away.
    unsafe { get_internal_gl() }.flush();
    set_default_camera();
    rt
}

/// Builds the cached logos. Call once after the window and fonts exist.
pub fn init(font: &Font) {
    let mut mark = Vec::new();
    let mut img = Image::from_file_with_format(MARK_PNG, Some(ImageFormat::Png))
        .unwrap_or_else(|_| Image::gen_image_color(4, 4, Color::new(0.39, 0.4, 0.95, 1.0)));
    loop {
        mark.push(texture(&img));
        if img.width <= 16 {
            break;
        }
        img = half(&img);
    }
    let app = [512u32, 256, 128, 64, 32]
        .iter()
        .map(|&side| bake_app_logo(side, font))
        .collect();
    BRAND.with(|b| *b.borrow_mut() = Some(Brand { mark, app }));
}

/// Physical pixels per world / UI unit of the current camera.
fn px_per_unit() -> f32 {
    // SAFETY: reads the current projection of macroquad's own batcher.
    let m = unsafe { get_internal_gl() }.quad_gl.get_projection_matrix();
    (m.x_axis.x.abs() * screen_width() / 2.0).max(1e-3)
}

/// The level whose side is closest to (but not below) twice the drawn size.
fn pick(levels: &[Texture2D], px: f32) -> Option<&Texture2D> {
    levels
        .iter()
        .rev()
        .find(|t| t.width() >= px * 1.9)
        .or_else(|| levels.first())
}

/// The game's brand mark (cup and moon) centred at `c`, `size` units wide, with a soft glow.
pub fn draw_brand_mark(center: Vec2, size: f32, glow_k: f32) {
    if glow_k > 0.0 {
        glow(
            center.x,
            center.y,
            size * 1.05,
            alpha(c::INDIGO_GLOW, 0.32 * glow_k),
        );
    }
    let px = size * px_per_unit();
    let drawn = BRAND.with(|b| {
        let b = b.borrow();
        let Some(t) = b.as_ref().and_then(|b| pick(&b.mark, px)) else {
            return false;
        };
        draw_texture_ex(
            t,
            center.x - size / 2.0,
            center.y - size / 2.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(size, size)),
                ..Default::default()
            },
        );
        true
    });
    if !drawn {
        draw_app_logo(center, size);
    }
}

/// The baked "S" logo; `false` when [`init`] hasn't run.
pub(crate) fn draw_app_logo(center: Vec2, size: f32) -> bool {
    let px = size * px_per_unit();
    BRAND.with(|b| {
        let b = b.borrow();
        let Some(t) = b.as_ref().and_then(|b| {
            b.app
                .iter()
                .rev()
                .map(|rt| &rt.texture)
                .find(|t| t.width() >= px * 1.9)
                .or_else(|| b.app.first().map(|rt| &rt.texture))
        }) else {
            return false;
        };
        draw_texture_ex(
            t,
            center.x - size / 2.0,
            center.y - size / 2.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(size, size)),
                // render targets come out upside down
                flip_y: true,
                ..Default::default()
            },
        );
        true
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halving_keeps_colour_under_transparent_edges() {
        // one opaque red pixel next to three transparent black ones
        let img = Image {
            bytes: vec![
                255, 0, 0, 255, 0, 0, 0, 0, //
                0, 0, 0, 0, 0, 0, 0, 0,
            ],
            width: 2,
            height: 2,
        };
        let h = half(&img);
        assert_eq!((h.width, h.height), (1, 1));
        assert_eq!(&h.bytes[..3], &[255, 0, 0]);
        assert_eq!(h.bytes[3], 64);
    }
}
