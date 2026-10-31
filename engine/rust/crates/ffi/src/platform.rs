//! Platform FFI surface: system information and the standard directories of
//! an application, both backed by the `platform` crate.
//!
//! Two calls carry the data:
//!
//! * [`ffi_platform_info`] fills a [`PlatformInfo`] (release it with
//!   [`ffi_platform_info_free`]).
//! * [`ffi_platform_dir`] returns the path of one standard directory as an
//!   owned string (release it with [`crate::ffi_free_string`]).
//!
//! Nothing else of `platform` crosses the boundary on purpose: clocks,
//! timers, threads and processes are used from Rust, and C++ has its own
//! equivalents of those.
//!
//! (The dependency is imported as `platform_crate` in `Cargo.toml` for the
//! same reason `vfs` is imported as `vfs_crate`: this module is itself named
//! `platform`.)
//!
//! ## Enums as `u8`
//! Every enum crosses as a `u8`, like `Root` and `VfsMode` in the `vfs`
//! module and `Severity` in `logger`. The values are the `FFI_*` constants
//! below, which cbindgen copies into the header. A value this side does not
//! know is reported as the matching `*_UNKNOWN` constant (`255`), so the
//! `platform` enums can grow without breaking an older C++ caller.
//!
//! ## Unknown numbers are `0`
//! [`PlatformInfo::physical_cores`] and [`PlatformInfo::total_memory_bytes`]
//! are `0` when the platform does not report them. A machine never really has
//! zero physical cores or zero bytes of memory, so `0` cannot be confused
//! with a real value.
//!
//! ## Error reporting: thread-local last error
//! Same convention as the `vfs` module. A failing call returns `false` or
//! null; [`ffi_platform_last_error_code`] and
//! [`ffi_platform_last_error_message`] then say why. They read a
//! `thread_local!` slot that every `ffi_platform_*` call on the current
//! thread first clears and then fills only on its own failure, so check them
//! right after the call that failed. The code is [`FFI_PLATFORM_ERROR_NONE`]
//! (`255`) when the most recent call succeeded or none has been made yet.
//! Malformed arguments (a null or non-UTF-8 pointer, an unknown `kind` byte)
//! are [`FFI_PLATFORM_ERROR_INVALID_ARGUMENT`]; everything `platform` itself
//! reports keeps its own code.

use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::fmt;
use std::os::raw::c_char;

use ccore::platform::{Architecture, Endian, Platform};
use platform_crate::PlatformError;
use platform_crate::info::{DisplayServer, SystemInfo};
use platform_crate::paths::{AppDirs, DirKind};
use platform_crate::window::WindowError;

/// [`PlatformInfo::platform`]: Windows.
pub const FFI_PLATFORM_WINDOWS: u8 = 0;
/// [`PlatformInfo::platform`]: Linux.
pub const FFI_PLATFORM_LINUX: u8 = 1;
/// [`PlatformInfo::platform`]: macOS.
pub const FFI_PLATFORM_MACOS: u8 = 2;
/// [`PlatformInfo::platform`]: Android.
pub const FFI_PLATFORM_ANDROID: u8 = 3;
/// [`PlatformInfo::platform`]: iOS.
pub const FFI_PLATFORM_IOS: u8 = 4;
/// [`PlatformInfo::platform`]: any other operating system.
pub const FFI_PLATFORM_UNKNOWN: u8 = 255;

/// [`PlatformInfo::architecture`]: 32-bit x86.
pub const FFI_ARCH_X86: u8 = 0;
/// [`PlatformInfo::architecture`]: x86-64.
pub const FFI_ARCH_X86_64: u8 = 1;
/// [`PlatformInfo::architecture`]: 32-bit ARM.
pub const FFI_ARCH_ARM: u8 = 2;
/// [`PlatformInfo::architecture`]: 64-bit ARM.
pub const FFI_ARCH_ARM64: u8 = 3;
/// [`PlatformInfo::architecture`]: any other architecture.
pub const FFI_ARCH_UNKNOWN: u8 = 255;

