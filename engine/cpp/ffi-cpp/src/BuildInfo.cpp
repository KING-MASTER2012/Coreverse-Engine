#include "cv-ffi/BuildInfo.hpp"

#include "ffi.h"

namespace cv_ffi {

std::string BuildInfoString()
{
    // SAFETY: ffi_build_info_string is documented as always returning a
    // valid, non-null pointer.
    char* info = ffi_build_info_string();
    std::string result(info);
    // SAFETY: info was just returned by ffi_build_info_string above and has
    // not been freed yet.
    ffi_free_string(info);
    return result;
}

} // namespace cv_ffi
