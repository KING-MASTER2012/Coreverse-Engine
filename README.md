<div style = "text-align: center">

<img src="assets/emblems/coreverse-emblem.svg" width="250" alt="Coreverse Engine Logo">

<h1>Coreverse Engine</h1>

<h3>Core & Universe</h3>

<p><strong>Build Worlds Without Limits.</strong><br>
A native, open-source game engine built around Rust, C++, Qt, Vulkan, and modern cross-platform tooling.</p>

<p>
  <a href="https://www.gnu.org/licenses/gpl-3.0">
    <img src="https://img.shields.io/badge/License-GPLv3-blue.svg?style=for-the-badge" alt="License: GPL v3">
  </a>
  <a href="https://www.rust-lang.org/">
    <img src="https://img.shields.io/badge/Rust-2024-orange.svg?style=for-the-badge&logo=rust" alt="Rust 2024">
  </a>
  <a href="https://isocpp.org/">
    <img src="https://img.shields.io/badge/C%2B%2B-23-00599C.svg?style=for-the-badge&logo=c%2B%2B" alt="C++23">
  </a>
  <a href="https://www.qt.io/">
    <img src="https://img.shields.io/badge/Qt-6-41CD52.svg?style=for-the-badge&logo=qt" alt="Qt 6">
  </a>
  <a href="https://cmake.org/">
    <img src="https://img.shields.io/badge/CMake-Ninja-064F8C.svg?style=for-the-badge&logo=cmake" alt="CMake + Ninja">
  </a>
  <a href="https://vcpkg.io/">
    <img src="https://img.shields.io/badge/Dependencies-vcpkg-16437E.svg?style=for-the-badge" alt="vcpkg">
  </a>
</p>

</div>

---

## 🎨 Project Status Legend

| Color  | Meaning                                                                           |
|:------:| :-------------------------------------------------------------------------------- |
|   🟢   | **Active / Current** — Implemented or actively maintained                         |
|   🔵   | **Target / Planned** — Part of the intended architecture or roadmap               |
|   🟡   | **Development / Experimental** — Under active implementation or subject to change |
|   ⚪   | **External / Separate Repository** — Deliberately outside this repository         |

The status indicators below describe the **current direction of the Coreverse Engine repository**, not the entire Coreverse ecosystem.

---

## 🟢 Overview

**Coreverse Engine** is the native engine and editor repository of the Coreverse ecosystem.

The project focuses on the systems required to build, run, inspect, and render interactive worlds while keeping the architecture modular and performance-oriented.

Coreverse uses a deliberately asymmetric hybrid architecture:

* **Rust** handles the engine's background and systems-oriented infrastructure.
* **C++23 + Qt 6** powers the editor and rendering layer.
* **C++ calls Rust through a stable FFI boundary.**
* **Rust does not depend on or call back into the C++ layer.**
* **CMake + Ninja** provide the native build orchestration.
* **vcpkg** manages native third-party dependencies.
* Platform-native compilers remain the authoritative production toolchains, while **Clang/LLVM is also used for diagnostics, static analysis, tooling, and verification**.

The result is a native engine architecture that combines Rust's safety and concurrency model with the mature graphics, tooling, and desktop ecosystem of C++ and Qt.

> **Coreverse Engine is the Engine repository.**
> The Launcher, Website, cloud services, backend services, and external scripting ABI repositories are maintained separately.

---

## 🟢 Current Status

An honest snapshot of what exists today (details and the prioritized to-do list live in [`PROGRESS.md`](PROGRESS.md)):

