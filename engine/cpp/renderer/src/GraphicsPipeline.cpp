#include "renderer/GraphicsPipeline.hpp"

#include "renderer/RenderDevice.hpp"

namespace renderer {

    GraphicsPipeline::~GraphicsPipeline()
    {
        Release();
    }

    GraphicsPipeline::GraphicsPipeline(GraphicsPipeline&& other) noexcept
        : m_device(other.m_device), m_nativeHandle(other.m_nativeHandle), m_pushConstantSize(other.m_pushConstantSize)
    {
        other.m_device = nullptr;
        other.m_nativeHandle = nullptr;
        other.m_pushConstantSize = 0;
    }

    GraphicsPipeline& GraphicsPipeline::operator=(GraphicsPipeline&& other) noexcept
    {
        if (this != &other) {
            Release();
            m_device = other.m_device;
            m_nativeHandle = other.m_nativeHandle;
            m_pushConstantSize = other.m_pushConstantSize;
            other.m_device = nullptr;
            other.m_nativeHandle = nullptr;
            other.m_pushConstantSize = 0;
        }
        return *this;
    }

    void GraphicsPipeline::Release() noexcept
    {
        if (m_nativeHandle != nullptr && m_device != nullptr) {
            m_device->ReleaseGraphicsPipeline(m_nativeHandle);
        }
        m_nativeHandle = nullptr;
        m_device = nullptr;
        m_pushConstantSize = 0;
    }

} // namespace renderer
