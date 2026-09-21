#pragma once

#include <cstdint>
#include <expected>

#include "renderer/RenderError.hpp"

namespace renderer {

class RenderDevice;
class Surface;

/// "Wait as long as it takes" for Swapchain::Acquire()'s `timeoutNs`.
inline constexpr std::uint64_t kNoTimeout = ~std::uint64_t{0};

/// A size in pixels.
struct Extent2D {
    std::uint32_t width = 0;
    std::uint32_t height = 0;

    friend bool operator==(const Extent2D&, const Extent2D&) = default;
};

/// Outcome of Swapchain::Acquire()/Present() beyond plain success.
/// Deliberately not merged into RenderErrorCode: a swapchain going
/// out-of-date (e.g. after a window resize) is routine, and callers
/// handle it very differently from an initialization failure — they
/// rebuild the swapchain and retry, not propagate/log/abort.
enum class SwapchainStatus {
    Ok,
    Suboptimal, ///< Still presentable, but should be rebuilt soon.
    OutOfDate,  ///< Must be rebuilt before acquiring/presenting again.
    NotReady,   ///< Acquire() only: no image became available within the timeout. Nothing was acquired; try again.
};

struct AcquireResult {
    std::uint32_t imageIndex = 0;
    SwapchainStatus status = SwapchainStatus::Ok;
};

struct SwapchainDesc {
    std::uint32_t preferredImageCount = 2; ///< Hint only; the backend clamps to what the surface actually supports.
    std::uint32_t width = 0;
    std::uint32_t height = 0;
};

/// What a backend reports about a swapchain after (re)building it. Only
/// backends and Swapchain itself use this — see RenderDevice::RebuildSwapchain().
struct SwapchainInfo {
    std::uint32_t imageCount = 0;
    Extent2D extent{};
};

/// Move-only RAII handle to a presentable swapchain. Construction
/// happens through RenderDevice::CreateSwapchain(); the destructor
/// releases the backend resource (the swapchain and its image views)
/// automatically.
///
/// A Swapchain must be destroyed before the Surface it was created
/// from, and before the RenderDevice's Shutdown() — same ordering rule
/// as Buffer/Surface (RenderDevice.hpp).
class Swapchain
{
public:
    Swapchain() noexcept = default;
    ~Swapchain();

    Swapchain(const Swapchain&) = delete;
    Swapchain& operator=(const Swapchain&) = delete;

    Swapchain(Swapchain&& other) noexcept;
    Swapchain& operator=(Swapchain&& other) noexcept;

    [[nodiscard]] bool IsValid() const noexcept
    {
        return m_nativeHandle != nullptr;
    }

    [[nodiscard]] std::uint32_t GetImageCount() const noexcept
    {
        return m_imageCount;
    }

    /// Size, in pixels, of the swapchain's images — what the surface
    /// actually gave, which is not necessarily the SwapchainDesc that
    /// was asked for (see SwapchainDesc). Zero-sized after a failed
    /// Recreate() (see there).
    [[nodiscard]] Extent2D GetExtent() const noexcept
    {
        return m_extent;
    }

    /// Rebuilds this swapchain in place on `surface` (the Surface it was
    /// created from) — the answer to SwapchainStatus::OutOfDate /
    /// Suboptimal and to a window resize. `desc` is treated exactly as in
    /// RenderDevice::CreateSwapchain(): hints only, the surface's own
    /// extent wins whenever it dictates one.
    ///
    /// The same Swapchain object keeps working afterwards
    /// (GetNativeHandle() is unchanged), but everything derived from the
    /// old swapchain is invalid: handles from GetImageNativeHandle(),
    /// acquired image indices, and anything built on them (framebuffers,
    /// ...). GetImageCount()/GetExtent() reflect the new state.
    ///
    /// Waits for the device to go idle first, so this is a resize-time
    /// operation, not something to call every frame. A FrameSync
    /// (FrameSync.hpp) needs no notification afterwards: it notices the
    /// rebuilt swapchain on its next BeginFrame().
    ///
    /// Failure semantics:
    ///  * Anything detected before the old swapchain is touched — e.g.
    ///    RenderErrorCode::ZeroExtent for a minimized window, or a
    ///    different Surface than the one this swapchain was created
    ///    from: nothing changed, the swapchain stays as it was. Try again
    ///    once the surface has a size.
    ///  * The replacement itself could not be created: the old swapchain
    ///    was retired by the driver and is dropped. The object stays
    ///    valid but empty (GetImageCount() == 0, Acquire()/Present()
    ///    report OutOfDate) until a later Recreate() succeeds.
    ///    Destroying it is always fine.
    [[nodiscard]] std::expected<void, RenderError> Recreate(const Surface& surface, const SwapchainDesc& desc) noexcept;

