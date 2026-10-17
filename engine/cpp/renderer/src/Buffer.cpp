#include "renderer/Buffer.hpp"

#include "renderer/RenderDevice.hpp"

namespace renderer {

    Buffer::~Buffer()
    {
        Release();
    }

    Buffer::Buffer(Buffer&& other) noexcept
        : m_device(other.m_device), m_nativeHandle(other.m_nativeHandle), m_size(other.m_size)
    {
        other.m_device = nullptr;
        other.m_nativeHandle = nullptr;
        other.m_size = 0;
    }

    Buffer& Buffer::operator=(Buffer&& other) noexcept
    {
        if (this != &other) {
            Release();
            m_device = other.m_device;
            m_nativeHandle = other.m_nativeHandle;
            m_size = other.m_size;
            other.m_device = nullptr;
            other.m_nativeHandle = nullptr;
            other.m_size = 0;
        }
        return *this;
    }

    std::expected<void, RenderError> Buffer::Write(const void* data, std::size_t size, std::size_t offset) noexcept
    {
        if (!IsValid()) {
            return std::unexpected(
                RenderError{RenderErrorCode::InitializationFailed, "Buffer::Write on an invalid Buffer"}
            );
        }
        if (data == nullptr || offset > m_size || size > m_size - offset) {
            return std::unexpected(
                RenderError{RenderErrorCode::InitializationFailed, "Buffer::Write range is out of bounds"}
            );
        }
        if (size == 0) {
            return {};
        }
        return m_device->WriteBufferData(m_nativeHandle, data, size, offset);
    }

    void Buffer::Release() noexcept
    {
        if (m_nativeHandle != nullptr && m_device != nullptr) {
            m_device->ReleaseBuffer(m_nativeHandle);
        }
        m_nativeHandle = nullptr;
        m_device = nullptr;
        m_size = 0;
    }

} // namespace renderer
