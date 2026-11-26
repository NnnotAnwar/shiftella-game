//! The polaroid journal (design/handson.md §11.4): the sim keeps compact scene snapshots
//! (`SceneSnapData`); here they become render `SceneSnap`s, are re-drawn into photos (cached,
//! one new photo per frame) and shown in the morning summary (develop, pick one of three) and
//! in the scrapbook (J).

use std::collections::HashMap;

use macroquad::prelude::*;

use super::world_view::{self as wv, sim};
use super::{Ctx, GameScreen, Panel};
use crate::content::Content;
use crate::render::cat::CatPose;
use crate::render::{
    DecorTheme, Expression, Facing, LatteArt, SceneSnap, Season, SnapActor, SnapCup, SnapFloor,
    SnapProp, SnapRenderer, SnapTile,
};
use crate::settings::Locale;
use crate::ui::Ui;
use crate::ui::handson::{
    JOURNAL_SIZE, JournalEntry, JournalLabels, JournalState, JournalView, PolaroidView, journal,
};
use sim::{Highlight, SceneSnapData, SnapExpression, SnapTileData};

/// Photo size of a polaroid (4:3). Large enough that the enlarged journal view stays crisp.
pub const PHOTO_W: u32 = 960;
pub const PHOTO_H: u32 = 720;

/// Rendered photos by highlight key, and the renderer (created on the first frame that needs
/// it: tests and headless runs never touch the GPU).
#[derive(Default)]
pub struct Photos {
    renderer: Option<SnapRenderer>,
    /// Whole render targets are kept (a dropped target may hand its texture back to the GPU).
    cache: HashMap<u64, RenderTarget>,
    /// Keys wanted this frame (rendered next frame, one at a time).
    wanted: Vec<(u64, SceneSnap)>,
    pub journal: JournalState,
    /// The morning pick: which candidate is chosen, and how far it has developed (0..1).
    pub pick: usize,
    pub develop: f32,
    /// The night the morning pick belongs to.
    pub pick_night: u32,
}

/// A stable key for a highlight's photo.
pub fn photo_key(h: &Highlight) -> u64 {
    let mut k: u64 = 0xcbf2_9ce4_8422_2325;
    for v in [
        h.night as u64,
        h.minute as u64,
        h.score as u64,
        h.snap.seed as u64,
    ] {
        k = (k ^ v).wrapping_mul(0x0100_0000_01b3);
    }
    k ^ (h.kind as u64) << 56
}

fn floor(id: &str) -> SnapFloor {
    SnapFloor::from_id(id)
}

fn facing_dir(f: Facing) -> Vec2 {
    match f {
        Facing::Left => vec2(-1.0, 0.0),
        Facing::Right => vec2(1.0, 0.0),
        Facing::Up => vec2(0.0, -1.0),
        Facing::Down => vec2(0.0, 1.0),
    }
}

