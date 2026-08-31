// Faz 5.2 proof: RenderDevice::CreateBuffer() -> Buffer (move-only RAII)
// -> destructor releases it through VMA, entirely through the
// renderer::Buffer/RenderDevice abstraction. Also exercises move
// semantics, since that's the whole point of Buffer being move-only:
// a moved-from Buffer must not double-release when it goes out of
// scope. Run under RENDERER_ENABLE_VALIDATION (Debug builds) so a
// leak or misuse would surface as Vulkan validation output rather
// than passing silently.

#include <cstdio>
#include <memory>
#include <utility>

#include "renderer/RenderDeviceFactory.hpp"

int main()
{
    auto deviceResult = renderer::CreateRenderDevice(renderer::GraphicsAPI::Vulkan);
    if (!deviceResult)
    {
        std::fprintf(stderr, "CreateRenderDevice failed: %s\n", deviceResult.error().detail.c_str());
        return 1;
    }
    std::unique_ptr<renderer::RenderDevice> device = std::move(*deviceResult);

    renderer::BufferDesc desc{};
    desc.size = 256;
    desc.usage = renderer::BufferUsage::TransferSrc | renderer::BufferUsage::TransferDst;
    desc.memoryUsage = renderer::BufferMemoryUsage::CpuToGpu;

    {
        auto bufferResult = device->CreateBuffer(desc);
        if (!bufferResult)
        {
            std::fprintf(stderr, "CreateBuffer failed: %s\n", bufferResult.error().detail.c_str());
            device->Shutdown();
            return 1;
        }

        renderer::Buffer buffer = std::move(*bufferResult);
        if (!buffer.IsValid() || buffer.GetSize() != desc.size)
        {
            std::fprintf(stderr, "Buffer round-trip produced an unexpected state\n");
            device->Shutdown();
            return 1;
        }

        // Exercise move semantics: the moved-from Buffer must report
        // itself invalid so its (no-op) destructor doesn't try to
        // release anything a second time.
        renderer::Buffer moved = std::move(buffer);
        if (buffer.IsValid())
        {
            std::fprintf(stderr, "Moved-from Buffer still reports valid\n");
            device->Shutdown();
            return 1;
        }

        // `moved` releases the buffer here, at end of scope — before
        // device->Shutdown() below, per the ordering rule RenderDevice.hpp
        // documents.
    }

    std::printf("buffer round-trip ok (%zu bytes)\n", desc.size);
    device->Shutdown();
    return 0;
}
