use camino::{Utf8Path, Utf8PathBuf};
use std::sync::Arc;

use fs::{FileSystem, FsMetadata, MmapHandle};
use root::Root;

use crate::error::VfsError;
use crate::format::{self, PackedIndex};

use super::Backend;

/// Read-only backend serving files straight out of a memory-mapped
/// `.coreproject` archive. Used in `Release` mode for every root whose
/// [`crate::root_registry::RootDescriptor::packed`] is `true`.
///
/// The actual memory-mapping is delegated to the injected [`FileSystem`]
/// (`fs.mmap_read`) - this backend doesn't know or care whether that's a
/// real `memmap2::Mmap` or an in-memory buffer used in tests.
pub struct PackedBackend {
    root: Root,
    mmap: MmapHandle,
    index: PackedIndex,
}

impl PackedBackend {
    pub fn open(
        root: Root,
        archive_path: &Utf8Path,
        fs: Arc<dyn FileSystem>,
    ) -> Result<Self, VfsError> {
        let index = format::read_index(&*fs, archive_path)?;
        let mmap = fs.mmap_read(archive_path).map_err(|e| VfsError::Io {
            root,
            path: archive_path.to_path_buf(),
            source: e,
        })?;
        Ok(Self { root, mmap, index })
    }

    pub fn root(&self) -> Root {
        self.root
    }

    fn slice(&self, rel: &Utf8Path) -> Result<&[u8], VfsError> {
        let entry = self
            .index
            .entries
            .get(&(self.root, rel.to_path_buf()))
            .ok_or_else(|| VfsError::NotFound {
                root: self.root,
                path: rel.to_path_buf(),
            })?;
        let start = (self.index.data_start + entry.offset) as usize;
        let end = start + entry.len as usize;
        self.mmap.get(start..end).ok_or_else(|| {
            VfsError::CorruptArchive(format!("entry '{rel}' points outside archive bounds"))
        })
    }
}

impl Backend for PackedBackend {
    fn read_bytes(&self, rel: &Utf8Path) -> Result<Vec<u8>, VfsError> {
        self.slice(rel).map(|s| s.to_vec())
    }

    fn write_bytes(&self, _rel: &Utf8Path, _data: &[u8]) -> Result<(), VfsError> {
        Err(VfsError::ReadOnlyBackend(self.root))
    }

    fn remove(&self, _rel: &Utf8Path) -> Result<(), VfsError> {
        Err(VfsError::ReadOnlyBackend(self.root))
    }

    fn exists(&self, rel: &Utf8Path) -> bool {
        self.index
            .entries
            .contains_key(&(self.root, rel.to_path_buf()))
    }

    fn metadata(&self, rel: &Utf8Path) -> Result<FsMetadata, VfsError> {
        let entry = self
            .index
            .entries
            .get(&(self.root, rel.to_path_buf()))
            .ok_or_else(|| VfsError::NotFound {
                root: self.root,
                path: rel.to_path_buf(),
            })?;
        Ok(FsMetadata {
            len: entry.len,
            is_dir: false,
        })
    }

    fn list_dir(&self, rel: &Utf8Path) -> Result<Vec<Utf8PathBuf>, VfsError> {
        let prefix = rel.as_str();
        let mut out: Vec<Utf8PathBuf> = self
            .index
            .entries
            .keys()
            .filter(|(r, p)| *r == self.root && p.as_str().starts_with(prefix))
            .map(|(_, p)| p.clone())
            .collect();
        out.sort();
        Ok(out)
    }

    fn is_read_only(&self) -> bool {
        true
    }
}