| Area                                                                                                                                                                                               | State                                                                                                                                                                                                                                                                                                                                                                                                                                    |
|:---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|:-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **Rust core** (`ccore`, `fs`, `vfs`, `log-core`, `log-sinks`)                                                                                                                                      | 🟢 Implemented and tested                                                                                                                                                                                                                                                                                                                                                                                                                |
| **Math** (`math`: vectors, matrices, quaternions, geometry, curves, noise, ... ; `std` and `no_std`/`libm` backends, optional `serde`)                                                             | 🟢 Implemented, tested and benchmarked (criterion); the C++ mirror `cv-math` (Vec/Quat/Mat4) is verified against it by a shared golden file                                                                                                                                                                                                                                                                                              |
| **Platform** (`platform`: OS/CPU/memory/display-server info, clocks and timers, standard directories, environment, process and thread helpers; support matrix in the [book](docs/src/platform.md)) | 🟢 Implemented and tested (Linux; Windows/macOS in CI); Android/iOS compile-checked in CI only. Window API (`platform::window`: `App` lifecycle, `WindowEvent` incl. surface lost/created, `NativeWindowHandle`, monitors, cursor, fullscreen) with a headless backend and a `winit` desktop backend (feature `winit`; smoke-tested on Linux/X11 under Xvfb in CI, only compiled on Windows and macOS). No input, no mobile backends yet |
| **FFI** (`ffi` crate + C++ RAII wrappers `cv-ffi`: logger, diagnostics, VFS, platform info and directories, build info)                                                                            | 🟢 Implemented and tested                                                                                                                                                                                                                                                                                                                                                                                                                |
| **Renderer** (Vulkan backend: device, buffers, surface, swapchain + depth buffer, frame sync, graphics pipelines, push constants, validation as a test signal)                                     | 🟢 First graphics pass (Vulkan 1.3 dynamic rendering), device-loss recovery; no descriptor sets/textures/shader loading from the VFS yet                                                                                                                                                                                                                                                                                                 |
| **Editor** (Qt 6 window with a native Vulkan viewport and render loop)                                                                                                                             | 🟡 Early: the viewport draws a spinning cube; no project/scene/asset workflows yet                                                                                                                                                                                                                                                                                                                                                       |
| **Other Rust crates** (`ecs`, `asset`, `audio`, `physics`, `runtime`, `scripting`, ...)                                                                                                            | 🔵 Placeholders (documented skeletons that inherit the workspace settings and lints); `acse` (Auto Connecting System for ECS) and `aose` (Auto from OOP convert System to ECS) are the planned ECS macro systems                                                                                                                                                                                                                         |
| **Documentation** (mdBook, `docs/` + `book.toml`)                                                                                                                                                  | 🟢 Book builds; published by `deploy-docs.yml`                                                                                                                                                                                                                                                                                                                                                                                           |
| **CI** (Linux/Windows/macOS builds, Rust tests, Clippy, formatting, CodeQL)                                                                                                                        | 🟢 Green                                                                                                                                                                                                                                                                                                                                                                                                                                 |

---

## 🟢 Architecture

Coreverse is split into two primary native layers.

### 🦀 Rust — Engine Systems

Rust is responsible for the engine's background and infrastructure-oriented systems.

Typical responsibilities include:

* Core engine services
* Virtual File System (VFS)
* Asset and project infrastructure
* Job and task systems
* Event and messaging infrastructure
* Resource management
* Configuration systems
* Runtime services
* Other concurrency-sensitive or infrastructure-heavy subsystems

Rust provides:

* Memory safety
* Data-race prevention
* Strong ownership semantics
* Modern concurrency primitives
* Reliable error handling
* Safe-by-default abstractions

### ⚙️ C++23 + Qt 6 — Editor & Rendering

The native C++ layer is responsible for the systems that require close integration with graphics APIs and Qt.

This includes:

* Qt-based Engine Editor
* Rendering infrastructure
* Vulkan renderer
* Graphics resource management
* GPU synchronization
* Render scheduling
* Editor viewport systems
* Scene and tooling interfaces
* Native platform integration

Qt is used throughout the desktop-facing editor and associated native UI infrastructure.

### 🔗 FFI Boundary

The architectural dependency direction is intentional:

```text
┌───────────────────────────────────────────────┐
│               C++23 / Qt 6                    │
│                                               │
│  Editor • Renderer • Native Tooling           │
└──────────────────────┬────────────────────────┘
                       │
                       │ C / FFI boundary
                       ▼
┌───────────────────────────────────────────────┐
│                  Rust 2024                    │
│                                               │
│  Core Systems • VFS • Jobs • Events • Runtime │
└───────────────────────────────────────────────┘
```

The C++ layer consumes Rust APIs through an explicit ABI boundary.

Rust remains independent of C++ at the architectural level. This keeps the ownership model, dependency graph, and compilation boundaries easier to reason about.

---

## 🟢 Core Principles

### 1. Safety Without Sacrificing Native Performance

Rust is used where memory safety and concurrency provide significant architectural value.

