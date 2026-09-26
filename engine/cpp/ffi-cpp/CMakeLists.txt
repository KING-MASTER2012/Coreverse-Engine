# cv-ffi: C++ RAII wrappers over the `ffi` crate's raw C ABI
# (engine/rust/crates/ffi, generated header at ${FFI_INCLUDE_DIR}/ffi.h —
# see cmake/FfiHeader.cmake).
#
# Every ffi_* function that hands out an owned resource (a Logger*, a
# DiagnosticBuilder*, a heap string/byte buffer) has a matching free/destroy
# call that must run exactly once. This library is that bookkeeping, done
# once here instead of in every C++ caller: Logger/DiagnosticBuilder are
# move-only RAII types (same shape as renderer::Buffer/Swapchain — see
# renderer/include/renderer/Buffer.hpp), and the free functions in
# cv_ffi::vfs and cv_ffi::BuildInfoString() convert a raw ffi_vfs_*/
# ffi_build_info_string() call plus its matching free into one
# std::expected-returning call.
#
# ffi.h itself is deliberately NOT included by any header under include/ —
# callers of this library never see the raw C ABI, only cv_ffi's own types
# (Logger, DiagnosticBuilder, cv_ffi::vfs::Root, ...); only the .cpp files
# here include it.

add_library(cv-ffi STATIC
        src/BuildInfo.cpp
        src/Logger.cpp
        src/Diagnostic.cpp
        src/Vfs.cpp
)

add_library(cv-ffi::cv-ffi ALIAS cv-ffi)

target_include_directories(cv-ffi PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)

target_compile_features(cv-ffi PUBLIC cxx_std_23)

target_link_libraries(cv-ffi PRIVATE compiler_warnings)

# Wires up ffi.h (cbindgen) and the Rust staticlib — see cmake/FfiHeader.cmake.
# PRIVATE: ffi.h is only used from the .cpp files above (see the file
# comment), so it does not need to propagate to cv-ffi's own consumers.
link_ffi(cv-ffi)
