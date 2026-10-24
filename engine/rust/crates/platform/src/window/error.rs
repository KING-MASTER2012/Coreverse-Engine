//! Errors of the window API.

use ccore::CoreError;

use super::WindowId;

/// Why a window operation failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum WindowError {
    /// A [`WindowDesc`](super::WindowDesc) cannot be used (a zero or
    /// non-finite size, a minimum larger than the maximum, ...).
    #[error("invalid window description: {0}")]
    InvalidDesc(String),
    /// An argument or configuration value is out of range (a size that is
    /// not finite and positive, a scale factor of zero, ...).
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    /// No window with this id exists (it was never created or has been
    /// closed).
    #[error("no window with id {0}")]
    UnknownWindow(WindowId),
    /// The headless event loop has nothing left to do (no queued events, no
    /// pending redraw, a control flow that waits) and the app has not asked to
    /// exit. A real backend would block forever here; the headless one cannot
    /// be woken by anything, so it reports it instead.
    #[error("the event loop is idle and the app has not exited; nothing can wake it")]
    Stalled,
    /// The event loop ran for this many steps without exiting or going idle
    /// (typically `ControlFlow::Poll` in an app that never calls `exit`).
    #[error("the event loop did not exit or go idle within {0} steps")]
    StepLimit(usize),
    /// More than this many events were delivered in one step, which means
    /// an app that answers every event with another one (for example a
    /// resize inside its own `Resized` handler).
    #[error("more than {0} events in a single step; an event handler keeps producing events")]
    EventStorm(usize),
}

impl From<WindowError> for CoreError {
    fn from(err: WindowError) -> Self {
        let message = err.to_string();
        match err {
            WindowError::InvalidDesc(_)
            | WindowError::InvalidArgument(_)
            | WindowError::UnknownWindow(_) => CoreError::InvalidArgument(message),
            WindowError::Stalled | WindowError::StepLimit(_) | WindowError::EventStorm(_) => {
                CoreError::InvalidState(message)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_into_core_error_by_kind() {
        let invalid: CoreError = WindowError::InvalidDesc("no".into()).into();
        assert!(matches!(invalid, CoreError::InvalidArgument(_)));
        let unknown: CoreError = WindowError::UnknownWindow(WindowId::from_u64(7)).into();
        assert!(matches!(unknown, CoreError::InvalidArgument(ref m) if m.contains('7')));
        let stalled: CoreError = WindowError::Stalled.into();
        assert!(matches!(stalled, CoreError::InvalidState(_)));
        let limit: CoreError = WindowError::StepLimit(5).into();
        assert!(matches!(limit, CoreError::InvalidState(ref m) if m.contains('5')));
    }
}
