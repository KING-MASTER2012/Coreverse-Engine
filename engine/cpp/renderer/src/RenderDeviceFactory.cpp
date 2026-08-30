#include "renderer/RenderDeviceFactory.hpp"

#include <memory>
#include <utility>

#if defined(RENDERER_HAS_VULKAN_BACKEND)
#include "backend/vulkan/VulkanRenderDevice.hpp"
#endif

namespace renderer
{

    std::expected<std::unique_ptr<RenderDevice>, RenderError> CreateRenderDevice(GraphicsAPI api)
    {
        switch (api)
        {
        case GraphicsAPI::Vulkan:
            {
#if defined(RENDERER_HAS_VULKAN_BACKEND)
                auto device = std::make_unique<backend::vulkan::VulkanRenderDevice>();
                if (auto initResult = device->Initialize(); !initResult)
                {
                    return std::unexpected(std::move(initResult.error()));
                }
                return device;
#else
                return std::unexpected(
                    RenderError{RenderErrorCode::BackendUnavailable, "Vulkan backend not compiled into this build"});
#endif
            }

        // Declared in GraphicsAPI now so RenderDeviceFactory's switch shape
        // doesn't change later — each of these gains a
        // `#if defined(RENDERER_HAS_<API>_BACKEND)` arm identical to
        // Vulkan's above once that backend actually exists (Faz 5 only
        // implements Vulkan).
        case GraphicsAPI::OpenGL:
        case GraphicsAPI::Metal:
        case GraphicsAPI::D3D11:
        case GraphicsAPI::D3D12:
            return std::unexpected(
                RenderError{RenderErrorCode::BackendUnavailable, "Backend not implemented yet"});
        }

        return std::unexpected(RenderError{RenderErrorCode::Unknown, "Unrecognized GraphicsAPI value"});
    }

} // namespace renderer
