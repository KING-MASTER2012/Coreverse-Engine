// Phase 6.2, Linux: Qt's "xcb" platform (X11, or XWayland). QWidget::winId()
// is the X11 Window; the renderer's Xlib surface also wants a Display*,
// which Qt's xcb connection does not hand out, so the bridge opens its own
// connection to the same X server. Window ids are server-side, so a window
// created through Qt's connection is valid on this one too.

#include <QGuiApplication>
#include <QWidget>

#include "NativeWindowBridge.hpp"

// After every Qt header on purpose: Xlib defines plain macros (Bool,
// Status, None, ...) that break Qt's own headers if they are parsed first.
#include <X11/Xlib.h>

namespace editor {

struct NativeWindowBridge::Impl {
    Display* display = nullptr;

    ~Impl()
    {
        if (display != nullptr) {
            XCloseDisplay(display);
        }
    }
};

NativeWindowBridge::NativeWindowBridge() : m_impl(std::make_unique<Impl>()) {}

NativeWindowBridge::~NativeWindowBridge() = default;

std::expected<std::unique_ptr<NativeWindowBridge>, QString> NativeWindowBridge::create(QWidget& widget)
{
    const QString platform = QGuiApplication::platformName();
    if (platform != QLatin1String("xcb")) {
        return std::unexpected(
            QStringLiteral(
                "the Vulkan viewport needs Qt's 'xcb' platform (X11, or XWayland) on Linux, but this session "
                "uses '%1' — run the editor with QT_QPA_PLATFORM=xcb"
            )
                .arg(platform)
        );
    }

    const WId windowId = widget.winId();
    if (windowId == 0) {
        return std::unexpected(QStringLiteral("QWidget::winId() returned 0: the widget has no native window"));
    }

    auto bridge = std::unique_ptr<NativeWindowBridge>(new NativeWindowBridge());

    // nullptr = $DISPLAY, the same server Qt's xcb platform just connected to.
    bridge->m_impl->display = XOpenDisplay(nullptr);
    if (bridge->m_impl->display == nullptr) {
        return std::unexpected(QStringLiteral("XOpenDisplay failed: cannot open a second connection to the X server"));
    }

    bridge->m_handle.xlibDisplay = bridge->m_impl->display;
    bridge->m_handle.xlibWindow = static_cast<unsigned long>(windowId);
    return bridge;
}

void NativeWindowBridge::pixelSizeChanged(const QSize& /*pixelSize*/, qreal /*devicePixelRatio*/) noexcept
{
    // The Xlib surface reads the window's geometry from the server itself.
}

} // namespace editor
