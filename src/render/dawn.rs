//! The walk home at dawn (design/v0.2.md F1): Holešovice after the night shift — an indigo →
//! lavender → peach sky, the city's silhouette (the two towers of St Anthony's, the TV tower,
//! the old power plant's chimneys), wet cobbles, streetlamps going out, pigeons, and one of four
//! ways home: the Vltava embankment, Stromovka park, past the bakery, or the first tram.
//!
//! A side-on strip in its own units: [`VIEW_H`] tall, the caller picks the width from the screen
//! aspect and a left edge with [`camera_left`]. Stateless: the picture is a function of the
//! [`DawnFrame`] (handy for screenshots). Karolina is the game's own person rig.

use macroquad::prelude::*;

use super::colors as c;
use super::draw::*;
use super::look::{Accessory, Facing, PersonLook, PersonPose, Pose, TopStyle};
use super::outside::{Weather as Rain, world_text};
use super::person::draw_person;
use super::season::{SeasonLook, Snowfall, draw_breath, draw_falling_leaves};

/// The strip's height in its own units (a person is ~83 tall).
pub const VIEW_H: f32 = 430.0;
/// Where feet stand on the pavement.
pub const GROUND_Y: f32 = 356.0;
/// The top of the pavement (the back edge where railings, houses and the tram stand).
pub const PAVE_Y: f32 = 322.0;
/// The far horizon (skyline base).
pub const HORIZON_Y: f32 = 240.0;
/// Extra sky above the strip's 0 line: the caller's view starts at `-SKY_EXTRA`.
pub const SKY_EXTRA: f32 = 110.0;
/// Karolina's door at the café.
pub const CAFE_DOOR_X: f32 = 236.0;

/// Which way home.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DawnRoute {
    Embankment,
    Park,
    Bakery,
    Tram,
}

impl DawnRoute {
    pub fn from_id(id: &str) -> Option<DawnRoute> {
        Some(match id {
            "embankment" => DawnRoute::Embankment,
            "park" => DawnRoute::Park,
            "bakery" => DawnRoute::Bakery,
            "tram" => DawnRoute::Tram,
            _ => return None,
        })
    }
}

/// Her front door at the end of a route.
pub fn home_x(route: Option<DawnRoute>) -> f32 {
    match route {
        Some(DawnRoute::Tram) => 2900.0,
        _ => 1950.0,
    }
}

/// The tram stop (tram route).
pub const TRAM_STOP_X: f32 = 640.0;
/// Where the tram lets her off near home.
pub fn tram_home_stop(route: Option<DawnRoute>) -> f32 {
    home_x(route) - 300.0
}

/// One frame of the walk.
#[derive(Debug, Clone, Copy)]
pub struct DawnFrame {
    /// `None` = still locking the café door (the route isn't chosen yet).
    pub route: Option<DawnRoute>,
    /// Karolina's feet (x along the strip; y = [`GROUND_Y`] unless boarding).
    pub karolina: Vec2,
    pub pose: Pose,
    pub facing: Facing,
    /// Hidden while she rides the tram.
    pub visible: bool,
    /// The tram's front x (tram route), `None` = not on screen.
    pub tram_x: Option<f32>,
    /// 0 = blue hour … 1 = the sun is up.
    pub light: f32,
    pub t: f32,
    pub season: SeasonLook,
    /// 0 dry, 1 rain, 2 downpour.
    pub rain: f32,
    pub fog: f32,
    /// Her flat's window upstairs (0..1).
    pub window_lit: f32,
}

/// The view's left edge so that `focus` sits a little left of centre (clamped to the strip).
pub fn camera_left(route: Option<DawnRoute>, focus: f32, view_w: f32) -> f32 {
    let max = home_x(route) + 300.0 - view_w;
    (focus - view_w * 0.42).clamp(0.0, max.max(0.0))
}

/// Karolina off duty: a plum coat over the sweater, a scarf when it's cold.
pub fn karolina_look(season: &SeasonLook) -> PersonLook {
    let mut l = PersonLook::preset("player");
    l.apron = false;
    l.top = Color::from_hex(0x7b4f8e);
    l.top_style = TopStyle::Coat;
    if season.breath > 0.3 {
        l.accessory = Accessory::Scarf;
        l.accent = Color::from_hex(0xe8b04b);
    }
    l
}

// ───────────────────────── palette ─────────────────────────

struct Sky {
    top: Color,
    mid: Color,
    horizon: Color,
    /// Haze for far silhouettes.
    haze: Color,
    /// Light falling on facades (cool → warm).
    key: Color,
}

fn sky(light: f32) -> Sky {
    let l = light.clamp(0.0, 1.0);
    let top = mix(Color::from_hex(0x1d1a45), Color::from_hex(0x3e3b86), l);
    let mid = mix(Color::from_hex(0x4d3f8a), Color::from_hex(0x9a86cf), l);
    let horizon = mix(Color::from_hex(0x9a74a8), Color::from_hex(0xffc9a3), l);
    Sky {
        top,
        mid,
        horizon,
        haze: mix(mid, horizon, 0.45),
        key: mix(Color::from_hex(0x8d84c8), Color::from_hex(0xffd2b0), l),
    }
}

/// A facade colour in the dawn light (shadowed, tinted by the sky).
fn lit(base: Color, sk: &Sky, k: f32) -> Color {
    mix(shade(base, 0.55 + 0.25 * k), sk.key, 0.18 + 0.12 * k)
}

fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// x of a parallax layer (`p` = 1 moves with the ground, 0 stays).
fn px(x: f32, left: f32, p: f32) -> f32 {
    x + left * (1.0 - p)
}

// ───────────────────────── the frame ─────────────────────────

/// Draws the whole strip in the current camera (`view` = the visible rect in strip units).
pub fn draw_dawn(view: Rect, f: &DawnFrame) {
    let sk = sky(f.light);
    let left = view.x;
    draw_sky(view, f, &sk);
    draw_skyline(view, f, &sk);
    match f.route {
        Some(DawnRoute::Embankment) => draw_river(view, f, &sk),
        Some(DawnRoute::Park) => draw_park_back(view, f, &sk),
        Some(DawnRoute::Bakery) => draw_street_back(view, f, &sk, 0.55),
        Some(DawnRoute::Tram) => draw_street_back(view, f, &sk, 0.6),
        None => draw_street_back(view, f, &sk, 0.55),
    }
    if f.fog > 0.0 {
        rect_v(
            view.x,
            HORIZON_Y - 60.0,
            view.w,
            PAVE_Y - HORIZON_Y + 60.0,
            alpha(sk.haze, 0.0),
            alpha(sk.haze, 0.55 * f.fog),
        );
    }
    // the near row: houses / railings / trees behind the pavement
    draw_near_back(view, f, &sk);
    draw_cafe(view, f, &sk);
    draw_home(view, f, &sk);
    if f.route == Some(DawnRoute::Tram) {
        draw_tram_stop(view, f, &sk, TRAM_STOP_X);
        draw_tram_stop(view, f, &sk, tram_home_stop(f.route));
        if let Some(x) = f.tram_x {
            draw_tram(x, f, &sk);
        }
        draw_wires(view, f, &sk);
    }
    draw_pavement(view, f, &sk);
    draw_lamps(view, f, &sk);
    draw_pigeons(view, f, &sk);
    draw_footprints(f);
    if f.visible {
        draw_karolina(f, &sk);
    }
    // weather over everything
    let rain = Rain {
        intensity: f.rain,
        wind: 0.12,
    };
    rain.draw_rain(view, f.t);
    Snowfall {
        intensity: f.season.snowfall,
        wind: 0.1,
    }
    .draw(view, f.t);
    draw_falling_leaves(view, f.season.leaves * 0.35, 0.4, f.t);
    if f.fog > 0.0 {
        rect_v(
            view.x,
            PAVE_Y - 30.0,
            view.w,
            VIEW_H - PAVE_Y + 30.0,
            alpha(sk.haze, 0.12 * f.fog),
            alpha(sk.haze, 0.0),
        );
    }
    // soft letterbox shading and a warm low sun wash
    rect_v(
        view.x,
        view.y,
        view.w,
        60.0,
        Color::new(0.05, 0.03, 0.12, 0.35),
        Color::new(0.05, 0.03, 0.12, 0.0),
    );
    rect_v(
        view.x,
        VIEW_H - 70.0,
        view.w,
        70.0,
        Color::new(0.05, 0.03, 0.12, 0.0),
        Color::new(0.05, 0.03, 0.12, 0.45),
    );
    let _ = left;
}

fn draw_sky(view: Rect, f: &DawnFrame, sk: &Sky) {
    let band = HORIZON_Y * 0.5;
    rect_v(view.x, view.y, view.w, band - view.y, sk.top, sk.mid);
    rect_v(
        view.x,
        band,
        view.w,
        HORIZON_Y - band + 40.0,
        sk.mid,
        sk.horizon,
    );
    // fading stars
    let star_a = (0.65 - f.light).max(0.0) * 1.3;
    if star_a > 0.0 {
        for i in 0..46u32 {
            let x = view.x + hash01(i * 3 + 1) * view.w;
            let y = view.y + 6.0 + hash01(i * 3 + 2) * 200.0;
            let tw = 0.6 + 0.4 * noise1(f.t * 0.7, i);
            let r = 0.6 + hash01(i * 7) * 0.9;
            let fade = (1.0 - (y - view.y) / 240.0).max(0.0);
            draw_circle(x, y, r, Color::new(0.95, 0.93, 1.0, star_a * tw * fade));
        }
    }
    // the sun: a soft peach dawn behind the skyline, climbing with the light
    let sx = view.x + view.w * 0.7;
    let sy = HORIZON_Y + 6.0 - 46.0 * f.light;
    glow(
        sx,
        sy,
        260.0,
        alpha(Color::from_hex(0xffb38a), 0.28 + 0.3 * f.light),
    );
    glow(
        sx,
        sy,
        90.0,
        alpha(Color::from_hex(0xffe0b8), 0.25 + 0.45 * f.light),
    );
    draw_circle(
        sx,
        sy,
        13.0,
        alpha(Color::from_hex(0xfff1d6), 0.25 + 0.65 * f.light),
    );
    // long soft clouds lit pink from below
    for i in 0..6u32 {
        let w = 120.0 + hash01(i * 5) * 160.0;
        let x0 = hash01(i * 5 + 1) * (view.w + w) - w * 0.5 + f.t * (2.0 + hash01(i) * 2.0);
        let x = view.x + x0.rem_euclid(view.w + w) - w * 0.5;
        let y = view.y + 40.0 + hash01(i * 5 + 2) * 200.0;
        let body = mix(sk.mid, sk.top, 0.3);
        ellipse(
            x,
            y,
            w * 0.5,
            7.0 + hash01(i * 5 + 3) * 5.0,
            alpha(body, 0.55),
        );
        ellipse(
            x + 6.0,
            y + 4.0,
            w * 0.42,
            3.5,
            alpha(
                mix(sk.horizon, Color::from_hex(0xffa8a8), 0.4),
                0.35 + 0.35 * f.light,
            ),
        );
    }
}

