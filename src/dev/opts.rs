//! Scene options set by the dev panel for this thread. The scene code reads its `SHIFTELLA_*`
//! options through [`crate::app::scene_env`]; while a dev opener runs ([`with`]) that answers
//! from here instead of the environment, so the panel opens exactly the screens the env vars
//! do — without touching the process environment (tests build scenes on many threads).

use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static OPTS: RefCell<Option<HashMap<String, String>>> = const { RefCell::new(None) };
}

/// Options that still come from the environment while an opener runs (the screenshot run's
/// own settings: instant cutscene text in a shot, …).
const PASS_THROUGH: &[&str] = &["SHIFTELLA_SHOT"];

/// `Some(answer)` while [`with`] runs on this thread: the opener's value for `name`, or `None`
/// for an option it didn't set (the environment is ignored then). `None` outside [`with`].
pub fn get(name: &str) -> Option<Option<String>> {
    if PASS_THROUGH.contains(&name) {
        return None;
    }
    OPTS.with(|o| {
        o.borrow()
            .as_ref()
            .map(|m| m.get(name).filter(|v| !v.trim().is_empty()).cloned())
    })
}

/// Runs `f` with these scene options (`("SHIFTELLA_PAGE", "3")`, …) in force on this thread.
pub fn with<R>(opts: &[(&str, &str)], f: impl FnOnce() -> R) -> R {
    let map = opts
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let before = OPTS.with(|o| o.borrow_mut().replace(map));
    struct Restore(Option<HashMap<String, String>>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let prev = self.0.take();
            OPTS.with(|o| *o.borrow_mut() = prev);
        }
    }
    let _restore = Restore(before);
    f()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_hold_only_inside_with() {
        assert_eq!(get("SHIFTELLA_PAGE"), None);
        with(&[("SHIFTELLA_PAGE", "5")], || {
            assert_eq!(get("SHIFTELLA_PAGE"), Some(Some("5".into())));
            // an option the opener didn't set masks the environment
            assert_eq!(get("SHIFTELLA_NIGHT"), Some(None));
            assert_eq!(
                crate::app::scene_env("SHIFTELLA_PAGE").as_deref(),
                Some("5")
            );
            with(&[("SHIFTELLA_NIGHT", "3")], || {
                assert_eq!(get("SHIFTELLA_PAGE"), Some(None));
            });
            assert_eq!(get("SHIFTELLA_PAGE"), Some(Some("5".into())));
        });
        assert_eq!(get("SHIFTELLA_PAGE"), None);
        assert_eq!(
            get("SHIFTELLA_SHOT"),
            None,
            "the shot setting passes through"
        );
    }
}
