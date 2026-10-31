# Runtime

The runtime is the part of the engine that opens a window and runs the frame loop. It is split along the language
boundary the rest of the engine uses: **Rust owns the window**, **C++ owns the game and the renderer**.

```text
 coreverse-runtime (C++)                cv-ffi (C++)                 ffi (Rust)               platform (Rust)
 ───────────────────────                ────────────                 ──────────               ───────────────
 RuntimeApp : cv_ffi::window::App ◄──── Run(app) ───────────────────► ffi_platform_run ─────► event loop (winit / headless)
   Resumed / OnEvent / AboutToWait      Context, Event (std::variant)   callback table          WindowEvent, InputEvent
   SurfaceCreated(handle) ──► renderer::NativeWindowHandle ──► Vulkan surface ──► swapchain
```

The window system never reaches C++. What reaches C++ is a table of callbacks, a `Context` to create windows with, and
events as plain data, including the native handle a graphics API needs to present into a window.

## The Rust side: `ffi_platform_run`

C++ cannot implement the Rust `App` trait, so it hands over a table of function pointers (`PlatformCallbacks`):

| Callback | Called when |
|:---------|:------------|
| `resumed(user_data, ctx)` | the application may show windows: create them here. Once at the start on the desktop, after every return from the background on Android and iOS |
| `suspended(user_data, ctx)` | the application goes to the background (mobile): every surface is lost right before |
| `window_event(user_data, ctx, window, event)` | a window event, see below |
| `about_to_wait(user_data, ctx)` | all pending events are handled and the loop is about to wait: simulate a frame, ask for a redraw |

`ffi_platform_run(callbacks, config, &exit_code)` runs the loop on the calling thread until the application calls
`ffi_platform_ctx_exit`, and returns `false` with the usual thread-local last error when the loop could not run. A null
function pointer is skipped. `config` may be null (desktop backend); `FFI_BACKEND_HEADLESS` runs without any window system,
which is what the C++ tests and `coreverse-runtime --headless` use. The headless backend returns an error instead of
waiting when nothing can wake the loop, so an application that never exits stops with `FFI_PLATFORM_ERROR_WINDOW`.

The functions a callback uses, all taking the `ctx` of the callback and returning `false` on failure:

| Function | Does |
|:---------|:-----|
| `ffi_platform_window_desc_default`, `ffi_platform_ctx_create_window` | fill a `PlatformWindowDesc` with the defaults, create a window (the id comes back as a `u64`) |
| `ffi_platform_ctx_close_window`, `_set_title`, `_set_inner_size`, `_set_fullscreen`, `_request_redraw` | change a window |
| `ffi_platform_ctx_set_cursor_visible`, `_set_cursor_grab` | the mouse cursor (`FFI_CURSOR_GRAB_NONE`/`CONFINED`/`LOCKED`) |
| `ffi_platform_ctx_set_ime_allowed`, `_set_ime_cursor_area` | input-method events for a text field |
| `ffi_platform_ctx_set_control_flow` | `FFI_CONTROL_FLOW_POLL` (a game), `WAIT` (a tool), `WAIT_UNTIL` with a timeout |
| `ffi_platform_ctx_window_info` | size in physical pixels, scale factor, focus, occlusion, fullscreen, cursor grab, IME permission |
| `ffi_platform_ctx_exit` | end the loop; the code becomes the result of `ffi_platform_run` |

### Rules the callbacks must follow

* **The context is only valid inside its callback.** A `ffi_platform_ctx_*` call made while no callback is running (after
  it returned, after the loop ended, from another thread) is refused with `FFI_PLATFORM_ERROR_INVALID_ARGUMENT` instead of
  touching freed memory. The loop lends the same object to every callback, so a pointer kept from one callback happens to
  work inside a later one; do not rely on it. Keep window ids, not the context.
