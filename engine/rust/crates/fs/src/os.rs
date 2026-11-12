use camino::{Utf8Path, Utf8PathBuf};
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, Write};
use std::sync::atomic::{AtomicU32, Ordering};

use crate::{FileSystem, FsMetadata, MappedFile, MmapHandle, ReadSeek};

/// The real OS filesystem - a thin wrapper over `std::fs`. Exists purely
/// so callers depend on the [`FileSystem`] trait instead of `std::fs`
/// directly, making them swappable (see [`crate::MemoryFileSystem`]) and
/// testable without touching a real disk.
pub struct OsFileSystem;

/// The error for a file another reader/writer holds locked.
fn busy(path: &Utf8Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::ResourceBusy,
        format!("'{path}' is memory-mapped or being written by someone else"),
    )
}

/// A not-yet-existing sibling of `path` for write-then-rename.
fn temp_sibling(path: &Utf8Path) -> Utf8PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let name = path.file_name().unwrap_or("file");
    let unique = format!(
        ".{name}.{}.{}.tmp",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    path.with_file_name(unique)
}

impl FileSystem for OsFileSystem {
    fn open_read(&self, path: &Utf8Path) -> io::Result<Box<dyn ReadSeek + Send>> {
        Ok(Box::new(File::open(path)?))
    }

    /// Takes an exclusive lock *before* truncating, and keeps it until the
    /// writer is dropped. A file that is memory-mapped (see
    /// [`FileSystem::mmap_read`]) holds a shared lock, so this fails with
    /// [`io::ErrorKind::ResourceBusy`] instead of truncating the file under
    /// the mapping, and an archive being written cannot be mapped half done.
    fn open_write(&self, path: &Utf8Path) -> io::Result<Box<dyn Write + Send>> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        // No `truncate(true)`: that would empty the file before the lock is
        // known to be ours.
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        match file.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return Err(busy(path)),
            // A filesystem without lock support: write as before.
            Err(TryLockError::Error(_)) => {}
        }
        file.set_len(0)?;
        Ok(Box::new(file))
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
            // A name that is not valid UTF-8 cannot be represented by
            // `Utf8Path`. Converting it lossily would list a path (with
            // U+FFFD in it) that does not exist and make distinct files
            // collide, so such entries are skipped instead.
            if let Ok(name) = entry.file_name().into_string() {
                out.push(path.join(name));
            }
        }
        Ok(out)
    }

    fn rename(&self, from: &Utf8Path, to: &Utf8Path) -> io::Result<()> {
        fs::rename(from, to)
    }

    /// Crash-safe: the data goes to a temporary file in the same directory
    /// that is flushed to disk and then renamed over `path`, so a crash or
    /// full disk leaves either the old file or the new one - never a
    /// truncated mix. (Writing *through* a symlink keeps the plain
    /// in-place behaviour so the link itself is not replaced.)
    fn write_bytes(&self, path: &Utf8Path, data: &[u8]) -> io::Result<()> {
        if self.is_symlink(path) {
            return self.open_write(path)?.write_all(data);
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let temp = temp_sibling(path);
        let written = File::create(&temp).and_then(|mut file| {
            file.write_all(data)?;
            file.sync_all()
        });
        match written.and_then(|()| fs::rename(&temp, path)) {
            Ok(()) => Ok(()),
            Err(e) => {
                let _ = fs::remove_file(&temp);
                Err(e)
            }
        }
    }

    fn canonicalize(&self, path: &Utf8Path) -> io::Result<Utf8PathBuf> {
        let resolved = fs::canonicalize(path)?;
        Utf8PathBuf::from_path_buf(resolved).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "canonical path is not valid UTF-8",
            )
        })
    }

    fn is_symlink(&self, path: &Utf8Path) -> bool {
        fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink())
    }

    #[allow(
        unsafe_code,
        reason = "memmap2::Mmap::map is an unsafe fn with no safe equivalent; \
                  see the Safety comment below for why this call upholds its contract"
    )]
    fn mmap_read(&self, path: &Utf8Path) -> io::Result<MmapHandle> {
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            // FILE_SHARE_READ only: while the mapping lives, no process can
            // open the file for writing or delete it.
            options.share_mode(0x0000_0001);
        }
        let file = options.open(path)?;

        // A shared lock for as long as the mapping lives: writers that take
        // the exclusive lock (`open_write`) wait out - here: refuse - a
        // mapped file, and we refuse to map a file mid-write.
        let locked = match file.try_lock_shared() {
            Ok(()) => true,
            Err(TryLockError::WouldBlock) => return Err(busy(path)),
            // A filesystem without lock support: map anyway, unprotected
            // (`MappedFile::is_locked` reports it).
            Err(TryLockError::Error(_)) => false,
        };

        // Safety: the contract of `Mmap::map` is that the file is not
        // modified while mapped. The mapping keeps `file` - and with it the
        // shared lock above - alive, so every writer that honours the lock
        // (all of this engine's: `open_write`, and `write_bytes` replaces the
        // file by renaming, which leaves the mapped file untouched) is kept
        // out; on Windows the share mode makes that binding for everyone.
        // What remains is a foreign Unix process that ignores the advisory
        // lock and truncates the file in place; that is outside what this
        // function can prevent, and is documented on `MappedFile`.
        let map = unsafe { memmap2::Mmap::map(&file) }?;
        Ok(MmapHandle::Mapped(MappedFile::new(map, file, locked)))
    }
}
