# cmake/Ccache.cmake
# Transparently speeds up incremental Ninja rebuilds by wiring ccache/sccache
# in as a compiler launcher, if either is present on the system. No-op otherwise.

find_program(CV_CCACHE_PROGRAM NAMES sccache ccache)

if(CV_CCACHE_PROGRAM)
    message(STATUS "Coreverse: using '${CV_CCACHE_PROGRAM}' as compiler launcher")
    set(CMAKE_C_COMPILER_LAUNCHER   "${CV_CCACHE_PROGRAM}" CACHE STRING "" FORCE)
    set(CMAKE_CXX_COMPILER_LAUNCHER "${CV_CCACHE_PROGRAM}" CACHE STRING "" FORCE)
else()
    message(STATUS "Coreverse: no ccache/sccache found, compiling without a cache")
endif()
