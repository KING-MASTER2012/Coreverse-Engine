//! The window event loop for C++: [`ffi_platform_run`] and the callback table.
//!
//! C++ cannot implement the Rust `App` trait, so it hands over a table of
//! function pointers instead ([`PlatformCallbacks`]) and `ffi_platform_run`
//! runs the event loop of the `platform` crate on the calling thread until the
//! application exits. Every callback gets a [`PlatformContext`] to create
//! windows, ask for redraws, change the cursor and exit with; window events
//! arrive as a [`PlatformEvent`].
//!
//! ```text
//! C++ main()                                   Rust (platform crate)
//!   ffi_platform_run(&callbacks, &config, &code) ──► event loop (winit, or headless)
//!                      ◄── resumed(user_data, ctx)
//!   ffi_platform_ctx_create_window(ctx, ...) ──►     window created
//!                      ◄── window_event(user_data, ctx, id, &event)   (SurfaceCreated ...)
//!                      ◄── about_to_wait(user_data, ctx)
//!   ffi_platform_ctx_exit(ctx, 0) ──►                loop ends
//!   ffi_platform_run returns true, code = 0
//! ```
//!
//! ## Threads and re-entrancy
//! `ffi_platform_run` runs the loop on the calling thread and calls every
//! callback on that same thread. macOS requires it to be the main thread; the
//! other desktop systems accept only the main thread here too. A callback must
//! not call `ffi_platform_run` again.
//!
//! ## The context is only valid inside its callback
//! The `ctx` pointer of a callback is valid until that callback returns. A
//! `ffi_platform_ctx_*` call made while no callback is running (after it
//! returned, after the loop ended, or from another thread) is refused with
//! [`FFI_PLATFORM_ERROR_INVALID_ARGUMENT`](crate::platform::FFI_PLATFORM_ERROR_INVALID_ARGUMENT)
//! instead of touching freed memory. The loop lends the same context object to
//! every callback, so a pointer kept from one callback happens to work inside a
//! later one; do not rely on it. Keep window ids (plain `u64`s), not the
//! context.
//!
//! ## Event data is only valid inside its callback
//! The [`PlatformEvent`] pointer and every string inside it (key text, IME
//! text) are valid until the callback returns; copy what you need. The native
//! handle of [`FFI_EVENT_SURFACE_CREATED`] is different: the objects it points
//! to live until the matching [`FFI_EVENT_SURFACE_LOST`] has been handled, so
//! a swapchain may be built from it and must be destroyed before that handler
//! returns.
//!
//! ## No unwinding through the loop
//! A C++ callback must not let an exception escape: unwinding through Rust
//! frames is undefined behavior. Catch it in the callback and call
//! [`ffi_platform_ctx_exit`] with an error code instead. A Rust panic is
//! caught here and reported as [`FFI_PLATFORM_ERROR_PANIC`](crate::platform::FFI_PLATFORM_ERROR_PANIC).
//!
//! ## Errors
//! Every function returns `false` on failure and fills the thread-local last
//! error of the [`platform`](crate::platform) module
//! (`ffi_platform_last_error_code` / `ffi_platform_last_error_message`).
//!
//! ## Backends
//! [`FFI_BACKEND_DESKTOP`] opens real windows (Windows, macOS, Linux; on
//! Android and iOS it fails until their backends exist).
//! [`FFI_BACKEND_HEADLESS`] needs no display: windows exist only as state, no
//! surface is ever created and no input arrives. It returns an error instead
//! of waiting when nothing can wake the loop, so an application that does not
//! call exit stops with one; it is meant for tests and servers.

use std::cell::Cell;
use std::ffi::{CStr, CString, c_char, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::{Duration, Instant};

use platform_crate::input::{
    ButtonState, ImeEvent, InputEvent, Key, KeyEvent, Modifiers, MouseButton, ScrollDelta,
    TouchPhase,
};
use platform_crate::window::{
    App, AppContext, Backend, ControlFlow, CursorGrab, Fullscreen, HeadlessBackend, HeadlessConfig,
    LogicalPosition, LogicalSize, NativeWindowHandle, PhysicalSize, WindowDesc, WindowError,
    WindowEvent, WindowId,
};

use crate::keys;
use crate::platform::{Failure, clear_last_error, set_last_error};

/// [`PlatformRunConfig::backend`]: real windows on the desktop.
pub const FFI_BACKEND_DESKTOP: u8 = 0;
/// [`PlatformRunConfig::backend`]: no display, for tests and servers.
pub const FFI_BACKEND_HEADLESS: u8 = 1;

/// [`PlatformEvent::kind`]: the user asked the window to close. Nothing is
/// closed yet; call [`ffi_platform_ctx_close_window`] or ignore it.
pub const FFI_EVENT_CLOSE_REQUESTED: u8 = 0;
/// [`PlatformEvent::kind`]: the window has been closed.
pub const FFI_EVENT_DESTROYED: u8 = 1;
/// [`PlatformEvent::kind`]: the client area changed size; `data.size`.
pub const FFI_EVENT_RESIZED: u8 = 2;
/// [`PlatformEvent::kind`]: the window moved; `data.position`.
pub const FFI_EVENT_MOVED: u8 = 3;
/// [`PlatformEvent::kind`]: the scale factor changed; `data.scale_factor_changed`.
pub const FFI_EVENT_SCALE_FACTOR_CHANGED: u8 = 4;
/// [`PlatformEvent::kind`]: keyboard focus gained or lost; `data.flag` is
/// `true` when gained.
pub const FFI_EVENT_FOCUSED: u8 = 5;
/// [`PlatformEvent::kind`]: the window became fully hidden (`data.flag` is
/// `true`) or visible again.
pub const FFI_EVENT_OCCLUDED: u8 = 6;
/// [`PlatformEvent::kind`]: draw a frame now (after
/// [`ffi_platform_ctx_request_redraw`]).
pub const FFI_EVENT_REDRAW_REQUESTED: u8 = 7;
/// [`PlatformEvent::kind`]: the window has a presentation surface; build the
/// swapchain from `data.surface`.
pub const FFI_EVENT_SURFACE_CREATED: u8 = 8;
/// [`PlatformEvent::kind`]: the surface is gone; destroy everything built from
/// the last `FFI_EVENT_SURFACE_CREATED` handle before returning.
pub const FFI_EVENT_SURFACE_LOST: u8 = 9;
/// [`PlatformEvent::kind`]: keyboard, mouse, touch or IME input; `data.input`.
pub const FFI_EVENT_INPUT: u8 = 10;
/// [`PlatformEvent::kind`]: an event this header does not know. Ignore it.
pub const FFI_EVENT_UNKNOWN: u8 = 255;

/// [`PlatformInputEvent::kind`]: a key went down or up; `data.key`.
pub const FFI_INPUT_KEY: u8 = 0;
/// [`PlatformInputEvent::kind`]: the held modifiers changed; `data.modifiers`
/// is a mask of the `FFI_MOD_*` bits.
pub const FFI_INPUT_MODIFIERS_CHANGED: u8 = 1;
/// [`PlatformInputEvent::kind`]: the cursor moved; `data.point` in physical pixels.
pub const FFI_INPUT_CURSOR_MOVED: u8 = 2;
/// [`PlatformInputEvent::kind`]: the cursor entered the window.
pub const FFI_INPUT_CURSOR_ENTERED: u8 = 3;
/// [`PlatformInputEvent::kind`]: the cursor left the window.
pub const FFI_INPUT_CURSOR_LEFT: u8 = 4;
/// [`PlatformInputEvent::kind`]: a mouse button went down or up; `data.mouse_button`.
pub const FFI_INPUT_MOUSE_BUTTON: u8 = 5;
/// [`PlatformInputEvent::kind`]: the wheel or touchpad scrolled; `data.scroll`.
pub const FFI_INPUT_MOUSE_WHEEL: u8 = 6;
/// [`PlatformInputEvent::kind`]: raw mouse movement for camera control;
/// `data.point` holds the delta.
pub const FFI_INPUT_RAW_MOUSE_MOTION: u8 = 7;
/// [`PlatformInputEvent::kind`]: a touch changed; `data.touch`.
pub const FFI_INPUT_TOUCH: u8 = 8;
/// [`PlatformInputEvent::kind`]: an input method event; `data.ime`.
pub const FFI_INPUT_IME: u8 = 9;
/// [`PlatformInputEvent::kind`]: an input event this header does not know.
pub const FFI_INPUT_UNKNOWN: u8 = 255;

/// [`PlatformKeyEvent::logical_kind`]: a named key; `logical_named` is an
/// `FFI_NAMED_KEY_*` number.
pub const FFI_LOGICAL_NAMED: u8 = 0;
/// [`PlatformKeyEvent::logical_kind`]: a key that types text; `logical_text`.
pub const FFI_LOGICAL_CHARACTER: u8 = 1;
/// [`PlatformKeyEvent::logical_kind`]: a dead key; `logical_text` is its accent
/// or null.
pub const FFI_LOGICAL_DEAD: u8 = 2;
/// [`PlatformKeyEvent::logical_kind`]: the platform could not tell.
pub const FFI_LOGICAL_UNIDENTIFIED: u8 = 3;

/// `state`: the key or button went down.
pub const FFI_BUTTON_PRESSED: u8 = 0;
/// `state`: the key or button went up.
pub const FFI_BUTTON_RELEASED: u8 = 1;

/// [`PlatformInputData::modifiers`] bit: Shift.
pub const FFI_MOD_SHIFT: u8 = 1;
/// [`PlatformInputData::modifiers`] bit: Control.
pub const FFI_MOD_CONTROL: u8 = 2;
/// [`PlatformInputData::modifiers`] bit: Alt (Option on macOS).
pub const FFI_MOD_ALT: u8 = 4;
/// [`PlatformInputData::modifiers`] bit: the Windows key, or Command on macOS.
pub const FFI_MOD_SUPER: u8 = 8;

/// [`PlatformMouseButtonEvent::button`]: the primary button.
pub const FFI_MOUSE_LEFT: u8 = 0;
/// [`PlatformMouseButtonEvent::button`]: the secondary button.
pub const FFI_MOUSE_RIGHT: u8 = 1;
/// [`PlatformMouseButtonEvent::button`]: the wheel button.
pub const FFI_MOUSE_MIDDLE: u8 = 2;
/// [`PlatformMouseButtonEvent::button`]: the "back" side button.
pub const FFI_MOUSE_BACK: u8 = 3;
/// [`PlatformMouseButtonEvent::button`]: the "forward" side button.
pub const FFI_MOUSE_FORWARD: u8 = 4;
/// [`PlatformMouseButtonEvent::button`]: another button; `other` is its number.
pub const FFI_MOUSE_OTHER: u8 = 5;

/// [`PlatformScrollEvent::unit`]: the amounts are lines (notched wheels).
pub const FFI_SCROLL_LINES: u8 = 0;
/// [`PlatformScrollEvent::unit`]: the amounts are pixels (touchpads).
pub const FFI_SCROLL_PIXELS: u8 = 1;

/// [`PlatformTouchEvent::phase`]: a finger touched the screen.
pub const FFI_TOUCH_STARTED: u8 = 0;
/// [`PlatformTouchEvent::phase`]: the finger moved.
pub const FFI_TOUCH_MOVED: u8 = 1;
/// [`PlatformTouchEvent::phase`]: the finger was lifted.
pub const FFI_TOUCH_ENDED: u8 = 2;
/// [`PlatformTouchEvent::phase`]: the system took the touch away.
pub const FFI_TOUCH_CANCELLED: u8 = 3;

/// [`PlatformImeEvent::kind`]: composition became available.
pub const FFI_IME_ENABLED: u8 = 0;
/// [`PlatformImeEvent::kind`]: the text being composed changed; an empty
/// `text` clears it.
pub const FFI_IME_PREEDIT: u8 = 1;
/// [`PlatformImeEvent::kind`]: composition finished; insert `text`.
pub const FFI_IME_COMMIT: u8 = 2;
/// [`PlatformImeEvent::kind`]: composition is no longer available.
pub const FFI_IME_DISABLED: u8 = 3;
/// [`PlatformImeEvent::kind`]: an IME event this header does not know.
pub const FFI_IME_UNKNOWN: u8 = 255;

/// [`PlatformWindowDesc::fullscreen`]: a normal window.
pub const FFI_FULLSCREEN_NONE: u8 = 0;
/// [`PlatformWindowDesc::fullscreen`]: borderless, the size of the monitor.
pub const FFI_FULLSCREEN_BORDERLESS: u8 = 1;

/// [`ffi_platform_ctx_set_cursor_grab`]: the cursor moves freely.
pub const FFI_CURSOR_GRAB_NONE: u8 = 0;
/// [`ffi_platform_ctx_set_cursor_grab`]: the cursor cannot leave the window.
pub const FFI_CURSOR_GRAB_CONFINED: u8 = 1;
/// [`ffi_platform_ctx_set_cursor_grab`]: the cursor is held in place; use the
/// raw mouse motion input.
pub const FFI_CURSOR_GRAB_LOCKED: u8 = 2;

/// [`ffi_platform_ctx_set_control_flow`]: never wait, start the next iteration
/// at once (a game).
pub const FFI_CONTROL_FLOW_POLL: u8 = 0;
/// [`ffi_platform_ctx_set_control_flow`]: sleep until the next event (a tool).
pub const FFI_CONTROL_FLOW_WAIT: u8 = 1;
/// [`ffi_platform_ctx_set_control_flow`]: sleep until the next event or until
/// `wait_millis` milliseconds have passed.
pub const FFI_CONTROL_FLOW_WAIT_UNTIL: u8 = 2;

/// The application's view of the event loop, only valid inside the callback it
/// was passed to (see the module docs). Opaque: pass it to the
/// `ffi_platform_ctx_*` functions. (Not `repr(C)` on purpose, so that cbindgen
/// declares it as an incomplete C type.)
pub struct PlatformContext {
    _private: (),
}

/// A size in physical pixels.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PlatformSize {
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
}

