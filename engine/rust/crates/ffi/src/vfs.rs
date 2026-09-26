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
//! ## `list_dir` / string arrays
//! [`ffi_vfs_list_dir`] hands back a `char**` of `*out_count` entries
//! (each `rel()`'s worth — `root` is already the call's own `root`
//! argument, no point repeating it per entry) rather than a single
//! delimited string, so a directory entry containing any separator
//! byte can never be misparsed. There is no single-element analog to
//! [`crate::ffi_free_string`] for it: the caller MUST free the whole
//! array (elements and all) with [`ffi_vfs_free_string_array`], passing
//! back the exact same count, and MUST NOT free the individual
//! elements separately.
//!
//! ## `add_file`: one function, an optional destination
//! [`ffi_vfs_add_file`] covers both [`vfs_crate::file_manager::add_file`]
//! and [`vfs_crate::file_manager::add_file_to`] through one function —
//! `dest_rel == null` keeps the source file name (the `add_file`
//! behavior), non-null renames/relocates it within `dest_root` (the
//! `add_file_to` behavior). Same "null means absent, not an error"
//! convention as `diagnostic`'s optional fields (e.g.
//! `ffi_diagnostic_builder_set_line`).
//!
//! ## `new_temp_path`: no opaque `VfsPath` handle
//! [`ffi_vfs_new_temp_path`] returns the root/rel pair a fresh
//! [`VfsPath`] would carry (`*out_root`, `*out_rel`) rather than a new
//! opaque handle type — every function in this module already takes a
//! `VfsPath` apart into exactly that pair on the way in, so handing the
//! same pair back out keeps this module to one path representation
//! instead of two.

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

/// Removes `root:/rel` (see [`vfs_crate::file_manager::remove`]).
/// Returns `false` on failure — check
/// [`ffi_vfs_last_error_code`]/[`ffi_vfs_last_error_message`].
///
/// # Safety
/// `rel` must be a valid, NUL-terminated UTF-8 C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_vfs_remove(root: u8, rel: *const c_char) -> bool {
    clear_last_error();
    // SAFETY: forwarded from the caller's contract.
    let Ok(path) = (unsafe { required_vfs_path(root, rel) }) else {
        return false;
    };
    match vfs_crate::file_manager::remove(&path) {
        Ok(()) => true,
        Err(e) => {
            set_last_error(e);
            false
        }
    }
}

/// Reads `root:/rel`'s metadata (see
/// [`vfs_crate::file_manager::metadata`]). On success, `*out_is_dir`
/// and `*out_len` are set and this returns `true`. On failure,
/// both are left untouched — check
/// [`ffi_vfs_last_error_code`]/[`ffi_vfs_last_error_message`].
///
/// # Safety
/// `rel` must be a valid, NUL-terminated UTF-8 C string. `out_is_dir`
/// and `out_len` must each be valid, non-null, writable pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_vfs_metadata(
    root: u8,
    rel: *const c_char,
    out_is_dir: *mut bool,
    out_len: *mut u64,
) -> bool {
    clear_last_error();
    if out_is_dir.is_null() || out_len.is_null() {
        set_last_error(VfsError::InvalidPath(
            "out_is_dir/out_len was null".into(),
        ));
        return false;
    }
    // SAFETY: forwarded from the caller's contract.
    let Ok(path) = (unsafe { required_vfs_path(root, rel) }) else {
        return false;
    };
    match vfs_crate::file_manager::metadata(&path) {
        Ok(meta) => {
            // SAFETY: caller guarantees out_is_dir/out_len are valid,
            // writable, non-null (checked above).
            unsafe {
                *out_is_dir = meta.is_dir;
                *out_len = meta.len;
            }
            true
        }
        Err(e) => {
            set_last_error(e);
            false
        }
    }
}

/// Lists the entries of the directory `root:/rel` (see
/// [`vfs_crate::file_manager::list_dir`]). On success, `*out_entries`
/// is set to a heap-allocated array of `*out_count` heap-allocated,
/// NUL-terminated UTF-8 C strings (each entry's path relative to
/// `root`, matching what [`vfs_crate::VfsPath::rel`] would give for
/// it — `root` itself is not repeated per entry), and this returns
/// `true`. The caller MUST free it with [`ffi_vfs_free_string_array`],
/// passing back the exact same count — freeing the entries individually
/// (e.g. with [`crate::ffi_free_string`]) is not supported, since the
/// array itself is also heap-allocated and must go with them. On
/// failure, `*out_entries`/`*out_count` are left untouched — check
/// [`ffi_vfs_last_error_code`]/[`ffi_vfs_last_error_message`].
///
/// # Safety
/// `rel` must be a valid, NUL-terminated UTF-8 C string. `out_entries`
/// and `out_count` must each be valid, non-null, writable pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_vfs_list_dir(
    root: u8,
    rel: *const c_char,
    out_entries: *mut *mut *mut c_char,
    out_count: *mut usize,
) -> bool {
    clear_last_error();
    if out_entries.is_null() || out_count.is_null() {
        set_last_error(VfsError::InvalidPath(
            "out_entries/out_count was null".into(),
        ));
        return false;
    }
    // SAFETY: forwarded from the caller's contract.
    let Ok(path) = (unsafe { required_vfs_path(root, rel) }) else {
        return false;
    };
    match vfs_crate::file_manager::list_dir(&path) {
        Ok(entries) => {
            // Every entry's `rel` is plain camino UTF-8 text built from
            // path components the backend enumerated itself — never
            // caller-supplied, so an interior NUL here would mean a
            // corrupt backend, not a malformed request. Skip such an
            // entry rather than fail the whole listing over it.
            let mut strings: Vec<*mut c_char> = Vec::with_capacity(entries.len());
            for entry in entries {
                if let Ok(c) = CString::new(entry.rel().as_str()) {
                    strings.push(c.into_raw());
                }
            }
            let len = strings.len();
            let boxed = strings.into_boxed_slice();
            let ptr = Box::into_raw(boxed).cast::<*mut c_char>();
            // SAFETY: caller guarantees out_entries/out_count are
            // valid, writable, non-null (checked above).
            unsafe {
                *out_entries = ptr;
                *out_count = len;
            }
            true
        }
        Err(e) => {
            set_last_error(e);
            false
        }
    }
}

