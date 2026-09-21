// Phase 6.2, Windows: QWidget::winId() already *is* the HWND.

#include <QWidget>

#include "NativeWindowBridge.hpp"

#ifndef NOMINMAX
    #define NOMINMAX
#endif
#ifndef WIN32_LEAN_AND_MEAN
    #define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

namespace editor {

    struct NativeWindowBridge::Impl {};

    NativeWindowBridge::NativeWindowBridge() : m_impl(std::make_unique<Impl>()) {}

    NativeWindowBridge::~NativeWindowBridge() = default;

    std::expected<std::unique_ptr<NativeWindowBridge>, QString> NativeWindowBridge::create(QWidget& widget)
    {
        const WId windowId = widget.winId();
        if (windowId == 0) {
            return std::unexpected(QStringLiteral("QWidget::winId() returned 0: the widget has no native window"));
        }

        auto bridge = std::unique_ptr<NativeWindowBridge>(new NativeWindowBridge());
        bridge->m_handle.hwnd = reinterpret_cast<void*>(windowId);
        bridge->m_handle.hinstance = ::GetModuleHandleW(nullptr);
        return bridge;
    }

    void NativeWindowBridge::pixelSizeChanged(const QSize& /*pixelSize*/, qreal /*devicePixelRatio*/) noexcept
    {
        // The Win32 surface reads the client rectangle itself.
    }

} // namespace editor
