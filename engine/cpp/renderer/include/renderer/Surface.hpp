#pragma once

#include <cstddef>

namespace renderer
{

class RenderDevice;

/// Raw native window handle, passed straight through to the backend —
/// deliberately *not* wrapped in a class hierarchy or a windowing
/// abstraction. Every platform's fields are the exact types its own
/// windowing/Vulkan WSI extension already expects (Win32's HWND +
/// HINSTANCE, Xlib's Display*/Window, Wayland's wl_display*/wl_surface*);
/// only one branch is compiled per platform. The caller (Qt editor,
/// or a dummy test window here in Faz 5.3) fills in exactly the fields
/// its platform has — there is nothing else to abstract here without
/// adding a layer that would just be unwrapped again one line later.
struct NativeWindowHandle
{
#if defined(_WIN32)
    void* hinstance = nullptr; ///< HINSTANCE
    void* hwnd = nullptr;      ///< HWND
#elif defined(__APPLE__)
    void* metalLayer = nullptr; ///< CAMetalLayer*
#elif defined(__linux__)
    // Both are optional; a caller fills in whichever windowing system
    // it's actually using (X11 vs Wayland) and leaves the other null.
    void* xlibDisplay = nullptr; ///< Display*
    unsigned long xlibWindow = 0; ///< Window (Xlib)
    void* waylandDisplay = nullptr; ///< wl_display*
    void* waylandSurface = nullptr; ///< wl_surface*
#endif
};

/// Move-only RAII handle to a platform presentation surface. Construction
/// happens through RenderDevice::CreateSurface(); the destructor releases
/// the backend resource automatically.
///
/// A Surface must be destroyed before the RenderDevice that created it
/// calls Shutdown() — same ordering rule as Buffer (RenderDevice.hpp);
/// the device does not track surfaces it handed out, so nothing else
/// enforces this.
class Surface
{
public:
    Surface() noexcept = default;
    ~Surface();

    Surface(const Surface&) = delete;
    Surface& operator=(const Surface&) = delete;

    Surface(Surface&& other) noexcept;
    Surface& operator=(Surface&& other) noexcept;

    [[nodiscard]] bool IsValid() const noexcept
    {
        return m_nativeHandle != nullptr;
    }

    /// Backend-owned opaque resource pointer — same escape-hatch
    /// contract as Buffer::GetNativeHandle() (see Buffer.hpp).
    [[nodiscard]] void* GetNativeHandle() const noexcept
    {
        return m_nativeHandle;
    }

private:
    friend class RenderDevice;

    Surface(RenderDevice* device, void* nativeHandle) noexcept
        : m_device(device)
        , m_nativeHandle(nativeHandle)
    {
    }

    void Release() noexcept;

    RenderDevice* m_device = nullptr;
    void* m_nativeHandle = nullptr;
};

} // namespace renderer
