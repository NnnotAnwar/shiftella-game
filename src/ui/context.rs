//! The immediate-mode `Ui` context: input in UI units, ids, hot/active state,
//! hover easing, z-ordered blocking regions, clipping, keyboard helpers, sounds.

use std::collections::HashMap;
use std::hash::Hash;

use macroquad::miniquad::CursorIcon;
use macroquad::miniquad::window::set_mouse_cursor;
use macroquad::prelude::*;

use super::draw::{self, set_scissor};
use super::id::Id;
use super::layout::RectExt;
use super::text::Fonts;
use super::theme::Theme;
use super::toast::{ToastKind, ToastState};
use super::widgets::{Popup, TextEdit};

/// What the game should play (map to `Sfx::UiClick` etc.). Drain with [`Ui::drain_sounds`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiSound {
    Click,
    Toggle,
    Toast(ToastKind),
    /// Dialogue line advanced.
    Advance,
    /// A cutscene choice was picked.
    Choice,
}

/// Result of interacting with a widget this frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct Response {
    pub id: Option<Id>,
    pub rect: Rect,
    /// Mouse is over it (and not blocked by something on top).
    pub hovered: bool,
    /// Mouse button is held on it.
    pub pressed: bool,
    /// Released over it after pressing on it.
    pub clicked: bool,
    pub right_clicked: bool,
    /// The bound value changed (toggles, sliders, text...).
    pub changed: bool,
    /// Eased 0..1 hover amount (for drawing).
    pub hover_t: f32,
    /// Eased 0..1 press amount.
    pub press_t: f32,
}

/// Per-widget animation scratch, keyed by id. `v`/`t`/`n` meaning is widget specific.
#[derive(Clone, Copy, Debug, Default)]
pub struct Anim {
    pub hover: f32,
    pub press: f32,
    pub v: f32,
    pub t: f32,
    pub n: f32,
    pub(crate) seen: u64,
}

/// Exponential approach towards `target`; `tau` ≈ time to cover 63 %.
pub fn approach(current: f32, target: f32, dt: f32, tau: f32) -> f32 {
    if tau <= 0.0 {
        return target;
    }
    let k = 1.0 - (-dt / tau).exp();
    let v = current + (target - current) * k;
    if (v - target).abs() < 1e-3 { target } else { v }
}

/// Smoothstep easing 0..1.
pub fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Cubic ease-out 0..1.
pub fn ease_out(t: f32) -> f32 {
    let t = 1.0 - t.clamp(0.0, 1.0);
    1.0 - t * t * t
}

const HOVER_TAU: f32 = 0.045;

pub struct Ui {
    pub fonts: Fonts,
    pub theme: Theme,
    /// Faces for avatars by display name (the game registers its people; see
    /// [`Ui::register_face`]). Names without a face show initials.
    pub(crate) faces: std::collections::HashMap<String, crate::render::PersonLook>,
    user_scale: f32,
    screen: Vec2,
    time: f64,
    dt: f32,
    frame: u64,
    /// `SHIFTELLA_LAYOUT_DEBUG=1`: widgets / text outside their container are outlined in
    /// red and logged once.
    layout_debug: bool,
    layout_issues: Vec<(Rect, String)>,
    layout_logged: std::collections::HashSet<String>,
    // input (UI units)
    mouse: Vec2,
    mouse_prev: Vec2,
    down: bool,
    pressed: bool,
    released: bool,
    rpressed: bool,
    wheel: Vec2,
    chars: Vec<char>,
    ctrl: bool,
    shift: bool,
    consumed: Vec<KeyCode>,
    repeat_key: Option<KeyCode>,
    repeat_at: f64,
    repeat_fire: Vec<KeyCode>,
    // interaction
    hot: Option<Id>,
    active: Option<Id>,
    pub(crate) focus: Option<Id>,
    pub(crate) focus_seen: bool,
    pub(crate) focus_claimed: bool,
    hover_id: Option<Id>,
    hover_since: f64,
    hovered_any: bool,
    hot_kind: CursorIcon,
    cursor: CursorIcon,
    anims: HashMap<Id, Anim>,
    z: u32,
    regions: Vec<(Rect, u32)>,
    prev_regions: Vec<(Rect, u32)>,
    sounds: Vec<UiSound>,
    tooltip: Option<(String, Rect)>,
    clip: Vec<Rect>,
    ids: Vec<Id>,
    pub(crate) toasts: ToastState,
    pub(crate) popup: Option<Popup>,
    pub(crate) popup_pick: Option<(Id, usize)>,
    pub(crate) text_edit: TextEdit,
}

