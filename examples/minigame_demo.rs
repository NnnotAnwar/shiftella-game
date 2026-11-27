//! Brewing mini-games over a dark café backdrop. `cargo run --example minigame_demo`
//!
//! * `SHIFTELLA_MG=froth|latte|pour|tea|plate|stir|mulled` (default `latte`), `SHIFTELLA_SHAPE=heart|rosetta|
//!   tulip|leaf|swan|snowflake|moon|train|cat|note|heart_l`, `SHIFTELLA_PLATE=medovnik|soup|topinka`,
//!   `SHIFTELLA_TEA=black|green|herbal|fruit`,
//!   `SHIFTELLA_TOUCH=extra_foam|light_foam|strong|mild|cinnamon|honey`, `SHIFTELLA_SEED=n`,
//!   `SHIFTELLA_VARIANT=chocolate|two_sugars|…` (minigames.ron variant ids), `SHIFTELLA_PRACTISED=1`,
//!   `SHIFTELLA_LANG=cs|en|uk|ru`, `SHIFTELLA_RELAXED=1`, `SHIFTELLA_TITLE="Flat white"`,
//!   `SHIFTELLA_GUEST=Vera` (shows the guest chip and a wish line), `SHIFTELLA_KB=1` (scripted
//!   keyboard play instead of mouse).
//! * Keys: F2–F6 pick a game, F7 next shape / set, F1 toggles autoplay, F8 restarts.
//! * Screenshot: `SHIFTELLA_SHOT=shots/latte.png SHIFTELLA_T=6` — plays the scripted input up
//!   to `T` seconds, renders ~40 frames, saves the PNG and exits.
//! * `SHIFTELLA_DEMO=cat` shows the station cat poses (also with `SHIFTELLA_SHOT`).

use macroquad::prelude::*;

use shiftella_game::i18n::Lang;
use shiftella_game::minigame::*;
use shiftella_game::render::cat::{CatLook, CatPose, draw_cat_ex};
use shiftella_game::render::draw as rd;
use shiftella_game::ui::draw::{Paint, fill_rrect};
use shiftella_game::ui::*;

fn conf() -> Conf {
    shiftella_game::platform::hide_for_automation();
    Conf {
        window_title: "Shiftella — mini-games".to_owned(),
        window_width: 1100,
        window_height: 680,
        high_dpi: true,
        sample_count: 4,
        ..Default::default()
    }
}

fn env(k: &str) -> Option<String> {
    std::env::var(k).ok().filter(|s| !s.is_empty())
}

fn load_text(lang: Lang) -> MiniGameText {
    let src = include_str!("../assets/i18n/src/minigame.tr");
    let mut tables: shiftella_game::i18n::source::Tables = Default::default();
    shiftella_game::i18n::source::parse_into(&mut tables, src, "minigame.tr").expect("minigame.tr");
    let src2 = include_str!("../assets/i18n/src/minigames2.tr");
    shiftella_game::i18n::source::parse_into(&mut tables, src2, "minigames2.tr")
        .expect("minigames2.tr");
    let table = tables[lang.index()].clone();
    MiniGameText::from_lookup(|k| table.get(k).cloned().unwrap_or_else(|| k.to_string()))
}

struct Setup {
    kind: MiniGameKind,
    variation: Variation,
    seed: u64,
    relaxed: bool,
    practised: bool,
    keyboard: bool,
    title: Option<String>,
    guest: Option<String>,
    lang: Lang,
}

