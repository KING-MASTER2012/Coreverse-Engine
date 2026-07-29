use crate::error::CoreError;
use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

/// A semantic version (`major.minor.patch`).
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub struct Version {
    /// Major version
    pub major: u32,
    /// Minor version
    pub minor: u32,
    /// Patch version
    pub patch: u32,
}

impl Version {
    /// Function that creates new version
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Whether `other` is API-compatible with `self` under semver rules:
    /// same major version, and not older.
    pub fn is_compatible_with(&self, other: &Version) -> bool {
        self.major == other.major && self <= other
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch))
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl FromStr for Version {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.trim().splitn(3, '.');
        let mut next = |name: &str| -> Result<u32, CoreError> {
            parts
                .next()
                .ok_or_else(|| {
                    CoreError::InvalidArgument(format!("missing {name} in version '{s}'"))
                })?
                .parse::<u32>()
                .map_err(|_| CoreError::InvalidArgument(format!("invalid {name} in version '{s}'")))
        };

        let major = next("major")?;
        let minor = next("minor")?;
        let patch = next("patch")?;

        Ok(Self {
            major,
            minor,
            patch,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_displays() {
        let v: Version = "1.4.2".parse().unwrap();
        assert_eq!(v, Version::new(1, 4, 2));
        assert_eq!(v.to_string(), "1.4.2");
    }

    #[test]
    fn rejects_malformed_input() {
        assert!("1.4".parse::<Version>().is_err());
        assert!("1.x.2".parse::<Version>().is_err());
    }

    #[test]
    fn compatibility_requires_same_major_and_not_older() {
        let base = Version::new(1, 2, 0);
        assert!(base.is_compatible_with(&Version::new(1, 3, 0)));
        assert!(!base.is_compatible_with(&Version::new(2, 0, 0)));
        assert!(!base.is_compatible_with(&Version::new(1, 1, 0)));
    }
}
