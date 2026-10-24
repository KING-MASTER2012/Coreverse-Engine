// Phase 5 (platform) proof: cv-ffi's platform wrappers against the real Rust
// library -- cv_ffi::platform::Info() (OS, CPU, memory, display server) and
// StandardDir() (the per-application directories), including the failure
// paths: an unknown directory kind and an unusable application name must come
// back as std::unexpected with the Rust error message, and a failure must not
// poison the next call.
//
// Runs on the desktop platforms only (Linux, macOS, Windows): mobile has no
// standard directories (Unsupported) and no test runner.

#include <algorithm>
#include <bit>
#include <cctype>
#include <cstdint>
#include <cstdio>
#include <filesystem>
#include <string>
#include <string_view>

#include "cv-ffi/Platform.hpp"

namespace {

namespace platform = cv_ffi::platform;

int g_failures = 0;

void Check(bool ok, std::string_view what)
{
    if (!ok) {
        std::fprintf(stderr, "FAIL: %.*s\n", static_cast<int>(what.size()), what.data());
        ++g_failures;
    }
}

std::string ToLower(std::string text)
{
    std::ranges::transform(text, text.begin(), [](unsigned char c) { return static_cast<char>(std::tolower(c)); });
    return text;
}

constexpr platform::Platform ExpectedPlatform()
{
#if defined(_WIN32)
    return platform::Platform::Windows;
#elif defined(__APPLE__)
    return platform::Platform::MacOS;
#elif defined(__linux__)
    return platform::Platform::Linux;
#else
    return platform::Platform::Unknown;
#endif
}

constexpr platform::Architecture ExpectedArchitecture()
{
#if defined(__x86_64__) || defined(_M_X64)
    return platform::Architecture::X86_64;
#elif defined(__aarch64__) || defined(_M_ARM64)
    return platform::Architecture::Arm64;
#else
    return platform::Architecture::Unknown;
#endif
}

void TestInfo()
{
    const auto result = platform::Info();
    Check(result.has_value(), "Info() succeeds");
    if (!result) {
        std::fprintf(stderr, "  Info() error: %s\n", result.error().c_str());
        return;
    }
    const platform::SystemInfo& info = *result;

    Check(info.platform == ExpectedPlatform(), "platform matches the C++ compile-time platform");
    if (ExpectedArchitecture() != platform::Architecture::Unknown) {
        Check(info.architecture == ExpectedArchitecture(), "architecture matches the C++ compile-time architecture");
    }
    const platform::Endian nativeEndian =
        std::endian::native == std::endian::little ? platform::Endian::Little : platform::Endian::Big;
    Check(info.endian == nativeEndian, "endian matches std::endian::native");
    Check(info.pointerWidth == sizeof(void*) * 8, "pointer width matches sizeof(void*)");
    Check(info.logicalCores >= 1, "at least one logical core");
    if (info.physicalCores) {
        Check(*info.physicalCores >= 1, "a reported physical core count is at least 1");
    }
    Check(info.totalMemoryBytes.has_value(), "total memory is reported on desktop platforms");
    if (info.totalMemoryBytes) {
        Check(*info.totalMemoryBytes >= std::uint64_t{64} * 1024 * 1024, "total memory is at least 64 MiB");
    }
    if (info.os) {
        Check(!info.os->name.empty(), "OS name is not empty");
        Check(!info.os->version.empty(), "OS version is not empty");
    }
    if (ExpectedPlatform() == platform::Platform::Windows) {
        Check(info.displayServer == platform::DisplayServer::Win32, "Windows reports Win32");
        Check(info.os.has_value(), "Windows reports its version");
    }
    if (ExpectedPlatform() == platform::Platform::MacOS) {
        Check(info.displayServer == platform::DisplayServer::Cocoa, "macOS reports Cocoa");
        Check(info.os.has_value(), "macOS reports its version");
    }

    std::printf(
        "platform: %u-bit, %u logical cores, %s %s\n",
        info.pointerWidth,
        info.logicalCores,
        info.os ? info.os->name.c_str() : "unknown OS",
        info.os ? info.os->version.c_str() : ""
    );
}

void TestStandardDirs()
{
    constexpr std::string_view app = "cv-ffi-test";
    constexpr platform::DirKind appKinds[] = {
        platform::DirKind::Config,
        platform::DirKind::Data,
        platform::DirKind::Cache,
        platform::DirKind::Log,
        platform::DirKind::Temp,
    };

    std::string seen[std::size(appKinds)];
    for (std::size_t i = 0; i < std::size(appKinds); ++i) {
        const auto dir = platform::StandardDir(appKinds[i], app);
        Check(dir.has_value(), "an application directory is available");
        if (!dir) {
            std::fprintf(stderr, "  error: %s\n", dir.error().c_str());
            continue;
        }
        seen[i] = *dir;
        Check(std::filesystem::path(*dir).is_absolute(), "an application directory is absolute");
        Check(dir->find(app) != std::string::npos, "an application directory carries the application name");
        // The organization directory: lowercase on Linux, "Coreverse" elsewhere.
        const std::string_view organization =
            ExpectedPlatform() == platform::Platform::Linux ? "coreverse" : "Coreverse";
        Check(dir->find(organization) != std::string::npos, "an application directory carries the organization name");
        Check(ToLower(*dir).find("coreverse") != std::string::npos, "organization name is present case-insensitively");
    }

    // Config, data, cache and log never share a directory.
    for (std::size_t i = 0; i < 4; ++i) {
        for (std::size_t j = i + 1; j < 4; ++j) {
            Check(seen[i].empty() || seen[j].empty() || seen[i] != seen[j], "config/data/cache/log directories differ");
        }
    }

    const auto home = platform::StandardDir(platform::DirKind::Home, app);
    Check(home.has_value() && std::filesystem::path(*home).is_absolute(), "home directory is absolute");

    const auto exeDir = platform::StandardDir(platform::DirKind::ExecutableDir, app);
    Check(exeDir.has_value() && std::filesystem::is_directory(*exeDir), "executable directory exists");

    // The application name does not matter for the system directories.
    const auto home2 = platform::StandardDir(platform::DirKind::Home, "another-app");
    Check(home && home2 && *home == *home2, "home does not depend on the application name");

    // Nothing was created by asking.
    if (!seen[0].empty()) {
        std::error_code ec;
        Check(
            !std::filesystem::exists(seen[0], ec) || ec, "StandardDir does not create the directory (or it pre-existed)"
        );
    }
}

void TestFailures()
{
    const auto badKind = platform::StandardDir(static_cast<platform::DirKind>(200), "cv-ffi-test");
    Check(!badKind.has_value(), "an unknown directory kind fails");
    if (!badKind) {
        Check(badKind.error().find("200") != std::string::npos, "the error names the bad kind byte");
    }

    for (const std::string_view name : {"", ".", "..", "a/b", "a\\b", "bad:name", "trailing."}) {
        const auto result = platform::StandardDir(platform::DirKind::Data, name);
        Check(!result.has_value(), "an unusable application name fails");
        if (!result) {
            Check(!result.error().empty(), "the error message is not empty");
        }
    }

    // A failure must not poison the next call.
    const auto ok = platform::StandardDir(platform::DirKind::Data, "cv-ffi-test");
    Check(ok.has_value(), "a successful call after a failure succeeds");
}

} // namespace

int main()
{
    TestInfo();
    TestStandardDirs();
    TestFailures();

    if (g_failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", g_failures);
        return 1;
    }
    std::printf("cv_ffi_platform_test: all checks passed\n");
    return 0;
}
