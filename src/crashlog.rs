//! The crash log (owner's item 15, design/v0.2.md G): a small rolling text file,
//! `shiftella.log`, in the save folder ([`crate::save::saves_dir`], so `SHIFTELLA_SAVES`
//! moves it too). Nothing is ever sent anywhere — the player can open the folder from
//! Settings and attach the file to a bug report if they like.
//!
//! * Every start of the game adds a run header (version, OS, time); the file keeps the last
//!   [`KEEP_RUNS`] runs and at most [`MAX_BYTES`] (older runs are dropped on the next start).
//! * A few info lines: startup, the language, scene changes (menu → day 3 → night 3 → a
//!   cutscene …), save / load errors. Never anything per frame; a run stops logging after
//!   [`MAX_LINES_PER_RUN`] lines (a panic is still written).
//! * A panic hook writes the message, the source location, the thread, the version, OS,
//!   language, scene and clock, and a short backtrace, then runs the default hook (stderr).
//!
//! `SHIFTELLA_TEST_PANIC=1` makes the game panic on purpose right after boot (to check the
//! log by hand; harmless: nothing is saved). Before [`init`] every function here is a no-op,
//! so unit tests and tools never write a log.

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::i18n::Lang;

/// File name inside the save folder.
pub const FILE_NAME: &str = "shiftella.log";
/// Runs kept in the file (this one included).
pub const KEEP_RUNS: usize = 5;
/// The file is trimmed to this size on every start.
pub const MAX_BYTES: usize = 64 * 1024;
/// Info lines written per run at most (a panic report is always written).
pub const MAX_LINES_PER_RUN: usize = 400;
/// Every run starts with a line beginning with this.
pub const RUN_MARK: &str = "=== Shiftella ";

struct State {
    path: PathBuf,
    started: Instant,
    lines: usize,
    lang: Option<Lang>,
    scene: String,
    /// Minutes since 22:00 of the current night (shown in a panic report only).
    clock: Option<i32>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);
/// The log file for the panic hook when the state is locked by the panicking thread.
static HOOK_PATH: OnceLock<PathBuf> = OnceLock::new();

/// The folder the log is in (the save folder).
pub fn log_dir() -> PathBuf {
    crate::save::saves_dir()
}

/// The log file.
pub fn log_path() -> PathBuf {
    log_dir().join(FILE_NAME)
}

/// Starts this run's log at [`log_path`] and installs the panic hook. Call once at startup,
/// after `SHIFTELLA_SAVES` is final.
pub fn init() {
    init_at(log_path());
}

/// [`init`] with an explicit file (tests).
pub fn init_at(path: PathBuf) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let old = std::fs::read_to_string(&path).unwrap_or_default();
    let header = run_header();
    let budget = MAX_BYTES.saturating_sub(header.len() + 1);
    let mut text = trim_runs(&old, KEEP_RUNS - 1, budget);
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&header);
    text.push('\n');
    let _ = std::fs::write(&path, text);
    let _ = HOOK_PATH.set(path.clone());
    *lock() = Some(State {
        path,
        started: Instant::now(),
        lines: 0,
        lang: None,
        scene: String::new(),
        clock: None,
    });
    install_hook();
}

/// The log file of this run, once [`init`] ran.
pub fn active_path() -> Option<PathBuf> {
    lock().as_ref().map(|s| s.path.clone())
}

pub fn info(msg: &str) {
    write_line("info ", msg);
}

pub fn warn(msg: &str) {
    write_line("warn ", msg);
}

pub fn error(msg: &str) {
    write_line("error", msg);
}

/// The game language; logged when it changes.
pub fn set_lang(lang: Lang) {
    let changed = {
        let mut g = lock();
        match g.as_mut() {
            Some(s) if s.lang != Some(lang) => {
                s.lang = Some(lang);
                true
            }
            _ => false,
        }
    };
    if changed {
        info(&format!("language: {}", lang.code()));
    }
}

/// What is on screen (`"menu"`, `"day 3"`, `"night 3"`, `"cutscene c_opening"` …); logged
/// when it changes. Cheap to call every frame.
pub fn set_scene(scene: &str) {
    let changed = {
        let mut g = lock();
        match g.as_mut() {
            Some(s) if s.scene != scene => {
                s.scene.clear();
                s.scene.push_str(scene);
                true
            }
            _ => false,
        }
    };
    if changed {
        info(&format!("scene: {scene}"));
    }
}

