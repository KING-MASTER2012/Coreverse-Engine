// Faz 5.5 proof: RenderDevice::AcquireCommandBuffer() -> Begin() ->
// clear an acquired swapchain image to a solid color -> End() ->
// Submit() -> Swapchain::Present(). This is a single-shot loop
// (acquire/render/present once, matching the plan's "pencerede düz
// renk gösterilip kapatılır"), so it doesn't need caller-managed
// semaphores — Swapchain::Acquire()/Present() still accept one (see
// Swapchain.hpp) for a real multi-frame render loop to use later;
// this test uses RenderDevice::WaitIdle() between GPU-dependent steps
// instead, which is enough for a single frame and keeps the test on
// the same public abstraction every earlier Faz 5 test used — no
// backend escape hatch needed here.

#include <cstdio>
#include <memory>
#include <utility>

#include "renderer/RenderDeviceFactory.hpp"

#if defined(_WIN32)
#include <windows.h>
#elif defined(__linux__)
#include <X11/Xlib.h>
#endif

namespace
{

#if defined(_WIN32)

struct DummyWindow
{
    HWND hwnd = nullptr;

    DummyWindow()
    {
        WNDCLASSW wc{};
        wc.lpfnWndProc = DefWindowProcW;
        wc.hInstance = GetModuleHandleW(nullptr);
        wc.lpszClassName = L"CoreVerseFaz55DummyWindow";
        RegisterClassW(&wc);

        hwnd = CreateWindowExW(0, wc.lpszClassName, L"CoreVerse Faz 5.5", WS_OVERLAPPEDWINDOW, CW_USEDEFAULT,
                                CW_USEDEFAULT, 320, 240, nullptr, nullptr, wc.hInstance, nullptr);
    }

    ~DummyWindow()
    {
        if (hwnd != nullptr)
        {
            DestroyWindow(hwnd);
        }
    }

    [[nodiscard]] bool IsValid() const
    {
        return hwnd != nullptr;
    }

    [[nodiscard]] renderer::NativeWindowHandle ToNativeHandle() const
    {
        renderer::NativeWindowHandle handle{};
        handle.hinstance = GetModuleHandleW(nullptr);
        handle.hwnd = hwnd;
        return handle;
    }

    static constexpr std::uint32_t width = 320;
    static constexpr std::uint32_t height = 240;
};

#elif defined(__linux__)

struct DummyWindow
{
    Display* display = nullptr;
    Window window = 0;

    DummyWindow()
    {
        display = XOpenDisplay(nullptr);
        if (display == nullptr)
        {
            return;
        }
        const int screen = DefaultScreen(display);
        window = XCreateSimpleWindow(display, RootWindow(display, screen), 0, 0, 320, 240, 0,
                                      BlackPixel(display, screen), WhitePixel(display, screen));
    }

    ~DummyWindow()
    {
        if (display != nullptr)
        {
            if (window != 0)
            {
                XDestroyWindow(display, window);
            }
            XCloseDisplay(display);
        }
    }

    [[nodiscard]] bool IsValid() const
    {
        return display != nullptr && window != 0;
    }

    [[nodiscard]] renderer::NativeWindowHandle ToNativeHandle() const
    {
        renderer::NativeWindowHandle handle{};
        handle.xlibDisplay = display;
        handle.xlibWindow = window;
        return handle;
    }

    static constexpr std::uint32_t width = 320;
    static constexpr std::uint32_t height = 240;
};

#endif

} // namespace

