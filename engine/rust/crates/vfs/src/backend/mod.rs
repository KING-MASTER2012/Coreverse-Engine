pub mod hybrid;
pub mod loose;
pub mod packed;

use camino::{Utf8Path, Utf8PathBuf};

use fs::FsMetadata;

use crate::error::VfsError;

/// A storage backend for a single root.
///
/// Implementations: [`loose::LooseBackend`] (files via an injected
/// [`fs::FileSystem`], read/write), [`packed::PackedBackend`] (memory-mapped
/// `.coreproject` slice, read-only), and [`hybrid::HybridBackend`] (packed
/// base + loose writable overlay, used in `Release` mode for packed roots).
///
/// Metadata uses [`fs::FsMetadata`] directly rather than a duplicate VFS-level
/// type - "how big is this / is it a directory" is already an FS-layer
/// concept, no need to wrap it again here.
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

    fn metadata(&self, rel: &Utf8Path) -> Result<FsMetadata, VfsError>;

    fn list_dir(&self, rel: &Utf8Path) -> Result<Vec<Utf8PathBuf>, VfsError>;

    fn is_read_only(&self) -> bool {
        false
    }
}
