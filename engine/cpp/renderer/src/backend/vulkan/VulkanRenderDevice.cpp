#include "backend/vulkan/VulkanRenderDevice.hpp"

#include <algorithm>
#include <cstdio>
#include <cstring>
#include <memory>
#include <new>
#include <span>
#include <string>
#include <utility>
#include <vector>

#include "renderer/Diagnostics.hpp"

namespace renderer::backend::vulkan {

namespace {

constexpr const char* kValidationLayerName = "VK_LAYER_KHRONOS_validation";

/// How long FrameSync::BeginFrame() waits for a swapchain image before
/// reporting FrameStatus::NotReady. Long enough to ride out a vsync
/// interval (even a 30 Hz one) without spurious skips, short enough that a
/// GUI thread driving the loop from a timer stays responsive when the
/// window is hidden or occluded and the presentation engine hands no
/// images back.
constexpr std::uint64_t kFrameAcquireTimeoutNs = 100'000'000; // 100 ms

/// Turns a failed Vulkan call into a RenderError. Device loss and memory
/// exhaustion get their own codes (a render loop reacts to them very
/// differently from anything else); everything else keeps `fallback`, and
/// the VkResult is always in the detail string.
RenderError MakeVkError(RenderErrorCode fallback, const char* call, VkResult result)
{
    RenderErrorCode code = fallback;
    switch (result) {
        case VK_ERROR_DEVICE_LOST:
            code = RenderErrorCode::DeviceLost;
            break;
        case VK_ERROR_OUT_OF_HOST_MEMORY:
        case VK_ERROR_OUT_OF_DEVICE_MEMORY:
            code = RenderErrorCode::OutOfMemory;
            break;
        default:
            break;
    }
    return RenderError{code, std::string(call) + " failed (VkResult=" + std::to_string(result) + ")"};
}

/// Pairs the two handles VMA needs to free a buffer. Buffer.hpp only
/// stores a backend-agnostic void*, so this is what that void* actually
/// points at for the Vulkan backend; only CreateBuffer/ReleaseBuffer
/// (this file) ever interpret it.
struct VulkanBufferHandle {
    VkBuffer buffer;
    VmaAllocation allocation;
    /// Created with host access (CpuToGpu / GpuToCpu). A GpuOnly buffer may still be mappable on
    /// an integrated GPU, so Buffer::Write() goes by what was asked for, not by what the driver
    /// happened to hand out - the same code behaves the same on every device.
    bool hostAccess;
};

/// Everything Phase 5.4's swapchain owns beyond the VkSwapchainKHR handle
/// itself. Buffer.hpp/Surface.hpp/Swapchain.hpp only store a backend-
/// agnostic void*, so this is what that void* actually points at for
/// the Vulkan backend; only CreateSwapchain/ReleaseSwapchain/
/// AcquireSwapchainImage/PresentSwapchainImage (this file) ever
/// interpret it.
struct VulkanSwapchainHandle {
    VkSurfaceKHR surface =
        VK_NULL_HANDLE; ///< The surface this swapchain presents to; Recreate() must be given the same one.
    VkSwapchainKHR swapchain = VK_NULL_HANDLE; ///< VK_NULL_HANDLE while empty (after a failed rebuild).
    VkFormat format = VK_FORMAT_UNDEFINED;
    VkExtent2D extent{};
    std::vector<VkImage> images;
    std::vector<VkImageView> imageViews;

