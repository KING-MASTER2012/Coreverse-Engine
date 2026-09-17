//! `DiagnosticBuilder` FFI surface, plus runtime producer registration.
//!
//! [`log_core::DiagnosticBuilder`] is a Rust-idiomatic, self-consuming
//! fluent builder (`fn module(mut self, ...) -> Self`, etc.) — that
//! shape doesn't survive the FFI boundary, where a handle's address
//! must stay stable across calls. [`DiagnosticBuilder`] here wraps it in
//! an `Option` and simulates in-place mutation on each setter: take the
//! inner builder out, move-call the real setter, put the result back.
//! The `Option` is only ever empty for the instant inside a single
//! setter call; if the crate root docs' safety contract is honored
//! (never touch a handle after it's been consumed), a caller can never
//! observe it empty.
//!
//! Only a deliberately small subset of `DiagnosticBuilder`'s fields are
//! exposed here (`module`, `file`, `line`, `persistent`) — `language`,
//! `column`, `suggestion`, `documentation`, and `extra` have no current
//! caller and are left out rather than growing the ABI surface ahead of
//! actual need. Adding one later is a single new setter function, same
//! pattern as the four below.
//!
//! See the crate root docs for the null-pointer contract and the opaque
//! handle pattern this module follows.

use std::ffi::CStr;
use std::os::raw::c_char;

use crate::logger::Logger;

/// Opaque FFI handle wrapping [`log_core::DiagnosticBuilder`]. See the
/// module docs for why it holds an `Option` instead of the builder
/// directly.
pub struct DiagnosticBuilder(Option<log_core::DiagnosticBuilder>);

/// Panic message used when a setter finds the wrapped builder already
/// gone — this can only happen if the caller used a `DiagnosticBuilder*`
/// after passing it to [`ffi_diagnostic_builder_emit`] or
/// [`ffi_diagnostic_builder_destroy`], which the crate root docs
/// document as undefined behavior already; this `.expect` exists to
/// fail loudly in debug builds rather than silently corrupt state.
const CONSUMED_BUILDER: &str = "DiagnosticBuilder used after being emitted or destroyed";

/// Creates a new [`DiagnosticBuilder`] wrapping
/// [`log_core::DiagnosticBuilder::new`]. Returns null if `producer`,
/// `category`, or `message` is null or not valid UTF-8, or if
/// `severity` isn't a valid severity byte (0 = Trace .. 7 = Fatal; see
/// [`log_core::Severity::from_u8`]). The caller MUST eventually pass the
/// returned pointer to exactly one of [`ffi_diagnostic_builder_emit`] or
/// [`ffi_diagnostic_builder_destroy`] — never both, never neither.
///
/// # Safety
/// `producer`, `category`, and `message` must each be null or point to
/// a valid, NUL-terminated UTF-8 C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_diagnostic_builder_create(
    producer: *const c_char,
    category: *const c_char,
    number: u32,
    severity: u8,
    message: *const c_char,
) -> *mut DiagnosticBuilder {
    if producer.is_null() || category.is_null() || message.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: all three pointers were just checked non-null; caller
    // guarantees each points to a valid, NUL-terminated C string.
    let Ok(producer) = (unsafe { CStr::from_ptr(producer) }).to_str() else {
        return std::ptr::null_mut();
    };
    let Ok(category) = (unsafe { CStr::from_ptr(category) }).to_str() else {
        return std::ptr::null_mut();
    };
    let Ok(message) = (unsafe { CStr::from_ptr(message) }).to_str() else {
        return std::ptr::null_mut();
    };
    let Some(severity) = log_core::Severity::from_u8(severity) else {
        return std::ptr::null_mut();
    };

    let inner = log_core::DiagnosticBuilder::new(producer, category, number, severity, message);
    Box::into_raw(Box::new(DiagnosticBuilder(Some(inner))))
}

