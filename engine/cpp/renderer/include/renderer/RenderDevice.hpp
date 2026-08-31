#pragma once

#include <expected>
#include <string_view>

#include "renderer/Buffer.hpp"
#include "renderer/GraphicsAPI.hpp"
#include "renderer/RenderError.hpp"
#include "renderer/Surface.hpp"

namespace renderer
{

/// Abstract, backend-agnostic handle to a graphics device. Concrete
/// backends (VulkanRenderDevice, ...) own every API-specific resource
/// behind this interface; nothing declared here names a Vulkan/D3D/
/// Metal type, and no consumer of this header pulls one in transitively.
///
/// This is deliberately a *thin* common base, not a lowest-common-
/// denominator API: it only covers what every backend can do the same
/// way (report itself, tear itself down, allocate a buffer, create a
/// presentation surface). Anything backend-specific stays out of this
/// interface entirely — callers who need it check GetAPI() and downcast
/// to the concrete backend type (e.g. VulkanRenderDevice::GetVkDevice())
/// rather than this class growing a GetNativeHandle()-style grab bag
/// that would have to be re-interpreted per backend anyway. That
/// downcast is the escape hatch: no backend's advanced functionality is
/// blocked by this abstraction existing.
///
/// Ownership: obtained via CreateRenderDevice() (RenderDeviceFactory.hpp)
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
    /// Every Buffer/Surface this device created must already be
    /// destroyed before this runs.
    virtual void Shutdown() noexcept = 0;

    /// Allocates a GPU buffer. The returned Buffer must be destroyed
    /// before this device's Shutdown() runs — same ordering rule as any
    /// other device-owned resource.
    [[nodiscard]] virtual std::expected<Buffer, RenderError> CreateBuffer(const BufferDesc& desc) noexcept = 0;

    /// Creates a presentation surface bound to a native window. `handle`
    /// is not validated or reinterpreted by this interface — it is
    /// handed straight to the backend, which reads whichever of its
    /// platform's fields are set (see NativeWindowHandle in Surface.hpp).
    /// The returned Surface must be destroyed before this device's
    /// Shutdown() runs — same ordering rule as Buffer.
    [[nodiscard]] virtual std::expected<Surface, RenderError> CreateSurface(const NativeWindowHandle& handle) noexcept = 0;

protected:
    friend class Buffer;
    friend class Surface;

    /// Lets a derived backend construct a Buffer. Buffer's constructor
    /// is private with only RenderDevice as a friend, and friendship
    /// does not extend to derived classes in C++ — this protected
    /// static helper is what a backend's CreateBuffer() override
    /// actually calls to produce the Buffer it returns.
    static Buffer MakeBuffer(RenderDevice* device, void* nativeHandle, std::size_t size) noexcept
    {
        return Buffer(device, nativeHandle, size);
    }

    /// Same purpose as MakeBuffer(), for Surface — see there.
    static Surface MakeSurface(RenderDevice* device, void* nativeHandle) noexcept
    {
        return Surface(device, nativeHandle);
    }

    /// Releases the backend resource behind a Buffer's native handle.
    /// Called only by Buffer's destructor/move-assignment — never call
    /// this directly; release a buffer by letting its Buffer object be
    /// destroyed (or moved-from) instead.
    virtual void ReleaseBuffer(void* nativeHandle) noexcept = 0;

    /// Same contract as ReleaseBuffer(), for Surface — called only by
    /// Surface's destructor/move-assignment.
    virtual void ReleaseSurface(void* nativeHandle) noexcept = 0;
};

} // namespace renderer
