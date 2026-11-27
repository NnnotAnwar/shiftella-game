//! Photo mode (design/v0.2.md I.6): at night **P** or the little camera button beside the
//! atmosphere meter raises Karolina's instant camera. The café holds still (the sim waits, the
//! animations slow to a breath), a viewfinder frames part of it; the player may pan / zoom a
//! little (WASD, drag, wheel) and clicks to take the photo. A flash, then the print slides out
//! and develops (shake it — drag over it — to hurry it) and goes into the journal as an extra
//! photo of the night (`Command::TakePhoto`: scene data, never an image file). Esc cancels.

use macroquad::prelude::*;

use super::world_view::sim;
use super::{Ctx, GameScreen, Panel};
use crate::map::TILE;
use crate::ui::draw::{fill_rrect, line, stroke_rrect, with_a};
use crate::ui::handson::PolaroidView;
use crate::ui::{Align, Glass, Icon, IconButton, RectExt, TextStyle, ToastKind, Ui, Weight};
use sim::{NightPhoto, Phase, signature};

/// The viewfinder: what the camera looked at before it was raised.
#[derive(Debug, Clone, Copy)]
struct Finder {
    since: f32,
    center: Vec2,
    zoom: f32,
}

/// The print that just came out of the camera.
#[derive(Debug, Clone)]
struct Print {
    index: usize,
    born: f32,
    develop: f32,
    /// Where the viewfinder was (the print shrinks out of it).
    from: Rect,
    /// Real time it finished developing (then it rests a moment and fades).
    done_at: Option<f32>,
}

#[derive(Debug, Clone, Default)]
pub struct PhotoMode {
    finder: Option<Finder>,
    /// Screenshot scenes: raise the camera once the café camera is placed.
    pending: bool,
    print: Option<Print>,
    /// Real time of the last shutter (the flash).
    flash: Option<f32>,
}

impl PhotoMode {
    /// The viewfinder is up (the café holds still).
    pub fn is_open(&self) -> bool {
        self.finder.is_some() || self.pending
    }
    /// Screenshot runs: raise the camera once the café camera is placed.
    pub fn raise_for_shot(&mut self) {
        self.pending = true;
    }
    /// A print is developing on screen (its photo has to be rendered).
    pub fn developing(&self) -> bool {
        self.print.is_some()
    }
}

/// The photo key in the journal's cache (stable per photo).
pub fn photo_key(p: &NightPhoto) -> u64 {
    let mut k: u64 = 0x9e37_79b9_7f4a_7c15;
    for v in [
        p.night as u64,
        p.minute as u64,
        p.snap.seed as u64,
        p.snap.focus.0.to_bits() as u64,
        p.snap.focus.1.to_bits() as u64,
    ] {
        k = (k ^ v).wrapping_mul(0x0100_0000_01b3);
    }
    k | 1 << 63
}

/// The caption of an extra photo ("00:42, just us and the rain").
pub fn caption(loc: &crate::settings::Locale, p: &NightPhoto) -> String {
    let m = p.minute;
    let (h, mm) = ((22 + m / 60) % 24, m % 60);
    loc.trf(
        &p.caption_key(),
        &[("time", &crate::i18n::format_clock(h, mm))],
    )
}

fn can_open(g: &GameScreen) -> bool {
    g.world.phase() == Phase::Night
        && !g.player.is_playing()
        && g.summary.is_none()
        && !g.walk.is_open()
        && g.pause.is_none()
        && !g.hands.overlay()
        && !g.sig.open
        && g.panel != Some(Panel::Build)
        // a stranger's story card is a card like a moment's; the storm talk holds the café
        && g.stories.card.is_none()
        && !g.stories.talk_open()
}

/// Raises the camera (or says the film is used up).
pub fn open(g: &mut GameScreen, loc: &crate::settings::Locale) {
    if !can_open(g) {
        return;
    }
    if g.world.photos_left() == 0 {
        g.toast(
            ToastKind::Info,
            Icon::Camera,
            loc.tr_owned("photo.no_film"),
            Some(loc.tr_owned("photo.no_film_body")),
        );
        return;
    }
    g.panel = None;
    g.selected = None;
    g.bubble = None;
    g.photo.finder = Some(Finder {
        since: g.time,
        center: g.cam.center,
        zoom: g.cam.zoom,
    });
}