/// A position in physical pixels.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PlatformPosition {
    /// Horizontal offset.
    pub x: i32,
    /// Vertical offset, growing downwards.
    pub y: i32,
}

/// A point or delta with fractional coordinates.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PlatformPoint {
    /// Horizontal value.
    pub x: f64,
    /// Vertical value.
    pub y: f64,
}

/// Payload of [`FFI_EVENT_SCALE_FACTOR_CHANGED`].
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PlatformScaleFactorChange {
    /// The new scale factor.
    pub scale_factor: f64,
    /// The new client-area size in physical pixels.
    pub inner_size: PlatformSize,
}

/// What a graphics API needs to create a presentation surface, one group of
/// fields per windowing system; the others are null or `0`. The names and types
/// are those of the renderer's C++ `renderer::NativeWindowHandle`, so a handle
/// converts to it field by field.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PlatformNativeHandle {
    /// An `FFI_DISPLAY_*` constant: which group of fields is filled.
    /// [`FFI_DISPLAY_UNKNOWN`](crate::platform::FFI_DISPLAY_UNKNOWN) for a
    /// handle this header does not know.
    pub kind: u8,
    /// Windows: the `HWND`.
    pub hwnd: *mut c_void,
    /// Windows: the `HINSTANCE`, null when unknown (the renderer then uses the
    /// current module).
    pub hinstance: *mut c_void,
    /// macOS and iOS: the `CAMetalLayer*`.
    pub metal_layer: *mut c_void,
    /// X11: the Xlib `Display*`, null when unknown.
    pub xlib_display: *mut c_void,
    /// X11: the Xlib `Window`.
    pub xlib_window: u64,
    /// Wayland: the `wl_display*`.
    pub wayland_display: *mut c_void,
    /// Wayland: the `wl_surface*`.
    pub wayland_surface: *mut c_void,
    /// Android: the `ANativeWindow*`.
    pub native_window: *mut c_void,
}

/// Payload of [`FFI_INPUT_KEY`]. The strings are valid inside the callback only.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PlatformKeyEvent {
    /// The physical key, an `FFI_KEY_*` number: where it is on the keyboard,
    /// independent of the layout. Use it for controls.
    pub physical_key: u16,
    /// What the key means under the layout: an `FFI_LOGICAL_*` constant.
    pub logical_kind: u8,
    /// For [`FFI_LOGICAL_NAMED`]: an `FFI_NAMED_KEY_*` number.
    pub logical_named: u16,
    /// For [`FFI_LOGICAL_CHARACTER`] and [`FFI_LOGICAL_DEAD`]: the text, UTF-8;
    /// otherwise null.
    pub logical_text: *const c_char,
    /// The text the press produces (UTF-8), null when none. Control characters
    /// for Enter, Tab and Backspace are included.
    pub text: *const c_char,
    /// `FFI_BUTTON_PRESSED` or `FFI_BUTTON_RELEASED`.
    pub state: u8,
    /// `true` for the auto-repeat of a held key.
    pub repeat: bool,
    /// `true` when the system made the event up (the release of a key that was
    /// held while the window lost focus).
    pub synthetic: bool,
}

/// Payload of [`FFI_INPUT_MOUSE_BUTTON`].
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PlatformMouseButtonEvent {
    /// An `FFI_MOUSE_*` constant.
    pub button: u8,
    /// The button number when `button` is [`FFI_MOUSE_OTHER`], else `0`.
    pub other: u16,
    /// `FFI_BUTTON_PRESSED` or `FFI_BUTTON_RELEASED`.
    pub state: u8,
}

/// Payload of [`FFI_INPUT_MOUSE_WHEEL`]. Positive values mean the scrolled
/// content moves right and down: rolling a wheel away from you gives a positive
/// `y`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PlatformScrollEvent {
    /// `FFI_SCROLL_LINES` or `FFI_SCROLL_PIXELS`.
    pub unit: u8,
    /// Horizontal amount.
    pub x: f64,
    /// Vertical amount.
    pub y: f64,
}

/// Payload of [`FFI_INPUT_TOUCH`].
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PlatformTouchEvent {
    /// Identifies the finger from its start to its end.
    pub id: u64,
    /// An `FFI_TOUCH_*` constant.
    pub phase: u8,
    /// Where, in physical pixels.
    pub x: f64,
    /// Where, in physical pixels.
    pub y: f64,
}

/// Payload of [`FFI_INPUT_IME`]. The text is valid inside the callback only.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PlatformImeEvent {
    /// An `FFI_IME_*` constant.
    pub kind: u8,
    /// The composed or committed text (UTF-8); null for enabled and disabled.
    pub text: *const c_char,
    /// `true` when the input method shows a caret or selection inside the
    /// preedit text.
    pub has_cursor: bool,
    /// Start of the caret or selection, in bytes of `text`.
    pub cursor_start: u64,
    /// End of the caret or selection, in bytes of `text`.
    pub cursor_end: u64,
}

/// The payload of a [`PlatformInputEvent`]; which member is valid depends on
/// `kind`.
#[repr(C)]
#[derive(Clone, Copy)]
pub union PlatformInputData {
    /// Nothing (cursor entered and left, unknown events).
    pub empty: u8,
    /// [`FFI_INPUT_KEY`].
    pub key: PlatformKeyEvent,
    /// [`FFI_INPUT_MODIFIERS_CHANGED`]: a mask of `FFI_MOD_*` bits.
    pub modifiers: u8,
    /// [`FFI_INPUT_CURSOR_MOVED`] and [`FFI_INPUT_RAW_MOUSE_MOTION`].
    pub point: PlatformPoint,
    /// [`FFI_INPUT_MOUSE_BUTTON`].
    pub mouse_button: PlatformMouseButtonEvent,
    /// [`FFI_INPUT_MOUSE_WHEEL`].
    pub scroll: PlatformScrollEvent,
    /// [`FFI_INPUT_TOUCH`].
    pub touch: PlatformTouchEvent,
    /// [`FFI_INPUT_IME`].
    pub ime: PlatformImeEvent,
}

/// Keyboard, mouse, touch or input-method input.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PlatformInputEvent {
    /// An `FFI_INPUT_*` constant: which member of `data` is valid.
    pub kind: u8,
    /// The payload.
    pub data: PlatformInputData,
}

/// The payload of a [`PlatformEvent`]; which member is valid depends on `kind`.
#[repr(C)]
#[derive(Clone, Copy)]
pub union PlatformEventData {
    /// No payload.
    pub empty: u8,
    /// [`FFI_EVENT_RESIZED`].
    pub size: PlatformSize,
    /// [`FFI_EVENT_MOVED`].
    pub position: PlatformPosition,
    /// [`FFI_EVENT_SCALE_FACTOR_CHANGED`].
    pub scale_factor_changed: PlatformScaleFactorChange,
    /// [`FFI_EVENT_FOCUSED`] and [`FFI_EVENT_OCCLUDED`].
    pub flag: bool,
    /// [`FFI_EVENT_SURFACE_CREATED`].
    pub surface: PlatformNativeHandle,
    /// [`FFI_EVENT_INPUT`].
    pub input: PlatformInputEvent,
}

/// Something that happened to a window.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PlatformEvent {
    /// An `FFI_EVENT_*` constant: which member of `data` is valid.
    pub kind: u8,
    /// The payload.
    pub data: PlatformEventData,
}

/// What the application does when something happens. Every function pointer is
/// optional: a null one is skipped.
///
/// All callbacks run on the thread that called [`ffi_platform_run`], with
/// `user_data` as their first argument.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PlatformCallbacks {
    /// Passed back to every callback; the application's own state.
    pub user_data: *mut c_void,
    /// The application may (again) show windows: create them here. Called once
    /// at the start on the desktop; on Android and iOS also after every return
    /// from the background.
    pub resumed: Option<unsafe extern "C" fn(*mut c_void, *mut PlatformContext)>,
    /// The application goes to the background (mobile only): save state; every
    /// surface is lost right before.
    pub suspended: Option<unsafe extern "C" fn(*mut c_void, *mut PlatformContext)>,
    /// A window event. Arguments: `user_data`, the context, the window id and
    /// the event.
    pub window_event:
        Option<unsafe extern "C" fn(*mut c_void, *mut PlatformContext, u64, *const PlatformEvent)>,
    /// All pending events were handled and the loop is about to wait: update
    /// the game, ask for a redraw. The place where a frame is simulated.
    pub about_to_wait: Option<unsafe extern "C" fn(*mut c_void, *mut PlatformContext)>,
}

