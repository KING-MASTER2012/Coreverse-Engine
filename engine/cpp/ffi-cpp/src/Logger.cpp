#include "cv-ffi/Logger.hpp"

#include "ffi.h"

namespace cv_ffi {

std::expected<Logger, std::string> Logger::Create()
{
    ::Logger* handle = ffi_logger_create();
    // ffi_logger_create is documented as always returning a valid, non-null
    // pointer — this branch exists only so a future change to that contract
    // fails loudly here instead of handing out an invalid Logger silently.
    if (handle == nullptr) {
        return std::unexpected("ffi_logger_create returned null");
    }
    return Logger(handle);
}

Logger::~Logger()
{
    Release();
}

Logger::Logger(Logger&& other) noexcept : m_handle(other.m_handle)
{
    other.m_handle = nullptr;
}

Logger& Logger::operator=(Logger&& other) noexcept
{
    if (this != &other) {
        Release();
        m_handle = other.m_handle;
        other.m_handle = nullptr;
    }
    return *this;
}

void Logger::Release() noexcept
{
    // SAFETY: m_handle is either null (a documented no-op for
    // ffi_logger_destroy) or a pointer this Logger owns exclusively (move
    // semantics guarantee no other Logger holds it) that has not yet been
    // destroyed.
    ffi_logger_destroy(m_handle);
    m_handle = nullptr;
}

void Logger::AddConsoleSink()
{
    // ffi_logger_add_console_sink dereferences `logger` unconditionally (no
    // null check on the Rust side — see logger.rs) -- guard it here rather
    // than risk UB on an invalid Logger (default-constructed, or moved
    // from).
    if (m_handle == nullptr) {
        return;
    }
    // SAFETY: m_handle is a live Logger, just checked, for the lifetime of
    // this call (nothing else can destroy m_handle concurrently — this
    // library does not support calling into the same Logger from multiple
    // threads without external synchronization, matching the wrapped Rust
    // type's own thread-safety contract).
    ffi_logger_add_console_sink(m_handle);
}

std::expected<void, std::string> Logger::AddFileSink(std::string_view projectRoot)
{
    // See AddConsoleSink()'s comment: ffi_logger_add_file_sink also
    // dereferences `logger` unconditionally.
    if (m_handle == nullptr) {
        return std::unexpected("AddFileSink() called on an invalid Logger");
    }
    const std::string projectRootStr(projectRoot);
    // SAFETY: m_handle is a live Logger, just checked; project_root is a
    // NUL-terminated C string that outlives the call.
    const bool ok = ffi_logger_add_file_sink(m_handle, projectRootStr.c_str());
    if (!ok) {
        // ffi_logger_add_file_sink reports failure with no accompanying
        // message (unlike ffi::vfs's thread-local last-error slot) — the
        // Rust side only distinguishes "null/invalid UTF-8 project_root"
        // from "couldn't create/clear the Logs directory", neither of which
        // it currently surfaces beyond the bool. Name the most likely cause
        // rather than making up a message that might be wrong.
        return std::unexpected(
            "ffi_logger_add_file_sink failed (invalid project root, or its Logs directory "
            "couldn't be created/cleared)"
        );
    }
    return {};
}

void Logger::SetMinSeverity(Severity severity)
{
    // See AddConsoleSink()'s comment: ffi_logger_set_min_severity also
    // dereferences `logger` unconditionally.
    if (m_handle == nullptr) {
        return;
    }
    // SAFETY: m_handle is a live Logger, just checked.
    ffi_logger_set_min_severity(m_handle, static_cast<std::uint8_t>(severity));
}

void Logger::Shutdown() noexcept
{
    // ffi_logger_shutdown also dereferences `logger` unconditionally (see
    // AddConsoleSink()'s comment) -- this guard is why Shutdown() is safe
    // to call on a default-constructed/moved-from Logger, as its own doc
    // comment promises.
    if (m_handle == nullptr) {
        return;
    }
    // SAFETY: m_handle is a live Logger, just checked.
    ffi_logger_shutdown(m_handle);
}

} // namespace cv_ffi
