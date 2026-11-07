//! `FileManager` must refuse destructive operations on a root itself.
//!
//! Its own test binary: `VfsContext::init_with_fs` succeeds once per
//! process (see `list_dir_and_metadata.rs`).

use std::sync::Arc;

use camino::Utf8Path;
use fs::MemoryFileSystem;
use vfs::{Root, VfsContext, VfsError, VfsMode, VfsPath, file_manager};

#[test]
fn removing_or_writing_a_root_is_rejected_and_keeps_its_files() {
    let fs = Arc::new(MemoryFileSystem::default());
    VfsContext::init_with_fs(Utf8Path::new("."), None, VfsMode::Development, fs)
        .expect("init_with_fs should succeed once in this process");

    let file = VfsPath::new(Root::Cache, "keep/me.txt").unwrap();
    file_manager::write_bytes(&file, b"important").unwrap();

    for root in ["", ".", "./."] {
        let path = VfsPath::new(Root::Cache, root).expect("root path is valid");
        assert!(matches!(
            file_manager::remove(&path),
            Err(VfsError::InvalidPath(_))
        ));
        assert!(matches!(
            file_manager::write_bytes(&path, b"x"),
            Err(VfsError::InvalidPath(_))
        ));
    }

    assert_eq!(file_manager::read_bytes(&file).unwrap(), b"important");

    // Listing the root is still allowed.
    let root = VfsPath::new(Root::Cache, "").unwrap();
    assert!(!file_manager::list_dir(&root).unwrap().is_empty());
}
