#pragma once

#include <string>

namespace renderer
{

    /// Coarse-grained reason a renderer operation failed. Kept small and
    /// backend-agnostic on purpose — this is the "common base" side of the
    /// abstraction; a backend that needs to preserve its own diagnostic
    /// detail (a VkResult, an HRESULT, ...) puts it in `detail` rather than
    /// growing this enum with backend-specific cases.
    enum class RenderErrorCode
    {
        BackendUnavailable,   ///< Requested GraphicsAPI has no backend in this build.
        InitializationFailed, ///< Backend-level init (instance/device/etc.) failed.
        NoSuitableDevice,     ///< No physical device met minimum requirements.
        OutOfMemory,
        DeviceLost,
        Unknown,
    };

    /// Backend-agnostic error payload for std::expected-returning renderer
    /// calls. `detail` carries the backend's own message (e.g. a formatted
    /// VkResult) for logging; callers that only need to branch on failure
    /// kind should switch on `code` and treat `detail` as opaque.
    struct RenderError
    {
        RenderErrorCode code;
        std::string detail;
    };

} // namespace renderer
