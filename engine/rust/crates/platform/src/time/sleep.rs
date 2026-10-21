//! Sleeping with sub-millisecond accuracy.

use std::time::{Duration, Instant};

use crate::sys;

/// How long before the deadline the OS sleep ends and spinning takes over.
///
/// Sleeping hands the thread back to the scheduler, which wakes it up late by
/// an amount that depends on the platform (about half a millisecond with
/// Windows' high-resolution timer, up to a millisecond or more elsewhere).
/// Spinning through that last stretch trades a little CPU for precision.
const SPIN_MARGIN: Duration = if cfg!(windows) {
    Duration::from_millis(1)
} else {
    Duration::from_millis(2)
};

/// Sleeps for `duration`, accurately: an OS sleep for all but the last
/// couple of milliseconds, then a spin loop. Never returns early.
///
/// On Windows the sleep uses a high-resolution waitable timer (Windows 10
/// 1803 and later). Without it, the sleep falls back to
/// [`std::thread::sleep`], whose granularity there is about 15 ms: the call
/// then still never returns early, but may overshoot.
pub fn precise_sleep(duration: Duration) {
    match Instant::now().checked_add(duration) {
        Some(deadline) => precise_sleep_until(deadline),
        // Too far in the future to represent: nothing to be precise about.
        None => sys::sleep_coarse(duration),
    }
}

/// Like [`precise_sleep`], but until `deadline`. Returns at once when the
/// deadline has passed. Sleeping to a deadline instead of for a duration
/// keeps a frame limiter from drifting.
pub fn precise_sleep_until(deadline: Instant) {
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return;
        }
        if remaining > SPIN_MARGIN {
            sys::sleep_coarse(remaining - SPIN_MARGIN);
        } else {
            std::hint::spin_loop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_returns_early() {
        for millis in [0_u64, 1, 3, 12] {
            let wanted = Duration::from_millis(millis);
            let start = Instant::now();
            precise_sleep(wanted);
            assert!(start.elapsed() >= wanted, "slept less than {millis} ms");
        }
    }

    #[test]
    fn finishes_in_reasonable_time() {
        // A very loose bound: this only catches a loop that never ends.
        let start = Instant::now();
        precise_sleep(Duration::from_millis(5));
        assert!(start.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn past_deadline_returns_immediately() {
        let start = Instant::now();
        precise_sleep_until(start);
        assert!(start.elapsed() < Duration::from_secs(1));
    }
}
