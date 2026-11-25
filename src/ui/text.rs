//! Text: Inter in four weights, rasterised at physical pixels, sized in points.
//!
//! macroquad 0.4.16 already multiplies `font_size` by the window DPI inside
//! `draw_text_ex`, so we pass `font_size = pt × ui_scale` and
//! `font_scale = pt / font_size`: glyphs are rasterised at exactly the physical size
//! and laid out in UI units. Baselines are snapped to the pixel grid ⇒ crisp text.

use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use macroquad::prelude::*;

use super::draw::{faded, snap};

/// Font weight (static Inter files).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Weight {
    Regular,
    Medium,
    SemiBold,
    Bold,
}

/// Horizontal alignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// How a run of text looks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyle {
    /// Size in points.
    pub size: f32,
    pub weight: Weight,
    pub color: Color,
    /// Digits get the fixed advance of "0" (clocks, money, tables).
    pub tabular: bool,
    /// Extra letter spacing in points.
    pub tracking: f32,
    /// Line height as a multiple of `size`.
    pub line_height: f32,
}

impl TextStyle {
    pub const fn new(size: f32, weight: Weight, color: Color) -> Self {
        TextStyle {
            size,
            weight,
            color,
            tabular: false,
            tracking: 0.0,
            line_height: 1.4,
        }
    }
    pub const fn color(mut self, c: Color) -> Self {
        self.color = c;
        self
    }
    pub const fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
    pub const fn weight(mut self, w: Weight) -> Self {
        self.weight = w;
        self
    }
    pub const fn tabular(mut self) -> Self {
        self.tabular = true;
        self
    }
    pub const fn tracking(mut self, t: f32) -> Self {
        self.tracking = t;
        self
    }
    pub const fn line_height(mut self, lh: f32) -> Self {
        self.line_height = lh;
        self
    }
    /// Line box height in points.
    pub fn line_h(&self) -> f32 {
        (self.size * self.line_height).round()
    }
}

/// The four Inter faces plus measurement caches.
pub struct Fonts {
    faces: [Font; 4],
    scale: Cell<f32>,
    advances: RefCell<HashMap<(Weight, u16, char), f32>>,
    caps: RefCell<HashMap<(Weight, u16), f32>>,
}

impl Fonts {
    /// Load `assets/fonts/Inter-{Regular,Medium,SemiBold,Bold}.ttf`.
    pub async fn load() -> Result<Fonts, macroquad::Error> {
        Fonts::load_from("assets/fonts").await
    }

    pub async fn load_from(dir: &str) -> Result<Fonts, macroquad::Error> {
        let mut faces = Vec::with_capacity(4);
        for name in ["Regular", "Medium", "SemiBold", "Bold"] {
            let mut f = load_ttf_font(&format!("{dir}/Inter-{name}.ttf")).await?;
            f.set_filter(FilterMode::Linear);
            faces.push(f);
        }
        let faces: [Font; 4] = faces.try_into().expect("four faces");
        Ok(Fonts {
            faces,
            scale: Cell::new(1.0),
            advances: RefCell::new(HashMap::new()),
            caps: RefCell::new(HashMap::new()),
        })
    }

    /// The raw macroquad font of a weight.
    pub fn face(&self, w: Weight) -> &Font {
        &self.faces[w as usize]
    }

    /// Set by `Ui::begin_frame` (user `ui_scale`; DPI is handled by macroquad).
    pub fn set_scale(&self, s: f32) {
        self.scale.set(s);
    }

    fn raster(&self, pt: f32) -> (u16, f32) {
        let fs = (pt * self.scale.get()).round().max(1.0);
        (fs as u16, pt / fs)
    }

    fn advance_unit(&self, w: Weight, fs: u16, c: char) -> f32 {
        if let Some(a) = self.advances.borrow().get(&(w, fs, c)) {
            return *a;
        }
        let mut buf = [0u8; 4];
        let a = measure_text(c.encode_utf8(&mut buf), Some(self.face(w)), fs, 1.0).width;
        self.advances.borrow_mut().insert((w, fs, c), a);
        a
    }

    /// Advance of one character (points).
    pub fn advance(&self, c: char, style: &TextStyle) -> f32 {
        let (fs, k) = self.raster(style.size);
        self.advance_unit(style.weight, fs, c) * k
    }

