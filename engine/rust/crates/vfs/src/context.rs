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

impl VfsMode {
    /// Stable numeric id for this mode, for callers (like the `ffi`
    /// crate) that need to pass it across an ABI boundary. Mirrors
    /// `log_core::Severity::as_u8` / `root::Root::as_u8`.
    pub fn as_u8(self) -> u8 {
        match self {
            VfsMode::Development => 0,
            VfsMode::Release => 1,
        }
    }

    /// Inverse of [`Self::as_u8`]. Returns `None` for any byte other
    /// than `0` or `1`.
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(VfsMode::Development),
            1 => Some(VfsMode::Release),
            _ => None,
        }
    }
}

/// The single global entry point into the VFS. Call [`VfsContext::init`]
/// once at startup (before any [`crate::file_manager`] call), then reach it
/// from anywhere via [`VfsContext::global`].
///
/// # Re-init
/// [`Self::init`]/[`Self::init_with_fs`] can succeed **at most once per
/// process** ([`VFS`] is a [`OnceLock`](std::sync::OnceLock), which has
/// no safe "clear" — resetting it would need either `unsafe` code
/// overwriting a shared `static` behind other threads' backs, or
/// replacing it with a lock every [`Self::global`] call would then pay
/// for, and this crate has no caller that needs a mid-process re-init
/// (switching projects in the editor is a process restart today; see
/// PROGRESS.md). If that changes, revisit this decision rather than
/// reaching for the `unsafe` route above.
///
/// A consequence for this crate's own tests: every `#[test]` that calls
/// [`Self::init_with_fs`] needs the global to still be empty, so each
/// one belongs in its own file under `tests/` (a separate test
/// binary/process — `cargo test` gives every integration test file its
/// own process already) rather than sharing a file with another
/// VFS-init test. `tests/public_api.rs`'s `init_with_fs` test and
/// `tests/list_dir_and_metadata.rs` (added alongside the FFI's
/// `list_dir`/`metadata` support) follow this rule; keep it for any
/// test added later that also needs a live [`VfsContext`].
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