/// [`PlatformInfo::endian`]: little endian.
pub const FFI_ENDIAN_LITTLE: u8 = 0;
/// [`PlatformInfo::endian`]: big endian.
pub const FFI_ENDIAN_BIG: u8 = 1;

/// [`PlatformInfo::display_server`]: Windows (Win32).
pub const FFI_DISPLAY_WIN32: u8 = 0;
/// [`PlatformInfo::display_server`]: macOS (Cocoa).
pub const FFI_DISPLAY_COCOA: u8 = 1;
/// [`PlatformInfo::display_server`]: X11.
pub const FFI_DISPLAY_X11: u8 = 2;
/// [`PlatformInfo::display_server`]: Wayland.
pub const FFI_DISPLAY_WAYLAND: u8 = 3;
/// [`PlatformInfo::display_server`]: Android.
pub const FFI_DISPLAY_ANDROID: u8 = 4;
/// [`PlatformInfo::display_server`]: iOS (UIKit).
pub const FFI_DISPLAY_UIKIT: u8 = 5;
/// [`PlatformInfo::display_server`]: no display (a session without one).
pub const FFI_DISPLAY_HEADLESS: u8 = 6;
/// [`PlatformInfo::display_server`]: could not be determined.
pub const FFI_DISPLAY_UNKNOWN: u8 = 255;

/// [`ffi_platform_dir`] `kind`: user-editable settings.
pub const FFI_DIR_CONFIG: u8 = 0;
/// [`ffi_platform_dir`] `kind`: persistent application data.
pub const FFI_DIR_DATA: u8 = 1;
/// [`ffi_platform_dir`] `kind`: data that can be recreated.
pub const FFI_DIR_CACHE: u8 = 2;
/// [`ffi_platform_dir`] `kind`: log files.
pub const FFI_DIR_LOG: u8 = 3;
/// [`ffi_platform_dir`] `kind`: scratch files.
pub const FFI_DIR_TEMP: u8 = 4;
/// [`ffi_platform_dir`] `kind`: the user's home directory.
pub const FFI_DIR_HOME: u8 = 5;
/// [`ffi_platform_dir`] `kind`: the directory of the running executable.
pub const FFI_DIR_EXECUTABLE_DIR: u8 = 6;

/// [`ffi_platform_last_error_code`]: the feature does not exist on this
/// platform (`PlatformError::Unsupported`).
pub const FFI_PLATFORM_ERROR_UNSUPPORTED: u8 = 0;
/// [`ffi_platform_last_error_code`]: a standard-library I/O error
/// (`PlatformError::Io`), also an unusable application name.
pub const FFI_PLATFORM_ERROR_IO: u8 = 1;
/// [`ffi_platform_last_error_code`]: an operating-system call failed
/// (`PlatformError::Os`).
pub const FFI_PLATFORM_ERROR_OS: u8 = 2;
/// [`ffi_platform_last_error_code`]: the OS returned a path that is not
/// valid UTF-8 (`PlatformError::NonUtf8Path`).
pub const FFI_PLATFORM_ERROR_NON_UTF8_PATH: u8 = 3;
/// [`ffi_platform_last_error_code`]: the caller passed a null or non-UTF-8
/// pointer or an unknown `kind` byte.
pub const FFI_PLATFORM_ERROR_INVALID_ARGUMENT: u8 = 4;
/// [`ffi_platform_last_error_code`]: the OS reported text that cannot cross
/// the boundary as a C string (an interior NUL byte).
pub const FFI_PLATFORM_ERROR_INVALID_DATA: u8 = 5;
/// [`ffi_platform_last_error_code`]: a `PlatformError` variant this header
/// does not know yet.
pub const FFI_PLATFORM_ERROR_OTHER: u8 = 6;
/// [`ffi_platform_last_error_code`]: a window operation failed (`WindowError`):
/// an unknown window id, an unusable window description, or the window system
/// could not be started. The message says which.
pub const FFI_PLATFORM_ERROR_WINDOW: u8 = 7;
/// [`ffi_platform_last_error_code`]: a Rust panic was caught at the boundary of
/// [`ffi_platform_run`](crate::platform_run::ffi_platform_run); the event loop
/// is gone.
pub const FFI_PLATFORM_ERROR_PANIC: u8 = 8;
/// [`ffi_platform_last_error_code`]: the most recent call succeeded, or none
/// has been made on this thread.
pub const FFI_PLATFORM_ERROR_NONE: u8 = 255;

