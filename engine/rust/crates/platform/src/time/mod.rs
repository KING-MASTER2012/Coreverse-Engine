//! Clocks, timers and sleeping.
//!
//! Everything that needs "what time is it" is generic over [`TimeSource`], so
//! tests drive it with a [`ManualClock`] instead of the real clock and never
//! depend on how fast the machine runs.
//!
//! * [`Clock`] — the real monotonic clock; [`WallClock`] — calendar time.
//! * [`Stopwatch`] — start/stop/reset elapsed time.
//! * [`FrameTimer`] — per-frame delta and smoothed FPS.
//! * [`FixedStep`] — fixed-timestep accumulator for simulation updates.
//! * [`precise_sleep`] — sleep that is accurate to well under a millisecond.

mod fixed;
mod frame;
mod sleep;
mod source;
mod stopwatch;

pub use fixed::{DEFAULT_MAX_STEPS, FixedStep};
pub use frame::FrameTimer;
pub use sleep::{precise_sleep, precise_sleep_until};
pub use source::{Clock, ManualClock, TimeSource, WallClock};
pub use stopwatch::Stopwatch;
