//! VFS FFI surface: process-wide init, and core read/exists/write
//! operations against the global `vfs::VfsContext`.
//!
//! Unlike `Logger`/`DiagnosticBuilder`, there's no opaque handle here:
//! [`vfs_crate::VfsContext`] is already a process-wide singleton (a
//! `OnceLock`, set once via [`ffi_vfs_init`] and read from everywhere
//! after) — the Rust API has no "create a context" step to wrap, so
//! neither does this module. **`ffi_vfs_init` can only succeed once per
//! process** — a second call always fails with `VfsError::AlreadyInitialized`
//! (code 6). Call it once, at startup, before any other `ffi_vfs_*`
//! function.
//!
//! (This module is named `vfs` — matching `vfs_crate::file_manager`'s
//! shape closely — but the dependency itself is imported as
//! `vfs_crate` in `Cargo.toml`, because `use vfs::Foo;` written inside
//! a module that is *itself* named `vfs` would ambiguously resolve to
//! `crate::vfs` instead of the external crate.)
//!
//! ## Root / mode as `u8`
//! `Root` and `VfsMode` cross the boundary as `u8`, converted via
//! [`vfs_crate::Root::as_u8`]/`from_u8` and
//! [`vfs_crate::VfsMode::as_u8`]/`from_u8` — same convention as
//! [`log_core::Severity::as_u8`] in the `logger`/`diagnostic` modules:
//! no cbindgen dependency-parsing needed to expose a real C enum for a
//! small, closed set.
//!
//! ## Error reporting: thread-local last error, not a return payload
//! Every fallible function here returns `bool` for whether *the call*
//! succeeded; any actual result value comes back through an out
//! parameter (`ffi_vfs_exists`'s `out_exists`, `ffi_vfs_read_bytes`'s
//! `out_data`/`out_len`) — never packed into the return value itself,
//! so a meaningful `bool` result (like "does this file exist") is never
//! confused with call success/failure.
//!
//! On `false`, call [`ffi_vfs_last_error_code`] /
//! [`ffi_vfs_last_error_message`] for why. Both read from a
//! `thread_local!` slot that the most recent `ffi_vfs_*` call on the
//! *current thread* overwrites (cleared on entry, set only on that
//! call's own failure) — check them immediately after the call that
//! failed, before making another `ffi_vfs_*` call on the same thread.
//! `ffi_vfs_last_error_code` returns `255` when the most recent call
//! succeeded or none has been made yet (real codes only span `0..=7`,
//! see [`vfs_crate::VfsError::code`]).
//!
//! Malformed FFI arguments themselves (a null/non-UTF-8 pointer, an
//! out-of-range `root`/`mode` byte, a null required out-parameter) are
//! reported as `VfsError::InvalidPath` (code 4) — the closest existing
//! variant — rather than adding an FFI-only variant to
//! `vfs_crate::VfsError`.
//!
//! ## Scope of this first pass
//! `init`, `read_to_string`, `read_bytes`, `write_bytes`, `exists`.
//! `remove`, `list_dir`, `metadata`, `add_file`/`add_file_to`, and
//! `new_temp_path` all exist on `vfs_crate::file_manager` already and
//! are the same pattern to add on top of this — left out until there's
//! an actual caller for them (matching this crate's general "don't grow
//! the ABI ahead of a real need" approach — see `diagnostic`'s module
//! docs for the same call on `DiagnosticBuilder`'s optional fields).

use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use camino::Utf8Path;
use vfs_crate::{Root, VfsContext, VfsError, VfsMode, VfsPath};

thread_local! {
    static LAST_ERROR: RefCell<Option<VfsError>> = const { RefCell::new(None) };
}

fn set_last_error(err: VfsError) {
    LAST_ERROR.with(|slot| *slot.borrow_mut() = Some(err));
}

fn clear_last_error() {
    LAST_ERROR.with(|slot| *slot.borrow_mut() = None);
}

