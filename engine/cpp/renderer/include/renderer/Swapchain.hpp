#pragma once

#include <cstdint>
#include <expected>

#include "renderer/RenderError.hpp"

namespace renderer
{

class RenderDevice;

/// Outcome of Swapchain::Acquire()/Present() beyond plain success.
/// Deliberately not merged into RenderErrorCode: a swapchain going
/// out-of-date (e.g. after a window resize) is routine, and callers
/// handle it very differently from an initialization failure — they
/// rebuild the swapchain and retry, not propagate/log/abort.
enum class SwapchainStatus
{
    Ok,
    Suboptimal, ///< Still presentable, but should be rebuilt soon.
    OutOfDate,  ///< Must be rebuilt before acquiring/presenting again.
};

struct AcquireResult
{
    std::uint32_t imageIndex = 0;
    SwapchainStatus status = SwapchainStatus::Ok;
};

struct SwapchainDesc
{
    std::uint32_t preferredImageCount = 2; ///< Hint only; the backend clamps to what the surface actually supports.
    std::uint32_t width = 0;
    std::uint32_t height = 0;
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

    /// Acquires the next presentable image; blocks until one is ready.
    /// Suboptimal/OutOfDate are not errors — they mean the swapchain
    /// should be rebuilt (typically after a resize). The caller decides
    /// when; a Suboptimal result still hands back a usable image index.
    [[nodiscard]] std::expected<AcquireResult, RenderError> Acquire() noexcept;

    /// Presents a previously acquired image. `waitSemaphore` is a
    /// backend-native semaphore handle (e.g. a VkSemaphore signaled by
    /// the work that rendered into this image); nullptr presents
    /// without waiting on anything, which is only meaningful before any
    /// rendering has been wired up (Faz 5.4) — Faz 5.5's render loop is
    /// expected to pass a real one obtained through the backend's own
    /// escape hatch (see RenderDevice.hpp's class comment).
    [[nodiscard]] std::expected<SwapchainStatus, RenderError> Present(std::uint32_t imageIndex,
                                                                       void* waitSemaphore = nullptr) noexcept;

    /// Backend-owned opaque resource pointer — same escape-hatch
    /// contract as Buffer::GetNativeHandle() (see Buffer.hpp).
    [[nodiscard]] void* GetNativeHandle() const noexcept
    {
        return m_nativeHandle;
    }

private:
    friend class RenderDevice;

    Swapchain(RenderDevice* device, void* nativeHandle, std::uint32_t imageCount) noexcept
        : m_device(device)
        , m_nativeHandle(nativeHandle)
        , m_imageCount(imageCount)
    {
    }

    void Release() noexcept;

    RenderDevice* m_device = nullptr;
    void* m_nativeHandle = nullptr;
    std::uint32_t m_imageCount = 0;
};

} // namespace renderer