/// Destroys a [`DiagnosticBuilder`] previously returned by
/// [`ffi_diagnostic_builder_create`] WITHOUT emitting it — use this to
/// abandon a diagnostic instead of calling
/// [`ffi_diagnostic_builder_emit`].
///
/// # Safety
/// `builder` must be null (a no-op) or a pointer previously returned by
/// [`ffi_diagnostic_builder_create`] that has not yet been passed to
/// this function or to [`ffi_diagnostic_builder_emit`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_diagnostic_builder_destroy(builder: *mut DiagnosticBuilder) {
    if builder.is_null() {
        return;
    }
    // SAFETY: caller upholds the contract documented above.
    drop(unsafe { Box::from_raw(builder) });
}

/// Sets the diagnostic's module name (see
/// [`log_core::DiagnosticBuilder::module`]). Returns `false` — leaving
/// `builder` unchanged — if `module` is null or not valid UTF-8.
///
/// # Safety
/// `builder` must be a valid, non-null pointer previously returned by
/// [`ffi_diagnostic_builder_create`] and not yet passed to
/// [`ffi_diagnostic_builder_emit`] or [`ffi_diagnostic_builder_destroy`].
/// `module` must be null or point to a valid, NUL-terminated UTF-8 C
/// string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_diagnostic_builder_set_module(
    builder: *mut DiagnosticBuilder,
    module: *const c_char,
) -> bool {
    if module.is_null() {
        return false;
    }
    // SAFETY: just checked non-null; caller guarantees a valid,
    // NUL-terminated C string.
    let Ok(module) = (unsafe { CStr::from_ptr(module) }).to_str() else {
        return false;
    };
    // SAFETY: caller guarantees `builder` is a live DiagnosticBuilder.
    let wrapper = unsafe { &mut *builder };
    let inner = wrapper.0.take().expect(CONSUMED_BUILDER);
    wrapper.0 = Some(inner.module(module));
    true
}

/// Sets the diagnostic's file path (see
/// [`log_core::DiagnosticBuilder::file`]). Returns `false` — leaving
/// `builder` unchanged — if `file` is null or not valid UTF-8.
///
/// # Safety
/// `builder` must be a valid, non-null pointer previously returned by
/// [`ffi_diagnostic_builder_create`] and not yet passed to
/// [`ffi_diagnostic_builder_emit`] or [`ffi_diagnostic_builder_destroy`].
/// `file` must be null or point to a valid, NUL-terminated UTF-8 C
/// string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_diagnostic_builder_set_file(
    builder: *mut DiagnosticBuilder,
    file: *const c_char,
) -> bool {
    if file.is_null() {
        return false;
    }
    // SAFETY: just checked non-null; caller guarantees a valid,
    // NUL-terminated C string.
    let Ok(file) = (unsafe { CStr::from_ptr(file) }).to_str() else {
        return false;
    };
    // SAFETY: caller guarantees `builder` is a live DiagnosticBuilder.
    let wrapper = unsafe { &mut *builder };
    let inner = wrapper.0.take().expect(CONSUMED_BUILDER);
    wrapper.0 = Some(inner.file(file));
    true
}

/// Sets the diagnostic's line number (see
/// [`log_core::DiagnosticBuilder::line`]). `line` represents
/// `Option<u32>`: a null pointer means "no line" (the default — a valid
/// no-op, not an error) and always returns `true`; a non-null pointer
/// sets the line to the pointed-to value.
///
/// # Safety
/// `builder` must be a valid, non-null pointer previously returned by
/// [`ffi_diagnostic_builder_create`] and not yet passed to
/// [`ffi_diagnostic_builder_emit`] or [`ffi_diagnostic_builder_destroy`].
/// `line` must be null or point to a valid `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_diagnostic_builder_set_line(
    builder: *mut DiagnosticBuilder,
    line: *const u32,
) -> bool {
    // SAFETY: caller guarantees `builder` is a live DiagnosticBuilder.
    let wrapper = unsafe { &mut *builder };
    let inner = wrapper.0.take().expect(CONSUMED_BUILDER);
    // SAFETY: caller guarantees `line` is null or points to a valid u32.
    wrapper.0 = Some(match unsafe { line.as_ref() } {
        Some(&line) => inner.line(line),
        None => inner,
    });
    true
}