impl Setup {
    fn from_env() -> Setup {
        let kind = env("SHIFTELLA_MG")
            .and_then(|s| MiniGameKind::from_id(&s))
            .unwrap_or(MiniGameKind::LatteArt);
        let mut variation = env("SHIFTELLA_VARIANT")
            .map(|v| Variation::from_variant_id(kind, &v))
            .unwrap_or_default();
        if let Some(s) = env("SHIFTELLA_SHAPE").and_then(|s| LatteShape::from_id(&s)) {
            variation.shape = Some(s);
        }
        if let Some(p) = env("SHIFTELLA_PLATE").and_then(|s| PlateSet::from_id(&s)) {
            variation.plate = Some(p);
        }
        if let Some(k) = env("SHIFTELLA_TEA").and_then(|s| TeaKind::from_id(&s)) {
            variation.tea_kind = Some(k);
        }
        variation.touch = env("SHIFTELLA_TOUCH").and_then(|s| RegularTouch::from_id(&s));
        Setup {
            kind,
            variation,
            seed: env("SHIFTELLA_SEED")
                .and_then(|s| s.parse().ok())
                .unwrap_or(7),
            relaxed: env("SHIFTELLA_RELAXED").is_some_and(|s| s != "0"),
            practised: env("SHIFTELLA_PRACTISED").is_some_and(|s| s != "0"),
            keyboard: env("SHIFTELLA_KB").is_some_and(|s| s != "0"),
            title: env("SHIFTELLA_TITLE"),
            guest: env("SHIFTELLA_GUEST"),
            lang: env("SHIFTELLA_LANG")
                .and_then(|s| Lang::from_code(&s))
                .unwrap_or(Lang::En),
        }
    }

    fn start(&self, text: &MiniGameText) -> MiniGame {
        let title = self.title.clone().unwrap_or_else(|| {
            match self.kind {
                MiniGameKind::MilkFroth => "Flat white",
                MiniGameKind::LatteArt => "Café latte",
                MiniGameKind::PourOver => "Night filter",
                MiniGameKind::TeaSteep => "Grandma's tea",
                MiniGameKind::Plate => "Midnight snack",
                MiniGameKind::SoupStir => "Grandma's soup",
                MiniGameKind::MulledSpice => "Mulled wine",
            }
            .to_string()
        });
        let params = MiniGameParams::new(self.seed)
            .variant(self.variation)
            .relaxed(self.relaxed)
            .practised(self.practised)
            .regular(self.guest.as_deref().map(str::to_lowercase).as_deref());
        let mg = MiniGame::start(self.kind, params)
            .with_title(title)
            .with_text(text.clone());
        match &self.guest {
            Some(g) => mg.with_guest(g.clone(), "“Something warm, not too sweet — long day.”"),
            None => mg,
        }
    }
}

/// A blurry night café behind the overlay.
fn backdrop(ui: &Ui, t: f32) {
    let s = ui.screen();
    let th = &ui.theme;
    fill_rrect(
        s,
        0.0,
        Paint::vertical(s, th.violet_night[0], th.violet_night[1]),
    );
    // window with the street glow
    let win = Rect::new(s.w * 0.08, s.h * 0.1, s.w * 0.34, s.h * 0.46);
    fill_rrect(
        win,
        10.0,
        Paint::vertical(win, Color::from_hex(0x1f2448), Color::from_hex(0x2c2350)),
    );
    for i in 0..26u32 {
        let x = win.x + rd::hash01(i * 7) * win.w;
        let ph = (t * (0.4 + rd::hash01(i * 3) * 0.5) + rd::hash01(i * 11)).fract();
        let y = win.y + ph * win.h;
        draw_line(
            x,
            y,
            x - 2.0,
            y + 14.0,
            1.0,
            Color::new(0.7, 0.72, 1.0, 0.25),
        );
    }
    rd::glow(
        win.x + win.w * 0.7,
        win.y + win.h * 0.35,
        120.0,
        Color::new(0.5, 0.45, 1.0, 0.25),
    );
    // warm bokeh lamps
    for i in 0..14u32 {
        let x = rd::hash01(i * 13 + 1) * s.w;
        let y = s.h * (0.08 + rd::hash01(i * 5 + 2) * 0.55);
        let r = 18.0 + rd::hash01(i * 17) * 46.0;
        let warm = i % 3 != 0;
        let c = if warm {
            Color::new(1.0, 0.72, 0.38, 0.22)
        } else {
            Color::new(0.6, 0.58, 1.0, 0.16)
        };
        let f = 0.85 + 0.15 * (t * 0.7 + i as f32).sin();
        rd::glow(x, y, r * f, c);
    }
    // counter
    let counter = Rect::new(0.0, s.h * 0.72, s.w, s.h * 0.28);
    rd::rect_v(
        counter.x,
        counter.y,
        counter.w,
        counter.h,
        Color::from_hex(0x3a2433),
        Color::from_hex(0x1a1020),
    );
    rd::rect_v(
        0.0,
        counter.y - 6.0,
        s.w,
        8.0,
        Color::from_hex(0x6e4a52),
        Color::from_hex(0x3a2433),
    );
}