/// The sim's snapshot → the render's (looks by seed / preset, cups by recipe).
pub fn snap_of(content: &Content, d: &SceneSnapData) -> SceneSnap {
    let tiles = d
        .tiles
        .iter()
        .map(|t| match t {
            SnapTileData::Void => SnapTile::Void,
            SnapTileData::Floor(id) => SnapTile::Floor(floor(id)),
            SnapTileData::Wall => SnapTile::Wall,
            SnapTileData::Window => SnapTile::Window,
            SnapTileData::Door(id) => SnapTile::Door(floor(id)),
        })
        .collect();
    let props = d
        .props
        .iter()
        .map(|p| SnapProp {
            sprite: p.sprite.clone(),
            tile: p.tile,
            size: p.size,
            facing: wv::facing(p.facing),
            on: p.on,
            dirty: p.dirty,
        })
        .collect();
    let actors: Vec<SnapActor> = d
        .actors
        .iter()
        .map(|a| SnapActor {
            look: wv::look_of(&a.look),
            feet: vec2(a.feet.0, a.feet.1),
            facing: wv::facing(a.facing),
            pose: wv::pose(a.pose),
            expression: match a.expression {
                SnapExpression::Neutral => Expression::Neutral,
                SnapExpression::Smile => Expression::Smile,
                SnapExpression::Sad => Expression::Sad,
                SnapExpression::Surprised => Expression::Surprised,
                SnapExpression::Thinking => Expression::Thinking,
                SnapExpression::Laugh => Expression::Laugh,
                SnapExpression::Tired => Expression::Tired,
            },
            emote: a.emote.map(wv::emote),
        })
        .collect();
    // cups stand on the table in front of the seat (as in the live café)
    let cups = d
        .cups
        .iter()
        .map(|c| {
            let sitter = d
                .actors
                .iter()
                .find(|a| a.feet == c.pos && a.seat_tile.is_some());
            let pos = match sitter {
                Some(a) => {
                    let (sx, sy) = a.seat_tile.unwrap_or_default();
                    let f = wv::facing(a.facing);
                    let table = vec2(sx as f32, sy as f32) + facing_dir(f);
                    let side = if f == Facing::Left { 0.7 } else { 0.3 };
                    (table + vec2(side, 0.42)) * crate::map::TILE - vec2(0.0, 24.0)
                }
                None => vec2(c.pos.0, c.pos.1 - 40.0),
            };
            SnapCup {
                pos,
                glyph: super::hands::cup_of(content, c.recipe.as_deref().unwrap_or(&c.item)),
                art: c.latte.as_deref().and_then(LatteArt::from_id),
            }
        })
        .collect();
    let season = Season::from_id(&d.season).unwrap_or(Season::LateAutumn);
    SceneSnap {
        origin: d.origin,
        size: d.size,
        tiles,
        props,
        actors,
        cups,
        cat: d.cat.map(|c| (vec2(c.0, c.1 + 16.0), CatPose::Sleep)),
        // the instant camera's flash: a photo reads a little brighter than the hour
        night: d.night_progress * 0.5,
        rain: d.rain,
        season,
        theme: DecorTheme::from_id(&d.theme).unwrap_or_default(),
        garlands: if d.garlands { 1.0 } else { 0.0 },
        blackout: d.blackout,
        window_light: d.window_light,
        coziness: (d.coziness / 100.0).clamp(0.0, 1.0),
        train: d.train.then_some(0.45),
        time: 3.0 + (d.seed % 11) as f32,
        seed: d.seed,
        focus: Some(vec2(d.focus.0, d.focus.1 - 24.0)),
        // a little closer than the sim's window: people read as people on a polaroid
        zoom: if d.zoom > 0.0 { d.zoom * 1.55 } else { 1.55 },
    }
}

/// Values in sim args may be i18n keys (`regular.mirek.name`), `@key`s or money.
pub fn sys_args(loc: &Locale, args: &[(String, String)]) -> Vec<(String, String)> {
    args.iter()
        .map(|(k, v)| {
            let weekday = (k == "weekday")
                .then(|| crate::content::Weekday::ALL.iter().find(|w| w.id() == v))
                .flatten();
            let v = if let Some(w) = weekday {
                loc.tr_owned(&w.long_key())
            } else if let Some(key) = v.strip_prefix('@') {
                loc.tr_owned(key)
            } else if let Some(n) = v.strip_prefix("money:") {
                crate::i18n::format_money(loc.lang, n.parse().unwrap_or(0))
            } else if v.contains('.') && loc.has(v) {
                loc.tr_owned(v)
            } else {
                v.clone()
            };
            (k.clone(), v)
        })
        .collect()
}

/// The caption of a highlight in the player's language.
pub fn caption(loc: &Locale, h: &Highlight) -> String {
    wv::tr_args(loc, &h.caption_key, &sys_args(loc, &h.args))
}

/// "7 Nov 2039" for a story date (none in sandbox).
pub fn date_text(loc: &Locale, date: Option<(i32, u32, u32)>) -> String {
    match date {
        Some((y, m, d)) => loc.trf(
            "journal.date",
            &[
                ("day", &d.to_string()),
                ("month", loc.tr(&format!("journal.month.{m}"))),
                ("year", &y.to_string()),
            ],
        ),
        None => String::new(),
    }
}

