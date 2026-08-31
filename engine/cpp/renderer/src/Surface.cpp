#include "renderer/Surface.hpp"

#include "renderer/RenderDevice.hpp"

namespace renderer
{

    Surface::~Surface()
    {
        Release();
    }

    Surface::Surface(Surface&& other) noexcept
        : m_device(other.m_device)
        , m_nativeHandle(other.m_nativeHandle)
    {
        other.m_device = nullptr;
        other.m_nativeHandle = nullptr;
    }

    Surface& Surface::operator=(Surface&& other) noexcept
    {
        if (this != &other)
        {
            Release();
            m_device = other.m_device;
            m_nativeHandle = other.m_nativeHandle;
            other.m_device = nullptr;
            other.m_nativeHandle = nullptr;
        }
        return *this;
    }

    void Surface::Release() noexcept
    {
        if (m_nativeHandle != nullptr && m_device != nullptr)
        {
            m_device->ReleaseSurface(m_nativeHandle);
        }
        m_nativeHandle = nullptr;
        m_device = nullptr;
    }

} // namespace renderer
