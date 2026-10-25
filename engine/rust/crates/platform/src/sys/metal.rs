//! The `CAMetalLayer` of a window's view on macOS and iOS.
//!
//! Vulkan on Apple platforms (MoltenVK) presents into a `CAMetalLayer`, but
//! `winit` hands out the `NSView` / `UIView`. `raw-window-metal` makes the view
//! layer-backed and adds a `CAMetalLayer` sublayer that follows the view's size
//! and scale factor. This is the only place that needs it, so the one `unsafe`
//! call lives here.

use std::num::NonZeroUsize;

use raw_window_metal::Layer;
use winit::raw_window_handle::{RawWindowHandle, WindowHandle};

/// The address of the `CAMetalLayer` for the view behind `handle`, or `None`
/// when the handle is not an AppKit or UIKit one.
///
/// Must run on the main thread (as every AppKit/UIKit call). The layer is kept
/// alive by the view's own layer, which holds the sublayer, so the address
/// stays valid as long as the view does; do not call this twice for one view,
/// each call adds another sublayer.
pub(crate) fn metal_layer(handle: &WindowHandle<'_>) -> Option<NonZeroUsize> {
    let layer = match handle.as_raw() {
        // SAFETY: `handle` is a `WindowHandle`, which guarantees that the
        // pointer it carries is a valid `NSView` for as long as it is borrowed.
        RawWindowHandle::AppKit(h) => unsafe { Layer::from_ns_view(h.ns_view) },
        // SAFETY: as above, for a `UIView`.
        RawWindowHandle::UiKit(h) => unsafe { Layer::from_ui_view(h.ui_view) },
        _ => return None,
    };
    // The `Layer` (a +1 reference) is dropped on return; the view's layer keeps
    // the sublayer alive.
    Some(layer.as_ptr().addr())
}
