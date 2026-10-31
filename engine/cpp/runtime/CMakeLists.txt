# coreverse-runtime: the game-side executable skeleton (Phase 10).
#
# It owns no window code of its own: the Rust `platform` crate opens the
# window and runs the event loop, cv-ffi (cv_ffi::window::Run) hands the events
# to C++, and -- when the renderer is built -- the native window handle of a
# surface event goes straight into renderer::NativeWindowHandle to build the
# swapchain. Without the renderer (BUILD_RENDERER=OFF) it still runs the loop,
# which is what `--headless` and the ctest below need: no display, no GPU.
#
#   coreverse-runtime [--headless] [--frames N] [--trace] [--no-renderer]
#                     [--title TEXT]
#
# See docs/src/runtime.md.

add_executable(coreverse-runtime
        src/main.cpp
        src/RuntimeApp.cpp
)

target_include_directories(coreverse-runtime PRIVATE src)
target_link_libraries(coreverse-runtime PRIVATE cv-ffi::cv-ffi compiler_warnings)

if(TARGET renderer::renderer)
    target_sources(coreverse-runtime PRIVATE src/RendererBridge.cpp)
    target_link_libraries(coreverse-runtime PRIVATE renderer::renderer)
    target_compile_definitions(coreverse-runtime PRIVATE CV_RUNTIME_RENDERER=1)
else()
    message(STATUS "coreverse-runtime: the renderer is not built, the runtime runs without presenting")
endif()

if(BUILD_TESTS)
    # The loop on the headless backend: no display, no GPU, so it runs on every
    # CI machine. Three frames, then a clean exit with code 0.
    add_test(NAME coreverse_runtime_headless
            COMMAND coreverse-runtime --headless --frames 3)
    set_tests_properties(coreverse_runtime_headless PROPERTIES LABELS "ffi")
endif()