/// The city far away: roofs, the two-towered church, the TV tower, chimneys and a crane.
fn draw_skyline(view: Rect, f: &DawnFrame, sk: &Sky) {
    let p = 0.12;
    let left = view.x;
    let far = mix(sk.haze, Color::from_hex(0x6a5a9a), 0.35);
    let base = HORIZON_Y + 12.0;
    // Letná's long green ridge
    let ridge = alpha(mix(far, sk.horizon, 0.25), 1.0);
    let mut pts = Vec::new();
    let x0 = view.x;
    pts.push(vec2(x0, base + 10.0));
    for i in 0..=24 {
        let x = x0 + view.w * i as f32 / 24.0;
        let lx = x - left * p;
        let h = 16.0 + 10.0 * (lx * 0.004).sin() + 6.0 * (lx * 0.011 + 1.3).sin();
        pts.push(vec2(x, base - h));
    }
    pts.push(vec2(x0 + view.w, base + 10.0));
    for w in pts.windows(2) {
        quad(
            [
                w[0],
                w[1],
                vec2(w[1].x, base + 10.0),
                vec2(w[0].x, base + 10.0),
            ],
            [ridge, ridge, ridge, ridge],
        );
    }
    // roofs
    let roof = mix(far, sk.mid, 0.2);
    let start = ((left * p - 80.0) / 46.0).floor() as i32;
    let end = ((left * p + view.w + 80.0) / 46.0).ceil() as i32;
    for i in start..end {
        let k = i.unsigned_abs() * 7 + if i < 0 { 3 } else { 0 };
        let lx = i as f32 * 46.0 + hash01(k) * 20.0;
        let x = px(lx, left, p);
        let w = 34.0 + hash01(k + 1) * 30.0;
        let h = 14.0 + hash01(k + 2) * 26.0;
        rect_v(x, base - h, w, h + 12.0, roof, roof);
        if hash01(k + 3) > 0.5 {
            poly(
                &[
                    vec2(x - 2.0, base - h),
                    vec2(x + w + 2.0, base - h),
                    vec2(x + w * 0.5, base - h - 9.0),
                ],
                roof,
            );
        }
        // a few windows still lit from the night
        if hash01(k + 4) > 0.55 {
            let wx = x + 6.0 + hash01(k + 5) * (w - 14.0);
            let wy = base - h + 5.0 + hash01(k + 6) * (h - 8.0).max(1.0);
            let a = (1.0 - f.light * 0.8) * 0.8;
            draw_rectangle(wx, wy, 2.5, 3.0, alpha(c::AMBER, a));
        }
    }
    // landmarks
    let church = px(430.0, left, p);
    let tw = mix(far, sk.top, 0.18);
    for dx in [-12.0, 12.0] {
        let x = church + dx;
        rect_v(x - 6.0, base - 70.0, 12.0, 70.0, tw, tw);
        poly(
            &[
                vec2(x - 7.0, base - 70.0),
                vec2(x + 7.0, base - 70.0),
                vec2(x, base - 96.0),
            ],
            tw,
        );
        draw_line(x, base - 96.0, x, base - 103.0, 1.2, tw);
    }
    rect_v(church - 20.0, base - 40.0, 40.0, 40.0, tw, tw);
    poly(
        &[
            vec2(church - 22.0, base - 40.0),
            vec2(church + 22.0, base - 40.0),
            vec2(church, base - 54.0),
        ],
        tw,
    );
    // the TV tower: three slim legs and the pods
    let tv = px(820.0, left, p);
    let tvc = mix(far, sk.top, 0.12);
    for dx in [-5.0, 0.0, 5.0] {
        rect_v(tv + dx - 2.0, base - 150.0, 4.0, 150.0, tvc, tvc);
    }
    for (y, w) in [(118.0, 22.0), (92.0, 18.0)] {
        rrect_v(tv - w / 2.0, base - y, w, 10.0, 4.0, tvc, tvc);
    }
    draw_line(tv, base - 150.0, tv, base - 176.0, 1.5, tvc);
    // the red aircraft light blinks until the sun is up
    let blink = ((f.t * 1.3).sin() > 0.6) as i32 as f32 * (1.0 - f.light * 0.7);
    glow(
        tv,
        base - 176.0,
        5.0,
        alpha(Color::from_hex(0xff6a6a), 0.8 * blink),
    );
    // power plant chimneys with a lazy plume
    let pp = px(180.0, left, p);
    for (dx, h) in [(0.0, 110.0), (26.0, 96.0)] {
        let x = pp + dx;
        poly_v(
            &[
                vec2(x - 6.0, base - h),
                vec2(x + 6.0, base - h),
                vec2(x + 9.0, base),
                vec2(x - 9.0, base),
            ],
            tw,
            tw,
        );
        for j in 0..5 {
            let k = j as f32;
            let drift = (f.t * 4.0 + k * 22.0) % 110.0;
            glow(
                x + drift * 0.9,
                base - h - 6.0 - drift * 0.25,
                10.0 + drift * 0.25,
                alpha(sk.horizon, 0.18 * (1.0 - drift / 110.0)),
            );
        }
    }
    // a crane
    let cr = px(640.0, left, p);
    draw_line(cr, base, cr, base - 120.0, 2.0, tw);
    draw_line(cr - 30.0, base - 116.0, cr + 70.0, base - 116.0, 1.6, tw);
    draw_line(cr + 50.0, base - 116.0, cr + 50.0, base - 90.0, 0.8, tw);
}

// ───────────────────────── route backgrounds ─────────────────────────

fn draw_river(view: Rect, f: &DawnFrame, sk: &Sky) {
    let left = view.x;
    let top = HORIZON_Y + 16.0;
    // the far bank: a dark strip of trees and lamps
    let bank = mix(sk.haze, Color::from_hex(0x3a2f66), 0.6);
    rect_v(
        view.x,
        HORIZON_Y + 2.0,
        view.w,
        top - HORIZON_Y + 2.0,
        bank,
        bank,
    );
    for i in 0..40u32 {
        let lx = i as f32 * 60.0 + hash01(i) * 30.0;
        let x = px(lx, left, 0.3);
        if x < view.x - 40.0 || x > view.right() + 40.0 {
            continue;
        }
        ellipse(x, HORIZON_Y + 4.0, 22.0 + hash01(i + 9) * 14.0, 10.0, bank);
        let a = (1.0 - f.light) * 0.9;
        glow(x + 10.0, top - 3.0, 6.0, alpha(c::AMBER, 0.5 * a));
    }
    // water: the sky upside down, darker
    let water_top = mix(sk.horizon, sk.mid, 0.3);
    let water_bot = mix(sk.mid, sk.top, 0.6);
    rect_v(view.x, top, view.w, PAVE_Y - top, water_top, water_bot);
    // the sun's path on the water
    let sx = view.x + view.w * 0.7;
    for i in 0..26u32 {
        let y = top + 3.0 + i as f32 * 2.4;
        let w = 6.0 + i as f32 * 2.2 + 8.0 * noise1(f.t * 1.5 + i as f32, i);
        let a = (0.25 + 0.45 * f.light) * (1.0 - i as f32 / 30.0);
        draw_line(
            sx - w * 0.5 + (f.t * 3.0 + i as f32).sin() * 3.0,
            y,
            sx + w * 0.5,
            y,
            1.0,
            alpha(Color::from_hex(0xffe2c2), a),
        );
    }
    // glints and ripples
    for i in 0..90u32 {
        let lx = hash01(i * 3) * 3000.0;
        let x = view.x + (px(lx, left, 0.6) - view.x).rem_euclid(view.w + 40.0) - 20.0;
        let y = top + 4.0 + hash01(i * 3 + 1) * (PAVE_Y - top - 8.0);
        let ph = (f.t * (0.8 + hash01(i) * 0.6) + hash01(i * 3 + 2) * 6.0).sin() * 0.5 + 0.5;
        let w = 4.0 + hash01(i * 7) * 10.0;
        draw_line(
            x,
            y,
            x + w,
            y,
            0.8,
            alpha(mix(sk.horizon, WHITE, 0.3), 0.25 * ph),
        );
    }
    // Libeň bridge: a long deck on arches
    let bx = px(900.0, left, 0.45);
    let deck = mix(sk.haze, Color::from_hex(0x4a3f74), 0.55);
    rect_v(bx - 60.0, top - 18.0, 900.0, 8.0, deck, shade(deck, 0.85));
    for i in 0..6 {
        let ax = bx - 20.0 + i as f32 * 150.0;
        arc(
            vec2(ax + 60.0, top - 10.0),
            62.0,
            16.0,
            std::f32::consts::PI,
            std::f32::consts::TAU,
            3.0,
            deck,
        );
        rect_v(ax - 4.0, top - 12.0, 8.0, 18.0, deck, alpha(deck, 0.4));
        // the bridge's reflection
        rect_v(
            ax - 4.0,
            top + 6.0,
            8.0,
            16.0,
            alpha(deck, 0.25),
            alpha(deck, 0.0),
        );
    }
    for i in 0..18 {
        let lx = bx - 40.0 + i as f32 * 50.0;
        glow(
            lx,
            top - 22.0,
            5.0,
            alpha(c::AMBER, 0.6 * (1.0 - f.light * 0.8)),
        );
    }
    // houseboats with warm windows, a swan
    for (lx, col) in [(520.0, 0x4d6b8a), (1180.0, 0x8a5a6e), (1720.0, 0x5a7a6a)] {
        let x = px(lx, left, 0.75);
        draw_houseboat(x, PAVE_Y - 8.0, Color::from_hex(col), f, sk);
    }
    let swan_x = px(820.0 + f.t * 5.0, left, 0.7);
    draw_swan(swan_x, PAVE_Y - 18.0, f.t, sk);
    // the embankment wall and railing
    let wall = lit(Color::from_hex(0x8a7a9a), sk, 0.5);
    rect_v(view.x, PAVE_Y - 6.0, view.w, 8.0, wall, shade(wall, 0.8));
}

