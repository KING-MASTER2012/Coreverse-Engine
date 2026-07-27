use camino::{Utf8Path, Utf8PathBuf};
use std::collections::{HashMap, HashSet};
use std::io::{self, Cursor, Write};
use std::sync::{Arc, RwLock};

use crate::{FileSystem, FsMetadata, MmapHandle, ReadSeek};

/// Fully in-memory filesystem - no real disk I/O at all. Meant for tests:
/// build a project layout in memory, run VFS logic against it, and assert
/// on the results without ever touching a real path (so tests are
/// hermetic and can't accidentally point at something like `D:/`).
#[derive(Clone, Default)]
pub struct MemoryFileSystem {
    files: Arc<RwLock<HashMap<Utf8PathBuf, Arc<[u8]>>>>,
    dirs: Arc<RwLock<HashSet<Utf8PathBuf>>>,
}

impl MemoryFileSystem {
    /// Init function of memory system
    pub fn new() -> Self {
        Self::default()
    }

    /// Test helper: seed a file directly, without going through
    /// `open_write`. Also marks its parent directories as existing.
    pub fn seed(&self, path: impl Into<Utf8PathBuf>, data: impl Into<Vec<u8>>) {
        let path = path.into();
        if let Some(parent) = path.parent() {
            self.mark_dir(parent);
        }
        self.files
            .write()
            .unwrap()
            .insert(path, Arc::from(data.into()));
    }

    fn mark_dir(&self, path: &Utf8Path) {
        let mut dirs = self.dirs.write().unwrap();
        let mut current = Some(path.to_path_buf());
        while let Some(p) = current {
            if !dirs.insert(p.clone()) {
                break; // already marked, so are its ancestors
            }
            current = p.parent().map(|p| p.to_path_buf());
        }
    }

    fn not_found(path: &Utf8Path) -> io::Error {
        io::Error::new(io::ErrorKind::NotFound, path.to_string())
    }
}

impl FileSystem for MemoryFileSystem {
    fn open_read(&self, path: &Utf8Path) -> io::Result<Box<dyn ReadSeek + Send>> {
        let data = self
            .files
            .read()
            .unwrap()
            .get(path)
            .cloned()
            .ok_or_else(|| Self::not_found(path))?;
        Ok(Box::new(Cursor::new(data.to_vec())))
    }

    fn open_write(&self, path: &Utf8Path) -> io::Result<Box<dyn Write + Send>> {
        if let Some(parent) = path.parent() {
            self.mark_dir(parent);
        }
        Ok(Box::new(MemoryWriter {
            path: path.to_path_buf(),
            buf: Vec::new(),
            files: Arc::clone(&self.files),
        }))
    }

    fn create_dir_all(&self, path: &Utf8Path) -> io::Result<()> {
        self.mark_dir(path);
        Ok(())
    }

    fn remove_file(&self, path: &Utf8Path) -> io::Result<()> {
        self.files
            .write()
            .unwrap()
            .remove(path)
            .map(|_| ())
            .ok_or_else(|| Self::not_found(path))
    }

    fn remove_dir_all(&self, path: &Utf8Path) -> io::Result<()> {
        let prefix_owned = format!("{path}/");
        self.files
            .write()
            .unwrap()
            .retain(|p, _| !(p.as_str() == path.as_str() || p.as_str().starts_with(&prefix_owned)));
        self.dirs
            .write()
            .unwrap()
            .retain(|p| !(p.as_str() == path.as_str() || p.as_str().starts_with(&prefix_owned)));
        Ok(())
    }

    fn exists(&self, path: &Utf8Path) -> bool {
        self.files.read().unwrap().contains_key(path) || self.dirs.read().unwrap().contains(path)
    }

    fn metadata(&self, path: &Utf8Path) -> io::Result<FsMetadata> {
        if let Some(data) = self.files.read().unwrap().get(path) {
            return Ok(FsMetadata {
                len: data.len() as u64,
                is_dir: false,
            });
        }
        if self.dirs.read().unwrap().contains(path) {
            return Ok(FsMetadata {
                len: 0,
                is_dir: true,
            });
        }
        Err(Self::not_found(path))
    }

    fn read_dir(&self, path: &Utf8Path) -> io::Result<Vec<Utf8PathBuf>> {
        if !self.exists(path) {
            return Err(Self::not_found(path));
        }
        let mut out: Vec<Utf8PathBuf> = self
            .files
            .read()
            .unwrap()
            .keys()
            .filter(|p| p.parent() == Some(path))
            .cloned()
            .collect();
        out.extend(
            self.dirs
                .read()
                .unwrap()
                .iter()
                .filter(|p| p.parent() == Some(path))
                .cloned(),
        );
        out.sort();
        out.dedup();
        Ok(out)
    }

    fn mmap_read(&self, path: &Utf8Path) -> io::Result<MmapHandle> {
        let data = self
            .files
            .read()
            .unwrap()
            .get(path)
            .cloned()
            .ok_or_else(|| Self::not_found(path))?;
        Ok(MmapHandle::InMemory(data))
    }
}

/// Buffers writes and commits them to the shared map on `flush`/`Drop`,
/// mirroring how a real file is only reliably "there" once closed/flushed.
struct MemoryWriter {
    path: Utf8PathBuf,
    buf: Vec<u8>,
    files: Arc<RwLock<HashMap<Utf8PathBuf, Arc<[u8]>>>>,
}

impl Write for MemoryWriter {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.files
            .write()
            .unwrap()
            .insert(self.path.clone(), Arc::from(self.buf.as_slice()));
        Ok(())
    }
}

impl Drop for MemoryWriter {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}
