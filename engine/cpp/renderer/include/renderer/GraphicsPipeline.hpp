#pragma once

#include <cstdint>
#include <span>

namespace renderer {

class RenderDevice;

/// Vertex attribute formats the renderer needs so far (32-bit float vectors).
enum class VertexFormat {
    Float2,
    Float3,
    Float4,
};

/// One vertex attribute read from the (single) vertex buffer binding.
struct VertexAttribute {
    std::uint32_t location = 0; ///< `layout(location = N)` in the vertex shader.
    VertexFormat format = VertexFormat::Float3;
    std::uint32_t offset = 0; ///< Byte offset inside one vertex.
};

enum class CullMode {
    Off,  ///< No culling. (Not named `None`: X11 defines that as a macro.)
    Back, ///< Discard back faces. Front faces are counter-clockwise (as seen in a Y-up, GL-style clip space).
    Front,
};

/// Everything needed to build a graphics pipeline for the swapchain it is created against.
/// The shaders are SPIR-V (GLSL compiled with glslangValidator/glslc; see
/// BuiltinShaders.hpp for the engine's own). Entry point is `main` for both.
struct GraphicsPipelineDesc {
    std::span<const std::uint32_t> vertexShader;   ///< SPIR-V words.
    std::span<const std::uint32_t> fragmentShader; ///< SPIR-V words.

    /// One vertex buffer binding (binding 0), per-vertex rate; `vertexStride` bytes per vertex.
    std::uint32_t vertexStride = 0;
    std::span<const VertexAttribute> vertexAttributes;

    /// Size in bytes of the push constant block shared by both stages (0 = none; max 128, the
    /// minimum every Vulkan device supports; must be a multiple of 4). Push constants are how
    /// per-draw data such as an MVP matrix reaches the shaders; descriptor-set based uniform
    /// buffers are not part of the renderer yet.
    std::uint32_t pushConstantSize = 0;

    CullMode cullMode = CullMode::Back;
    bool depthTest = false;  ///< Needs a swapchain created with SwapchainDesc::depthBuffer.
    bool depthWrite = false; ///< Only meaningful together with depthTest.
};

/// Move-only RAII handle to a compiled graphics pipeline - same ownership rules as Buffer:
/// created by RenderDevice::CreateGraphicsPipeline(), released by the destructor, must be
/// destroyed before the device's Shutdown(). It is built for the swapchain's color/depth
/// formats; if a swapchain is recreated with different formats the pipeline must be rebuilt
/// (a plain resize keeps the formats).
class GraphicsPipeline
{
public:
    GraphicsPipeline() noexcept = default;
    ~GraphicsPipeline();

    GraphicsPipeline(const GraphicsPipeline&) = delete;
    GraphicsPipeline& operator=(const GraphicsPipeline&) = delete;

    GraphicsPipeline(GraphicsPipeline&& other) noexcept;
    GraphicsPipeline& operator=(GraphicsPipeline&& other) noexcept;

    [[nodiscard]] bool IsValid() const noexcept
    {
        return m_nativeHandle != nullptr;
    }

    [[nodiscard]] std::uint32_t GetPushConstantSize() const noexcept
    {
        return m_pushConstantSize;
    }

    /// Backend-specific handle (a heap-allocated struct in the Vulkan backend); same contract
    /// as Buffer::GetNativeHandle().
    [[nodiscard]] void* GetNativeHandle() const noexcept
    {
        return m_nativeHandle;
    }

private:
    friend class RenderDevice;

    GraphicsPipeline(RenderDevice* device, void* nativeHandle, std::uint32_t pushConstantSize) noexcept
        : m_device(device), m_nativeHandle(nativeHandle), m_pushConstantSize(pushConstantSize)
    {}

    void Release() noexcept;

    RenderDevice* m_device = nullptr;
    void* m_nativeHandle = nullptr;
    std::uint32_t m_pushConstantSize = 0;
};

} // namespace renderer
