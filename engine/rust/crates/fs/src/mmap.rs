use std::ops::Deref;
use std::sync::Arc;

/// A read-only, contiguous view over a file's bytes.
///
/// [`crate::OsFileSystem`] produces a real memory-mapped [`memmap2::Mmap`];
/// [`crate::MemoryFileSystem`] produces a plain in-memory buffer. Callers
/// (e.g. `vfs`'s packed backend) just see `&[u8]` either way via [`Deref`],
/// and don't need to care which one they got.
pub enum MmapHandle {
    /// Values in map
    Mapped(memmap2::Mmap),
    /// Values in memory
    InMemory(Arc<[u8]>),
}

impl Deref for MmapHandle {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        match self {
            MmapHandle::Mapped(m) => &m[..],
            MmapHandle::InMemory(v) => &v[..],
        }
    }
}
