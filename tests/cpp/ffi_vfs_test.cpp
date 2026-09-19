// Faz 7b proof: the vfs FFI surface -- process-wide ffi_vfs_init(),
// write/read/exists against a Config-root file, the thread-local
// last-error mechanism (code + message), and two explicit failure
// paths (AlreadyInitialized from a second init call, InvalidPath from
// a bad root byte).

#include <cstdint>
#include <cstdio>
#include <cstring>
#include <filesystem>
#include <random>
#include <string>
#include <system_error>

#include "ffi.h"

namespace
{
    // Root byte values from root::Root::as_u8() (see
    // engine/rust/common/root/src/lib.rs) -- Config is used here because
    // it's documented as always-loose (writable in every VfsMode), so a
    // Development-mode init is enough to exercise it.
    constexpr std::uint8_t kRootConfig = 3;

    // VfsMode byte values from vfs::VfsMode::as_u8() (see
    // engine/rust/crates/vfs/src/context.rs).
    constexpr std::uint8_t kModeDevelopment = 0;

    // VfsError codes from vfs::VfsError::code() (see
    // engine/rust/crates/vfs/src/error.rs).
    constexpr std::uint8_t kErrorNotFound = 1;
    constexpr std::uint8_t kErrorInvalidPath = 4;
    constexpr std::uint8_t kErrorAlreadyInitialized = 6;
    constexpr std::uint8_t kErrorNoneRecorded = 255;

    // RAII guard for a unique temporary directory -- same pattern as
    // tests/cpp/ffi_logger_test.cpp's TempDirGuard.
    class TempDirGuard
    {
    public:
        TempDirGuard()
        {
            std::random_device rd;
            std::mt19937_64 gen(rd());
            std::uniform_int_distribution<std::uint64_t> dist;
            path_ = std::filesystem::temp_directory_path() /
                    ("coreverse_ffi_vfs_test_" + std::to_string(dist(gen)));
            std::filesystem::create_directories(path_);
        }

        ~TempDirGuard()
        {
            std::error_code ec;
            std::filesystem::remove_all(path_, ec);
        }

        TempDirGuard(const TempDirGuard&) = delete;
        TempDirGuard& operator=(const TempDirGuard&) = delete;

        const std::filesystem::path& path() const { return path_; }

    private:
        std::filesystem::path path_;
    };

    bool ExpectLastError(std::uint8_t expected_code, const char* what)
    {
        const std::uint8_t code = ffi_vfs_last_error_code();
        if (code != expected_code)
        {
            std::fprintf(stderr, "%s: expected error code %u, got %u\n", what,
                         static_cast<unsigned>(expected_code), static_cast<unsigned>(code));
            return false;
        }
        char* message = ffi_vfs_last_error_message();
        if (message == nullptr)
        {
            std::fprintf(stderr, "%s: ffi_vfs_last_error_message() returned null\n", what);
            return false;
        }
        const bool has_message = std::strlen(message) > 0;
        ffi_free_string(message);
        if (!has_message)
        {
            std::fprintf(stderr, "%s: last error message was empty\n", what);
        }
        return has_message;
    }
} // namespace