/// The story date of a night (the story calendar; none in sandbox).
pub fn night_date(content: &Content, w: &sim::World, night: u32) -> Option<(i32, u32, u32)> {
    if w.mode() != sim::Mode::Story {
        return None;
    }
    content.story.calendar.as_ref().map(|c| {
        let d = c.date_of_night(night);
        (d.year, d.month, d.day)
    })
}

impl Photos {
    /// The photo of `h` if it's ready; otherwise it's queued for the next frame.
    pub fn get(&mut self, content: &Content, h: &Highlight) -> Option<Texture2D> {
        let k = photo_key(h);
        if let Some(t) = self.cache.get(&k) {
            return Some(t.texture.clone());
        }
        if !self.wanted.iter().any(|w| w.0 == k) {
            self.wanted.push((k, snap_of(content, &h.snap)));
        }
        None
    }

    /// Renders one wanted photo (call before the UI camera is set).
    pub fn render_pending(&mut self) {
        if self.wanted.is_empty() {
            return;
        }
        let (k, snap) = self.wanted.remove(0);
        let r = self.renderer.get_or_insert_with(SnapRenderer::new);
        let rt = r.render(&snap, PHOTO_W, PHOTO_H);
        self.cache.insert(k, rt);
        // a long session: keep the cache bounded (journal pages re-render on demand)
        if self.cache.len() > 48 {
            let drop: Vec<u64> = self.cache.keys().take(8).copied().collect();
            for d in drop {
                self.cache.remove(&d);
            }
        }
    }
}

/// The polaroid view of a highlight (photo when ready).
pub fn polaroid_view(
    g: &mut GameScreen,
    content: &Content,
    loc: &Locale,
    h: &Highlight,
    date: Option<(i32, u32, u32)>,
    develop: f32,
) -> PolaroidView {
    let photo = g.photos.get(content, h);
    PolaroidView {
        develop: if photo.is_some() { develop } else { 0.0 },
        photo,
        flip_y: true,
        caption: caption(loc, h),
        date: date_text(loc, date),
        tilt: crate::render::polaroid_tilt(h.snap.seed).to_degrees(),
    }
}

/// The scrapbook (J): 30 story slots (dashed until kept), endless pages in sandbox.
pub fn journal_panel(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    let content = cx.content;
    let loc = cx.loc;
    let kept: Vec<sim::JournalEntry> = g.world.journal().to_vec();
    let story = g.world.mode() == sim::Mode::Story;
    let nights: Vec<u32> = if story {
        (1..=content.story.nights_total).collect()
    } else {
        kept.iter().map(|e| e.night).collect()
    };
    // only the page on screen needs photos
    let page = g.photos.journal.page;
    let mut entries = Vec::with_capacity(nights.len());
    for (i, n) in nights.iter().enumerate() {
        let title = if story {
            content
                .story
                .night(*n)
                .map(|d| loc.tr_owned(&d.title_key()))
                .unwrap_or_default()
        } else {
            String::new()
        };
        let on_page = i / 6 == page || g.photos.journal.open == Some(i);
        let pol = kept
            .iter()
            .find(|e| e.night == *n)
            .filter(|_| on_page)
            .map(|e| polaroid_view(g, content, &loc, &e.highlight, e.date, 1.0))
            .or_else(|| {
                // off-page kept entries still count as filled (no photo needed)
                kept.iter().find(|e| e.night == *n).map(|e| PolaroidView {
                    photo: None,
                    flip_y: true,
                    caption: caption(&loc, &e.highlight),
                    date: date_text(&loc, e.date),
                    tilt: 0.0,
                    develop: 1.0,
                })
            });
        entries.push(JournalEntry {
            night: *n,
            title,
            polaroid: pol,
        });
    }
    let labels = JournalLabels {
        title: loc.tr_owned("journal.title"),
        page: loc.tr_owned("ui.journal.page"),
        night: loc.tr("journal.night").replace("{night}", "{n}"),
        close: loc.tr_owned("ui.common.close"),
        prev: loc.tr_owned("ui.journal.prev"),
        next: loc.tr_owned("ui.journal.next"),
    };
    let s = ui.screen();
    let size = vec2(
        JOURNAL_SIZE.x.min(s.w - 24.0),
        JOURNAL_SIZE.y.min(s.h - 150.0),
    );
    let r = Rect::new(
        (s.w - size.x) / 2.0,
        (84.0 + (s.h - 60.0 - 84.0 - size.y) / 2.0).max(84.0),
        size.x,
        size.y,
    );
    g.anchors.push(("panel", r));
    ui.region(r);
    let view = JournalView { entries };
    if journal(ui, "journal", r, &view, &labels, &mut g.photos.journal).is_some() {
        g.panel = None;
    }
}

