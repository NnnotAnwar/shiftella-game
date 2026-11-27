//! Hands-on rituals over the piece (the owner: "more interesting than just clicking a tick").
//! When Karolina herself heads for an opening / closing checklist item (or an appliance chore
//! with a gesture) and the player is watching at ×1, a small glass card opens over the piece
//! with its gesture (`minigame::ritual`): pull the cord, hold the needle in the green, count
//! the float… The player may play it, or not:
//!
//! * the first touch sends `Command::Ritual { Start }` — she waits for it at the piece (a
//!   while at most);
//! * the finished gesture sends `Done { perfect }` — the item completes at once; a perfect one
//!   floats a small "Perfect" and counts towards a perfect opening / closing;
//! * untouched, it simply closes when she is done (same ticks as ever); Esc / right-click on
//!   the card lets her finish it herself (`LetGo`).
//!
//! It never opens at ×2 / ×3, under the autoplayer or screenshot automation (unless a scene
//! stages it), over another card or panel, or for a piece off screen; pausing freezes it. The
//! first time each gesture shows, a one-line hint sits under it (remembered in
//! `Settings::ritual_hints`).

use std::collections::BTreeSet;

use macroquad::prelude::*;

use super::hud::world_to_ui;
use super::world_view::{self as wv, sim};
use super::{Ctx, GameScreen};
use crate::content::Content;
use crate::content::handson::{RitualGesture, SlideLook, StrokeLook};
use crate::map::TILE;
use crate::minigame::MgInput;
use crate::minigame::ritual::{Gesture, GestureStatus, STAGE, seed_for};
use crate::settings::Locale;
use crate::ui::draw::{fill_rrect, line};
use crate::ui::{Align, Glass, Icon, RectExt, Ui, draw_icon, ease_out};
use sim::{Command, PlayerAct, RitualStep};

/// Seconds the card stays after a finished gesture (the little done beat).
const DONE_HOLD: f32 = 0.55;
/// Seconds a hint must have been on screen to count as seen (or the gesture was touched).
const HINT_SEEN_AFTER: f32 = 1.5;

/// The card on screen.
#[derive(Debug, Clone)]
pub struct RitualOpen {
    /// The gesture key (`World::ritual_key_now`).
    pub key: String,
    /// The label (`app.checklist.<id>` / `equip.act.<id>`).
    pub label: String,
    pub gesture: Gesture,
    pub started_sent: bool,
    pub done_sent: bool,
    /// Real seconds on screen (not counting pauses).
    pub shown: f32,
    /// The one-line hint shows (the first time this gesture kind plays).
    pub hint: bool,
    /// The handover note's lines on offer: (suggestion id, text).
    pub lines: Vec<(Option<String>, String)>,
    /// The piece's top (world px).
    pub anchor: Vec2,
}

/// The rituals' screen state (`GameScreen::rituals`).
#[derive(Debug, Clone, Default)]
pub struct RitualUi {
    pub open: Option<RitualOpen>,
    /// Let go (Esc / right-click): this key stays closed until her act changes.
    pub dismissed: Option<String>,
    /// Hint kinds seen this session, not yet in the settings.
    pub seen: Vec<String>,
    /// Screenshot scenes: open even under automation.
    pub staged: bool,
    /// The card last frame (UI px).
    pub card: Option<Rect>,
}

impl RitualUi {
    /// The card holds the keyboard (Space, the arrows, Esc).
    pub fn wants_keys(&self) -> bool {
        self.open
            .as_ref()
            .is_some_and(|o| o.gesture.finished().is_none())
    }
}

/// The piece an act of hers happens at (world px, its top centre).
fn anchor_world(g: &GameScreen, content: &Content, act: &PlayerAct) -> Option<Vec2> {
    let w = &g.world;
    let item = |id: &str| content.app.checklists.iter().position(|c| c.id == id);
    let uid = match act {
        PlayerAct::Checklist { id } => super::hands_view::ritual_piece(w, content, item(id)?)?,
        PlayerAct::PlayRecord { .. } => {
            super::hands_view::ritual_piece(w, content, item("record_on")?)?
        }
        PlayerAct::Use { uid, .. } => *uid,
        _ => return None,
    };
    let p = w.furniture_by_uid(uid)?;
    let (sw, _) = content
        .furniture_def(&p.def)
        .map(|d| sim::grid::rotated_size(d, p.rot))
        .unwrap_or((1, 1));
    Some(vec2(p.x as f32 + sw as f32 * 0.5, p.y as f32 - 0.45) * TILE)
}

