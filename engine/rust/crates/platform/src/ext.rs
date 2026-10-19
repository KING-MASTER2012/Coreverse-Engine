//! Convenience queries on [`Platform`].

use ccore::platform::Platform;

/// Classification helpers for [`Platform`]. They live here (not in `ccore`)
/// because they are policy for the platform layer, not identity.
pub trait PlatformExt: Copy {
    /// Windows, Linux or macOS.
    fn is_desktop(self) -> bool;
    /// Android or iOS.
    fn is_mobile(self) -> bool;
    /// Unix-like: Linux, macOS, Android and iOS.
    fn is_unix(self) -> bool;
}

impl PlatformExt for Platform {
    fn is_desktop(self) -> bool {
        matches!(self, Platform::Windows | Platform::Linux | Platform::MacOS)
    }

    fn is_mobile(self) -> bool {
        matches!(self, Platform::Android | Platform::IOS)
    }

    fn is_unix(self) -> bool {
        matches!(
            self,
            Platform::Linux | Platform::MacOS | Platform::Android | Platform::IOS
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_table() {
        let cases = [
            (Platform::Windows, true, false, false),
            (Platform::Linux, true, false, true),
            (Platform::MacOS, true, false, true),
            (Platform::Android, false, true, true),
            (Platform::IOS, false, true, true),
            (Platform::Unknown, false, false, false),
        ];
        for (platform, desktop, mobile, unix) in cases {
            assert_eq!(platform.is_desktop(), desktop, "{platform} desktop");
            assert_eq!(platform.is_mobile(), mobile, "{platform} mobile");
            assert_eq!(platform.is_unix(), unix, "{platform} unix");
        }
    }

    #[test]
    fn current_platform_matches_cfg_unix() {
        assert_eq!(Platform::current().is_unix(), cfg!(unix));
    }
}
