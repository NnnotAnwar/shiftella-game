//! Live background of the main menu: Café Nocturne seen from the rainy street at night.
//! Wet cobbles with puddle reflections, warm windows with guests inside, the flickering holo
//! sign (one letter is cracked), a delivery drone, and every ~40 s a night train on the
//! embankment behind the houses with its light sweeping the street. Slow camera drift.
//!
//! Drawn in its own "stage" space ([`STAGE`], y down) scaled to cover the screen.

use macroquad::prelude::*;

use crate::render::colors as pal;
use crate::render::draw::{
    alpha, ellipse, glow, hash, hash01, mix, poly, quad, rect_h, rect_v, rrect, rrect_v, shade,
    soft_shadow,
};
use crate::render::{PersonLook, Weather, draw_steam};

/// Size of the painted stage.
pub const STAGE: Vec2 = Vec2::new(1600.0, 900.0);
/// Seconds between two night trains.
pub const TRAIN_EVERY: f32 = 40.0;
const TRAIN_FIRST: f32 = 5.0;
const TRAIN_SECONDS: f32 = 9.0;
const DRONE_EVERY: f32 = 23.0;

const GROUND: f32 = 760.0;
const CAFE_X0: f32 = 640.0;
const CAFE_X1: f32 = 1460.0;
const WIN: Rect = Rect {
    x: 700.0,
    y: 586.0,
    w: 400.0,
    h: 150.0,
};
const DOOR: Rect = Rect {
    x: 1160.0,
    y: 596.0,
    w: 96.0,
    h: 164.0,
};

/// The menu background. Keeps its own time; `update` it every frame.
pub struct Backdrop {
    pub t: f32,
    weather: Weather,
    guests: [PersonLook; 3],
    font: Option<Font>,
    /// New glyphs of the sign are cached between batches (see `ui::text::GlyphCache`).
    glyphs: crate::ui::GlyphCache,
}

impl Default for Backdrop {
    fn default() -> Self {
        Self::new(None)
    }
}

