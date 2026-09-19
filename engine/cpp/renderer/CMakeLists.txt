# renderer: static library exposing a graphics-API-agnostic interface
# (include/renderer/) with backend implementations under src/backend/.
#
# Only Vulkan is built today (Phase 5); RENDERER_HAS_<API>_BACKEND compile
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
        # Defined here (compile definition) rather than relied upon via
        # include order: volk.h itself only defines this *after* its own
        # "was <vulkan/vulkan.h> already included?" check, so anything
        # that pulls in the full umbrella header ahead of volk.h in any
        # translation unit of this target -- directly or transitively,
        # now or in a future file -- would otherwise hit
        # "you need to define VK_NO_PROTOTYPES before including
        # vulkan.h". A compile-line definition applies before the very
        # first header in every TU, so this holds regardless of include
        # order. See VmaImpl.cpp's comment for the (now secondary,
        # belt-and-suspenders) same-file ordering.
        VK_NO_PROTOTYPES
        $<$<CONFIG:Debug>:RENDERER_ENABLE_VALIDATION>
)

# Phase 5.3: Vulkan's platform surface structs/functions (VkWin32SurfaceCreateInfoKHR,
# vkCreateXlibSurfaceKHR, ...) are only declared when the matching
# VK_USE_PLATFORM_* macro is defined before <vulkan/vulkan.h> is first
# included — CMake is what actually knows the target platform, so it's
# set here rather than guessed in source.
if(WIN32)
    # windows.h (pulled in transitively by <vulkan/vulkan.h> once
    # VK_USE_PLATFORM_WIN32_KHR is defined - see vulkan.h's own
    # "#ifdef VK_USE_PLATFORM_WIN32_KHR / #include <windows.h>" branch)
    # defines `max`/`min` function-like macros unless NOMINMAX is defined
    # first. Left unguarded, any later `std::max`/`std::min` call in this
    # translation unit silently expands through that macro instead - MSVC
    # reports it as "C2589: '(': illegal token on right side of '::'"
    # pointing at the `std::max`/`std::min` call itself, which is a
    # confusing way to find out windows.h did this. NOMINMAX must be
    # defined before windows.h's first include, so it goes on the
    # compiler command line (via target_compile_definitions) rather than
    # relying on include order in the .cpp files. WIN32_LEAN_AND_MEAN is
    # the standard companion define - trims windows.h down to the common
    # subset, avoiding a similar class of surprise macro/name collisions
    # from the parts of it this renderer never needed anyway.
    target_compile_definitions(renderer PRIVATE VK_USE_PLATFORM_WIN32_KHR NOMINMAX WIN32_LEAN_AND_MEAN)
elseif(APPLE)
    target_compile_definitions(renderer PRIVATE VK_USE_PLATFORM_METAL_EXT)
elseif(UNIX)
    # Phase 5 (Wayland milestone): Xlib and Wayland are independent WSI
    # extensions — Vulkan-Headers allows VK_USE_PLATFORM_XLIB_KHR and
    # VK_USE_PLATFORM_WAYLAND_KHR to be defined together, and
    # vulkan_wayland.h only forward-declares `struct wl_display` /
    # `struct wl_surface` itself (see VulkanRenderDevice.cpp), so no
    # wayland-client vcpkg dependency is needed just to build against
    # it. Both default ON so a plain Linux build supports either
    # windowing system out of the box; a packager/CI leg that only
    # cares about one can turn the other off.
    option(RENDERER_ENABLE_XLIB    "Enable the Xlib (X11) Vulkan surface backend on Linux" ON)
    option(RENDERER_ENABLE_WAYLAND "Enable the Wayland Vulkan surface backend on Linux" ON)

    if(NOT RENDERER_ENABLE_XLIB AND NOT RENDERER_ENABLE_WAYLAND)
        message(FATAL_ERROR "renderer: at least one of RENDERER_ENABLE_XLIB / RENDERER_ENABLE_WAYLAND must be ON on Linux")
    endif()

    if(RENDERER_ENABLE_XLIB)
        target_compile_definitions(renderer PRIVATE VK_USE_PLATFORM_XLIB_KHR)
    endif()
    if(RENDERER_ENABLE_WAYLAND)
        target_compile_definitions(renderer PRIVATE VK_USE_PLATFORM_WAYLAND_KHR)
    endif()
endif()
