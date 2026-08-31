// Faz 5.3 proof: RenderDevice::CreateSurface() from a real native
// window handle. Uses a dummy window created directly by this test
// (not Faz 6's Qt window, per the plan) via the platform's own raw
// windowing API — no windowing abstraction is introduced just for this
// test, matching Surface.hpp's decision not to wrap the native handle.

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
        wc.lpszClassName = L"CoreVerseFaz53DummyWindow";
        RegisterClassW(&wc);

        hwnd = CreateWindowExW(0, wc.lpszClassName, L"CoreVerse Faz 5.3", WS_OVERLAPPEDWINDOW, CW_USEDEFAULT,
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
};

#endif

} // namespace

int main()
{
#if !defined(_WIN32) && !defined(__linux__)
    std::fprintf(stderr, "surface_test: no dummy window implementation for this platform\n");
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
        if (!surface.IsValid())
        {
            std::fprintf(stderr, "Surface round-trip produced an unexpected state\n");
            device->Shutdown();
            return 1;
        }
        // `surface` releases the VkSurfaceKHR here, at end of scope —
        // before device->Shutdown() below, per the ordering rule
        // RenderDevice.hpp documents.
    }

    std::printf("surface create/destroy ok\n");
    device->Shutdown();
    return 0;
#endif
}
