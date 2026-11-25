//! Shiftella — entry point. The game itself lives in the library (`src/lib.rs`):
//! [`shiftella_game::app::run`] boots, shows the menu and runs the state machine.

use macroquad::prelude::*;

/// Window size in logical points; `SHIFTELLA_WINDOW=WxH` overrides it (screenshots, QA).
fn window_size() -> (i32, i32) {
    std::env::var("SHIFTELLA_WINDOW")
        .ok()
        .and_then(|v| {
            let (w, h) = v.split_once('x')?;
            Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
        })
        .filter(|&(w, h): &(i32, i32)| w >= 640 && h >= 400)
        .unwrap_or((1280, 720))
}

fn window_conf() -> Conf {
    let (window_width, window_height) = window_size();
    Conf {
        window_title: "Shiftella".to_owned(),
        window_width,
        window_height,
        high_dpi: true, // crisp on Retina
        sample_count: std::env::var("SHIFTELLA_MSAA")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(4),
        ..Default::default()
    }
}

/// macOS drops the first click on an inactive window unless the view accepts it
/// (miniquad's view doesn't): add `acceptsFirstMouse:` → YES, so one click both focuses
/// the window and presses the button under the cursor.
#[cfg(target_os = "macos")]
fn accept_first_mouse() {
    use std::ffi::{c_char, c_void};
    unsafe extern "C" {
        fn objc_getClass(name: *const c_char) -> *mut c_void;
        fn sel_registerName(name: *const c_char) -> *mut c_void;
        fn class_addMethod(
            cls: *mut c_void,
            sel: *mut c_void,
            imp: *const c_void,
            types: *const c_char,
        ) -> i8;
    }
    extern "C" fn yes(_this: *mut c_void, _sel: *mut c_void, _event: *mut c_void) -> i8 {
        1
    }
    // SAFETY: plain Objective-C runtime calls with NUL-terminated names; the class is
    // miniquad's own view class, registered before the first frame.
    unsafe {
        let cls = objc_getClass(c"RenderViewClass".as_ptr());
        if !cls.is_null() {
            class_addMethod(
                cls,
                sel_registerName(c"acceptsFirstMouse:".as_ptr()),
                yes as *const c_void,
                c"c@:@".as_ptr(),
            );
        }
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    #[cfg(target_os = "macos")]
    accept_first_mouse();
    shiftella_game::app::run().await;
}