/// The night's clock (minutes since 22:00), only for a panic report. Every frame is fine.
pub fn set_clock(minute: Option<i32>) {
    if let Some(s) = lock().as_mut() {
        s.clock = minute;
    }
}

/// Opens the log folder in the system's file manager (macOS `open`, Windows `explorer`,
/// elsewhere `xdg-open`). Never opens anything in an invisible automation run.
pub fn open_log_folder() -> Result<(), String> {
    let dir = log_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    if crate::platform::automation_hidden() {
        return Ok(());
    }
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "explorer"
    } else {
        "xdg-open"
    };
    match std::process::Command::new(opener).arg(&dir).spawn() {
        Ok(mut child) => {
            // reap it in the background (no zombie while the game runs)
            std::thread::spawn(move || {
                let _ = child.wait();
            });
            info(&format!("opened the log folder ({opener})"));
            Ok(())
        }
        Err(e) => {
            let msg = format!("can't open {} with {opener}: {e}", dir.display());
            warn(&msg);
            Err(msg)
        }
    }
}

/// `SHIFTELLA_TEST_PANIC=1`: panic on purpose (checking the crash log by hand).
pub fn test_panic_if_asked() {
    if std::env::var("SHIFTELLA_TEST_PANIC").is_ok_and(|v| !v.is_empty() && v != "0") {
        panic!("SHIFTELLA_TEST_PANIC: a deliberate test crash");
    }
}

// ---------------------------------------------------------------------------------------------

fn lock() -> std::sync::MutexGuard<'static, Option<State>> {
    STATE.lock().unwrap_or_else(|p| p.into_inner())
}

fn write_line(level: &str, msg: &str) {
    let mut g = lock();
    let Some(s) = g.as_mut() else {
        return;
    };
    if s.lines >= MAX_LINES_PER_RUN {
        return;
    }
    s.lines += 1;
    let mut line = format!("{} {level} {}", stamp(s.started), one_line(msg));
    if s.lines == MAX_LINES_PER_RUN {
        line.push_str("\n… (line limit for this run reached)");
    }
    append(&s.path, &line);
}

fn append(path: &Path, text: &str) {
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(f, "{text}");
    }
}

/// A message on one line (a save error can carry newlines).
fn one_line(msg: &str) -> String {
    msg.replace(['\n', '\r'], " ⏎ ")
}

/// `21:03:14 +12.3s` — UTC wall clock and seconds since the start of the run.
fn stamp(started: Instant) -> String {
    let (_, _, _, h, m, s) = utc_now();
    format!(
        "{h:02}:{m:02}:{s:02} +{:.1}s",
        started.elapsed().as_secs_f32()
    )
}

/// `=== Shiftella 0.1.1 · 2026-11-26 21:03:11 UTC · macos 26.1 aarch64 ===`
pub fn run_header() -> String {
    let (y, mo, d, h, m, s) = utc_now();
    format!(
        "{RUN_MARK}{} · {y:04}-{mo:02}-{d:02} {h:02}:{m:02}:{s:02} UTC · {} ===",
        env!("CARGO_PKG_VERSION"),
        os_label()
    )
}

/// `macos 26.1 aarch64` (the macOS version from SystemVersion.plist when it can be read).
pub fn os_label() -> String {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let version = if cfg!(target_os = "macos") {
        std::fs::read_to_string("/System/Library/CoreServices/SystemVersion.plist")
            .ok()
            .and_then(|p| plist_string(&p, "ProductVersion"))
    } else {
        None
    };
    match version {
        Some(v) => format!("{os} {v} {arch}"),
        None => format!("{os} {arch}"),
    }
}

fn plist_string(plist: &str, key: &str) -> Option<String> {
    let at = plist.find(&format!("<key>{key}</key>"))?;
    let rest = &plist[at..];
    let a = rest.find("<string>")? + "<string>".len();
    let b = rest[a..].find("</string>")?;
    Some(rest[a..a + b].trim().to_string())
}

fn utc_now() -> (i64, u32, u32, u32, u32, u32) {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    utc(secs)
}

/// Unix seconds → (year, month, day, hour, minute, second), UTC (Howard Hinnant's
/// days-to-civil).
pub fn utc(secs: i64) -> (i64, u32, u32, u32, u32, u32) {
    let days = secs.div_euclid(86_400);
    let sod = secs.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (
        y,
        m,
        d,
        (sod / 3600) as u32,
        (sod % 3600 / 60) as u32,
        (sod % 60) as u32,
    )
}

