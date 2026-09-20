// Phase 6.1 proof: QApplication + QMainWindow open and close cleanly,
// with the window title populated via a real Rust ffi call
// (ffi_build_info_string()). Confirms Qt6 bring-up, CMake wiring, and
// the FFI boundary.
//
// Phase 6.2/6.3: the window's central widget is a native-window viewport
// the Vulkan renderer presents into (device, surface and swapchain live
// in ViewportRenderer, swapchain rebuilt on resize). It renders no frames
// yet — that is Phase 6.4.

#include <QApplication>

#include "EditorWindow.hpp"

int main(int argc, char** argv)
{
    QApplication app(argc, argv);

    editor::EditorWindow window;
    window.show();

    return QApplication::exec();
}