    // Optional depth buffer (SwapchainDesc::depthBuffer), one image shared by all frames in
    // flight: BeginRendering()'s barrier chains consecutive frames' depth use in queue order.
    VkFormat depthFormat = VK_FORMAT_UNDEFINED; ///< VK_FORMAT_UNDEFINED = no depth buffer.
    VkImage depthImage = VK_NULL_HANDLE;
    VmaAllocation depthAllocation = VK_NULL_HANDLE;
    VkImageView depthView = VK_NULL_HANDLE;
};

struct VulkanPipelineHandle {
    VkPipeline pipeline = VK_NULL_HANDLE;
    VkPipelineLayout layout = VK_NULL_HANDLE;
    uint32_t pushConstantSize = 0;
};

bool HasStencil(VkFormat format) noexcept
{
    return format == VK_FORMAT_D32_SFLOAT_S8_UINT || format == VK_FORMAT_D24_UNORM_S8_UINT;
}

VkImageAspectFlags DepthAspectOf(VkFormat format) noexcept
{
    VkImageAspectFlags aspect = VK_IMAGE_ASPECT_DEPTH_BIT;
    if (HasStencil(format)) {
        aspect |= VK_IMAGE_ASPECT_STENCIL_BIT;
    }
    return aspect;
}

/// First depth format the device can use as an attachment (the spec guarantees one of the three).
VkFormat ChooseDepthFormat(VkPhysicalDevice physicalDevice) noexcept
{
    constexpr VkFormat candidates[] = {VK_FORMAT_D32_SFLOAT, VK_FORMAT_D32_SFLOAT_S8_UINT, VK_FORMAT_D24_UNORM_S8_UINT};
    for (const VkFormat format : candidates) {
        VkFormatProperties props{};
        vkGetPhysicalDeviceFormatProperties(physicalDevice, format, &props);
        if ((props.optimalTilingFeatures & VK_FORMAT_FEATURE_DEPTH_STENCIL_ATTACHMENT_BIT) != 0) {
            return format;
        }
    }
    return VK_FORMAT_UNDEFINED;
}

/// Everything CreateSwapchain()/RebuildSwapchain() decide about a
/// swapchain before a single Vulkan object is created. Kept separate from
/// the creation step on purpose: a failure while *querying* is known to
/// have left an existing swapchain untouched, whereas a failure while
/// creating a replacement has already retired it (see RebuildSwapchain()).
struct SwapchainConfig {
    VkSurfaceKHR surface = VK_NULL_HANDLE;
    VkSurfaceFormatKHR format{};
    VkPresentModeKHR presentMode = VK_PRESENT_MODE_FIFO_KHR;
    VkSurfaceTransformFlagBitsKHR preTransform = VK_SURFACE_TRANSFORM_IDENTITY_BIT_KHR;
    VkExtent2D extent{};
    uint32_t imageCount = 0;
    VkFormat depthFormat = VK_FORMAT_UNDEFINED; ///< Chosen when SwapchainDesc::depthBuffer is set.
};

std::expected<SwapchainConfig, RenderError> QuerySwapchainConfig(
    VkPhysicalDevice physicalDevice, uint32_t graphicsQueueFamily, VkSurfaceKHR vkSurface, const SwapchainDesc& desc
) noexcept
{
    // Phase 5.1 picked the graphics queue family without checking present
    // support, since there was no surface yet to check it against. Now
    // that there is one, verify it — a queue family that can't present
    // to this surface makes the whole swapchain unusable, so this fails
    // loudly here rather than at some confusing point later.
    VkBool32 presentSupported = VK_FALSE;
    if (const VkResult result =
            vkGetPhysicalDeviceSurfaceSupportKHR(physicalDevice, graphicsQueueFamily, vkSurface, &presentSupported);
        result != VK_SUCCESS || presentSupported == VK_FALSE) {
        return std::unexpected(
            RenderError{
                RenderErrorCode::NoSuitableDevice, "graphics queue family does not support presenting to this surface"
            }
        );
    }

    VkSurfaceCapabilitiesKHR capabilities{};
    if (const VkResult result = vkGetPhysicalDeviceSurfaceCapabilitiesKHR(physicalDevice, vkSurface, &capabilities);
        result != VK_SUCCESS) {
        return std::unexpected(
            RenderError{RenderErrorCode::InitializationFailed, "vkGetPhysicalDeviceSurfaceCapabilitiesKHR failed"}
        );
    }

    uint32_t formatCount = 0;
    vkGetPhysicalDeviceSurfaceFormatsKHR(physicalDevice, vkSurface, &formatCount, nullptr);
    if (formatCount == 0) {
        return std::unexpected(RenderError{RenderErrorCode::NoSuitableDevice, "surface exposes no formats"});
    }
    std::vector<VkSurfaceFormatKHR> formats(formatCount);
    vkGetPhysicalDeviceSurfaceFormatsKHR(physicalDevice, vkSurface, &formatCount, formats.data());

    VkSurfaceFormatKHR chosenFormat = formats[0];
    for (const VkSurfaceFormatKHR& candidate : formats) {
        if (candidate.format == VK_FORMAT_B8G8R8A8_SRGB && candidate.colorSpace == VK_COLOR_SPACE_SRGB_NONLINEAR_KHR) {
            chosenFormat = candidate;
            break;
        }
    }

    uint32_t presentModeCount = 0;
    vkGetPhysicalDeviceSurfacePresentModesKHR(physicalDevice, vkSurface, &presentModeCount, nullptr);
    std::vector<VkPresentModeKHR> presentModes(presentModeCount);
    vkGetPhysicalDeviceSurfacePresentModesKHR(physicalDevice, vkSurface, &presentModeCount, presentModes.data());

    // FIFO is the only mode every Vulkan implementation is required to
    // support (VK_PRESENT_MODE_FIFO_KHR); prefer MAILBOX (low-latency,
    // no tearing) when it's actually available.
    VkPresentModeKHR chosenPresentMode = VK_PRESENT_MODE_FIFO_KHR;
    for (VkPresentModeKHR mode : presentModes) {
        if (mode == VK_PRESENT_MODE_MAILBOX_KHR) {
            chosenPresentMode = mode;
            break;
        }
    }

    VkExtent2D extent{};
    if (capabilities.currentExtent.width != UINT32_MAX) {
        // The surface dictates its own extent (the common case for a
        // real window) — desc.width/height are ignored in favor of it.
        extent = capabilities.currentExtent;
    } else {
        extent.width = std::clamp(desc.width, capabilities.minImageExtent.width, capabilities.maxImageExtent.width);
        extent.height = std::clamp(desc.height, capabilities.minImageExtent.height, capabilities.maxImageExtent.height);
    }

    // A 0x0 extent (a minimized Win32 window reports exactly this) is not
    // a valid swapchain size — vkCreateSwapchainKHR would just fail. It's
    // also not an error worth retiring an existing swapchain over, so
    // report it before anything is created.
    if (extent.width == 0 || extent.height == 0) {
        return std::unexpected(
            RenderError{
                RenderErrorCode::ZeroExtent, "the surface currently has a zero-sized extent (minimized window?)"
            }
        );
    }

    uint32_t imageCount = std::max(desc.preferredImageCount, capabilities.minImageCount);
    if (capabilities.maxImageCount > 0) {
        imageCount = std::min(imageCount, capabilities.maxImageCount);
    }

    SwapchainConfig config;
    config.surface = vkSurface;
    config.format = chosenFormat;
    config.presentMode = chosenPresentMode;
    config.preTransform = capabilities.currentTransform;
    config.extent = extent;
    config.imageCount = imageCount;
    if (desc.depthBuffer) {
        config.depthFormat = ChooseDepthFormat(physicalDevice);
        if (config.depthFormat == VK_FORMAT_UNDEFINED) {
            return std::unexpected(
                RenderError{RenderErrorCode::NoSuitableDevice, "the device supports no depth attachment format"}
            );
        }
    }
    return config;
}

void DestroyDepthBuffer(VkDevice device, VmaAllocator allocator, VulkanSwapchainHandle& handle) noexcept
{
    if (handle.depthView != VK_NULL_HANDLE) {
        vkDestroyImageView(device, handle.depthView, nullptr);
    }
    if (handle.depthImage != VK_NULL_HANDLE) {
        vmaDestroyImage(allocator, handle.depthImage, handle.depthAllocation);
    }
    handle.depthView = VK_NULL_HANDLE;
    handle.depthImage = VK_NULL_HANDLE;
    handle.depthAllocation = VK_NULL_HANDLE;
    handle.depthFormat = VK_FORMAT_UNDEFINED;
}

std::expected<void, RenderError> CreateDepthBuffer(
    VkDevice device, VmaAllocator allocator, VkFormat format, VkExtent2D extent, VulkanSwapchainHandle& handle
) noexcept
{
    VkImageCreateInfo imageInfo{};
    imageInfo.sType = VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO;
    imageInfo.imageType = VK_IMAGE_TYPE_2D;
    imageInfo.format = format;
    imageInfo.extent = {extent.width, extent.height, 1};
    imageInfo.mipLevels = 1;
    imageInfo.arrayLayers = 1;
    imageInfo.samples = VK_SAMPLE_COUNT_1_BIT;
    imageInfo.tiling = VK_IMAGE_TILING_OPTIMAL;
    imageInfo.usage = VK_IMAGE_USAGE_DEPTH_STENCIL_ATTACHMENT_BIT;
    imageInfo.sharingMode = VK_SHARING_MODE_EXCLUSIVE;
    imageInfo.initialLayout = VK_IMAGE_LAYOUT_UNDEFINED;

    VmaAllocationCreateInfo allocInfo{};
    allocInfo.usage = VMA_MEMORY_USAGE_AUTO;
    allocInfo.flags = VMA_ALLOCATION_CREATE_DEDICATED_MEMORY_BIT;

    if (const VkResult result =
            vmaCreateImage(allocator, &imageInfo, &allocInfo, &handle.depthImage, &handle.depthAllocation, nullptr);
        result != VK_SUCCESS) {
        handle.depthImage = VK_NULL_HANDLE;
        return std::unexpected(
            RenderError{
                RenderErrorCode::InitializationFailed,
                "vmaCreateImage failed for the depth buffer (VkResult=" + std::to_string(result) + ")"
            }
        );
    }

    VkImageViewCreateInfo viewInfo{};
    viewInfo.sType = VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO;
    viewInfo.image = handle.depthImage;
    viewInfo.viewType = VK_IMAGE_VIEW_TYPE_2D;
    viewInfo.format = format;
    viewInfo.subresourceRange.aspectMask = DepthAspectOf(format);
    viewInfo.subresourceRange.levelCount = 1;
    viewInfo.subresourceRange.layerCount = 1;
    if (const VkResult result = vkCreateImageView(device, &viewInfo, nullptr, &handle.depthView);
        result != VK_SUCCESS) {
        vmaDestroyImage(allocator, handle.depthImage, handle.depthAllocation);
        handle.depthImage = VK_NULL_HANDLE;
        handle.depthAllocation = VK_NULL_HANDLE;
        handle.depthView = VK_NULL_HANDLE;
        return std::unexpected(
            RenderError{RenderErrorCode::InitializationFailed, "vkCreateImageView failed for the depth buffer"}
        );
    }
    handle.depthFormat = format;
    return {};
}

void DestroySwapchainResources(VkDevice device, VmaAllocator allocator, VulkanSwapchainHandle& handle) noexcept
{
    DestroyDepthBuffer(device, allocator, handle);
    for (VkImageView view : handle.imageViews) {
        vkDestroyImageView(device, view, nullptr);
    }
    // vkDestroySwapchainKHR(VK_NULL_HANDLE) is a valid no-op, which is
    // what makes destroying an already-emptied handle safe.
    vkDestroySwapchainKHR(device, handle.swapchain, nullptr);

    handle.imageViews.clear();
    handle.images.clear();
    handle.swapchain = VK_NULL_HANDLE;
    handle.format = VK_FORMAT_UNDEFINED;
    handle.extent = {};
}

/// Creates the VkSwapchainKHR and its image views. `oldSwapchain` is
/// handed to Vulkan as-is: if it is not VK_NULL_HANDLE it is RETIRED by
/// this call — even when creation fails — so the caller must treat it as
/// gone (destroy it) whether this succeeds or not.
std::expected<VulkanSwapchainHandle, RenderError> CreateSwapchainResources(
    VkDevice device, VmaAllocator allocator, const SwapchainConfig& config, VkSwapchainKHR oldSwapchain
) noexcept
{
    VkSwapchainCreateInfoKHR createInfo{};
    createInfo.sType = VK_STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR;
    createInfo.surface = config.surface;
    createInfo.minImageCount = config.imageCount;
    createInfo.imageFormat = config.format.format;
    createInfo.imageColorSpace = config.format.colorSpace;
    createInfo.imageExtent = config.extent;
    createInfo.imageArrayLayers = 1;
    // COLOR_ATTACHMENT for a normal render-pass-based draw, TRANSFER_DST
    // so Phase 5.5's "clear to a solid color" proof can use either a
    // render pass clear or a plain vkCmdClearColorImage — left open on
    // purpose rather than betting on which one 5.5 picks.
    createInfo.imageUsage = VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT | VK_IMAGE_USAGE_TRANSFER_DST_BIT;
    createInfo.imageSharingMode = VK_SHARING_MODE_EXCLUSIVE;
    createInfo.preTransform = config.preTransform;
    createInfo.compositeAlpha = VK_COMPOSITE_ALPHA_OPAQUE_BIT_KHR;
    createInfo.presentMode = config.presentMode;
    createInfo.clipped = VK_TRUE;
    createInfo.oldSwapchain = oldSwapchain;

    VkSwapchainKHR swapchain = VK_NULL_HANDLE;
    if (const VkResult result = vkCreateSwapchainKHR(device, &createInfo, nullptr, &swapchain); result != VK_SUCCESS) {
        return std::unexpected(
            RenderError{
                RenderErrorCode::InitializationFailed,
                "vkCreateSwapchainKHR failed (VkResult=" + std::to_string(result) + ")"
            }
        );
    }

    uint32_t actualImageCount = 0;
    vkGetSwapchainImagesKHR(device, swapchain, &actualImageCount, nullptr);
    std::vector<VkImage> images(actualImageCount);
    vkGetSwapchainImagesKHR(device, swapchain, &actualImageCount, images.data());

    std::vector<VkImageView> imageViews(actualImageCount, VK_NULL_HANDLE);
    for (uint32_t i = 0; i < actualImageCount; ++i) {
        VkImageViewCreateInfo viewInfo{};
        viewInfo.sType = VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO;
        viewInfo.image = images[i];
        viewInfo.viewType = VK_IMAGE_VIEW_TYPE_2D;
        viewInfo.format = config.format.format;
        viewInfo.components = {
            VK_COMPONENT_SWIZZLE_IDENTITY,
            VK_COMPONENT_SWIZZLE_IDENTITY,
            VK_COMPONENT_SWIZZLE_IDENTITY,
            VK_COMPONENT_SWIZZLE_IDENTITY
        };
        viewInfo.subresourceRange.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
        viewInfo.subresourceRange.baseMipLevel = 0;
        viewInfo.subresourceRange.levelCount = 1;
        viewInfo.subresourceRange.baseArrayLayer = 0;
        viewInfo.subresourceRange.layerCount = 1;

        if (const VkResult result = vkCreateImageView(device, &viewInfo, nullptr, &imageViews[i]);
            result != VK_SUCCESS) {
            // Roll back everything already created before bailing —
            // nothing has been handed to the caller yet to release.
            for (uint32_t created = 0; created < i; ++created) {
                vkDestroyImageView(device, imageViews[created], nullptr);
            }
            vkDestroySwapchainKHR(device, swapchain, nullptr);
            return std::unexpected(
                RenderError{RenderErrorCode::InitializationFailed, "vkCreateImageView failed for a swapchain image"}
            );
        }
    }

    VulkanSwapchainHandle handle;
    handle.surface = config.surface;
    handle.swapchain = swapchain;
    handle.format = config.format.format;
    handle.extent = config.extent;
    handle.images = std::move(images);
    handle.imageViews = std::move(imageViews);

    if (config.depthFormat != VK_FORMAT_UNDEFINED) {
        if (auto depth = CreateDepthBuffer(device, allocator, config.depthFormat, config.extent, handle); !depth) {
            DestroySwapchainResources(device, allocator, handle); // also retires the swapchain just made
            return std::unexpected(depth.error());
        }
    }
    return handle;
}

/// Resources of one in-flight slot: what a frame needs to itself, as
/// opposed to what it needs per swapchain image.
struct FrameSlot {
    /// Signaled by vkAcquireNextImageKHR when the acquired image can be
    /// written; waited on by this slot's submit.
    VkSemaphore imageAvailable = VK_NULL_HANDLE;
    /// Signaled when this slot's submit has finished on the GPU. Created
    /// signaled, so the very first wait on it passes.
    VkFence inFlight = VK_NULL_HANDLE;
    /// Reused every time this slot comes around (the pool allows resetting
    /// individual command buffers); freed with the FrameSync.
    VkCommandBuffer commandBuffer = VK_NULL_HANDLE;
};

/// What FrameSync's void* actually points at for the Vulkan backend; only
/// CreateFrameSync/ReleaseFrameSync/BeginFrameSync/EndFrameSync (this
/// file) ever interpret it.
///
/// Semaphore ownership follows the one rule that holds up under both
/// FIFO and MAILBOX presentation: the semaphore acquire signals belongs to
/// the in-flight *slot* (a slot's previous submit is known to be finished
/// once its fence passed), while the semaphore the submit signals for
/// present to wait on belongs to the swapchain *image* (only the image
/// coming back from the presentation engine proves that the previous
/// present of it consumed the semaphore).
struct VulkanFrameSyncHandle {
    std::vector<FrameSlot> slots;
    std::vector<VkSemaphore> renderFinished; ///< One per swapchain image.
    /// One per swapchain image: the fence of the slot whose frame last
    /// rendered into that image (not owned). Needed when there are more
    /// slots than images, where a slot's own fence does not cover the
    /// image it is handed.
    std::vector<VkFence> imageInFlight;
    /// The VkSwapchainKHR the per-image state above was sized for; a
    /// different one (or a different image count) means the swapchain was
    /// rebuilt since.
    VkSwapchainKHR trackedSwapchain = VK_NULL_HANDLE;
    std::uint32_t currentSlot = 0;
    bool frameOpen = false;         ///< BeginFrame() returned Ready and EndFrame() has not run yet.
    bool acquireSuboptimal = false; ///< The open frame's acquire reported VK_SUBOPTIMAL_KHR.
    bool failed = false;            ///< A failed EndFrame() (or half-done BeginFrame()) left the state unusable.
};

/// Destroys everything a VulkanFrameSyncHandle owns. Safe on a partially
/// built or already emptied handle (vkDestroy*(VK_NULL_HANDLE) are valid
/// no-ops). The caller guarantees the device is idle.
void DestroyFrameSyncResources(VkDevice device, VkCommandPool commandPool, VulkanFrameSyncHandle& handle) noexcept
{
    std::vector<VkCommandBuffer> commandBuffers;
    for (FrameSlot& slot : handle.slots) {
        vkDestroySemaphore(device, slot.imageAvailable, nullptr);
        vkDestroyFence(device, slot.inFlight, nullptr);
        if (slot.commandBuffer != VK_NULL_HANDLE) {
            commandBuffers.push_back(slot.commandBuffer);
        }
    }
    if (!commandBuffers.empty()) {
        vkFreeCommandBuffers(
            device, commandPool, static_cast<std::uint32_t>(commandBuffers.size()), commandBuffers.data()
        );
    }
    for (VkSemaphore semaphore : handle.renderFinished) {
        vkDestroySemaphore(device, semaphore, nullptr);
    }

    handle.slots.clear();
    handle.renderFinished.clear();
    handle.imageInFlight.clear();
    handle.trackedSwapchain = VK_NULL_HANDLE;
}

/// (Re)sizes the per-swapchain-image state to `swapchain` and remembers
/// which swapchain that was. The caller guarantees the device is idle:
/// nothing may still be using the semaphores this may replace.
std::expected<void, RenderError>
SyncPerImageState(VkDevice device, VulkanFrameSyncHandle& handle, const VulkanSwapchainHandle& swapchain) noexcept
{
    const std::size_t imageCount = swapchain.images.size();

    if (handle.renderFinished.size() != imageCount) {
        for (VkSemaphore semaphore : handle.renderFinished) {
            vkDestroySemaphore(device, semaphore, nullptr);
        }
        handle.renderFinished.clear();

        VkSemaphoreCreateInfo semaphoreInfo{};
        semaphoreInfo.sType = VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO;
        for (std::size_t i = 0; i < imageCount; ++i) {
            VkSemaphore semaphore = VK_NULL_HANDLE;
            if (const VkResult result = vkCreateSemaphore(device, &semaphoreInfo, nullptr, &semaphore);
                result != VK_SUCCESS) {
                for (VkSemaphore created : handle.renderFinished) {
                    vkDestroySemaphore(device, created, nullptr);
                }
                handle.renderFinished.clear();
                return std::unexpected(MakeVkError(RenderErrorCode::InitializationFailed, "vkCreateSemaphore", result));
            }
            handle.renderFinished.push_back(semaphore);
        }
    }

    // Whatever fences the old images were tied to no longer say anything
    // about the new ones.
    handle.imageInFlight.assign(imageCount, VK_NULL_HANDLE);
    handle.trackedSwapchain = swapchain.swapchain;
    return {};
}

/// Maps a Vulkan message severity bit to RenderDevice::LogSeverity for
/// SetLogCallback() — see that method's doc comment for why this is the
/// renderer's own small enum rather than passing the VkDebugUtils* bits
/// straight through.
RenderDevice::LogSeverity ToLogSeverity(VkDebugUtilsMessageSeverityFlagBitsEXT severity)
{
    if (severity >= VK_DEBUG_UTILS_MESSAGE_SEVERITY_ERROR_BIT_EXT) {
        return RenderDevice::LogSeverity::Error;
    }
    if (severity >= VK_DEBUG_UTILS_MESSAGE_SEVERITY_WARNING_BIT_EXT) {
        return RenderDevice::LogSeverity::Warning;
    }
    if (severity >= VK_DEBUG_UTILS_MESSAGE_SEVERITY_INFO_BIT_EXT) {
        return RenderDevice::LogSeverity::Info;
    }
    return RenderDevice::LogSeverity::Verbose;
}

VKAPI_ATTR VkBool32 VKAPI_CALL DebugMessengerCallback(
    VkDebugUtilsMessageSeverityFlagBitsEXT severity,
    VkDebugUtilsMessageTypeFlagsEXT /*messageType*/,
    const VkDebugUtilsMessengerCallbackDataEXT* callbackData,
    void* userData
)
{
    // Validation output lands on stderr so leak/misuse checks aren't
    // silent, regardless of whether a LogCallback is also set (stderr is
    // always there even before/without a logger; a LogCallback is
    // additional, not a replacement). Errors are additionally counted
    // (renderer/Diagnostics.hpp) so a test can fail on them.
    if (severity >= VK_DEBUG_UTILS_MESSAGE_SEVERITY_WARNING_BIT_EXT) {
        std::fprintf(stderr, "[vulkan] %s\n", callbackData->pMessage);
    }
    if (severity >= VK_DEBUG_UTILS_MESSAGE_SEVERITY_ERROR_BIT_EXT) {
        detail::NoteValidationError();
    }

    // pUserData is set to the owning VulkanRenderDevice in
    // SetupDebugMessenger() below; forward to whatever LogCallback (if
    // any) SetLogCallback() last installed on it.
    if (userData != nullptr) {
        auto* device = static_cast<VulkanRenderDevice*>(userData);
        device->NotifyLogCallback(ToLogSeverity(severity), callbackData->pMessage);
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

    for (const auto& layer : layers) {
        if (std::strcmp(layer.layerName, kValidationLayerName) == 0) {
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
    if (const VkResult volkResult = volkInitialize(); volkResult != VK_SUCCESS) {
        return std::unexpected(
            RenderError{
                RenderErrorCode::InitializationFailed,
                "volkInitialize failed (VkResult=" + std::to_string(volkResult) + ")"
            }
        );
    }

    if (auto result = CreateInstance(); !result) {
        return result;
    }

    if (m_validationEnabled) {
        if (auto result = SetupDebugMessenger(); !result) {
            return result;
        }
    }

    if (auto result = SelectPhysicalDevice(); !result) {
        return result;
    }

    if (auto result = CreateLogicalDeviceAndQueues(); !result) {
        return result;
    }

    if (auto result = CreateAllocator(); !result) {
        return result;
    }

    if (auto result = CreateCommandPool(); !result) {
        return result;
    }

    return {};
}

std::expected<void, RenderError> VulkanRenderDevice::CreateInstance()
{
    m_validationEnabled = ValidationLayersRequestedAndSupported();

    VkApplicationInfo appInfo{};
    appInfo.sType = VK_STRUCTURE_TYPE_APPLICATION_INFO;
    appInfo.pApplicationName = "Coreverse";
    appInfo.applicationVersion = VK_MAKE_VERSION(0, 1, 0);
    appInfo.pEngineName = "Coreverse Engine";
    appInfo.engineVersion = VK_MAKE_VERSION(0, 1, 0);
    appInfo.apiVersion = VK_API_VERSION_1_3;

    std::vector<const char*> extensions;
    // VK_KHR_surface plus whichever platform-specific WSI extension(s)
    // this build was compiled for are required unconditionally —
    // surface creation (Phase 5.3) needs them regardless of whether
    // validation is enabled. Win32/Metal are mutually exclusive with
    // everything else, matching NativeWindowHandle's #if ladder in
    // Surface.hpp. Linux is the one platform where more than one WSI
    // extension can be requested in the same build: Xlib and Wayland
    // are independent CMake options (RENDERER_ENABLE_XLIB /
    // RENDERER_ENABLE_WAYLAND, see renderer/CMakeLists.txt) and either,
    // both, or (checked at configure time) neither-is-rejected can be
    // compiled in — CreateSurface() below picks whichever one the
    // caller's NativeWindowHandle actually filled in at runtime.
    extensions.push_back(VK_KHR_SURFACE_EXTENSION_NAME);
#if defined(_WIN32)
    extensions.push_back(VK_KHR_WIN32_SURFACE_EXTENSION_NAME);
#elif defined(__APPLE__)
    extensions.push_back(VK_EXT_METAL_SURFACE_EXTENSION_NAME);
#elif defined(__linux__)
    #if defined(VK_USE_PLATFORM_XLIB_KHR)
    extensions.push_back(VK_KHR_XLIB_SURFACE_EXTENSION_NAME);
    #endif
    #if defined(VK_USE_PLATFORM_WAYLAND_KHR)
    extensions.push_back(VK_KHR_WAYLAND_SURFACE_EXTENSION_NAME);
    #endif
#endif
    if (m_validationEnabled) {
        extensions.push_back(VK_EXT_DEBUG_UTILS_EXTENSION_NAME);
    }

    std::vector<const char*> layers;
    if (m_validationEnabled) {
        layers.push_back(kValidationLayerName);
    }

    VkInstanceCreateInfo createInfo{};
    createInfo.sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO;
    createInfo.pApplicationInfo = &appInfo;
    createInfo.enabledExtensionCount = static_cast<uint32_t>(extensions.size());
    createInfo.ppEnabledExtensionNames = extensions.data();
    createInfo.enabledLayerCount = static_cast<uint32_t>(layers.size());
    createInfo.ppEnabledLayerNames = layers.data();

    if (const VkResult result = vkCreateInstance(&createInfo, nullptr, &m_instance); result != VK_SUCCESS) {
        return std::unexpected(
            RenderError{
                RenderErrorCode::InitializationFailed,
                "vkCreateInstance failed (VkResult=" + std::to_string(result) + ")"
            }
        );
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
    createInfo.pUserData = this; // see DebugMessengerCallback's use of userData above

    // volk loads extension entry points (incl. vkCreateDebugUtilsMessengerEXT)
    // automatically once volkLoadInstance() ran with the extension enabled —
    // no manual vkGetInstanceProcAddr lookup needed.
    if (const VkResult result = vkCreateDebugUtilsMessengerEXT(m_instance, &createInfo, nullptr, &m_debugMessenger);
        result != VK_SUCCESS) {
        return std::unexpected(
            RenderError{RenderErrorCode::InitializationFailed, "vkCreateDebugUtilsMessengerEXT failed"}
        );
    }
    return {};
}

std::expected<void, RenderError> VulkanRenderDevice::SelectPhysicalDevice()
{
    uint32_t deviceCount = 0;
    vkEnumeratePhysicalDevices(m_instance, &deviceCount, nullptr);
    if (deviceCount == 0) {
        return std::unexpected(
            RenderError{RenderErrorCode::NoSuitableDevice, "No Vulkan-capable physical devices found"}
        );
    }

    std::vector<VkPhysicalDevice> devices(deviceCount);
    vkEnumeratePhysicalDevices(m_instance, &deviceCount, devices.data());

    // Prefer a discrete GPU; fall back to whatever enumerates first
    // (integrated GPU, software rasterizer, ...) rather than failing —
    // Phase 5 just needs *a* working device, not the best one.
    VkPhysicalDevice fallback = VK_NULL_HANDLE;
    for (VkPhysicalDevice candidate : devices) {
        VkPhysicalDeviceProperties props{};
        vkGetPhysicalDeviceProperties(candidate, &props);

        if (fallback == VK_NULL_HANDLE) {
            fallback = candidate;
        }

        if (props.deviceType == VK_PHYSICAL_DEVICE_TYPE_DISCRETE_GPU) {
            m_physicalDevice = candidate;
            m_deviceName = props.deviceName;
            break;
        }
    }

    if (m_physicalDevice == VK_NULL_HANDLE) {
        m_physicalDevice = fallback;
        VkPhysicalDeviceProperties props{};
        vkGetPhysicalDeviceProperties(m_physicalDevice, &props);
        m_deviceName = props.deviceName;
    }

    uint32_t familyCount = 0;
    vkGetPhysicalDeviceQueueFamilyProperties(m_physicalDevice, &familyCount, nullptr);
    std::vector<VkQueueFamilyProperties> families(familyCount);
    vkGetPhysicalDeviceQueueFamilyProperties(m_physicalDevice, &familyCount, families.data());

    for (uint32_t i = 0; i < familyCount; ++i) {
        if (families[i].queueFlags & VK_QUEUE_GRAPHICS_BIT) {
            m_graphicsQueueFamily = i;
            break;
        }
    }

    if (m_graphicsQueueFamily == UINT32_MAX) {
        return std::unexpected(
            RenderError{RenderErrorCode::NoSuitableDevice, "Selected device has no graphics-capable queue family"}
        );
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
    // Phase 5.4's swapchain calls are never loaded (volk leaves them null,
    // which crashes on the first call rather than failing cleanly).
    static constexpr const char* kDeviceExtensions[] = {VK_KHR_SWAPCHAIN_EXTENSION_NAME};
    deviceCreateInfo.enabledExtensionCount = 1;
    deviceCreateInfo.ppEnabledExtensionNames = kDeviceExtensions;

    // Vulkan 1.3 dynamic rendering.md (vkCmdBeginRendering) backs the graphics pass. Enable it when
    // the device offers it; without it the device still works for ClearColor() and the graphics
    // pass calls report RenderErrorCode::Unsupported (e.g. older MoltenVK).
    VkPhysicalDeviceProperties deviceProps{};
    vkGetPhysicalDeviceProperties(m_physicalDevice, &deviceProps);
    VkPhysicalDeviceVulkan13Features supported13{};
    supported13.sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_VULKAN_1_3_FEATURES;
    if (deviceProps.apiVersion >= VK_API_VERSION_1_3) {
        VkPhysicalDeviceFeatures2 supported{};
        supported.sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_FEATURES_2;
        supported.pNext = &supported13;
        vkGetPhysicalDeviceFeatures2(m_physicalDevice, &supported);
    }
    m_dynamicRendering = deviceProps.apiVersion >= VK_API_VERSION_1_3 && supported13.dynamicRendering == VK_TRUE;
    VkPhysicalDeviceVulkan13Features enabled13{};
    enabled13.sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_VULKAN_1_3_FEATURES;
    enabled13.dynamicRendering = VK_TRUE;
    if (m_dynamicRendering) {
        deviceCreateInfo.pNext = &enabled13;
    }

    // Device-layer validation is deprecated (instance-layer validation
    // has covered everything since Vulkan 1.1) but harmless to set for
    // older loaders that might still be present — cheap to keep.
    std::vector<const char*> layers;
    if (m_validationEnabled) {
        layers.push_back(kValidationLayerName);
    }
    deviceCreateInfo.enabledLayerCount = static_cast<uint32_t>(layers.size());
    deviceCreateInfo.ppEnabledLayerNames = layers.data();

    if (const VkResult result = vkCreateDevice(m_physicalDevice, &deviceCreateInfo, nullptr, &m_device);
        result != VK_SUCCESS) {
        return std::unexpected(
            RenderError{
                RenderErrorCode::InitializationFailed, "vkCreateDevice failed (VkResult=" + std::to_string(result) + ")"
            }
        );
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

    if (const VkResult result = vmaCreateAllocator(&allocatorInfo, &m_allocator); result != VK_SUCCESS) {
        return std::unexpected(
            RenderError{
                RenderErrorCode::InitializationFailed,
                "vmaCreateAllocator failed (VkResult=" + std::to_string(result) + ")"
            }
        );
    }
    return {};
}

std::expected<void, RenderError> VulkanRenderDevice::CreateCommandPool()
{
    VkCommandPoolCreateInfo poolInfo{};
    poolInfo.sType = VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO;
    // RESET_COMMAND_BUFFER: FrameSync re-records the same command buffers
    // across frames (vkResetCommandBuffer, then Begin) instead of
    // allocating fresh ones every time.
    poolInfo.flags = VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT;
    poolInfo.queueFamilyIndex = m_graphicsQueueFamily;

    if (const VkResult result = vkCreateCommandPool(m_device, &poolInfo, nullptr, &m_commandPool);
        result != VK_SUCCESS) {
        return std::unexpected(
            RenderError{
                RenderErrorCode::InitializationFailed,
                "vkCreateCommandPool failed (VkResult=" + std::to_string(result) + ")"
            }
        );
    }
    return {};
}

std::expected<Buffer, RenderError> VulkanRenderDevice::CreateBuffer(const BufferDesc& desc) noexcept
{
    if (desc.size == 0) {
        return std::unexpected(RenderError{RenderErrorCode::InitializationFailed, "Buffer size must be non-zero"});
    }

    VkBufferUsageFlags usageFlags = 0;
    if (HasFlag(desc.usage, BufferUsage::VertexBuffer)) {
        usageFlags |= VK_BUFFER_USAGE_VERTEX_BUFFER_BIT;
    }
    if (HasFlag(desc.usage, BufferUsage::IndexBuffer)) {
        usageFlags |= VK_BUFFER_USAGE_INDEX_BUFFER_BIT;
    }
    if (HasFlag(desc.usage, BufferUsage::UniformBuffer)) {
        usageFlags |= VK_BUFFER_USAGE_UNIFORM_BUFFER_BIT;
    }
    if (HasFlag(desc.usage, BufferUsage::TransferSrc)) {
        usageFlags |= VK_BUFFER_USAGE_TRANSFER_SRC_BIT;
    }
    if (HasFlag(desc.usage, BufferUsage::TransferDst)) {
        usageFlags |= VK_BUFFER_USAGE_TRANSFER_DST_BIT;
    }

    VkBufferCreateInfo bufferInfo{};
    bufferInfo.sType = VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO;
    bufferInfo.size = static_cast<VkDeviceSize>(desc.size);
    bufferInfo.usage = usageFlags;
    bufferInfo.sharingMode = VK_SHARING_MODE_EXCLUSIVE;

    VmaAllocationCreateInfo allocInfo{};
    switch (desc.memoryUsage) {
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
        result != VK_SUCCESS) {
        return std::unexpected(
            RenderError{
                RenderErrorCode::OutOfMemory, "vmaCreateBuffer failed (VkResult=" + std::to_string(result) + ")"
            }
        );
    }

    // Buffer only stores a backend-agnostic void*; this heap-allocated
    // pair is what the Vulkan backend puts behind it so ReleaseBuffer()
    // can hand both handles back to VMA later.
    auto* handle = new VulkanBufferHandle{buffer, allocation, desc.memoryUsage != BufferMemoryUsage::GpuOnly};
    return RenderDevice::MakeBuffer(this, handle, desc.size);
}

void VulkanRenderDevice::ReleaseBuffer(void* nativeHandle) noexcept
{
    if (nativeHandle == nullptr) {
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
    if (handle.hwnd != nullptr) {
        VkWin32SurfaceCreateInfoKHR createInfo{};
        createInfo.sType = VK_STRUCTURE_TYPE_WIN32_SURFACE_CREATE_INFO_KHR;
        createInfo.hinstance = static_cast<HINSTANCE>(handle.hinstance);
        createInfo.hwnd = static_cast<HWND>(handle.hwnd);
        result = vkCreateWin32SurfaceKHR(m_instance, &createInfo, nullptr, &surface);
    }
#elif defined(__APPLE__)
    if (handle.metalLayer != nullptr) {
        VkMetalSurfaceCreateInfoEXT createInfo{};
        createInfo.sType = VK_STRUCTURE_TYPE_METAL_SURFACE_CREATE_INFO_EXT;
        createInfo.pLayer = static_cast<const CAMetalLayer*>(handle.metalLayer);
        result = vkCreateMetalSurfaceEXT(m_instance, &createInfo, nullptr, &surface);
    }
#elif defined(__linux__)
    // Both backends may be compiled in at once (see renderer/CMakeLists.txt),
    // so pick whichever one the caller actually populated — Xlib first
    // for parity with the previous X11-only behavior, falling through
    // to Wayland only if Xlib didn't handle it (either because it isn't
    // compiled in, or because the caller left xlibDisplay null).
    bool handled = false;
    #if defined(VK_USE_PLATFORM_XLIB_KHR)
    if (!handled && handle.xlibDisplay != nullptr) {
        VkXlibSurfaceCreateInfoKHR createInfo{};
        createInfo.sType = VK_STRUCTURE_TYPE_XLIB_SURFACE_CREATE_INFO_KHR;
        createInfo.dpy = static_cast<Display*>(handle.xlibDisplay);
        createInfo.window = static_cast<Window>(handle.xlibWindow);
        result = vkCreateXlibSurfaceKHR(m_instance, &createInfo, nullptr, &surface);
        handled = true;
    }
    #endif
    #if defined(VK_USE_PLATFORM_WAYLAND_KHR)
    // struct wl_display/wl_surface are forward-declared by
    // vulkan_wayland.h itself (see the CMakeLists.txt comment above) —
    // we never dereference them, only hand the pointers Qt/the caller
    // gave us straight to the Vulkan WSI extension, so no
    // wayland-client header is needed here either.
    if (!handled && handle.waylandDisplay != nullptr) {
        VkWaylandSurfaceCreateInfoKHR createInfo{};
        createInfo.sType = VK_STRUCTURE_TYPE_WAYLAND_SURFACE_CREATE_INFO_KHR;
        createInfo.display = static_cast<struct wl_display*>(handle.waylandDisplay);
        createInfo.surface = static_cast<struct wl_surface*>(handle.waylandSurface);
        result = vkCreateWaylandSurfaceKHR(m_instance, &createInfo, nullptr, &surface);
        handled = true;
    }
    #endif
#endif

    if (result != VK_SUCCESS) {
        return std::unexpected(
            RenderError{
                RenderErrorCode::InitializationFailed,
                "surface creation failed (VkResult=" + std::to_string(result) + ")"
            }
        );
    }

    return RenderDevice::MakeSurface(this, surface);
}

void VulkanRenderDevice::ReleaseSurface(void* nativeHandle) noexcept
{
    if (nativeHandle == nullptr) {
        return;
    }
    vkDestroySurfaceKHR(m_instance, static_cast<VkSurfaceKHR>(nativeHandle), nullptr);
}

std::expected<Swapchain, RenderError>
VulkanRenderDevice::CreateSwapchain(const Surface& surface, const SwapchainDesc& desc) noexcept
{
    if (!surface.IsValid()) {
        return std::unexpected(
            RenderError{RenderErrorCode::InitializationFailed, "CreateSwapchain called with an invalid Surface"}
        );
    }
    const auto vkSurface = static_cast<VkSurfaceKHR>(surface.GetNativeHandle());

    auto config = QuerySwapchainConfig(m_physicalDevice, m_graphicsQueueFamily, vkSurface, desc);
    if (!config) {
        return std::unexpected(std::move(config.error()));
    }

    auto resources = CreateSwapchainResources(m_device, m_allocator, *config, VK_NULL_HANDLE);
    if (!resources) {
        return std::unexpected(std::move(resources.error()));
    }

    const auto imageCount = static_cast<std::uint32_t>(resources->images.size());
    const Extent2D extent{resources->extent.width, resources->extent.height};
    auto* handle = new VulkanSwapchainHandle(std::move(*resources));
    return RenderDevice::MakeSwapchain(this, handle, imageCount, extent);
}

std::expected<void, RenderError> VulkanRenderDevice::RebuildSwapchain(
    void* nativeHandle, const Surface& surface, const SwapchainDesc& desc, SwapchainInfo& info
) noexcept
{
    if (!surface.IsValid()) {
        return std::unexpected(
            RenderError{RenderErrorCode::InitializationFailed, "Swapchain::Recreate called with an invalid Surface"}
        );
    }
    auto* handle = static_cast<VulkanSwapchainHandle*>(nativeHandle);
    const auto vkSurface = static_cast<VkSurfaceKHR>(surface.GetNativeHandle());

    if (vkSurface != handle->surface) {
        return std::unexpected(
            RenderError{
                RenderErrorCode::InitializationFailed,
                "Swapchain::Recreate called with a different Surface than the swapchain was created from"
            }
        );
    }

    // Everything that can be checked without creating anything happens
    // first, so ZeroExtent & co. leave the existing swapchain untouched.
    auto config = QuerySwapchainConfig(m_physicalDevice, m_graphicsQueueFamily, vkSurface, desc);
    if (!config) {
        return std::unexpected(std::move(config.error()));
    }

    // Nothing tracks frames in flight yet (Phase 6.4), so the only way to
    // know the old images are no longer in use is to wait for the device.
    WaitIdle();

    // Handing the old swapchain over lets the driver recycle its
    // resources for the new one. Either way it is retired by this call.
    auto resources = CreateSwapchainResources(m_device, m_allocator, *config, handle->swapchain);
    DestroySwapchainResources(m_device, m_allocator, *handle);

    if (!resources) {
        // The old swapchain is gone and there is no replacement: leave
        // an empty-but-valid object that a later Recreate() can revive
        // (Acquire()/Present() report OutOfDate meanwhile).
        info = SwapchainInfo{};
        return std::unexpected(std::move(resources.error()));
    }

    *handle = std::move(*resources);
    info.imageCount = static_cast<std::uint32_t>(handle->images.size());
    info.extent = Extent2D{handle->extent.width, handle->extent.height};
    return {};
}

void VulkanRenderDevice::ReleaseSwapchain(void* nativeHandle) noexcept
{
    if (nativeHandle == nullptr) {
        return;
    }
    auto* handle = static_cast<VulkanSwapchainHandle*>(nativeHandle);
    DestroySwapchainResources(m_device, m_allocator, *handle);
    delete handle;
}

std::expected<AcquireResult, RenderError>
VulkanRenderDevice::AcquireSwapchainImage(void* nativeHandle, void* signalSemaphore, std::uint64_t timeoutNs) noexcept
{
    if (m_deviceLostSimulated.load(std::memory_order_relaxed)) {
        return std::unexpected(RenderError{RenderErrorCode::DeviceLost, "simulated device loss (testing)"});
    }
    auto* handle = static_cast<VulkanSwapchainHandle*>(nativeHandle);
    const auto semaphore = static_cast<VkSemaphore>(signalSemaphore);

    // Emptied by a failed Swapchain::Recreate(): there is nothing to
    // acquire from until a later Recreate() succeeds.
    if (handle->swapchain == VK_NULL_HANDLE) {
        return AcquireResult{0, SwapchainStatus::OutOfDate};
    }

    VkFence fence = VK_NULL_HANDLE;
    if (semaphore == VK_NULL_HANDLE) {
        // No semaphore for the GPU to signal into — fall back to a
        // fence and wait on it ourselves, so this call stays usable
        // stand-alone (the single-shot tests still call it exactly this
        // way; a render loop goes through FrameSync instead). Vulkan
        // requires at least one of {semaphore, fence} to be valid.
        VkFenceCreateInfo fenceInfo{};
        fenceInfo.sType = VK_STRUCTURE_TYPE_FENCE_CREATE_INFO;
        if (const VkResult result = vkCreateFence(m_device, &fenceInfo, nullptr, &fence); result != VK_SUCCESS) {
            return std::unexpected(MakeVkError(RenderErrorCode::InitializationFailed, "vkCreateFence", result));
        }
    }

    uint32_t imageIndex = 0;
    const VkResult acquireResult =
        vkAcquireNextImageKHR(m_device, handle->swapchain, timeoutNs, semaphore, fence, &imageIndex);

    if (fence != VK_NULL_HANDLE) {
        if (acquireResult == VK_SUCCESS || acquireResult == VK_SUBOPTIMAL_KHR) {
            vkWaitForFences(m_device, 1, &fence, VK_TRUE, UINT64_MAX);
        }
        vkDestroyFence(m_device, fence, nullptr);
    }

    switch (acquireResult) {
        case VK_SUCCESS:
            return AcquireResult{imageIndex, SwapchainStatus::Ok};
        case VK_SUBOPTIMAL_KHR:
            return AcquireResult{imageIndex, SwapchainStatus::Suboptimal};
        case VK_ERROR_OUT_OF_DATE_KHR:
            return AcquireResult{0, SwapchainStatus::OutOfDate};
        case VK_TIMEOUT:
        case VK_NOT_READY:
            return AcquireResult{0, SwapchainStatus::NotReady};
        default:
            return std::unexpected(MakeVkError(RenderErrorCode::Unknown, "vkAcquireNextImageKHR", acquireResult));
    }
}

std::expected<SwapchainStatus, RenderError>
VulkanRenderDevice::PresentSwapchainImage(void* nativeHandle, std::uint32_t imageIndex, void* waitSemaphore) noexcept
{
    if (m_deviceLostSimulated.load(std::memory_order_relaxed)) {
        return std::unexpected(RenderError{RenderErrorCode::DeviceLost, "simulated device loss (testing)"});
    }
    auto* handle = static_cast<VulkanSwapchainHandle*>(nativeHandle);
    const auto semaphore = static_cast<VkSemaphore>(waitSemaphore);

    if (handle->swapchain == VK_NULL_HANDLE) {
        return SwapchainStatus::OutOfDate; // emptied by a failed Recreate(), see Acquire
    }

    VkPresentInfoKHR presentInfo{};
    presentInfo.sType = VK_STRUCTURE_TYPE_PRESENT_INFO_KHR;
    if (semaphore != VK_NULL_HANDLE) {
        presentInfo.waitSemaphoreCount = 1;
        presentInfo.pWaitSemaphores = &semaphore;
    }
    presentInfo.swapchainCount = 1;
    presentInfo.pSwapchains = &handle->swapchain;
    presentInfo.pImageIndices = &imageIndex;

    const VkResult result = vkQueuePresentKHR(m_graphicsQueue, &presentInfo);
    switch (result) {
        case VK_SUCCESS:
            return SwapchainStatus::Ok;
        case VK_SUBOPTIMAL_KHR:
            return SwapchainStatus::Suboptimal;
        case VK_ERROR_OUT_OF_DATE_KHR:
            return SwapchainStatus::OutOfDate;
        default:
            return std::unexpected(MakeVkError(RenderErrorCode::Unknown, "vkQueuePresentKHR", result));
    }
}

void* VulkanRenderDevice::GetSwapchainImageHandle(void* swapchainNativeHandle, std::uint32_t index) noexcept
{
    auto* handle = static_cast<VulkanSwapchainHandle*>(swapchainNativeHandle);
    if (index >= handle->images.size()) {
        return nullptr;
    }
    // VkImage is itself a non-dispatchable handle (a pointer on 64-bit
    // builds), so it converts to void* directly — the caller gets it
    // back the same way Buffer/Surface/Swapchain's own native handles
    // work.
    return handle->images[index];
}

std::expected<CommandBuffer, RenderError> VulkanRenderDevice::AcquireCommandBuffer() noexcept
{
    VkCommandBufferAllocateInfo allocInfo{};
    allocInfo.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO;
    allocInfo.commandPool = m_commandPool;
    allocInfo.level = VK_COMMAND_BUFFER_LEVEL_PRIMARY;
    allocInfo.commandBufferCount = 1;

    VkCommandBuffer commandBuffer = VK_NULL_HANDLE;
    if (const VkResult result = vkAllocateCommandBuffers(m_device, &allocInfo, &commandBuffer); result != VK_SUCCESS) {
        return std::unexpected(
            RenderError{
                RenderErrorCode::InitializationFailed,
                "vkAllocateCommandBuffers failed (VkResult=" + std::to_string(result) + ")"
            }
        );
    }

    // Borrowed, not owned (CommandBuffer.hpp) — the pool it came from
    // frees it implicitly at Shutdown(); there is deliberately no
    // ReleaseCommandBuffer path to call.
    return RenderDevice::MakeCommandBuffer(this, commandBuffer);
}

std::expected<void, RenderError> VulkanRenderDevice::BeginCommandBuffer(void* commandBufferHandle) noexcept
{
    const auto commandBuffer = static_cast<VkCommandBuffer>(commandBufferHandle);

    VkCommandBufferBeginInfo beginInfo{};
    beginInfo.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO;
    beginInfo.flags = VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT;

    if (const VkResult result = vkBeginCommandBuffer(commandBuffer, &beginInfo); result != VK_SUCCESS) {
        return std::unexpected(MakeVkError(RenderErrorCode::Unknown, "vkBeginCommandBuffer", result));
    }
    return {};
}

std::expected<void, RenderError> VulkanRenderDevice::EndCommandBuffer(void* commandBufferHandle) noexcept
{
    const auto commandBuffer = static_cast<VkCommandBuffer>(commandBufferHandle);

    if (const VkResult result = vkEndCommandBuffer(commandBuffer); result != VK_SUCCESS) {
        return std::unexpected(MakeVkError(RenderErrorCode::Unknown, "vkEndCommandBuffer", result));
    }
    return {};
}

std::expected<void, RenderError>
VulkanRenderDevice::RecordClearColor(void* commandBufferHandle, void* imageHandle, const ClearColor& color) noexcept
{
    const auto commandBuffer = static_cast<VkCommandBuffer>(commandBufferHandle);
    const auto image = static_cast<VkImage>(imageHandle);

    // A freshly acquired swapchain image starts life in
    // VK_IMAGE_LAYOUT_UNDEFINED; vkCmdClearColorImage needs it in
    // TRANSFER_DST_OPTIMAL, and presenting afterward needs
    // PRESENT_SRC_KHR — this pair of barriers is the entire "render
    // pass" this milestone needs, deliberately skipping a real
    // VkRenderPass/VkFramebuffer for a single clear.
    VkImageSubresourceRange range{};
    range.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
    range.baseMipLevel = 0;
    range.levelCount = 1;
    range.baseArrayLayer = 0;
    range.layerCount = 1;

    VkImageMemoryBarrier toTransferDst{};
    toTransferDst.sType = VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER;
    toTransferDst.oldLayout = VK_IMAGE_LAYOUT_UNDEFINED;
    toTransferDst.newLayout = VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL;
    toTransferDst.srcQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED;
    toTransferDst.dstQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED;
    toTransferDst.image = image;
    toTransferDst.subresourceRange = range;
    toTransferDst.srcAccessMask = 0;
    toTransferDst.dstAccessMask = VK_ACCESS_TRANSFER_WRITE_BIT;

    vkCmdPipelineBarrier(
        commandBuffer,
        VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT,
        VK_PIPELINE_STAGE_TRANSFER_BIT,
        0,
        0,
        nullptr,
        0,
        nullptr,
        1,
        &toTransferDst
    );

    VkClearColorValue clearValue{};
    clearValue.float32[0] = color.r;
    clearValue.float32[1] = color.g;
    clearValue.float32[2] = color.b;
    clearValue.float32[3] = color.a;

    vkCmdClearColorImage(commandBuffer, image, VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL, &clearValue, 1, &range);

    VkImageMemoryBarrier toPresent{};
    toPresent.sType = VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER;
    toPresent.oldLayout = VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL;
    toPresent.newLayout = VK_IMAGE_LAYOUT_PRESENT_SRC_KHR;
    toPresent.srcQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED;
    toPresent.dstQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED;
    toPresent.image = image;
    toPresent.subresourceRange = range;
    toPresent.srcAccessMask = VK_ACCESS_TRANSFER_WRITE_BIT;
    toPresent.dstAccessMask = 0;

    vkCmdPipelineBarrier(
        commandBuffer,
        VK_PIPELINE_STAGE_TRANSFER_BIT,
        VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT,
        0,
        0,
        nullptr,
        0,
        nullptr,
        1,
        &toPresent
    );

    return {};
}

std::expected<void, RenderError> VulkanRenderDevice::Submit(
    const CommandBuffer& commandBuffer, void* waitSemaphore, void* signalSemaphore, void* fence
) noexcept
{
    if (m_deviceLostSimulated.load(std::memory_order_relaxed)) {
        return std::unexpected(RenderError{RenderErrorCode::DeviceLost, "simulated device loss (testing)"});
    }
    const auto vkCommandBuffer = static_cast<VkCommandBuffer>(commandBuffer.GetNativeHandle());
    const auto wait = static_cast<VkSemaphore>(waitSemaphore);
    const auto signal = static_cast<VkSemaphore>(signalSemaphore);
    const auto vkFence = static_cast<VkFence>(fence);

    // Matches RecordClearColor()'s first barrier: whatever waits on
    // `wait` only needs to block the transfer stage, not the whole
    // pipeline, since a clear is the only work submitted so far. The first
    // real graphics pass must change this to
    // VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT (the stage where the
    // swapchain image is first written).
    // The image is first touched at the transfer stage by ClearColor() and at the color
    // attachment output stage by the graphics pass (BeginRendering()); wait for both.
    constexpr VkPipelineStageFlags waitStage =
        VK_PIPELINE_STAGE_TRANSFER_BIT | VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT;

    VkSubmitInfo submitInfo{};
    submitInfo.sType = VK_STRUCTURE_TYPE_SUBMIT_INFO;
    submitInfo.commandBufferCount = 1;
    submitInfo.pCommandBuffers = &vkCommandBuffer;
    if (wait != VK_NULL_HANDLE) {
        submitInfo.waitSemaphoreCount = 1;
        submitInfo.pWaitSemaphores = &wait;
        submitInfo.pWaitDstStageMask = &waitStage;
    }
    if (signal != VK_NULL_HANDLE) {
        submitInfo.signalSemaphoreCount = 1;
        submitInfo.pSignalSemaphores = &signal;
    }

    if (const VkResult result = vkQueueSubmit(m_graphicsQueue, 1, &submitInfo, vkFence); result != VK_SUCCESS) {
        return std::unexpected(MakeVkError(RenderErrorCode::Unknown, "vkQueueSubmit", result));
    }
    return {};
}

std::expected<FrameSync, RenderError>
VulkanRenderDevice::CreateFrameSync(const Swapchain& swapchain, const FrameSyncDesc& desc) noexcept
{
    if (!swapchain.IsValid()) {
        return std::unexpected(
            RenderError{RenderErrorCode::InitializationFailed, "CreateFrameSync called with an invalid Swapchain"}
        );
    }
    const auto* swapchainHandle = static_cast<const VulkanSwapchainHandle*>(swapchain.GetNativeHandle());
    const std::uint32_t slotCount = std::clamp(desc.framesInFlight, std::uint32_t{1}, kMaxFramesInFlight);

    auto handle = std::make_unique<VulkanFrameSyncHandle>();
    handle->slots.resize(slotCount);

    // Nothing has been handed out yet, so a failure rolls back everything
    // created so far right here.
    const auto fail = [&](RenderError error) {
        DestroyFrameSyncResources(m_device, m_commandPool, *handle);
        return std::unexpected(std::move(error));
    };

    for (FrameSlot& slot : handle->slots) {
        VkSemaphoreCreateInfo semaphoreInfo{};
        semaphoreInfo.sType = VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO;
        if (const VkResult result = vkCreateSemaphore(m_device, &semaphoreInfo, nullptr, &slot.imageAvailable);
            result != VK_SUCCESS) {
            return fail(MakeVkError(RenderErrorCode::InitializationFailed, "vkCreateSemaphore", result));
        }

        VkFenceCreateInfo fenceInfo{};
        fenceInfo.sType = VK_STRUCTURE_TYPE_FENCE_CREATE_INFO;
        fenceInfo.flags =
            VK_FENCE_CREATE_SIGNALED_BIT; // the first frame of every slot must not wait for a frame that never ran
        if (const VkResult result = vkCreateFence(m_device, &fenceInfo, nullptr, &slot.inFlight);
            result != VK_SUCCESS) {
            return fail(MakeVkError(RenderErrorCode::InitializationFailed, "vkCreateFence", result));
        }

        VkCommandBufferAllocateInfo allocInfo{};
        allocInfo.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO;
        allocInfo.commandPool = m_commandPool;
        allocInfo.level = VK_COMMAND_BUFFER_LEVEL_PRIMARY;
        allocInfo.commandBufferCount = 1;
        if (const VkResult result = vkAllocateCommandBuffers(m_device, &allocInfo, &slot.commandBuffer);
            result != VK_SUCCESS) {
            return fail(MakeVkError(RenderErrorCode::InitializationFailed, "vkAllocateCommandBuffers", result));
        }
    }

    if (auto result = SyncPerImageState(m_device, *handle, *swapchainHandle); !result) {
        return fail(std::move(result.error()));
    }

    return RenderDevice::MakeFrameSync(this, handle.release(), slotCount);
}

void VulkanRenderDevice::ReleaseFrameSync(void* nativeHandle) noexcept
{
    if (nativeHandle == nullptr) {
        return;
    }
    auto* handle = static_cast<VulkanFrameSyncHandle*>(nativeHandle);

    // If the device is already gone (the FrameSync outlived Shutdown(),
    // against the documented order) there is nothing left to destroy the
    // Vulkan objects with; freeing the bookkeeping is all that can be done.
    if (m_device != VK_NULL_HANDLE) {
        WaitIdle();
        DestroyFrameSyncResources(m_device, m_commandPool, *handle);
    }
    delete handle;
}

std::expected<BeginFrameResult, RenderError>
VulkanRenderDevice::BeginFrameSync(void* frameSyncHandle, Swapchain& swapchain) noexcept
{
    if (m_deviceLostSimulated.load(std::memory_order_relaxed)) {
        return std::unexpected(RenderError{RenderErrorCode::DeviceLost, "simulated device loss (testing)"});
    }
    auto* fs = static_cast<VulkanFrameSyncHandle*>(frameSyncHandle);
    auto* sc = static_cast<VulkanSwapchainHandle*>(swapchain.GetNativeHandle());

    if (fs->failed) {
        return std::unexpected(
            RenderError{RenderErrorCode::Unknown, "FrameSync is unusable after an earlier failure; recreate it"}
        );
    }
    if (fs->frameOpen) {
        // Waiting on the slot's fence below would never return: it was
        // reset when that frame began and is only signaled by its submit.
        return std::unexpected(RenderError{RenderErrorCode::Unknown, "BeginFrame() called again before EndFrame()"});
    }

    // Emptied by a failed Swapchain::Recreate(): nothing to acquire from
    // until a later Recreate() succeeds.
    if (sc->swapchain == VK_NULL_HANDLE) {
        return BeginFrameResult{FrameStatus::OutOfDate, {}};
    }

    // Swapchain::Recreate() waited for the device to go idle, so nothing
    // still uses the old per-image state; a swapchain this FrameSync was
    // never used with gets the same treatment. The image-count check
    // catches a rebuild that happened twice between frames and handed back
    // an equal VkSwapchainKHR value.
    if (sc->swapchain != fs->trackedSwapchain || sc->images.size() != fs->renderFinished.size()) {
        WaitIdle();
        if (auto result = SyncPerImageState(m_device, *fs, *sc); !result) {
            return std::unexpected(std::move(result.error()));
        }
    }

    FrameSlot& slot = fs->slots[fs->currentSlot];

    // The GPU is done with this slot's previous frame — and so with its
    // acquire semaphore and its command buffer — once its fence passed.
    if (const VkResult result = vkWaitForFences(m_device, 1, &slot.inFlight, VK_TRUE, UINT64_MAX);
        result != VK_SUCCESS) {
        return std::unexpected(MakeVkError(RenderErrorCode::Unknown, "vkWaitForFences", result));
    }

    // Nothing is modified until the acquire succeeded: OutOfDate/NotReady
    // leave the fence signaled (a reset fence with no submit coming would
    // deadlock the next wait) and the acquire semaphore untouched.
    std::uint32_t imageIndex = 0;
    const VkResult acquireResult = vkAcquireNextImageKHR(
        m_device, sc->swapchain, kFrameAcquireTimeoutNs, slot.imageAvailable, VK_NULL_HANDLE, &imageIndex
    );
    switch (acquireResult) {
        case VK_SUCCESS:
            fs->acquireSuboptimal = false;
            break;
        case VK_SUBOPTIMAL_KHR:
            fs->acquireSuboptimal =
                true; // still a usable image and a signaled semaphore: render it, rebuild afterwards
            break;
        case VK_ERROR_OUT_OF_DATE_KHR:
            return BeginFrameResult{FrameStatus::OutOfDate, {}};
        case VK_TIMEOUT:
        case VK_NOT_READY:
            return BeginFrameResult{FrameStatus::NotReady, {}};
        default:
            return std::unexpected(MakeVkError(RenderErrorCode::Unknown, "vkAcquireNextImageKHR", acquireResult));
    }

    // From here on the acquire semaphore is signaled and only this frame's
    // submit can consume it: a failure below cannot be skipped past.
    const auto failFrame = [&](RenderError error) {
        fs->failed = true;
        return std::unexpected(std::move(error));
    };

    // More slots than swapchain images: this image may still be in use by
    // a frame of another slot.
    if (const VkFence previous = fs->imageInFlight[imageIndex];
        previous != VK_NULL_HANDLE && previous != slot.inFlight) {
        if (const VkResult result = vkWaitForFences(m_device, 1, &previous, VK_TRUE, UINT64_MAX);
            result != VK_SUCCESS) {
            return failFrame(MakeVkError(RenderErrorCode::Unknown, "vkWaitForFences", result));
        }
    }
    fs->imageInFlight[imageIndex] = slot.inFlight;

    if (const VkResult result = vkResetFences(m_device, 1, &slot.inFlight); result != VK_SUCCESS) {
        return failFrame(MakeVkError(RenderErrorCode::Unknown, "vkResetFences", result));
    }
    if (const VkResult result = vkResetCommandBuffer(slot.commandBuffer, 0); result != VK_SUCCESS) {
        // The fence is already reset with no submit coming, which is why
        // this is fatal rather than skippable.
        return failFrame(MakeVkError(RenderErrorCode::Unknown, "vkResetCommandBuffer", result));
    }

    fs->frameOpen = true;

    BeginFrameResult begin;
    begin.status = FrameStatus::Ready;
    begin.frame.frameIndex = fs->currentSlot;
    begin.frame.imageIndex = imageIndex;
    begin.frame.imageHandle = sc->images[imageIndex];
    begin.frame.extent = Extent2D{sc->extent.width, sc->extent.height};
    begin.frame.commandBuffer = RenderDevice::MakeCommandBuffer(this, slot.commandBuffer);
    return begin;
}

std::expected<SwapchainStatus, RenderError>
VulkanRenderDevice::EndFrameSync(void* frameSyncHandle, Swapchain& swapchain, const Frame& frame) noexcept
{
    if (m_deviceLostSimulated.load(std::memory_order_relaxed)) {
        return std::unexpected(RenderError{RenderErrorCode::DeviceLost, "simulated device loss (testing)"});
    }
    auto* fs = static_cast<VulkanFrameSyncHandle*>(frameSyncHandle);

    if (fs->failed) {
        return std::unexpected(
            RenderError{RenderErrorCode::Unknown, "FrameSync is unusable after an earlier failure; recreate it"}
        );
    }
    if (!fs->frameOpen) {
        return std::unexpected(
            RenderError{RenderErrorCode::Unknown, "EndFrame() called without a matching BeginFrame()"}
        );
    }

    FrameSlot& slot = fs->slots[fs->currentSlot];
    if (frame.frameIndex != fs->currentSlot || frame.imageIndex >= fs->renderFinished.size() ||
        frame.commandBuffer.GetNativeHandle() != slot.commandBuffer) {
        return std::unexpected(
            RenderError{RenderErrorCode::Unknown, "EndFrame() called with a Frame that BeginFrame() did not hand out"}
        );
    }

    const bool acquireSuboptimal = fs->acquireSuboptimal;
    VkSemaphore renderFinished = fs->renderFinished[frame.imageIndex];

    // Advance first: whatever happens below, the slot's fence state is
    // what it is and the next BeginFrame() must look at the next slot.
    fs->frameOpen = false;
    fs->acquireSuboptimal = false;
    fs->currentSlot = (fs->currentSlot + 1) % static_cast<std::uint32_t>(fs->slots.size());

    // Waits for the acquire semaphore, signals the image's render-finished
    // semaphore and this slot's fence.
    if (auto result = Submit(frame.commandBuffer, slot.imageAvailable, renderFinished, slot.inFlight); !result) {
        fs->failed = true; // the fence was reset for a submit that did not happen
        return std::unexpected(std::move(result.error()));
    }

    auto present = PresentSwapchainImage(swapchain.GetNativeHandle(), frame.imageIndex, renderFinished);
    if (!present) {
        fs->failed = true;
        return std::unexpected(std::move(present.error()));
    }

    if (*present == SwapchainStatus::Ok && acquireSuboptimal) {
        return SwapchainStatus::Suboptimal;
    }
    return *present;
}

void VulkanRenderDevice::SetLogCallback(LogCallback callback) noexcept
{
    const std::lock_guard<std::mutex> lock(m_logCallbackMutex);
    m_logCallback = std::move(callback);
}

/// Called by DebugMessengerCallback (via the `pUserData` set in
/// SetupDebugMessenger) for every message the debug messenger surfaces.
/// Not part of the RenderDevice interface — only DebugMessengerCallback
/// calls this, through the `VulkanRenderDevice*` it gets back from
/// pUserData, which is why this is a plain (non-override) method rather
/// than something declared on RenderDevice itself.
void VulkanRenderDevice::NotifyLogCallback(LogSeverity severity, std::string_view message) noexcept
{
    const std::lock_guard<std::mutex> lock(m_logCallbackMutex);
    if (m_logCallback) {
        m_logCallback(severity, message);
    }
}

void VulkanRenderDevice::WaitIdle() noexcept
{
    if (m_device != VK_NULL_HANDLE) {
        vkDeviceWaitIdle(m_device);
    }
}

void VulkanRenderDevice::Shutdown() noexcept
{
    // Everything below needs a live VkDevice, so make sure no
    // outstanding GPU work is still touching it before tearing
    // anything down — matches WaitIdle()'s own contract.
    WaitIdle();

    if (m_commandPool != VK_NULL_HANDLE) {
        vkDestroyCommandPool(m_device, m_commandPool, nullptr);
        m_commandPool = VK_NULL_HANDLE;
    }

    // Allocator must be torn down before the VkDevice it wraps — it
    // still needs a valid device to free any memory it holds.
    if (m_allocator != VK_NULL_HANDLE) {
        vmaDestroyAllocator(m_allocator);
        m_allocator = VK_NULL_HANDLE;
    }

    if (m_device != VK_NULL_HANDLE) {
        vkDestroyDevice(m_device, nullptr);
        m_device = VK_NULL_HANDLE;
        m_graphicsQueue = VK_NULL_HANDLE;
    }

    if (m_debugMessenger != VK_NULL_HANDLE) {
        vkDestroyDebugUtilsMessengerEXT(m_instance, m_debugMessenger, nullptr);
        m_debugMessenger = VK_NULL_HANDLE;
    }

    if (m_instance != VK_NULL_HANDLE) {
        vkDestroyInstance(m_instance, nullptr);
        m_instance = VK_NULL_HANDLE;
    }
}

// ---------------------------------------------------------------------------
// Graphics pass: buffers writes, pipelines and rendering.md commands (Phase 10)
// ---------------------------------------------------------------------------

namespace {

RenderError UnsupportedDynamicRendering()
{
    return RenderError{
        RenderErrorCode::Unsupported, "the device lacks Vulkan 1.3 dynamic rendering.md, which the graphics pass needs"
    };
}

VkFormat ToVkFormat(VertexFormat format) noexcept
{
    switch (format) {
        case VertexFormat::Float2:
            return VK_FORMAT_R32G32_SFLOAT;
        case VertexFormat::Float3:
            return VK_FORMAT_R32G32B32_SFLOAT;
        case VertexFormat::Float4:
            return VK_FORMAT_R32G32B32A32_SFLOAT;
    }
    return VK_FORMAT_UNDEFINED;
}

VkCullModeFlags ToVkCullMode(CullMode mode) noexcept
{
    switch (mode) {
        case CullMode::Off:
            return VK_CULL_MODE_NONE;
        case CullMode::Back:
            return VK_CULL_MODE_BACK_BIT;
        case CullMode::Front:
            return VK_CULL_MODE_FRONT_BIT;
    }
    return VK_CULL_MODE_NONE;
}

constexpr uint32_t kMaxPushConstantSize = 128; // the minimum maxPushConstantsSize every device must support
constexpr VkShaderStageFlags kPushConstantStages = VK_SHADER_STAGE_VERTEX_BIT | VK_SHADER_STAGE_FRAGMENT_BIT;

VkShaderModule CreateShaderModule(VkDevice device, std::span<const std::uint32_t> words) noexcept
{
    VkShaderModuleCreateInfo info{};
    info.sType = VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO;
    info.codeSize = words.size_bytes();
    info.pCode = words.data();
    VkShaderModule module = VK_NULL_HANDLE;
    if (vkCreateShaderModule(device, &info, nullptr, &module) != VK_SUCCESS) {
        return VK_NULL_HANDLE;
    }
    return module;
}

} // namespace

void VulkanRenderDevice::SimulateDeviceLossForTesting() noexcept
{
    m_deviceLostSimulated.store(true, std::memory_order_relaxed);
}

std::expected<void, RenderError>
VulkanRenderDevice::WriteBufferData(void* bufferHandle, const void* data, std::size_t size, std::size_t offset) noexcept
{
    const auto* handle = static_cast<const VulkanBufferHandle*>(bufferHandle);
    if (!handle->hostAccess) {
        return std::unexpected(
            RenderError{RenderErrorCode::Unsupported, "Buffer::Write needs a host-visible buffer (not GpuOnly)"}
        );
    }
    void* mapped = nullptr;
    if (const VkResult result = vmaMapMemory(m_allocator, handle->allocation, &mapped); result != VK_SUCCESS) {
        return std::unexpected(
            RenderError{RenderErrorCode::Unsupported, "Buffer::Write needs a host-visible buffer (not GpuOnly)"}
        );
    }
    std::memcpy(static_cast<char*>(mapped) + offset, data, size);
    vmaFlushAllocation(m_allocator, handle->allocation, offset, size); // no-op on coherent memory
    vmaUnmapMemory(m_allocator, handle->allocation);
    return {};
}

std::expected<GraphicsPipeline, RenderError>
VulkanRenderDevice::CreateGraphicsPipeline(const Swapchain& swapchain, const GraphicsPipelineDesc& desc) noexcept
{
    if (!m_dynamicRendering) {
        return std::unexpected(UnsupportedDynamicRendering());
    }
    const auto invalid = [](const char* message) {
        return std::unexpected(RenderError{RenderErrorCode::InitializationFailed, message});
    };
    if (desc.vertexShader.empty() || desc.fragmentShader.empty()) {
        return invalid("GraphicsPipelineDesc needs both a vertex and a fragment shader");
    }
    if (desc.pushConstantSize > kMaxPushConstantSize || desc.pushConstantSize % 4 != 0) {
        return invalid("GraphicsPipelineDesc::pushConstantSize must be a multiple of 4 and at most 128");
    }
    if (!desc.vertexAttributes.empty() && desc.vertexStride == 0) {
        return invalid("GraphicsPipelineDesc has vertex attributes but a zero vertexStride");
    }
    const auto* target = static_cast<const VulkanSwapchainHandle*>(swapchain.GetNativeHandle());
    if (target == nullptr || target->swapchain == VK_NULL_HANDLE) {
        return invalid("CreateGraphicsPipeline needs a valid (non-empty) swapchain");
    }
    if (desc.depthTest && target->depthFormat == VK_FORMAT_UNDEFINED) {
        return invalid("depthTest needs a swapchain created with SwapchainDesc::depthBuffer");
    }

    const VkShaderModule vertexModule = CreateShaderModule(m_device, desc.vertexShader);
    const VkShaderModule fragmentModule = CreateShaderModule(m_device, desc.fragmentShader);
    const auto destroyModules = [&] {
        vkDestroyShaderModule(m_device, vertexModule, nullptr); // VK_NULL_HANDLE is a valid no-op
        vkDestroyShaderModule(m_device, fragmentModule, nullptr);
    };
    if (vertexModule == VK_NULL_HANDLE || fragmentModule == VK_NULL_HANDLE) {
        destroyModules();
        return std::unexpected(RenderError{RenderErrorCode::InitializationFailed, "vkCreateShaderModule failed"});
    }

    VkPipelineShaderStageCreateInfo stages[2]{};
    stages[0].sType = VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO;
    stages[0].stage = VK_SHADER_STAGE_VERTEX_BIT;
    stages[0].module = vertexModule;
    stages[0].pName = "main";
    stages[1].sType = VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO;
    stages[1].stage = VK_SHADER_STAGE_FRAGMENT_BIT;
    stages[1].module = fragmentModule;
    stages[1].pName = "main";

    VkVertexInputBindingDescription binding{};
    binding.binding = 0;
    binding.stride = desc.vertexStride;
    binding.inputRate = VK_VERTEX_INPUT_RATE_VERTEX;
    std::vector<VkVertexInputAttributeDescription> attributes;
    attributes.reserve(desc.vertexAttributes.size());
    for (const VertexAttribute& attribute : desc.vertexAttributes) {
        VkVertexInputAttributeDescription out{};
        out.location = attribute.location;
        out.binding = 0;
        out.format = ToVkFormat(attribute.format);
        out.offset = attribute.offset;
        attributes.push_back(out);
    }
    VkPipelineVertexInputStateCreateInfo vertexInput{};
    vertexInput.sType = VK_STRUCTURE_TYPE_PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO;
    vertexInput.vertexBindingDescriptionCount = desc.vertexStride != 0 ? 1U : 0U;
    vertexInput.pVertexBindingDescriptions = &binding;
    vertexInput.vertexAttributeDescriptionCount = static_cast<uint32_t>(attributes.size());
    vertexInput.pVertexAttributeDescriptions = attributes.data();

    VkPipelineInputAssemblyStateCreateInfo inputAssembly{};
    inputAssembly.sType = VK_STRUCTURE_TYPE_PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO;
    inputAssembly.topology = VK_PRIMITIVE_TOPOLOGY_TRIANGLE_LIST;

    VkPipelineViewportStateCreateInfo viewport{};
    viewport.sType = VK_STRUCTURE_TYPE_PIPELINE_VIEWPORT_STATE_CREATE_INFO;
    viewport.viewportCount = 1;
    viewport.scissorCount = 1;

    // Front faces are counter-clockwise in a Y-up clip space; BeginRendering() flips Y with a
    // negative-height viewport, which keeps that convention (the same one the math crate uses).
    VkPipelineRasterizationStateCreateInfo rasterization{};
    rasterization.sType = VK_STRUCTURE_TYPE_PIPELINE_RASTERIZATION_STATE_CREATE_INFO;
    rasterization.polygonMode = VK_POLYGON_MODE_FILL;
    rasterization.cullMode = ToVkCullMode(desc.cullMode);
    rasterization.frontFace = VK_FRONT_FACE_COUNTER_CLOCKWISE;
    rasterization.lineWidth = 1.0f;

    VkPipelineMultisampleStateCreateInfo multisample{};
    multisample.sType = VK_STRUCTURE_TYPE_PIPELINE_MULTISAMPLE_STATE_CREATE_INFO;
    multisample.rasterizationSamples = VK_SAMPLE_COUNT_1_BIT;

    VkPipelineDepthStencilStateCreateInfo depthStencil{};
    depthStencil.sType = VK_STRUCTURE_TYPE_PIPELINE_DEPTH_STENCIL_STATE_CREATE_INFO;
    depthStencil.depthTestEnable = desc.depthTest ? VK_TRUE : VK_FALSE;
    depthStencil.depthWriteEnable = (desc.depthTest && desc.depthWrite) ? VK_TRUE : VK_FALSE;
    depthStencil.depthCompareOp = VK_COMPARE_OP_LESS;

    VkPipelineColorBlendAttachmentState blendAttachment{};
    blendAttachment.colorWriteMask =
        VK_COLOR_COMPONENT_R_BIT | VK_COLOR_COMPONENT_G_BIT | VK_COLOR_COMPONENT_B_BIT | VK_COLOR_COMPONENT_A_BIT;
    VkPipelineColorBlendStateCreateInfo colorBlend{};
    colorBlend.sType = VK_STRUCTURE_TYPE_PIPELINE_COLOR_BLEND_STATE_CREATE_INFO;
    colorBlend.attachmentCount = 1;
    colorBlend.pAttachments = &blendAttachment;

    const VkDynamicState dynamicStates[] = {VK_DYNAMIC_STATE_VIEWPORT, VK_DYNAMIC_STATE_SCISSOR};
    VkPipelineDynamicStateCreateInfo dynamic{};
    dynamic.sType = VK_STRUCTURE_TYPE_PIPELINE_DYNAMIC_STATE_CREATE_INFO;
    dynamic.dynamicStateCount = 2;
    dynamic.pDynamicStates = dynamicStates;

    VkPushConstantRange pushRange{};
    pushRange.stageFlags = kPushConstantStages;
    pushRange.offset = 0;
    pushRange.size = desc.pushConstantSize;
    VkPipelineLayoutCreateInfo layoutInfo{};
    layoutInfo.sType = VK_STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO;
    layoutInfo.pushConstantRangeCount = desc.pushConstantSize != 0 ? 1U : 0U;
    layoutInfo.pPushConstantRanges = &pushRange;
    VkPipelineLayout layout = VK_NULL_HANDLE;
    if (const VkResult result = vkCreatePipelineLayout(m_device, &layoutInfo, nullptr, &layout); result != VK_SUCCESS) {
        destroyModules();
        return std::unexpected(MakeVkError(RenderErrorCode::InitializationFailed, "vkCreatePipelineLayout", result));
    }

    // Dynamic rendering.md: the attachment formats are part of the pipeline instead of a render pass.
    // The depth format is declared whenever the swapchain has a depth buffer (BeginRendering()
    // always attaches it), even for a pipeline that does not test depth.
    VkPipelineRenderingCreateInfo rendering{};
    rendering.sType = VK_STRUCTURE_TYPE_PIPELINE_RENDERING_CREATE_INFO;
    rendering.colorAttachmentCount = 1;
    rendering.pColorAttachmentFormats = &target->format;
    rendering.depthAttachmentFormat = target->depthFormat;
    rendering.stencilAttachmentFormat = HasStencil(target->depthFormat) ? target->depthFormat : VK_FORMAT_UNDEFINED;

    VkGraphicsPipelineCreateInfo pipelineInfo{};
    pipelineInfo.sType = VK_STRUCTURE_TYPE_GRAPHICS_PIPELINE_CREATE_INFO;
    pipelineInfo.pNext = &rendering;
    pipelineInfo.stageCount = 2;
    pipelineInfo.pStages = stages;
    pipelineInfo.pVertexInputState = &vertexInput;
    pipelineInfo.pInputAssemblyState = &inputAssembly;
    pipelineInfo.pViewportState = &viewport;
    pipelineInfo.pRasterizationState = &rasterization;
    pipelineInfo.pMultisampleState = &multisample;
    pipelineInfo.pDepthStencilState = &depthStencil;
    pipelineInfo.pColorBlendState = &colorBlend;
    pipelineInfo.pDynamicState = &dynamic;
    pipelineInfo.layout = layout;
    pipelineInfo.renderPass = VK_NULL_HANDLE;

    VkPipeline pipeline = VK_NULL_HANDLE;
    const VkResult pipelineResult =
        vkCreateGraphicsPipelines(m_device, VK_NULL_HANDLE, 1, &pipelineInfo, nullptr, &pipeline);
    destroyModules();
    if (pipelineResult != VK_SUCCESS) {
        vkDestroyPipelineLayout(m_device, layout, nullptr);
        return std::unexpected(
            MakeVkError(RenderErrorCode::InitializationFailed, "vkCreateGraphicsPipelines", pipelineResult)
        );
    }

    auto* handle = new (std::nothrow) VulkanPipelineHandle{pipeline, layout, desc.pushConstantSize};
    if (handle == nullptr) {
        vkDestroyPipeline(m_device, pipeline, nullptr);
        vkDestroyPipelineLayout(m_device, layout, nullptr);
        return std::unexpected(RenderError{RenderErrorCode::InitializationFailed, "out of memory for pipeline handle"});
    }
    return MakeGraphicsPipeline(this, handle, desc.pushConstantSize);
}

void VulkanRenderDevice::ReleaseGraphicsPipeline(void* nativeHandle) noexcept
{
    if (nativeHandle == nullptr) {
        return;
    }
    auto* handle = static_cast<VulkanPipelineHandle*>(nativeHandle);
    vkDestroyPipeline(m_device, handle->pipeline, nullptr);
    vkDestroyPipelineLayout(m_device, handle->layout, nullptr);
    delete handle;
}

std::expected<void, RenderError> VulkanRenderDevice::RecordBeginRendering(
    void* commandBufferHandle, void* swapchainHandle, std::uint32_t imageIndex, const RenderingDesc& desc
) noexcept
{
    if (!m_dynamicRendering) {
        return std::unexpected(UnsupportedDynamicRendering());
    }
    const auto* target = static_cast<const VulkanSwapchainHandle*>(swapchainHandle);
    if (target == nullptr || target->swapchain == VK_NULL_HANDLE || imageIndex >= target->images.size()) {
        return std::unexpected(
            RenderError{RenderErrorCode::InitializationFailed, "BeginRendering: invalid swapchain or image index"}
        );
    }
    const auto commandBuffer = static_cast<VkCommandBuffer>(commandBufferHandle);

    // Color: the image arrives from vkAcquireNextImageKHR (contents irrelevant, it is cleared);
    // Submit() waits for the acquire semaphore at the color attachment output stage, which is
    // the source stage here so the transition happens after the image is really available.
    VkImageMemoryBarrier toColor{};
    toColor.sType = VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER;
    toColor.srcAccessMask = 0;
    toColor.dstAccessMask = VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT;
    toColor.oldLayout = VK_IMAGE_LAYOUT_UNDEFINED;
    toColor.newLayout = VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL;
    toColor.srcQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED;
    toColor.dstQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED;
    toColor.image = target->images[imageIndex];
    toColor.subresourceRange = {VK_IMAGE_ASPECT_COLOR_BIT, 0, 1, 0, 1};
    vkCmdPipelineBarrier(
        commandBuffer,
        VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
        VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
        0,
        0,
        nullptr,
        0,
        nullptr,
        1,
        &toColor
    );

    const bool hasDepth = target->depthFormat != VK_FORMAT_UNDEFINED;
    if (hasDepth) {
        // One depth image serves every frame in flight. This barrier's source scope covers all
        // earlier-submitted depth work in queue order, so frame N+1 never overlaps frame N's
        // depth accesses.
        constexpr VkPipelineStageFlags depthStages =
            VK_PIPELINE_STAGE_EARLY_FRAGMENT_TESTS_BIT | VK_PIPELINE_STAGE_LATE_FRAGMENT_TESTS_BIT;
        VkImageMemoryBarrier toDepth{};
        toDepth.sType = VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER;
        toDepth.srcAccessMask = VK_ACCESS_DEPTH_STENCIL_ATTACHMENT_WRITE_BIT;
        toDepth.dstAccessMask =
            VK_ACCESS_DEPTH_STENCIL_ATTACHMENT_READ_BIT | VK_ACCESS_DEPTH_STENCIL_ATTACHMENT_WRITE_BIT;
        toDepth.oldLayout = VK_IMAGE_LAYOUT_UNDEFINED; // contents are cleared below
        toDepth.newLayout = VK_IMAGE_LAYOUT_DEPTH_STENCIL_ATTACHMENT_OPTIMAL;
        toDepth.srcQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED;
        toDepth.dstQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED;
        toDepth.image = target->depthImage;
        toDepth.subresourceRange = {DepthAspectOf(target->depthFormat), 0, 1, 0, 1};
        vkCmdPipelineBarrier(commandBuffer, depthStages, depthStages, 0, 0, nullptr, 0, nullptr, 1, &toDepth);
    }

    VkRenderingAttachmentInfo colorAttachment{};
    colorAttachment.sType = VK_STRUCTURE_TYPE_RENDERING_ATTACHMENT_INFO;
    colorAttachment.imageView = target->imageViews[imageIndex];
    colorAttachment.imageLayout = VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL;
    colorAttachment.loadOp = VK_ATTACHMENT_LOAD_OP_CLEAR;
    colorAttachment.storeOp = VK_ATTACHMENT_STORE_OP_STORE;
    colorAttachment.clearValue.color = {{desc.color.r, desc.color.g, desc.color.b, desc.color.a}};

    VkRenderingAttachmentInfo depthAttachment{};
    depthAttachment.sType = VK_STRUCTURE_TYPE_RENDERING_ATTACHMENT_INFO;
    depthAttachment.imageView = target->depthView;
    depthAttachment.imageLayout = VK_IMAGE_LAYOUT_DEPTH_STENCIL_ATTACHMENT_OPTIMAL;
    depthAttachment.loadOp = VK_ATTACHMENT_LOAD_OP_CLEAR;
    depthAttachment.storeOp = VK_ATTACHMENT_STORE_OP_DONT_CARE;
    depthAttachment.clearValue.depthStencil = {desc.clearDepth, 0};

    VkRenderingAttachmentInfo stencilAttachment = depthAttachment;
    stencilAttachment.loadOp = VK_ATTACHMENT_LOAD_OP_DONT_CARE;

    VkRenderingInfo renderingInfo{};
    renderingInfo.sType = VK_STRUCTURE_TYPE_RENDERING_INFO;
    renderingInfo.renderArea = {{0, 0}, target->extent};
    renderingInfo.layerCount = 1;
    renderingInfo.colorAttachmentCount = 1;
    renderingInfo.pColorAttachments = &colorAttachment;
    renderingInfo.pDepthAttachment = hasDepth ? &depthAttachment : nullptr;
    renderingInfo.pStencilAttachment = (hasDepth && HasStencil(target->depthFormat)) ? &stencilAttachment : nullptr;
    vkCmdBeginRendering(commandBuffer, &renderingInfo);

    // Negative height flips Y (core since Vulkan 1.1): clip space stays Y-up like the math crate's.
    VkViewport viewport{};
    viewport.x = 0.0f;
    viewport.y = static_cast<float>(target->extent.height);
    viewport.width = static_cast<float>(target->extent.width);
    viewport.height = -static_cast<float>(target->extent.height);
    viewport.minDepth = 0.0f;
    viewport.maxDepth = 1.0f;
    vkCmdSetViewport(commandBuffer, 0, 1, &viewport);
    const VkRect2D scissor{{0, 0}, target->extent};
    vkCmdSetScissor(commandBuffer, 0, 1, &scissor);
    return {};
}

std::expected<void, RenderError> VulkanRenderDevice::RecordEndRendering(
    void* commandBufferHandle, void* swapchainHandle, std::uint32_t imageIndex
) noexcept
{
    if (!m_dynamicRendering) {
        return std::unexpected(UnsupportedDynamicRendering());
    }
    const auto* target = static_cast<const VulkanSwapchainHandle*>(swapchainHandle);
    if (target == nullptr || imageIndex >= target->images.size()) {
        return std::unexpected(
            RenderError{RenderErrorCode::InitializationFailed, "EndRendering: invalid swapchain or image index"}
        );
    }
    const auto commandBuffer = static_cast<VkCommandBuffer>(commandBufferHandle);
    vkCmdEndRendering(commandBuffer);

    VkImageMemoryBarrier toPresent{};
    toPresent.sType = VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER;
    toPresent.srcAccessMask = VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT;
    toPresent.dstAccessMask = 0;
    toPresent.oldLayout = VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL;
    toPresent.newLayout = VK_IMAGE_LAYOUT_PRESENT_SRC_KHR;
    toPresent.srcQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED;
    toPresent.dstQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED;
    toPresent.image = target->images[imageIndex];
    toPresent.subresourceRange = {VK_IMAGE_ASPECT_COLOR_BIT, 0, 1, 0, 1};
    vkCmdPipelineBarrier(
        commandBuffer,
        VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
        VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT,
        0,
        0,
        nullptr,
        0,
        nullptr,
        1,
        &toPresent
    );
    return {};
}

std::expected<void, RenderError>
VulkanRenderDevice::RecordBindPipeline(void* commandBufferHandle, void* pipelineHandle) noexcept
{
    const auto* pipeline = static_cast<const VulkanPipelineHandle*>(pipelineHandle);
    vkCmdBindPipeline(
        static_cast<VkCommandBuffer>(commandBufferHandle), VK_PIPELINE_BIND_POINT_GRAPHICS, pipeline->pipeline
    );
    return {};
}

std::expected<void, RenderError>
VulkanRenderDevice::RecordBindVertexBuffer(void* commandBufferHandle, void* bufferHandle, std::size_t offset) noexcept
{
    const auto* buffer = static_cast<const VulkanBufferHandle*>(bufferHandle);
    const VkDeviceSize vkOffset = offset;
    vkCmdBindVertexBuffers(static_cast<VkCommandBuffer>(commandBufferHandle), 0, 1, &buffer->buffer, &vkOffset);
    return {};
}

std::expected<void, RenderError> VulkanRenderDevice::RecordBindIndexBuffer(
    void* commandBufferHandle, void* bufferHandle, IndexType type, std::size_t offset
) noexcept
{
    const auto* buffer = static_cast<const VulkanBufferHandle*>(bufferHandle);
    vkCmdBindIndexBuffer(
        static_cast<VkCommandBuffer>(commandBufferHandle),
        buffer->buffer,
        offset,
        type == IndexType::U16 ? VK_INDEX_TYPE_UINT16 : VK_INDEX_TYPE_UINT32
    );
    return {};
}

std::expected<void, RenderError> VulkanRenderDevice::RecordPushConstants(
    void* commandBufferHandle, void* pipelineHandle, const void* data, std::uint32_t size
) noexcept
{
    const auto* pipeline = static_cast<const VulkanPipelineHandle*>(pipelineHandle);
    vkCmdPushConstants(
        static_cast<VkCommandBuffer>(commandBufferHandle), pipeline->layout, kPushConstantStages, 0, size, data
    );
    return {};
}

std::expected<void, RenderError> VulkanRenderDevice::RecordDraw(
    void* commandBufferHandle,
    std::uint32_t vertexCount,
    std::uint32_t instanceCount,
    std::uint32_t firstVertex,
    std::uint32_t firstInstance
) noexcept
{
    vkCmdDraw(
        static_cast<VkCommandBuffer>(commandBufferHandle), vertexCount, instanceCount, firstVertex, firstInstance
    );
    return {};
}

std::expected<void, RenderError> VulkanRenderDevice::RecordDrawIndexed(
    void* commandBufferHandle,
    std::uint32_t indexCount,
    std::uint32_t instanceCount,
    std::uint32_t firstIndex,
    std::int32_t vertexOffset,
    std::uint32_t firstInstance
) noexcept
{
    vkCmdDrawIndexed(
        static_cast<VkCommandBuffer>(commandBufferHandle),
        indexCount,
        instanceCount,
        firstIndex,
        vertexOffset,
        firstInstance
    );
    return {};
}

} // namespace renderer::backend::vulkan
