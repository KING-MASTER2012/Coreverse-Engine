# PROGRESS.md
Last changed 19.09.2026 by KING-MASTER2012.

## What happened compared to the previous commit?
- fixed: tests/cpp/CMakeLists.txt referenced a Wayland test source and a helper script path that don't exist, so configuring with BUILD_TESTS=ON failed on Linux.
- fixed: ffi.h is now regenerated when any ffi source changes (was: only lib.rs), and when a new module file is added.
- added: ctest labels (`ffi`, `gpu`) and CMake test presets, incl. `*-nogpu` variants for machines without a Vulkan device.
- changed: CI configures/builds/tests through the CMake presets and runs ctest (Linux: software Vulkan + Xvfb + headless Weston; Windows: ffi tests only). Experimental macOS job added.
- changed: the presets now share ./vcpkg_installed with Bootstrap instead of installing the vcpkg packages a second time.

## What will be done?
- Phase 6.2: Qt native-handle bridge (ViewportWidget).
- Phase 6.3: renderer init/shutdown in the editor, swapchain resize/recreate.
- Phase 6.4: frame synchronization abstraction and a real render loop.
- Phase 7c: VFS FFI extensions (list_dir/metadata/remove), C++ RAII wrappers, renderer logging through the ffi logger.
- Asset system will be added.
- IO system will be added.
