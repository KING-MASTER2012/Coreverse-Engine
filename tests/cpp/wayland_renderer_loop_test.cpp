// Faz 5 (Wayland milestone) proof: the exact same single-shot render
// loop render_loop_test.cpp already proved against Xlib (acquire ->
// clear -> submit -> present), but through a real wl_surface backed by
// a running Wayland compositor instead. This is a *separate* test
// binary rather than another #elif branch in render_loop_test.cpp: now
// that Xlib and Wayland can both be compiled into `renderer` at once
// (see renderer/CMakeLists.txt), a single process picking one native
// windowing system at compile time no longer mirrors how a Linux build
// actually ships — this test specifically exercises the Wayland path
// end to end, independent of whether Xlib is also enabled.
//
// Unlike the Xlib dummy window (which opens the existing X server via
// XOpenDisplay(nullptr), e.g. Xvfb in CI), there is no display-less
// libwayland call that hands back a usable compositor: wl_display_connect()
// requires an actual compositor listening on WAYLAND_DISPLAY. This test
// does not start one itself — see run-with-weston-headless.sh,
// which wraps the test binary in CMakeLists.txt's add_test() and runs
// a throwaway headless Weston instance around it for exactly that reason.

#include <cstdio>
#include <cstring>
#include <memory>
#include <utility>
#include <wayland-client.h>

#include "renderer/RenderDeviceFactory.hpp"

namespace {

struct DummyWaylandWindow {
    wl_display* display = nullptr;
    wl_registry* registry = nullptr;
    wl_compositor* compositor = nullptr;
    wl_surface* surface = nullptr;

    static void
    RegistryGlobal(void* data, wl_registry* registry, uint32_t name, const char* interface, uint32_t /*version*/)
    {
        auto* self = static_cast<DummyWaylandWindow*>(data);
        // version 1 is enough for wl_compositor_create_surface(), the
        // only compositor entry point this dummy window needs.
        if (std::strcmp(interface, wl_compositor_interface.name) == 0) {
            self->compositor =
                static_cast<wl_compositor*>(wl_registry_bind(registry, name, &wl_compositor_interface, 1));
        }
    }

    static void RegistryGlobalRemove(void* /*data*/, wl_registry* /*registry*/, uint32_t /*name*/)
    {
        // No global we care about is ever removed mid-test (the test
        // is a few milliseconds of a freshly started compositor's
        // life), so there's nothing to react to here — the listener
        // still needs a non-null callback pointer, though.
    }

    DummyWaylandWindow()
    {
        // nullptr -> read from the WAYLAND_DISPLAY env var, which
        // run-with-weston-headless.sh sets to the socket name
        // of the throwaway compositor it just started.
        display = wl_display_connect(nullptr);
        if (display == nullptr) {
            return;
        }

        registry = wl_display_get_registry(display);
        static constexpr wl_registry_listener listener{&RegistryGlobal, &RegistryGlobalRemove};
        wl_registry_add_listener(registry, &listener, this);

        // Round-trips once: sends the get_registry request and blocks
        // until the compositor's reply (the global announcements,
        // handled by RegistryGlobal above) has been fully processed.
        wl_display_roundtrip(display);

        if (compositor == nullptr) {
            return;
        }
        surface = wl_compositor_create_surface(compositor);
    }

    ~DummyWaylandWindow()
    {
        if (surface != nullptr) {
            wl_surface_destroy(surface);
        }
        if (compositor != nullptr) {
            wl_compositor_destroy(compositor);
        }
        if (registry != nullptr) {
            wl_registry_destroy(registry);
        }
        if (display != nullptr) {
            wl_display_disconnect(display);
        }
    }

    [[nodiscard]] bool IsValid() const
    {
        return display != nullptr && compositor != nullptr && surface != nullptr;
    }

    [[nodiscard]] renderer::NativeWindowHandle ToNativeHandle() const
    {
        renderer::NativeWindowHandle handle{};
        handle.waylandDisplay = display;
        handle.waylandSurface = surface;
        return handle;
    }

    // A wl_surface has no inherent size until something (an xdg_toplevel
    // role, in a real app) assigns it one; Vulkan's WSI extension for
    // Wayland doesn't query the compositor for a size either — the
    // swapchain's extent is simply whatever the caller asks for in
    // SwapchainDesc, same as every other platform here.
    static constexpr std::uint32_t width = 320;
    static constexpr std::uint32_t height = 240;
};

} // namespace

