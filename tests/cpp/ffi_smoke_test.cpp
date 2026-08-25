// Smoke test for the Rust <-> CMake bridge: calls into cv-ffi (which
// wraps ccore::BuildInfo) and checks a real, non-empty string comes
// back across the FFI boundary — proof the whole chain (Corrosion,
// cbindgen, static linking) actually works, not just that it configures.

#include <cstdio>
#include <cstring>

#include "ffi.h"

int main()
{
    char* info = ffi_build_info_string();
    if (info == nullptr)
    {
        std::fprintf(stderr, "cv_ffi_build_info_string() returned null\n");
        return 1;
    }

    std::printf("cv-ffi build info: %s\n", info);
    const bool ok = std::strlen(info) > 0;

    ffi_free_string(info);
    return ok ? 0 : 1;
}
