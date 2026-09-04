#include "renderer/CommandBuffer.hpp"

#include "renderer/RenderDevice.hpp"

namespace renderer
{

    std::expected<void, RenderError> CommandBuffer::Begin() noexcept
    {
        return m_device->BeginCommandBuffer(m_nativeHandle);
    }

    std::expected<void, RenderError> CommandBuffer::End() noexcept
    {
        return m_device->EndCommandBuffer(m_nativeHandle);
    }

    std::expected<void, RenderError> CommandBuffer::ClearColor(void* imageTarget, const renderer::ClearColor& color) noexcept
    {
        return m_device->RecordClearColor(m_nativeHandle, imageTarget, color);
    }

} // namespace renderer
