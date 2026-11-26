//! The crash log end to end (its own test process, so the panic hook is ours): rolling runs,
//! info lines without per-frame spam, and a deliberate panic written with its location.

use std::path::PathBuf;

use shiftella_game::crashlog;
use shiftella_game::i18n::Lang;

fn tmp_log() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("crashlog_test");
    let _ = std::fs::remove_dir_all(&dir);
    dir.join(crashlog::FILE_NAME)
}

#[test]
fn the_crash_log_rolls_and_records_a_panic() {
    let path = tmp_log();
    // seven starts: only the last five runs stay
    for i in 0..7 {
        crashlog::init_at(path.clone());
        crashlog::info(&format!("startup {i}"));
    }
    let text = std::fs::read_to_string(&path).unwrap();
    assert_eq!(
        text.matches(crashlog::RUN_MARK).count(),
        crashlog::KEEP_RUNS
    );
    assert!(!text.contains("startup 1"), "{text}");
    assert!(text.contains("startup 2") && text.contains("startup 6"));
    assert!(text.contains(env!("CARGO_PKG_VERSION")));

    // scene / language only when they change (the game calls these every frame)
    for _ in 0..100 {
        crashlog::set_lang(Lang::En);
        crashlog::set_scene("menu");
        crashlog::set_clock(None);
    }
    crashlog::set_scene("night 3");
    crashlog::set_clock(Some(160));
    crashlog::warn("can't load Auto: broken\nfile");
    let text = std::fs::read_to_string(&path).unwrap();
    assert_eq!(text.matches("scene: menu").count(), 1);
    assert_eq!(text.matches("language: en").count(), 1);
    assert!(text.contains("scene: night 3"));
    assert!(text.contains("warn  can't load Auto: broken ⏎ file"));

    // a panic (caught here; in the game it ends the run)
    let r = std::panic::catch_unwind(|| {
        panic!("deliberate test panic {}", 42);
    });
    assert!(r.is_err());
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("PANIC: deliberate test panic 42"), "{text}");
    assert!(text.contains("tests/crashlog.rs:"), "{text}");
    assert!(
        text.contains("language en · scene night 3 · clock 00:40"),
        "{text}"
    );
    assert!(text.contains("backtrace:"));
    assert!(text.len() < crashlog::MAX_BYTES);
}
