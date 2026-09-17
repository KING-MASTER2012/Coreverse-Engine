// Faz 7a proof: the log-core/log-sinks FFI surface -- opaque Logger and
// DiagnosticBuilder handles, console + file sinks, runtime producer
// registration. Three independent checks:
//   1. ffi_producer_register() -- ABI link + duplicate-code rejection.
//   2. Console sink -- create logger, attach ConsoleSink, emit a few
//      severities, clean shutdown. Output goes to stdout/stderr for
//      visual/manual inspection (same idea as renderer_device_test).
//   3. File sink -- attach FileSink rooted at a unique temporary
//      directory, emit a diagnostic, and actually read the resulting
//      Logs/latest.log back to confirm the round trip -- not just that
//      it compiles and links.

#include <cstdint>
#include <cstdio>
#include <filesystem>
#include <fstream>
#include <random>
#include <sstream>
#include <string>
#include <system_error>

#include "ffi.h"

namespace {
// Severity bytes from log_core::Severity (see
// engine/rust/crates/log-core/src/severity.rs) -- 0 = Trace .. 7 = Fatal.
constexpr std::uint8_t kSeverityInfo = 2;
constexpr std::uint8_t kSeverityWarning = 4;
constexpr std::uint8_t kSeverityError = 5;

// RAII guard for a unique temporary directory: removes it (and
// everything under it) on scope exit, success or failure, so this
// test never leaves anything behind in the OS temp dir.
class TempDirGuard
{
public:
    TempDirGuard()
    {
        std::random_device rd;
        std::mt19937_64 gen(rd());
        std::uniform_int_distribution<std::uint64_t> dist;
        path_ = std::filesystem::temp_directory_path() / ("coreverse_ffi_logger_test_" + std::to_string(dist(gen)));
        std::filesystem::create_directories(path_);
    }

    ~TempDirGuard()
    {
        std::error_code ec;
        std::filesystem::remove_all(path_, ec);
    }

    TempDirGuard(const TempDirGuard&) = delete;
    TempDirGuard& operator=(const TempDirGuard&) = delete;

    const std::filesystem::path& path() const
    {
        return path_;
    }

private:
    std::filesystem::path path_;
};

bool ReadFileContains(const std::filesystem::path& file, const std::string& needle)
{
    std::ifstream in(file);
    if (!in) {
        return false;
    }
    std::ostringstream contents;
    contents << in.rdbuf();
    return contents.str().find(needle) != std::string::npos;
}

bool TestProducerRegistration()
{
    const bool ok = ffi_producer_register("CV-FFI-LOGGER-TEST", "FFI Logger Test", "Coreverse");
    if (!ok) {
        std::fprintf(stderr, "ffi_producer_register() failed\n");
        return false;
    }
    // Re-registering the same code must fail -- proves collisions are
    // actually detected, not silently accepted.
    const bool duplicate_accepted = ffi_producer_register("CV-FFI-LOGGER-TEST", "Duplicate", "Coreverse");
    if (duplicate_accepted) {
        std::fprintf(stderr, "ffi_producer_register() accepted a duplicate code\n");
        return false;
    }
    return true;
}

bool TestConsoleSink()
{
    Logger* logger = ffi_logger_create();
    if (logger == nullptr) {
        std::fprintf(stderr, "ffi_logger_create() returned null\n");
        return false;
    }

    bool ok = ffi_logger_add_console_sink(logger);
    ok = ffi_logger_set_min_severity(logger, kSeverityInfo) && ok;

    struct Case {
        std::uint8_t severity;
        const char* message;
    };
    const Case cases[] = {
        {kSeverityInfo, "console sink info"},
        {kSeverityWarning, "console sink warning"},
        {kSeverityError, "console sink error"},
    };

    for (const Case& c : cases) {
        DiagnosticBuilder* builder =
            ffi_diagnostic_builder_create("CV-FFI-LOGGER-TEST", "TEST", 1, c.severity, c.message);
        if (builder == nullptr) {
            std::fprintf(stderr, "ffi_diagnostic_builder_create() returned null\n");
            ok = false;
            continue;
        }
        ok = ffi_diagnostic_builder_emit(logger, builder) && ok;
    }

    ffi_logger_shutdown(logger);
    ffi_logger_destroy(logger);
    if (!ok) {
        std::fprintf(stderr, "console sink test failed (see above)\n");
    }
    return ok;
}

bool TestFileSink()
{
    TempDirGuard temp_dir;
    Logger* logger = ffi_logger_create();
    if (logger == nullptr) {
        std::fprintf(stderr, "ffi_logger_create() returned null\n");
        return false;
    }

    const std::string project_root = temp_dir.path().string();
    if (!ffi_logger_add_file_sink(logger, project_root.c_str())) {
        std::fprintf(stderr, "ffi_logger_add_file_sink() failed\n");
        ffi_logger_destroy(logger);
        return false;
    }

    const char* kMessage = "file sink round-trip message";
    DiagnosticBuilder* builder =
        ffi_diagnostic_builder_create("CV-FFI-LOGGER-TEST", "TEST", 2, kSeverityError, kMessage);
    if (builder == nullptr) {
        std::fprintf(stderr, "ffi_diagnostic_builder_create() returned null\n");
        ffi_logger_shutdown(logger);
        ffi_logger_destroy(logger);
        return false;
    }

    std::uint32_t line = 42;
    bool ok = ffi_diagnostic_builder_set_module(builder, "ffi_logger_test");
    ok = ffi_diagnostic_builder_set_file(builder, "ffi_logger_test.cpp") && ok;
    ok = ffi_diagnostic_builder_set_line(builder, &line) && ok;
    ffi_diagnostic_builder_set_persistent(builder, false);

    // ffi_diagnostic_builder_emit() always consumes/frees `builder` --
    // regardless of the setters' return values above -- so `builder`
    // is invalid after this line no matter what.
    ok = ffi_diagnostic_builder_emit(logger, builder) && ok;

    // Verify *before* shutdown: FileSink::shutdown() deletes every
    // non-persistent log file, including latest.log, which is exactly
    // what we're checking here.
    const auto latest_log = temp_dir.path() / "Logs" / "latest.log";
    ok = ReadFileContains(latest_log, kMessage) && ok;
    if (!ok) {
        std::fprintf(stderr, "file sink test failed (see above)\n");
    }

    ffi_logger_shutdown(logger);
    ffi_logger_destroy(logger);
    return ok;
}
} // namespace

int main()
{
    bool ok = true;
    ok = TestProducerRegistration() && ok;
    ok = TestConsoleSink() && ok;
    ok = TestFileSink() && ok;
    return ok ? 0 : 1;
}
