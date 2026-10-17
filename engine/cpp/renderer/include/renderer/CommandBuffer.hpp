#pragma once

#include <cstddef>
#include <cstdint>
#include <expected>

#include "renderer/RenderError.hpp"

namespace renderer {

class Buffer;
class GraphicsPipeline;
class RenderDevice;
class Swapchain;

/// A color to clear to; components are linear [0,1] regardless of the
/// swapchain's actual pixel format — the backend converts as needed.
struct ClearColor {
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
/// What BeginRendering() clears its attachments to.
struct RenderingDesc {
    ClearColor color{};      ///< Clear value of the swapchain image.
    float clearDepth = 1.0f; ///< Clear value of the depth buffer (if the swapchain has one).
};

enum class IndexType {
    U16,
    U32,
};

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

    // ---- Graphics pass (Phase 10) ----
    //
    // The sequence for one frame is Begin(); BeginRendering(); { BindPipeline(); Bind*Buffer();
    // PushConstants(); Draw*()... }; EndRendering(); End(). BeginRendering() transitions the
    // swapchain image (and depth buffer) itself and sets a full-surface viewport and scissor
    // (Y flipped, so clip space is Y-up like OpenGL/the math crate); EndRendering() leaves the
    // image ready for presentation. Use it instead of ClearColor() - the two are alternatives
    // for the same frame, not meant to be mixed.
    //
    // All of these return RenderErrorCode::Unsupported when the device lacks Vulkan 1.3's
    // dynamic rendering.md. `swapchain`, `pipeline` and the buffers must outlive the frame's
    // execution on the GPU (i.e. until FrameSync has waited for it).

    /// Begins rendering.md into swapchain image `imageIndex` (Frame::imageIndex), clearing it.
    [[nodiscard]] std::expected<void, RenderError>
    BeginRendering(const Swapchain& swapchain, std::uint32_t imageIndex, const RenderingDesc& desc) noexcept;

    /// Ends the pass started by BeginRendering() and transitions the image for presentation.
    [[nodiscard]] std::expected<void, RenderError>
    EndRendering(const Swapchain& swapchain, std::uint32_t imageIndex) noexcept;

    [[nodiscard]] std::expected<void, RenderError> BindPipeline(const GraphicsPipeline& pipeline) noexcept;

    /// Binds `buffer` (BufferUsage::VertexBuffer) as vertex binding 0, starting at byte `offset`.
    [[nodiscard]] std::expected<void, RenderError>
    BindVertexBuffer(const Buffer& buffer, std::size_t offset = 0) noexcept;

    /// Binds `buffer` (BufferUsage::IndexBuffer) as the index buffer, starting at byte `offset`.
    [[nodiscard]] std::expected<void, RenderError>
    BindIndexBuffer(const Buffer& buffer, IndexType type, std::size_t offset = 0) noexcept;

    /// Sets the push constants of the pipeline's block; `size` must equal
    /// pipeline.GetPushConstantSize().
    [[nodiscard]] std::expected<void, RenderError>
    PushConstants(const GraphicsPipeline& pipeline, const void* data, std::uint32_t size) noexcept;

    [[nodiscard]] std::expected<void, RenderError> Draw(
        std::uint32_t vertexCount,
        std::uint32_t instanceCount = 1,
        std::uint32_t firstVertex = 0,
        std::uint32_t firstInstance = 0
    ) noexcept;

    [[nodiscard]] std::expected<void, RenderError> DrawIndexed(
        std::uint32_t indexCount,
        std::uint32_t instanceCount = 1,
        std::uint32_t firstIndex = 0,
        std::int32_t vertexOffset = 0,
        std::uint32_t firstInstance = 0
    ) noexcept;

    [[nodiscard]] void* GetNativeHandle() const noexcept
    {
        return m_nativeHandle;
    }

private:
    friend class RenderDevice;

    CommandBuffer(RenderDevice* device, void* nativeHandle) noexcept : m_device(device), m_nativeHandle(nativeHandle) {}

    RenderDevice* m_device = nullptr;
    void* m_nativeHandle = nullptr;
};

} // namespace renderer
