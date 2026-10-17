#include "ViewportRenderer.hpp"

#include <QColor>
#include <QDebug>
#include <cmath>
#include <iterator>
#include <string_view>
#include <utility>

#include "NativeWindowBridge.hpp"
#include "ViewportWidget.hpp"
#include "cv-ffi/Diagnostic.hpp"
#include "cv-ffi/Logger.hpp"
#include "cv-math/Math.hpp"
#include "renderer/BuiltinMeshes.hpp"
#include "renderer/BuiltinShaders.hpp"
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

/// Consecutive device-loss recoveries (without a rendered frame in between) before giving up.
constexpr unsigned kMaxRecoveryAttempts = 3;

/// The cube's rotation speed, radians per second around Y (X turns at two thirds of it).
constexpr float kCubeSpinRadiansPerSecond = 0.9F;
constexpr float kFovYRadians = 1.0471976F; // 60 degrees

QString ToQString(const renderer::RenderError& error)
{
    return QString::fromStdString(error.detail);
}

/// Producer/category this ViewportRenderer's renderer log messages are
/// emitted under — see EditorWindow's constructor for where
/// `"CV-RENDERER"` is registered as a producer (once, process-wide;
/// registering it here on every ViewportRenderer::initialize() call would
/// fail every time after the first — see
/// `log_core::producer::register_dynamic`'s "collides" failure mode).
constexpr std::string_view kRendererLogProducer = "CV-RENDERER";
constexpr std::string_view kRendererLogCategory = "VULKAN";

cv_ffi::Severity ToCvFfiSeverity(renderer::RenderDevice::LogSeverity severity)
{
    switch (severity) {
        case renderer::RenderDevice::LogSeverity::Verbose:
            return cv_ffi::Severity::Trace;
        case renderer::RenderDevice::LogSeverity::Info:
            return cv_ffi::Severity::Info;
        case renderer::RenderDevice::LogSeverity::Warning:
            return cv_ffi::Severity::Warning;
        case renderer::RenderDevice::LogSeverity::Error:
            return cv_ffi::Severity::Error;
    }
    return cv_ffi::Severity::Info; // unreachable: every enumerator above is handled
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

ViewportRenderer::ViewportRenderer(ViewportWidget& viewport, cv_ffi::Logger* logger, QObject* parent)
    : QObject(parent), m_viewport(viewport), m_logger(logger)
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
    desc.depthBuffer = true; // for the graphics pass (BeginRendering() attaches it)
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

    if (auto resources = createSceneResources(); !resources) {
        shutdown();
        return std::unexpected(resources.error());
    }

    if (m_logger != nullptr) {
        // Captures `this` and the (not-owned) `m_logger` pointer, not a
        // Logger by value: this lambda's lifetime is exactly this device's
        // lifetime (cleared in shutdown() before the device is torn down),
        // and m_logger's own lifetime is EditorWindow's responsibility (see
        // the constructor's doc comment).
        m_device->SetLogCallback([this](renderer::RenderDevice::LogSeverity severity, std::string_view message) {
            auto builder = cv_ffi::DiagnosticBuilder::Create(
                kRendererLogProducer, kRendererLogCategory, 0, ToCvFfiSeverity(severity), message
            );
            if (builder) {
                std::move(*builder).Emit(*m_logger);
            }
            // A failed Create() (message/producer/category not valid UTF-8 —
            // practically unreachable, see DiagnosticBuilder::Create's doc
            // comment) just drops this one message; the same message is
            // still on stderr via the renderer's own unconditional logging
            // (RenderDevice::LogCallback's doc comment).
        });
    }

    if (!m_recovering) {
        m_framesRendered = 0; // a recovery keeps counting: the frame count is the viewport's, not the device's
    }
    m_framesAtLastStats = m_framesRendered;
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
        // Cleared before WaitIdle()/teardown below: once this returns, the
        // device may still produce messages (Shutdown() itself can log) but
        // this ViewportRenderer is no longer certain m_logger is safe to
        // reach through -- see the constructor's doc comment on m_logger's
        // lifetime being the caller's responsibility, not this object's.
        m_device->SetLogCallback(nullptr);
        // The GPU must be done with the swapchain images (and everything the
        // frame sync recorded) before any of it goes away.
        m_device->WaitIdle();
    }

    // Reverse creation order — see the member comment in the header.
    // Everything that references the device goes first, in dependency order. WaitIdle() above
    // makes sure the GPU is done with the pipeline and buffers.
    m_pipeline = renderer::GraphicsPipeline{};
    m_indexBuffer = renderer::Buffer{};
    m_vertexBuffer = renderer::Buffer{};
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
    qWarning().noquote() << "editor: rendering.md stopped:" << reason;
    emit renderingFailed(reason);
}