/// Converts a raw, caller-supplied C string into an `&str`, recording an
/// `InvalidPath` failure through [`set_last_error`] if it's null or not
/// valid UTF-8. Shared by every function below that takes a string
/// argument, instead of repeating the same null/UTF-8 check at each
/// call site.
///
/// # Safety
/// `ptr` must be null or point to a valid, NUL-terminated C string that
/// outlives the returned `&str`.
unsafe fn required_str<'a>(ptr: *const c_char, arg_name: &str) -> Result<&'a str, ()> {
    if ptr.is_null() {
        set_last_error(VfsError::InvalidPath(format!("{arg_name} was null")));
        return Err(());
    }
    // SAFETY: caller upholds the contract documented above.
    match unsafe { CStr::from_ptr(ptr) }.to_str() {
        Ok(s) => Ok(s),
        Err(_) => {
            set_last_error(VfsError::InvalidPath(format!(
                "{arg_name} was not valid UTF-8"
            )));
            Err(())
        }
    }
}

/// Converts a raw `root` byte into a [`Root`], recording an
/// `InvalidPath` failure through [`set_last_error`] if it's out of
/// range.
fn required_root(root: u8) -> Result<Root, ()> {
    match Root::from_u8(root) {
        Some(r) => Ok(r),
        None => {
            set_last_error(VfsError::InvalidPath(format!(
                "root byte {root} is not a valid Root (0-8, see Root::as_u8)"
            )));
            Err(())
        }
    }
}

/// Combines [`required_root`] and [`required_str`] into the
/// [`VfsPath`] every function below actually needs, running
/// [`VfsPath`]'s own validation (no absolute paths, no `..`
/// components) too.
///
/// # Safety
/// `rel` must be null or point to a valid, NUL-terminated UTF-8 C
/// string.
unsafe fn required_vfs_path(root: u8, rel: *const c_char) -> Result<VfsPath, ()> {
    let root = required_root(root)?;
    // SAFETY: forwarded from the caller's contract.
    let rel = unsafe { required_str(rel, "rel") }?;
    VfsPath::new(root, rel).map_err(set_last_error)
}

/// The [`VfsError::code`] of the most recent failing `ffi_vfs_*` call
/// on this thread, or `255` if the most recent call (on this thread)
/// succeeded or none has been made yet.
#[unsafe(no_mangle)]
pub extern "C" fn ffi_vfs_last_error_code() -> u8 {
    LAST_ERROR.with(|slot| slot.borrow().as_ref().map_or(u8::MAX, VfsError::code))
}

/// A heap-allocated, human-readable message for the most recent failing
/// `ffi_vfs_*` call on this thread (via [`VfsError`]'s `Display` impl),
/// or null if the most recent call succeeded or none has been made yet.
/// The caller MUST free a non-null result with [`crate::ffi_free_string`].
#[unsafe(no_mangle)]
pub extern "C" fn ffi_vfs_last_error_message() -> *mut c_char {
    LAST_ERROR.with(|slot| match slot.borrow().as_ref() {
        // A CString::new failure here (an interior NUL in the message
        // itself) is vanishingly unlikely and not actionable by the
        // caller either way -- fall back to null, same as "no error".
        Some(err) => CString::new(err.to_string())
            .map(CString::into_raw)
            .unwrap_or(std::ptr::null_mut()),
        None => std::ptr::null_mut(),
    })
}

