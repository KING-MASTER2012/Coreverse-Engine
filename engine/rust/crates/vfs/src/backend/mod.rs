/// Packed base + loose writable overlay ([`hybrid::HybridBackend`]).
pub mod hybrid;
/// Loose files on disk via an injected [`fs::FileSystem`] ([`loose::LooseBackend`]).
pub mod loose;
/// Memory-mapped, read-only `.coreproject` archive backend ([`packed::PackedBackend`]).
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
    /// Reads the whole file at `rel` (root-relative) into memory.
    fn read_bytes(&self, rel: &Utf8Path) -> Result<Vec<u8>, VfsError>;

    /// Reads `rel` and validates it as UTF-8. Default impl built on
    /// [`Backend::read_bytes`]; override only if a backend can avoid the
    /// extra UTF-8 validation pass.
    fn read_to_string(&self, rel: &Utf8Path) -> Result<String, VfsError> {
        let bytes = self.read_bytes(rel)?;
        String::from_utf8(bytes)
            .map_err(|e| VfsError::CorruptArchive(format!("'{rel}' is not valid utf-8: {e}")))
    }

    /// Writes `data` to `rel`, creating or truncating it. Fails on a
    /// read-only backend (see [`Backend::is_read_only`]).
    fn write_bytes(&self, rel: &Utf8Path, data: &[u8]) -> Result<(), VfsError>;

    /// Deletes the file at `rel`. Fails on a read-only backend.
    fn remove(&self, rel: &Utf8Path) -> Result<(), VfsError>;

    /// Whether a file exists at `rel`.
    fn exists(&self, rel: &Utf8Path) -> bool;

    /// Size and file/directory status for `rel`.
    fn metadata(&self, rel: &Utf8Path) -> Result<FsMetadata, VfsError>;

    /// Lists the direct children of the directory at `rel`.
    fn list_dir(&self, rel: &Utf8Path) -> Result<Vec<Utf8PathBuf>, VfsError>;

    /// Whether [`Backend::write_bytes`]/[`Backend::remove`] always fail on
    /// this backend. `false` unless overridden (e.g. [`packed::PackedBackend`]).
    fn is_read_only(&self) -> bool {
        false
    }
}
