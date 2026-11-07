use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use std::fmt;

use root::Root;

use crate::error::VfsError;

/// A path inside the VFS: a [`Root`] plus a path relative to it.
///
/// This is the only public constructor for VFS paths, and it enforces these
/// invariants on every platform: the relative part is never absolute or
/// rooted (`/x`), never carries a Windows drive/UNC prefix (`C:foo`), never
/// contains `:` (drive letters, alternate data streams) or NUL, and never
/// contains `..` (so a path can never escape its root, even accidentally).
///
/// An *empty* relative part (or one made only of `.`) is allowed and names
/// the root itself - needed to list a whole root - but see
/// [`VfsPath::is_root`]: destructive operations refuse it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VfsPath {
    root: Root,
    rel: Utf8PathBuf,
}

impl VfsPath {
    /// Builds a `VfsPath` from `root` and a relative path, normalizing `\`
    /// to `/` and rejecting absolute paths or `..` components.
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

        validate_relative(rel)?;

        Ok(Self {
            root,
            rel: normalized,
        })
    }

    /// The [`Root`] this path is relative to.
    pub fn root(&self) -> Root {
        self.root
    }

    /// The path's root-relative, `/`-separated portion.
    pub fn rel(&self) -> &Utf8Path {
        &self.rel
    }

    /// Whether this path names the root directory itself (an empty
    /// relative part, or one made only of `.`).
    ///
    /// Reading or listing the root is fine; writing to it or removing it is
    /// not, which is why `FileManager::write_bytes` / `remove` reject it.
    pub fn is_root(&self) -> bool {
        is_root_rel(&self.rel)
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

    /// The file extension of [`Self::rel`], if any (no leading dot).
    pub fn extension(&self) -> Option<&str> {
        self.rel.extension()
    }

    /// The final path segment of [`Self::rel`], if any.
    pub fn file_name(&self) -> Option<&str> {
        self.rel.file_name()
    }
}

impl fmt::Display for VfsPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}:/{}", self.root, self.rel)
    }
}

/// Whether `rel` has no real segments, i.e. it addresses the root itself
/// (`""`, `"."`, `"./."`).
pub(crate) fn is_root_rel(rel: &Utf8Path) -> bool {
    rel.components()
        .all(|component| matches!(component, Utf8Component::CurDir))
}

/// Validates the root-relative part of a VFS path.
///
/// Shared by [`VfsPath::new`] and the loose backend (whose methods take raw
/// paths and are public API), so a path that bypassed `VfsPath` still cannot
/// leave its root. The rules are deliberately the same on every platform:
/// a path that is fine on Linux but escapes on Windows (`/x` is not
/// "absolute" there, `C:foo` is drive-relative) would make assets behave
/// differently - and unsafely - per OS.
pub(crate) fn validate_relative(rel: &Utf8Path) -> Result<(), VfsError> {
    let text = rel.as_str();

    if text.contains('\0') {
        return Err(VfsError::InvalidPath(
            "path may not contain a NUL byte".to_string(),
        ));
    }
    if rel.is_absolute() || text.starts_with('/') {
        return Err(VfsError::InvalidPath(format!(
            "'{rel}' must be relative to its root, not absolute"
        )));
    }

    for component in rel.components() {
        match component {
            Utf8Component::Prefix(_) | Utf8Component::RootDir => {
                return Err(VfsError::InvalidPath(format!(
                    "'{rel}' must be relative to its root, not absolute or drive-qualified"
                )));
            }
            Utf8Component::ParentDir => {
                return Err(VfsError::InvalidPath(format!(
                    "'{rel}' may not contain '..' (root escape is not allowed)"
                )));
            }
            Utf8Component::Normal(name) if name.contains(':') => {
                return Err(VfsError::InvalidPath(format!(
                    "'{rel}' may not contain ':' (drive letters and alternate data streams are not allowed)"
                )));
            }
            Utf8Component::CurDir | Utf8Component::Normal(_) => {}
        }
    }
    Ok(())
}