C++ is used where direct integration with Qt, graphics APIs, platform SDKs, and established native tooling is advantageous.

### 2. Native First

Coreverse is a native engine rather than a browser-based runtime or managed application framework.

The Engine and Editor are designed around:

* Native binaries
* Native graphics APIs
* Native filesystem access
* Native threading
* Native debugging
* Native profiling

### 3. Explicit Boundaries

The Rust/C++ interface is deliberately explicit.

The architecture avoids unnecessary bidirectional dependencies and keeps cross-language communication concentrated around well-defined interfaces.

### 4. Tooling Is Part of the Engine

Compilers, debuggers, static analyzers, sanitizers, linters, and IDE tooling are treated as part of the development architecture rather than optional extras.

### 5. Cross-Platform by Design

Linux is the primary development and build platform, while Windows and macOS remain first-class targets with their appropriate native toolchains.

---

## 🟢 Rendering

Coreverse's rendering layer is implemented in native C++ and integrated into the Qt-based Engine Editor.

### Current Graphics Direction

| Graphics API |           Status            | Intended Platforms                     |
| :----------- |:---------------------------:|:---------------------------------------|
| **Vulkan**   |  🟢 Implemented (primary)   | Linux, Windows, macOS via MoltenVK     |
| **OpenGL**   |         🔵 Planned          | Cross-platform fallback                |
| **Metal**    | 🔵 Planned (native backend) | macOS / Apple platforms                |
| **Direct3D** |         🔵 Planned          | Windows                                |

The renderer sits behind a backend-neutral `RenderDevice` interface; today only the Vulkan backend exists (on macOS it presents through MoltenVK into a `CAMetalLayer`). It is designed around explicit graphics APIs, multithreaded workloads, predictable resource lifetime, and minimal abstraction overhead.

---

## 🟢 Engine Editor

The Coreverse Editor is a **native Qt 6 / C++ application** rather than a web application.

The Editor is intended to provide:

* Scene editing
* Entity and component inspection
* Asset management
* Resource inspection
* Rendering viewport
* Engine diagnostics
* Project configuration
* Profiling and debugging workflows
* Future visual scripting integration

Qt provides the desktop UI foundation while the underlying Engine systems remain separated into their native subsystems.

---

## 🟢 Build System

Coreverse uses **CMake + Ninja** as its native build system.

### Build Stack

```text
CMake
   │
   ├── Ninja
   │
   ├── C++23 / Qt 6
   │
   ├── Rust 2024
   │
   └── vcpkg
```

### CMake

CMake is responsible for:

* Project configuration
* Native target generation
* Rust/C++ integration
* Qt integration
* Platform-specific configuration
* Compiler configuration
* Dependency integration
* Installation and packaging infrastructure

The minimum CMake version is **4.4** — the single source of truth is `cmake.minVersion` in [`devkit/scripts/setup/config/tool-versions.json`](devkit/scripts/setup/config/tool-versions.json), which the bootstrap scripts install and verify; `CMakeLists.txt` and `CMakePresets.json` declare the same minimum. Platform configuration goes through the presets in [`CMakePresets.json`](CMakePresets.json) (`linux-x64-gcc`, `linux-x64-clang`, `windows-x64`, `windows-x64-clang`, `windows-x64-clang-gnu`, `windows-x64-static`, `macos-arm64-clang`).

### Ninja

Ninja is used as the underlying high-performance build executor.

This keeps incremental builds fast and makes the build graph explicit and reproducible.

### vcpkg

**vcpkg** manages native third-party C/C++ dependencies and integrates directly with the CMake-based build.

The repository therefore avoids depending on manually configured system copies of arbitrary C++ libraries wherever practical.

---

## 🟢 Compiler & Toolchain Strategy

Coreverse uses the **native compiler ecosystem of each operating system as the primary production toolchain**.

At the same time, Clang/LLVM is used as an additional verification and analysis toolchain.

| Platform       | Primary Compiler | Secondary / Verification |
| :------------- | :--------------- | :----------------------- |
| 🐧 **Linux**   | **GCC**          | Clang / LLVM             |
| 🪟 **Windows** | **MSVC**         | Clang / LLVM             |
| 🍎 **macOS**   | **Apple Clang**  | Clang tooling            |

### Linux

**GCC** is the primary compiler and primary production build toolchain.

Clang is additionally used for:

