//! What a window backend provides.

use super::app::App;
use super::error::WindowError;

/// Runs an [`App`]: owns the event loop, creates the real windows and delivers
/// the callbacks.
///
/// There are (or will be) three backends, all behind this one contract: the
/// [headless backend](super::HeadlessBackend) that needs no display, a desktop
/// backend on `winit`, and the Android and iOS backends. Code written against
/// [`App`] and [`AppContext`](super::AppContext) does not change between them.
pub trait Backend {
    /// Runs `app` until it exits and returns the exit code it asked for
    /// ([`AppContext::exit`](super::AppContext::exit) gives `0`).
    ///
    /// Real backends do not return until the app exits (on some platforms,
    /// for example iOS, not even then). The headless backend returns an error
    /// instead of waiting for events that can never come, see
    /// [`WindowError::Stalled`].
    ///
    /// # Errors
    /// Whatever the backend cannot recover from; for the headless backend
    /// [`WindowError::Stalled`], [`WindowError::StepLimit`],
    /// [`WindowError::EventStorm`] and [`WindowError::InvalidArgument`] for a
    /// bad configuration or event.
    fn run<A: App>(self, app: A) -> Result<i32, WindowError>;
}