/// What the operating system and hardware report, in a C layout.
///
/// Filled by [`ffi_platform_info`]; the three string fields are owned by the
/// struct, so release it with [`ffi_platform_info_free`] when done.
#[repr(C)]
pub struct PlatformInfo {
    /// One of the `FFI_PLATFORM_*` constants: the OS the library was compiled
    /// for.
    pub platform: u8,
    /// One of the `FFI_ARCH_*` constants: the CPU architecture the library was
    /// compiled for.
    pub architecture: u8,
    /// One of the `FFI_ENDIAN_*` constants.
    pub endian: u8,
    /// One of the `FFI_DISPLAY_*` constants: the display server of the
    /// current session.
    pub display_server: u8,
    /// Pointer width of the target in bits (32 or 64).
    pub pointer_width: u32,
    /// Logical cores this process may use, at least 1.
    pub logical_cores: u32,
    /// Physical cores of the machine, `0` when the platform does not report
    /// them.
    pub physical_cores: u32,
    /// Installed physical memory in bytes, `0` when unknown.
    pub total_memory_bytes: u64,
    /// OS product name (`"Windows"`, `"macOS"`, `"Ubuntu"`, ...), or null when
    /// unknown. Owned by the struct.
    pub os_name: *mut c_char,
    /// OS product version (`"24.04"`, `"14.5"`, ...), or null when unknown.
    /// Owned by the struct.
    pub os_version: *mut c_char,
    /// Kernel release (`uname -r`), or null when the platform has none.
    /// Owned by the struct.
    pub kernel: *mut c_char,
}

/// Why the most recent call on this thread failed.
#[derive(Debug)]
pub(crate) enum Failure {
    /// An error of the `platform` crate itself.
    Platform(PlatformError),
    /// A malformed argument from the caller.
    InvalidArgument(String),
    /// Text from the OS that cannot be represented as a C string.
    InvalidData(String),
    /// An error of the window API.
    Window(WindowError),
    /// A panic caught at the boundary; the text is its message.
    Panic(String),
}

impl Failure {
    pub(crate) fn code(&self) -> u8 {
        match self {
            Self::Platform(PlatformError::Unsupported { .. }) => FFI_PLATFORM_ERROR_UNSUPPORTED,
            Self::Platform(PlatformError::Io(_)) => FFI_PLATFORM_ERROR_IO,
            Self::Platform(PlatformError::Os { .. }) => FFI_PLATFORM_ERROR_OS,
            Self::Platform(PlatformError::NonUtf8Path(_)) => FFI_PLATFORM_ERROR_NON_UTF8_PATH,
            // `PlatformError` is `#[non_exhaustive]`.
            Self::Platform(_) => FFI_PLATFORM_ERROR_OTHER,
            Self::InvalidArgument(_) => FFI_PLATFORM_ERROR_INVALID_ARGUMENT,
            Self::InvalidData(_) => FFI_PLATFORM_ERROR_INVALID_DATA,
            Self::Window(_) => FFI_PLATFORM_ERROR_WINDOW,
            Self::Panic(_) => FFI_PLATFORM_ERROR_PANIC,
        }
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Platform(err) => err.fmt(f),
            Self::Window(err) => err.fmt(f),
            Self::InvalidArgument(message) | Self::InvalidData(message) | Self::Panic(message) => {
                f.write_str(message)
            }
        }
    }
}

thread_local! {
    static LAST_ERROR: RefCell<Option<Failure>> = const { RefCell::new(None) };
}

pub(crate) fn set_last_error(failure: Failure) {
    LAST_ERROR.with(|slot| *slot.borrow_mut() = Some(failure));
}

pub(crate) fn clear_last_error() {
    LAST_ERROR.with(|slot| *slot.borrow_mut() = None);
}

