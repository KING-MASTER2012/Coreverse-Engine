# cmake/CompilerWarnings.cmake
# Strict, modern warning set applied via an INTERFACE target
# (engine/cpp targets should `target_link_libraries(<tgt> PRIVATE cv_compiler_warnings)`).
#
# Six toolchains are in play across the project's compiler matrix:
#   - MSVC (cl.exe)                    -> real MSVC, MSVC_FRONTEND
#   - clang-cl (Windows, secondary)    -> COMPILER_ID=Clang, FRONTEND_VARIANT=MSVC
#   - clang/clang++ GNU driver         -> COMPILER_ID=Clang, FRONTEND_VARIANT=GNU
#     (Windows-Clang-GNU/mingw, Linux secondary)
#   - GCC (Linux primary)              -> COMPILER_ID=GNU
#   - Apple Clang (macOS, only option) -> COMPILER_ID=AppleClang, FRONTEND_VARIANT=GNU
#
# CMake sets MSVC=ON for BOTH real cl.exe and clang-cl (they share the
# same command-line flag surface), so clang-cl must be routed to the
# MSVC-style flag list, not the GNU-style one, even though its
# COMPILER_ID is "Clang".

function(cv_set_project_warnings target_name)
    set(MSVC_STYLE_WARNINGS
            /W4
            /permissive-
            /w14242 /w14254 /w14263 /w14265 /w14287
            /we4289 /w14296 /w14311 /w14545 /w14546
            /w14547 /w14549 /w14555 /w14619 /w14640
            /w14826 /w14905 /w14906 /w14928
    )

    set(GNU_STYLE_WARNINGS
            -Wall -Wextra -Wpedantic
            -Wshadow -Wnon-virtual-dtor -Wold-style-cast
            -Wcast-align -Wunused -Woverloaded-virtual
            -Wconversion -Wsign-conversion -Wnull-dereference
            -Wdouble-promotion -Wformat=2 -Wimplicit-fallthrough
    )

    set(_cv_is_msvc_frontend FALSE)
    if(MSVC)
        # Covers real cl.exe. Also covers clang-cl, EXCEPT we still
        # want to give it a couple of Clang-only diagnostics on top,
        # so we special-case it below instead of short-circuiting here.
        set(_cv_is_msvc_frontend TRUE)
    endif()

    if(_cv_is_msvc_frontend)
        target_compile_options(${target_name} INTERFACE ${MSVC_STYLE_WARNINGS})
        if(CMAKE_CXX_COMPILER_ID MATCHES "Clang")
            # clang-cl: layer on a couple of Clang-native checks that
            # have no MSVC equivalent, using MSVC-style flag passthrough.
            target_compile_options(${target_name} INTERFACE
                    -Wthread-safety
                    -Wno-unknown-warning-option
            )
        endif()
    elseif(CMAKE_CXX_COMPILER_ID MATCHES "Clang|GNU")
        # Plain clang/clang++ (GNU driver), GCC, and Apple Clang all
        # accept the same GNU-style flag syntax.
        target_compile_options(${target_name} INTERFACE ${GNU_STYLE_WARNINGS})
    endif()
endfunction()
