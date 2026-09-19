#pragma once

#include <QMainWindow>

namespace editor {

/// Coreverse Editor's main window.
///
/// Phase 6.1: a bare QMainWindow whose title is populated from a real
/// ffi_build_info_string() call — proof the Qt6/CMake/FFI chain works
/// end to end. renderer is not linked in yet.
///
/// Phase 6.3 adds initializeRenderer()/shutdownRenderer() here, called
/// from show()-time and closeEvent() respectively, once Phase 6.2's
/// native-window handle bridge exists to feed CreateSurface().
class EditorWindow : public QMainWindow
{
    Q_OBJECT

public:
    EditorWindow();
};

} // namespace editor
