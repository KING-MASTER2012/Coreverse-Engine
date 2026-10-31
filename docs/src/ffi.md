# Rust and C++ FFI

The `ffi` crate exposes a C ABI; `cbindgen` generates `ffi.h` from it at build time (`cmake/FfiHeader.cmake`) and
Corrosion links the static library. The C++ side never calls the raw functions directly: `engine/cpp/ffi-cpp`
(`cv-ffi`) wraps them.

| Area                                          | Rust                           | C++ wrapper                                                                |
|:----------------------------------------------|:-------------------------------|:---------------------------------------------------------------------------|
| Logging                                       | `log-core`, `log-sinks`        | `cv_ffi::Logger`, `cv_ffi::DiagnosticBuilder`, `cv_ffi::RegisterProducer`  |
| Virtual file system                           | `vfs`                          | `cv_ffi::vfs::*`                                                           |
| Platform information and standard directories | `platform`                     | `cv_ffi::platform::Info()`, `cv_ffi::platform::StandardDir()`              |
| Window event loop and input                   | `platform` (`window`, `input`) | `cv_ffi::window::Run()`, `Context`, `Event` (see [Runtime](runtime.md))    |
| Build info                                    | `ffi`                          | `cv_ffi::BuildInfoString()`                                                |

## Platform functions

| C function                                                        | C++ wrapper                           | Returns                                                                                                                                                                                          |
|:------------------------------------------------------------------|:--------------------------------------|:-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `ffi_platform_info(PlatformInfo*)`, `ffi_platform_info_free`      | `platform::Info()`                    | `std::expected<SystemInfo, std::string>`: OS name/version/kernel, logical and physical cores, memory, display server, plus the compile-time platform, architecture, endianness and pointer width |
| `ffi_platform_dir(kind, app)`                                     | `platform::StandardDir(DirKind, app)` | `std::expected<std::string, std::string>`: the config, data, cache, log, temp, home or executable directory of an application (nothing is created)                                               |
| `ffi_platform_last_error_code`, `ffi_platform_last_error_message` | used inside the wrappers              | the reason for the most recent failure on the calling thread                                                                                                                                     |

Conventions, shared with the VFS functions:

* **Enums are `u8`.** The values are the `FFI_PLATFORM_*`, `FFI_ARCH_*`, `FFI_ENDIAN_*`, `FFI_DISPLAY_*` and `FFI_DIR_*`
  constants of `ffi.h`; the C++ enums mirror them, `static_assert`s pin the directory kinds, and an unknown value is
  mapped to `Unknown`.
* **Unknown numbers are `0`.** `physical_cores` and `total_memory_bytes` are `0` when the platform does not report them;
  the C++ side turns that into an unset `std::optional`.
* **Errors use a thread-local slot.** A failing call returns `false` or null; the code (`FFI_PLATFORM_ERROR_*`, `255`
  for none) and a message are read right afterwards on the same thread. A bad `kind` byte, a null or non-UTF-8 `app`
  and an unusable application name are reported as errors, not undefined behavior.
* **Ownership.** Strings are freed with `ffi_free_string`; the three strings inside `PlatformInfo` are freed by
  `ffi_platform_info_free` (which nulls them, so a second call is harmless). `cv_ffi::platform` does both for you.

`cv-ffi` needs only the Rust `ffi` library, not Qt, so CMake builds it whenever `BUILD_EDITOR`, `BUILD_RUNTIME` or
`BUILD_TESTS` is on; its tests (`cv_ffi_platform_test` and `cv_ffi_window_test`, label `ffi`) therefore also run in a headless configuration
(`-DBUILD_EDITOR=OFF -DBUILD_RENDERER=OFF -DBUILD_TESTS=ON`). See [Platform layer](platform.md) for what the functions
report on each operating system.

## Window event loop

`ffi_platform_run`, the callback table and the `ffi_platform_ctx_*` functions run the window event loop of the `platform`
crate for C++. They have their own chapter, [Runtime](runtime.md), because the lifetime rules of the context and of
event data matter more than the signatures. They follow the same conventions as above: a function returns `false` on
failure and the reason is read from the thread-local last error, which has two more codes for this area,
`FFI_PLATFORM_ERROR_WINDOW` (`7`, a window operation failed or the window system could not start) and
`FFI_PLATFORM_ERROR_PANIC` (`8`, a Rust panic was caught at the boundary).

## Renderer and logging

The renderer does not depend on this layer. The editor installs a log callback on the render device that forwards
messages (including Vulkan validation output) to the Rust logger.

`math` is *not* exposed through the FFI: the C++ side has its own header-only implementation with an identical
memory layout, so matrices can cross the boundary as plain floats. The golden test keeps both in sync
(see [Math](math.md)).