/// The morning pick in the summary: the best candidate develops, the others sit beside it.
/// Returns the rect it used.
pub fn morning_pick(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx, area: Rect) -> Option<Rect> {
    let content = cx.content;
    let loc = cx.loc;
    let (night, cands) = g.world.pending_polaroids()?;
    let cands: Vec<Highlight> = cands.to_vec();
    if cands.is_empty() {
        return None;
    }
    if g.photos.pick_night != night {
        g.photos.pick_night = night;
        g.photos.pick = 0;
        g.photos.develop = 0.0;
    }
    let pick = g.photos.pick.min(cands.len() - 1);
    let date = night_date(content, &g.world, night);
    let t = ui.theme.clone();
    let big_w = (area.w - 24.0).min(250.0);
    let size = crate::ui::handson::polaroid_size(big_w);
    let c = vec2(area.center().x, area.y + 12.0 + size.y / 2.0);
    let big = polaroid_view(g, content, &loc, &cands[pick], date, g.photos.develop);
    let resp = crate::ui::handson::polaroid(ui, ("morning_pol", night, pick), c, big_w, &big);
    // shaking it (dragging over the print) develops it faster
    let dt = ui.dt();
    let shake = if resp.hovered && ui.mouse_down() {
        2.5
    } else {
        1.0
    };
    if big.photo.is_some() {
        g.photos.develop = (g.photos.develop + dt / 2.0 * shake).min(1.0);
    }
    g.anchors.push((
        "summary.polaroid",
        Rect::new(c.x - size.x / 2.0, c.y - size.y / 2.0, size.x, size.y),
    ));
    g.hands.tut.bump("polaroid_shown");
    let mut y = c.y + size.y / 2.0 + 10.0;
    if g.photos.develop < 0.98 {
        ui.fonts.draw_in(
            loc.tr("journal.shake"),
            Rect::new(area.x, y, area.w, 16.0),
            &t.small().color(t.text_dim),
            crate::ui::Align::Center,
        );
    }
    y += 22.0;
    // the alternatives: one click keeps that one instead
    let others: Vec<usize> = (0..cands.len()).filter(|i| *i != pick).collect();
    if !others.is_empty() {
        let sw = ((area.w - 24.0) / others.len().max(2) as f32 - 16.0).min(104.0);
        let ssize = crate::ui::handson::polaroid_size(sw);
        let total = others.len() as f32 * (sw + 16.0) - 16.0;
        let mut x = area.center().x - total / 2.0 + sw / 2.0;
        for i in others {
            let v = polaroid_view(g, content, &loc, &cands[i], date, 1.0);
            let r = crate::ui::handson::polaroid(
                ui,
                ("morning_alt", night, i),
                vec2(x, y + ssize.y / 2.0),
                sw,
                &v,
            );
            ui.tooltip(&r, loc.tr("journal.keep"));
            if r.clicked {
                g.photos.pick = i;
                g.photos.develop = 0.75;
            }
            x += sw + 16.0;
        }
        y += ssize.y + 8.0;
    }
    Some(Rect::new(area.x, area.y, area.w, y - area.y))
}

/// Keeps the chosen morning polaroid (the summary closes).
pub fn keep_morning(g: &mut GameScreen, content: &std::rc::Rc<Content>) {
    if g.world.pending_polaroids().is_some() {
        let i = g.photos.pick;
        let _ = g.command(content, sim::Command::PickPolaroid { index: i });
        g.photos.pick = 0;
        g.hands.tut.bump("polaroid_kept");
    }
}

/// Renders the photos the last frame asked for (before the UI camera is set).
pub fn prepare(g: &mut GameScreen) {
    if g.panel == Some(Panel::Journal) || g.summary.is_some() {
        g.photos.render_pending();
    }
}
