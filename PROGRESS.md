# PROGRESS.md
Last changed 21.09.2026 by KING-MASTER2012.

## What happened compared to the previous commit?
- added (Phase 6.4): `renderer::FrameSync` (renderer/FrameSync.hpp) - the frame-level synchronization abstraction. `RenderDevice::CreateFrameSync()` builds it; `BeginFrame(swapchain)` waits for the in-flight slot's previous frame, acquires an image (100 ms timeout, so a hidden window can never block the GUI thread) and hands out a reset command buffer; `EndFrame(swapchain, frame)` submits and presents. Semaphores/fences stay inside the backend (acquire semaphore + fence + command buffer per in-flight slot, render-finished semaphore per swapchain image); a rebuilt swapchain is noticed by the next `BeginFrame()`, no extra call. Vulkan implementation in VulkanRenderDevice.cpp; `framesInFlight` is clamped to [1, 8].
- added (Phase 6.4): the editor render loop. `ViewportRenderer` owns a `FrameSync` (after the swapchain, so it dies first) and renders from a ~60 Hz timer on the GUI thread: a clear to a slowly cycling color, FPS in the status bar. The loop rests while the viewport is hidden or the window is minimized, and while the swapchain cannot be rebuilt yet (zero-sized viewport, failed rebuild - retried on the next resize and every 250 ms). `rebuildSwapchain(force)` no longer skips a rebuild just because the size is unchanged, so `OutOfDate` is always handled. Fatal errors (device lost, failed submit/present) stop rendering and show up in the status bar; recovering from them is not implemented.
- added: `Swapchain::Acquire(semaphore, timeoutNs)` and `SwapchainStatus::NotReady` (timeout expired, nothing acquired). `VK_ERROR_DEVICE_LOST` / `VK_ERROR_OUT_OF_*_MEMORY` now map to `RenderErrorCode::DeviceLost` / `OutOfMemory` for acquire, present, submit and begin/end instead of `Unknown`.
- added: validation as a test signal. `renderer::ValidationErrorCount()` (renderer/Diagnostics.hpp) counts validation-layer errors, `RenderDevice::IsValidationEnabled()` says whether the layer is running, `RENDERER_FORCE_VALIDATION` (CMake option) enables the layer in every build type. Linux CI installs `vulkan-validationlayers`, configures with the option and sets `CV_REQUIRE_VALIDATION=1`, which makes the tests fail if the layer is not actually running.
- added: `frame_sync_test` (gpu): 1, 2 and more-frames-than-images in flight, ~140 frames each with a resize in the middle, Acquire timeout, misuse checks, move semantics, zero validation errors. `editor --smoke-frames N` + ctest `editor_render_smoke` (labels `gpu;editor`): the real Qt window renders N frames, is resized half-way, quits by itself and must find no validation errors. CI runs it in its own non-blocking step (`continue-on-error`, like macOS) because the vcpkg Qt xcb plugin is unverified there.
- note: `AcquireCommandBuffer()` allocates a new command buffer on every call and only frees them at `Shutdown()` - use it once per long-lived buffer; per-frame buffers come from `FrameSync`. Documented on the function.
- note: `Submit()` still waits at the TRANSFER stage (right for a clear). The first real graphics pass must change it to COLOR_ATTACHMENT_OUTPUT (comment in `VulkanRenderDevice::Submit`).
- note: nothing is drawn in the viewport except the clear color yet.

## What will be done?
- Check that the vcpkg Qt build on Linux ships the xcb platform plugin (the editor and its smoke test need it to open a window there); then drop `continue-on-error` from the CI editor smoke step.
- A multi-frame variant of the Wayland render loop test (needs a look at how Weston's frame callbacks interact with acquire).
- Native Wayland support for the editor viewport (needs Qt's private wl_surface accessor).
- Device-lost recovery for the editor (rebuild device, surface, swapchain and frame sync).
- First real graphics pass (pipeline + draw); render thread when the GUI thread becomes the bottleneck.
- Phase 7c: VFS FFI extensions (list_dir/metadata/remove), C++ RAII wrappers, renderer logging through the ffi logger (the validation callback and `ValidationErrorCount()` are the place to hook it into), VfsContext re-init decision.
- Debug bootstrap.sh on macOS (needs the Bootstrap step's log).
- Asset system will be added.
- IO system will be added.
