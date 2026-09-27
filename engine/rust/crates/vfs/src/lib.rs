//! Coreverse's virtual filesystem: root-relative paths (`VfsPath`) resolved,
//! through a registry of mounted [`Root`]s, to one of several storage
//! [`backend`]s (loose files on disk, a packed archive, or a hybrid of the
//! two), all driven through the process-global [`VfsContext`].

/// Storage backends a [`Root`] can be mounted on (loose, packed, hybrid).
pub mod backend;
/// Process-global VFS state: root registration, mode, and lookup.
pub mod context;
/// Error type shared by every fallible operation in this crate.
pub mod error;
/// Free functions for reading/writing through the global [`VfsContext`].
pub mod file_manager;
/// On-disk layout of the packed archive format (`.coreproject`-adjacent).
pub mod format;
/// Root-relative path type ([`VfsPath`]) and its parsing/validation.
pub mod path;
/// Compile-time registry of roots declared across the workspace.
pub mod root_registry;

pub use context::{VfsContext, VfsMode};
pub use error::VfsError;
pub use path::VfsPath;
pub use root_registry::RootDescriptor;

// So callers only need to depend on `vfs`, not `root`/`fs` directly.
pub use root::Root;

// Re-exported so tests can do `vfs::MemoryFileSystem` and pass it to
// `VfsContext::init_with_fs` without adding `fs` as a separate dependency.
pub use fs::{FileSystem, FsMetadata, MemoryFileSystem, OsFileSystem};
