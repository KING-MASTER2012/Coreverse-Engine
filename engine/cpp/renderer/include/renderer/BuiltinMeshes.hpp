#pragma once

#include <cstdint>

#include "renderer/BuiltinShaders.hpp"

namespace renderer {

/// A cube spanning [-1, 1] on every axis, one color per corner, for the colored mesh shaders
/// (ColoredVertex). Triangles are counter-clockwise seen from outside in a right-handed, Y-up
/// space - what CullMode::Back expects. Used by the editor viewport and the graphics pass test.
inline constexpr ColoredVertex kCubeVertices[] = {
    {{-1.0F, -1.0F, -1.0F}, {0.1F, 0.1F, 0.1F}},
    {{1.0F, -1.0F, -1.0F}, {1.0F, 0.2F, 0.2F}},
    {{1.0F, 1.0F, -1.0F}, {1.0F, 1.0F, 0.2F}},
    {{-1.0F, 1.0F, -1.0F}, {0.2F, 1.0F, 0.2F}},
    {{-1.0F, -1.0F, 1.0F}, {0.2F, 0.2F, 1.0F}},
    {{1.0F, -1.0F, 1.0F}, {1.0F, 0.2F, 1.0F}},
    {{1.0F, 1.0F, 1.0F}, {1.0F, 1.0F, 1.0F}},
    {{-1.0F, 1.0F, 1.0F}, {0.2F, 1.0F, 1.0F}},
};

/// 12 triangles, 16-bit indices (IndexType::U16).
inline constexpr std::uint16_t kCubeIndices[] = {
    4, 5, 6, 4, 6, 7, // +Z
    1, 0, 3, 1, 3, 2, // -Z
    1, 2, 6, 1, 6, 5, // +X
    0, 4, 7, 0, 7, 3, // -X
    7, 6, 2, 7, 2, 3, // +Y
    0, 1, 5, 0, 5, 4, // -Y
};

} // namespace renderer
