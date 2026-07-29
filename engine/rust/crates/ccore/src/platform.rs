use std::fmt;

/// The operating system the code is currently compiled for.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Platform {
    /// Windows
    Windows,
    /// Linux distributions
    Linux,
    /// MacOS
    MacOS,
    /// Android
    Android,
    /// IOS
    IOS,
    /// Other
    Unknown,
}

impl Platform {
    /// Function that sets platform info
    pub const fn current() -> Self {
        if cfg!(target_os = "windows") {
            Platform::Windows
        } else if cfg!(target_os = "linux") {
            Platform::Linux
        } else if cfg!(target_os = "macos") {
            Platform::MacOS
        } else if cfg!(target_os = "android") {
            Platform::Android
        } else if cfg!(target_os = "ios") {
            Platform::IOS
        } else {
            Platform::Unknown
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Platform::Windows => "windows",
            Platform::Linux => "linux",
            Platform::MacOS => "macos",
            Platform::Android => "android",
            Platform::IOS => "ios",
            Platform::Unknown => "unknown",
        };
        write!(f, "{s}")
    }
}

/// The CPU architecture the code is currently compiled for.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Architecture {
    /// x86 Based CPU
    X86,
    /// x86_64 Based CPU
    X86_64,
    /// Arm Based CPU
    Arm,
    /// Arm64 Based CPU
    Arm64,
    /// Unknown CPUs
    Unknown,
}

impl Architecture {
    /// Function that sets architecture info
    pub const fn current() -> Self {
        if cfg!(target_arch = "x86_64") {
            Architecture::X86_64
        } else if cfg!(target_arch = "x86") {
            Architecture::X86
        } else if cfg!(target_arch = "aarch64") {
            Architecture::Arm64
        } else if cfg!(target_arch = "arm") {
            Architecture::Arm
        } else {
            Architecture::Unknown
        }
    }
}

impl fmt::Display for Architecture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Architecture::X86 => "x86",
            Architecture::X86_64 => "x86_64",
            Architecture::Arm => "arm",
            Architecture::Arm64 => "arm64",
            Architecture::Unknown => "unknown",
        };
        write!(f, "{s}")
    }
}

/// Byte order of the target platform.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Endian {
    /// Little Endian
    Little,
    /// Big Endian
    Big,
}

impl Endian {
    /// Function that sets endian info
    pub const fn current() -> Self {
        if cfg!(target_endian = "little") {
            Endian::Little
        } else {
            Endian::Big
        }
    }
}

impl fmt::Display for Endian {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Endian::Little => "little",
            Endian::Big => "big",
        };
        write!(f, "{s}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_platform_is_not_unknown_on_this_test_target() {
        // Whatever CI/dev machine this runs on is one of the three known
        // desktop OSes, so this should never fall through to Unknown here.
        assert_ne!(Platform::current(), Platform::Unknown);
    }
}
