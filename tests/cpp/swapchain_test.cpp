// Faz 5.4 proof: RenderDevice::CreateSwapchain() builds a swapchain
// sized/formatted from the real Surface's capabilities, the image
// count it actually got is logged, and everything tears down cleanly
// — entirely through the RenderDevice/Surface/Swapchain abstraction.
// Also exercises Acquire() once, since it's a self-contained call this
// milestone can already validate; Present() is left for Faz 5.5, once
// there is an actual rendered image to present (presenting an
// untouched image now would just trip validation for no reason).

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
        wc.lpszClassName = L"CoreVerseFaz54DummyWindow";
        RegisterClassW(&wc);

        hwnd = CreateWindowExW(0, wc.lpszClassName, L"CoreVerse Faz 5.4", WS_OVERLAPPEDWINDOW, CW_USEDEFAULT,
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
    std::fprintf(stderr, "swapchain_test: no dummy window implementation for this platform\n");
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

        if (!swapchain.IsValid() || swapchain.GetImageCount() == 0)
        {
            std::fprintf(stderr, "Swapchain round-trip produced an unexpected state\n");
            device->Shutdown();
            return 1;
        }
        std::printf("swapchain image count: %u\n", swapchain.GetImageCount());

        auto acquireResult = swapchain.Acquire();
        if (!acquireResult)
        {
            std::fprintf(stderr, "Acquire failed: %s\n", acquireResult.error().detail.c_str());
            device->Shutdown();
            return 1;
        }
        std::printf("acquired image index: %u\n", acquireResult->imageIndex);

        // `swapchain` destructs here (declared after `surface`, so it
        // releases first), then `surface` — matching the ordering rule
        // RenderDevice.hpp documents.
    }

    device->Shutdown();
    return 0;
#endif
}
