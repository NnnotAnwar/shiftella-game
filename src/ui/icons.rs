//! Vector icon set: 24 pt grid, 2 pt stroke (min 1.45 pt), round caps & joins.
//! Drawn live with strokes, so every size is crisp. Colour = tint (use opaque colours).
//!
//! ```ignore
//! draw_icon(Icon::Coffee, vec2(40.0, 30.0), 18.0, theme.amber);
//! ```

use std::f32::consts::PI;

use macroquad::prelude::*;

use super::draw::{Paint, fill_convex, stroke_path, stroke_round};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Icon {
    Pause,
    Play,
    Fast,
    Faster,
    Skip,
    Crew,
    Priorities,
    Calendar,
    Build,
    Log,
    Settings,
    Coffee,
    Money,
    Star,
    Moon,
    Sun,
    Train,
    Rain,
    Bell,
    Check,
    Close,
    Undo,
    Back,
    Plus,
    Minus,
    Globe,
    Volume,
    Mute,
    Phone,
    Note,
    Heart,
    Warning,
    Info,
    Lock,
    User,
    Clock,
    Chart,
    Home,
    Door,
    Broom,
    ChefHat,
    Cup,
    CashRegister,
    Swap,
    Plane,
    Megaphone,
    ClipboardCheck,
    FileExport,
    ChevronDown,
    ChevronUp,
    ChevronRight,
    Search,
    Sparkle,
    Flame,
    Snowflake,
    Candy,
    Bean,
    Bolt,
    Feather,
    Gem,
    Stopwatch,
    Bowl,
    Anise,
    Leaf,
    Tray,
    Dishes,
    Vinyl,
    Repeat,
    Book,
    Camera,
    Cloud,
    CloudSun,
    LinkBroken,
    Forward,
    Jar,
    Flask,
    Sunrise,
    ChevronLeft,
}

impl Icon {
    pub const ALL: [Icon; 78] = [
        Icon::Pause,
        Icon::Play,
        Icon::Fast,
        Icon::Faster,
        Icon::Skip,
        Icon::Crew,
        Icon::Priorities,
        Icon::Calendar,
        Icon::Build,
        Icon::Log,
        Icon::Settings,
        Icon::Coffee,
        Icon::Money,
        Icon::Star,
        Icon::Moon,
        Icon::Sun,
        Icon::Train,
        Icon::Rain,
        Icon::Bell,
        Icon::Check,
        Icon::Close,
        Icon::Undo,
        Icon::Back,
        Icon::Plus,
        Icon::Minus,
        Icon::Globe,
        Icon::Volume,
        Icon::Mute,
        Icon::Phone,
        Icon::Note,
        Icon::Heart,
        Icon::Warning,
        Icon::Info,
        Icon::Lock,
        Icon::User,
        Icon::Clock,
        Icon::Chart,
        Icon::Home,
        Icon::Door,
        Icon::Broom,
        Icon::ChefHat,
        Icon::Cup,
        Icon::CashRegister,
        Icon::Swap,
        Icon::Plane,
        Icon::Megaphone,
        Icon::ClipboardCheck,
        Icon::FileExport,
        Icon::ChevronDown,
        Icon::ChevronUp,
        Icon::ChevronRight,
        Icon::Search,
        Icon::Sparkle,
        Icon::Flame,
        Icon::Snowflake,
        Icon::Candy,
        Icon::Bean,
        Icon::Bolt,
        Icon::Feather,
        Icon::Gem,
        Icon::Stopwatch,
        Icon::Bowl,
        Icon::Anise,
        Icon::Leaf,
        Icon::Tray,
        Icon::Dishes,
        Icon::Vinyl,
        Icon::Repeat,
        Icon::Book,
        Icon::Camera,
        Icon::Cloud,
        Icon::CloudSun,
        Icon::LinkBroken,
        Icon::Forward,
        Icon::Jar,
        Icon::Flask,
        Icon::Sunrise,
        Icon::ChevronLeft,
    ];

