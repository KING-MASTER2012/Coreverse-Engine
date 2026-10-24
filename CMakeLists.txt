cmake_minimum_required(VERSION 4.4)
# Single source of truth for the CMake minimum is
# devkit/scripts/setup/config/tool-versions.json (cmake.minVersion): the
# bootstrap scripts install/verify exactly that version, CMakePresets.json
# ("cmakeMinimumRequired") and this line must match it. When raising the
# minimum, change all three together. Declaring 4.4 as the minimum also
# switches every policy introduced up to 4.4 to its NEW behavior.
project(CoreverseEngine VERSION 0.1.0 LANGUAGES CXX C)

list(APPEND CMAKE_MODULE_PATH "${CMAKE_SOURCE_DIR}/cmake")

# ------------------------------------------------------------------
# Language standard
# ------------------------------------------------------------------
set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_C_STANDARD 17)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

# Fallback for anyone configuring outside the CMakePresets.json workflow
# (single-config generators only — multi-config generators like
# "Ninja Multi-Config" ignore CMAKE_BUILD_TYPE entirely).
if(NOT CMAKE_BUILD_TYPE AND NOT CMAKE_CONFIGURATION_TYPES)
    set(CMAKE_BUILD_TYPE "Debug" CACHE STRING "Build type" FORCE)
endif()

# ------------------------------------------------------------------
# Output directories — applied per-config so this works identically
# under single-config (Ninja) and multi-config (Ninja Multi-Config).
# ------------------------------------------------------------------
foreach(_cfg "" ${CMAKE_CONFIGURATION_TYPES})
    if(_cfg)
        string(TOUPPER "${_cfg}" _cfg_upper)
        set(_suffix "_${_cfg_upper}")
    else()
        set(_suffix "")
    endif()
    set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY${_suffix} ${CMAKE_BINARY_DIR}/lib)
    set(CMAKE_LIBRARY_OUTPUT_DIRECTORY${_suffix} ${CMAKE_BINARY_DIR}/lib)
    set(CMAKE_RUNTIME_OUTPUT_DIRECTORY${_suffix} ${CMAKE_BINARY_DIR}/bin)
endforeach()
unset(_cfg)
unset(_cfg_upper)
unset(_suffix)

# ------------------------------------------------------------------
# Ninja quality-of-life: keep colored diagnostics, use ccache/sccache
# for incremental rebuilds when available.
# ------------------------------------------------------------------
if(CMAKE_GENERATOR MATCHES "Ninja")
    if(CMAKE_CXX_COMPILER_ID MATCHES "Clang")
        add_compile_options(-fcolor-diagnostics)
    elseif(CMAKE_CXX_COMPILER_ID MATCHES "GNU")
        add_compile_options(-fdiagnostics-color=always)
    endif()
endif()
include(Ccache)

# ------------------------------------------------------------------
# Shared warning set, opt-in per target via
# target_link_libraries(<tgt> PRIVATE compiler_warnings)
# ------------------------------------------------------------------
include(CompilerWarnings)
add_library(compiler_warnings INTERFACE)
set_project_warnings(compiler_warnings)

include(StaticAnalysis)
include(ClangTools)

# ------------------------------------------------------------------
# Compiler-role reporting. COMPILER_ROLE is set per-preset in
# CMakePresets.json: PRIMARY = distribution compiler for this
# platform (MSVC/Windows, GCC/Linux, Apple Clang/macOS), SECONDARY =
# validation-only (clang-cl/Windows, Clang/Linux). Not set at all
# when configuring outside the preset workflow.
# ------------------------------------------------------------------
if(DEFINED COMPILER_ROLE AND COMPILER_ROLE STREQUAL "SECONDARY")
    message(STATUS "Coreverse: SECONDARY (validation-only) compiler build — do not package this output for distribution")
elseif(DEFINED COMPILER_ROLE AND COMPILER_ROLE STREQUAL "PRIMARY")
    message(STATUS "Coreverse: PRIMARY (distribution) compiler build")
endif()

# ------------------------------------------------------------------
# Rust <-> CMake bridge (Corrosion) and the ffi C ABI boundary.
# Only ffi is imported from the workspace — it's the single crate
# C++ is allowed to link against; every other Rust crate stays behind
# it and is pulled in transitively through Cargo, not through CMake.
# ------------------------------------------------------------------
include(FetchContent)
FetchContent_Declare(
        Corrosion
        GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
        GIT_TAG v0.6.1
)
FetchContent_MakeAvailable(Corrosion)

corrosion_import_crate(
        MANIFEST_PATH ${CMAKE_SOURCE_DIR}/Cargo.toml
        CRATES ffi
)

# Corrosion 0.6.1 hardcodes
#   INTERFACE_LINK_DIRECTORIES /Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/lib
# on every imported crate target on macOS. On CI runners that directory is
# the Command Line Tools SDK, whose .tbd stubs (newer format, e.g.
# "arm64e.x1-macos") the linker of the selected full Xcode cannot parse:
#   ld: tapi error: malformed file ... libSystem.B.tbd: unknown architecture
# Anything linked against `ffi` then fails. The compiler's own -isysroot
# already provides libSystem/libc++, so the extra -L (and the -rpath CMake
# derives from it) is dropped again here.
if(APPLE)
    foreach(_cv_rust_target ffi ffi-static ffi-shared)
        if(TARGET ${_cv_rust_target})
            get_target_property(_cv_link_dirs ${_cv_rust_target} INTERFACE_LINK_DIRECTORIES)
            if(_cv_link_dirs)
                list(FILTER _cv_link_dirs EXCLUDE REGEX "/Library/Developer/CommandLineTools(/|$)")
                set_property(TARGET ${_cv_rust_target} PROPERTY INTERFACE_LINK_DIRECTORIES "${_cv_link_dirs}")
            endif()
            unset(_cv_link_dirs)
        endif()
    endforeach()
    unset(_cv_rust_target)
endif()

include(FfiHeader)
generate_ffi_header()

# ------------------------------------------------------------------
# Build switches
# ------------------------------------------------------------------
option(BUILD_EDITOR   "Build the Qt editor"          ON)
option(BUILD_RENDERER "Build the renderer library"   ON)
option(BUILD_TESTS    "Build C++ tests"               OFF)

enable_testing()

# cv-math (header-only C++ mirror of the Rust `math` crate) has no dependencies,
# so it is always available to the renderer, the editor and the tests.
add_subdirectory(engine/cpp/math)

if(BUILD_RENDERER)
    add_subdirectory(engine/cpp/renderer)
endif()

# cv-ffi (C++ RAII wrappers over ffi.h — see engine/cpp/ffi-cpp/CMakeLists.txt)
# needs only the Rust `ffi` library, not Qt. It is built whenever the editor is
# (its consumer) and whenever the tests are, so its own tests (e.g.
# cv_ffi_platform_test) also run in a headless configuration with
# -DBUILD_EDITOR=OFF.
if(BUILD_EDITOR OR BUILD_TESTS)
    add_subdirectory(engine/cpp/ffi-cpp)
endif()

if(BUILD_EDITOR)
    add_subdirectory(engine/cpp/editor)
endif()

if(BUILD_TESTS)
    add_subdirectory(tests/cpp)
endif()
