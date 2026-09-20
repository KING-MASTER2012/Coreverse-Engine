#pragma once

#include <QSize>
#include <QString>
#include <expected>
#include <memory>

#include "renderer/Surface.hpp"

class QWidget;

namespace editor {

/// Phase 6.2: turns a Qt widget's native window into the
/// renderer::NativeWindowHandle that RenderDevice::CreateSurface()
/// consumes — the "native-window handle bridge" renderer/Surface.hpp
/// has been waiting for since Phase 5.3.
///
/// The bridge exists because a QWidget::winId() alone is not what the
/// renderer needs on every platform:
///
///   Windows  winId() is the HWND; the HINSTANCE is the module's.
///   Linux    Only Qt's "xcb" platform (X11, or XWayland inside a
///            Wayland session) is supported: winId() is the X11 Window,
///            and the renderer's Xlib surface additionally needs a
///            Display*, which the bridge opens (and closes again).
///            Qt's native "wayland" platform is not supported yet — it
///            would need Qt's private wl_surface accessor.
///   macOS    winId() is an NSView*; Vulkan (MoltenVK) presents into a
///            CAMetalLayer, so the bridge makes the view layer-backed
///            with one and keeps its drawable size in step.
///
/// The bridge owns whatever platform resource the handle refers to, so
/// it must outlive every Surface created from handle() — destroy the
/// Surface (and Swapchain) first.
class NativeWindowBridge
{
public:
    /// Forces the widget's native window into existence (winId()) and
    /// describes it for the renderer. The widget should have
    /// Qt::WA_NativeWindow set (ViewportWidget does). Fails with a
    /// human-readable reason on an unsupported Qt platform.
    [[nodiscard]] static std::expected<std::unique_ptr<NativeWindowBridge>, QString> create(QWidget& widget);

    ~NativeWindowBridge();

    NativeWindowBridge(const NativeWindowBridge&) = delete;
    NativeWindowBridge& operator=(const NativeWindowBridge&) = delete;
    NativeWindowBridge(NativeWindowBridge&&) = delete;
    NativeWindowBridge& operator=(NativeWindowBridge&&) = delete;

    /// Ready to pass to RenderDevice::CreateSurface().
    [[nodiscard]] const renderer::NativeWindowHandle& handle() const noexcept
    {
        return m_handle;
    }

    /// Tells the platform layer that the drawable's size in device
    /// pixels changed. Only macOS has anything to do (the CAMetalLayer's
    /// drawableSize is what MoltenVK sizes the swapchain from); on
    /// Windows and X11 the surface tracks the window by itself.
    void pixelSizeChanged(const QSize& pixelSize, qreal devicePixelRatio) noexcept;

private:
    NativeWindowBridge();

    struct Impl;
    std::unique_ptr<Impl> m_impl;
    renderer::NativeWindowHandle m_handle{};
};

} // namespace editor