fn draw_houseboat(x: f32, water: f32, col: Color, f: &DawnFrame, sk: &Sky) {
    let hull = lit(Color::from_hex(0x3a3550), sk, 0.4);
    poly_v(
        &[
            vec2(x - 70.0, water - 14.0),
            vec2(x + 70.0, water - 14.0),
            vec2(x + 60.0, water),
            vec2(x - 62.0, water),
        ],
        hull,
        shade(hull, 0.7),
    );
    let cab = lit(col, sk, 0.7);
    rrect_v(
        x - 50.0,
        water - 40.0,
        96.0,
        27.0,
        3.0,
        cab,
        shade(cab, 0.8),
    );
    rect_v(
        x - 54.0,
        water - 43.0,
        104.0,
        4.0,
        shade(cab, 0.7),
        shade(cab, 0.7),
    );
    for i in 0..4 {
        let wx = x - 42.0 + i as f32 * 23.0;
        let on = hash01((x as i32 as u32).wrapping_add(i)) > 0.35;
        let wc = if on {
            alpha(c::AMBER, 0.85 - f.light * 0.3)
        } else {
            shade(cab, 0.6)
        };
        rrect_v(wx, water - 34.0, 13.0, 11.0, 2.0, wc, shade(wc, 0.85));
        if on {
            glow(wx + 6.0, water - 28.0, 16.0, alpha(c::AMBER, 0.2));
            // the light's ripple in the water
            rect_v(
                wx + 2.0,
                water + 2.0,
                9.0,
                10.0,
                alpha(c::AMBER, 0.3),
                alpha(c::AMBER, 0.0),
            );
        }
    }
    // festive: a string of lights on the roof
    if f.season.festive > 0.0 {
        for i in 0..9 {
            let lx = x - 52.0 + i as f32 * 13.0;
            let ly = water - 45.0 + ((i as f32) * 1.3).sin() * 1.5;
            let col = [0xffc46b, 0xf0607a, 0xa5b4fc][i % 3];
            glow(
                lx,
                ly,
                4.0,
                alpha(Color::from_hex(col), 0.9 * f.season.festive),
            );
        }
    }
    if f.season.snow_cover > 0.0 {
        rrect_v(
            x - 55.0,
            water - 47.0,
            106.0,
            5.0,
            2.0,
            alpha(Color::from_hex(0xe6e2f6), f.season.snow_cover),
            alpha(Color::from_hex(0xc9c2ec), f.season.snow_cover),
        );
    }
}

fn draw_swan(x: f32, y: f32, t: f32, sk: &Sky) {
    let body = mix(Color::from_hex(0xf4f0fa), sk.horizon, 0.25);
    let bob = (t * 1.4).sin() * 0.8;
    ellipse(x, y + bob, 13.0, 5.5, body);
    ellipse(x + 6.0, y - 2.0 + bob, 7.0, 4.0, body);
    capsule(
        vec2(x - 9.0, y - 1.0 + bob),
        vec2(x - 12.0, y - 14.0 + bob),
        2.0,
        body,
    );
    ellipse(x - 13.5, y - 15.5 + bob, 3.2, 2.3, body);
    poly(
        &[
            vec2(x - 16.0, y - 16.0 + bob),
            vec2(x - 21.0, y - 14.5 + bob),
            vec2(x - 16.0, y - 14.0 + bob),
        ],
        Color::from_hex(0xf09a4e),
    );
    // its V on the water
    draw_line(
        x + 12.0,
        y + 5.0,
        x + 40.0,
        y + 9.0,
        0.8,
        alpha(WHITE, 0.25),
    );
    draw_line(
        x + 12.0,
        y + 6.0,
        x + 38.0,
        y + 2.0,
        0.8,
        alpha(WHITE, 0.18),
    );
}

fn tree_colors(season: &SeasonLook) -> (Color, Color) {
    // late autumn amber → frosted lavender → bare, snowy
    let autumn = (Color::from_hex(0xc9803e), Color::from_hex(0x8a4a3a));
    let frost = (Color::from_hex(0x9a8fb8), Color::from_hex(0x5e557e));
    let k = season.frost.max(season.snow_cover);
    (mix(autumn.0, frost.0, k), mix(autumn.1, frost.1, k))
}

fn draw_tree(x: f32, base: f32, h: f32, k: u32, f: &DawnFrame, sk: &Sky, near: f32) {
    let trunk = lit(Color::from_hex(0x4a3a3a), sk, near);
    rect_v(
        x - 4.0 * near,
        base - h * 0.45,
        8.0 * near,
        h * 0.45,
        trunk,
        shade(trunk, 0.8),
    );
    let (a, b) = tree_colors(&f.season);
    let leaves = 1.0 - f.season.snow_cover * 0.6;
    let ca = lit(a, sk, near);
    let cb = lit(b, sk, near);
    let sway = (f.t * 0.6 + k as f32).sin() * 1.5;
    for j in 0..5u32 {
        let ox = (hash01(k * 11 + j) - 0.5) * h * 0.55 + sway;
        let oy = -h * (0.55 + hash01(k * 11 + j + 5) * 0.35);
        let r = h * (0.22 + hash01(k * 11 + j + 9) * 0.12);
        ellipse_v(
            x + ox,
            base + oy,
            r,
            r * 0.86,
            alpha(ca, leaves),
            alpha(cb, leaves),
        );
    }
    // bare branches show through when the leaves are gone
    if leaves < 0.9 {
        for j in 0..4u32 {
            let ang = -1.2 + j as f32 * 0.8;
            let a0 = vec2(x, base - h * 0.42);
            let a1 = a0 + vec2(ang.sin(), -ang.cos().abs()) * h * 0.45;
            draw_line(a0.x, a0.y, a1.x, a1.y, 1.6 * near, trunk);
        }
    }
    if f.season.snow_cover > 0.0 {
        ellipse(
            x + sway,
            base - h * 0.86,
            h * 0.3,
            h * 0.07,
            alpha(Color::from_hex(0xe9e5f8), f.season.snow_cover),
        );
    }
}

fn draw_park_back(view: Rect, f: &DawnFrame, sk: &Sky) {
    let left = view.x;
    // a misty meadow to the far trees
    let meadow = lit(Color::from_hex(0x5a6a5a), sk, 0.3);
    let meadow = mix(meadow, Color::from_hex(0xd8d4ee), f.season.snow_cover * 0.7);
    rect_v(
        view.x,
        HORIZON_Y + 4.0,
        view.w,
        PAVE_Y - HORIZON_Y,
        mix(meadow, sk.haze, 0.5),
        meadow,
    );
    for i in 0..50u32 {
        let lx = i as f32 * 70.0 + hash01(i) * 40.0;
        let x = px(lx, left, 0.35);
        if x < view.x - 80.0 || x > view.right() + 80.0 {
            continue;
        }
        let h = 60.0 + hash01(i + 3) * 40.0;
        draw_tree(x, HORIZON_Y + 24.0, h, i, f, sk, 0.35);
    }
    // the pond with the sky in it
    let pond_x = px(1150.0, left, 0.6);
    let py = PAVE_Y - 22.0;
    ellipse_v(
        pond_x,
        py,
        210.0,
        16.0,
        mix(sk.horizon, sk.mid, 0.25),
        mix(sk.mid, sk.top, 0.5),
    );
    for i in 0..10 {
        let y = py - 8.0 + i as f32 * 1.8;
        let w = 30.0 + (f.t * 0.8 + i as f32).sin() * 10.0;
        draw_line(
            pond_x - w + 40.0,
            y,
            pond_x + w + 40.0,
            y,
            0.8,
            alpha(WHITE, 0.15),
        );
    }
    if f.fog > 0.0 {
        rect_v(
            view.x,
            HORIZON_Y,
            view.w,
            70.0,
            alpha(sk.haze, 0.6 * f.fog),
            alpha(sk.haze, 0.0),
        );
    }
}

