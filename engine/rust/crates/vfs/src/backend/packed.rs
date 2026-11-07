use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use std::collections::BTreeSet;
use std::sync::Arc;

use fs::{FileSystem, FsMetadata, MmapHandle};
use root::Root;

use crate::error::VfsError;
use crate::format::{self, PackedIndex, PackedIndexEntry};
use crate::path::is_root_rel;

use super::Backend;

/// An opened `.coreproject` archive: its parsed index plus the memory map
/// of its data.
///
/// One archive serves several roots (every `packed` root lives in the same
/// file), so it is opened - index read, file mapped - **once** and shared by
/// every [`PackedBackend`] through an [`Arc`].
pub struct PackedArchive {
    mmap: MmapHandle,
    index: PackedIndex,
}

impl PackedArchive {
    /// Opens the archive at `archive_path`, reads its index (validated, see
    /// [`format::read_index`]) and memory-maps its data through `fs`. Fails
    /// if the file is missing, unreadable, or its index is corrupt.
    ///
    /// `root` is only used to label I/O errors: the root on whose behalf the
    /// archive is being opened (the archive itself spans every packed root).
    pub fn open(
        root: Root,
        archive_path: &Utf8Path,
        fs: &dyn FileSystem,
    ) -> Result<Self, VfsError> {
        let index = format::read_index(fs, archive_path)?;
        let mmap = fs.mmap_read(archive_path).map_err(|e| VfsError::Io {
            root,
            path: archive_path.to_path_buf(),
            source: e,
        })?;
        Ok(Self { mmap, index })
    }
}

/// Read-only backend serving files straight out of a memory-mapped
/// `.coreproject` archive. Used in `Release` mode for every root whose
/// [`crate::root_registry::RootDescriptor::packed`] is `true`.
///
/// The actual memory-mapping is delegated to the injected [`FileSystem`]
/// (`fs.mmap_read`) - this backend doesn't know or care whether that's a
/// real `memmap2::Mmap` or an in-memory buffer used in tests.
///
/// Archives store only files; directories are *virtual*: a directory
/// exists if at least one entry lives under it, and listing it returns its
/// direct children (files and sub-directories), exactly like
/// [`super::loose::LooseBackend`] does, so the two backends can be merged
/// by [`super::hybrid::HybridBackend`].
pub struct PackedBackend {
    root: Root,
    archive: Arc<PackedArchive>,
}

impl PackedBackend {
    /// Opens the `.coreproject` archive at `archive_path` for `root`.
    ///
    /// Convenience for a single root; when several roots share the archive
    /// open it once with [`PackedArchive::open`] and use
    /// [`Self::from_archive`] instead.
    pub fn open(
        root: Root,
        archive_path: &Utf8Path,
        fs: Arc<dyn FileSystem>,
    ) -> Result<Self, VfsError> {
        let archive = PackedArchive::open(root, archive_path, &*fs)?;
        Ok(Self::from_archive(root, Arc::new(archive)))
    }

    /// Serves `root` out of an already opened, shared archive.
    pub fn from_archive(root: Root, archive: Arc<PackedArchive>) -> Self {
        Self { root, archive }
    }

    /// The [`Root`] this backend is mounted on.
    pub fn root(&self) -> Root {
        self.root
    }

    /// `rel` without `.` segments, so `a/./b` and `a/b` address the same
    /// entry. (Entries are stored as written; `..` and absolute paths are
    /// never valid lookups and simply match nothing.)
    fn normalize(rel: &Utf8Path) -> Utf8PathBuf {
        rel.components()
            .filter(|component| !matches!(component, Utf8Component::CurDir))
            .collect()
    }

    fn file_entry(&self, rel: &Utf8Path) -> Option<&PackedIndexEntry> {
        self.archive
            .index
            .entries
            .get(&(self.root, Self::normalize(rel)))
    }

    /// The direct children of the virtual directory `dir`, sorted; empty if
    /// nothing lives under it.
    fn children(&self, dir: &Utf8Path) -> BTreeSet<Utf8PathBuf> {
        let mut out = BTreeSet::new();
        for (root, path) in self.archive.index.entries.keys() {
            if *root != self.root {
                continue;
            }
            let Ok(rest) = path.strip_prefix(dir) else {
                continue;
            };
            if let Some(Utf8Component::Normal(name)) = rest.components().next() {
                out.insert(dir.join(name));
            }
        }
        out
    }

    fn slice(&self, rel: &Utf8Path) -> Result<&[u8], VfsError> {
        let entry = self.file_entry(rel).ok_or_else(|| VfsError::NotFound {
            root: self.root,
            path: rel.to_path_buf(),
        })?;
        // Offsets come from the archive (untrusted), so every step is checked:
        // plain `+`/`as` would wrap in release builds and could point an
        // entry at the header or at another entry's data.
        let out_of_bounds =
            || VfsError::CorruptArchive(format!("entry '{rel}' points outside archive bounds"));
        let start = self
            .archive
            .index
            .data_start
            .checked_add(entry.offset)
            .and_then(|start| usize::try_from(start).ok())
            .ok_or_else(out_of_bounds)?;
        let end = usize::try_from(entry.len)
            .ok()
            .and_then(|len| start.checked_add(len))
            .ok_or_else(out_of_bounds)?;
        self.archive.mmap.get(start..end).ok_or_else(out_of_bounds)
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
        self.file_entry(rel).is_some()
            || is_root_rel(rel)
            || !self.children(&Self::normalize(rel)).is_empty()
    }

    fn metadata(&self, rel: &Utf8Path) -> Result<FsMetadata, VfsError> {
        if let Some(entry) = self.file_entry(rel) {
            return Ok(FsMetadata {
                len: entry.len,
                is_dir: false,
            });
        }
        if is_root_rel(rel) || !self.children(&Self::normalize(rel)).is_empty() {
            return Ok(FsMetadata {
                len: 0,
                is_dir: true,
            });
        }
        Err(VfsError::NotFound {
            root: self.root,
            path: rel.to_path_buf(),
        })
    }

    fn list_dir(&self, rel: &Utf8Path) -> Result<Vec<Utf8PathBuf>, VfsError> {
        let dir = Self::normalize(rel);
        let children = self.children(&dir);

        if children.is_empty() && !is_root_rel(rel) {
            return Err(if self.file_entry(rel).is_some() {
                VfsError::Io {
                    root: self.root,
                    path: rel.to_path_buf(),
                    source: std::io::Error::other("not a directory"),
                }
            } else {
                VfsError::NotFound {
                    root: self.root,
                    path: rel.to_path_buf(),
                }
            });
        }
        Ok(children.into_iter().collect())
    }

    fn is_read_only(&self) -> bool {
        true
    }
}