/// Sets whether the diagnostic's target log file should survive a clean
/// logger shutdown (see [`log_core::DiagnosticBuilder::persistent`]).
/// Defaults to `false`.
///
/// # Safety
/// `builder` must be a valid, non-null pointer previously returned by
/// [`ffi_diagnostic_builder_create`] and not yet passed to
/// [`ffi_diagnostic_builder_emit`] or [`ffi_diagnostic_builder_destroy`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_diagnostic_builder_set_persistent(
    builder: *mut DiagnosticBuilder,
    persistent: bool,
) {
    // SAFETY: caller guarantees `builder` is a live DiagnosticBuilder.
    let wrapper = unsafe { &mut *builder };
    let inner = wrapper.0.take().expect(CONSUMED_BUILDER);
    wrapper.0 = Some(inner.persistent(persistent));
}

/// Builds the [`log_core::Diagnostic`] and emits it through `logger`
/// (see [`log_sinks::Logger::emit`]), consuming and freeing `builder` —
/// the pointer is invalid after this call regardless of the return
/// value, exactly as if it had been passed to
/// [`ffi_diagnostic_builder_destroy`]. Returns `false` only if `builder`
/// was null; does not reflect whether any sink accepted the diagnostic,
/// since `Logger::emit` itself never fails (a diagnostic below the
/// logger's minimum severity is silently dropped instead).
///
/// # Safety
/// `logger` must be a valid, non-null pointer previously returned by
/// [`crate::logger::ffi_logger_create`] and not yet passed to
/// [`crate::logger::ffi_logger_destroy`]. `builder` must be null (a
/// no-op returning `false`) or a pointer previously returned by
/// [`ffi_diagnostic_builder_create`] and not yet passed to this function
/// or to [`ffi_diagnostic_builder_destroy`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_diagnostic_builder_emit(
    logger: *mut Logger,
    builder: *mut DiagnosticBuilder,
) -> bool {
    if builder.is_null() {
        return false;
    }
    // SAFETY: caller upholds the contract documented above — `builder`
    // was returned by ffi_diagnostic_builder_create and not yet freed.
    let mut wrapper = unsafe { Box::from_raw(builder) };
    let inner = wrapper.0.take().expect(CONSUMED_BUILDER);
    // SAFETY: caller guarantees `logger` is a live Logger.
    unsafe { &*logger }.inner().emit(inner.build());
    true
}

/// Registers a producer at runtime (see
/// [`log_core::producer::register_dynamic`]) so its code can appear in
/// diagnostics built via [`ffi_diagnostic_builder_create`]. This is a
/// process-global registration, not tied to any particular `Logger`.
/// Returns `false` if any argument is null or not valid UTF-8, or if
/// `code` collides with an existing static or dynamic registration.
///
/// # Safety
/// `code`, `display_name`, and `organization` must each be null or
/// point to a valid, NUL-terminated UTF-8 C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_producer_register(
    code: *const c_char,
    display_name: *const c_char,
    organization: *const c_char,
) -> bool {
    if code.is_null() || display_name.is_null() || organization.is_null() {
        return false;
    }
    // SAFETY: all three pointers were just checked non-null; caller
    // guarantees each points to a valid, NUL-terminated C string.
    let Ok(code) = (unsafe { CStr::from_ptr(code) }).to_str() else {
        return false;
    };
    let Ok(display_name) = (unsafe { CStr::from_ptr(display_name) }).to_str() else {
        return false;
    };
    let Ok(organization) = (unsafe { CStr::from_ptr(organization) }).to_str() else {
        return false;
    };
    log_core::producer::register_dynamic(code, display_name, organization).is_ok()
}
