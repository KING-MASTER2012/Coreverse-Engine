# Platform layer

The `platform` crate (`engine/rust/crates/platform`) is Coreverse's operating-system abstraction. It sits directly above
`ccore` and has no other engine dependencies. It answers the questions the rest of the engine must not answer with
`#[cfg]` blocks of its own: *what machine is this, what time is it, where do I put my files, how do I name a thread.*

Windows and the application lifecycle have an API, a headless backend and a desktop backend (see [Windows and the
application lifecycle](#windows-and-the-application-lifecycle)); keyboard, mouse, touch and IME events with a per-frame query layer are described under [Input](#input); mobile backends
are a separate phase (see [What is not here yet](#what-is-not-here-yet)).

## Modules

| Module    | What it gives you                                                                                                                                                                                                        |
|:----------|:-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `info`    | OS name and version, logical/physical cores, installed memory, `DisplayServer` (Win32, Cocoa, X11, Wayland, Android, UIKit, Headless), all in one `SystemInfo::collect()`                                                |
| `time`    | Monotonic `Clock` and `WallClock`, `Stopwatch`, `FrameTimer` (delta time, smoothed FPS), `FixedStep` accumulator, `precise_sleep`                                                                                        |
| `paths`   | `AppDirs`: config, data, cache, log, temp, home and executable directories as `Utf8PathBuf`                                                                                                                              |
| `env`     | Read-only environment access (`var`, `var_path`, `var_bool`, `args`, `current_dir`)                                                                                                                                      |
| `process` | `id`, `current_exe`, `open_url`, `reveal_in_file_manager`                                                                                                                                                                |
| `thread`  | OS-level thread names and abstract priorities (`Low`, `Normal`, `High`, `TimeCritical`)                                                                                                                                  |
| `window`  | The callback-based `App` lifecycle, `WindowDesc`, `WindowEvent`, `NativeWindowHandle`, monitors, cursor, a headless backend and (feature `winit`) a desktop backend; see [below](#windows-and-the-application-lifecycle) |
| `input`   | `InputEvent` (keyboard with physical `KeyCode` and logical `Key`, mouse, touch, IME) and `InputState`, the per-frame "held / just pressed / just released" queries; see [Input](#input) |
| `support` | `support(feature, platform)`: the table below, queryable at run time                                                                                                                                                     |
| `error`   | `PlatformError` (`Unsupported`, `Io`, `Os`, `NonUtf8Path`), convertible into `ccore::CoreError`                                                                                                                          |

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

| Feature                                    | Windows  |  Linux  | macOS  |   Android   |     iOS     |
|:-------------------------------------------|:--------:|:-------:|:------:|:-----------:|:-----------:|
| OS version                                 |   Full   |  Full   |  Full  |   Partial   |    Full     |
| Logical cores                              |   Full   |  Full   |  Full  |    Full     |    Full     |
| Physical cores                             |   Full   | Partial |  Full  |   Partial   |    Full     |
| Total memory                               |   Full   |  Full   |  Full  |    Full     |    Full     |
| Display server                             |   Full   | Partial |  Full  |    Full     |    Full     |
| Clocks                                     |   Full   |  Full   |  Full  |    Full     |    Full     |
| Precise sleep                              |   Full   |  Full   |  Full  |    Full     |    Full     |
| Standard directories                       |   Full   |  Full   |  Full  | Unsupported | Unsupported |
| Environment / process id / executable path |   Full   |  Full   |  Full  |    Full     |    Full     |
| Open URL                                   |   Full   | Partial |  Full  | Unsupported | Unsupported |
| Reveal in file manager                     |   Full   | Partial |  Full  | Unsupported | Unsupported |
| Thread name                                |   Full   |  Full   |  Full  |   Partial   |   Partial   |
| Thread priority                            |   Full   | Partial |  Full  |   Partial   |   Partial   |

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

## Windows and the application lifecycle

`platform::window` is an engine-owned window API. A backend runs your `App`, creates the real windows and calls you
back; code written against `App` and `AppContext` does not change between backends.

```rust
use platform::window::{App, AppContext, WindowDesc, WindowEvent, WindowId};

struct Game;

impl App for Game {
    fn resumed(&mut self, ctx: &mut AppContext) {
        ctx.create_window(WindowDesc::new("Game")).unwrap();   // windows are created here
    }
    fn window_event(&mut self, ctx: &mut AppContext, window: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => ctx.exit(),
            WindowEvent::SurfaceCreated(handle) => { /* create the swapchain from `handle` */ }
            WindowEvent::SurfaceLost => { /* destroy everything made from that handle */ }
            WindowEvent::RedrawRequested => { /* draw a frame */ }
            _ => {}
        }
    }
    fn about_to_wait(&mut self, ctx: &mut AppContext) { /* per-frame work, then request_redraw */ }
}
```

**Order of callbacks** (the contract every backend keeps):

| Moment                                 | What is delivered, in this order                                                                                              |
|:---------------------------------------|:------------------------------------------------------------------------------------------------------------------------------|
| Loop start, and after every suspension | `resumed`                                                                                                                     |
| Each loop iteration                    | the pending events, then `about_to_wait`, then `RedrawRequested` for the windows that asked (not while suspended)             |
| App goes to the background             | `SurfaceLost` for every window that has a surface, then `suspended`                                                           |
| System replaces a window's surface     | `SurfaceLost`, then `SurfaceCreated` with the new handle                                                                      |
| `close_window`                         | `SurfaceLost` (if it has a surface), `Destroyed`, then the window is gone (it is still readable while `Destroyed` is handled) |
| `exit`                                 | the callback finishes; every remaining window gets `SurfaceLost` (if any) and `Destroyed`; the loop returns the exit code     |

Every `SurfaceCreated` is eventually followed by exactly one `SurfaceLost`, so releasing swapchain resources in that
handler is always enough. `WindowEvent` is `#[non_exhaustive]`, so keep a catch-all arm.

**Units and requests.** `WindowDesc` and the setters use logical (DPI-independent) units; what windows report (`Resized`,
`Window::inner_size`) is in physical pixels, the size a swapchain needs. Title and visibility read back immediately; size,
position and fullscreen are requests that come back as `Resized`/`Moved` events, because the window system may adjust
them.

**`NativeWindowHandle`** has one variant per windowing system, mirroring `DisplayServer`:

| Variant   | Carries                       | Used for        |
|:----------|:------------------------------|:----------------|
| `Win32`   | `HWND`, optional `HINSTANCE`  | Windows         |
| `Cocoa`   | `CAMetalLayer*`               | macOS           |
| `X11`     | `Window`, optional `Display*` | Linux (Xlib)    |
| `Wayland` | `wl_display*`, `wl_surface*`  | Linux (Wayland) |
| `Android` | `ANativeWindow*`              | Android         |
| `UiKit`   | `CAMetalLayer*`               | iOS             |

The fields are the ones the renderer's C++ `renderer::NativeWindowHandle` already takes (plus the two mobile ones). Pointers
are stored as non-zero integers, so the type is `Copy + Send + Sync` and the crate stays free of `unsafe`.

**Backends.**

| Backend                                                                                | Status                                                    | Needs     |
|:---------------------------------------------------------------------------------------|:----------------------------------------------------------|:----------|
| Headless (`HeadlessBackend`, `HeadlessLoop`)                                           | Implemented; tested on every CI platform                  | nothing   |
| Desktop (`WinitBackend`, cargo feature `winit`; Windows, macOS, Linux X11 and Wayland) | Implemented; smoke-tested on Linux (X11) under Xvfb in CI | a display |
| Android, iOS                                                                           | Planned; compile-checked only                             | a device  |

The headless backend simulates an operating system: a test queues `HostEvent`s (a resize, a close click, `Suspend`, a new
Android surface) and the loop turns them into the same callbacks, in the same order, as a real backend; requests such as
a resize come back as events like a window manager would send them. `HeadlessLoop` runs the loop step by step and hands
the app back for inspection; `HeadlessBackend` implements `Backend::run`. It has no clock, so `ControlFlow::WaitUntil`
behaves like `Wait`, and a loop that is waiting with nothing queued is idle (`Backend::run` reports it as
`WindowError::Stalled` instead of blocking forever).

### Desktop backend (`winit` feature)

`platform::window::WinitBackend` opens real windows through [`winit`](https://crates.io/crates/winit) 0.30. It is behind the
`winit` cargo feature (off by default, so code that only needs the platform services or the headless backend does not build
it): `platform = { path = "...", features = ["winit"] }`. An `App` written against the headless backend runs on it
unchanged.

```rust
let exit_code = WinitBackend::new().run(Game)?;   // on macOS this must be the main thread
```

What it does on a real desktop, beyond the contract above:

* **Surfaces.** `SurfaceCreated` is delivered right after the window is created, with the native handle of the session's
  window system (`Win32`, `X11`, `Wayland`, `Cocoa`); `SurfaceLost`/`Destroyed` follow `close_window` and `exit` as in the
  headless backend. On macOS the handle carries a `CAMetalLayer` that `raw-window-metal` adds to the window's view.
* **DPI.** New windows start at the primary monitor's scale factor. When a window moves to a monitor with another scale
  factor, `ScaleFactorChanged` reports the new physical size, which keeps the logical size. `Resized` is only delivered when
  the size really changed.
* **Monitors.** `AppContext::monitors()` and `primary_monitor()` list name, position, size, scale factor and refresh rate.
  The list is read when the app resumes, when a window is created and when a scale factor changes. Wayland has no primary
  monitor; the first one is reported as primary.
* **Cursor.** `set_cursor_icon`, `set_cursor_visible` and `set_cursor_grab` (`None`, `Confined`, `Locked`). Icon and
  visibility read back at once; a grab the platform refuses leaves the previous mode, and `Window::cursor_grab()` always
  reports what is in effect. Which mode works depends on the platform (Windows and X11 confine, macOS locks).
* **Fullscreen.** `Fullscreen::Borderless` on the window's monitor. The new size arrives as `Resized`. Window managers keep
  a fullscreen window within its maximum size, so a window with a small `max_inner_size` cannot fill a larger monitor.
* **Failures.** A window that cannot be created is fatal: the loop exits and `Backend::run` returns
  `WindowError::Backend`. So does an event loop that cannot start (no display).

Not covered yet: exclusive fullscreen, choosing the monitor for fullscreen, window icons, and anything
Android or iOS specific.

## Input

Keyboard, mouse, touch and input-method (IME) events arrive as `WindowEvent::Input(InputEvent)`, the same on every
backend. `platform::input` also has `InputState`, which turns the event stream into "is it held, did it just go down" for
game loops.

| Events (`InputEvent`) | What it carries |
|:----------------------|:----------------|
| `Key(KeyEvent)` | the **physical** key (`KeyCode`, where it sits on the keyboard: `KeyW` is the same key on QWERTY, AZERTY and Dvorak, use it for controls), the **logical** key (`Key`: a character such as `"A"` or a `NamedKey` such as Enter, use it for shortcuts), the text the press produces, `repeat` for auto-repeat, `synthetic` for events the system made up |
| `ModifiersChanged` | `Modifiers`: Shift, Control, Alt, Super (left and right are not told apart) |
| `CursorMoved`, `CursorEntered`, `CursorLeft` | positions in physical pixels (`PointerPosition`, fractional) from the client area's top-left corner |
| `MouseButton`, `MouseWheel` | `MouseButton` (Left, Right, Middle, Back, Forward, Other) with `ButtonState`; `ScrollDelta` in lines (wheels) or pixels (touchpads) |
| `RawMouseMotion` | unaccelerated relative movement for camera control; keeps coming while the cursor is `CursorGrab::Locked`; goes to the window that has focus |
| `Touch(TouchEvent)` | finger id, `TouchPhase` (Started, Moved, Ended, Cancelled) and position |
| `Ime(ImeEvent)` | `Enabled`, `Preedit` (text being composed), `Commit`, `Disabled` |

`InputEvent`, `KeyCode`, `NamedKey`, `Key` and the other enums are `#[non_exhaustive]`; keep a catch-all arm.

**IME.** An input method only talks to a window that asked for it: call `AppContext::set_ime_allowed(window, true)` while a
text field has focus and `set_ime_cursor_area` to tell the system where the caret is (so its candidate window does not
cover the text). `Window::is_ime_allowed` reads back at once. Text typed without composition arrives as `KeyEvent::text`,
text composed by an IME as `Commit`. The headless backend drops IME events while the permission is off, as a real system
would.

**Querying state.** Feed every event to `InputState::handle_event`, ask during the update, call `end_frame` once per frame:

```rust
fn window_event(&mut self, _: &mut AppContext, _: WindowId, event: WindowEvent) {
    self.input.handle_event(&event);
}

fn about_to_wait(&mut self, _: &mut AppContext) {
    if self.input.key_just_pressed(KeyCode::Space) { self.jump(); }
    let (dx, dy) = self.input.raw_mouse_delta();
    self.camera.turn(dx, dy);
    self.input.end_frame();
}
```

| Query | Meaning |
|:------|:--------|
| `key_pressed`, `mouse_pressed`, `logical_key_pressed` | held right now |
| `*_just_pressed`, `*_just_released` | the change happened this frame; a press and release within one frame reports both and is not held afterwards, so a quick tap is not lost |
| `cursor_position`, `cursor_delta` | position (`None` while the cursor is outside the window) and movement of this frame; a position from before the cursor left never produces a jump |
| `raw_mouse_delta`, `scroll_lines`, `scroll_pixels` | sums over the frame |
| `touches`, `touches_started`, `touches_ended` | fingers on the screen and those that started or ended this frame (a cancelled touch counts as ended) |
| `text`, `ime_preedit` | printable text typed this frame (control characters left out), and the text an IME is composing |

Auto-repeat never counts as a new press. When the window loses focus everything that is held is released (and reported as
just released), so no key stays stuck after Alt-Tab. One `InputState` follows one window; with several windows keep one
per `WindowId`.

**Where it was verified.** The event types, `InputState` and the headless backend (`HostEvent::Input`) by unit and
doc tests on every Rust CI platform. The `winit` conversions (every key, button, scroll and IME variant) by unit tests, and
the X11 path by hand on Linux under Xvfb with `xdotool`: mouse movement, buttons and wheel, `W`, Shift+H, `i`, Ctrl+A with
the right physical and logical keys, text and modifiers all arrived and were seen by `InputState`. The CI smoke test only
checks that the IME permission and caret area are accepted; it does not inject input. Not run anywhere: raw mouse motion
(Xvfb does not produce it), touch, real IME composition, Windows, macOS and Wayland input.

## Standard directories

`<org>` is `paths::ORGANIZATION` (`Coreverse`; `coreverse` on Linux, which keeps one constant for the name) and
`<app>` the name given to `AppDirs::new`. The application name becomes a directory name, so an empty name, `.`, `..`,
path separators, control characters, any of `` :*?"<>| `` or a trailing space or dot are rejected.

| Kind   | Windows                            | macOS                                              | Linux                                                         |
|:-------|:-----------------------------------|:---------------------------------------------------|:--------------------------------------------------------------|
| config | `%APPDATA%\<org>\<app>\config`     | `~/Library/Application Support/<org>/<app>/config` | `$XDG_CONFIG_HOME/<org>/<app>`                                |
| data   | `%LOCALAPPDATA%\<org>\<app>\data`  | `~/Library/Application Support/<org>/<app>/data`   | `$XDG_DATA_HOME/<org>/<app>`                                  |
| cache  | `%LOCALAPPDATA%\<org>\<app>\cache` | `~/Library/Caches/<org>/<app>`                     | `$XDG_CACHE_HOME/<org>/<app>`                                 |
| log    | `%LOCALAPPDATA%\<org>\<app>\log`   | `~/Library/Logs/<org>/<app>`                       | `$XDG_STATE_HOME/<org>/<app>`                                 |
| temp   | `<temp dir>\<org>\<app>`           | `<temp dir>/<org>/<app>`                           | `$XDG_RUNTIME_DIR/<org>/<app>`, else `<temp dir>/<org>/<app>` |

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

| What                                                                                                                        | Where it runs                                                                                                                                                                                                                                                                                  |
|:----------------------------------------------------------------------------------------------------------------------------|:-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `platform` unit and doc tests (`cargo test`)                                                                                | Linux, Windows and macOS (`rust.yml`)                                                                                                                                                                                                                                                          |
| `cargo check` and Clippy for `aarch64-linux-android` and `aarch64-apple-ios`                                                | `platform-mobile` job in `rust.yml`; nothing links, so no NDK or Xcode project is needed                                                                                                                                                                                                       |
| Windows-specific code (`windows-sys`, waitable timer, `SHGetKnownFolderPath`) and Apple-specific code (QoS, `sysctlbyname`) | Executed by the Windows and macOS runners of `rust-cross-platform` in `rust.yml` (`cargo test --workspace`) and on a developer's own machine; type-checking them from Linux is possible with `cargo check -p platform --target <triple>`                                                       |
| `ffi` platform functions                                                                                                    | Rust tests in `ffi/src/platform.rs` on every Rust CI platform; C++ side by `cv_ffi_platform_test` (`ctest -L ffi`)                                                                                                                                                                             |
| `window` API and headless backend                                                                                           | Part of the `platform` unit and doc tests, so they run on every Rust CI platform; no display is needed                                                                                                                                                                                         |
| Desktop backend on Linux (X11)                                                                                              | `platform-window-xvfb` job in `rust.yml`: a real window on Xvfb, once with openbox as window manager (fullscreen included) and once without (`cargo test -p platform --features winit --test winit_smoke`). The test skips itself when there is no display, unless `CV_REQUIRE_DISPLAY` is set |
| Input events and `InputState` | Unit and doc tests (all Rust CI platforms); `winit` conversions by unit tests; real X11 input by hand under Xvfb with `xdotool` (not in CI); the smoke test checks the IME permission only |
| Desktop backend on Windows and macOS, Wayland                                                                               | Compiled (and linted) by the `rust-cross-platform` jobs, which build with `--all-features`; **no CI job opens a window there**, so run `WinitBackend` on a real machine to verify                                                                                                              |

No code in the crate has been run on an Android or iOS device.

## What is not here yet

The Android and iOS backends are not part of the crate yet (the code compiles for both, nothing runs on a device). Touch
and IME on them come with those backends. The FFI event loop for the C++ runtime and the renderer integration come after that. Gamepad, clipboard, dynamic-library loading and dialogs are not planned. Each phase gets its
own design review before it is implemented; see `PROGRESS.md`.