/// Facades of a far street row (`p` = their parallax).
fn draw_street_back(view: Rect, f: &DawnFrame, sk: &Sky, p: f32) {
    let left = view.x;
    let base = PAVE_Y - 4.0;
    let start = ((left * p - 200.0) / 120.0).floor() as i32;
    let end = ((left * p + view.w + 200.0) / 120.0).ceil() as i32;
    for i in start..end {
        let k = i.unsigned_abs() * 13 + if i < 0 { 5 } else { 0 };
        let lx = i as f32 * 120.0;
        let x = px(lx, left, p);
        let w = 118.0;
        let h = 120.0 + hash01(k) * 40.0;
        let pal = [0x9a8fb8, 0x8a7aa8, 0xa08a9a, 0x7f8aa8];
        let col = mix(
            lit(Color::from_hex(pal[(k % 4) as usize]), sk, 0.35),
            sk.haze,
            0.35,
        );
        rect_v(x, base - h, w, h, col, shade(col, 0.85));
        rect_v(
            x - 2.0,
            base - h - 4.0,
            w + 4.0,
            5.0,
            shade(col, 0.8),
            shade(col, 0.8),
        );
        if f.season.snow_cover > 0.0 {
            rect_v(
                x - 2.0,
                base - h - 7.0,
                w + 4.0,
                4.0,
                alpha(Color::from_hex(0xe0dcf2), f.season.snow_cover),
                alpha(Color::from_hex(0xc8c2e6), f.season.snow_cover),
            );
        }
        for row in 0..4 {
            for colm in 0..4 {
                let wx = x + 12.0 + colm as f32 * 26.0;
                let wy = base - h + 14.0 + row as f32 * 28.0;
                if wy > base - 20.0 {
                    continue;
                }
                let on = hash01(k * 31 + row * 4 + colm) > 0.78;
                let wc = if on {
                    alpha(c::AMBER, 0.75 * (1.0 - f.light * 0.5))
                } else {
                    mix(shade(col, 0.7), sk.horizon, 0.25)
                };
                draw_rectangle(wx, wy, 12.0, 16.0, wc);
            }
        }
    }
}

// ───────────────────────── the near row ─────────────────────────

fn draw_near_back(view: Rect, f: &DawnFrame, sk: &Sky) {
    let left = view.x;
    match f.route {
        Some(DawnRoute::Embankment) => {
            // the railing along the river
            let rail = lit(Color::from_hex(0x3a3452), sk, 0.9);
            let y = PAVE_Y - 30.0;
            draw_line(view.x, y, view.right(), y, 3.0, rail);
            draw_line(view.x, y + 12.0, view.right(), y + 12.0, 1.5, rail);
            let start = (view.x / 40.0).floor() as i32;
            for i in start..start + (view.w / 40.0) as i32 + 2 {
                let x = i as f32 * 40.0;
                rect_v(x - 1.5, y, 3.0, 30.0, rail, rail);
            }
            if f.season.snow_cover > 0.0 {
                draw_line(
                    view.x,
                    y - 2.0,
                    view.right(),
                    y - 2.0,
                    2.5,
                    alpha(Color::from_hex(0xeae6fa), f.season.snow_cover),
                );
            }
        }
        Some(DawnRoute::Park) => {
            for i in 0..14u32 {
                let x = 500.0 + i as f32 * 150.0 + hash01(i * 3) * 60.0;
                if x < view.x - 100.0 || x > view.right() + 100.0 || x > home_x(f.route) - 260.0 {
                    continue;
                }
                draw_tree(
                    x,
                    PAVE_Y + 2.0,
                    150.0 + hash01(i) * 50.0,
                    i + 100,
                    f,
                    sk,
                    1.0,
                );
            }
            // a bench under the trees
            for bx in [900.0, 1460.0] {
                draw_bench(bx, PAVE_Y + 4.0, f, sk);
            }
        }
        Some(DawnRoute::Bakery) | Some(DawnRoute::Tram) | None => {
            draw_houses(view, f, sk, 430.0, home_x(f.route) - 200.0);
            if f.route == Some(DawnRoute::Bakery) {
                draw_bakery(1180.0, f, sk);
            }
        }
    }
    let _ = left;
}

fn draw_bench(x: f32, base: f32, f: &DawnFrame, sk: &Sky) {
    let wood = lit(c::WOOD, sk, 1.0);
    let iron = lit(Color::from_hex(0x2c2a3e), sk, 1.0);
    rect_v(x - 34.0, base - 22.0, 68.0, 5.0, wood, shade(wood, 0.8));
    rect_v(x - 34.0, base - 40.0, 68.0, 4.0, wood, shade(wood, 0.8));
    rect_v(x - 34.0, base - 33.0, 68.0, 4.0, wood, shade(wood, 0.8));
    for dx in [-28.0, 28.0] {
        rect_v(x + dx - 1.5, base - 40.0, 3.0, 40.0, iron, iron);
    }
    if f.season.snow_cover > 0.0 {
        rect_v(
            x - 35.0,
            base - 25.0,
            70.0,
            3.0,
            alpha(Color::from_hex(0xeae6fa), f.season.snow_cover),
            alpha(Color::from_hex(0xd0caee), f.season.snow_cover),
        );
    }
}

/// A row of Holešovice tenements behind the pavement (pastel, four storeys, lit windows).
fn draw_houses(view: Rect, f: &DawnFrame, sk: &Sky, from: f32, to: f32) {
    let pal = [0xd9b86a, 0x9cc4b0, 0xd99a9a, 0xb0a4d9, 0xe0c9a0, 0x8fb0c9];
    let mut x = from;
    let mut i = 0u32;
    while x < to {
        let w = 150.0 + hash01(i * 5 + 1) * 70.0;
        if x + w > view.x - 20.0 && x < view.right() + 20.0 {
            let h = 165.0 + hash01(i * 5 + 2) * 60.0;
            let base = Color::from_hex(pal[(hash(i * 7 + 3) % pal.len() as u32) as usize]);
            draw_house(x, w, h, base, i, f, sk);
        }
        x += w;
        i += 1;
    }
}

fn draw_house(x: f32, w: f32, h: f32, base: Color, k: u32, f: &DawnFrame, sk: &Sky) {
    let top = PAVE_Y - h;
    let col = lit(base, sk, 1.0);
    rect_v(x, top, w, h, mix(col, sk.key, 0.12), shade(col, 0.78));
    // cornice and roof
    let cor = shade(col, 0.72);
    rect_v(x - 3.0, top - 6.0, w + 6.0, 7.0, mix(cor, sk.key, 0.2), cor);
    let roof = lit(Color::from_hex(0x6a4a5a), sk, 0.8);
    poly_v(
        &[
            vec2(x - 3.0, top - 6.0),
            vec2(x + w + 3.0, top - 6.0),
            vec2(x + w - 18.0, top - 30.0),
            vec2(x + 18.0, top - 30.0),
        ],
        mix(roof, sk.key, 0.15),
        roof,
    );
    // a chimney
    let cx = x + 30.0 + hash01(k * 3) * (w - 60.0);
    rect_v(cx, top - 44.0, 10.0, 20.0, roof, shade(roof, 0.8));
    if f.season.snow_cover > 0.0 {
        let s = f.season.snow_cover;
        poly(
            &[
                vec2(x + 16.0, top - 30.0),
                vec2(x + w - 16.0, top - 30.0),
                vec2(x + w - 10.0, top - 22.0),
                vec2(x + 10.0, top - 22.0),
            ],
            alpha(Color::from_hex(0xe6e2f6), s),
        );
    }
    // windows: 2–3 floors above a ground floor
    let cols = ((w - 20.0) / 34.0).floor().max(2.0) as u32;
    let gap = (w - 20.0) / cols as f32;
    let rows = (((h - 74.0) / 44.0).floor() as u32).clamp(2, 3);
    for row in 0..rows {
        for cc in 0..cols {
            let wx = x + 10.0 + cc as f32 * gap + (gap - 16.0) / 2.0;
            let wy = top + 18.0 + row as f32 * 44.0;
            let on = hash01(k * 97 + row * 13 + cc) > 0.72;
            let frame = shade(col, 0.6);
            rect_v(wx - 2.0, wy - 2.0, 20.0, 28.0, frame, frame);
            if on {
                let a = 0.9 - f.light * 0.35;
                rect_v(
                    wx,
                    wy,
                    16.0,
                    24.0,
                    alpha(c::AMBER, a),
                    alpha(c::AMBER_DEEP, a),
                );
                glow(wx + 8.0, wy + 12.0, 22.0, alpha(c::AMBER, 0.18 * a));
            } else {
                rect_v(
                    wx,
                    wy,
                    16.0,
                    24.0,
                    mix(sk.horizon, sk.mid, 0.4),
                    mix(sk.mid, sk.top, 0.6),
                );
                draw_line(wx + 8.0, wy, wx + 8.0, wy + 24.0, 1.0, frame);
            }
            // geraniums on some sills
            if hash01(k * 41 + row * 3 + cc) > 0.8 {
                rect_v(
                    wx - 2.0,
                    wy + 25.0,
                    20.0,
                    4.0,
                    lit(c::POT, sk, 1.0),
                    lit(c::POT, sk, 0.8),
                );
                for j in 0..4 {
                    ellipse(
                        wx + 1.0 + j as f32 * 5.0,
                        wy + 24.0,
                        2.6,
                        2.2,
                        lit(Color::from_hex(0xd9485a), sk, 1.0),
                    );
                }
            }
        }
    }
    // the ground floor: a gate
    let gx = x + w * 0.5 - 14.0;
    let gate = lit(Color::from_hex(0x4a3a4e), sk, 0.9);
    rrect_top(gx, PAVE_Y - 52.0, 28.0, 52.0, 12.0, gate, shade(gate, 0.75));
}

