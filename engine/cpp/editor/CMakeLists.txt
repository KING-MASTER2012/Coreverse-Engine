# editor: Qt6 executable.
#
# Faz 6.1 brings up a bare QApplication + QMainWindow and proves the
# Qt6/CMake/FFI chain end to end (window title comes from a real
# ffi_build_info_string() call) — deliberately WITHOUT linking
# `renderer` yet. That's on purpose: it isolates failure domains one
# at a time (Qt/CMake -> native window bridge -> FFI -> Vulkan
# surface) instead of debugging all four together. renderer::renderer
# is added starting Faz 6.3, once the native-window handle bridge
# (Faz 6.2) exists.
#
# Qt6 is a system install, not a vcpkg dependency — CMAKE_PREFIX_PATH
# is wired to $env{QT6_DIR} in the base CMakePresets.json preset, so
# every platform just needs that environment variable pointing at its
# Qt6 install (e.g. .../Qt/6.x/msvc2022_64 on Windows).
find_package(Qt6 REQUIRED COMPONENTS Core Gui Widgets)

add_executable(editor
        src/main.cpp
        src/EditorWindow.cpp
        src/EditorWindow.hpp
)

# AUTOMOC scoped to this target only — EditorWindow.hpp uses Q_OBJECT,
# so its moc'd source needs generating; nothing else in the project
# uses Qt so this stays off the global CMAKE_AUTOMOC.
set_target_properties(editor PROPERTIES AUTOMOC ON)

target_include_directories(editor PRIVATE src)

target_compile_features(editor PUBLIC cxx_std_23)

target_link_libraries(editor
        PRIVATE
        compiler_warnings
        Qt6::Core
        Qt6::Gui
        Qt6::Widgets
)

link_ffi(editor)
