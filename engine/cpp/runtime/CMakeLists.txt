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
# On Android it is a shared library for the NativeActivity (cv_android_main in
# src/android_entry.cpp) and on iOS an application bundle; see docs/src/runtime.md.

set(CV_RUNTIME_SOURCES src/RuntimeApp.cpp)
if(TARGET renderer::renderer)
    list(APPEND CV_RUNTIME_SOURCES src/RendererBridge.cpp)
endif()

if(ANDROID)
    # Android loads a shared library into the NativeActivity and has no main():
    # the entry is `android_main` in the Rust ffi library, which calls
    # `cv_android_main` from android_entry.cpp. The library name goes into the
    # manifest as `android.app.lib_name`.
    add_library(coreverse-runtime SHARED ${CV_RUNTIME_SOURCES} src/android_entry.cpp)
    # NativeActivity finds `ANativeActivity_onCreate` with dlsym. Nothing in the
    # program references it (it lives in a static library), so the linker would
    # drop it without this.
    target_link_options(coreverse-runtime PRIVATE -Wl,--undefined=ANativeActivity_onCreate)
    target_link_libraries(coreverse-runtime PRIVATE log android)
else()
    add_executable(coreverse-runtime ${CV_RUNTIME_SOURCES} src/main.cpp)
    if(IOS)
        # iOS runs an application bundle; main() hands the thread to UIKit
        # through ffi_platform_run and does not return.
        set_target_properties(coreverse-runtime PROPERTIES
                MACOSX_BUNDLE TRUE
                MACOSX_BUNDLE_GUI_IDENTIFIER "dev.coreverse.runtime"
                MACOSX_BUNDLE_BUNDLE_NAME "Coreverse"
                XCODE_ATTRIBUTE_PRODUCT_BUNDLE_IDENTIFIER "dev.coreverse.runtime")
    endif()
endif()

target_include_directories(coreverse-runtime PRIVATE src)
target_link_libraries(coreverse-runtime PRIVATE cv-ffi::cv-ffi compiler_warnings)

if(TARGET renderer::renderer)
    target_link_libraries(coreverse-runtime PRIVATE renderer::renderer)
    target_compile_definitions(coreverse-runtime PRIVATE CV_RUNTIME_RENDERER=1)
else()
    message(STATUS "coreverse-runtime: the renderer is not built, the runtime runs without presenting")
endif()

if(BUILD_TESTS AND NOT ANDROID AND NOT IOS)
    # The loop on the headless backend: no display, no GPU, so it runs on every
    # CI machine. Three frames, then a clean exit with code 0.
    add_test(NAME coreverse_runtime_headless
            COMMAND coreverse-runtime --headless --frames 3)
    set_tests_properties(coreverse_runtime_headless PROPERTIES LABELS "ffi")
endif()
