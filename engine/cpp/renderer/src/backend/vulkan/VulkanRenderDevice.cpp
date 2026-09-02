#include "backend/vulkan/VulkanRenderDevice.hpp"

#include <algorithm>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

namespace renderer::backend::vulkan
{

namespace
{

constexpr const char* kValidationLayerName = "VK_LAYER_KHRONOS_validation";

/// Pairs the two handles VMA needs to free a buffer. Buffer.hpp only
/// stores a backend-agnostic void*, so this is what that void* actually
/// points at for the Vulkan backend; only CreateBuffer/ReleaseBuffer
/// (this file) ever interpret it.
struct VulkanBufferHandle
{
    VkBuffer buffer;
    VmaAllocation allocation;
};

/// Everything Faz 5.4's swapchain owns beyond the VkSwapchainKHR handle
/// itself. Buffer.hpp/Surface.hpp/Swapchain.hpp only store a backend-
/// agnostic void*, so this is what that void* actually points at for
/// the Vulkan backend; only CreateSwapchain/ReleaseSwapchain/
/// AcquireSwapchainImage/PresentSwapchainImage (this file) ever
/// interpret it.
struct VulkanSwapchainHandle
{
    VkSwapchainKHR swapchain = VK_NULL_HANDLE;
    VkFormat format = VK_FORMAT_UNDEFINED;
    VkExtent2D extent{};
    std::vector<VkImage> images;
    std::vector<VkImageView> imageViews;
};

VKAPI_ATTR VkBool32 VKAPI_CALL DebugMessengerCallback(VkDebugUtilsMessageSeverityFlagBitsEXT severity,
                                                       VkDebugUtilsMessageTypeFlagsEXT /*messageType*/,
                                                       const VkDebugUtilsMessengerCallbackDataEXT* callbackData,
                                                       void* /*userData*/)
{
    // Faz 5.1/5.2 only need validation output to land somewhere visible
    // so leak/misuse checks aren't silent; this gets routed through
    // cv-log once the renderer is wired to ffi (later phase).
    if (severity >= VK_DEBUG_UTILS_MESSAGE_SEVERITY_WARNING_BIT_EXT)
    {
        std::fprintf(stderr, "[vulkan] %s\n", callbackData->pMessage);
    }
    return VK_FALSE;
}

} // namespace

VulkanRenderDevice::~VulkanRenderDevice()
{
    Shutdown();
}

bool VulkanRenderDevice::ValidationLayersRequestedAndSupported()
{
#if !defined(RENDERER_ENABLE_VALIDATION)
    return false;
#else
    uint32_t layerCount = 0;
    vkEnumerateInstanceLayerProperties(&layerCount, nullptr);
    std::vector<VkLayerProperties> layers(layerCount);
    vkEnumerateInstanceLayerProperties(&layerCount, layers.data());

    for (const auto& layer : layers)
    {
        if (std::strcmp(layer.layerName, kValidationLayerName) == 0)
        {
            return true;
        }
    }
    // vulkan-validation is an *optional* vcpkg feature (vcpkg.json) —
    // not being installed is not an error, it just means we silently
    // run without validation rather than failing bring-up over it.
    return false;
#endif
}

std::expected<void, RenderError> VulkanRenderDevice::Initialize()
{
    if (const VkResult volkResult = volkInitialize(); volkResult != VK_SUCCESS)
    {
        return std::unexpected(
            RenderError{RenderErrorCode::InitializationFailed,
                        "volkInitialize failed (VkResult=" + std::to_string(volkResult) + ")"});
    }

    if (auto result = CreateInstance(); !result)
    {
        return result;
    }

    if (m_validationEnabled)
    {
        if (auto result = SetupDebugMessenger(); !result)
        {
            return result;
        }
    }

    if (auto result = SelectPhysicalDevice(); !result)
    {
        return result;
    }

    if (auto result = CreateLogicalDeviceAndQueues(); !result)
    {
        return result;
    }

    if (auto result = CreateAllocator(); !result)
    {
        return result;
    }

    return {};
}

std::expected<void, RenderError> VulkanRenderDevice::CreateInstance()
{
    m_validationEnabled = ValidationLayersRequestedAndSupported();

    VkApplicationInfo appInfo{};
    appInfo.sType = VK_STRUCTURE_TYPE_APPLICATION_INFO;
    appInfo.pApplicationName = "CoreVerse";
    appInfo.applicationVersion = VK_MAKE_VERSION(0, 1, 0);
    appInfo.pEngineName = "CoreVerse Engine";
    appInfo.engineVersion = VK_MAKE_VERSION(0, 1, 0);
    appInfo.apiVersion = VK_API_VERSION_1_3;

    std::vector<const char*> extensions;
    // VK_KHR_surface plus the one platform-specific WSI extension this
    // build was compiled for are required unconditionally — surface
    // creation (Faz 5.3) needs them regardless of whether validation is
    // enabled. Only one of the platform branches below is compiled in
    // per build, matching NativeWindowHandle's #if ladder in Surface.hpp.
    // Linux: only Xlib is wired up for now (see CreateSurface()) —
    // Wayland's fields exist on NativeWindowHandle for forward
    // compatibility but aren't implemented yet, so its extension isn't
    // requested here either.
    extensions.push_back(VK_KHR_SURFACE_EXTENSION_NAME);
#if defined(_WIN32)
    extensions.push_back(VK_KHR_WIN32_SURFACE_EXTENSION_NAME);
#elif defined(__APPLE__)
    extensions.push_back(VK_EXT_METAL_SURFACE_EXTENSION_NAME);
#elif defined(__linux__)
    extensions.push_back(VK_KHR_XLIB_SURFACE_EXTENSION_NAME);
#endif
    if (m_validationEnabled)
    {
        extensions.push_back(VK_EXT_DEBUG_UTILS_EXTENSION_NAME);
    }

    std::vector<const char*> layers;
    if (m_validationEnabled)
    {
        layers.push_back(kValidationLayerName);
    }

    VkInstanceCreateInfo createInfo{};
    createInfo.sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO;
    createInfo.pApplicationInfo = &appInfo;
    createInfo.enabledExtensionCount = static_cast<uint32_t>(extensions.size());
    createInfo.ppEnabledExtensionNames = extensions.data();
    createInfo.enabledLayerCount = static_cast<uint32_t>(layers.size());
    createInfo.ppEnabledLayerNames = layers.data();

    if (const VkResult result = vkCreateInstance(&createInfo, nullptr, &m_instance); result != VK_SUCCESS)
    {
        return std::unexpected(RenderError{RenderErrorCode::InitializationFailed,
                                            "vkCreateInstance failed (VkResult=" + std::to_string(result) + ")"});
    }

    volkLoadInstance(m_instance);
    return {};
}

std::expected<void, RenderError> VulkanRenderDevice::SetupDebugMessenger()
{
    VkDebugUtilsMessengerCreateInfoEXT createInfo{};
    createInfo.sType = VK_STRUCTURE_TYPE_DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT;
    createInfo.messageSeverity =
        VK_DEBUG_UTILS_MESSAGE_SEVERITY_WARNING_BIT_EXT | VK_DEBUG_UTILS_MESSAGE_SEVERITY_ERROR_BIT_EXT;
    createInfo.messageType = VK_DEBUG_UTILS_MESSAGE_TYPE_GENERAL_BIT_EXT |
                              VK_DEBUG_UTILS_MESSAGE_TYPE_VALIDATION_BIT_EXT |
                              VK_DEBUG_UTILS_MESSAGE_TYPE_PERFORMANCE_BIT_EXT;
    createInfo.pfnUserCallback = DebugMessengerCallback;

    // volk loads extension entry points (incl. vkCreateDebugUtilsMessengerEXT)
    // automatically once volkLoadInstance() ran with the extension enabled —
    // no manual vkGetInstanceProcAddr lookup needed.
    if (const VkResult result = vkCreateDebugUtilsMessengerEXT(m_instance, &createInfo, nullptr, &m_debugMessenger);
        result != VK_SUCCESS)
    {
        return std::unexpected(
            RenderError{RenderErrorCode::InitializationFailed, "vkCreateDebugUtilsMessengerEXT failed"});
    }
    return {};
}

std::expected<void, RenderError> VulkanRenderDevice::SelectPhysicalDevice()
{
    uint32_t deviceCount = 0;
    vkEnumeratePhysicalDevices(m_instance, &deviceCount, nullptr);
    if (deviceCount == 0)
    {
        return std::unexpected(
            RenderError{RenderErrorCode::NoSuitableDevice, "No Vulkan-capable physical devices found"});
    }

    std::vector<VkPhysicalDevice> devices(deviceCount);
    vkEnumeratePhysicalDevices(m_instance, &deviceCount, devices.data());

    // Prefer a discrete GPU; fall back to whatever enumerates first
    // (integrated GPU, software rasterizer, ...) rather than failing —
    // Faz 5 just needs *a* working device, not the best one.
    VkPhysicalDevice fallback = VK_NULL_HANDLE;
    for (VkPhysicalDevice candidate : devices)
    {
        VkPhysicalDeviceProperties props{};
        vkGetPhysicalDeviceProperties(candidate, &props);

        if (fallback == VK_NULL_HANDLE)
        {
            fallback = candidate;
        }

        if (props.deviceType == VK_PHYSICAL_DEVICE_TYPE_DISCRETE_GPU)
        {
            m_physicalDevice = candidate;
            m_deviceName = props.deviceName;
            break;
        }
    }

    if (m_physicalDevice == VK_NULL_HANDLE)
    {
        m_physicalDevice = fallback;
        VkPhysicalDeviceProperties props{};
        vkGetPhysicalDeviceProperties(m_physicalDevice, &props);
        m_deviceName = props.deviceName;
    }

    uint32_t familyCount = 0;
    vkGetPhysicalDeviceQueueFamilyProperties(m_physicalDevice, &familyCount, nullptr);
    std::vector<VkQueueFamilyProperties> families(familyCount);
    vkGetPhysicalDeviceQueueFamilyProperties(m_physicalDevice, &familyCount, families.data());

    for (uint32_t i = 0; i < familyCount; ++i)
    {
        if (families[i].queueFlags & VK_QUEUE_GRAPHICS_BIT)
        {
            m_graphicsQueueFamily = i;
            break;
        }
    }

    if (m_graphicsQueueFamily == UINT32_MAX)
    {
        return std::unexpected(
            RenderError{RenderErrorCode::NoSuitableDevice, "Selected device has no graphics-capable queue family"});
    }

    return {};
}

std::expected<void, RenderError> VulkanRenderDevice::CreateLogicalDeviceAndQueues()
{
    constexpr float queuePriority = 1.0f;

    VkDeviceQueueCreateInfo queueCreateInfo{};
    queueCreateInfo.sType = VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO;
    queueCreateInfo.queueFamilyIndex = m_graphicsQueueFamily;
    queueCreateInfo.queueCount = 1;
    queueCreateInfo.pQueuePriorities = &queuePriority;

    VkDeviceCreateInfo deviceCreateInfo{};
    deviceCreateInfo.sType = VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO;
    deviceCreateInfo.queueCreateInfoCount = 1;
    deviceCreateInfo.pQueueCreateInfos = &queueCreateInfo;

    // VK_KHR_swapchain is a device extension, not an instance one — it
    // has to be requested here, or vkCreateSwapchainKHR and the rest of
    // Faz 5.4's swapchain calls are never loaded (volk leaves them null,
    // which crashes on the first call rather than failing cleanly).
    static constexpr const char* kDeviceExtensions[] = {VK_KHR_SWAPCHAIN_EXTENSION_NAME};
    deviceCreateInfo.enabledExtensionCount = 1;
    deviceCreateInfo.ppEnabledExtensionNames = kDeviceExtensions;

    // Device-layer validation is deprecated (instance-layer validation
    // has covered everything since Vulkan 1.1) but harmless to set for
    // older loaders that might still be present — cheap to keep.
    std::vector<const char*> layers;
    if (m_validationEnabled)
    {
        layers.push_back(kValidationLayerName);
    }
    deviceCreateInfo.enabledLayerCount = static_cast<uint32_t>(layers.size());
    deviceCreateInfo.ppEnabledLayerNames = layers.data();

    if (const VkResult result = vkCreateDevice(m_physicalDevice, &deviceCreateInfo, nullptr, &m_device);
        result != VK_SUCCESS)
    {
        return std::unexpected(RenderError{RenderErrorCode::InitializationFailed,
                                            "vkCreateDevice failed (VkResult=" + std::to_string(result) + ")"});
    }

    volkLoadDevice(m_device);
    vkGetDeviceQueue(m_device, m_graphicsQueueFamily, 0, &m_graphicsQueue);

    return {};
}

std::expected<void, RenderError> VulkanRenderDevice::CreateAllocator()
{
    // VMA's dynamic function loading no longer auto-detects volk; it
    // requires vkGetInstanceProcAddr/vkGetDeviceProcAddr explicitly via
    // pVulkanFunctions (everything else it needs, it resolves itself
    // from those two). Both are volk globals, already loaded by the
    // volkInitialize()/volkLoadInstance() calls earlier in Initialize().
    VmaVulkanFunctions vulkanFunctions{};
    vulkanFunctions.vkGetInstanceProcAddr = vkGetInstanceProcAddr;
    vulkanFunctions.vkGetDeviceProcAddr = vkGetDeviceProcAddr;

    VmaAllocatorCreateInfo allocatorInfo{};
    allocatorInfo.vulkanApiVersion = VK_API_VERSION_1_3;
    allocatorInfo.physicalDevice = m_physicalDevice;
    allocatorInfo.device = m_device;
    allocatorInfo.instance = m_instance;
    allocatorInfo.pVulkanFunctions = &vulkanFunctions;

    if (const VkResult result = vmaCreateAllocator(&allocatorInfo, &m_allocator); result != VK_SUCCESS)
    {
        return std::unexpected(RenderError{RenderErrorCode::InitializationFailed,
                                            "vmaCreateAllocator failed (VkResult=" + std::to_string(result) + ")"});
    }
    return {};
}

std::expected<Buffer, RenderError> VulkanRenderDevice::CreateBuffer(const BufferDesc& desc) noexcept
{
    if (desc.size == 0)
    {
        return std::unexpected(RenderError{RenderErrorCode::InitializationFailed, "Buffer size must be non-zero"});
    }

    VkBufferUsageFlags usageFlags = 0;
    if (HasFlag(desc.usage, BufferUsage::VertexBuffer))
    {
        usageFlags |= VK_BUFFER_USAGE_VERTEX_BUFFER_BIT;
    }
    if (HasFlag(desc.usage, BufferUsage::IndexBuffer))
    {
        usageFlags |= VK_BUFFER_USAGE_INDEX_BUFFER_BIT;
    }
    if (HasFlag(desc.usage, BufferUsage::UniformBuffer))
    {
        usageFlags |= VK_BUFFER_USAGE_UNIFORM_BUFFER_BIT;
    }
    if (HasFlag(desc.usage, BufferUsage::TransferSrc))
    {
        usageFlags |= VK_BUFFER_USAGE_TRANSFER_SRC_BIT;
    }
    if (HasFlag(desc.usage, BufferUsage::TransferDst))
    {
        usageFlags |= VK_BUFFER_USAGE_TRANSFER_DST_BIT;
    }

    VkBufferCreateInfo bufferInfo{};
    bufferInfo.sType = VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO;
    bufferInfo.size = static_cast<VkDeviceSize>(desc.size);
    bufferInfo.usage = usageFlags;
    bufferInfo.sharingMode = VK_SHARING_MODE_EXCLUSIVE;

    VmaAllocationCreateInfo allocInfo{};
    switch (desc.memoryUsage)
    {
    case BufferMemoryUsage::GpuOnly:
        allocInfo.usage = VMA_MEMORY_USAGE_AUTO_PREFER_DEVICE;
        break;
    case BufferMemoryUsage::CpuToGpu:
        allocInfo.usage = VMA_MEMORY_USAGE_AUTO_PREFER_DEVICE;
        allocInfo.flags = VMA_ALLOCATION_CREATE_HOST_ACCESS_SEQUENTIAL_WRITE_BIT;
        break;
    case BufferMemoryUsage::GpuToCpu:
        allocInfo.usage = VMA_MEMORY_USAGE_AUTO_PREFER_HOST;
        allocInfo.flags = VMA_ALLOCATION_CREATE_HOST_ACCESS_RANDOM_BIT;
        break;
    }

    VkBuffer buffer = VK_NULL_HANDLE;
    VmaAllocation allocation = VK_NULL_HANDLE;
    if (const VkResult result = vmaCreateBuffer(m_allocator, &bufferInfo, &allocInfo, &buffer, &allocation, nullptr);
        result != VK_SUCCESS)
    {
        return std::unexpected(RenderError{RenderErrorCode::OutOfMemory,
                                            "vmaCreateBuffer failed (VkResult=" + std::to_string(result) + ")"});
    }

    // Buffer only stores a backend-agnostic void*; this heap-allocated
    // pair is what the Vulkan backend puts behind it so ReleaseBuffer()
    // can hand both handles back to VMA later.
    auto* handle = new VulkanBufferHandle{buffer, allocation};
    return RenderDevice::MakeBuffer(this, handle, desc.size);
}

void VulkanRenderDevice::ReleaseBuffer(void* nativeHandle) noexcept
{
    if (nativeHandle == nullptr)
    {
        return;
    }
    auto* handle = static_cast<VulkanBufferHandle*>(nativeHandle);
    vmaDestroyBuffer(m_allocator, handle->buffer, handle->allocation);
    delete handle;
}

std::expected<Surface, RenderError> VulkanRenderDevice::CreateSurface(const NativeWindowHandle& handle) noexcept
{
    VkSurfaceKHR surface = VK_NULL_HANDLE;
    VkResult result = VK_ERROR_EXTENSION_NOT_PRESENT;

#if defined(_WIN32)
    if (handle.hwnd != nullptr)
    {
        VkWin32SurfaceCreateInfoKHR createInfo{};
        createInfo.sType = VK_STRUCTURE_TYPE_WIN32_SURFACE_CREATE_INFO_KHR;
        createInfo.hinstance = static_cast<HINSTANCE>(handle.hinstance);
        createInfo.hwnd = static_cast<HWND>(handle.hwnd);
        result = vkCreateWin32SurfaceKHR(m_instance, &createInfo, nullptr, &surface);
    }
#elif defined(__APPLE__)
    if (handle.metalLayer != nullptr)
    {
        VkMetalSurfaceCreateInfoEXT createInfo{};
        createInfo.sType = VK_STRUCTURE_TYPE_METAL_SURFACE_CREATE_INFO_EXT;
        createInfo.pLayer = static_cast<const CAMetalLayer*>(handle.metalLayer);
        result = vkCreateMetalSurfaceEXT(m_instance, &createInfo, nullptr, &surface);
    }
#elif defined(__linux__)
    // Only Xlib is implemented for now — NativeWindowHandle's Wayland
    // fields exist so Surface.hpp doesn't need to change shape when
    // Wayland support is actually added, matching GraphicsAPI.hpp's
    // "declare now, implement later" convention.
    if (handle.xlibDisplay != nullptr)
    {
        VkXlibSurfaceCreateInfoKHR createInfo{};
        createInfo.sType = VK_STRUCTURE_TYPE_XLIB_SURFACE_CREATE_INFO_KHR;
        createInfo.dpy = static_cast<Display*>(handle.xlibDisplay);
        createInfo.window = static_cast<Window>(handle.xlibWindow);
        result = vkCreateXlibSurfaceKHR(m_instance, &createInfo, nullptr, &surface);
    }
#endif

    if (result != VK_SUCCESS)
    {
        return std::unexpected(RenderError{RenderErrorCode::InitializationFailed,
                                            "surface creation failed (VkResult=" + std::to_string(result) + ")"});
    }

    return RenderDevice::MakeSurface(this, surface);
}

void VulkanRenderDevice::ReleaseSurface(void* nativeHandle) noexcept
{
    if (nativeHandle == nullptr)
    {
        return;
    }
    vkDestroySurfaceKHR(m_instance, static_cast<VkSurfaceKHR>(nativeHandle), nullptr);
}

std::expected<Swapchain, RenderError> VulkanRenderDevice::CreateSwapchain(const Surface& surface,
                                                                           const SwapchainDesc& desc) noexcept
{
    if (!surface.IsValid())
    {
        return std::unexpected(
            RenderError{RenderErrorCode::InitializationFailed, "CreateSwapchain called with an invalid Surface"});
    }
    const auto vkSurface = static_cast<VkSurfaceKHR>(surface.GetNativeHandle());

    // Faz 5.1 picked the graphics queue family without checking present
    // support, since there was no surface yet to check it against. Now
    // that there is one, verify it — a queue family that can't present
    // to this surface makes the whole swapchain unusable, so this fails
    // loudly here rather than at some confusing point later.
    VkBool32 presentSupported = VK_FALSE;
    if (const VkResult result = vkGetPhysicalDeviceSurfaceSupportKHR(m_physicalDevice, m_graphicsQueueFamily,
                                                                      vkSurface, &presentSupported);
        result != VK_SUCCESS || presentSupported == VK_FALSE)
    {
        return std::unexpected(RenderError{RenderErrorCode::NoSuitableDevice,
                                            "graphics queue family does not support presenting to this surface"});
    }

    VkSurfaceCapabilitiesKHR capabilities{};
    if (const VkResult result = vkGetPhysicalDeviceSurfaceCapabilitiesKHR(m_physicalDevice, vkSurface, &capabilities);
        result != VK_SUCCESS)
    {
        return std::unexpected(
            RenderError{RenderErrorCode::InitializationFailed, "vkGetPhysicalDeviceSurfaceCapabilitiesKHR failed"});
    }

    uint32_t formatCount = 0;
    vkGetPhysicalDeviceSurfaceFormatsKHR(m_physicalDevice, vkSurface, &formatCount, nullptr);
    if (formatCount == 0)
    {
        return std::unexpected(RenderError{RenderErrorCode::NoSuitableDevice, "surface exposes no formats"});
    }
    std::vector<VkSurfaceFormatKHR> formats(formatCount);
    vkGetPhysicalDeviceSurfaceFormatsKHR(m_physicalDevice, vkSurface, &formatCount, formats.data());

    VkSurfaceFormatKHR chosenFormat = formats[0];
    for (const VkSurfaceFormatKHR& candidate : formats)
    {
        if (candidate.format == VK_FORMAT_B8G8R8A8_SRGB && candidate.colorSpace == VK_COLOR_SPACE_SRGB_NONLINEAR_KHR)
        {
            chosenFormat = candidate;
            break;
        }
    }

    uint32_t presentModeCount = 0;
    vkGetPhysicalDeviceSurfacePresentModesKHR(m_physicalDevice, vkSurface, &presentModeCount, nullptr);
    std::vector<VkPresentModeKHR> presentModes(presentModeCount);
    vkGetPhysicalDeviceSurfacePresentModesKHR(m_physicalDevice, vkSurface, &presentModeCount, presentModes.data());

    // FIFO is the only mode every Vulkan implementation is required to
    // support (VK_PRESENT_MODE_FIFO_KHR); prefer MAILBOX (low-latency,
    // no tearing) when it's actually available.
    VkPresentModeKHR chosenPresentMode = VK_PRESENT_MODE_FIFO_KHR;
    for (VkPresentModeKHR mode : presentModes)
    {
        if (mode == VK_PRESENT_MODE_MAILBOX_KHR)
        {
            chosenPresentMode = mode;
            break;
        }
    }

    VkExtent2D extent{};
    if (capabilities.currentExtent.width != UINT32_MAX)
    {
        // The surface dictates its own extent (the common case for a
        // real window) — desc.width/height are ignored in favor of it.
        extent = capabilities.currentExtent;
    }
    else
    {
        extent.width =
            std::clamp(desc.width, capabilities.minImageExtent.width, capabilities.maxImageExtent.width);
        extent.height =
            std::clamp(desc.height, capabilities.minImageExtent.height, capabilities.maxImageExtent.height);
    }

    uint32_t imageCount = std::max(desc.preferredImageCount, capabilities.minImageCount);
    if (capabilities.maxImageCount > 0)
    {
        imageCount = std::min(imageCount, capabilities.maxImageCount);
    }

    VkSwapchainCreateInfoKHR createInfo{};
    createInfo.sType = VK_STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR;
    createInfo.surface = vkSurface;
    createInfo.minImageCount = imageCount;
    createInfo.imageFormat = chosenFormat.format;
    createInfo.imageColorSpace = chosenFormat.colorSpace;
    createInfo.imageExtent = extent;
    createInfo.imageArrayLayers = 1;
    // COLOR_ATTACHMENT for a normal render-pass-based draw, TRANSFER_DST
    // so Faz 5.5's "clear to a solid color" proof can use either a
    // render pass clear or a plain vkCmdClearColorImage — left open on
    // purpose rather than betting on which one 5.5 picks.
    createInfo.imageUsage = VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT | VK_IMAGE_USAGE_TRANSFER_DST_BIT;
    createInfo.imageSharingMode = VK_SHARING_MODE_EXCLUSIVE;
    createInfo.preTransform = capabilities.currentTransform;
    createInfo.compositeAlpha = VK_COMPOSITE_ALPHA_OPAQUE_BIT_KHR;
    createInfo.presentMode = chosenPresentMode;
    createInfo.clipped = VK_TRUE;
    createInfo.oldSwapchain = VK_NULL_HANDLE;

    VkSwapchainKHR swapchain = VK_NULL_HANDLE;
    if (const VkResult result = vkCreateSwapchainKHR(m_device, &createInfo, nullptr, &swapchain);
        result != VK_SUCCESS)
    {
        return std::unexpected(RenderError{RenderErrorCode::InitializationFailed,
                                            "vkCreateSwapchainKHR failed (VkResult=" + std::to_string(result) + ")"});
    }

    uint32_t actualImageCount = 0;
    vkGetSwapchainImagesKHR(m_device, swapchain, &actualImageCount, nullptr);
    std::vector<VkImage> images(actualImageCount);
    vkGetSwapchainImagesKHR(m_device, swapchain, &actualImageCount, images.data());

    std::vector<VkImageView> imageViews(actualImageCount, VK_NULL_HANDLE);
    for (uint32_t i = 0; i < actualImageCount; ++i)
    {
        VkImageViewCreateInfo viewInfo{};
        viewInfo.sType = VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO;
        viewInfo.image = images[i];
        viewInfo.viewType = VK_IMAGE_VIEW_TYPE_2D;
        viewInfo.format = chosenFormat.format;
        viewInfo.components = {VK_COMPONENT_SWIZZLE_IDENTITY, VK_COMPONENT_SWIZZLE_IDENTITY,
                                VK_COMPONENT_SWIZZLE_IDENTITY, VK_COMPONENT_SWIZZLE_IDENTITY};
        viewInfo.subresourceRange.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
        viewInfo.subresourceRange.baseMipLevel = 0;
        viewInfo.subresourceRange.levelCount = 1;
        viewInfo.subresourceRange.baseArrayLayer = 0;
        viewInfo.subresourceRange.layerCount = 1;

        if (const VkResult result = vkCreateImageView(m_device, &viewInfo, nullptr, &imageViews[i]);
            result != VK_SUCCESS)
        {
            // Roll back everything already created before bailing —
            // this is still inside CreateSwapchain(), so nothing has
            // been handed to the caller yet for them to release.
            for (uint32_t created = 0; created < i; ++created)
            {
                vkDestroyImageView(m_device, imageViews[created], nullptr);
            }
            vkDestroySwapchainKHR(m_device, swapchain, nullptr);
            return std::unexpected(
                RenderError{RenderErrorCode::InitializationFailed, "vkCreateImageView failed for a swapchain image"});
        }
    }

    auto* handle = new VulkanSwapchainHandle{swapchain, chosenFormat.format, extent, std::move(images),
                                              std::move(imageViews)};
    return RenderDevice::MakeSwapchain(this, handle, actualImageCount);
}

void VulkanRenderDevice::ReleaseSwapchain(void* nativeHandle) noexcept
{
    if (nativeHandle == nullptr)
    {
        return;
    }
    auto* handle = static_cast<VulkanSwapchainHandle*>(nativeHandle);
    for (VkImageView view : handle->imageViews)
    {
        vkDestroyImageView(m_device, view, nullptr);
    }
    vkDestroySwapchainKHR(m_device, handle->swapchain, nullptr);
    delete handle;
}

std::expected<AcquireResult, RenderError> VulkanRenderDevice::AcquireSwapchainImage(void* nativeHandle) noexcept
{
    auto* handle = static_cast<VulkanSwapchainHandle*>(nativeHandle);

    // Vulkan requires at least one of {semaphore, fence} to be valid.
    // Faz 5.4 has no frame-submission architecture yet to hand this a
    // semaphore to signal into, so it uses a fence and waits on it
    // itself, making Acquire() self-contained and synchronous for now.
    // Faz 5.5's render loop is expected to pass its own semaphore
    // through a refined call path once there's real work to wait on it.
    VkFenceCreateInfo fenceInfo{};
    fenceInfo.sType = VK_STRUCTURE_TYPE_FENCE_CREATE_INFO;
    VkFence fence = VK_NULL_HANDLE;
    if (const VkResult result = vkCreateFence(m_device, &fenceInfo, nullptr, &fence); result != VK_SUCCESS)
    {
        return std::unexpected(RenderError{RenderErrorCode::InitializationFailed, "vkCreateFence failed"});
    }

    uint32_t imageIndex = 0;
    const VkResult acquireResult =
        vkAcquireNextImageKHR(m_device, handle->swapchain, UINT64_MAX, VK_NULL_HANDLE, fence, &imageIndex);

    if (acquireResult == VK_SUCCESS || acquireResult == VK_SUBOPTIMAL_KHR)
    {
        vkWaitForFences(m_device, 1, &fence, VK_TRUE, UINT64_MAX);
    }
    vkDestroyFence(m_device, fence, nullptr);

    switch (acquireResult)
    {
    case VK_SUCCESS:
        return AcquireResult{imageIndex, SwapchainStatus::Ok};
    case VK_SUBOPTIMAL_KHR:
        return AcquireResult{imageIndex, SwapchainStatus::Suboptimal};
    case VK_ERROR_OUT_OF_DATE_KHR:
        return AcquireResult{0, SwapchainStatus::OutOfDate};
    default:
        return std::unexpected(RenderError{RenderErrorCode::Unknown, "vkAcquireNextImageKHR failed (VkResult=" +
                                                                          std::to_string(acquireResult) + ")"});
    }
}

std::expected<SwapchainStatus, RenderError>
VulkanRenderDevice::PresentSwapchainImage(void* nativeHandle, std::uint32_t imageIndex, void* waitSemaphore) noexcept
{
    auto* handle = static_cast<VulkanSwapchainHandle*>(nativeHandle);
    const auto semaphore = static_cast<VkSemaphore>(waitSemaphore);

    VkPresentInfoKHR presentInfo{};
    presentInfo.sType = VK_STRUCTURE_TYPE_PRESENT_INFO_KHR;
    if (semaphore != VK_NULL_HANDLE)
    {
        presentInfo.waitSemaphoreCount = 1;
        presentInfo.pWaitSemaphores = &semaphore;
    }
    presentInfo.swapchainCount = 1;
    presentInfo.pSwapchains = &handle->swapchain;
    presentInfo.pImageIndices = &imageIndex;

    const VkResult result = vkQueuePresentKHR(m_graphicsQueue, &presentInfo);
    switch (result)
    {
    case VK_SUCCESS:
        return SwapchainStatus::Ok;
    case VK_SUBOPTIMAL_KHR:
        return SwapchainStatus::Suboptimal;
    case VK_ERROR_OUT_OF_DATE_KHR:
        return SwapchainStatus::OutOfDate;
    default:
        return std::unexpected(
            RenderError{RenderErrorCode::Unknown, "vkQueuePresentKHR failed (VkResult=" + std::to_string(result) + ")"});
    }
}

void VulkanRenderDevice::Shutdown() noexcept
{
    // Allocator must be torn down before the VkDevice it wraps — it
    // still needs a valid device to free any memory it holds.
    if (m_allocator != VK_NULL_HANDLE)
    {
        vmaDestroyAllocator(m_allocator);
        m_allocator = VK_NULL_HANDLE;
    }

    if (m_device != VK_NULL_HANDLE)
    {
        vkDeviceWaitIdle(m_device);
        vkDestroyDevice(m_device, nullptr);
        m_device = VK_NULL_HANDLE;
        m_graphicsQueue = VK_NULL_HANDLE;
    }

    if (m_debugMessenger != VK_NULL_HANDLE)
    {
        vkDestroyDebugUtilsMessengerEXT(m_instance, m_debugMessenger, nullptr);
        m_debugMessenger = VK_NULL_HANDLE;
    }

    if (m_instance != VK_NULL_HANDLE)
    {
        vkDestroyInstance(m_instance, nullptr);
        m_instance = VK_NULL_HANDLE;
    }
}

} // namespace renderer::backend::vulkan
