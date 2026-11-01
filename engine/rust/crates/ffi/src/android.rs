//! The Android entry point.
//!
//! An Android application is a `NativeActivity` (the `android-native-activity`
//! feature of the `platform` crate). The activity library does not call `main`:
//! it starts a thread and calls a Rust function named `android_main` with an
//! [`AndroidApp`], the handle to the activity that the window backend needs.
//! This crate defines `android_main` so that a C++ application does not have
//! to:
//!
//! ```text
//! NativeActivity ──► ANativeActivity_onCreate (android-activity) ──► thread
//!      thread ──► android_main(app)              (this module: keeps `app`)
//!              ──► cv_android_main()             (the C++ application)
//!              ──► cv_ffi::window::Run(...) ──► ffi_platform_run
//!                                                  uses the kept `app`
//! ```
//!
//! # What the C++ application provides and links
//! * `extern "C" int cv_android_main(void)`: the application's entry; the
//!   usual body builds its `cv_ffi::window::App` and calls `Run`. Its return
//!   value is only logged. A missing definition is a link error, on purpose.
//! * The final shared library must keep the symbol `ANativeActivity_onCreate`,
//!   which nothing else references and a static library does not bring in by
//!   itself: link with `-Wl,--undefined=ANativeActivity_onCreate` (the runtime's
//!   CMake does). Its name goes into the manifest as `android.app.lib_name`.
//!
//! `ffi_platform_run` has to be called on the thread `android_main` runs on,
//! which is the thread `cv_android_main` is called on.

use std::sync::Mutex;

use platform_crate::window::AndroidApp;

unsafe extern "C" {
    /// The C++ application's entry point; see the module docs.
    fn cv_android_main() -> i32;
}

/// The `AndroidApp` of the running activity, set for the duration of
/// `cv_android_main`.
static APP: Mutex<Option<AndroidApp>> = Mutex::new(None);

/// The `AndroidApp` that `android_main` received, while it runs.
pub(crate) fn current_app() -> Option<AndroidApp> {
    APP.lock().ok().and_then(|app| app.clone())
}

/// Called by the activity library on its own thread once the activity exists.
/// Keeps the `AndroidApp` and runs the C++ application until it returns; when
/// it does, the activity is finished.
#[unsafe(no_mangle)]
pub fn android_main(app: AndroidApp) {
    if let Ok(mut slot) = APP.lock() {
        *slot = Some(app);
    }
    // SAFETY: `cv_android_main` is defined by the application, which promises
    // the signature documented above and that no exception escapes it.
    let _exit_code = unsafe { cv_android_main() };
    if let Ok(mut slot) = APP.lock() {
        *slot = None;
    }
}