    /// Stable snake_case name (for data files / debugging).
    pub fn name(self) -> &'static str {
        use Icon::*;
        match self {
            Pause => "pause",
            Play => "play",
            Fast => "fast",
            Faster => "faster",
            Skip => "skip",
            Crew => "crew",
            Priorities => "priorities",
            Calendar => "calendar",
            Build => "build",
            Log => "log",
            Settings => "settings",
            Coffee => "coffee",
            Money => "money",
            Star => "star",
            Moon => "moon",
            Sun => "sun",
            Train => "train",
            Rain => "rain",
            Bell => "bell",
            Check => "check",
            Close => "close",
            Undo => "undo",
            Back => "back",
            Plus => "plus",
            Minus => "minus",
            Globe => "globe",
            Volume => "volume",
            Mute => "mute",
            Phone => "phone",
            Note => "note",
            Heart => "heart",
            Warning => "warning",
            Info => "info",
            Lock => "lock",
            User => "user",
            Clock => "clock",
            Chart => "chart",
            Home => "home",
            Door => "door",
            Broom => "broom",
            ChefHat => "chef_hat",
            Cup => "cup",
            CashRegister => "cash_register",
            Swap => "swap",
            Plane => "plane",
            Megaphone => "megaphone",
            ClipboardCheck => "clipboard_check",
            FileExport => "file_export",
            ChevronDown => "chevron_down",
            ChevronUp => "chevron_up",
            ChevronRight => "chevron_right",
            Search => "search",
            Sparkle => "sparkle",
            Flame => "flame",
            Snowflake => "snowflake",
            Candy => "candy",
            Bean => "bean",
            Bolt => "bolt",
            Feather => "feather",
            Gem => "gem",
            Stopwatch => "stopwatch",
            Bowl => "bowl",
            Anise => "anise",
            Leaf => "leaf",
            Tray => "tray",
            Dishes => "dishes",
            Vinyl => "vinyl",
            Repeat => "repeat",
            Book => "book",
            Camera => "camera",
            Cloud => "cloud",
            CloudSun => "cloud_sun",
            LinkBroken => "link_broken",
            Forward => "forward",
            Jar => "jar",
            Flask => "flask",
            Sunrise => "sunrise",
            ChevronLeft => "chevron_left",
        }
    }

    pub fn from_name(name: &str) -> Option<Icon> {
        Icon::ALL.iter().copied().find(|i| i.name() == name)
    }
}

type P = (f32, f32);

/// Maps 24-grid coordinates to the screen and strokes.
struct Pen {
    o: Vec2,
    s: f32,
    w: f32,
    c: Color,
    rot: Option<(f32, f32)>,
}

impl Pen {
    fn p(&self, (x, y): P) -> Vec2 {
        let (x, y) = match self.rot {
            Some((cs, sn)) => {
                let (dx, dy) = (x - 12.0, y - 12.0);
                (12.0 + dx * cs - dy * sn, 12.0 + dx * sn + dy * cs)
            }
            None => (x, y),
        };
        self.o + vec2(x, y) * self.s
    }
    fn map(&self, pts: &[P]) -> Vec<Vec2> {
        pts.iter().map(|&q| self.p(q)).collect()
    }
    fn line(&self, a: P, b: P) {
        stroke_path(&[self.p(a), self.p(b)], false, self.w, Paint::Solid(self.c));
    }
    fn poly(&self, pts: &[P], closed: bool) {
        stroke_round(&self.map(pts), closed, self.w, self.c);
    }
    fn arc_pts(cx: f32, cy: f32, r: f32, a0: f32, a1: f32) -> Vec<P> {
        let n = (((a1 - a0).abs() / 360.0) * 44.0).ceil().max(4.0) as usize;
        (0..=n)
            .map(|i| {
                let a = (a0 + (a1 - a0) * i as f32 / n as f32).to_radians();
                (cx + a.cos() * r, cy + a.sin() * r)
            })
            .collect()
    }
    fn arc(&self, cx: f32, cy: f32, r: f32, a0: f32, a1: f32) {
        let pts = Self::arc_pts(cx, cy, r, a0, a1);
        stroke_path(&self.map(&pts), false, self.w, Paint::Solid(self.c));
    }
    fn circle(&self, cx: f32, cy: f32, r: f32) {
        let mut pts = Self::arc_pts(cx, cy, r, 0.0, 360.0);
        pts.pop();
        stroke_path(&self.map(&pts), true, self.w, Paint::Solid(self.c));
    }
    fn ellipse(&self, cx: f32, cy: f32, rx: f32, ry: f32) {
        let pts: Vec<P> = (0..40)
            .map(|i| {
                let a = i as f32 / 40.0 * 2.0 * PI;
                (cx + a.cos() * rx, cy + a.sin() * ry)
            })
            .collect();
        stroke_path(&self.map(&pts), true, self.w, Paint::Solid(self.c));
    }
    fn rrect(&self, x: f32, y: f32, w: f32, h: f32, r: f32) {
        let mut pts = Vec::new();
        pts.extend(Self::arc_pts(x + r, y + r, r, 180.0, 270.0));
        pts.extend(Self::arc_pts(x + w - r, y + r, r, 270.0, 360.0));
        pts.extend(Self::arc_pts(x + w - r, y + h - r, r, 0.0, 90.0));
        pts.extend(Self::arc_pts(x + r, y + h - r, r, 90.0, 180.0));
        stroke_path(&self.map(&pts), true, self.w, Paint::Solid(self.c));
    }
    fn dot(&self, x: f32, y: f32, r: f32) {
        let pts: Vec<Vec2> = Self::arc_pts(x, y, r, 0.0, 360.0)
            .into_iter()
            .take(44)
            .map(|q| self.p(q))
            .collect();
        fill_convex(&pts, Paint::Solid(self.c));
    }
    fn tri(&self, a: P, b: P, c: P) {
        self.poly(&[a, b, c], true);
    }
}

