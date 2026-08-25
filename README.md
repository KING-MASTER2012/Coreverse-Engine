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
* OpenGL renderer where applicable
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

| Graphics API |            Status            | Intended Platforms                     |
| :----------- | :--------------------------: | :------------------------------------- |
| **Vulkan**   |          🟢 Primary          | Linux, Windows, future Android targets |
| **OpenGL**   | 🟢 Supported / Compatibility | Cross-platform fallback                |
| **Metal**    |          🔵 Planned          | macOS / Apple platforms                |
| **Direct3D** |          🔵 Planned          | Windows                                |

The renderer is designed around explicit graphics APIs, multithreaded workloads, predictable resource lifetime, and minimal abstraction overhead.

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

CI pipelines are used to verify:

* Supported platform builds
* Compiler compatibility
* Static analysis
* Linting
* Warnings and diagnostics
* Cross-platform regressions
* Dependency integration
* Release configuration

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

A simplified structure is:

```text
coreverse-engine/
├── .github/
│   ├── workflows/          # CI/CD
│   ├── ISSUE_TEMPLATE/
│   └── ...
│
├── assets/
│   └── emblems/
│
├── docs/                   # Technical documentation
│
├── engine/
│   ├── cpp/                # C++23 native engine & renderer
│   └── rust/               # Rust 2024 engine systems
│
├── editor/                 # Qt 6 / C++ Engine Editor
│
├── scripts/                # Development and build utilities
│
├── third_party/            # External repositories / integrations
│
├── CMakeLists.txt
├── CMakePresets.json
├── Cargo.toml
├── vcpkg.json
├── LICENSE
└── README.md
```

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

Install the following:

* **Rust** — Rust 2024 Edition / current stable toolchain
* **CMake**
* **Ninja**
* **vcpkg**
* **Qt 6**
* A supported platform compiler:

  * Linux → GCC
  * Windows → MSVC
  * macOS → Apple Clang
* Git

Additional tooling is recommended for development and diagnostics:

* Clang / LLVM
* `clang-tidy`
* LLDB
* GDB
* Visual Studio Debugger on Windows

### Clone

```bash
git clone https://github.com/KING-MASTER2012/Coreverse-Engine.git
cd Coreverse-Engine
```

### Configure

The repository uses CMake with Ninja and vcpkg.

A typical configuration looks like:

```bash
cmake -S . -B build \
  -G Ninja \
  -DCMAKE_BUILD_TYPE=Release
```

### Build

```bash
cmake --build build
```

For development builds:

```bash
cmake -S . -B build \
  -G Ninja \
  -DCMAKE_BUILD_TYPE=Debug

cmake --build build
```

> Prefer the repository's CMake presets when available, as they provide the canonical project configuration for each platform and toolchain.

---

## 🟢 Documentation

Technical documentation is maintained under [`docs/`](docs/).

Recommended documentation areas include:

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