impl Backdrop {
    /// `font` — a bold face for the sign (e.g. `ui.fonts.face(Weight::Bold).clone()`).
    pub fn new(font: Option<Font>) -> Backdrop {
        Backdrop {
            t: 0.0,
            weather: Weather {
                intensity: 1.0,
                wind: 0.14,
            },
            guests: [
                PersonLook::preset("mirek"),
                PersonLook::preset("vera"),
                PersonLook::random(41, true),
            ],
            font,
            glyphs: Default::default(),
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.t += dt;
    }

    /// Train progress 0..1 right → left, or `None` between trains.
    pub fn train(&self) -> Option<f32> {
        let tt = self.t - TRAIN_FIRST;
        if tt < 0.0 {
            return None;
        }
        let p = (tt % TRAIN_EVERY) / TRAIN_SECONDS;
        (p <= 1.0).then_some(p)
    }

    /// The stage camera covering the screen, with a slow drift.
    pub fn camera(&self) -> (Camera2D, f32) {
        let (sw, sh) = (screen_width(), screen_height());
        let k = (sw / STAGE.x).max(sh / STAGE.y) * 1.05;
        let t = self.t;
        let drift = vec2((t * 0.045).sin() * 26.0, (t * 0.031).cos() * 9.0);
        let cam = Camera2D {
            target: STAGE / 2.0 + drift + vec2(18.0, 0.0),
            zoom: vec2(2.0 * k / sw, 2.0 * k / sh),
            ..Default::default()
        };
        (cam, k)
    }

    /// Paints the whole scene (sets its own camera).
    pub fn draw(&self) {
        let (cam, k) = self.camera();
        set_camera(&cam);
        clear_background(pal::NIGHT_0);
        let t = self.t;
        let train = self.train();
        self.sky(t);
        self.town(t);
        self.embankment(t, train);
        self.neighbours(t);
        self.cafe(t, k);
        self.street(t);
        self.lamp_post(t);
        self.drone(t);
        self.weather
            .draw_rain(Rect::new(-40.0, -20.0, STAGE.x + 80.0, STAGE.y + 40.0), t);
        // rain made visible in the lamp cone
        self.lamp_rain(t);
        if let Some(p) = train {
            self.sweep(p);
        }
        rect_v(
            -40.0,
            STAGE.y - 120.0,
            STAGE.x + 80.0,
            160.0,
            alpha(pal::NIGHT_0, 0.0),
            alpha(pal::NIGHT_0, 0.55),
        );
    }

    fn sky(&self, t: f32) {
        rect_v(
            -60.0,
            -60.0,
            STAGE.x + 120.0,
            560.0,
            Color::from_hex(0x0b0818),
            Color::from_hex(0x2a1c4a),
        );
        // moon behind thin clouds
        let moon = vec2(1240.0, 120.0);
        glow(moon.x, moon.y, 220.0, alpha(pal::LAVENDER_RIM, 0.10));
        glow(moon.x, moon.y, 70.0, alpha(pal::LAVENDER_RIM, 0.25));
        draw_circle(moon.x, moon.y, 20.0, alpha(Color::from_hex(0xeeeaff), 0.62));
        draw_circle(
            moon.x - 6.0,
            moon.y - 4.0,
            5.0,
            alpha(Color::from_hex(0xb8b0e0), 0.35),
        );
        draw_circle(
            moon.x + 7.0,
            moon.y + 6.0,
            3.5,
            alpha(Color::from_hex(0xb8b0e0), 0.3),
        );
        // stars
        for i in 0..70u32 {
            let x = hash01(i * 3 + 1) * STAGE.x;
            let y = hash01(i * 3 + 2) * 300.0;
            let tw = 0.4 + 0.6 * ((t * (0.6 + hash01(i) * 1.5) + i as f32).sin() * 0.5 + 0.5);
            draw_circle(x, y, 0.9 + hash01(i * 7) * 0.8, alpha(WHITE, 0.25 * tw));
        }
        // cloud bands drifting
        for i in 0..6 {
            let y = 60.0 + i as f32 * 48.0;
            let x = ((t * (4.0 + i as f32) + i as f32 * 300.0) % 2200.0) - 400.0;
            ellipse(
                x,
                y,
                320.0 + i as f32 * 30.0,
                22.0,
                alpha(Color::from_hex(0x3a2a5e), 0.18),
            );
        }
        // horizon glow of the town
        glow(500.0, 470.0, 520.0, alpha(pal::VIOLET, 0.16));
        glow(1100.0, 480.0, 480.0, alpha(pal::AMBER_DEEP, 0.06));
    }

    fn town(&self, t: f32) {
        // far layer
        let mut x = -60.0;
        let mut i = 0u32;
        while x < STAGE.x + 60.0 {
            let w = 50.0 + hash01(i * 5 + 3) * 80.0;
            let h = 60.0 + hash01(i * 5 + 4) * 90.0;
            let top = 470.0 - h;
            let col = mix(
                Color::from_hex(0x1c1533),
                Color::from_hex(0x261c44),
                hash01(i),
            );
            rect_v(x, top, w, h + 10.0, col, shade(col, 0.8));
            if hash01(i * 9) > 0.55 {
                poly(
                    &[
                        vec2(x - 3.0, top),
                        vec2(x + w + 3.0, top),
                        vec2(x + w * 0.5, top - 20.0),
                    ],
                    col,
                );
            }
            let cols = (w / 13.0) as i32;
            let rows = (h / 15.0) as i32;
            for cy in 0..rows {
                for cx in 0..cols {
                    let k = i * 131 + (cy * 17 + cx) as u32;
                    if hash01(k) > 0.82 {
                        let on = 0.5 + 0.5 * ((t * 0.2 + hash01(k + 1) * 50.0).sin()).max(0.0);
                        let wc = mix(pal::AMBER, Color::from_hex(0xc9c2ff), hash01(k + 2));
                        draw_rectangle(
                            x + 5.0 + cx as f32 * 13.0,
                            top + 8.0 + cy as f32 * 15.0,
                            4.0,
                            5.0,
                            alpha(wc, 0.5 * on),
                        );
                    }
                }
            }
            x += w + 4.0;
            i += 1;
        }
        // a church tower over Holešovice with a lit clock
        let tx = 358.0;
        rect_v(
            tx - 26.0,
            250.0,
            52.0,
            230.0,
            Color::from_hex(0x2a1f48),
            Color::from_hex(0x1c1533),
        );
        poly(
            &[
                vec2(tx - 32.0, 252.0),
                vec2(tx + 32.0, 252.0),
                vec2(tx, 160.0),
            ],
            Color::from_hex(0x2f2350),
        );
        draw_line(tx, 160.0, tx, 140.0, 2.0, Color::from_hex(0x3a2c60));
        glow(tx, 300.0, 40.0, alpha(pal::AMBER, 0.35));
        draw_circle(tx, 300.0, 13.0, alpha(Color::from_hex(0xffe3a8), 0.9));
        let a = t * 0.02;
        draw_line(
            tx,
            300.0,
            tx + a.sin() * 9.0,
            300.0 - a.cos() * 9.0,
            1.6,
            Color::from_hex(0x4a3a5e),
        );
        draw_line(tx, 300.0, tx + 3.0, 294.0, 1.6, Color::from_hex(0x4a3a5e));
        // fog
        for i in 0..4 {
            let y = 440.0 + i as f32 * 18.0;
            let x = ((t * (6.0 + i as f32 * 2.0) + i as f32 * 400.0) % 2400.0) - 500.0;
            ellipse(x, y, 520.0, 18.0, alpha(Color::from_hex(0x8c7cc0), 0.07));
            ellipse(
                x + 1100.0,
                y + 6.0,
                440.0,
                16.0,
                alpha(Color::from_hex(0x8c7cc0), 0.06),
            );
        }
    }

    fn embankment(&self, t: f32, train: Option<f32>) {
        const DECK: f32 = 336.0;
        let stone_top = Color::from_hex(0x2a2046);
        let stone_bot = Color::from_hex(0x1a1430);
        // viaduct arches: piers + spandrels drawn in vertical strips
        let span = 170.0;
        let mut x0 = -80.0;
        while x0 < STAGE.x + 80.0 {
            let x1 = x0 + span;
            let pier = 26.0;
            rect_v(x0, DECK + 12.0, pier, 470.0 - DECK, stone_top, stone_bot);
            let (a0, a1) = (x0 + pier, x1);
            let cx = (a0 + a1) / 2.0;
            let rx = (a1 - a0) / 2.0;
            let steps = 14;
            for i in 0..steps {
                let xa = a0 + (a1 - a0) * i as f32 / steps as f32;
                let xb = a0 + (a1 - a0) * (i + 1) as f32 / steps as f32;
                let arc = |x: f32| {
                    let u = ((x - cx) / rx).clamp(-1.0, 1.0);
                    DECK + 70.0 - (1.0 - u * u).sqrt() * 44.0
                };
                quad(
                    [
                        vec2(xa, DECK + 12.0),
                        vec2(xb, DECK + 12.0),
                        vec2(xb, arc(xb)),
                        vec2(xa, arc(xa)),
                    ],
                    [
                        stone_top,
                        stone_top,
                        shade(stone_top, 0.9),
                        shade(stone_top, 0.9),
                    ],
                );
            }
            // rim light on the pier edge
            draw_line(
                x0 + pier,
                DECK + 60.0,
                x0 + pier,
                470.0,
                1.0,
                alpha(pal::LAVENDER_RIM, 0.10),
            );
            x0 = x1;
        }
        // deck + parapet
        rect_v(
            -80.0,
            DECK,
            STAGE.x + 160.0,
            14.0,
            Color::from_hex(0x3a2e5e),
            stone_top,
        );
        draw_line(
            -80.0,
            DECK,
            STAGE.x + 80.0,
            DECK,
            2.0,
            alpha(pal::LAVENDER_RIM, 0.45),
        );
        // catenary
        let mut x = 30.0;
        while x < STAGE.x {
            draw_line(x, DECK, x, DECK - 84.0, 2.0, Color::from_hex(0x2e2450));
            draw_line(
                x - 12.0,
                DECK - 76.0,
                x + 12.0,
                DECK - 76.0,
                1.5,
                Color::from_hex(0x2e2450),
            );
            x += 170.0;
        }
        draw_line(
            -80.0,
            DECK - 74.0,
            STAGE.x + 80.0,
            DECK - 72.0,
            1.0,
            alpha(pal::LAVENDER_RIM, 0.22),
        );
        // the train
        if let Some(p) = train {
            let len = 1300.0;
            let x = STAGE.x + 80.0 - (STAGE.x + len + 160.0) * p;
            let (top, h) = (DECK - 60.0, 56.0);
            glow(
                x + len / 2.0,
                top + h / 2.0,
                len * 0.55,
                alpha(pal::AMBER, 0.10),
            );
            rrect_v(
                x,
                top,
                len,
                h,
                16.0,
                Color::from_hex(0x3d4272),
                Color::from_hex(0x1f2142),
            );
            ellipse(
                x + 14.0,
                top + h * 0.55,
                28.0,
                h * 0.45,
                Color::from_hex(0x3a3f6a),
            );
            rect_h(
                x + 24.0,
                top + h - 14.0,
                len - 34.0,
                3.0,
                pal::INDIGO_GLOW,
                pal::VIOLET_SOFT,
            );
            glow(x + 4.0, top + h - 18.0, 40.0, alpha(WHITE, 0.7));
            // headlight beam ahead of the train
            quad(
                [
                    vec2(x + 2.0, top + h - 22.0),
                    vec2(x + 2.0, top + h - 12.0),
                    vec2(x - 260.0, top + h + 6.0),
                    vec2(x - 260.0, top + h - 44.0),
                ],
                [
                    alpha(WHITE, 0.22),
                    alpha(WHITE, 0.22),
                    alpha(WHITE, 0.0),
                    alpha(WHITE, 0.0),
                ],
            );
            let mut cx = x + 40.0;
            let mut k = 0u32;
            while cx < x + len - 30.0 {
                if (cx - x) % 260.0 < 220.0 {
                    let wc = mix(Color::from_hex(0xfff0d0), pal::AMBER, hash01(k) * 0.6);
                    rrect_v(cx, top + 10.0, 24.0, 22.0, 4.0, wc, shade(wc, 0.85));
                    if hash01(k + 7) > 0.65 {
                        ellipse(
                            cx + 12.0,
                            top + 27.0,
                            5.0,
                            6.0,
                            alpha(Color::from_hex(0x3a2f45), 0.7),
                        );
                        ellipse(
                            cx + 12.0,
                            top + 18.0,
                            3.6,
                            3.8,
                            alpha(Color::from_hex(0x3a2f45), 0.7),
                        );
                    }
                }
                cx += 30.0;
                k += 1;
            }
            for i in 0..5 {
                let yy = top + 6.0 + i as f32 * 10.0;
                let ox = ((t * 900.0 + i as f32 * 170.0) % (len + 200.0)) - 100.0;
                draw_line(
                    x + ox,
                    yy,
                    x + ox + 70.0,
                    yy,
                    1.0,
                    alpha(pal::LAVENDER_RIM, 0.25),
                );
            }
            // sparks from the pantograph
            if (t * 7.0).fract() < 0.15 {
                glow(
                    x + len * 0.3,
                    DECK - 73.0,
                    18.0,
                    alpha(pal::INDIGO_SOFT, 0.9),
                );
            }
        }
        // the little station on the viaduct: canopy + HOLEŠOVICE board
        rect_v(
            -20.0,
            DECK - 54.0,
            360.0,
            9.0,
            Color::from_hex(0x3a2e60),
            Color::from_hex(0x241b40),
        );
        for px in [20.0, 170.0, 320.0] {
            draw_line(px, DECK - 45.0, px, DECK, 3.0, Color::from_hex(0x2a2048));
            glow(px + 55.0, DECK - 42.0, 34.0, alpha(pal::INDIGO_SOFT, 0.4));
            draw_circle(px + 55.0, DECK - 43.0, 3.0, alpha(WHITE, 0.85));
        }
        rrect(
            108.0,
            DECK - 38.0,
            124.0,
            20.0,
            4.0,
            Color::from_hex(0x2b3a8c),
        );
        self.text(
            "HOLEŠOVICE",
            vec2(170.0, DECK - 23.0),
            13.0,
            alpha(WHITE, 0.9),
            true,
        );
    }

    fn neighbours(&self, t: f32) {
        // left: an old house with a closed bakery
        let x0 = 380.0;
        rect_v(
            x0,
            400.0,
            CAFE_X0 - x0 + 10.0,
            GROUND - 400.0,
            Color::from_hex(0x2b2246),
            Color::from_hex(0x1b1530),
        );
        poly(
            &[
                vec2(x0 - 10.0, 402.0),
                vec2(CAFE_X0 + 10.0, 402.0),
                vec2((x0 + CAFE_X0) / 2.0, 352.0),
            ],
            Color::from_hex(0x231b3c),
        );
        for (i, wx) in [420.0, 520.0].iter().enumerate() {
            let lit = i == 1;
            let c = if lit {
                alpha(pal::AMBER, 0.55 + 0.05 * (t * 0.7).sin())
            } else {
                Color::from_hex(0x15112a)
            };
            rrect_v(*wx, 440.0, 54.0, 56.0, 3.0, c, shade(c, 0.8));
            if lit {
                glow(wx + 27.0, 468.0, 60.0, alpha(pal::AMBER, 0.12));
                // a cat silhouette on the sill
                ellipse(wx + 40.0, 492.0, 8.0, 6.0, Color::from_hex(0x1a1426));
                draw_circle(wx + 46.0, 484.0, 4.5, Color::from_hex(0x1a1426));
                poly(
                    &[
                        vec2(wx + 43.0, 481.0),
                        vec2(wx + 45.0, 475.0),
                        vec2(wx + 47.0, 480.0),
                    ],
                    Color::from_hex(0x1a1426),
                );
            }
            draw_rectangle(wx - 4.0, 496.0, 62.0, 4.0, Color::from_hex(0x3a2e58));
        }
        // shuttered bakery
        rrect_v(
            410.0,
            600.0,
            190.0,
            160.0,
            2.0,
            Color::from_hex(0x2a2338),
            Color::from_hex(0x1e1a2a),
        );
        for i in 0..14 {
            draw_line(
                410.0,
                606.0 + i as f32 * 11.0,
                600.0,
                606.0 + i as f32 * 11.0,
                1.0,
                Color::from_hex(0x34304a),
            );
        }
        // right neighbour
        rect_v(
            CAFE_X1 - 10.0,
            392.0,
            STAGE.x - CAFE_X1 + 80.0,
            GROUND - 392.0,
            Color::from_hex(0x261f40),
            Color::from_hex(0x18132b),
        );
        for (wx, wy) in [(1500.0, 430.0), (1500.0, 520.0), (1500.0, 600.0)] {
            rrect_v(
                wx,
                wy,
                60.0,
                80.0,
                3.0,
                Color::from_hex(0x15112a),
                Color::from_hex(0x100d20),
            );
            draw_rectangle(wx - 4.0, wy + 80.0, 68.0, 4.0, Color::from_hex(0x3a2e58));
        }
        glow(
            1530.0,
            460.0,
            30.0,
            alpha(pal::INDIGO_SOFT, 0.1 + 0.05 * (t * 1.3).sin()),
        );
    }

    fn cafe(&self, t: f32, k: f32) {
        let (x0, x1) = (CAFE_X0, CAFE_X1);
        // body: two storeys of violet plaster
        rect_v(
            x0,
            380.0,
            x1 - x0,
            GROUND - 380.0,
            Color::from_hex(0x4a3a78),
            Color::from_hex(0x2f2452),
        );
        // rim light from the street lamp side
        rect_h(
            x0,
            380.0,
            40.0,
            GROUND - 380.0,
            alpha(pal::AMBER, 0.10),
            alpha(pal::AMBER, 0.0),
        );
        // wet plaster: darker streaks under the cornice and stains
        for i in 0..18u32 {
            let sx = x0 + 20.0 + hash01(i * 11 + 3) * (x1 - x0 - 40.0);
            let len = 40.0 + hash01(i * 7 + 1) * 120.0;
            rect_v(
                sx,
                388.0,
                6.0 + hash01(i) * 10.0,
                len,
                alpha(Color::from_hex(0x1c1438), 0.22),
                alpha(Color::from_hex(0x1c1438), 0.0),
            );
        }
        for (sx, sy, r) in [
            (900.0, 470.0, 60.0),
            (1300.0, 430.0, 46.0),
            (720.0, 520.0, 40.0),
        ] {
            ellipse(sx, sy, r, r * 0.6, alpha(Color::from_hex(0x2a1f4a), 0.25));
        }
        // drainpipe
        rect_h(
            x1 - 18.0,
            384.0,
            8.0,
            GROUND - 384.0,
            Color::from_hex(0x5a4a80),
            Color::from_hex(0x2a2048),
        );
        for yy in [450.0, 560.0, 680.0] {
            draw_rectangle(x1 - 21.0, yy, 14.0, 4.0, Color::from_hex(0x3a2e5e));
        }
        // roof
        quad(
            [
                vec2(x0 - 16.0, 380.0),
                vec2(x1 + 16.0, 380.0),
                vec2(x1 - 30.0, 324.0),
                vec2(x0 + 30.0, 324.0),
            ],
            [
                Color::from_hex(0x2a1f44),
                Color::from_hex(0x2a1f44),
                Color::from_hex(0x1d1533),
                Color::from_hex(0x1d1533),
            ],
        );
        for i in 0..5 {
            let y = 332.0 + i as f32 * 10.0;
            draw_line(
                x0 + 20.0 - i as f32 * 8.0,
                y,
                x1 - 20.0 + i as f32 * 8.0,
                y,
                1.0,
                alpha(pal::LAVENDER_RIM, 0.07),
            );
        }
        // chimney with a wisp
        rect_v(
            1320.0,
            290.0,
            36.0,
            56.0,
            Color::from_hex(0x3a2c5e),
            Color::from_hex(0x261d40),
        );
        draw_steam(vec2(1338.0, 286.0), t, 0.6);
        // cornice + band between floors
        rect_v(
            x0 - 14.0,
            376.0,
            x1 - x0 + 28.0,
            12.0,
            Color::from_hex(0x6a58a0),
            Color::from_hex(0x3e3168),
        );
        rect_v(
            x0 - 6.0,
            548.0,
            x1 - x0 + 12.0,
            8.0,
            Color::from_hex(0x5e4e92),
            Color::from_hex(0x3a2e60),
        );
        // upper windows: grandma's old flat (a faint lamp) and a dark one
        for (i, wx) in [740.0, 980.0, 1220.0].iter().enumerate() {
            let r = Rect::new(*wx, 404.0, 110.0, 86.0);
            let lit = i == 0;
            let top = if lit {
                Color::from_hex(0x7a4fa0)
            } else {
                Color::from_hex(0x1c1734)
            };
            let bot = if lit {
                Color::from_hex(0xc4884e)
            } else {
                Color::from_hex(0x14102a)
            };
            rrect_v(
                r.x - 6.0,
                r.y - 6.0,
                r.w + 12.0,
                r.h + 12.0,
                6.0,
                Color::from_hex(0x5c4c8c),
                Color::from_hex(0x3a2e62),
            );
            rrect_v(r.x, r.y, r.w, r.h, 4.0, top, bot);
            if lit {
                glow(
                    r.x + r.w / 2.0,
                    r.y + r.h * 0.7,
                    90.0,
                    alpha(pal::AMBER, 0.18),
                );
                // curtain
                quad(
                    [
                        vec2(r.x, r.y),
                        vec2(r.x + 30.0, r.y),
                        vec2(r.x + 22.0, r.y + r.h),
                        vec2(r.x, r.y + r.h),
                    ],
                    [alpha(pal::VIOLET_SOFT, 0.55); 4],
                );
                // a lamp + plant silhouette
                ellipse(
                    r.x + 78.0,
                    r.y + 112.0,
                    12.0,
                    4.0,
                    Color::from_hex(0x2a1c30),
                );
                poly(
                    &[
                        vec2(r.x + 70.0, r.y + 96.0),
                        vec2(r.x + 86.0, r.y + 96.0),
                        vec2(r.x + 82.0, r.y + 84.0),
                        vec2(r.x + 74.0, r.y + 84.0),
                    ],
                    Color::from_hex(0x3a2440),
                );
            } else {
                // reflection of the sky
                quad(
                    [
                        vec2(r.x + 14.0, r.y),
                        vec2(r.x + 40.0, r.y),
                        vec2(r.x + 8.0, r.y + r.h),
                        vec2(r.x, r.y + r.h),
                    ],
                    [alpha(pal::LAVENDER_RIM, 0.10); 4],
                );
            }
            draw_line(
                r.x + r.w / 2.0,
                r.y,
                r.x + r.w / 2.0,
                r.y + r.h,
                3.0,
                Color::from_hex(0x4a3c78),
            );
            draw_line(
                r.x,
                r.y + r.h * 0.42,
                r.x + r.w,
                r.y + r.h * 0.42,
                3.0,
                Color::from_hex(0x4a3c78),
            );
            draw_rectangle(
                r.x - 10.0,
                r.y + r.h + 6.0,
                r.w + 20.0,
                6.0,
                Color::from_hex(0x6a58a0),
            );
            self.weather.draw_window_rain(r, t);
        }

        // ── ground floor: the café front
        rect_v(
            x0,
            556.0,
            x1 - x0,
            GROUND - 556.0,
            Color::from_hex(0x3b2c63),
            Color::from_hex(0x2a1f4a),
        );
        // base plinth
        rect_v(
            x0,
            GROUND - 22.0,
            x1 - x0,
            22.0,
            Color::from_hex(0x2a2044),
            Color::from_hex(0x1c1630),
        );
        self.shop_window(t);
        self.door(t);
        // awning
        let aw = Rect::new(WIN.x - 16.0, 560.0, WIN.w + 32.0, 24.0);
        let stripes = 12;
        for i in 0..stripes {
            let sx = aw.x + aw.w * i as f32 / stripes as f32;
            let c = if i % 2 == 0 {
                Color::from_hex(0x5b4bd6)
            } else {
                Color::from_hex(0x7c3aed)
            };
            rect_v(
                sx,
                aw.y,
                aw.w / stripes as f32 + 0.5,
                aw.h,
                tint(c, 0.15),
                shade(c, 0.75),
            );
            ellipse(
                sx + aw.w / stripes as f32 / 2.0,
                aw.y + aw.h,
                aw.w / stripes as f32 / 2.0,
                6.0,
                shade(c, 0.75),
            );
        }
        draw_line(
            aw.x,
            aw.y,
            aw.x + aw.w,
            aw.y,
            2.0,
            alpha(pal::LAVENDER_RIM, 0.5),
        );
        // the holo sign over the awning
        self.holo_sign(t, k);
        // little lamp over the door
        let lp = vec2(DOOR.x + DOOR.w / 2.0, DOOR.y - 14.0);
        glow(lp.x, lp.y + 10.0, 70.0, alpha(pal::AMBER, 0.35));
        rrect_v(
            lp.x - 10.0,
            lp.y - 6.0,
            20.0,
            10.0,
            4.0,
            Color::from_hex(0x4a3a60),
            Color::from_hex(0x2a2040),
        );
        ellipse(lp.x, lp.y + 5.0, 7.0, 2.5, Color::from_hex(0xfff0c8));
        // house number plate
        rrect(1290.0, 640.0, 34.0, 24.0, 4.0, Color::from_hex(0x2b3a8c));
        self.text("47", vec2(1307.0, 657.0), 12.0, alpha(WHITE, 0.85), true);
    }

    fn shop_window(&self, t: f32) {
        let w = WIN;
        // frame
        rrect_v(
            w.x - 10.0,
            w.y - 10.0,
            w.w + 20.0,
            w.h + 20.0,
            6.0,
            Color::from_hex(0x5c4c8c),
            Color::from_hex(0x30255a),
        );
        // warm interior
        rect_v(
            w.x,
            w.y,
            w.w,
            w.h,
            Color::from_hex(0xd98a52),
            Color::from_hex(0x7a3f3a),
        );
        // back wall details: shelves with jars, a framed photo
        for sy in [w.y + 34.0, w.y + 62.0] {
            draw_rectangle(
                w.x + 230.0,
                sy,
                150.0,
                4.0,
                alpha(Color::from_hex(0x4a2a28), 0.7),
            );
            for j in 0..7 {
                let jx = w.x + 236.0 + j as f32 * 20.0;
                let h = 10.0 + hash01(j + sy as u32) * 8.0;
                rrect(
                    jx,
                    sy - h,
                    12.0,
                    h,
                    2.0,
                    alpha(mix(pal::CREAM, pal::AMBER_DEEP, hash01(j * 3)), 0.6),
                );
            }
        }
        rrect(
            w.x + 120.0,
            w.y + 18.0,
            30.0,
            38.0,
            2.0,
            alpha(Color::from_hex(0x5a3530), 0.8),
        );
        rrect(
            w.x + 124.0,
            w.y + 22.0,
            22.0,
            30.0,
            2.0,
            alpha(pal::CREAM, 0.5),
        );
        // pendant lamps
        for (i, lx) in [w.x + 70.0, w.x + 200.0, w.x + 330.0].iter().enumerate() {
            let sway = (t * 0.8 + i as f32).sin() * 1.5;
            draw_line(
                *lx,
                w.y,
                *lx + sway,
                w.y + 26.0,
                1.0,
                alpha(Color::from_hex(0x3a2030), 0.8),
            );
            glow(
                *lx + sway,
                w.y + 34.0,
                80.0,
                alpha(Color::from_hex(0xffd48a), 0.45),
            );
            poly(
                &[
                    vec2(*lx - 12.0 + sway, w.y + 34.0),
                    vec2(*lx + 12.0 + sway, w.y + 34.0),
                    vec2(*lx + 5.0 + sway, w.y + 24.0),
                    vec2(*lx - 5.0 + sway, w.y + 24.0),
                ],
                Color::from_hex(0x4f46e5),
            );
            ellipse(*lx + sway, w.y + 35.0, 8.0, 2.4, Color::from_hex(0xfff4d6));
        }
        // counter + coffee machine at the right
        rect_v(
            w.x + 250.0,
            w.y + 100.0,
            150.0,
            50.0,
            Color::from_hex(0x6a3a30),
            Color::from_hex(0x3e2226),
        );
        rrect_v(
            w.x + 330.0,
            w.y + 72.0,
            40.0,
            30.0,
            4.0,
            Color::from_hex(0xc9cfe2),
            Color::from_hex(0x8e96ad),
        );
        draw_steam(vec2(w.x + 350.0, w.y + 70.0), t, 0.8);
        // guests (real people, warm-tinted) at a table and the barista
        let warm = |look: &PersonLook| {
            let mut l = *look;
            l.top = mix(l.top, pal::AMBER_DEEP, 0.25);
            l.skin = mix(l.skin, pal::AMBER, 0.15);
            l
        };
        let table = vec2(w.x + 120.0, w.y + 128.0);
        draw_rectangle(
            table.x - 4.0,
            table.y - 6.0,
            8.0,
            30.0,
            alpha(Color::from_hex(0x3a1f22), 0.9),
        );
        ellipse(table.x, table.y - 8.0, 44.0, 7.0, Color::from_hex(0x8a4f38));
        let pose = |feet: Vec2, f: crate::render::Facing, pose: crate::render::Pose| {
            crate::render::PersonPose {
                feet,
                facing: f,
                pose,
                t,
            }
        };
        let draw_scaled = |look: &PersonLook, p: crate::render::PersonPose| {
            crate::render::draw_person(look, &p);
        };
        draw_scaled(
            &warm(&self.guests[0]),
            pose(
                vec2(table.x - 58.0, w.y + 150.0),
                crate::render::Facing::Right,
                crate::render::Pose::Sit,
            ),
        );
        draw_scaled(
            &warm(&self.guests[1]),
            pose(
                vec2(table.x + 58.0, w.y + 150.0),
                crate::render::Facing::Left,
                crate::render::Pose::Sit,
            ),
        );
        draw_scaled(
            &warm(&self.guests[2]),
            pose(
                vec2(w.x + 300.0, w.y + 104.0),
                crate::render::Facing::Down,
                crate::render::Pose::Work,
            ),
        );
        // cups with steam
        draw_steam(vec2(table.x - 18.0, table.y - 14.0), t, 0.5);
        draw_steam(vec2(table.x + 20.0, table.y - 14.0), t + 1.3, 0.5);
        // glass: tint, reflection, lettering, mullions, rain
        rect_v(
            w.x,
            w.y,
            w.w,
            w.h,
            alpha(Color::from_hex(0x2a1c50), 0.12),
            alpha(Color::from_hex(0x2a1c50), 0.28),
        );
        quad(
            [
                vec2(w.x + 30.0, w.y),
                vec2(w.x + 90.0, w.y),
                vec2(w.x + 40.0, w.y + w.h),
                vec2(w.x - 20.0, w.y + w.h),
            ],
            [alpha(WHITE, 0.06); 4],
        );
        self.text(
            "KAVÁRNA · 22–06",
            vec2(w.x + w.w * 0.25, w.y + w.h - 12.0),
            12.0,
            alpha(Color::from_hex(0xfff0d0), 0.6),
            true,
        );
        draw_line(
            w.x + w.w / 2.0,
            w.y,
            w.x + w.w / 2.0,
            w.y + w.h,
            4.0,
            Color::from_hex(0x3e3168),
        );
        self.weather.draw_window_rain(w, t);
        self.weather
            .draw_window_rain(Rect::new(w.x + w.w / 2.0, w.y, w.w / 2.0, w.h), t + 3.0);
        // light spilling onto the pavement
        quad(
            [
                vec2(w.x, GROUND),
                vec2(w.x + w.w, GROUND),
                vec2(w.x + w.w + 120.0, 900.0),
                vec2(w.x - 120.0, 900.0),
            ],
            [
                alpha(pal::AMBER, 0.20),
                alpha(pal::AMBER, 0.20),
                alpha(pal::AMBER, 0.0),
                alpha(pal::AMBER, 0.0),
            ],
        );
    }

    fn door(&self, t: f32) {
        let d = DOOR;
        rrect_v(
            d.x - 8.0,
            d.y - 8.0,
            d.w + 16.0,
            d.h + 8.0,
            6.0,
            Color::from_hex(0x5c4c8c),
            Color::from_hex(0x30255a),
        );
        rrect_v(
            d.x,
            d.y,
            d.w,
            d.h,
            3.0,
            Color::from_hex(0x3a2440),
            Color::from_hex(0x24172c),
        );
        let g = Rect::new(d.x + 12.0, d.y + 12.0, d.w - 24.0, 74.0);
        rect_v(
            g.x,
            g.y,
            g.w,
            g.h,
            Color::from_hex(0xe0a060),
            Color::from_hex(0x9a5a40),
        );
        self.weather.draw_window_rain(g, t + 7.0);
        // "OTEVŘENO" tag
        rrect(
            g.x + 8.0,
            g.y + 44.0,
            g.w - 16.0,
            16.0,
            3.0,
            alpha(pal::EMERALD, 0.85),
        );
        self.text(
            "OTEVŘENO",
            vec2(g.x + g.w / 2.0, g.y + 56.0),
            9.0,
            alpha(WHITE, 0.95),
            true,
        );
        draw_circle(
            d.x + d.w - 16.0,
            d.y + 100.0,
            4.0,
            Color::from_hex(0xc79a3e),
        );
        // doormat (where the key is)
        rrect(
            d.x - 4.0,
            GROUND - 6.0,
            d.w + 8.0,
            8.0,
            2.0,
            Color::from_hex(0x5a3a2a),
        );
        quad(
            [
                vec2(d.x, GROUND),
                vec2(d.x + d.w, GROUND),
                vec2(d.x + d.w + 60.0, 900.0),
                vec2(d.x - 60.0, 900.0),
            ],
            [
                alpha(pal::AMBER, 0.12),
                alpha(pal::AMBER, 0.12),
                alpha(pal::AMBER, 0.0),
                alpha(pal::AMBER, 0.0),
            ],
        );
    }

    fn holo_sign(&self, t: f32, k: f32) {
        let fl = crate::render::holo_flicker(t, 11);
        let r = Rect::new(756.0, 498.0, 348.0, 54.0);
        let c = r.center();
        glow(c.x, c.y, 260.0, alpha(pal::INDIGO_GLOW, 0.22 * fl));
        rrect_v(
            r.x,
            r.y,
            r.w,
            r.h,
            10.0,
            alpha(Color::from_hex(0x3b3ea8), 0.55 * fl),
            alpha(Color::from_hex(0x5b3aa8), 0.5 * fl),
        );
        let edge = alpha(pal::INDIGO_SOFT, 0.95 * fl);
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.5, edge);
        // cup icon
        let ic = vec2(r.x + 26.0, c.y + 1.0);
        rrect(
            ic.x - 8.0,
            ic.y - 10.0,
            16.0,
            12.0,
            3.0,
            alpha(pal::AMBER, 0.95 * fl),
        );
        draw_circle_lines(
            ic.x + 9.0,
            ic.y - 5.0,
            3.4,
            1.6,
            alpha(pal::AMBER, 0.95 * fl),
        );
        // letters; one is cracked and stutters now and then (never long enough to vanish)
        let text = "CAFÉ NOCTURNE";
        let size = 21.0;
        if let Some(font) = &self.font {
            let fs = (size * k).round().max(1.0) as u16;
            let sc = size / fs as f32;
            let gaps = (text.chars().count() - 1) as f32 * 2.0;
            self.glyphs.prepare(font, text, fs);
            let total = measure_text(text, Some(font), fs, sc).width + gaps;
            let mut x = c.x + 14.0 - total / 2.0;
            for (i, ch) in text.chars().enumerate() {
                let s = ch.to_string();
                let adv = measure_text(&s, Some(font), fs, sc).width;
                let broken = i == 6;
                let a = if broken {
                    let h = hash((t * 9.0) as u32);
                    if h % 11 < 2 { 0.5 * fl } else { 0.9 * fl }
                } else {
                    fl
                };
                draw_text_ex(
                    &s,
                    x,
                    c.y + 4.0,
                    TextParams {
                        font: Some(font),
                        font_size: fs,
                        font_scale: sc,
                        color: alpha(Color::from_hex(0xeef0ff), a),
                        ..Default::default()
                    },
                );
                x += adv + 2.0;
            }
            // the small Czech subtitle under the name
            let sub = "kavárna u nádraží";
            let size = 10.5;
            let fs = (size * k).round().max(1.0) as u16;
            let sc = size / fs as f32;
            self.glyphs.prepare(font, sub, fs);
            let w = measure_text(sub, Some(font), fs, sc).width + 1.5 * sub.chars().count() as f32;
            let mut x = c.x + 14.0 - w / 2.0;
            for ch in sub.chars() {
                let s = ch.to_string();
                draw_text_ex(
                    &s,
                    x,
                    c.y + 19.0,
                    TextParams {
                        font: Some(font),
                        font_size: fs,
                        font_scale: sc,
                        color: alpha(Color::from_hex(0xc7c9ff), 0.85 * fl),
                        ..Default::default()
                    },
                );
                x += measure_text(&s, Some(font), fs, sc).width + 1.5;
            }
        }
        let scan = (t * 0.8).fract();
        draw_line(
            r.x + 2.0,
            r.y + scan * r.h,
            r.x + r.w - 2.0,
            r.y + scan * r.h,
            1.0,
            alpha(WHITE, 0.18 * fl),
        );
        if fl < 0.8 {
            let gy = r.y + r.h * hash01((t * 20.0) as u32);
            draw_line(
                r.x - 8.0,
                gy,
                r.x + r.w + 8.0,
                gy,
                2.0,
                alpha(pal::VIOLET_SOFT, 0.5),
            );
        }
        // its glow on the wet pavement
        quad(
            [
                vec2(r.x, GROUND),
                vec2(r.x + r.w, GROUND),
                vec2(r.x + r.w + 40.0, 900.0),
                vec2(r.x - 40.0, 900.0),
            ],
            [
                alpha(pal::INDIGO_GLOW, 0.05 * fl),
                alpha(pal::INDIGO_GLOW, 0.05 * fl),
                alpha(pal::INDIGO_GLOW, 0.0),
                alpha(pal::INDIGO_GLOW, 0.0),
            ],
        );
    }

