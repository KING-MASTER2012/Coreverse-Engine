use camino::Utf8PathBuf;
use thiserror::Error;

use root::Root;

#[derive(Debug, Error)]
pub enum VfsError {
    #[error("root '{0:?}' is not registered in the VFS context")]
    RootNotRegistered(Root),

    #[error("path not found: {root:?}:/{path}")]
    NotFound { root: Root, path: Utf8PathBuf },

    #[error("backend for root '{0:?}' is read-only (packed), write rejected")]
    ReadOnlyBackend(Root),

    #[error("io error at {root:?}:/{path}: {source}")]
    Io {
        root: Root,
        path: Utf8PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("invalid vfs path: {0}")]
    InvalidPath(String),

    #[error("packed archive is corrupt: {0}")]
    CorruptArchive(String),

    #[error("vfs context is already initialized")]
    AlreadyInitialized,

    #[error("vfs context not initialized - call VfsContext::init() first")]
    NotInitialized,
}

impl VfsError {
    /// Stable numeric code for this error variant, for callers (like the
    /// `ffi` crate) that need to hand the failure reason across an ABI
    /// boundary without cloning/matching the full error. Payload fields
    /// (which `Root`, which path, the underlying `io::Error`) are not
    /// encoded here — use `Display`/`to_string()` (already provided via
    /// `thiserror`) for a full human-readable message instead. One-way
    /// only (no `from_u8`): callers never construct a `VfsError`
    /// themselves, only read a code back after a call failed.
    pub fn code(&self) -> u8 {
        match self {
            VfsError::RootNotRegistered(_) => 0,
            VfsError::NotFound { .. } => 1,
            VfsError::ReadOnlyBackend(_) => 2,
            VfsError::Io { .. } => 3,
            VfsError::InvalidPath(_) => 4,
            VfsError::CorruptArchive(_) => 5,
            VfsError::AlreadyInitialized => 6,
            VfsError::NotInitialized => 7,
        }
    }
}
