use camino::{Utf8Path, Utf8PathBuf};
use std::fs::{self, File};
use std::io::{self, Write};

use crate::{FileSystem, FsMetadata, MmapHandle, ReadSeek};

/// The real OS filesystem - a thin wrapper over `std::fs`. Exists purely
/// so callers depend on the [`FileSystem`] trait instead of `std::fs`
/// directly, making them swappable (see [`crate::MemoryFileSystem`]) and
/// testable without touching a real disk.
pub struct OsFileSystem;

impl FileSystem for OsFileSystem {
    fn open_read(&self, path: &Utf8Path) -> io::Result<Box<dyn ReadSeek + Send>> {
        Ok(Box::new(File::open(path)?))
    }

    fn open_write(&self, path: &Utf8Path) -> io::Result<Box<dyn Write + Send>> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(Box::new(File::create(path)?))
    }

    fn create_dir_all(&self, path: &Utf8Path) -> io::Result<()> {
        fs::create_dir_all(path)
    }

    fn remove_file(&self, path: &Utf8Path) -> io::Result<()> {
        fs::remove_file(path)
    }

    fn remove_dir_all(&self, path: &Utf8Path) -> io::Result<()> {
        fs::remove_dir_all(path)
    }

    fn exists(&self, path: &Utf8Path) -> bool {
        path.exists()
    }

    fn metadata(&self, path: &Utf8Path) -> io::Result<FsMetadata> {
        let meta = fs::metadata(path)?;
        Ok(FsMetadata {
            len: meta.len(),
            is_dir: meta.is_dir(),
        })
    }

    fn read_dir(&self, path: &Utf8Path) -> io::Result<Vec<Utf8PathBuf>> {
        let mut out = Vec::new();
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            out.push(path.join(name.as_ref()));
        }
        Ok(out)
    }

    fn mmap_read(&self, path: &Utf8Path) -> io::Result<MmapHandle> {
        let file = File::open(path)?;
        // Safety: the engine treats mapped files (packed archives) as
        // immutable for the duration they stay mapped - nothing else in
        // this process writes to them while mapped.
        let mmap = unsafe { memmap2::Mmap::map(&file) }?;
        Ok(MmapHandle::Mapped(mmap))
    }
}
