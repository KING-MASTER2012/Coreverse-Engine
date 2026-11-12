//! Archive export limits, crash-safe writes and directory listing of the
//! real filesystem.

use std::sync::atomic::{AtomicU32, Ordering};

use camino::{Utf8Path, Utf8PathBuf};
use fs::{FileSystem, MemoryFileSystem, OsFileSystem};
use vfs::format::{self, PackedEntry};
use vfs::{Root, VfsError};

struct TempDir(Utf8PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let unique = format!(
            "cv_fs_{name}_{}_{}",
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

#[test]
fn write_archive_rejects_paths_the_header_cannot_represent() {
    let memory = MemoryFileSystem::new();
    let too_long = PackedEntry {
        root: Root::Assets,
        rel: "a".repeat(usize::from(u16::MAX) + 1).into(),
        data: vec![1],
    };
    let result = format::write_archive(&memory, &[too_long], Utf8Path::new("x.cvpk"));
    assert!(
        matches!(result, Err(VfsError::InvalidPath(_))),
        "{result:?}"
    );
    // Nothing - not even a partial file - was written.
    assert!(!memory.exists(Utf8Path::new("x.cvpk")));
    assert!(!memory.exists(Utf8Path::new("x.cvpk.tmp")));

    // The largest representable path still round-trips.
    let longest = PackedEntry {
        root: Root::Assets,
        rel: "a".repeat(usize::from(u16::MAX)).into(),
        data: vec![7],
    };
    format::write_archive(&memory, &[longest], Utf8Path::new("ok.cvpk")).unwrap();
    let index = format::read_index(&memory, Utf8Path::new("ok.cvpk")).unwrap();
    assert_eq!(index.entries.len(), 1);
}

#[test]
fn write_bytes_replaces_files_without_leaving_temporaries() {
    let dir = TempDir::new("atomic");
    let target = dir.0.join("sub/data.bin");

    OsFileSystem.write_bytes(&target, b"first version").unwrap();
    OsFileSystem.write_bytes(&target, b"second").unwrap();
    assert_eq!(OsFileSystem.read_bytes(&target).unwrap(), b"second");

    let leftovers: Vec<_> = std::fs::read_dir(target.parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(leftovers, ["data.bin"], "no temporary file may remain");
}

#[test]
fn rename_moves_a_file_on_both_filesystems() {
    let dir = TempDir::new("rename");
    let from = dir.0.join("from.txt");
    let to = dir.0.join("to.txt");
    OsFileSystem.write_bytes(&from, b"x").unwrap();
    OsFileSystem.write_bytes(&to, b"old").unwrap();
    OsFileSystem.rename(&from, &to).unwrap();
    assert!(!OsFileSystem.exists(&from));
    assert_eq!(OsFileSystem.read_bytes(&to).unwrap(), b"x");

    let memory = MemoryFileSystem::new();
    memory.seed("a/from.txt", b"y".to_vec());
    memory
        .rename(Utf8Path::new("a/from.txt"), Utf8Path::new("b/to.txt"))
        .unwrap();
    assert!(!memory.exists(Utf8Path::new("a/from.txt")));
    assert_eq!(memory.read_bytes(Utf8Path::new("b/to.txt")).unwrap(), b"y");
}

#[cfg(unix)]
#[test]
fn read_dir_skips_names_that_are_not_utf8() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let dir = TempDir::new("non_utf8");
    std::fs::write(dir.0.join("good.txt"), "x").unwrap();
    std::fs::write(
        dir.0.as_std_path().join(OsStr::from_bytes(b"bad\xff.txt")),
        "x",
    )
        .unwrap();
    std::fs::write(
        dir.0.as_std_path().join(OsStr::from_bytes(b"bad\xfe.txt")),
        "x",
    )
        .unwrap();

    let listed = OsFileSystem.read_dir(&dir.0).unwrap();
    // Neither a lossy U+FFFD name (a path that does not exist) nor a
    // collision between the two bad names.
    assert_eq!(listed, [dir.0.join("good.txt")]);
}

#[cfg(unix)]
#[test]
fn an_opened_archive_survives_being_replaced_on_disk() {
    use std::sync::Arc;
    use vfs::backend::packed::PackedBackend;
    use vfs::backend::Backend;

    let dir = TempDir::new("replace_archive");
    let path = dir.0.join("game.cvpk");
    let fs: Arc<dyn FileSystem> = Arc::new(OsFileSystem);
    let entry = |data: &[u8]| PackedEntry {
        root: Root::Assets,
        rel: "a.bin".into(),
        data: data.to_vec(),
    };

    format::write_archive(&*fs, &[entry(b"first")], &path).unwrap();
    let open = PackedBackend::open(Root::Assets, &path, Arc::clone(&fs)).unwrap();

    // Re-exporting while the game has the archive open: the exporter writes
    // `<path>.tmp` and renames it over the archive, so the open one is not
    // truncated underneath its mapping.
    format::write_archive(&*fs, &[entry(b"second")], &path).unwrap();
    assert_eq!(open.read_bytes(Utf8Path::new("a.bin")).unwrap(), b"first");

    let reopened = PackedBackend::open(Root::Assets, &path, fs).unwrap();
    assert_eq!(
        reopened.read_bytes(Utf8Path::new("a.bin")).unwrap(),
        b"second"
    );
}