impl Ui {
    pub fn new(fonts: Fonts) -> Ui {
        let layout_debug = std::env::var("SHIFTELLA_LAYOUT_DEBUG").is_ok_and(|v| v != "0");
        super::text::LAYOUT_DEBUG.store(layout_debug, std::sync::atomic::Ordering::Relaxed);
        Ui {
            fonts,
            theme: Theme::SHIFTELLA,
            faces: std::collections::HashMap::new(),
            layout_debug,
            layout_issues: Vec::new(),
            layout_logged: std::collections::HashSet::new(),
            user_scale: 1.0,
            screen: vec2(1280.0, 720.0),
            time: 0.0,
            dt: 1.0 / 60.0,
            frame: 0,
            mouse: Vec2::ZERO,
            mouse_prev: Vec2::ZERO,
            down: false,
            pressed: false,
            released: false,
            rpressed: false,
            wheel: Vec2::ZERO,
            chars: Vec::new(),
            ctrl: false,
            shift: false,
            consumed: Vec::new(),
            repeat_key: None,
            repeat_at: 0.0,
            repeat_fire: Vec::new(),
            hot: None,
            active: None,
            focus: None,
            focus_seen: false,
            focus_claimed: false,
            hover_id: None,
            hover_since: 0.0,
            hovered_any: false,
            hot_kind: CursorIcon::Default,
            cursor: CursorIcon::Default,
            anims: HashMap::new(),
            z: 0,
            regions: Vec::new(),
            prev_regions: Vec::new(),
            sounds: Vec::new(),
            tooltip: None,
            clip: Vec::new(),
            ids: Vec::new(),
            toasts: ToastState::default(),
            popup: None,
            popup_pick: None,
            text_edit: TextEdit::default(),
        }
    }

    // --- scale & screen ---------------------------------------------------

    /// User UI scale (settings: 0.9–1.3; clamped to 0.75–1.5). Applied on top of DPI.
    pub fn set_scale(&mut self, s: f32) {
        self.user_scale = s.clamp(0.75, 1.5);
    }
    pub fn scale(&self) -> f32 {
        self.user_scale
    }
    /// Settings → Text size (1.0–1.3): scales the theme's type scale only (panels sized by
    /// their text grow with it; the UI scale stays). See [`crate::ui::TextStyle::scale`].
    pub fn set_text_scale(&mut self, s: f32) {
        self.theme.text_scale = if s.is_finite() {
            s.clamp(1.0, 1.3)
        } else {
            1.0
        };
    }
    pub fn text_scale(&self) -> f32 {
        self.theme.text_scale
    }
    /// Screen in UI units.
    pub fn screen(&self) -> Rect {
        Rect::new(0.0, 0.0, self.screen.x, self.screen.y)
    }
    pub fn mouse(&self) -> Vec2 {
        self.mouse
    }
    pub fn mouse_delta(&self) -> Vec2 {
        self.mouse - self.mouse_prev
    }
    pub fn time(&self) -> f64 {
        self.time
    }
    pub fn dt(&self) -> f32 {
        self.dt
    }
    /// Avatars of `name` show this person's face instead of initials. Call at the start of a
    /// frame, before drawing: a new face is rendered into a small texture right away.
    pub fn register_face(&mut self, name: &str, look: crate::render::PersonLook) {
        if self.faces.get(name) != Some(&look) {
            self.faces.insert(name.to_string(), look);
        }
        if crate::render::faces::prepare(name, &look) {
            set_default_camera();
        }
    }

    /// Frames since the Ui was created (detects "not drawn last frame").
    pub fn frame_count(&self) -> u64 {
        self.frame
    }
    /// Converts a UI-unit point to window logical points (what macroquad's default camera uses).
    pub fn to_screen_points(&self, p: Vec2) -> Vec2 {
        p * self.user_scale
    }

