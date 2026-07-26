pub mod hybrid;
pub mod loose;
pub mod packed;

use camino::{Utf8Path, Utf8PathBuf};

use crate::error::VfsError;

/// Metadata about a single VFS entry. Kept intentionally small - the
/// `metadata!` macro in `vfs-macros` produces a compile-time constant
/// version of this for assets baked at build time; this is the runtime
/// equivalent, used for anything resolved at runtime (mods, cache, saves).
#[derive(Debug, Clone, Copy)]
pub struct VfsMetadata {
    pub len: u64,
    pub is_dir: bool,
}

/// A storage backend for a single root.
///
/// Implementations: [`loose::LooseBackend`] (real files on disk,
/// read/write), [`packed::PackedBackend`] (memory-mapped `.coreproject`
/// slice, read-only), and [`hybrid::HybridBackend`] (packed base + loose
/// writable overlay, used in `Release` mode for packed roots).
pub trait Backend: Send + Sync {
    fn read_bytes(&self, rel: &Utf8Path) -> Result<Vec<u8>, VfsError>;

    fn read_to_string(&self, rel: &Utf8Path) -> Result<String, VfsError> {
        let bytes = self.read_bytes(rel)?;
        String::from_utf8(bytes)
            .map_err(|e| VfsError::CorruptArchive(format!("'{rel}' is not valid utf-8: {e}")))
    }

    fn write_bytes(&self, rel: &Utf8Path, data: &[u8]) -> Result<(), VfsError>;

    fn remove(&self, rel: &Utf8Path) -> Result<(), VfsError>;

    fn exists(&self, rel: &Utf8Path) -> bool;

    fn metadata(&self, rel: &Utf8Path) -> Result<VfsMetadata, VfsError>;

    fn list_dir(&self, rel: &Utf8Path) -> Result<Vec<Utf8PathBuf>, VfsError>;

    fn is_read_only(&self) -> bool {
        false
    }
}
