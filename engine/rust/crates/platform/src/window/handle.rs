//! The native handle of a window's presentation surface.

use std::num::{NonZeroU64, NonZeroUsize};

use crate::info::DisplayServer;

/// What a graphics API needs to create a presentation surface for a window,
/// one variant per windowing system.
///
/// The variants carry the same objects the renderer's C++
/// `renderer::NativeWindowHandle` expects (`HWND`/`HINSTANCE`, `Display*` and
/// `Window`, `wl_display*`/`wl_surface*`, `CAMetalLayer*`) plus the two
/// mobile ones (`ANativeWindow*` on Android, `CAMetalLayer*` on iOS), so a
/// handle can be passed to the renderer without conversion. Pointers are
/// stored as non-zero integers: this type is `Send`, `Sync` and `Copy`, and
/// the crate stays free of `unsafe`. Whoever turns a handle back into a
/// pointer takes on the rule that it is only valid between the
/// [`SurfaceCreated`](super::WindowEvent::SurfaceCreated) event that delivered
/// it and the next [`SurfaceLost`](super::WindowEvent::SurfaceLost).
///
/// The variants mirror [`DisplayServer`] (see [`display_server`](Self::display_server));
/// a window without a surface (every window of the headless backend, until a
/// test attaches one) has no handle at all, see
/// [`Window::native_handle`](super::Window::native_handle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NativeWindowHandle {
    /// Windows: an `HWND`, and optionally the `HINSTANCE` of the module that
    /// owns it (the renderer falls back to the current module when it is
    /// `None`).
    Win32 {
        /// The `HWND`.
        hwnd: NonZeroUsize,
        /// The `HINSTANCE`, if known.
        hinstance: Option<NonZeroUsize>,
    },
    /// macOS: the `CAMetalLayer*` of the window's view (what MoltenVK's
    /// `VK_EXT_metal_surface` takes).
    Cocoa {
        /// The `CAMetalLayer*`.
        metal_layer: NonZeroUsize,
    },
    /// X11 (Xlib): a `Window` and, optionally, the `Display*` it lives on.
    X11 {
        /// The Xlib `Display*`, if known.
        display: Option<NonZeroUsize>,
        /// The Xlib `Window` (an `unsigned long`, widened to 64 bits).
        window: NonZeroU64,
    },
    /// Wayland: the `wl_display*` and the window's `wl_surface*`.
    Wayland {
        /// The `wl_display*`.
        display: NonZeroUsize,
        /// The `wl_surface*`.
        surface: NonZeroUsize,
    },
    /// Android: the `ANativeWindow*` of the activity. Android destroys it when
    /// the app goes to the background, so expect `SurfaceLost` /
    /// `SurfaceCreated` pairs.
    Android {
        /// The `ANativeWindow*`.
        native_window: NonZeroUsize,
    },
    /// iOS: the `CAMetalLayer*` of the window's view. Like Android, the
    /// surface can be lost and recreated while the app is suspended.
    UiKit {
        /// The `CAMetalLayer*`.
        metal_layer: NonZeroUsize,
    },
}

impl NativeWindowHandle {
    /// The windowing system this handle belongs to.
    #[must_use]
    pub const fn display_server(&self) -> DisplayServer {
        match self {
            Self::Win32 { .. } => DisplayServer::Win32,
            Self::Cocoa { .. } => DisplayServer::Cocoa,
            Self::X11 { .. } => DisplayServer::X11,
            Self::Wayland { .. } => DisplayServer::Wayland,
            Self::Android { .. } => DisplayServer::Android,
            Self::UiKit { .. } => DisplayServer::UiKit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nz(value: usize) -> NonZeroUsize {
        NonZeroUsize::new(value).expect("test values are non-zero")
    }

    #[test]
    fn every_variant_maps_to_its_display_server() {
        let cases = [
            (
                NativeWindowHandle::Win32 {
                    hwnd: nz(1),
                    hinstance: None,
                },
                DisplayServer::Win32,
            ),
            (
                NativeWindowHandle::Cocoa { metal_layer: nz(2) },
                DisplayServer::Cocoa,
            ),
            (
                NativeWindowHandle::X11 {
                    display: None,
                    window: NonZeroU64::new(3).unwrap(),
                },
                DisplayServer::X11,
            ),
            (
                NativeWindowHandle::Wayland {
                    display: nz(4),
                    surface: nz(5),
                },
                DisplayServer::Wayland,
            ),
            (
                NativeWindowHandle::Android {
                    native_window: nz(6),
                },
                DisplayServer::Android,
            ),
            (
                NativeWindowHandle::UiKit { metal_layer: nz(7) },
                DisplayServer::UiKit,
            ),
        ];
        for (handle, expected) in cases {
            assert_eq!(handle.display_server(), expected);
        }
    }

    #[test]
    fn handles_are_copy_send_sync_and_compare_by_value() {
        fn assert_traits<T: Copy + Send + Sync + Eq + std::hash::Hash>() {}
        assert_traits::<NativeWindowHandle>();

        let a = NativeWindowHandle::Android {
            native_window: nz(0x1000),
        };
        let b = NativeWindowHandle::Android {
            native_window: nz(0x1000),
        };
        let c = NativeWindowHandle::Android {
            native_window: nz(0x2000),
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn an_optional_handle_costs_nothing_extra() {
        assert_eq!(
            size_of::<Option<NativeWindowHandle>>(),
            size_of::<NativeWindowHandle>()
        );
    }
}