/// Keeps the last `keep` runs of an old log (split at [`RUN_MARK`] lines) and at most
/// `max_bytes`: whole old runs go first; a single run that is still too long keeps its tail.
pub fn trim_runs(old: &str, keep: usize, max_bytes: usize) -> String {
    if keep == 0 || old.trim().is_empty() {
        return String::new();
    }
    let mut runs: Vec<String> = Vec::new();
    for line in old.lines() {
        if line.starts_with(RUN_MARK) || runs.is_empty() {
            runs.push(String::new());
        }
        let r = runs.last_mut().expect("a run");
        r.push_str(line);
        r.push('\n');
    }
    let skip = runs.len().saturating_sub(keep);
    let mut runs: Vec<String> = runs.into_iter().skip(skip).collect();
    while runs.len() > 1 && runs.iter().map(String::len).sum::<usize>() > max_bytes {
        runs.remove(0);
    }
    let mut out = runs.concat();
    if out.len() > max_bytes {
        let note = "… (older lines dropped)\n";
        let mut cut = out.len() - max_bytes.saturating_sub(note.len());
        while cut < out.len() && !out.is_char_boundary(cut) {
            cut += 1;
        }
        // start at a line
        let cut = out[cut..]
            .find('\n')
            .map(|i| cut + i + 1)
            .unwrap_or(out.len());
        out = format!("{note}{}", &out[cut..]);
    }
    out
}

/// The panic report written to the log (pure, for tests).
pub fn panic_report(
    message: &str,
    location: Option<&str>,
    thread: &str,
    lang: Option<Lang>,
    scene: &str,
    clock: Option<i32>,
) -> String {
    let mut r = String::new();
    let _ = writeln!(r, "PANIC: {}", one_line(message));
    let _ = writeln!(r, "  at {}", location.unwrap_or("<unknown location>"));
    let clock = clock
        .map(|m| {
            let t = (22 * 60 + m).rem_euclid(24 * 60);
            format!(" · clock {:02}:{:02}", t / 60, t % 60)
        })
        .unwrap_or_default();
    let _ = write!(
        r,
        "  thread {thread} · version {} · {} · language {} · scene {}{clock}",
        env!("CARGO_PKG_VERSION"),
        os_label(),
        lang.map(|l| l.code()).unwrap_or("?"),
        if scene.is_empty() { "?" } else { scene },
    );
    r
}

/// Backtrace lines kept in a panic report.
const BACKTRACE_LINES: usize = 40;

/// The backtrace from the code that panicked on (drops the hook's and std's own frames).
fn trim_backtrace(bt: &str) -> Vec<&str> {
    let lines: Vec<&str> = bt.lines().collect();
    let start = lines
        .iter()
        .take(30)
        .rposition(|l| l.contains("core::panicking::") || l.contains("rust_begin_unwind"))
        .map(|i| i + 1)
        .unwrap_or(0);
    // a frame's own "at file:line" lines belong to the dropped frame
    let mut out: Vec<&str> = lines[start..].to_vec();
    while out
        .first()
        .is_some_and(|l| l.trim_start().starts_with("at "))
    {
        out.remove(0);
    }
    out
}

fn install_hook() {
    static INSTALLED: OnceLock<()> = OnceLock::new();
    if INSTALLED.set(()).is_err() {
        return;
    }
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "<non-text panic payload>".into());
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()));
        let thread = std::thread::current()
            .name()
            .unwrap_or("<unnamed>")
            .to_string();
        // the panicking thread may hold the state (a panic inside a log call): don't wait
        let (path, lang, scene, clock, started) = match STATE.try_lock() {
            Ok(g) => snapshot(&g),
            Err(std::sync::TryLockError::Poisoned(p)) => snapshot(&p.into_inner()),
            Err(std::sync::TryLockError::WouldBlock) => (None, None, String::new(), None, None),
        };
        let path = path.or_else(|| HOOK_PATH.get().cloned());
        if let Some(path) = path {
            let mut text = String::new();
            let when = started.map(stamp).unwrap_or_default();
            let _ = writeln!(
                text,
                "{when} PANIC ----------------------------------------"
            );
            text.push_str(&panic_report(
                &message,
                location.as_deref(),
                &thread,
                lang,
                &scene,
                clock,
            ));
            let bt = std::backtrace::Backtrace::force_capture().to_string();
            let mut lines = trim_backtrace(&bt).into_iter();
            text.push_str("\n  backtrace:");
            for l in lines.by_ref().take(BACKTRACE_LINES) {
                text.push_str("\n    ");
                text.push_str(l.trim_end());
            }
            if lines.next().is_some() {
                text.push_str("\n    …");
            }
            append(&path, &text);
        }
        default(info);
    }));
}

