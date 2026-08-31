#pragma once

#include <cstddef>
#include <cstdint>

namespace renderer
{

class RenderDevice;

/// Bitmask describing what a buffer will be used for. Covers only the
/// handful of uses every backend supports the same way — anything more
/// exotic goes through the escape hatch on the concrete RenderDevice,
/// not through this enum (see RenderDevice.hpp's class comment).
enum class BufferUsage : std::uint32_t
{
    None          = 0,
    VertexBuffer  = 1u << 0,
    IndexBuffer   = 1u << 1,
    UniformBuffer = 1u << 2,
    TransferSrc   = 1u << 3,
    TransferDst   = 1u << 4,
};

constexpr BufferUsage operator|(BufferUsage lhs, BufferUsage rhs) noexcept
{
    return static_cast<BufferUsage>(static_cast<std::uint32_t>(lhs) | static_cast<std::uint32_t>(rhs));
}

constexpr bool HasFlag(BufferUsage value, BufferUsage flag) noexcept
{
    return (static_cast<std::uint32_t>(value) & static_cast<std::uint32_t>(flag)) != 0;
}

/// Where a buffer's memory should live: left device-local for the GPU
/// to read fastest (CPU writes would go through a staging buffer, not
/// modeled yet), or made host-visible for direct CPU access in one
/// direction or the other.
enum class BufferMemoryUsage
{
    GpuOnly,
    CpuToGpu, ///< Host-visible, optimized for sequential CPU writes (uploads).
    GpuToCpu, ///< Host-visible, optimized for CPU reads (readback).
};

struct BufferDesc
{
    std::size_t size = 0;
    BufferUsage usage = BufferUsage::None;
    BufferMemoryUsage memoryUsage = BufferMemoryUsage::GpuOnly;
};

/// Move-only RAII handle to a GPU buffer. Construction happens through
/// RenderDevice::CreateBuffer(); the destructor releases the backend
/// resource automatically, so there is no explicit Destroy() to forget
/// or call twice.
///
/// A Buffer must be destroyed before the RenderDevice that created it
/// calls Shutdown() — same ordering rule as any other device-owned
/// resource; the device does not track buffers it handed out, so
/// nothing else enforces this.
class Buffer
{
public:
    Buffer() noexcept = default;
    ~Buffer();

    Buffer(const Buffer&) = delete;
    Buffer& operator=(const Buffer&) = delete;

    Buffer(Buffer&& other) noexcept;
    Buffer& operator=(Buffer&& other) noexcept;

    [[nodiscard]] std::size_t GetSize() const noexcept
    {
        return m_size;
    }

    [[nodiscard]] bool IsValid() const noexcept
    {
        return m_nativeHandle != nullptr;
    }

    /// Backend-owned opaque resource pointer. Not for general use — it
    /// exists so a concrete backend's own APIs (e.g. a Vulkan command
    /// recording call needing the underlying VkBuffer) can retrieve it
    /// through their own accessor after checking RenderDevice::GetAPI().
    [[nodiscard]] void* GetNativeHandle() const noexcept
    {
        return m_nativeHandle;
    }

private:
    friend class RenderDevice;

    Buffer(RenderDevice* device, void* nativeHandle, std::size_t size) noexcept
        : m_device(device)
        , m_nativeHandle(nativeHandle)
        , m_size(size)
    {
    }

    void Release() noexcept;

    RenderDevice* m_device = nullptr;
    void* m_nativeHandle = nullptr;
    std::size_t m_size = 0;
};

} // namespace renderer
