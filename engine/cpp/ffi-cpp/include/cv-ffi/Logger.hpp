#pragma once

#include <expected>
#include <string>
#include <string_view>

#include "cv-ffi/Severity.hpp"

// Opaque handle from the cbindgen-generated header (ffi.h); forward-declared
// here so this header does not have to include ffi.h itself (kept out of
// cv-ffi's public headers — see Vfs.hpp's file comment for why).
extern "C" {
struct Logger;
}

namespace cv_ffi {

/// Move-only RAII handle wrapping the `ffi` crate's opaque `Logger*`
/// (engine/rust/crates/ffi/src/logger.rs). Construction happens through
/// Logger::Create(); the destructor releases the underlying Rust `Box`
/// automatically via `ffi_logger_destroy`, so there is no explicit Destroy()
/// to forget or call twice — same RAII shape as renderer::Buffer/Swapchain
/// (see renderer/include/renderer/Buffer.hpp).
///
/// ## Shutdown() is NOT called by the destructor
/// Unlike Buffer/Swapchain, destroying a Logger does not by itself flush or
/// clean up its sinks — the underlying `ffi_logger_shutdown` deletes any of
/// the sinks' log files that were never marked persistent, which is a
/// meaningful, visible side effect (lost logs), not mere resource cleanup.
/// Running it implicitly from a destructor — including one that runs during
/// stack unwinding after an exception, or one whose object outlives more
/// than it should due to a bug — would make that data loss silent and
/// timing-dependent. Call Shutdown() explicitly on every clean exit path,
/// before the Logger is destroyed; a Logger that is simply destroyed without
/// it (a crash, an early return you didn't intend) just leaves the session's
/// log files on disk, which is the safer default to fall into.
class Logger
{
public:
    Logger() noexcept = default;
    ~Logger();

    Logger(const Logger&) = delete;
    Logger& operator=(const Logger&) = delete;

    Logger(Logger&& other) noexcept;
    Logger& operator=(Logger&& other) noexcept;

    /// Creates a Logger with no sinks attached yet — see AddConsoleSink()/
    /// AddFileSink(). `ffi_logger_create` is documented as always returning
    /// a valid pointer, so this has nothing to fail on; kept as
    /// std::expected anyway for symmetry with every other Create() in this
    /// library and so a future Rust-side failure mode doesn't need a
    /// signature change here.
    [[nodiscard]] static std::expected<Logger, std::string> Create();

    [[nodiscard]] bool IsValid() const noexcept
    {
        return m_handle != nullptr;
    }

    /// Attaches a console sink (color on, no severity floor of its own).
    /// Always succeeds if this Logger is valid — see
    /// `ffi_logger_add_console_sink`'s doc comment.
    void AddConsoleSink();

    /// Attaches a file sink rooted at `<projectRoot>/Logs`, using the
    /// default routing rules (see `log_sinks::file_vfs::default_route`).
    /// Fails if `projectRoot`'s `Logs` directory couldn't be created or its
    /// standard log files couldn't be cleared.
    [[nodiscard]] std::expected<void, std::string> AddFileSink(std::string_view projectRoot);

    /// Sets the minimum severity this Logger accepts before fanning a
    /// diagnostic out to its sinks (individual sinks may apply their own,
    /// stricter threshold on top of this).
    void SetMinSeverity(Severity severity);

    /// Flushes every attached sink and deletes any of their log files that
    /// were never marked persistent. See the class comment: call this
    /// explicitly, once, before this Logger is destroyed on a clean exit —
    /// the destructor does not call it. Safe to call more than once (each
    /// call just re-runs the same flush), but calling it and then
    /// continuing to log through this Logger is pointless: nothing removes
    /// it from being usable, but any file sink's already-deleted files will
    /// simply be recreated by the next diagnostic routed to them.
    void Shutdown() noexcept;

    /// Backend-owned opaque resource pointer — same escape-hatch contract
    /// as renderer::Buffer::GetNativeHandle(). Used by DiagnosticBuilder::
    /// Emit(), which needs the raw `Logger*` to pass to `ffi_diagnostic_
    /// builder_emit`; not otherwise meant to be dereferenced by callers.
    [[nodiscard]] ::Logger* GetNativeHandle() const noexcept
    {
        return m_handle;
    }

private:
    explicit Logger(::Logger* handle) noexcept : m_handle(handle) {}

    void Release() noexcept;

    ::Logger* m_handle = nullptr;
};

} // namespace cv_ffi
