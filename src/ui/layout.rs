//! Rect layout helpers (pure, no drawing): anchoring, splitting, rows/columns, grids.
//!
//! ```ignore
//! let hud = anchored(ui.screen(), Anchor::TopLeft, vec2(340.0, 64.0), 12.0);
//! let (icon, rest) = hud.inset(10.0).split_left(24.0, 8.0);
//! for (i, r) in row(rest, 4, 4.0).iter().enumerate() { /* 4 equal cells */ }
//! let mut f = Flow::row(bar, 6.0);
//! let a = f.next(80.0); let b = f.next(120.0); let rest = f.rest();
//! ```

use macroquad::prelude::{Rect, Vec2, vec2};

/// Where a block sticks to its container.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

/// Place a block of `size` inside `container` at `anchor`, `margin` away from the edges.
pub fn anchored(container: Rect, anchor: Anchor, size: Vec2, margin: f32) -> Rect {
    anchored_xy(container, anchor, size, vec2(margin, margin))
}

/// Like [`anchored`] with separate x/y margins.
pub fn anchored_xy(container: Rect, anchor: Anchor, size: Vec2, margin: Vec2) -> Rect {
    use Anchor::*;
    let c = container;
    let x = match anchor {
        TopLeft | Left | BottomLeft => c.x + margin.x,
        Top | Center | Bottom => c.x + (c.w - size.x) / 2.0,
        TopRight | Right | BottomRight => c.x + c.w - size.x - margin.x,
    };
    let y = match anchor {
        TopLeft | Top | TopRight => c.y + margin.y,
        Left | Center | Right => c.y + (c.h - size.y) / 2.0,
        BottomLeft | Bottom | BottomRight => c.y + c.h - size.y - margin.y,
    };
    Rect::new(x, y, size.x, size.y)
}

/// Convenience methods on `Rect`.
pub trait RectExt: Sized {
    fn inset(self, d: f32) -> Rect;
    fn inset_xy(self, dx: f32, dy: f32) -> Rect;
    /// Inset each side separately (left, top, right, bottom).
    fn inset_ltrb(self, l: f32, t: f32, r: f32, b: f32) -> Rect;
    fn grow(self, d: f32) -> Rect;
    fn translate(self, d: Vec2) -> Rect;
    /// Cut `w` from the left; returns (left, rest) with `gap` between.
    fn split_left(self, w: f32, gap: f32) -> (Rect, Rect);
    fn split_right(self, w: f32, gap: f32) -> (Rect, Rect);
    fn split_top(self, h: f32, gap: f32) -> (Rect, Rect);
    fn split_bottom(self, h: f32, gap: f32) -> (Rect, Rect);
    /// Rect of `size` centred inside self.
    fn centered(self, size: Vec2) -> Rect;
    fn with_h(self, h: f32) -> Rect;
    fn with_w(self, w: f32) -> Rect;
    fn right(self) -> f32;
    fn bottom(self) -> f32;
    fn center_y(self) -> f32;
    fn intersect_or_empty(self, other: Rect) -> Rect;
}

impl RectExt for Rect {
    fn inset(self, d: f32) -> Rect {
        self.inset_xy(d, d)
    }
    fn inset_xy(self, dx: f32, dy: f32) -> Rect {
        Rect::new(
            self.x + dx,
            self.y + dy,
            (self.w - 2.0 * dx).max(0.0),
            (self.h - 2.0 * dy).max(0.0),
        )
    }
    fn inset_ltrb(self, l: f32, t: f32, r: f32, b: f32) -> Rect {
        Rect::new(
            self.x + l,
            self.y + t,
            (self.w - l - r).max(0.0),
            (self.h - t - b).max(0.0),
        )
    }
    fn grow(self, d: f32) -> Rect {
        Rect::new(self.x - d, self.y - d, self.w + 2.0 * d, self.h + 2.0 * d)
    }
    fn translate(self, d: Vec2) -> Rect {
        Rect::new(self.x + d.x, self.y + d.y, self.w, self.h)
    }
    fn split_left(self, w: f32, gap: f32) -> (Rect, Rect) {
        let w = w.min(self.w);
        (
            Rect::new(self.x, self.y, w, self.h),
            Rect::new(
                self.x + w + gap,
                self.y,
                (self.w - w - gap).max(0.0),
                self.h,
            ),
        )
    }
    fn split_right(self, w: f32, gap: f32) -> (Rect, Rect) {
        let w = w.min(self.w);
        (
            Rect::new(self.x + self.w - w, self.y, w, self.h),
            Rect::new(self.x, self.y, (self.w - w - gap).max(0.0), self.h),
        )
    }
    fn split_top(self, h: f32, gap: f32) -> (Rect, Rect) {
        let h = h.min(self.h);
        (
            Rect::new(self.x, self.y, self.w, h),
            Rect::new(
                self.x,
                self.y + h + gap,
                self.w,
                (self.h - h - gap).max(0.0),
            ),
        )
    }
    fn split_bottom(self, h: f32, gap: f32) -> (Rect, Rect) {
        let h = h.min(self.h);
        (
            Rect::new(self.x, self.y + self.h - h, self.w, h),
            Rect::new(self.x, self.y, self.w, (self.h - h - gap).max(0.0)),
        )
    }
    fn centered(self, size: Vec2) -> Rect {
        Rect::new(
            self.x + (self.w - size.x) / 2.0,
            self.y + (self.h - size.y) / 2.0,
            size.x,
            size.y,
        )
    }
    fn with_h(self, h: f32) -> Rect {
        Rect::new(self.x, self.y, self.w, h)
    }
    fn with_w(self, w: f32) -> Rect {
        Rect::new(self.x, self.y, w, self.h)
    }
    fn right(self) -> f32 {
        self.x + self.w
    }
    fn bottom(self) -> f32 {
        self.y + self.h
    }
    fn center_y(self) -> f32 {
        self.y + self.h / 2.0
    }
    fn intersect_or_empty(self, other: Rect) -> Rect {
        let x0 = self.x.max(other.x);
        let y0 = self.y.max(other.y);
        let x1 = self.right().min(other.right());
        let y1 = self.bottom().min(other.bottom());
        Rect::new(x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0))
    }
}

