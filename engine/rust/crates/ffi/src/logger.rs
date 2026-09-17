//! `Logger` FFI surface.
//!
//! [`Logger`] is the opaque handle: create it, attach one or more sinks
//! (console and/or file), optionally raise its minimum severity, then
//! hand it to [`crate::diagnostic::ffi_diagnostic_builder_emit`] to
//! actually emit diagnostics through it. Call [`ffi_logger_shutdown`]
//! before [`ffi_logger_destroy`] on a clean exit so non-persistent log
//! files get cleaned up.
//!
//! See the crate root docs for the null-pointer contract and the opaque
//! handle pattern this module follows.

use std::ffi::CStr;
use std::os::raw::c_char;

use log_sinks::{ConsoleSink, FileSink, NativeLogVfs};

/// Opaque FFI handle wrapping [`log_sinks::Logger`]. Defined locally
/// (rather than re-exporting `log_sinks::Logger` directly) — see the
/// crate root docs' "Opaque handles" section for why. C++ only ever
/// holds a `Logger*`; it never inspects or relies on this layout.
pub struct Logger(log_sinks::Logger);

impl Logger {
    /// Exposes the wrapped [`log_sinks::Logger`] to the rest of this
    /// crate — used by [`crate::diagnostic::ffi_diagnostic_builder_emit`]
    /// to call [`log_sinks::Logger::emit`]. Not part of the C ABI.
    pub(crate) fn inner(&self) -> &log_sinks::Logger {
        &self.0
    }
}

/// Creates a new [`Logger`] with no sinks attached yet — see
/// [`ffi_logger_add_console_sink`] / [`ffi_logger_add_file_sink`]. The
/// caller MUST eventually pass the returned pointer to exactly one call
/// of [`ffi_logger_destroy`].
///
/// # Safety
/// Always returns a valid, non-null pointer.
#[unsafe(no_mangle)]
pub extern "C" fn ffi_logger_create() -> *mut Logger {
    Box::into_raw(Box::new(Logger(log_sinks::Logger::new())))
}

/// Destroys a [`Logger`] previously returned by [`ffi_logger_create`],
/// dropping every sink it owns. This does not itself flush or clean up
/// sinks — call [`ffi_logger_shutdown`] first if that's needed.
///
/// # Safety
/// `logger` must be null (a no-op) or a pointer previously returned by
/// [`ffi_logger_create`] that has not yet been passed to this function.
/// Calling this twice on the same pointer, or passing any pointer not
/// obtained from [`ffi_logger_create`], is undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_logger_destroy(logger: *mut Logger) {
    if logger.is_null() {
        return;
    }
    // SAFETY: caller upholds the contract documented above.
    drop(unsafe { Box::from_raw(logger) });
}

/// Flushes every sink attached to `logger` (deletes any of its log
/// files that were never marked persistent). Call this once, before
/// [`ffi_logger_destroy`], on a clean engine/editor shutdown.
///
/// # Safety
/// `logger` must be a valid, non-null pointer previously returned by
/// [`ffi_logger_create`] and not yet passed to [`ffi_logger_destroy`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_logger_shutdown(logger: *mut Logger) {
    // SAFETY: caller guarantees `logger` is a live Logger.
    unsafe { &*logger }.inner().shutdown();
}

/// Attaches a [`ConsoleSink`] with its default configuration to
/// `logger` (color on, no severity floor of its own — see
/// [`ConsoleSink::new`]). Always returns `true` — kept `bool` rather
/// than `void` so this stays symmetric with
/// [`ffi_logger_add_file_sink`], which can genuinely fail.
///
/// # Safety
/// `logger` must be a valid, non-null pointer previously returned by
/// [`ffi_logger_create`] and not yet passed to [`ffi_logger_destroy`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_logger_add_console_sink(logger: *mut Logger) -> bool {
    // SAFETY: caller guarantees `logger` is a live Logger.
    unsafe { &*logger }.inner().add_sink(Box::new(ConsoleSink::new()));
    true
}

/// Attaches a [`FileSink`] rooted at `<project_root>/Logs` (via
/// [`NativeLogVfs`]) to `logger`, using the default routing rules.
/// Returns `false` — and attaches nothing — if `project_root` is null,
/// not valid UTF-8, or if the `Logs` directory couldn't be created or
/// its standard log files couldn't be cleared.
///
/// # Safety
/// `logger` must be a valid, non-null pointer previously returned by
/// [`ffi_logger_create`] and not yet passed to [`ffi_logger_destroy`].
/// `project_root` must be null or point to a valid, NUL-terminated
/// UTF-8 C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_logger_add_file_sink(
    logger: *mut Logger,
    project_root: *const c_char,
) -> bool {
    if project_root.is_null() {
        return false;
    }
    // SAFETY: caller guarantees `project_root` is a valid,
    // NUL-terminated C string.
    let Ok(project_root) = unsafe { CStr::from_ptr(project_root) }.to_str() else {
        return false;
    };
    let Ok(vfs) = NativeLogVfs::new(project_root) else {
        return false;
    };
    let Ok(sink) = FileSink::new(vfs) else {
        return false;
    };
    // SAFETY: caller guarantees `logger` is a live Logger.
    unsafe { &*logger }.inner().add_sink(Box::new(sink));
    true
}

/// Sets the minimum severity `logger` accepts before fanning a
/// diagnostic out to its sinks (individual sinks may apply their own,
/// stricter threshold on top of this). Returns `false` — leaving the
/// threshold unchanged — if `severity` isn't a valid severity byte
/// (0 = Trace .. 7 = Fatal; see [`log_core::Severity::from_u8`]).
///
/// # Safety
/// `logger` must be a valid, non-null pointer previously returned by
/// [`ffi_logger_create`] and not yet passed to [`ffi_logger_destroy`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_logger_set_min_severity(logger: *mut Logger, severity: u8) -> bool {
    let Some(severity) = log_core::Severity::from_u8(severity) else {
        return false;
    };
    // SAFETY: caller guarantees `logger` is a live Logger.
    unsafe { &*logger }.inner().set_min_severity(severity);
    true
}
