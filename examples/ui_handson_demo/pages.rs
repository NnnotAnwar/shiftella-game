//! The demo pages: a stand-in café behind the glass, and every hands-on widget on mock data.

use macroquad::prelude::*;

use shiftella_game::ui::draw::{self, Paint, fill_rrect, glow_circle, hline, line, with_a};
use shiftella_game::ui::handson::*;
use shiftella_game::ui::*;

use crate::lang::{Lang, s as tr};
use crate::mock;
use crate::{PAGES, Page};

pub struct Demo {
    pub page: Page,
    pub lang: Lang,
    pub mode: BrewMode,
    pub plain_mode: BrewMode,
    pub rail: RailState,
    pub log: Vec<String>,
    pub duties: Option<Vec<Vec<Duty>>>,
    pub duties_st: DutiesState,
    pub playing: usize,
    pub repeat: bool,
    pub records_st: RecordState,
    pub book_st: BookState,
    pub pantry_st: BookState,
    pub tried: u32,
    pub cells: Vec<[ShiftCell; 7]>,
    pub rozpis_st: RozpisState,
    pub split: TipSplit,
    pub star: Option<usize>,
    pub granted: bool,
    pub sent: bool,
    pub journal_st: JournalState,
    pub photos: Vec<RenderTarget>,
    pub coach_step: usize,
    pub frame: u64,
}

impl Demo {
    pub fn new(page: Page, lang: Lang) -> Demo {
        Demo {
            page,
            lang,
            mode: BrewMode::ByHand,
            plain_mode: BrewMode::Quick,
            rail: RailState { focused: Some(1) },
            log: Vec::new(),
            duties: None,
            duties_st: DutiesState::default(),
            playing: 1,
            repeat: true,
            records_st: RecordState::default(),
            book_st: BookState {
                selected: Some(0),
                ..Default::default()
            },
            pantry_st: BookState {
                tab: 3,
                slots: vec![4, 6],
                jar_cursor: 4,
                ..Default::default()
            },
            tried: 0,
            cells: mock::rozpis_cells(),
            rozpis_st: RozpisState::default(),
            split: TipSplit::Half,
            star: Some(0),
            granted: false,
            sent: false,
            journal_st: JournalState::default(),
            photos: Vec::new(),
            coach_step: 0,
            frame: 0,
        }
    }

    /// Render targets (mock polaroid photos) are made before any UI is drawn.
    pub fn prepare_textures(&mut self) {
        self.frame += 1;
        if self.photos.is_empty() {
            for i in 0..5 {
                self.photos.push(photo(i));
            }
            set_default_camera();
        }
    }

    fn note(&mut self, what: String) {
        eprintln!("demo: {what}");
        self.log.push(what);
        if self.log.len() > 4 {
            self.log.remove(0);
        }
    }
}

pub fn draw(ui: &mut Ui, d: &mut Demo) {
    let world = matches!(
        d.page,
        Page::Hud | Page::Order | Page::Moment | Page::Rail | Page::Atmosphere | Page::Coach
    );
    backdrop(ui, world);
    match d.page {
        Page::Hud => hud(ui, d),
        Page::Order => order(ui, d),
        Page::Rail => rail(ui, d),
        Page::Moment => moment(ui, d),
        Page::Duties => duties(ui, d),
        Page::Records => records(ui, d),
        Page::Atmosphere => atmosphere_page(ui, d),
        Page::Rozpis => rozpis(ui, d),
        Page::Payroll => payroll(ui, d),
        Page::Journal => journal_page(ui, d),
        Page::Polaroid => polaroid_page(ui, d),
        Page::Coach => coach(ui, d),
        Page::Book => book(ui, d, false),
        Page::Pantry => book(ui, d, true),
    }
    header(ui, d);
}

// ---------------------------------------------------------------------------
// Chrome: page header, log
// ---------------------------------------------------------------------------

