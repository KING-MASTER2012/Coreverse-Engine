use camino::{Utf8Path, Utf8PathBuf};
use memmap2::Mmap;
use std::fs::File;

use root::Root;

use crate::error::VfsError;
use crate::format::{self, PackedIndex};

use super::{Backend, VfsMetadata};

/// Read-only backend serving files straight out of a memory-mapped
/// `.coreproject` archive. Used in `Release` mode for every root whose
/// [`crate::root_registry::RootDescriptor::packed`] is `true`.
pub struct PackedBackend {
    root: Root,
    mmap: Mmap,
    index: PackedIndex,
}

impl PackedBackend {
    pub fn open(root: Root, archive_path: &Utf8Path) -> Result<Self, VfsError> {
        let index = format::read_index(archive_path)?;
        let file = File::open(archive_path).map_err(|e| VfsError::Io {
            root,
            path: archive_path.to_path_buf(),
            source: e,
        })?;
        // Safety: the archive is treated as immutable engine data for the
        // whole process lifetime; nothing else in this process writes to
        // it while it stays mapped.
        let mmap = unsafe { Mmap::map(&file) }.map_err(|e| VfsError::Io {
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

    fn metadata(&self, rel: &Utf8Path) -> Result<VfsMetadata, VfsError> {
        let entry = self
            .index
            .entries
            .get(&(self.root, rel.to_path_buf()))
            .ok_or_else(|| VfsError::NotFound {
                root: self.root,
                path: rel.to_path_buf(),
            })?;
        Ok(VfsMetadata {
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
