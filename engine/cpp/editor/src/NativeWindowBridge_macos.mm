// Phase 6.2, macOS: QWidget::winId() is an NSView*. Vulkan (MoltenVK)
// presents into a CAMetalLayer, so the view is made layer-backed with one
// — the same thing Qt's own QVulkanInstance does for a QWindow. This is
// Objective-C++ (compiled with ARC, see CMakeLists.txt) because there is
// no way to talk to AppKit/QuartzCore from plain C++.

#include "NativeWindowBridge.hpp"

#include <QWidget>

#import <AppKit/AppKit.h>
#import <QuartzCore/CAMetalLayer.h>

namespace editor {

struct NativeWindowBridge::Impl {
    CAMetalLayer* layer = nil; // kept alive here; the NSView retains it too
};

NativeWindowBridge::NativeWindowBridge() : m_impl(std::make_unique<Impl>()) {}

NativeWindowBridge::~NativeWindowBridge() = default;

std::expected<std::unique_ptr<NativeWindowBridge>, QString> NativeWindowBridge::create(QWidget& widget)
{
    NSView* view = (__bridge NSView*)reinterpret_cast<void*>(widget.winId());
    if (view == nil) {
        return std::unexpected(QStringLiteral("QWidget::winId() returned 0: the widget has no native window"));
    }

    CAMetalLayer* layer = nil;
    if ([view.layer isKindOfClass:[CAMetalLayer class]]) {
        layer = (CAMetalLayer*)view.layer;
    } else {
        layer = [CAMetalLayer layer];
        view.wantsLayer = YES;
        view.layer = layer;
    }

    auto bridge = std::unique_ptr<NativeWindowBridge>(new NativeWindowBridge());
    bridge->m_impl->layer = layer;
    bridge->m_handle.metalLayer = (__bridge void*)layer;
    bridge->pixelSizeChanged(widget.size() * widget.devicePixelRatioF(), widget.devicePixelRatioF());
    return bridge;
}

void NativeWindowBridge::pixelSizeChanged(const QSize& pixelSize, qreal devicePixelRatio) noexcept
{
    m_impl->layer.contentsScale = devicePixelRatio;
    m_impl->layer.drawableSize = CGSizeMake(pixelSize.width(), pixelSize.height());
}

} // namespace editor
