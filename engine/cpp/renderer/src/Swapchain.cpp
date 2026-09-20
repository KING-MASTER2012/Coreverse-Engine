#include "renderer/Swapchain.hpp"

#include "renderer/RenderDevice.hpp"

namespace renderer {

Swapchain::~Swapchain()
{
    Release();
}

Swapchain::Swapchain(Swapchain&& other) noexcept
    : m_device(other.m_device), m_nativeHandle(other.m_nativeHandle), m_imageCount(other.m_imageCount),
      m_extent(other.m_extent)
{
    other.m_device = nullptr;
    other.m_nativeHandle = nullptr;
    other.m_imageCount = 0;
    other.m_extent = {};
}

Swapchain& Swapchain::operator=(Swapchain&& other) noexcept
{
    if (this != &other) {
        Release();
        m_device = other.m_device;
        m_nativeHandle = other.m_nativeHandle;
        m_imageCount = other.m_imageCount;
        m_extent = other.m_extent;
        other.m_device = nullptr;
        other.m_nativeHandle = nullptr;
        other.m_imageCount = 0;
        other.m_extent = {};
    }
    return *this;
}

void Swapchain::Release() noexcept
{
    if (m_nativeHandle != nullptr && m_device != nullptr) {
        m_device->ReleaseSwapchain(m_nativeHandle);
    }
    m_nativeHandle = nullptr;
    m_device = nullptr;
    m_imageCount = 0;
    m_extent = {};
}

std::expected<void, RenderError> Swapchain::Recreate(const Surface& surface, const SwapchainDesc& desc) noexcept
{
    if (m_device == nullptr || m_nativeHandle == nullptr) {
        return std::unexpected(
            RenderError{RenderErrorCode::InitializationFailed, "Recreate() called on an invalid Swapchain"}
        );
    }

    // The backend updates `info` on every path that changes what the
    // swapchain holds — including a failure that had to drop the old
    // resources — so the accessors below never go stale.
    SwapchainInfo info{m_imageCount, m_extent};
    auto result = m_device->RebuildSwapchain(m_nativeHandle, surface, desc, info);
    m_imageCount = info.imageCount;
    m_extent = info.extent;
    return result;
}

std::expected<AcquireResult, RenderError> Swapchain::Acquire(void* signalSemaphore) noexcept
{
    return m_device->AcquireSwapchainImage(m_nativeHandle, signalSemaphore);
}

std::expected<SwapchainStatus, RenderError> Swapchain::Present(std::uint32_t imageIndex, void* waitSemaphore) noexcept
{
    return m_device->PresentSwapchainImage(m_nativeHandle, imageIndex, waitSemaphore);
}

void* Swapchain::GetImageNativeHandle(std::uint32_t index) const noexcept
{
    return m_device->GetSwapchainImageHandle(m_nativeHandle, index);
}

} // namespace renderer
