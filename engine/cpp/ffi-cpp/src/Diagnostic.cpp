#include "cv-ffi/Diagnostic.hpp"

#include "cv-ffi/Logger.hpp"
#include "ffi.h"

namespace cv_ffi {

std::expected<DiagnosticBuilder, std::string> DiagnosticBuilder::Create(
    std::string_view producer,
    std::string_view category,
    std::uint32_t number,
    Severity severity,
    std::string_view message
)
{
    const std::string producerStr(producer);
    const std::string categoryStr(category);
    const std::string messageStr(message);

    // SAFETY: all three C strings are NUL-terminated (std::string::c_str())
    // and outlive the call.
    ::DiagnosticBuilder* handle = ffi_diagnostic_builder_create(
        producerStr.c_str(), categoryStr.c_str(), number, static_cast<std::uint8_t>(severity), messageStr.c_str()
    );
    if (handle == nullptr) {
        return std::unexpected(
            "ffi_diagnostic_builder_create failed (invalid UTF-8 in producer/category/message, "
            "or an invalid severity byte)"
        );
    }
    return DiagnosticBuilder(handle);
}

DiagnosticBuilder::~DiagnosticBuilder()
{
    Release();
}

DiagnosticBuilder::DiagnosticBuilder(DiagnosticBuilder&& other) noexcept : m_handle(other.m_handle)
{
    other.m_handle = nullptr;
}

DiagnosticBuilder& DiagnosticBuilder::operator=(DiagnosticBuilder&& other) noexcept
{
    if (this != &other) {
        Release();
        m_handle = other.m_handle;
        other.m_handle = nullptr;
    }
    return *this;
}

void DiagnosticBuilder::Release() noexcept
{
    // SAFETY: m_handle is either null (a documented no-op for
    // ffi_diagnostic_builder_destroy) or a pointer this object owns
    // exclusively that has not yet been passed to that function or to
    // ffi_diagnostic_builder_emit (Emit() below clears m_handle before the
    // emit call returns control to any caller that could then destroy this
    // object, so Release() never double-frees/double-consumes).
    ffi_diagnostic_builder_destroy(m_handle);
    m_handle = nullptr;
}

DiagnosticBuilder&& DiagnosticBuilder::Module(std::string_view module) &&
{
    // ffi_diagnostic_builder_set_module dereferences `builder`
    // unconditionally (no null check on the Rust side — see
    // diagnostic.rs) -- guard it here rather than risk UB on an invalid
    // DiagnosticBuilder (default-constructed, moved-from, or already
    // consumed by Emit()).
    if (m_handle == nullptr) {
        return std::move(*this);
    }
    const std::string moduleStr(module);
    // SAFETY: m_handle is a live DiagnosticBuilder, just checked; module is
    // NUL-terminated and outlives the call. A false return (invalid UTF-8)
    // leaves the underlying builder's module field unset, same as never
    // calling this — not reported further here since string_view content is
    // already caller-controlled C++ text, not raw FFI input; the same
    // "can't actually happen" reasoning ffi_build_info_string's doc comment
    // uses.
    ffi_diagnostic_builder_set_module(m_handle, moduleStr.c_str());
    return std::move(*this);
}

DiagnosticBuilder&& DiagnosticBuilder::File(std::string_view file) &&
{
    // See Module()'s comment: ffi_diagnostic_builder_set_file also
    // dereferences `builder` unconditionally.
    if (m_handle == nullptr) {
        return std::move(*this);
    }
    const std::string fileStr(file);
    // SAFETY: m_handle is a live DiagnosticBuilder, just checked; file is
    // NUL-terminated and outlives the call.
    ffi_diagnostic_builder_set_file(m_handle, fileStr.c_str());
    return std::move(*this);
}

DiagnosticBuilder&& DiagnosticBuilder::Line(std::optional<std::uint32_t> line) &&
{
    // See Module()'s comment: ffi_diagnostic_builder_set_line also
    // dereferences `builder` unconditionally — it has nothing else to
    // null-check (its `line` out-parameter is documented as null-or-valid,
    // handled either way), so this guard is the only thing standing between
    // a moved-from/default-constructed DiagnosticBuilder and UB here.
    if (m_handle == nullptr) {
        return std::move(*this);
    }
    const std::uint32_t lineValue = line.value_or(0);
    // SAFETY: m_handle is a live DiagnosticBuilder, just checked; the
    // pointer passed is either null (line unset — a valid, documented "no
    // value") or points to a live local std::uint32_t for the duration of
    // this call.
    ffi_diagnostic_builder_set_line(m_handle, line ? &lineValue : nullptr);
    return std::move(*this);
}

DiagnosticBuilder&& DiagnosticBuilder::Persistent(bool persistent) &&
{
    // See Module()'s comment: ffi_diagnostic_builder_set_persistent also
    // dereferences `builder` unconditionally.
    if (m_handle == nullptr) {
        return std::move(*this);
    }
    // SAFETY: m_handle is a live DiagnosticBuilder, just checked.
    ffi_diagnostic_builder_set_persistent(m_handle, persistent);
    return std::move(*this);
}

void DiagnosticBuilder::Emit(Logger& logger) &&
{
    // ffi_diagnostic_builder_emit dereferences `logger` unconditionally (see
    // its Safety doc: "caller guarantees `logger` is a live Logger" — no
    // null check on the Rust side, unlike `builder`, which it does check).
    // An invalid Logger (Logger::Create() having failed, or a
    // default-constructed one) is exactly the case that contract rules out,
    // so guard it here rather than handing a null pointer to a function
    // that will dereference it regardless.
    if (logger.IsValid()) {
        // SAFETY: logger.GetNativeHandle() is a live Logger, just checked;
        // it stays live for the duration of this call (the caller keeps
        // `logger` alive across the call, per this function's own signature
        // taking it by reference). m_handle is a live DiagnosticBuilder not
        // yet passed to this function or to ffi_diagnostic_builder_destroy.
        ffi_diagnostic_builder_emit(logger.GetNativeHandle(), m_handle);
        // ffi_diagnostic_builder_emit always frees `builder` on this path
        // (see its doc comment) regardless of its own return value.
    } else {
        // No live Logger to emit into: drop the diagnostic instead of
        // risking UB. Still frees `builder` (Release(), below) so this
        // object ends up in the same "consumed" state either way.
        Release();
    }
    // Either branch above has now freed the underlying builder — clear the
    // handle so Release() (destructor or a later move) is a documented
    // no-op instead of a double-free.
    m_handle = nullptr;
}

} // namespace cv_ffi

namespace cv_ffi {

std::expected<void, std::string>
RegisterProducer(std::string_view code, std::string_view displayName, std::string_view organization)
{
    const std::string codeStr(code);
    const std::string displayNameStr(displayName);
    const std::string organizationStr(organization);

    // SAFETY: all three C strings are NUL-terminated and outlive the call.
    const bool ok = ffi_producer_register(codeStr.c_str(), displayNameStr.c_str(), organizationStr.c_str());
    if (!ok) {
        return std::unexpected(
            "ffi_producer_register failed (invalid UTF-8, or code '" + codeStr + "' already registered)"
        );
    }
    return {};
}

} // namespace cv_ffi
