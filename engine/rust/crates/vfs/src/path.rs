use camino::{Utf8Path, Utf8PathBuf};
use std::fmt;

use root::Root;

use crate::error::VfsError;

/// A path inside the VFS: a [`Root`] plus a path relative to it.
///
/// This is the only public constructor for VFS paths, and it enforces two
/// invariants: the relative part is never absolute, and it never contains
/// `..` (so a path can never escape its root, even accidentally).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VfsPath {
    root: Root,
    rel: Utf8PathBuf,
}

impl VfsPath {
    pub fn new(root: Root, rel: impl AsRef<Utf8Path>) -> Result<Self, VfsError> {
        let rel = rel.as_ref();

        if rel.is_absolute() {
            return Err(VfsError::InvalidPath(format!(
                "'{rel}' must be relative to its root, not absolute"
            )));
        }
        if rel.components().any(|c| c.as_str() == "..") {
            return Err(VfsError::InvalidPath(format!(
                "'{rel}' may not contain '..' (root escape is not allowed)"
            )));
        }

        Ok(Self {
            root,
            rel: rel.to_path_buf(),
        })
    }

    pub fn root(&self) -> Root {
        self.root
    }

    pub fn rel(&self) -> &Utf8Path {
        &self.rel
    }

    /// Builds a child path, e.g. `assets_dir.join("textures/hero.png")?`.
    pub fn join(&self, segment: impl AsRef<Utf8Path>) -> Result<Self, VfsError> {
        Self::new(self.root, self.rel.join(segment.as_ref()))
    }

    pub fn extension(&self) -> Option<&str> {
        self.rel.extension()
    }

    pub fn file_name(&self) -> Option<&str> {
        self.rel.file_name()
    }
}

impl fmt::Display for VfsPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}:/{}", self.root, self.rel)
    }
}
