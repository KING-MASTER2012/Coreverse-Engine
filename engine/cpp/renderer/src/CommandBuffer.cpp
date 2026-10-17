#include "renderer/CommandBuffer.hpp"

#include "renderer/Buffer.hpp"
#include "renderer/GraphicsPipeline.hpp"
#include "renderer/RenderDevice.hpp"
#include "renderer/Swapchain.hpp"

namespace renderer {

std::expected<void, RenderError> CommandBuffer::Begin() noexcept
{
    return m_device->BeginCommandBuffer(m_nativeHandle);
}

std::expected<void, RenderError> CommandBuffer::End() noexcept
{
    return m_device->EndCommandBuffer(m_nativeHandle);
}

std::expected<void, RenderError>
CommandBuffer::ClearColor(void* imageTarget, const renderer::ClearColor& color) noexcept
{
    return m_device->RecordClearColor(m_nativeHandle, imageTarget, color);
}

std::expected<void, RenderError>
CommandBuffer::BeginRendering(const Swapchain& swapchain, std::uint32_t imageIndex, const RenderingDesc& desc) noexcept
{
    return m_device->RecordBeginRendering(m_nativeHandle, swapchain.GetNativeHandle(), imageIndex, desc);
}

std::expected<void, RenderError>
CommandBuffer::EndRendering(const Swapchain& swapchain, std::uint32_t imageIndex) noexcept
{
    return m_device->RecordEndRendering(m_nativeHandle, swapchain.GetNativeHandle(), imageIndex);
}

std::expected<void, RenderError> CommandBuffer::BindPipeline(const GraphicsPipeline& pipeline) noexcept
{
    return m_device->RecordBindPipeline(m_nativeHandle, pipeline.GetNativeHandle());
}

std::expected<void, RenderError> CommandBuffer::BindVertexBuffer(const Buffer& buffer, std::size_t offset) noexcept
{
    return m_device->RecordBindVertexBuffer(m_nativeHandle, buffer.GetNativeHandle(), offset);
}

std::expected<void, RenderError>
CommandBuffer::BindIndexBuffer(const Buffer& buffer, IndexType type, std::size_t offset) noexcept
{
    return m_device->RecordBindIndexBuffer(m_nativeHandle, buffer.GetNativeHandle(), type, offset);
}

std::expected<void, RenderError>
CommandBuffer::PushConstants(const GraphicsPipeline& pipeline, const void* data, std::uint32_t size) noexcept
{
    if (size != pipeline.GetPushConstantSize() || (size != 0 && data == nullptr)) {
        return std::unexpected(
            RenderError{RenderErrorCode::InitializationFailed, "PushConstants size does not match the pipeline's block"}
        );
    }
    if (size == 0) {
        return {};
    }
    return m_device->RecordPushConstants(m_nativeHandle, pipeline.GetNativeHandle(), data, size);
}

std::expected<void, RenderError> CommandBuffer::Draw(
    std::uint32_t vertexCount, std::uint32_t instanceCount, std::uint32_t firstVertex, std::uint32_t firstInstance
) noexcept
{
    return m_device->RecordDraw(m_nativeHandle, vertexCount, instanceCount, firstVertex, firstInstance);
}

std::expected<void, RenderError> CommandBuffer::DrawIndexed(
    std::uint32_t indexCount,
    std::uint32_t instanceCount,
    std::uint32_t firstIndex,
    std::int32_t vertexOffset,
    std::uint32_t firstInstance
) noexcept
{
    return m_device->RecordDrawIndexed(
        m_nativeHandle, indexCount, instanceCount, firstIndex, vertexOffset, firstInstance
    );
}

} // namespace renderer