    // --- frame ------------------------------------------------------------

    /// Read input for this frame. Call first thing in the frame.
    pub fn begin_frame(&mut self) {
        self.frame += 1;
        self.dt = get_frame_time().min(0.1);
        self.time += self.dt as f64;
        let dpi = screen_dpi_scale();
        draw::set_px(1.0 / (dpi * self.user_scale));
        self.fonts.set_scale(self.user_scale);
        self.screen = vec2(screen_width(), screen_height()) / self.user_scale;

        self.mouse_prev = self.mouse;
        self.mouse = Vec2::from(mouse_position()) / self.user_scale;
        self.down = is_mouse_button_down(MouseButton::Left);
        self.pressed = is_mouse_button_pressed(MouseButton::Left);
        self.released = is_mouse_button_released(MouseButton::Left);
        self.rpressed = is_mouse_button_pressed(MouseButton::Right);
        let (wx, wy) = mouse_wheel();
        self.wheel = vec2(wx, wy);
        self.chars.clear();
        while let Some(c) = get_char_pressed() {
            self.chars.push(c);
        }
        self.ctrl = is_key_down(KeyCode::LeftControl)
            || is_key_down(KeyCode::RightControl)
            || is_key_down(KeyCode::LeftSuper)
            || is_key_down(KeyCode::RightSuper);
        self.shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
        self.consumed.clear();
        self.update_repeat();

        self.prev_regions = std::mem::take(&mut self.regions);
        self.z = 0;
        self.hot = None;
        self.hot_kind = CursorIcon::Default;
        self.hovered_any = false;
        self.sounds.clear();
        self.tooltip = None;
        self.clip.clear();
        self.ids.clear();
        self.focus_seen = false;
        self.focus_claimed = false;

        // Esc closes the innermost transient thing first.
        if is_key_pressed(KeyCode::Escape) {
            if self.popup.is_some() {
                self.popup = None;
                self.consume_key(KeyCode::Escape);
            } else if self.focus.is_some() {
                self.focus = None;
                self.consume_key(KeyCode::Escape);
            }
        }
    }

    fn update_repeat(&mut self) {
        const KEYS: [KeyCode; 6] = [
            KeyCode::Backspace,
            KeyCode::Delete,
            KeyCode::Left,
            KeyCode::Right,
            KeyCode::Up,
            KeyCode::Down,
        ];
        self.repeat_fire.clear();
        for k in KEYS {
            if is_key_pressed(k) {
                self.repeat_key = Some(k);
                self.repeat_at = self.time + 0.42;
                self.repeat_fire.push(k);
            }
        }
        if let Some(k) = self.repeat_key {
            if !is_key_down(k) {
                self.repeat_key = None;
            } else if self.time >= self.repeat_at {
                self.repeat_at = self.time + 0.035;
                if !self.repeat_fire.contains(&k) {
                    self.repeat_fire.push(k);
                }
            }
        }
    }

    /// Switch to the UI camera (UI units, y down). Call after drawing the world.
    pub fn set_camera(&self) {
        let mut cam =
            Camera2D::from_display_rect(Rect::new(0.0, 0.0, self.screen.x, self.screen.y));
        // from_display_rect is meant for render targets; for the screen flip Y (brief §3).
        cam.zoom.y = -cam.zoom.y;
        set_camera(&cam);
    }

