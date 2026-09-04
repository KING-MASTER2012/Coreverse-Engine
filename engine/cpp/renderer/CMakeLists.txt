# renderer: static library exposing a graphics-API-agnostic interface
# (include/renderer/) with backend implementations under src/backend/.
#
# Only Vulkan is built today (Faz 5); RENDERER_HAS_<API>_BACKEND compile
# definitions gate each backend so RenderDeviceFactory.cpp's switch gains
# a `#if`-guarded case per backend without every platform having to
# build every backend it doesn't have (e.g. no D3D12 on Linux).
#
# find_package(*) below are linked PRIVATE further down — nothing in
# include/renderer/ includes a Vulkan/VMA header, so no target linking
# against `renderer` transitively depends on them.

find_package(VulkanHeaders CONFIG REQUIRED)
find_package(volk CONFIG REQUIRED)
find_package(VulkanMemoryAllocator CONFIG REQUIRED)

add_library(renderer STATIC
        src/RenderDeviceFactory.cpp
        src/Buffer.cpp
        src/Surface.cpp
        src/Swapchain.cpp
        src/CommandBuffer.cpp
        src/backend/vulkan/VulkanRenderDevice.cpp
        src/backend/vulkan/VmaImpl.cpp
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

# Faz 5.3: Vulkan's platform surface structs/functions (VkWin32SurfaceCreateInfoKHR,
# vkCreateXlibSurfaceKHR, ...) are only declared when the matching
# VK_USE_PLATFORM_* macro is defined before <vulkan/vulkan.h> is first
# included — CMake is what actually knows the target platform, so it's
# set here rather than guessed in source. Only Xlib is implemented on
# Linux for now (see VulkanRenderDevice.cpp); XCB/Wayland are not yet.
if(WIN32)
    target_compile_definitions(renderer PRIVATE VK_USE_PLATFORM_WIN32_KHR)
elseif(APPLE)
    target_compile_definitions(renderer PRIVATE VK_USE_PLATFORM_METAL_EXT)
elseif(UNIX)
    target_compile_definitions(renderer PRIVATE VK_USE_PLATFORM_XLIB_KHR)
endif()
