use camino::{Utf8Path, Utf8PathBuf};
use std::fs;

use root::Root;

use crate::error::VfsError;

use super::{Backend, VfsMetadata};

/// Real files on disk, rooted at `base`. Used directly in `Development`
/// mode for every root, and as the writable overlay inside
/// [`super::hybrid::HybridBackend`] for release builds.
pub struct LooseBackend {
    root: Root,
    base: Utf8PathBuf,
}

impl LooseBackend {
    pub fn new(root: Root, base: impl Into<Utf8PathBuf>) -> Self {
        Self {
            root,
            base: base.into(),
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
        fs::read(self.full_path(rel)).map_err(|e| self.io_err(rel, e))
    }

    fn write_bytes(&self, rel: &Utf8Path, data: &[u8]) -> Result<(), VfsError> {
        let full = self.full_path(rel);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).map_err(|e| self.io_err(rel, e))?;
        }
        fs::write(&full, data).map_err(|e| self.io_err(rel, e))
    }

    fn remove(&self, rel: &Utf8Path) -> Result<(), VfsError> {
        let full = self.full_path(rel);
        if full.is_dir() {
            fs::remove_dir_all(&full).map_err(|e| self.io_err(rel, e))
        } else {
            fs::remove_file(&full).map_err(|e| self.io_err(rel, e))
        }
    }

    fn exists(&self, rel: &Utf8Path) -> bool {
        self.full_path(rel).exists()
    }

    fn metadata(&self, rel: &Utf8Path) -> Result<VfsMetadata, VfsError> {
        let meta = fs::metadata(self.full_path(rel)).map_err(|e| self.io_err(rel, e))?;
        Ok(VfsMetadata {
            len: meta.len(),
            is_dir: meta.is_dir(),
        })
    }

    fn list_dir(&self, rel: &Utf8Path) -> Result<Vec<Utf8PathBuf>, VfsError> {
        let full = self.full_path(rel);
        let mut out = Vec::new();
        for entry in fs::read_dir(&full).map_err(|e| self.io_err(rel, e))? {
            let entry = entry.map_err(|e| self.io_err(rel, e))?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            out.push(rel.join(name.as_ref()));
        }
        Ok(out)
    }
}
