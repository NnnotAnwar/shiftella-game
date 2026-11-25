//! The in-game **Shiftella** app on a phone / tablet: device frame, status bar,
//! glass app header and tab bar. Screens are drawn by the caller inside `content`.
//!
//! ```ignore
//! let out = ui.phone_frame("phone", rect, &PhoneFrame::new("03:33", &tabs).selected(0));
//! ui.push_clip(out.content); /* schedule grid … */ ui.pop_clip();
//! ```

use std::hash::Hash;

use macroquad::prelude::*;

use super::context::{Ui, UiSound};
use super::draw::{
    self, Paint, clip_convex, fill_convex, fill_rrect, mix, rounded_rect_path, shadow_rrect,
    stroke_rrect, with_a,
};
use super::glass::{Elevation, Glass, draw_glass};
use super::icons::{Icon, draw_icon};
use super::layout::RectExt;
use super::text::{Align, TextStyle, Weight};
use super::widgets::IconButton;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Device {
    Phone,
    Tablet,
}

impl Device {
    /// Device size for a given height (phone 9:19.5, tablet landscape 4:3).
    pub fn size_for_height(self, h: f32) -> Vec2 {
        match self {
            Device::Phone => vec2((h * 9.0 / 19.5).round(), h),
            Device::Tablet => vec2((h * 4.0 / 3.0).round(), h),
        }
    }
    fn radius(self) -> f32 {
        match self {
            Device::Phone => 40.0,
            Device::Tablet => 26.0,
        }
    }
    fn bezel(self) -> f32 {
        match self {
            Device::Phone => 9.0,
            Device::Tablet => 12.0,
        }
    }
}

/// Phone frame description.
#[derive(Clone, Copy, Debug)]
pub struct PhoneFrame<'a> {
    pub device: Device,
    /// Status bar clock, e.g. "03:33".
    pub time: &'a str,
    /// 0..1
    pub battery: f32,
    /// 0..4 bars
    pub signal: u8,
    pub title: &'a str,
    pub subtitle: Option<&'a str>,
    pub tabs: &'a [(Icon, &'a str)],
    pub selected_tab: usize,
    /// Unread count on the bell.
    pub badge: u32,
    /// Show a back chevron instead of the logo.
    pub back: bool,
    /// Signed-in user (avatar in the header).
    pub user: Option<&'a str>,
}

