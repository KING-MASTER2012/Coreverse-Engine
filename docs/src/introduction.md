# Coreverse Engine

Coreverse is a hybrid game engine: **Rust** for the core systems, **C++23** for the editor and the
renderer, joined by a small C ABI (the `ffi` crate and its C++ wrappers).

This book describes what is **implemented today**. Planned work is tracked in
[`PROGRESS.md`](https://github.com/KING-MASTER2012/Coreverse-Engine/blob/main/PROGRESS.md) and the roadmap in the
repository `README.md`.

## Status

| Area                                                                                  | State                                                                                                                                                                                                                                                         |
|:--------------------------------------------------------------------------------------|:--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Rust core (`ccore`, `fs`, `vfs`, `log-core`, `log-sinks`)                             | Implemented and tested                                                                                                                                                                                                                                        |
| `math` (vectors, matrices, quaternions, geometry, curves, noise)                      | Implemented and tested; mirrored in C++ (`cv-math`)                                                                                                                                                                                                           |
| `platform` (system info, time, standard directories, env, process, thread)            | Implemented and tested on Linux; Windows/macOS in CI; Android/iOS compile-checked only; window API with a headless and a `winit` desktop backend (opened for real only on Linux/X11 in CI); input events with a per-frame query layer; no mobile backends yet |
| FFI (`ffi` crate, `cv-ffi` C++ wrappers: logging, VFS, platform info and directories) | Implemented and tested                                                                                                                                                                                                                                        |
| Renderer (Vulkan)                                                                     | Graphics pass with depth, pipelines, vertex/index buffers, push constants; device-loss recovery                                                                                                                                                               |
| Editor (Qt 6)                                                                         | Viewport that draws a spinning cube; no project/scene/asset workflows yet                                                                                                                                                                                     |
| Other Rust crates (`ecs`, `asset`, `audio`, `physics`, ...)                           | Placeholders                                                                                                                                                                                                                                                  |

## Where to go next

* New to the code base: [Architecture](architecture.md), then [Building and testing](building.md).
* Touching the GPU code: [Rendering](rendering.md).
* Using vectors/matrices from Rust or C++: [Math](math.md).
* Asking the operating system for time, directories or system information: [Platform layer](platform.md).