const fn platform_code(platform: Platform) -> u8 {
    match platform {
        Platform::Windows => FFI_PLATFORM_WINDOWS,
        Platform::Linux => FFI_PLATFORM_LINUX,
        Platform::MacOS => FFI_PLATFORM_MACOS,
        Platform::Android => FFI_PLATFORM_ANDROID,
        Platform::IOS => FFI_PLATFORM_IOS,
        Platform::Unknown => FFI_PLATFORM_UNKNOWN,
    }
}

const fn architecture_code(architecture: Architecture) -> u8 {
    match architecture {
        Architecture::X86 => FFI_ARCH_X86,
        Architecture::X86_64 => FFI_ARCH_X86_64,
        Architecture::Arm => FFI_ARCH_ARM,
        Architecture::Arm64 => FFI_ARCH_ARM64,
        Architecture::Unknown => FFI_ARCH_UNKNOWN,
    }
}

const fn endian_code(endian: Endian) -> u8 {
    match endian {
        Endian::Little => FFI_ENDIAN_LITTLE,
        Endian::Big => FFI_ENDIAN_BIG,
    }
}

const fn display_server_code(display_server: DisplayServer) -> u8 {
    match display_server {
        DisplayServer::Win32 => FFI_DISPLAY_WIN32,
        DisplayServer::Cocoa => FFI_DISPLAY_COCOA,
        DisplayServer::X11 => FFI_DISPLAY_X11,
        DisplayServer::Wayland => FFI_DISPLAY_WAYLAND,
        DisplayServer::Android => FFI_DISPLAY_ANDROID,
        DisplayServer::UiKit => FFI_DISPLAY_UIKIT,
        DisplayServer::Headless => FFI_DISPLAY_HEADLESS,
        // `DisplayServer` is `#[non_exhaustive]`: `Unknown` and any variant
        // added later.
        _ => FFI_DISPLAY_UNKNOWN,
    }
}

const fn dir_kind_from_code(kind: u8) -> Option<DirKind> {
    match kind {
        FFI_DIR_CONFIG => Some(DirKind::Config),
        FFI_DIR_DATA => Some(DirKind::Data),
        FFI_DIR_CACHE => Some(DirKind::Cache),
        FFI_DIR_LOG => Some(DirKind::Log),
        FFI_DIR_TEMP => Some(DirKind::Temp),
        FFI_DIR_HOME => Some(DirKind::Home),
        FFI_DIR_EXECUTABLE_DIR => Some(DirKind::ExecutableDir),
        _ => None,
    }
}

/// Converts a count to `u32`, saturating (no real machine gets near it).
fn saturating_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// Converts an optional OS string to an owned C string pointer, null when
/// absent. An interior NUL byte is an error instead of a silent truncation.
fn optional_c_string(value: Option<&str>, what: &str) -> Result<*mut c_char, Failure> {
    match value {
        None => Ok(std::ptr::null_mut()),
        Some(text) => CString::new(text)
            .map(CString::into_raw)
            .map_err(|_| Failure::InvalidData(format!("{what} contains an interior NUL byte"))),
    }
}

/// Frees a pointer made by [`optional_c_string`].
fn free_c_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        // SAFETY: every non-null pointer in a `PlatformInfo` came from
        // `CString::into_raw` in `optional_c_string` and is reset to null
        // when freed, so it is freed exactly once.
        drop(unsafe { CString::from_raw(ptr) });
    }
}

/// The `info.os` fields that become strings, as a `Result` so `?` can be used.
fn build_info(info: &SystemInfo) -> Result<PlatformInfo, Failure> {
    let os_name = optional_c_string(info.os.as_ref().map(|os| os.name.as_str()), "OS name")?;
    let os_version =
        match optional_c_string(info.os.as_ref().map(|os| os.version.as_str()), "OS version") {
            Ok(ptr) => ptr,
            Err(e) => {
                free_c_string(os_name);
                return Err(e);
            }
        };
    let kernel = match optional_c_string(
        info.os.as_ref().and_then(|os| os.kernel.as_deref()),
        "kernel release",
    ) {
        Ok(ptr) => ptr,
        Err(e) => {
            free_c_string(os_name);
            free_c_string(os_version);
            return Err(e);
        }
    };

    Ok(PlatformInfo {
        platform: platform_code(info.platform),
        architecture: architecture_code(info.architecture),
        endian: endian_code(info.endian),
        display_server: display_server_code(info.display_server),
        pointer_width: info.pointer_width,
        logical_cores: saturating_u32(info.cpu.logical_cores),
        physical_cores: info.cpu.physical_cores.map_or(0, saturating_u32),
        total_memory_bytes: info.total_memory_bytes.unwrap_or(0),
        os_name,
        os_version,
        kernel,
    })
}

