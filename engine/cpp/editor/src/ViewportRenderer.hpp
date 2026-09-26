#pragma once

#include <QElapsedTimer>
#include <QObject>
#include <QSize>
#include <QString>
#include <cstdint>
#include <expected>
#include <memory>

#include <QTimer>

#include "renderer/FrameSync.hpp"
#include "renderer/RenderDevice.hpp"
#include "renderer/Surface.hpp"
#include "renderer/Swapchain.hpp"

namespace cv_ffi {
class Logger;
}

namespace editor {

class NativeWindowBridge;
class ViewportWidget;

/// Owns everything the renderer needs to present into one ViewportWidget —
/// the RenderDevice, the Surface on the widget's native window, the
/// Swapchain on that surface (Phase 6.3) and the FrameSync that keeps
/// frames in flight (Phase 6.4) — keeps the swapchain in step with the
/// widget's size, and drives the render loop.
///
/// The render loop (Phase 6.4) is a ~60 Hz timer on the GUI thread; each
/// tick renders one frame: FrameSync::BeginFrame(), record a clear to a
/// slowly cycling color, FrameSync::EndFrame(). It rests whenever nothing
/// could be seen (viewport hidden, window minimized) and whenever the
/// swapchain has to be rebuilt but cannot be yet (zero-sized viewport, a
/// failed rebuild — those retry on the next resize and on a slow timer).
/// Nothing but a clear is drawn: pipelines and draw calls are later phases.
///
/// Lifetime rules (renderer/RenderDevice.hpp): a FrameSync and a Swapchain
/// must die before the Surface they present to, a Surface before the
/// device's Shutdown(), and the native window (and, on X11, the Display* the
/// bridge holds) must outlive the Surface. shutdown() enforces exactly that
/// order, and the members are declared so that the destructor's automatic
/// teardown follows it too.
class ViewportRenderer : public QObject
{
    Q_OBJECT

public:
    /// `viewport` must outlive this object (EditorWindow owns both, and
    /// destroys the renderer first). `logger`, if non-null, must also
    /// outlive this object — every validation/log message the renderer
    /// produces while initialized is routed to it (Phase 7c; see
    /// initialize()'s doc comment) — which EditorWindow's member
    /// declaration order guarantees the same way it already guarantees
    /// `viewport`'s. Pass nullptr (the default) to skip that routing
    /// entirely — messages still reach the renderer's own stderr output
    /// and renderer::ValidationErrorCount() either way.
    explicit ViewportRenderer(ViewportWidget& viewport, cv_ffi::Logger* logger = nullptr, QObject* parent = nullptr);
    ~ViewportRenderer() override;

    ViewportRenderer(const ViewportRenderer&) = delete;
    ViewportRenderer& operator=(const ViewportRenderer&) = delete;

    /// Creates the device, the surface, the swapchain and the frame
    /// synchronization, in that order, and starts the render loop. On
    /// failure everything already created is torn down again and the reason
    /// is returned; the renderer stays usable for a retry. Calling it while
    /// already initialized is a no-op.
    ///
    /// If this ViewportRenderer was constructed with a non-null `logger`,
    /// also installs a RenderDevice::LogCallback (renderer/RenderDevice.hpp)
    /// on the new device that turns every backend log/validation message
    /// into a diagnostic emitted through that Logger — producer
    /// `"CV-RENDERER"`, category `"VULKAN"` (see RegisterProducer() having
    /// been called for that code — EditorWindow does this once at startup,
    /// not here, since it's a process-global registration, not a per-device
    /// one). shutdown() clears the callback again before the device goes
    /// away.
    [[nodiscard]] std::expected<void, QString> initialize();

    /// Stops the loop and tears everything down in the order the renderer
    /// requires (log callback, frame sync, swapchain, surface, device, then
    /// the native window bridge). Idempotent; the destructor calls it too.
    void shutdown() noexcept;

    [[nodiscard]] bool isInitialized() const noexcept
    {
        return m_swapchain.IsValid();
    }

    [[nodiscard]] QString deviceName() const;

    /// Whether the graphics API's validation layer is running on this device.
    [[nodiscard]] bool validationEnabled() const noexcept;