fn close(g: &mut GameScreen) {
    if let Some(f) = g.photo.finder.take() {
        // back where she was looking
        g.cam.center = f.center;
        g.cam.zoom = f.zoom;
    }
}

/// Keeps the viewfinder's frame inside the café (call right after the camera's own input).
/// `s` = the screen rect (logical points).
pub fn clamp_camera(g: &mut GameScreen, s: Rect) {
    if g.photo.pending {
        g.photo.pending = false;
        g.photo.finder = Some(Finder {
            since: g.time - 10.0,
            center: g.cam.center,
            zoom: g.cam.zoom,
        });
    }
    let Some(f) = g.photo.finder else {
        return;
    };
    let r = signature::data().photo;
    g.cam.zoom = g.cam.zoom.clamp(f.zoom * r.zoom.0, f.zoom * r.zoom.1);
    // the viewfinder may go anywhere over the café (owner: "let me shoot the left or the right
    // part"): keep the framed area inside the café's bounds, not near where it opened
    let fr = finder_rect(s);
    let z = g.cam.zoom.max(0.01);
    let off = (fr.center() - s.center()) / z;
    let half = fr.size() / z / 2.0;
    let b = g.scene.bounds();
    let fit = |c: f32, lo: f32, hi: f32, h: f32| {
        if hi - lo <= 2.0 * h {
            (lo + hi) / 2.0
        } else {
            c.clamp(lo + h, hi - h)
        }
    };
    let framed = g.cam.center + off;
    let framed = vec2(
        fit(framed.x, b.x, b.right(), half.x),
        fit(framed.y, b.y, b.bottom(), half.y),
    );
    g.cam.center = framed - off;
}

/// Esc / P while the camera is up. True when photo mode had the keyboard.
pub fn hotkeys(g: &mut GameScreen, ui: &mut Ui, loc: &crate::settings::Locale) -> bool {
    if g.photo.finder.is_some() {
        if ui.esc() || crate::settings::keys::pressed(ui, crate::settings::keys::Action::Photo) {
            close(g);
        }
        return true;
    }
    if g.world.phase() == Phase::Night
        && crate::settings::keys::pressed(ui, crate::settings::keys::Action::Photo)
        && can_open(g)
    {
        open(g, loc);
        return true;
    }
    false
}

/// The viewfinder rect (4:3, centred, room for the HUD above and the hint below).
fn finder_rect(s: Rect) -> Rect {
    let h = (s.h - 190.0).clamp(160.0, 520.0);
    let w = (h * 4.0 / 3.0).min(s.w - 60.0);
    let h = w * 0.75;
    Rect::new((s.w - w) / 2.0, (s.h - h) / 2.0 - 6.0, w, h)
}

/// The camera button beside the atmosphere meter (and the back-step chip), night only.
pub fn button(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx) {
    if g.world.phase() != Phase::Night || g.summary.is_some() || g.photo.finder.is_some() {
        return;
    }
    let loc = cx.loc;
    let size = crate::ui::handson::ATMO_SIZE;
    // under the atmosphere meter, at the left edge (the rail and the back-step chip sit
    // to its right)
    let top = g
        .anchors
        .iter()
        .find(|a| a.0 == "hud.atmosphere")
        .map(|a| a.1.bottom() + 6.0)
        .unwrap_or(12.0 + 64.0 + 6.0 + size.y + 6.0);
    let r = Rect::new(12.0, top, 30.0, 30.0);
    g.anchors.push(("hud.camera", r));
    ui.glass(r, &Glass::hud().radius(8.0));
    let left_n = g.world.photos_left();
    let tip = format!(
        "{} (P) · {}",
        loc.tr("photo.button_hint"),
        loc.trf(
            "photo.film",
            &[
                ("n", &left_n.to_string()),
                ("max", &signature::data().photo.per_night.to_string()),
            ],
        )
    );
    let ok = left_n > 0 && can_open(g);
    let b = IconButton::ghost()
        .size(15.0)
        .tooltip(&tip)
        .disabled(left_n == 0);
    if ui
        .icon_button("photo_btn", r.inset(2.0), Icon::Camera, &b)
        .clicked
        && ok
    {
        open(g, &loc);
    }
    // film left tonight: a small amber number in the corner
    if left_n > 0 {
        let st = TextStyle::new(9.5, Weight::Bold, ui.theme.amber).tabular();
        ui.fonts.draw_in(
            &left_n.to_string(),
            Rect::new(r.right() - 11.0, r.bottom() - 12.0, 10.0, 11.0),
            &st,
            Align::Center,
        );
    }
}