/// How [`ffi_platform_run`] runs.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PlatformRunConfig {
    /// `FFI_BACKEND_DESKTOP` or `FFI_BACKEND_HEADLESS`.
    pub backend: u8,
    /// Headless only: the scale factor windows start with; `0` for the default.
    pub headless_scale_factor: f64,
    /// Headless only: the width of the virtual monitor in pixels; `0` for the
    /// default (together with the height).
    pub headless_monitor_width: u32,
    /// Headless only: the height of the virtual monitor in pixels; `0` for the
    /// default.
    pub headless_monitor_height: u32,
}

/// A description of a window to create. Start from
/// [`ffi_platform_window_desc_default`] and change what you need. Sizes and
/// positions are logical units (independent of the monitor's scale factor).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PlatformWindowDesc {
    /// The title, UTF-8, NUL-terminated; null for the default.
    pub title: *const c_char,
    /// Width of the client area.
    pub width: f64,
    /// Height of the client area.
    pub height: f64,
    /// Whether `x` and `y` are used; otherwise the window system chooses.
    pub has_position: bool,
    /// Horizontal position of the top-left corner.
    pub x: f64,
    /// Vertical position of the top-left corner.
    pub y: f64,
    /// Whether `min_width` and `min_height` are used.
    pub has_min_size: bool,
    /// Smallest width the user can resize to.
    pub min_width: f64,
    /// Smallest height the user can resize to.
    pub min_height: f64,
    /// Whether `max_width` and `max_height` are used.
    pub has_max_size: bool,
    /// Largest width the user can resize to.
    pub max_width: f64,
    /// Largest height the user can resize to.
    pub max_height: f64,
    /// Whether the user can resize the window.
    pub resizable: bool,
    /// Whether the window is shown.
    pub visible: bool,
    /// Whether the window has a title bar and borders.
    pub decorations: bool,
    /// An `FFI_FULLSCREEN_*` constant.
    pub fullscreen: u8,
}

/// The current state of a window, filled by [`ffi_platform_ctx_window_info`].
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PlatformWindowInfo {
    /// Width of the client area in physical pixels.
    pub inner_width: u32,
    /// Height of the client area in physical pixels.
    pub inner_height: u32,
    /// Physical pixels per logical unit.
    pub scale_factor: f64,
    /// Whether the window has keyboard focus.
    pub focused: bool,
    /// Whether the window is fully hidden.
    pub occluded: bool,
    /// Whether the window is shown.
    pub visible: bool,
    /// An `FFI_FULLSCREEN_*` constant.
    pub fullscreen: u8,
    /// An `FFI_CURSOR_GRAB_*` constant.
    pub cursor_grab: u8,
    /// Whether input-method events are allowed.
    pub ime_allowed: bool,
}

thread_local! {
    /// The context of the callback that is running on this thread, to refuse
    /// a stale or foreign `PlatformContext*`.
    static ACTIVE: Cell<*mut AppContext> = const { Cell::new(std::ptr::null_mut()) };
}

/// Marks a context as the active one for the duration of a callback.
struct ActiveGuard {
    context: *mut AppContext,
    previous: *mut AppContext,
}

impl ActiveGuard {
    fn enter(context: &mut AppContext) -> Self {
        let context: *mut AppContext = context;
        let previous = ACTIVE.replace(context);
        Self { context, previous }
    }

    fn as_ptr(&self) -> *mut PlatformContext {
        self.context.cast()
    }
}

impl Drop for ActiveGuard {
    fn drop(&mut self) {
        ACTIVE.set(self.previous);
    }
}

/// Strings that must outlive a callback's event argument.
#[derive(Default)]
struct Texts(Vec<CString>);

impl Texts {
    /// A NUL-terminated copy of `text` (interior NUL bytes dropped); the
    /// pointer is valid as long as `self` lives.
    fn add(&mut self, text: &str) -> *const c_char {
        let bytes: Vec<u8> = text.bytes().filter(|byte| *byte != 0).collect();
        let owned = CString::new(bytes).unwrap_or_default();
        let ptr = owned.as_ptr();
        self.0.push(owned);
        ptr
    }
}

const fn event(kind: u8, data: PlatformEventData) -> PlatformEvent {
    PlatformEvent { kind, data }
}

const fn no_payload(kind: u8) -> PlatformEvent {
    event(kind, PlatformEventData { empty: 0 })
}

fn pointer(address: usize) -> *mut c_void {
    std::ptr::without_provenance_mut(address)
}

const fn size(size: PhysicalSize) -> PlatformSize {
    PlatformSize {
        width: size.width,
        height: size.height,
    }
}

fn native_handle(handle: &NativeWindowHandle) -> PlatformNativeHandle {
    let mut out = PlatformNativeHandle {
        kind: crate::platform::FFI_DISPLAY_UNKNOWN,
        hwnd: std::ptr::null_mut(),
        hinstance: std::ptr::null_mut(),
        metal_layer: std::ptr::null_mut(),
        xlib_display: std::ptr::null_mut(),
        xlib_window: 0,
        wayland_display: std::ptr::null_mut(),
        wayland_surface: std::ptr::null_mut(),
        native_window: std::ptr::null_mut(),
    };
    match *handle {
        NativeWindowHandle::Win32 { hwnd, hinstance } => {
            out.kind = crate::platform::FFI_DISPLAY_WIN32;
            out.hwnd = pointer(hwnd.get());
            out.hinstance = hinstance.map_or(std::ptr::null_mut(), |h| pointer(h.get()));
        }
        NativeWindowHandle::Cocoa { metal_layer } => {
            out.kind = crate::platform::FFI_DISPLAY_COCOA;
            out.metal_layer = pointer(metal_layer.get());
        }
        NativeWindowHandle::X11 { display, window } => {
            out.kind = crate::platform::FFI_DISPLAY_X11;
            out.xlib_display = display.map_or(std::ptr::null_mut(), |d| pointer(d.get()));
            out.xlib_window = window.get();
        }
        NativeWindowHandle::Wayland { display, surface } => {
            out.kind = crate::platform::FFI_DISPLAY_WAYLAND;
            out.wayland_display = pointer(display.get());
            out.wayland_surface = pointer(surface.get());
        }
        NativeWindowHandle::Android { native_window } => {
            out.kind = crate::platform::FFI_DISPLAY_ANDROID;
            out.native_window = pointer(native_window.get());
        }
        NativeWindowHandle::UiKit { metal_layer } => {
            out.kind = crate::platform::FFI_DISPLAY_UIKIT;
            out.metal_layer = pointer(metal_layer.get());
        }
        // `NativeWindowHandle` is `#[non_exhaustive]`.
        _ => {}
    }
    out
}

const fn button_state(state: ButtonState) -> u8 {
    match state {
        ButtonState::Pressed => FFI_BUTTON_PRESSED,
        ButtonState::Released => FFI_BUTTON_RELEASED,
    }
}

fn modifier_mask(modifiers: Modifiers) -> u8 {
    let mut mask = 0;
    for (modifier, bit) in [
        (Modifiers::SHIFT, FFI_MOD_SHIFT),
        (Modifiers::CONTROL, FFI_MOD_CONTROL),
        (Modifiers::ALT, FFI_MOD_ALT),
        (Modifiers::SUPER, FFI_MOD_SUPER),
    ] {
        if modifiers.contains(modifier) {
            mask |= bit;
        }
    }
    mask
}

fn key_event(key: &KeyEvent, texts: &mut Texts) -> PlatformKeyEvent {
    let (logical_kind, logical_named, logical_text) = match &key.logical_key {
        Key::Named(named) => (FFI_LOGICAL_NAMED, keys::named_key(*named), std::ptr::null()),
        Key::Character(text) => (FFI_LOGICAL_CHARACTER, 0, texts.add(text)),
        Key::Dead(accent) => (
            FFI_LOGICAL_DEAD,
            0,
            accent.map_or(std::ptr::null(), |c| texts.add(c.encode_utf8(&mut [0; 4]))),
        ),
        // `Key` is `#[non_exhaustive]`: unidentified and any later variant.
        _ => (FFI_LOGICAL_UNIDENTIFIED, 0, std::ptr::null()),
    };
    PlatformKeyEvent {
        physical_key: keys::key_code(key.physical_key),
        logical_kind,
        logical_named,
        logical_text,
        text: key
            .text
            .as_deref()
            .map_or(std::ptr::null(), |text| texts.add(text)),
        state: button_state(key.state),
        repeat: key.repeat,
        synthetic: key.synthetic,
    }
}

const fn input_event(kind: u8, data: PlatformInputData) -> PlatformInputEvent {
    PlatformInputEvent { kind, data }
}

fn input_to_c(input: &InputEvent, texts: &mut Texts) -> PlatformInputEvent {
    let unknown = input_event(FFI_INPUT_UNKNOWN, PlatformInputData { empty: 0 });
    match input {
        InputEvent::Key(key) => input_event(
            FFI_INPUT_KEY,
            PlatformInputData {
                key: key_event(key, texts),
            },
        ),
        InputEvent::ModifiersChanged(modifiers) => input_event(
            FFI_INPUT_MODIFIERS_CHANGED,
            PlatformInputData {
                modifiers: modifier_mask(*modifiers),
            },
        ),
        InputEvent::CursorMoved(position) => input_event(
            FFI_INPUT_CURSOR_MOVED,
            PlatformInputData {
                point: PlatformPoint {
                    x: position.x,
                    y: position.y,
                },
            },
        ),
        InputEvent::CursorEntered => {
            input_event(FFI_INPUT_CURSOR_ENTERED, PlatformInputData { empty: 0 })
        }
        InputEvent::CursorLeft => {
            input_event(FFI_INPUT_CURSOR_LEFT, PlatformInputData { empty: 0 })
        }
        InputEvent::MouseButton { button, state } => {
            let (button, other) = match button {
                MouseButton::Left => (FFI_MOUSE_LEFT, 0),
                MouseButton::Right => (FFI_MOUSE_RIGHT, 0),
                MouseButton::Middle => (FFI_MOUSE_MIDDLE, 0),
                MouseButton::Back => (FFI_MOUSE_BACK, 0),
                MouseButton::Forward => (FFI_MOUSE_FORWARD, 0),
                MouseButton::Other(number) => (FFI_MOUSE_OTHER, *number),
                // `MouseButton` is `#[non_exhaustive]`.
                _ => (FFI_MOUSE_OTHER, u16::MAX),
            };
            input_event(
                FFI_INPUT_MOUSE_BUTTON,
                PlatformInputData {
                    mouse_button: PlatformMouseButtonEvent {
                        button,
                        other,
                        state: button_state(*state),
                    },
                },
            )
        }
        InputEvent::MouseWheel(delta) => {
            let (unit, x, y) = match *delta {
                ScrollDelta::Lines { x, y } => (FFI_SCROLL_LINES, x, y),
                ScrollDelta::Pixels { x, y } => (FFI_SCROLL_PIXELS, x, y),
                // `ScrollDelta` is `#[non_exhaustive]`.
                _ => return unknown,
            };
            input_event(
                FFI_INPUT_MOUSE_WHEEL,
                PlatformInputData {
                    scroll: PlatformScrollEvent { unit, x, y },
                },
            )
        }
        InputEvent::RawMouseMotion { delta_x, delta_y } => input_event(
            FFI_INPUT_RAW_MOUSE_MOTION,
            PlatformInputData {
                point: PlatformPoint {
                    x: *delta_x,
                    y: *delta_y,
                },
            },
        ),
        InputEvent::Touch(touch) => {
            let phase = match touch.phase {
                TouchPhase::Started => FFI_TOUCH_STARTED,
                TouchPhase::Moved => FFI_TOUCH_MOVED,
                TouchPhase::Ended => FFI_TOUCH_ENDED,
                TouchPhase::Cancelled => FFI_TOUCH_CANCELLED,
                // `TouchPhase` is `#[non_exhaustive]`.
                _ => return unknown,
            };
            input_event(
                FFI_INPUT_TOUCH,
                PlatformInputData {
                    touch: PlatformTouchEvent {
                        id: touch.id,
                        phase,
                        x: touch.position.x,
                        y: touch.position.y,
                    },
                },
            )
        }
        InputEvent::Ime(ime) => {
            let converted = match ime {
                ImeEvent::Enabled => ime_event(FFI_IME_ENABLED, std::ptr::null(), None),
                ImeEvent::Disabled => ime_event(FFI_IME_DISABLED, std::ptr::null(), None),
                ImeEvent::Preedit { text, cursor } => {
                    ime_event(FFI_IME_PREEDIT, texts.add(text), *cursor)
                }
                ImeEvent::Commit(text) => ime_event(FFI_IME_COMMIT, texts.add(text), None),
                // `ImeEvent` is `#[non_exhaustive]`.
                _ => ime_event(FFI_IME_UNKNOWN, std::ptr::null(), None),
            };
            input_event(FFI_INPUT_IME, PlatformInputData { ime: converted })
        }
        // `InputEvent` is `#[non_exhaustive]`.
        _ => unknown,
    }
}