impl<'a> PhoneFrame<'a> {
    pub fn new(time: &'a str, tabs: &'a [(Icon, &'a str)]) -> Self {
        PhoneFrame {
            device: Device::Phone,
            time,
            battery: 0.72,
            signal: 3,
            title: "Shiftella",
            subtitle: None,
            tabs,
            selected_tab: 0,
            badge: 0,
            back: false,
            user: None,
        }
    }
    pub fn tablet(mut self) -> Self {
        self.device = Device::Tablet;
        self
    }
    pub fn title(mut self, t: &'a str) -> Self {
        self.title = t;
        self
    }
    pub fn subtitle(mut self, s: &'a str) -> Self {
        self.subtitle = Some(s);
        self
    }
    pub fn selected(mut self, i: usize) -> Self {
        self.selected_tab = i;
        self
    }
    pub fn badge(mut self, n: u32) -> Self {
        self.badge = n;
        self
    }
    pub fn battery(mut self, b: f32) -> Self {
        self.battery = b;
        self
    }
    pub fn back(mut self, b: bool) -> Self {
        self.back = b;
        self
    }
    pub fn user(mut self, name: &'a str) -> Self {
        self.user = Some(name);
        self
    }
}

/// What the frame reports back.
#[derive(Clone, Copy, Debug)]
pub struct PhoneOut {
    /// The whole glass screen.
    pub screen: Rect,
    /// Free area between header and tab bar — draw app screens here.
    pub content: Rect,
    pub tab_clicked: Option<usize>,
    pub bell_clicked: bool,
    pub back_clicked: bool,
}

impl Ui {
    pub fn phone_frame(&mut self, key: impl Hash, r: Rect, f: &PhoneFrame) -> PhoneOut {
        let id = self.id(key);
        self.push_id(id);
        let t = self.theme.clone();
        let dev = f.device;
        let rad = dev.radius();
        self.region(r);

        // body: dark titanium with a lit rim
        shadow_rrect(r, rad, t.shadow_ambient);
        shadow_rrect(
            r,
            rad,
            draw::Shadow::new(0.0, 18.0, 46.0, 0.0, Color::new(0.02, 0.01, 0.06, 0.6)),
        );
        shadow_rrect(
            r,
            rad,
            draw::Shadow::new(0.0, 0.0, 60.0, -10.0, with_a(t.indigo, 0.10)),
        );
        fill_rrect(
            r,
            rad,
            Paint::vertical(r, Color::from_hex(0x34314F), Color::from_hex(0x15131F)),
        );
        stroke_rrect(
            r,
            rad,
            1.0,
            Paint::vertical(
                r,
                Color::new(1.0, 1.0, 1.0, 0.38),
                Color::new(1.0, 1.0, 1.0, 0.06),
            ),
        );
        let glass_r = r.inset(2.5);
        fill_rrect(glass_r, rad - 2.5, Color::from_hex(0x050409));

        // screen
        let b = dev.bezel();
        let screen = r.inset(b);
        let srad = rad - b;
        fill_rrect(
            screen,
            srad,
            Paint::vertical(screen, t.violet_night[0], t.violet_night[1]),
        );
        corner_glow(screen, srad, with_a(t.indigo, 0.30), true);
        corner_glow(screen, srad, with_a(t.violet, 0.16), false);

        // status bar
        let sb_h = if dev == Device::Phone { 34.0 } else { 24.0 };
        let sb = Rect::new(screen.x, screen.y, screen.w, sb_h);
        let time_st = TextStyle::new(12.0, Weight::SemiBold, t.text).tabular();
        let pad = if dev == Device::Phone { 26.0 } else { 18.0 };
        let base = self.fonts.baseline_in(
            sb.translate(vec2(0.0, if dev == Device::Phone { 2.0 } else { 0.0 })),
            &time_st,
        );
        self.fonts.draw(f.time, sb.x + pad, base, &time_st);
        match dev {
            Device::Phone => {
                let island =
                    Rect::new(screen.x + screen.w / 2.0 - 38.0, screen.y + 8.0, 76.0, 20.0);
                fill_rrect(island, 10.0, BLACK);
                fill_rrect(
                    Rect::new(island.right() - 16.0, island.y + 6.0, 8.0, 8.0),
                    4.0,
                    Color::from_hex(0x111827),
                );
            }
            Device::Tablet => {
                fill_rrect(
                    Rect::new(r.x + r.w / 2.0 - 3.0, r.y + b / 2.0 - 3.0, 6.0, 6.0),
                    3.0,
                    Color::from_hex(0x0B0A14),
                );
            }
        }
        let mut sx = sb.right() - pad;
        let cy = base - 4.5;
        // battery
        let bat = Rect::new(sx - 22.0, cy - 5.5, 20.0, 11.0);
        stroke_rrect(bat, 3.0, 1.0, Color::new(1.0, 1.0, 1.0, 0.55));
        fill_rrect(
            Rect::new(bat.right() + 1.0, cy - 2.0, 1.5, 4.0),
            0.75,
            Color::new(1.0, 1.0, 1.0, 0.55),
        );
        let lvl = f.battery.clamp(0.0, 1.0);
        let fillc = if lvl < 0.2 { t.red_400 } else { t.text };
        fill_rrect(
            Rect::new(bat.x + 2.0, bat.y + 2.0, (bat.w - 4.0) * lvl, bat.h - 4.0),
            1.5,
            fillc,
        );
        sx = bat.x - 7.0;
        // signal bars
        for i in 0..4u8 {
            let h = 4.0 + i as f32 * 2.0;
            let x = sx - (3 - i) as f32 * 4.5 - 3.0;
            let on = i < f.signal;
            fill_rrect(
                Rect::new(x, cy + 5.0 - h, 3.0, h),
                1.0,
                if on {
                    t.text
                } else {
                    Color::new(1.0, 1.0, 1.0, 0.25)
                },
            );
        }

        // app header
        let hp = if dev == Device::Phone { 10.0 } else { 12.0 };
        let head_h = if f.subtitle.is_some() { 50.0 } else { 44.0 };
        let head = Rect::new(
            screen.x + hp,
            sb.bottom() + 2.0,
            screen.w - 2.0 * hp,
            head_h,
        );
        draw_glass(
            &t,
            head,
            &Glass::hud().radius(14.0).elevation(Elevation::Low),
        );
        let mut out = PhoneOut {
            screen,
            content: Rect::default(),
            tab_clicked: None,
            bell_clicked: false,
            back_clicked: false,
        };
        let mut x = head.x + 10.0;
        let hcy = head.center_y();
        if f.back {
            let br = Rect::new(x - 2.0, hcy - 14.0, 28.0, 28.0);
            out.back_clicked = self
                .icon_button("back", br, Icon::Back, &IconButton::ghost().size(18.0))
                .clicked;
            x += 30.0;
        } else {
            // logo mark: indigo→violet squircle with a white "S"
            let lr = Rect::new(x, hcy - 13.0, 26.0, 26.0);
            shadow_rrect(
                lr,
                8.0,
                draw::Shadow::new(0.0, 2.0, 10.0, 0.0, with_a(t.indigo, 0.55)),
            );
            fill_rrect(lr, 8.0, Paint::diagonal(lr, t.indigo_400, t.violet));
            stroke_rrect(
                lr,
                8.0,
                1.0,
                Paint::vertical(
                    lr,
                    Color::new(1.0, 1.0, 1.0, 0.45),
                    Color::new(1.0, 1.0, 1.0, 0.0),
                ),
            );
            let s_st = TextStyle::new(15.0, Weight::Bold, WHITE);
            self.fonts
                .draw_in("S", lr.translate(vec2(0.3, 0.0)), &s_st, Align::Center);
            x += 34.0;
        }
        let title_st = t.h1().size(15.0);
        let right_reserve = 34.0 + if f.user.is_some() { 32.0 } else { 0.0 };
        let tw = (head.right() - right_reserve - x).max(0.0);
        match f.subtitle {
            Some(s) => {
                self.fonts.draw_in(
                    f.title,
                    Rect::new(x, hcy - 17.0, tw, 18.0),
                    &title_st,
                    Align::Left,
                );
                self.fonts.draw_in(
                    s,
                    Rect::new(x, hcy + 1.0, tw, 16.0),
                    &t.small(),
                    Align::Left,
                );
            }
            None => {
                self.fonts.draw_in(
                    f.title,
                    Rect::new(x, head.y, tw, head.h),
                    &title_st,
                    Align::Left,
                );
            }
        }
        let mut rx = head.right() - 8.0;
        if let Some(u) = f.user {
            self.avatar(vec2(rx - 12.0, hcy), 24.0, u, None, false);
            rx -= 32.0;
        }
        let bell = Rect::new(rx - 28.0, hcy - 14.0, 28.0, 28.0);
        let mut ib = IconButton::ghost().size(18.0);
        if f.badge > 0 {
            ib = ib.badge(f.badge);
        }
        out.bell_clicked = self.icon_button("bell", bell, Icon::Bell, &ib).clicked;

        // tab bar
        let tb_h = if dev == Device::Phone { 54.0 } else { 50.0 };
        let home_space = if dev == Device::Phone { 16.0 } else { 8.0 };
        let tb = Rect::new(
            screen.x + hp,
            screen.bottom() - home_space - tb_h - 4.0,
            screen.w - 2.0 * hp,
            tb_h,
        );
        if !f.tabs.is_empty() {
            draw_glass(&t, tb, &Glass::hud().radius(18.0).elevation(Elevation::Low));
            let n = f.tabs.len();
            let cw = (tb.w - 8.0) / n as f32;
            for (i, (icon, label)) in f.tabs.iter().enumerate() {
                let cell = Rect::new(tb.x + 4.0 + i as f32 * cw, tb.y + 4.0, cw, tb.h - 8.0);
                let resp = self.interact(id.with(("tab", i)), cell);
                let on = i == f.selected_tab;
                if resp.clicked && !on {
                    out.tab_clicked = Some(i);
                    self.sound(UiSound::Click);
                }
                let k = self.ease_value(id.with(("tab_on", i)), if on { 1.0 } else { 0.0 }, 0.06);
                if k > 0.01 {
                    let pill = Rect::new(cell.x + cell.w / 2.0 - 22.0, cell.y + 2.0, 44.0, 24.0);
                    draw::push_alpha(k);
                    shadow_rrect(
                        pill,
                        12.0,
                        draw::Shadow::new(0.0, 0.0, 14.0, 0.0, with_a(t.indigo, 0.45)),
                    );
                    fill_rrect(
                        pill,
                        12.0,
                        Paint::diagonal(pill, with_a(t.indigo, 0.42), with_a(t.violet, 0.30)),
                    );
                    stroke_rrect(pill, 12.0, 1.0, Color::new(1.0, 1.0, 1.0, 0.12));
                    draw::pop_alpha();
                }
                let fg = if on {
                    WHITE
                } else {
                    mix(t.text_dim, t.text, resp.hover_t)
                };
                draw_icon(
                    *icon,
                    vec2(cell.x + cell.w / 2.0, cell.y + 14.0),
                    17.0,
                    if on { t.indigo_300 } else { fg },
                );
                let st =
                    TextStyle::new(11.0, if on { Weight::SemiBold } else { Weight::Medium }, fg);
                self.fonts.draw_in(
                    label,
                    Rect::new(cell.x + 2.0, cell.y + 28.0, cell.w - 4.0, 14.0),
                    &st,
                    Align::Center,
                );
            }
        }
        if dev == Device::Phone {
            let hi = Rect::new(
                screen.x + screen.w / 2.0 - 54.0,
                screen.bottom() - 9.0,
                108.0,
                4.0,
            );
            fill_rrect(hi, 2.0, Color::new(1.0, 1.0, 1.0, 0.55));
        }
        let top = head.bottom() + 8.0;
        let bottom = if f.tabs.is_empty() {
            screen.bottom() - home_space - 4.0
        } else {
            tb.y - 8.0
        };
        out.content = Rect::new(
            screen.x + hp + 2.0,
            top,
            screen.w - 2.0 * (hp + 2.0),
            (bottom - top).max(0.0),
        );
        self.pop_id();
        out
    }
}

/// Soft colour wash from the top-right (or bottom-left) corner of a rounded screen.
fn corner_glow(r: Rect, rad: f32, c: Color, top_right: bool) {
    let path = rounded_rect_path(r, rad);
    let reach = r.w.min(r.h) * 0.95;
    let (corner, dir) = if top_right {
        (vec2(r.right(), r.y), vec2(-1.0, 1.0).normalize())
    } else {
        (vec2(r.x, r.bottom()), vec2(1.0, -1.0).normalize())
    };
    let line_p = corner + dir * reach;
    let region = clip_convex(&path, line_p, dir);
    if region.len() < 3 {
        return;
    }
    fill_convex(
        &region,
        Paint::Linear {
            from: corner,
            to: line_p,
            c0: c,
            c1: with_a(c, 0.0),
        },
    );
}