/// Is the café's camera showing this world point (with a margin)?
fn on_screen(g: &GameScreen, p: Vec2) -> bool {
    let s = (p - g.cam.center) * g.cam.zoom + vec2(screen_width(), screen_height()) / 2.0;
    s.x > 40.0 && s.y > 60.0 && s.x < screen_width() - 40.0 && s.y < screen_height() - 40.0
}

/// The gesture that may show now: (key, the piece's top) — or `None` (she does it herself).
pub fn eligible(g: &GameScreen, content: &Content) -> Option<(String, Vec2)> {
    let w = &g.world;
    if w.phase() != sim::Phase::Night
        || (g.automation && !g.rituals.staged)
        || g.hands.autoplay.is_some()
        || g.speed > 1
        || g.player.is_playing()
        || g.summary.is_some()
        || g.panel.is_some()
        || g.hands.overlay()
        || g.hands.records_open
        || g.sig.open
        || g.walk.is_open()
        || g.photo.is_open()
        || g.stories.talk_open()
        || g.stories.card.is_some()
        || !g.reading.is_empty()
    {
        return None;
    }
    let cur = w.player().current.as_ref()?;
    let key = w.ritual_key_now(content)?;
    sim::ritual_gesture(content, &key)?;
    let at = anchor_world(g, content, &cur.act)?;
    (g.rituals.staged || on_screen(g, at)).then_some((key, at))
}

/// The label of a gesture key.
fn label_of(loc: &Locale, key: &str) -> String {
    match key.strip_prefix("equip.") {
        Some(a) => loc.tr_owned(&format!("equip.act.{a}")),
        None => loc.tr_owned(&format!("app.checklist.{key}")),
    }
}

/// The handover note's lines: tonight's suggestions, or a few calm lines of its own.
fn handover_lines(
    g: &GameScreen,
    content: &Content,
    loc: &Locale,
    n: usize,
) -> Vec<(Option<String>, String)> {
    let mut out: Vec<(Option<String>, String)> = g
        .world
        .handover_suggestions(content)
        .iter()
        .take(n)
        .map(|s| {
            let text = wv::tr_args(
                loc,
                &format!("app.handover.option.{}", s.id),
                &super::journal_ui::sys_args(loc, &s.args),
            );
            (Some(s.id.clone()), text)
        })
        .collect();
    for k in ["quiet", "thanks", "lamp"] {
        if out.len() >= n.max(2) {
            break;
        }
        out.push((None, loc.tr_owned(&format!("ritual.line.{k}"))));
    }
    out
}

/// Opens / closes the card for what she is doing now. `hints` = the gesture kinds whose hint
/// was seen (the settings).
pub fn sync(
    g: &mut GameScreen,
    content: &Content,
    loc: &Locale,
    hints: &BTreeSet<String>,
    relaxed: bool,
) {
    let want = eligible(g, content);
    if g.rituals.dismissed.as_deref() != want.as_ref().map(|w| w.0.as_str()) {
        g.rituals.dismissed = None;
    }
    // a finished gesture keeps its card a moment (she is already done)
    if let Some(o) = g.rituals.open.as_ref()
        && o.gesture.finished().is_some()
    {
        if o.gesture.since_done >= DONE_HOLD {
            close(g, content, false);
        }
        return;
    }
    match (&want, g.rituals.open.as_ref()) {
        (Some((key, at)), Some(o)) if &o.key == key => {
            let at = *at;
            if let Some(o) = g.rituals.open.as_mut() {
                o.anchor = at;
            }
        }
        (_, Some(_)) => {
            // she finished by herself, or the moment passed (×2, a card opened…)
            // (paused: the card simply waits)
            if g.frozen() && want.is_none() && g.world.ritual_key_now(content).is_some() {
                return;
            }
            close(g, content, true);
            if let Some((key, at)) = want
                && g.rituals.dismissed.as_deref() != Some(key.as_str())
            {
                open(g, content, loc, hints, relaxed, key, at);
            }
        }
        (Some((key, at)), None) => {
            // a fresh card only at ×1, never while paused
            if g.speed == 1 && !g.frozen() && g.rituals.dismissed.as_deref() != Some(key.as_str()) {
                let (key, at) = (key.clone(), *at);
                open(g, content, loc, hints, relaxed, key, at);
            }
        }
        (None, None) => {}
    }
}

