# Architecture

```text
engine/
├── rust/            Rust 2024 workspace
│   ├── common/      ccore, fs, log-core, log-sinks ...
│   ├── crates/      vfs, ffi, math, platform, and placeholder crates for future systems
│   └── macros/      proc macros (vfs-macros, math-macros)
└── cpp/             C++23
    ├── math/        cv-math: header-only mirror of the Rust math crate
    ├── renderer/    Vulkan renderer library
    ├── ffi-cpp/     cv-ffi: RAII wrappers over the Rust C ABI
    └── editor/      Qt 6 editor
```

## Layers

* **Rust core** owns things that benefit from memory safety and testing: the virtual file system, logging,
  filesystem helpers, math, and the operating-system abstraction (`platform`: system info, clocks and timers,
  standard directories, process and thread helpers; see [Platform layer](platform.md)).
* **`ffi`** is a C ABI over the Rust core (`cbindgen` generates `ffi.h`; Corrosion builds and links the static
  library from CMake). It exposes logging, the VFS, build info, and platform information and directories; the other
  Rust crates are not exposed to C++.
* **`cv-ffi`** wraps that ABI in C++ RAII types (`Logger`, `DiagnosticBuilder`, `vfs::*`, `platform::*`).
* **`renderer`** is deliberately independent of Rust and of Qt. It talks to the GPU through the backend-neutral
  `RenderDevice` interface; only a Vulkan backend exists.
* **`cv-math`** is header-only and dependency-free, so the renderer, the editor and tests can all use it.
* **`editor`** is the only Qt code. It routes renderer log messages into the Rust logger through `cv-ffi`.

## Conventions

* Rust: edition 2024, workspace lints (`missing_docs` and `unsafe_code` are denied; the `ffi`/`fs` crates opt in to
  `unsafe` where they must).
* C++: C++23, `std::expected` for errors (no exceptions across the renderer API), clang-format and clang-tidy
  configuration in the repository root.
* One implementation per concept: the C++ math types are checked against the Rust ones by a shared golden file
  (see [Math](math.md)).
