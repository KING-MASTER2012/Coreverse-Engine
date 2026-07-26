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