* Static analysis
* `clang-tidy`
* Compilation verification
* Additional diagnostics
* LLVM-based tooling

### Windows

**MSVC** is the primary compiler and production build toolchain.

Clang/LLVM is additionally used for:

* Static analysis
* `clang-tidy`
* Additional compiler diagnostics
* LLVM tooling
* Cross-checking problematic code paths

Visual Studio's native toolchain and debugger remain first-class development tools.

### macOS

**Apple Clang** is used as the primary compiler in accordance with the native Apple development ecosystem.

---

## 🟢 Developer & CI Verification

Toolchain validation does not depend exclusively on a single compiler.

Coreverse uses both local developer checks and CI/CD verification.

### Developer Environment

Developers can use:

* Native compiler diagnostics
* Clang diagnostics
* `clang-tidy`
* LLVM tooling
* GDB
* LLDB
* Visual Studio Debugger
* Qt debugging tools
* Platform-specific profiling and diagnostic tools

### CI/CD

The pipeline in [`.github/workflows/`](.github/workflows/) (entry point `ci.yml`) consists of:

| Workflow                                                                                     | What it does                                                                                                                                       |
|:---------------------------------------------------------------------------------------------|:---------------------------------------------------------------------------------------------------------------------------------------------------|
| `rust.yml`                                                                                   | Cargo check/test for the whole workspace on Linux (bootstrap toolchain, including the `no_std` math backend) and `cargo test` on Windows and macOS |
| `cpp.yml`                                                                                    | C++/Qt builds and tests: Linux (GCC, Vulkan validation layers, Xvfb editor smoke test, Wayland test), Windows (MSVC), macOS (Apple Clang)          |
| `lint.yml`                                                                                   | Clippy with `-D warnings` and Rustdoc for `math`                                                                                                   |
| `formatting.yml`                                                                             | `cargo fmt --check` and `clang-format` over C++ **and** Objective-C++ (`*.mm`) sources                                                             |
| `codeql.yml`, `dependency-review.yml`                                                        | Security analysis and dependency review                                                                                                            |
| `nightly.yml`, `release.yml`, `deploy.yml`, `deploy-docs.yml`, `benchmarks.yml`, `stale.yml` | Scheduled/manual/release automation (benchmarks: manual `cargo bench`; documentation: mdBook to GitHub Pages)                                      |

CI pipelines are used to verify:

* Supported platform builds
* Compiler compatibility
* Linting
* Formatting
* Warnings and diagnostics
* Cross-platform regressions
* Dependency integration

The goal is not merely to produce a successful build, but to continuously validate the codebase against multiple diagnostic ecosystems.

---

## 🟢 Platform Matrix

### Operating Systems

| Platform             |    Status    | Primary Toolchain |        Development Priority         |
| :------------------- |:------------:| :---------------- |:-----------------------------------:|
| 🐧 **Linux**         | 🟢 Supported | GCC               |             **Primary**             |
| 🪟 **Windows**       | 🟢 Supported | MSVC              | **Secondary / Official Validation** |
| 🍎 **macOS**         | 🟢 Supported | Apple Clang       |    **Cross-platform Validation**    |
| 📱 **Android / iOS** |  🔵 Future   | Platform-specific |               Future                |

Linux is the primary development environment for Coreverse Engine.

Windows remains a first-class target and is continuously validated with MSVC.

macOS follows the native Apple toolchain through Apple Clang.

---

## 🟢 Repository Scope

This repository contains the **Coreverse Engine and its native Editor infrastructure**.

The main directories are:

```text
coreverse-engine/
├── .github/workflows/      # CI/CD
├── assets/                 # Branding (emblems)
├── cmake/                  # CMake modules (toolchains, Rust integration, clang tools, ...)
├── devkit/scripts/         # Bootstrap, configure and format scripts (setup/config/*.json)
├── docs/                   # mdBook sources (docs/src)
├── engine/
│   ├── cpp/                # C++23: editor (Qt 6), renderer (Vulkan), ffi-cpp (cv-ffi)
│   └── rust/               # Rust 2024 workspace: common/ (ccore, fs, ...), crates/ (vfs, ffi, math, ...), macros/
├── tests/                  # C++ tests (ctest) and shared Rust test helpers
├── tools/                  # Developer tools (project-generator)
├── CMakeLists.txt
├── CMakePresets.json
├── Cargo.toml              # Rust workspace
├── PROGRESS.md             # Change log and prioritized to-do list
├── vcpkg.json              # Native dependency manifest
├── LICENSE
└── README.md
```

