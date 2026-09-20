#pragma once

#include <cstdint>
#include <expected>

#include "renderer/RenderError.hpp"

namespace renderer {

class RenderDevice;
class Surface;

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
    /// Waits for the device to go idle first — there is no per-frame
    /// synchronization to wait on yet (Phase 6.4) — so this is a
    /// resize-time operation, not something to call every frame.
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

    /// Acquires the next presentable image; blocks until one is ready.
    /// `signalSemaphore` is a backend-native semaphore handle (e.g. a
    /// VkSemaphore) the GPU signals once the image is actually
    /// available — pass one obtained through the backend's own escape
    /// hatch so a subsequent Submit() can wait on it before rendering.
    /// nullptr (the default) makes this call synchronous instead: it
    /// blocks the CPU until the image is ready using an internal fence,
    /// which is enough before any real submission work exists.
    /// Suboptimal/OutOfDate are not errors — they mean the swapchain
    /// should be rebuilt (typically after a resize). The caller decides
    /// when; a Suboptimal result still hands back a usable image index.
    [[nodiscard]] std::expected<AcquireResult, RenderError> Acquire(void* signalSemaphore = nullptr) noexcept;

    /// Presents a previously acquired image. `waitSemaphore` is a
    /// backend-native semaphore handle (e.g. a VkSemaphore signaled by
    /// the work that rendered into this image); nullptr presents
    /// without waiting on anything, which is only meaningful before any
    /// rendering has been wired up (Phase 5.4) — Phase 5.5's render loop is
    /// expected to pass a real one obtained through the backend's own
    /// escape hatch (see RenderDevice.hpp's class comment).
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
