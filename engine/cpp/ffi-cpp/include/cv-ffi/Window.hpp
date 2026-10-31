#pragma once

#include <chrono>
#include <cstdint>
#include <expected>
#include <optional>
#include <string>
#include <string_view>
#include <utility>
#include <variant>

#include "cv-ffi/Keys.hpp"
#include "cv-ffi/Platform.hpp"

/// The window event loop of the Rust `platform` crate for C++ (wraps
/// `ffi_platform_run` and the `ffi_platform_ctx_*` functions of
/// engine/rust/crates/ffi/src/platform_run.rs).
///
/// An application derives from `App`, overrides the callbacks it needs and
/// hands itself to `Run()`, which returns when the application exits:
///
/// ```cpp
/// class Game : public cv_ffi::window::App
/// {
///     void Resumed(cv_ffi::window::Context& ctx) override
///     {
///         cv_ffi::window::WindowDesc desc;
///         desc.title = "Game";
///         m_window = ctx.CreateWindow(desc).value();
///         (void)ctx.SetControlFlow(cv_ffi::window::ControlFlow::Poll);
///     }
///
///     void OnEvent(cv_ffi::window::Context& ctx, cv_ffi::window::WindowId, const cv_ffi::window::Event& event) override
///     {
///         if (std::holds_alternative<cv_ffi::window::CloseRequested>(event)) {
///             (void)ctx.Exit(0);
///         }
///     }
///     // ...
/// };
///
/// Game game;
/// auto code = cv_ffi::window::Run(game); // std::expected<int, std::string>
/// ```
///
/// Like the rest of cv-ffi, this header never includes `ffi.h`; the raw C ABI
/// stays inside the .cpp files.
namespace cv_ffi::window {

/// Identifies a window. Plain numbers, so they may be kept; the `Context` may
/// not.
using WindowId = std::uint64_t;

/// Whether a key or button went down or up (`FFI_BUTTON_*`).
enum class ButtonState : std::uint8_t {
    Pressed = 0,
    Released = 1,
};

/// A mouse button (`FFI_MOUSE_*`). `Other` comes with its number in
/// `MouseButtonInput::other`.
enum class MouseButton : std::uint8_t {
    Left = 0,
    Right = 1,
    Middle = 2,
    Back = 3,
    Forward = 4,
    Other = 5,
};

/// The unit of a scroll amount (`FFI_SCROLL_*`).
enum class ScrollUnit : std::uint8_t {
    Lines = 0,  ///< Notched wheels.
    Pixels = 1, ///< Touchpads.
};

/// Where a touch is in its life (`FFI_TOUCH_*`).
enum class TouchPhase : std::uint8_t {
    Started = 0,
    Moved = 1,
    Ended = 2,
    Cancelled = 3, ///< The system took the touch away.
};

/// What an input method reports (`FFI_IME_*`).
enum class ImeKind : std::uint8_t {
    Enabled = 0,
    Preedit = 1, ///< The text being composed changed; empty text clears it.
    Commit = 2,  ///< Composition finished: insert the text.
    Disabled = 3,
    Unknown = 255,
};

/// The modifier keys that are held, a bit mask (`FFI_MOD_*`). Combine with `|`,
/// test with `Has()`.
enum class Modifiers : std::uint8_t {
    None = 0,
    Shift = 1,
    Control = 2,
    Alt = 4,   ///< Option on macOS.
    Super = 8, ///< The Windows key, or Command on macOS.
};

[[nodiscard]] constexpr Modifiers operator|(Modifiers a, Modifiers b) noexcept
{
    return static_cast<Modifiers>(static_cast<std::uint8_t>(a) | static_cast<std::uint8_t>(b));
}

[[nodiscard]] constexpr Modifiers operator&(Modifiers a, Modifiers b) noexcept
{
    return static_cast<Modifiers>(static_cast<std::uint8_t>(a) & static_cast<std::uint8_t>(b));
}

/// Whether every modifier in `flag` is in `set`.
[[nodiscard]] constexpr bool Has(Modifiers set, Modifiers flag) noexcept
{
    return (set & flag) == flag;
}

/// Whether a window covers its monitor (`FFI_FULLSCREEN_*`).
enum class Fullscreen : std::uint8_t {
    None = 0,
    Borderless = 1, ///< The size of the monitor, no video mode change.
};

/// How a window holds on to the mouse cursor (`FFI_CURSOR_GRAB_*`).
enum class CursorGrab : std::uint8_t {
    None = 0,
    Confined = 1, ///< The cursor cannot leave the window.
    Locked = 2,   ///< The cursor is held in place; use RawMouseMotion.
};

/// How the loop waits between iterations (`FFI_CONTROL_FLOW_*`).
enum class ControlFlow : std::uint8_t {
    Poll = 0,      ///< Never wait: a game that draws continuously.
    Wait = 1,      ///< Sleep until the next event: a tool.
    WaitUntil = 2, ///< Sleep until the next event or until the timeout.
};

/// Which event loop `Run()` uses (`FFI_BACKEND_*`).
enum class Backend : std::uint8_t {
    Desktop = 0,  ///< Real windows (Windows, macOS, Linux).
    Headless = 1, ///< No display: for tests and servers. See `Run()`.
};

/// What a graphics API needs to create a presentation surface. One group of
/// fields is filled according to `kind`, the others stay null/zero. The field
/// names and types are those of the renderer's `renderer::NativeWindowHandle`,
/// so a handle converts to it field by field.
///
/// The pointers are valid from the `SurfaceCreated` event that delivered the
/// handle until the matching `SurfaceLost` has been handled.
struct NativeWindowHandle {
    platform::DisplayServer kind = platform::DisplayServer::Unknown;
    void* hwnd = nullptr;            ///< Windows: HWND.
    void* hinstance = nullptr;       ///< Windows: HINSTANCE, null when unknown.
    void* metalLayer = nullptr;      ///< macOS, iOS: CAMetalLayer*.
    void* xlibDisplay = nullptr;     ///< X11: Display*, null when unknown.
    std::uint64_t xlibWindow = 0;    ///< X11: Window.
    void* waylandDisplay = nullptr;  ///< Wayland: wl_display*.
    void* waylandSurface = nullptr;  ///< Wayland: wl_surface*.
    void* nativeWindow = nullptr;    ///< Android: ANativeWindow*.
};

// --- window events ----------------------------------------------------------

/// The user asked the window to close. Nothing is closed yet: call
/// `Context::CloseWindow()` or `Context::Exit()`, or ignore it.
struct CloseRequested {};
/// The window has been closed.
struct Destroyed {};
/// The client area changed size (physical pixels).
struct Resized {
    std::uint32_t width = 0;
    std::uint32_t height = 0;
};
/// The window moved (physical pixels, top-left corner).
struct Moved {
    std::int32_t x = 0;
    std::int32_t y = 0;
};
/// The scale factor changed (for example the window moved to another monitor).
/// `width` x `height` is the new client-area size in physical pixels.
struct ScaleFactorChanged {
    double scaleFactor = 1.0;
    std::uint32_t width = 0;
    std::uint32_t height = 0;
};
/// The window gained or lost keyboard focus.
struct Focused {
    bool focused = false;
};
/// The window became fully hidden (`true`) or visible again.
struct Occluded {
    bool occluded = false;
};
/// Draw a frame now (after `Context::RequestRedraw()`).
struct RedrawRequested {};
/// The window has a presentation surface: build the swapchain from `handle`.
struct SurfaceCreated {
    NativeWindowHandle handle;
};
/// The surface is gone: destroy everything built from the last
/// `SurfaceCreated` handle before returning from the handler.
struct SurfaceLost {};

// --- input events -----------------------------------------------------------

/// What a key means under the active layout and modifiers.
struct LogicalKey {
    enum class Kind : std::uint8_t {
        Named,        ///< `named` is set.
        Character,    ///< `text` is the typed text.
        Dead,         ///< An accent waiting for a letter; `text` is the accent, maybe empty.
        Unidentified, ///< The platform could not tell.
    };