Placeholder directories (`benchmarks/`, `examples/`, `formats/`, `fuzz/`, `omniscript/`, `plugins/`, `templates/`, `test_assets/`) exist for planned work and are empty for now. The vcpkg checkout lives in `third_party/vcpkg` (cloned by bootstrap, not committed).

> The exact directory structure may evolve as the Engine architecture matures.

---

## ⚪ Separate Coreverse Repositories

The following components are intentionally **not part of this repository**:

| Component                           | Repository Role                              |
| :---------------------------------- | :------------------------------------------- |
| **Coreverse Launcher**              | Desktop project/engine launcher              |
| **Coreverse Website**               | Public website and web infrastructure        |
| **Coreverse Backend / Services**    | Online services and server infrastructure    |
| **Scripting ABI / API**             | Cross-language scripting interface and ABI   |
| **Cloud / Database Infrastructure** | Backend storage and online platform services |

This separation keeps the Engine repository focused on native engine development instead of turning it into a monolithic ecosystem repository.

---

## 🟢 Development Workflow

A typical development workflow looks like:

```text
Developer
   │
   ▼
CMake Configuration
   │
   ├── vcpkg dependencies
   │
   ├── Rust targets
   │
   └── C++ / Qt targets
   │
   ▼
Ninja Build
   │
   ▼
Native Engine + Editor
   │
   ├── Compiler diagnostics
   ├── Clang tooling
   ├── Debugger
   └── Runtime validation
   │
   ▼
CI/CD
   │
   ├── Linux / GCC
   ├── Windows / MSVC
   ├── macOS / Apple Clang
   └── Additional Clang verification
```

---

## 🟢 Getting Started

### Prerequisites

The exact minimum versions are pinned in [`devkit/scripts/setup/config/tool-versions.json`](devkit/scripts/setup/config/tool-versions.json) and enforced by the bootstrap scripts. At the time of writing:

* **Rust** — Rust 2024 Edition, cargo ≥ 1.97.1 (via rustup)
* **CMake** ≥ 4.4.0 and **Ninja**
* **vcpkg** (cloned by bootstrap into `third_party/vcpkg`)
* **Qt 6** (installed through vcpkg)
* **LLVM/Clang 22.1.8** (`clang-format`, `clang-tidy`, LLDB — formatting output differs between LLVM major versions, so CI and the format scripts use this one)
* A supported platform compiler:

    * Linux → GCC
    * Windows → MSVC (Visual Studio)
    * macOS → Apple Clang
* Git

### Clone

```bash
git clone https://github.com/KING-MASTER2012/Coreverse-Engine.git
cd Coreverse-Engine
```

### Bootstrap

The easiest path is the bootstrap script. It detects/installs the toolchain, clones vcpkg and configures CMake:

```bash
# Linux
./devkit/scripts/setup/shell/bootstrap.sh

# Windows (PowerShell 7+)
./devkit/scripts/setup/powershell/bootstrap.ps1
```

Add `--dry-run` (`-DryRun` on Windows) to only see what would happen. On macOS, install the tools with Homebrew instead (see the `macos` job in [`cpp.yml`](.github/workflows/cpp.yml) for the exact list); the macOS path of `bootstrap.sh` is not verified yet.

### Configure & Build

Use the CMake presets — they are the canonical configuration for each platform and toolchain:

```bash
cmake --preset linux-x64-gcc               # or windows-x64, macos-arm64-clang, ...
cmake --build --preset linux-gcc-release   # see `cmake --build --list-presets`
```

The wrapper scripts `devkit/scripts/configure-linux.sh`, `configure-macos.sh` and `configure-windows.ps1` add architecture sanity checks around the same presets.

### Test

```bash
cargo test --workspace --all-features                              # Rust
cargo test -p math --no-default-features --features no_std         # math, libm backend
ctest --preset linux-gcc-release-nogpu                             # C++ tests that need no GPU (incl. cv-math vs. the Rust math crate)
```