/// The corner bakery: a striped awning, a warm window with loaves, a pretzel sign, steam.
fn draw_bakery(x: f32, f: &DawnFrame, sk: &Sky) {
    let w = 200.0;
    let h = 215.0;
    let top = PAVE_Y - h;
    let col = lit(Color::from_hex(0xe8d2a8), sk, 1.0);
    rect_v(x, top, w, h, mix(col, sk.key, 0.15), shade(col, 0.8));
    rect_v(
        x - 3.0,
        top - 6.0,
        w + 6.0,
        7.0,
        shade(col, 0.7),
        shade(col, 0.6),
    );
    // upper windows (the baker's flat is awake)
    for i in 0..4 {
        let wx = x + 22.0 + i as f32 * 44.0;
        rect_v(
            wx,
            top + 22.0,
            18.0,
            26.0,
            alpha(c::AMBER, 0.85),
            alpha(c::AMBER_DEEP, 0.85),
        );
        rect_v(
            wx,
            top + 70.0,
            18.0,
            26.0,
            mix(sk.horizon, sk.mid, 0.4),
            mix(sk.mid, sk.top, 0.6),
        );
    }
    // the shop window: warm light, shelves of bread
    let sw = Rect::new(x + 14.0, PAVE_Y - 92.0, 120.0, 70.0);
    rect_v(
        sw.x - 4.0,
        sw.y - 4.0,
        sw.w + 8.0,
        sw.h + 8.0,
        lit(c::WOOD_DARK, sk, 1.0),
        lit(c::WOOD_DARK, sk, 0.8),
    );
    rect_v(
        sw.x,
        sw.y,
        sw.w,
        sw.h,
        Color::from_hex(0xffd59a),
        Color::from_hex(0xf0a35c),
    );
    for row in 0..2 {
        let sy = sw.y + 26.0 + row as f32 * 28.0;
        draw_line(sw.x, sy, sw.right(), sy, 2.0, Color::from_hex(0xa8683e));
        for j in 0..6 {
            let bx = sw.x + 12.0 + j as f32 * 19.0;
            let loaf = if (j + row) % 2 == 0 {
                0xc98a4a
            } else {
                0xb06f38
            };
            ellipse_v(
                bx,
                sy - 5.0,
                8.0,
                5.0,
                Color::from_hex(0xe0a762),
                Color::from_hex(loaf),
            );
        }
    }
    // the baker behind the glass (her counter hides her legs)
    let baker = PersonLook {
        apron: false,
        height: 0.82,
        top: Color::from_hex(0xf4efe6),
        top_style: TopStyle::Shirt,
        accessory: Accessory::Hairclip,
        ..PersonLook::random(733, false)
    };
    draw_person(
        &baker,
        &PersonPose {
            feet: vec2(sw.x + sw.w - 30.0, sw.bottom() - 2.0),
            facing: Facing::Down,
            pose: Pose::Work,
            t: f.t,
        },
    );
    rect_v(
        sw.x,
        sw.bottom() - 24.0,
        sw.w,
        24.0,
        Color::from_hex(0xc98a5a),
        Color::from_hex(0x9c6a47),
    );
    draw_line(
        sw.x,
        sw.bottom() - 24.0,
        sw.right(),
        sw.bottom() - 24.0,
        2.0,
        Color::from_hex(0xe0a772),
    );
    for j in 0..3 {
        ellipse_v(
            sw.x + 20.0 + j as f32 * 16.0,
            sw.bottom() - 28.0,
            7.0,
            4.0,
            Color::from_hex(0xe8b070),
            Color::from_hex(0xb06f38),
        );
    }
    // the glass sheen over her
    rect_v(
        sw.x,
        sw.y,
        sw.w,
        sw.h * 0.6,
        alpha(WHITE, 0.12),
        alpha(WHITE, 0.0),
    );
    rect_v(
        sw.x - 4.0,
        sw.bottom(),
        sw.w + 8.0,
        4.0,
        lit(c::WOOD_DARK, sk, 1.0),
        lit(c::WOOD_DARK, sk, 0.8),
    );
    if f.season.festive > 0.0 {
        for j in 0..4 {
            let hx = sw.x + 14.0 + j as f32 * 30.0;
            let hy = sw.y + 10.0;
            ellipse(hx - 2.5, hy, 3.5, 3.5, Color::from_hex(0xa0522d));
            ellipse(hx + 2.5, hy, 3.5, 3.5, Color::from_hex(0xa0522d));
            poly(
                &[
                    vec2(hx - 6.0, hy + 1.0),
                    vec2(hx + 6.0, hy + 1.0),
                    vec2(hx, hy + 8.0),
                ],
                Color::from_hex(0xa0522d),
            );
        }
    }
    // the door, open, light on the pavement
    let dx = sw.right() + 14.0;
    rect_v(
        dx,
        PAVE_Y - 96.0,
        40.0,
        96.0,
        Color::from_hex(0xffd59a),
        Color::from_hex(0xe89a52),
    );
    rect_v(
        dx + 30.0,
        PAVE_Y - 96.0,
        12.0,
        96.0,
        lit(c::WOOD, sk, 1.0),
        lit(c::WOOD_DARK, sk, 1.0),
    );
    glow(dx + 20.0, PAVE_Y + 14.0, 90.0, alpha(c::AMBER, 0.28));
    glow(sw.center().x, PAVE_Y + 18.0, 110.0, alpha(c::AMBER, 0.22));
    // steam from the door
    for j in 0..6 {
        let k = j as f32;
        let age = (f.t * 0.35 + k / 6.0) % 1.0;
        glow(
            dx + 18.0 + (age * 9.0 + k).sin() * 6.0,
            PAVE_Y - 96.0 - age * 60.0,
            8.0 + age * 14.0,
            alpha(WHITE, 0.16 * (1.0 - age)),
        );
    }
    // the striped awning
    let ay = PAVE_Y - 112.0;
    for i in 0..10 {
        let sx = x + 4.0 + i as f32 * 19.2;
        let col = if i % 2 == 0 {
            lit(Color::from_hex(0xc0443e), sk, 1.0)
        } else {
            lit(Color::from_hex(0xf3e6d0), sk, 1.0)
        };
        quad(
            [
                vec2(sx, ay),
                vec2(sx + 19.2, ay),
                vec2(sx + 23.0, ay + 18.0),
                vec2(sx + 3.8, ay + 18.0),
            ],
            [col, col, shade(col, 0.85), shade(col, 0.85)],
        );
        ellipse(sx + 13.4, ay + 18.0, 9.6, 3.5, shade(col, 0.85));
    }
    if f.season.snow_cover > 0.0 {
        rect_v(
            x + 2.0,
            ay - 3.0,
            196.0,
            4.0,
            alpha(Color::from_hex(0xeae6fa), f.season.snow_cover),
            alpha(Color::from_hex(0xd2ccef), f.season.snow_cover),
        );
    }
    // the hanging pretzel sign
    let px0 = x + w - 10.0;
    draw_line(
        px0,
        ay - 10.0,
        px0 + 26.0,
        ay - 10.0,
        2.0,
        lit(Color::from_hex(0x2c2a3e), sk, 1.0),
    );
    let pc = vec2(px0 + 20.0, ay + 6.0);
    let gold = Color::from_hex(0xe8b04b);
    draw_line(pc.x, ay - 10.0, pc.x, pc.y - 10.0, 1.0, gold);
    arc(
        pc + vec2(-5.0, 0.0),
        6.0,
        7.0,
        0.0,
        std::f32::consts::TAU,
        2.4,
        gold,
    );
    arc(
        pc + vec2(5.0, 0.0),
        6.0,
        7.0,
        0.0,
        std::f32::consts::TAU,
        2.4,
        gold,
    );
    glow(pc.x, pc.y, 16.0, alpha(gold, 0.25));
}

fn draw_tram_stop(view: Rect, f: &DawnFrame, sk: &Sky, x: f32) {
    if x < view.x - 100.0 || x > view.right() + 100.0 {
        return;
    }
    let iron = lit(Color::from_hex(0x2c2a3e), sk, 1.0);
    // the shelter: a glass box with a roof
    let glass = alpha(mix(sk.horizon, WHITE, 0.3), 0.18);
    rect_v(
        x - 50.0,
        PAVE_Y - 80.0,
        100.0,
        80.0,
        glass,
        alpha(glass, 0.05),
    );
    rect_v(x - 56.0, PAVE_Y - 86.0, 112.0, 7.0, iron, iron);
    for dx in [-50.0, 50.0] {
        rect_v(x + dx - 1.5, PAVE_Y - 80.0, 3.0, 80.0, iron, iron);
    }
    if f.season.snow_cover > 0.0 {
        rect_v(
            x - 56.0,
            PAVE_Y - 90.0,
            112.0,
            4.0,
            alpha(Color::from_hex(0xeae6fa), f.season.snow_cover),
            alpha(Color::from_hex(0xd2ccef), f.season.snow_cover),
        );
    }
    // the sign post with the green-and-white stop sign and a small timetable screen
    rect_v(x + 66.0, PAVE_Y - 120.0, 3.0, 120.0, iron, iron);
    rrect_v(
        x + 56.0,
        PAVE_Y - 132.0,
        24.0,
        18.0,
        3.0,
        Color::from_hex(0xf3efe6),
        Color::from_hex(0xd8d2c8),
    );
    rrect_v(
        x + 59.0,
        PAVE_Y - 129.0,
        18.0,
        6.0,
        1.5,
        Color::from_hex(0x3f8a6a),
        Color::from_hex(0x2f6a4f),
    );
    rrect_v(
        x - 30.0,
        PAVE_Y - 70.0,
        34.0,
        20.0,
        2.0,
        Color::from_hex(0x1b1a2e),
        Color::from_hex(0x24223a),
    );
    world_text("17", vec2(x - 25.0, PAVE_Y - 56.0), 10.0, c::AMBER, false);
    world_text(
        "2'",
        vec2(x - 10.0, PAVE_Y - 56.0),
        10.0,
        alpha(c::AMBER, 0.7),
        false,
    );
}

/// Overhead tram wires and poles (tram route).
fn draw_wires(view: Rect, f: &DawnFrame, sk: &Sky) {
    let iron = alpha(lit(Color::from_hex(0x2c2a3e), sk, 1.0), 0.9);
    let y = 154.0;
    draw_line(view.x, y, view.right(), y, 1.2, iron);
    draw_line(
        view.x,
        y + 6.0,
        view.right(),
        y + 6.0,
        0.8,
        alpha(iron, 0.6),
    );
    let start = (view.x / 360.0).floor() as i32;
    for i in start..start + (view.w / 360.0) as i32 + 2 {
        let x = i as f32 * 360.0 + 120.0;
        rect_v(x - 2.5, y - 20.0, 5.0, PAVE_Y - y + 20.0, iron, iron);
        draw_line(x, y - 14.0, x - 60.0, y, 1.0, iron);
    }
    let _ = f;
}