fn ime_event(kind: u8, text: *const c_char, cursor: Option<(usize, usize)>) -> PlatformImeEvent {
    PlatformImeEvent {
        kind,
        text,
        has_cursor: cursor.is_some(),
        cursor_start: cursor.map_or(0, |(start, _)| start as u64),
        cursor_end: cursor.map_or(0, |(_, end)| end as u64),
    }
}

/// Converts a window event to its C form. Pointers inside it point into
/// `texts`.
fn event_to_c(window_event: &WindowEvent, texts: &mut Texts) -> PlatformEvent {
    match window_event {
        WindowEvent::CloseRequested => no_payload(FFI_EVENT_CLOSE_REQUESTED),
        WindowEvent::Destroyed => no_payload(FFI_EVENT_DESTROYED),
        WindowEvent::Resized(new_size) => event(
            FFI_EVENT_RESIZED,
            PlatformEventData {
                size: size(*new_size),
            },
        ),
        WindowEvent::Moved(position) => event(
            FFI_EVENT_MOVED,
            PlatformEventData {
                position: PlatformPosition {
                    x: position.x,
                    y: position.y,
                },
            },
        ),
        WindowEvent::ScaleFactorChanged {
            scale_factor,
            inner_size,
        } => event(
            FFI_EVENT_SCALE_FACTOR_CHANGED,
            PlatformEventData {
                scale_factor_changed: PlatformScaleFactorChange {
                    scale_factor: *scale_factor,
                    inner_size: size(*inner_size),
                },
            },
        ),
        WindowEvent::Focused(focused) => {
            event(FFI_EVENT_FOCUSED, PlatformEventData { flag: *focused })
        }
        WindowEvent::Occluded(occluded) => {
            event(FFI_EVENT_OCCLUDED, PlatformEventData { flag: *occluded })
        }
        WindowEvent::RedrawRequested => no_payload(FFI_EVENT_REDRAW_REQUESTED),
        WindowEvent::SurfaceCreated(handle) => event(
            FFI_EVENT_SURFACE_CREATED,
            PlatformEventData {
                surface: native_handle(handle),
            },
        ),
        WindowEvent::SurfaceLost => no_payload(FFI_EVENT_SURFACE_LOST),
        WindowEvent::Input(input) => event(
            FFI_EVENT_INPUT,
            PlatformEventData {
                input: input_to_c(input, texts),
            },
        ),
        // `WindowEvent` is `#[non_exhaustive]`.
        _ => no_payload(FFI_EVENT_UNKNOWN),
    }
}

/// The `App` that forwards everything to the C callback table.
struct FfiApp {
    callbacks: PlatformCallbacks,
}

impl FfiApp {
    fn call(
        &self,
        callback: Option<unsafe extern "C" fn(*mut c_void, *mut PlatformContext)>,
        ctx: &mut AppContext,
    ) {
        if let Some(callback) = callback {
            let guard = ActiveGuard::enter(ctx);
            // SAFETY: the function pointer was supplied by the caller of
            // `ffi_platform_run`, who promised it is callable with these
            // arguments; `ctx` is valid until the callback returns.
            unsafe { callback(self.callbacks.user_data, guard.as_ptr()) };
        }
    }
}

impl App for FfiApp {
    fn resumed(&mut self, ctx: &mut AppContext) {
        self.call(self.callbacks.resumed, ctx);
    }

    fn suspended(&mut self, ctx: &mut AppContext) {
        self.call(self.callbacks.suspended, ctx);
    }

    fn window_event(&mut self, ctx: &mut AppContext, window: WindowId, event: WindowEvent) {
        let Some(callback) = self.callbacks.window_event else {
            return;
        };
        let mut texts = Texts::default();
        let converted = event_to_c(&event, &mut texts);
        let guard = ActiveGuard::enter(ctx);
        // SAFETY: as in `call`; `converted` and the strings in `texts` live
        // until the end of this function.
        unsafe {
            callback(
                self.callbacks.user_data,
                guard.as_ptr(),
                window.as_u64(),
                &raw const converted,
            );
        }
    }

    fn about_to_wait(&mut self, ctx: &mut AppContext) {
        self.call(self.callbacks.about_to_wait, ctx);
    }
}

/// Runs `callbacks` on `backend` until the application exits.
fn run_backend<B: Backend>(backend: B, callbacks: PlatformCallbacks) -> Result<i32, WindowError> {
    backend.run(FfiApp { callbacks })
}

#[cfg(any(windows, target_os = "macos", target_os = "linux"))]
fn run_desktop(callbacks: PlatformCallbacks) -> Result<i32, WindowError> {
    run_backend(platform_crate::window::WinitBackend::new(), callbacks)
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn run_desktop(_callbacks: PlatformCallbacks) -> Result<i32, WindowError> {
    Err(WindowError::Backend(
        "the desktop window backend is not available on this platform".to_owned(),
    ))
}

fn run_headless(
    config: &PlatformRunConfig,
    callbacks: PlatformCallbacks,
) -> Result<i32, WindowError> {
    let mut headless = HeadlessConfig::default();
    if config.headless_scale_factor != 0.0 {
        headless = headless.with_scale_factor(config.headless_scale_factor);
    }
    if config.headless_monitor_width != 0 && config.headless_monitor_height != 0 {
        headless = headless.with_monitor_size(PhysicalSize::new(
            config.headless_monitor_width,
            config.headless_monitor_height,
        ));
    }
    run_backend(HeadlessBackend::new(headless), callbacks)
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    let text = payload
        .downcast_ref::<&str>()
        .map(|text| (*text).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".to_owned());
    format!("a panic reached the platform loop: {text}")
}

/// Runs the application's event loop on the calling thread until it exits, and
/// writes the exit code it asked for (`ffi_platform_ctx_exit`; `0` for a plain
/// exit) to `*out_exit_code`. Returns `false` when the loop could not run or
/// stopped on an error; see [`ffi_platform_last_error_code`](crate::platform::ffi_platform_last_error_code)
/// (`FFI_PLATFORM_ERROR_WINDOW` when the window system failed or, for the
/// headless backend, when the application never exits).
///
/// `config` may be null: the desktop backend with defaults.
///
/// # Safety
/// * `callbacks` must be a valid, non-null pointer to a [`PlatformCallbacks`];
///   it is copied, so it need not outlive this call. Every non-null function
///   pointer in it must be callable as documented and must not let a C++
///   exception or `longjmp` leave it. `user_data` is passed through untouched
///   and may be anything the callbacks understand.
/// * `config` must be null or a valid pointer to a [`PlatformRunConfig`].
/// * `out_exit_code` must be a valid, non-null, writable pointer.
/// * Call it on the main thread (required on macOS, and by the other desktop
///   systems for this backend), and not from inside one of its own callbacks.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_platform_run(
    callbacks: *const PlatformCallbacks,
    config: *const PlatformRunConfig,
    out_exit_code: *mut i32,
) -> bool {
    clear_last_error();
    if callbacks.is_null() || out_exit_code.is_null() {
        set_last_error(Failure::InvalidArgument(
            "callbacks and out_exit_code must not be null".to_owned(),
        ));
        return false;
    }
    // SAFETY: non-null and valid per the contract; `PlatformCallbacks` is
    // `Copy`.
    let callbacks = unsafe { *callbacks };
    let config = if config.is_null() {
        PlatformRunConfig {
            backend: FFI_BACKEND_DESKTOP,
            headless_scale_factor: 0.0,
            headless_monitor_width: 0,
            headless_monitor_height: 0,
        }
    } else {
        // SAFETY: non-null and valid per the contract.
        unsafe { *config }
    };

    let outcome = catch_unwind(AssertUnwindSafe(|| match config.backend {
        FFI_BACKEND_DESKTOP => run_desktop(callbacks),
        FFI_BACKEND_HEADLESS => run_headless(&config, callbacks),
        other => Err(WindowError::InvalidArgument(format!(
            "unknown backend {other}"
        ))),
    }));
    match outcome {
        Ok(Ok(code)) => {
            // SAFETY: `out_exit_code` is valid and writable per the contract.
            unsafe { out_exit_code.write(code) };
            true
        }
        Ok(Err(error)) => {
            set_last_error(Failure::Window(error));
            false
        }
        Err(payload) => {
            set_last_error(Failure::Panic(panic_message(payload.as_ref())));
            false
        }
    }
}

/// Resolves a `PlatformContext*` to the context of the running callback.
/// Sets the last error and returns `None` for null, stale or foreign pointers
/// and for a window error of `operation`.
fn with_context<R>(
    ctx: *mut PlatformContext,
    operation: impl FnOnce(&mut AppContext) -> Result<R, WindowError>,
) -> Option<R> {
    let context = ctx.cast::<AppContext>();
    if context.is_null() || context != ACTIVE.get() {
        set_last_error(Failure::InvalidArgument(
            "ctx is null or not the context of a callback that is running on this thread"
                .to_owned(),
        ));
        return None;
    }
    // SAFETY: `context` is the pointer `ActiveGuard::enter` stored for the
    // callback that is running right now on this thread, so it points to the
    // live `AppContext` the event loop lent to that callback; the loop does not
    // touch it until the callback returns.
    let context = unsafe { &mut *context };
    match operation(context) {
        Ok(value) => Some(value),
        Err(error) => {
            set_last_error(Failure::Window(error));
            None
        }
    }
}

/// `with_context` for operations without a result.
fn with_context_ok(
    ctx: *mut PlatformContext,
    operation: impl FnOnce(&mut AppContext) -> Result<(), WindowError>,
) -> bool {
    clear_last_error();
    with_context(ctx, operation).is_some()
}

/// Reads a UTF-8 C string argument.
///
/// # Safety
/// `text` must be null or a valid NUL-terminated string.
unsafe fn read_text(text: *const c_char, what: &str) -> Result<Option<String>, Failure> {
    if text.is_null() {
        return Ok(None);
    }
    // SAFETY: non-null and NUL-terminated per the contract.
    unsafe { CStr::from_ptr(text) }
        .to_str()
        .map(|text| Some(text.to_owned()))
        .map_err(|_| Failure::InvalidArgument(format!("{what} is not valid UTF-8")))
}

const fn fullscreen_from_code(code: u8) -> Option<Fullscreen> {
    match code {
        FFI_FULLSCREEN_NONE => Some(Fullscreen::None),
        FFI_FULLSCREEN_BORDERLESS => Some(Fullscreen::Borderless),
        _ => None,
    }
}

const fn fullscreen_code(fullscreen: Fullscreen) -> u8 {
    match fullscreen {
        Fullscreen::Borderless => FFI_FULLSCREEN_BORDERLESS,
        // `Fullscreen` is `#[non_exhaustive]`.
        _ => FFI_FULLSCREEN_NONE,
    }
}

const fn grab_from_code(code: u8) -> Option<CursorGrab> {
    match code {
        FFI_CURSOR_GRAB_NONE => Some(CursorGrab::None),
        FFI_CURSOR_GRAB_CONFINED => Some(CursorGrab::Confined),
        FFI_CURSOR_GRAB_LOCKED => Some(CursorGrab::Locked),
        _ => None,
    }
}

const fn grab_code(grab: CursorGrab) -> u8 {
    match grab {
        CursorGrab::Confined => FFI_CURSOR_GRAB_CONFINED,
        CursorGrab::Locked => FFI_CURSOR_GRAB_LOCKED,
        // `CursorGrab` is `#[non_exhaustive]`.
        _ => FFI_CURSOR_GRAB_NONE,
    }
}

/// Fills `*out` with the defaults of a window: title "Coreverse", 1280 x 720,
/// system-chosen position, resizable, visible, decorated, not fullscreen. Change
/// the fields you need, then pass it to [`ffi_platform_ctx_create_window`]. The
/// default title is a static string that lives for the whole process.
///
/// # Safety
/// `out` must be a valid, non-null, writable pointer to a [`PlatformWindowDesc`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_platform_window_desc_default(out: *mut PlatformWindowDesc) {
    let defaults = WindowDesc::default();
    let desc = PlatformWindowDesc {
        title: c"Coreverse".as_ptr(),
        width: defaults.inner_size.width,
        height: defaults.inner_size.height,
        has_position: false,
        x: 0.0,
        y: 0.0,
        has_min_size: false,
        min_width: 0.0,
        min_height: 0.0,
        has_max_size: false,
        max_width: 0.0,
        max_height: 0.0,
        resizable: defaults.resizable,
        visible: defaults.visible,
        decorations: defaults.decorations,
        fullscreen: fullscreen_code(defaults.fullscreen),
    };
    // SAFETY: `out` is valid and writable per the contract.
    unsafe { out.write(desc) };
}