int main()
{
#if !defined(_WIN32) && !defined(__linux__)
    std::fprintf(stderr, "render_loop_test: no dummy window implementation for this platform\n");
    return 1;
#else
    DummyWindow window;
    if (!window.IsValid())
    {
        std::fprintf(stderr, "failed to create dummy test window\n");
        return 1;
    }

    auto deviceResult = renderer::CreateRenderDevice(renderer::GraphicsAPI::Vulkan);
    if (!deviceResult)
    {
        std::fprintf(stderr, "CreateRenderDevice failed: %s\n", deviceResult.error().detail.c_str());
        return 1;
    }
    std::unique_ptr<renderer::RenderDevice> device = std::move(*deviceResult);

    {
        auto surfaceResult = device->CreateSurface(window.ToNativeHandle());
        if (!surfaceResult)
        {
            std::fprintf(stderr, "CreateSurface failed: %s\n", surfaceResult.error().detail.c_str());
            device->Shutdown();
            return 1;
        }
        renderer::Surface surface = std::move(*surfaceResult);

        renderer::SwapchainDesc swapchainDesc{};
        swapchainDesc.preferredImageCount = 2;
        swapchainDesc.width = DummyWindow::width;
        swapchainDesc.height = DummyWindow::height;

        auto swapchainResult = device->CreateSwapchain(surface, swapchainDesc);
        if (!swapchainResult)
        {
            std::fprintf(stderr, "CreateSwapchain failed: %s\n", swapchainResult.error().detail.c_str());
            device->Shutdown();
            return 1;
        }
        renderer::Swapchain swapchain = std::move(*swapchainResult);

        // Synchronous acquire (no semaphore passed) — blocks until an
        // image is ready, same call shape Faz 5.4 already proved.
        auto acquireResult = swapchain.Acquire();
        if (!acquireResult)
        {
            std::fprintf(stderr, "Acquire failed: %s\n", acquireResult.error().detail.c_str());
            device->Shutdown();
            return 1;
        }

        void* imageHandle = swapchain.GetImageNativeHandle(acquireResult->imageIndex);
        if (imageHandle == nullptr)
        {
            std::fprintf(stderr, "GetImageNativeHandle returned null for a just-acquired image\n");
            device->Shutdown();
            return 1;
        }

        auto commandBufferResult = device->AcquireCommandBuffer();
        if (!commandBufferResult)
        {
            std::fprintf(stderr, "AcquireCommandBuffer failed: %s\n", commandBufferResult.error().detail.c_str());
            device->Shutdown();
            return 1;
        }
        renderer::CommandBuffer commandBuffer = *commandBufferResult;

        if (auto result = commandBuffer.Begin(); !result)
        {
            std::fprintf(stderr, "CommandBuffer::Begin failed: %s\n", result.error().detail.c_str());
            device->Shutdown();
            return 1;
        }

        // CoreVerse's own accent color — an arbitrary but recognizable
        // solid fill, standing in for "the window showed something".
        constexpr renderer::ClearColor color{0.10f, 0.45f, 0.85f, 1.0f};
        if (auto result = commandBuffer.ClearColor(imageHandle, color); !result)
        {
            std::fprintf(stderr, "CommandBuffer::ClearColor failed: %s\n", result.error().detail.c_str());
            device->Shutdown();
            return 1;
        }

        if (auto result = commandBuffer.End(); !result)
        {
            std::fprintf(stderr, "CommandBuffer::End failed: %s\n", result.error().detail.c_str());
            device->Shutdown();
            return 1;
        }

        if (auto result = device->Submit(commandBuffer, nullptr, nullptr, nullptr); !result)
        {
            std::fprintf(stderr, "Submit failed: %s\n", result.error().detail.c_str());
            device->Shutdown();
            return 1;
        }

        // Make sure the clear actually finished before presenting an
        // image nothing has otherwise signaled readiness for.
        device->WaitIdle();

        auto presentResult = swapchain.Present(acquireResult->imageIndex);
        if (!presentResult)
        {
            std::fprintf(stderr, "Present failed: %s\n", presentResult.error().detail.c_str());
            device->Shutdown();
            return 1;
        }

        // Let the presented frame actually finish before tearing down
        // the swapchain/surface below.
        device->WaitIdle();

        std::printf("cleared to (%.2f, %.2f, %.2f) and presented — window closing\n", static_cast<double>(color.r),
                     static_cast<double>(color.g), static_cast<double>(color.b));

        // `swapchain` destructs here (declared after `surface`), then
        // `surface` — matching the ordering rule RenderDevice.hpp
        // documents.
    }

    device->Shutdown();
    return 0;
#endif
}
