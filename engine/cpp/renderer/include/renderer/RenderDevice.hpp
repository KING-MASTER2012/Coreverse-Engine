#pragma once

#include <cstdint>
#include <expected>
#include <string_view>

#include "renderer/Buffer.hpp"
#include "renderer/CommandBuffer.hpp"
#include "renderer/GraphicsAPI.hpp"
#include "renderer/RenderError.hpp"
#include "renderer/Surface.hpp"
#include "renderer/Swapchain.hpp"

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
/// presentation surface, build a swapchain, record and submit a minimal
/// command buffer). Anything backend-specific stays out of this
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

    /// Blocks until this device has finished all outstanding GPU work.
    /// Shutdown() calls this itself before releasing anything, but a
    /// caller that created its own backend-native sync objects (via the
    /// escape hatch — e.g. Faz 5.5's render loop creating VkSemaphores
    /// directly) must call this before destroying those objects itself;
    /// this device has no way to track resources it didn't hand out.
    virtual void WaitIdle() noexcept = 0;

    /// Releases every backend resource this device owns. Safe to call
    /// more than once; the destructor calls it too, so an explicit call
    /// is only needed when teardown order must be controlled (e.g.
    /// before destroying a window this device's surface depends on).
    /// Every Buffer/Surface/Swapchain this device created must already
    /// be destroyed before this runs.
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

    /// Builds a swapchain on a previously created Surface, sized and
    /// formatted according to what that surface's capabilities actually
    /// allow (desc is a set of hints, not a guarantee — see
    /// SwapchainDesc in Swapchain.hpp). The returned Swapchain must be
    /// destroyed before its Surface, and before this device's
    /// Shutdown() — same ordering rule as Buffer/Surface.
    [[nodiscard]] virtual std::expected<Swapchain, RenderError> CreateSwapchain(const Surface& surface,
                                                                                 const SwapchainDesc& desc) noexcept = 0;

    /// Borrows a command buffer from a pool this device owns. Unlike
    /// Create*() above, the returned CommandBuffer is not RAII-owned —
    /// see CommandBuffer.hpp's class comment — so there is no matching
    /// Destroy call for it.
    [[nodiscard]] virtual std::expected<CommandBuffer, RenderError> AcquireCommandBuffer() noexcept = 0;

    /// Submits a recorded (Begin()/.../End()'d) command buffer to this
    /// device's graphics queue. `waitSemaphore`/`signalSemaphore`/`fence`
    /// are backend-native sync object handles (nullptr/VK_NULL_HANDLE-
    /// equivalent to skip any of them) — obtained through the backend's
    /// own escape hatch, since owning and pacing sync objects is
    /// render-loop-specific, not something this abstraction should
    /// impose a shape on this early.
    [[nodiscard]] virtual std::expected<void, RenderError>
    Submit(const CommandBuffer& commandBuffer, void* waitSemaphore, void* signalSemaphore, void* fence) noexcept = 0;

protected:
    friend class Buffer;
    friend class Surface;
    friend class Swapchain;
    friend class CommandBuffer;

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

    /// Same purpose as MakeBuffer(), for Swapchain — see there.
    static Swapchain MakeSwapchain(RenderDevice* device, void* nativeHandle, std::uint32_t imageCount) noexcept
    {
        return Swapchain(device, nativeHandle, imageCount);
    }

    /// Same purpose as MakeBuffer(), for CommandBuffer — see there.
    static CommandBuffer MakeCommandBuffer(RenderDevice* device, void* nativeHandle) noexcept
    {
        return CommandBuffer(device, nativeHandle);
    }

    /// Releases the backend resource behind a Buffer's native handle.
    /// Called only by Buffer's destructor/move-assignment — never call
    /// this directly; release a buffer by letting its Buffer object be
    /// destroyed (or moved-from) instead.
    virtual void ReleaseBuffer(void* nativeHandle) noexcept = 0;

    /// Same contract as ReleaseBuffer(), for Surface — called only by
    /// Surface's destructor/move-assignment.
    virtual void ReleaseSurface(void* nativeHandle) noexcept = 0;

    /// Same contract as ReleaseBuffer(), for Swapchain — called only by
    /// Swapchain's destructor/move-assignment.
    virtual void ReleaseSwapchain(void* nativeHandle) noexcept = 0;

    /// Backs Swapchain::Acquire() — called only by Swapchain, never
    /// directly. `signalSemaphore` is nullable; see Swapchain::Acquire()
    /// for what nullptr means.
    virtual std::expected<AcquireResult, RenderError> AcquireSwapchainImage(void* nativeHandle,
                                                                             void* signalSemaphore) noexcept = 0;

    /// Backs Swapchain::Present() — called only by Swapchain, never
    /// directly.
    virtual std::expected<SwapchainStatus, RenderError>
    PresentSwapchainImage(void* nativeHandle, std::uint32_t imageIndex, void* waitSemaphore) noexcept = 0;

    /// Backs Swapchain::GetImageNativeHandle() — called only by
    /// Swapchain, never directly.
    virtual void* GetSwapchainImageHandle(void* swapchainNativeHandle, std::uint32_t index) noexcept = 0;

    /// Backs CommandBuffer::Begin()/End()/ClearColor() — called only by
    /// CommandBuffer, never directly.
    virtual std::expected<void, RenderError> BeginCommandBuffer(void* commandBufferHandle) noexcept = 0;
    virtual std::expected<void, RenderError> EndCommandBuffer(void* commandBufferHandle) noexcept = 0;
    virtual std::expected<void, RenderError>
    RecordClearColor(void* commandBufferHandle, void* imageHandle, const ClearColor& color) noexcept = 0;
};

} // namespace renderer