    /// Draw overlays (dropdown popup, tooltip), finish interaction bookkeeping.
    pub fn end_frame(&mut self) {
        self.clip.clear();
        set_scissor(None);
        if self.layout_debug {
            self.layout_issues.extend(super::text::take_overflows());
            for (r, what) in std::mem::take(&mut self.layout_issues) {
                super::draw::stroke_rrect(r, 0.0, 1.5, Color::new(1.0, 0.2, 0.3, 0.9));
                let key = format!("{what} @ {:.0},{:.0} {:.0}x{:.0}", r.x, r.y, r.w, r.h);
                if self.layout_logged.insert(key.clone()) {
                    eprintln!("layout: {key}");
                }
            }
            for text in super::text::take_cuts() {
                let key = format!("cut: {text}");
                if self.layout_logged.insert(key.clone()) {
                    eprintln!("layout: {key}");
                }
            }
        }
        self.draw_popup();
        self.draw_tooltip();

        if self.released || !self.down {
            self.active = None;
        }
        if self.focus.is_some() && (!self.focus_seen || (self.pressed && !self.focus_claimed)) {
            self.focus = None;
        }
        if !self.hovered_any {
            self.hover_id = None;
        }
        let frame = self.frame;
        self.anims.retain(|_, a| frame - a.seen < 240);

        let want = if self.active.is_some() && self.focus.is_none() {
            self.cursor
        } else {
            self.hot_kind
        };
        if want != self.cursor {
            self.cursor = want;
            set_mouse_cursor(want);
        }
    }

    // --- ids --------------------------------------------------------------

    /// Id of `key` under the current id scope.
    pub fn id(&self, key: impl Hash) -> Id {
        self.ids.last().copied().unwrap_or(Id::ROOT).with(key)
    }
    pub fn push_id(&mut self, key: impl Hash) {
        let id = self.id(key);
        self.ids.push(id);
    }
    pub fn pop_id(&mut self) {
        self.ids.pop();
    }

    // --- regions, z, clipping ----------------------------------------------

    /// Declare an opaque UI area (panel, card, HUD block). Widgets drawn after it sit on a
    /// higher layer, the world won't get clicks there and lower widgets are blocked.
    pub fn region(&mut self, r: Rect) {
        self.z += 1;
        self.regions.push((r, self.z));
    }

    /// Current layer (increments with every [`Ui::region`]).
    pub fn layer(&self) -> u32 {
        self.z
    }

    /// True while the mouse is over UI or a widget is being dragged/edited.
    /// During the frame it uses last frame's regions plus those declared so far.
    pub fn wants_mouse(&self) -> bool {
        let m = self.mouse;
        self.active.is_some()
            || self.popup.is_some()
            || self.prev_regions.iter().any(|(r, _)| r.contains(m))
            || self.regions.iter().any(|(r, _)| r.contains(m))
    }

    /// True while a text field has focus (game hotkeys must be ignored).
    pub fn wants_keyboard(&self) -> bool {
        self.focus.is_some()
    }

    /// Is `p` covered by a higher layer? A region with the widget's own rect (its glass /
    /// card background, often declared right after `interact`) never blocks the widget.
    fn blocked(&self, p: Vec2, own: Rect) -> bool {
        let same = |a: &Rect| {
            (a.x - own.x).abs() < 1.0
                && (a.y - own.y).abs() < 1.0
                && (a.w - own.w).abs() < 1.0
                && (a.h - own.h).abs() < 1.0
        };
        self.prev_regions
            .iter()
            .any(|(r, z)| *z > self.z && r.contains(p) && !same(r))
    }

    /// Intersect with the current clip and enable the GPU scissor.
    pub fn push_clip(&mut self, r: Rect) {
        let r = match self.clip.last() {
            Some(c) => c.intersect_or_empty(r),
            None => r,
        };
        self.clip.push(r);
        set_scissor(Some(r));
    }
    pub fn pop_clip(&mut self) {
        self.clip.pop();
        set_scissor(self.clip.last().copied());
    }
    pub fn clip_rect(&self) -> Option<Rect> {
        self.clip.last().copied()
    }

    /// Layout debug: a widget sticking out of the screen, or partly out of the panel it is
    /// drawn on (the last region that overlaps it), outside any scroll clip.
    fn check_layout(&mut self, r: Rect) {
        let s = self.screen();
        if r.w <= 0.5 || r.h <= 0.5 {
            return;
        }
        let inside = |a: &Rect, b: &Rect| {
            b.x >= a.x - 0.5
                && b.y >= a.y - 0.5
                && b.right() <= a.right() + 0.5
                && b.bottom() <= a.bottom() + 0.5
        };
        if self.clip.last().is_some() {
            // scroll areas clip their rows; overflow there is by design
            return;
        }
        if !inside(&s, &r) && r != s {
            self.layout_issues.push((r, "widget off screen".into()));
            return;
        }
        if let Some((p, _)) = self
            .regions
            .iter()
            .rev()
            .find(|(p, _)| *p != r && *p != s && p.overlaps(&r))
            && !inside(p, &r)
        {
            self.layout_issues
                .push((r, "widget outside its panel".into()));
        }
    }

