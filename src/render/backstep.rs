//! A break on the back step (design/v0.2.md F2): the café's back door ajar, warm light on the
//! wet yard, rain dripping off the striped awning (or snow, or a clear starry sky), crates of
//! empty bottles, grandma's geranium, a far night train behind the fence — and Karolina sitting
//! on the step with a cup while the station cat comes over to sit beside her.
//!
//! Same units as [`super::dawn`]: [`VIEW_H`] tall, the caller picks the width. Stateless.

use macroquad::prelude::*;

use super::cat::{CatLook, CatPose, draw_cat_ex};
use super::colors as c;
use super::draw::*;
use super::look::{Facing, PersonLook, PersonPose, Pose};
use super::outside::Weather as Rain;
use super::person::{draw_cup, draw_person};
use super::season::{SeasonLook, Snowfall, draw_breath};

pub use super::dawn::VIEW_H;

/// The step's top (Karolina sits here).
const STEP_Y: f32 = 318.0;
const YARD_Y: f32 = 336.0;

#[derive(Debug, Clone, Copy)]
pub struct BackstepFrame {
    /// Seconds since she stepped out.
    pub t: f32,
    pub season: SeasonLook,
    /// 0 dry, 1 rain, 2 downpour.
    pub rain: f32,
    pub fog: f32,
    /// The station cat's affection 0..1 (it comes closer and purrs when high).
    pub cat: f32,
    /// The phone in her hands lights up (0..1).
    pub phone: f32,
}

/// Where Karolina sits (her feet), for the UI's message card.
pub fn karolina_feet(view: Rect) -> Vec2 {
    vec2(view.x + view.w * 0.42, STEP_Y + 26.0)
}