/// Initializes the global [`VfsContext`] against the real OS filesystem
/// (see [`VfsContext::init`]). `project_root` is the directory each
/// registered root's dev path is relative to (see
/// `vfs_crate::root_registry::RootDescriptor`). `archive_path` is the
/// `.coreproject` file to serve packed roots from; may be null
/// (required only when `mode` is Release *and* at least one registered
/// root is packed). `mode` is `0` = Development, `1` = Release (see
/// [`VfsMode::as_u8`]).
///
/// **Can only succeed once per process** — a second call always fails
/// with `VfsError::AlreadyInitialized` (code 6).
///
/// # Safety
/// `project_root` must be a valid, NUL-terminated UTF-8 C string.
/// `archive_path` must be null or a valid, NUL-terminated UTF-8 C
/// string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_vfs_init(
    project_root: *const c_char,
    archive_path: *const c_char,
    mode: u8,
) -> bool {
    clear_last_error();

    // SAFETY: caller upholds the contract documented above.
    let Ok(project_root) = (unsafe { required_str(project_root, "project_root") }) else {
        return false;
    };
    let archive_path = if archive_path.is_null() {
        None
    } else {
        // SAFETY: caller upholds the contract documented above.
        match unsafe { required_str(archive_path, "archive_path") } {
            Ok(s) => Some(s),
            Err(()) => return false,
        }
    };
    let Some(mode) = VfsMode::from_u8(mode) else {
        set_last_error(VfsError::InvalidPath(format!(
            "mode byte {mode} is not a valid VfsMode (0 = Development, 1 = Release)"
        )));
        return false;
    };

    match VfsContext::init(
        Utf8Path::new(project_root),
        archive_path.map(Utf8Path::new),
        mode,
    ) {
        Ok(()) => true,
        Err(e) => {
            set_last_error(e);
            false
        }
    }
}

/// Reads `root:/rel` as UTF-8 text (see
/// [`vfs_crate::file_manager::read_to_string`]). Returns null on
/// failure — check [`ffi_vfs_last_error_code`]/[`ffi_vfs_last_error_message`],
/// which also cover a text file whose bytes happen to contain an
/// interior NUL (valid UTF-8, but not representable as a NUL-terminated
/// C string — use [`ffi_vfs_read_bytes`] instead for arbitrary content).
/// The caller MUST free a non-null result with [`crate::ffi_free_string`].
///
/// # Safety
/// `rel` must be a valid, NUL-terminated UTF-8 C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_vfs_read_to_string(root: u8, rel: *const c_char) -> *mut c_char {
    clear_last_error();
    // SAFETY: forwarded from the caller's contract.
    let Ok(path) = (unsafe { required_vfs_path(root, rel) }) else {
        return std::ptr::null_mut();
    };
    match vfs_crate::file_manager::read_to_string(&path) {
        Ok(s) => match CString::new(s) {
            Ok(c) => c.into_raw(),
            Err(_) => {
                set_last_error(VfsError::InvalidPath(
                    "file content contains an interior NUL byte -- use ffi_vfs_read_bytes instead"
                        .into(),
                ));
                std::ptr::null_mut()
            }
        },
        Err(e) => {
            set_last_error(e);
            std::ptr::null_mut()
        }
    }
}

/// Reads `root:/rel` as raw bytes (see
/// [`vfs_crate::file_manager::read_bytes`]). On success, `*out_data`/
/// `*out_len` are set to a heap-allocated buffer and its length; the
/// caller MUST free it with [`ffi_vfs_free_bytes`], passing back the
/// exact same length. On failure, `*out_data`/`*out_len` are left
/// untouched — check [`ffi_vfs_last_error_code`]/[`ffi_vfs_last_error_message`].
///
/// # Safety
/// `rel` must be a valid, NUL-terminated UTF-8 C string. `out_data` and
/// `out_len` must each be valid, non-null, writable pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_vfs_read_bytes(
    root: u8,
    rel: *const c_char,
    out_data: *mut *mut u8,
    out_len: *mut usize,
) -> bool {
    clear_last_error();
    if out_data.is_null() || out_len.is_null() {
        set_last_error(VfsError::InvalidPath("out_data/out_len was null".into()));
        return false;
    }
    // SAFETY: forwarded from the caller's contract.
    let Ok(path) = (unsafe { required_vfs_path(root, rel) }) else {
        return false;
    };
    match vfs_crate::file_manager::read_bytes(&path) {
        Ok(data) => {
            let boxed = data.into_boxed_slice();
            let len = boxed.len();
            let ptr = Box::into_raw(boxed).cast::<u8>();
            // SAFETY: caller guarantees out_data/out_len are valid,
            // writable, non-null pointers (checked above).
            unsafe {
                *out_data = ptr;
                *out_len = len;
            }
            true
        }
        Err(e) => {
            set_last_error(e);
            false
        }
    }
}