    /// Height of capital letters above the baseline (points).
    pub fn cap_height(&self, style: &TextStyle) -> f32 {
        let (fs, k) = self.raster(style.size);
        let key = (style.weight, fs);
        if let Some(c) = self.caps.borrow().get(&key) {
            return *c * k;
        }
        let c = measure_text("H", Some(self.face(style.weight)), fs, 1.0).offset_y;
        self.caps.borrow_mut().insert(key, c);
        c * k
    }

    fn per_char(style: &TextStyle) -> bool {
        style.tabular || style.tracking != 0.0
    }

    /// Width of a single line (points).
    pub fn measure(&self, text: &str, style: &TextStyle) -> f32 {
        if text.is_empty() {
            return 0.0;
        }
        if Self::per_char(style) {
            let digit = self.advance('0', style);
            let n = text.chars().count() as f32;
            return text
                .chars()
                .map(|c| {
                    if style.tabular && c.is_ascii_digit() {
                        digit
                    } else {
                        self.advance(c, style)
                    }
                })
                .sum::<f32>()
                + style.tracking * (n - 1.0).max(0.0);
        }
        let (fs, k) = self.raster(style.size);
        measure_text(text, Some(self.face(style.weight)), fs, k).width
    }

    /// Draw one line with its baseline at `y`. Returns the width.
    pub fn draw(&self, text: &str, x: f32, y: f32, style: &TextStyle) -> f32 {
        if text.is_empty() {
            return 0.0;
        }
        let (fs, k) = self.raster(style.size);
        let color = faded(style.color);
        let font = self.face(style.weight);
        let (x0, y) = (snap(x), snap(y));
        let params = TextParams {
            font: Some(font),
            font_size: fs,
            font_scale: k,
            color,
            ..Default::default()
        };
        if !Self::per_char(style) {
            draw_text_ex(text, x0, y, params);
            return self.measure(text, style);
        }
        let digit = self.advance('0', style);
        let mut cx = x0;
        let mut buf = [0u8; 4];
        for c in text.chars() {
            let adv = self.advance(c, style);
            let s = c.encode_utf8(&mut buf);
            if style.tabular && c.is_ascii_digit() {
                draw_text_ex(s, snap(cx + (digit - adv) * 0.5), y, params.clone());
                cx += digit;
            } else {
                if c != ' ' {
                    draw_text_ex(s, snap(cx), y, params.clone());
                }
                cx += adv;
            }
            cx += style.tracking;
        }
        cx - x0 - style.tracking
    }

    /// Baseline that vertically centres capitals inside `r`.
    pub fn baseline_in(&self, r: Rect, style: &TextStyle) -> f32 {
        r.y + (r.h + self.cap_height(style)) * 0.5
    }

    /// Single line inside `r`: vertically centred, aligned, ellipsised to fit.
    /// Returns the drawn width.
    pub fn draw_in(&self, text: &str, r: Rect, style: &TextStyle, align: Align) -> f32 {
        let t = self.ellipsize(text, r.w, style);
        let w = self.measure(&t, style);
        let x = match align {
            Align::Left => r.x,
            Align::Center => r.x + (r.w - w) * 0.5,
            Align::Right => r.x + r.w - w,
        };
        self.draw(&t, x, self.baseline_in(r, style), style)
    }

    /// Draw so the text ends at `right` (baseline `y`).
    pub fn draw_right(&self, text: &str, right: f32, y: f32, style: &TextStyle) -> f32 {
        let w = self.measure(text, style);
        self.draw(text, right - w, y, style)
    }

    /// Draw centred on `cx` (baseline `y`).
    pub fn draw_centered(&self, text: &str, cx: f32, y: f32, style: &TextStyle) -> f32 {
        let w = self.measure(text, style);
        self.draw(text, cx - w * 0.5, y, style)
    }

