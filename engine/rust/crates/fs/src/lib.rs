//! Export of files
mod memory;
mod metadata;
mod mmap;
mod os;
mod stream;

pub use memory::MemoryFileSystem;
pub use metadata::FsMetadata;
pub use mmap::MmapHandle;
pub use os::OsFileSystem;
pub use stream::ReadSeek;

use camino::{Utf8Path, Utf8PathBuf};
use std::io::{self, Read, Write};

/// OS-level file abstraction - the lowest layer of the engine's storage
/// stack. It knows nothing about `Root`s, projects, or packed archives
/// (that's `vfs`'s job) or asset types (that's the future asset system's
/// job) - it only knows how to move bytes at a path, pluggable so the
/// same higher-level code can run against a real disk ([`OsFileSystem`]),
/// an in-memory store for tests ([`MemoryFileSystem`]), or (later)
/// something like a zip/archive-backed filesystem.
///
/// Implementations must be safe to share across threads (`Send + Sync`)
/// since a single instance is expected to live behind an `Arc` and be
/// used from anywhere in the engine.
pub trait FileSystem: Send + Sync {
    /// Opens `path` for reading. Returns a stream rather than bytes so
    /// large files (audio, video, big data tables) can be read
    /// incrementally instead of always loading the whole thing into
    /// memory - this is where "IO" lives in the new layering, as a
    /// capability of `FileSystem` rather than a separate system.
    fn open_read(&self, path: &Utf8Path) -> io::Result<Box<dyn ReadSeek + Send>>;

    /// Opens `path` for writing (truncating/creating as needed).
    /// Implementations should create missing parent directories.
    fn open_write(&self, path: &Utf8Path) -> io::Result<Box<dyn Write + Send>>;
    /// Function what creates its all directories and its all files
    fn create_dir_all(&self, path: &Utf8Path) -> io::Result<()>;
    /// Function what deletes file
    fn remove_file(&self, path: &Utf8Path) -> io::Result<()>;
    /// Function what deletes its all directories and its all files
    fn remove_dir_all(&self, path: &Utf8Path) -> io::Result<()>;
    /// Function what checks that file is non-exists
    fn exists(&self, path: &Utf8Path) -> bool;
    /// Function what gets that a metadata of file
    fn metadata(&self, path: &Utf8Path) -> io::Result<FsMetadata>;
    /// Function what reads its all directories and its all files.
    fn read_dir(&self, path: &Utf8Path) -> io::Result<Vec<Utf8PathBuf>>;

    /// Memory-maps `path` read-only. Used for large, immutable data such
    /// as a packed `.coreproject` archive, where copying the whole file
    /// into a `Vec<u8>` up front would be wasteful.
    fn mmap_read(&self, path: &Utf8Path) -> io::Result<MmapHandle>;

    // --- Convenience defaults built on the primitives above ---
    // Most callers just want whole-file bytes/strings and shouldn't have
    // to write a manual stream loop for that.
    /// Function what reads file content as bytes
    fn read_bytes(&self, path: &Utf8Path) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.open_read(path)?.read_to_end(&mut buf)?;
        Ok(buf)
    }
    /// Function what reads file content as string
    fn read_to_string(&self, path: &Utf8Path) -> io::Result<String> {
        let mut s = String::new();
        self.open_read(path)?.read_to_string(&mut s)?;
        Ok(s)
    }
    /// Function what writes bytes as a file content
    fn write_bytes(&self, path: &Utf8Path, data: &[u8]) -> io::Result<()> {
        self.open_write(path)?.write_all(data)
    }
}
