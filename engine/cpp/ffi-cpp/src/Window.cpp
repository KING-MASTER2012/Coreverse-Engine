#include "cv-ffi/Window.hpp"

#include <exception>

#include "ffi.h"

namespace cv_ffi::window {

namespace {

/// `ffi_platform_*` calls report failure via `false` plus the thread-local last
/// error of ffi::platform (see Platform.cpp). Read it right after the failing call.
std::string LastError()
{
    char* message = ffi_platform_last_error_message();
    if (message == nullptr) {
        return "platform: no error recorded (ffi_platform_last_error_message returned null)";
    }
    std::string result(message);
    ffi_free_string(message);
    return result;
}

std::expected<void, std::string> Check(bool ok)
{
    if (ok) {
        return {};
    }
    return std::unexpected(LastError());
}

::PlatformContext* RawContext(void* raw) noexcept
{
    return static_cast<::PlatformContext*>(raw);
}

// The enums travel as raw bytes, so they must match the header.
static_assert(static_cast<std::uint8_t>(ButtonState::Pressed) == FFI_BUTTON_PRESSED);
static_assert(static_cast<std::uint8_t>(ButtonState::Released) == FFI_BUTTON_RELEASED);
static_assert(static_cast<std::uint8_t>(MouseButton::Left) == FFI_MOUSE_LEFT);
static_assert(static_cast<std::uint8_t>(MouseButton::Right) == FFI_MOUSE_RIGHT);
static_assert(static_cast<std::uint8_t>(MouseButton::Middle) == FFI_MOUSE_MIDDLE);
static_assert(static_cast<std::uint8_t>(MouseButton::Back) == FFI_MOUSE_BACK);
static_assert(static_cast<std::uint8_t>(MouseButton::Forward) == FFI_MOUSE_FORWARD);
static_assert(static_cast<std::uint8_t>(MouseButton::Other) == FFI_MOUSE_OTHER);
static_assert(static_cast<std::uint8_t>(ScrollUnit::Lines) == FFI_SCROLL_LINES);
static_assert(static_cast<std::uint8_t>(ScrollUnit::Pixels) == FFI_SCROLL_PIXELS);
static_assert(static_cast<std::uint8_t>(TouchPhase::Started) == FFI_TOUCH_STARTED);
static_assert(static_cast<std::uint8_t>(TouchPhase::Moved) == FFI_TOUCH_MOVED);
static_assert(static_cast<std::uint8_t>(TouchPhase::Ended) == FFI_TOUCH_ENDED);
static_assert(static_cast<std::uint8_t>(TouchPhase::Cancelled) == FFI_TOUCH_CANCELLED);
static_assert(static_cast<std::uint8_t>(ImeKind::Enabled) == FFI_IME_ENABLED);
static_assert(static_cast<std::uint8_t>(ImeKind::Preedit) == FFI_IME_PREEDIT);
static_assert(static_cast<std::uint8_t>(ImeKind::Commit) == FFI_IME_COMMIT);
static_assert(static_cast<std::uint8_t>(ImeKind::Disabled) == FFI_IME_DISABLED);
static_assert(static_cast<std::uint8_t>(Modifiers::Shift) == FFI_MOD_SHIFT);
static_assert(static_cast<std::uint8_t>(Modifiers::Control) == FFI_MOD_CONTROL);
static_assert(static_cast<std::uint8_t>(Modifiers::Alt) == FFI_MOD_ALT);
static_assert(static_cast<std::uint8_t>(Modifiers::Super) == FFI_MOD_SUPER);
static_assert(static_cast<std::uint8_t>(Fullscreen::None) == FFI_FULLSCREEN_NONE);
static_assert(static_cast<std::uint8_t>(Fullscreen::Borderless) == FFI_FULLSCREEN_BORDERLESS);
static_assert(static_cast<std::uint8_t>(CursorGrab::None) == FFI_CURSOR_GRAB_NONE);
static_assert(static_cast<std::uint8_t>(CursorGrab::Confined) == FFI_CURSOR_GRAB_CONFINED);
static_assert(static_cast<std::uint8_t>(CursorGrab::Locked) == FFI_CURSOR_GRAB_LOCKED);
static_assert(static_cast<std::uint8_t>(ControlFlow::Poll) == FFI_CONTROL_FLOW_POLL);
static_assert(static_cast<std::uint8_t>(ControlFlow::Wait) == FFI_CONTROL_FLOW_WAIT);
static_assert(static_cast<std::uint8_t>(ControlFlow::WaitUntil) == FFI_CONTROL_FLOW_WAIT_UNTIL);
static_assert(static_cast<std::uint8_t>(Backend::Desktop) == FFI_BACKEND_DESKTOP);
static_assert(static_cast<std::uint8_t>(Backend::Headless) == FFI_BACKEND_HEADLESS);

std::string Text(const char* text)
{
    return text == nullptr ? std::string{} : std::string(text);
}

platform::DisplayServer ToDisplayServer(std::uint8_t value) noexcept
{
    switch (value) {
        case FFI_DISPLAY_WIN32:
            return platform::DisplayServer::Win32;
        case FFI_DISPLAY_COCOA:
            return platform::DisplayServer::Cocoa;
        case FFI_DISPLAY_X11:
            return platform::DisplayServer::X11;
        case FFI_DISPLAY_WAYLAND:
            return platform::DisplayServer::Wayland;
        case FFI_DISPLAY_ANDROID:
            return platform::DisplayServer::Android;
        case FFI_DISPLAY_UIKIT:
            return platform::DisplayServer::UIKit;
        default:
            return platform::DisplayServer::Unknown;
    }
}

NativeWindowHandle ToHandle(const ::PlatformNativeHandle& raw) noexcept
{
    NativeWindowHandle handle;
    handle.kind = ToDisplayServer(raw.kind);
    handle.hwnd = raw.hwnd;
    handle.hinstance = raw.hinstance;
    handle.metalLayer = raw.metal_layer;
    handle.xlibDisplay = raw.xlib_display;
    handle.xlibWindow = raw.xlib_window;
    handle.waylandDisplay = raw.wayland_display;
    handle.waylandSurface = raw.wayland_surface;
    handle.nativeWindow = raw.native_window;
    return handle;
}

ButtonState ToButtonState(std::uint8_t value) noexcept
{
    return value == FFI_BUTTON_RELEASED ? ButtonState::Released : ButtonState::Pressed;
}

Event ToKeyEvent(const ::PlatformKeyEvent& raw)
{
    KeyInput key;
    key.physical = static_cast<KeyCode>(raw.physical_key);
    switch (raw.logical_kind) {
        case FFI_LOGICAL_NAMED:
            key.logical.kind = LogicalKey::Kind::Named;
            key.logical.named = static_cast<NamedKey>(raw.logical_named);
            break;
        case FFI_LOGICAL_CHARACTER:
            key.logical.kind = LogicalKey::Kind::Character;
            key.logical.text = Text(raw.logical_text);
            break;
        case FFI_LOGICAL_DEAD:
            key.logical.kind = LogicalKey::Kind::Dead;
            key.logical.text = Text(raw.logical_text);
            break;
        default:
            key.logical.kind = LogicalKey::Kind::Unidentified;
            break;
    }
    if (raw.text != nullptr) {
        key.text = std::string(raw.text);
    }
    key.state = ToButtonState(raw.state);
    key.repeat = raw.repeat;
    key.synthetic = raw.synthetic;
    return key;
}

Event ToInputEvent(const ::PlatformInputEvent& raw)
{
    switch (raw.kind) {
        case FFI_INPUT_KEY:
            return ToKeyEvent(raw.data.key);
        case FFI_INPUT_MODIFIERS_CHANGED:
            return ModifiersChanged{static_cast<Modifiers>(raw.data.modifiers)};
        case FFI_INPUT_CURSOR_MOVED:
            return CursorMoved{raw.data.point.x, raw.data.point.y};
        case FFI_INPUT_CURSOR_ENTERED:
            return CursorEntered{};
        case FFI_INPUT_CURSOR_LEFT:
            return CursorLeft{};
        case FFI_INPUT_MOUSE_BUTTON: {
            const ::PlatformMouseButtonEvent& button = raw.data.mouse_button;
            return MouseButtonInput{static_cast<MouseButton>(button.button), button.other, ToButtonState(button.state)};
        }
        case FFI_INPUT_MOUSE_WHEEL: {
            const ::PlatformScrollEvent& scroll = raw.data.scroll;
            return MouseWheel{
                scroll.unit == FFI_SCROLL_PIXELS ? ScrollUnit::Pixels : ScrollUnit::Lines, scroll.x, scroll.y
            };
        }
        case FFI_INPUT_RAW_MOUSE_MOTION:
            return RawMouseMotion{raw.data.point.x, raw.data.point.y};
        case FFI_INPUT_TOUCH: {
            const ::PlatformTouchEvent& touch = raw.data.touch;
            return TouchInput{touch.id, static_cast<TouchPhase>(touch.phase), touch.x, touch.y};
        }
        case FFI_INPUT_IME: {
            const ::PlatformImeEvent& ime = raw.data.ime;
            ImeInput input;
            input.kind = ime.kind <= FFI_IME_DISABLED ? static_cast<ImeKind>(ime.kind) : ImeKind::Unknown;
            input.text = Text(ime.text);
            if (ime.has_cursor) {
                input.cursor = std::make_pair(ime.cursor_start, ime.cursor_end);
            }
            return input;
        }
        default:
            return UnknownEvent{};
    }
}

Event ToEvent(const ::PlatformEvent& raw)
{
    switch (raw.kind) {
        case FFI_EVENT_CLOSE_REQUESTED:
            return CloseRequested{};
        case FFI_EVENT_DESTROYED:
            return Destroyed{};
        case FFI_EVENT_RESIZED:
            return Resized{raw.data.size.width, raw.data.size.height};
        case FFI_EVENT_MOVED:
            return Moved{raw.data.position.x, raw.data.position.y};
        case FFI_EVENT_SCALE_FACTOR_CHANGED:
            return ScaleFactorChanged{
                raw.data.scale_factor_changed.scale_factor,
                raw.data.scale_factor_changed.inner_size.width,
                raw.data.scale_factor_changed.inner_size.height
            };
        case FFI_EVENT_FOCUSED:
            return Focused{raw.data.flag};
        case FFI_EVENT_OCCLUDED:
            return Occluded{raw.data.flag};
        case FFI_EVENT_REDRAW_REQUESTED:
            return RedrawRequested{};
        case FFI_EVENT_SURFACE_CREATED:
            return SurfaceCreated{ToHandle(raw.data.surface)};
        case FFI_EVENT_SURFACE_LOST:
            return SurfaceLost{};
        case FFI_EVENT_INPUT:
            return ToInputEvent(raw.data.input);
        default:
            return UnknownEvent{};
    }
}

/// State shared between `Run()` and the C callbacks.
struct RunState {
    App* app = nullptr;
    bool failed = false;
    std::string failure;
};

RunState& State(void* userData) noexcept
{
    return *static_cast<RunState*>(userData);
}

/// Runs `callback` for the application; an exception stops the loop (an
/// exception must not unwind through the Rust frames below us).
template <typename Callback>
void Guarded(void* userData, ::PlatformContext* rawContext, Callback&& callback) noexcept
{
    RunState& state = State(userData);
    if (state.failed) {
        return;
    }
    try {
        Context ctx(rawContext);
        callback(*state.app, ctx);
    } catch (const std::exception& error) {
        state.failed = true;
        state.failure = error.what();
        ffi_platform_ctx_exit(rawContext, -1);
    } catch (...) {
        state.failed = true;
        state.failure = "unknown exception";
        ffi_platform_ctx_exit(rawContext, -1);
    }
}

extern "C" {

void CvOnResumed(void* userData, ::PlatformContext* ctx)
{
    Guarded(userData, ctx, [](App& app, Context& context) { app.Resumed(context); });
}

void CvOnSuspended(void* userData, ::PlatformContext* ctx)
{
    Guarded(userData, ctx, [](App& app, Context& context) { app.Suspended(context); });
}

void CvOnWindowEvent(void* userData, ::PlatformContext* ctx, std::uint64_t window, const ::PlatformEvent* event)
{
    Guarded(userData, ctx, [window, event](App& app, Context& context) {
        app.OnEvent(context, window, ToEvent(*event));
    });
}

void CvOnAboutToWait(void* userData, ::PlatformContext* ctx)
{
    Guarded(userData, ctx, [](App& app, Context& context) { app.AboutToWait(context); });
}

} // extern "C"

} // namespace

// --- Context ------------------------------------------------------------------

std::expected<WindowId, std::string> Context::CreateWindow(const WindowDesc& desc)
{
    ::PlatformWindowDesc raw{};
    ffi_platform_window_desc_default(&raw);
    raw.title = desc.title.c_str();
    raw.width = desc.width;
    raw.height = desc.height;
    if (desc.position) {
        raw.has_position = true;
        raw.x = desc.position->first;
        raw.y = desc.position->second;
    }
    if (desc.minSize) {
        raw.has_min_size = true;
        raw.min_width = desc.minSize->first;
        raw.min_height = desc.minSize->second;
    }
    if (desc.maxSize) {
        raw.has_max_size = true;
        raw.max_width = desc.maxSize->first;
        raw.max_height = desc.maxSize->second;
    }
    raw.resizable = desc.resizable;
    raw.visible = desc.visible;
    raw.decorations = desc.decorations;
    raw.fullscreen = static_cast<std::uint8_t>(desc.fullscreen);

    std::uint64_t id = 0;
    if (!ffi_platform_ctx_create_window(RawContext(m_raw), &raw, &id)) {
        return std::unexpected(LastError());
    }
    return id;
}

std::expected<void, std::string> Context::CloseWindow(WindowId window)
{
    return Check(ffi_platform_ctx_close_window(RawContext(m_raw), window));
}

std::expected<void, std::string> Context::SetTitle(WindowId window, std::string_view title)
{
    const std::string owned(title); // NUL-terminated copy
    return Check(ffi_platform_ctx_set_title(RawContext(m_raw), window, owned.c_str()));
}

std::expected<void, std::string> Context::SetInnerSize(WindowId window, double width, double height)
{
    return Check(ffi_platform_ctx_set_inner_size(RawContext(m_raw), window, width, height));
}

std::expected<void, std::string> Context::SetFullscreen(WindowId window, Fullscreen fullscreen)
{
    return Check(ffi_platform_ctx_set_fullscreen(RawContext(m_raw), window, static_cast<std::uint8_t>(fullscreen)));
}

std::expected<void, std::string> Context::RequestRedraw(WindowId window)
{
    return Check(ffi_platform_ctx_request_redraw(RawContext(m_raw), window));
}

std::expected<void, std::string> Context::SetCursorVisible(WindowId window, bool visible)
{
    return Check(ffi_platform_ctx_set_cursor_visible(RawContext(m_raw), window, visible));
}

std::expected<void, std::string> Context::SetCursorGrab(WindowId window, CursorGrab grab)
{
    return Check(ffi_platform_ctx_set_cursor_grab(RawContext(m_raw), window, static_cast<std::uint8_t>(grab)));
}

std::expected<void, std::string> Context::SetImeAllowed(WindowId window, bool allowed)
{
    return Check(ffi_platform_ctx_set_ime_allowed(RawContext(m_raw), window, allowed));
}

std::expected<void, std::string>
Context::SetImeCursorArea(WindowId window, double x, double y, double width, double height)
{
    return Check(ffi_platform_ctx_set_ime_cursor_area(RawContext(m_raw), window, x, y, width, height));
}

std::expected<void, std::string> Context::SetControlFlow(ControlFlow flow, std::chrono::milliseconds wait)
{
    const auto millis = wait.count() < 0 ? std::uint64_t{0} : static_cast<std::uint64_t>(wait.count());
    return Check(ffi_platform_ctx_set_control_flow(RawContext(m_raw), static_cast<std::uint8_t>(flow), millis));
}

std::expected<void, std::string> Context::Exit(int exitCode)
{
    return Check(ffi_platform_ctx_exit(RawContext(m_raw), exitCode));
}

std::expected<WindowInfo, std::string> Context::GetWindowInfo(WindowId window) const
{
    ::PlatformWindowInfo raw{};
    if (!ffi_platform_ctx_window_info(RawContext(m_raw), window, &raw)) {
        return std::unexpected(LastError());
    }
    WindowInfo info;
    info.innerWidth = raw.inner_width;
    info.innerHeight = raw.inner_height;
    info.scaleFactor = raw.scale_factor;
    info.focused = raw.focused;
    info.occluded = raw.occluded;
    info.visible = raw.visible;
    info.fullscreen = raw.fullscreen == FFI_FULLSCREEN_BORDERLESS ? Fullscreen::Borderless : Fullscreen::None;
    info.cursorGrab = raw.cursor_grab == FFI_CURSOR_GRAB_LOCKED     ? CursorGrab::Locked
                      : raw.cursor_grab == FFI_CURSOR_GRAB_CONFINED ? CursorGrab::Confined
                                                                    : CursorGrab::None;
    info.imeAllowed = raw.ime_allowed;
    return info;
}

// --- App / Run ------------------------------------------------------------------

void App::Resumed(Context&) {}
void App::Suspended(Context&) {}
void App::OnEvent(Context&, WindowId, const Event&) {}
void App::AboutToWait(Context&) {}

std::expected<int, std::string> Run(App& app, const RunConfig& config)
{
    RunState state;
    state.app = &app;

    ::PlatformCallbacks callbacks{};
    callbacks.user_data = &state;
    callbacks.resumed = &CvOnResumed;
    callbacks.suspended = &CvOnSuspended;
    callbacks.window_event = &CvOnWindowEvent;
    callbacks.about_to_wait = &CvOnAboutToWait;

    ::PlatformRunConfig raw{};
    raw.backend = static_cast<std::uint8_t>(config.backend);
    raw.headless_scale_factor = config.headlessScaleFactor;
    raw.headless_monitor_width = config.headlessMonitorWidth;
    raw.headless_monitor_height = config.headlessMonitorHeight;

    int exitCode = 0;
    const bool ok = ffi_platform_run(&callbacks, &raw, &exitCode);
    // The last error must be read before anything else can overwrite it.
    std::string error = ok ? std::string{} : LastError();
    if (state.failed) {
        return std::unexpected("an application callback threw: " + state.failure);
    }
    if (!ok) {
        return std::unexpected(std::move(error));
    }
    return exitCode;
}

} // namespace cv_ffi::window
