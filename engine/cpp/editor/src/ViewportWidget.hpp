#pragma once

#include <QSize>
#include <QWidget>

class QHideEvent;
class QPaintEngine;
class QResizeEvent;
class QShowEvent;

namespace editor {

/// Phase 6.2: the widget the renderer presents into.
///
/// A plain QWidget normally has no window of its own — it is painted into
/// its top-level window's backing store by Qt — which is useless for
/// Vulkan, whose surface needs a real native window (HWND / X11 Window /
/// NSView). Qt::WA_NativeWindow gives this widget one, and
/// Qt::WA_PaintOnScreen + a null paintEngine() tell Qt to keep its hands
/// off the pixels entirely, so nothing paints over (or flickers under)
/// what the swapchain presents.
///
/// The widget itself knows nothing about the renderer; it only reports its
/// size in device pixels (and, since Phase 6.4, whether it is shown, so the
/// render loop can rest while nobody can see it). NativeWindowBridge
/// describes the native window to the renderer, and ViewportRenderer drives
/// the Vulkan objects.
class ViewportWidget : public QWidget
{
    Q_OBJECT

public:
    explicit ViewportWidget(QWidget* parent = nullptr);

    /// The widget's size in device pixels (logical size * device pixel
    /// ratio) — the size a swapchain for it should have.
    [[nodiscard]] QSize pixelSize() const;

    /// Required by Qt::WA_PaintOnScreen: this widget is never painted by Qt.
    [[nodiscard]] QPaintEngine* paintEngine() const override;

signals:
    /// Emitted from every resize, with the new size in device pixels.
    void pixelSizeChanged(const QSize& pixelSize);

    /// Emitted when the widget is shown or hidden (a hidden tab, a
    /// collapsed dock, a hidden window).
    void visibleChanged(bool visible);

protected:
    void resizeEvent(QResizeEvent* event) override;
    void showEvent(QShowEvent* event) override;
    void hideEvent(QHideEvent* event) override;
};

} // namespace editor
