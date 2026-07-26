use camino::Utf8Path;
use std::collections::HashMap;
use std::sync::OnceLock;

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
}

static VFS: OnceLock<VfsContext> = OnceLock::new();

impl VfsContext {
    /// Builds the context from every [`RootDescriptor`] registered via
    /// `inventory::submit!` across every linked crate.
    ///
    /// - `project_root`: directory that each descriptor's `dev_path` is
    ///   relative to.
    /// - `archive_path`: path to the exported `.coreproject` file. Only
    ///   opened if `mode == Release` and at least one registered root is
    ///   `packed`.
    pub fn init(
        project_root: &Utf8Path,
        archive_path: Option<&Utf8Path>,
        mode: VfsMode,
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
                    Box::new(LooseBackend::new(*root, overlay_base))
                }
                (VfsMode::Release, true) => {
                    let archive_path = archive_path.ok_or_else(|| {
                        VfsError::InvalidPath(
                            "release mode with packed roots requires an archive_path".into(),
                        )
                    })?;
                    let packed = PackedBackend::open(*root, archive_path)?;
                    let overlay = LooseBackend::new(*root, overlay_base);
                    Box::new(HybridBackend::new(packed, overlay))
                }
            };
            backends.insert(*root, backend);
        }

        let ctx = VfsContext { backends };
        VFS.set(ctx).map_err(|_| VfsError::AlreadyInitialized)
    }

    pub fn global() -> Result<&'static VfsContext, VfsError> {
        VFS.get().ok_or(VfsError::NotInitialized)
    }

    pub(crate) fn backend(&self, root: Root) -> Result<&dyn Backend, VfsError> {
        self.backends
            .get(&root)
            .map(|b| b.as_ref())
            .ok_or(VfsError::RootNotRegistered(root))
    }
}
