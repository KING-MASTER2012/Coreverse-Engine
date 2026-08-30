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

VKAPI_ATTR VkBool32 VKAPI_CALL DebugMessengerCallback(VkDebugUtilsMessageSeverityFlagBitsEXT severity,
                                                       VkDebugUtilsMessageTypeFlagsEXT /*messageType*/,
                                                       const VkDebugUtilsMessengerCallbackDataEXT* callbackData,
                                                       void* /*userData*/)
{
    // Faz 5.1 only needs validation output to land somewhere visible so
    // 5.2+'s leak/misuse checks aren't silent; this gets routed through
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
    // Faz 5.1 just needs *a* working device, not the best one.
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

void VulkanRenderDevice::Shutdown() noexcept
{
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