/// A Prague tram (cream and burgundy, warm windows) whose front is at `x`.
fn draw_tram(x: f32, f: &DawnFrame, sk: &Sky) {
    let len = 380.0;
    let bottom = PAVE_Y - 2.0;
    let top = bottom - 104.0;
    let x0 = x - len;
    // shadow on the rails
    soft_shadow(x - len / 2.0, bottom + 2.0, len * 0.55, 6.0, 0.5);
    let red = lit(Color::from_hex(0xb8323e), sk, 1.0);
    let cream = lit(Color::from_hex(0xf3e6d0), sk, 1.0);
    rrect_v(x0, top + 54.0, len, 46.0, 6.0, red, shade(red, 0.75));
    rrect_v(x0, top, len, 58.0, 10.0, cream, shade(cream, 0.9));
    // the windows, warmly lit, with a few passengers
    let n = 8;
    for i in 0..n {
        let wx = x0 + 16.0 + i as f32 * ((len - 40.0) / n as f32);
        let wr = Rect::new(wx, top + 10.0, 34.0, 34.0);
        rrect_v(
            wr.x,
            wr.y,
            wr.w,
            wr.h,
            4.0,
            Color::from_hex(0xffd59a),
            Color::from_hex(0xf0a35c),
        );
        if hash01(i as u32 * 7 + 3) > 0.6 {
            ellipse(
                wr.center().x,
                wr.y + 18.0,
                6.0,
                7.0,
                alpha(Color::from_hex(0x5a3f4e), 0.75),
            );
            rect_v(
                wr.center().x - 9.0,
                wr.y + 24.0,
                18.0,
                10.0,
                alpha(Color::from_hex(0x5a3f4e), 0.75),
                alpha(Color::from_hex(0x5a3f4e), 0.75),
            );
        }
        rect_v(wr.x, wr.y, wr.w, 8.0, alpha(WHITE, 0.25), alpha(WHITE, 0.0));
        if f.season.snow_cover > 0.0 {
            rect_v(
                wr.x,
                wr.y + wr.h - 3.0,
                wr.w,
                3.0,
                alpha(WHITE, 0.4),
                alpha(WHITE, 0.0),
            );
        }
    }
    // doors
    for dx in [0.3, 0.7] {
        let dxp = x0 + len * dx;
        rect_v(
            dxp - 14.0,
            top + 8.0,
            28.0,
            88.0,
            shade(red, 0.7),
            shade(red, 0.55),
        );
        rect_v(
            dxp - 11.0,
            top + 12.0,
            9.0,
            36.0,
            Color::from_hex(0xffd59a),
            Color::from_hex(0xf0a35c),
        );
        rect_v(
            dxp + 2.0,
            top + 12.0,
            9.0,
            36.0,
            Color::from_hex(0xffd59a),
            Color::from_hex(0xf0a35c),
        );
    }
    // the front: a rounded nose with the route number and a headlight
    rrect_v(
        x - 22.0,
        top + 2.0,
        26.0,
        98.0,
        10.0,
        shade(cream, 0.95),
        shade(red, 0.8),
    );
    rrect_v(
        x - 18.0,
        top + 6.0,
        18.0,
        12.0,
        2.0,
        Color::from_hex(0x1b1a2e),
        Color::from_hex(0x24223a),
    );
    world_text("17", vec2(x - 15.0, top + 16.0), 9.0, c::AMBER, false);
    glow(
        x - 2.0,
        bottom - 22.0,
        26.0,
        alpha(Color::from_hex(0xfff1d6), 0.6),
    );
    glow(
        x + 40.0,
        bottom - 10.0,
        60.0,
        alpha(Color::from_hex(0xfff1d6), 0.12),
    );
    // the bogies
    for dx in [0.15, 0.5, 0.85] {
        let bx = x0 + len * dx;
        rrect_v(
            bx - 24.0,
            bottom - 6.0,
            48.0,
            8.0,
            3.0,
            Color::from_hex(0x2c2a3e),
            Color::from_hex(0x1b1a2e),
        );
    }
    // the pantograph to the wire
    let px0 = x0 + len * 0.45;
    let iron = Color::from_hex(0x2c2a3e);
    draw_line(px0 - 20.0, top, px0, 158.0, 1.6, iron);
    draw_line(px0 + 20.0, top, px0, 158.0, 1.6, iron);
    draw_line(px0 - 14.0, 156.0, px0 + 14.0, 156.0, 2.0, iron);
    if f.season.snow_cover > 0.0 {
        rrect_v(
            x0 + 6.0,
            top - 3.0,
            len - 12.0,
            5.0,
            2.0,
            alpha(Color::from_hex(0xeae6fa), f.season.snow_cover),
            alpha(Color::from_hex(0xd2ccef), f.season.snow_cover),
        );
    }
    // a soft warm spill from the windows onto the pavement
    for i in 0..4 {
        glow(
            x0 + 60.0 + i as f32 * 90.0,
            bottom + 18.0,
            60.0,
            alpha(c::AMBER, 0.12),
        );
    }
}

// ───────────────────────── the café and home ─────────────────────────

fn draw_cafe(view: Rect, f: &DawnFrame, sk: &Sky) {
    if view.x > 460.0 {
        return;
    }
    let w = 430.0;
    let h = 250.0;
    let top = PAVE_Y - h;
    let wall = lit(c::WALL_PLASTER_TOP, sk, 1.0);
    rect_v(
        -60.0,
        top,
        w + 60.0,
        h,
        mix(wall, sk.key, 0.1),
        shade(wall, 0.75),
    );
    rect_v(
        -60.0,
        top - 8.0,
        w + 66.0,
        9.0,
        lit(c::WALL_CAP_EDGE, sk, 1.0),
        lit(c::WALL_CAP, sk, 1.0),
    );
    // upper floor windows: dark (the flat over the café is empty)
    for i in 0..5 {
        let wx = 10.0 + i as f32 * 82.0;
        rect_v(
            wx,
            top + 22.0,
            26.0,
            36.0,
            mix(sk.horizon, sk.mid, 0.5),
            mix(sk.mid, sk.top, 0.6),
        );
    }
    // the big café window: lamps off, chairs up on the tables, one lamp still glowing
    let win = Rect::new(24.0, PAVE_Y - 150.0, 176.0, 120.0);
    rect_v(
        win.x - 5.0,
        win.y - 5.0,
        win.w + 10.0,
        win.h + 10.0,
        lit(c::WOOD_PLUM, sk, 1.0),
        lit(c::WOOD_PLUM, sk, 0.8),
    );
    rect_v(
        win.x,
        win.y,
        win.w,
        win.h,
        Color::from_hex(0x3a2a4e),
        Color::from_hex(0x2a1f3a),
    );
    glow(win.x + 40.0, win.y + 40.0, 60.0, alpha(c::AMBER, 0.35));
    for j in 0..3 {
        let tx = win.x + 30.0 + j as f32 * 56.0;
        let ty = win.bottom() - 30.0;
        let s = Color::from_hex(0x1f1730);
        rect_v(tx - 16.0, ty, 32.0, 4.0, s, s);
        rect_v(tx - 2.0, ty, 4.0, 30.0, s, s);
        // a chair upside down on the table
        draw_line(tx - 10.0, ty, tx - 10.0, ty - 16.0, 2.0, s);
        draw_line(tx + 6.0, ty, tx + 6.0, ty - 16.0, 2.0, s);
        rect_v(tx - 12.0, ty - 4.0, 20.0, 3.0, s, s);
    }
    rect_v(
        win.x,
        win.y,
        win.w,
        30.0,
        alpha(WHITE, 0.1),
        alpha(WHITE, 0.0),
    );
    draw_line(
        win.center().x,
        win.y,
        win.center().x,
        win.bottom(),
        2.0,
        lit(c::WOOD_PLUM, sk, 1.0),
    );
    // the sign, switched off but still readable
    let sr = Rect::new(36.0, top + 70.0, 150.0, 22.0);
    rrect_v(
        sr.x,
        sr.y,
        sr.w,
        sr.h,
        6.0,
        Color::from_hex(0x2a2048),
        Color::from_hex(0x201634),
    );
    world_text(
        "Café Nocturne",
        vec2(sr.center().x, sr.y + 15.5),
        12.0,
        alpha(c::INDIGO_SOFT, 0.75),
        true,
    );
    // the door
    let d = Rect::new(CAFE_DOOR_X - 26.0, PAVE_Y - 104.0, 52.0, 104.0);
    rect_v(
        d.x - 5.0,
        d.y - 5.0,
        d.w + 10.0,
        d.h + 5.0,
        lit(c::WOOD_PLUM, sk, 1.0),
        lit(c::WOOD_PLUM, sk, 0.8),
    );
    rect_v(
        d.x,
        d.y,
        d.w,
        d.h,
        lit(c::WOOD_DARK, sk, 1.0),
        shade(lit(c::WOOD_DARK, sk, 1.0), 0.8),
    );
    rect_v(
        d.x + 10.0,
        d.y + 10.0,
        d.w - 20.0,
        34.0,
        Color::from_hex(0x2a1f3a),
        Color::from_hex(0x3a2a4e),
    );
    draw_circle(d.right() - 9.0, d.y + 58.0, 2.5, Color::from_hex(0xe8b04b));
    // the lamp over the door, still on
    let on = 1.0 - smooth(0.6, 1.0, f.light);
    glow(CAFE_DOOR_X, d.y - 14.0, 40.0, alpha(c::AMBER, 0.55 * on));
    draw_circle(
        CAFE_DOOR_X,
        d.y - 14.0,
        4.0,
        mix(Color::from_hex(0x6a5a7a), Color::from_hex(0xfff1d6), on),
    );
    // a little "closed" sign on the door
    rrect_v(
        d.x + 12.0,
        d.y + 18.0,
        d.w - 24.0,
        12.0,
        2.0,
        Color::from_hex(0xf3e6d0),
        Color::from_hex(0xd8cdb8),
    );
    // the café's planter and a garland in the festive weeks
    if f.season.festive > 0.0 {
        for i in 0..14 {
            let gx = win.x - 4.0 + i as f32 * 13.5;
            let gy = win.y - 10.0 + ((i as f32) * 0.9).sin() * 2.0;
            let col = [0xffc46b, 0xf0607a, 0xa5b4fc, 0x10b981][i % 4];
            glow(
                gx,
                gy,
                5.0,
                alpha(Color::from_hex(col), 0.9 * f.season.festive),
            );
        }
    }
    if f.season.snow_cover > 0.0 {
        rect_v(
            win.x - 6.0,
            win.bottom() + 3.0,
            win.w + 12.0,
            4.0,
            alpha(Color::from_hex(0xeae6fa), f.season.snow_cover),
            alpha(Color::from_hex(0xd2ccef), f.season.snow_cover),
        );
    }
    // the station's edge to the left: a hint of the platform canopy
    let canopy = lit(Color::from_hex(0x4a4060), sk, 0.8);
    rect_v(
        -60.0,
        PAVE_Y - 200.0,
        30.0,
        200.0,
        canopy,
        shade(canopy, 0.7),
    );
}