/// `n` equal cells side by side.
pub fn row(r: Rect, n: usize, gap: f32) -> Vec<Rect> {
    weighted_row(r, &vec![1.0; n], gap)
}

/// `n` equal cells stacked top to bottom.
pub fn column(r: Rect, n: usize, gap: f32) -> Vec<Rect> {
    weighted_column(r, &vec![1.0; n], gap)
}

/// Cells whose widths are proportional to `weights`.
pub fn weighted_row(r: Rect, weights: &[f32], gap: f32) -> Vec<Rect> {
    let spans = split_span(r.x, r.w, weights, gap);
    spans
        .into_iter()
        .map(|(x, w)| Rect::new(x, r.y, w, r.h))
        .collect()
}

pub fn weighted_column(r: Rect, weights: &[f32], gap: f32) -> Vec<Rect> {
    let spans = split_span(r.y, r.h, weights, gap);
    spans
        .into_iter()
        .map(|(y, h)| Rect::new(r.x, y, r.w, h))
        .collect()
}

fn split_span(start: f32, len: f32, weights: &[f32], gap: f32) -> Vec<(f32, f32)> {
    let n = weights.len();
    if n == 0 {
        return Vec::new();
    }
    let total: f32 = weights.iter().sum::<f32>().max(1e-6);
    let free = (len - gap * (n - 1) as f32).max(0.0);
    let mut pos = start;
    weights
        .iter()
        .map(|w| {
            let size = free * w / total;
            let span = (pos, size);
            pos += size + gap;
            span
        })
        .collect()
}

/// `cols × rows` grid of equal cells, row-major.
pub fn grid(r: Rect, cols: usize, rows: usize, gap: Vec2) -> Vec<Rect> {
    let mut out = Vec::with_capacity(cols * rows);
    for rr in weighted_column(r, &vec![1.0; rows], gap.y) {
        out.extend(weighted_row(rr, &vec![1.0; cols], gap.x));
    }
    out
}

/// Grid cell `i` (row-major) with fixed cell size, starting at `origin`.
pub fn grid_cell(origin: Vec2, cell: Vec2, gap: Vec2, cols: usize, i: usize) -> Rect {
    let cols = cols.max(1);
    let (c, r) = (i % cols, i / cols);
    Rect::new(
        origin.x + c as f32 * (cell.x + gap.x),
        origin.y + r as f32 * (cell.y + gap.y),
        cell.x,
        cell.y,
    )
}

/// Sequential cursor that hands out rects along a row or column.
#[derive(Clone, Copy, Debug)]
pub struct Flow {
    area: Rect,
    gap: f32,
    horizontal: bool,
    cursor: f32,
    end: f32,
}

impl Flow {
    /// Left → right.
    pub fn row(area: Rect, gap: f32) -> Self {
        Flow {
            area,
            gap,
            horizontal: true,
            cursor: area.x,
            end: area.x + area.w,
        }
    }
    /// Top → bottom.
    pub fn column(area: Rect, gap: f32) -> Self {
        Flow {
            area,
            gap,
            horizontal: false,
            cursor: area.y,
            end: area.y + area.h,
        }
    }
    /// Right → left (use for trailing buttons). `next` returns rects from the right edge.
    pub fn row_rev(area: Rect, gap: f32) -> Self {
        Flow {
            area,
            gap,
            horizontal: true,
            cursor: area.x + area.w,
            end: area.x,
        }
    }

    fn reversed(&self) -> bool {
        self.end < self.cursor
    }

