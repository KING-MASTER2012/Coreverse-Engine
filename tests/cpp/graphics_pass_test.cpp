// Graphics pass proof (Phase 10): depth-buffered swapchain -> graphics pipeline built from
// the built-in colored-mesh SPIR-V -> vertex/index buffers filled with Buffer::Write() ->
// per-frame MVP from cv-math pushed as push constants -> BeginRendering()/DrawIndexed()/
// EndRendering() through FrameSync, including a mid-loop resize (depth buffer rebuilt with the
// swapchain), and device-loss recovery: after SimulateDeviceLossForTesting() frame-level calls
// fail with RenderErrorCode::DeviceLost, everything is destroyed, a new device is created and
// the same scene renders again.
//
// The validation layer turns synchronization/layout/format mistakes (wrong barriers, depth
// format mismatches between pipeline and attachment, ...) into a failing test; set
// CV_REQUIRE_VALIDATION=1 (CI does) to also fail when the layer is not running. A device
// without Vulkan 1.3 dynamic rendering reports Unsupported and the test skips itself.

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <iterator>
#include <memory>
#include <string>
#include <utility>

#include "cv-math/Math.hpp"
#include "renderer/BuiltinMeshes.hpp"
#include "renderer/BuiltinShaders.hpp"
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
        wc.lpszClassName = L"CoreverseGraphicsPassTestWindow";
        RegisterClassW(&wc);

        hwnd = CreateWindowExW(
            0,
            wc.lpszClassName,
            L"Coreverse graphics pass test",
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
constexpr int kSkipExitCode = 0;

/// True when the environment variable is set to something other than "" or "0".
bool EnvFlagSet(const char* name)
{
    #if defined(_WIN32)
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

using renderer::kCubeIndices;
using renderer::kCubeVertices;

enum class LoopResult {
    Ok,
    DeviceLost,
    Failed,
};

/// Everything the editor will own for a viewport, in one place so the test can tear it down and
/// build it again after a (simulated) device loss.
struct Scene {
    std::unique_ptr<renderer::RenderDevice> device; // declared first: destroyed last
    renderer::Surface surface;
    renderer::Swapchain swapchain;
    renderer::FrameSync sync;
    renderer::Buffer vertexBuffer;
    renderer::Buffer indexBuffer;
    renderer::GraphicsPipeline pipeline;
    unsigned frameCounter = 0;
    unsigned rebuilds = 0;

    /// Returns false on failure; `unsupported` is set when the device simply cannot run the
    /// graphics pass (no dynamic rendering).
    bool Build(const DummyWindow& window, bool& unsupported)
    {
        auto deviceResult = renderer::CreateRenderDevice(renderer::GraphicsAPI::Vulkan);
        if (!deviceResult) {
            return Fail("CreateRenderDevice failed", deviceResult.error());
        }
        device = std::move(*deviceResult);
        if (EnvFlagSet("CV_REQUIRE_VALIDATION") && !device->IsValidationEnabled()) {
            return Fail("CV_REQUIRE_VALIDATION is set but the validation layer is not running");
        }

        auto surfaceResult = device->CreateSurface(window.ToNativeHandle());
        if (!surfaceResult) {
            return Fail("CreateSurface failed", surfaceResult.error());
        }
        surface = std::move(*surfaceResult);

        auto swapchainResult = device->CreateSwapchain(surface, SwapchainDescFor(kInitialWidth, kInitialHeight));
        if (!swapchainResult) {
            return Fail("CreateSwapchain (with depth buffer) failed", swapchainResult.error());
        }
        swapchain = std::move(*swapchainResult);

        auto syncResult = device->CreateFrameSync(swapchain, renderer::FrameSyncDesc{2});
        if (!syncResult) {
            return Fail("CreateFrameSync failed", syncResult.error());
        }
        sync = std::move(*syncResult);

        renderer::BufferDesc vertexDesc{};
        vertexDesc.size = sizeof(kCubeVertices);
        vertexDesc.usage = renderer::BufferUsage::VertexBuffer;
        vertexDesc.memoryUsage = renderer::BufferMemoryUsage::CpuToGpu;
        auto vertexResult = device->CreateBuffer(vertexDesc);
        if (!vertexResult) {
            return Fail("CreateBuffer (vertex) failed", vertexResult.error());
        }
        vertexBuffer = std::move(*vertexResult);
        if (auto written = vertexBuffer.Write(kCubeVertices, sizeof(kCubeVertices)); !written) {
            return Fail("Buffer::Write (vertex) failed", written.error());
        }

        renderer::BufferDesc indexDesc{};
        indexDesc.size = sizeof(kCubeIndices);
        indexDesc.usage = renderer::BufferUsage::IndexBuffer;
        indexDesc.memoryUsage = renderer::BufferMemoryUsage::CpuToGpu;
        auto indexResult = device->CreateBuffer(indexDesc);
        if (!indexResult) {
            return Fail("CreateBuffer (index) failed", indexResult.error());
        }
        indexBuffer = std::move(*indexResult);
        if (auto written = indexBuffer.Write(kCubeIndices, sizeof(kCubeIndices)); !written) {
            return Fail("Buffer::Write (index) failed", written.error());
        }

        renderer::GraphicsPipelineDesc pipelineDesc{};
        pipelineDesc.vertexShader = renderer::ColoredMeshVertexShader();
        pipelineDesc.fragmentShader = renderer::ColoredMeshFragmentShader();
        pipelineDesc.vertexStride = sizeof(renderer::ColoredVertex);
        pipelineDesc.vertexAttributes = renderer::kColoredVertexAttributes;
        pipelineDesc.pushConstantSize = renderer::kColoredMeshPushConstantSize;
        pipelineDesc.cullMode = renderer::CullMode::Back;
        pipelineDesc.depthTest = true;
        pipelineDesc.depthWrite = true;
        auto pipelineResult = device->CreateGraphicsPipeline(swapchain, pipelineDesc);
        if (!pipelineResult) {
            if (pipelineResult.error().code == renderer::RenderErrorCode::Unsupported) {
                unsupported = true;
            }
            return Fail("CreateGraphicsPipeline failed", pipelineResult.error());
        }
        pipeline = std::move(*pipelineResult);
        return true;
    }

    static renderer::SwapchainDesc SwapchainDescFor(std::uint32_t width, std::uint32_t height)
    {
        renderer::SwapchainDesc desc{};
        desc.preferredImageCount = 2;
        desc.width = width;
        desc.height = height;
        desc.depthBuffer = true;
        return desc;
    }

    /// Destroys every resource in dependency order, then the device.
    void Teardown()
    {
        // Frames may still be executing and reference the pipeline and buffers: resources must
        // not be destroyed before the GPU is done with them.
        if (device) {
            device->WaitIdle();
        }
        pipeline = renderer::GraphicsPipeline{};
        indexBuffer = renderer::Buffer{};
        vertexBuffer = renderer::Buffer{};
        sync = renderer::FrameSync{};
        swapchain = renderer::Swapchain{};
        surface = renderer::Surface{};
        if (device) {
            device->Shutdown();
            device.reset();
        }
    }

    bool Rebuild(std::uint32_t width, std::uint32_t height)
    {
        auto result = swapchain.Recreate(surface, SwapchainDescFor(width, height));
        if (!result) {
            return Fail("Swapchain::Recreate failed", result.error());
        }
        ++rebuilds;
        return true;
    }

    bool RecordFrame(renderer::Frame& frame)
    {
        namespace m = cv_math;
        const float time = static_cast<float>(frameCounter) / 60.0F;
        const renderer::Extent2D extent = swapchain.GetExtent();
        const float aspect = static_cast<float>(extent.width) / static_cast<float>(extent.height);
        const m::Mat4 model = m::Mat4::fromRotationY(time * 0.9F) * m::Mat4::fromRotationX(time * 0.6F);
        const m::Mat4 view = m::Mat4::lookAtRh({0.0F, 0.0F, 5.0F}, {0.0F, 0.0F, 0.0F}, {0.0F, 1.0F, 0.0F});
        const m::Mat4 proj = m::Mat4::perspectiveRh(1.0471976F, aspect, 0.1F, 100.0F);
        const m::Mat4 mvp = proj * view * model;

        renderer::CommandBuffer& cb = frame.commandBuffer;
        if (auto r = cb.Begin(); !r) {
            return Fail("CommandBuffer::Begin failed", r.error());
        }
        renderer::RenderingDesc rendering{};
        rendering.color = {0.05F, 0.05F, 0.08F, 1.0F};
        rendering.clearDepth = 1.0F;
        if (auto r = cb.BeginRendering(swapchain, frame.imageIndex, rendering); !r) {
            return Fail("BeginRendering failed", r.error());
        }
        if (auto r = cb.BindPipeline(pipeline); !r) {
            return Fail("BindPipeline failed", r.error());
        }
        if (auto r = cb.BindVertexBuffer(vertexBuffer); !r) {
            return Fail("BindVertexBuffer failed", r.error());
        }
        if (auto r = cb.BindIndexBuffer(indexBuffer, renderer::IndexType::U16); !r) {
            return Fail("BindIndexBuffer failed", r.error());
        }
        if (auto r = cb.PushConstants(pipeline, mvp.data(), renderer::kColoredMeshPushConstantSize); !r) {
            return Fail("PushConstants failed", r.error());
        }
        if (auto r = cb.DrawIndexed(static_cast<std::uint32_t>(std::size(kCubeIndices))); !r) {
            return Fail("DrawIndexed failed", r.error());
        }
        if (auto r = cb.EndRendering(swapchain, frame.imageIndex); !r) {
            return Fail("EndRendering failed", r.error());
        }
        if (auto r = cb.End(); !r) {
            return Fail("CommandBuffer::End failed", r.error());
        }
        return true;
    }

    /// Renders `count` frames (optionally resizing the window once, halfway), handling
    /// OutOfDate/Suboptimal like the editor does. A DeviceLost from FrameSync ends the loop with
    /// LoopResult::DeviceLost instead of failing - the caller decides whether that is expected.
    LoopResult RunFrames(const DummyWindow& window, unsigned count, bool resizeHalfway)
    {
        unsigned done = 0;
        unsigned consecutiveNotReady = 0;
        bool resized = false;
        while (done < count) {
            window.Pump();
            if (resizeHalfway && !resized && done >= count / 2) {
                window.Resize(kInitialWidth * 3 / 2, kInitialHeight * 3 / 2);
                resized = true;
            }

            auto begin = sync.BeginFrame(swapchain);
            if (!begin) {
                if (begin.error().code == renderer::RenderErrorCode::DeviceLost) {
                    return LoopResult::DeviceLost;
                }
                Fail("BeginFrame failed", begin.error());
                return LoopResult::Failed;
            }
            if (begin->status == renderer::FrameStatus::OutOfDate) {
                const renderer::Extent2D extent = swapchain.GetExtent();
                if (!Rebuild(extent.width, extent.height)) {
                    return LoopResult::Failed;
                }
                continue;
            }
            if (begin->status == renderer::FrameStatus::NotReady) {
                if (++consecutiveNotReady > 20) {
                    Fail("BeginFrame reported NotReady for 20 attempts in a row");
                    return LoopResult::Failed;
                }
                continue;
            }
            consecutiveNotReady = 0;

            renderer::Frame& frame = begin->frame;
            if (!RecordFrame(frame)) {
                return LoopResult::Failed;
            }
            auto end = sync.EndFrame(swapchain, frame);
            if (!end) {
                if (end.error().code == renderer::RenderErrorCode::DeviceLost) {
                    return LoopResult::DeviceLost;
                }
                Fail("EndFrame failed", end.error());
                return LoopResult::Failed;
            }
            ++done;
            ++frameCounter;
            if (*end == renderer::SwapchainStatus::Suboptimal || *end == renderer::SwapchainStatus::OutOfDate) {
                const renderer::Extent2D extent = swapchain.GetExtent();
                if (!Rebuild(extent.width, extent.height)) {
                    return LoopResult::Failed;
                }
            }
        }
        return LoopResult::Ok;
    }
};

/// Buffer::Write and pipeline creation misuse must be reported, not crash.
bool CheckMisuse(Scene& scene)
{
    renderer::BufferDesc gpuOnly{};
    gpuOnly.size = 64;
    gpuOnly.usage = renderer::BufferUsage::VertexBuffer;
    gpuOnly.memoryUsage = renderer::BufferMemoryUsage::GpuOnly;
    auto gpuBuffer = scene.device->CreateBuffer(gpuOnly);
    if (!gpuBuffer) {
        return Fail("CreateBuffer (GpuOnly) failed", gpuBuffer.error());
    }
    const float zeros[16] = {};
    if (auto r = gpuBuffer->Write(zeros, sizeof(zeros));
        r || r.error().code != renderer::RenderErrorCode::Unsupported) {
        return Fail("Buffer::Write on a GpuOnly buffer must fail with Unsupported");
    }
    if (scene.vertexBuffer.Write(zeros, sizeof(zeros), sizeof(kCubeVertices))) {
        return Fail("Buffer::Write past the end of the buffer unexpectedly succeeded");
    }
    if (scene.vertexBuffer.Write(nullptr, 4)) {
        return Fail("Buffer::Write(nullptr) unexpectedly succeeded");
    }
    if (renderer::Buffer{}.Write(zeros, 4)) {
        return Fail("Buffer::Write on an invalid Buffer unexpectedly succeeded");
    }

    renderer::GraphicsPipelineDesc empty{};
    if (scene.device->CreateGraphicsPipeline(scene.swapchain, empty)) {
        return Fail("CreateGraphicsPipeline without shaders unexpectedly succeeded");
    }
    renderer::GraphicsPipelineDesc badPush{};
    badPush.vertexShader = renderer::ColoredMeshVertexShader();
    badPush.fragmentShader = renderer::ColoredMeshFragmentShader();
    badPush.pushConstantSize = 130;
    if (scene.device->CreateGraphicsPipeline(scene.swapchain, badPush)) {
        return Fail("CreateGraphicsPipeline with an oversized push constant block unexpectedly succeeded");
    }
    return true;
}

#endif

} // namespace

int main()
{
#if !defined(_WIN32) && !defined(__linux__)
    std::fprintf(stderr, "graphics_pass_test: no dummy window implementation for this platform\n");
    return 1;
#else
    DummyWindow window;
    if (!window.IsValid()) {
        std::fprintf(stderr, "failed to create dummy test window\n");
        return 1;
    }

    Scene scene;
    bool unsupported = false;
    if (!scene.Build(window, unsupported)) {
        scene.Teardown();
        if (unsupported) {
            std::printf("graphics_pass_test: SKIPPED - the device has no Vulkan 1.3 dynamic rendering\n");
            return kSkipExitCode;
        }
        return 1;
    }
    std::printf(
        "device: %s, validation %s\n",
        scene.device->GetDeviceName().data(),
        scene.device->IsValidationEnabled() ? "ON" : "off"
    );

    if (!CheckMisuse(scene)) {
        scene.Teardown();
        return 1;
    }

    // 1) A normal run, with a resize in the middle (depth buffer rebuilt with the swapchain).
    if (scene.RunFrames(window, 120, true) != LoopResult::Ok) {
        scene.Teardown();
        return 1;
    }
    std::printf("rendered %u frames, %u swapchain rebuild(s)\n", scene.frameCounter, scene.rebuilds);
    if (scene.rebuilds == 0) {
        std::fprintf(stderr, "expected the resize to rebuild the swapchain at least once\n");
        scene.Teardown();
        return 1;
    }

    // 2) Device loss: frame-level calls start failing with DeviceLost ...
    scene.device->SimulateDeviceLossForTesting();
    if (scene.RunFrames(window, 1, false) != LoopResult::DeviceLost) {
        std::fprintf(stderr, "a lost device must be reported as RenderErrorCode::DeviceLost\n");
        scene.Teardown();
        return 1;
    }

    // ... and the recovery is: destroy everything, create a new device, rebuild, carry on.
    scene.Teardown();
    scene.frameCounter = 0;
    scene.rebuilds = 0;
    if (!scene.Build(window, unsupported)) {
        std::fprintf(stderr, "rebuilding after the device loss failed\n");
        scene.Teardown();
        return 1;
    }
    if (scene.RunFrames(window, 60, false) != LoopResult::Ok) {
        scene.Teardown();
        return 1;
    }
    std::printf("recovered from device loss, rendered %u more frames\n", scene.frameCounter);

    scene.Teardown();

    if (renderer::ValidationErrorCount() != 0) {
        std::fprintf(stderr, "the validation layer reported errors (see the [vulkan] lines above)\n");
        return 1;
    }
    std::printf("graphics pass OK\n");
    return 0;
#endif
}
