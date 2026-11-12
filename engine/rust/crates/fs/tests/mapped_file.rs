//! A memory-mapped file is locked for as long as the mapping lives, so it
//! cannot be truncated or rewritten in place underneath it.

use std::io::{ErrorKind, Write};
use std::sync::atomic::{AtomicU32, Ordering};

use camino::Utf8PathBuf;
use fs::{FileSystem, MmapHandle, OsFileSystem};

struct TempDir(Utf8PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let unique = format!(
            "cv_mmap_{name}_{}_{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let dir = Utf8PathBuf::from_path_buf(std::env::temp_dir().join(unique))
            .expect("temp dir is valid UTF-8");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        Self(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// `open_write` refuses with `ResourceBusy` where file locks are advisory
/// (Unix) and with a sharing violation (`PermissionDenied`) on Windows.
fn is_refused(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        ErrorKind::ResourceBusy | ErrorKind::PermissionDenied
    )
}

#[test]
fn a_mapped_file_cannot_be_truncated_by_a_writer() {
    let dir = TempDir::new("truncate");
    let path = dir.0.join("archive.bin");
    std::fs::write(&path, b"original contents").unwrap();

    let mapping = OsFileSystem.mmap_read(&path).unwrap();
    assert_eq!(&mapping[..], b"original contents");
    if let MmapHandle::Mapped(mapped) = &mapping {
        assert!(mapped.is_locked(), "a local filesystem supports locks");
    } else {
        panic!("the OS filesystem returns a real mapping");
    }

    // Before the fix `File::create` emptied the file here, and reading the
    // mapping afterwards was a SIGBUS.
    let refused = OsFileSystem
        .open_write(&path)
        .err()
        .expect("writing to a mapped file must be refused");
    assert!(is_refused(&refused), "{refused:?}");
    assert_eq!(
        std::fs::read(&path).unwrap(),
        b"original contents",
        "the refused writer must not have truncated the file"
    );
    assert_eq!(&mapping[..], b"original contents");

    // Once the mapping is gone the file is writable again.
    drop(mapping);
    let mut writer = OsFileSystem.open_write(&path).expect("unlocked again");
    writer.write_all(b"new").unwrap();
    drop(writer);
    assert_eq!(std::fs::read(&path).unwrap(), b"new");
}

#[test]
fn a_file_being_written_cannot_be_mapped() {
    let dir = TempDir::new("mapping_a_writer");
    let path = dir.0.join("half-written.bin");

    let mut writer = OsFileSystem.open_write(&path).unwrap();
    writer.write_all(b"partial").unwrap();

    let refused = OsFileSystem
        .mmap_read(&path)
        .err()
        .expect("a file with an open writer must not be mapped");
    assert!(is_refused(&refused), "{refused:?}");

    drop(writer);
    assert_eq!(&OsFileSystem.mmap_read(&path).unwrap()[..], b"partial");
}

#[cfg(unix)]
#[test]
fn replacing_a_mapped_file_by_rename_leaves_the_mapping_intact() {
    let dir = TempDir::new("rename");
    let path = dir.0.join("archive.bin");
    OsFileSystem.write_bytes(&path, b"version one").unwrap();

    let mapping = OsFileSystem.mmap_read(&path).unwrap();

    // `write_bytes` writes a new file and renames it over the old one: the
    // mapped file is never touched, so this is allowed and safe.
    OsFileSystem.write_bytes(&path, b"version two").unwrap();
    assert_eq!(&mapping[..], b"version one");
    assert_eq!(OsFileSystem.read_bytes(&path).unwrap(), b"version two");
}
