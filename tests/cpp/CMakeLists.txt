# ------------------------------------------------------------------
# ctest labels
#
# Every test carries exactly one of two labels so CI (and headless
# dev machines) can pick the subset they can actually run — see the
# "*-nogpu" presets in CMakePresets.json, which exclude `gpu`:
#
#   ffi  Pure Rust <-> C++ FFI tests. Need nothing but the built
#        binary.
#   gpu  Need a Vulkan ICD (a real GPU driver, or a software one such
#        as lavapipe) and, for the surface/swapchain/render-loop
#        tests, a display server (Win32, an X server such as Xvfb,
#        or — for the Wayland test — a compositor).
#
# The editor smoke test additionally carries `editor`: it also needs a Qt
# platform plugin that can open a window (on Linux, "xcb" — see the test
# below), which is more than the renderer tests need, so CI can run it
# separately (`ctest -L editor`) and hold it back from the main run
# (`ctest -LE editor`) while that setup is being proven out.
#
# Set CV_REQUIRE_VALIDATION=1 when running the gpu tests to make the ones
# that check for validation errors (frame_sync_test, editor_render_smoke)
# fail if the Vulkan validation layer is not actually running — without it
# they pass on a machine that has no layer installed, having checked nothing.
#
# The timeout keeps a wedged driver or a missing display from hanging
# a whole CI run: Acquire/Present can block indefinitely otherwise.
# ------------------------------------------------------------------
#   unit Pure C++ tests that need nothing at all (no Rust library, no GPU,
#        no display), e.g. the header-only cv-math library.
set(CV_FFI_TEST_PROPERTIES LABELS "ffi")
set(CV_UNIT_TEST_PROPERTIES LABELS "unit")
set(CV_GPU_TEST_PROPERTIES LABELS "gpu" TIMEOUT 120)

# cv-math vs. the Rust `math` crate: same layout and same results as
# tests/math_golden/golden.txt (written by the Rust test cpp_golden).
add_executable(math_layout_test math_layout_test.cpp)
target_link_libraries(math_layout_test PRIVATE cv-math::cv-math compiler_warnings)
target_compile_definitions(math_layout_test PRIVATE
        CV_MATH_GOLDEN_FILE="${CMAKE_SOURCE_DIR}/tests/math_golden/golden.txt")

add_test(NAME math_layout_test COMMAND math_layout_test)
set_tests_properties(math_layout_test PROPERTIES ${CV_UNIT_TEST_PROPERTIES})

# Proves the full pipeline end to end: ffi (Rust) -> Corrosion ->
# cbindgen-generated header -> a linked, running C++ executable.
add_executable(ffi_smoke_test ffi_smoke_test.cpp)
target_link_libraries(ffi_smoke_test PRIVATE compiler_warnings)
link_ffi(ffi_smoke_test)

add_test(NAME ffi_smoke_test COMMAND ffi_smoke_test)
set_tests_properties(ffi_smoke_test PROPERTIES ${CV_FFI_TEST_PROPERTIES})

# Phase 7a proof: log-core/log-sinks FFI surface -- opaque Logger and
# DiagnosticBuilder handles, console + file sinks, runtime producer
# registration.
add_executable(ffi_logger_test ffi_logger_test.cpp)
target_link_libraries(ffi_logger_test PRIVATE compiler_warnings)
link_ffi(ffi_logger_test)

add_test(NAME ffi_logger_test COMMAND ffi_logger_test)
set_tests_properties(ffi_logger_test PROPERTIES ${CV_FFI_TEST_PROPERTIES})

# Phase 7b proof: vfs FFI surface -- ffi_vfs_init() plus
# read_bytes/read_to_string/write_bytes/exists against the global
# VfsContext, and the thread-local last-error mechanism.
add_executable(ffi_vfs_test ffi_vfs_test.cpp)
target_link_libraries(ffi_vfs_test PRIVATE compiler_warnings)
link_ffi(ffi_vfs_test)

add_test(NAME ffi_vfs_test COMMAND ffi_vfs_test)
set_tests_properties(ffi_vfs_test PROPERTIES ${CV_FFI_TEST_PROPERTIES})

# Phase 5 (platform) proof: cv-ffi's platform wrappers -- cv_ffi::platform::Info()
# and StandardDir() -- against the real Rust library, including the
# failure paths and the thread-local last-error message.
add_executable(cv_ffi_platform_test cv_ffi_platform_test.cpp)
target_link_libraries(cv_ffi_platform_test PRIVATE cv-ffi::cv-ffi compiler_warnings)

add_test(NAME cv_ffi_platform_test COMMAND cv_ffi_platform_test)
set_tests_properties(cv_ffi_platform_test PROPERTIES ${CV_FFI_TEST_PROPERTIES})

