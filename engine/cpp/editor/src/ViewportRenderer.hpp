#pragma once

#include <QObject>
#include <QSize>
#include <QString>
#include <cstdint>
#include <expected>
#include <memory>

#include <QTimer>

#include "renderer/RenderDevice.hpp"
#include "renderer/Surface.hpp"
#include "renderer/Swapchain.hpp"

namespace editor {

class NativeWindowBridge;
class ViewportWidget;

/// Phase 6.3: owns everything the renderer needs to present into one
/// ViewportWidget — the RenderDevice, the Surface on the widget's native
/// window and the Swapchain on that surface — and keeps the swapchain in
/// step with the widget's size.
///
/// Lifetime rules (renderer/RenderDevice.hpp): a Swapchain must die before
/// its Surface, a Surface before the device's Shutdown(), and the native
/// window (and, on X11, the Display* the bridge holds) must outlive the
/// Surface. shutdown() enforces exactly that order, and the members are
/// declared so that the destructor's automatic teardown follows it too.
///
/// This class does not render anything yet: acquiring, recording and
/// presenting frames is Phase 6.4.
class ViewportRenderer : public QObject
{
    Q_OBJECT

public:
    /// `viewport` must outlive this object (EditorWindow owns both, and
    /// destroys the renderer first).
    explicit ViewportRenderer(ViewportWidget& viewport, QObject* parent = nullptr);
    ~ViewportRenderer() override;

    ViewportRenderer(const ViewportRenderer&) = delete;
    ViewportRenderer& operator=(const ViewportRenderer&) = delete;

    /// Creates the device, the surface and the swapchain, in that order.
    /// On failure everything already created is torn down again and the
    /// reason is returned; the renderer stays usable for a retry. Calling
    /// it while already initialized is a no-op.
    [[nodiscard]] std::expected<void, QString> initialize();

    /// Tears everything down in the order the renderer requires
    /// (swapchain, surface, device, then the native window bridge).
    /// Idempotent; the destructor calls it too.
    void shutdown() noexcept;

    [[nodiscard]] bool isInitialized() const noexcept
    {
        return m_swapchain.IsValid();
    }

    [[nodiscard]] QString deviceName() const;

    /// Actual size of the swapchain's images — what the surface gave, not
    /// necessarily the widget's size at that instant (resizes are applied
    /// one event-loop turn later, see scheduleSwapchainRebuild()).
    [[nodiscard]] QSize swapchainExtent() const noexcept;

    [[nodiscard]] std::uint32_t swapchainImageCount() const noexcept
    {
        return m_swapchain.GetImageCount();
    }

    /// Rebuilds the swapchain for the widget's current size right now.
    /// A minimized/zero-sized viewport is not an error: the old swapchain
    /// is kept until the widget has a size again. Normally called through
    /// the resize timer; public so callers (and tests) can force it.
    [[nodiscard]] std::expected<void, QString> rebuildSwapchain();

public slots:
    /// Coalesces a burst of resizes (a window drag sends dozens) into a
    /// single rebuild on the next event-loop turn.
    void scheduleSwapchainRebuild();

signals:
    /// The swapchain was rebuilt; carries its new size and image count.
    void swapchainRebuilt(const QSize& extent, quint32 imageCount);

    /// A rebuild failed with something other than "the viewport has no
    /// size right now". The swapchain is unusable until a later rebuild
    /// succeeds.
    void swapchainRebuildFailed(const QString& reason);

private:
    [[nodiscard]] renderer::SwapchainDesc makeSwapchainDesc() const;

    ViewportWidget& m_viewport;
    QTimer m_resizeTimer;

    // Declaration order is teardown order in reverse: the swapchain goes
    // first, then the surface, then the device, and the bridge — which
    // owns the platform resource the surface points at — goes last.
    std::unique_ptr<NativeWindowBridge> m_bridge;
    std::unique_ptr<renderer::RenderDevice> m_device;
    renderer::Surface m_surface;
    renderer::Swapchain m_swapchain;
};

} // namespace editor
