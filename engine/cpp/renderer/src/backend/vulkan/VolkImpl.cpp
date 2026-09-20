// Exactly one translation unit must define VOLK_IMPLEMENTATION before
// including volk.h -- this is that file. It compiles volk's loader
// (volk.c, which volk.h pulls in from beside itself) INTO the renderer
// library instead of linking vcpkg's prebuilt volk::volk static library.
//
// Why: volk only defines the function pointers for a platform's
// extensions (vkCreateWin32SurfaceKHR, vkCreateXlibSurfaceKHR,
// vkCreateWaylandSurfaceKHR, vkCreateMetalSurfaceEXT, ...) when the
// matching VK_USE_PLATFORM_* macro is defined while volk.c is compiled.
// vcpkg's volk::volk is built without any of them, so linking it left
// exactly those symbols unresolved as soon as an executable pulled in
// VulkanRenderDevice::CreateSurface() (MSVC: LNK2019 on
// vkCreateWin32SurfaceKHR). The renderer target already defines the right
// VK_USE_PLATFORM_* macros for the platform it is built on (see
// CMakeLists.txt) and this file is part of that target, so volk.c is
// compiled with the same set as every other translation unit that
// includes volk.h.
//
// volk::volk_headers (what CMakeLists.txt links) provides volk.h and
// volk.c on the include path; no prebuilt library is involved.
#define VOLK_IMPLEMENTATION
#include <volk.h>