# Phase 10 (runtime integration) proof: cv-ffi's window event loop --
# cv_ffi::window::Run() and the Context -- against the real Rust library on
# the headless backend (no display needed): callback order, exit codes, the
# stale-context guard, an exception leaving a callback, and the key tables.
add_executable(cv_ffi_window_test cv_ffi_window_test.cpp)
target_link_libraries(cv_ffi_window_test PRIVATE cv-ffi::cv-ffi compiler_warnings)

add_test(NAME cv_ffi_window_test COMMAND cv_ffi_window_test)
set_tests_properties(cv_ffi_window_test PROPERTIES ${CV_FFI_TEST_PROPERTIES})

# Everything below needs the renderer library (and a Vulkan ICD at run time),
# so it is skipped when the renderer is not built (-DBUILD_RENDERER=OFF).
if(BUILD_RENDERER)

    # Phase 5.1 proof: RenderDevice abstraction -> VulkanRenderDevice bring-up
    # (instance/device/queues), selected device name logged, clean teardown.
    add_executable(renderer_device_test renderer_device_test.cpp)
    target_link_libraries(renderer_device_test PRIVATE compiler_warnings renderer::renderer)
    add_test(NAME renderer_device_test COMMAND renderer_device_test)
    set_tests_properties(renderer_device_test PROPERTIES ${CV_GPU_TEST_PROPERTIES})

    # Phase 5.2 proof: RenderDevice::CreateBuffer() round-trip through VMA,
    # incl. move semantics, entirely via the Buffer/RenderDevice abstraction.
    add_executable(buffer_roundtrip_test buffer_roundtrip_test.cpp)
    target_link_libraries(buffer_roundtrip_test PRIVATE compiler_warnings renderer::renderer)
    add_test(NAME buffer_roundtrip_test COMMAND buffer_roundtrip_test)
    set_tests_properties(buffer_roundtrip_test PROPERTIES ${CV_GPU_TEST_PROPERTIES})

    # Phase 5.3 proof: RenderDevice::CreateSurface() from a real native window
    # handle. Uses a dummy window this test creates itself (Win32/Xlib),
    # not Phase 6's Qt window.
    add_executable(surface_test surface_test.cpp)
    target_link_libraries(surface_test PRIVATE compiler_warnings renderer::renderer)
    if(WIN32)
        target_link_libraries(surface_test PRIVATE user32)
    elseif(UNIX AND NOT APPLE)
        find_package(X11 REQUIRED)
        target_link_libraries(surface_test PRIVATE X11::X11)
    endif()
    add_test(NAME surface_test COMMAND surface_test)
    set_tests_properties(surface_test PROPERTIES ${CV_GPU_TEST_PROPERTIES})

    # Phase 5.4 proof: RenderDevice::CreateSwapchain() built from a real
    # Surface's capabilities — image count logged, clean teardown.
    add_executable(swapchain_test swapchain_test.cpp)
    target_link_libraries(swapchain_test PRIVATE compiler_warnings renderer::renderer)
    if(WIN32)
        target_link_libraries(swapchain_test PRIVATE user32)
    elseif(UNIX AND NOT APPLE)
        find_package(X11 REQUIRED)
        target_link_libraries(swapchain_test PRIVATE X11::X11)
    endif()
    add_test(NAME swapchain_test COMMAND swapchain_test)
    set_tests_properties(swapchain_test PROPERTIES ${CV_GPU_TEST_PROPERTIES})

    # Phase 5.5 proof: acquire -> record a clear-color command buffer -> submit
    # -> present, entirely through the RenderDevice/Swapchain/CommandBuffer
    # abstraction. Single-shot (one frame, then closes), matching the plan.
    add_executable(render_loop_test render_loop_test.cpp)
    target_link_libraries(render_loop_test PRIVATE compiler_warnings renderer::renderer)
    if(WIN32)
        target_link_libraries(render_loop_test PRIVATE user32)
    elseif(UNIX AND NOT APPLE)
        find_package(X11 REQUIRED)
        target_link_libraries(render_loop_test PRIVATE X11::X11)
    endif()
    add_test(NAME render_loop_test COMMAND render_loop_test)
    set_tests_properties(render_loop_test PROPERTIES ${CV_GPU_TEST_PROPERTIES})

    # Phase 6.4 proof: FrameSync keeps 1, 2 and more-than-swapchain-images frames
    # in flight through hundreds of acquire/record/submit/present rounds with a
    # window resize in the middle, and fails on any validation-layer error
    # (renderer::ValidationErrorCount()). Also covers Swapchain::Acquire()'s
    # timeout (SwapchainStatus::NotReady) and FrameSync's misuse checks.
    add_executable(frame_sync_test frame_sync_test.cpp)
    target_link_libraries(frame_sync_test PRIVATE compiler_warnings renderer::renderer)
    if(WIN32)
        target_link_libraries(frame_sync_test PRIVATE user32)
    elseif(UNIX AND NOT APPLE)
        find_package(X11 REQUIRED)
        target_link_libraries(frame_sync_test PRIVATE X11::X11)
    endif()
    add_test(NAME frame_sync_test COMMAND frame_sync_test)
    set_tests_properties(frame_sync_test PROPERTIES ${CV_GPU_TEST_PROPERTIES})

    # Graphics pass proof (Phase 10): depth-buffered swapchain, graphics pipeline from the built-in
    # colored-mesh shaders, vertex/index buffers, cv-math MVP via push constants, resize and
    # device-loss recovery; the validation layer must stay silent. Skips itself (exit 0) on a
    # device without Vulkan 1.3 dynamic rendering.
    add_executable(graphics_pass_test graphics_pass_test.cpp)
    target_link_libraries(graphics_pass_test PRIVATE compiler_warnings renderer::renderer cv-math::cv-math)
    if(WIN32)
        target_link_libraries(graphics_pass_test PRIVATE user32)
    elseif(UNIX AND NOT APPLE)
        find_package(X11 REQUIRED)
        target_link_libraries(graphics_pass_test PRIVATE X11::X11)
    endif()
    add_test(NAME graphics_pass_test COMMAND graphics_pass_test)
    set_tests_properties(graphics_pass_test PROPERTIES ${CV_GPU_TEST_PROPERTIES})

    # Phase 5 (Wayland milestone) proof: the same single-shot render loop as
    # render_loop_test.cpp above, but through a real wl_surface instead of
    # Xlib. Only built when Wayland is actually compiled into `renderer`
    # (RENDERER_ENABLE_WAYLAND, see renderer/CMakeLists.txt) — Wayland has
    # no Win32/macOS equivalent, unlike the Xlib tests above. Needs a real
    # libwayland-client (unlike the renderer itself, which only needs
    # vulkan_wayland.h's own forward-declared wl_display/wl_surface — see
    # that file's comments) because this test actually calls into it to
    # connect to a compositor and create a surface.
    #
    # The target keeps the name `wayland_render_loop_test` (that's what the
    # CI workflows and run-with-weston-headless.sh refer to), while the
    # source file is named wayland_renderer_loop_test.cpp.
    if(UNIX AND NOT APPLE AND RENDERER_ENABLE_WAYLAND)
        find_package(PkgConfig REQUIRED)
        pkg_check_modules(WaylandClient REQUIRED IMPORTED_TARGET wayland-client)

        add_executable(wayland_render_loop_test wayland_renderer_loop_test.cpp)
        target_link_libraries(wayland_render_loop_test PRIVATE compiler_warnings renderer::renderer PkgConfig::WaylandClient)

        # wl_display_connect() needs an actual compositor listening on
        # WAYLAND_DISPLAY — there's no display-less equivalent of Xlib's
        # "just open the existing X server" trick above.
        # run-with-weston-headless.sh (next to this file) spins up a
        # throwaway headless Weston instance around the test binary for
        # exactly that reason; running the binary directly (like every
        # other add_test() here) would just fail wherever no compositor
        # happens to already be running.
        add_test(NAME wayland_render_loop_test
                COMMAND bash "${CMAKE_CURRENT_SOURCE_DIR}/run-with-weston-headless.sh"
                "$<TARGET_FILE:wayland_render_loop_test>")
        set_tests_properties(wayland_render_loop_test PROPERTIES LABELS "gpu;wayland" TIMEOUT 120)
    endif()

    # Phase 6.4 proof, through the real Qt window: the editor renders 120 frames
    # into its viewport, is resized half-way (the swapchain must follow), quits by
    # itself and finds no validation errors after tearing everything down — see
    # EditorWindow::enableSmokeTest(). Only exists when the editor is built.
    #
    # Linux runs it on Qt's "xcb" platform (X11; under Xvfb in CI), which is the
    # only platform the editor's native-window bridge supports there. That needs
    # the vcpkg Qt build to ship the xcb platform plugin and the system to have
    # its runtime libraries (libxcb-cursor0, ...): if either is missing this is
    # the test that says so.
    if(TARGET editor)
        add_test(NAME editor_render_smoke COMMAND editor --smoke-frames 120)
        set_tests_properties(editor_render_smoke PROPERTIES LABELS "gpu;editor" TIMEOUT 120)
        if(UNIX AND NOT APPLE)
            set_property(TEST editor_render_smoke APPEND PROPERTY ENVIRONMENT "QT_QPA_PLATFORM=xcb")
        endif()

        # Same, but a quarter of the way through the device is made to report DeviceLost (as after
        # a driver reset): the renderer must rebuild everything on a new device exactly once and
        # keep rendering to the end.
        add_test(NAME editor_device_loss_smoke COMMAND editor --smoke-frames 120 --simulate-device-loss)
        set_tests_properties(editor_device_loss_smoke PROPERTIES LABELS "gpu;editor" TIMEOUT 120)
        if(UNIX AND NOT APPLE)
            set_property(TEST editor_device_loss_smoke APPEND PROPERTY ENVIRONMENT "QT_QPA_PLATFORM=xcb")
        endif()
    endif()

endif()