fn draw_home(view: Rect, f: &DawnFrame, sk: &Sky) {
    let hx = home_x(f.route);
    if hx + 300.0 < view.x || hx - 260.0 > view.right() {
        return;
    }
    let x = hx - 200.0;
    let w = 500.0;
    let h = 260.0;
    let top = PAVE_Y - h;
    let col = lit(Color::from_hex(0xa8c9b4), sk, 1.0);
    rect_v(x, top, w, h, mix(col, sk.key, 0.14), shade(col, 0.78));
    rect_v(
        x - 3.0,
        top - 6.0,
        w + 6.0,
        7.0,
        shade(col, 0.7),
        shade(col, 0.6),
    );
    let roof = lit(Color::from_hex(0x6a4a5a), sk, 0.8);
    poly_v(
        &[
            vec2(x - 3.0, top - 6.0),
            vec2(x + w + 3.0, top - 6.0),
            vec2(x + w - 24.0, top - 34.0),
            vec2(x + 24.0, top - 34.0),
        ],
        mix(roof, sk.key, 0.15),
        roof,
    );
    if f.season.snow_cover > 0.0 {
        poly(
            &[
                vec2(x + 22.0, top - 34.0),
                vec2(x + w - 22.0, top - 34.0),
                vec2(x + w - 14.0, top - 25.0),
                vec2(x + 14.0, top - 25.0),
            ],
            alpha(Color::from_hex(0xe6e2f6), f.season.snow_cover),
        );
    }
    // windows; hers (second floor, over the door) lights up when she's in
    for row in 0..3u32 {
        for cc in 0..7u32 {
            let wx = x + 30.0 + cc as f32 * 66.0;
            let wy = top + 20.0 + row as f32 * 50.0;
            let frame = shade(col, 0.6);
            rect_v(wx - 2.0, wy - 2.0, 24.0, 32.0, frame, frame);
            let hers = row == 1 && cc == 3;
            let lit_k = if hers {
                f.window_lit
            } else if hash01(row * 9 + cc + 5) > 0.82 {
                0.8 - f.light * 0.4
            } else {
                0.0
            };
            rect_v(
                wx,
                wy,
                20.0,
                28.0,
                mix(sk.horizon, sk.mid, 0.4),
                mix(sk.mid, sk.top, 0.6),
            );
            if lit_k > 0.0 {
                rect_v(
                    wx,
                    wy,
                    20.0,
                    28.0,
                    alpha(c::AMBER, lit_k),
                    alpha(c::AMBER_DEEP, lit_k),
                );
                glow(wx + 10.0, wy + 14.0, 30.0, alpha(c::AMBER, 0.25 * lit_k));
            }
            draw_line(wx + 10.0, wy, wx + 10.0, wy + 28.0, 1.0, frame);
            if hers {
                // her flower box (grandma's geraniums)
                rect_v(
                    wx - 4.0,
                    wy + 29.0,
                    28.0,
                    5.0,
                    lit(c::POT, sk, 1.0),
                    lit(c::POT, sk, 0.8),
                );
                for j in 0..5 {
                    ellipse(
                        wx - 1.0 + j as f32 * 5.5,
                        wy + 28.0,
                        2.8,
                        2.4,
                        lit(Color::from_hex(0xd9485a), sk, 1.0),
                    );
                }
            }
        }
    }
    // the green front door with a fanlight and the number
    let d = Rect::new(hx - 28.0, PAVE_Y - 108.0, 56.0, 108.0);
    let green = lit(Color::from_hex(0x3f7a5e), sk, 1.0);
    rect_v(
        d.x - 6.0,
        d.y - 22.0,
        d.w + 12.0,
        d.h + 22.0,
        shade(col, 0.65),
        shade(col, 0.55),
    );
    rrect_top(
        d.x,
        d.y - 16.0,
        d.w,
        d.h + 16.0,
        26.0,
        green,
        shade(green, 0.75),
    );
    ellipse(
        d.center().x,
        d.y - 2.0,
        20.0,
        12.0,
        mix(Color::from_hex(0xffd59a), sk.horizon, 0.4),
    );
    draw_line(
        d.center().x,
        d.y + 8.0,
        d.center().x,
        d.bottom(),
        1.5,
        shade(green, 0.6),
    );
    draw_circle(
        d.center().x - 6.0,
        d.y + 58.0,
        2.5,
        Color::from_hex(0xe8b04b),
    );
    draw_circle(
        d.center().x + 6.0,
        d.y + 58.0,
        2.5,
        Color::from_hex(0xe8b04b),
    );
    rrect_v(
        d.right() + 12.0,
        d.y + 20.0,
        18.0,
        13.0,
        2.0,
        Color::from_hex(0x2f5a8a),
        Color::from_hex(0x24466e),
    );
    world_text("12", vec2(d.right() + 21.0, d.y + 30.0), 9.0, WHITE, true);
    // the step
    rect_v(
        d.x - 10.0,
        PAVE_Y - 4.0,
        d.w + 20.0,
        6.0,
        lit(Color::from_hex(0x9a90a8), sk, 1.0),
        lit(Color::from_hex(0x7a7088), sk, 1.0),
    );
}

// ───────────────────────── the pavement ─────────────────────────

fn draw_pavement(view: Rect, f: &DawnFrame, sk: &Sky) {
    let wet = f.season.wet.max((f.rain * 0.8).min(1.0));
    let stone = lit(Color::from_hex(0x6a6080), sk, 0.9);
    let dark = shade(stone, 0.55);
    rect_v(
        view.x,
        PAVE_Y,
        view.w,
        VIEW_H - PAVE_Y,
        shade(stone, 0.8),
        dark,
    );
    // the curb at the back
    rect_v(view.x, PAVE_Y, view.w, 4.0, mix(stone, sk.key, 0.3), stone);
    // cobbles: rows of rounded stones, the wet tops catching the sky
    let sheen = mix(sk.horizon, sk.mid, 0.3);
    let mut row = 0u32;
    let mut y = PAVE_Y + 8.0;
    while y < VIEW_H + 6.0 {
        let k = (y - PAVE_Y) / (VIEW_H - PAVE_Y);
        let sw = 12.0 + k * 6.0;
        let sh = 3.4 + k * 2.2;
        let off = if row.is_multiple_of(2) { 0.0 } else { sw * 0.5 };
        let start = ((view.x - off) / sw).floor() as i32;
        let end = ((view.right() - off) / sw).ceil() as i32 + 1;
        for i in start..end {
            let x = i as f32 * sw + off;
            let h = hash2(i, row as i32);
            let col = mix(stone, dark, 0.15 + h * 0.4);
            ellipse(x, y, sw * 0.44, sh, col);
            if wet > 0.0 && h > 0.35 {
                ellipse(
                    x - sw * 0.06,
                    y - sh * 0.35,
                    sw * 0.26,
                    sh * 0.32,
                    alpha(sheen, (0.18 + 0.22 * f.light) * wet),
                );
            }
        }
        y += sh * 2.0 + 1.2;
        row += 1;
    }
    // puddles mirroring the dawn
    if wet > 0.25 && f.season.snow_cover < 0.5 {
        for i in 0..12u32 {
            let x = 300.0 + i as f32 * 330.0 + hash01(i) * 120.0;
            if x < view.x - 80.0 || x > view.right() + 80.0 {
                continue;
            }
            let y = GROUND_Y + 14.0 + hash01(i + 7) * 40.0;
            let rx = 34.0 + hash01(i + 3) * 30.0;
            ellipse_v(
                x,
                y,
                rx,
                6.0 + hash01(i + 4) * 3.0,
                alpha(sk.horizon, 0.75 * wet),
                alpha(sk.mid, 0.75 * wet),
            );
            draw_line(
                x - rx * 0.5,
                y - 1.0,
                x + rx * 0.3,
                y - 1.0,
                1.0,
                alpha(WHITE, 0.25 * wet),
            );
            if f.rain > 0.0 {
                for j in 0..3u32 {
                    let a = ((f.t * 1.4 + hash01(i * 3 + j) * 3.0) % 1.0).max(0.0);
                    let rx2 = a * 10.0;
                    draw_ellipse_lines(
                        x + (hash01(i * 5 + j) - 0.5) * rx,
                        y,
                        rx2,
                        rx2 * 0.3,
                        0.0,
                        0.8,
                        alpha(WHITE, 0.4 * (1.0 - a)),
                    );
                }
            }
        }
    }
    // wet leaves stuck to the stones
    if f.season.leaves > 0.05 {
        let n = (view.w / 30.0 * f.season.leaves) as i32;
        for i in 0..n {
            let lx = ((view.x / 30.0).floor() as i32 + i) as f32 * 30.0;
            let k = lx as i32 as u32;
            if hash01(k) > 0.55 {
                continue;
            }
            let x = lx + hash01(k + 1) * 30.0;
            let y = PAVE_Y + 10.0 + hash01(k + 2) * (VIEW_H - PAVE_Y - 20.0);
            let cols = [0xe08a3a, 0xc9603a, 0xd9a440, 0x9a4a3a];
            let col = lit(Color::from_hex(cols[(hash(k) % 4) as usize]), sk, 1.0);
            let a = hash01(k + 3) * 6.0;
            let d = vec2(a.cos(), a.sin() * 0.4) * 5.0;
            poly(
                &[
                    vec2(x, y) - d,
                    vec2(x, y) + vec2(-d.y, d.x) * 0.4,
                    vec2(x, y) + d,
                    vec2(x, y) - vec2(-d.y, d.x) * 0.4,
                ],
                alpha(col, 0.85),
            );
        }
    }
    // snow lying on the pavement
    if f.season.snow_cover > 0.0 {
        let s = f.season.snow_cover;
        rect_v(
            view.x,
            PAVE_Y,
            view.w,
            VIEW_H - PAVE_Y,
            alpha(mix(Color::from_hex(0xe6e2f6), sk.key, 0.2), 0.85 * s),
            alpha(Color::from_hex(0xb8b0dc), 0.85 * s),
        );
        for i in 0..40u32 {
            let x = view.x + hash01(i * 2) * view.w;
            let y = PAVE_Y + 6.0 + hash01(i * 2 + 1) * (VIEW_H - PAVE_Y);
            ellipse(x, y, 14.0, 2.5, alpha(WHITE, 0.25 * s));
        }
    }
}

