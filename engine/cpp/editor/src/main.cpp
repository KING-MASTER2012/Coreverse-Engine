// Phase 6.1 proof: QApplication + QMainWindow open and close cleanly,
// with the window title populated via a real Rust ffi call
// (ffi_build_info_string()). Confirms Qt6 bring-up, CMake wiring, and
// the FFI boundary all work before renderer/Vulkan enter the picture
// in Phase 6.3+.

#include <QApplication>

#include "EditorWindow.hpp"

int main(int argc, char** argv)
{
    QApplication app(argc, argv);

    editor::EditorWindow window;
    window.show();

    return QApplication::exec();
}
