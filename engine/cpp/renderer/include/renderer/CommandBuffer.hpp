#pragma once

#include <expected>

#include "renderer/RenderError.hpp"

namespace renderer
{

class RenderDevice;

/// A color to clear to; components are linear [0,1] regardless of the
/// swapchain's actual pixel format — the backend converts as needed.
struct ClearColor
{
    float r = 0.0f;
    float g = 0.0f;
    float b = 0.0f;
    float a = 1.0f;
};

/// Non-owning handle to a command buffer borrowed from a device-owned
/// pool. Deliberately thin — Begin/End/ClearColor only, matching this
/// milestone's own scope (a single clear-color pass); pipeline state,
/// draw calls, and everything else stay out of this interface rather
/// than being modeled ahead of any actual need for them. Advanced
/// recording goes through the escape hatch — a concrete backend's own
/// accessor reaching the raw command buffer — once a caller needs it;
/// see RenderDevice.hpp's class comment for the same pattern applied
/// everywhere else in this abstraction.
///
/// Unlike Buffer/Surface/Swapchain, this is *not* RAII-owned: it comes
/// from a pool the device manages internally and is implicitly freed
/// when that pool is reset or the device shuts down, not when this
/// wrapper goes out of scope — there is nothing here to release.
/// Copyable on purpose; it's a lightweight reference to pool-owned
/// state, not a resource.
class CommandBuffer
{
public:
    CommandBuffer() noexcept = default;

    [[nodiscard]] bool IsValid() const noexcept
    {
        return m_nativeHandle != nullptr;
    }

    [[nodiscard]] std::expected<void, RenderError> Begin() noexcept;
    [[nodiscard]] std::expected<void, RenderError> End() noexcept;

    /// Clears `imageTarget` — a swapchain image's native handle, from
    /// Swapchain::GetImageNativeHandle() — to a solid color. This
    /// milestone's entire "render" is this one call between Begin() and
    /// End().
    [[nodiscard]] std::expected<void, RenderError> ClearColor(void* imageTarget, const ClearColor& color) noexcept;

    [[nodiscard]] void* GetNativeHandle() const noexcept
    {
        return m_nativeHandle;
    }

private:
    friend class RenderDevice;

    CommandBuffer(RenderDevice* device, void* nativeHandle) noexcept
        : m_device(device)
        , m_nativeHandle(nativeHandle)
    {
    }

    RenderDevice* m_device = nullptr;
    void* m_nativeHandle = nullptr;
};

} // namespace renderer
