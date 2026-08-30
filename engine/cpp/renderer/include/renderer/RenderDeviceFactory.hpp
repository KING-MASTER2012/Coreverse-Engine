#pragma once

#include <expected>
#include <memory>

#include "renderer/GraphicsAPI.hpp"
#include "renderer/RenderDevice.hpp"
#include "renderer/RenderError.hpp"

namespace renderer
{

    /// Constructs and fully initializes a RenderDevice for the requested
    /// backend, selected at runtime. A single process/platform build can
    /// compile in more than one backend (e.g. Windows: D3D12, D3D11,
    /// Vulkan, OpenGL all available in the same binary) — this is the one
    /// call site that picks among them.
    ///
    /// Fails via the returned std::expected — never throws — if `api`
    /// wasn't compiled into this build, or if the backend's own bring-up
    /// (instance/device creation, adapter enumeration, ...) fails.
    [[nodiscard]] std::expected<std::unique_ptr<RenderDevice>, RenderError> CreateRenderDevice(GraphicsAPI api);

} // namespace renderer
