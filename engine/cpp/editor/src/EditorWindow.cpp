#include "EditorWindow.hpp"

#include <QCloseEvent>
#include <QDebug>
#include <QMetaObject>
#include <QShowEvent>
#include <QStatusBar>

#include "ViewportRenderer.hpp"
#include "ViewportWidget.hpp"
#include "ffi.h"

namespace editor {

namespace {
/// Builds the window title via ffi_build_info_string(), freeing the
/// Rust-allocated string before returning. This exercises the full
/// FFI round trip (call across the boundary + the matching
/// ffi_free_string, per ffi's ownership contract — see
/// engine/rust/crates/ffi/src/lib.rs) rather than just proving the
/// crate links.
QString MakeWindowTitle()
{
    char* info = ffi_build_info_string();
    if (info == nullptr) {
        // Documented as always non-null (see ffi_build_info_string's
        // doc comment) — this branch is defensive, not expected.
        return QStringLiteral("Coreverse Editor — <build info unavailable>");
    }

    const QString title = QStringLiteral("Coreverse Editor — %1").arg(QString::fromUtf8(info));
    ffi_free_string(info);
    return title;
}
} // namespace

EditorWindow::EditorWindow()
{
    setWindowTitle(MakeWindowTitle());
    resize(1280, 800);

    m_viewport = new ViewportWidget(this);
    setCentralWidget(m_viewport);

    m_renderer = std::make_unique<ViewportRenderer>(*m_viewport);
    connect(m_renderer.get(), &ViewportRenderer::swapchainRebuilt, this, [this] { showRendererStatus(); });
    connect(m_renderer.get(), &ViewportRenderer::swapchainRebuildFailed, this, [this](const QString& reason) {
        statusBar()->showMessage(tr("Renderer: swapchain rebuild failed — %1").arg(reason));
    });

    statusBar()->showMessage(tr("Renderer: starting…"));
}

EditorWindow::~EditorWindow()
{
    // closeEvent() normally got here first; this covers every other way a
    // window can go away. Either way the renderer must be gone before the
    // viewport's native window is destroyed (by ~QMainWindow, after this).
    shutdownRenderer();
}

void EditorWindow::showEvent(QShowEvent* event)
{
    QMainWindow::showEvent(event);

    if (!m_rendererStartScheduled) {
        m_rendererStartScheduled = true;
        // Queued, not direct: at this point the window is being shown but
        // the layout may not have given the viewport its final size yet.
        QMetaObject::invokeMethod(this, &EditorWindow::initializeRenderer, Qt::QueuedConnection);
    }
}

void EditorWindow::closeEvent(QCloseEvent* event)
{
    shutdownRenderer();
    QMainWindow::closeEvent(event);
}

void EditorWindow::initializeRenderer()
{
    if (m_renderer == nullptr) {
        return; // already shut down (closed before the queued call ran)
    }

    if (auto result = m_renderer->initialize(); !result) {
        qWarning().noquote() << "editor: renderer unavailable:" << result.error();
        statusBar()->showMessage(tr("Renderer unavailable — %1").arg(result.error()));
        return;
    }

    qInfo().noquote() << "editor: renderer up on" << m_renderer->deviceName();
    showRendererStatus();
}

void EditorWindow::shutdownRenderer()
{
    if (m_renderer != nullptr) {
        m_renderer->shutdown();
        m_renderer.reset();
    }
}

void EditorWindow::showRendererStatus()
{
    if (m_renderer == nullptr || !m_renderer->isInitialized()) {
        return;
    }

    const QSize extent = m_renderer->swapchainExtent();
    statusBar()->showMessage(tr("Vulkan — %1 — swapchain %2×%3, %4 images")
                                 .arg(m_renderer->deviceName())
                                 .arg(extent.width())
                                 .arg(extent.height())
                                 .arg(m_renderer->swapchainImageCount()));
}

} // namespace editor