    Kind kind = Kind::Unidentified;
    NamedKey named = NamedKey::Unknown;
    std::string text;
};

/// A key went down or up.
struct KeyInput {
    /// Where the key is on the keyboard, independent of the layout: for controls.
    KeyCode physical = KeyCode::Unidentified;
    /// What the key means under the layout: for shortcuts and text.
    LogicalKey logical;
    /// The text the press produces, if any. Enter, Tab and Backspace report their
    /// control characters here; filter with `std::iscntrl` if you want printable
    /// text only.
    std::optional<std::string> text;
    ButtonState state = ButtonState::Pressed;
    /// The auto-repeat of a held key.
    bool repeat = false;
    /// Made up by the system (the release of a key held while the window lost focus).
    bool synthetic = false;
};
/// The set of held modifier keys changed.
struct ModifiersChanged {
    Modifiers modifiers = Modifiers::None;
};
/// The cursor moved inside the window (physical pixels).
struct CursorMoved {
    double x = 0.0;
    double y = 0.0;
};
struct CursorEntered {};
struct CursorLeft {};
/// A mouse button went down or up.
struct MouseButtonInput {
    MouseButton button = MouseButton::Left;
    /// The button number when `button` is `Other`.
    std::uint16_t other = 0;
    ButtonState state = ButtonState::Pressed;
};
/// The wheel or touchpad scrolled. Positive values mean the scrolled content moves
/// right and down: rolling a wheel away from you gives a positive `y`.
struct MouseWheel {
    ScrollUnit unit = ScrollUnit::Lines;
    double x = 0.0;
    double y = 0.0;
};
/// Raw, unaccelerated mouse movement for camera control; it keeps coming while the
/// cursor is locked.
struct RawMouseMotion {
    double dx = 0.0;
    double dy = 0.0;
};
/// A finger touched, moved on or left the screen.
struct TouchInput {
    std::uint64_t id = 0;
    TouchPhase phase = TouchPhase::Started;
    double x = 0.0;
    double y = 0.0;
};
/// An input method composed text (only after `Context::SetImeAllowed(true)`).
struct ImeInput {
    ImeKind kind = ImeKind::Unknown;
    /// The composed or committed text; empty for `Enabled` and `Disabled`.
    std::string text;
    /// The caret or selection inside a preedit text as byte offsets.
    std::optional<std::pair<std::uint64_t, std::uint64_t>> cursor;
};
/// An event this library does not know: ignore it.
struct UnknownEvent {};

/// Everything `App::OnEvent` can be called with.
using Event = std::variant<
    CloseRequested, Destroyed, Resized, Moved, ScaleFactorChanged, Focused, Occluded, RedrawRequested, SurfaceCreated,
    SurfaceLost, KeyInput, ModifiersChanged, CursorMoved, CursorEntered, CursorLeft, MouseButtonInput, MouseWheel,
    RawMouseMotion, TouchInput, ImeInput, UnknownEvent>;

// --- windows ----------------------------------------------------------------

/// A window to create. Sizes and positions are logical units (independent of the
/// monitor's scale factor). The defaults are those of the Rust `WindowDesc`; a test
/// keeps them equal.
struct WindowDesc {
    std::string title = "Coreverse";
    double width = 1280.0;
    double height = 720.0;
    /// The top-left corner; unset lets the window system choose.
    std::optional<std::pair<double, double>> position;
    std::optional<std::pair<double, double>> minSize;
    std::optional<std::pair<double, double>> maxSize;
    bool resizable = true;
    bool visible = true;
    bool decorations = true;
    Fullscreen fullscreen = Fullscreen::None;
};

/// The state of a window at this moment.
struct WindowInfo {
    /// Client area in physical pixels.
    std::uint32_t innerWidth = 0;
    std::uint32_t innerHeight = 0;
    /// Physical pixels per logical unit.
    double scaleFactor = 1.0;
    bool focused = false;
    bool occluded = false;
    bool visible = true;
    Fullscreen fullscreen = Fullscreen::None;
    CursorGrab cursorGrab = CursorGrab::None;
    bool imeAllowed = false;
};

/// The application's view of the event loop. Only valid inside the callback it was
/// passed to: do not store it, and do not use it from another thread. A call made
/// while no callback is running fails with an error message instead of crashing
/// (inside a later callback a kept pointer happens to work, since the loop lends
/// the same object to every callback; do not rely on it). Keep window ids, not the
/// context.
///
/// Every function returns the message of the Rust error on failure (an unknown
/// window, an unusable description, ...).
class Context
{
public:
    /// Wraps the raw context of a callback. Only `Run()` makes these.
    explicit Context(void* raw) noexcept : m_raw(raw) {}

