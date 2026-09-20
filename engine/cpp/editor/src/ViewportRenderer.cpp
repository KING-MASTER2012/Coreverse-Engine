#include "ViewportRenderer.hpp"

#include <QDebug>
#include <utility>

#include "NativeWindowBridge.hpp"
#include "ViewportWidget.hpp"
#include "renderer/GraphicsAPI.hpp"
#include "renderer/RenderDeviceFactory.hpp"
#include "renderer/RenderError.hpp"

namespace editor {

namespace {

QString ToQString(const renderer::RenderError& error)
{
    return QString::fromStdString(error.detail);
}

} // namespace

ViewportRenderer::ViewportRenderer(ViewportWidget& viewport, QObject* parent) : QObject(parent), m_viewport(viewport)
{
    // Interval 0 = "next time the event loop is idle": every resize event
    // already queued gets processed first, so a drag costs one rebuild.
    m_resizeTimer.setSingleShot(true);
    m_resizeTimer.setInterval(0);
    connect(&m_resizeTimer, &QTimer::timeout, this, [this] {
        if (auto result = rebuildSwapchain(); !result) {
            qWarning().noquote() << "editor: swapchain rebuild failed:" << result.error();
            emit swapchainRebuildFailed(result.error());
        }
    });
    connect(&m_viewport, &ViewportWidget::pixelSizeChanged, this, &ViewportRenderer::scheduleSwapchainRebuild);
}

ViewportRenderer::~ViewportRenderer()
{
    shutdown();
}

renderer::SwapchainDesc ViewportRenderer::makeSwapchainDesc() const
{
    const QSize size = m_viewport.pixelSize();

    renderer::SwapchainDesc desc{};
    desc.preferredImageCount = 2;
    desc.width = static_cast<std::uint32_t>(size.width());
    desc.height = static_cast<std::uint32_t>(size.height());
    return desc;
}

std::expected<void, QString> ViewportRenderer::initialize()
{
    if (isInitialized()) {
        return {};
    }

    auto bridge = NativeWindowBridge::create(m_viewport);
    if (!bridge) {
        return std::unexpected(QStringLiteral("native window: %1").arg(bridge.error()));
    }
    m_bridge = std::move(*bridge);

    auto device = renderer::CreateRenderDevice(renderer::GraphicsAPI::Vulkan);
    if (!device) {
        shutdown();
        return std::unexpected(QStringLiteral("render device: %1").arg(ToQString(device.error())));
    }
    m_device = std::move(*device);

    auto surface = m_device->CreateSurface(m_bridge->handle());
    if (!surface) {
        shutdown();
        return std::unexpected(QStringLiteral("surface: %1").arg(ToQString(surface.error())));
    }
    m_surface = std::move(*surface);

    auto swapchain = m_device->CreateSwapchain(m_surface, makeSwapchainDesc());
    if (!swapchain) {
        shutdown();
        return std::unexpected(QStringLiteral("swapchain: %1").arg(ToQString(swapchain.error())));
    }
    m_swapchain = std::move(*swapchain);

    return {};
}

void ViewportRenderer::shutdown() noexcept
{
    m_resizeTimer.stop();

    if (m_device != nullptr) {
        // Nothing tracks frames in flight yet (Phase 6.4), so make sure the
        // GPU is done with the swapchain images before they go away.
        m_device->WaitIdle();
    }

    // Reverse creation order — see the member comment in the header.
    m_swapchain = renderer::Swapchain{};
    m_surface = renderer::Surface{};
    if (m_device != nullptr) {
        m_device->Shutdown();
        m_device.reset();
    }
    m_bridge.reset();
}

QString ViewportRenderer::deviceName() const
{
    if (m_device == nullptr) {
        return {};
    }
    const std::string_view name = m_device->GetDeviceName();
    return QString::fromUtf8(name.data(), static_cast<qsizetype>(name.size()));
}

QSize ViewportRenderer::swapchainExtent() const noexcept
{
    const renderer::Extent2D extent = m_swapchain.GetExtent();
    return QSize(static_cast<int>(extent.width), static_cast<int>(extent.height));
}

void ViewportRenderer::scheduleSwapchainRebuild()
{
    if (isInitialized()) {
        m_resizeTimer.start();
    }
}

std::expected<void, QString> ViewportRenderer::rebuildSwapchain()
{
    if (!isInitialized()) {
        return std::unexpected(QStringLiteral("the renderer is not initialized"));
    }

    const QSize pixelSize = m_viewport.pixelSize();
    if (pixelSize.isEmpty()) {
        return {}; // hidden or collapsed: keep the old swapchain, try again on the next resize
    }
    if (pixelSize == swapchainExtent()) {
        return {}; // already the right size
    }

    m_bridge->pixelSizeChanged(pixelSize, m_viewport.devicePixelRatioF());

    if (auto result = m_swapchain.Recreate(m_surface, makeSwapchainDesc()); !result) {
        if (result.error().code == renderer::RenderErrorCode::ZeroExtent) {
            return {}; // e.g. minimized: the surface has no size; nothing to rebuild yet
        }
        return std::unexpected(ToQString(result.error()));
    }

    emit swapchainRebuilt(swapchainExtent(), m_swapchain.GetImageCount());
    return {};
}

} // namespace editor
