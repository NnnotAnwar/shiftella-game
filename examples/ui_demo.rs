//! UI kit gallery. `cargo run --example ui_demo`
//!
//! Pages: 1 Gallery · 2 HUD · 3 Shiftella app · 4 Cutscene · 5 Narration (F1–F5 or Tab).
//! Screenshot: `SHIFTELLA_SHOT=shots/x.png SHIFTELLA_PAGE=hud cargo run --example ui_demo`
//! Optional: `SHIFTELLA_UI_SCALE=1.2`, `SHIFTELLA_FRAMES=30`.

use macroquad::prelude::*;

use shiftella_game::ui::draw::{self, Paint, fill_rrect, glow_circle, hline, line, with_a};
use shiftella_game::ui::layout::{grid_cell, weighted_row};
use shiftella_game::ui::*;

fn conf() -> Conf {
    Conf {
        window_title: "Shiftella — UI kit".to_owned(),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        sample_count: 4,
        ..Default::default()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Page {
    Gallery,
    Hud,
    App,
    Cutscene,
    Narration,
}

const PAGES: [Page; 5] = [
    Page::Gallery,
    Page::Hud,
    Page::App,
    Page::Cutscene,
    Page::Narration,
];

struct Demo {
    page: Page,
    toggle_a: bool,
    toggle_b: bool,
    check_a: bool,
    check_b: bool,
    volume: f32,
    nights: f32,
    seg: usize,
    tab: usize,
    lang: usize,
    seed: String,
    cafe: String,
    selected_row: usize,
    checklist: [bool; 7],
    speed: usize,
    phone_tab: usize,
    tablet_tab: usize,
    dialogue: DialogueBox,
    narration: DialogueBox,
    line_i: usize,
    picked: Option<usize>,
    toasts_pushed: bool,
}

#[macroquad::main(conf)]
async fn main() {
    let fonts = Fonts::load().await.expect("fonts in assets/fonts");
    let mut ui = Ui::new(fonts);
    if let Some(s) = std::env::var("SHIFTELLA_UI_SCALE")
        .ok()
        .and_then(|s| s.parse().ok())
    {
        ui.set_scale(s);
    }
    let shot = std::env::var("SHIFTELLA_SHOT").ok();
    let frames: u32 = std::env::var("SHIFTELLA_FRAMES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);
    let page = match std::env::var("SHIFTELLA_PAGE").as_deref() {
        Ok("hud") => Page::Hud,
        Ok("app") | Ok("phone") => Page::App,
        Ok("cutscene") => Page::Cutscene,
        Ok("narration") => Page::Narration,
        _ => Page::Gallery,
    };
    let mut d = Demo {
        page,
        toggle_a: true,
        toggle_b: false,
        check_a: true,
        check_b: false,
        volume: 0.7,
        nights: 3.0,
        seg: 1,
        tab: 0,
        lang: 0,
        seed: "PRAHA-2039".to_string(),
        cafe: String::new(),
        selected_row: 1,
        checklist: [true, true, true, false, false, true, false],
        speed: 1,
        phone_tab: 1,
        tablet_tab: 0,
        dialogue: DialogueBox::new(),
        narration: DialogueBox::new(),
        line_i: 0,
        picked: None,
        toasts_pushed: false,
    };
    // SHIFTELLA_LOG=1: open the history log with a few past lines; SHIFTELLA_LINE=n: start line
    if std::env::var("SHIFTELLA_LOG").is_ok() {
        d.dialogue.log_open = true;
        for (who, text) in [
            (
                Some("Oksana"),
                "Dobrý večer, paní Lído. Venku leje jako z konve.",
            ),
            (None, "Za oknem tiše projel prázdný vlak."),
            (Some("Babička Lida"), "Zase o minutu dřív. Jako vždycky."),
            (
                Some("Matěj"),
                "Ďakujem, že ste ma zastúpili. Їжак ґречно їсть, jak se říká.",
            ),
        ] {
            d.dialogue.history.push_back(HistoryLine {
                speaker: who.map(str::to_string),
                text: text.to_string(),
            });
        }
    }
    if let Some(n) = std::env::var("SHIFTELLA_LINE")
        .ok()
        .and_then(|s| s.parse().ok())
    {
        d.line_i = n;
        d.picked = Some(0);
    }
    if shot.is_some() {
        d.dialogue.chars_per_sec = 0.0;
        d.narration.chars_per_sec = 0.0;
    }
    let mut frame = 0;
    let mut build_time = 0.0_f64;
    loop {
        let started = std::time::Instant::now();
        ui.begin_frame();
        clear_background(ui.theme.canvas[0]);
        ui.set_camera();
        backdrop(&ui, d.page);

        match d.page {
            Page::Gallery => gallery(&mut ui, &mut d),
            Page::Hud => hud(&mut ui, &mut d),
            Page::App => app(&mut ui, &mut d),
            Page::Cutscene => cutscene(&mut ui, &mut d),
            Page::Narration => narration(&mut ui, &mut d),
        }
        if d.page != Page::Cutscene && d.page != Page::Narration {
            page_switcher(&mut ui, &mut d);
        }
        for ev in ui.draw_toasts(ToastAnchor::BottomRight) {
            if ev.tag == 99 {
                ui.push_toast(Toast::new(
                    ToastKind::Info,
                    "Odebráno ze směny · Ctrl+Z vrátí",
                ));
            }
        }
        ui.end_frame();
        build_time += started.elapsed().as_secs_f64();

        // page hotkeys (after widgets, so consumed keys are respected)
        let fkeys = [
            KeyCode::F1,
            KeyCode::F2,
            KeyCode::F3,
            KeyCode::F4,
            KeyCode::F5,
        ];
        for (i, k) in fkeys.iter().enumerate() {
            if ui.key(*k) {
                d.page = PAGES[i];
            }
        }
        if ui.key(KeyCode::Tab) {
            let i = PAGES.iter().position(|p| *p == d.page).unwrap_or(0);
            d.page = PAGES[(i + 1) % PAGES.len()];
        }
        if ui.esc() && matches!(d.page, Page::Cutscene | Page::Narration) {
            d.page = Page::Gallery;
        }
        let _ = ui.drain_sounds();

        if let Some(path) = &shot {
            frame += 1;
            if frame >= frames {
                let mut img = get_screen_data();
                // the framebuffer's alpha is < 1 under translucent glass: make the PNG opaque
                for px in img.bytes.as_chunks_mut::<4>().0 {
                    px[3] = 255;
                }
                img.export_png(path);
                eprintln!(
                    "ui_demo: {:?} — avg UI build {:.2} ms/frame over {frames} frames",
                    d.page,
                    build_time * 1000.0 / frames as f64
                );
                break;
            }
        }
        next_frame().await;
    }
}

// ---------------------------------------------------------------------------
// Backdrop: night canvas + soft colour blobs (so the glass has something to frost)
// ---------------------------------------------------------------------------

fn backdrop(ui: &Ui, page: Page) {
    let t = &ui.theme;
    let s = ui.screen();
    let c = t.canvas;
    fill_rrect(
        Rect::new(0.0, 0.0, s.w, s.h * 0.55),
        0.0,
        Paint::vertical(Rect::new(0.0, 0.0, s.w, s.h * 0.55), c[0], c[1]),
    );
    fill_rrect(
        Rect::new(0.0, s.h * 0.55, s.w, s.h * 0.45 + 1.0),
        0.0,
        Paint::vertical(Rect::new(0.0, s.h * 0.55, s.w, s.h * 0.45), c[1], c[2]),
    );
    if matches!(page, Page::Hud | Page::Cutscene | Page::Narration) {
        fake_world(ui);
        return;
    }
    glow_circle(vec2(s.w * 0.18, s.h * 0.15), 420.0, with_a(t.indigo, 0.22));
    glow_circle(vec2(s.w * 0.82, s.h * 0.85), 460.0, with_a(t.violet, 0.20));
    glow_circle(
        vec2(s.w * 0.55, s.h * 0.45),
        260.0,
        with_a(t.amber_deep, 0.06),
    );
}

/// A stand-in for the café world: floor, lamp pools, windows with rain.
fn fake_world(ui: &Ui) {
    let t = &ui.theme;
    let s = ui.screen();
    let floor = Rect::new(s.w * 0.12, s.h * 0.2, s.w * 0.76, s.h * 0.68);
    draw::shadow_rrect(
        floor,
        10.0,
        draw::Shadow::new(0.0, 20.0, 60.0, 0.0, Color::new(0.0, 0.0, 0.0, 0.5)),
    );
    fill_rrect(
        floor,
        8.0,
        Paint::vertical(floor, Color::from_hex(0x3A2A33), Color::from_hex(0x2A1F2E)),
    );
    for i in 0..14 {
        let y = floor.y + 18.0 + i as f32 * 34.0;
        if y < floor.bottom() - 4.0 {
            hline(
                floor.x + 6.0,
                floor.right() - 6.0,
                y,
                Color::new(0.0, 0.0, 0.0, 0.18),
            );
        }
    }
    // back wall with windows
    let wall = Rect::new(floor.x, floor.y - 46.0, floor.w, 52.0);
    fill_rrect(
        wall,
        6.0,
        Paint::vertical(wall, Color::from_hex(0x2C2942), Color::from_hex(0x1C1A2B)),
    );
    for k in 0..4 {
        let wr = Rect::new(floor.x + 70.0 + k as f32 * 230.0, wall.y + 8.0, 120.0, 34.0);
        fill_rrect(
            wr,
            4.0,
            Paint::vertical(wr, Color::from_hex(0x3C5591), Color::from_hex(0x22305A)),
        );
        glow_circle(
            vec2(wr.x + 60.0, wr.bottom() + 40.0),
            110.0,
            with_a(Color::from_hex(0x8EA6FF), 0.10),
        );
        for j in 0..7 {
            let x = wr.x + 10.0 + j as f32 * 16.0;
            line(
                vec2(x, wr.y + 5.0 + (j % 3) as f32 * 6.0),
                vec2(x - 2.0, wr.y + 14.0 + (j % 3) as f32 * 6.0),
                1.0,
                Color::new(0.78, 0.83, 1.0, 0.5),
            );
        }
    }
    // tables + lamp pools
    for (i, p) in [
        vec2(0.3, 0.45),
        vec2(0.5, 0.42),
        vec2(0.7, 0.47),
        vec2(0.38, 0.7),
        vec2(0.62, 0.72),
    ]
    .iter()
    .enumerate()
    {
        let c = vec2(s.w * p.x, s.h * p.y);
        glow_circle(c, 120.0, with_a(t.amber, 0.16));
        let tr = Rect::new(c.x - 30.0, c.y - 22.0, 60.0, 44.0);
        draw::shadow_rrect(
            tr,
            10.0,
            draw::Shadow::new(3.0, 6.0, 12.0, 0.0, Color::new(0.0, 0.0, 0.0, 0.45)),
        );
        fill_rrect(
            tr,
            10.0,
            Paint::vertical(tr, Color::from_hex(0xB98457), Color::from_hex(0x8C5D3C)),
        );
        if i % 2 == 0 {
            draw::fill_circle(c + vec2(10.0, -4.0), 5.0, Color::from_hex(0xEEE6DA));
        }
    }
    // counter
    let counter = Rect::new(floor.x + 30.0, floor.bottom() - 70.0, 260.0, 40.0);
    fill_rrect(
        counter,
        6.0,
        Paint::vertical(
            counter,
            Color::from_hex(0xC79A6B),
            Color::from_hex(0xA9784E),
        ),
    );
    // night darkness
    fill_rrect(s, 0.0, with_a(Color::from_hex(0x07061A), 0.28));
}

fn page_switcher(ui: &mut Ui, d: &mut Demo) {
    let labels = ["Gallery", "HUD", "Shiftella", "Cutscene", "Narration"];
    let w = 420.0;
    let r = anchored(ui.screen(), Anchor::Bottom, vec2(w, 30.0), 10.0);
    if d.page == Page::Hud {
        return;
    }
    let mut idx = PAGES.iter().position(|p| *p == d.page).unwrap_or(0);
    ui.glass(r.grow(3.0), &Glass::hud().radius(11.0));
    if ui.segmented("pages", r, &labels, &mut idx).changed {
        d.page = PAGES[idx];
    }
}

// ---------------------------------------------------------------------------
// 1. Gallery
// ---------------------------------------------------------------------------

fn card(ui: &mut Ui, r: Rect, title: &str) -> Rect {
    ui.glass(r, &Glass::panel());
    let t = ui.theme.clone();
    ui.fonts.draw(title, r.x + 16.0, r.y + 27.0, &t.h1());
    ui.divider(r.x + 1.0, r.right() - 1.0, r.y + 40.0);
    Rect::new(r.x + 16.0, r.y + 52.0, r.w - 32.0, r.h - 64.0)
}

fn gallery(ui: &mut Ui, d: &mut Demo) {
    let t = ui.theme.clone();
    let s = ui.screen();
    let cols = weighted_row(
        Rect::new(16.0, 16.0, s.w - 32.0, s.h - 64.0),
        &[1.1, 0.95, 1.05],
        12.0,
    );

    // --- column 1: buttons + inputs -------------------------------------
    let c1 = cols[0];
    let (top, bottom) = c1.split_top(286.0, 12.0);
    let inner = card(ui, top, "Buttons");
    let mut f = Flow::column(inner, 8.0);
    let row1 = f.next(28.0);
    let mut fr = Flow::row(row1, 6.0);
    let b = Button::new("Začít noc")
        .primary()
        .icon(Icon::Play)
        .shortcut("Enter");
    let w = ui.button_width(&b);
    ui.button("start", fr.next(w), &b);
    let b = Button::new("Najmout").icon(Icon::Plus).shortcut("H");
    let w = ui.button_width(&b);
    ui.button("hire", fr.next(w), &b);
    let rest = fr.rest();
    ui.button("ghost", rest, &Button::new("Zrušit").ghost());

    let mut fr = Flow::row(f.next(28.0), 6.0);
    let b = Button::new("Vyhodit").danger().icon(Icon::Close);
    let w = ui.button_width(&b);
    ui.button("fire", fr.next(w), &b);
    let b = Button::new("Zamčeno")
        .icon(Icon::Lock)
        .disabled(true)
        .tooltip("Odemkne se v noci 5");
    let w = ui.button_width(&b);
    ui.button("locked", fr.next(w), &b);
    let b = Button::new("Vybráno").selected(true);
    let w = ui.button_width(&b);
    ui.button("sel", fr.next(w), &b);
    let b = Button::new("Další").trailing(Icon::ChevronRight).ghost();
    let w = ui.button_width(&b);
    ui.button("next", fr.next(w), &b);

    // toolbar style (ghost + keycaps)
    let mut fr = Flow::row(f.next(28.0), 2.0);
    for (i, (ic, l, k)) in [
        (Icon::Crew, "Tým", "C"),
        (Icon::Priorities, "Priority", "P"),
        (Icon::Phone, "Shiftella", "T"),
    ]
    .iter()
    .enumerate()
    {
        let b = Button::new(l)
            .ghost()
            .icon(*ic)
            .shortcut(k)
            .selected(i == 2);
        let w = ui.button_width(&b);
        ui.button(("tb", i), fr.next(w), &b);
    }
    f.space(2.0);
    ui.section(f.next(14.0), "Icon buttons · badges · keycaps");
    let mut fr = Flow::row(f.next(28.0), 4.0);
    for (i, ic) in [Icon::Pause, Icon::Play, Icon::Fast, Icon::Faster]
        .iter()
        .enumerate()
    {
        ui.icon_button(
            ("spd", i),
            fr.next(28.0),
            *ic,
            &IconButton::secondary()
                .selected(i == d.speed)
                .tooltip("Rychlost · 1/2/3"),
        );
    }
    fr.space(8.0);
    ui.icon_button(
        "bell",
        fr.next(28.0),
        Icon::Bell,
        &IconButton::ghost().badge(3),
    );
    ui.icon_button("set", fr.next(28.0), Icon::Settings, &IconButton::ghost());
    ui.icon_button("vol", fr.next(28.0), Icon::Volume, &IconButton::ghost());
    fr.space(8.0);
    let kr = fr.next(120.0);
    let mut x = kr.x;
    for k in ["Ctrl", "Z", "Esc"] {
        let r = ui.keycap(vec2(x, kr.center_y()), k, false);
        x = r.right() + 4.0;
    }
    f.space(4.0);
    ui.section(f.next(14.0), "Chips");
    let mut x = inner.x;
    let y = f.next(22.0).y;
    for (txt, tone, ic) in [
        ("Ranní ptáče", Tone::Ok, None),
        ("Líný", Tone::Danger, None),
        ("Vlak 00:15", Tone::Info, Some(Icon::Train)),
        ("Déšť", Tone::Neutral, Some(Icon::Rain)),
    ] {
        let r = ui.chip(vec2(x, y), txt, tone, ic);
        x = r.right() + 6.0;
    }

    let inner = card(ui, bottom, "Inputs");
    let mut f = Flow::column(inner, 8.0);
    ui.toggle_row("t1", f.next(24.0), "Hudba", &mut d.toggle_a);
    ui.toggle_row("t2", f.next(24.0), "Pauza při událostech", &mut d.toggle_b);
    let (l, r) = f.next(24.0).split_left(150.0, 8.0);
    ui.checkbox("c1", l, "Ďakujem, ľúbim", &mut d.check_a, false);
    ui.checkbox("c2", r, "Automaticky uložit", &mut d.check_b, false);
    let (l, r) = f.next(24.0).split_left(90.0, 8.0);
    ui.label(l, "Hlasitost", &t.small(), Align::Left);
    let (val, sl) = r.split_right(40.0, 8.0);
    ui.slider("vol", sl, &mut d.volume, 0.0, 1.0, None);
    ui.label(
        val,
        &format!("{:.0} %", d.volume * 100.0),
        &t.number().size(12.0),
        Align::Right,
    );
    let (l, r) = f.next(24.0).split_left(90.0, 8.0);
    ui.label(l, "Noci", &t.small(), Align::Left);
    let (val, sl) = r.split_right(40.0, 8.0);
    ui.slider("nights", sl, &mut d.nights, 1.0, 5.0, Some(1.0));
    ui.label(
        val,
        &format!("{}", d.nights as i32),
        &t.number().size(12.0),
        Align::Right,
    );
    ui.segmented(
        "diff",
        f.next(26.0),
        &["Cozy", "Classic", "Sandbox"],
        &mut d.seg,
    );
    let (l, r) = f.next(32.0).split_left(150.0, 8.0);
    ui.text_input("seed", l, &mut d.seed, "Seed", 24);
    ui.text_input("cafe", r, &mut d.cafe, "Název kavárny…", 32);
    let (l, _) = f.next(28.0).split_left(150.0, 8.0);
    ui.dropdown(
        "lang",
        l,
        &["Čeština", "English", "Slovenčina", "Українська"],
        &mut d.lang,
    );

    // --- column 2: lists --------------------------------------------------
    let c2 = cols[1];
    let inner = card(ui, c2, "Crew · lists");
    let mut f = Flow::column(inner, 6.0);
    let tabs = [
        Tab::new("Směna").icon(Icon::Crew).badge(5),
        Tab::new("Všichni"),
        Tab::new("Uchazeči").badge(3),
    ];
    ui.tabs("tabs", f.next(32.0), &tabs, &mut d.tab);
    f.space(2.0);
    let staff = [
        (
            "Lída Nováková",
            "Za barem · káva",
            Some(t.emerald_400),
            "92 %",
        ),
        ("Oksana Kovalenko", "Pokladna", Some(t.emerald_400), "61 %"),
        ("Matěj Dvořák", "Přestávka", Some(t.amber), "34 %"),
        ("Ľubica Horváthová", "Mimo směnu", None, "18 %"),
    ];
    for (i, (n, sub, st, v)) in staff.iter().enumerate() {
        let tone = if i == 3 { Tone::Danger } else { Tone::Neutral };
        let row = ListRow::new(n)
            .subtitle(sub)
            .avatar(n, *st)
            .trailing(v, tone)
            .selected(i == d.selected_row)
            .dim(st.is_none());
        if ui.list_row(("staff", i), f.next(44.0), &row).clicked {
            d.selected_row = i;
        }
    }
    let row = ListRow::new("Podklady pro mzdy")
        .subtitle("Týden 48 · 4 575 Kč")
        .icon(Icon::FileExport, t.indigo_300)
        .chevron();
    ui.list_row("payroll", f.next(44.0), &row);
    f.space(4.0);
    ui.section(f.next(14.0), "Needs");
    for (i, (need, label, v)) in [
        (Need::Energy, "Energie", 0.82),
        (Need::Mood, "Nálada", 0.46),
        (Need::Hunger, "Hlad", 0.18),
    ]
    .iter()
    .enumerate()
    {
        let r = f.next(16.0);
        let (l, rest) = r.split_left(70.0, 8.0);
        ui.label(l, label, &t.small().color(t.text), Align::Left);
        let (bar, val) = rest.split_right(36.0, 8.0);
        ui.need_bar(
            Rect::new(bar.x, bar.center_y() - 3.0, bar.w, 6.0),
            *need,
            *v,
        );
        let col = if *v < 0.25 { t.red_400 } else { t.text_dim };
        ui.label(
            val,
            &format!("{:.0} %", v * 100.0),
            &t.number().size(12.0).color(col),
            Align::Right,
        );
        let _ = i;
    }
    f.space(4.0);
    ui.section(f.next(14.0), "Checklist · scroll area");
    let view = f.rest();
    let items = [
        "Zapnout kávovar",
        "Doplnit mléko a croissanty",
        "Zkontrolovat pokladnu (1 250 Kč)",
        "Rozsvítit vývěsní štít",
        "Otevřít dveře v 22:00",
        "Předávka od denní směny",
        "Zamknout sklad",
    ];
    let content_h = items.len() as f32 * 30.0;
    let sa = ui.begin_scroll("chk", view, content_h);
    for (i, it) in items.iter().enumerate() {
        let r = Rect::new(view.x, sa.top() + i as f32 * 30.0, view.w - 8.0, 28.0);
        if sa.visible(r.y, r.h) {
            let meta = if d.checklist[i] { Some("22:05") } else { None };
            ui.check_item(("chk", i), r, it, meta, &mut d.checklist[i]);
        }
    }
    ui.end_scroll(sa);

    // --- column 3: type + icons --------------------------------------------
    let c3 = cols[2];
    let (top, bottom) = c3.split_top(318.0, 12.0);
    let inner = card(ui, top, "Typography");
    let mut y = inner.y;
    let samples: [(&str, TextStyle); 6] = [
        ("Shiftella", t.display().size(30.0)),
        ("Příliš žluťoučký kůň", t.title()),
        ("Ďakujem, ľúbim · Їжак ґречно їсть", t.h2()),
        ("Body 13 — Café Nocturne, Praha-Holešovice.", t.body()),
        ("Small 12 — noční vlak Rn 333 přijede ve 03:33", t.small()),
        ("SECTION · MICRO 11", t.micro()),
    ];
    for (s_, st) in samples {
        let lh = st.line_h().max(st.size + 6.0);
        ui.fonts
            .draw_in(s_, Rect::new(inner.x, y, inner.w, lh), &st, Align::Left);
        y += lh + 2.0;
    }
    y += 4.0;
    let money = t.h1().color(t.amber).tabular();
    ui.fonts.draw("1 250 Kč", inner.x, y + 16.0, &money);
    ui.fonts
        .draw("12 480 Kč", inner.x + 110.0, y + 16.0, &money);
    let clock = t.clock();
    ui.fonts
        .draw_right("03:33", inner.right(), y + 18.0, &clock);
    ui.fonts.draw_right(
        "01:11",
        inner.right() - 74.0,
        y + 18.0,
        &clock.color(t.text_dim),
    );
    y += 30.0;
    let para = "Babička Lida věří, že vlak Rn 333 jednou přiveze zpět jejího snoubence. Uplynulo šedesát let, ale v kavárně se pořád svítí až do rána.";
    let used = ui.fonts.draw_wrapped(
        para,
        Rect::new(inner.x, y, inner.w, 60.0),
        &t.small().color(t.text_dim),
        Align::Left,
        Some(2),
    );
    y += used + 4.0;
    ui.fonts.draw_in(
        "Jedna dlouhá řádka, která se nevejde a skončí výpustkou na konci textu",
        Rect::new(inner.x, y, inner.w * 0.7, 18.0),
        &t.body(),
        Align::Left,
    );

    let inner = card(ui, bottom, &format!("Icons · {}", Icon::ALL.len()));
    let cols_n = ((inner.w + 4.0) / 34.0).floor() as usize;
    for (i, ic) in Icon::ALL.iter().enumerate() {
        let r = grid_cell(
            vec2(inner.x, inner.y - 4.0),
            vec2(30.0, 30.0),
            vec2(4.0, 4.0),
            cols_n,
            i,
        );
        let resp = ui.interact(ui.id(("icon", i)), r);
        if resp.hover_t > 0.01 {
            fill_rrect(r, 6.0, Color::new(1.0, 1.0, 1.0, 0.07 * resp.hover_t));
        }
        let col = match i % 7 {
            0 => t.indigo_300,
            3 => t.amber,
            5 => t.violet_400,
            _ => t.text,
        };
        draw_icon(*ic, vec2(r.x + 15.0, r.y + 15.0), 20.0, col);
        ui.tooltip(&resp, ic.name());
    }
}

// ---------------------------------------------------------------------------
// 2. HUD over the (fake) world
// ---------------------------------------------------------------------------

fn hud(ui: &mut Ui, d: &mut Demo) {
    let t = ui.theme.clone();
    let s = ui.screen();
    if !d.toasts_pushed {
        d.toasts_pushed = true;
        ui.push_toast(
            Toast::new(ToastKind::Info, "Tip od hosta")
                .body("+40 Kč · Pan Šimek byl spokojený")
                .icon(Icon::Money),
        );
        ui.push_toast(
            Toast::new(ToastKind::Warning, "@shiftella_bot: Matěj má zpoždění")
                .body("Píše, že dorazí ve 22:20. Kdo ho zastoupí u pokladny?")
                .icon(Icon::Clock)
                .action("Oksana", 1)
                .action("Počkat", 2),
        );
        ui.push_toast(
            Toast::new(ToastKind::Event, "Vlak Rn 333 přijíždí")
                .body("Stanice Frýdlant → Holešovice · ~8 hostů za 4 minuty")
                .icon(Icon::Train)
                .primary_action("Ukázat", 3)
                .sticky(),
        );
    }

    // clock + speed
    let r = anchored(s, Anchor::TopLeft, vec2(340.0, 64.0), 12.0);
    ui.glass(r, &Glass::hud());
    draw_icon(Icon::Moon, vec2(r.x + 24.0, r.y + 26.0), 18.0, t.indigo_300);
    let w = ui.fonts.draw("03:33", r.x + 42.0, r.y + 34.0, &t.clock());
    ui.fonts
        .draw("Pá · noc 3", r.x + 50.0 + w, r.y + 33.0, &t.small());
    let mut fr = Flow::row_rev(Rect::new(r.x, r.y + 12.0, r.w - 12.0, 28.0), 2.0);
    for (i, ic) in [Icon::Faster, Icon::Fast, Icon::Play, Icon::Pause]
        .iter()
        .enumerate()
    {
        let idx = 3 - i;
        if ui
            .icon_button(
                ("speed", idx),
                fr.next(28.0),
                *ic,
                &IconButton::ghost().size(16.0).selected(d.speed == idx),
            )
            .clicked
        {
            d.speed = idx;
        }
    }
    // night progress with train markers
    let bar = Rect::new(r.x + 14.0, r.bottom() - 14.0, r.w - 28.0, 5.0);
    let prog = 0.69;
    fill_rrect(bar, 2.5, Color::new(1.0, 1.0, 1.0, 0.08));
    let fb = Rect::new(bar.x, bar.y, bar.w * prog, bar.h);
    fill_rrect(fb, 2.5, Paint::horizontal(bar, t.violet, t.indigo_300));
    let dawn = Rect::new(bar.x + bar.w * 0.875, bar.y, bar.w * 0.125, bar.h);
    fill_rrect(
        dawn,
        2.5,
        Paint::horizontal(dawn, with_a(t.amber, 0.0), with_a(t.amber, 0.45)),
    );
    for tp in [0.1, 0.28, 0.86] {
        let x = bar.x + bar.w * tp;
        draw::fill_convex(
            &[
                vec2(x - 3.0, bar.y - 6.0),
                vec2(x + 3.0, bar.y - 6.0),
                vec2(x, bar.y - 2.0),
            ],
            Paint::Solid(t.amber),
        );
    }
    let knob = vec2(fb.right(), bar.center_y());
    glow_circle(knob, 10.0, with_a(t.indigo_300, 0.5));
    draw::fill_circle(knob, 4.5, WHITE);

    // event ribbon
    let er = anchored(s, Anchor::Top, vec2(400.0, 32.0), 12.0);
    ui.glass(er, &Glass::hud().radius(8.0));
    draw_icon(
        Icon::Train,
        vec2(er.x + 18.0, er.center_y()),
        16.0,
        t.violet_400,
    );
    ui.fonts.draw(
        "03:29",
        er.x + 34.0,
        ui.fonts.baseline_in(er, &t.number().size(12.0)),
        &t.number().size(12.0).color(t.text_disabled),
    );
    ui.label(
        Rect::new(er.x + 76.0, er.y, er.w - 120.0, er.h),
        "Vlak Rn 333 projel stanicí Frýdlant — příští zastávka Holešovice",
        &t.body_medium(),
        Align::Left,
    );
    draw::vline(er.right() - 40.0, er.y + 8.0, er.bottom() - 8.0, t.divider);
    ui.keycap(vec2(er.right() - 30.0, er.center_y()), "L", false);

    // resources
    let rr = anchored(s, Anchor::TopRight, vec2(340.0, 64.0), 12.0);
    ui.glass(rr, &Glass::hud());
    let base = rr.y + 30.0;
    draw_icon(Icon::Money, vec2(rr.x + 22.0, base - 5.0), 18.0, t.amber);
    let w = ui.fonts.draw(
        "12 480 Kč",
        rr.x + 38.0,
        base,
        &t.h1().color(t.amber).tabular(),
    );
    let mut x = rr.x + 46.0 + w + 12.0;
    draw_icon(Icon::Star, vec2(x, base - 5.0), 16.0, t.amber);
    x += 12.0;
    let w = ui.fonts.draw("4,2", x, base, &t.h2().tabular());
    ui.fonts
        .draw("+0,1", x + w + 4.0, base, &t.key().color(t.emerald_400));
    x += w + 44.0;
    draw_icon(Icon::User, vec2(x, base - 5.0), 16.0, t.text_dim);
    ui.fonts.draw("14 / 20", x + 12.0, base, &t.h2().tabular());
    hline(rr.x + 12.0, rr.right() - 12.0, rr.y + 40.0, t.divider);
    let small = t.small();
    ui.fonts.draw(
        "+1 860 Kč za noc",
        rr.x + 14.0,
        rr.y + 56.0,
        &small.color(t.emerald_400),
    );
    ui.fonts.draw("fronta 3", rr.x + 134.0, rr.y + 56.0, &small);
    draw_icon(
        Icon::Rain,
        vec2(rr.x + 210.0, rr.y + 52.0),
        14.0,
        t.indigo_300,
    );
    ui.fonts
        .draw("déšť, 9 °C", rr.x + 222.0, rr.y + 56.0, &small);

    // crew panel (open, left)
    let pr = Rect::new(12.0, 88.0, 300.0, s.h - 88.0 - 64.0);
    ui.glass(pr, &Glass::panel());
    let (content, _close) = ui.panel_header(pr, "Tým", Some("C"), Some("4 na směně"));
    let mut f = Flow::column(content.inset_xy(8.0, 8.0), 4.0);
    let names = [
        (
            "Lída Nováková",
            "Kávovar · latte",
            Some(t.emerald_400),
            0.86,
            0.72,
            0.6,
        ),
        (
            "Oksana Kovalenko",
            "Pokladna",
            Some(t.emerald_400),
            0.58,
            0.81,
            0.4,
        ),
        (
            "Matěj Dvořák",
            "Na cestě · 22:20",
            Some(t.amber),
            0.31,
            0.44,
            0.7,
        ),
        (
            "Ľubica Horváthová",
            "Úklid stolů",
            Some(t.emerald_400),
            0.21,
            0.55,
            0.18,
        ),
    ];
    for (i, (n, sub, st, e, m, h)) in names.iter().enumerate() {
        let r = f.next(48.0);
        if ui
            .list_row(
                ("crew", i),
                r,
                &ListRow::new(n)
                    .subtitle(sub)
                    .avatar(n, *st)
                    .selected(i == d.selected_row),
            )
            .clicked
        {
            d.selected_row = i;
        }
        let bx = r.right() - 92.0;
        for (k, (need, v)) in [(Need::Energy, *e), (Need::Mood, *m), (Need::Hunger, *h)]
            .iter()
            .enumerate()
        {
            ui.need_bar(
                Rect::new(bx + k as f32 * 28.0, r.center_y() - 2.0, 24.0, 4.0),
                *need,
                *v,
            );
        }
    }
    f.space(6.0);
    let sel = names[d.selected_row.min(3)];
    let card_r = f.next(196.0);
    ui.glass(card_r, &Glass::card());
    let cr = card_r.inset(12.0);
    ui.avatar(vec2(cr.x + 22.0, cr.y + 22.0), 44.0, sel.0, None, true);
    ui.fonts.draw(sel.0, cr.x + 56.0, cr.y + 18.0, &t.h2());
    ui.fonts.draw(
        "Barista · 24 let · 165 Kč/h",
        cr.x + 56.0,
        cr.y + 36.0,
        &t.small(),
    );
    let (mut x, mut cy) = (cr.x, cr.y + 54.0);
    for (txt, tone) in [
        ("Noční sova", Tone::Ok),
        ("Upovídaná", Tone::Neutral),
        ("Nemá ráda déšť", Tone::Danger),
    ] {
        if x + ui.chip_width(txt, false) > cr.right() {
            x = cr.x;
            cy += 28.0;
        }
        let r = ui.chip(vec2(x, cy), txt, tone, None);
        x = r.right() + 6.0;
    }
    let mut yy = cy + 34.0;
    for (need, label, v) in [
        (Need::Energy, "Energie", sel.3),
        (Need::Mood, "Nálada", sel.4),
        (Need::Hunger, "Hlad", sel.5),
    ] {
        ui.label(
            Rect::new(cr.x, yy, 60.0, 14.0),
            label,
            &t.small().color(t.text),
            Align::Left,
        );
        ui.need_bar(Rect::new(cr.x + 64.0, yy + 4.0, cr.w - 108.0, 6.0), need, v);
        ui.label(
            Rect::new(cr.right() - 36.0, yy, 36.0, 14.0),
            &format!("{:.0} %", v * 100.0),
            &t.number().size(12.0),
            Align::Right,
        );
        yy += 18.0;
    }
    let br = f.next(28.0);
    let (a, b) = br.split_left(br.w / 2.0 - 3.0, 6.0);
    ui.button("break", a, &Button::new("Pauza").icon(Icon::Coffee));
    if ui
        .button("remove", b, &Button::new("Ze směny").danger())
        .clicked
    {
        ui.push_toast(Toast::new(ToastKind::Success, "Hotovo").body("Ľubica jde domů"));
    }

    // toolbar
    let items = [
        (Icon::Crew, "Tým", "C"),
        (Icon::Priorities, "Priority", "P"),
        (Icon::Phone, "Shiftella", "T"),
        (Icon::Build, "Stavba", "B"),
        (Icon::Log, "Deník", "L"),
    ];
    let widths: Vec<f32> = items
        .iter()
        .map(|(ic, l, k)| ui.button_width(&Button::new(l).ghost().icon(*ic).shortcut(k)))
        .collect();
    let total: f32 = widths.iter().sum::<f32>() + 4.0 * (items.len() as f32) + 36.0;
    let tb = anchored(s, Anchor::BottomLeft, vec2(total, 36.0), 12.0);
    ui.glass(tb, &Glass::hud());
    let mut fr = Flow::row(tb.inset(4.0), 4.0);
    for (i, ((ic, l, k), w)) in items.iter().zip(widths).enumerate() {
        ui.button(
            ("tool", i),
            fr.next(w),
            &Button::new(l)
                .ghost()
                .icon(*ic)
                .shortcut(k)
                .selected(i == 0),
        );
    }
    ui.icon_button(
        "menu",
        fr.next(28.0),
        Icon::Settings,
        &IconButton::ghost().tooltip("Menu · Esc"),
    );
}

// ---------------------------------------------------------------------------
// 3. Shiftella app on tablet + phone
// ---------------------------------------------------------------------------

const TABS: [(Icon, &str); 5] = [
    (Icon::Calendar, "Rozpis"),
    (Icon::Swap, "Výměny"),
    (Icon::Plane, "Volno"),
    (Icon::Megaphone, "Nástěnka"),
    (Icon::ClipboardCheck, "Checklist"),
];

fn app(ui: &mut Ui, d: &mut Demo) {
    let s = ui.screen();
    let th = (s.h - 120.0).min(560.0);
    let tsize = Device::Tablet.size_for_height(th);
    let psize = Device::Phone.size_for_height((s.h - 70.0).min(640.0));
    let gap = 36.0;
    let x0 = (s.w - tsize.x - psize.x - gap) / 2.0;
    let tablet_r = Rect::new(x0, (s.h - 40.0 - tsize.y) / 2.0, tsize.x, tsize.y);
    let phone_r = Rect::new(
        x0 + tsize.x + gap,
        (s.h - 40.0 - psize.y) / 2.0,
        psize.x,
        psize.y,
    );

    let out = ui.phone_frame(
        "tablet",
        tablet_r,
        &PhoneFrame::new("03:33", &TABS)
            .tablet()
            .subtitle("Café Nocturne · Holešovice")
            .selected(d.tablet_tab)
            .badge(2)
            .user("Lída Nováková"),
    );
    if let Some(i) = out.tab_clicked {
        d.tablet_tab = i;
    }
    schedule_screen(ui, out.content);

    let out = ui.phone_frame(
        "phone",
        phone_r,
        &PhoneFrame::new("03:33", &TABS[..4])
            .title("Výměny směn")
            .selected(d.phone_tab)
            .badge(5)
            .battery(0.18),
    );
    if let Some(i) = out.tab_clicked {
        d.phone_tab = i;
    }
    swaps_screen(ui, d, out.content);
}

fn schedule_screen(ui: &mut Ui, r: Rect) {
    let t = ui.theme.clone();
    // header row
    let (head, body) = r.split_top(30.0, 8.0);
    ui.fonts.draw(
        "Rozpis · týden 48",
        head.x + 2.0,
        ui.fonts.baseline_in(head, &t.h1()),
        &t.h1(),
    );
    let mut fr = Flow::row_rev(head, 6.0);
    let b = Button::new("Publikovat").primary().icon(Icon::Check);
    let w = ui.button_width(&b);
    ui.button("publish", fr.next(w).inset_xy(0.0, 1.0), &b);
    let b = Button::new("Mzdy").icon(Icon::FileExport);
    let w = ui.button_width(&b);
    ui.button("payroll", fr.next(w).inset_xy(0.0, 1.0), &b);
    let chip_w = ui.chip_width("Koncept", true);
    let cr = fr.next(chip_w);
    ui.chip(
        vec2(cr.x, cr.center_y() - 11.0),
        "Koncept",
        Tone::Warn,
        Some(Icon::Note),
    );

    let days = [
        "Po 24", "Út 25", "St 26", "Čt 27", "Pá 28", "So 29", "Ne 30",
    ];
    let name_w = 150.0;
    let sum_w = 44.0;
    let grid = body.inset_ltrb(0.0, 0.0, 0.0, 0.0);
    ui.glass(grid, &Glass::card().radius(10.0));
    let inner = grid.inset(8.0);
    let col_w = (inner.w - name_w - sum_w) / 7.0;
    let hy = inner.y;
    let key_st = t.micro();
    ui.fonts
        .draw("ZAMĚSTNANEC", inner.x + 4.0, hy + 13.0, &key_st);
    for (i, dname) in days.iter().enumerate() {
        let x = inner.x + name_w + i as f32 * col_w;
        let today = i == 2;
        let st = t.key().color(if today {
            t.indigo_300
        } else if i >= 5 {
            t.violet_400
        } else {
            t.text_dim
        });
        ui.fonts
            .draw_centered(dname, x + col_w / 2.0, hy + 13.0, &st);
        if today {
            fill_rrect(
                Rect::new(x + col_w / 2.0 - 10.0, hy + 18.0, 20.0, 2.0),
                1.0,
                t.indigo_400,
            );
        }
    }
    ui.fonts
        .draw_centered("Σ H", inner.right() - sum_w / 2.0, hy + 13.0, &key_st);
    // rows
    #[derive(Clone, Copy)]
    enum C {
        Shift(&'static str),
        Late(&'static str),
        Leave,
        Toil,
        Off,
    }
    use C::*;
    let rows: [(&str, &str, [C; 7], &str); 6] = [
        (
            "Lída Nováková",
            "Vedoucí",
            [
                Shift("22–06"),
                Shift("22–06"),
                Off,
                Shift("22–06"),
                Shift("22–06"),
                Off,
                Off,
            ],
            "32",
        ),
        (
            "Oksana Kovalenko",
            "Barista",
            [
                Off,
                Shift("22–02"),
                Shift("22–06"),
                Shift("22–06"),
                Off,
                Shift("00–06"),
                Shift("22–06"),
            ],
            "34",
        ),
        (
            "Matěj Dvořák",
            "Pokladna",
            [
                Shift("22–06"),
                Late("22:20"),
                Shift("22–06"),
                Off,
                Leave,
                Leave,
                Leave,
            ],
            "22",
        ),
        (
            "Ľubica Horváthová",
            "Úklid",
            [
                Shift("02–06"),
                Off,
                Shift("02–06"),
                Shift("02–06"),
                Shift("02–06"),
                Toil,
                Off,
            ],
            "16",
        ),
        (
            "Šimon Bartoš",
            "Kuchař",
            [
                Shift("22–04"),
                Shift("22–04"),
                Shift("22–04"),
                Off,
                Shift("22–04"),
                Shift("22–04"),
                Off,
            ],
            "30",
        ),
        (
            "Їжак Ґречний",
            "Brigáda",
            [
                Off,
                Off,
                Off,
                Shift("00–04"),
                Shift("00–04"),
                Shift("22–06"),
                Shift("22–06"),
            ],
            "24",
        ),
    ];
    let row_h = ((inner.bottom() - hy - 28.0) / rows.len() as f32).min(46.0);
    for (ri, (name, role, cells, sum)) in rows.iter().enumerate() {
        let y = hy + 26.0 + ri as f32 * row_h;
        let rr = Rect::new(inner.x, y, inner.w, row_h - 2.0);
        if ri % 2 == 0 {
            fill_rrect(rr, 6.0, Color::new(1.0, 1.0, 1.0, 0.025));
        }
        ui.avatar(vec2(rr.x + 16.0, rr.center_y()), 26.0, name, None, false);
        ui.fonts.draw_in(
            name,
            Rect::new(rr.x + 36.0, rr.center_y() - 15.0, name_w - 40.0, 16.0),
            &t.strong().size(12.0),
            Align::Left,
        );
        ui.fonts.draw_in(
            role,
            Rect::new(rr.x + 36.0, rr.center_y() + 1.0, name_w - 40.0, 14.0),
            &t.small().size(11.0),
            Align::Left,
        );
        for (ci, c) in cells.iter().enumerate() {
            let cell = Rect::new(
                inner.x + name_w + ci as f32 * col_w + 3.0,
                rr.y + 6.0,
                col_w - 6.0,
                rr.h - 12.0,
            );
            match c {
                Shift(txt) | Late(txt) => {
                    let late = matches!(c, Late(_));
                    let (c0, c1) = if late {
                        (t.amber, t.amber_deep)
                    } else {
                        (t.indigo, t.violet)
                    };
                    draw::shadow_rrect(
                        cell,
                        6.0,
                        draw::Shadow::new(0.0, 2.0, 8.0, -1.0, with_a(c0, 0.35)),
                    );
                    fill_rrect(cell, 6.0, Paint::diagonal(cell, c0, c1));
                    draw::stroke_rrect(
                        cell,
                        6.0,
                        1.0,
                        Paint::vertical(
                            cell,
                            Color::new(1.0, 1.0, 1.0, 0.32),
                            Color::new(1.0, 1.0, 1.0, 0.04),
                        ),
                    );
                    let st = TextStyle::new(
                        11.0,
                        Weight::SemiBold,
                        if late {
                            Color::from_hex(0x2A1A0A)
                        } else {
                            WHITE
                        },
                    )
                    .tabular();
                    ui.fonts.draw_in(txt, cell, &st, Align::Center);
                }
                Leave => {
                    fill_rrect(cell, 6.0, with_a(t.amber, 0.12));
                    draw::stroke_rrect(cell, 6.0, 1.0, with_a(t.amber, 0.3));
                    ui.fonts
                        .draw_in("dovol.", cell, &t.key().color(t.amber), Align::Center);
                }
                Toil => {
                    fill_rrect(cell, 6.0, with_a(t.violet, 0.16));
                    draw::stroke_rrect(cell, 6.0, 1.0, with_a(t.violet_400, 0.35));
                    ui.fonts
                        .draw_in("NV", cell, &t.key().color(t.violet_400), Align::Center);
                }
                Off => {
                    ui.fonts
                        .draw_in("—", cell, &t.small().color(t.text_disabled), Align::Center);
                }
            }
        }
        let sr = Rect::new(inner.right() - sum_w, rr.y, sum_w, rr.h);
        ui.fonts.draw_in(sum, sr, &t.number(), Align::Center);
    }
}

fn swaps_screen(ui: &mut Ui, d: &mut Demo, r: Rect) {
    let t = ui.theme.clone();
    let mut seg = 0usize;
    let (top, body) = r.split_top(28.0, 10.0);
    ui.segmented("swap_seg", top, &["Čeká (2)", "Vyřízené"], &mut seg);
    // content taller than the view → clipped scroll
    let content_h = 600.0;
    let sa = ui.begin_scroll("swap_scroll", body, content_h);
    let mut y = sa.top();
    let w = body.w - 10.0; // keep clear of the scrollbar
    let x = body.x;
    for (i, (from, to, when, note)) in [
        (
            "Matěj Dvořák",
            "Oksana Kovalenko",
            "Pá 28. 11. · 22:00–06:00",
            "Mám zkoušku, prosím",
        ),
        (
            "Ľubica Horváthová",
            "Šimon Bartoš",
            "So 29. 11. · 02:00–06:00",
            "Výměna za neděli?",
        ),
    ]
    .iter()
    .enumerate()
    {
        let cr = Rect::new(x, y, w, 128.0);
        ui.glass(cr, &Glass::card());
        let c = cr.inset(12.0);
        ui.avatar(vec2(c.x + 14.0, c.y + 14.0), 28.0, from, None, false);
        draw_icon(Icon::Swap, vec2(c.x + 42.0, c.y + 14.0), 14.0, t.text_dim);
        ui.avatar(vec2(c.x + 70.0, c.y + 14.0), 28.0, to, None, false);
        ui.fonts.draw_in(
            from.split(' ').next().unwrap_or(from),
            Rect::new(c.x + 92.0, c.y, c.w - 92.0, 16.0),
            &t.strong(),
            Align::Left,
        );
        ui.fonts.draw_in(
            &format!("→ {}", to.split(' ').next().unwrap_or(to)),
            Rect::new(c.x + 92.0, c.y + 14.0, c.w - 92.0, 16.0),
            &t.small(),
            Align::Left,
        );
        ui.fonts.draw_in(
            when,
            Rect::new(c.x, c.y + 36.0, c.w, 16.0),
            &t.small_medium().color(t.indigo_300).tabular(),
            Align::Left,
        );
        ui.fonts.draw_in(
            note,
            Rect::new(c.x, c.y + 54.0, c.w, 16.0),
            &t.small(),
            Align::Left,
        );
        let (a, b) = Rect::new(c.x, c.bottom() - 26.0, c.w, 26.0).split_left(c.w / 2.0 - 3.0, 6.0);
        ui.button(
            ("ok", i),
            a,
            &Button::new("Schválit").primary().icon(Icon::Check),
        );
        ui.button(("no", i), b, &Button::new("Zamítnout").ghost());
        y += 128.0 + 10.0;
    }
    // announcement
    let ar = Rect::new(x, y, w, 96.0);
    ui.glass(ar, &Glass::card().tint(t.violet));
    let c = ar.inset(12.0);
    draw_icon(
        Icon::Megaphone,
        vec2(c.x + 9.0, c.y + 8.0),
        16.0,
        t.violet_400,
    );
    ui.fonts
        .draw("Nástěnka", c.x + 24.0, c.y + 13.0, &t.strong());
    ui.fonts
        .draw_right("včera", c.right(), c.y + 13.0, &t.small());
    ui.fonts.draw_wrapped(
        "Від понеділка нова кава в меню! Ochutnávka pro tým ve 21:45.",
        Rect::new(c.x, c.y + 24.0, c.w, 40.0),
        &t.small().color(t.text),
        Align::Left,
        Some(3),
    );
    y += 106.0;
    // checklist
    let lr = Rect::new(x, y, w, 30.0);
    ui.section(lr, "Zavírání · 4/7");
    y += 30.0;
    for (i, it) in ["Vypnout kávovar", "Spočítat tržbu", "Předávka ranní směně"]
        .iter()
        .enumerate()
    {
        let rr = Rect::new(x, y + i as f32 * 30.0, w - 8.0, 28.0);
        ui.check_item(("pchk", i), rr, it, None, &mut d.checklist[i + 3]);
    }
    ui.end_scroll(sa);
}

// ---------------------------------------------------------------------------
// 4. Cutscene: letterbox + dialogue + choices
// ---------------------------------------------------------------------------

const LINES: [(&str, &str); 3] = [
    (
        "Babička Lida",
        "Ten vlak… Rn 333. V roce 1979 jím odjel a slíbil, že se vrátí. Každou noc ve 03:33 nechám na stole jeden šálek navíc.",
    ),
    (
        "Oksana",
        "Пані Лідо, сьогодні знову дощ. Може, цього разу він таки приїде?",
    ),
    (
        "Babička Lida",
        "Uvidíme, děvče. Uvař mi, prosím, ještě jedno kakao.",
    ),
];

fn draw_portrait_placeholder(ui: &Ui, r: Rect, speaker: &str) {
    // the render agent will provide `draw_portrait`; this is a soft silhouette
    let t = &ui.theme;
    let c = vec2(r.x + r.w / 2.0, r.y + r.h * 0.42);
    glow_circle(c, r.w * 0.6, with_a(t.amber, 0.12));
    let hair = if speaker.starts_with("Oks") {
        Color::from_hex(0x3B2A22)
    } else {
        Color::from_hex(0xCFCBD6)
    };
    let body = Rect::new(r.x + r.w * 0.18, r.y + r.h * 0.62, r.w * 0.64, r.h * 0.5);
    ui.push_clip_static(r);
    fill_rrect(
        body,
        r.w * 0.3,
        Paint::vertical(body, Color::from_hex(0x4F46E5), Color::from_hex(0x2B2470)),
    );
    draw::fill_circle(c, r.w * 0.22, Color::from_hex(0xE9BFA0));
    draw::arc(c, r.w * 0.22, 3.3, 6.1, r.w * 0.1, hair);
    ui.pop_clip_static();
}

trait StaticClip {
    fn push_clip_static(&self, r: Rect);
    fn pop_clip_static(&self);
}

impl StaticClip for Ui {
    fn push_clip_static(&self, r: Rect) {
        draw::set_scissor(Some(r));
    }
    fn pop_clip_static(&self) {
        draw::set_scissor(None);
    }
}

fn cutscene(ui: &mut Ui, d: &mut Demo) {
    let free = ui.letterbox(1.0);
    d.dialogue.bottom_inset = ui.screen().h - free.bottom();
    let i = d.line_i.min(LINES.len() - 1);
    let (who, text) = LINES[i];
    let labels = DialogueLabels {
        skip: "Přeskočit",
        log: "Historie",
        history: "Historie",
    };
    let line = DialogueLine::new(who, text)
        .portrait(true)
        .color(if who == "Oksana" {
            ui.theme.violet
        } else {
            ui.theme.indigo
        })
        .awaiting_choice(i == 0 && d.picked.is_none())
        .labels(labels);
    let resp = d.dialogue.dialogue_box(ui, &line);
    if let Some(p) = resp.portrait {
        draw_portrait_placeholder(ui, p, who);
    }
    if resp.advance {
        d.line_i = (d.line_i + 1).min(LINES.len() - 1);
    }
    if line.awaiting_choice && resp.revealed {
        let choices = [
            "Nalít šálek navíc a čekat u okna",
            "„Babi, pojďte si sednout, uvařím to já.“",
            "Подивитися розклад потягів у Shiftella",
        ];
        if let Some(c) = ui.choice_list("choices", resp.rect, &choices) {
            d.picked = Some(c);
            d.line_i = 1;
        }
    }
}

fn narration(ui: &mut Ui, d: &mut Demo) {
    let resp = d.narration.narration_box(
        ui,
        "Praha-Holešovice, listopad 2039. Déšť ťuká na okna Café Nocturne a poslední noční vlak se ztrácí v mlze. Ніч тільки починається.",
    );
    ui.fade(0.0);
    if resp.advance {
        d.page = Page::Gallery;
    }
}
