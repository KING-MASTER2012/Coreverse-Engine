#include "ViewportRenderer.hpp"

#include <QColor>
#include <QDebug>
#include <cmath>
#include <utility>

#include "NativeWindowBridge.hpp"
#include "ViewportWidget.hpp"
#include "renderer/GraphicsAPI.hpp"
#include "renderer/RenderDeviceFactory.hpp"
#include "renderer/RenderError.hpp"

namespace editor {

namespace {

/// ~60 frames per second. The timer only paces the loop: FrameSync bounds how
/// far the CPU runs ahead of the GPU, and the presentation engine (vsync)
/// bounds the rate for real.
constexpr int kFrameIntervalMs = 16;

/// How soon a failed swapchain rebuild is tried again (besides retrying on
/// every resize).
constexpr int kRebuildRetryMs = 250;

/// One full trip around the hue circle, in seconds. Slow enough to be calm,
/// fast enough to see at a glance that frames are being presented.
constexpr float kColorCycleSeconds = 12.0F;

/// Frames in flight: the CPU may record one frame while the GPU renders the
/// previous one.
constexpr std::uint32_t kFramesInFlight = 2;

QString ToQString(const renderer::RenderError& error)
{
    return QString::fromStdString(error.detail);
}

renderer::ClearColor ClearColorAt(qint64 elapsedMs)
{
    const float seconds = static_cast<float>(elapsedMs) / 1000.0F;
    const float hue = std::fmod(seconds / kColorCycleSeconds, 1.0F);
    const QColor color = QColor::fromHsvF(hue, 0.55F, 0.55F);
    return renderer::ClearColor{
        static_cast<float>(color.redF()), static_cast<float>(color.greenF()), static_cast<float>(color.blueF()), 1.0F
    };
}

} // namespace

ViewportRenderer::ViewportRenderer(ViewportWidget& viewport, QObject* parent) : QObject(parent), m_viewport(viewport)
{
    m_viewportVisible = m_viewport.isVisible();

    // Interval 0 = "next time the event loop is idle": every resize event
    // already queued gets processed first, so a drag costs one rebuild. While
    // the swapchain is waiting for a rebuild the size shortcut is skipped: it
    // has to be rebuilt whatever its size is.
    m_resizeTimer.setSingleShot(true);
    m_resizeTimer.setInterval(0);
    connect(&m_resizeTimer, &QTimer::timeout, this, [this] { rebuildAndUpdate(m_awaitingRebuild); });
    connect(&m_viewport, &ViewportWidget::pixelSizeChanged, this, &ViewportRenderer::scheduleSwapchainRebuild);

    m_retryTimer.setSingleShot(true);
    m_retryTimer.setInterval(kRebuildRetryMs);
    connect(&m_retryTimer, &QTimer::timeout, this, [this] {
        if (m_awaitingRebuild) {
            rebuildAndUpdate(true);
        }
    });

    m_frameTimer.setTimerType(Qt::PreciseTimer);
    m_frameTimer.setInterval(kFrameIntervalMs);
    connect(&m_frameTimer, &QTimer::timeout, this, &ViewportRenderer::renderFrame);

    connect(&m_viewport, &ViewportWidget::visibleChanged, this, [this](bool visible) {
        m_viewportVisible = visible;
        if (visible && m_awaitingRebuild) {
            scheduleSwapchainRebuild(); // a widget that was hidden usually comes back with a (new) size
        }
        updateFrameTimer();
    });
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

    auto frameSync = m_device->CreateFrameSync(m_swapchain, renderer::FrameSyncDesc{kFramesInFlight});
    if (!frameSync) {
        shutdown();
        return std::unexpected(QStringLiteral("frame sync: %1").arg(ToQString(frameSync.error())));
    }
    m_frameSync = std::move(*frameSync);

    m_framesRendered = 0;
    m_framesAtLastStats = 0;
    m_awaitingRebuild = false;
    m_renderingFailed = false;
    m_viewportVisible = m_viewport.isVisible();
    m_clock.start();
    updateFrameTimer();

    return {};
}

void ViewportRenderer::shutdown() noexcept
{
    m_resizeTimer.stop();
    m_frameTimer.stop();
    m_retryTimer.stop();

    if (m_device != nullptr) {
        // The GPU must be done with the swapchain images (and everything the
        // frame sync recorded) before any of it goes away.
        m_device->WaitIdle();
    }

    // Reverse creation order — see the member comment in the header.
    m_frameSync = renderer::FrameSync{};
    m_swapchain = renderer::Swapchain{};
    m_surface = renderer::Surface{};
    if (m_device != nullptr) {
        m_device->Shutdown();
        m_device.reset();
    }
    m_bridge.reset();

    m_awaitingRebuild = false;
}

QString ViewportRenderer::deviceName() const
{
    if (m_device == nullptr) {
        return {};
    }
    const std::string_view name = m_device->GetDeviceName();
    return QString::fromUtf8(name.data(), static_cast<qsizetype>(name.size()));
}

bool ViewportRenderer::validationEnabled() const noexcept
{
    return m_device != nullptr && m_device->IsValidationEnabled();
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

void ViewportRenderer::setWindowMinimized(bool minimized)
{
    if (m_windowMinimized == minimized) {
        return;
    }
    m_windowMinimized = minimized;
    if (!minimized && m_awaitingRebuild) {
        scheduleSwapchainRebuild(); // a restored window usually has a size again
    }
    updateFrameTimer();
}

std::expected<bool, QString> ViewportRenderer::rebuildSwapchain(bool force)
{
    if (!isInitialized()) {
        return std::unexpected(QStringLiteral("the renderer is not initialized"));
    }

    const QSize pixelSize = m_viewport.pixelSize();
    if (pixelSize.isEmpty()) {
        return false; // hidden or collapsed: keep the old swapchain, try again on the next resize
    }
    if (!force && pixelSize == swapchainExtent()) {
        return false; // already the right size
    }

    m_bridge->pixelSizeChanged(pixelSize, m_viewport.devicePixelRatioF());

    if (auto result = m_swapchain.Recreate(m_surface, makeSwapchainDesc()); !result) {
        if (result.error().code == renderer::RenderErrorCode::ZeroExtent) {
            return false; // e.g. minimized: the surface has no size; nothing to rebuild yet
        }
        return std::unexpected(ToQString(result.error()));
    }

    m_awaitingRebuild = false;
    m_retryTimer.stop();
    emit swapchainRebuilt(swapchainExtent(), m_swapchain.GetImageCount());
    return true;
}

void ViewportRenderer::rebuildAndUpdate(bool force)
{
    auto result = rebuildSwapchain(force);
    if (!result) {
        qWarning().noquote() << "editor: swapchain rebuild failed:" << result.error();
        m_awaitingRebuild = true; // the swapchain is empty now: nothing can be presented until a rebuild works
        m_retryTimer.start();
        emit swapchainRebuildFailed(result.error());
    } else if (!*result && force) {
        m_awaitingRebuild = true; // it has to be rebuilt but has no size to be rebuilt to: wait for a resize
    }
    updateFrameTimer();
}

void ViewportRenderer::updateFrameTimer()
{
    const bool shouldRun = isInitialized() && m_frameSync.IsValid() && !m_renderingFailed && !m_awaitingRebuild &&
                           !m_windowMinimized && m_viewportVisible;

    if (shouldRun && !m_frameTimer.isActive()) {
        m_statsClock.restart();
        m_framesAtLastStats = m_framesRendered;
        m_frameTimer.start();
    } else if (!shouldRun && m_frameTimer.isActive()) {
        m_frameTimer.stop();
    }
}

void ViewportRenderer::failRendering(const QString& reason)
{
    m_renderingFailed = true;
    m_frameTimer.stop();
    qWarning().noquote() << "editor: rendering stopped:" << reason;
    emit renderingFailed(reason);
}

void ViewportRenderer::renderFrame()
{
    if (!isInitialized() || !m_frameSync.IsValid() || m_renderingFailed) {
        return;
    }

    auto begin = m_frameSync.BeginFrame(m_swapchain);
    if (!begin) {
        failRendering(QStringLiteral("begin frame: %1").arg(ToQString(begin.error())));
        return;
    }

    switch (begin->status) {
        case renderer::FrameStatus::OutOfDate:
            rebuildAndUpdate(true);
            return;
        case renderer::FrameStatus::NotReady:
            return; // no image within the timeout (occluded window?): the next tick tries again
        case renderer::FrameStatus::Ready:
            break;
    }

    // From here the frame is committed: an acquired image cannot be handed
    // back, so a recording failure ends rendering instead of skipping ahead.
    renderer::Frame& frame = begin->frame;

    if (auto result = frame.commandBuffer.Begin(); !result) {
        failRendering(QStringLiteral("record: %1").arg(ToQString(result.error())));
        return;
    }
    if (auto result = frame.commandBuffer.ClearColor(frame.imageHandle, ClearColorAt(m_clock.elapsed())); !result) {
        failRendering(QStringLiteral("record: %1").arg(ToQString(result.error())));
        return;
    }
    if (auto result = frame.commandBuffer.End(); !result) {
        failRendering(QStringLiteral("record: %1").arg(ToQString(result.error())));
        return;
    }

    auto end = m_frameSync.EndFrame(m_swapchain, frame);
    if (!end) {
        failRendering(QStringLiteral("end frame: %1").arg(ToQString(end.error())));
        return;
    }

    ++m_framesRendered;
    emit frameRendered(m_framesRendered);

    const qint64 statsElapsedMs = m_statsClock.elapsed();
    if (statsElapsedMs >= 1000) {
        const double fps =
            static_cast<double>(m_framesRendered - m_framesAtLastStats) * 1000.0 / static_cast<double>(statsElapsedMs);
        m_statsClock.restart();
        m_framesAtLastStats = m_framesRendered;
        emit frameStatsUpdated(fps, m_framesRendered);
    }

    switch (*end) {
        case renderer::SwapchainStatus::OutOfDate:
            rebuildAndUpdate(true);
            break;
        case renderer::SwapchainStatus::Suboptimal:
            // Rebuild when the size is what is suboptimal; some platforms keep
            // reporting Suboptimal for reasons a rebuild does not fix, and
            // rebuilding on every frame for those would be worse than the
            // suboptimality.
            if (m_viewport.pixelSize() != swapchainExtent()) {
                rebuildAndUpdate(false);
            }
            break;
        case renderer::SwapchainStatus::Ok:
        case renderer::SwapchainStatus::NotReady:
            break;
    }
}

} // namespace editor