    /// Is the mouse over `r`, not blocked by a higher layer and inside the clip?
    pub fn hovering(&self, r: Rect) -> bool {
        let m = self.mouse;
        r.contains(m) && !self.blocked(m, r) && self.clip.last().is_none_or(|c| c.contains(m))
    }

    // --- animation ---------------------------------------------------------

    pub fn anim(&mut self, id: Id) -> &mut Anim {
        let frame = self.frame;
        let a = self.anims.entry(id).or_default();
        a.seen = frame;
        a
    }

    /// Ease the widget's `v` towards `target`, returns the new value.
    pub fn ease_value(&mut self, id: Id, target: f32, tau: f32) -> f32 {
        let dt = self.dt;
        let a = self.anim(id);
        a.v = approach(a.v, target, dt, tau);
        a.v
    }

    // --- interaction -------------------------------------------------------

    /// Core hit-testing for a clickable rect.
    pub fn interact(&mut self, id: Id, r: Rect) -> Response {
        self.interact_with(id, r, true)
    }

    pub(crate) fn interact_with(&mut self, id: Id, r: Rect, enabled: bool) -> Response {
        if self.layout_debug {
            self.check_layout(r);
        }
        let over = enabled && self.hovering(r);
        let free = self.active.is_none() || self.active == Some(id);
        let hovered = over && free;
        if hovered {
            self.hot = Some(id);
            self.hot_kind = CursorIcon::Pointer;
            self.hovered_any = true;
            if self.hover_id != Some(id) {
                self.hover_id = Some(id);
                self.hover_since = self.time;
            }
        }
        if over && self.pressed && self.active.is_none() {
            self.active = Some(id);
        }
        let is_active = self.active == Some(id);
        let clicked = is_active && self.released && over;
        let pressed = is_active && self.down;
        let right_clicked = over && self.rpressed && self.active.is_none();
        let dt = self.dt;
        let a = self.anim(id);
        a.hover = approach(a.hover, if hovered { 1.0 } else { 0.0 }, dt, HOVER_TAU);
        a.press = approach(a.press, if pressed { 1.0 } else { 0.0 }, dt, 0.03);
        Response {
            id: Some(id),
            rect: r,
            hovered,
            pressed,
            clicked,
            right_clicked,
            changed: false,
            hover_t: a.hover,
            press_t: a.press,
        }
    }

    pub(crate) fn set_hot_cursor(&mut self, c: CursorIcon) {
        self.hot_kind = c;
    }

    /// Show `text` as a tooltip once `resp` has been hovered for ~0.45 s.
    pub fn tooltip(&mut self, resp: &Response, text: &str) {
        if resp.hovered
            && !resp.pressed
            && self.hover_id == resp.id
            && self.time - self.hover_since > 0.45
        {
            self.tooltip = Some((text.to_string(), resp.rect));
        }
    }

    pub fn mouse_pressed(&self) -> bool {
        self.pressed
    }
    pub fn mouse_down(&self) -> bool {
        self.down
    }
    pub fn mouse_released(&self) -> bool {
        self.released
    }
    pub fn wheel(&self) -> Vec2 {
        self.wheel
    }
    pub fn is_active(&self, id: Id) -> bool {
        self.active == Some(id)
    }
    pub fn hot(&self) -> Option<Id> {
        self.hot
    }

    // --- keyboard ----------------------------------------------------------

    /// Hotkey pressed this frame and not consumed / not typing into a text field.
    pub fn key(&self, k: KeyCode) -> bool {
        self.focus.is_none() && !self.consumed.contains(&k) && is_key_pressed(k)
    }

    /// Like [`Ui::key`] but also fires on key repeat (arrows, Backspace).
    pub fn key_repeat(&self, k: KeyCode) -> bool {
        self.focus.is_none() && !self.consumed.contains(&k) && self.repeat_fire.contains(&k)
    }

