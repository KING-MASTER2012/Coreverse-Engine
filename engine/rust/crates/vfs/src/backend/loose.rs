use camino::{Utf8Path, Utf8PathBuf};
use std::sync::Arc;

use fs::{FileSystem, FsMetadata};
use root::Root;

use crate::error::VfsError;

use super::Backend;

/// Files rooted at `base`, backed by whatever [`FileSystem`] implementation
/// is injected - real disk ([`fs::OsFileSystem`]) in normal use, or
/// [`fs::MemoryFileSystem`] in tests. Used directly in `Development` mode
/// for every root, and as the writable overlay inside
/// [`super::hybrid::HybridBackend`] for release builds.
pub struct LooseBackend {
    root: Root,
    base: Utf8PathBuf,
    fs: Arc<dyn FileSystem>,
}

impl LooseBackend {
    pub fn new(root: Root, base: impl Into<Utf8PathBuf>, fs: Arc<dyn FileSystem>) -> Self {
        Self {
            root,
            base: base.into(),
            fs,
        }
    }

    pub fn root(&self) -> Root {
        self.root
    }

    fn full_path(&self, rel: &Utf8Path) -> Utf8PathBuf {
        self.base.join(rel)
    }

    fn io_err(&self, rel: &Utf8Path, source: std::io::Error) -> VfsError {
        if source.kind() == std::io::ErrorKind::NotFound {
            VfsError::NotFound {
                root: self.root,
                path: rel.to_path_buf(),
            }
        } else {
            VfsError::Io {
                root: self.root,
                path: rel.to_path_buf(),
                source,
            }
        }
    }
}

impl Backend for LooseBackend {
    fn read_bytes(&self, rel: &Utf8Path) -> Result<Vec<u8>, VfsError> {
        self.fs
            .read_bytes(&self.full_path(rel))
            .map_err(|e| self.io_err(rel, e))
    }

    fn write_bytes(&self, rel: &Utf8Path, data: &[u8]) -> Result<(), VfsError> {
        self.fs
            .write_bytes(&self.full_path(rel), data)
            .map_err(|e| self.io_err(rel, e))
    }

    fn remove(&self, rel: &Utf8Path) -> Result<(), VfsError> {
        let full = self.full_path(rel);
        let meta = self.fs.metadata(&full).map_err(|e| self.io_err(rel, e))?;
        if meta.is_dir {
            self.fs
                .remove_dir_all(&full)
                .map_err(|e| self.io_err(rel, e))
        } else {
            self.fs.remove_file(&full).map_err(|e| self.io_err(rel, e))
        }
    }

    fn exists(&self, rel: &Utf8Path) -> bool {
        self.fs.exists(&self.full_path(rel))
    }

    fn metadata(&self, rel: &Utf8Path) -> Result<FsMetadata, VfsError> {
        self.fs
            .metadata(&self.full_path(rel))
            .map_err(|e| self.io_err(rel, e))
    }

    fn list_dir(&self, rel: &Utf8Path) -> Result<Vec<Utf8PathBuf>, VfsError> {
        let full = self.full_path(rel);
        let entries = self.fs.read_dir(&full).map_err(|e| self.io_err(rel, e))?;
        // `FileSystem::read_dir` returns children in the same "namespace" as
        // the path passed in (i.e. joined onto `self.base`) - translate them
        // back to paths relative to this backend's root, like the rest of
        // the VFS API expects.
        Ok(entries
            .into_iter()
            .filter_map(|full_child| {
                full_child
                    .strip_prefix(&self.base)
                    .ok()
                    .map(|p| p.to_path_buf())
            })
            .collect())
    }
}
