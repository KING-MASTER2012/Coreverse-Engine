# PROGRESS.md
Last changed 20.09.2026 by KING-MASTER2012.

## What happened compared to the previous commit?
- fixed: every executable linking `renderer` failed with unresolved externals (MSVC LNK2019 `vkCreateWin32SurfaceKHR`; the same on Linux/macOS). vcpkg's prebuilt `volk::volk` is compiled without the VK_USE_PLATFORM_* defines, so it has none of the platform surface functions. `renderer` now links the header-only `volk::volk_headers` and compiles volk's implementation itself (src/backend/vulkan/VolkImpl.cpp) with the platform defines it already sets.
- fixed: the Formatting CI job checked with Ubuntu's clang-format 18 while the project requires 22.1.8, so correctly formatted code was rejected. CI now installs the version from tool-versions.json and reports all violations at once. tests/cpp/ffi_vfs_test.cpp was really unformatted and is fixed.
- added: devkit/scripts/format.sh and devkit/scripts/format.ps1 (rustfmt + clang-format over the whole project, `--check` / `-Check` for a dry run).
- fixed: bootstrap.sh's vcpkg install used a doubled --x-install-root (`<root>/./<root>/vcpkg_installed`), so on Linux/macOS Bootstrap and the CMake presets never shared ./vcpkg_installed.
- changed: the macOS CI job installs its tools explicitly instead of running bootstrap.sh (see the comment in cpp.yml).

## What will be done?
- Phase 6.2: Qt native-handle bridge (ViewportWidget).
- Phase 6.3: renderer init/shutdown in the editor, swapchain resize/recreate.
- Phase 6.4: frame synchronization abstraction and a real render loop.
- Phase 7c: VFS FFI extensions (list_dir/metadata/remove), C++ RAII wrappers, renderer logging through the ffi logger.
- Debug bootstrap.sh on macOS (needs the Bootstrap step's log).
- Asset system will be added.
- IO system will be added.
