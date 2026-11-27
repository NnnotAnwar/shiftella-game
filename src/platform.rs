//! Invisible automation runs (screenshots, autoplay, QA sweeps).
//!
//! `SHIFTELLA_HIDDEN=1` — or any run with `SHIFTELLA_SHOT` / `SHIFTELLA_AUTOPLAY` set — must
//! not pop up a window or take focus from whoever is working on the Mac. Call
//! [`hide_for_automation`] from the window `Conf` function, i.e. **before** miniquad creates
//! the window; it is a no-op otherwise (and everywhere but macOS). `SHIFTELLA_HIDDEN=0` keeps
//! the window visible (to watch an autoplay run).
//!
//! On macOS it swaps a few AppKit methods for this process only:
//! * `-[NSApplication setActivationPolicy:]` → always *Accessory* (no Dock icon, not in ⌘-Tab);
//! * `-[NSRunningApplication activateWithOptions:]` and
//!   `-[NSApplication activateIgnoringOtherApps:]` → do nothing (never frontmost, no focus
//!   steal — miniquad activates the app on its first update);
//! * `-[NSWindow makeKeyAndOrderFront:]` / `orderFront:` → the window is made fully
//!   transparent, click-through, shadowless, kept out of Mission Control and ordered *behind*
//!   the other windows; it is never made key.
//!
//! The game still renders every frame into the window's GL drawable (miniquad drives drawing
//! from its own loop, not from `drawRect:`), so `get_screen_data()` returns the real image.
//! App Nap is held off so hidden runs keep their frame rate.

/// True when this run should stay invisible (see the module docs).
pub fn automation_hidden() -> bool {
    let var = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    match var("SHIFTELLA_HIDDEN").as_deref() {
        Some("0") => false,
        Some(_) => true,
        None => var("SHIFTELLA_SHOT").is_some() || var("SHIFTELLA_AUTOPLAY").is_some(),
    }
}

/// Makes this process's window invisible and focus-neutral when [`automation_hidden`].
/// Call before the window exists (from the macroquad `Conf` function). Idempotent.
pub fn hide_for_automation() {
    if !automation_hidden() {
        return;
    }
    #[cfg(target_os = "macos")]
    macos::install();
}

