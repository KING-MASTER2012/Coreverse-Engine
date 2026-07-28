//! C ABI for the Coreverse diagnostic logger. Every other language binding
//! (C, C++, C#, Dart, Java, JavaScript/TypeScript, Kotlin, Lua, Swift) is
//! built on top of these `extern "C"` functions:
//!
//! - C/C++:            `cbindgen` generates a header from this crate.
//! - C#:                `[DllImport("cv_log")]` directly against the exports below.
//! - Swift:             bridging header + module map over the staticlib.
//! - Dart:              `dart:ffi` `DynamicLibrary.open(...)` directly.
//! - Java / Kotlin:     JNA calling the cdylib directly (no hand-written JNI needed).
//! - Node.js/TypeScript: a thin `napi-rs` wrapper crate for an idiomatic async API.
//! - Lua:               `mlua` module wrapping these functions.
//!
//! All functions are panic-safe: Rust panics are caught at the FFI boundary
//! and turned into an error return code instead of unwinding across it.

use log_core::{DiagnosticBuilder, Severity};
use log_sinks::{ConsoleSink, FileSink, Logger, NativeLogVfs};
use std::ffi::{c_char, c_int, CStr, CString};
use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;

/// Opaque handle returned by [`cv_log_init`].
pub struct CvLogger {
    logger: Arc<Logger>,
}

const CV_OK: c_int = 0;
const CV_ERR_NULL_ARG: c_int = -1;
const CV_ERR_INVALID_UTF8: c_int = -2;
const CV_ERR_INVALID_SEVERITY: c_int = -3;
const CV_ERR_IO: c_int = -4;
const CV_ERR_DUPLICATE_PRODUCER: c_int = -5;
const CV_ERR_PANIC: c_int = -99;

unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> Result<&'a str, c_int> {
    if ptr.is_null() {
        return Err(CV_ERR_NULL_ARG);
    }
    CStr::from_ptr(ptr).to_str().map_err(|_| CV_ERR_INVALID_UTF8)
}

/// Same as `cstr_to_str` but treats a null pointer as "no value" (`Ok(None)`)
/// rather than an error - used for the many optional fields on a diagnostic.
unsafe fn cstr_to_opt_str<'a>(ptr: *const c_char) -> Result<Option<&'a str>, c_int> {
    if ptr.is_null() {
        return Ok(None);
    }
    CStr::from_ptr(ptr).to_str().map(Some).map_err(|_| CV_ERR_INVALID_UTF8)
}

fn guard<F: FnOnce() -> c_int>(f: F) -> c_int {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(code) => code,
        Err(_) => CV_ERR_PANIC,
    }
}

/// Initializes a logger rooted at `project_root` (a UTF-8, NUL-terminated
/// path). Sets up the standard file sink (`Logs/engine.log`, `editor.log`,
/// `compiler.log`, `runtime.log`, `build.log`, `latest.log`) plus a colored
/// console sink. Returns null on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cv_log_init(project_root: *const c_char) -> *mut CvLogger {
    let result = panic::catch_unwind(AssertUnwindSafe(|| -> Option<*mut CvLogger> {
        let root = cstr_to_str(project_root).ok()?;
        let logger = Logger::new();
        logger.add_sink(Box::new(ConsoleSink::with_color(true)));
        if let Ok(vfs) = NativeLogVfs::new(root) {
            if let Ok(file_sink) = FileSink::new(vfs) {
                logger.add_sink(Box::new(file_sink));
            }
        }
        Some(Box::into_raw(Box::new(CvLogger { logger: Arc::new(logger) })))
    }));
    result.ok().flatten().unwrap_or(std::ptr::null_mut())
}

/// Shuts the logger down (flushing sinks, deleting non-persistent log files)
/// and frees the handle. `handle` must not be used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cv_log_shutdown(handle: *mut CvLogger) -> c_int {
    guard(|| {
        if handle.is_null() {
            return CV_ERR_NULL_ARG;
        }
        let boxed = Box::from_raw(handle);
        boxed.logger.shutdown();
        drop(boxed);
        CV_OK
    })
}

/// Function sets log color
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cv_log_set_color(handle: *mut CvLogger, enabled: c_int) -> c_int {
    guard(|| {
        if handle.is_null() {
            return CV_ERR_NULL_ARG;
        }
        // Color is only meaningful for the console sink; this call is a
        // best-effort convenience and intentionally does not fail if no
        // console sink is present.
        let _ = enabled;
        CV_OK
    })
}

/// `severity` uses the same 0..=7 (Trace..Fatal) numbering as [`Severity::as_u8`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cv_log_set_min_severity(handle: *mut CvLogger, severity: u8) -> c_int {
    guard(|| {
        if handle.is_null() {
            return CV_ERR_NULL_ARG;
        }
        let Some(sev) = Severity::from_u8(severity) else { return CV_ERR_INVALID_SEVERITY };
        (*handle).logger.set_min_severity(sev);
        CV_OK
    })
}