fn open(
    g: &mut GameScreen,
    content: &Content,
    loc: &Locale,
    hints: &BTreeSet<String>,
    relaxed: bool,
    key: String,
    at: Vec2,
) {
    let Some(kind) = sim::ritual_gesture(content, &key).cloned() else {
        return;
    };
    let lines = match &kind {
        RitualGesture::Lines { count } => handover_lines(g, content, loc, *count as usize),
        _ => Vec::new(),
    };
    let seed = seed_for(g.world.seed(), g.world.night(), &key);
    let gesture = Gesture::new(&kind, seed, relaxed, lines.len());
    let hint = !hints.contains(kind.id()) && !g.rituals.seen.iter().any(|s| s == kind.id());
    g.rituals.open = Some(RitualOpen {
        label: label_of(loc, &key),
        key,
        gesture,
        started_sent: false,
        done_sent: false,
        shown: 0.0,
        hint,
        lines,
        anchor: at,
    });
}

/// Closes the card; `let_go` = a started gesture is handed back to her.
fn close(g: &mut GameScreen, content: &Content, let_go: bool) {
    let Some(o) = g.rituals.open.take() else {
        return;
    };
    if o.hint && (o.shown >= HINT_SEEN_AFTER || o.gesture.started()) {
        let id = o.gesture.kind.id().to_string();
        if !g.rituals.seen.contains(&id) {
            g.rituals.seen.push(id);
        }
    }
    if let_go && o.started_sent && !o.done_sent {
        let _ = g.world.apply(
            content,
            Command::Ritual {
                key: o.key,
                step: RitualStep::LetGo,
            },
        );
    }
}

/// Esc / right-click: she finishes it herself (and the card stays shut for this act).
pub fn let_go(g: &mut GameScreen, content: &Content) {
    if let Some(key) = g.rituals.open.as_ref().map(|o| o.key.clone()) {
        g.rituals.dismissed = Some(key);
        close(g, content, true);
    }
}

/// One step of the open gesture with this input (real seconds `dt`); sends the commands.
/// Returns the sounds to play.
pub fn step(
    g: &mut GameScreen,
    content: &Content,
    input: &MgInput,
    dt: f32,
) -> Vec<(crate::audio::Sfx, f32)> {
    let mut sounds = Vec::new();
    // pausing pauses the gesture too
    if g.frozen() {
        return sounds;
    }
    let Some(o) = g.rituals.open.as_mut() else {
        return sounds;
    };
    o.shown += dt;
    let before = o.gesture.started();
    let status = o.gesture.step(input, dt);
    let start_now = !o.started_sent && o.gesture.started();
    let key = o.key.clone();
    if start_now {
        o.started_sent = true;
        if !before {
            sounds.push((crate::audio::Sfx::UiClick, 0.25));
        }
        if g.world
            .apply(
                content,
                Command::Ritual {
                    key: key.clone(),
                    step: RitualStep::Start,
                },
            )
            .is_err()
        {
            g.rituals.open = None;
            return sounds;
        }
    }
    let Some(o) = g.rituals.open.as_mut() else {
        return sounds;
    };
    if let GestureStatus::Done { perfect } = status
        && !o.done_sent
    {
        o.done_sent = true;
        let anchor = o.anchor;
        let kind = o.gesture.kind.clone();
        // the handover note: the line she wrote goes first (and the next best with it)
        let picks: Vec<String> = match o.gesture.chosen_line() {
            Some(i) => {
                let mut p: Vec<String> = o
                    .lines
                    .get(i)
                    .and_then(|l| l.0.clone())
                    .into_iter()
                    .collect();
                if let Some(other) = o
                    .lines
                    .iter()
                    .enumerate()
                    .find(|(j, l)| *j != i && l.0.is_some())
                    .and_then(|(_, l)| l.0.clone())
                {
                    p.push(other);
                }
                p
            }
            None => Vec::new(),
        };
        if !picks.is_empty() {
            let _ = g.world.apply(content, Command::PickHandover { picks });
        }
        let _ = g.world.apply(
            content,
            Command::Ritual {
                key,
                step: RitualStep::Done { perfect },
            },
        );
        sounds.extend(done_sounds(&kind));
        if perfect {
            g.hands.floats.push(super::hands::Float {
                at: anchor - vec2(0.0, 10.0),
                text: "ritual.perfect".into(),
                born: g.time,
                args: Vec::new(),
            });
        }
    }
    sounds
}