int main()
{
    bool ok = true;

    // Before any ffi_vfs_* call, there's nothing recorded yet.
    if (ffi_vfs_last_error_code() != kErrorNoneRecorded)
    {
        std::fprintf(stderr, "expected no last error before any call\n");
        ok = false;
    }

    TempDirGuard temp_dir;
    const std::string project_root = temp_dir.path().string();

    if (!ffi_vfs_init(project_root.c_str(), nullptr, kModeDevelopment))
    {
        std::fprintf(stderr, "ffi_vfs_init() failed\n");
        ExpectLastError(ffi_vfs_last_error_code(), "ffi_vfs_init");
        return 1;
    }

    // VfsContext is a process-wide singleton -- a second init must fail
    // with AlreadyInitialized, not silently reset anything.
    if (ffi_vfs_init(project_root.c_str(), nullptr, kModeDevelopment))
    {
        std::fprintf(stderr, "second ffi_vfs_init() unexpectedly succeeded\n");
        ok = false;
    }
    ok = ExpectLastError(kErrorAlreadyInitialized, "double ffi_vfs_init") && ok;

    // write_bytes -> exists -> read_bytes round trip.
    const char kPayload[] = "faz 7b vfs round-trip";
    const auto* payload_bytes = reinterpret_cast<const std::uint8_t*>(kPayload);
    const std::size_t payload_len = sizeof(kPayload) - 1; // exclude the trailing NUL

    if (!ffi_vfs_write_bytes(kRootConfig, "ffi_vfs_test.bin", payload_bytes, payload_len))
    {
        std::fprintf(stderr, "ffi_vfs_write_bytes() failed\n");
        ExpectLastError(ffi_vfs_last_error_code(), "ffi_vfs_write_bytes");
        ok = false;
    }

    bool exists = false;
    if (!ffi_vfs_exists(kRootConfig, "ffi_vfs_test.bin", &exists) || !exists)
    {
        std::fprintf(stderr, "ffi_vfs_exists() for the written file failed or returned false\n");
        ok = false;
    }

    bool missing_exists = true;
    if (!ffi_vfs_exists(kRootConfig, "does_not_exist.bin", &missing_exists) || missing_exists)
    {
        std::fprintf(stderr, "ffi_vfs_exists() for a missing file failed or returned true\n");
        ok = false;
    }

    std::uint8_t* read_data = nullptr;
    std::size_t read_len = 0;
    if (!ffi_vfs_read_bytes(kRootConfig, "ffi_vfs_test.bin", &read_data, &read_len))
    {
        std::fprintf(stderr, "ffi_vfs_read_bytes() failed\n");
        ExpectLastError(ffi_vfs_last_error_code(), "ffi_vfs_read_bytes");
        ok = false;
    }
    else
    {
        const bool matches =
            read_len == payload_len && std::memcmp(read_data, payload_bytes, payload_len) == 0;
        if (!matches)
        {
            std::fprintf(stderr, "ffi_vfs_read_bytes() content mismatch\n");
            ok = false;
        }
        ffi_vfs_free_bytes(read_data, read_len);
    }

    // read_to_string on the same file (valid UTF-8 payload).
    char* read_string = ffi_vfs_read_to_string(kRootConfig, "ffi_vfs_test.bin");
    if (read_string == nullptr)
    {
        std::fprintf(stderr, "ffi_vfs_read_to_string() returned null\n");
        ExpectLastError(ffi_vfs_last_error_code(), "ffi_vfs_read_to_string");
        ok = false;
    }
    else
    {
        const bool matches = std::strcmp(read_string, kPayload) == 0;
        ffi_free_string(read_string);
        if (!matches)
        {
            std::fprintf(stderr, "ffi_vfs_read_to_string() content mismatch\n");
            ok = false;
        }
    }

    // NotFound failure path.
    char* missing = ffi_vfs_read_to_string(kRootConfig, "still_does_not_exist.bin");
    if (missing != nullptr)
    {
        std::fprintf(stderr, "ffi_vfs_read_to_string() for a missing file unexpectedly succeeded\n");
        ffi_free_string(missing);
        ok = false;
    }
    ok = ExpectLastError(kErrorNotFound, "ffi_vfs_read_to_string on a missing file") && ok;

    // InvalidPath failure path -- a root byte outside 0-8.
    bool unused_exists = false;
    if (ffi_vfs_exists(255, "x", &unused_exists))
    {
        std::fprintf(stderr, "ffi_vfs_exists() with an invalid root byte unexpectedly succeeded\n");
        ok = false;
    }
    ok = ExpectLastError(kErrorInvalidPath, "ffi_vfs_exists with an invalid root byte") && ok;

    if (!ok)
    {
        std::fprintf(stderr, "ffi_vfs_test failed (see above)\n");
    }
    return ok ? 0 : 1;
}
