//! `ffi` — Coreverse's C ABI boundary.
//!
//! This is the *only* crate in the workspace that C++ (editor, renderer)
//! links against directly. Every other Rust system (`ccore`, and later
//! `vfs`, `log-core`, `log-sinks`, ...) stays pure, safe Rust; this
//! crate's job is only to re-expose the pieces C++ needs behind a thin
//! `extern "C"` surface, hand that surface to `cbindgen` to generate a C
//! header, and confine every `unsafe` block the FFI boundary requires to
//! one place.
//!
//! Building the header requires cbindgen >= 0.28 — earlier versions
//! can't parse the `#[unsafe(no_mangle)]` attribute syntax the 2024
//! edition mandates for `no_mangle`. Install with
//! `cargo install cbindgen --locked`, not an OS package manager (most
//! still ship something older).
//!
//! ## Adding a new export
//! 1. Write the wrapper here as `extern "C"`, `#[unsafe(no_mangle)]`.
//! 2. Keep it thin — real logic stays in the wrapped crate (`ccore`,
//!    etc.); this function should just translate types across the
//!    boundary (Rust `String`/`Option`/`Result` <-> C pointers/codes).
//! 3. Document the safety contract in a `# Safety` section — cbindgen
//!    carries doc comments through to the generated header, so this is
//!    the actual contract the C++ caller sees.
//! 4. Any heap allocation crossing the boundary needs a matching
//!    `ffi_free_*` function; never let C++ call `free()` on
//!    Rust-allocated memory or `Box`/`CString` free on C++-allocated
//!    memory — the allocators are not guaranteed to be the same one.
//!
//! ## Null-pointer contract
//! Across this whole crate, only the functions documented as destroying
//! or freeing something (`ffi_free_string`, `ffi_logger_destroy`,
//! `ffi_diagnostic_builder_destroy`, ...) treat a null pointer as a
//! no-op. Every other function requires a valid, non-null pointer for
//! any handle/owned-resource argument it takes — passing null there is
//! undefined behavior, not a recoverable error. (A `const T*` argument
//! that stands for `Option<T>`, like `DiagnosticBuilder`'s optional
//! `line`, is the one exception: there, null legitimately means "no
//! value" rather than misuse — see `diagnostic::ffi_diagnostic_builder_set_line`.)
//!
//! ## Opaque handles
//! `logger::Logger` and `diagnostic::DiagnosticBuilder` are C ABI opaque
//! handles: `Box::into_raw`/`Box::from_raw` pairs wrapping a real,
//! documented Rust type (`log_sinks::Logger`, `log_core::DiagnosticBuilder`).
//! Each wraps its inner type in a locally-defined struct instead of
//! re-exporting the inner crate's type directly, so cbindgen forward-declares
//! it as an opaque C struct without needing `parse.parse_deps` (this
//! crate's `cbindgen.toml` doesn't set it) to see into `log-sinks`/`log-core`.
//! C++ only ever holds the pointer; it never inspects the layout.
//!
//! This is also the pattern for any *stateful* type crossing the
//! boundary; `ffi_build_info_string` below predates it and stays a
//! stateless string return because `BuildInfo` has no state to own.

#![allow(
    unsafe_code,
    reason = "this crate is the project's designated unsafe FFI boundary; see module docs"
)]

/// `Logger` opaque handle: lifecycle, sink registration, severity
/// threshold. See the module docs for the full function list.
pub mod logger;

/// `DiagnosticBuilder` opaque handle: construction, field setters,
/// emit — plus runtime producer registration. See the module docs for
/// the full function list.
pub mod diagnostic;

/// VFS FFI surface: process-wide init plus core read/exists/write
/// operations against the global `vfs::VfsContext`. No opaque handle —
/// see the module docs for why, and for its own last-error convention
/// (distinct from this crate's other two modules, since VFS operations
/// have a real `enum`'s worth of distinct failure reasons).
pub mod vfs;

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Returns a heap-allocated, null-terminated C string describing this
/// build (git hash, profile, target, build date — see
/// [`ccore::BuildInfo`]). The caller MUST free it with
/// [`ffi_free_string`] — never with `free()` or any other allocator,
/// since Rust's global allocator produced it.
///
/// # Safety
/// Always returns a valid, non-null pointer to a NUL-terminated string.
/// The pointer stays valid until passed to [`ffi_free_string`].
#[unsafe(no_mangle)]
pub extern "C" fn ffi_build_info_string() -> *mut c_char {
    let info = ccore::build_info!();
    // BuildInfo::Display is a fixed format string over four
    // &'static str fields that all come from `env!`/`option_env!` at
    // compile time — none of them can contain an interior NUL, so
    // this can never actually fail.
    CString::new(info.to_string())
        .expect("BuildInfo::Display never produces an interior NUL byte")
        .into_raw()
}

/// Frees a string previously returned by [`ffi_build_info_string`]
/// (or any other `ffi_*` function documented as returning an
/// owned string).
///
/// # Safety
/// `ptr` must be null (a no-op) or a pointer previously returned by a
/// `ffi_*` function, not yet freed. Calling this twice on the same
/// pointer, or passing any pointer not obtained this way (including
/// one obtained from `malloc`/`new`), is undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    // SAFETY: caller upholds the contract documented above — `ptr`
    // originated from `CString::into_raw` and hasn't been freed yet.
    drop(unsafe { CString::from_raw(ptr) });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_info_string_round_trips() {
        let ptr = ffi_build_info_string();
        assert!(!ptr.is_null());
        // SAFETY: `ptr` was just returned by ffi_build_info_string
        // above and hasn't been freed.
        unsafe {
            let s = CStr::from_ptr(ptr).to_str().expect("valid UTF-8");
            assert!(!s.is_empty());
            ffi_free_string(ptr);
        }
    }

    #[test]
    fn free_string_is_a_noop_on_null() {
        // SAFETY: null is documented as an explicit no-op.
        unsafe { ffi_free_string(std::ptr::null_mut()) };
    }
}