    fn street(&self, t: f32) {
        // pavement slabs
        rect_v(
            -60.0,
            GROUND,
            STAGE.x + 120.0,
            44.0,
            Color::from_hex(0x2c2440),
            Color::from_hex(0x221b34),
        );
        let mut x = -60.0;
        while x < STAGE.x + 60.0 {
            draw_line(
                x,
                GROUND,
                x - 10.0,
                GROUND + 44.0,
                1.0,
                alpha(Color::from_hex(0x120e1e), 0.6),
            );
            x += 64.0;
        }
        // curb
        rect_v(
            -60.0,
            GROUND + 44.0,
            STAGE.x + 120.0,
            8.0,
            Color::from_hex(0x4a3e6a),
            Color::from_hex(0x2a2140),
        );
        // cobbles in perspective rows
        let mut y = GROUND + 52.0;
        let mut row = 0u32;
        while y < STAGE.y + 20.0 {
            let h = 9.0 + row as f32 * 1.6;
            let w = 22.0 + row as f32 * 3.0;
            let off = if row.is_multiple_of(2) { 0.0 } else { w / 2.0 };
            let mut cx = -60.0 - off;
            let mut i = 0u32;
            while cx < STAGE.x + 60.0 {
                let v = hash01(row * 977 + i);
                let base = mix(Color::from_hex(0x2a2238), Color::from_hex(0x3a3050), v);
                rrect_v(
                    cx + 1.0,
                    y + 1.0,
                    w - 2.0,
                    h - 2.0,
                    3.0,
                    tint(base, 0.12),
                    base,
                );
                cx += w;
                i += 1;
            }
            y += h;
            row += 1;
        }
        // wet sheen + puddles that mirror the warm window and the sign
        rect_v(
            -60.0,
            GROUND + 52.0,
            STAGE.x + 120.0,
            160.0,
            alpha(Color::from_hex(0x6a5a9a), 0.10),
            alpha(Color::from_hex(0x6a5a9a), 0.0),
        );
        for (px, py, rx, ry) in [
            (900.0, 840.0, 190.0, 22.0),
            (1210.0, 870.0, 120.0, 16.0),
            (520.0, 858.0, 140.0, 18.0),
            (300.0, 812.0, 90.0, 12.0),
        ] {
            ellipse(px, py, rx, ry, alpha(Color::from_hex(0x0e0a1c), 0.85));
            // mirrored light: thin wobbling strips
            let n = 16;
            for j in 0..n {
                let yy = py - ry * 0.7 + j as f32 * (ry * 1.4 / n as f32);
                let wob = (t * 2.2 + j as f32 * 0.9 + px * 0.01).sin() * 4.0;
                let span = rx * (1.0 - ((yy - py) / ry).powi(2)).max(0.0).sqrt() * 0.92;
                // the window reflects where it is above the puddle
                let lit_l = (WIN.x - 20.0).max(px - span);
                let lit_r = (WIN.x + WIN.w + 20.0).min(px + span);
                if lit_r > lit_l {
                    draw_line(
                        lit_l + wob,
                        yy,
                        lit_r + wob,
                        yy,
                        1.4,
                        alpha(pal::AMBER, 0.16),
                    );
                }
                let sl = 780.0_f32.max(px - span);
                let sr = 1080.0_f32.min(px + span);
                if sr > sl && j % 3 == 0 {
                    draw_line(
                        sl - wob,
                        yy + 1.0,
                        sr - wob,
                        yy + 1.0,
                        1.0,
                        alpha(pal::INDIGO_GLOW, 0.35),
                    );
                }
                let ll = 520.0_f32.max(px - span);
                let lr = 600.0_f32.min(px + span);
                if lr > ll {
                    draw_line(
                        ll + wob,
                        yy,
                        lr + wob,
                        yy,
                        1.4,
                        alpha(Color::from_hex(0xffe0a8), 0.2),
                    );
                }
            }
            // rings from drops
            let ph = (t * 1.7 + px * 0.013) % 1.0;
            draw_ellipse_lines(
                px + (px * 0.37) % 60.0 - 30.0,
                py,
                3.0 + ph * 16.0,
                1.0 + ph * 4.0,
                0.0,
                1.0,
                alpha(pal::LAVENDER_RIM, 0.4 * (1.0 - ph)),
            );
        }
    }

