#include "EditorWindow.hpp"

#include "ffi.h"

namespace editor
{

    namespace
    {
        /// Builds the window title via ffi_build_info_string(), freeing the
        /// Rust-allocated string before returning. This exercises the full
        /// FFI round trip (call across the boundary + the matching
        /// ffi_free_string, per ffi's ownership contract — see
        /// engine/rust/crates/ffi/src/lib.rs) rather than just proving the
        /// crate links.
        QString MakeWindowTitle()
        {
            char* info = ffi_build_info_string();
            if (info == nullptr)
            {
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
    }

} // namespace editor