fn draw_footprints(f: &DawnFrame) {
    if f.season.snow_cover < 0.3 {
        return;
    }
    let from = CAFE_DOOR_X;
    let to = f.karolina.x - 10.0;
    let mut x = from;
    let mut i = 0;
    while x < to {
        if f.route == Some(DawnRoute::Tram) && x > TRAM_STOP_X && x < tram_home_stop(f.route) {
            x += 18.0;
            continue;
        }
        let dy = if i % 2 == 0 { -3.0 } else { 3.0 };
        ellipse(
            x,
            GROUND_Y + dy,
            4.0,
            1.8,
            Color::new(0.45, 0.4, 0.62, 0.35 * f.season.snow_cover),
        );
        x += 18.0;
        i += 1;
    }
}

fn draw_lamps(view: Rect, f: &DawnFrame, sk: &Sky) {
    let on = 1.0 - smooth(0.55, 0.95, f.light);
    let iron = lit(Color::from_hex(0x2c2a3e), sk, 1.0);
    let step = 520.0;
    let start = (view.x / step).floor() as i32;
    for i in start..start + (view.w / step) as i32 + 2 {
        let x = i as f32 * step + 470.0;
        if x > home_x(f.route) - 230.0 {
            continue;
        }
        let base = PAVE_Y + 6.0;
        rect_v(x - 3.0, base - 150.0, 6.0, 150.0, iron, shade(iron, 0.8));
        rect_v(x - 6.0, base - 18.0, 12.0, 18.0, iron, iron);
        // the old Prague lantern
        poly_v(
            &[
                vec2(x - 9.0, base - 168.0),
                vec2(x + 9.0, base - 168.0),
                vec2(x + 6.0, base - 150.0),
                vec2(x - 6.0, base - 150.0),
            ],
            mix(Color::from_hex(0x6a5a7a), Color::from_hex(0xfff1d6), on),
            mix(Color::from_hex(0x5a4a6a), c::AMBER, on),
        );
        poly(
            &[
                vec2(x - 12.0, base - 168.0),
                vec2(x + 12.0, base - 168.0),
                vec2(x, base - 178.0),
            ],
            iron,
        );
        if on > 0.0 {
            glow(x, base - 160.0, 46.0, alpha(c::AMBER, 0.5 * on));
            glow(x, base + 10.0, 70.0, alpha(c::AMBER, 0.18 * on));
        }
        if f.season.snow_cover > 0.0 {
            ellipse(
                x,
                base - 178.0,
                10.0,
                3.0,
                alpha(Color::from_hex(0xeeeaff), f.season.snow_cover),
            );
        }
        if f.season.festive > 0.0 {
            // a small wreath on the post
            arc(
                vec2(x, base - 110.0),
                8.0,
                8.0,
                0.0,
                std::f32::consts::TAU,
                3.0,
                alpha(Color::from_hex(0x2f6a4f), f.season.festive),
            );
            draw_circle(x, base - 102.0, 2.0, alpha(c::ROSE, f.season.festive));
        }
    }
}

/// Where the pigeons peck on this route.
fn pigeon_spots(route: Option<DawnRoute>) -> &'static [f32] {
    match route {
        None => &[330.0],
        Some(DawnRoute::Embankment) => &[700.0, 1500.0],
        Some(DawnRoute::Park) => &[760.0, 1300.0],
        Some(DawnRoute::Bakery) => &[980.0, 1450.0],
        Some(DawnRoute::Tram) => &[460.0, 2700.0],
    }
}

fn draw_pigeons(view: Rect, f: &DawnFrame, sk: &Sky) {
    let body = lit(Color::from_hex(0x8a8aa8), sk, 1.0);
    let neck = Color::from_hex(0x5fa88f);
    for (gi, &gx) in pigeon_spots(f.route).iter().enumerate() {
        if gx < view.x - 200.0 || gx > view.right() + 200.0 {
            continue;
        }
        // they take off when she comes close
        let fly = if f.visible {
            smooth(gx - 90.0, gx + 60.0, f.karolina.x)
        } else {
            0.0
        };
        for j in 0..5u32 {
            let k = gi as u32 * 17 + j;
            let ox = (hash01(k) - 0.5) * 70.0;
            let oy = 14.0 + hash01(k + 1) * 30.0;
            let delay = hash01(k + 2) * 0.35;
            let up = ((fly - delay) / (1.0 - delay)).clamp(0.0, 1.0);
            let e = up * up;
            let p = vec2(
                gx + ox + e * (180.0 + hash01(k + 3) * 120.0),
                GROUND_Y + oy - 20.0 - e * 260.0,
            );
            if p.y < -20.0 {
                continue;
            }
            let flip = if hash01(k + 4) > 0.5 { 1.0 } else { -1.0 };
            if up <= 0.0 {
                soft_shadow(p.x, p.y + 6.0, 7.0, 2.0, 0.3);
                let peck = ((f.t * 2.2 + hash01(k) * 5.0).sin() > 0.7) as i32 as f32;
                ellipse_v(p.x, p.y, 7.0, 4.6, mix(body, WHITE, 0.15), shade(body, 0.8));
                let head = p + vec2(flip * (6.5 + peck * 2.0), -4.0 + peck * 5.0);
                ellipse(head.x - flip * 2.0, head.y + 1.5, 2.8, 2.6, neck);
                draw_circle(head.x, head.y, 2.7, body);
                draw_line(
                    head.x + flip * 2.4,
                    head.y,
                    head.x + flip * 4.6,
                    head.y + 0.8,
                    1.0,
                    Color::from_hex(0xe0a070),
                );
                ellipse(p.x - flip * 7.0, p.y - 1.0, 3.5, 1.6, shade(body, 0.7));
            } else {
                let flap = (f.t * 18.0 + k as f32).sin();
                ellipse(p.x, p.y, 6.5, 3.6, body);
                poly(
                    &[
                        p + vec2(-2.0, -1.0),
                        p + vec2(3.0, -1.0),
                        p + vec2(-4.0, -9.0 * flap),
                    ],
                    shade(body, 0.85),
                );
                draw_circle(p.x + 6.0, p.y - 2.0, 2.4, body);
            }
        }
    }
}

fn draw_karolina(f: &DawnFrame, sk: &Sky) {
    let look = karolina_look(&f.season);
    let pose = PersonPose {
        feet: f.karolina,
        facing: f.facing,
        pose: f.pose,
        t: f.t,
    };
    soft_shadow(f.karolina.x, f.karolina.y + 2.0, 18.0, 4.5, 0.4);
    // her blurred reflection in the wet stones
    let wet = f.season.wet.max(f.rain.min(1.0)) * (1.0 - f.season.snow_cover);
    if wet > 0.2 {
        rect_v(
            f.karolina.x - 9.0,
            f.karolina.y + 2.0,
            18.0,
            34.0,
            alpha(mix(look.top, sk.horizon, 0.4), 0.28 * wet),
            alpha(look.top, 0.0),
        );
    }
    draw_person(&look, &pose);
    draw_breath(&look, &pose, f.t, f.season.breath);
    if f.rain > 0.3 && f.facing != Facing::Up {
        draw_umbrella(f.karolina, f.t);
    }
}

/// A violet umbrella over her head (rain).
fn draw_umbrella(feet: Vec2, t: f32) {
    let sway = (t * 2.0).sin() * 1.5;
    let top = feet + vec2(14.0 + sway, -116.0);
    let handle = feet + vec2(12.0, -40.0);
    draw_line(
        handle.x,
        handle.y,
        top.x,
        top.y + 4.0,
        1.6,
        Color::from_hex(0x2c2a3e),
    );
    let canopy = Color::from_hex(0x7c3aed);
    let mut pts = Vec::new();
    for i in 0..=16 {
        let a = std::f32::consts::PI + std::f32::consts::PI * i as f32 / 16.0;
        pts.push(top + vec2(a.cos() * 40.0, a.sin() * 22.0 + 12.0));
    }
    poly_v(&pts, mix(canopy, WHITE, 0.25), shade(canopy, 0.8));
    for i in 0..4 {
        let x = top.x - 40.0 + i as f32 * 20.0 + 10.0;
        ellipse(x, top.y + 12.0, 10.0, 2.5, shade(canopy, 0.75));
    }
    draw_circle(top.x, top.y - 10.0, 1.6, Color::from_hex(0x2c2a3e));
}
