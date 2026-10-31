#pragma once

#include <cstdint>
#include <memory>
#include <optional>
#include <string>

#include "cv-ffi/Window.hpp"

namespace runtime {

/// What the command line asked for.
struct Options {
    /// Run on the headless backend: no window system, no surface, no input.
    bool headless = false;
    /// Print every window event.
    bool trace = false;
    /// Exit after this many frames; 0 runs until the window is closed.
    std::uint64_t maxFrames = 0;
    /// Do not create a device and swapchain even if the renderer is built.
    bool useRenderer = true;
    std::string title = "Coreverse";
};

/// Owns the renderer when there is one; defined in RendererBridge.hpp.
class RendererBridge;

/// The application the runtime executable runs: one window, a continuous loop,
/// Escape or the close button to leave.
///
/// The window events come from the Rust platform crate through cv-ffi. A
/// `SurfaceCreated` event builds the renderer's swapchain from the native
/// handle it carries, `SurfaceLost` tears it down again, `Resized` rebuilds it
/// and every `RedrawRequested` draws a frame.
class RuntimeApp final : public cv_ffi::window::App
{
public:
    explicit RuntimeApp(Options options);
    ~RuntimeApp() override;

    RuntimeApp(const RuntimeApp&) = delete;
    RuntimeApp& operator=(const RuntimeApp&) = delete;

    void Resumed(cv_ffi::window::Context& ctx) override;
    void Suspended(cv_ffi::window::Context& ctx) override;
    void OnEvent(cv_ffi::window::Context& ctx, cv_ffi::window::WindowId window, const cv_ffi::window::Event& event) override;
    void AboutToWait(cv_ffi::window::Context& ctx) override;

    /// Frames drawn (or, without a renderer, redraw events handled).
    [[nodiscard]] std::uint64_t FramesRendered() const noexcept { return m_frames; }

    /// Whether the application stopped because of an error (exit code 2).
    [[nodiscard]] bool Failed() const noexcept { return m_failed; }

private:
    void DrawFrame(cv_ffi::window::Context& ctx);
    void Fail(cv_ffi::window::Context& ctx, const std::string& message);

    Options m_options;
    std::optional<cv_ffi::window::WindowId> m_window;
    std::uint64_t m_frames = 0;
    bool m_failed = false;
    std::unique_ptr<RendererBridge> m_renderer;
};

/// A one-line description of an event, for `--trace`.
[[nodiscard]] std::string Describe(const cv_ffi::window::Event& event);

} // namespace runtime
