#pragma once

#include <QMainWindow>
#include <memory>

class QCloseEvent;
class QShowEvent;

namespace editor {

    class ViewportRenderer;
    class ViewportWidget;

    /// Coreverse Editor's main window.
    ///
    /// Phase 6.1: a QMainWindow whose title is populated from a real
    /// ffi_build_info_string() call — proof the Qt6/CMake/FFI chain works
    /// end to end.
    ///
    /// Phase 6.2/6.3: its central widget is the ViewportWidget the renderer
    /// presents into. The renderer is brought up from showEvent() (queued, so
    /// the layout has settled and the native window has its real size) and
    /// shut down from closeEvent() — before the native window goes away,
    /// which the renderer's teardown order requires. If Vulkan is
    /// unavailable the editor still opens; the status bar says why.
    class EditorWindow : public QMainWindow
    {
        Q_OBJECT

    public:
        EditorWindow();
        ~EditorWindow() override;

    protected:
        void showEvent(QShowEvent* event) override;
        void closeEvent(QCloseEvent* event) override;

    private:
        /// Creates the device/surface/swapchain for the viewport. A failure is
        /// reported in the status bar and the log, never fatal.
        void initializeRenderer();

        /// Tears the renderer down; idempotent.
        void shutdownRenderer();

        void showRendererStatus();

        ViewportWidget* m_viewport = nullptr; // owned by Qt (central widget)
        std::unique_ptr<ViewportRenderer> m_renderer;
        bool m_rendererStartScheduled = false;
    };

} // namespace editor
