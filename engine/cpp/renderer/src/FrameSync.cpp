#include "renderer/FrameSync.hpp"

#include "renderer/RenderDevice.hpp"

namespace renderer {

FrameSync::~FrameSync()
{
    Release();
}

FrameSync::FrameSync(FrameSync&& other) noexcept
    : m_device(other.m_device), m_nativeHandle(other.m_nativeHandle), m_framesInFlight(other.m_framesInFlight)
{
    other.m_device = nullptr;
    other.m_nativeHandle = nullptr;
    other.m_framesInFlight = 0;
}

FrameSync& FrameSync::operator=(FrameSync&& other) noexcept
{
    if (this != &other) {
        Release();
        m_device = other.m_device;
        m_nativeHandle = other.m_nativeHandle;
        m_framesInFlight = other.m_framesInFlight;
        other.m_device = nullptr;
        other.m_nativeHandle = nullptr;
        other.m_framesInFlight = 0;
    }
    return *this;
}

void FrameSync::Release() noexcept
{
    if (m_nativeHandle != nullptr && m_device != nullptr) {
        m_device->ReleaseFrameSync(m_nativeHandle);
    }
    m_nativeHandle = nullptr;
    m_device = nullptr;
    m_framesInFlight = 0;
}

std::expected<BeginFrameResult, RenderError> FrameSync::BeginFrame(Swapchain& swapchain) noexcept
{
    if (m_device == nullptr || m_nativeHandle == nullptr) {
        return std::unexpected(
            RenderError{RenderErrorCode::InitializationFailed, "BeginFrame() called on an invalid FrameSync"}
        );
    }
    if (!swapchain.IsValid()) {
        return std::unexpected(
            RenderError{RenderErrorCode::InitializationFailed, "BeginFrame() called with an invalid Swapchain"}
        );
    }
    return m_device->BeginFrameSync(m_nativeHandle, swapchain);
}

std::expected<SwapchainStatus, RenderError> FrameSync::EndFrame(Swapchain& swapchain, const Frame& frame) noexcept
{
    if (m_device == nullptr || m_nativeHandle == nullptr) {
        return std::unexpected(
            RenderError{RenderErrorCode::InitializationFailed, "EndFrame() called on an invalid FrameSync"}
        );
    }
    if (!swapchain.IsValid()) {
        return std::unexpected(
            RenderError{RenderErrorCode::InitializationFailed, "EndFrame() called with an invalid Swapchain"}
        );
    }
    return m_device->EndFrameSync(m_nativeHandle, swapchain, frame);
}

} // namespace renderer
