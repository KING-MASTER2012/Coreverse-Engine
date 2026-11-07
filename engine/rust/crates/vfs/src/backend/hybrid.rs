use camino::{Utf8Path, Utf8PathBuf};
use std::collections::BTreeSet;

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
///
/// There are no "whiteouts": removing a file only removes the overlay copy,
/// so if the archive holds a file at the same path, that original becomes
/// visible again ("reset to default"). A file that exists *only* in the
/// archive cannot be removed ([`VfsError::ReadOnlyBackend`]).
pub struct HybridBackend {
    packed: PackedBackend,
    overlay: LooseBackend,
}

impl HybridBackend {
    /// Builds a hybrid backend over an already-open `packed` archive and an
    /// already-open `overlay` directory. Both must be mounted on the same
    /// [`Root`].
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
        // "Not there" in one layer is normal (a directory may exist only in
        // the archive or only in the overlay); any other error is real and
        // is reported instead of silently yielding a partial listing.
        let mut layers = Vec::new();
        let mut first_not_found = None;
        for result in [self.packed.list_dir(rel), self.overlay.list_dir(rel)] {
            match result {
                Ok(entries) => layers.push(entries),
                Err(e) if e.is_not_found() => {
                    first_not_found.get_or_insert(e);
                }
                Err(e) => return Err(e),
            }
        }
        if layers.is_empty() {
            return Err(first_not_found.unwrap_or_else(|| VfsError::NotFound {
                root: self.root(),
                path: rel.to_path_buf(),
            }));
        }

        let merged: BTreeSet<Utf8PathBuf> = layers.into_iter().flatten().collect();
        Ok(merged.into_iter().collect())
    }
}