* **Event data is only valid inside its callback.** The `PlatformEvent` and the strings in it (key text, IME text) are
  freed when the callback returns; copy what is needed. The native handle of `FFI_EVENT_SURFACE_CREATED` is the exception:
  what it points to lives until the matching `FFI_EVENT_SURFACE_LOST` has been handled, so a swapchain may be built from
  it and must be destroyed before that handler returns.
* **No exception may leave a callback.** Unwinding through Rust frames is undefined behavior. `cv_ffi::window::Run` catches
  it for you, stops the loop and returns the message; a plain C caller must do the same.
* **Threads.** `ffi_platform_run` runs the loop and every callback on the calling thread. macOS requires the main thread, and
  the desktop backend wants it on the other systems too. A callback must not call `ffi_platform_run` again.
* **Panics** inside Rust are caught at the boundary and reported as `FFI_PLATFORM_ERROR_PANIC`.
* The desktop backend exists on Windows, macOS and Linux. On Android and iOS `ffi_platform_run` with the desktop backend fails
  with `FFI_PLATFORM_ERROR_WINDOW` until their backends exist (a later phase); the `ffi` crate does not link `winit` there.

### Events

`PlatformEvent` is a tag (`kind`, an `FFI_EVENT_*` constant) and a union (`data`):

| `kind` | `data` member | Meaning |
|:-------|:--------------|:--------|
| `CLOSE_REQUESTED`, `DESTROYED`, `REDRAW_REQUESTED`, `SURFACE_LOST` | none | the user asked to close (nothing is closed yet); the window is gone; draw a frame; destroy everything built from the surface handle |
| `RESIZED` | `size` | client area in physical pixels |
| `MOVED` | `position` | top-left corner in physical pixels |
| `SCALE_FACTOR_CHANGED` | `scale_factor_changed` | new factor and the new client-area size |
| `FOCUSED`, `OCCLUDED` | `flag` | gained/lost focus, hidden/visible |
| `SURFACE_CREATED` | `surface` | a `PlatformNativeHandle` for the graphics API |
| `INPUT` | `input` | a `PlatformInputEvent`: its own `kind` and `data` for key, modifiers, cursor, mouse button, wheel, raw mouse motion, touch and IME |
| `UNKNOWN` (255) | none | an event this header does not know: ignore it |

