#pragma once

#include <QMainWindow>
#include <memory>

#include <QtGlobal>

#include "cv-ffi/Logger.hpp"

class QCloseEvent;
class QEvent;
class QShowEvent;

namespace editor {

class ViewportRenderer;
class ViewportWidget;

/// Exit codes of the smoke-test mode (see EditorWindow::enableSmokeTest());
/// 0 is success. main() adds one more for validation errors found after
/// teardown.
namespace smoke_exit {
inline constexpr int kRendererFailed = 1;
inline constexpr int kTimedOut = 2;
inline constexpr int kNoSwapchainRebuild = 3;
inline constexpr int kValidationErrors = 4;
inline constexpr int kValidationMissing = 5;
} // namespace smoke_exit

/// Coreverse Editor's main window.
///
/// Phase 6.1: a QMainWindow whose title is populated from a real
/// ffi_build_info_string() call — proof the Qt6/CMake/FFI chain works
/// end to end.
///
/// Phase 6.2/6.3/6.4: its central widget is the ViewportWidget the renderer
/// presents into. The renderer is brought up from showEvent() (queued, so
/// the layout has settled and the native window has its real size), renders
/// a frame loop from then on, and is shut down from closeEvent() — before
/// the native window goes away, which the renderer's teardown order
/// requires. If Vulkan is unavailable the editor still opens; the status
/// bar says why.
///
/// Phase 7c: owns the cv_ffi::Logger every diagnostic in this process goes
/// through — a console sink always, a file sink under the project root once
/// one is open (not yet wired up to an actual "open project" flow; see
/// PROGRESS.md). Registers the `"CV-RENDERER"` producer once at
/// construction and hands `&m_logger` to ViewportRenderer, which routes the
/// renderer's own validation/log messages through it (see
/// ViewportRenderer::initialize()'s doc comment). m_logger is declared
/// before m_renderer below specifically so it outlives it, matching that
/// requirement.
class EditorWindow : public QMainWindow
{
    Q_OBJECT

public:
    EditorWindow();
    ~EditorWindow() override;

    /// Smoke-test mode (`editor --smoke-frames N`): renders `frames` frames,
    /// resizes the window half-way through, and quits the application with
    /// exit code 0 once the swapchain was rebuilt for that resize — or with
    /// one of smoke_exit's codes if the renderer failed to start or stopped,
    /// no rebuild happened, or the whole thing took longer than 30 seconds.
    /// It exists so the real Qt window path has a test (ctest label
    /// `editor`), not just the renderer library.
    void enableSmokeTest(quint64 frames);

protected:
    void showEvent(QShowEvent* event) override;
    void closeEvent(QCloseEvent* event) override;
    void changeEvent(QEvent* event) override;

private:
    /// Creates the device/surface/swapchain/frame sync for the viewport and
    /// starts rendering. A failure is reported in the status bar and the
    /// log, never fatal (except in smoke-test mode, where it fails the test).
    void initializeRenderer();

    /// Tears the renderer down; idempotent.
    void shutdownRenderer();

    void showRendererStatus();

    void onFrameRendered(quint64 totalFrames);

    ViewportWidget* m_viewport = nullptr; // owned by Qt (central widget)
    // Declaration order matters: m_logger must outlive m_renderer, which
    // holds a pointer to it for the lifetime of every ViewportRenderer it
    // owns (see the class comment above and ViewportRenderer's own
    // constructor doc comment) -- members are destroyed in reverse
    // declaration order, so m_logger going after m_renderer here would
    // destroy it first.
    cv_ffi::Logger m_logger;
    std::unique_ptr<ViewportRenderer> m_renderer;
    bool m_rendererStartScheduled = false;
    double m_framesPerSecond = 0.0; ///< Last measurement, 0 until the first one.

    quint64 m_smokeFrames = 0; ///< 0 = not in smoke-test mode.
    bool m_smokeResized = false;
    bool m_smokeDone = false;
    unsigned m_smokeRebuilds = 0;
};

} // namespace editor