    /// Acquires the next presentable image; blocks until one is ready or
    /// `timeoutNs` nanoseconds have passed (kNoTimeout, the default, never
    /// gives up). A timeout is reported as SwapchainStatus::NotReady, not
    /// as an error: nothing was acquired and the call can simply be
    /// repeated — which is what a GUI thread wants for a window that is
    /// hidden or occluded, where images may not be handed back for a while.
    /// `signalSemaphore` is a backend-native semaphore handle (e.g. a
    /// VkSemaphore) the GPU signals once the image is actually
    /// available — pass one obtained through the backend's own escape
    /// hatch so a subsequent Submit() can wait on it before rendering.
    /// nullptr (the default) makes this call synchronous instead: it
    /// blocks the CPU until the image is ready using an internal fence,
    /// which is enough for a single-shot render. A real render loop
    /// should use FrameSync (FrameSync.hpp), which owns all of this.
    /// Suboptimal/OutOfDate are not errors — they mean the swapchain
    /// should be rebuilt (typically after a resize). The caller decides
    /// when; a Suboptimal result still hands back a usable image index.
    [[nodiscard]] std::expected<AcquireResult, RenderError>
    Acquire(void* signalSemaphore = nullptr, std::uint64_t timeoutNs = kNoTimeout) noexcept;

    /// Presents a previously acquired image. `waitSemaphore` is a
    /// backend-native semaphore handle (e.g. a VkSemaphore signaled by
    /// the work that rendered into this image); nullptr presents
    /// without waiting on anything, which is only correct for a
    /// single-shot render that already waited for the GPU itself (see
    /// render_loop_test). A real render loop uses FrameSync
    /// (FrameSync.hpp), which passes the right semaphore.
    [[nodiscard]] std::expected<SwapchainStatus, RenderError>
    Present(std::uint32_t imageIndex, void* waitSemaphore = nullptr) noexcept;

    /// Backend-owned opaque resource pointer — same escape-hatch
    /// contract as Buffer::GetNativeHandle() (see Buffer.hpp).
    [[nodiscard]] void* GetNativeHandle() const noexcept
    {
        return m_nativeHandle;
    }

    /// Backend-owned opaque handle for the image at `index` (0 ..
    /// GetImageCount()-1) — e.g. a VkImage for the Vulkan backend. Used
    /// to target a specific swapchain image with a command, such as
    /// CommandBuffer::ClearColor(). Same escape-hatch contract as
    /// GetNativeHandle(): what this actually points to is backend-
    /// defined, meaningful only to that backend's own recording calls.
    [[nodiscard]] void* GetImageNativeHandle(std::uint32_t index) const noexcept;

private:
    friend class RenderDevice;

    Swapchain(RenderDevice* device, void* nativeHandle, std::uint32_t imageCount, Extent2D extent) noexcept
        : m_device(device), m_nativeHandle(nativeHandle), m_imageCount(imageCount), m_extent(extent)
    {}

    void Release() noexcept;

    RenderDevice* m_device = nullptr;
    void* m_nativeHandle = nullptr;
    std::uint32_t m_imageCount = 0;
    Extent2D m_extent{};
};

} // namespace renderer