/// # Safety
/// The title pointer of `desc` must be null or a valid NUL-terminated string.
unsafe fn window_desc(desc: &PlatformWindowDesc) -> Result<WindowDesc, Failure> {
    // SAFETY: forwarded from this function's contract.
    let title = unsafe { read_text(desc.title, "the window title") }?;
    let fullscreen = fullscreen_from_code(desc.fullscreen).ok_or_else(|| {
        Failure::InvalidArgument(format!("unknown fullscreen mode {}", desc.fullscreen))
    })?;
    let mut window = WindowDesc::default()
        .with_inner_size(LogicalSize::new(desc.width, desc.height))
        .with_resizable(desc.resizable)
        .with_visible(desc.visible)
        .with_decorations(desc.decorations)
        .with_fullscreen(fullscreen);
    if let Some(title) = title {
        window = window.with_title(title);
    }
    if desc.has_position {
        window = window.with_position(LogicalPosition::new(desc.x, desc.y));
    }
    if desc.has_min_size {
        window = window.with_min_inner_size(LogicalSize::new(desc.min_width, desc.min_height));
    }
    if desc.has_max_size {
        window = window.with_max_inner_size(LogicalSize::new(desc.max_width, desc.max_height));
    }
    Ok(window)
}

/// Creates a window and writes its id to `*out_id`. The window exists at once
/// (readable with [`ffi_platform_ctx_window_info`]); on the desktop its surface
/// follows as an `FFI_EVENT_SURFACE_CREATED` event.
///
/// # Safety
/// * `ctx` must come from a callback that is still running (a stale or foreign
///   pointer is refused with an error, a pointer that is not a context at all
///   is undefined behavior).
/// * `desc` must be a valid, non-null pointer to a [`PlatformWindowDesc`] whose
///   `title` is null or a valid NUL-terminated string.
/// * `out_id` must be a valid, non-null, writable pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_platform_ctx_create_window(
    ctx: *mut PlatformContext,
    desc: *const PlatformWindowDesc,
    out_id: *mut u64,
) -> bool {
    clear_last_error();
    if desc.is_null() || out_id.is_null() {
        set_last_error(Failure::InvalidArgument(
            "desc and out_id must not be null".to_owned(),
        ));
        return false;
    }
    // SAFETY: valid per the contract.
    let desc = match unsafe { window_desc(&*desc) } {
        Ok(desc) => desc,
        Err(failure) => {
            set_last_error(failure);
            return false;
        }
    };
    match with_context(ctx, |context| context.create_window(desc)) {
        Some(id) => {
            // SAFETY: `out_id` is valid and writable per the contract.
            unsafe { out_id.write(id.as_u64()) };
            true
        }
        None => false,
    }
}

/// Closes a window. Its `FFI_EVENT_SURFACE_LOST` (if it had a surface) and
/// `FFI_EVENT_DESTROYED` events follow.
///
/// # Safety
/// `ctx` must come from a callback that is still running, see
/// [`ffi_platform_ctx_create_window`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_platform_ctx_close_window(ctx: *mut PlatformContext, id: u64) -> bool {
    with_context_ok(ctx, |context| context.close_window(WindowId::from_u64(id)))
}

/// Sets the title of a window.
///
/// # Safety
/// `ctx` as for [`ffi_platform_ctx_create_window`]; `title` must be a valid,
/// non-null, NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_platform_ctx_set_title(
    ctx: *mut PlatformContext,
    id: u64,
    title: *const c_char,
) -> bool {
    clear_last_error();
    // SAFETY: valid per the contract.
    let title = match unsafe { read_text(title, "the title") } {
        Ok(Some(title)) => title,
        Ok(None) => {
            set_last_error(Failure::InvalidArgument(
                "title must not be null".to_owned(),
            ));
            return false;
        }
        Err(failure) => {
            set_last_error(failure);
            return false;
        }
    };
    with_context(ctx, |context| {
        context.set_title(WindowId::from_u64(id), title)
    })
        .is_some()
}

/// Resizes the client area of a window to `width` x `height` logical units.
///
/// # Safety
/// `ctx` as for [`ffi_platform_ctx_create_window`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_platform_ctx_set_inner_size(
    ctx: *mut PlatformContext,
    id: u64,
    width: f64,
    height: f64,
) -> bool {
    with_context_ok(ctx, |context| {
        context.set_inner_size(WindowId::from_u64(id), LogicalSize::new(width, height))
    })
}

/// Switches a window to or from fullscreen (`FFI_FULLSCREEN_*`).
///
/// # Safety
/// `ctx` as for [`ffi_platform_ctx_create_window`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_platform_ctx_set_fullscreen(
    ctx: *mut PlatformContext,
    id: u64,
    fullscreen: u8,
) -> bool {
    let Some(fullscreen) = fullscreen_from_code(fullscreen) else {
        clear_last_error();
        set_last_error(Failure::InvalidArgument(format!(
            "unknown fullscreen mode {fullscreen}"
        )));
        return false;
    };
    with_context_ok(ctx, |context| {
        context.set_fullscreen(WindowId::from_u64(id), fullscreen)
    })
}

/// Asks for an `FFI_EVENT_REDRAW_REQUESTED` event for the window in the next
/// loop iteration.
///
/// # Safety
/// `ctx` as for [`ffi_platform_ctx_create_window`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_platform_ctx_request_redraw(
    ctx: *mut PlatformContext,
    id: u64,
) -> bool {
    with_context_ok(ctx, |context| {
        context.request_redraw(WindowId::from_u64(id))
    })
}

/// Shows or hides the mouse cursor over a window.
///
/// # Safety
/// `ctx` as for [`ffi_platform_ctx_create_window`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_platform_ctx_set_cursor_visible(
    ctx: *mut PlatformContext,
    id: u64,
    visible: bool,
) -> bool {
    with_context_ok(ctx, |context| {
        context.set_cursor_visible(WindowId::from_u64(id), visible)
    })
}

/// Confines or locks the mouse cursor (`FFI_CURSOR_GRAB_*`). Systems differ in
/// what they support; the window system's refusal is not an error here, read
/// the result back with [`ffi_platform_ctx_window_info`].
///
/// # Safety
/// `ctx` as for [`ffi_platform_ctx_create_window`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_platform_ctx_set_cursor_grab(
    ctx: *mut PlatformContext,
    id: u64,
    grab: u8,
) -> bool {
    let Some(grab) = grab_from_code(grab) else {
        clear_last_error();
        set_last_error(Failure::InvalidArgument(format!(
            "unknown cursor grab mode {grab}"
        )));
        return false;
    };
    with_context_ok(ctx, |context| {
        context.set_cursor_grab(WindowId::from_u64(id), grab)
    })
}

/// Allows or forbids input-method events for a window: switch it on while a
/// text field has focus.
///
/// # Safety
/// `ctx` as for [`ffi_platform_ctx_create_window`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_platform_ctx_set_ime_allowed(
    ctx: *mut PlatformContext,
    id: u64,
    allowed: bool,
) -> bool {
    with_context_ok(ctx, |context| {
        context.set_ime_allowed(WindowId::from_u64(id), allowed)
    })
}

/// Tells the input method where the text caret is (logical units of the client
/// area), so that its candidate window does not cover the text.
///
/// # Safety
/// `ctx` as for [`ffi_platform_ctx_create_window`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_platform_ctx_set_ime_cursor_area(
    ctx: *mut PlatformContext,
    id: u64,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> bool {
    with_context_ok(ctx, |context| {
        context.set_ime_cursor_area(
            WindowId::from_u64(id),
            LogicalPosition::new(x, y),
            LogicalSize::new(width, height),
        )
    })
}

/// Chooses how the loop waits between iterations (`FFI_CONTROL_FLOW_*`):
/// `Poll` for a game that draws continuously, `Wait` for a tool,
/// `WaitUntil` to wake up after `wait_millis` milliseconds at the latest
/// (ignored for the other two).
///
/// # Safety
/// `ctx` as for [`ffi_platform_ctx_create_window`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_platform_ctx_set_control_flow(
    ctx: *mut PlatformContext,
    kind: u8,
    wait_millis: u64,
) -> bool {
    let flow = match kind {
        FFI_CONTROL_FLOW_POLL => ControlFlow::Poll,
        FFI_CONTROL_FLOW_WAIT => ControlFlow::Wait,
        FFI_CONTROL_FLOW_WAIT_UNTIL => {
            ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(wait_millis))
        }
        other => {
            clear_last_error();
            set_last_error(Failure::InvalidArgument(format!(
                "unknown control flow {other}"
            )));
            return false;
        }
    };
    with_context_ok(ctx, |context| {
        context.set_control_flow(flow);
        Ok(())
    })
}

