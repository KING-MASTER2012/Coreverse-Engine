use camino::Utf8Path;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use fs::{FileSystem, OsFileSystem};
use root::Root;

use crate::backend::{Backend, hybrid::HybridBackend, loose::LooseBackend, packed::PackedBackend};
use crate::error::VfsError;
use crate::root_registry::RootDescriptor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfsMode {
    /// Every root is a plain [`LooseBackend`] pointed at `dev_path`.
    Development,
    /// `packed` roots are served from the `.coreproject` archive (with a
    /// loose overlay for overrides); non-packed roots stay loose.
    Release,
}

/// The single global entry point into the VFS. Call [`VfsContext::init`]
/// once at startup (before any [`crate::file_manager`] call), then reach it
/// from anywhere via [`VfsContext::global`].
pub struct VfsContext {
    backends: HashMap<Root, Box<dyn Backend>>,
    fs: Arc<dyn FileSystem>,
}

static VFS: OnceLock<VfsContext> = OnceLock::new();

impl VfsContext {
    /// Convenience over [`Self::init_with_fs`] using the real OS
    /// filesystem - what every non-test caller wants.
    pub fn init(
        project_root: &Utf8Path,
        archive_path: Option<&Utf8Path>,
        mode: VfsMode,
    ) -> Result<(), VfsError> {
        Self::init_with_fs(project_root, archive_path, mode, Arc::new(OsFileSystem))
    }

    /// Same as [`Self::init`], but with an injectable [`FileSystem`] - pass
    /// a [`fs::MemoryFileSystem`] in tests to exercise VFS logic without
    /// touching a real disk (no more accidentally pointing at `D:/`).
    ///
    /// Builds the context from every [`RootDescriptor`] registered via
    /// `inventory::submit!` across every linked crate.
    ///
    /// - `project_root`: directory that each descriptor's `dev_path` is
    ///   relative to.
    /// - `archive_path`: path to the exported `.coreproject` file. Only
    ///   opened if `mode == Release` and at least one registered root is
    ///   `packed`.
    pub fn init_with_fs(
        project_root: &Utf8Path,
        archive_path: Option<&Utf8Path>,
        mode: VfsMode,
        fs: Arc<dyn FileSystem>,
    ) -> Result<(), VfsError> {
        let mut descriptors: HashMap<Root, RootDescriptor> = HashMap::new();
        for d in inventory::iter::<RootDescriptor> {
            descriptors.insert(d.root, *d);
        }

        let mut backends: HashMap<Root, Box<dyn Backend>> = HashMap::new();

        for (root, desc) in &descriptors {
            let overlay_base = project_root.join(desc.dev_path);
            let backend: Box<dyn Backend> = match (mode, desc.packed) {
                (VfsMode::Development, _) | (VfsMode::Release, false) => {
                    Box::new(LooseBackend::new(*root, overlay_base, Arc::clone(&fs)))
                }
                (VfsMode::Release, true) => {
                    let archive_path = archive_path.ok_or_else(|| {
                        VfsError::InvalidPath(
                            "release mode with packed roots requires an archive_path".into(),
                        )
                    })?;
                    let packed = PackedBackend::open(*root, archive_path, Arc::clone(&fs))?;
                    let overlay = LooseBackend::new(*root, overlay_base, Arc::clone(&fs));
                    Box::new(HybridBackend::new(packed, overlay))
                }
            };
            backends.insert(*root, backend);
        }

        let ctx = VfsContext {
            backends,
            fs: Arc::clone(&fs),
        };
        VFS.set(ctx).map_err(|_| VfsError::AlreadyInitialized)
    }

    pub fn global() -> Result<&'static VfsContext, VfsError> {
        VFS.get().ok_or(VfsError::NotInitialized)
    }

    /// The [`FileSystem`] this context was initialized with - used by
    /// [`crate::file_manager::add_file_to`] to read source files that live
    /// outside any `Root` (e.g. a path the user picked in an OS file
    /// dialog), so that path stays swappable/testable too.
    pub fn fs(&self) -> &Arc<dyn FileSystem> {
        &self.fs
    }

    pub(crate) fn backend(&self, root: Root) -> Result<&dyn Backend, VfsError> {
        self.backends
            .get(&root)
            .map(|b| b.as_ref())
            .ok_or(VfsError::RootNotRegistered(root))
    }
}
