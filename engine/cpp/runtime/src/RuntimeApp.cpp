#include "RuntimeApp.hpp"

#include <chrono>
#include <cstdio>
#include <string>
#include <variant>

#if defined(CV_RUNTIME_RENDERER)
    #include "RendererBridge.hpp"
#else
namespace runtime {
// No renderer in this build: an empty type keeps RuntimeApp's unique_ptr complete.
class RendererBridge
{
};
} // namespace runtime
#endif

namespace runtime {

namespace window = cv_ffi::window;

namespace {

template <typename... Ts>
struct Overloaded : Ts... {
    using Ts::operator()...;
};
template <typename... Ts>
Overloaded(Ts...) -> Overloaded<Ts...>;

const char* DisplayName(cv_ffi::platform::DisplayServer kind)
{
    using cv_ffi::platform::DisplayServer;
    switch (kind) {
        case DisplayServer::Win32: return "Win32";
        case DisplayServer::Cocoa: return "Cocoa";
        case DisplayServer::X11: return "X11";
        case DisplayServer::Wayland: return "Wayland";
        case DisplayServer::Android: return "Android";
        case DisplayServer::UIKit: return "UIKit";
        default: return "unknown";
    }
}

double SecondsSinceStart()
{
    static const auto start = std::chrono::steady_clock::now();
    return std::chrono::duration<double>(std::chrono::steady_clock::now() - start).count();
}

} // namespace

std::string Describe(const window::Event& event)
{
    return std::visit(
        Overloaded{
            [](const window::CloseRequested&) { return std::string("CloseRequested"); },
            [](const window::Destroyed&) { return std::string("Destroyed"); },
            [](const window::Resized& e) { return "Resized " + std::to_string(e.width) + "x" + std::to_string(e.height); },
            [](const window::Moved& e) { return "Moved " + std::to_string(e.x) + "," + std::to_string(e.y); },
            [](const window::ScaleFactorChanged& e) { return "ScaleFactorChanged " + std::to_string(e.scaleFactor); },
            [](const window::Focused& e) { return std::string("Focused ") + (e.focused ? "true" : "false"); },
            [](const window::Occluded& e) { return std::string("Occluded ") + (e.occluded ? "true" : "false"); },
            [](const window::RedrawRequested&) { return std::string("RedrawRequested"); },
            [](const window::SurfaceCreated& e) {
                return std::string("SurfaceCreated ") + DisplayName(e.handle.kind);
            },
            [](const window::SurfaceLost&) { return std::string("SurfaceLost"); },
            [](const window::KeyInput& e) {
                std::string text = "Key ";
                text += window::KeyName(e.physical);
                text += e.state == window::ButtonState::Pressed ? (e.repeat ? " repeat" : " pressed") : " released";
                if (e.logical.kind == window::LogicalKey::Kind::Named) {
                    text += " logical=";
                    text += window::NamedKeyName(e.logical.named);
                } else if (e.logical.kind == window::LogicalKey::Kind::Character) {
                    text += " logical='" + e.logical.text + "'";
                }
                return text;
            },
            [](const window::ModifiersChanged& e) {
                std::string text = "Modifiers";
                text += window::Has(e.modifiers, window::Modifiers::Shift) ? " shift" : "";
                text += window::Has(e.modifiers, window::Modifiers::Control) ? " ctrl" : "";
                text += window::Has(e.modifiers, window::Modifiers::Alt) ? " alt" : "";
                text += window::Has(e.modifiers, window::Modifiers::Super) ? " super" : "";
                return text;
            },
            [](const window::CursorMoved& e) {
                return "CursorMoved " + std::to_string(e.x) + "," + std::to_string(e.y);
            },
            [](const window::CursorEntered&) { return std::string("CursorEntered"); },
            [](const window::CursorLeft&) { return std::string("CursorLeft"); },
            [](const window::MouseButtonInput& e) {
                return "MouseButton " + std::to_string(static_cast<int>(e.button))
                       + (e.state == window::ButtonState::Pressed ? " pressed" : " released");
            },
            [](const window::MouseWheel& e) { return "MouseWheel " + std::to_string(e.x) + "," + std::to_string(e.y); },
            [](const window::RawMouseMotion& e) {
                return "RawMouseMotion " + std::to_string(e.dx) + "," + std::to_string(e.dy);
            },
            [](const window::TouchInput& e) { return "Touch " + std::to_string(e.id); },
            [](const window::ImeInput& e) { return "Ime " + std::to_string(static_cast<int>(e.kind)) + " " + e.text; },
            [](const window::UnknownEvent&) { return std::string("UnknownEvent"); },
        },
        event
    );
}

RuntimeApp::RuntimeApp(Options options) : m_options(std::move(options)) {}

RuntimeApp::~RuntimeApp() = default;

void RuntimeApp::Fail(window::Context& ctx, const std::string& message)
{
    std::fprintf(stderr, "coreverse-runtime: %s\n", message.c_str());
    m_failed = true;
    (void)ctx.Exit(2);
}

void RuntimeApp::Resumed(window::Context& ctx)
{
    if (m_window) {
        return; // coming back from the background: the window is still there
    }
    window::WindowDesc desc;
    desc.title = m_options.title;
    auto id = ctx.CreateWindow(desc);
    if (!id) {
        Fail(ctx, "cannot create the window: " + id.error());
        return;
    }
    m_window = *id;
    // A game draws continuously.
    if (auto flow = ctx.SetControlFlow(window::ControlFlow::Poll); !flow) {
        Fail(ctx, flow.error());
    }
}

void RuntimeApp::Suspended(window::Context&)
{
    if (m_options.trace) {
        std::puts("Suspended");
    }
}

void RuntimeApp::OnEvent(window::Context& ctx, window::WindowId window, const window::Event& event)
{
    if (m_options.trace) {
        std::printf("window %llu: %s\n", static_cast<unsigned long long>(window), Describe(event).c_str());
    }

    std::visit(
        Overloaded{
            [&](const window::CloseRequested&) { (void)ctx.Exit(0); },
            [&](const window::SurfaceCreated& created) {
#if defined(CV_RUNTIME_RENDERER)
                if (!m_options.useRenderer) {
                    return;
                }
                if (m_renderer == nullptr) {
                    m_renderer = std::make_unique<RendererBridge>();
                }
                std::uint32_t width = 1;
                std::uint32_t height = 1;
                if (auto info = ctx.GetWindowInfo(window)) {
                    width = info->innerWidth;
                    height = info->innerHeight;
                }
                if (auto result = m_renderer->OnSurfaceCreated(created.handle, width, height); !result) {
                    Fail(ctx, result.error());
                }
#else
                (void)created;
#endif
            },
            [&](const window::SurfaceLost&) {
#if defined(CV_RUNTIME_RENDERER)
                // Everything built from the native handle goes before this returns.
                if (m_renderer != nullptr) {
                    m_renderer->OnSurfaceLost();
                }
#endif
            },
            [&](const window::Resized& resized) {
#if defined(CV_RUNTIME_RENDERER)
                if (m_renderer != nullptr) {
                    if (auto result = m_renderer->Resize(resized.width, resized.height); !result) {
                        Fail(ctx, result.error());
                    }
                }
#else
                (void)resized;
#endif
            },
            [&](const window::RedrawRequested&) { DrawFrame(ctx); },
            [&](const window::KeyInput& key) {
                if (key.state != window::ButtonState::Pressed || key.repeat) {
                    return;
                }
                if (key.physical == window::KeyCode::Escape) {
                    (void)ctx.Exit(0);
                } else if (key.physical == window::KeyCode::F11) {
                    auto info = ctx.GetWindowInfo(window);
                    const bool full = info && info->fullscreen == window::Fullscreen::Borderless;
                    (void)ctx.SetFullscreen(window, full ? window::Fullscreen::None : window::Fullscreen::Borderless);
                }
            },
            [](const auto&) {},
        },
        event
    );
}

void RuntimeApp::AboutToWait(window::Context& ctx)
{
    // The loop is idle: ask for the next frame.
    if (m_window) {
        if (auto redraw = ctx.RequestRedraw(*m_window); !redraw) {
            Fail(ctx, redraw.error());
        }
    }
}

void RuntimeApp::DrawFrame(window::Context& ctx)
{
    bool counted = true;
#if defined(CV_RUNTIME_RENDERER)
    if (m_renderer != nullptr && m_renderer->HasSurface()) {
        auto drawn = m_renderer->DrawFrame(SecondsSinceStart());
        if (!drawn) {
            Fail(ctx, drawn.error());
            return;
        }
        counted = *drawn;
    }
#else
    (void)SecondsSinceStart;
#endif
    if (!counted) {
        return;
    }
    ++m_frames;
    if (m_options.maxFrames != 0 && m_frames >= m_options.maxFrames) {
        (void)ctx.Exit(0);
    }
}

} // namespace runtime
