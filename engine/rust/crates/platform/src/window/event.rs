//! Window identity, the events a window reports and the loop's control flow.

use std::fmt;
use std::time::Instant;

use crate::input::InputEvent;

use super::geometry::{PhysicalPosition, PhysicalSize};
use super::handle::NativeWindowHandle;

/// Identifies a window for as long as it exists. Ids are never reused within
/// one event loop, so a stale id never refers to a different window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowId(u64);

impl WindowId {
    /// The id with this raw value. Useful across the FFI boundary; an id that
    /// was never handed out simply refers to no window.
    #[must_use]
    pub const fn from_u64(raw: u64) -> Self {
        Self(raw)
    }

    /// The raw value of the id.
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl fmt::Display for WindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Something that happened to a window, delivered to
/// [`App::window_event`](super::App::window_event).
///
/// The enum is `#[non_exhaustive]`: more events may be added, so keep a
/// catch-all arm.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum WindowEvent {
    /// The user (or the system) asked the window to close. Nothing has been
    /// closed yet: call [`AppContext::close_window`](super::AppContext::close_window)
    /// to do it, or ignore the event to keep the window.
    CloseRequested,
    /// The window has been closed. It is still readable through
    /// [`AppContext::window`](super::AppContext::window) while this event is
    /// handled and gone afterwards. If it had a surface,
    /// [`SurfaceLost`](Self::SurfaceLost) was delivered first.
    Destroyed,
    /// The client area changed size (physical pixels).
    Resized(PhysicalSize),
    /// The window moved (physical pixels, top-left corner).
    Moved(PhysicalPosition),
    /// The scale factor changed, for example because the window moved to
    /// another monitor. `inner_size` is the new physical size of the client
    /// area, which keeps its logical size; no separate
    /// [`Resized`](Self::Resized) follows.
    ScaleFactorChanged {
        /// The new scale factor.
        scale_factor: f64,
        /// The new client-area size in physical pixels.
        inner_size: PhysicalSize,
    },
    /// The window gained (`true`) or lost (`false`) keyboard focus.
    Focused(bool),
    /// The window became fully hidden (`true`), for example minimized or
    /// covered, or visible again (`false`). A good time to stop rendering.
    Occluded(bool),
    /// The window should draw a frame. Only delivered after
    /// [`AppContext::request_redraw`](super::AppContext::request_redraw) (or a
    /// request from the system, such as exposure), at most once per loop
    /// iteration and window, and not while the app is suspended.
    RedrawRequested,
    /// A presentation surface now exists for the window: create the swapchain
    /// from this handle. Delivered when the window gets its surface (on the
    /// desktop right after creation, on Android and iOS whenever the system
    /// provides a new one).
    SurfaceCreated(NativeWindowHandle),
    /// The window's surface is gone: destroy everything created from the last
    /// [`SurfaceCreated`](Self::SurfaceCreated) handle (swapchain, framebuffers)
    /// before returning from the handler. Every `SurfaceCreated` is eventually
    /// followed by exactly one `SurfaceLost`: when the app is suspended, when
    /// the system replaces the surface, or when the window is destroyed.
    SurfaceLost,
    /// The user did something with an input device: a key, the mouse, a touch
    /// or an input method. See [`InputEvent`], and
    /// [`InputState`](crate::input::InputState) to query input per frame
    /// instead of tracking it by hand.
    Input(InputEvent),
}

/// How the event loop waits between iterations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ControlFlow {
    /// Never wait: start the next iteration immediately. For games that draw
    /// continuously.
    Poll,
    /// Sleep until the next event arrives. The default, and what editors and
    /// tools want.
    #[default]
    Wait,
    /// Sleep until the next event arrives or the instant is reached,
    /// whichever comes first.
    WaitUntil(Instant),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_ids_order_and_convert() {
        let a = WindowId::from_u64(1);
        let b = WindowId::from_u64(2);
        assert!(a < b);
        assert_eq!(a.as_u64(), 1);
        assert_eq!(b.to_string(), "2");
        assert_eq!(WindowId::from_u64(5), WindowId::from_u64(5));
    }

    #[test]
    fn control_flow_defaults_to_wait() {
        assert_eq!(ControlFlow::default(), ControlFlow::Wait);
    }
}
