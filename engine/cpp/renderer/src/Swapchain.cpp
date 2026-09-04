#include "renderer/Swapchain.hpp"

#include "renderer/RenderDevice.hpp"

namespace renderer
{

    Swapchain::~Swapchain()
    {
        Release();
    }

    Swapchain::Swapchain(Swapchain&& other) noexcept
        : m_device(other.m_device)
        , m_nativeHandle(other.m_nativeHandle)
        , m_imageCount(other.m_imageCount)
    {
        other.m_device = nullptr;
        other.m_nativeHandle = nullptr;
        other.m_imageCount = 0;
    }

    Swapchain& Swapchain::operator=(Swapchain&& other) noexcept
    {
        if (this != &other)
        {
            Release();
            m_device = other.m_device;
            m_nativeHandle = other.m_nativeHandle;
            m_imageCount = other.m_imageCount;
            other.m_device = nullptr;
            other.m_nativeHandle = nullptr;
            other.m_imageCount = 0;
        }
        return *this;
    }

    void Swapchain::Release() noexcept
    {
        if (m_nativeHandle != nullptr && m_device != nullptr)
        {
            m_device->ReleaseSwapchain(m_nativeHandle);
        }
        m_nativeHandle = nullptr;
        m_device = nullptr;
        m_imageCount = 0;
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
