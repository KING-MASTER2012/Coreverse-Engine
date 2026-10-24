//! Windows and the application lifecycle: an engine-owned API that every
//! window backend implements.
//!
//! This module is the *API* and the *headless backend*; the desktop backend
//! (on `winit`) and the Android and iOS backends come later and plug into the
//! same [`Backend`] contract. Input events (keyboard, mouse, touch, IME) are
//! not part of it yet; [`WindowEvent`] is `#[non_exhaustive]` so they can be
//! added.
//!
//! # The pieces
//!
//! * [`App`] — the callbacks of an application: [`resumed`](App::resumed),
//!   [`suspended`](App::suspended), [`window_event`](App::window_event) and
//!   [`about_to_wait`](App::about_to_wait). The lifecycle (when each is
//!   called, in which order) is documented on the trait.
//! * [`AppContext`] — passed to every callback: create windows
//!   ([`WindowDesc`] describes one), change them, ask for redraws, choose the
//!   [`ControlFlow`], exit.
//! * [`Window`] — the readable state of a window; [`WindowId`] names it.
//! * [`WindowEvent`] — what happens to a window, including
//!   [`SurfaceCreated`](WindowEvent::SurfaceCreated) and
//!   [`SurfaceLost`](WindowEvent::SurfaceLost) for platforms (Android, iOS)
//!   where the presentation surface disappears and comes back while the app
//!   lives.
//! * [`NativeWindowHandle`] — the handle a graphics API needs to create a
//!   surface, one variant per windowing system, Android and iOS included.
//! * [`Backend`] — runs an [`App`]. [`HeadlessBackend`] needs no display;
//!   [`HeadlessLoop`] is the same loop driven step by step for tests.
//!
//! # Units
//!
//! Requests ([`WindowDesc`], [`AppContext::set_inner_size`]) use logical,
//! DPI-independent units; what windows report (`Resized`, [`Window::inner_size`])
//! uses physical pixels, because that is the size of a swapchain. See
//! [`geometry`](LogicalSize) types for the conversion.
//!
//! # Example
//!
//! ```
//! use platform::window::{
//!     App, AppContext, Backend, HeadlessBackend, HostEvent, WindowDesc, WindowEvent, WindowId,
//! };
//!
//! #[derive(Default)]
//! struct Hello {
//!     frames: u32,
//! }
//!
//! impl App for Hello {
//!     fn resumed(&mut self, ctx: &mut AppContext) {
//!         // Windows are created here, never before.
//!         ctx.create_window(WindowDesc::new("Hello")).unwrap();
//!     }
//!
//!     fn window_event(&mut self, ctx: &mut AppContext, window: WindowId, event: WindowEvent) {
//!         match event {
//!             WindowEvent::CloseRequested => ctx.exit(),
//!             WindowEvent::RedrawRequested => self.frames += 1, // draw into the swapchain
//!             _ => {}
//!         }
//!         let _ = window;
//!     }
//!
//!     fn about_to_wait(&mut self, ctx: &mut AppContext) {
//!         // Per-frame work: update, then ask for a redraw.
//!         let ids: Vec<_> = ctx.windows().map(|w| w.id()).collect();
//!         for id in ids {
//!             ctx.request_redraw(id).unwrap();
//!         }
//!     }
//! }
//!
//! let mut backend = HeadlessBackend::default();
//! backend.push(HostEvent::CloseRequested { window: WindowId::from_u64(1) });
//! assert_eq!(backend.run(Hello::default()).unwrap(), 0);
//! ```
//!
//! # Backends
//!
//! | Backend | Status | Needs |
//! | :------ | :----- | :---- |
//! | headless ([`HeadlessBackend`], [`HeadlessLoop`]) | implemented, tested on every CI platform | nothing |
//! | desktop (`winit`: Windows, macOS, Linux X11/Wayland) | planned | a display |
//! | Android, iOS | planned, compile-checked only | a device |

mod app;
mod backend;
mod desc;
mod error;
mod event;
mod geometry;
mod handle;
mod headless;
mod state;

#[cfg(test)]
mod tests;

pub use app::{App, AppContext};
pub use backend::Backend;
pub use desc::{Fullscreen, WindowDesc};
pub use error::WindowError;
pub use event::{ControlFlow, WindowEvent, WindowId};
pub use geometry::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize};
pub use handle::NativeWindowHandle;
pub use headless::{
    DEFAULT_STEP_LIMIT, HeadlessBackend, HeadlessConfig, HeadlessLoop, HostEvent,
    MAX_EVENTS_PER_STEP, RunOutcome, StepOutcome,
};
pub use state::Window;
