#pragma once

#include <cstdint>
#include <expected>

#include "renderer/CommandBuffer.hpp"
#include "renderer/RenderError.hpp"
#include "renderer/Swapchain.hpp"

namespace renderer {

class RenderDevice;

/// Upper bound for FrameSyncDesc::framesInFlight; larger requests are clamped.
inline constexpr std::uint32_t kMaxFramesInFlight = 8;

struct FrameSyncDesc {
    /// How many frames the CPU may record ahead of the GPU. 2 is the usual
    /// trade-off between latency and keeping both processors busy. Clamped
    /// to [1, kMaxFramesInFlight].
    std::uint32_t framesInFlight = 2;
};

/// Everything one frame of rendering needs, handed out by
/// FrameSync::BeginFrame() and handed back to FrameSync::EndFrame().
struct Frame {
    std::uint32_t frameIndex = 0; ///< 0 .. framesInFlight-1: which in-flight slot this frame occupies.
    std::uint32_t imageIndex = 0; ///< Which swapchain image this frame renders into.
    void* imageHandle = nullptr;  ///< Swapchain::GetImageNativeHandle(imageIndex) — the target for ClearColor().
    Extent2D extent{};            ///< Size of that image in pixels.

    /// Reset and ready to record: the caller does Begin() ... End(). Owned
    /// by the FrameSync and reused every framesInFlight frames — it must
    /// not be kept past EndFrame().
    CommandBuffer commandBuffer{};
};

enum class FrameStatus {
    Ready,     ///< `frame` is valid; record it and pass it to EndFrame() — that call is mandatory.
    OutOfDate, ///< The swapchain must be rebuilt (Swapchain::Recreate()) before another frame can start.
    NotReady,  ///< No image became available in time (hidden/occluded window, ...). Nothing started; try again later.
};

struct BeginFrameResult {
    FrameStatus status = FrameStatus::NotReady;
    Frame frame{}; ///< Only meaningful when status == FrameStatus::Ready.
};

/// Move-only RAII owner of everything needed to keep several frames in
/// flight at once: per in-flight slot an "image available" semaphore, a
/// "frame finished" fence and a command buffer, and per swapchain image a
/// "render finished" semaphore. Construction happens through
/// RenderDevice::CreateFrameSync(); the destructor releases all of it.
///
/// This is the frame-level answer to the synchronization question, on
/// purpose instead of exposing semaphores and fences: those are Vulkan
/// concepts (D3D12 has fences but no binary semaphores, Metal has neither),
/// whereas "begin a frame, record, end a frame" is what every backend can
/// express in its own terms.
///
/// One frame:
///
///   auto begin = sync.BeginFrame(swapchain);        // waits for the slot's previous frame, acquires an image
///   if (!begin) { /* error: stop rendering */ }
///   if (begin->status == FrameStatus::OutOfDate) { swapchain.Recreate(...); return; }
///   if (begin->status == FrameStatus::NotReady)  { return; }
///   Frame& frame = begin->frame;
///   frame.commandBuffer.Begin(); ... record into frame.imageHandle ...; frame.commandBuffer.End();
///   auto status = sync.EndFrame(swapchain, frame);  // submits and presents
///   if (status && *status != SwapchainStatus::Ok) { /* rebuild the swapchain soon */ }
///
/// Rules:
///  * A BeginFrame() that returns Ready must be followed by EndFrame().
///    The image is already acquired at that point; abandoning the frame
///    would leave the acquire unmatched. A failure while recording is
///    therefore fatal for the loop (stop rendering and rebuild the device),
///    not something to skip past — and an EndFrame() that fails leaves the
///    FrameSync unusable: every later BeginFrame() reports an error.
///  * The swapchain is passed on each call instead of being bound at
///    creation, so Swapchain::Recreate() — which keeps the Swapchain
///    object — needs no bookkeeping here: BeginFrame() notices a rebuilt
///    swapchain (new images, possibly a different image count) by itself.
///  * Between a Ready BeginFrame() and its EndFrame() the swapchain must
///    not be rebuilt: rebuild after EndFrame() (which is exactly where
///    Suboptimal/OutOfDate are reported), or on OutOfDate/NotReady from
///    BeginFrame().
///  * Single-threaded use: BeginFrame()/EndFrame() from one thread at a
///    time, alternating.
///
/// Lifetime: a FrameSync must be destroyed before the RenderDevice's
/// Shutdown() — same ordering rule as Buffer/Surface/Swapchain — and a
/// useful place in the teardown order is before the Swapchain it was
/// used with. Destroying it waits for the device to go idle.
class FrameSync
{
public:
    FrameSync() noexcept = default;
    ~FrameSync();

    FrameSync(const FrameSync&) = delete;
    FrameSync& operator=(const FrameSync&) = delete;

    FrameSync(FrameSync&& other) noexcept;
    FrameSync& operator=(FrameSync&& other) noexcept;

    [[nodiscard]] bool IsValid() const noexcept
    {
        return m_nativeHandle != nullptr;
    }

    /// The (clamped) number of frames in flight this object was built with.
    [[nodiscard]] std::uint32_t GetFramesInFlight() const noexcept
    {
        return m_framesInFlight;
    }

    /// Starts the next frame: waits until the GPU finished the frame that
    /// last used this in-flight slot, acquires the next swapchain image
    /// (giving up after a short internal timeout rather than blocking the
    /// caller indefinitely) and hands out a reset command buffer.
    /// OutOfDate and NotReady are outcomes, not errors — see FrameStatus.
    /// An error means the device or surface is unusable (RenderErrorCode::
    /// DeviceLost, ...); the loop should stop.
    [[nodiscard]] std::expected<BeginFrameResult, RenderError> BeginFrame(Swapchain& swapchain) noexcept;

    /// Submits the frame's command buffer and presents its image. The
    /// returned status tells whether the swapchain should be rebuilt:
    /// Suboptimal/OutOfDate are routine (typically after a resize), and
    /// the frame was still submitted — rebuild before the next
    /// BeginFrame(). Suboptimal is also returned when BeginFrame() saw it.
    [[nodiscard]] std::expected<SwapchainStatus, RenderError>
    EndFrame(Swapchain& swapchain, const Frame& frame) noexcept;

    /// Backend-owned opaque resource pointer — same escape-hatch
    /// contract as Buffer::GetNativeHandle() (see Buffer.hpp).
    [[nodiscard]] void* GetNativeHandle() const noexcept
    {
        return m_nativeHandle;
    }

private:
    friend class RenderDevice;

    FrameSync(RenderDevice* device, void* nativeHandle, std::uint32_t framesInFlight) noexcept
        : m_device(device), m_nativeHandle(nativeHandle), m_framesInFlight(framesInFlight)
    {}

    void Release() noexcept;

    RenderDevice* m_device = nullptr;
    void* m_nativeHandle = nullptr;
    std::uint32_t m_framesInFlight = 0;
};

} // namespace renderer