pub fn draw_backstep(view: Rect, f: &BackstepFrame) {
    let door_x = view.x + view.w * 0.42 - 70.0;
    // the night sky over the yard, the glow of the station
    rect_v(
        view.x,
        0.0,
        view.w,
        200.0,
        Color::from_hex(0x120c22),
        Color::from_hex(0x2a2050),
    );
    glow(
        view.x + view.w * 0.8,
        200.0,
        260.0,
        alpha(Color::from_hex(0x6a5aa8), 0.35),
    );
    if f.rain < 0.2 && f.season.snowfall < 0.2 {
        for i in 0..40u32 {
            let x = view.x + hash01(i * 3 + 1) * view.w;
            let y = 6.0 + hash01(i * 3 + 2) * 150.0;
            let tw = 0.55 + 0.45 * noise1(f.t * 0.8, i);
            draw_circle(x, y, 0.6 + hash01(i) * 0.8, alpha(WHITE, 0.6 * tw));
        }
    }
    // far roofs and the station canopy
    for i in 0..14u32 {
        let x = view.x - 20.0 + i as f32 * 70.0;
        let h = 30.0 + hash01(i) * 50.0;
        let col = mix(
            Color::from_hex(0x1d1636),
            Color::from_hex(0x2a2048),
            hash01(i + 5),
        );
        rect_v(x, 200.0 - h, 66.0, h + 20.0, col, col);
        if hash01(i + 9) > 0.5 {
            draw_rectangle(x + 20.0, 210.0 - h, 5.0, 6.0, alpha(c::AMBER, 0.7));
        }
    }
    // a night train gliding past behind the fence every so often
    let cycle = 16.0;
    let ph = (f.t + 6.0) % cycle;
    if ph < 5.0 {
        let tx = view.x - 600.0 + ph / 5.0 * (view.w + 1200.0);
        let ty = 196.0;
        rrect_v(
            tx,
            ty,
            560.0,
            22.0,
            6.0,
            Color::from_hex(0x3a3462),
            Color::from_hex(0x2a2448),
        );
        for j in 0..18 {
            draw_rectangle(
                tx + 10.0 + j as f32 * 30.0,
                ty + 6.0,
                18.0,
                8.0,
                alpha(Color::from_hex(0xc9c2ff), 0.75),
            );
        }
        glow(
            tx + 560.0,
            ty + 12.0,
            30.0,
            alpha(Color::from_hex(0xfff1d6), 0.5),
        );
    }
    // the courtyard fence
    let fence = Color::from_hex(0x2a2140);
    rect_v(view.x, 214.0, view.w, 10.0, fence, shade(fence, 0.8));
    let mut x = view.x;
    while x < view.right() {
        rect_v(x, 210.0, 4.0, 30.0, fence, fence);
        x += 22.0;
    }
    // the back wall of the café: old bricks
    let wall_top = 226.0;
    rect_v(
        view.x,
        wall_top,
        view.w,
        YARD_Y - wall_top,
        Color::from_hex(0x4a3a6a),
        Color::from_hex(0x3a2c56),
    );
    let mut row = 0;
    let mut y = wall_top + 4.0;
    while y < YARD_Y {
        let off = if row % 2 == 0 { 0.0 } else { 14.0 };
        let mut bx = view.x - 28.0 + off;
        while bx < view.right() {
            let h = hash2((bx / 28.0) as i32, row);
            let col = mix(Color::from_hex(0x56427a), Color::from_hex(0x3e2f5e), h);
            rrect_v(bx + 1.0, y, 26.0, 9.0, 1.5, col, shade(col, 0.85));
            bx += 28.0;
        }
        y += 11.0;
        row += 1;
    }
    // the drainpipe
    let pipe_x = view.x + view.w * 0.82;
    rect_v(
        pipe_x - 5.0,
        wall_top - 20.0,
        10.0,
        YARD_Y - wall_top + 20.0,
        Color::from_hex(0x6a6488),
        Color::from_hex(0x4a4468),
    );
    // the door, ajar: warm light from the kitchen
    let d = Rect::new(door_x, 196.0, 76.0, STEP_Y - 196.0);
    rect_v(
        d.x - 6.0,
        d.y - 6.0,
        d.w + 12.0,
        d.h + 6.0,
        Color::from_hex(0x2a1f3a),
        Color::from_hex(0x221a30),
    );
    rect_v(
        d.x,
        d.y,
        d.w,
        d.h,
        Color::from_hex(0xffd59a),
        Color::from_hex(0xf0a35c),
    );
    // a glimpse of the kitchen: a shelf with jars
    draw_line(
        d.x + 4.0,
        d.y + 34.0,
        d.x + 40.0,
        d.y + 34.0,
        2.0,
        Color::from_hex(0xa8683e),
    );
    for j in 0..3 {
        rrect_v(
            d.x + 8.0 + j as f32 * 11.0,
            d.y + 22.0,
            8.0,
            12.0,
            2.0,
            Color::from_hex(0xe8b04b),
            Color::from_hex(0xc98a3a),
        );
    }
    // the door leaf, opened outwards
    quad(
        [
            vec2(d.right() - 2.0, d.y),
            vec2(d.right() + 34.0, d.y + 8.0),
            vec2(d.right() + 34.0, d.bottom() + 4.0),
            vec2(d.right() - 2.0, d.bottom()),
        ],
        [
            Color::from_hex(0x6e4532),
            Color::from_hex(0x5a3828),
            Color::from_hex(0x4a2e22),
            Color::from_hex(0x5e3a2a),
        ],
    );
    // light spilling into the yard
    glow(
        d.center().x + 30.0,
        YARD_Y + 30.0,
        220.0,
        alpha(c::AMBER, 0.3),
    );
    glow(d.center().x, d.y + d.h * 0.5, 120.0, alpha(c::AMBER, 0.22));
    // the bulb over the door
    let bulb = vec2(d.center().x, d.y - 34.0);
    glow(bulb.x, bulb.y, 60.0, alpha(c::AMBER, 0.45));
    draw_circle(bulb.x, bulb.y, 4.5, Color::from_hex(0xfff1d6));
    // the striped awning
    let aw = Rect::new(d.x - 70.0, d.y - 28.0, d.w + 160.0, 26.0);
    let n = 12;
    for i in 0..n {
        let sx = aw.x + i as f32 * aw.w / n as f32;
        let sw = aw.w / n as f32;
        let col = if i % 2 == 0 {
            Color::from_hex(0x4f46e5)
        } else {
            Color::from_hex(0xe8dcc6)
        };
        quad(
            [
                vec2(sx, aw.y),
                vec2(sx + sw, aw.y),
                vec2(sx + sw * 1.05, aw.bottom()),
                vec2(sx - sw * 0.05, aw.bottom()),
            ],
            [
                shade(col, 0.6),
                shade(col, 0.6),
                shade(col, 0.85),
                shade(col, 0.85),
            ],
        );
        ellipse(sx + sw / 2.0, aw.bottom(), sw / 2.0, 4.0, shade(col, 0.75));
    }
    if f.season.snow_cover > 0.0 {
        rrect_v(
            aw.x,
            aw.y - 4.0,
            aw.w,
            6.0,
            3.0,
            alpha(Color::from_hex(0xe6e2f6), f.season.snow_cover),
            alpha(Color::from_hex(0xc8c2e6), f.season.snow_cover),
        );
    }
    // festive lights along the awning
    if f.season.festive > 0.0 {
        for i in 0..18 {
            let lx = aw.x + 6.0 + i as f32 * (aw.w - 12.0) / 17.0;
            let ly = aw.bottom() + 6.0 + ((i as f32) * 0.8).sin() * 2.0;
            let col = [0xffc46b, 0xf0607a, 0xa5b4fc, 0x10b981][i % 4];
            let tw = 0.7 + 0.3 * noise1(f.t * 1.2, i as u32);
            glow(
                lx,
                ly,
                6.0,
                alpha(Color::from_hex(col), 0.9 * f.season.festive * tw),
            );
        }
    }
    // the yard: wet cobbles, a puddle with the door's light in it
    rect_v(
        view.x,
        YARD_Y,
        view.w,
        VIEW_H - YARD_Y,
        Color::from_hex(0x3a3256),
        Color::from_hex(0x1f1a34),
    );
    let mut row = 0u32;
    let mut yy = YARD_Y + 6.0;
    while yy < VIEW_H + 4.0 {
        let k = (yy - YARD_Y) / (VIEW_H - YARD_Y);
        let sw = 14.0 + k * 6.0;
        let sh = 3.6 + k * 2.0;
        let off = if row.is_multiple_of(2) { 0.0 } else { sw * 0.5 };
        let mut i = ((view.x - off) / sw).floor() as i32;
        while (i as f32) * sw + off < view.right() + sw {
            let x = i as f32 * sw + off;
            let h = hash2(i, row as i32 + 50);
            ellipse(
                x,
                yy,
                sw * 0.44,
                sh,
                mix(Color::from_hex(0x4a4068), Color::from_hex(0x2e2848), h),
            );
            if h > 0.4 {
                ellipse(
                    x - 1.0,
                    yy - sh * 0.35,
                    sw * 0.24,
                    sh * 0.3,
                    alpha(Color::from_hex(0xffc46b), 0.08 * (1.0 - k)),
                );
            }
            i += 1;
        }
        yy += sh * 2.0 + 1.0;
        row += 1;
    }
    if f.season.snow_cover > 0.0 {
        rect_v(
            view.x,
            YARD_Y,
            view.w,
            VIEW_H - YARD_Y,
            alpha(Color::from_hex(0xcdc6ec), 0.8 * f.season.snow_cover),
            alpha(Color::from_hex(0x8f86bf), 0.8 * f.season.snow_cover),
        );
    } else {
        let px = d.center().x + 110.0;
        ellipse_v(
            px,
            YARD_Y + 54.0,
            60.0,
            9.0,
            alpha(Color::from_hex(0xf0a35c), 0.55),
            alpha(Color::from_hex(0x3a2c56), 0.6),
        );
        if f.rain > 0.0 {
            for j in 0..4u32 {
                let a = (f.t * 1.2 + hash01(j) * 2.0) % 1.0;
                draw_ellipse_lines(
                    px - 30.0 + j as f32 * 18.0,
                    YARD_Y + 54.0,
                    a * 9.0,
                    a * 2.6,
                    0.0,
                    0.8,
                    alpha(WHITE, 0.4 * (1.0 - a)),
                );
            }
        }
    }
    soft_shadow(d.center().x + 40.0, STEP_Y + 26.0, 130.0, 10.0, 0.35);
    // the concrete step
    rect_v(
        d.x - 30.0,
        STEP_Y,
        d.w + 120.0,
        20.0,
        Color::from_hex(0x8a80a8),
        Color::from_hex(0x5e5680),
    );
    rect_v(
        d.x - 30.0,
        STEP_Y,
        d.w + 120.0,
        3.0,
        Color::from_hex(0xb0a8cc),
        Color::from_hex(0x8a80a8),
    );
    rect_v(
        d.x - 40.0,
        STEP_Y + 18.0,
        d.w + 140.0,
        14.0,
        Color::from_hex(0x6a6290),
        Color::from_hex(0x4a4470),
    );
    // crates of empty bottles, grandma's geranium in an old tin
    let cx = view.x + view.w * 0.66;
    for (dx, dy) in [(0.0, 0.0), (6.0, -34.0)] {
        let r = Rect::new(cx + dx, YARD_Y - 4.0 + dy, 60.0, 34.0);
        rect_v(
            r.x,
            r.y,
            r.w,
            r.h,
            Color::from_hex(0x9c6a47),
            Color::from_hex(0x6e4532),
        );
        draw_line(
            r.x,
            r.y + 12.0,
            r.right(),
            r.y + 12.0,
            1.5,
            Color::from_hex(0x5a3828),
        );
        for j in 0..5 {
            let bx = r.x + 8.0 + j as f32 * 11.0;
            rrect_v(
                bx - 3.0,
                r.y - 14.0,
                6.0,
                16.0,
                2.0,
                alpha(Color::from_hex(0x7fbf9a), 0.85),
                alpha(Color::from_hex(0x3f7a5e), 0.85),
            );
            draw_line(
                bx - 1.0,
                r.y - 12.0,
                bx - 1.0,
                r.y - 4.0,
                1.0,
                alpha(WHITE, 0.4),
            );
        }
        if f.season.snow_cover > 0.0 {
            rect_v(
                r.x,
                r.y - 16.0,
                r.w,
                3.0,
                alpha(Color::from_hex(0xe6e2f6), f.season.snow_cover),
                alpha(Color::from_hex(0xe6e2f6), 0.0),
            );
        }
    }
    let gx = d.x - 46.0;
    rrect_v(
        gx - 12.0,
        STEP_Y - 22.0,
        24.0,
        22.0,
        3.0,
        Color::from_hex(0xa8a4c0),
        Color::from_hex(0x6a6488),
    );
    for j in 0..7u32 {
        let a = j as f32 / 7.0 * std::f32::consts::TAU;
        ellipse(
            gx + a.cos() * 9.0,
            STEP_Y - 28.0 + a.sin() * 4.0,
            6.0,
            4.0,
            Color::from_hex(0x3f7a5e),
        );
    }
    for j in 0..4u32 {
        let a = j as f32 * 1.6;
        ellipse(
            gx + a.cos() * 6.0,
            STEP_Y - 34.0 + a.sin() * 3.0,
            3.4,
            3.0,
            Color::from_hex(0xd9485a),
        );
    }
    // Karolina on the step with a cup, her phone glowing in the other hand
    let feet = karolina_feet(view);
    let look = PersonLook::preset("player");
    let pose = PersonPose {
        feet,
        facing: Facing::Down,
        pose: Pose::Sit,
        t: f.t,
    };
    soft_shadow(feet.x, feet.y + 2.0, 24.0, 5.0, 0.4);
    draw_person(&look, &pose);
    draw_breath(&look, &pose, f.t, f.season.breath);
    draw_cup(feet + vec2(-13.0, -34.0), 1.6, f.t, true);
    if f.phone > 0.0 {
        let pp = feet + vec2(11.0, -36.0);
        glow(
            pp.x,
            pp.y - 8.0,
            26.0,
            alpha(Color::from_hex(0xa5b4fc), 0.45 * f.phone),
        );
        rrect_v(
            pp.x - 4.0,
            pp.y - 8.0,
            8.0,
            12.0,
            2.0,
            mix(
                Color::from_hex(0x2c2a3e),
                Color::from_hex(0xc9c2ff),
                f.phone,
            ),
            mix(
                Color::from_hex(0x2c2a3e),
                Color::from_hex(0x818cf8),
                f.phone,
            ),
        );
    }
    // the station cat pads in from the yard and sits by her (closer when it trusts her)
    let arrive = ((f.t - 2.0) / 4.0).clamp(0.0, 1.0);
    let e = 1.0 - (1.0 - arrive) * (1.0 - arrive);
    let near = 46.0 - 18.0 * f.cat;
    let from = vec2(view.right() + 40.0, YARD_Y + 40.0);
    let to = vec2(feet.x + near, STEP_Y + 1.0);
    let cp = from.lerp(to, e);
    let pose = if arrive < 1.0 && f.t > 2.0 {
        CatPose::Walk
    } else if f.t > 14.0 && f.cat > 0.6 {
        CatPose::Sleep
    } else {
        CatPose::Sit
    };
    if f.t > 2.0 {
        soft_shadow(cp.x, cp.y + 1.0, 14.0, 3.0, 0.35);
        draw_cat_ex(
            cp,
            &CatLook {
                scale: 1.25,
                flip: true,
                ..Default::default()
            },
            pose,
            f.t,
            if arrive >= 1.0 { f.cat } else { 0.0 },
        );
    }
    // rain: the yard gets it, the awning keeps the step dry; drips off the awning's edge
    let rain = Rain {
        intensity: f.rain,
        wind: 0.08,
    };
    rain.draw_rain(Rect::new(view.x, 0.0, aw.x - view.x, VIEW_H), f.t);
    rain.draw_rain(
        Rect::new(aw.right(), 0.0, view.right() - aw.right(), VIEW_H),
        f.t + 3.1,
    );
    rain.draw_rain(Rect::new(aw.x, 0.0, aw.w, aw.y), f.t + 1.7);
    if f.rain > 0.0 {
        for i in 0..16u32 {
            let dx = aw.x + 8.0 + hash01(i) * (aw.w - 16.0);
            let period = 0.9 + hash01(i + 3) * 0.8;
            let a = ((f.t + hash01(i + 7) * period) % period) / period;
            let y = aw.bottom() + 4.0 + a * a * (YARD_Y + 30.0 - aw.bottom());
            ellipse(
                dx,
                y,
                1.3,
                2.6,
                alpha(Color::from_hex(0xc9c2ff), 0.75 * (1.0 - a * 0.5)),
            );
            if a > 0.92 {
                draw_ellipse_lines(dx, YARD_Y + 30.0, 5.0, 1.5, 0.0, 0.8, alpha(WHITE, 0.4));
            }
        }
        // the drainpipe gurgles
        for i in 0..6 {
            let a = (f.t * 1.6 + i as f32 / 6.0) % 1.0;
            ellipse(
                pipe_x + 7.0,
                YARD_Y - 6.0 + a * 14.0,
                1.6,
                2.4,
                alpha(Color::from_hex(0xc9c2ff), 0.6 * (1.0 - a)),
            );
        }
    }
    Snowfall {
        intensity: f
            .season
            .snowfall
            .max(if f.season.snow_cover > 0.5 { 0.6 } else { 0.0 }),
        wind: 0.1,
    }
    .draw(view, f.t);
    if f.fog > 0.0 {
        rect_v(
            view.x,
            120.0,
            view.w,
            200.0,
            alpha(Color::from_hex(0x6a5aa8), 0.0),
            alpha(Color::from_hex(0x6a5aa8), 0.3 * f.fog),
        );
    }
    // a soft vignette
    rect_v(
        view.x,
        0.0,
        view.w,
        70.0,
        Color::new(0.03, 0.02, 0.08, 0.55),
        Color::new(0.03, 0.02, 0.08, 0.0),
    );
    rect_v(
        view.x,
        VIEW_H - 80.0,
        view.w,
        80.0,
        Color::new(0.03, 0.02, 0.08, 0.0),
        Color::new(0.03, 0.02, 0.08, 0.6),
    );
}
