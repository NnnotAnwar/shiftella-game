//! A lightweight per-frame profiler: `SHIFTELLA_PROFILE=path.csv` writes one CSV row per
//! frame with the CPU time of the frame's parts and a few counters (glyphs rasterised,
//! static layer rebuilds, light map / render target creations, face bakes, snapshots).
//!
//! Off (the default) every call is one relaxed atomic load and nothing else: no clock reads,
//! no allocation. `tools/profile_summary.py path.csv` prints medians / p95 / p99 per part.
//!
//! Times are the main thread's **CPU** time (`CLOCK_THREAD_CPUTIME_ID`), not wall time, so
//! other processes loading the machine don't show up as "slow frames"; `frame_wall` and
//! `present_wall` (the `next_frame().await`: GL flush of the batched draw calls + swap) are
//! wall time.
//!
//! Sections nest: `world` contains `lighting`, `ui` contains `photos`; `frame` is everything
//! from the top of the loop to `next_frame()`.

use std::cell::RefCell;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

static ON: AtomicBool = AtomicBool::new(false);

/// A timed part of the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Sec {
    /// The whole frame (top of the loop → before `next_frame()`).
    Frame,
    /// Face bakes for avatars (`register_faces` & co).
    Faces,
    /// Cutscene player, events, the sim ticks.
    Sim,
    /// Hands-on loop, autoplayer, autosave, audio decisions.
    Hands,
    /// `WorldScene::update` + `refresh` (people views, tracks, emotes).
    Refresh,
    /// The world draw (street, statics, furniture queue, people, lighting, emotes).
    World,
    /// The light map (inside `world`).
    Lighting,
    /// Overlays drawn over the world (hands view, build overlay, stories).
    Overlay,
    /// HUD, panels, overlays, toasts.
    Ui,
    /// Polaroid snapshots rendered for the journal (inside `ui`).
    Photos,
    /// The walk home / back step screen (instead of world + ui).
    Walk,
    /// `next_frame().await` (wall time: flush + swap).
    Present,
}

const SECS: [(&str, Sec); 12] = [
    ("frame", Sec::Frame),
    ("faces", Sec::Faces),
    ("sim", Sec::Sim),
    ("hands", Sec::Hands),
    ("refresh", Sec::Refresh),
    ("world", Sec::World),
    ("lighting", Sec::Lighting),
    ("overlay", Sec::Overlay),
    ("ui", Sec::Ui),
    ("photos", Sec::Photos),
    ("walk", Sec::Walk),
    ("present", Sec::Present),
];

/// Things counted per frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Counter {
    /// Characters newly rasterised into a font atlas.
    Glyphs,
    /// `GlyphCache::prepare` calls that had to rasterise (each flushes the batch).
    GlyphFlushes,
    /// Static layer (floors, walls) rebuilds.
    Statics,
    /// Render targets created (light map, static layer, faces, photos).
    RenderTargets,
    /// Faces baked.
    FaceBakes,
    /// Polaroid snapshots rendered.
    PhotoRenders,
    /// Sim ticks.
    Ticks,
    /// GL draw calls (from the vendored miniquad's counters).
    DrawCalls,
    /// GL buffer uploads (`glBufferSubData`; two per macroquad batch).
    BufferUpdates,
    /// Kilobytes of vertex + index data uploaded.
    UploadKb,
    /// Vertices uploaded (all of them).
    Vertices,
    /// Vertices of the UI's own meshes (`ui::draw`: glass, rounded rects, strokes, icons).
    UiVerts,
    /// Vertices of the world's mesh brushes (`render::draw`: fans, radial gradients, quads).
    BrushVerts,
}

const COUNTERS: [&str; 13] = [
    "glyphs",
    "glyph_flushes",
    "statics",
    "render_targets",
    "face_bakes",
    "photo_renders",
    "ticks",
    "draw_calls",
    "buffer_updates",
    "upload_kb",
    "vertices",
    "ui_verts",
    "brush_verts",
];

struct State {
    out: std::io::BufWriter<std::fs::File>,
    frame: u64,
    started: Instant,
    frame_wall: Instant,
    cpu: [u64; SECS.len()],
    counts: [u64; COUNTERS.len()],
    present_wall_ns: u64,
    /// miniquad's GL counters at the start of this frame.
    gl0: (u64, u64, u64, u64),
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

/// True while profiling (one relaxed load).
#[inline(always)]
pub fn on() -> bool {
    ON.load(Ordering::Relaxed)
}

/// Starts profiling if `SHIFTELLA_PROFILE` names a file (call once, on the main thread).
pub fn init() {
    let Some(path) = std::env::var("SHIFTELLA_PROFILE")
        .ok()
        .filter(|p| !p.trim().is_empty())
    else {
        return;
    };
    let file = match std::fs::File::create(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("warning: SHIFTELLA_PROFILE={path}: {e}");
            return;
        }
    };
    let mut out = std::io::BufWriter::new(file);
    let mut head = String::from("frame,t,scene,frame_wall,present_wall");
    for (name, _) in SECS {
        head.push(',');
        head.push_str(name);
    }
    for c in COUNTERS {
        head.push(',');
        head.push_str(c);
    }
    let _ = writeln!(out, "{head}");
    let now = Instant::now();
    STATE.with(|s| {
        *s.borrow_mut() = Some(State {
            out,
            frame: 0,
            started: now,
            frame_wall: now,
            cpu: [0; SECS.len()],
            counts: [0; COUNTERS.len()],
            present_wall_ns: 0,
            gl0: macroquad::miniquad::stats::read(),
        })
    });
    ON.store(true, Ordering::Relaxed);
}