void ViewportRenderer::simulateDeviceLoss() noexcept
{
    if (m_device != nullptr) {
        m_device->SimulateDeviceLossForTesting();
    }
}

std::expected<void, QString> ViewportRenderer::createSceneResources()
{
    renderer::BufferDesc vertexDesc{};
    vertexDesc.size = sizeof(renderer::kCubeVertices);
    vertexDesc.usage = renderer::BufferUsage::VertexBuffer;
    vertexDesc.memoryUsage = renderer::BufferMemoryUsage::CpuToGpu;
    auto vertexBuffer = m_device->CreateBuffer(vertexDesc);
    if (!vertexBuffer) {
        return std::unexpected(QStringLiteral("vertex buffer: %1").arg(ToQString(vertexBuffer.error())));
    }
    m_vertexBuffer = std::move(*vertexBuffer);
    if (auto written = m_vertexBuffer.Write(renderer::kCubeVertices, sizeof(renderer::kCubeVertices)); !written) {
        return std::unexpected(QStringLiteral("vertex buffer upload: %1").arg(ToQString(written.error())));
    }

    renderer::BufferDesc indexDesc{};
    indexDesc.size = sizeof(renderer::kCubeIndices);
    indexDesc.usage = renderer::BufferUsage::IndexBuffer;
    indexDesc.memoryUsage = renderer::BufferMemoryUsage::CpuToGpu;
    auto indexBuffer = m_device->CreateBuffer(indexDesc);
    if (!indexBuffer) {
        return std::unexpected(QStringLiteral("index buffer: %1").arg(ToQString(indexBuffer.error())));
    }
    m_indexBuffer = std::move(*indexBuffer);
    if (auto written = m_indexBuffer.Write(renderer::kCubeIndices, sizeof(renderer::kCubeIndices)); !written) {
        return std::unexpected(QStringLiteral("index buffer upload: %1").arg(ToQString(written.error())));
    }

    renderer::GraphicsPipelineDesc pipelineDesc{};
    pipelineDesc.vertexShader = renderer::ColoredMeshVertexShader();
    pipelineDesc.fragmentShader = renderer::ColoredMeshFragmentShader();
    pipelineDesc.vertexStride = sizeof(renderer::ColoredVertex);
    pipelineDesc.vertexAttributes = renderer::kColoredVertexAttributes;
    pipelineDesc.pushConstantSize = renderer::kColoredMeshPushConstantSize;
    pipelineDesc.cullMode = renderer::CullMode::Back;
    pipelineDesc.depthTest = true;
    pipelineDesc.depthWrite = true;
    auto pipeline = m_device->CreateGraphicsPipeline(m_swapchain, pipelineDesc);
    if (!pipeline) {
        if (pipeline.error().code == renderer::RenderErrorCode::Unsupported) {
            qWarning().noquote() << "editor: graphics pass unavailable, drawing the clear color only:"
                                 << ToQString(pipeline.error());
            return {};
        }
        return std::unexpected(QStringLiteral("graphics pipeline: %1").arg(ToQString(pipeline.error())));
    }
    m_pipeline = std::move(*pipeline);
    return {};
}

