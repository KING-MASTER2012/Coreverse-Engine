use camino::Utf8PathBuf;
use thiserror::Error;

use root::Root;

/// Every fallible outcome the VFS's public API can return.
#[derive(Debug, Error)]
pub enum VfsError {
    /// A [`Root`] was used that no `RootDescriptor` registered.
    #[error("root '{0:?}' is not registered in the VFS context")]
    RootNotRegistered(Root),

    /// No file exists at `path` under `root`.
    #[error("path not found: {root:?}:/{path}")]
    NotFound {
        /// The root the lookup was scoped to.
        root: Root,
        /// The root-relative path that was not found.
        path: Utf8PathBuf,
    },

    /// A write/remove was attempted on a root whose backend is read-only
    /// (a packed archive with no writable overlay).
    #[error("backend for root '{0:?}' is read-only (packed), write rejected")]
    ReadOnlyBackend(Root),

    /// The underlying OS filesystem or archive read/write failed.
    #[error("io error at {root:?}:/{path}: {source}")]
    Io {
        /// The root the operation was scoped to.
        root: Root,
        /// The root-relative path being accessed.
        path: Utf8PathBuf,
        /// The underlying OS error.
        #[source]
        source: std::io::Error,
    },

    /// A `VfsPath` failed to parse or validate.
    #[error("invalid vfs path: {0}")]
    InvalidPath(String),

    /// A `.coreproject` archive's index or layout could not be read.
    #[error("packed archive is corrupt: {0}")]
    CorruptArchive(String),

    /// [`crate::context::VfsContext::init`]/`init_with_fs` was called more
    /// than once in this process.
    #[error("vfs context is already initialized")]
    AlreadyInitialized,

    /// A VFS call was made before [`crate::context::VfsContext::init`]/
    /// `init_with_fs` ran.
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