int main()
{
    DummyWaylandWindow window;
    if (!window.IsValid()) {
        std::fprintf(
            stderr,
            "failed to connect to a Wayland compositor / create a wl_surface "
            "(is WAYLAND_DISPLAY set to a running compositor?)\n"
        );
        return 1;
    }

    auto deviceResult = renderer::CreateRenderDevice(renderer::GraphicsAPI::Vulkan);
    if (!deviceResult) {
        std::fprintf(stderr, "CreateRenderDevice failed: %s\n", deviceResult.error().detail.c_str());
        return 1;
    }
    std::unique_ptr<renderer::RenderDevice> device = std::move(*deviceResult);

    {
        auto surfaceResult = device->CreateSurface(window.ToNativeHandle());
        if (!surfaceResult) {
            std::fprintf(stderr, "CreateSurface failed: %s\n", surfaceResult.error().detail.c_str());
            device->Shutdown();
            return 1;
        }
        renderer::Surface surface = std::move(*surfaceResult);

        renderer::SwapchainDesc swapchainDesc{};
        swapchainDesc.preferredImageCount = 2;
        swapchainDesc.width = DummyWaylandWindow::width;
        swapchainDesc.height = DummyWaylandWindow::height;

        auto swapchainResult = device->CreateSwapchain(surface, swapchainDesc);
        if (!swapchainResult) {
            std::fprintf(stderr, "CreateSwapchain failed: %s\n", swapchainResult.error().detail.c_str());
            device->Shutdown();
            return 1;
        }
        renderer::Swapchain swapchain = std::move(*swapchainResult);

        auto acquireResult = swapchain.Acquire();
        if (!acquireResult) {
            std::fprintf(stderr, "Acquire failed: %s\n", acquireResult.error().detail.c_str());
            device->Shutdown();
            return 1;
        }

        void* imageHandle = swapchain.GetImageNativeHandle(acquireResult->imageIndex);
        if (imageHandle == nullptr) {
            std::fprintf(stderr, "GetImageNativeHandle returned null for a just-acquired image\n");
            device->Shutdown();
            return 1;
        }

        auto commandBufferResult = device->AcquireCommandBuffer();
        if (!commandBufferResult) {
            std::fprintf(stderr, "AcquireCommandBuffer failed: %s\n", commandBufferResult.error().detail.c_str());
            device->Shutdown();
            return 1;
        }
        renderer::CommandBuffer commandBuffer = *commandBufferResult;

        if (auto result = commandBuffer.Begin(); !result) {
            std::fprintf(stderr, "CommandBuffer::Begin failed: %s\n", result.error().detail.c_str());
            device->Shutdown();
            return 1;
        }

        // Same accent color render_loop_test.cpp uses for its Xlib
        // window — this test proves the same pipeline works, not a
        // different one, so there's no reason for it to look different.
        constexpr renderer::ClearColor color{0.10f, 0.45f, 0.85f, 1.0f};
        if (auto result = commandBuffer.ClearColor(imageHandle, color); !result) {
            std::fprintf(stderr, "CommandBuffer::ClearColor failed: %s\n", result.error().detail.c_str());
            device->Shutdown();
            return 1;
        }

        if (auto result = commandBuffer.End(); !result) {
            std::fprintf(stderr, "CommandBuffer::End failed: %s\n", result.error().detail.c_str());
            device->Shutdown();
            return 1;
        }

        if (auto result = device->Submit(commandBuffer, nullptr, nullptr, nullptr); !result) {
            std::fprintf(stderr, "Submit failed: %s\n", result.error().detail.c_str());
            device->Shutdown();
            return 1;
        }

        device->WaitIdle();

        auto presentResult = swapchain.Present(acquireResult->imageIndex);
        if (!presentResult) {
            std::fprintf(stderr, "Present failed: %s\n", presentResult.error().detail.c_str());
            device->Shutdown();
            return 1;
        }

        device->WaitIdle();

        std::printf(
            "cleared to (%.2f, %.2f, %.2f) and presented via Wayland — closing\n",
            static_cast<double>(color.r),
            static_cast<double>(color.g),
            static_cast<double>(color.b)
        );

        // `swapchain` destructs here (declared after `surface`), then
        // `surface` — same ordering rule as render_loop_test.cpp.
    }

    device->Shutdown();
    return 0;
}
