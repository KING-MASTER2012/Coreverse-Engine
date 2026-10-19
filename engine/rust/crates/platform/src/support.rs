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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::info::{self, DisplayServer};

    const ALL_FEATURES: [Feature; 5] = [
        Feature::OsVersion,
        Feature::LogicalCores,
        Feature::PhysicalCores,
        Feature::TotalMemory,
        Feature::DisplayServer,
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
