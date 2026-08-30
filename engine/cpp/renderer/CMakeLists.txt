# renderer: static library exposing a graphics-API-agnostic interface
# (include/renderer/) with backend implementations under src/backend/.
#
# Only Vulkan is built today (Faz 5); RENDERER_HAS_<API>_BACKEND compile
# definitions gate each backend so RenderDeviceFactory.cpp's switch gains
# a `#if`-guarded case per backend without every platform having to
# build every backend it doesn't have (e.g. no D3D12 on Linux).
#
# find_package(*) below are linked PRIVATE further down — nothing in
# include/renderer/ includes a Vulkan (or any backend) header, so no
# target linking against `renderer` transitively depends on them.

find_package(VulkanHeaders CONFIG REQUIRED)
find_package(volk CONFIG REQUIRED)
find_package(VulkanMemoryAllocator CONFIG REQUIRED)

add_library(renderer STATIC
        src/RenderDeviceFactory.cpp
        src/backend/vulkan/VulkanRenderDevice.cpp
)

add_library(renderer::renderer ALIAS renderer)

target_include_directories(renderer
        PUBLIC
        ${CMAKE_CURRENT_SOURCE_DIR}/include
        PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}/src
)

target_compile_features(renderer PUBLIC cxx_std_23)

target_link_libraries(renderer
        PRIVATE
        compiler_warnings
        Vulkan::Headers
        volk::volk
        GPUOpen::VulkanMemoryAllocator
)

target_compile_definitions(renderer
        PRIVATE
        RENDERER_HAS_VULKAN_BACKEND
        $<$<CONFIG:Debug>:RENDERER_ENABLE_VALIDATION>
)
