# Rust and C++ FFI

The `ffi` crate exposes a C ABI; `cbindgen` generates `ffi.h` from it at build time (`cmake/FfiHeader.cmake`) and
Corrosion links the static library. The C++ side never calls the raw functions directly: `engine/cpp/ffi-cpp`
(`cv-ffi`) wraps them.

| Area | Rust | C++ wrapper |
| :--- | :--- | :---------- |
| Logging | `log-core`, `log-sinks` | `cv_ffi::Logger`, `cv_ffi::DiagnosticBuilder`, `cv_ffi::RegisterProducer` |
| Virtual file system | `vfs` | `cv_ffi::vfs::*` |
| Build info | `ffi` | `cv_ffi::BuildInfo` |

The renderer does not depend on this layer. The editor installs a log callback on the render device that forwards
messages (including Vulkan validation output) to the Rust logger.

`math` is *not* exposed through the FFI: the C++ side has its own header-only implementation with an identical
memory layout, so matrices can cross the boundary as plain floats. The golden test keeps both in sync
(see [Math](math.md)).
