# Building and testing

The exact tool versions are pinned in `devkit/scripts/setup/config/tool-versions.json`. The CMake minimum is **4.4**
(declared in `CMakeLists.txt`, `CMakePresets.json` and that file).

## Set up

```bash
# Linux
./devkit/scripts/setup/shell/bootstrap.sh
# Windows (PowerShell 7+)
./devkit/scripts/setup/powershell/bootstrap.ps1
```

Bootstrap installs/verifies the toolchain, clones vcpkg into `third_party/vcpkg` and configures CMake. On macOS
install the tools with Homebrew (see the `macos` job in `.github/workflows/cpp.yml`).

## Build

```bash
cmake --preset linux-x64-gcc               # or windows-x64, macos-arm64-clang, ...
cmake --build --preset linux-gcc-release
```

## Test

```bash
cargo test --workspace --all-features                          # Rust
cargo test -p math --no-default-features --features no_std     # math on the libm backend
ctest --preset linux-gcc-release-nogpu                         # C++ tests without a GPU
ctest --preset linux-gcc-release                               # + GPU and editor tests
```

ctest labels: `unit` (pure C++), `ffi` (needs the Rust library), `gpu` (needs a Vulkan device; runs on software
Vulkan under Xvfb in CI) and `editor` (opens a Qt window). Set `CV_REQUIRE_VALIDATION=1` to make the GPU tests fail if
the Vulkan validation layer is not running.

## Benchmarks

```bash
cargo bench -p math
```

## Format

```bash
devkit/scripts/format.sh --check    # Rust (cargo fmt) + C++/Objective-C++ (clang-format)
```