#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::{c_char, c_void};
    use std::sync::Once;
    use std::sync::atomic::{AtomicPtr, Ordering};

    type Id = *mut c_void;
    type Sel = *mut c_void;

    #[link(name = "AppKit", kind = "framework")]
    unsafe extern "C" {}

    unsafe extern "C" {
        fn objc_getClass(name: *const c_char) -> Id;
        fn sel_registerName(name: *const c_char) -> Sel;
        fn class_getInstanceMethod(cls: Id, sel: Sel) -> *mut c_void;
        fn method_getTypeEncoding(m: *mut c_void) -> *const c_char;
        fn class_replaceMethod(
            cls: Id,
            sel: Sel,
            imp: *const c_void,
            types: *const c_char,
        ) -> *const c_void;
        fn objc_msgSend();
    }

    /// NSApplicationActivationPolicyAccessory.
    const ACCESSORY: i64 = 1;

    static ORIG_POLICY: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
    static ORIG_ORDER_BACK: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

    fn sel(name: &std::ffi::CStr) -> Sel {
        // SAFETY: a NUL-terminated selector name.
        unsafe { sel_registerName(name.as_ptr()) }
    }

    /// Replaces `-[cls sel]` with `imp`, returning the previous implementation.
    fn swap(class: &std::ffi::CStr, name: &std::ffi::CStr, imp: *const c_void) -> *mut c_void {
        // SAFETY: Objective-C runtime calls on AppKit classes with their own type encodings.
        unsafe {
            let cls = objc_getClass(class.as_ptr());
            if cls.is_null() {
                return std::ptr::null_mut();
            }
            let s = sel(name);
            let m = class_getInstanceMethod(cls, s);
            if m.is_null() {
                return std::ptr::null_mut();
            }
            class_replaceMethod(cls, s, imp, method_getTypeEncoding(m)) as *mut c_void
        }
    }

    // ── the replacements (exact ObjC signatures: arm64 passes each by its real type) ──

    extern "C" fn set_policy(this: Id, cmd: Sel, _policy: i64) -> i8 {
        let orig = ORIG_POLICY.load(Ordering::Relaxed);
        if orig.is_null() {
            return 1;
        }
        // SAFETY: the original IMP of -[NSApplication setActivationPolicy:] (BOOL (id, SEL, NSInteger)).
        let f: extern "C" fn(Id, Sel, i64) -> i8 = unsafe { std::mem::transmute(orig) };
        f(this, cmd, ACCESSORY)
    }

    extern "C" fn activate_with_options(_this: Id, _cmd: Sel, _options: u64) -> i8 {
        1
    }

    extern "C" fn activate_ignoring(_this: Id, _cmd: Sel, _flag: i8) {}

    extern "C" fn order_front(this: Id, _cmd: Sel, _sender: Id) {
        // SAFETY: typed objc_msgSend calls on the NSWindow miniquad just created.
        unsafe {
            let send_f64: extern "C" fn(Id, Sel, f64) =
                std::mem::transmute(objc_msgSend as *const ());
            let send_bool: extern "C" fn(Id, Sel, i8) =
                std::mem::transmute(objc_msgSend as *const ());
            let send_u64: extern "C" fn(Id, Sel, u64) =
                std::mem::transmute(objc_msgSend as *const ());
            send_f64(this, sel(c"setAlphaValue:"), 0.0);
            send_bool(this, sel(c"setIgnoresMouseEvents:"), 1);
            send_bool(this, sel(c"setHasShadow:"), 0);
            // Transient | IgnoresCycle | FullScreenAuxiliary-free: out of Mission Control / ⌘-`
            send_u64(this, sel(c"setCollectionBehavior:"), (1 << 3) | (1 << 6));
            let orig = ORIG_ORDER_BACK.load(Ordering::Relaxed);
            if !orig.is_null() {
                let f: extern "C" fn(Id, Sel, Id) = std::mem::transmute(orig);
                f(this, sel(c"orderBack:"), std::ptr::null_mut());
            }
        }
    }

    /// Keeps App Nap from throttling a hidden run (`-[NSProcessInfo beginActivityWithOptions:reason:]`).
    fn hold_off_app_nap() {
        // SAFETY: typed objc_msgSend calls; the returned activity token is kept for the
        // process lifetime on purpose (never released).
        unsafe {
            let send_id: extern "C" fn(Id, Sel) -> Id =
                std::mem::transmute(objc_msgSend as *const ());
            let send_str: extern "C" fn(Id, Sel, *const c_char) -> Id =
                std::mem::transmute(objc_msgSend as *const ());
            let send_activity: extern "C" fn(Id, Sel, u64, Id) -> Id =
                std::mem::transmute(objc_msgSend as *const ());
            let pi_cls = objc_getClass(c"NSProcessInfo".as_ptr());
            let str_cls = objc_getClass(c"NSString".as_ptr());
            if pi_cls.is_null() || str_cls.is_null() {
                return;
            }
            let info = send_id(pi_cls, sel(c"processInfo"));
            let reason = send_str(
                str_cls,
                sel(c"stringWithUTF8String:"),
                c"Shiftella automation run".as_ptr(),
            );
            // NSActivityUserInitiatedAllowingIdleSystemSleep | NSActivityLatencyCritical
            let options: u64 = 0x00EF_FFFF | 0xFF_0000_0000;
            let _ = send_activity(
                info,
                sel(c"beginActivityWithOptions:reason:"),
                options,
                reason,
            );
        }
    }

    pub fn install() {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            // the original orderBack: (kept, not replaced) is what a hidden window uses
            // SAFETY: reading an existing IMP from the runtime.
            unsafe {
                let cls = objc_getClass(c"NSWindow".as_ptr());
                if !cls.is_null() {
                    let m = class_getInstanceMethod(cls, sel(c"orderBack:"));
                    if !m.is_null() {
                        unsafe extern "C" {
                            fn method_getImplementation(m: *mut c_void) -> *mut c_void;
                        }
                        ORIG_ORDER_BACK.store(method_getImplementation(m), Ordering::Relaxed);
                    }
                }
            }
            let p = swap(
                c"NSApplication",
                c"setActivationPolicy:",
                set_policy as *const c_void,
            );
            ORIG_POLICY.store(p, Ordering::Relaxed);
            swap(
                c"NSApplication",
                c"activateIgnoringOtherApps:",
                activate_ignoring as *const c_void,
            );
            swap(
                c"NSRunningApplication",
                c"activateWithOptions:",
                activate_with_options as *const c_void,
            );
            swap(
                c"NSWindow",
                c"makeKeyAndOrderFront:",
                order_front as *const c_void,
            );
            swap(c"NSWindow", c"orderFront:", order_front as *const c_void);
            hold_off_app_nap();
        });
    }
}

static POINTER_PARKED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Screenshot runs: the real desktop cursor no longer reaches the (invisible) window's UI or
/// world — it would hover whatever it happens to be over and make two runs of the same scene
/// differ. The pointer is then parked far off-screen.
pub fn park_pointer() {
    POINTER_PARKED.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// The mouse position in window points (`macroquad::input::mouse_position`), or far
/// off-screen in a screenshot run ([`park_pointer`]).
pub fn pointer() -> (f32, f32) {
    if POINTER_PARKED.load(std::sync::atomic::Ordering::Relaxed) {
        (-10_000.0, -10_000.0)
    } else {
        macroquad::input::mouse_position()
    }
}