    /// Next rect of `size` along the flow axis.
    pub fn next(&mut self, size: f32) -> Rect {
        let a = self.area;
        if self.reversed() {
            self.cursor -= size;
            let r = Rect::new(self.cursor, a.y, size, a.h);
            self.cursor -= self.gap;
            return r;
        }
        let r = if self.horizontal {
            Rect::new(self.cursor, a.y, size, a.h)
        } else {
            Rect::new(a.x, self.cursor, a.w, size)
        };
        self.cursor += size + self.gap;
        r
    }

    /// Skip `d` along the axis.
    pub fn space(&mut self, d: f32) {
        if self.reversed() {
            self.cursor -= d;
        } else {
            self.cursor += d;
        }
    }

    /// Whatever is left.
    pub fn rest(&self) -> Rect {
        let a = self.area;
        if self.reversed() {
            return Rect::new(self.end, a.y, (self.cursor - self.end).max(0.0), a.h);
        }
        let left = (self.end - self.cursor).max(0.0);
        if self.horizontal {
            Rect::new(self.cursor, a.y, left, a.h)
        } else {
            Rect::new(a.x, self.cursor, a.w, left)
        }
    }

    /// Position the cursor reached (x for rows, y for columns), excluding the last gap.
    pub fn used(&self) -> f32 {
        if self.reversed() {
            self.area.x + self.area.w - self.cursor - self.gap
        } else if self.horizontal {
            self.cursor - self.gap - self.area.x
        } else {
            self.cursor - self.gap - self.area.y
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn anchors_respect_margin() {
        let screen = Rect::new(0.0, 0.0, 1280.0, 720.0);
        let s = vec2(340.0, 64.0);
        let tl = anchored(screen, Anchor::TopLeft, s, 12.0);
        assert_eq!((tl.x, tl.y), (12.0, 12.0));
        let br = anchored(screen, Anchor::BottomRight, s, 12.0);
        assert!(approx(br.x + br.w, 1268.0) && approx(br.y + br.h, 708.0));
        let top = anchored(screen, Anchor::Top, vec2(400.0, 32.0), 12.0);
        assert!(approx(top.x, 440.0) && approx(top.y, 12.0));
        let c = anchored(screen, Anchor::Center, vec2(100.0, 100.0), 0.0);
        assert!(approx(c.x, 590.0) && approx(c.y, 310.0));
    }

    #[test]
    fn rows_fill_width_with_gaps() {
        let cells = row(Rect::new(10.0, 0.0, 100.0, 20.0), 4, 4.0);
        assert_eq!(cells.len(), 4);
        assert!(approx(cells[0].x, 10.0));
        assert!(approx(cells[0].w, 22.0));
        assert!(approx(cells[3].x + cells[3].w, 110.0));
        let w = weighted_row(Rect::new(0.0, 0.0, 100.0, 10.0), &[1.0, 3.0], 0.0);
        assert!(approx(w[0].w, 25.0) && approx(w[1].w, 75.0));
    }

    #[test]
    fn grid_is_row_major() {
        let g = grid(Rect::new(0.0, 0.0, 100.0, 50.0), 2, 2, vec2(0.0, 0.0));
        assert_eq!(g.len(), 4);
        assert!(approx(g[1].x, 50.0) && approx(g[1].y, 0.0));
        assert!(approx(g[2].x, 0.0) && approx(g[2].y, 25.0));
        let c = grid_cell(vec2(0.0, 0.0), vec2(10.0, 10.0), vec2(2.0, 2.0), 3, 4);
        assert!(approx(c.x, 12.0) && approx(c.y, 12.0));
    }

    #[test]
    fn splits_and_flow() {
        let r = Rect::new(0.0, 0.0, 100.0, 40.0);
        let (a, b) = r.split_left(30.0, 10.0);
        assert!(approx(a.w, 30.0) && approx(b.x, 40.0) && approx(b.w, 60.0));
        let (bot, top) = r.split_bottom(10.0, 0.0);
        assert!(approx(bot.y, 30.0) && approx(top.h, 30.0));
        let mut f = Flow::row(r, 4.0);
        let x = f.next(20.0);
        let y = f.next(30.0);
        assert!(approx(x.x, 0.0) && approx(y.x, 24.0));
        assert!(approx(f.rest().x, 58.0) && approx(f.rest().w, 42.0));
        assert!(approx(f.used(), 54.0));
        let mut rev = Flow::row_rev(r, 4.0);
        let last = rev.next(20.0);
        assert!(approx(last.x, 80.0));
        assert!(approx(rev.next(10.0).x, 66.0));
        let mut col = Flow::column(r, 2.0);
        col.next(10.0);
        assert!(approx(col.next(10.0).y, 12.0));
    }

    #[test]
    fn inset_never_negative() {
        let r = Rect::new(0.0, 0.0, 10.0, 10.0).inset(8.0);
        assert_eq!(r.w, 0.0);
        let i = Rect::new(0.0, 0.0, 10.0, 10.0).intersect_or_empty(Rect::new(20.0, 0.0, 5.0, 5.0));
        assert_eq!(i.w, 0.0);
    }
}
