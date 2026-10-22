//! Which feature works how well on which platform.
//!
//! [`support`] is the single source of truth for the matrix in the crate
//! documentation; a test checks it against what the functions really do on the
//! platform the tests run on.

use ccore::platform::Platform;

/// A queryable piece of platform functionality.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[non_exhaustive]
pub enum Feature {
    /// [`os_version`](crate::info::os_version).
    OsVersion,
    /// [`CpuInfo::logical_cores`](crate::info::CpuInfo::logical_cores).
    LogicalCores,
    /// [`CpuInfo::physical_cores`](crate::info::CpuInfo::physical_cores).
    PhysicalCores,
    /// [`total_memory_bytes`](crate::info::total_memory_bytes).
    TotalMemory,
    /// [`display_server`](crate::info::display_server).
    DisplayServer,
    /// The monotonic [`Clock`](crate::time::Clock) and the
    /// [`WallClock`](crate::time::WallClock).
    Clocks,
    /// [`precise_sleep`](crate::time::precise_sleep).
    PreciseSleep,
    /// [`AppDirs`](crate::paths::AppDirs): config, data, cache, log, temp,
    /// home and executable directories.
    StandardDirs,
    /// The [`env`](crate::env) helpers and [`process::id`](crate::process::id)
    /// / [`process::current_exe`](crate::process::current_exe).
    Environment,
    /// [`process::open_url`](crate::process::open_url).
    OpenUrl,
    /// [`process::reveal_in_file_manager`](crate::process::reveal_in_file_manager).
    RevealInFileManager,
    /// [`thread::set_name`](crate::thread::set_name).
    ThreadName,
    /// [`thread::set_priority`](crate::thread::set_priority).
    ThreadPriority,
}

/// How well a [`Feature`] works on a platform.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Support {
    /// Implemented and expected to work.
    Full,
    /// Implemented, but with documented gaps or best-effort data (the feature
    /// may return nothing on some systems).
    Partial,
    /// Not implemented: the call returns
    /// [`PlatformError::Unsupported`](crate::error::PlatformError::Unsupported)
    /// or nothing.
    Unsupported,
}

