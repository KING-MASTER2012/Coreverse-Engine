//! A stopwatch over any [`TimeSource`].

use std::time::Duration;

use super::source::{Clock, TimeSource};

/// Measures elapsed time; can be paused and resumed. Elapsed time accumulates
/// across start/stop cycles until [`reset`](Self::reset).
#[derive(Clone, Debug)]
pub struct Stopwatch<S: TimeSource = Clock> {
    source: S,
    accumulated: Duration,
    /// Source reading at the last `start`, while running.
    started_at: Option<Duration>,
}

impl Stopwatch<Clock> {
    /// A running stopwatch on a new real [`Clock`].
    #[must_use]
    pub fn start_new() -> Self {
        Self::started(Clock::new())
    }
}

impl<S: TimeSource> Stopwatch<S> {
    /// A stopped stopwatch at zero.
    #[must_use]
    pub fn new(source: S) -> Self {
        Self {
            source,
            accumulated: Duration::ZERO,
            started_at: None,
        }
    }

    /// A stopwatch that is already running.
    #[must_use]
    pub fn started(source: S) -> Self {
        let mut watch = Self::new(source);
        watch.start();
        watch
    }

    /// Starts or resumes; does nothing when already running.
    pub fn start(&mut self) {
        if self.started_at.is_none() {
            self.started_at = Some(self.source.now());
        }
    }

    /// Pauses, keeping the elapsed time; does nothing when stopped.
    pub fn stop(&mut self) {
        if self.started_at.is_some() {
            self.accumulated = self.elapsed();
            self.started_at = None;
        }
    }

    /// Stops and clears the elapsed time.
    pub fn reset(&mut self) {
        self.accumulated = Duration::ZERO;
        self.started_at = None;
    }

    /// Clears the elapsed time and starts running.
    pub fn restart(&mut self) {
        self.reset();
        self.start();
    }

    /// Whether the stopwatch is running.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.started_at.is_some()
    }

    /// Total elapsed time. A source that moves backwards counts as no time.
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        let running = self.started_at.map_or(Duration::ZERO, |start| {
            self.source.now().saturating_sub(start)
        });
        self.accumulated.saturating_add(running)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::ManualClock;

    #[test]
    fn counts_only_while_running() {
        let clock = ManualClock::new();
        let mut watch = Stopwatch::new(&clock);
        clock.advance(Duration::from_secs(1));
        assert_eq!(watch.elapsed(), Duration::ZERO);
        assert!(!watch.is_running());

        watch.start();
        clock.advance(Duration::from_secs(2));
        assert_eq!(watch.elapsed(), Duration::from_secs(2));

        watch.stop();
        clock.advance(Duration::from_secs(10));
        assert_eq!(watch.elapsed(), Duration::from_secs(2));

        watch.start();
        clock.advance(Duration::from_secs(3));
        assert_eq!(watch.elapsed(), Duration::from_secs(5));
    }

    #[test]
    fn start_and_stop_are_idempotent() {
        let clock = ManualClock::new();
        let mut watch = Stopwatch::started(&clock);
        clock.advance(Duration::from_secs(1));
        watch.start(); // must not restart the measurement
        clock.advance(Duration::from_secs(1));
        assert_eq!(watch.elapsed(), Duration::from_secs(2));
        watch.stop();
        watch.stop();
        assert_eq!(watch.elapsed(), Duration::from_secs(2));
    }

    #[test]
    fn reset_and_restart() {
        let clock = ManualClock::new();
        let mut watch = Stopwatch::started(&clock);
        clock.advance(Duration::from_secs(4));
        watch.reset();
        assert!(!watch.is_running());
        assert_eq!(watch.elapsed(), Duration::ZERO);

        watch.restart();
        clock.advance(Duration::from_secs(1));
        assert!(watch.is_running());
        assert_eq!(watch.elapsed(), Duration::from_secs(1));
    }

    #[test]
    fn tolerates_a_source_going_backwards() {
        let clock = ManualClock::new();
        clock.set(Duration::from_secs(10));
        let watch = Stopwatch::started(&clock);
        clock.set(Duration::from_secs(5));
        assert_eq!(watch.elapsed(), Duration::ZERO);
    }

    #[test]
    fn real_clock_stopwatch_runs() {
        let watch = Stopwatch::start_new();
        std::thread::sleep(Duration::from_millis(2));
        assert!(watch.elapsed() >= Duration::from_millis(2));
    }
}