    Context(const Context&) = delete;
    Context& operator=(const Context&) = delete;

    /// Creates a window. It exists at once (see `GetWindowInfo()`); on the desktop its
    /// surface follows as a `SurfaceCreated` event.
    [[nodiscard]] std::expected<WindowId, std::string> CreateWindow(const WindowDesc& desc);
    /// Closes a window; `SurfaceLost` (if it had a surface) and `Destroyed` follow.
    [[nodiscard]] std::expected<void, std::string> CloseWindow(WindowId window);
    [[nodiscard]] std::expected<void, std::string> SetTitle(WindowId window, std::string_view title);
    /// Resizes the client area (logical units). The new size is reported back as `Resized`.
    [[nodiscard]] std::expected<void, std::string> SetInnerSize(WindowId window, double width, double height);
    [[nodiscard]] std::expected<void, std::string> SetFullscreen(WindowId window, Fullscreen fullscreen);
    /// Asks for a `RedrawRequested` event for the window in the next loop iteration.
    [[nodiscard]] std::expected<void, std::string> RequestRedraw(WindowId window);
    [[nodiscard]] std::expected<void, std::string> SetCursorVisible(WindowId window, bool visible);
    /// Confines or locks the cursor. A system that cannot do it does not fail here;
    /// read the result back with `GetWindowInfo()`.
    [[nodiscard]] std::expected<void, std::string> SetCursorGrab(WindowId window, CursorGrab grab);
    /// Allows input-method events: switch it on while a text field has focus.
    [[nodiscard]] std::expected<void, std::string> SetImeAllowed(WindowId window, bool allowed);
    /// Tells the input method where the text caret is (logical units).
    [[nodiscard]] std::expected<void, std::string>
    SetImeCursorArea(WindowId window, double x, double y, double width, double height);
    /// Chooses how the loop waits. `wait` is only used with `WaitUntil`.
    [[nodiscard]] std::expected<void, std::string>
    SetControlFlow(ControlFlow flow, std::chrono::milliseconds wait = std::chrono::milliseconds{0});
    /// Ends the loop after the current callback; `Run()` returns `exitCode`.
    [[nodiscard]] std::expected<void, std::string> Exit(int exitCode = 0);
    [[nodiscard]] std::expected<WindowInfo, std::string> GetWindowInfo(WindowId window) const;

private:
    void* m_raw;
};

/// What the application does when something happens. Override what you need; every
/// callback runs on the thread that called `Run()`.
///
/// A callback must not let an exception escape into the event loop: `Run()` catches
/// it, stops the loop and returns it as an error, but the callbacks after the throwing one are skipped.
class App
{
public:
    virtual ~App() = default;