    /// Cut with "…" so it fits `max_w`.
    pub fn ellipsize<'a>(&self, text: &'a str, max_w: f32, style: &TextStyle) -> Cow<'a, str> {
        ellipsize_with(text, max_w, |s| self.measure(s, style))
    }

    /// Greedy word-wrap into lines no wider than `max_w`.
    pub fn wrap(&self, text: &str, max_w: f32, style: &TextStyle) -> Vec<String> {
        wrap_with(text, max_w, |s| self.measure(s, style))
    }

    /// Wrapped paragraph starting at the top of `r`. At most `max_lines`
    /// (the last one ellipsised). Returns the used height.
    pub fn draw_wrapped(
        &self,
        text: &str,
        r: Rect,
        style: &TextStyle,
        align: Align,
        max_lines: Option<usize>,
    ) -> f32 {
        let lines = self.wrap(text, r.w, style);
        self.draw_lines(&lines, r, style, align, max_lines, usize::MAX)
    }

    /// Height a wrapped paragraph would take.
    pub fn wrapped_height(
        &self,
        text: &str,
        w: f32,
        style: &TextStyle,
        max_lines: Option<usize>,
    ) -> f32 {
        let n = self.wrap(text, w, style).len();
        let n = max_lines.map_or(n, |m| n.min(m));
        n as f32 * style.line_h()
    }

    /// Draw pre-wrapped lines, revealing only the first `visible_chars`
    /// characters (typewriter). Returns the used height.
    pub fn draw_lines(
        &self,
        lines: &[String],
        r: Rect,
        style: &TextStyle,
        align: Align,
        max_lines: Option<usize>,
        visible_chars: usize,
    ) -> f32 {
        let lh = style.line_h();
        let cap = self.cap_height(style);
        let count = max_lines.map_or(lines.len(), |m| lines.len().min(m));
        let mut left = visible_chars;
        for (i, line) in lines.iter().take(count).enumerate() {
            let base = r.y + i as f32 * lh + (lh + cap) * 0.5;
            let truncated = i + 1 == count && count < lines.len();
            let full: Cow<str> = if truncated {
                Cow::Owned(format!("{}…", line.trim_end()))
            } else {
                Cow::Borrowed(line.as_str())
            };
            let full: Cow<str> = if self.measure(&full, style) > r.w + 0.5 {
                Cow::Owned(self.ellipsize(&full, r.w, style).into_owned())
            } else {
                full
            };
            let n = full.chars().count();
            let shown: Cow<str> = if left >= n {
                full.clone()
            } else {
                Cow::Owned(full.chars().take(left).collect())
            };
            left = left.saturating_sub(n + 1);
            // alignment uses the full line so revealed text does not slide
            let w = self.measure(&full, style);
            let x = match align {
                Align::Left => r.x,
                Align::Center => r.x + (r.w - w) * 0.5,
                Align::Right => r.x + r.w - w,
            };
            self.draw(&shown, x, base, style);
            if left == 0 && shown.len() < full.len() {
                break;
            }
        }
        count as f32 * lh
    }
}

/// Total characters in wrapped lines, counting one per line break (typewriter length).
pub fn reveal_len(lines: &[String]) -> usize {
    lines
        .iter()
        .map(|l| l.chars().count() + 1)
        .sum::<usize>()
        .saturating_sub(1)
}

/// Greedy word wrap with an injected measure function (testable without fonts).
/// Explicit `\n` start new lines; words longer than the width are broken.
pub fn wrap_with(text: &str, max_w: f32, measure: impl Fn(&str) -> f32) -> Vec<String> {
    let mut out = Vec::new();
    for para in text.split('\n') {
        let mut line = String::new();
        for word in glue_single_letters(para) {
            let word = word.as_str();
            let candidate = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if measure(&candidate) <= max_w {
                line = candidate;
                continue;
            }
            if !line.is_empty() {
                out.push(std::mem::take(&mut line));
            }
            if measure(word) <= max_w {
                line = word.to_string();
                continue;
            }
            // break a long word by characters
            let mut piece = String::new();
            for c in word.chars() {
                piece.push(c);
                if measure(&piece) > max_w && piece.chars().count() > 1 {
                    piece.pop();
                    out.push(std::mem::take(&mut piece));
                    piece.push(c);
                }
            }
            line = piece;
        }
        out.push(line);
    }
    out
}

/// Split into words, binding one-letter words to the next one with a no-break space
/// (Czech/Slovak/Ukrainian typography: "u okna", "v mlze", "і ми" never end a line).
fn glue_single_letters(para: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut pending = String::new();
    for w in para.split(' ').filter(|w| !w.is_empty()) {
        let single = w.chars().count() == 1 && w.chars().all(char::is_alphabetic);
        if !pending.is_empty() {
            pending.push('\u{a0}');
        }
        pending.push_str(w);
        if !single {
            out.push(std::mem::take(&mut pending));
        }
    }
    if !pending.is_empty() {
        out.push(pending);
    }
    out
}