/// The viewfinder over the café, the shutter, the flash and the developing print.
pub fn frame_ui(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx, dt: f32) {
    let content = cx.content;
    let loc = cx.loc;
    let s = ui.screen();
    let t = ui.theme.clone();
    if let Some(f) = g.photo.finder {
        // the café holds still under the viewfinder
        let fr = finder_rect(s);
        let a = ((g.time - f.since) / 0.25).clamp(0.0, 1.0);
        let dim = Color::new(0.02, 0.01, 0.06, 0.62 * a);
        fill_rrect(Rect::new(0.0, 0.0, s.w, fr.y), 0.0, dim);
        fill_rrect(
            Rect::new(0.0, fr.bottom(), s.w, s.h - fr.bottom()),
            0.0,
            dim,
        );
        fill_rrect(Rect::new(0.0, fr.y, fr.x, fr.h), 0.0, dim);
        fill_rrect(
            Rect::new(fr.right(), fr.y, s.w - fr.right(), fr.h),
            0.0,
            dim,
        );
        // the polaroid's white edge and the corner marks
        stroke_rrect(
            fr.inset(-6.0),
            3.0,
            6.0,
            with_a(Color::from_hex(0xF4EFE4), 0.85 * a),
        );
        let cm = with_a(WHITE, 0.8 * a);
        let l = 18.0;
        for (p, dx, dy) in [
            (vec2(fr.x + 10.0, fr.y + 10.0), 1.0, 1.0),
            (vec2(fr.right() - 10.0, fr.y + 10.0), -1.0, 1.0),
            (vec2(fr.x + 10.0, fr.bottom() - 10.0), 1.0, -1.0),
            (vec2(fr.right() - 10.0, fr.bottom() - 10.0), -1.0, -1.0),
        ] {
            line(p, p + vec2(l * dx, 0.0), 1.5, cm);
            line(p, p + vec2(0.0, l * dy), 1.5, cm);
        }
        // a focus mark in the middle
        let c = fr.center();
        crate::ui::draw::stroke_circle(c, 9.0, 1.0, with_a(WHITE, 0.45 * a));
        // the hint and the film counter under the frame
        let hint = loc.tr_owned("photo.hint");
        let film = loc.trf(
            "photo.film",
            &[
                ("n", &g.world.photos_left().to_string()),
                ("max", &signature::data().photo.per_night.to_string()),
            ],
        );
        let st = t.small().color(with_a(t.text, 0.9 * a));
        let hr = Rect::new(fr.x, fr.bottom() + 12.0, fr.w, 18.0);
        ui.fonts.draw_in(&hint, hr, &st, Align::Center);
        let fs = t.small_medium().color(with_a(t.amber, a));
        crate::ui::draw_icon(
            Icon::Camera,
            vec2(fr.x + 8.0, fr.y - 14.0),
            13.0,
            with_a(t.amber, a),
        );
        ui.fonts.draw_in(
            &film,
            Rect::new(fr.x + 20.0, fr.y - 24.0, fr.w / 2.0, 20.0),
            &fs,
            Align::Left,
        );
        // a click inside the frame takes it (after a short beat, so the opening click doesn't)
        let m = ui.mouse();
        let shoot = (ui.mouse_pressed() && fr.contains(m) && g.time - f.since > 0.2)
            || ui.enter()
            || ui.take_key(KeyCode::Space);
        if shoot {
            let scale = ui.scale();
            let tl = g.cam.screen_to_world(vec2(fr.x, fr.y) * scale);
            let br = g.cam.screen_to_world(vec2(fr.right(), fr.bottom()) * scale);
            let focus = (tl + br) / 2.0;
            let width = br.x - tl.x;
            match g.command(
                content,
                sim::Command::TakePhoto {
                    focus: (focus.x, focus.y),
                    width,
                },
            ) {
                Ok(()) => {
                    // the instant camera's shutter and the print sliding out
                    cx.audio.play_sfx(crate::audio::Sfx::PhotoShutter);
                    cx.audio.play_sfx_vol(crate::audio::Sfx::PhotoEject, 0.8);
                    g.photo.flash = Some(g.time);
                    g.photo.print = Some(Print {
                        index: g.world.photos().len().saturating_sub(1),
                        born: g.time,
                        develop: 0.0,
                        from: fr,
                        done_at: None,
                    });
                    close(g);
                }
                Err(e) => {
                    super::panels::command_error(g, &loc, &e);
                    close(g);
                }
            }
        }
    }
    // the flash
    if let Some(at) = g.photo.flash {
        let a = 1.0 - (g.time - at) / 0.35;
        if a <= 0.0 {
            g.photo.flash = None;
        } else {
            fill_rrect(s, 0.0, Color::new(1.0, 0.97, 0.9, 0.85 * a * a));
        }
    }
    print_ui(g, ui, cx, dt);
}