/// Frees an array previously returned by [`ffi_vfs_list_dir`].
///
/// # Safety
/// `entries`/`count` must be exactly the `*out_entries`/`*out_count`
/// pair a prior [`ffi_vfs_list_dir`] call wrote (not a pointer/length
/// crafted any other way), not yet freed. `entries` may be null (a
/// no-op) only when no such call ever wrote a non-null pointer there.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_vfs_free_string_array(entries: *mut *mut c_char, count: usize) {
    if entries.is_null() {
        return;
    }
    // SAFETY: caller upholds the contract documented above -- `entries`
    // originated from a `Box<[*mut c_char]>` of exactly `count`
    // elements via ffi_vfs_list_dir, and each element originated from
    // `CString::into_raw` there.
    let boxed = unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(entries, count)) };
    for ptr in boxed.iter() {
        if !ptr.is_null() {
            // SAFETY: each element is a live CString::into_raw pointer
            // from ffi_vfs_list_dir, per the contract above.
            drop(unsafe { CString::from_raw(*ptr) });
        }
    }
}

/// Copies `src` (read through the VFS's own [`fs::FileSystem`] — see
/// [`vfs_crate::file_manager::add_file_to`]) into `dest_root`. With
/// `dest_rel == null`, keeps `src`'s file name at `dest_root`'s top
/// level (see [`vfs_crate::file_manager::add_file`]); with a non-null
/// `dest_rel`, copies to that exact path within `dest_root` instead
/// (see [`vfs_crate::file_manager::add_file_to`]). Returns `false` on
/// failure — check
/// [`ffi_vfs_last_error_code`]/[`ffi_vfs_last_error_message`].
///
/// # Safety
/// `src` must be a valid, NUL-terminated UTF-8 C string. `dest_rel`
/// must be null or point to a valid, NUL-terminated UTF-8 C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_vfs_add_file(
    src: *const c_char,
    dest_root: u8,
    dest_rel: *const c_char,
) -> bool {
    clear_last_error();
    // SAFETY: forwarded from the caller's contract.
    let Ok(src) = (unsafe { required_str(src, "src") }) else {
        return false;
    };
    let Ok(dest_root) = required_root(dest_root) else {
        return false;
    };
    let src_path = Utf8Path::new(src);

    let result = if dest_rel.is_null() {
        vfs_crate::file_manager::add_file(src_path, dest_root)
    } else {
        // SAFETY: forwarded from the caller's contract.
        let Ok(dest_rel) = (unsafe { required_str(dest_rel, "dest_rel") }) else {
            return false;
        };
        match VfsPath::new(dest_root, dest_rel) {
            Ok(dest) => vfs_crate::file_manager::add_file_to(src_path, &dest),
            Err(e) => {
                set_last_error(e);
                return false;
            }
        }
    };

    match result {
        Ok(()) => true,
        Err(e) => {
            set_last_error(e);
            false
        }
    }
}

/// Builds a fresh scratch path under [`Root::Temp`], unique within
/// this process (see [`vfs_crate::file_manager::new_temp_path`]). On
/// success, `*out_root` and `*out_rel` are set (the latter to a
/// heap-allocated string the caller MUST free with
/// [`crate::ffi_free_string`]) and this returns `true`. On failure,
/// both are left untouched — check
/// [`ffi_vfs_last_error_code`]/[`ffi_vfs_last_error_message`].
///
/// # Safety
/// `name_hint` must be a valid, NUL-terminated UTF-8 C string.
/// `out_root` and `out_rel` must each be valid, non-null, writable
/// pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_vfs_new_temp_path(
    name_hint: *const c_char,
    out_root: *mut u8,
    out_rel: *mut *mut c_char,
) -> bool {
    clear_last_error();
    if out_root.is_null() || out_rel.is_null() {
        set_last_error(VfsError::InvalidPath(
            "out_root/out_rel was null".into(),
        ));
        return false;
    }
    // SAFETY: forwarded from the caller's contract.
    let Ok(name_hint) = (unsafe { required_str(name_hint, "name_hint") }) else {
        return false;
    };
    match vfs_crate::file_manager::new_temp_path(name_hint) {
        Ok(path) => match CString::new(path.rel().as_str()) {
            Ok(rel) => {
                // SAFETY: caller guarantees out_root/out_rel are
                // valid, writable, non-null (checked above).
                unsafe {
                    *out_root = path.root().as_u8();
                    *out_rel = rel.into_raw();
                }
                true
            }
            Err(_) => {
                // process id + name_hint: can't actually contain a NUL
                // unless name_hint itself smuggled one in past its own
                // UTF-8 check above, which can't happen either -- kept
                // for the same reason ffi_build_info_string keeps its
                // .expect() rather than silently returning garbage.
                set_last_error(VfsError::InvalidPath(
                    "temp path contains an interior NUL byte".into(),
                ));
                false
            }
        },
        Err(e) => {
            set_last_error(e);
            false
        }
    }
}
