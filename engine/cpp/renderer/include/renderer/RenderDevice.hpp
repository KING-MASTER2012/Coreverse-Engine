#pragma once

#include <string_view>

#include "renderer/GraphicsAPI.hpp"

namespace renderer
{

/// Abstract, backend-agnostic handle to a graphics device. Concrete
/// backends (VulkanRenderDevice, ...) own every API-specific resource
/// behind this interface; nothing declared here names a Vulkan/D3D/
/// Metal type, and no consumer of this header pulls one in transitively.
///
/// This is deliberately a *thin* common base, not a lowest-common-
/// denominator API: it only covers what every backend can do the same
/// way (report itself, tear itself down). Anything backend-specific
/// stays out of this interface entirely — callers who need it check
/// GetAPI() and downcast to the concrete backend type (e.g.
/// VulkanRenderDevice::GetVkDevice()) rather than this class growing a
/// GetNativeHandle()-style grab bag that would have to be re-interpreted
/// per backend anyway. That downcast is the escape hatch: no backend's
/// advanced functionality is blocked by this abstraction existing.
///
/// Ownership: obtained via CreateRenderDevice() (RenderDeviceFactory.h)
/// as a std::unique_ptr<RenderDevice>. Non-copyable and non-movable —
/// it's used polymorphically through the base pointer, so neither
/// operation should be possible.
class RenderDevice
{
public:
    RenderDevice() = default;
    virtual ~RenderDevice() = default;

    RenderDevice(const RenderDevice&) = delete;
    RenderDevice& operator=(const RenderDevice&) = delete;
    RenderDevice(RenderDevice&&) = delete;
    RenderDevice& operator=(RenderDevice&&) = delete;

    /// Which backend this instance implements. Check this before
    /// downcasting to a concrete backend type for backend-specific
    /// functionality.
    [[nodiscard]] virtual GraphicsAPI GetAPI() const noexcept = 0;

    /// Human-readable name of the selected physical device (e.g. a GPU
    /// model string), for logging/diagnostics.
    [[nodiscard]] virtual std::string_view GetDeviceName() const noexcept = 0;

    /// Releases every backend resource this device owns. Safe to call
    /// more than once; the destructor calls it too, so an explicit call
    /// is only needed when teardown order must be controlled (e.g.
    /// before destroying a window this device's surface depends on).
    virtual void Shutdown() noexcept = 0;
};

} // namespace renderer
