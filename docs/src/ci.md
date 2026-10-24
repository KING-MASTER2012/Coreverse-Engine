# Continuous integration

Workflows live in `.github/workflows/`; `ci.yml` is the entry point.

| Workflow                                                | What it does                                                                                                                                                                                                                             |
|:--------------------------------------------------------|:-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `rust.yml`                                              | Cargo check/test on Linux (bootstrap toolchain, plus the `no_std` math backend); `cargo test` on Windows and macOS; `cargo check` and Clippy of the `platform` crate for `aarch64-linux-android` and `aarch64-apple-ios` (nothing links) |
| `cpp.yml`                                               | C++/Qt builds and tests on Linux (GCC, Xvfb, software Vulkan with validation, Wayland test), Windows (MSVC) and macOS (Apple Clang)                                                                                                      |
| `lint.yml`                                              | Clippy with `-D warnings`, Rustdoc                                                                                                                                                                                                       |
| `formatting.yml`                                        | `cargo fmt --check`, clang-format over C++ and Objective-C++                                                                                                                                                                             |
| `codeql.yml`, `dependency-review.yml`                   | Security analysis, dependency review                                                                                                                                                                                                     |
| `deploy-docs.yml`                                       | Builds this book with mdBook and publishes it to GitHub Pages                                                                                                                                                                            |
| `benchmarks.yml`                                        | Manual: runs `cargo bench`                                                                                                                                                                                                               |
| `nightly.yml`, `release.yml`, `deploy.yml`, `stale.yml` | Scheduled / release automation                                                                                                                                                                                                           |

The CMake minimum, the Rust toolchain and LLVM versions come from `devkit/scripts/setup/config/tool-versions.json`.
