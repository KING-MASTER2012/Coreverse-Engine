# editor: Qt6 executable.
#
# Phase 6.1 brought up a bare QApplication + QMainWindow and proved the
# Qt6/CMake/FFI chain end to end (window title from a real
# ffi_build_info_string() call), deliberately WITHOUT linking `renderer`,
# to isolate failure domains one at a time (Qt/CMake -> native window
# bridge -> FFI -> Vulkan surface) instead of debugging all four at once.
#
# Phase 6.2 adds the native-window bridge (NativeWindowBridge, one
# implementation per platform) and the viewport widget it describes;
# Phase 6.3 links `renderer` and lets ViewportRenderer own the device,
# surface and swapchain for that widget.
#
# Phase 7c moves every direct ffi.h call (ffi_build_info_string, the
# Logger/DiagnosticBuilder/VFS calls) behind cv-ffi (engine/cpp/ffi-cpp) --
# this target links that instead of calling link_ffi(editor)/including
# ffi.h itself now.
#
# Qt6 comes from vcpkg (qtbase[widgets] in vcpkg.json), so the vcpkg
# toolchain finds it — no QT6_DIR / CMAKE_PREFIX_PATH needed.
find_package(Qt6 REQUIRED COMPONENTS Core Gui Widgets)

if(NOT TARGET renderer::renderer)
    message(FATAL_ERROR "The editor presents through the renderer library: enable BUILD_RENDERER (or disable BUILD_EDITOR).")
endif()

if(NOT TARGET cv-ffi::cv-ffi)
    message(FATAL_ERROR "The editor talks to the VFS/logger through cv-ffi, which BUILD_EDITOR should have already added (see the root CMakeLists.txt).")
endif()

set(EDITOR_SOURCES
        src/main.cpp
        src/EditorWindow.cpp
        src/EditorWindow.hpp
        src/NativeWindowBridge.hpp
        src/ViewportRenderer.cpp
        src/ViewportRenderer.hpp
        src/ViewportWidget.cpp
        src/ViewportWidget.hpp
)

# Exactly one NativeWindowBridge implementation per platform. Each keeps its
# platform headers (windows.h, Xlib, AppKit) out of every other file.
if(WIN32)
    list(APPEND EDITOR_SOURCES src/NativeWindowBridge_win32.cpp)
elseif(APPLE)
    # AppKit/QuartzCore are Objective-C: no way to reach them from plain C++.
    enable_language(OBJCXX)
    list(APPEND EDITOR_SOURCES src/NativeWindowBridge_macos.mm)
    set_source_files_properties(src/NativeWindowBridge_macos.mm PROPERTIES COMPILE_OPTIONS "-fobjc-arc")
elseif(UNIX)
    # Needs Xlib (see NativeWindowBridge_x11.cpp). Linked by name instead of
    # through find_package(X11 REQUIRED), like the renderer's own Xlib
    # support: the headers are only needed when compiling, and a
    # configure-time check would make any configure fail on a machine that
    # installs the X11 dev packages only after it (CI's Bootstrap step does
    # a configure of its own before cpp.yml installs them).
    list(APPEND EDITOR_SOURCES src/NativeWindowBridge_x11.cpp)
else()
    message(FATAL_ERROR "The editor has no NativeWindowBridge implementation for this platform.")
endif()

add_executable(editor ${EDITOR_SOURCES})

# AUTOMOC scoped to this target only — EditorWindow, ViewportWidget and
# ViewportRenderer use Q_OBJECT, so their moc'd sources need generating;
# nothing else in the project uses Qt so this stays off the global
# CMAKE_AUTOMOC.
set_target_properties(editor PROPERTIES AUTOMOC ON)

target_include_directories(editor PRIVATE src)

target_compile_features(editor PUBLIC cxx_std_23)

target_link_libraries(editor
        PRIVATE
        compiler_warnings
        renderer::renderer
        cv-ffi::cv-ffi
        Qt6::Core
        Qt6::Gui
        Qt6::Widgets
)

if(APPLE)
    # The .mm file includes headers that use std::expected, so it needs
    # the same language level as the C++ sources.
    set_target_properties(editor PROPERTIES OBJCXX_STANDARD 23 OBJCXX_STANDARD_REQUIRED ON)
    target_link_libraries(editor PRIVATE "-framework AppKit" "-framework QuartzCore")
elseif(UNIX)
    target_link_libraries(editor PRIVATE X11)
endif()

# ffi.h/the Rust staticlib are reached through cv-ffi (see
# engine/cpp/ffi-cpp/CMakeLists.txt) -- the editor itself no longer talks
# to the raw C ABI directly, so no link_ffi(editor) call here.
