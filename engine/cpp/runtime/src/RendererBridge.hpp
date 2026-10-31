#pragma once

#include <cstdint>
#include <expected>
#include <memory>
#include <string>

#include "cv-ffi/Window.hpp"
#include "renderer/RenderDevice.hpp"
#include "renderer/Surface.hpp"
#include "renderer/Swapchain.hpp"
#include "renderer/FrameSync.hpp"

namespace runtime {

/// Converts the native window handle of a platform `SurfaceCreated` event to the
/// renderer's `NativeWindowHandle`, field by field. Fails when the handle is
/// not the kind this platform's renderer expects (an X11 or Wayland handle on
/// Linux, an HWND on Windows, ...), because handing a Vulkan surface call the
/// wrong kind of pointer would not fail gracefully.
[[nodiscard]] std::expected<renderer::NativeWindowHandle, std::string>
ToRendererHandle(const cv_ffi::window::NativeWindowHandle& handle);

/// The renderer side of the runtime: a Vulkan device, and a surface, swapchain
/// and frame synchronisation for the window that currently has a surface.
///
/// The device outlives surfaces (on Android the surface goes away and comes
/// back while the application lives); the surface and everything built from it
/// are created on `SurfaceCreated` and destroyed on `SurfaceLost`.
class RendererBridge
{
public:
    RendererBridge() = default;
    ~RendererBridge();

    RendererBridge(const RendererBridge&) = delete;
    RendererBridge& operator=(const RendererBridge&) = delete;

    /// Builds the surface and swapchain from the handle of a `SurfaceCreated`
    /// event (and the device on the first call). `width` x `height` is the
    /// client area in physical pixels.
    [[nodiscard]] std::expected<void, std::string>
    OnSurfaceCreated(const cv_ffi::window::NativeWindowHandle& handle, std::uint32_t width, std::uint32_t height);

    /// Destroys the swapchain and the surface. Must run before the handler of
    /// `SurfaceLost` returns: the native window behind them is going away.
    void OnSurfaceLost() noexcept;

    /// Rebuilds the swapchain for a new size. A zero size (minimised) keeps the old one.
    [[nodiscard]] std::expected<void, std::string> Resize(std::uint32_t width, std::uint32_t height);

    /// Draws and presents one frame (a clear colour that cycles with `seconds`).
    /// Does nothing without a surface. Returns whether a frame was presented.
    [[nodiscard]] std::expected<bool, std::string> DrawFrame(double seconds);

    [[nodiscard]] bool HasSurface() const noexcept { return m_surface.IsValid(); }

    /// Waits for the GPU and destroys everything, in reverse creation order.
    void Shutdown() noexcept;

private:
    [[nodiscard]] renderer::SwapchainDesc MakeSwapchainDesc() const noexcept;

    std::unique_ptr<renderer::RenderDevice> m_device;
    renderer::Surface m_surface;
    renderer::Swapchain m_swapchain;
    renderer::FrameSync m_frameSync;
    std::uint32_t m_width = 0;
    std::uint32_t m_height = 0;
};

} // namespace runtime