/// Ends the event loop after the current callback and makes
/// [`ffi_platform_run`] report `exit_code`. A second call replaces the code.
///
/// # Safety
/// `ctx` as for [`ffi_platform_ctx_create_window`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_platform_ctx_exit(ctx: *mut PlatformContext, exit_code: i32) -> bool {
    with_context_ok(ctx, |context| {
        context.exit_with_code(exit_code);
        Ok(())
    })
}

/// Reads the current state of a window into `*out`.
///
/// # Safety
/// `ctx` as for [`ffi_platform_ctx_create_window`]; `out` must be a valid,
/// non-null, writable pointer to a [`PlatformWindowInfo`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_platform_ctx_window_info(
    ctx: *mut PlatformContext,
    id: u64,
    out: *mut PlatformWindowInfo,
) -> bool {
    clear_last_error();
    if out.is_null() {
        set_last_error(Failure::InvalidArgument("out must not be null".to_owned()));
        return false;
    }
    let info = with_context(ctx, |context| {
        let window_id = WindowId::from_u64(id);
        let window = context
            .window(window_id)
            .ok_or(WindowError::UnknownWindow(window_id))?;
        let inner = window.inner_size();
        Ok(PlatformWindowInfo {
            inner_width: inner.width,
            inner_height: inner.height,
            scale_factor: window.scale_factor(),
            focused: window.is_focused(),
            occluded: window.is_occluded(),
            visible: window.is_visible(),
            fullscreen: fullscreen_code(window.fullscreen()),
            cursor_grab: grab_code(window.cursor_grab()),
            ime_allowed: window.is_ime_allowed(),
        })
    });
    match info {
        Some(info) => {
            // SAFETY: `out` is valid and writable per the contract.
            unsafe { out.write(info) };
            true
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{
        FFI_PLATFORM_ERROR_INVALID_ARGUMENT, FFI_PLATFORM_ERROR_NONE, FFI_PLATFORM_ERROR_WINDOW,
        ffi_platform_last_error_code, ffi_platform_last_error_message,
    };
    use platform_crate::input::{KeyCode, NamedKey, PointerPosition, TouchEvent};
    use platform_crate::window::{HostEvent, PhysicalPosition};
    use std::num::{NonZeroU64, NonZeroUsize};

    // --- reading events the way C++ does ------------------------------------

    fn text(ptr: *const c_char) -> String {
        if ptr.is_null() {
            return "null".to_owned();
        }
        // SAFETY: the pointers in the tests come from `Texts` that is alive.
        unsafe { CStr::from_ptr(ptr) }.to_str().unwrap().to_owned()
    }

    fn describe_input(input: &PlatformInputEvent) -> String {
        // SAFETY: each arm reads the union member that `kind` says is valid.
        unsafe {
            match input.kind {
                FFI_INPUT_KEY => {
                    let k = input.data.key;
                    format!(
                        "key phys={} logical={}/{}/{} text={} state={} repeat={} synthetic={}",
                        k.physical_key,
                        k.logical_kind,
                        k.logical_named,
                        text(k.logical_text),
                        text(k.text),
                        k.state,
                        k.repeat,
                        k.synthetic
                    )
                }
                FFI_INPUT_MODIFIERS_CHANGED => format!("modifiers {}", input.data.modifiers),
                FFI_INPUT_CURSOR_MOVED => {
                    format!("cursor {} {}", input.data.point.x, input.data.point.y)
                }
                FFI_INPUT_CURSOR_ENTERED => "cursor-entered".to_owned(),
                FFI_INPUT_CURSOR_LEFT => "cursor-left".to_owned(),
                FFI_INPUT_MOUSE_BUTTON => {
                    let b = input.data.mouse_button;
                    format!("mouse {} other={} state={}", b.button, b.other, b.state)
                }
                FFI_INPUT_MOUSE_WHEEL => {
                    let s = input.data.scroll;
                    format!("scroll unit={} {} {}", s.unit, s.x, s.y)
                }
                FFI_INPUT_RAW_MOUSE_MOTION => {
                    format!("raw {} {}", input.data.point.x, input.data.point.y)
                }
                FFI_INPUT_TOUCH => {
                    let t = input.data.touch;
                    format!("touch id={} phase={} {} {}", t.id, t.phase, t.x, t.y)
                }
                FFI_INPUT_IME => {
                    let i = input.data.ime;
                    format!(
                        "ime kind={} text={} cursor={}:{}:{}",
                        i.kind,
                        text(i.text),
                        i.has_cursor,
                        i.cursor_start,
                        i.cursor_end
                    )
                }
                other => format!("input-unknown {other}"),
            }
        }
    }

    fn describe(event: &PlatformEvent) -> String {
        // SAFETY: each arm reads the union member that `kind` says is valid.
        unsafe {
            match event.kind {
                FFI_EVENT_CLOSE_REQUESTED => "close-requested".to_owned(),
                FFI_EVENT_DESTROYED => "destroyed".to_owned(),
                FFI_EVENT_RESIZED => {
                    format!(
                        "resized {}x{}",
                        event.data.size.width, event.data.size.height
                    )
                }
                FFI_EVENT_MOVED => {
                    format!("moved {},{}", event.data.position.x, event.data.position.y)
                }
                FFI_EVENT_SCALE_FACTOR_CHANGED => {
                    let c = event.data.scale_factor_changed;
                    format!(
                        "scale {} {}x{}",
                        c.scale_factor, c.inner_size.width, c.inner_size.height
                    )
                }
                FFI_EVENT_FOCUSED => format!("focused {}", event.data.flag),
                FFI_EVENT_OCCLUDED => format!("occluded {}", event.data.flag),
                FFI_EVENT_REDRAW_REQUESTED => "redraw".to_owned(),
                FFI_EVENT_SURFACE_CREATED => {
                    let s = event.data.surface;
                    format!(
                        "surface kind={} hwnd={:?} hinst={:?} metal={:?} xdpy={:?} xwin={} wl={:?}/{:?} android={:?}",
                        s.kind,
                        s.hwnd,
                        s.hinstance,
                        s.metal_layer,
                        s.xlib_display,
                        s.xlib_window,
                        s.wayland_display,
                        s.wayland_surface,
                        s.native_window
                    )
                }
                FFI_EVENT_SURFACE_LOST => "surface-lost".to_owned(),
                FFI_EVENT_INPUT => describe_input(&event.data.input),
                other => format!("unknown {other}"),
            }
        }
    }

    fn convert(event: &WindowEvent) -> String {
        let mut texts = Texts::default();
        describe(&event_to_c(event, &mut texts))
    }

    fn input(event: InputEvent) -> String {
        convert(&WindowEvent::Input(event))
    }

    // --- a recording application --------------------------------------------

    type Hook = Box<dyn FnMut(&mut Probe, *mut PlatformContext, &str)>;

    struct Probe {
        log: Vec<String>,
        saved_context: *mut PlatformContext,
        window: u64,
        hook: Option<Hook>,
    }

    impl Probe {
        fn new(hook: Hook) -> Self {
            Self {
                log: Vec::new(),
                saved_context: std::ptr::null_mut(),
                window: 0,
                hook: Some(hook),
            }
        }

        fn fire(&mut self, ctx: *mut PlatformContext, name: &str) {
            self.log.push(name.to_owned());
            if let Some(mut hook) = self.hook.take() {
                hook(self, ctx, name);
                self.hook = Some(hook);
            }
        }

        fn callbacks(&mut self) -> PlatformCallbacks {
            PlatformCallbacks {
                user_data: std::ptr::from_mut(self).cast(),
                resumed: Some(on_resumed),
                suspended: Some(on_suspended),
                window_event: Some(on_window_event),
                about_to_wait: Some(on_about_to_wait),
            }
        }
    }

    /// # Safety
    /// `user` is the `Probe` of the test that is running the loop.
    unsafe fn probe<'a>(user: *mut c_void) -> &'a mut Probe {
        // SAFETY: see above.
        unsafe { &mut *user.cast::<Probe>() }
    }

    unsafe extern "C" fn on_resumed(user: *mut c_void, ctx: *mut PlatformContext) {
        // SAFETY: `user` is the test's `Probe`.
        unsafe { probe(user) }.fire(ctx, "resumed");
    }

    unsafe extern "C" fn on_suspended(user: *mut c_void, ctx: *mut PlatformContext) {
        // SAFETY: `user` is the test's `Probe`.
        unsafe { probe(user) }.fire(ctx, "suspended");
    }

    unsafe extern "C" fn on_about_to_wait(user: *mut c_void, ctx: *mut PlatformContext) {
        // SAFETY: `user` is the test's `Probe`.
        unsafe { probe(user) }.fire(ctx, "about_to_wait");
    }

    unsafe extern "C" fn on_window_event(
        user: *mut c_void,
        ctx: *mut PlatformContext,
        window: u64,
        event: *const PlatformEvent,
    ) {
        // SAFETY: `user` is the test's `Probe`; `event` is valid for the call.
        let probe = unsafe { probe(user) };
        // SAFETY: as above.
        let line = format!("window {window}: {}", describe(unsafe { &*event }));
        probe.fire(ctx, &line);
    }

    fn headless_config() -> PlatformRunConfig {
        PlatformRunConfig {
            backend: FFI_BACKEND_HEADLESS,
            headless_scale_factor: 0.0,
            headless_monitor_width: 0,
            headless_monitor_height: 0,
        }
    }

    fn default_desc() -> PlatformWindowDesc {
        let mut desc = std::mem::MaybeUninit::<PlatformWindowDesc>::uninit();
        // SAFETY: `desc` is a valid, writable location.
        unsafe {
            ffi_platform_window_desc_default(desc.as_mut_ptr());
            desc.assume_init()
        }
    }

    fn create_window(ctx: *mut PlatformContext, desc: &PlatformWindowDesc) -> u64 {
        let mut id = 0;
        // SAFETY: `ctx` is the context of the running callback; `desc` and
        // `id` are valid.
        assert!(unsafe { ffi_platform_ctx_create_window(ctx, desc, &raw mut id) });
        id
    }

    fn run_headless(probe: &mut Probe) -> (bool, i32) {
        let callbacks = probe.callbacks();
        let config = headless_config();
        let mut code = -1;
        // SAFETY: all pointers are valid for the call.
        let ok =
            unsafe { ffi_platform_run(&raw const callbacks, &raw const config, &raw mut code) };
        (ok, code)
    }

    fn last_error() -> (u8, Option<String>) {
        let message = ffi_platform_last_error_message();
        let text = (!message.is_null()).then(|| {
            // SAFETY: a non-null message is a valid string owned by us.
            let text = unsafe { CStr::from_ptr(message) }
                .to_str()
                .unwrap()
                .to_owned();
            // SAFETY: it came from `ffi_platform_last_error_message`.
            unsafe { crate::ffi_free_string(message) };
            text
        });
        (ffi_platform_last_error_code(), text)
    }

    // --- the loop -----------------------------------------------------------

    #[test]
    fn a_headless_run_calls_back_in_order_and_returns_the_exit_code() {
        let mut probe = Probe::new(Box::new(|probe, ctx, name| match name {
            "resumed" => {
                let mut desc = default_desc();
                desc.title = c"Probe".as_ptr();
                desc.width = 800.0;
                desc.height = 600.0;
                probe.window = create_window(ctx, &desc);
            }
            "about_to_wait" => {
                // SAFETY: `ctx` belongs to the running callback.
                assert!(unsafe { ffi_platform_ctx_exit(ctx, 7) });
            }
            _ => {}
        }));
        let (ok, code) = run_headless(&mut probe);
        assert!(ok, "{:?}", last_error());
        assert_eq!(code, 7);
        assert_eq!(probe.window, 1);
        assert_eq!(probe.log.first().map(String::as_str), Some("resumed"));
        let wait = probe.log.iter().position(|l| l == "about_to_wait").unwrap();
        assert!(wait > 0, "{:?}", probe.log);
        // The window is destroyed when the loop ends.
        assert_eq!(
            probe.log.last().map(String::as_str),
            Some("window 1: destroyed")
        );
        assert_eq!(last_error().0, FFI_PLATFORM_ERROR_NONE);
    }

    #[test]
    fn window_state_can_be_changed_and_read_back() {
        let mut probe = Probe::new(Box::new(|probe, ctx, name| {
            if name == "resumed" {
                let mut desc = default_desc();
                desc.width = 640.0;
                desc.height = 480.0;
                desc.resizable = false;
                probe.window = create_window(ctx, &desc);
                let id = probe.window;
                let mut slot = std::mem::MaybeUninit::<PlatformWindowInfo>::uninit();
                // SAFETY: `ctx` belongs to the running callback; the pointers
                // are valid.
                unsafe {
                    assert!(ffi_platform_ctx_window_info(ctx, id, slot.as_mut_ptr()));
                    let info = slot.assume_init();
                    assert_eq!((info.inner_width, info.inner_height), (640, 480));
                    assert!((info.scale_factor - 1.0).abs() < f64::EPSILON);
                    assert!(info.visible && !info.ime_allowed && !info.focused);
                    assert_eq!(info.fullscreen, FFI_FULLSCREEN_NONE);
                    assert_eq!(info.cursor_grab, FFI_CURSOR_GRAB_NONE);

                    assert!(ffi_platform_ctx_set_title(ctx, id, c"Renamed".as_ptr()));
                    assert!(ffi_platform_ctx_set_inner_size(ctx, id, 320.0, 200.0));
                    assert!(ffi_platform_ctx_set_ime_allowed(ctx, id, true));
                    assert!(ffi_platform_ctx_set_ime_cursor_area(
                        ctx, id, 1.0, 2.0, 3.0, 4.0
                    ));
                    assert!(ffi_platform_ctx_set_cursor_visible(ctx, id, false));
                    assert!(ffi_platform_ctx_request_redraw(ctx, id));
                    assert!(ffi_platform_ctx_set_control_flow(
                        ctx,
                        FFI_CONTROL_FLOW_POLL,
                        0
                    ));
                    assert!(ffi_platform_ctx_set_control_flow(
                        ctx,
                        FFI_CONTROL_FLOW_WAIT_UNTIL,
                        5
                    ));
                    assert!(ffi_platform_ctx_set_control_flow(
                        ctx,
                        FFI_CONTROL_FLOW_WAIT,
                        0
                    ));

                    // A new size is a request: the window system reports it
                    // back with a `Resized` event, which comes later.
                    assert!(ffi_platform_ctx_window_info(ctx, id, slot.as_mut_ptr()));
                    assert!(slot.assume_init().ime_allowed);
                }
            }
            // The redraw requested in `resumed` is delivered after the first
            // `about_to_wait`; the loop then has nothing left to wake it, so
            // the redraw handler is where this application exits.
            if name.ends_with("redraw") {
                // SAFETY: `ctx` belongs to the running callback.
                unsafe { ffi_platform_ctx_exit(ctx, 0) };
            }
        }));
        let (ok, code) = run_headless(&mut probe);
        assert!(ok, "{:?} {:?}", last_error(), probe.log);
        assert_eq!(code, 0);
        assert!(
            probe.log.iter().any(|line| line.ends_with("redraw")),
            "{:?}",
            probe.log
        );
        assert!(
            probe
                .log
                .iter()
                .any(|line| line.ends_with("resized 320x200")),
            "{:?}",
            probe.log
        );
    }

    #[test]
    fn a_window_description_is_converted_field_by_field() {
        let mut desc = default_desc();
        desc.title = c"Editor".as_ptr();
        desc.width = 1600.0;
        desc.height = 900.0;
        desc.has_position = true;
        desc.x = 10.0;
        desc.y = 20.0;
        desc.has_min_size = true;
        desc.min_width = 320.0;
        desc.min_height = 200.0;
        desc.has_max_size = true;
        desc.max_width = 3000.0;
        desc.max_height = 2000.0;
        desc.resizable = false;
        desc.visible = false;
        desc.decorations = false;
        desc.fullscreen = FFI_FULLSCREEN_BORDERLESS;
        // SAFETY: the title is a valid static string.
        let converted = unsafe { window_desc(&desc) }.unwrap();
        assert_eq!(converted.title, "Editor");
        assert_eq!(converted.inner_size, LogicalSize::new(1600.0, 900.0));
        assert_eq!(converted.position, Some(LogicalPosition::new(10.0, 20.0)));
        assert_eq!(
            converted.min_inner_size,
            Some(LogicalSize::new(320.0, 200.0))
        );
        assert_eq!(
            converted.max_inner_size,
            Some(LogicalSize::new(3000.0, 2000.0))
        );
        assert!(!converted.resizable && !converted.visible && !converted.decorations);
        assert_eq!(converted.fullscreen, Fullscreen::Borderless);

        desc.fullscreen = 9;
        // SAFETY: as above.
        assert!(matches!(
            unsafe { window_desc(&desc) },
            Err(Failure::InvalidArgument(_))
        ));
    }

    #[test]
    fn the_default_description_matches_the_window_api_defaults() {
        let desc = default_desc();
        let defaults = WindowDesc::default();
        // SAFETY: the default title is a valid static string.
        let converted = unsafe { window_desc(&desc) }.unwrap();
        assert_eq!(converted.title, defaults.title);
        assert_eq!(converted.inner_size, defaults.inner_size);
        assert_eq!(converted.position, defaults.position);
        assert_eq!(converted.resizable, defaults.resizable);
        assert_eq!(converted.visible, defaults.visible);
        assert_eq!(converted.decorations, defaults.decorations);
        assert_eq!(converted.fullscreen, defaults.fullscreen);
    }

    #[test]
    fn an_invalid_description_or_window_is_reported_with_the_window_code() {
        let mut probe = Probe::new(Box::new(|_, ctx, name| {
            if name != "resumed" {
                return;
            }
            let mut desc = default_desc();
            desc.width = 0.0;
            let mut id = 0;
            // SAFETY: `ctx` belongs to the running callback; the pointers are
            // valid.
            unsafe {
                assert!(!ffi_platform_ctx_create_window(
                    ctx,
                    &raw const desc,
                    &raw mut id
                ));
                assert_eq!(ffi_platform_last_error_code(), FFI_PLATFORM_ERROR_WINDOW);
                assert!(!ffi_platform_ctx_close_window(ctx, 99));
                assert_eq!(ffi_platform_last_error_code(), FFI_PLATFORM_ERROR_WINDOW);
                let mut info = std::mem::MaybeUninit::<PlatformWindowInfo>::uninit();
                assert!(!ffi_platform_ctx_window_info(ctx, 99, info.as_mut_ptr()));
                assert!(!ffi_platform_ctx_set_fullscreen(ctx, 1, 9));
                assert_eq!(
                    ffi_platform_last_error_code(),
                    FFI_PLATFORM_ERROR_INVALID_ARGUMENT
                );
                assert!(!ffi_platform_ctx_set_cursor_grab(ctx, 1, 9));
                assert!(!ffi_platform_ctx_set_control_flow(ctx, 9, 0));
                assert!(!ffi_platform_ctx_set_title(ctx, 1, std::ptr::null()));
                assert!(ffi_platform_ctx_exit(ctx, 3));
            }
        }));
        let (ok, code) = run_headless(&mut probe);
        assert!(ok);
        assert_eq!(code, 3);
    }

    #[test]
    fn a_stale_or_null_context_is_refused_instead_of_used() {
        let mut probe = Probe::new(Box::new(|probe, ctx, name| match name {
            "resumed" => probe.saved_context = ctx,
            "about_to_wait" => {
                // SAFETY: `ctx` belongs to the running callback.
                unsafe { ffi_platform_ctx_exit(ctx, 0) };
            }
            _ => {}
        }));
        let (ok, _) = run_headless(&mut probe);
        assert!(ok);

        // SAFETY: the pointer is stale, which the functions detect without
        // dereferencing it.
        unsafe {
            assert!(!ffi_platform_ctx_exit(probe.saved_context, 5));
            let (code, message) = last_error();
            assert_eq!(code, FFI_PLATFORM_ERROR_INVALID_ARGUMENT);
            assert!(message.unwrap().contains("running"));
            assert!(!ffi_platform_ctx_exit(std::ptr::null_mut(), 5));
            assert_eq!(
                ffi_platform_last_error_code(),
                FFI_PLATFORM_ERROR_INVALID_ARGUMENT
            );
        }
    }

    #[test]
    fn a_loop_that_never_exits_stops_with_a_window_error() {
        let mut probe = Probe::new(Box::new(|_, _, _| {}));
        let (ok, code) = run_headless(&mut probe);
        assert!(!ok);
        assert_eq!(code, -1, "the exit code is not written on failure");
        assert_eq!(last_error().0, FFI_PLATFORM_ERROR_WINDOW);
    }

    #[test]
    fn missing_callbacks_are_skipped() {
        let callbacks = PlatformCallbacks {
            user_data: std::ptr::null_mut(),
            resumed: None,
            suspended: None,
            window_event: None,
            about_to_wait: None,
        };
        let config = headless_config();
        let mut code = 0;
        // SAFETY: all pointers are valid.
        let ok =
            unsafe { ffi_platform_run(&raw const callbacks, &raw const config, &raw mut code) };
        assert!(
            !ok,
            "nothing exits the loop, so the headless backend stalls"
        );
        assert_eq!(last_error().0, FFI_PLATFORM_ERROR_WINDOW);
    }

    #[test]
    fn bad_run_arguments_are_refused() {
        let mut probe = Probe::new(Box::new(|_, _, _| {}));
        let callbacks = probe.callbacks();
        let mut code = 0;
        let mut config = headless_config();
        // SAFETY: the null pointers are the case under test; the rest is valid.
        unsafe {
            assert!(!ffi_platform_run(
                std::ptr::null(),
                &raw const config,
                &raw mut code
            ));
            assert_eq!(
                ffi_platform_last_error_code(),
                FFI_PLATFORM_ERROR_INVALID_ARGUMENT
            );
            assert!(!ffi_platform_run(
                &raw const callbacks,
                &raw const config,
                std::ptr::null_mut()
            ));
            config.backend = 42;
            assert!(!ffi_platform_run(
                &raw const callbacks,
                &raw const config,
                &raw mut code
            ));
        }
        let (code, message) = last_error();
        assert_eq!(code, FFI_PLATFORM_ERROR_WINDOW);
        assert!(message.unwrap().contains("42"));
    }

    // --- events from the loop, end to end -----------------------------------

    #[test]
    fn host_events_arrive_converted_through_the_callback_table() {
        let mut probe = Probe::new(Box::new(|probe, ctx, name| {
            if name == "resumed" {
                probe.window = create_window(ctx, &default_desc());
            }
            if name == "about_to_wait" && probe.log.iter().any(|l| l.contains("cursor 3.5 4.5")) {
                // SAFETY: `ctx` belongs to the running callback.
                unsafe { ffi_platform_ctx_exit(ctx, 0) };
            }
        }));
        let callbacks = probe.callbacks();
        let mut backend = HeadlessBackend::new(HeadlessConfig::default());
        let window = WindowId::from_u64(1);
        let key =
            KeyEvent::new(KeyCode::KeyW, Key::character("w"), ButtonState::Pressed).with_text("w");
        for event in [
            HostEvent::Resized {
                window,
                size: PhysicalSize::new(100, 50),
            },
            HostEvent::Focused {
                window,
                focused: true,
            },
            HostEvent::Input {
                window,
                event: InputEvent::Key(key),
            },
            HostEvent::Input {
                window,
                event: InputEvent::CursorMoved(PointerPosition::new(3.5, 4.5)),
            },
        ] {
            backend.push(event);
        }
        let result = run_backend(backend, callbacks);
        assert_eq!(result.unwrap_or(-1), 0, "{:?}", probe.log);
        let lines: Vec<&str> = probe.log.iter().map(String::as_str).collect();
        let position = |needle: &str| {
            lines
                .iter()
                .position(|l| l.contains(needle))
                .unwrap_or_else(|| panic!("{needle} missing in {lines:?}"))
        };
        assert!(position("resized 100x50") < position("focused true"));
        assert!(position("focused true") < position("key phys="));
        assert!(position("key phys=") < position("cursor 3.5 4.5"));
        assert!(lines.iter().any(|l| l.contains("window 1:")));
    }

    // --- conversions --------------------------------------------------------

    #[test]
    fn plain_window_events_convert() {
        assert_eq!(convert(&WindowEvent::CloseRequested), "close-requested");
        assert_eq!(convert(&WindowEvent::Destroyed), "destroyed");
        assert_eq!(
            convert(&WindowEvent::Resized(PhysicalSize::new(640, 480))),
            "resized 640x480"
        );
        assert_eq!(
            convert(&WindowEvent::Moved(PhysicalPosition::new(-5, 7))),
            "moved -5,7"
        );
        assert_eq!(
            convert(&WindowEvent::ScaleFactorChanged {
                scale_factor: 1.5,
                inner_size: PhysicalSize::new(960, 540)
            }),
            "scale 1.5 960x540"
        );
        assert_eq!(convert(&WindowEvent::Focused(true)), "focused true");
        assert_eq!(convert(&WindowEvent::Occluded(false)), "occluded false");
        assert_eq!(convert(&WindowEvent::RedrawRequested), "redraw");
        assert_eq!(convert(&WindowEvent::SurfaceLost), "surface-lost");
    }

    fn nz(value: usize) -> NonZeroUsize {
        NonZeroUsize::new(value).unwrap()
    }

    #[test]
    fn every_kind_of_native_handle_keeps_its_pointers() {
        let handles = [
            NativeWindowHandle::Win32 {
                hwnd: nz(0x10),
                hinstance: Some(nz(0x20)),
            },
            NativeWindowHandle::Win32 {
                hwnd: nz(0x10),
                hinstance: None,
            },
            NativeWindowHandle::Cocoa {
                metal_layer: nz(0x30),
            },
            NativeWindowHandle::X11 {
                display: Some(nz(0x40)),
                window: NonZeroU64::new(0x1234_5678_9abc).unwrap(),
            },
            NativeWindowHandle::X11 {
                display: None,
                window: NonZeroU64::new(5).unwrap(),
            },
            NativeWindowHandle::Wayland {
                display: nz(0x50),
                surface: nz(0x60),
            },
            NativeWindowHandle::Android {
                native_window: nz(0x70),
            },
            NativeWindowHandle::UiKit {
                metal_layer: nz(0x80),
            },
        ];
        let described: Vec<String> = handles
            .iter()
            .map(|h| convert(&WindowEvent::SurfaceCreated(*h)))
            .collect();
        let nothing = "0x0";
        assert_eq!(
            described[0],
            format!(
                "surface kind=0 hwnd=0x10 hinst=0x20 metal={nothing} xdpy={nothing} xwin=0 wl={nothing}/{nothing} android={nothing}"
            )
        );
        assert!(described[1].contains("hinst=0x0"));
        assert!(described[2].starts_with("surface kind=1 ") && described[2].contains("metal=0x30"));
        assert!(
            described[3].contains("kind=2")
                && described[3].contains("xdpy=0x40")
                && described[3].contains(&format!("xwin={}", 0x1234_5678_9abc_u64))
        );
        assert!(described[4].contains("xdpy=0x0") && described[4].contains("xwin=5"));
        assert!(described[5].contains("kind=3") && described[5].contains("wl=0x50/0x60"));
        assert!(described[6].contains("kind=4") && described[6].contains("android=0x70"));
        assert!(described[7].contains("kind=5") && described[7].contains("metal=0x80"));
    }

    #[test]
    fn key_events_carry_both_identities_and_their_text() {
        let a = KeyEvent::new(KeyCode::KeyA, Key::character("A"), ButtonState::Pressed)
            .with_text("A")
            .with_repeat(true);
        let line = input(InputEvent::Key(a));
        assert_eq!(
            line,
            format!(
                "key phys={} logical={FFI_LOGICAL_CHARACTER}/0/A text=A state={FFI_BUTTON_PRESSED} repeat=true synthetic=false",
                keys::FFI_KEY_KEY_A
            )
        );

        let enter = KeyEvent::new(
            KeyCode::Enter,
            Key::Named(NamedKey::Enter),
            ButtonState::Released,
        )
            .with_synthetic(true);
        assert_eq!(
            input(InputEvent::Key(enter)),
            format!(
                "key phys={} logical={FFI_LOGICAL_NAMED}/{}/null text=null state={FFI_BUTTON_RELEASED} repeat=false synthetic=true",
                keys::FFI_KEY_ENTER,
                keys::FFI_NAMED_KEY_ENTER
            )
        );

        let dead = KeyEvent::new(
            KeyCode::Backquote,
            Key::Dead(Some('`')),
            ButtonState::Pressed,
        );
        assert!(input(InputEvent::Key(dead)).contains(&format!("logical={FFI_LOGICAL_DEAD}/0/`")));
        let unknown = KeyEvent::new(
            KeyCode::Unidentified,
            Key::Unidentified,
            ButtonState::Pressed,
        );
        let line = input(InputEvent::Key(unknown));
        assert!(
            line.contains("phys=0 ")
                && line.contains(&format!("logical={FFI_LOGICAL_UNIDENTIFIED}"))
        );
    }

    #[test]
    fn text_with_an_interior_nul_loses_the_nul_instead_of_the_text() {
        let event = KeyEvent::new(KeyCode::KeyA, Key::character("a"), ButtonState::Pressed)
            .with_text("a\0b");
        assert!(input(InputEvent::Key(event)).contains("text=ab "));
    }

    #[test]
    fn mouse_wheel_cursor_and_raw_motion_events_convert() {
        assert_eq!(
            input(InputEvent::ModifiersChanged(
                Modifiers::SHIFT | Modifiers::SUPER
            )),
            format!("modifiers {}", FFI_MOD_SHIFT | FFI_MOD_SUPER)
        );
        assert_eq!(
            input(InputEvent::CursorMoved(PointerPosition::new(1.5, -2.0))),
            "cursor 1.5 -2"
        );
        assert_eq!(input(InputEvent::CursorEntered), "cursor-entered");
        assert_eq!(input(InputEvent::CursorLeft), "cursor-left");
        assert_eq!(
            input(InputEvent::MouseButton {
                button: MouseButton::Back,
                state: ButtonState::Pressed
            }),
            format!("mouse {FFI_MOUSE_BACK} other=0 state={FFI_BUTTON_PRESSED}")
        );
        assert_eq!(
            input(InputEvent::MouseButton {
                button: MouseButton::Other(9),
                state: ButtonState::Released
            }),
            format!("mouse {FFI_MOUSE_OTHER} other=9 state={FFI_BUTTON_RELEASED}")
        );
        assert_eq!(
            input(InputEvent::MouseWheel(ScrollDelta::Lines {
                x: 0.0,
                y: 1.0
            })),
            format!("scroll unit={FFI_SCROLL_LINES} 0 1")
        );
        assert_eq!(
            input(InputEvent::MouseWheel(ScrollDelta::Pixels {
                x: 3.0,
                y: -4.0
            })),
            format!("scroll unit={FFI_SCROLL_PIXELS} 3 -4")
        );
        assert_eq!(
            input(InputEvent::RawMouseMotion {
                delta_x: 2.0,
                delta_y: -1.0
            }),
            "raw 2 -1"
        );
    }

    #[test]
    fn touch_and_ime_events_convert() {
        assert_eq!(
            input(InputEvent::Touch(TouchEvent::new(
                7,
                platform_crate::input::TouchPhase::Cancelled,
                PointerPosition::new(9.0, 8.0)
            ))),
            format!("touch id=7 phase={FFI_TOUCH_CANCELLED} 9 8")
        );
        assert_eq!(
            input(InputEvent::Ime(ImeEvent::Enabled)),
            format!("ime kind={FFI_IME_ENABLED} text=null cursor=false:0:0")
        );
        assert_eq!(
            input(InputEvent::Ime(ImeEvent::Preedit {
                text: "にほ".to_owned(),
                cursor: Some((0, 6))
            })),
            format!("ime kind={FFI_IME_PREEDIT} text=にほ cursor=true:0:6")
        );
        assert_eq!(
            input(InputEvent::Ime(ImeEvent::Commit("日本".to_owned()))),
            format!("ime kind={FFI_IME_COMMIT} text=日本 cursor=false:0:0")
        );
        assert_eq!(
            input(InputEvent::Ime(ImeEvent::Disabled)),
            format!("ime kind={FFI_IME_DISABLED} text=null cursor=false:0:0")
        );
    }

    #[test]
    fn the_c_layout_is_what_the_header_promises() {
        // Sizes the C++ side relies on when it reads these structs; cbindgen
        // writes the same layout into the header, this only guards against an
        // accidental change of a field type.
        assert_eq!(std::mem::size_of::<PlatformSize>(), 8);
        assert_eq!(std::mem::size_of::<PlatformPosition>(), 8);
        assert_eq!(std::mem::size_of::<PlatformPoint>(), 16);
        assert_eq!(std::mem::align_of::<PlatformEvent>(), 8);
        assert_eq!(std::mem::size_of::<PlatformNativeHandle>(), 72);
        assert_eq!(std::mem::offset_of!(PlatformEvent, kind), 0);
    }
}
