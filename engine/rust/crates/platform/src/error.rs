//! The error type shared by every module of this crate.

use std::path::PathBuf;

use ccore::CoreError;
use ccore::platform::Platform;

/// Everything that can go wrong when talking to the operating system.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PlatformError {
    /// The feature does not exist on the platform the crate was compiled for
    /// (see [`support`](crate::support::support) for the full matrix).
    #[error("{feature} is not supported on {platform}")]
    Unsupported {
        /// Short name of the missing feature, e.g. `"total memory"`.
        feature: &'static str,
        /// The platform that lacks it.
        platform: Platform,
    },

    /// A standard-library I/O operation failed.
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),

    /// An operating-system call failed.
    #[error("{call} failed: {message} (code {code})")]
    Os {
        /// Name of the failing call, e.g. `"sysconf"`.
        call: &'static str,
        /// The OS error code (`errno` / `GetLastError`), `0` when the call
        /// succeeded but returned something unusable.
        code: i32,
        /// Human-readable description.
        message: String,
    },

    /// A path handed out by the OS is not valid UTF-8, so it cannot be
    /// represented as a `camino::Utf8PathBuf`.
    #[error("path is not valid UTF-8: {}", .0.display())]
    NonUtf8Path(PathBuf),
}

/// Shorthand for `Result<T, PlatformError>`.
pub type Result<T> = std::result::Result<T, PlatformError>;

impl PlatformError {
    /// An [`Unsupported`](Self::Unsupported) error for the current platform.
    #[allow(
        dead_code,
        reason = "each OS backend uses a different subset of these constructors"
    )]
    pub(crate) fn unsupported(feature: &'static str) -> Self {
        Self::Unsupported {
            feature,
            platform: Platform::current(),
        }
    }

    /// An [`Os`](Self::Os) error built from the calling thread's last OS error
    /// (`errno` on unix, `GetLastError` on Windows).
    #[allow(
        dead_code,
        reason = "each OS backend uses a different subset of these constructors"
    )]
    pub(crate) fn last_os_error(call: &'static str) -> Self {
        let err = std::io::Error::last_os_error();
        Self::Os {
            call,
            code: err.raw_os_error().unwrap_or(0),
            message: err.to_string(),
        }
    }

    /// An [`Os`](Self::Os) error for a call that succeeded but returned data
    /// this crate cannot use.
    #[allow(
        dead_code,
        reason = "each OS backend uses a different subset of these constructors"
    )]
    pub(crate) fn unusable(call: &'static str, message: impl Into<String>) -> Self {
        Self::Os {
            call,
            code: 0,
            message: message.into(),
        }
    }
}

impl From<PlatformError> for CoreError {
    fn from(err: PlatformError) -> Self {
        let message = err.to_string();
        match err {
            PlatformError::Unsupported { .. } => CoreError::Unsupported(message),
            PlatformError::Io(_) | PlatformError::Os { .. } => CoreError::Io(message),
            PlatformError::NonUtf8Path(_) => CoreError::InvalidArgument(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_names_feature_and_platform() {
        let err = PlatformError::unsupported("total memory");
        let text = err.to_string();
        assert!(text.contains("total memory"), "{text}");
        assert!(text.contains(&Platform::current().to_string()), "{text}");
    }

    #[test]
    fn converts_into_core_error() {
        assert!(matches!(
            CoreError::from(PlatformError::unsupported("x")),
            CoreError::Unsupported(_)
        ));
        assert!(matches!(
            CoreError::from(PlatformError::unusable("call", "bad data")),
            CoreError::Io(_)
        ));
        let io = std::io::Error::other("boom");
        assert!(matches!(
            CoreError::from(PlatformError::from(io)),
            CoreError::Io(_)
        ));
        assert!(matches!(
            CoreError::from(PlatformError::NonUtf8Path(PathBuf::from("x"))),
            CoreError::InvalidArgument(_)
        ));
    }

    #[test]
    fn os_error_carries_call_name() {
        let err = PlatformError::unusable("sysconf", "no pages");
        assert_eq!(err.to_string(), "sysconf failed: no pages (code 0)");
    }
}