/// The print: it shrinks out of the viewfinder to the lower right, develops (shake it to
/// hurry it), rests a moment and fades. Click it to open the journal.
fn print_ui(g: &mut GameScreen, ui: &mut Ui, cx: &mut Ctx, dt: f32) {
    let content = cx.content;
    let loc = cx.loc;
    let Some(p) = g.photo.print.clone() else {
        return;
    };
    let Some(photo) = g.world.photos().get(p.index).cloned() else {
        g.photo.print = None;
        return;
    };
    let s = ui.screen();
    let age = g.time - p.born;
    let w = 168.0;
    let size = crate::ui::handson::polaroid_size(w);
    let rest = vec2(s.w - 24.0 - w / 2.0, s.h - 84.0 - size.y / 2.0);
    let out = crate::ui::ease_out((age / 0.45).clamp(0.0, 1.0));
    let mut c = p.from.center().lerp(rest, out);
    let wid = p.from.w + (w - p.from.w) * out;
    let tex = g.photos.get_snap(content, photo_key(&photo), &photo.snap);
    let mut alpha = 1.0;
    if let Some(done) = p.done_at {
        let f = ((g.time - done - 2.0) / 0.6).clamp(0.0, 1.0);
        if f >= 1.0 {
            g.photo.print = None;
            return;
        }
        c.y += 40.0 * f;
        alpha = 1.0 - f;
    }
    let view = PolaroidView {
        develop: if tex.is_some() { p.develop } else { 0.0 },
        photo: tex.clone(),
        flip_y: true,
        caption: caption(&loc, &photo),
        date: super::journal_ui::date_text(&loc, photo.date),
        tilt: crate::render::polaroid_tilt(photo.snap.seed).to_degrees(),
    };
    let ps = crate::ui::handson::polaroid_size(wid);
    ui.region(Rect::new(c.x - ps.x / 2.0, c.y - ps.y / 2.0, ps.x, ps.y));
    crate::ui::draw::push_alpha(alpha);
    let resp = crate::ui::handson::polaroid(ui, ("photo_print", p.index), c, wid, &view);
    if out >= 1.0 {
        let label = if p.develop < 0.98 {
            loc.tr_owned("photo.shake")
        } else {
            loc.tr_owned("photo.taken")
        };
        let st = TextStyle::new(11.0, Weight::SemiBold, ui.theme.text);
        let lr = Rect::new(c.x - w / 2.0, c.y + size.y / 2.0 + 4.0, w, 16.0);
        ui.fonts.draw_in(&label, lr, &st, Align::Center);
    }
    crate::ui::draw::pop_alpha();
    // shaking it (dragging over the print) develops it faster, like the morning polaroid
    let shake = if resp.hovered && ui.mouse_down() {
        2.5
    } else {
        1.0
    };
    if resp.clicked && p.develop >= 0.98 {
        g.photo.print = None;
        g.panel = Some(Panel::Journal);
        return;
    }
    let now = g.time;
    if let Some(pr) = g.photo.print.as_mut()
        && tex.is_some()
        && out >= 1.0
        && pr.done_at.is_none()
    {
        pr.develop = (pr.develop + dt / 2.2 * shake).min(1.0);
        if pr.develop >= 1.0 {
            pr.done_at = Some(now);
        }
    }
}

// ───────────────────────── screenshot scenes ─────────────────────────