    fn lamp_post(&self, t: f32) {
        let x = 560.0;
        soft_shadow(x, GROUND + 20.0, 22.0, 6.0, 0.6);
        rect_v(
            x - 4.0,
            420.0,
            8.0,
            GROUND + 20.0 - 420.0,
            Color::from_hex(0x3a3050),
            Color::from_hex(0x1e1830),
        );
        draw_line(x, 424.0, x + 34.0, 412.0, 4.0, Color::from_hex(0x3a3050));
        let bulb = vec2(x + 40.0, 418.0);
        let fl = 0.96 + 0.04 * (t * 13.0).sin();
        glow(bulb.x, bulb.y + 6.0, 120.0 * fl, alpha(pal::AMBER, 0.35));
        rrect_v(
            bulb.x - 14.0,
            bulb.y - 8.0,
            28.0,
            10.0,
            4.0,
            Color::from_hex(0x4a4566),
            Color::from_hex(0x2a2540),
        );
        ellipse(bulb.x, bulb.y + 3.0, 10.0, 3.0, Color::from_hex(0xfff0c8));
        quad(
            [
                bulb + vec2(-8.0, 4.0),
                bulb + vec2(8.0, 4.0),
                vec2(bulb.x + 120.0, GROUND + 40.0),
                vec2(bulb.x - 120.0, GROUND + 40.0),
            ],
            [
                alpha(pal::AMBER, 0.16),
                alpha(pal::AMBER, 0.16),
                alpha(pal::AMBER, 0.02),
                alpha(pal::AMBER, 0.02),
            ],
        );
        glow(bulb.x, GROUND + 30.0, 140.0, alpha(pal::AMBER, 0.12));
    }

