cmake_minimum_required(VERSION 3.25...4.4)
# The <min>...<max> form does NOT raise the real minimum: any CMake
# >= 3.25 can still configure this project. It tells CMake "if you're
# newer than 3.25, adopt every policy's NEW behavior up through 4.4"
# instead of silently keeping every post-3.25 policy on its old,
# deprecated default. Bump the <max> component when a new CMake
# minor/major ships and CI has confirmed it still configures cleanly.
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
foreach(_cv_cfg "" ${CMAKE_CONFIGURATION_TYPES})
    if(_cv_cfg)
        string(TOUPPER "${_cv_cfg}" _cv_cfg_upper)
        set(_cv_suffix "_${_cv_cfg_upper}")
    else()
        set(_cv_suffix "")
    endif()
    set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY${_cv_suffix} ${CMAKE_BINARY_DIR}/lib)
    set(CMAKE_LIBRARY_OUTPUT_DIRECTORY${_cv_suffix} ${CMAKE_BINARY_DIR}/lib)
    set(CMAKE_RUNTIME_OUTPUT_DIRECTORY${_cv_suffix} ${CMAKE_BINARY_DIR}/bin)
endforeach()
unset(_cv_cfg)
unset(_cv_cfg_upper)
unset(_cv_suffix)

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
# target_link_libraries(<tgt> PRIVATE cv_compiler_warnings)
# ------------------------------------------------------------------
include(CompilerWarnings)
add_library(cv_compiler_warnings INTERFACE)
cv_set_project_warnings(cv_compiler_warnings)

include(StaticAnalysis)
include(ClangTools)

# ------------------------------------------------------------------
# Compiler-role reporting. CV_COMPILER_ROLE is set per-preset in
# CMakePresets.json: PRIMARY = distribution compiler for this
# platform (MSVC/Windows, GCC/Linux, Apple Clang/macOS), SECONDARY =
# validation-only (clang-cl/Windows, Clang/Linux). Not set at all
# when configuring outside the preset workflow.
# ------------------------------------------------------------------
if(DEFINED CV_COMPILER_ROLE AND CV_COMPILER_ROLE STREQUAL "SECONDARY")
    message(STATUS "Coreverse: SECONDARY (validation-only) compiler build — do not package this output for distribution")
elseif(DEFINED CV_COMPILER_ROLE AND CV_COMPILER_ROLE STREQUAL "PRIMARY")
    message(STATUS "Coreverse: PRIMARY (distribution) compiler build")
endif()

# ------------------------------------------------------------------
# Build switches
# ------------------------------------------------------------------
option(CV_BUILD_EDITOR   "Build the Qt editor"          ON)
option(CV_BUILD_RENDERER "Build the renderer library"   ON)

enable_testing()

if(CV_BUILD_RENDERER)
    add_subdirectory(engine/cpp/renderer)
endif()

if(CV_BUILD_EDITOR)
    add_subdirectory(engine/cpp/editor)
endif()

# NOTE: engine/rust (Corrosion import) and tests/ (CTest wiring) are
# added in the next phases, once the cv-ffi crate and C++ targets exist.