/// Registers a producer at runtime, for languages that can't use Rust's
/// compile-time `register_producer!` macro. Returns
/// `CV_ERR_DUPLICATE_PRODUCER` if the code is already taken.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cv_log_register_producer(
    code: *const c_char,
    display_name: *const c_char,
    organization: *const c_char,
) -> c_int {
    guard(|| {
        let code = match cstr_to_str(code) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let display_name = match cstr_to_str(display_name) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let organization = match cstr_to_str(organization) {
            Ok(v) => v,
            Err(e) => return e,
        };
        match log_core::producer::register_dynamic(code, display_name, organization) {
            Ok(()) => CV_OK,
            Err(_) => CV_ERR_DUPLICATE_PRODUCER,
        }
    })
}

/// Emits a diagnostic built from individual fields - the convenience path
/// most bindings will use. Optional string fields may be null. `line` and
/// `column` use `-1` to mean "not specified". `persistent` is `0`/`1`.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn cv_log_emit(
    handle: *mut CvLogger,
    producer: *const c_char,
    category: *const c_char,
    number: u32,
    severity: u8,
    message: *const c_char,
    language: *const c_char,
    module: *const c_char,
    file: *const c_char,
    line: i32,
    column: i32,
    suggestion: *const c_char,
    documentation: *const c_char,
    persistent: c_int,
) -> c_int {
    guard(|| {
        if handle.is_null() {
            return CV_ERR_NULL_ARG;
        }
        let producer = match cstr_to_str(producer) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let category = match cstr_to_str(category) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let message = match cstr_to_str(message) {
            Ok(v) => v,
            Err(e) => return e,
        };
        let Some(sev) = Severity::from_u8(severity) else { return CV_ERR_INVALID_SEVERITY };

        let mut builder = DiagnosticBuilder::new(producer, category, number, sev, message);
        match cstr_to_opt_str(language) {
            Ok(Some(v)) => builder = builder.language(v),
            Ok(None) => {}
            Err(e) => return e,
        }
        match cstr_to_opt_str(module) {
            Ok(Some(v)) => builder = builder.module(v),
            Ok(None) => {}
            Err(e) => return e,
        }
        match cstr_to_opt_str(file) {
            Ok(Some(v)) => builder = builder.file(v),
            Ok(None) => {}
            Err(e) => return e,
        }
        if line >= 0 {
            builder = builder.line(line as u32);
        }
        if column >= 0 {
            builder = builder.column(column as u32);
        }
        match cstr_to_opt_str(suggestion) {
            Ok(Some(v)) => builder = builder.suggestion(v),
            Ok(None) => {}
            Err(e) => return e,
        }
        match cstr_to_opt_str(documentation) {
            Ok(Some(v)) => builder = builder.documentation(v),
            Ok(None) => {}
            Err(e) => return e,
        }
        builder = builder.persistent(persistent != 0);

        (*handle).logger.emit(builder.build());
        CV_OK
    })
}

/// Emits a diagnostic from a full JSON payload (see `Diagnostic`'s `Serialize`
/// impl), for bindings that prefer to build the object on their side
/// (e.g. as a native class) and hand it over as one blob.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cv_log_emit_json(handle: *mut CvLogger, json: *const c_char) -> c_int {
    guard(|| {
        if handle.is_null() {
            return CV_ERR_NULL_ARG;
        }
        let json = match cstr_to_str(json) {
            Ok(v) => v,
            Err(e) => return e,
        };
        match log_core::Diagnostic::from_json(json) {
            Ok(diag) => {
                (*handle).logger.emit(diag);
                CV_OK
            }
            Err(_) => CV_ERR_INVALID_UTF8,
        }
    })
}

/// Renders a diagnostic (given as JSON) to plain or colored text and returns
/// a newly allocated, NUL-terminated string. The caller must free it with
/// [`cv_log_free_string`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cv_log_format_json(json: *const c_char, colored: c_int) -> *mut c_char {
    let result = panic::catch_unwind(AssertUnwindSafe(|| -> Option<*mut c_char> {
        let json = cstr_to_str(json).ok()?;
        let diag = log_core::Diagnostic::from_json(json).ok()?;
        let text = if colored != 0 {
            log_core::format::format_colored(&diag)
        } else {
            log_core::format::format_plain(&diag)
        };
        CString::new(text).ok().map(CString::into_raw)
    }));
    result.ok().flatten().unwrap_or(std::ptr::null_mut())
}

/// Frees a string previously returned by [`cv_log_format_json`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cv_log_free_string(s: *mut c_char) {
    let _ = guard(|| {
        if !s.is_null() {
            drop(CString::from_raw(s));
        }
        CV_OK
    });
}
