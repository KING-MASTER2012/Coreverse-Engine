#include "EditorWindow.hpp"

#include <QApplication>
#include <QCloseEvent>
#include <QDebug>
#include <QEvent>
#include <QMetaObject>
#include <QShowEvent>
#include <QSize>
#include <QStatusBar>
#include <cstdlib>

#include <QTimer>

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

/// The smoke test as a whole must not take longer than this: a renderer
/// that never starts or a loop that never reaches its frame count has to
/// fail the test, not stall CI.
constexpr int kSmokeTimeoutMs = 30'000;

/// CV_REQUIRE_VALIDATION=1 (set by CI): fail if the validation layer is not
/// running, so a passing smoke test cannot silently mean "nothing was checked".
bool ValidationRequired()
{
    const QString value = qEnvironmentVariable("CV_REQUIRE_VALIDATION");
    return !value.isEmpty() && value != QLatin1String("0");
}
} // namespace

EditorWindow::EditorWindow()
{
    setWindowTitle(MakeWindowTitle());
    resize(1280, 800);

    m_viewport = new ViewportWidget(this);
    setCentralWidget(m_viewport);

    m_renderer = std::make_unique<ViewportRenderer>(*m_viewport);
    connect(m_renderer.get(), &ViewportRenderer::swapchainRebuilt, this, [this] {
        ++m_smokeRebuilds;
        showRendererStatus();
    });
    connect(m_renderer.get(), &ViewportRenderer::swapchainRebuildFailed, this, [this](const QString& reason) {
        statusBar()->showMessage(tr("Renderer: swapchain rebuild failed — %1").arg(reason));
    });
    connect(m_renderer.get(), &ViewportRenderer::frameStatsUpdated, this, [this](double framesPerSecond) {
        m_framesPerSecond = framesPerSecond;
        showRendererStatus();
    });
    connect(m_renderer.get(), &ViewportRenderer::frameRendered, this, &EditorWindow::onFrameRendered);
    connect(m_renderer.get(), &ViewportRenderer::renderingFailed, this, [this](const QString& reason) {
        statusBar()->showMessage(tr("Renderer stopped — %1").arg(reason));
        if (m_smokeFrames != 0 && !m_smokeDone) {
            m_smokeDone = true;
            QApplication::exit(smoke_exit::kRendererFailed);
        }
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

void EditorWindow::enableSmokeTest(quint64 frames)
{
    m_smokeFrames = frames;

    QTimer::singleShot(kSmokeTimeoutMs, this, [this] {
        if (!m_smokeDone) {
            m_smokeDone = true;
            qCritical().noquote() << "editor smoke test: timed out after" << kSmokeTimeoutMs << "ms with"
                                  << (m_renderer != nullptr ? m_renderer->framesRendered() : 0) << "frame(s) rendered";
            QApplication::exit(smoke_exit::kTimedOut);
        }
    });
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

void EditorWindow::changeEvent(QEvent* event)
{
    QMainWindow::changeEvent(event);

    if (event->type() == QEvent::WindowStateChange && m_renderer != nullptr) {
        m_renderer->setWindowMinimized(isMinimized());
    }
}

void EditorWindow::initializeRenderer()
{
    if (m_renderer == nullptr) {
        return; // already shut down (closed before the queued call ran)
    }

    if (auto result = m_renderer->initialize(); !result) {
        qWarning().noquote() << "editor: renderer unavailable:" << result.error();
        statusBar()->showMessage(tr("Renderer unavailable — %1").arg(result.error()));
        if (m_smokeFrames != 0 && !m_smokeDone) {
            m_smokeDone = true;
            QApplication::exit(smoke_exit::kRendererFailed);
        }
        return;
    }

    qInfo().noquote() << "editor: renderer up on" << m_renderer->deviceName() << "- validation"
                      << (m_renderer->validationEnabled() ? "on" : "off");
    showRendererStatus();

    if (m_smokeFrames != 0 && ValidationRequired() && !m_renderer->validationEnabled()) {
        qCritical() << "editor smoke test: CV_REQUIRE_VALIDATION is set but the validation layer is not running";
        m_smokeDone = true;
        QApplication::exit(smoke_exit::kValidationMissing);
    }
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
    QString message = tr("Vulkan — %1 — swapchain %2×%3, %4 images")
                          .arg(m_renderer->deviceName())
                          .arg(extent.width())
                          .arg(extent.height())
                          .arg(m_renderer->swapchainImageCount());
    if (m_framesPerSecond > 0.0) {
        message += tr(" — %1 fps").arg(m_framesPerSecond, 0, 'f', 1);
    }
    statusBar()->showMessage(message);
}

void EditorWindow::onFrameRendered(quint64 totalFrames)
{
    if (m_smokeFrames == 0 || m_smokeDone) {
        return;
    }

    // Half-way through, resize the window: the swapchain has to follow, which
    // is the one thing the frame loop alone does not exercise.
    if (!m_smokeResized && totalFrames >= m_smokeFrames / 2) {
        m_smokeResized = true;
        resize(size() + QSize(160, 100));
    }

    if (totalFrames >= m_smokeFrames) {
        m_smokeDone = true;
        if (m_smokeRebuilds == 0) {
            qCritical() << "editor smoke test: the window was resized but the swapchain was never rebuilt";
            QApplication::exit(smoke_exit::kNoSwapchainRebuild);
            return;
        }
        qInfo().noquote() << "editor smoke test: OK -" << totalFrames << "frames," << m_smokeRebuilds
                          << "swapchain rebuild(s)";
        QApplication::exit(0);
    }
}

} // namespace editor