The input events are those of [Input](platform.md#input). A key carries the **physical** key as an `FFI_KEY_*` number
(where it sits on the keyboard, for controls), the **logical** key (`FFI_LOGICAL_*`: an `FFI_NAMED_KEY_*` number, a
character, a dead key or unidentified), the text it produces, and the `repeat` and `synthetic` flags. The key numbers are
assigned once in `ffi/src/keys.rs` and are part of the ABI: a key added to the `platform` crate later gets the next free
number, and a key the header does not know arrives as `65535`, so the enums can grow without breaking a caller that was
built earlier.

`PlatformNativeHandle` has one group of fields per windowing system, named like the renderer's
`renderer::NativeWindowHandle` (`hwnd`/`hinstance`, `metal_layer`, `xlib_display`/`xlib_window`,
`wayland_display`/`wayland_surface`, `native_window`), and a `kind` (`FFI_DISPLAY_*`) saying which group is filled.

## The C++ side: `cv_ffi::window`

`cv-ffi` wraps all of it (`#include "cv-ffi/Window.hpp"`, never `ffi.h`): derive from `App`, override what you need and call
`Run`:

```cpp
class Game : public cv_ffi::window::App {
    void Resumed(cv_ffi::window::Context& ctx) override {
        cv_ffi::window::WindowDesc desc;
        desc.title = "Game";
        m_window = ctx.CreateWindow(desc).value();
        (void)ctx.SetControlFlow(cv_ffi::window::ControlFlow::Poll);
    }
    void OnEvent(cv_ffi::window::Context& ctx, cv_ffi::window::WindowId, const cv_ffi::window::Event& event) override {
        if (std::holds_alternative<cv_ffi::window::CloseRequested>(event)) { (void)ctx.Exit(0); }
    }
    void AboutToWait(cv_ffi::window::Context& ctx) override { (void)ctx.RequestRedraw(m_window); }
    cv_ffi::window::WindowId m_window = 0;
};

Game game;
std::expected<int, std::string> code = cv_ffi::window::Run(game);   // Backend::Headless for tests
```

`Event` is a `std::variant` (`Resized`, `SurfaceCreated`, `KeyInput`, `MouseButtonInput`, ...), `KeyCode` and `NamedKey`
are enums generated from the Rust tables (`KeyName(KeyCode)` gives "KeyW", "ArrowLeft", ...), and every `Context` function
returns `std::expected<..., std::string>` with the message of the Rust error. `Run` also returns an error when a callback
throws, and then skips the callbacks after the throwing one. `static_assert`s in `Keys.cpp` and `Window.cpp` check every
key number and every constant against `ffi.h`, so a drift between the Rust and the C++ side fails the build.

## The executable: `coreverse-runtime`

```text
coreverse-runtime [--headless] [--frames N] [--trace] [--no-renderer] [--title TEXT]
```

`RuntimeApp` (`engine/cpp/runtime/src/`) creates one window and runs a continuous loop: `AboutToWait` asks for a redraw,
`RedrawRequested` draws a frame, Escape or the close button exits, F11 toggles borderless fullscreen. When the renderer is
built (`BUILD_RENDERER=ON`, the default) the surface events drive it:

| Event | `RendererBridge` |
|:------|:-----------------|
| `SurfaceCreated(handle)` | converts the handle to `renderer::NativeWindowHandle` (it refuses a handle of the wrong kind), creates the Vulkan device on the first surface, then the surface, swapchain and frame sync |
| `Resized` | rebuilds the swapchain (a zero size keeps the old one) |
| `RedrawRequested` | acquires an image, clears it with a slowly cycling colour, presents |
| `SurfaceLost` | waits for the GPU and destroys frame sync, swapchain and surface before the handler returns; the device stays for the next surface |

The renderer's `NativeWindowHandle` now has an Android field (`nativeWindow`, an `ANativeWindow*`, with the matching
`vkCreateAndroidSurfaceKHR` branch and CMake definition), next to Win32, Xlib, Wayland and Metal. iOS uses the Metal
field like macOS: Vulkan's Metal surface takes the `CAMetalLayer` of a `UIView` as well as of an `NSView`.

CMake option `BUILD_RUNTIME` (default `ON`) builds it. With `-DBUILD_TESTS=ON` it also registers
`coreverse_runtime_headless`, which runs `coreverse-runtime --headless --frames 3` and expects exit code 0; it needs no
display and no GPU.

## What was verified where

| Check | Where it ran |
|:------|:-------------|
| `ffi` crate: callback order, exit codes, context guard, error paths, every event and input conversion, native handle kinds | Rust unit tests (`cargo test -p ffi`), headless backend |
| `cv_ffi::window`: lifecycle, window state, errors as messages, context used outside a callback, exception in a callback, a loop that never exits, key tables | C++ test `cv_ffi_window_test` (headless), `coreverse_runtime_headless` |
| Real window, X11 handle, input through Rust → ffi → C++ | by hand on Linux under Xvfb with `xdotool` (not in CI) |
| Native handle into a Vulkan swapchain and presenting frames | by hand on Linux under Xvfb with Mesa's software Vulkan (lavapipe): `coreverse-runtime --frames 200` built device, surface and swapchain from the X11 handle and presented 200 frames; without a Vulkan driver it stops with the error and exit code 2 (not in CI) |
| Windows, macOS, Android, iOS | compiled by CI only; nothing of it has been run |

## What is not here yet

* The Android and iOS backends (the next phase): until then `ffi_platform_run` cannot start a window on them, and the
  `ANativeWindow` and mobile `CAMetalLayer` paths of the renderer are compile-checked only.
* A game on top of the loop: `RuntimeApp` presents a clear colour. Scene, assets and the ECS are separate work.
* Gamepads, clipboard and dialogs are outside the plan.
