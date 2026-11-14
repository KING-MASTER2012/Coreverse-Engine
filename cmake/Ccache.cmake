# cmake/Ccache.cmake
# Transparently speeds up (incremental or CI) rebuilds by wiring ccache/sccache
# in as a compiler launcher. No-op when none is available in "auto" mode.
#
# COREVERSE_COMPILER_LAUNCHER (cache variable; its default is taken from the
# environment variable of the same name, which is how the CI selects a tool
# without touching the presets):
#   auto     the default: sccache if it is on PATH, else ccache, else nothing
#   ccache   use ccache; configuring fails if it is not on PATH
#   sccache  use sccache; configuring fails if it is not on PATH
#   none     no launcher. Needed where the compiler has to be seen running
#            (CodeQL's build tracing does not see a compile that was a cache hit)
#
# A launcher that was set explicitly (-DCMAKE_CXX_COMPILER_LAUNCHER=..., a
# preset) is never overridden. A later configure can tell "set by this file"
# from "set by the user" through _COREVERSE_APPLIED_LAUNCHER.

set(_cv_launcher_default "auto")
if(NOT "$ENV{COREVERSE_COMPILER_LAUNCHER}" STREQUAL "")
    set(_cv_launcher_default "$ENV{COREVERSE_COMPILER_LAUNCHER}")
endif()
set(COREVERSE_COMPILER_LAUNCHER "${_cv_launcher_default}" CACHE STRING
        "Compiler launcher: auto (sccache, else ccache, else none) | ccache | sccache | none")
set_property(CACHE COREVERSE_COMPILER_LAUNCHER PROPERTY STRINGS auto ccache sccache none)
unset(_cv_launcher_default)

set(_cv_user_set FALSE)
foreach(_cv_launcher_var CMAKE_C_COMPILER_LAUNCHER CMAKE_CXX_COMPILER_LAUNCHER)
    if(${_cv_launcher_var} AND NOT "${${_cv_launcher_var}}" STREQUAL "${_COREVERSE_APPLIED_LAUNCHER}")
        set(_cv_user_set TRUE)
    endif()
endforeach()
unset(_cv_launcher_var)
if(_cv_user_set)
    message(STATUS "Coreverse: compiler launcher was set explicitly, leaving it alone")
    unset(_cv_user_set)
    return()
endif()
unset(_cv_user_set)

set(_cv_launcher_program "")
if(COREVERSE_COMPILER_LAUNCHER STREQUAL "none")
    # Deliberately no launcher.
elseif(COREVERSE_COMPILER_LAUNCHER STREQUAL "auto")
    unset(CV_LAUNCHER_PROGRAM CACHE)
    find_program(CV_LAUNCHER_PROGRAM NAMES sccache ccache)
    if(CV_LAUNCHER_PROGRAM)
        set(_cv_launcher_program "${CV_LAUNCHER_PROGRAM}")
    endif()
elseif(COREVERSE_COMPILER_LAUNCHER STREQUAL "ccache" OR COREVERSE_COMPILER_LAUNCHER STREQUAL "sccache")
    unset(CV_LAUNCHER_PROGRAM CACHE)
    find_program(CV_LAUNCHER_PROGRAM NAMES ${COREVERSE_COMPILER_LAUNCHER})
    if(NOT CV_LAUNCHER_PROGRAM)
        message(FATAL_ERROR
                "Coreverse: COREVERSE_COMPILER_LAUNCHER=${COREVERSE_COMPILER_LAUNCHER} "
                "but '${COREVERSE_COMPILER_LAUNCHER}' was not found on PATH.")
    endif()
    set(_cv_launcher_program "${CV_LAUNCHER_PROGRAM}")
else()
    message(FATAL_ERROR
            "Coreverse: COREVERSE_COMPILER_LAUNCHER must be auto, ccache, sccache or none "
            "(got '${COREVERSE_COMPILER_LAUNCHER}').")
endif()

if(_cv_launcher_program)
    message(STATUS "Coreverse: using '${_cv_launcher_program}' as compiler launcher")
    set(CMAKE_C_COMPILER_LAUNCHER   "${_cv_launcher_program}" CACHE STRING "" FORCE)
    set(CMAKE_CXX_COMPILER_LAUNCHER "${_cv_launcher_program}" CACHE STRING "" FORCE)
    set(_COREVERSE_APPLIED_LAUNCHER "${_cv_launcher_program}" CACHE INTERNAL "")
else()
    if(_COREVERSE_APPLIED_LAUNCHER)
        # A previous configure of this build directory set one; take it back.
        set(CMAKE_C_COMPILER_LAUNCHER   "" CACHE STRING "" FORCE)
        set(CMAKE_CXX_COMPILER_LAUNCHER "" CACHE STRING "" FORCE)
        set(_COREVERSE_APPLIED_LAUNCHER "" CACHE INTERNAL "")
    endif()
    if(COREVERSE_COMPILER_LAUNCHER STREQUAL "none")
        message(STATUS "Coreverse: compiler launcher disabled (COREVERSE_COMPILER_LAUNCHER=none)")
    else()
        message(STATUS "Coreverse: no ccache/sccache found, compiling without a cache")
    endif()
endif()
unset(_cv_launcher_program)
