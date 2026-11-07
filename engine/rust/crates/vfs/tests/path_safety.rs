//! Regression tests for the VFS safety fixes: path validation, root
//! protection, symlink containment and hostile packed archives.
//!
//! These use the backends directly (no global `VfsContext`), so they can
//! share one test binary.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use camino::{Utf8Path, Utf8PathBuf};
use fs::{FileSystem, MemoryFileSystem, OsFileSystem};
use vfs::backend::Backend;
use vfs::backend::loose::LooseBackend;
use vfs::backend::packed::PackedBackend;
use vfs::format::{self, PackedEntry};
use vfs::{Root, VfsError, VfsPath};

/// A scratch directory on the real filesystem, removed on drop.
struct TempDir(Utf8PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let unique = format!(
            "cv_vfs_{name}_{}_{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let dir = Utf8PathBuf::from_path_buf(std::env::temp_dir().join(unique))
            .expect("temp dir is valid UTF-8");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        Self(dir)
    }

    fn path(&self) -> &Utf8Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn is_invalid_path<T: std::fmt::Debug>(result: Result<T, VfsError>) -> bool {
    matches!(result, Err(VfsError::InvalidPath(_)))
}

// -- VfsPath validation -------------------------------------------------

#[test]
fn vfs_path_rejects_rooted_drive_and_stream_paths_on_every_platform() {
    for bad in [
        "/etc/passwd",
        "\\windows\\system32",
        "C:foo",
        "C:/Windows",
        "C:\\Windows",
        "file.txt:stream",
        "a/../b",
        "..",
        "a\0b",
    ] {
        assert!(
            is_invalid_path(VfsPath::new(Root::Assets, bad)),
            "{bad:?} must be rejected"
        );
    }
}

#[test]
fn vfs_path_accepts_normal_and_root_paths() {
    for good in ["a", "a/b.txt", "a/./b", "dir with spaces/x y.png"] {
        assert!(VfsPath::new(Root::Assets, good).is_ok(), "{good:?}");
    }
    for root in ["", ".", "./."] {
        let path = VfsPath::new(Root::Assets, root).expect("root path is valid");
        assert!(path.is_root(), "{root:?} names the root");
    }
    assert!(!VfsPath::new(Root::Assets, "a").unwrap().is_root());
}

// -- LooseBackend: root protection and raw-path validation --------------

#[test]
fn loose_backend_refuses_to_remove_or_write_the_root() {
    let dir = TempDir::new("root_guard");
    let assets = dir.path().join("Assets");
    std::fs::create_dir_all(assets.join("sub")).unwrap();
    std::fs::write(assets.join("a.txt"), "a").unwrap();

    let backend = LooseBackend::new(Root::Assets, assets.clone(), Arc::new(OsFileSystem));

    for root in ["", ".", "./."] {
        assert!(
            is_invalid_path(backend.remove(Utf8Path::new(root))),
            "remove({root:?})"
        );
        assert!(
            is_invalid_path(backend.write_bytes(Utf8Path::new(root), b"x")),
            "write_bytes({root:?})"
        );
    }
    assert!(assets.join("a.txt").exists(), "the root must be untouched");
    assert!(assets.join("sub").exists());

    // Listing the root is still fine.
    assert_eq!(backend.list_dir(Utf8Path::new("")).unwrap().len(), 2);
}

#[test]
fn loose_backend_rejects_raw_escape_paths() {
    let dir = TempDir::new("raw_paths");
    let assets = dir.path().join("Assets");
    std::fs::create_dir_all(&assets).unwrap();
    std::fs::write(dir.path().join("secret.txt"), "secret").unwrap();

    let backend = LooseBackend::new(Root::Assets, assets, Arc::new(OsFileSystem));
    for bad in [
        "../secret.txt",
        "/etc/hostname",
        "C:/x",
        "a/../../secret.txt",
    ] {
        assert!(
            is_invalid_path(backend.read_bytes(Utf8Path::new(bad))),
            "read {bad:?}"
        );
        assert!(
            is_invalid_path(backend.write_bytes(Utf8Path::new(bad), b"x")),
            "write {bad:?}"
        );
        assert!(!backend.exists(Utf8Path::new(bad)), "exists {bad:?}");
    }
}

// -- LooseBackend: symlink containment ----------------------------------

#[cfg(unix)]
#[test]
fn loose_backend_blocks_symlink_escapes() {
    use std::os::unix::fs::symlink;

    let dir = TempDir::new("symlinks");
    let assets = dir.path().join("Assets");
    let outside = dir.path().join("outside");
    std::fs::create_dir_all(&assets).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("secret.txt"), "TOP SECRET").unwrap();
    std::fs::write(assets.join("ok.txt"), "fine").unwrap();

    symlink(outside.join("secret.txt"), assets.join("file_link")).unwrap();
    symlink(&outside, assets.join("dir_link")).unwrap();
    symlink(outside.join("missing"), assets.join("dangling")).unwrap();
    // A link that stays inside the root is legitimate.
    symlink(assets.join("ok.txt"), assets.join("inner_link")).unwrap();

    let backend = LooseBackend::new(Root::Assets, assets.clone(), Arc::new(OsFileSystem));

    // Reads through an escaping file or directory symlink are refused.
    assert!(is_invalid_path(
        backend.read_bytes(Utf8Path::new("file_link"))
    ));
    assert!(is_invalid_path(
        backend.read_bytes(Utf8Path::new("dir_link/secret.txt"))
    ));
    assert!(!backend.exists(Utf8Path::new("dir_link/secret.txt")));

    // Writes through them are refused too - nothing lands outside.
    assert!(is_invalid_path(
        backend.write_bytes(Utf8Path::new("dir_link/pwned.txt"), b"x")
    ));
    assert!(is_invalid_path(
        backend.write_bytes(Utf8Path::new("dangling"), b"x")
    ));
    assert!(!outside.join("pwned.txt").exists());
    assert!(!outside.join("missing").exists());

    // Links that stay inside the root, and normal files, keep working.
    assert_eq!(
        backend.read_bytes(Utf8Path::new("inner_link")).unwrap(),
        b"fine"
    );
    backend
        .write_bytes(Utf8Path::new("new/dir/file.txt"), b"hi")
        .expect("writing a new nested file works");
    assert_eq!(
        backend
            .read_bytes(Utf8Path::new("new/dir/file.txt"))
            .unwrap(),
        b"hi"
    );

    // Removing an escaping link deletes the link, never its target.
    backend
        .remove(Utf8Path::new("dir_link"))
        .expect("remove link");
    assert!(!assets.join("dir_link").exists());
    assert!(outside.join("secret.txt").exists(), "target must survive");
    backend
        .remove(Utf8Path::new("file_link"))
        .expect("remove link");
    assert!(outside.join("secret.txt").exists(), "target must survive");
}