    pub(crate) fn raw_repeat(&self, k: KeyCode) -> bool {
        self.repeat_fire.contains(&k)
    }

    /// Mark a key as used so later `key()` calls (e.g. game hotkeys) ignore it.
    pub fn consume_key(&mut self, k: KeyCode) {
        if !self.consumed.contains(&k) {
            self.consumed.push(k);
        }
    }

    /// Hotkey that is consumed when it fires (first caller wins).
    pub fn take_key(&mut self, k: KeyCode) -> bool {
        let hit = self.key(k);
        if hit {
            self.consume_key(k);
        }
        hit
    }

    /// Esc pressed (consumed): close the top panel, else open the menu.
    pub fn esc(&mut self) -> bool {
        self.take_key(KeyCode::Escape)
    }
    /// Enter / keypad Enter pressed (consumed).
    pub fn enter(&mut self) -> bool {
        self.take_key(KeyCode::Enter) || self.take_key(KeyCode::KpEnter)
    }
    /// Ctrl/Cmd+Z (without Shift).
    pub fn undo(&mut self) -> bool {
        self.ctrl && !self.shift && self.take_key(KeyCode::Z)
    }
    /// Ctrl/Cmd+Shift+Z or Ctrl+Y.
    pub fn redo(&mut self) -> bool {
        self.ctrl && ((self.shift && self.take_key(KeyCode::Z)) || self.take_key(KeyCode::Y))
    }
    pub fn ctrl(&self) -> bool {
        self.ctrl
    }
    pub fn shift(&self) -> bool {
        self.shift
    }
    /// Characters typed this frame (the UI drains macroquad's char queue).
    pub fn typed_chars(&self) -> &[char] {
        &self.chars
    }

    // --- sounds ------------------------------------------------------------

    pub fn sound(&mut self, s: UiSound) {
        self.sounds.push(s);
    }
    /// Sounds requested this frame. Typical: `for s in ui.drain_sounds() { audio.play(map(s)) }`.
    pub fn drain_sounds(&mut self) -> Vec<UiSound> {
        std::mem::take(&mut self.sounds)
    }

    // --- tooltip overlay ---------------------------------------------------

    fn draw_tooltip(&mut self) {
        let Some((text, anchor)) = self.tooltip.take() else {
            return;
        };
        let t = &self.theme;
        let style = t.small().color(t.text);
        let max_w = 260.0;
        let lines = self.fonts.wrap(&text, max_w, &style);
        let w = lines
            .iter()
            .map(|l| self.fonts.measure(l, &style))
            .fold(0.0, f32::max)
            + 18.0;
        let h = lines.len() as f32 * style.line_h() + 12.0;
        let mut x = anchor.x + (anchor.w - w) / 2.0;
        x = x.clamp(6.0, self.screen.x - w - 6.0);
        let mut y = anchor.bottom() + 6.0;
        if y + h > self.screen.y - 6.0 {
            y = anchor.y - h - 6.0;
        }
        let r = Rect::new(x, y, w, h);
        super::glass::draw_glass(&self.theme, r, &super::glass::Glass::tooltip());
        self.fonts.draw_lines(
            &lines,
            r.inset_xy(9.0, 6.0),
            &style,
            super::text::Align::Left,
            None,
            usize::MAX,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approach_converges() {
        let mut v = 0.0;
        for _ in 0..60 {
            v = approach(v, 1.0, 1.0 / 60.0, 0.05);
        }
        assert_eq!(v, 1.0);
        assert_eq!(approach(0.3, 0.7, 0.016, 0.0), 0.7);
        let half = approach(0.0, 1.0, 0.05, 0.05);
        assert!((half - (1.0 - (-1.0f32).exp())).abs() < 1e-4);
    }

    #[test]
    fn easing_bounds() {
        assert_eq!(ease(0.0), 0.0);
        assert_eq!(ease(1.0), 1.0);
        assert_eq!(ease_out(2.0), 1.0);
        assert!((ease(0.5) - 0.5).abs() < 1e-6);
    }
}