/// Frees a buffer previously returned by [`ffi_vfs_read_bytes`].
///
/// # Safety
/// `data`/`len` must be exactly the `*out_data`/`*out_len` pair a prior
/// [`ffi_vfs_read_bytes`] call wrote (not a pointer/length crafted any
/// other way), not yet freed. `data` may be null (a no-op) only when no
/// such call ever wrote a non-null pointer there.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_vfs_free_bytes(data: *mut u8, len: usize) {
    if data.is_null() {
        return;
    }
    // SAFETY: caller upholds the contract documented above -- `data`
    // originated from a `Box<[u8]>` of exactly `len` elements via
    // `ffi_vfs_read_bytes`.
    drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(data, len)) });
}

/// Writes `data` (`len` bytes) to `root:/rel`, creating or truncating it
/// as needed (see [`vfs_crate::file_manager::write_bytes`]). Returns
/// `false` on failure — check [`ffi_vfs_last_error_code`]/
/// [`ffi_vfs_last_error_message`].
///
/// # Safety
/// `rel` must be a valid, NUL-terminated UTF-8 C string. `data` must be
/// null only if `len` is `0`; otherwise it must point to at least `len`
/// readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_vfs_write_bytes(
    root: u8,
    rel: *const c_char,
    data: *const u8,
    len: usize,
) -> bool {
    clear_last_error();
    if data.is_null() && len != 0 {
        set_last_error(VfsError::InvalidPath(
            "data was null but len was not 0".into(),
        ));
        return false;
    }
    // SAFETY: forwarded from the caller's contract.
    let Ok(path) = (unsafe { required_vfs_path(root, rel) }) else {
        return false;
    };
    let slice: &[u8] = if len == 0 {
        &[]
    } else {
        // SAFETY: caller guarantees `data` points to at least `len`
        // readable bytes (null-with-zero-len handled above).
        unsafe { std::slice::from_raw_parts(data, len) }
    };
    match vfs_crate::file_manager::write_bytes(&path, slice) {
        Ok(()) => true,
        Err(e) => {
            set_last_error(e);
            false
        }
    }
}

/// Checks whether `root:/rel` exists (see
/// [`vfs_crate::file_manager::exists`]). On success, `*out_exists` is
/// set and this returns `true`. On failure (e.g. the VFS isn't
/// initialized, or `root`/`rel` is malformed), `*out_exists` is left
/// untouched and this returns `false` — check
/// [`ffi_vfs_last_error_code`]/[`ffi_vfs_last_error_message`]. Note the
/// split: the return value is always "did the check itself succeed",
/// never "does the file exist" — that answer is always in
/// `*out_exists`.
///
/// # Safety
/// `rel` must be a valid, NUL-terminated UTF-8 C string. `out_exists`
/// must be a valid, non-null, writable pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_vfs_exists(
    root: u8,
    rel: *const c_char,
    out_exists: *mut bool,
) -> bool {
    clear_last_error();
    if out_exists.is_null() {
        set_last_error(VfsError::InvalidPath("out_exists was null".into()));
        return false;
    }
    // SAFETY: forwarded from the caller's contract.
    let Ok(path) = (unsafe { required_vfs_path(root, rel) }) else {
        return false;
    };
    match vfs_crate::file_manager::exists(&path) {
        Ok(exists) => {
            // SAFETY: caller guarantees out_exists is valid, writable,
            // non-null (checked above).
            unsafe { *out_exists = exists };
            true
        }
        Err(e) => {
            set_last_error(e);
            false
        }
    }
}
