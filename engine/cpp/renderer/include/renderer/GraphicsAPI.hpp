#pragma once

namespace renderer
{

    /// Identifies which concrete graphics backend a RenderDevice implements.
    /// Only Vulkan is implemented as of Faz 5 — the others are declared now
    /// so RenderDeviceFactory's switch and any GraphicsAPI-keyed logic don't
    /// need to change shape when a backend is added, only gain a case.
    enum class GraphicsAPI
    {
        Vulkan,
        OpenGL,
        Metal,
        D3D11,
        D3D12,
    };

} // namespace renderer
