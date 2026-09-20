# PROGRESS.md
Last changed 20.09.2026 by KING-MASTER2012.

## What happened compared to the previous commit?
- added (Phase 6.2): the native-window bridge. `ViewportWidget` is a QWidget with its own native window (Qt::WA_NativeWindow + WA_PaintOnScreen) that the renderer presents into; `NativeWindowBridge` turns it into a `renderer::NativeWindowHandle` - Windows: HWND/HINSTANCE, Linux: X11 Window + an Xlib Display* (Qt's "xcb" platform, i.e. X11 or XWayland), macOS: a CAMetalLayer set on the NSView (Objective-C++, NativeWindowBridge_macos.mm).
- added (Phase 6.3): the editor links `renderer`. `ViewportRenderer` owns the device, surface and swapchain of the viewport, is started from EditorWindow::showEvent (queued) and shut down in closeEvent in the order the renderer requires (swapchain, surface, device, native window). Resizes are coalesced into one swapchain rebuild per event-loop turn. A missing/unsupported Vulkan setup is not fatal: the editor opens and the status bar says why.
- added: `Swapchain::Recreate()` (rebuild in place on resize, hands the old swapchain to Vulkan), `Swapchain::GetExtent()`, `Extent2D`, `RenderErrorCode::ZeroExtent` (a minimized window is not an error) and `RenderDevice::RebuildSwapchain()` for backends. The Vulkan swapchain creation was split into query / create / destroy helpers. swapchain_test now resizes its window twice and checks the rebuilt extent.
- note: nothing is rendered in the viewport yet (Phase 6.4).
- note: on Linux only Qt's "xcb" platform is supported; under a native Wayland Qt platform the editor shows a message telling to run with QT_QPA_PLATFORM=xcb.

## What will be done?
- Phase 6.4: frame synchronization abstraction (frames in flight) and a real render loop in the viewport.
- Check that the vcpkg Qt build on Linux ships the xcb platform plugin (the editor needs it to open a window there).
- Native Wayland support for the editor viewport (needs Qt's private wl_surface accessor).
- Phase 7c: VFS FFI extensions (list_dir/metadata/remove), C++ RAII wrappers, renderer logging through the ffi logger.
- Debug bootstrap.sh on macOS (needs the Bootstrap step's log).
- Asset system will be added.
- IO system will be added.