/// The support level of `feature` on `platform`.
#[must_use]
pub const fn support(feature: Feature, platform: Platform) -> Support {
    use Platform::{Android, IOS, Linux, MacOS, Unknown, Windows};
    use Support::{Full, Partial, Unsupported};
    match (feature, platform) {
        (Feature::LogicalCores, Unknown) => Partial,
        (Feature::LogicalCores, _) => Full,

        (Feature::PhysicalCores, Windows | MacOS | IOS) => Full,
        // Read from sysfs, which containers and some Android builds hide.
        (Feature::PhysicalCores, Linux | Android) => Partial,
        (Feature::PhysicalCores, Unknown) => Unsupported,

        (Feature::TotalMemory, Unknown) => Unsupported,
        (Feature::TotalMemory, _) => Full,

        // Android reads a system property and falls back to the kernel release.
        (Feature::OsVersion, Android) => Partial,
        (Feature::OsVersion, Unknown) => Unsupported,
        (Feature::OsVersion, _) => Full,

        // Linux is detected from environment variables only.
        (Feature::DisplayServer, Linux) => Partial,
        (Feature::DisplayServer, Unknown) => Unsupported,
        (Feature::DisplayServer, _) => Full,

        // Built on std only, so it works on every platform.
        (Feature::Clocks, _) => Full,
        // Without an OS backend the sleep part is a plain `thread::sleep`.
        (Feature::PreciseSleep, Unknown) => Partial,
        (Feature::PreciseSleep, _) => Full,

        (Feature::StandardDirs, Windows | Linux | MacOS) => Full,
        // No such convention on mobile (app sandboxes), no backend elsewhere.
        (Feature::StandardDirs, Android | IOS | Unknown) => Unsupported,

        // Built on std only.
        (Feature::Environment, _) => Full,

        (Feature::OpenUrl, Windows | MacOS) => Full,
        // Needs `xdg-open`, which minimal systems and containers lack.
        (Feature::OpenUrl, Linux) => Partial,
        (Feature::OpenUrl, Android | IOS | Unknown) => Unsupported,

        (Feature::RevealInFileManager, Windows | MacOS) => Full,
        // Opens the folder; there is no portable way to select the file.
        (Feature::RevealInFileManager, Linux) => Partial,
        (Feature::RevealInFileManager, Android | IOS | Unknown) => Unsupported,

        (Feature::ThreadName, Windows | Linux | MacOS) => Full,
        // Implemented, but never run on a device.
        (Feature::ThreadName, Android | IOS) => Partial,
        (Feature::ThreadName, Unknown) => Unsupported,

        (Feature::ThreadPriority, Windows | MacOS) => Full,
        // Raising the priority needs privileges (CAP_SYS_NICE).
        (Feature::ThreadPriority, Linux) => Partial,
        (Feature::ThreadPriority, Android | IOS) => Partial,
        (Feature::ThreadPriority, Unknown) => Unsupported,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::PlatformError;
    use crate::info::{self, DisplayServer};
    use crate::paths::AppDirs;
    use crate::time::{Clock, TimeSource, WallClock, precise_sleep};
    use std::time::{Duration, Instant};

    const ALL_FEATURES: [Feature; 13] = [
        Feature::OsVersion,
        Feature::LogicalCores,
        Feature::PhysicalCores,
        Feature::TotalMemory,
        Feature::DisplayServer,
        Feature::Clocks,
        Feature::PreciseSleep,
        Feature::StandardDirs,
        Feature::Environment,
        Feature::OpenUrl,
        Feature::RevealInFileManager,
        Feature::ThreadName,
        Feature::ThreadPriority,
    ];

    #[test]
    fn matrix_is_defined_for_every_pair() {
        let platforms = [
            Platform::Windows,
            Platform::Linux,
            Platform::MacOS,
            Platform::Android,
            Platform::IOS,
            Platform::Unknown,
        ];
        for feature in ALL_FEATURES {
            for platform in platforms {
                let _ = support(feature, platform);
            }
        }
        // Spot checks of the documented table.
        assert_eq!(
            support(Feature::PhysicalCores, Platform::Windows),
            Support::Full
        );
        assert_eq!(
            support(Feature::PhysicalCores, Platform::Linux),
            Support::Partial
        );
        assert_eq!(
            support(Feature::TotalMemory, Platform::Unknown),
            Support::Unsupported
        );
    }

    /// `Full` must really work and `Unsupported` must really fail on the
    /// platform the tests run on (Windows, Linux and macOS in CI).
    #[test]
    fn claims_match_behaviour_on_this_platform() {
        let platform = Platform::current();
        for feature in ALL_FEATURES {
            let works = match feature {
                Feature::OsVersion => info::os_version().is_ok(),
                Feature::LogicalCores => info::cpu_info().logical_cores >= 1,
                Feature::PhysicalCores => info::cpu_info().physical_cores.is_some(),
                Feature::TotalMemory => info::total_memory_bytes().is_ok_and(|b| b > 0),
                Feature::DisplayServer => info::display_server() != DisplayServer::Unknown,
                Feature::Clocks => {
                    WallClock.unix_time().is_ok() && Clock::new().now() < Duration::from_secs(60)
                }
                Feature::PreciseSleep => {
                    let start = Instant::now();
                    precise_sleep(Duration::from_millis(1));
                    start.elapsed() >= Duration::from_millis(1)
                }
                Feature::StandardDirs => AppDirs::new("support-check")
                    .and_then(|dirs| dirs.config())
                    .is_ok(),
                Feature::Environment => {
                    crate::process::id() > 0 && crate::process::current_exe().is_ok()
                }
                // Neither may start a launcher in a test, so this checks that
                // the call is implemented (it rejects bad input) rather than
                // that a launcher is installed; that is why Linux is Partial.
                Feature::OpenUrl => matches!(
                    crate::process::open_url("javascript:alert(1)"),
                    Err(PlatformError::Io(ref e)) if e.kind() == std::io::ErrorKind::InvalidInput
                ),
                Feature::RevealInFileManager => matches!(
                    crate::process::reveal_in_file_manager(camino::Utf8Path::new(
                        "/definitely/not/here/coreverse"
                    )),
                    Err(PlatformError::Io(ref e)) if e.kind() == std::io::ErrorKind::NotFound
                ),
                Feature::ThreadName => {
                    std::thread::spawn(|| crate::thread::set_name("support-check").is_ok())
                        .join()
                        .unwrap_or(false)
                }
                Feature::ThreadPriority => std::thread::spawn(|| {
                    crate::thread::set_priority(crate::thread::ThreadPriority::Normal).is_ok()
                })
                .join()
                .unwrap_or(false),
            };
            match support(feature, platform) {
                Support::Full => assert!(works, "{feature:?} is Full on {platform} but failed"),
                Support::Unsupported => {
                    assert!(
                        !works,
                        "{feature:?} is Unsupported on {platform} but worked"
                    );
                }
                Support::Partial => {}
            }
        }
    }
}
