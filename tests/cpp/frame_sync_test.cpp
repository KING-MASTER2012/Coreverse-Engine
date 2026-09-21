// Phase 6.4 proof: FrameSync keeps several frames in flight over a real
// swapchain — many acquire/record/submit/present rounds through the public
// RenderDevice/Swapchain/FrameSync abstraction only, including a window
// resize in the middle, with 1, 2 and more frames in flight than the
// swapchain has images.
//
// The interesting failures here are synchronization mistakes (a reused
// semaphore, a fence waited on twice, an image rendered to while still being
// presented): they do not crash, they just corrupt. The validation layer is
// what reports them, and renderer::ValidationErrorCount() is what turns that
// report into a failing test. Set CV_REQUIRE_VALIDATION=1 (CI does) to also
// fail when the layer is not actually running — otherwise "0 errors" would
// prove nothing on a machine without it.

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <memory>
#include <string>
#include <utility>

#include "renderer/Diagnostics.hpp"
#include "renderer/RenderDeviceFactory.hpp"

#if defined(_WIN32)
    #include <windows.h>
#elif defined(__linux__)
    #include <X11/Xlib.h>
#endif

namespace {

#if defined(_WIN32)

struct DummyWindow {
    HWND hwnd = nullptr;

    DummyWindow()
    {
        WNDCLASSW wc{};
        wc.lpfnWndProc = DefWindowProcW;
        wc.hInstance = GetModuleHandleW(nullptr);
        wc.lpszClassName = L"CoreverseFrameSyncTestWindow";
        RegisterClassW(&wc);

        hwnd = CreateWindowExW(
            0,
            wc.lpszClassName,
            L"Coreverse Phase 6.4",
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            320,
            240,
            nullptr,
            nullptr,
            wc.hInstance,
            nullptr
        );
        if (hwnd != nullptr) {
            // A window that was never shown may never get its images back
            // from the presentation engine on some drivers.
            ShowWindow(hwnd, SW_SHOWNA);
        }
    }

    ~DummyWindow()
    {
        if (hwnd != nullptr) {
            DestroyWindow(hwnd);
        }
    }

    [[nodiscard]] bool IsValid() const
    {
        return hwnd != nullptr;
    }

    /// Resizes the window so its CLIENT area is exactly newWidth x
    /// newHeight (which is what the surface's extent reports).
    void Resize(std::uint32_t newWidth, std::uint32_t newHeight) const
    {
        RECT rect{0, 0, static_cast<LONG>(newWidth), static_cast<LONG>(newHeight)};
        AdjustWindowRect(&rect, WS_OVERLAPPEDWINDOW, FALSE);
        SetWindowPos(
            hwnd,
            nullptr,
            0,
            0,
            rect.right - rect.left,
            rect.bottom - rect.top,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE
        );
    }