// -- Packed archives: hostile input -------------------------------------

/// A syntactically valid archive header with one entry and a custom
/// `(offset, len)`, followed by `payload` as the data section.
fn archive_with_entry(count: u32, offset: u64, len: u64, payload: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"CVPK");
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&count.to_le_bytes());
    // root id 0 (Assets), path "x"
    bytes.push(0);
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(b"x");
    bytes.extend_from_slice(&offset.to_le_bytes());
    bytes.extend_from_slice(&len.to_le_bytes());
    bytes.extend_from_slice(payload);
    bytes
}

fn memory_fs_with(name: &str, bytes: Vec<u8>) -> Arc<dyn FileSystem> {
    let fs = MemoryFileSystem::new();
    fs.seed(name, bytes);
    Arc::new(fs)
}

#[test]
fn archive_with_absurd_entry_count_is_rejected_without_allocating() {
    // Header claims u32::MAX entries in a ~40 byte file. Before the fix this
    // asked the allocator for ~400 GB and aborted the process.
    let fs = memory_fs_with("huge.cvpk", archive_with_entry(u32::MAX, 0, 0, b""));
    let err = format::read_index(&*fs, Utf8Path::new("huge.cvpk"))
        .err()
        .expect("archive must be rejected");
    assert!(matches!(err, VfsError::CorruptArchive(_)), "{err:?}");
}

#[test]
fn archive_entry_with_overflowing_offset_is_rejected() {
    // data_start + offset wraps to 0 in release builds: the entry would
    // alias the header ("CVPK") instead of failing.
    let header_len = archive_with_entry(1, 0, 0, b"").len() as u64;
    let evil_offset = u64::MAX - header_len + 1;
    let fs = memory_fs_with(
        "evil.cvpk",
        archive_with_entry(1, evil_offset, 8, b"SECRET!!"),
    );

    let err = format::read_index(&*fs, Utf8Path::new("evil.cvpk"))
        .err()
        .expect("archive must be rejected");
    assert!(matches!(err, VfsError::CorruptArchive(_)), "{err:?}");
}

#[test]
fn archive_entry_past_end_of_file_is_rejected() {
    let fs = memory_fs_with("short.cvpk", archive_with_entry(1, 0, 100, b"only-8by"));
    let err = format::read_index(&*fs, Utf8Path::new("short.cvpk"))
        .err()
        .expect("archive must be rejected");
    assert!(matches!(err, VfsError::CorruptArchive(_)), "{err:?}");

    // offset + len overflowing u64
    let fs = memory_fs_with("wrap.cvpk", archive_with_entry(1, u64::MAX, 2, b"zz"));
    let err = format::read_index(&*fs, Utf8Path::new("wrap.cvpk"))
        .err()
        .expect("archive must be rejected");
    assert!(matches!(err, VfsError::CorruptArchive(_)), "{err:?}");
}

#[test]
fn valid_archive_still_round_trips() {
    let fs: Arc<dyn FileSystem> = Arc::new(MemoryFileSystem::new());
    let entries = vec![
        PackedEntry {
            root: Root::Assets,
            rel: "a/one.bin".into(),
            data: vec![1, 2, 3],
        },
        PackedEntry {
            root: Root::Assets,
            rel: "two.bin".into(),
            data: vec![9; 1000],
        },
        PackedEntry {
            root: Root::Assets,
            rel: "empty.bin".into(),
            data: Vec::new(),
        },
    ];
    format::write_archive(&*fs, &entries, Utf8Path::new("ok.cvpk")).unwrap();

    let packed =
        PackedBackend::open(Root::Assets, Utf8Path::new("ok.cvpk"), Arc::clone(&fs)).unwrap();
    assert_eq!(
        packed.read_bytes(Utf8Path::new("a/one.bin")).unwrap(),
        [1, 2, 3]
    );
    assert_eq!(
        packed.read_bytes(Utf8Path::new("two.bin")).unwrap(),
        vec![9; 1000]
    );
    assert!(
        packed
            .read_bytes(Utf8Path::new("empty.bin"))
            .unwrap()
            .is_empty()
    );
}
