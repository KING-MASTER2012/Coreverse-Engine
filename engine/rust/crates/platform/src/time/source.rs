//! Time sources: the real monotonic clock, the wall clock and a manual clock.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::error::{PlatformError, Result};

/// A monotonic time source.
///
/// `now` returns the time elapsed since an arbitrary but fixed origin of the
/// source. Only differences between two readings of the same source mean
/// anything. Timers in this module are generic over this trait so that tests
/// can substitute [`ManualClock`].
pub trait TimeSource {
    /// Time since the source's origin.
    fn now(&self) -> Duration;
}

impl<T: TimeSource + ?Sized> TimeSource for &T {
    fn now(&self) -> Duration {
        (**self).now()
    }
}

impl<T: TimeSource + ?Sized> TimeSource for Arc<T> {
    fn now(&self) -> Duration {
        (**self).now()
    }
}

/// The real monotonic clock, backed by [`Instant`] (`QueryPerformanceCounter`,
/// `clock_gettime(CLOCK_MONOTONIC)` or `mach_absolute_time`).
///
/// The origin is the moment the clock was created; copies of a clock share it,
/// so their readings are comparable.
#[derive(Copy, Clone, Debug)]
pub struct Clock {
    origin: Instant,
}

impl Clock {
    /// A clock whose origin is now.
    #[must_use]
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeSource for Clock {
    fn now(&self) -> Duration {
        self.origin.elapsed()
    }
}

/// Calendar time. Unlike [`Clock`] it can jump (NTP, the user changing the
/// time), so use it for timestamps, never for measuring durations.
#[derive(Copy, Clone, Debug, Default)]
pub struct WallClock;

impl WallClock {
    /// Time since the Unix epoch (1970-01-01T00:00:00Z).
    ///
    /// # Errors
    ///
    /// [`PlatformError::Os`] when the system clock is set before the epoch.
    pub fn unix_time(&self) -> Result<Duration> {
        SystemTime::now().duration_since(UNIX_EPOCH).map_err(|err| {
            PlatformError::unusable(
                "SystemTime",
                format!(
                    "the system clock is {:?} before the Unix epoch",
                    err.duration()
                ),
            )
        })
    }
}

/// A clock that only moves when told to. For tests.
///
/// It is `Sync`, so a test can keep one handle (or an [`Arc`]/reference, both
/// of which are [`TimeSource`]s) while the timer under test owns another.
#[derive(Debug, Default)]
pub struct ManualClock {
    nanos: AtomicU64,
}

impl ManualClock {
    /// A clock at time zero.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Moves the clock forward, saturating at roughly 584 years.
    pub fn advance(&self, by: Duration) {
        let by = saturating_nanos(by);
        // The closure always returns `Some`, so `fetch_update` cannot fail.
        let _ = self
            .nanos
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
                Some(n.saturating_add(by))
            });
    }

    /// Sets the clock to an absolute time. Going backwards is allowed so
    /// tests can check that timers tolerate it.
    pub fn set(&self, to: Duration) {
        self.nanos.store(saturating_nanos(to), Ordering::Relaxed);
    }
}

fn saturating_nanos(d: Duration) -> u64 {
    u64::try_from(d.as_nanos()).unwrap_or(u64::MAX)
}

impl TimeSource for ManualClock {
    fn now(&self) -> Duration {
        Duration::from_nanos(self.nanos.load(Ordering::Relaxed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_is_monotonic() {
        let clock = Clock::new();
        let a = clock.now();
        let b = clock.now();
        assert!(b >= a);
    }

    #[test]
    fn clock_copies_share_the_origin() {
        let clock = Clock::new();
        let copy = clock;
        std::thread::sleep(Duration::from_millis(2));
        // Both read the same origin, so the later reading is never smaller.
        let a = clock.now();
        let b = copy.now();
        assert!(b >= a);
        assert!(a >= Duration::from_millis(2));
    }

    #[test]
    fn wall_clock_is_after_2020() {
        // 2020-01-01T00:00:00Z
        let t = WallClock.unix_time().expect("system clock after the epoch");
        assert!(t > Duration::from_secs(1_577_836_800));
    }

    #[test]
    fn manual_clock_moves_only_when_told() {
        let clock = ManualClock::new();
        assert_eq!(clock.now(), Duration::ZERO);
        clock.advance(Duration::from_millis(5));
        clock.advance(Duration::from_millis(7));
        assert_eq!(clock.now(), Duration::from_millis(12));
        clock.set(Duration::from_secs(1));
        assert_eq!(clock.now(), Duration::from_secs(1));
    }

    #[test]
    fn manual_clock_saturates() {
        let clock = ManualClock::new();
        clock.advance(Duration::MAX);
        clock.advance(Duration::from_secs(1));
        assert_eq!(clock.now(), Duration::from_nanos(u64::MAX));
    }

    #[test]
    fn references_and_arcs_are_time_sources() {
        let clock = Arc::new(ManualClock::new());
        let by_ref: &ManualClock = &clock;
        clock.advance(Duration::from_secs(3));
        assert_eq!(by_ref.now(), Duration::from_secs(3));
        assert_eq!(TimeSource::now(&clock), Duration::from_secs(3));
    }
}
