#pragma once

#include <cstdint>

namespace cv_ffi {

/// Mirrors `log_core::Severity` (engine/rust/crates/log-core/src/severity.rs)
/// exactly — same order, same numeric values (`Severity::as_u8`/`from_u8`
/// on the Rust side) — since every function in this library that takes or
/// returns a severity just forwards the `u8` across the FFI boundary. Keep
/// the two in sync if either changes.
enum class Severity : std::uint8_t {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Notice = 3,
    Warning = 4,
    Error = 5,
    Critical = 6,
    Fatal = 7,
};

} // namespace cv_ffi
