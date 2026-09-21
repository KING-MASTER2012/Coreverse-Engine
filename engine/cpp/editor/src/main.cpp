// Phase 6.1 proof: QApplication + QMainWindow open and close cleanly,
// with the window title populated via a real Rust ffi call
// (ffi_build_info_string()). Confirms Qt6 bring-up, CMake wiring, and
// the FFI boundary.
//
// Phase 6.2/6.3/6.4: the window's central widget is a native-window viewport
// the Vulkan renderer presents into (device, surface, swapchain and frame
// synchronization live in ViewportRenderer, which also runs the render loop
// and rebuilds the swapchain on resize).
//
// `editor --smoke-frames N` is the automated test of that whole path: it
// renders N frames, resizes the window on the way, and exits (see
// EditorWindow::enableSmokeTest()).

#include <QApplication>
#include <QCommandLineOption>
#include <QCommandLineParser>
#include <QDebug>

#include "EditorWindow.hpp"
#include "renderer/Diagnostics.hpp"

int main(int argc, char** argv)
{
    QApplication app(argc, argv);

    QCommandLineParser parser;
    parser.setApplicationDescription(QStringLiteral("Coreverse Editor"));
    parser.addHelpOption();
    const QCommandLineOption smokeOption(
        QStringLiteral("smoke-frames"),
        QStringLiteral(
            "Render <count> frames (resizing the window on the way), then exit: 0 on success. For automated tests."
        ),
        QStringLiteral("count")
    );
    parser.addOption(smokeOption);
    parser.process(app);

    quint64 smokeFrames = 0;
    if (parser.isSet(smokeOption)) {
        bool ok = false;
        smokeFrames = parser.value(smokeOption).toULongLong(&ok);
        if (!ok || smokeFrames == 0) {
            qCritical() << "--smoke-frames needs a positive number";
            return 64; // EX_USAGE
        }
    }

    int exitCode = 0;
    {
        editor::EditorWindow window;
        if (smokeFrames != 0) {
            window.enableSmokeTest(smokeFrames);
        }
        window.show();
        exitCode = QApplication::exec();
    } // the renderer is fully torn down here — after this, validation has said everything it will say

    if (smokeFrames != 0 && exitCode == 0 && renderer::ValidationErrorCount() != 0) {
        qCritical() << "editor smoke test: the validation layer reported errors (see the [vulkan] lines above)";
        return editor::smoke_exit::kValidationErrors;
    }
    return exitCode;
}
