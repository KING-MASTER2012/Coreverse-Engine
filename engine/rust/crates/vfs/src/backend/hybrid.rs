use camino::{Utf8Path, Utf8PathBuf};

use fs::FsMetadata;
use root::Root;

use crate::error::VfsError;

use super::{Backend, loose::LooseBackend, packed::PackedBackend};

/// Packed archive as the read-only base, with a loose directory as a
/// writable overlay on top.
///
/// Reads check the overlay first (so dev overrides / downloaded patches /
/// mod replacements win), then fall back to the packed archive. Writes
/// always go to the overlay - the packed half is never mutated at runtime.
pub struct HybridBackend {
    packed: PackedBackend,
    overlay: LooseBackend,
}

impl HybridBackend {
    pub fn new(packed: PackedBackend, overlay: LooseBackend) -> Self {
        Self { packed, overlay }
    }

    fn root(&self) -> Root {
        self.packed.root()
    }
}

impl Backend for HybridBackend {
    fn read_bytes(&self, rel: &Utf8Path) -> Result<Vec<u8>, VfsError> {
        if self.overlay.exists(rel) {
            self.overlay.read_bytes(rel)
        } else {
            self.packed.read_bytes(rel)
        }
    }

    fn write_bytes(&self, rel: &Utf8Path, data: &[u8]) -> Result<(), VfsError> {
        self.overlay.write_bytes(rel, data)
    }

    fn remove(&self, rel: &Utf8Path) -> Result<(), VfsError> {
        if self.overlay.exists(rel) {
            self.overlay.remove(rel)
        } else {
            Err(VfsError::ReadOnlyBackend(self.root()))
        }
    }

    fn exists(&self, rel: &Utf8Path) -> bool {
        self.overlay.exists(rel) || self.packed.exists(rel)
    }

    fn metadata(&self, rel: &Utf8Path) -> Result<FsMetadata, VfsError> {
        if self.overlay.exists(rel) {
            self.overlay.metadata(rel)
        } else {
            self.packed.metadata(rel)
        }
    }

    fn list_dir(&self, rel: &Utf8Path) -> Result<Vec<Utf8PathBuf>, VfsError> {
        let mut out = self.packed.list_dir(rel).unwrap_or_default();
        if let Ok(loose_entries) = self.overlay.list_dir(rel) {
            for entry in loose_entries {
                if !out.contains(&entry) {
                    out.push(entry);
                }
            }
        }
        out.sort();
        out.dedup();
        Ok(out)
    }
}
