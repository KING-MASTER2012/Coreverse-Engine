// Exactly one translation unit must define VMA_IMPLEMENTATION before
// including vk_mem_alloc.h — this is that file, kept separate from
// VulkanRenderDevice.cpp so the (large, generated) VMA implementation
// doesn't slow down edit/rebuild cycles on the file that actually
// changes during development.
//
// volk.h is included first so VK_NO_PROTOTYPES is already in effect
// before vk_mem_alloc.h's implementation includes <vulkan/vulkan.h>
// itself — this keeps both headers looking at the same (volk-provided)
// declarations. VulkanRenderDevice::CreateAllocator() still passes
// vkGetInstanceProcAddr/vkGetDeviceProcAddr explicitly via
// VmaAllocatorCreateInfo::pVulkanFunctions; current VMA requires that
// regardless.
#include <volk.h>

#define VMA_IMPLEMENTATION
#include <vk_mem_alloc.h>