/// The `FFI_PLATFORM_ERROR_*` code of the most recent failing
/// `ffi_platform_*` call on this thread, or `255`
/// ([`FFI_PLATFORM_ERROR_NONE`]) if the most recent call succeeded or none has
/// been made yet.
#[unsafe(no_mangle)]
pub extern "C" fn ffi_platform_last_error_code() -> u8 {
    LAST_ERROR.with(|slot| {
        slot.borrow()
            .as_ref()
            .map_or(FFI_PLATFORM_ERROR_NONE, Failure::code)
    })
}

/// A heap-allocated, human-readable message for the most recent failing
/// `ffi_platform_*` call on this thread, or null if the most recent call
/// succeeded or none has been made yet. The caller MUST free a non-null
/// result with [`crate::ffi_free_string`].
#[unsafe(no_mangle)]
pub extern "C" fn ffi_platform_last_error_message() -> *mut c_char {
    LAST_ERROR.with(|slot| match slot.borrow().as_ref() {
        // A CString::new failure (an interior NUL in the message) is not
        // actionable by the caller; fall back to null like "no error".
        Some(failure) => CString::new(failure.to_string())
            .map(CString::into_raw)
            .unwrap_or(std::ptr::null_mut()),
        None => std::ptr::null_mut(),
    })
}

/// Collects the system information (`platform::info::SystemInfo::collect`)
/// into `*out`. Returns `false` and leaves `*out` untouched when an OS-reported
/// string cannot be represented as a C string (see
/// [`ffi_platform_last_error_code`]); a platform that merely cannot report
/// something leaves the field at `0` or null instead.
///
/// On success the caller owns the strings in `*out` and MUST release them
/// with [`ffi_platform_info_free`] — never with `free()`.
///
/// # Safety
/// `out` must be a valid, non-null, writable pointer to a `PlatformInfo`. Its
/// previous content is overwritten without being freed, so it must not own
/// anything.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_platform_info(out: *mut PlatformInfo) -> bool {
    clear_last_error();
    if out.is_null() {
        set_last_error(Failure::InvalidArgument("out was null".into()));
        return false;
    }
    match build_info(&SystemInfo::collect()) {
        Ok(info) => {
            // SAFETY: `out` is valid and writable per the contract above.
            // `write` does not drop the previous content, which the
            // contract says owns nothing.
            unsafe { out.write(info) };
            true
        }
        Err(failure) => {
            set_last_error(failure);
            false
        }
    }
}

/// Releases the strings of a [`PlatformInfo`] filled by
/// [`ffi_platform_info`] and sets the three pointers to null, so a second
/// call on the same struct is harmless. Null `info` is a no-op.
///
/// # Safety
/// `info` must be null or point to a `PlatformInfo` whose string fields are
/// null or exactly the pointers [`ffi_platform_info`] wrote (and not freed
/// through anything else).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_platform_info_free(info: *mut PlatformInfo) {
    if info.is_null() {
        return;
    }
    // SAFETY: `info` is non-null and valid per the contract above.
    let info = unsafe { &mut *info };
    free_c_string(info.os_name);
    free_c_string(info.os_version);
    free_c_string(info.kernel);
    info.os_name = std::ptr::null_mut();
    info.os_version = std::ptr::null_mut();
    info.kernel = std::ptr::null_mut();
}