/// This thread's CPU time in nanoseconds (0 where unsupported).
fn thread_cpu_ns() -> u64 {
    #[cfg(target_os = "macos")]
    {
        unsafe extern "C" {
            fn clock_gettime_nsec_np(clock_id: u32) -> u64;
        }
        /// `CLOCK_THREAD_CPUTIME_ID` in `<time.h>`.
        const CLOCK_THREAD_CPUTIME_ID: u32 = 16;
        // SAFETY: a plain libc call with a valid clock id.
        unsafe { clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID) }
    }
    #[cfg(not(target_os = "macos"))]
    {
        0
    }
}

/// A running section: its CPU time is added when it is dropped.
pub struct Span {
    start: Option<(Sec, u64, Instant)>,
}

/// Times a section until the returned guard drops (nothing at all when profiling is off).
#[inline]
pub fn span(sec: Sec) -> Span {
    if !on() {
        return Span { start: None };
    }
    Span {
        start: Some((sec, thread_cpu_ns(), Instant::now())),
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        let Some((sec, cpu0, wall0)) = self.start else {
            return;
        };
        let cpu = thread_cpu_ns().saturating_sub(cpu0);
        let wall = wall0.elapsed().as_nanos() as u64;
        STATE.with(|s| {
            if let Some(s) = s.borrow_mut().as_mut() {
                s.cpu[sec as usize] += cpu;
                if sec == Sec::Present {
                    s.present_wall_ns += wall;
                }
            }
        });
    }
}

/// Adds `n` to a counter of this frame.
#[inline]
pub fn count(c: Counter, n: u64) {
    if !on() {
        return;
    }
    STATE.with(|s| {
        if let Some(s) = s.borrow_mut().as_mut() {
            s.counts[c as usize] += n;
        }
    });
}

/// Ends a frame (call right after `next_frame().await`): writes its row and starts the next.
/// `scene` is a short label of what was on screen (no commas).
pub fn frame_end(scene: &str) {
    if !on() {
        return;
    }
    STATE.with(|s| {
        let mut b = s.borrow_mut();
        let Some(s) = b.as_mut() else { return };
        let now = Instant::now();
        let gl = macroquad::miniquad::stats::read();
        s.counts[Counter::BufferUpdates as usize] += gl.0 - s.gl0.0;
        s.counts[Counter::UploadKb as usize] += (gl.1 - s.gl0.1) / 1024;
        s.counts[Counter::DrawCalls as usize] += gl.2 - s.gl0.2;
        s.counts[Counter::Vertices as usize] += gl.3 - s.gl0.3;
        s.gl0 = gl;
        let ms = |ns: u64| ns as f64 / 1e6;
        let mut row = format!(
            "{},{:.3},{},{:.3},{:.3}",
            s.frame,
            (now - s.started).as_secs_f64(),
            scene.replace(',', ";"),
            (now - s.frame_wall).as_secs_f64() * 1000.0,
            ms(s.present_wall_ns)
        );
        for v in s.cpu {
            row.push_str(&format!(",{:.3}", ms(v)));
        }
        for v in s.counts {
            row.push_str(&format!(",{v}"));
        }
        let _ = writeln!(s.out, "{row}");
        s.frame += 1;
        s.frame_wall = now;
        s.cpu = [0; SECS.len()];
        s.counts = [0; COUNTERS.len()];
        s.present_wall_ns = 0;
    });
}

/// Flushes the CSV (call before the process exits).
pub fn finish() {
    if !on() {
        return;
    }
    STATE.with(|s| {
        if let Some(s) = s.borrow_mut().as_mut() {
            let _ = s.out.flush();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_by_default_and_spans_cost_nothing() {
        // the test binary never sets SHIFTELLA_PROFILE for this thread's state
        let s = span(Sec::World);
        assert!(s.start.is_none() || on());
        count(Counter::Glyphs, 3);
        frame_end("test");
    }

    #[test]
    fn thread_cpu_time_moves_forward() {
        let a = thread_cpu_ns();
        let mut x = 0u64;
        for i in 0..2_000_000u64 {
            x = x.wrapping_add(i * i);
        }
        std::hint::black_box(x);
        let b = thread_cpu_ns();
        if cfg!(target_os = "macos") {
            assert!(b > a);
        }
    }
}