/// The sound of a finished gesture (the café's own CC0 sounds).
fn done_sounds(kind: &RitualGesture) -> Vec<(crate::audio::Sfx, f32)> {
    use crate::audio::Sfx;
    let s = match kind {
        RitualGesture::Cord => (Sfx::LampSwitch, 0.8),
        RitualGesture::Needle { .. } => (Sfx::MachineHiss, 0.6),
        RitualGesture::Count { .. } | RitualGesture::Sort { .. } => (Sfx::TillCoins, 0.6),
        RitualGesture::Strokes { look, .. } => match look {
            StrokeLook::Chalk => (Sfx::Chalk, 0.7),
            StrokeLook::Brush => (Sfx::GroupClean, 0.6),
            StrokeLook::Sponge => (Sfx::CupSaucer, 0.5),
            StrokeLook::Cloth => (Sfx::UiClick, 0.3),
        },
        RitualGesture::Shelf { .. } => (Sfx::FridgeDoor, 0.5),
        RitualGesture::Drop { .. } => (Sfx::UiClick, 0.35),
        RitualGesture::Lines { .. } => (Sfx::PageFlip, 0.6),
        RitualGesture::Slide { look } => match look {
            SlideLook::Bottle => (Sfx::BottleClink, 0.6),
            SlideLook::Tray => (Sfx::OvenDoor, 0.6),
            SlideLook::Bag => (Sfx::TrashBag, 0.6),
        },
        RitualGesture::Stir => (Sfx::PotStir, 0.7),
    };
    vec![s]
}

/// The card's layout (UI px): its rect, the stage's top-left, the hint's lines.
struct Layout {
    card: Rect,
    origin: Vec2,
    /// The stage's width (wider for long handover lines).
    stage_w: f32,
    hint: Vec<String>,
}

/// Over the piece at `anchor_ui` (under it when the HUD band is above), wide enough for its
/// title and hint (the hint wraps to two lines at most), inside the screen.
fn layout(ui: &Ui, anchor_ui: Vec2, title: &str, hint: Option<&str>, lines: &[String]) -> Layout {
    let t = ui.theme.clone();
    let s = ui.screen();
    let ls = t.small();
    let stage_w = lines
        .iter()
        .map(|l| ui.fonts.measure(l, &ls) + 28.0)
        .fold(STAGE.x, f32::max)
        .min(300.0);
    let tw = ui
        .fonts
        .measure(title, &t.small().weight(crate::ui::Weight::SemiBold));
    let hs = t.small();
    let hw = hint.map_or(0.0, |h| ui.fonts.measure(h, &hs));
    let w = (tw + 72.0)
        .max(stage_w + 16.0)
        .max(hw.min(300.0) + 30.0)
        .min(316.0);
    let hint: Vec<String> = hint
        .map(|h| {
            ui.fonts
                .wrap(h, w - 30.0, &hs)
                .into_iter()
                .take(2)
                .collect()
        })
        .unwrap_or_default();
    let h = 28.0
        + STAGE.y
        + 8.0
        + hint.len() as f32 * hs.line_h()
        + if hint.is_empty() { 0.0 } else { 4.0 };
    let x = (anchor_ui.x - w / 2.0).clamp(8.0, (s.w - w - 8.0).max(8.0));
    let mut y = anchor_ui.y - h - 22.0;
    // the clock, the doors chip and the money card own the top band
    if y < 128.0 {
        y = anchor_ui.y + 52.0;
    }
    let y = y.clamp(56.0, (s.h - h - 64.0).max(56.0));
    let card = Rect::new(x, y, w, h);
    Layout {
        card,
        origin: vec2(card.x + (w - stage_w) / 2.0, card.y + 28.0),
        stage_w,
        hint,
    }
}