/// The path of one standard directory of the application `app` (see
/// `platform::paths::AppDirs`), as a UTF-8 string. `kind` is one of the
/// `FFI_DIR_*` constants. The directory is not created.
///
/// Returns null on failure — check [`ffi_platform_last_error_code`] and
/// [`ffi_platform_last_error_message`]; `FFI_PLATFORM_ERROR_UNSUPPORTED` means
/// the platform has no standard directories (Android, iOS). The caller MUST
/// free a non-null result with [`crate::ffi_free_string`].
///
/// `app` is also required for `FFI_DIR_HOME` and `FFI_DIR_EXECUTABLE_DIR`,
/// which do not depend on it.
///
/// # Safety
/// `app` must be null (reported as an error) or a valid, NUL-terminated C
/// string that outlives the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_platform_dir(kind: u8, app: *const c_char) -> *mut c_char {
    clear_last_error();

    let Some(kind) = dir_kind_from_code(kind) else {
        set_last_error(Failure::InvalidArgument(format!(
            "kind byte {kind} is not a valid directory kind (0-6, see the FFI_DIR_* constants)"
        )));
        return std::ptr::null_mut();
    };
    if app.is_null() {
        set_last_error(Failure::InvalidArgument("app was null".into()));
        return std::ptr::null_mut();
    }
    // SAFETY: `app` is non-null and, per the contract above, a valid C string.
    let Ok(app) = unsafe { CStr::from_ptr(app) }.to_str() else {
        set_last_error(Failure::InvalidArgument("app was not valid UTF-8".into()));
        return std::ptr::null_mut();
    };

    let path = match AppDirs::new(app).and_then(|dirs| dirs.dir(kind)) {
        Ok(path) => path,
        Err(e) => {
            set_last_error(Failure::Platform(e));
            return std::ptr::null_mut();
        }
    };
    match CString::new(path.into_string()) {
        Ok(c) => c.into_raw(),
        Err(_) => {
            set_last_error(Failure::InvalidData(
                "directory path contains an interior NUL byte".into(),
            ));
            std::ptr::null_mut()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi_free_string;

    /// Takes ownership of a string returned by this module.
    fn take(ptr: *mut c_char) -> Option<String> {
        if ptr.is_null() {
            return None;
        }
        // SAFETY: `ptr` is a live pointer returned by this crate.
        let text = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap().to_owned();
        // SAFETY: freed once, right after the copy above.
        unsafe { ffi_free_string(ptr) };
        Some(text)
    }

    fn last_error() -> (u8, Option<String>) {
        (
            ffi_platform_last_error_code(),
            take(ffi_platform_last_error_message()),
        )
    }

    fn dir(kind: u8, app: &str) -> *mut c_char {
        let app = CString::new(app).unwrap();
        // SAFETY: `app` is a valid C string for the duration of the call.
        unsafe { ffi_platform_dir(kind, app.as_ptr()) }
    }

    fn empty_info() -> PlatformInfo {
        PlatformInfo {
            platform: 0,
            architecture: 0,
            endian: 0,
            display_server: 0,
            pointer_width: 0,
            logical_cores: 0,
            physical_cores: 0,
            total_memory_bytes: 0,
            os_name: std::ptr::null_mut(),
            os_version: std::ptr::null_mut(),
            kernel: std::ptr::null_mut(),
        }
    }

    #[test]
    fn enum_codes_match_the_documented_values() {
        assert_eq!(platform_code(Platform::Windows), 0);
        assert_eq!(platform_code(Platform::Linux), 1);
        assert_eq!(platform_code(Platform::MacOS), 2);
        assert_eq!(platform_code(Platform::Android), 3);
        assert_eq!(platform_code(Platform::IOS), 4);
        assert_eq!(platform_code(Platform::Unknown), 255);

        assert_eq!(architecture_code(Architecture::X86), 0);
        assert_eq!(architecture_code(Architecture::X86_64), 1);
        assert_eq!(architecture_code(Architecture::Arm), 2);
        assert_eq!(architecture_code(Architecture::Arm64), 3);
        assert_eq!(architecture_code(Architecture::Unknown), 255);

        assert_eq!(endian_code(Endian::Little), 0);
        assert_eq!(endian_code(Endian::Big), 1);

        assert_eq!(display_server_code(DisplayServer::Win32), 0);
        assert_eq!(display_server_code(DisplayServer::Cocoa), 1);
        assert_eq!(display_server_code(DisplayServer::X11), 2);
        assert_eq!(display_server_code(DisplayServer::Wayland), 3);
        assert_eq!(display_server_code(DisplayServer::Android), 4);
        assert_eq!(display_server_code(DisplayServer::UiKit), 5);
        assert_eq!(display_server_code(DisplayServer::Headless), 6);
        assert_eq!(display_server_code(DisplayServer::Unknown), 255);
    }

    #[test]
    fn dir_kind_codes_cover_every_kind_and_nothing_else() {
        let expected = [
            (FFI_DIR_CONFIG, DirKind::Config),
            (FFI_DIR_DATA, DirKind::Data),
            (FFI_DIR_CACHE, DirKind::Cache),
            (FFI_DIR_LOG, DirKind::Log),
            (FFI_DIR_TEMP, DirKind::Temp),
            (FFI_DIR_HOME, DirKind::Home),
            (FFI_DIR_EXECUTABLE_DIR, DirKind::ExecutableDir),
        ];
        for (code, kind) in expected {
            assert_eq!(dir_kind_from_code(code), Some(kind));
        }
        assert_eq!(dir_kind_from_code(7), None);
        assert_eq!(dir_kind_from_code(255), None);
    }

    #[test]
    fn info_matches_the_platform_crate() {
        let mut info = empty_info();
        // SAFETY: `info` is a valid, writable struct that owns nothing.
        assert!(unsafe { ffi_platform_info(&mut info) });
        assert_eq!(ffi_platform_last_error_code(), FFI_PLATFORM_ERROR_NONE);

        let expected = SystemInfo::collect();
        assert_eq!(info.platform, platform_code(Platform::current()));
        assert_eq!(
            info.architecture,
            architecture_code(Architecture::current())
        );
        assert_eq!(info.endian, endian_code(Endian::current()));
        assert_eq!(info.pointer_width, usize::BITS);
        assert!(info.logical_cores >= 1);
        assert_eq!(
            info.os_name.is_null(),
            expected.os.is_none(),
            "name is present exactly when the OS version is known"
        );
        assert_eq!(info.os_version.is_null(), expected.os.is_none());
        if let Some(os) = &expected.os {
            // SAFETY: `info` was filled by `ffi_platform_info` and not freed.
            let name = unsafe { CStr::from_ptr(info.os_name) }.to_str().unwrap();
            assert_eq!(name, os.name);
        }

        // SAFETY: `info` was filled by `ffi_platform_info`.
        unsafe { ffi_platform_info_free(&mut info) };
        assert!(info.os_name.is_null());
        assert!(info.os_version.is_null());
        assert!(info.kernel.is_null());
        // A second free is harmless.
        // SAFETY: all string fields are null now.
        unsafe { ffi_platform_info_free(&mut info) };
    }

    #[test]
    fn info_rejects_a_null_out_pointer() {
        // SAFETY: null `out` is documented as an error, not UB.
        assert!(!unsafe { ffi_platform_info(std::ptr::null_mut()) });
        let (code, message) = last_error();
        assert_eq!(code, FFI_PLATFORM_ERROR_INVALID_ARGUMENT);
        assert!(message.unwrap().contains("out"));
    }

    #[test]
    fn info_free_is_a_noop_on_null() {
        // SAFETY: null is documented as a no-op.
        unsafe { ffi_platform_info_free(std::ptr::null_mut()) };
    }

    #[test]
    fn dir_agrees_with_app_dirs_for_every_kind() {
        for kind in 0..=6u8 {
            let from_ffi = take(dir(kind, "ffi-test"));
            let from_crate = AppDirs::new("ffi-test")
                .and_then(|dirs| dirs.dir(dir_kind_from_code(kind).unwrap()));
            match (from_ffi, from_crate) {
                (Some(ffi), Ok(path)) => {
                    assert_eq!(ffi, path.as_str(), "kind {kind}");
                    assert_eq!(ffi_platform_last_error_code(), FFI_PLATFORM_ERROR_NONE);
                }
                (None, Err(err)) => {
                    let (code, message) = last_error();
                    assert_eq!(code, Failure::Platform(err).code(), "kind {kind}");
                    assert!(!message.unwrap().is_empty());
                }
                (ffi, crate_result) => {
                    panic!("kind {kind}: ffi {ffi:?} disagrees with crate {crate_result:?}")
                }
            }
        }
    }

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    #[test]
    fn config_dir_carries_organization_and_app_name() {
        let path = take(dir(FFI_DIR_CONFIG, "ffi-test")).expect("desktop platforms have dirs");
        let lower = path.to_lowercase();
        assert!(lower.contains("coreverse"), "{path}");
        assert!(path.contains("ffi-test"), "{path}");
    }

    #[cfg(any(target_os = "android", target_os = "ios"))]
    #[test]
    fn mobile_platforms_report_unsupported() {
        assert!(dir(FFI_DIR_DATA, "ffi-test").is_null());
        assert_eq!(
            ffi_platform_last_error_code(),
            FFI_PLATFORM_ERROR_UNSUPPORTED
        );
    }

    #[test]
    fn dir_rejects_an_unknown_kind() {
        assert!(dir(7, "ffi-test").is_null());
        let (code, message) = last_error();
        assert_eq!(code, FFI_PLATFORM_ERROR_INVALID_ARGUMENT);
        assert!(message.unwrap().contains('7'));
    }

    #[test]
    fn dir_rejects_a_null_app() {
        // SAFETY: null `app` is documented as an error, not UB.
        assert!(unsafe { ffi_platform_dir(FFI_DIR_DATA, std::ptr::null()) }.is_null());
        assert_eq!(
            ffi_platform_last_error_code(),
            FFI_PLATFORM_ERROR_INVALID_ARGUMENT
        );
    }

    #[test]
    fn dir_rejects_a_non_utf8_app() {
        let bytes = [0xFFu8, 0xFE, 0x00];
        // SAFETY: `bytes` is NUL-terminated and outlives the call.
        let ptr = unsafe { ffi_platform_dir(FFI_DIR_DATA, bytes.as_ptr().cast()) };
        assert!(ptr.is_null());
        assert_eq!(
            ffi_platform_last_error_code(),
            FFI_PLATFORM_ERROR_INVALID_ARGUMENT
        );
    }

    #[test]
    fn dir_reports_an_unusable_app_name_as_io() {
        assert!(dir(FFI_DIR_DATA, "..").is_null());
        let (code, message) = last_error();
        assert_eq!(code, FFI_PLATFORM_ERROR_IO);
        assert!(message.unwrap().contains("application name"));
    }

    #[test]
    fn a_successful_call_clears_the_last_error() {
        assert!(dir(7, "ffi-test").is_null());
        assert_eq!(
            ffi_platform_last_error_code(),
            FFI_PLATFORM_ERROR_INVALID_ARGUMENT
        );

        let mut info = empty_info();
        // SAFETY: `info` is valid, writable and owns nothing.
        assert!(unsafe { ffi_platform_info(&mut info) });
        assert_eq!(ffi_platform_last_error_code(), FFI_PLATFORM_ERROR_NONE);
        assert!(ffi_platform_last_error_message().is_null());
        // SAFETY: `info` was filled by `ffi_platform_info`.
        unsafe { ffi_platform_info_free(&mut info) };
    }

    #[test]
    fn last_error_is_per_thread() {
        assert!(dir(7, "ffi-test").is_null());
        assert_eq!(
            ffi_platform_last_error_code(),
            FFI_PLATFORM_ERROR_INVALID_ARGUMENT
        );

        std::thread::spawn(|| {
            assert_eq!(ffi_platform_last_error_code(), FFI_PLATFORM_ERROR_NONE);
        })
            .join()
            .unwrap();

        assert_eq!(
            ffi_platform_last_error_code(),
            FFI_PLATFORM_ERROR_INVALID_ARGUMENT
        );
    }

    #[test]
    fn no_error_recorded_before_any_call() {
        std::thread::spawn(|| {
            assert_eq!(ffi_platform_last_error_code(), FFI_PLATFORM_ERROR_NONE);
            assert!(ffi_platform_last_error_message().is_null());
        })
            .join()
            .unwrap();
    }
}
