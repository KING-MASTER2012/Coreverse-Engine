//! Its own test binary/process, on purpose: `VfsContext::init_with_fs`
//! can only succeed once per process (see `context.rs`'s "Re-init"
//! doc), and `tests/public_api.rs` already uses its one shot on its own
//! `init_with_fs` test. Keeping this in a separate file gives it a
//! separate process and a fresh `VfsContext` to call `init_with_fs`
//! with, instead of racing/conflicting with that one.
//!
//! Everything below lives in a single `#[test]` function for the same
//! reason this is a separate *file*: `cargo test` runs the `#[test]`
//! functions *within* one binary concurrently, and a second one here
//! calling `init_with_fs` would hit the same one-shot limit against the
//! first, in-process this time instead of across files.
//!
//! Covers `list_dir`/`metadata`/`remove`/`add_file`/`new_temp_path` at
//! the `vfs_crate::file_manager` level -- the same calls `ffi::vfs`'s
//! Phase 7c additions (`ffi_vfs_list_dir`, `ffi_vfs_metadata`, ...)
//! forward to, so a bug here would be a bug there too.

use std::sync::Arc;

use camino::Utf8Path;
use fs::MemoryFileSystem;
use vfs::{Root, VfsContext, VfsMode, VfsPath, file_manager};

#[test]
fn list_dir_metadata_remove_and_add_file() {
    let fs = Arc::new(MemoryFileSystem::default());
    VfsContext::init_with_fs(Utf8Path::new("."), None, VfsMode::Development, fs)
        .expect("init_with_fs should succeed exactly once in this process");

    // -- metadata: size and is_dir --------------------------------------
    let dir = VfsPath::new(Root::Cache, "sub").expect("valid path");
    let a = dir.join("a.txt").expect("valid path");
    let b = dir.join("b.txt").expect("valid path");

    file_manager::write_bytes(&a, b"hello").expect("write a.txt");
    file_manager::write_bytes(&b, b"world!!").expect("write b.txt");

    let meta_a = file_manager::metadata(&a).expect("metadata a.txt");
    assert_eq!(meta_a.len, 5);
    assert!(!meta_a.is_dir);

    let dir_meta = file_manager::metadata(&dir).expect("metadata sub");
    assert!(dir_meta.is_dir);

    // -- list_dir: both files, relative to `root` ------------------------
    let mut entries: Vec<String> = file_manager::list_dir(&dir)
        .expect("list_dir sub")
        .into_iter()
        .map(|p| p.rel().to_string())
        .collect();
    entries.sort();
    assert_eq!(
        entries,
        vec!["sub/a.txt".to_string(), "sub/b.txt".to_string()]
    );

    // -- remove: the file is gone, its sibling is not ---------------------
    file_manager::remove(&a).expect("remove a.txt");
    assert!(!file_manager::exists(&a).expect("exists a.txt"));
    assert!(file_manager::exists(&b).expect("exists b.txt"));

    // -- new_temp_path: unique, under Root::Temp ---------------------------
    let temp1 = file_manager::new_temp_path("scratch").expect("new_temp_path");
    let temp2 = file_manager::new_temp_path("scratch").expect("new_temp_path");
    assert_eq!(temp1.root(), Root::Temp);
    assert_ne!(
        temp1, temp2,
        "two calls in the same process must not collide"
    );

    // -- add_file / add_file_to: both keep-the-name and rename paths -------
    // `add_file`/`add_file_to` read `src` through the VfsContext's own
    // `FileSystem` (see `file_manager::add_file_to`'s doc comment) --
    // not through the VFS's Root/rel addressing -- matching a real path
    // outside the project (e.g. one an OS file dialog handed back).
    // Seed one directly through that same FileSystem.
    let ctx = VfsContext::global().expect("initialized above");
    ctx.fs()
        .write_bytes(Utf8Path::new("external/dropped.txt"), b"dropped")
        .expect("seed external file");
    file_manager::add_file(Utf8Path::new("external/dropped.txt"), Root::Cache)
        .expect("add_file");
    let copied = VfsPath::new(Root::Cache, "dropped.txt").expect("valid path");
    assert_eq!(
        file_manager::read_bytes(&copied).expect("read dropped.txt"),
        b"dropped"
    );

    // add_file_to (explicit destination)
    let dest = VfsPath::new(Root::Cache, "renamed/dest.txt").expect("valid path");
    file_manager::add_file_to(Utf8Path::new("external/dropped.txt"), &dest)
        .expect("add_file_to");
    assert_eq!(
        file_manager::read_bytes(&dest).expect("read renamed/dest.txt"),
        b"dropped"
    );
}
