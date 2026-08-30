#pragma once

#include <cstdint>
#include <expected>
#include <string>

#include <volk.h>

#include "renderer/RenderDevice.hpp"
#include "renderer/RenderError.hpp"

namespace renderer::backend::vulkan
{

/// Vulkan implementation of RenderDevice. Owns the VkInstance, the
/// (optional) debug messenger, the selected VkPhysicalDevice, the
/// VkDevice, and the graphics queue pulled from it.
///
/// This header lives under src/, not include/renderer/ — it is never
/// part of the public renderer API. Callers only ever see this type
/// through the RenderDevice base pointer returned by
/// CreateRenderDevice(GraphicsAPI::Vulkan); Vulkan headers therefore
/// never leak into consumers of `renderer` (Qt editor included).
class VulkanRenderDevice final : public RenderDevice
{
public:
    VulkanRenderDevice() = default;
    ~VulkanRenderDevice() override;

    /// Performs the actual instance/device bring-up. Split from the
    /// constructor so failure is reported through std::expected instead
    /// of an exception; CreateRenderDevice() only hands the object back
    /// to the caller once this has succeeded. Not part of RenderDevice
    /// — it's backend-specific by nature (every backend's bring-up
    /// takes different inputs), so it isn't forced into the shared
    /// interface; the factory calls it on the concrete type directly.
    [[nodiscard]] std::expected<void, RenderError> Initialize();

    [[nodiscard]] GraphicsAPI GetAPI() const noexcept override
    {
        return GraphicsAPI::Vulkan;
    }

    [[nodiscard]] std::string_view GetDeviceName() const noexcept override
    {
        return m_deviceName;
    }

    void Shutdown() noexcept override;

    // --- Vulkan-specific escape hatch. Only reachable by a caller that
    // already checked GetAPI() == GraphicsAPI::Vulkan and downcast to
    // this concrete type — see RenderDevice.h's class comment. ---
    [[nodiscard]] VkInstance GetVkInstance() const noexcept
    {
        return m_instance;
    }

    [[nodiscard]] VkPhysicalDevice GetVkPhysicalDevice() const noexcept
    {
        return m_physicalDevice;
    }

    [[nodiscard]] VkDevice GetVkDevice() const noexcept
    {
        return m_device;
    }

    [[nodiscard]] VkQueue GetGraphicsQueue() const noexcept
    {
        return m_graphicsQueue;
    }

    [[nodiscard]] uint32_t GetGraphicsQueueFamily() const noexcept
    {
        return m_graphicsQueueFamily;
    }

private:
    [[nodiscard]] std::expected<void, RenderError> CreateInstance();
    [[nodiscard]] std::expected<void, RenderError> SetupDebugMessenger();
    [[nodiscard]] std::expected<void, RenderError> SelectPhysicalDevice();
    [[nodiscard]] std::expected<void, RenderError> CreateLogicalDeviceAndQueues();

    static bool ValidationLayersRequestedAndSupported();

    VkInstance m_instance = VK_NULL_HANDLE;
    VkDebugUtilsMessengerEXT m_debugMessenger = VK_NULL_HANDLE;
    VkPhysicalDevice m_physicalDevice = VK_NULL_HANDLE;
    VkDevice m_device = VK_NULL_HANDLE;
    VkQueue m_graphicsQueue = VK_NULL_HANDLE;
    uint32_t m_graphicsQueueFamily = UINT32_MAX;
    std::string m_deviceName;
    bool m_validationEnabled = false;
};

} // namespace renderer::backend::vulkan
