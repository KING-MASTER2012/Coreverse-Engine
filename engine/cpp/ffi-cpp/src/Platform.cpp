#include "cv-ffi/Platform.hpp"

#include "ffi.h"

namespace cv_ffi::platform {

namespace {

/// `ffi_platform_*` calls report failure via `false`/null plus a thread-local
/// last-error slot (see ffi::platform's module docs). This reads the slot into
/// an owned std::string and frees the Rust-allocated message, so every wrapper
/// below can `return std::unexpected(LastError());`. Must be called right
/// after the failing `ffi_platform_*` call, before another one on this thread
/// overwrites the slot.
std::string LastError()
{
    char* message = ffi_platform_last_error_message();
    if (message == nullptr) {
        return "platform: no error recorded (ffi_platform_last_error_message returned null)";
    }
    std::string result(message);
    ffi_free_string(message);
    return result;
}

/// Copies a possibly-null C string into an optional.
std::optional<std::string> OptionalString(const char* text)
{
    if (text == nullptr) {
        return std::nullopt;
    }
    return std::string(text);
}

/// Releases a `PlatformInfo`'s Rust-allocated strings on scope exit.
class InfoGuard
{
public:
    InfoGuard() = default;
    InfoGuard(const InfoGuard&) = delete;
    InfoGuard& operator=(const InfoGuard&) = delete;
    ~InfoGuard()
    {
        ffi_platform_info_free(&raw);
    }

    ::PlatformInfo raw{};
};

// Unknown bytes map to Unknown (or the first value, where there is no
// Unknown), so a newer Rust side cannot make an older C++ side misbehave.

// DirKind is passed to Rust as its raw byte, so it must match the header.
static_assert(static_cast<std::uint8_t>(DirKind::Config) == FFI_DIR_CONFIG);
static_assert(static_cast<std::uint8_t>(DirKind::Data) == FFI_DIR_DATA);
static_assert(static_cast<std::uint8_t>(DirKind::Cache) == FFI_DIR_CACHE);
static_assert(static_cast<std::uint8_t>(DirKind::Log) == FFI_DIR_LOG);
static_assert(static_cast<std::uint8_t>(DirKind::Temp) == FFI_DIR_TEMP);
static_assert(static_cast<std::uint8_t>(DirKind::Home) == FFI_DIR_HOME);
static_assert(static_cast<std::uint8_t>(DirKind::ExecutableDir) == FFI_DIR_EXECUTABLE_DIR);

Platform ToPlatform(std::uint8_t value)
{
    switch (value) {
        case FFI_PLATFORM_WINDOWS:
            return Platform::Windows;
        case FFI_PLATFORM_LINUX:
            return Platform::Linux;
        case FFI_PLATFORM_MACOS:
            return Platform::MacOS;
        case FFI_PLATFORM_ANDROID:
            return Platform::Android;
        case FFI_PLATFORM_IOS:
            return Platform::IOS;
        default:
            return Platform::Unknown;
    }
}

Architecture ToArchitecture(std::uint8_t value)
{
    switch (value) {
        case FFI_ARCH_X86:
            return Architecture::X86;
        case FFI_ARCH_X86_64:
            return Architecture::X86_64;
        case FFI_ARCH_ARM:
            return Architecture::Arm;
        case FFI_ARCH_ARM64:
            return Architecture::Arm64;
        default:
            return Architecture::Unknown;
    }
}

Endian ToEndian(std::uint8_t value)
{
    return value == FFI_ENDIAN_BIG ? Endian::Big : Endian::Little;
}

DisplayServer ToDisplayServer(std::uint8_t value)
{
    switch (value) {
        case FFI_DISPLAY_WIN32:
            return DisplayServer::Win32;
        case FFI_DISPLAY_COCOA:
            return DisplayServer::Cocoa;
        case FFI_DISPLAY_X11:
            return DisplayServer::X11;
        case FFI_DISPLAY_WAYLAND:
            return DisplayServer::Wayland;
        case FFI_DISPLAY_ANDROID:
            return DisplayServer::Android;
        case FFI_DISPLAY_UIKIT:
            return DisplayServer::UIKit;
        case FFI_DISPLAY_HEADLESS:
            return DisplayServer::Headless;
        default:
            return DisplayServer::Unknown;
    }
}

} // namespace

std::expected<SystemInfo, std::string> Info()
{
    InfoGuard guard;
    // SAFETY: guard.raw is a valid, writable struct that owns nothing yet.
    if (!ffi_platform_info(&guard.raw)) {
        return std::unexpected(LastError());
    }
    const ::PlatformInfo& raw = guard.raw;

    SystemInfo info;
    info.platform = ToPlatform(raw.platform);
    info.architecture = ToArchitecture(raw.architecture);
    info.endian = ToEndian(raw.endian);
    info.pointerWidth = raw.pointer_width;
    info.displayServer = ToDisplayServer(raw.display_server);
    info.logicalCores = raw.logical_cores;
    if (raw.physical_cores != 0) {
        info.physicalCores = raw.physical_cores;
    }
    if (raw.total_memory_bytes != 0) {
        info.totalMemoryBytes = raw.total_memory_bytes;
    }
    // The name and version are present exactly when the OS version is known.
    if (raw.os_name != nullptr && raw.os_version != nullptr) {
        info.os = OsVersion{raw.os_name, raw.os_version, OptionalString(raw.kernel)};
    }
    return info;
}

std::expected<std::string, std::string> StandardDir(DirKind kind, std::string_view app)
{
    const std::string appStr(app);
    // SAFETY: appStr.c_str() is a NUL-terminated C string that outlives the call.
    char* path = ffi_platform_dir(static_cast<std::uint8_t>(kind), appStr.c_str());
    if (path == nullptr) {
        return std::unexpected(LastError());
    }
    std::string result(path);
    ffi_free_string(path);
    return result;
}

} // namespace cv_ffi::platform
