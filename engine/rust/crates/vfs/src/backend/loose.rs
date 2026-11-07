use camino::{Utf8Path, Utf8PathBuf};
use std::sync::{Arc, OnceLock};

use fs::{FileSystem, FsMetadata};
use root::Root;

use crate::error::VfsError;
use crate::path::{is_root_rel, validate_relative};

use super::Backend;

/// Files rooted at `base`, backed by whatever [`FileSystem`] implementation
/// is injected - real disk ([`fs::OsFileSystem`]) in normal use, or
/// [`fs::MemoryFileSystem`] in tests. Used directly in `Development` mode
/// for every root, and as the writable overlay inside
/// [`super::hybrid::HybridBackend`] for release builds.
///
/// Its methods are public API and take raw paths, so the backend defends
/// itself: every path is validated like a `VfsPath` (no `..`, no absolute or
/// drive-qualified paths) and checked to stay inside `base` after symlinks
/// are resolved. Writing to or removing the root itself is refused.
///
/// The symlink check happens right before each operation, so it cannot stop
/// an attacker who swaps a path component for a symlink *between* the check
/// and the operation; keep roots out of directories untrusted users can write.
pub struct LooseBackend {
    root: Root,
    base: Utf8PathBuf,
    fs: Arc<dyn FileSystem>,
    /// `base` with symlinks resolved, cached once `base` exists.
    base_canonical: OnceLock<Utf8PathBuf>,
}

impl LooseBackend {
    /// Mounts `root` at `base` (an absolute or process-relative directory),
    /// resolving every relative path through `fs`.
    pub fn new(root: Root, base: impl Into<Utf8PathBuf>, fs: Arc<dyn FileSystem>) -> Self {
        Self {
            root,
            base: base.into(),
            fs,
            base_canonical: OnceLock::new(),
        }
    }

    /// The [`Root`] this backend is mounted on.
    pub fn root(&self) -> Root {
        self.root
    }

    /// Validates `rel` and returns its location on the filesystem, failing if
    /// it would leave `base` (directly or through a symlink).
    fn resolve(&self, rel: &Utf8Path) -> Result<Utf8PathBuf, VfsError> {
        validate_relative(rel)?;
        let full = self.base.join(rel);
        self.ensure_contained(&full, rel)?;
        Ok(full)
    }

    /// Like [`Self::resolve`], but for operations that modify or delete
    /// `rel`: the root directory itself is refused.
    fn resolve_mutable(&self, rel: &Utf8Path) -> Result<Utf8PathBuf, VfsError> {
        if is_root_rel(rel) {
            return Err(VfsError::InvalidPath(format!(
                "'{rel}' refers to the root of {:?} itself; it cannot be written or removed",
                self.root
            )));
        }
        self.resolve(rel)
    }

    /// `base` with symlinks resolved, or `None` while `base` does not exist
    /// yet (then nothing can be inside it, so there is nothing to escape).
    fn canonical_base(&self) -> std::io::Result<Option<&Utf8PathBuf>> {
        if let Some(cached) = self.base_canonical.get() {
            return Ok(Some(cached));
        }
        match self.fs.canonicalize(&self.base) {
            Ok(resolved) => Ok(Some(self.base_canonical.get_or_init(|| resolved))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Errors unless `full` - after following symlinks - is inside `base`.
    ///
    /// A path that does not exist yet (a file about to be created) is judged
    /// by its nearest existing ancestor; a dangling symlink is refused
    /// outright because creating through it would write to its target.
    fn ensure_contained(&self, full: &Utf8Path, rel: &Utf8Path) -> Result<(), VfsError> {
        let escape = || {
            VfsError::InvalidPath(format!(
                "'{rel}' resolves outside the root of {:?} (symlink escape)",
                self.root
            ))
        };
        let Some(base) = self.canonical_base().map_err(|e| self.io_err(rel, e))? else {
            return Ok(());
        };

        let mut probe = full;
        loop {
            match self.fs.canonicalize(probe) {
                Ok(resolved) => {
                    return if resolved.starts_with(base) {
                        Ok(())
                    } else {
                        Err(escape())
                    };
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    if self.fs.is_symlink(probe) {
                        return Err(escape());
                    }
                    match probe.parent() {
                        Some(parent) => probe = parent,
                        None => return Ok(()),
                    }
                }
                Err(e) => return Err(self.io_err(rel, e)),
            }
        }
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
        let full = self.resolve(rel)?;
        self.fs.read_bytes(&full).map_err(|e| self.io_err(rel, e))
    }

    fn write_bytes(&self, rel: &Utf8Path, data: &[u8]) -> Result<(), VfsError> {
        let full = self.resolve_mutable(rel)?;
        self.fs
            .write_bytes(&full, data)
            .map_err(|e| self.io_err(rel, e))
    }

    fn remove(&self, rel: &Utf8Path) -> Result<(), VfsError> {
        // A symlink inside the root may point anywhere; removing it only
        // deletes the link, so it is judged by its parent directory (the
        // link's target is irrelevant) and never followed.
        validate_relative(rel)?;
        if is_root_rel(rel) {
            // Always an error for the root; reuse its message.
            return Err(self.resolve_mutable(rel).unwrap_err());
        }
        let full = self.base.join(rel);
        if let Some(parent) = full.parent() {
            self.ensure_contained(parent, rel)?;
        }
        if self.fs.is_symlink(&full) {
            return self
                .fs
                .remove_file(&full)
                .or_else(|_| self.fs.remove_dir_all(&full))
                .map_err(|e| self.io_err(rel, e));
        }
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
        self.resolve(rel).is_ok_and(|full| self.fs.exists(&full))
    }

    fn metadata(&self, rel: &Utf8Path) -> Result<FsMetadata, VfsError> {
        let full = self.resolve(rel)?;
        self.fs.metadata(&full).map_err(|e| self.io_err(rel, e))
    }

    fn list_dir(&self, rel: &Utf8Path) -> Result<Vec<Utf8PathBuf>, VfsError> {
        let full = self.resolve(rel)?;
        let entries = self.fs.read_dir(&full).map_err(|e| self.io_err(rel, e))?;
        // `FileSystem::read_dir` returns children in the same "namespace" as
        // the path passed in (i.e. joined onto `self.base`) - translate them
        // back to paths relative to this backend's root, like the rest of
        // the VFS API expects.
        let mut out: Vec<Utf8PathBuf> = entries
            .into_iter()
            .filter_map(|full_child| {
                full_child
                    .strip_prefix(&self.base)
                    .ok()
                    .map(|p| p.to_path_buf())
            })
            .collect();
        // The OS returns directory entries in arbitrary order; sorted like
        // the packed backend so listings are deterministic.
        out.sort();
        Ok(out)
    }
}
