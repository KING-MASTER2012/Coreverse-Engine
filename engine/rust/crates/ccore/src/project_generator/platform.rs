use serde::{Deserialize, Serialize};

/// A build target platform. `Build/` gets one subfolder per platform,
/// named via [`Platform::dir_name`] (note the conventional casing:
/// `macOS`, `iOS`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Platform {
    Windows,
    Linux,
    MacOS,
    Android,
    IOS,
}

impl Platform {
    /// Every platform the generator scaffolds by default.
    pub const ALL: [Platform; 5] = [
        Platform::Windows,
        Platform::Linux,
        Platform::MacOS,
        Platform::Android,
        Platform::IOS,
    ];

    /// Directory name under `Build/`.
    pub fn dir_name(self) -> &'static str {
        match self {
            Platform::Windows => "Windows",
            Platform::Linux => "Linux",
            Platform::MacOS => "macOS",
            Platform::Android => "Android",
            Platform::IOS => "iOS",
        }
    }

    /// A reasonable starting target triple for `Build.toml` - meant to be
    /// edited per-project (e.g. once you decide on MSVC vs GNU, or which
    /// Apple silicon/Android ABI to target).
    pub fn default_target_triple(self) -> &'static str {
        match self {
            Platform::Windows => "x86_64-pc-windows-msvc",
            Platform::Linux => "x86_64-unknown-linux-gnu",
            Platform::MacOS => "aarch64-apple-darwin",
            Platform::Android => "aarch64-linux-android",
            Platform::IOS => "aarch64-apple-ios",
        }
    }
}