    fn lamp_rain(&self, t: f32) {
        let b = vec2(600.0, 422.0);
        for i in 0..60u32 {
            let ph = hash01(i * 13 + 5);
            let y = b.y + ((ph * 380.0 + t * 700.0 * (0.85 + ph * 0.3)) % 380.0);
            let spread = (y - b.y) / (GROUND + 40.0 - b.y) * 115.0 + 6.0;
            let x = b.x + (hash01(i * 7 + 1) * 2.0 - 1.0) * spread + (y - b.y) * 0.14;
            let a = 0.45 * (1.0 - (y - b.y) / 400.0);
            draw_line(
                x,
                y,
                x + 2.0,
                y + 14.0,
                1.2,
                alpha(Color::from_hex(0xffe8c0), a),
            );
        }
    }

    fn drone(&self, t: f32) {
        let ph = (t + 8.0) % DRONE_EVERY / 14.0;
        if ph > 1.0 {
            return;
        }
        let x = -80.0 + (STAGE.x + 160.0) * ph;
        let y = 210.0 + (ph * 6.0).sin() * 24.0 + (t * 3.0).sin() * 2.0;
        // parcel on a line
        draw_line(x, y + 6.0, x, y + 22.0, 1.0, alpha(pal::LAVENDER_RIM, 0.5));
        rrect(
            x - 7.0,
            y + 22.0,
            14.0,
            11.0,
            2.0,
            Color::from_hex(0x8a6a4a),
        );
        // body + rotors
        rrect_v(
            x - 14.0,
            y - 4.0,
            28.0,
            9.0,
            4.0,
            Color::from_hex(0x4a4a6e),
            Color::from_hex(0x24243a),
        );
        for dx in [-20.0, 20.0] {
            draw_line(
                x + dx * 0.6,
                y,
                x + dx,
                y - 3.0,
                2.0,
                Color::from_hex(0x2a2a40),
            );
            ellipse(x + dx, y - 4.0, 10.0, 2.0, alpha(pal::LAVENDER_RIM, 0.25));
        }
        let blink = (t * 2.5).fract() < 0.18;
        if blink {
            glow(x - 15.0, y + 1.0, 14.0, alpha(pal::ROSE, 0.8));
            glow(x + 15.0, y + 1.0, 14.0, alpha(pal::EMERALD, 0.8));
        }
        glow(x, y + 5.0, 24.0, alpha(pal::INDIGO_GLOW, 0.35));
        // a faint downward beam
        quad(
            [
                vec2(x - 3.0, y + 4.0),
                vec2(x + 3.0, y + 4.0),
                vec2(x + 26.0, y + 110.0),
                vec2(x - 26.0, y + 110.0),
            ],
            [
                alpha(pal::INDIGO_SOFT, 0.12),
                alpha(pal::INDIGO_SOFT, 0.12),
                alpha(pal::INDIGO_SOFT, 0.0),
                alpha(pal::INDIGO_SOFT, 0.0),
            ],
        );
    }

