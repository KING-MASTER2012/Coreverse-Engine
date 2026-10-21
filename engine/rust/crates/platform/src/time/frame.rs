//! Per-frame timing.

use std::time::Duration;

use super::source::{Clock, TimeSource};

/// Smoothing factor used unless [`FrameTimer::with_smoothing`] says otherwise.
const DEFAULT_SMOOTHING: f64 = 0.1;

/// Measures the time between frames and a smoothed frame rate.
///
/// Call [`tick`](Self::tick) once per frame. The smoothed rate is an
/// exponential moving average of the frame time, so one slow frame does not
/// make a displayed FPS counter jump.
#[derive(Clone, Debug)]
pub struct FrameTimer<S: TimeSource = Clock> {
    source: S,
    last: Duration,
    raw_delta: Duration,
    delta: Duration,
    /// Moving average of the frame time in seconds; `None` before a sample.
    average: Option<f64>,
    smoothing: f64,
    max_delta: Option<Duration>,
    frames: u64,
}

impl FrameTimer<Clock> {
    /// A timer on a new real [`Clock`].
    #[must_use]
    pub fn start_new() -> Self {
        Self::new(Clock::new())
    }
}

impl<S: TimeSource> FrameTimer<S> {
    /// A timer whose first frame starts now.
    #[must_use]
    pub fn new(source: S) -> Self {
        let last = source.now();
        Self {
            source,
            last,
            raw_delta: Duration::ZERO,
            delta: Duration::ZERO,
            average: None,
            smoothing: DEFAULT_SMOOTHING,
            max_delta: None,
            frames: 0,
        }
    }

    /// Sets the weight of the newest frame in the moving average, clamped to
    /// `(0, 1]`: `1.0` disables smoothing, small values smooth heavily. A NaN
    /// keeps the default of `0.1`.
    #[must_use]
    pub fn with_smoothing(mut self, factor: f64) -> Self {
        if !factor.is_nan() {
            self.smoothing = factor.clamp(f64::MIN_POSITIVE, 1.0);
        }
        self
    }

    /// Caps [`delta`](Self::delta) at `max`, so that a long stall (a
    /// breakpoint, a window drag) does not become one huge simulation step.
    /// FPS figures keep using the real frame time.
    #[must_use]
    pub fn with_max_delta(mut self, max: Duration) -> Self {
        self.max_delta = Some(max);
        self
    }

    /// Ends the current frame and starts the next. Returns the new
    /// [`delta`](Self::delta).
    pub fn tick(&mut self) -> Duration {
        let now = self.source.now();
        self.raw_delta = now.saturating_sub(self.last);
        self.last = now;
        self.delta = self
            .max_delta
            .map_or(self.raw_delta, |max| self.raw_delta.min(max));
        self.frames = self.frames.saturating_add(1);

        // Two ticks at the same instant carry no rate information.
        if !self.raw_delta.is_zero() {
            let sample = self.raw_delta.as_secs_f64();
            self.average = Some(match self.average {
                None => sample,
                Some(average) => average + self.smoothing * (sample - average),
            });
        }
        self.delta
    }

    /// Duration of the last frame, capped by
    /// [`with_max_delta`](Self::with_max_delta). Zero before the first tick.
    #[must_use]
    pub fn delta(&self) -> Duration {
        self.delta
    }

    /// Duration of the last frame, uncapped.
    #[must_use]
    pub fn raw_delta(&self) -> Duration {
        self.raw_delta
    }

    /// Frames per second of the last frame alone; `0.0` when unknown.
    #[must_use]
    pub fn fps(&self) -> f64 {
        rate(self.raw_delta.as_secs_f64())
    }

    /// Frames per second from the moving average; `0.0` before two ticks have
    /// produced a measurable frame time.
    #[must_use]
    pub fn smoothed_fps(&self) -> f64 {
        self.average.map_or(0.0, rate)
    }

    /// Number of [`tick`](Self::tick) calls so far.
    #[must_use]
    pub fn frame_count(&self) -> u64 {
        self.frames
    }
}

fn rate(seconds_per_frame: f64) -> f64 {
    if seconds_per_frame > 0.0 {
        1.0 / seconds_per_frame
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::ManualClock;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn reports_delta_and_fps() {
        let clock = ManualClock::new();
        let mut timer = FrameTimer::new(&clock);
        assert_eq!(timer.delta(), Duration::ZERO);
        assert!(approx(timer.fps(), 0.0));

        clock.advance(Duration::from_millis(10));
        assert_eq!(timer.tick(), Duration::from_millis(10));
        assert_eq!(timer.frame_count(), 1);
        assert!(approx(timer.fps(), 100.0));
        assert!(approx(timer.smoothed_fps(), 100.0));
    }

    #[test]
    fn smoothing_damps_a_slow_frame() {
        let clock = ManualClock::new();
        let mut timer = FrameTimer::new(&clock).with_smoothing(0.5);
        for _ in 0..4 {
            clock.advance(Duration::from_millis(10));
            timer.tick();
        }
        clock.advance(Duration::from_millis(30));
        timer.tick();

        // Average moves halfway from 10 ms to 30 ms.
        assert!(approx(timer.fps(), 1000.0 / 30.0));
        assert!(approx(timer.smoothed_fps(), 1000.0 / 20.0));
    }

    #[test]
    fn max_delta_caps_delta_but_not_fps() {
        let clock = ManualClock::new();
        let mut timer = FrameTimer::new(&clock).with_max_delta(Duration::from_millis(50));
        clock.advance(Duration::from_secs(1));
        assert_eq!(timer.tick(), Duration::from_millis(50));
        assert_eq!(timer.raw_delta(), Duration::from_secs(1));
        assert!(approx(timer.fps(), 1.0));
    }

    #[test]
    fn zero_length_frames_do_not_divide_by_zero() {
        let clock = ManualClock::new();
        let mut timer = FrameTimer::new(&clock);
        timer.tick();
        timer.tick();
        assert!(approx(timer.fps(), 0.0));
        assert!(approx(timer.smoothed_fps(), 0.0));
        assert_eq!(timer.frame_count(), 2);
    }

    #[test]
    fn smoothing_factor_is_clamped() {
        let clock = ManualClock::new();
        let timer = FrameTimer::new(&clock).with_smoothing(7.0);
        assert!(approx(timer.smoothing, 1.0));
        let timer = FrameTimer::new(&clock).with_smoothing(-1.0);
        assert!(timer.smoothing > 0.0);
        let timer = FrameTimer::new(&clock).with_smoothing(f64::NAN);
        assert!(approx(timer.smoothing, DEFAULT_SMOOTHING));
    }
}
