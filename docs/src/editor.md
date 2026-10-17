# Editor

The editor (`engine/cpp/editor`) is a Qt 6 widgets application. Its central widget is a viewport backed by a native
window that the renderer presents into (`NativeWindowBridge`: X11/xcb, Win32, or a `CAMetalLayer` on macOS).

`ViewportRenderer` owns the device, surface, swapchain, frame sync, and the scene resources (a coloured cube's
vertex/index buffers and its pipeline). Every ~16 ms it renders one frame: the cube spins (MVP from `cv-math` in push
constants) over an animated background. It handles resizes (swapchain and depth buffer are rebuilt), minimising,
and device loss (see [Rendering](rendering.md)). On a device without dynamic rendering it falls back to the animated
clear colour.

The editor routes renderer log messages (including Vulkan validation output) into the Rust logger through `cv-ffi`.

## Smoke tests

```bash
editor --smoke-frames 120                        # render 120 frames, resize half-way, exit 0 on success
editor --smoke-frames 120 --simulate-device-loss # also lose the device at frame 30 and require recovery
```

Both are ctest tests (`editor_render_smoke`, `editor_device_loss_smoke`, labels `gpu;editor`); they also fail on any
Vulkan validation error.

## Not there yet

Project open/create, scene and asset browsing, a file log sink (needs a project root), native Wayland viewport
(the editor runs through XWayland/xcb on Linux).
