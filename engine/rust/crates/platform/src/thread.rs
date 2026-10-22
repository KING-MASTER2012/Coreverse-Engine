//! Thread names and priorities.
//!
//! Both act on the *calling* thread, because that is what every OS offers
//! without extra privileges: a thread names itself and sets its own priority
//! when it starts.

use crate::error::{PlatformError, Result};
use crate::sys;

/// How urgently a thread wants the CPU. The levels are abstract; each OS maps
/// them to its own mechanism (see [`set_priority`]).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub enum ThreadPriority {
    /// Background work that should not get in the way (asset streaming,
    /// compression).
    Low,
    /// The default of a new thread.
    #[default]
    Normal,
    /// Latency-sensitive work (the game loop, the render thread).
    High,
    /// Work that must not miss a deadline (audio mixing). Use sparingly: such
    /// a thread can starve everything else.
    TimeCritical,
}

/// Names the calling thread for debuggers, profilers and crash reports. This
/// is the OS-level name; [`std::thread::Builder::name`] already sets it for
/// threads spawned by Rust, so this is mainly for the main thread and for
/// threads created elsewhere.
///
/// The name is cut to what the OS accepts, on a character boundary: 15 bytes
/// on Linux and Android, 63 on macOS and iOS, no practical limit on Windows
/// (which needs Windows 10 1607 or later).
///
/// # Errors
///
/// * [`PlatformError::Io`] with
///   [`InvalidInput`](std::io::ErrorKind::InvalidInput) when `name` contains a
///   NUL character.
/// * [`PlatformError::Unsupported`] on platforms without a backend.
/// * [`PlatformError::Os`] when the OS refuses.
pub fn set_name(name: &str) -> Result<()> {
    if name.contains('\0') {
        return Err(PlatformError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "thread name contains a NUL character",
        )));
    }
    sys::set_thread_name(name)
}

/// Sets the priority of the calling thread.
///
/// | Level          | Windows                       | macOS and iOS (QoS)   | Linux and Android (nice) |
/// | :------------- | :---------------------------- | :-------------------- | :----------------------- |
/// | `Low`          | `THREAD_PRIORITY_LOWEST`      | `UTILITY`             | 10                       |
/// | `Normal`       | `THREAD_PRIORITY_NORMAL`      | `DEFAULT`             | 0                        |
/// | `High`         | `THREAD_PRIORITY_HIGHEST`     | `USER_INITIATED`      | -5                       |
/// | `TimeCritical` | `THREAD_PRIORITY_TIME_CRITICAL` | `USER_INTERACTIVE`  | -10                      |
///
/// On Linux a thread may always make itself *nicer* (`Low`) but needs
/// `CAP_SYS_NICE` or a matching `RLIMIT_NICE` to be less nice, which includes
/// returning from `Low` to `Normal`. Without it the call fails with
/// [`PlatformError::Io`] of kind
/// [`PermissionDenied`](std::io::ErrorKind::PermissionDenied) and the
/// priority is unchanged; callers should treat that as "no boost available",
/// not as a fatal error.
///
/// # Errors
///
/// * [`PlatformError::Io`] with kind `PermissionDenied`, as described above.
/// * [`PlatformError::Unsupported`] on platforms without a backend.
/// * [`PlatformError::Os`] when the OS refuses for another reason.
pub fn set_priority(priority: ThreadPriority) -> Result<()> {
    sys::set_thread_priority(priority)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{Feature, Support, support};
    use ccore::platform::Platform;

    fn supported(feature: Feature) -> bool {
        support(feature, Platform::current()) != Support::Unsupported
    }

    /// Runs `f` on a fresh thread, so one test's name or priority cannot
    /// leak into another's.
    fn on_new_thread<R: Send + 'static>(f: impl FnOnce() -> R + Send + 'static) -> R {
        std::thread::spawn(f).join().expect("test thread panicked")
    }

    #[test]
    fn default_priority_is_normal_and_levels_are_ordered() {
        assert_eq!(ThreadPriority::default(), ThreadPriority::Normal);
        assert!(ThreadPriority::Low < ThreadPriority::Normal);
        assert!(ThreadPriority::Normal < ThreadPriority::High);
        assert!(ThreadPriority::High < ThreadPriority::TimeCritical);
    }

    #[test]
    fn nul_in_a_name_is_rejected_on_every_platform() {
        let err = set_name("a\0b").unwrap_err();
        match err {
            PlatformError::Io(io) => assert_eq!(io.kind(), std::io::ErrorKind::InvalidInput),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn naming_works_where_supported() {
        let result = on_new_thread(|| set_name("coreverse-test"));
        if supported(Feature::ThreadName) {
            result.unwrap();
        } else {
            assert!(matches!(result, Err(PlatformError::Unsupported { .. })));
        }
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    #[test]
    fn the_kernel_sees_the_name_cut_to_fifteen_bytes() {
        let comm = on_new_thread(|| {
            set_name("a-rather-long-thread-name").unwrap();
            std::fs::read_to_string("/proc/thread-self/comm").unwrap()
        });
        assert_eq!(comm.trim_end(), "a-rather-long-t");

        let comm = on_new_thread(|| {
            // 'é' is two bytes; the fifteenth byte would split the last one.
            set_name("ééééééééé").unwrap();
            std::fs::read_to_string("/proc/thread-self/comm").unwrap()
        });
        assert_eq!(comm.trim_end(), "ééééééé");
    }

    /// The nice value of the calling thread, from `/proc`.
    #[cfg(target_os = "linux")]
    fn current_nice() -> i32 {
        let stat = std::fs::read_to_string("/proc/thread-self/stat").unwrap();
        // The name in field 2 may contain spaces; everything after the last
        // ')' is plain. Field 19 (nice) is the 17th of those.
        let rest = &stat[stat.rfind(')').unwrap() + 1..];
        rest.split_whitespace().nth(16).unwrap().parse().unwrap()
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_low_priority_sets_nice_ten() {
        let nice = on_new_thread(|| {
            set_priority(ThreadPriority::Low).unwrap();
            current_nice()
        });
        assert_eq!(nice, 10);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_raising_priority_needs_privileges() {
        let outcome = on_new_thread(|| {
            set_priority(ThreadPriority::Low).unwrap();
            set_priority(ThreadPriority::Normal)
        });
        match outcome {
            // Privileged (root or CAP_SYS_NICE): allowed.
            Ok(()) => {}
            Err(PlatformError::Io(io)) => {
                assert_eq!(io.kind(), std::io::ErrorKind::PermissionDenied);
            }
            Err(other) => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn normal_priority_is_accepted_on_a_fresh_thread() {
        // A new thread already has Normal priority; asking for it again must
        // not need any privilege.
        if cfg!(target_os = "linux") || !supported(Feature::ThreadPriority) {
            return;
        }
        on_new_thread(|| set_priority(ThreadPriority::Normal).unwrap());
    }

    #[test]
    fn unsupported_platforms_report_it() {
        if supported(Feature::ThreadPriority) {
            return;
        }
        assert!(matches!(
            set_priority(ThreadPriority::High),
            Err(PlatformError::Unsupported { .. })
        ));
    }
}