The GPU/editor tests (`ctest --preset linux-gcc-release`) need a Vulkan driver; CI runs them with software Vulkan under Xvfb. They include the graphics pass test and the editor smoke tests (one of them loses the device on purpose and requires recovery). Set `CV_REQUIRE_VALIDATION=1` to fail when the Vulkan validation layer is not running.

Benchmarks: `cargo bench -p math`. Documentation: `mdbook build` (see [`docs/`](docs/)).

### Format

```bash
devkit/scripts/format.sh            # Rust (cargo fmt) + C++/Objective-C++ (clang-format), in place
devkit/scripts/format.sh --check    # what CI runs
```

On Windows use `devkit/scripts/format.ps1` (`-Check`). The CMake targets `format`, `format-check` and `tidy` cover the same file sets.

---

## 🟢 Documentation

Technical documentation is an mdBook (`book.toml`, pages in [`docs/src/`](docs/src/SUMMARY.md)): introduction, architecture, building and testing, FFI, the platform layer, rendering, math, editor and CI. Build it with `mdbook build`; `deploy-docs.yml` publishes it to GitHub Pages.

Areas the book should grow into include:

* **Engine Architecture**
* **Rust/C++ FFI Architecture**
* **Rendering Architecture**
* **ECS and Runtime Systems**
* **Build System**
* **Platform Support**
* **Editor Architecture**
* **Development & Contribution Guidelines**

Documentation should describe the actual implementation and current architecture rather than future plans presented as completed functionality.

---

## 🟡 Roadmap

The roadmap evolves together with the engine architecture.

### Rendering

* [ ] Advanced Vulkan rendering pipeline
* [ ] GPU-driven rendering
* [ ] Advanced synchronization and resource management
* [ ] Hardware ray tracing
* [ ] Metal backend
* [ ] Direct3D backend

### Engine Systems

* [ ] Expanded ECS/runtime infrastructure
* [ ] Advanced asynchronous job scheduling
* [ ] Asset dependency graph
* [ ] Improved hot-reloading
* [ ] Runtime profiling infrastructure
* [ ] Expanded serialization and project systems

### Editor

* [ ] Advanced Scene Editor
* [ ] Integrated profiling tools
* [ ] Resource inspectors
* [ ] Visual scripting
* [ ] Advanced rendering diagnostics
* [ ] Editor extensibility APIs

### Platform Support

* [ ] Expanded macOS validation
* [ ] Mobile runtime architecture
* [ ] Additional graphics backends
* [ ] Broader hardware compatibility

---

## 🟡 Project Philosophy

### ⚡ Performance

Runtime abstractions should justify their cost.

### 🦀 Safety

Memory safety and concurrency safety are architectural priorities, particularly in core and infrastructure systems.

### 🧩 Modularity

Engine subsystems should remain independently understandable, testable, and replaceable.

### 🛠️ Developer Experience

Fast builds, useful diagnostics, strong tooling, and predictable project structure are core engineering requirements.

### 🌍 Cross-Platform

Platform-specific behavior should be isolated rather than allowed to leak throughout the codebase.

### 🔍 Verification

A successful build from one compiler is not considered sufficient evidence of correctness.

Coreverse continuously benefits from multiple compiler, analyzer, debugger, and CI environments.

---

## 🟢 Contributing

Contributions are welcome across:

* Engine systems
* Rust infrastructure
* C++ rendering
* Qt Editor development
* Build infrastructure
* Platform support
* Diagnostics and tooling
* Documentation
* Tests and validation

Before contributing, review:

* [Contribution Guidelines](.github/CONTRIBUTING.md)
* [Code of Conduct](.github/CODE_OF_CONDUCT.md)

A typical workflow is:

```bash
git checkout -b feature/your-feature
git add .
git commit -m "feat(renderer): improve command submission"
git push origin feature/your-feature
```

Then open a Pull Request against the appropriate branch.

---

## 🟢 License

Coreverse Engine is released under the **GNU General Public License v3.0 (GPL-3.0)**.

See [`LICENSE`](LICENSE) for the complete license text.

---

<div style = "text-align: center">

<h3>Coreverse Engine</h3>

<p><em>Core & Universe — Build Worlds Without Limits.</em></p>

<p>
  <strong>Rust.</strong> <strong>C++.</strong> <strong>Qt.</strong> <strong>Vulkan.</strong> <strong>Native by design.</strong>
</p>

</div>
