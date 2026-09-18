// Exactly one translation unit must define VMA_IMPLEMENTATION before
// including vk_mem_alloc.h — this is that file, kept separate from
// VulkanRenderDevice.cpp so the (large, generated) VMA implementation
// doesn't slow down edit/rebuild cycles on the file that actually
// changes during development.
//
// volk.h is included first so VK_NO_PROTOTYPES is already in effect
// before vk_mem_alloc.h's implementation includes <vulkan/vulkan.h>
// itself -- this keeps both headers looking at the same (volk-provided)
// declarations. (VK_NO_PROTOTYPES is now also set project-wide via
// CMakeLists.txt's target_compile_definitions(renderer ...), so this
// ordering is no longer load-bearing on its own -- kept anyway since it
// costs nothing and is still the locally-correct story for this file.)
// VulkanRenderDevice::CreateAllocator() still passes
// vkGetInstanceProcAddr/vkGetDeviceProcAddr explicitly via
// VmaAllocatorCreateInfo::pVulkanFunctions; current VMA requires that
// regardless.
#include <volk.h>

#define VMA_IMPLEMENTATION
#include <vk_mem_alloc.h>
