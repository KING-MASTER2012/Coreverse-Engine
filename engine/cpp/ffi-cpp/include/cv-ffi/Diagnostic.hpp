#pragma once

#include <cstdint>
#include <expected>
#include <optional>
#include <string>
#include <string_view>

#include "cv-ffi/Severity.hpp"

// See Logger.hpp for why this is forward-declared rather than pulled in via
// ffi.h, and Logger.hpp itself for the real ::Logger declaration.
extern "C" {
struct DiagnosticBuilder;
}

namespace cv_ffi {

class Logger;

/// Move-only RAII handle wrapping the `ffi` crate's opaque
/// `DiagnosticBuilder*` (engine/rust/crates/ffi/src/diagnostic.rs). Unlike
/// Logger/Vfs, this wraps a genuinely single-use, self-consuming builder —
/// see the class comment on Emit()/Destroy() for what that means for this
/// type's lifetime.
///
/// Only `module`, `file`, `line`, and `persistent` are exposed, matching the
/// Rust-side FFI's own "deliberately small subset" (see ffi::diagnostic's
/// module docs) — `language`, `column`, `suggestion`, `documentation`, and
/// `extra` have no current caller here either. Add a setter the same way if
/// one gets a caller.
class DiagnosticBuilder
{
public:
    DiagnosticBuilder() noexcept = default;
    ~DiagnosticBuilder();

    DiagnosticBuilder(const DiagnosticBuilder&) = delete;
    DiagnosticBuilder& operator=(const DiagnosticBuilder&) = delete;

    DiagnosticBuilder(DiagnosticBuilder&& other) noexcept;
    DiagnosticBuilder& operator=(DiagnosticBuilder&& other) noexcept;

    /// Creates a new DiagnosticBuilder (see
    /// `log_core::DiagnosticBuilder::new`). Fails if `producer`, `category`,
    /// or `message` cannot be sent across the FFI boundary — practically,
    /// this only happens if one of them is not valid UTF-8, since these are
    /// C++ std::string_views, never null pointers.
    [[nodiscard]] static std::expected<DiagnosticBuilder, std::string> Create(
        std::string_view producer,
        std::string_view category,
        std::uint32_t number,
        Severity severity,
        std::string_view message
    );

    [[nodiscard]] bool IsValid() const noexcept
    {
        return m_handle != nullptr;
    }

    // Every setter below is rvalue-qualified (`&&`) and returns *this by
    // value-moved reference, mirroring the Rust builder's own
    // self-consuming `fn field(mut self, ...) -> Self` shape (see
    // ffi::diagnostic's module docs for why the *Rust* FFI wrapper needs an
    // Option<> to simulate that in the first place) — so the pattern here
    // is: `DiagnosticBuilder::Create(...)->Module(...).File(...).Emit(logger)`,
    // and a bare lvalue can only call these once each, by construction: a
    // second call would need another `&&`-qualified access, which an lvalue
    // cannot provide.

    [[nodiscard]] DiagnosticBuilder&& Module(std::string_view module) &&;
    [[nodiscard]] DiagnosticBuilder&& File(std::string_view file) &&;
    [[nodiscard]] DiagnosticBuilder&& Line(std::optional<std::uint32_t> line) &&;
    [[nodiscard]] DiagnosticBuilder&& Persistent(bool persistent) &&;

    /// Builds the diagnostic and emits it through `logger`, consuming this
    /// DiagnosticBuilder — same "single use" contract as the wrapped
    /// `ffi_diagnostic_builder_emit`: after this call the object holds no
    /// live handle (IsValid() becomes false) and calling Emit()/letting the
    /// destructor run again is a no-op, never a double-free, because
    /// ownership of the underlying pointer transferred into this call
    /// before the FFI call happened.
    void Emit(Logger& logger) &&;

private:
    explicit DiagnosticBuilder(::DiagnosticBuilder* handle) noexcept : m_handle(handle) {}

    void Release() noexcept;

    ::DiagnosticBuilder* m_handle = nullptr;
};

/// Registers a producer at runtime (see `log_core::producer::register_
/// dynamic`) so its code can appear in diagnostics built via
/// DiagnosticBuilder::Create(). Process-global, not tied to any particular
/// Logger. Fails if `code` collides with an existing static or dynamic
/// registration, or if any argument is not valid UTF-8.
[[nodiscard]] std::expected<void, std::string>
RegisterProducer(std::string_view code, std::string_view displayName, std::string_view organization);

} // namespace cv_ffi
