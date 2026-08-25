# cmake/StaticAnalysis.cmake
# Hybrid static-analysis policy:
#   - If the primary/secondary compiler pair for the current platform
#     both ship a static analyzer, BOTH run (GCC -fanalyzer + clang-tidy
#     on Linux; MSVC /analyze + clang-tidy on Windows).
#   - Where only one side has an analyzer of its own (macOS: only
#     Apple Clang), clang-tidy alone runs — it is the project-wide
#     fallback because neither GCC nor MSVC provide a separate lint
#     tool of their own.
#
# Opt-in per target:
#   enable_static_analysis(<target>)
# Globally gated behind ENABLE_STATIC_ANALYSIS so normal dev builds
# stay fast; flip it on for CI / pre-merge analysis passes.

option(ENABLE_STATIC_ANALYSIS "Run compiler-native analyzer + clang-tidy on annotated targets" OFF)

find_program(CLANG_TIDY_PROGRAM NAMES clang-tidy)

function(enable_static_analysis target_name)
    if(NOT ENABLE_STATIC_ANALYSIS)
        return()
    endif()

    # --- Compiler-native analyzer half of the hybrid ---
    if(CMAKE_CXX_COMPILER_ID STREQUAL "GNU")
        target_compile_options(${target_name} PRIVATE -fanalyzer)
    elseif(MSVC AND NOT CMAKE_CXX_COMPILER_ID MATCHES "Clang")
        # Real cl.exe only — clang-cl reports MSVC=ON too, but it has
        # no /analyze implementation of its own; it gets clang-tidy below.
        target_compile_options(${target_name} PRIVATE /analyze)
    endif()

    # --- clang-tidy half — always attempted, this is the fallback tool ---
    if(CLANG_TIDY_PROGRAM)
        set_target_properties(${target_name} PROPERTIES
                CXX_CLANG_TIDY "${CLANG_TIDY_PROGRAM};--quiet"
        )
    else()
        message(STATUS "Coreverse: clang-tidy not found — static analysis for '${target_name}' will only use the native compiler analyzer, if any")
    endif()
endfunction()
