pub mod backend;
pub mod context;
pub mod error;
pub mod file_manager;
pub mod format;
pub mod path;
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
