//! Fixed-timestep accumulator.

use std::time::Duration;

/// Steps per [`FixedStep::advance`] call unless
/// [`FixedStep::with_max_steps`] says otherwise.
pub const DEFAULT_MAX_STEPS: u32 = 8;

const NANOS_PER_SEC: u128 = 1_000_000_000;

/// Turns variable frame times into a whole number of fixed simulation steps.
///
/// ```
/// # use std::time::Duration;
/// # use platform::time::FixedStep;
/// let mut fixed = FixedStep::from_hz(60).unwrap();
/// let steps = fixed.advance(Duration::from_millis(40));
/// assert_eq!(steps, 2); // 40 ms is two 60 Hz steps with time left over
/// let blend = fixed.alpha(); // render between the last two states
/// assert!((0.0..1.0).contains(&blend));
/// ```
///
/// A frame that took very long would demand many steps, and running them
/// makes the next frame even longer (the "spiral of death"). Each
/// [`advance`](Self::advance) therefore runs at most `max_steps` steps and
/// discards the time beyond that.
#[derive(Copy, Clone, Debug)]
pub struct FixedStep {
    step: Duration,
    accumulator: Duration,
    max_steps: u32,
}

impl FixedStep {
    /// A accumulator for steps of length `step`, or `None` when `step` is zero.
    #[must_use]
    pub fn new(step: Duration) -> Option<Self> {
        (!step.is_zero()).then_some(Self {
            step,
            accumulator: Duration::ZERO,
            max_steps: DEFAULT_MAX_STEPS,
        })
    }

    /// An accumulator for `hz` steps per second, or `None` when `hz` is zero
    /// or so large that a step would round to zero nanoseconds.
    #[must_use]
    pub fn from_hz(hz: u32) -> Option<Self> {
        if hz == 0 {
            return None;
        }
        Self::new(Duration::from_secs(1) / hz)
    }

    /// Sets the cap on steps per [`advance`](Self::advance); `0` counts as `1`.
    #[must_use]
    pub fn with_max_steps(mut self, max_steps: u32) -> Self {
        self.max_steps = max_steps.max(1);
        self
    }

    /// Adds `frame_delta` and returns how many steps to run now.
    pub fn advance(&mut self, frame_delta: Duration) -> u32 {
        let total = self.accumulator.saturating_add(frame_delta).as_nanos();
        let step = self.step.as_nanos();
        let due = total / step;
        // What does not make a whole step stays; whatever exceeds the cap is
        // dropped, so the remainder is the same either way.
        self.accumulator = duration_from_nanos(total % step);
        u32::try_from(due).map_or(self.max_steps, |due| due.min(self.max_steps))
    }

    /// How far the accumulator is into the next step, in `[0, 1)`. Use it to
    /// interpolate rendering between the previous and the current state.
    #[must_use]
    pub fn alpha(&self) -> f64 {
        self.accumulator.as_secs_f64() / self.step.as_secs_f64()
    }

    /// The step length.
    #[must_use]
    pub fn step(&self) -> Duration {
        self.step
    }

    /// Time accumulated towards the next step.
    #[must_use]
    pub fn accumulated(&self) -> Duration {
        self.accumulator
    }

    /// Drops the accumulated time, e.g. after loading or unpausing.
    pub fn reset(&mut self) {
        self.accumulator = Duration::ZERO;
    }
}

fn duration_from_nanos(nanos: u128) -> Duration {
    let secs = u64::try_from(nanos / NANOS_PER_SEC).unwrap_or(u64::MAX);
    // `% 1e9` always fits in u32.
    let subsec = u32::try_from(nanos % NANOS_PER_SEC).unwrap_or(0);
    Duration::new(secs, subsec)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: fn(u64) -> Duration = Duration::from_millis;

    #[test]
    fn zero_step_is_rejected() {
        assert!(FixedStep::new(Duration::ZERO).is_none());
        assert!(FixedStep::from_hz(0).is_none());
        assert!(FixedStep::from_hz(u32::MAX).is_none());
    }

    #[test]
    fn whole_steps_and_remainder() {
        let mut fixed = FixedStep::new(MS(10)).unwrap();
        assert_eq!(fixed.advance(MS(4)), 0);
        assert_eq!(fixed.advance(MS(4)), 0);
        assert_eq!(fixed.advance(MS(4)), 1); // 12 ms accumulated
        assert_eq!(fixed.accumulated(), MS(2));
        assert_eq!(fixed.advance(MS(28)), 3); // 30 ms
        assert_eq!(fixed.accumulated(), Duration::ZERO);
    }

    #[test]
    fn alpha_is_the_fraction_of_the_next_step() {
        let mut fixed = FixedStep::new(MS(10)).unwrap();
        fixed.advance(MS(15));
        assert!((fixed.alpha() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn long_frames_are_capped_and_the_excess_dropped() {
        let mut fixed = FixedStep::new(MS(10)).unwrap().with_max_steps(5);
        assert_eq!(fixed.advance(Duration::from_secs(10)), 5);
        // The 10 s did not pile up: the next frame starts clean.
        assert_eq!(fixed.accumulated(), Duration::ZERO);
        assert_eq!(fixed.advance(MS(10)), 1);
    }

    #[test]
    fn default_and_minimum_cap() {
        let mut fixed = FixedStep::new(MS(1)).unwrap();
        assert_eq!(fixed.advance(MS(100)), DEFAULT_MAX_STEPS);
        let mut fixed = FixedStep::new(MS(1)).unwrap().with_max_steps(0);
        assert_eq!(fixed.advance(MS(100)), 1);
    }

    #[test]
    fn huge_delta_does_not_overflow() {
        let mut fixed = FixedStep::new(Duration::from_nanos(3)).unwrap();
        assert_eq!(fixed.advance(Duration::MAX), DEFAULT_MAX_STEPS);
        assert!(fixed.accumulated() < Duration::from_nanos(3));
    }

    #[test]
    fn sixty_hertz_step() {
        let fixed = FixedStep::from_hz(60).unwrap();
        assert_eq!(fixed.step(), Duration::from_nanos(16_666_666));
    }

    #[test]
    fn reset_clears_the_accumulator() {
        let mut fixed = FixedStep::new(MS(10)).unwrap();
        fixed.advance(MS(7));
        fixed.reset();
        assert_eq!(fixed.accumulated(), Duration::ZERO);
    }
}