    /// The train's light band sweeping across the street and façades.
    fn sweep(&self, p: f32) {
        let x = STAGE.x + 200.0 - (STAGE.x + 600.0) * p;
        let w = 360.0;
        let a = 0.14 * (1.0 - (2.0 * p - 1.0).powi(4));
        for (y0, y1) in [(230.0, 560.0), (560.0, STAGE.y)] {
            quad(
                [
                    vec2(x - w, y0),
                    vec2(x, y0),
                    vec2(x + 60.0, y1),
                    vec2(x - w + 60.0, y1),
                ],
                [
                    alpha(pal::LAVENDER_RIM, 0.0),
                    alpha(pal::LAVENDER_RIM, a),
                    alpha(pal::LAVENDER_RIM, a * 0.6),
                    alpha(pal::LAVENDER_RIM, 0.0),
                ],
            );
        }
    }

    fn text(&self, text: &str, pos: Vec2, size: f32, col: Color, centered: bool) {
        crate::render::world_text(text, pos, size, col, centered);
    }
}

fn tint(c: Color, k: f32) -> Color {
    mix(c, WHITE, k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_train_comes_every_forty_seconds() {
        let mut b = Backdrop::new(None);
        b.t = TRAIN_FIRST + 1.0;
        assert!(b.train().is_some());
        b.t += TRAIN_SECONDS + 1.0;
        assert!(b.train().is_none());
        b.t = TRAIN_FIRST + TRAIN_EVERY + 2.0;
        assert!(b.train().is_some());
    }
}
