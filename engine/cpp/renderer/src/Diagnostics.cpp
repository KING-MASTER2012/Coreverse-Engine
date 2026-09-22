#include "renderer/Diagnostics.hpp"

#include <atomic>

namespace renderer {

namespace {

// Validation callbacks can arrive from any driver/layer thread.
std::atomic<std::uint64_t> g_validationErrors{0};

} // namespace

std::uint64_t ValidationErrorCount() noexcept
{
    return g_validationErrors.load(std::memory_order_relaxed);
}

void ResetValidationErrorCount() noexcept
{
    g_validationErrors.store(0, std::memory_order_relaxed);
}

namespace detail {

void NoteValidationError() noexcept
{
    g_validationErrors.fetch_add(1, std::memory_order_relaxed);
}

} // namespace detail

} // namespace renderer
