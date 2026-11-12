use std::fs::File;
use std::ops::Deref;
use std::sync::Arc;

/// A read-only, contiguous view over a file's bytes.
///
/// [`crate::OsFileSystem`] produces a real memory-mapped [`memmap2::Mmap`];
/// [`crate::MemoryFileSystem`] produces a plain in-memory buffer. Callers
/// (e.g. `vfs`'s packed backend) just see `&[u8]` either way via [`Deref`],
/// and don't need to care which one they got.
pub enum MmapHandle {
    /// A real memory map of a file, which stays locked while it exists.
    Mapped(MappedFile),
    /// Values in memory
    InMemory(Arc<[u8]>),
}

/// A memory-mapped file together with the lock that keeps it unchanged.
///
/// Reading a mapping after the file was truncated or rewritten in place is
/// undefined behaviour (in practice a `SIGBUS` crash). So the file is held
/// open with a shared lock for as long as the mapping lives: a writer that
/// honours the lock - every writer in this engine does, see
/// [`crate::OsFileSystem::open_write`] - cannot start until the mapping is
/// gone, and on Windows the lock plus the open mode (`FILE_SHARE_READ`) are
/// enforced for every process.
///
/// On Unix the lock is advisory (`flock`): a program that ignores it and
/// truncates the file in place can still break a live mapping. Replacing the
/// file by renaming a new one over it (what [`crate::FileSystem::write_bytes`]
/// does) is always safe - the mapping keeps the old file alive.
pub struct MappedFile {
    // Field order matters: the map is dropped (unmapped) before the file is
    // closed (which releases the lock).
    map: memmap2::Mmap,
    _file: File,
    locked: bool,
}

impl MappedFile {
    pub(crate) fn new(map: memmap2::Mmap, file: File, locked: bool) -> Self {
        Self {
            map,
            _file: file,
            locked,
        }
    }

    /// Whether the shared lock was obtained. `false` only on a filesystem
    /// that does not support file locks (some network and FUSE mounts); the
    /// mapping works, but is not protected against concurrent writers.
    pub fn is_locked(&self) -> bool {
        self.locked
    }
}

impl Deref for MmapHandle {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        match self {
            MmapHandle::Mapped(m) => &m.map[..],
            MmapHandle::InMemory(v) => &v[..],
        }
    }
}
