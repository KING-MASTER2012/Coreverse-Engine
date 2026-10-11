# cmake/ClangTools.cmake
# Style (clang-format) and lint (clang-tidy, as a standalone report
# target separate from the per-target CXX_CLANG_TIDY wiring in
# StaticAnalysis.cmake) — both always sourced from the Clang toolchain
# regardless of which compiler is primary on the host platform, since
# neither GCC nor MSVC ship an equivalent formatter or linter.
#
# Targets added (only if the underlying tool is found):
#   format        - reformat all tracked C++ / Objective-C++ sources in place
#   format-check  - fail if any tracked C++ / Objective-C++ source is not formatted
#   tidy          - run clang-tidy over compile_commands.json and report
#
# File sets (kept identical to devkit/scripts/format.sh|ps1 and the CI
# "Formatting" workflow, which also cover tests/cpp):
#   FORMAT_SOURCES - engine/cpp + tests/cpp: *.cpp *.cc *.cxx *.h *.hpp *.hxx
#                    and the Objective-C++ files *.mm / *.m (.clang-format has
#                    a Cpp and an ObjC section).
#   TIDY_SOURCES   - the translation units only (no headers; they are reached
#                    through HeaderFilterRegex). The macOS-only *.mm files are
#                    included on Apple hosts only: on other hosts they are not
#                    in compile_commands.json and clang-tidy could not parse
#                    their Cocoa headers.

find_program(CLANG_FORMAT_PROGRAM NAMES clang-format)

if(NOT CLANG_TIDY_PROGRAM)
    find_program(CLANG_TIDY_PROGRAM NAMES clang-tidy)
endif()

file(GLOB_RECURSE FORMAT_SOURCES
        CONFIGURE_DEPENDS
        ${CMAKE_SOURCE_DIR}/engine/cpp/*.cpp
        ${CMAKE_SOURCE_DIR}/engine/cpp/*.cc
        ${CMAKE_SOURCE_DIR}/engine/cpp/*.cxx
        ${CMAKE_SOURCE_DIR}/engine/cpp/*.h
        ${CMAKE_SOURCE_DIR}/engine/cpp/*.hpp
        ${CMAKE_SOURCE_DIR}/engine/cpp/*.hxx
        ${CMAKE_SOURCE_DIR}/engine/cpp/*.mm
        ${CMAKE_SOURCE_DIR}/engine/cpp/*.m
        ${CMAKE_SOURCE_DIR}/tests/cpp/*.cpp
        ${CMAKE_SOURCE_DIR}/tests/cpp/*.cc
        ${CMAKE_SOURCE_DIR}/tests/cpp/*.cxx
        ${CMAKE_SOURCE_DIR}/tests/cpp/*.h
        ${CMAKE_SOURCE_DIR}/tests/cpp/*.hpp
        ${CMAKE_SOURCE_DIR}/tests/cpp/*.hxx
        ${CMAKE_SOURCE_DIR}/tests/cpp/*.mm
        ${CMAKE_SOURCE_DIR}/tests/cpp/*.m
)

file(GLOB_RECURSE TIDY_SOURCES
        CONFIGURE_DEPENDS
        ${CMAKE_SOURCE_DIR}/engine/cpp/*.cpp
        ${CMAKE_SOURCE_DIR}/engine/cpp/*.cc
        ${CMAKE_SOURCE_DIR}/engine/cpp/*.cxx
)
if(APPLE)
    file(GLOB_RECURSE TIDY_OBJC_SOURCES
            CONFIGURE_DEPENDS
            ${CMAKE_SOURCE_DIR}/engine/cpp/*.mm
    )
    list(APPEND TIDY_SOURCES ${TIDY_OBJC_SOURCES})
endif()

if(CLANG_FORMAT_PROGRAM AND FORMAT_SOURCES)
    add_custom_target(format
            COMMAND ${CLANG_FORMAT_PROGRAM} -i ${FORMAT_SOURCES}
            WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
            COMMENT "Coreverse: formatting C++ / Objective-C++ sources with clang-format"
            VERBATIM
    )
    add_custom_target(format-check
            COMMAND ${CLANG_FORMAT_PROGRAM} --dry-run --Werror ${FORMAT_SOURCES}
            WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
            COMMENT "Coreverse: checking C++ / Objective-C++ source formatting"
            VERBATIM
    )
elseif(NOT CLANG_FORMAT_PROGRAM)
    message(STATUS "Coreverse: clang-format not found — 'format' / 'format-check' targets unavailable")
endif()

if(CLANG_TIDY_PROGRAM AND TIDY_SOURCES)
    add_custom_target(tidy
            COMMAND ${CLANG_TIDY_PROGRAM} -p ${CMAKE_BINARY_DIR} ${TIDY_SOURCES}
            WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
            COMMENT "Coreverse: running clang-tidy report over all C++ sources"
            VERBATIM
    )
elseif(NOT CLANG_TIDY_PROGRAM)
    message(STATUS "Coreverse: clang-tidy not found — 'tidy' target unavailable")
endif()
