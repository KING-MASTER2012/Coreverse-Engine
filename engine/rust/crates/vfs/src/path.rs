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

        // `VfsPath`'s relative part is a portable identifier - it's used as
        // a stable string/key across backends (e.g. `MemoryFileSystem`'s
        // map, `rel()` string comparisons) and must not depend on the
        // host's path-separator convention. Normalize any `\` to `/` up
        // front so a Windows-style input (or one built via `join`, see
        // below) can never smuggle a backslash into that identifier, and
        // so the `..`-escape check below can't be bypassed by a
        // platform-specific separator it doesn't recognize.
        let normalized = if rel.as_str().contains('\\') {
            Utf8PathBuf::from(rel.as_str().replace('\\', "/"))
        } else {
            rel.to_path_buf()
        };
        let rel = normalized.as_path();

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
            rel: normalized,
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
        let segment = segment.as_ref();
        // Built with an explicit `/` rather than `Utf8PathBuf::join` (which
        // inserts the *platform* separator - `\` on Windows). See the note
        // in `new()`: this relative part must stay `/`-separated on every
        // platform.
        let joined = if self.rel.as_str().is_empty() {
            segment.to_path_buf()
        } else {
            Utf8PathBuf::from(format!("{}/{}", self.rel, segment))
        };
        Self::new(self.root, joined)
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