fn header(ui: &mut Ui, d: &mut Demo) {
    let t = ui.theme.clone();
    let s = ui.screen();
    let name = PAGES
        .iter()
        .find(|(p, _)| *p == d.page)
        .map(|(_, n)| *n)
        .unwrap_or("");
    let label = format!(
        "{name} · {} · {:.1}× · Tab / F2 / F3 F4",
        d.lang.code(),
        ui.scale()
    );
    let w = ui.fonts.measure(&label, &t.small()) + 24.0;
    let r = Rect::new(s.w - w - 10.0, s.h - 30.0, w, 22.0);
    fill_rrect(r, 11.0, Color::new(0.03, 0.02, 0.09, 0.55));
    ui.fonts.draw_in(&label, r, &t.small(), Align::Center);
    for (i, l) in d.log.iter().rev().enumerate() {
        ui.fonts.draw(
            l,
            14.0,
            s.h - 14.0 - i as f32 * 16.0,
            &t.small().color(with_a(t.text_dim, 1.0 - i as f32 * 0.2)),
        );
    }
}

// ---------------------------------------------------------------------------
// Backdrop: night canvas, or a stand-in café floor so the glass has a world under it
// ---------------------------------------------------------------------------

fn backdrop(ui: &Ui, world: bool) {
    let t = &ui.theme;
    let s = ui.screen();
    let c = t.canvas;
    fill_rrect(s, 0.0, Paint::vertical(s, c[0], c[2]));
    if world {
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

/// Table centres of the stand-in café (fractions of the screen).
pub const TABLES: [(f32, f32); 5] = [
    (0.3, 0.46),
    (0.5, 0.43),
    (0.7, 0.48),
    (0.38, 0.72),
    (0.62, 0.73),
];

pub fn table_point(ui: &Ui, i: usize) -> Vec2 {
    let s = ui.screen();
    vec2(s.w * TABLES[i].0, s.h * TABLES[i].1)
}

fn fake_world(ui: &Ui) {
    let t = &ui.theme;
    let s = ui.screen();
    let floor = Rect::new(s.w * 0.1, s.h * 0.2, s.w * 0.8, s.h * 0.7);
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
    for i in 0..16 {
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
    let wall = Rect::new(floor.x, floor.y - 46.0, floor.w, 52.0);
    fill_rrect(
        wall,
        6.0,
        Paint::vertical(wall, Color::from_hex(0x2C2942), Color::from_hex(0x1C1A2B)),
    );
    for k in 0..4 {
        let wr = Rect::new(
            floor.x + 70.0 + k as f32 * (floor.w - 200.0) / 3.0,
            wall.y + 8.0,
            120.0,
            34.0,
        );
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
    for (i, p) in TABLES.iter().enumerate() {
        let c = vec2(s.w * p.0, s.h * p.1);
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
    fill_rrect(s, 0.0, with_a(Color::from_hex(0x07061A), 0.28));
}

// ---------------------------------------------------------------------------
// Pages
// ---------------------------------------------------------------------------

fn order(ui: &mut Ui, d: &mut Demo) {
    let l = d.lang;
    let labels = mock::order_labels(l);
    let v = mock::mirek_order(l);
    let at = table_point(ui, 0);
    let out = order_card(ui, "order_mirek", at, &v, &labels, &mut d.mode, &[]);
    if let Some(a) = out.action {
        d.note(format!("order: {a:?} ({:?})", d.mode));
    }
    let p = mock::plain_order(l);
    let at2 = table_point(ui, 4);
    let out = order_card(ui, "order_plain", at2, &p, &labels, &mut d.plain_mode, &[]);
    if let Some(a) = out.action {
        d.note(format!("plain order: {a:?} ({:?})", d.plain_mode));
    }
}

fn rail(ui: &mut Ui, d: &mut Demo) {
    let l = d.lang;
    let s = ui.screen();
    let more = s_more(l);
    if let Some(a) = ticket_rail(
        ui,
        "rail",
        vec2(s.w / 2.0, 84.0),
        &mock::tickets(l),
        &mut d.rail,
        &more,
    ) {
        d.note(format!("rail: {a:?}"));
    }
    let mut st = RailState::default();
    let _ = ticket_rail(
        ui,
        "rail_many",
        vec2(s.w / 2.0, 200.0),
        &mock::many_tickets(l),
        &mut st,
        &more,
    );
}

fn s_more(l: Lang) -> String {
    tr(
        l,
        [
            "More orders waiting",
            "Další čekající objednávky",
            "Ще замовлення чекають",
            "Ещё заказы ждут",
        ],
    )
}

fn hud(ui: &mut Ui, d: &mut Demo) {
    let l = d.lang;
    let s = ui.screen();
    let atmo = Rect::new(14.0, s.h - 50.0, ATMO_SIZE.x, ATMO_SIZE.y);
    atmosphere_meter(ui, "atmo", atmo, &mock::atmosphere(l, 64.0));
    let more = s_more(l);
    let tickets = mock::tickets(l);
    let top = vec2(s.w / 2.0, 70.0);
    let rail = ticket_rail_rect(top, tickets.len());
    if let Some(a) = ticket_rail(ui, "rail", top, &tickets, &mut d.rail, &more) {
        d.note(format!("rail: {a:?}"));
    }
    // the order card keeps clear of the rail; the moment keeps clear of both
    let labels = mock::order_labels(l);
    let v = mock::mirek_order(l);
    let order = order_card_rect_hint(ui, &v, table_point(ui, 0), &[rail, atmo]);
    let mv = mock::moment_cat(l);
    let close = s_later(l);
    if let Some(a) = moment_card(
        ui,
        "m_cat",
        table_point(ui, 4),
        &mv,
        &close,
        &[rail, atmo, order],
    ) {
        d.note(format!("moment: {a:?}"));
    }
    if let Some(a) = order_card(
        ui,
        "order_mirek",
        table_point(ui, 0),
        &v,
        &labels,
        &mut d.mode,
        &[rail, atmo],
    )
    .action
    {
        d.note(format!("order: {a:?}"));
    }
}

/// Where the order card will go (same placement rule), so the moment can avoid it.
fn order_card_rect_hint(ui: &Ui, v: &OrderView, at: Vec2, avoid: &[Rect]) -> Rect {
    handson::common::place_near(ui.screen(), at, order_card_size(ui, v), 22.0, avoid)
}

fn s_later(l: Lang) -> String {
    tr(
        l,
        [
            "Later · Esc",
            "Později · Esc",
            "Пізніше · Esc",
            "Позже · Esc",
        ],
    )
}

fn moment(ui: &mut Ui, d: &mut Demo) {
    let l = d.lang;
    let close = s_later(l);
    let at = table_point(ui, 1);
    let cat = mock::moment_cat(l);
    let first = handson::moment_card_rect(ui, at, &cat, &[]);
    if let Some(a) = moment_card(ui, "m_cat", at, &cat, &close, &[]) {
        d.note(format!("moment cat: {a:?}"));
    }
    let at = table_point(ui, 4);
    if let Some(a) = moment_card(
        ui,
        "m_window",
        at,
        &mock::moment_window(l),
        &close,
        &[first],
    ) {
        d.note(format!("moment window: {a:?}"));
    }
}

fn duties(ui: &mut Ui, d: &mut Demo) {
    let l = d.lang;
    let mut v = mock::duties_view(l);
    let assigned = d
        .duties
        .get_or_insert_with(|| v.helpers.iter().map(|h| h.duties.clone()).collect());
    for (h, a) in v.helpers.iter_mut().zip(assigned.iter()) {
        h.duties = a.clone();
    }
    let s = ui.screen();
    let size = duties_board_size(v.helpers.len());
    let r = Rect::new(
        ((s.w - size.x) / 2.0).max(12.0).round(),
        60.0,
        size.x.min(s.w - 24.0),
        size.y.min(s.h - 100.0),
    );
    if let Some(a) = duties_board(
        ui,
        "duties",
        r,
        &v,
        &mock::duties_labels(l),
        &mut d.duties_st,
    ) {
        match a {
            DutiesAction::Toggle { helper, duty } => {
                let list = &mut assigned[helper];
                if let Some(i) = list.iter().position(|x| *x == duty) {
                    list.remove(i);
                } else if list.len() < v.helpers[helper].capacity as usize {
                    list.push(duty);
                }
            }
            DutiesAction::Move { duty, from, to }
                if assigned[to].len() < v.helpers[to].capacity as usize
                    && !assigned[to].contains(&duty) =>
            {
                assigned[from].retain(|x| *x != duty);
                assigned[to].push(duty);
            }
            _ => {}
        }
        d.note(format!("duties: {a:?}"));
    }
}

fn records(ui: &mut Ui, d: &mut Demo) {
    let l = d.lang;
    let v = mock::records_view(l, d.playing, d.repeat);
    let s = ui.screen();
    let at = vec2(s.w * 0.5, s.h * 0.62);
    if let Some(a) = record_picker(
        ui,
        "records",
        at,
        &v,
        &mock::record_labels(l),
        &mut d.records_st,
    ) {
        match a {
            RecordAction::Play(i) => d.playing = i,
            RecordAction::SetRepeat(r) => d.repeat = r,
            RecordAction::Close => {}
        }
        d.note(format!("records: {a:?}"));
    }
    // a little jukebox under the picker
    let jb = Rect::new(at.x - 36.0, at.y, 72.0, 96.0);
    draw::shadow_rrect(jb, 30.0, ui.theme.shadow_lg);
    fill_rrect(
        jb,
        30.0,
        Paint::vertical(jb, Color::from_hex(0xC98A43), Color::from_hex(0x6B3F1F)),
    );
    glow_circle(vec2(at.x, at.y + 34.0), 40.0, with_a(ui.theme.amber, 0.5));
    fill_rrect(
        Rect::new(at.x - 22.0, at.y + 14.0, 44.0, 40.0),
        20.0,
        with_a(Color::from_hex(0xFFE0A0), 0.8),
    );
}

fn book(ui: &mut Ui, d: &mut Demo, pantry: bool) {
    let l = d.lang;
    let result = if pantry {
        Some(if d.tried.is_multiple_of(2) {
            mock::discovered(l)
        } else {
            mock::close_result(l)
        })
    } else {
        None
    };
    let v = mock::book_view(l, !pantry, result, pantry);
    let labels = mock::book_labels(l);
    let st = if pantry {
        &mut d.pantry_st
    } else {
        &mut d.book_st
    };
    if let Some(a) = recipe_book(ui, "book", ui.screen(), &v, &labels, st) {
        if matches!(a, BookAction::Try(_)) {
            d.tried += 1;
        }
        d.note(format!("book: {a:?}"));
    }
}

// ---------------------------------------------------------------------------
// Mock photos: a tiny procedural café scene per polaroid, rendered once
// ---------------------------------------------------------------------------

fn photo(i: usize) -> RenderTarget {
    use shiftella_game::render::{Expression, PersonLook, draw_portrait};
    let (w, h) = (360.0, 270.0);
    let rt = render_target(w as u32, h as u32);
    rt.texture.set_filter(FilterMode::Linear);
    let mut cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, w, h));
    cam.render_target = Some(rt.clone());
    set_camera(&cam);
    clear_background(Color::from_hex(0x1B1530));
    let full = Rect::new(0.0, 0.0, w, h);
    let walls = [0x3A2A4A_u32, 0x2E2442, 0x45304A, 0x302A50, 0x3B2C3F];
    fill_rrect(
        full,
        0.0,
        Paint::vertical(
            full,
            Color::from_hex(walls[i % 5]),
            Color::from_hex(0x1A1424),
        ),
    );
    // window with night rain
    let win = Rect::new(30.0 + i as f32 * 14.0, 26.0, 130.0, 96.0);
    fill_rrect(
        win,
        4.0,
        Paint::vertical(win, Color::from_hex(0x2B4A8A), Color::from_hex(0x18264A)),
    );
    for j in 0..14 {
        let x = win.x + 8.0 + j as f32 * 9.0;
        let y = win.y + 6.0 + ((j * 17 + i * 5) % 40) as f32;
        line(
            vec2(x, y),
            vec2(x - 3.0, y + 14.0),
            1.2,
            Color::new(0.8, 0.86, 1.0, 0.45),
        );
    }
    draw::vline(
        win.center().x,
        win.y,
        win.bottom(),
        Color::from_hex(0x5A4A6A),
    );
    // lamp glow, a regular, the table and a cup
    glow_circle(vec2(250.0, 120.0), 170.0, Color::new(1.0, 0.72, 0.35, 0.35));
    let looks = ["mirek", "vera", "radek", "hana", "lori"];
    let look = PersonLook::preset(looks[i % 5]);
    let exprs = [
        Expression::Smile,
        Expression::Laugh,
        Expression::Neutral,
        Expression::Tired,
        Expression::Smile,
    ];
    draw_portrait(
        &look,
        exprs[i % 5],
        Rect::new(150.0, 60.0, 190.0, 220.0),
        1.0,
    );
    let table = Rect::new(0.0, 196.0, w, 80.0);
    fill_rrect(
        table,
        0.0,
        Paint::vertical(table, Color::from_hex(0xA8764A), Color::from_hex(0x6E4A2E)),
    );
    let dishes = [
        mock::cappuccino(),
        mock::lemon_tea(),
        mock::black(),
        mock::hot_choc(),
        mock::linden(),
    ];
    handson::draw_dish(vec2(110.0, 196.0), 74.0, dishes[i % 5], None, 0.4);
    // SAFETY: flushes macroquad's batcher so the target is drawn before the UI frame
    unsafe { get_internal_gl() }.flush();
    rt
}

fn polaroid_of(d: &Demo, i: usize, l: Lang) -> PolaroidView {
    let caps = mock::captions(l);
    let date = match l {
        Lang::En => format!("{} Nov 2039", 7 + i),
        _ => format!("{}. 11. 2039", 7 + i),
    };
    PolaroidView {
        photo: d.photos.get(i % 5).map(|rt| rt.texture.clone()),
        flip_y: true,
        caption: caps[i % caps.len()].0.clone(),
        date,
        tilt: [-2.5, 1.8, -1.0, 2.6, -1.8, 0.8][i % 6],
        develop: 1.0,
    }
}

fn atmosphere_page(ui: &mut Ui, d: &mut Demo) {
    let l = d.lang;
    let s = ui.screen();
    // three meters at different levels; the top one cycles to show the level-up flare
    for (i, v) in [18.0_f32, 52.0, 86.0].iter().enumerate() {
        let r = Rect::new(
            24.0 + i as f32 * 200.0,
            s.h - 60.0,
            ATMO_SIZE.x,
            ATMO_SIZE.y,
        );
        atmosphere_meter(ui, ("atmo", i), r, &mock::atmosphere(l, *v));
    }
    let cyc = ((d.frame / 90) % 5) as f32 * 20.0 + 10.0;
    let r = Rect::new(24.0, 24.0, ATMO_SIZE.x, ATMO_SIZE.y);
    atmosphere_meter(ui, "atmo_cycle", r, &mock::atmosphere(l, cyc));
    // the hover card, shown open for the screenshot
    let hr = Rect::new(
        s.w - ATMO_SIZE.x - 260.0,
        s.h - 120.0,
        ATMO_SIZE.x,
        ATMO_SIZE.y,
    );
    let resp = atmosphere_meter(ui, "atmo_hover", hr, &mock::atmosphere(l, 64.0));
    if !resp.hovered {
        show_atmo_popover(ui, hr, &mock::atmosphere(l, 64.0));
    }
}

/// The meter's hover card drawn statically (screenshots can't hover).
fn show_atmo_popover(ui: &mut Ui, at: Rect, v: &AtmosphereView) {
    let t = ui.theme.clone();
    let w = 236.0;
    let h = 38.0 + v.components.len() as f32 * 20.0 + 46.0;
    let pr = Rect::new(at.x, at.y - h - 8.0, w, h);
    shiftella_game::ui::draw_glass(&t, pr, &Glass::tooltip().radius(12.0));
    let inner = pr.inset(12.0);
    ui.fonts.draw_in(
        &v.title.to_uppercase(),
        inner.with_h(14.0),
        &t.micro(),
        Align::Left,
    );
    ui.fonts.draw_in(
        &format!("{:.0}", v.value),
        inner.with_h(14.0),
        &t.number().color(t.amber),
        Align::Right,
    );
    let mut y = inner.y + 24.0;
    for (i, (label, f)) in v.components.iter().enumerate() {
        let weak = i == 3;
        ui.fonts.draw_in(
            label,
            Rect::new(inner.x, y, 84.0, 16.0),
            &t.small().color(if weak { t.amber } else { t.text_dim }),
            Align::Left,
        );
        ui.progress(
            Rect::new(inner.x + 90.0, y + 6.0, inner.w - 90.0, 5.0),
            *f,
            if weak { t.amber_deep } else { t.indigo_400 },
        );
        y += 20.0;
    }
    if let Some(tip) = &v.tip {
        let tr = Rect::new(inner.x, y + 4.0, inner.w, 34.0);
        fill_rrect(tr, 8.0, with_a(t.amber, 0.09));
        draw_icon(Icon::Info, vec2(tr.x + 13.0, tr.center_y()), 13.0, t.amber);
        ui.fonts.draw_in(
            tip,
            tr.inset_ltrb(26.0, 0.0, 6.0, 0.0),
            &t.small().color(t.text),
            Align::Left,
        );
    }
}

fn rozpis(ui: &mut Ui, d: &mut Demo) {
    let l = d.lang;
    let v = mock::rozpis_view(l, &d.cells);
    let s = ui.screen();
    let size = rozpis_board_size(v.rows.len());
    let w = size.x.max(660.0).min(s.w - 56.0);
    let panel = Rect::new(
        ((s.w - w - 32.0) / 2.0).round(),
        40.0,
        w + 32.0,
        (size.y + 70.0).min(s.h - 110.0),
    );
    ui.glass(panel, &Glass::panel().solid());
    let sub = tr(
        l,
        [
            "Week 3 · draft",
            "Týden 3 · koncept",
            "Тиждень 3 · чернетка",
            "Неделя 3 · черновик",
        ],
    );
    let (body, _) = ui.panel_header(panel, &s_rozpis(l), Some("T"), Some(&sub));
    let r = body.inset_ltrb(16.0, 12.0, 16.0, 14.0);
    if let Some(a) = rozpis_board(
        ui,
        "rozpis",
        r,
        &v,
        &mock::rozpis_labels(l),
        &mut d.rozpis_st,
    ) {
        match a {
            RozpisAction::SetCell { row, day, shift } => d.cells[row][day] = shift,
            RozpisAction::Clear => {
                for r in d.cells.iter_mut().skip(1) {
                    *r = [ShiftCell::Off; 7];
                }
            }
            RozpisAction::AutoFill => d.cells = mock::rozpis_cells(),
            RozpisAction::FlashAutofill => d.rozpis_st.flash_until = ui.time() + 1.2,
            RozpisAction::Publish => {}
        }
        d.note(format!("rozpis: {a:?}"));
    }
    // loose pieces below: rings and chips at a glance
    let y = panel.bottom() + 16.0;
    let mut x = panel.x;
    for (i, wb) in [92.0, 72.0, 52.0, 30.0].iter().enumerate() {
        wellbeing_ring(ui, vec2(x + 20.0 + i as f32 * 48.0, y + 20.0), 36.0, *wb);
    }
    x += 210.0;
    for (i, row) in v.rows.iter().skip(1).take(3).enumerate() {
        for (j, p) in row.prefs.iter().enumerate() {
            if x + pref_chip_width(ui, p) > s.w - 12.0 {
                break;
            }
            let r = pref_chip(ui, ("loose", i, j), vec2(x, y + 10.0), p, 0.0);
            x = r.right() + 6.0;
        }
    }
}

fn s_rozpis(l: Lang) -> String {
    tr(l, ["Rozpis", "Rozpis", "Розклад", "Расписание"])
}

fn payroll(ui: &mut Ui, d: &mut Demo) {
    let l = d.lang;
    let v = mock::payroll_view(l, d.split, d.star, d.granted, d.sent);
    let s = ui.screen();
    let size = vec2(
        PAYROLL_SIZE.x.min(s.w - 24.0),
        PAYROLL_SIZE.y.min(s.h - 80.0),
    );
    let r = Rect::new(((s.w - size.x) / 2.0).round(), 40.0, size.x, size.y);
    if let Some(a) = payroll_screen(ui, "payroll", r, &v, &mock::payroll_labels(l)) {
        match a {
            PayrollAction::SetSplit(sp) => d.split = sp,
            PayrollAction::Star(st) => d.star = st,
            PayrollAction::GrantToil(_) => d.granted = true,
            PayrollAction::Send => d.sent = true,
            _ => {}
        }
        d.note(format!("payroll: {a:?}"));
    }
}

fn journal_page(ui: &mut Ui, d: &mut Demo) {
    let l = d.lang;
    let caps = mock::captions(l);
    let mut entries = Vec::new();
    for n in 1..=14u32 {
        let has = !matches!(n, 4 | 9) && n <= 12;
        entries.push(JournalEntry {
            night: n,
            title: caps[(n as usize - 1) % caps.len()].1.clone(),
            polaroid: has.then(|| polaroid_of(d, n as usize - 1, l)),
        });
    }
    let s = ui.screen();
    let size = vec2(
        JOURNAL_SIZE.x.min(s.w - 24.0),
        JOURNAL_SIZE.y.min(s.h - 60.0),
    );
    let r = Rect::new(
        ((s.w - size.x) / 2.0).round(),
        ((s.h - size.y) / 2.0 - 10.0).round(),
        size.x,
        size.y,
    );
    if std::env::var("SHIFTELLA_OPEN").is_ok() && d.frame < 3 {
        d.journal_st.open = Some(1);
    }
    if let Some(a) = journal(
        ui,
        "journal",
        r,
        &JournalView { entries },
        &mock::journal_labels(l),
        &mut d.journal_st,
    ) {
        d.note(format!("journal: {a:?}"));
    }
}

fn polaroid_page(ui: &mut Ui, d: &mut Demo) {
    let l = d.lang;
    let s = ui.screen();
    // the morning reveal: the best one develops, two alternatives below
    let dev = ((d.frame as f32 - 5.0) / 100.0).clamp(0.0, 1.0);
    let dev = std::env::var("SHIFTELLA_DEVELOP")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(dev);
    let mut main = polaroid_of(d, 4, l);
    main.develop = dev;
    main.tilt = -1.5;
    let t = ui.theme.clone();
    let title = tr(
        l,
        [
            "Tonight's polaroid",
            "Dnešní polaroid",
            "Полароїд цієї ночі",
            "Полароид этой ночи",
        ],
    );
    ui.fonts.draw_in(
        &title,
        Rect::new(0.0, 24.0, s.w, 24.0),
        &t.title(),
        Align::Center,
    );
    let mw = (s.h * 0.62 / handson::journal::polaroid_aspect()).min(360.0);
    polaroid(
        ui,
        "main",
        vec2(s.w / 2.0, 60.0 + polaroid_size(mw).y / 2.0),
        mw,
        &main,
    );
    for (k, i) in [1usize, 2].iter().enumerate() {
        let p = polaroid_of(d, *i, l);
        let c = vec2(
            s.w / 2.0 + (k as f32 - 0.5) * 230.0,
            s.h - 12.0 - polaroid_size(170.0).y / 2.0,
        );
        if polaroid(ui, ("alt", k), c, 170.0, &p).clicked {
            d.note(format!("keep polaroid {i}"));
        }
    }
    // small sizes for legibility checks
    for (k, w) in [120.0_f32, 150.0].iter().enumerate() {
        let p = polaroid_of(d, k, l);
        polaroid(
            ui,
            ("small", k),
            vec2(110.0 + k as f32 * 10.0, 140.0 + k as f32 * 170.0),
            *w,
            &p,
        );
    }
}

fn coach(ui: &mut Ui, d: &mut Demo) {
    let l = d.lang;
    let s = ui.screen();
    // a stand-in HUD: rail + atmosphere meter + a window lamp in the world
    let more = s_more(l);
    let tickets = mock::tickets(l);
    let top = vec2(s.w / 2.0, 70.0);
    let rail = ticket_rail_rect(top, tickets.len());
    let _ = ticket_rail(ui, "rail", top, &tickets, &mut d.rail, &more);
    let atmo = Rect::new(14.0, s.h - 50.0, ATMO_SIZE.x, ATMO_SIZE.y);
    atmosphere_meter(ui, "atmo", atmo, &mock::atmosphere(l, 64.0));
    let lamp = table_point(ui, 2) + vec2(0.0, -24.0);
    glow_circle(lamp, 30.0, with_a(ui.theme.amber, 0.6));
    let skip = tr(
        l,
        [
            "Skip tutorial",
            "Přeskočit návod",
            "Пропустити навчання",
            "Пропустить обучение",
        ],
    );
    let steps = [
        CoachMark {
            label: tr(
                l,
                [
                    "Lida's note · 1/3",
                    "Lidina poznámka · 1/3",
                    "Нотатка Ліди · 1/3",
                    "Заметка Лиды · 1/3",
                ],
            ),
            title: None,
            body: tr(
                l,
                [
                    "The light in the window first, Karolinko. Always first.",
                    "Světlo do okna nejdřív, Karolínko. Vždycky nejdřív.",
                    "Спершу світло у вікні, Каролінко. Завжди спершу.",
                    "Сначала свет в окне, Каролинка. Всегда сначала.",
                ],
            ),
            lida: true,
            target: Some(Rect::new(lamp.x - 18.0, lamp.y - 18.0, 36.0, 36.0)),
            next: tr(l, ["Got it", "Jasně", "Зрозуміло", "Понятно"]),
            skip: Some(skip.clone()),
            ring: true,
        },
        CoachMark {
            label: tr(
                l,
                [
                    "Tutorial · 2/3",
                    "Návod · 2/3",
                    "Навчання · 2/3",
                    "Обучение · 2/3",
                ],
            ),
            title: Some(tr(
                l,
                ["Who's waiting", "Kdo čeká", "Хто чекає", "Кто ждёт"],
            )),
            body: tr(
                l,
                [
                    "Orders nobody has taken wear a hollow ring. Those are mine.",
                    "Objednávky bez nikoho mají prázdný kroužek. Ty jsou moje.",
                    "Замовлення без власника мають порожнє кільце. Ці — мої.",
                    "У заказов без хозяина пустое кольцо. Эти — мои.",
                ],
            ),
            lida: false,
            target: Some(rail),
            next: tr(l, ["Next", "Dál", "Далі", "Дальше"]),
            skip: Some(skip),
            ring: true,
        },
        CoachMark {
            label: tr(
                l,
                [
                    "Tutorial · 3/3",
                    "Návod · 3/3",
                    "Навчання · 3/3",
                    "Обучение · 3/3",
                ],
            ),
            title: Some(tr(
                l,
                [
                    "It feels warmer",
                    "Je tu tepleji",
                    "Стало тепліше",
                    "Стало теплее",
                ],
            )),
            body: tr(
                l,
                [
                    "It feels warmer in here when the lamps are on.",
                    "Když svítí lampy, je tu hned tepleji.",
                    "Коли горять лампи, тут тепліше.",
                    "Когда горят лампы, здесь теплее.",
                ],
            ),
            lida: false,
            target: Some(atmo),
            next: tr(l, ["Done", "Hotovo", "Готово", "Готово"]),
            skip: None,
            ring: true,
        },
    ];
    let step = std::env::var("SHIFTELLA_STEP")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(d.coach_step)
        .min(2);
    match coach_mark(ui, ("coach", step), &steps[step], &[atmo, rail], true) {
        Some(CoachAction::Next) => d.coach_step = (step + 1) % 3,
        Some(CoachAction::Skip) => d.coach_step = 0,
        None => {}
    }
}
