# cmake/ClangTools.cmake
# Style (clang-format) and lint (clang-tidy, as a standalone report
# target separate from the per-target CXX_CLANG_TIDY wiring in
# StaticAnalysis.cmake) — both always sourced from the Clang toolchain
# regardless of which compiler is primary on the host platform, since
# neither GCC nor MSVC ship an equivalent formatter or linter.
#
# Targets added (only if the underlying tool is found):
#   format        - reformat all tracked C++ sources in place
#   format-check  - fail if any tracked C++ source is not formatted
#   tidy          - run clang-tidy over compile_commands.json and report

find_program(CV_CLANG_FORMAT_PROGRAM NAMES clang-format)

if(NOT CV_CLANG_TIDY_PROGRAM)
    find_program(CV_CLANG_TIDY_PROGRAM NAMES clang-tidy)
endif()

file(GLOB_RECURSE CV_CXX_SOURCES
        CONFIGURE_DEPENDS
        ${CMAKE_SOURCE_DIR}/engine/cpp/*.cpp
        ${CMAKE_SOURCE_DIR}/engine/cpp/*.h
        ${CMAKE_SOURCE_DIR}/engine/cpp/*.hpp
)

if(CV_CLANG_FORMAT_PROGRAM AND CV_CXX_SOURCES)
    add_custom_target(format
            COMMAND ${CV_CLANG_FORMAT_PROGRAM} -i ${CV_CXX_SOURCES}
            WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
            COMMENT "Coreverse: formatting C++ sources with clang-format"
            VERBATIM
    )
    add_custom_target(format-check
            COMMAND ${CV_CLANG_FORMAT_PROGRAM} --dry-run --Werror ${CV_CXX_SOURCES}
            WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
            COMMENT "Coreverse: checking C++ source formatting"
            VERBATIM
    )
elseif(NOT CV_CLANG_FORMAT_PROGRAM)
    message(STATUS "Coreverse: clang-format not found — 'format' / 'format-check' targets unavailable")
endif()

if(CV_CLANG_TIDY_PROGRAM AND CV_CXX_SOURCES)
    add_custom_target(tidy
            COMMAND ${CV_CLANG_TIDY_PROGRAM} -p ${CMAKE_BINARY_DIR} ${CV_CXX_SOURCES}
            WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
            COMMENT "Coreverse: running clang-tidy report over all C++ sources"
            VERBATIM
    )
elseif(NOT CV_CLANG_TIDY_PROGRAM)
    message(STATUS "Coreverse: clang-tidy not found — 'tidy' target unavailable")
endif()