/// Draw `icon` centred at `center`, `size` pt square.
pub fn draw_icon(icon: Icon, center: Vec2, size: f32, color: Color) {
    let s = size / 24.0;
    let mut pen = Pen {
        o: center - vec2(12.0, 12.0) * s,
        s,
        w: (2.0 * s).max(1.45),
        c: color,
        rot: None,
    };
    use Icon::*;
    match icon {
        Pause => {
            pen.line((9.0, 6.0), (9.0, 18.0));
            pen.line((15.0, 6.0), (15.0, 18.0));
        }
        Play => pen.tri((8.0, 5.5), (18.5, 12.0), (8.0, 18.5)),
        Fast => {
            pen.tri((3.5, 6.5), (11.5, 12.0), (3.5, 17.5));
            pen.tri((12.5, 6.5), (20.5, 12.0), (12.5, 17.5));
        }
        Faster => {
            for x in [2.5, 9.0, 15.5] {
                pen.tri((x, 7.5), (x + 6.0, 12.0), (x, 16.5));
            }
        }
        Skip => {
            pen.tri((4.0, 6.5), (11.0, 12.0), (4.0, 17.5));
            pen.tri((11.5, 6.5), (18.5, 12.0), (11.5, 17.5));
            pen.line((20.5, 6.0), (20.5, 18.0));
        }
        Crew => {
            pen.circle(9.0, 8.5, 3.3);
            pen.arc(9.0, 20.0, 6.0, 180.0, 360.0);
            pen.arc(16.5, 8.5, 2.7, 250.0, 450.0);
            pen.arc(16.5, 19.0, 4.8, 270.0, 360.0);
        }
        Priorities => {
            pen.line((9.0, 6.5), (20.0, 6.5));
            pen.line((9.0, 12.0), (17.0, 12.0));
            pen.line((9.0, 17.5), (13.5, 17.5));
            pen.dot(4.5, 6.5, 1.4);
            pen.dot(4.5, 12.0, 1.4);
            pen.dot(4.5, 17.5, 1.4);
        }
        Calendar => {
            pen.rrect(3.5, 5.0, 17.0, 15.5, 3.0);
            pen.line((3.5, 10.0), (20.5, 10.0));
            pen.line((8.5, 3.0), (8.5, 6.5));
            pen.line((15.5, 3.0), (15.5, 6.5));
            pen.dot(8.5, 14.5, 1.1);
            pen.dot(12.0, 14.5, 1.1);
            pen.dot(15.5, 14.5, 1.1);
        }
        Build => {
            pen.line((4.5, 19.5), (12.0, 12.0));
            pen.poly(&[(11.9, 5.0), (19.0, 12.1), (16.1, 15.0), (9.0, 7.9)], true);
        }
        Log => {
            pen.rrect(4.0, 3.5, 16.0, 17.0, 3.0);
            pen.line((8.0, 8.5), (16.0, 8.5));
            pen.line((8.0, 12.0), (16.0, 12.0));
            pen.line((8.0, 15.5), (12.5, 15.5));
        }
        Settings => {
            let mut pts = Vec::new();
            for k in 0..8 {
                let a = k as f32 * 45.0;
                for (da, r) in [(-24.0, 6.6), (-11.0, 9.0), (11.0, 9.0), (24.0, 6.6)] {
                    let t = (a + da).to_radians();
                    pts.push((12.0 + t.cos() * r, 12.0 + t.sin() * r));
                }
            }
            pen.poly(&pts, true);
            pen.circle(12.0, 12.0, 2.8);
        }
        Coffee => {
            let mut body = vec![(5.0, 9.5), (5.0, 15.0)];
            body.extend(Pen::arc_pts(9.0, 15.0, 4.0, 180.0, 90.0));
            body.extend(Pen::arc_pts(13.0, 15.0, 4.0, 90.0, 0.0));
            body.push((17.0, 9.5));
            pen.poly(&body, true);
            pen.arc(17.0, 13.0, 2.9, -90.0, 90.0);
            pen.line((9.0, 3.5), (9.0, 6.0));
            pen.line((13.0, 3.5), (13.0, 6.0));
        }
        Money => {
            pen.rrect(2.5, 6.0, 19.0, 12.0, 2.5);
            pen.circle(12.0, 12.0, 2.8);
            pen.dot(6.3, 12.0, 1.0);
            pen.dot(17.7, 12.0, 1.0);
        }
        Star => {
            let pts: Vec<P> = (0..10)
                .map(|i| {
                    let r = if i % 2 == 0 { 9.5 } else { 4.4 };
                    let a = (-90.0 + i as f32 * 36.0_f32).to_radians();
                    (12.0 + a.cos() * r, 12.8 + a.sin() * r)
                })
                .collect();
            pen.poly(&pts, true);
        }
        Moon => {
            let mut pts = Pen::arc_pts(16.5, 7.5, 6.36, 225.0, 45.0);
            pts.extend(Pen::arc_pts(12.0, 12.0, 9.0, 0.0, 270.0));
            pen.rot = Some(((-0.25_f32).cos(), (-0.25_f32).sin()));
            pen.poly(&pts, true);
        }
        Sun => {
            pen.circle(12.0, 12.0, 4.0);
            for k in 0..8 {
                let a = (k as f32 * 45.0_f32).to_radians();
                let (c, s2) = (a.cos(), a.sin());
                pen.line(
                    (12.0 + c * 7.2, 12.0 + s2 * 7.2),
                    (12.0 + c * 9.6, 12.0 + s2 * 9.6),
                );
            }
        }
        Train => {
            pen.rrect(5.0, 3.0, 14.0, 14.5, 3.5);
            pen.line((5.0, 10.0), (19.0, 10.0));
            pen.line((12.0, 3.0), (12.0, 10.0));
            pen.dot(8.7, 13.8, 1.1);
            pen.dot(15.3, 13.8, 1.1);
            pen.line((8.0, 17.5), (6.0, 21.0));
            pen.line((16.0, 17.5), (18.0, 21.0));
        }
        Rain => {
            // open-bottom cloud: big arc → flat top-right → small right bump
            let mut c = Pen::arc_pts(9.0, 10.0, 7.0, 135.6, 343.4);
            c.push((17.5, 8.0));
            c.extend(Pen::arc_pts(17.5, 12.5, 4.5, 270.0, 416.0));
            pen.poly(&c, false);
            pen.line((8.0, 15.0), (8.0, 20.0));
            pen.line((12.0, 17.0), (12.0, 22.0));
            pen.line((16.0, 15.0), (16.0, 20.0));
        }
        Bell => {
            let mut b = vec![(4.0, 17.5), (6.0, 14.5)];
            b.extend(Pen::arc_pts(12.0, 10.0, 6.0, 180.0, 360.0));
            b.extend([(18.0, 14.5), (20.0, 17.5)]);
            pen.poly(&b, true);
            pen.arc(12.0, 18.0, 2.4, 20.0, 160.0);
        }
        Check => pen.poly(&[(5.0, 12.5), (10.0, 17.5), (19.0, 7.0)], false),
        Close => {
            pen.line((6.5, 6.5), (17.5, 17.5));
            pen.line((17.5, 6.5), (6.5, 17.5));
        }
        Undo => {
            let mut p = vec![(5.0, 9.5), (14.0, 9.5)];
            p.extend(Pen::arc_pts(14.0, 14.0, 4.5, -90.0, 90.0));
            p.push((9.0, 18.5));
            pen.poly(&p, false);
            pen.poly(&[(8.5, 6.0), (5.0, 9.5), (8.5, 13.0)], false);
        }
        Back => {
            pen.line((19.0, 12.0), (5.0, 12.0));
            pen.poly(&[(11.0, 6.0), (5.0, 12.0), (11.0, 18.0)], false);
        }
        Plus => {
            pen.line((12.0, 5.0), (12.0, 19.0));
            pen.line((5.0, 12.0), (19.0, 12.0));
        }
        Minus => pen.line((5.0, 12.0), (19.0, 12.0)),
        Globe => {
            pen.circle(12.0, 12.0, 9.0);
            pen.ellipse(12.0, 12.0, 4.0, 9.0);
            pen.line((3.0, 12.0), (21.0, 12.0));
        }
        Volume | Mute => {
            pen.poly(
                &[
                    (3.5, 9.5),
                    (7.5, 9.5),
                    (12.5, 5.0),
                    (12.5, 19.0),
                    (7.5, 14.5),
                    (3.5, 14.5),
                ],
                true,
            );
            if icon == Volume {
                pen.arc(12.5, 12.0, 4.0, -45.0, 45.0);
                pen.arc(12.5, 12.0, 7.8, -48.0, 48.0);
            } else {
                pen.line((16.0, 9.5), (21.0, 14.5));
                pen.line((21.0, 9.5), (16.0, 14.5));
            }
        }
        Phone => {
            pen.rrect(6.5, 2.5, 11.0, 19.0, 2.8);
            pen.line((10.5, 18.0), (13.5, 18.0));
        }
        Note => {
            pen.poly(
                &[
                    (5.0, 3.5),
                    (19.0, 3.5),
                    (19.0, 15.0),
                    (14.5, 20.5),
                    (5.0, 20.5),
                ],
                true,
            );
            pen.poly(&[(19.0, 15.0), (14.5, 15.0), (14.5, 20.5)], false);
            pen.line((8.5, 8.5), (15.5, 8.5));
            pen.line((8.5, 12.0), (13.0, 12.0));
        }
        Heart => {
            let pts: Vec<P> = (0..56)
                .map(|i| {
                    let t = i as f32 / 56.0 * 2.0 * PI;
                    let x = 16.0 * t.sin().powi(3);
                    let y = 13.0 * t.cos()
                        - 5.0 * (2.0 * t).cos()
                        - 2.0 * (3.0 * t).cos()
                        - (4.0 * t).cos();
                    (12.0 + x * 0.55, 12.2 - (y + 2.5) * 0.55)
                })
                .collect();
            pen.poly(&pts, true);
        }
        Warning => {
            pen.tri((12.0, 3.5), (21.0, 19.5), (3.0, 19.5));
            pen.line((12.0, 9.5), (12.0, 13.5));
            pen.dot(12.0, 16.6, 1.2);
        }
        Info => {
            pen.circle(12.0, 12.0, 9.0);
            pen.line((12.0, 11.0), (12.0, 16.5));
            pen.dot(12.0, 7.8, 1.2);
        }
        Lock => {
            pen.rrect(5.0, 11.0, 14.0, 10.0, 2.5);
            pen.line((8.0, 11.0), (8.0, 8.0));
            pen.line((16.0, 11.0), (16.0, 8.0));
            pen.arc(12.0, 8.0, 4.0, 180.0, 360.0);
            pen.dot(12.0, 16.0, 1.3);
        }
        User => {
            pen.circle(12.0, 8.0, 4.0);
            pen.arc(12.0, 21.0, 7.5, 180.0, 360.0);
        }
        Clock => {
            pen.circle(12.0, 12.0, 9.0);
            pen.poly(&[(12.0, 7.0), (12.0, 12.0), (15.5, 14.0)], false);
        }
        Chart => {
            pen.poly(&[(4.0, 4.0), (4.0, 20.0), (20.0, 20.0)], false);
            pen.line((8.5, 16.0), (8.5, 12.5));
            pen.line((13.0, 16.0), (13.0, 8.0));
            pen.line((17.5, 16.0), (17.5, 11.0));
        }
        Home => {
            pen.poly(&[(3.0, 11.0), (12.0, 3.5), (21.0, 11.0)], false);
            pen.poly(&[(5.5, 9.5), (5.5, 20.0), (18.5, 20.0), (18.5, 9.5)], false);
            pen.poly(
                &[(10.0, 20.0), (10.0, 14.5), (14.0, 14.5), (14.0, 20.0)],
                false,
            );
        }
        Door => {
            pen.line((3.5, 20.5), (20.5, 20.5));
            pen.poly(&[(6.5, 20.5), (6.5, 3.5), (17.5, 3.5), (17.5, 20.5)], false);
            pen.dot(14.3, 12.5, 1.2);
        }
        Broom => {
            pen.line((20.0, 3.0), (13.0, 11.0));
            pen.poly(
                &[(10.0, 9.5), (15.5, 14.0), (12.0, 21.0), (3.0, 15.5)],
                true,
            );
            pen.line((7.5, 18.0), (10.5, 13.5));
        }
        ChefHat => {
            let mut h = vec![(7.0, 20.0), (7.0, 13.0)];
            h.extend(Pen::arc_pts(7.8, 9.5, 3.6, 105.0, 255.0));
            h.extend(Pen::arc_pts(12.0, 7.0, 4.2, 200.0, 340.0));
            h.extend(Pen::arc_pts(16.2, 9.5, 3.6, 285.0, 435.0));
            h.extend([(17.0, 13.0), (17.0, 20.0)]);
            pen.poly(&h, true);
            pen.line((7.0, 16.5), (17.0, 16.5));
        }
        Cup => {
            pen.poly(&[(6.5, 7.0), (17.5, 7.0), (16.0, 21.0), (8.0, 21.0)], true);
            pen.poly(&[(5.0, 7.0), (6.5, 3.5), (17.5, 3.5), (19.0, 7.0)], true);
            pen.line((7.1, 12.0), (16.9, 12.0));
            pen.line((7.6, 16.0), (16.4, 16.0));
        }
        CashRegister => {
            pen.rrect(3.0, 12.0, 18.0, 9.0, 2.0);
            pen.rrect(6.5, 3.5, 11.0, 5.0, 1.5);
            pen.line((12.0, 8.5), (12.0, 12.0));
            for x in [7.5, 10.5, 13.5, 16.5] {
                pen.dot(x, 15.0, 1.0);
            }
            pen.line((3.0, 18.0), (21.0, 18.0));
        }
        Swap => {
            pen.line((4.0, 8.0), (19.0, 8.0));
            pen.poly(&[(15.5, 4.5), (19.0, 8.0), (15.5, 11.5)], false);
            pen.line((20.0, 16.0), (5.0, 16.0));
            pen.poly(&[(8.5, 12.5), (5.0, 16.0), (8.5, 19.5)], false);
        }
        Plane => {
            let a = std::f32::consts::FRAC_PI_4;
            pen.rot = Some((a.cos(), a.sin()));
            pen.poly(
                &[
                    (12.0, 2.5),
                    (13.5, 5.0),
                    (13.5, 18.0),
                    (12.0, 21.0),
                    (10.5, 18.0),
                    (10.5, 5.0),
                ],
                true,
            );
            pen.poly(
                &[(10.5, 10.0), (3.0, 14.5), (3.0, 15.5), (10.5, 13.5)],
                false,
            );
            pen.poly(
                &[(13.5, 10.0), (21.0, 14.5), (21.0, 15.5), (13.5, 13.5)],
                false,
            );
            pen.poly(&[(10.5, 18.0), (8.0, 20.5), (10.8, 20.0)], false);
            pen.poly(&[(13.5, 18.0), (16.0, 20.5), (13.2, 20.0)], false);
        }
        Megaphone => {
            pen.poly(
                &[
                    (3.5, 10.0),
                    (8.0, 10.0),
                    (17.5, 5.0),
                    (17.5, 19.0),
                    (8.0, 14.0),
                    (3.5, 14.0),
                ],
                true,
            );
            pen.poly(
                &[(6.5, 14.0), (7.8, 19.5), (10.5, 19.5), (9.8, 15.2)],
                false,
            );
            pen.line((20.5, 10.0), (20.5, 14.0));
        }
        ClipboardCheck => {
            pen.rrect(5.0, 4.5, 14.0, 17.0, 2.5);
            pen.rrect(9.0, 2.5, 6.0, 4.0, 1.2);
            pen.poly(&[(8.5, 13.5), (11.0, 16.0), (15.5, 11.0)], false);
        }
        FileExport => {
            pen.poly(
                &[
                    (6.0, 3.0),
                    (14.0, 3.0),
                    (18.5, 7.5),
                    (18.5, 21.0),
                    (6.0, 21.0),
                ],
                true,
            );
            pen.poly(&[(14.0, 3.0), (14.0, 7.5), (18.5, 7.5)], false);
            pen.line((12.25, 10.5), (12.25, 17.5));
            pen.poly(&[(9.25, 14.5), (12.25, 17.5), (15.25, 14.5)], false);
        }
        ChevronDown => pen.poly(&[(6.0, 9.0), (12.0, 15.0), (18.0, 9.0)], false),
        ChevronUp => pen.poly(&[(6.0, 15.0), (12.0, 9.0), (18.0, 15.0)], false),
        ChevronRight => pen.poly(&[(9.0, 6.0), (15.0, 12.0), (9.0, 18.0)], false),
        ChevronLeft => pen.poly(&[(15.0, 6.0), (9.0, 12.0), (15.0, 18.0)], false),
        Search => {
            pen.circle(10.5, 10.5, 6.5);
            pen.line((15.5, 15.5), (20.5, 20.5));
        }
        Sparkle => {
            pen.poly(
                &[
                    (12.0, 3.0),
                    (13.8, 10.2),
                    (21.0, 12.0),
                    (13.8, 13.8),
                    (12.0, 21.0),
                    (10.2, 13.8),
                    (3.0, 12.0),
                    (10.2, 10.2),
                ],
                true,
            );
        }
        // --- hands-on update: tastes -------------------------------------------
        Flame => {
            pen.poly(
                &[
                    (12.0, 2.8),
                    (15.6, 7.4),
                    (18.0, 12.0),
                    (17.6, 16.4),
                    (15.0, 19.8),
                    (12.0, 21.0),
                    (9.0, 19.8),
                    (6.4, 16.4),
                    (6.2, 12.4),
                    (8.0, 9.0),
                    (9.6, 11.6),
                    (10.4, 7.6),
                ],
                true,
            );
            pen.poly(
                &[(12.0, 12.8), (14.2, 16.0), (13.6, 18.4), (12.0, 19.0)],
                false,
            );
        }
        Snowflake => {
            for k in 0..3 {
                let a = (90.0 + k as f32 * 60.0_f32).to_radians();
                let (dx, dy) = (a.cos() * 9.0, a.sin() * 9.0);
                pen.line((12.0 - dx, 12.0 - dy), (12.0 + dx, 12.0 + dy));
            }
            for k in 0..6 {
                let a = (90.0 + k as f32 * 60.0_f32).to_radians();
                let b = (12.0 + a.cos() * 5.8, 12.0 + a.sin() * 5.8);
                for side in [-1.0_f32, 1.0] {
                    let w = a + side * 0.75;
                    pen.line(b, (b.0 + w.cos() * 2.8, b.1 + w.sin() * 2.8));
                }
            }
        }
        Candy => {
            pen.circle(12.0, 12.0, 4.6);
            pen.tri((7.4, 12.0), (3.0, 8.6), (3.0, 15.4));
            pen.tri((16.6, 12.0), (21.0, 8.6), (21.0, 15.4));
            pen.arc(12.0, 12.0, 2.0, 200.0, 290.0);
        }
        Bean => {
            let a = 35.0_f32.to_radians();
            pen.rot = Some((a.cos(), a.sin()));
            pen.ellipse(12.0, 12.0, 5.8, 8.6);
            pen.poly(
                &[(12.0, 3.8), (10.6, 8.0), (13.4, 16.0), (12.0, 20.2)],
                false,
            );
        }
        Bolt => pen.poly(
            &[
                (13.8, 2.6),
                (5.6, 13.4),
                (11.6, 13.4),
                (10.2, 21.4),
                (18.4, 10.4),
                (12.4, 10.4),
            ],
            true,
        ),
        Feather => {
            pen.poly(
                &[
                    (19.4, 3.0),
                    (13.0, 4.4),
                    (8.6, 8.8),
                    (7.0, 14.0),
                    (7.6, 16.8),
                ],
                false,
            );
            pen.poly(
                &[
                    (19.4, 3.0),
                    (18.4, 9.0),
                    (14.6, 13.6),
                    (10.0, 16.4),
                    (7.6, 16.8),
                ],
                false,
            );
            pen.line((4.2, 20.0), (15.4, 8.0));
        }
        Gem => {
            pen.poly(
                &[
                    (7.0, 4.5),
                    (17.0, 4.5),
                    (21.0, 9.5),
                    (12.0, 20.5),
                    (3.0, 9.5),
                ],
                true,
            );
            pen.line((3.4, 9.5), (20.6, 9.5));
            pen.poly(&[(9.6, 4.8), (8.8, 9.5), (12.0, 20.0)], false);
            pen.poly(&[(14.4, 4.8), (15.2, 9.5), (12.0, 20.0)], false);
        }
        Stopwatch => {
            pen.circle(12.0, 13.6, 7.4);
            pen.line((12.0, 13.6), (14.8, 10.4));
            pen.line((9.6, 3.2), (14.4, 3.2));
            pen.line((12.0, 3.2), (12.0, 6.0));
            pen.line((18.0, 6.6), (19.6, 8.2));
        }
        Bowl => {
            pen.line((3.2, 11.4), (20.8, 11.4));
            pen.arc(12.0, 11.4, 8.8, 0.0, 180.0);
            pen.line((9.0, 20.2), (15.0, 20.2));
            pen.poly(&[(9.4, 8.2), (8.6, 6.2), (9.6, 4.0)], false);
            pen.poly(&[(14.0, 8.2), (13.2, 6.2), (14.2, 4.0)], false);
        }
        Anise => {
            let pts: Vec<P> = (0..16)
                .map(|i| {
                    let a = (-90.0 + i as f32 * 22.5_f32).to_radians();
                    let r = if i % 2 == 0 { 9.2 } else { 3.6 };
                    (12.0 + a.cos() * r, 12.0 + a.sin() * r)
                })
                .collect();
            pen.poly(&pts, true);
            pen.dot(12.0, 12.0, 1.3);
        }
        Leaf => {
            pen.poly(
                &[
                    (4.6, 19.4),
                    (4.8, 12.4),
                    (8.4, 7.2),
                    (13.6, 4.8),
                    (19.6, 4.4),
                    (19.0, 10.6),
                    (16.2, 15.6),
                    (11.0, 18.8),
                ],
                true,
            );
            pen.line((4.6, 19.4), (14.6, 9.4));
        }
        // --- duties, records, book, journal, rozpis -----------------------------
        Tray => {
            pen.arc(12.0, 17.0, 7.6, 180.0, 360.0);
            pen.line((2.8, 17.4), (21.2, 17.4));
            pen.dot(12.0, 7.9, 1.4);
            pen.line((6.0, 20.2), (18.0, 20.2));
        }
        Dishes => {
            pen.rrect(3.4, 5.6, 17.2, 3.6, 1.8);
            pen.rrect(4.6, 10.6, 14.8, 3.6, 1.8);
            pen.rrect(3.4, 15.6, 17.2, 3.6, 1.8);
        }
        Vinyl => {
            pen.circle(12.0, 12.0, 8.8);
            pen.circle(12.0, 12.0, 3.0);
            pen.dot(12.0, 12.0, 0.9);
            pen.arc(12.0, 12.0, 6.0, 200.0, 280.0);
        }
        Repeat => {
            pen.arc(12.0, 12.0, 7.4, 130.0, 400.0);
            arrowhead(&pen, (12.0, 12.0), 7.4, 400.0);
        }
        Book => {
            pen.poly(
                &[
                    (12.0, 6.6),
                    (8.2, 5.0),
                    (3.4, 5.4),
                    (3.4, 18.4),
                    (8.2, 18.0),
                    (12.0, 19.6),
                ],
                false,
            );
            pen.poly(
                &[
                    (12.0, 6.6),
                    (15.8, 5.0),
                    (20.6, 5.4),
                    (20.6, 18.4),
                    (15.8, 18.0),
                    (12.0, 19.6),
                ],
                false,
            );
            pen.line((12.0, 6.6), (12.0, 19.6));
        }
        Camera => {
            pen.rrect(3.0, 7.0, 18.0, 12.6, 2.6);
            pen.circle(12.0, 13.2, 3.7);
            pen.poly(&[(8.2, 7.0), (9.6, 4.6), (14.4, 4.6), (15.8, 7.0)], false);
            pen.dot(17.4, 10.0, 0.95);
        }
        Cloud => pen.poly(&cloud_pts(0.0, 0.0, 1.0), true),
        CloudSun => {
            pen.arc(8.6, 9.0, 3.6, 120.0, 330.0);
            for a in [165.0_f32, 210.0, 255.0, 300.0] {
                let r = a.to_radians();
                pen.line(
                    (8.6 + r.cos() * 5.6, 9.0 + r.sin() * 5.6),
                    (8.6 + r.cos() * 7.4, 9.0 + r.sin() * 7.4),
                );
            }
            pen.poly(&cloud_pts(2.0, 2.4, 0.8), true);
        }
        LinkBroken => {
            let k = std::f32::consts::FRAC_1_SQRT_2;
            pen.rot = Some((k, -k));
            pen.rrect(2.0, 9.0, 9.4, 6.0, 3.0);
            pen.rrect(12.6, 9.0, 9.4, 6.0, 3.0);
            pen.rot = None;
            pen.line((14.6, 3.6), (14.0, 6.0));
            pen.line((9.4, 18.0), (10.0, 20.4));
        }
        Forward => {
            pen.poly(
                &[
                    (4.0, 19.0),
                    (4.0, 15.4),
                    (6.6, 11.6),
                    (11.2, 10.2),
                    (19.0, 10.2),
                ],
                false,
            );
            pen.poly(&[(15.0, 6.2), (19.0, 10.2), (15.0, 14.2)], false);
        }
        Jar => {
            pen.rrect(5.6, 8.0, 12.8, 13.2, 3.2);
            pen.rrect(7.0, 3.6, 10.0, 4.4, 1.4);
            pen.line((9.0, 14.2), (15.0, 14.2));
        }
        Flask => {
            pen.poly(
                &[
                    (9.6, 3.4),
                    (9.6, 9.4),
                    (4.6, 18.2),
                    (5.8, 20.6),
                    (18.2, 20.6),
                    (19.4, 18.2),
                    (14.4, 9.4),
                    (14.4, 3.4),
                ],
                false,
            );
            pen.line((8.4, 3.4), (15.6, 3.4));
            pen.line((7.2, 15.2), (16.8, 15.2));
        }
        Sunrise => {
            pen.arc(12.0, 16.0, 5.0, 180.0, 360.0);
            pen.line((2.8, 16.0), (21.2, 16.0));
            pen.line((7.6, 19.6), (16.4, 19.6));
            pen.line((12.0, 5.4), (12.0, 8.0));
            pen.line((5.2, 8.6), (7.0, 10.4));
            pen.line((18.8, 8.6), (17.0, 10.4));
        }
    }
}

