#include "RendererBridge.hpp"

#include <algorithm>
#include <cmath>
#include <utility>

#include "renderer/CommandBuffer.hpp"
#include "renderer/GraphicsAPI.hpp"
#include "renderer/RenderDeviceFactory.hpp"
#include "renderer/RenderError.hpp"

namespace runtime {

namespace {

constexpr std::uint32_t kFramesInFlight = 2;
/// How many times in a row DrawFrame() rebuilds after a lost device/surface before giving up.
constexpr std::uint32_t kMaxConsecutiveRecoveries = 3;
constexpr double kColorCycleSeconds = 8.0;

std::string Describe(const renderer::RenderError& error)
{
    return error.detail;
}

/// A calm, slowly cycling background: hue turns once every `kColorCycleSeconds`,
/// saturation and value are fixed (the same look as the editor's viewport).
renderer::ClearColor ClearColorAt(double seconds)
{
    const double hue = std::fmod(seconds / kColorCycleSeconds, 1.0) * 6.0;
    const double saturation = 0.55;
    const double value = 0.55;
    const double chroma = value * saturation;
    const double x = chroma * (1.0 - std::fabs(std::fmod(hue, 2.0) - 1.0));
    double r = 0.0;
    double g = 0.0;
    double b = 0.0;
    switch (static_cast<int>(hue)) {
        case 0:
            r = chroma;
            g = x;
            break;
        case 1:
            r = x;
            g = chroma;
            break;
        case 2:
            g = chroma;
            b = x;
            break;
        case 3:
            g = x;
            b = chroma;
            break;
        case 4:
            r = x;
            b = chroma;
            break;
        default:
            r = chroma;
            b = x;
            break;
    }
    const double m = value - chroma;
    return renderer::ClearColor{static_cast<float>(r + m), static_cast<float>(g + m), static_cast<float>(b + m), 1.0F};
}

} // namespace

std::expected<renderer::NativeWindowHandle, std::string>
ToRendererHandle(const cv_ffi::window::NativeWindowHandle& handle)
{
    using cv_ffi::platform::DisplayServer;
    renderer::NativeWindowHandle native{};
#if defined(_WIN32)
    if (handle.kind != DisplayServer::Win32 || handle.hwnd == nullptr) {
        return std::unexpected("the surface handle is not a Win32 window");
    }
    native.hwnd = handle.hwnd;
    native.hinstance = handle.hinstance;
#elif defined(__ANDROID__)
    if (handle.kind != DisplayServer::Android || handle.nativeWindow == nullptr) {
        return std::unexpected("the surface handle is not an Android native window");
    }
    native.nativeWindow = handle.nativeWindow;
#elif defined(__APPLE__)
    if ((handle.kind != DisplayServer::Cocoa && handle.kind != DisplayServer::UIKit) || handle.metalLayer == nullptr) {
        return std::unexpected("the surface handle is not a Metal layer");
    }
    native.metalLayer = handle.metalLayer;
#elif defined(__linux__)
    if (handle.kind == DisplayServer::X11 && handle.xlibWindow != 0) {
        native.xlibDisplay = handle.xlibDisplay;
        native.xlibWindow = static_cast<unsigned long>(handle.xlibWindow);
    } else if (
        handle.kind == DisplayServer::Wayland && handle.waylandDisplay != nullptr && handle.waylandSurface != nullptr
    ) {
        native.waylandDisplay = handle.waylandDisplay;
        native.waylandSurface = handle.waylandSurface;
    } else {
        return std::unexpected("the surface handle is neither an X11 nor a Wayland window");
    }
#else
    (void)handle;
    (void)native;
    return std::unexpected("this platform has no renderer surface");
#endif
    return native;
}

RendererBridge::~RendererBridge()
{
    Shutdown();
}

renderer::SwapchainDesc RendererBridge::MakeSwapchainDesc() const noexcept
{
    renderer::SwapchainDesc desc{};
    desc.preferredImageCount = 2;
    desc.width = m_width;
    desc.height = m_height;
    return desc;
}

std::expected<void, std::string> RendererBridge::OnSurfaceCreated(
    const cv_ffi::window::NativeWindowHandle& handle, std::uint32_t width, std::uint32_t height
)
{
    auto native = ToRendererHandle(handle);
    if (!native) {
        return std::unexpected(native.error());
    }

    if (m_device == nullptr) {
        auto device = renderer::CreateRenderDevice(renderer::GraphicsAPI::Vulkan);
        if (!device) {
            return std::unexpected("render device: " + Describe(device.error()));
        }
        m_device = std::move(*device);
    }

    // A second SurfaceCreated without a SurfaceLost in between would leak the first.
    ReleaseSurface();
    m_handle.reset();

    m_width = width;
    m_height = height;

    auto surface = m_device->CreateSurface(*native);
    if (!surface) {
        return std::unexpected("surface: " + Describe(surface.error()));
    }
    m_surface = std::move(*surface);

    auto swapchain = m_device->CreateSwapchain(m_surface, MakeSwapchainDesc());
    if (!swapchain) {
        ReleaseSurface();
        return std::unexpected("swapchain: " + Describe(swapchain.error()));
    }
    m_swapchain = std::move(*swapchain);

    auto frameSync = m_device->CreateFrameSync(m_swapchain, renderer::FrameSyncDesc{kFramesInFlight});
    if (!frameSync) {
        ReleaseSurface();
        return std::unexpected("frame sync: " + Describe(frameSync.error()));
    }
    m_frameSync = std::move(*frameSync);
    m_handle = handle;
    return {};
}

void RendererBridge::OnSurfaceLost() noexcept
{
    ReleaseSurface();
    m_handle.reset();
}

void RendererBridge::ReleaseSurface() noexcept
{
    if (m_device != nullptr) {
        // The GPU must be done with the swapchain images before they go away.
        m_device->WaitIdle();
    }
    m_frameSync = renderer::FrameSync{};
    m_swapchain = renderer::Swapchain{};
    m_surface = renderer::Surface{};
}

std::expected<void, std::string> RendererBridge::Resize(std::uint32_t width, std::uint32_t height)
{
    if (!HasSurface() || width == 0 || height == 0) {
        return {}; // nothing to rebuild yet, or minimised: keep what there is
    }
    m_width = width;
    m_height = height;
    if (auto result = m_swapchain.Recreate(m_surface, MakeSwapchainDesc()); !result) {
        if (result.error().code == renderer::RenderErrorCode::ZeroExtent) {
            return {};
        }
        return std::unexpected("swapchain recreate: " + Describe(result.error()));
    }
    return {};
}

std::expected<bool, std::string> RendererBridge::FrameFailed(std::string_view stage, const renderer::RenderError& error)
{
    if (error.code == renderer::RenderErrorCode::DeviceLost || error.code == renderer::RenderErrorCode::SurfaceLost) {
        return Recover(stage, error);
    }
    return std::unexpected(std::string(stage) + ": " + Describe(error));
}

std::expected<bool, std::string> RendererBridge::Recover(std::string_view stage, const renderer::RenderError& error)
{
    if (!m_handle.has_value() || m_recoveries >= kMaxConsecutiveRecoveries) {
        return std::unexpected(std::string(stage) + ": " + Describe(error));
    }
    ++m_recoveries;

    // Copy first: Shutdown()/OnSurfaceLost() forget m_handle, OnSurfaceCreated() sets it again.
    const cv_ffi::window::NativeWindowHandle handle = *m_handle;
    if (error.code == renderer::RenderErrorCode::DeviceLost) {
        Shutdown(); // every resource belongs to the dead device
    } else {
        OnSurfaceLost();
    }
    if (auto created = OnSurfaceCreated(handle, m_width, m_height); !created) {
        return std::unexpected(
            std::string(stage) + " (recovery after '" + Describe(error) + "' failed): " + created.error()
        );
    }
    return false; // this frame is skipped; the next one draws on the rebuilt objects
}

std::expected<bool, std::string> RendererBridge::DrawFrame(double seconds)
{
    if (!HasSurface() || !m_frameSync.IsValid()) {
        return false;
    }

    auto begin = m_frameSync.BeginFrame(m_swapchain);
    if (!begin) {
        return FrameFailed("begin frame", begin.error());
    }
    switch (begin->status) {
        case renderer::FrameStatus::OutOfDate:
            if (auto resized = Resize(m_width, m_height); !resized) {
                return std::unexpected(resized.error());
            }
            return false;
        case renderer::FrameStatus::NotReady:
            return false; // no image within the timeout (occluded?): try again next frame
        case renderer::FrameStatus::Ready:
            break;
    }

    renderer::Frame& frame = begin->frame;
    renderer::CommandBuffer& commands = frame.commandBuffer;
    if (auto r = commands.Begin(); !r) {
        return FrameFailed("record begin", r.error());
    }
    if (auto r = commands.ClearColor(frame.imageHandle, ClearColorAt(seconds)); !r) {
        return FrameFailed("record clear", r.error());
    }
    if (auto r = commands.End(); !r) {
        return FrameFailed("record end", r.error());
    }

    auto end = m_frameSync.EndFrame(m_swapchain, frame);
    if (!end) {
        return FrameFailed("end frame", end.error());
    }
    if (*end == renderer::SwapchainStatus::OutOfDate) {
        if (auto resized = Resize(m_width, m_height); !resized) {
            return std::unexpected(resized.error());
        }
    }
    m_recoveries = 0; // a frame went all the way through: the device is working
    return true;
}

void RendererBridge::Shutdown() noexcept
{
    OnSurfaceLost();
    if (m_device != nullptr) {
        m_device->Shutdown();
        m_device.reset();
    }
}

} // namespace runtime
