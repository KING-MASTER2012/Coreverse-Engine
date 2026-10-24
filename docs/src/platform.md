# Platform layer

The `platform` crate (`engine/rust/crates/platform`) is Coreverse's operating-system abstraction. It sits directly above
`ccore` and has no other engine dependencies. It answers the questions the rest of the engine must not answer with
`#[cfg]` blocks of its own: *what machine is this, what time is it, where do I put my files, how do I name a thread.*

Window and input handling are **not** part of the crate yet; they are a separate group of phases (see
[What is not here yet](#what-is-not-here-yet)).

## Modules

| Module    | What it gives you                                                                                                                                                         |
|:----------|:--------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `info`    | OS name and version, logical/physical cores, installed memory, `DisplayServer` (Win32, Cocoa, X11, Wayland, Android, UIKit, Headless), all in one `SystemInfo::collect()` |
| `time`    | Monotonic `Clock` and `WallClock`, `Stopwatch`, `FrameTimer` (delta time, smoothed FPS), `FixedStep` accumulator, `precise_sleep`                                         |
| `paths`   | `AppDirs`: config, data, cache, log, temp, home and executable directories as `Utf8PathBuf`                                                                               |
| `env`     | Read-only environment access (`var`, `var_path`, `var_bool`, `args`, `current_dir`)                                                                                       |
| `process` | `id`, `current_exe`, `open_url`, `reveal_in_file_manager`                                                                                                                 |
| `thread`  | OS-level thread names and abstract priorities (`Low`, `Normal`, `High`, `TimeCritical`)                                                                                   |
| `support` | `support(feature, platform)`: the table below, queryable at run time                                                                                                      |
| `error`   | `PlatformError` (`Unsupported`, `Io`, `Os`, `NonUtf8Path`), convertible into `ccore::CoreError`                                                                           |

`Platform`, `Architecture` and `Endian` are re-exported from `ccore::platform`, where the compile-time tags stay;
`PlatformExt` adds `is_desktop`, `is_mobile` and `is_unix` to `Platform`.

Design decisions worth knowing:

* **Timers are generic over a `TimeSource`.** Tests drive `Stopwatch`, `FrameTimer` and `FixedStep` with a fake clock
  and never depend on the real one.
* **No environment writes.** `std::env::set_var` is `unsafe` in edition 2024 and the workspace denies `unsafe`, so
  `env` offers no way to set a variable.
* **Nothing is created implicitly.** `AppDirs` only computes paths; `AppDirs::ensure` creates a directory with
  `std::fs` when you ask for it. A path that is not valid UTF-8 is an error, never a lossy conversion.
* **`unsafe` lives in one place.** The private `sys` module wraps the few OS calls std does not expose (`uname`,
  `sysconf`, `sysctlbyname`, kernel32/ntdll, the Windows waitable timer, `SHGetKnownFolderPath`) in safe functions
  behind a single `#[allow(unsafe_code, reason = "...")]` on that module. External dependencies are `libc` (unix) and
  `windows-sys` (Windows) only, plus `camino` and `thiserror`.

## Support matrix

`Full` means implemented and expected to work, `Partial` means implemented with documented gaps or best-effort data,
`Unsupported` means the call returns `PlatformError::Unsupported` (or nothing). The table is produced from
`platform::support::support`, which is the source of truth; a test checks it against what the functions really do on
the platform the tests run on.

| Feature | Windows | Linux | macOS | Android | iOS |
| :------ | :-----: | :---: | :---: | :-----: | :-: |
| OS version | Full | Full | Full | Partial | Full |
| Logical cores | Full | Full | Full | Full | Full |
| Physical cores | Full | Partial | Full | Partial | Full |
| Total memory | Full | Full | Full | Full | Full |
| Display server | Full | Partial | Full | Full | Full |
| Clocks | Full | Full | Full | Full | Full |
| Precise sleep | Full | Full | Full | Full | Full |
| Standard directories | Full | Full | Full | Unsupported | Unsupported |
| Environment / process id / executable path | Full | Full | Full | Full | Full |
| Open URL | Full | Partial | Full | Unsupported | Unsupported |
| Reveal in file manager | Full | Partial | Full | Unsupported | Unsupported |
| Thread name | Full | Full | Full | Partial | Partial |
| Thread priority | Full | Partial | Full | Partial | Partial |

What "Partial" means in practice:

* **Android** reports the kernel release when the version property is missing; physical cores come from sysfs, which
  some Android builds hide; thread name and priority are implemented but have never run on a device.
* **Linux** physical cores also come from sysfs (hidden in some containers). The display server is derived from
  `WAYLAND_DISPLAY`, `DISPLAY` and `XDG_SESSION_TYPE`. `open_url` and `reveal_in_file_manager` need `xdg-open`, and
  the latter only opens the folder. Raising a thread's priority needs privileges and reports `PermissionDenied`
  otherwise.
* **Precise sleep** is a sleep plus a short spin at the end. On Windows it uses a high-resolution waitable timer; on
  Windows older than 10 1803 it may overshoot by the system timer granularity but never wakes early.
* Platforms other than the five above still compile and report `Unsupported`.

## Standard directories

`<org>` is `paths::ORGANIZATION` (`Coreverse`; `coreverse` on Linux, which keeps one constant for the name) and
`<app>` the name given to `AppDirs::new`. The application name becomes a directory name, so an empty name, `.`, `..`,
path separators, control characters, any of `` :*?"<>| `` or a trailing space or dot are rejected.

| Kind   | Windows                            | macOS                                               | Linux                                                         |
|:-------|:-----------------------------------|:----------------------------------------------------|:--------------------------------------------------------------|
| config | `%APPDATA%\<org>\<app>\config`     | `~/Library/Application Support/<org>/<app>/config`  | `$XDG_CONFIG_HOME/<org>/<app>`                                |
| data   | `%LOCALAPPDATA%\<org>\<app>\data`  | `~/Library/Application Support/<org>/<app>/data`    | `$XDG_DATA_HOME/<org>/<app>`                                  |
| cache  | `%LOCALAPPDATA%\<org>\<app>\cache` | `~/Library/Caches/<org>/<app>`                      | `$XDG_CACHE_HOME/<org>/<app>`                                 |
| log    | `%LOCALAPPDATA%\<org>\<app>\log`   | `~/Library/Logs/<org>/<app>`                        | `$XDG_STATE_HOME/<org>/<app>`                                 |
| temp   | `<temp dir>\<org>\<app>`           | `<temp dir>/<org>/<app>`                            | `$XDG_RUNTIME_DIR/<org>/<app>`, else `<temp dir>/<org>/<app>` |

On Windows the roots come from `SHGetKnownFolderPath` (Roaming for config, Local for the rest). On Linux an `XDG_*`
variable is used only when it is an absolute path, as the XDG specification requires; otherwise the default under the
home directory applies (`~/.config`, `~/.local/share`, `~/.cache`, `~/.local/state`). `$XDG_RUNTIME_DIR` is preferred
for temp because it is private to the user, unlike `/tmp`.

## From C++

Two things cross the FFI boundary, through `cv_ffi::platform` (see [Rust and C++ FFI](ffi.md)):

```cpp
#include "cv-ffi/Platform.hpp"

namespace platform = cv_ffi::platform;

auto info = platform::Info();                      // std::expected<SystemInfo, std::string>
auto dir  = platform::StandardDir(platform::DirKind::Config, "editor");
if (!dir) { /* dir.error() is the Rust error message */ }
```

Clocks, timers, threads and processes are not exposed: Rust code uses them directly and C++ has its own.

## Verification

| What                                                                                                                        | Where it runs                                                                                                                                                                                                                            |
|:----------------------------------------------------------------------------------------------------------------------------|:-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `platform` unit and doc tests (`cargo test`)                                                                                | Linux, Windows and macOS (`rust.yml`)                                                                                                                                                                                                    |
| `cargo check` and Clippy for `aarch64-linux-android` and `aarch64-apple-ios`                                                | `platform-mobile` job in `rust.yml`; nothing links, so no NDK or Xcode project is needed                                                                                                                                                 |
| Windows-specific code (`windows-sys`, waitable timer, `SHGetKnownFolderPath`) and Apple-specific code (QoS, `sysctlbyname`) | Executed by the Windows and macOS runners of `rust-cross-platform` in `rust.yml` (`cargo test --workspace`) and on a developer's own machine; type-checking them from Linux is possible with `cargo check -p platform --target <triple>` |
| `ffi` platform functions                                                                                                    | Rust tests in `ffi/src/platform.rs` on every Rust CI platform; C++ side by `cv_ffi_platform_test` (`ctest -L ffi`)                                                                                                                       |

No code in the crate has been run on an Android or iOS device.

## What is not here yet

Window, input, gamepad, clipboard, dynamic-library loading and dialogs are not part of this crate. Window and input
are planned as a separate group of phases with an engine-owned API, a headless backend first and a `winit` backend for
the desktop; each phase gets its own design review before it is implemented. See `PROGRESS.md`.
