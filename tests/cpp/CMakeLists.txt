# Proves the full pipeline end to end: ffi (Rust) -> Corrosion ->
# cbindgen-generated header -> a linked, running C++ executable.
add_executable(ffi_smoke_test ffi_smoke_test.cpp)
target_link_libraries(ffi_smoke_test PRIVATE compiler_warnings)
link_ffi(ffi_smoke_test)

add_test(NAME ffi_smoke_test COMMAND ffi_smoke_test)

# Faz 5.1 proof: RenderDevice abstraction -> VulkanRenderDevice bring-up
# (instance/device/queues), selected device name logged, clean teardown.
add_executable(renderer_device_test renderer_device_test.cpp)
target_link_libraries(renderer_device_test PRIVATE compiler_warnings renderer::renderer)
add_test(NAME renderer_device_test COMMAND renderer_device_test)

# Faz 5.2 proof: RenderDevice::CreateBuffer() round-trip through VMA,
# incl. move semantics, entirely via the Buffer/RenderDevice abstraction.
add_executable(buffer_roundtrip_test buffer_roundtrip_test.cpp)
target_link_libraries(buffer_roundtrip_test PRIVATE compiler_warnings renderer::renderer)
add_test(NAME buffer_roundtrip_test COMMAND buffer_roundtrip_test)

# Faz 5.3 proof: RenderDevice::CreateSurface() from a real native window
# handle. Uses a dummy window this test creates itself (Win32/Xlib),
# not Faz 6's Qt window.
add_executable(surface_test surface_test.cpp)
target_link_libraries(surface_test PRIVATE compiler_warnings renderer::renderer)
if(WIN32)
    target_link_libraries(surface_test PRIVATE user32)
elseif(UNIX AND NOT APPLE)
    find_package(X11 REQUIRED)
    target_link_libraries(surface_test PRIVATE X11::X11)
endif()
add_test(NAME surface_test COMMAND surface_test)

# Faz 5.4 proof: RenderDevice::CreateSwapchain() built from a real
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

# Faz 5.5 proof: acquire -> record a clear-color command buffer -> submit
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