type Snapshot = (
    Option<PathBuf>,
    Option<Lang>,
    String,
    Option<i32>,
    Option<Instant>,
);

fn snapshot(g: &Option<State>) -> Snapshot {
    match g {
        Some(s) => (
            Some(s.path.clone()),
            s.lang,
            s.scene.clone(),
            s.clock,
            Some(s.started),
        ),
        None => (None, None, String::new(), None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_dates() {
        assert_eq!(utc(0), (1970, 1, 1, 0, 0, 0));
        // 2026-11-26 21:03:11 UTC
        assert_eq!(utc(1_795_726_991), (2026, 11, 26, 21, 3, 11));
        // a leap day
        assert_eq!(utc(1_709_208_000), (2024, 2, 29, 12, 0, 0));
    }

    fn log_of(runs: usize, lines: usize) -> String {
        let mut s = String::new();
        for r in 0..runs {
            s.push_str(&format!("{RUN_MARK}0.1.1 · run {r} ===\n"));
            for l in 0..lines {
                s.push_str(&format!("00:00:00 +0.0s info  run {r} line {l}\n"));
            }
        }
        s
    }

    #[test]
    fn old_runs_roll_off() {
        let t = trim_runs(&log_of(7, 3), KEEP_RUNS - 1, MAX_BYTES);
        assert_eq!(t.matches(RUN_MARK).count(), 4);
        assert!(t.starts_with(&format!("{RUN_MARK}0.1.1 · run 3 ===")));
        assert!(t.contains("run 6 line 2"));
        assert_eq!(trim_runs("", 4, MAX_BYTES), "");
        assert_eq!(trim_runs(&log_of(3, 1), 0, MAX_BYTES), "");
    }

    #[test]
    fn the_log_stays_small() {
        // big runs: whole old runs go first
        let t = trim_runs(&log_of(4, 400), 4, 20_000);
        assert!(t.len() <= 20_000, "{}", t.len());
        assert!(t.contains("run 3 line 399"));
        assert!(!t.contains("run 0 line"));
        // one huge run keeps its tail, starting at a line
        let t = trim_runs(&log_of(1, 5000), 4, 4_000);
        assert!(t.len() <= 4_000, "{}", t.len());
        assert!(t.starts_with("… (older lines dropped)\n00:00:00"));
        assert!(t.ends_with("run 0 line 4999\n"));
    }

    #[test]
    fn a_panic_report_names_the_place_and_the_scene() {
        let r = panic_report(
            "index out of bounds",
            Some("src/sim/guests.rs:12:5"),
            "main",
            Some(Lang::Ru),
            "night 3",
            Some(160),
        );
        assert!(r.starts_with("PANIC: index out of bounds\n  at src/sim/guests.rs:12:5\n"));
        assert!(r.contains("thread main"));
        assert!(r.contains(env!("CARGO_PKG_VERSION")));
        assert!(r.contains(std::env::consts::OS));
        assert!(r.contains("language ru"));
        assert!(r.contains("scene night 3 · clock 00:40"));
        // the preparation half hour is before midnight
        assert!(panic_report("x", None, "t", None, "", Some(-20)).contains("clock 21:40"));
    }

    #[test]
    fn the_backtrace_starts_at_the_panic() {
        let bt = "   0: std::backtrace::Backtrace::create\n   1: crashlog::hook\n   5: __rustc::rust_begin_unwind\n   6: core::panicking::panic_fmt\n             at /rustc/x/panicking.rs:75:14\n   7: shiftella_game::sim::tick\n             at src/sim/mod.rs:10:5\n";
        let t = trim_backtrace(bt);
        assert_eq!(t[0].trim(), "7: shiftella_game::sim::tick");
        assert_eq!(t.len(), 2);
    }

    #[test]
    fn plist_version() {
        let p = "<dict>\n\t<key>ProductName</key>\n\t<string>macOS</string>\n\t<key>ProductVersion</key>\n\t<string>26.1</string>\n</dict>";
        assert_eq!(plist_string(p, "ProductVersion").as_deref(), Some("26.1"));
        assert_eq!(plist_string(p, "Nope"), None);
    }
}
