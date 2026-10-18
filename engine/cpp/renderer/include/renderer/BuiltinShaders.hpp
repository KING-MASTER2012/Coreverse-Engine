#pragma once

#include <cstdint>
#include <span>

#include "renderer/GraphicsPipeline.hpp"

namespace renderer {

/// The engine's built-in "colored mesh" shader pair (SPIR-V; GLSL sources and the generator
/// script are in engine/cpp/renderer/shaders/). Vertex layout is ColoredVertex, the push
/// constant block is one 4x4 matrix (the MVP, 64 bytes, column-major like cv_math::Mat4).
[[nodiscard]] std::span<const std::uint32_t> ColoredMeshVertexShader() noexcept;
[[nodiscard]] std::span<const std::uint32_t> ColoredMeshFragmentShader() noexcept;

/// Vertex for the colored mesh shaders: location 0 = position, location 1 = color.
struct ColoredVertex {
    float position[3];
    float color[3];
};
static_assert(sizeof(ColoredVertex) == 24, "ColoredVertex must be tightly packed");

/// The two attributes of ColoredVertex, for GraphicsPipelineDesc::vertexAttributes.
inline constexpr VertexAttribute kColoredVertexAttributes[] = {
    {0, VertexFormat::Float3, 0},
    {1, VertexFormat::Float3, 12},
};

/// Push constant size the colored mesh shaders expect (one mat4).
inline constexpr std::uint32_t kColoredMeshPushConstantSize = 64;

} // namespace renderer
