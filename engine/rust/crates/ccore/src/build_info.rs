use std::fmt;

/// Compile-time build metadata for an executable or crate.
///
/// `ccore` doesn't populate this itself — the values normally come from a
/// consuming crate's `build.rs` (e.g. emitting `cargo:rustc-env=GIT_HASH=...`)
/// combined with `env!(...)`/`option_env!(...)` at the call site. Use
/// [`BuildInfo::new`] directly, or the [`build_info!`] macro to assemble
/// one from the usual environment variables with `"unknown"` fallbacks.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct BuildInfo {
    /// Git Hash
    pub git_hash: &'static str,
    /// Build Date
    pub build_date: &'static str,
    /// Profile
    pub profile: &'static str,
    /// Target
    pub target: &'static str,
}

impl BuildInfo {
    /// Function that creates a new BuildInfo
    pub const fn new(
        git_hash: &'static str,
        build_date: &'static str,
        profile: &'static str,
        target: &'static str,
    ) -> Self {
        Self {
            git_hash,
            build_date,
            profile,
            target,
        }
    }
}

impl fmt::Display for BuildInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}, {}, {})",
            self.git_hash, self.profile, self.target, self.build_date
        )
    }
}

/// Builds a [`BuildInfo`] from environment variables set at compile time,
/// falling back to `"unknown"` for anything not set.
///
/// Expects a `build.rs` in the *consuming* crate to have emitted
/// `GIT_HASH` and `BUILD_DATE` via `cargo:rustc-env=...` (neither is
/// available to compiled code otherwise). `PROFILE` and `TARGET` follow
/// the same pattern - they're visible to `build.rs` itself but need to be
/// re-emitted the same way to reach the compiled binary.
#[macro_export]
macro_rules! build_info {
    () => {
        $crate::BuildInfo::new(
            option_env!("GIT_HASH").unwrap_or("unknown"),
            option_env!("BUILD_DATE").unwrap_or("unknown"),
            option_env!("PROFILE").unwrap_or("unknown"),
            option_env!("TARGET").unwrap_or("unknown"),
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays_readable_summary() {
        let info = BuildInfo::new("abc123", "2026-07-29", "release", "x86_64-pc-windows-msvc");
        assert_eq!(
            info.to_string(),
            "abc123 (release, x86_64-pc-windows-msvc, 2026-07-29)"
        );
    }

    #[test]
    fn macro_falls_back_to_unknown_without_build_rs() {
        let info = build_info!();
        assert_eq!(info.git_hash, "unknown");
    }
}
