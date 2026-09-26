#pragma once

#include <string>

namespace cv_ffi {

    /// Wraps `ffi_build_info_string()`/`ffi_free_string()` (git hash, profile,
    /// target, build date — see `ccore::BuildInfo`) into a single call that
    /// returns an owned std::string, so callers never handle the raw
    /// Rust-allocated `char*` (or its matching free) directly.
    [[nodiscard]] std::string BuildInfoString();

} // namespace cv_ffi