    /// Keeps the window responsive while the loop runs.
    void Pump() const
    {
        MSG msg;
        while (PeekMessageW(&msg, nullptr, 0, 0, PM_REMOVE)) {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
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

struct DummyWindow {
    Display* display = nullptr;
    Window window = 0;

    DummyWindow()
    {
        display = XOpenDisplay(nullptr);
        if (display == nullptr) {
            return;
        }
        const int screen = DefaultScreen(display);
        window = XCreateSimpleWindow(
            display,
            RootWindow(display, screen),
            0,
            0,
            320,
            240,
            0,
            BlackPixel(display, screen),
            WhitePixel(display, screen)
        );
        XMapWindow(display, window);
        XSync(display, False);
    }

    ~DummyWindow()
    {
        if (display != nullptr) {
            if (window != 0) {
                XDestroyWindow(display, window);
            }
            XCloseDisplay(display);
        }
    }

    [[nodiscard]] bool IsValid() const
    {
        return display != nullptr && window != 0;
    }

    void Resize(std::uint32_t newWidth, std::uint32_t newHeight) const
    {
        XResizeWindow(display, window, newWidth, newHeight);
        XSync(display, False); // the surface's extent is read back from the server
    }

    void Pump() const {}

    [[nodiscard]] renderer::NativeWindowHandle ToNativeHandle() const
    {
        renderer::NativeWindowHandle handle{};
        handle.xlibDisplay = display;
        handle.xlibWindow = window;
        return handle;
    }
};

#endif

#if defined(_WIN32) || defined(__linux__)

constexpr std::uint32_t kInitialWidth = 320;
constexpr std::uint32_t kInitialHeight = 240;

/// True when the environment variable is set to something other than "" or "0".
bool EnvFlagSet(const char* name)
{
    #if defined(_WIN32)
    // std::getenv is flagged as unsafe by MSVC (C4996).
    char* value = nullptr;
    std::size_t length = 0;
    if (_dupenv_s(&value, &length, name) != 0 || value == nullptr) {
        return false;
    }
    const bool set = value[0] != '\0' && value[0] != '0';
    std::free(value);
    return set;
    #else
    const char* value = std::getenv(name);
    return value != nullptr && value[0] != '\0' && value[0] != '0';
    #endif
}

bool Fail(const char* what, const renderer::RenderError& error)
{
    std::fprintf(stderr, "%s: %s\n", what, error.detail.c_str());
    return false;
}

bool Fail(const char* what)
{
    std::fprintf(stderr, "%s\n", what);
    return false;
}

struct LoopStats {
    unsigned ready = 0;
    unsigned notReady = 0;
    unsigned outOfDate = 0;
    unsigned suboptimal = 0;
    unsigned rebuilds = 0;
};

/// Records the frame's clear and ends the command buffer.
bool RecordClear(renderer::Frame& frame, unsigned frameNumber)
{
    if (auto result = frame.commandBuffer.Begin(); !result) {
        return Fail("CommandBuffer::Begin failed", result.error());
    }

    const float phase = static_cast<float>(frameNumber % 60U) / 60.0F;
    const renderer::ClearColor color{phase, 0.45F, 1.0F - phase, 1.0F};
    if (auto result = frame.commandBuffer.ClearColor(frame.imageHandle, color); !result) {
        return Fail("CommandBuffer::ClearColor failed", result.error());
    }

    if (auto result = frame.commandBuffer.End(); !result) {
        return Fail("CommandBuffer::End failed", result.error());
    }
    return true;
}

/// Runs until `readyFramesWanted` frames were rendered and presented,
/// handling OutOfDate/Suboptimal the way the editor does (rebuild between
/// frames), and checking what FrameSync promises about every frame.
bool RunLoop(
    const DummyWindow& window,
    renderer::Surface& surface,
    renderer::Swapchain& swapchain,
    renderer::FrameSync& sync,
    unsigned readyFramesWanted,
    std::uint32_t width,
    std::uint32_t height,
    LoopStats& stats
)
{
    const auto rebuild = [&]() {
        renderer::SwapchainDesc desc{};
        desc.preferredImageCount = 2;
        desc.width = width;
        desc.height = height;
        auto result = swapchain.Recreate(surface, desc);
        if (!result) {
            if (result.error().code == renderer::RenderErrorCode::ZeroExtent) {
                return true; // nothing to rebuild yet
            }
            return Fail("Swapchain::Recreate failed", result.error());
        }
        ++stats.rebuilds;
        return true;
    };

    unsigned done = 0;
    unsigned consecutiveNotReady = 0;
    while (done < readyFramesWanted) {
        window.Pump();

        auto begin = sync.BeginFrame(swapchain);
        if (!begin) {
            return Fail("BeginFrame failed", begin.error());
        }

        if (begin->status == renderer::FrameStatus::OutOfDate) {
            ++stats.outOfDate;
            if (!rebuild()) {
                return false;
            }
            continue;
        }
        if (begin->status == renderer::FrameStatus::NotReady) {
            ++stats.notReady;
            if (++consecutiveNotReady > 20) {
                return Fail("BeginFrame reported NotReady for 20 attempts in a row: no image ever became available");
            }
            continue;
        }
        consecutiveNotReady = 0;

        renderer::Frame& frame = begin->frame;

        // What FrameSync promises about a Ready frame.
        if (frame.frameIndex != stats.ready % sync.GetFramesInFlight()) {
            std::fprintf(
                stderr,
                "frame %u used slot %u, expected %u\n",
                stats.ready,
                frame.frameIndex,
                stats.ready % sync.GetFramesInFlight()
            );
            return false;
        }
        if (frame.imageIndex >= swapchain.GetImageCount()) {
            return Fail("BeginFrame handed out an image index beyond the swapchain's image count");
        }
        if (frame.imageHandle == nullptr || frame.imageHandle != swapchain.GetImageNativeHandle(frame.imageIndex)) {
            return Fail("Frame::imageHandle does not match Swapchain::GetImageNativeHandle()");
        }
        if (!(frame.extent == swapchain.GetExtent())) {
            return Fail("Frame::extent does not match Swapchain::GetExtent()");
        }
        if (!frame.commandBuffer.IsValid()) {
            return Fail("BeginFrame handed out an invalid command buffer");
        }

        if (!RecordClear(frame, stats.ready)) {
            return false;
        }

        auto end = sync.EndFrame(swapchain, frame);
        if (!end) {
            return Fail("EndFrame failed", end.error());
        }
        ++stats.ready;
        ++done;

        if (*end == renderer::SwapchainStatus::Suboptimal || *end == renderer::SwapchainStatus::OutOfDate) {
            if (*end == renderer::SwapchainStatus::Suboptimal) {
                ++stats.suboptimal;
            } else {
                ++stats.outOfDate;
            }
            if (!rebuild()) {
                return false;
            }
        }
    }
    return true;
}

/// Acquire() with a timeout: taking images without ever presenting them
/// must end in NotReady — not in a hang — once the swapchain has none left.
bool TestAcquireTimeout(renderer::RenderDevice& device, const renderer::Surface& surface)
{
    renderer::SwapchainDesc desc{};
    desc.preferredImageCount = 2;
    desc.width = kInitialWidth;
    desc.height = kInitialHeight;
    auto swapchainResult = device.CreateSwapchain(surface, desc);
    if (!swapchainResult) {
        return Fail("CreateSwapchain failed", swapchainResult.error());
    }
    renderer::Swapchain swapchain = std::move(*swapchainResult);

    constexpr std::uint64_t timeoutNs = 50'000'000; // 50 ms
    for (std::uint32_t attempt = 0; attempt <= swapchain.GetImageCount(); ++attempt) {
        auto acquire = swapchain.Acquire(nullptr, timeoutNs);
        if (!acquire) {
            return Fail("Acquire failed", acquire.error());
        }
        if (acquire->status == renderer::SwapchainStatus::NotReady) {
            std::printf("Acquire timeout: NotReady after %u image(s)\n", attempt);
            return true;
        }
    }
    return Fail("Acquire never reported NotReady although no image was ever presented");
}

bool RunTest(const DummyWindow& window, renderer::RenderDevice& device)
{
    auto surfaceResult = device.CreateSurface(window.ToNativeHandle());
    if (!surfaceResult) {
        return Fail("CreateSurface failed", surfaceResult.error());
    }
    renderer::Surface surface = std::move(*surfaceResult);

    if (!TestAcquireTimeout(device, surface)) {
        return false;
    }

    renderer::SwapchainDesc swapchainDesc{};
    swapchainDesc.preferredImageCount = 2;
    swapchainDesc.width = kInitialWidth;
    swapchainDesc.height = kInitialHeight;
    auto swapchainResult = device.CreateSwapchain(surface, swapchainDesc);
    if (!swapchainResult) {
        return Fail("CreateSwapchain failed", swapchainResult.error());
    }
    renderer::Swapchain swapchain = std::move(*swapchainResult);
    std::printf(
        "swapchain: %u images, %ux%u\n",
        swapchain.GetImageCount(),
        swapchain.GetExtent().width,
        swapchain.GetExtent().height
    );

    // ---- creation edge cases ------------------------------------------------
    {
        renderer::FrameSync empty;
        if (empty.IsValid()) {
            return Fail("a default-constructed FrameSync claims to be valid");
        }
        renderer::Swapchain invalidSwapchain;
        if (device.CreateFrameSync(invalidSwapchain, renderer::FrameSyncDesc{})) {
            return Fail("CreateFrameSync on an invalid Swapchain unexpectedly succeeded");
        }
        if (empty.BeginFrame(swapchain)) {
            return Fail("BeginFrame on an invalid FrameSync unexpectedly succeeded");
        }

        auto zero = device.CreateFrameSync(swapchain, renderer::FrameSyncDesc{0});
        if (!zero) {
            return Fail("CreateFrameSync(framesInFlight = 0) failed", zero.error());
        }
        if (zero->GetFramesInFlight() != 1) {
            return Fail("framesInFlight = 0 was not clamped to 1");
        }
        auto huge = device.CreateFrameSync(swapchain, renderer::FrameSyncDesc{1000});
        if (!huge) {
            return Fail("CreateFrameSync(framesInFlight = 1000) failed", huge.error());
        }
        if (huge->GetFramesInFlight() != renderer::kMaxFramesInFlight) {
            return Fail("a huge framesInFlight was not clamped to kMaxFramesInFlight");
        }
    }

    // ---- the loop, for several frames-in-flight settings ---------------------
    struct Phase {
        std::uint32_t framesInFlight;
        std::uint32_t resizeWidth;
        std::uint32_t resizeHeight;
    };
    // 6 frames in flight is more than the swapchain has images on typical
    // drivers, which exercises the "image still in use by another slot" path.
    constexpr Phase phases[] = {{1, 400, 300}, {2, 320, 240}, {6, 480, 360}};

    std::uint32_t currentWidth = kInitialWidth;
    std::uint32_t currentHeight = kInitialHeight;

    for (const Phase& phase : phases) {
        auto syncResult = device.CreateFrameSync(swapchain, renderer::FrameSyncDesc{phase.framesInFlight});
        if (!syncResult) {
            return Fail("CreateFrameSync failed", syncResult.error());
        }
        renderer::FrameSync sync = std::move(*syncResult);
        if (sync.GetFramesInFlight() != phase.framesInFlight) {
            return Fail("FrameSync reports a different framesInFlight than requested");
        }

        LoopStats stats;
        if (!RunLoop(window, surface, swapchain, sync, 60, currentWidth, currentHeight, stats)) {
            return false;
        }

        // Misuse must be reported, not deadlock or corrupt state: a second
        // BeginFrame() while a frame is open, and an EndFrame() with no
        // frame open.
        {
            auto first = sync.BeginFrame(swapchain);
            if (!first) {
                return Fail("BeginFrame failed", first.error());
            }
            if (first->status != renderer::FrameStatus::Ready) {
                return Fail("expected a Ready frame for the misuse check");
            }
            if (sync.BeginFrame(swapchain)) {
                return Fail("a second BeginFrame() before EndFrame() unexpectedly succeeded");
            }
            if (!RecordClear(first->frame, stats.ready)) {
                return false;
            }
            auto ended = sync.EndFrame(swapchain, first->frame);
            if (!ended) {
                return Fail("EndFrame failed after a rejected second BeginFrame", ended.error());
            }
            ++stats.ready;
            if (sync.EndFrame(swapchain, first->frame)) {
                return Fail("EndFrame() without an open frame unexpectedly succeeded");
            }
            // Both rejections left the FrameSync usable — but the slot
            // counter moved on by one frame, which RunLoop's slot check
            // (based on stats.ready) must follow.
        }

        // Resize in the middle of the loop: whatever the driver reports
        // (Suboptimal, OutOfDate, or nothing at all), the loop has to carry on.
        window.Resize(phase.resizeWidth, phase.resizeHeight);
        currentWidth = phase.resizeWidth;
        currentHeight = phase.resizeHeight;

        if (!RunLoop(window, surface, swapchain, sync, 20, currentWidth, currentHeight, stats)) {
            return false;
        }

        // Force a rebuild even if the driver never asked for one, so the
        // "FrameSync notices a rebuilt swapchain" path always runs.
        renderer::SwapchainDesc desc{};
        desc.preferredImageCount = 2;
        desc.width = currentWidth;
        desc.height = currentHeight;
        device.WaitIdle();
        if (auto result = swapchain.Recreate(surface, desc); !result) {
            return Fail("Swapchain::Recreate failed", result.error());
        }
        ++stats.rebuilds;
        if (swapchain.GetExtent().width != currentWidth || swapchain.GetExtent().height != currentHeight) {
            std::fprintf(
                stderr,
                "swapchain is %ux%u after resizing the window to %ux%u\n",
                swapchain.GetExtent().width,
                swapchain.GetExtent().height,
                currentWidth,
                currentHeight
            );
            return false;
        }

        if (!RunLoop(window, surface, swapchain, sync, 60, currentWidth, currentHeight, stats)) {
            return false;
        }

        std::printf(
            "framesInFlight=%u: %u frames, %u rebuilds (%u suboptimal, %u out-of-date, %u not-ready), swapchain %u "
            "images %ux%u\n",
            phase.framesInFlight,
            stats.ready,
            stats.rebuilds,
            stats.suboptimal,
            stats.outOfDate,
            stats.notReady,
            swapchain.GetImageCount(),
            swapchain.GetExtent().width,
            swapchain.GetExtent().height
        );

        if (renderer::ValidationErrorCount() != 0) {
            return Fail("the validation layer reported errors (see the [vulkan] lines above)");
        }

        // Move semantics: the moved-from FrameSync is empty, the new one works.
        renderer::FrameSync moved = std::move(sync);
        if (sync.IsValid() ||
            !moved.IsValid()) { // NOLINT(bugprone-use-after-move): checking the moved-from state is the point
            return Fail("FrameSync move left the wrong object valid");
        }
        if (!RunLoop(window, surface, swapchain, moved, 3, currentWidth, currentHeight, stats)) {
            return false; // same `stats`: the slot sequence carries on across the move
        }
        // `moved` (and with it all per-frame state) is destroyed here, while
        // the swapchain lives on for the next phase.
    }

    // `swapchain` destructs here (declared after `surface`), then `surface`.
    return true;
}

#endif

} // namespace

int main()
{
#if !defined(_WIN32) && !defined(__linux__)
    std::fprintf(stderr, "frame_sync_test: no dummy window implementation for this platform\n");
    return 1;
#else
    DummyWindow window;
    if (!window.IsValid()) {
        std::fprintf(stderr, "failed to create dummy test window\n");
        return 1;
    }

    auto deviceResult = renderer::CreateRenderDevice(renderer::GraphicsAPI::Vulkan);
    if (!deviceResult) {
        std::fprintf(stderr, "CreateRenderDevice failed: %s\n", deviceResult.error().detail.c_str());
        return 1;
    }
    std::unique_ptr<renderer::RenderDevice> device = std::move(*deviceResult);

    std::printf(
        "device: %s, validation %s\n",
        std::string(device->GetDeviceName()).c_str(),
        device->IsValidationEnabled() ? "ON" : "off"
    );

    if (EnvFlagSet("CV_REQUIRE_VALIDATION") && !device->IsValidationEnabled()) {
        std::fprintf(stderr, "CV_REQUIRE_VALIDATION is set but the validation layer is not running\n");
        device->Shutdown();
        return 1;
    }

    const bool ok = RunTest(window, *device);
    device->Shutdown();

    if (!ok) {
        return 1;
    }
    if (renderer::ValidationErrorCount() != 0) {
        std::fprintf(stderr, "the validation layer reported errors during teardown (see the [vulkan] lines above)\n");
        return 1;
    }

    std::printf("frame sync loop OK\n");
    return 0;
#endif
}
