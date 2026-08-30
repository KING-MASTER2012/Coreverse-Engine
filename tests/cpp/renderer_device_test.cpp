// Faz 5.1 proof: RenderDevice abstraction obtains a Vulkan backend
// device, logs the selected physical device's name, and tears down
// cleanly — entirely through the RenderDevice base pointer. This test
// never names a Vulkan type; it only sees renderer::RenderDevice.

#include <cstdio>
#include <memory>

#include "renderer/RenderDeviceFactory.hpp"

int main()
{
    auto result = renderer::CreateRenderDevice(renderer::GraphicsAPI::Vulkan);
    if (!result)
    {
        std::fprintf(stderr, "CreateRenderDevice failed: %s\n", result.error().detail.c_str());
        return 1;
    }

    std::unique_ptr<renderer::RenderDevice> device = std::move(*result);

    const std::string_view name = device->GetDeviceName();
    std::printf("renderer device: %.*s\n", static_cast<int>(name.size()), name.data());

    device->Shutdown();
    return 0;
}