std::expected<void, QString> ViewportRenderer::recordFrame(renderer::Frame& frame)
{
    renderer::CommandBuffer& cb = frame.commandBuffer;
    const auto fail = [](const char* what, const renderer::RenderError& error) {
        return std::unexpected(QStringLiteral("record %1: %2").arg(QString::fromUtf8(what), ToQString(error)));
    };

    if (auto r = cb.Begin(); !r) {
        return fail("begin", r.error());
    }

    if (!m_pipeline.IsValid()) {
        if (auto r = cb.ClearColor(frame.imageHandle, ClearColorAt(m_clock.elapsed())); !r) {
            return fail("clear", r.error());
        }
        if (auto r = cb.End(); !r) {
            return fail("end", r.error());
        }
        return {};
    }

    namespace m = cv_math;
    const float seconds = static_cast<float>(m_clock.elapsed()) / 1000.0F;
    const renderer::Extent2D extent = m_swapchain.GetExtent();
    const float aspect = static_cast<float>(extent.width) / static_cast<float>(extent.height);
    const m::Mat4 model = m::Mat4::fromRotationY(seconds * kCubeSpinRadiansPerSecond) *
                          m::Mat4::fromRotationX(seconds * kCubeSpinRadiansPerSecond * 0.66F);
    const m::Mat4 view = m::Mat4::lookAtRh({0.0F, 0.0F, 5.0F}, {0.0F, 0.0F, 0.0F}, {0.0F, 1.0F, 0.0F});
    const m::Mat4 proj = m::Mat4::perspectiveRh(kFovYRadians, aspect, 0.1F, 100.0F);
    const m::Mat4 mvp = proj * view * model;

    // Keep the animated background, darker than before so the cube stands out.
    renderer::RenderingDesc rendering{};
    rendering.color = ClearColorAt(m_clock.elapsed());
    rendering.color.r *= 0.5F;
    rendering.color.g *= 0.5F;
    rendering.color.b *= 0.5F;
    rendering.clearDepth = 1.0F;

    if (auto r = cb.BeginRendering(m_swapchain, frame.imageIndex, rendering); !r) {
        return fail("begin rendering.md", r.error());
    }
    if (auto r = cb.BindPipeline(m_pipeline); !r) {
        return fail("bind pipeline", r.error());
    }
    if (auto r = cb.BindVertexBuffer(m_vertexBuffer); !r) {
        return fail("bind vertex buffer", r.error());
    }
    if (auto r = cb.BindIndexBuffer(m_indexBuffer, renderer::IndexType::U16); !r) {
        return fail("bind index buffer", r.error());
    }
    if (auto r = cb.PushConstants(m_pipeline, mvp.data(), renderer::kColoredMeshPushConstantSize); !r) {
        return fail("push constants", r.error());
    }
    if (auto r = cb.DrawIndexed(static_cast<std::uint32_t>(std::size(renderer::kCubeIndices))); !r) {
        return fail("draw", r.error());
    }
    if (auto r = cb.EndRendering(m_swapchain, frame.imageIndex); !r) {
        return fail("end rendering.md", r.error());
    }
    if (auto r = cb.End(); !r) {
        return fail("end", r.error());
    }
    return {};
}

void ViewportRenderer::recoverFromDeviceLoss(const QString& reason)
{
    if (++m_recoveryAttempts > kMaxRecoveryAttempts) {
        failRendering(QStringLiteral("device lost again right after recovering %1 times: %2")
                          .arg(kMaxRecoveryAttempts)
                          .arg(reason));
        return;
    }
    qWarning().noquote() << "editor: device lost (" << reason << ") - rebuilding the renderer, attempt"
                         << m_recoveryAttempts;

    m_recovering = true;
    shutdown(); // WaitIdle() on a lost device just fails; everything is destroyed regardless
    const auto result = initialize();
    m_recovering = false;

    if (!result) {
        failRendering(QStringLiteral("could not recover from device loss: %1").arg(result.error()));
        return;
    }
    ++m_deviceRecoveries;
    qInfo().noquote() << "editor: recovered from device loss on" << deviceName();
    emit swapchainRebuilt(swapchainExtent(), m_swapchain.GetImageCount());
    emit deviceRecovered();
}

void ViewportRenderer::renderFrame()
{
    if (!isInitialized() || !m_frameSync.IsValid() || m_renderingFailed) {
        return;
    }

    auto begin = m_frameSync.BeginFrame(m_swapchain);
    if (!begin) {
        if (begin.error().code == renderer::RenderErrorCode::DeviceLost) {
            recoverFromDeviceLoss(ToQString(begin.error()));
            return;
        }
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
    // back, so a recording failure ends rendering.md instead of skipping ahead.
    renderer::Frame& frame = begin->frame;

    if (auto recorded = recordFrame(frame); !recorded) {
        failRendering(recorded.error());
        return;
    }

    auto end = m_frameSync.EndFrame(m_swapchain, frame);
    if (!end) {
        if (end.error().code == renderer::RenderErrorCode::DeviceLost) {
            recoverFromDeviceLoss(ToQString(end.error()));
            return;
        }
        failRendering(QStringLiteral("end frame: %1").arg(ToQString(end.error())));
        return;
    }

    m_recoveryAttempts = 0; // a frame made it all the way: the device is healthy again
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