fn cat_gallery(ui: &mut Ui, t: f32) {
    let s = ui.screen();
    let th = ui.theme.clone();
    let card = s.centered(vec2(860.0, 420.0));
    ui.glass(card, &Glass::panel());
    let floor = Rect::new(card.x + 16.0, card.y + 50.0, card.w - 32.0, card.h - 66.0);
    fill_rrect(
        floor,
        10.0,
        Paint::vertical(floor, Color::from_hex(0x2a1f3a), Color::from_hex(0x140e20)),
    );
    rd::radial(
        floor.center().x,
        floor.y + 80.0,
        420.0,
        200.0,
        Color::new(1.0, 0.72, 0.42, 0.12),
        Color::new(1.0, 0.72, 0.42, 0.0),
    );
    ui.fonts
        .draw("Station cat", card.x + 18.0, card.y + 31.0, &th.h1());
    let poses = [
        (CatPose::Sit, 0.2, "Sit"),
        (CatPose::Walk, 0.5, "Walk"),
        (CatPose::Eat, 0.6, "Eat"),
        (CatPose::Sleep, 0.4, "Sleep"),
        (CatPose::Sit, 1.0, "Sit · affection 1.0"),
    ];
    let look_big = CatLook {
        scale: 2.4,
        ..CatLook::default()
    };
    for (i, (pose, aff, label)) in poses.iter().enumerate() {
        let x = floor.x + 90.0 + i as f32 * 166.0;
        let y = floor.y + 250.0;
        draw_cat_ex(vec2(x, y), &look_big, *pose, t + i as f32 * 0.7, *aff);
        ui.fonts.draw_in(
            label,
            Rect::new(x - 80.0, y + 30.0, 160.0, 18.0),
            &th.small(),
            Align::Center,
        );
    }
    // world scale (as in the café: 1 tile = 64 px)
    let small = CatLook::default();
    for (i, pose) in [CatPose::Sit, CatPose::Walk, CatPose::Eat, CatPose::Sleep]
        .iter()
        .enumerate()
    {
        draw_cat_ex(
            vec2(floor.x + 40.0 + i as f32 * 60.0, floor.y + 60.0),
            &small,
            *pose,
            t,
            0.3,
        );
    }
}

