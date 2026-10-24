#pragma once

#include <cstdint>
#include <expected>
#include <optional>
#include <string>
#include <string_view>

namespace cv_ffi::platform {

/// Mirrors the `FFI_PLATFORM_*` constants of `ffi::platform`
/// (engine/rust/crates/ffi/src/platform.rs), i.e. `ccore::platform::Platform`.
/// Keep the numbering in sync with the Rust side. A value this library does
/// not know is mapped to `Unknown`.
enum class Platform : std::uint8_t {
    Windows = 0,
    Linux = 1,
    MacOS = 2,
    Android = 3,
    IOS = 4,
    Unknown = 255,
};

/// Mirrors `FFI_ARCH_*` (`ccore::platform::Architecture`).
enum class Architecture : std::uint8_t {
    X86 = 0,
    X86_64 = 1,
    Arm = 2,
    Arm64 = 3,
    Unknown = 255,
};

/// Mirrors `FFI_ENDIAN_*` (`ccore::platform::Endian`).
enum class Endian : std::uint8_t {
    Little = 0,
    Big = 1,
};

/// Mirrors `FFI_DISPLAY_*` (`platform::info::DisplayServer`): the windowing
/// system of the current session, not of any particular Qt platform plugin.
enum class DisplayServer : std::uint8_t {
    Win32 = 0,
    Cocoa = 1,
    X11 = 2,
    Wayland = 3,
    Android = 4,
    UIKit = 5,
    Headless = 6,
    Unknown = 255,
};

/// Mirrors `FFI_DIR_*` (`platform::paths::DirKind`).
enum class DirKind : std::uint8_t {
    /// User-editable settings. Roams with the user profile on Windows.
    Config = 0,
    /// Persistent application data that is not configuration.
    Data = 1,
    /// Data that can be recreated; may be deleted at any time.
    Cache = 2,
    /// Log files.
    Log = 3,
    /// Scratch files that need not survive a reboot.
    Temp = 4,
    /// The user's home directory. Not specific to the application.
    Home = 5,
    /// The directory of the running executable. Not specific to the application.
    ExecutableDir = 6,
};

/// Name and version of the running operating system.
struct OsVersion {
    /// "Windows", "macOS", "Ubuntu", ...
    std::string name;
    /// "10.0.26100", "14.5", "24.04", ...
    std::string version;
    /// Kernel release (`uname -r`), unset where the platform has none.
    std::optional<std::string> kernel;
};

/// What the operating system and hardware report (`platform::info::SystemInfo`).
/// Fields the platform cannot report are unset.
struct SystemInfo {
    /// The OS the Rust library was compiled for.
    Platform platform = Platform::Unknown;
    /// The CPU architecture the Rust library was compiled for.
    Architecture architecture = Architecture::Unknown;
    Endian endian = Endian::Little;
    /// Pointer width of the target in bits (32 or 64).
    std::uint32_t pointerWidth = 0;
    DisplayServer displayServer = DisplayServer::Unknown;
    /// Logical cores this process may use, at least 1.
    std::uint32_t logicalCores = 1;
    /// Physical cores of the machine, if the platform reports them.
    std::optional<std::uint32_t> physicalCores;
    /// Installed physical memory in bytes, if the platform reports it.
    std::optional<std::uint64_t> totalMemoryBytes;
    /// Name and version of the running OS, if known.
    std::optional<OsVersion> os;
};

/// Wraps `ffi_platform_info()`/`ffi_platform_info_free()` into one call that
/// returns an owned value, so callers never see the raw struct or its
/// Rust-allocated strings. Fails only when the OS reports text that cannot be
/// represented as a C string; the message comes from `ffi_platform_last_error_message`.
[[nodiscard]] std::expected<SystemInfo, std::string> Info();

/// The path of one standard directory of the application `app` (wraps
/// `ffi_platform_dir`). The directory is not created. `app` is also required
/// for `DirKind::Home` and `DirKind::ExecutableDir`, which ignore it.
///
/// Fails with the message of the Rust error when the platform has no standard
/// directories (Android, iOS), when `app` is not usable as a directory name
/// (empty, ".", "..", path separators, ...), or when the OS cannot say.
[[nodiscard]] std::expected<std::string, std::string> StandardDir(DirKind kind, std::string_view app);

} // namespace cv_ffi::platform