/// Shorten `text` with a trailing "…" so `measure` ≤ `max_w`.
pub fn ellipsize_with<'a>(
    text: &'a str,
    max_w: f32,
    measure: impl Fn(&str) -> f32,
) -> Cow<'a, str> {
    if measure(text) <= max_w + 0.01 {
        return Cow::Borrowed(text);
    }
    // bounds[k] = byte index after the first k chars
    let mut bounds: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
    bounds.push(text.len());
    let n = bounds.len() - 1;
    let fits = |k: usize| measure(&format!("{}…", text[..bounds[k]].trim_end())) <= max_w;
    if !fits(0) {
        return Cow::Borrowed("");
    }
    // largest k < n whose prefix + "…" fits (monotonic)
    let (mut lo, mut hi) = (0usize, n.saturating_sub(1));
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        if fits(mid) {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    if lo == 0 {
        return Cow::Borrowed("…");
    }
    Cow::Owned(format!("{}…", text[..bounds[lo]].trim_end()))
}

/// Up to two initials, uppercase ("Lída Nováková" → "LN", "їжак" → "Ї").
pub fn initials(name: &str) -> String {
    let words: Vec<&str> = name.split_whitespace().collect();
    let mut out = String::new();
    let pick = match words.len() {
        0 => return out,
        1 => vec![words[0]],
        n => vec![words[0], words[n - 1]],
    };
    for w in pick {
        if let Some(c) = w.chars().find(|c| c.is_alphanumeric()) {
            out.extend(c.to_uppercase());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fake monospace font: every char is 10 wide.
    fn mono(s: &str) -> f32 {
        s.chars().count() as f32 * 10.0
    }

    #[test]
    fn wraps_on_words() {
        let lines = wrap_with("Příliš žluťoučký kůň úpěl", 100.0, mono);
        assert_eq!(lines, vec!["Příliš", "žluťoučký", "kůň úpěl"]);
        for l in &lines {
            assert!(mono(l) <= 100.0);
        }
    }

    #[test]
    fn wraps_newlines_and_long_words() {
        let lines = wrap_with("ab\n\nabcdefghijkl", 50.0, mono);
        assert_eq!(lines, vec!["ab", "", "abcde", "fghij", "kl"]);
    }

    #[test]
    fn wrap_keeps_text() {
        let s = "Їжак ґречно їсть яблуко біля вокзалу";
        let joined = wrap_with(s, 80.0, mono).join(" ").replace('\u{a0}', " ");
        assert_eq!(joined, s);
    }

    #[test]
    fn one_letter_words_stick_to_the_next() {
        // "u" must not stay at the end of a line
        let lines = wrap_with("kdo zastoupí u pokladny", 140.0, mono);
        assert_eq!(lines, vec!["kdo zastoupí", "u\u{a0}pokladny"]);
        assert_eq!(wrap_with("a", 100.0, mono), vec!["a"]);
    }

    #[test]
    fn ellipsis_fits() {
        assert_eq!(ellipsize_with("short", 100.0, mono), "short");
        let e = ellipsize_with("Ďakujem, ľúbim ťa", 80.0, mono);
        assert!(mono(&e) <= 80.0);
        assert!(e.ends_with('…'));
        assert!(e.starts_with("Ďakujem"));
        assert_eq!(ellipsize_with("abc", 5.0, mono), "");
        assert_eq!(ellipsize_with("abc", 12.0, mono), "…");
    }

    #[test]
    fn reveal_counts_breaks() {
        let lines = vec!["ab".to_string(), "cde".to_string()];
        assert_eq!(reveal_len(&lines), 6);
    }

    #[test]
    fn initials_unicode() {
        assert_eq!(initials("Lída Nováková"), "LN");
        assert_eq!(initials("čeněk"), "Č");
        assert_eq!(initials("  їжак  ґречний "), "ЇҐ");
        assert_eq!(initials(""), "");
        assert_eq!(initials("Jan Petr Svoboda"), "JS");
    }
}