#[macroquad::main(conf)]
async fn main() {
    let fonts = Fonts::load().await.expect("fonts in assets/fonts");
    let mut ui = Ui::new(fonts);
    let mut setup = Setup::from_env();
    let text = load_text(setup.lang);
    let shot = env("SHIFTELLA_SHOT");
    let cat_demo = env("SHIFTELLA_DEMO").as_deref() == Some("cat");
    let target_t: f32 = env("SHIFTELLA_T")
        .and_then(|s| s.parse().ok())
        .unwrap_or(4.0);
    let frames = 40;
    let dt_fixed = 1.0 / 60.0;
    let mut autoplay = shot.is_some();
    let mut mg = setup.start(&text);
    let mut status = MiniGameStatus::Running;
    let mut t = 0.0f32;
    if shot.is_some() {
        // fast-forward the scripted run up to just before T
        let pre = (target_t - frames as f32 * dt_fixed).max(0.0);
        while mg.elapsed() < pre {
            let input = if setup.keyboard {
                mg.autoplay_keyboard_input()
            } else {
                mg.autoplay_input()
            };
            status = mg.step(&input, dt_fixed);
        }
        t = pre;
    }
    let mut frame = 0;
    let mut done_for = 0.0;
    loop {
        ui.begin_frame();
        let dt = if shot.is_some() {
            dt_fixed
        } else {
            get_frame_time().min(0.05)
        };
        t += dt;
        clear_background(ui.theme.canvas[0]);
        ui.set_camera();
        backdrop(&ui, t);
        if cat_demo {
            cat_gallery(&mut ui, t);
        } else {
            if autoplay {
                let input = if setup.keyboard {
                    mg.autoplay_keyboard_input()
                } else {
                    mg.autoplay_input()
                };
                status = mg.step(&input, dt);
                let _ = ui.esc();
            } else {
                status = mg.update(&mut ui, dt);
            }
            let screen = ui.screen();
            mg.draw(&mut ui, screen);
            for s in mg.drain_sounds() {
                eprintln!("sound: {s:?} -> {:?}", s.cue());
            }
            if let MiniGameStatus::Done { quality } = status {
                let st = ui.theme.small().color(ui.theme.text);
                let msg = format!("Done · quality {quality:.2} · F8 restarts");
                ui.fonts.draw_in(
                    &msg,
                    Rect::new(0.0, ui.screen().h - 40.0, ui.screen().w, 20.0),
                    &st,
                    Align::Center,
                );
                done_for += dt;
            }
        }
        ui.end_frame();
        let _ = ui.drain_sounds();

        // demo hotkeys
        let keys = [
            KeyCode::F2,
            KeyCode::F3,
            KeyCode::F4,
            KeyCode::F5,
            KeyCode::F6,
        ];
        let mut restart = ui.key(KeyCode::F8) || (done_for > 2.5 && shot.is_none());
        for (i, k) in keys.iter().enumerate() {
            if ui.key(*k) {
                setup.kind = MiniGameKind::ALL[i];
                restart = true;
            }
        }
        if ui.key(KeyCode::F7) {
            setup.seed += 1;
            match setup.kind {
                MiniGameKind::LatteArt => {
                    let i = setup
                        .variation
                        .shape
                        .map(|s| LatteShape::ALL.iter().position(|x| *x == s).unwrap_or(0) + 1)
                        .unwrap_or(0);
                    setup.variation.shape = Some(LatteShape::ALL[i % LatteShape::ALL.len()]);
                }
                MiniGameKind::Plate => {
                    let i = setup
                        .variation
                        .plate
                        .map(|s| PlateSet::ALL.iter().position(|x| *x == s).unwrap_or(0) + 1)
                        .unwrap_or(0);
                    setup.variation.plate = Some(PlateSet::ALL[i % PlateSet::ALL.len()]);
                }
                MiniGameKind::TeaSteep => {
                    let i = setup
                        .variation
                        .tea_kind
                        .map(|s| TeaKind::ALL.iter().position(|x| *x == s).unwrap_or(0) + 1)
                        .unwrap_or(0);
                    setup.variation.tea_kind = Some(TeaKind::ALL[i % TeaKind::ALL.len()]);
                }
                _ => {}
            }
            restart = true;
        }
        if ui.key(KeyCode::F1) {
            autoplay = !autoplay;
        }
        if restart {
            mg = setup.start(&text);
            done_for = 0.0;
        }

        if let Some(path) = &shot {
            frame += 1;
            if frame >= frames {
                let mut img = get_screen_data();
                for px in img.bytes.as_chunks_mut::<4>().0 {
                    px[3] = 255;
                }
                img.export_png(path);
                eprintln!(
                    "minigame_demo: saved {path} at t={:.2}s ({:?})",
                    mg.elapsed(),
                    status
                );
                break;
            }
        }
        next_frame().await;
    }
}