/// Arrowhead at the end of an arc (angle `deg` on a circle of radius `r` around `c`),
/// pointing along the direction of increasing angle.
fn arrowhead(pen: &Pen, c: P, r: f32, deg: f32) {
    let a = deg.to_radians();
    let tip = (c.0 + a.cos() * r, c.1 + a.sin() * r);
    let back = vec2(a.sin(), -a.cos());
    for side in [-0.7_f32, 0.7] {
        let (s, co) = side.sin_cos();
        let d = vec2(back.x * co - back.y * s, back.x * s + back.y * co) * 4.2;
        pen.line(tip, (tip.0 + d.x, tip.1 + d.y));
    }
}

/// Outline of a small cloud on the 24 grid, scaled by `k` around the centre and moved.
fn cloud_pts(dx: f32, dy: f32, k: f32) -> Vec<P> {
    let mut pts: Vec<P> = Vec::new();
    pts.extend(Pen::arc_pts(7.6, 14.6, 4.0, 90.0, 250.0));
    pts.extend(Pen::arc_pts(12.6, 11.4, 5.2, 205.0, 335.0));
    pts.extend(Pen::arc_pts(17.0, 14.8, 3.8, 280.0, 450.0));
    pts.into_iter()
        .map(|(x, y)| (12.0 + (x - 12.0) * k + dx, 12.0 + (y - 12.0) * k + dy))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_roundtrip_and_unique() {
        let mut names: Vec<&str> = Icon::ALL.iter().map(|i| i.name()).collect();
        for i in Icon::ALL {
            assert_eq!(Icon::from_name(i.name()), Some(i));
        }
        names.sort();
        names.dedup();
        assert_eq!(names.len(), Icon::ALL.len());
    }
}
