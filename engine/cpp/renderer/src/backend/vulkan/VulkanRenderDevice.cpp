#include "backend/vulkan/VulkanRenderDevice.hpp"

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