/// The UI pass: open / close, the input, the card. Called each frame at night.
pub fn frame(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx, dt: f32) {
    let content = cx.content;
    let loc = cx.loc;
    sync(
        g,
        content,
        &loc,
        &cx.settings.ritual_hints,
        cx.settings.relaxed_minigames,
    );
    // newly seen hints go to the settings (once each)
    let fresh: Vec<String> = g
        .rituals
        .seen
        .iter()
        .filter(|s| !cx.settings.ritual_hints.contains(*s))
        .cloned()
        .collect();
    if !fresh.is_empty() {
        cx.settings.ritual_hints.extend(fresh);
        let _ = cx.settings.save();
    }
    let Some(o) = g.rituals.open.as_ref() else {
        g.rituals.card = None;
        return;
    };
    // a hint on screen long enough counts as read right away
    if o.hint && o.shown >= HINT_SEEN_AFTER {
        let id = o.gesture.kind.id().to_string();
        if !cx.settings.ritual_hints.contains(&id) {
            cx.settings.ritual_hints.insert(id);
            let _ = cx.settings.save();
        }
    }
    let anchor_ui = world_to_ui(g, ui, o.anchor);
    let hint_text = o
        .hint
        .then(|| loc.tr_owned(&format!("ritual.hint.{}", o.gesture.kind.id())));
    let texts: Vec<String> = o.lines.iter().map(|l| l.1.clone()).collect();
    let lay = layout(ui, anchor_ui, &o.label, hint_text.as_deref(), &texts);
    let (card, origin) = (lay.card, lay.origin);
    g.rituals.card = Some(card);
    if let Some(o) = g.rituals.open.as_mut() {
        o.gesture.width = lay.stage_w;
    }
    let Some(o) = g.rituals.open.as_ref() else {
        return;
    };
    let done = o.gesture.finished().is_some();
    // input: the mouse inside the stage, Space and the arrows while it plays
    let resp = ui.interact(ui.id("ritual_card"), card);
    let mut input = MgInput::default();
    if !done && !g.frozen() {
        let typing = ui.wants_keyboard();
        let kd = |k: KeyCode| !typing && is_key_down(k);
        let stage = Rect::new(origin.x, origin.y, lay.stage_w, STAGE.y);
        let m = ui.mouse();
        input.pointer = Some(m - origin);
        input.pressed = ui.mouse_pressed() && stage.contains(m);
        input.down = ui.mouse_down();
        input.released = ui.mouse_released();
        input.hold = kd(KeyCode::Space);
        input.hold_pressed = ui.take_key(KeyCode::Space);
        input.left = ui.take_key(KeyCode::Left);
        input.right = ui.take_key(KeyCode::Right);
        input.up = ui.take_key(KeyCode::Up);
        input.down_tap = ui.take_key(KeyCode::Down);
        input.axis = vec2(
            kd(KeyCode::Right) as i32 as f32 - kd(KeyCode::Left) as i32 as f32,
            kd(KeyCode::Down) as i32 as f32 - kd(KeyCode::Up) as i32 as f32,
        );
        if ui.esc() || resp.right_clicked {
            let_go(g, content);
            g.rituals.card = None;
            return;
        }
    }
    // (a staged screenshot keeps the gesture as the script left it)
    if !(g.automation && g.rituals.staged) {
        for (sfx, vol) in step(g, content, &input, dt) {
            cx.audio.play_sfx_vol(sfx, vol);
        }
    }
    draw(g, ui, &loc, &lay);
}

/// The card: the item's name, the gesture's stage, a thin progress line, the first-time hint.
fn draw(g: &GameScreen, ui: &mut Ui, loc: &Locale, lay: &Layout) {
    let (card, origin) = (lay.card, lay.origin);
    let Some(o) = g.rituals.open.as_ref() else {
        return;
    };
    let t = ui.theme.clone();
    let paused = g.frozen();
    let fade_in = ease_out((o.shown / 0.18).min(1.0));
    let fade_out = if o.gesture.finished().is_some() {
        1.0 - (o.gesture.since_done / DONE_HOLD).clamp(0.0, 1.0).powi(2)
    } else {
        1.0
    };
    let a = fade_in * fade_out * if paused { 0.7 } else { 1.0 };
    crate::ui::draw::push_alpha(a);
    ui.glass(card, &Glass::hud().radius(10.0));
    let icon = match o.gesture.kind {
        RitualGesture::Lines { .. } => Icon::Note,
        RitualGesture::Count { .. } | RitualGesture::Sort { .. } => Icon::CashRegister,
        RitualGesture::Drop { .. } => Icon::Vinyl,
        _ => Icon::ClipboardCheck,
    };
    draw_icon(icon, vec2(card.x + 16.0, card.y + 14.0), 12.0, t.indigo_300);
    let title = t.small().weight(crate::ui::Weight::SemiBold).color(t.text);
    ui.fonts.draw_in(
        &o.label,
        Rect::new(card.x + 28.0, card.y + 4.0, card.w - 64.0, 20.0),
        &title,
        Align::Left,
    );
    // Esc = she finishes it herself
    let esc = Rect::new(card.right() - 34.0, card.y + 6.0, 26.0, 16.0);
    fill_rrect(esc, 4.0, Color::new(1.0, 1.0, 1.0, 0.07));
    ui.fonts
        .draw_in("Esc", esc, &t.micro().color(t.text_dim), Align::Center);
    let esc_resp = ui.interact(ui.id("ritual_esc"), esc);
    ui.tooltip(&esc_resp, loc.tr("ritual.let_go"));
    o.gesture.draw(
        ui,
        origin,
        &o.lines.iter().map(|l| l.1.clone()).collect::<Vec<_>>(),
        g.time,
    );
    // feedback words under the stage's top edge
    let note = if o.gesture.count_over() {
        Some(loc.tr("ritual.too_much"))
    } else if o.gesture.drop_missed() {
        Some(loc.tr("ritual.not_yet"))
    } else {
        None
    };
    if let Some(n) = note {
        ui.fonts.draw_in(
            n,
            Rect::new(origin.x + 6.0, origin.y + 2.0, lay.stage_w - 12.0, 14.0),
            &t.micro().color(t.amber),
            Align::Left,
        );
    }
    let k = o.gesture.progress();
    let py = origin.y + STAGE.y + 3.0;
    let sw = lay.stage_w;
    line(
        vec2(origin.x + 4.0, py),
        vec2(origin.x + sw - 4.0, py),
        2.0,
        Color::new(1.0, 1.0, 1.0, 0.08),
    );
    if k > 0.0 {
        line(
            vec2(origin.x + 4.0, py),
            vec2(origin.x + 4.0 + (sw - 8.0) * k, py),
            2.0,
            t.indigo_300,
        );
    }
    if !lay.hint.is_empty() {
        let hs = t.small().color(t.text_dim);
        let lh = hs.line_h();
        draw_icon(
            Icon::Info,
            vec2(card.x + 15.0, py + 4.0 + lh / 2.0),
            10.0,
            t.indigo_300,
        );
        for (i, l) in lay.hint.iter().enumerate() {
            ui.fonts.draw_in(
                l,
                Rect::new(card.x + 24.0, py + 4.0 + i as f32 * lh, card.w - 30.0, lh),
                &hs,
                Align::Left,
            );
        }
    }
    crate::ui::draw::pop_alpha();
}