    /// Actual size of the swapchain's images — what the surface gave, not
    /// necessarily the widget's size at that instant (resizes are applied
    /// one event-loop turn later, see scheduleSwapchainRebuild()).
    [[nodiscard]] QSize swapchainExtent() const noexcept;

    [[nodiscard]] std::uint32_t swapchainImageCount() const noexcept
    {
        return m_swapchain.GetImageCount();
    }

    /// Frames rendered and presented since initialize().
    [[nodiscard]] quint64 framesRendered() const noexcept
    {
        return m_framesRendered;
    }

    /// Whether the render loop is currently ticking (as opposed to resting
    /// or stopped for good after an error).
    [[nodiscard]] bool isRendering() const noexcept
    {
        return m_frameTimer.isActive();
    }

    /// Rebuilds the swapchain for the widget's current size right now.
    /// Returns true if it was rebuilt and false if there was nothing to do:
    /// the viewport has no size right now (hidden, collapsed, minimized —
    /// the old swapchain is kept until it has one again), or it already has
    /// the right size. `force` skips the "already the right size" shortcut,
    /// for when the swapchain has to be rebuilt regardless of size
    /// (SwapchainStatus::OutOfDate). Normally called through the resize
    /// timer; public so callers (and tests) can force it.
    [[nodiscard]] std::expected<bool, QString> rebuildSwapchain(bool force = false);

public slots:
    /// Coalesces a burst of resizes (a window drag sends dozens) into a
    /// single rebuild on the next event-loop turn.
    void scheduleSwapchainRebuild();

    /// The top-level window was minimized/restored: nothing to draw for
    /// while minimized. (Hiding the viewport itself is noticed on its own.)
    void setWindowMinimized(bool minimized);

signals:
    /// The swapchain was rebuilt; carries its new size and image count.
    void swapchainRebuilt(const QSize& extent, quint32 imageCount);

    /// A rebuild failed with something other than "the viewport has no
    /// size right now". The render loop rests until a later rebuild
    /// succeeds (retried on the next resize and every few hundred ms).
    void swapchainRebuildFailed(const QString& reason);

    /// One frame was rendered and presented; carries the running total.
    void frameRendered(quint64 totalFrames);

    /// Roughly once a second while rendering: frames per second over the
    /// last interval and the running total.
    void frameStatsUpdated(double framesPerSecond, quint64 totalFrames);

    /// Rendering stopped for good: the device or surface is unusable (device
    /// lost, out of memory, a failed submit/present). Recovery — rebuilding
    /// the whole renderer — is not implemented yet; the editor keeps
    /// running without a viewport image.
    void renderingFailed(const QString& reason);

private slots:
    void renderFrame();

private:
    [[nodiscard]] renderer::SwapchainDesc makeSwapchainDesc() const;

    /// Starts or stops the frame timer according to the state flags below.
    void updateFrameTimer();

    /// rebuildSwapchain() plus the loop bookkeeping around it: a rebuild that
    /// has to happen (`force`) but cannot yet — no size, or an error — puts
    /// the loop to rest until one succeeds.
    void rebuildAndUpdate(bool force);

    /// Stops rendering for good and tells the world why.
    void failRendering(const QString& reason);

    ViewportWidget& m_viewport;
    cv_ffi::Logger* m_logger = nullptr; // not owned; see the constructor's doc comment
    QTimer m_resizeTimer;
    QTimer m_frameTimer;
    QTimer m_retryTimer;
    QElapsedTimer m_clock;      ///< Drives the clear color animation.
    QElapsedTimer m_statsClock; ///< Measures the interval for frameStatsUpdated().
    quint64 m_framesRendered = 0;
    quint64 m_framesAtLastStats = 0;

    bool m_viewportVisible = false;
    bool m_windowMinimized = false;
    bool m_awaitingRebuild = false; ///< The swapchain cannot present until a rebuild succeeds.
    bool m_renderingFailed = false;

    // Declaration order is teardown order in reverse: the frame sync goes
    // first, then the swapchain, the surface, the device, and the bridge —
    // which owns the platform resource the surface points at — goes last.
    std::unique_ptr<NativeWindowBridge> m_bridge;
    std::unique_ptr<renderer::RenderDevice> m_device;
    renderer::Surface m_surface;
    renderer::Swapchain m_swapchain;
    renderer::FrameSync m_frameSync;
};

} // namespace editor