/// `SHIFTELLA_SCENE=photo_mode` (the viewfinder up at night) and `photo_print` (a print
/// developing in the corner).
pub fn scene_setup(
    g: &mut GameScreen,
    content: &std::rc::Rc<crate::content::Content>,
    loc: &crate::settings::Locale,
    name: &str,
) {
    match name {
        "photo_print" => {
            let c = g.cam.center;
            let _ = g.command(
                content,
                sim::Command::TakePhoto {
                    focus: (c.x, c.y),
                    width: 10.0 * TILE,
                },
            );
            g.photo.print = Some(Print {
                index: g.world.photos().len().saturating_sub(1),
                born: -10.0,
                develop: std::env::var("SHIFTELLA_DEVELOP")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0.6),
                from: Rect::new(0.0, 0.0, 100.0, 75.0),
                done_at: None,
            });
        }
        _ => {
            g.photo.pending = true;
            let _ = loc;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Content;
    use crate::i18n::Lang;
    use crate::settings::Locale;
    use std::rc::Rc;

    #[test]
    fn the_camera_holds_the_cafe_and_counts_the_film() {
        let root = env!("CARGO_MANIFEST_DIR");
        let c = Rc::new(Content::load(&format!("{root}/assets/data")).expect("content"));
        let i = crate::settings::load_i18n(&format!("{root}/assets/i18n")).expect("i18n");
        let loc = Locale::new(&i, Lang::Ru);
        let mut g = GameScreen::new_story(&c, "cozy", true);
        // by day there is no camera
        open(&mut g, &loc);
        assert!(!g.photo.is_open());
        g.fast_forward(&c, &loc, 3, Some(60), true);
        g.set_speed(1);
        // whatever the fast-forward left open (an order card, a moment card)
        g.hands.order = None;
        g.hands.moment = None;
        g.hands.book = None;
        g.panel = None;
        assert!(
            can_open(&g),
            "phase {:?} cut {} sum {} walk {} pause {} overlay {}",
            g.world.phase(),
            g.player.is_playing(),
            g.summary.is_some(),
            g.walk.is_open(),
            g.pause.is_some(),
            g.hands.overlay()
        );
        let before = (g.cam.center, g.cam.zoom);
        open(&mut g, &loc);
        assert!(g.photo.is_open());
        assert!(g.frozen(), "the café holds still under the viewfinder");
        // the viewfinder can go to either end of the café, but not past it
        let screen = Rect::new(0.0, 0.0, 1280.0, 720.0);
        let r = signature::data().photo;
        g.cam.zoom = before.1 * 10.0;
        clamp_camera(&mut g, screen);
        assert!((g.cam.zoom - before.1 * r.zoom.1).abs() < 1e-3);
        let b = g.scene.bounds();
        for dx in [-500.0 * TILE, 500.0 * TILE] {
            g.cam.center = before.0 + vec2(dx, 0.0);
            clamp_camera(&mut g, screen);
            let fr = finder_rect(screen);
            let framed = g.cam.center + (fr.center() - screen.center()) / g.cam.zoom;
            let half = fr.w / g.cam.zoom / 2.0;
            assert!(framed.x - half >= b.x - 0.5 && framed.x + half <= b.right() + 0.5);
            // and it really reaches that end, not just a few tiles from the start
            let end = if dx < 0.0 {
                b.x + half
            } else {
                b.right() - half
            };
            assert!(
                (framed.x - end).abs() < 1.0,
                "reaches the {} end",
                if dx < 0.0 { "left" } else { "right" }
            );
        }
        close(&mut g);
        assert!(!g.photo.is_open() && !g.frozen());
        assert_eq!(
            (g.cam.center, g.cam.zoom),
            before,
            "back where she was looking"
        );
        // the film runs out; then the camera only says so
        for _ in 0..r.per_night {
            let k = g.world.karolina().unwrap().pos;
            g.command(
                &c,
                sim::Command::TakePhoto {
                    focus: k,
                    width: 8.0 * TILE,
                },
            )
            .unwrap();
        }
        g.notices.clear();
        open(&mut g, &loc);
        assert!(!g.photo.is_open());
        assert!(
            g.notices.iter().any(|n| matches!(n,
            crate::screens::game::Notice::Toast { title, .. } if title == loc.tr("photo.no_film")))
        );
        // captions and keys are stable per photo
        let p = &g.world.photos()[0];
        assert_eq!(photo_key(p), photo_key(&p.clone()));
        assert_ne!(photo_key(p), photo_key(&g.world.photos()[1]));
        let cap = caption(&loc, p);
        assert!(cap.contains("23:") && !cap.contains('{'), "{cap}");
    }
}
