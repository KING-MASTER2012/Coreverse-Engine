#pragma once

#include <cstdint>

namespace renderer {

    /// Number of graphics-API validation *errors* reported since the process
    /// started (or since the last ResetValidationErrorCount()) — for Vulkan,
    /// messages of ERROR severity from VK_LAYER_KHRONOS_validation. Zero when
    /// validation is not running (see RenderDevice::IsValidationEnabled()).
    ///
    /// The messages themselves still go to stderr; this counter exists so a
    /// test can fail on a synchronization or usage error that would otherwise
    /// only scroll past in a log.
    [[nodiscard]] std::uint64_t ValidationErrorCount() noexcept;

    void ResetValidationErrorCount() noexcept;

    namespace detail {

        /// Called by backends from their validation callback. Not for other callers.
        void NoteValidationError() noexcept;

    } // namespace detail

} // namespace renderer