/// Headless frame for tests and staged screenshots: sync + one step with this input.
pub fn headless(
    g: &mut GameScreen,
    content: &Content,
    loc: &Locale,
    hints: &BTreeSet<String>,
    input: &MgInput,
    dt: f32,
) {
    sync(g, content, loc, hints, false);
    let _ = step(g, content, input, dt);
}

impl GameScreen {
    /// Screenshots (`SHIFTELLA_SCENE=hands_ux`, `SHIFTELLA_HUX=gesture`,
    /// `SHIFTELLA_GESTURE=<key>`, `SHIFTELLA_GESTURE_T=<seconds>`): Karolina at the piece of
    /// that checklist item / chore with its gesture open, played by the script a little.
    pub fn stage_ritual(&mut self, content: &Content, loc: &Locale, key: &str, secs: f32) {
        let _ = self.world.apply(content, Command::PlayerClearQueue);
        let _ = self.world.apply(content, Command::PlayerCancel);
        self.rituals.staged = true;
        // (nothing else open over the café: a moment's or an order's card from the night)
        self.hands.order = None;
        self.hands.moment = None;
        self.hands.book = None;
        self.hands.minigame = None;
        // (the item back to open when the night already did it: a closing shot)
        if let Some(c) = self.world.app.checklist.iter_mut().find(|c| c.id == key) {
            c.done = false;
            c.by = None;
            c.minute = None;
            c.claim = None;
        }
        let cmd = match key.strip_prefix("equip.") {
            Some(a) => {
                let action = sim::EquipAction::ALL.iter().copied().find(|x| x.id() == a);
                let uid = action.and_then(|action| {
                    self.world
                        .furniture()
                        .iter()
                        .find(|p| self.world.equipment_action(content, p.uid) == Some(action))
                        .map(|p| p.uid)
                        .map(|uid| (uid, action))
                });
                uid.map(|(uid, action)| Command::UseEquipment { uid, action })
            }
            None => Some(Command::TickChecklist { id: key.into() }),
        };
        if let Some(c) = cmd {
            let _ = self.world.apply(content, c);
        }
        // she walks over (the camera on the piece)
        for _ in 0..600 {
            let p = self.world.player();
            if p.current.as_ref().is_some_and(|c| c.stage >= 1)
                || (p.current.is_none() && p.queue.is_empty())
            {
                break;
            }
            self.world.tick(content);
        }
        let _ = self.world.drain_events();
        let hints = BTreeSet::new();
        headless(self, content, loc, &hints, &MgInput::default(), 0.0);
        // the whole café in the picture (screenshots have a window)
        if self.automation && self.rituals.open.is_some() {
            self.cam =
                self.scene
                    .overview_camera(Rect::new(0.0, 0.0, screen_width(), screen_height()));
            self.cam_ready = true;
        }
        // she waits at the piece for it (the card stays for the picture)
        if let Some(o) = self.rituals.open.as_mut()
            && !o.started_sent
        {
            o.started_sent = true;
            let key = o.key.clone();
            let _ = self.world.apply(
                content,
                Command::Ritual {
                    key,
                    step: RitualStep::Start,
                },
            );
        }
        // played a little by the script (never to the end: the picture shows it in play)
        let dt = 1.0 / 30.0;
        let mut t = 0.0;
        while t < secs {
            let Some(o) = self.rituals.open.as_ref() else {
                break;
            };
            let input = o.gesture.autoplay_input();
            let mut probe = o.gesture.clone();
            probe.step(&input, dt);
            if probe.finished().is_some() || probe.progress() > 0.85 {
                break;
            }
            let _ = step(self, content, &input, dt);
            t += dt;
        }
        let _ = self.world.drain_events();
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;

    fn content() -> Rc<Content> {
        let root = env!("CARGO_MANIFEST_DIR");
        Rc::new(Content::load(&format!("{root}/assets/data")).expect("content"))
    }

    fn i18n() -> crate::i18n::I18n {
        let root = env!("CARGO_MANIFEST_DIR");
        crate::settings::load_i18n(&format!("{root}/assets/i18n")).expect("i18n")
    }

    fn prep(c: &Rc<Content>) -> GameScreen {
        let mut g = GameScreen::new_story(c, "cozy", true);
        g.start_night(c);
        let _ = g.world.drain_events();
        g.cam_ready = true;
        g
    }

    /// Queues `id` for her and steps the screen until the card opens (or she is done).
    fn until_card(g: &mut GameScreen, c: &Rc<Content>, loc: &Locale, hints: &BTreeSet<String>) {
        for _ in 0..400 {
            headless(g, c, loc, hints, &MgInput::default(), 1.0 / 30.0);
            if g.rituals.open.is_some() {
                return;
            }
            g.world.tick(c);
        }
    }

    #[test]
    fn a_gesture_finishes_the_item_sooner_and_counts_as_by_hand() {
        let c = content();
        let i18n = i18n();
        let loc = Locale::new(&i18n, crate::i18n::Lang::En);
        let mut g = prep(&c);
        g.rituals.staged = true;
        let hints = BTreeSet::new();
        g.world
            .apply(
                &c,
                Command::TickChecklist {
                    id: "light_window".into(),
                },
            )
            .unwrap();
        until_card(&mut g, &c, &loc, &hints);
        let o = g.rituals.open.as_ref().expect("the cord shows");
        assert_eq!(o.key, "light_window");
        assert!(o.hint, "the first time: the one-line hint");
        // the player pulls the cord
        for _ in 0..400 {
            let input = g
                .rituals
                .open
                .as_ref()
                .map(|o| o.gesture.autoplay_input())
                .unwrap_or_default();
            headless(&mut g, &c, &loc, &hints, &input, 1.0 / 30.0);
            g.world.tick(&c);
            if matches!(
                g.world.checklist_mark("light_window"),
                sim::ChecklistMark::Done { .. }
            ) {
                break;
            }
        }
        assert!(matches!(
            g.world.checklist_mark("light_window"),
            sim::ChecklistMark::Done { .. }
        ));
        assert_eq!(g.world.ritual_by_hand("light_window"), Some(true));
        assert!(g.world.window_light_on(&c));
        assert!(
            g.rituals.seen.iter().any(|s| s == "cord"),
            "the hint was seen"
        );
    }

    #[test]
    fn ignored_it_closes_by_itself_and_she_finishes_as_before() {
        let c = content();
        let i18n = i18n();
        let loc = Locale::new(&i18n, crate::i18n::Lang::En);
        let mut a = prep(&c);
        a.rituals.staged = true;
        let mut b = prep(&c);
        let hints = BTreeSet::new();
        for g in [&mut a, &mut b] {
            g.world
                .apply(
                    &c,
                    Command::TickChecklist {
                        id: "count_float".into(),
                    },
                )
                .unwrap();
        }
        let mut shown = false;
        let mut ticks = [None, None];
        for t in 0..800u32 {
            headless(&mut a, &c, &loc, &hints, &MgInput::default(), 1.0 / 30.0);
            shown |= a.rituals.open.is_some();
            a.world.tick(&c);
            b.world.tick(&c);
            for (k, g) in [&a, &b].iter().enumerate() {
                if ticks[k].is_none()
                    && matches!(
                        g.world.checklist_mark("count_float"),
                        sim::ChecklistMark::Done { .. }
                    )
                {
                    ticks[k] = Some(t);
                }
            }
            if ticks.iter().all(Option::is_some) {
                break;
            }
        }
        headless(&mut a, &c, &loc, &hints, &MgInput::default(), 1.0 / 30.0);
        assert!(shown, "the float card showed");
        assert!(
            ticks[0].is_some() && ticks[0] == ticks[1],
            "same ticks: {ticks:?}"
        );
        assert!(a.rituals.open.is_none(), "the card went with her");
        assert_eq!(a.world.ritual_by_hand("count_float"), None);
    }

    #[test]
    fn it_never_opens_fast_under_the_autoplayer_or_automation() {
        let c = content();
        let i18n = i18n();
        let loc = Locale::new(&i18n, crate::i18n::Lang::En);
        let hints = BTreeSet::new();
        for case in 0..3 {
            let mut g = prep(&c);
            g.rituals.staged = case != 2;
            match case {
                0 => g.set_speed(2),
                1 => {
                    g.hands.autoplay = Some(super::super::hands::Autoplay {
                        cooldown: 0.0,
                        keyboard: false,
                        skip_minigames: false,
                    })
                }
                _ => g.automation = true,
            }
            g.world
                .apply(
                    &c,
                    Command::TickChecklist {
                        id: "light_window".into(),
                    },
                )
                .unwrap();
            for _ in 0..200 {
                headless(&mut g, &c, &loc, &hints, &MgInput::default(), 1.0 / 30.0);
                assert!(g.rituals.open.is_none(), "case {case}");
                g.world.tick(&c);
            }
        }
    }

    #[test]
    fn the_hint_shows_once_and_esc_lets_her_finish() {
        let c = content();
        let i18n = i18n();
        let loc = Locale::new(&i18n, crate::i18n::Lang::En);
        let mut g = prep(&c);
        g.rituals.staged = true;
        let mut hints = BTreeSet::new();
        g.world
            .apply(
                &c,
                Command::TickChecklist {
                    id: "check_stock".into(),
                },
            )
            .unwrap();
        until_card(&mut g, &c, &loc, &hints);
        assert!(g.rituals.open.as_ref().is_some_and(|o| o.hint));
        // a touch, then Esc: she waits no longer and finishes it herself
        for _ in 0..30 {
            let tap = g.rituals.open.as_ref().unwrap().gesture.autoplay_input();
            headless(&mut g, &c, &loc, &hints, &tap, 1.0 / 30.0);
            if g.rituals.open.as_ref().is_some_and(|o| o.gesture.started()) {
                break;
            }
        }
        assert!(g.world.ritual_hold().is_some(), "she waits for the shelf");
        let_go(&mut g, &c);
        assert!(g.world.ritual_hold().is_none());
        assert!(g.rituals.open.is_none());
        hints.extend(g.rituals.seen.iter().cloned());
        assert!(hints.contains("shelf"));
        for _ in 0..200 {
            headless(&mut g, &c, &loc, &hints, &MgInput::default(), 1.0 / 30.0);
            assert!(
                g.rituals.open.is_none(),
                "dismissed: no reopening for this act"
            );
            g.world.tick(&c);
        }
        assert!(matches!(
            g.world.checklist_mark("check_stock"),
            sim::ChecklistMark::Done { .. }
        ));
        assert_eq!(g.world.ritual_by_hand("check_stock"), None);
        // the fridge check later uses the same kind: no hint again
        g.world.skip_prep(&c);
        g.world.tick = 470 * sim::TICKS_PER_MINUTE;
        g.world
            .apply(
                &c,
                Command::TickChecklist {
                    id: "fridge_check".into(),
                },
            )
            .unwrap();
        until_card(&mut g, &c, &loc, &hints);
        let o = g.rituals.open.as_ref().expect("the fridge shelf");
        assert!(!o.hint, "hints show once per gesture kind");
    }

    #[test]
    fn pausing_pauses_the_gesture() {
        let c = content();
        let i18n = i18n();
        let loc = Locale::new(&i18n, crate::i18n::Lang::En);
        let mut g = prep(&c);
        g.rituals.staged = true;
        let hints = BTreeSet::new();
        g.world
            .apply(
                &c,
                Command::TickChecklist {
                    id: "light_window".into(),
                },
            )
            .unwrap();
        until_card(&mut g, &c, &loc, &hints);
        g.toggle_pause();
        for _ in 0..100 {
            let input = g.rituals.open.as_ref().unwrap().gesture.autoplay_input();
            headless(&mut g, &c, &loc, &hints, &input, 1.0 / 30.0);
        }
        let o = g.rituals.open.as_ref().expect("still there, waiting");
        assert!(!o.gesture.started() && o.gesture.progress() == 0.0);
    }
}
