#include "ViewportWidget.hpp"

#include <QHideEvent>
#include <QResizeEvent>
#include <QShowEvent>
#include <cmath>

namespace editor {

    ViewportWidget::ViewportWidget(QWidget* parent) : QWidget(parent)
    {
        setAttribute(Qt::WA_NativeWindow);
        setAttribute(Qt::WA_PaintOnScreen);
        setAttribute(Qt::WA_NoSystemBackground);
        setAttribute(Qt::WA_OpaquePaintEvent);

        // Small enough not to fight any layout, big enough that a splitter
        // can never squeeze the swapchain down to nothing by accident.
        setMinimumSize(64, 64);
    }

    QSize ViewportWidget::pixelSize() const
    {
        const qreal ratio = devicePixelRatioF();
        return QSize(static_cast<int>(std::lround(width() * ratio)), static_cast<int>(std::lround(height() * ratio)));
    }

    QPaintEngine* ViewportWidget::paintEngine() const
    {
        return nullptr;
    }

    void ViewportWidget::resizeEvent(QResizeEvent* event)
    {
        QWidget::resizeEvent(event);
        emit pixelSizeChanged(pixelSize());
    }

    void ViewportWidget::showEvent(QShowEvent* event)
    {
        QWidget::showEvent(event);
        emit visibleChanged(true);
    }

    void ViewportWidget::hideEvent(QHideEvent* event)
    {
        QWidget::hideEvent(event);
        emit visibleChanged(false);
    }

} // namespace editor