    /// The application may show windows: create them here. Called once at the start on
    /// the desktop; on Android and iOS also after every return from the background.
    virtual void Resumed(Context& ctx);
    /// The application goes to the background (mobile only): every surface is lost right before.
    virtual void Suspended(Context& ctx);
    /// A window event. The event is only valid during the call, but a copy may be kept.
    virtual void OnEvent(Context& ctx, WindowId window, const Event& event);
    /// All pending events were handled and the loop is about to wait: update the game,
    /// request a redraw.
    virtual void AboutToWait(Context& ctx);
};

/// How `Run()` runs.
struct RunConfig {
    Backend backend = Backend::Desktop;
    /// Headless only: the scale factor windows start with; 0 for the default.
    double headlessScaleFactor = 0.0;
    /// Headless only: the virtual monitor in pixels; 0 for the default (set both).
    std::uint32_t headlessMonitorWidth = 0;
    std::uint32_t headlessMonitorHeight = 0;
};

/// Runs `app`'s event loop on the calling thread until it exits and returns the exit
/// code it asked for (`Context::Exit`; 0 for a plain exit). Call it on the main thread
/// (required on macOS), and not from inside one of `app`'s own callbacks.
///
/// Fails with the message of the Rust error when the window system cannot start, when a
/// callback throws, or (headless only) when the application never exits: the headless
/// backend reports it instead of waiting for events that can never come.
[[nodiscard]] std::expected<int, std::string> Run(App& app, const RunConfig& config = {});

} // namespace cv_ffi::window
